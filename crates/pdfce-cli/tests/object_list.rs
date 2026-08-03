//! # `pdfce-cli object-list` integration tests
//!
//! Black-box tests over the **real binary** for `object-list` — the
//! paint-order object inventory that is the discovery path for the
//! `--object` index `object-move` / `object-delete` / `node-move` consume,
//! and for the `--hit` headless hit-test query.
//!
//! ## What these tests are actually protecting
//!
//! Three distinct contracts, none of which is cosmetic:
//!
//! 1. **The index correspondence.** `object-list`'s `index=` and
//!    `object-move --object` must name the same object. They do because
//!    both come from one `pdfce_core::vector::decompose_page` walk, but
//!    "because the implementation currently shares a function" is not a
//!    guarantee — [`listed_indices_are_the_ones_object_move_consumes`]
//!    pins it observably, by listing an object and then editing *that*
//!    index and checking exactly one object changed. If a future change
//!    ever inserted a filter on one side and not the other, this fails.
//!
//! 2. **The hit-test oracle.** `--hit` calls the same
//!    `pdfce_core::vector::hit_test_point` the GUI's
//!    `ObjectModelProvider::hit_test` calls, which makes this subcommand
//!    the headless authority on GUI selection behaviour.
//!    [`a_click_on_a_stroked_line_selects_it`] is the regression for the
//!    concrete diagnosis this was built to settle: on
//!    `fixtures/synthetic/dimension/linear-base.pdf` — a page whose only
//!    content is one 1 pt horizontal stroked line, a *zero-height* bounding
//!    box — a click on the line hits at every sane tolerance, INCLUDING
//!    `--tolerance 0` (the stroke's own half-width carries it). A
//!    degenerate-bbox path being unhittable would be a real core bug and
//!    this is what would catch it.
//!
//! 3. **Clean refusals.** An out-of-range page, a `--page 0`, and a
//!    malformed `--hit` operand must each produce a named error and a
//!    non-zero exit, never a panic and never a confident wrong answer
//!    (rule 4: fuzzy, never sneaky).
//!
//! Fixtures used (provenance in each directory's `PROVENANCE.md`):
//! `fixtures/synthetic/vector/edit.pdf` (line / filled rectangle /
//! stroked triangle — three index-predictable path objects),
//! `fixtures/synthetic/text/simple-winansi.pdf` (one text object), and
//! `fixtures/synthetic/dimension/linear-base.pdf` (one stroked line).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

const BIN: &str = env!("CARGO_BIN_EXE_pdfce-cli");
/// `exit::RUNTIME_ERROR` in the CLI's stable exit-code contract — what a
/// bad `--page` / `--hit` operand yields.
const RUNTIME_ERROR: i32 = 1;

fn fixture(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/synthetic")
        .join(rel)
}

fn temp_path(tag: &str) -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "pdfce_objlist_{tag}_{}_{n}.pdf",
        std::process::id()
    ))
}

fn run(sub: &str, args: &[&str]) -> Output {
    Command::new(BIN)
        .arg(sub)
        .args(args)
        .output()
        .expect("the binary runs")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// The `hit …` line's fields, for the tests that only care about the query
/// answer. Panics if the run did not emit one.
fn hit_line(out: &Output) -> String {
    stdout(out)
        .lines()
        .find(|l| l.starts_with("hit "))
        .unwrap_or_else(|| panic!("no `hit` line in output:\n{}", stdout(out)))
        .to_owned()
}

#[test]
fn lists_path_objects_in_paint_order_with_geometry() {
    let f = fixture("vector/edit.pdf");
    let out = run("object-list", &[f.to_str().unwrap(), "--page", "1"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    let rows: Vec<&str> = text.lines().filter(|l| l.starts_with("object ")).collect();
    assert_eq!(rows.len(), 3, "three path objects: {text}");

    // Paint order: index 0 painted first, so indices ascend down the
    // listing and the LAST row is topmost.
    for (i, row) in rows.iter().enumerate() {
        assert!(row.contains(&format!("index={i}")), "row {i}: {row}");
        assert!(row.contains("kind=path"), "row {i}: {row}");
    }

    // Object 0 is the open stroked line: a stroke-only path with two
    // anchors and no closed subpath.
    assert!(
        rows[0].contains("bbox=50,50,150,150")
            && rows[0].contains("subpaths=1")
            && rows[0].contains("anchors=2")
            && rows[0].contains("closed=0")
            && rows[0].contains("paint=stroke"),
        "line: {}",
        rows[0]
    );
    // Object 1 is the `re` rectangle: filled (nonzero), four anchors, one
    // closed subpath. The `paint=` token is what explains a hit-test
    // result — a filled path is hit by its interior, a stroke-only path
    // only near its outline.
    assert!(
        rows[1].contains("bbox=200,50,280,110")
            && rows[1].contains("anchors=4")
            && rows[1].contains("closed=1")
            && rows[1].contains("paint=fill-nonzero"),
        "rectangle: {}",
        rows[1]
    );
    // Object 2 is the closed stroked triangle.
    assert!(
        rows[2].contains("anchors=3") && rows[2].contains("closed=1"),
        "triangle: {}",
        rows[2]
    );

    // The summary line tallies what was listed, and discloses whether the
    // decomposition dropped anything (a `MAX_OBJECTS`/`MAX_NODES` cap would
    // otherwise silently shift every index past the drop).
    assert!(
        text.contains("objects=3 paths=3 text=0 images=0 forms=0"),
        "summary: {text}"
    );
    assert!(
        text.contains("dropped_objects=0 dropped_nodes=0"),
        "summary: {text}"
    );
}

/// The whole reason this subcommand exists: the `index=` it prints IS the
/// `--object` the editing subcommands take.
///
/// Proven observably rather than by inspection — list the objects, then
/// move the LAST listed index and require exactly one object to change and
/// the edit to undo byte-identically. One past that index must be refused,
/// which pins the count as well as the base.
#[test]
fn listed_indices_are_the_ones_object_move_consumes() {
    let f = fixture("vector/edit.pdf");
    let listed = run("object-list", &[f.to_str().unwrap()]);
    assert!(listed.status.success(), "{}", stderr(&listed));
    let count = stdout(&listed)
        .lines()
        .filter(|l| l.starts_with("object "))
        .count();
    assert_eq!(count, 3);

    // The highest listed index is editable...
    let out_path = temp_path("corr");
    let moved = run(
        "object-move",
        &[
            f.to_str().unwrap(),
            "--object",
            &(count - 1).to_string(),
            "--dx=5",
            "--dy=5",
            "-o",
            out_path.to_str().unwrap(),
            "--verify-undo",
        ],
    );
    assert!(moved.status.success(), "{}", stderr(&moved));
    let text = stdout(&moved);
    assert!(text.contains("changed=1"), "{text}");
    assert!(text.contains("undo_identical=1"), "{text}");
    let _ = std::fs::remove_file(&out_path);

    // ...and one past it is refused, so the listing's count is the real
    // addressable range, not a prefix of it.
    let past = temp_path("past");
    let refused = run(
        "object-move",
        &[
            f.to_str().unwrap(),
            "--object",
            &count.to_string(),
            "--dx=5",
            "--dy=5",
            "-o",
            past.to_str().unwrap(),
        ],
    );
    assert!(
        !refused.status.success(),
        "index == count must be refused: {}",
        stdout(&refused)
    );
    assert!(!past.exists());
}

#[test]
fn lists_a_text_object_with_its_bbox() {
    let f = fixture("text/simple-winansi.pdf");
    let out = run("object-list", &[f.to_str().unwrap()]);
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("kind=text"), "{text}");
    // A text object is bbox-only (it carries no editable node geometry), so
    // the row is bbox + the `approximate=` disclosure: 1 means the box was
    // estimated from the positioning operators rather than glyph metrics,
    // which is why a click can land slightly off a glyph.
    assert!(text.contains("bbox=48,646,96,724"), "{text}");
    assert!(text.contains("approximate=1"), "{text}");
    assert!(text.contains("objects=1 paths=0 text=1"), "{text}");
}

/// A page past the end, and `--page 0`, are both clean named refusals — a
/// non-zero exit and a message naming the valid range, never a panic and
/// never an empty "success" listing that would read as "this page has no
/// objects".
#[test]
fn an_out_of_range_page_is_refused_cleanly() {
    let f = fixture("dimension/linear-base.pdf");
    for bad in ["7", "0"] {
        let out = run("object-list", &[f.to_str().unwrap(), "--page", bad]);
        assert_eq!(
            out.status.code(),
            Some(RUNTIME_ERROR),
            "--page {bad}: {}",
            stderr(&out)
        );
        let err = stderr(&out);
        assert!(err.contains("out of range"), "--page {bad}: {err}");
        assert!(err.contains("1 page(s)"), "--page {bad}: {err}");
        // A refusal lists nothing — no partial output to misread.
        assert!(
            !stdout(&out).contains("object page="),
            "--page {bad} printed rows"
        );
        // Not a panic.
        assert!(!err.contains("panicked"), "--page {bad}: {err}");
    }
}

/// The diagnosis regression (module docs §2). A page whose only content is
/// one stroked horizontal line — a path with a **zero-height** bounding box
/// — is hit by a click on it at every sane tolerance, and at `--tolerance 0`
/// too, because the stroke's own half-width (1 pt line ⇒ 0.5 pt each side)
/// is the hittable band. If this ever fails, hit-testing genuinely broke
/// for degenerate-bbox stroked geometry.
#[test]
fn a_click_on_a_stroked_line_selects_it() {
    let f = fixture("dimension/linear-base.pdf");
    // The bbox this same subcommand reports is 100,200..300,200; (200,200)
    // is its midpoint, dead on the line.
    for tol in ["0", "0.5", "3", "6"] {
        let out = run(
            "object-list",
            &[f.to_str().unwrap(), "--hit", "200,200", "--tolerance", tol],
        );
        assert!(out.status.success(), "tol {tol}: {}", stderr(&out));
        assert!(
            hit_line(&out).contains("index=0 kind=path"),
            "tol {tol}: a click ON the line must select it: {}",
            hit_line(&out)
        );
    }

    // 3 pt above the line: outside the 0.5 pt stroke band at a tight
    // tolerance, inside it at a forgiving one. This is the tolerance
    // actually doing something, not a constant that happens to pass.
    let tight = run(
        "object-list",
        &[
            f.to_str().unwrap(),
            "--hit",
            "200,203",
            "--tolerance",
            "0.5",
        ],
    );
    assert!(
        hit_line(&tight).contains("index=none"),
        "{}",
        hit_line(&tight)
    );
    let loose = run(
        "object-list",
        &[f.to_str().unwrap(), "--hit", "200,203", "--tolerance", "6"],
    );
    assert!(hit_line(&loose).contains("index=0"), "{}", hit_line(&loose));
}

/// A miss is an ANSWER, not an error: exit 0, with `index=none` as the
/// machine-readable field. A script asking "what is under this point?"
/// must be able to distinguish "nothing is there" from "the query failed",
/// and only the former is a success.
#[test]
fn a_hit_that_misses_reports_none_and_still_succeeds() {
    let f = fixture("dimension/linear-base.pdf");
    let out = run(
        "object-list",
        &[f.to_str().unwrap(), "--hit", "200,300", "--tolerance", "6"],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    let line = hit_line(&out);
    assert!(line.contains("index=none"), "{line}");
    assert!(line.contains("kind=none"), "{line}");
    // The inventory is still printed — a miss does not suppress the listing
    // that would tell the operator what IS on the page.
    assert!(
        stdout(&out).contains("object page=1 index=0"),
        "{}",
        stdout(&out)
    );
}

/// A nonsense `--tolerance` is refused rather than silently turning every
/// query into a miss.
///
/// This matters more than it looks: clap parses `--tolerance` as a bare
/// `f64`, so `nan` and negatives both arrive intact, and both would make
/// `hit_test_point` reject everything. An operator would read the resulting
/// `index=none` as "nothing is there" — a confident wrong answer, which is
/// exactly what rule 4 forbids. The refusal must also come BEFORE any
/// `object` row is printed, so a failed run never leaves half an answer on
/// stdout for a script to parse.
#[test]
fn a_nonsense_tolerance_is_refused_before_anything_is_printed() {
    let f = fixture("dimension/linear-base.pdf");
    for bad in ["-1", "nan"] {
        let out = run(
            "object-list",
            &[f.to_str().unwrap(), "--hit", "200,200", "--tolerance", bad],
        );
        assert_eq!(
            out.status.code(),
            Some(RUNTIME_ERROR),
            "--tolerance {bad}: {}",
            stdout(&out)
        );
        assert!(
            stderr(&out).contains("--tolerance must be"),
            "--tolerance {bad}: {}",
            stderr(&out)
        );
        assert!(
            stdout(&out).is_empty(),
            "--tolerance {bad} printed a partial answer: {}",
            stdout(&out)
        );
    }

    // Without `--hit` the tolerance is unused, so it is not policed — a
    // stray value must not break a plain inventory.
    let listing = run("object-list", &[f.to_str().unwrap(), "--tolerance", "-1"]);
    assert!(listing.status.success(), "{}", stderr(&listing));
}

/// Topmost wins: where two objects overlap, `--hit` reports the
/// LAST-painted one, matching the selection convention the GUI applies.
#[test]
fn a_hit_reports_the_topmost_object_only() {
    let f = fixture("vector/edit.pdf");
    // Inside the filled rectangle (bbox 200,50..280,110), which nothing
    // else covers — the single unambiguous case; the ordering guarantee
    // itself is unit-tested in core's `hit.rs`.
    let out = run("object-list", &[f.to_str().unwrap(), "--hit", "240,80"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(hit_line(&out).contains("index=1"), "{}", hit_line(&out));
}

/// A malformed coordinate is refused by name rather than silently parsed
/// into a confident wrong answer about what a click selects.
#[test]
fn a_malformed_hit_operand_is_refused() {
    let f = fixture("dimension/linear-base.pdf");
    for bad in ["oops", "200", "200,", "200,abc", "nan,0"] {
        let out = run("object-list", &[f.to_str().unwrap(), "--hit", bad]);
        assert_eq!(
            out.status.code(),
            Some(RUNTIME_ERROR),
            "--hit {bad} should be refused: {}",
            stdout(&out)
        );
        assert!(
            stderr(&out).contains("malformed --hit"),
            "--hit {bad}: {}",
            stderr(&out)
        );
    }
}
