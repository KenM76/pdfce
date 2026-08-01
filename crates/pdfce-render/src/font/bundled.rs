//! # The bundled standard-14 substitute faces (decision 004 §4.2/§6.5)
//!
//! Fourteen bare-CFF faces from pdfium's `chromefontdata` set
//! (Foxit-origin, BSD-3-Clause via Google's pdfium grant; 264,741
//! bytes total). Extracted by `tools/extract-base14/extract.py`;
//! provenance — source commit, per-file SHA-256, verbatim license —
//! lives in `assets/fonts/PROVENANCE.md` (rule R22: verified, never
//! asserted). Metric fidelity vs the Core-14 AFMs is documented in
//! decision 004 §3.5 (Courier/Symbol/Dingbats exact; Helvetica 4 and
//! Times 1 peripheral deltas, named in the conformance test).
//!
//! `include_bytes!` keeps the renderer I/O-free (R19) and the WASM
//! fork working with no shell support. The two pdfium
//! multiple-master fallback faces (FoxitSansMM/FoxitSerifMM) are
//! deliberately NOT bundled (004 declined them for Pass 1).

use std::collections::HashMap;

use super::{FallbackKey, FontData};

macro_rules! face {
    ($file:literal) => {
        FontData::from_static(include_bytes!(concat!("../../assets/fonts/", $file)))
    };
}

/// The full bundled map: every [`FallbackKey`] slot filled.
#[must_use]
pub fn faces() -> HashMap<FallbackKey, FontData> {
    use FallbackKey as K;
    HashMap::from([
        (K::Sans, face!("FoxitSans.cff")),
        (K::SansBold, face!("FoxitSansBold.cff")),
        (K::SansItalic, face!("FoxitSansItalic.cff")),
        (K::SansBoldItalic, face!("FoxitSansBoldItalic.cff")),
        (K::Serif, face!("FoxitSerif.cff")),
        (K::SerifBold, face!("FoxitSerifBold.cff")),
        (K::SerifItalic, face!("FoxitSerifItalic.cff")),
        (K::SerifBoldItalic, face!("FoxitSerifBoldItalic.cff")),
        (K::Fixed, face!("FoxitFixed.cff")),
        (K::FixedBold, face!("FoxitFixedBold.cff")),
        (K::FixedItalic, face!("FoxitFixedItalic.cff")),
        (K::FixedBoldItalic, face!("FoxitFixedBoldItalic.cff")),
        (K::Symbol, face!("FoxitSymbol.cff")),
        (K::Dingbats, face!("FoxitDingbats.cff")),
    ])
}
