//! # `render-profile` — the standing answer to "where does the time go?"
//!
//! Loads a PDF, renders one page at a series of scales, and reports the
//! load/parse/render split, the scaling curve, and what the page's
//! content actually looks like to the renderer.
//!
//! ## Why this is a committed tool and not a scratch file
//!
//! On 2026-08-07 three throwaway probes were written into `interpret.rs`
//! and deleted within hours. **Two produced figures wrong by two orders
//! of magnitude, and both were believed and acted on:**
//!
//! - `Mask::new` reported as 10.1 s of an 18 s render; it is 1.02 s. The
//!   figure came from an ablation that skipped `intersect_clip`
//!   entirely, which also makes every `q` cheap and lets tiny-skia skip
//!   mask sampling — construction plus use, attributed to construction.
//! - Mean clip bbox reported as **0.663% of the page**; it is **66.36%**
//!   — a fraction printed as a percent. That 100× error became the
//!   stated premise of a follow-on optimization, and is still written
//!   into `intersect_clip`'s doc comment as "clips in real drawings are
//!   SMALL relative to the paper".
//!
//! Neither survived contact with a second measurement. Both survived for
//! hours because **there was no second measurement to make** — the probe
//! that produced them no longer existed. A harness that must be
//! rewritten each session is one nobody runs, and an unrepeatable number
//! ages into a fact.
//!
//! ## Reading the output
//!
//! **The scaling curve is the diagnostic**, not any single row. A cost
//! that is quadratic in area rises by the same factor at every doubling;
//! one that jumps at a single step is a cache boundary. On the reference
//! CAD sheet the steps ran 3.23× / 3.14× / 14.1× — three smooth steps
//! then a cliff, which identified a working set crossing L3 rather than
//! an algorithmic term. **A single before/after pair could not have told
//! those apart.**
//!
//! `parse` is the content-stream interpretation *plus* rasterization,
//! because they are not separable from outside: the interpreter paints
//! as it walks. `load` is `Document::from_bytes` — the object graph and
//! xref only. When `load` is a rounding error, optimizing the reader is
//! wasted effort, and on the reference sheet it is ~0.005%.
//!
//! ## Usage
//!
//! ```text
//! cargo run --release -- <file.pdf> [--page N] [--scales 0.25,0.5,1,2] [--repeat N]
//! ```
//!
//! Exits 2 on a usage or load error, 0 otherwise. It reports; it does
//! not judge, and has no pass/fail threshold to drift out of date.

use std::time::Instant;

use pdfce_core::document::Document;
use pdfce_core::page_tree;
use pdfce_core::view::DocumentView;
use pdfce_render::{RenderOptions, profile, render_page_with_view};

fn main() -> std::process::ExitCode {
    let mut args = std::env::args().skip(1);
    let mut path: Option<String> = None;
    let mut page_index: usize = 0;
    let mut scales: Vec<f32> = vec![0.25, 0.5, 1.0, 2.0];
    let mut repeat: usize = 1;

    while let Some(a) = args.next() {
        match a.as_str() {
            "--page" => {
                page_index = args.next().and_then(|v| v.parse().ok()).unwrap_or(0);
            }
            "--scales" => {
                if let Some(v) = args.next() {
                    scales = v.split(',').filter_map(|s| s.trim().parse().ok()).collect();
                }
            }
            "--repeat" => {
                repeat = args.next().and_then(|v| v.parse().ok()).unwrap_or(1).max(1);
            }
            "-h" | "--help" => {
                eprintln!(
                    "render-profile <file.pdf> [--page N] [--scales 0.25,0.5,1,2] [--repeat N]"
                );
                return std::process::ExitCode::SUCCESS;
            }
            other => path = Some(other.to_owned()),
        }
    }

    let Some(path) = path else {
        eprintln!("usage: render-profile <file.pdf> [--page N] [--scales …] [--repeat N]");
        return std::process::ExitCode::from(2);
    };

    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("cannot read {path}: {e}");
            return std::process::ExitCode::from(2);
        }
    };
    let input_len = bytes.len();

    let t = Instant::now();
    let doc = match Document::from_bytes(bytes) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("cannot load {path}: {e}");
            return std::process::ExitCode::from(2);
        }
    };
    let load = t.elapsed();

    let pages = match page_tree::pages(&doc) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("cannot read page tree: {e}");
            return std::process::ExitCode::from(2);
        }
    };
    let Some(page) = pages.get(page_index) else {
        eprintln!("page {page_index} out of range ({} pages)", pages.len());
        return std::process::ExitCode::from(2);
    };

    let view = DocumentView::new(&doc, doc.bytes(), doc.version());
    let opts = RenderOptions::default();

    println!("file      : {path}");
    println!("bytes     : {input_len}");
    println!("pages     : {}, profiling page {page_index}", pages.len());
    println!("load      : {:.3} ms  (object graph + xref only)", load.as_secs_f64() * 1e3);
    println!();
    println!(
        "{:>7}  {:>12}  {:>10}  {:>8}  {:>9}",
        "scale", "pixels", "render", "step", "per Mpx"
    );

    let mut prev: Option<(f64, f64)> = None;
    for &scale in &scales {
        profile::reset();
        let mut best = f64::MAX;
        let mut px = 0u64;
        for _ in 0..repeat {
            let t = Instant::now();
            match render_page_with_view(&view, page, scale, &opts) {
                Ok(r) => {
                    px = u64::from(r.pixmap.width()) * u64::from(r.pixmap.height());
                    best = best.min(t.elapsed().as_secs_f64());
                }
                Err(e) => {
                    eprintln!("render at {scale}x failed: {e}");
                    return std::process::ExitCode::from(2);
                }
            }
        }
        let mpx = px as f64 / 1e6;
        // The step ratio is the diagnostic — see the module docs.
        let step = match prev {
            Some((_, pt)) => format!("{:.2}x", best / pt),
            None => "—".to_owned(),
        };
        println!(
            "{scale:>7}  {px:>12}  {:>9.2}s  {step:>8}  {:>8.2}s",
            best,
            if mpx > 0.0 { best / mpx } else { 0.0 }
        );
        prev = Some((mpx, best));
    }

    // Counters come from the LAST scale rendered. They are geometry and
    // counts, not timings, so they do not vary with scale except where
    // device-space bounds clamp to the page.
    let c = profile::snapshot();
    println!();
    println!("content (at {}x):", scales.last().copied().unwrap_or(1.0));
    println!("  paints            : {}", c.paints);
    println!("    unclipped       : {}", c.paints_unclipped);
    println!(
        "    bbox-cullable   : {} ({:.2}% of clipped)",
        c.paints_cullable,
        c.cullable_pct()
    );
    println!("  clip operations   : {}", c.clips);
    println!(
        "    mean bbox       : {:.2}% of page (individual), {:.2}% (accumulated)",
        c.mean_clip_indiv_pct(),
        c.mean_clip_accum_pct()
    );
    if c.clips > 0 && c.mean_clip_indiv_pct() > 25.0 {
        println!();
        println!(
            "  NOTE: clips cover a large share of the page. Optimizations premised on\n  \
             clips being small relative to the paper do not apply to this file."
        );
    }

    std::process::ExitCode::SUCCESS
}
