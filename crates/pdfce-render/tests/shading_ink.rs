//! # A shading and a fill of the same ink must be the same colour
//!
//! # What was wrong
//!
//! A shading's colour was resolved to three-channel sRGB when its colour **ramp
//! was built**, so by the time anything composited there were no colorants
//! left. On a page that composites in ink that meant a `CMYK → sRGB → CMYK`
//! round trip, and the return leg is a *different function* from the outbound
//! one — a calibrated table out, a naive formula back. The ink that arrived was
//! not the ink that left.
//!
//! ## ★★ Why it stayed invisible for so long, which is the transferable part
//!
//! **Everything on the page took the same round trip**, so everything was
//! consistently slightly wrong *together* and nothing looked out of place.
//!
//! It became visible only when the other half was fixed. `Pass 130.1` gave a
//! `DeviceCMYK` image its authored ink, so images stopped round-tripping — and
//! from then on the same colour drawn as a shading and as an image came out
//! **different**. The operator found it on a conformance sheet whose shading
//! boxes print a live shading beside a reference *image* of what it should look
//! like, captioned *"the shadings should look like the reference image"*. Two
//! of four pairs visibly disagreed. **That box carries no trap cross**, so no
//! automated check in this project could see it; it was found by a human
//! looking at the page.
//!
//! ⇒ Fixing one half of a two-halves-agree-wrongly situation converts a silent
//! shared error into a visible disagreement. That is an argument *for* fixing
//! halves — the disagreement is information — but the second half becomes
//! urgent in a way it was not before.
//!
//! # The oracle, and why it needs no reference render
//!
//! Each fixture draws the **same `DeviceCMYK` colour twice**: once as a flat
//! filled rectangle, once as an axial shading whose function is **constant**.
//! A constant shading is the same colour everywhere, so any pixel of it is
//! comparable to any pixel of the fill and the assertion needs no geometry, no
//! parametric position, and nothing remembered.
//!
//! Verified to fail against a build with the fix disabled: the fill rendered
//! `(151, 64, 133)` and the shading `(160, 90, 113)` — **18 levels apart**.
//!
//! # What this does NOT cover
//!
//! **Mesh shadings** (types 4–7) — and the reader who fixes them should look
//! next door rather than here.
//!
//! ★ This section said *"a mesh still bridges through sRGB and still disagrees
//! with an image of the same colour… Named here so a reader who fixes the mesh
//! case knows this file is where its test belongs."* The first half stopped
//! being true in `Pass 137.1`, the very next Pass. The second half was a
//! prediction and it turned out **wrong**: the mesh tests live in
//! `mesh_ink.rs`, with their own fixtures, because a mesh needed an entirely
//! different carrier (`Shade::Ink`, per-vertex) rather than the ramp this
//! file's fixtures exercise. Two defects with one symptom had two fixes and
//! want two test files.
//!
//! Kept rather than deleted because the *shape* is the lesson: a doc comment
//! that says where a future change belongs is a guess about work nobody has
//! done yet, and it ages worse than a description of what is.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::{Path, PathBuf};

use pdfce_core::document::Document;
use pdfce_core::page_tree;
use pdfce_render::RenderedPage;

const SCALE: f32 = 3.0;

fn render(name: &str) -> RenderedPage {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/synthetic/shading")
        .join(name);
    let doc = Document::load(Path::new(&path)).expect("fixture loads");
    let pages = page_tree::pages(&doc).expect("page tree");
    pdfce_render::render_page(&doc, &pages[0], SCALE).expect("renders")
}

/// The mean RGB of a patch, as `(r, g, b)` in f64 so a comparison is not
/// quantised before it is made.
fn patch(page: &RenderedPage, x0: u32, x1: u32) -> (f64, f64, f64) {
    let w = page.pixmap.width();
    let h = page.pixmap.height();
    let (y0, y1) = (h / 2 - 6, h / 2 + 6);
    let px = page.pixmap.pixels();
    let (mut r, mut g, mut b, mut n) = (0.0, 0.0, 0.0, 0.0);
    for y in y0..y1 {
        for x in x0..x1 {
            let p = px[(y * w + x) as usize];
            r += f64::from(p.red());
            g += f64::from(p.green());
            b += f64::from(p.blue());
            n += 1.0;
        }
    }
    (r / n, g / n, b / n)
}

/// The flat fill occupies roughly x 10–90 of a 200 pt page; the shading 110–190.
fn fill_and_shading(page: &RenderedPage) -> ((f64, f64, f64), (f64, f64, f64)) {
    let w = f64::from(page.pixmap.width());
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let at = |a: f64, b: f64| ((w * a) as u32, (w * b) as u32);
    let (f0, f1) = at(0.20, 0.30);
    let (s0, s1) = at(0.70, 0.80);
    (patch(page, f0, f1), patch(page, s0, s1))
}

fn mean_abs(a: (f64, f64, f64), b: (f64, f64, f64)) -> f64 {
    ((a.0 - b.0).abs() + (a.1 - b.1).abs() + (a.2 - b.2).abs()) / 3.0
}

/// ★★★ THE ONE THAT MATTERS. On a page that composites in ink, a shading and a
/// fill of the same authored `DeviceCMYK` colour must be the same colour.
#[test]
fn a_shading_and_a_fill_of_one_ink_agree_on_a_subtractive_page() {
    let page = render("shading-vs-fill-cmyk.pdf");
    let (fill, shading) = fill_and_shading(&page);
    let d = mean_abs(fill, shading);
    assert!(
        d <= 1.0,
        "★ the SAME authored ink rendered two different colours: fill {fill:?} \
         vs shading {shading:?}, mean |diff| {d:.2}. The shading's ramp resolved \
         to sRGB before anything composited, so it took a CMYK -> sRGB -> CMYK \
         round trip the fill did not. Measured at 18.33 before the fix"
    );
}

/// The additive control.
///
/// On a page with no group colour space there is no colorant buffer and no
/// round trip for either object, so they agreed even before the fix. Asserted
/// so that a future change which breaks the *additive* path cannot hide behind
/// the subtractive test passing — the two paths are different code.
#[test]
fn a_shading_and_a_fill_of_one_ink_agree_on_an_additive_page_too() {
    let page = render("shading-vs-fill-rgb.pdf");
    let (fill, shading) = fill_and_shading(&page);
    let d = mean_abs(fill, shading);
    assert!(
        d <= 1.0,
        "fill {fill:?} vs shading {shading:?}, mean |diff| {d:.2}"
    );
}

/// ★ The two pages are allowed to differ from EACH OTHER, and pinning that they
/// do not would be wrong.
///
/// A subtractive page converts its result out of ink at the end; an additive
/// one never entered ink. The two are *required* to differ where that
/// conversion is not the identity, and this project has measured that gap at up
/// to ~100 levels on saturated overlaps. What must hold is that within each
/// page the two objects agree — which the tests above assert — not that the
/// pages agree with one another.
///
/// This test exists to stop somebody "tightening" the two tests above into a
/// cross-page equality that would be false for a correct renderer.
#[test]
fn the_two_pages_are_not_required_to_match_each_other() {
    let cmyk = render("shading-vs-fill-cmyk.pdf");
    let rgb = render("shading-vs-fill-rgb.pdf");
    let (cf, cs) = fill_and_shading(&cmyk);
    let (rf, rs) = fill_and_shading(&rgb);
    // Each page is internally consistent...
    assert!(mean_abs(cf, cs) <= 1.0);
    assert!(mean_abs(rf, rs) <= 1.0);
    // ...and no claim is made about cmyk-vs-rgb. Recorded, not asserted.
    let across = mean_abs(cf, rf);
    assert!(
        across.is_finite(),
        "cross-page difference is {across:.2} — recorded deliberately, never pinned"
    );
}
