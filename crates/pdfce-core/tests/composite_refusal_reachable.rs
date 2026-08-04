//! Regression: the composite (R-INV-4) refusal must be able to FIRE.
//!
//! # Why this deserves its own file
//!
//! The R-INV-4 refusal existed, was carefully worded, and could not be
//! reached. `edit-text` classified the anchor's font *after* `match_run`, and
//! `match_run` needs per-code slots that composite runs do not have — so a
//! composite run died there and reported `NoMatch`. The operator was told
//! *"text to edit was not found in an editable run on the page"*: their text
//! was present, in a font pdfce declines to edit, and the message said it was
//! absent. Two different problems, two different next actions, and only the
//! wrong one on offer.
//!
//! Nothing in the type system says a refusal must be reachable. The code
//! compiled, every test passed, and the carefully-worded message was dead.
//! That is the same shape as a guard behind an unpassable filter (R96) and as
//! an `#[allow]` insisting a dead function is live (R93) — and it is why this
//! is pinned by a test that asserts the ERROR VARIANT rather than by a
//! comment asserting the intent.
//!
//! # What would break it again
//!
//! Moving the font classification back below `match_run`, or dropping the
//! composite branch that decodes enough text for the run to be findable.
//! Either would restore `NoMatch`, and this test is the only thing that would
//! say so.

use std::path::{Path, PathBuf};

use pdfce_core::document::Document;
use pdfce_core::text_edit::{EditError, EditOptions, EditRequest, RInvTrigger, edit_text};

/// A composite (`/Type0`, `Identity-H`) fixture WITH an injective
/// `/ToUnicode`, so its text is genuinely findable.
///
/// The sibling `cidfonttype2-nocmap-embedded.pdf` cannot serve here: with no
/// character map its text is undecodable, so `NoMatch` is the honest answer
/// and the test would pass for the wrong reason.
fn composite_with_tounicode() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/synthetic/text/cidfonttype2-with-tounicode.pdf")
}

fn load(path: &Path) -> Document {
    let bytes =
        std::fs::read(path).unwrap_or_else(|e| panic!("missing fixture {}: {e}", path.display()));
    Document::from_bytes(bytes).expect("fixture parses")
}

#[test]
fn editing_a_composite_run_refuses_by_name_rather_than_reporting_no_match() {
    let doc = load(&composite_with_tounicode());
    let req = EditRequest::find_replace(0, "A", "B");
    let err = edit_text(&doc, &req, &EditOptions::default())
        .expect_err("a composite run must not be editable");

    match err {
        EditError::Refused(r) => {
            assert_eq!(
                r.trigger,
                RInvTrigger::Composite,
                "a composite run must refuse with the COMPOSITE trigger, not some other one"
            );
            // The message has to distinguish "this font can never be edited"
            // from "pdfce cannot do it yet" (R110). This fixture's CMap is
            // injective, so the honest answer is the second — and an
            // operator told the first would go looking for a different font
            // they do not need.
            let msg = r.message;
            assert!(
                msg.contains("composite"),
                "the refusal must name what it refused: {msg}"
            );
            assert!(
                msg.contains("CAN be inverted"),
                "this fixture's /ToUnicode is injective, so the refusal must say pdfce is the \
                 limitation rather than implying the font is: {msg}"
            );
        }
        EditError::NoMatch(what) => panic!(
            "REGRESSION: got NoMatch({what:?}) instead of the composite refusal. The text IS on \
             the page — reporting it as missing tells the operator to look for something that is \
             not the problem. This is the exact defect this file exists to pin: the font must be \
             classified BEFORE match_run, because a font-level refusal is a property of the run \
             rather than of whether the sought text happens to sit inside it."
        ),
        other => panic!("expected the composite refusal, got {other:?}"),
    }
}

/// The positive control: a SIMPLE embedded font must still edit.
///
/// The fix reorders a shared code path, so the way it could go wrong is by
/// refusing everything — which would satisfy the assertion above completely
/// while breaking in-place editing for every document that has ever worked.
#[test]
fn a_simple_font_run_still_edits_after_the_reorder() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/synthetic/text/subset-simple-embedded.pdf");
    let doc = load(&path);
    // Reordering only characters the embedded subset already carries, so
    // this exercises the edit path rather than the coverage floor (R-INV-1).
    let req = EditRequest::find_replace(0, "ABC", "ACB");
    let out = edit_text(&doc, &req, &EditOptions::default())
        .expect("a simple embedded font must still be editable after the reorder");
    assert!(
        !out.bytes.is_empty(),
        "a successful edit must produce a document"
    );
    Document::from_bytes(out.bytes).expect("the edited document must re-parse");
}
