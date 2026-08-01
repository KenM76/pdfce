//! # Vector geometry primitives (ISO 32000-1 §8.3, §8.5)
//!
//! The hand-rolled 2-D affine geometry the read-only vector object model
//! (`docs/decisions/011-first-beta-scaled-measurement-dimensioning-tool.md`
//! §2.1) is built on: a [`Point`], a PDF-convention affine [`Matrix`], an
//! axis-aligned [`Bounds`], a working [`Rgb`] colour, and the **shared
//! path-construction primitives** ([`cubic_from_v`], [`cubic_from_y`],
//! [`rect_corners`]) that `pdfce-render`'s interpreter also calls, so the
//! trap-prone operand arithmetic (`v`/`y`'s implicit control points,
//! `re`'s corner expansion) exists as ONE implementation rather than two
//! (the geometry analogue of the R49/R60 "one pipeline" discipline; the
//! Z2 risk mitigation in decision 011 Appendix A Pass 9a).
//!
//! ## Why hand-rolled and not a dependency (rule 13)
//!
//! pdfce carries a ZERO-new-dependency posture through this Pass. An
//! affine 2×3 matrix, a point, and a bounding box are ~200 lines of
//! arithmetic; a linear-algebra crate would be a copyleft/licensing
//! classification and a WASM-fork weight for no benefit. `pdfce-render`
//! rasterizes with `tiny-skia::Transform`, but `pdfce-core` must stay
//! free of `tiny-skia` (it is GUI-adjacent render weight the WASM engine
//! fork should not inherit), so the core object model has its own matrix.
//! The two are kept in agreement **by construction**: the render walk
//! calls the shared primitives here for the exact same node values, and
//! an acceptance cross-check (in `pdfce-render`'s tests) compares the full
//! page-space geometry the two produce on the fixtures.
//!
//! ## The PDF coordinate convention (row vectors, §8.3.3–§8.3.4)
//!
//! PDF transforms a point by **left-multiplication of a row vector**:
//! `[x' y' 1] = [x y 1] × M`, where
//!
//! ```text
//!       | a b 0 |
//!   M = | c d 0 |     x' = a·x + c·y + e     y' = b·x + d·y + f
//!       | e f 1 |
//! ```
//!
//! so `Matrix` stores exactly the six numbers a PDF `cm`/`Tm`/`/Matrix`
//! operand carries, in that order, and [`Matrix::map_point`] applies them
//! with that formula. Composition is the row-vector product: applying `A`
//! then `B` to a point is `p × A × B = p × (A·B)`, which is what
//! [`Matrix::post_concat`] computes (`A.post_concat(B)` = "apply A then
//! B"), matching `tiny-skia`'s `post_concat` and the render interpreter's
//! `m.post_concat(ctm)` for the `cm` operator.

/// A point in some 2-D coordinate space (PDF user space, or a page-space
/// image of it under a CTM). Values are `f64` — content-stream operands
/// are real numbers and the object model keeps full precision through the
/// transform chain, narrowing to `f32` only at the render/GUI boundary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// Horizontal coordinate.
    pub x: f64,
    /// Vertical coordinate (PDF user space is Y-up).
    pub y: f64,
}

impl Point {
    /// A point from its two coordinates.
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Euclidean distance to `other`. Uses [`f64::hypot`], which is
    /// overflow-robust for the coordinate magnitudes a PDF can carry.
    #[must_use]
    pub fn distance(self, other: Self) -> f64 {
        (self.x - other.x).hypot(self.y - other.y)
    }

    /// The midpoint of the segment `self`–`other`.
    #[must_use]
    pub fn midpoint(self, other: Self) -> Self {
        Self::new((self.x + other.x) / 2.0, (self.y + other.y) / 2.0)
    }

    /// Whether both coordinates are finite (neither `NaN` nor infinite).
    ///
    /// The decomposition tolerates the degenerate/hostile operands a
    /// fuzzed content stream produces (`1e308 1e308 m`, `NaN` via a
    /// malformed real): a non-finite point is kept in the node list for
    /// lossless provenance but skipped by hit-testing and centerline
    /// math, which is what this predicate gates.
    #[must_use]
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

/// A 2×3 affine transform in PDF row-vector convention (module docs).
///
/// The six fields are the `cm`/`Tm`/`/Matrix` operand order `a b c d e f`.
/// Deliberately `Copy` (48 bytes) so it threads through the graphics-state
/// stack and the per-object capture without allocation ceremony.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix {
    /// Row-vector coefficient a (x-scale / cos component).
    pub a: f64,
    /// Row-vector coefficient b (y-shear / sin component).
    pub b: f64,
    /// Row-vector coefficient c (x-shear / −sin component).
    pub c: f64,
    /// Row-vector coefficient d (y-scale / cos component).
    pub d: f64,
    /// Row-vector translation e (Δx).
    pub e: f64,
    /// Row-vector translation f (Δy).
    pub f: f64,
}

impl Matrix {
    /// The identity transform — the initial CTM the page content stream is
    /// decomposed under, so an object's page-space geometry is genuine PDF
    /// default user space (§8.3.2.3).
    pub const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    /// A matrix from its six row-vector coefficients (the operand order of
    /// `cm`, `Tm`, and a `/Matrix` array).
    #[must_use]
    pub const fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        Self { a, b, c, d, e, f }
    }

    /// A pure translation `[1 0 0 1 tx ty]` — the `Td`/`TD` text-line
    /// offset and the building block of the text-matrix walk (§9.4.2).
    #[must_use]
    pub const fn translate(tx: f64, ty: f64) -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: tx,
            f: ty,
        }
    }

    /// Transform `p` by this matrix: `p' = p × M` (module docs' formula).
    #[must_use]
    pub fn map_point(self, p: Point) -> Point {
        Point::new(
            self.a * p.x + self.c * p.y + self.e,
            self.b * p.x + self.d * p.y + self.f,
        )
    }

    /// The composition "apply `self`, then `other`" — the row-vector
    /// product `self · other`, so `self.post_concat(other).map_point(p)`
    /// equals `other.map_point(self.map_point(p))`.
    ///
    /// This is the operation the `cm` operator performs on the CTM
    /// (`CTM′ = M · CTM`, §8.3.4) and it is named and oriented to match
    /// `tiny-skia::Transform::post_concat` so the render interpreter's
    /// `m.post_concat(ctm)` and this object model's CTM update are the same
    /// composition — the agree-by-construction requirement for the CTM
    /// itself.
    #[must_use]
    pub fn post_concat(self, other: Self) -> Self {
        Self {
            a: self.a * other.a + self.b * other.c,
            b: self.a * other.b + self.b * other.d,
            c: self.c * other.a + self.d * other.c,
            d: self.c * other.b + self.d * other.d,
            e: self.e * other.a + self.f * other.c + other.e,
            f: self.e * other.b + self.f * other.d + other.f,
        }
    }

    /// The signed area scale factor (`a·d − b·c`) — used to sanity-flag a
    /// degenerate (non-invertible) CTM and to estimate a coarse glyph
    /// scale for a text object's approximate bounds.
    #[must_use]
    pub fn determinant(self) -> f64 {
        self.a * self.d - self.b * self.c
    }
}

/// An axis-aligned bounding box in one coordinate space, or the empty box.
///
/// Stored as min/max corners; the empty box is `min > max` on both axes
/// and is what an object with no finite geometry yields. Kept `Copy` for
/// the same threading reasons as [`Matrix`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    /// Lower-left corner (smaller x, smaller y).
    pub min: Point,
    /// Upper-right corner (larger x, larger y).
    pub max: Point,
}

impl Bounds {
    /// The empty box — `min = +∞`, `max = −∞` — so [`Bounds::union_point`]
    /// of the empty box with any finite point yields a degenerate box AT
    /// that point, and unioning two empty boxes stays empty. This is the
    /// standard "grow from nothing" accumulator seed.
    pub const EMPTY: Self = Self {
        min: Point {
            x: f64::INFINITY,
            y: f64::INFINITY,
        },
        max: Point {
            x: f64::NEG_INFINITY,
            y: f64::NEG_INFINITY,
        },
    };

    /// Whether this box encloses no area — the accumulator seed, or a box
    /// that never saw a finite point.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.min.x > self.max.x || self.min.y > self.max.y
    }

    /// Grow the box to include `p`; a non-finite `p` is ignored (it would
    /// poison the box with `NaN`/∞, exactly the hostile-operand case the
    /// fuzz target drives).
    #[must_use]
    pub fn union_point(self, p: Point) -> Self {
        if !p.is_finite() {
            return self;
        }
        Self {
            min: Point::new(self.min.x.min(p.x), self.min.y.min(p.y)),
            max: Point::new(self.max.x.max(p.x), self.max.y.max(p.y)),
        }
    }

    /// The union of two boxes (either may be empty).
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        self.union_point(other.min).union_point(other.max)
    }

    /// Grow the box by `margin` on every side (a no-op on the empty box).
    /// Used to give a text object's origin-derived bounds a coarse glyph
    /// margin, and to widen a hit-test bbox pre-filter by the tolerance.
    #[must_use]
    pub fn inflate(self, margin: f64) -> Self {
        if self.is_empty() {
            return self;
        }
        Self {
            min: Point::new(self.min.x - margin, self.min.y - margin),
            max: Point::new(self.max.x + margin, self.max.y + margin),
        }
    }

    /// Whether `p` lies within the closed box.
    #[must_use]
    pub fn contains(self, p: Point) -> bool {
        !self.is_empty()
            && p.x >= self.min.x
            && p.x <= self.max.x
            && p.y >= self.min.y
            && p.y <= self.max.y
    }

    /// Whether this box lies wholly inside `outer` (the fully-contained
    /// marquee-enclosure test — decision 011's default, grounded in
    /// Inkscape's default rubber-band-selects-fully-enclosed behavior,
    /// R61).
    #[must_use]
    pub fn contained_by(self, outer: Self) -> bool {
        !self.is_empty()
            && !outer.is_empty()
            && self.min.x >= outer.min.x
            && self.min.y >= outer.min.y
            && self.max.x <= outer.max.x
            && self.max.y <= outer.max.y
    }

    /// Whether the two boxes overlap at all (the alternate, partial-
    /// enclosure marquee test, selectable via [`crate::vector::hit`]).
    #[must_use]
    pub fn intersects(self, other: Self) -> bool {
        !self.is_empty()
            && !other.is_empty()
            && self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }
}

/// A working RGB colour in `[0, 1]` components — the object model's record
/// of an object's paint colour at paint time (§8.6.4 device colours),
/// captured for display/inspection.
///
/// Uses the SAME naive un-colour-managed conversions as the Pass 1
/// renderer (`from_gray`/`from_rgb`/`from_cmyk`) so a decomposed object's
/// recorded colour matches the pixel the renderer paints; real colour
/// management is later work in both places.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgb {
    /// Red, 0.0–1.0.
    pub r: f32,
    /// Green, 0.0–1.0.
    pub g: f32,
    /// Blue, 0.0–1.0.
    pub b: f32,
}

impl Rgb {
    /// Black — the initial colour in every device space (§8.6.4).
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
    };

    /// From a DeviceGray value (`g`/`G`).
    #[must_use]
    pub fn from_gray(v: f32) -> Self {
        let v = v.clamp(0.0, 1.0);
        Self { r: v, g: v, b: v }
    }

    /// From DeviceRGB components (`rg`/`RG`).
    #[must_use]
    pub fn from_rgb(r: f32, g: f32, b: f32) -> Self {
        Self {
            r: r.clamp(0.0, 1.0),
            g: g.clamp(0.0, 1.0),
            b: b.clamp(0.0, 1.0),
        }
    }

    /// From DeviceCMYK components (`k`/`K`) via the naive additive
    /// conversion `1 − min(1, x + k)` — the documented Pass 1
    /// simplification shared with `pdfce-render`.
    #[must_use]
    pub fn from_cmyk(c: f32, m: f32, y: f32, k: f32) -> Self {
        Self {
            r: 1.0 - (c + k).min(1.0),
            g: 1.0 - (m + k).min(1.0),
            b: 1.0 - (y + k).min(1.0),
        }
    }
}

// ---------------------------------------------------------------------------
// Shared path-construction primitives (the agree-by-construction anchor)
// ---------------------------------------------------------------------------
//
// These three pure functions encode the operand arithmetic of the three
// construction operators the PDF spec (§8.5.2.1, Table 59) defines by
// implication rather than literally — the exact places a second, forked
// decomposition would drift from the renderer. `pdfce-render`'s
// interpreter calls them for the identical node values (narrowing the
// `f64` results to `f32`, which round-trips its `f32` operands exactly),
// so this object model and the render agree on these operators by sharing
// one implementation, not by two hand-derived copies staying in sync.

/// The three control/anchor points of the cubic the `v` operator appends
/// (`x2 y2 x3 y3 v`, Table 59): its **first control point is the current
/// point** — the classic "v/y trap" that silently mis-renders if forgotten.
///
/// Returns `(first_control, second_control, endpoint)` where
/// `first_control == current`.
#[must_use]
pub fn cubic_from_v(current: Point, x2: f64, y2: f64, x3: f64, y3: f64) -> (Point, Point, Point) {
    (current, Point::new(x2, y2), Point::new(x3, y3))
}

/// The three control/anchor points of the cubic the `y` operator appends
/// (`x1 y1 x3 y3 y`, Table 59): its **second control point is the
/// endpoint** — the mirror trap of `v`.
///
/// Returns `(first_control, second_control, endpoint)` where
/// `second_control == endpoint`.
#[must_use]
pub fn cubic_from_y(x1: f64, y1: f64, x3: f64, y3: f64) -> (Point, Point, Point) {
    let end = Point::new(x3, y3);
    (Point::new(x1, y1), end, end)
}

/// The four corner anchors of the rectangle the `re` operator appends
/// (`x y w h re`, Table 59), in the spec's defined expansion order
/// `(x,y) → (x+w,y) → (x+w,y+h) → (x,y+h)` closing back to `(x,y)`.
///
/// A negative `w`/`h` is legal and yields a rectangle traced the other
/// way — kept as-is (the caller's fill winding, not this function, is what
/// interprets orientation), exactly as `tiny-skia`'s `re` expansion does.
#[must_use]
pub fn rect_corners(x: f64, y: f64, w: f64, h: f64) -> [Point; 4] {
    [
        Point::new(x, y),
        Point::new(x + w, y),
        Point::new(x + w, y + h),
        Point::new(x, y + h),
    ]
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

    fn approx(a: Point, b: Point) -> bool {
        (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9
    }

    #[test]
    fn identity_maps_a_point_to_itself() {
        let p = Point::new(3.5, -7.25);
        assert!(approx(Matrix::IDENTITY.map_point(p), p));
    }

    #[test]
    fn map_point_uses_the_row_vector_formula() {
        // Scale 2 in x, 3 in y, translate (10, 20).
        let m = Matrix::new(2.0, 0.0, 0.0, 3.0, 10.0, 20.0);
        assert!(approx(
            m.map_point(Point::new(1.0, 1.0)),
            Point::new(12.0, 23.0)
        ));
    }

    #[test]
    fn post_concat_is_apply_self_then_other() {
        // A: translate by (5, 0). B: scale x by 2.
        let a = Matrix::translate(5.0, 0.0);
        let b = Matrix::new(2.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        let composed = a.post_concat(b);
        let p = Point::new(1.0, 0.0);
        // apply A then B: (1,0) -> (6,0) -> (12,0)
        assert!(approx(composed.map_point(p), Point::new(12.0, 0.0)));
        // and it equals b.map(a.map(p)) by definition
        assert!(approx(composed.map_point(p), b.map_point(a.map_point(p))));
    }

    #[test]
    fn post_concat_matches_pdf_cm_premultiply_semantics() {
        // A 90° rotation composed with a translation, verifying the same
        // orientation the render interpreter's `m.post_concat(ctm)` uses:
        // rotating (1,0) by +90° gives (0,1); then translate (0,10).
        let rot = Matrix::new(0.0, 1.0, -1.0, 0.0, 0.0, 0.0);
        let tr = Matrix::translate(0.0, 10.0);
        let ctm = rot.post_concat(tr);
        assert!(approx(
            ctm.map_point(Point::new(1.0, 0.0)),
            Point::new(0.0, 11.0)
        ));
    }

    #[test]
    fn bounds_accumulate_and_ignore_non_finite() {
        let b = Bounds::EMPTY
            .union_point(Point::new(1.0, 2.0))
            .union_point(Point::new(-3.0, 5.0))
            .union_point(Point::new(f64::NAN, 0.0)); // ignored
        assert_eq!(b.min, Point::new(-3.0, 2.0));
        assert_eq!(b.max, Point::new(1.0, 5.0));
    }

    #[test]
    fn bounds_containment_and_intersection() {
        let outer = Bounds {
            min: Point::new(0.0, 0.0),
            max: Point::new(10.0, 10.0),
        };
        let inner = Bounds {
            min: Point::new(2.0, 2.0),
            max: Point::new(4.0, 4.0),
        };
        let straddle = Bounds {
            min: Point::new(8.0, 8.0),
            max: Point::new(12.0, 12.0),
        };
        assert!(inner.contained_by(outer));
        assert!(!straddle.contained_by(outer));
        assert!(straddle.intersects(outer));
        assert!(outer.contains(Point::new(5.0, 5.0)));
        assert!(!outer.contains(Point::new(11.0, 5.0)));
    }

    #[test]
    fn v_operator_first_control_is_the_current_point() {
        let cur = Point::new(3.0, 4.0);
        let (c1, c2, end) = cubic_from_v(cur, 10.0, 11.0, 20.0, 21.0);
        assert_eq!(c1, cur);
        assert_eq!(c2, Point::new(10.0, 11.0));
        assert_eq!(end, Point::new(20.0, 21.0));
    }

    #[test]
    fn y_operator_second_control_is_the_endpoint() {
        let (c1, c2, end) = cubic_from_y(10.0, 11.0, 20.0, 21.0);
        assert_eq!(c1, Point::new(10.0, 11.0));
        assert_eq!(c2, Point::new(20.0, 21.0));
        assert_eq!(end, Point::new(20.0, 21.0));
        assert_eq!(c2, end);
    }

    #[test]
    fn re_corners_follow_the_spec_expansion_order() {
        let c = rect_corners(1.0, 2.0, 4.0, 3.0);
        assert_eq!(c[0], Point::new(1.0, 2.0));
        assert_eq!(c[1], Point::new(5.0, 2.0));
        assert_eq!(c[2], Point::new(5.0, 5.0));
        assert_eq!(c[3], Point::new(1.0, 5.0));
    }
}
