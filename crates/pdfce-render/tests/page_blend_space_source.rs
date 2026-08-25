//! ★ `Pass 122.5` — where a page's blending colour space comes from when the
//! page group declares none, and the disclosure that says which.
//!
//! # The claim under test, and why it is a setting rather than a fix
//!
//! **ISO 32000-1 is determinate here and determinate AGAINST consulting the
//! output intent.** §11.4.7 and §11.6.3 each state independently that *"if not
//! otherwise specified, the page group's colour space **shall** be inherited
//! from the native colour space of the output device"* — `shall`, no hedge —
//! and `/OutputIntent` is absent from the 1.7 transparency model entirely.
//!
//! **ISO 32000-2 opens it, and only informatively.** Annex P offers *"from the
//! output device, **or** from the output intent"* with no ranking, no
//! condition and no precedence, so two conformant PDF 2.0 processors render
//! the same file in two different blending spaces and both cite the same
//! annex. There is no reading under which one answer is simply wrong, which is
//! exactly what makes this a setting.
//!
//! # What actually turns on it
//!
//! Overprint, and not by a matter of degree. §11.7.4.3's second bullet makes
//! `B(c_b, c_s)` equal `c_s` for every component *"specified in the current
//! colour space"*; in sRGB every source colour has already been converted to
//! all three components, so every component is specified and `B = c_s`
//! **everywhere**. Overprint in an additive space is therefore
//! **unrepresentable**, not merely unsimulated — no compositing work recovers
//! it, only an n-colorant buffer does.
//!
//! # Why the assertions are written as a PAIR
//!
//! A test that only checked the new default would pass against an
//! implementation that ignored the setting and always used ink. A test that
//! only checked `DeviceNative` would pass against one that never used it. Each
//! variant is therefore asserted to produce the *other* answer on the same
//! file — the same non-vacuity discipline `R162` asks for.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use pdfce_core::document::Document;
use pdfce_core::settings::PageBlendSpaceSource;
use pdfce_render::{RenderOptions, RenderedPage};

/// Render page 1 of a Ghent patch under an explicit blend-space source.
///
/// The corpus lives outside the repository (`docs/LEGAL.md` §5 — the Ghent
/// suite is not redistributable here), so every test in this file **skips**
/// rather than fails when it is absent. A skip is announced on stdout: a
/// silently-skipped test is indistinguishable from a passing one, which is the
/// failure mode this project keeps re-learning.
fn render(name: &str, source: PageBlendSpaceSource) -> Option<RenderedPage> {
    let path = PathBuf::from(r"D:\Dev\temp\ghent-patches").join(name);
    if !path.exists() {
        println!("SKIP: {} not present (external corpus)", path.display());
        return None;
    }
    let doc = Document::from_bytes(std::fs::read(&path).ok()?).expect("fixture parses");
    let pages = pdfce_core::page_tree::pages(&doc).expect("page tree");
    let options = RenderOptions::default().with_page_blend_space_source(source);
    Some(pdfce_render::render_page_with(&doc, &pages[0], 2.0, &options).expect("renders"))
}

/// The patch this Pass was found on: PDF/X-3, no page group `/CS`, a CMYK
/// output intent, and overprint content.
const OVERPRINT_PATCH: &str = "1_GWG011_Overprint-Mode_x3.pdf";

/// Under the shipped default, a CMYK output intent supplies the blending
/// space, the colorant buffer engages, and the provenance says so.
#[test]
fn a_cmyk_output_intent_supplies_the_space_by_default() {
    let Some(page) = render(
        OVERPRINT_PATCH,
        PageBlendSpaceSource::OutputIntentIfSubtractive,
    ) else {
        return;
    };
    assert_eq!(
        page.diagnostics.blend_space_from, "output_intent",
        "the default must take the space from the output intent on a file \
         whose page group declares none"
    );
    assert!(
        page.diagnostics.blend_space_subtractive > 0,
        "a CMYK output intent must yield a subtractive page"
    );
}

/// ★ The other half of the pair. `DeviceNative` is ISO 32000-1 to the letter,
/// and must produce the *opposite* answer on the same file — otherwise the
/// setting is decorative and the test above proves nothing about it.
#[test]
fn device_native_reproduces_the_iso_32000_1_answer() {
    let Some(page) = render(OVERPRINT_PATCH, PageBlendSpaceSource::DeviceNative) else {
        return;
    };
    assert_eq!(
        page.diagnostics.blend_space_from, "device_native",
        "DeviceNative must not consult the output intent"
    );
    assert_eq!(
        page.diagnostics.blend_space_subtractive, 0,
        "pdfce's output device is an RGBA8 pixmap, so §11.4.7's answer for \
         this file is additive"
    );
}

/// A page that DECLARES its group `/CS` is answered by Table 147, and no
/// setting reaches it.
///
/// This is the guard against the fix over-reaching: the setting exists to fill
/// a silence, and a file that is not silent must be unaffected by it. Asserted
/// across BOTH variants, because "unaffected" is a claim about the whole
/// setting rather than about one of its values.
#[test]
fn a_declared_page_group_is_immune_to_the_setting() {
    const DECLARED: &str = "1_GWG160_Transp_Basic_BM_DeviceCMYK_Non-knockout_X4.pdf";
    let Some(native) = render(DECLARED, PageBlendSpaceSource::DeviceNative) else {
        return;
    };
    let Some(intent) = render(DECLARED, PageBlendSpaceSource::OutputIntentIfSubtractive) else {
        return;
    };
    for (label, page) in [("device_native", &native), ("output_intent", &intent)] {
        assert_eq!(
            page.diagnostics.blend_space_from, "page_group",
            "{label}: a declared /Group /CS is its own answer"
        );
        assert!(
            page.diagnostics.blend_space_subtractive > 0,
            "{label}: this patch declares /Group /CS /DeviceCMYK"
        );
    }
    // And the pixels agree, which is the assertion that would actually catch
    // a regression: the provenance string could be right while the space
    // silently differed.
    assert_eq!(
        native.pixmap.data(),
        intent.pixmap.data(),
        "a declared page group must render identically under every value of \
         the setting"
    );
}

/// The provenance is never empty for a page that painted content.
///
/// Reporting it only when it is *interesting* would make its absence ambiguous
/// between "not inferred" and "not recorded", and an ambiguous disclosure is
/// worse than none — a reader cannot tell which question it answered.
#[test]
fn the_provenance_is_always_reported() {
    let Some(page) = render(
        OVERPRINT_PATCH,
        PageBlendSpaceSource::OutputIntentIfSubtractive,
    ) else {
        return;
    };
    assert!(
        !page.diagnostics.blend_space_from.is_empty(),
        "a page that painted content must always disclose where its blending \
         space came from"
    );
}

/// ★ A shading painted under overprint cannot honour it, and now says so.
///
/// Found 2026-08-25 when the operator read `GWG 1.0` cells `e` and `j` and
/// said they carry no trap X but are *"the wrong colour … always have been"*.
/// He was right, and `shading.rs` contained no mention of overprint at all —
/// the gap was real and, unlike the image equivalent, entirely undisclosed.
///
/// The count is asserted as **exactly 2** rather than merely non-zero because
/// two is the number of shading cells on that patch, and it is the figure that
/// ties the counter to the cells he named. A counter that fired on the wrong
/// population would still be non-zero.
#[test]
fn a_shading_under_overprint_discloses_that_it_cannot_honour_it() {
    let Some(page) = render(
        "1_GWG010_CMYK_OP_x3.pdf",
        PageBlendSpaceSource::OutputIntentIfSubtractive,
    ) else {
        return;
    };
    assert_eq!(
        page.diagnostics.overprint_shadings_unsupported, 2,
        "GWG 1.0 paints two shadings under overprint; both are bridged \
         through sRGB and neither can overprint, so both must be disclosed"
    );
}
