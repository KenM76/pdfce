//! Integration tests for [`EditSession::adopt_widget`] — registering an
//! **existing** widget annotation as a form field (ISO 32000-1 §12.7.3).
//!
//! ## The fixture is a real AcroForm, and that is load-bearing
//!
//! `fixtures/external/pdfbox/.../compression/acroform.pdf` carries 12 fields
//! over 13 widgets, and — the reason it is the right subject — **both**
//! representations §12.7.3.1 permits:
//!
//! - eleven **merged field-widgets**: one dictionary that is both the field
//!   and its widget, carrying `/FT`, `/T`, `/V`, `/DA`;
//! - two **bare kids** of the `GroupOption` radio field, carrying no field
//!   keys at all — their only link to their identity is `/Parent`.
//!
//! A hand-built fixture would have exercised whichever shape the author
//! happened to write down, and the whole point of this verb is that the two
//! shapes have **different outcomes**: one adopts losslessly, the other
//! cannot be adopted at all. `examples/orphan_probe.rs` is where those
//! numbers were measured; this file pins the behaviour that follows from
//! them.
//!
//! The corpus is optional, so each test skips rather than fails when it is
//! absent — with an explicit `SKIP:` line, because a test that quietly
//! passes when it ran nothing is worse than one that fails.

use pdfce_core::document::Document;
use pdfce_core::edit::{EditError, EditSession};
use pdfce_core::graph::ObjectGraph;
use pdfce_core::object::{ObjId, Object};
use pdfce_core::pageops::InsertPosition;
use pdfce_core::writer::SaveOptions;

const ACROFORM: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/external/pdfbox/pdfbox/src/test/resources/input/compression/acroform.pdf"
);
const BLANK: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/synthetic/outline/no-outline.pdf"
);

/// A session over a blank target with the AcroForm fixture's page 0 inserted
/// — i.e. **exactly the state `insert_pages` leaves a shell in**, which is
/// the situation this verb exists for.
///
/// Returns `None` when the external corpus is absent.
fn orphaned_session() -> Option<(EditSession, usize, usize)> {
    let src_bytes = std::fs::read(ACROFORM).ok()?;
    let src = Document::from_bytes(src_bytes).expect("source must parse");
    let target =
        Document::from_bytes(std::fs::read(BLANK).expect("blank target")).expect("target parses");
    let mut session = EditSession::new(target);
    let outcome = session
        .insert_pages(&src.view(), &[0], InsertPosition::End)
        .expect("insert must succeed");
    Some((
        session,
        outcome.orphaned_widgets,
        outcome.orphaned_widgets_unrecoverable,
    ))
}

/// Every widget on the last page, in `/Annots` order, split by whether it
/// carries its own `/T`.
fn widgets(session: &EditSession) -> (Vec<ObjId>, Vec<ObjId>) {
    let view = session.view();
    let slots = session.page_slots().expect("pages");
    let last = slots.last().expect("a page");
    let Object::Dict(page) = view.resolved(last.id) else {
        panic!("page is not a dict")
    };
    let Some(Object::Array(annots)) = page.get(b"Annots").map(|a| view.resolve(a).clone()) else {
        panic!("no /Annots")
    };
    let (mut named, mut bare) = (Vec::new(), Vec::new());
    for entry in &annots {
        let Object::Reference(id) = entry else {
            continue;
        };
        let Object::Dict(d) = view.resolved(*id) else {
            continue;
        };
        if !matches!(d.get(b"Subtype"), Some(Object::Name(n)) if n.as_bytes() == b"Widget") {
            continue;
        }
        if d.contains_key(b"T") {
            named.push(*id);
        } else {
            bare.push(*id);
        }
    }
    (named, bare)
}

/// A widget dictionary, cloned so it outlives the borrow of the session.
///
/// Cloned rather than borrowed because every use here is a before/after
/// comparison across a `&mut self` call, and a borrow of `session.view()`
/// cannot survive one.
fn widget_dict(session: &EditSession, id: ObjId) -> pdfce_core::object::Dict {
    match session.view().resolved(id) {
        Object::Dict(d) => d.clone(),
        other => panic!("object {id:?} is not a dict: {other:?}"),
    }
}
/// Field names visible in the **saved** bytes, which is what any other tool
/// sees. Nothing here inspects the session overlay.
fn saved_field_names(session: &EditSession) -> Vec<String> {
    let (bytes, _) = session
        .to_incremental_bytes(&SaveOptions::identity())
        .expect("save must succeed");
    let doc = Document::from_bytes(bytes).expect("pdfce's own output must reparse");
    match pdfce_core::forms::parse_acroform(&doc) {
        Some(form) => form
            .fields
            .iter()
            .map(|f| f.fully_qualified_name.clone())
            .collect(),
        None => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// The measured premise
// ---------------------------------------------------------------------------

/// Would catch: the fixture changing shape under these tests, or
/// `orphaned_widgets_unrecoverable` drifting from what it counts.
///
/// Every other test here rests on "11 adoptable, 2 not", so that claim gets
/// asserted once, explicitly, rather than being an assumption spread across
/// the file. If this fails, the others are testing something else.
#[test]
fn the_fixture_carries_both_widget_shapes_and_the_counts_agree() {
    let Some((session, orphaned, unrecoverable)) = orphaned_session() else {
        eprintln!("SKIP: the external pdfbox corpus is not present");
        return;
    };
    assert_eq!(orphaned, 13, "13 widgets came across");
    assert_eq!(
        unrecoverable, 2,
        "the two GroupOption radio kids lost their identity"
    );
    let (named, bare) = widgets(&session);
    assert_eq!(named.len(), 11, "merged field-widgets");
    assert_eq!(bare.len(), 2, "bare kids");
    assert_eq!(
        unrecoverable,
        bare.len(),
        "the counter must count exactly the widgets that cannot be adopted"
    );
    assert!(
        saved_field_names(&session).is_empty(),
        "insert_pages must not have merged /AcroForm — that is the premise"
    );
}

// ---------------------------------------------------------------------------
// Adoption
// ---------------------------------------------------------------------------

/// Would catch: adoption registering the field in the session but not in the
/// saved bytes, or registering it under the wrong name.
///
/// Asserted through `parse_acroform` over a **reparsed save**, because that
/// is the only view another tool has. A field that exists in the overlay and
/// not in the file is the failure this verb is most likely to have.
#[test]
fn a_merged_field_widget_adopts_losslessly() {
    let Some((mut session, ..)) = orphaned_session() else {
        eprintln!("SKIP: the external pdfbox corpus is not present");
        return;
    };
    let (named, _) = widgets(&session);
    let widget = named[0];

    let out = session
        .adopt_widget(widget, None)
        .expect("a merged field-widget must adopt");
    assert_eq!(out.field_id, widget, "a merged field-widget IS its field");
    assert_eq!(out.name, "TextField");
    assert_eq!(out.field_type.as_deref(), Some("Tx"));
    assert!(!out.renamed, "no name was supplied, so nothing was renamed");
    assert!(
        out.acroform_created,
        "the blank target had no /AcroForm before this"
    );

    assert_eq!(saved_field_names(&session), vec!["TextField".to_owned()]);
}

/// Would catch: adoption clobbering the widget's existing appearance,
/// geometry or value — the thing the verb exists to avoid.
///
/// `add_text_field` authors a new widget; this must author nothing. So the
/// widget's dictionary is compared key-for-key before and after, and the ONLY
/// permitted difference is nothing at all when no rename was asked for.
#[test]
fn adoption_writes_no_geometry_appearance_or_value() {
    let Some((mut session, ..)) = orphaned_session() else {
        eprintln!("SKIP: the external pdfbox corpus is not present");
        return;
    };
    let (named, _) = widgets(&session);
    let widget = named[0];
    let before = widget_dict(&session, widget);

    session.adopt_widget(widget, None).expect("must adopt");

    let after = widget_dict(&session, widget);
    assert_eq!(
        before, after,
        "adopting must not touch the widget's own dictionary at all"
    );
}

/// Would catch: a rename being reported but not written, or written but not
/// reported. Both produce a document whose field name disagrees with what the
/// shell told the operator.
#[test]
fn a_rename_is_both_written_and_reported() {
    let Some((mut session, ..)) = orphaned_session() else {
        eprintln!("SKIP: the external pdfbox corpus is not present");
        return;
    };
    let (named, _) = widgets(&session);
    let out = session
        .adopt_widget(named[0], Some("Customer Name"))
        .expect("must adopt");
    assert!(out.renamed);
    assert_eq!(out.name, "Customer Name");
    assert_eq!(
        saved_field_names(&session),
        vec!["Customer Name".to_owned()],
        "the name in the FILE must be the name that was reported"
    );
}

/// Would catch: a second adoption dropping the first, which is what happens
/// if `/Fields` is overwritten rather than appended to — and it is invisible
/// until the second call.
#[test]
fn adopting_several_widgets_accumulates_rather_than_replacing() {
    let Some((mut session, ..)) = orphaned_session() else {
        eprintln!("SKIP: the external pdfbox corpus is not present");
        return;
    };
    let (named, _) = widgets(&session);
    for w in named.iter().take(4) {
        session.adopt_widget(*w, None).expect("must adopt");
    }
    let names = saved_field_names(&session);
    assert_eq!(names.len(), 4, "all four must survive, got {names:?}");
    assert!(names.contains(&"TextField".to_owned()));
    assert!(names.contains(&"CheckBox1".to_owned()));
}

// ---------------------------------------------------------------------------
// Refusals
// ---------------------------------------------------------------------------

/// Would catch: a bare kid being adopted under an invented name.
///
/// This is the refusal the whole design turns on. A kid carries no `/T`, so
/// anything pdfce wrote there would be a name the source never used — and for
/// a radio group, adopting the two kids separately produces two independent
/// check boxes where there was one mutually-exclusive field. The form looks
/// right and behaves wrong, which is the worst available outcome.
#[test]
fn a_bare_kid_widget_is_refused_rather_than_named() {
    let Some((mut session, ..)) = orphaned_session() else {
        eprintln!("SKIP: the external pdfbox corpus is not present");
        return;
    };
    let (_, bare) = widgets(&session);
    assert_eq!(bare.len(), 2, "fixture premise");
    for w in &bare {
        match session.adopt_widget(*w, None) {
            Err(EditError::WidgetHasNoFieldIdentity { id }) => assert_eq!(id, w.num),
            other => panic!("a bare kid must be refused, got {other:?}"),
        }
    }
    assert!(
        saved_field_names(&session).is_empty(),
        "a refusal must register nothing"
    );

    // With an explicit name the operator HAS chosen, it is allowed — the
    // refusal is about pdfce guessing, not about the widget being unusable.
    let out = session
        .adopt_widget(bare[0], Some("RadioA"))
        .expect("an explicitly named kid must adopt");
    assert!(out.renamed);
    assert_eq!(saved_field_names(&session), vec!["RadioA".to_owned()]);
}

/// Would catch: a name collision being allowed through.
///
/// §12.7.3.1 makes the fully qualified name the field's *identity*, so two
/// top-level fields called `TextField` are one field with two widgets —
/// filling either fills both. No viewer reports it; the operator finds it by
/// typing. That makes silent acceptance the worst outcome and a loud refusal
/// the right one.
#[test]
fn a_colliding_name_is_refused() {
    let Some((mut session, ..)) = orphaned_session() else {
        eprintln!("SKIP: the external pdfbox corpus is not present");
        return;
    };
    let (named, _) = widgets(&session);
    session.adopt_widget(named[0], None).expect("first adopts");

    match session.adopt_widget(named[1], Some("TextField")) {
        Err(EditError::FieldNameTaken { name }) => assert_eq!(name, "TextField"),
        other => panic!("a colliding name must be refused, got {other:?}"),
    }
    assert_eq!(
        saved_field_names(&session).len(),
        1,
        "the refused adoption must leave nothing behind"
    );
}

/// Would catch: double-adoption listing one widget twice in `/Fields`, which
/// produces a form that reports more fields than it has controls.
#[test]
fn adopting_the_same_widget_twice_is_refused() {
    let Some((mut session, ..)) = orphaned_session() else {
        eprintln!("SKIP: the external pdfbox corpus is not present");
        return;
    };
    let (named, _) = widgets(&session);
    session.adopt_widget(named[0], None).expect("first adopts");
    match session.adopt_widget(named[0], None) {
        Err(EditError::WidgetAlreadyOwned { id }) => assert_eq!(id, named[0].num),
        other => panic!("a second adoption must be refused, got {other:?}"),
    }
    assert_eq!(saved_field_names(&session).len(), 1);
}

/// Would catch: a non-widget object being accepted. A page and a plain
/// annotation are both dictionaries on the same page, so nothing about the
/// call site distinguishes them from a widget.
#[test]
fn a_non_widget_is_refused() {
    let Some((mut session, ..)) = orphaned_session() else {
        eprintln!("SKIP: the external pdfbox corpus is not present");
        return;
    };
    let page = session.page_slots().expect("pages")[0].id;
    match session.adopt_widget(page, Some("NotAField")) {
        Err(EditError::NotAWidget { id }) => assert_eq!(id, page.num),
        other => panic!("a page must be refused, got {other:?}"),
    }
    match session.adopt_widget(ObjId::new(99_999, 0), Some("Nothing")) {
        Err(EditError::NotAWidget { .. }) => {}
        other => panic!("a missing object must be refused, got {other:?}"),
    }
    assert!(saved_field_names(&session).is_empty());
}

// ---------------------------------------------------------------------------
// Undo
// ---------------------------------------------------------------------------

/// Would catch: undo removing the registration but leaving the rename, so the
/// widget keeps a name the operator undid.
#[test]
fn undo_removes_the_registration_and_the_rename_together() {
    let Some((mut session, ..)) = orphaned_session() else {
        eprintln!("SKIP: the external pdfbox corpus is not present");
        return;
    };
    let (named, _) = widgets(&session);
    let widget = named[0];
    let before = widget_dict(&session, widget);

    session
        .adopt_widget(widget, Some("Renamed"))
        .expect("must adopt");
    assert_eq!(saved_field_names(&session), vec!["Renamed".to_owned()]);

    assert!(session.undo().is_some(), "one undo entry must exist");
    assert!(
        saved_field_names(&session).is_empty(),
        "the registration must be gone"
    );
    let after = widget_dict(&session, widget);
    assert_eq!(
        before, after,
        "and so must the rename — the widget must be exactly as it arrived"
    );
}

// ---------------------------------------------------------------------------
// `/FT` is inheritable and `/T` is not — the distinction the count turns on
// ---------------------------------------------------------------------------

/// Byte-author a minimal PDF, so a shape no corpus file happens to contain
/// can still be tested. Same construction as `tests/page_ops.rs`.
fn build(objects: &[(u32, &str)]) -> Vec<u8> {
    let mut buf = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets: Vec<(u32, usize)> = Vec::new();
    for (num, body) in objects {
        offsets.push((*num, buf.len()));
        buf.extend_from_slice(format!("{num} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let xref_at = buf.len();
    let max_num = objects.iter().map(|(n, _)| *n).max().unwrap_or(0);
    buf.extend_from_slice(format!("xref\n0 {}\n", max_num + 1).as_bytes());
    buf.extend_from_slice(b"0000000000 65535 f \n");
    for num in 1..=max_num {
        match offsets.iter().find(|(n, _)| *n == num) {
            Some((_, off)) => buf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes()),
            None => buf.extend_from_slice(b"0000000000 65535 f \n"),
        }
    }
    buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R /ID [<0102> <0304>] >>\nstartxref\n{xref_at}\n%%EOF\n",
            max_num + 1
        )
        .as_bytes(),
    );
    buf
}

/// Would catch: `orphaned_widgets_unrecoverable` testing `/FT` instead of
/// `/T`.
///
/// ## Why this needs a hand-built fixture, and why the sabotage battery
/// demanded one
///
/// `orphan_probe`'s real AcroForm cannot distinguish the two predicates: its
/// bare radio kids carry **neither** `/FT` nor `/T`, so counting by either
/// gives 2. Swapping the key in the implementation left the entire suite
/// green — the code's own comment argued for `/T` on the grounds that
/// §12.7.3.1 makes `/FT` **inheritable**, and that argument was reasoned but
/// completely unmeasured.
///
/// This document supplies the case that separates them: three widgets on one
/// page, of which
///
/// - `Named` carries `/T` **and** `/FT` — adoptable, not counted;
/// - `TypeOnly` carries `/FT` and **no** `/T` — a kid that inherits nothing
///   useful; `/FT` tells a viewer how to *draw* it but there is still no name
///   to fill, export or refer to it by, so it **must** be counted;
/// - `Naked` carries neither — counted, and the case the real corpus covers.
///
/// Expected count is therefore **2**, and counting by `/FT` yields **1**.
///
/// The wider point, worth keeping: a field with no name is unusable no matter
/// how much type information survives, because §12.7.3.1 makes the fully
/// qualified name the field's identity. `/FT` surviving is not partial
/// recovery — it is decoration on something that cannot be addressed.
#[test]
fn a_widget_with_ft_but_no_t_is_still_unrecoverable() {
    let source = build(&[
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (
            2,
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 100] /Resources << >> >>",
        ),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /Annots [4 0 R 5 0 R 6 0 R] >>",
        ),
        (
            4,
            "<< /Type /Annot /Subtype /Widget /Rect [0 0 10 10] /FT /Tx /T (Named) >>",
        ),
        (
            5,
            "<< /Type /Annot /Subtype /Widget /Rect [0 0 10 10] /FT /Btn >>",
        ),
        (6, "<< /Type /Annot /Subtype /Widget /Rect [0 0 10 10] >>"),
    ]);
    let src = Document::from_bytes(source).expect("hand-built source must parse");
    let target =
        Document::from_bytes(std::fs::read(BLANK).expect("blank target")).expect("target parses");
    let mut session = EditSession::new(target);

    let outcome = session
        .insert_pages(&src.view(), &[0], InsertPosition::End)
        .expect("insert must succeed");
    assert_eq!(outcome.orphaned_widgets, 3);
    assert_eq!(
        outcome.orphaned_widgets_unrecoverable, 2,
        "a widget with /FT but no /T has no NAME, so it is unrecoverable — \
counting by /FT would say 1"
    );

    // And the verb agrees with the counter, which is the property that makes
    // the number actionable: exactly the widgets it counts are the ones
    // `adopt_widget` refuses.
    let (named, bare) = widgets(&session);
    assert_eq!(named.len(), 1);
    assert_eq!(bare.len(), 2);
    for w in &bare {
        assert!(
            matches!(
                session.adopt_widget(*w, None),
                Err(EditError::WidgetHasNoFieldIdentity { .. })
            ),
            "widget {w:?} is counted as unrecoverable, so it must also be refused"
        );
    }
    assert!(session.adopt_widget(named[0], None).is_ok());
}
