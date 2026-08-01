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

use pdfce_core::vector::Point;

/// The screen-space radius, in page-space points, within which a drag that
/// begins near a selected object's anchor is treated as a **node drag**
/// rather than a whole-object move. A forgiving fixed value (selection
/// tolerance is the object provider's ~3 pt; a node grab is a touch larger so
/// an anchor is easy to catch). Zoom-invariant screen-pixel node grabbing is
/// a follow-up refinement; this fixed page-space value is honest and simple.
pub const NODE_GRAB_TOLERANCE: f64 = 6.0;

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
/// [`NODE_GRAB_TOLERANCE`], else a whole-object **move**.
///
/// This is the one non-trivial decision of the tool — that a drag which
/// begins on an anchor grabs the node, and otherwise moves the object — and
/// it is pure, so it is tested here rather than in the live handler.
#[must_use]
pub fn classify_drag(object_index: usize, start: Point, anchors: &[Point]) -> VectorDrag {
    let node = nearest_anchor(start, anchors, NODE_GRAB_TOLERANCE);
    VectorDrag {
        object_index,
        node,
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
        let d = classify_drag(7, Point::new(101.0, 101.5), &anchors);
        assert_eq!(d.object_index, 7);
        assert_eq!(d.node, Some(1));
    }

    #[test]
    fn a_drag_starting_away_from_anchors_is_a_move() {
        let anchors = [Point::new(0.0, 0.0), Point::new(100.0, 100.0)];
        let d = classify_drag(3, Point::new(50.0, 50.0), &anchors);
        assert_eq!(d.node, None);
        // The move delta is current − start.
        assert_eq!(d.delta(Point::new(60.0, 45.0)), (10.0, -5.0));
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
