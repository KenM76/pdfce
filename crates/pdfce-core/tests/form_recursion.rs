//! # Descending into form XObjects — the objects a page-sized wrapper was hiding
//!
//! Integration test for `decompose_page`'s form recursion and
//! [`FormLeaf`](pdfce_core::vector::decompose::FormLeaf).
//!
//! # What was wrong, and it reached the operator
//!
//! `decompose_page` emitted a form XObject as **one opaque object** bounded by
//! its `/BBox` and never entered it. On a page whose visible body is wrapped in
//! a form — what SolidWorks emits per orthographic view, and what a great many
//! print files emit per panel — the form is an object in paint order **above**
//! everything drawn before it, and the hit test answers every click, anywhere,
//! with the form.
//!
//! His report, relayed from the GUI project: *"when I click on one of the
//! objects all I get is the page selected."* He was selecting a real object. It
//! was a form. Measured on one print-conformance page: **sixteen** ~20 × 20 pt
//! forms, one per blend-mode cell, each swallowing every click aimed at the
//! swatch inside it.
//!
//! ## ★ Why nothing caught it
//!
//! **No committed fixture had a form XObject at all.** Every vector fixture in
//! this repository draws straight onto the page, so the entire form branch of
//! the walk was exercised only by a stub resolver that returns a shape and no
//! content. `fixtures/synthetic/forms-xobject/` exists to end that.
//!
//! # The properties asserted
//!
//! | Property | Asserted by |
//! |---|---|
//! | a page-sized form yields the objects inside it | `a_page_sized_form_yields_the_objects_inside_it` |
//! | leaves are in page space, so one hit test serves both lists | (same) |
//! | nesting produces a containment path, and an intermediate form is not a leaf | `a_nested_form_reports_its_whole_containment_path` |
//! | a form invoking itself terminates, and says it did | `a_self_referential_form_terminates_and_is_counted` |
//! | a form invoked twice contributes twice, in two places | `a_form_invoked_twice_contributes_its_contents_twice` |
//! | the flat list does **not** move | `the_flat_object_list_is_unchanged_by_recursion` |
//! | a leaf refuses to claim it is editable | `a_leaf_names_its_own_stream_and_is_not_editable` |
//!
//! ## ★★ The last two are the load-bearing ones, and not for obvious reasons
//!
//! `the_flat_object_list_is_unchanged_by_recursion` guards a **safety**
//! property, not a compatibility one. Eleven call sites in `edit.rs` resolve a
//! paint-order index and apply content-stream surgery **to the page's stream**.
//! A leaf's token range indexes the *form's* stream — a different buffer. If a
//! leaf ever appears in `objects`, those verbs will apply a form-relative range
//! to the page and corrupt it silently, because the range is in bounds. Keeping
//! the lists separate is what makes those eleven sites correct by construction
//! rather than by a guard somebody must remember to add to each.
//!
//! `a_leaf_names_its_own_stream_and_is_not_editable` is the same fact from the
//! caller's side, in the vocabulary the shell already uses for text.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::{Path, PathBuf};

use pdfce_core::document::Document;
use pdfce_core::page_tree;
use pdfce_core::text_extract::ContentStreamRef;
use pdfce_core::vector::decompose::{ImageSource, PageObjects};
use pdfce_core::vector::{Matrix, VectorObject, decompose_page};

fn model(name: &str) -> PageObjects {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/synthetic/forms-xobject")
        .join(name);
    let doc = Document::load(Path::new(&path)).expect("fixture loads");
    let pages = page_tree::pages(&doc).expect("page tree");
    decompose_page(&doc.view(), &pages[0], Matrix::IDENTITY).expect("decomposes")
}

/// How many objects in the flat list are form XObjects.
fn form_count(m: &PageObjects) -> usize {
    m.objects
        .iter()
        .filter(|o| matches!(o, VectorObject::Image(i) if i.source == ImageSource::Form))
        .count()
}

/// ★ THE HEADLINE. One page-sized form, three squares inside it.
///
/// Before the recursion the page offered exactly one selectable thing — the
/// wrapper — and it covered the whole sheet.
#[test]
fn a_page_sized_form_yields_the_objects_inside_it() {
    let m = model("page-sized-form.pdf");

    assert_eq!(m.objects.len(), 1, "the flat list is the wrapper alone");
    assert_eq!(form_count(&m), 1);
    assert_eq!(
        m.leaves.len(),
        3,
        "★ the three squares inside the form must be reachable; \
         without the recursion this is 0 and every click hits the wrapper"
    );

    // Page space, not form space -- so a caller can hit-test the flat list and
    // the leaf list against ONE point without transforming anything. The
    // squares are at (10,10), (80,80) and (150,150) in the form, and the form
    // is placed at the origin, so those are their page coordinates too.
    let mut origins: Vec<(i64, i64)> = m
        .leaves
        .iter()
        .map(|l| {
            let b = l.object.page_bbox();
            (b.min.x.round() as i64, b.min.y.round() as i64)
        })
        .collect();
    origins.sort_unstable();
    assert_eq!(origins, vec![(10, 10), (80, 80), (150, 150)]);

    for leaf in &m.leaves {
        assert_eq!(leaf.containment.len(), 1, "one enclosing form");
    }
    assert_eq!(m.diagnostics.form_cycles, 0);
    assert_eq!(m.diagnostics.form_depth_overflows, 0);
}

/// Nesting: form A holds form B holds one square.
///
/// Two things are asserted and the second is the easy one to forget: the
/// containment path has BOTH forms in it, outermost first, and the
/// **intermediate form is not itself a leaf**. Emitting it as one would put a
/// second large hit target into the very list built to stop the first one
/// winning every click.
#[test]
fn a_nested_form_reports_its_whole_containment_path() {
    let m = model("nested-forms.pdf");

    assert_eq!(m.objects.len(), 1);
    assert_eq!(m.leaves.len(), 1, "only the square is a leaf");
    let leaf = &m.leaves[0];
    assert_eq!(
        leaf.containment.len(),
        2,
        "outer form then inner form, outermost first"
    );
    assert_ne!(
        leaf.containment[0], leaf.containment[1],
        "two distinct forms"
    );

    // The square is at (20,20) inside the inner form, which the outer form
    // places at +(50,50). Geometry composing through two levels is the thing
    // a single-level test could not have caught.
    let b = leaf.object.page_bbox();
    assert_eq!((b.min.x.round() as i64, b.min.y.round() as i64), (70, 70));
}

/// ★★ A form that invokes ITSELF terminates, and the walk says it did.
///
/// ISO 32000-1 §8.10.1 does not forbid this and nothing makes the file
/// invalid — it is simply unbounded to a naive walker. A decomposer that hangs
/// has a defect; one that refuses the whole page has a different defect. The
/// right answer is to descend once, notice the repeat, stop, and **count it**,
/// because a silently truncated list presented as "everything on the page" is
/// the failure this project cares most about.
///
/// The guard is keyed on the form's **object number**: the same stream is
/// reachable under different resource names, so a name-keyed guard would miss
/// the cycle entirely.
#[test]
fn a_self_referential_form_terminates_and_is_counted() {
    let m = model("self-referential-form.pdf");

    assert_eq!(m.leaves.len(), 1, "the square inside, once");
    assert_eq!(
        m.diagnostics.form_cycles, 1,
        "★ the repeat must be COUNTED, not silently dropped -- an incomplete \
         list presented as complete is worse than a refusal"
    );
    assert_eq!(
        m.diagnostics.form_depth_overflows, 0,
        "the cycle guard caught it, not the depth bound"
    );
}

/// A form invoked twice contributes its contents twice, in two places, naming
/// the same form both times.
///
/// ★ Worth pinning because it looks like a bug and is not: it is what the page
/// actually draws. It is also exactly the situation that makes editing a leaf
/// inside a shared form change **every** invocation — `ARCHITECTURE.md` §12
/// decision 076, which rules that edit-in-place is the default and that
/// copy-on-write is a separate verb.
#[test]
fn a_form_invoked_twice_contributes_its_contents_twice() {
    let m = model("shared-form-twice.pdf");

    assert_eq!(form_count(&m), 2, "two invocations, two flat objects");
    assert_eq!(m.leaves.len(), 2);
    assert_eq!(
        m.leaves[0].parent(),
        m.leaves[1].parent(),
        "both name the SAME form -- that is what 'shared' means"
    );

    let mut origins: Vec<(i64, i64)> = m
        .leaves
        .iter()
        .map(|l| {
            let b = l.object.page_bbox();
            (b.min.x.round() as i64, b.min.y.round() as i64)
        })
        .collect();
    origins.sort_unstable();
    assert_eq!(
        origins,
        vec![(10, 10), (120, 120)],
        "the same contents, drawn in two different places"
    );
}

/// ★★★ THE SAFETY PROPERTY. Recursion must not put leaves into `objects`.
///
/// Eleven call sites in `edit.rs` resolve a paint-order index and apply
/// content-stream surgery **to the page's stream**. A leaf's token range
/// indexes the **form's** stream, a different buffer. A leaf in `objects` would
/// be handed to those verbs and corrupt the page silently, because the range is
/// in bounds.
///
/// Asserted as "the flat list contains only what the page's own stream drew",
/// which is the property those eleven sites depend on.
#[test]
fn the_flat_object_list_is_unchanged_by_recursion() {
    for name in [
        "page-sized-form.pdf",
        "nested-forms.pdf",
        "shared-form-twice.pdf",
    ] {
        let m = model(name);
        assert_eq!(
            m.objects.len(),
            form_count(&m),
            "{name}: every flat object is drawn by the PAGE's stream -- here \
             they are all `Do`s. A leaf appearing in this list is a corruption \
             hazard, not a cosmetic issue"
        );
        assert!(!m.leaves.is_empty(), "{name}: the leaves went somewhere");
    }
}

/// A leaf names its own content stream and refuses to claim it is editable.
///
/// Deliberately the same vocabulary `text_extract` uses for a `TextRun` inside
/// a form, so a form-interior path and a form-interior text run describe
/// themselves identically. A shell reconciles both in one selection; two
/// vocabularies for one fact would be its problem and our fault.
#[test]
fn a_leaf_names_its_own_stream_and_is_not_editable() {
    let m = model("page-sized-form.pdf");
    let leaf = &m.leaves[0];

    let parent = leaf.parent().expect("a leaf always has an enclosing form");
    assert_eq!(
        leaf.stream(),
        ContentStreamRef::Form { object: parent.num },
        "the leaf's token range indexes the FORM's buffer, not the page's"
    );
    assert!(
        !leaf.stream().is_page(),
        "a leaf is never in the page's own stream"
    );
    assert!(
        !leaf.is_editable(),
        "editing through the recursion is not built; claiming otherwise is how \
         a caller reaches for a verb that would corrupt the page"
    );
}
