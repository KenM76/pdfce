//! Rich-text form fields (`/Ff` bit 26): the refusal that prevents a WRONG
//! VALUE, and the explicit downgrade that is the way through.
//!
//! ## Why the refusal is a correctness guard, not a fidelity one
//!
//! ISO 32000-1 §12.7.3.4 says the rich text string *"in addition to the `RV`
//! or `RC` entry, shall be used to generate the appearance"*, and §12.7.3.3
//! requires regeneration on every value change for these fields. Appearance
//! generation is therefore bound to `/RV`, **not** `/V`. Writing a fresh `/V`
//! while leaving `/RV` in place yields a field whose appearance a conforming
//! reader rebuilds from the OLD text — the document displays a value nobody
//! typed.
//!
//! The fixture makes that visible on purpose: its `/V` and `/RV` say
//! DIFFERENT things, so a test cannot pass by the two happening to agree.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use pdfce_core::document::Document;
use pdfce_core::edit::{EditError, EditSession};
use pdfce_core::object::Object;
use std::path::{Path, PathBuf};

/// Fixture paths are resolved from `CARGO_MANIFEST_DIR`, not the CWD — an
/// integration test runs with the CRATE root as its working directory, not the
/// workspace root. Same helper shape as `add_text.rs`.
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/synthetic/forms")
        .join(name)
}

fn session() -> EditSession {
    EditSession::new(
        Document::load(&fixture("radio-choice-form.pdf")).expect("load the rich-text fixture"),
    )
}

fn notes_field(session: &EditSession) -> pdfce_core::forms::Field {
    pdfce_core::forms::parse_acroform(&session.graph())
        .expect("the fixture has an AcroForm")
        .fields
        .into_iter()
        .find(|f| f.fully_qualified_name == "Notes")
        .expect("the fixture has a Notes field")
}

/// The fixture is what the tests below claim it is. Asserted rather than
/// assumed: every test here is meaningless if bit 26 is not actually set.
#[test]
fn the_fixture_field_really_is_rich_text_with_a_disagreeing_plain_twin() {
    let s = session();
    let notes = notes_field(&s);
    assert!(notes.is_rich_text(), "Notes must be a rich-text field");
    let Some(Object::Dict(d)) = s.value(notes.id) else {
        panic!("Notes is not a dict");
    };
    assert!(d.get(b"RV").is_some(), "the fixture must carry /RV");
    assert!(d.get(b"DS").is_some(), "the fixture must carry /DS");
    // Decoded, not `{:?}`-formatted: `Object::String`'s Debug prints a byte
    // list, so a `contains("<b>")` against it fails while the markup is
    // present — which is exactly what happened on the first run of this test.
    let Some(Object::String(rv_bytes)) = d.get(b"RV") else {
        panic!("/RV is not a string");
    };
    let rv = String::from_utf8_lossy(rv_bytes);
    assert!(rv.contains("<b>"), "the /RV must carry real markup: {rv}");
    // The plain twin and the rich value disagree ON PURPOSE — see the module
    // doc. If they ever agree, the correctness bug these tests guard becomes
    // invisible and the suite silently stops proving anything.
    assert!(
        rv.contains("ORIGINAL") && notes.value.display_text().contains("ORIGINAL"),
        "fixture drift: /V and /RV must both mention ORIGINAL"
    );
}

/// A plain fill is REFUSED BY NAME, before anything is written.
#[test]
fn a_plain_fill_of_a_rich_text_field_is_refused_by_name() {
    let mut s = session();
    let err = s
        .fill_text_field("Notes", "plain replacement")
        .expect_err("a plain fill must not silently corrupt a rich-text field");
    assert!(
        matches!(err, EditError::FieldIsRichText { ref name } if name == "Notes"),
        "expected FieldIsRichText, got {err:?}"
    );
    assert!(
        !s.is_modified(),
        "a refusal must leave the session untouched — rule 4: refuse BEFORE mutating"
    );
}

/// The explicit downgrade accepts it, and leaves a CONSISTENT plain field.
///
/// All four postconditions matter together. Any one of them alone leaves the
/// document in a state that is either malformed or still wrong on screen:
/// clearing the flag but keeping `/RV` leaves the old text recoverable and the
/// dictionary self-contradictory; removing `/RV` but keeping the flag makes a
/// rich-text field with no rich value.
#[test]
fn the_explicit_downgrade_converts_the_field_and_removes_every_rich_entry() {
    let mut s = session();
    s.fill_text_field_downgrading_rich_text("Notes", "plain replacement")
        .expect("the explicit downgrade accepts a rich-text field");

    let notes = notes_field(&s);
    assert!(
        !notes.is_rich_text(),
        "1/4: the field must no longer be rich text"
    );
    assert_eq!(
        notes.value.display_text(),
        "plain replacement",
        "2/4: /V must hold exactly what was typed"
    );
    let Some(Object::Dict(d)) = s.value(notes.id) else {
        panic!("Notes is not a dict after the downgrade");
    };
    assert!(
        d.get(b"RV").is_none(),
        "3/4: /RV must be REMOVED — a stale rich value would still drive the appearance, \
         and would leave the old text recoverable from the dictionary"
    );
    assert!(
        d.get(b"DS").is_none(),
        "4/4: /DS styles the rich value and means nothing without one"
    );
}

/// One undo puts the whole downgrade back — flag, `/RV`, `/DS` and value —
/// because it is ONE command, not four.
#[test]
fn undoing_the_downgrade_restores_the_rich_text_field_whole() {
    let mut s = session();
    s.fill_text_field_downgrading_rich_text("Notes", "plain replacement")
        .unwrap();
    s.undo().expect("undo the downgrade");

    let notes = notes_field(&s);
    assert!(notes.is_rich_text(), "the RichText flag must come back");
    let Some(Object::Dict(d)) = s.value(notes.id) else {
        panic!("Notes is not a dict after undo");
    };
    assert!(d.get(b"RV").is_some(), "/RV must come back");
    assert!(d.get(b"DS").is_some(), "/DS must come back");
    assert!(!s.is_modified(), "one undo restores the pristine session");
}

/// The downgrade survives save-and-reopen — it is a real document change, not
/// a session-only view of one.
#[test]
fn the_downgrade_round_trips_through_a_save() {
    let mut s = session();
    s.fill_text_field_downgrading_rich_text("Notes", "plain replacement")
        .unwrap();
    let bytes = s
        .to_incremental_bytes(&pdfce_core::writer::SaveOptions::identity())
        .unwrap()
        .0;

    let reopened = EditSession::new(Document::from_bytes(bytes).unwrap());
    let notes = notes_field(&reopened);
    assert!(!notes.is_rich_text());
    assert_eq!(notes.value.display_text(), "plain replacement");
    let Some(Object::Dict(d)) = reopened.value(notes.id) else {
        panic!("not a dict");
    };
    assert!(d.get(b"RV").is_none(), "/RV must stay gone after a save");
}

/// The downgrade does NOT touch an ordinary text field's flags — it is
/// conditional on the field actually being rich text, not applied blindly.
#[test]
fn the_downgrade_entry_point_is_harmless_on_a_plain_field() {
    let mut s = EditSession::new(
        Document::load(&fixture("demo-form.pdf")).expect("load the plain fixture"),
    );
    s.fill_text_field_downgrading_rich_text("FullName", "Ken")
        .expect("a plain field fills normally through this entry point too");
    let form = pdfce_core::forms::parse_acroform(&s.graph()).unwrap();
    let f = form
        .fields
        .iter()
        .find(|f| f.fully_qualified_name == "FullName")
        .unwrap();
    assert_eq!(f.value.display_text(), "Ken");
    assert!(!f.is_rich_text());
}
