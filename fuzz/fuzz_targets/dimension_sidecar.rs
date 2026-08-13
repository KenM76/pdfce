//! Fuzz target: the ce-dimension `/PieceInfo` sidecar reader
//! (`pdfce_core::dimension::sidecar`, ISO 32000-1 §14.5; `Pass 12.M2`,
//! extended by `Pass 69.0`'s style cascade).
//!
//! Feeds arbitrary bytes to `Document::from_bytes`, then reads the
//! **authoritative dimensioning model** back out of whatever the loader
//! produced — the same call the GUI and every CLI ce-dimension subcommand
//! make on open.
//!
//! ## Why this surface needs a target of its own
//!
//! Everything the sidecar reader consumes comes **out of the file**, and it is
//! the one part of the ce-dimension subsystem that is not a pure function of
//! operator input. Three properties of the data make it worth fuzzing rather
//! than only fixture-testing:
//!
//! 1. **Numbers from the file become geometry.** Page coordinates, standoffs,
//!    arc radii and (since `Pass 69.0`) text heights, stroke widths and arrow
//!    lengths all flow into `/Rect` and `/L` computation. `MAX_PAGE_VALUE` and
//!    `positive_of` are the guards; a fuzzer is what finds the shape that
//!    walks past them (a NaN written as a real, an integer that overflows on
//!    `try_from`, a denominator of zero reaching a division).
//! 2. **The reader is deliberately TOTAL and lenient.** Its contract is that a
//!    malformed sidecar yields `None` or a partial model, never a panic — and
//!    a contract stated in a doc comment is not a contract until something
//!    tries to violate it.
//! 3. **`Pass 69.0` widened the input surface by nine optional keys per group
//!    and thirteen per ce dimension**, several of them numeric, one an array
//!    (`/Color`), and one a name parsed through a token table. Every one is a
//!    new place for a file to say something absurd.
//!
//! ## The invariant asserted
//!
//! For ANY input: reading the model returns normally, and every group and
//! record it produced can be walked and resolved — including the full style
//! cascade — without panicking, aborting or looping. Style resolution is
//! driven explicitly rather than left implicit, because a value that only
//! misbehaves once it is *combined* with its group's (an inherited NaN
//! reaching `extension_metrics`, say) would otherwise never be touched.
//!
//! Shares the loader entry point with `load_document`, so the existing corpus
//! keeps its value: any input that loads now drives the sidecar for free.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdfce_core::dimension::{resolve_style, style_provenance};
use pdfce_core::document::Document;
use pdfce_core::edit::EditSession;

fuzz_target!(|data: &[u8]| {
    let Ok(doc) = Document::from_bytes(data.to_vec()) else {
        return;
    };
    let session = EditSession::new(doc);
    let model = session.dimension_model();

    for group in model.groups() {
        let _ = group.unit();
        let _ = group.scale;
        let _ = group.style;
        let _ = model.member_count(group.id);
    }

    for record in model.dimensions() {
        // The derived-display path (geometry -> scale -> formatted string).
        let _ = model.display(record.id);
        let _ = record.kind.measured_points();
        let _ = record.kind.axis_frame();
        let _ = record.kind.label_anchor();
        let _ = record.kind.linear_geometry();

        // The Pass 69.0 cascade, driven explicitly: resolution COMBINES a
        // file-supplied override with a file-supplied group default, and the
        // combination is where an inherited absurdity would surface.
        if let Some(group) = model.group(record.group) {
            let style = resolve_style(group, &record.style);
            let _ = style.extension_metrics();
            let _ = style.breaks_line_for_text();
            let _ = style_provenance(group, &record.style).each();
        }
    }
});
