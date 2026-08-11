//! AES-128 in CBC mode — the `/AESV2` crypt filter method (ISO 32000-1 §7.6.2).
//!
//! # Why this is a dependency when [`md5`](super::md5) and [`rc4`](super::rc4)
//! are not
//!
//! [`md5`](super::md5)'s module docs record the judgement that MD5 and RC4 were
//! cheaper and lower-risk to write in-crate than to depend on, **and state in
//! the same breath that the reasoning does not extend to AES**: AES has real
//! implementation hazards (timing, key schedules, mode handling), a live
//! ecosystem, and well-audited permissive crates. That sentence was written
//! before this module existed, precisely so it could not be quietly reused to
//! justify hand-rolling the next cipher. It is honoured here.
//!
//! So the block cipher and the CBC chaining come from RustCrypto's `aes` and
//! `cbc`. What stays in-crate is the part that is **not** a cryptographic
//! hazard and **is** a policy decision: which bytes are the IV, and what to do
//! about padding that does not verify. See "Padding is a policy, not a
//! primitive" below.
//!
//! # The wire format (§7.6.2, and TRAP T5)
//!
//! ```text
//! ciphertext = IV(16 bytes, random) ‖ AES-128-CBC(object_key, IV, pad(plaintext))
//! pad        = PKCS#5/7 to a 16-byte block, ALWAYS present
//!              (a full 16-byte block of 0x10 when the plaintext is already
//!               a multiple of 16 — there is no "no padding needed" case)
//! ```
//!
//! Three consequences that are easy to get wrong and silent when you do:
//!
//! - **The IV is prefixed to the data, not stored in the dictionary.** There is
//!   no `/IV` key anywhere in an encryption dictionary; a reader that looks for
//!   one finds nothing and decrypts with a zero IV, which corrupts exactly the
//!   *first* 16 bytes and leaves the rest perfect. On a `/FlateDecode` stream
//!   that is a header failure; on a raw stream it is 16 bytes of noise most
//!   viewers never draw.
//! - **T5 — `/Length` on an AES stream counts the IV and the padding.** It is
//!   the length of the *ciphertext*, so the plaintext is always strictly
//!   shorter, by at least 17 bytes (16 IV + at least 1 pad byte). See
//!   [`crate::document`]'s decryption walk for what that costs the object model.
//! - **T1 — the per-object key derivation gains four `sAlT` bytes** in its MD5
//!   *input*, but the derived key stays `min(n+5, 16)` bytes. That is handled in
//!   [`FileKey::object_key`](super::standard::FileKey::object_key), not here;
//!   this module is handed a finished 16-byte key.
//!
//! # Padding is a policy, not a primitive
//!
//! `cbc` can strip PKCS#7 itself (`decrypt_padded_b2b::<Pkcs7>`) and returns an
//! `Err` when the padding does not verify. This module deliberately does **not**
//! use that, and takes the raw block API instead, because "the padding did not
//! verify" is a question about a possibly-damaged file, and the right answer to
//! it is a pdfce product decision rather than a cryptographic one:
//!
//! **When the padding verifies, it is stripped. When it does not, every
//! decrypted byte is returned instead, and nothing is reported.**
//!
//! The reasoning, in order:
//!
//! 1. **The key is already known to be right.** Padding is checked *after*
//!    authentication has succeeded against `/U` or `/O`. So invalid padding
//!    here does not mean "wrong password" — it means the bytes are damaged, or
//!    the producer wrote non-conforming padding.
//! 2. **Keeping the bytes is strictly better than discarding them.** The great
//!    majority of PDF streams are `/FlateDecode`, which is self-terminating —
//!    it stops at its own end-of-stream marker and simply ignores up to 16
//!    trailing junk bytes. Returning the unstripped plaintext therefore
//!    recovers the stream completely in the common case, where returning an
//!    error or the untouched ciphertext recovers nothing at all.
//! 3. **The usual argument against lenient padding does not apply.** Lenient
//!    PKCS#7 handling is dangerous when an attacker can observe whether it
//!    succeeded — the padding-oracle attack. pdfce is a local file reader: there
//!    is no oracle, no attacker-observable response, and no adaptive query. The
//!    hazard being traded away does not exist in this program, and the recovery
//!    being bought is real.
//!
//! This is a deliberate leniency and it is tested in both directions
//! (`invalid_padding_keeps_every_byte`, `valid_padding_is_stripped`) so it
//! cannot decay into an accident.
//!
//! # What is NOT here
//!
//! **Encryption.** pdfce cannot write an encrypted document at all — both save
//! paths refuse one (`WriteError::EncryptedSaveUnsupported`). Adding an encrypt
//! function here before there is a writer that could call it would be an
//! untested code path wearing the appearance of a capability.
//!
//! **AES-256 (`/AESV3`).** The cipher is the same primitive at a different key
//! length, but its *key derivation* is Algorithm 2.A, not Algorithm 1, and at
//! `/R` 6 that algorithm is unsourced past step (a). `/AESV3` stays refused at
//! parse time. When it lands it will call into this module's CBC routine with a
//! 32-byte key; the split is drawn here for that reason.

use aes::Aes128;
use aes::cipher::{Block, BlockModeDecrypt, KeyIvInit};

/// The AES block size in bytes. Fixed at 16 for every AES key length — it is
/// the *key* that varies between AES-128/192/256, never the block.
pub const BLOCK_LEN: usize = 16;

/// The length of the initialisation vector prefixed to every `/AESV2`
/// ciphertext. Equal to the block size, which is a property of CBC rather than
/// a coincidence: CBC XORs the IV into the first block.
pub const IV_LEN: usize = 16;

/// The shortest byte string that can be a well-formed `/AESV2` ciphertext:
/// a 16-byte IV plus one full padded block. There is no shorter valid case,
/// because §7.6.2's padding is *always* present — an empty plaintext still
/// encrypts to a full block of `0x10`.
pub const MIN_CIPHERTEXT_LEN: usize = IV_LEN + BLOCK_LEN;

type Aes128CbcDec = cbc::Decryptor<Aes128>;

/// Decrypt an `/AESV2` string or stream: strip the IV, run AES-128-CBC, and
/// remove PKCS#7 padding if it verifies.
///
/// `key` is the finished per-object key from Algorithm 1 — already salted with
/// `sAlT` (T1) by the caller. `data` is the raw ciphertext **including** its
/// leading IV, exactly as it sits in the file.
///
/// # Returns
///
/// The plaintext, which is always **shorter** than `data` by at least
/// [`MIN_CIPHERTEXT_LEN`] minus the last block's payload. Callers that track
/// byte spans must record the returned length rather than assuming the
/// length-preserving behaviour RC4 gave them.
///
/// # Malformed input
///
/// This returns a `Vec` rather than a `Result` because every failure mode here
/// is "this file is damaged", and the caller's only sensible response is the
/// same one it already has for a stream whose `/Length` overruns the buffer:
/// carry on and let the object fail to decode, with an error about the object.
/// Raising a distinct error per malformation would give the operator a
/// cryptographic message for a corruption problem.
///
/// - `data` shorter than [`MIN_CIPHERTEXT_LEN`], or a `key` that is not 16
///   bytes → an empty `Vec`. There is no plaintext to recover.
/// - A ciphertext body that is not a whole number of blocks → the trailing
///   partial block is **ignored**. CBC has no meaning for a partial block, and
///   the whole blocks before it are still recoverable.
/// - Padding that does not verify → every decrypted byte is returned unstripped.
///   See the module docs; this is deliberate.
///
/// # Examples
///
/// A round trip through the matching encryptor, showing that the IV travels
/// with the data and that the plaintext comes back shorter than the ciphertext:
///
/// ```
/// use pdfce_core::crypto::aes::{decrypt_cbc_128, MIN_CIPHERTEXT_LEN};
///
/// // A ciphertext produced with key `[0x42; 16]` and IV `[0x24; 16]` over the
/// // plaintext `b"hello world! this is my plaintext."`.
/// let ciphertext: Vec<u8> = {
///     let mut v = vec![0x24; 16]; // the IV, as it appears in the file
///     v.extend_from_slice(&[
///         0xc7, 0xfe, 0x24, 0x7e, 0xf9, 0x7b, 0x21, 0xf0, 0x7c, 0xbd, 0xd2, 0x6c,
///         0xb5, 0xd3, 0x46, 0xbf, 0xd2, 0x78, 0x67, 0xcb, 0x00, 0xd9, 0x48, 0x67,
///         0x23, 0xe1, 0x59, 0x97, 0x8f, 0xb9, 0xa5, 0xf9, 0x14, 0xcf, 0xb2, 0x28,
///         0xa7, 0x10, 0xde, 0x41, 0x71, 0xe3, 0x96, 0xe7, 0xb6, 0xcf, 0x85, 0x9e,
///     ]);
///     v
/// };
///
/// let plain = decrypt_cbc_128(&[0x42; 16], &ciphertext);
/// assert_eq!(plain, b"hello world! this is my plaintext.");
/// assert!(plain.len() < ciphertext.len());
/// assert!(ciphertext.len() >= MIN_CIPHERTEXT_LEN);
/// ```
///
/// Anything too short to carry an IV and one block yields no plaintext:
///
/// ```
/// use pdfce_core::crypto::aes::decrypt_cbc_128;
/// assert!(decrypt_cbc_128(&[0x42; 16], b"too short").is_empty());
/// ```
#[must_use]
pub fn decrypt_cbc_128(key: &[u8], data: &[u8]) -> Vec<u8> {
    // A key of the wrong length cannot be turned into an AES-128 key at all.
    // Algorithm 1 always yields 16 bytes for /AESV2 (`/Length` is 128, so
    // `min(16 + 5, 16)` is 16), so this is unreachable from the document path
    // -- but this is a `pub fn` and the check is one comparison.
    let Ok(key): Result<[u8; 16], _> = key.try_into() else {
        return Vec::new();
    };
    if data.len() < MIN_CIPHERTEXT_LEN {
        return Vec::new();
    }

    // Split the IV off the front. Both halves are in bounds: the length check
    // above guarantees at least IV_LEN + BLOCK_LEN bytes.
    let (iv, body) = data.split_at(IV_LEN);
    let Ok(iv): Result<[u8; 16], _> = iv.try_into() else {
        return Vec::new();
    };

    // Whole blocks only. A trailing partial block is malformed -- CBC is
    // defined over whole blocks -- and truncating it loses nothing that could
    // have been decrypted anyway.
    let whole = body.len() - (body.len() % BLOCK_LEN);
    let Some(body) = body.get(..whole) else {
        return Vec::new();
    };
    if body.is_empty() {
        return Vec::new();
    }

    let blocks: Vec<Block<Aes128>> = body
        .chunks_exact(BLOCK_LEN)
        .map(|c| {
            let mut b = Block::<Aes128>::default();
            b.copy_from_slice(c);
            b
        })
        .collect();
    let mut out = vec![Block::<Aes128>::default(); blocks.len()];

    let mut dec = Aes128CbcDec::new(&key.into(), &iv.into());
    if dec.decrypt_blocks_b2b(&blocks, &mut out).is_err() {
        // `out` is allocated at exactly `blocks.len()`, so the only documented
        // error (output too small) cannot happen. Degrade rather than panic:
        // this crate parses untrusted input and must not abort its host.
        return Vec::new();
    }

    let mut plain: Vec<u8> = out.into_iter().flatten().collect();
    strip_pkcs7(&mut plain);
    plain
}

/// Remove PKCS#7 padding **if and only if** it verifies, leaving the buffer
/// untouched otherwise.
///
/// §7.6.2 mandates the padding, so a conforming file always has it and always
/// takes the stripping branch. The non-verifying branch exists for damaged and
/// non-conforming files, and returning the bytes unstripped is what lets a
/// self-terminating filter like `/FlateDecode` still recover the stream — see
/// the module docs for the full argument, including why the padding-oracle
/// objection does not apply to a local file reader.
///
/// A valid pad is a final byte `n` in `1..=16` whose value is repeated in all
/// `n` trailing bytes, and which does not exceed the buffer.
fn strip_pkcs7(buf: &mut Vec<u8>) {
    let Some(&last) = buf.last() else { return };
    let n = usize::from(last);

    // 0 is never a valid pad length, and a pad longer than one block -- or
    // longer than the data -- is malformed.
    if n == 0 || n > BLOCK_LEN || n > buf.len() {
        return;
    }
    // Every one of the n trailing bytes must equal n. Checking only the last
    // byte would strip a plaintext that merely happens to end in 0x01.
    //
    // `.get()` rather than a slice index: the guard above already proves
    // `n <= buf.len()`, but this function decides how many bytes of a
    // possibly-hostile file to discard, and a checked access costs nothing to
    // keep the proof local instead of two statements away.
    let tail_start = buf.len() - n;
    if buf
        .get(tail_start..)
        .is_some_and(|tail| tail.iter().all(|&b| usize::from(b) == n))
    {
        buf.truncate(tail_start);
    }
}

#[cfg(test)]
// Tests slice and `expect` against fixtures they construct three lines above,
// where a panic IS the failure report and a checked access would only convert
// a precise line number into a silent `None`. The crate-level bans exist for
// the library's untrusted-input paths, which is not what any of this is.
#[allow(clippy::indexing_slicing, clippy::expect_used)]
mod tests {
    use super::*;
    use aes::cipher::BlockModeEncrypt;

    type Aes128CbcEnc = cbc::Encryptor<Aes128>;

    /// Encrypt the way a conforming producer would: random-looking IV in
    /// front, PKCS#7 padded body behind. Used to build ciphertexts whose
    /// plaintext is known, so the assertions below are about *this* module's
    /// framing rather than about AES itself.
    ///
    /// **The padding is applied by hand rather than with `cbc`'s `Pkcs7`
    /// helper**, for two reasons. It keeps the `block-padding` feature off in
    /// the real dependency (R24's spirit: no feature enabled that the shipping
    /// code does not need). And it means [`strip_pkcs7`] is being checked
    /// against a pad this test wrote out explicitly — §7.6.2's rule, spelled
    /// as code — rather than against the same library's opinion of the same
    /// rule, which would agree with itself even if both were wrong.
    fn encrypt(key: &[u8; 16], iv: &[u8; 16], plain: &[u8]) -> Vec<u8> {
        // PKCS#7: append `n` bytes of value `n`, where n is 1..=16 chosen so
        // the result is block-aligned. Note n is never 0 -- an already-aligned
        // plaintext gains a whole extra block.
        let n = BLOCK_LEN - (plain.len() % BLOCK_LEN);
        let mut buf = plain.to_vec();
        buf.extend(std::iter::repeat_n(
            u8::try_from(n).expect("n is 1..=16"),
            n,
        ));

        let mut blocks: Vec<Block<Aes128>> = buf
            .chunks_exact(BLOCK_LEN)
            .map(|c| {
                let mut b = Block::<Aes128>::default();
                b.copy_from_slice(c);
                b
            })
            .collect();
        Aes128CbcEnc::new(key.into(), iv.into()).encrypt_blocks(&mut blocks);

        let mut out = iv.to_vec();
        out.extend(blocks.into_iter().flatten());
        out
    }

    /// The whole point of the module: a conforming ciphertext round-trips to
    /// exactly its plaintext, IV and padding both removed.
    #[test]
    fn round_trips_a_conforming_ciphertext() {
        let key = [0x42u8; 16];
        let iv = [0x24u8; 16];
        for plain in [
            &b""[..],
            b"a",
            b"exactly sixteen!",
            b"hello world! this is my plaintext.",
            &[0xFFu8; 1000][..],
        ] {
            let ct = encrypt(&key, &iv, plain);
            assert_eq!(
                decrypt_cbc_128(&key, &ct),
                plain,
                "plain len {}",
                plain.len()
            );
        }
    }

    /// T5, stated as an assertion rather than a comment: the ciphertext is
    /// always at least 17 bytes longer than the plaintext, which is the fact
    /// that forces the object model to record a shortened span.
    #[test]
    fn ciphertext_is_always_at_least_17_bytes_longer() {
        let (key, iv) = ([0x01u8; 16], [0x02u8; 16]);
        for len in [0usize, 1, 15, 16, 17, 31, 32, 33] {
            let plain = vec![0xABu8; len];
            let ct = encrypt(&key, &iv, &plain);
            assert!(
                ct.len() > plain.len() + IV_LEN,
                "len {len}: ct {} vs plain {}",
                ct.len(),
                plain.len()
            );
            assert_eq!(decrypt_cbc_128(&key, &ct).len(), len);
        }
    }

    /// A plaintext that is already a whole number of blocks still gets a
    /// FULL block of padding. Getting this wrong strips 16 real bytes off
    /// every such stream -- and only off those, so it hides well.
    #[test]
    fn a_block_aligned_plaintext_still_carries_a_full_pad_block() {
        let (key, iv) = ([0x03u8; 16], [0x04u8; 16]);
        let plain = b"exactly sixteen!";
        let ct = encrypt(&key, &iv, plain);
        assert_eq!(ct.len(), IV_LEN + 32, "16 bytes of data + 16 of pad");
        assert_eq!(decrypt_cbc_128(&key, &ct), plain);
    }

    /// The IV is data, not configuration. Decrypting with the IV omitted --
    /// the mistake a reader makes when it looks for an `/IV` dictionary key
    /// and finds none -- corrupts the first block and leaves the rest intact,
    /// which is exactly why the bug survives casual testing.
    #[test]
    fn the_iv_comes_from_the_data_not_a_zero_default() {
        let (key, iv) = ([0x05u8; 16], [0x77u8; 16]);
        let plain = b"the first sixteen bytes are the ones that break";
        let ct = encrypt(&key, &iv, plain);

        assert_eq!(decrypt_cbc_128(&key, &ct), plain);

        // Same body, but a zero IV substituted: only block 0 differs.
        let mut zeroed = vec![0u8; IV_LEN];
        zeroed.extend_from_slice(&ct[IV_LEN..]);
        let wrong = decrypt_cbc_128(&key, &zeroed);
        assert_ne!(&wrong[..BLOCK_LEN], &plain[..BLOCK_LEN]);
        assert_eq!(&wrong[BLOCK_LEN..], &plain[BLOCK_LEN..]);
    }

    /// The documented leniency, direction one: padding that verifies is gone.
    #[test]
    fn valid_padding_is_stripped() {
        let mut b = b"data\x03\x03\x03".to_vec();
        strip_pkcs7(&mut b);
        assert_eq!(b, b"data");

        let mut full = vec![0x10u8; 16];
        strip_pkcs7(&mut full);
        assert!(full.is_empty(), "a full block of 0x10 is all padding");
    }

    /// The documented leniency, direction two: padding that does not verify
    /// costs nothing. Without this test the lenient branch could be deleted
    /// and every other test here would still pass.
    #[test]
    fn invalid_padding_keeps_every_byte() {
        // Last byte says 3, but the three trailing bytes are not all 3.
        let mut mismatched = b"data\x01\x02\x03".to_vec();
        strip_pkcs7(&mut mismatched);
        assert_eq!(mismatched, b"data\x01\x02\x03");

        // 0 is never a valid pad length.
        let mut zero = b"data\x00".to_vec();
        strip_pkcs7(&mut zero);
        assert_eq!(zero, b"data\x00");

        // A pad claiming more bytes than exist.
        let mut over = b"\x09\x09".to_vec();
        strip_pkcs7(&mut over);
        assert_eq!(over, b"\x09\x09");

        // Longer than one block is malformed even though the bytes agree.
        let mut long = vec![0x11u8; 17];
        strip_pkcs7(&mut long);
        assert_eq!(long.len(), 17);
    }

    /// A plaintext that merely *ends* in bytes resembling padding must not be
    /// truncated. This is the case a last-byte-only check gets wrong.
    #[test]
    fn a_plaintext_ending_in_one_is_not_mistaken_for_padding() {
        let (key, iv) = ([0x06u8; 16], [0x07u8; 16]);
        let plain = b"value\x01";
        let ct = encrypt(&key, &iv, plain);
        assert_eq!(decrypt_cbc_128(&key, &ct), plain);
    }

    /// Malformed inputs degrade to "no plaintext" instead of panicking. A
    /// crate that parses untrusted files must not abort its host.
    #[test]
    fn malformed_input_yields_no_plaintext_and_never_panics() {
        let key = [0x08u8; 16];
        assert!(decrypt_cbc_128(&key, b"").is_empty());
        assert!(
            decrypt_cbc_128(&key, &[0u8; 31]).is_empty(),
            "under the minimum"
        );
        assert!(
            decrypt_cbc_128(&key, &[0u8; 16]).is_empty(),
            "IV but no body"
        );
        // A key of the wrong length is refused rather than padded or truncated.
        assert!(decrypt_cbc_128(&[0u8; 5], &[0u8; 64]).is_empty());
        assert!(decrypt_cbc_128(&[0u8; 32], &[0u8; 64]).is_empty());
    }

    /// A trailing partial block is ignored, not fatal: the whole blocks in
    /// front of it still decrypt.
    #[test]
    fn a_trailing_partial_block_is_ignored() {
        let (key, iv) = ([0x09u8; 16], [0x0Au8; 16]);
        let plain = b"sixteen bytes ok";
        let mut ct = encrypt(&key, &iv, plain);
        ct.extend_from_slice(b"partial");
        assert_eq!(decrypt_cbc_128(&key, &ct), plain);
    }
}
