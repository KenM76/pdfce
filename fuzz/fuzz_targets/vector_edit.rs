//! Fuzz target: the Pass 9c-min vector-edit surgery planners
//! (`pdfce_core::vector::edit`).
//!
//! Decision 011 Appendix A Pass 9c-min acceptance: *"Fuzz over operand
//! rewriting (degenerate coords, huge operands) 0 crashes."* Over ANY input
//! bytes this drives, for the object model the tokenizer + decomposer
//! produce:
//!
//! 1. `plan_delete` on EVERY object (path / text / image) — the pure
//!    byte-span removal + splice;
//! 2. `plan_move` on every PATH object with a spread of page-space deltas,
//!    including degenerate ones (`NaN`, `±∞`, `1e300`) — exercises the CTM
//!    linear-inverse, the per-operator operand rewrite, the malformed-arity
//!    refusal, and `emit_number` over hostile magnitudes;
//! 3. `plan_move_node` on every path object across a range of node indices
//!    (past the anchor count, so the out-of-range / rectangle / implicit
//!    refusals are all reached) with degenerate target points — exercises
//!    the anchor enumeration (the decompose-mirroring subpath bookkeeping),
//!    the affine inverse, and the single-operator re-emit;
//! 4. `anchor_count` on every path object.
//!
//! Invariant (ARCHITECTURE.md §10 panic-free policy): for ANY input, none of
//! these panics, aborts, or runs unbounded — the planners either return a
//! `Vec<u8>` or a by-name `VectorEditError`, every access is checked, and
//! `MAX_NODES`/`MAX_OBJECTS` cap the decomposition upstream.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdfce_core::content::ContentStream;
use pdfce_core::vector::{
    Matrix, NoXObjects, Point, VectorObject, anchor_count, decompose, plan_delete, plan_move,
    plan_move_node,
};

/// A spread of page-space deltas, from tame to hostile.
const DELTAS: [(f64, f64); 5] = [
    (0.0, 0.0),
    (10.0, -7.5),
    (1e300, -1e300),
    (f64::NAN, 0.0),
    (f64::INFINITY, f64::NEG_INFINITY),
];

fuzz_target!(|data: &[u8]| {
    let Ok(content) = ContentStream::parse(data.to_vec()) else {
        return;
    };
    let model = decompose(&content, Matrix::IDENTITY, &NoXObjects);

    for obj in &model.objects {
        // Delete works on any object kind.
        let _ = plan_delete(&content, obj);

        let VectorObject::Path(path) = obj else {
            continue;
        };

        // Move with every delta (degenerate coords + huge operands).
        for (dx, dy) in DELTAS {
            let _ = plan_move(&content, path, dx, dy);
        }

        // Node drag across a range that overruns the anchor count, with
        // degenerate targets, so every refusal branch is reachable.
        let n = anchor_count(&content, path);
        for node in 0..n.saturating_add(2) {
            for pt in [
                Point::new(0.0, 0.0),
                Point::new(1e300, -1e300),
                Point::new(f64::NAN, f64::INFINITY),
            ] {
                let _ = plan_move_node(&content, path, node, pt);
            }
        }
    }
});
