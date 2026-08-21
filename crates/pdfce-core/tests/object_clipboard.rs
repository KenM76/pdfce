//! `Pass 120.0`/`120.3` — the object clipboard: `copy_objects`,
//! `paste_objects`, `paste_preview`, `cut_objects`.
//!
//! ## The claim this Pass was filed against, and what checking it found
//!
//! `pdfceGUI` asked for a clipboard on the reading that
//! `EditSession::import_object` already does the hard part — a recursive
//! object-graph copy with reference remapping, cycle handling and stream
//! re-staging — so the ask was *"expose the one you have at object
//! granularity"*. **The reading is correct, and it is the smaller half.**
//!
//! `import_object` copies *indirect objects*. A page's content objects are
//! **byte ranges inside a content stream**, and the operators in those bytes
//! name their resources **by page-local name**: `/F1 12 Tf`, `/Im1 Do`. On the
//! destination page `/F1` is a different font. Pasting the bytes verbatim
//! draws the right shapes in the wrong typeface, or draws nothing, **and
//! neither failure errors.**
//!
//! So the tests here are weighted accordingly: the object-graph copy gets one
//! test, and **resource-name rebinding gets four**, because that is where the
//! silent wrongness lives.
//!
//! ## What is pinned
//!
//! 1. **★ A paste into a page whose `/F1` is a DIFFERENT font renders the
//!    clip's font, not the destination's** — the whole point, and the one
//!    failure a shell could not detect for itself.
//! 2. **The clip owns its resources**, so copy → drop the source session →
//!    paste still works.
//! 3. Paste-in-place, paste-with-offset and paste-rotated are one verb taking
//!    a page-space matrix.
//! 4. The preview and the verb cannot disagree; the preview commits nothing.
//! 5. Cut is **one** undo entry, and refuses with nothing deleted if the copy
//!    half refuses.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use pdfce_core::document::Document;
use pdfce_core::edit::EditSession;
use pdfce_core::vector::{Matrix, Point};
use pdfce_core::writer::SaveOptions;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// A one-page PDF whose content is `content`, whose `/F1` is `base_font`, and
/// which carries an image XObject `/Im1`.
///
/// `base_font` is a parameter for exactly one test — the one that pastes
/// between two documents whose `/F1` means different things — and that test is
/// the reason this whole Pass is more than `import_object`.
fn pdf_with_font(content: &str, base_font: &str) -> Vec<u8> {
    let image = "<< /Type /XObject /Subtype /Image /Width 1 /Height 1 \
                 /ColorSpace /DeviceGray /BitsPerComponent 8 /Length 1 >>\nstream\n\u{0}\nendstream";
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
         /Resources << /Font << /F1 5 0 R >> /XObject << /Im1 6 0 R >> >> >>"
            .to_owned(),
        format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len() + 1
        ),
        format!(
            "<< /Type /Font /Subtype /Type1 /BaseFont /{base_font} /Encoding /WinAnsiEncoding >>"
        ),
        image.to_owned(),
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

fn pdf(content: &str) -> Vec<u8> {
    pdf_with_font(content, "Helvetica")
}

const MIXED: &str =
    "0 0 10 10 re S\nBT /F1 12 Tf 20 20 Td (hi) Tj ET\nq 5 0 0 5 40 40 cm /Im1 Do Q";

/// The saved bytes of a session, as text, for byte-level assertions.
fn saved(session: &EditSession) -> String {
    let (bytes, _) = session
        .to_incremental_bytes(&SaveOptions::identity())
        .unwrap();
    String::from_utf8_lossy(&bytes).into_owned()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

// ---------------------------------------------------------------------------
// ★ THE ONE THAT MATTERS: resource-name rebinding
// ---------------------------------------------------------------------------

/// ★★ **A text run copied from a Helvetica document and pasted into a
/// Courier one must still be Helvetica.**
///
/// Both documents call their font `/F1`. Pasting the copied bytes verbatim
/// would bind to the *destination's* `/F1` and silently render the text in
/// Courier — the right glyphs, the wrong typeface, no error anywhere. This is
/// the failure `import_object` alone cannot prevent, because the name is not a
/// reference and there is nothing in the object graph to remap.
///
/// The assertion is on the saved bytes rather than on a render, because what
/// must be true is structural: the pasted operator names a resource that
/// resolves to a Helvetica font.
#[test]
fn a_pasted_font_is_the_clips_font_not_the_destinations() {
    let source = Document::from_bytes(pdf_with_font(MIXED, "Helvetica")).unwrap();
    let source_session = EditSession::new(source);
    let clip = source_session.copy_objects(0, &[1]).expect("copy the text");

    let destination = Document::from_bytes(pdf_with_font("0 0 5 5 re f", "Courier")).unwrap();
    let mut session = EditSession::new(destination);
    let outcome = session
        .paste_objects(0, &clip, Matrix::IDENTITY)
        .expect("paste succeeds");
    assert_eq!(outcome.objects_pasted, 1);
    assert!(
        outcome.resources_added >= 1,
        "the font had to arrive with it: {outcome:?}"
    );

    let text = saved(&session);
    assert!(
        text.contains("/Helvetica"),
        "the clip's own font must have been imported: {text}"
    );
    assert!(
        !text.contains("/F1 12 Tf"),
        "the pasted operator must NOT still say /F1 -- that is the destination's Courier: {text}"
    );
}

/// The rebound name is the one actually written into the page's
/// `/Resources`, not merely *a* fresh name.
///
/// Detecting the name in the content and binding it in the resources are two
/// halves that could disagree — and if they did, the paste would name a
/// resource that is not there, which draws nothing.
#[test]
fn the_rewritten_name_is_the_one_bound_in_the_pages_resources() {
    let source = Document::from_bytes(pdf(MIXED)).unwrap();
    let session_a = EditSession::new(source);
    let clip = session_a.copy_objects(0, &[1]).unwrap();

    let destination = Document::from_bytes(pdf("0 0 5 5 re f")).unwrap();
    let mut session = EditSession::new(destination);
    session.paste_objects(0, &clip, Matrix::IDENTITY).unwrap();

    let text = saved(&session);
    // Find the name the pasted `Tf` uses, then assert the page binds it.
    let tf = text.find(" Tf").expect("a Tf survived the paste");
    let head = &text[..tf];
    let name_start = head.rfind('/').expect("the Tf names a font");
    let name: String = head[name_start + 1..]
        .split_whitespace()
        .next()
        .unwrap()
        .to_owned();
    assert!(
        name.starts_with("pdfceP"),
        "a pasted binding gets a fresh pdfce-prefixed name, got {name:?}"
    );
    // ★ TWO occurrences, not one. `contains` alone was satisfied by the
    // CONTENT STREAM's own `/pdfcePF0 12 Tf` — so the first draft asserted
    // "the name I just found is present", which is a tautology, and passed
    // with the resource binding disabled. The second occurrence is the
    // `/Resources` entry, which is the half being claimed.
    assert!(
        text.matches(&format!("/{name}")).count() >= 2,
        "the page's /Resources must bind {name:?} as well as the content naming it: {text}"
    );
}

/// An `/XObject` invocation is rebound too — the same machinery, a different
/// category.
///
/// Pinned separately rather than assumed from the font case: the two go
/// through the same table, and a table with one wrong row fails for exactly
/// one category while every font test stays green.
#[test]
fn an_image_invocation_is_rebound_as_well() {
    let source = Document::from_bytes(pdf(MIXED)).unwrap();
    let session_a = EditSession::new(source);
    let clip = session_a.copy_objects(0, &[2]).expect("copy the image");
    assert_eq!(clip.kinds(), vec!["image"]);
    assert_eq!(
        clip.resource_count(),
        1,
        "the image object itself must travel on the clip"
    );

    let destination = Document::from_bytes(pdf("0 0 5 5 re f")).unwrap();
    let mut session = EditSession::new(destination);
    let outcome = session.paste_objects(0, &clip, Matrix::IDENTITY).unwrap();
    assert_eq!(outcome.resources_added, 1);
    let text = saved(&session);
    assert!(
        text.contains("pdfcePX0 Do"),
        "the pasted Do must name the freshly-bound XObject: {text}"
    );
    // ★ AND the object it names must have ARRIVED. The first draft stopped at
    // the line above and passed with the whole resource import disabled --
    // name rewriting alone satisfied it, so it tested half the mechanism while
    // reading as if it tested both.
    let subtype_count = text.matches("/Subtype /Image").count();
    assert_eq!(
        subtype_count, 2,
        "the destination's own image plus the imported one: {text}"
    );
}

/// A path names no resource at all, so its clip carries none — the negative
/// case, which stops "everything gets a resource" from passing vacuously.
#[test]
fn a_plain_path_carries_no_resources() {
    let doc = Document::from_bytes(pdf(MIXED)).unwrap();
    let session = EditSession::new(doc);
    let clip = session.copy_objects(0, &[0]).expect("copy the path");
    assert_eq!(clip.kinds(), vec!["path"]);
    assert_eq!(
        clip.resource_count(),
        0,
        "a stroked rectangle references nothing"
    );
}

// ---------------------------------------------------------------------------
// The clip owns what it needs
// ---------------------------------------------------------------------------

/// ★ **Copy, drop the source session, paste.**
///
/// The clip carries the transitive closure of its resources by value, with
/// stream payloads owned as bytes rather than as spans into a document that
/// may already be gone. That is what makes cross-document paste the same code
/// path as same-document paste — and what will make `Pass 120.1`'s `to_bytes`
/// a serialisation problem rather than a design problem.
#[test]
fn a_clip_outlives_the_document_it_came_from() {
    let clip = {
        let doc = Document::from_bytes(pdf_with_font(MIXED, "Times-BoldItalic")).unwrap();
        let session = EditSession::new(doc);
        session.copy_objects(0, &[1, 2]).unwrap()
        // `session` and its `Document` are dropped here.
    };
    assert_eq!(clip.len(), 2);

    let destination = Document::from_bytes(pdf("0 0 5 5 re f")).unwrap();
    let mut session = EditSession::new(destination);
    let outcome = session.paste_objects(0, &clip, Matrix::IDENTITY).unwrap();
    assert_eq!(outcome.objects_pasted, 2);
    let text = saved(&session);
    // ★ `Times-BoldItalic`, not Helvetica. The first draft asserted the source
    // font was `/Helvetica` -- which the DESTINATION fixture also uses, so the
    // test passed with the entire resource import disabled. A distinctive font
    // is the difference between exercising the mechanism and covering it.
    assert!(
        text.contains("/Times-BoldItalic"),
        "the clip's own font must have arrived from a document that is gone: {text}"
    );
}

/// Paint order survives the round trip.
///
/// Pasting a selection back in a different order restacks it — a filled shape
/// that was behind text arriving in front of it — which is a visible change
/// nobody asked for and which no error reports.
#[test]
fn paint_order_is_preserved() {
    let doc = Document::from_bytes(pdf(MIXED)).unwrap();
    let session = EditSession::new(doc);
    let clip = session.copy_objects(0, &[0, 1, 2]).unwrap();
    let kinds: Vec<&str> = clip.items.iter().map(|i| i.kind).collect();
    assert_eq!(kinds, vec!["path", "text", "image"]);
}

// ---------------------------------------------------------------------------
// Placement — one verb, four gestures
// ---------------------------------------------------------------------------

/// Paste-in-place puts the content back where it was; paste-with-offset moves
/// it by exactly the offset.
///
/// Asserted on the reported `bbox`, which is what a shell draws its paste
/// outline from — so a wrong answer here is wrong on screen before it is wrong
/// in the file.
#[test]
fn paste_in_place_and_with_offset_land_where_they_say() {
    let doc = Document::from_bytes(pdf(MIXED)).unwrap();
    let session = EditSession::new(doc);
    let clip = session.copy_objects(0, &[0]).unwrap();
    let source_bbox = clip.bbox();

    let in_place = session.paste_preview(0, &clip, Matrix::IDENTITY).unwrap();
    assert!(
        close(in_place.bbox.min.x, source_bbox.min.x)
            && close(in_place.bbox.max.y, source_bbox.max.y),
        "paste-in-place must not move it: {:?} vs {source_bbox:?}",
        in_place.bbox
    );

    let offset = session
        .paste_preview(0, &clip, Matrix::translate(100.0, 50.0))
        .unwrap();
    assert!(
        close(offset.bbox.min.x, source_bbox.min.x + 100.0)
            && close(offset.bbox.min.y, source_bbox.min.y + 50.0),
        "paste-with-offset must move by exactly the offset: {:?}",
        offset.bbox
    );
}

/// A rotated paste's reported bounds map **all four corners**.
///
/// The naive two-corner version is right for a translation and a scale and
/// wrong for a rotation — and a paste outline that is wrong only when rotated
/// is the kind of bug that ships.
#[test]
fn a_rotated_paste_reports_bounds_that_enclose_the_rotation() {
    let doc = Document::from_bytes(pdf("0 0 100 10 re S")).unwrap();
    let session = EditSession::new(doc);
    let clip = session.copy_objects(0, &[0]).unwrap();
    let before = clip.bbox();
    let quarter = Matrix::rotate(std::f64::consts::FRAC_PI_2).about(Point::new(0.0, 0.0));
    let rotated = session.paste_preview(0, &clip, quarter).unwrap().bbox;

    let wide = before.max.x - before.min.x;
    let tall = rotated.max.y - rotated.min.y;
    assert!(
        close(tall, wide),
        "a quarter-turn makes the wide box tall: {rotated:?} from {before:?}"
    );
}

// ---------------------------------------------------------------------------
// The preview
// ---------------------------------------------------------------------------

/// ★ The preview commits nothing and answers exactly what the paste would.
///
/// Three cases, including two refusals, because agreement on the happy path is
/// what a second implementation would also manage.
#[test]
fn the_preview_answers_exactly_what_the_paste_would() {
    let doc = Document::from_bytes(pdf(MIXED)).unwrap();
    let mut session = EditSession::new(doc);
    let good = session.copy_objects(0, &[0, 1]).unwrap();

    let mut future = good.clone();
    future.version = 999;

    let mut dangling = good.clone();
    dangling.objects.clear();

    for (label, clip) in [
        ("good", &good),
        ("from the future", &future),
        ("dangling", &dangling),
    ] {
        let previewed = session.paste_preview(0, clip, Matrix::IDENTITY);
        assert_eq!(session.undo_depth(), 0, "preview committed for {label}");
        let applied = session.paste_objects(0, clip, Matrix::IDENTITY);
        assert_eq!(
            previewed.is_ok(),
            applied.is_ok(),
            "preview and paste disagree for {label}: {previewed:?} vs {applied:?}"
        );
        if applied.is_ok() {
            session.undo();
        }
    }
}

/// A payload from a newer build is refused **by name**, not partially
/// understood.
#[test]
fn a_clip_from_a_newer_build_is_refused_by_name() {
    let doc = Document::from_bytes(pdf(MIXED)).unwrap();
    let mut session = EditSession::new(doc);
    let mut clip = session.copy_objects(0, &[0]).unwrap();
    clip.version = pdfce_core::vector::CLIP_VERSION + 1;

    let err = session
        .paste_objects(0, &clip, Matrix::IDENTITY)
        .expect_err("a newer format is refused");
    let message = err.to_string();
    assert!(
        message.contains("newer build"),
        "the refusal must say which way the mismatch runs: {message}"
    );
    assert_eq!(session.undo_depth(), 0);
}

// ---------------------------------------------------------------------------
// Cut — Pass 120.3
// ---------------------------------------------------------------------------

/// ★ **Cut is ONE undo entry.**
///
/// The requester's own framing: *"otherwise Ctrl+X then Ctrl+Z gives the
/// operator their objects back but leaves the clipboard changed, or takes two
/// presses."* The copy half is `&self` and commits nothing, so only the
/// deletion reaches the undo stack.
#[test]
fn cut_is_one_undo_entry_and_returns_the_clip() {
    let doc = Document::from_bytes(pdf(MIXED)).unwrap();
    let mut session = EditSession::new(doc);
    let clip = session.cut_objects(0, &[1]).expect("cut the text run");
    assert_eq!(clip.len(), 1);
    assert_eq!(
        session.undo_depth(),
        1,
        "one gesture, one undo -- not one for the copy and one for the delete"
    );

    session.undo().expect("the cut undoes");
    let (_bytes, report) = session
        .to_incremental_bytes(&SaveOptions::identity())
        .unwrap();
    assert_eq!(
        report.objects_written, 0,
        "cut then undo leaves no trace in the save: {report:?}"
    );
}

/// ★ **A cut whose COPY half refuses deletes nothing.**
///
/// The order matters and is not incidental: copy first, delete second. Reversed,
/// a selection that cannot be copied would be gone with nothing on the
/// clipboard — the one outcome from which the operator cannot recover by
/// pasting.
#[test]
fn a_cut_that_cannot_copy_deletes_nothing() {
    let doc = Document::from_bytes(pdf(MIXED)).unwrap();
    let mut session = EditSession::new(doc);
    let err = session
        .cut_objects(0, &[0, 99])
        .expect_err("99 is not on this page");
    assert!(err.to_string().contains("99"), "{err}");
    assert_eq!(
        session.undo_depth(),
        0,
        "nothing was deleted -- the copy refused first"
    );
}

// ---------------------------------------------------------------------------
// Session hygiene
// ---------------------------------------------------------------------------

/// Paste is one undoable command however many objects arrive, and undo leaves
/// the file byte-identical.
#[test]
fn paste_is_one_command_and_undoes_completely() {
    let doc = Document::from_bytes(pdf(MIXED)).unwrap();
    let mut session = EditSession::new(doc);
    let clip = session.copy_objects(0, &[0, 1, 2]).unwrap();
    let outcome = session.paste_objects(0, &clip, Matrix::IDENTITY).unwrap();
    assert_eq!(outcome.objects_pasted, 3);
    assert_eq!(session.undo_depth(), 1, "three objects, one command");

    session.undo().expect("the paste undoes");
    let (_bytes, report) = session
        .to_incremental_bytes(&SaveOptions::identity())
        .unwrap();
    assert_eq!(
        report.objects_written, 0,
        "a pasted-then-undone page must appear in no update section: {report:?}"
    );
}

/// Copying commits nothing — it is `&self`, and the undo stack proves it.
#[test]
fn copying_commits_nothing() {
    let doc = Document::from_bytes(pdf(MIXED)).unwrap();
    let session = EditSession::new(doc);
    let _clip = session.copy_objects(0, &[0, 1, 2]).unwrap();
    assert_eq!(session.undo_depth(), 0);
    assert!(!session.is_modified());
}

/// An empty selection produces an empty clip, and pasting one is a no-op
/// rather than an error — a caller need not special-case it.
#[test]
fn an_empty_clip_pastes_as_a_no_op() {
    let doc = Document::from_bytes(pdf(MIXED)).unwrap();
    let mut session = EditSession::new(doc);
    let clip = session.copy_objects(0, &[]).unwrap();
    assert!(clip.is_empty());
    let outcome = session.paste_objects(0, &clip, Matrix::IDENTITY).unwrap();
    assert_eq!(outcome.objects_pasted, 0);
    assert_eq!(session.undo_depth(), 0, "a no-op paste is not a command");
}

/// Pasting the same clip twice yields two independent copies, each with its
/// own resource binding — a paste must not alias the previous one's names.
#[test]
fn pasting_twice_binds_two_independent_sets() {
    let doc = Document::from_bytes(pdf(MIXED)).unwrap();
    let mut session = EditSession::new(doc);
    let clip = session.copy_objects(0, &[1]).unwrap();
    let first = session.paste_objects(0, &clip, Matrix::IDENTITY).unwrap();
    let second = session
        .paste_objects(0, &clip, Matrix::translate(10.0, 0.0))
        .unwrap();
    assert_eq!(first.resources_added, second.resources_added);
    assert_eq!(session.undo_depth(), 2, "two gestures, two undo entries");

    let text = saved(&session);
    assert!(
        text.contains("pdfcePF0") && text.contains("pdfcePF1"),
        "the second paste must not reuse the first's binding: {text}"
    );
}

// ---------------------------------------------------------------------------
// Serialisation — Pass 120.1
// ---------------------------------------------------------------------------

/// ★ **A serialised clip round-trips, and the round trip is what makes
/// cross-session paste free rather than a second feature.**
///
/// The strong form: serialise, parse, and paste the PARSED clip into a
/// document whose `/F1` is a different font. If anything is lost on the way
/// through — the bindings, the CTM, the font object's payload — the paste
/// either refuses or renders wrong, and both are caught here rather than by an
/// operator six months later.
#[test]
fn a_serialised_clip_round_trips_and_still_pastes_correctly() {
    let source = Document::from_bytes(pdf_with_font(MIXED, "Times-BoldItalic")).unwrap();
    let session_a = EditSession::new(source);
    let clip = session_a.copy_objects(0, &[1, 2]).unwrap();

    let bytes = clip.to_bytes();
    let parsed = pdfce_core::vector::ObjectClip::from_bytes(&bytes).expect("it parses back");
    assert_eq!(
        parsed, clip,
        "the round trip must be exact, not approximate"
    );

    let destination = Document::from_bytes(pdf_with_font("0 0 5 5 re f", "Courier")).unwrap();
    let mut session = EditSession::new(destination);
    let outcome = session
        .paste_objects(0, &parsed, Matrix::IDENTITY)
        .expect("the parsed clip pastes");
    assert_eq!(outcome.objects_pasted, 2);
    let text = saved(&session);
    assert!(
        text.contains("/Times-BoldItalic"),
        "the font must have survived serialisation: {text}"
    );
}

/// The matrix survives **bit-exactly**.
///
/// Decimal round-tripping would change a CTM in the last place on every
/// copy/paste cycle, and a shape that drifts a little every time is a bug that
/// takes months to attribute. Asserted on an awkward value rather than on a
/// round one, because `1.0` round-trips through anything.
#[test]
fn a_matrix_survives_serialisation_bit_exactly() {
    let doc =
        Document::from_bytes(pdf("q 0.13333 0 0 0.13333 7.77 3.331 cm 0 0 10 10 re S Q")).unwrap();
    let session = EditSession::new(doc);
    let clip = session.copy_objects(0, &[0]).unwrap();
    let parsed =
        pdfce_core::vector::ObjectClip::from_bytes(&clip.to_bytes()).expect("it parses back");
    let bits = |m: Matrix| [m.a, m.b, m.c, m.d, m.e, m.f].map(f64::to_bits);
    assert_eq!(
        bits(parsed.items[0].ctm),
        bits(clip.items[0].ctm),
        "the CTM must be bit-identical after a round trip"
    );
}

/// A payload that is not a clip is refused **by name**, before any length
/// prefix is read.
#[test]
fn a_foreign_payload_is_refused_by_name() {
    let err = pdfce_core::vector::ObjectClip::from_bytes(b"this is not a clip at all")
        .expect_err("a foreign payload is refused");
    assert!(
        err.to_string().contains("not a pdfce clipboard payload"),
        "{err}"
    );
}

/// A truncated payload is refused, not read past.
///
/// Swept over **every** prefix length rather than one, because a length-prefix
/// format has as many truncation points as it has fields and "it survived the
/// one I tried" is not the claim being made.
#[test]
fn every_truncation_is_refused_rather_than_read_past() {
    let doc = Document::from_bytes(pdf(MIXED)).unwrap();
    let session = EditSession::new(doc);
    let bytes = session.copy_objects(0, &[0, 1, 2]).unwrap().to_bytes();

    for cut in 0..bytes.len() {
        let result = pdfce_core::vector::ObjectClip::from_bytes(&bytes[..cut]);
        assert!(
            result.is_err(),
            "a payload truncated to {cut} of {} bytes must be refused",
            bytes.len()
        );
    }
    // And the whole thing still parses, so the sweep above is not passing
    // because the parser refuses everything.
    assert!(pdfce_core::vector::ObjectClip::from_bytes(&bytes).is_ok());
}

/// A serialised clip from a newer build is refused **before** its body is
/// read.
#[test]
fn a_serialised_clip_from_a_newer_build_is_refused() {
    let doc = Document::from_bytes(pdf(MIXED)).unwrap();
    let session = EditSession::new(doc);
    let mut clip = session.copy_objects(0, &[0]).unwrap();
    clip.version = pdfce_core::vector::CLIP_VERSION + 1;
    let err = pdfce_core::vector::ObjectClip::from_bytes(&clip.to_bytes())
        .expect_err("a newer format is refused");
    assert!(err.to_string().contains("newer build"), "{err}");
}
