//! # A `DeviceGray` fill overprinting a spot backdrop — the §8.6.7 ambiguity
//! (`Pass 143.0`)
//!
//! ## The two defensible readings
//!
//! ISO 32000-1 **§8.6.7** scopes `OPM 1`'s zero-tint rule to *"a colour
//! component **in a `DeviceCMYK` colour space**"*. Its single escape hatch —
//! *"or is implicitly converted to `DeviceCMYK`; see 8.6.5.7"* — points at a
//! clause titled **"Implicit Conversion of CIE-Based Colour Spaces"**, which
//! reaches CIE-based spaces and nothing else. `DeviceGray` is a *device*
//! space, so:
//!
//! * the **literal** reading is that `OPM 1` does not reach it, and a grey
//!   fill writes all four components and knocks the spot backdrop out;
//! * **Acrobat** converts grey to K-only `DeviceCMYK` first and *then*
//!   applies `OPM 1`, so its zero C, M and Y preserve the backdrop.
//!
//! Both are defensible, so pdfce ships both behind
//! `OverprintZeroTintScope` and defaults to Acrobat's — this is a
//! print-conformance axis and the measurement instrument is authored to press
//! behaviour.
//!
//! ## ★★★ Why these tests exist rather than a conformance-suite run
//!
//! Because the licensed corpus **does not contain the case**. Measured
//! 2026-08-28 by instrumenting `cmyk_group_rules` across all 51 of its
//! patches: **zero** paint a `DeviceGray` source through Table 149. The patch
//! whose name promises exactly that authors its greys as `DeviceCMYK
//! [0 0 0 k]`, which already takes the direct-CMYK row and always did.
//!
//! So the oracle that scored every other overprint Pass is **silent here**,
//! and without these fixtures the setting would be correct, wired, documented
//! and unexercised (`R151`).
//!
//! ## ★★ And the route the filed diagnosis named contributed 0 %
//!
//! `Pass 143.0` was filed against `overprint::classify` mapping `DeviceGray`
//! to `SourceKind::OtherProcess`, whose Table 149 row is `[Source; 4]`.
//! Changing that alone moved **zero pixels** — on these fixtures and on all
//! 51 corpus patches — because `Interpreter::overprint_would_change` returned
//! `false` for `DeviceGray`, so the paint never reached `paint_overprint`,
//! never reached `classify`, and was painted normally.
//!
//! That predicate carried a comment calling its `_ => false` arm *"a known
//! under-count rather than a claim of zero"* — true of the **disclosure**, and
//! the sentence did not say that the same arm also gated the **behaviour**.
//! Only an A/B of rendered pixels separated the two routes (`R219`): the
//! classification change compiled, was reached, looked correct, and did
//! nothing.
//!
//! Fixture provenance: `fixtures/synthetic/overprint/PROVENANCE.md`;
//! generator `tools/gen-grey-overprint-fixtures.py`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use pdfce_core::document::Document;
use pdfce_core::page_tree;
use pdfce_core::settings::OverprintZeroTintScope;
use pdfce_render::{RenderOptions, RenderedPage};

/// 1.0 keeps the 200 × 200 pt page a 200 × 200 raster, so the sampled point
/// below is a whole device pixel and no rounding judgement enters any
/// assertion.
const SCALE: f32 = 1.0;

fn render(name: &str, scope: OverprintZeroTintScope) -> RenderedPage {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/synthetic/overprint")
        .join(name);
    let doc =
        Document::from_bytes(std::fs::read(&path).expect("fixture file")).expect("fixture parses");
    let pages = page_tree::pages(&doc).expect("page tree");
    let opts = RenderOptions::default().with_overprint_zero_tint_scope(scope);
    pdfce_render::render_page_with(&doc, &pages[0], SCALE, &opts).expect("renders")
}

fn px(page: &RenderedPage, x: u32, y: u32) -> (u8, u8, u8) {
    let p = page
        .pixmap
        .pixels()
        .get((y * page.pixmap.width() + x) as usize)
        .expect("pixel in range");
    (p.red(), p.green(), p.blue())
}

/// A point well inside the 80 × 80 pt mark, which sits on the 120 × 120 pt
/// spot backdrop. Both fixtures place the mark identically, so one coordinate
/// serves every test.
fn mark(page: &RenderedPage) -> (u8, u8, u8) {
    // The page is 200 pt tall and the mark spans y = 60..140 in PDF space;
    // device y is flipped, so device y = 100 is the mark's middle either way.
    px(page, 100, 100)
}

/// The spot ink is deliberately **chromatic** (C=0.8 M=0.2 Y=0.9 K=0,
/// green-dominant). A neutral backdrop would make "preserved" and "knocked
/// out" differ only in lightness, which a rounding change could imitate.
fn is_greenish(c: (u8, u8, u8)) -> bool {
    c.1 > c.0 && c.1 > c.2
}

/// Grey is neutral by construction, so this is the "knocked out" signature.
fn is_neutral(c: (u8, u8, u8)) -> bool {
    let (r, g, b) = (i32::from(c.0), i32::from(c.1), i32::from(c.2));
    (r - g).abs() <= 1 && (g - b).abs() <= 1
}

// ---------------------------------------------------------------------------
// 1. THE ORACLE-FREE CLAIM — the strongest assertion in this file
// ---------------------------------------------------------------------------

/// ★ Needs no reference render, no remembered colour and no threshold.
///
/// `grey_op_over_spot.pdf` and `cmyk_k_op_over_spot.pdf` differ in exactly one
/// way: one says `0.5 g`, the other says `0 0 0 0.5 k`. They are the same ink
/// stated two ways. *"Treat grey as the K-only CMYK it converts to"* means
/// precisely that they must land on **identical pixels** — so the setting's
/// whole meaning is checkable by comparing pdfce against itself.
///
/// A memorised expected colour can be memorised wrong (`R215`); this cannot.
#[test]
fn grey_matches_the_cmyk_k_only_reference_exactly() {
    let grey = render("grey_op_over_spot.pdf", OverprintZeroTintScope::GreyAsKOnly);
    let cmyk = render(
        "cmyk_k_op_over_spot.pdf",
        OverprintZeroTintScope::GreyAsKOnly,
    );
    assert_eq!(
        mark(&grey),
        mark(&cmyk),
        "a 0.5 grey and a 0 0 0 0.5 CMYK are the same ink; under GreyAsKOnly \
         they must composite identically"
    );
    assert!(
        is_greenish(mark(&grey)),
        "and the shared result must be the PRESERVED spot, not a shared \
         failure to paint: got {:?}",
        mark(&grey)
    );
}

// ---------------------------------------------------------------------------
// 2. THE SETTING MOVES THE PIXEL, IN THE DIRECTION EACH READING PREDICTS
// ---------------------------------------------------------------------------

#[test]
fn the_literal_reading_knocks_the_spot_out() {
    let page = render(
        "grey_op_over_spot.pdf",
        OverprintZeroTintScope::DeviceCmykOnly,
    );
    assert!(
        is_neutral(mark(&page)),
        "§8.6.7 to the letter: a DeviceGray source gets no zero-tint rule, so \
         it writes all four components and the spot is gone. Expected a \
         neutral grey, got {:?}",
        mark(&page)
    );
}

#[test]
fn the_default_reading_preserves_the_spot() {
    let page = render("grey_op_over_spot.pdf", OverprintZeroTintScope::GreyAsKOnly);
    assert!(
        is_greenish(mark(&page)),
        "Acrobat's reading: grey converts to K-only CMYK, whose zero C, M and \
         Y leave the backdrop standing. Expected the green spot to survive, \
         got {:?}",
        mark(&page)
    );
}

/// The default must BE Acrobat's reading, not merely reachable. Asserted
/// against `Default::default()` rather than against the named variant, so
/// flipping the `#[default]` attribute fails here rather than silently
/// changing what every consumer renders.
#[test]
fn the_shipped_default_is_the_acrobat_reading() {
    let explicit = render("grey_op_over_spot.pdf", OverprintZeroTintScope::GreyAsKOnly);
    let defaulted = render("grey_op_over_spot.pdf", OverprintZeroTintScope::default());
    assert_eq!(mark(&explicit), mark(&defaulted));
    assert!(is_greenish(mark(&defaulted)));
}

// ---------------------------------------------------------------------------
// 3. THE TWO CONTROLS — what must NOT move, which is how over-breadth shows
// ---------------------------------------------------------------------------

/// Overprint is **off**. §8.6.7 does not apply at all, so no value of this
/// setting may touch the result. A fix that moves this pixel is reaching
/// paints it has no business reaching.
#[test]
fn overprint_off_is_untouched_by_every_scope() {
    let a = render(
        "grey_noop_over_spot.pdf",
        OverprintZeroTintScope::DeviceCmykOnly,
    );
    let b = render(
        "grey_noop_over_spot.pdf",
        OverprintZeroTintScope::GreyAsKOnly,
    );
    let c = render(
        "grey_noop_over_spot.pdf",
        OverprintZeroTintScope::AllProcessSpaces,
    );
    assert_eq!(mark(&a), mark(&b));
    assert_eq!(mark(&b), mark(&c));
    assert!(
        is_neutral(mark(&a)),
        "with overprint off the grey simply covers the spot: {:?}",
        mark(&a)
    );
}

/// The property: a grey **image** is unaffected by every scope. Table 149
/// gives the direct-CMYK row the qualifier *"and not in a sampled image"*, so
/// a CMYK image already falls to the process row where `OPM 0` and `OPM 1`
/// are identical, and a grey image is that case's analogue.
///
/// # ★★ WHAT THIS TEST DOES **NOT** PIN, established by sabotage
///
/// Its first version claimed to pin the `!in_image_sample` guard in
/// `overprint::classify` — *"if that guard is ever removed, this test fails
/// rather than the comment quietly going stale."* **That claim was false, and
/// three separate sabotages proved it:** removing the guard, widening
/// `GreyAsKOnly` to every space, and changing both image call sites' literal
/// `DeviceCmykOnly` to `AllProcessSpaces` each left this test GREEN.
///
/// The reason is that a grey image never enters the overprint machinery at
/// all under any scope — there are **three** redundant things stopping it, so
/// disabling any one changes nothing. The test therefore verifies a true and
/// useful END-TO-END property (a grey image does not move) while pinning
/// **none** of the individual mechanisms it names.
///
/// ★ That distinction is the point of writing it down rather than deleting
/// the test. A surviving sabotage does not always mean the test is weak; here
/// it meant the **comment's claim about coverage** was wrong. The test stays,
/// with an honest description of its own reach.
#[test]
fn a_grey_image_is_never_upgraded_whatever_the_scope() {
    let a = render(
        "grey_image_op_over_spot.pdf",
        OverprintZeroTintScope::DeviceCmykOnly,
    );
    let b = render(
        "grey_image_op_over_spot.pdf",
        OverprintZeroTintScope::GreyAsKOnly,
    );
    let c = render(
        "grey_image_op_over_spot.pdf",
        OverprintZeroTintScope::AllProcessSpaces,
    );
    assert_eq!(mark(&a), mark(&b), "GreyAsKOnly must not reach an image");
    assert_eq!(
        mark(&b),
        mark(&c),
        "and neither must AllProcessSpaces — the guard is on the image, not \
         on the space"
    );
    assert!(
        is_neutral(mark(&a)),
        "the grey image covers the spot under every scope: {:?}",
        mark(&a)
    );
}

// ---------------------------------------------------------------------------
// 4. THE SCOPES ARE DISTINGUISHABLE — without this, three names for one thing
// ---------------------------------------------------------------------------

/// ★★ The test that makes `AllProcessSpaces` a real value rather than a
/// synonym.
///
/// Found by sabotage: widening `GreyAsKOnly` to match **every** space left
/// the entire suite green, because no fixture put a non-grey process source
/// over a spot backdrop. A setting whose values cannot be told apart by any
/// test is three names for one behaviour, and nothing would have caught a
/// later change collapsing them.
///
/// Pure red converts to `C=0, M=1, Y=1, K=0`, so exactly one component is
/// zero and the backdrop's **cyan** is what is at stake. Under
/// `AllProcessSpaces` it survives; under the other two it does not.
#[test]
fn all_process_spaces_reaches_rgb_and_the_narrower_scopes_do_not() {
    let literal = render(
        "rgb_op_over_spot.pdf",
        OverprintZeroTintScope::DeviceCmykOnly,
    );
    let grey_only = render("rgb_op_over_spot.pdf", OverprintZeroTintScope::GreyAsKOnly);
    let all = render(
        "rgb_op_over_spot.pdf",
        OverprintZeroTintScope::AllProcessSpaces,
    );

    assert_eq!(
        mark(&literal),
        mark(&grey_only),
        "GreyAsKOnly must NOT reach a DeviceRGB source — if this fails, the scope has been widened and the three values have collapsed into two"
    );
    assert_ne!(
        mark(&grey_only),
        mark(&all),
        "AllProcessSpaces must reach it, or the widest scope is unreachable and the enum has a variant that does nothing"
    );
    // And the direction: the preserved cyan pulls the red toward the spot.
    let (r0, _, _) = mark(&grey_only);
    let (r1, _, _) = mark(&all);
    assert!(
        r1 < r0,
        "preserving the backdrop's cyan must DARKEN the red, not lighten it:          {:?} -> {:?}",
        mark(&grey_only),
        mark(&all)
    );
}
