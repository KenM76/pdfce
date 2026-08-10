//! # Integration tests for the embedded-file (attachment) reader
//!
//! These complement `attachments.rs`'s in-module unit tests rather than
//! repeating them. The unit tests assert *what each fixture contains*;
//! these assert properties that can only be checked **from outside the
//! crate**, or that hold across the whole fixture set at once:
//!
//! 1. **The public API is actually usable from outside.** A module can
//!    compile perfectly and still be unusable downstream — a public
//!    function returning a private type, a field a caller cannot read, a
//!    trait bound that only satisfies itself. `pdfce-cli` and `pdfce-gui`
//!    will consume this surface, and the cheapest place to discover that
//!    it does not work is here, not in the shell crates.
//! 2. **Nothing panics on hostile input**, checked by mutating every
//!    fixture byte-wise and re-reading it. This is the crate's
//!    panic-free policy (`lib.rs`) exercised against the module rather
//!    than merely asserted about it.
//! 3. **The listing is deterministic and bounded** — a GUI panel that
//!    reorders itself between refreshes is a defect, and so is one that
//!    can be made to allocate without limit.
//!
//! Spec context lives in the module's own docs; it is not repeated here.

use pdfce_core::attachments::{
    Attachment, AttachmentError, AttachmentKind, AttachmentNotes, DeclaredSizeCheck, NameHazard,
    SafeName, attachment_bytes, extract_attachment, list_attachments, list_attachments_with_notes,
    sanitize_attachment_name,
};
use pdfce_core::document::Document;

/// Every fixture in `fixtures/synthetic/attachments/`, by name.
const FIXTURES: [&str; 11] = [
    "doc-level-simple.pdf",
    "doc-level-unicode-name.pdf",
    "doc-level-kids-tree.pdf",
    "page-level-annot.pdf",
    "both-kinds.pdf",
    "size-lies.pdf",
    "flate-size-truth.pdf",
    "annot-contents-beats-desc.pdf",
    "ef-platform-slots.pdf",
    "hostile-names.pdf",
    "degenerate.pdf",
];

fn fixture_bytes(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/synthetic/attachments")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn load(name: &str) -> Document {
    Document::from_bytes(fixture_bytes(name)).unwrap_or_else(|e| panic!("parse {name}: {e}"))
}

/// Catches: a public function whose signature mentions a type a
/// downstream crate cannot name, and a struct whose fields are readable
/// in-crate but not out of it.
///
/// This test's *body* is almost beside the point — the value is that it
/// compiles. Every public type in the module is named in the `use` block
/// above and every documented field is touched below, so a visibility
/// regression is a build failure in this file rather than a surprise in
/// `pdfce-cli`.
#[test]
fn the_whole_public_surface_is_reachable_from_another_crate() {
    let doc = load("doc-level-simple.pdf");
    let view = doc.view();

    let (found, notes): (Vec<Attachment>, AttachmentNotes) = list_attachments_with_notes(&doc);
    let a: &Attachment = &found[0];

    // Every documented field, read.
    let _ = (&a.name, &a.name_bytes, a.name_source, a.name_exact);
    let _ = (&a.description, &a.kind, a.declared_size, a.size_check);
    let _ = (&a.mime, &a.created, &a.modified, &a.checksum);
    let _ = (a.stream_id, &a.ef_key, a.filespec_id);
    let _ = (
        notes.page_tree_unwalkable,
        notes.truncated,
        notes.name_tree_budget_exhausted,
        notes.name_tree_cycles,
        notes.malformed_tree_entries,
        notes.annotations_without_filespec,
        notes.filespecs_without_stream,
        notes.unresolvable_streams,
        notes.may_be_encrypted,
    );

    match &a.kind {
        AttachmentKind::DocumentLevel { tree_key } => assert!(!tree_key.is_empty()),
        AttachmentKind::PageAnnotation {
            page_index,
            page_id,
            annot_id,
            icon,
        } => {
            let _ = (page_index, page_id, annot_id, icon);
        }
        _ => {}
    }

    let safe: SafeName = a.safe_name();
    let _: (&str, bool, &[NameHazard]) = (&safe.value, safe.changed, &safe.hazards);
    assert_eq!(safe, sanitize_attachment_name(&a.name));

    let extracted = extract_attachment(&view, a).expect("this fixture has bytes");
    assert_eq!(Some(extracted.data.clone()), attachment_bytes(&view, a));
    let _: (Option<u64>, DeclaredSizeCheck) = (extracted.declared_size, extracted.size_check);

    // The error type is nameable and comparable downstream.
    let no_stream = AttachmentError::NoEmbeddedStream;
    assert!(!no_stream.to_string().is_empty());
}

/// Catches: any fixture becoming unreadable, and any of them producing a
/// listing that contradicts its own invariants (a `stream_id` with no
/// `ef_key`, a size verdict that does not match the declaration, a
/// `NameSource::TreeKey` on an annotation that has no key).
///
/// A sweep rather than nine separate tests, because the invariants are
/// properties of the *model* and should hold for every document forever,
/// including ones added later.
#[test]
fn every_fixture_yields_a_self_consistent_listing() {
    for name in FIXTURES {
        let doc = load(name);
        let view = doc.view();
        let found = list_attachments(&doc);

        for a in &found {
            // A stream and the key that supplied it travel together.
            assert_eq!(
                a.stream_id.is_some(),
                a.ef_key.is_some(),
                "{name}: stream_id and ef_key disagree for {:?}",
                a.name
            );

            // The size verdict must be consistent with the declaration.
            match a.size_check {
                DeclaredSizeCheck::NotDeclared => assert!(a.declared_size.is_none(), "{name}"),
                DeclaredSizeCheck::NoStream => {
                    assert!(a.declared_size.is_some(), "{name}");
                    assert!(a.stream_id.is_none(), "{name}");
                }
                DeclaredSizeCheck::Unverified
                | DeclaredSizeCheck::Agrees { .. }
                | DeclaredSizeCheck::Disagrees { .. } => {
                    assert!(a.declared_size.is_some(), "{name}");
                    assert!(a.stream_id.is_some(), "{name}");
                }
                _ => {}
            }

            // An annotation has no name-tree key, so it can never claim
            // to have taken its name from one.
            if matches!(a.kind, AttachmentKind::PageAnnotation { .. }) {
                assert_ne!(
                    a.name_source,
                    pdfce_core::attachments::NameSource::TreeKey,
                    "{name}: an annotation has no tree key to fall back on"
                );
            }

            // Extraction agrees with the listing about whether bytes exist.
            match extract_attachment(&view, a) {
                Ok(got) => {
                    assert!(a.stream_id.is_some(), "{name}: bytes from nowhere");
                    // Once decoded, the verdict is never "unverified".
                    assert_ne!(got.size_check, DeclaredSizeCheck::Unverified, "{name}");
                    if let Some(declared) = a.declared_size {
                        let actual = got.data.len() as u64;
                        let expected = if declared == actual {
                            DeclaredSizeCheck::Agrees { bytes: actual }
                        } else {
                            DeclaredSizeCheck::Disagrees { declared, actual }
                        };
                        assert_eq!(got.size_check, expected, "{name}");
                    }
                }
                Err(AttachmentError::NoEmbeddedStream) => {
                    assert!(a.stream_id.is_none(), "{name}");
                }
                Err(other) => panic!("{name}: unexpected extraction failure {other}"),
            }
        }
    }
}

/// Catches: a listing whose order depends on hash iteration or on some
/// other run-to-run nondeterminism.
///
/// A GUI attachment panel calls this on every refresh. Rows that reshuffle
/// under the operator's cursor are a defect, and one that only reproduces
/// occasionally is the worst kind.
#[test]
fn listing_is_deterministic() {
    for name in FIXTURES {
        let doc = load(name);
        let first = list_attachments(&doc);
        for _ in 0..8 {
            assert_eq!(list_attachments(&doc), first, "{name} reordered");
        }
    }
}

/// Catches: a panic, a hang, or an unbounded allocation on damaged input.
///
/// Every fixture is mutated one byte at a time across a fixed, seeded
/// stride, and each mutant that still parses as a document is listed and
/// fully extracted. Most mutants fail to parse — that is fine and is not
/// what this is testing; the ones that *do* parse are structurally
/// plausible garbage, which is exactly the input class §7.3.10's
/// "shall not be considered an error" rule exists for.
///
/// The seed is fixed rather than random so a failure is reproducible.
/// There is no `rand` dependency and there should not be one for this:
/// a 64-bit xorshift is four lines and cannot drift between runs.
#[test]
fn byte_level_corruption_never_panics() {
    // xorshift64*, fixed seed. Deterministic across platforms and runs.
    let mut state: u64 = 0x2026_0810_A77A_C4ED;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    let mut parsed = 0usize;
    for name in FIXTURES {
        let original = fixture_bytes(name);
        for _ in 0..300 {
            let mut mutant = original.clone();
            let pos = (next() as usize) % mutant.len();
            let byte = (next() % 256) as u8;
            mutant[pos] = byte;

            // A mutant that does not parse tells us nothing; skip it.
            let Ok(doc) = Document::from_bytes(mutant) else {
                continue;
            };
            parsed += 1;

            let view = doc.view();
            let (found, _notes) = list_attachments_with_notes(&doc);
            // The cap is a real cap even on damaged input.
            assert!(found.len() <= pdfce_core::attachments::MAX_ATTACHMENTS);
            for a in &found {
                // Both of these must be total functions on any input.
                let _ = a.safe_name();
                let _ = attachment_bytes(&view, a);
            }
        }
    }

    // Guard against the test silently becoming a no-op if a future parser
    // change starts rejecting every mutant.
    assert!(
        parsed > 100,
        "only {parsed} mutants parsed; this test is not exercising anything"
    );
}

/// Catches: `sanitize_attachment_name` violating any of its own promises
/// for *some* input. The unit tests check named cases; this checks the
/// invariants over a wide generated set, including every single Unicode
/// scalar in the ASCII and Latin-1 ranges plus a spread of structural
/// shapes.
///
/// The invariants, restated from the function's contract: the result is
/// always non-empty, is always a single path component, is never a
/// reserved device name, never ends in a dot or space, never contains a
/// control character or U+FFFD, and never exceeds the char cap.
#[test]
fn the_sanitiser_never_breaks_its_own_contract() {
    let mut cases: Vec<String> = Vec::new();

    // Every low code point, alone and embedded in a plausible filename.
    for cp in 0u32..0x0100 {
        if let Some(ch) = char::from_u32(cp) {
            cases.push(ch.to_string());
            cases.push(format!("file{ch}name.txt"));
            cases.push(format!("{ch}{ch}{ch}"));
        }
    }
    // Structural shapes that have historically broken sanitisers.
    for shape in [
        "",
        ".",
        "..",
        "...",
        "./",
        "../",
        "/",
        "\\",
        "//",
        r"\\server\share\file.txt",
        r"C:",
        r"C:\\",
        "con",
        "CON.",
        "COM1 ",
        "NUL:stream",
        "\u{FFFD}",
        "\u{FFFD}.exe",
        " leading space.txt",
        "trailing space.txt ",
        "\u{202E}gnp.exe",
        "🙂.txt",
    ] {
        cases.push(shape.to_owned());
    }
    // Length extremes.
    cases.push("z".repeat(10_000));
    cases.push(format!("{}{}", "z".repeat(10_000), "."));
    cases.push("é".repeat(5_000));

    for raw in &cases {
        let safe = sanitize_attachment_name(raw);
        let v = &safe.value;

        assert!(!v.is_empty(), "empty result for {raw:?}");
        assert!(
            !v.contains(['/', '\\']),
            "{raw:?} -> {v:?} is not a single component"
        );
        assert!(!v.contains(':'), "{raw:?} -> {v:?} kept a colon");
        assert!(
            !v.chars().any(char::is_control),
            "{raw:?} -> {v:?} kept a control character"
        );
        assert!(
            !v.contains('\u{FFFD}'),
            "{raw:?} -> {v:?} kept a replacement character"
        );
        assert!(
            !v.chars().any(|c| matches!(c,
                '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')),
            "{raw:?} -> {v:?} kept a bidi override"
        );
        assert!(
            !v.ends_with('.') && !v.ends_with(' '),
            "{raw:?} -> {v:?} ends in a dot or space"
        );
        assert_ne!(v, "..", "{raw:?} stayed a traversal");
        assert_ne!(v, ".", "{raw:?} stayed a dot component");
        assert!(
            v.chars().count() <= pdfce_core::attachments::MAX_SAFE_NAME_CHARS,
            "{raw:?} -> {} chars",
            v.chars().count()
        );

        // `changed` must be an honest description of the transformation,
        // in both directions — it is what a GUI keys "we renamed this" on.
        assert_eq!(safe.changed, v != raw, "{raw:?}: changed flag is wrong");
        if safe.changed {
            assert!(
                !safe.hazards.is_empty(),
                "{raw:?} was changed with no reason given"
            );
        } else {
            assert!(
                safe.hazards.is_empty(),
                "{raw:?} was unchanged but reported {:?}",
                safe.hazards
            );
        }
        // Idempotent: sanitising a sanitised name is a no-op. Otherwise a
        // caller that re-sanitises (easy to do accidentally through a
        // round trip) would keep mangling the name.
        let again = sanitize_attachment_name(v);
        assert_eq!(&again.value, v, "not idempotent for {raw:?}");
        assert!(!again.changed, "not idempotent for {raw:?}");
    }
}

/// Catches: the module's own headline promise — that a caller can tell
/// the two mechanisms apart *without* making two calls — being quietly
/// broken by a refactor that merges the kinds or drops the page index.
///
/// Deliberately phrased as the question a caller actually asks: "which of
/// these attachments would I lose if I deleted page 2?"
#[test]
fn a_caller_can_answer_which_attachments_a_page_delete_would_destroy() {
    let doc = load("both-kinds.pdf");
    let found = list_attachments(&doc);

    let doomed: Vec<&str> = found
        .iter()
        .filter(|a| matches!(a.kind, AttachmentKind::PageAnnotation { page_index: 1, .. }))
        .map(|a| a.name.as_str())
        .collect();
    let survivors: Vec<&str> = found
        .iter()
        .filter(|a| matches!(a.kind, AttachmentKind::DocumentLevel { .. }))
        .map(|a| a.name.as_str())
        .collect();

    assert_eq!(doomed, ["on-page-two.txt"]);
    assert_eq!(survivors, ["whole-document.txt"]);
}
