//! CLI tests for `find-text` — the first slice of the Reader-parity sweep.
//!
//! # Why this command exists
//!
//! A 2026-08-10 audit against Acrobat Reader found pdfce well ahead on
//! editing and behind on plain consumption. Text search was the starkest
//! case: `pdfce-core` has had the whole scan — extract, match, turn the
//! matched glyph span into a page-space quad — since redaction's
//! search-to-mark shipped, but it was **buried inside a mutating
//! redaction verb** and unreachable on its own. pdfce could find text
//! only as a side effect of marking it for destruction.
//!
//! # The assertion that matters most
//!
//! [`find_and_redaction_search_agree_on_geometry`]. The scan is now
//! shared between `find-text` and `redact-mark --search`, and the reason
//! is not code reuse — it is that two copies of glyph-span-to-quad
//! geometry drift in the worst available direction: **a redaction
//! covering a slightly different box than the search that found it.** An
//! operator searches, sees a hit, marks it, and the mark is not quite
//! where the hit was. Sharing one scanner makes that unrepresentable;
//! this test makes the sharing observable from outside.
//!
//! # The documented limits are tested as limits
//!
//! Two things `find-text` deliberately does not do are asserted here so
//! they stay deliberate: it does not match `/ActualText` runs (they carry
//! no per-glyph geometry, so a hit could be counted but not located), and
//! finding nothing exits `0` (a search that matched nothing succeeded —
//! a non-zero exit would make "no hits" indistinguishable from "could not
//! read the file" in a pipeline).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_pdfce-cli");

fn fixture(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/synthetic")
        .join(rel)
}

fn run(args: &[&str]) -> Output {
    Command::new(BIN)
        .args(args)
        .output()
        .expect("could not spawn pdfce-cli")
}

fn code(out: &Output) -> u8 {
    u8::try_from(out.status.code().expect("process was killed by a signal")).unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).expect("stdout must be valid UTF-8")
}

/// A hit is reported with its page and its box, not merely counted.
///
/// "page 1" is not an answer when a word appears six times on it. The
/// rectangle is what lets a caller draw a box, crop an image, or hand a
/// coordinate to another command.
#[test]
fn a_hit_reports_where_it_is() {
    let f = fixture("text/composite-editable.pdf");
    let out = run(&["find-text", &f.display().to_string(), "--needle", "ABC"]);
    assert_eq!(code(&out), 0);
    let s = stdout(&out);

    assert!(s.contains("matches=1"), "{s}");
    let hit = s
        .lines()
        .find(|l| l.starts_with("match "))
        .unwrap_or_else(|| panic!("no match line in:\n{s}"));
    assert!(hit.contains("page=1"), "1-based page: {hit}");
    assert!(
        hit.contains("rect=72.00,589.44,158.40,640.80"),
        "the on-page box must be reported: {hit}"
    );
}

/// A case-insensitive hit reports the text as the DOCUMENT spells it.
///
/// Searching `abc` and being told the document contains `abc` would be a
/// small lie with a real cost: an operator reviewing hits before redacting
/// needs to see which casing they actually matched.
#[test]
fn a_case_insensitive_hit_reports_the_documents_own_spelling() {
    let f = fixture("text/composite-editable.pdf");
    let out = run(&[
        "find-text",
        &f.display().to_string(),
        "--needle",
        "abc",
        "--ignore-case",
    ]);
    assert_eq!(code(&out), 0);
    let s = stdout(&out);
    assert!(s.contains("matches=1"), "{s}");
    assert!(
        s.contains(r#"text="ABC""#),
        "the matched text must be the document's, not the needle's:\n{s}"
    );
}

/// ★ `find-text` and `redact-mark --search` produce the SAME box.
///
/// Both come from one scan in core. This asserts it from outside, against
/// the bytes `redact-mark` actually wrote, so the sharing is observable
/// rather than a claim in a doc comment — and so that splitting the
/// scanner back into two copies has to break a test.
///
/// The `/QuadPoints` array is compared, not `/Rect`: the annotation's
/// rect carries a small margin around the covered area, which is a
/// separate and deliberate difference. Comparing rects would fail for a
/// reason that has nothing to do with the property under test.
#[test]
fn find_and_redaction_search_agree_on_geometry() {
    let f = fixture("text/composite-editable.pdf");

    let found = stdout(&run(&[
        "find-text",
        &f.display().to_string(),
        "--needle",
        "ABC",
    ]));
    assert!(found.contains("matches=1"), "{found}");

    let dir = std::env::temp_dir().join(format!("pdfce-findtext-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let marked = dir.join("marked.pdf");
    let out = run(&[
        "redact-mark",
        &f.display().to_string(),
        "--search",
        "ABC",
        "-o",
        &marked.display().to_string(),
    ]);
    assert_eq!(code(&out), 0, "redact-mark failed");

    let bytes = std::fs::read(&marked).expect("read the marked file");
    let text = String::from_utf8_lossy(&bytes);
    let quad = text
        .split("QuadPoints [")
        .nth(1)
        .and_then(|s| s.split(']').next())
        .unwrap_or_else(|| panic!("no /QuadPoints in the marked file"));

    // The four corner pairs, as the writer rounded them.
    let nums: Vec<f64> = quad
        .split_whitespace()
        .filter_map(|t| t.parse::<f64>().ok())
        .collect();
    assert_eq!(nums.len(), 8, "a quad is four points: {quad:?}");
    let xs = [nums[0], nums[2], nums[4], nums[6]];
    let ys = [nums[1], nums[3], nums[5], nums[7]];
    let lo = |v: [f64; 4]| v.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = |v: [f64; 4]| v.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    let expect = format!(
        "rect={:.2},{:.2},{:.2},{:.2}",
        lo(xs),
        lo(ys),
        hi(xs),
        hi(ys)
    );
    assert!(
        found.contains(&expect),
        "find-text and redact-mark must report the same geometry.\n\
         redact-mark wrote {expect}\nfind-text said:\n{found}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// An `/ActualText` run is NOT matched, and that is deliberate.
///
/// Those runs carry a replacement string with no per-glyph geometry, so a
/// match inside one could be counted but not located — and a hit the
/// caller cannot point at is worse than no hit, because it invites a
/// redaction that has nowhere to go.
///
/// `actual-text-drucker.pdf` extracts as "Druc"/"ker" through
/// `/ActualText`, so searching its visible text finds nothing. Asserted
/// so the limit stays a decision rather than becoming a bug report.
#[test]
fn actual_text_runs_are_not_matched() {
    let f = fixture("text/actual-text-drucker.pdf");
    let out = run(&["find-text", &f.display().to_string(), "--needle", "Druc"]);
    assert_eq!(code(&out), 0);
    assert!(
        stdout(&out).contains("matches=0"),
        "an /ActualText run has no glyph geometry to point at:\n{}",
        stdout(&out)
    );
}

/// Finding nothing is a SUCCESSFUL search.
///
/// A non-zero exit would make "no hits" indistinguishable from "could not
/// read the file" in a shell pipeline. The count is on the summary line
/// for a caller that wants to branch on it.
#[test]
fn no_matches_still_exits_zero() {
    let f = fixture("text/composite-editable.pdf");
    let out = run(&[
        "find-text",
        &f.display().to_string(),
        "--needle",
        "zzzznotpresent",
    ]);
    assert_eq!(code(&out), 0, "an empty result is not a failure");
    assert!(stdout(&out).contains("matches=0"));
}
