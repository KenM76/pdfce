//! # Image XObjects and inline images → RGBA pixmaps (ISO 32000-1 §8.9)
//!
//! Turns a PDF *sampled image* — an image XObject (`/Subtype /Image`)
//! or an inline image (`BI`/`ID`/`EI`) — into a [`tiny_skia::Pixmap`]
//! the interpreter can paint through the CTM. Spec sources:
//! `iso32000__s__8.9.md` (Table 89, image space, sample packing,
//! stencil masks), `iso32000__s__8.9.5.2.md` (Table 90 `Decode`
//! defaults, the linear transform, the `[1 0]` inversion, image-mask
//! polarity), `iso32000__s__8.9.7.md` (inline-image abbreviations),
//! `color__indexed.md` (§8.6.6.3 palettes), `color__iccbased.md`
//! (the `N`-component fallback) in the PDF-spec RAG.
//!
//! ## This module does NOT place the image
//!
//! Placement is entirely the CTM's job (§8.9.4): "the unit square of
//! user space… corresponds to the boundary of the image in image
//! space." This module only produces `Width × Height` RGBA texels with
//! **row 0 at the top**, exactly as §8.9.3 orders the samples. The
//! y-flip that §8.9.4's implicit matrix `[1/w 0 0 −1/h 0 1]` describes
//! is applied by the caller ([`crate::interpret`]) when it builds the
//! pattern transform. Keeping the flip out of here means the pixmap is
//! in the same orientation a PNG would be, which is what makes the
//! pixel-level tests readable.
//!
//! ## The decode pipeline, in the order §8.9.5.2 mandates
//!
//! ```text
//! raw stream bytes
//!   → /Filter chain + terminal codec      pdfce_core::image_codec
//!   → unpack to BitsPerComponent integers §8.9.3 (rows byte-padded)
//!   → Decode transform                    §8.9.5.2 (linear, may invert)
//!   → colour-space conversion             §8.6 / §8.6.6.3
//!   → RGBA texel
//! ```
//!
//! Getting this order wrong is the classic image bug: applying `Decode`
//! after colour conversion silently breaks `Indexed` (whose `Decode`
//! output *is* a palette index, not a colour) and every inverted
//! stencil mask.
//!
//! ## Where the samples come from (decision 005 R23/R26)
//!
//! The first stage is [`pdfce_core::image_codec::decode_image`], not
//! `filters::decode_stream`. It runs the byte-stream *prefix* of the
//! `/Filter` chain and then dispatches the single terminal image codec
//! (`DCTDecode`, `CCITTFaxDecode`, `JBIG2Decode`, `JPXDecode`), handing
//! back a [`CodedImage`] — samples **plus** the geometry and colour
//! model the codestream itself declares.
//!
//! Everything downstream of that first stage is unchanged and still
//! lives here: sample unpacking, `/Decode`, `/ColorSpace` resolution,
//! `Indexed` palettes, stencil masks, RGBA texels. **Core decodes and
//! models; render paints.** In particular the codec layer applies no
//! `/Decode` array and no "Adobe CMYK inversion" of its own (rules R26
//! and R29 — decision 006 settled that no such inversion exists in any
//! shipping PDF engine), so a CMYK JPEG's polarity is settled here, by
//! `/Decode`, exactly as §8.9.5.2 says it should be. The signed-slope
//! ramp below is therefore load-bearing: `/Decode [1 0 …]` IS the
//! sanctioned inversion mechanism, and it must survive any refactor.
//!
//! ## When the codestream and the dictionary disagree
//!
//! A JPEG whose SOF geometry differs from `/Width`//`/Height` is a
//! producer bug. pdfce splits the difference along the only seam that
//! neither shears the picture nor moves it:
//!
//! - **the dictionary wins for placement** — the pixmap is `/Width` ×
//!   `/Height`, because §8.9.4 maps the image onto the unit square of
//!   user space regardless of how many samples it turns out to contain;
//! - **the codestream wins for sample reading** — the row stride comes
//!   from the codec's own width, component count and bit depth, because
//!   that is the physical layout of the bytes in hand.
//!
//! The divergence is counted in [`ImageNotes::codec_geometry_mismatch`]
//! and surfaced, never silently absorbed.
//!
//! ## `JPXDecode` inverts three of Table 89's rules, and only three
//!
//! `JPXDecode` is the one filter for which the image dictionary is not
//! simply authoritative, and the three exceptions are exact. They are
//! implemented in [`decode_sampled`], each at the point where the
//! ordinary rule would otherwise apply:
//!
//! | Entry | Ordinary image | `JPXDecode` |
//! |---|---|---|
//! | `/ColorSpace` | **Required**; missing is malformed. | **Optional.** Present → the dictionary still wins and the codestream's colour specifications "shall be ignored". Absent → [`codestream_space`] supplies it from the codec's declared colour model, per §7.4.9's fallback ladder. |
//! | `/BitsPerComponent` | **Required**, one of 1/2/4/8/16. | **"Optional and shall be ignored if present."** The dictionary is not consulted at all; the codec's delivered depth is used. Honouring a stated value is not merely redundant, it is wrong. |
//! | `/Decode` | Applied (§8.9.5.2). | **Ignored** — "except in the case where the image is treated as a mask; that is, when `ImageMask` is true", which is the [`decode_stencil`] branch this function never reaches. |
//!
//! The trap in the *other* direction is worth naming because it looks
//! like the same rule: "the codestream is authoritative for JPX" is
//! **false** as a blanket statement. A present `/ColorSpace` wins. Only
//! where the dictionary is silent (colour) or explicitly disqualified
//! (bit depth, `Decode`) does the codestream take over. Getting that
//! backwards mis-colours precisely the files whose producer bothered to
//! tag them.
//!
//! `/Width` and `/Height` are **not** on that list: §7.4.9 requires them
//! to "match" the codestream but supplies no conflict-resolution rule,
//! so the ordinary dictionary-for-placement / codestream-for-stride
//! split above continues to govern them, with the divergence counted.
//!
//! ## Stencil masks are a separate path on purpose
//!
//! An image with `/ImageMask true` (§8.9.6.2) carries **no colour at
//! all** — its 1-bit samples say only "mark the page with the current
//! non-stroking colour here" or "leave the previous contents alone."
//! `Decode` is a polarity switch for it, not a colour transform, and
//! the default `[0 1]` means **0 = ink** (the opposite of the usual
//! bitmap intuition). Trying to unify this with the ordinary path via a
//! synthetic `DeviceGray` space gets both the polarity and the
//! transparency wrong, so [`decode`] branches early and never mixes
//! them.
//!
//! ## Honesty (`fuzzy, never sneaky`)
//!
//! Nothing here approximates. An image whose data needs a filter pdfce
//! has not implemented ([`ImageError::UnsupportedFilter`]) or a colour
//! space out of this slice's scope
//! ([`ImageError::UnsupportedColorSpace`]) is **not drawn at all** and
//! is counted by the caller; it is never substituted with a grey box or
//! a guessed colour. Softer divergences that still produce a
//! *recognizable* image — a `/SMask` this slice ignores, a truncated
//! sample array, a palette index past the end of a short lookup table —
//! are drawn and reported through [`ImageNotes`].
//!
//! ## Resource ceiling (ARCHITECTURE.md §10.1 — pdfce policy)
//!
//! `Width` and `Height` are attacker-controlled integers, and the RGBA
//! buffer is `4 × W × H` bytes, so the product is checked **before any
//! allocation or decode** against [`MAX_IMAGE_PIXELS`].

// decision 018: read paths take a `DocumentView` (graph + byte source), so
// the same code renders a loaded file or an editing session's unsaved state.
use pdfce_core::filters::{self, FilterError};
use pdfce_core::graph::ObjectGraph;
use pdfce_core::image_codec::{self, Codec, CodecColorModel, CodedImage, ImageCodecError};
use pdfce_core::object::{Dict, Object};
use pdfce_core::settings::CmykIntent;
use pdfce_core::view::DocumentView;
use tiny_skia::{Pixmap, PremultipliedColorU8};

use crate::color::ColorDiagnostics;
use crate::font::RenderPolicy;
use crate::gstate::Rgb;
use crate::mask::{self, AlphaPlane};

/// Maximum `Width × Height` accepted for a single image (pdfce policy,
/// ARCHITECTURE.md §10.1).
///
/// Re-exported from [`pdfce_core::image_codec`] rather than restated,
/// so the rasterizer's ceiling and the codec layer's ceiling are the
/// same number by construction. Two independently-maintained copies of
/// a guard is how a guard quietly stops guarding.
pub use pdfce_core::image_codec::MAX_IMAGE_PIXELS;

/// Where an image came from, which decides §8.9.7's stricter rules.
///
/// An inline image may not use `JBIG2Decode` (§7.4.7 states it
/// outright) or `JPXDecode`; `DCT` and `CCF` *are* legal inline filter
/// abbreviations (Table 94). Passing this in rather than sniffing it
/// keeps the rule where the spec puts it — on the *construct*, not on
/// the data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageOrigin {
    /// An image XObject reached through `Do` (§8.8).
    XObject,
    /// An inline image (`BI`/`ID`/`EI`, §8.9.7).
    Inline,
}

/// Guard on `Indexed` colour-space nesting while resolving a
/// `/ColorSpace` entry.
///
/// A colour space can legitimately nest two deep (`Indexed` over
/// `ICCBased`), and a named resource can add one hop per lookup. A
/// self-referential `/ColorSpace << /CS0 [/Indexed /CS0 …] >>` would
/// otherwise recurse forever (ARCHITECTURE.md §10.1's cycle rule).
const MAX_COLORSPACE_DEPTH: usize = 8;

/// Why an image could not be turned into pixels at all.
///
/// Every variant means **nothing was drawn**. The caller counts these
/// in `Diagnostics::images_unsupported` and carries on with the rest of
/// the page — an image pdfce cannot decode is a fidelity shortfall, not
/// a reason to abandon a page a reader could otherwise show.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ImageError {
    /// The sample data needs a filter this build does not implement, or
    /// its data is corrupt. The payload names it.
    #[error("image data could not be decoded: {0}")]
    UnsupportedFilter(String),
    /// The image uses a **codec** this build does not implement
    /// (`CCITTFaxDecode`, `JBIG2Decode`, `JPXDecode` until Passes 2.2
    /// and 2.3 land), or one that §8.9.7 forbids in an inline image.
    /// Separate from [`ImageError::UnsupportedFilter`] because "which
    /// codec do I need?" has a specific, actionable answer.
    #[error("{0}")]
    CodecUnsupported(String),
    /// The codec is implemented but a specific **sub-feature** of it is
    /// not — arithmetic-coded JPEG, 12-bit JPEG, an Adobe transform
    /// byte outside 0–2. The payload is a stable diagnostic key such as
    /// `"DCT/arithmetic"` so occurrences can be counted **by name**
    /// (decision 005 rule R27), never rolled into a generic "decode
    /// failed."
    #[error("unsupported codec feature: {0}")]
    CodecFeature(&'static str),
    /// The `/ColorSpace` is outside this slice's scope (`Lab`,
    /// `Separation`, `DeviceN`, `Pattern`, or an unresolvable name).
    /// The payload names it.
    #[error("image colour space {0} is not supported")]
    UnsupportedColorSpace(String),
    /// Table 89's "entries inconsistent with each other" rule: a
    /// missing/zero `Width`/`Height`, a `BitsPerComponent` outside
    /// {1,2,4,8,16}, an image mask with a bit depth other than 1, and
    /// so on. The payload says which.
    #[error("malformed image dictionary: {0}")]
    Malformed(&'static str),
    /// `Width × Height` exceeds [`MAX_IMAGE_PIXELS`] (pdfce guard).
    #[error("image exceeds MAX_IMAGE_PIXELS ({MAX_IMAGE_PIXELS} pixels)")]
    TooLarge,
}

/// Which of §8.9.6.1's transparency mechanisms supplied an image's alpha.
///
/// Exactly one can be in force per image — `/SMask` and `/Mask` are
/// separate entries and `/Mask` is either a stream or an array, never
/// both — so this is an `Option<MaskApplied>` on [`ImageNotes`] rather
/// than a set of independent flags. The precedence when a document names
/// more than one is documented on [`decode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MaskApplied {
    /// `/SMask` — a separate greyscale image, one continuous alpha per
    /// sample (§8.9.5 Table 89, §11.6.5.3). The mechanism a transparent
    /// PNG's alpha channel becomes.
    SoftMask,
    /// `/Mask` as a stream — a separate 1-bit stencil selecting which
    /// base texels paint (§8.9.6.3). Binary, never partial.
    Stencil,
    /// `/Mask` as an array — ranges of the base image's own
    /// pre-`/Decode` samples that vanish (§8.9.6.4). The mechanism a
    /// single-transparent-colour PNG (`tRNS` on a truecolour image)
    /// becomes.
    ColourKey,
    /// A JPX codestream's own opacity channel, switched on by
    /// `/SMaskInData 1` (Table 89). Not a dictionary entry at all — the
    /// alpha travels inside the image's own bytes.
    EmbeddedAlpha,
}

impl MaskApplied {
    /// A stable, greppable name for the diagnostics surfaces.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::SoftMask => "smask",
            Self::Stencil => "stencil",
            Self::ColourKey => "colour-key",
            Self::EmbeddedAlpha => "jpx-embedded-alpha",
        }
    }
}

/// Divergences that did **not** stop the image from being drawn.
///
/// Distinct from [`ImageError`] because the operator's question is
/// different: an error means "this image is missing from the page", a
/// note means "this image is on the page but is not exactly what the
/// document specifies."
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ImageNotes {
    /// The sample array was shorter than `stride × Height`; the missing
    /// samples were read as 0. (§8.9.3 gives an exact length; a short
    /// stream is malformed, but refusing to draw the 90% that *is*
    /// present helps nobody.)
    pub truncated: bool,
    /// Which transparency mechanism produced this image's alpha, or
    /// `None` for an image that is opaque because the document says so.
    ///
    /// Census, not shortfall: every variant means pdfce **did** the
    /// work. The shortfall twin is
    /// [`mask_refused`](ImageNotes::mask_refused).
    pub mask_applied: Option<MaskApplied>,
    /// A `/SMask` or `/Mask` was present and could **not** be turned
    /// into alpha, so the base image was drawn **fully opaque** —
    /// visually wrong wherever the mask would have hidden something.
    /// The payload is [`crate::mask::MaskRefusal::key`], a stable name
    /// so occurrences can be counted **by reason** (rule R27).
    ///
    /// This is the residue of the pre-transparency build's blanket
    /// `mask_deferred`: that note fired for every masked image, because
    /// none were composited. It now fires only for the ones pdfce
    /// genuinely could not handle.
    pub mask_refused: Option<&'static str>,
    /// The mask's pixel dimensions differed from the base image's, so
    /// its samples were point-sampled across the base's grid
    /// (§8.9.6.3: "need not have the same resolution … their boundaries
    /// on the page will coincide"). Conformant and common; recorded
    /// because a resampled mask cannot be pixel-exact and a parity
    /// investigation should not have to re-derive why.
    pub mask_resampled: bool,
    /// The `/SMask` carried `/Matte` (Table 146) and pdfce **undid the
    /// preblend** per §11.6.5.3's `c = m + (c′ − m)/α`.
    ///
    /// Census, not shortfall — but recorded, because a `/Matte` image's
    /// partially-transparent samples are reconstructed by a division
    /// that amplifies quantisation error by `1/α`. A parity
    /// investigation that finds a `/Matte` image disagreeing with
    /// another engine in its near-transparent fringes should know that
    /// before spending an afternoon on it.
    pub matte_undone: bool,
    /// A `/Matte` was present and **not** undone, with the reason.
    ///
    /// The alpha is applied either way — that half is conformant
    /// regardless — so the picture's *shape* is right and only the
    /// colours in the partially-transparent regions stay shifted toward
    /// the matte colour. Reasons: `"matte/dimension-mismatch"` (Table
    /// 145 makes equal dimensions a `shall` when `/Matte` is present, so
    /// a mismatch means the file is wrong about one of the two, and
    /// dividing by a resampled α would use the wrong α for every
    /// sample), `"matte/indexed"` (spec ambiguity `SM-A4`: Table 146
    /// counts `n` from the parent's `/ColorSpace`, which for `Indexed`
    /// is **1**, while §11.6.5.3 requires the *colour table* values to
    /// be preblended, which needs the base space's `n` — the two rules
    /// contradict and pdfce will not pick a side silently), and
    /// `"matte/length"` (the array is not `n` long).
    pub matte_not_undone: Option<&'static str>,
    /// At least one sample indexed past the end of a short `Indexed`
    /// lookup table and was painted black (`color__indexed.md`: real
    /// producers trim trailing unused palette entries).
    pub palette_out_of_range: bool,
    /// The image's colour space was `/Separation /None` or an all-`/None`
    /// `/DeviceN`, so **nothing was painted** — the image is fully
    /// transparent and the page shows through it.
    ///
    /// §8.6.6.4/.5: such a colorant "shall never be painted on the page".
    /// This is pdfce OBEYING the standard, and it is recorded for R183's
    /// reason: a picture that is correctly absent is otherwise
    /// indistinguishable from one that failed to decode.
    ///
    /// ★ Measured 2026-08-17: **pdfium paints this BLACK.** pdfce is
    /// deliberately right and the reference renderer is wrong, which is a
    /// finding rather than a failure — but it means any pixel-parity run
    /// containing a `/None` image will show a maximal divergence that is
    /// pdfce's correctness, not its defect.
    pub colorant_none_suppressed: bool,
    /// The image was decoded through a `crate::color::ColorSpace` that
    /// pdfce converts by its OWN colorimetry rather than by a colour
    /// management engine — `Lab`, `CalGray` and `CalRGB`, whose XYZ→sRGB
    /// step is documented as pdfce's engineering choice (Bradford
    /// adaptation to D65, the sRGB matrix and transfer function, no
    /// rendering intent and no gamut mapping).
    ///
    /// Disclosed because it is precisely the kind of divergence that
    /// otherwise lands in a parity harness's *unexplained* bucket and
    /// costs somebody an afternoon: two engines can both be defensible
    /// here and still differ by tens of levels in the saturated corners.
    pub uncalibrated_colorimetry: Option<&'static str>,
    /// The `/Decode` array's length was not `2 × components`, so the
    /// Table 90 default was used instead (`iso32000__s__8.9.5.2.md`
    /// recommends this over truncating, which silently mis-tints).
    pub decode_array_ignored: bool,
    /// The codestream's own geometry disagreed with the image
    /// dictionary (`/Width`, `/Height`, `/BitsPerComponent`, or the
    /// component count implied by `/ColorSpace`). The image was still
    /// drawn — see the module docs for which side wins what — but one
    /// of the two is wrong about the file (decision 005 §6.4).
    pub codec_geometry_mismatch: bool,
    /// This was a 4-component DCT image in YCCK storage (effective
    /// transform 1/2) — the **benign census** half of decision 006
    /// §4.4's split. The mandated YCCK→CMYK inverse recovers true ink
    /// directly (TN #5116 §13.1) and carries no polarity ambiguity;
    /// verified pixel-identical to pdfium across the corpus. Volume,
    /// not shortfall — no warning attaches.
    pub dct_cmyk_image: bool,
    /// This was a 4-component DCT image with effective transform **0**
    /// and **no `/Decode`** — the one shape where the undocumented
    /// Photoshop inverted-storage convention could make it render as
    /// its own negative with nothing to disambiguate (decision 006
    /// rule **R30**). Reported, never repaired: the image was drawn
    /// from the raw samples, exactly as pdfium/pdf.js/MuPDF/Poppler
    /// draw it; pdfce differs only in *saying so*.
    pub dct_cmyk_polarity_unverifiable: bool,
    /// This JPX image declares `/SMaskInData 2` — Table 89's "colour
    /// channels that have been **preblended with a background**" plus
    /// an opacity channel that would need a `Matte` entry to undo.
    ///
    /// Recognized and deferred, never approximated. The image *was*
    /// drawn, from the preblended colour channels exactly as stored —
    /// which is what it genuinely looks like composited over that
    /// backdrop, so the picture is right wherever it is opaque and
    /// shows the backdrop where it is not.
    ///
    /// **Still deferred after the transparency Pass, and for a reason
    /// that has nothing to do with clause 11.** §11.6.5.3's
    /// un-premultiply is now implemented ([`crate::mask::undo_matte`])
    /// and would apply here unchanged — but it needs the opacity channel
    /// and the matte colour, and neither is available: Table 89's
    /// `/SMaskInData 2` names a *premultiplied* opacity channel type
    /// that `hayro-jpeg2000` does not parse, so the codec layer leaves
    /// `CodedImage::embedded_alpha` as `None`, and a JPX codestream
    /// carries no `/Matte` (that entry lives on a soft-mask image
    /// dictionary, which this construct does not have). The blocker is a
    /// decoder gap, not a spec gap.
    ///
    /// Separate from [`mask_refused`](ImageNotes::mask_refused) because
    /// the divergence is different in kind: a refused mask means
    /// "correct colours, missing transparency", this means "the colours
    /// themselves have a backdrop mixed into them".
    pub jpx_smask_in_data_preblended: bool,
    /// LZW framing anomalies in the byte-stream part of the chain — a
    /// stream with no `ClearCode`, or one that ended with no
    /// `EndOfInformation`. Both recovered, both non-conformant.
    pub lzw_framing_anomalies: usize,
}

/// A decoded image: `Width × Height` RGBA texels, row 0 at the **top**
/// (module docs), plus what diverged.
#[derive(Debug)]
pub struct DecodedImage {
    /// The texels. Premultiplied RGBA, as tiny-skia requires.
    pub pixmap: Pixmap,
    /// Divergences that still produced pixels.
    pub notes: ImageNotes,
}

/// Decode an image XObject or inline image into RGBA texels.
///
/// - `dict` is the image dictionary (Table 89) or the inline image's
///   already-normalized parameter dictionary (Table 93 abbreviations
///   expanded by `pdfce_core::content`, so exactly one key spelling
///   reaches this function).
/// - `raw` is the **still-encoded** sample data; the `/Filter` chain and
///   any terminal image codec are run here so that `/DecodeParms`
///   predictors and codec dispatch are handled by the one
///   implementation in `pdfce-core`.
/// - `resources` is the current resource dictionary, needed for a
///   `/ColorSpace` given as a *name* referring to the `/ColorSpace`
///   subdictionary (§8.9.7 permits this for inline images from PDF 1.2,
///   and image XObjects have always permitted it).
/// - `fill` is the current **non-stroking** colour, used only by the
///   stencil-mask path (§8.9.6.2) and ignored otherwise.
/// - `origin` selects §8.9.7's stricter inline-image filter rules.
///
/// ## Transparency precedence when a document names more than one
///
/// §8.9.6 is silent on this, and `iso32000__s__8.9.5.2.md` carried it as
/// an open gap until 2026-08-08 — but the answer is **normative and
/// verbatim**, it simply lives in Table 89's `SMask` row rather than in
/// §8.9.6 where a reader would look for it:
///
/// > "shall **override** the current soft mask in the graphics state,
/// > **as well as the image's `Mask` entry, if any**. However, the other
/// > transparency-related graphics state parameters — blend mode and
/// > alpha constant — shall remain in effect."
///
/// §11.6.4.3 says the same thing independently. So co-presence is
/// **legal, not an error**: the loser is ignored, and — R34 — must still
/// round-trip byte-identical, which it does here because nothing in this
/// path writes.
///
/// The ladder pdfce walks:
///
/// 1. **`/ImageMask true`** short-circuits everything — such an image
///    has no colour, so there is nothing for a mask to make transparent
///    (and §8.9.6.2 forbids it carrying `/Mask` at all).
/// 2. **`/SMask`** (and `/SMaskInData` ≠ 0), by the quotation above.
/// 3. **`/Mask`** — stream (stencil) or array (colour key), dispatched
///    on the resolved type as §8.9.6.1 requires.
/// 4. **The JPX codestream's own opacity channel**, when
///    `/SMaskInData 1` switched it on (Table 89).
///
/// Below all of that sits the ExtGState soft mask and then 1.0, neither
/// of which this Pass implements — see the module docs of
/// [`crate::mask`] and `iso32000__ref__image_transparency.md`.
///
/// A mechanism that is present but refused (see
/// [`crate::mask::MaskRefusal`]) does **not** fall through to the next
/// one: the document named a mechanism, pdfce could not honour it, and
/// quietly substituting a different one would be exactly the kind of
/// plausible-looking guess `fuzzy, never sneaky` forbids. The refusal is
/// recorded in [`ImageNotes::mask_refused`] and the image draws opaque.
///
/// # Errors
///
/// [`ImageError`] — see its variants. Every one means "nothing drawn".
/// A mask that cannot be decoded is **not** one of them: the picture is
/// still drawn, and the shortfall is a note rather than an error.
pub fn decode(
    doc: &DocumentView<'_>,
    dict: &Dict,
    raw: &[u8],
    resources: &Dict,
    fill: Rgb,
    origin: ImageOrigin,
    policy: RenderPolicy,
) -> Result<DecodedImage, ImageError> {
    let width = positive_dimension(doc, dict, b"Width")?;
    let height = positive_dimension(doc, dict, b"Height")?;

    // Ceiling FIRST — before the filter chain runs and before any
    // pixmap allocation (module docs / ARCHITECTURE.md §10.1). The
    // codec layer applies the same ceiling to the *codestream's* own
    // declared geometry; this one covers the dictionary's, which is
    // what sizes the pixmap.
    if u64::from(width).saturating_mul(u64::from(height)) > MAX_IMAGE_PIXELS {
        return Err(ImageError::TooLarge);
    }

    // `DCT-A1` (R169) travels with the decode rather than being read
    // from anywhere ambient: the polarity rule changes the SAMPLES, so a
    // cached or re-run decode under a different setting must be a
    // different call, not the same call with a different global.
    let coded = image_codec::decode_image_view_with(
        doc,
        dict,
        raw,
        origin == ImageOrigin::Inline,
        policy.cmyk_jpeg_polarity,
    )
    .map_err(map_codec_error)?;

    let mut notes = ImageNotes {
        codec_geometry_mismatch: coded.notes.geometry_mismatch,
        dct_cmyk_image: coded.notes.cmyk_image,
        dct_cmyk_polarity_unverifiable: coded.notes.cmyk_polarity_unverifiable,
        jpx_smask_in_data_preblended: coded.notes.jpx_smask_in_data_preblended,
        lzw_framing_anomalies: coded.notes.lzw_framing_anomalies,
        ..ImageNotes::default()
    };
    // §8.9.6.2: an image mask is a completely different object — no
    // colour space, no colour conversion, `Decode` is a polarity bit.
    // Step 1 of the precedence ladder in this function's docs: it
    // short-circuits before any mask is even looked for.
    if matches!(
        dict.get(b"ImageMask").map(|o| doc.resolve(o)),
        Some(Object::Boolean(true))
    ) {
        return decode_stencil(dict, &coded, width, height, fill, notes);
    }

    let mut tr = resolve_transparency(doc, dict, &coded, resources, &mut notes);
    if let Some(plane) = &tr.alpha
        && plane.dimensions() != (width, height)
    {
        notes.mask_resampled = true;
        // Table 145's `Width` row: "**If a `Matte` entry … is present,
        // shall be the same as the `Width` value of the parent image**;
        // otherwise independent of it." A mismatched `/Matte` mask is
        // therefore non-conformant, and un-premultiplying with a
        // resampled α would divide each sample by an alpha that is not
        // its own — recovering colours from the wrong equation. The
        // ALPHA is still honoured (that half is conformant either way);
        // only the colour correction is dropped, by name.
        if tr.matte.is_some() {
            tr.matte = None;
            notes.matte_not_undone = Some("matte/dimension-mismatch");
        }
    }

    decode_sampled(
        doc,
        dict,
        &coded,
        width,
        height,
        resources,
        notes,
        tr.alpha.as_ref(),
        tr.colour_key,
        tr.matte.as_deref(),
        policy,
    )
}

/// Whichever alpha source won [`decode`]'s precedence ladder, plus the
/// `/Matte` that may travel with a soft mask.
///
/// A struct rather than a tuple because the three fields are not
/// interchangeable and two of them are `Option`s of the same shape —
/// exactly the situation where a positional return silently swaps under
/// a later edit.
struct Transparency<'a> {
    /// Per-texel alpha, for the three mechanisms that have one.
    alpha: Option<AlphaPlane>,
    /// The un-parsed colour-key `/Mask` array (§8.9.6.4), which cannot
    /// become a plane — see [`resolve_transparency`].
    colour_key: Option<&'a Object>,
    /// `/Matte` (Table 146), in the **parent image's** colour space.
    matte: Option<Vec<f32>>,
}

/// Walk the precedence ladder documented on [`decode`] and produce
/// whichever alpha source wins.
///
/// Returns the resolved [`AlphaPlane`] for the three per-sample
/// mechanisms, **or** the un-parsed `/Mask` array object for colour-key
/// masking — which cannot become a plane here, because §8.9.6.4's ranges
/// are tested against the base image's own pre-`/Decode` samples and
/// those only exist inside [`decode_sampled`]'s pixel loop.
///
/// Every refusal is recorded in `notes` and none of them falls through
/// to the next mechanism (see [`decode`]'s precedence section for why).
fn resolve_transparency<'a>(
    doc: &DocumentView<'a>,
    dict: &'a Dict,
    coded: &CodedImage,
    resources: &Dict,
    notes: &mut ImageNotes,
) -> Transparency<'a> {
    let none = Transparency {
        alpha: None,
        colour_key: None,
        matte: None,
    };

    // Rung 2 — `/SMask`.
    if let Some(entry) = dict.get(b"SMask") {
        return match mask::soft_mask_plane(doc, entry, resources) {
            Ok(soft) => {
                notes.mask_applied = Some(MaskApplied::SoftMask);
                Transparency {
                    alpha: Some(soft.plane),
                    colour_key: None,
                    matte: soft.matte,
                }
            }
            Err(err) => {
                notes.mask_refused = Some(err.key());
                none
            }
        };
    }

    // Rung 3 — `/Mask`, dispatched on its resolved type (§8.9.6.1: a
    // stream is an explicit mask, an array is a colour-key mask).
    if let Some(entry) = dict.get(b"Mask") {
        return match doc.resolve(entry) {
            Object::Array(_) => {
                // Parsed in `decode_sampled`, where the component count
                // is known; the entry is carried, not the ranges.
                notes.mask_applied = Some(MaskApplied::ColourKey);
                Transparency {
                    colour_key: Some(entry),
                    ..none
                }
            }
            _ => match mask::stencil_plane(doc, entry) {
                Ok(plane) => {
                    notes.mask_applied = Some(MaskApplied::Stencil);
                    Transparency {
                        alpha: Some(plane),
                        ..none
                    }
                }
                Err(err) => {
                    notes.mask_refused = Some(err.key());
                    none
                }
            },
        };
    }

    // Rung 4 — the JPX codestream's own opacity channel. Present only
    // when `/SMaskInData` is 1 (Table 89: the default of 0 means the
    // channel "shall be ignored", so a JPX image with alpha inside it
    // and no `/SMaskInData` is CORRECTLY drawn opaque and is not a
    // shortfall). `/SMaskInData 2` leaves this `None` and is reported
    // through `jpx_smask_in_data_preblended` instead — those colour
    // samples carry a backdrop that needs `/Matte` to undo.
    if let Some(bytes) = &coded.embedded_alpha {
        match AlphaPlane::from_bytes(coded.width, coded.height, bytes.clone()) {
            Some(plane) => {
                notes.mask_applied = Some(MaskApplied::EmbeddedAlpha);
                return Transparency {
                    alpha: Some(plane),
                    ..none
                };
            }
            None => {
                // A short opacity channel is a codec bug, not a
                // document defect; named all the same.
                notes.mask_refused = Some("mask/short-embedded-alpha");
            }
        }
    }

    none
}

/// The physical layout of the sample bytes in hand.
///
/// Distinct from the *image's* `/Width`//`/Height`//`/BitsPerComponent`
/// because a codec declares its own geometry and the two can disagree
/// (module docs). This struct is what the row-stride arithmetic uses;
/// the dictionary's numbers size the pixmap.
#[derive(Debug, Clone, Copy)]
struct SampleLayout {
    /// Samples per row, from the codestream when one exists.
    width: u32,
    /// Components per sample, from the codestream when one exists.
    components: usize,
    /// Bits per component, from the codestream when one exists.
    bits: u32,
}

impl SampleLayout {
    /// Resolve the layout, preferring the codec's declaration and
    /// falling back to the PDF-declared one.
    ///
    /// `codec: None` means no codec ran, so the dictionary is the only
    /// description there is and its values are used verbatim — which is
    /// exactly the pre-Pass-2.1 behaviour, unchanged.
    fn resolve(
        coded: &CodedImage,
        dict_width: u32,
        dict_components: usize,
        dict_bits: u32,
    ) -> Self {
        match coded.codec {
            None => Self {
                width: dict_width,
                components: dict_components,
                bits: dict_bits,
            },
            Some(_) => Self {
                width: if coded.width > 0 {
                    coded.width
                } else {
                    dict_width
                },
                components: if coded.components > 0 {
                    usize::from(coded.components)
                } else {
                    dict_components
                },
                bits: if coded.bits_per_component > 0 {
                    u32::from(coded.bits_per_component)
                } else {
                    dict_bits
                },
            },
        }
    }
}

/// Read a required positive integer dimension (`Width`/`Height`).
///
/// Table 89 marks both Required; zero or negative is Table 89's
/// "entries inconsistent with each other" case (an image with no
/// samples cannot be painted, and a zero stride would divide by zero
/// downstream).
fn positive_dimension(doc: &DocumentView<'_>, dict: &Dict, key: &[u8]) -> Result<u32, ImageError> {
    let raw = dict
        .get(key)
        .map(|o| doc.resolve(o))
        .and_then(Object::as_int)
        .ok_or(ImageError::Malformed("missing /Width or /Height"))?;
    u32::try_from(raw)
        .ok()
        .filter(|&v| v > 0)
        .ok_or(ImageError::Malformed("/Width or /Height is not positive"))
}

/// Turn a filter failure into an image failure, preserving the filter
/// name so the diagnostics can say *which* codec is missing (the
/// operator's next question is always "so what do I need?").
fn map_filter_error(err: FilterError) -> ImageError {
    ImageError::UnsupportedFilter(err.to_string())
}

/// Turn a codec failure into an image failure, keeping the three
/// distinctions the diagnostics need to count separately (decision 005
/// §6.4): "this codec is not built", "this *feature* of this codec is
/// not built", and "these bytes are broken."
fn map_codec_error(err: ImageCodecError) -> ImageError {
    match err {
        ImageCodecError::Filter(inner) => map_filter_error(inner),
        ImageCodecError::FeatureUnsupported { feature } => ImageError::CodecFeature(feature),
        ImageCodecError::TooLarge => ImageError::TooLarge,
        // `Unsupported` and `NotAllowedInline` are both "pdfce will not
        // decode this codec here"; the message already says which and
        // why, and an operator's next action is the same either way.
        other
        @ (ImageCodecError::Unsupported { .. } | ImageCodecError::NotAllowedInline { .. }) => {
            ImageError::CodecUnsupported(other.to_string())
        }
        other => ImageError::UnsupportedFilter(other.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Stencil masks (§8.9.6.2)
// ---------------------------------------------------------------------------

/// Build the RGBA texels for an image mask: opaque `fill` where the
/// sample says "mark", fully transparent where it says "leave alone".
///
/// §8.9.6.2's three normative rules are enforced here:
/// 1. no `/ColorSpace` (ignored if present — `iso32000__s__8.9.5.2.md`
///    recommends honouring `ImageMask` over a non-conformant
///    `ColorSpace`);
/// 2. `/BitsPerComponent` shall be 1 (forced, and a stated other value
///    is a hard inconsistency rather than something to guess around) —
///    **except for `JPXDecode`**, where Table 89's "optional and shall
///    be ignored if present" overrides §8.9.6.2's requirement, being
///    the more specific rule;
/// 3. `/Decode [0 1]` (the **default**) means **0 marks the page**;
///    `[1 0]` reverses it. This is the *one* case where a JPX image's
///    `/Decode` is honoured: §7.4.9 says it "shall be ignored, except
///    in the case where the image is treated as a mask; that is, when
///    `ImageMask` is true", which is precisely this function.
///
/// ## Why the sample is thresholded rather than read as a raw bit
///
/// Every other stencil codec delivers genuine 1-bit samples, so the
/// mask sample *is* the bit. JPX does not: §7.4.9 requires the
/// codestream to "provide a single colour channel with 1-bit samples"
/// for a mask, but pdfce's JPX adapter normalizes every depth to 8 bits
/// (Table 89 makes the delivered depth the reader's choice), so those
/// 1-bit samples arrive as 0 and 255. Reading them at one bit per
/// sample would unpack eight neighbouring pixels out of every one and
/// shear the mask beyond recognition.
///
/// So the row stride and the sample width come from the codec's own
/// declared depth, and the result is compared against zero. For a 1-bit
/// codec that is exactly the old behaviour (`0`/`1` are the only
/// values); for JPX it is exact for conformant data (`0`/`255`); and
/// for a non-conformant deeper mask "any non-zero marks" is a stated
/// fail-soft rather than an invented threshold.
fn decode_stencil(
    dict: &Dict,
    coded: &CodedImage,
    width: u32,
    height: u32,
    fill: Rgb,
    mut notes: ImageNotes,
) -> Result<DecodedImage, ImageError> {
    let data = &coded.samples;
    if let Some(bpc) = dict.get(b"BitsPerComponent").and_then(Object::as_int)
        && bpc != 1
        && coded.codec != Some(Codec::Jpx)
    {
        return Err(ImageError::Malformed(
            "/ImageMask true requires /BitsPerComponent 1",
        ));
    }

    // Polarity: the sample value that MARKS the page. Default `[0 1]`
    // → 0 marks. `[1 0]` → 1 marks. Anything else is not a legal image
    // mask `Decode`; fall back to the default rather than inventing a
    // meaning.
    let ink_sample: u32 = match decode_pairs(dict) {
        Some(pairs) => match pairs.as_slice() {
            [(a, b)] if *a > *b => 1,
            [(_, _)] => 0,
            _ => {
                notes.decode_array_ignored = true;
                0
            }
        },
        None => 0,
    };

    // §8.9.6.2 forces one component, so the component count is fixed by
    // the construct; the width and the *delivered* bit depth come from
    // the codec, because they describe the bytes actually in hand (see
    // the threshold note in this function's docs).
    let layout = SampleLayout::resolve(coded, width, 1, 1);
    let stride = row_stride(layout.width, 1, layout.bits)?;
    if data.len() < stride.saturating_mul(height as usize) {
        notes.truncated = true;
    }

    let mut pixmap = Pixmap::new(width, height).ok_or(ImageError::TooLarge)?;
    let ink = premultiplied(fill, 255);
    // A fully transparent texel must have zero colour too: tiny-skia
    // stores PREMULTIPLIED components, and `r > a` is an invalid
    // premultiplied colour it will refuse to construct.
    let clear = PremultipliedColorU8::from_rgba(0, 0, 0, 0).unwrap_or(ink);

    let texels = pixmap.pixels_mut();
    for y in 0..height as usize {
        let row_bit_base = y.saturating_mul(stride).saturating_mul(8);
        for x in 0..width as usize {
            let raw = read_sample(data, row_bit_base + x * layout.bits as usize, layout.bits);
            let sample = u32::from(raw != 0);
            let Some(slot) = texels.get_mut(y * width as usize + x) else {
                continue;
            };
            *slot = if sample == ink_sample { ink } else { clear };
        }
    }

    Ok(DecodedImage { pixmap, notes })
}

// ---------------------------------------------------------------------------
// Ordinary sampled images (§8.9.3, §8.9.5.2, §8.6)
// ---------------------------------------------------------------------------

/// Build the RGBA texels for a colour image.
///
/// `alpha` is a resolved soft/stencil/embedded alpha plane, sampled per
/// texel across the base's grid; `colour_key_entry` is the un-parsed
/// `/Mask` array for §8.9.6.4 masking. At most one of them is ever
/// `Some` — [`resolve_transparency`] enforces the precedence.
///
/// Both are applied **in the existing pixel loop**, not in a second pass
/// over the pixmap. That is a deliberate performance choice as much as a
/// tidiness one: the loop already reads every raw sample (which is what
/// colour-key masking needs) and already writes every texel (which is
/// what alpha needs), so transparency costs one array index and one
/// multiply per texel rather than a whole extra traversal of a
/// potentially 40-megapixel buffer.
#[allow(clippy::too_many_arguments)] // Each argument is a distinct input
// the loop genuinely needs; bundling them into a struct would move the
// same seven values behind one name without removing any of them.
fn decode_sampled(
    doc: &DocumentView<'_>,
    dict: &Dict,
    coded: &CodedImage,
    width: u32,
    height: u32,
    resources: &Dict,
    mut notes: ImageNotes,
    alpha: Option<&AlphaPlane>,
    colour_key_entry: Option<&Object>,
    matte: Option<&[f32]>,
    policy: RenderPolicy,
) -> Result<DecodedImage, ImageError> {
    // Two independent operator choices ride in on `policy` here, and they
    // touch different halves of the loop below: `cmyk_intent` decides
    // COLOUR (§8.6.4.4) and `mask_resample` decides ALPHA (`SM-A1`).
    let intent = policy.cmyk_intent;
    let data = &coded.samples;
    // Table 89 makes this filter — and only this filter — able to
    // supply its own colour space, bit depth and (non-)`Decode`.
    let jpx = coded.codec == Some(Codec::Jpx);

    let space = match dict.get(b"ColorSpace").map(|o| doc.resolve(o)) {
        // `/ColorSpace` present: the DICTIONARY wins, for every filter
        // including JPX. Table 89 is explicit — "If ColorSpace is
        // present, any colour space specifications in the JPEG2000 data
        // shall be ignored." Reading "the codestream is authoritative
        // for JPX" as a blanket rule and overriding a stated
        // `/ColorSpace` here is the inverted-inversion bug, and it would
        // produce wrong colour on exactly the files a producer took the
        // trouble to tag.
        Some(obj) => resolve_space(doc, obj, resources, 0, intent)?,
        // `/ColorSpace` absent. Required for every other image (Table
        // 89: "Required for images, except those that use the JPXDecode
        // filter"), so this is malformed unless the codestream can
        // supply it.
        None if jpx => codestream_space(coded)?,
        None => return Err(ImageError::Malformed("image has no /ColorSpace")),
    };
    let components = space.components();

    // `/BitsPerComponent` is Required (Table 89) — except for JPX,
    // where it is "optional and shall be ignored if present. The bit
    // depth is determined by the conforming reader in the process of
    // decoding." Note the two-step: for JPX a stated value is not
    // merely redundant, honouring it is *wrong*, so the dictionary is
    // not consulted at all. For DCT the codestream is likewise
    // authoritative (always 8, "each component value shall occupy a
    // byte") but the entry is still Required, so an absent one is
    // tolerated rather than mandated away.
    let declared_bpc = dict
        .get(b"BitsPerComponent")
        .map(|o| doc.resolve(o))
        .and_then(Object::as_int);
    let bpc = match declared_bpc {
        // The Table 89 override: the codestream's depth, whatever the
        // dictionary said. The divergence is already counted by the
        // codec layer.
        _ if jpx => u32::from(coded.bits_per_component).max(1),
        Some(v @ (1 | 2 | 4 | 8 | 16)) => v as u32,
        Some(_) if coded.codec.is_none() => {
            return Err(ImageError::Malformed(
                "/BitsPerComponent is not 1, 2, 4, 8, or 16",
            ));
        }
        None if coded.codec.is_none() => {
            return Err(ImageError::Malformed("image has no /BitsPerComponent"));
        }
        // A codec ran and the dictionary is absent or nonsense; the
        // codestream's own depth is the truth (and the disagreement is
        // already counted by the codec layer).
        _ => u32::from(coded.bits_per_component).max(1),
    };

    // The physical byte layout, which is the codec's when there is one
    // (module docs: "the codestream wins for sample reading").
    let layout = SampleLayout::resolve(coded, width, components, bpc);
    // §8.9.5.2's domain: raw samples run 0 … 2ⁿ − 1, at the depth the
    // samples are ACTUALLY packed at.
    let max_sample = f32::from(u16::MAX).min(((1u32 << layout.bits.min(16)) - 1) as f32);

    // §8.9.5.2 + Table 90. `Decode` maps each raw integer linearly into
    // the colour space's component range; the DEFAULT is colour-space
    // dependent and is emphatically not always `[0 1]`.
    //
    // JPX is the one filter that bypasses this entirely: Table 89 says
    // "If the image uses the JPXDecode filter and ImageMask is false,
    // Decode shall be ignored by a conforming reader", and §7.4.9 says
    // the same from the filter's side ("shall be ignored, except in the
    // case where the image is treated as a mask"). The `ImageMask true`
    // half of that exception is honoured by `decode` branching to
    // `decode_stencil` before it ever reaches here, where `Decode` is a
    // polarity switch rather than a colour transform — so suppressing it
    // in this function is exactly the right scope. This is the one place
    // a shared "apply Decode" helper silently corrupts JPX output.
    let decode = match decode_pairs(dict) {
        _ if jpx => space.default_decode(max_sample),
        Some(pairs) if pairs.len() == components => pairs,
        Some(_) => {
            notes.decode_array_ignored = true;
            space.default_decode(max_sample)
        }
        None => space.default_decode(max_sample),
    };
    // Precompute (offset, slope) per component so the inner loop is one
    // multiply-add: y = Dmin + x·(Dmax − Dmin)/(2ⁿ − 1). A NEGATIVE
    // slope is the `[1 0]` inversion and must survive — this is exactly
    // where a `min`/`max` "normalization" would destroy it.
    let ramp: Vec<(f32, f32)> = decode
        .iter()
        .map(|&(dmin, dmax)| (dmin, (dmax - dmin) / max_sample))
        .collect();

    let stride = row_stride(layout.width, layout.components, layout.bits)?;
    if data.len() < stride.saturating_mul(height as usize) {
        notes.truncated = true;
    }
    // A codestream that declares a different component count from the
    // one `/ColorSpace` implies is Table 89's "entries inconsistent with
    // each other" case. Real files do it, so it is counted rather than
    // refused; only `layout.components` of them are read, so the rows
    // stay aligned either way.
    if coded.codec.is_some() && layout.components != components {
        notes.codec_geometry_mismatch = true;
    }
    let readable = components.min(layout.components);

    // `color__indexed.md`'s named fast path: convert the ≤256-entry
    // palette once, then the per-pixel loop is a table lookup with no
    // colour maths at all.
    let palette = space.palette();

    // §8.9.6.4's ranges are counted against the IMAGE's colour space, so
    // the component count is the one resolved above — 1 for `Indexed`
    // (the index), not the base space's width. A length mismatch drops
    // the mask by name rather than masking the wrong colours.
    let colour_key = match colour_key_entry {
        Some(entry) => match mask::ColourKey::parse(doc, entry, components) {
            Ok(key) => Some(key),
            Err(err) => {
                notes.mask_applied = None;
                notes.mask_refused = Some(err.key());
                None
            }
        },
        None => None,
    };

    // §11.6.5.3's un-premultiply, validated once rather than per texel.
    // Two ways it is dropped here, both named:
    //
    // - `Indexed`: spec ambiguity SM-A4. Table 146 counts `/Matte`'s `n`
    //   from the parent's `/ColorSpace`, which for `Indexed` is 1 (the
    //   index); §11.6.5.3 says "the colour values in the colour table
    //   (not the index values themselves) shall be preblended", which
    //   needs the BASE space's `n`. Those cannot both be satisfied, and
    //   un-premultiplying a palette index is meaningless in any reading.
    // - a length that is not `n`: Table 146 is exact about it, and a
    //   short array would apply the matte to a prefix of the components,
    //   producing a colour cast on some channels and not others.
    let matte = match matte {
        None => None,
        Some(_) if palette.is_some() => {
            notes.matte_not_undone = Some("matte/indexed");
            None
        }
        Some(m) if m.len() != components => {
            notes.matte_not_undone = Some("matte/length");
            None
        }
        Some(m) => {
            notes.matte_undone = true;
            Some(m)
        }
    };

    // Hoisted out of the pixel loop so the overwhelmingly common
    // no-colour-key case pays a perfectly-predicted loop-invariant
    // branch rather than two stack stores per component per texel. On a
    // 40-megapixel CMYK image that is 320 million stores avoided; this
    // project has already had one render-performance emergency and the
    // cheapest time to not create the next one is now.
    let keying = colour_key.is_some();

    // A `Space::Special` conversion runs the document's own function per
    // distinct sample tuple; everything else is closed-form arithmetic and
    // wants no cache at all. `tinting` is the loop-invariant branch, in
    // the same spirit as `keying` above it.
    let tinting = matches!(&space, Space::Special(_));
    // §8.6.6.4/.5: a `/None` colorant "shall never be painted on the
    // page". The whole image is therefore transparent — NOT white.
    //
    // The first version of this Pass returned white from the conversion
    // instead, which looks identical on a blank page and is wrong the
    // moment anything is underneath: an opaque white image ERASES the
    // backdrop the standard requires to show through. Caught by a fixture
    // whose divergence from pdfium was maximal in both directions.
    let suppressed = matches!(&space, Space::Special(cs) if !cs.paints());
    if suppressed {
        notes.colorant_none_suppressed = true;
    }
    if let Space::Special(cs) = &space {
        notes.uncalibrated_colorimetry = match &**cs {
            crate::color::ColorSpace::Lab { .. } => Some("Lab"),
            crate::color::ColorSpace::CalGray { .. } => Some("CalGray"),
            crate::color::ColorSpace::CalRgb { .. } => Some("CalRGB"),
            _ => None,
        };
    }
    let mut tint_cache = tinting.then(|| TintCache::new(layout.bits, readable));
    let mut scratch_diag = ColorDiagnostics::default();

    // Per-component clamp bounds. Only `Lab` differs from 0–1, and it
    // differs enough to matter — see the `default_decode` note.
    let clamp_range: Vec<(f32, f32)> = match &space {
        Space::Special(cs) => (0..cs.components())
            .map(|i| cs.component_range(i))
            .collect(),
        _ => vec![(0.0, 1.0); components],
    };

    let mut pixmap = Pixmap::new(width, height).ok_or(ImageError::TooLarge)?;
    let mut out_of_range = false;
    let texels = pixmap.pixels_mut();

    for y in 0..height as usize {
        let row_bit_base = y.saturating_mul(stride).saturating_mul(8);
        for x in 0..width as usize {
            let first = x.saturating_mul(layout.components);
            // Read the plane BEFORE the colour work: §11.6.5.3's
            // un-premultiply divides by this very value, and it must be
            // applied before the colour-space conversion below.
            // `SM-A1` (R169): the mask→base resampling filter. Passed
            // per call, not stored on the plane, so the same decoded mask
            // can be sampled two ways in one session without a rebuild.
            let plane_alpha = alpha.map_or(255u8, |p| {
                p.at(x as u32, y as u32, width, height, policy.mask_resample)
            });
            // The pre-`/Decode` integers, kept alive across the colour
            // conversion because §8.9.6.4 tests THESE, not the colours
            // they become ("representing colour values BEFORE decoding
            // with the `Decode` array"). Filling it is skipped entirely
            // when no colour-key mask is in force.
            let mut raw_comps = [0u32; MAX_IMAGE_COMPONENTS];
            let rgb = match &palette {
                Some(table) => {
                    // Indexed: one component, and after the (default:
                    // identity) Decode transform it IS the palette
                    // index. §8.6.6.3's clamp is normative.
                    let raw = read_sample(
                        data,
                        row_bit_base + first * layout.bits as usize,
                        layout.bits,
                    );
                    if keying && let Some(slot) = raw_comps.first_mut() {
                        *slot = raw;
                    }
                    let (dmin, slope) = ramp.first().copied().unwrap_or((0.0, 1.0));
                    let value = dmin + raw as f32 * slope;
                    let index = value.round().max(0.0) as usize;
                    match table.get(index) {
                        Some(&c) => c,
                        None => {
                            out_of_range = true;
                            Rgb::BLACK
                        }
                    }
                }
                None => {
                    let mut comps = [0.0f32; MAX_IMAGE_COMPONENTS];
                    for c in 0..readable {
                        let raw = read_sample(
                            data,
                            row_bit_base + (first + c) * layout.bits as usize,
                            layout.bits,
                        );
                        // Filled unconditionally when the space needs a
                        // cache key, not only when colour-keying: the key
                        // IS the raw tuple, so `tinting` joins `keying` as
                        // a reason to keep these.
                        if (keying || tinting)
                            && let Some(slot) = raw_comps.get_mut(c)
                        {
                            *slot = raw;
                        }
                        let (dmin, slope) = ramp.get(c).copied().unwrap_or((0.0, 1.0));
                        // §8.9.5.2's output clamping: "if an output
                        // value falls outside the range allowed for a
                        // component it shall be adjusted to the nearest
                        // allowed value."
                        //
                        // The allowed range is the SPACE's, not 0–1 — the
                        // distinction only bites for `Lab`, whose L is
                        // 0–100 and whose a/b are routinely negative.
                        // Clamping those to 0–1 would flatten the image to
                        // near-black.
                        let (lo, hi) = clamp_range.get(c).copied().unwrap_or((0.0, 1.0));
                        if let Some(slot) = comps.get_mut(c) {
                            *slot = (dmin + raw as f32 * slope).clamp(lo, hi);
                        }
                    }
                    // §11.6.5.3: "If a colour conversion is required,
                    // inversion of the preblending shall precede the
                    // colour conversion", and it is done "in the colour
                    // space specified by the parent image's ColorSpace
                    // entry" — which is exactly the state `comps` is in
                    // on this line and nowhere after it.
                    if let Some(m) = matte {
                        mask::undo_matte(&mut comps, components.min(4), m, plane_alpha);
                    }
                    match &mut tint_cache {
                        Some(cache) => {
                            cache.lookup(&space, intent, &raw_comps[..readable], &comps[..readable])
                        }
                        None => space.to_rgb(intent, &comps, &mut scratch_diag),
                    }
                }
            };
            // Alpha, in the order the precedence ladder resolved: a
            // colour-key hit is absolute (0 or 255, §8.9.6.4 has no
            // partial state), otherwise the plane's sample, otherwise
            // opaque.
            let a = match &colour_key {
                _ if suppressed => 0,
                Some(key) if key.masks(&raw_comps[..readable]) => 0,
                _ => plane_alpha,
            };
            if let Some(slot) = texels.get_mut(y * width as usize + x) {
                *slot = premultiplied(rgb, a);
            }
        }
    }
    notes.palette_out_of_range = out_of_range;

    Ok(DecodedImage { pixmap, notes })
}

/// Row stride in bytes: `ceil(Width × components × BitsPerComponent / 8)`.
///
/// §8.9.3: "each row of the image shall begin on a byte boundary",
/// padded with trailing zero bits. The `ceil` is **per row**, not per
/// image — a 3-pixel-wide 1-bpc image has a 1-byte stride with 5
/// padding bits on every row, and computing it image-wide instead
/// shears the picture diagonally.
///
/// `pub(crate)` for [`crate::mask`], which unpacks `/SMask` and `/Mask`
/// samples under exactly the same §8.9.3 rules. Sharing the function is
/// the point: a mask whose stride was computed differently from the base
/// image's would shear against it, and that is the sort of divergence
/// two copies of "the same" arithmetic produce.
pub(crate) fn row_stride(width: u32, components: usize, bpc: u32) -> Result<usize, ImageError> {
    let bits = u64::from(width)
        .checked_mul(components as u64)
        .and_then(|v| v.checked_mul(u64::from(bpc)))
        .ok_or(ImageError::TooLarge)?;
    usize::try_from(bits.div_ceil(8)).map_err(|_| ImageError::TooLarge)
}

/// Read one `bpc`-wide sample at `bit_offset` bits into `data`.
///
/// §8.9.3 packs "from high-order to low-order bits", and because `bpc`
/// is always 1, 2, 4, 8 or 16 while rows start on byte boundaries, a
/// sample can never straddle a byte boundary for `bpc < 8` — which is
/// what makes this a shift-and-mask rather than a bit-stream reader.
///
/// Out-of-range reads return **0** rather than failing: the caller has
/// already flagged the stream as truncated, and returning a value keeps
/// the surviving majority of the image on the page.
///
/// `pub(crate)` for [`crate::mask`] — see [`row_stride`] for why the
/// mask path shares this rather than restating it.
pub(crate) fn read_sample(data: &[u8], bit_offset: usize, bpc: u32) -> u32 {
    let byte_index = bit_offset / 8;
    let at = |i: usize| u32::from(data.get(i).copied().unwrap_or(0));
    match bpc {
        16 => (at(byte_index) << 8) | at(byte_index + 1),
        8 => at(byte_index),
        // 1, 2, 4: `bit_offset % 8` is always a multiple of `bpc`.
        b => {
            let shift = 8u32.saturating_sub(b + (bit_offset % 8) as u32);
            let mask = (1u32 << b) - 1;
            (at(byte_index) >> shift) & mask
        }
    }
}

/// Read `/Decode` as `(Dmin, Dmax)` pairs, or `None` when absent.
///
/// **Pairs are never normalized.** `Dmin > Dmax` is §8.9.5.2's
/// inversion idiom, not a malformed rectangle — the exact opposite of
/// §7.9.5's rule for `/BBox` and `/MediaBox`, and confusing the two is
/// the named trap in `iso32000__s__8.9.5.2.md`.
///
/// `pub(crate)` for [`crate::mask`]: a soft mask's `/Decode` inverts
/// alpha by the same `Dmin > Dmax` idiom, and a stencil mask's is a
/// polarity switch read from the same pair.
pub(crate) fn decode_pairs(dict: &Dict) -> Option<Vec<(f32, f32)>> {
    let items = dict.get(b"Decode")?.as_array()?;
    if items.len() < 2 {
        return None;
    }
    Some(
        items
            .chunks_exact(2)
            .map(|pair| {
                let lo = pair.first().and_then(Object::as_number).unwrap_or(0.0) as f32;
                let hi = pair.get(1).and_then(Object::as_number).unwrap_or(1.0) as f32;
                (lo, hi)
            })
            .collect(),
    )
}

/// Pack an [`Rgb`] plus alpha into a tiny-skia premultiplied texel.
///
/// ## This is a MULTIPLY, and it has to be
///
/// tiny-skia stores colours **premultiplied**: the stored component is
/// `colour × alpha`, not the colour with an alpha stapled beside it.
/// Before transparency landed, every texel this module produced had
/// `alpha == 255` (a stencil mask's transparent texels being explicitly
/// `TRANSPARENT` rather than routed through here), so the function could
/// get away with `min(round(v × 255), alpha)` — a *clamp*, which is
/// exactly right at `alpha == 255` and exactly right at the extremes
/// `v == 0` and `v == 1`, and wrong everywhere else.
///
/// That is a genuinely nasty shape of bug: mid-grey at half alpha would
/// have come out as `min(128, 128) = 128` — full-strength grey, twice as
/// bright as the correct `0.5 × 128 = 64` — while every pure black, pure
/// white and fully-opaque pixel stayed right. A test over a black-and-
/// white checkerboard would have passed. The fixtures therefore use a
/// deliberately mid-toned ramp (`tools/gen-image-fixtures.py`'s
/// `alpha_at`), which is the only kind of data that can catch it.
///
/// Multiplying by `alpha` (0–255) rather than by 255 and then scaling
/// keeps the type's own invariant — `component ≤ alpha` — true by
/// construction rather than by a `min` that hides the arithmetic error.
/// At `alpha == 255` the result is bit-identical to the old code, so no
/// opaque image moved a single pixel when this changed.
fn premultiplied(c: Rgb, alpha: u8) -> PremultipliedColorU8 {
    let a = f32::from(alpha);
    let q = |v: f32| (v.clamp(0.0, 1.0) * a).round() as u8;
    PremultipliedColorU8::from_rgba(q(c.r), q(c.g), q(c.b), alpha)
        .unwrap_or(PremultipliedColorU8::TRANSPARENT)
}

// ---------------------------------------------------------------------------
// Colour spaces (§8.6, §8.6.6.3)
// ---------------------------------------------------------------------------

/// The image colour spaces this slice converts.
///
/// Deliberately *not* a general colour-space model — that arrives with
/// the `cs`/`sc`/`scn` operators in a later Pass. This is the minimum
/// that covers the overwhelming majority of real images: the three
/// device spaces, their CIE-based aliases handled by the same maths,
/// `ICCBased` through its `N`-component fallback, and `Indexed` over
/// any of those.
///
/// `pub(crate)` for [`crate::mask`], which needs exactly one thing from
/// it: [`Space::components`], to enforce that an `/SMask`'s colour space
/// carries one component per sample.
/// Ceiling on an image colour space's component count.
///
/// Bounds two things at once: the per-pixel component buffer, and a
/// malformed file's ability to make the row-stride arithmetic enormous.
/// A real `DeviceN` is a duotone (2), a hexachrome (6), or occasionally a
/// packaging file with a dozen inks; 32 is comfortably past any of them
/// and matches the guard `crate::shading` puts on `/Function` outputs.
pub(crate) const MAX_IMAGE_COMPONENTS: usize = 32;

/// Memoises [`Space::Special`] conversions on the **pre-`/Decode`
/// integer samples**.
///
/// # Why a cache is load-bearing rather than an optimisation
///
/// A `Separation` or `DeviceN` conversion runs the document's
/// `/tintTransform` — a §7.10 function, which for `FunctionType 4` is a
/// PostScript calculator interpreted per call. A 40-megapixel duotone
/// would run it 40 million times to produce, at most, 65 536 distinct
/// answers. Without memoisation this Pass would trade "the image is
/// missing" for "the page takes a minute", which is not obviously the
/// better failure.
///
/// # Why the key is the RAW samples
///
/// The raw integers are the natural quantisation: two texels with
/// identical samples have identical colour by definition, so the cache is
/// **exact** — it changes speed and nothing else. Keying on the decoded
/// floats instead would need an epsilon, and an epsilon here would be a
/// silent colour approximation of exactly the kind rule 4 forbids.
///
/// The key packs each component into `bits` and requires
/// `components × bits ≤ 64`. That covers 8 channels at 8 bits and 4 at
/// 16 — every duotone, every hexachrome at 8 bits, every `Lab` image.
/// Wider inputs fall back to computing per pixel (correct, slower), which
/// is the honest degradation: a cache that dropped precision to fit would
/// change the picture.
struct TintCache {
    /// Distinct sample tuples seen, and the colour each produced.
    seen: std::collections::HashMap<u64, Rgb>,
    /// Bits per component, for packing the key.
    bits: u32,
    /// How many components participate in the key.
    components: usize,
    /// Whether the key fits in 64 bits at all.
    packable: bool,
    /// Diagnostics from the DISTINCT conversions, not from the texels.
    ///
    /// This is why the cache owns them: routed straight from the pixel
    /// loop, `tint_transform_not_applied` would report once per texel and
    /// a shell would print "8 million spot-colour conversions had no tint
    /// transform" for one broken image.
    diag: ColorDiagnostics,
}

impl TintCache {
    fn new(bits: u32, components: usize) -> Self {
        let packable = components > 0 && bits > 0 && (components as u32).saturating_mul(bits) <= 64;
        Self {
            seen: std::collections::HashMap::new(),
            bits,
            components,
            packable,
            diag: ColorDiagnostics::default(),
        }
    }

    /// Pack the raw samples into a key, or `None` when they do not fit.
    fn key(&self, raw: &[u32]) -> Option<u64> {
        if !self.packable {
            return None;
        }
        let mut k = 0u64;
        for v in raw.iter().take(self.components) {
            k = (k << self.bits) | u64::from(*v);
        }
        Some(k)
    }

    /// The colour for one texel, computed once per distinct sample tuple.
    fn lookup(&mut self, space: &Space, intent: CmykIntent, raw: &[u32], comps: &[f32]) -> Rgb {
        match self.key(raw) {
            Some(k) => {
                if let Some(hit) = self.seen.get(&k) {
                    return *hit;
                }
                let rgb = space.to_rgb(intent, comps, &mut self.diag);
                self.seen.insert(k, rgb);
                rgb
            }
            None => space.to_rgb(intent, comps, &mut self.diag),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Space {
    /// `DeviceGray` / `CalGray` / `ICCBased` with `N 1`.
    Gray,
    /// `DeviceRGB` / `CalRGB` / `ICCBased` with `N 3`.
    Rgb,
    /// `DeviceCMYK` / `ICCBased` with `N 4`.
    Cmyk,
    /// `[/Indexed base hival lookup]` (§8.6.6.3). The palette is
    /// resolved to RGB at construction — see [`Space::palette`].
    Indexed(Vec<Rgb>),
    /// Any space this rasterizer does not decode itself, delegated whole
    /// to [`crate::color::ColorSpace`]: `Separation`, `DeviceN`, `Lab`,
    /// `CalGray` and `CalRGB`.
    ///
    /// # Why delegate rather than add four more arms here
    ///
    /// [`crate::color`] already parses every one of these, already
    /// evaluates a `/tintTransform` through [`pdfce_core::function`], and
    /// already knows `/All` and `/None`. Re-implementing them here would
    /// put a **second** answer to "what colour is this tint?" in the
    /// binary — and the two would be reached by different content (a
    /// filled rectangle versus an image), so a divergence would show up
    /// as *the same spot colour printing two different ways on one page*.
    /// That is the exact failure `pdfce_core::function` was centralised to
    /// prevent, stated in [`crate::color`]'s own module docs.
    ///
    /// The cost is that conversion is no longer a closed-form arithmetic
    /// step — a `Separation` runs a §7.10 function per distinct sample
    /// tuple — which is what [`TintCache`] exists to bound.
    Special(std::sync::Arc<crate::color::ColorSpace>),
}

impl Space {
    /// Number of colour components a *sample* carries.
    ///
    /// For `Indexed` this is **1** — the index — not the base space's
    /// count. That distinction drives the row stride, the `Decode`
    /// array length, and the predictor's `/Colors`, and getting it
    /// wrong shears the image (`color__indexed.md`).
    pub(crate) fn components(&self) -> usize {
        match self {
            Self::Gray | Self::Indexed(_) => 1,
            Self::Rgb => 3,
            Self::Cmyk => 4,
            // A `DeviceN` carries one component per colorant name, so this
            // is the ONLY space whose sample width is not fixed by its
            // family. It drives the row stride and the `/Decode` length,
            // so a wrong answer here shears the image rather than
            // discolouring it.
            Self::Special(cs) => cs.components(),
        }
    }

    /// Table 90's default `Decode` array for this space.
    ///
    /// `[0 1]` per component for the device spaces; **`[0 2ⁿ−1]`** for
    /// `Indexed`, which makes the transform the identity so raw samples
    /// pass through as palette indices unchanged (§8.9.5.2 NOTE 2).
    /// `ICCBased`'s true default is the profile's `Range`, which pdfce
    /// does not parse; `[0 1]` per component is the documented
    /// `N`-fallback approximation (`color__iccbased.md`) and is correct
    /// for every profile whose range is the usual 0–1.
    fn default_decode(&self, max_sample: f32) -> Vec<(f32, f32)> {
        match self {
            Self::Indexed(_) => vec![(0.0, max_sample)],
            // ★ NOT `[0 1]` per component. Table 90's default is the
            // space's own component RANGE, and `Lab` is the case where
            // that is not 0–1: its L runs 0–100 and its a/b run over the
            // `/Range` array's values, which are routinely negative.
            // Defaulting a `Lab` image to `[0 1]` would collapse every
            // sample into the darkest corner of the space and paint a
            // near-black picture — plausible enough to be mistaken for a
            // badly exposed scan rather than for a decode bug.
            Self::Special(cs) => (0..cs.components())
                .map(|i| cs.component_range(i))
                .collect(),
            _ => vec![(0.0, 1.0); self.components()],
        }
    }

    /// The resolved palette, for `Indexed` only.
    fn palette(&self) -> Option<&[Rgb]> {
        match self {
            Self::Indexed(table) => Some(table),
            _ => None,
        }
    }

    /// Convert decoded components (already clamped into range) to RGB.
    ///
    /// `diag` is threaded because [`Self::Special`] delegates to
    /// [`crate::color::ColorSpace::to_rgb`], which counts its own
    /// shortfalls (a missing `/tintTransform`, a `/Separation /All`
    /// approximation). Callers in a per-pixel loop must route through
    /// [`TintCache`] rather than calling this directly — otherwise those
    /// counters would tick once per texel and report millions.
    fn to_rgb(&self, intent: CmykIntent, comps: &[f32], diag: &mut ColorDiagnostics) -> Rgb {
        let c = |i: usize| comps.get(i).copied().unwrap_or(0.0);
        match self {
            // `None` here means the space paints nothing at all —
            // `/Separation /None`, or an all-`/None` `DeviceN`
            // (§8.6.6.4/.5, "shall never be painted on the page").
            //
            // The colour returned is irrelevant BECAUSE THE ALPHA IS
            // ZERO: the decoder sets `suppressed` from the same
            // `paints()` query and forces every texel transparent. Black
            // is chosen over white deliberately — if the alpha path were
            // ever bypassed, a black block is an obvious defect that gets
            // reported, whereas white is invisible on the blank page a
            // test most likely uses and silently erases content on a real
            // one. Fail loudly, not plausibly.
            Self::Special(cs) => cs.to_rgb(comps, intent, diag).unwrap_or(Rgb::BLACK),
            // An Indexed space never reaches here — the palette path
            // short-circuits it — but returning grey rather than
            // panicking keeps this total.
            Self::Gray | Self::Indexed(_) => Rgb::from_gray(c(0)),
            Self::Rgb => Rgb::from_rgb(c(0), c(1), c(2)),
            // The same calibrated conversion the `k`/`K` operators use —
            // one function in `pdfce_core::color`, reached through the
            // same `Rgb` constructor — so an image and a filled rectangle
            // of the "same" CMYK agree on screen by construction rather
            // than by two formulas being kept in step (gstate.rs docs).
            Self::Cmyk => Rgb::from_cmyk(intent, c(0), c(1), c(2), c(3)),
        }
    }
}

/// The colour space a JPX codestream supplies when `/ColorSpace` is
/// absent — §7.4.9's fallback ladder, terminal rung.
///
/// Table 89: "If `ColorSpace` is absent, the colour space
/// specifications in the JPEG2000 data shall be used." §7.4.9 spells out
/// what "used" means when the codestream's specification is not one a
/// reader supports: "the next lower colour space … shall be used", and
/// "**if no supported colour space is found, the colour space used shall
/// be `DeviceGray`, `DeviceRGB`, or `DeviceCMYK`, depending on …
/// whether the number of channels in the JPEG2000 data is 1, 3, or
/// 4**."
///
/// `pdfce_core::image_codec::jpx` walks the upper rungs (enumerated
/// spaces, and an embedded ICC profile's own data-colour-space
/// signature) and reports the result as a [`CodecColorModel`]. This
/// function is the last step: turning that into one of the three device
/// spaces this rasterizer converts.
///
/// # Errors
///
/// [`ImageError::UnsupportedColorSpace`] for a channel count with no PDF
/// device-space mapping, and for an ICC profile whose colour space pdfce
/// cannot approximate as a device space (a `Lab ` profile's samples are
/// not device components; painting them as RGB would be a plausible-
/// looking, entirely wrong picture). Refusing is the `fuzzy, never
/// sneaky` outcome — nothing is drawn and the caller counts it.
fn codestream_space(coded: &CodedImage) -> Result<Space, ImageError> {
    match coded.color_model {
        CodecColorModel::Gray => Ok(Space::Gray),
        CodecColorModel::Rgb | CodecColorModel::Untransformed3 => Ok(Space::Rgb),
        CodecColorModel::Cmyk => Ok(Space::Cmyk),
        // Neither can reach here: `Bilevel` belongs to the fax codecs,
        // whose `/ColorSpace` is Required, and `Unspecified` means no
        // codec ran at all. Mapped rather than unreachable! so this
        // stays total (`pdfce-core` denies `panic!` for the same
        // reason).
        CodecColorModel::Bilevel => Ok(Space::Gray),
        // `Unspecified`, `Unknown { .. }`, and — because
        // `CodecColorModel` is `#[non_exhaustive]` — any model a future
        // codec adds before this function learns about it. Refusing an
        // unrecognized model is the only safe default: the alternative
        // is painting samples of unknown meaning as if they were RGB.
        _ => Err(ImageError::UnsupportedColorSpace(
            "the JPEG2000 codestream's colour space".into(),
        )),
    }
}

/// Resolve a `/ColorSpace` value to a [`Space`].
///
/// Handles the four shapes §8.6/§8.9.7 allow in an image: a device
/// name, a name referring to the resource dictionary's `/ColorSpace`
/// subdictionary, an `[/ICCBased stream]` array, and an
/// `[/Indexed base hival lookup]` array. `depth` guards the nesting
/// (`Indexed` over `ICCBased` is two levels; a self-referential named
/// resource is unbounded).
///
/// `pub(crate)` for [`crate::mask`]: a soft mask has its own
/// `/ColorSpace`, resolved by the same rules (including the named-
/// resource hop), and only then checked for the single-component
/// constraint §8.9.5 Table 89 puts on it.
pub(crate) fn resolve_space(
    doc: &DocumentView<'_>,
    obj: &Object,
    resources: &Dict,
    depth: usize,
    intent: CmykIntent,
) -> Result<Space, ImageError> {
    if depth > MAX_COLORSPACE_DEPTH {
        return Err(ImageError::UnsupportedColorSpace(
            "colour space nested too deeply".into(),
        ));
    }
    match obj {
        Object::Name(n) => match n.as_bytes() {
            b"DeviceGray" | b"CalGray" | b"G" => Ok(Space::Gray),
            b"DeviceRGB" | b"CalRGB" | b"RGB" => Ok(Space::Rgb),
            b"DeviceCMYK" | b"CMYK" => Ok(Space::Cmyk),
            // §7.8.3: any other name is a key in the resource
            // dictionary's `/ColorSpace` subdictionary.
            other => {
                let entry = resources
                    .get(b"ColorSpace")
                    .map(|o| doc.resolve(o))
                    .and_then(Object::as_dict)
                    .and_then(|cs| cs.get(other))
                    .map(|o| doc.resolve(o));
                match entry {
                    Some(inner) => resolve_space(doc, inner, resources, depth + 1, intent),
                    None => Err(ImageError::UnsupportedColorSpace(format!(
                        "/{}",
                        String::from_utf8_lossy(other)
                    ))),
                }
            }
        },
        Object::Array(items) => resolve_space_array(doc, items, resources, depth, intent),
        _ => Err(ImageError::UnsupportedColorSpace(
            "/ColorSpace is neither a name nor an array".into(),
        )),
    }
}

/// The array forms of `/ColorSpace`.
fn resolve_space_array(
    doc: &DocumentView<'_>,
    items: &[Object],
    resources: &Dict,
    depth: usize,
    intent: CmykIntent,
) -> Result<Space, ImageError> {
    let family = items
        .first()
        .map(|o| doc.resolve(o))
        .and_then(Object::as_name)
        .map(|n| n.as_bytes().to_vec())
        .unwrap_or_default();

    match family.as_slice() {
        // A one-element array is just the name (`[/DeviceRGB]`), which
        // real producers emit.
        _ if items.len() == 1 => {
            let name = Object::Name(pdfce_core::object::Name(family));
            resolve_space(doc, &name, resources, depth + 1, intent)
        }
        // `[/ICCBased stream]` — §8.6.5.5. pdfce does not parse ICC
        // profiles; the spec's own fallback is the stream's `/N`
        // (1 → Gray, 3 → RGB, 4 → CMYK), which is exactly what
        // `/Alternate` would default to (`color__iccbased.md`).
        b"ICCBased" => {
            let n = items
                .get(1)
                .map(|o| doc.resolve(o))
                .and_then(Object::as_dict)
                .and_then(|d| d.get(b"N"))
                .map(|o| doc.resolve(o))
                .and_then(Object::as_int);
            match n {
                Some(1) => Ok(Space::Gray),
                Some(3) => Ok(Space::Rgb),
                Some(4) => Ok(Space::Cmyk),
                _ => Err(ImageError::UnsupportedColorSpace(
                    "/ICCBased without a usable /N".into(),
                )),
            }
        }
        // `[/Indexed base hival lookup]` — §8.6.6.3. Kept HERE rather
        // than delegated, because an image's `Indexed` space is not a
        // per-sample conversion at all: the sample IS the palette index,
        // the width of a sample is the index width and not the base
        // space's component count, and the whole palette is resolved once
        // at construction. `crate::color`'s `Indexed` answers a different
        // question (what colour is index N) and would give the row stride
        // the wrong number of components.
        b"Indexed" | b"I" => resolve_indexed(doc, items, resources, depth, intent),
        // Everything else `crate::color` knows how to parse — the two
        // this Pass exists for (`Separation`, `DeviceN`) and the three
        // that came free with them (`Lab`, `CalGray`, `CalRGB`).
        //
        // Before this, EVERY one of these was an outright refusal and the
        // image was dropped from the raster entirely. On the operator's
        // Ghent X-4 file that was 18 pictures across three pages, which is
        // the largest single hole this crate had.
        b"Separation" | b"DeviceN" | b"Lab" | b"CalGray" | b"CalRGB" => {
            let obj = Object::Array(items.to_vec());
            let mut scratch = ColorDiagnostics::default();
            match crate::color::resolve_object(doc, &obj, resources, depth, &mut scratch) {
                Some(cs) if cs.components() > 0 && cs.components() <= MAX_IMAGE_COMPONENTS => {
                    Ok(Space::Special(cs))
                }
                // A space that resolves to zero components, or to more
                // than the guard allows, is refused rather than clamped:
                // the component count sets the row stride, so a wrong one
                // does not discolour the image, it shears it.
                Some(cs) => Err(ImageError::UnsupportedColorSpace(format!(
                    "/{} with {} component(s)",
                    String::from_utf8_lossy(&family),
                    cs.components()
                ))),
                None => Err(ImageError::UnsupportedColorSpace(format!(
                    "/{} (did not resolve)",
                    String::from_utf8_lossy(&family)
                ))),
            }
        }
        other => Err(ImageError::UnsupportedColorSpace(format!(
            "/{}",
            String::from_utf8_lossy(other)
        ))),
    }
}

/// Build an [`Space::Indexed`] palette from `[/Indexed base hival lookup]`.
///
/// Per §8.6.6.3: the table is `m × (hival + 1)` bytes where `m` is the
/// **base** space's component count; "each byte shall be an unsigned
/// integer in the range 0 to 255 that shall be scaled to the range of
/// the corresponding colour component" — i.e. the table is always
/// 8-bit-per-component regardless of the image's own
/// `BitsPerComponent`, which governs only the width of the *index*.
///
/// A short table is tolerated (producers trim unused trailing entries);
/// the palette simply ends early and out-of-range indices paint black
/// with [`ImageNotes::palette_out_of_range`] set.
fn resolve_indexed(
    doc: &DocumentView<'_>,
    items: &[Object],
    resources: &Dict,
    depth: usize,
    intent: CmykIntent,
) -> Result<Space, ImageError> {
    // The palette is built ONCE, at construction, so its conversions are
    // bounded by `hival + 1` and want no cache. The diagnostics are
    // scratch for the same reason: a shortfall in a 256-entry palette is
    // reported by the entry count, not by a counter.
    let mut palette_diag = ColorDiagnostics::default();
    let base_obj =
        items
            .get(1)
            .map(|o| doc.resolve(o))
            .ok_or(ImageError::UnsupportedColorSpace(
                "/Indexed without a base space".into(),
            ))?;
    let base = resolve_space(doc, base_obj, resources, depth + 1, intent)?;
    if matches!(base, Space::Indexed(_)) {
        // §8.6.6.3: the base "shall not be … another Indexed space".
        return Err(ImageError::UnsupportedColorSpace(
            "/Indexed over /Indexed".into(),
        ));
    }
    let m = base.components();

    // `hival` is a MAXIMUM INDEX, not a count — the table has
    // `hival + 1` entries. Normative ceiling: 255.
    let hival = items
        .get(2)
        .map(|o| doc.resolve(o))
        .and_then(Object::as_int)
        .filter(|&v| (0..=255).contains(&v))
        .ok_or(ImageError::UnsupportedColorSpace(
            "/Indexed /hival missing or outside 0..=255".into(),
        ))? as usize;

    // The lookup may be a byte STRING (PDF 1.2, and the form §8.6.6.3's
    // own example uses) or a STREAM. A reader that handles only the
    // stream case fails on the spec's own example.
    let lookup_obj =
        items
            .get(3)
            .map(|o| doc.resolve(o))
            .ok_or(ImageError::UnsupportedColorSpace(
                "/Indexed without a lookup table".into(),
            ))?;
    let lookup: Vec<u8> = match lookup_obj {
        Object::String(bytes) => bytes.clone(),
        Object::Stream(stream) => {
            // `doc.slice`, not `span.slice(doc.bytes())`: on a session
            // view the payload may live in the R45 staging half and
            // there is no single buffer to index (decision 018 §4).
            let raw = doc
                .slice(stream.data_span)
                .ok_or(ImageError::UnsupportedColorSpace(
                    "/Indexed lookup stream is out of bounds".into(),
                ))?;
            filters::decode_stream(&stream.dict, raw).map_err(map_filter_error)?
        }
        _ => {
            return Err(ImageError::UnsupportedColorSpace(
                "/Indexed lookup is neither a string nor a stream".into(),
            ));
        }
    };

    let mut table = Vec::with_capacity(hival + 1);
    for i in 0..=hival {
        let base_off = i.saturating_mul(m);
        let Some(entry) = lookup.get(base_off..base_off + m) else {
            // Short table: stop here. Indices past the end paint black
            // and set `palette_out_of_range`.
            break;
        };
        let mut comps = [0.0f32; 4];
        for (c, slot) in comps.iter_mut().take(m).enumerate() {
            *slot = f32::from(entry.get(c).copied().unwrap_or(0)) / 255.0;
        }
        table.push(base.to_rgb(intent, &comps, &mut palette_diag));
    }
    Ok(Space::Indexed(table))
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

    #[test]
    fn row_stride_rounds_up_per_row() {
        // §8.9.3: a 3-pixel 1-bpc row is 1 byte with 5 padding bits.
        assert_eq!(row_stride(3, 1, 1).unwrap(), 1);
        assert_eq!(row_stride(9, 1, 1).unwrap(), 2);
        assert_eq!(row_stride(2, 3, 8).unwrap(), 6);
        assert_eq!(row_stride(2, 1, 4).unwrap(), 1);
        assert_eq!(row_stride(3, 1, 4).unwrap(), 2);
    }

    #[test]
    fn sub_byte_samples_unpack_high_order_first() {
        // 0b01_10_11_00 at 2 bpc → 1, 2, 3, 0.
        let data = [0b0110_1100u8];
        let got: Vec<u32> = (0..4).map(|i| read_sample(&data, i * 2, 2)).collect();
        assert_eq!(got, vec![1, 2, 3, 0]);
        // 1 bpc.
        let data = [0b1010_0000u8];
        let got: Vec<u32> = (0..4).map(|i| read_sample(&data, i, 1)).collect();
        assert_eq!(got, vec![1, 0, 1, 0]);
    }

    #[test]
    fn sixteen_bit_samples_are_big_endian() {
        assert_eq!(read_sample(&[0x12, 0x34], 0, 16), 0x1234);
    }

    #[test]
    fn reads_past_the_end_are_zero_not_a_panic() {
        assert_eq!(read_sample(&[], 0, 8), 0);
        assert_eq!(read_sample(&[0xFF], 8, 16), 0);
    }

    #[test]
    fn indexed_default_decode_is_the_identity() {
        // Table 90 / §8.9.5.2 NOTE 2: [0 2ⁿ−1], so y = x.
        let space = Space::Indexed(vec![Rgb::BLACK]);
        assert_eq!(space.default_decode(255.0), vec![(0.0, 255.0)]);
        assert_eq!(space.components(), 1, "the sample is ONE index");
    }

    #[test]
    fn device_default_decode_is_zero_to_one_per_component() {
        assert_eq!(Space::Rgb.default_decode(255.0), vec![(0.0, 1.0); 3]);
        assert_eq!(Space::Cmyk.default_decode(15.0), vec![(0.0, 1.0); 4]);
    }

    #[test]
    fn decode_pairs_are_not_normalized() {
        // `[1 0]` is inversion (§8.9.5.2 NOTE 3), NOT a malformed
        // rectangle — the named trap.
        let mut d = Dict::new();
        d.insert(
            pdfce_core::object::Name::from(b"Decode"),
            Object::Array(vec![Object::Integer(1), Object::Integer(0)]),
        );
        assert_eq!(decode_pairs(&d), Some(vec![(1.0, 0.0)]));
    }
}
