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
}
