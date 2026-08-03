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
//! **selectable-for-move/delete** objects carrying **bbox + token range
//! only** — not node-editable in the beta (dimensioning cares about path
//! geometry). A text object's bbox is a documented **approximation** (see
//! [`TextObject`]).
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

use crate::content::{ContentStream, ContentToken, ContentTokenKind};
use crate::graph::ObjectGraph;
use crate::object::{Dict, Object};
use crate::page_tree::Page;
use crate::span::ByteSpan;
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
    /// The defining-operator token range.
    pub tokens: TokenRange,
    /// The equivalent byte span.
    pub bytes: ByteSpan,
}

/// A text object (`BT`…`ET`) — selectable-for-move/delete, bbox only.
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
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextObject {
    /// Page-space approximate bounds (module docs).
    pub page_bbox: Bounds,
    /// Always `true` — the bbox is an origin-derived approximation.
    pub approximate: bool,
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
    Image,
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
            Some(b"Image") => Some(XObjectShape::Image),
            Some(b"Form") => Some(XObjectShape::Form {
                bbox: dict_rect(self.view, &stream.dict, b"BBox").unwrap_or(Bounds::EMPTY),
                matrix: dict_matrix(self.view, &stream.dict).unwrap_or(Matrix::IDENTITY),
            }),
            // Structural inference for a malformed missing /Subtype, matching
            // the renderer's Width+Height ⇒ image, BBox ⇒ form heuristic.
            _ => {
                if stream.dict.contains_key(b"Width") && stream.dict.contains_key(b"Height") {
                    Some(XObjectShape::Image)
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

/// Read a six-number `/Matrix` entry (Table 95) as a [`Matrix`].
fn dict_matrix(view: &DocumentView<'_>, dict: &Dict) -> Option<Matrix> {
    let items = view.resolve(dict.get(b"Matrix")?).as_array()?;
    let n: Vec<f64> = items.iter().filter_map(Object::as_number).collect();
    let [a, b, c, d, e, f] = <[f64; 6]>::try_from(n).ok()?;
    Some(Matrix::new(a, b, c, d, e, f))
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
    let resolver = DocumentXObjects {
        view,
        resources: &page.resources,
    };
    Ok(decompose(&content, initial, &resolver))
}

/// Decompose an already-tokenized content stream, with an explicit
/// XObject resolver and initial CTM.
///
/// This is the walk's true entry point: [`decompose_page`] is the
/// [`DocumentView`]-backed convenience over it, and unit tests / the fuzz
/// target call this directly with [`NoXObjects`] (or a stub) over a
/// [`ContentStream::parse`] result — no document at all required.
#[must_use]
pub fn decompose(
    content: &ContentStream,
    initial: Matrix,
    xobjects: &dyn XObjectResolver,
) -> PageObjects {
    let mut d = Decomposer::new(content, initial, xobjects);
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
/// the device colours, and the two text-state parameters the approximate
/// text bbox needs. Saved/restored by `q`/`Q` (§8.4.2), like the
/// renderer's [`crate::content`]-driven state.
#[derive(Debug, Clone, Copy)]
struct GState {
    ctm: Matrix,
    line_width: f64,
    fill_color: Rgb,
    stroke_color: Rgb,
    font_size: f64,
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

/// The in-progress text object (`BT`…`ET`).
struct TextAccum {
    token_start: usize,
    origins: Bounds,
    max_font_size: f64,
    text_matrix: Matrix,
    line_matrix: Matrix,
}

struct Decomposer<'a> {
    content: &'a ContentStream,
    xobjects: &'a dyn XObjectResolver,
    stack: Vec<GState>,
    gs: GState,
    path: Option<PathAccum>,
    text: Option<TextAccum>,
    objects: Vec<VectorObject>,
    diag: DecomposeDiagnostics,
    total_nodes: usize,
}

impl<'a> Decomposer<'a> {
    fn new(content: &'a ContentStream, initial: Matrix, xobjects: &'a dyn XObjectResolver) -> Self {
        Self {
            content,
            xobjects,
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
        // The one non-operator "operation": a complete inline image.
        if let ContentTokenKind::InlineImage { .. } = &operator.kind {
            self.emit_image(
                ImageSource::Inline,
                self.gs.ctm,
                unit_square(),
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
            b"q" => self.stack.push(self.gs),
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
                self.text = Some(TextAccum {
                    token_start: op_index,
                    origins: Bounds::EMPTY,
                    max_font_size: 0.0,
                    text_matrix: Matrix::IDENTITY,
                    line_matrix: Matrix::IDENTITY,
                });
            }
            b"ET" => self.end_text(op_index),

            // ---- text state / positioning the bbox approximation needs ----
            b"Tf" => {
                // `Tf name size`: the number operand is the size.
                if let Some(size) = nums.last().copied() {
                    self.gs.font_size = size;
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
            b"Tj" | b"TJ" | b"'" | b"\"" => self.record_text_origin(),

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

    /// Record a text-showing origin: the pen position (text-space origin
    /// mapped through the text matrix and the CTM) plus the current font
    /// size, for the approximate bbox.
    fn record_text_origin(&mut self) {
        let ctm = self.gs.ctm;
        let font_size = self.gs.font_size;
        if let Some(t) = self.text.as_mut() {
            let origin = ctm.map_point(t.text_matrix.map_point(Point::new(0.0, 0.0)));
            t.origins = t.origins.union_point(origin);
            if font_size > t.max_font_size {
                t.max_font_size = font_size;
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
        self.diag.text += 1;
        self.objects.push(VectorObject::Text(TextObject {
            page_bbox,
            approximate: true,
            tokens: TokenRange {
                start: t.token_start,
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
            Some(XObjectShape::Image) => {
                self.emit_image(
                    ImageSource::XObject,
                    self.gs.ctm,
                    unit_square(),
                    first,
                    op_index,
                );
            }
            Some(XObjectShape::Form { bbox, matrix }) => {
                let ctm = matrix.post_concat(self.gs.ctm);
                let corners = bounds_corners(bbox);
                self.emit_image(ImageSource::Form, ctm, corners, first, op_index);
            }
            None => self.diag.unresolved_xobject += 1,
        }
    }

    /// Emit an image/form object: `local_corners` are the four corners in
    /// the object's own space (unit square, or a form `/BBox`), mapped to
    /// page space by `ctm`.
    fn emit_image(
        &mut self,
        source: ImageSource,
        ctm: Matrix,
        local_corners: [Point; 4],
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

    fn model(src: &[u8]) -> PageObjects {
        let cs = ContentStream::parse(src.to_vec()).unwrap();
        decompose(&cs, Matrix::IDENTITY, &NoXObjects)
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
        let texts: Vec<_> = m
            .objects
            .iter()
            .filter_map(|o| match o {
                VectorObject::Text(t) => Some(t),
                _ => None,
            })
            .collect();
        assert_eq!(texts.len(), 1);
        assert!(texts[0].approximate);
        // origin (72,700) inflated by the 12 pt font size
        assert!(texts[0].page_bbox.contains(Point::new(72.0, 700.0)));
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
    }

    #[test]
    fn do_image_and_form_classified_via_the_resolver() {
        struct Stub;
        impl XObjectResolver for Stub {
            fn classify(&self, name: &[u8]) -> Option<XObjectShape> {
                match name {
                    b"Im0" => Some(XObjectShape::Image),
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
        assert_eq!(imgs[1].source, ImageSource::Form);
        assert_eq!(imgs[1].page_bbox.max, Point::new(9.0, 7.0)); // (4,2)+(5,5)
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
