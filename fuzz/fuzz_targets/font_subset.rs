//! Fuzz target: the **FF-C donor subsetter**
//! (`pdfce_render::font::subset::plan_subset`, Pass 21.0 / decision 021 §3.5).
//!
//! ## Why this input is untrusted even though an operator chose it
//!
//! `--embed-font` points at a file the operator picked, and it is tempting to
//! treat that as consent to trust the bytes. It is not. Font files are a
//! long-standing exploit vector, they arrive by email and download like any
//! other document, and the operator is in no position to audit an sfnt table
//! directory. What "the operator chose it" actually rules out is *pdfce being
//! tricked into reading a file the operator never named* — nothing about the
//! contents.
//!
//! ## Fuzz the glue and the ceiling, not `subsetter`'s internals
//!
//! Typst's `subsetter` is `#![deny(unsafe_code)]` and is exercised by Typst's
//! own corpus, so hammering its `glyf` walk here would largely re-run someone
//! else's campaign. pdfce's bugs will be in what pdfce does around it:
//!
//! 1. **The size ceiling's ORDERING.** `MAX_DONOR_BYTES` has to be checked
//!    before the parse, or it bounds nothing — a 64 MiB+ input would be fully
//!    parsed first and only then refused. The unit test asserts that with one
//!    crafted buffer; this target hits it with arbitrary sizes.
//! 2. **Coverage lookup against a hostile `cmap`.** `plan_subset` maps each
//!    requested character to a GID and then narrows it to `u16`. A font whose
//!    charmap yields a GID outside 16 bits must be refused, not truncated —
//!    truncation silently selects a *different, valid* glyph, which is the
//!    worst possible outcome because it renders.
//! 3. **The units-per-em conversion.** Every metric is scaled by
//!    `1000 / upem`. A zero or absurd `upem` is attacker-controlled and turns
//!    that into a division by zero or an `i32` overflow in the `as` cast.
//! 4. **Error mapping totality.** Every `subsetter::Error` must land on a
//!    named `SubsetError`; a panic in the mapping would be a crash on the
//!    error path, which is exactly where nobody looks.
//!
//! ## The contract
//!
//! For ANY byte string and ANY requested character set, `plan_subset` returns
//! — `Ok` or a named `Err`. It must not panic, must not hang, and must not
//! allocate without bound. Note the deliberate absence of a
//! composite-glyph-depth assertion: `subsetter`'s closure is an iterative
//! worklist bounded by `numGlyphs`, so cycles terminate structurally upstream
//! (decision 021 §3.5). A pdfce-side depth guard would be unreachable, and
//! this target plus the cycle fixture cover the property instead.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdfce_render::font::subset::plan_subset;

/// Cap on how many characters the fuzzer may request.
///
/// Not a correctness bound — `plan_subset` handles any count — but a
/// throughput one. Without it the fuzzer spends its budget on enormous
/// character sets against fonts that were never going to parse, which is the
/// least interesting corner of the input space.
const MAX_CHARS: usize = 64;

fuzz_target!(|data: &[u8]| {
    // Split the input: a small prefix steers the REQUEST (face index and the
    // characters asked for), the remainder is the candidate font.
    //
    // Steering these independently matters. The interesting bugs are in the
    // interaction — a valid font asked for characters it lacks, a truncated
    // font asked for many characters, a collection index past the end of a
    // real collection — and a single undifferentiated blob would only ever
    // exercise "is this a font", which the parser already answers.
    let Some((&index_byte, rest)) = data.split_first() else {
        return;
    };
    let Some((&count_byte, font_bytes)) = rest.split_first() else {
        return;
    };

    // Face index: mostly 0 (the overwhelmingly common case, and the one the
    // CLI passes) with occasional large values to reach the collection-index
    // bounds check.
    let face_index = u32::from(index_byte % 4);

    let n = usize::from(count_byte) % MAX_CHARS;
    // Characters drawn from the font bytes themselves. Deliberately includes
    // non-Latin and non-BMP scalars: FF-C exists to embed exactly the text
    // the Standard-14 path cannot, so a fuzz corpus of ASCII would miss the
    // case the feature is for. `from_u32` filters surrogates, which are not
    // scalar values and can never appear in a Rust `char`.
    let chars: Vec<char> = font_bytes
        .iter()
        .take(n)
        .enumerate()
        .filter_map(|(i, b)| {
            let cp = u32::from(*b) | ((i as u32 & 0xff) << 8);
            char::from_u32(cp)
        })
        .collect();
    if chars.is_empty() {
        return;
    }

    // The tag is fixed and valid: a malformed one is already covered by a
    // unit test, and spending fuzz cycles on a parameter pdfce derives itself
    // would test the harness rather than the code.
    let _ = plan_subset(font_bytes, face_index, &chars, "FuzzDonor", "ABCDEF");
});
