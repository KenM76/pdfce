//! # The authoritative `/PieceInfo` sidecar (ISO 32000-1 §14.5, decision 011 §2.4)
//!
//! Serialise the [`DimensionModel`] to — and parse it back from — the PDF
//! object graph, for storage under the document catalog's
//! `/PieceInfo /pdfce /Private` (§14.5 Table 319). This is pdfce's
//! **authoritative** dimensioning model: groups, scale, units, number format,
//! per-dimension geometry, best-fit params + residual, and the annotation/`/AP`
//! wiring handles the scale-repropagation needs.
//!
//! ## Why `/PieceInfo` is authoritative but its cross-tool survival is not spec-guaranteed
//!
//! Per `iso32000__s__14.5.md` (NOTE 1): private `/PieceInfo` data **"may be
//! ignored by general-purpose conforming readers"** — there is **no** ISO
//! preservation guarantee. Survival across a pdfce round-trip is guaranteed
//! only by **pdfce's own R34 minimal-diff save** (untouched objects re-emitted
//! byte-identical), not by §14.5. That is exactly why the load-bearing scale is
//! **also** mirrored into a reader-visible `/Measure` dict
//! ([`super::measure_dict`]): if a foreign editor drops `/PieceInfo`, the
//! `/Measure`-encoded scale still survives. On load, native-vs-sidecar
//! disagreement ⇒ disclose + prefer the sidecar (decision 011 §2.4, Z8).
//!
//! ## Format
//!
//! A self-describing `Object::Dict` with `/Version`, a `/Groups` array, and a
//! `/Dimensions` array. Deserialisation is **total and lenient** — a malformed
//! or partial sidecar yields `None` (the caller then starts fresh rather than
//! panicking), and unknown keys are ignored (forward-compat, §14.5's
//! `PictureEdit`/`PictureEditExtended` pattern).

use crate::object::{Dict, Name, Object};
use crate::vector::{AxisConstraint, Point};

use super::fit::FitCircle;
use super::group::{
    DimStandard, DimensionId, DimensionKind, DimensionModel, DimensionRecord, Group, GroupId,
};
use super::units::{DecimalMarker, FractionMode, NumberFormat, ScaleState, Unit};

/// The sidecar schema version pdfce writes (bumped only on a breaking layout
/// change; readers ignore unknown extra keys, §14.5 forward-compat).
pub const SIDECAR_VERSION: i64 = 1;

/// Serialise the whole [`DimensionModel`] to the `Object` pdfce stores as the
/// `/PieceInfo /pdfce /Private` value (§14.5). Deterministic — the same model
/// always yields the same bytes, so a no-change save is a no-op (R34).
#[must_use]
pub fn serialize_model(model: &DimensionModel) -> Object {
    let mut d = Dict::new();
    d.insert(Name::from(b"Version"), Object::Integer(SIDECAR_VERSION));
    d.insert(
        Name::from(b"Groups"),
        Object::Array(model.groups().iter().map(serialize_group).collect()),
    );
    d.insert(
        Name::from(b"Dimensions"),
        Object::Array(model.dimensions().iter().map(serialize_dimension).collect()),
    );
    Object::Dict(d)
}

/// Parse a [`DimensionModel`] back from a stored sidecar `Object`. `None` if
/// the object is not a dict, is not the recognised schema, or is missing the
/// group/dimension arrays (the caller then starts a fresh model).
/// The schema version a sidecar object declares, or `None` if it is not a
/// recognisable sidecar at all.
///
/// Exists so the write side can tell "this file has no pdfce sidecar" (fine,
/// start one) from "this file's sidecar was written by a newer pdfce than this
/// one" (refuse to overwrite it). Those two look identical to
/// [`deserialize_model`], and treating the second as the first is how an
/// operator's calibrated scales get silently destroyed by an older build.
#[must_use]
pub fn sidecar_version(obj: &Object) -> Option<i64> {
    obj.as_dict()?.get(b"Version").and_then(Object::as_int)
}

#[must_use]
pub fn deserialize_model(obj: &Object) -> Option<DimensionModel> {
    let d = obj.as_dict()?;
    // Version gate — a RANGE, not an equality.
    //
    // This used to demand exact equality and answer `None` on any mismatch,
    // which the caller turns into a FRESH model. That is silent data loss in
    // both directions: an older sidecar would be discarded on the first
    // version bump, and a sidecar written by a NEWER pdfce is discarded today,
    // taking every group, every calibrated scale and every membership with it
    // — while the `/Line` annotations keep rendering perfectly, so nothing
    // looks wrong until the next save makes it permanent.
    //
    // Older is readable because every key this schema has ever gained is
    // OPTIONAL with a default (see `/Offset`, `/TextAlong`), so an old
    // document is simply one that used the defaults.
    //
    // NEWER is a different problem and is NOT solved here: this returns the
    // groups and dimensions it can understand, and [`sidecar_version`] lets
    // the session refuse to WRITE over a file it cannot fully represent
    // (`EditError::SidecarWrittenByNewerBuild`). Reading is safe; writing is
    // what would destroy the parts this build does not know about.
    let version = d.get(b"Version").and_then(Object::as_int)?;
    if version > SIDECAR_VERSION {
        // Still parsed, not refused — a reader should show what it can. The
        // write-side guard is the session's.
    }
    let mut model = DimensionModel::empty();
    if let Some(groups) = d.get(b"Groups").and_then(Object::as_array) {
        for g in groups {
            if let Some(group) = deserialize_group(g) {
                model.insert_group(group);
            }
        }
    }
    if let Some(dims) = d.get(b"Dimensions").and_then(Object::as_array) {
        for dim in dims {
            if let Some(record) = deserialize_dimension(dim) {
                model.insert_dimension(record);
            }
        }
    }
    // A model with at least the default group is required to be coherent.
    model.group(super::group::DEFAULT_GROUP_ID)?;
    Some(model)
}

// ---- group (de)serialization ------------------------------------------------

fn serialize_group(g: &Group) -> Object {
    let mut d = Dict::new();
    d.insert(Name::from(b"Id"), Object::Integer(i64::from(g.id.0)));
    d.insert(
        Name::from(b"Name"),
        Object::String(g.name.as_bytes().to_vec()),
    );
    match g.scale {
        ScaleState::NeverSet => {
            d.insert(Name::from(b"Scale"), Object::Name(Name::from(b"never")));
        }
        ScaleState::OneToOne => {
            d.insert(Name::from(b"Scale"), Object::Name(Name::from(b"one")));
        }
        ScaleState::Calibrated { scale } => {
            d.insert(
                Name::from(b"Scale"),
                Object::Name(Name::from(b"calibrated")),
            );
            d.insert(Name::from(b"ScaleValue"), Object::Real(scale));
        }
    }
    d.insert(
        Name::from(b"Unit"),
        Object::String(g.format.unit.token().as_bytes().to_vec()),
    );
    // Written only when NOT the default, so a document that never left ANSI
    // point-decimal keeps byte-identical sidecar output.
    if g.standard != DimStandard::Ansi {
        d.insert(Name::from(b"Standard"), Object::Name(Name::from(b"iso")));
    }
    if g.format.decimal_marker != DecimalMarker::Point {
        d.insert(
            Name::from(b"DecimalMarker"),
            Object::Name(Name::from(b"comma")),
        );
    }
    match g.format.fraction {
        FractionMode::Decimal { places } => {
            d.insert(Name::from(b"Frac"), Object::Name(Name::from(b"decimal")));
            d.insert(Name::from(b"Places"), Object::Integer(i64::from(places)));
        }
        FractionMode::Fraction {
            denominator,
            reduce,
        } => {
            d.insert(Name::from(b"Frac"), Object::Name(Name::from(b"fraction")));
            d.insert(
                Name::from(b"Denom"),
                Object::Integer(i64::from(denominator)),
            );
            d.insert(Name::from(b"Reduce"), Object::Boolean(reduce));
        }
    }
    d.insert(Name::from(b"Visible"), Object::Boolean(g.visible));
    if let Some(ocg) = g.ocg {
        d.insert(Name::from(b"Ocg"), Object::Reference(ocg));
    }
    Object::Dict(d)
}

fn deserialize_group(obj: &Object) -> Option<Group> {
    let d = obj.as_dict()?;
    let id = GroupId(u32::try_from(d.get(b"Id").and_then(Object::as_int)?).ok()?);
    let name = string_of(d.get(b"Name")?)?;
    let unit = Unit::parse(&string_of(d.get(b"Unit")?)?)?;
    let scale = match name_of(d.get(b"Scale"))?.as_slice() {
        b"never" => ScaleState::NeverSet,
        b"one" => ScaleState::OneToOne,
        b"calibrated" => ScaleState::Calibrated {
            scale: d.get(b"ScaleValue").and_then(Object::as_number)?,
        },
        _ => return None,
    };
    let fraction = match name_of(d.get(b"Frac"))?.as_slice() {
        b"decimal" => FractionMode::Decimal {
            places: u32::try_from(d.get(b"Places").and_then(Object::as_int).unwrap_or(2)).ok()?,
        },
        b"fraction" => FractionMode::Fraction {
            denominator: u32::try_from(d.get(b"Denom").and_then(Object::as_int).unwrap_or(16))
                .ok()?,
            reduce: bool_of(d.get(b"Reduce")).unwrap_or(false),
        },
        _ => return None,
    };
    let visible = bool_of(d.get(b"Visible")).unwrap_or(true);
    let ocg = d.get(b"Ocg").and_then(Object::as_reference);
    Some(Group {
        id,
        name,
        scale,
        format: NumberFormat {
            unit,
            fraction,
            // Both OPTIONAL keys at the existing schema version, absent
            // meaning the pre-27.2 behaviour — the same additive discipline
            // `/Offset` and `/TextAlong` use, and for the same reason: a
            // version bump would trip the write-side refusal on every existing
            // dimensioned file.
            decimal_marker: match name_of(d.get(b"DecimalMarker")).as_deref() {
                Some(b"comma") => DecimalMarker::Comma,
                _ => DecimalMarker::Point,
            },
        },
        ocg,
        visible,
        standard: match name_of(d.get(b"Standard")).as_deref() {
            Some(b"iso") => DimStandard::Iso,
            _ => DimStandard::Ansi,
        },
    })
}

// ---- dimension (de)serialization --------------------------------------------

fn serialize_dimension(dim: &DimensionRecord) -> Object {
    let mut d = Dict::new();
    d.insert(Name::from(b"Id"), Object::Integer(i64::from(dim.id.0)));
    d.insert(
        Name::from(b"Group"),
        Object::Integer(i64::from(dim.group.0)),
    );
    match dim.kind {
        DimensionKind::Linear {
            a,
            b,
            constraint,
            offset,
            text_along,
        } => {
            d.insert(Name::from(b"Kind"), Object::Name(Name::from(b"linear")));
            d.insert(Name::from(b"A"), point_array(a));
            d.insert(Name::from(b"B"), point_array(b));
            d.insert(
                Name::from(b"Constraint"),
                Object::Name(Name(constraint_token(constraint).to_vec())),
            );
            // OPTIONAL, and deliberately NOT a schema-version bump.
            //
            // `deserialize_model` gates on `Version` with exact equality and
            // answers `None` on a mismatch — which the caller turns into a
            // FRESH model. Bumping the version for this key would therefore
            // make every older file silently lose every group, every
            // calibrated scale and every membership, while its `/Line`
            // annotations kept rendering perfectly — so nothing would look
            // wrong until the next save made the loss permanent.
            //
            // An absent key reads back as the 0.0 default, which draws exactly
            // what the pre-27.0 build drew. Written only when non-zero, so a
            // file that never used a standoff keeps byte-identical sidecar
            // output.
            if offset != 0.0 {
                d.insert(Name::from(b"Offset"), Object::Real(offset));
            }
            // Same optional-key discipline as /Offset: absent means centred,
            // which is where every pre-27.1 label sits.
            if text_along != 0.0 {
                d.insert(Name::from(b"TextAlong"), Object::Real(text_along));
            }
        }
        DimensionKind::Circular { fit, show_diameter } => {
            d.insert(Name::from(b"Kind"), Object::Name(Name::from(b"circular")));
            d.insert(Name::from(b"Center"), point_array(fit.center));
            d.insert(Name::from(b"Radius"), Object::Real(fit.radius));
            d.insert(Name::from(b"Residual"), Object::Real(fit.residual));
            d.insert(Name::from(b"Diameter"), Object::Boolean(show_diameter));
        }
    }
    if let Some(annot) = dim.annot {
        d.insert(Name::from(b"Annot"), Object::Reference(annot));
    }
    if let Some(ap) = dim.ap {
        d.insert(Name::from(b"Ap"), Object::Reference(ap));
    }
    Object::Dict(d)
}

fn deserialize_dimension(obj: &Object) -> Option<DimensionRecord> {
    let d = obj.as_dict()?;
    let id = DimensionId(u32::try_from(d.get(b"Id").and_then(Object::as_int)?).ok()?);
    let group = GroupId(u32::try_from(d.get(b"Group").and_then(Object::as_int)?).ok()?);
    let kind = match name_of(d.get(b"Kind"))?.as_slice() {
        b"linear" => DimensionKind::Linear {
            a: point_of(d.get(b"A")?)?,
            b: point_of(d.get(b"B")?)?,
            constraint: parse_constraint(&name_of(d.get(b"Constraint"))?)?,
            // Absent in every sidecar written before Pass 27.0. The 0.0
            // default is what makes that migration free rather than lossy.
            offset: placement_of(d.get(b"Offset")),
            text_along: placement_of(d.get(b"TextAlong")),
        },
        b"circular" => DimensionKind::Circular {
            fit: FitCircle {
                center: point_of(d.get(b"Center")?)?,
                radius: d.get(b"Radius").and_then(Object::as_number)?,
                residual: d
                    .get(b"Residual")
                    .and_then(Object::as_number)
                    .unwrap_or(0.0),
            },
            show_diameter: bool_of(d.get(b"Diameter")).unwrap_or(false),
        },
        _ => return None,
    };
    Some(DimensionRecord {
        id,
        group,
        kind,
        annot: d.get(b"Annot").and_then(Object::as_reference),
        ap: d.get(b"Ap").and_then(Object::as_reference),
    })
}

// ---- small object helpers ---------------------------------------------------

fn point_array(p: Point) -> Object {
    Object::Array(vec![Object::Real(p.x), Object::Real(p.y)])
}

/// The largest page-space magnitude a sidecar value may claim (Pass 27.3).
///
/// PDF's own architectural limit for a page dimension is 14,400 units (200
/// inches, Annex C.1), so a coordinate or standoff three orders past that is
/// not geometry — it is corruption, a hand edit, or another product's bug.
/// The ceiling is deliberately generous rather than tight: the job here is to
/// stop absurdity reaching the writer, not to second-guess an unusual drawing.
const MAX_PAGE_VALUE: f64 = 1.0e7;

/// Whether a file-supplied page-space number is usable.
///
/// # Why this guard exists
///
/// These values come out of the FILE, and everything downstream of them is
/// geometry that ends up in `/Rect` and `/L`. Measured on 2026-08-05, with no
/// guard:
///
/// - `/Offset 1e308` wrote a **300-digit decimal** into `/Rect`, far past
///   PDF's ~3.4e38 architectural limit for a real;
/// - `/Offset inf` made the dimension **silently vanish** — `/Rect [-2 -2 3
///   3]`, `/L [0 0 0 0]` — while `/Contents` still read "200.00 pt". A
///   measurement that disappears while still claiming a value is the worst of
///   the available outcomes, because nothing on screen says anything is wrong.
///
/// The bounds accumulator already drops non-finite points, which is what makes
/// the failure quiet rather than loud. This stops it upstream instead.
fn usable_page_value(v: f64) -> bool {
    v.is_finite() && v.abs() <= MAX_PAGE_VALUE
}

/// A file-supplied placement scalar, or the 0.0 default if it is unusable.
///
/// Defaulting rather than dropping the record: a standoff is a presentation
/// detail with a meaningful zero, so a corrupt one costs the operator the
/// dimension's POSITION, not the dimension. The measured points are held to a
/// stricter standard below, because a dimension whose geometry is corrupt has
/// no meaning to preserve.
fn placement_of(obj: Option<&Object>) -> f64 {
    obj.and_then(Object::as_number)
        .filter(|v| usable_page_value(*v))
        .unwrap_or(0.0)
}

fn point_of(obj: &Object) -> Option<Point> {
    let a = obj.as_array()?;
    let x = a.first()?.as_number()?;
    let y = a.get(1)?.as_number()?;
    // `None` drops the whole dimension record — the sidecar's existing
    // malformed-entry posture. A measured point that is infinite or absurd
    // does not describe anything, and keeping the record would mean drawing a
    // dimension between coordinates nobody chose.
    (usable_page_value(x) && usable_page_value(y)).then(|| Point::new(x, y))
}

const fn constraint_token(c: AxisConstraint) -> &'static [u8] {
    match c {
        AxisConstraint::Aligned => b"aligned",
        AxisConstraint::Horizontal => b"horizontal",
        AxisConstraint::Vertical => b"vertical",
    }
}

fn parse_constraint(bytes: &[u8]) -> Option<AxisConstraint> {
    match bytes {
        b"aligned" => Some(AxisConstraint::Aligned),
        b"horizontal" => Some(AxisConstraint::Horizontal),
        b"vertical" => Some(AxisConstraint::Vertical),
        _ => None,
    }
}

fn string_of(obj: &Object) -> Option<String> {
    match obj {
        Object::String(bytes) => Some(String::from_utf8_lossy(bytes).into_owned()),
        _ => None,
    }
}

fn name_of(obj: Option<&Object>) -> Option<Vec<u8>> {
    obj?.as_name().map(|n| n.as_bytes().to_vec())
}

fn bool_of(obj: Option<&Object>) -> Option<bool> {
    match obj? {
        Object::Boolean(b) => Some(*b),
        _ => None,
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
    use crate::dimension::group::DEFAULT_GROUP_ID;
    use crate::object::ObjId;

    fn sample_model() -> DimensionModel {
        let mut m = DimensionModel::new();
        // Calibrate the default group and add a couple of groups + dims.
        m.set_group_scale(
            DEFAULT_GROUP_ID,
            ScaleState::Calibrated { scale: 0.05 },
            NumberFormat::decimal(Unit::Meter, 3),
        );
        let fp = m.add_group("Floor Plan", Unit::FeetInches);
        m.set_group_scale(
            fp,
            ScaleState::OneToOne,
            NumberFormat::feet_inches(8, false),
        );
        m.set_group_visible(fp, false);
        let d1 = m.add_dimension(
            DEFAULT_GROUP_ID,
            DimensionKind::Linear {
                a: Point::new(1.0, 2.0),
                b: Point::new(3.0, 4.0),
                constraint: AxisConstraint::Horizontal,
                offset: 0.0,
                text_along: 0.0,
            },
        );
        // Wire fake object handles to prove they round-trip.
        m.dimension_mut(d1).unwrap().annot = Some(ObjId::new(20, 0));
        m.dimension_mut(d1).unwrap().ap = Some(ObjId::new(21, 0));
        m.add_dimension(
            fp,
            DimensionKind::Circular {
                fit: FitCircle {
                    center: Point::new(50.0, 60.0),
                    radius: 12.5,
                    residual: 0.3,
                },
                show_diameter: true,
            },
        );
        m.group_mut(fp).unwrap().ocg = Some(ObjId::new(30, 0));
        m
    }

    #[test]
    fn model_round_trips_through_the_sidecar() {
        let m = sample_model();
        let obj = serialize_model(&m);
        let back = deserialize_model(&obj).expect("valid sidecar");
        assert_eq!(back, m, "sidecar round-trip must be lossless");
    }

    #[test]
    fn a_malformed_sidecar_yields_none_not_panic() {
        assert!(deserialize_model(&Object::Null).is_none());
        assert!(deserialize_model(&Object::Integer(3)).is_none());
        // Wrong version.
        let mut d = Dict::new();
        d.insert(Name::from(b"Version"), Object::Integer(999));
        assert!(deserialize_model(&Object::Dict(d)).is_none());
        // Right version but no default group → incoherent → None.
        let mut d2 = Dict::new();
        d2.insert(Name::from(b"Version"), Object::Integer(SIDECAR_VERSION));
        d2.insert(Name::from(b"Groups"), Object::Array(vec![]));
        assert!(deserialize_model(&Object::Dict(d2)).is_none());
    }

    #[test]
    fn wiring_handles_and_ocg_survive_the_round_trip() {
        let m = sample_model();
        let back = deserialize_model(&serialize_model(&m)).unwrap();
        let d1 = back.dimensions()[0];
        assert_eq!(d1.annot, Some(ObjId::new(20, 0)));
        assert_eq!(d1.ap, Some(ObjId::new(21, 0)));
        let fp = back
            .groups()
            .iter()
            .find(|g| g.name == "Floor Plan")
            .unwrap();
        assert_eq!(fp.ocg, Some(ObjId::new(30, 0)));
        assert!(!fp.visible);
    }
}
