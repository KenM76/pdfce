//! # Clause-11 transparency is not implemented — and now it SAYS so
//!
//! `/BM` (blend mode, ISO 32000-1 §11.3.5) and `/SMask` (soft mask, §11.6.5)
//! in an `ExtGState` are not implemented by `pdfce-render`. That is a known
//! gap. What was NOT acceptable is that `apply_ext_gstate` read `LW`, `LC`,
//! `LJ`, `ML`, `D`, `ca` and `CA` and silently dropped the rest, so a page
//! asking for a Multiply blend or a soft mask rendered as though it had
//! asked for neither and **nothing on the result line said a word about
//! it**. Project rule 4 forbids exactly that: an inference or a shortfall
//! the operator cannot see must still be reported.
//!
//! ## Why the two counters are separate, which is the whole design
//!
//! Their failure directions are opposite, and only one of them can expose
//! content:
//!
//! - An ignored **blend mode** composites the same marks by the wrong rule.
//!   The page is not blank where they are, it is *wrong* there — and a
//!   Multiply that composited as Normal looks like a perfectly ordinary
//!   opaque overlay. Nobody notices.
//! - An ignored **soft mask** paints marks the document asked to be faded
//!   or masked away, so it paints MORE than was asked for. On a page whose
//!   design relies on a mask to hide something, that is the difference
//!   between a rendering artefact and showing what was meant to be hidden.
//!
//! Folding them into one number would make the second indistinguishable
//! from the first at exactly the moment the distinction matters.
//!
//! ## What this gap actually costs, measured
//!
//! On the operator's Ghent PDF Output Suite 5.0 X-4 file, 2026-08-17:
//! **113 ignored blend modes and 36 ignored soft masks across six pages**,
//! with page 2 alone accounting for 76 and 31. Page 2 had previously
//! reported no unsupported images, no unpainted patterns and no refused
//! shadings — it looked *clean*, and it was compositing wrongly the whole
//! time. That measurement was impossible to take before these counters
//! existed, which is the argument for disclosing a gap before implementing
//! it: it re-ordered the render queue.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use pdfce_core::document::Document;
use pdfce_core::page_tree;
use pdfce_render::{RenderOptions, RenderedPage, render_page_with};

/// Assemble a classic single-page PDF with a correct xref table.
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

/// A page whose `/GS0` is `gs_dict`, painting a filled square through it.
fn page(gs_dict: &str) -> Vec<u8> {
    let content = "/GS0 gs 1 0 0 rg 10 10 40 40 re f";
    let stream = format!("{content}\n");
    build(&[
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (
            2,
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 60 60] >>",
        ),
        (
            3,
            &format!(
                "<< /Type /Page /Parent 2 0 R /Contents 4 0 R \
                 /Resources << /ExtGState << /GS0 {gs_dict} >> >> >>"
            ),
        ),
        (
            4,
            &format!("<< /Length {} >>\nstream\n{stream}endstream", stream.len()),
        ),
    ])
}

fn render(bytes: Vec<u8>) -> RenderedPage {
    let doc = Document::from_bytes(bytes).expect("fixture parses");
    let p = page_tree::pages(&doc).expect("page tree").remove(0);
    render_page_with(&doc, &p, 1.0, &RenderOptions::default()).expect("render")
}

#[test]
fn a_non_normal_blend_mode_is_counted() {
    let r = render(page("<< /Type /ExtGState /BM /Multiply >>"));
    assert_eq!(r.diagnostics.blend_modes_ignored, 1);
    assert_eq!(r.diagnostics.soft_masks_ignored, 0);
}

/// Table 58 allows `/BM` to be an ARRAY — "the first blend mode in the
/// array that the conforming reader supports". Reading only the name form
/// would miss every producer that writes the array, and miss it silently.
#[test]
fn a_blend_mode_given_as_an_array_is_counted() {
    let r = render(page("<< /Type /ExtGState /BM [/Darken /Normal] >>"));
    assert_eq!(r.diagnostics.blend_modes_ignored, 1);
}

/// `/BM /Normal` is what pdfce actually does, so it is NOT a shortfall.
/// Counting it would put a large number on ordinary documents and train
/// every reader to ignore the counter — which is how a real signal gets
/// lost inside a true one.
#[test]
fn normal_and_compatible_blend_modes_are_not_counted() {
    assert_eq!(
        render(page("<< /Type /ExtGState /BM /Normal >>"))
            .diagnostics
            .blend_modes_ignored,
        0
    );
    assert_eq!(
        render(page("<< /Type /ExtGState /BM /Compatible >>"))
            .diagnostics
            .blend_modes_ignored,
        0
    );
}

#[test]
fn a_soft_mask_is_counted() {
    let r = render(page(
        "<< /Type /ExtGState /SMask << /S /Alpha /G 9 0 R >> >>",
    ));
    assert_eq!(r.diagnostics.soft_masks_ignored, 1);
    assert_eq!(r.diagnostics.blend_modes_ignored, 0);
}

/// `/SMask /None` is the RESET — it turns a soft mask OFF, which is
/// precisely pdfce's behaviour already. Counting it would report a
/// shortfall on a page that asked for nothing pdfce cannot do, and
/// producers emit `/SMask /None` constantly to clear inherited state.
#[test]
fn smask_none_is_not_counted() {
    let r = render(page("<< /Type /ExtGState /SMask /None >>"));
    assert_eq!(r.diagnostics.soft_masks_ignored, 0);
}

/// Both at once, and the marks still land: the disclosure is about how
/// they were composited, not about whether anything was drawn. A test that
/// only checked the counters would pass on a page that drew nothing.
#[test]
fn the_marks_are_still_painted_while_the_gap_is_disclosed() {
    let r = render(page(
        "<< /Type /ExtGState /BM /Screen /SMask << /S /Luminosity /G 9 0 R >> >>",
    ));
    assert_eq!(r.diagnostics.blend_modes_ignored, 1);
    assert_eq!(r.diagnostics.soft_masks_ignored, 1);
    let p = r.pixmap.pixel(30, 30).expect("in bounds").demultiply();
    assert!(
        p.red() > 200 && p.green() < 55 && p.blue() < 55,
        "the red square is still painted, got ({}, {}, {})",
        p.red(),
        p.green(),
        p.blue()
    );
}
