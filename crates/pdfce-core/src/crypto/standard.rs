//! The `/Standard` security handler — ISO 32000-1 §7.6.3, Algorithms 1–7.
//!
//! This module answers three questions and nothing else:
//!
//! 1. **What does this `/Encrypt` dictionary ask for**, and is it something
//!    pdfce can do? ([`EncryptionConfig::parse`])
//! 2. **Does this password open it**, and what is the file encryption key?
//!    ([`EncryptionConfig::authenticate`])
//! 3. **What key decrypts the string/stream in object `N G`?**
//!    ([`FileKey::object_key`], Algorithm 1)
//!
//! It does not touch the parser, does not know what a page is, and does not
//! decide policy — the caller decides what to do with a refusal or with a
//! permission bit. That separation is deliberate: every trap in clause 7.6 is
//! a *derivation* trap, and derivation is exactly what is testable in
//! isolation against a fixture whose password is known.
//!
//! # Scope of this increment: RC4 only
//!
//! Implemented: `/V` 1, 2 and 4 at `/R` 2, 3 and 4 with `/CFM /V2` — i.e.
//! **RC4 at 40–128 bits**. Refused, by name, with the reason stated:
//!
//! | Configuration | Why refused |
//! |---|---|
//! | `/CFM /AESV2` | AES-128 is the next increment; the cipher is missing, not the plumbing |
//! | `/V 5` (`/R` 5, 6) | AES-256; `/R 6`'s Algorithm 2.B is additionally **unsourced** past step (a) |
//! | `/V 0`, `/V 3` | `/V 3` is an *unpublished* algorithm that "shall not appear in a conforming PDF file"; `/V 0` is undocumented. Nobody can open these |
//! | `/Filter` ≠ `/Standard` | Public-key and third-party handlers |
//!
//! A refusal names the configuration rather than saying "encrypted files are
//! not supported", because those are very different facts to an operator
//! holding a file that Chrome opens.
//!
//! # The five traps this module is built around
//!
//! Transcribed from `iso32000__ref__encryption_impl.md` §C; each produces a
//! **silently wrong key**, i.e. a file that fails to open with the right
//! password and gives no hint why.
//!
//! - **T9/T13** — Algorithm 2 step (h) truncates the digest to `n` bytes
//!   *between* its 50 rounds. Algorithm 3 step (c) runs the same 50 rounds and
//!   does **not** truncate. Two loops, three pages apart, opposite rules.
//! - **T10** — `/P` is hashed as an *unsigned little-endian 32-bit* value but
//!   stored as a *signed* PDF integer. `-44` hashes as `D4 FF FF FF`.
//! - **T11** — Algorithm 2 step (f) fires when `/EncryptMetadata` is **false**
//!   and only at `/R` ≥ 4. The default is `true`, so most files skip it.
//! - **T15** — at `/R` ≥ 3 the last 16 bytes of `/U` are *arbitrary padding*.
//!   Comparing all 32 rejects every conforming file.
//! - **T16** — Algorithm 3's RC4 loop counts **1 → 19**; Algorithm 7's counts
//!   **19 → 0** (twenty rounds, the last with key XOR 0, undoing Algorithm 3's
//!   plain pass). Reversing them fails silently.
//! - **T2** — Algorithm 1 truncates the object number to its **3** low bytes
//!   and the generation to **2**, little-endian. Normative, not a bug.
//!
//! # Authentication order is not ours to choose
//!
//! §7.6.3.1 requires trying the **empty user password** first, silently. A
//! file with an empty user password and a non-empty owner password — the
//! "permissions-only" PDF, the common case — opens with no prompt in every
//! conforming reader. Prompting for it would read to an operator as pdfce
//! failing to open a file that every other viewer opens. [`Self::authenticate`]
//! takes the password as `Option`, and `None` means "try the empty one",
//! which is a different thing from the user typing an empty box.
//!
//! # Permissions are disclosed, never silently enforced
//!
//! §7.6.3.1, verbatim: *"There is nothing inherent in PDF encryption that
//! enforces the document permissions."* The bits are the author's stated
//! intent, are trivially editable in the plaintext `/P`, and carry no
//! integrity protection at `/V` 1–4 (**N7**). ISO 32000-1 also specifies no
//! mapping from a bit to a *reader operation* (**N4**) — "assemble the
//! document" is not an object-level predicate. So [`Permissions`] reports what
//! the document asks for and pdfce shows it; which pdfce operations a bit
//! gates is a product decision that belongs in the shells, disclosed under
//! rule 4, not buried here.

use crate::crypto::md5::{Md5, md5};
use crate::crypto::rc4::rc4;
use crate::object::{Dict, ObjId, Object};

/// The 32-byte padding string, ISO 32000-1 §7.6.3.3 (Algorithm 2 step (a)).
///
/// Transcribed from the printed clause. Used three ways: to pad a short
/// password, *as* the password when there is none, and as the plaintext
/// Algorithm 4 encrypts to produce `/U`.
pub const PADDING: [u8; 32] = [
    0x28, 0xBF, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41, 0x64, 0x00, 0x4E, 0x56, 0xFF, 0xFA, 0x01, 0x08,
    0x2E, 0x2E, 0x00, 0xB6, 0xD0, 0x68, 0x3E, 0x80, 0x2F, 0x0C, 0xA9, 0xFE, 0x64, 0x53, 0x69, 0x7A,
];

/// Why pdfce will not decrypt a particular document.
///
/// Every variant names the *configuration*, not the capability. An operator
/// holding a file that another viewer opens needs to know which of "pdfce
/// hasn't implemented this yet", "this is a different security handler" and
/// "no conforming reader may open this" applies — those have three different
/// next actions.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EncryptionUnsupported {
    /// `/Filter` names a handler other than `/Standard`.
    ///
    /// The public-key handler (`/SubFilter` beginning `adbe.pkcs7.`) lands
    /// here; so does any third-party handler.
    #[error("security handler /{0} is not the standard password handler")]
    Handler(String),

    /// `/Filter` is absent.
    ///
    /// Table 20 makes `/Filter` **required**: *"If this entry is absent,
    /// other security handlers shall not decrypt the document."* This is a
    /// conformance-correct refusal — no reader is permitted to open it.
    #[error("/Encrypt has no /Filter; Table 20 forbids any handler from decrypting this document")]
    NoFilter,

    /// `/V` 0 (including absent, whose default is 0) or `/V` 3.
    ///
    /// `/V` 0 is "undocumented"; `/V` 3 is "an unpublished algorithm … shall
    /// not appear in a conforming PDF file". Neither is openable by anyone
    /// except the producer.
    #[error(
        "/V {0} is an undocumented or unpublished algorithm that no conforming reader can open"
    )]
    UndocumentedAlgorithm(i64),

    /// AES, at any key length. A capability gap with a known shape.
    #[error("{0} encryption is not implemented yet (this increment covers RC4 only)")]
    CipherNotImplemented(&'static str),

    /// `/R` 6's Algorithm 2.B is not sourced in the project's spec corpus
    /// past step (a), so an implementation would be guesswork.
    ///
    /// Distinguished from `/R` 5 in the message on purpose: `/R` 5 is a
    /// sourced algorithm pdfce simply has not written, while `/R` 6 is a
    /// *sourcing* gap. Those resolve differently.
    #[error(
        "/R 6 (AES-256, hardened) cannot be implemented: its key-derivation algorithm is not available in the project's spec corpus"
    )]
    UnsourcedRevision,

    /// `/CFM` outside the four names Table 25 defines.
    ///
    /// Table 25 puts a `shall` on the *diagnostic* here, unusually:
    /// applications "shall report that the file is encrypted with an
    /// unsupported algorithm".
    #[error("crypt filter method /{0} is not one of None, V2, AESV2, AESV3")]
    UnknownCfm(String),

    /// The `/Encrypt` dictionary is missing an entry the algorithms need, or
    /// holds a value of the wrong type.
    ///
    /// **N6**: ISO 32000-1 states no error model for this at all — no clause
    /// says what a reader should do when `/O` is the wrong length or `/R`
    /// disagrees with `/V`. Refusing with the specific field named is pdfce
    /// policy.
    #[error("/Encrypt is malformed: {0}")]
    Malformed(&'static str),
}

/// The cipher a crypt filter selects (Table 25's `/CFM`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cipher {
    /// `/None` — the security handler decrypts privately. pdfce cannot know
    /// how, so a document that routes real content through it is refused
    /// upstream; `Identity`-like passthrough is the only safe reading.
    None,
    /// `/V2` — RC4 with the file key length. **Implemented.**
    Rc4,
    /// `/AESV2` — AES-128 in CBC mode. Recognised, not implemented.
    Aes128,
    /// `/AESV3` — AES-256 in CBC mode. Recognised, not implemented.
    Aes256,
}

/// Access permissions as the document's author stated them (Table 22).
///
/// **This is a report, not an enforcement mechanism.** See the module docs:
/// the bits are unauthenticated at `/V` 1–4 and the standard explicitly
/// disclaims enforcement. Fields are named for what Table 22 says they
/// control, not for pdfce operations, because ISO 32000-1 defines no mapping
/// between the two (**N4**).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Permissions {
    /// The raw flag word, unsigned. Preserved because a save must re-emit it
    /// verbatim — the bits pdfce ignores still feed Algorithm 2's hash, so
    /// "normalising" them would break authentication (**N3**).
    pub raw: u32,
    /// Revision, which decides whether bits 9–12 are meaningful at all.
    pub revision: u8,
}

impl Permissions {
    /// Bit `n` (1-based, as Table 22 numbers them).
    fn bit(self, n: u32) -> bool {
        self.raw & (1 << (n - 1)) != 0
    }

    /// Bit 3 — print the document.
    ///
    /// At `/R` ≥ 3 this means "print, possibly at degraded quality"; bit 12
    /// controls whether full-fidelity printing is allowed.
    #[must_use]
    pub fn print(self) -> bool {
        self.bit(3)
    }

    /// Bit 4 — modify contents, other than what bits 6, 9 and 11 control.
    #[must_use]
    pub fn modify_contents(self) -> bool {
        self.bit(4)
    }

    /// Bit 5 — copy or extract text and graphics.
    ///
    /// At `/R` 2 this subsumes accessibility extraction; at `/R` ≥ 3 that
    /// splits out into bit 10.
    #[must_use]
    pub fn copy(self) -> bool {
        self.bit(5)
    }

    /// Bit 6 — add or modify annotations and fill form fields; with bit 4
    /// also set, create or modify form fields.
    #[must_use]
    pub fn annotate(self) -> bool {
        self.bit(6)
    }

    /// Bit 9 — fill in existing form fields, even if bit 6 is clear.
    /// Meaningful only at `/R` ≥ 3; `false` below that (the bit is reserved).
    #[must_use]
    pub fn fill_forms(self) -> bool {
        self.revision >= 3 && self.bit(9)
    }

    /// Bit 10 — extract text and graphics for accessibility.
    /// Meaningful only at `/R` ≥ 3.
    #[must_use]
    pub fn accessibility_extract(self) -> bool {
        self.revision >= 3 && self.bit(10)
    }

    /// Bit 11 — assemble: insert, rotate, delete pages; create bookmarks and
    /// thumbnails. Even if bit 4 is clear. Meaningful only at `/R` ≥ 3.
    #[must_use]
    pub fn assemble(self) -> bool {
        self.revision >= 3 && self.bit(11)
    }

    /// Bit 12 — print to a representation from which a faithful digital copy
    /// could be generated. Meaningful only at `/R` ≥ 3.
    #[must_use]
    pub fn print_high_quality(self) -> bool {
        self.revision >= 3 && self.bit(12)
    }

    /// Whether `bit` is granted — the iterable form of the accessors above.
    ///
    /// Returns `None` when the bit carries no meaning at this document's
    /// revision, which a front end must render differently from `Some(false)`:
    /// "the author did not permit this" and "this document's encryption
    /// revision has no such concept" are different statements, and collapsing
    /// them shows the operator a restriction nobody wrote.
    #[must_use]
    pub fn granted(self, bit: PermissionBit) -> Option<bool> {
        if bit.applies_at(self.revision) {
            Some(self.bit(bit.position()))
        } else {
            None
        }
    }
}

/// One permission a document's author may declare (Table 22).
///
/// Enumerated so a front end can iterate the whole set and show a complete
/// picture. A partial list would be worse than none: an operator seeing four
/// permissions cannot tell whether the other four were omitted because they
/// are allowed, because they are absent, or because nobody implemented them.
///
/// Ordered as Table 22 orders the bits, with the two print entries adjacent
/// because they are read together — bit 3 is "may print", bit 12 is "may
/// print at full quality", and bit 12 without bit 3 means nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PermissionBit {
    /// Bit 3 — print the document.
    Print,
    /// Bit 12 — print at full fidelity. `/R` 3+ only.
    PrintHighQuality,
    /// Bit 4 — modify contents, other than what bits 6, 9 and 11 govern.
    ModifyContents,
    /// Bit 5 — copy or extract text and graphics.
    Copy,
    /// Bit 6 — add or modify annotations and fill form fields.
    Annotate,
    /// Bit 9 — fill existing form fields even if bit 6 is clear. `/R` 3+ only.
    FillForms,
    /// Bit 10 — extract for accessibility. `/R` 3+ only.
    AccessibilityExtract,
    /// Bit 11 — insert, rotate or delete pages. `/R` 3+ only.
    Assemble,
}

impl PermissionBit {
    /// Every permission, in Table 22 order.
    #[must_use]
    pub const fn all() -> [Self; 8] {
        [
            Self::Print,
            Self::PrintHighQuality,
            Self::ModifyContents,
            Self::Copy,
            Self::Annotate,
            Self::FillForms,
            Self::AccessibilityExtract,
            Self::Assemble,
        ]
    }

    /// The 1-based bit position Table 22 assigns.
    #[must_use]
    pub const fn position(self) -> u32 {
        match self {
            Self::Print => 3,
            Self::ModifyContents => 4,
            Self::Copy => 5,
            Self::Annotate => 6,
            Self::FillForms => 9,
            Self::AccessibilityExtract => 10,
            Self::Assemble => 11,
            Self::PrintHighQuality => 12,
        }
    }

    /// Whether this bit carries any meaning at handler revision `revision`.
    ///
    /// Bits 9–12 were introduced at `/R` 3. Below that they are reserved, and
    /// **reporting a reserved bit as "not allowed" would invent a restriction
    /// the document never expressed** — the author of an `/R` 2 file did not
    /// decline to permit form-filling; the concept did not exist to decline.
    #[must_use]
    pub const fn applies_at(self, revision: u8) -> bool {
        match self {
            Self::Print | Self::ModifyContents | Self::Copy | Self::Annotate => true,
            Self::FillForms
            | Self::AccessibilityExtract
            | Self::Assemble
            | Self::PrintHighQuality => revision >= 3,
        }
    }
}

/// Which password opened the document.
///
/// The distinction matters because the owner password grants full access
/// regardless of `/P` (§7.6.3.1), while the user password — and the empty
/// default password — grant `/P`-limited access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthKind {
    /// The empty user password authenticated, so no prompt was shown. This is
    /// the "permissions-only" document.
    EmptyUser,
    /// A supplied user password authenticated.
    User,
    /// An owner password authenticated. Full access; `/P` is advisory only.
    Owner,
}

/// A parsed, supported `/Encrypt` dictionary.
///
/// Constructing one is the whole of "can pdfce open this?"; it holds no key
/// and proves no password. [`Self::authenticate`] is the next step.
#[derive(Debug, Clone)]
pub struct EncryptionConfig {
    /// `/V` — the algorithm family.
    pub version: i64,
    /// `/R` — the handler revision, which selects between Algorithms 4 and 5,
    /// decides whether Algorithm 2's 50-round loop runs, and decides whether
    /// bits 9–12 of `/P` mean anything.
    pub revision: u8,
    /// File encryption key length in **bytes** (`n` in the algorithms).
    /// 5 at `/R` 2; `/Length` ÷ 8 above that.
    pub key_len: usize,
    /// `/O`, 32 bytes. Opaque; never re-derived on save (**R33**, T15).
    pub o: Vec<u8>,
    /// `/U`, 32 bytes. Opaque; only the first 16 are ever compared (T15).
    pub u: Vec<u8>,
    /// `/P` as an unsigned bit field (T10, A5).
    pub p: u32,
    /// `/EncryptMetadata`. Default `true`; `false` adds four `0xFF` bytes to
    /// Algorithm 2's hash at `/R` ≥ 4 (T11).
    pub encrypt_metadata: bool,
    /// The cipher for stream data (`/StmF`'s filter).
    pub stream_cipher: Cipher,
    /// The cipher for strings (`/StrF`'s filter). Independent of the stream
    /// cipher — a document may leave one in the clear via `/Identity`.
    pub string_cipher: Cipher,
}

impl EncryptionConfig {
    /// Parse an `/Encrypt` dictionary, refusing anything not implemented.
    ///
    /// `resolve` looks up an indirect reference, because `/O`, `/U` and the
    /// crypt-filter dictionaries **may** be indirect even though the
    /// `/Encrypt` dictionary itself must be direct in the trailer. It returns
    /// `None` for a dangling reference, matching §7.3.10's "shall not be
    /// considered an error".
    ///
    /// # Errors
    ///
    /// [`EncryptionUnsupported`] naming the specific configuration.
    ///
    /// Slicing is in bounds by construction: `/O` and `/U` are sliced to 32
    /// bytes only inside a `s.len() >= 32` guard, and the shorter case is a
    /// named refusal rather than a truncation.
    #[allow(clippy::indexing_slicing)]
    pub fn parse(
        dict: &Dict,
        resolve: &dyn Fn(ObjId) -> Option<Object>,
    ) -> Result<Self, EncryptionUnsupported> {
        let get = |key: &[u8]| -> Option<Object> {
            match dict.get(key) {
                Some(Object::Reference(id)) => resolve(*id),
                Some(other) => Some(other.clone()),
                None => None,
            }
        };

        // /Filter is Required (Table 20). Its absence is not "assume
        // Standard" — it is a document no handler may decrypt.
        match get(b"Filter") {
            Some(Object::Name(n)) => {
                if n.as_bytes() != b"Standard" {
                    return Err(EncryptionUnsupported::Handler(
                        String::from_utf8_lossy(n.as_bytes()).into_owned(),
                    ));
                }
            }
            _ => return Err(EncryptionUnsupported::NoFilter),
        }

        // /V defaults to 0, which is itself a refusal (Table 20: "shall not
        // be used"). Reading a missing /V as "probably 1" would open the door
        // to guessing at a document nobody can open.
        let version = match get(b"V") {
            Some(Object::Integer(v)) => v,
            None => 0,
            Some(_) => return Err(EncryptionUnsupported::Malformed("/V is not an integer")),
        };
        if version == 0 || version == 3 {
            return Err(EncryptionUnsupported::UndocumentedAlgorithm(version));
        }

        let revision = match get(b"R") {
            Some(Object::Integer(r)) if (2..=6).contains(&r) => r as u8,
            Some(Object::Integer(r)) => {
                return Err(EncryptionUnsupported::UndocumentedAlgorithm(r));
            }
            _ => {
                return Err(EncryptionUnsupported::Malformed(
                    "/R is missing or not an integer",
                ));
            }
        };
        if revision == 6 {
            return Err(EncryptionUnsupported::UnsourcedRevision);
        }
        if revision == 5 || version == 5 {
            return Err(EncryptionUnsupported::CipherNotImplemented("AES-256"));
        }

        // `n`, the file key length in bytes. §7.6.3 Algorithm 2 step (i):
        // "shall always be 5 for security handlers of revision 2" — the
        // /Length entry does not apply there even if present.
        let key_len = if revision == 2 {
            5
        } else {
            match get(b"Length") {
                Some(Object::Integer(bits)) if (40..=128).contains(&bits) && bits % 8 == 0 => {
                    (bits / 8) as usize
                }
                // Absent /Length defaults to 40 bits (Table 20).
                None => 5,
                Some(_) => {
                    return Err(EncryptionUnsupported::Malformed(
                        "/Length is not a multiple of 8 in 40..=128",
                    ));
                }
            }
        };

        let o = match get(b"O") {
            Some(Object::String(s)) if s.len() >= 32 => s[..32].to_vec(),
            Some(Object::String(_)) => {
                return Err(EncryptionUnsupported::Malformed(
                    "/O is shorter than 32 bytes",
                ));
            }
            _ => {
                return Err(EncryptionUnsupported::Malformed(
                    "/O is missing or not a string",
                ));
            }
        };
        let u = match get(b"U") {
            Some(Object::String(s)) if s.len() >= 32 => s[..32].to_vec(),
            Some(Object::String(_)) => {
                return Err(EncryptionUnsupported::Malformed(
                    "/U is shorter than 32 bytes",
                ));
            }
            _ => {
                return Err(EncryptionUnsupported::Malformed(
                    "/U is missing or not a string",
                ));
            }
        };

        // T10/A5: stored signed, hashed unsigned. The cast is the whole of
        // the fix, and it must happen exactly here — a `/P` that reaches
        // Algorithm 2 as an `i64` produces a different hash and an
        // unexplainable authentication failure.
        let p = match get(b"P") {
            Some(Object::Integer(v)) => v as i32 as u32,
            _ => {
                return Err(EncryptionUnsupported::Malformed(
                    "/P is missing or not an integer",
                ));
            }
        };

        let encrypt_metadata = match get(b"EncryptMetadata") {
            Some(Object::Boolean(b)) => b,
            // Default true (Table 21). Absence is not "false".
            _ => true,
        };

        // At /V < 4 there are no crypt filters; the whole document is RC4.
        let (stream_cipher, string_cipher) = if version < 4 {
            (Cipher::Rc4, Cipher::Rc4)
        } else {
            let cf = match get(b"CF") {
                Some(Object::Dict(d)) => d,
                // /V 4 with no /CF means every filter name resolves to
                // Identity (Table 26), i.e. nothing is encrypted. Legal, and
                // handled by the lookup below returning None.
                _ => Dict::new(),
            };
            let named = |which: &[u8]| -> Result<Cipher, EncryptionUnsupported> {
                let name = match get(which) {
                    Some(Object::Name(n)) => n.as_bytes().to_vec(),
                    // Table 20: /StmF and /StrF default to /Identity.
                    _ => b"Identity".to_vec(),
                };
                if name == b"Identity" {
                    return Ok(Cipher::None);
                }
                let entry = match cf.get(&name) {
                    Some(Object::Reference(id)) => resolve(*id),
                    Some(other) => Some(other.clone()),
                    None => None,
                };
                let Some(Object::Dict(fd)) = entry else {
                    // N10: the standard is silent on a /StmF naming a filter
                    // absent from /CF. Treating it as Identity would silently
                    // hand ciphertext to the content parser, so refuse.
                    return Err(EncryptionUnsupported::Malformed(
                        "/StmF or /StrF names a crypt filter absent from /CF",
                    ));
                };
                match fd.get(b"CFM") {
                    Some(Object::Name(n)) => match n.as_bytes() {
                        b"None" => Ok(Cipher::None),
                        b"V2" => Ok(Cipher::Rc4),
                        b"AESV2" => Ok(Cipher::Aes128),
                        b"AESV3" => Ok(Cipher::Aes256),
                        other => Err(EncryptionUnsupported::UnknownCfm(
                            String::from_utf8_lossy(other).into_owned(),
                        )),
                    },
                    // Table 25: /CFM defaults to /None.
                    _ => Ok(Cipher::None),
                }
            };
            (named(b"StmF")?, named(b"StrF")?)
        };

        for c in [stream_cipher, string_cipher] {
            match c {
                Cipher::Aes128 => {
                    return Err(EncryptionUnsupported::CipherNotImplemented("AES-128"));
                }
                Cipher::Aes256 => {
                    return Err(EncryptionUnsupported::CipherNotImplemented("AES-256"));
                }
                Cipher::None | Cipher::Rc4 => {}
            }
        }

        Ok(Self {
            version,
            revision,
            key_len,
            o,
            u,
            p,
            encrypt_metadata,
            stream_cipher,
            string_cipher,
        })
    }

    /// The permissions the document's author declared.
    #[must_use]
    pub fn permissions(&self) -> Permissions {
        Permissions {
            raw: self.p,
            revision: self.revision,
        }
    }

    /// Algorithm 2 — compute the file encryption key from a *user* password.
    ///
    /// `id0` is the first element of the trailer `/ID` array. It is hashed
    /// unconditionally by step (e); a file with no `/ID` hashes nothing there,
    /// which is what an empty slice gives.
    ///
    /// The two traps live in the last four lines: step (h)'s 50 rounds
    /// truncate the digest to `n` bytes **each round** (T9 — feeding the full
    /// 16 bytes back gives a different key for every `n < 16`, i.e. every
    /// 40-bit file), and step (f) fires only on `/R` ≥ 4 with
    /// `/EncryptMetadata false` (T11).
    ///
    /// Slicing is in bounds by construction: `self.key_len` is fixed by
    /// [`Self::parse`] to 5 at `/R` 2 and to `/Length / 8` for a `/Length` it
    /// has already range-checked to `40..=128`, so it is always `5..=16` --
    /// never longer than the 16-byte digest being sliced.
    #[allow(clippy::indexing_slicing)]
    fn file_key_from_user_password(&self, password: &[u8], id0: &[u8]) -> Vec<u8> {
        let mut h = Md5::new();
        h.update(&pad_password(password)); // (a), (b)
        h.update(&self.o); // (c)
        h.update(&self.p.to_le_bytes()); // (d) — unsigned, low byte first
        h.update(id0); // (e)
        if self.revision >= 4 && !self.encrypt_metadata {
            h.update(&[0xFF, 0xFF, 0xFF, 0xFF]); // (f)
        }
        let mut digest = h.finish(); // (g)

        if self.revision >= 3 {
            // (h) — 50 rounds, truncating to n each time. T9.
            for _ in 0..50 {
                digest = md5(&digest[..self.key_len]);
            }
        }
        digest[..self.key_len].to_vec() // (i)
    }

    /// Algorithm 3 steps (a)–(d) — the RC4 key derived from an *owner*
    /// password, used to encrypt `/O` (and, run backwards, to recover the
    /// user password from it).
    ///
    /// The 50-round loop here does **not** truncate (T13). That is the single
    /// most commonly transposed pair in clause 7.6: Algorithm 2 step (h)
    /// passes "the first `n` bytes", Algorithm 3 step (c) passes "**it**".
    ///
    /// `self.key_len` is `5..=16`; see [`Self::file_key_from_user_password`].
    #[allow(clippy::indexing_slicing)]
    fn owner_rc4_key(&self, owner_password: &[u8]) -> Vec<u8> {
        let mut digest = md5(&pad_password(owner_password)); // (a), (b)
        if self.revision >= 3 {
            // (c) — 50 rounds, WHOLE digest each time. T13.
            for _ in 0..50 {
                digest = md5(&digest);
            }
        }
        digest[..self.key_len].to_vec() // (d)
    }

    /// Algorithms 4 and 5 — compute what `/U` should be for a given file key.
    ///
    /// Returns 32 bytes at `/R` 2 (Algorithm 4) and 16 at `/R` ≥ 3 (Algorithm
    /// 5 stops before step (f), whose "16 bytes of arbitrary padding" is
    /// exactly the part that must not be compared — T15).
    fn expected_u(&self, file_key: &[u8], id0: &[u8]) -> Vec<u8> {
        if self.revision == 2 {
            // Algorithm 4 (b): RC4 the padding string with the file key.
            rc4(file_key, &PADDING)
        } else {
            // Algorithm 5 (b), (c): MD5 of padding ‖ ID[0].
            let mut h = Md5::new();
            h.update(&PADDING);
            h.update(id0);
            let digest = h.finish();

            // (d): RC4 with the file key.
            let mut out = rc4(file_key, &digest);

            // (e): 19 more rounds, key XOR counter 1..=19. T16 — this loop
            // counts UP; Algorithm 7's counts DOWN from 19 to 0.
            for counter in 1u8..=19 {
                let key: Vec<u8> = file_key.iter().map(|b| b ^ counter).collect();
                out = rc4(&key, &out);
            }
            out
        }
    }

    /// Algorithm 6 — does this password authenticate as the user password?
    ///
    /// Returns the file key on success. The comparison is on the first 16
    /// bytes at `/R` ≥ 3, per the algorithm's own parenthetical — comparing
    /// all 32 rejects every conforming file, because the tail is arbitrary
    /// (T15).
    ///
    /// Slicing is guarded on the same line: both lengths are checked `>= n`
    /// before either is sliced, so a malformed short `/U` is a failed
    /// authentication rather than a panic.
    #[allow(clippy::indexing_slicing)]
    fn try_user_password(&self, password: &[u8], id0: &[u8]) -> Option<Vec<u8>> {
        let key = self.file_key_from_user_password(password, id0);
        let expect = self.expected_u(&key, id0);
        let n = if self.revision == 2 { 32 } else { 16 };
        if self.u.len() >= n && expect.len() >= n && expect[..n] == self.u[..n] {
            Some(key)
        } else {
            None
        }
    }

    /// Algorithm 7 — does this password authenticate as the *owner* password?
    ///
    /// Owner authentication is definitionally two-stage (**N5**): decrypting
    /// `/O` with a key derived from the candidate yields *the user password*,
    /// which is then run through Algorithm 6. There is no owner key and no way
    /// to recover the owner password itself — knowing the owner password gives
    /// you the user password for free, and the reverse is impossible.
    ///
    /// The loop counts **19 down to 0** — twenty rounds. The `0` round has key
    /// XOR 0, i.e. the plain key, and is the inverse of Algorithm 3's
    /// un-countered step (f). Running 1..=19 instead, or counting up, fails
    /// silently for every `/R` ≥ 3 file (T16).
    fn try_owner_password(&self, password: &[u8], id0: &[u8]) -> Option<Vec<u8>> {
        let key = self.owner_rc4_key(password); // (a)

        let user_pw = if self.revision == 2 {
            // (b), R 2: a single RC4 pass. RC4 is its own inverse (T17).
            rc4(&key, &self.o)
        } else {
            // (b), R >= 3: 20 rounds, counters 19, 18, …, 1, 0.
            let mut data = self.o.clone();
            for counter in (0u8..=19).rev() {
                let k: Vec<u8> = key.iter().map(|b| b ^ counter).collect();
                data = rc4(&k, &data);
            }
            data
        };

        // (c): the result "purports to be the user password". It is a padded
        // 32-byte block, and `pad_password` truncating to 32 makes feeding it
        // back in idempotent.
        self.try_user_password(&user_pw, id0)
    }

    /// Authenticate and derive the file encryption key.
    ///
    /// `password` is `None` to mean "try the default (empty) user password",
    /// which §7.6.3.1 requires a reader to do **first and silently**. That is
    /// deliberately distinct from `Some(b"")`, which is the operator typing an
    /// empty box — the two produce the same key here, but the returned
    /// [`AuthKind`] differs, and the shells use it to decide whether they ever
    /// showed a prompt.
    ///
    /// Order follows the clause: empty user password, then the supplied
    /// password as a user password, then as an owner password. Trying owner
    /// first would work, but would report [`AuthKind::Owner`] for a document
    /// whose two passwords happen to be equal, overstating the access granted.
    #[must_use]
    pub fn authenticate(&self, password: Option<&[u8]>, id0: &[u8]) -> Option<(FileKey, AuthKind)> {
        let make = |key: Vec<u8>, kind: AuthKind| {
            Some((
                FileKey {
                    key,
                    version: self.version,
                    stream_cipher: self.stream_cipher,
                    string_cipher: self.string_cipher,
                },
                kind,
            ))
        };

        // §7.6.3.1 step 1 — always, silently, before any prompt.
        if let Some(key) = self.try_user_password(b"", id0) {
            // A supplied password that also happens to be empty is still the
            // no-prompt case; report it as such.
            if password.is_none_or(<[u8]>::is_empty) {
                return make(key, AuthKind::EmptyUser);
            }
            // The empty password works but the operator supplied a different
            // one. Their password may be the owner password, which grants
            // more; check that before settling for the default.
            if let Some(pw) = password
                && let Some(okey) = self.try_owner_password(pw, id0)
            {
                return make(okey, AuthKind::Owner);
            }
            return make(key, AuthKind::EmptyUser);
        }

        let pw = password?;
        if let Some(key) = self.try_user_password(pw, id0) {
            return make(key, AuthKind::User);
        }
        if let Some(key) = self.try_owner_password(pw, id0) {
            return make(key, AuthKind::Owner);
        }
        None
    }
}

/// A file encryption key, plus what to do with it.
///
/// Held separately from [`EncryptionConfig`] because the config describes the
/// document and this describes a *successful authentication* — a document can
/// be parsed and refused, or parsed and prompted for, without one ever
/// existing.
#[derive(Debug, Clone)]
pub struct FileKey {
    /// The file encryption key, `n` bytes.
    key: Vec<u8>,
    /// `/V`, which Algorithm 1 needs to decide whether `n` is fixed at 5.
    version: i64,
    /// Cipher for stream data.
    stream_cipher: Cipher,
    /// Cipher for strings.
    string_cipher: Cipher,
}

impl FileKey {
    /// Algorithm 1 — the per-object key for object `id`.
    ///
    /// Two traps, both in the byte layout:
    ///
    /// - **T2** — the object number contributes its **3** low bytes and the
    ///   generation its **2**, both little-endian. This is normative
    ///   truncation, not an implementation shortcut, and it means objects
    ///   whose numbers differ only above 2^24 share a key.
    /// - **T1** — for AES the four bytes `73 41 6C 54` (`sAlT`) extend the
    ///   *MD5 input*, not the derived key; the key stays `min(n+5, 16)`.
    ///   Not reachable in this increment (AES is refused at parse time), but
    ///   the length rule is written here because that is where the next
    ///   increment will need it and where getting it wrong would be invisible.
    ///
    /// Slicing is in bounds by construction: `n` is `min(key.len() + 5, 16)`
    /// and the digest is exactly 16 bytes, so the `min` is what makes it
    /// safe -- which is also the spec's own rule, not a defensive clamp.
    #[must_use]
    #[allow(clippy::indexing_slicing)]
    pub fn object_key(&self, id: ObjId, cipher: Cipher) -> Vec<u8> {
        // §7.6.2: at /V 5 the file key is used as-is with no per-object step
        // (T24). Unreachable here (V 5 is refused), stated so the next
        // increment does not have to rediscover it.
        if self.version >= 5 {
            return self.key.clone();
        }

        let mut h = Md5::new();
        h.update(&self.key);
        let num = id.num.to_le_bytes();
        h.update(&num[..3]); // 3 low bytes, LE
        let generation = id.generation.to_le_bytes();
        h.update(&generation[..2]); // 2 low bytes, LE
        if cipher == Cipher::Aes128 {
            h.update(b"sAlT");
        }
        let digest = h.finish();
        let n = (self.key.len() + 5).min(16);
        digest[..n].to_vec()
    }

    /// Decrypt a **string** belonging to object `id`.
    ///
    /// **T3** — the `id` is the *containing indirect object's*, at any nesting
    /// depth. A string four levels inside a dictionary is keyed on the object
    /// that dictionary belongs to, not on anything nearer.
    #[must_use]
    pub fn decrypt_string(&self, id: ObjId, data: &[u8]) -> Vec<u8> {
        match self.string_cipher {
            Cipher::None => data.to_vec(),
            Cipher::Rc4 => rc4(&self.object_key(id, Cipher::Rc4), data),
            // Unreachable: parse refuses AES. Returning the input unchanged
            // rather than panicking keeps a library parsing untrusted input
            // from aborting the host, and the refusal upstream means no
            // document can reach it.
            Cipher::Aes128 | Cipher::Aes256 => data.to_vec(),
        }
    }

    /// Decrypt a **stream's** raw data, belonging to object `id`.
    ///
    /// "Raw" is load-bearing: §7.6.2 W1 puts encryption *outside* the filter
    /// chain, so this runs **before** `/FlateDecode` and friends. Decrypting
    /// after decoding would attempt to inflate ciphertext.
    #[must_use]
    pub fn decrypt_stream(&self, id: ObjId, data: &[u8]) -> Vec<u8> {
        match self.stream_cipher {
            Cipher::None => data.to_vec(),
            Cipher::Rc4 => rc4(&self.object_key(id, Cipher::Rc4), data),
            Cipher::Aes128 | Cipher::Aes256 => data.to_vec(),
        }
    }

    /// Whether strings are encrypted at all — `false` when `/StrF` is
    /// `/Identity`, which some producers use to leave metadata legible.
    #[must_use]
    pub fn strings_encrypted(&self) -> bool {
        self.string_cipher != Cipher::None
    }

    /// Whether stream data is encrypted at all.
    #[must_use]
    pub fn streams_encrypted(&self) -> bool {
        self.stream_cipher != Cipher::None
    }
}

/// Algorithm 2 step (a) — pad or truncate a password to exactly 32 bytes.
///
/// An empty password becomes the padding string in full, which is what makes
/// the "no user password" case work: the default password *is* `PADDING`.
///
/// Slicing is in bounds by construction: `take` is `min`-clamped to 32, so
/// both `out[..take]` and `PADDING[..32 - take]` stay within their fixed
/// 32-byte arrays for every input length including zero and including
/// passwords longer than 32 bytes.
#[must_use]
#[allow(clippy::indexing_slicing)]
pub fn pad_password(password: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    let take = password.len().min(32);
    out[..take].copy_from_slice(&password[..take]);
    out[take..].copy_from_slice(&PADDING[..32 - take]);
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::object::Name;

    #[test]
    fn empty_password_is_the_padding_string() {
        assert_eq!(pad_password(b""), PADDING);
    }

    #[test]
    fn short_password_takes_the_padding_prefix() {
        let p = pad_password(b"abc");
        assert_eq!(&p[..3], b"abc");
        assert_eq!(&p[3..], &PADDING[..29]);
    }

    /// "If the password string is more than 32 bytes long, use only its first
    /// 32 bytes" — no padding at all, and no hashing-down of the excess.
    #[test]
    fn long_password_is_truncated_not_hashed() {
        let long = vec![b'x'; 100];
        assert_eq!(pad_password(&long), [b'x'; 32]);
        // Exactly 32 is the boundary: still no padding bytes.
        let exact = vec![b'y'; 32];
        assert_eq!(pad_password(&exact), [b'y'; 32]);
    }

    /// T10 — `/P` is stored signed and hashed unsigned.
    ///
    /// The standard's own example: `-44`. As a `u32` that is `0xFFFFFFD4`,
    /// and low-byte-first it hashes as `D4 FF FF FF`. A parser that kept the
    /// value in an `i64` and took `to_le_bytes()` would feed eight bytes, not
    /// four, and would produce a wrong key for every document.
    #[test]
    fn p_is_hashed_as_unsigned_little_endian() {
        let p = -44i64 as i32 as u32;
        assert_eq!(p, 0xFFFF_FFD4);
        assert_eq!(p.to_le_bytes(), [0xD4, 0xFF, 0xFF, 0xFF]);
    }

    /// `granted` must distinguish "the author said no" from "the revision has
    /// no such concept". Collapsing them shows a restriction nobody wrote.
    #[test]
    fn reserved_bits_report_none_rather_than_false() {
        // `-44` has bits 9-12 SET. At R2 they are reserved, so every one of
        // them must report `None` — not `Some(true)` (which would invent a
        // permission) and not `Some(false)` (which would invent a
        // restriction).
        let raw = -44i64 as i32 as u32;
        let r2 = Permissions { raw, revision: 2 };
        let r3 = Permissions { raw, revision: 3 };

        for bit in [
            PermissionBit::FillForms,
            PermissionBit::AccessibilityExtract,
            PermissionBit::Assemble,
            PermissionBit::PrintHighQuality,
        ] {
            assert_eq!(r2.granted(bit), None, "{bit:?} is reserved at R2");
            assert_eq!(r3.granted(bit), Some(true), "{bit:?} is set at R3");
        }

        // The four that exist at every revision answer at both.
        for bit in [
            PermissionBit::Print,
            PermissionBit::ModifyContents,
            PermissionBit::Copy,
            PermissionBit::Annotate,
        ] {
            assert!(
                r2.granted(bit).is_some(),
                "{bit:?} applies at every revision"
            );
            assert_eq!(r2.granted(bit), r3.granted(bit));
        }
    }

    /// Every bit position must match Table 22, and no two may collide.
    #[test]
    fn permission_bit_positions_are_table_22() {
        let all = PermissionBit::all();
        assert_eq!(all.len(), 8, "Table 22 defines eight meaningful bits");
        let mut seen: Vec<u32> = Vec::new();
        for bit in all {
            let p = bit.position();
            assert!((3..=12).contains(&p), "{bit:?} at {p} is outside Table 22");
            assert!(!seen.contains(&p), "two permissions claim bit {p}");
            seen.push(p);
        }
        // Bits 7 and 8 are reserved and must not be claimed by anything.
        assert!(
            !seen.contains(&7) && !seen.contains(&8),
            "bits 7 and 8 are reserved"
        );
    }

    /// T12 — the standard's `-44` example is R2-only.
    ///
    /// At R2 it means print + copy. At R3+ the same number *also* grants
    /// fill-forms, accessibility extraction, assemble and high-quality print,
    /// because bits 9–12 are set in `0xFFFFFFD4` and become meaningful. An
    /// author copying the standard's example into an R4 file grants four
    /// permissions they did not intend.
    #[test]
    fn minus_44_grants_more_at_r3_than_at_r2() {
        let raw = -44i64 as i32 as u32;
        let r2 = Permissions { raw, revision: 2 };
        let r4 = Permissions { raw, revision: 4 };

        // Same at both revisions.
        assert!(r2.print() && r4.print());
        assert!(r2.copy() && r4.copy());
        assert!(!r2.modify_contents() && !r4.modify_contents());
        assert!(!r2.annotate() && !r4.annotate());

        // The four that differ.
        assert!(!r2.fill_forms() && r4.fill_forms());
        assert!(!r2.accessibility_extract() && r4.accessibility_extract());
        assert!(!r2.assemble() && r4.assemble());
        assert!(!r2.print_high_quality() && r4.print_high_quality());
    }

    /// T2 — the object number contributes 3 bytes and the generation 2.
    ///
    /// Verified by consequence rather than by inspecting the hash input:
    /// object 1 and object 0x1000001 differ only in the byte that is
    /// truncated away, so they must share a key. That is a normative
    /// property, and it is the only way to observe the truncation from
    /// outside the function.
    #[test]
    fn object_key_truncates_object_number_to_three_bytes() {
        let fk = FileKey {
            key: vec![1, 2, 3, 4, 5],
            version: 2,
            stream_cipher: Cipher::Rc4,
            string_cipher: Cipher::Rc4,
        };
        let a = fk.object_key(ObjId::new(1, 0), Cipher::Rc4);
        let b = fk.object_key(ObjId::new(0x0100_0001, 0), Cipher::Rc4);
        assert_eq!(a, b, "bits above 2^24 must not affect the key");

        // And a difference within the low 3 bytes must.
        let c = fk.object_key(ObjId::new(2, 0), Cipher::Rc4);
        assert_ne!(a, c);

        // Key length is min(n + 5, 16), so 5 + 5 = 10 here.
        assert_eq!(a.len(), 10);
    }

    /// The `min(n + 5, 16)` cap: at a 16-byte file key the object key does
    /// not grow to 21, it stops at the digest length.
    #[test]
    fn object_key_length_is_capped_at_sixteen() {
        let fk = FileKey {
            key: vec![0u8; 16],
            version: 4,
            stream_cipher: Cipher::Rc4,
            string_cipher: Cipher::Rc4,
        };
        assert_eq!(fk.object_key(ObjId::new(1, 0), Cipher::Rc4).len(), 16);
    }

    /// T1 — the `sAlT` bytes change the key's *value*, not its length.
    ///
    /// AES is refused at parse time in this increment, so this is a guard on
    /// the next one: if the salt were appended to the key instead of the hash
    /// input, the length would change here and every AES document would fail.
    #[test]
    fn aes_salt_changes_value_not_length() {
        let fk = FileKey {
            key: vec![9u8; 16],
            version: 4,
            stream_cipher: Cipher::Rc4,
            string_cipher: Cipher::Rc4,
        };
        let plain = fk.object_key(ObjId::new(7, 0), Cipher::Rc4);
        let salted = fk.object_key(ObjId::new(7, 0), Cipher::Aes128);
        assert_eq!(plain.len(), salted.len());
        assert_ne!(plain, salted);
    }

    fn name(s: &str) -> Object {
        Object::Name(Name(s.as_bytes().to_vec()))
    }

    fn nothing(_: ObjId) -> Option<Object> {
        None
    }

    fn minimal_encrypt(entries: Vec<(&str, Object)>) -> Dict {
        let mut d = Dict::new();
        d.insert(Name(b"Filter".to_vec()), name("Standard"));
        d.insert(Name(b"O".to_vec()), Object::String(vec![0u8; 32]));
        d.insert(Name(b"U".to_vec()), Object::String(vec![0u8; 32]));
        d.insert(Name(b"P".to_vec()), Object::Integer(-44));
        for (k, v) in entries {
            d.insert(Name(k.as_bytes().to_vec()), v);
        }
        d
    }

    /// `/V` 3 is the unpublished algorithm; `/V` 0 is undocumented. Both are
    /// refused as "nobody can open this", which is a different message from
    /// "pdfce hasn't implemented it".
    #[test]
    fn refuses_undocumented_algorithms() {
        for v in [0i64, 3] {
            let d = minimal_encrypt(vec![("V", Object::Integer(v)), ("R", Object::Integer(3))]);
            assert_eq!(
                EncryptionConfig::parse(&d, &nothing).unwrap_err(),
                EncryptionUnsupported::UndocumentedAlgorithm(v)
            );
        }
        // Absent /V defaults to 0 and is refused the same way.
        let d = minimal_encrypt(vec![("R", Object::Integer(3))]);
        assert_eq!(
            EncryptionConfig::parse(&d, &nothing).unwrap_err(),
            EncryptionUnsupported::UndocumentedAlgorithm(0)
        );
    }

    /// A missing `/Filter` is not "assume Standard" — Table 20 makes it a
    /// document no handler is permitted to decrypt.
    #[test]
    fn refuses_missing_filter_as_a_conformance_matter() {
        let mut d = minimal_encrypt(vec![("V", Object::Integer(2)), ("R", Object::Integer(3))]);
        d.remove(b"Filter");
        assert_eq!(
            EncryptionConfig::parse(&d, &nothing).unwrap_err(),
            EncryptionUnsupported::NoFilter
        );
    }

    #[test]
    fn refuses_public_key_handler_by_name() {
        let mut d = minimal_encrypt(vec![("V", Object::Integer(2)), ("R", Object::Integer(3))]);
        d.insert(Name(b"Filter".to_vec()), name("Adobe.PubSec"));
        assert!(matches!(
            EncryptionConfig::parse(&d, &nothing),
            Err(EncryptionUnsupported::Handler(h)) if h == "Adobe.PubSec"
        ));
    }

    /// `/R` 6 and `/R` 5 are refused for *different* reasons, and the
    /// distinction is the point: one is a missing implementation, the other a
    /// missing source. They resolve differently and the diagnostic must not
    /// blur them.
    #[test]
    fn distinguishes_unimplemented_from_unsourced() {
        let r5 = minimal_encrypt(vec![("V", Object::Integer(5)), ("R", Object::Integer(5))]);
        assert_eq!(
            EncryptionConfig::parse(&r5, &nothing).unwrap_err(),
            EncryptionUnsupported::CipherNotImplemented("AES-256")
        );

        let r6 = minimal_encrypt(vec![("V", Object::Integer(5)), ("R", Object::Integer(6))]);
        assert_eq!(
            EncryptionConfig::parse(&r6, &nothing).unwrap_err(),
            EncryptionUnsupported::UnsourcedRevision
        );
    }

    /// `/R` 2 fixes `n` at 5 regardless of `/Length` — Algorithm 2 step (i)
    /// says "shall always be 5 for security handlers of revision 2".
    /// A `/Length 128` on an R2 file must not widen the key.
    #[test]
    fn r2_ignores_length() {
        let d = minimal_encrypt(vec![
            ("V", Object::Integer(1)),
            ("R", Object::Integer(2)),
            ("Length", Object::Integer(128)),
        ]);
        let c = EncryptionConfig::parse(&d, &nothing).expect("R2 RC4 is supported");
        assert_eq!(c.key_len, 5);
    }

    /// Absent `/Length` defaults to 40 bits, not to "whatever /V implies".
    #[test]
    fn absent_length_defaults_to_forty_bits() {
        let d = minimal_encrypt(vec![("V", Object::Integer(2)), ("R", Object::Integer(3))]);
        let c = EncryptionConfig::parse(&d, &nothing).expect("R3 RC4 is supported");
        assert_eq!(c.key_len, 5);
    }

    /// `/EncryptMetadata` defaults to **true**; its absence must not be read
    /// as false, which would add four bytes to Algorithm 2's hash (T11) and
    /// produce a wrong key for every ordinary R4 document.
    #[test]
    fn encrypt_metadata_defaults_true() {
        let d = minimal_encrypt(vec![("V", Object::Integer(2)), ("R", Object::Integer(3))]);
        let c = EncryptionConfig::parse(&d, &nothing).expect("supported");
        assert!(c.encrypt_metadata);
    }

    /// A `/V 4` document routing streams through an AES filter is refused by
    /// cipher name, not by revision — the plumbing is fine, the cipher is not
    /// written yet.
    #[test]
    fn refuses_aesv2_by_cipher_name() {
        let mut cf = Dict::new();
        let mut stdcf = Dict::new();
        stdcf.insert(Name(b"CFM".to_vec()), name("AESV2"));
        cf.insert(Name(b"StdCF".to_vec()), Object::Dict(stdcf));

        let d = minimal_encrypt(vec![
            ("V", Object::Integer(4)),
            ("R", Object::Integer(4)),
            ("Length", Object::Integer(128)),
            ("CF", Object::Dict(cf)),
            ("StmF", name("StdCF")),
            ("StrF", name("StdCF")),
        ]);
        assert_eq!(
            EncryptionConfig::parse(&d, &nothing).unwrap_err(),
            EncryptionUnsupported::CipherNotImplemented("AES-128")
        );
    }

    /// `/V 4` + `/CFM /V2` is the supported crypt-filter case, and `/StmF`
    /// and `/StrF` are independent — a document may encrypt streams and leave
    /// strings in the clear.
    #[test]
    fn v4_v2_filter_is_supported_and_stmf_strf_are_independent() {
        let mut cf = Dict::new();
        let mut stdcf = Dict::new();
        stdcf.insert(Name(b"CFM".to_vec()), name("V2"));
        cf.insert(Name(b"StdCF".to_vec()), Object::Dict(stdcf));

        let d = minimal_encrypt(vec![
            ("V", Object::Integer(4)),
            ("R", Object::Integer(4)),
            ("Length", Object::Integer(128)),
            ("CF", Object::Dict(cf)),
            ("StmF", name("StdCF")),
            ("StrF", name("Identity")),
        ]);
        let c = EncryptionConfig::parse(&d, &nothing).expect("V2 crypt filter is supported");
        assert_eq!(c.stream_cipher, Cipher::Rc4);
        assert_eq!(c.string_cipher, Cipher::None);
        assert_eq!(c.key_len, 16);
    }

    /// N10 — a `/StmF` naming a filter absent from `/CF` is refused rather
    /// than silently treated as Identity. Guessing Identity would hand
    /// ciphertext to the content parser, which fails much later and in a
    /// place that looks nothing like an encryption problem.
    #[test]
    fn refuses_stmf_naming_an_absent_crypt_filter() {
        let d = minimal_encrypt(vec![
            ("V", Object::Integer(4)),
            ("R", Object::Integer(4)),
            ("CF", Object::Dict(Dict::new())),
            ("StmF", name("StdCF")),
        ]);
        assert!(matches!(
            EncryptionConfig::parse(&d, &nothing),
            Err(EncryptionUnsupported::Malformed(_))
        ));
    }

    /// An unknown `/CFM` gets its own diagnostic — Table 25 puts a `shall` on
    /// *reporting* this case, which is unusual enough to be worth honouring
    /// precisely.
    #[test]
    fn unknown_cfm_is_named() {
        let mut cf = Dict::new();
        let mut f = Dict::new();
        f.insert(Name(b"CFM".to_vec()), name("Whirlpool"));
        cf.insert(Name(b"StdCF".to_vec()), Object::Dict(f));

        let d = minimal_encrypt(vec![
            ("V", Object::Integer(4)),
            ("R", Object::Integer(4)),
            ("CF", Object::Dict(cf)),
            ("StmF", name("StdCF")),
        ]);
        assert!(matches!(
            EncryptionConfig::parse(&d, &nothing),
            Err(EncryptionUnsupported::UnknownCfm(n)) if n == "Whirlpool"
        ));
    }
}
