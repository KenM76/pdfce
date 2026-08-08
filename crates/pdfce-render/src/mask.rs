//! # Image transparency: `/SMask`, `/Mask`, and colour-key masking
//!
//! Everything that decides **how opaque a base image's texel is**, as
//! opposed to what colour it is. [`crate::image`] owns colour; this
//! module owns alpha, and the two meet in exactly one place — the
//! per-texel `premultiplied(rgb, alpha)` call at the bottom of
//! `image::decode_sampled`.
//!
//! ## The four mechanisms, and which one this module handles
//!
//! §8.9.6.1 enumerates four ways a sampled image can be partly
//! transparent. They are **not** four flavours of one thing; they differ
//! in where the alpha lives, what its resolution is, and whether it is
//! binary or continuous:
//!
//! | Mechanism | Key | Alpha source | Handled |
//! |---|---|---|---|
//! | Stencil mask | `/ImageMask true` | the image's own 1-bit samples | `image::decode_stencil` — **not here** (the image *is* the mask; it has no colour of its own) |
//! | Explicit mask | `/Mask` → **stream** | a *separate* 1-bit image XObject | [`stencil_plane`] |
//! | Colour-key mask | `/Mask` → **array** | ranges of the base image's own **pre-`/Decode`** samples | [`ColourKey`] |
//! | Soft mask | `/SMask` → stream | a *separate* greyscale image, one continuous alpha per sample | [`soft_mask_plane`] |
//!
//! A fifth source exists that is not a dictionary entry at all: a JPX
//! codestream's own opacity channel, surfaced by
//! `CodedImage::embedded_alpha` when `/SMaskInData` is `1` (Table 89 —
//! the default `0` means "shall be ignored", so a JPX file with alpha
//! inside it and no `/SMaskInData` is *correctly* drawn opaque). That
//! arrives already as one 8-bit sample per pixel and becomes a plane
//! through [`AlphaPlane::from_bytes`].
//!
//! ## Why an [`AlphaPlane`] rather than "just index the mask"
//!
//! §8.9.6.3 is explicit, and it is the rule most naive implementations
//! get wrong:
//!
//! > "The base image and the image mask **need not have the same
//! > resolution** (`Width` and `Height` values), but since all images
//! > shall be defined on the unit square in user space, **their
//! > boundaries on the page will coincide**."
//!
//! So a 4×4 mask over a 64×64 base is legal and means "each mask sample
//! covers a 16×16 block of the base." Indexing the mask with the base's
//! own `(x, y)` would read 60 rows past the end of a 4-row mask and,
//! thanks to the read-past-the-end-is-zero rule, would produce a
//! plausible-looking, entirely wrong picture. [`AlphaPlane::at`] does the
//! unit-square mapping instead: it converts the base texel's **centre**
//! to a normalized coordinate and takes the mask sample containing it.
//!
//! Soft masks say the same thing in their own words, and say it more
//! strongly — Table 145's `Width` row (`Height`: "Same considerations"),
//! verbatim:
//!
//! > "**If a `Matte` entry (see Table 146) is present, shall be the same
//! > as the `Width` value of the parent image; otherwise independent of
//! > it. Both images shall be mapped to the unit square in user space
//! > (as are all images), regardless of whether the samples coincide
//! > individually.**"
//!
//! "Regardless of whether the samples coincide individually" settles it:
//! a size-mismatched `/SMask` is **normal and conformant**, and the
//! correspondence between the two grids is purely geometric. The one
//! exception is the `/Matte` case, where equality is a `shall` — see
//! [`undo_matte`] and [`crate::image::ImageNotes::matte_not_undone`].
//!
//! ## Sampling is nearest-neighbour — a disclosed pdfce choice
//!
//! **ISO 32000-1 specifies no resampling algorithm for a size-mismatched
//! mask** (spec-ambiguity `SM-A1` in `iso32000__s__11.6.5.md`: the words
//! "resample" and "nearest neighbour" do not appear, and the three
//! occurrences of "bilinear" are unrelated to images). So this is
//! pdfce's call, not the spec's, and it is recorded as such rather than
//! presented as compliance.
//!
//! §8.9.5.3's `/Interpolate` governs how the *base image* is sampled
//! onto the page and is applied by the pattern shader in
//! `interpret::paint_image`, downstream of this module. The mask→base
//! resampling here is a different question: it happens in image space,
//! before any page geometry exists. Nearest-neighbour is chosen because
//! it is the only choice that cannot invent an alpha value that appears
//! nowhere in the mask — a bilinear blend across a stencil mask's 0/1
//! boundary would produce half-transparent edge texels the document
//! never asked for. It also matches the spirit of `/Interpolate` being
//! an explicit opt-in: smoothing is something a PDF asks for, not
//! something a reader supplies. `fuzzy, never sneaky` applies to alpha
//! too.
//!
//! ## Polarity — the classic silent-inversion bug
//!
//! Every mechanism here has a polarity switch, and every one of them
//! defaults to the *opposite* of the "1 means set" bitmap intuition:
//!
//! - **Explicit mask** (§8.9.6.3 + §8.9.6.2): with the default
//!   `/Decode [0 1]`, a sample value of **0 marks** — i.e. the base
//!   image **is** painted there — and **1 masks out**, leaving the
//!   previous page contents. `/Decode [1 0]` reverses both.
//! - **Soft mask**: the mask sample, after its own `/Decode`, **is** the
//!   alpha: 0.0 fully transparent, 1.0 fully opaque. `/Decode [1 0]`
//!   inverts it.
//! - **Colour key**: a sample is masked (**not** painted) when *all* of
//!   its components fall inside the ranges — the ranges name what
//!   *disappears*, not what survives.
//!
//! Getting any of these backwards produces a photographic negative of
//! the transparency: the picture shows exactly where it should not.
//! Each has a dedicated fixture in `fixtures/synthetic/transparency/`.
//!
//! ## What is refused rather than approximated
//!
//! A mask pdfce cannot decode does **not** silently become "opaque and
//! never mind". It returns a [`MaskRefusal`] whose [`MaskRefusal::key`]
//! is a stable diagnostic name, the base image is drawn opaque, and the
//! caller counts it under `Diagnostics::images_mask_unsupported`. That
//! is the same contract `image::ImageError` has for colour: a shortfall
//! is named, never absorbed.

use pdfce_core::graph::ObjectGraph;
use pdfce_core::image_codec::{self, Codec, MAX_IMAGE_PIXELS};
use pdfce_core::object::{Dict, Object};
use pdfce_core::view::DocumentView;

use crate::image::{decode_pairs, read_sample, resolve_space, row_stride};

/// Why a `/SMask` or `/Mask` could not be turned into alpha.
///
/// Every variant means **the base image was drawn fully opaque** — the
/// same visual outcome the pre-transparency build had, but named and
/// counted instead of silent. The distinction from
/// [`crate::image::ImageError`] is deliberate: an `ImageError` means the
/// picture is *missing*, a `MaskRefusal` means the picture is *there but
/// too opaque*.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MaskRefusal {
    /// `/SMask` or `/Mask` resolved to something that is not a stream
    /// (and, for `/Mask`, not an array either). `/SMask /None` — which
    /// belongs in an `ExtGState`, not an image dictionary — lands here.
    #[error("the mask entry is not an image stream")]
    NotAStream,
    /// The mask dictionary is internally inconsistent: a missing or
    /// non-positive `/Width`//`/Height`, a `/Mask` stream without
    /// `/ImageMask true` (§8.9.6.3 requires it), a soft mask with more
    /// than one colour component.
    #[error("malformed mask: {0}")]
    Malformed(&'static str),
    /// The mask's samples could not be decoded — an unimplemented codec,
    /// a broken filter chain, or bytes that are not what they claim.
    #[error("mask data could not be decoded: {0}")]
    Undecodable(String),
    /// A soft mask whose `/ColorSpace` is not a single-component space.
    ///
    /// Table 145 is unusually blunt here — `ColorSpace`: "**Required;
    /// shall be `DeviceGray`**" — so a three-component soft mask is
    /// non-conformant, not merely unsupported. pdfce is deliberately a
    /// shade more permissive than the letter of that rule and accepts
    /// any **single-component** space (`DeviceGray`, `CalGray`,
    /// `ICCBased` with `/N 1`), because those are indistinguishable at
    /// the sample level and real producers emit all three; widening
    /// further is where it stops, since there is no defined meaning for
    /// reducing three components to one alpha and guessing one
    /// (luminance? the red channel? the maximum?) would be an invention.
    #[error("soft mask colour space {0} is not a single-component space")]
    UnsupportedColorSpace(String),
    /// `Width × Height` past [`MAX_IMAGE_PIXELS`] (pdfce guard,
    /// ARCHITECTURE.md §10.1). Checked on the *mask's* own dimensions,
    /// which are attacker-controlled independently of the base image's.
    #[error("mask exceeds MAX_IMAGE_PIXELS ({MAX_IMAGE_PIXELS} pixels)")]
    TooLarge,
    /// The colour-key `/Mask` array's length was not `2 × n` for the
    /// base image's component count (§8.9.6.4), so no range test could
    /// be built. Truncating or padding it would mask the wrong colours,
    /// which is worse than masking none.
    #[error("colour-key /Mask array length is not 2 x the component count")]
    ColourKeyLength,
}

impl MaskRefusal {
    /// A stable, greppable diagnostic key.
    ///
    /// Counted **by name** for the same reason `ImageError::CodecFeature`
    /// is (decision 005 rule R27): "this file's soft mask is in a colour
    /// space pdfce refuses" and "this file's soft mask is 40 gigapixels"
    /// lead an operator to different next actions, and a single lumped
    /// counter cannot express that.
    #[must_use]
    pub const fn key(&self) -> &'static str {
        match self {
            Self::NotAStream => "mask/not-a-stream",
            Self::Malformed(_) => "mask/malformed",
            Self::Undecodable(_) => "mask/undecodable",
            Self::UnsupportedColorSpace(_) => "mask/colour-space",
            Self::TooLarge => "mask/too-large",
            Self::ColourKeyLength => "mask/colour-key-length",
        }
    }
}

/// Per-texel alpha at the **mask's** own resolution, ready to be sampled
/// across a base image of any size.
///
/// Always 8-bit, whatever the mask's own `BitsPerComponent`: alpha is
/// consumed by `tiny_skia::PremultipliedColorU8`, which is 8-bit, so
/// carrying 16-bit alpha further than the decode loop would buy nothing
/// and would double the buffer. The narrowing happens once, here, at the
/// point the samples are read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlphaPlane {
    /// Samples per row.
    width: u32,
    /// Rows.
    height: u32,
    /// `width × height` alphas, row-major, **row 0 at the top** — the
    /// same order [`crate::image`] produces texels in, and the same order
    /// §8.9.3 orders samples in.
    alpha: Vec<u8>,
}

impl AlphaPlane {
    /// Wrap an already-8-bit alpha buffer (the JPX in-codestream opacity
    /// channel, `CodedImage::embedded_alpha`).
    ///
    /// Returns `None` for a zero dimension or a buffer shorter than
    /// `width × height`; a short opacity channel is a codec bug, and
    /// padding it with zeros would silently erase the tail of the image.
    #[must_use]
    pub fn from_bytes(width: u32, height: u32, alpha: Vec<u8>) -> Option<Self> {
        let want = usize::try_from(u64::from(width).checked_mul(u64::from(height))?).ok()?;
        if want == 0 || alpha.len() < want {
            return None;
        }
        Some(Self {
            width,
            height,
            alpha,
        })
    }

    /// The mask's own pixel dimensions.
    #[must_use]
    pub const fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Alpha for base-image texel `(bx, by)` of a `bw × bh` base image.
    ///
    /// ## The mapping
    ///
    /// Both images occupy §8.9.4's unit square, so base texel `bx`
    /// spans `[bx/bw, (bx+1)/bw)` horizontally and its **centre** is at
    /// `(bx + ½)/bw`. The mask sample containing that centre is
    /// `floor((bx + ½)/bw × mw)`, computed in integer arithmetic as
    /// `((2·bx + 1) · mw) / (2·bw)` so no float rounding can put a
    /// boundary texel on the wrong side.
    ///
    /// The `==` fast path is not merely an optimization: it makes the
    /// overwhelmingly common equal-dimensions case exactly a direct
    /// index, so a producer that matched the dimensions (as pdfce's own
    /// `image_import` always does) gets a bit-exact 1:1 mapping with no
    /// dependence on the rounding rule above.
    ///
    /// Out-of-range reads return **255 (opaque)** rather than 0. A mask
    /// that cannot be consulted must not make content disappear —
    /// "invisible" is the failure mode an operator cannot see, so the
    /// safe direction is toward showing too much.
    #[must_use]
    pub fn at(&self, bx: u32, by: u32, bw: u32, bh: u32) -> u8 {
        let (mx, my) = if self.width == bw && self.height == bh {
            (bx, by)
        } else {
            (
                Self::project(bx, bw, self.width),
                Self::project(by, bh, self.height),
            )
        };
        let idx = (my as usize)
            .checked_mul(self.width as usize)
            .and_then(|row| row.checked_add(mx as usize));
        idx.and_then(|i| self.alpha.get(i).copied()).unwrap_or(255)
    }

    /// One axis of the unit-square mapping described on [`Self::at`].
    fn project(index: u32, base_extent: u32, mask_extent: u32) -> u32 {
        if base_extent == 0 || mask_extent == 0 {
            return 0;
        }
        let numerator = u64::from(index)
            .saturating_mul(2)
            .saturating_add(1)
            .saturating_mul(u64::from(mask_extent));
        let projected = numerator / (2 * u64::from(base_extent));
        // `min` rather than a modulo: the arithmetic can only overshoot
        // by one at the very last texel of a shrinking map, and clamping
        // is the behaviour "the boundaries coincide" implies.
        u32::try_from(projected)
            .unwrap_or(mask_extent - 1)
            .min(mask_extent - 1)
    }
}

/// A decoded `/SMask`, plus its `/Matte` if it declared one.
#[derive(Debug, Clone, PartialEq)]
pub struct SoftMask {
    /// The alpha itself.
    pub plane: AlphaPlane,
    /// `/Matte` (Table 146) — the matte colour the **parent image's**
    /// samples were preblended with, in the parent's own colour space.
    ///
    /// `n` is the **parent's** component count, not the mask's: Table
    /// 146 says "n numbers, where n is the number of components in the
    /// colour space specified by the `ColorSpace` entry in the *parent
    /// image's* image dictionary". A `DeviceCMYK` parent therefore has a
    /// four-element `/Matte` behind a one-component soft mask, which is
    /// why the length is validated against the base image rather than
    /// here. Applied by [`undo_matte`].
    pub matte: Option<Vec<f32>>,
}

/// Undo §11.6.5.3's preblend on one sample, in place.
///
/// ## The equation
///
/// §11.6.5.3 states the **forward** transform, verbatim:
///
/// > "The preblending computation, performed independently for each
/// > component, shall be
/// >
/// > **c′ = m + α × (c − m)**
/// >
/// > where `c′` is the value to be provided in the image source data,
/// > `c` is the original image component value, `m` is the matte colour
/// > component value, and `α` is the corresponding mask sample."
///
/// A reader needs the inverse, which the spec sanctions without printing
/// ("the conforming reader may sometimes need to invert the formula
/// shown previously"):
///
/// ```text
/// c = m + (c′ − m) / α            for α ≠ 0
/// ```
///
/// ## The two `shall`s this function honours
///
/// 1. **"The resulting `c` value shall lie within the range of colour
///    component values for the image colour space."** For the device
///    spaces pdfce converts, that range is 0.0–1.0, so the result is
///    clamped. This is not defensive coding: at small α the division
///    routinely overshoots, and an unclamped component becomes a wrong
///    colour rather than a saturated one.
/// 2. **"The computation shall not malfunction because of exceptions
///    caused by overflow or division by zero"** (§11.3.2). At `α == 0`
///    the recovered colour is *undefined* (§11.2: "at any point where
///    either the shape or the opacity of an object is equal to 0.0, its
///    colour shall be undefined") and is multiplied by zero downstream,
///    so any finite value is conformant. `c = m` is the substitute
///    taken: it needs no division, it is in-gamut by Table 146's "valid
///    colour components in that colour space", and it is what the
///    forward formula itself yields at `α = 0`.
///
/// ## Ordering — why this runs where it does
///
/// §11.6.5.3: "The preblending computation shall be done in the colour
/// space specified by the **parent image's** `ColorSpace` entry… **If a
/// colour conversion is required, inversion of the preblending shall
/// precede the colour conversion.**" So the call site is between the
/// `/Decode` transform (which produces parent-colour-space components)
/// and `Space::to_rgb` (the conversion). Running it after the conversion
/// would un-premultiply in RGB using matte components expressed in, say,
/// CMYK — a plausible-looking, entirely wrong colour.
///
/// ## The residual hazard, stated rather than hidden
///
/// `1/α` amplifies both quantisation error and any lossy-codec error, so
/// a nearly-transparent sample recovers a nearly-arbitrary colour. That
/// is inherent to the representation, not to this implementation — the
/// information genuinely is not in the file — and it is invisible in the
/// result precisely because such samples are then composited at nearly
/// zero opacity. Recorded here so a future parity investigation over a
/// `/Matte` image does not mistake it for a bug.
///
/// `matte` shorter than `count` leaves the surplus components untouched;
/// a mismatch is rejected by the caller before this is reached, so that
/// path exists only to keep the function total.
pub fn undo_matte(comps: &mut [f32], count: usize, matte: &[f32], alpha: u8) {
    if alpha == 0 {
        // c = m. No division, in-gamut, and equal to what the forward
        // formula produces at α = 0.
        for (slot, &m) in comps.iter_mut().take(count).zip(matte) {
            *slot = m;
        }
        return;
    }
    let a = f32::from(alpha) / 255.0;
    for (slot, &m) in comps.iter_mut().take(count).zip(matte) {
        *slot = (m + (*slot - m) / a).clamp(0.0, 1.0);
    }
}

/// Decode an `/SMask` image XObject into alpha (§8.9.5 Table 89,
/// §11.6.5.3, Table 145).
///
/// A soft mask is an ordinary sampled image in every respect except
/// what its samples *mean*: after the standard §8.9.5.2 `/Decode`
/// transform the value is not a colour, it is the base image's alpha,
/// with 0.0 fully transparent and 1.0 fully opaque.
///
/// ## The polarity trap, named once
///
/// A soft mask's decoded **0.0 is invisible**. A stencil mask's decoded
/// **0 is ink** (§8.9.6.2). The two masking mechanisms in this module
/// therefore have **exactly opposite** senses for the same sample value,
/// which is why they are separate functions with separate fixtures
/// rather than one parameterised routine.
///
/// ## Why this does not simply call [`crate::image::decode`]
///
/// Three reasons, each of which would be a real bug if ignored:
///
/// 1. **Recursion — bounded by the spec, not merely by this code.**
///    Table 145 lists `SMask`: "**Shall be absent**" (and `Mask`:
///    "**Shall be absent**"), so a conformant soft mask cannot carry one
///    and the nesting depth is exactly 1. Routing through the general
///    decoder would honour a non-conformant nested `/SMask` and let a
///    self-referential pair recurse until the stack ran out. This
///    function never looks at the mask's own mask entries, so the bound
///    is structural rather than a guard that can be forgotten.
/// 2. **Cost.** The general decoder builds a full RGBA pixmap; a soft
///    mask needs one byte per sample. For a 4000×4000 mask that is 16 MB
///    against 64 MB, for a result that would then be thrown away.
/// 3. **Honesty.** The general decoder's contract is "a colour space I
///    cannot convert means nothing is drawn". A soft mask's contract is
///    different: the colour space must be *single-component*, and a
///    three-component one is not an unsupported space but a malformed
///    mask.
///
/// # Errors
///
/// [`MaskRefusal`] — see its variants. Every one means the base image is
/// drawn opaque and the refusal is counted by name.
pub fn soft_mask_plane(
    doc: &DocumentView<'_>,
    entry: &Object,
    resources: &Dict,
) -> Result<SoftMask, MaskRefusal> {
    let (dict, raw) = mask_stream(doc, entry)?;
    let (width, height) = mask_dimensions(doc, dict)?;

    // Table 145, `ImageMask`: "Shall be false or absent." §8.9.6.2's
    // "the image IS the mask" and §11.6.5.3's "the image is the ALPHA of
    // another image" are different constructs with opposite polarities,
    // and an `/SMask` claiming to be an `/ImageMask` is neither. Refuse
    // rather than pick one interpretation.
    //
    // Note that a genuinely 1-bit `/SMask` is legal and is NOT this
    // case: Table 145 says `BitsPerComponent` is "Required" and imposes
    // no value restriction, so Table 89's 1/2/4/8/16 all apply. Such a
    // mask is a two-level alpha, read through the ordinary §8.9.5.2
    // transform below, and must not be routed to the stencil path.
    if matches!(
        dict.get(b"ImageMask").map(|o| doc.resolve(o)),
        Some(Object::Boolean(true))
    ) {
        return Err(MaskRefusal::Malformed(
            "/SMask carries /ImageMask true (a stencil, not a soft mask)",
        ));
    }

    let coded = image_codec::decode_image_view(doc, dict, raw, false)
        .map_err(|e| MaskRefusal::Undecodable(e.to_string()))?;

    // Table 145: `ColorSpace` is "Required; shall be DeviceGray". pdfce
    // accepts any space that carries ONE component per sample —
    // `DeviceGray`, `CalGray`, and `ICCBased` with `/N 1` — because
    // those are indistinguishable at the sample level and real producers
    // emit all three. Anything wider is refused; see
    // `MaskRefusal::UnsupportedColorSpace` for why widening stops there.
    //
    // A JPX soft mask is the one case where `/ColorSpace` may be absent.
    // Table 145 restates it as Required without repeating Table 89's
    // "except those that use the JPXDecode filter" exemption; treating
    // that omission as a withdrawal of the exemption would refuse a
    // conformant JPX soft mask, so Table 89's more specific filter rule
    // governs and the codestream's own single channel defines the space.
    // The component check below covers it either way.
    let space = match dict.get(b"ColorSpace").map(|o| doc.resolve(o)) {
        Some(obj) => Some(
            resolve_space(doc, obj, resources, 0)
                .map_err(|e| MaskRefusal::UnsupportedColorSpace(e.to_string()))?,
        ),
        None if coded.codec == Some(Codec::Jpx) => None,
        None => return Err(MaskRefusal::Malformed("/SMask has no /ColorSpace")),
    };
    if let Some(space) = &space
        && space.components() != 1
    {
        return Err(MaskRefusal::UnsupportedColorSpace(format!(
            "{} components",
            space.components()
        )));
    }

    // Bit depth: the dictionary's, unless a codec delivered something
    // else (`SampleLayout`'s rule in `image.rs`, restated for the one
    // component a mask has). JPX ignores the dictionary outright.
    let declared = dict
        .get(b"BitsPerComponent")
        .map(|o| doc.resolve(o))
        .and_then(Object::as_int)
        .filter(|v| matches!(v, 1 | 2 | 4 | 8 | 16))
        .map(|v| v as u32);
    let bits = match (coded.codec, declared) {
        (Some(_), _) if coded.bits_per_component > 0 => u32::from(coded.bits_per_component),
        (_, Some(v)) => v,
        (None, None) => return Err(MaskRefusal::Malformed("/SMask has no /BitsPerComponent")),
        (Some(_), None) => 8,
    };
    let sample_width = if coded.codec.is_some() && coded.width > 0 {
        coded.width
    } else {
        width
    };

    // §8.9.5.2 unchanged: `y = Dmin + x·(Dmax − Dmin)/(2ⁿ − 1)`. Table
    // 145 restricts only the DEFAULT (`[0 1]`), not the semantics, so
    // the ordinary transform applies: a mask with no `/Decode` maps
    // sample 0 to alpha 0 (transparent) and the maximum sample to alpha
    // 1 (opaque). `/Decode [1 0]` is the sanctioned inversion and MUST
    // survive as a negative slope — the same trap `image.rs` names for
    // colour.
    let max_sample = ((1u32 << bits.min(16)) - 1) as f32;
    let (dmin, dmax) = match decode_pairs(dict) {
        Some(pairs) => pairs.first().copied().unwrap_or((0.0, 1.0)),
        None => (0.0, 1.0),
    };
    let slope = (dmax - dmin) / max_sample;

    let stride = row_stride(sample_width, 1, bits).map_err(|_| MaskRefusal::TooLarge)?;
    let mut alpha = vec![0u8; (width as usize).saturating_mul(height as usize)];
    for y in 0..height as usize {
        let row_bit_base = y.saturating_mul(stride).saturating_mul(8);
        for x in 0..width as usize {
            let raw = read_sample(&coded.samples, row_bit_base + x * bits as usize, bits);
            let value = (dmin + raw as f32 * slope).clamp(0.0, 1.0);
            if let Some(slot) = alpha.get_mut(y * width as usize + x) {
                *slot = (value * 255.0).round() as u8;
            }
        }
    }

    Ok(SoftMask {
        plane: AlphaPlane {
            width,
            height,
            alpha,
        },
        matte: matte_components(doc, dict),
    })
}

/// Decode an explicit `/Mask` image XObject into alpha (§8.9.6.3).
///
/// ## Polarity, stated once so it can be checked once
///
/// §8.9.6.3 requires the mask to be a stencil (`/ImageMask true`), so
/// §8.9.6.2's polarity rule governs it verbatim:
///
/// > "If the `Decode` array is `[ 0 1 ]` (the default for an image
/// > mask), a sample value of **0 shall mark the page** with the current
/// > colour, and a **1 shall leave the previous contents unchanged**. If
/// > the `Decode` array is `[ 1 0 ]`, these meanings shall be reversed."
///
/// "Marks the page" for a *stencil* means "paints the fill colour"; for
/// an *explicit mask* the same sample value means "shows the base
/// image". So: **sample 0 → base image visible (alpha 255); sample 1 →
/// masked out (alpha 0)**, and `/Decode [1 0]` swaps them.
///
/// The two-step is worth spelling out because §8.9.6.3 itself **never
/// names a sample value**. It is three sentences long and defines the
/// mask as "an image mask, **as described in sub-clause 8.9.6.2**" —
/// which is where the polarity lives. A reader that greps §8.9.6.3 for
/// "0" or "1" finds nothing and is then free to invent the convention
/// they expect; that is the mechanism by which this bug gets written.
///
/// And note that it is the **opposite** of a soft mask's, where decoded
/// 0.0 is invisible. Same module, same word "mask", inverted meaning.
///
/// # Errors
///
/// [`MaskRefusal`] — the base image is drawn opaque and the refusal is
/// counted by name.
pub fn stencil_plane(doc: &DocumentView<'_>, entry: &Object) -> Result<AlphaPlane, MaskRefusal> {
    let (dict, raw) = mask_stream(doc, entry)?;
    let (width, height) = mask_dimensions(doc, dict)?;

    // §8.9.6.3: "the ImageMask entry in the mask image's dictionary shall
    // be true." A `/Mask` stream without it is not a stencil, and reading
    // an 8-bit colour image's samples as 1-bit coverage would shear the
    // mask by a factor of eight. Refuse by name.
    if !matches!(
        dict.get(b"ImageMask").map(|o| doc.resolve(o)),
        Some(Object::Boolean(true))
    ) {
        return Err(MaskRefusal::Malformed(
            "/Mask stream without /ImageMask true (§8.9.6.3 requires it)",
        ));
    }

    let coded = image_codec::decode_image_view(doc, dict, raw, false)
        .map_err(|e| MaskRefusal::Undecodable(e.to_string()))?;

    // The sample value that MASKS (hides the base image). Default
    // `/Decode [0 1]` → 0 marks the page → 0 SHOWS, so 1 hides.
    // `/Decode [1 0]` reverses it.
    let hidden_sample: u32 = match decode_pairs(dict) {
        Some(pairs) => match pairs.as_slice() {
            [(a, b)] if a > b => 0,
            _ => 1,
        },
        None => 1,
    };

    // The delivered bit depth, not the declared one — pdfce's JPX
    // adapter normalizes every depth to 8, so a conformant 1-bit JPX
    // stencil arrives as 0/255 bytes and must be read at 8 bits or it
    // unpacks eight neighbours out of every sample. Identical reasoning
    // (and identical fail-soft `!= 0` threshold) to
    // `image::decode_stencil`.
    let bits = match coded.codec {
        Some(_) if coded.bits_per_component > 0 => u32::from(coded.bits_per_component),
        Some(_) => 8,
        None => 1,
    };
    let sample_width = if coded.codec.is_some() && coded.width > 0 {
        coded.width
    } else {
        width
    };
    let stride = row_stride(sample_width, 1, bits).map_err(|_| MaskRefusal::TooLarge)?;

    let mut alpha = vec![0u8; (width as usize).saturating_mul(height as usize)];
    for y in 0..height as usize {
        let row_bit_base = y.saturating_mul(stride).saturating_mul(8);
        for x in 0..width as usize {
            let raw = read_sample(&coded.samples, row_bit_base + x * bits as usize, bits);
            let sample = u32::from(raw != 0);
            if let Some(slot) = alpha.get_mut(y * width as usize + x) {
                *slot = if sample == hidden_sample { 0 } else { 255 };
            }
        }
    }

    Ok(AlphaPlane {
        width,
        height,
        alpha,
    })
}

/// Colour-key masking: the ranges of **pre-`/Decode`** sample values that
/// vanish (§8.9.6.4).
///
/// Verbatim, because the "before decoding" clause is the whole trap:
///
/// > "an array of **2 × n integers**, `[ min1 max1 … minn maxn ]`, where
/// > n is the number of colour components in the image's colour space.
/// > **Each integer shall be in the range 0 to 2^BitsPerComponent − 1,
/// > representing colour values BEFORE decoding with the `Decode`
/// > array.** An image sample shall be masked (not painted) if **all** of
/// > its colour components before decoding, `c1 … cn`, fall within the
/// > specified ranges (that is, if `mini ≤ ci ≤ maxi` for all
/// > `1 ≤ i ≤ n`)."
///
/// Two consequences that shape where this type is used:
///
/// 1. The test cannot run on the RGBA texels — by then `/Decode` and the
///    colour conversion have both happened and the original integers are
///    gone. It runs inside `image::decode_sampled`'s pixel loop, on the
///    values [`crate::image`]'s `read_sample` just returned.
/// 2. For an `Indexed` image, `n` is **1** — the index — not the base
///    space's component count, because "the image's colour space" *is*
///    the `Indexed` space. §8.9.6.4 does not spell this out; it follows
///    from the definition and is flagged as an inference in
///    `iso32000__s__8.9.5.2.md`.
///
/// §8.9.6.4 also warns that colour-key masking over a `DCTDecode` or
/// lossy `JPXDecode` stream "can produce unexpected results" — lossy
/// round-tripping shifts sample values off the intended range. pdfce
/// applies the mask anyway (that is what the document asked for) and
/// says nothing extra: the spec's own note is about the *producer's*
/// choice, and pdfce's output matches every other reader's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColourKey {
    /// One inclusive `(min, max)` per colour component, in component
    /// order.
    ranges: Vec<(u32, u32)>,
}

impl ColourKey {
    /// Parse the `/Mask` array against an image with `components`
    /// components.
    ///
    /// # Errors
    ///
    /// [`MaskRefusal::ColourKeyLength`] when the array is not exactly
    /// `2 × components` long. Padding or truncating would mask a
    /// different set of colours than the document named, which is a
    /// worse outcome than masking none — so the mask is dropped, named
    /// and counted.
    pub fn parse(
        doc: &DocumentView<'_>,
        entry: &Object,
        components: usize,
    ) -> Result<Self, MaskRefusal> {
        let items = doc
            .resolve(entry)
            .as_array()
            .ok_or(MaskRefusal::ColourKeyLength)?;
        if components == 0 || items.len() != components.saturating_mul(2) {
            return Err(MaskRefusal::ColourKeyLength);
        }
        let ranges = items
            .chunks_exact(2)
            .map(|pair| {
                // Negative bounds are not legal ("in the range 0 to
                // 2ⁿ−1") but cost nothing to survive: clamping at 0
                // keeps the comparison total and matches the intent of
                // any producer that emitted one.
                let lo = pair
                    .first()
                    .map(|o| doc.resolve(o))
                    .and_then(Object::as_int)
                    .unwrap_or(0)
                    .max(0) as u32;
                let hi = pair
                    .get(1)
                    .map(|o| doc.resolve(o))
                    .and_then(Object::as_int)
                    .unwrap_or(0)
                    .max(0) as u32;
                (lo, hi)
            })
            .collect();
        Ok(Self { ranges })
    }

    /// Is this sample masked out?
    ///
    /// `raw` holds the pre-`/Decode` component values in component
    /// order. **All** components must be inside their range — an "any"
    /// test would erase most of a photograph the moment one channel
    /// matched.
    ///
    /// A `raw` shorter than the range list (a codestream delivering
    /// fewer components than `/ColorSpace` promised — already counted as
    /// `codec_geometry_mismatch`) returns `false`: an incomplete test
    /// cannot establish "all components match", and the safe direction
    /// is toward showing too much.
    #[must_use]
    pub fn masks(&self, raw: &[u32]) -> bool {
        if raw.len() < self.ranges.len() {
            return false;
        }
        self.ranges
            .iter()
            .zip(raw)
            .all(|(&(lo, hi), &c)| c >= lo && c <= hi)
    }
}

// ---------------------------------------------------------------------------
// Shared plumbing
// ---------------------------------------------------------------------------

/// Resolve a `/SMask`//`/Mask` entry to `(dictionary, still-encoded bytes)`.
///
/// `doc.slice` rather than `span.slice(doc.bytes())` for the decision-018
/// reason: on an [`EditSession`](pdfce_core::edit::EditSession) view the
/// payload may live in the R45 staging half, where there is no single
/// buffer to index. A mask pdfce *just wrote* this session is exactly the
/// case that must work.
fn mask_stream<'d>(
    doc: &'d DocumentView<'_>,
    entry: &'d Object,
) -> Result<(&'d Dict, &'d [u8]), MaskRefusal> {
    let Object::Stream(stream) = doc.resolve(entry) else {
        return Err(MaskRefusal::NotAStream);
    };
    let raw = doc
        .slice(stream.data_span)
        .ok_or(MaskRefusal::Undecodable("stream bytes unavailable".into()))?;
    Ok((&stream.dict, raw))
}

/// The mask's own `/Width` and `/Height`, guard included.
///
/// The ceiling is applied to the **mask's** product, independently of
/// the base image's: a 2×2 base image may name a 60,000×60,000 soft
/// mask, and the base image's own check says nothing about it.
fn mask_dimensions(doc: &DocumentView<'_>, dict: &Dict) -> Result<(u32, u32), MaskRefusal> {
    let read = |key: &[u8]| -> Option<u32> {
        dict.get(key)
            .map(|o| doc.resolve(o))
            .and_then(Object::as_int)
            .and_then(|v| u32::try_from(v).ok())
            .filter(|&v| v > 0)
    };
    let width = read(b"Width").ok_or(MaskRefusal::Malformed("mask has no positive /Width"))?;
    let height = read(b"Height").ok_or(MaskRefusal::Malformed("mask has no positive /Height"))?;
    if u64::from(width).saturating_mul(u64::from(height)) > MAX_IMAGE_PIXELS {
        return Err(MaskRefusal::TooLarge);
    }
    Ok((width, height))
}

/// Read `/Matte` as component values, or `None` when absent.
///
/// Not acted on — see [`SoftMask::matte`] for why, and for what is
/// disclosed instead. Parsed rather than merely detected so that the
/// eventual implementation has the numbers already in hand and so that a
/// `/Matte` that is present but empty (which some producers emit) is
/// treated as absent rather than as a refusal.
fn matte_components(doc: &DocumentView<'_>, dict: &Dict) -> Option<Vec<f32>> {
    let items = dict.get(b"Matte").map(|o| doc.resolve(o))?.as_array()?;
    if items.is_empty() {
        return None;
    }
    Some(
        items
            .iter()
            .map(|o| doc.resolve(o).as_number().unwrap_or(0.0) as f32)
            .collect(),
    )
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

    fn plane(w: u32, h: u32, alpha: &[u8]) -> AlphaPlane {
        AlphaPlane::from_bytes(w, h, alpha.to_vec()).unwrap()
    }

    #[test]
    fn equal_dimensions_index_directly() {
        let p = plane(2, 2, &[0, 64, 128, 255]);
        assert_eq!(p.at(0, 0, 2, 2), 0);
        assert_eq!(p.at(1, 0, 2, 2), 64);
        assert_eq!(p.at(0, 1, 2, 2), 128);
        assert_eq!(p.at(1, 1, 2, 2), 255);
    }

    #[test]
    fn a_smaller_mask_is_stretched_over_the_base() {
        // §8.9.6.3: "the base image and the image mask need not have the
        // same resolution … their boundaries on the page will coincide."
        // A 2x1 mask over a 4x1 base gives each mask sample two base
        // texels; indexing 1:1 would read past the end for x >= 2.
        let p = plane(2, 1, &[0, 255]);
        let got: Vec<u8> = (0..4).map(|x| p.at(x, 0, 4, 1)).collect();
        assert_eq!(got, vec![0, 0, 255, 255]);
    }

    #[test]
    fn a_larger_mask_is_point_sampled_at_texel_centres() {
        // A 4x1 mask over a 2x1 base: base texel 0's centre is at 0.25,
        // which lands in mask sample 1; texel 1's centre is at 0.75,
        // which lands in mask sample 3.
        let p = plane(4, 1, &[10, 20, 30, 40]);
        assert_eq!(p.at(0, 0, 2, 1), 20);
        assert_eq!(p.at(1, 0, 2, 1), 40);
    }

    #[test]
    fn the_last_texel_never_walks_off_the_end() {
        // The one place the integer mapping can overshoot.
        let p = plane(3, 3, &[1, 2, 3, 4, 5, 6, 7, 8, 9]);
        for n in 1..=9u32 {
            assert_eq!(p.at(n - 1, n - 1, n, n), p.at(n - 1, n - 1, n, n));
            let _ = p.at(n - 1, n - 1, n, n);
        }
        // 100x100 base over a 3x3 mask: the bottom-right corner must be
        // the mask's own bottom-right sample, not a read past the end.
        assert_eq!(p.at(99, 99, 100, 100), 9);
    }

    #[test]
    fn an_unreadable_plane_reads_opaque_not_invisible() {
        // Deliberate direction of failure: content that should be hidden
        // and is not is a visible bug; content that should be visible and
        // is not is an invisible one.
        let p = AlphaPlane {
            width: 2,
            height: 2,
            alpha: vec![0, 0],
        };
        assert_eq!(p.at(1, 1, 2, 2), 255);
    }

    #[test]
    fn from_bytes_refuses_a_short_buffer() {
        assert!(AlphaPlane::from_bytes(4, 4, vec![0; 15]).is_none());
        assert!(AlphaPlane::from_bytes(0, 4, vec![0; 16]).is_none());
        assert!(AlphaPlane::from_bytes(4, 4, vec![0; 16]).is_some());
    }

    #[test]
    fn colour_key_masks_only_when_every_component_is_inside() {
        let key = ColourKey {
            ranges: vec![(0, 10), (200, 255), (0, 0)],
        };
        assert!(key.masks(&[5, 255, 0]));
        assert!(!key.masks(&[11, 255, 0]), "one component outside → painted");
        assert!(!key.masks(&[5, 199, 0]));
        assert!(!key.masks(&[5, 255, 1]));
    }

    #[test]
    fn colour_key_bounds_are_inclusive() {
        let key = ColourKey {
            ranges: vec![(10, 20)],
        };
        assert!(key.masks(&[10]));
        assert!(key.masks(&[20]));
        assert!(!key.masks(&[9]));
        assert!(!key.masks(&[21]));
    }

    #[test]
    fn colour_key_with_too_few_components_cannot_conclude_all() {
        let key = ColourKey {
            ranges: vec![(0, 255), (0, 255), (0, 255)],
        };
        assert!(!key.masks(&[0, 0]));
    }
}
