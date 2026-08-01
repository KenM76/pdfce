//! # FlateDecode (ISO 32000-1 §7.4.4) — inflate + un-predict
//!
//! §7.4.4.1 delegates entirely to RFC 1950/1951: stream data is a
//! **zlib stream** (2-byte header, deflate body, Adler-32 trailer),
//! not raw deflate. Decoding is two stages when `/Predictor > 1`:
//! inflate first, then un-predict the inflated bytes
//! (`crate::filters::predictor`). Spec source: `filter__flate.md` in
//! the PDF-spec RAG (incl. Table 8 parameter defaults).
//!
//! ## Bomb guard
//!
//! Inflation is **incremental**: fixed-size chunks accumulate against
//! [`super::MAX_DECODED_LEN`] and the decode aborts the moment the
//! ceiling is crossed — a hostile 1 MB stream claiming ~1 GB of output
//! (deflate's ~1032:1 worst case) costs pdfce at most the ceiling, not
//! the claim. This is the concrete §10.1 obligation ROADMAP Pass 1
//! names for this filter.
//!
//! ## Failure semantics
//!
//! Corrupt zlib data and truncated streams both `Err` — see the
//! fail-clean contract in `super`. A zlib Adler-32 mismatch surfaces
//! however `miniz_oxide` reports it; pdfce does not add a
//! tolerate-bad-checksum path in Pass 1 (real-world checksum sloppiness
//! is a documented candidate for a *labeled* tolerance later,
//! `filter__flate.md` gotchas → `C:\personal_rag\pdf\`).

use flate2::{Decompress, FlushDecompress, Status};

use super::{FilterError, MAX_DECODED_LEN, predictor};
use crate::object::Dict;

/// Inflate `data` and apply any predictor from `parms` (Table 8:
/// `Predictor`/`Colors`/`BitsPerComponent`/`Columns`; `EarlyChange` is
/// LZW-only and ignored here).
///
/// # Errors
///
/// [`FilterError`] — corrupt/truncated zlib data, ceiling crossed, or
/// invalid/inconsistent predictor parameters.
pub fn decode(data: &[u8], parms: Option<&Dict>) -> Result<Vec<u8>, FilterError> {
    let inflated = inflate_bounded(data)?;
    match predictor::Params::from_dict(parms)? {
        None => Ok(inflated),
        Some(p) => predictor::unpredict(inflated, &p),
    }
}

/// Incremental, ceiling-bounded zlib inflation (module docs).
///
/// Uses the raw [`Decompress`] state machine rather than the
/// `std::io::Read` wrapper deliberately: the wrapper reports a
/// truncated stream as a clean `Ok(0)` EOF, which is indistinguishable
/// from success — the exact silent-partial-data failure mode the
/// fail-clean contract forbids. With the raw API, completeness is
/// explicit: only [`Status::StreamEnd`] (the deflate final block seen
/// and checksum consumed) is success; input exhaustion before that is
/// [`FilterError::Truncated`].
fn inflate_bounded(data: &[u8]) -> Result<Vec<u8>, FilterError> {
    // 64 KiB chunks: large enough to amortize call overhead, small
    // enough that the ceiling overshoot is negligible.
    const CHUNK: usize = 64 * 1024;

    let mut inflater = Decompress::new(true); // true = expect zlib wrapper (RFC 1950)
    let mut out: Vec<u8> = Vec::new();
    let mut chunk = [0u8; CHUNK];
    loop {
        let before_in = inflater.total_in();
        let before_out = inflater.total_out();
        let remaining = usize::try_from(before_in)
            .ok()
            .and_then(|n| data.get(n..))
            .unwrap_or(&[]);
        // FlushDecompress::None, NOT Finish: Finish demands the whole
        // output fit the provided buffer in one call (flate2 contract)
        // and errors otherwise — incompatible with chunked decoding.
        // Stream completion is detected via Status::StreamEnd instead.
        let status = inflater
            .decompress(remaining, &mut chunk, FlushDecompress::None)
            .map_err(|e| FilterError::Corrupt {
                filter: "FlateDecode",
                detail: e.to_string(),
            })?;
        let produced = usize::try_from(inflater.total_out() - before_out).unwrap_or(0);
        if out.len().saturating_add(produced) > MAX_DECODED_LEN {
            return Err(FilterError::OutputTooLarge);
        }
        out.extend_from_slice(chunk.get(..produced).unwrap_or(&[]));

        match status {
            Status::StreamEnd => return Ok(out),
            Status::Ok | Status::BufError => {
                let consumed = inflater.total_in() - before_in;
                if consumed == 0 && produced == 0 {
                    // No progress possible and the stream never
                    // reached its end marker: the input is truncated.
                    return Err(FilterError::Truncated {
                        filter: "FlateDecode",
                    });
                }
            }
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use crate::object::{Name, Object};
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::io::Write as _;

    fn deflate(data: &[u8]) -> Vec<u8> {
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(data).unwrap();
        enc.finish().unwrap()
    }

    #[test]
    fn roundtrip_no_predictor() {
        let original = b"q 1 0 0 1 72 712 cm BT ET Q".repeat(100);
        assert_eq!(decode(&deflate(&original), None).unwrap(), original);
    }

    #[test]
    fn corrupt_zlib_errs_never_passes_raw_bytes() {
        // THE regression test for the fail-clean contract (decision
        // 001 §6.1 item 4): corrupt input must produce Err — the
        // documented upstream antipattern returned the raw bytes as if
        // decoded.
        let garbage = b"this is not zlib data at all";
        let result = decode(garbage, None);
        assert!(matches!(result, Err(FilterError::Corrupt { .. })));
    }

    #[test]
    fn truncated_stream_errs() {
        let full = deflate(&b"some reasonable content ".repeat(50));
        let cut = &full[..full.len() / 2];
        let result = decode(cut, None);
        assert!(matches!(
            result,
            Err(FilterError::Truncated { .. } | FilterError::Corrupt { .. })
        ));
    }

    #[test]
    fn decompression_bomb_is_aborted_at_ceiling() {
        // ~300 MiB of zeros compresses to well under 1 MiB; decoding
        // must abort at MAX_DECODED_LEN, not allocate the full claim.
        let bomb_plain = vec![0u8; super::MAX_DECODED_LEN + 1024];
        let bomb = deflate(&bomb_plain);
        assert!(bomb.len() < 1024 * 1024, "bomb should compress tiny");
        assert_eq!(
            decode(&bomb, None).unwrap_err(),
            FilterError::OutputTooLarge
        );
    }

    #[test]
    fn predictor_params_flow_through() {
        // PNG Up (tag 2) over two 4-byte rows — the xref-stream shape
        // (Predictor 12, Columns 4, defaults elsewhere).
        // Reconstructed rows: [1,2,3,4] then [11,22,33,44].
        // Up-filtered:        [1,2,3,4] then [10,20,30,40], each row
        // prefixed by tag 2.
        let filtered: Vec<u8> = [&[2u8][..], &[1, 2, 3, 4], &[2], &[10, 20, 30, 40]].concat();
        let mut parms = Dict::new();
        parms.insert(Name::from(b"Predictor"), Object::Integer(12));
        parms.insert(Name::from(b"Columns"), Object::Integer(4));
        let out = decode(&deflate(&filtered), Some(&parms)).unwrap();
        assert_eq!(out, vec![1, 2, 3, 4, 11, 22, 33, 44]);
    }
}
