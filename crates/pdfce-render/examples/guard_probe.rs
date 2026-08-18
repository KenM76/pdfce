//! Two-sided discharge harness for a `pdfce-render` resource guard, against
//! a corpus directory.
//!
//! ```text
//! cargo run --release -p pdfce-render --example guard_probe -- <dir> [more dirs]
//! ```
//!
//! # Why this exists
//!
//! `ROADMAP.md` carries a standing rule that **every new resource guard is
//! run against the veraPDF §6.1.12 implementation-limits suite before it
//! ships**, two-sidedly: it must be shown to FIRE on a real file, and to
//! stay SILENT across the whole suite. That rule has caught `MAX_TOKEN_LEN`
//! and `MAX_XOBJECT_DEPTH` before.
//!
//! `MAX_DISPLAY_LIST_BYTES` (`Pass 75.0`) shipped without the discharge, and
//! the completion report said so rather than implying otherwise. This is the
//! instrument that discharges it.
//!
//! # What it reports, and why "silent" is the half that needs a corpus
//!
//! For every PDF it walks, it records page 1 and classifies the outcome:
//!
//! | outcome | meaning |
//! |---|---|
//! | `recorded` | a display list was produced — the guard did not fire |
//! | `TOO-LARGE` | the guard fired — **the half a false positive shows up in** |
//! | `refused` | the recorder refused for a capability reason (a shading, an overprint composite, a soft mask); nothing to do with the guard |
//! | `unloadable` | the file did not parse — a §6.1.12 suite is full of deliberately broken files, so this is expected and is NOT a failure |
//!
//! A guard's *firing* half can be shown with one constructed input, and
//! `display_list`'s unit tests already do exactly that against a small
//! `max_bytes`. Its *silence* half cannot: the only way to know a ceiling
//! does not trip on legitimate files is to run it over files somebody else
//! chose. That asymmetry is why this harness takes a corpus and not a
//! fixture.
//!
//! # Reading the result honestly
//!
//! A run where nothing is `TOO-LARGE` discharges the silent half **for the
//! files it walked, at the scale it used**, and nothing more. It is not
//! evidence that no document anywhere reaches 256 MiB — the largest sheet
//! this project has measured holds ~29.5 MiB, which is 8.5× under the
//! ceiling, so a suite of small conformance files was never going to reach
//! it. Say that when reporting, rather than letting a clean run read as a
//! stronger claim than it is.

use std::path::{Path, PathBuf};

use pdfce_core::document::Document;
use pdfce_core::page_tree;
use pdfce_render::{RenderError, RenderOptions, record_page};

fn main() {
    let dirs: Vec<String> = std::env::args().skip(1).collect();
    if dirs.is_empty() {
        eprintln!("usage: guard_probe <dir> [dir...]");
        std::process::exit(2);
    }
    let mut files = Vec::new();
    for d in &dirs {
        collect(Path::new(d), &mut files);
    }
    files.sort();
    println!("walking {} file(s)", files.len());

    let options = RenderOptions::default();
    let (mut recorded, mut too_large, mut refused, mut unloadable) = (0, 0, 0, 0);
    let mut largest: (usize, String) = (0, String::new());

    for path in &files {
        let Ok(doc) = Document::load(path) else {
            unloadable += 1;
            continue;
        };
        let Ok(pages) = page_tree::pages(&doc) else {
            unloadable += 1;
            continue;
        };
        let Some(page) = pages.first() else {
            unloadable += 1;
            continue;
        };
        match record_page(&doc.view(), page, 1.0, 0, &options) {
            Ok(list) => {
                recorded += 1;
                let bytes = list.memory_bytes();
                if bytes > largest.0 {
                    largest = (bytes, path.display().to_string());
                }
            }
            Err(RenderError::PageNotRecordable { reason }) => {
                if reason == pdfce_render::PoisonReason::TooLarge {
                    too_large += 1;
                    println!("  TOO-LARGE  {}", path.display());
                } else {
                    refused += 1;
                }
            }
            Err(_) => unloadable += 1,
        }
    }

    println!();
    println!("recorded    {recorded}");
    println!("TOO-LARGE   {too_large}   <- the guard firing; nonzero needs explaining");
    println!("refused     {refused}   (capability, not the guard)");
    println!("unloadable  {unloadable}   (expected in a deliberately-broken suite)");
    println!();
    println!(
        "largest list {} bytes ({:.2} MiB) - {}",
        largest.0,
        largest.0 as f64 / (1024.0 * 1024.0),
        largest.1
    );
    println!(
        "ceiling      {} bytes ({} MiB)",
        pdfce_render::MAX_DISPLAY_LIST_BYTES,
        pdfce_render::MAX_DISPLAY_LIST_BYTES / (1024 * 1024)
    );
    if largest.0 > 0 {
        println!(
            "headroom     {:.0}x",
            pdfce_render::MAX_DISPLAY_LIST_BYTES as f64 / largest.0 as f64
        );
    }
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("pdf")) {
            out.push(p);
        }
    }
}
