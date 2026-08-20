//! # viewer — the page-view state machine and its geometry
//!
//! Everything about "which page am I looking at, and how big is it on
//! screen" lives here, deliberately separated from the egui widget code
//! in `main.rs`. The split exists for one concrete reason: **this module
//! is unit-testable and `main.rs` is not.** A windowed UI cannot be
//! exercised headlessly on a CI runner, but zoom-ladder arithmetic,
//! fit-scale derivation, page-index clamping and the raster-size ceiling
//! are exactly the parts where an off-by-one or a divide-by-zero would
//! show up as a user-visible bug — so they are pure functions with tests,
//! and `main.rs` is reduced to wiring.
//!
//! ## The view model
//!
//! [`ViewState`] carries three things:
//!
//! - `page_index` — 0-based into the flattened page vector from
//!   [`pdfce_core::page_tree::pages`]. The UI displays it 1-based; the
//!   conversion happens once, in `ui_text::page_nav_label`.
//! - `zoom` — the **effective** scale in device pixels per PDF user-space
//!   unit, which is precisely the `scale` argument
//!   [`pdfce_render::render_page`] takes. `1.0` is 72 DPI, i.e. "actual
//!   size" on a nominal 72-point-per-inch display.
//! - `fit` — whether `zoom` is a value the operator pinned
//!   ([`FitMode::None`]) or one derived from the viewport each frame
//!   ([`FitMode::Page`] / [`FitMode::Width`]). This is a *mode*, not a
//!   one-shot action: "Fit page" that stops fitting the moment the window
//!   is resized is the behaviour every viewer gets right and would be
//!   conspicuous to get wrong.
//!
//! ## Why the zoom ladder is a table, not a multiplier
//!
//! Repeatedly multiplying by, say, √2 produces zoom levels like 141%,
//! 199%, 281% — technically fine, but the operator can never get back to
//! a round number, and two different click sequences that "should" land
//! on 100% land on 99.6% and 100.4% instead. A fixed ladder of familiar
//! percentages ([`ZOOM_LADDER`]) makes zoom-in/zoom-out exactly
//! reversible and always lands somewhere nameable. Zoom values *off* the
//! ladder (from ctrl+scroll, or from a fit mode) are handled by taking
//! the next rung strictly above/below the current value, so the ladder
//! also acts as a "snap back to sanity" mechanism.
//!
//! ## The raster-size ceiling is a real constraint, not a formality
//!
//! `pdfce-render` refuses to allocate a pixmap with an edge over
//! [`pdfce_render::MAX_PIXMAP_EDGE`] (16,384 px — docs/ARCHITECTURE.md
//! §10.1's allocation guard). A letter page never comes close, but ISO
//! 32000-1 Annex C permits pages up to 14,400 units on an edge, and such
//! a page hits the ceiling at about 1.1× zoom. Rather than let the
//! operator zoom into an error message, [`max_zoom_for_page`] lowers the
//! ceiling per page and [`ViewState`] clamps against it — the zoom
//! buttons simply stop, which is self-explanatory in a way that
//! "requested raster size 115200x86400 is empty or exceeds
//! MAX_PIXMAP_EDGE" is not.

use eframe::egui::{Pos2, Rect};
use pdfce_core::page_tree::Page;
use pdfce_render::tiny_skia::{Point, Transform};

/// Lowest zoom the UI offers: 10%, enough to see a poster-sized page
/// whole.
pub const MIN_ZOOM: f32 = 0.10;

/// Highest zoom the UI offers, before the per-page raster ceiling is
/// applied: 800%, past which a screen shows a few glyphs at a time and
/// the pixmap is enormous.
pub const MAX_ZOOM: f32 = 8.0;

/// The zoom levels the +/− buttons step through. Ascending, and it
/// contains `1.0` so "actual size" is always reachable by stepping.
pub const ZOOM_LADDER: &[f32] = &[
    0.10, 0.25, 0.33, 0.50, 0.67, 0.75, 1.00, 1.25, 1.50, 2.00, 3.00, 4.00, 6.00, 8.00,
];

/// How `ViewState::zoom` is being decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FitMode {
    /// The operator pinned an explicit zoom; the viewport no longer
    /// influences it.
    None,
    /// Recompute each frame so the whole page is visible.
    #[default]
    Page,
    /// Recompute each frame so the page's full width is visible.
    Width,
}

/// Which page is shown, at what scale, and how that scale is chosen.
#[derive(Debug, Clone, Copy)]
pub struct ViewState {
    /// 0-based index into the flattened page vector.
    pub page_index: usize,
    /// Effective device pixels per PDF user-space unit — the exact
    /// value handed to [`pdfce_render::render_page`].
    pub zoom: f32,
    /// Whether `zoom` is pinned or derived from the viewport.
    pub fit: FitMode,
}

impl Default for ViewState {
    /// First-open defaults: page 1, fit-page.
    ///
    /// Fit-page rather than 100% is a deliberate choice. Opening at a
    /// raw 100% produces a wildly different first impression depending
    /// on the page size — a business card fills a thumb's worth of the
    /// window, an A0 poster overflows it — and both read as a bug even
    /// though nothing is wrong. Fit-page always shows the operator the
    /// thing they just opened.
    fn default() -> Self {
        Self {
            page_index: 0,
            zoom: 1.0,
            fit: FitMode::Page,
        }
    }
}

impl ViewState {
    /// Move to `index`, clamped into `0..page_count`.
    ///
    /// Clamping rather than erroring is right for a *view*: the only
    /// ways to get an out-of-range index are a keyboard repeat past the
    /// end and a page count that shrank, and in both cases the operator
    /// wants the nearest valid page, not a message.
    pub fn go_to_page(&mut self, index: usize, page_count: usize) {
        self.page_index = clamp_page_index(index, page_count);
    }

    /// Step one page toward the end, stopping at the last page.
    ///
    /// Saturating rather than wrapping: wrap-around page navigation
    /// silently teleports an operator from page 400 to page 1, which is
    /// disorienting and is not what any document reader does.
    pub fn next_page(&mut self, page_count: usize) {
        self.go_to_page(self.page_index.saturating_add(1), page_count);
    }

    /// Step one page toward the start, stopping at the first page.
    pub fn prev_page(&mut self, page_count: usize) {
        self.go_to_page(self.page_index.saturating_sub(1), page_count);
    }

    /// Pin the zoom to an explicit value, clamped to `[MIN_ZOOM, max]`,
    /// and drop out of any fit mode.
    ///
    /// `max` is the per-page ceiling from [`max_zoom_for_page`], passed
    /// in rather than recomputed so this stays a pure state transition
    /// with no page argument.
    pub fn set_zoom(&mut self, zoom: f32, max: f32) {
        self.zoom = clamp_zoom(zoom, max);
        self.fit = FitMode::None;
    }

    /// Multiply the current zoom (the ctrl+scroll path), clamped, and
    /// drop out of any fit mode.
    pub fn zoom_by(&mut self, factor: f32, max: f32) {
        self.set_zoom(self.zoom * factor, max);
    }

    /// Step to the next ladder rung above the current zoom.
    pub fn zoom_in(&mut self, max: f32) {
        self.set_zoom(ladder_step_up(self.zoom), max);
    }

    /// Step to the next ladder rung below the current zoom.
    pub fn zoom_out(&mut self, max: f32) {
        self.set_zoom(ladder_step_down(self.zoom), max);
    }

    /// Enter a fit mode. The zoom itself is recomputed by
    /// [`ViewState::apply_fit`] once the viewport size is known, which
    /// in immediate mode is not until the frame is being laid out.
    pub fn set_fit(&mut self, fit: FitMode) {
        self.fit = fit;
    }

    /// If a fit mode is active, recompute `zoom` from the viewport.
    /// A no-op under [`FitMode::None`], so it is safe (and intended) to
    /// call unconditionally every frame.
    pub fn apply_fit(&mut self, page_pts: (f32, f32), viewport: (f32, f32), max: f32) {
        if self.fit == FitMode::None {
            return;
        }
        self.zoom = clamp_zoom(fit_scale(page_pts, viewport, self.fit), max);
    }

    /// The zoom as a whole percentage, for the toolbar readout.
    ///
    /// Rounds rather than truncates so a fit scale of 0.99997 reads as
    /// `100%`, not `99%`.
    #[must_use]
    pub fn zoom_percent(&self) -> u32 {
        (self.zoom * 100.0).round().max(0.0) as u32
    }
}

/// Clamp a page index into `0..page_count`, mapping the empty-document
/// case to `0`.
///
/// Returning `0` for an empty document rather than panicking keeps the
/// "no pages" condition a *presentation* decision (the canvas shows
/// `ui_text::canvas_no_pages`) instead of a crash, which matters because
/// a valid PDF really can have `/Count 0`.
#[must_use]
pub fn clamp_page_index(index: usize, page_count: usize) -> usize {
    index.min(page_count.saturating_sub(1))
}

/// Clamp a zoom value into `[MIN_ZOOM, max]`, mapping NaN to `1.0`.
///
/// NaN is reachable in practice: a degenerate page whose CropBox has
/// zero width makes `viewport_width / page_width` infinite or NaN, and
/// an unclamped NaN would propagate into the render scale and then into
/// a pixmap size, where it becomes a much less obvious failure. Mapping
/// it to actual size fails visibly and harmlessly.
#[must_use]
pub fn clamp_zoom(zoom: f32, max: f32) -> f32 {
    if !zoom.is_finite() {
        return 1.0;
    }
    // `max` can legitimately fall below MIN_ZOOM for an absurdly large
    // page, in which case the ceiling must win — hence clamping to the
    // top first and the bottom second would be wrong; take the ceiling
    // last.
    zoom.max(MIN_ZOOM).min(max.max(f32::MIN_POSITIVE))
}

/// The next ladder rung strictly above `zoom`, or [`MAX_ZOOM`] if none
/// is (i.e. the caller is already at or past the top).
#[must_use]
pub fn ladder_step_up(zoom: f32) -> f32 {
    // `> zoom + epsilon` rather than `> zoom` so a value that is a
    // floating-point hair below a rung (0.9999999 vs 1.0) advances past
    // that rung instead of "stepping up" to a visually identical scale.
    let threshold = zoom + zoom.abs() * 1e-4;
    ZOOM_LADDER
        .iter()
        .copied()
        .find(|&rung| rung > threshold)
        .unwrap_or(MAX_ZOOM)
}

/// The next ladder rung strictly below `zoom`, or [`MIN_ZOOM`] if none
/// is.
#[must_use]
pub fn ladder_step_down(zoom: f32) -> f32 {
    let threshold = zoom - zoom.abs() * 1e-4;
    ZOOM_LADDER
        .iter()
        .copied()
        .rev()
        .find(|&rung| rung < threshold)
        .unwrap_or(MIN_ZOOM)
}

/// The scale at which `page_pts` fits `viewport` under `fit`.
///
/// Both arguments are in the same unit only by coincidence — `page_pts`
/// is PDF user-space units and `viewport` is egui logical points — and
/// the result is the ratio between them, which is exactly the "device
/// pixels per user-space unit" the renderer wants. (On a HiDPI display
/// egui's own `pixels_per_point` then multiplies again; that is handled
/// at the call site, not here, because it is a display property rather
/// than a document one.)
///
/// Returns `1.0` for a degenerate page or viewport rather than dividing
/// by zero. [`FitMode::None`] also returns `1.0`, though callers are
/// expected not to ask.
#[must_use]
pub fn fit_scale(page_pts: (f32, f32), viewport: (f32, f32), fit: FitMode) -> f32 {
    let (pw, ph) = page_pts;
    let (vw, vh) = viewport;
    if pw <= 0.0 || ph <= 0.0 || vw <= 0.0 || vh <= 0.0 {
        return 1.0;
    }
    match fit {
        FitMode::None => 1.0,
        FitMode::Width => vw / pw,
        // Fit-page is the *smaller* of the two ratios: satisfying the
        // tighter constraint necessarily satisfies the looser one, and
        // taking the larger would overflow the other axis.
        FitMode::Page => (vw / pw).min(vh / ph),
    }
}

/// The highest zoom at which this page still rasterizes within
/// [`pdfce_render::MAX_PIXMAP_EDGE`], capped at [`MAX_ZOOM`].
///
/// See the module docs for why this exists. Two subtleties:
///
/// - **`pixels_per_point` is part of the calculation.** The zoom the
///   operator sees is a *logical* scale (points per PDF unit); the raster
///   is made at `zoom × pixels_per_point` so it stays sharp on a HiDPI
///   display (see [`raster_scale`]). On a 2× display, therefore, every
///   page hits the pixmap ceiling at half the zoom it otherwise would —
///   omitting this factor is how a guard like this passes its tests and
///   still fails on the one machine that matters.
/// - **A one-pixel guard band is subtracted** before dividing, because
///   the renderer computes its pixmap edge with `ceil()`: a scale that
///   divides out to exactly the limit rounds *up* past it and is refused.
///
/// [`raster_scale`]: crate::viewer::raster_scale
#[must_use]
pub fn max_zoom_for_page(page_pts: (f32, f32), pixels_per_point: f32) -> f32 {
    let longest = page_pts.0.max(page_pts.1);
    let ppp = if pixels_per_point.is_finite() && pixels_per_point > 0.0 {
        pixels_per_point
    } else {
        1.0
    };
    if !longest.is_finite() || longest <= 0.0 {
        return MAX_ZOOM;
    }
    #[allow(
        clippy::cast_precision_loss,
        reason = "MAX_PIXMAP_EDGE is 16384; f32 is exact to 2^24" // ui-text-exempt: clippy lint justification, never displayed
    )]
    let ceiling = (pdfce_render::MAX_PIXMAP_EDGE - 1) as f32 / (longest * ppp);
    // `clamp` is safe here (MIN_ZOOM < MAX_ZOOM, neither is NaN, and
    // `ceiling` is finite because `longest` and `ppp` were both checked
    // above) — so clippy's `manual_clamp` suggestion is taken rather
    // than suppressed.
    ceiling.clamp(MIN_ZOOM, MAX_ZOOM)
}

/// The device-pixel scale to rasterize at for a given logical `zoom`.
///
/// `zoom` is points per PDF user-space unit — what the operator sees as
/// a percentage and what fit modes compute. The raster has to be made in
/// *pixels*, so it is multiplied by the display's `pixels_per_point`.
/// Getting this wrong is not a crash; it is a viewer that looks
/// permanently slightly blurry on every HiDPI laptop and perfectly sharp
/// on the developer's external monitor.
#[must_use]
pub fn raster_scale(zoom: f32, pixels_per_point: f32) -> f32 {
    let ppp = if pixels_per_point.is_finite() && pixels_per_point > 0.0 {
        pixels_per_point
    } else {
        1.0
    };
    zoom * ppp
}

/// A page's on-screen extent in PDF user-space units, with `/Rotate`
/// already applied (a 90°-rotated portrait page is landscape on screen).
///
/// Delegates to [`pdfce_render::page_device_geometry`] at scale `1.0`
/// rather than reading `page.crop_box` directly. That is the point: the
/// GUI's idea of how big a page is and the renderer's idea of how big
/// the pixmap will be come from **one** function, so they cannot drift
/// apart — a fit-page computed from an un-rotated CropBox against a
/// rotated raster is the classic version of this bug.
#[must_use]
pub fn page_extent_pts(page: &Page) -> (f32, f32) {
    let (w, h, _) = pdfce_render::page_device_geometry(page, 1.0);
    #[allow(
        clippy::cast_precision_loss,
        reason = "page edges are bounded by MAX_PIXMAP_EDGE" // ui-text-exempt: clippy lint justification, never displayed
    )]
    (w as f32, h as f32)
}

// ---------------------------------------------------------------------------
// Canvas-interaction geometry (Pass 12.0, `docs/ui_specs/pass-12.0-canvas-
// substrate.md` §2)
// ---------------------------------------------------------------------------
//
// Two distinct coordinate spaces, named here because the substrate and the
// authoring APIs live in different ones and a future Pass that conflates
// them would author geometry in the wrong frame (spec §2.1):
//
// - **Canvas space** — page-device points at zoom 1.0: Y-**down**, origin
//   top-left, `/Rotate` already resolved into a possibly-swapped
//   width/height. This is the space [`page_extent_pts`] measures and the
//   space the on-screen raster is drawn in. `screen_to_page`/`page_to_screen`
//   convert between the screen and this space; they carry NO rotation logic
//   (rotation is already baked into the `extent` they are handed — see
//   [`page_extent_pts`]).
// - **PDF user space** — Y-**up**, origin at the *un-rotated*
//   MediaBox/CropBox lower-left, exactly what an annotation `/Rect`, a
//   content-stream operand, or the Pass 9a object model expresses. The
//   second bridge, `canvas_to_pdf_space`/`pdf_space_to_canvas`, converts
//   between canvas space and this one by reusing — and inverting — the SAME
//   device transform [`pdfce_render::page_device_geometry`] computes to
//   rasterize the page, so the interaction geometry and the render agree by
//   construction rather than by two hand-derived rotation formulas quietly
//   drifting apart.

/// Map a screen point to **canvas space** (§2.1).
///
/// `image_rect` is the canvas Response's own `.rect` for this frame
/// (the rect the page raster occupies on screen); `extent` is
/// [`page_extent_pts`] for the current page (the rotated device
/// width/height); `zoom` is [`ViewState::zoom`]. The page raster is drawn
/// at `image_rect.min` scaled by `zoom`, so undoing that — subtract the
/// origin, divide by the zoom — is the whole of the arithmetic.
///
/// **No rotation branch lives here on purpose.** Rotation-correctness comes
/// entirely from `extent` already carrying the rotated width/height (see
/// [`page_extent_pts`]); adding a rotation-aware branch here as well would
/// double-apply it. The `extent` argument is consulted only to reject a
/// degenerate page (per the contract below) — the mapping itself is a pure
/// affine undo of the draw.
///
/// Returns [`Pos2::ZERO`] for a degenerate page or zoom (zero/negative/
/// non-finite `extent` or `zoom`), mirroring [`fit_scale`]/[`clamp_zoom`]'s
/// "fail to a finite, harmless value, never a NaN/panic" discipline: there
/// is no sensible canvas coordinate for a page with no area.
#[must_use]
pub fn screen_to_page(pos: Pos2, image_rect: Rect, extent: (f32, f32), zoom: f32) -> Pos2 {
    if !geometry_inputs_ok(extent, zoom) {
        return Pos2::ZERO;
    }
    Pos2::new(
        (pos.x - image_rect.min.x) / zoom,
        (pos.y - image_rect.min.y) / zoom,
    )
}

/// The exact inverse of [`screen_to_page`]: **canvas space** → screen.
///
/// Needed every frame by the live-preview overlay (a stored canvas-space
/// geometry must be projected back to the screen to be drawn) and, once
/// Pass 9a exists, to draw a hit-tested object's selection outline. Same
/// degenerate-input contract as [`screen_to_page`].
#[must_use]
pub fn page_to_screen(page_pt: Pos2, image_rect: Rect, extent: (f32, f32), zoom: f32) -> Pos2 {
    if !geometry_inputs_ok(extent, zoom) {
        return Pos2::ZERO;
    }
    Pos2::new(
        page_pt.x * zoom + image_rect.min.x,
        page_pt.y * zoom + image_rect.min.y,
    )
}

/// Whether the geometry inputs describe a real, finite page at a real
/// zoom — the shared degenerate-input guard for the screen⟷canvas bridge.
#[must_use]
fn geometry_inputs_ok(extent: (f32, f32), zoom: f32) -> bool {
    zoom.is_finite() && zoom > 0.0 && extent.0.is_finite() && extent.0 > 0.0 && extent.1 > 0.0
}

/// Convert a **canvas-space** point (§2.1) into genuine **PDF user space**
/// (§2.3) — the frame every `pdfce-core` authoring API consumes.
///
/// Implemented by inverting the SAME transform
/// [`pdfce_render::page_device_geometry`] computes to rasterize this page
/// at scale 1.0 (its third tuple element, a
/// [`pdfce_render::tiny_skia::Transform`]). Canvas space *is* that
/// transform's output space at scale 1.0, so its inverse is exactly the
/// canvas→user map, rotation and Y-flip included, with no second formula to
/// keep in sync (the geometry analogue of "reuse the renderer's own walk so
/// they agree by construction").
///
/// Returns `None` only for a genuinely non-invertible page transform (a
/// degenerate page). Callers decline the commit rather than author garbage
/// geometry, exactly as `add_markup_shape` already declines when there is
/// no current page.
#[must_use]
#[allow(
    dead_code,
    reason = "built and tested this Pass but called from no commit path yet (spec 2.3); Pass 6.1/8/12.M2/9c-min are the first consumers" // ui-text-exempt: clippy lint justification, never displayed
)]
pub fn canvas_to_pdf_space(point: Pos2, page: &Page) -> Option<Pos2> {
    let (_, _, ctm) = pdfce_render::page_device_geometry(page, 1.0);
    let inverse = ctm.invert()?;
    Some(apply_transform(&inverse, point))
}

/// The exact inverse of [`canvas_to_pdf_space`]: **PDF user space** →
/// **canvas space** (§2.3).
///
/// Needed by any future consumer that receives geometry already in PDF
/// space — the primary case being Pass 9a's object-model provider handing
/// back a hit-tested object's bounds in PDF space, which the selection
/// overlay must project to the screen via
/// `page_to_screen(pdf_space_to_canvas(bounds, page), ..)`. Returns `None`
/// under the same non-invertible-page condition as
/// [`canvas_to_pdf_space`], so the two bridges decline together.
#[must_use]
#[allow(
    dead_code,
    reason = "built and tested this Pass; first live consumer is Pass 9a's selection-outline projection (spec 2.3)" // ui-text-exempt: clippy lint justification, never displayed
)]
pub fn pdf_space_to_canvas(point: Pos2, page: &Page) -> Option<Pos2> {
    let (_, _, ctm) = pdfce_render::page_device_geometry(page, 1.0);
    // Guard on invertibility so the two directions accept/decline the same
    // pages; the forward map itself does not need the inverse, but a page
    // whose transform cannot round-trip has no well-defined canvas point.
    ctm.invert()?;
    Some(apply_transform(&ctm, point))
}

/// Apply a `tiny_skia` [`Transform`] to a single egui [`Pos2`].
///
/// One place the `Pos2` ⟷ `tiny_skia::Point` marshalling lives, so the two
/// bridge directions cannot marshal inconsistently.
#[must_use]
#[allow(
    dead_code,
    reason = "used only by the two PDF-space bridges, themselves not yet called from a commit path (spec 2.3)" // ui-text-exempt: clippy lint justification, never displayed
)]
fn apply_transform(transform: &Transform, point: Pos2) -> Pos2 {
    let mut mapped = [Point::from_xy(point.x, point.y)];
    transform.map_points(&mut mapped);
    Pos2::new(mapped[0].x, mapped[0].y)
}

#[cfg(test)]
#[allow(clippy::float_cmp, reason = "ladder rungs are exact f32 literals")] // ui-text-exempt: clippy lint justification, never displayed
mod tests {
    use super::*;

    // ---- page-index clamping -------------------------------------

    #[test]
    fn clamping_keeps_indices_inside_the_document() {
        assert_eq!(clamp_page_index(0, 5), 0);
        assert_eq!(clamp_page_index(4, 5), 4);
        assert_eq!(clamp_page_index(5, 5), 4);
        assert_eq!(clamp_page_index(usize::MAX, 5), 4);
        // A page-less document must clamp to 0, not underflow.
        assert_eq!(clamp_page_index(3, 0), 0);
    }

    #[test]
    fn page_stepping_saturates_at_both_ends() {
        let mut v = ViewState::default();
        v.next_page(3);
        assert_eq!(v.page_index, 1);
        v.next_page(3);
        v.next_page(3);
        v.next_page(3);
        assert_eq!(v.page_index, 2);
        v.prev_page(3);
        assert_eq!(v.page_index, 1);
        v.prev_page(3);
        v.prev_page(3);
        assert_eq!(v.page_index, 0);
    }

    #[test]
    fn stepping_an_empty_document_stays_at_zero() {
        let mut v = ViewState::default();
        v.next_page(0);
        assert_eq!(v.page_index, 0);
        v.prev_page(0);
        assert_eq!(v.page_index, 0);
    }

    // ---- zoom ladder ---------------------------------------------

    #[test]
    fn ladder_is_ascending_and_contains_actual_size() {
        assert!(ZOOM_LADDER.windows(2).all(|w| w[0] < w[1]));
        assert!(ZOOM_LADDER.contains(&1.0));
        assert_eq!(ZOOM_LADDER.first().copied(), Some(MIN_ZOOM));
        assert_eq!(ZOOM_LADDER.last().copied(), Some(MAX_ZOOM));
    }

    #[test]
    fn ladder_stepping_is_exactly_reversible() {
        // The property the fixed ladder exists to guarantee: in-then-out
        // returns to the same rung, for every rung.
        for &rung in ZOOM_LADDER {
            if rung < MAX_ZOOM {
                assert_eq!(ladder_step_down(ladder_step_up(rung)), rung);
            }
            if rung > MIN_ZOOM {
                assert_eq!(ladder_step_up(ladder_step_down(rung)), rung);
            }
        }
    }

    #[test]
    fn ladder_stepping_saturates_rather_than_wrapping() {
        assert_eq!(ladder_step_up(MAX_ZOOM), MAX_ZOOM);
        assert_eq!(ladder_step_up(999.0), MAX_ZOOM);
        assert_eq!(ladder_step_down(MIN_ZOOM), MIN_ZOOM);
        assert_eq!(ladder_step_down(0.001), MIN_ZOOM);
    }

    #[test]
    fn ladder_snaps_an_off_ladder_zoom_to_a_neighbouring_rung() {
        // Arriving from ctrl+scroll or a fit mode, 137% steps up to 150%
        // and down to 125% — never to 137.0001%.
        assert_eq!(ladder_step_up(1.37), 1.50);
        assert_eq!(ladder_step_down(1.37), 1.25);
    }

    #[test]
    fn a_hair_below_a_rung_still_steps_past_it() {
        // Guards the epsilon in ladder_step_up: without it, a fit scale
        // of 0.99999 would "step up" to 1.0, a visually identical zoom,
        // and the button would look broken.
        assert_eq!(ladder_step_up(0.999_99), 1.25);
        assert_eq!(ladder_step_down(1.000_01), 0.75);
    }

    // ---- zoom clamping -------------------------------------------

    #[test]
    fn zoom_clamps_to_the_configured_range() {
        assert_eq!(clamp_zoom(0.001, MAX_ZOOM), MIN_ZOOM);
        assert_eq!(clamp_zoom(100.0, MAX_ZOOM), MAX_ZOOM);
        assert_eq!(clamp_zoom(2.0, MAX_ZOOM), 2.0);
    }

    #[test]
    fn a_page_ceiling_below_the_floor_still_wins() {
        // An absurd page can push the raster ceiling under MIN_ZOOM. The
        // ceiling has to win, or the render would be refused at a zoom
        // the UI claims is legal.
        assert_eq!(clamp_zoom(1.0, 0.05), 0.05);
    }

    #[test]
    fn non_finite_zoom_falls_back_to_actual_size() {
        assert_eq!(clamp_zoom(f32::NAN, MAX_ZOOM), 1.0);
        assert_eq!(clamp_zoom(f32::INFINITY, MAX_ZOOM), 1.0);
    }

    #[test]
    fn zoom_percent_rounds_rather_than_truncating() {
        let mut v = ViewState::default();
        v.set_zoom(0.999_97, MAX_ZOOM);
        assert_eq!(v.zoom_percent(), 100);
        v.set_zoom(0.335, MAX_ZOOM);
        assert_eq!(v.zoom_percent(), 34);
    }

    // ---- fit scale -----------------------------------------------

    #[test]
    fn fit_width_uses_the_width_ratio_only() {
        // A tall page in a wide, short viewport: fit-width overflows
        // vertically on purpose (that is what scrolling is for).
        assert_eq!(
            fit_scale((100.0, 400.0), (300.0, 200.0), FitMode::Width),
            3.0
        );
    }

    #[test]
    fn fit_page_takes_the_tighter_of_the_two_constraints() {
        // width ratio 3.0, height ratio 0.5 -> 0.5, so the whole page
        // fits.
        assert_eq!(
            fit_scale((100.0, 400.0), (300.0, 200.0), FitMode::Page),
            0.5
        );
        // And symmetrically when height is the loose axis.
        assert_eq!(
            fit_scale((400.0, 100.0), (200.0, 300.0), FitMode::Page),
            0.5
        );
    }

    #[test]
    fn fit_page_result_never_overflows_either_axis() {
        // The property, checked over a spread of shapes rather than one
        // hand-picked case.
        for &(pw, ph) in &[(612.0, 792.0), (792.0, 612.0), (1.0, 5000.0), (5000.0, 1.0)] {
            for &(vw, vh) in &[(800.0, 600.0), (300.0, 1200.0), (50.0, 50.0)] {
                let s = fit_scale((pw, ph), (vw, vh), FitMode::Page);
                assert!(pw * s <= vw * 1.001);
                assert!(ph * s <= vh * 1.001);
            }
        }
    }

    #[test]
    fn degenerate_geometry_falls_back_to_actual_size() {
        assert_eq!(fit_scale((0.0, 100.0), (300.0, 300.0), FitMode::Page), 1.0);
        assert_eq!(fit_scale((100.0, 100.0), (0.0, 300.0), FitMode::Width), 1.0);
        assert_eq!(fit_scale((100.0, 100.0), (300.0, -1.0), FitMode::Page), 1.0);
    }

    #[test]
    fn fit_mode_survives_a_viewport_change_but_an_explicit_zoom_does_not() {
        // "Fit page" is a mode, not a one-shot: resizing the window
        // re-fits. Pinning a zoom ends that.
        let mut v = ViewState::default();
        v.set_fit(FitMode::Page);
        v.apply_fit((100.0, 100.0), (200.0, 200.0), MAX_ZOOM);
        assert_eq!(v.zoom, 2.0);
        v.apply_fit((100.0, 100.0), (400.0, 400.0), MAX_ZOOM);
        assert_eq!(v.zoom, 4.0);

        v.set_zoom(1.0, MAX_ZOOM);
        assert_eq!(v.fit, FitMode::None);
        v.apply_fit((100.0, 100.0), (800.0, 800.0), MAX_ZOOM);
        assert_eq!(v.zoom, 1.0);
    }

    #[test]
    fn zooming_by_a_factor_leaves_fit_mode() {
        let mut v = ViewState::default();
        v.set_fit(FitMode::Width);
        v.zoom_by(1.1, MAX_ZOOM);
        assert_eq!(v.fit, FitMode::None);
    }

    // ---- raster-size ceiling -------------------------------------

    #[test]
    fn a_normal_page_is_not_constrained_by_the_raster_ceiling() {
        // US Letter: 16383 / 792 ≈ 20.7, far above MAX_ZOOM.
        assert_eq!(max_zoom_for_page((612.0, 792.0), 1.0), MAX_ZOOM);
    }

    #[test]
    fn an_annex_c_maximum_page_is_constrained() {
        // 14,400 user units is ISO 32000-1 Annex C's largest page edge.
        let max = max_zoom_for_page((14_400.0, 14_400.0), 1.0);
        assert!(max < MAX_ZOOM);
        // And the ceiling must actually keep the raster legal: the
        // renderer ceil()s, so check the rounded-up edge too.
        let edge = (14_400.0_f32 * max).ceil() as u32;
        assert!(edge <= pdfce_render::MAX_PIXMAP_EDGE);
    }

    #[test]
    fn the_ceiling_is_what_actually_clamps_zoom_in_on_a_huge_page() {
        let page = (14_400.0, 14_400.0);
        let max = max_zoom_for_page(page, 1.0);
        let mut v = ViewState::default();
        for _ in 0..20 {
            v.zoom_in(max);
        }
        assert_eq!(v.zoom, max);
        assert!((page.0 * v.zoom).ceil() as u32 <= pdfce_render::MAX_PIXMAP_EDGE);
    }

    #[test]
    fn degenerate_page_extent_does_not_produce_a_nonsense_ceiling() {
        assert_eq!(max_zoom_for_page((0.0, 0.0), 1.0), MAX_ZOOM);
        assert_eq!(max_zoom_for_page((f32::NAN, 10.0), 1.0), MAX_ZOOM);
    }

    // ---- HiDPI ----------------------------------------------------

    #[test]
    fn raster_scale_multiplies_zoom_by_the_display_density() {
        assert_eq!(raster_scale(1.5, 2.0), 3.0);
        assert_eq!(raster_scale(1.5, 1.0), 1.5);
    }

    #[test]
    fn a_nonsense_pixels_per_point_is_treated_as_one() {
        // egui should never hand us these, but a zero here would render
        // a zero-size pixmap and a NaN would render nothing at all —
        // both far worse than ignoring a bad density.
        assert_eq!(raster_scale(2.0, 0.0), 2.0);
        assert_eq!(raster_scale(2.0, f32::NAN), 2.0);
        assert_eq!(
            max_zoom_for_page((14_400.0, 1.0), 0.0),
            max_zoom_for_page((14_400.0, 1.0), 1.0)
        );
    }

    #[test]
    fn the_raster_ceiling_accounts_for_display_density() {
        // The bug this pins: a guard computed in logical points passes
        // on a 1x developer monitor and blows the pixmap limit on a 2x
        // laptop, because the raster is twice as many pixels.
        let page = (14_400.0, 14_400.0);
        let max_1x = max_zoom_for_page(page, 1.0);
        let max_2x = max_zoom_for_page(page, 2.0);
        assert!(max_2x < max_1x);
        let edge = (page.0 * raster_scale(max_2x, 2.0)).ceil() as u32;
        assert!(edge <= pdfce_render::MAX_PIXMAP_EDGE);
    }

    // ---- canvas-interaction geometry (Pass 12.0 §2) --------------------

    use eframe::egui;
    use pdfce_core::object::{Dict, ObjId};
    use pdfce_core::page_tree::Rect as PageRect;

    /// A minimal page fixture: a `w`×`h` MediaBox/CropBox at the origin
    /// with the given clockwise `/Rotate`. Enough for the geometry
    /// functions, which read only `crop_box` and `rotate`.
    fn test_page(w: f64, h: f64, rotate: u16) -> Page {
        Page {
            id: ObjId::new(1, 0),
            resources: Dict::new(),
            media_box: PageRect::from_corners(0.0, 0.0, w, h),
            crop_box: PageRect::from_corners(0.0, 0.0, w, h),
            rotate,
            contents: Vec::new(),
            contents_unresolved: 0,
            contents_flattened: 0,
        }
    }

    /// Two `Pos2` are equal within a few `f32` ULPs of accumulated error.
    fn near(a: Pos2, b: Pos2) -> bool {
        (a.x - b.x).abs() <= 1e-3 && (a.y - b.y).abs() <= 1e-3
    }

    #[test]
    fn screen_page_round_trips_at_every_rotation() {
        // Property 1 (§2.2): page_to_screen ∘ screen_to_page == identity,
        // for the extent `page_extent_pts` actually returns at each of the
        // four legal rotations. The four angles test that NOTHING
        // rotation-specific leaks into these functions — they are agnostic
        // to rotation, because `extent` already carries it.
        for &rotate in &[0u16, 90, 180, 270] {
            let page = test_page(200.0, 300.0, rotate);
            let extent = page_extent_pts(&page);
            for &zoom in &[MIN_ZOOM, 0.5, 1.0, 2.5, MAX_ZOOM] {
                let display = egui::vec2(extent.0 * zoom, extent.1 * zoom);
                let rect = Rect::from_min_size(Pos2::new(37.0, 11.0), display);
                for &p in &[
                    Pos2::new(37.0, 11.0),
                    Pos2::new(100.0, 250.0),
                    rect.center(),
                    rect.max,
                ] {
                    let round =
                        page_to_screen(screen_to_page(p, rect, extent, zoom), rect, extent, zoom);
                    // Round-trip within a few ULPs at rotate={0,90,180,270},
                    // zoom across the ladder extremes, for several points.
                    assert!(near(round, p));
                }
            }
        }
    }

    #[test]
    fn screen_to_page_distance_scales_as_one_over_zoom() {
        // Property 2 (§2.2): a fixed SCREEN distance maps to a page-space
        // distance of screen_distance / zoom — the invariance Pass 12.M1's
        // screen-space snap tolerance will rely on.
        let extent = (200.0, 300.0);
        for &zoom in &[MIN_ZOOM, 0.5, 1.0, 3.0, MAX_ZOOM] {
            let rect = Rect::from_min_size(
                Pos2::new(5.0, 9.0),
                egui::vec2(extent.0 * zoom, extent.1 * zoom),
            );
            let a = screen_to_page(Pos2::new(50.0, 50.0), rect, extent, zoom);
            let b = screen_to_page(Pos2::new(90.0, 50.0), rect, extent, zoom);
            let page_dx = (b.x - a.x).abs();
            // A 40px screen span maps to a 40/zoom page span, for every zoom.
            assert!((page_dx - 40.0 / zoom).abs() <= 1e-3);
        }
    }

    #[test]
    fn screen_page_reject_degenerate_inputs_without_panicking() {
        // Property 4 (§2.2): zero/negative/non-finite geometry falls back
        // to a finite, harmless value rather than a NaN or a panic.
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(100.0, 100.0));
        assert_eq!(
            screen_to_page(Pos2::new(5.0, 5.0), rect, (0.0, 100.0), 1.0),
            Pos2::ZERO
        );
        assert_eq!(
            screen_to_page(Pos2::new(5.0, 5.0), rect, (100.0, 100.0), 0.0),
            Pos2::ZERO
        );
        assert_eq!(
            page_to_screen(Pos2::new(5.0, 5.0), rect, (100.0, -1.0), 1.0),
            Pos2::ZERO
        );
        assert_eq!(
            page_to_screen(Pos2::new(5.0, 5.0), rect, (100.0, 100.0), f32::NAN),
            Pos2::ZERO
        );
    }

    #[test]
    fn canvas_pdf_bridge_round_trips_at_every_rotation() {
        // §2.3 test plan: pdf_space_to_canvas ∘ canvas_to_pdf_space is the
        // identity at each rotation.
        for &rotate in &[0u16, 90, 180, 270] {
            let page = test_page(200.0, 300.0, rotate);
            for &p in &[
                Pos2::new(0.0, 0.0),
                Pos2::new(50.0, 80.0),
                Pos2::new(120.0, 240.0),
            ] {
                let user = canvas_to_pdf_space(p, &page).unwrap();
                let back = pdf_space_to_canvas(user, &page).unwrap();
                assert!(near(back, p), "rotate={rotate} p={p:?} back={back:?}"); // ui-text-exempt: test failure message, never displayed
            }
        }
    }

    #[test]
    fn pdf_space_to_canvas_agrees_with_the_renderer_by_construction() {
        // §2.3: the forward map must equal `page_device_geometry`'s own
        // (already pixel-tested) transform — this is what proves "agrees
        // with the renderer by construction", not merely self-consistent.
        for &rotate in &[0u16, 90, 180, 270] {
            let page = test_page(200.0, 300.0, rotate);
            let (_, _, ctm) = pdfce_render::page_device_geometry(&page, 1.0);
            for &p in &[
                Pos2::new(0.0, 0.0),
                Pos2::new(200.0, 0.0),
                Pos2::new(0.0, 300.0),
            ] {
                let via_bridge = pdf_space_to_canvas(p, &page).unwrap();
                let via_render = apply_transform(&ctm, p);
                assert!(near(via_bridge, via_render), "rotate={rotate} p={p:?}"); // ui-text-exempt: test failure message, never displayed
            }
        }
    }

    #[test]
    fn pdf_space_bridge_places_the_lower_left_corner_at_the_bottom() {
        // A concrete orientation check, un-rotated: PDF user-space (Y-up)
        // origin (0,0) is the page's lower-left, which in canvas space
        // (Y-down) is the BOTTOM-left — i.e. y == page height.
        let page = test_page(200.0, 300.0, 0);
        let ll = pdf_space_to_canvas(Pos2::new(0.0, 0.0), &page).unwrap();
        assert!(near(ll, Pos2::new(0.0, 300.0)));
        let ul = pdf_space_to_canvas(Pos2::new(0.0, 300.0), &page).unwrap();
        assert!(near(ul, Pos2::new(0.0, 0.0)));
    }
}
