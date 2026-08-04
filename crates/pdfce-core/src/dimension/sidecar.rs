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
use super::group::{DimensionId, DimensionKind, DimensionModel, DimensionRecord, Group, GroupId};
use super::units::{FractionMode, NumberFormat, ScaleState, Unit};

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
#[must_use]
pub fn deserialize_model(obj: &Object) -> Option<DimensionModel> {
    let d = obj.as_dict()?;
    // Version gate: only recognise what we wrote (forward-compat: a newer
    // major would be a different, unhandled version → None → fresh model).
    if d.get(b"Version").and_then(Object::as_int) != Some(SIDECAR_VERSION) {
        return None;
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
        format: NumberFormat { unit, fraction },
        ocg,
        visible,
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
            offset: d.get(b"Offset").and_then(Object::as_number).unwrap_or(0.0),
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

fn point_of(obj: &Object) -> Option<Point> {
    let a = obj.as_array()?;
    let x = a.first()?.as_number()?;
    let y = a.get(1)?.as_number()?;
    Some(Point::new(x, y))
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
