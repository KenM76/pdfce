//! # Font machinery for text rendering (decision 004)
//!
//! The `FontEnvironment` seam (004 §6.3) plus the submodules that
//! enact rules R17–R22: [`bundled`] (the 14 Foxit substitute faces,
//! provenance in `assets/fonts/PROVENANCE.md`), [`program`] (embedded
//! font-program parsing via skrifa — the ONE parser, R21), and
//! [`select`] (BaseFont-name / descriptor-driven substitute choice).
//!
//! ## The seam's contract (R19 — deterministic by default)
//!
//! `pdfce-render` never discovers, opens, or reads a font from the
//! filesystem, environment, or OS. Its default [`FontEnvironment`] is
//! the bundled 14 and nothing else: same input → same pixels on every
//! machine, in the CLI, and in the WASM fork. Additional faces arrive
//! only through this API, supplied by the shell (`pdfce-gui` /
//! `pdfce-cli` own any system-font discovery). No `cfg(target_os)`
//! appears anywhere in this crate (decision 003 R10).

pub mod bundled;
pub mod coredata;
pub mod program;
pub mod select;
/// Donor-face subsetting for FF-C (Pass 21.x, decision 021). Produces the
/// plain-data `FontEmbedPlan` that `pdfce-core::font_embed` emits from.
pub mod subset;

use pdfce_core::settings::{
    CmykIntent, CmykJpegPolarity, MaskResample, MinifyFilter, MissingAppearanceState,
};
use std::collections::HashMap;
use std::sync::Arc;

/// Shared, immutable font-program bytes.
///
/// `Arc`-backed so a face parsed once can be shared across pages and
/// threads without copying. The renderer never *obtains* bytes — it
/// only ever receives them (R19).
#[derive(Clone)]
pub struct FontData(Arc<dyn AsRef<[u8]> + Send + Sync>);

impl FontData {
    /// Wrap owned bytes.
    #[must_use]
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(Arc::new(bytes))
    }

    /// Wrap a static slice (the bundled faces — zero-copy).
    #[must_use]
    pub fn from_static(bytes: &'static [u8]) -> Self {
        Self(Arc::new(bytes))
    }

    /// The raw bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        (*self.0).as_ref()
    }
}

impl std::fmt::Debug for FontData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FontData({} bytes)", self.bytes().len())
    }
}

/// Which substitute a document font falls back to when it carries no
/// embedded program: the twelve Latin standard-14 slots plus Symbol
/// and ZapfDingbats (§9.8.1 Table 123 `Flags` drive the non-std-14
/// classification in [`select`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FallbackKey {
    /// Helvetica slot (sans-serif regular).
    Sans,
    /// Helvetica-Bold.
    SansBold,
    /// Helvetica-Oblique.
    SansItalic,
    /// Helvetica-BoldOblique.
    SansBoldItalic,
    /// Times-Roman.
    Serif,
    /// Times-Bold.
    SerifBold,
    /// Times-Italic.
    SerifItalic,
    /// Times-BoldItalic.
    SerifBoldItalic,
    /// Courier.
    Fixed,
    /// Courier-Bold.
    FixedBold,
    /// Courier-Oblique.
    FixedItalic,
    /// Courier-BoldOblique.
    FixedBoldItalic,
    /// Symbol.
    Symbol,
    /// ZapfDingbats.
    Dingbats,
}

/// The provenance of the glyphs a document font paints — the three
/// trust levels of decision 012 (rule R63), replacing the earlier
/// two-state `substituted: bool`.
///
/// The distinction is operator-facing, not cosmetic: a bundled
/// substitute is *pdfce's* plausible Base-14 shape, while a supplied
/// face is *the operator's own* deliberate choice. Both are still
/// substitutes — neither is the document's embedded program — and a
/// supplied glyph is **never** presented as embedded, nor a bundled one
/// as supplied. The decision-004 §3.6 fact holds for all three:
/// positions come from the PDF's own `/Widths`, so a supplied face
/// improves *shapes*, not *layout*.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GlyphSource {
    /// The document's own embedded font program (exact letterforms).
    Embedded,
    /// A bundled Foxit Base-14 substitute face selected by name /
    /// descriptor (plausible, but pdfce's shapes — decision 004 §4.2).
    Bundled,
    /// An operator-supplied face, matched by name through the
    /// [`FontEnvironment::named`] seam the shell filled from a font
    /// folder (decision 012 — the operator's own shapes).
    Supplied,
}

impl GlyphSource {
    /// Whether this source is a substitute (bundled or supplied) rather
    /// than the document's own embedded program — i.e. whether R20/R63
    /// disclosure applies at all.
    #[must_use]
    pub fn is_substitute(self) -> bool {
        matches!(self, Self::Bundled | Self::Supplied)
    }
}

/// The set of faces available to the renderer.
///
/// `Default` == [`FontEnvironment::bundled`]: the 14 Foxit faces and
/// nothing else (R19). The shell may layer overrides on top.
#[derive(Debug, Clone)]
pub struct FontEnvironment {
    fallbacks: HashMap<FallbackKey, FontData>,
    named: HashMap<String, FontData>,
}

impl FontEnvironment {
    /// The bundled standard-14 substitutes. Infallible, no I/O.
    #[must_use]
    pub fn bundled() -> Self {
        Self {
            fallbacks: bundled::faces(),
            named: HashMap::new(),
        }
    }

    /// Replace a fallback slot with a caller-supplied face.
    pub fn insert_fallback(&mut self, key: FallbackKey, data: FontData) {
        self.fallbacks.insert(key, data);
    }

    /// Offer a face by `BaseFont` name (e.g. a system CJK face the
    /// shell discovered), consulted before the descriptor-derived
    /// fallback.
    pub fn insert_named(&mut self, base_font: &str, data: FontData) {
        self.named.insert(base_font.to_owned(), data);
    }

    /// The face for a fallback slot (always present in a bundled or
    /// bundled-derived environment).
    #[must_use]
    pub fn fallback(&self, key: FallbackKey) -> Option<&FontData> {
        self.fallbacks.get(&key)
    }

    /// A shell-supplied face for an exact `BaseFont` name, if any.
    #[must_use]
    pub fn named(&self, base_font: &str) -> Option<&FontData> {
        self.named.get(base_font)
    }

    /// Every shell-supplied face name, **sorted**, for a shell that needs to
    /// OFFER them rather than merely resolve one.
    ///
    /// # Why this exists
    ///
    /// [`Self::named`] answers "do you have this face?", which is all the
    /// renderer ever needs — it is handed a `/BaseFont` name by the document.
    /// A shell building a font picker has the opposite problem: it has no name
    /// to ask about, it needs the list. Before this, `pdfce-gui` could load an
    /// operator's font folder, register every face in it, render with them —
    /// and still not enumerate them, so its Add-Text font list was frozen at
    /// the fourteen Standard-14 faces and the GUI could not embed a donor for
    /// non-Latin text even though `pdfce-core` and `pdfce-cli` both could
    /// (Pass 21.0's never-started GUI slice).
    ///
    /// # Why sorted, and why owned `&str`s in a `Vec`
    ///
    /// The backing store is a `HashMap`, whose iteration order varies run to
    /// run. An unsorted list would reshuffle a font picker between launches,
    /// and — worse for this project — would make a scripted GUI run
    /// non-reproducible, which is exactly what `tools/gui-drive.ps1` depends
    /// on. A `Vec` rather than an `impl Iterator` because the sort has to
    /// materialise it anyway, so an iterator return would only hide that cost
    /// without avoiding it.
    #[must_use]
    pub fn named_faces(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.named.keys().map(String::as_str).collect();
        names.sort_unstable();
        names
    }

    /// Strip a §9.6.4 subset tag (`ABCDEF+Helvetica` → `Helvetica`) for a
    /// name lookup: exactly six uppercase letters then `+`; anything else is
    /// returned unchanged.
    ///
    /// Hoisted here (Pass 14.3 UI spec §7) so the CLI (`pdfce-cli`) and the
    /// GUI (`pdfce-gui`) share ONE copy of the Bundled-vs-Supplied name
    /// normalization rather than each carrying a private, drift-prone regex.
    /// `FontEnvironment` already owns the registry the classification
    /// consults, so it is the natural home — and it keeps `pdfce-render`
    /// GUI-dependency-free (the load-bearing separation, `ARCHITECTURE.md`
    /// §3), which a `pdfce-gui`-side helper would not.
    #[must_use]
    pub fn subset_stem(base_font: &str) -> &str {
        match base_font.split_once('+') {
            Some((tag, rest)) if tag.len() == 6 && tag.bytes().all(|b| b.is_ascii_uppercase()) => {
                rest
            }
            _ => base_font,
        }
    }

    /// Classify a **non-embedded** run's `base_font` into the operator-facing
    /// [`GlyphSource::Supplied`] vs [`GlyphSource::Bundled`] trust level: it
    /// is `Supplied` when this environment has a shell-registered
    /// [`Self::named`] face for the name (matched by its subset-stripped stem
    /// or exactly), else `Bundled` (a plausible pdfce Base-14 substitute).
    ///
    /// This is the ONE copy of the refinement decision-012's shell applies on
    /// top of `pdfce-core`'s Embedded/NonEmbedded report (Pass 14.1 judgment
    /// call #1): core reports Embedded/NonEmbedded only; the shell — CLI and
    /// GUI alike — refines NonEmbedded here. It returns `Supplied`/`Bundled`
    /// only; an Embedded run never reaches this (the caller already knows it
    /// is [`GlyphSource::Embedded`]). Either way it is a SHAPE-only
    /// distinction: positions still come from the PDF's own `/Widths`
    /// (decision 004 §3.6).
    #[must_use]
    pub fn classify_nonembedded(&self, base_font: &str) -> GlyphSource {
        if self.named(Self::subset_stem(base_font)).is_some() || self.named(base_font).is_some() {
            GlyphSource::Supplied
        } else {
            GlyphSource::Bundled
        }
    }
}

impl Default for FontEnvironment {
    fn default() -> Self {
        Self::bundled()
    }
}

/// Per-render knobs (decision 004 §6.3).
///
/// `#[non_exhaustive]` so later Passes can add image, annotation and
/// overprint options without a breaking change — and so that callers
/// construct it through [`Default`] plus field assignment, which keeps
/// every future addition source-compatible.
///
/// The default is [`FontEnvironment::bundled`] plus **annotations on**,
/// which is what makes rendering reproducible on any machine (R19) and
/// matches what a reader shows by default (a document's stamps, markup and
/// form-field appearances are part of the page).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RenderOptions {
    /// The faces available to the renderer. Replace or extend this to
    /// hand the renderer a shell-discovered system face — the renderer
    /// itself never goes looking.
    pub fonts: FontEnvironment,
    /// Whether to paint annotation appearances (`/AP` `/N`) over the page
    /// content (Pass 6.0, ISO 32000-1 §12.5). **Default `true`** — a
    /// reader shows annotations. Set `false` to reproduce the pre-6.0
    /// content-only raster (the CLI's `render-page --no-annotations` and
    /// the GUI's annotation-visibility toggle), which keeps the round-trip
    /// raster oracle's self-comparison and any A/B baseline reproducible.
    pub annotations: bool,
    /// An optional flag the render polls between operators so a caller
    /// can abandon it in flight ([`crate::cancel::RenderCancel`]).
    ///
    /// **`None` by default**, which is not merely a neutral default: it
    /// means every existing caller — the CLI, the round-trip oracle,
    /// the R85 preview-equals-saved harness — keeps a render that cannot
    /// be interrupted, so none of them can acquire a new failure mode
    /// from this field existing. Only a caller that opts in can be
    /// cancelled.
    pub cancel: Option<crate::cancel::RenderCancel>,
    /// How `DeviceCMYK` is converted to sRGB for display
    /// (ISO 32000-1 §8.6.4.4).
    ///
    /// **Default [`CmykIntent::Calibrated`]** — agreement with what
    /// Acrobat's default profile and pdfium produce. §8.6.4.4 mandates no
    /// conversion at all, so this is a choice rather than a fact, which is
    /// exactly why it is a knob: it is the operator's call, and pdfce's
    /// job is to default it to what is usually followed.
    ///
    /// The visible consequence of the default is that solid black ink
    /// (`0 0 0 1 k`) renders `#231F20` rather than `#000000`.
    /// [`CmykIntent::NeutralBlack`] is the answer for CAD and line
    /// drawings, where every stroke is pure K and true black is expected.
    pub cmyk_intent: CmykIntent,
    /// Which filter resamples a size-mismatched `/SMask` or explicit
    /// `/Mask` (spec ambiguity `SM-A1`, §8.9.6.3 / Table 145).
    ///
    /// **Default [`MaskResample::Nearest`]** — the shipped behaviour.
    /// **Evidence tier (d)**: a reasoned guess, not a sourced claim. The
    /// spec fixes where the two grids line up and says nothing at all
    /// about the filter, and no Acrobat citation, census, or documented
    /// third-party behaviour exists for this question.
    pub mask_resample: MaskResample,
    /// How an image drawn smaller than its own pixel grid is sampled
    /// (spec ambiguity `IM-A1`, §8.9.5.3).
    ///
    /// **Default [`MinifyFilter::PointSample`]** — the shipped behaviour.
    /// **Evidence tier (d)**: a guess. §8.9.5.3 defines interpolation only
    /// for magnification and never mentions minification, so
    /// `/Interpolate false` does not actually legislate this direction.
    pub image_minify: MinifyFilter,
    /// How a four-component JPEG that declares no `/Decode` is read
    /// (spec ambiguity `DCT-A1`, §7.4.8 + Table 13).
    ///
    /// **Default [`CmykJpegPolarity::NeverInvert`]** — the shipped
    /// behaviour and standing rule R29. **Evidence tier (c)**, the
    /// strongest-sourced default in the ambiguity register.
    pub cmyk_jpeg_polarity: CmykJpegPolarity,
    /// What to paint for an annotation whose `/AP` `/N` is a multi-entry
    /// subdictionary with no `/AS` (spec ambiguity `AS-A1`, §12.5.5).
    ///
    /// **Default [`MissingAppearanceState::PaintNothing`]** — the shipped
    /// behaviour. **Evidence tier (d)**: a guess, and deliberately the
    /// conservative one; the alternatives are empirical guesses that would
    /// put a plausible appearance on screen with nothing to say pdfce
    /// picked it.
    pub missing_as: MissingAppearanceState,
}

/// The subset of [`RenderOptions`] that has to reach the interpreter and
/// the annotation walk — every operator setting whose effect is a
/// rendering decision.
///
/// # Why this is a struct and not four more parameters
///
/// [`crate::interpret::run`] already carries an
/// `#[allow(clippy::too_many_arguments)]` whose comment explains that its
/// parameters are `RenderOptions` *decomposed into the pieces the
/// interpreter actually uses*. R169 turns one such piece (the CMYK intent)
/// into four, and four scalars threaded independently through `run`,
/// `run_nested`, `run_form_at`, `trace_paths` and the annotation walk is
/// four chances for one of them to be dropped at a recursion seam —
/// silently, because a dropped setting looks exactly like a setting the
/// operator never changed.
///
/// # Why a parameter and not a global
///
/// Two renders of the same page must never differ for a reason invisible
/// at the call site. That is the property `tools/render-parity` depends
/// on, and a `static` or thread-local would destroy it: a render's output
/// would depend on when the settings file was last read, which is not a
/// question a caller can answer or a test can pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct RenderPolicy {
    /// See [`RenderOptions::cmyk_intent`].
    pub cmyk_intent: CmykIntent,
    /// See [`RenderOptions::mask_resample`].
    pub mask_resample: MaskResample,
    /// See [`RenderOptions::image_minify`].
    pub image_minify: MinifyFilter,
    /// See [`RenderOptions::cmyk_jpeg_polarity`].
    pub cmyk_jpeg_polarity: CmykJpegPolarity,
    /// See [`RenderOptions::missing_as`]. Read by the annotation walk
    /// ([`crate::annot`]), not by the content-stream interpreter — it
    /// travels here because it is one of the same operator's rendering
    /// choices and splitting the bundle by consumer would mean two
    /// bundles that must be kept in step.
    pub missing_as: MissingAppearanceState,
}

impl Default for RenderOptions {
    /// [`FontEnvironment::bundled`] with annotation painting **on** —
    /// annotations must NOT default off, or `render_page` (the no-options
    /// entry point every existing caller uses) would silently stop
    /// showing a document's markup.
    fn default() -> Self {
        Self {
            fonts: FontEnvironment::default(),
            annotations: true,
            cancel: None,
            // Every R169 knob reads its default off the enum that models
            // the choice, never a literal — so `RenderOptions::default()`,
            // `Settings::default()` and the generated settings file's own
            // comments cannot come to disagree about what pdfce does out
            // of the box. `settings_defaults_match_render_defaults` in
            // this module pins that.
            cmyk_intent: CmykIntent::default(),
            mask_resample: MaskResample::default(),
            image_minify: MinifyFilter::default(),
            cmyk_jpeg_polarity: CmykJpegPolarity::default(),
            missing_as: MissingAppearanceState::default(),
        }
    }
}

impl RenderOptions {
    /// Set whether annotation appearances are painted (Pass 6.0, §12.5),
    /// returning `self` for chaining.
    ///
    /// A consuming builder rather than direct field assignment because
    /// [`RenderOptions`] is `#[non_exhaustive]`: an out-of-crate caller
    /// cannot use struct-update syntax to flip one field, and the
    /// `let mut o = default(); o.annotations = false;` form trips
    /// `clippy::field_reassign_with_default`. This keeps the one common
    /// tweak a single readable expression:
    /// `RenderOptions::default().with_annotations(false)`.
    #[must_use]
    pub fn with_annotations(mut self, annotations: bool) -> Self {
        self.annotations = annotations;
        self
    }

    /// Set how `DeviceCMYK` is converted for display (§8.6.4.4),
    /// returning `self` for chaining.
    ///
    /// Same `#[non_exhaustive]` reasoning as [`Self::with_annotations`].
    /// This is the seam the operator's persisted setting arrives through:
    /// `RenderOptions::default().with_cmyk_intent(settings.cmyk_intent)`.
    #[must_use]
    pub fn with_cmyk_intent(mut self, intent: CmykIntent) -> Self {
        self.cmyk_intent = intent;
        self
    }

    /// Attach a cancellation flag, returning `self` for chaining.
    ///
    /// Same consuming-builder reason as [`Self::with_annotations`]:
    /// [`RenderOptions`] is `#[non_exhaustive]`, so an out-of-crate
    /// caller cannot reach the field with struct-update syntax.
    #[must_use]
    pub fn with_cancel(mut self, cancel: crate::cancel::RenderCancel) -> Self {
        self.cancel = Some(cancel);
        self
    }

    /// Set the mask resampling filter (`SM-A1`), returning `self` for
    /// chaining. Same `#[non_exhaustive]` reasoning as
    /// [`Self::with_annotations`].
    #[must_use]
    pub fn with_mask_resample(mut self, filter: MaskResample) -> Self {
        self.mask_resample = filter;
        self
    }

    /// Set the image minification filter (`IM-A1`), returning `self` for
    /// chaining.
    #[must_use]
    pub fn with_image_minify(mut self, filter: MinifyFilter) -> Self {
        self.image_minify = filter;
        self
    }

    /// Set the CMYK-JPEG polarity rule (`DCT-A1`), returning `self` for
    /// chaining.
    #[must_use]
    pub fn with_cmyk_jpeg_polarity(mut self, polarity: CmykJpegPolarity) -> Self {
        self.cmyk_jpeg_polarity = polarity;
        self
    }

    /// Set the missing-`/AS` policy (`AS-A1`), returning `self` for
    /// chaining.
    #[must_use]
    pub fn with_missing_as(mut self, policy: MissingAppearanceState) -> Self {
        self.missing_as = policy;
        self
    }

    /// The rendering-decision subset of these options, as the one value
    /// the interpreter and the annotation walk thread down.
    ///
    /// Deliberately a *projection* rather than a stored field: the
    /// builders above set individual options, and a stored bundle would
    /// have to be rebuilt by every one of them or go stale. Projecting on
    /// demand makes staleness unrepresentable.
    #[must_use]
    pub const fn policy(&self) -> RenderPolicy {
        RenderPolicy {
            cmyk_intent: self.cmyk_intent,
            mask_resample: self.mask_resample,
            image_minify: self.image_minify,
            cmyk_jpeg_polarity: self.cmyk_jpeg_polarity,
            missing_as: self.missing_as,
        }
    }
}

#[cfg(test)]
mod render_policy_tests {
    use super::{RenderOptions, RenderPolicy};

    #[test]
    fn settings_defaults_match_render_defaults() {
        // The two halves of every R169 knob: `Settings` is what the
        // operator's file says, `RenderOptions` is what the renderer does
        // when nobody said anything. If they disagree, "the default"
        // silently means two different things depending on whether a
        // settings file happens to exist — which is the exact failure the
        // settings module's own docs warn about, one crate boundary over.
        let settings = pdfce_core::settings::Settings::default();
        let options = RenderOptions::default();
        assert_eq!(settings.cmyk_intent, options.cmyk_intent);
        assert_eq!(settings.mask_resample, options.mask_resample);
        assert_eq!(settings.image_minify, options.image_minify);
        assert_eq!(settings.cmyk_jpeg_polarity, options.cmyk_jpeg_polarity);
        assert_eq!(settings.missing_as, options.missing_as);
    }

    #[test]
    fn the_policy_projection_carries_every_field() {
        // A field added to `RenderPolicy` but forgotten in `policy()`
        // would compile and would silently ignore the operator's choice.
        // Building a non-default options value and comparing the whole
        // projection catches that without naming the fields twice.
        let options = RenderOptions::default()
            .with_cmyk_intent(pdfce_core::settings::CmykIntent::Naive)
            .with_mask_resample(pdfce_core::settings::MaskResample::Bilinear)
            .with_image_minify(pdfce_core::settings::MinifyFilter::Smooth)
            .with_cmyk_jpeg_polarity(pdfce_core::settings::CmykJpegPolarity::InvertOnApp14)
            .with_missing_as(pdfce_core::settings::MissingAppearanceState::FirstEntry);
        assert_eq!(
            options.policy(),
            RenderPolicy {
                cmyk_intent: pdfce_core::settings::CmykIntent::Naive,
                mask_resample: pdfce_core::settings::MaskResample::Bilinear,
                image_minify: pdfce_core::settings::MinifyFilter::Smooth,
                cmyk_jpeg_polarity: pdfce_core::settings::CmykJpegPolarity::InvertOnApp14,
                missing_as: pdfce_core::settings::MissingAppearanceState::FirstEntry,
            }
        );
        assert_ne!(options.policy(), RenderPolicy::default());
    }
}
