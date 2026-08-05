//! # `pdfce-cli` vector-edit integration tests (Pass 9c-min)
//!
//! Black-box tests over the **real binary** for the three basic vector edits
//! (decision 011 §2.5): `object-move`, `object-delete`, `node-move`. They
//! assert the process contract a batch script depends on — exit codes, the
//! stable report line, the `verify_undo` byte-identity flag, and the named
//! refusals — over the committed `fixtures/synthetic/vector/edit.pdf`
//! (three isolated, index-predictable objects: line / rectangle / triangle;
//! provenance in that directory's `PROVENANCE.md`).
//!
//! The strong invariant checked here (the R46/§5.7 named exception) is the
//! `--verify-undo` flag reporting `undo_identical=1`: undoing the edit
//! reproduces the input byte for byte, and the output is a byte-prefix of the
//! input plus one appended revision that names exactly one object.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

const BIN: &str = env!("CARGO_BIN_EXE_pdfce-cli");
/// `exit::EDIT_REFUSED` in the CLI's stable exit-code contract.
const EDIT_REFUSED: i32 = 9;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/synthetic/vector/edit.pdf")
}

fn temp_path(tag: &str) -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("pdfce_vedit_{tag}_{}_{n}.pdf", std::process::id()))
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

#[test]
fn object_move_succeeds_and_undoes_byte_identically() {
    let out_path = temp_path("move");
    let out = run(
        "object-move",
        &[
            fixture().to_str().unwrap(),
            "--page",
            "1",
            "--object",
            "0",
            "--dx=30",
            "--dy=-20",
            "-o",
            out_path.to_str().unwrap(),
            "--verify-undo",
        ],
    );
    assert!(out.status.success(), "{}", stdout(&out));
    let text = stdout(&out);
    assert!(text.contains("object-move"), "{text}");
    assert!(
        text.contains("changed=1"),
        "exactly one object changed: {text}"
    );
    assert!(
        text.contains("undo_identical=1"),
        "undo byte-identical: {text}"
    );

    // The output is a byte-prefix of the input (incremental append) — the
    // R46/§5.7 content-identity property, observable from the bytes.
    let base = std::fs::read(fixture()).unwrap();
    let produced = std::fs::read(&out_path).unwrap();
    assert!(produced.starts_with(&base), "incremental prefix");
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn object_delete_removes_one_object() {
    let out_path = temp_path("del");
    let out = run(
        "object-delete",
        &[
            fixture().to_str().unwrap(),
            "--object",
            "2",
            "-o",
            out_path.to_str().unwrap(),
            "--verify-undo",
        ],
    );
    assert!(out.status.success(), "{}", stdout(&out));
    let text = stdout(&out);
    assert!(text.contains("object-delete"), "{text}");
    assert!(text.contains("changed=1"), "{text}");
    assert!(text.contains("undo_identical=1"), "{text}");
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn node_move_relocates_an_anchor() {
    let out_path = temp_path("node");
    let out = run(
        "node-move",
        &[
            fixture().to_str().unwrap(),
            "--object",
            "0",
            "--node",
            "1",
            "--x",
            "200",
            "--y",
            "100",
            "-o",
            out_path.to_str().unwrap(),
            "--verify-undo",
        ],
    );
    assert!(out.status.success(), "{}", stdout(&out));
    let text = stdout(&out);
    assert!(text.contains("node-move"), "{text}");
    assert!(text.contains("undo_identical=1"), "{text}");
    let _ = std::fs::remove_file(&out_path);
}

/// A rectangle corner drags (Pass 30.0), and the operator is TOLD that the
/// rectangle had to be rewritten as four lines to express it.
///
/// This test previously asserted the refusal. The disclosure half is the part
/// that needed a CLI-level test rather than a core one: the core produces the
/// string, and the only thing that can go wrong from here is the front end
/// dropping it — which is precisely what every caller of these methods did
/// before, because the return type let them.
///
/// Also pins WHICH stream it lands on. The stdout line is a fixed-shape record
/// that scripts parse; a prose block spliced into it would break them, and
/// nothing but a test stops a later edit from moving it there.
#[test]
fn node_move_on_a_rectangle_corner_expands_it_and_says_so() {
    let out_path = temp_path("rect");
    // Object 1 is the `re` rectangle; node 0 is its lower-left corner.
    let out = run(
        "node-move",
        &[
            fixture().to_str().unwrap(),
            "--object",
            "1",
            "--node",
            "0",
            "--x",
            "0",
            "--y",
            "0",
            "-o",
            out_path.to_str().unwrap(),
            "--verify-undo",
        ],
    );
    assert!(out.status.success(), "{}", stdout(&out));
    let text = stdout(&out);
    assert!(text.contains("node-move"), "{text}");
    // Undo restores the single `re` from five operators — a shrink, which a
    // same-length rewrite would never exercise.
    assert!(text.contains("undo_identical=1"), "{text}");

    let err = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(
        err.contains("rectangle"),
        "the operator must be told the rectangle was rewritten: {err:?}"
    );
    assert!(
        !text.contains("rectangle"),
        "the disclosure belongs on stderr; stdout is the machine-readable          record: {text}"
    );
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn an_out_of_range_object_is_refused() {
    let out_path = temp_path("oor");
    let out = run(
        "object-move",
        &[
            fixture().to_str().unwrap(),
            "--object",
            "999",
            "--dx=1",
            "--dy=1",
            "-o",
            out_path.to_str().unwrap(),
        ],
    );
    assert_eq!(out.status.code(), Some(EDIT_REFUSED));
    assert!(!out_path.exists());
}
