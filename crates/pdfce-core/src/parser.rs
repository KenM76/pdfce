//! # COS object parser (ISO 32000-1 §7.3 object grammar)
//!
//! Recursive-descent parser from the token stream (`crate::lexer`) to
//! the object model (`crate::object`). Spec sources:
//! `iso32000__s__7.3.md` (grammar), `iso32000__s__7.3.10.md` (indirect
//! objects, the `N G R` lookahead), `iso32000__s__7.3.8.md` (stream
//! framing) in the PDF-spec RAG. Clause numbers are ISO 32000-1:2008.
//!
//! ## What this layer does and doesn't
//!
//! [`Parser`] parses **one syntactic region** — a direct object, or one
//! complete `N G obj … endobj` indirect-object definition at a known
//! offset. It does NOT walk cross-reference tables (that's
//! `crate::xref`), does not resolve references (document layer), and
//! does not decode stream data (filter layer). The one place it needs
//! outside help is a stream's `/Length`, which may itself be an
//! indirect reference (§7.3.10 EXAMPLE 3 sanctions this for single-pass
//! writers) — the caller supplies a resolver callback for that case,
//! keeping the parser xref-agnostic.
//!
//! ## The `N G R` lookahead (§7.3.10)
//!
//! An indirect reference is *three* tokens — `Integer Integer Keyword(R)`
//! — indistinguishable from two array elements `1 0` followed by more
//! content until the third token is seen. The parser therefore keeps a
//! two-token peek buffer: on reading an `Integer`, it peeks up to two
//! tokens ahead and commits to a reference only when it sees
//! `Integer` + `R`. (This ambiguity is exactly why §7.8.2 bans indirect
//! references inside content streams.)
//!
//! ## Stream framing rules enforced (§7.3.8.1)
//!
//! - The `stream` keyword shall be followed by CRLF or LF alone —
//!   **CR alone is a hard error** (the spec's own NOTE 2 explains the
//!   ambiguity it would create). The RAG calls this "the single
//!   highest-value byte rule in the whole file-structure area."
//! - Data is exactly `/Length` bytes from the byte after that EOL.
//! - After the data: optional EOL, then `endstream`, then `endobj`.
//!   Anything else is a `/Length` inconsistency (§7.3.8.2 "an error") —
//!   fail-clean, no scan-for-`endstream` guessing in Pass 1 (recovery
//!   heuristics are a later, corpus-driven, deliberate addition; see
//!   `C:\personal_rag\pdf\`).
//!
//! ## Guards (ARCHITECTURE.md §10 — pdfce policy, not spec)
//!
//! [`MAX_NESTING_DEPTH`] bounds recursion (the spec bounds nothing
//! here; a `[[[[…` bomb must not overflow the stack).

use crate::lexer::{LexError, Lexer, Token, TokenKind};
use crate::object::{Dict, IndirectObject, Name, ObjId, Object, Provenance, Stream};
use crate::span::ByteSpan;

/// Maximum container (array/dictionary) nesting depth.
///
/// pdfce policy (ARCHITECTURE.md §10): ISO 32000-1 does not bound
/// object nesting (Annex C bounds only `q`/`Q` nesting), so the guard
/// value is ours. 256 is far beyond any legitimate document structure
/// while keeping worst-case recursion shallow enough for any thread's
/// stack.
pub const MAX_NESTING_DEPTH: usize = 256;

/// A structural parse error: what went wrong and where.
///
/// C-GOOD-ERR via `thiserror`; offsets are absolute buffer offsets, the
/// same coordinate system as [`ByteSpan`].
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("parse error at byte {offset}: {kind}")]
pub struct ParseError {
    /// Byte offset where the problem was detected.
    pub offset: usize,
    /// What was wrong.
    pub kind: ParseErrorKind,
}

/// Classification of structural parse errors.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseErrorKind {
    /// The lexer failed underneath the parser.
    #[error("lexical error: {0}")]
    Lex(#[from] LexError),
    /// Input ended where an object (or more of one) was required.
    #[error("unexpected end of input")]
    UnexpectedEof,
    /// A token that cannot begin/continue the construct being parsed.
    /// The payload is a static description of what was expected.
    #[error("unexpected token; expected {0}")]
    Unexpected(&'static str),
    /// Dictionary key position held a non-name object (§7.3.7 "key
    /// shall be a name").
    #[error("dictionary key is not a name")]
    DictKeyNotName,
    /// The same key appeared twice in one dictionary (§7.3.7 "shall
    /// not"; reader behaviour undefined — Pass 1 is strict).
    #[error("duplicate dictionary key")]
    DuplicateDictKey,
    /// Container nesting exceeded [`MAX_NESTING_DEPTH`] (pdfce guard).
    #[error("nesting exceeds MAX_NESTING_DEPTH ({MAX_NESTING_DEPTH})")]
    DepthExceeded,
    /// An indirect-object header wasn't `N G obj` with valid numbers
    /// (N positive, G in 0..=65535 — §7.3.10/§7.5.4 ranges).
    #[error("malformed indirect-object header")]
    BadObjectHeader,
    /// `endobj` missing after the object body.
    #[error("missing endobj")]
    MissingEndobj,
    /// `stream` keyword not directly preceded by a dictionary
    /// (§7.3.8.1 — a stream is a dictionary plus data).
    #[error("stream keyword without preceding dictionary")]
    StreamWithoutDict,
    /// The byte(s) after the `stream` keyword violate §7.3.8.1: must
    /// be CRLF or LF alone, never CR alone, never anything else.
    #[error("stream keyword not followed by CRLF or LF (CR alone is forbidden by \u{a7}7.3.8.1)")]
    BadStreamEol,
    /// `/Length` missing, non-integer, negative, or (when indirect)
    /// unresolvable via the caller's resolver.
    #[error("stream /Length missing, invalid, or unresolvable")]
    BadStreamLength,
    /// The `/Length`-delimited data region was not followed by
    /// (optional EOL +) `endstream` — the file's `/Length` is
    /// inconsistent (§7.3.8.2).
    #[error("endstream not found where /Length points (stream extent inconsistent)")]
    StreamExtentMismatch,
}

impl ParseError {
    const fn new(offset: usize, kind: ParseErrorKind) -> Self {
        Self { offset, kind }
    }
}

impl From<LexError> for ParseError {
    fn from(e: LexError) -> Self {
        Self {
            offset: e.offset,
            kind: ParseErrorKind::Lex(e),
        }
    }
}

/// Resolver for indirect `/Length` values (§7.3.10 EXAMPLE 3 pattern).
///
/// Given the referenced id, return the integer value of that object, or
/// `None` if it cannot be resolved / is not an integer. The document
/// layer implements this against the xref table; tests implement it
/// with a closure over a map. Kept as a trait alias-like type for
/// signature clarity.
pub type LengthResolver<'r> = &'r mut dyn FnMut(ObjId) -> Option<i64>;

/// Recursive-descent parser over a byte buffer.
///
/// Create positioned at an offset ([`Parser::at`]) and call one of the
/// top-level entry points ([`Parser::parse_object`],
/// [`Parser::parse_indirect_object`]). The parser owns a lexer plus a
/// two-token peek buffer (see module docs on the `N G R` lookahead).
#[derive(Debug)]
pub struct Parser<'a> {
    buf: &'a [u8],
    lexer: Lexer<'a>,
    /// Peeked-but-unconsumed tokens, oldest first (max 2 in practice).
    peeked: Vec<Token>,
}

impl<'a> Parser<'a> {
    /// Parser over `buf` starting at absolute offset `pos`.
    #[must_use]
    pub const fn at(buf: &'a [u8], pos: usize) -> Self {
        Self {
            buf,
            lexer: Lexer::at(buf, pos),
            peeked: Vec::new(),
        }
    }

    /// Current absolute offset for diagnostics: the start of the oldest
    /// unconsumed peeked token, or the lexer cursor.
    fn offset(&self) -> usize {
        self.peeked
            .first()
            .map_or_else(|| self.lexer.pos(), |t| t.span.start)
    }

    /// Next token, consuming.
    fn next(&mut self) -> Result<Option<Token>, ParseError> {
        if self.peeked.is_empty() {
            Ok(self.lexer.next_token()?)
        } else {
            Ok(Some(self.peeked.remove(0)))
        }
    }

    /// Peek the `n`-th upcoming token (0-based) without consuming.
    fn peek(&mut self, n: usize) -> Result<Option<&Token>, ParseError> {
        while self.peeked.len() <= n {
            match self.lexer.next_token()? {
                Some(t) => self.peeked.push(t),
                None => break,
            }
        }
        Ok(self.peeked.get(n))
    }

    /// Next token or `UnexpectedEof`.
    fn expect_any(&mut self) -> Result<Token, ParseError> {
        let off = self.offset();
        self.next()?
            .ok_or(ParseError::new(off, ParseErrorKind::UnexpectedEof))
    }

    /// Is `tok` the keyword whose lexeme is `word`? (Keywords carry no
    /// copied bytes — the span against the buffer is the value; see
    /// `lexer::TokenKind::Keyword`.)
    fn is_keyword(&self, tok: &Token, word: &[u8]) -> bool {
        matches!(tok.kind, TokenKind::Keyword) && tok.lexeme(self.buf) == Some(word)
    }

    // -----------------------------------------------------------------------
    // Direct objects
    // -----------------------------------------------------------------------

    /// Parse one direct object (any §7.3 value, including `N G R`
    /// references). This is the entry point for trailer dictionaries
    /// and other bare-value positions.
    ///
    /// # Errors
    ///
    /// [`ParseError`] on malformed syntax — see [`ParseErrorKind`].
    pub fn parse_object(&mut self) -> Result<Object, ParseError> {
        self.parse_value(0)
    }

    /// Core value parser. `depth` counts container nesting for the
    /// [`MAX_NESTING_DEPTH`] guard.
    fn parse_value(&mut self, depth: usize) -> Result<Object, ParseError> {
        if depth > MAX_NESTING_DEPTH {
            return Err(ParseError::new(
                self.offset(),
                ParseErrorKind::DepthExceeded,
            ));
        }
        let tok = self.expect_any()?;
        match tok.kind {
            TokenKind::Integer(v) => self.maybe_reference(v, &tok),
            TokenKind::Real(v) => Ok(Object::Real(v)),
            TokenKind::String(s) => Ok(Object::String(s)),
            TokenKind::Name(n) => Ok(Object::Name(Name(n))),
            TokenKind::ArrayOpen => self.parse_array_body(depth),
            TokenKind::DictOpen => Ok(Object::Dict(self.parse_dict_body(depth)?)),
            TokenKind::Keyword => {
                let lexeme = tok.lexeme(self.buf).unwrap_or(&[]);
                match lexeme {
                    b"true" => Ok(Object::Boolean(true)),
                    b"false" => Ok(Object::Boolean(false)),
                    b"null" => Ok(Object::Null),
                    _ => Err(ParseError::new(
                        tok.span.start,
                        ParseErrorKind::Unexpected("an object"),
                    )),
                }
            }
            // Closers/braces at value position are structural errors at
            // this level; the container parsers consume their own
            // closers before recursing.
            _ => Err(ParseError::new(
                tok.span.start,
                ParseErrorKind::Unexpected("an object"),
            )),
        }
    }

    /// After an `Integer`, decide reference vs plain integer via the
    /// two-token lookahead (module docs): `Integer Integer R` → a
    /// [`Object::Reference`], anything else → the integer stands alone.
    ///
    /// The reference's numbers must be in range (§7.3.10: object number
    /// positive; §7.5.4: generation ≤ 65,535) — out-of-range values
    /// mean the three tokens were NOT a reference after all, and since
    /// bare keyword `R` can't otherwise appear at value position, that
    /// is a structural error surfaced when the parser reaches it.
    fn maybe_reference(&mut self, value: i64, tok: &Token) -> Result<Object, ParseError> {
        // Copy out (kind-class, span) pairs so no peek borrow is held
        // while the buffer is sliced for the `R` check.
        let t1_is_int = self
            .peek(0)?
            .is_some_and(|t| matches!(t.kind, TokenKind::Integer(_)));
        let looks_like_ref = t1_is_int && {
            let t2 = self
                .peek(1)?
                .map(|t| (matches!(t.kind, TokenKind::Keyword), t.span));
            match t2 {
                Some((true, span)) => span.slice(self.buf) == Some(b"R"),
                _ => false,
            }
        };
        if !looks_like_ref {
            return Ok(Object::Integer(value));
        }
        // Commit: consume `gen` and `R`.
        let gen_tok = self.expect_any()?;
        let TokenKind::Integer(gen_value) = gen_tok.kind else {
            // Unreachable by construction of the lookahead; kept as a
            // structured error per the panic-free policy.
            return Err(ParseError::new(
                gen_tok.span.start,
                ParseErrorKind::Unexpected("generation number"),
            ));
        };
        self.expect_any()?; // the `R`, verified by the lookahead

        let (Ok(num), Ok(generation)) = (u32::try_from(value), u16::try_from(gen_value)) else {
            return Err(ParseError::new(
                tok.span.start,
                ParseErrorKind::Unexpected("reference numbers in range"),
            ));
        };
        if num == 0 {
            // Object number 0 is reserved for the free-list head
            // (§7.5.4); it never identifies a real object.
            return Err(ParseError::new(
                tok.span.start,
                ParseErrorKind::Unexpected("positive object number"),
            ));
        }
        Ok(Object::Reference(ObjId::new(num, generation)))
    }

    /// Parse array elements after `[`, through the matching `]`.
    fn parse_array_body(&mut self, depth: usize) -> Result<Object, ParseError> {
        let mut items = Vec::new();
        loop {
            match self.peek(0)? {
                None => {
                    return Err(ParseError::new(
                        self.offset(),
                        ParseErrorKind::UnexpectedEof,
                    ));
                }
                Some(t) if matches!(t.kind, TokenKind::ArrayClose) => {
                    self.next()?;
                    return Ok(Object::Array(items));
                }
                Some(_) => items.push(self.parse_value(depth + 1)?),
            }
        }
    }

    /// Parse dictionary entries after `<<`, through the matching `>>`.
    ///
    /// Enforces §7.3.7: keys are names; duplicate keys are malformed
    /// (spec: "shall not have the same key"; reader behaviour
    /// undefined → Pass 1 strict, real-world tolerance only with
    /// corpus evidence, per the module's failure philosophy).
    fn parse_dict_body(&mut self, depth: usize) -> Result<Dict, ParseError> {
        let mut dict = Dict::new();
        loop {
            let tok = self.expect_any()?;
            match tok.kind {
                TokenKind::DictClose => return Ok(dict),
                TokenKind::Name(key) => {
                    if dict.0.iter().any(|(k, _)| k.as_bytes() == key) {
                        return Err(ParseError::new(
                            tok.span.start,
                            ParseErrorKind::DuplicateDictKey,
                        ));
                    }
                    let value = self.parse_value(depth + 1)?;
                    dict.0.push((Name(key), value));
                }
                _ => {
                    return Err(ParseError::new(
                        tok.span.start,
                        ParseErrorKind::DictKeyNotName,
                    ));
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Indirect objects (§7.3.10) + stream bodies (§7.3.8)
    // -----------------------------------------------------------------------

    /// Parse one complete indirect-object definition
    /// (`N G obj <value> endobj`, or the stream form) starting at this
    /// parser's position — normally an offset taken from the xref
    /// table.
    ///
    /// `resolve_length` is consulted only when a stream's `/Length` is
    /// an indirect reference (§7.3.10 EXAMPLE 3); pass a closure over
    /// the xref/document (or `&mut |_| None` where indirect lengths
    /// are illegal, e.g. xref-stream bootstrap — §7.5.8.2's directness
    /// rules).
    ///
    /// # Errors
    ///
    /// [`ParseError`] on malformed structure — see [`ParseErrorKind`].
    pub fn parse_indirect_object(
        &mut self,
        resolve_length: LengthResolver<'_>,
    ) -> Result<IndirectObject, ParseError> {
        // --- header: N G obj ---
        let num_tok = self.expect_any()?;
        let TokenKind::Integer(num) = num_tok.kind else {
            return Err(ParseError::new(
                num_tok.span.start,
                ParseErrorKind::BadObjectHeader,
            ));
        };
        let gen_tok = self.expect_any()?;
        let TokenKind::Integer(gen_value) = gen_tok.kind else {
            return Err(ParseError::new(
                gen_tok.span.start,
                ParseErrorKind::BadObjectHeader,
            ));
        };
        let obj_tok = self.expect_any()?;
        if !self.is_keyword(&obj_tok, b"obj") {
            return Err(ParseError::new(
                obj_tok.span.start,
                ParseErrorKind::BadObjectHeader,
            ));
        }
        let (Ok(num), Ok(generation)) = (u32::try_from(num), u16::try_from(gen_value)) else {
            return Err(ParseError::new(
                num_tok.span.start,
                ParseErrorKind::BadObjectHeader,
            ));
        };
        if num == 0 {
            return Err(ParseError::new(
                num_tok.span.start,
                ParseErrorKind::BadObjectHeader,
            ));
        }
        let id = ObjId::new(num, generation);

        // --- body ---
        let value = self.parse_value(0)?;

        // --- terminator: endobj, or stream … endstream endobj ---
        let term = self.expect_any()?;
        if self.is_keyword(&term, b"endobj") {
            return Ok(IndirectObject {
                id,
                value,
                provenance: Provenance::File(ByteSpan::from_range(
                    num_tok.span.start..term.span.end(),
                )),
            });
        }
        if self.is_keyword(&term, b"stream") {
            let Object::Dict(dict) = value else {
                return Err(ParseError::new(
                    term.span.start,
                    ParseErrorKind::StreamWithoutDict,
                ));
            };
            let stream = self.parse_stream_tail(dict, &term, resolve_length)?;
            let end_tok = self.expect_any()?;
            if !self.is_keyword(&end_tok, b"endobj") {
                return Err(ParseError::new(
                    end_tok.span.start,
                    ParseErrorKind::MissingEndobj,
                ));
            }
            return Ok(IndirectObject {
                id,
                value: Object::Stream(stream),
                provenance: Provenance::File(ByteSpan::from_range(
                    num_tok.span.start..end_tok.span.end(),
                )),
            });
        }
        Err(ParseError::new(
            term.span.start,
            ParseErrorKind::MissingEndobj,
        ))
    }

    /// Handle everything after a `stream` keyword: the §7.3.8.1 EOL
    /// rule, the `/Length`-delimited data span, and the `endstream`
    /// keyword. Returns with the parser positioned after `endstream`.
    fn parse_stream_tail(
        &mut self,
        dict: Dict,
        stream_kw: &Token,
        resolve_length: LengthResolver<'_>,
    ) -> Result<Stream, ParseError> {
        // The peek buffer must be empty here: `stream` was just
        // consumed and stream *data* must not be lexed. (It is empty by
        // construction — parse_value never leaves lookahead beyond the
        // value it returned, and the `stream` token itself was taken
        // with expect_any. debug_assert documents the reasoning.)
        debug_assert!(self.peeked.is_empty());

        // §7.3.8.1: `stream` followed by CRLF or LF alone; CR alone is
        // FORBIDDEN. Data begins at the byte after that EOL.
        let after_kw = stream_kw.span.end();
        let data_start = match (self.buf.get(after_kw), self.buf.get(after_kw + 1)) {
            (Some(b'\r'), Some(b'\n')) => after_kw + 2,
            (Some(b'\n'), _) => after_kw + 1,
            _ => {
                return Err(ParseError::new(after_kw, ParseErrorKind::BadStreamEol));
            }
        };

        // /Length: required, integer, non-negative; possibly indirect.
        let length = match dict.get(b"Length") {
            Some(Object::Integer(v)) => *v,
            Some(Object::Reference(id)) => resolve_length(*id).ok_or(ParseError::new(
                stream_kw.span.start,
                ParseErrorKind::BadStreamLength,
            ))?,
            _ => {
                return Err(ParseError::new(
                    stream_kw.span.start,
                    ParseErrorKind::BadStreamLength,
                ));
            }
        };
        let Ok(length) = usize::try_from(length) else {
            return Err(ParseError::new(
                stream_kw.span.start,
                ParseErrorKind::BadStreamLength,
            ));
        };

        let data_end = data_start.saturating_add(length);
        if data_end > self.buf.len() {
            return Err(ParseError::new(
                data_start,
                ParseErrorKind::StreamExtentMismatch,
            ));
        }

        // After the data: optional EOL ("should", not counted in
        // Length), then `endstream`. Re-enter token scanning there —
        // the lexer's whitespace skipping absorbs the optional EOL.
        self.lexer = Lexer::at(self.buf, data_end);
        let end_tok = self.expect_any().map_err(|e| match e.kind {
            // EOF right after the data reads better as an extent error.
            ParseErrorKind::UnexpectedEof => {
                ParseError::new(data_end, ParseErrorKind::StreamExtentMismatch)
            }
            _ => e,
        })?;
        if !self.is_keyword(&end_tok, b"endstream") {
            return Err(ParseError::new(
                end_tok.span.start,
                ParseErrorKind::StreamExtentMismatch,
            ));
        }

        Ok(Stream {
            dict,
            data_span: ByteSpan::from_range(data_start..data_end),
        })
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;

    fn parse(input: &[u8]) -> Object {
        Parser::at(input, 0).parse_object().unwrap()
    }

    fn parse_err(input: &[u8]) -> ParseErrorKind {
        Parser::at(input, 0).parse_object().unwrap_err().kind
    }

    fn no_lengths(_: ObjId) -> Option<i64> {
        None
    }

    // ---- scalars and containers ----

    #[test]
    fn scalars() {
        assert_eq!(parse(b"true"), Object::Boolean(true));
        assert_eq!(parse(b"false"), Object::Boolean(false));
        assert_eq!(parse(b"null"), Object::Null);
        assert_eq!(parse(b"42"), Object::Integer(42));
        assert_eq!(parse(b"4."), Object::Real(4.0));
        assert_eq!(parse(b"(hi)"), Object::String(b"hi".to_vec()));
        assert_eq!(parse(b"/Type"), Object::Name(Name::from(b"Type")));
    }

    #[test]
    fn heterogeneous_array_spec_example() {
        // §7.3.6 EXAMPLE.
        let Object::Array(a) = parse(b"[ 549 3.14 false (Ralph) /SomeName ]") else {
            panic!("not an array");
        };
        assert_eq!(a.len(), 5);
        assert_eq!(a[0], Object::Integer(549));
        assert_eq!(a[2], Object::Boolean(false));
        assert_eq!(a[4], Object::Name(Name::from(b"SomeName")));
    }

    #[test]
    fn nested_dict() {
        let obj = parse(b"<< /A << /B [1 2] >> /C null >>");
        let d = obj.as_dict().unwrap();
        let inner = d.get(b"A").unwrap().as_dict().unwrap();
        assert_eq!(inner.get(b"B").unwrap().as_array().unwrap().len(), 2);
        // §7.3.7: null value ≡ absent.
        assert!(d.get(b"C").is_none());
    }

    // ---- the N G R lookahead ----

    #[test]
    fn reference_lookahead_in_array() {
        // §7.3.10: `[1 0 R 2 0 R]` is two references, not six values.
        let Object::Array(a) = parse(b"[1 0 R 2 0 R]") else {
            panic!("not an array");
        };
        assert_eq!(
            a,
            vec![
                Object::Reference(ObjId::new(1, 0)),
                Object::Reference(ObjId::new(2, 0)),
            ]
        );
    }

    #[test]
    fn integers_that_are_not_references_stay_integers() {
        // Two integers followed by a non-R token: plain integers.
        let Object::Array(a) = parse(b"[1 0 3]") else {
            panic!("not an array");
        };
        assert_eq!(
            a,
            vec![Object::Integer(1), Object::Integer(0), Object::Integer(3)]
        );
        // Trailing pair at EOF (no third token): plain integers.
        let Object::Array(a) = parse(b"[1 0]") else {
            panic!("not an array");
        };
        assert_eq!(a.len(), 2);
    }

    #[test]
    fn reference_as_dict_value() {
        let obj = parse(b"<< /Root 2 0 R >>");
        let d = obj.as_dict().unwrap();
        assert_eq!(
            d.get(b"Root").unwrap().as_reference(),
            Some(ObjId::new(2, 0))
        );
    }

    // ---- strictness ----

    #[test]
    fn duplicate_dict_key_is_error() {
        assert_eq!(
            parse_err(b"<< /A 1 /A 2 >>"),
            ParseErrorKind::DuplicateDictKey
        );
    }

    #[test]
    fn non_name_dict_key_is_error() {
        assert_eq!(parse_err(b"<< 1 2 >>"), ParseErrorKind::DictKeyNotName);
    }

    #[test]
    fn depth_guard_trips() {
        let mut bomb = vec![b'['; MAX_NESTING_DEPTH + 8];
        bomb.extend_from_slice(&vec![b']'; MAX_NESTING_DEPTH + 8]);
        assert_eq!(parse_err(&bomb), ParseErrorKind::DepthExceeded);
    }

    #[test]
    fn unclosed_array_is_eof_error() {
        assert_eq!(parse_err(b"[1 2"), ParseErrorKind::UnexpectedEof);
    }

    #[test]
    fn stray_keyword_at_value_position_is_error() {
        assert!(matches!(
            parse_err(b"frobnicate"),
            ParseErrorKind::Unexpected(_)
        ));
    }

    // ---- indirect objects ----

    #[test]
    fn indirect_object_spec_example_and_span() {
        // §7.3.10 EXAMPLE 1 — and the span covers the FULL definition
        // (the provenance contract, decision 001 §6.1 item 1).
        let buf: &[u8] = b"12 0 obj\n    (Brillig)\nendobj";
        let io = Parser::at(buf, 0)
            .parse_indirect_object(&mut no_lengths)
            .unwrap();
        assert_eq!(io.id, ObjId::new(12, 0));
        assert_eq!(io.value, Object::String(b"Brillig".to_vec()));
        assert_eq!(io.file_span().unwrap().slice(buf).unwrap(), buf);
    }

    #[test]
    fn stream_with_direct_length() {
        let buf: &[u8] = b"5 0 obj << /Length 9 >>\nstream\nsome data\nendstream endobj";
        let io = Parser::at(buf, 0)
            .parse_indirect_object(&mut no_lengths)
            .unwrap();
        let Object::Stream(s) = &io.value else {
            panic!("not a stream");
        };
        assert_eq!(s.data_span.slice(buf).unwrap(), b"some data");
        assert_eq!(io.file_span().unwrap().slice(buf).unwrap(), buf);
    }

    #[test]
    fn stream_with_crlf_after_keyword() {
        let buf: &[u8] = b"5 0 obj << /Length 4 >>\nstream\r\nabcd\nendstream endobj";
        let io = Parser::at(buf, 0)
            .parse_indirect_object(&mut no_lengths)
            .unwrap();
        let Object::Stream(s) = &io.value else {
            panic!("not a stream");
        };
        assert_eq!(s.data_span.slice(buf).unwrap(), b"abcd");
    }

    #[test]
    fn stream_cr_alone_after_keyword_is_error() {
        // §7.3.8.1: "and NOT by CR alone."
        let buf: &[u8] = b"5 0 obj << /Length 4 >>\nstream\rabcd\nendstream endobj";
        let e = Parser::at(buf, 0)
            .parse_indirect_object(&mut no_lengths)
            .unwrap_err();
        assert_eq!(e.kind, ParseErrorKind::BadStreamEol);
    }

    #[test]
    fn stream_with_indirect_length_resolves_via_callback() {
        // §7.3.10 EXAMPLE 3's single-pass-writer pattern.
        let buf: &[u8] = b"7 0 obj << /Length 8 0 R >>\nstream\n0123456\nendstream endobj";
        let mut resolver = |id: ObjId| (id == ObjId::new(8, 0)).then_some(7);
        let io = Parser::at(buf, 0)
            .parse_indirect_object(&mut resolver)
            .unwrap();
        let Object::Stream(s) = &io.value else {
            panic!("not a stream");
        };
        assert_eq!(s.data_span.slice(buf).unwrap(), b"0123456");
    }

    #[test]
    fn stream_wrong_length_is_extent_mismatch() {
        // §7.3.8.2: inconsistent extent "is an error" — no silent
        // endstream scanning in Pass 1.
        let buf: &[u8] = b"5 0 obj << /Length 3 >>\nstream\nsome data\nendstream endobj";
        let e = Parser::at(buf, 0)
            .parse_indirect_object(&mut no_lengths)
            .unwrap_err();
        assert_eq!(e.kind, ParseErrorKind::StreamExtentMismatch);
    }

    #[test]
    fn stream_missing_length_is_error() {
        let buf: &[u8] = b"5 0 obj << >>\nstream\nxx\nendstream endobj";
        let e = Parser::at(buf, 0)
            .parse_indirect_object(&mut no_lengths)
            .unwrap_err();
        assert_eq!(e.kind, ParseErrorKind::BadStreamLength);
    }

    #[test]
    fn object_number_zero_is_rejected() {
        // §7.5.4: object 0 is permanently the free-list head.
        let buf: &[u8] = b"0 0 obj null endobj";
        let e = Parser::at(buf, 0)
            .parse_indirect_object(&mut no_lengths)
            .unwrap_err();
        assert_eq!(e.kind, ParseErrorKind::BadObjectHeader);
    }

    #[test]
    fn missing_endobj_is_error() {
        let buf: &[u8] = b"3 0 obj 42 trailer";
        let e = Parser::at(buf, 0)
            .parse_indirect_object(&mut no_lengths)
            .unwrap_err();
        assert_eq!(e.kind, ParseErrorKind::MissingEndobj);
    }
}
