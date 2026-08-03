//! # In-place text FORMATTING surgery (Pass 14.2, extended by Pass 19.1)
//!
//! This module changes the **size**, **fill colour**, **font-family/style**,
//! **character spacing**, **horizontal scaling** and **baseline position**
//! of text already on a page — in place, minimal-diff — by reusing Pass
//! 14.1's advance-preserving content-stream surgery
//! (`crate::text_edit::edit`) with different operators changed. It is the
//! third slice of decision 014's Acrobat-text-editing family
//! (`docs/decisions/014-acrobat-text-editing.md` §3 "Formatting", §5.2's
//! "13.2"; R32/R34/R47/R69/R70/R72) plus decision 019's slice 19.1
//! (R88/R89).
//!
//! ## The six operations, and what each rewrites
//!
//! | Op | Operator changed | Advance affected? | Font gate? |
//! |---|---|---|---|
//! | **size** | `Tf` size operand | yes (ΔA) | no — never needs new glyphs |
//! | **fill colour** | `rg`/`g`/`k` (the chosen space) | no | no |
//! | **font family/style** | `Tf` font resource | yes (ΔA) | YES — re-encode + coverage gate |
//! | **character spacing** (19.1) | `Tc` (§9.3.2) | yes — `Tc` is a term of §9.4.4 | no |
//! | **horizontal scaling** (19.1) | `Tz` (§9.3.4) | yes — `Th` multiplies the whole displacement | no |
//! | **super/subscript** (19.1) | `Ts` (§9.3.7) + `Tf` size | yes, via the size reduction only | no |
//!
//! ## Pass 19.1: the parity controls, and what is deliberately absent
//!
//! Decision 019 §3.1 established from a named Adobe source that current
//! Acrobat retains exactly **character spacing, horizontal scaling and a
//! coarse superscript/subscript toggle** in this family. Those three are
//! what this slice ships. Two neighbours are deliberately NOT here:
//!
//! - **`Tw` (word spacing)** is engine-only — read, tracked, restored,
//!   preserved byte-verbatim, and never emitted by an authoring control. It
//!   is structurally void for 2-byte composite codes (§9.3.3: word spacing
//!   "shall NOT apply to occurrences of the byte value 32 in multiple-byte
//!   codes"), and its honest use case — inter-word slack — already belongs
//!   to the reflow layer, which chose `TJ` over `Tw` for exactly the
//!   reasons that make `Tw` a poor dial (decision 015 §3.1). Whether a
//!   control is ever built is gated on a corpus census (decision 019 §3.3).
//! - **Free-form numeric `Ts`** is slice 19.2's deliberate exceed. This
//!   slice emits `Ts` only at the two **fixed, derived** values of the
//!   super/subscript toggle, so the emission and restore machinery is
//!   proven before an operator can type an arbitrary rise into it.
//!
//! ### Superscript/subscript is TWO operators, not one
//!
//! A script toggle emits `Ts` (the baseline shift) **and** a reduced `Tf`
//! size, because a superscript that is not also smaller is just raised
//! text. Both are derived as ratios of the run's **base** size — see
//! [`SUPERSCRIPT`] for the ratios, their provenance, and why they are
//! pdfce's own documented choice rather than a parity claim.
//!
//! ### R89 — why spacing is stored as a ratio and derived at emit time
//!
//! `Tc` and `Ts` are in *unscaled text space units* and are explicitly
//! **not** scaled by the font size (§9.3, Table 105's closing note). A
//! superscript applied at 10 pt and then resized to 20 pt would therefore
//! keep its absolute rise and land at the wrong height, and tracking dialled
//! in at one size would look like a different amount of tracking at another.
//! [`MetricSpec`] is the discriminated store-the-ratio/derive-the-operand
//! model that closes this, and [`MetricSpec::resolve`] is the one place a
//! ratio becomes a number.
//!
//! ### R88 — scoping by restore-by-value, and the four rungs
//!
//! Everything in this family is **graphics state that persists**: §9.3 says
//! text-state values "are retained across text objects in a single content
//! stream". An emitted `0.5 Tc` therefore bleeds into every following show
//! operator unless something puts the previous value back — and the obvious
//! `q … Q` wrap is **illegal inside a text object** (§8.2 Table 51 /
//! Figure 9). So each emitted operator is paired with an explicit restore
//! emitted inside the same text object, resolved through the shared ladder
//! in [`crate::text_state`]:
//!
//! 1. never set in this stream → the Table 105 **spec default**;
//! 2. set by its own operator → that operator's **raw bytes**, so a
//!    `0.5000 Tc` is not renormalized into a diff;
//! 3. set as a **side effect** of `TD` or `"` → the value **re-spelled**,
//!    because replaying a `"` would show its string a second time
//!    (§9.4.3 Table 109) — disclosed as a narrowing;
//! 4. inherited from outside the buffer → **refuse and disclose**, never a
//!    guessed default.
//!
//! [`push_state_param`] is the single application point for all four.
//!
//! ### The `Tz` × justify interaction
//!
//! Changing a run's width inside a justified line invalidates that line's
//! slack. pdfce detects it, discloses it, and offers re-justification rather
//! than silently leaving the line mis-aligned — see
//! [`disclosure_justify_invalidated`], which also records a place where
//! decision 019's stated mechanism does not match this module's actual
//! emission shape.
//!
//! All of these are the SAME surgery as 14.1 REPLACE — locate the anchor show
//! operator, recompute the §9.4.4 advance, relayout the single line, save
//! **incrementally** (R34) — but instead of changing the shown *codes* they
//! change the *text state* around the run. That is done by **state-wrap**:
//! the anchor operator's element list is split at the matched code range
//! into `pre | mid | post` segments, and re-emitted as
//!
//! ```text
//! [pre in original state]  <state-set ops>  [mid]  <state-restore ops>  [post in original state]
//! ```
//!
//! State-set/restore operators (`Tf`, `rg`/`g`/`k`) do **not** move the
//! text matrix, so `pre`/`post` and every following operator keep their
//! exact positions — except for the §9.4.4 advance delta a size or font
//! change introduces on `mid` (a bigger size, or a different face's
//! metrics, advances differently). That ΔA is handled EXACTLY as 14.1
//! handles a REPLACE's ΔA: under [`FollowerDisposition::Reflow`] (default)
//! the line shifts by ΔA — `post` and advance-relative followers move for
//! free, an absolute-`Tm` follower gets ΔA added to its `e`, and the line
//! MAY overflow the original margin (disclosed; block re-wrap is FF-A);
//! under [`FollowerDisposition::Pin`] a trailing compensating `TJ` consumes
//! ΔA so nothing after the run moves. Restoring the prior state after `mid`
//! is what keeps the edit LOCAL — only this run changes; every subsequent
//! operator sees byte-for-byte the state it saw before (R32/R46).
//!
//! ## (1) Size → `Tf` operand (pdfce's own choice; Acrobat unconfirmed)
//!
//! A size change touches ONLY the `Tf` size operand — never the fill-colour
//! operator or any other text-state (a binding minimal-diff rule: a
//! size-only edit emits a `Tf … Tf` wrap and nothing else). Acrobat's own
//! documentation confirms size is one of exactly two properties changeable
//! on an embedded-but-not-local font (colour is the other,
//! `Acrobat_Features/text_edit__formatting_options.md`), so size **never
//! needs new glyph outlines** and always works on existing glyphs. Acrobat
//! has no confirmed baseline for arbitrary-vs-preset point values, for
//! same-line reflow-vs-redraw, or for a selection spanning multiple
//! original sizes, so pdfce's behaviour here is its OWN documented choice,
//! NOT a parity claim: it accepts arbitrary point values; applies the
//! single-line advance-preserving relayout above; and a selection that
//! spanned multiple original sizes flattens to the one new size (disclosed).
//!
//! ## (2) Fill colour → `rg`/`g`/`k` (parity-PLUS: the space is preserved)
//!
//! The operator chooses the colour MODEL — RGB, CMYK, or grayscale — and
//! pdfce STORES THAT ACTUAL SPACE (`rg`/`k`/`g`); it does **not** convert
//! everything to DeviceRGB the way Acrobat's Edit-Text tool does (Acrobat
//! always emits `rg` even when CMYK sliders were used —
//! `Acrobat_Features/text_edit__formatting_options.md`). This is a genuine
//! minimal-diff differentiator. Colour does not affect advances, so no
//! relayout. If the run's ORIGINAL fill colour was recorded as
//! [`FillState::Other`](crate::text_edit::edit) — a non-device space
//! (ICCBased/Separation/DeviceN/Indexed) pdfce does not decode — a
//! colour-touching edit that cannot preserve that space's semantics is
//! DISCLOSED as a space-narrowing conversion, never a silent downgrade
//! (rule 4). The run's tail (`post`) and everything after it are restored
//! to the original colour byte-verbatim (the recorded operator bytes),
//! even for an `Other` space pdfce cannot itself interpret.
//!
//! ## (3) Font family/style → `Tf` resource (the load-bearing one)
//!
//! Changing the selected run's typeface means re-associating it with a
//! DIFFERENT font RESOURCE and re-encoding the SAME characters into that
//! face's encoding (reusing 14.1's inverse-encoding builder against the
//! target font's `/Encoding`). It is gated on **coverage**: the target
//! must supply a code for EVERY character in the run. Two refusal gates,
//! both reusing 14.1's machinery verbatim:
//!
//! - the target is composite / symbolic-no-encoding / `/ToUnicode`-only
//!   ([`classify_font`](crate::text_edit::edit)) → refused by name;
//! - a character the target's resolved `/Encoding` cannot show, or (for an
//!   embedded-SUBSET target) a code it does not already carry on the page →
//!   **REFUSE-and-disclose by name**, with NOTHING applied — never
//!   `.notdef`, never a silent substitution, never partial.
//!
//! A successful family change NEVER triggers font embedding: the coverage
//! gate excludes anything needing glyph data pdfce cannot resolve, so
//! subsetting stays FF-C. **Scope boundary (binding):** the target must be
//! a REAL, already-existing font RESOURCE on the page (swap `Times-Roman`
//! for an existing `Times-Bold`); this cut adds no new resource-dict entry
//! and embeds nothing, so only the content stream changes. Algorithmic
//! faux-bold/faux-italic synthesis is FF-H — not built here. An
//! outlined/vectorized-text target has no font resource to swap and is
//! refused with 14.1's existing "font resource is unresolvable" reason, not
//! a new error class.
//!
//! ## Tagged PDFs (§14.7, T-disclose, R72) — the anti-Acrobat property
//!
//! Because the surgery rewrites only the anchor operator's bytes (and, under
//! reflow, a following `Tm`), any enclosing `BDC …/MCID n… EMC` wrapper is
//! preserved **by construction** — the structure tree's `(Pg, MCID)`
//! reference stays valid. A formatting change (colour/weight) is a
//! CONFIRMED trigger of Acrobat's tag-tree-corruption defect
//! (`Acrobat_Features/text_edit__formatting_options.md` §"Selection-scope"):
//! pdfce instead DISCLOSES that the structure tree's `/ActualText` and
//! reading order were not updated, and does not corrupt them.
//!
//! ## Save mode (R34/R70) — incremental, not a scrub
//!
//! Formatting is not redaction: it uses the default incremental save. Only
//! the edited content stream object (+ any collapsed extra content objects
//! on a multi-stream page) is re-emitted; everything else is the original
//! bytes verbatim. Prior text/colour survives in the document history by
//! design, and this is disclosed. No resource/font dict is re-emitted in
//! this cut, because the family-change target is an existing resource.

use std::collections::BTreeSet;

use crate::content::{ContentError, ContentStream};
use crate::document::Document;
use crate::object::{Dict, Object};
use crate::page_tree::{self, PageTreeError};
use crate::span::ByteSpan;
use crate::text_edit::EditGlyphSource;
use crate::text_edit::edit::{
    EditError, EditRequest, FollowerDisposition, FontClass, MatchRun, OpRec, Rec, ShowData,
    ShowElem, ShowOp, Walk, carried_codes, classify_font, compensating_tj, emit_show, emit_tm,
    find_anchor, glyph_advance_with, is_subset_tag, match_run, resolve_font_dict, splice,
    trust_disclosure, write_incremental,
};
use crate::text_edit::encoding::{InverseEncoding, RInvTrigger, Refusal};
use crate::text_extract::font::ExtractFont;
use crate::text_state::{AmbientRestoreError, AmbientTextState, TextStateParam};
use crate::writer::content::emit_number;

/// How close two text-state operands must be before pdfce treats a
/// requested value as "already in force" and emits nothing.
///
/// This is a **minimal-diff** guard, not a numeric tolerance
/// (`ARCHITECTURE.md` §5): asking for `100 Tz` on a run already at the
/// Table 105 default must not add a `100 Tz … 100 Tz` pair to a stream that
/// did not need one. Both sides of the comparison are operands parsed from,
/// or destined for, the same decimal writer, so a tight epsilon is all that
/// is wanted — this is not a geometric tolerance.
const STATE_EPS: f64 = 1e-9;

// ===================================================================
// Pass 19.1 — the direct text-state formatting controls
// (decision 019 §3.1/§3.2; standing rules R88/R89)
// ===================================================================

/// How a size-relative typographic quantity was expressed by the operator
/// — the discriminated unit model standing rule **R89** requires.
///
/// ## Why this type exists at all
///
/// `Tc` (character spacing) and `Ts` (text rise) are in **unscaled text
/// space units** and are explicitly **not** scaled by the font size (ISO
/// 32000-1 §9.3, Table 105's closing note: they "shall be specified in a
/// coordinate system that shall be defined by the text matrix `Tm` but
/// **shall not be scaled by the font size parameter `Tfs`**").
///
/// That produces a trap with teeth: a tracking of `0.24` text-space units
/// reads as 2% of the em at 12 pt and as 0.5% of the em at 48 pt. Storing
/// only the *operand* means a later resize silently mis-scales every
/// spacing the operator set. Storing the operator's **intent** — "20‰ of
/// the em" — and re-deriving the operand at emit time is what keeps a
/// resize correct. That is R89, and this enum is it.
///
/// `Tz` deliberately does **not** take a `MetricSpec`: it is a
/// dimensionless percentage of normal glyph width (§9.3.4), so it has
/// nothing to be relative *to*.
///
/// # Examples
///
/// ```
/// use pdfce_core::text_edit::MetricSpec;
///
/// // 20/1000 em of tracking, resolved against two different font sizes.
/// let tracking = MetricSpec::Relative(20.0);
/// assert!((tracking.resolve(12.0) - 0.24).abs() < 1e-12);
/// assert!((tracking.resolve(48.0) - 0.96).abs() < 1e-12);
///
/// // An absolute value is the operand itself, at every size.
/// let fixed = MetricSpec::Absolute(0.5);
/// assert_eq!(fixed.resolve(12.0), 0.5);
/// assert_eq!(fixed.resolve(48.0), 0.5);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum MetricSpec {
    /// The value **is** the operand, in unscaled text-space units — exactly
    /// what gets written into the content stream, at every font size.
    Absolute(f64),
    /// The value is in **thousandths of an em** (‰ of the font size) — the
    /// typographic "tracking" unit, and the same unit space `TJ`'s own
    /// numeric adjustments live in (§9.4.3: a `TJ` number is expressed "in
    /// thousandths of a unit of text space"). Resolved against the run's
    /// effective font size at emit time.
    Relative(f64),
}

impl MetricSpec {
    /// Resolve to the operand that will be written, given the font size the
    /// run is shown at.
    ///
    /// This is the whole of R89 in one function: the *only* place a
    /// relative quantity becomes a number, so a resize that re-runs the
    /// planner re-derives rather than reusing a stale operand.
    #[must_use]
    pub fn resolve(self, font_size: f64) -> f64 {
        match self {
            Self::Absolute(v) => v,
            // ‰ of the em: 20 ‰ at 12 pt is 0.24 unscaled text-space units.
            Self::Relative(per_mille) => per_mille * font_size / 1000.0,
        }
    }

    /// The unit this spec is expressed in, for a disclosure string.
    #[must_use]
    pub const fn unit_label(self) -> &'static str {
        match self {
            Self::Absolute(_) => "unscaled text-space units",
            Self::Relative(_) => "thousandths of an em",
        }
    }

    /// The number the operator supplied, in its own unit.
    #[must_use]
    pub const fn raw(self) -> f64 {
        match self {
            Self::Absolute(v) | Self::Relative(v) => v,
        }
    }
}

/// The coarse superscript / subscript toggle — Acrobat's actual retained
/// baseline control, and the only baseline surface this slice exposes.
///
/// Free-form numeric `Ts` is deliberately **not** here: decision 019 §3.2
/// ships it as slice 19.2's "deliberate exceed". This slice emits `Ts` only
/// at the two **fixed, derived** values below, so the emission, restore,
/// tracking and round-trip machinery is proven before an operator can type
/// an arbitrary number into it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScriptPosition {
    /// Baseline — `Ts` 0 and no size reduction. Requesting this on a run
    /// that inherits a non-zero ambient rise is how an operator flattens it
    /// back without a free-form control.
    Normal,
    /// Raised and reduced — [`SUPERSCRIPT`].
    Superscript,
    /// Lowered and reduced — [`SUBSCRIPT`].
    Subscript,
}

impl ScriptPosition {
    /// The metrics this position applies. [`Self::Normal`] returns `None`
    /// and means "rise 0, size unchanged".
    #[must_use]
    pub const fn metrics(self) -> Option<ScriptMetrics> {
        match self {
            Self::Normal => None,
            Self::Superscript => Some(SUPERSCRIPT),
            Self::Subscript => Some(SUBSCRIPT),
        }
    }

    /// A short operator-facing name, for disclosures and CLI output.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Superscript => "superscript",
            Self::Subscript => "subscript",
        }
    }
}

/// The two ratios that define one script position, both expressed as
/// fractions of the run's **base** font size so R89's re-derivation is
/// automatic.
///
/// A public value type rather than two hidden constants precisely because
/// rule 4 (fuzzy, never sneaky) forbids a hidden magic number: the CLI and
/// the save report quote these by value, so an operator can always see what
/// pdfce applied.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct ScriptMetrics {
    /// The emitted `Tf` size as a fraction of the base size.
    pub size_factor: f64,
    /// The emitted `Ts` rise as a fraction of the **base** size — positive
    /// raises the baseline (§9.3.7).
    ///
    /// Deliberately a fraction of the *base* size and not of the reduced
    /// size, which keeps the two ratios independent: changing
    /// [`Self::size_factor`] does not move the baseline, and a superscript
    /// aligns against the size of the text it is superscript *to*.
    pub rise_ratio: f64,
}

/// pdfce's superscript metrics — **pdfce's own documented choice, NOT a
/// parity claim.**
///
/// ## Provenance of these two numbers (read before changing them)
///
/// Adobe's actual internal ratios are **unknown and unsourced**. The
/// Acrobat feature-parity catalog records the toggle itself as confirmed
/// and the numbers as an explicit **GAP**
/// (`Acrobat_Features/text_edit__spacing_and_scaling_controls.md`: "the
/// exact size-ratio and baseline-offset VALUES these toggles apply
/// internally are not documented by any source found"), together with an
/// explicit licence to choose: "pdfce is free to pick its own defensible
/// typographic defaults (commonly ~58–65% size / ~33% rise in general
/// typographic practice) **without this being a parity claim about
/// Acrobat's own numbers**."
///
/// So the values below are chosen, not measured, and are justified by:
///
/// 1. **Decision 019 §3.2 declared them** (size factor 0.60, superscript
///    rise +0.34, subscript rise −0.18) as pdfce's defaults. That decision
///    is this slice's specification; re-picking numbers mid-build would
///    make the decision record a fiction.
/// 2. **They sit inside the general-practice band the catalog recorded**
///    (0.60 ∈ 0.58–0.65; 0.34 ≈ 0.33), so they are ordinary rather than
///    idiosyncratic — the result will not look unusual in another viewer.
/// 3. **The subscript drop is deliberately shallower than the superscript
///    rise** (0.18 vs 0.34). Not a rounding accident: a raised glyph has
///    the ascender band and the interline gap to move into, while a lowered
///    one runs into its own line's descenders almost immediately.
///    Symmetric ratios collide with descenders.
///
/// The mechanism itself has no fuzz in it and is fully spec-governed: `Ts`
/// sets the distance "to move the baseline up or down from its default
/// location", in unscaled text space units, positive up (§9.3.7), entering
/// `Trm` as a translation — so it changes position and **not** advance.
pub const SUPERSCRIPT: ScriptMetrics = ScriptMetrics {
    size_factor: 0.60,
    rise_ratio: 0.34,
};

/// pdfce's subscript metrics — same provenance and same caveats as
/// [`SUPERSCRIPT`]; see that constant's documentation for why the drop is
/// shallower than the rise.
pub const SUBSCRIPT: ScriptMetrics = ScriptMetrics {
    size_factor: 0.60,
    rise_ratio: -0.18,
};

/// The colour MODEL an operator dials a new fill colour in — and the exact
/// device space pdfce STORES (parity-plus: no forced DeviceRGB conversion).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FillModel {
    /// DeviceGray — one component, stored as the `g` operator (§8.6.4.2).
    Gray,
    /// DeviceRGB — three components, stored as the `rg` operator (§8.6.4.3).
    Rgb,
    /// DeviceCMYK — four components, stored as the `k` operator (§8.6.4.4).
    Cmyk,
}

impl FillModel {
    /// The PDF operator this model stores (`g` / `rg` / `k`) — the exact
    /// device space pdfce preserves.
    #[must_use]
    pub const fn operator(self) -> &'static str {
        match self {
            Self::Gray => "g",
            Self::Rgb => "rg",
            Self::Cmyk => "k",
        }
    }

    /// The number of colour components this model requires (1 / 3 / 4).
    #[must_use]
    pub const fn arity(self) -> usize {
        match self {
            Self::Gray => 1,
            Self::Rgb => 3,
            Self::Cmyk => 4,
        }
    }

    /// The human-readable device space name, for disclosures.
    #[must_use]
    pub const fn space_name(self) -> &'static str {
        match self {
            Self::Gray => "DeviceGray",
            Self::Rgb => "DeviceRGB",
            Self::Cmyk => "DeviceCMYK",
        }
    }
}

/// A new fill colour: the model plus its components (each `0.0..=1.0`,
/// §8.6.4). The component count must match [`FillModel::arity`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct NewFill {
    /// Which device space to store the colour in.
    pub model: FillModel,
    /// The colour components, in operator order (`gray`; `r g b`;
    /// `c m y k`).
    pub components: Vec<f64>,
}

impl NewFill {
    /// A new fill colour, validating the component count against the model.
    ///
    /// # Errors
    ///
    /// [`FormatError::BadColor`] when `components.len()` is not
    /// [`FillModel::arity`], or when any component is outside `0.0..=1.0`.
    pub fn new(model: FillModel, components: Vec<f64>) -> Result<Self, FormatError> {
        if components.len() != model.arity() {
            return Err(FormatError::BadColor(format!(
                "{} needs {} colour component(s), got {}",
                model.space_name(),
                model.arity(),
                components.len()
            )));
        }
        if let Some(bad) = components
            .iter()
            .copied()
            .find(|c| !(0.0..=1.0).contains(c))
        {
            return Err(FormatError::BadColor(format!(
                "colour component {bad} is outside 0.0..=1.0 (§8.6.4)"
            )));
        }
        Ok(Self { model, components })
    }
}

/// How the operator names the family-change target font.
///
/// The target must be a REAL, already-existing font RESOURCE on the page
/// (scope boundary). It is located by either its `/Resources /Font`
/// resource key (`F2`) or its `/BaseFont` (`Times-Bold`, matched exactly or
/// with the §9.6.4 subset tag stripped).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FontSelector {
    /// The resource-key or base-font string the operator supplied.
    pub selector: String,
}

impl FontSelector {
    /// A selector from an operator-supplied string.
    #[must_use]
    pub fn new(selector: &str) -> Self {
        Self {
            selector: selector.to_owned(),
        }
    }
}

/// One in-place text-FORMAT request against a page.
///
/// The anchor operator is located exactly as a [`EditRequest`] locates it:
/// by [`Self::find`] within one show operator's decoded text, or (with
/// [`Self::pinned_span`]) the operator whose byte span matches a
/// [`GlyphProvenance::operator_span`](crate::text_extract::GlyphProvenance).
/// At least one of [`Self::set_size`] / [`Self::set_fill`] / [`Self::set_font`]
/// must be present, else the request is a no-op ([`FormatError::NoOp`]).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct FormatRequest {
    /// 0-based page index.
    pub page_index: usize,
    /// The text to locate within one show operator's decoded run.
    pub find: String,
    /// When set, only consider the show operator whose decoded-buffer byte
    /// span equals this (the provenance-pinned path).
    pub pinned_span: Option<ByteSpan>,
    /// New `Tf` font size (points). `None` leaves the size unchanged.
    pub set_size: Option<f64>,
    /// New fill colour (device space preserved). `None` leaves it unchanged.
    pub set_fill: Option<NewFill>,
    /// New font family/style — an existing page font resource. `None`
    /// leaves the face unchanged.
    pub set_font: Option<FontSelector>,
    /// New character spacing `Tc` (§9.3.2), in the operator's own unit
    /// (R89). `None` leaves the run at its ambient `Tc`.
    pub set_char_spacing: Option<MetricSpec>,
    /// New horizontal scaling `Tz` as a **percentage** of normal width
    /// (§9.3.4; 100 = normal). `None` leaves the run at its ambient `Tz`.
    pub set_h_scale: Option<f64>,
    /// New baseline position — the coarse superscript/subscript toggle.
    /// `None` leaves both the run's rise and its size alone.
    pub set_script: Option<ScriptPosition>,
}

impl FormatRequest {
    /// A format request located by find text (no span pin, no ops yet).
    /// Chain [`Self::size`] / [`Self::fill`] / [`Self::font`] to add
    /// operations.
    #[must_use]
    pub fn new(page_index: usize, find: &str) -> Self {
        Self {
            page_index,
            find: find.to_owned(),
            pinned_span: None,
            set_size: None,
            set_fill: None,
            set_font: None,
            set_char_spacing: None,
            set_h_scale: None,
            set_script: None,
        }
    }

    /// Add a size change (points), returning `self`.
    #[must_use]
    pub fn size(mut self, points: f64) -> Self {
        self.set_size = Some(points);
        self
    }

    /// Add a fill-colour change, returning `self`.
    #[must_use]
    pub fn fill(mut self, fill: NewFill) -> Self {
        self.set_fill = Some(fill);
        self
    }

    /// Add a font-family/style change, returning `self`.
    #[must_use]
    pub fn font(mut self, selector: FontSelector) -> Self {
        self.set_font = Some(selector);
        self
    }

    /// Add a character-spacing (`Tc`) change, returning `self`.
    #[must_use]
    pub fn char_spacing(mut self, spec: MetricSpec) -> Self {
        self.set_char_spacing = Some(spec);
        self
    }

    /// Add a horizontal-scaling (`Tz`) change in **percent** (100 =
    /// normal), returning `self`.
    #[must_use]
    pub fn h_scale(mut self, percent: f64) -> Self {
        self.set_h_scale = Some(percent);
        self
    }

    /// Add a superscript/subscript/normal baseline change, returning
    /// `self`.
    #[must_use]
    pub fn script(mut self, position: ScriptPosition) -> Self {
        self.set_script = Some(position);
        self
    }

    /// Whether any formatting operation was requested.
    ///
    /// `pub(crate)` rather than private because **two** entry points must
    /// make this check — the free [`set_format`] and
    /// [`EditSession::format_text`](crate::edit::EditSession::format_text) —
    /// and until Pass 19.1 the session had its own hand-listed copy of the
    /// condition. That copy did exactly what a duplicated predicate always
    /// does: it went stale the moment a field was added, so every request
    /// that used only the new spacing controls was rejected as a no-op on
    /// the session path (the path the GUI drives) while working perfectly on
    /// the free-function path. One predicate, two callers.
    #[must_use]
    pub(crate) const fn is_empty(&self) -> bool {
        self.set_size.is_none()
            && self.set_fill.is_none()
            && self.set_font.is_none()
            && self.set_char_spacing.is_none()
            && self.set_h_scale.is_none()
            && self.set_script.is_none()
    }
}

/// Per-format options.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct FormatOptions {
    /// How the rest of the line is disposed after a size/font change's ΔA
    /// (default [`FollowerDisposition::Reflow`]). A colour-only change never
    /// shifts anything, so this is moot for it.
    pub disposition: FollowerDisposition,
}

impl FormatOptions {
    /// Set the follower [`FollowerDisposition`], returning `self`.
    #[must_use]
    pub fn with_disposition(mut self, disposition: FollowerDisposition) -> Self {
        self.disposition = disposition;
        self
    }
}

/// The outcome of a successful format edit.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct FormatOutcome {
    /// The saved (incrementally-appended) PDF bytes.
    pub bytes: Vec<u8>,
    /// The disclosure/diagnostic report.
    pub report: FormatReport,
}

/// What the format edit did and disclosed (fuzzy-never-sneaky, rule 4).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct FormatReport {
    /// The `/BaseFont` of the run AFTER the edit (the target's, if the
    /// family changed; else the run's own).
    pub base_font: String,
    /// The core-visible glyph source of the post-edit font.
    pub glyph_source: EditGlyphSource,
    /// Whether the post-edit font is an embedded **subset**.
    pub subset: bool,
    /// `(old, new)` size when the size changed.
    pub size_change: Option<(f64, f64)>,
    /// The stored fill-colour operator (`rg`/`g`/`k`) when the colour
    /// changed — the ACTUAL space pdfce stored, never force-DeviceRGB.
    pub fill_space: Option<&'static str>,
    /// Whether the run's original fill colour was a non-device (`Other`)
    /// space that a colour change narrowed (disclosed).
    pub fill_narrowed: bool,
    /// `(old_base_font, new_base_font)` when the family/style changed.
    pub font_change: Option<(String, String)>,
    /// `(ambient operand, emitted operand)` for `Tc` when character
    /// spacing was requested. Present even when the two are equal and
    /// therefore nothing was emitted — the operator asked, so the report
    /// answers.
    pub char_spacing_change: Option<(f64, f64)>,
    /// `(ambient operand, emitted operand)` for `Tz` (percentages) when
    /// horizontal scaling was requested.
    pub h_scale_change: Option<(f64, f64)>,
    /// The baseline position applied, when the script toggle was used.
    pub script: Option<ScriptPosition>,
    /// `(base size, emitted reduced size)` when a super/subscript reduced
    /// the run's `Tf` size. `None` for [`ScriptPosition::Normal`], which
    /// does not resize.
    pub script_size: Option<(f64, f64)>,
    /// `(ambient Ts, emitted Ts)` when the script toggle changed the rise.
    pub rise_change: Option<(f64, f64)>,
    /// Which parameters were restored by **re-spelling** the ambient value
    /// rather than by replaying the producer's own bytes — the
    /// [`AmbientOrigin::ObservedIndirect`](crate::text_state::AmbientOrigin)
    /// rung of R88's ladder. The value is right; only the spelling is
    /// pdfce's. Disclosed, never silent (rule 4).
    pub restore_narrowed: Vec<TextStateParam>,
    /// Whether the edited run's show operator carries `TJ` numeric
    /// adjustments whose slack a spacing/scaling change has invalidated —
    /// the `Tz` × justify interaction (§9.3.4/§9.4.4). Disclosed with a
    /// re-justify offer; never silently left mis-justified.
    pub justify_slack_invalidated: bool,
    /// `ΔA = A_new − A_old` in text-space units (0 for a colour-only edit).
    pub advance_delta: f64,
    /// The follower disposition actually used.
    pub disposition: FollowerDisposition,
    /// How many following absolute `Tm`s were repositioned by `ΔA` (reflow).
    pub followers_repositioned: u64,
    /// The `/MCID` of the enclosing marked-content sequence, if the edit was
    /// inside a Tagged-PDF sequence (its wrapper is preserved; §14.7).
    pub tagged_mcid: Option<i64>,
    /// The content-stream object number that was rewritten.
    pub content_object: u32,
    /// Extra content objects collapsed/emptied on a multi-stream page.
    pub extra_objects_emptied: u64,
    /// Every operator-facing disclosure, verbatim (surfaced by the UI/CLI).
    pub disclosures: Vec<String>,
}

/// A failure to format — every variant is a clean, named outcome, never a
/// crash (rule 4).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FormatError {
    /// A font-classification refusal against the family-change target
    /// (composite / symbolic-no-encoding / `/ToUnicode`-only) — by name.
    #[error(transparent)]
    Refused(Refusal),
    /// The family-change target does not cover every character in the run
    /// (encoding gap, or an embedded-subset code it does not already carry).
    /// NOTHING was applied.
    #[error(transparent)]
    CoverageFailure(Refusal),
    /// No formatting operation was requested (every `--set-*` /
    /// spacing / scaling / script flag absent).
    #[error(
        "no formatting operation was requested (need one of size, colour, font, character \
         spacing, horizontal scaling, or super/subscript)"
    )]
    NoOp,
    /// A text-state operator would have to be emitted for this run, but its
    /// **ambient** value cannot be restored afterwards — so pdfce refuses
    /// rather than leaking the new value into every following show operator
    /// (R88 tier 3, decision 019 §3.4). Nothing was applied.
    #[error(transparent)]
    AmbientUnrestorable(#[from] AmbientRestoreError),
    /// A `--h-scale` percentage outside the range pdfce will write.
    #[error("invalid horizontal scaling: {0}")]
    BadHorizScale(String),
    /// A `--set-color` model/component mismatch or out-of-range component.
    #[error("invalid colour: {0}")]
    BadColor(String),
    /// `--set-font` names no existing font resource on the page.
    #[error(
        "the target font {0:?} is not an existing font resource on this page; \
         adding a new font resource / embedding a new face is deferred (FF-C)"
    )]
    TargetFontMissing(String),
    /// No page at the requested index.
    #[error("no page at index {0}")]
    PageIndex(usize),
    /// The find text was not present in any editable run.
    #[error("text to format ({0:?}) was not found in an editable run on the page")]
    NoMatch(String),
    /// The run is real but this cut cannot format it (composite font, a
    /// `'`/`"` anchor, a cross-element `TJ` match, an unresolvable font
    /// resource — including outlined/vectorized text).
    #[error("this run cannot be formatted in the first cut: {0}")]
    Unsupported(String),
    /// The document is encrypted (out of scope for text editing).
    #[error(
        "the document is encrypted; in-place text formatting of encrypted files is out of scope"
    )]
    Encrypted,
    /// The page's content stream could not be parsed.
    #[error("content stream parse failed: {0}")]
    Content(#[from] ContentError),
    /// The page tree could not be walked.
    #[error("page tree error: {0}")]
    PageTree(#[from] PageTreeError),
    /// The incremental save failed.
    #[error("save failed: {0}")]
    Write(#[from] crate::writer::WriteError),
}

impl FormatError {
    /// Map a reused 14.1 [`EditError`] (from the shared locate/match/classify/
    /// save helpers) onto the corresponding [`FormatError`], so a caller sees
    /// one error vocabulary.
    fn from_edit(err: EditError) -> Self {
        match err {
            EditError::Refused(r) => Self::Refused(r),
            EditError::PageIndex(i) => Self::PageIndex(i),
            EditError::NoMatch(s) => Self::NoMatch(s),
            EditError::Unsupported(s) => Self::Unsupported(s),
            EditError::Encrypted => Self::Encrypted,
            EditError::Content(e) => Self::Content(e),
            EditError::PageTree(e) => Self::PageTree(e),
            EditError::Write(e) => Self::Write(e),
        }
    }
}

/// Format the page's own text in place: change the run's size, fill colour,
/// and/or font family — reusing Pass 14.1's advance-preserving surgery — and
/// save incrementally.
///
/// Returns the appended PDF bytes plus a [`FormatReport`] carrying every
/// disclosure. A family change whose target cannot cover the run yields
/// [`FormatError::CoverageFailure`] BEFORE any save (nothing partially
/// applied, rule 4).
///
/// # Errors
///
/// See [`FormatError`]: a no-op, an invalid colour, a missing target font, a
/// named coverage/classification refusal, no match, an unsupported run, an
/// encrypted document, a parse/page-tree failure, or a save failure.
pub fn set_format(
    doc: &Document,
    req: &FormatRequest,
    opts: &FormatOptions,
) -> Result<FormatOutcome, FormatError> {
    if req.is_empty() {
        return Err(FormatError::NoOp);
    }
    if doc.trailer().contains_key(b"Encrypt") {
        return Err(FormatError::Encrypted);
    }
    let pages = page_tree::pages(doc)?;
    let page = pages
        .get(req.page_index)
        .ok_or(FormatError::PageIndex(req.page_index))?;
    // BASE READ (decision 018 caller audit) — same rationale as
    // `text_edit::edit_text`: this is the one-shot `&Document` entry point,
    // planning against the file as loaded for an incremental save.
    let stream = ContentStream::from_page(&doc.view(), page)?;
    let plan = plan_format(doc, page, &stream, req, opts)?;
    // Incremental save (R34/R70), exactly as 14.1: the plan's report already
    // carries the correct content_object / extra_objects_emptied, so the
    // write's returned identity is discarded.
    let (bytes, _content_object, _extra) =
        write_incremental(doc, page, &plan.new_content).map_err(FormatError::from_edit)?;
    Ok(FormatOutcome {
        bytes,
        report: plan.report,
    })
}

/// The result of planning a format edit WITHOUT committing it: the fully
/// state-wrapped replacement content buffer plus the complete
/// [`FormatReport`]. The `pdfce-core` session sibling of the 14.1
/// [`EditPlan`](crate::text_edit::edit::EditPlan) (Pass 14.3 §0.2) —
/// see that type's docs for why the surgery is split from the save.
pub(crate) struct FormatPlan {
    /// The spliced, decoded replacement content for the page's first content
    /// object.
    pub(crate) new_content: Vec<u8>,
    /// The complete disclosure/diagnostic report.
    pub(crate) report: FormatReport,
}

/// Plan a FORMAT edit over an already-decoded content `stream`: locate the
/// anchor, apply the size/colour/family state-wrap, relayout, and splice —
/// returning the new content buffer and the full report, but performing NO
/// save. Factored out of [`set_format`] so the interactive
/// [`EditSession::format_text`](crate::edit::EditSession::format_text) reuses
/// the identical surgery (Pass 14.3 §0.2). Callers must pre-check
/// [`FormatRequest::is_empty`] (→ [`FormatError::NoOp`]) and encryption; this
/// planner assumes both, exactly as [`plan_edit`](crate::text_edit::edit::plan_edit)
/// assumes the encryption pre-check.
///
/// # Errors
///
/// See [`FormatError`]: a coverage/classification refusal, a missing target
/// font, no match, an unsupported run, a page with no `/Contents`, or a
/// content-parse failure.
pub(crate) fn plan_format(
    doc: &Document,
    page: &crate::page_tree::Page,
    stream: &ContentStream,
    req: &FormatRequest,
    opts: &FormatOptions,
) -> Result<FormatPlan, FormatError> {
    // Content-object identity + the save-independent report object numbers.
    let content_id = *page
        .contents
        .first()
        .ok_or_else(|| FormatError::Unsupported("the page has no /Contents to edit".to_owned()))?;
    let extra_emptied = page.contents.len().saturating_sub(1) as u64;

    // --- pass 1: record every operator with its text + fill state (the
    //     shared 14.1 walk, now also carrying fill colour) ---
    let mut walk = Walk::new(doc, &page.resources);
    for op in stream.operations() {
        walk.operation(&op, &stream.buf);
    }
    let recs = walk.recs;

    // --- locate the anchor (identical to 14.1) ---
    let locate = EditRequest {
        page_index: req.page_index,
        find: req.find.clone(),
        replace: String::new(),
        pinned_span: req.pinned_span,
    };
    let anchor_index = find_anchor(&recs, &locate).map_err(FormatError::from_edit)?;
    let (a_start, a_end, anchor) = match recs.get(anchor_index) {
        Some(OpRec {
            start,
            end,
            rec: Rec::Show(s),
        }) => (*start, *end, s),
        _ => return Err(FormatError::NoMatch(req.find.clone())),
    };
    if matches!(anchor.op, ShowOp::Quote | ShowOp::DoubleQuote) {
        return Err(FormatError::Unsupported(
            "formatting a run shown with the ' or \" operator is deferred (first cut edits Tj/TJ)"
                .to_owned(),
        ));
    }

    // --- map the find text to a contiguous code range in one element ---
    let m = match_run(anchor, &req.find).map_err(FormatError::from_edit)?;

    // --- resolve the run's OWN font (needed for A_old, and for size/colour
    //     the effective face) ---
    let orig_dict =
        resolve_font_dict(doc, &page.resources, &anchor.font_name).ok_or_else(|| {
            FormatError::Unsupported(
            "the run's font resource is unresolvable (outlined/vector art has no font to format)"
                .to_owned(),
        )
        })?;
    // `&doc.view()` (Pass 17.1) — base-relative planner, see `edit.rs`.
    let orig_font = ExtractFont::resolve(&doc.view(), orig_dict);
    let orig_size = anchor.tf_size;

    // --- resolve the family-change target, if any, and re-encode the run ---
    let font_plan = plan_font(doc, page_resources(page), &recs, req)?;

    // --- the run's BASE size: what it would be shown at with no script
    //     reduction. Every R89 ratio (script rise, script size, relative
    //     `Tc`) is derived from THIS, so a later resize re-derives all of
    //     them consistently rather than carrying a stale operand. ---
    let base_size = req.set_size.unwrap_or(orig_size);

    // --- resolve the super/subscript toggle into its two derived operands
    //     (decision 019 §3.2, R89) ---
    let script = req.set_script;
    let script_metrics = script.and_then(ScriptPosition::metrics);
    // The size actually written into the emitted `Tf`. `Normal` and "no
    // script requested" both leave it at the base size.
    let emitted_size =
        script_metrics.map_or(base_size, |m| derived_operand(base_size * m.size_factor));
    // The rise this slice would put in force over the run. `Normal` is an
    // explicit request for the spec default, which is how an operator
    // flattens an inherited rise without a free-form control.
    let script_rise =
        script.map(|_| script_metrics.map_or(0.0, |m| derived_operand(base_size * m.rise_ratio)));

    // --- resolve `Tc` (R89: relative specs resolve against the BASE size,
    //     not the script-reduced one, so a superscript keeps the same
    //     tracking as the text it sits beside) ---
    // An ABSOLUTE spec passes through untouched — what the operator typed
    // is what the file gets. Only a RELATIVE one is pdfce's own arithmetic
    // and therefore rounded.
    let new_tc = req.set_char_spacing.map(|spec| match spec {
        MetricSpec::Absolute(v) => v,
        MetricSpec::Relative(_) => derived_operand(spec.resolve(base_size)),
    });

    // --- validate `Tz` before anything is planned ---
    if let Some(pct) = req.set_h_scale {
        if !pct.is_finite() {
            return Err(FormatError::BadHorizScale(format!(
                "{pct} is not a finite number"
            )));
        }
        // §9.3.4 makes `Tz` a percentage of normal width and does not
        // forbid 0 or a negative (which mirrors glyphs), but pdfce will not
        // WRITE one: 0 collapses every advance in the run to zero, which
        // looks like a rendering bug rather than an edit, and a mirrored
        // run has no operator-facing meaning in a formatting control.
        // Refused by name rather than silently clamped (rule 4).
        if pct <= 0.0 {
            return Err(FormatError::BadHorizScale(format!(
                "{pct}% would collapse or mirror the run; horizontal scaling must be > 0 \
                 (§9.3.4: a percentage of normal width, 100 = normal)"
            )));
        }
    }

    let new_codes: Vec<u8> = match &font_plan {
        Some(plan) => plan.new_codes.clone(),
        None => m.old_codes.clone(),
    };
    let advance_font: &ExtractFont = font_plan.as_ref().map_or(&orig_font, |p| &p.font);

    // --- advance delta (§9.4.4) ---
    //
    // `tx = ((w0 − Tj/1000)·Tfs + Tc + Tw)·Th`. Before Pass 19.1 both sides
    // used the run's AMBIENT `Tc`/`Th` because neither could change; now
    // they can, and A_new must be evaluated at the values that will be IN
    // FORCE over `mid`. Getting this wrong does not misdraw the run — it
    // mis-positions everything after it, because ΔA is what the follower
    // relayout is driven by.
    //
    // `Tw` is unchanged by this slice (decision 019 §3.3 keeps word spacing
    // engine-only) but is still multiplied by the NEW `Th`, exactly as
    // §9.3.4 requires ("It shall also affect the spacing parameters `Tc`
    // and `Tw`"). `Ts` deliberately does not appear: rise is a `Trm`
    // translation (§9.3.7) and changes position, not advance.
    let eff_tc = new_tc.unwrap_or_else(|| anchor.tc());
    let eff_th = req
        .set_h_scale
        .map_or_else(|| anchor.th(), |pct| pct / 100.0);
    let a_old: f64 = m
        .old_codes
        .iter()
        .map(|&c| {
            glyph_advance_with(
                &orig_font,
                c,
                orig_size,
                anchor.tc(),
                anchor.tw(),
                anchor.th(),
            )
        })
        .sum();
    let a_new: f64 = new_codes
        .iter()
        .map(|&c| glyph_advance_with(advance_font, c, emitted_size, eff_tc, anchor.tw(), eff_th))
        .sum();
    let delta = a_new - a_old;

    // --- build the state-set / state-restore operator sequences ---
    let size_changed = req.set_size.is_some();
    let font_changed = font_plan.is_some();
    // A super/subscript reduces `Tfs`, so it touches `Tf` even when no
    // explicit size was asked for.
    let tf_touched = size_changed || font_changed || (emitted_size - orig_size).abs() > STATE_EPS;

    let set_font_name: &[u8] = font_plan
        .as_ref()
        .map_or(anchor.font_name.as_slice(), |p| p.resource.as_slice());

    let mut set_ops: Vec<u8> = Vec::new();
    let mut restore_ops: Vec<u8> = Vec::new();
    if tf_touched {
        push_tf(&mut set_ops, set_font_name, emitted_size);
        push_tf(&mut restore_ops, &anchor.font_name, orig_size);
    }
    let mut fill_narrowed = false;
    if let Some(nf) = &req.set_fill {
        push_space(&mut set_ops);
        push_fill(&mut set_ops, nf);
        push_space(&mut restore_ops);
        restore_ops.extend_from_slice(&anchor.fill_color.restore_bytes());
        fill_narrowed = anchor.fill_color.is_other();
    }

    // --- the Pass 19.1 text-state parameters, each through R88's ladder ---
    //
    // `Tf` and the fill operator above are handled by 14.2's own bespoke
    // restores (the run's recorded font name/size, and `FillState`'s own
    // three-tier `restore_bytes`). The six §9.3 single-operand parameters
    // go through the SHARED ladder in `text_state`, which is where the
    // spec-default / observed-bytes / re-spelled / refuse trichotomy lives.
    let mut restore_narrowed: Vec<TextStateParam> = Vec::new();
    let mut emitted_state: Vec<TextStateParam> = Vec::new();
    if let Some(tc) = new_tc {
        push_state_param(
            &mut set_ops,
            &mut restore_ops,
            &anchor.text_state,
            TextStateParam::CharSpacing,
            tc,
            &mut restore_narrowed,
            &mut emitted_state,
        )?;
    }
    if let Some(pct) = req.set_h_scale {
        push_state_param(
            &mut set_ops,
            &mut restore_ops,
            &anchor.text_state,
            TextStateParam::HorizScale,
            pct,
            &mut restore_narrowed,
            &mut emitted_state,
        )?;
    }
    if let Some(rise) = script_rise {
        push_state_param(
            &mut set_ops,
            &mut restore_ops,
            &anchor.text_state,
            TextStateParam::Rise,
            rise,
            &mut restore_narrowed,
            &mut emitted_state,
        )?;
    }

    // --- the `Tz` × justify interaction (decision 019 §6 slice 19.1) ---
    //
    // §9.3.4: `Th` affects "both the glyph's shape and its horizontal
    // displacement… as well as any positioning adjustments performed by the
    // `TJ` operator". A justified line carries its slack as `TJ` numbers
    // sized against the line's width at the time it was justified. A
    // spacing/scaling/script change alters that width by ΔA, so the slack
    // no longer fills the measure and the line's right edge walks.
    //
    // NOTE (a correction to decision 019's framing, verified against this
    // module's actual emission shape): the surviving `TJ` numbers are NOT
    // themselves rescaled by the new `Th`. They sit in the `pre`/`post`
    // segments, which are OUTSIDE the set/restore wrap and therefore run at
    // the ambient `Th`. The invalidation is real, but its mechanism is the
    // width change, not a rescale of the adjustments — see
    // `disclosure_justify_invalidated`.
    let run_carries_tj_slack = anchor
        .elems
        .iter()
        .any(|e| matches!(e, ShowElem::Num(n) if n.abs() > STATE_EPS));
    let justify_slack_invalidated = run_carries_tj_slack
        && delta != 0.0
        && (req.set_h_scale.is_some() || req.set_char_spacing.is_some() || script.is_some());

    // --- re-emit the anchor as pre | set | mid | restore | post ---
    let (pre, post) = split_segments(anchor, &m);
    let mid = vec![ShowElem::Str(new_codes.clone())];
    let mut replacement: Vec<u8> = Vec::new();
    let push_seg = |seg: Vec<u8>, out: &mut Vec<u8>| {
        if seg.is_empty() {
            return;
        }
        if !out.is_empty() {
            out.push(b' ');
        }
        out.extend_from_slice(&seg);
    };
    push_seg(emit_show(&pre), &mut replacement);
    push_seg(std::mem::take(&mut set_ops), &mut replacement);
    push_seg(emit_show(&mid), &mut replacement);
    push_seg(std::mem::take(&mut restore_ops), &mut replacement);
    push_seg(emit_show(&post), &mut replacement);

    let mut edits: Vec<(usize, usize, Vec<u8>)> = vec![(a_start, a_end, replacement)];

    // --- relayout: reflow followers by ΔA, or pin with a compensating TJ ---
    let mut followers = 0u64;
    if delta != 0.0 {
        match opts.disposition {
            FollowerDisposition::Reflow => {
                for r in recs.iter().skip(anchor_index + 1) {
                    match &r.rec {
                        Rec::Boundary => break,
                        Rec::Show(s) if matches!(s.op, ShowOp::Quote | ShowOp::DoubleQuote) => {
                            break;
                        }
                        Rec::Tm([a, b, c, d, e, f]) => {
                            edits.push((r.start, r.end, emit_tm([*a, *b, *c, *d, *e + delta, *f])));
                            followers += 1;
                        }
                        _ => {}
                    }
                }
            }
            FollowerDisposition::Pin => {
                // Append a compensating `[N] TJ` after the whole run so
                // nothing following moves (surgery ref §2). After restore the
                // active size is `orig_size`, so scale N by it.
                if let Some(n) = compensating_tj(delta, orig_size, anchor.th())
                    && let Some((_, _, bytes)) = edits.first_mut()
                {
                    bytes.push(b' ');
                    bytes.push(b'[');
                    emit_number(bytes, n);
                    bytes.extend_from_slice(b"] TJ");
                }
            }
        }
    }

    // --- splice (NO save here; the caller — the free function or the
    //     session — performs its own write step, Pass 14.3 §0.2) ---
    let new_content = splice(&stream.buf, &mut edits);

    // --- report + disclosures ---
    let (report_font, embedded, subset) = match &font_plan {
        Some(plan) => (plan.font.base_font.clone(), plan.embedded, plan.subset),
        None => {
            let (e, s) = embed_and_subset(doc, orig_dict, &orig_font);
            (orig_font.base_font.clone(), e, s)
        }
    };

    let mut disclosures: Vec<String> = Vec::new();
    if let Some(plan) = &font_plan {
        disclosures.extend(plan.disclosures.iter().cloned());
    }
    disclosures.push(disclosure_save());
    disclosures.push(trust_disclosure(embedded, &report_font));
    if size_changed {
        disclosures.push(disclosure_size(orig_size, base_size));
    }
    if let Some(nf) = &req.set_fill {
        disclosures.push(disclosure_fill(nf));
        if fill_narrowed {
            disclosures.push(disclosure_narrowing(nf));
        }
    }
    if delta != 0.0 && matches!(opts.disposition, FollowerDisposition::Reflow) {
        disclosures.push(disclosure_reflow());
    }
    // --- Pass 19.1 disclosures ---
    if let Some(spec) = req.set_char_spacing {
        disclosures.push(disclosure_char_spacing(
            spec,
            base_size,
            anchor.text_state.char_spacing.value,
            eff_tc,
        ));
    }
    if let Some(pct) = req.set_h_scale {
        disclosures.push(disclosure_h_scale(anchor.text_state.h_scale.value, pct));
    }
    if let Some(pos) = script {
        disclosures.push(disclosure_script(
            pos,
            base_size,
            emitted_size,
            script_rise.unwrap_or(0.0),
        ));
    }
    if !emitted_state.is_empty() {
        disclosures.push(disclosure_state_scope(&emitted_state));
    }
    if !restore_narrowed.is_empty() {
        disclosures.push(disclosure_restore_narrowed(
            &restore_narrowed,
            &anchor.text_state,
        ));
    }
    if justify_slack_invalidated {
        disclosures.push(disclosure_justify_invalidated(
            req.set_h_scale.is_some(),
            req.set_char_spacing.is_some() || script.is_some(),
            delta,
        ));
    }
    if let Some(mcid) = anchor.mcid {
        disclosures.push(disclosure_tagged(mcid));
    }
    if extra_emptied > 0 {
        disclosures.push(format!(
            "multi-stream page: {extra_emptied} additional /Contents stream(s) were collapsed \
             into the first and emptied so the edit's byte offsets stay coherent."
        ));
    }

    let report = FormatReport {
        base_font: report_font,
        glyph_source: if embedded {
            EditGlyphSource::Embedded
        } else {
            EditGlyphSource::NonEmbedded
        },
        subset,
        size_change: size_changed.then_some((orig_size, base_size)),
        fill_space: req.set_fill.as_ref().map(|nf| nf.model.operator()),
        fill_narrowed,
        font_change: font_plan
            .as_ref()
            .map(|p| (orig_font.base_font.clone(), p.font.base_font.clone())),
        char_spacing_change: new_tc.map(|tc| (anchor.text_state.char_spacing.value, tc)),
        h_scale_change: req
            .set_h_scale
            .map(|pct| (anchor.text_state.h_scale.value, pct)),
        script,
        script_size: script_metrics.map(|_| (base_size, emitted_size)),
        rise_change: script_rise.map(|r| (anchor.text_state.rise.value, r)),
        restore_narrowed,
        justify_slack_invalidated,
        advance_delta: delta,
        disposition: opts.disposition,
        followers_repositioned: followers,
        tagged_mcid: anchor.mcid,
        content_object: content_id.num,
        extra_objects_emptied: extra_emptied,
        disclosures,
    };
    Ok(FormatPlan {
        new_content,
        report,
    })
}

// ===================================================================
// Family-change planning (the load-bearing gate)
// ===================================================================

/// The resolved plan for a family/style change: the target resource key, the
/// target [`ExtractFont`], the re-encoded run codes, and the target's
/// embedded/subset classification.
struct FontPlan {
    resource: Vec<u8>,
    font: ExtractFont,
    new_codes: Vec<u8>,
    embedded: bool,
    subset: bool,
    disclosures: Vec<String>,
}

/// Plan the family/style change, or `None` when the request does not change
/// the font. Applies the coverage gate: the target's resolved `/Encoding`
/// must show every character in the run, and (for an embedded subset) every
/// resulting code must already be carried on the page — else
/// [`FormatError::CoverageFailure`], with nothing applied.
fn plan_font(
    doc: &Document,
    resources: &Dict,
    recs: &[OpRec],
    req: &FormatRequest,
) -> Result<Option<FontPlan>, FormatError> {
    let Some(sel) = &req.set_font else {
        return Ok(None);
    };
    // Locate an existing font resource by key, then by /BaseFont.
    let (resource, target_dict) = resolve_target_resource(doc, resources, &sel.selector)
        .ok_or_else(|| FormatError::TargetFontMissing(sel.selector.clone()))?;
    let target = ExtractFont::resolve(&doc.view(), target_dict);

    // Font-level refuse triggers (composite / symbolic-no-encoding /
    // /ToUnicode-only) — reuse 14.1's classifier verbatim.
    let FontClass { embedded, subset } =
        classify_font(doc, target_dict, &target).map_err(FormatError::from_edit)?;

    // Build the inverse map and re-encode the SAME characters (the matched
    // text) into the target face.
    let glyph_names = target.glyph_names().ok_or_else(|| {
        FormatError::Unsupported("the target font has no invertible encoding".to_owned())
    })?;
    let inverse = InverseEncoding::build(&target.base_font, glyph_names);
    let prefer: BTreeSet<u8> = BTreeSet::new();
    let encoded = inverse
        .encode_str(&req.find, &prefer)
        .map_err(FormatError::CoverageFailure)?;

    // Embedded-subset floor on the TARGET: a resulting code the subset does
    // not already carry on the page is a coverage failure (can't add glyphs
    // without FF-C).
    if embedded && subset {
        let carried = carried_codes(recs, &resource);
        for (u, &code) in req.find.chars().zip(encoded.codes.iter()) {
            if !carried.contains(&code) {
                return Err(FormatError::CoverageFailure(Refusal {
                    trigger: RInvTrigger::TargetAbsent,
                    character: Some(u),
                    base_font: target.base_font.clone(),
                    message: format!(
                        "coverage failure: target font '{}' is an embedded SUBSET that does not \
                         already carry code {} for character U+{:04X} '{}'; embedding a new glyph \
                         is deferred to FF-C. Nothing was applied.",
                        target.base_font, code, u as u32, u
                    ),
                }));
            }
        }
    }

    let mut disclosures = encoded.disclosures;
    disclosures.push(format!(
        "font: the run's family/style was changed to '{}' and its {} character(s) were re-encoded \
         into that face's /Encoding (minimal-diff; no new font resource added, no embedding — \
         subsetting is FF-C).",
        target.base_font,
        req.find.chars().count()
    ));

    Ok(Some(FontPlan {
        resource,
        font: target,
        new_codes: encoded.codes,
        embedded,
        subset,
        disclosures,
    }))
}

/// Locate a family-change target resource by resource key first, then by
/// `/BaseFont` (exact, or with the §9.6.4 subset tag stripped).
fn resolve_target_resource<'a>(
    doc: &'a Document,
    resources: &'a Dict,
    selector: &str,
) -> Option<(Vec<u8>, &'a Dict)> {
    let fonts = resources
        .get(b"Font")
        .map(|o| doc.resolve(o))
        .and_then(Object::as_dict)?;
    // 1. exact resource-key match.
    if let Some(dict) = fonts
        .get(selector.as_bytes())
        .map(|o| doc.resolve(o))
        .and_then(Object::as_dict)
    {
        return Some((selector.as_bytes().to_vec(), dict));
    }
    // 2. /BaseFont match (exact or subset-stripped).
    for (key, val) in fonts.iter() {
        let Some(dict) = doc.resolve(val).as_dict() else {
            continue;
        };
        let base = dict
            .get(b"BaseFont")
            .map(|o| doc.resolve(o))
            .and_then(Object::as_name)
            .map(|n| String::from_utf8_lossy(n.as_bytes()).into_owned())
            .unwrap_or_default();
        if base == selector || subset_stem(&base) == selector {
            return Some((key.as_bytes().to_vec(), dict));
        }
    }
    None
}

// ===================================================================
// Segment splitting + operator emission
// ===================================================================

/// Split the anchor operator's element list into the `pre` and `post`
/// segments around the matched code range `[b_lo, b_hi)` in element `m.elem`.
/// The `mid` segment (the matched codes) is supplied fresh by the caller
/// (re-encoded for a font change, or the original codes otherwise).
fn split_segments(anchor: &ShowData, m: &MatchRun) -> (Vec<ShowElem>, Vec<ShowElem>) {
    let mut pre: Vec<ShowElem> = Vec::new();
    let mut post: Vec<ShowElem> = Vec::new();
    for (i, e) in anchor.elems.iter().enumerate() {
        if i < m.elem {
            pre.push(e.clone());
        } else if i > m.elem {
            post.push(e.clone());
        } else if let ShowElem::Str(bytes) = e {
            // The matched element: bytes before the match go to pre, bytes
            // after to post; the matched bytes themselves are the mid segment.
            if let Some(head) = bytes.get(..m.b_lo)
                && !head.is_empty()
            {
                pre.push(ShowElem::Str(head.to_vec()));
            }
            if let Some(tail) = bytes.get(m.b_hi..)
                && !tail.is_empty()
            {
                post.push(ShowElem::Str(tail.to_vec()));
            }
        } else {
            // A Num element cannot be the matched element (match_run only
            // matches inside a string); keep it defensively.
            pre.push(e.clone());
        }
    }
    (pre, post)
}

/// Round an operand pdfce **derived by its own arithmetic** to a precision
/// a PDF real can carry, so representation noise never reaches the file.
///
/// `12.0 × 0.60` is `7.199999999999999` in `f64`, and the writer's
/// shortest-round-trip formatter faithfully emits all sixteen digits of it.
/// That is three separate problems: it bloats a stream pdfce is trying to
/// keep minimal, it makes a diff unreadable, and it exceeds the ~5
/// significant decimal digits Annex C records as PDF's traditional real
/// precision — so the extra digits are noise that no consumer can use.
///
/// Six decimal places is chosen rather than five: it is past Annex C's
/// precision (so nothing meaningful is lost), it is far past any
/// typographic need (a millionth of a point), and it leaves headroom so a
/// small legitimate value such as a `0.00024` tracking is not flattened.
///
/// **Only pdfce's own derived values go through this.** An operand the
/// operator supplied absolutely is written exactly as given: rounding a
/// number a human typed would be a silent modification, which is the
/// opposite of what this function is for.
fn derived_operand(v: f64) -> f64 {
    const SCALE: f64 = 1_000_000.0;
    if v.is_finite() {
        (v * SCALE).round() / SCALE
    } else {
        v
    }
}

/// Plan ONE §9.3 text-state parameter change: append its set operator to
/// `set_ops` and the operator that puts the ambient value back to
/// `restore_ops` — or refuse, if the ambient value cannot be restored.
///
/// This is the single point where standing rule **R88**'s ladder is applied
/// in the formatting path, and it is deliberately one function rather than
/// three near-copies so the refuse tier cannot be forgotten on the third
/// one.
///
/// ## The scoping mechanism, and why it is not `q`/`Q`
///
/// Text state is graphics state and §9.3 keeps it alive across text objects
/// for the whole content stream — so an emitted `0.5 Tc` bleeds into every
/// following show operator unless something puts it back. The obvious fix,
/// wrapping the run in `q … Q`, is **illegal**: `q`/`Q` are Special
/// graphics state operators and are not admitted inside a text object
/// (§8.2 Table 51 / Figure 9), and splitting the `BT … ET` to use them
/// outside would discard `Tm` (§9.4.1) and force absolute re-positioning of
/// everything downstream, destroying the minimal-diff property. So the
/// scope is closed by restoring **by value**, inside the text object.
///
/// ## What each branch does
///
/// | ambient origin | restore emitted | disclosed? |
/// |---|---|---|
/// | never set in this stream | the Table 105 default (`0 Tc`, `100 Tz`, `0 Ts`) | no — provably correct |
/// | set by its own operator | that operator's **raw bytes**, so `0.5000 Tc` does not come back as `0.5 Tc` | no — byte-faithful |
/// | set as a side effect of `TD` or `"` | the value **re-spelled** as its own operator | **yes** — `restore_narrowed` |
/// | inherited from outside the buffer | nothing — the whole edit refuses | **yes** — a named error |
///
/// The third row is the trap Amendment A.1 added to the record: `"` sets
/// `Tw` **and** `Tc` *and shows a string* (Table 109), so replaying its
/// bytes as a "restore" would repaint the text. The bytes are simultaneously
/// a faithful record of the value and a catastrophic restore instruction,
/// which is why "use the raw bytes when they exist" is not the rule.
///
/// ## The no-op skip
///
/// A requested value already in force emits **nothing at all** — neither a
/// set nor a restore. This is a minimal-diff obligation (`ARCHITECTURE.md`
/// §5): `--h-scale 100` on an unscaled run must not add `100 Tz … 100 Tz`
/// to a stream that never needed it. It also has a useful second effect: a
/// no-change is never a refusal, even when the ambient is unrestorable,
/// because nothing needs restoring.
///
/// # Errors
///
/// [`FormatError::AmbientUnrestorable`] when the ambient value was
/// inherited from outside the content stream being edited (R88 tier 3) —
/// with nothing applied, rather than a guessed default that would silently
/// change content pdfce did not touch.
fn push_state_param(
    set_ops: &mut Vec<u8>,
    restore_ops: &mut Vec<u8>,
    ambient: &AmbientTextState,
    param: TextStateParam,
    new_operand: f64,
    restore_narrowed: &mut Vec<TextStateParam>,
    emitted: &mut Vec<TextStateParam>,
) -> Result<(), FormatError> {
    let current = ambient.get(param);
    if (new_operand - current.value).abs() <= STATE_EPS {
        // Already in force — see "The no-op skip" above.
        return Ok(());
    }
    // Build the restore FIRST: if it refuses, nothing has been appended to
    // either buffer and the caller's `?` aborts the whole plan cleanly.
    let restore = current.restore_bytes(param)?;
    if !current.is_byte_faithful() {
        restore_narrowed.push(param);
    }

    push_space(set_ops);
    emit_number(set_ops, new_operand);
    set_ops.push(b' ');
    set_ops.extend_from_slice(param.operator());

    push_space(restore_ops);
    restore_ops.extend_from_slice(&restore);

    emitted.push(param);
    Ok(())
}

/// Append a `/<name> <size> Tf` operator (§9.3.1) to `out`.
fn push_tf(out: &mut Vec<u8>, name: &[u8], size: f64) {
    if !out.is_empty() {
        out.push(b' ');
    }
    out.push(b'/');
    out.extend_from_slice(name);
    out.push(b' ');
    emit_number(out, size);
    out.extend_from_slice(b" Tf");
}

/// Append the chosen device fill operator (`<c…> rg`/`g`/`k`) to `out` — the
/// ACTUAL space, never force-DeviceRGB (parity-plus).
fn push_fill(out: &mut Vec<u8>, nf: &NewFill) {
    for (i, c) in nf.components.iter().enumerate() {
        if i > 0 {
            out.push(b' ');
        }
        emit_number(out, *c);
    }
    out.push(b' ');
    out.extend_from_slice(nf.model.operator().as_bytes());
}

/// Space-separate two operator groups being concatenated.
fn push_space(out: &mut Vec<u8>) {
    if !out.is_empty() {
        out.push(b' ');
    }
}

// ===================================================================
// Small helpers
// ===================================================================

/// Strip a §9.6.4 subset tag (`ABCDEF+Times-Bold` -> `Times-Bold`).
fn subset_stem(base: &str) -> &str {
    match base.split_once('+') {
        Some((tag, rest)) if is_subset_tag(base) && !tag.is_empty() => rest,
        _ => base,
    }
}

/// The page's `/Resources` dictionary (already resolved on the [`Page`]).
fn page_resources(page: &crate::page_tree::Page) -> &Dict {
    &page.resources
}

/// Classify a simple font's embedded/subset status for the REPORT, WITHOUT
/// the R-INV-2/3/4 refuse triggers — a size/colour-only edit keeps the run's
/// own codes and must never be gated by the inverse-encoding triggers (a
/// symbolic or `/ToUnicode`-only font can still be resized/recoloured).
fn embed_and_subset(doc: &Document, font_dict: &Dict, font: &ExtractFont) -> (bool, bool) {
    let embedded = font_dict
        .get(b"FontDescriptor")
        .map(|o| doc.resolve(o))
        .and_then(Object::as_dict)
        .is_some_and(|d| {
            d.contains_key(b"FontFile")
                || d.contains_key(b"FontFile2")
                || d.contains_key(b"FontFile3")
        });
    (embedded, is_subset_tag(&font.base_font))
}

fn disclosure_save() -> String {
    "save: this formatting change was written INCREMENTALLY (R34/R70); the prior text state \
     survives in the document's revision history by design. To truly remove content, use \
     redaction (Pass 8) — a distinct, security operation."
        .to_owned()
}

fn disclosure_size(old: f64, new: f64) -> String {
    format!(
        "size: the run's Tf size was changed from {old} to {new} pt (only the Tf operand; the \
         fill-colour operator and all other text-state are untouched). Arbitrary point values, \
         the single-line advance-preserving relayout, and flattening a selection that spanned \
         MULTIPLE original sizes to this one size are pdfce's OWN documented choices — Acrobat's \
         behaviour here is unconfirmed, so this is NOT a parity claim."
    )
}

fn disclosure_fill(nf: &NewFill) -> String {
    format!(
        "colour: the run's fill colour was set in {} (stored as the '{}' operator). pdfce PRESERVES \
         the chosen space and does NOT force-convert to DeviceRGB the way Acrobat's Edit-Text tool \
         does — a minimal-diff differentiator.",
        nf.model.space_name(),
        nf.model.operator()
    )
}

fn disclosure_narrowing(nf: &NewFill) -> String {
    format!(
        "colour NARROWING (disclosed, not silent — rule 4): the run's ORIGINAL fill colour was in a \
         non-device space (ICCBased/Separation/DeviceN/Indexed) that pdfce records as 'Other' and \
         cannot preserve through a device colour edit; this change narrows the edited run's colour \
         to {}. The rest of the run and the document keep their original colour (restored verbatim).",
        nf.model.space_name()
    )
}

fn disclosure_reflow() -> String {
    "relayout: the edited line was shifted by the advance delta and MAY now overflow the original \
     right margin; block re-wrap (reflow) is deferred (FF-A) — pin the tail (--pin) or enable \
     reflow to re-wrap."
        .to_owned()
}

/// Disclose a character-spacing change, in BOTH the unit the operator typed
/// and the operand actually written — R89's re-derivation is invisible
/// otherwise, and an operator who typed "20" deserves to be told it became
/// `0.24 Tc` and why.
fn disclosure_char_spacing(spec: MetricSpec, base_size: f64, ambient: f64, emitted: f64) -> String {
    let derivation = match spec {
        MetricSpec::Absolute(_) => String::from(
            "an ABSOLUTE value, written as typed at every font size (R89 MetricSpec::Absolute)",
        ),
        MetricSpec::Relative(per_mille) => format!(
            "a RELATIVE value: {per_mille} thousandths of the run's {base_size} pt base size \
             re-derives to {emitted} unscaled text-space units. Because Tc is NOT scaled by the \
             font size (§9.3, Table 105), storing the ratio rather than the operand is what keeps \
             a later resize correct (R89)"
        ),
    };
    format!(
        "character spacing: Tc {ambient} -> {emitted} for the matched run only (§9.3.2, unscaled \
         text-space units). You supplied {} in {} — {derivation}. Note Tc enters the §9.4.4 \
         advance, so the run's width changed and the rest of the line was relaid out accordingly.",
        spec.raw(),
        spec.unit_label()
    )
}

/// Disclose a horizontal-scaling change and the two things §9.3.4 says it
/// does that an operator may not expect (it reshapes glyphs, and it scales
/// the spacing parameters as well).
fn disclosure_h_scale(ambient: f64, emitted: f64) -> String {
    format!(
        "horizontal scaling: Tz {ambient}% -> {emitted}% for the matched run only (§9.3.4, a \
         percentage of normal width; 100 = normal). Tz affects BOTH the glyph's shape and its \
         horizontal displacement — it is a stretch, not just a re-spacing — and it also scales \
         the character- and word-spacing parameters in force over the run. The run's width \
         therefore changed and the rest of the line was relaid out."
    )
}

/// Disclose a super/subscript, quoting BOTH ratios by value.
///
/// Rule 4 makes this mandatory rather than nice: these numbers are pdfce's
/// own choice, not a measurement of Acrobat, and a chosen constant that is
/// never shown to the operator is exactly the "hidden magic number" the rule
/// exists to prevent. See [`SUPERSCRIPT`] for their provenance.
fn disclosure_script(pos: ScriptPosition, base_size: f64, emitted_size: f64, rise: f64) -> String {
    match pos.metrics() {
        Some(m) => format!(
            "{}: applied as Ts {rise} (rise = {} x the {base_size} pt base size, §9.3.7 — \
             positive moves the baseline UP) together with Tf {emitted_size} (size = {} x base). \
             These two ratios are pdfce's OWN documented defaults, NOT a parity claim: Acrobat's \
             internal values are undocumented by any source, so pdfce chose ordinary typographic \
             ones and discloses them by value. Both are fractions of the BASE size, so a later \
             resize re-derives them (R89). Rise is a text-rendering-matrix translation, so it \
             moves the run WITHOUT changing its advance; the reduced size does change the \
             advance, and the line was relaid out for it.",
            pos.label(),
            m.rise_ratio,
            m.size_factor
        ),
        None => format!(
            "{}: the matched run's baseline was reset to Ts 0 (§9.3.7) and its size left \
             unchanged. This is how an inherited non-zero rise is flattened for one run; a \
             free-form numeric rise is deferred (decision 019 §3.2, slice 19.2).",
            pos.label()
        ),
    }
}

/// Disclose the scoping mechanism itself — which operators were written and
/// why each one is paired with a restore.
///
/// Worth its own disclosure because the leak this prevents is the single
/// named `must_have` of the Acrobat catalog's spacing entry, and because a
/// reader diffing the content stream will see operators appear on BOTH
/// sides of the run and should know that is deliberate.
fn disclosure_state_scope(emitted: &[TextStateParam]) -> String {
    let names: Vec<String> = emitted.iter().map(ToString::to_string).collect();
    format!(
        "scope: {} written immediately before the matched run and RESTORED to the run's ambient \
         value immediately after it, inside the same text object. Text state is graphics state \
         and persists across text objects for the whole content stream (§9.3), so without the \
         restore this change would bleed into every following show operator. It is restored by \
         VALUE and not by q/Q, because q/Q are not permitted inside a text object (§8.2 Table 51) \
         and splitting the BT..ET to use them would discard the text matrix (§9.4.1). Text after \
         the run is provably unaffected.",
        names.join(" + ")
    )
}

/// Disclose that a restore had to be **re-spelled** rather than replayed —
/// R88's fourth rung (Amendment A.1).
fn disclosure_restore_narrowed(narrowed: &[TextStateParam], ambient: &AmbientTextState) -> String {
    let detail: Vec<String> = narrowed
        .iter()
        .map(|p| {
            let setter = ambient
                .get(*p)
                .indirect_setter()
                .unwrap_or("a side-effect operator");
            format!("{p} (originally set by `{setter}`)")
        })
        .collect();
    format!(
        "restore NARROWING (disclosed, not silent — rule 4): the ambient value of {} was restored \
         by RE-SPELLING it as its own operator rather than by replaying the producer's original \
         bytes. The value is exactly right; only its spelling is pdfce's. This is required, not a \
         shortcut: `TD` sets leading while ALSO moving to the next line, and `\"` sets word and \
         character spacing while ALSO SHOWING A STRING (§9.4.2 Table 108, §9.4.3 Table 109) — \
         replaying either as a restore would move the text or paint it twice.",
        detail.join(", ")
    )
}

/// Disclose that a spacing/scaling change invalidated a justified line's
/// slack, and OFFER the remedy — never silently leave the line wrong.
///
/// The wording is deliberately precise about the mechanism, because the
/// obvious explanation is wrong for pdfce's emission shape. `Th` does scale
/// `TJ` adjustments (§9.3.4), but the adjustments that carry a line's slack
/// sit OUTSIDE the set/restore wrap this module emits, so they run at the
/// ambient `Th` and are not rescaled. What actually breaks the justification
/// is that the edited run's own width moved by ΔA while the slack numbers
/// stayed put.
fn disclosure_justify_invalidated(h_scale: bool, spacing: bool, delta: f64) -> String {
    let cause = match (h_scale, spacing) {
        (true, true) => "horizontal scaling and the spacing/size change",
        (true, false) => "horizontal scaling",
        _ => "the spacing/size change",
    };
    format!(
        "JUSTIFY invalidated (disclosed, with a remedy — never silently left wrong): this run's \
         show operator carries TJ numeric adjustments, which is the shape a JUSTIFIED line's \
         slack takes (it can also be a producer's kerning — pdfce cannot tell the two apart from \
         the bytes alone). {cause} moved the run's width by {delta:.4} text-space units, so slack \
         computed against the OLD width no longer fills the measure and the line's right edge \
         will not align. The surviving TJ numbers were NOT rescaled: they sit outside the \
         restored state wrap and still run at the ambient Tz (§9.3.4 would rescale them only \
         inside it). REMEDY: re-justify the paragraph — `pdfce-cli reflow --page N --block K \
         --align justified` — which recomputes the slack against the new widths."
    )
}

fn disclosure_tagged(mcid: i64) -> String {
    format!(
        "tagged PDF: the format edit is inside a marked-content sequence (/MCID {mcid}); its \
         BDC/EMC+MCID wrapper was PRESERVED (structure references stay valid), but the structure \
         tree's /ActualText and reading order were NOT updated. A formatting change (colour/weight) \
         is a CONFIRMED trigger of Acrobat's tag-tree-corruption defect; pdfce DISCLOSES this \
         staleness rather than silently corrupting the accessibility tree (R72)."
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

    /// A one-page PDF with a Helvetica (WinAnsi, non-embedded) run and a
    /// device fill colour set before the run. `content` is the page content.
    fn fill_pdf(content: &str) -> Vec<u8> {
        build_pdf(
            content,
            &[(
                b"F1".to_vec(),
                b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
                    .to_vec(),
            )],
        )
    }

    /// Build a one-page PDF with the given content stream and a `/Font`
    /// subdictionary of `(key, font-object-body)` pairs (objects numbered
    /// from 5). Object layout mirrors the 14.1 test helper.
    fn build_pdf(content: &str, fonts: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
        let mut objects: Vec<(u32, Vec<u8>)> = Vec::new();
        // /Font entries → objects 5.. ; build the subdictionary text.
        let mut font_dict = Vec::new();
        font_dict.extend_from_slice(b"<< ");
        for (i, (key, _)) in fonts.iter().enumerate() {
            let num = 5 + i as u32;
            font_dict.push(b'/');
            font_dict.extend_from_slice(key);
            font_dict.extend_from_slice(format!(" {num} 0 R ").as_bytes());
        }
        font_dict.extend_from_slice(b">>");

        objects.push((1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()));
        let mut pages = b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] \
              /Resources << /Font "
            .to_vec();
        pages.extend_from_slice(&font_dict);
        pages.extend_from_slice(b" >> >>");
        objects.push((2, pages));
        objects.push((
            3,
            b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>".to_vec(),
        ));
        let body = content.as_bytes();
        let mut s = format!("<< /Length {} >>\nstream\n", body.len()).into_bytes();
        s.extend_from_slice(body);
        s.extend_from_slice(b"\nendstream");
        objects.push((4, s));
        for (i, (_, obj)) in fonts.iter().enumerate() {
            objects.push((5 + i as u32, obj.clone()));
        }

        let mut out = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();
        let mut offsets = std::collections::BTreeMap::new();
        let highest = 4 + fonts.len() as u32;
        for (num, obj) in &objects {
            offsets.insert(*num, out.len());
            out.extend_from_slice(format!("{num} 0 obj\n").as_bytes());
            out.extend_from_slice(obj);
            out.extend_from_slice(b"\nendobj\n");
        }
        let xref_at = out.len();
        out.extend_from_slice(format!("xref\n0 {}\n", highest + 1).as_bytes());
        out.extend_from_slice(b"0000000000 65535 f \n");
        for num in 1..=highest {
            match offsets.get(&num) {
                Some(off) => out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes()),
                None => out.extend_from_slice(b"0000000000 65535 f \n"),
            }
        }
        out.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
                highest + 1
            )
            .as_bytes(),
        );
        out
    }

    fn as_text(bytes: &[u8]) -> String {
        String::from_utf8_lossy(bytes).into_owned()
    }

    #[test]
    fn size_change_is_minimal_diff_and_touches_only_tf() {
        // Original blue fill; change only the size. The blue `rg` operator
        // must survive byte-verbatim, and no colour operator is added.
        let src = fill_pdf("BT /F1 12 Tf 0 0 1 rg 72 700 Td (hello) Tj ET\n");
        let doc = Document::from_bytes(src.clone()).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello").size(24.0),
            &FormatOptions::default(),
        )
        .unwrap();
        // Incremental append: original is a byte-prefix.
        assert_eq!(out.bytes.get(..src.len()), Some(src.as_slice()));
        assert_eq!(out.report.size_change, Some((12.0, 24.0)));
        assert!(
            out.report.fill_space.is_none(),
            "size-only touches no colour"
        );
        // The appended revision carries a `/F1 24 Tf` wrap. Only the Tf
        // operand changed: no new colour operator is emitted, and the
        // original blue `0 0 1 rg` survives byte-verbatim in the prefix.
        let text = as_text(&out.bytes);
        assert!(text.contains("/F1 24 Tf"), "new size emitted: {text}");
        assert!(
            text.contains("0 0 1 rg"),
            "original colour untouched: {text}"
        );
        // The incremental save re-emits the WHOLE edited content stream, so
        // the one original `rg` legitimately reappears in the appended
        // revision. The size-only invariant is that NO NEW colour operator
        // was added: the appended stream still carries exactly ONE `rg` and
        // zero `k`/DeviceGray `g` operators.
        let appended = as_text(&out.bytes[src.len()..]);
        assert_eq!(
            appended.matches(" rg").count(),
            1,
            "no extra colour op: {appended}"
        );
        assert!(!appended.contains(" k"), "no cmyk added: {appended}");
        // ΔA > 0: a bigger size advances more.
        assert!(out.report.advance_delta > 0.0);
    }

    #[test]
    fn color_change_stores_the_chosen_space_not_devicergb() {
        let src = fill_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        // Ask for CMYK — Acrobat would store rg; pdfce stores k.
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello")
                .fill(NewFill::new(FillModel::Cmyk, vec![0.0, 1.0, 1.0, 0.0]).unwrap()),
            &FormatOptions::default(),
        )
        .unwrap();
        assert_eq!(out.report.fill_space, Some("k"));
        assert!(!out.report.fill_narrowed);
        assert_eq!(out.report.advance_delta, 0.0, "colour never shifts advance");
        let text = as_text(&out.bytes);
        assert!(text.contains("0 1 1 0 k"), "CMYK stored as k: {text}");
    }

    #[test]
    fn color_change_on_other_space_discloses_narrowing() {
        // The run's fill is set through /CS0 cs … scn — an Other space.
        let src = build_pdf(
            "/CS0 cs 0.2 0.4 0.6 scn BT /F1 12 Tf 72 700 Td (hello) Tj ET\n",
            &[(
                b"F1".to_vec(),
                b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
                    .to_vec(),
            )],
        );
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello")
                .fill(NewFill::new(FillModel::Rgb, vec![1.0, 0.0, 0.0]).unwrap()),
            &FormatOptions::default(),
        )
        .unwrap();
        assert!(out.report.fill_narrowed, "Other original ⇒ narrowing");
        assert!(
            out.report
                .disclosures
                .iter()
                .any(|d| d.contains("NARROWING")),
            "narrowing disclosed"
        );
        // The tail restore re-emits the original scn sequence verbatim.
        let text = as_text(&out.bytes);
        assert!(
            text.contains("/CS0 cs 0.2 0.4 0.6 scn"),
            "verbatim restore: {text}"
        );
    }

    #[test]
    fn family_change_to_a_covering_target_succeeds_and_reencodes() {
        let src = build_pdf(
            "BT /F1 12 Tf 72 700 Td (hello) Tj ET\n",
            &[
                (
                    b"F1".to_vec(),
                    b"<< /Type /Font /Subtype /Type1 /BaseFont /Times-Roman /Encoding /WinAnsiEncoding >>"
                        .to_vec(),
                ),
                (
                    b"F2".to_vec(),
                    b"<< /Type /Font /Subtype /Type1 /BaseFont /Times-Bold /Encoding /WinAnsiEncoding >>"
                        .to_vec(),
                ),
            ],
        );
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello").font(FontSelector::new("F2")),
            &FormatOptions::default(),
        )
        .unwrap();
        assert_eq!(
            out.report.font_change,
            Some(("Times-Roman".to_owned(), "Times-Bold".to_owned()))
        );
        let text = as_text(&out.bytes);
        assert!(text.contains("/F2 12 Tf"), "Tf swapped to target: {text}");
        // WinAnsi→WinAnsi keeps the same codes, so "hello" still shows.
        assert!(text.contains("(hello) Tj"), "re-encoded run: {text}");
    }

    #[test]
    fn family_change_by_base_font_name_resolves_the_resource() {
        let src = build_pdf(
            "BT /F1 12 Tf 72 700 Td (hi) Tj ET\n",
            &[
                (
                    b"F1".to_vec(),
                    b"<< /Type /Font /Subtype /Type1 /BaseFont /Times-Roman /Encoding /WinAnsiEncoding >>"
                        .to_vec(),
                ),
                (
                    b"F2".to_vec(),
                    b"<< /Type /Font /Subtype /Type1 /BaseFont /Times-Bold /Encoding /WinAnsiEncoding >>"
                        .to_vec(),
                ),
            ],
        );
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hi").font(FontSelector::new("Times-Bold")),
            &FormatOptions::default(),
        )
        .unwrap();
        assert!(as_text(&out.bytes).contains("/F2 12 Tf"));
    }

    #[test]
    fn family_change_to_a_partially_covering_target_is_refused_nothing_applied() {
        // F3 remaps code 111 ('o') to /bullet, so 'o' is uncovered.
        let src = build_pdf(
            "BT /F1 12 Tf 72 700 Td (hello) Tj ET\n",
            &[
                (
                    b"F1".to_vec(),
                    b"<< /Type /Font /Subtype /Type1 /BaseFont /Times-Roman /Encoding /WinAnsiEncoding >>"
                        .to_vec(),
                ),
                (
                    b"F3".to_vec(),
                    b"<< /Type /Font /Subtype /Type1 /BaseFont /Times-Bold /Encoding \
                       << /Type /Encoding /BaseEncoding /WinAnsiEncoding /Differences [111 /bullet] >> >>"
                        .to_vec(),
                ),
            ],
        );
        let doc = Document::from_bytes(src).unwrap();
        let err = set_format(
            &doc,
            &FormatRequest::new(0, "hello").font(FontSelector::new("F3")),
            &FormatOptions::default(),
        )
        .unwrap_err();
        match err {
            FormatError::CoverageFailure(r) => {
                assert_eq!(r.character, Some('o'));
            }
            other => panic!("expected a coverage failure, got {other:?}"),
        }
    }

    #[test]
    fn missing_target_font_is_refused_by_name() {
        let src = fill_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let err = set_format(
            &doc,
            &FormatRequest::new(0, "hello").font(FontSelector::new("Nonexistent")),
            &FormatOptions::default(),
        )
        .unwrap_err();
        assert!(matches!(err, FormatError::TargetFontMissing(_)));
    }

    #[test]
    fn color_change_on_a_tagged_run_keeps_mcid_and_discloses() {
        let src = build_pdf(
            "/P << /MCID 0 >> BDC BT /F1 12 Tf 72 700 Td (hello) Tj ET EMC\n",
            &[(
                b"F1".to_vec(),
                b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
                    .to_vec(),
            )],
        );
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello")
                .fill(NewFill::new(FillModel::Rgb, vec![1.0, 0.0, 0.0]).unwrap()),
            &FormatOptions::default(),
        )
        .unwrap();
        assert_eq!(out.report.tagged_mcid, Some(0));
        let text = as_text(&out.bytes);
        // The MCID wrapper survives the edit (anti-corruption property).
        assert!(text.contains("/MCID 0"), "MCID preserved: {text}");
        assert!(
            out.report.disclosures.iter().any(|d| d.contains("R72")),
            "tagged staleness disclosed"
        );
    }

    #[test]
    fn no_op_request_is_refused() {
        let src = fill_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let err = set_format(
            &doc,
            &FormatRequest::new(0, "hello"),
            &FormatOptions::default(),
        )
        .unwrap_err();
        assert!(matches!(err, FormatError::NoOp));
    }

    #[test]
    fn outlined_text_target_is_refused_with_the_no_font_reason() {
        // A run whose font resource does not resolve (empty /Font dict) — the
        // outlined/vector-art analog: no font resource to format.
        let src = build_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n", &[]);
        let doc = Document::from_bytes(src).unwrap();
        let err = set_format(
            &doc,
            &FormatRequest::new(0, "hello").size(18.0),
            &FormatOptions::default(),
        )
        .unwrap_err();
        match err {
            FormatError::NoMatch(_) | FormatError::Unsupported(_) => {}
            other => panic!("expected no-font/unsupported, got {other:?}"),
        }
    }

    // ===============================================================
    // Pass 19.1 — Tc / Tz / super-subscript
    // ===============================================================

    /// Re-open a SAVED file and return the ambient §9.3 text state that the
    /// show operator containing `needle` runs under, exactly as a fresh
    /// reader would compute it.
    ///
    /// This is the round-trip oracle for every 19.1 test: asserting on the
    /// bytes pdfce emitted only proves pdfce wrote what pdfce meant, while
    /// re-walking the reloaded file proves a *reader* sees it. The leak
    /// tests in particular are meaningless without it — the whole failure
    /// mode is state that looks right at the edited operator and is wrong at
    /// the next one.
    fn ambient_after_reload(bytes: &[u8], needle: &str) -> AmbientTextState {
        let doc = Document::from_bytes(bytes.to_vec()).expect("saved file reloads");
        let pages = crate::page_tree::pages(&doc).expect("page tree");
        let page = pages.first().expect("one page");
        let stream = ContentStream::from_page(&doc.view(), page).expect("content");
        let mut walk = Walk::new(&doc, &page.resources);
        for op in stream.operations() {
            walk.operation(&op, &stream.buf);
        }
        walk.recs
            .iter()
            .find_map(|r| match &r.rec {
                Rec::Show(s) if s.text.contains(needle) => Some(s.text_state.clone()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no show operator containing {needle:?} after reload"))
    }

    /// Every emitted operator is scoped to the matched run: a `Tj` following
    /// the edited one must see the ORIGINAL ambient state.
    ///
    /// This is the Acrobat catalog's named `must_have` for this whole
    /// feature family — text state is graphics state and §9.3 retains it
    /// "across text objects in a single content stream", so an unrestored
    /// `Tc` silently re-spaces the rest of the page.
    #[test]
    fn no_ambient_bleed_past_the_edited_run() {
        let src = fill_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj 0 -14 Td (follower) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello")
                .char_spacing(MetricSpec::Absolute(0.5))
                .h_scale(90.0)
                .script(ScriptPosition::Superscript),
            &FormatOptions::default(),
        )
        .unwrap();

        let after = ambient_after_reload(&out.bytes, "follower");
        assert_eq!(after.char_spacing.value, 0.0, "Tc leaked past the run");
        assert_eq!(after.h_scale.value, 100.0, "Tz leaked past the run");
        assert_eq!(after.rise.value, 0.0, "Ts leaked past the run");
    }

    /// Tier 1 of R88's ladder: nothing set the parameter, so the restore is
    /// the provable Table 105 default.
    #[test]
    fn rung_one_unset_ambient_restores_the_spec_default() {
        let src = fill_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello").char_spacing(MetricSpec::Absolute(0.5)),
            &FormatOptions::default(),
        )
        .unwrap();
        let text = as_text(&out.bytes);
        assert!(text.contains("0.5 Tc"), "set emitted: {text}");
        assert!(
            text.contains("0 Tc"),
            "spec-default restore emitted: {text}"
        );
        assert!(out.report.restore_narrowed.is_empty(), "nothing narrowed");
        assert_eq!(out.report.char_spacing_change, Some((0.0, 0.5)));
    }

    /// Tier 2: the ambient was set by its own operator, so the restore is
    /// that operator's RAW BYTES — `90.00 Tz` must not come back as `90 Tz`.
    ///
    /// Byte fidelity is not cosmetic: a renormalized operand is a diff in an
    /// object pdfce claims not to have logically touched (R32/R46).
    #[test]
    fn rung_two_observed_ambient_restores_the_raw_operand_bytes() {
        let src = fill_pdf("BT /F1 12 Tf 90.00 Tz 72 700 Td (hello) Tj (tail) Tj ET\n");
        let doc = Document::from_bytes(src.clone()).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello").h_scale(75.0),
            &FormatOptions::default(),
        )
        .unwrap();
        let appended = as_text(&out.bytes[src.len()..]);
        assert!(appended.contains("75 Tz"), "set emitted: {appended}");
        assert!(
            appended.contains("90.00 Tz"),
            "the trailing-zero spelling must survive the restore verbatim — a renormalized \
             `90 Tz` would be a diff in an operand pdfce did not logically change: {appended}"
        );
        assert!(out.report.restore_narrowed.is_empty());
        // The EDITED run runs at the new value…
        assert_eq!(
            ambient_after_reload(&out.bytes, "hello").h_scale.value,
            75.0
        );
        // …and a reader agrees the ambient came back for the next one.
        assert_eq!(ambient_after_reload(&out.bytes, "tail").h_scale.value, 90.0);
    }

    /// Tier 3 (Amendment A.1's fourth rung): the ambient `Tc` was set as a
    /// SIDE EFFECT of `"`, whose bytes also SHOW A STRING (§9.4.3 Table
    /// 109). Replaying them as a restore would paint the text twice, so the
    /// value is re-spelled as `Tc` and the narrowing is disclosed.
    #[test]
    fn rung_three_indirect_ambient_is_respelled_and_disclosed() {
        let src = fill_pdf("BT /F1 12 Tf 14 TL 72 700 Td 2 0.25 (lead) \" (hello) Tj ET\n");
        let doc = Document::from_bytes(src.clone()).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello").char_spacing(MetricSpec::Absolute(0.9)),
            &FormatOptions::default(),
        )
        .unwrap();

        assert_eq!(
            out.report.restore_narrowed,
            vec![TextStateParam::CharSpacing],
            "an ObservedIndirect restore must be reported as narrowed"
        );
        let text = as_text(&out.bytes);
        assert!(text.contains("0.9 Tc"), "set emitted: {text}");
        assert!(
            text.contains("0.25 Tc"),
            "the `\"` operand must come back RE-SPELLED as its own operator: {text}"
        );
        // The `"` must appear exactly ONCE in the APPENDED revision — a
        // tier-2-style restore that replayed the ambient's raw bytes would
        // have shown "lead" a second time. This is the trap Amendment A.1
        // added the fourth rung for.
        let appended = as_text(&out.bytes[src.len()..]);
        assert_eq!(
            appended.matches("(lead) \"").count(),
            1,
            "the show string must not be repainted by the restore: {appended}"
        );
        assert!(
            out.report
                .disclosures
                .iter()
                .any(|d| d.contains("restore NARROWING")),
            "the re-spelling must be disclosed, not silent"
        );
    }

    /// Tier 4: the ambient was inherited from the context that invoked a
    /// form XObject (§8.10.1), so no restore can be written into the buffer
    /// being edited. pdfce REFUSES rather than emitting a guessed `0 Tc`,
    /// which would silently change content it did not touch.
    ///
    /// Exercised against the planner helper directly, and deliberately so:
    /// `text_edit::edit::Walk` does not descend into form XObjects at all
    /// (only the extraction walk does), so a run inside a form is never
    /// located as a format anchor in the first place and the tier is
    /// unreachable end-to-end today. Manufacturing a fake trigger to make an
    /// end-to-end test possible would test the fake; testing the ladder at
    /// its real application point tests the code that will run the day the
    /// authoring walk does descend.
    #[test]
    fn rung_four_form_xobject_ambient_refuses_rather_than_guessing() {
        let mut ambient = AmbientTextState::initial();
        ambient.apply_operator(b"Tc", &[0.5], b"0.5 Tc");
        ambient.enter_form(Some(12));

        let mut set_ops = Vec::new();
        let mut restore_ops = Vec::new();
        let mut narrowed = Vec::new();
        let mut emitted = Vec::new();
        let err = push_state_param(
            &mut set_ops,
            &mut restore_ops,
            &ambient,
            TextStateParam::CharSpacing,
            0.9,
            &mut narrowed,
            &mut emitted,
        )
        .unwrap_err();

        match err {
            FormatError::AmbientUnrestorable(inner) => {
                let msg = inner.to_string();
                assert!(msg.contains("character spacing"), "{msg}");
                assert!(msg.contains("form XObject 12"), "{msg}");
            }
            other => panic!("expected an unrestorable-ambient refusal, got {other:?}"),
        }
        assert!(
            set_ops.is_empty() && restore_ops.is_empty() && emitted.is_empty(),
            "a refusal must leave NOTHING partially applied (rule 4)"
        );

        // A parameter no operator ever set stays restorable inside the form:
        // its Table 105 default is provably in force there too.
        let mut ok_set = Vec::new();
        let mut ok_restore = Vec::new();
        push_state_param(
            &mut ok_set,
            &mut ok_restore,
            &ambient,
            TextStateParam::Rise,
            4.0,
            &mut narrowed,
            &mut emitted,
        )
        .expect("a never-set parameter is restorable even inside a form");
        assert_eq!(ok_restore, b"0 Ts");
    }

    /// R89: a relative `Tc` is stored as a ratio and the OPERAND is derived
    /// at emit time, so the same request at two sizes yields two operands in
    /// the same proportion. Without this, tracking dialled in at 12 pt is a
    /// different amount of tracking at 24 pt — `Tc` is in unscaled text
    /// space units and is NOT scaled by `Tfs` (§9.3, Table 105).
    #[test]
    fn r89_relative_char_spacing_is_derived_from_the_font_size() {
        for (size, expected) in [(12.0_f64, 0.24_f64), (24.0, 0.48)] {
            let src = fill_pdf(&format!("BT /F1 {size} Tf 72 700 Td (hello) Tj ET\n"));
            let doc = Document::from_bytes(src).unwrap();
            let out = set_format(
                &doc,
                &FormatRequest::new(0, "hello").char_spacing(MetricSpec::Relative(20.0)),
                &FormatOptions::default(),
            )
            .unwrap();
            let (_, emitted) = out.report.char_spacing_change.unwrap();
            assert!(
                (emitted - expected).abs() < 1e-12,
                "20 per-mille at {size} pt should be {expected}, got {emitted}"
            );
        }
    }

    /// R89 for the script toggle: a superscript is a RATIO of the base size,
    /// so re-applying it after a resize puts the baseline at the same
    /// PROPORTIONAL height rather than leaving a stale absolute rise.
    #[test]
    fn r89_superscript_rise_and_size_re_derive_after_a_resize() {
        let src = fill_pdf("BT /F1 10 Tf 72 700 Td (hello) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();

        // At the run's own 10 pt.
        let at_10 = set_format(
            &doc,
            &FormatRequest::new(0, "hello").script(ScriptPosition::Superscript),
            &FormatOptions::default(),
        )
        .unwrap();
        // Note the EXACT expected operands: these are the values after
        // `derived_operand` rounding, which is what actually reaches the
        // file. `10.0 * 0.34` is `3.4000000000000004` in `f64`, and writing
        // sixteen digits of representation noise into a content stream is a
        // minimal-diff defect, not a precision feature.
        assert_eq!(at_10.report.rise_change, Some((0.0, 3.4)));
        assert_eq!(at_10.report.script_size, Some((10.0, 6.0)));

        // Resized to 20 pt in the SAME command: both derive from the new
        // base size, not from the old one.
        let at_20 = set_format(
            &doc,
            &FormatRequest::new(0, "hello")
                .size(20.0)
                .script(ScriptPosition::Superscript),
            &FormatOptions::default(),
        )
        .unwrap();
        assert_eq!(at_20.report.rise_change, Some((0.0, 6.8)));
        assert_eq!(at_20.report.script_size, Some((20.0, 12.0)));

        // The proportion is what is preserved — that is the whole point.
        let (_, r10) = at_10.report.rise_change.unwrap();
        let (_, r20) = at_20.report.rise_change.unwrap();
        assert!((r20 / 20.0 - r10 / 10.0).abs() < 1e-12);
    }

    /// A superscript emits BOTH operators — the rise and the size reduction
    /// — and both survive a save + reload.
    #[test]
    fn superscript_emits_rise_and_reduced_size_and_round_trips() {
        let src = fill_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello").script(ScriptPosition::Superscript),
            &FormatOptions::default(),
        )
        .unwrap();

        let text = as_text(&out.bytes);
        assert!(text.contains("4.08 Ts"), "0.34 x 12 pt rise: {text}");
        assert!(text.contains("/F1 7.2 Tf"), "0.60 x 12 pt size: {text}");
        assert!(text.contains("0 Ts"), "rise restored: {text}");
        assert!(text.contains("/F1 12 Tf"), "size restored: {text}");

        // A reader sees the run raised…
        let at_run = ambient_after_reload(&out.bytes, "hello");
        assert_eq!(at_run.rise.value, 4.08);
        // …and the reduced size shortened the run, so ΔA is negative.
        assert!(out.report.advance_delta < 0.0);
        assert_eq!(out.report.script, Some(ScriptPosition::Superscript));
    }

    /// The subscript drop is DELIBERATELY shallower than the superscript
    /// rise (0.18 vs 0.34) — a lowered glyph runs into its own line's
    /// descenders almost immediately, while a raised one has the ascender
    /// band to move into. Asserted so the asymmetry cannot be "tidied" into
    /// symmetry by a later reader who assumes it was an oversight.
    #[test]
    fn subscript_drops_the_baseline_less_than_superscript_raises_it() {
        let src = fill_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello").script(ScriptPosition::Subscript),
            &FormatOptions::default(),
        )
        .unwrap();
        let (_, rise) = out.report.rise_change.unwrap();
        assert!((rise - (-12.0 * 0.18)).abs() < 1e-12, "got {rise}");
        assert!(rise < 0.0, "subscript moves the baseline DOWN (§9.3.7)");
        assert!(
            SUBSCRIPT.rise_ratio.abs() < SUPERSCRIPT.rise_ratio,
            "the drop must stay shallower than the rise"
        );
        assert_eq!(SUBSCRIPT.size_factor, SUPERSCRIPT.size_factor);
    }

    /// `--no-script` flattens an INHERITED rise for one run: it emits the
    /// spec default and restores the ambient. This is the only way to undo a
    /// producer's rise until free-form `Ts` ships (19.2).
    #[test]
    fn no_script_flattens_an_inherited_rise_and_restores_it() {
        let src = fill_pdf("BT /F1 12 Tf 4 Ts 72 700 Td (hello) Tj (tail) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello").script(ScriptPosition::Normal),
            &FormatOptions::default(),
        )
        .unwrap();
        assert_eq!(out.report.rise_change, Some((4.0, 0.0)));
        assert!(
            out.report.script_size.is_none(),
            "Normal does not resize the run"
        );
        // The run itself is flat; the run AFTER it is still raised.
        assert_eq!(ambient_after_reload(&out.bytes, "hello").rise.value, 0.0);
        assert_eq!(ambient_after_reload(&out.bytes, "tail").rise.value, 4.0);
    }

    /// Minimal-diff (`ARCHITECTURE.md` §5): a requested value that is
    /// ALREADY in force emits neither a set nor a restore. `--h-scale 100`
    /// on an unscaled run must not add `100 Tz … 100 Tz` to the stream.
    #[test]
    fn a_value_already_in_force_emits_no_operator_at_all() {
        let src = fill_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n");
        let doc = Document::from_bytes(src.clone()).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello").h_scale(100.0),
            &FormatOptions::default(),
        )
        .unwrap();
        let appended = as_text(&out.bytes[src.len()..]);
        assert!(
            !appended.contains("Tz"),
            "no Tz should be written when 100 is already in force: {appended}"
        );
        // The request is still reported — the operator asked, so the report
        // answers — and the advance is untouched.
        assert_eq!(out.report.h_scale_change, Some((100.0, 100.0)));
        assert_eq!(out.report.advance_delta, 0.0);
    }

    /// A `Tz` change inside a line whose show operator carries `TJ`
    /// adjustments — the shape justified slack takes — must be DISCLOSED
    /// with a re-justify offer, never silently left mis-aligned.
    #[test]
    fn tz_on_a_justified_line_discloses_and_offers_re_justify() {
        // A justified line as `reflow_apply` emits one: words separated by
        // negative TJ slack numbers.
        let src = fill_pdf("BT /F1 12 Tf 72 700 Td [(hello) -220 (wide) -220 (world)] TJ ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello").h_scale(80.0),
            &FormatOptions::default(),
        )
        .unwrap();

        assert!(out.report.justify_slack_invalidated);
        let d = out
            .report
            .disclosures
            .iter()
            .find(|d| d.contains("JUSTIFY invalidated"))
            .expect("the justify interaction must be disclosed");
        assert!(d.contains("reflow"), "a remedy must be offered: {d}");
        assert!(
            d.contains("--align justified"),
            "the remedy must be actionable: {d}"
        );

        // The surviving slack numbers are OUTSIDE the restored wrap, so they
        // are still there byte-for-byte — the disclosure says exactly this,
        // and it is the point on which decision 019's stated mechanism was
        // imprecise for this module's emission shape.
        let text = as_text(&out.bytes);
        assert_eq!(text.matches("-220").count(), 4, "2 original + 2 re-emitted");
    }

    /// A spacing/scaling change on a line with NO adjustments is not a
    /// justify problem and must not cry wolf.
    #[test]
    fn a_plain_line_does_not_claim_justification_was_invalidated() {
        let src = fill_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello").h_scale(80.0),
            &FormatOptions::default(),
        )
        .unwrap();
        assert!(!out.report.justify_slack_invalidated);
        assert!(
            !out.report
                .disclosures
                .iter()
                .any(|d| d.contains("JUSTIFY invalidated"))
        );
    }

    /// §9.4.4's advance is `((w0 − Tj/1000)·Tfs + Tc + Tw)·Th`, so a `Tc` or
    /// `Tz` change moves ΔA and therefore moves the followers. Before 19.1
    /// both sides of the delta used the AMBIENT values, which was correct
    /// only because neither could change; asserting the sign here is what
    /// keeps that from silently regressing.
    #[test]
    fn spacing_and_scaling_enter_the_advance_delta() {
        let src = fill_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();

        // Positive Tc widens every glyph's advance ⇒ ΔA > 0.
        let wider = set_format(
            &doc,
            &FormatRequest::new(0, "hello").char_spacing(MetricSpec::Absolute(0.5)),
            &FormatOptions::default(),
        )
        .unwrap();
        assert!(
            wider.report.advance_delta > 0.0,
            "{:?}",
            wider.report.advance_delta
        );

        // Th < 1 compresses the whole displacement ⇒ ΔA < 0.
        let narrower = set_format(
            &doc,
            &FormatRequest::new(0, "hello").h_scale(50.0),
            &FormatOptions::default(),
        )
        .unwrap();
        assert!(narrower.report.advance_delta < 0.0);
    }

    /// `Tz` is a percentage of normal width (§9.3.4); 0 would collapse the
    /// run to zero width and a negative would mirror it. Neither is a
    /// formatting operation, so both are refused BY NAME rather than
    /// silently clamped.
    #[test]
    fn a_collapsing_or_mirroring_h_scale_is_refused_by_name() {
        let src = fill_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        for bad in [0.0, -50.0] {
            let err = set_format(
                &doc,
                &FormatRequest::new(0, "hello").h_scale(bad),
                &FormatOptions::default(),
            )
            .unwrap_err();
            match err {
                FormatError::BadHorizScale(msg) => {
                    assert!(msg.contains("§9.3.4"), "cite the clause: {msg}");
                }
                other => panic!("expected BadHorizScale for {bad}, got {other:?}"),
            }
        }
    }

    /// All three controls at once, through a save + reload: the edited run
    /// carries every new value and the untouched neighbours carry none.
    #[test]
    fn all_three_controls_round_trip_through_save_and_reload() {
        let src = fill_pdf(
            "BT /F1 12 Tf 0.25 Tc 110 Tz 72 700 Td (before) Tj (hello) Tj (after) Tj ET\n",
        );
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello")
                .char_spacing(MetricSpec::Absolute(0.75))
                .h_scale(85.0)
                .script(ScriptPosition::Subscript),
            &FormatOptions::default(),
        )
        .unwrap();

        let edited = ambient_after_reload(&out.bytes, "hello");
        assert_eq!(edited.char_spacing.value, 0.75);
        assert_eq!(edited.h_scale.value, 85.0);
        assert!((edited.rise.value - (-12.0 * 0.18)).abs() < 1e-12);

        for neighbour in ["before", "after"] {
            let ts = ambient_after_reload(&out.bytes, neighbour);
            assert_eq!(ts.char_spacing.value, 0.25, "{neighbour}: Tc not restored");
            assert_eq!(ts.h_scale.value, 110.0, "{neighbour}: Tz not restored");
            assert_eq!(ts.rise.value, 0.0, "{neighbour}: Ts not restored");
        }
    }

    /// A no-op request is still a no-op: none of the three new controls
    /// makes an empty request non-empty on its own absence.
    #[test]
    fn the_new_controls_count_as_formatting_operations() {
        let src = fill_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        assert!(matches!(
            set_format(
                &doc,
                &FormatRequest::new(0, "hello"),
                &FormatOptions::default()
            ),
            Err(FormatError::NoOp)
        ));
        // …but each new control alone is enough to be a real request.
        for req in [
            FormatRequest::new(0, "hello").char_spacing(MetricSpec::Absolute(0.1)),
            FormatRequest::new(0, "hello").h_scale(90.0),
            FormatRequest::new(0, "hello").script(ScriptPosition::Superscript),
        ] {
            set_format(&doc, &req, &FormatOptions::default())
                .expect("each 19.1 control is a formatting operation on its own");
        }
    }
}
