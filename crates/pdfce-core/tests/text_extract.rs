//! # Pass 4 integration tests — the §9.10 extraction contract, end to end
//!
//! Unit tests in `text_extract::{cmap, layout}` cover the pieces in
//! isolation. These drive the whole pipeline over the synthetic fixtures
//! in `fixtures/synthetic/text/` (provenance: that directory's
//! `PROVENANCE.md`), one fixture per clause, so a failure names the
//! clause it broke.
//!
//! The organising assertion in almost every test below is the pair
//! `plain_text()` / `sourced_text()`. That pair is the executable form
//! of the module's central claim — that pdfce knows which characters
//! came from the file and which it invented — and a bug that blurs the
//! two shows up here as the two accessors returning the same string when
//! they should differ, or differing when they should not.

use std::path::{Path, PathBuf};

use pdfce_core::document::Document;
use pdfce_core::text_extract::{
    self, ArtifactKind, ExtractOptions, ExtractedText, LadderRung, TextOrigin,
};

/// A fixture path under `fixtures/synthetic/text/`.
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/synthetic/text")
        .join(name)
}

/// Load and extract with default options.
fn extract(name: &str) -> ExtractedText {
    extract_with(name, &ExtractOptions::default())
}

/// Load and extract with explicit options.
fn extract_with(name: &str, options: &ExtractOptions) -> ExtractedText {
    let doc = Document::load(&fixture(name)).expect("fixture loads");
    text_extract::extract_document(&doc, options).expect("extraction runs")
}

// ---------------------------------------------------------------------------
// Ladder rung 2 — simple font, standard encoding, AGL
// ---------------------------------------------------------------------------

#[test]
fn rung2_resolves_a_winansi_simple_font() {
    let text = extract("simple-winansi.pdf");
    assert_eq!(text.sourced_text(), "HelloworldSecond line");
    let d = &text.diagnostics;
    assert_eq!(d.codes_total, 21);
    assert_eq!(
        d.via_encoding_agl, 21,
        "every code must come from §9.10.2 method 2"
    );
    assert_eq!(d.via_to_unicode, 0);
    assert_eq!(d.ladder_failures, 0);
    assert_eq!(d.via_glyph_name_extension, 0, "the precondition HOLDS here");
    assert_eq!(d.sourced_fraction(), Some(1.0));
}

#[test]
fn a_tj_offset_with_no_space_glyph_derives_a_space() {
    // S3/S4: the gap between "Hello" and "world" is a TJ offset and
    // nothing else — there is no space character anywhere in the file.
    // The derived space must appear in plain text, must NOT appear in
    // sourced text, and must be counted.
    let text = extract("simple-winansi.pdf");
    assert!(
        text.plain_text().contains("Hello world"),
        "plain text: {:?}",
        text.plain_text()
    );
    assert!(
        !text.sourced_text().contains("Hello world"),
        "the space is pdfce's, not the file's"
    );
    assert_eq!(text.diagnostics.spaces_derived, 1);
}

#[test]
fn a_new_td_line_derives_a_line_break() {
    // S5: no line markers exist in a content stream.
    let text = extract("simple-winansi.pdf");
    assert!(text.plain_text().contains("world\nSecond"));
    assert!(text.sourced_text().contains("worldSecond"));
    assert_eq!(text.diagnostics.lines_derived, 1);
}

#[test]
fn an_untagged_document_says_so_by_name() {
    let text = extract("simple-winansi.pdf");
    assert!(!text.diagnostics.tagged);
    assert!(
        text.diagnostics
            .notes
            .iter()
            .any(|n| n.contains("untagged document")),
        "notes: {:?}",
        text.diagnostics.notes
    );
}

// ---------------------------------------------------------------------------
// Ladder rung 1 — /ToUnicode, all three §9.10.3 forms
// ---------------------------------------------------------------------------

#[test]
fn rung1_resolves_identity_h_through_to_unicode() {
    let text = extract("identity-h-tounicode.pdf");
    let sourced = text.sourced_text();
    // Form B (<0001>..<0003> over <0048>) gives H, I, J.
    assert!(sourced.starts_with("HIJ"), "sourced: {sourced:?}");
    // Form C's middle array element is a THREE code point ligature.
    assert!(sourced.contains("ffl"), "one-to-many mapping lost");
    // Form A's surrogate pair is a supplementary-plane character.
    assert!(
        sourced.contains('\u{2003E}'),
        "surrogate pair truncated — a UCS-2 decoder would do exactly this"
    );
}

#[test]
fn an_uncovered_code_falls_through_the_ladder_and_is_counted() {
    // <0099> is deliberately absent from the fixture's ToUnicode CMap.
    // §9.10.3 N4 records that the standard says nothing about an
    // uncovered code; pdfce's per-code fallthrough finds nothing else
    // (a composite font has no rung 2), so it must reach the failure
    // clause — U+FFFD, counted, never invented.
    let text = extract("identity-h-tounicode.pdf");
    let d = &text.diagnostics;
    assert_eq!(d.ladder_failures, 1);
    assert!(text.sourced_text().contains('\u{FFFD}'));
    assert_eq!(d.via_to_unicode, d.codes_total - 1);
}

#[test]
fn extraction_succeeds_on_a_file_rendering_correctly_refuses() {
    // §9.7.5.2 forbids Identity-H with a non-embedded font, and
    // pdfce-render refuses such a font outright. §9.10.2 rung 1 needs
    // only the /ToUnicode entry. The two directions have different
    // requirements and this is the file that proves it.
    let text = extract("identity-h-tounicode.pdf");
    assert!(text.diagnostics.codes_total > 0);
    assert!(text.diagnostics.via_to_unicode > 0);
}

#[test]
fn one_code_may_produce_several_glyphs_worth_of_text() {
    let text = extract("identity-h-tounicode.pdf");
    let ligature = text
        .pages
        .iter()
        .flat_map(|p| p.runs.iter())
        .flat_map(|r| r.glyphs.iter().map(move |g| (r, g)))
        .find(|(_, g)| g.code == 0x0011)
        .expect("the ligature code was shown");
    let (run, glyph) = ligature;
    assert_eq!(glyph.text_len, 3, "ffl is three bytes, not one");
    let start = glyph.text_start as usize;
    assert_eq!(&run.text[start..start + glyph.text_len as usize], "ffl");
}

// ---------------------------------------------------------------------------
// The Identity-H dead end — Pass 4's headline honesty metric
// ---------------------------------------------------------------------------

#[test]
fn identity_h_without_to_unicode_recovers_nothing_and_says_why() {
    let text = extract("identity-h-no-tounicode.pdf");
    let d = &text.diagnostics;

    assert!(d.codes_total > 0, "codes were shown");
    assert_eq!(
        d.ladder_failures, d.codes_total,
        "§9.10.2 excludes Identity-H from rung 3 by name and an \
         Adobe-Identity-0 descendant satisfies neither half of the second \
         disjunct — EVERY code must fail"
    );
    assert_eq!(d.sourced_codes(), 0);
    assert_eq!(d.sourced_fraction(), Some(0.0));
    assert_eq!(d.identity_fonts_without_to_unicode, 1);

    // Every extracted character is the replacement character, and none
    // of them is a plausible-looking guess reconstructed from glyph
    // indices.
    let sourced = text.sourced_text();
    assert!(!sourced.is_empty());
    assert!(
        sourced
            .chars()
            .all(|c| c == '\u{FFFD}' || c.is_whitespace()),
        "a fabricated character escaped: {sourced:?}"
    );

    assert!(
        d.notes
            .iter()
            .any(|n| n.contains("Identity-H") && n.contains("no Unicode is recoverable")),
        "the dead end must be named, not merely counted: {:?}",
        d.notes
    );
}

#[test]
fn every_failed_glyph_is_marked_as_such_individually() {
    let text = extract("identity-h-no-tounicode.pdf");
    let glyphs: Vec<_> = text
        .pages
        .iter()
        .flat_map(|p| p.runs.iter())
        .flat_map(|r| r.glyphs.iter())
        .collect();
    assert!(!glyphs.is_empty());
    assert!(glyphs.iter().all(|g| g.rung == LadderRung::Failed));
    assert!(glyphs.iter().all(|g| !g.rung.is_sourced()));
}

// ---------------------------------------------------------------------------
// /ActualText — §14.9.4's own example
// ---------------------------------------------------------------------------

#[test]
fn actual_text_replaces_the_glyphs_it_covers() {
    // §14.9.4's EXAMPLE: the glyphs read Dru / k- / ker, the sequence's
    // /ActualText is (c), and the clause's own gloss is that the
    // character content is "Drucker".
    let text = extract("actual-text-drucker.pdf");
    assert_eq!(
        text.sourced_text(),
        "Drucker",
        "the clause's own worked example is the assertion"
    );
    assert_eq!(text.diagnostics.actual_text_applied, 1);
}

#[test]
fn plain_text_keeps_the_derived_line_break_the_sourced_text_omits() {
    // The `'` operator moves to the next line, so pdfce derives a line
    // break there — honestly, because the baseline really did move. The
    // pair of accessors is what makes both answers available: the
    // standard's "Drucker" is the sourced one.
    let text = extract("actual-text-drucker.pdf");
    assert_eq!(text.plain_text(), "Druc\nker");
    assert_eq!(text.sourced_text(), "Drucker");
    assert_eq!(text.diagnostics.lines_derived, 1);
}

#[test]
fn an_actual_text_run_is_atomic_and_carries_no_glyphs() {
    // §14.9.4 N4: no length relationship exists between replacement and
    // replaced content (2 shown characters become 1 here), so
    // character-level mapping back to glyph positions is impossible —
    // not merely unimplemented. The API has to say so.
    let text = extract("actual-text-drucker.pdf");
    let run = text
        .pages
        .iter()
        .flat_map(|p| p.runs.iter())
        .find(|r| r.origin == TextOrigin::ActualText)
        .expect("the replacement run exists");
    assert_eq!(run.text, "c");
    assert!(run.glyphs.is_empty());
    assert!(
        run.is_sourced(),
        "the VALUE is sourced; only its offsets are not"
    );
    assert!(
        run.bbox.is_some(),
        "the covered region is the only positional information available"
    );
}

#[test]
fn no_derived_word_space_is_inserted_next_to_a_replacement() {
    // §14.9.4 NOTE 2 makes ActualText a *character* substitution, and
    // requires no word break between consecutive ones. Inserting one
    // here would give "Dru c ker".
    let text = extract("actual-text-drucker.pdf");
    assert!(!text.plain_text().contains(' '));
    assert_eq!(text.diagnostics.spaces_derived, 0);
}

// ---------------------------------------------------------------------------
// Artifacts and ReversedChars — §14.8
// ---------------------------------------------------------------------------

#[test]
fn artifacts_are_classified_kept_and_excluded_by_policy() {
    let text = extract("artifact-and-reversed.pdf");
    let d = &text.diagnostics;
    assert_eq!(
        d.artifact_sequences, 2,
        "one with a property list, one bare"
    );
    assert!(d.artifact_chars > 0);

    // Excluded from plain text by DEFAULT policy...
    let plain = text.plain_text();
    assert!(!plain.contains("Running head"));
    assert!(plain.contains("Real content"));

    // ...but always present in the run list, because §14.8.2.2's A1
    // records that no `shall` requires a reader to exclude them.
    let artifact_runs: Vec<_> = text
        .pages
        .iter()
        .flat_map(|p| p.runs.iter())
        .filter(|r| r.artifact.is_some())
        .collect();
    assert!(
        artifact_runs
            .iter()
            .any(|r| r.text.contains("Running head"))
    );
    assert!(
        artifact_runs
            .iter()
            .any(|r| r.artifact == Some(ArtifactKind::Pagination)),
        "Table 330's /Type must be read"
    );
    assert!(
        artifact_runs
            .iter()
            .any(|r| r.artifact == Some(ArtifactKind::Unspecified)),
        "the bare /Artifact BMC form is a generic artifact, not an error"
    );
}

#[test]
fn including_artifacts_is_a_caller_decision() {
    let options = ExtractOptions::default().with_artifacts(true);
    let text = extract_with("artifact-and-reversed.pdf", &options);
    assert!(text.plain_text().contains("Running head"));
    assert!(text.includes_artifacts());
}

#[test]
fn reversed_chars_reverses_within_each_string_not_across_them() {
    // §14.8.2.3.3's own example: "( olleH ) Tj -200 0 Td ( . dlrow ) Tj"
    // "represents the text Hello world .". Reversing the SEQUENCE
    // instead of each STRING is the classic bug and produces the words
    // in the wrong order.
    let text = extract("artifact-and-reversed.pdf");
    let sourced = text.sourced_text();
    assert!(
        sourced.contains("Hello") && sourced.contains("world ."),
        "sourced: {sourced:?}"
    );
    let hello = sourced.find("Hello").expect("Hello present");
    let world = sourced.find("world").expect("world present");
    assert!(
        hello < world,
        "the strings themselves stay in reading order"
    );
    assert_eq!(text.diagnostics.reversed_chars_sequences, 1);
}

// ---------------------------------------------------------------------------
// Document-level facts — §14.8.1, §14.8.2.3.1, §14.7
// ---------------------------------------------------------------------------

#[test]
fn tagged_suspects_and_struct_tree_are_all_reported() {
    let text = extract("tagged-marked.pdf");
    let d = &text.diagnostics;
    assert!(d.tagged);
    assert!(d.suspects);
    assert!(d.struct_tree_present);
    assert_eq!(d.tag_suspect_sequences, 1);

    for expected in ["Suspects", "StructTreeRoot", "TagSuspect"] {
        assert!(
            d.notes.iter().any(|n| n.contains(expected)),
            "missing a named diagnostic for {expected}: {:?}",
            d.notes
        );
    }
    // A tagged document must NOT get the untagged warning.
    assert!(!d.notes.iter().any(|n| n.contains("untagged document")));
}

#[test]
fn mcid_is_recorded_for_a_later_structure_pass() {
    let text = extract("tagged-marked.pdf");
    let run = text
        .pages
        .iter()
        .flat_map(|p| p.runs.iter())
        .find(|r| r.text.contains("Inside MCID"))
        .expect("the MCID-tagged run exists");
    assert_eq!(run.mcid, Some(0));
}

// ---------------------------------------------------------------------------
// API-shape and cross-cutting behaviour
// ---------------------------------------------------------------------------

#[test]
fn every_glyph_text_range_indexes_its_own_run() {
    // A range that is out of bounds, or that does not fall on a char
    // boundary, would panic a caller doing the obvious slice — so this
    // is the invariant the whole per-glyph provenance model rests on.
    for name in [
        "simple-winansi.pdf",
        "identity-h-tounicode.pdf",
        "actual-text-drucker.pdf",
        "artifact-and-reversed.pdf",
        "tagged-marked.pdf",
    ] {
        let text = extract(name);
        for page in &text.pages {
            for run in &page.runs {
                let mut expected_start = 0usize;
                for g in &run.glyphs {
                    let start = g.text_start as usize;
                    let end = start + g.text_len as usize;
                    assert!(end <= run.text.len(), "{name}: range past the run");
                    assert!(run.text.is_char_boundary(start), "{name}: split a char");
                    assert!(run.text.is_char_boundary(end), "{name}: split a char");
                    assert_eq!(start, expected_start, "{name}: glyph ranges must tile");
                    expected_start = end;
                }
                if !run.glyphs.is_empty() {
                    assert_eq!(
                        expected_start,
                        run.text.len(),
                        "{name}: a glyph run's text must be fully covered by its glyphs"
                    );
                }
            }
        }
    }
}

#[test]
fn derived_runs_never_carry_glyphs_and_sourced_runs_always_do() {
    for name in ["simple-winansi.pdf", "actual-text-drucker.pdf"] {
        let text = extract(name);
        for page in &text.pages {
            for run in &page.runs {
                match run.origin {
                    TextOrigin::Glyphs => assert!(!run.glyphs.is_empty(), "{name}"),
                    _ => assert!(run.glyphs.is_empty(), "{name}: {:?}", run.origin),
                }
            }
        }
    }
}

#[test]
fn sourced_text_is_a_subsequence_of_plain_text() {
    // The two accessors differ only by insertions. If sourced_text ever
    // contained a character plain_text does not, pdfce would be dropping
    // file content on the "friendly" path.
    for name in [
        "simple-winansi.pdf",
        "identity-h-tounicode.pdf",
        "identity-h-no-tounicode.pdf",
        "actual-text-drucker.pdf",
        "artifact-and-reversed.pdf",
        "tagged-marked.pdf",
    ] {
        let text = extract(name);
        let plain = text.plain_text();
        let sourced = text.sourced_text();
        let mut haystack = plain.chars();
        for ch in sourced.chars() {
            assert!(
                haystack.any(|h| h == ch),
                "{name}: sourced text is not a subsequence of plain text"
            );
        }
    }
}

#[test]
fn display_matches_plain_text() {
    let text = extract("simple-winansi.pdf");
    assert_eq!(text.to_string(), text.plain_text());
}

#[test]
fn page_and_document_extraction_agree() {
    use pdfce_core::page_tree;

    let doc = Document::load(&fixture("simple-winansi.pdf")).expect("loads");
    let pages = page_tree::pages(&doc).expect("page tree");
    let options = ExtractOptions::default();
    let one = text_extract::extract_page(&doc, &pages[0], 0, &options).expect("page");
    let all = text_extract::extract_document(&doc, &options).expect("document");
    assert_eq!(one.plain_text(), all.pages[0].plain_text());
    assert_eq!(one.diagnostics.codes_total, all.diagnostics.codes_total);
}

#[test]
fn a_page_index_past_the_end_is_a_named_refusal() {
    let doc = Document::load(&fixture("simple-winansi.pdf")).expect("loads");
    let err = text_extract::extract_pages(&doc, &[7], &ExtractOptions::default())
        .expect_err("index 7 does not exist");
    assert!(matches!(
        err,
        text_extract::ExtractError::NoSuchPage { index: 7, count: 1 }
    ));
}

#[test]
fn extraction_leaves_the_document_bytes_untouched() {
    // Extraction is READ-ONLY. This is not a hypothetical: the walk
    // resolves objects, decodes streams and builds caches, and a future
    // change that memoised something into the document would break the
    // round-trip invariant everywhere at once.
    let doc = Document::load(&fixture("identity-h-tounicode.pdf")).expect("loads");
    let before = doc.bytes().to_vec();
    let _ = text_extract::extract_document(&doc, &ExtractOptions::default()).expect("extracts");
    assert_eq!(doc.bytes(), before.as_slice());
}
