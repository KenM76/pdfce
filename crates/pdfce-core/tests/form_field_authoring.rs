//! Authoring NEW form fields (§12.7.2 registration + §12.5.6.19 widget).
//!
//! ## What these tests are really checking
//!
//! A created field is **three coordinated writes** — the merged field/widget
//! dictionary, the page's `/Annots` entry, and the `/AcroForm` `/Fields`
//! registration. Any two without the third produce a document that is broken
//! in a way nothing visibly reports: registered-but-not-annotated is
//! invisible, annotated-but-not-registered is not a form field at all.
//!
//! So the load-bearing assertion throughout is not "the bytes were written"
//! but **"`parse_acroform` reads it back as the field we meant"** — the same
//! parser the fill path, the CLI and the GUI all use. A field that only the
//! writer can see is not a field.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use pdfce_core::document::Document;
use pdfce_core::edit::{EditError, EditSession, NewTextField};
use pdfce_core::forms::{self, FieldFlags, FieldType};
use pdfce_core::graph::ObjectGraph;
use pdfce_core::object::Object;
use pdfce_core::page_tree::Rect;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/synthetic")
        .join(name)
}

fn session(rel: &str) -> EditSession {
    EditSession::new(Document::load(&fixture(rel)).expect("load fixture"))
}

fn rect() -> Rect {
    Rect {
        llx: 20.0,
        lly: 100.0,
        urx: 220.0,
        ury: 124.0,
    }
}

fn field_named(s: &EditSession, name: &str) -> Option<forms::Field> {
    forms::parse_acroform(&s.graph())?
        .fields
        .into_iter()
        .find(|f| f.fully_qualified_name == name)
}

/// The headline: a field created on a page that had NO form at all is read
/// back by the ordinary parser as a fillable text field.
#[test]
fn a_field_created_on_a_formless_page_parses_back_as_a_text_field() {
    let mut s = session("dimension/plain-base.pdf");
    assert!(
        forms::parse_acroform(&s.graph()).is_none(),
        "precondition: this fixture has no AcroForm, so the test proves \
         creation from nothing rather than appending to something"
    );

    s.add_text_field(&NewTextField::new(0, "Customer", rect()))
        .expect("author a text field");

    let f = field_named(&s, "Customer").expect("the field parses back");
    assert_eq!(f.field_type, Some(FieldType::Text));
    assert!(
        f.is_fillable(),
        "a field created to be typed in must be fillable"
    );
    assert_eq!(f.widgets.len(), 1, "one widget");
    assert!(
        f.widgets[0].merged,
        "single-widget fields use the §12.5.6.19 MERGED shape"
    );
}

/// All three writes land — and this is asserted at the OBJECT level, because
/// `parse_acroform` succeeding could in principle mask a missing `/Annots`
/// entry (the field would be registered but never drawn).
#[test]
fn the_field_is_registered_annotated_and_given_an_appearance() {
    let mut s = session("dimension/plain-base.pdf");
    let id = s
        .add_text_field(&NewTextField::new(0, "Customer", rect()))
        .unwrap();

    let graph = s.graph();
    let d = graph
        .resolved(id)
        .as_dict()
        .expect("field is a dict")
        .clone();
    assert!(
        d.get(b"AP").is_some(),
        "1/3: an /AP, or it is invisible (R43/R51)"
    );

    // /AcroForm /Fields registration.
    let catalog = graph
        .resolved(graph.catalog_id().unwrap())
        .as_dict()
        .unwrap()
        .clone();
    let af = match catalog.get(b"AcroForm") {
        Some(Object::Dict(d)) => d.clone(),
        Some(Object::Reference(r)) => graph.resolved(*r).as_dict().unwrap().clone(),
        other => panic!("no /AcroForm: {other:?}"),
    };
    let Some(Object::Array(fields)) = af.get(b"Fields") else {
        panic!("/AcroForm has no /Fields array");
    };
    assert!(
        fields.contains(&Object::Reference(id)),
        "2/3: registered in /AcroForm /Fields"
    );
    // §12.7.3.3: the /DA names /Helv, so /DR /Font /Helv must resolve it or
    // another viewer regenerating from /DA cannot.
    let Some(Object::Dict(dr)) = af.get(b"DR") else {
        panic!("/AcroForm has no /DR");
    };
    let Some(Object::Dict(fonts)) = dr.get(b"Font") else {
        panic!("/DR has no /Font");
    };
    assert!(
        fonts.get(b"Helv").is_some(),
        "the /DA's font resolves in /DR"
    );

    // Page /Annots.
    let page_id = s.page_slots().unwrap()[0].id;
    let page = graph.resolved(page_id).as_dict().unwrap().clone();
    let annots = match page.get(b"Annots") {
        Some(Object::Array(a)) => a.clone(),
        Some(Object::Reference(r)) => match graph.resolved(*r) {
            Object::Array(a) => a.clone(),
            other => panic!("/Annots is not an array: {other:?}"),
        },
        other => panic!("page has no /Annots: {other:?}"),
    };
    assert!(
        annots.contains(&Object::Reference(id)),
        "3/3: present in the page's /Annots, or it is registered but never drawn"
    );
}

/// Additive (R46): every original byte survives, and the result round-trips.
#[test]
fn authoring_a_field_is_additive_and_the_result_reopens() {
    let original = std::fs::read(fixture("dimension/plain-base.pdf")).unwrap();
    let mut s = EditSession::new(Document::from_bytes(original.clone()).unwrap());
    s.add_text_field(&NewTextField::new(0, "Customer", rect()))
        .unwrap();
    let out = s
        .to_incremental_bytes(&pdfce_core::writer::SaveOptions::identity())
        .unwrap()
        .0;

    assert!(
        out.starts_with(&original),
        "an additive author must not modify any original byte"
    );

    let reopened = EditSession::new(Document::from_bytes(out).unwrap());
    let f = field_named(&reopened, "Customer").expect("survives save and reopen");
    assert_eq!(f.field_type, Some(FieldType::Text));
}

/// The value and properties asked for are the ones stored.
#[test]
fn the_requested_value_and_properties_are_what_gets_written() {
    let mut s = session("dimension/plain-base.pdf");
    s.add_text_field(
        &NewTextField::new(0, "Notes", rect())
            .with_value("hello")
            .with_max_len(40)
            .with_tooltip("Your notes")
            .with_flags(true, false, true),
    )
    .unwrap();

    let f = field_named(&s, "Notes").unwrap();
    assert_eq!(f.value.display_text(), "hello");
    assert_eq!(f.max_len, Some(40));
    assert_eq!(
        f.alternate_name.as_deref().map(String::from_utf8_lossy),
        Some("Your notes".into()),
        "/TU is what a screen reader announces, so it must survive verbatim"
    );
    assert!(f.flags.has(FieldFlags::MULTILINE));
    assert!(f.flags.has(FieldFlags::REQUIRED));
    assert!(!f.flags.has(FieldFlags::READ_ONLY));
}

/// A created field is immediately FILLABLE through the existing fill verb —
/// the proof that authoring produced a real field and not merely a
/// field-shaped dictionary.
#[test]
fn a_created_field_can_immediately_be_filled_by_the_existing_verb() {
    let mut s = session("dimension/plain-base.pdf");
    s.add_text_field(&NewTextField::new(0, "Customer", rect()))
        .unwrap();
    s.fill_text_field("Customer", "Ken Mantle")
        .expect("the ordinary fill path accepts a field pdfce just authored");
    assert_eq!(
        field_named(&s, "Customer").unwrap().value.display_text(),
        "Ken Mantle"
    );
}

/// One undo removes the whole field — dictionary, annotation and
/// registration together, because it is ONE command.
#[test]
fn one_undo_removes_the_entire_field() {
    let original = std::fs::read(fixture("dimension/plain-base.pdf")).unwrap();
    let mut s = EditSession::new(Document::from_bytes(original.clone()).unwrap());
    s.add_text_field(&NewTextField::new(0, "Customer", rect()))
        .unwrap();
    assert!(s.is_modified());

    s.undo().expect("undo the authoring");
    assert!(!s.is_modified(), "undo restores the pristine session");
    assert!(
        forms::parse_acroform(&s.graph()).is_none(),
        "the /AcroForm created by the add is gone too, not left empty"
    );
    let out = s
        .to_incremental_bytes(&pdfce_core::writer::SaveOptions::identity())
        .unwrap()
        .0;
    assert_eq!(out, original, "after undo the file is byte-identical");
}

/// A second field on a document that ALREADY has a form appends rather than
/// replacing — the existing fields must survive.
#[test]
fn adding_to_an_existing_form_appends_and_keeps_the_existing_fields() {
    let mut s = session("forms/demo-form.pdf");
    let before = forms::parse_acroform(&s.graph()).unwrap().fields.len();
    s.add_text_field(&NewTextField::new(0, "Extra", rect()))
        .unwrap();

    let form = forms::parse_acroform(&s.graph()).unwrap();
    assert_eq!(form.fields.len(), before + 1);
    assert!(
        form.fields
            .iter()
            .any(|f| f.fully_qualified_name == "FullName"),
        "the document's own fields are untouched"
    );
}

// -- Refusals ---------------------------------------------------------------

/// A name already used by a field of a DIFFERENT type is refused by name.
/// One `/V` cannot be both a text string and a button on-state.
#[test]
fn a_name_used_by_a_different_field_type_is_refused_by_name() {
    let mut s = session("forms/demo-form.pdf");
    // `Subscribe` is the fixture's check box.
    let err = s
        .add_text_field(&NewTextField::new(0, "Subscribe", rect()))
        .expect_err("a text field may not take a button's name");
    assert!(
        matches!(err, EditError::FieldNameTypeConflict { ref name, existing }
            if name == "Subscribe" && existing == "button"),
        "expected FieldNameTypeConflict, got {err:?}"
    );
    assert!(!s.is_modified(), "a refusal writes nothing");
}

/// An empty name is refused: §12.7.3.2 builds the fully-qualified name from
/// `/T`, so a nameless field could never be filled or exported by name.
#[test]
fn an_empty_name_is_refused() {
    let mut s = session("dimension/plain-base.pdf");
    assert!(matches!(
        s.add_text_field(&NewTextField::new(0, "   ", rect())),
        Err(EditError::FieldNameEmpty)
    ));
    assert!(!s.is_modified());
}

/// A zero-area rectangle is refused — it would create a field that exists,
/// accepts a value, and can never be seen or clicked.
#[test]
fn a_degenerate_rectangle_is_refused() {
    let mut s = session("dimension/plain-base.pdf");
    let flat = Rect {
        llx: 20.0,
        lly: 100.0,
        urx: 220.0,
        ury: 100.0,
    };
    assert!(matches!(
        s.add_text_field(&NewTextField::new(0, "Flat", flat)),
        Err(EditError::FieldRectDegenerate { .. })
    ));
    assert!(!s.is_modified());
}

/// A page index past the end is refused rather than silently landing on the
/// last page.
#[test]
fn a_page_out_of_range_is_refused() {
    let mut s = session("dimension/plain-base.pdf");
    assert!(
        s.add_text_field(&NewTextField::new(99, "Customer", rect()))
            .is_err()
    );
    assert!(!s.is_modified());
}
