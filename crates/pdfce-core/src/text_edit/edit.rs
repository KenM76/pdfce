//! # In-place text-edit surgery — REPLACE a show operand (Pass 14.1)
//!
//! This module extends Pass 8.0's advance-preserving **REMOVE** surgery
//! (`crate::redact`) to **REPLACE**: it rewrites the operand of a `Tj`/`TJ`
//! show operator with new text, re-encodes that text in the run's own font
//! encoding (`crate::text_edit::encoding`), preserves the §9.4.4 advance so
//! un-edited text stays put, and saves the change **incrementally**
//! (decision 014 §3; R34/R47/R70/R72).
//!
//! ## What REPLACE inherits from REMOVE, and what is new
//!
//! REMOVE is the special case of REPLACE where the new run is **empty**
//! (`A_new = 0`). The advance bookkeeping is identical
//! (`iso32000__ref__text_edit_surgery.md` §0): map a run to bytes by
//! decoding character **codes**, never slicing raw bytes; each code's width
//! `w0` comes from the **same** `/Widths`/AFM the render path uses. What is
//! new:
//!
//! - The replacement run has its own advance `A_new`, so the delta
//!   `ΔA = A_new − A_old` has either sign.
//! - Editing's default posture is the **opposite** of redaction's: it
//!   **REFLOWS** — a longer word pushes the rest of the line right, a
//!   shorter word pulls it left — so the default is to let the line shift by
//!   `ΔA`, not to pin survivors (surgery ref §3). A `PIN` posture (the
//!   Pass-8.0 compensating-`TJ` path) stays available for a run whose tail
//!   is absolutely positioned and must not move.
//! - The new codes come from the inverse-encoding builder; a character the
//!   run's font cannot provide is **REFUSED by name**, never faked
//!   (`crate::text_edit::encoding`, rule 4 / R71).
//!
//! ## The advance-delta formula (§9.4.4)
//!
//! Horizontal writing mode. The advance applied to `Tm` after painting one
//! glyph is, in text-space units,
//! `tx = ((w0/1000 − Tj/1000)·Tfs + Tc + Tw)·Th`, where `Tw` contributes
//! **only** when the code is the single byte `0x20` (§9.3.3). A run's total
//! advance folds this over its codes; `ΔA = A_new − A_old` drives the edit.
//! `Tfs`, `Tc`, `Tw`, `Th` are unchanged by the edit (same text state), so
//! only the width sum and the `0x20`-count differ.
//!
//! ## Disposition of "the rest of the line" (surgery ref §3)
//!
//! "The rest of the line" is the run of subsequent operators up to the next
//! `Tm`/`Td`/`TD`/`T*`/`'`/`"` re-anchor. Under `REFLOW` (default):
//! advance-relative followers auto-shift by `ΔA` for free (nothing to do); a
//! follower re-anchored by an **absolute `Tm`** does NOT auto-shift (§9.4.2:
//! `Tm` REPLACES, not concatenates), so its `e` operand gets `ΔA` added; a
//! `Td`/`TD`/`T*` marks the line boundary and is left alone. The edited line
//! MAY overflow the original right margin — that is **DISCLOSED**, not
//! reflowed; block re-wrap is deferred (FF-A). Under `PIN`: a compensating
//! `TJ` number consumes `ΔA` so survivors do not move.
//!
//! ## Marked content / tagged PDFs (§14.6/§14.7, T-disclose, R72)
//!
//! The edit rewrites ONLY the show operator(s) (and, under reflow, one or
//! more following `Tm`s). The enclosing `BDC …/MCID n… EMC` wrapper and the
//! `/MCID` value are therefore preserved **by construction** — the structure
//! tree's `(Pg, MCID)` reference stays valid. What goes stale is the
//! `/ActualText`/reading-order the tree records; 14.1 **DISCLOSES** that
//! (a stale `/ActualText` would win on extraction, §14.9.4) and does not
//! regenerate it (FF-H).
//!
//! ## Save mode (R34/R36/R70)
//!
//! Editing is NOT redaction: it uses the **default incremental save**. Prior
//! text survives in the document's history by design, and this is DISCLOSED
//! — truly removing text is REDACTION (Pass 8, R35), a different operation.
//! Only the edited content stream object (+ any collapsed extra content
//! objects on a multi-stream page) is re-emitted; everything else is the
//! original file bytes verbatim (incremental append ⇒ the original is a
//! byte-prefix of the output, R32/R46).
//!
//! ## Scope of the first cut (decision 014 §5.2 "13.1")
//!
//! Simple (`Type1`/`TrueType`/`MMType1`) fonts only; `Tj`/`TJ` anchors only.
//! NO reflow/block re-wrap (line overflow is disclosed), NO family-change
//! formatting (Pass 14.2), NO font subsetting (FF-C), NO composite/CJK/RTL
//! editing (R-INV-4), NO add-new-text (FF-D). The `'`/`"` show operators and
//! form-XObject content are named non-goals of this cut.

use std::collections::{BTreeSet, HashMap};

use crate::content::{ContentError, ContentStream, ContentTokenKind, Operation};
use crate::document::Document;
use crate::object::{Dict, Name, Object, Stream};
use crate::page_tree::{self, Page, PageTreeError};
use crate::span::ByteSpan;
use crate::text_edit::encoding::{InverseEncoding, RInvTrigger, Refusal};
use crate::text_extract::font::ExtractFont;
use crate::text_state::{AmbientTextState, TextStateParam};
use crate::writer::content::{emit_literal_string, emit_number};
use crate::writer::{DirtySet, SaveOptions, WriteError, save_incremental};

// ===================================================================
// Fill-colour graphics state (§8.6.8) — recorded by the walk for Pass
// 14.2's formatting surgery (`crate::text_edit::format`)
// ===================================================================
//
// Pass 14.1 (this module) never reads a run's fill colour: a REPLACE
// rewrites only the show operator's *codes*, so the colour operator that
// precedes the run is left byte-verbatim and no restore is needed. Pass
// 14.2 DOES need it — a localized size/colour/font change re-emits the
// show operator wrapped in state-set/state-restore operators, and the
// restore must reinstate the exact prior fill colour so every following
// operator is byte-for-byte unaffected. The walk therefore records the
// current fill colour on every show operator; because 14.1's `edit_text`
// ignores the new field, its output bytes are unchanged (verified by the
// unaltered 14.1 fixtures/tests).

/// The three *device* fill-colour operators (§8.6.4.2/.3/.4). Each names
/// its own colour space inline, so a device colour is fully modelled by
/// its operator and components — the space pdfce can both classify and,
/// for 14.2, restore by re-emitting the recorded operator bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeviceSpace {
    /// `g` — DeviceGray, one component.
    Gray,
    /// `rg` — DeviceRGB, three components.
    Rgb,
    /// `k` — DeviceCMYK, four components.
    Cmyk,
}

/// The fill colour in effect at a show operator, recorded so Pass 14.2 can
/// **restore** it byte-faithfully after a wrapped edit.
///
/// - [`Self::Default`] — no fill-colour operator has run; the §8.6.8
///   default (black `DeviceGray 0`) is in effect. Restored by emitting
///   `0 g`.
/// - [`Self::Device`] — a `g`/`rg`/`k` operator set it; both the classified
///   space+components (for the narrowing decision) and the operator's raw
///   bytes (for a byte-identical restore) are kept.
/// - [`Self::Other`] — a colour set through `sc`/`scn` in a resource-named
///   space (ICCBased, Separation/spot, DeviceN, Indexed, …): present but
///   NOT decoded (the [`crate::text_extract::TextColor::Other`] analog).
///   The raw operator byte sequence (the `cs` that set the space plus the
///   `sc`/`scn` that set the value) is kept so a tail can be restored
///   verbatim even though pdfce cannot interpret the colour.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FillState {
    /// §8.6.8 default: black DeviceGray 0. No operator set the fill colour.
    Default,
    /// A device fill colour, with its classified space, components, and the
    /// raw operator bytes for a faithful restore.
    Device {
        /// Which device operator set it.
        space: DeviceSpace,
        /// The colour components as written (0.0..=1.0 by §8.6.4).
        comps: Vec<f64>,
        /// The raw bytes of the setting operator, e.g. `1 0 0 rg`.
        raw: Vec<u8>,
    },
    /// A non-device fill colour pdfce does not decode; the raw operator
    /// byte sequence to re-emit for a verbatim restore.
    Other {
        /// The `cs`(space) + `sc`/`scn`(value) operator bytes, space-joined.
        raw: Vec<u8>,
    },
}

impl FillState {
    /// Whether this is a non-device (`Other`) fill colour — the space a
    /// 14.2 colour edit cannot preserve and must DISCLOSE narrowing for
    /// (rule 4).
    pub(crate) const fn is_other(&self) -> bool {
        matches!(self, Self::Other { .. })
    }

    /// The operator bytes that reinstate this fill colour, for the
    /// state-restore half of a 14.2 wrapped edit. `Default` restores with
    /// `0 g` (the §8.6.8 default made explicit); `Device`/`Other` re-emit
    /// their recorded raw bytes verbatim (minimal-diff restore).
    pub(crate) fn restore_bytes(&self) -> Vec<u8> {
        match self {
            Self::Default => b"0 g".to_vec(),
            Self::Device { raw, .. } | Self::Other { raw } => raw.clone(),
        }
    }

    /// The operator bytes that reinstate this colour as the **stroking**
    /// colour (Pass 19.2).
    ///
    /// The only difference from [`Self::restore_bytes`] is the `Default`
    /// arm, and it is a difference that matters: §8.6.8 gives the stroking
    /// and non-stroking colours *separate* graphics-state entries with the
    /// same initial value (black `DeviceGray 0`), and the operators that
    /// set them are spelled in different cases — `G`/`RG`/`K`/`SC`/`SCN`
    /// stroking, `g`/`rg`/`k`/`sc`/`scn` non-stroking. Restoring an unset
    /// stroking colour with `0 g` would put the *fill* colour back to black
    /// while leaving the stroking colour wherever synthetic bold left it —
    /// a silent corruption of two parameters at once.
    ///
    /// The `Device`/`Other` arms re-emit their own recorded bytes, which
    /// were already captured from an uppercase operator by the walk, so no
    /// case conversion is performed (or possible — an `Other` restore is an
    /// opaque `CS … SCN` sequence).
    pub(crate) fn restore_bytes_stroking(&self) -> Vec<u8> {
        match self {
            Self::Default => b"0 G".to_vec(),
            Self::Device { raw, .. } | Self::Other { raw } => raw.clone(),
        }
    }
}

/// The line width (§8.4.3.2 `w`) in effect at a show operator, recorded so
/// Pass 19.2's synthetic bold can restore it (Pass 19.2).
///
/// ## Why this is tracked at all, and why it is not a `f64`
///
/// Synthetic bold emits text rendering mode 2 (fill-then-stroke) plus a
/// line width, and §9.3.6 interprets that width **in user space** — it is
/// the ordinary graphics-state line width, the same one a later `S` on a
/// *path* would use. So a synthetic-bold run that does not put the width
/// back does not merely leave stale text state: it changes the weight of
/// every subsequent stroked path in the content stream. That is a
/// minimal-diff violation in content pdfce never claimed to touch.
///
/// It is an enum rather than a bare number for the same reason
/// [`crate::text_state::AmbientOrigin`] is: the restore must know whether
/// the value is *provably* Table 52's initial (in which case `1 w` is
/// correct and byte-faithful in spirit) or was set by an operator whose
/// exact spelling should come back unchanged.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LineWidth {
    /// No `w` operator has run; Table 52's initial line width of **1.0** is
    /// in force.
    Initial,
    /// A `w` operator set it; its raw bytes are kept for a byte-faithful
    /// restore (`0.5000 w` comes back as `0.5000 w`, not `0.5 w`).
    Observed {
        /// The width operand as parsed — used to decide whether an emitted
        /// width is a no-op, never to spell the restore.
        value: f64,
        /// The whole operator as written, e.g. `0.5000 w`.
        raw: Vec<u8>,
    },
}

impl LineWidth {
    /// The width in force, for the no-op comparison.
    pub(crate) const fn value(&self) -> f64 {
        match self {
            // §8.4.3.2 / Table 52: the initial line width is 1.0.
            Self::Initial => 1.0,
            Self::Observed { value, .. } => *value,
        }
    }

    /// The operator bytes that reinstate this line width.
    pub(crate) fn restore_bytes(&self) -> Vec<u8> {
        match self {
            Self::Initial => b"1 w".to_vec(),
            Self::Observed { raw, .. } => raw.clone(),
        }
    }
}

/// Multiply two PDF 3×2 matrices, `m` applied **first** (§8.3.3).
///
/// A PDF matrix `[a b c d e f]` denotes
///
/// ```text
/// | a  b  0 |
/// | c  d  0 |
/// | e  f  1 |
/// ```
///
/// and a point is a **row** vector multiplied on the left, so composing
/// "apply `m`, then apply `n`" is the product `m × n` in that order. Getting
/// the order backwards produces a transform that is right for every
/// symmetric case (pure scale, pure translation applied to the origin) and
/// wrong for exactly the asymmetric ones this Pass introduces — a shear, and
/// a translation under a shear. That is why this is a named function with a
/// test rather than six lines inlined at the two call sites.
pub(crate) fn mat_mul(m: [f64; 6], n: [f64; 6]) -> [f64; 6] {
    [
        m[0] * n[0] + m[1] * n[2],
        m[0] * n[1] + m[1] * n[3],
        m[2] * n[0] + m[3] * n[2],
        m[2] * n[1] + m[3] * n[3],
        m[4] * n[0] + m[5] * n[2] + n[4],
        m[4] * n[1] + m[5] * n[3] + n[5],
    ]
}

/// The identity matrix — `BT`'s reset value for both `Tm` and `Tlm`
/// (§9.4.1 Table 107).
pub(crate) const IDENTITY: [f64; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// How the rest of the edited line is treated after a REPLACE (surgery ref
/// §3). The default is [`Self::Reflow`] — in-place editing intends the line
/// to grow/shrink.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum FollowerDisposition {
    /// Let the line shift by `ΔA`: advance-relative followers move for free;
    /// an absolute-`Tm` follower gets `ΔA` added to its `e` (the default).
    #[default]
    Reflow,
    /// Pin survivors in place with a compensating `TJ` number (the Pass-8.0
    /// path), for a justified / right-aligned tail that must not move.
    Pin,
}

/// One in-place text-edit request against a page.
///
/// The anchor operator is located by finding [`Self::find`] in a single
/// show operator's decoded text — either the first such operator, or (when
/// [`Self::pinned_span`] is set from a Pass-14.0
/// [`GlyphProvenance::operator_span`](crate::text_extract::GlyphProvenance))
/// exactly the operator that provenance points at. That `operator_span` is
/// how Pass 14.0's model LOCATES the run to rewrite; this surgery re-tokenizes
/// the same content buffer and matches the same span.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct EditRequest {
    /// 0-based page index.
    pub page_index: usize,
    /// The text to locate within one show operator's decoded run.
    pub find: String,
    /// The replacement text (re-encoded into the run's font).
    pub replace: String,
    /// When set, only consider the show operator that this byte span in the
    /// decoded content buffer NAMES — the provenance-pinned path.
    ///
    /// "Names", not "equals": two spans identify the same operator here, and
    /// both are accepted (see [`pin_names_operator`]). A span covering the
    /// operator token alone (`Tj`) is what
    /// [`GlyphProvenance::operator_span`](crate::text_extract::GlyphProvenance::operator_span)
    /// publishes; a span covering the operands too (`(hello) Tj`) is what this
    /// module's own walk records. Requiring exact equality against the second
    /// form silently broke every provenance-pinned request — fixed in Pass
    /// 19.3, with the history in `pin_names_operator`'s documentation.
    pub pinned_span: Option<ByteSpan>,
}

impl EditRequest {
    /// A find/replace request on `page_index` (no span pin).
    #[must_use]
    pub fn find_replace(page_index: usize, find: &str, replace: &str) -> Self {
        Self {
            page_index,
            find: find.to_owned(),
            replace: replace.to_owned(),
            pinned_span: None,
        }
    }
}

/// Per-edit options.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct EditOptions {
    /// How the rest of the line is disposed (default [`FollowerDisposition::Reflow`]).
    pub disposition: FollowerDisposition,
}

impl EditOptions {
    /// Set the follower [`FollowerDisposition`], returning `self` — the
    /// out-of-crate constructor, since [`EditOptions`] is `#[non_exhaustive]`
    /// (a struct literal is not usable from `pdfce-cli`).
    #[must_use]
    pub fn with_disposition(mut self, disposition: FollowerDisposition) -> Self {
        self.disposition = disposition;
        self
    }
}

/// Which trust level the edited run's glyphs come from, as far as
/// `pdfce-core` can determine WITHOUT a font rasterizer (R21). The shell
/// refines [`Self::NonEmbedded`] into decision-012 `Bundled` vs `Supplied`
/// by consulting its own `FontEnvironment`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EditGlyphSource {
    /// The run's font carries an embedded program (`/FontFile`/`2`/`3`).
    Embedded,
    /// The run's font is non-embedded — a bundled Base-14 or an
    /// operator-supplied face renders it (decision 012).
    NonEmbedded,
}

/// The outcome of a successful edit.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct EditOutcome {
    /// The saved (incrementally-appended) PDF bytes.
    pub bytes: Vec<u8>,
    /// The disclosure/diagnostic report.
    pub report: EditReport,
}

/// What the edit did and what it disclosed (fuzzy-never-sneaky, rule 4).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct EditReport {
    /// The `/BaseFont` of the edited run (subset tag included).
    pub base_font: String,
    /// The core-visible glyph source (see [`EditGlyphSource`]).
    pub glyph_source: EditGlyphSource,
    /// Whether the run's font is an embedded **subset** (a 6-letter `+`
    /// tag) — the one refusal case in decision 014's four-case table.
    pub subset: bool,
    /// `ΔA = A_new − A_old` in text-space units.
    pub advance_delta: f64,
    /// The follower disposition actually used.
    pub disposition: FollowerDisposition,
    /// How many following absolute `Tm`s were repositioned by `ΔA` (reflow).
    pub followers_repositioned: u64,
    /// The `/MCID` of the enclosing marked-content sequence, if the edit was
    /// inside a Tagged-PDF sequence (its wrapper is preserved; §14.7).
    pub tagged_mcid: Option<i64>,
    /// The content-stream object number that was rewritten.
    pub content_object: u32,
    /// Extra content objects collapsed/emptied on a multi-stream page.
    pub extra_objects_emptied: u64,
    /// Every operator-facing disclosure, verbatim (surfaced by the UI/CLI).
    pub disclosures: Vec<String>,
}

/// A failure to edit — every variant is a clean, named outcome, never a
/// crash (rule 4). A [`Self::Refused`] is the inverse-encoding gate saying
/// no by name.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EditError {
    /// The inverse-encoding / font-on-edit gate refused, by name.
    #[error(transparent)]
    Refused(Refusal),
    /// No page at the requested index.
    #[error("no page at index {0}")]
    PageIndex(usize),
    /// The find text was not present in any editable run.
    #[error("text to edit ({0:?}) was not found in an editable run on the page")]
    NoMatch(String),
    /// The run is real but this cut cannot edit it (composite font, a
    /// `'`/`"` anchor, a cross-element `TJ` match, …).
    #[error("this run cannot be edited in the first cut: {0}")]
    Unsupported(String),
    /// The document is encrypted (out of scope for text editing).
    #[error("the document is encrypted; in-place text editing of encrypted files is out of scope")]
    Encrypted,
    /// The page's content stream could not be parsed.
    #[error("content stream parse failed: {0}")]
    Content(#[from] ContentError),
    /// The page tree could not be walked.
    #[error("page tree error: {0}")]
    PageTree(#[from] PageTreeError),
    /// The incremental save failed.
    #[error("save failed: {0}")]
    Write(#[from] WriteError),
}

// ===================================================================
// The walk — one pass over the page content, recording every operator
// ===================================================================

/// One element of a decoded show operator's operand list.
///
/// `pub(crate)` because Pass 14.2's formatting surgery
/// (`crate::text_edit::format`) reconstructs an anchor operator's element
/// list — splitting it at the match into pre/mid/post segments — and
/// re-emits each segment with [`emit_show`].
#[derive(Debug, Clone)]
pub(crate) enum ShowElem {
    /// A show string (its raw code bytes).
    Str(Vec<u8>),
    /// A `TJ` kerning number (thousandths of text space, §9.4.3).
    Num(f64),
}

/// Which show operator an anchor is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShowOp {
    Tj,
    TJ,
    Quote,
    DoubleQuote,
}

/// One code decoded off a show operator, with everything needed to map a
/// text match back to the bytes to splice.
#[derive(Debug, Clone)]
pub(crate) struct ShowSlot {
    /// The character code this slot shows.
    ///
    /// `u32`, not `u8`, since Pass 21.1. A simple font's code IS one byte
    /// and always will be (§9.4.3), so this is wider than that case needs —
    /// but a composite font addresses glyphs by a multi-byte code, and the
    /// `u8` here is the specific thing that made composite runs
    /// unrepresentable rather than merely unimplemented. Widening it is
    /// behaviour-preserving on its own; it is the substrate the rest of
    /// 21.1 needs, landed separately so that the step which DOES change
    /// behaviour is not tangled with a mechanical type change.
    ///
    /// Pair it with [`Self::width`]: a code's value does not tell you how
    /// many bytes it occupied, and every byte-range calculation needs that.
    pub(crate) code: u32,
    /// How many bytes this code occupied in the operand string.
    ///
    /// 1 for a simple font, 2 for `Identity-H`. Carried per slot rather
    /// than per run because it is what the byte-range arithmetic in
    /// [`match_run`] consumes, and deriving it there from the font would
    /// mean re-answering a question the decode already answered — the kind
    /// of duplicated predicate that drifts (R92).
    pub(crate) width: u8,
    /// Index into the operator's [`ShowElem`] list.
    pub(crate) elem: usize,
    /// Byte offset of this code within that element's string.
    pub(crate) byte_in_elem: usize,
    /// Byte range of this code's characters within the decoded `text`.
    pub(crate) t0: usize,
    pub(crate) t1: usize,
}

/// A recorded show operator with its full text state.
///
/// `pub(crate)` so the Pass 14.2 formatting surgery can read the same
/// recorded text-state (font resource, size, spacing, MCID) the Pass 14.1
/// REPLACE surgery reads, plus the fill colour 14.2 additionally needs.
#[derive(Debug, Clone)]
pub(crate) struct ShowData {
    pub(crate) font_name: Vec<u8>,
    pub(crate) tf_size: f64,
    /// The ambient §9.3 text state at this operator, **with each
    /// parameter's restore provenance** (Pass 19.0).
    ///
    /// This replaced three bare `f64`s (`tc`/`tw`/`th`). The values are
    /// still reachable through [`Self::tc`]/[`Self::tw`]/[`Self::th`] for
    /// the §9.4.4 advance arithmetic, but the struct now additionally
    /// knows how to *put each one back* — which is what a formatting
    /// surgery that emits `Tc`/`Tz`/`Ts` for one run needs, and what R88's
    /// three-tier ladder is expressed in. It also covers `Ts` and `Tr`,
    /// which this walk did not track at all before.
    pub(crate) text_state: AmbientTextState,
    pub(crate) mcid: Option<i64>,
    pub(crate) op: ShowOp,
    pub(crate) elems: Vec<ShowElem>,
    pub(crate) text: String,
    pub(crate) slots: Vec<ShowSlot>,
    /// The fill colour in effect (§8.6.8) — recorded for Pass 14.2's
    /// colour-restore; unused by Pass 14.1's REPLACE.
    pub(crate) fill_color: FillState,
    /// The **stroking** colour in effect (§8.6.8) — recorded for Pass
    /// 19.2's synthetic bold, which paints in text rendering mode 2 and
    /// must therefore both *set* the stroking colour (to match the fill,
    /// §9.3.6) and put the previous one back.
    pub(crate) stroke_color: FillState,
    /// The line width in effect (§8.4.3.2) — recorded for the same reason:
    /// a stroked-text width is the ordinary user-space line width, shared
    /// with path stroking, so it must be restored.
    pub(crate) line_width: LineWidth,
    /// The text matrix `Tm` in force at the **start** of this show operator
    /// (§9.4.2), i.e. before any of its own glyphs have advanced it.
    ///
    /// Pass 19.2 needs this because synthetic italic is a **shear
    /// premultiplied into `Tm`**, and a shear can only be premultiplied
    /// into a matrix that is known. Nothing before 19.2 read the text
    /// matrix in the authoring path at all: 14.1's relayout works in
    /// *deltas* (add ΔA to a follower's `e`), which never requires knowing
    /// the absolute matrix.
    pub(crate) text_matrix: [f64; 6],
    /// Whether [`Self::text_matrix`] is trustworthy.
    ///
    /// The walk advances `Tm` across each show operator by the §9.4.4
    /// displacement of its glyphs, which requires resolving the font and
    /// its widths. When that is not possible — an unresolvable font
    /// resource, or a composite run this walk does not decode — the
    /// accumulated matrix silently stops tracking reality. Rather than
    /// publish a plausible-looking wrong matrix, the walk marks it
    /// **unknown** and any consumer that needs an absolute position
    /// refuses (rule 4: fuzzy, never sneaky). A `Tm`/`Td`/`TD`/`T*`
    /// operator re-establishes the matrix absolutely and clears the flag.
    pub(crate) matrix_known: bool,
}

impl ShowData {
    /// `Tc` — character spacing in effect at this operator (§9.3.2).
    pub(crate) fn tc(&self) -> f64 {
        self.text_state.char_spacing.value
    }

    /// `Tw` — word spacing in effect at this operator (§9.3.3).
    pub(crate) fn tw(&self) -> f64 {
        self.text_state.word_spacing.value
    }

    /// `Th` — horizontal scaling as a **ratio** (`Tz` ÷ 100, §9.3.4), the
    /// form §9.4.4's displacement formula multiplies by.
    pub(crate) fn th(&self) -> f64 {
        self.text_state.h_scale.value / 100.0
    }
}

/// What one operator contributes to the relayout scan.
#[derive(Debug, Clone)]
pub(crate) enum Rec {
    Show(Box<ShowData>),
    /// An absolute `Tm` with its six operands.
    Tm([f64; 6]),
    /// A `Td`/`TD`/`T*`/`'`/`"` — a line boundary reflow does not cross.
    Boundary,
    /// `ET` — the end of a text object (§9.4.1).
    ///
    /// Recorded from Pass 19.2 onward because synthetic italic must know
    /// **where the current text object stops**. The shear is emitted as an
    /// injected `Tm`, and any `Tm` overwrites `Tlm` as well (§9.4.2 Table
    /// 108), so a later `Td`/`TD`/`T*` *in the same text object* would
    /// derive its line from pdfce's injected matrix instead of the
    /// producer's. Past `ET` the question is moot: the next `BT` resets
    /// both matrices to the identity (Table 107), so nothing carries over.
    EndText,
    /// Anything else.
    Ignore,
}

/// One recorded operator: its byte span in the decoded buffer + its role.
pub(crate) struct OpRec {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) rec: Rec,
}

/// The text-state machine of the walk (a focused sibling of
/// `redact::Surgeon`, reusing the same `content` tokenizer and §9.4.4
/// advance model rather than a second interpreter).
pub(crate) struct Walk<'a> {
    doc: &'a Document,
    resources: &'a Dict,
    font_cache: HashMap<Vec<u8>, Option<ExtractFont>>,
    /// The graphics-state subset this walk models. One struct rather than
    /// loose fields because `q`/`Q` save and restore **all** of it at once
    /// (§8.4.2), and a stack of loose fields is how a `Q` comes to restore
    /// four of five things.
    gs: GState,
    /// The `q` save stack (§8.4.4). Bounded like the extraction walk's, so
    /// a hostile stream of `q`s cannot grow it without limit.
    gs_stack: Vec<GState>,
    // marked-content stack (§14.6/§14.7) — the current /MCID is the top.
    mc_stack: Vec<Option<i64>>,
    /// The text matrix `Tm` (§9.4.2), maintained across the whole walk.
    ///
    /// Unlike the six §9.3 parameters this is **not** graphics state — it
    /// is not saved by `q` or restored by `Q` (Table 52 lists no text
    /// matrix), and it exists only between `BT` and `ET`. So it lives on
    /// the walk rather than inside [`GState`], and that distinction is
    /// load-bearing: putting it in `GState` would have made a `Q` restore a
    /// text matrix, which no conforming reader does.
    tm: [f64; 6],
    /// The line matrix `Tlm` (§9.4.2) — what `Td`/`TD`/`T*` translate from.
    tlm: [f64; 6],
    /// Whether [`Self::tm`] still reflects reality; see
    /// [`ShowData::matrix_known`].
    tm_known: bool,
    pub(crate) recs: Vec<OpRec>,
}

/// The graphics state this authoring walk tracks, as `q`/`Q` see it.
///
/// # Pass 19.0: why this became a struct with a stack behind it
///
/// The walk previously kept `font_name`/`tf_size`/`tc`/`tw`/`th`/`fill`/
/// `last_cs` as loose fields and had **no `q` or `Q` arm at all**. Every
/// one of those is graphics state (§8.4.2, Table 52 + §9.3), so a stream
/// that wrote
///
/// ```text
/// q  1 0 0 rg  0.5 Tc  BT (a) Tj ET  Q   BT (b) Tj ET
/// ```
///
/// left the model believing `(b)` was red with `0.5 Tc`, when every
/// conforming reader discards both at the `Q`. That mis-modelled ambient
/// is exactly what a formatting restore would have re-emitted — writing a
/// wrong `0.5 Tc` into a stream that did not have one. The fix is
/// structural: one struct, one stack, one place a `Q` can go wrong.
#[derive(Debug, Clone)]
struct GState {
    /// The `/Fn` resource name selected by the most recent `Tf` (§9.3.1).
    font_name: Vec<u8>,
    /// `Tfs` — the `Tf` size operand (§9.3.1).
    tf_size: f64,
    /// The six shared §9.3 parameters, with restore provenance.
    ambient: AmbientTextState,
    /// The fill colour (§8.6.8) — recorded for Pass 14.2's restore.
    fill: FillState,
    /// The raw bytes of the most recent `cs` (set-fill-colour-space)
    /// operator, so a following `sc`/`scn` can record a re-emittable
    /// `cs … scn` sequence for the `Other` case.
    last_cs: Option<Vec<u8>>,
    /// The **stroking** colour (§8.6.8) — Pass 19.2. A separate
    /// graphics-state entry from `fill`, set by the uppercase operators.
    stroke: FillState,
    /// The stroking analogue of `last_cs`: the most recent `CS`
    /// (set-stroking-colour-space) operator's bytes.
    last_cs_stroking: Option<Vec<u8>>,
    /// The line width (§8.4.3.2) — Pass 19.2, for the synthetic-bold
    /// stroke width restore.
    line_width: LineWidth,
}

impl GState {
    /// The state at the start of a content stream: no font selected, and
    /// every §9.3 parameter at its Table 105 initial value.
    fn initial() -> Self {
        Self {
            font_name: Vec::new(),
            tf_size: 0.0,
            ambient: AmbientTextState::initial(),
            fill: FillState::Default,
            last_cs: None,
            stroke: FillState::Default,
            last_cs_stroking: None,
            line_width: LineWidth::Initial,
        }
    }
}

impl<'a> Walk<'a> {
    pub(crate) fn new(doc: &'a Document, resources: &'a Dict) -> Self {
        Self {
            doc,
            resources,
            font_cache: HashMap::new(),
            gs: GState::initial(),
            gs_stack: Vec::new(),
            mc_stack: Vec::new(),
            tm: IDENTITY,
            tlm: IDENTITY,
            tm_known: true,
            recs: Vec::new(),
        }
    }

    /// Resolve a `/Font /<name>` resource to an [`ExtractFont`] (cached).
    fn font(&mut self, name: &[u8]) -> Option<ExtractFont> {
        if let Some(hit) = self.font_cache.get(name) {
            return hit.clone();
        }
        let resolved = resolve_font(self.doc, self.resources, name);
        self.font_cache.insert(name.to_vec(), resolved.clone());
        resolved
    }

    fn nums(op: &Operation<'_>) -> Vec<f64> {
        op.operands
            .iter()
            .filter_map(|t| match &t.kind {
                ContentTokenKind::Operand(o) => o.as_number(),
                _ => None,
            })
            .collect()
    }

    fn current_mcid(&self) -> Option<i64> {
        self.mc_stack.iter().rev().find_map(|m| *m)
    }

    /// Apply §9.4.2 Table 108's next-line rule: `Tlm_new = translate(tx, ty)
    /// × Tlm_old`, and `Tm = Tlm_new`.
    ///
    /// Note that the translation composes with the **line** matrix, not with
    /// the current text matrix — which is the whole reason a `Td` after a
    /// long run returns to the left margin instead of continuing from where
    /// the glyphs stopped. It is also why an injected `Tm` is dangerous:
    /// `Tm` overwrites `Tlm` too, so the *next* `Td` would translate from
    /// pdfce's matrix rather than the producer's line origin.
    ///
    /// The matrix becomes known again here for the same reason it does at a
    /// `Tm`: the new value is derived from `Tlm`, which is only ever set
    /// absolutely (by `BT`, by `Tm`, or by a previous next-line), and never
    /// drifts with glyph advances.
    fn next_line(&mut self, tx: f64, ty: f64) {
        self.tlm = mat_mul([1.0, 0.0, 0.0, 1.0, tx, ty], self.tlm);
        self.tm = self.tlm;
        self.tm_known = true;
    }

    /// Advance `Tm` by one show operator's total horizontal displacement
    /// (§9.4.4), in the **unrotated text space** the displacement is defined
    /// in: `Tm_new = translate(tx, 0) × Tm_old`.
    ///
    /// `tx` is the sum over the operator's elements of
    /// `((w0 − Tj/1000)·Tfs + Tc + Tw)·Th` for each shown glyph, and
    /// `(−Tj/1000)·Tfs·Th` for each standalone `TJ` number ("since no glyph
    /// was painted", §9.4.3's implementation note — the `Tc`/`Tw` terms do
    /// **not** apply to a bare adjustment, and adding them is the classic
    /// way to make justified text drift).
    ///
    /// If the font could not be resolved, or the run is composite (this walk
    /// decodes only simple fonts), the displacement is unknowable here and
    /// the matrix is marked **unknown** rather than left silently stale.
    fn advance_matrix(&mut self, font: Option<&ExtractFont>, elems: &[ShowElem]) {
        let Some(font) = font.filter(|f| f.is_simple()) else {
            self.tm_known = false;
            return;
        };
        let p = self.gs.ambient.params();
        let mut tx = 0.0;
        for e in elems {
            match e {
                ShowElem::Str(bytes) => {
                    for &code in bytes {
                        tx += glyph_advance_with(
                            font,
                            code,
                            self.gs.tf_size,
                            p.char_spacing,
                            p.word_spacing,
                            p.h_scale,
                        );
                    }
                }
                ShowElem::Num(v) => {
                    tx += (-v / 1000.0) * self.gs.tf_size * p.h_scale;
                }
            }
        }
        self.tm = mat_mul([1.0, 0.0, 0.0, 1.0, tx, 0.0], self.tm);
    }

    /// Build a [`FillState::Device`] from a device fill operator's numeric
    /// operands and its raw byte span. The raw bytes (`buf[start..end]`) are
    /// kept for a byte-faithful restore (Pass 14.2); the components feed the
    /// narrowing decision only.
    fn device_fill(
        space: DeviceSpace,
        n: &[f64],
        buf: &[u8],
        start: usize,
        end: usize,
    ) -> FillState {
        FillState::Device {
            space,
            comps: n.to_vec(),
            raw: buf.get(start..end).map(<[u8]>::to_vec).unwrap_or_default(),
        }
    }

    /// Process one operation, updating text state and recording it.
    pub(crate) fn operation(&mut self, op: &Operation<'_>, buf: &[u8]) {
        let (start, end) = op_span(op);
        let Some(name) = op.operator_name(buf) else {
            self.recs.push(OpRec {
                start,
                end,
                rec: Rec::Ignore,
            });
            return;
        };
        let n = Self::nums(op);
        let rec = match name {
            // --- special graphics state (§8.4.4) ---
            //
            // Pass 19.0. These arms did not exist, which meant text state
            // and fill colour set inside a `q … Q` bracket leaked past the
            // `Q` in the model. See [`GState`]'s doc comment for the
            // worked example and why it matters to a restore.
            b"q" => {
                self.gs_stack.push(self.gs.clone());
                // A hostile stream of `q`s must not grow the stack without
                // bound; 256 is far past any real nesting and matches the
                // extraction walk's guard.
                if self.gs_stack.len() > 256 {
                    self.gs_stack.remove(0);
                }
                Rec::Ignore
            }
            b"Q" => {
                if let Some(prev) = self.gs_stack.pop() {
                    self.gs = prev;
                }
                Rec::Ignore
            }
            b"Tf" => {
                if let Some(fname) = op.operands.iter().find_map(|t| match &t.kind {
                    ContentTokenKind::Operand(Object::Name(nm)) => Some(nm.as_bytes().to_vec()),
                    _ => None,
                }) {
                    self.gs.font_name = fname;
                }
                if let Some(size) = n.last() {
                    self.gs.tf_size = *size;
                }
                Rec::Ignore
            }
            // --- text state (§9.3 Table 105) ---
            //
            // Pass 19.0: six operators, ONE update rule, shared with the
            // extraction and vector walks. `Ts` and `Tr` are new here —
            // this walk tracked neither, which is why pdfce could not
            // restore an ambient rise or rendering mode it had never
            // observed (decision 019 §1.2). The raw operator bytes are
            // captured for the R88 tier-2 restore.
            b"Tc" | b"Tw" | b"Tz" | b"TL" | b"Ts" | b"Tr" => {
                let raw = buf.get(start..end).unwrap_or_default();
                self.gs.ambient.apply_operator(name, &n, raw);
                Rec::Ignore
            }
            // --- text object delimiters (§9.4.1 Table 107) ---
            //
            // Pass 19.2. `BT` "shall initialize the text matrix Tm and the
            // text line matrix Tlm to the identity matrix" — and NOTHING
            // else: it does not reset text state (§9.3's retention rule),
            // which is why the ambient ladder exists at all.
            b"BT" => {
                self.tm = IDENTITY;
                self.tlm = IDENTITY;
                self.tm_known = true;
                Rec::Ignore
            }
            b"ET" => Rec::EndText,
            b"Tm" => match n.as_slice() {
                [a, b, c, d, e, f] => {
                    let m = [*a, *b, *c, *d, *e, *f];
                    // "Tm shall set the text matrix AND the text line
                    // matrix" (Table 108) — both, absolutely, which also
                    // makes the matrix known again after any drift.
                    self.tm = m;
                    self.tlm = m;
                    self.tm_known = true;
                    Rec::Tm(m)
                }
                _ => Rec::Ignore,
            },
            // `TD` additionally "sets the leading parameter to -ty"
            // (§9.4.2 Table 108). Tracked so the ambient `TL` this walk
            // publishes is the value actually in force — but as
            // ObservedIndirect, because re-emitting the `TD` to restore it
            // would also move the line.
            b"TD" => {
                if let [tx, ty] = n.as_slice() {
                    self.gs
                        .ambient
                        .set_indirect(TextStateParam::Leading, -*ty, "TD");
                    self.next_line(*tx, *ty);
                }
                Rec::Boundary
            }
            b"Td" => {
                if let [tx, ty] = n.as_slice() {
                    self.next_line(*tx, *ty);
                }
                Rec::Boundary
            }
            // `T*` is "0 −TL Td" (Table 108). `TL` comes from the shared
            // ambient state, which is exactly why 19.0 had to track it.
            b"T*" => {
                self.next_line(0.0, -self.gs.ambient.leading.value);
                Rec::Boundary
            }
            // --- general graphics state (§8.4.3.2) ---
            //
            // Pass 19.2: the line width is not text state, but synthetic
            // bold sets it (stroked text takes its width from here, in
            // USER space, §9.3.6), so it must be restorable.
            b"w" => {
                if let Some(v) = n.first() {
                    self.gs.line_width = LineWidth::Observed {
                        value: *v,
                        raw: buf.get(start..end).unwrap_or_default().to_vec(),
                    };
                }
                Rec::Ignore
            }
            // --- STROKING colour (§8.6.8) ---
            //
            // The uppercase twins of the fill arms below. Text painting
            // ignored these before Pass 19.2 because rendering mode 0 does
            // not stroke; synthetic bold uses mode 2, which does.
            b"G" => {
                self.gs.stroke = Self::device_fill(DeviceSpace::Gray, &n, buf, start, end);
                self.gs.last_cs_stroking = None;
                Rec::Ignore
            }
            b"RG" => {
                self.gs.stroke = Self::device_fill(DeviceSpace::Rgb, &n, buf, start, end);
                self.gs.last_cs_stroking = None;
                Rec::Ignore
            }
            b"K" => {
                self.gs.stroke = Self::device_fill(DeviceSpace::Cmyk, &n, buf, start, end);
                self.gs.last_cs_stroking = None;
                Rec::Ignore
            }
            b"CS" => {
                self.gs.last_cs_stroking = buf.get(start..end).map(<[u8]>::to_vec);
                Rec::Ignore
            }
            b"SC" | b"SCN" => {
                let mut raw = Vec::new();
                if let Some(cs) = &self.gs.last_cs_stroking {
                    raw.extend_from_slice(cs);
                    raw.push(b' ');
                }
                if let Some(here) = buf.get(start..end) {
                    raw.extend_from_slice(here);
                }
                self.gs.stroke = FillState::Other { raw };
                Rec::Ignore
            }
            // Fill-colour graphics state (§8.6.8). Recorded so Pass 14.2's
            // formatting surgery can classify (device vs Other) and RESTORE
            // the prior colour byte-faithfully after a wrapped edit. Only
            // the lowercase (fill) operators matter here; `G`/`RG`/`K`/`SC`/
            // `SCN` set the STROKE colour, which text painting does not use
            // by default (§9.3.1 render mode 0). Recording these does not
            // change Pass 14.1's REPLACE output — it never reads the field.
            b"g" => {
                self.gs.fill = Self::device_fill(DeviceSpace::Gray, &n, buf, start, end);
                self.gs.last_cs = None;
                Rec::Ignore
            }
            b"rg" => {
                self.gs.fill = Self::device_fill(DeviceSpace::Rgb, &n, buf, start, end);
                self.gs.last_cs = None;
                Rec::Ignore
            }
            b"k" => {
                self.gs.fill = Self::device_fill(DeviceSpace::Cmyk, &n, buf, start, end);
                self.gs.last_cs = None;
                Rec::Ignore
            }
            b"cs" => {
                // Set-fill-colour-space: remember the raw bytes so a
                // following `sc`/`scn` records a re-emittable `cs … scn`
                // restore sequence.
                self.gs.last_cs = buf.get(start..end).map(<[u8]>::to_vec);
                Rec::Ignore
            }
            b"sc" | b"scn" => {
                // A fill colour set in a resource-named space — pdfce does
                // not decode it (TextColor::Other). Keep the raw operator
                // bytes (with the preceding `cs`, if any) to restore verbatim.
                let mut raw = Vec::new();
                if let Some(cs) = &self.gs.last_cs {
                    raw.extend_from_slice(cs);
                    raw.push(b' ');
                }
                if let Some(here) = buf.get(start..end) {
                    raw.extend_from_slice(here);
                }
                self.gs.fill = FillState::Other { raw };
                Rec::Ignore
            }
            b"BDC" | b"BMC" => {
                self.mc_stack.push(mcid_of(self.doc, op));
                Rec::Ignore
            }
            b"EMC" => {
                self.mc_stack.pop();
                Rec::Ignore
            }
            b"Tj" => self.record_show(op, ShowOp::Tj),
            b"TJ" => self.record_show(op, ShowOp::TJ),
            // `'` is "T* then Tj" (Table 109), so it moves to the next line
            // BEFORE showing — the matrix recorded on the show must be the
            // post-move one.
            b"'" => {
                self.next_line(0.0, -self.gs.ambient.leading.value);
                self.record_show(op, ShowOp::Quote)
            }
            b"\"" => {
                // Table 109: `"` sets `Tw` and `Tc` before showing. Routed
                // through the shared update rule so both are recorded with
                // the same provenance discipline as a standalone operator.
                let raw = buf.get(start..end).unwrap_or_default();
                self.gs.ambient.apply_operator(name, &n, raw);
                // `"` is `aw ac string "` ≡ set Tw/Tc, then `'` — so it too
                // moves to the next line before showing. The leading read
                // here is the value AFTER the Tw/Tc update, which is
                // correct: `"` does not touch `TL`.
                self.next_line(0.0, -self.gs.ambient.leading.value);
                self.record_show(op, ShowOp::DoubleQuote)
            }
            _ => Rec::Ignore,
        };
        self.recs.push(OpRec { start, end, rec });
    }

    /// Decode a show operator into text + slots under the current font.
    fn record_show(&mut self, op: &Operation<'_>, kind: ShowOp) -> Rec {
        let font = self.font(&self.gs.font_name.clone());
        let mut elems: Vec<ShowElem> = Vec::new();
        let mut text = String::new();
        let mut slots: Vec<ShowSlot> = Vec::new();

        // Collect operand elements (a string, or a TJ array of strings and
        // kerning numbers).
        let mut raw: Vec<ShowElem> = Vec::new();
        for t in op.operands {
            match &t.kind {
                ContentTokenKind::Operand(Object::String(s)) => raw.push(ShowElem::Str(s.clone())),
                ContentTokenKind::Operand(Object::Array(a)) => {
                    for item in a {
                        match item {
                            Object::String(s) => raw.push(ShowElem::Str(s.clone())),
                            other => {
                                if let Some(v) = other.as_number() {
                                    raw.push(ShowElem::Num(v));
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Decode each string element into codes (simple font = 1 byte/code).
        //
        // A composite font is NOT decoded here.
        //
        // This comment used to say "edit is refused later, R-INV-4". That is
        // FALSE, and the falsehood has an operator-facing cost. Because the
        // run is never decoded, the text is never located; because it is
        // never located, `classify_font` is never reached for it; so
        // R-INV-4's carefully-worded composite refusal NEVER FIRES from
        // `edit-text`. What the operator actually sees is "text to edit was
        // not found in an editable run on the page" — which reads as "your
        // text isn't there" when the truth is "it is there, in a font pdfce
        // declines to edit". Those two lead to completely different next
        // actions, and only one of them is available.
        //
        // Found by trying to reach the R-INV-4 message through the CLI with
        // a composite fixture built for the purpose, and failing. Recorded
        // here rather than silently corrected because the fix is a change to
        // the LOCATION path — decode composite runs far enough to match, so
        // the right refusal can fire — and that belongs in its own slice
        // with its own tests, not bolted onto a comment repair.
        //
        // OWED (Pass 21.1): make a composite run locatable-but-refused
        // rather than invisible, so the specific refusal reaches the person
        // who has to act on it.
        let simple = font.as_ref().is_some_and(ExtractFont::is_simple);
        for (ei, elem) in raw.iter().enumerate() {
            match elem {
                ShowElem::Str(bytes) => {
                    if simple && let Some(f) = font.as_ref() {
                        for (bi, &byte) in bytes.iter().enumerate() {
                            let (chars, _) = f.to_unicode(u32::from(byte));
                            let t0 = text.len();
                            text.push_str(&chars);
                            let t1 = text.len();
                            slots.push(ShowSlot {
                                code: u32::from(byte),
                                width: 1,
                                elem: ei,
                                byte_in_elem: bi,
                                t0,
                                t1,
                            });
                        }
                    } else if let Some(f) = font.as_ref() {
                        // COMPOSITE: decode far enough to be FINDABLE, and no
                        // further. Text is populated so this run can be
                        // selected as the anchor; NO SLOTS ARE PUSHED.
                        //
                        // The original reason was that `ShowSlot::code` was a
                        // `u8` and could not hold a 2-byte CID. That is no
                        // longer true — it is a `u32` with a `width` as of
                        // this Pass — so the honest reasons now are two, and
                        // the second is the one that would be easy to get
                        // wrong:
                        //
                        // 1. The re-encode and splice paths downstream are
                        //    still single-byte, so a slot here would be a
                        //    handle nothing can use.
                        //
                        // 2. Pushing slots would WEAKEN the regression test
                        //    that pins the composite refusal's reachability
                        //    (`tests/composite_refusal_reachable.rs`). That
                        //    test catches someone moving the font
                        //    classification back below `match_run` — it works
                        //    because, without slots, `match_run` fails and
                        //    the wrong `NoMatch` surfaces. Give composite
                        //    runs slots and `match_run` would SUCCEED, so the
                        //    refusal would still fire from `classify_font`
                        //    and the test would pass on the broken ordering.
                        //    The guard would go quiet while the defect it
                        //    guards became reachable again.
                        //
                        // So slots arrive in the same change that makes the
                        // encoder multi-byte, and that change owes the
                        // regression test a new way to detect the ordering —
                        // asserting the refusal fires BEFORE any match work,
                        // rather than relying on match failing.
                        //
                        // The point of decoding at all is that the run must
                        // be REACHABLE for the composite refusal to fire on
                        // it. Before this, composite runs were invisible to
                        // the anchor search, so the operator got "not found"
                        // — a wrong answer — instead of a refusal naming the
                        // font.
                        //
                        // Two bytes per code assumes `Identity-H`, which is
                        // what real-world composite text overwhelmingly uses
                        // and what pdfce itself writes (Pass 21.0). A
                        // composite font on some other CMap decodes to
                        // nothing here and stays invisible — the OLD
                        // behaviour, not a new regression, and narrower than
                        // it was.
                        // Slice pattern rather than `pair[0]`/`pair[1]`:
                        // `chunks_exact(2)` does guarantee the length, but
                        // `clippy::indexing_slicing` is DENIED crate-wide
                        // because this crate parses untrusted input, and
                        // carving out an exception wherever the author can
                        // see the invariant is how a deny-by-default rule
                        // becomes advisory. The pattern makes the guarantee
                        // the compiler's rather than the reader's.
                        for pair in bytes.chunks_exact(2) {
                            let [hi, lo] = pair else { continue };
                            let code = u32::from(*hi) << 8 | u32::from(*lo);
                            let (chars, _) = f.to_unicode(code);
                            text.push_str(&chars);
                        }
                    }
                    elems.push(ShowElem::Str(bytes.clone()));
                }
                ShowElem::Num(v) => elems.push(ShowElem::Num(*v)),
            }
        }

        // Snapshot the matrices BEFORE this operator's own glyphs advance
        // them: `text_matrix` is defined as the matrix in force at the start
        // of the operator, which is what a shear must be premultiplied into.
        let at_start = ShowData {
            font_name: self.gs.font_name.clone(),
            tf_size: self.gs.tf_size,
            text_state: self.gs.ambient.clone(),
            mcid: self.current_mcid(),
            op: kind,
            elems,
            text,
            slots,
            fill_color: self.gs.fill.clone(),
            stroke_color: self.gs.stroke.clone(),
            line_width: self.gs.line_width.clone(),
            text_matrix: self.tm,
            matrix_known: self.tm_known,
        };
        self.advance_matrix(font.as_ref(), &at_start.elems);
        Rec::Show(Box::new(at_start))
    }
}

// ===================================================================
// The public entry point
// ===================================================================

/// Edit the page's own text in place: locate the run, re-encode the new
/// text, relayout the line, and save incrementally.
///
/// Returns the appended PDF bytes plus an [`EditReport`] carrying every
/// disclosure. A character the run's font cannot provide yields
/// [`EditError::Refused`] BEFORE any save — the refusal never reaches the
/// writer (rule 4 / R71).
///
/// # Errors
///
/// See [`EditError`]: a named refusal, no match, an unsupported run, an
/// encrypted document, a parse/page-tree failure, or a save failure.
pub fn edit_text(
    doc: &Document,
    req: &EditRequest,
    opts: &EditOptions,
) -> Result<EditOutcome, EditError> {
    if doc.trailer().contains_key(b"Encrypt") {
        return Err(EditError::Encrypted);
    }
    let pages = page_tree::pages(doc)?;
    let page = pages
        .get(req.page_index)
        .ok_or(EditError::PageIndex(req.page_index))?;
    // BASE READ (decision 018 caller audit): `edit_text` is the one-shot
    // `&Document` entry point — it plans against the file as loaded and
    // hands the plan to an incremental save. The GUI's accumulating
    // multi-edit path is `EditSession::current_page_content`, not this.
    let stream = ContentStream::from_page(&doc.view(), page)?;
    let plan = plan_edit(doc, page, &stream, req, opts)?;
    // Incremental save (R34/R70): replace the first content object with the
    // spliced buffer, empty any extras. The report's content_object /
    // extra_objects_emptied are already correct (plan derives them from
    // `page.contents`), so the returned identity is discarded here.
    let (bytes, _content_object, _extra) = write_incremental(doc, page, &plan.new_content)?;
    Ok(EditOutcome {
        bytes,
        report: plan.report,
    })
}

/// The result of planning an edit WITHOUT committing it: the fully-spliced
/// replacement content-stream buffer plus the complete [`EditReport`].
///
/// This is the seam (Pass 14.3 UI spec §0.2) that lets the interactive
/// [`EditSession::edit_text`](crate::edit::EditSession::edit_text) reuse the
/// EXACT locate/re-encode/relayout/gate logic of the free-function
/// [`edit_text`] while landing the mutation as one undo-able command against
/// the session's in-memory object graph, instead of producing already-saved
/// bytes. `content_object` and `extra_objects_emptied` are derived from
/// `page.contents` (not from a save), so the report is complete before any
/// write happens — both the free function and the session path then perform
/// their own write step (`write_incremental` vs. session staging + command).
pub(crate) struct EditPlan {
    /// The spliced, decoded replacement content for the page's first content
    /// object (the whole page content, edited).
    pub(crate) new_content: Vec<u8>,
    /// The complete disclosure/diagnostic report.
    pub(crate) report: EditReport,
}

/// Plan a REPLACE edit over an already-decoded content `stream`: locate the
/// anchor, re-encode, gate, relayout, and splice — returning the new content
/// buffer and the full report, but performing NO save.
///
/// Factored out of [`edit_text`] so the interactive session path reuses the
/// identical surgery (Pass 14.3 §0.2). The free function passes the page's
/// [`ContentStream::from_page`] decode; the session passes a
/// [`ContentStream::parse`] of its own current (possibly already-edited) raw
/// content, which is what makes five sequential edits accumulate correctly.
///
/// # Errors
///
/// See [`EditError`]: a named refusal, no match, an unsupported run, a
/// page with no `/Contents`, or a content-parse failure.
pub(crate) fn plan_edit(
    doc: &Document,
    page: &Page,
    stream: &ContentStream,
    req: &EditRequest,
    opts: &EditOptions,
) -> Result<EditPlan, EditError> {
    // Content-object identity: also validates the page HAS a content stream
    // to edit (matching `write_incremental`'s own first check), and yields
    // the report's save-independent object numbers.
    let content_id = *page
        .contents
        .first()
        .ok_or_else(|| EditError::Unsupported("the page has no /Contents to edit".to_owned()))?;
    let extra_emptied = page.contents.len().saturating_sub(1) as u64;

    // --- pass 1: record every operator with its text state ---
    let mut walk = Walk::new(doc, &page.resources);
    for op in stream.operations() {
        walk.operation(&op, &stream.buf);
    }
    let recs = walk.recs;

    // --- locate the anchor show operator ---
    let anchor_index = find_anchor(&recs, req)?;
    let OpRec {
        start: a_start,
        end: a_end,
        rec: Rec::Show(anchor),
    } = recs
        .get(anchor_index)
        .ok_or(EditError::NoMatch(req.find.clone()))?
    else {
        return Err(EditError::NoMatch(req.find.clone()));
    };
    if matches!(anchor.op, ShowOp::Quote | ShowOp::DoubleQuote) {
        return Err(EditError::Unsupported(
            "editing a run shown with the ' or \" operator is deferred (first cut edits Tj/TJ)"
                .to_owned(),
        ));
    }

    // --- resolve the anchor font + classify it (R-INV-2/3/4) ---
    //
    // ORDER MATTERS, and it was wrong. This block used to sit AFTER
    // `match_run`, which meant a composite run reported `NoMatch` — "text to
    // edit was not found in an editable run on the page" — instead of the
    // composite refusal, because `match_run` needs per-code slots and
    // composite runs have none. The operator was told their text was absent
    // when it was present in a font pdfce declines to edit.
    //
    // Classifying first fixes it without touching `match_run`: a font-level
    // refusal (R-INV-2/3/4) is a property of the RUN, not of whether the
    // sought text happens to be inside it, so there was never a reason to
    // establish the match before applying it. For a simple font nothing
    // changes — the same call, the same inputs, a few lines earlier.
    let font_dict =
        resolve_font_dict(doc, &page.resources, &anchor.font_name).ok_or_else(|| {
            EditError::Unsupported("the run's font resource is unresolvable".to_owned())
        })?;
    // `&doc.view()` (Pass 17.1) — see `ExtractFont::resolve`. The text-edit
    // planner is base-relative by contract (`EditSession::edit_text` plans
    // against the base and splices the result), so the base view is correct.
    let font = ExtractFont::resolve(&doc.view(), font_dict);
    let class = classify_font(doc, font_dict, &font)?;

    // --- map the find text to a contiguous code range in one element ---
    let m = match_run(anchor, &req.find)?;

    // --- build the inverse map and encode the replacement (R-INV-1/5/6/7/8) ---
    let glyph_names = font.glyph_names().ok_or_else(|| {
        EditError::Unsupported("the run's font has no invertible encoding".to_owned())
    })?;
    let inverse = InverseEncoding::build(&font.base_font, glyph_names);

    // The R-INV-5 tie-break seed: codes already used in this run.
    // Narrowed to `u8` deliberately: this is the "prefer codes already in
    // use" hint for the SINGLE-BYTE inverse encoder, and a composite code
    // has no meaning to it. `filter_map` rather than a cast, so a
    // multi-byte code is dropped rather than silently truncated into a
    // different, valid, wrong code.
    let prefer: BTreeSet<u8> = anchor
        .slots
        .iter()
        .filter_map(|s| u8::try_from(s.code).ok())
        .collect();
    let encoded = inverse
        .encode_str(&req.replace, &prefer)
        .map_err(EditError::Refused)?;

    // --- embedded-subset floor: a new code the subset does not already
    //     carry is REFUSED by name (the one refusal in the four-case table).
    if class.embedded && class.subset {
        let carried = carried_codes(&recs, &anchor.font_name);
        for (u, &code) in req.replace.chars().zip(encoded.codes.iter()) {
            if !carried.contains(&code) {
                return Err(EditError::Refused(Refusal {
                    trigger: RInvTrigger::TargetAbsent,
                    character: Some(u),
                    base_font: font.base_font.clone(),
                    message: format!(
                        "R-INV-1 (embedded-subset floor): character U+{:04X} '{}' maps to code {} \
                         which font '{}' (an embedded SUBSET) does not already carry on this page; \
                         embedding a new glyph is deferred to FF-C (font subsetting). This is \
                         exactly Acrobat's 'embedded-but-not-local' floor.",
                        u as u32, u, code, font.base_font
                    ),
                }));
            }
        }
    }

    // --- advance delta (§9.4.4) ---
    let a_old: f64 = m
        .old_codes
        .iter()
        .map(|&c| glyph_advance(&font, c, anchor))
        .sum();
    let a_new: f64 = encoded
        .codes
        .iter()
        .map(|&c| glyph_advance(&font, c, anchor))
        .sum();
    let delta = a_new - a_old;

    // --- re-emit the anchor operator ---
    let pin_num = match opts.disposition {
        FollowerDisposition::Pin => compensating_tj(delta, anchor.tf_size, anchor.th()),
        FollowerDisposition::Reflow => None,
    };
    let new_op_bytes = emit_edited_operator(anchor, &m, &encoded.codes, pin_num);
    let mut edits: Vec<(usize, usize, Vec<u8>)> = vec![(*a_start, *a_end, new_op_bytes)];

    // --- reflow: shift following absolute Tm(s) on the same line by ΔA ---
    let mut followers = 0u64;
    if matches!(opts.disposition, FollowerDisposition::Reflow) && delta != 0.0 {
        for r in recs.iter().skip(anchor_index + 1) {
            match &r.rec {
                Rec::Boundary => break,
                Rec::Show(s) if matches!(s.op, ShowOp::Quote | ShowOp::DoubleQuote) => break,
                Rec::Tm([a, b, c, d, e, f]) => {
                    let moved = emit_tm([*a, *b, *c, *d, *e + delta, *f]);
                    edits.push((r.start, r.end, moved));
                    followers += 1;
                }
                _ => {}
            }
        }
    }

    // --- splice the edits into the decoded buffer ---
    let new_content = splice(&stream.buf, &mut edits);

    // --- assemble the report + disclosures (NO save happens here; the
    //     caller — the free function or the session — performs its own
    //     write step, Pass 14.3 §0.2) ---
    let mut disclosures = Vec::new();
    disclosures.extend(encoded.disclosures);
    disclosures.push(trust_disclosure(class.embedded, &font.base_font));
    disclosures.push(
        "save: this edit was written INCREMENTALLY (R34/R70); the prior text survives in the \
         document's revision history by design. To truly remove text, use redaction (Pass 8) — a \
         distinct, security operation."
            .to_owned(),
    );
    if matches!(opts.disposition, FollowerDisposition::Reflow) {
        disclosures.push(
            "relayout: the edited line was shifted by the advance delta and MAY now overflow the \
             original right margin; block re-wrap (reflow) is deferred (FF-A) — enable reflow to \
             re-wrap."
                .to_owned(),
        );
    }
    if let Some(mcid) = anchor.mcid {
        disclosures.push(format!(
            "tagged PDF: the edit is inside a marked-content sequence (/MCID {mcid}); its \
             BDC/EMC+MCID wrapper was PRESERVED (structure references stay valid), but the \
             structure tree's /ActualText and reading order were NOT updated and are now STALE \
             (a stale /ActualText wins on extraction, §14.9.4). pdfce discloses this rather than \
             silently corrupting the accessibility tree (R72)."
        ));
    }
    if extra_emptied > 0 {
        disclosures.push(format!(
            "multi-stream page: {extra_emptied} additional /Contents stream(s) were collapsed \
             into the first and emptied so the edit's byte offsets stay coherent."
        ));
    }

    let report = EditReport {
        base_font: font.base_font.clone(),
        glyph_source: if class.embedded {
            EditGlyphSource::Embedded
        } else {
            EditGlyphSource::NonEmbedded
        },
        subset: class.subset,
        advance_delta: delta,
        disposition: opts.disposition,
        followers_repositioned: followers,
        tagged_mcid: anchor.mcid,
        content_object: content_id.num,
        extra_objects_emptied: extra_emptied,
        disclosures,
    };
    Ok(EditPlan {
        new_content,
        report,
    })
}

// ===================================================================
// Locating + matching
// ===================================================================

/// Find the anchor operator: the pinned span if given, else the first show
/// operator whose decoded text contains `find`.
pub(crate) fn find_anchor(recs: &[OpRec], req: &EditRequest) -> Result<usize, EditError> {
    for (i, r) in recs.iter().enumerate() {
        let Rec::Show(s) = &r.rec else { continue };
        if let Some(pin) = req.pinned_span {
            if pin_names_operator(r, pin) {
                return Ok(i);
            }
            continue;
        }
        if s.text.contains(&req.find) {
            return Ok(i);
        }
    }
    Err(EditError::NoMatch(req.find.clone()))
}

/// Whether `pin` names the operation `r` — under **either** of the two byte-
/// span conventions this codebase publishes for "the show operator".
///
/// # The defect this function exists to close (found Pass 19.3, live)
///
/// There are two conventions, they disagree, and until this was written the
/// pinned path silently required the one that no caller actually produces:
///
/// | Producer | Span of `(hello) Tj` at offset 23 |
/// |---|---|
/// | [`op_span`] — this module's own walk, recorded into [`OpRec`] | `23..39` (first operand → operator end) |
/// | [`GlyphProvenance::operator_span`](crate::text_extract::GlyphProvenance::operator_span) — the extraction walk (`page.rs`, `self.cur_op_span = op.operator.span`) | `37..39` (the `Tj` token alone) |
///
/// The old test for equality against the FIRST form meant that every request
/// pinned from provenance — which is every request the GUI's Edit Text tool
/// builds, since it pins from `model.provenance(...).operator_span` — failed
/// with [`EditError::NoMatch`] before it ever reached the surgery. Observed
/// in the running application: the shipped property bar's "Apply size"
/// refused with *"text to format (…) was not found in an editable run on the
/// page"* on a perfectly ordinary one-`Tj` page. Two doc comments asserted
/// the opposite — `EditRequest::pinned_span`'s "this surgery … matches the
/// same span", and `page.rs`'s "the surgery locates the operator by exactly
/// this span" — which is presumably why it went unnoticed: the claim was
/// written down, so it was believed.
///
/// # Why accept both rather than pick one
///
/// The two spans are not rival encodings of the same idea; each is correct
/// for its own reader. Extraction publishes the operator *token* because that
/// is what identifies the operator to a consumer that never re-tokenizes the
/// operands. The authoring walk records the operand-inclusive extent because
/// that is the byte range it is about to splice. Forcing either side to adopt
/// the other's convention would change a published field's meaning (and, for
/// provenance, one that is already in operator-visible CLI output) to fix a
/// comparison — so the comparison is what gets fixed.
///
/// # The rule, and why it cannot alias the wrong operator
///
/// A pin names `r` when it **ends where `r` ends** and **starts at or after
/// `r` starts**. Both conventions satisfy that for the operator they mean.
/// Nothing else can: two distinct operations in one stream have distinct end
/// offsets (an operator token is at least one byte and they do not overlap),
/// so `pin.end() == r.end` already identifies the operation uniquely; the
/// start bound is kept as a cheap sanity check that the pin lies inside the
/// operation rather than reaching back over an earlier one.
fn pin_names_operator(r: &OpRec, pin: ByteSpan) -> bool {
    pin.end() == r.end && pin.start >= r.start
}

/// A resolved match within one show operator.
///
/// `pub(crate)` so Pass 14.2's formatting surgery can reuse the identical
/// single-element, contiguous-code-range match the REPLACE surgery uses.
pub(crate) struct MatchRun {
    /// Which element the matched codes live in.
    pub(crate) elem: usize,
    /// Byte range within that element's string.
    pub(crate) b_lo: usize,
    pub(crate) b_hi: usize,
    /// The old codes being replaced (for `A_old`).
    pub(crate) old_codes: Vec<u8>,
}

/// Map `find` (a substring of the operator's decoded text) to a contiguous
/// code range within a single string element.
pub(crate) fn match_run(anchor: &ShowData, find: &str) -> Result<MatchRun, EditError> {
    if find.is_empty() {
        return Err(EditError::Unsupported("empty find text".to_owned()));
    }
    let pos = anchor
        .text
        .find(find)
        .ok_or_else(|| EditError::NoMatch(find.to_owned()))?;
    let end = pos + find.len();

    let matched: Vec<&ShowSlot> = anchor
        .slots
        .iter()
        .filter(|s| s.t0 < end && s.t1 > pos)
        .collect();
    let first = matched
        .first()
        .ok_or_else(|| EditError::NoMatch(find.to_owned()))?;
    let elem = first.elem;
    if matched.iter().any(|s| s.elem != elem) {
        return Err(EditError::Unsupported(
            "the match spans more than one TJ string element (cross-element edit deferred)"
                .to_owned(),
        ));
    }
    // Simple font ⇒ one byte per code, so the matched byte range is
    // contiguous from the first matched code to just past the last.
    let b_lo = matched.iter().map(|s| s.byte_in_elem).min().unwrap_or(0);
    // `+ width`, not `+ 1`: the end of a code is its start plus however many
    // bytes it occupied. Those were the same number for every code that
    // could reach here before Pass 21.1, which is exactly why the constant
    // looked correct.
    let b_hi = matched
        .iter()
        .map(|s| s.byte_in_elem + usize::from(s.width))
        .max()
        .unwrap_or(b_lo);
    // `MatchRun::old_codes` stays `Vec<u8>`: its consumers are the
    // single-byte re-encode paths (14.1 replace, 14.2 format), which have no
    // representation for a multi-byte code. Narrowed with `filter_map`
    // rather than a cast — a truncated code is a DIFFERENT, VALID code, and
    // would be spliced back into the page as confidently wrong text.
    //
    // A composite run never reaches here: `classify_font` refuses it above
    // `match_run` (Pass 21.1). This narrowing is therefore belt to that
    // brace, and it is the belt that will still be holding if the ordering
    // is ever changed back.
    let old_codes = matched
        .iter()
        .filter_map(|s| u8::try_from(s.code).ok())
        .collect();
    Ok(MatchRun {
        elem,
        b_lo,
        b_hi,
        old_codes,
    })
}

/// Every code shown under `font_name` anywhere on the page — the "already
/// carries" set for the embedded-subset floor (a code in use ⇒ its glyph is
/// physically present in the subset program).
pub(crate) fn carried_codes(recs: &[OpRec], font_name: &[u8]) -> BTreeSet<u8> {
    let mut set = BTreeSet::new();
    for r in recs {
        if let Rec::Show(s) = &r.rec
            && s.font_name == font_name
        {
            for slot in &s.slots {
                // Same narrowing rationale as `prefer`: this set answers the
                // SIMPLE-font embedded-subset floor ("is this one-byte code
                // already carried on the page"), so a multi-byte code is
                // dropped rather than truncated. Truncating would add a
                // code the page does not actually carry, which would let
                // R-INV-1 pass on a glyph that is not there.
                if let Ok(b) = u8::try_from(slot.code) {
                    set.insert(b);
                }
            }
        }
    }
    set
}

// ===================================================================
// Font classification (R-INV-2/3/4)
// ===================================================================

/// The classification the font-on-edit gate needs.
///
/// `pub(crate)` so Pass 14.2's family-change surgery can classify the
/// TARGET font with the identical R-INV-2/3/4 refuse triggers before
/// re-encoding the run into it.
pub(crate) struct FontClass {
    pub(crate) embedded: bool,
    pub(crate) subset: bool,
}

/// Classify the anchor font and apply the font-level refuse triggers
/// R-INV-2/3/4 (the per-character triggers are the inverse map's job).
pub(crate) fn classify_font(
    doc: &Document,
    font_dict: &Dict,
    font: &ExtractFont,
) -> Result<FontClass, EditError> {
    let subtype = font_dict
        .get(b"Subtype")
        .map(|o| doc.resolve(o))
        .and_then(Object::as_name)
        .map(|n| n.as_bytes().to_vec())
        .unwrap_or_default();

    // R-INV-4: composite (Type 0 / CIDFont).
    //
    // Still refused. The re-encoding path is single-byte end to end —
    // `InverseEncoding` maps a char to `Vec<u8>` and the operand writer
    // assumes that width — so making composite runs editable is a change to
    // the whole encoding seam, not a branch here. That is the rest of Pass
    // 21.1.
    //
    // What HAS changed is WHY. This message used to cite glyph subsetting
    // (FF-C) as a co-blocker. FF-C shipped in Pass 21.0, so that sentence
    // became false and would have sent anyone reading it to look at the
    // wrong thing. It also read as a permanent limitation, when standing
    // rule R110 has since established that composite editability is a
    // property of the FONT — whether its `/ToUnicode` is injective — rather
    // than a blanket one.
    //
    // So the refusal now consults the CMap and says which kind of "no" this
    // is. A font whose map cannot be inverted will NEVER be editable and
    // names the specific obstruction; a font whose map inverts cleanly is
    // waiting on pdfce, and says so. That distinction costs one lookup and
    // is the difference between an operator fixing their document and an
    // operator waiting for a release that would not have helped them.
    if subtype.as_slice() == b"Type0" || !font.is_simple() {
        let why = match font.to_unicode_cmap() {
            Some(cmap) => match cmap.injective_inverse() {
                Ok(_) => "This font's character map CAN be inverted, so editing it is possible in principle; pdfce's re-encoding path does not handle multi-byte codes yet."
                    .to_owned(),
                Err(e) => format!(
                    "This font's character map cannot be inverted, so pdfce could not know which code to write back: {e}"
                ),
            },
            None => "This font declares no /ToUnicode character map, so pdfce cannot tell which code produces a given character."
                .to_owned(),
        };
        return Err(EditError::Refused(Refusal {
            trigger: RInvTrigger::Composite,
            character: None,
            base_font: font.base_font.clone(),
            message: format!(
                "R-INV-4: font '{}' is a composite (Type 0 / CIDFont) run, which pdfce cannot edit in place. {why}",
                font.base_font
            ),
        }));
    }

    let descriptor = font_dict
        .get(b"FontDescriptor")
        .map(|o| doc.resolve(o))
        .and_then(Object::as_dict);
    let embedded = descriptor.is_some_and(|d| {
        d.contains_key(b"FontFile") || d.contains_key(b"FontFile2") || d.contains_key(b"FontFile3")
    });
    let flags = descriptor
        .and_then(|d| {
            doc.resolve(d.get(b"Flags").unwrap_or(&Object::Null))
                .as_int()
        })
        .unwrap_or(0);
    // §9.8.2 Table 123: bit 3 (value 4) Symbolic, bit 6 (value 32) Nonsymbolic.
    let symbolic = (flags & 0x4) != 0 && (flags & 0x20) == 0;

    // Is /Encoding a usable (invertible) Name or Dict, vs absent / a stream?
    let encoding_usable = matches!(
        font_dict.get(b"Encoding").map(|o| doc.resolve(o)),
        Some(Object::Name(_) | Object::Dict(_))
    );

    // R-INV-2: symbolic embedded font whose code→glyph relation lives in the
    // font program's built-in cmap; /Encoding is ignored (§9.6.6.4 Branch B),
    // so pdfce cannot build an invertible table from PDF objects.
    if symbolic && embedded && !encoding_usable {
        return Err(EditError::Refused(Refusal {
            trigger: RInvTrigger::SymbolicNoEncoding,
            character: None,
            base_font: font.base_font.clone(),
            message: format!(
                "R-INV-2: font '{}' is symbolic with a built-in/custom cmap and no usable \
                 /Encoding (§9.6.6.4 Branch B ignores /Encoding); its code↔glyph relation lives \
                 inside the embedded program, which pdfce-core does not parse (R21). Editing is \
                 refused.",
                font.base_font
            ),
        }));
    }

    // R-INV-3: the only code↔char relation is /ToUnicode (one-way/lossy, §0),
    // and there is no authoritative /Encoding to invert — the base table was
    // unreadable and a /ToUnicode is present.
    let builtin_unreadable = font.notes.iter().any(|n| {
        matches!(
            n,
            crate::text_extract::font::FontNote::BuiltinEncodingUnreadable
        )
    });
    let has_to_unicode = font_dict.contains_key(b"ToUnicode");
    if !encoding_usable && builtin_unreadable && has_to_unicode {
        return Err(EditError::Refused(Refusal {
            trigger: RInvTrigger::ToUnicodeOnly,
            character: None,
            base_font: font.base_font.clone(),
            message: format!(
                "R-INV-3: font '{}' relates codes to characters only through /ToUnicode, which is \
                 one-way and lossy (§0) and cannot be inverted; it has no authoritative /Encoding \
                 to invert instead. Editing is refused.",
                font.base_font
            ),
        }));
    }

    Ok(FontClass {
        embedded,
        subset: is_subset_tag(&font.base_font),
    })
}

// ===================================================================
// Geometry + emission
// ===================================================================

/// The §9.4.4 horizontal advance `tx` for one code, in text-space units,
/// under the run's text state. `Tw` applies only to the single byte `0x20`
/// (§9.3.3). Width `w0` comes from the SAME `/Widths`/AFM the render path
/// uses ([`ExtractFont::width`] already scales it to text space).
fn glyph_advance(font: &ExtractFont, code: u8, s: &ShowData) -> f64 {
    glyph_advance_with(font, code, s.tf_size, s.tc(), s.tw(), s.th())
}

/// The §9.4.4 advance for one code with **explicit** text-state scalars —
/// the size- and font-independent form Pass 14.2 needs, since a formatting
/// edit varies the font (`font`) and/or the size (`tf_size`) from the run's
/// recorded state while `Tc`/`Tw`/`Th` stay put. [`glyph_advance`] is the
/// Pass-14.1 wrapper that passes the run's own recorded state, so its
/// numbers are byte-for-byte what Pass 14.1 always computed.
pub(crate) fn glyph_advance_with(
    font: &ExtractFont,
    code: u8,
    tf_size: f64,
    tc: f64,
    tw: f64,
    th: f64,
) -> f64 {
    let w0 = f64::from(font.width(u32::from(code)));
    let tw = if code == 0x20 { tw } else { 0.0 };
    (w0 * tf_size + tc + tw) * th
}

/// The compensating `TJ` number that consumes `ΔA` so survivors do not move
/// (surgery ref §2): `N = ΔA·1000/(Tfs·Th)`. `None` when the scale is ~0
/// (invisible text advances nothing — pinning is a no-op).
pub(crate) fn compensating_tj(delta: f64, tfs: f64, th: f64) -> Option<f64> {
    let scale = tfs * th;
    if scale.abs() < f64::EPSILON {
        None
    } else {
        Some(delta * 1000.0 / scale)
    }
}

/// Re-emit the anchor operator with the matched codes replaced by
/// `new_codes` and, for PIN, a trailing compensating number.
fn emit_edited_operator(
    anchor: &ShowData,
    m: &MatchRun,
    new_codes: &[u8],
    pin_num: Option<f64>,
) -> Vec<u8> {
    // Build the final element list: the matched element's string has its
    // [b_lo, b_hi) byte range replaced by the new codes.
    let mut elems: Vec<ShowElem> = Vec::new();
    for (i, e) in anchor.elems.iter().enumerate() {
        match e {
            ShowElem::Str(bytes) if i == m.elem => {
                let mut out = Vec::new();
                out.extend_from_slice(bytes.get(..m.b_lo).unwrap_or(&[]));
                out.extend_from_slice(new_codes);
                out.extend_from_slice(bytes.get(m.b_hi..).unwrap_or(&[]));
                elems.push(ShowElem::Str(out));
            }
            ShowElem::Str(bytes) => elems.push(ShowElem::Str(bytes.clone())),
            ShowElem::Num(v) => elems.push(ShowElem::Num(*v)),
        }
    }

    // A single-string Tj under REFLOW stays a `(str) Tj`; anything else (a
    // pin compensation, or a genuine TJ) is emitted as a `[ … ] TJ` array.
    let single_str = matches!(elems.as_slice(), [ShowElem::Str(_)]);
    if anchor.op == ShowOp::Tj && single_str && pin_num.is_none() {
        let mut out = Vec::new();
        if let Some(ShowElem::Str(s)) = elems.first() {
            emit_literal_string(&mut out, s);
        }
        out.extend_from_slice(b" Tj");
        return out;
    }

    let mut out = Vec::new();
    out.push(b'[');
    let mut first = true;
    for e in &elems {
        if !first {
            out.push(b' ');
        }
        first = false;
        match e {
            ShowElem::Str(s) => emit_literal_string(&mut out, s),
            ShowElem::Num(v) => emit_number(&mut out, *v),
        }
    }
    if let Some(n) = pin_num {
        out.push(b' ');
        emit_number(&mut out, n);
    }
    out.extend_from_slice(b"] TJ");
    out
}

/// Emit a list of [`ShowElem`]s as one show operator: a lone string as
/// `(s) Tj`; anything else (multiple strings, or strings with `TJ` kerning
/// numbers) as `[ … ] TJ`. This is Pass 14.2's segment emitter — the
/// formatting surgery splits an anchor operator into pre/mid/post element
/// lists and emits each with this. An empty list yields empty bytes; the
/// caller is expected to skip an empty segment rather than emit `[] TJ`.
pub(crate) fn emit_show(elems: &[ShowElem]) -> Vec<u8> {
    if elems.is_empty() {
        return Vec::new();
    }
    if let [ShowElem::Str(s)] = elems {
        let mut out = Vec::new();
        emit_literal_string(&mut out, s);
        out.extend_from_slice(b" Tj");
        return out;
    }
    let mut out = Vec::new();
    out.push(b'[');
    let mut first = true;
    for e in elems {
        if !first {
            out.push(b' ');
        }
        first = false;
        match e {
            ShowElem::Str(s) => emit_literal_string(&mut out, s),
            ShowElem::Num(v) => emit_number(&mut out, *v),
        }
    }
    out.extend_from_slice(b"] TJ");
    out
}

/// Re-emit a `Tm` operator from its six operands.
pub(crate) fn emit_tm(m: [f64; 6]) -> Vec<u8> {
    let mut out = Vec::new();
    for (i, v) in m.iter().enumerate() {
        if i > 0 {
            out.push(b' ');
        }
        emit_number(&mut out, *v);
    }
    out.extend_from_slice(b" Tm");
    out
}

/// Splice sorted, non-overlapping `(start, end, bytes)` edits into `buf`.
pub(crate) fn splice(buf: &[u8], edits: &mut [(usize, usize, Vec<u8>)]) -> Vec<u8> {
    edits.sort_by_key(|e| e.0);
    let mut out = Vec::with_capacity(buf.len());
    let mut cursor = 0usize;
    for (start, end, bytes) in edits.iter() {
        if *start < cursor {
            continue; // defensive: skip an overlapping edit
        }
        if let Some(gap) = buf.get(cursor..*start) {
            out.extend_from_slice(gap);
        }
        out.extend_from_slice(bytes);
        cursor = *end;
    }
    if let Some(tail) = buf.get(cursor..) {
        out.extend_from_slice(tail);
    }
    out
}

// ===================================================================
// Save (incremental, R34/R70)
// ===================================================================

/// Replace the page's first content object with `new_content` (emptying any
/// extra content objects) and save **incrementally**. Returns the appended
/// bytes, the rewritten content object number, and how many extras were
/// emptied.
pub(crate) fn write_incremental(
    doc: &Document,
    page: &Page,
    new_content: &[u8],
) -> Result<(Vec<u8>, u32, u64), EditError> {
    let content_id = *page
        .contents
        .first()
        .ok_or_else(|| EditError::Unsupported("the page has no /Contents to edit".to_owned()))?;

    let mut dirty = DirtySet::empty();
    let base_len = doc.bytes().len();
    let mut staging: Vec<u8> = Vec::new();

    let span = stage(&mut staging, base_len, new_content);
    dirty.replace(content_id, make_raw_stream(span, new_content.len()));

    let mut extra = 0u64;
    for id in page.contents.iter().skip(1) {
        let empty = stage(&mut staging, base_len, &[]);
        dirty.replace(*id, make_raw_stream(empty, 0));
        extra += 1;
    }

    dirty.set_staging(staging);
    let (bytes, _report) = save_incremental(doc, &dirty, &SaveOptions::identity())?;
    Ok((bytes, content_id.num, extra))
}

/// Append `bytes` to staging and return their combined-space span.
fn stage(staging: &mut Vec<u8>, base_len: usize, bytes: &[u8]) -> ByteSpan {
    let start = base_len + staging.len();
    staging.extend_from_slice(bytes);
    ByteSpan::new(start, bytes.len())
}

/// A raw (unfiltered) content stream object with the given data span and
/// length — the edited content is emitted verbatim, no `/Filter`.
///
/// `pub(crate)` so the session-integrated
/// [`EditSession::edit_text`](crate::edit::EditSession::edit_text) (Pass 14.3
/// §0.2) builds the identical replacement Stream object the free-function
/// `write_incremental` path builds — one definition, no drift.
pub(crate) fn make_raw_stream(span: ByteSpan, len: usize) -> Object {
    let mut dict = Dict::new();
    dict.insert(
        Name::from(b"Length"),
        Object::Integer(i64::try_from(len).unwrap_or(i64::MAX)),
    );
    Object::Stream(Stream {
        dict,
        data_span: span,
    })
}

// ===================================================================
// Small helpers
// ===================================================================

/// The byte span (operands + operator) of a whole operation.
fn op_span(op: &Operation<'_>) -> (usize, usize) {
    let start = op
        .operands
        .first()
        .map_or(op.operator.span.start, |t| t.span.start);
    (start, op.operator.span.end())
}

/// The `/MCID` integer of a `BDC`/`BMC` operator, if its property operand is
/// an inline dict carrying one (§14.7.4.2). A named property resource is not
/// resolved in the first cut — its MCID is treated as absent.
fn mcid_of(doc: &Document, op: &Operation<'_>) -> Option<i64> {
    for t in op.operands {
        if let ContentTokenKind::Operand(Object::Dict(d)) = &t.kind {
            return doc.resolve(d.get(b"MCID")?).as_int();
        }
    }
    None
}

/// Resolve a `/Font /<name>` resource to its font dictionary.
pub(crate) fn resolve_font_dict<'a>(
    doc: &'a Document,
    resources: &'a Dict,
    name: &[u8],
) -> Option<&'a Dict> {
    let fonts = resources
        .get(b"Font")
        .map(|o| doc.resolve(o))
        .and_then(Object::as_dict)?;
    fonts
        .get(name)
        .map(|o| doc.resolve(o))
        .and_then(Object::as_dict)
}

/// Resolve a `/Font /<name>` resource to an [`ExtractFont`].
fn resolve_font(doc: &Document, resources: &Dict, name: &[u8]) -> Option<ExtractFont> {
    resolve_font_dict(doc, resources, name).map(|d| ExtractFont::resolve(&doc.view(), d))
}

/// Whether a `/BaseFont` name carries a §9.6.4 subset tag (`ABCDEF+…`):
/// exactly six uppercase letters then `+`.
pub(crate) fn is_subset_tag(base_font: &str) -> bool {
    matches!(base_font.split_once('+'), Some((tag, _))
        if tag.len() == 6 && tag.bytes().all(|b| b.is_ascii_uppercase()))
}

/// The three-trust-level disclosure the core can produce (Embedded vs
/// NonEmbedded); the shell refines NonEmbedded into Bundled/Supplied.
pub(crate) fn trust_disclosure(embedded: bool, base_font: &str) -> String {
    if embedded {
        format!(
            "font: '{base_font}' has an embedded program; the edit renders with the document's \
             own glyphs (GlyphSource::Embedded)."
        )
    } else {
        format!(
            "font: '{base_font}' is NON-embedded; a bundled Base-14 substitute or an \
             operator-supplied face (--font-dir, decision 012) renders the edited glyphs \
             (shapes only — positions come from /Widths). The shell reports Bundled vs Supplied."
        )
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

    /// A minimal one-page PDF with a Helvetica (WinAnsi, non-embedded) run.
    /// `content` is the page content stream; the font is object 5.
    fn helvetica_pdf(content: &str) -> Vec<u8> {
        let mut objects: Vec<(u32, Vec<u8>)> = Vec::new();
        objects.push((1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()));
        objects.push((
            2,
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] \
              /Resources << /Font << /F1 5 0 R >> >> >>"
                .to_vec(),
        ));
        objects.push((
            3,
            b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>".to_vec(),
        ));
        let body = content.as_bytes();
        let mut s = format!("<< /Length {} >>\nstream\n", body.len()).into_bytes();
        s.extend_from_slice(body);
        s.extend_from_slice(b"\nendstream");
        objects.push((4, s));
        objects.push((
            5,
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
                .to_vec(),
        ));

        let mut out = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();
        let mut offsets = std::collections::BTreeMap::new();
        for (num, obj) in &objects {
            offsets.insert(*num, out.len());
            out.extend_from_slice(format!("{num} 0 obj\n").as_bytes());
            out.extend_from_slice(obj);
            out.extend_from_slice(b"\nendobj\n");
        }
        let xref_at = out.len();
        let highest = 5u32;
        out.extend_from_slice(format!("xref\n0 {}\n", highest + 1).as_bytes());
        out.extend_from_slice(b"0000000000 65535 f \n");
        for num in 1..=highest {
            match offsets.get(&num) {
                Some(off) => out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes()),
                None => out.extend_from_slice(b"0000000000 65535 f \n"),
            }
        }
        out.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
                highest + 1
            )
            .as_bytes(),
        );
        out
    }

    fn extract_first_page_text(bytes: &[u8]) -> String {
        let doc = Document::from_bytes(bytes.to_vec()).unwrap();
        let pages = page_tree::pages(&doc).unwrap();
        let page =
            crate::text_extract::extract_page(&doc, &pages[0], 0, &Default::default()).unwrap();
        page.sourced_text()
    }

    #[test]
    fn reflow_edit_only_changes_edited_stream_and_reextracts() {
        let src = helvetica_pdf("BT /F1 12 Tf 72 700 Td (teh cat) Tj ET\n");
        let doc = Document::from_bytes(src.clone()).unwrap();
        let out = edit_text(
            &doc,
            &EditRequest::find_replace(0, "teh", "the"),
            &EditOptions::default(),
        )
        .unwrap();
        // Incremental save ⇒ the original file is a byte-prefix of the output
        // (every untouched object is verbatim).
        assert_eq!(out.bytes.get(..src.len()), Some(src.as_slice()));
        assert!(out.bytes.len() > src.len());
        // The edit round-trips: the corrected text extracts, the typo is gone.
        let text = extract_first_page_text(&out.bytes);
        assert!(text.contains("the cat"), "got {text:?}");
        assert!(!text.contains("teh"));
        assert_eq!(out.report.glyph_source, EditGlyphSource::NonEmbedded);
        assert!(!out.report.subset);
    }

    #[test]
    fn tm_follower_is_repositioned_by_delta() {
        // "Hello " (Td) then "World" re-anchored by an absolute Tm on the
        // same line. Editing "Hello"->"Hi" shortens the run, so the follower
        // Tm's e must decrease by |ΔA|.
        let src =
            helvetica_pdf("BT /F1 12 Tf 100 700 Td (Hello ) Tj 1 0 0 1 240 700 Tm (World) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let out = edit_text(
            &doc,
            &EditRequest::find_replace(0, "Hello", "Hi"),
            &EditOptions::default(),
        )
        .unwrap();
        assert_eq!(out.report.followers_repositioned, 1);
        assert!(out.report.advance_delta < 0.0, "shorter run ⇒ negative ΔA");
        let text = extract_first_page_text(&out.bytes);
        assert!(text.contains("Hi"));
        assert!(text.contains("World"));
    }

    #[test]
    fn missing_glyph_char_is_refused_never_written() {
        // WinAnsi Helvetica has no code for an astral char ⇒ R-INV-8 refusal.
        let src = helvetica_pdf("BT /F1 12 Tf 72 700 Td (hi) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let err = edit_text(
            &doc,
            &EditRequest::find_replace(0, "hi", "h\u{1D54F}"),
            &EditOptions::default(),
        )
        .unwrap_err();
        match err {
            EditError::Refused(r) => assert_eq!(r.trigger, RInvTrigger::BeyondRepertoire),
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn pinned_span_locates_the_exact_operator() {
        // Two identical runs; pin the SECOND by its operator span.
        let content = "BT /F1 12 Tf 72 700 Td (cat) Tj 72 680 Td (cat) Tj ET\n";
        let src = helvetica_pdf(content);
        // The second "(cat) Tj" span in the decoded buffer.
        let start = content.rfind("(cat) Tj").unwrap();
        let span = ByteSpan::new(start, "(cat) Tj".len());
        let doc = Document::from_bytes(src).unwrap();
        let mut req = EditRequest::find_replace(0, "cat", "dog");
        req.pinned_span = Some(span);
        let out = edit_text(&doc, &req, &EditOptions::default()).unwrap();
        let text = extract_first_page_text(&out.bytes);
        // First run unchanged, second edited.
        assert!(text.contains("cat"));
        assert!(text.contains("dog"));
    }

    /// **The Pass-19.3 regression fixture.** A pin taken from
    /// `GlyphProvenance::operator_span` — the operator TOKEN alone, which is
    /// what every GUI request carries — must locate the same operator as the
    /// operand-inclusive span the test above uses.
    ///
    /// Before `pin_names_operator`, this returned `NoMatch`, which meant the
    /// shipped Edit Text property bar could not apply anything at all. The
    /// test pins the SECOND of two identical runs, so "it accidentally found
    /// the right one" is not a way to pass.
    #[test]
    fn a_pin_taken_from_provenance_locates_the_same_operator() {
        let content = "BT /F1 12 Tf 72 700 Td (cat) Tj 72 680 Td (cat) Tj ET\n";
        let src = helvetica_pdf(content);
        let doc = Document::from_bytes(src).unwrap();

        // Exactly what the extraction walk publishes: `op.operator.span`.
        let tj = content.rfind("Tj").unwrap();
        let mut req = EditRequest::find_replace(0, "cat", "dog");
        req.pinned_span = Some(ByteSpan::new(tj, "Tj".len()));

        let out = edit_text(&doc, &req, &EditOptions::default()).unwrap();
        let text = extract_first_page_text(&out.bytes);
        assert!(text.contains("cat"), "the FIRST run is untouched: {text}");
        assert!(text.contains("dog"), "the SECOND run was edited: {text}");
    }

    /// …and the pin still discriminates. A span that ends inside a different
    /// operator must not silently match a neighbour — otherwise the fix would
    /// have traded a refusal for the far worse failure of editing the wrong
    /// run.
    #[test]
    fn a_pin_that_names_no_operator_still_refuses() {
        let content = "BT /F1 12 Tf 72 700 Td (cat) Tj 72 680 Td (cat) Tj ET\n";
        let src = helvetica_pdf(content);
        let doc = Document::from_bytes(src).unwrap();
        let mut req = EditRequest::find_replace(0, "cat", "dog");
        // Ends one byte short of the first `Tj`, so it names nothing.
        let tj = content.find("Tj").unwrap();
        req.pinned_span = Some(ByteSpan::new(tj, 1));
        let err = edit_text(&doc, &req, &EditOptions::default()).unwrap_err();
        assert!(matches!(err, EditError::NoMatch(_)), "{err}");
    }

    // -- Pass 19.0: the authoring walk's text-state model ---------------

    /// Run the authoring [`Walk`] over a page's content and return the
    /// recorded show operators, in stream order.
    fn show_records(content: &str) -> Vec<ShowData> {
        let src = helvetica_pdf(content);
        let doc = Document::from_bytes(src).unwrap();
        let pages = crate::page_tree::pages(&doc).unwrap();
        let page = &pages[0];
        let stream = ContentStream::from_page(&doc.view(), page).unwrap();
        let mut walk = Walk::new(&doc, &page.resources);
        for op in stream.operations() {
            walk.operation(&op, &stream.buf);
        }
        walk.recs
            .into_iter()
            .filter_map(|r| match r.rec {
                Rec::Show(s) => Some(*s),
                _ => None,
            })
            .collect()
    }

    /// The regression this slice exists to fix. Before Pass 19.0 the
    /// authoring walk had **no `b"Ts"` arm and no `b"Tr"` arm**, so an
    /// ambient rise or rendering mode was invisible to every formatting
    /// surgery — which is why pdfce could not restore one.
    #[test]
    fn the_authoring_walk_tracks_rise_and_render_mode() {
        let recs = show_records("BT /F1 12 Tf 3 Ts 2 Tr 72 700 Td (hi) Tj ET\n");
        assert_eq!(recs.len(), 1);
        let ts = &recs[0].text_state;
        assert_eq!(ts.rise.value, 3.0, "Ts was not tracked");
        assert_eq!(ts.render_mode.value, 2.0, "Tr was not tracked");
        assert_eq!(ts.restore_bytes(TextStateParam::Rise).unwrap(), b"3 Ts");
        assert_eq!(
            ts.restore_bytes(TextStateParam::RenderMode).unwrap(),
            b"2 Tr"
        );
    }

    /// §8.4.2/§9.3: text state is graphics state, so `Q` discards whatever
    /// was set since the matching `q`. The walk had no `q`/`Q` arms at all
    /// before Pass 19.0, so the second run below inherited the first run's
    /// state — and a restore built on that would have written a `3 Ts` into
    /// a stream that did not have one.
    #[test]
    fn q_and_q_restore_every_text_state_parameter() {
        let recs = show_records(
            "q 0.5 Tc 3 Ts 2 Tr 90 Tz 1 Tw BT /F1 12 Tf 72 700 Td (in) Tj ET Q \
             BT /F1 12 Tf 72 680 Td (out) Tj ET\n",
        );
        assert_eq!(recs.len(), 2);

        let inside = &recs[0].text_state;
        assert_eq!(inside.char_spacing.value, 0.5);
        assert_eq!(inside.rise.value, 3.0);
        assert_eq!(inside.render_mode.value, 2.0);
        assert_eq!(inside.h_scale.value, 90.0);
        assert_eq!(inside.word_spacing.value, 1.0);

        let outside = &recs[1].text_state;
        assert_eq!(
            *outside,
            AmbientTextState::initial(),
            "everything set inside the q … Q bracket must be discarded by the Q"
        );
        // …and therefore restores to the Table 105 defaults, not to the
        // bracket's values.
        assert_eq!(
            outside.restore_bytes(TextStateParam::Rise).unwrap(),
            b"0 Ts"
        );
        assert_eq!(
            outside.restore_bytes(TextStateParam::HorizScale).unwrap(),
            b"100 Tz"
        );
    }

    /// `Q` also restores the fill colour, which is likewise graphics state
    /// (§8.6.8) and was likewise leaking past the bracket.
    #[test]
    fn q_and_q_restore_the_fill_colour_too() {
        let recs = show_records(
            "q 1 0 0 rg BT /F1 12 Tf 72 700 Td (red) Tj ET Q \
             BT /F1 12 Tf 72 680 Td (black) Tj ET\n",
        );
        assert_eq!(recs.len(), 2);
        assert!(matches!(recs[0].fill_color, FillState::Device { .. }));
        assert_eq!(
            recs[1].fill_color,
            FillState::Default,
            "the Q must put the fill colour back to the §8.6.8 default"
        );
    }

    /// R88 tier 2, on the authoring path: the restore re-emits the operand
    /// **as written**, so a producer's `0.5000` does not come back as
    /// `0.5`. A renormalized number is a diff in bytes pdfce claims not to
    /// have logically touched.
    #[test]
    fn the_authoring_walk_keeps_raw_operand_bytes_for_restore() {
        let recs = show_records("BT /F1 12 Tf 0.5000 Tc 72 700 Td (hi) Tj ET\n");
        let ts = &recs[0].text_state;
        assert_eq!(ts.char_spacing.value, 0.5);
        assert_eq!(
            ts.restore_bytes(TextStateParam::CharSpacing).unwrap(),
            b"0.5000 Tc"
        );
    }

    /// An unbalanced `Q` must not pop state the stream never pushed, and
    /// must not panic — §7.8.2's recovery posture, and the same guard the
    /// extraction walk has always had.
    #[test]
    fn an_unbalanced_q_is_survivable() {
        let recs = show_records("Q Q 0.5 Tc BT /F1 12 Tf 72 700 Td (hi) Tj ET\n");
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].text_state.char_spacing.value, 0.5);
    }
}
