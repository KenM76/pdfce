//! # Clause-11 transparency: what composites, what does not, and what SAYS so
//!
//! **This file's premise changed one commit after it was written**, and it
//! is re-pointed rather than rewritten so the change stays legible. It was
//! called `transparency_is_disclosed` because NEITHER `/BM` (§11.3.5) nor
//! `/SMask` (§11.6.5) was implemented and the point was that pdfce at least
//! said so. Blend modes now composite for real. Soft masks still do not.
//!
//! So the subjects here are now:
//!
//! - **Soft masks** — still unimplemented, still disclosed. Unchanged.
//! - **Unrecognised blend-mode names** — a name outside Tables 136/137 is
//!   composited as `Normal` and counted. Also unchanged in spirit; what
//!   changed is that "unrecognised" is now a much smaller set than
//!   "non-Normal".
//! - **The census/shortfall split** — `blend_modes_applied` counts modes
//!   pdfce honoured, `blend_modes_ignored` counts names it did not know.
//!   Those were ONE counter an hour ago, and merging them again would make
//!   a real shortfall invisible inside an ordinary census.
//!
//! The numeric verification that pdfce's blend modes match ISO 32000-1
//! Tables 136 and 137 lives in `blend_modes.rs`, not here. This file is
//! about DISCLOSURE; that one is about arithmetic.
//!
//! ## Why the two counters stay separate, which is the design
//!
//! Their failure directions are opposite and only one can expose content:
//!
//! - An ignored **blend mode** composites the same marks by the wrong
//!   rule. The page is not blank there, it is *wrong* there — and a
//!   Multiply that composited as Normal looks like a perfectly ordinary
//!   opaque overlay. Nobody notices.
//! - An ignored **soft mask** paints marks the document asked to be faded
//!   or masked away, so it paints MORE than was asked for. On a page whose
//!   design relies on a mask to hide something, that is the difference
//!   between a rendering artefact and showing what was meant to be hidden.
//!
//! ## What this gap actually costs, measured
//!
//! On the operator's Ghent PDF Output Suite 5.0 X-4 file, 2026-08-17:
//! **113 blend modes and 36 soft masks across six pages**, with page 2
//! alone accounting for 76 and 31. Page 2 had previously reported no
//! unsupported images, no unpainted patterns and no refused shadings — it
//! looked *clean*, and it was compositing wrongly the whole time. That
//! measurement was impossible to take before these counters existed, which
//! is the argument for disclosing a gap before implementing it: it
//! re-ordered the render queue.

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

/// A mode pdfce implements is counted as APPLIED, not as ignored. Getting
/// this backwards would report a shortfall on a page pdfce rendered
/// correctly, which is the fastest way to make a diagnostic worthless.
#[test]
fn an_implemented_blend_mode_is_counted_as_applied() {
    let r = render(page("<< /Type /ExtGState /BM /Multiply >>"));
    assert_eq!(r.diagnostics.blend_modes_applied, 1);
    assert_eq!(r.diagnostics.blend_modes_ignored, 0);
    assert_eq!(r.diagnostics.soft_masks_ignored, 0);
}

/// A name pdfce does not apply — the shortfall case. It composites as
/// `Normal` and says so; it does NOT refuse to paint, because the marks
/// belong on the page and only the compositing rule is in doubt.
#[test]
fn an_unrecognised_blend_mode_name_is_counted_as_ignored() {
    let r = render(page("<< /Type /ExtGState /BM /NotARealMode >>"));
    assert_eq!(r.diagnostics.blend_modes_ignored, 1);
    assert_eq!(r.diagnostics.blend_modes_applied, 0);
    // The marks still landed.
    let p = r.pixmap.pixel(30, 30).expect("in bounds").demultiply();
    assert!(p.red() > 200, "an unknown /BM must not suppress the paint");
}

/// Table 58 allows `/BM` to be an ARRAY — "the first blend mode in the
/// array that the conforming reader supports". Reading only the name form
/// would miss every producer that writes the array, and miss it silently.
#[test]
fn a_blend_mode_given_as_an_array_is_honoured() {
    let r = render(page("<< /Type /ExtGState /BM [/Darken /Normal] >>"));
    assert_eq!(r.diagnostics.blend_modes_applied, 1);
    assert_eq!(r.diagnostics.blend_modes_ignored, 0);
}

/// `Normal` and `Compatible` are what pdfce does anyway, so NEITHER
/// counter moves. Counting them in the census would put a large number on
/// ordinary documents — producers emit `/BM /Normal` constantly to reset
/// inherited state — and train every reader to ignore the counter, which is
/// how a real signal gets lost inside a true one.
#[test]
fn normal_and_compatible_move_neither_counter() {
    for gs in [
        "<< /Type /ExtGState /BM /Normal >>",
        "<< /Type /ExtGState /BM /Compatible >>",
    ] {
        let d = render(page(gs)).diagnostics;
        assert_eq!(d.blend_modes_applied, 0, "{gs}");
        assert_eq!(d.blend_modes_ignored, 0, "{gs}");
    }
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
/// Both gaps at once, and the marks still land: the disclosure is about
/// HOW they were composited, not about whether anything was drawn. A test
/// that only checked the counters would pass on a page that drew nothing.
///
/// `Multiply` is used rather than `Screen` deliberately. The first version
/// of this test used `/BM /Screen` and asserted the square was still RED —
/// which passed only because blend modes were being ignored. Screen of red
/// over a white page is WHITE, correctly, so implementing the feature broke
/// the assertion. Multiply of red over white is red, so the assertion
/// survives for the reason it was written (the paint happened) rather than
/// for the reason it originally passed (the blend did not).
#[test]
fn the_marks_are_still_painted_while_the_soft_mask_gap_is_disclosed() {
    let r = render(page(
        "<< /Type /ExtGState /BM /Multiply /SMask << /S /Luminosity /G 9 0 R >> >>",
    ));
    assert_eq!(r.diagnostics.blend_modes_applied, 1);
    assert_eq!(r.diagnostics.soft_masks_ignored, 1);
    let p = r.pixmap.pixel(30, 30).expect("in bounds").demultiply();
    assert!(
        p.red() > 200 && p.green() < 55 && p.blue() < 55,
        "Multiply of red over a white page is red, got ({}, {}, {})",
        p.red(),
        p.green(),
        p.blue()
    );
}

/// **The blend actually changes pixels**, which no counter can tell you —
/// and it blends against the RIGHT backdrop, which is the harder half.
///
/// ★ The first version of this test asserted that `Screen` over the page
/// came out WHITE, and it passed. It was asserting a BUG. §11.4.7 makes the
/// page an *isolated* transparency group whose initial backdrop is fully
/// TRANSPARENT — white is composited once at the end — and §11.4.5 says
/// blend modes inside a group "shall not be influenced by the group's
/// backdrop". pdfce was filling the buffer opaque white and handing every
/// blend function `cb = 1.0`, which is harmless only for the four modes
/// satisfying `B(1.0, cs) = cs` (`Normal`, `Compatible`, `Multiply`,
/// `Darken`) and wrong for the other eleven.
///
/// So the honest proof needs a real backdrop object underneath. Blue, then
/// red screened over it: `Screen(cb, cs) = cb + cs − cb·cs`, componentwise
/// `(0,0,1)` with `(1,0,0)` gives `(1,0,1)` — magenta. That value can only
/// arise if the blend ran AND saw blue rather than white.
#[test]
fn screen_blends_against_the_object_beneath_not_against_the_paper() {
    // Blue square painted Normal, then a red square screened over it.
    let content = "0 0 1 rg 10 10 40 40 re f /GS0 gs 1 0 0 rg 10 10 40 40 re f";
    let stream = format!(
        "{content}
"
    );
    let bytes = build(&[
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (
            2,
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 60 60] >>",
        ),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /Contents 4 0 R /Resources              << /ExtGState << /GS0 << /Type /ExtGState /BM /Screen >> >> >> >>",
        ),
        (
            4,
            &format!(
                "<< /Length {} >>
stream
{stream}endstream",
                stream.len()
            ),
        ),
    ]);
    let r = render(bytes);
    let p = r.pixmap.pixel(30, 30).expect("in bounds").demultiply();
    assert!(
        p.red() > 250 && p.green() < 5 && p.blue() > 250,
        "Screen of red over blue is magenta, got ({}, {}, {}) — (255,0,0) means the blend never ran, (255,255,255) means it ran against the paper instead of against the blue square",
        p.red(),
        p.green(),
        p.blue()
    );
}

/// The page group's initial backdrop is TRANSPARENT (§11.4.7), so the FIRST
/// object painted at a pixel is unblended whatever the mode says — there is
/// nothing to blend with yet. Pinned because it is the exact behaviour the
/// old white-fill got wrong, and because it looks like a bug until you read
/// the clause.
#[test]
fn the_first_object_at_a_pixel_is_unblended_because_the_page_starts_transparent() {
    let r = render(page("<< /Type /ExtGState /BM /Screen >>"));
    let p = r.pixmap.pixel(30, 30).expect("in bounds").demultiply();
    assert!(
        p.red() > 250 && p.green() < 5 && p.blue() < 5,
        "an isolated group's first object survives its own blend mode, got ({}, {}, {}) — white here means the buffer was pre-filled",
        p.red(),
        p.green(),
        p.blue()
    );
    // And the paper still arrives: outside the square is white, not
    // transparent, because the group is flattened over white at the end.
    let bg = r.pixmap.pixel(2, 2).expect("in bounds").demultiply();
    assert_eq!(
        (bg.red(), bg.green(), bg.blue(), bg.alpha()),
        (255, 255, 255, 255),
        "uncovered page must be opaque white after flattening"
    );
}

// -- §11.4.7 transparency groups -------------------------------------------

/// A page invoking form `/Fm0`, whose stream dictionary carries `extra`.
fn page_with_form(extra: &str) -> Vec<u8> {
    let content = "/Fm0 Do";
    let stream = format!("{content}\n");
    let form_body = "1 0 0 rg 10 10 40 40 re f";
    let form = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 60 60] {extra} /Length {} >>\n\
         stream\n{form_body}\nendstream",
        form_body.len()
    );
    build(&[
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (
            2,
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 60 60] >>",
        ),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /Contents 4 0 R \
             /Resources << /XObject << /Fm0 5 0 R >> >> >>",
        ),
        (
            4,
            &format!("<< /Length {} >>\nstream\n{stream}endstream", stream.len()),
        ),
        (5, &form),
    ])
}

/// **The finding these counters exist for, and it is now fixed.** A form
/// carrying `/Group << /S /Transparency >>` is a COMPOSITING SCOPE
/// (§11.4.7, Table 96): its contents belong in their own buffer, whose
/// RESULT is then composited with the blend mode, constant alpha and soft
/// mask in force at the `Do` (§11.4.5).
///
/// pdfce used to paint the contents straight onto the page, applying those
/// to each object INSIDE instead. That was invisible until it was counted,
/// and the way it surfaced is the whole argument for counting a gap before
/// closing it: the Ghent X-4 file's blend-mode panel still showed the
/// suite's failure crosses AFTER blend modes were implemented and verified
/// correct both in isolation and against a coloured backdrop, while every
/// blend-mode counter looked healthy. The page carried 148 form XObjects
/// and `/Group` was never read.
///
/// With group compositing in, that panel renders clean — the crosses are
/// gone and the swatches match the suite's own reference sheet.
#[test]
fn a_transparency_group_is_composited_as_a_unit() {
    let r = render(page_with_form("/Group << /S /Transparency >>"));
    assert_eq!(r.diagnostics.transparency_groups_composited, 1);
    assert_eq!(
        r.diagnostics.transparency_groups_flattened, 0,
        "flattening is now the FALLBACK, taken only if the buffer cannot \
         be allocated"
    );
    let p = r.pixmap.pixel(30, 30).expect("in bounds").demultiply();
    assert!(p.red() > 200, "the group's contents are still painted");
}

/// **The composite is what carries the outer blend mode**, and this is the
/// assertion that separates a real group implementation from a counter that
/// merely says "group".
///
/// A blue square is painted, then a form containing a red square is invoked
/// under `/BM /Screen`. If the group is composited as a unit, the outer
/// Screen applies once to the group's RESULT: `Screen(blue, red)` is
/// magenta. If the group were flattened, the red square would be screened
/// against the blue directly — which happens to give the same colour here,
/// so the distinguishing half is the ALPHA: a flattened group applies the
/// outer constant alpha to each object inside as well.
#[test]
fn the_outer_blend_mode_applies_to_the_groups_result() {
    let content = "0 0 1 rg 10 10 40 40 re f /GS0 gs /Fm0 Do";
    let stream = format!("{content}\n");
    let form_body = "1 0 0 rg 10 10 40 40 re f";
    let form = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 60 60] \
         /Group << /S /Transparency >> /Length {} >>\nstream\n{form_body}\nendstream",
        form_body.len()
    );
    let bytes = build(&[
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (
            2,
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 60 60] >>",
        ),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /Contents 4 0 R /Resources \
             << /XObject << /Fm0 5 0 R >> \
                /ExtGState << /GS0 << /Type /ExtGState /BM /Screen >> >> >> >>",
        ),
        (
            4,
            &format!("<< /Length {} >>\nstream\n{stream}endstream", stream.len()),
        ),
        (5, &form),
    ]);
    let r = render(bytes);
    assert_eq!(r.diagnostics.transparency_groups_composited, 1);
    let p = r.pixmap.pixel(30, 30).expect("in bounds").demultiply();
    assert!(
        p.red() > 250 && p.green() < 5 && p.blue() > 250,
        "Screen of the group's red result over blue is magenta, got \
         ({}, {}, {})",
        p.red(),
        p.green(),
        p.blue()
    );
}

/// The blend mode in force at the `Do` must NOT also apply to each object
/// inside the group — §11.4.5 says it applies to the group's result. So the
/// group's contents start at `Normal` however the outer state is set.
///
/// Without the reset, a group's first object would be blended once on the
/// way in and again on the way out. With a single opaque object the double
/// application is invisible for `Multiply` (idempotent against white) and
/// very visible for `Screen`, which is why the fixture uses two objects and
/// checks the one UNDERNEATH.
#[test]
fn the_outer_blend_mode_does_not_leak_into_the_groups_contents() {
    let content = "/GS0 gs /Fm0 Do";
    let stream = format!("{content}\n");
    // Inside the group: blue, then red over it, both Normal. Red must win.
    let form_body = "0 0 1 rg 10 10 40 40 re f 1 0 0 rg 10 10 40 40 re f";
    let form = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 60 60] \
         /Group << /S /Transparency >> /Length {} >>\nstream\n{form_body}\nendstream",
        form_body.len()
    );
    let bytes = build(&[
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (
            2,
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 60 60] >>",
        ),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /Contents 4 0 R /Resources \
             << /XObject << /Fm0 5 0 R >> \
                /ExtGState << /GS0 << /Type /ExtGState /BM /Multiply >> >> >> >>",
        ),
        (
            4,
            &format!("<< /Length {} >>\nstream\n{stream}endstream", stream.len()),
        ),
        (5, &form),
    ]);
    let r = render(bytes);
    let p = r.pixmap.pixel(30, 30).expect("in bounds").demultiply();
    assert!(
        p.red() > 250 && p.green() < 5 && p.blue() < 5,
        "inside the group the two fills are Normal, so red covers blue; \
         Multiply then applies once to the result over white paper, \
         leaving red. Got ({}, {}, {})",
        p.red(),
        p.green(),
        p.blue()
    );
}

/// `/I` (isolated) and `/K` (knockout) are still counted, and `/K` gets its
/// own shortfall counter because compositing a knockout group as an
/// ordinary one gets the outer boundary right and the INTERNAL occlusion
/// order wrong: in a knockout group each element composites against the
/// group's initial backdrop, so later elements REPLACE earlier ones rather
/// than layering over them.
#[test]
fn isolated_and_knockout_groups_are_counted_separately() {
    for (extra, knockout) in [
        ("/Group << /S /Transparency /I true >>", 0),
        ("/Group << /S /Transparency /K true >>", 1),
        ("/Group << /S /Transparency /I true /K true >>", 1),
    ] {
        let d = render(page_with_form(extra)).diagnostics;
        assert_eq!(d.transparency_groups_composited, 1, "{extra}");
        assert_eq!(d.transparency_groups_special, 1, "{extra}");
        assert_eq!(
            d.transparency_groups_knockout_approximated, knockout,
            "{extra}"
        );
    }
}

/// A form with NO `/Group` is an ordinary reusable content stream and is
/// not a compositing scope. Counting it would put a large number on
/// ordinary documents — forms are how every producer factors repeated
/// content — and bury the real signal.
#[test]
fn a_plain_form_xobject_is_not_a_transparency_group() {
    let d = render(page_with_form("")).diagnostics;
    assert_eq!(d.transparency_groups_composited, 0);
    assert_eq!(d.transparency_groups_flattened, 0);
    assert_eq!(d.transparency_groups_special, 0);
}

/// `/Group` exists for more than transparency — Table 95 allows other
/// subtypes, and only `/S /Transparency` makes a compositing scope.
#[test]
fn a_group_that_is_not_a_transparency_group_is_not_counted() {
    let d = render(page_with_form("/Group << /S /SomethingElse >>")).diagnostics;
    assert_eq!(d.transparency_groups_composited, 0);
    assert_eq!(d.transparency_groups_flattened, 0);
}

/// The FOUR NON-SEPARABLE modes are recognised names that pdfce
/// deliberately declines to composite, so they land in the shortfall
/// counter rather than the census — the same bucket as a typo, for a
/// completely different reason.
///
/// Pinned because the alternative is worse in a way that is easy to talk
/// yourself into: `tiny_skia` HAS these four modes and mapping to them is
/// one line. They are wrong by up to 107/255 on 9.4–15.5% of colour pairs
/// (its `clip_color` gates the low-gamut rescale on `mx >= 0` where the
/// standard uses `mn < 0`, leaving the branch dead). A wrong rendering
/// that looks plausible is worse than a disclosed one that does not.
#[test]
fn the_non_separable_modes_are_refused_not_silently_wrong() {
    for name in ["Hue", "Saturation", "Color", "Luminosity"] {
        let d = render(page(&format!("<< /Type /ExtGState /BM /{name} >>"))).diagnostics;
        assert_eq!(d.blend_modes_ignored, 1, "/BM /{name} must be refused");
        assert_eq!(
            d.blend_modes_applied, 0,
            "/BM /{name} must not count as applied"
        );
    }
}

/// **Isolated and non-isolated groups differ, and this is the fixture that
/// can tell them apart.**
///
/// The page paints blue. A group then paints red with `/BM /Screen` INSIDE
/// it. What the red screens against depends entirely on `/I` (Table 96):
///
/// - `/I true` (isolated): the group's initial backdrop is TRANSPARENT, so
///   the red has nothing to blend with and stays red. The group's result is
///   then composited over the blue normally — still red.
/// - `/I false` (the DEFAULT, non-isolated): the group's initial backdrop
///   is the page, so the red screens against BLUE and comes out magenta.
///
/// pdfce buffers a group only when buffering changes the answer — a
/// non-isolated group under a neutral outer state is painted inline, which
/// IS the non-isolated semantics rather than an approximation of them. That
/// optimisation and this correctness property are the same decision, so
/// this test guards both: break the buffering condition in either direction
/// and one of these two assertions fails.
#[test]
fn an_isolated_group_blends_against_transparency_a_non_isolated_one_against_the_page() {
    let render_with = |iso: &str| {
        let content = "0 0 1 rg 10 10 40 40 re f /Fm0 Do";
        let stream = format!(
            "{content}
"
        );
        let form_body = "/GS0 gs 1 0 0 rg 10 10 40 40 re f";
        let form = format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 60 60]              /Group << /S /Transparency {iso} >> /Resources              << /ExtGState << /GS0 << /Type /ExtGState /BM /Screen >> >> >>              /Length {} >>
stream
{form_body}
endstream",
            form_body.len()
        );
        let bytes = build(&[
            (1, "<< /Type /Catalog /Pages 2 0 R >>"),
            (
                2,
                "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 60 60] >>",
            ),
            (
                3,
                "<< /Type /Page /Parent 2 0 R /Contents 4 0 R /Resources                  << /XObject << /Fm0 5 0 R >> >> >>",
            ),
            (
                4,
                &format!(
                    "<< /Length {} >>
stream
{stream}endstream",
                    stream.len()
                ),
            ),
            (5, &form),
        ]);
        let r = render(bytes);
        let p = r.pixmap.pixel(30, 30).expect("in bounds").demultiply();
        (p.red(), p.green(), p.blue())
    };

    let (ir, ig, ib) = render_with("/I true");
    assert!(
        ir > 250 && ig < 5 && ib < 5,
        "an ISOLATED group's contents see a transparent backdrop, so the red survives its own Screen: expected red, got ({ir}, {ig}, {ib})"
    );

    let (nr, ng, nb) = render_with("");
    assert!(
        nr > 250 && ng < 5 && nb > 250,
        "a NON-isolated group's contents see the page, so the red screens against blue: expected magenta, got ({nr}, {ng}, {nb})"
    );
}
