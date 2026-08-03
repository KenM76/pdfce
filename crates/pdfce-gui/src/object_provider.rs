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
    Bounds, MarqueeMode, Matrix, PageObjects, Point, VectorObject, decompose_page,
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
