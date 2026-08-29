//! Fuzz target: the two CLIPBOARD payload readers —
//! `pdfce_core::vector::ObjectClip::from_bytes` and
//! `pdfce_core::formclip::FieldClip::from_bytes` (`Pass 120.1` / `Pass 167.0`).
//!
//! ## Why these belong in the fuzz corpus, and why one of them is overdue
//!
//! Both parse **untrusted bytes that arrive from outside the process**. That
//! is the whole point of the two formats: a clip is written to a file, put on
//! the OS clipboard, carried between documents, and read back by a different
//! run of the program. A payload can therefore be truncated by a failed
//! write, corrupted in transit, produced by a newer build, or hand-crafted.
//!
//! `ObjectClip::from_bytes` shipped in `Pass 120.1` and had **no fuzz target**
//! until this one — found while adding the field clipboard, not by review.
//! `ARCHITECTURE.md` §10.2 asks for a target on every untrusted-input parser,
//! and the gap survived because the clipboard reads like an internal format
//! rather than like a file format. It is a file format.
//!
//! ## The shape a fuzzer is good at here
//!
//! Both readers are **length-prefixed binary containers wrapping COS syntax**,
//! which is two attack surfaces stacked:
//!
//! 1. **The framing.** Every count and every byte-string length comes out of
//!    the payload. A `u32::MAX` element count must be refused *before* it is
//!    allocated for, and `at + n` must not wrap. The `Reader` is written with
//!    `checked_add` + `get` for exactly this, and this target is what proves
//!    it stays that way.
//! 2. **The embedded COS objects.** Each object value is re-parsed by the
//!    crate's own `Parser`, so a payload can hand it arbitrarily nested
//!    dictionaries, unterminated strings, or a `/Length` that disagrees with
//!    the payload beside it. The stream-reconstruction step
//!    (`Object::Dict` + a payload becomes an `Object::Stream` whose span is
//!    synthesised) is a place where a length could be trusted and is not.
//!
//! ## The invariant asserted
//!
//! For ANY input, both readers return normally — `Ok(..)` or a `ClipError` —
//! and never panic, abort, or loop. Every successful parse is **re-serialised
//! and re-parsed**, which exercises the writers on parser-derived data and
//! catches the asymmetric case: a payload the reader accepts but the writer
//! then cannot express, or a round trip that does not converge.
//!
//! The re-parse result is compared for the field clipboard, where the format
//! is total (every field survives serialisation). It is deliberately **not**
//! compared for `ObjectClip`, whose `to_bytes` documents that it drops the
//! `annotations` payload — so equality would fail by design rather than by
//! defect.
//!
//! ## Also driven: the prefix corpus
//!
//! A valid payload truncated at every length is the single most likely
//! real-world corruption (a partial write, a clipboard read that returned
//! short), so each successful parse's own re-serialisation is fed back in at
//! a few cut points rather than leaving that case to the fuzzer's luck.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdfce_core::formclip::FieldClip;
use pdfce_core::vector::ObjectClip;

fuzz_target!(|data: &[u8]| {
    // -- the object clipboard (Pass 120.1) --------------------------------
    if let Ok(clip) = ObjectClip::from_bytes(data) {
        let bytes = clip.to_bytes();
        // NOT compared for equality: `to_bytes` documents that it drops the
        // annotations payload, so a mismatch here is the format's stated
        // limit rather than a defect.
        let _ = ObjectClip::from_bytes(&bytes);
        for cut in cuts(bytes.len()) {
            let _ = ObjectClip::from_bytes(bytes.get(..cut).unwrap_or_default());
        }
    }

    // -- the field clipboard (Pass 167.0) ---------------------------------
    if let Ok(clip) = FieldClip::from_bytes(data) {
        let bytes = clip.to_bytes();
        match FieldClip::from_bytes(&bytes) {
            Ok(again) => assert!(
                again == clip,
                "the field-clip format is TOTAL: everything it holds survives \
                 serialisation, so a round trip must converge",
            ),
            Err(e) => panic!("a clip this build wrote must parse back: {e}"),
        }
        for cut in cuts(bytes.len()) {
            let _ = FieldClip::from_bytes(bytes.get(..cut).unwrap_or_default());
        }
    }

    // -- cross-format confusion -------------------------------------------
    //
    // The two payloads share a shell, a clipboard and a file extension habit,
    // so each reader must refuse the other's bytes on the MAGIC rather than
    // on whatever a length prefix reads out of the wrong offset.
    let _ = ObjectClip::from_bytes(data);
    let _ = FieldClip::from_bytes(data);
});

/// A handful of truncation points: the empty payload, a short prefix, the
/// midpoint, and one byte short of complete.
fn cuts(len: usize) -> [usize; 4] {
    [0, len / 4, len / 2, len.saturating_sub(1)]
}
