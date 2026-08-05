//! # vector_edit_tool — Pass 9c-min object-edit gesture state (decision 011 §2.5)
//!
//! The `pdfce-gui`-side state and pure geometry for the
//! [`CanvasTool::VectorEdit`](crate::canvas::CanvasTool::VectorEdit) tool:
//! click-select, drag-to-move, drag-a-node, Delete-to-remove. The actual
//! surgery is `pdfce-core`'s ([`EditSession::move_object`](pdfce_core::edit::EditSession::move_object)
//! / `move_node` / `delete_object`) — this module owns ONLY the transient
//! drag bookkeeping and the two headless-testable decisions the gesture
//! needs (is this drag a node-drag or an object-move? where did it start?),
//! keeping `main.rs`'s per-frame handler thin (the same split as
//! `canvas.rs`/`viewer.rs` exist for: `main.rs` is not headlessly testable,
//! these are).
//!
//! All geometry is in **PDF user / page space** (the frame the object model
//! and the snap engine share), plain `f64` — egui-free, so the drag
//! classification is unit-tested here.

use pdfce_core::vector::{Handle, Point};

/// The radius **in screen pixels** within which a drag beginning near a
/// selected object's anchor is treated as a **node drag** rather than a
/// whole-object move.
///
/// # Why this became a screen measure
///
/// It was `6.0` in PAGE space, with a doc comment calling that "honest and
/// simple" and zoom-invariance "a follow-up refinement". Page-space is the
/// wrong frame for a grab radius, for the same reason
/// [`canvas::SELECT_SCREEN_TOLERANCE_PX`](crate::canvas::SELECT_SCREEN_TOLERANCE_PX)
/// gives: what the operator is aiming with is a pointer on a screen, so a
/// fixed page radius means the target shrinks as they zoom out and swells as
/// they zoom in. Every other canvas tolerance in pdfce already converts
/// through [`canvas::screen_tolerance_to_page`](crate::canvas::screen_tolerance_to_page);
/// this was the one that did not.
///
/// Swelling is the dangerous direction, and it is specific to this project's
/// files. A measured CAD export puts **6,681 anchors in one path object**, and
/// this grab searches the WHOLE object's anchors — so at high zoom a page-space
/// radius sweeps up many anchors from subpaths the operator is not pointing at,
/// and the nearest silently wins with nothing drawn beforehand to say which one
/// is about to move.
///
/// # Widened 6 → 10 px in Pass 26.2, and why that is now the safe direction
///
/// It was matched to `SELECT_SCREEN_TOLERANCE_PX` (6 px) on the argument that
/// "a node grab that loses to an object move costs one undo, while a node grab
/// that wins when the operator meant to move the object edits geometry they
/// were not looking at."
///
/// That argument was sound when a node grab could fire from ANY selected path
/// object, against its whole flat anchor list, with nothing drawn to say which
/// point was about to move. Two things have changed since. The grab is now
/// scoped to the anchors of the ONE part the operator has entered, not the
/// object's 6,681. And Pass 36.3 draws every one of those points, so the
/// operator can see exactly what they are aiming at before they press.
///
/// With the target visible and the candidate set small, the risk the tight
/// radius was protecting against is largely gone, and what remains is the
/// operator's actual complaint: *"it is still hard to pick endpoints."* A grab
/// radius slightly LARGER than the 6 px mark is the ordinary vector-editor
/// arrangement — you aim at a square and are forgiven for missing it by a
/// pixel or two.
///
/// Still deliberately smaller than a fingertip: at 10 px two adjacent anchors
/// of a dense CAD subpath can both be in range, and `classify_drag` takes the
/// nearest, which is the one under the pointer.
pub const NODE_GRAB_SCREEN_TOLERANCE_PX: f32 = 10.0;

/// An in-progress vector-edit drag (session/view state, exactly like the
/// measure tool's per-page state — never itself an edit; the edit is the one
/// `EditSession` command committed on release).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VectorDrag {
    /// The object being edited (its paint-order index / `TargetId`).
    pub object_index: usize,
    /// `Some(node)` when this is a node drag (the anchor's index in
    /// decomposition order); `None` for a whole-object move.
    pub node: Option<usize>,
    /// `Some((node, side))` when this drag grabbed a Bézier HANDLE rather than
    /// the on-curve node — it moves a control point and leaves the node put.
    ///
    /// Set independently of [`Self::node`] and takes precedence over it: the
    /// two grab zones overlap, and a handle is closest to its own node exactly
    /// when the curve is nearly flat there, so a node-wins rule would make the
    /// handle unreachable precisely when it is most wanted (decision 028 §Q3).
    pub handle: Option<(usize, Handle)>,
    /// The page-space point the drag started at (for the move delta, or a
    /// node drag's reference).
    pub start: Point,
}

impl VectorDrag {
    /// The page-space displacement from the drag start to `current` — the
    /// `(dx, dy)` a whole-object move commits.
    #[must_use]
    pub fn delta(&self, current: Point) -> (f64, f64) {
        (current.x - self.start.x, current.y - self.start.y)
    }
}

/// Classify a drag that started at `start` (page space) over object
/// `object_index`, given the object's anchor points `anchors` (page space,
/// decomposition order): a **node drag** on the nearest anchor within
/// `tol_page`, else a whole-object **move**.
///
/// `tol_page` is passed in rather than read from a constant so the caller can
/// convert [`NODE_GRAB_SCREEN_TOLERANCE_PX`] at the CURRENT zoom — the radius
/// depends on view state this pure function must not know about.
///
/// This is the one non-trivial decision of the tool — that a drag which
/// begins on an anchor grabs the node, and otherwise moves the object — and
/// it is pure, so it is tested here rather than in the live handler.
#[must_use]
pub fn classify_drag(
    object_index: usize,
    start: Point,
    anchors: &[Point],
    tol_page: f64,
) -> VectorDrag {
    let node = nearest_anchor(start, anchors, tol_page);
    VectorDrag {
        object_index,
        node,
        handle: None,
        start,
    }
}

/// The index of the anchor nearest `query` within `tol` page-space units, or
/// `None`. Ties resolve to the lower index (deterministic). Non-finite
/// anchors are skipped (they cannot be a grab target).
#[must_use]
pub fn nearest_anchor(query: Point, anchors: &[Point], tol: f64) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for (i, &a) in anchors.iter().enumerate() {
        if !a.is_finite() {
            continue;
        }
        let d = query.distance(a);
        if d <= tol && best.is_none_or(|(_, bd)| d < bd) {
            best = Some((i, d));
        }
    }
    best.map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_drag_starting_on_an_anchor_is_a_node_drag() {
        let anchors = [Point::new(0.0, 0.0), Point::new(100.0, 100.0)];
        // Start 2 pt from the second anchor: within tolerance → node 1.
        let d = classify_drag(7, Point::new(101.0, 101.5), &anchors, 6.0);
        assert_eq!(d.object_index, 7);
        assert_eq!(d.node, Some(1));
    }

    #[test]
    fn a_drag_starting_away_from_anchors_is_a_move() {
        let anchors = [Point::new(0.0, 0.0), Point::new(100.0, 100.0)];
        let d = classify_drag(3, Point::new(50.0, 50.0), &anchors, 6.0);
        assert_eq!(d.node, None);
        // The move delta is current − start.
        assert_eq!(d.delta(Point::new(60.0, 45.0)), (10.0, -5.0));
    }

    /// **The grab radius is a SCREEN measure, so it must not change what it
    /// catches as the operator zooms.**
    ///
    /// The bug this pins: `NODE_GRAB_TOLERANCE` was a fixed page-space `6.0`,
    /// so the radius swelled with zoom. On this project's own files that is
    /// not a nicety — one measured CAD object holds 6,681 anchors and this
    /// grab searches all of them, so a swollen radius sweeps up nodes from
    /// subpaths the operator is not pointing at and the nearest silently wins.
    ///
    /// Expressed as "the same screen-space aim picks the same node at any
    /// zoom", because that is the property the operator actually experiences.
    /// A test asserting a particular page distance would restate the
    /// implementation instead.
    #[test]
    fn the_node_grab_catches_the_same_node_at_every_zoom() {
        // The anchors are far apart in PAGE space so they stay far apart on
        // SCREEN at every zoom tested. That constraint is the test getting
        // corrected by its own first run: with anchors 10 pt apart, zoom 0.25
        // puts them 2.5 px apart on screen, where a 6 px grab genuinely cannot
        // tell them apart and picked the wrong one. That is not a defect — it
        // is what "zoomed too far out to aim at individual points" means, and
        // it is the ceiling and node marks (still unshipped) that address it,
        // not the tolerance.
        let anchors = [Point::new(0.0, 0.0), Point::new(1000.0, 0.0)];
        for zoom in [0.25_f32, 0.5, 1.0, 2.0, 8.0] {
            let tol = crate::canvas::screen_tolerance_to_page(NODE_GRAB_SCREEN_TOLERANCE_PX, zoom);
            let press_page = f64::from(4.0_f32 / zoom);
            let d = classify_drag(0, Point::new(press_page, 0.0), &anchors, tol);
            assert_eq!(
                d.node,
                Some(0),
                "a 4 px aim must grab node 0 at zoom {zoom}"
            );
        }
    }

    /// And the same aim 4 px past the MIDPOINT never grabs, at any zoom.
    ///
    /// The other half: a tolerance that grew without bound would eventually
    /// catch something everywhere, which would pass the test above while
    /// making the whole-object move gesture unreachable.
    #[test]
    fn a_press_well_clear_of_every_anchor_stays_a_move_at_every_zoom() {
        let anchors = [Point::new(0.0, 0.0), Point::new(1000.0, 0.0)];
        for zoom in [0.25_f32, 0.5, 1.0, 2.0, 8.0] {
            let tol = crate::canvas::screen_tolerance_to_page(NODE_GRAB_SCREEN_TOLERANCE_PX, zoom);
            let d = classify_drag(0, Point::new(500.0, 0.0), &anchors, tol);
            assert_eq!(
                d.node, None,
                "midway is a move, not a node grab, at zoom {zoom}"
            );
        }
    }

    #[test]
    fn nearest_anchor_picks_the_closest_and_skips_non_finite() {
        let anchors = [
            Point::new(f64::NAN, 0.0),
            Point::new(10.0, 0.0),
            Point::new(12.0, 0.0),
        ];
        assert_eq!(
            nearest_anchor(Point::new(11.0, 0.0), &anchors, 6.0),
            Some(1)
        );
        // Nothing within tolerance.
        assert_eq!(nearest_anchor(Point::new(100.0, 0.0), &anchors, 6.0), None);
    }
}
