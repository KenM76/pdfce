//! # Hit-test geometry (ISO 32000-1 §8.5.3 fill rules)
//!
//! Point→object and marquee→objects hit-testing over a page's
//! [`super::PageObjects`], in **page space**, per decision 011 §2.1's
//! "hit-test geometry … is what the snapping engine (12.M1) and the GUI
//! selection consume." All math is `pdfce-core`-local (GUI-free) so the
//! GUI target provider (`pdfce-gui`) stays a thin adapter that only
//! converts coordinate spaces.
//!
//! ## What "hits" an object
//!
//! - **Filled path** (`f`/`B`/…): the point is *inside* the fill, tested
//!   with the object's own winding rule (§8.5.3.3) — nonzero winding
//!   number ≠ 0, or even-odd crossing parity odd — with every subpath
//!   treated as closed (a fill "implicitly closes all open subpaths",
//!   §8.5.3.1). A near-miss just outside the edge also hits within
//!   `tolerance`.
//! - **Stroked path** (`S`/`s`/`B`/…): the point is within
//!   `stroke_half_width + tolerance` of the path outline, where the
//!   stroke half-width is the user-space line width scaled into page space
//!   by the object's CTM (§8.4.3.2 — line width is a user-space quantity).
//! - **Clip/no-op path** (`n`): the point is within `tolerance` of the
//!   outline (invisible geometry is still selectable, but only precisely).
//! - **Text / image / form**: the point is inside the object's page bbox
//!   (inflated by `tolerance`). These carry no editable node geometry, so
//!   a bbox test is the whole of it.
//!
//! ## Topmost wins
//!
//! [`super::PageObjects::objects`] is in paint order, so
//! [`hit_test_point`] scans back-to-front and returns the **last-painted**
//! (topmost) object at the point — the selection convention every editor
//! uses.
//!
//! ## Bézier handling
//!
//! Curves are flattened to [`FLATTEN_STEPS`] line segments for the
//! inside/proximity tests — a fixed subdivision (bounded work for the
//! fuzz target) that is well within a screen pixel at any realistic zoom
//! for the tolerances selection uses.

use super::decompose::{FillRule, PageObjects, PathObject, Segment, Subpath, VectorObject};
use super::geometry::{Bounds, Matrix, Point};

/// Fixed cubic-flattening subdivision (module docs). 16 chords is
/// sub-pixel for selection tolerances and bounds the per-object work a
/// hostile node count can force.
pub const FLATTEN_STEPS: usize = 16;

/// How a marquee rectangle decides which objects it selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarqueeMode {
    /// An object is selected only if its page bbox is **fully enclosed** by
    /// the marquee — decision 011's default, grounded in Inkscape's default
    /// rubber-band behavior (R61).
    Enclosed,
    /// An object is selected if its page bbox **touches** the marquee (any
    /// overlap) — the alternate Inkscape "touch" convention.
    Touched,
}

/// Test a page-space `point` against a page's objects and return the
/// **topmost** (last-painted) object's index, or `None` for a miss.
///
/// `tolerance` is a page-space slack (the GUI converts a few screen pixels
/// into page units and passes it here), widening every object's hittable
/// region so a click near — not dead-on — an edge still selects.
#[must_use]
pub fn hit_test_point(model: &PageObjects, point: Point, tolerance: f64) -> Option<usize> {
    if !point.is_finite() {
        return None;
    }
    model
        .objects
        .iter()
        .enumerate()
        .rev()
        .find(|(_, obj)| object_hit(obj, point, tolerance))
        .map(|(i, _)| i)
}

/// The indices of every object selected by a page-space marquee `rect`,
/// per `mode`, in paint order.
#[must_use]
pub fn hit_test_rect(model: &PageObjects, rect: Bounds, mode: MarqueeMode) -> Vec<usize> {
    if rect.is_empty() {
        return Vec::new();
    }
    model
        .objects
        .iter()
        .enumerate()
        .filter(|(_, obj)| {
            let bb = obj.page_bbox();
            match mode {
                MarqueeMode::Enclosed => bb.contained_by(rect),
                MarqueeMode::Touched => bb.intersects(rect),
            }
        })
        .map(|(i, _)| i)
        .collect()
}

/// Whether `point` hits `obj` within `tolerance` (module docs' per-kind
/// rules).
fn object_hit(obj: &VectorObject, point: Point, tolerance: f64) -> bool {
    match obj {
        VectorObject::Path(p) => path_hit(p, point, tolerance),
        VectorObject::Text(t) => t.page_bbox.inflate(tolerance).contains(point),
        VectorObject::Image(i) => i.page_bbox.inflate(tolerance).contains(point),
    }
}

/// Whether `point` hits a path object: inside its fill (if filled) or
/// within the stroke/clip proximity threshold of its outline.
fn path_hit(path: &PathObject, point: Point, tolerance: f64) -> bool {
    // A cheap bbox reject first (the object's page bbox widened by the
    // stroke half-width and tolerance).
    let half = stroke_half_width(path);
    if !path.page_bbox.inflate(half + tolerance).contains(point) {
        return false;
    }

    let subpaths = path.page_subpaths();

    if let Some(rule) = path.style.fill
        && point_inside(&subpaths, point, rule)
    {
        return true;
    }

    let threshold = if path.style.stroke {
        half + tolerance
    } else {
        // A filled-only or `n` path: no stroke, but a near-edge click
        // should still land, so use the tolerance alone as the proximity
        // band.
        tolerance
    };
    outline_within(&subpaths, point, threshold)
}

/// The user-space line width scaled into page space by the object's CTM,
/// halved — the distance the stroke extends either side of the path
/// centerline (§8.4.3.2). A width-0 hairline gets a tiny nominal value so
/// it is still selectable.
fn stroke_half_width(path: &PathObject) -> f64 {
    if !path.style.stroke {
        return 0.0;
    }
    let scale = ctm_scale(path.ctm);
    let w = if path.line_width <= 0.0 {
        0.1
    } else {
        path.line_width
    };
    (w * scale) / 2.0
}

/// A scalar page-space scale estimate for a CTM — the square root of the
/// absolute determinant (the geometric-mean linear scale). Used to map a
/// user-space line width into page space for stroke proximity. A
/// degenerate/non-finite CTM yields a harmless 1.0.
fn ctm_scale(ctm: Matrix) -> f64 {
    let d = ctm.determinant().abs();
    if d.is_finite() && d > 0.0 {
        d.sqrt()
    } else {
        1.0
    }
}

/// Whether `point` is inside the region the subpaths fill, under `rule`
/// (every subpath treated as closed — a fill implicitly closes, §8.5.3.1).
fn point_inside(subpaths: &[Subpath], point: Point, rule: FillRule) -> bool {
    let mut winding = 0i32;
    let mut crossings = 0u32;
    for sp in subpaths {
        let poly = flatten(sp);
        accumulate_crossings(&poly, point, &mut winding, &mut crossings);
    }
    match rule {
        FillRule::NonZero => winding != 0,
        FillRule::EvenOdd => crossings % 2 == 1,
    }
}

/// Whether `point` is within `threshold` of any outline segment (stroke /
/// clip proximity). Closed subpaths include their closing edge.
fn outline_within(subpaths: &[Subpath], point: Point, threshold: f64) -> bool {
    let t2 = threshold * threshold;
    for sp in subpaths {
        let poly = flatten(sp);
        let n = poly.len();
        if n == 0 {
            continue;
        }
        for w in poly.windows(2) {
            let [a, b] = w else { continue };
            if dist_sq_point_segment(point, *a, *b) <= t2 {
                return true;
            }
        }
        // Closing edge, for a closed subpath (a stroked `h`/`re`/`s`).
        if sp.closed
            && n >= 2
            && let (Some(&last), Some(&firstp)) = (poly.last(), poly.first())
            && dist_sq_point_segment(point, last, firstp) <= t2
        {
            return true;
        }
    }
    false
}

/// Flatten one subpath (page space) to a polyline of on-curve vertices,
/// cubics subdivided into [`FLATTEN_STEPS`] chords. Non-finite vertices
/// are dropped (a hostile operand cannot poison the ray cast).
fn flatten(sp: &Subpath) -> Vec<Point> {
    let mut out: Vec<Point> = Vec::new();
    let push = |p: Point, out: &mut Vec<Point>| {
        if p.is_finite() {
            out.push(p);
        }
    };
    push(sp.start, &mut out);
    let mut from = sp.start;
    for seg in &sp.segments {
        match *seg {
            Segment::Line { to } => {
                push(to, &mut out);
                from = to;
            }
            Segment::Cubic { c1, c2, to } => {
                for step in 1..=FLATTEN_STEPS {
                    let t = step as f64 / FLATTEN_STEPS as f64;
                    push(cubic_at(from, c1, c2, to, t), &mut out);
                }
                from = to;
            }
        }
    }
    out
}

/// A cubic Bézier point at parameter `t` (de Casteljau, closed form).
fn cubic_at(p0: Point, c1: Point, c2: Point, p3: Point, t: f64) -> Point {
    let u = 1.0 - t;
    let w0 = u * u * u;
    let w1 = 3.0 * u * u * t;
    let w2 = 3.0 * u * t * t;
    let w3 = t * t * t;
    Point::new(
        w0 * p0.x + w1 * c1.x + w2 * c2.x + w3 * p3.x,
        w0 * p0.y + w1 * c1.y + w2 * c2.y + w3 * p3.y,
    )
}

/// Fold one closed polygon's edge crossings of the ray `y = point.y,
/// x ≥ point.x` into the running winding number (signed, for nonzero) and
/// crossing count (unsigned, for even-odd). Standard robust half-open
/// (`[y0, y1)`) crossing test.
fn accumulate_crossings(poly: &[Point], point: Point, winding: &mut i32, crossings: &mut u32) {
    if poly.len() < 2 {
        return;
    }
    // Every consecutive pair, plus the closing edge (last → first) so the
    // polygon is treated as closed (a fill implicitly closes).
    let closing = match (poly.first(), poly.last()) {
        (Some(&f), Some(&l)) => Some((l, f)),
        _ => None,
    };
    let pairs = poly.windows(2).filter_map(|w| match w {
        [a, b] => Some((*a, *b)),
        _ => None,
    });
    for (a, b) in pairs.chain(closing) {
        // Half-open interval avoids double-counting a vertex on the ray.
        let a_below = a.y <= point.y;
        let b_below = b.y <= point.y;
        if a_below == b_below {
            continue;
        }
        // The edge crosses the horizontal line through `point`; find the x
        // of the intersection.
        let dy = b.y - a.y;
        if dy == 0.0 {
            continue;
        }
        let t = (point.y - a.y) / dy;
        let x = a.x + t * (b.x - a.x);
        if x >= point.x {
            *crossings += 1;
            if b.y > a.y {
                *winding += 1; // upward edge
            } else {
                *winding -= 1; // downward edge
            }
        }
    }
}

/// Squared distance from `p` to the segment `a`–`b` (avoids a `sqrt` in
/// the proximity loop). A degenerate segment (`a == b`) reduces to the
/// point distance.
fn dist_sq_point_segment(p: Point, a: Point, b: Point) -> f64 {
    let vx = b.x - a.x;
    let vy = b.y - a.y;
    let wx = p.x - a.x;
    let wy = p.y - a.y;
    let len2 = vx * vx + vy * vy;
    if len2 <= 0.0 {
        return wx * wx + wy * wy;
    }
    let t = ((wx * vx + wy * vy) / len2).clamp(0.0, 1.0);
    let dx = wx - t * vx;
    let dy = wy - t * vy;
    dx * dx + dy * dy
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
    use crate::content::ContentStream;
    use crate::vector::decompose::{NoXObjects, decompose};

    fn model(src: &[u8]) -> PageObjects {
        let cs = ContentStream::parse(src.to_vec()).unwrap();
        decompose(&cs, Matrix::IDENTITY, &NoXObjects)
    }

    #[test]
    fn a_click_inside_a_filled_rectangle_hits_it() {
        let m = model(b"10 10 80 80 re f");
        assert_eq!(hit_test_point(&m, Point::new(50.0, 50.0), 1.0), Some(0));
        // outside, beyond tolerance
        assert_eq!(hit_test_point(&m, Point::new(200.0, 200.0), 1.0), None);
    }

    #[test]
    fn a_filled_rectangle_with_a_hole_is_even_odd_empty_in_the_hole() {
        // Outer 0..100 square, inner 40..60 square, even-odd fill => the
        // inner square is a hole.
        let m = model(b"0 0 100 100 re 40 40 20 20 re f*");
        // inside the outer ring
        assert_eq!(hit_test_point(&m, Point::new(10.0, 10.0), 0.5), Some(0));
        // inside the hole -> miss (even-odd)
        assert_eq!(hit_test_point(&m, Point::new(50.0, 50.0), 0.5), None);
    }

    #[test]
    fn a_click_near_a_stroked_line_hits_within_the_stroke_and_tolerance() {
        // A 1 pt horizontal line from (0,50) to (100,50).
        let m = model(b"0 50 m 100 50 l S");
        assert_eq!(hit_test_point(&m, Point::new(50.0, 50.4), 0.5), Some(0));
        // Well away from the line -> miss.
        assert_eq!(hit_test_point(&m, Point::new(50.0, 70.0), 0.5), None);
    }

    #[test]
    fn topmost_object_wins_at_an_overlap() {
        // Two overlapping filled rectangles; the second painted is on top.
        let m = model(b"0 0 60 60 re f 20 20 60 60 re f");
        // In the overlap region, the later (index 1) object wins.
        assert_eq!(hit_test_point(&m, Point::new(40.0, 40.0), 0.5), Some(1));
        // In the first-only region, the first wins.
        assert_eq!(hit_test_point(&m, Point::new(5.0, 5.0), 0.5), Some(0));
    }

    #[test]
    fn marquee_enclosed_selects_only_fully_contained_objects() {
        let m = model(b"10 10 20 20 re f 200 200 20 20 re f");
        let rect = Bounds {
            min: Point::new(0.0, 0.0),
            max: Point::new(100.0, 100.0),
        };
        assert_eq!(hit_test_rect(&m, rect, MarqueeMode::Enclosed), vec![0]);
        // A marquee that only clips the first still selects it under Touched.
        let clip = Bounds {
            min: Point::new(0.0, 0.0),
            max: Point::new(15.0, 15.0),
        };
        assert_eq!(
            hit_test_rect(&m, clip, MarqueeMode::Enclosed),
            Vec::<usize>::new()
        );
        assert_eq!(hit_test_rect(&m, clip, MarqueeMode::Touched), vec![0]);
    }

    #[test]
    fn a_curve_is_hittable_after_flattening() {
        // A cubic bump from (0,0) to (100,0), control points up high.
        let m = model(b"0 0 m 30 100 70 100 100 0 c S");
        // Near the apex of the flattened curve.
        assert!(hit_test_point(&m, Point::new(50.0, 75.0), 3.0).is_some());
    }

    #[test]
    fn non_finite_query_point_is_a_miss_not_a_panic() {
        let m = model(b"0 0 100 100 re f");
        assert_eq!(hit_test_point(&m, Point::new(f64::NAN, 0.0), 1.0), None);
    }
}
