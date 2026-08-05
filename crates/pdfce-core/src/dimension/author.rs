//! # Dimension annotation + `/AP` authoring (decision 011 §2.3, ISO 32000-1 §12.5.6.7 / §12.9)
//!
//! Turn a stored [`DimensionKind`] + its group's scale/format into a real,
//! portable `/Line` annotation with `/IT /LineDimension` and a **fully-baked
//! `/AP`** (leader + extension ticks + arrowheads + value label). This is the
//! **additive** authoring half (decision 011 §5.8 overlay-append, R46
//! zero-exception): it emits new indirect objects only; no page content-stream
//! byte is touched. [`crate::edit`] allocates object numbers and wires
//! `/AP`/`/P`/`/OC` in (mirroring `add_markup`).
//!
//! ## Why a baked `/AP`, always (R44)
//!
//! Like [`crate::annot_author`], every dimension carries a complete `/AP` so
//! the drawn dimension + value render in **any** reader (pdfium included, the
//! R59 gate), never relying on a consumer to synthesise appearance from the
//! `/Line`/`/IT`/`/Measure` keys. The value text is laid out in Base-14
//! Helvetica (§9.6.2.1 program-free dict, shared with [`crate::vartext`]).
//!
//! ## The `/Measure` mirror (interop, §12.9)
//!
//! When the group has a scale set, the group's portable `/Measure` dict
//! ([`super::measure_dict::build_measure_dict`]) is attached to the annotation
//! (per-annotation scale, PDF 1.7 — the co-equal alternative to a page
//! `/Viewport`, and the one that side-steps the geometric-partition problem for
//! overlapping different-scale groups). This is the reader-visible scale that
//! survives even if a foreign editor drops the `/PieceInfo` sidecar.
//!
//! ## Deterministic regeneration (the scale-change story)
//!
//! [`author_dimension`] is a **pure function of** `(kind, scale, format)`, so
//! changing a group's scale re-runs it for every member and replaces each
//! member's `/AP` stream object + `/Contents` + `/Measure` (the Pass 7.1
//! regenerate-appearances pattern, decision 011 §2.3).

use crate::fontdata::Std14;
use crate::object::{Dict, Name, Object};
use crate::page_tree::Rect;
use crate::vartext::standard14_font_dict;
use crate::vector::Point;
use crate::writer::content::{ContentBuilder, LineCap, LineJoin, Paint};

use super::group::DimensionKind;
use super::measure_dict::build_measure_dict;
use super::units::{NumberFormat, ScaleState, format_measurement};

/// The resource name the dimension label's font is authored under (matches the
/// `/AP` `/Resources` `/Font` key and the `Tf` operator).
const FONT_RESOURCE: &[u8] = b"Helv";

/// The dimension label point size.
const LABEL_SIZE: f64 = 10.0;

/// The leader/extension stroke width (points).
const LINE_WIDTH: f64 = 0.75;

/// Arrowhead length (points).
const ARROW_LEN: f64 = 7.0;

/// Gap left between the measured point and the start of its extension line.
///
/// **Convention, not mandated.** ANSI/ASME practice is about 1.5 mm in
/// mechanical drawing (1.6-3 mm architectural); ISO 129-1 requires a gap but
/// leaves the value to the field, expressing related distances as multiples of
/// the line width. 4 pt is about 1.4 mm. Decision 026 records the sourcing and
/// the confidence behind both of these numbers; neither is a standard's
/// requirement and neither should be cited as one.
const EXT_GAP: f64 = 4.0;

/// How far an extension line continues PAST the dimension line.
///
/// **Convention, not mandated** — see [`EXT_GAP`]. ~1 mm mechanical, ~3 mm
/// architectural under ANSI practice; 8x line width under ISO. 3 pt is about
/// 1 mm.
const EXT_OVERSHOOT: f64 = 3.0;

/// The annotation-dictionary keys [`author_dimension`] OWNS — the ones a
/// regeneration must overwrite, and must REMOVE when the new state does not
/// produce them.
///
/// Declared here, next to the code that writes them, because "which keys does
/// authoring own" has exactly one correct answer and it belongs where the
/// authoring happens. A regenerator keeping its own list would drift the first
/// time this function learns a new key — and the failure would be silent: a
/// stale `/Measure` left behind by an uncalibrated regeneration claims a scale
/// that no longer applies, and every conforming reader believes it.
///
/// `/AP` is deliberately NOT here. The appearance stream's object id is
/// allocated once when the dimension is wired into a document and reused
/// across regenerations, so the reference must survive; only the stream's
/// CONTENT is rewritten.
///
/// `/C` is deliberately NOT here either, though this function does write it on
/// first authoring. It is a default colour, not something derived from the
/// geometry, scale or format — so nothing about a regeneration makes an
/// existing `/C` stale. Owning it would mean the first recolouring feature
/// silently loses its work the next time anything is regenerated, which is a
/// bug that would be very hard to attribute.
pub const AUTHORED_ANNOT_KEYS: [&[u8]; 6] =
    [b"Type", b"Subtype", b"IT", b"Rect", b"L", b"Contents"];

/// The key authoring writes only when the group is calibrated (§12.9 Table
/// 261) — separated from [`AUTHORED_ANNOT_KEYS`] only for documentation; it is
/// handled by the same overwrite-or-remove rule.
pub const AUTHORED_MEASURE_KEY: &[u8] = b"Measure";

/// The result of authoring one dimension — the pieces [`crate::edit`] wires
/// into a document. Mirrors [`crate::annot_author::AuthoredAppearance`] so the
/// edit-session wiring is identical (allocate `/AP` + annot numbers, stage
/// content, patch `/Annots`).
#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredDimension {
    /// The `/Line` annotation dict: `/Type /Annot /Subtype /Line /IT
    /// /LineDimension /Rect /L /C /Contents` and (when scaled) `/Measure`.
    /// **Missing `/AP`, `/P`, `/OC`** — the session adds them.
    pub annot: Dict,
    /// The `/AP` `/N` form-XObject dict (`/BBox` = `/Rect`, identity matrix,
    /// `/Resources` carrying the Helvetica label font). `/Length` added by the
    /// serializer.
    pub ap_dict: Dict,
    /// The appearance content-stream bytes (raw, unfiltered).
    pub ap_content: Vec<u8>,
    /// The computed `/Rect`, guaranteed positive-area.
    pub rect: Rect,
    /// The display label (for CLI/GUI echo; also stored as `/Contents`).
    pub label: String,
}

/// Author a dimension's `/Line` annotation + baked `/AP` from its geometry and
/// its group's scale + format. Pure and deterministic (regeneration-safe).
///
/// # Examples
///
/// ```
/// use pdfce_core::dimension::{author_dimension, DimensionKind, NumberFormat, ScaleState, Unit};
/// use pdfce_core::vector::{AxisConstraint, Point};
///
/// let kind = DimensionKind::Linear {
///     a: Point::new(100.0, 100.0),
///     b: Point::new(200.0, 100.0),
///     constraint: AxisConstraint::Horizontal,
///     offset: 0.0,
///     text_along: 0.0,
/// };
/// let authored = author_dimension(
///     &kind,
///     ScaleState::Calibrated { scale: 0.01 },
///     NumberFormat::decimal(Unit::Meter, 2),
/// );
/// // 100 pt at 0.01 m/pt = 1.00 m.
/// assert_eq!(authored.label, "1.00 m");
/// assert_eq!(authored.annot.get(b"Subtype").unwrap().as_name().unwrap().as_bytes(), b"Line");
/// assert_eq!(authored.annot.get(b"IT").unwrap().as_name().unwrap().as_bytes(), b"LineDimension");
/// assert!(authored.annot.get(b"Measure").is_some()); // scale mirror
/// ```
#[must_use]
pub fn author_dimension(
    kind: &DimensionKind,
    scale: ScaleState,
    format: NumberFormat,
) -> AuthoredDimension {
    let display = format_measurement(kind.measured_points(), scale, format);
    let label = format!("{}{}", kind.caption_prefix(), display.text);

    // The leader endpoints in page space (the /L pair).
    let (l0, l1) = leader_endpoints(kind);

    // Accumulate the drawn bbox as we build the appearance content.
    let mut bounds = BoundsAcc::new();
    let mut b = ContentBuilder::new();
    b.set_stroke_gray(0.0);
    b.set_fill_gray(0.0);
    b.set_line_width(LINE_WIDTH);
    b.set_line_cap(LineCap::Butt);
    b.set_line_join(LineJoin::Miter);

    match *kind {
        DimensionKind::Linear { .. } => {
            // The measured points, for the extension lines. `linear_geometry`
            // is the ONE definition of this frame — `leader_endpoints` above
            // reads the same function for the dimension line's ends, so the
            // two cannot disagree about where the dimension sits.
            let (ext_a, ext_b) = kind
                .linear_geometry()
                .map_or((l0, l1), |(_, _, pa, pb)| (pa, pb));
            draw_linear(&mut b, &mut bounds, l0, l1, ext_a, ext_b);
        }
        DimensionKind::Circular { fit, .. } => {
            draw_circular(&mut b, &mut bounds, fit.center, fit.radius, l1);
        }
    }

    // The value label. Anchored where the operator DROPPED it along the
    // dimension line (`label_anchor`), not unconditionally at the midpoint —
    // SolidWorks stores a dimension's placement as a point, and sliding the
    // number along its own line is half of what that point expresses. Falls
    // back to the midpoint for a circular dimension, which has no such axis.
    let mid = kind.label_anchor().unwrap_or_else(|| l0.midpoint(l1));
    let text_w = estimate_text_width(&label, LABEL_SIZE);
    let tx = mid.x - text_w / 2.0;
    let ty = mid.y + LABEL_SIZE * 0.4; // just above the leader
    bounds.add(Point::new(tx, ty - LABEL_SIZE));
    bounds.add(Point::new(tx + text_w, ty + LABEL_SIZE));
    b.begin_text();
    b.set_font(FONT_RESOURCE, LABEL_SIZE);
    b.set_text_matrix(1.0, 0.0, 0.0, 1.0, tx, ty);
    b.show_text(label.as_bytes());
    b.end_text();

    let rect = bounds.into_rect();

    // Annotation dict: /Line + /IT /LineDimension + /L + /C + /Contents + /Measure.
    let mut annot = Dict::new();
    annot.insert(Name::from(b"Type"), Object::Name(Name::from(b"Annot")));
    annot.insert(Name::from(b"Subtype"), Object::Name(Name::from(b"Line")));
    annot.insert(
        Name::from(b"IT"),
        Object::Name(Name::from(b"LineDimension")),
    );
    annot.insert(Name::from(b"Rect"), rect_array(rect));
    annot.insert(
        Name::from(b"L"),
        Object::Array(vec![
            Object::Real(l0.x),
            Object::Real(l0.y),
            Object::Real(l1.x),
            Object::Real(l1.y),
        ]),
    );
    annot.insert(
        Name::from(b"C"),
        Object::Array(vec![
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(0.0),
        ]),
    );
    annot.insert(
        Name::from(b"Contents"),
        Object::String(label.as_bytes().to_vec()),
    );
    // The portable /Measure scale mirror (only when a scale is set).
    if let Some(s) = scale.effective_scale(format.unit) {
        annot.insert(Name::from(b"Measure"), build_measure_dict(s, format));
    }

    AuthoredDimension {
        annot,
        ap_dict: ap_form_dict(rect),
        ap_content: b.into_bytes(),
        rect,
        label,
    }
}

/// The `/L` leader endpoints for a dimension: the two picked points for a
/// linear dimension, or centre→rim (`centre + (radius, 0)`) for a circular one.
fn leader_endpoints(kind: &DimensionKind) -> (Point, Point) {
    match *kind {
        // The DIMENSION LINE's ends, not the picked points (Pass 27.0). These
        // differ whenever the constraint is Horizontal/Vertical and the picks
        // are not already aligned, or whenever there is a standoff. Returning
        // the picks here is what drew a constrained dimension at an angle and
        // wrote a `/L` that disagreed with the drawn line.
        DimensionKind::Linear { a, b, .. } => kind
            .linear_geometry()
            .map_or((a, b), |(dim_a, dim_b, _, _)| (dim_a, dim_b)),
        DimensionKind::Circular { fit, .. } => (
            fit.center,
            Point::new(fit.center.x + fit.radius, fit.center.y),
        ),
    }
}

/// Draw a linear ce dimension: the dimension line, real extension (witness)
/// lines back to the measured points, and terminators (Pass 27.0).
///
/// # What changed, and why the old shape was wrong
///
/// This used to stroke a line straight between the two PICKED points and add
/// a 4 pt perpendicular tick at each end. That is only correct when the picks
/// already lie on the constraint axis and there is no standoff — i.e. almost
/// never. `ext_a`/`ext_b` are the measured points; `a`/`c` are the dimension
/// line's own ends, which the caller derives from
/// [`DimensionKind::linear_geometry`].
///
/// Extension lines are **omitted, not clamped**, when they would be shorter
/// than the gap they must leave — which is exactly the zero-standoff case,
/// where the dimension line already passes through the point and a witness
/// line would be a stub of nothing. The two extension lines may point in
/// OPPOSITE directions (picks straddling the dimension line), so each takes
/// its own direction from its own endpoints rather than from the sign of the
/// standoff.
fn draw_linear(
    b: &mut ContentBuilder,
    bounds: &mut BoundsAcc,
    a: Point,
    c: Point,
    ext_a: Point,
    ext_b: Point,
) {
    let (ux, uy) = unit_vector(a, c);

    // The dimension line.
    b.move_to(a.x, a.y);
    b.line_to(c.x, c.y);
    b.paint(Paint::Stroke);
    bounds.add(a);
    bounds.add(c);

    // Extension lines: from just clear of the measured point, to just past the
    // dimension line. Both offsets are DRAFTING CONVENTION, not mandated by
    // any standard — ANSI practice is ~1.5 mm gap and ~1 mm overshoot in
    // mechanical work; ISO expresses both as multiples of the line width.
    // Decision 026 records the sourcing and the confidence for each; they are
    // constants here so a per-standard style can replace them without moving
    // the geometry.
    for (point, dim_end) in [(ext_a, a), (ext_b, c)] {
        let (dx, dy) = (dim_end.x - point.x, dim_end.y - point.y);
        let len = dx.hypot(dy);
        if !len.is_finite() || len <= EXT_GAP + EXT_OVERSHOOT {
            // Too short to draw as a witness line: the dimension line is
            // already at (or through) the point.
            continue;
        }
        let (nx, ny) = (dx / len, dy / len);
        let start = Point::new(point.x + nx * EXT_GAP, point.y + ny * EXT_GAP);
        let end = Point::new(
            point.x + nx * (len + EXT_OVERSHOOT),
            point.y + ny * (len + EXT_OVERSHOOT),
        );
        b.move_to(start.x, start.y);
        b.line_to(end.x, end.y);
        b.paint(Paint::Stroke);
        bounds.add(start);
        bounds.add(end);
    }

    // Arrowheads pointing outward at each end (toward the extension ticks).
    arrowhead(b, bounds, a, (-ux, -uy));
    arrowhead(b, bounds, c, (ux, uy));
}

/// Draw a circular dimension: the fitted circle outline + a radius leader from
/// centre to rim with an arrowhead at the rim.
fn draw_circular(
    b: &mut ContentBuilder,
    bounds: &mut BoundsAcc,
    center: Point,
    radius: f64,
    rim: Point,
) {
    // The fitted circle outline (four kappa cubics), for context.
    if radius.is_finite() && radius > 0.0 {
        emit_circle(b, center, radius);
        bounds.add(Point::new(center.x - radius, center.y - radius));
        bounds.add(Point::new(center.x + radius, center.y + radius));
    }
    // The radius leader centre → rim.
    b.move_to(center.x, center.y);
    b.line_to(rim.x, rim.y);
    b.paint(Paint::Stroke);
    bounds.add(center);
    bounds.add(rim);
    // Arrowhead at the rim pointing outward.
    let (ux, uy) = unit_vector(center, rim);
    arrowhead(b, bounds, rim, (ux, uy));
}

/// Emit a filled arrowhead at `tip`, pointing along the unit direction `dir`.
fn arrowhead(b: &mut ContentBuilder, bounds: &mut BoundsAcc, tip: Point, dir: (f64, f64)) {
    let (ux, uy) = dir;
    if !(ux.is_finite() && uy.is_finite()) {
        return;
    }
    let (px, py) = (-uy, ux);
    let half = ARROW_LEN * 0.35;
    let bx = tip.x - ux * ARROW_LEN;
    let by = tip.y - uy * ARROW_LEN;
    let b1 = Point::new(bx + px * half, by + py * half);
    let b2 = Point::new(bx - px * half, by - py * half);
    b.move_to(tip.x, tip.y);
    b.line_to(b1.x, b1.y);
    b.line_to(b2.x, b2.y);
    b.close_subpath();
    b.paint(Paint::Fill);
    bounds.add(tip);
    bounds.add(b1);
    bounds.add(b2);
}

/// Emit a circle outline centred at `c` radius `r` as four kappa cubics.
fn emit_circle(b: &mut ContentBuilder, c: Point, r: f64) {
    const KAPPA: f64 = 0.552_284_749_830_793_4;
    let o = r * KAPPA;
    b.move_to(c.x + r, c.y);
    b.curve_to(c.x + r, c.y + o, c.x + o, c.y + r, c.x, c.y + r);
    b.curve_to(c.x - o, c.y + r, c.x - r, c.y + o, c.x - r, c.y);
    b.curve_to(c.x - r, c.y - o, c.x - o, c.y - r, c.x, c.y - r);
    b.curve_to(c.x + o, c.y - r, c.x + r, c.y - o, c.x + r, c.y);
    b.close_subpath();
    b.paint(Paint::Stroke);
}

/// The unit vector from `a` to `b`, or `(1, 0)` for a degenerate (zero-length)
/// segment.
fn unit_vector(a: Point, b: Point) -> (f64, f64) {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = dx.hypot(dy);
    if len <= f64::EPSILON || !len.is_finite() {
        (1.0, 0.0)
    } else {
        (dx / len, dy / len)
    }
}

/// A coarse text-width estimate (Helvetica averages ~0.5em per character) —
/// good enough to centre the baked label; exact centring is not load-bearing.
fn estimate_text_width(label: &str, size: f64) -> f64 {
    label.chars().count() as f64 * size * 0.5
}

/// The `/AP` form-XObject dict for a dimension: `/BBox` = the page-space
/// `/Rect` (geometry drawn in absolute coords), identity matrix, `/Resources`
/// carrying the Base-14 Helvetica label font.
fn ap_form_dict(rect: Rect) -> Dict {
    let mut fonts = Dict::new();
    fonts.insert(
        Name(FONT_RESOURCE.to_vec()),
        Object::Dict(standard14_font_dict(Std14::Helvetica)),
    );
    let mut resources = Dict::new();
    resources.insert(Name::from(b"Font"), Object::Dict(fonts));

    let mut d = Dict::new();
    d.insert(Name::from(b"Type"), Object::Name(Name::from(b"XObject")));
    d.insert(Name::from(b"Subtype"), Object::Name(Name::from(b"Form")));
    d.insert(Name::from(b"BBox"), rect_array(rect));
    d.insert(Name::from(b"Resources"), Object::Dict(resources));
    d
}

/// A `[llx lly urx ury]` array.
fn rect_array(r: Rect) -> Object {
    Object::Array(vec![
        Object::Real(r.llx),
        Object::Real(r.lly),
        Object::Real(r.urx),
        Object::Real(r.ury),
    ])
}

/// A running bounds accumulator that yields a strictly-positive `/Rect`
/// (§12.5.5 WF4: a degenerate `/BBox` is a NEGATIVE RESULT).
struct BoundsAcc {
    llx: f64,
    lly: f64,
    urx: f64,
    ury: f64,
}

impl BoundsAcc {
    fn new() -> Self {
        Self {
            llx: f64::INFINITY,
            lly: f64::INFINITY,
            urx: f64::NEG_INFINITY,
            ury: f64::NEG_INFINITY,
        }
    }

    fn add(&mut self, p: Point) {
        if p.is_finite() {
            self.llx = self.llx.min(p.x);
            self.lly = self.lly.min(p.y);
            self.urx = self.urx.max(p.x);
            self.ury = self.ury.max(p.y);
        }
    }

    fn into_rect(self) -> Rect {
        // A small margin so strokes/arrowheads are not clipped by the BBox.
        let margin = 2.0;
        let (mut llx, mut lly, mut urx, mut ury) = if self.llx.is_finite() {
            (self.llx, self.lly, self.urx, self.ury)
        } else {
            (0.0, 0.0, 1.0, 1.0)
        };
        llx -= margin;
        lly -= margin;
        urx += margin;
        ury += margin;
        if urx - llx < 1.0 {
            urx = llx + 1.0;
        }
        if ury - lly < 1.0 {
            ury = lly + 1.0;
        }
        Rect { llx, lly, urx, ury }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::float_cmp
)]
mod tests {
    use super::*;
    use crate::content::ContentStream;
    use crate::dimension::fit::FitCircle;
    use crate::dimension::units::Unit;
    use crate::vector::AxisConstraint;

    fn linear() -> DimensionKind {
        DimensionKind::Linear {
            a: Point::new(100.0, 100.0),
            b: Point::new(200.0, 100.0),
            constraint: AxisConstraint::Horizontal,
            offset: 0.0,
            text_along: 0.0,
        }
    }

    #[test]
    fn linear_dimension_bakes_a_line_it_and_measure() {
        let d = author_dimension(
            &linear(),
            ScaleState::Calibrated { scale: 0.01 },
            NumberFormat::decimal(Unit::Meter, 2),
        );
        assert_eq!(d.label, "1.00 m");
        assert_eq!(
            d.annot
                .get(b"Subtype")
                .unwrap()
                .as_name()
                .unwrap()
                .as_bytes(),
            b"Line"
        );
        assert_eq!(
            d.annot.get(b"IT").unwrap().as_name().unwrap().as_bytes(),
            b"LineDimension"
        );
        assert!(d.annot.get(b"L").is_some());
        assert!(d.annot.get(b"Measure").is_some());
        assert_eq!(
            d.annot.get(b"Contents").unwrap(),
            &Object::String(b"1.00 m".to_vec())
        );
        // The /Rect has positive area.
        assert!(d.rect.width() > 0.0 && d.rect.height() > 0.0);
    }

    #[test]
    fn appearance_content_reparses_as_a_content_stream() {
        // The baked /AP must never emit a stream the tokenizer rejects (R59
        // renders it; a malformed stream would fail render).
        let d = author_dimension(
            &linear(),
            ScaleState::Calibrated { scale: 0.01 },
            NumberFormat::decimal(Unit::Meter, 2),
        );
        ContentStream::parse(d.ap_content.clone()).expect("baked /AP must reparse");
        // The label text is shown, and a font is set.
        let s = String::from_utf8(d.ap_content.clone()).unwrap();
        assert!(s.contains("/Helv 10 Tf"), "{s}");
        assert!(s.contains("(1.00 m) Tj"), "{s}");
    }

    #[test]
    fn never_set_scale_bakes_raw_units_and_no_measure() {
        let d = author_dimension(
            &linear(),
            ScaleState::NeverSet,
            NumberFormat::decimal(Unit::Meter, 2),
        );
        assert_eq!(d.label, "100.00 pt");
        assert!(d.annot.get(b"Measure").is_none());
    }

    #[test]
    fn circular_dimension_prefixes_r_or_dia() {
        let fit = FitCircle {
            center: Point::new(50.0, 50.0),
            radius: 20.0,
            residual: 0.1,
        };
        let r = author_dimension(
            &DimensionKind::Circular {
                fit,
                show_diameter: false,
            },
            ScaleState::Calibrated { scale: 0.05 },
            NumberFormat::decimal(Unit::Centimeter, 2),
        );
        assert!(r.label.starts_with("R "), "{}", r.label);
        let dia = author_dimension(
            &DimensionKind::Circular {
                fit,
                show_diameter: true,
            },
            ScaleState::Calibrated { scale: 0.05 },
            NumberFormat::decimal(Unit::Centimeter, 2),
        );
        assert!(dia.label.starts_with("DIA "), "{}", dia.label);
        // Diameter reads twice the radius.
        assert_eq!(r.label, "R 1.00 cm"); // 20 pt * 0.05 = 1.0
        assert_eq!(dia.label, "DIA 2.00 cm"); // 40 pt * 0.05 = 2.0
    }

    #[test]
    fn regeneration_is_deterministic() {
        // Same inputs → byte-identical appearance (regeneration-safe).
        let a = author_dimension(
            &linear(),
            ScaleState::OneToOne,
            NumberFormat::decimal(Unit::Inch, 2),
        );
        let b = author_dimension(
            &linear(),
            ScaleState::OneToOne,
            NumberFormat::decimal(Unit::Inch, 2),
        );
        assert_eq!(a, b);
    }
}
