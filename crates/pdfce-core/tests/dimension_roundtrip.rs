//! Integration tests for the Pass 12.M2 dimensioning subsystem's in-document
//! wiring (decision 011 §2.4): additive authoring (existing content
//! byte-verbatim), the hybrid storage (`/Line` + `/IT /LineDimension` + baked
//! `/AP` + portable `/Measure` mirror + authoritative `/PieceInfo` sidecar),
//! the per-group `/OCG` layer registered in `/OCProperties`, and the
//! scale-change → regenerate-all-members story.
//!
//! Public-API only (the same surface the CLI/GUI use). A minimal synthetic PDF
//! is built inline (catalog = obj 1, pages = obj 2, page = obj 3).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use pdfce_core::dimension::{
    DEFAULT_GROUP_ID, DimensionKind, NumberFormat, ScaleState, Unit, deserialize_model,
};
use pdfce_core::document::Document;
use pdfce_core::edit::EditSession;
use pdfce_core::graph::ObjectGraph;
use pdfce_core::object::{ObjId, Object};
use pdfce_core::vector::{AxisConstraint, Point};
use pdfce_core::writer::SaveOptions;

/// Build a minimal one-page PDF: catalog(1) → pages(2) → page(3).
fn minimal_pdf() -> Vec<u8> {
    minimal_pdf_with_catalog("<< /Type /Catalog /Pages 2 0 R >>")
}

/// The same one-page PDF with an arbitrary catalog body — so a test can plant
/// a sidecar the writer would never produce (a newer schema version) without
/// duplicating the byte-offset bookkeeping, which is the part that is easy to
/// get subtly wrong and hard to notice.
fn minimal_pdf_with_catalog(catalog: &str) -> Vec<u8> {
    let bodies = [
        catalog,
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Resources << >> >>",
    ];
    let mut buf = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in bodies.iter().enumerate() {
        offsets.push(buf.len());
        buf.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", i + 1).as_bytes());
    }
    let xref_at = buf.len();
    let size = bodies.len() + 1;
    buf.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for off in &offsets {
        buf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n")
            .as_bytes(),
    );
    buf
}

fn linear() -> DimensionKind {
    DimensionKind::Linear {
        a: Point::new(100.0, 200.0),
        b: Point::new(300.0, 200.0),
        constraint: AxisConstraint::Horizontal,
        offset: 0.0,
        text_along: 0.0,
    }
}

fn session() -> (Vec<u8>, EditSession) {
    let bytes = minimal_pdf();
    let doc = Document::from_bytes(bytes.clone()).unwrap();
    (bytes, EditSession::new(doc))
}

fn save(session: &EditSession) -> Vec<u8> {
    session
        .to_incremental_bytes(&SaveOptions::identity())
        .unwrap()
        .0
}

#[test]
fn dimension_is_additive_existing_content_byte_verbatim() {
    // The R46 zero-exception acceptance: an additive dimension leaves every
    // original byte in place (incremental append), so the saved file starts
    // with the original file verbatim.
    let (original, mut s) = session();
    s.add_dimension(0, DEFAULT_GROUP_ID, linear()).unwrap();
    let out = save(&s);
    assert!(
        out.starts_with(&original),
        "an additive dimension must not modify any original byte"
    );
    assert!(
        out.len() > original.len(),
        "the dimension objects were appended"
    );
}

#[test]
fn dimension_authors_line_it_oc_measure_and_registers_the_ocg() {
    let (_orig, mut s) = session();
    // Calibrate the default group first so the dimension carries a /Measure.
    s.set_group_scale(
        DEFAULT_GROUP_ID,
        ScaleState::Calibrated { scale: 0.01 },
        NumberFormat::decimal(Unit::Meter, 2),
    )
    .unwrap();
    let (annot_id, _dim) = s.add_dimension(0, DEFAULT_GROUP_ID, linear()).unwrap();

    let reloaded = Document::from_bytes(save(&s)).unwrap();
    let Object::Dict(annot) = &reloaded.get(annot_id).unwrap().value else {
        panic!("annotation is not a dict");
    };
    assert_eq!(
        annot.get(b"Subtype").unwrap().as_name().unwrap().as_bytes(),
        b"Line"
    );
    assert_eq!(
        annot.get(b"IT").unwrap().as_name().unwrap().as_bytes(),
        b"LineDimension"
    );
    assert!(annot.get(b"AP").is_some(), "baked /AP present");
    assert!(
        annot.get(b"Measure").is_some(),
        "portable /Measure scale mirror present"
    );
    // The /OC points at the group OCG.
    let oc = annot
        .get(b"OC")
        .and_then(Object::as_reference)
        .expect("/OC ref");

    // The OCG is registered in the catalog /OCProperties (MANDATORY §8.11.4.2).
    let catalog = reloaded.catalog().unwrap();
    let ocp = catalog.get(b"OCProperties").unwrap().as_dict().unwrap();
    let ocgs = ocp.get(b"OCGs").unwrap().as_array().unwrap();
    assert!(
        ocgs.contains(&Object::Reference(oc)),
        "the annotation's OCG must be registered in /OCProperties /OCGs"
    );
    assert!(
        ocp.get(b"D")
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"Order")
            .is_some()
    );
}

#[test]
fn sidecar_survives_the_save_round_trip_and_matches_the_session_model() {
    let (_orig, mut s) = session();
    s.add_dimension(0, DEFAULT_GROUP_ID, linear()).unwrap();
    let g = s
        .add_dimension_group("Floor Plan", Unit::FeetInches)
        .unwrap();
    s.set_group_scale(
        g,
        ScaleState::Calibrated { scale: 0.002 },
        NumberFormat::feet_inches(8, false),
    )
    .unwrap();
    let expected = s.dimension_model();

    // Reload from disk and read the /PieceInfo /pdfce /Private sidecar back.
    let reloaded = Document::from_bytes(save(&s)).unwrap();
    let catalog = reloaded.catalog().unwrap();
    let piece = catalog.get(b"PieceInfo").unwrap().as_dict().unwrap();
    let pdfce = piece.get(b"pdfce").unwrap().as_dict().unwrap();
    // §14.5 Table 319: /LastModified is Required.
    assert!(pdfce.get(b"LastModified").is_some());
    let private = pdfce.get(b"Private").unwrap();
    let recovered = deserialize_model(private).expect("sidecar deserializes");

    assert_eq!(
        recovered.groups().len(),
        expected.groups().len(),
        "every group survived"
    );
    assert_eq!(recovered.dimensions().len(), 1, "the dimension survived");
    // The Floor Plan group's feet-inches scale survived exactly.
    let fp = recovered
        .groups()
        .iter()
        .find(|gr| gr.name == "Floor Plan")
        .unwrap();
    assert_eq!(fp.unit(), Unit::FeetInches);
    assert!(matches!(fp.scale, ScaleState::Calibrated { .. }));
}

#[test]
fn changing_group_scale_regenerates_all_member_labels() {
    // The decision-011 headline: change the group scale → all members update.
    let (_orig, mut s) = session();
    let (a_id, _) = s.add_dimension(0, DEFAULT_GROUP_ID, linear()).unwrap();
    let (b_id, _) = s
        .add_dimension(
            0,
            DEFAULT_GROUP_ID,
            DimensionKind::Linear {
                a: Point::new(0.0, 0.0),
                b: Point::new(100.0, 0.0),
                constraint: AxisConstraint::Horizontal,
                offset: 0.0,
                text_along: 0.0,
            },
        )
        .unwrap();

    // Calibrate: 1 pt = 0.01 m. The 200-pt and 100-pt lines become 2 m and 1 m.
    let regenerated = s
        .set_group_scale(
            DEFAULT_GROUP_ID,
            ScaleState::Calibrated { scale: 0.01 },
            NumberFormat::decimal(Unit::Meter, 2),
        )
        .unwrap();
    assert_eq!(regenerated, 2, "both members regenerated in one command");

    let reloaded = Document::from_bytes(save(&s)).unwrap();
    let contents = |id: ObjId| -> String {
        let Object::Dict(d) = &reloaded.get(id).unwrap().value else {
            panic!()
        };
        match d.get(b"Contents").unwrap() {
            Object::String(bytes) => String::from_utf8_lossy(bytes).into_owned(),
            _ => panic!("contents not a string"),
        }
    };
    assert_eq!(contents(a_id), "2.00 m");
    assert_eq!(contents(b_id), "1.00 m");
}

/// **Moving a ce dimension relocates it without re-measuring it.**
///
/// A translation is a rigid motion, so the measured value must be identical
/// before and after — moving a dimension repositions the annotation, it does
/// not change what was measured. The `/Rect` and `/L` must shift by exactly
/// the requested delta.
///
/// `/L` is the assertion that matters. The regeneration this shares with
/// `set_group_scale` used to rewrite only `/Rect`, `/Contents` and `/Measure`,
/// which is indistinguishable from correct for a scale change and leaves a
/// moved dimension's measured line (§12.5.6.7) pointing at where it used to
/// be — a file that renders right and reports the wrong endpoints to every
/// other reader.
#[test]
fn moving_a_ce_dimension_shifts_its_geometry_and_keeps_its_value() {
    let (_orig, mut s) = session();
    let (annot_id, dim_id) = s.add_dimension(0, DEFAULT_GROUP_ID, linear()).unwrap();
    s.set_group_scale(
        DEFAULT_GROUP_ID,
        ScaleState::Calibrated { scale: 0.01 },
        NumberFormat::decimal(Unit::Meter, 2),
    )
    .unwrap();

    let read = |bytes: Vec<u8>| -> (Vec<f64>, Vec<f64>, String) {
        let doc = Document::from_bytes(bytes).unwrap();
        let Object::Dict(d) = &doc.get(annot_id).unwrap().value else {
            panic!("annotation is not a dictionary")
        };
        let nums = |key: &[u8]| -> Vec<f64> {
            d.get(key)
                .and_then(Object::as_array)
                .map(|a| a.iter().filter_map(Object::as_number).collect())
                .unwrap_or_default()
        };
        let label = match d.get(b"Contents").unwrap() {
            Object::String(b) => String::from_utf8_lossy(b).into_owned(),
            _ => panic!("contents not a string"),
        };
        (nums(b"Rect"), nums(b"L"), label)
    };

    let (rect0, l0, label0) = read(save(&s));
    assert_eq!(
        l0.len(),
        4,
        "a /Line annotation carries /L as [x1 y1 x2 y2]"
    );

    s.move_dimension(dim_id, 25.0, -10.0).unwrap();
    let (rect1, l1, label1) = read(save(&s));

    assert_eq!(
        label1, label0,
        "a translation preserves every distance, so the measured value must not change"
    );
    for (i, (after, before)) in l1.iter().zip(&l0).enumerate() {
        let expected = before + if i % 2 == 0 { 25.0 } else { -10.0 };
        assert!(
            (after - expected).abs() < 0.001,
            "/L component {i} must shift by the requested delta: {after} vs {expected}"
        );
    }
    for (i, (after, before)) in rect1.iter().zip(&rect0).enumerate() {
        let expected = before + if i % 2 == 0 { 25.0 } else { -10.0 };
        assert!(
            (after - expected).abs() < 0.001,
            "/Rect component {i} must shift by the requested delta: {after} vs {expected}"
        );
    }
}

/// `dimension_rects` reports what is CURRENTLY on the page, overlay and all.
///
/// The overlay half is the point: a shell hit-tests this to let an operator
/// click a ce dimension, so it has to describe where the dimension is now, not
/// where the file said it was on open. Reporting the stale rect would make a
/// moved dimension clickable at its old position and dead at its new one.
#[test]
fn dimension_rects_reports_the_current_position_not_the_opened_one() {
    let (_orig, mut s) = session();
    let (_annot, dim_id) = s.add_dimension(0, DEFAULT_GROUP_ID, linear()).unwrap();

    let before = s.dimension_rects(0);
    assert_eq!(before.len(), 1, "the authored dimension is on page 0");
    assert_eq!(before[0].0, dim_id);

    s.move_dimension(dim_id, 30.0, 15.0).unwrap();
    let after = s.dimension_rects(0);
    assert_eq!(after.len(), 1);
    for (i, (a, b)) in after[0].1.iter().zip(&before[0].1).enumerate() {
        let expected = b + if i % 2 == 0 { 30.0 } else { 15.0 };
        assert!(
            (a - expected).abs() < 0.001,
            "rect component {i} must reflect the move already made this session"
        );
    }

    // A page with no ce dimensions reports none rather than every page's.
    assert!(
        s.dimension_rects(7).is_empty(),
        "an out-of-range page must be empty, not a fallback to page 0"
    );
}

/// **Deleting a ce dimension removes all four of its traces.**
///
/// The interesting assertion is not "it's gone from the page" — it is that the
/// SIDECAR record went too. Leaving it would make pdfce keep believing in a
/// dimension the file no longer contains, and the next group-wide re-format
/// would try to regenerate an annotation that is not there.
#[test]
fn deleting_a_ce_dimension_removes_the_annotation_and_the_sidecar_record() {
    let (_orig, mut s) = session();
    let (annot_id, dim_id) = s.add_dimension(0, DEFAULT_GROUP_ID, linear()).unwrap();
    assert_eq!(s.dimension_rects(0).len(), 1);

    s.delete_dimension(dim_id).unwrap();

    assert!(
        s.dimension_rects(0).is_empty(),
        "nothing is left on the page to click"
    );
    assert!(
        s.dimension_model().dimension(dim_id).is_none(),
        "the sidecar record must go too, or pdfce keeps believing in it"
    );
    // The group survives on purpose — it carries a calibrated scale that is
    // not cheap to redo, and losing it as a side effect would be silent.
    assert!(
        s.dimension_model().group(DEFAULT_GROUP_ID).is_some(),
        "removing the last member must not take the group with it"
    );

    // On reload the page must not still point at the annotation, and the
    // annotation object must be gone. Checked from the saved FILE rather than
    // the session, because a removal that only exists in the overlay would
    // pass every in-memory assertion and still ship a dangling reference.
    let reloaded = Document::from_bytes(save(&s)).unwrap();
    let pages = pdfce_core::page_tree::pages(&reloaded).unwrap();
    let page = reloaded.get(pages[0].id).unwrap().value.clone();
    let refs: Vec<ObjId> = page
        .as_dict()
        .and_then(|d| d.get(b"Annots").cloned())
        .map(|a| reloaded.view().resolve(&a).clone())
        .and_then(|a| {
            a.as_array()
                .map(|arr| arr.iter().filter_map(Object::as_reference).collect())
        })
        .unwrap_or_default();
    assert!(
        !refs.contains(&annot_id),
        "the /Annots reference must be dropped, or the page points at nothing: {refs:?}"
    );
}

/// Undo restores a deleted ce dimension completely.
#[test]
fn undoing_a_ce_dimension_delete_restores_it() {
    let (_orig, mut s) = session();
    let (_annot, dim_id) = s.add_dimension(0, DEFAULT_GROUP_ID, linear()).unwrap();
    let before = save(&s);
    s.delete_dimension(dim_id).unwrap();
    assert_ne!(save(&s), before, "the delete must actually change the file");
    s.undo().expect("undo the delete");
    assert_eq!(
        save(&s),
        before,
        "undoing a delete must restore the byte-identical prior save"
    );
    assert_eq!(
        s.dimension_rects(0).len(),
        1,
        "and it must be clickable again"
    );
}

/// A stale id is refused by name rather than silently doing nothing.
#[test]
fn deleting_an_unknown_ce_dimension_is_refused_by_name() {
    let (_orig, mut s) = session();
    let err = s
        .delete_dimension(pdfce_core::dimension::DimensionId(999))
        .expect_err("an unknown id must refuse");
    assert!(
        matches!(
            err,
            pdfce_core::edit::EditError::DimensionNotFound { id: 999 }
        ),
        "got {err:?}"
    );
}

/// **The defect the operator reported, pinned as an invariant.**
///
/// *"It looks like it give me the correct horizontal or vertical dimension but
/// it shows at an angle."* — 2026-08-04.
///
/// `leader_endpoints` returned the two PICKED points verbatim, so a dimension
/// constrained to Horizontal was drawn along whatever angle the two clicks
/// happened to make, while `measured_length` correctly reported only the
/// horizontal component. The line disagreed with its own caption.
///
/// The invariant is the fix stated as a property: the dimension line's length
/// equals the measured value, for every constraint and every pick pair. That
/// is checked here over a spread of inputs rather than one, because the old
/// behaviour was CORRECT whenever the picks happened to be axis-aligned — a
/// single well-chosen example would have passed before the fix.
#[test]
fn the_drawn_line_is_exactly_as_long_as_the_number_printed_on_it() {
    use pdfce_core::vector::AxisConstraint;

    let picks = [
        ((100.0, 200.0), (300.0, 200.0)), // already horizontal
        ((100.0, 200.0), (300.0, 260.0)), // the reported case: skewed
        ((300.0, 260.0), (100.0, 200.0)), // reversed pick order
        ((100.0, 200.0), (100.0, 400.0)), // already vertical
        ((120.0, 180.0), (260.0, 90.0)),  // skewed the other way
    ];
    for constraint in [
        AxisConstraint::Horizontal,
        AxisConstraint::Vertical,
        AxisConstraint::Aligned,
    ] {
        for (offset, ((ax, ay), (bx, by))) in
            [0.0_f64, 25.0, -40.0].into_iter().zip(picks.iter().cycle())
        {
            let kind = DimensionKind::Linear {
                a: Point::new(*ax, *ay),
                b: Point::new(*bx, *by),
                constraint,
                offset,
                text_along: 0.0,
            };
            let Some((dim_a, dim_b, ext_a, ext_b)) = kind.linear_geometry() else {
                continue; // degenerate aligned pick: no axis, refused by design
            };
            let drawn = (dim_b.x - dim_a.x).hypot(dim_b.y - dim_a.y);
            let measured = kind.measured_points();
            assert!(
                (drawn - measured).abs() < 0.001,
                "{constraint:?} offset={offset}: the drawn line is {drawn} long but the \
                 label says {measured}"
            );
            // And the extension lines really do reach the measured points —
            // that is the other half of what was asked for.
            assert_eq!(
                (ext_a.x, ext_a.y),
                (*ax, *ay),
                "the first extension line must anchor on the first picked point"
            );
            assert_eq!((ext_b.x, ext_b.y), (*bx, *by));
        }
    }
}

/// A horizontal ce dimension is drawn HORIZONTALLY even when the picks are not.
///
/// The invariant above would also be satisfied by a line of the right length
/// pointing the wrong way; this pins the direction.
#[test]
fn a_constrained_dimension_line_runs_along_its_constraint() {
    use pdfce_core::vector::AxisConstraint;

    let h = DimensionKind::Linear {
        a: Point::new(100.0, 200.0),
        b: Point::new(300.0, 260.0),
        constraint: AxisConstraint::Horizontal,
        offset: 0.0,
        text_along: 0.0,
    };
    let (a, b, _, _) = h.linear_geometry().unwrap();
    assert!(
        (a.y - b.y).abs() < 0.001,
        "a horizontal dimension's line must have equal y at both ends, got {a:?} {b:?}"
    );

    let v = DimensionKind::Linear {
        a: Point::new(100.0, 200.0),
        b: Point::new(160.0, 400.0),
        constraint: AxisConstraint::Vertical,
        offset: 0.0,
        text_along: 0.0,
    };
    let (a, b, _, _) = v.linear_geometry().unwrap();
    assert!(
        (a.x - b.x).abs() < 0.001,
        "a vertical dimension's line must have equal x at both ends, got {a:?} {b:?}"
    );
}

/// The standoff's SIGN must not depend on which point was clicked first.
///
/// Without canonicalising the normal, clicking right-to-left negates it, and
/// the same positive offset puts the dimension line on the opposite side of
/// the drawing — which an operator experiences as the control working
/// backwards at random.
#[test]
fn the_standoff_direction_does_not_depend_on_pick_order() {
    use pdfce_core::vector::AxisConstraint;

    let forward = DimensionKind::Linear {
        a: Point::new(100.0, 200.0),
        b: Point::new(300.0, 200.0),
        constraint: AxisConstraint::Horizontal,
        offset: 30.0,
        text_along: 0.0,
    };
    let backward = DimensionKind::Linear {
        a: Point::new(300.0, 200.0),
        b: Point::new(100.0, 200.0),
        constraint: AxisConstraint::Horizontal,
        offset: 30.0,
        text_along: 0.0,
    };
    let (fa, _, _, _) = forward.linear_geometry().unwrap();
    let (ba, _, _, _) = backward.linear_geometry().unwrap();
    assert!(
        fa.y > 200.0 && ba.y > 200.0,
        "a positive standoff must put the line ABOVE the feature either way: \
         {fa:?} vs {ba:?}"
    );
}

/// A sidecar written before the offset field existed still loads completely.
///
/// The hazard this guards is specific and severe: `deserialize_model` gates on
/// `Version` with exact equality and answers `None` on a mismatch, which the
/// caller turns into a FRESH model — so a schema-version bump would have
/// silently discarded every group, every calibrated scale and every membership
/// of every existing dimensioned file, while the `/Line` annotations kept
/// rendering perfectly. `/Offset` is therefore an OPTIONAL key at the existing
/// version, and this proves the old shape still round-trips.
#[test]
fn a_sidecar_without_the_offset_key_still_loads_every_group_and_dimension() {
    use pdfce_core::object::{Dict, Name};

    let (_orig, mut s) = session();
    s.add_dimension(0, DEFAULT_GROUP_ID, linear()).unwrap();
    s.set_group_scale(
        DEFAULT_GROUP_ID,
        ScaleState::Calibrated { scale: 0.01 },
        NumberFormat::decimal(Unit::Meter, 2),
    )
    .unwrap();

    // Serialise, then strip /Offset from every dimension — exactly the shape a
    // pre-27.0 build wrote.
    let serialized = pdfce_core::dimension::serialize_model(&s.dimension_model());
    let mut d: Dict = serialized.as_dict().unwrap().clone();
    let stripped: Vec<Object> = d
        .get(b"Dimensions")
        .and_then(Object::as_array)
        .unwrap()
        .iter()
        .map(|dim| {
            let mut c = dim.as_dict().unwrap().clone();
            c.remove(b"Offset");
            Object::Dict(c)
        })
        .collect();
    d.insert(Name::from(b"Dimensions"), Object::Array(stripped));

    let recovered = deserialize_model(&Object::Dict(d)).expect("an older sidecar must still load");
    assert_eq!(
        recovered.dimensions().len(),
        1,
        "the dimension must survive a sidecar with no /Offset"
    );
    assert!(
        matches!(
            recovered.group(DEFAULT_GROUP_ID).map(|g| g.scale),
            Some(ScaleState::Calibrated { .. })
        ),
        "and so must the calibrated scale — losing it is the failure mode this pins"
    );
}

/// **A sidecar from a newer pdfce is refused for writing, never overwritten.**
///
/// This is the other half of the version-gate hazard, and the dangerous half.
/// The gate used to demand exact equality and answer `None` on a mismatch,
/// which the session turns into a FRESH model — so an older build opening a
/// newer file would start empty and the next save would write that emptiness
/// over the operator's groups, calibrated scales and memberships. Nothing
/// would look wrong in between: the `/Line` annotations keep rendering
/// perfectly, so the loss is invisible until it is permanent.
///
/// The assertion that matters is the second one: after the refusal, nothing
/// has been staged, so there is no emptiness waiting to be written.
#[test]
fn a_sidecar_from_a_newer_build_refuses_writes_instead_of_discarding_it() {
    let doc = Document::from_bytes(minimal_pdf_with_catalog(
        "<< /Type /Catalog /Pages 2 0 R /PieceInfo << /pdfce << /LastModified (D:20260804000000Z) /Private << /Version 999 /Groups [] /Dimensions [] >> >> >> >>",
    ))
    .unwrap();
    let mut s = EditSession::new(doc);

    for (what, result) in [
        ("add", s.add_dimension(0, DEFAULT_GROUP_ID, linear()).err()),
        (
            "scale",
            s.set_group_scale(
                DEFAULT_GROUP_ID,
                ScaleState::Calibrated { scale: 0.01 },
                NumberFormat::decimal(Unit::Meter, 2),
            )
            .err(),
        ),
        (
            "delete",
            s.delete_dimension(pdfce_core::dimension::DimensionId(0))
                .err(),
        ),
    ] {
        assert!(
            matches!(
                result,
                Some(pdfce_core::edit::EditError::SidecarWrittenByNewerBuild { found: 999, .. })
            ),
            "the {what} path must refuse a newer sidecar by name, got {result:?}"
        );
    }
    assert!(
        !s.is_modified(),
        "every refusal happens BEFORE any mutation — nothing is staged to overwrite the sidecar"
    );
}

/// Undo of a move restores the dimension exactly.
#[test]
fn undoing_a_ce_dimension_move_restores_it() {
    let (_orig, mut s) = session();
    let (_annot, dim_id) = s.add_dimension(0, DEFAULT_GROUP_ID, linear()).unwrap();
    let before = save(&s);
    s.move_dimension(dim_id, 40.0, 40.0).unwrap();
    assert_ne!(save(&s), before, "the move must actually change the file");
    s.undo().expect("undo the move");
    assert_eq!(
        save(&s),
        before,
        "undoing a move must restore the byte-identical prior save"
    );
}

/// A stale dimension id is refused by name rather than silently doing nothing.
#[test]
fn moving_an_unknown_ce_dimension_is_refused_by_name() {
    let (_orig, mut s) = session();
    let err = s
        .move_dimension(pdfce_core::dimension::DimensionId(999), 1.0, 1.0)
        .expect_err("an unknown id must refuse");
    assert!(
        matches!(
            err,
            pdfce_core::edit::EditError::DimensionNotFound { id: 999 }
        ),
        "got {err:?}"
    );
}

#[test]
fn toggling_a_layer_moves_the_group_ocg_into_d_off() {
    let (_orig, mut s) = session();
    let g = s.add_dimension_group("Hidden", Unit::Millimeter).unwrap();
    // Author a dimension so the group gets an OCG.
    s.add_dimension(0, g, linear()).unwrap();
    // Hide the layer.
    let visible = s.toggle_dimension_layer(g, false).unwrap();
    assert!(!visible);

    let reloaded = Document::from_bytes(save(&s)).unwrap();
    // The group's OCG must now be in /OCProperties /D /OFF.
    let model = s.dimension_model();
    let ocg = model
        .groups()
        .iter()
        .find(|gr| gr.name == "Hidden")
        .and_then(|gr| gr.ocg)
        .expect("group has an OCG");
    let catalog = reloaded.catalog().unwrap();
    let d = catalog
        .get(b"OCProperties")
        .unwrap()
        .as_dict()
        .unwrap()
        .get(b"D")
        .unwrap()
        .as_dict()
        .unwrap();
    let off = d.get(b"OFF").unwrap().as_array().unwrap();
    assert!(
        off.contains(&Object::Reference(ocg)),
        "a hidden group's OCG must be in /D /OFF"
    );
}

#[test]
fn undo_of_a_dimension_removes_everything() {
    let (original, mut s) = session();
    s.add_dimension(0, DEFAULT_GROUP_ID, linear()).unwrap();
    assert!(s.is_modified());
    s.undo().expect("undo the dimension");
    assert!(!s.is_modified(), "undo restores the pristine session");
    // A save now is byte-identical to the original (nothing was written).
    let out = save(&s);
    assert_eq!(out, original, "after undo, the file is byte-identical");
}
