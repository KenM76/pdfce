//! Rendering presets for the PDF subset standards — PDF/X, PDF/A, PDF/UA.
//!
//! # What a preset is, and the one thing it is not
//!
//! A preset is a **named bundle of values for settings that already exist**,
//! applied in one act and individually editable afterwards. It adds no
//! rendering mode, decides no conformance verdict, and validates nothing.
//! Asking pdfce to render "as PDF/X-4" does not make a file PDF/X-4 and does
//! not check whether it is one.
//!
//! It is not, and must never become, a claim that the output is conformant.
//! What it claims is narrower and checkable: *for this standard, these are the
//! values, and here is where each one comes from.*
//!
//! # ★★ WHY EVERY ENTRY CARRIES ITS OWN EVIDENCE TIER
//!
//! `pdfce-gui` asked for this and refused to guess at it, in terms worth
//! repeating because they are the whole design constraint:
//!
//! > *"The mechanism is ours and is straightforward… **The values are not
//! > ours.** They are a claim about a standard, and a control labelled
//! > `ISO 15930-7` carries that standard's authority whether or not we
//! > intended it to. A guessed vector would be worse than no preset."*
//!
//! That is exactly right, and it is why [`Evidence`] exists. A control that
//! says *ISO 15930-7* while setting four values nobody sourced is borrowing
//! an ISO committee's credibility for a developer's opinion. Every entry here
//! therefore states whether it is **sourced to a clause**, **implied** by one,
//! a **best-effort** engineering judgement, or **not applicable** — and
//! [`RenderPreset::disclosures`] says so out loud, so a shell can put the
//! honest sentence next to the honest button.
//!
//! # ★★★ THE THREE FINDINGS THAT SHAPED THIS MODULE
//!
//! All three came from sourcing the standards rather than from designing the
//! API, and each one changed the design.
//!
//! **1. Only PDF/X claims to bind a renderer at all.** PDF/A and PDF/UA
//! explicitly decline: both carry a Scope bullet saying the standard *"does
//! not apply to … operational details of rendering"* (ISO 19005-3 §5.5,
//! ISO 19005-4 §5.2, ISO 14289-1 §6.3). So a PDF/A preset is thin by right,
//! and a PDF/UA preset is **empty by right** — see
//! [`RenderStandard::PdfUa1`], where "no preset" is the *sourced answer*
//! rather than an omission somebody will later mistake for one and fill in.
//!
//! **2. PDF/X itself concedes that more than one conforming rendering
//! exists**, and says what the remedy is — and it is not a setting vector.
//! ISO 15930-4:2003 §5 and ISO 15930-9:2020 §5, word for word identical
//! seventeen years apart: *"To the extent that … this document permit more
//! than one rendering of a conforming file, a conforming processor may use
//! embedded job ticket or metadata information to control the rendering more
//! precisely."* A preset is a reasonable second-best; the standard's own
//! answer is out-of-band data pdfce does not read. That sentence belongs
//! beside the control, and [`RenderPreset::disclosures`] puts it there.
//!
//! **3. `cmyk_intent` is the WRONG MECHANISM for these standards, not a
//! mis-set value.** Every PDF/X and PDF/A level guarantees a *colorimetric*
//! definition of device colour — `DestOutputProfile`, `/DefaultCMYK`, or
//! PDF/A-4's blending space. [`super::CmykIntent`] selects among **fixed
//! built-in tables** and is none of those. No value of it is conformant, so
//! the preset sets the least-wrong one and **discloses that the file's own
//! output intent was not applied**. Rule 4 makes that obligatory rather than
//! optional: the operator cannot see a colour transform that did not happen.
//!
//! # Why `NotApplicable` is a state and not simply "leave it at the default"
//!
//! Roughly a third of the cells in the sourced grid are axes a given standard
//! does not reach. `MeshPatchPadding` for any PDF/X level is the clearest:
//! the complete clause lists of ISO 15930-7 and -9 contain **no shading
//! clause at all**.
//!
//! A preset modelled as a fully-populated `Settings` value cannot express
//! that. It would write a value for every key, and a shell reading it back
//! would report *"ISO 15930-7 requires per-record mesh padding"* — a
//! requirement that does not exist, asserted under the standard's name. So
//! [`PresetAction::LeaveAlone`] is a real state: the preset touches nothing,
//! and says which keys it deliberately did not touch.
//!
//! # Scope, stated so it is not over-read
//!
//! ★ These presets govern **how pdfce renders a file**. They do not govern
//! how pdfce *writes* one, and applying one to a document does not move it
//! toward conformance in any respect. `to-pdfa` and `validate-pdfa` remain
//! unimplemented (`ROADMAP.md`), and nothing here is a step toward them.

use super::{
    CmykIntent, MaskResample, MeshPatchPadding, MinifyFilter, PageBlendSpaceSource, Settings,
};
use crate::pageops::separation::SeparationPolicy;

/// A PDF subset standard a render preset can be built for.
///
/// # Which parts are here, and which deliberately are not
///
/// One variant per **rendering-distinct** conformance family, not one per
/// published part. `PDF/X-4` and `PDF/X-4p` differ only in whether the output
/// intent's profile is embedded or referenced externally — a *file* property
/// with no rendering consequence pdfce can act on — so they share a variant
/// and the doc says so.
///
/// **PDF/X-5n and PDF/X-6n are absent, and that is sourced rather than
/// lazy.** Both are built on an *n-colorant* process model, and pdfce's
/// compositor carries **four planes, not a runtime N**
/// (`docs/NEXT_SESSION.md`'s standing not-done list). A preset for them would
/// name a standard pdfce cannot render the defining feature of. That is a
/// capability boundary, and the honest form of it is absence plus this
/// paragraph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RenderStandard {
    /// **PDF/X-1a** — ISO 15930-1:2001 and ISO 15930-4:2003.
    ///
    /// CMYK, greyscale and spot only; no device-independent colour. **Live
    /// transparency is forbidden**, which is what makes most of this grid
    /// not-applicable rather than differently-valued.
    PdfX1a,
    /// **PDF/X-3** — ISO 15930-3:2002 and ISO 15930-6:2003.
    ///
    /// Shares most of X-1a's requirements but permits device-independent
    /// colour, and permits a device space only where the output intent's
    /// profile is that same space. Transparency still forbidden.
    ///
    /// # ★ THIS PRODUCES AN IDENTICAL VECTOR TO [`Self::PdfX1a`], ON PURPOSE
    ///
    /// Reported by `pdfce-gui` 2026-08-25: applying X-3 leaves settings its
    /// matcher then reports as X-1a. That is **correct, and not a bug to
    /// fix.**
    ///
    /// The two parts differ in **what colour spaces a FILE may contain** —
    /// X-1a restricts print elements to CMYK, greyscale and spot; X-3 permits
    /// device-independent colour against a matching output intent. **No
    /// render-radius setting can see that difference.** It is a constraint on
    /// the document, checked by a validator, not a knob a renderer turns.
    ///
    /// So the honest preset for both is the same preset, and the variants
    /// stay separate because the *labels* differ and an operator selecting
    /// "PDF/X-3" should see PDF/X-3 named back to them. **Do not merge them,
    /// and do not manufacture a difference to justify keeping them apart.**
    /// A preset that invented a divergence here would be asserting a
    /// rendering requirement neither standard contains — the exact failure
    /// [`PresetAction::LeaveAlone`] exists to prevent, one level up.
    PdfX3,
    /// **PDF/X-4 and PDF/X-4p** — ISO 15930-7:2010.
    ///
    /// **The first part to permit live transparency**, which is why it is the
    /// only PDF/X level where the blending-space axis is live at all — and
    /// why it is the one `pdfce-gui` asked about.
    PdfX4,
    /// **PDF/X-5g and PDF/X-5pg** — ISO 15930-8:2010.
    ///
    /// X-4 plus externally-referenced graphics. The reference mechanism is a
    /// file-structure feature; on these six axes it renders as X-4 does.
    PdfX5g,
    /// **PDF/X-6 and PDF/X-6p** — ISO 15930-9:2020, the PDF 2.0-based part.
    ///
    /// ★ Note one real difference from X-4 that must NOT be attached to
    /// [`CmykIntent`]: X-4 §6.23 restricted `/RI` to the four ICC rendering
    /// intents and **X-6 dropped that restriction**. That is the ICC
    /// rendering intent, which pdfce does not carry at all.
    PdfX6,
    /// **PDF/A-1** — ISO 19005-1:2005, conformance levels `a` and `b`.
    ///
    /// **Transparency is forbidden**, so the transparency axes are
    /// not-applicable exactly as for PDF/X-1a.
    PdfA1,
    /// **PDF/A-2 and PDF/A-3** — ISO 19005-2:2011, ISO 19005-3:2012.
    ///
    /// Transparency permitted. Carries the strongest blending-space sentence
    /// in the whole corpus — see [`RenderPreset::disclosures`].
    PdfA2,
    /// **PDF/A-4** — ISO 19005-4:2020, including the `e` and `f` flavours.
    PdfA4,
    /// **PDF/UA-1 and PDF/UA-2** — ISO 14289-1, ISO 14289-2.
    ///
    /// ★★ **This variant exists to return an EMPTY preset, and that is the
    /// sourced answer rather than an unfinished one.**
    ///
    /// PDF/UA is a structure and accessibility standard. ISO 14289-1 §6.3
    /// places operational rendering details outside its scope, and
    /// **PDF/UA-2 deleted UA-1's conforming-reader and assistive-technology
    /// clauses outright**. Measured rather than asserted: nine rendering
    /// terms (`transparen*`, `blend`, `Interpolate`, `OutputIntent`,
    /// `SMask`, `colour`, `color`, `image`, `Group`) across all **197**
    /// veraPDF PDF/UA rules return **zero hits**. Colour contrast is handed
    /// explicitly to WCAG 2.2, which governs presentation and not a PDF
    /// renderer's sampling choices.
    ///
    /// Absence would have been indistinguishable from an oversight, and the
    /// next session would have filled it in. A variant that answers "nothing,
    /// and here is the measurement" cannot be mistaken for a gap.
    PdfUa1,
}

/// Where a preset entry's value comes from.
///
/// Ordered from strongest to weakest, and [`Evidence::BestEffort`] is not an
/// apology — it is the honest label for a value chosen by engineering
/// judgement where the standard is silent, which for these six axes is most
/// of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Evidence {
    /// A clause of the standard says this, and the clause has been read.
    Sourced,
    /// A clause constrains the file in a way that determines this value
    /// without stating it, or a NOTE in a related standard states it
    /// informatively.
    ///
    /// The PDF/X-4 blending space is the type case: its only ISO support is
    /// **ISO 32000-2:2020 §11.4.7 NOTE 3**, which is *informative*, and in a
    /// *different standard*. ISO 15930-7 §6.20 has not been read against it.
    /// That is a genuinely weaker footing than [`Self::Sourced`] and is
    /// labelled as such rather than rounded up.
    Implied,
    /// The standard is silent and pdfce chose. A defensible engineering
    /// judgement, and **not** a claim about the standard.
    BestEffort,
    /// The standard does not reach this axis at all.
    ///
    /// Carried as evidence rather than as an absence so that
    /// [`RenderPreset::disclosures`] can say *which* axes were deliberately
    /// left alone. "The preset did not set this" and "the preset forgot this"
    /// look identical from outside unless one of them says so.
    NotApplicable,
}

impl Evidence {
    /// A short word for a shell to put beside a value.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Sourced => "sourced",
            Self::Implied => "implied",
            Self::BestEffort => "best-effort",
            Self::NotApplicable => "not applicable",
        }
    }

    /// Whether this entry is a claim about the standard at all.
    ///
    /// `false` for [`Self::BestEffort`] — which is the point of separating
    /// the tiers. A shell that wants to show *"what does ISO 15930-7 actually
    /// require?"* filters on this and gets a much shorter, much more honest
    /// list than the preset as a whole.
    #[must_use]
    pub fn is_a_claim_about_the_standard(self) -> bool {
        matches!(self, Self::Sourced | Self::Implied | Self::NotApplicable)
    }
}

/// Which setting an entry governs.
///
/// A key rather than a closure over `Settings` so that a preset can be
/// inspected, printed and compared without being applied — a shell has to be
/// able to show the operator what a preset WOULD do before it does it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PresetKey {
    /// [`Settings::page_blend_space_source`].
    PageBlendSpaceSource,
    /// [`Settings::mesh_patch_padding`].
    MeshPatchPadding,
    /// [`Settings::mask_resample`].
    MaskResample,
    /// [`Settings::image_minify`].
    ImageMinify,
    /// [`Settings::cmyk_intent`].
    CmykIntent,
    /// [`Settings::separations`].
    Separations,
}

impl PresetKey {
    /// The key's name as it appears in a settings file.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PageBlendSpaceSource => "page_blend_space_source",
            Self::MeshPatchPadding => "mesh_patch_padding",
            Self::MaskResample => "mask_resample",
            Self::ImageMinify => "image_minify",
            Self::CmykIntent => "cmyk_intent",
            Self::Separations => "separations",
        }
    }
}

/// What a preset does to one setting.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum PresetAction {
    /// Set [`Settings::page_blend_space_source`].
    BlendSpace(PageBlendSpaceSource),
    /// Set [`Settings::mesh_patch_padding`].
    MeshPadding(MeshPatchPadding),
    /// Set [`Settings::mask_resample`].
    Mask(MaskResample),
    /// Set [`Settings::image_minify`].
    Minify(MinifyFilter),
    /// Set [`Settings::cmyk_intent`].
    Cmyk(CmykIntent),
    /// Set [`Settings::separations`].
    Separations(SeparationPolicy),
    /// **Change nothing**, and say so.
    ///
    /// The state that makes a preset honest. See the module docs: writing a
    /// value here would assert a requirement the standard does not contain,
    /// under that standard's name.
    LeaveAlone,
}

impl PresetAction {
    /// The value as a shell should print it, or `"-"` for
    /// [`PresetAction::LeaveAlone`].
    ///
    /// # Why this lives here and not in the shell that prints it
    ///
    /// [`PresetAction`] is `#[non_exhaustive]`, so a downstream crate cannot
    /// match it exhaustively and must carry a wildcard arm. That wildcard is
    /// a trap: add a seventh variant and every shell keeps compiling while
    /// printing the new setting as whatever the fallback says — silently, and
    /// under a button labelled with an ISO number.
    ///
    /// Formatting it here inverts that. The `match` below has no wildcard, so
    /// a new variant is a **compile error in this file**, which is where
    /// somebody who just added one is already looking.
    ///
    /// ★ `"-"` rather than an empty string or a repeated default for the
    /// leave-alone case. A blank column reads as missing data and a repeated
    /// default reads as *"this is the value"* — which is the one thing a
    /// not-applicable cell must never say.
    #[must_use]
    pub fn value_string(self) -> String {
        match self {
            Self::BlendSpace(v) => format!("{v:?}"),
            Self::MeshPadding(v) => format!("{v:?}"),
            Self::Mask(v) => format!("{v:?}"),
            Self::Minify(v) => format!("{v:?}"),
            Self::Cmyk(v) => format!("{v:?}"),
            Self::Separations(v) => format!("{v:?}"),
            Self::LeaveAlone => "-".to_owned(),
        }
    }

    /// Whether this action changes a setting at all.
    #[must_use]
    pub fn sets_a_value(self) -> bool {
        !matches!(self, Self::LeaveAlone)
    }
}

/// One setting, the value a standard implies for it, and where that came from.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct PresetEntry {
    /// Which setting.
    pub key: PresetKey,
    /// What to do with it.
    pub action: PresetAction,
    /// How well founded that is.
    pub evidence: Evidence,
    /// Why — one sentence, written for the operator, not for a maintainer.
    pub why: &'static str,
}

/// A named bundle of render settings for one subset standard.
///
/// Build with [`RenderPreset::for_standard`], inspect with
/// [`RenderPreset::entries`], apply with [`RenderPreset::apply`], and **say
/// what it did** with [`RenderPreset::disclosures`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct RenderPreset {
    /// The standard this preset is for.
    pub standard: RenderStandard,
    entries: Vec<PresetEntry>,
}

impl RenderPreset {
    /// The preset for a standard.
    ///
    /// Always succeeds. A standard with no rendering requirements returns a
    /// preset whose every entry is [`PresetAction::LeaveAlone`] — applying it
    /// correctly does nothing, and [`Self::disclosures`] explains why. See
    /// [`RenderStandard::PdfUa1`] for why that is better than returning
    /// `None`.
    #[must_use]
    pub fn for_standard(standard: RenderStandard) -> Self {
        Self {
            standard,
            entries: entries_for(standard),
        }
    }

    /// Every entry, in a stable order.
    #[must_use]
    pub fn entries(&self) -> &[PresetEntry] {
        &self.entries
    }

    /// Apply the preset to `settings`, returning the keys it actually changed.
    ///
    /// [`PresetAction::LeaveAlone`] entries are skipped, and a key whose
    /// value already matched is **not** reported as changed — a shell showing
    /// *"this preset changed 4 settings"* must not count the ones that were
    /// already right.
    pub fn apply(&self, settings: &mut Settings) -> Vec<PresetKey> {
        let mut changed = Vec::new();
        for e in &self.entries {
            let moved = match e.action {
                PresetAction::BlendSpace(v) => {
                    let was = settings.page_blend_space_source;
                    settings.page_blend_space_source = v;
                    was != v
                }
                PresetAction::MeshPadding(v) => {
                    let was = settings.mesh_patch_padding;
                    settings.mesh_patch_padding = v;
                    was != v
                }
                PresetAction::Mask(v) => {
                    let was = settings.mask_resample;
                    settings.mask_resample = v;
                    was != v
                }
                PresetAction::Minify(v) => {
                    let was = settings.image_minify;
                    settings.image_minify = v;
                    was != v
                }
                PresetAction::Cmyk(v) => {
                    let was = settings.cmyk_intent;
                    settings.cmyk_intent = v;
                    was != v
                }
                PresetAction::Separations(v) => {
                    let was = settings.separations;
                    settings.separations = v;
                    was != v
                }
                PresetAction::LeaveAlone => false,
            };
            if moved {
                changed.push(e.key);
            }
        }
        changed
    }

    /// The keys this preset deliberately did **not** set.
    ///
    /// Separate from "keys it set to their current value", and the
    /// distinction is the whole point: one means *the standard does not reach
    /// this axis*, the other means *it does and you already agree with it*.
    #[must_use]
    pub fn left_alone(&self) -> Vec<PresetKey> {
        self.entries
            .iter()
            .filter(|e| matches!(e.action, PresetAction::LeaveAlone))
            .map(|e| e.key)
            .collect()
    }

    /// Everything an operator is owed about this preset, as plain sentences.
    ///
    /// # Why this is not optional
    ///
    /// Project rule 4. A preset is an **inference** — pdfce's reading of what
    /// a standard implies — and most of it is invisible by construction: a
    /// colour transform that did not happen leaves nothing on screen to look
    /// at. In `pdfce-cli` the invocation is the commit, so these are printed
    /// on the way past; in a GUI they belong off-canvas, beside the control,
    /// never drawn into the page.
    #[must_use]
    pub fn disclosures(&self) -> Vec<String> {
        let mut out = vec![format!(
            "render preset: {} — {} setting(s) stated, {} deliberately left alone. \
             This changes how pdfce RENDERS the file. It does not make the file \
             conformant and does not check whether it is.",
            self.standard.title(),
            self.entries
                .iter()
                .filter(|e| !matches!(e.action, PresetAction::LeaveAlone))
                .count(),
            self.left_alone().len(),
        )];

        let claims = self
            .entries
            .iter()
            .filter(|e| {
                !matches!(e.action, PresetAction::LeaveAlone)
                    && e.evidence.is_a_claim_about_the_standard()
            })
            .count();
        let guesses = self
            .entries
            .iter()
            .filter(|e| e.evidence == Evidence::BestEffort)
            .count();
        if guesses > 0 {
            out.push(format!(
                "render preset: {claims} value(s) are sourced to or implied by the standard; \
                 {guesses} are pdfce's own engineering judgement where the standard is SILENT, \
                 and are not a claim about it"
            ));
        }

        // ★ The standard's own admission, quoted because it is the most
        // useful sentence in the whole exercise and a preset that hid it
        // would be overselling itself.
        if self.standard.is_pdf_x() {
            out.push(
                "render preset: ISO 15930 itself allows that more than one rendering of a \
                 conforming file may be permitted, and its stated remedy is embedded job-ticket \
                 or metadata information — which pdfce does not read. A preset is a reasonable \
                 second best, not the standard's own answer"
                    .to_owned(),
            );
        }

        if self.standard.declines_to_specify_rendering() {
            out.push(format!(
                "render preset: {} places the operational details of rendering OUTSIDE its \
                 scope, so most of this grid is not-applicable by the standard's own words \
                 rather than by pdfce declining to look",
                self.standard.title()
            ));
        }

        if self.standard.output_intent_is_colorimetric() {
            out.push(
                "render preset: this standard guarantees a COLORIMETRIC definition of device \
                 colour (an output intent's destination profile). pdfce does not apply it — \
                 `cmyk_intent` selects among fixed built-in tables and is not an ICC path — so \
                 CMYK on this page is converted by a table, not by the file's own profile. \
                 This is a capability gap, not a mis-set value"
                    .to_owned(),
            );
        }

        for e in &self.entries {
            if matches!(e.action, PresetAction::LeaveAlone) {
                out.push(format!(
                    "render preset: `{}` left alone ({}) — {}",
                    e.key.as_str(),
                    e.evidence.label(),
                    e.why
                ));
            }
        }
        out
    }
}

impl RenderStandard {
    /// Every standard a preset exists for, in publication order.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[
            Self::PdfX1a,
            Self::PdfX3,
            Self::PdfX4,
            Self::PdfX5g,
            Self::PdfX6,
            Self::PdfA1,
            Self::PdfA2,
            Self::PdfA4,
            Self::PdfUa1,
        ]
    }

    /// The name an operator would recognise, with its ISO number.
    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            Self::PdfX1a => "PDF/X-1a (ISO 15930-1, -4)",
            Self::PdfX3 => "PDF/X-3 (ISO 15930-3, -6)",
            Self::PdfX4 => "PDF/X-4 and X-4p (ISO 15930-7)",
            Self::PdfX5g => "PDF/X-5g and X-5pg (ISO 15930-8)",
            Self::PdfX6 => "PDF/X-6 and X-6p (ISO 15930-9)",
            Self::PdfA1 => "PDF/A-1 (ISO 19005-1)",
            Self::PdfA2 => "PDF/A-2 and PDF/A-3 (ISO 19005-2, -3)",
            Self::PdfA4 => "PDF/A-4 (ISO 19005-4)",
            Self::PdfUa1 => "PDF/UA-1 and PDF/UA-2 (ISO 14289-1, -2)",
        }
    }

    /// The token a CLI flag or settings file uses.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PdfX1a => "pdf-x1a",
            Self::PdfX3 => "pdf-x3",
            Self::PdfX4 => "pdf-x4",
            Self::PdfX5g => "pdf-x5g",
            Self::PdfX6 => "pdf-x6",
            Self::PdfA1 => "pdf-a1",
            Self::PdfA2 => "pdf-a2",
            Self::PdfA4 => "pdf-a4",
            Self::PdfUa1 => "pdf-ua",
        }
    }

    /// Parse a CLI token.
    ///
    /// # Errors
    ///
    /// Returns the unrecognised token so a caller can name it.
    pub fn parse(token: &str) -> Result<Self, &str> {
        Self::all()
            .iter()
            .copied()
            .find(|s| s.as_str() == token)
            .ok_or(token)
    }

    /// Whether this is a PDF/X part.
    #[must_use]
    pub fn is_pdf_x(self) -> bool {
        matches!(
            self,
            Self::PdfX1a | Self::PdfX3 | Self::PdfX4 | Self::PdfX5g | Self::PdfX6
        )
    }

    /// Whether the standard puts rendering outside its own scope.
    ///
    /// True for PDF/A and PDF/UA, both of which say so in their Scope
    /// clauses; false for PDF/X, which is the only family that claims to bind
    /// a conforming processor's rendering.
    #[must_use]
    pub fn declines_to_specify_rendering(self) -> bool {
        !self.is_pdf_x()
    }

    /// Whether the standard guarantees a colorimetric device-colour
    /// definition that pdfce does not apply.
    ///
    /// True for every PDF/X and PDF/A level: each requires an output intent
    /// (or, for PDF/A-2 and later without one, a declared blending space).
    /// pdfce reads none of them for colour conversion, so this drives a
    /// disclosure rather than a value.
    #[must_use]
    pub fn output_intent_is_colorimetric(self) -> bool {
        !matches!(self, Self::PdfUa1)
    }

    /// Whether the standard forbids live transparency.
    ///
    /// The single fact that makes most of this grid not-applicable for the
    /// early parts: no transparency means no page group, no blending space
    /// question and no soft masks to resample.
    #[must_use]
    pub fn forbids_transparency(self) -> bool {
        matches!(self, Self::PdfX1a | Self::PdfX3 | Self::PdfA1)
    }
}

/// The grid.
///
/// One function rather than a table literal so each cell can carry its own
/// sentence, and so the transparency-forbidding parts can be built from the
/// fact that produces them instead of having it copied into three places.
fn entries_for(standard: RenderStandard) -> Vec<PresetEntry> {
    use Evidence::{BestEffort, Implied, NotApplicable, Sourced};
    use PresetAction::{Cmyk, LeaveAlone, Mask, Minify};
    use PresetKey as K;

    // PDF/UA reaches none of these axes, and that is measured.
    if standard == RenderStandard::PdfUa1 {
        const WHY: &str = "PDF/UA is a structure and accessibility standard; ISO 14289-1 §6.3 \
                           puts rendering outside its scope, PDF/UA-2 deleted the \
                           conforming-reader clauses entirely, and nine rendering terms across \
                           all 197 veraPDF PDF/UA rules return zero hits";
        return [
            K::PageBlendSpaceSource,
            K::MeshPatchPadding,
            K::MaskResample,
            K::ImageMinify,
            K::CmykIntent,
            K::Separations,
        ]
        .into_iter()
        .map(|key| PresetEntry {
            key,
            action: LeaveAlone,
            evidence: NotApplicable,
            why: WHY,
        })
        .collect();
    }

    let mut out = Vec::with_capacity(6);

    // --- 1. the blending colour space ---------------------------------------
    out.push(if standard.forbids_transparency() {
        PresetEntry {
            key: K::PageBlendSpaceSource,
            action: LeaveAlone,
            evidence: Sourced,
            why: "this standard forbids live transparency, so a page group's blending space \
                  never arises in a conforming file",
        }
    } else if standard.is_pdf_x() {
        PresetEntry {
            key: K::PageBlendSpaceSource,
            action: PresetAction::BlendSpace(PageBlendSpaceSource::OutputIntentIfSubtractive),
            evidence: Implied,
            // Worded to the letter of what is actually held: the support is
            // informative, and in another standard. Saying "ISO 15930-7
            // requires" here would be the exact overreach this module exists
            // to prevent.
            why: "ISO 32000-2:2020 §11.4.7 NOTE 3 identifies a PDF/X-4 file's output-intent \
                  destination profile as the implied default page blending colour space; that \
                  NOTE is informative, and ISO 15930-7 §6.20 has not been read against it",
        }
    } else {
        PresetEntry {
            key: K::PageBlendSpaceSource,
            action: PresetAction::BlendSpace(PageBlendSpaceSource::OutputIntentIfSubtractive),
            evidence: Sourced,
            why: "PDF/A-2 §6.2.10 and PDF/A-4 §6.2.9 require that a transparent page with no \
                  PDF/A output intent declare a group colour space whose value SHALL be used as \
                  the default blending space — so in a conforming file the space is never \
                  undetermined",
        }
    });

    // --- 2. mesh patch padding ----------------------------------------------
    out.push(if standard.is_pdf_x() {
        PresetEntry {
            key: K::MeshPatchPadding,
            action: LeaveAlone,
            evidence: Sourced,
            why: "the complete clause lists of ISO 15930-7 and ISO 15930-9 contain no shading \
                  clause at all, so no PDF/X part reaches this",
        }
    } else {
        PresetEntry {
            key: K::MeshPatchPadding,
            action: PresetAction::MeshPadding(MeshPatchPadding::PerRecord),
            evidence: BestEffort,
            why: "ISO 32000-1 §8.7.4.5.5 is permanently ambiguous here and ISO 32000-2 repeats \
                  it unchanged; no subset standard resolves it, so pdfce's own default stands",
        }
    });

    // --- 3. mask resampling --------------------------------------------------
    out.push(if standard.forbids_transparency() {
        PresetEntry {
            key: K::MaskResample,
            action: LeaveAlone,
            evidence: Sourced,
            why: "soft masks are a transparency feature and this standard forbids them; an \
                  explicit /Mask is unaffected and keeps pdfce's default",
        }
    } else {
        PresetEntry {
            key: K::MaskResample,
            action: Mask(MaskResample::Nearest),
            evidence: BestEffort,
            why: "no subset standard specifies mask resampling; nearest-neighbour preserves a \
                  stencil's hard 0/255 boundary, which is what a mask defines",
        }
    });

    // --- 4. image minification ----------------------------------------------
    //
    // ★ THE ONE AXIS WHERE A PRESET ACTUALLY DIVERGES FROM THE SHIPPED
    // DEFAULT, as of 2026-08-25. `MinifyFilter::default()` moved to `Smooth`
    // on the operator's Acrobat comparison — the right answer for looking at
    // a page. For conformance output the spec-literal reading is the safer
    // one: it resamples nothing it was not told to.
    out.push(PresetEntry {
        key: K::ImageMinify,
        action: Minify(MinifyFilter::PointSample),
        evidence: BestEffort,
        why: "ISO 32000-1 §8.9.5.3 legislates interpolation for MAGNIFICATION only and every \
              subset standard's /Interpolate rule is a file rule about that direction; \
              point-sampling is the reading that invents no samples, which is the conservative \
              choice for conformance output and NOT what pdfce defaults to for viewing",
    });

    // --- 5. CMYK intent ------------------------------------------------------
    out.push(PresetEntry {
        key: K::CmykIntent,
        action: Cmyk(CmykIntent::NeutralBlack),
        evidence: BestEffort,
        why: "this standard guarantees a colorimetric device-colour definition and pdfce applies \
              none of them — `cmyk_intent` picks among fixed built-in tables. No value is \
              conformant; this one keeps pure K neutral, which is what an ink-destined file \
              most often needs",
    });

    // --- 6. separations ------------------------------------------------------
    //
    // ★ `separations` is `SeparationPolicy` — the §14.11.4 preseparated-page-
    // set policy — and NOT spot-colorant handling. The distinction matters:
    // the brief that commissioned this grid had it wrong, and the corrected
    // question turned out to have a sharper answer.
    out.push(if standard.is_pdf_x() {
        PresetEntry {
            key: K::Separations,
            action: LeaveAlone,
            evidence: if matches!(standard, RenderStandard::PdfX1a | RenderStandard::PdfX3) {
                Sourced
            } else {
                Implied
            },
            why: "ISO 15930-1 §6.2, -3 §6.1 and -4 §6.1 each state that a pre-separated file \
                  shall not be permitted, so /SeparationInfo cannot appear in a conforming \
                  PDF/X file and the policy is inert",
        }
    } else {
        PresetEntry {
            key: K::Separations,
            action: PresetAction::Separations(SeparationPolicy::default()),
            evidence: BestEffort,
            why: "PDF/A does not forbid a pre-separated page set, so the PDF/X reasoning does \
                  not transfer and pdfce's own default policy stands",
        }
    });

    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// Every standard produces a preset, and every preset covers every axis.
    ///
    /// A missing entry and a `LeaveAlone` entry are completely different
    /// claims — "we did not think about this" versus "the standard does not
    /// reach this" — and only the second is defensible under a button
    /// labelled with an ISO number. Covering all six keys for all standards
    /// makes the first one unrepresentable.
    #[test]
    fn every_standard_covers_every_axis() {
        let keys = [
            PresetKey::PageBlendSpaceSource,
            PresetKey::MeshPatchPadding,
            PresetKey::MaskResample,
            PresetKey::ImageMinify,
            PresetKey::CmykIntent,
            PresetKey::Separations,
        ];
        for &std in RenderStandard::all() {
            let preset = RenderPreset::for_standard(std);
            assert_eq!(
                preset.entries().len(),
                keys.len(),
                "{} does not cover every axis",
                std.title()
            );
            for k in keys {
                assert!(
                    preset.entries().iter().any(|e| e.key == k),
                    "{} says nothing about {}",
                    std.title(),
                    k.as_str()
                );
            }
        }
    }

    /// ★★ THE TEST THAT GUARDS AGAINST OVERCLAIMING, which is the one risk
    /// this whole module exists to manage.
    ///
    /// An entry may only be labelled `Sourced` or `Implied` — i.e. may only
    /// present itself as a claim about the standard — if it either sets
    /// nothing, or is one of the specific cells the spec sourcing actually
    /// established. Everything else must be `BestEffort`.
    ///
    /// Without this, the cheapest way to make a preset look authoritative is
    /// to relabel a guess, and nothing in the type system would object. The
    /// allow-list below is deliberately short and deliberately awkward to
    /// extend: adding a cell to it should require going and reading a clause.
    #[test]
    fn only_sourced_cells_may_claim_to_be_sourced() {
        for &std in RenderStandard::all() {
            for e in RenderPreset::for_standard(std).entries() {
                if !e.evidence.is_a_claim_about_the_standard() {
                    continue;
                }
                let sets_something = !matches!(e.action, PresetAction::LeaveAlone);
                if !sets_something {
                    // A not-applicable or forbidden-feature cell sets nothing;
                    // claiming the standard is why it set nothing is exactly
                    // what those tiers are for.
                    continue;
                }
                // The only cells permitted to SET a value under a
                // standard-claiming tier.
                let permitted = matches!(
                    (std, e.key),
                    (
                        RenderStandard::PdfX4
                            | RenderStandard::PdfX5g
                            | RenderStandard::PdfX6
                            | RenderStandard::PdfA2
                            | RenderStandard::PdfA4,
                        PresetKey::PageBlendSpaceSource
                    )
                );
                assert!(
                    permitted,
                    "{} sets {} at tier `{}` — a value presented as a claim about the \
                     standard must be one the sourcing actually established, or it borrows \
                     an ISO committee's authority for an opinion",
                    std.title(),
                    e.key.as_str(),
                    e.evidence.label()
                );
            }
        }
    }

    /// PDF/UA changes nothing at all, and that is the answer rather than a
    /// stub.
    #[test]
    fn the_accessibility_standard_has_no_rendering_preset() {
        let preset = RenderPreset::for_standard(RenderStandard::PdfUa1);
        let mut settings = Settings::default();
        let before = settings.clone();

        let changed = preset.apply(&mut settings);
        assert!(
            changed.is_empty(),
            "PDF/UA moved a render setting: {changed:?}"
        );
        assert_eq!(settings, before, "PDF/UA changed the settings");
        assert_eq!(preset.left_alone().len(), 6, "all six axes are left alone");

        // And it SAYS so — an empty preset that stayed silent would be
        // indistinguishable from a broken one.
        let said = preset.disclosures().join(" ");
        assert!(
            said.contains("outside its scope"),
            "the reason must be stated, not merely implied by inaction: {said}"
        );
    }

    /// A standard that forbids transparency leaves the transparency axes
    /// alone rather than setting them.
    ///
    /// Setting a blending space for PDF/X-1a would be, in `pdfce-gui`'s own
    /// phrase, noise dressed as authority: the file cannot contain a page
    /// group, so a value for one asserts a requirement that does not exist.
    #[test]
    fn a_standard_that_forbids_transparency_sets_no_transparency_axis() {
        for std in [
            RenderStandard::PdfX1a,
            RenderStandard::PdfX3,
            RenderStandard::PdfA1,
        ] {
            let preset = RenderPreset::for_standard(std);
            for key in [PresetKey::PageBlendSpaceSource, PresetKey::MaskResample] {
                let e = preset
                    .entries()
                    .iter()
                    .find(|e| e.key == key)
                    .expect("axis covered");
                assert!(
                    matches!(e.action, PresetAction::LeaveAlone),
                    "{} set {} despite forbidding transparency",
                    std.title(),
                    key.as_str()
                );
            }
        }
    }

    /// No PDF/X level sets mesh padding, and every one of them says why.
    #[test]
    fn no_pdf_x_level_reaches_the_mesh_padding_axis() {
        for &std in RenderStandard::all().iter().filter(|s| s.is_pdf_x()) {
            let e = RenderPreset::for_standard(std)
                .entries()
                .iter()
                .copied()
                .find(|e| e.key == PresetKey::MeshPatchPadding)
                .expect("axis covered");
            assert!(matches!(e.action, PresetAction::LeaveAlone));
            assert_eq!(e.evidence, Evidence::Sourced);
            assert!(e.why.contains("no shading clause"));
        }
    }

    /// ★ The preset actually diverges from the shipped defaults somewhere.
    ///
    /// The whole exercise would be theatre if every preset were a no-op on a
    /// fresh `Settings` — the operator would click a button labelled
    /// `ISO 15930-7` and get exactly what they already had. As of
    /// 2026-08-25 the divergence is `image_minify`: the viewing default moved
    /// to `Smooth`, and conformance output keeps the spec-literal
    /// `PointSample`.
    ///
    /// If a future change makes this pass vacuously, that is a signal the
    /// presets have stopped earning their place, not a test to delete.
    #[test]
    fn a_preset_changes_something_on_a_fresh_settings() {
        let mut settings = Settings::default();
        let changed = RenderPreset::for_standard(RenderStandard::PdfX4).apply(&mut settings);
        assert!(
            changed.contains(&PresetKey::ImageMinify),
            "PDF/X-4 no longer diverges from the shipped defaults anywhere: {changed:?}"
        );
        assert_eq!(settings.image_minify, MinifyFilter::PointSample);
    }

    /// Applying a preset twice reports no second change.
    ///
    /// `apply` returns what it MOVED, not what it wrote, so a shell can say
    /// "4 settings changed" truthfully. A second application must report an
    /// empty list or that sentence is a lie the second time it is shown.
    #[test]
    fn applying_a_preset_twice_reports_no_second_change() {
        let preset = RenderPreset::for_standard(RenderStandard::PdfX4);
        let mut settings = Settings::default();
        let first = preset.apply(&mut settings);
        assert!(!first.is_empty());
        let second = preset.apply(&mut settings);
        assert!(
            second.is_empty(),
            "reported a change it did not make: {second:?}"
        );
    }

    /// The colorimetric gap is disclosed wherever it exists.
    ///
    /// Every PDF/X and PDF/A level guarantees a colorimetric device-colour
    /// definition that pdfce does not apply. That is invisible by
    /// construction — a colour transform that did not happen leaves nothing
    /// on screen — so rule 4 makes saying it obligatory.
    #[test]
    fn the_unapplied_output_intent_is_disclosed() {
        for &std in RenderStandard::all() {
            let said = RenderPreset::for_standard(std).disclosures().join(" ");
            // Case-insensitive: the sentence emphasises COLORIMETRIC in
            // capitals, and a test that pinned the capitalisation would fail
            // the next time somebody reworded the emphasis rather than the
            // claim. What must hold is that the gap is NAMED.
            let said_lower = said.to_lowercase();
            if std.output_intent_is_colorimetric() {
                assert!(
                    said_lower.contains("colorimetric") && said_lower.contains("capability gap"),
                    "{} hides the unapplied output intent: {said}",
                    std.title()
                );
            }
        }
    }

    /// Every PDF/X preset repeats the standard's own admission that more than
    /// one conforming rendering may exist.
    #[test]
    fn a_pdf_x_preset_does_not_oversell_itself() {
        for &std in RenderStandard::all() {
            let said = RenderPreset::for_standard(std).disclosures().join(" ");
            assert_eq!(
                std.is_pdf_x(),
                said.contains("more than one rendering"),
                "{} states the multiple-rendering concession incorrectly",
                std.title()
            );
            assert!(
                said.contains("does not make the file conformant"),
                "{} could be read as a conformance claim: {said}",
                std.title()
            );
        }
    }

    /// Tokens round-trip, and an unknown one is returned rather than guessed.
    #[test]
    fn every_standard_token_round_trips() {
        for &std in RenderStandard::all() {
            assert_eq!(RenderStandard::parse(std.as_str()), Ok(std));
        }
        assert_eq!(RenderStandard::parse("pdf-x9"), Err("pdf-x9"));
    }
}
