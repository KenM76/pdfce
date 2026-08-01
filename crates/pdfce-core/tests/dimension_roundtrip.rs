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
use pdfce_core::object::{ObjId, Object};
use pdfce_core::vector::{AxisConstraint, Point};
use pdfce_core::writer::SaveOptions;

/// Build a minimal one-page PDF: catalog(1) → pages(2) → page(3).
fn minimal_pdf() -> Vec<u8> {
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R >>",
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
