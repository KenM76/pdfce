//! # `render_page_region` — a region must be the same pixels as the crop
//!
//! Answers the request in `D:\Dev\FeatureRequests\pdfce_FeatureRequests\`
//! (`request_region_rasterisation.md`, from the `pdfceGUI` session): rasterise
//! a sub-rectangle of a page so a viewer at high magnification pays for the
//! pixels it shows rather than for the whole sheet.
//!
//! ## The oracle, and why it is the only one worth having
//!
//! A region render is trivially "correct" against itself — it produces *some*
//! pixmap of the right size, and eyeballing it proves nothing. The real
//! contract is **differential**: rendering region *R* at scale *s* must
//! produce exactly the pixels that cropping a full-page render at scale *s* to
//! *R* would produce. Anything less and a tiled viewer shows seams, doubled
//! strokes, or content shifted by a pixel per tile — defects that look like
//! rendering bugs and are actually transform bugs.
//!
//! So every test here compares against a full-page render. That is a slow
//! oracle and an exact one, which is the right trade for a transform.
//!
//! ## What each test pins
//!
//! | test | the failure it catches |
//! |---|---|
//! | `a_region_matches_the_same_crop_of_the_full_page` | any error in the device-space translation — the whole point |
//! | `four_tiles_reassemble_into_the_whole_page` | seams: an off-by-one in the floor/ceil that would lose or double a row between adjacent tiles |
//! | `a_region_is_bounded_by_its_own_size_not_the_pages` | the actual feature: that the guard now applies to the region, i.e. that deep zoom is reachable at all |
//! | `an_empty_region_refuses_by_name` | a degenerate rect producing a zero-sized pixmap rather than a named error |

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::{Path, PathBuf};

use pdfce_core::document::Document;
use pdfce_core::page_tree::{self, Rect};
use pdfce_render::{
    MAX_PIXMAP_EDGE, RenderError, RenderOptions, render_page_region, render_page_view,
};

fn fixture(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/synthetic")
        .join(rel)
}

/// The sub-rectangle of a full-page pixmap corresponding to a device-space
/// origin and size, as a flat RGBA vector — the oracle's crop.
fn crop(pixmap: &pdfce_render::tiny_skia::Pixmap, x0: u32, y0: u32, w: u32, h: u32) -> Vec<u8> {
    let stride = pixmap.width() as usize * 4;
    let data = pixmap.data();
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for row in 0..h {
        let src = (y0 + row) as usize * stride + x0 as usize * 4;
        out.extend_from_slice(&data[src..src + (w as usize) * 4]);
    }
    out
}

/// ★ The contract: a region is the crop.
#[test]
fn a_region_matches_the_same_crop_of_the_full_page() {
    let doc = Document::load(&fixture("addtext/plain.pdf")).expect("fixture loads");
    let pages = page_tree::pages(&doc).expect("page tree");
    let page = &pages[0];
    let scale = 2.0;

    let full = render_page_view(&doc.view(), page, scale).expect("full page rasterises");

    // A region well inside the page, on integral page-space coordinates so the
    // expected device origin is exact at this scale and the comparison is not
    // testing the rounding policy by accident (that is the tiling test's job).
    let cb = page.crop_box;
    let region = Rect::from_corners(cb.llx + 50.0, cb.lly + 60.0, cb.llx + 250.0, cb.lly + 220.0);

    let got = render_page_region(&doc.view(), page, scale, region, &RenderOptions::default())
        .expect("region rasterises");

    let w = (200.0 * scale) as u32;
    let h = (160.0 * scale) as u32;
    assert_eq!(got.pixmap.width(), w, "region width");
    assert_eq!(got.pixmap.height(), h, "region height");

    // Device y is flipped: the region's TOP edge in page space (ury) is the
    // small device y. Getting this backwards is the single most likely defect,
    // and it would still produce a plausible-looking picture of the wrong part
    // of the page.
    let x0 = (50.0 * scale) as u32;
    let y0 = ((cb.ury - (cb.lly + 220.0)) * f64::from(scale)) as u32;

    let expected = crop(&full.pixmap, x0, y0, w, h);
    let differing = expected
        .iter()
        .zip(got.pixmap.data().iter())
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(
        differing,
        0,
        "a region must be byte-identical to the corresponding crop of the full \
         page; {differing} of {} bytes differ. A large count with the right \
         SIZE means the translation is wrong, not the scale.",
        expected.len()
    );
}

/// ★ Four tiles reassemble into the whole page, with no seam and no overlap.
///
/// This is the test a tiled viewer actually depends on. The floor/ceil policy
/// on the region's device bounds is chosen so a requested region is fully
/// covered rather than cropped; a policy that rounded instead would lose a row
/// between adjacent tiles and show a hairline seam that reads as a rendering
/// artefact.
#[test]
fn four_tiles_reassemble_into_the_whole_page() {
    let doc = Document::load(&fixture("addtext/plain.pdf")).expect("fixture loads");
    let pages = page_tree::pages(&doc).expect("page tree");
    let page = &pages[0];
    let scale = 1.0;
    let full = render_page_view(&doc.view(), page, scale).expect("full page");

    let cb = page.crop_box;
    let (mx, my) = ((cb.llx + cb.urx) / 2.0, (cb.lly + cb.ury) / 2.0);
    let quadrants = [
        Rect::from_corners(cb.llx, my, mx, cb.ury), // top-left in page space
        Rect::from_corners(mx, my, cb.urx, cb.ury), // top-right
        Rect::from_corners(cb.llx, cb.lly, mx, my), // bottom-left
        Rect::from_corners(mx, cb.lly, cb.urx, my), // bottom-right
    ];

    let mut covered = 0u64;
    for (i, q) in quadrants.iter().enumerate() {
        let tile = render_page_region(&doc.view(), page, scale, *q, &RenderOptions::default())
            .unwrap_or_else(|e| panic!("quadrant {i} rasterises: {e}"));

        let x0 = ((q.llx - cb.llx) * f64::from(scale)).floor() as u32;
        let y0 = ((cb.ury - q.ury) * f64::from(scale)).floor() as u32;
        let (w, h) = (tile.pixmap.width(), tile.pixmap.height());

        // Only compare the part that lies inside the full-page raster: the
        // ceil policy can push a tile one pixel past the page edge, which is
        // correct (the region was covered) and simply has no oracle there.
        let w = w.min(full.pixmap.width().saturating_sub(x0));
        let h = h.min(full.pixmap.height().saturating_sub(y0));
        assert!(w > 0 && h > 0, "quadrant {i} must overlap the page");

        let expected = crop(&full.pixmap, x0, y0, w, h);
        let stride = tile.pixmap.width() as usize * 4;
        let mut actual = Vec::with_capacity(expected.len());
        for row in 0..h {
            let src = row as usize * stride;
            actual.extend_from_slice(&tile.pixmap.data()[src..src + (w as usize) * 4]);
        }
        assert_eq!(
            expected
                .iter()
                .zip(actual.iter())
                .filter(|(a, b)| a != b)
                .count(),
            0,
            "quadrant {i} must match the full-page raster exactly — a mismatch \
             here is the seam a tiled viewer would show"
        );
        covered += u64::from(w) * u64::from(h);
    }

    let page_px = u64::from(full.pixmap.width()) * u64::from(full.pixmap.height());
    assert_eq!(
        covered, page_px,
        "the four quadrants must cover every pixel exactly once — {covered} vs \
         {page_px} means a gap (seam) or an overlap (doubled strokes)"
    );
}

// The `/Rotate` axis-swap case is NOT here, deliberately.
//
// It lives as a unit test in `crates/pdfce-render/src/lib.rs`
// (`a_region_of_a_rotated_page_is_the_crop_of_the_rotated_page`) because
// **no fixture in `fixtures/synthetic/` carries a `/Rotate` key at all** —
// the first draft of that test lived here, found nothing to load, and
// skipped, i.e. it reported success while testing nothing. The in-memory
// `doc_with_content` helper can set `/Rotate 90` directly, so the coverage is
// real there and unreachable here. Recorded rather than silently moved,
// because "no fixture exercises page rotation" is a corpus gap that outlives
// this feature.

/// ★ The feature itself: the guard now bounds the REGION, not the page.
///
/// This is what makes deep zoom reachable. At a scale that would make the
/// whole page exceed `MAX_PIXMAP_EDGE` — and therefore fail outright — a
/// modest region must still render.
#[test]
fn a_region_is_bounded_by_its_own_size_not_the_pages() {
    let doc = Document::load(&fixture("addtext/plain.pdf")).expect("fixture loads");
    let pages = page_tree::pages(&doc).expect("page tree");
    let page = &pages[0];
    let cb = page.crop_box;

    // A scale at which the full page is definitively over the guard.
    let scale = (f64::from(MAX_PIXMAP_EDGE) / (cb.ury - cb.lly) * 1.5) as f32;

    assert!(
        matches!(
            render_page_view(&doc.view(), page, scale),
            Err(RenderError::BadRasterSize { .. })
        ),
        "the premise of this test is that the WHOLE page is over the guard at \
         this scale"
    );

    // A 40x30 pt region at the same scale is small, and must succeed.
    let region = Rect::from_corners(cb.llx + 10.0, cb.lly + 10.0, cb.llx + 50.0, cb.lly + 40.0);
    let got = render_page_region(&doc.view(), page, scale, region, &RenderOptions::default())
        .expect("a small region must render at a zoom the whole page cannot");
    assert!(got.pixmap.width() > 0 && got.pixmap.height() > 0);
    assert!(
        got.pixmap.width() <= MAX_PIXMAP_EDGE && got.pixmap.height() <= MAX_PIXMAP_EDGE,
        "and it is still bounded — by its own size"
    );
}

/// A degenerate region is a named refusal, not a zero-sized pixmap.
#[test]
fn an_empty_region_refuses_by_name() {
    let doc = Document::load(&fixture("addtext/plain.pdf")).expect("fixture loads");
    let pages = page_tree::pages(&doc).expect("page tree");
    let page = &pages[0];
    let cb = page.crop_box;
    let empty = Rect::from_corners(cb.llx + 10.0, cb.lly + 10.0, cb.llx + 10.0, cb.lly + 10.0);
    assert!(matches!(
        render_page_region(&doc.view(), page, 1.0, empty, &RenderOptions::default()),
        Err(RenderError::BadRasterSize { .. })
    ));
}
