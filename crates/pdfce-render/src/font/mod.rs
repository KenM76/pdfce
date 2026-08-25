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

// The annotation scope lives in `crate::annot` — beside the markup
// classification it selects over and the walk that enforces it — rather
// than here, so the sourced Table 169 partition and the type that consumes
// it cannot drift apart across two files.
use crate::annot::AnnotationScope;
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

    /// Resolve a document's `/BaseFont` to a face that could be **embedded**
    /// into the file for it (Pass 67.0 phase E).
    ///
    /// # Why this lives here and not in either shell
    ///
    /// `pdfce-core` decides what may lawfully be written into a font
    /// dictionary; it must never learn what fonts a machine has (project
    /// rule 2 — the GUI-core separation is what keeps the WASM fork a
    /// shell-crate swap). So the name→bytes step is the shell's. But BOTH
    /// shells need it and they must not disagree, so it lives on the type
    /// that already owns the registry — the same argument that put
    /// [`Self::subset_stem`] and [`Self::classify_nonembedded`] here rather
    /// than in `pdfce-gui`.
    ///
    /// # The ladder, in order, and why each rung is where it is
    ///
    /// | Rung | Match | Reported as |
    /// |---|---|---|
    /// | 1 | a registered face whose name is the `/BaseFont` verbatim | `Exact` |
    /// | 2 | a registered face whose name is the `/BaseFont` with its §9.6.4 subset tag stripped | `Exact` |
    /// | 3 | a registered face named by [`select::candidate_names`] for the standard-14 slot the name denotes | `Alias` |
    /// | 4 | pdfce's own bundled substitute for that slot, if `allow_bundled` | `Bundled` |
    ///
    /// Rungs 1 and 2 are both `Exact` because a subset tag is a statement
    /// about the *program that used to be here*, not about the face: after
    /// [`Self::subset_stem`], `ABCDEF+Arial` and `Arial` name the same face,
    /// and calling the match inexact would misreport it.
    ///
    /// Rung 4 is opt-in rather than automatic. The bundled faces are
    /// BSD-3-Clause (pdfium's Foxit-origin set, see
    /// `THIRD_PARTY_LICENSES.md`), and **embedding one puts it inside a
    /// document the operator then distributes** — a different act from using
    /// it to draw pixels on their own screen, and one that carries the
    /// licence's attribution condition with it. Whether to accept that is
    /// the operator's decision, so the shells expose it as a flag and never
    /// take it silently.
    ///
    /// # What this does NOT do
    ///
    /// It does not read a `/FontDescriptor`. [`select::by_descriptor`]
    /// classifies an unrecognised name by its Table 123 flags, which is
    /// exactly right for *rendering* — any plausible face beats a blank —
    /// and wrong for embedding, where the result is written into the
    /// document permanently. A font pdfce cannot name is a font pdfce
    /// reports as unresolved.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfce_render::{FontData, FontEnvironment};
    /// use pdfce_render::font::EmbedMatch;
    ///
    /// let mut env = FontEnvironment::bundled();
    /// env.insert_named("ArialMT", FontData::new(vec![0u8; 4]));
    ///
    /// // The document says `Helvetica`; the folder has Arial.
    /// let hit = env.resolve_for_embedding("Helvetica", false).expect("alias");
    /// assert_eq!(hit.quality, EmbedMatch::Alias);
    /// assert_eq!(hit.face_name, "ArialMT");
    ///
    /// // A subset tag does not make the match inexact.
    /// let hit = env.resolve_for_embedding("ABCDEF+ArialMT", false).expect("exact");
    /// assert_eq!(hit.quality, EmbedMatch::Exact);
    ///
    /// // Nothing answers to this, and the bundled faces are not offered.
    /// assert!(env.resolve_for_embedding("Wingdings", false).is_none());
    /// ```
    #[must_use]
    pub fn resolve_for_embedding(
        &self,
        base_font: &str,
        allow_bundled: bool,
    ) -> Option<EmbedDonor<'_>> {
        if let Some(data) = self.named(base_font) {
            return Some(EmbedDonor {
                data,
                face_name: base_font.to_owned(),
                quality: EmbedMatch::Exact,
            });
        }
        let stem = Self::subset_stem(base_font);
        if let Some(data) = self.named(stem) {
            return Some(EmbedDonor {
                data,
                face_name: stem.to_owned(),
                quality: EmbedMatch::Exact,
            });
        }
        let key = select::by_name(base_font)?;
        for candidate in select::candidate_names(key) {
            if let Some(data) = self.named(candidate) {
                return Some(EmbedDonor {
                    data,
                    face_name: (*candidate).to_owned(),
                    quality: EmbedMatch::Alias,
                });
            }
        }
        if !allow_bundled {
            return None;
        }
        let data = self.fallback(key)?;
        Some(EmbedDonor {
            data,
            face_name: bundled::face_name(key).to_owned(),
            quality: EmbedMatch::Bundled,
        })
    }
}

/// How [`FontEnvironment::resolve_for_embedding`] reached a donor.
///
/// The provenance `pdfce_core::font_embed_missing::FontMatch` mirrors. Two
/// enums rather than one because the crate boundary is load-bearing:
/// `pdfce-core` must not depend on `pdfce-render`, so neither can name the
/// other's type, and a shell converts between them in one line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EmbedMatch {
    /// The face answers to the name the document spells (with or without a
    /// §9.6.4 subset tag).
    Exact,
    /// The face was reached through a standard-14 family equivalence.
    Alias,
    /// One of pdfce's own bundled substitute faces.
    Bundled,
}

/// A face a shell may embed for a document font, with the provenance of the
/// match.
#[derive(Debug, Clone)]
pub struct EmbedDonor<'a> {
    /// The program bytes.
    pub data: &'a FontData,
    /// The name the face was matched under — what an operator recognises.
    ///
    /// Owned rather than borrowed: three of the four rungs produce a name
    /// that is not a sub-slice of the input (the registry key, a candidate
    /// from a static table, a bundled face's own label), so a borrow would
    /// have to be `'static` on some rungs and input-lifetimed on others.
    pub face_name: String,
    /// How it was reached.
    pub quality: EmbedMatch,
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
/// The default is [`FontEnvironment::bundled`] plus **annotations on, every
/// class** ([`AnnotationScope::DocumentAndMarkups`]), which is what makes
/// rendering reproducible on any machine (R19) and matches what a reader
/// shows by default (a document's stamps, markup and form-field appearances
/// are part of the page).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RenderOptions {
    /// The faces available to the renderer. Replace or extend this to
    /// hand the renderer a shell-discovered system face — the renderer
    /// itself never goes looking.
    pub fonts: FontEnvironment,
    /// Whether to paint annotation appearances (`/AP` `/N`) over the page
    /// content at all (Pass 6.0, ISO 32000-1 §12.5). **Default `true`** — a
    /// reader shows annotations. Set `false` to reproduce the pre-6.0
    /// content-only raster (the CLI's `render-page --no-annotations` and
    /// the GUI's annotation-visibility toggle), which keeps the round-trip
    /// raster oracle's self-comparison and any A/B baseline reproducible.
    ///
    /// # This is a MASTER GATE over [`Self::annotation_scope`]
    ///
    /// Two fields now describe annotation painting, and they compose in
    /// exactly one direction: `annotations = false` forces the effective
    /// scope to [`AnnotationScope::ContentOnly`] whatever
    /// [`Self::annotation_scope`] says, and `annotations = true` lets the
    /// scope decide. The composition lives in one place —
    /// [`Self::effective_annotation_scope`] — and every consumer in this
    /// crate reads it from there rather than looking at either field.
    ///
    /// The gate can therefore only ever **subtract**: it cannot cause page
    /// content to disappear (that is [`AnnotationScope::FormFieldsOnly`]'s
    /// doing, and this field suppresses annotations, not content), and it
    /// cannot make an excluded class paint. So the pre-existing meaning of
    /// `with_annotations(false)` — "reproduce the content-only raster" — is
    /// preserved *unconditionally*, which is the property the round-trip
    /// oracle depends on.
    ///
    /// # Why the `bool` was kept rather than replaced
    ///
    /// [`AnnotationScope`] could have absorbed it; `ContentOnly` is
    /// precisely `annotations = false`. It was not absorbed because this
    /// struct's own contract invites `let mut o = RenderOptions::default();
    /// o.annotations = false;` (see the type docs — `#[non_exhaustive]`
    /// means field assignment is the documented way to reach a field), and
    /// deleting a `pub` field a type's documentation tells callers to
    /// assign is a breaking change dressed as a refactor. Keeping it costs
    /// one `match` arm in [`Self::effective_annotation_scope`] and buys
    /// every existing caller — including any this crate cannot see —
    /// compiling and rendering exactly as before.
    pub annotations: bool,
    /// Skip geometry too small to be seen, trading a little fidelity for a
    /// large speed-up at page-fit zoom. **Default `false`.**
    ///
    /// # What it does
    ///
    /// A Form XObject whose transformed `/BBox` is narrower than
    /// [`SUBPIXEL_CULL_PX`] in BOTH axes is not executed at all, and is
    /// counted on `Diagnostics::subpixel_culled`.
    ///
    /// # Why it is off by default, and why it exists at all
    ///
    /// It is **lossy**. Those forms are not invisible — each contributes
    /// anti-aliased coverage, and hundreds of them contribute a visible
    /// tint. Decision 082 says a render may skip work only where skipping
    /// is EXACT, and a lossy speed-up is the operator's call, so it is
    /// offered rather than taken: `Pass 74.4`'s `/BBox` cull is exact and
    /// always on; this one is neither.
    ///
    /// # Measured, on `tools/gen-scale-demo`
    ///
    /// 342 mitochondria, each a full section of ~2 500 path operators, at
    /// a page-fit zoom where every one is about 1/70th of a pixel across.
    /// All the work, none of the picture:
    ///
    /// | render | time off | time on | pixels differing |
    /// |---|---|---|---|
    /// | whole page, 1.6x | 1 468 ms | **108 ms** | **0** of 1 242 640 |
    ///
    /// ★ And the number that keeps this honest, because "zero" above
    /// would otherwise read as "free". The loss appears as the objects
    /// approach the threshold rather than sitting far below it — same
    /// page, a 1 pt window, ~339 forms dropped each time:
    ///
    /// | scale | pixels differing | worst channel delta |
    /// |---|---|---|
    /// | 20 | 18 of 400 | 16 |
    /// | 35 | 47 of 1 296 | 54 |
    /// | 60 | 82 of 3 600 | **62** — about a quarter of a channel |
    ///
    /// So it is genuinely lossy, the loss is largest exactly where the
    /// speed-up is smallest, and at the zoom where the speed-up is
    /// enormous the loss happens to be nil. That shape is why this is a
    /// switch and not a heuristic.
    ///
    /// # It is DISCLOSED, not silent (rule 4)
    ///
    /// The counter is on the metrics line whether the option is on or
    /// off, so a raster produced this way carries the number of things it
    /// left out. An operator comparing two renders can see which one paid
    /// for its speed and how much.
    pub subpixel_culling: bool,
    /// **Which classes** of annotation to paint when [`Self::annotations`]
    /// permits any — the four-way Acrobat print scope (Document / Document
    /// and Markups / Document and Stamps / Form fields only) plus pdfce's
    /// own content-only scope. See [`AnnotationScope`].
    ///
    /// **Default [`AnnotationScope::DocumentAndMarkups`]** — every class,
    /// which together with `annotations = true` is exactly the behaviour
    /// every pre-existing caller already had. A caller that never mentions
    /// this field cannot observe that it exists.
    ///
    /// Read it through [`Self::effective_annotation_scope`], never
    /// directly: this field alone does not account for the
    /// [`Self::annotations`] gate.
    pub annotation_scope: AnnotationScope,
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
    /// The operator's layer-visibility override, or `None` to obey the
    /// document's own default configuration (§8.11.4.3).
    ///
    /// **`None` by default**, and that default is load-bearing: it means
    /// every existing caller renders the document as the document asks,
    /// which is what a reader does with a file it was just handed. Only
    /// a front end that has an operator turning layers on and off sets
    /// this.
    ///
    /// Owned here and BORROWED by [`RenderPolicy`], so the policy stays
    /// `Copy`. See [`crate::LayerVisibility`] for the replace-not-merge
    /// contract.
    pub layers: Option<crate::LayerVisibility>,
    /// Apply `View`-event `/AS` usage applications at this magnification
    /// (§8.11.4.4), or `None` to render the `/D`-initial state.
    ///
    /// # ★ `None` is the PRINT answer, and it is the default for a reason
    ///
    /// §8.11.4.5 says the `/D`-initial state *"shall be the state used by
    /// printing and aggregating application[s]. Such applications **shall
    /// not** apply the changes based on usage application dictionaries"*.
    /// Only a **viewer** examines `/AS`.
    ///
    /// So this is opt-IN. A caller that has not thought about whether it
    /// is a viewer gets the print-correct answer, and the only way to
    /// apply usage is to say so — which means a print or aggregate path
    /// cannot acquire it by inheriting a default. Defaulting the other
    /// way would put a `shall not` violation one forgotten argument away.
    ///
    /// The value is a SCALE FACTOR where `1.0` is 100 %. §8.11.4.4 never
    /// defines the quantity; the unit is sourced from §12.3.2.2 and
    /// Annex C.2 (see `pdfce_core::annot::apply_view_usage`).
    ///
    /// A viewer must re-render when this changes — §8.11.4.5 requires the
    /// dictionaries to be reapplied "whenever there is a change to a
    /// factor that [they] depend on (such as zoom level)". In the GUI
    /// that falls out of the raster cache keying on scale.
    pub view_magnification: Option<f32>,
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
    /// Where a page's blending colour space comes from when its group
    /// declares none — spec ambiguity `PGB-A1`. See
    /// [`pdfce_core::settings::PageBlendSpaceSource`], whose docs carry the
    /// clause citations and the reason this is a setting rather than a fix.
    pub page_blend_space_source: pdfce_core::settings::PageBlendSpaceSource,
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
// `Eq` is deliberately NOT derived: `view_magnification` is an `f32`,
// and a policy carrying a float has no total equality. `PartialEq` is
// what the projection test actually needs, and claiming `Eq` for a type
// that can hold a NaN would be a lie the compiler happens to allow via
// the other fields.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[non_exhaustive]
pub struct RenderPolicy<'a> {
    /// See [`RenderOptions::cmyk_intent`].
    pub cmyk_intent: CmykIntent,
    /// See [`RenderOptions::page_blend_space_source`].
    pub page_blend_space_source: pdfce_core::settings::PageBlendSpaceSource,
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
    /// The operator's layer-visibility override for this render, or
    /// `None` to obey the document's default configuration
    /// (§8.11.4.3 `/OCProperties /D`).
    ///
    /// A BORROW, so `RenderPolicy` stays `Copy` while the set itself is
    /// owned by the [`RenderOptions`] that outlives the render. See
    /// [`crate::LayerVisibility`] for why it REPLACES the document's
    /// defaults rather than merging with them.
    pub layers: Option<&'a crate::LayerVisibility>,
    /// See [`RenderOptions::view_magnification`].
    pub view_magnification: Option<f32>,
    /// See [`RenderOptions::subpixel_culling`]. The only LOSSY entry in
    /// this bundle, which is why its counter is reported separately.
    pub subpixel_culling: bool,
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
            // ISO 32000-2 Annex P's route, taken only when the output
            // intent is subtractive -- see `PageBlendSpaceSource`, whose
            // docs carry why this is a choice and not a right answer.
            page_blend_space_source: pdfce_core::settings::PageBlendSpaceSource::default(),
            // OFF. The one lossy knob in this struct, and decision 082
            // puts that choice with the operator rather than with the
            // default.
            subpixel_culling: false,
            // Every class painted — the pre-existing "annotations on"
            // behaviour, spelled as a scope. See `AnnotationScope`'s type
            // docs for why the default is Acrobat Pro's rather than
            // Reader's: it is a compatibility decision about what
            // `render_page` has always drawn, not a choice of which product
            // to imitate.
            annotation_scope: AnnotationScope::default(),
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
            // See the field docs: None means "render the document as the
            // document asks", which is the only correct default for a
            // caller that has no operator behind it.
            layers: None,
            // See the field docs: the print-correct answer is the safe
            // default, and a viewer opts in.
            view_magnification: None,
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

    /// Set which classes of annotation are painted (the four-way Acrobat
    /// print scope), returning `self` for chaining.
    ///
    /// Same `#[non_exhaustive]` consuming-builder reasoning as
    /// [`Self::with_annotations`]. This does **not** touch
    /// [`Self::annotations`], so
    /// `RenderOptions::default().with_annotations(false).with_annotation_scope(s)`
    /// still paints nothing — the `bool` is a master gate that only
    /// subtracts. Callers that want the scope to be the only control
    /// should leave `annotations` at its `true` default and set this alone.
    #[must_use]
    pub fn with_annotation_scope(mut self, scope: AnnotationScope) -> Self {
        self.annotation_scope = scope;
        self
    }

    /// The annotation scope this render will actually use — the
    /// composition of [`Self::annotations`] and [`Self::annotation_scope`].
    ///
    /// **The one place the two fields are combined**, and the only value
    /// the render path reads. Having exactly one composition point is the
    /// point: two knobs that describe the same thing are a standing
    /// invitation for a caller's setting to be honoured on one code path
    /// and dropped on another, which is the failure mode
    /// [`RenderPolicy`]'s own docs were written to prevent one level down.
    ///
    /// The rule is a single sentence: **the `bool` can only subtract.**
    /// `annotations = false` ⇒ [`AnnotationScope::ContentOnly`], full stop;
    /// otherwise the scope field stands.
    ///
    /// ```
    /// use pdfce_render::{AnnotationScope, RenderOptions};
    ///
    /// // The default: every annotation class, exactly as before.
    /// let options = RenderOptions::default();
    /// assert_eq!(
    ///     options.effective_annotation_scope(),
    ///     AnnotationScope::DocumentAndMarkups
    /// );
    ///
    /// // Acrobat's "Document": page content plus form fields, no markups.
    /// let options = RenderOptions::default()
    ///     .with_annotation_scope(AnnotationScope::Document);
    /// assert_eq!(options.effective_annotation_scope(), AnnotationScope::Document);
    ///
    /// // The master gate wins over any scope.
    /// let options = options.with_annotations(false);
    /// assert_eq!(
    ///     options.effective_annotation_scope(),
    ///     AnnotationScope::ContentOnly
    /// );
    /// ```
    #[must_use]
    pub const fn effective_annotation_scope(&self) -> AnnotationScope {
        if self.annotations {
            self.annotation_scope
        } else {
            AnnotationScope::ContentOnly
        }
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

    /// Set where a page's blending colour space comes from when its group
    /// declares none (§11.4.7; spec ambiguity `PGB-A1`), returning `self`
    /// for chaining.
    ///
    /// Same `#[non_exhaustive]` reasoning as [`Self::with_annotations`].
    /// This is the seam the operator's persisted setting arrives through:
    /// `RenderOptions::default().with_page_blend_space_source(settings.page_blend_space_source)`.
    ///
    /// # What it changes
    ///
    /// Whether a PDF/X file that declares no page group composites in ink
    /// or on screen — and therefore whether **overprint can be represented
    /// at all**, which in an additive space it cannot be (see
    /// [`pdfce_core::settings::PageBlendSpaceSource`]). On the Ghent PDF
    /// Output Suite this moves 24 of 51 patches between the two paths.
    ///
    /// The choice is disclosed rather than silent: the resulting space's
    /// provenance is reported as `blend_space_from` on
    /// `pdfce-cli render-page`'s metrics line.
    #[must_use]
    pub fn with_page_blend_space_source(
        mut self,
        source: pdfce_core::settings::PageBlendSpaceSource,
    ) -> Self {
        self.page_blend_space_source = source;
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
    /// Override which optional-content groups are hidden for this render
    /// (§8.11), returning `self` for chaining.
    ///
    /// The set REPLACES the document's default configuration. Build it
    /// from [`pdfce_core::annot::optional_content_default_off`] and apply
    /// the operator's toggles — passing only the groups the operator
    /// touched would show every layer the document had turned off. See
    /// [`crate::LayerVisibility`].
    pub fn with_layers(mut self, layers: crate::LayerVisibility) -> Self {
        self.layers = Some(layers);
        self
    }

    /// Render as a VIEWER at this magnification, applying `View`-event
    /// `/AS` usage applications (§8.11.4.4). See
    /// [`RenderOptions::view_magnification`] — do not call this on a
    /// print or aggregate path.
    #[must_use]
    pub fn with_view_magnification(mut self, magnification: f32) -> Self {
        self.view_magnification = Some(magnification);
        self
    }

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
    pub const fn policy(&self) -> RenderPolicy<'_> {
        RenderPolicy {
            cmyk_intent: self.cmyk_intent,
            page_blend_space_source: self.page_blend_space_source,
            mask_resample: self.mask_resample,
            image_minify: self.image_minify,
            cmyk_jpeg_polarity: self.cmyk_jpeg_polarity,
            missing_as: self.missing_as,
            layers: self.layers.as_ref(),
            view_magnification: self.view_magnification,
            subpixel_culling: self.subpixel_culling,
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
        // `layers` is the one field whose value is BORROWED from the
        // options, so it is set here too: a projection that dropped it
        // would render every layer the document turns off, which is
        // exactly the operator choice this gate exists to protect.
        let hidden = crate::LayerVisibility::hiding([pdfce_core::object::ObjId::new(10, 0)]);
        let options = RenderOptions::default()
            .with_cmyk_intent(pdfce_core::settings::CmykIntent::Naive)
            .with_page_blend_space_source(pdfce_core::settings::PageBlendSpaceSource::DeviceNative)
            .with_mask_resample(pdfce_core::settings::MaskResample::Bilinear)
            .with_image_minify(pdfce_core::settings::MinifyFilter::Smooth)
            .with_cmyk_jpeg_polarity(pdfce_core::settings::CmykJpegPolarity::InvertOnApp14)
            .with_missing_as(pdfce_core::settings::MissingAppearanceState::FirstEntry)
            .with_layers(hidden.clone())
            .with_view_magnification(2.5);
        assert_eq!(
            options.policy(),
            RenderPolicy {
                cmyk_intent: pdfce_core::settings::CmykIntent::Naive,
                // NOT the default. This test proves a builder call reaches
                // the policy, and a field left at its default would pass
                // whether or not `with_page_blend_space_source` did anything.
                page_blend_space_source: pdfce_core::settings::PageBlendSpaceSource::DeviceNative,
                mask_resample: pdfce_core::settings::MaskResample::Bilinear,
                image_minify: pdfce_core::settings::MinifyFilter::Smooth,
                cmyk_jpeg_polarity: pdfce_core::settings::CmykJpegPolarity::InvertOnApp14,
                missing_as: pdfce_core::settings::MissingAppearanceState::FirstEntry,
                layers: Some(&hidden),
                view_magnification: Some(2.5),
                // Not set by any `with_*` above, so it must still be the
                // default. That is the assertion, not the boilerplate:
                // this test exists to catch a knob that reaches
                // `RenderOptions` and never reaches `RenderPolicy`.
                subpixel_culling: false,
            }
        );
        assert_ne!(options.policy(), RenderPolicy::default());
    }
}
