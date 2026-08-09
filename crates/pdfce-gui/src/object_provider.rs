//! # object_provider — Pass 9a's concrete [`CanvasTargetProvider`]
//!
//! The thin `pdfce-gui` adapter that plugs `pdfce-core`'s read-only vector
//! object model (`pdfce_core::vector`) into the Pass 12.0 canvas substrate
//! as its hit-test target provider, replacing the shippable
//! [`crate::canvas::EmptyTargetProvider`]. Decision 011 §2.1: *"Pass 9a's
//! real provider is a thin `pdfce-gui` adapter that CALLS INTO
//! `pdfce-core`'s read-only object model (which stays GUI-free); the
//! adapter owns the trait impl, the object model owns none of it."*
//!
//! ## What lives here vs in core (GUI-core separation)
//!
//! ALL geometry — decomposition, hit-testing, marquee enclosure — is
//! `pdfce_core::vector`, in PDF user space. This module owns exactly two
//! things core cannot: (1) the **coordinate-space translation** between the
//! substrate's *canvas space* (`viewer`'s Y-down/rotated device convention)
//! and PDF user space, and (2) the [`TargetId`] ↔ object-index encoding.
//! The translation reuses the SAME transform
//! [`pdfce_render::page_device_geometry`] computes to rasterize the page (at
//! scale 1.0, which *is* canvas space), inverted — so selection geometry
//! and the render agree by construction, exactly as
//! `viewer::canvas_to_pdf_space` does (this provider is the batched,
//! object-model-backed sibling of that per-point bridge).
//!
//! ## Single-page by design
//!
//! The canvas shows one page at a time and only ever queries
//! `view.page_index`, so a provider is built for the **current page** and
//! rebuilt on page change / edit (`OpenDoc::ensure_object_provider`). A
//! query for any other `page_index` returns nothing — cheap, and it keeps
//! the decomposition off the hot path of a large document (only the visible
//! page is decomposed, not all N).
//!
//! ## TargetId encoding
//!
//! A [`TargetId`] is the object's index into
//! [`pdfce_core::vector::PageObjects::objects`] (paint order), cast to
//! `u64`. The substrate treats it opaquely (spec §4.1); only this provider
//! mints and decodes it.

use eframe::egui::{Pos2, Rect};
use pdfce_core::page_tree::Page;
use pdfce_core::vector::{
    Bounds, Handle, MarqueeMode, Matrix, PageObjects, Point, Segment, VectorObject, decompose_page,
    hit_test_point_all, hit_test_rect,
};
use pdfce_core::view::DocumentView;
use pdfce_render::page_device_geometry;
use pdfce_render::tiny_skia::{Point as SkPoint, Transform};

use crate::canvas::{CanvasTargetProvider, TargetId};

/// The fallback canvas-space slack a click may miss an object's edge by,
/// used ONLY when the caller cannot supply a live zoom (a non-finite or
/// non-positive zoom makes [`crate::canvas::screen_tolerance_to_page`]
/// return `0.0`, which would make selection impossible rather than merely
/// fussy).
///
/// Canvas space is the page's device space at zoom 1.0, where one unit is
/// one PDF point (the `page_device_geometry` scale-1.0 map is
/// distance-preserving — a pure rotation + Y-flip + translation), so this is
/// also, in effect, a ~3 pt page-space tolerance.
///
/// **This used to be the only tolerance**, applied at every zoom level, and
/// that was a bug: the pointer is divided by `zoom` before it reaches
/// [`ObjectModelProvider::hit_test`], so a constant canvas-space tolerance
/// is a *shrinking* on-screen catch radius — 1.5 px at 50% zoom, 0.75 px at
/// 25%. Objects were effectively unclickable whenever the operator zoomed
/// out to see a whole drawing. The live tolerance now arrives as a
/// parameter, derived from [`crate::canvas::SELECT_SCREEN_TOLERANCE_PX`].
const FALLBACK_SELECT_TOLERANCE: f64 = 3.0;

/// The object-model-backed target provider for one page (module docs).
pub struct ObjectModelProvider {
    /// The page this provider answers for; queries for any other index
    /// miss.
    page_index: usize,
    /// The decomposed objects, in PDF user space (paint order).
    objects: PageObjects,
    /// PDF user space → canvas space (the render device map at scale 1.0).
    to_canvas: Transform,
    /// Canvas space → PDF user space (the inverse), or `None` for a
    /// degenerate (non-invertible) page — then the provider declines every
    /// query rather than fabricate geometry.
    to_pdf: Option<Transform>,
}

impl ObjectModelProvider {
    /// Build a provider for `page` (at `page_index`) from `view`.
    ///
    /// Returns `None` only if the page's content cannot be decoded/tokenized
    /// (the same failure the renderer would hit) — the caller then falls
    /// back to [`crate::canvas::EmptyTargetProvider`], so selection simply
    /// finds nothing rather than breaking.
    ///
    /// # Pass a SESSION view, not the base document (decision 018)
    ///
    /// `OpenDoc::ensure_object_provider` passes `&session.view()`. Passing
    /// `&session.document().view()` — which is what this did through Pass
    /// 16.2 — decomposes the base revision, so hit-testing, marquee
    /// selection and the measure tool's snapping all address geometry the
    /// operator can no longer see and miss geometry they can. The raster
    /// and this provider must be built from the *same* view or the canvas
    /// shows one document and responds as another.
    #[must_use]
    pub fn build(view: &DocumentView<'_>, page: &Page, page_index: usize) -> Option<Self> {
        let objects = decompose_page(view, page, Matrix::IDENTITY).ok()?;
        let (_, _, to_canvas) = page_device_geometry(page, 1.0);
        Some(Self {
            page_index,
            objects,
            to_canvas,
            to_pdf: to_canvas.invert(),
        })
    }

    /// Construct directly from parts — the seam the headless unit tests use
    /// (a [`PageObjects`] plus an explicit canvas↔PDF transform), so the
    /// adapter logic is proven without a live `Document` or egui frame.
    #[cfg(test)]
    fn from_parts(page_index: usize, objects: PageObjects, to_canvas: Transform) -> Self {
        Self {
            page_index,
            objects,
            to_canvas,
            to_pdf: to_canvas.invert(),
        }
    }

    /// The current page's decomposed vector objects (Pass 12.M1 §10 ask #4).
    ///
    /// The **same-crate escape hatch** the snap engine (and the future Taubin
    /// best-fit circle) reads the already-decomposed objects through, so the
    /// snap query reuses the ONE decomposition this provider built for
    /// selection rather than a second `decompose_page` per frame — avoiding the
    /// exact "two decompositions quietly diverge" Z2 pattern decision 011 warns
    /// against (ui-spec §3.3). It does NOT widen the opaque
    /// [`crate::canvas::CanvasTargetProvider`] trait (the substrate stays
    /// opaque, spec §4.1); this is `pdfce-gui`-internal wiring on the CONCRETE
    /// provider. Snapping happens in **PDF user / page space** — the same frame
    /// `PageObjects` stores — so the caller converts the pointer with
    /// [`Self::canvas_to_pdf`]'s public sibling (`viewer::canvas_to_pdf_space`)
    /// or this provider's own transform before querying.
    #[allow(
        dead_code,
        reason = "Pass 12.M1 accessor; the first live consumer is the Pass 12.M2 measure tools' snap query + Taubin fit (ui-spec 3.3/10)" // ui-text-exempt: clippy lint justification, never displayed
    )]
    pub(crate) fn page_objects(&self) -> &PageObjects {
        &self.objects
    }

    /// Which subpath of `object` a canvas-space click lands on — the second
    /// selection level, for objects that hold a whole drawing view.
    ///
    /// A thin adapter over [`pdfce_core::vector::hit_test_subpaths`], exactly
    /// like [`Self::hit_test_all`] is over the per-object query: convert canvas
    /// space to PDF user space, apply the same degenerate-tolerance fallback,
    /// and let the core own the geometry. Sharing that fallback matters —
    /// without it a click could select an object and then find none of its
    /// subpaths, which reads as "the second level is broken" rather than "the
    /// tolerance was zero".
    ///
    /// Nearest first. Empty for a non-path object or an out-of-range index.
    pub(crate) fn subpath_hits(&self, object: usize, point: Pos2, tolerance: f64) -> Vec<usize> {
        let Some(pdf) = self.canvas_to_pdf(point) else {
            return Vec::new();
        };
        let tolerance = if tolerance.is_finite() && tolerance > 0.0 {
            tolerance
        } else {
            FALLBACK_SELECT_TOLERANCE
        };
        pdfce_core::vector::hit_test_subpaths(&self.objects, object, pdf, tolerance)
    }

    /// A subpath's bounds in **canvas** space, for drawing its outline.
    ///
    /// The object's own bounds would draw a rectangle around the entire view
    /// and tell the operator they had selected the whole thing again — which is
    /// the misunderstanding entering the object exists to resolve.
    pub(crate) fn subpath_bounds_canvas(&self, object: usize, subpath: usize) -> Option<Rect> {
        let b = pdfce_core::vector::subpath_bounds(&self.objects, object, subpath)?;
        self.pdf_bounds_to_canvas(b)
    }

    /// The page-space anchor sample points of the object at paint-order
    /// `index` (Pass 12.M2b — the circular best-fit tool's fit input, ui-spec
    /// §3.3). A path object contributes every anchor of every subpath, in
    /// **PDF user / page space** (the frame [`Self::page_objects`] stores and
    /// [`fit_circle_taubin`](pdfce_core::dimension::fit_circle_taubin)
    /// consumes); a text/image/form object (or an out-of-range index)
    /// contributes nothing (they carry no snap/fit node geometry in the beta,
    /// the same exclusion `snap.rs` applies). Reuses the ONE decomposition the
    /// selection provider already built — never a second `decompose_page`
    /// (the Z2 divergence guard, ui-spec §3.3).
    #[allow(
        dead_code,
        reason = "Pass 12.M2b circular-fit accessor; the live consumer is the MeasureCircular tool's fit set (ui-spec 3.3)" // ui-text-exempt: clippy lint justification, never displayed
    )]
    pub(crate) fn object_sample_points(&self, index: usize) -> Vec<Point> {
        match self.objects.objects.get(index) {
            Some(VectorObject::Path(path)) => path
                .page_subpaths()
                .iter()
                .flat_map(|sp| sp.anchors().collect::<Vec<_>>())
                .collect(),
            _ => Vec::new(),
        }
    }

    /// The anchors of ONE subpath, each paired with its **object-scoped**
    /// index — the Node rung's pick set (decision 028 §Q1).
    ///
    /// # Why not `object_sample_points`, which already returns anchors
    ///
    /// That one returns the whole object's flat list, and using it as a node
    /// pick set is the R83 hazard decision 028 found already shipped: on a
    /// measured CAD export one path object holds 6,681 anchors, so "the
    /// nearest anchor to the press" can easily belong to a subpath the
    /// operator is not pointing at, and nothing is drawn beforehand to say
    /// which. Scoping the pick set to the ENTERED subpath is what makes the
    /// grab predictable — the operator can only hit points they descended
    /// into and can see.
    ///
    /// # Why the index is object-scoped even though the set is subpath-scoped
    ///
    /// Decision 025 §1.3(b): the number pdfce shows and the number
    /// `pdfce-cli node-move --node N` addresses must be the same number.
    /// `vector::anchor_count` counts across the whole object, so the running
    /// offset is added here rather than letting the GUI invent a second
    /// numbering that would disagree with every other consumer (R92).
    ///
    /// Returns empty for a non-path object or an out-of-range index — the same
    /// exclusion `object_sample_points` applies, for the same reason (text and
    /// image objects are not node-editable, decision 011 §2.1).
    /// How many parts (subpaths) the path object at paint-order `index` has,
    /// or `0` for a non-path object (Pass 36.3).
    ///
    /// Exists so the "show points" draw can iterate an object's parts without
    /// reaching into `objects.objects` and re-doing the `VectorObject::Path`
    /// match at a call site whose job is painting. `0` for a non-path is the
    /// honest answer rather than an `Option`: a text run has no parts, and a
    /// loop over none of them is exactly the right amount of drawing.
    pub(crate) fn subpath_count(&self, index: usize) -> usize {
        match self.objects.objects.get(index) {
            Some(VectorObject::Path(path)) => path.page_subpaths().len(),
            _ => 0,
        }
    }

    pub(crate) fn subpath_node_points(&self, index: usize, subpath: usize) -> Vec<(usize, Point)> {
        let Some(VectorObject::Path(path)) = self.objects.objects.get(index) else {
            return Vec::new();
        };
        let subpaths = path.page_subpaths();
        // The running offset IS the object-scoped index of the target
        // subpath's first anchor, because `anchor_count` flattens the same
        // walk in the same order.
        let mut offset = 0usize;
        for (i, sp) in subpaths.iter().enumerate() {
            let anchors: Vec<Point> = sp.anchors().collect();
            if i == subpath {
                return anchors
                    .into_iter()
                    .enumerate()
                    .map(|(k, p)| (offset + k, p))
                    .collect();
            }
            offset += anchors.len();
        }
        Vec::new()
    }

    /// **Every** anchor of the path object at paint-order `index`, each with
    /// its object-scoped index — [`Self::subpath_node_points`] flattened
    /// across all subpaths.
    ///
    /// # Why the whole object and not one subpath
    ///
    /// A multi-node **selection** is object-scoped: nothing stops an
    /// operator Ctrl-clicking one anchor on a shape's outer subpath and
    /// another on a hole inside it, and `selected_nodes` holds both by their
    /// object-scoped index. A multi-node **drag** therefore has to look up
    /// positions across the whole object — asking per-subpath would mean
    /// the caller re-deriving which subpath each selected index falls in,
    /// which is exactly the offset arithmetic `subpath_node_points` exists
    /// to keep in one place.
    ///
    /// Empty for a non-path object, for the same reason
    /// [`Self::subpath_count`] returns `0`: a text run has no anchors, and a
    /// loop over none of them is the right amount of work.
    pub(crate) fn object_node_points(&self, index: usize) -> Vec<(usize, Point)> {
        let Some(VectorObject::Path(path)) = self.objects.objects.get(index) else {
            return Vec::new();
        };
        path.page_subpaths()
            .iter()
            .flat_map(|sp| sp.anchors())
            .enumerate()
            .collect()
    }

    /// The Bézier control points ("handles") of one subpath, each tagged with
    /// the **object-scoped index of the node it belongs to** and which side of
    /// that node it shapes.
    ///
    /// # Which handle belongs to which node
    ///
    /// A cubic segment carries two control points, and they belong to
    /// *different* nodes — this is the part that is easy to get backwards.
    /// Segment `k` runs from anchor `k` to anchor `k+1`, so its `c1` shapes
    /// the curve LEAVING anchor `k` and its `c2` shapes the curve ARRIVING at
    /// anchor `k+1`. That is exactly the split
    /// [`pdfce_core::vector::Handle`] names, and it is why the enum is worded
    /// by direction of travel rather than "first/second": first-and-second are
    /// properties of an operator, and an operator says nothing about which
    /// node the operator of the program has selected.
    ///
    /// Straight segments contribute nothing. pdfce refuses to invent a handle
    /// for a line — turning a line into a curve is a different operation with
    /// a different name — so a node with no curve on a side simply has no mark
    /// there, and the absence is stated in the readout rather than drawn as a
    /// ghost (decision 028 §Q2).
    ///
    /// `v`/`y` implicit control points need no special handling here: the
    /// decomposition already resolves them into explicit `c1`/`c2`
    /// (`Segment::Cubic`'s own doc comment), so this sees one uniform shape
    /// and the promotion-to-`c` happens far downstream in the planner.
    pub(crate) fn subpath_handle_points(
        &self,
        object: usize,
        subpath: usize,
    ) -> Vec<(usize, Handle, Point)> {
        let Some(VectorObject::Path(path)) = self.objects.objects.get(object) else {
            return Vec::new();
        };
        let subpaths = path.page_subpaths();
        let mut offset = 0usize;
        for (i, sp) in subpaths.iter().enumerate() {
            let anchors = sp.anchors().count();
            if i != subpath {
                offset += anchors;
                continue;
            }
            let mut out = Vec::new();
            for (k, seg) in sp.segments.iter().enumerate() {
                if let Segment::Cubic { c1, c2, .. } = *seg {
                    // `c1` shapes the curve leaving anchor k …
                    out.push((offset + k, Handle::Outgoing, c1));
                    // … and `c2` shapes the curve arriving at anchor k+1.
                    out.push((offset + k + 1, Handle::Incoming, c2));
                }
            }
            return out;
        }
        Vec::new()
    }

    /// The handle of `subpath` nearest `point` within `tolerance`, as
    /// `(node index, side)` — the Node rung's handle pick.
    ///
    /// # Why handles are hit-tested BEFORE nodes
    ///
    /// A handle sits close to its own node exactly when the curve is nearly
    /// flat there. If the node won ties, the handle would be unreachable
    /// precisely in the case where the operator most wants it — to pull a flat
    /// segment into a curve. Checking the smaller target first is the standard
    /// resolution and the one decision 028 §Q3 specifies.
    /// `point` is in **PDF page space**, unlike [`Self::nearest_node`]'s
    /// canvas-space input: the only caller is the drag classifier, which has
    /// already converted the press origin to page space to compute the drag's
    /// reference point. Converting back to canvas just to convert forward
    /// again would be two chances to disagree with itself for no benefit.
    pub(crate) fn nearest_handle(
        &self,
        object: usize,
        subpath: usize,
        pdf: Point,
        tolerance: f64,
    ) -> Option<(usize, Handle)> {
        let mut best: Option<((usize, Handle), f64)> = None;
        for (index, side, p) in self.subpath_handle_points(object, subpath) {
            if !p.is_finite() {
                continue;
            }
            let d = p.distance(pdf);
            if d <= tolerance && best.is_none_or(|(_, bd)| d < bd) {
                best = Some(((index, side), d));
            }
        }
        best.map(|(hit, _)| hit)
    }

    /// The object-scoped index of the anchor of `subpath` nearest `point`
    /// within `tolerance`, or `None` — the Node rung's pick.
    ///
    /// Takes canvas space and converts internally, exactly as
    /// [`Self::subpath_hits`] does, so the canvas→PDF frame conversion stays
    /// in the one place that owns it rather than being re-derived by each
    /// caller (R92). `tolerance` is in PDF units, already converted from
    /// screen pixels by [`canvas::screen_tolerance_to_page`](crate::canvas::screen_tolerance_to_page).
    ///
    /// Ties resolve to the lower index, matching
    /// [`vector_edit_tool::nearest_anchor`](crate::vector_edit_tool::nearest_anchor),
    /// so a point equidistant from two anchors picks the same one whether it
    /// was reached by clicking or by dragging.
    pub(crate) fn nearest_node(
        &self,
        object: usize,
        subpath: usize,
        point: Pos2,
        tolerance: f64,
    ) -> Option<usize> {
        let pdf = self.canvas_to_pdf(point)?;
        let mut best: Option<(usize, f64)> = None;
        for (index, p) in self.subpath_node_points(object, subpath) {
            if !p.is_finite() {
                continue;
            }
            let d = p.distance(pdf);
            if d <= tolerance && best.is_none_or(|(_, bd)| d < bd) {
                best = Some((index, d));
            }
        }
        best.map(|(index, _)| index)
    }

    /// Map a canvas-space point into PDF user space (the object model's
    /// frame), or `None` on a degenerate page.
    fn canvas_to_pdf(&self, p: Pos2) -> Option<Point> {
        let inv = self.to_pdf?;
        let mut pts = [SkPoint::from_xy(p.x, p.y)];
        inv.map_points(&mut pts);
        let out = pts[0];
        Some(Point::new(f64::from(out.x), f64::from(out.y)))
    }

    /// Map a PDF-space point into canvas space (for a selection outline).
    fn pdf_to_canvas(&self, p: Point) -> Pos2 {
        // Narrowing to f32 for egui; the object bounds are page geometry,
        // well within f32 range.
        #[allow(clippy::cast_possible_truncation)]
        let mut pts = [SkPoint::from_xy(p.x as f32, p.y as f32)];
        self.to_canvas.map_points(&mut pts);
        Pos2::new(pts[0].x, pts[0].y)
    }

    /// The canvas-space rect enclosing a PDF-space [`Bounds`] under the
    /// page transform (its four corners mapped, then bounded — the
    /// transform may rotate, so the axis-aligned canvas rect is the bound
    /// of the mapped quad).
    fn pdf_bounds_to_canvas(&self, b: Bounds) -> Option<Rect> {
        if b.is_empty() {
            return None;
        }
        let corners = [
            Point::new(b.min.x, b.min.y),
            Point::new(b.max.x, b.min.y),
            Point::new(b.max.x, b.max.y),
            Point::new(b.min.x, b.max.y),
        ];
        let mut rect: Option<Rect> = None;
        for c in corners {
            let p = self.pdf_to_canvas(c);
            rect = Some(match rect {
                None => Rect::from_min_max(p, p),
                Some(r) => r.union(Rect::from_min_max(p, p)),
            });
        }
        rect
    }

    /// The PDF-space bounding box of a canvas-space marquee rect (its four
    /// corners mapped back, then bounded).
    fn canvas_rect_to_pdf_bounds(&self, rect: Rect) -> Option<Bounds> {
        let corners = [
            rect.left_top(),
            rect.right_top(),
            rect.right_bottom(),
            rect.left_bottom(),
        ];
        let mut b = Bounds::EMPTY;
        for c in corners {
            b = b.union_point(self.canvas_to_pdf(c)?);
        }
        if b.is_empty() { None } else { Some(b) }
    }
}

impl CanvasTargetProvider for ObjectModelProvider {
    /// Every object under the pointer, front-most first — the required
    /// point query (`canvas::CanvasTargetProvider::hit_test`, the topmost
    /// one, is the trait's provided method over this, so the two cannot
    /// disagree; see that method's docs).
    ///
    /// A thin adapter, as the module docs promise: convert canvas space to
    /// PDF user space, resolve the tolerance, and hand both to
    /// [`pdfce_core::vector::hit_test_point_all`], which owns the geometry.
    fn hit_test_all(&self, page_index: usize, point: Pos2, tolerance: f64) -> Vec<TargetId> {
        if page_index != self.page_index {
            return Vec::new();
        }
        let Some(pdf) = self.canvas_to_pdf(point) else {
            return Vec::new();
        };
        // A degenerate tolerance (0.0 from a non-finite/zero zoom, or a
        // negative value) would silently make every click a miss. Fall back
        // to the fixed canvas-space value instead: fussy at low zoom is a
        // far better failure than "selection is broken".
        let tolerance = if tolerance.is_finite() && tolerance > 0.0 {
            tolerance
        } else {
            FALLBACK_SELECT_TOLERANCE
        };
        hit_test_point_all(&self.objects, pdf, tolerance)
            .into_iter()
            .map(|i| TargetId(i as u64))
            .collect()
    }

    fn hit_test_rect(&self, page_index: usize, rect: Rect) -> Vec<TargetId> {
        if page_index != self.page_index {
            return Vec::new();
        }
        let Some(bounds) = self.canvas_rect_to_pdf_bounds(rect) else {
            return Vec::new();
        };
        // Marquee default: fully-enclosed (decision 011 / Inkscape default,
        // R61). The provider is the trait's one place this convention is
        // decided (spec §4.2).
        hit_test_rect(&self.objects, bounds, MarqueeMode::Enclosed)
            .into_iter()
            .map(|i| TargetId(i as u64))
            .collect()
    }

    fn bounds(&self, page_index: usize, target: TargetId) -> Option<Rect> {
        if page_index != self.page_index {
            return None;
        }
        let obj = self.objects.objects.get(usize::try_from(target.0).ok()?)?;
        self.pdf_bounds_to_canvas(obj.page_bbox())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdfce_core::content::ContentStream;
    use pdfce_core::vector::{NoXObjects, decompose};

    /// A provider over a content stream, with an identity canvas transform
    /// (so canvas space == PDF space and the assertions read directly).
    fn provider(src: &[u8]) -> ObjectModelProvider {
        let cs = ContentStream::parse(src.to_vec()).expect("parse");
        let objects = decompose(&cs, Matrix::IDENTITY, &NoXObjects);
        ObjectModelProvider::from_parts(0, objects, Transform::identity())
    }

    #[test]
    fn click_inside_a_filled_rectangle_returns_its_target() {
        // One filled rectangle 10..90 square; a click at its centre hits it.
        let p = provider(b"10 10 80 80 re f");
        let hit = p.hit_test(0, Pos2::new(50.0, 50.0), 3.0);
        assert_eq!(hit, Some(TargetId(0)));
        // A click on empty canvas misses.
        assert_eq!(p.hit_test(0, Pos2::new(200.0, 200.0), 3.0), None);
        // A query for a different page misses regardless.
        assert_eq!(p.hit_test(1, Pos2::new(50.0, 50.0), 3.0), None);
    }

    /// The regression test for the zoom-inverted-tolerance bug: a click that
    /// misses a hairline stroke by 4 canvas units must MISS at a tight
    /// tolerance and HIT at a forgiving one.
    ///
    /// This is what makes the fix meaningful rather than cosmetic. Before it,
    /// the tolerance was hard-coded at 3.0 canvas units at every zoom, so at
    /// "Fit page" (~0.5x on a letter page in a typical window) the operator's
    /// real on-screen catch radius was ~1.5 px and thin geometry could not be
    /// clicked at all. The tolerance now arrives from the caller as
    /// `screen_tolerance_to_page(SELECT_SCREEN_TOLERANCE_PX, zoom)`, which
    /// GROWS in canvas units as zoom shrinks — keeping the on-screen radius
    /// constant.
    #[test]
    fn selection_tolerance_is_honoured_per_query_not_baked_in() {
        // A zero-width horizontal line at y=20; click 4 units above it.
        let p = provider(b"10 20 m 100 20 l S");
        let near_miss = Pos2::new(50.0, 24.0);

        // Tight tolerance (the old zoomed-out effective radius): a miss.
        assert_eq!(p.hit_test(0, near_miss, 1.5), None);
        // Forgiving tolerance (what a zoomed-out click now supplies): a hit.
        assert_eq!(p.hit_test(0, near_miss, 6.0), Some(TargetId(0)));

        // A degenerate tolerance must NOT silently disable selection — it
        // falls back to the fixed canvas-space value, so a click within
        // 3.0 units still lands.
        assert_eq!(p.hit_test(0, Pos2::new(50.0, 22.0), 0.0), Some(TargetId(0)));
        assert_eq!(
            p.hit_test(0, Pos2::new(50.0, 22.0), f64::NAN),
            Some(TargetId(0))
        );
    }

    /// The zoom-invariance law itself, end to end: the canvas-space tolerance
    /// a click supplies scales as `1 / zoom`, so the SCREEN-space catch radius
    /// is the same number of pixels at every zoom level.
    #[test]
    fn screen_tolerance_keeps_the_on_screen_catch_radius_constant() {
        use crate::canvas::{SELECT_SCREEN_TOLERANCE_PX, screen_tolerance_to_page};
        for zoom in [0.25_f32, 0.5, 1.0, 2.0, 4.0] {
            let canvas_tol = screen_tolerance_to_page(SELECT_SCREEN_TOLERANCE_PX, zoom);
            // Canvas units * zoom = screen px, by the same distance law
            // `viewer::screen_to_page` uses.
            let screen_px = canvas_tol * f64::from(zoom);
            assert!(
                (screen_px - f64::from(SELECT_SCREEN_TOLERANCE_PX)).abs() < 1e-6,
                "zoom {zoom}: on-screen radius drifted to {screen_px} px"
            );
        }
    }

    #[test]
    fn bounds_round_trips_the_object_bbox_into_canvas_space() {
        let p = provider(b"10 10 80 80 re f");
        let r = p.bounds(0, TargetId(0)).expect("bounds");
        // Under the identity transform the canvas rect is the PDF bbox.
        assert!((r.min.x - 10.0).abs() < 1e-3 && (r.min.y - 10.0).abs() < 1e-3);
        assert!((r.max.x - 90.0).abs() < 1e-3 && (r.max.y - 90.0).abs() < 1e-3);
        // A stale target id resolves to nothing (spec §4.4 — the substrate
        // silently drops it).
        assert_eq!(p.bounds(0, TargetId(99)), None);
    }

    #[test]
    fn marquee_encloses_only_fully_contained_objects() {
        // Two rectangles; a marquee over the first only encloses it.
        let p = provider(b"10 10 20 20 re f 200 200 20 20 re f");
        let hits = p.hit_test_rect(
            0,
            Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 100.0)),
        );
        assert_eq!(hits, vec![TargetId(0)]);
        // A marquee spanning both encloses both.
        let both = p.hit_test_rect(
            0,
            Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(300.0, 300.0)),
        );
        assert_eq!(both, vec![TargetId(0), TargetId(1)]);
        // Wrong page: nothing.
        assert!(p.hit_test_rect(1, Rect::EVERYTHING).is_empty());
    }

    #[test]
    fn a_text_object_is_selectable_by_its_bbox() {
        // A text object is bbox-only but still a valid target.
        let p = provider(b"BT /F1 12 Tf 40 40 Td (Hi) Tj ET");
        // The show origin (40,40) is inside the inflated text bbox.
        assert!(p.hit_test(0, Pos2::new(40.0, 40.0), 3.0).is_some());
    }

    /// Overlapping objects are all reported, front-most first, in CANVAS
    /// space — the input click-through cycling steps through. Without this
    /// the covered rectangle here is unselectable by any click.
    #[test]
    fn overlapping_objects_are_all_reported_front_most_first() {
        // A small filled rectangle painted first, then a big one over it.
        let p = provider(b"40 40 20 20 re f 0 0 100 100 re f");
        let hits = p.hit_test_all(0, Pos2::new(50.0, 50.0), 3.0);
        assert_eq!(hits, vec![TargetId(1), TargetId(0)]);
        // The topmost query is exactly that list's head.
        assert_eq!(p.hit_test(0, Pos2::new(50.0, 50.0), 3.0), Some(TargetId(1)));
        // Only the cover is under a point outside the covered object.
        assert_eq!(
            p.hit_test_all(0, Pos2::new(5.0, 5.0), 3.0),
            vec![TargetId(1)]
        );
        // A miss is an empty list, and a wrong page is too.
        assert!(p.hit_test_all(0, Pos2::new(500.0, 500.0), 3.0).is_empty());
        assert!(p.hit_test_all(1, Pos2::new(50.0, 50.0), 3.0).is_empty());
    }

    /// The tolerance fallback applies to the all-hits query as well: a
    /// degenerate tolerance must not silently make cycling find nothing
    /// when plain selection would still have found something.
    #[test]
    fn a_degenerate_tolerance_falls_back_for_the_all_hits_query_too() {
        let p = provider(b"10 20 m 100 20 l S");
        let near = Pos2::new(50.0, 22.0);
        assert_eq!(p.hit_test_all(0, near, 0.0), vec![TargetId(0)]);
        assert_eq!(p.hit_test_all(0, near, f64::NAN), vec![TargetId(0)]);
    }

    #[test]
    fn page_objects_feeds_the_snap_engine_from_the_one_decomposition() {
        // The §10 ask #4 accessor: the snap engine reads the provider's
        // already-decomposed objects (no second `decompose_page`) and resolves
        // a query in the same PDF/page space `PageObjects` stores.
        use pdfce_core::vector::{Point, SnapConfig, SnapKind, snap_candidates};
        let p = provider(b"10 20 m 100 20 l S");
        let model = p.page_objects();
        let cands = snap_candidates(Point::new(11.0, 21.0), &SnapConfig::new(5.0), model);
        assert_eq!(cands[0].kind, SnapKind::Endpoint);
        assert_eq!(cands[0].point, Point::new(10.0, 20.0));
    }
}

/// The Node rung's pick sets: which points belong to which part, and which
/// handle belongs to which node.
///
/// Separate from the module's main test block because these answer a
/// different question — not "does a click find the object" but "does the
/// index the operator sees mean what `node-move --node N` means".
#[cfg(test)]
mod node_rung_tests {
    use super::*;
    use pdfce_core::content::ContentStream;
    use pdfce_core::vector::{NoXObjects, decompose};

    fn provider(src: &[u8]) -> ObjectModelProvider {
        let cs = ContentStream::parse(src.to_vec()).expect("parse");
        let objects = decompose(&cs, Matrix::IDENTITY, &NoXObjects);
        ObjectModelProvider::from_parts(0, objects, Transform::identity())
    }

    /// **Node indices stay OBJECT-scoped across a subpath boundary.**
    ///
    /// This is decision 025 §1.3(b) made testable. The pick set is scoped to
    /// one part, but the numbering is not — because the number pdfce shows and
    /// the number `pdfce-cli node-move --node N` addresses have to be the same
    /// number. A subpath-scoped index would restart at 0 on the second part
    /// and quietly address a point in the first.
    #[test]
    fn the_second_parts_points_keep_counting_from_the_first() {
        // Two parts of two anchors each: indices 0,1 then 2,3.
        let p = provider(b"0 0 m 10 0 l 100 5 m 110 5 l S");
        let first: Vec<usize> = p
            .subpath_node_points(0, 0)
            .into_iter()
            .map(|(i, _)| i)
            .collect();
        let second: Vec<usize> = p
            .subpath_node_points(0, 1)
            .into_iter()
            .map(|(i, _)| i)
            .collect();
        assert_eq!(first, vec![0, 1]);
        assert_eq!(
            second,
            vec![2, 3],
            "the second part must continue the object's numbering, not restart"
        );
    }

    /// The pick set contains ONLY the named part's points.
    ///
    /// The whole reason the rung exists: a measured CAD object holds 6,681
    /// anchors, and offering all of them as a grab target is what made the
    /// old ungated gesture unpredictable.
    #[test]
    fn a_parts_pick_set_excludes_every_other_part() {
        let p = provider(b"0 0 m 10 0 l 100 5 m 110 5 l S");
        let pts: Vec<Point> = p
            .subpath_node_points(0, 1)
            .into_iter()
            .map(|(_, q)| q)
            .collect();
        assert_eq!(pts.len(), 2);
        assert!(
            pts.iter().all(|q| q.x >= 100.0),
            "part 1's pick set must not contain part 0's points: {pts:?}"
        );
    }

    /// **A cubic's two control points belong to DIFFERENT nodes** — the thing
    /// most likely to be implemented backwards.
    ///
    /// Segment k runs from anchor k to anchor k+1, so `c1` shapes the curve
    /// LEAVING anchor k and `c2` shapes the curve ARRIVING at anchor k+1.
    /// Assigning both to one node would look plausible, draw two handles in
    /// roughly the right place, and make every handle drag move the wrong end
    /// of the curve.
    #[test]
    fn a_cubics_two_handles_belong_to_the_nodes_at_its_two_ends() {
        // m(0,0) then c with c1=(10,40) c2=(60,40) to=(70,0).
        // Anchors: 0 -> (0,0), 1 -> (70,0).
        let p = provider(b"0 0 m 10 40 60 40 70 0 c S");
        let hs = p.subpath_handle_points(0, 0);
        assert_eq!(hs.len(), 2, "one cubic contributes exactly two handles");

        let outgoing = hs
            .iter()
            .find(|(_, s, _)| *s == Handle::Outgoing)
            .expect("c1");
        assert_eq!(outgoing.0, 0, "c1 shapes the curve LEAVING anchor 0");
        assert_eq!(outgoing.2, Point::new(10.0, 40.0));

        let incoming = hs
            .iter()
            .find(|(_, s, _)| *s == Handle::Incoming)
            .expect("c2");
        assert_eq!(incoming.0, 1, "c2 shapes the curve ARRIVING at anchor 1");
        assert_eq!(incoming.2, Point::new(60.0, 40.0));
    }

    /// **A straight segment contributes no handle, and none is invented.**
    ///
    /// pdfce refuses to turn a line into a curve without being asked, so the
    /// absence must show up as nothing drawn — not as a placeholder sitting on
    /// the node, which would advertise an edit that will be refused.
    #[test]
    fn a_straight_part_has_no_handles_at_all() {
        let p = provider(b"0 0 m 10 0 l 20 0 l S");
        assert!(p.subpath_handle_points(0, 0).is_empty());
    }

    /// `v` and `y` resolve to explicit control points before they get here.
    ///
    /// Worth pinning because the GUI would otherwise need to know about the
    /// short spellings, and getting `v` (first control = current point) and
    /// `y` (second control = endpoint) confused is the classic error in this
    /// operator family.
    #[test]
    fn the_short_curve_spellings_still_yield_two_handles() {
        // `v`: c1 is implicitly the current point (0,0), c2 = (60,40).
        let p = provider(b"0 0 m 60 40 70 0 v S");
        let hs = p.subpath_handle_points(0, 0);
        assert_eq!(hs.len(), 2, "`v` is a cubic and has both handles resolved");
        let outgoing = hs
            .iter()
            .find(|(_, s, _)| *s == Handle::Outgoing)
            .expect("c1");
        assert_eq!(
            outgoing.2,
            Point::new(0.0, 0.0),
            "`v`'s first control point IS the current point"
        );
    }

    /// A handle grab resolves to the node it belongs to, not to the nearest
    /// node in space.
    #[test]
    fn grabbing_a_handle_names_its_own_node() {
        let p = provider(b"0 0 m 10 40 60 40 70 0 c S");
        // Press right on c2 = (60,40), which is far nearer anchor 1 (70,0)
        // than anchor 0 — and is c2, so it must report node 1 / Incoming.
        let hit = p.nearest_handle(0, 0, Point::new(60.0, 40.0), 2.0);
        assert_eq!(hit, Some((1, Handle::Incoming)));
    }

    /// An out-of-range part yields nothing rather than panicking or wrapping.
    #[test]
    fn an_out_of_range_part_yields_no_points_or_handles() {
        let p = provider(b"0 0 m 10 0 l S");
        assert!(p.subpath_node_points(0, 9).is_empty());
        assert!(p.subpath_handle_points(0, 9).is_empty());
    }
}
