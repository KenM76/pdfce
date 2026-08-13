//! Measurement harness for [`pdfce_render::render_page_region`] — the numbers
//! behind `docs/render-region-measurements.md`.
//!
//! ```text
//! cargo run --release -p pdfce-render --example region_bench -- <file.pdf>
//! ```
//!
//! # Why this exists as a committed example rather than a throwaway script
//!
//! It answers one question that governs whether a tiled viewer is a good idea,
//! and the answer is counter-intuitive enough that it must stay re-runnable:
//! **how much of a region render's cost is resolution- and area-independent?**
//!
//! The `FLOOR` case renders a **1x1 point** region. Whatever that costs is not
//! fill — it is content-stream interpretation and path construction, paid in
//! full no matter how few pixels come out. On a dense CAD sheet it is
//! essentially the entire cost, which is why `render_page_region` buys
//! *reachable zoom and bounded memory* and **not** speed, and why a 3x3 tile
//! ring is several times slower than one region covering the same area.
//!
//! Re-run it before changing anything about the interpreter's per-operator
//! cost, or before deciding a display-list cache is not worth building.
//!
//! `--release` matters: a debug build's ratios are not the shipped ratios.
use std::time::Instant;

use pdfce_core::document::Document;
use pdfce_core::page_tree::{self, Rect};
use pdfce_render::{RenderOptions, render_page_region, render_page_view};

fn main() {
    let path = std::env::args().nth(1).expect("usage: region_bench <pdf>");
    let t = Instant::now();
    let doc = Document::load(std::path::Path::new(&path)).expect("load");
    let pages = page_tree::pages(&doc).expect("pages");
    let page = &pages[0];
    println!("load {:?}  page {:?}", t.elapsed(), page.crop_box);
    let cb = page.crop_box;
    let opts = RenderOptions::default();

    for scale in [1.0f32, 2.0] {
        let t = Instant::now();
        match render_page_view(&doc.view(), page, scale) {
            Ok(r) => println!(
                "FULL  scale {scale:>5}  {}x{} = {:>10} px  {:?}",
                r.pixmap.width(),
                r.pixmap.height(),
                r.pixmap.width() * r.pixmap.height(),
                t.elapsed()
            ),
            Err(e) => println!("FULL  scale {scale:>5}  ERR {e}"),
        }
    }

    // THE FLOOR: a 1x1 pt region. Whatever this costs is resolution- and
    // area-independent -- i.e. it is content-stream interpretation plus path
    // construction, and it is what a display-list cache would remove.
    for _ in 0..2 {
        let tiny = Rect::from_corners(
            cb.llx + 500.0,
            cb.lly + 400.0,
            cb.llx + 501.0,
            cb.lly + 401.0,
        );
        let t = Instant::now();
        match render_page_region(&doc.view(), page, 1.0, tiny, &opts) {
            Ok(r) => println!(
                "FLOOR  1x1pt        {}x{} = {:>10} px  {:?}",
                r.pixmap.width(),
                r.pixmap.height(),
                r.pixmap.width() * r.pixmap.height(),
                t.elapsed()
            ),
            Err(e) => println!("FLOOR  ERR {e}"),
        }
    }

    // A 400x300 pt viewport near the middle, at increasing zoom.
    for scale in [1.0f32, 2.0, 8.0, 32.0] {
        let region = Rect::from_corners(
            cb.llx + (cb.urx - cb.llx) * 0.40,
            cb.lly + (cb.ury - cb.lly) * 0.40,
            cb.llx + (cb.urx - cb.llx) * 0.40 + 400.0 / f64::from(scale),
            cb.lly + (cb.ury - cb.lly) * 0.40 + 300.0 / f64::from(scale),
        );
        let t = Instant::now();
        match render_page_region(&doc.view(), page, scale, region, &opts) {
            Ok(r) => println!(
                "REGION scale {scale:>5}  {}x{} = {:>10} px  {:?}",
                r.pixmap.width(),
                r.pixmap.height(),
                r.pixmap.width() * r.pixmap.height(),
                t.elapsed()
            ),
            Err(e) => println!("REGION scale {scale:>5}  ERR {e}"),
        }
    }
}
