//! # Text extraction — the §9.10 pipeline, with its seams exposed
//!
//! Turning a page's glyphs back into characters. This is the
//! **extraction** direction: the same font dictionary, the same content
//! stream and the same show strings as `pdfce-render`, walked toward the
//! opposite answer. §9.10.1 states why they cannot be one pipeline:
//!
//! > "Unicode values identify characters, not glyphs."
//!
//! Rendering asks *which glyph*; extraction asks *which character*. The
//! two diverge at the font dictionary and never rejoin. Extraction
//! therefore lives here in `pdfce-core` — it is document analysis, not
//! rasterization — and needs no font program for three of the ladder's
//! four rungs.
//!
//! Spec sources in the PDF-spec RAG, starting with the purpose-built
//! consolidator `iso32000__ref__text_extraction.md`: `__s__9.10.md`
//! (the ladder), `__s__9.10.3.md` (`ToUnicode` CMaps), `__s__7.9.2.md`
//! with `__annex__d3.md` (text strings, PDFDocEncoding), `__s__14.6.md`
//! (marked content), `__s__14.8.md` (Tagged PDF, artifacts, order, word
//! breaks), `__s__14.9.4.md` (`/ActualText`), `__s__9.4.md` (the text
//! object and its matrices).
//!
//! ## The one idea this module is built around: SOURCED vs DERIVED
//!
//! Almost everything a naïve extractor presents as "the text of this
//! page" is a mixture of two very different things, and ISO 32000-1 is
//! unusually explicit about which is which. The single most useful table
//! in the spec RAG's consolidator:
//!
//! | Aspect | Tagged PDF | Untagged | Clause |
//! |---|---|---|---|
//! | code → Unicode | **SOURCED** (`shall` be mappable) | DERIVED where the ladder fails | §14.8.2.4.2 / §9.10.2 |
//! | inter-word spaces | **SOURCED** — "the spacing characters … shall be present"; the reader "does not need to guess" | **DERIVED, entirely** — no clause requires any space signal | §14.8.2.5 |
//! | line breaks | **DERIVED even in Tagged PDF** | DERIVED | §14.8 S5 |
//! | reading order | SOURCED via the structure tree | DERIVED | §14.8.2.3.1 |
//!
//! and the consolidated negative results behind it are blunt:
//!
//! - **S2** — no interword-space guarantee outside Tagged PDF.
//! - **S3** — a space may be a glyph, a `TJ` offset, a `Td`/`Tm` jump, a
//!   new `BT` block, or **nothing**; the standard assigns word-break
//!   meaning to none of them.
//! - **S4** — `TJ` negative-offset thresholds are reader heuristics with
//!   **zero** spec basis. Table 109 defines `TJ` numbers purely as a
//!   text-matrix translation, and the standard's own illustration of
//!   them is *kerning* (`[(A) 120 (W) …]`), an intra-word use.
//! - **S5** — no line or paragraph markers exist in a content stream.
//! - **S6** — `Tw` is not a word-break signal, and is inert under
//!   `Identity-H` (multi-byte codes), i.e. in modern documents.
//! - **S9** — no definition of *word*, *line*, *paragraph*, *column* or
//!   *reading order* exists for an untagged document at all.
//!
//! So pdfce **labels the seam instead of hiding it**. Every character in
//! the output belongs to a [`TextRun`] whose [`TextRun::origin`] says
//! where it came from, and every derived judgement is counted in
//! [`TextDiagnostics`]. Two accessors make the distinction operational:
//!
//! - [`ExtractedText::plain_text`] — what a "Copy page text" button
//!   produces: sourced characters plus pdfce's derived whitespace.
//! - [`ExtractedText::sourced_text`] — **only** what the file actually
//!   says, with every derived space and line break omitted.
//!
//! This is rule 4 ("fuzzy, never sneaky") applied literally: the guesses
//! are still made — an extractor that refused to guess would emit
//! `Helloworld` — but they are made *visibly*, and a caller that wants
//! only the sourced content can have exactly that.
//!
//! ## Pipeline
//!
//! ```text
//! page /Contents  ──► ContentStream tokens (crate::content, lossless)
//!    │
//!    ├─ graphics state: q/Q/cm ─────────────► CTM
//!    ├─ marked-content stack: BMC/BDC/EMC ──► /Artifact, /Span+/ActualText,
//!    │                                        /ReversedChars, /MCID, /TagSuspect
//!    └─ text object: BT … ET (§9.4)
//!        ├─ Tf ──► font dict ──► ExtractFont (the §9.10.2 ladder, `font.rs`)
//!        └─ Tj ' " TJ ──► codes ──► ladder ──► characters + positions
//!                                     │
//!                                     └─► layout.rs: DERIVED word/line breaks
//! ```
//!
//! Form XObjects (`Do`) are executed with their own `/Resources` and
//! `/Matrix`, bounded by [`ExtractOptions::max_form_depth`] — Pass 1.1
//! measured 1,168 form executions across the corpus, so text inside
//! forms is not an edge case.
//!
//! ## What this Pass ships, and what it names instead of shipping
//!
//! | Capability | Status |
//! |---|---|
//! | Ladder rungs 1, 2, 4 | complete |
//! | Ladder rung 3 (CJK collections) | **structural + named diagnostic** — the `registry-ordering-UCS2` CMaps are Adobe resource files pdfce does not bundle |
//! | `/ActualText` replacement | complete, counted |
//! | `/Alt`, `/E` | **recognized and counted, not substituted** — see below |
//! | Artifact classification | complete; excluded from `plain_text` by default, always present in [`PageText::runs`] |
//! | `/ReversedChars` | complete (per-string reversal) |
//! | Structure-tree reading order | **recognition only** — presence, `/Suspects`, counts; traversal deferred by name |
//! | Bidi logical reordering | **detection only** — deferred by name, see below |
//!
//! ### Why `/Alt` and `/E` are not substituted
//!
//! §14.9.3 makes `Alt` "a complete (or whole) word or phrase
//! **substitution**" and §14.9.5 makes `E` an expansion — but §14.9.3
//! NOTE 1 says alternate descriptions are for items "that **do not**
//! translate naturally into text", i.e. they *describe* content rather
//! than *replace* it. §14.9.4 N1 records that no clause states a
//! precedence between `ActualText` and `Alt` on the same element. pdfce
//! therefore uses `ActualText` for the character stream and treats
//! `Alt`/`E` as an accessibility surface, counting both
//! ([`TextDiagnostics::alt_entries`], [`TextDiagnostics::expansion_entries`])
//! so an operator can see that a page carries description text pdfce did
//! not put in the output. Product policy; nothing to cite either way.
//!
//! ### Why bidi is deferred rather than half-done
//!
//! R17 permits `unicode-bidi` **only** in this path (never in
//! `pdfce-render`), and the deferral is not a licensing problem — it is
//! that ISO 32000-1's own position is a hard negative result: **B1**
//! UAX #9 is cited exactly once in the whole standard, in a *layout*
//! attribute row; **B2** the standard's only RTL mechanism is
//! `ReversedChars`, which pdfce implements; **B3** visual→logical
//! reordering is specified nowhere. Applying UAX #9 to extracted text is
//! therefore a wholly derived transformation, and shipping it in the
//! same Pass that first draws the sourced/derived line would blur the
//! line it exists to draw. pdfce detects RTL characters, counts them
//! ([`TextDiagnostics::rtl_runs`]) and names the deferral. Adding the
//! dependency later changes no existing output.

pub mod cmap;
pub mod font;
mod layout;
mod page;

use std::fmt;

use crate::content::ContentError;
use crate::document::Document;
use crate::page_tree::{self, Page, PageTreeError, Rect};
use crate::span::ByteSpan;
use crate::view::DocumentView;

pub use font::{ExtractFont, FontNote, LadderRung, Rung3Gap};

/// Where a run of extracted characters came from.
///
/// The distinction between the first two variants and the last two is
/// the whole point of the module (see the module docs): the first two
/// are characters the *file* provides, the last two are pdfce's own
/// judgement about how those characters are separated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TextOrigin {
    /// Characters decoded from shown glyphs through the §9.10.2 ladder.
    /// [`TextRun::glyphs`] is populated and positionally meaningful.
    ///
    /// Note that *sourced run* does not mean *sourced character*: a run
    /// may contain U+FFFD from [`LadderRung::Failed`]. Per-character
    /// provenance is in [`ExtractedGlyph::rung`].
    Glyphs,
    /// Characters supplied by an `/ActualText` entry (§14.9.4) covering
    /// a marked-content sequence.
    ///
    /// **The run is atomic.** [`TextRun::glyphs`] is empty, deliberately:
    /// §14.9.4 N4 records that no length relationship exists between the
    /// replacement and the replaced content — the standard's own example
    /// replaces two shown characters (`k-`) with one (`c`) — so
    /// character-level mapping back to glyph positions is *impossible*,
    /// not merely unimplemented. A search-highlight or redaction-by-text
    /// feature can locate a hit inside such a run only at
    /// [`TextRun::bbox`] granularity.
    ActualText,
    /// Whitespace pdfce inserted because two glyphs' geometry implied a
    /// word gap. **DERIVED** — S1–S4: no clause requires any inter-word
    /// signal outside Tagged PDF, and `TJ` offset thresholds have zero
    /// spec basis.
    DerivedWordSpace,
    /// A line break pdfce inserted because the baseline moved.
    /// **DERIVED** — S5: no line or paragraph markers exist in a content
    /// stream, in tagged or untagged files alike.
    DerivedLineBreak,
}

impl TextOrigin {
    /// Whether the characters in this run came from the file rather than
    /// from pdfce's segmentation heuristics.
    #[must_use]
    pub const fn is_sourced(self) -> bool {
        matches!(self, Self::Glyphs | Self::ActualText)
    }

    /// A short stable identifier for machine-readable output.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Glyphs => "glyphs",
            Self::ActualText => "actual_text",
            Self::DerivedWordSpace => "derived_word_space",
            Self::DerivedLineBreak => "derived_line_break",
        }
    }
}

/// Table 330's artifact classification (§14.8.2.2.2).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ArtifactKind {
    /// "Ancillary page features such as running heads and folios (page
    /// numbers)." Note §14.8.2.2's NOTE 2: a **watermark is a
    /// `Pagination` artifact**, not a `Background` one.
    Pagination,
    /// "Purely cosmetic typographical or design elements such as
    /// footnote rules or background screens."
    Layout,
    /// "Production aids extraneous to the document itself, such as cut
    /// marks and colour bars."
    Page,
    /// (PDF 1.7) "Images, patterns or coloured blocks" behind content.
    Background,
    /// `/Artifact BMC` with no property list, or a property list with no
    /// `/Type`. Table 330 makes `/Type` optional, and §14.8.2.2.2's
    /// first form is explicitly "a generic artifact".
    Unspecified,
    /// A `/Type` value outside Table 330's four names.
    Other(String),
}

impl ArtifactKind {
    /// A short stable identifier for machine-readable output.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Pagination => "pagination",
            Self::Layout => "layout",
            Self::Page => "page",
            Self::Background => "background",
            Self::Unspecified => "unspecified",
            Self::Other(name) => name,
        }
    }
}

/// Which decoded content buffer a [`GlyphProvenance::operator_span`]
/// indexes.
///
/// A page's own `/Contents` streams are concatenated into ONE decoded
/// buffer (§7.7.3.3); a form XObject executed via `Do` (§8.10.1) is a
/// **separate** decoded buffer with its own coordinate system. An operator
/// byte span is meaningless without the buffer it indexes, so provenance
/// names it. The later in-place edit surgery (decision 014, Pass 14.1)
/// re-tokenizes exactly the named stream and locates the operator by
/// [`GlyphProvenance::operator_span`].
///
/// This is provenance, not a derived judgement: it records where in the
/// file a glyph physically came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ContentStreamRef {
    /// The page's own concatenated `/Contents` buffer.
    Page,
    /// A form XObject's decoded content buffer, identified by the object
    /// number of the form stream (§8.10.1). The span indexes THAT form's
    /// decoded bytes, not the page's — the two are different buffers.
    Form {
        /// The form XObject stream's object number.
        object: u32,
    },
}

/// A fill colour captured from the graphics state at the instant a glyph
/// was shown (§8.6.8; text is painted in the current *fill* colour under
/// the default text-rendering modes 0/2/4/6).
///
/// **Provenance only, and deliberately PARTIAL.** Pass 14.0 reads the
/// three *device* colour operators — `g` (DeviceGray, §8.6.4.2), `rg`
/// (DeviceRGB, §8.6.4.3) and `k` (DeviceCMYK, §8.6.4.4) — because each
/// names its own colour space inline and needs no `/ColorSpace` resource
/// lookup. A fill colour set through `sc`/`scn` in a resource-named space
/// (ICCBased, Separation, DeviceN, Pattern, Indexed, a CalRGB, …) is
/// recorded as [`TextColor::Other`] rather than decoded: the value is
/// present, but interpreting it needs the colour-space machinery that is
/// out of this read-only Pass's scope. The point of the `Other` variant is
/// rule 4 (fuzzy-never-sneaky): a colour that pdfce did not model is
/// reported as unmodelled, never silently flattened to black.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum TextColor {
    /// DeviceGray — one component in `0.0..=1.0` (`g`).
    Gray(f32),
    /// DeviceRGB — three components in `0.0..=1.0` (`rg`).
    Rgb(f32, f32, f32),
    /// DeviceCMYK — four components in `0.0..=1.0` (`k`).
    Cmyk(f32, f32, f32, f32),
    /// A fill colour set in a colour space this read-only Pass does not
    /// decode (see the type docs). Present-but-unmodelled.
    Other,
}

/// Per-glyph **provenance**: the source-operator identity and full text
/// state behind one shown glyph — the substrate the later in-place edit
/// surgery (decision 014, Pass 14.1) needs to LOCATE and RE-ENCODE a run
/// without disturbing its neighbours.
///
/// Populated **only** when [`ExtractOptions::capture_provenance`] is set
/// (default off): with the flag off, [`ExtractedGlyph::provenance`] is
/// `None` for every glyph and the Pass 4 extraction output is byte-for-byte
/// unchanged for every existing caller. Everything in here is SOURCED from
/// the content stream and graphics state — none of it is derived, in
/// deliberate contrast to the entirely-derived block model in
/// [`crate::text_edit`].
///
/// The matrices are in PDF's 6-element `[a b c d e f]` row-vector form
/// (§8.3.3): [`Self::text_matrix`] is `Tm` at the instant the glyph was
/// shown (before its own §9.4.4 advance), and [`Self::ctm`] is the current
/// transformation matrix then in effect. Together with [`Self::tf_size`]
/// (the raw `Tfs` from `Tf`, §9.3.1 — NOT the matrix-folded effective size
/// in [`ExtractedGlyph::size`]) they reconstruct the exact §9.4.4 text
/// rendering matrix the surgery must reproduce.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct GlyphProvenance {
    /// Which decoded content buffer [`Self::operator_span`] indexes.
    pub content_stream: ContentStreamRef,
    /// Byte span of the show operator (`Tj`/`'`/`"`/`TJ`) that produced
    /// this glyph, within the decoded buffer named by
    /// [`Self::content_stream`]. Every glyph of one operator shares this
    /// span — a `TJ` array is a single operator (§9.4.3).
    pub operator_span: ByteSpan,
    /// The font resource name selected by the governing `Tf` (§9.3.1): the
    /// `/F1`-style key into the current `/Resources` `/Font` subdictionary,
    /// as raw name bytes (a PDF name is a byte string, §7.3.5). `None` only
    /// if the glyph was shown with no `Tf` in effect (malformed, §9.4.1).
    pub font_resource: Option<Vec<u8>>,
    /// The raw `Tfs` argument of the governing `Tf` (§9.3.1), in unscaled
    /// text-space units — the number the surgery re-emits, distinct from
    /// the effective size that folds in the matrices' scale.
    pub tf_size: f32,
    /// The fill colour in effect (§8.6.8), or `None` when none was set and
    /// the §8.6.8 default (black `DeviceGray 0`) applies. See [`TextColor`]
    /// for why it is deliberately partial in this Pass.
    pub fill_color: Option<TextColor>,
    /// `Tm` at the instant this glyph was shown, `[a b c d e f]` (§9.4.2).
    pub text_matrix: [f32; 6],
    /// The CTM in effect when this glyph was shown, `[a b c d e f]`
    /// (§8.3.4).
    pub ctm: [f32; 6],
}

/// One glyph's contribution to a [`TextRun`], with its provenance and
/// its place on the page.
///
/// No longer `Copy`: [`Self::provenance`] carries an owned font-resource
/// name (§9.3.1) when provenance capture is enabled, so the struct owns a
/// heap allocation. It stays `Clone`. Every workspace consumer accesses
/// glyphs by reference, so dropping `Copy` is transparent.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ExtractedGlyph {
    /// The character code as it appeared in the show string: one byte
    /// for a simple font, two big-endian bytes for a composite one.
    pub code: u32,
    /// Which §9.10.2 rung produced this glyph's characters.
    pub rung: LadderRung,
    /// Byte offset of this glyph's characters within [`TextRun::text`].
    pub text_start: u32,
    /// Byte length of this glyph's characters within [`TextRun::text`].
    ///
    /// **Not one.** One code may produce many code points — §9.10.3's
    /// own example decomposes `ffl` from a single code — so a
    /// per-character index into the run's text is wrong by construction.
    pub text_len: u32,
    /// Glyph origin x in **default user space** (§9.4.4's text rendering
    /// matrix applied to the origin).
    pub x: f32,
    /// Glyph origin y in default user space.
    pub y: f32,
    /// Horizontal advance in default user space, including `Tc`, `Tw`,
    /// `Tz` and any `TJ` adjustment attributed to this glyph.
    pub advance: f32,
    /// Effective font size in default user space (the y-scale of the
    /// text rendering matrix). Drives the derived line/word thresholds
    /// and is what §14.8.2.4.3 means by a `FontSize` "derived from the
    /// a, b, c and d fields of the current text matrix" rather than from
    /// `Tfs` alone.
    pub size: f32,
    /// `true` when the glyph was shown in text rendering mode 3
    /// (invisible) or 7 (clip).
    ///
    /// It is still **real content**: §14.8.2.2.3 item 3 is a `shall` —
    /// "page content shall be considered to include all text and
    /// illustrations in their entirety, regardless of whether they are
    /// visible". This flag exists because invisible text is the OCR
    /// "sandwich" convention and because redaction verification needs to
    /// know that covered-but-present text is still extractable, not so
    /// that a caller can drop it.
    pub invisible: bool,
    /// Source-operator identity and text state behind this glyph — the
    /// substrate later edit surgery needs (decision 014, Pass 14.1).
    ///
    /// `None` unless the extraction set [`ExtractOptions::capture_provenance`];
    /// this keeps the default Pass 4 output byte-for-byte unchanged. When
    /// present it is SOURCED, never derived. See [`GlyphProvenance`].
    pub provenance: Option<GlyphProvenance>,
}

/// One contiguous run of extracted text sharing an origin and a
/// marked-content context.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TextRun {
    /// The run's characters.
    pub text: String,
    /// Where they came from.
    pub origin: TextOrigin,
    /// Per-glyph provenance and geometry. Non-empty **only** for
    /// [`TextOrigin::Glyphs`] — see [`TextOrigin::ActualText`] for why.
    pub glyphs: Vec<ExtractedGlyph>,
    /// The enclosing `/Artifact` classification, if the run sits inside
    /// one (§14.8.2.2.2).
    ///
    /// Artifact runs are **always present** in [`PageText::runs`]; only
    /// [`ExtractedText::plain_text`] filters them, and only when
    /// [`ExtractOptions::include_artifacts`] is false. §14.8.2.2's A1/A3
    /// are the reason: no `shall` requires a reader to exclude artifacts
    /// from extracted text — every reader-side verb in that clause is
    /// `may`/`can`/`probably should` — so exclusion is pdfce policy, and
    /// policy must be reversible by the caller.
    pub artifact: Option<ArtifactKind>,
    /// The enclosing `/MCID` (§14.7.4.2), the join key to the structure
    /// tree. Recorded now so a later structure-order Pass has it; not
    /// used for ordering this Pass.
    pub mcid: Option<u32>,
    /// Bounding box in default user space, when the run has geometry.
    /// `None` for derived-whitespace runs and for an `/ActualText` run
    /// that covered no glyphs.
    pub bbox: Option<Rect>,
}

impl TextRun {
    /// Whether every character in this run came from the file.
    #[must_use]
    pub const fn is_sourced(&self) -> bool {
        self.origin.is_sourced()
    }
}

/// Everything one page's extraction produced.
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct PageText {
    /// Zero-based index of the page in the document.
    pub page_index: usize,
    /// The runs, in **page content order** — the sequencing of graphics
    /// objects in the content stream (§14.8.2.3.1), which is one of the
    /// two orderings the standard defines and the only one available
    /// without a structure tree. It relates to appearance only through a
    /// writer `should`, and "the two orderings … may or may not
    /// coincide".
    pub runs: Vec<TextRun>,
    /// What this page's extraction had to derive, tolerate or defer.
    pub diagnostics: TextDiagnostics,
    /// Whether [`PageText::plain_text`] includes artifact runs.
    ///
    /// Captured from [`ExtractOptions::include_artifacts`] at extraction
    /// time and kept private on purpose: the text accessors take no
    /// arguments, so this must be a property of the *result*, and a
    /// caller that could flip it after the fact would be able to make
    /// `plain_text` disagree with the counters in `diagnostics`.
    include_artifacts: bool,
}

/// Everything a document's extraction produced.
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct ExtractedText {
    /// One entry per page extracted, in page order.
    pub pages: Vec<PageText>,
    /// The union of every page's diagnostics, plus the document-level
    /// facts (tagged, `/Suspects`, structure tree present).
    pub diagnostics: TextDiagnostics,
    /// Whether [`ExtractedText::plain_text`] includes artifact runs —
    /// see [`PageText`]'s field of the same name.
    include_artifacts: bool,
}

/// What an extraction had to derive, tolerate, or defer.
///
/// Every counter here corresponds to a specific clause or to a specific
/// sourced *silence* in ISO 32000-1. This struct is the mechanism that
/// makes "fuzzy, never sneaky" checkable rather than aspirational: if
/// pdfce guessed, the guess is counted; if pdfce could not climb a rung,
/// the rung is named.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct TextDiagnostics {
    /// Character codes taken off show strings, total.
    pub codes_total: u64,
    /// Codes resolved by ladder rung 1 (`/ToUnicode`). Sourced.
    pub via_to_unicode: u64,
    /// Codes resolved by ladder rung 2 (encoding + AGL). Sourced.
    pub via_encoding_agl: u64,
    /// Codes resolved by ladder rung 3. **Always zero this Pass** — see
    /// [`Rung3Gap`]. Present so the hole is visible in every report.
    pub via_cid_collection: u64,
    /// Codes resolved by pdfce's counted glyph-name extension (the font
    /// failed §9.10.2 method 2's whole-array precondition but the name
    /// mapped through the AGL anyway). **Not sourced.**
    pub via_glyph_name_extension: u64,
    /// Codes that fell through the entire ladder to §9.10.2's failure
    /// clause and became U+FFFD. **The headline honesty metric.**
    pub ladder_failures: u64,
    /// Distinct fonts that are `Identity-H`/`Identity-V` (or
    /// `Adobe-Identity-0`) with **no** `/ToUnicode` — the modern
    /// dead end. For these, "no Unicode is recoverable" is §9.10.2's own
    /// answer, not a pdfce limitation.
    pub identity_fonts_without_to_unicode: u64,
    /// Distinct fonts whose descendant declares one of rung 3's four
    /// Adobe collections, for which the `registry-ordering-UCS2` CMap is
    /// not bundled.
    pub ucs2_cmaps_unavailable: u64,
    /// Distinct fonts using a Table 118 predefined CMap pdfce does not
    /// bundle.
    pub predefined_cmaps_unavailable: u64,
    /// `/ActualText` replacements applied (§14.9.4).
    pub actual_text_applied: u64,
    /// Empty `/ActualText` values (`()`), which suppressed their covered
    /// content. §14.9.4 N7: the standard says nothing about what an
    /// empty replacement means; treating it as suppression is policy.
    pub actual_text_suppressions: u64,
    /// `/Alt` entries seen and **not** substituted (see the module docs).
    pub alt_entries: u64,
    /// `/E` expansion entries seen and not substituted.
    pub expansion_entries: u64,
    /// Marked-content sequences tagged `/Artifact`.
    pub artifact_sequences: u64,
    /// Characters inside artifact sequences.
    pub artifact_chars: u64,
    /// `/ReversedChars` sequences whose show strings were reversed
    /// (§14.8.2.3.3).
    pub reversed_chars_sequences: u64,
    /// Word spaces pdfce **derived** from glyph geometry. DERIVED.
    pub spaces_derived: u64,
    /// Line breaks pdfce **derived** from baseline movement. DERIVED.
    pub lines_derived: u64,
    /// Runs containing right-to-left characters, for which logical-order
    /// reordering is deferred (see the module docs).
    pub rtl_runs: u64,
    /// Glyphs shown in text rendering mode 3 or 7 — invisible, yet real
    /// content per §14.8.2.2.3.
    pub invisible_glyphs: u64,
    /// Whether the document declares `/MarkInfo` `/Marked true`. When
    /// false, §14.8.1's four guarantees (T1–T4) do **not** hold and
    /// every segmentation judgement below is derived.
    pub tagged: bool,
    /// Whether `/MarkInfo` `/Suspects` is true — the producer's own
    /// disclaimer that its page content order does not meet Tagged PDF
    /// specifications (§14.8.2.3.1).
    pub suspects: bool,
    /// Whether the catalog carries a `/StructTreeRoot`. Recognition
    /// only: structure-order traversal is deferred by name this Pass.
    pub struct_tree_present: bool,
    /// `/TagSuspect` marked-content sequences encountered.
    pub tag_suspect_sequences: u64,
    /// Form XObjects executed while extracting.
    pub forms_executed: u64,
    /// Form XObject nestings refused by [`ExtractOptions::max_form_depth`].
    pub form_depth_overflows: u64,
    /// Content streams that could not be tokenized at all.
    pub pages_unreadable: u64,
    /// Distinct fonts for which advance widths had to be estimated (no
    /// `/Widths`, not a standard-14 face; the real metrics live in the
    /// font program, which `pdfce-core` cannot read — R21). Affects
    /// positions and therefore derived whitespace, never characters.
    pub fonts_with_estimated_widths: u64,
    /// Named, human-readable diagnostics, de-duplicated and in first-seen
    /// order. Same pattern as `pdfce-render`'s `image_notes`.
    pub notes: Vec<String>,
}

impl TextDiagnostics {
    /// Fold `other` into `self` — used to roll a page's counters into
    /// the document's, and a form XObject's into its parent page's.
    pub fn merge(&mut self, other: &Self) {
        self.codes_total += other.codes_total;
        self.via_to_unicode += other.via_to_unicode;
        self.via_encoding_agl += other.via_encoding_agl;
        self.via_cid_collection += other.via_cid_collection;
        self.via_glyph_name_extension += other.via_glyph_name_extension;
        self.ladder_failures += other.ladder_failures;
        self.identity_fonts_without_to_unicode += other.identity_fonts_without_to_unicode;
        self.ucs2_cmaps_unavailable += other.ucs2_cmaps_unavailable;
        self.predefined_cmaps_unavailable += other.predefined_cmaps_unavailable;
        self.actual_text_applied += other.actual_text_applied;
        self.actual_text_suppressions += other.actual_text_suppressions;
        self.alt_entries += other.alt_entries;
        self.expansion_entries += other.expansion_entries;
        self.artifact_sequences += other.artifact_sequences;
        self.artifact_chars += other.artifact_chars;
        self.reversed_chars_sequences += other.reversed_chars_sequences;
        self.spaces_derived += other.spaces_derived;
        self.lines_derived += other.lines_derived;
        self.rtl_runs += other.rtl_runs;
        self.invisible_glyphs += other.invisible_glyphs;
        self.tagged |= other.tagged;
        self.suspects |= other.suspects;
        self.struct_tree_present |= other.struct_tree_present;
        self.tag_suspect_sequences += other.tag_suspect_sequences;
        self.forms_executed += other.forms_executed;
        self.form_depth_overflows += other.form_depth_overflows;
        self.pages_unreadable += other.pages_unreadable;
        self.fonts_with_estimated_widths += other.fonts_with_estimated_widths;
        for note in &other.notes {
            self.note(note.clone());
        }
    }

    /// Record a named diagnostic, de-duplicated.
    ///
    /// De-duplication is by exact text, which is why every note that
    /// varies per font embeds the font name: two different broken fonts
    /// must produce two different lines, and the same broken font on 400
    /// pages must produce one.
    pub fn note(&mut self, text: String) {
        if !self.notes.contains(&text) {
            self.notes.push(text);
        }
    }

    /// Codes whose Unicode value ISO 32000-1 §9.10.2 itself sanctions.
    #[must_use]
    pub const fn sourced_codes(&self) -> u64 {
        self.via_to_unicode + self.via_encoding_agl + self.via_cid_collection
    }

    /// The fraction of character codes resolved by a sourced rung, in
    /// `0.0..=1.0`. `None` when no codes were seen at all (a page with
    /// no text — distinct from a page whose text all failed).
    ///
    /// This is the number to put in front of an operator: "94% of this
    /// document's characters are what the file says they are; 6% are
    /// U+FFFD because three fonts carry no Unicode information."
    #[must_use]
    pub fn sourced_fraction(&self) -> Option<f64> {
        if self.codes_total == 0 {
            return None;
        }
        Some(self.sourced_codes() as f64 / self.codes_total as f64)
    }
}

/// Knobs on the derived half of extraction.
///
/// The three ratios below have **no spec basis whatsoever** (S3/S4) and
/// are exposed rather than hard-coded precisely because of that: a
/// constant with no source is a constant that should be arguable. Their
/// defaults were chosen to be conservative — biased toward *not*
/// inserting a break — because a missing space is a visible defect the
/// operator can see and correct, while a spurious space silently breaks
/// a word in a search index.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ExtractOptions {
    /// Whether [`ExtractedText::plain_text`] includes artifact runs
    /// (running heads, folios, watermarks). Default `false`.
    ///
    /// Excluding them is **policy, not conformance**: §14.8.2.2's A1
    /// records that no `shall` requires a reader to exclude artifacts
    /// from extracted text. The runs are always in [`PageText::runs`]
    /// regardless.
    pub include_artifacts: bool,
    /// Insert a derived word space when the horizontal gap between two
    /// consecutive glyphs on one baseline exceeds this fraction of the
    /// effective font size. Default `0.20`.
    pub word_gap_ratio: f32,
    /// Treat a baseline movement larger than this fraction of the
    /// effective font size as a derived line break. Default `0.30`.
    pub line_gap_ratio: f32,
    /// Treat a backward horizontal jump larger than this fraction of the
    /// effective font size, on the same baseline, as a derived line
    /// break. Default `0.50`.
    ///
    /// Without this, a two-column page whose columns share baselines
    /// runs the two columns together into one line with no separator at
    /// all. §14.8.2.3.1 makes column ordering derived in an untagged
    /// file, so this is a guess about a thing the standard declines to
    /// define.
    pub backward_jump_ratio: f32,
    /// Maximum form-XObject nesting depth (§8.10.1). Default 64 —
    /// matching `pdfce-render`'s `MAX_XOBJECT_DEPTH`, which was itself
    /// **corpus-corrected**: a *conformant* veraPDF PDF/A file contains
    /// a deliberate 32-deep form chain, and Annex C sets no form-nesting
    /// limit at all.
    pub max_form_depth: usize,
    /// Capture per-glyph [`GlyphProvenance`] during the walk. Default
    /// `false`.
    ///
    /// When `false` (the Pass 4 default), [`ExtractedGlyph::provenance`] is
    /// `None` for every glyph and the extraction output is byte-for-byte
    /// what it has always been — no existing test or accessor changes. When
    /// `true`, every `Glyphs`-origin glyph records the show-operator span,
    /// governing font resource and `Tf` size, fill colour, and text/CTM
    /// matrices (§9.4). This is the read substrate the editable text model
    /// ([`crate::text_edit`]) and the later edit surgery (decision 014,
    /// Pass 14.1) build on; the flag exists so that cost is paid only by
    /// callers that need it.
    pub capture_provenance: bool,
}

impl Default for ExtractOptions {
    fn default() -> Self {
        Self {
            include_artifacts: false,
            word_gap_ratio: 0.20,
            line_gap_ratio: 0.30,
            backward_jump_ratio: 0.50,
            max_form_depth: 64,
            capture_provenance: false,
        }
    }
}

impl ExtractOptions {
    /// Set [`Self::include_artifacts`], consuming and returning `self`.
    ///
    /// A consuming setter rather than a plain field write because this
    /// struct is `#[non_exhaustive]` — future Passes will add knobs, and
    /// `#[non_exhaustive]` forbids a downstream crate from using a
    /// struct expression at all, including the `..Default::default()`
    /// form. Field *assignment* on a `let mut` binding still works, so
    /// both styles are available; this one keeps a one-liner a one-liner
    /// (C-BUILDER).
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfce_core::text_extract::ExtractOptions;
    ///
    /// let options = ExtractOptions::default().with_artifacts(true);
    /// assert!(options.include_artifacts);
    /// ```
    #[must_use]
    pub const fn with_artifacts(mut self, include: bool) -> Self {
        self.include_artifacts = include;
        self
    }

    /// Set the three derived-segmentation ratios at once, consuming and
    /// returning `self`.
    ///
    /// Grouped into one method deliberately: the three are a *tuning*,
    /// not three independent knobs — raising the word-gap ratio without
    /// considering the line-gap ratio produces a segmentation nobody
    /// reasoned about. See the struct docs for why any value here is
    /// arguable (S3/S4: no spec basis whatsoever).
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfce_core::text_extract::ExtractOptions;
    ///
    /// // Less eager to break words apart, more eager to break lines.
    /// let options = ExtractOptions::default().with_gap_ratios(0.35, 0.20, 0.50);
    /// assert!((options.word_gap_ratio - 0.35).abs() < 1e-6);
    /// ```
    #[must_use]
    pub const fn with_gap_ratios(mut self, word: f32, line: f32, backward: f32) -> Self {
        self.word_gap_ratio = word;
        self.line_gap_ratio = line;
        self.backward_jump_ratio = backward;
        self
    }

    /// Set [`Self::max_form_depth`], consuming and returning `self`.
    #[must_use]
    pub const fn with_max_form_depth(mut self, depth: usize) -> Self {
        self.max_form_depth = depth;
        self
    }

    /// Set [`Self::capture_provenance`], consuming and returning `self`.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfce_core::text_extract::ExtractOptions;
    ///
    /// let options = ExtractOptions::default().with_provenance(true);
    /// assert!(options.capture_provenance);
    /// ```
    #[must_use]
    pub const fn with_provenance(mut self, capture: bool) -> Self {
        self.capture_provenance = capture;
        self
    }
}

/// Why an extraction could not run at all.
///
/// Note how little is in here: a *page* that fails to tokenize is
/// counted in [`TextDiagnostics::pages_unreadable`] and skipped, not
/// propagated, because one broken page must not cost a caller the other
/// 399. These variants are for failures that make the question
/// unanswerable.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ExtractError {
    /// The page tree could not be walked.
    #[error("page tree: {0}")]
    PageTree(#[from] PageTreeError),
    /// A page index was past the end of the document.
    #[error("page index {index} is past the end ({count} pages)")]
    NoSuchPage {
        /// The zero-based index requested.
        index: usize,
        /// How many pages the document actually has.
        count: usize,
    },
    /// A content stream could not be tokenized and the caller asked for
    /// exactly that page.
    #[error("content stream: {0}")]
    Content(#[from] ContentError),
}

impl ExtractedText {
    /// The plain-text rendering: sourced characters **plus** pdfce's
    /// derived whitespace, which is what a "Copy page text" affordance
    /// puts on the clipboard.
    ///
    /// Artifact runs are included only if
    /// [`ExtractOptions::include_artifacts`] was set for the extraction;
    /// the flag is captured at extraction time so this accessor needs no
    /// arguments and cannot silently disagree with the run list.
    ///
    /// Pages are joined with a form feed (U+000C) — the conventional
    /// page separator in extracted text, and the right character rather
    /// than merely the traditional one: U+000C is Unicode line-break
    /// class **BK** (mandatory break), so a conforming text renderer
    /// starts a new line at it, while a caller that wants page
    /// boundaries can still split on it unambiguously. A newline would
    /// be indistinguishable from a derived line break; a blank line
    /// would be two more invented characters.
    ///
    /// Note that the separator is itself pdfce's choice — the file says
    /// nothing about how its pages concatenate — so a caller reading
    /// [`ExtractedText::pages`] directly, rather than this string, is
    /// working from the unjoined truth.
    #[must_use]
    pub fn plain_text(&self) -> String {
        let mut out = String::new();
        for (i, page) in self.pages.iter().enumerate() {
            if i > 0 {
                out.push('\u{000C}');
            }
            out.push_str(&page.plain_text());
        }
        out
    }

    /// **Only** the characters the file actually provides: every derived
    /// space and derived line break omitted.
    ///
    /// This is the honest lower bound on "what does this document say",
    /// and it is what a test asserting spec-example behaviour should
    /// compare against. §14.9.4's `Drucker` example is the canonical
    /// case: the shown glyphs are `Dru`, `k-`, `ker` with an
    /// `/ActualText` of `c`, and the *sourced* text is exactly
    /// `Drucker` — the line break between `c` and `ker` is pdfce's
    /// derived judgement, not the file's.
    #[must_use]
    pub fn sourced_text(&self) -> String {
        self.pages.iter().map(PageText::sourced_text).collect()
    }

    /// Whether the extraction included artifact runs in
    /// [`Self::plain_text`].
    #[must_use]
    pub const fn includes_artifacts(&self) -> bool {
        self.include_artifacts
    }
}

impl fmt::Display for ExtractedText {
    /// Same as [`ExtractedText::plain_text`].
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.plain_text())
    }
}

impl PageText {
    /// This page's plain text — see [`ExtractedText::plain_text`].
    #[must_use]
    pub fn plain_text(&self) -> String {
        self.runs
            .iter()
            .filter(|r| r.artifact.is_none() || self.include_artifacts)
            .map(|r| r.text.as_str())
            .collect()
    }

    /// This page's sourced characters only — see
    /// [`ExtractedText::sourced_text`].
    #[must_use]
    pub fn sourced_text(&self) -> String {
        self.runs
            .iter()
            .filter(|r| r.is_sourced() && (r.artifact.is_none() || self.include_artifacts))
            .map(|r| r.text.as_str())
            .collect()
    }
}

/// Extract text from one page.
///
/// `page_index` is recorded in the result and is otherwise unused, so a
/// caller extracting a subset can label the pages it got.
///
/// # Errors
///
/// [`ExtractError::Content`] when the page's content streams cannot be
/// tokenized. A page whose content is *missing* is not an error — it
/// extracts to zero runs.
///
/// # Examples
///
/// ```no_run
/// use pdfce_core::document::Document;
/// use pdfce_core::{page_tree, text_extract};
///
/// let doc = Document::load(std::path::Path::new("in.pdf"))?;
/// let pages = page_tree::pages(&doc)?;
/// let options = text_extract::ExtractOptions::default();
/// let page = text_extract::extract_page(&doc, &pages[0], 0, &options)?;
/// println!("{}", page.plain_text());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn extract_page(
    doc: &Document,
    page: &Page,
    page_index: usize,
    options: &ExtractOptions,
) -> Result<PageText, ExtractError> {
    extract_page_view(&doc.view(), page, page_index, options)
}

/// [`extract_page`] over an explicit [`DocumentView`] — i.e. over **a
/// revision the caller names**, rather than over a loaded file.
///
/// # Why this twin exists (Pass 17.1, decision 018 §8)
///
/// Text extraction feeds two very different consumers, and until this Pass
/// both got the same answer whether it was right for them or not:
///
/// - **"What does this FILE say?"** — the CLI's `extract-text`, the
///   redaction search census, `reflow_apply`'s planner. These want the file
///   as loaded, and keep calling the `&Document` form above, which is now a
///   one-line wrapper over this.
/// - **"What does the page IN FRONT OF ME say?"** — the GUI's in-place
///   text-edit model and Copy Text. Passing `session.document()` gave these
///   the base revision, so after one accepted edit the editing tool's own
///   model still described the text as it was BEFORE that edit: caret and
///   selection offsets computed against text the page no longer contains,
///   used to build the *next* `EditRequest`. That is not merely stale
///   display — it is a stale model feeding a mutation.
///
/// The split is deliberate rather than a blanket switch to the session: a
/// silent change to what a *search* matches mid-edit is its own surprise,
/// so the revision is now the caller's explicit choice, made once, at the
/// call site, with a comment saying which it wants and why.
///
/// # Errors
///
/// As [`extract_page`].
///
/// # Examples
///
/// ```no_run
/// use pdfce_core::document::Document;
/// use pdfce_core::edit::EditSession;
/// use pdfce_core::{page_tree, text_extract};
///
/// let doc = Document::load(std::path::Path::new("in.pdf"))?;
/// let session = EditSession::new(doc);
/// // The page as the operator currently has it, unsaved edits included.
/// let view = session.view();
/// let pages = page_tree::pages_in(&view)?;
/// let options = text_extract::ExtractOptions::default();
/// let page = text_extract::extract_page_view(&view, &pages[0], 0, &options)?;
/// println!("{}", page.plain_text());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn extract_page_view(
    doc: &DocumentView<'_>,
    page: &Page,
    page_index: usize,
    options: &ExtractOptions,
) -> Result<PageText, ExtractError> {
    let (items, mut diagnostics) = page::walk_page(doc, page, options)?;
    document_facts(doc, &mut diagnostics);
    let runs = layout::assemble(items, options, &mut diagnostics);
    Ok(PageText {
        page_index,
        runs,
        diagnostics,
        include_artifacts: options.include_artifacts,
    })
}

/// Extract text from every page of a document.
///
/// A page whose content stream cannot be tokenized is **counted**
/// ([`TextDiagnostics::pages_unreadable`]) and skipped with an empty
/// run list, never propagated as an error — one broken page in a 400
/// page document must not cost the caller the other 399.
///
/// # Errors
///
/// [`ExtractError::PageTree`] if the page tree itself cannot be walked.
pub fn extract_document(
    doc: &Document,
    options: &ExtractOptions,
) -> Result<ExtractedText, ExtractError> {
    extract_document_view(&doc.view(), options)
}

/// [`extract_document`] over an explicit [`DocumentView`].
///
/// See [`extract_page_view`] for why the revision is the caller's choice.
/// Note the page LIST comes from the same view, so a page this session
/// deleted is genuinely absent from the output and a page it inserted is
/// genuinely present — which is the difference between "the document" and
/// "the file", and the reason this cannot be emulated by looping the
/// `&Document` form over base page indices.
///
/// # Errors
///
/// As [`extract_document`].
pub fn extract_document_view(
    doc: &DocumentView<'_>,
    options: &ExtractOptions,
) -> Result<ExtractedText, ExtractError> {
    let pages = page_tree::pages_in(doc)?;
    let mut out = ExtractedText {
        pages: Vec::with_capacity(pages.len()),
        diagnostics: TextDiagnostics::default(),
        include_artifacts: options.include_artifacts,
    };
    for (index, page) in pages.iter().enumerate() {
        let page_text = match extract_page_view(doc, page, index, options) {
            Ok(t) => t,
            Err(_) => {
                let mut diagnostics = TextDiagnostics {
                    pages_unreadable: 1,
                    ..TextDiagnostics::default()
                };
                document_facts(doc, &mut diagnostics);
                diagnostics.note(format!(
                    "text: page {} content stream could not be read — no text extracted",
                    index + 1
                ));
                PageText {
                    page_index: index,
                    runs: Vec::new(),
                    diagnostics,
                    include_artifacts: options.include_artifacts,
                }
            }
        };
        out.diagnostics.merge(&page_text.diagnostics);
        out.pages.push(page_text);
    }
    Ok(out)
}

/// Extract text from a selected subset of pages, in the order given.
///
/// # Errors
///
/// [`ExtractError::NoSuchPage`] if any index is past the end;
/// [`ExtractError::PageTree`] if the page tree cannot be walked. Unlike
/// [`extract_document`], an unreadable *content stream* is still
/// counted-and-skipped rather than fatal.
pub fn extract_pages(
    doc: &Document,
    indices: &[usize],
    options: &ExtractOptions,
) -> Result<ExtractedText, ExtractError> {
    extract_pages_view(&doc.view(), indices, options)
}

/// [`extract_pages`] over an explicit [`DocumentView`].
///
/// See [`extract_page_view`] for why the revision is the caller's choice.
/// `indices` index the view's OWN page list, so under a session view they
/// mean "the operator's page 3", not "the file's page 3" — the same
/// convention `EditSession::pages` established.
///
/// # Errors
///
/// As [`extract_pages`].
pub fn extract_pages_view(
    doc: &DocumentView<'_>,
    indices: &[usize],
    options: &ExtractOptions,
) -> Result<ExtractedText, ExtractError> {
    let pages = page_tree::pages_in(doc)?;
    let mut out = ExtractedText {
        pages: Vec::with_capacity(indices.len()),
        diagnostics: TextDiagnostics::default(),
        include_artifacts: options.include_artifacts,
    };
    for &index in indices {
        let page = pages.get(index).ok_or(ExtractError::NoSuchPage {
            index,
            count: pages.len(),
        })?;
        let page_text = extract_page_view(doc, page, index, options).unwrap_or_else(|_| {
            let mut diagnostics = TextDiagnostics {
                pages_unreadable: 1,
                ..TextDiagnostics::default()
            };
            document_facts(doc, &mut diagnostics);
            PageText {
                page_index: index,
                runs: Vec::new(),
                diagnostics,
                include_artifacts: options.include_artifacts,
            }
        });
        out.diagnostics.merge(&page_text.diagnostics);
        out.pages.push(page_text);
    }
    Ok(out)
}

/// Read the three document-level facts that decide whether *anything*
/// about this extraction is sourced: `/MarkInfo` `/Marked`, `/Suspects`,
/// and the presence of `/StructTreeRoot`.
///
/// §14.8.1's four guarantees (every code mappable to Unicode, word
/// breaks explicit, artifacts distinguished, content in appearance
/// order) hold **only** for a Tagged PDF. In an untagged document every
/// one of them is pdfce's problem, which is exactly what the emitted
/// note says.
fn document_facts(doc: &DocumentView<'_>, diagnostics: &mut TextDiagnostics) {
    use crate::graph::ObjectGraph;
    use crate::object::Object;

    // `catalog_dict()` (the `ObjectGraph` provided method) rather than
    // `Document::catalog()` (Pass 17.1): the same trailer→`/Root` walk, but
    // available on any graph, so a session view answers with the SESSION's
    // catalog. That matters here — an edit can set `/MarkInfo` or attach a
    // `/StructTreeRoot`, and this function's whole job is to report whether
    // the extraction it accompanies was sourced or derived. `Option` rather
    // than `Result` costs nothing: both arms already mean "say nothing".
    let Some(catalog) = doc.catalog_dict() else {
        return;
    };
    diagnostics.struct_tree_present = catalog.get(b"StructTreeRoot").is_some();
    if let Some(mark_info) = catalog.get(b"MarkInfo").map(|o| doc.resolve(o))
        && let Some(mark_info) = mark_info.as_dict()
    {
        diagnostics.tagged = matches!(
            mark_info.get(b"Marked").map(|o| doc.resolve(o)),
            Some(Object::Boolean(true))
        );
        diagnostics.suspects = matches!(
            mark_info.get(b"Suspects").map(|o| doc.resolve(o)),
            Some(Object::Boolean(true))
        );
    }

    if !diagnostics.tagged {
        diagnostics.note(
            "text: untagged document — word spacing and reading order are derived, not sourced \
             (ISO 32000-1 §14.8.1)"
                .to_string(),
        );
    }
    if diagnostics.suspects {
        diagnostics.note(
            "text: /MarkInfo /Suspects true — the producer disclaims its own reading order \
             (ISO 32000-1 §14.8.2.3.1)"
                .to_string(),
        );
    }
    if diagnostics.struct_tree_present {
        diagnostics.note(
            "text: /StructTreeRoot present — structure-tree reading order is DEFERRED; runs are \
             in page content order (ISO 32000-1 §14.8.2.3.1)"
                .to_string(),
        );
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
    fn origin_sourcing_classification() {
        assert!(TextOrigin::Glyphs.is_sourced());
        assert!(TextOrigin::ActualText.is_sourced());
        assert!(!TextOrigin::DerivedWordSpace.is_sourced());
        assert!(!TextOrigin::DerivedLineBreak.is_sourced());
    }

    #[test]
    fn sourced_fraction_is_none_for_a_page_with_no_text() {
        let d = TextDiagnostics::default();
        assert_eq!(d.sourced_fraction(), None);
    }

    #[test]
    fn sourced_fraction_excludes_the_extension_and_the_failures() {
        let d = TextDiagnostics {
            codes_total: 100,
            via_to_unicode: 60,
            via_encoding_agl: 20,
            via_glyph_name_extension: 15,
            ladder_failures: 5,
            ..TextDiagnostics::default()
        };
        assert_eq!(d.sourced_codes(), 80);
        assert!((d.sourced_fraction().unwrap() - 0.80).abs() < 1e-9);
    }

    #[test]
    fn notes_deduplicate() {
        let mut d = TextDiagnostics::default();
        d.note("a".into());
        d.note("a".into());
        d.note("b".into());
        assert_eq!(d.notes, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn merge_folds_counters_and_or_s_the_flags() {
        let mut a = TextDiagnostics {
            codes_total: 3,
            tagged: false,
            ..TextDiagnostics::default()
        };
        let b = TextDiagnostics {
            codes_total: 4,
            tagged: true,
            spaces_derived: 2,
            ..TextDiagnostics::default()
        };
        a.merge(&b);
        assert_eq!(a.codes_total, 7);
        assert_eq!(a.spaces_derived, 2);
        assert!(a.tagged);
    }
}
