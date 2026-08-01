//! # The extraction walk over a page's content stream
//!
//! A single pass over [`crate::content::ContentStream`]'s lossless token
//! stream that maintains exactly the state extraction needs and nothing
//! else:
//!
//! | State | Operators | Clause |
//! |---|---|---|
//! | CTM | `q` `Q` `cm` | §8.4.4 |
//! | text object matrices `Tm`/`Tlm` | `BT` `ET` `Td` `TD` `Tm` `T*` | §9.4.2 |
//! | text state | `Tf` `Tc` `Tw` `Tz` `TL` `Ts` `Tr` | §9.3 |
//! | marked-content stack | `BMC` `BDC` `EMC` | §14.6 |
//! | form nesting | `Do` | §8.10.1 |
//!
//! Everything else — paths, colour, shading, images, `gs`, clipping — is
//! ignored by construction. This is *not* a stripped-down renderer: a
//! renderer needs colour to decide what a glyph looks like, and
//! extraction explicitly does not care whether a glyph is visible
//! (§14.8.2.2.3 item 3, a `shall`: "page content shall be considered to
//! include all text and illustrations in their entirety, regardless of
//! whether they are visible").
//!
//! ## The marked-content stack is the load-bearing part
//!
//! Four §14.6 tags change what comes out, and all four are attached by
//! nesting rather than by adjacency, so a stack is not an optimization:
//!
//! - **`/Artifact`** (§14.8.2.2.2) — classify the enclosed content;
//!   never drop it silently (A3).
//! - **`/Span` with `/ActualText`** (§14.9.4) — *replace* the enclosed
//!   content's characters.
//! - **`/ReversedChars`** (§14.8.2.3.3) — the enclosed show strings hold
//!   their characters in reverse of page content order.
//! - **`/TagSuspect`** (§14.8.2.3.1) — the producer disclaims its own
//!   ordering for the enclosed region.
//!
//! §14.6.2's rule for the `BDC` property-list operand is about
//! *indirectness*, not size: an all-direct dictionary may be inline, but
//! if any value is an indirect reference the list "shall be defined as a
//! named resource in the `Properties` subdictionary of the **current**
//! resource dictionary". Current, not the page's — which is why the
//! resource dictionary travels with the walk into form XObjects.
//!
//! ## Unbalanced marked content
//!
//! §14.6 N1: the standard states the nesting rules as *writer*
//! constraints and gives no reader-side recovery for an unbalanced `EMC`
//! or a `BMC` left open at end of stream. pdfce ignores an `EMC` with an
//! empty stack and lets an unclosed sequence expire at end of stream —
//! the two choices that cannot lose content. Both are pdfce policy.
//!
//! ## The axis-aligned assumption in the geometry
//!
//! Glyph origins are computed exactly, through the full §9.4.4 text
//! rendering matrix, and are therefore correct under any transform. The
//! *derived* line/word segmentation in [`super::layout`] then compares
//! those origins on the x and y axes, which assumes text runs left to
//! right along user-space x. Rotated text extracts with correct
//! characters and correct positions but over-produces derived line
//! breaks. This is a limitation of the derived layer only; it cannot
//! affect a sourced character, and
//! [`super::ExtractedText::sourced_text`] is unaffected by it.

use std::collections::HashMap;
use std::rc::Rc;

use crate::content::{ContentError, ContentStream, ContentTokenKind, Operation};
use crate::document::Document;
use crate::object::{Dict, Object};
use crate::page_tree::{Page, Rect};
use crate::span::ByteSpan;
use crate::textstring::decode_text_string;

use super::font::{ExtractFont, FontNote, LadderRung, Rung3Gap};
use super::{
    ArtifactKind, ContentStreamRef, ExtractOptions, GlyphProvenance, TextColor, TextDiagnostics,
};

/// One thing the walk produced, before derived whitespace is inserted.
#[derive(Debug, Clone)]
pub(super) enum Item {
    /// One shown glyph and the characters it decoded to.
    Glyph(GlyphItem),
    /// An `/ActualText` replacement covering a marked-content sequence.
    Replacement(ReplacementItem),
}

/// One shown glyph.
#[derive(Debug, Clone)]
pub(super) struct GlyphItem {
    /// The characters this code produced (possibly several, possibly
    /// U+FFFD if the ladder failed).
    pub chars: String,
    /// The character code, as segmented from the show string.
    pub code: u32,
    /// Which ladder rung produced `chars`.
    pub rung: LadderRung,
    /// Origin x in default user space.
    pub x: f32,
    /// Origin y in default user space.
    pub y: f32,
    /// Horizontal advance in default user space.
    pub advance: f32,
    /// Effective font size in default user space.
    pub size: f32,
    /// Text rendering mode 3 or 7.
    pub invisible: bool,
    /// Enclosing `/Artifact` classification, if any.
    pub artifact: Option<ArtifactKind>,
    /// Enclosing `/MCID`, if any.
    pub mcid: Option<u32>,
    /// Source-operator identity + text state, captured only when
    /// [`ExtractOptions::capture_provenance`] is set (otherwise `None`).
    pub provenance: Option<GlyphProvenance>,
}

/// An `/ActualText` replacement.
#[derive(Debug, Clone)]
pub(super) struct ReplacementItem {
    /// The decoded replacement text (a §7.9.2.2 text string).
    pub text: String,
    /// Enclosing `/Artifact` classification, if any.
    pub artifact: Option<ArtifactKind>,
    /// Enclosing `/MCID`, if any.
    pub mcid: Option<u32>,
    /// Bounding box of the glyphs the replacement covered, if it covered
    /// any. This is the *only* positional information an `/ActualText`
    /// run can carry — §14.9.4 N4 makes per-character correspondence
    /// impossible.
    pub bbox: Option<Rect>,
}

/// A 2-D affine transform in PDF's row-vector convention:
/// `[a b 0 / c d 0 / e f 1]` (§8.3.3).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Matrix {
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
}

impl Matrix {
    /// The identity transform.
    const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    /// `self × other`, in PDF's order — `self` applies first.
    fn mul(self, other: Self) -> Self {
        Self {
            a: self.a * other.a + self.b * other.c,
            b: self.a * other.b + self.b * other.d,
            c: self.c * other.a + self.d * other.c,
            d: self.c * other.b + self.d * other.d,
            e: self.e * other.a + self.f * other.c + other.e,
            f: self.e * other.b + self.f * other.d + other.f,
        }
    }

    /// A pure translation.
    const fn translate(tx: f32, ty: f32) -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: tx,
            f: ty,
        }
    }

    /// The magnitude of the transformed unit x vector — how much one
    /// unit of horizontal text space measures in user space.
    fn x_scale(self) -> f32 {
        self.a.hypot(self.b)
    }

    /// The magnitude of the transformed unit y vector.
    fn y_scale(self) -> f32 {
        self.c.hypot(self.d)
    }

    /// This matrix as PDF's 6-element `[a b c d e f]` row-vector array
    /// (§8.3.3) — the form [`GlyphProvenance`] carries for the surgery.
    const fn to_array(self) -> [f32; 6] {
        [self.a, self.b, self.c, self.d, self.e, self.f]
    }
}

/// §9.3's text state parameters, plus the current font.
#[derive(Clone)]
struct TextState {
    font: Option<Rc<ExtractFont>>,
    /// `Tfs` — font size (§9.3.1). No default: showing text with no
    /// `Tf` is malformed, and pdfce treats the size as 0 rather than
    /// inventing one.
    size: f32,
    /// `Tc` — character spacing, unscaled text-space units (§9.3.2).
    char_spacing: f32,
    /// `Tw` — word spacing (§9.3.3). Applies **only** to single-byte
    /// code 32, which makes it inert under `Identity-H`.
    word_spacing: f32,
    /// `Tz` — horizontal scaling, already divided by 100 (§9.3.4).
    h_scale: f32,
    /// `TL` — leading (§9.3.5).
    leading: f32,
    /// `Ts` — rise (§9.3.6).
    rise: f32,
    /// `Tr` — rendering mode (§9.3.6, Table 106).
    render_mode: i64,
    /// The current *fill* colour (§8.6.8), captured for provenance only.
    /// Part of the graphics state, so it is saved/restored by `q`/`Q` via
    /// this struct's `Clone`. `None` = unset, i.e. the §8.6.8 default black.
    /// Set only by the device operators `g`/`rg`/`k`; a colour set in a
    /// named space is recorded as [`TextColor::Other`] (see [`TextColor`]).
    fill_color: Option<TextColor>,
}

impl Default for TextState {
    fn default() -> Self {
        Self {
            font: None,
            size: 0.0,
            char_spacing: 0.0,
            word_spacing: 0.0,
            h_scale: 1.0,
            leading: 0.0,
            rise: 0.0,
            render_mode: 0,
            fill_color: None,
        }
    }
}

/// One level of the §14.6 marked-content stack.
#[derive(Debug, Clone)]
struct MarkedLevel {
    artifact: Option<ArtifactKind>,
    mcid: Option<u32>,
    reversed_chars: bool,
    /// Present when this level is a `/Span` carrying `/ActualText`, and
    /// this level is the OUTERMOST such level (see the nesting policy in
    /// [`Walk::begin_marked`]).
    actual_text: Option<String>,
    /// Bounding box accumulated over the glyphs this level suppressed.
    covered: Option<Rect>,
}

/// The whole walk state.
struct Walk<'a> {
    doc: &'a Document,
    options: &'a ExtractOptions,
    items: Vec<Item>,
    diagnostics: TextDiagnostics,

    ctm: Matrix,
    ctm_stack: Vec<Matrix>,
    ts: TextState,
    ts_stack: Vec<TextState>,
    tm: Matrix,
    tlm: Matrix,

    marked: Vec<MarkedLevel>,
    /// Font cache keyed by the resource name **plus** the resource
    /// dictionary's identity, because `/F1` inside a form XObject is a
    /// different font from `/F1` on the page (§8.10.1's resource
    /// switching — a correctness requirement, not an optimization).
    fonts: HashMap<(usize, Vec<u8>), Rc<ExtractFont>>,
    /// Fonts already reported, so a font used on 400 pages produces one
    /// diagnostic and one increment.
    fonts_seen: Vec<Vec<u8>>,
    depth: usize,
    /// XObject object numbers currently executing, for the §8.10.1 cycle
    /// guard. Keyed on object number, not resource name: the same stream
    /// can be reached under different names.
    active_xobjects: Vec<u32>,

    // --- provenance capture (only meaningful when the option is set) ---
    /// Which decoded buffer the current operator spans index: the page's
    /// own concatenated content, or the form XObject currently executing
    /// (§8.10.1). Saved/restored around every `Do`.
    stream_ref: ContentStreamRef,
    /// Byte span of the show operator currently being interpreted, in the
    /// buffer named by [`Self::stream_ref`]. Set for `Tj`/`'`/`"`/`TJ`
    /// before decoding, so [`Walk::show_code`] can attribute each glyph to
    /// its operator.
    cur_op_span: ByteSpan,
    /// The `/F1`-style resource name of the font selected by the most
    /// recent `Tf` (§9.3.1), as raw name bytes — for provenance only.
    cur_font_resource: Option<Vec<u8>>,
}

/// Walk one page and return its raw items plus diagnostics.
pub(super) fn walk_page(
    doc: &Document,
    page: &Page,
    options: &ExtractOptions,
) -> Result<(Vec<Item>, TextDiagnostics), ContentError> {
    let stream = ContentStream::from_page(doc, page)?;
    let mut walk = Walk {
        doc,
        options,
        items: Vec::new(),
        diagnostics: TextDiagnostics::default(),
        ctm: Matrix::IDENTITY,
        ctm_stack: Vec::new(),
        ts: TextState::default(),
        ts_stack: Vec::new(),
        tm: Matrix::IDENTITY,
        tlm: Matrix::IDENTITY,
        marked: Vec::new(),
        fonts: HashMap::new(),
        fonts_seen: Vec::new(),
        depth: 0,
        active_xobjects: Vec::new(),
        stream_ref: ContentStreamRef::Page,
        cur_op_span: ByteSpan::new(0, 0),
        cur_font_resource: None,
    };
    walk.run(&stream, &page.resources);
    Ok((walk.items, walk.diagnostics))
}

impl Walk<'_> {
    /// Execute one content stream against one resource dictionary.
    fn run(&mut self, stream: &ContentStream, resources: &Dict) {
        for op in stream.operations() {
            let Some(name) = op.operator_name(&stream.buf) else {
                // An inline image: a graphics object with no text.
                continue;
            };
            self.operator(name, &op, &stream.buf, resources);
        }
        // §14.6 N1: sequences left open at end of stream simply expire.
        // Any /ActualText they carried still has to be emitted, or the
        // replacement text would be lost along with the glyphs it
        // suppressed.
        while !self.marked.is_empty() {
            self.end_marked();
        }
    }

    /// Dispatch one operator.
    fn operator(&mut self, name: &[u8], op: &Operation<'_>, buf: &[u8], resources: &Dict) {
        let nums = |count: usize| -> Vec<f32> { operand_numbers(op, count) };
        match name {
            // --- graphics state (§8.4.4) ---
            b"q" => {
                self.ctm_stack.push(self.ctm);
                self.ts_stack.push(self.ts.clone());
                // A hostile stream of `q`s must not grow the stacks
                // without bound; 256 is far past any real nesting and
                // matches the posture of the other structural guards.
                if self.ctm_stack.len() > 256 {
                    self.ctm_stack.remove(0);
                    self.ts_stack.remove(0);
                }
            }
            b"Q" => {
                if let Some(m) = self.ctm_stack.pop() {
                    self.ctm = m;
                }
                if let Some(ts) = self.ts_stack.pop() {
                    self.ts = ts;
                }
            }
            b"cm" => {
                let v = nums(6);
                if let [a, b, c, d, e, f] = v[..] {
                    self.ctm = Matrix { a, b, c, d, e, f }.mul(self.ctm);
                }
            }

            // --- text object (§9.4.1) ---
            b"BT" => {
                self.tm = Matrix::IDENTITY;
                self.tlm = Matrix::IDENTITY;
            }
            b"ET" => {}

            // --- text state (§9.3) ---
            b"Tf" => self.select_font(op, resources),
            b"Tc" => {
                if let [v] = nums(1)[..] {
                    self.ts.char_spacing = v;
                }
            }
            b"Tw" => {
                if let [v] = nums(1)[..] {
                    self.ts.word_spacing = v;
                }
            }
            b"Tz" => {
                if let [v] = nums(1)[..] {
                    self.ts.h_scale = v / 100.0;
                }
            }
            b"TL" => {
                if let [v] = nums(1)[..] {
                    self.ts.leading = v;
                }
            }
            b"Ts" => {
                if let [v] = nums(1)[..] {
                    self.ts.rise = v;
                }
            }
            b"Tr" => {
                self.ts.render_mode = op
                    .operands
                    .last()
                    .and_then(operand_object)
                    .and_then(Object::as_int)
                    .unwrap_or(0);
            }

            // --- text positioning (§9.4.2) ---
            b"Td" => {
                if let [tx, ty] = nums(2)[..] {
                    self.next_line(tx, ty);
                }
            }
            b"TD" => {
                if let [tx, ty] = nums(2)[..] {
                    // "sets the leading parameter to -ty" (Table 108).
                    self.ts.leading = -ty;
                    self.next_line(tx, ty);
                }
            }
            b"Tm" => {
                let v = nums(6);
                if let [a, b, c, d, e, f] = v[..] {
                    self.tlm = Matrix { a, b, c, d, e, f };
                    self.tm = self.tlm;
                }
            }
            b"T*" => {
                let leading = self.ts.leading;
                self.next_line(0.0, -leading);
            }

            // --- text showing (§9.4.3, Table 109) ---
            // Each show operator records its own byte span (in the current
            // stream buffer) so every glyph it produces can be attributed
            // back to it for provenance (decision 014 Pass 14.1's surgery
            // locates the operator by exactly this span). Inert when
            // provenance capture is off — the field is simply never read.
            b"Tj" => {
                self.cur_op_span = op.operator.span;
                if let Some(s) = last_string(op) {
                    self.show(&s);
                }
            }
            b"'" => {
                self.cur_op_span = op.operator.span;
                let leading = self.ts.leading;
                self.next_line(0.0, -leading);
                if let Some(s) = last_string(op) {
                    self.show(&s);
                }
            }
            b"\"" => {
                // `aw ac string "` — sets word and character spacing,
                // then behaves like `'`.
                self.cur_op_span = op.operator.span;
                let v = operand_numbers(op, 3);
                if let Some(&aw) = v.first() {
                    self.ts.word_spacing = aw;
                }
                if let Some(&ac) = v.get(1) {
                    self.ts.char_spacing = ac;
                }
                let leading = self.ts.leading;
                self.next_line(0.0, -leading);
                if let Some(s) = last_string(op) {
                    self.show(&s);
                }
            }
            b"TJ" => {
                self.cur_op_span = op.operator.span;
                self.show_array(op);
            }

            // --- fill colour (§8.6.8), provenance only ---
            // Only the lowercase (fill) device operators are read; the
            // uppercase G/RG/K set the STROKE colour, which does not paint
            // text under the default rendering modes. A colour set through
            // sc/scn in a named space is left as the prior value or, once
            // any such operator is seen, marked Other — never decoded here.
            b"g" => {
                if let [gray] = nums(1)[..] {
                    self.ts.fill_color = Some(TextColor::Gray(gray));
                }
            }
            b"rg" => {
                if let [r, g, b] = nums(3)[..] {
                    self.ts.fill_color = Some(TextColor::Rgb(r, g, b));
                }
            }
            b"k" => {
                if let [c, m, y, kk] = nums(4)[..] {
                    self.ts.fill_color = Some(TextColor::Cmyk(c, m, y, kk));
                }
            }
            b"sc" | b"scn" => {
                // A fill colour in the current (possibly named) space.
                // pdfce's read-only walk does not track the fill colour
                // space, so the value is recorded as present-but-unmodelled
                // rather than guessed (§8.6.8; see TextColor::Other).
                self.ts.fill_color = Some(TextColor::Other);
            }

            // --- marked content (§14.6) ---
            b"BMC" => {
                let tag = operand_name(op, 0).unwrap_or_default();
                self.begin_marked(&tag, None);
            }
            b"BDC" => {
                let tag = operand_name(op, 0).unwrap_or_default();
                let props = self.resolve_properties(op, resources);
                self.begin_marked(&tag, props.as_ref());
            }
            b"EMC" => self.end_marked(),

            // --- XObjects (§8.10) ---
            b"Do" => self.do_xobject(op, buf, resources),

            _ => {}
        }
    }

    /// `Td`: "move to the start of the next line, offset from the start
    /// of the current line by (tx, ty)" — `Tlm = translate × Tlm`, then
    /// `Tm = Tlm` (Table 108).
    fn next_line(&mut self, tx: f32, ty: f32) {
        self.tlm = Matrix::translate(tx, ty).mul(self.tlm);
        self.tm = self.tlm;
    }

    /// `Tf`: resolve the named font resource, with caching.
    fn select_font(&mut self, op: &Operation<'_>, resources: &Dict) {
        // `Tf` is `font size Tf` — a NAME then a number. `operand_numbers`
        // filters non-numeric operands out, so asking it for "the second
        // of two" would silently return the font size at index 0 on a
        // well-formed operator and nothing at all once the name is
        // dropped. Read the size from the last operand directly.
        if let Some(size) = op
            .operands
            .last()
            .and_then(operand_object)
            .and_then(Object::as_number)
        {
            self.ts.size = size as f32;
        }
        let Some(name) = operand_name(op, 0) else {
            return;
        };
        // The cache key must include the resource dictionary's identity:
        // `/F1` in a form's own /Resources is a different font from the
        // page's `/F1`, and conflating them paints — or here, extracts —
        // the wrong characters entirely.
        let key = (std::ptr::from_ref(resources) as usize, name.clone());
        // Record the resource name alongside the font, so provenance can
        // report which /Resources /Font key painted a glyph. Set only on a
        // successful selection: the not-found path below keeps the previous
        // font, so it must keep the previous resource name too.
        if let Some(font) = self.fonts.get(&key) {
            self.ts.font = Some(Rc::clone(font));
            self.cur_font_resource = Some(name);
            return;
        }
        let font_dict = self
            .doc
            .resolve(resources.get(b"Font").unwrap_or(&Object::Null))
            .as_dict()
            .and_then(|fonts| fonts.get(&name))
            .map(|o| self.doc.resolve(o))
            .and_then(Object::as_dict);
        let Some(font_dict) = font_dict else {
            // A `Tf` naming a resource that is not there: §7.8.3 makes
            // this malformed with no recovery. Keep the previous font
            // rather than silently dropping the text that follows.
            self.diagnostics.note(format!(
                "text: font resource /{} not found in the current /Resources — \
                 following text uses the previously selected font",
                String::from_utf8_lossy(&name)
            ));
            return;
        };
        let font = Rc::new(ExtractFont::resolve(self.doc, font_dict));
        self.report_font(&font);
        self.fonts.insert(key, Rc::clone(&font));
        self.ts.font = Some(font);
        self.cur_font_resource = Some(name);
    }

    /// Turn a newly resolved font's [`FontNote`]s into counted, named
    /// diagnostics — once per distinct font, not once per `Tf`.
    fn report_font(&mut self, font: &ExtractFont) {
        let key = font.base_font.as_bytes().to_vec();
        if self.fonts_seen.contains(&key) {
            return;
        }
        self.fonts_seen.push(key);
        let name = if font.base_font.is_empty() {
            "<unnamed>"
        } else {
            &font.base_font
        };
        for note in &font.notes {
            match note {
                FontNote::Rung3(Rung3Gap::IdentityNoToUnicode) => {
                    self.diagnostics.identity_fonts_without_to_unicode += 1;
                    self.diagnostics.note(format!(
                        "text: font {name} is Identity-H/Adobe-Identity-0 with NO /ToUnicode — \
                         ISO 32000-1 §9.10.2 excludes it from every ladder rung, so no Unicode is \
                         recoverable for it"
                    ));
                }
                FontNote::Rung3(Rung3Gap::Ucs2NotBundled { cmap_name }) => {
                    self.diagnostics.ucs2_cmaps_unavailable += 1;
                    self.diagnostics.note(format!(
                        "text: font {name} uses a known Adobe character collection, but the \
                         {cmap_name} CID-to-Unicode CMap (§9.10.2 rung 3 step d) is an Adobe \
                         resource file pdfce does not bundle"
                    ));
                }
                FontNote::Rung3(Rung3Gap::PredefinedCmapNotBundled { cmap_name }) => {
                    self.diagnostics.predefined_cmaps_unavailable += 1;
                    self.diagnostics.note(format!(
                        "text: font {name} uses predefined CMap {cmap_name} — pdfce bundles \
                         neither its codespace nor a CID-to-Unicode mapping; 2-byte code \
                         segmentation assumed"
                    ));
                }
                FontNote::BuiltinEncodingUnreadable => {
                    self.diagnostics.note(format!(
                        "text: font {name} relies on its embedded program's built-in encoding, \
                         which pdfce-core cannot read; StandardEncoding assumed and any recovered \
                         characters are counted as the glyph-name extension, not as §9.10.2 rung 2"
                    ));
                }
                FontNote::UnknownSubtype => {
                    self.diagnostics.note(format!(
                        "text: font {name} has an absent or unrecognized /Subtype (Table 110); \
                         treated as a simple font"
                    ));
                }
                FontNote::ToUnicodeUnusable => {
                    self.diagnostics.note(format!(
                        "text: font {name} has a /ToUnicode entry that could not be decoded or \
                         yielded no mappings"
                    ));
                }
                FontNote::CodespaceWidthConflict {
                    font: f,
                    to_unicode,
                } => {
                    self.diagnostics.note(format!(
                        "text: font {name} declares a {to_unicode}-byte /ToUnicode codespace but \
                         its encoding implies {f}-byte codes (§9.10.3 requires consistency and \
                         states no recovery); segmented by the font's encoding"
                    ));
                }
                FontNote::WidthsEstimated => {
                    self.diagnostics.fonts_with_estimated_widths += 1;
                    self.diagnostics.note(format!(
                        "text: font {name} has no /Widths and is not a standard-14 face — advance \
                         widths are ESTIMATED, so derived word and line breaks near it are less \
                         reliable (characters are unaffected)"
                    ));
                }
            }
        }
    }

    /// `Tj` / `'` / `"`: show one string.
    fn show(&mut self, string: &[u8]) {
        let Some(font) = self.ts.font.clone() else {
            // Showing text with no font selected is malformed (§9.4.1's
            // "Tf shall precede"). There is nothing to decode with, so
            // the codes are counted as failures rather than dropped —
            // "we saw N characters we could not read" is the honest
            // report, and silence would hide the text entirely.
            self.diagnostics.codes_total += string.len() as u64;
            self.diagnostics.ladder_failures += string.len() as u64;
            self.diagnostics.note(
                "text: a show operator appeared with no font selected (§9.4.1 requires Tf first); \
                 those character codes are counted as unresolvable"
                    .to_string(),
            );
            return;
        };
        let start = self.items.len();
        for code in font.codes(string) {
            self.show_code(&font, code.value, code.word_spacing_applies);
        }
        // §14.8.2.3.3: "only the individual characters within each string
        // shall be reversed; the strings themselves shall be in natural
        // reading order." Per-string, not per-sequence — reversing the
        // sequence instead would reverse the word order of the whole run.
        if self.reversed_chars()
            && let Some(slice) = self.items.get_mut(start..)
        {
            slice.reverse();
        }
    }

    /// `TJ`: an array of strings and number adjustments (Table 109).
    ///
    /// The adjustment "shall be subtracted from the current horizontal
    /// coordinate", expressed in **thousandths of a unit of text space**
    /// — and it is applied *before* the next glyph, which is why it is
    /// carried into [`Walk::show_code`] rather than applied as a
    /// standalone translation.
    fn show_array(&mut self, op: &Operation<'_>) {
        let Some(Object::Array(items)) = op.operands.last().and_then(operand_object) else {
            return;
        };
        let items = items.clone();
        let Some(font) = self.ts.font.clone() else {
            for item in &items {
                if let Object::String(s) = item {
                    self.diagnostics.codes_total += s.len() as u64;
                    self.diagnostics.ladder_failures += s.len() as u64;
                }
            }
            return;
        };
        let start = self.items.len();
        for item in &items {
            match item {
                Object::String(s) => {
                    for code in font.codes(s) {
                        self.show_code(&font, code.value, code.word_spacing_applies);
                    }
                }
                other => {
                    if let Some(v) = other.as_number() {
                        // Table 109: "the amount shall be subtracted from
                        // the current horizontal coordinate", scaled by
                        // the font size and Tz. Applied NOW, so the next
                        // glyph is placed at the shifted origin.
                        let tx = -(v as f32) / 1000.0 * self.ts.size * self.ts.h_scale;
                        self.tm = Matrix::translate(tx, 0.0).mul(self.tm);
                    }
                }
            }
        }
        if self.reversed_chars()
            && let Some(slice) = self.items.get_mut(start..)
        {
            slice.reverse();
        }
    }

    /// Decode and place one character code, then advance the text matrix
    /// per §9.4.4.
    ///
    /// `TJ` adjustments are **not** a parameter here. §9.4.4 folds them
    /// into the displacement formula as `(w0 − Tj/1000)`, which reads as
    /// though the adjustment belonged to the glyph being shown — but
    /// Table 109 is explicit that the number "shall be subtracted from
    /// the current horizontal coordinate", i.e. it moves the position
    /// and *then* the next glyph is shown there. Folding it into the
    /// current glyph's advance instead places that glyph at the
    /// pre-shift origin, which leaves the whole shift invisible to a
    /// gap-based word-space heuristic reading origins. [`Walk::show_array`]
    /// therefore applies each adjustment to the text matrix as it meets
    /// it.
    fn show_code(&mut self, font: &ExtractFont, code: u32, word_spacing: bool) {
        let (chars, rung) = font.to_unicode(code);
        self.diagnostics.codes_total += 1;
        match rung {
            LadderRung::ToUnicode => self.diagnostics.via_to_unicode += 1,
            LadderRung::EncodingAgl => self.diagnostics.via_encoding_agl += 1,
            LadderRung::CidCollection => self.diagnostics.via_cid_collection += 1,
            LadderRung::GlyphNameExtension => self.diagnostics.via_glyph_name_extension += 1,
            LadderRung::Failed => self.diagnostics.ladder_failures += 1,
        }

        // §9.4.4: Trm = [Tfs·Th 0 0 Tfs 0 Ts] × Tm × CTM.
        let params = Matrix {
            a: self.ts.size * self.ts.h_scale,
            b: 0.0,
            c: 0.0,
            d: self.ts.size,
            e: 0.0,
            f: self.ts.rise,
        };
        let tm_ctm = self.tm.mul(self.ctm);
        let trm = params.mul(tm_ctm);

        // §9.4.4's displacement:
        //   tx = ((w0 − Tj/1000)·Tfs + Tc + Tw) · Th
        // Tw participates ONLY for a single-byte code 32 (§9.3.3) —
        // which is why it is inert under Identity-H and useless as a
        // word-break signal in modern documents (S6).
        let w0 = font.width(code);
        let tw = if word_spacing {
            self.ts.word_spacing
        } else {
            0.0
        };
        let tx = (w0 * self.ts.size + self.ts.char_spacing + tw) * self.ts.h_scale;

        let invisible = matches!(self.ts.render_mode, 3 | 7);
        if invisible {
            self.diagnostics.invisible_glyphs += 1;
        }

        let x = trm.e;
        let y = trm.f;
        let size = trm.y_scale();
        let advance = tx * tm_ctm.x_scale() * if tx < 0.0 { -1.0 } else { 1.0 };

        // `Tm` at the instant this glyph is shown — captured BEFORE the
        // advance below, because that is the matrix the glyph was placed
        // by (and the one the surgery must reproduce). Any TJ pre-shift is
        // already folded in: `show_array` applied it to `self.tm` before
        // calling here.
        let text_matrix_at_show = self.tm;

        // Advance first, so a suppressed glyph still moves the matrix.
        self.tm = Matrix::translate(tx, 0.0).mul(self.tm);

        // Inside an /ActualText sequence the glyphs are REPLACED, not
        // shown: accumulate their extent for the replacement run's bbox
        // and emit nothing (§14.9.4).
        if self.actual_text_active() {
            self.extend_covered(x, y, advance, size);
            return;
        }

        let artifact = self.artifact();
        if artifact.is_some() {
            self.diagnostics.artifact_chars += chars.chars().count() as u64;
        }
        // Provenance is built only on demand (default off), so the Pass 4
        // output is byte-for-byte unchanged for callers that do not ask for
        // it. When asked, it is a snapshot of SOURCED state — operator
        // span, governing font/size, fill colour, matrices — never derived.
        let provenance = if self.options.capture_provenance {
            Some(GlyphProvenance {
                content_stream: self.stream_ref,
                operator_span: self.cur_op_span,
                font_resource: self.cur_font_resource.clone(),
                tf_size: self.ts.size,
                fill_color: self.ts.fill_color,
                text_matrix: text_matrix_at_show.to_array(),
                ctm: self.ctm.to_array(),
            })
        } else {
            None
        };
        self.items.push(Item::Glyph(GlyphItem {
            chars,
            code,
            rung,
            x,
            y,
            advance,
            size,
            invisible,
            artifact,
            mcid: self.mcid(),
            provenance,
        }));
    }

    // -----------------------------------------------------------------
    // Marked content
    // -----------------------------------------------------------------

    /// `BDC`'s `properties` operand: an inline dictionary, or a name to
    /// resolve against the **current** resource dictionary's
    /// `/Properties` (§14.6.2).
    ///
    /// §14.6 N2: a name absent from `/Properties` is legal-by-silence;
    /// pdfce treats it as an empty property list.
    fn resolve_properties(&self, op: &Operation<'_>, resources: &Dict) -> Option<Dict> {
        match op.operands.last().and_then(operand_object)? {
            Object::Dict(d) => Some(d.clone()),
            Object::Name(n) => self
                .doc
                .resolve(resources.get(b"Properties")?)
                .as_dict()?
                .get(n.as_bytes())
                .map(|o| self.doc.resolve(o))
                .and_then(Object::as_dict)
                .cloned(),
            _ => None,
        }
    }

    /// Push a marked-content level, reading the four tags that matter.
    ///
    /// **`/ActualText` nesting policy:** an `/ActualText` inside a
    /// sequence that already has one is IGNORED. §14.9.4 N2 records that
    /// no clause says which applies when an ancestor and a descendant
    /// both carry one; "innermost wins" is the obvious reading but,
    /// combined with Table 323 scoping the entry to "the structure
    /// element **and its children**", it would make the ancestor's value
    /// cover the descendant's region *as well*, emitting both and
    /// duplicating text. Outermost-wins cannot duplicate, so that is
    /// pdfce's rule, and the ignored inner values are counted.
    fn begin_marked(&mut self, tag: &[u8], props: Option<&Dict>) {
        // A hostile stream of BMCs must not grow the stack without
        // bound. 256 matches the graphics-state guard above.
        if self.marked.len() >= 256 {
            return;
        }
        let mut level = MarkedLevel {
            artifact: self.artifact(),
            mcid: self.mcid(),
            reversed_chars: self.reversed_chars(),
            actual_text: None,
            covered: None,
        };

        match tag {
            b"Artifact" => {
                self.diagnostics.artifact_sequences += 1;
                level.artifact = Some(artifact_kind(self.doc, props));
            }
            b"ReversedChars" => {
                self.diagnostics.reversed_chars_sequences += 1;
                level.reversed_chars = true;
            }
            b"TagSuspect" => {
                self.diagnostics.tag_suspect_sequences += 1;
                self.diagnostics.note(
                    "text: /TagSuspect /Ordering region — the producer declares the enclosed \
                     content's order does not meet Tagged PDF specifications (§14.8.2.3.1)"
                        .to_string(),
                );
            }
            _ => {}
        }

        if let Some(props) = props {
            // /MCID (§14.7.4.2) — the join key to the structure tree.
            if let Some(mcid) = props
                .get(b"MCID")
                .map(|o| self.doc.resolve(o))
                .and_then(Object::as_int)
                .and_then(|v| u32::try_from(v).ok())
            {
                level.mcid = Some(mcid);
            }
            // /Alt and /E are counted, never substituted — see the
            // module docs on `super`.
            if props.get(b"Alt").is_some() {
                self.diagnostics.alt_entries += 1;
            }
            if props.get(b"E").is_some() {
                self.diagnostics.expansion_entries += 1;
            }
            if let Some(Object::String(bytes)) =
                props.get(b"ActualText").map(|o| self.doc.resolve(o))
            {
                // §14.9.4 names `/Span` normatively for the
                // marked-content form. Real producers attach
                // /ActualText to other tags (N5); pdfce honours it
                // wherever it appears — dropping recoverable text over a
                // tag name would be the worse error — and says so once.
                if tag != b"Span" {
                    self.diagnostics.note(format!(
                        "text: /ActualText on a /{} marked-content sequence — §14.9.4 names /Span \
                         normatively; honoured anyway",
                        String::from_utf8_lossy(tag)
                    ));
                }
                if self.actual_text_active() {
                    self.diagnostics.note(
                        "text: nested /ActualText — the outermost value covers the region \
                         (§14.9.4 N2 states no nesting rule); the inner value was not applied"
                            .to_string(),
                    );
                } else {
                    let decoded = decode_text_string(bytes);
                    if decoded.text.is_empty() {
                        self.diagnostics.actual_text_suppressions += 1;
                    } else {
                        self.diagnostics.actual_text_applied += 1;
                    }
                    level.actual_text = Some(decoded.text);
                }
            }
        }

        self.marked.push(level);
    }

    /// `EMC`: pop a level and, if it carried `/ActualText`, emit the
    /// replacement run now — at the end of the region it covered, which
    /// is where its characters belong in page content order.
    fn end_marked(&mut self) {
        // §14.6 N1: an EMC with an empty stack is unbalanced and the
        // standard states no recovery. Ignoring it cannot lose content.
        let Some(level) = self.marked.pop() else {
            return;
        };
        let Some(text) = level.actual_text else {
            return;
        };
        if text.is_empty() {
            // An empty /ActualText suppressed its content deliberately
            // (N7); emitting an empty run would be noise.
            return;
        }
        self.items.push(Item::Replacement(ReplacementItem {
            text,
            artifact: level.artifact,
            mcid: level.mcid,
            bbox: level.covered,
        }));
    }

    /// The innermost enclosing artifact classification.
    fn artifact(&self) -> Option<ArtifactKind> {
        self.marked.last().and_then(|l| l.artifact.clone())
    }

    /// The innermost enclosing `/MCID`.
    fn mcid(&self) -> Option<u32> {
        self.marked.last().and_then(|l| l.mcid)
    }

    /// Whether any enclosing sequence is `/ReversedChars`.
    fn reversed_chars(&self) -> bool {
        self.marked.last().is_some_and(|l| l.reversed_chars)
    }

    /// Whether any enclosing sequence is replacing its content.
    fn actual_text_active(&self) -> bool {
        self.marked.iter().any(|l| l.actual_text.is_some())
    }

    /// Grow the bounding box of the outermost active `/ActualText`
    /// level to include one suppressed glyph.
    fn extend_covered(&mut self, x: f32, y: f32, advance: f32, size: f32) {
        let Some(level) = self.marked.iter_mut().find(|l| l.actual_text.is_some()) else {
            return;
        };
        let (x0, x1) = (f64::from(x), f64::from(x + advance));
        // The glyph box is approximated as one em tall from the
        // baseline, with a quarter-em descender — enough to locate a
        // replacement run on the page, which is all §14.9.4 N4 permits
        // anyway.
        let (y0, y1) = (f64::from(y - size * 0.25), f64::from(y + size * 0.75));
        level.covered = Some(match level.covered {
            None => Rect::from_corners(x0, y0, x1, y1),
            Some(r) => Rect {
                llx: r.llx.min(x0.min(x1)),
                lly: r.lly.min(y0),
                urx: r.urx.max(x0.max(x1)),
                ury: r.ury.max(y1),
            },
        });
    }

    // -----------------------------------------------------------------
    // Form XObjects (§8.10.1)
    // -----------------------------------------------------------------

    /// `Do`: execute a form XObject's content with its own `/Resources`
    /// and `/Matrix`.
    ///
    /// Image XObjects are skipped (they hold no text). The recursion
    /// follows §8.10.1's five-step procedure in the parts that matter
    /// here: save state, concatenate `/Matrix`, execute with the form's
    /// resource dictionary, restore state.
    fn do_xobject(&mut self, op: &Operation<'_>, _buf: &[u8], resources: &Dict) {
        let Some(name) = operand_name(op, 0) else {
            return;
        };
        // Copy the document reference out of `self` first: everything
        // below reads through it while later lines take `&mut self`, and
        // the copy makes those two borrows provably independent.
        let doc = self.doc;
        let Some(entry) = doc
            .resolve(resources.get(b"XObject").unwrap_or(&Object::Null))
            .as_dict()
            .and_then(|d| d.get(&name))
        else {
            return;
        };
        // The object number, if this was an indirect reference — the
        // cycle guard's key.
        let obj_num = entry.as_reference().map(|id| id.num);
        let Object::Stream(stream) = doc.resolve(entry) else {
            return;
        };
        if doc
            .resolve(stream.dict.get(b"Subtype").unwrap_or(&Object::Null))
            .as_name()
            .is_none_or(|n| n.as_bytes() != b"Form")
        {
            return;
        }
        if self.depth >= self.options.max_form_depth {
            self.diagnostics.form_depth_overflows += 1;
            self.diagnostics.note(format!(
                "text: form XObject nesting exceeded {} levels — the deeper content was not \
                 extracted",
                self.options.max_form_depth
            ));
            return;
        }
        // §8.10.1 cycle guard, keyed on object number rather than
        // resource name: the same stream can be reached under different
        // names, and a name-keyed guard would miss the cycle.
        if let Some(num) = obj_num {
            if self.active_xobjects.contains(&num) {
                return;
            }
            self.active_xobjects.push(num);
        }

        let inner_resources = doc
            .resolve(stream.dict.get(b"Resources").unwrap_or(&Object::Null))
            .as_dict()
            .cloned()
            .unwrap_or_else(|| resources.clone());

        let content = stream
            .data_span
            .slice(doc.bytes())
            .and_then(|raw| crate::filters::decode_stream(&stream.dict, raw).ok())
            .and_then(|decoded| ContentStream::parse(decoded).ok());

        if let Some(content) = content {
            self.diagnostics.forms_executed += 1;
            let saved_ctm = self.ctm;
            let saved_tm = self.tm;
            let saved_tlm = self.tlm;
            let saved_ts = self.ts.clone();
            let saved_marked_depth = self.marked.len();
            // Provenance spans inside the form index the FORM's own decoded
            // buffer (§8.10.1 — a separate content stream), so the walk
            // switches its stream reference for the duration and restores
            // it on return. The font-resource mirror is restored too, since
            // a `Tf` inside the form selected from the form's /Resources.
            let saved_stream_ref = self.stream_ref;
            let saved_op_span = self.cur_op_span;
            let saved_font_resource = self.cur_font_resource.clone();
            if let Some(num) = obj_num {
                self.stream_ref = ContentStreamRef::Form { object: num };
            }

            if let Some(m) = matrix_of(doc, &stream.dict) {
                self.ctm = m.mul(self.ctm);
            }
            self.depth += 1;
            self.run(&content, &inner_resources);
            self.depth -= 1;

            // §8.10.1 steps (a)/(e): the form's state changes cannot
            // escape. Restoring explicitly rather than relying on the
            // form's own q/Q balance makes that structural — an
            // unbalanced `Q` inside a form provably cannot pop the
            // caller's state.
            self.ctm = saved_ctm;
            self.tm = saved_tm;
            self.tlm = saved_tlm;
            self.ts = saved_ts;
            self.marked.truncate(saved_marked_depth);
            self.stream_ref = saved_stream_ref;
            self.cur_op_span = saved_op_span;
            self.cur_font_resource = saved_font_resource;
        }

        if obj_num.is_some() {
            self.active_xobjects.pop();
        }
    }
}

// ---------------------------------------------------------------------------
// Operand helpers
// ---------------------------------------------------------------------------

/// The object carried by a content token, if it is an operand.
fn operand_object(token: &crate::content::ContentToken) -> Option<&Object> {
    match &token.kind {
        ContentTokenKind::Operand(o) => Some(o),
        _ => None,
    }
}

/// The last `count` numeric operands, in order. Returns fewer than
/// `count` (and the caller's slice pattern then fails to match) when the
/// operator is malformed.
fn operand_numbers(op: &Operation<'_>, count: usize) -> Vec<f32> {
    let start = op.operands.len().saturating_sub(count);
    op.operands
        .get(start..)
        .unwrap_or(&[])
        .iter()
        .filter_map(operand_object)
        .filter_map(|o| o.as_number())
        .map(|v| v as f32)
        .collect()
}

/// The operand at `index` as a name's bytes.
fn operand_name(op: &Operation<'_>, index: usize) -> Option<Vec<u8>> {
    op.operands
        .get(index)
        .and_then(operand_object)
        .and_then(Object::as_name)
        .map(|n| n.as_bytes().to_vec())
}

/// The last string operand.
fn last_string(op: &Operation<'_>) -> Option<Vec<u8>> {
    match op.operands.last().and_then(operand_object)? {
        Object::String(s) => Some(s.clone()),
        _ => None,
    }
}

/// Table 330's `/Type`, or `Unspecified` for the generic form.
fn artifact_kind(doc: &Document, props: Option<&Dict>) -> ArtifactKind {
    let Some(props) = props else {
        return ArtifactKind::Unspecified;
    };
    match props.get(b"Type").map(|o| doc.resolve(o)) {
        Some(Object::Name(n)) => match n.as_bytes() {
            b"Pagination" => ArtifactKind::Pagination,
            b"Layout" => ArtifactKind::Layout,
            b"Page" => ArtifactKind::Page,
            b"Background" => ArtifactKind::Background,
            other => ArtifactKind::Other(String::from_utf8_lossy(other).into_owned()),
        },
        _ => ArtifactKind::Unspecified,
    }
}

/// A form XObject's `/Matrix` (Table 95; default identity).
fn matrix_of(doc: &Document, dict: &Dict) -> Option<Matrix> {
    let items = doc.resolve(dict.get(b"Matrix")?).as_array()?;
    let v: Vec<f32> = items
        .iter()
        .filter_map(|o| doc.resolve(o).as_number())
        .map(|n| n as f32)
        .collect();
    match v[..] {
        [a, b, c, d, e, f] => Some(Matrix { a, b, c, d, e, f }),
        _ => None,
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

    #[test]
    fn matrix_multiplication_follows_pdf_row_vector_convention() {
        // Translate then scale: the translation is scaled too.
        let t = Matrix::translate(10.0, 0.0);
        let s = Matrix {
            a: 2.0,
            b: 0.0,
            c: 0.0,
            d: 2.0,
            e: 0.0,
            f: 0.0,
        };
        let m = t.mul(s);
        assert!((m.e - 20.0).abs() < 1e-6);
        assert!((m.a - 2.0).abs() < 1e-6);
    }

    #[test]
    fn scales_measure_transformed_unit_vectors() {
        let m = Matrix {
            a: 3.0,
            b: 4.0,
            c: 0.0,
            d: 5.0,
            e: 0.0,
            f: 0.0,
        };
        assert!((m.x_scale() - 5.0).abs() < 1e-6);
        assert!((m.y_scale() - 5.0).abs() < 1e-6);
    }
}
