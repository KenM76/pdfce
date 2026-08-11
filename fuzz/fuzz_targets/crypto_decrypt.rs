//! Fuzz target: the `/AESV2` decryption path (`pdfce_core::crypto::aes`).
//!
//! # Why this target exists, and why it did not before
//!
//! Increment 1's ciphers needed no fuzzing to be *safe*: RC4 is a keystream
//! XOR over a 256-byte permutation and MD5 is a fixed-size block loop, so
//! neither has an input-derived length, offset, or count anywhere in it.
//! Neither can index out of bounds no matter what bytes arrive.
//!
//! AES-128-CBC is the first cipher in this crate where **the ciphertext's own
//! bytes decide control flow**:
//!
//! - the first 16 bytes are consumed as an IV, so anything shorter than that
//!   is a split at an index the data chose;
//! - the remainder is chunked into 16-byte blocks, so a length that is not a
//!   multiple of the block size is a truncation the data chose;
//! - and the **last decrypted byte is read as a length and used to truncate
//!   the buffer** (PKCS#7). That is the sharpest edge in the module: a
//!   value between 0 and 255 arriving from decrypted attacker-controlled
//!   bytes, used as an offset. It is guarded — `n == 0`, `n > BLOCK_LEN`,
//!   `n > buf.len()` — and this target is what says the guard holds for
//!   inputs nobody thought to write down.
//!
//! Note the second-order property that makes fuzzing worth more here than the
//! unit tests: the pad length is read *after* decryption, so its value is
//! effectively random and **not** something a hand-written fixture can steer.
//! libFuzzer varying the key and IV varies the pad byte for free.
//!
//! # What is driven
//!
//! 1. [`decrypt_cbc_128`] over an arbitrary key **and** arbitrary ciphertext,
//!    with the split taken from the input so libFuzzer controls key length
//!    independently of data length. Wrong key lengths (0, 5, 15, 17, 32) are
//!    reachable and are a documented refusal path, not a panic.
//! 2. The same call at every length boundary that matters — 0, and each of
//!    the first few bytes around `IV_LEN` and `MIN_CIPHERTEXT_LEN` — because
//!    those are exactly the indices the splits use, and a corpus that only
//!    ever contains long inputs never exercises them.
//!
//! # Invariant
//!
//! For ANY key and ANY bytes: no panic, no abort, no unbounded work, and the
//! output is never longer than the input (the crate relies on that — the
//! decryption walk in `document.rs` writes the plaintext back into the
//! ciphertext's own byte span and would overwrite the following object if it
//! could grow). The length relation is asserted here rather than merely
//! documented, because it is a *memory-safety* precondition for the caller
//! and not just a property of the cipher.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdfce_core::crypto::aes::{IV_LEN, MIN_CIPHERTEXT_LEN, decrypt_cbc_128};

/// Run one decryption and assert the caller's precondition.
///
/// `document.rs` writes the result at `span.start` and shortens the recorded
/// length; a result longer than the input would silently run into the next
/// object. The check is cheap and converts a would-be corruption into a
/// reported crash.
fn drive(key: &[u8], data: &[u8]) {
    let plain = decrypt_cbc_128(key, data);
    assert!(
        plain.len() <= data.len(),
        "decryption must never grow its input: {} -> {} (key {} bytes)",
        data.len(),
        plain.len(),
        key.len()
    );
}

fuzz_target!(|data: &[u8]| {
    // The first byte chooses where the key ends, so libFuzzer can vary key
    // length and ciphertext length independently. Everything is `saturating`
    // / checked so the harness itself cannot be the thing that panics.
    let Some((&split, rest)) = data.split_first() else {
        // Even the empty input is a real case: `buf.last()` is `None` and the
        // padding strip must return without touching anything.
        drive(&[], &[]);
        return;
    };

    let at = usize::from(split).min(rest.len());
    let (key, body) = rest.split_at(at);
    drive(key, body);

    // Boundary sweep. The corpus will drift toward whatever lengths happen to
    // be interesting for coverage, and that is usually NOT the handful of
    // indices the splits are written against. Pin them explicitly.
    for n in [
        0,
        1,
        IV_LEN - 1,
        IV_LEN,
        IV_LEN + 1,
        MIN_CIPHERTEXT_LEN - 1,
        MIN_CIPHERTEXT_LEN,
        MIN_CIPHERTEXT_LEN + 1,
    ] {
        if let Some(prefix) = body.get(..n) {
            drive(key, prefix);
        }
    }
});
