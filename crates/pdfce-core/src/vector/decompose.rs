//! # Vector object decomposition (ISO 32000-1 §8.5 paths, §8.2 operators)
//!
//! The **read-only decomposition** that turns a page's lossless
//! content-token stream ([`crate::content::ContentStream`]) into a list of
//! selectable [`VectorObject`]s, per
//! `docs/decisions/011-first-beta-scaled-measurement-dimensioning-tool.md`
//! §2.1. It **indexes** the token model; it never rewrites it. Pass 9a is
//! **byte-inert** (R46): running this over the corpus changes zero output
//! bytes — proven by re-running the content-identity gate.
//!
//! ## What an object is (§8.5.2–§8.5.3, Table 59/60)
//!
//! Walking the token stream while tracking graphics state (CTM via
//! `q`/`Q`/`cm`, line width, device colour), a **path object** is the run
//! of path-construction operators (`m l c v y re h`) terminating in a
//! **painting operator** (`S s f F f* B B* b b* n`). Each object captures,
//! per decision 011 §2.1:
//!
//! 1. its subpaths as **node lists** ([`Subpath`] of anchors + Bézier
//!    control points) in **user space**, plus the effective **CTM** that
//!    maps them to **page space** (for hit-test/snap — see
//!    [`PathObject::page_subpaths`]);
//! 2. the **effective graphics state** at paint time (CTM captured at the
//!    path's *first* construction op — the same rule the renderer uses;
//!    line width; fill/stroke colour);
//! 3. the **content-token index range** ([`TokenRange`]) and the
//!    equivalent [`ByteSpan`] of its defining operators — the handle a
//!    later editing Pass (9c-min) maps back to the Pass 8.0 surgery
//!    interpreter. **9a captures it; it does not use it.**
//!
//! **Text objects** (`BT`…`ET`) and **image objects** (`Do` on an image
//! XObject, an inline `BI`/`ID`/`EI`, or a form `Do`) are decomposed as
//! **selectable-for-move/delete** objects — not node-editable in the beta
//! (dimensioning cares about path geometry) — carrying their bbox, their
//! token range, and the small **identifying detail** a human needs to tell
//! one from another: a text object's shown string and font
//! ([`TextPreview`], [`TextFont`]), an image object's pixel dimensions
//! ([`ImageObject::pixel_size`]). A text object's bbox is a documented
//! **approximation** (see [`TextObject`]).
//!
//! ## Identifying detail, and the two rules it obeys
//!
//! ui-spec `pass-17-dock-and-layer-tree.md` §B.4 asks for exactly this: an
//! object list that can only say `Text` three times is not a troubleshooting
//! tool. Two rules bind how it is produced:
//!
//! 1. **One decoder, never two.** Show-operator strings are decoded through
//!    [`crate::text_extract::ExtractFont`] — the same §9.10.2 ladder
//!    `extract-text` climbs — reached through the [`FontResolver`] seam.
//!    A second, simpler decoder here would disagree with `extract-text` on
//!    exactly the fonts that are hard, which is the decision 011 §Z2
//!    divergence shape one layer up from geometry.
//! 2. **Never invent a value that cannot be justified.** When the ladder
//!    recovers nothing for a text object, the preview is
//!    [`TextPreview::Undecodable`], **not** a string of mojibake; when the
//!    caller supplied no font resolver at all, it is
//!    [`TextPreview::Unavailable`], which is a different fact and says so.
//!    Rule 4 (fuzzy, never sneaky) applied where the operator is least able
//!    to catch a fabrication.
//!
//! ### Bounded memory (the 50k-object page)
//!
//! [`PageObjects`] now carries owned `String`s, so the cost is capped **at
//! decomposition**, not at display: a preview is cut at
//! [`MAX_TEXT_PREVIEW_CHARS`] characters and the decode loop *stops there*
//! (a 10 kB show string is not decoded and then thrown away), and font
//! names are cut at [`MAX_FONT_NAME_BYTES`]. Worst case per text object is
//! therefore ~256 B of preview (64 chars × 4 bytes for astral code points)
//! plus two ≤64 B names plus their `String` headers — under ~450 B. A
//! hostile page of 50,000 text objects costs ≈22 MB of preview at the
//! absolute worst and ≈5 MB for realistic Latin text, against the
//! [`MAX_OBJECTS`] ceiling of 1,000,000 objects that already bounds the
//! object list itself. Truncation is **disclosed**
//! ([`TextPreview::Decoded::truncated`]), never silent.
//!
//! ## Agreement with the renderer (the Z2 risk, decision 011)
//!
//! The construction rules here mirror `pdfce-render`'s interpreter
//! exactly — the `v`/`y` implicit-control-point traps and the `re`
//! expansion go through the SHARED primitives in
//! [`crate::vector::geometry`] (`cubic_from_v`/`cubic_from_y`/
//! `rect_corners`) that the renderer also calls, and the CTM update uses
//! the same `post_concat` composition. A cross-check acceptance test in
//! `pdfce-render` compares the full page-space geometry the two pipelines
//! produce on the vector fixtures, so a divergence is caught by a test,
//! not by a mis-rendered dimension.
//!
//! ## Panic-free / adversarial input (ARCHITECTURE.md §10)
//!
//! Every operand access is checked; every degenerate shape (missing
//! current point, unbalanced `q`/`Q`, mid-path `cm`, non-finite operands,
//! huge node counts) is tolerated and **counted** in
//! [`DecomposeDiagnostics`] rather than panicking — the same
//! "fuzzy, never sneaky" posture the renderer takes. A fuzz target
//! (`fuzz/fuzz_targets/vector_decompose.rs`) drives exactly these shapes.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use crate::content::{ContentStream, ContentToken, ContentTokenKind};
use crate::graph::ObjectGraph;
use crate::object::{Dict, Object};
use crate::page_tree::Page;
use crate::span::ByteSpan;
use crate::text_extract::{ExtractFont, LadderRung};
use crate::view::DocumentView;

use super::geometry::{Bounds, Matrix, Point, Rgb, cubic_from_v, cubic_from_y, rect_corners};

/// Guard on the number of objects a single page can decompose to
/// (ARCHITECTURE.md §10 adversarial-input posture). A legitimate complex
/// vector drawing has thousands of path objects; a hostile stream that is
/// nothing but paint operators would otherwise allocate without bound.
/// 1,000,000 is far above any real page and still cheap to reject past.
pub const MAX_OBJECTS: usize = 1_000_000;

/// Guard on the total number of path nodes retained across one page, for
/// the same reason as [`MAX_OBJECTS`]: a stream of a million `l` operators
/// is a memory-amplification vector. Past this bound, further construction
/// operators are counted-and-dropped (the object still terminates and is
/// emitted with the nodes it has).
pub const MAX_NODES: usize = 4_000_000;

/// How many decoded characters of a text object's shown string
/// [`TextPreview::Decoded`] retains.
///
/// A *preview*, not the text: the consumer is a one-line object row and a
/// one-line status readout, both of which elide well before this. The
/// number is set here rather than at display time because it is the
/// **memory bound** (module docs' "Bounded memory") — a page of 50,000 text
/// objects must not be able to make the object model larger than the file
/// it came from. Callers that want a page's actual text call
/// [`crate::text_extract::extract_page`], which is the pipeline for that
/// question and streams rather than retaining.
///
/// 64 is chosen to comfortably contain a caption, a dimension label or a
/// short heading — the strings that make a row identifiable — while a
/// paragraph-sized run is cut and **says** it was cut.
pub const MAX_TEXT_PREVIEW_CHARS: usize = 64;

/// Byte ceiling on a retained font name ([`TextFont::resource`] and
/// [`TextFont::base_font`]).
///
/// A `/BaseFont` is a PDF name, and §7.3.5 caps a name at 127 bytes in
/// practice; a hostile file can nevertheless carry a long one on every one
/// of a page's text objects. Cutting at 64 bytes keeps the per-object cost
/// bounded without touching any real font name (`ABCDEF+Helvetica-BoldOblique`
/// is 28). Truncation is on a UTF-8 character boundary, so the result is
/// always valid text.
pub const MAX_FONT_NAME_BYTES: usize = 64;

/// One segment of a subpath, in the coordinate space of its [`Subpath`]
/// (user space as stored; page space after [`PathObject::page_subpaths`]).
///
/// A subpath is a start anchor followed by a list of these. A straight
/// `l`/`re`-edge is a [`Segment::Line`]; every `c`/`v`/`y` cubic is a
/// [`Segment::Cubic`] with its two control points made explicit (the
/// `v`/`y` implicit points already resolved by the shared primitives, so a
/// consumer never has to re-derive them).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Segment {
    /// A straight line to `to`.
    Line {
        /// The segment's end anchor.
        to: Point,
    },
    /// A cubic Bézier with control points `c1`, `c2` ending at `to`.
    Cubic {
        /// First control point.
        c1: Point,
        /// Second control point.
        c2: Point,
        /// The segment's end anchor.
        to: Point,
    },
}

impl Segment {
    /// The segment's end anchor (the on-curve point a following segment
    /// starts from).
    #[must_use]
    pub fn end(self) -> Point {
        match self {
            Segment::Line { to } | Segment::Cubic { to, .. } => to,
        }
    }

    /// Map every point of this segment through `m` (user → page space).
    #[must_use]
    pub fn transformed(self, m: Matrix) -> Self {
        match self {
            Segment::Line { to } => Segment::Line {
                to: m.map_point(to),
            },
            Segment::Cubic { c1, c2, to } => Segment::Cubic {
                c1: m.map_point(c1),
                c2: m.map_point(c2),
                to: m.map_point(to),
            },
        }
    }
}

/// One subpath: a start anchor, its segments, and whether it was closed
/// (`h`, `re`, or a close-and-paint operator such as `s`/`b`).
#[derive(Debug, Clone, PartialEq)]
pub struct Subpath {
    /// The subpath's first on-curve anchor (`m`, or the implicit reopen
    /// after `h`).
    pub start: Point,
    /// The segments after the start, in order.
    pub segments: Vec<Segment>,
    /// Whether the subpath is closed (a closing edge back to `start`).
    pub closed: bool,
}

impl Subpath {
    /// Map every node of this subpath through `m` (user → page space).
    #[must_use]
    pub fn transformed(&self, m: Matrix) -> Self {
        Self {
            start: m.map_point(self.start),
            segments: self.segments.iter().map(|s| s.transformed(m)).collect(),
            closed: self.closed,
        }
    }

    /// The on-curve anchor points of this subpath (start + each segment
    /// end), in order — the snap/hit vertices dimensioning cares about.
    /// Control points are excluded (a snap target is an anchor, not a
    /// handle — Bézier handle editing is a fast-follow).
    pub fn anchors(&self) -> impl Iterator<Item = Point> + '_ {
        std::iter::once(self.start).chain(self.segments.iter().map(|s| s.end()))
    }
}

/// The painting disposition of a [`PathObject`] (§8.5.3, Table 60).
///
/// `fill` is `Some(rule)` for the fill operators (`f`/`F`/`B`/`b` →
/// [`FillRule::NonZero`]; `f*`/`B*`/`b*` → [`FillRule::EvenOdd`]);
/// `stroke` is true for the stroke operators (`S`/`s`/`B…`/`b…`). Both
/// false is the `n` no-op / clip-only path (§8.5.4): geometry that paints
/// nothing yet is still a selectable object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaintStyle {
    /// The fill winding rule, if the object is filled.
    pub fill: Option<FillRule>,
    /// Whether the object is stroked.
    pub stroke: bool,
}

impl PaintStyle {
    /// Whether the object paints nothing (`n` — a clip or bare end-path).
    #[must_use]
    pub fn is_invisible(self) -> bool {
        self.fill.is_none() && !self.stroke
    }
}

/// A path fill winding rule (§8.5.3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillRule {
    /// Nonzero winding (`f`, `F`, `B`, `b`).
    NonZero,
    /// Even-odd (`f*`, `B*`, `b*`).
    EvenOdd,
}

/// The half-open range of content-token indices `[start, end)` that a
/// decomposed object's **defining operators** occupy in
/// [`ContentStream::tokens`] — from the first construction operator's
/// first token through the painting operator (or `BT`→`ET`, or the `Do`
/// operation). The editing handle 9c-min will map to the Pass 8.0 surgery
/// interpreter; **9a captures it and does not use it**.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenRange {
    /// Index of the first defining token (inclusive).
    pub start: usize,
    /// Index one past the last defining token (exclusive) — i.e. the
    /// painting/`ET`/`Do` operator index plus one.
    pub end: usize,
}

impl TokenRange {
    /// The range as a [`std::ops::Range`] for slicing
    /// [`ContentStream::tokens`].
    #[must_use]
    pub fn as_range(self) -> std::ops::Range<usize> {
        self.start..self.end
    }
}

/// A path object — the node-editable heart of the model (module docs).
#[derive(Debug, Clone, PartialEq)]
pub struct PathObject {
    /// The subpaths in **user space** (decision 011 §2.1 item 1). Map them
    /// to page space via [`PathObject::page_subpaths`].
    pub subpaths: Vec<Subpath>,
    /// The effective CTM captured at the path's **first** construction op
    /// (the same rule the renderer's `path_ctm` uses), mapping user → page
    /// space.
    pub ctm: Matrix,
    /// Fill/stroke disposition at paint time.
    pub style: PaintStyle,
    /// Line width in **user-space** units at paint time (§8.4.3.2) — what a
    /// stroke-proximity hit-test widens by.
    pub line_width: f64,
    /// Non-stroking (fill) colour at paint time.
    pub fill_color: Rgb,
    /// Stroking colour at paint time.
    pub stroke_color: Rgb,
    /// The defining-operator token range (the future editing handle).
    pub tokens: TokenRange,
    /// The equivalent byte span in the decoded content buffer.
    pub bytes: ByteSpan,
    /// Precomputed **page-space** bounds (control-point hull — a
    /// conservative superset of the exact curve bounds), for the hit-test
    /// bbox pre-filter and marquee enclosure.
    pub page_bbox: Bounds,
}

impl PathObject {
    /// The subpaths mapped into **page space** by [`PathObject::ctm`] —
    /// what the hit-test, snapping engine (12.M1) and centerline
    /// derivation consume (decision 011 §2.1 item 1).
    #[must_use]
    pub fn page_subpaths(&self) -> Vec<Subpath> {
        self.subpaths
            .iter()
            .map(|s| s.transformed(self.ctm))
            .collect()
    }

    /// Whether the object is exactly one closed 4-anchor quad (an `re`
    /// rectangle or a hand-drawn 4-line closed quad) — the shape the
    /// filled-line centerline derivation (`super::centerline`) inspects.
    #[must_use]
    pub fn is_quad(&self) -> bool {
        matches!(self.subpaths.as_slice(), [only] if only.closed && subpath_is_quad(only))
    }
}

/// Whether the source of an image object was an inline image, an image
/// XObject, or a form XObject (§8.8/§8.9.7). Recorded for disclosure; all
/// three are bbox-selectable and none is node-editable in the beta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageSource {
    /// A `BI`/`ID`/`EI` inline image (§8.9.7).
    Inline,
    /// A `Do` on an image XObject (§8.9).
    XObject,
    /// A `Do` on a form XObject (§8.10) — treated as one opaque
    /// selectable object bounded by its `/BBox`; 9a does NOT recurse into
    /// the form's own content (per-form path decomposition is a
    /// fast-follow).
    Form,
}

/// An image/form object — selectable-for-move/delete, bbox only (module
/// docs). Its page bbox is the image unit square (§8.9.4) or the form
/// `/BBox` (§8.10.1) mapped by the effective transform.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageObject {
    /// The effective CTM at the `Do`/inline-image operator.
    pub ctm: Matrix,
    /// Page-space bounds (unit square or form `/BBox` under the transform).
    pub page_bbox: Bounds,
    /// Where the object came from.
    pub source: ImageSource,
    /// The image's size in **samples** — `(width, height)` from the image
    /// dictionary's `/Width` and `/Height` (ISO 32000-1 §8.9.5, Table 89:
    /// both **required** integers, "width/height … in samples"), or the
    /// inline image's normalized `/W`/`/H` (§8.9.7, Table 93).
    ///
    /// **This is a sample count, not a size on the page.** §8.9.5's own
    /// note is blunt about it: an image occupies the user-space unit square
    /// under the CTM, so its printed size comes from the CTM and has no
    /// fixed relationship to these numbers. `640×480` in a row answers
    /// "which image is this?" (and, against [`page_bbox`](Self::page_bbox),
    /// "at what effective resolution is it placed?"); it never answers "how
    /// big is it on the paper".
    ///
    /// `None` for a form XObject (§8.10 — a form has no samples, it has a
    /// `/BBox`), and for a malformed image whose `/Width`/`/Height` are
    /// absent, non-integer, negative or larger than `u32`. A missing value
    /// is reported as missing rather than guessed at from the CTM, which
    /// would be a fabricated number the operator could not check.
    pub pixel_size: Option<(u32, u32)>,
    /// The defining-operator token range.
    pub tokens: TokenRange,
    /// The equivalent byte span.
    pub bytes: ByteSpan,
}

/// The font in effect at a text object's first show operator.
///
/// Captured at the FIRST `Tj`/`TJ`/`'`/`"` rather than at `ET`, because a
/// text object that switches font mid-run should be identified by the font
/// its visible run *starts* in — the same run [`TextPreview`] previews. A
/// text object that never shows anything has no font (there is no
/// operand to report), and one that shows without a preceding `Tf` has an
/// empty [`resource`](Self::resource) recorded as `None` rather than as `""`.
#[derive(Debug, Clone, PartialEq)]
pub struct TextFont {
    /// The `/Tf` **resource name** as written in the content stream (`F1`,
    /// `TT2`), decoded from the name's bytes with invalid UTF-8 replaced,
    /// and cut at [`MAX_FONT_NAME_BYTES`].
    ///
    /// This is the key into the page's `/Font` resource dictionary — the
    /// handle a future edit needs — not a typeface name. Prefer
    /// [`base_font`](Self::base_font) when showing a human which typeface
    /// they are looking at.
    pub resource: String,
    /// `/BaseFont` from the resolved font dictionary (§9.6.2.1 Table 111 /
    /// §9.7.4 Table 120), subset tag included (`ABCDEF+Helvetica`), cut at
    /// [`MAX_FONT_NAME_BYTES`].
    ///
    /// `None` when no [`FontResolver`] was supplied, or the name is not in
    /// the resource dictionary, or the font dictionary carries no
    /// `/BaseFont`. Never synthesised from the resource name — `F1` is not
    /// evidence of any typeface.
    pub base_font: Option<String>,
    /// The **`Tf` size operand** in effect (§9.3.1, Table 105 `Tfs`).
    ///
    /// A *text-space* quantity, reported exactly as the file states it and
    /// **not** scaled by the text matrix or the CTM. A file that writes
    /// `/F1 1 Tf` and then `12 0 0 12 x y Tm` renders 12 pt type and is
    /// reported here as `1` — which is what the file says. Folding the
    /// matrices in would produce a confident number that disagrees with the
    /// operand an operator would find in the content stream, and pdfce has
    /// no glyph metrics with which to defend a measured alternative
    /// (see [`TextObject`]'s bbox note for the same limitation).
    pub size: f64,
}

/// What a text object's shown string decoded to — or, honestly, why it did
/// not (module docs' rule 2).
///
/// Four distinguishable answers, because "no text" has four genuinely
/// different causes and collapsing them into `Option<String>` would tell the
/// operator that the file is empty when in fact pdfce declined to guess.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextPreview {
    /// Strings were shown, but **no decoder was in scope**, so no decoding
    /// was attempted: either the caller supplied no [`FontResolver`] (what
    /// plain [`decompose`] does), or the `Tf` name did not resolve to a font
    /// in the page's `/Font` resources.
    ///
    /// A property of the LOOKUP, never of the document's text. The
    /// distinction from [`TextPreview::Empty`] matters: one says pdfce did
    /// not look, the other says there was nothing to find.
    Unavailable,
    /// Decoding ran and produced characters.
    Decoded {
        /// The decoded characters, at most [`MAX_TEXT_PREVIEW_CHARS`] of
        /// them.
        ///
        /// Sourced characters only: the codes are mapped through the
        /// §9.10.2 ladder and concatenated **verbatim**, with none of
        /// `text_extract`'s derived inter-word spacing or line breaking
        /// (§14.8.2.5 S2/S3/S5 — a content stream carries no word or line
        /// signal, and inventing one in a row label would be a guess the
        /// operator cannot review). A `TJ` array's kerning offsets are
        /// therefore invisible here, exactly as
        /// [`ExtractedText::sourced_text`](crate::text_extract::ExtractedText::sourced_text)
        /// treats them.
        text: String,
        /// Whether the shown string ran past [`MAX_TEXT_PREVIEW_CHARS`] and
        /// was cut. Disclosed so a display can mark the elision rather than
        /// silently present a prefix as the whole string.
        truncated: bool,
        /// Whether **some** codes in the decoded prefix defeated the ladder
        /// and were emitted as U+FFFD (`LadderRung::Failed`).
        ///
        /// The replacement characters are left in `text` — that is
        /// `text_extract`'s own disclosed policy for an unmappable code —
        /// and this flag is what lets a consumer say so in words instead of
        /// leaving the operator to interpret a row full of `�`.
        lossy: bool,
    },
    /// Codes were shown and **not one of them** could be mapped to a
    /// character: every code reached §9.10.2's failure clause ("there is no
    /// way to determine what the character code represents"), the canonical
    /// case being `Identity-H` with no `/ToUnicode`.
    ///
    /// A distinct variant rather than `Decoded { text: "���" }` because the
    /// honest answer is *"this text cannot be read"*, and a row of
    /// replacement characters looks instead like a pdfce bug.
    Undecodable,
    /// The text object showed no strings at all — a `BT`/`ET` that only
    /// positioned, or whose show operands were not strings.
    Empty,
}

/// A text object (`BT`…`ET`) — selectable-for-move/delete, with an
/// identifying preview.
///
/// **The bbox is a deliberate approximation.** `pdfce-core` has no glyph
/// metrics (font programs and widths live behind the `pdfce-render`
/// loader), so the bbox is the bounding box of the text-showing **origins**
/// (each `Tj`/`TJ`/`'`/`"` pen position, mapped to page space) inflated by
/// the largest `Tf` size seen — a coarse em box. It reliably covers the
/// text's start points (so the object is selectable) but does not measure
/// the exact horizontal extent, and it does not fold a scaling `Tm` into
/// the inflation. [`TextObject::approximate`] is always `true` to disclose
/// this (fuzzy, never sneaky); an exact text bbox is a fast-follow once the
/// dimensioning subsystem needs to snap to glyph geometry.
///
/// **The preview and font are identity, not content.** They exist so an
/// object row can say `Text · "Section A-A" · Helvetica 10` instead of
/// `Text` three times over (ui-spec §B.4 #1); they are capped, they carry no
/// positions, and they are not a text-extraction result. Anything that needs
/// the page's actual text — with provenance, derived spacing and reading
/// order — calls [`crate::text_extract::extract_page`].
#[derive(Debug, Clone, PartialEq)]
pub struct TextObject {
    /// Page-space approximate bounds (module docs).
    pub page_bbox: Bounds,
    /// Always `true` — the bbox is an origin-derived approximation.
    pub approximate: bool,
    /// What the object's shown strings decoded to (or why they did not).
    pub preview: TextPreview,
    /// The font in effect at the first show operator, if there was one.
    pub font: Option<TextFont>,
    /// The `BT`→`ET` token range.
    pub tokens: TokenRange,
    /// The equivalent byte span.
    pub bytes: ByteSpan,
}

/// A selectable object on a page — the unit the GUI target provider hands
/// back as a hit and the snapping engine (12.M1) consumes.
#[derive(Debug, Clone, PartialEq)]
pub enum VectorObject {
    /// A path object (node-editable in a later Pass).
    Path(PathObject),
    /// A text object (selectable, not node-editable).
    Text(TextObject),
    /// An image or form object (selectable, not node-editable).
    Image(ImageObject),
}

impl VectorObject {
    /// The object's page-space bounding box — the marquee-enclosure and
    /// hit-test pre-filter input for every object kind.
    #[must_use]
    pub fn page_bbox(&self) -> Bounds {
        match self {
            VectorObject::Path(p) => p.page_bbox,
            VectorObject::Text(t) => t.page_bbox,
            VectorObject::Image(i) => i.page_bbox,
        }
    }

    /// The object's defining-operator token range (the future editing
    /// handle for a path; the move/delete handle for text/image).
    #[must_use]
    pub fn tokens(&self) -> TokenRange {
        match self {
            VectorObject::Path(p) => p.tokens,
            VectorObject::Text(t) => t.tokens,
            VectorObject::Image(i) => i.tokens,
        }
    }

    /// The object's byte span in the decoded content buffer.
    #[must_use]
    pub fn bytes(&self) -> ByteSpan {
        match self {
            VectorObject::Path(p) => p.bytes,
            VectorObject::Text(t) => t.bytes,
            VectorObject::Image(i) => i.bytes,
        }
    }
}

/// Structural oddities tolerated during decomposition, counted rather than
/// silently absorbed — the object-model twin of the renderer's
/// [`crate::content`] diagnostics. Every count answers a "how honest is
/// this decomposition?" question.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DecomposeDiagnostics {
    /// Path objects emitted.
    pub paths: usize,
    /// Text objects emitted.
    pub text: usize,
    /// Image objects (inline + image XObject) emitted.
    pub images: usize,
    /// Form objects emitted (`Do` on a form XObject).
    pub forms: usize,
    /// `cm` seen mid-path-construction (the geometry is approximated with
    /// the first captured CTM, exactly as the renderer does).
    pub midpath_cm: usize,
    /// Unbalanced `Q` (empty graphics-state stack) tolerated.
    pub unbalanced_q: usize,
    /// A segment operator (`l`/`c`/`v`/`y`) with no current point — a
    /// §8.5.2.1 error, skipped.
    pub segment_without_current: usize,
    /// A `Do`/inline-image whose XObject could not be classified (no
    /// resolver, or an unresolvable name) — no object emitted.
    pub unresolved_xobject: usize,
    /// Construction operators dropped because [`MAX_NODES`] was hit.
    pub nodes_dropped: usize,
    /// Objects dropped because [`MAX_OBJECTS`] was hit.
    pub objects_dropped: usize,
}

/// The decomposition of one page (or one content stream): the ordered
/// object list plus its diagnostics.
///
/// Objects are in **paint order** — the order the renderer would paint
/// them, so the LAST object at a point is the topmost. [`page_bbox`] and
/// hit-testing rely on this ordering.
#[derive(Debug, Clone, PartialEq)]
pub struct PageObjects {
    /// The objects, in paint order.
    pub objects: Vec<VectorObject>,
    /// The initial CTM the stream was decomposed under (identity for a
    /// page — page-space geometry is then genuine PDF user space).
    pub initial: Matrix,
    /// Tolerated-oddity counts (module docs).
    pub diagnostics: DecomposeDiagnostics,
}

impl PageObjects {
    /// The union of every object's page bbox — the page's drawn extent in
    /// page space (empty if the page has no vector content).
    #[must_use]
    pub fn page_bbox(&self) -> Bounds {
        self.objects
            .iter()
            .fold(Bounds::EMPTY, |acc, o| acc.union(o.page_bbox()))
    }
}

// ---------------------------------------------------------------------------
// XObject classification seam (keeps `decompose` testable without a Document)
// ---------------------------------------------------------------------------

/// The classification of an XObject named by a `Do` operator, enough to
/// bound it without recursing (§8.8 Table 87).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum XObjectShape {
    /// An image XObject (§8.9): bounded by the user-space unit square under
    /// the current CTM.
    Image {
        /// `(/Width, /Height)` in samples (§8.9.5, Table 89), or `None` if
        /// the dictionary does not carry a usable pair — see
        /// [`ImageObject::pixel_size`], which this becomes.
        pixel_size: Option<(u32, u32)>,
    },
    /// A form XObject (§8.10): bounded by its `/BBox` (in form space) under
    /// `matrix × ctm`.
    Form {
        /// The form's `/BBox`, normalized, in form space.
        bbox: Bounds,
        /// The form's `/Matrix` (default identity).
        matrix: Matrix,
    },
}

/// The seam [`decompose`] uses to classify a `Do` operator's XObject.
///
/// Split out as a trait so the decomposition is testable with a stub (or
/// [`NoXObjects`]) and drivable by the fuzz target **without constructing a
/// [`DocumentView`]** — the heavy dependency only the `Do`-resolution path
/// needs. Real callers use [`DocumentXObjects`].
pub trait XObjectResolver {
    /// Classify the XObject named `name` in the current resource
    /// dictionary, or `None` if it cannot be resolved (absent, not a
    /// stream, no `/Subtype`).
    fn classify(&self, name: &[u8]) -> Option<XObjectShape>;
}

/// A resolver that classifies nothing — for content with no XObjects, and
/// for unit tests / the fuzz target that exercise the path/text walk
/// without a [`DocumentView`].
#[derive(Debug, Default, Clone, Copy)]
pub struct NoXObjects;

impl XObjectResolver for NoXObjects {
    fn classify(&self, _name: &[u8]) -> Option<XObjectShape> {
        None
    }
}

/// The production resolver: classifies a `Do` name against a page's
/// resolved `/XObject` subdictionary in a [`DocumentView`] (§7.8.3, §8.8).
///
/// Takes a view rather than a `&Document` (decision 018) so that an
/// XObject *created this session* — a dimension's or markup annotation's
/// form XObject, an image inserted by a future Pass — classifies as the
/// object model's caller sees it. A base-only resolver would decline to
/// classify it, and the object would silently vanish from selection and
/// snapping while remaining visible on the canvas: the two-views-disagree
/// failure decision 011 §Z2 warns against.
///
/// The field is named `view` rather than `doc` on purpose: the rename makes
/// every construction site a compile error rather than a silent
/// type-inference success, which is how the base-vs-session intent of each
/// one got audited when this changed.
pub struct DocumentXObjects<'a> {
    /// The document view, for resolving indirect `/XObject` entries against
    /// whichever revision the caller means.
    pub view: &'a DocumentView<'a>,
    /// The resource dictionary the `Do` name is looked up in.
    pub resources: &'a Dict,
}

impl XObjectResolver for DocumentXObjects<'_> {
    fn classify(&self, name: &[u8]) -> Option<XObjectShape> {
        let entry = self
            .resources
            .get(b"XObject")
            .map(|o| self.view.resolve(o))
            .and_then(Object::as_dict)?
            .get(name)?;
        let Object::Stream(stream) = self.view.resolve(entry) else {
            return None;
        };
        let subtype = stream
            .dict
            .get(b"Subtype")
            .map(|o| self.view.resolve(o))
            .and_then(Object::as_name)
            .map(|n| n.as_bytes());
        match subtype {
            Some(b"Image") => Some(XObjectShape::Image {
                pixel_size: dict_pixel_size(self.view, &stream.dict),
            }),
            Some(b"Form") => Some(XObjectShape::Form {
                bbox: dict_rect(self.view, &stream.dict, b"BBox").unwrap_or(Bounds::EMPTY),
                matrix: dict_matrix(self.view, &stream.dict).unwrap_or(Matrix::IDENTITY),
            }),
            // Structural inference for a malformed missing /Subtype, matching
            // the renderer's Width+Height ⇒ image, BBox ⇒ form heuristic.
            _ => {
                if stream.dict.contains_key(b"Width") && stream.dict.contains_key(b"Height") {
                    Some(XObjectShape::Image {
                        pixel_size: dict_pixel_size(self.view, &stream.dict),
                    })
                } else if stream.dict.contains_key(b"BBox") {
                    Some(XObjectShape::Form {
                        bbox: dict_rect(self.view, &stream.dict, b"BBox").unwrap_or(Bounds::EMPTY),
                        matrix: dict_matrix(self.view, &stream.dict).unwrap_or(Matrix::IDENTITY),
                    })
                } else {
                    None
                }
            }
        }
    }
}

/// Read a four-number rectangle entry, normalized per §7.9.5, as a
/// [`Bounds`] in the dictionary's own space.
fn dict_rect(view: &DocumentView<'_>, dict: &Dict, key: &[u8]) -> Option<Bounds> {
    let items = view.resolve(dict.get(key)?).as_array()?;
    let n: Vec<f64> = items
        .iter()
        .filter_map(|o| view.resolve(o).as_number())
        .collect();
    let [x0, y0, x1, y1] = <[f64; 4]>::try_from(n).ok()?;
    Some(Bounds {
        min: Point::new(x0.min(x1), y0.min(y1)),
        max: Point::new(x0.max(x1), y0.max(y1)),
    })
}

/// Read an image dictionary's `(/Width, /Height)` sample counts (§8.9.5,
/// Table 89 — both **required** integers), resolving indirect entries
/// through `view`.
///
/// `None` unless BOTH are present, integral and fit `u32`. Deliberately
/// strict, and deliberately all-or-nothing:
///
/// - A real entry is an integer. A `/Width 640.5` is malformed, and
///   rounding it would report a sample count no decoder would agree with.
/// - `/Width` and `/Height` are, in §8.9.5's own words about the resource
///   ceiling, attacker-controlled integers. A negative or `> u32::MAX`
///   value cannot be a sample count; reporting `Some` for it would put a
///   nonsense number in front of the operator.
/// - Reporting one axis without the other (`640×?`) is less useful than
///   reporting neither and saying so.
fn dict_pixel_size(view: &DocumentView<'_>, dict: &Dict) -> Option<(u32, u32)> {
    let read =
        |key: &[u8]| -> Option<u32> { u32::try_from(view.resolve(dict.get(key)?).as_int()?).ok() };
    Some((read(b"Width")?, read(b"Height")?))
}

/// The same read for an **inline** image's parameter dictionary (§8.9.7).
///
/// A separate function only because inline-image parameters are direct
/// objects (§8.9.7: the dictionary sits between `BI` and `ID` in the
/// content stream, so it has nowhere to hold an indirect reference), which
/// means there is no [`DocumentView`] to resolve through — and the
/// decomposition of an inline image must work without a document at all
/// (the [`NoXObjects`] / fuzz path). [`crate::content`] has already
/// normalized the Table 93 abbreviations `/W` and `/H` to `/Width` and
/// `/Height`, so this reads the same two keys as [`dict_pixel_size`].
fn inline_pixel_size(params: &Dict) -> Option<(u32, u32)> {
    let read = |key: &[u8]| -> Option<u32> { u32::try_from(params.get(key)?.as_int()?).ok() };
    Some((read(b"Width")?, read(b"Height")?))
}

/// Read a six-number `/Matrix` entry (Table 95) as a [`Matrix`].
fn dict_matrix(view: &DocumentView<'_>, dict: &Dict) -> Option<Matrix> {
    let items = view.resolve(dict.get(b"Matrix")?).as_array()?;
    let n: Vec<f64> = items.iter().filter_map(Object::as_number).collect();
    let [a, b, c, d, e, f] = <[f64; 6]>::try_from(n).ok()?;
    Some(Matrix::new(a, b, c, d, e, f))
}

// ---------------------------------------------------------------------------
// Font classification seam (the ONE decoder, reached without a Document)
// ---------------------------------------------------------------------------

/// The seam the decomposition uses to turn a `Tf` resource name into a
/// decoder for that font's show strings.
///
/// The exact twin of [`XObjectResolver`], and split out for the same three
/// reasons: the walk stays drivable with no [`DocumentView`] at all (unit
/// tests, the fuzz target), the *policy* of which revision a font is looked
/// up in belongs to the caller (decision 018 — a session view sees a font
/// added this session, a base view does not), and a caller that does not
/// care about text detail pays nothing.
///
/// The returned value is a [`crate::text_extract::ExtractFont`] — the
/// §9.10.2 ladder `extract-text` climbs — and **not** a bespoke encoding
/// table, so the object row and `extract-text` cannot disagree about what a
/// byte means (module docs' rule 1).
///
/// [`Arc`] rather than a borrow: one font resource is typically named by
/// many text objects on a page, and resolving a `/ToUnicode` CMap per
/// `Tf` would turn a linear walk quadratic. Implementations are expected to
/// cache — [`DocumentFonts`] does.
pub trait FontResolver {
    /// Resolve the font named `name` in the current resource dictionary,
    /// or `None` if it cannot be resolved (absent `/Font` dictionary, name
    /// not present, entry not a dictionary).
    fn resolve(&self, name: &[u8]) -> Option<Arc<ExtractFont>>;
}

/// A resolver that resolves nothing — the default, and what plain
/// [`decompose`] passes.
///
/// Every text object it produces carries [`TextPreview::Unavailable`],
/// which says *"no decoding was attempted"* rather than *"this object has
/// no text"*. The distinction is the whole point of having a named unit
/// struct here instead of an `Option`.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoFonts;

impl FontResolver for NoFonts {
    fn resolve(&self, _name: &[u8]) -> Option<Arc<ExtractFont>> {
        None
    }
}

/// The production font resolver: resolves a `Tf` name against a page's
/// `/Font` resource subdictionary (§7.8.3 Table 33, §9.6.2.1) in a
/// [`DocumentView`], memoizing each resolution.
///
/// ## Why the cache is not optional
///
/// [`ExtractFont::resolve`] parses a `/ToUnicode` CMap stream and builds a
/// 256-entry encoding table. A page that sets `/F1 10 Tf` inside every one
/// of a thousand `BT`/`ET` blocks — which is what a word processor emits —
/// would pay that a thousand times. The cache turns the walk back into one
/// resolution per distinct font resource per page.
///
/// [`RefCell`] because [`FontResolver::resolve`] takes `&self` (the walk
/// holds the resolver immutably, exactly as it holds
/// [`DocumentXObjects`]). The borrow is taken and released inside the
/// method with no reentrancy — `ExtractFont::resolve` cannot call back into
/// this — so it cannot panic. The consequence is that `DocumentFonts` is
/// not `Sync`; the decomposition is single-threaded per page and nothing
/// shares one across threads.
pub struct DocumentFonts<'a> {
    /// The document view fonts are resolved against (decision 018: pass a
    /// session view to see a font added this session, a base view for a
    /// one-shot CLI read).
    pub view: &'a DocumentView<'a>,
    /// The resource dictionary the `Tf` name is looked up in.
    pub resources: &'a Dict,
    /// Memoized resolutions, including negative ones (a name that is not in
    /// the dictionary must not be re-looked-up on every `Tf`).
    cache: RefCell<HashMap<Vec<u8>, Option<Arc<ExtractFont>>>>,
}

impl<'a> DocumentFonts<'a> {
    /// Build a resolver over `view`'s `resources`.
    #[must_use]
    pub fn new(view: &'a DocumentView<'a>, resources: &'a Dict) -> Self {
        Self {
            view,
            resources,
            cache: RefCell::new(HashMap::new()),
        }
    }

    /// The uncached lookup: `/Font` → `name` → a font dictionary →
    /// [`ExtractFont::resolve`].
    fn lookup(&self, name: &[u8]) -> Option<Arc<ExtractFont>> {
        let font_dict = self
            .resources
            .get(b"Font")
            .map(|o| self.view.resolve(o))
            .and_then(Object::as_dict)?
            .get(name)?;
        let dict = self.view.resolve(font_dict).as_dict()?;
        Some(Arc::new(ExtractFont::resolve(self.view, dict)))
    }
}

impl FontResolver for DocumentFonts<'_> {
    fn resolve(&self, name: &[u8]) -> Option<Arc<ExtractFont>> {
        if let Some(hit) = self.cache.borrow().get(name) {
            return hit.clone();
        }
        let resolved = self.lookup(name);
        self.cache
            .borrow_mut()
            .insert(name.to_vec(), resolved.clone());
        resolved
    }
}

// ---------------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------------

/// Decompose a page's content into selectable vector objects.
///
/// The page's `Contents` streams are concatenated, decoded, and tokenized
/// via [`ContentStream::from_page`] (no bytes change — R46), then walked
/// with `initial` as the starting CTM. Pass [`Matrix::IDENTITY`] to get
/// page-space geometry in genuine PDF default user space (what the GUI
/// provider and the dimensioning subsystem expect).
///
/// ## Which revision gets decomposed (decision 018)
///
/// `view` decides, and the choice is operator-visible:
///
/// - `&session.view()` decomposes **the edited state**. This is what the
///   GUI's `ObjectModelProvider` passes, so a shape the operator just
///   moved is selectable where it now appears, and the dimensioning tool
///   snaps to the geometry actually on screen.
/// - `&doc.view()` decomposes **the base revision**. This is what a
///   one-shot CLI operation passes, and what
///   [`EditSession`](crate::edit::EditSession)'s own vector-surgery path
///   passes deliberately: its `object_index` values must line up with a
///   caller that indexed the base, which is why editing a page whose
///   content was already rewritten this session is *refused* rather than
///   silently misindexed.
///
/// # Errors
///
/// [`crate::content::ContentError`] if the content streams cannot be
/// decoded or tokenized (the same failure the renderer would hit).
///
/// # Examples
///
/// ```no_run
/// use pdfce_core::document::Document;
/// use pdfce_core::page_tree::pages;
/// use pdfce_core::vector::{decompose_page, Matrix};
///
/// # fn demo(doc: &Document) -> Result<(), Box<dyn std::error::Error>> {
/// let page = &pages(doc)?[0];
/// let model = decompose_page(&doc.view(), page, Matrix::IDENTITY)?;
/// println!("{} selectable objects", model.objects.len());
/// # Ok(())
/// # }
/// ```
pub fn decompose_page(
    view: &DocumentView<'_>,
    page: &Page,
    initial: Matrix,
) -> Result<PageObjects, crate::content::ContentError> {
    let content = ContentStream::from_page(view, page)?;
    let xobjects = DocumentXObjects {
        view,
        resources: &page.resources,
    };
    // A page decomposition always resolves fonts: this is the entry point
    // the GUI provider and the CLI both call, and both surface the text
    // preview. The `NoFonts` path exists for callers that have no document
    // (unit tests, the fuzz target) and for `decompose`'s stable signature,
    // not as a cheaper mode a real caller should choose — `DocumentFonts`
    // memoizes, so the cost is one resolution per distinct font resource.
    let fonts = DocumentFonts::new(view, &page.resources);
    Ok(decompose_with_fonts(&content, initial, &xobjects, &fonts))
}

/// Decompose an already-tokenized content stream, with an explicit
/// XObject resolver and initial CTM, and **no font resolution**.
///
/// Geometry-only: every text object comes back with
/// [`TextPreview::Unavailable`] and no [`TextFont`]. That is the right
/// answer for every caller that indexes objects for *editing* — `edit.rs`'s
/// surgery planner, the snap engine, the fuzz targets — none of which needs
/// to know what a string says, and all of which would otherwise pay for a
/// `/ToUnicode` parse per font.
///
/// Callers that display objects to a human want
/// [`decompose_with_fonts`] (or [`decompose_page`], which supplies both
/// resolvers for a page). This function's signature is deliberately
/// unchanged from before text previews existed, so every geometry caller
/// stayed a no-diff.
#[must_use]
pub fn decompose(
    content: &ContentStream,
    initial: Matrix,
    xobjects: &dyn XObjectResolver,
) -> PageObjects {
    decompose_with_fonts(content, initial, xobjects, &NoFonts)
}

/// Decompose an already-tokenized content stream with **both** resolvers —
/// the walk's true entry point.
///
/// `xobjects` classifies `Do` names (§8.8) so an image/form object gets a
/// bbox and its sample count; `fonts` resolves `Tf` names (§9.6.2.1) so a
/// text object gets its decoded preview and typeface. Either may be the
/// inert [`NoXObjects`]/[`NoFonts`]; the walk's geometry is identical
/// either way, which is what makes the byte-inertness claim (R46) and the
/// renderer cross-check independent of whether a caller asked for text
/// detail.
///
/// # Examples
///
/// ```
/// use pdfce_core::content::ContentStream;
/// use pdfce_core::vector::{
///     Matrix, NoFonts, NoXObjects, TextPreview, VectorObject, decompose_with_fonts,
/// };
///
/// // With no font resolver the text object is honest about WHY it has no
/// // preview: nothing was attempted, as opposed to nothing being there.
/// let cs = ContentStream::parse(b"BT /F1 12 Tf 10 10 Td (Hi) Tj ET".to_vec())?;
/// let model = decompose_with_fonts(&cs, Matrix::IDENTITY, &NoXObjects, &NoFonts);
/// let VectorObject::Text(text) = &model.objects[0] else { panic!("a text object") };
/// assert_eq!(text.preview, TextPreview::Unavailable);
/// // The `Tf` operands are read straight from the stream, so the resource
/// // name and size are known even with no document behind them.
/// let font = text.font.as_ref().expect("a /Tf was in effect");
/// assert_eq!(font.resource, "F1");
/// assert_eq!(font.size, 12.0);
/// assert_eq!(font.base_font, None); // no resolver, so no typeface claim
/// # Ok::<(), pdfce_core::content::ContentError>(())
/// ```
#[must_use]
pub fn decompose_with_fonts(
    content: &ContentStream,
    initial: Matrix,
    xobjects: &dyn XObjectResolver,
    fonts: &dyn FontResolver,
) -> PageObjects {
    let mut d = Decomposer::new(content, initial, xobjects, fonts);
    d.run();
    PageObjects {
        objects: d.objects,
        initial,
        diagnostics: d.diag,
    }
}

// ---------------------------------------------------------------------------
// The walk
// ---------------------------------------------------------------------------

/// The Pass 9a subset of the graphics state the object model tracks — the
/// CTM (the only geometry-load-bearing part), the stroke geometry width,
/// the device colours, and the text-state parameters the approximate text
/// bbox and the text preview need. Saved/restored by `q`/`Q` (§8.4.2), like
/// the renderer's [`crate::content`]-driven state.
///
/// `Clone` rather than `Copy` since the resolved font joined it: the font
/// IS part of the text state and therefore part of the graphics state
/// (§9.3), so `q`/`Q` must save and restore it, and an [`Arc`] clone on a
/// `q` is one refcount bump.
#[derive(Debug, Clone)]
struct GState {
    ctm: Matrix,
    line_width: f64,
    fill_color: Rgb,
    stroke_color: Rgb,
    /// The `Tf` size operand (§9.3.1 `Tfs`), text space, unscaled.
    font_size: f64,
    /// The `Tf` resource name, verbatim from the content stream.
    font_resource: Option<Vec<u8>>,
    /// The decoder for [`Self::font_resource`], if the [`FontResolver`]
    /// could produce one.
    font: Option<Arc<ExtractFont>>,
    leading: f64,
}

impl GState {
    fn initial(ctm: Matrix) -> Self {
        Self {
            ctm,
            line_width: 1.0, // Table 52 initial
            fill_color: Rgb::BLACK,
            stroke_color: Rgb::BLACK,
            font_size: 0.0,
            font_resource: None,
            font: None,
            leading: 0.0,
        }
    }
}

/// The in-progress path object (mirrors the renderer's `Interpreter` path
/// fields: `path`/`path_ctm`/`current`/`subpath_start`/`needs_move`).
struct PathAccum {
    subpaths: Vec<Subpath>,
    open: Option<Subpath>,
    ctm: Matrix,
    current: Option<Point>,
    subpath_start: Option<Point>,
    needs_move: bool,
    token_start: usize,
}

/// The in-progress text object (`BT`…`ET`), including the preview
/// accumulator.
struct TextAccum {
    token_start: usize,
    origins: Bounds,
    max_font_size: f64,
    text_matrix: Matrix,
    line_matrix: Matrix,
    /// Decoded characters so far, never longer than
    /// [`MAX_TEXT_PREVIEW_CHARS`] characters.
    preview: String,
    /// Character count of `preview` (tracked rather than recounted, since
    /// `String::chars().count()` is O(n) and this is checked per code).
    preview_chars: usize,
    /// Whether decoding stopped at the cap with codes still to come.
    truncated: bool,
    /// Codes in the decoded prefix that the §9.10.2 ladder mapped.
    decoded_codes: usize,
    /// Codes in the decoded prefix that reached the ladder's failure clause.
    failed_codes: usize,
    /// Whether any show operator carried a string operand at all — the
    /// difference between [`TextPreview::Empty`] and a decode result.
    showed_any: bool,
    /// Whether at least one of those strings was decoded through a resolved
    /// font. False means no decoder was in scope (no resolver, or a `Tf`
    /// naming a font the resource dictionary does not hold), which is
    /// [`TextPreview::Unavailable`] — a fact about the LOOKUP, not about the
    /// document's text.
    decode_attempted: bool,
    /// The font at the FIRST show operator ([`TextFont`]'s own rationale).
    font: Option<TextFont>,
}

impl TextAccum {
    fn new(token_start: usize) -> Self {
        Self {
            token_start,
            origins: Bounds::EMPTY,
            max_font_size: 0.0,
            text_matrix: Matrix::IDENTITY,
            line_matrix: Matrix::IDENTITY,
            preview: String::new(),
            preview_chars: 0,
            truncated: false,
            decoded_codes: 0,
            failed_codes: 0,
            showed_any: false,
            decode_attempted: false,
            font: None,
        }
    }

    /// Fold the accumulator into the disclosed [`TextPreview`] (the four
    /// cases the enum documents).
    fn finish(self) -> (TextPreview, Option<TextFont>) {
        let preview = if !self.showed_any {
            TextPreview::Empty
        } else if !self.decode_attempted {
            // A show operator ran, but no decoder was in scope. Saying
            // "empty" here would blame the document for a failed lookup.
            TextPreview::Unavailable
        } else if self.decoded_codes == 0 && self.failed_codes > 0 {
            TextPreview::Undecodable
        } else {
            TextPreview::Decoded {
                text: self.preview,
                truncated: self.truncated,
                lossy: self.failed_codes > 0,
            }
        };
        (preview, self.font)
    }
}

struct Decomposer<'a> {
    content: &'a ContentStream,
    xobjects: &'a dyn XObjectResolver,
    fonts: &'a dyn FontResolver,
    stack: Vec<GState>,
    gs: GState,
    path: Option<PathAccum>,
    text: Option<TextAccum>,
    objects: Vec<VectorObject>,
    diag: DecomposeDiagnostics,
    total_nodes: usize,
}

impl<'a> Decomposer<'a> {
    fn new(
        content: &'a ContentStream,
        initial: Matrix,
        xobjects: &'a dyn XObjectResolver,
        fonts: &'a dyn FontResolver,
    ) -> Self {
        Self {
            content,
            xobjects,
            fonts,
            stack: Vec::new(),
            gs: GState::initial(initial),
            path: None,
            text: None,
            objects: Vec::new(),
            diag: DecomposeDiagnostics::default(),
            total_nodes: 0,
        }
    }

    /// Walk the token stream, mirroring [`ContentStream::operations`]'s
    /// operand-run/operator segmentation but tracking each operation's
    /// first-token index (the object token-range start).
    fn run(&mut self) {
        let mut run_start = 0usize;
        for (i, tok) in self.content.tokens.iter().enumerate() {
            match tok.kind {
                ContentTokenKind::Operand(_) => {}
                _ => {
                    let operands = self.content.tokens.get(run_start..i).unwrap_or(&[]);
                    self.operation(operands, tok, run_start, i);
                    run_start = i + 1;
                }
            }
        }
        // A trailing, unpainted path (malformed per §8.5.3 "a painting
        // operator shall follow") is dropped, matching the renderer's
        // discard of an unpainted `PathBuilder`; its tokens stay in the
        // stream (byte-inert).
    }

    /// Handle one operation: `operator` is the operator token (or an inline
    /// image), `operands` the preceding operand run, `first`/`op_index` the
    /// operation's token bounds.
    fn operation(
        &mut self,
        operands: &[ContentToken],
        operator: &ContentToken,
        first: usize,
        op_index: usize,
    ) {
        // The one non-operator "operation": a complete inline image. Its
        // parameter dictionary travels WITH the token (§8.9.7), so its
        // sample count is read here and needs no resolver.
        if let ContentTokenKind::InlineImage { params, .. } = &operator.kind {
            let pixel_size = inline_pixel_size(params);
            self.emit_image(
                ImageSource::Inline,
                self.gs.ctm,
                unit_square(),
                pixel_size,
                first,
                op_index,
            );
            return;
        }
        let Some(name) = operator.span.slice(&self.content.buf) else {
            return;
        };
        let nums = operand_nums(operands);
        match name {
            // ---- graphics state (Table 57) ----
            b"q" => self.stack.push(self.gs.clone()),
            b"Q" => match self.stack.pop() {
                Some(prev) => self.gs = prev,
                None => self.diag.unbalanced_q += 1,
            },
            b"cm" => {
                if let &[a, b, c, d, e, f] = nums.as_slice() {
                    self.gs.ctm = Matrix::new(a, b, c, d, e, f).post_concat(self.gs.ctm);
                }
            }
            b"w" => {
                if let &[lw] = nums.as_slice() {
                    self.gs.line_width = lw.max(0.0);
                }
            }
            // ---- device colours (§8.6.4, Table 74 subset) ----
            b"g" => set_color(&mut self.gs.fill_color, Rgb::from_gray, &nums),
            b"G" => set_color(&mut self.gs.stroke_color, Rgb::from_gray, &nums),
            b"rg" => set_rgb(&mut self.gs.fill_color, &nums),
            b"RG" => set_rgb(&mut self.gs.stroke_color, &nums),
            b"k" => set_cmyk(&mut self.gs.fill_color, &nums),
            b"K" => set_cmyk(&mut self.gs.stroke_color, &nums),

            // ---- path construction (Table 59) ----
            b"m" => {
                if let &[x, y] = nums.as_slice() {
                    self.move_to(Point::new(x, y), first);
                }
            }
            b"l" => {
                if let &[x, y] = nums.as_slice() {
                    self.line_to(Point::new(x, y), first);
                }
            }
            b"c" => {
                if let &[x1, y1, x2, y2, x3, y3] = nums.as_slice() {
                    self.curve_to(
                        Point::new(x1, y1),
                        Point::new(x2, y2),
                        Point::new(x3, y3),
                        first,
                    );
                }
            }
            b"v" => {
                // First control = current point (shared primitive).
                if let &[x2, y2, x3, y3] = nums.as_slice()
                    && let Some(cur) = self.current_for_segment(first)
                {
                    let (c1, c2, end) = cubic_from_v(cur, x2, y2, x3, y3);
                    self.append_cubic(c1, c2, end);
                }
            }
            b"y" => {
                // Second control = endpoint (shared primitive).
                if let &[x1, y1, x3, y3] = nums.as_slice()
                    && self.current_for_segment(first).is_some()
                {
                    let (c1, c2, end) = cubic_from_y(x1, y1, x3, y3);
                    self.append_cubic(c1, c2, end);
                }
            }
            b"h" => self.close_subpath(),
            b"re" => {
                if let &[x, y, w, h] = nums.as_slice() {
                    self.rect(x, y, w, h, first);
                }
            }

            // ---- path painting (Table 60) + clipping (Table 61) ----
            b"S" => self.paint(
                op_index,
                PaintStyle {
                    fill: None,
                    stroke: true,
                },
                false,
            ),
            b"s" => self.paint(
                op_index,
                PaintStyle {
                    fill: None,
                    stroke: true,
                },
                true,
            ),
            b"f" | b"F" => self.paint(
                op_index,
                PaintStyle {
                    fill: Some(FillRule::NonZero),
                    stroke: false,
                },
                false,
            ),
            b"f*" => self.paint(
                op_index,
                PaintStyle {
                    fill: Some(FillRule::EvenOdd),
                    stroke: false,
                },
                false,
            ),
            b"B" => self.paint(
                op_index,
                PaintStyle {
                    fill: Some(FillRule::NonZero),
                    stroke: true,
                },
                false,
            ),
            b"B*" => self.paint(
                op_index,
                PaintStyle {
                    fill: Some(FillRule::EvenOdd),
                    stroke: true,
                },
                false,
            ),
            b"b" => self.paint(
                op_index,
                PaintStyle {
                    fill: Some(FillRule::NonZero),
                    stroke: true,
                },
                true,
            ),
            b"b*" => self.paint(
                op_index,
                PaintStyle {
                    fill: Some(FillRule::EvenOdd),
                    stroke: true,
                },
                true,
            ),
            b"n" => self.paint(
                op_index,
                PaintStyle {
                    fill: None,
                    stroke: false,
                },
                false,
            ),

            // ---- text objects (Table 107) ----
            b"BT" => {
                self.discard_path(); // defensive: a path open across BT is malformed
                self.text = Some(TextAccum::new(op_index));
            }
            b"ET" => self.end_text(op_index),

            // ---- text state / positioning the bbox + preview need ----
            b"Tf" => {
                // `Tf name size` (§9.3.1): the name operand selects the font
                // resource, the number operand is the size. Both are part of
                // the graphics state, so both survive to the next `BT`.
                if let Some(size) = nums.last().copied() {
                    self.gs.font_size = size;
                }
                if let Some(resource) = last_name(operands) {
                    // Resolve eagerly rather than at the first show operator:
                    // `DocumentFonts` memoizes, so a repeated `Tf` is a hash
                    // lookup, and doing it here keeps the show path (which
                    // runs per string) free of resolution logic.
                    self.gs.font = self.fonts.resolve(&resource);
                    self.gs.font_resource = Some(resource);
                }
            }
            b"TL" => {
                if let &[v] = nums.as_slice() {
                    self.gs.leading = v;
                }
            }
            b"Td" => {
                if let &[tx, ty] = nums.as_slice() {
                    self.text_line_offset(tx, ty);
                }
            }
            b"TD" => {
                if let &[tx, ty] = nums.as_slice() {
                    self.gs.leading = -ty;
                    self.text_line_offset(tx, ty);
                }
            }
            b"Tm" => {
                if let &[a, b, c, d, e, f] = nums.as_slice()
                    && let Some(t) = self.text.as_mut()
                {
                    let m = Matrix::new(a, b, c, d, e, f);
                    t.text_matrix = m;
                    t.line_matrix = m;
                }
            }
            b"T*" => {
                let leading = self.gs.leading;
                self.text_line_offset(0.0, -leading);
            }
            b"Tj" | b"TJ" | b"'" | b"\"" => self.show_text(operands),

            // ---- external objects (§8.8) ----
            b"Do" => self.do_xobject(operands, first, op_index),

            // Everything else (state we don't model for geometry, shading,
            // marked content, unknown operators) is ignored for the object
            // model — it affects neither node geometry nor selectability.
            _ => {}
        }
    }

    // -- path construction helpers (mirror the renderer's Interpreter) --

    /// Capture the CTM at the path's first construction op; a mid-path
    /// `cm` is tolerated (keep the first CTM) and counted, exactly as the
    /// renderer's `capture_path_ctm` does.
    fn ensure_path(&mut self, first: usize) {
        if self.path.is_none() {
            // First construction op of a new object: capture today's CTM.
            self.path = Some(PathAccum {
                subpaths: Vec::new(),
                open: None,
                ctm: self.gs.ctm,
                current: None,
                subpath_start: None,
                needs_move: false,
                token_start: first,
            });
            return;
        }
        // An existing object seeing a different CTM = a mid-path `cm`
        // (legal, vanishingly rare): keep the captured CTM, count it.
        let ctm = self.gs.ctm;
        if self.path.as_ref().is_some_and(|p| p.ctm != ctm) {
            self.diag.midpath_cm += 1;
        }
    }

    fn move_to(&mut self, p: Point, first: usize) {
        self.ensure_path(first);
        finalize_open(self.path.as_mut());
        if let Some(pa) = self.path.as_mut() {
            pa.open = Some(Subpath {
                start: p,
                segments: Vec::new(),
                closed: false,
            });
            pa.current = Some(p);
            pa.subpath_start = Some(p);
            pa.needs_move = false;
        }
    }

    /// The renderer's `begin_segment`: a segment needs a current point;
    /// after `h`/`re` (`needs_move`) it opens a new subpath at the current
    /// point. Returns the current point, or `None` (skip + count) if there
    /// is no current point.
    fn current_for_segment(&mut self, first: usize) -> Option<Point> {
        // A segment with no path at all AND no current point is a
        // §8.5.2.1 error.
        let cur = self.path.as_ref().and_then(|p| p.current);
        let Some(cur) = cur else {
            self.diag.segment_without_current += 1;
            return None;
        };
        self.ensure_path(first);
        if let Some(pa) = self.path.as_mut()
            && pa.needs_move
        {
            pa.open = Some(Subpath {
                start: cur,
                segments: Vec::new(),
                closed: false,
            });
            pa.subpath_start = Some(cur);
            pa.needs_move = false;
        }
        Some(cur)
    }

    fn line_to(&mut self, p: Point, first: usize) {
        if self.current_for_segment(first).is_some() {
            self.push_segment(Segment::Line { to: p }, p);
        }
    }

    fn curve_to(&mut self, c1: Point, c2: Point, end: Point, first: usize) {
        if self.current_for_segment(first).is_some() {
            self.append_cubic(c1, c2, end);
        }
    }

    fn append_cubic(&mut self, c1: Point, c2: Point, end: Point) {
        self.push_segment(Segment::Cubic { c1, c2, to: end }, end);
    }

    fn push_segment(&mut self, seg: Segment, new_current: Point) {
        if self.total_nodes >= MAX_NODES {
            self.diag.nodes_dropped += 1;
            return;
        }
        if let Some(pa) = self.path.as_mut()
            && let Some(open) = pa.open.as_mut()
        {
            open.segments.push(seg);
            pa.current = Some(new_current);
            self.total_nodes += 1;
        }
    }

    /// `h` (§8.5.2.1): close the current subpath; the current point
    /// becomes the subpath start, and the next segment op opens a new
    /// subpath there.
    fn close_subpath(&mut self) {
        if let Some(pa) = self.path.as_mut() {
            if let Some(open) = pa.open.as_mut() {
                open.closed = true;
            }
            finalize_open_pa(pa);
            pa.current = pa.subpath_start;
            pa.needs_move = true;
        }
    }

    /// `re x y w h` (Table 59): a complete closed subpath, expanded via the
    /// shared [`rect_corners`] primitive.
    fn rect(&mut self, x: f64, y: f64, w: f64, h: f64, first: usize) {
        self.ensure_path(first);
        finalize_open(self.path.as_mut());
        if self.total_nodes.saturating_add(4) > MAX_NODES {
            self.diag.nodes_dropped += 1;
            return;
        }
        let c = rect_corners(x, y, w, h);
        if let Some(pa) = self.path.as_mut() {
            pa.subpaths.push(Subpath {
                start: c[0],
                segments: vec![
                    Segment::Line { to: c[1] },
                    Segment::Line { to: c[2] },
                    Segment::Line { to: c[3] },
                ],
                closed: true,
            });
            pa.current = Some(c[0]);
            pa.subpath_start = Some(c[0]);
            pa.needs_move = true;
            self.total_nodes += 4;
        }
    }

    /// Terminate the current path object with `style`; `close` first closes
    /// the open subpath (the `s`/`b`/`b*` operators).
    fn paint(&mut self, op_index: usize, style: PaintStyle, close: bool) {
        let Some(mut pa) = self.path.take() else {
            // A painting operator with no path: the renderer's empty-path
            // case (nothing drawn, a `n`/`W` clips everything). No object.
            return;
        };
        if close {
            if let Some(open) = pa.open.as_mut() {
                open.closed = true;
            } else if let Some(last) = pa.subpaths.last_mut() {
                last.closed = true;
            }
        }
        finalize_open_pa(&mut pa);

        if pa.subpaths.is_empty() {
            return; // no geometry (a lone `m` then paint)
        }
        if self.objects.len() >= MAX_OBJECTS {
            self.diag.objects_dropped += 1;
            return;
        }

        let ctm = pa.ctm;
        let page_bbox = subpaths_page_bounds(&pa.subpaths, ctm);
        let bytes = self.span_of(pa.token_start, op_index);
        let obj = PathObject {
            subpaths: pa.subpaths,
            ctm,
            style,
            line_width: self.gs.line_width,
            fill_color: self.gs.fill_color,
            stroke_color: self.gs.stroke_color,
            tokens: TokenRange {
                start: pa.token_start,
                end: op_index + 1,
            },
            bytes,
            page_bbox,
        };
        self.diag.paths += 1;
        self.objects.push(VectorObject::Path(obj));
    }

    /// Drop an in-progress path without emitting an object (a `BT` opening
    /// while a path is open — malformed, tolerated).
    fn discard_path(&mut self) {
        self.path = None;
    }

    // -- text helpers (approximate bbox, module docs) --

    fn text_line_offset(&mut self, tx: f64, ty: f64) {
        if let Some(t) = self.text.as_mut() {
            t.line_matrix = Matrix::translate(tx, ty).post_concat(t.line_matrix);
            t.text_matrix = t.line_matrix;
        }
    }

    /// Handle one text-showing operator (`Tj`/`TJ`/`'`/`"`): record the pen
    /// origin for the approximate bbox, capture the font on the first one,
    /// and decode the operand strings into the preview.
    ///
    /// The origin bookkeeping is exactly what this used to do and is
    /// unchanged — the bbox geometry (and therefore hit-testing, and
    /// therefore every existing test and fixture expectation) does not move
    /// because a preview was added.
    fn show_text(&mut self, operands: &[ContentToken]) {
        // Snapshot the graphics-state reads before borrowing `self.text`
        // mutably; `Arc::clone` is a refcount bump, not a font copy.
        let ctm = self.gs.ctm;
        let font_size = self.gs.font_size;
        let font = self.gs.font.clone();
        let resource = self.gs.font_resource.clone();

        let Some(t) = self.text.as_mut() else {
            return; // a show operator outside BT/ET — malformed, ignored
        };

        let origin = ctm.map_point(t.text_matrix.map_point(Point::new(0.0, 0.0)));
        t.origins = t.origins.union_point(origin);
        if font_size > t.max_font_size {
            t.max_font_size = font_size;
        }

        // The font of the FIRST show operator identifies the object
        // (`TextFont`'s own docs). A `Tf`-less show has no font to name.
        if t.font.is_none()
            && let Some(resource) = resource
        {
            t.font = Some(TextFont {
                resource: truncate_name(&String::from_utf8_lossy(&resource)),
                base_font: font
                    .as_ref()
                    .map(|f| truncate_name(&f.base_font))
                    .filter(|b| !b.is_empty()),
                size: font_size,
            });
        }

        // §9.4.3 Table 109: `Tj`/`'` take one string; `"` takes `aw ac
        // string`; `TJ` takes an array of strings interleaved with numeric
        // offsets. Every STRING operand in the run is shown text and every
        // number is positioning, so walking the operands and taking the
        // strings covers all four operators without a per-operator branch —
        // and tolerates a malformed run (a `Tj` with two strings) by
        // showing both, which is what a lenient reader would render.
        for token in operands {
            let ContentTokenKind::Operand(object) = &token.kind else {
                continue;
            };
            match object {
                Object::String(bytes) => self.decode_show_string(bytes),
                Object::Array(items) => {
                    for item in items {
                        if let Object::String(bytes) = item {
                            self.decode_show_string(bytes);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Decode one show string's bytes into the in-progress preview through
    /// the §9.10.2 ladder, stopping at [`MAX_TEXT_PREVIEW_CHARS`].
    ///
    /// **Stops decoding, not just appending.** The cap is a work bound as
    /// well as a memory bound: a hostile page can carry a megabyte of show
    /// strings per text object, and mapping every code through a
    /// `/ToUnicode` CMap only to discard the result would be an easy
    /// amplification (ARCHITECTURE.md §10). The consequence — that
    /// [`TextPreview::Decoded::lossy`] describes the decoded PREFIX rather
    /// than the whole string — is documented on the field.
    fn decode_show_string(&mut self, bytes: &[u8]) {
        let font = self.gs.font.clone();
        let Some(t) = self.text.as_mut() else {
            return;
        };
        if !bytes.is_empty() {
            t.showed_any = true;
        }
        let Some(font) = font else {
            return; // no decoder in scope → TextPreview::Unavailable
        };
        if !bytes.is_empty() {
            t.decode_attempted = true;
        }
        for code in font.codes(bytes) {
            if t.preview_chars >= MAX_TEXT_PREVIEW_CHARS {
                t.truncated = true;
                return;
            }
            let (text, rung) = font.to_unicode(code.value);
            if rung == LadderRung::Failed {
                t.failed_codes += 1;
            } else {
                t.decoded_codes += 1;
            }
            for ch in text.chars() {
                if t.preview_chars >= MAX_TEXT_PREVIEW_CHARS {
                    // One code can map to several characters (§9.10.3), so
                    // the cap can be reached mid-code; the rest of THIS
                    // code's characters are elided too, and disclosed.
                    t.truncated = true;
                    return;
                }
                t.preview.push(ch);
                t.preview_chars += 1;
            }
        }
    }

    fn end_text(&mut self, op_index: usize) {
        let Some(t) = self.text.take() else {
            return; // unbalanced ET
        };
        if t.origins.is_empty() {
            return; // a text object that showed nothing
        }
        if self.objects.len() >= MAX_OBJECTS {
            self.diag.objects_dropped += 1;
            return;
        }
        // Inflate the origin bounds by the largest em box seen — a coarse,
        // disclosed over-approximation (TextObject docs).
        let margin = (t.max_font_size).max(1.0);
        let page_bbox = t.origins.inflate(margin);
        let bytes = self.span_of(t.token_start, op_index);
        let token_start = t.token_start;
        let (preview, font) = t.finish();
        self.diag.text += 1;
        self.objects.push(VectorObject::Text(TextObject {
            page_bbox,
            approximate: true,
            preview,
            font,
            tokens: TokenRange {
                start: token_start,
                end: op_index + 1,
            },
            bytes,
        }));
    }

    // -- Do / image --

    fn do_xobject(&mut self, operands: &[ContentToken], first: usize, op_index: usize) {
        let Some(name) = last_name(operands) else {
            self.diag.unresolved_xobject += 1;
            return;
        };
        match self.xobjects.classify(&name) {
            Some(XObjectShape::Image { pixel_size }) => {
                self.emit_image(
                    ImageSource::XObject,
                    self.gs.ctm,
                    unit_square(),
                    pixel_size,
                    first,
                    op_index,
                );
            }
            Some(XObjectShape::Form { bbox, matrix }) => {
                let ctm = matrix.post_concat(self.gs.ctm);
                let corners = bounds_corners(bbox);
                // A form has no samples (§8.10) — `None`, not `Some((0, 0))`.
                self.emit_image(ImageSource::Form, ctm, corners, None, first, op_index);
            }
            None => self.diag.unresolved_xobject += 1,
        }
    }

    /// Emit an image/form object: `local_corners` are the four corners in
    /// the object's own space (unit square, or a form `/BBox`), mapped to
    /// page space by `ctm`; `pixel_size` is the sample count for an image
    /// and `None` for a form.
    fn emit_image(
        &mut self,
        source: ImageSource,
        ctm: Matrix,
        local_corners: [Point; 4],
        pixel_size: Option<(u32, u32)>,
        first: usize,
        op_index: usize,
    ) {
        if self.objects.len() >= MAX_OBJECTS {
            self.diag.objects_dropped += 1;
            return;
        }
        let page_bbox = local_corners
            .iter()
            .fold(Bounds::EMPTY, |acc, &c| acc.union_point(ctm.map_point(c)));
        let bytes = self.span_of(first, op_index);
        match source {
            ImageSource::Form => self.diag.forms += 1,
            ImageSource::Inline | ImageSource::XObject => self.diag.images += 1,
        }
        self.objects.push(VectorObject::Image(ImageObject {
            ctm,
            page_bbox,
            source,
            pixel_size,
            tokens: TokenRange {
                start: first,
                end: op_index + 1,
            },
            bytes,
        }));
    }

    /// The byte span in the decoded content buffer from token `start`'s
    /// first byte through token `end`'s last byte.
    fn span_of(&self, start: usize, end: usize) -> ByteSpan {
        let s = self.content.tokens.get(start).map_or(0, |t| t.span.start);
        let e = self
            .content
            .tokens
            .get(end)
            .map_or_else(|| self.content.buf.len(), |t| t.span.end());
        ByteSpan::from_range(s..e)
    }
}

// ---------------------------------------------------------------------------
// Free helpers
// ---------------------------------------------------------------------------

/// Finalize the open subpath of `pa` (if any), pushing it if it has at
/// least one segment (a lone `m` produces no contour, matching the
/// renderer's `PathBuilder` collapse of an empty move).
fn finalize_open_pa(pa: &mut PathAccum) {
    if let Some(open) = pa.open.take()
        && !open.segments.is_empty()
    {
        pa.subpaths.push(open);
    }
}

/// [`finalize_open_pa`] through an `Option<&mut PathAccum>`.
fn finalize_open(pa: Option<&mut PathAccum>) {
    if let Some(pa) = pa {
        finalize_open_pa(pa);
    }
}

/// Collect the numeric operands of an operation, in order (§8.5's operators
/// take numeric operands; a wrong-typed operand is skipped, matching the
/// renderer's tolerance).
fn operand_nums(operands: &[ContentToken]) -> Vec<f64> {
    operands
        .iter()
        .filter_map(|t| match &t.kind {
            ContentTokenKind::Operand(o) => o.as_number(),
            _ => None,
        })
        .collect()
}

/// Cut a name at [`MAX_FONT_NAME_BYTES`], on a UTF-8 character boundary.
///
/// `floor_char_boundary` is not stable, so the boundary is found by
/// scanning back from the limit — at most three bytes, since a UTF-8
/// sequence is at most four. Returning a byte-sliced `String` without this
/// would panic on a multi-byte name, which is precisely the adversarial
/// input a hostile `/BaseFont` would carry.
fn truncate_name(name: &str) -> String {
    if name.len() <= MAX_FONT_NAME_BYTES {
        return name.to_owned();
    }
    let mut end = MAX_FONT_NAME_BYTES;
    while end > 0 && !name.is_char_boundary(end) {
        end -= 1;
    }
    name.get(..end).unwrap_or_default().to_owned()
}

/// The last name operand of an operation (`Do`'s XObject name), taken from
/// the end of the run for the same reason the renderer's `last_name` does.
fn last_name(operands: &[ContentToken]) -> Option<Vec<u8>> {
    operands.iter().rev().find_map(|t| match &t.kind {
        ContentTokenKind::Operand(Object::Name(n)) => Some(n.as_bytes().to_vec()),
        _ => None,
    })
}

/// Set a colour from a single-component (`g`/`G`) operator.
fn set_color(slot: &mut Rgb, f: fn(f32) -> Rgb, nums: &[f64]) {
    if let &[v] = nums {
        *slot = f(v as f32);
    }
}

/// Set a colour from an `rg`/`RG` operator.
fn set_rgb(slot: &mut Rgb, nums: &[f64]) {
    if let &[r, g, b] = nums {
        *slot = Rgb::from_rgb(r as f32, g as f32, b as f32);
    }
}

/// Set a colour from a `k`/`K` operator.
fn set_cmyk(slot: &mut Rgb, nums: &[f64]) {
    if let &[c, m, y, k] = nums {
        *slot = Rgb::from_cmyk(c as f32, m as f32, y as f32, k as f32);
    }
}

/// The user-space unit square (§8.9.4's image-space boundary), as four
/// corners.
fn unit_square() -> [Point; 4] {
    [
        Point::new(0.0, 0.0),
        Point::new(1.0, 0.0),
        Point::new(1.0, 1.0),
        Point::new(0.0, 1.0),
    ]
}

/// The four corners of a [`Bounds`] (empty box → degenerate origin
/// corners, harmless downstream).
fn bounds_corners(b: Bounds) -> [Point; 4] {
    if b.is_empty() {
        return [Point::new(0.0, 0.0); 4];
    }
    [
        Point::new(b.min.x, b.min.y),
        Point::new(b.max.x, b.min.y),
        Point::new(b.max.x, b.max.y),
        Point::new(b.min.x, b.max.y),
    ]
}

/// The page-space bounding box of a set of user-space subpaths under
/// `ctm` — the control-point hull (a conservative superset of the exact
/// curve bounds; a curve never leaves its control hull).
fn subpaths_page_bounds(subpaths: &[Subpath], ctm: Matrix) -> Bounds {
    let mut b = Bounds::EMPTY;
    for sp in subpaths {
        b = b.union_point(ctm.map_point(sp.start));
        for seg in &sp.segments {
            match *seg {
                Segment::Line { to } => b = b.union_point(ctm.map_point(to)),
                Segment::Cubic { c1, c2, to } => {
                    b = b
                        .union_point(ctm.map_point(c1))
                        .union_point(ctm.map_point(c2))
                        .union_point(ctm.map_point(to));
                }
            }
        }
    }
    b
}

/// Whether a subpath is a 4-anchor quad: exactly 3 line segments after the
/// start (start + 3 lines closing back = 4 corners), all straight. An
/// `re` rectangle and a hand-drawn closed 4-line quad both match.
fn subpath_is_quad(sp: &Subpath) -> bool {
    sp.segments.len() == 3
        && sp
            .segments
            .iter()
            .all(|s| matches!(s, Segment::Line { .. }))
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
    use crate::PdfVersion;
    use crate::object::{Name, ObjId};
    use std::collections::BTreeMap;

    fn model(src: &[u8]) -> PageObjects {
        let cs = ContentStream::parse(src.to_vec()).unwrap();
        decompose(&cs, Matrix::IDENTITY, &NoXObjects)
    }

    // -- the font-resolution test rig ---------------------------------------
    //
    // A hand-built `ObjectGraph` (the same shape `view.rs`'s own tests use)
    // so the DECODING path is exercised without dragging a parsed file into
    // a unit test. Two font resources, chosen to be the two ends of the
    // §9.10.2 ladder:
    //
    //   /F1           Helvetica, a standard-14 simple font — rung 2 via the
    //                 AGL, so ASCII decodes exactly.
    //   /Undecodable  Type0 / Identity-H with an Adobe-Identity-0 descendant
    //                 and NO /ToUnicode — §9.10.2 excludes Identity-H from
    //                 rung 3's first disjunct by name and the descendant
    //                 satisfies neither half of the second, so every code
    //                 reaches the failure clause. Structurally the same case
    //                 `fixtures/synthetic/text/identity-h-no-tounicode.pdf`
    //                 pins for extraction.

    struct TestGraph {
        objects: BTreeMap<ObjId, Object>,
        trailer: Dict,
    }

    impl ObjectGraph for TestGraph {
        fn value(&self, id: ObjId) -> Option<&Object> {
            self.objects.get(&id)
        }
        fn trailer_entry(&self, key: &[u8]) -> Option<&Object> {
            self.trailer.get(key)
        }
    }

    fn dict(entries: &[(&[u8], Object)]) -> Dict {
        let mut d = Dict::new();
        for (k, v) in entries {
            d.insert(Name::from(*k), v.clone());
        }
        d
    }

    fn name(v: &[u8]) -> Object {
        Object::Name(Name::from(v))
    }

    /// A `/Font` resource dictionary holding the two fonts above.
    fn font_resources() -> Dict {
        let helvetica = dict(&[
            (b"Type", name(b"Font")),
            (b"Subtype", name(b"Type1")),
            (b"BaseFont", name(b"Helvetica")),
        ]);
        let descendant = dict(&[
            (b"Type", name(b"Font")),
            (b"Subtype", name(b"CIDFontType2")),
            (b"BaseFont", name(b"NoUnicode")),
            (
                b"CIDSystemInfo",
                Object::Dict(dict(&[
                    (b"Registry", Object::String(b"Adobe".to_vec())),
                    (b"Ordering", Object::String(b"Identity".to_vec())),
                    (b"Supplement", Object::Integer(0)),
                ])),
            ),
        ]);
        let undecodable = dict(&[
            (b"Type", name(b"Font")),
            (b"Subtype", name(b"Type0")),
            (b"BaseFont", name(b"NoUnicode")),
            (b"Encoding", name(b"Identity-H")),
            (
                b"DescendantFonts",
                Object::Array(vec![Object::Dict(descendant)]),
            ),
        ]);
        let fonts = dict(&[
            (b"F1", Object::Dict(helvetica.clone())),
            (b"F2", Object::Dict(helvetica)),
            (b"Undecodable", Object::Dict(undecodable)),
        ]);
        dict(&[(b"Font", Object::Dict(fonts))])
    }

    fn test_graph() -> TestGraph {
        TestGraph {
            objects: BTreeMap::new(),
            trailer: Dict::new(),
        }
    }

    /// Decompose `src` with a real [`DocumentFonts`] over [`font_resources`].
    fn model_with_fonts(src: &[u8]) -> PageObjects {
        let graph = test_graph();
        let view = DocumentView::new(&graph, b"", PdfVersion { major: 1, minor: 7 });
        let resources = font_resources();
        let fonts = DocumentFonts::new(&view, &resources);
        let cs = ContentStream::parse(src.to_vec()).unwrap();
        decompose_with_fonts(&cs, Matrix::IDENTITY, &NoXObjects, &fonts)
    }

    fn texts(m: &PageObjects) -> Vec<&TextObject> {
        m.objects
            .iter()
            .filter_map(|o| match o {
                VectorObject::Text(t) => Some(t),
                _ => None,
            })
            .collect()
    }

    fn paths(m: &PageObjects) -> Vec<&PathObject> {
        m.objects
            .iter()
            .filter_map(|o| match o {
                VectorObject::Path(p) => Some(p),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_stroked_line_is_one_path_object_with_two_anchors() {
        let m = model(b"10 20 m 100 200 l S");
        let ps = paths(&m);
        assert_eq!(ps.len(), 1);
        let sp = &ps[0].subpaths;
        assert_eq!(sp.len(), 1);
        assert_eq!(sp[0].start, Point::new(10.0, 20.0));
        assert_eq!(
            sp[0].segments,
            vec![Segment::Line {
                to: Point::new(100.0, 200.0)
            }]
        );
        assert!(!sp[0].closed);
        assert!(ps[0].style.stroke && ps[0].style.fill.is_none());
    }

    #[test]
    fn re_is_a_closed_quad_and_fills_nonzero() {
        let m = model(b"10 10 80 40 re f");
        let ps = paths(&m);
        assert_eq!(ps.len(), 1);
        assert!(ps[0].is_quad());
        assert_eq!(ps[0].style.fill, Some(FillRule::NonZero));
        // page bbox of the rectangle
        assert_eq!(ps[0].page_bbox.min, Point::new(10.0, 10.0));
        assert_eq!(ps[0].page_bbox.max, Point::new(90.0, 50.0));
    }

    #[test]
    fn v_and_y_control_points_come_from_the_shared_primitives() {
        // `v`: first control is the current point (10,10).
        let m = model(b"10 10 m 20 30 40 50 v S");
        let ps = paths(&m);
        assert_eq!(
            ps[0].subpaths[0].segments[0],
            Segment::Cubic {
                c1: Point::new(10.0, 10.0),
                c2: Point::new(20.0, 30.0),
                to: Point::new(40.0, 50.0),
            }
        );
        // `y`: second control is the endpoint.
        let m2 = model(b"10 10 m 20 30 40 50 y S");
        assert_eq!(
            paths(&m2)[0].subpaths[0].segments[0],
            Segment::Cubic {
                c1: Point::new(20.0, 30.0),
                c2: Point::new(40.0, 50.0),
                to: Point::new(40.0, 50.0),
            }
        );
    }

    #[test]
    fn cm_transforms_the_captured_ctm_and_page_space() {
        // Scale by 2 then draw a unit line: page-space nodes are doubled.
        let m = model(b"2 0 0 2 5 5 cm 0 0 m 10 0 l S");
        let ps = paths(&m);
        let page = ps[0].page_subpaths();
        assert_eq!(page[0].start, Point::new(5.0, 5.0)); // (0,0)*2 + (5,5)
        assert_eq!(page[0].segments[0].end(), Point::new(25.0, 5.0)); // (10,0)*2+(5,5)
        // but the stored user-space nodes are untransformed
        assert_eq!(ps[0].subpaths[0].start, Point::new(0.0, 0.0));
    }

    #[test]
    fn q_q_restores_the_ctm_so_a_later_object_is_untransformed() {
        let m = model(b"q 3 0 0 3 0 0 cm 0 0 m 1 0 l S Q 0 0 m 1 0 l S");
        let ps = paths(&m);
        assert_eq!(ps.len(), 2);
        // first object scaled x3, second at identity
        assert_eq!(
            ps[0].page_subpaths()[0].segments[0].end(),
            Point::new(3.0, 0.0)
        );
        assert_eq!(
            ps[1].page_subpaths()[0].segments[0].end(),
            Point::new(1.0, 0.0)
        );
    }

    #[test]
    fn multiple_subpaths_and_close_operators() {
        // Two subpaths, the second closed by `s`.
        let m = model(b"0 0 m 10 0 l 0 0 m 5 5 l 10 0 l s");
        let ps = paths(&m);
        assert_eq!(ps.len(), 1);
        assert_eq!(ps[0].subpaths.len(), 2);
        assert!(ps[0].subpaths[1].closed, "s closes the last subpath");
    }

    #[test]
    fn h_closes_and_reopens_a_subpath() {
        let m = model(b"0 0 m 10 0 l 10 10 l h 20 20 l S");
        let ps = paths(&m);
        assert_eq!(ps[0].subpaths.len(), 2);
        assert!(ps[0].subpaths[0].closed);
        // the reopened subpath starts at the closed subpath's start (0,0)
        assert_eq!(ps[0].subpaths[1].start, Point::new(0.0, 0.0));
    }

    #[test]
    fn n_paints_nothing_but_is_still_a_selectable_object() {
        let m = model(b"0 0 m 10 10 l n");
        let ps = paths(&m);
        assert_eq!(ps.len(), 1);
        assert!(ps[0].style.is_invisible());
    }

    #[test]
    fn token_range_covers_construction_through_paint() {
        // tokens: 0:"10" 1:"20" 2:m 3:"100" 4:"200" 5:l 6:S
        let cs = ContentStream::parse(b"10 20 m 100 200 l S".to_vec()).unwrap();
        let m = decompose(&cs, Matrix::IDENTITY, &NoXObjects);
        let p = match &m.objects[0] {
            VectorObject::Path(p) => p,
            _ => panic!(),
        };
        assert_eq!(p.tokens.start, 0);
        assert_eq!(p.tokens.end, 7); // exclusive, past the S at index 6
        // and the byte span slices to the object's source text
        assert_eq!(p.bytes.slice(&cs.buf).unwrap(), b"10 20 m 100 200 l S");
    }

    #[test]
    fn text_object_is_bbox_and_range_only_and_flagged_approximate() {
        let m = model(b"BT /F1 12 Tf 72 700 Td (Hi) Tj ET");
        let texts = texts(&m);
        assert_eq!(texts.len(), 1);
        assert!(texts[0].approximate);
        // origin (72,700) inflated by the 12 pt font size
        assert!(texts[0].page_bbox.contains(Point::new(72.0, 700.0)));
    }

    /// The `Tf` operands are read from the STREAM, so the resource name and
    /// size are known even with no document behind the walk — but no
    /// typeface is claimed and no decoding is attempted, and the preview
    /// says which of those it is.
    #[test]
    fn without_a_font_resolver_the_preview_is_unavailable_not_empty() {
        let m = model(b"BT /F1 12 Tf 72 700 Td (Hi) Tj ET");
        let t = texts(&m).remove(0);
        assert_eq!(t.preview, TextPreview::Unavailable);
        let font = t.font.as_ref().expect("the /Tf is in the stream");
        assert_eq!(font.resource, "F1");
        assert_eq!(font.size, 12.0);
        // No resolver ⇒ no /BaseFont claim. `F1` is not evidence of a
        // typeface and must never be presented as one.
        assert_eq!(font.base_font, None);
    }

    /// A `BT`/`ET` that positions but never shows a string is `Empty` — a
    /// different fact from "pdfce did not look", and the two must not
    /// collapse.
    #[test]
    fn a_text_object_that_shows_nothing_is_empty_not_unavailable() {
        // A `Tj` with an empty string still records an origin (so the
        // object exists) but shows no codes.
        let m = model(b"BT /F1 12 Tf 72 700 Td () Tj ET");
        let t = texts(&m).remove(0);
        assert_eq!(t.preview, TextPreview::Empty);
    }

    /// With a resolver in scope the shown string decodes through the SAME
    /// §9.10.2 ladder `extract-text` climbs, for `Tj`, `TJ`, `'` and `"`
    /// alike — every string operand in the run is shown text.
    #[test]
    fn show_operators_decode_through_the_extract_font_ladder() {
        // `TJ`'s kerning numbers are positioning, not text: they contribute
        // nothing to the preview (no derived spaces — see TextPreview).
        let m = model_with_fonts(b"BT /F1 12 Tf 10 10 Td [(He) -120 (llo)] TJ ( there) Tj ET");
        let t = texts(&m).remove(0);
        match &t.preview {
            TextPreview::Decoded {
                text,
                truncated,
                lossy,
            } => {
                assert_eq!(text, "Hello there");
                assert!(!truncated);
                assert!(!lossy);
            }
            other => panic!("expected a decoded preview, got {other:?}"),
        }
        assert_eq!(
            t.font.as_ref().and_then(|f| f.base_font.clone()),
            Some("Helvetica".to_owned())
        );
    }

    /// A font whose encoding defeats the ladder must report
    /// `Undecodable` — never a row of replacement characters, which reads
    /// as a pdfce bug rather than as an honest "this cannot be read".
    #[test]
    fn a_font_whose_encoding_defeats_decoding_reports_undecodable() {
        // `Identity-H` with no `/ToUnicode` and an `Adobe-Identity-0`
        // descendant satisfies neither disjunct of §9.10.2's rung 3, so
        // every code reaches the failure clause (the same property
        // `fixtures/synthetic/text/identity-h-no-tounicode.pdf` pins for
        // extraction).
        let m = model_with_fonts(b"BT /Undecodable 12 Tf 10 10 Td <00480049> Tj ET");
        let t = texts(&m).remove(0);
        assert_eq!(t.preview, TextPreview::Undecodable);
        // The font is still named — knowing WHICH font cannot be read is
        // most of the value of the disclosure.
        assert_eq!(
            t.font.as_ref().and_then(|f| f.base_font.clone()),
            Some("NoUnicode".to_owned())
        );
    }

    /// The memory bound, asserted rather than trusted: a long string is cut
    /// at `MAX_TEXT_PREVIEW_CHARS` and SAYS it was cut.
    #[test]
    fn a_long_string_is_truncated_at_the_documented_cap_and_discloses_it() {
        let long = "A".repeat(MAX_TEXT_PREVIEW_CHARS * 4);
        let src = format!("BT /F1 12 Tf 10 10 Td ({long}) Tj ET");
        let m = model_with_fonts(src.as_bytes());
        let t = texts(&m).remove(0);
        match &t.preview {
            TextPreview::Decoded {
                text, truncated, ..
            } => {
                assert_eq!(text.chars().count(), MAX_TEXT_PREVIEW_CHARS);
                assert!(truncated, "a cut preview must disclose the cut");
            }
            other => panic!("expected a decoded preview, got {other:?}"),
        }
    }

    /// The font is the one in effect at the FIRST show operator, not the
    /// last — the object is identified by the run it starts with, which is
    /// the run the preview previews.
    #[test]
    fn the_captured_font_is_the_one_at_the_first_show_operator() {
        let m = model_with_fonts(b"BT /F1 12 Tf 10 10 Td (a) Tj /F2 30 Tf (b) Tj ET");
        let t = texts(&m).remove(0);
        let font = t.font.as_ref().expect("a font");
        assert_eq!(font.resource, "F1");
        assert_eq!(font.size, 12.0);
    }

    /// `q`/`Q` save and restore the font, because the font is part of the
    /// text state and therefore part of the graphics state (§9.3).
    #[test]
    fn q_q_restores_the_font_resource() {
        let m = model_with_fonts(
            b"/F2 30 Tf q /F1 12 Tf BT 10 10 Td (a) Tj ET Q BT 20 20 Td (b) Tj ET",
        );
        let ts = texts(&m);
        assert_eq!(ts.len(), 2);
        assert_eq!(
            ts[0].font.as_ref().map(|f| f.resource.clone()),
            Some("F1".to_owned())
        );
        // After `Q` the outer `/F2 30 Tf` is in effect again.
        assert_eq!(
            ts[1].font.as_ref().map(|f| f.resource.clone()),
            Some("F2".to_owned())
        );
        assert_eq!(ts[1].font.as_ref().map(|f| f.size), Some(30.0));
    }

    #[test]
    fn inline_image_is_one_image_object_bounded_by_the_unit_square_ctm() {
        // Scale 100x50, translate (10,20): the inline image fills that box.
        let m = model(b"100 0 0 50 10 20 cm BI /W 1 /H 1 /CS /G /BPC 8 ID \x00 EI");
        let imgs: Vec<_> = m
            .objects
            .iter()
            .filter_map(|o| match o {
                VectorObject::Image(i) => Some(i),
                _ => None,
            })
            .collect();
        assert_eq!(imgs.len(), 1);
        assert_eq!(imgs[0].source, ImageSource::Inline);
        assert_eq!(imgs[0].page_bbox.min, Point::new(10.0, 20.0));
        assert_eq!(imgs[0].page_bbox.max, Point::new(110.0, 70.0));
        // §8.9.7 Table 93: `/W`/`/H` are normalized to `/Width`/`/Height` by
        // the tokenizer, so the sample count is read with no resolver at all.
        assert_eq!(imgs[0].pixel_size, Some((1, 1)));
    }

    /// The sample count is `None`, not a guess, when the dictionary does not
    /// carry a usable `/Width`+`/Height` pair (§8.9.5 Table 89 requires
    /// both, as integers).
    #[test]
    fn a_malformed_inline_image_reports_no_pixel_size() {
        // `/H` absent: an unfiltered inline image with no computable length
        // still tokenizes (the scan finds `EI`), and the object is emitted
        // with an honest `None` rather than half a size.
        let m = model(b"100 0 0 50 10 20 cm BI /W 4 /CS /G /BPC 8 ID \x00 EI");
        let imgs: Vec<_> = m
            .objects
            .iter()
            .filter_map(|o| match o {
                VectorObject::Image(i) => Some(i),
                _ => None,
            })
            .collect();
        assert_eq!(imgs.len(), 1);
        assert_eq!(imgs[0].pixel_size, None);
    }

    #[test]
    fn do_image_and_form_classified_via_the_resolver() {
        struct Stub;
        impl XObjectResolver for Stub {
            fn classify(&self, name: &[u8]) -> Option<XObjectShape> {
                match name {
                    b"Im0" => Some(XObjectShape::Image {
                        pixel_size: Some((640, 480)),
                    }),
                    b"Fm0" => Some(XObjectShape::Form {
                        bbox: Bounds {
                            min: Point::new(0.0, 0.0),
                            max: Point::new(4.0, 2.0),
                        },
                        matrix: Matrix::IDENTITY,
                    }),
                    _ => None,
                }
            }
        }
        let cs = ContentStream::parse(b"1 0 0 1 5 5 cm /Im0 Do /Fm0 Do /Zz Do".to_vec()).unwrap();
        let m = decompose(&cs, Matrix::IDENTITY, &Stub);
        let imgs: Vec<_> = m
            .objects
            .iter()
            .filter_map(|o| match o {
                VectorObject::Image(i) => Some(i),
                _ => None,
            })
            .collect();
        assert_eq!(imgs.len(), 2);
        assert_eq!(imgs[0].source, ImageSource::XObject);
        // §8.9.5 Table 89's sample count travels with the classification.
        assert_eq!(imgs[0].pixel_size, Some((640, 480)));
        assert_eq!(imgs[1].source, ImageSource::Form);
        assert_eq!(imgs[1].page_bbox.max, Point::new(9.0, 7.0)); // (4,2)+(5,5)
        // A form has no samples (§8.10) — never `Some((0, 0))`.
        assert_eq!(imgs[1].pixel_size, None);
        assert_eq!(m.diagnostics.unresolved_xobject, 1); // /Zz
    }

    #[test]
    fn unbalanced_q_and_missing_current_point_are_counted_not_panicked() {
        let m = model(b"Q Q 10 20 l S");
        assert_eq!(m.diagnostics.unbalanced_q, 2);
        assert_eq!(m.diagnostics.segment_without_current, 1);
        assert!(paths(&m).is_empty());
    }

    #[test]
    fn a_lone_move_then_paint_emits_no_object() {
        let m = model(b"10 10 m S");
        assert!(paths(&m).is_empty());
    }
}
