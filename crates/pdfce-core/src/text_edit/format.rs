//! # In-place text FORMATTING surgery (Pass 14.2, extended by Pass 19.1–19.4)
//!
//! This module changes the **size**, **fill colour**, **font-family/style**,
//! **character spacing**, **horizontal scaling**, **baseline position** and
//! **word spacing** of text already on a page — in place, minimal-diff — by
//! reusing Pass 14.1's advance-preserving content-stream surgery
//! (`crate::text_edit::edit`) with different operators changed. It is the
//! third slice of decision 014's Acrobat-text-editing family
//! (`docs/decisions/014-acrobat-text-editing.md` §3 "Formatting", §5.2's
//! "13.2"; R32/R34/R47/R69/R70/R72) plus decision 019's slices 19.1–19.4
//! (R88/R89/R90/R91).
//!
//! ## The seven operations, and what each rewrites
//!
//! | Op | Operator changed | Advance affected? | Font gate? |
//! |---|---|---|---|
//! | **size** | `Tf` size operand | yes (ΔA) | no — never needs new glyphs |
//! | **fill colour** | `rg`/`g`/`k` (the chosen space) | no | no |
//! | **font family/style** | `Tf` font resource | yes (ΔA) | YES — re-encode + coverage gate |
//! | **character spacing** (19.1) | `Tc` (§9.3.2) | yes — `Tc` is a term of §9.4.4 | no |
//! | **horizontal scaling** (19.1) | `Tz` (§9.3.4) | yes — `Th` multiplies the whole displacement | no |
//! | **super/subscript** (19.1) | `Ts` (§9.3.7) + `Tf` size | yes, via the size reduction only | no |
//! | **word spacing** (19.4) | `Tw` (§9.3.3) | yes — but only through code-32 glyphs | **YES — simple fonts only (R91)** |
//!
//! ## Pass 19.1/19.2: the parity controls and the deliberate exceed
//!
//! Decision 019 §3.1 established from a named Adobe source that current
//! Acrobat retains exactly **character spacing, horizontal scaling and a
//! coarse superscript/subscript toggle** in this family. 19.2 added the
//! deliberate exceed — free-form numeric `Ts` plus synthetic bold/italic.
//! This slice emits `Ts` at the two **fixed, derived** values of the
//! super/subscript toggle *and* at an operator-supplied number.
//!
//! ## Pass 19.4: `Tw`, and why it is the one control with a capability gate
//!
//! Decision 019 §3.3 deliberately withheld a word-spacing control behind a
//! **corpus census**, on the hypothesis that composite (Type0/Identity-H)
//! embedding had made `Tw` inert on most real documents. That census has
//! now been run (decision 019 Amendment E): over the 4,012-file corpus,
//! `Tw` is reachable on **91.6% of show operators** and 97.4% of glyphs,
//! and **81.2% of text-bearing documents contain no composite run at all**
//! — which clears §3.3's pre-declared BUILD band (≥60%) and *falsifies*
//! the "large … share" half of the composite-default premise on that
//! corpus. (The "growing" half remains untested: the corpus is drawn from
//! tooling conformance suites that predate modern producers, so nothing
//! here supports a claim about a trend in either direction.)
//!
//! What the census did **not** change is the spec (§9.3.3): word spacing
//! "shall be applied to every occurrence of the single-byte character code
//! 32 … It shall not apply to occurrences of the byte value 32 in
//! multiple-byte codes." So `Tw` is **structurally void for composite
//! runs**, and standing rule **R91** makes that a refusal, not a no-op:
//! [`plan_format`] refuses a word-spacing request on a composite run
//! **by name** ([`FormatError::WordSpacingComposite`]) before anything is
//! planned. Emitting a `Tw` there would write an operator that provably
//! does nothing while the report claimed it had been applied — the exact
//! silent-failure shape rule 4 exists to forbid.
//!
//! ### Three things about `Tw` an operator must be told, and is
//!
//! 1. **It hits EVERY code-32 in the run** — leading, trailing and doubled
//!    spaces included. There is no per-gap control; that is `TJ`'s job, and
//!    it is exactly why decision 015 §3.1 chose `TJ` over `Tw` for
//!    justification slack. [`disclosure_word_spacing`] states this
//!    verbatim, because an operator reaching for "the gap between these two
//!    words" will otherwise be surprised.
//! 2. **It multiplies with `Th`** (§9.3.4/§9.4.4:
//!    `tx = ((w0 − Tj/1000)·Tfs + Tc + Tw)·Th`), so a `Tw` set under a
//!    `50 Tz` context delivers half the visible gap the number suggests.
//! 3. **It invalidates a justified line's slack**, exactly as `Tz` does and
//!    for the same reason (the run's rendered width moved by ΔA while the
//!    `TJ` slack numbers stayed put). It reuses 19.1's
//!    disclose-and-offer-re-justify path —
//!    [`disclosure_justify_invalidated`] — rather than inventing a second
//!    one.
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
    EditError, EditRequest, EditTarget, FillState, FollowerDisposition, FontClass, MatchRun, OpRec,
    Rec, ShowData, ShowElem, ShowOp, Walk, carried_codes, classify_font, compensating_tj,
    emit_show, emit_tm, find_anchor, glyph_advance_with, is_subset_tag, mat_mul, match_run,
    resolve_font_dict, splice, trust_disclosure, write_incremental,
};
use crate::text_edit::encoding::{InverseEncoding, RInvTrigger, Refusal};
use crate::text_edit::synth::{
    BOLD_STROKE_RATIO, StyleSynthesis, SynthesisOffer, SynthesisPath, bold_stroke_width,
    matrix_scale, name_claims_bold, name_claims_italic, shear_into,
};
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
    /// New **word** spacing `Tw` (§9.3.3), in the operator's own unit
    /// (R89) — Pass 19.4, the final FF-H control.
    ///
    /// # The capability gate that makes this different from every sibling
    ///
    /// `Tw` is the only member of this family that is **spec-void for some
    /// runs**. §9.3.3: word spacing "shall be applied to every occurrence
    /// of the single-byte character code 32 in a string when using a
    /// simple font or a composite font that defines code 32 as a
    /// single-byte code. It shall not apply to occurrences of the byte
    /// value 32 in multiple-byte codes." A `Tw` emitted over a 2-byte
    /// composite run therefore changes nothing at all.
    ///
    /// pdfce does not emit it there and does not pretend to: a
    /// word-spacing request against a composite run is
    /// [`FormatError::WordSpacingComposite`], refused **by name** with
    /// nothing applied (standing rule **R91**). That is deliberately
    /// louder than a silent no-op — a control that appears to work and
    /// does nothing is the failure mode rule 4 exists to forbid.
    ///
    /// # What it affects, which is more than most operators expect
    ///
    /// **Every** code-32 in the formatted run: leading spaces, trailing
    /// spaces, and both halves of a doubled space. `Tw` has no notion of
    /// "the gap between these two words" — per-gap control is `TJ`'s, and
    /// that is precisely why decision 015 §3.1 chose `TJ` over `Tw` for
    /// justification slack. It also multiplies with `Th` (§9.4.4), so its
    /// visible effect is scaled by any horizontal scaling in force.
    pub set_word_spacing: Option<MetricSpec>,
    /// New horizontal scaling `Tz` as a **percentage** of normal width
    /// (§9.3.4; 100 = normal). `None` leaves the run at its ambient `Tz`.
    pub set_h_scale: Option<f64>,
    /// New baseline position — the coarse superscript/subscript toggle.
    /// `None` leaves both the run's rise and its size alone.
    pub set_script: Option<ScriptPosition>,
    /// New **free-form** baseline rise `Ts` (§9.3.7), in the operator's own
    /// unit (R89) — Pass 19.2's deliberate exceed over Acrobat, which
    /// dropped free-form baseline offset when text editing was consolidated
    /// (decision 019 §1.1/§3.2).
    ///
    /// [`MetricSpec::Absolute`] is the default and is written **as typed**:
    /// a rise is in unscaled text-space units, so what the operator asked
    /// for is what the file gets. [`MetricSpec::Relative`] opts the value
    /// into R89's re-derivation, resolving against the run's **base** size
    /// so a later resize moves the rise proportionally.
    ///
    /// Mutually exclusive with [`Self::set_script`]: both write `Ts`, and
    /// silently letting one win would be a rule-4 failure. Requesting both
    /// is [`FormatError::ConflictingRise`].
    pub set_rise: Option<MetricSpec>,
    /// Apply synthetic bold and/or italic to the run (R90) — a **fallback**
    /// for when no real Bold/Italic face resolves, never an alternative to
    /// one.
    ///
    /// Setting this is the operator's explicit, per-use acceptance of the
    /// offer; there is no global preference and nothing is ever applied
    /// silently (deliberately stricter than Acrobat's set-and-forget
    /// "Enable Artificial Bold/Italic styles"). If a real face *does*
    /// resolve on the page, the request is **refused** by name and pointed
    /// at that face ([`FormatError::RealFaceAvailable`]).
    pub set_synthetic: Option<StyleSynthesis>,
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
            set_word_spacing: None,
            set_h_scale: None,
            set_script: None,
            set_rise: None,
            set_synthetic: None,
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

    /// Add a word-spacing (`Tw`) change, returning `self` (Pass 19.4).
    ///
    /// Refused on a composite run — see [`Self::set_word_spacing`] for the
    /// §9.3.3 reason and R91.
    #[must_use]
    pub fn word_spacing(mut self, spec: MetricSpec) -> Self {
        self.set_word_spacing = Some(spec);
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

    /// Add a free-form baseline rise (`Ts`), returning `self` (Pass 19.2).
    #[must_use]
    pub fn rise(mut self, spec: MetricSpec) -> Self {
        self.set_rise = Some(spec);
        self
    }

    /// Request synthetic bold and/or italic, returning `self` (Pass 19.2).
    #[must_use]
    pub fn synthetic(mut self, synthesis: StyleSynthesis) -> Self {
        self.set_synthetic = Some(synthesis);
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
            && self.set_word_spacing.is_none()
            && self.set_h_scale.is_none()
            && self.set_script.is_none()
            && self.set_rise.is_none()
            // Spelled as a `match` rather than `is_none_or` because this
            // predicate is `const` (one definition, two callers — see the
            // doc comment) and `Option::is_none_or` is not yet const.
            && match self.set_synthetic {
                None => true,
                Some(s) => s.is_none(),
            }
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
    /// `(ambient operand, emitted operand)` for `Tw` when word spacing was
    /// requested (Pass 19.4). Present even when the two are equal and
    /// nothing was therefore emitted — same contract as
    /// [`Self::char_spacing_change`]: the operator asked, so the report
    /// answers.
    ///
    /// Never `Some` on a composite run: that request is refused outright
    /// (R91, [`FormatError::WordSpacingComposite`]) and no report exists.
    pub word_spacing_change: Option<(f64, f64)>,
    /// How many single-byte code-32 occurrences the emitted `Tw` actually
    /// applies to inside the formatted run (§9.3.3) — quoted so the
    /// "it hits EVERY space, not the one you meant" property is a number
    /// the operator can see rather than a sentence they might skim.
    ///
    /// **`Some(0)` is a real and important answer**: a run with no spaces
    /// takes a `Tw` that changes nothing visible. pdfce still emits and
    /// restores it (the operator asked for a state change and gets exactly
    /// that), and says so. `None` when no word spacing was requested.
    pub word_spacing_affected_codes: Option<usize>,
    /// `(ambient operand, emitted operand)` for `Tz` (percentages) when
    /// horizontal scaling was requested.
    pub h_scale_change: Option<(f64, f64)>,
    /// The baseline position applied, when the script toggle was used.
    pub script: Option<ScriptPosition>,
    /// `(base size, emitted reduced size)` when a super/subscript reduced
    /// the run's `Tf` size. `None` for [`ScriptPosition::Normal`], which
    /// does not resize.
    pub script_size: Option<(f64, f64)>,
    /// `(ambient Ts, emitted Ts)` when the script toggle **or** the Pass
    /// 19.2 free-form rise changed the baseline offset.
    pub rise_change: Option<(f64, f64)>,
    /// What was synthesized, if anything (Pass 19.2, R90).
    ///
    /// This is the **in-session** half of P-selfevident persistence: nothing
    /// is written into the PDF to record it. The saved bytes are
    /// re-detectable on their own — see
    /// [`synth::detect`](crate::text_edit::synth::detect).
    pub synthesis: StyleSynthesis,
    /// The user-space stroke width a synthetic bold emitted (§9.3.6), quoted
    /// by value so the number is never hidden from the operator (rule 4).
    pub synthetic_bold_width: Option<f64>,
    /// The `Tm` shear term a synthetic italic emitted, and the horizontal
    /// displacement `Trise · tan θ` it imposes on the run **because** the
    /// run is raised — decision 019 §3.6's named `Ts` × oblique interaction.
    /// The second element is `0.0` for a run at the baseline, which is the
    /// usual case and the reason the interaction surprises people.
    pub synthetic_italic: Option<(f64, f64)>,
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
         spacing, word spacing, horizontal scaling, super/subscript, baseline rise, or a \
         synthetic style)"
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
    /// Word spacing was requested for a run shown in a **composite**
    /// (Type 0 / CIDFont) font, where `Tw` is spec-void. Nothing was
    /// applied. Standing rule **R91**.
    ///
    /// This is a refusal rather than a silent skip on purpose. `Tw` over a
    /// 2-byte run is not "a change with no visible effect" — it is an
    /// operator the spec says shall not be applied at all, so writing one
    /// would add bytes to the file, claim success in the report, and
    /// change nothing. Refusing by name, and naming the font that caused
    /// it, is the only outcome that leaves the operator knowing what
    /// happened (rule 4).
    #[error(
        "word spacing (Tw) cannot be applied to this run: '{base_font}' is a COMPOSITE (Type 0 / \
         CIDFont) font, whose show strings use multi-byte codes. ISO 32000-1 §9.3.3 applies word \
         spacing only to the SINGLE-BYTE character code 32 and states it \"shall not apply to \
         occurrences of the byte value 32 in multiple-byte codes\" — so a Tw here would be written \
         into the file and do nothing. Nothing was applied (rule R91). To change inter-word \
         spacing on a composite run, distribute it as TJ adjustments instead: reflow the paragraph."
    )]
    WordSpacingComposite {
        /// The run's `/BaseFont`, so the refusal names the actual font.
        base_font: String,
    },
    /// Both a free-form rise and a super/subscript toggle were requested.
    /// Both write `Ts`; pdfce refuses rather than silently picking one.
    #[error(
        "a free-form baseline rise and a super/subscript toggle were both requested, and both \
         set the same operator (`Ts`, §9.3.7). Ask for one of them: the toggle applies pdfce's \
         documented script metrics (rise AND a size reduction), the free-form rise applies \
         exactly the number given and does not resize."
    )]
    ConflictingRise,
    /// Synthetic bold/italic was requested but a **real** face with that
    /// style resolves on the page. R90 makes synthesis fallback-only, so
    /// pdfce refuses and names the real face to use instead. Nothing was
    /// applied.
    #[error(
        "synthetic {style} was requested for '{run_font}', but a REAL {style} face is available \
         on this page as '{real_font}' (resource /{resource}). Synthesis is a fallback for when \
         no real face resolves, never an alternative to one (rule R90) — change the run's family \
         to '{real_font}' instead. Nothing was applied."
    )]
    RealFaceAvailable {
        /// The style asked for, e.g. `bold`.
        style: &'static str,
        /// The run's current `/BaseFont`.
        run_font: String,
        /// The `/BaseFont` of the real face that resolves.
        real_font: String,
        /// Its `/Font` resource key.
        resource: String,
    },
    /// Synthetic italic cannot be applied to this run, because the shear it
    /// requires is a `Tm` injection and this run's context will not survive
    /// one. Nothing was applied. See the variant payload for which of the
    /// three named conditions fired.
    #[error("synthetic italic cannot be applied to this run: {0}")]
    ShearUnsupported(String),
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
    /// A **pinned** request named a byte span that matches no show operator
    /// (`Pass 118.0`) — the format-side twin of
    /// [`EditError::PinnedSpanNotFound`], carried through rather than
    /// collapsed into [`Self::NoMatch`].
    ///
    /// It is carried because the format path is the one the shipped property
    /// bar drives, and it is where the `Pass 19.3` incident this variant
    /// exists to prevent was originally observed: *"Apply size refused with
    /// 'text to format (…) was not found in an editable run on the page' on a
    /// perfectly ordinary one-`Tj` page."* Collapsing it here would put that
    /// exact sentence back in front of the operator.
    #[error(
        "the pinned span {start}..{end} names no show operator in this content stream -- the text is not the problem; the pin is pointing at a different buffer (a form XObject's content is not the page's)"
    )]
    PinnedSpanNotFound {
        /// First byte of the pin, as supplied.
        start: usize,
        /// One past the last byte of the pin, as supplied.
        end: usize,
    },
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
            EditError::PinnedSpanNotFound { start, end } => Self::PinnedSpanNotFound { start, end },
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
        // `find_anchor` searches the recs it is handed, so the target selector
        // is never consulted on this path (`Pass 119.0`). Spelled out rather
        // than defaulted so a reader does not have to check.
        target: EditTarget::Auto,
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

    // --- resolve the run's OWN font (needed for A_old, and for size/colour
    //     the effective face) ---
    //
    // ORDERING NOTE (Pass 19.4, deliberate): this block sits BEFORE
    // `match_run`, where it used to sit after. The reason is the R91 gate
    // immediately below it. `Walk::record_show` does not decode a composite
    // run's string into text at all (`edit.rs`: "A composite font is not
    // decoded"), so `ShowData::text` is EMPTY for one — and `match_run`,
    // which searches that text, therefore fails with `NoMatch` on every
    // composite run before any font-aware gate could speak. Leaving the
    // order alone would have made the word-spacing refusal literally
    // unreachable, i.e. an untestable branch claiming to honour R91.
    //
    // The precedence change this introduces is small and strictly better:
    // a run whose font resource does not resolve now reports THAT rather
    // than "text not found" when the find text also misses.
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

    // --- R91: the `Tw` capability gate (Pass 19.4) ---
    //
    // §9.3.3 applies word spacing "to every occurrence of the single-byte
    // character code 32" and states it "shall not apply to occurrences of
    // the byte value 32 in multiple-byte codes". A composite run segments
    // its show strings into 2-byte codes (§9.7.6.2), so a `Tw` emitted for
    // one is inert by construction.
    //
    // Refused BY NAME, before anything is planned, and never silently
    // skipped: a skip would leave the report saying word spacing was
    // applied while the file's rendering was unchanged. The composite
    // predicate is `ExtractFont::is_simple` — the SAME call the extraction
    // walk uses to publish `GlyphProvenance::composite` (`page.rs`), so the
    // gate the GUI draws from provenance and the gate core enforces here
    // cannot disagree (R74: no capability rule re-derived in the shell).
    //
    // Gating on the run's OWN font rather than a family-change target is
    // correct and sufficient: `classify_font` already refuses a composite
    // TARGET (R-INV-4), so a surviving family change is simple→simple.
    if req.set_word_spacing.is_some() && !orig_font.is_simple() {
        return Err(FormatError::WordSpacingComposite {
            base_font: orig_font.base_font.clone(),
        });
    }

    // --- map the find text to a contiguous code range in one element ---
    let m = match_run(anchor, &req.find).map_err(FormatError::from_edit)?;

    // --- resolve the family-change target, if any, and re-encode the run ---
    let font_plan = plan_font(doc, page_resources(page), &recs, req)?;

    // --- the run's BASE size: what it would be shown at with no script
    //     reduction. Every R89 ratio (script rise, script size, relative
    //     `Tc`) is derived from THIS, so a later resize re-derives all of
    //     them consistently rather than carrying a stale operand. ---
    let base_size = req.set_size.unwrap_or(orig_size);

    // --- Pass 19.2: the free-form rise and the script toggle both write
    //     `Ts`, so asking for both is a conflict pdfce refuses rather than
    //     resolving silently (rule 4). Checked before anything is planned. ---
    if req.set_rise.is_some() && req.set_script.is_some() {
        return Err(FormatError::ConflictingRise);
    }

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

    // --- Pass 19.2: the free-form rise (decision 019 §3.2's deliberate
    //     exceed). It resolves through the SAME `MetricSpec` model as `Tc`
    //     and against the SAME base size (R89), so a run that is later
    //     resized moves a `Relative` rise proportionally and leaves an
    //     `Absolute` one exactly where the operator put it.
    //
    //     An ABSOLUTE spec is written as typed — deliberately NOT passed
    //     through `derived_operand`, because rounding a number a human typed
    //     is a silent modification (see that function's own docs). Only the
    //     RELATIVE case is pdfce's arithmetic and therefore rounded. ---
    let free_rise = req.set_rise.map(|spec| match spec {
        MetricSpec::Absolute(v) => v,
        MetricSpec::Relative(_) => derived_operand(spec.resolve(base_size)),
    });
    // Exactly one of the two can be present (the conflict was refused
    // above), so this is the single `Ts` operand for the whole planner.
    let new_rise = script_rise.or(free_rise);

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

    // --- resolve `Tw` (Pass 19.4) ---
    //
    // Identical unit model to `Tc`, and deliberately so: both are in
    // unscaled text-space units (§9.3, Table 105's closing note), both are
    // therefore R89 `MetricSpec` quantities, and both resolve against the
    // BASE size (Amendment B.3) so a script reduction does not shrink the
    // spaces relative to the text they sit between. One model, one parser,
    // one set of suffixes for an operator to learn.
    let new_tw = req.set_word_spacing.map(|spec| match spec {
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

    // The formatting path is SINGLE-BYTE by construction: it re-encodes into a
    // target simple font's encoding. `m.old_codes` widened to `u32` in Pass
    // 29.0 for the composite REPLACE path, so it is converted here rather than
    // widening this path too — a composite run never reaches formatting
    // (`classify_font` refuses it first), and pretending otherwise would put
    // two-byte codes into a single-byte emitter.
    let new_codes: Vec<u32> = match &font_plan {
        Some(plan) => plan.new_codes.iter().map(|&c| u32::from(c)).collect(),
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
    // Pass 19.4 makes `Tw` a THIRD term that can move: it is in the same
    // §9.4.4 sum, and §9.3.4 multiplies it by `Th` alongside `Tc` ("It
    // shall also affect the spacing parameters `Tc` and `Tw`"). Unlike
    // `Tc` it only reaches code-32 glyphs — `glyph_advance_with` applies
    // the §9.3.3 single-byte-32 rule itself, so the delta over a run with
    // no spaces is legitimately zero even though a `Tw` was emitted.
    //
    // `Ts` deliberately does not appear: rise is a `Trm` translation
    // (§9.3.7) and changes position, not advance.
    let eff_tc = new_tc.unwrap_or_else(|| anchor.tc());
    let eff_tw = new_tw.unwrap_or_else(|| anchor.tw());
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
                true,
            )
        })
        .sum();
    let a_new: f64 = new_codes
        .iter()
        .map(|&c| glyph_advance_with(advance_font, c, emitted_size, eff_tc, eff_tw, eff_th, true))
        .sum();
    let delta = a_new - a_old;

    // How many code-32s the emitted `Tw` will actually reach inside the
    // formatted run (§9.3.3). Counted on the codes that will BE SHOWN
    // (post-re-encode), because that is the string `Tw` operates on.
    // Reported by value — a `Some(0)` is the honest answer for a run with
    // no spaces, not a reason to suppress the operation.
    let tw_affected = new_tw.map(|_| new_codes.iter().filter(|&&c| c == 0x20).count());

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
    // `Tw` rides the identical ladder — including the `ObservedIndirect`
    // rung, which `Tw` is the *headline* case for: the `"` operator sets
    // `Tw` (and `Tc`) **while showing a string** (§9.4.3 Table 109), so a
    // restore that replayed the producer's `"` bytes would repaint the
    // text. Amendment A.1 added the rung for exactly this operator.
    if let Some(tw) = new_tw {
        push_state_param(
            &mut set_ops,
            &mut restore_ops,
            &anchor.text_state,
            TextStateParam::WordSpacing,
            tw,
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
    if let Some(rise) = new_rise {
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

    // --- Pass 19.2: synthetic bold / italic (R90, decision 019 §3.6) ---
    //
    // The gate runs FIRST and is fallback-only: if a real Bold/Italic face
    // resolves on this page, the request is refused and pointed at it. Only
    // then is anything emitted.
    let synthesis = req.set_synthetic.unwrap_or_default();
    let mut synthetic_bold_width: Option<f64> = None;
    let mut synthetic_italic: Option<(f64, f64)> = None;
    let mut synthesis_offer: Option<SynthesisOffer> = None;
    if !synthesis.is_none() {
        // The face the run will actually be shown in after this edit — a
        // family change is resolved before synthesis is considered, so
        // "change the family AND fake bold on top" is gated against the
        // TARGET's siblings, not the original's.
        let effective_font = font_plan
            .as_ref()
            .map_or(&orig_font.base_font, |p| &p.font.base_font);
        gate_synthesis(
            doc,
            page_resources(page),
            effective_font,
            synthesis,
            set_font_name,
        )?;
        synthesis_offer = Some(SynthesisOffer {
            synthesis,
            base_font: effective_font.clone(),
            path: SynthesisPath::InPlaceEdit,
        });

        if synthesis.bold() {
            let width = plan_synthetic_bold(
                &mut set_ops,
                &mut restore_ops,
                anchor,
                emitted_size,
                req.set_fill.as_ref(),
                &mut restore_narrowed,
                &mut emitted_state,
            )?;
            synthetic_bold_width = Some(width);
        }
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
    // Pass 19.4 adds `Tw` to the trigger set, and it belongs there for the
    // same reason `Tc` does — it is a term of the §9.4.4 advance, so it
    // moves the run's width. This is REUSE of 19.1's path, deliberately
    // not a second one: decision 015 §3.1 already established that `Tw`
    // and justification slack are two mechanisms competing for the same
    // inter-word space, and an operator who changes one must be told the
    // other is now stale.
    let justify_slack_invalidated = run_carries_tj_slack
        && delta != 0.0
        && (req.set_h_scale.is_some()
            || req.set_char_spacing.is_some()
            || req.set_word_spacing.is_some()
            || script.is_some());

    // --- re-emit the anchor as pre | set | mid | restore | post ---
    let (pre, post) = split_segments(anchor, &m);

    // --- Pass 19.2: synthetic italic, which is NOT a text-state change ---
    //
    // A shear is premultiplied into `Tm`, so unlike everything above it
    // cannot ride R88's ladder: `Tm` is not one of the six §9.3 parameters,
    // is not saved by `q`/`Q`, and — the trap decision 019 §3.6 names — is
    // *propagated* by `Td`/`TD`/`T*`, which derive the next line by
    // translating the line matrix rather than by re-stating it.
    //
    // pdfce therefore scopes the shear by re-emitting an **absolute `Tm`**
    // on both sides of the run: the sheared matrix before `mid`, the
    // original (unsheared) matrix at the run's end before `post`. That
    // closes the propagation into `post` and into any following show
    // operator — but it cannot close it into a following `Td`/`TD`/`T*`,
    // because any `Tm` overwrites `Tlm` as well and that operator would
    // then translate from pdfce's matrix instead of the producer's line
    // origin. That case is REFUSED by name below rather than mis-positioned
    // silently.
    if synthesis.italic() {
        let (tan, rise_offset) = plan_synthetic_italic(
            &mut set_ops,
            &mut restore_ops,
            anchor,
            &recs,
            anchor_index,
            opts.disposition,
            &pre,
            &orig_font,
            orig_size,
            a_new,
            new_rise,
        )?;
        synthetic_italic = Some((tan, rise_offset));
    }

    // Back to bytes for emission. Every code here fits by construction (the
    // single-byte re-encode above produced them), and `filter_map` keeps that
    // assumption from turning into a silent truncation if it ever stops
    // holding — a truncated code is a different, valid, wrong glyph.
    let mid_bytes: Vec<u8> = new_codes
        .iter()
        .filter_map(|&c| u8::try_from(c).ok())
        .collect();
    let mid = vec![ShowElem::Str(mid_bytes)];
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
    if let Some(spec) = req.set_word_spacing {
        disclosures.push(disclosure_word_spacing(
            spec,
            base_size,
            anchor.text_state.word_spacing.value,
            eff_tw,
            tw_affected.unwrap_or(0),
            eff_th,
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
    // --- Pass 19.2 disclosures ---
    if let Some(spec) = req.set_rise {
        disclosures.push(disclosure_rise(
            spec,
            base_size,
            anchor.text_state.rise.value,
            free_rise.unwrap_or(0.0),
        ));
    }
    if let Some(offer) = &synthesis_offer {
        disclosures.push(offer.disclosure());
    }
    if let Some(w) = synthetic_bold_width {
        disclosures.push(disclosure_synthetic_bold(w, emitted_size));
    }
    if let Some((tan, offset)) = synthetic_italic {
        disclosures.push(disclosure_synthetic_italic(tan, offset));
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
            req.set_word_spacing.is_some(),
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
        word_spacing_change: new_tw.map(|tw| (anchor.text_state.word_spacing.value, tw)),
        word_spacing_affected_codes: tw_affected,
        h_scale_change: req
            .set_h_scale
            .map(|pct| (anchor.text_state.h_scale.value, pct)),
        script,
        script_size: script_metrics.map(|_| (base_size, emitted_size)),
        rise_change: new_rise.map(|r| (anchor.text_state.rise.value, r)),
        synthesis,
        synthetic_bold_width,
        synthetic_italic,
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
            if !carried.contains(&u32::from(code)) {
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

// ===================================================================
// Pass 19.2 — synthetic bold / italic planning (R90)
// ===================================================================

/// The family stem of a `/BaseFont` name — the part that identifies the
/// typeface family rather than the style within it.
///
/// `ABCDEF+Times-BoldItalic` → `times`; `Arial,Bold` → `arial`;
/// `Helvetica` → `helvetica`. The §9.6.4 subset tag is stripped first, then
/// the name is cut at the first `-` or `,` (the two conventional
/// style-separator spellings, §9.6.2.2), and lowercased so the comparison is
/// case-insensitive.
///
/// This is a **heuristic on the name**, which is all `/BaseFont` offers
/// without parsing the embedded program. It is used only to decide whether a
/// *real* styled sibling exists on the page — i.e. only to make pdfce
/// **refuse** a synthesis and offer something better. Being wrong in the
/// conservative direction (failing to spot a sibling) costs a synthesis that
/// could have been a real face; being wrong the other way costs a refusal
/// the operator can override by naming the face directly with `--set-font`.
/// Neither silently produces bad output, which is what makes a heuristic
/// acceptable here at all.
fn family_stem(base_font: &str) -> String {
    let stem = subset_stem(base_font);
    let cut = stem.find(['-', ',']).unwrap_or(stem.len());
    stem.get(..cut).unwrap_or(stem).to_ascii_lowercase()
}

/// R90's gate: **refuse** synthesis when a real face with the requested
/// style is available on this page.
///
/// Synthesis is a fallback, not an alternative — so before faking a weight
/// or a slant, pdfce looks for a genuine sibling in the page's `/Font`
/// resources: same family stem, and a `/BaseFont` that claims the style
/// being asked for. If one is found the whole edit is refused and the
/// operator is told the resource key and name to use instead.
///
/// `current_resource` is excluded from the search so that a face which
/// *already* claims the style cannot recommend itself. (Asking for synthetic
/// bold on `Times-Bold` is a strange request, and it is refused with that
/// same face named — which reads correctly: the real bold is already there.)
///
/// # Errors
///
/// [`FormatError::RealFaceAvailable`], naming the resource and `/BaseFont`.
fn gate_synthesis(
    doc: &Document,
    resources: &Dict,
    run_font: &str,
    synthesis: StyleSynthesis,
    current_resource: &[u8],
) -> Result<(), FormatError> {
    let Some(fonts) = resources
        .get(b"Font")
        .map(|o| doc.resolve(o))
        .and_then(Object::as_dict)
    else {
        // No font resources to search: nothing better exists, so the
        // fallback is genuinely the only option. Proceed.
        return Ok(());
    };
    let want = family_stem(run_font);
    for (key, val) in fonts.iter() {
        let Some(dict) = doc.resolve(val).as_dict() else {
            continue;
        };
        let Some(base) = dict
            .get(b"BaseFont")
            .map(|o| doc.resolve(o))
            .and_then(Object::as_name)
            .map(|n| String::from_utf8_lossy(n.as_bytes()).into_owned())
        else {
            continue;
        };
        if family_stem(&base) != want {
            continue;
        }
        let is_self = key.as_bytes() == current_resource;
        let covers_bold = !synthesis.bold() || name_claims_bold(&base);
        let covers_italic = !synthesis.italic() || name_claims_italic(&base);
        // A real face only counts if it covers EVERY style asked for. A
        // `Times-Bold` does not satisfy a request for synthetic *italic*.
        if covers_bold && covers_italic && !is_self {
            return Err(FormatError::RealFaceAvailable {
                style: match synthesis {
                    StyleSynthesis::Bold => "bold",
                    StyleSynthesis::Italic => "italic",
                    StyleSynthesis::BoldItalic => "bold italic",
                    StyleSynthesis::None => "styled",
                },
                run_font: run_font.to_owned(),
                real_font: base,
                resource: String::from_utf8_lossy(key.as_bytes()).into_owned(),
            });
        }
    }
    Ok(())
}

// ===================================================================
// Pass 19.3 — the read-only pre-resolution query (ui-spec §1.1 Option B)
// ===================================================================

/// What [`gate_synthesis`] would decide for one style axis (or for a whole
/// combination), asked **without** submitting anything.
///
/// Deliberately the two outcomes the gate itself has, and nothing more: the
/// gate either finds a real face covering everything asked for (so a
/// `set_synthetic` would be refused) or it does not (so a `set_synthetic`
/// would apply). Inventing a third state here would let a preview promise
/// something the commit path cannot honour.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum StyleOutcome {
    /// A real face covering everything asked for resolves on this page.
    /// Submitting `set_synthetic(…)` right now would be **refused** with
    /// [`FormatError::RealFaceAvailable`] naming exactly these two strings.
    ///
    /// Note what this does **not** promise: pdfce will not switch to that
    /// face on the operator's behalf. The commit path refuses and points at
    /// it; changing the family is a separate, explicit act through the font
    /// control. Copy that implies otherwise misstates the mechanism.
    RealFaceResolves {
        /// The `/BaseFont` of the real face that resolves.
        real_font: String,
        /// Its `/Font` resource key on this page (no leading slash).
        resource: String,
    },
    /// No real face covers everything asked for, so submitting
    /// `set_synthetic(…)` right now would **apply** the synthesis.
    WouldSynthesize,
}

impl StyleOutcome {
    /// Whether a real face resolves — i.e. whether the gate would refuse.
    ///
    /// Named so callers do not have to pattern-match a `#[non_exhaustive]`
    /// enum just to ask a yes/no question.
    #[must_use]
    pub const fn is_real_face(&self) -> bool {
        matches!(self, Self::RealFaceResolves { .. })
    }
}

/// A **read-only** preview of what a [`StyleSynthesis`] request would do to
/// one run — computed without mutating anything and without duplicating the
/// gate.
///
/// # Why this exists (decision 019 §6 slice 19.3; ui-spec §1.1 "Option B")
///
/// Before this type, [`gate_synthesis`] was the *entire* mechanism deciding
/// whether a synthesis request succeeded, it was private, and nothing in the
/// public API answered "if I asked for Bold right now, would a real face
/// resolve or would pdfce synthesize?" ahead of actually submitting a
/// [`FormatRequest`]. A caller could therefore only learn the answer *after*
/// acting — the wrong side of rule 4 for a change that alters how the
/// operator's document renders. R90's own word for synthesis is
/// "declinable", and declining sensibly means knowing what is on offer
/// before the click, not after it.
///
/// # Why it carries per-axis probes and not one verdict
///
/// [`gate_synthesis`] is **all-or-nothing per combined request**: a candidate
/// face counts only if it covers *every* style asked for. So a page holding a
/// real `Arial-Bold` but no `Arial-BoldItalic` answers a Bold+Italic request
/// with "no real face — synthesize both", silently passing over an available
/// real Bold. That is shipped Pass 19.2 behaviour and this type does not
/// change it; what it does is make the situation **visible**, by carrying the
/// per-axis probes alongside the combined one so a caller can recognise the
/// mixed case ([`Self::is_mixed`]) and refuse it by name instead of walking
/// into it.
///
/// Composing a real-face family change for the covered axis with synthesis
/// for the uncovered one is a genuine, unscoped fast-follow (ui-spec §1.1's
/// "genuine wrinkle", §8 item 11) — deliberately **not** built here.
///
/// # Invariant (R74)
///
/// Every field is derived by calling [`gate_synthesis`] itself. No matching
/// rule ([`family_stem`], [`name_claims_bold`], [`name_claims_italic`]) is
/// re-derived here or — critically — in `pdfce-gui`: a GUI that
/// re-implemented the gate would lose it in the WASM fork and would drift
/// from the commit path the first time the heuristic changed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct StyleResolution {
    /// The combination that was asked about.
    pub want: StyleSynthesis,
    /// The run's own `/BaseFont`, for the caller's message.
    pub run_font: String,
    /// What the gate would decide for [`Self::want`] **as a whole** — i.e.
    /// exactly what `set_synthetic(want)` would do. `None` when `want` is
    /// [`StyleSynthesis::None`] (nothing was asked about).
    pub combined: Option<StyleOutcome>,
    /// The bold axis probed **alone**. `Some` only when `want.bold()`.
    pub bold_axis: Option<StyleOutcome>,
    /// The italic axis probed **alone**. `Some` only when `want.italic()`.
    pub italic_axis: Option<StyleOutcome>,
}

impl StyleResolution {
    /// The request mixes an axis that **has** a real face with one that does
    /// not — or has two real faces in different resources with no single one
    /// covering both.
    ///
    /// This is the case the all-or-nothing gate cannot answer honestly:
    /// synthesizing both discards a real face that exists, and synthesizing
    /// only the uncovered axis is work nobody has specified. A caller seeing
    /// `true` should **refuse and disclose**, naming which axis has the real
    /// face, rather than submit.
    #[must_use]
    pub fn is_mixed(&self) -> bool {
        // Only meaningful when the COMBINED probe says "would synthesize": if
        // one face covers everything, there is nothing mixed about it.
        if !matches!(self.combined, Some(StyleOutcome::WouldSynthesize)) {
            return false;
        }
        self.bold_axis
            .as_ref()
            .is_some_and(StyleOutcome::is_real_face)
            || self
                .italic_axis
                .as_ref()
                .is_some_and(StyleOutcome::is_real_face)
    }
}

/// Run [`gate_synthesis`] for one probe and translate its refusal into a
/// [`StyleOutcome`], leaving every other error a real error.
///
/// The whole point of routing *through* the gate rather than around it: the
/// preview cannot say something the commit path would not do, because it asks
/// the commit path's own function.
fn probe_synthesis(
    doc: &Document,
    resources: &Dict,
    run_font: &str,
    synthesis: StyleSynthesis,
    current_resource: &[u8],
) -> Result<StyleOutcome, FormatError> {
    match gate_synthesis(doc, resources, run_font, synthesis, current_resource) {
        Ok(()) => Ok(StyleOutcome::WouldSynthesize),
        Err(FormatError::RealFaceAvailable {
            real_font,
            resource,
            ..
        }) => Ok(StyleOutcome::RealFaceResolves {
            real_font,
            resource,
        }),
        Err(other) => Err(other),
    }
}

/// Preview what `want` would resolve to for the run located by `find` /
/// `pinned_span` on `page`, **without mutating anything**.
///
/// Side-effect-free by construction: it walks the already-decoded content
/// stream, locates the anchor exactly as [`plan_format`] does, resolves the
/// run's own font, then asks [`gate_synthesis`] up to three times. It writes
/// nothing, plans nothing, and touches no session state.
///
/// # What it deliberately does NOT model
///
/// A *pending* family change. The preview answers for the run **as it is
/// now**, because the property surface applies one control family per commit:
/// a font change is its own accepted edit, after which the caller rebuilds
/// its model and this query answers against the new face. Threading an
/// un-applied `set_font` through here would mean re-running [`plan_font`]'s
/// re-encoding work every frame to answer a question the operator has not
/// asked yet.
///
/// # Errors
///
/// The same location/resolution failures [`plan_format`] reports — no match,
/// an unsupported anchor, an unresolvable font resource, a content-parse
/// failure. [`FormatError::RealFaceAvailable`] is **never** returned: that
/// outcome is the point of the query and comes back as
/// [`StyleOutcome::RealFaceResolves`].
pub(crate) fn preview_style_resolution(
    doc: &Document,
    page: &crate::page_tree::Page,
    stream: &ContentStream,
    find: &str,
    pinned_span: Option<ByteSpan>,
    want: StyleSynthesis,
) -> Result<StyleResolution, FormatError> {
    let mut walk = Walk::new(doc, &page.resources);
    for op in stream.operations() {
        walk.operation(&op, &stream.buf);
    }
    let recs = walk.recs;

    // `find_anchor` matches within THESE recs, which are already this page's,
    // so the page index carried here is never read. Spelled 0 rather than
    // threaded through, so no caller can mistake it for a selector.
    let locate = EditRequest {
        page_index: 0,
        find: find.to_owned(),
        replace: String::new(),
        pinned_span,
        target: EditTarget::Auto,
    };
    let anchor_index = find_anchor(&recs, &locate).map_err(FormatError::from_edit)?;
    let anchor = match recs.get(anchor_index) {
        Some(OpRec {
            rec: Rec::Show(s), ..
        }) => s,
        _ => return Err(FormatError::NoMatch(find.to_owned())),
    };

    let orig_dict =
        resolve_font_dict(doc, &page.resources, &anchor.font_name).ok_or_else(|| {
            FormatError::Unsupported(
            "the run's font resource is unresolvable (outlined/vector art has no font to format)"
                .to_owned(),
        )
        })?;
    let run_font = ExtractFont::resolve(&doc.view(), orig_dict).base_font;
    let resources = page_resources(page);
    let current = anchor.font_name.as_slice();

    let combined = if want.is_none() {
        None
    } else {
        Some(probe_synthesis(doc, resources, &run_font, want, current)?)
    };
    let bold_axis = if want.bold() {
        Some(probe_synthesis(
            doc,
            resources,
            &run_font,
            StyleSynthesis::Bold,
            current,
        )?)
    } else {
        None
    };
    let italic_axis = if want.italic() {
        Some(probe_synthesis(
            doc,
            resources,
            &run_font,
            StyleSynthesis::Italic,
            current,
        )?)
    } else {
        None
    };

    Ok(StyleResolution {
        want,
        run_font,
        combined,
        bold_axis,
        italic_axis,
    })
}

/// Emit synthetic bold — text rendering mode 2 plus a stroke — and the three
/// restores it owes, returning the user-space stroke width for the report.
///
/// ## The three operators, and why each needs its own restore
///
/// | Emitted | Category | Restored by |
/// |---|---|---|
/// | `2 Tr` | text state (§9.3.6) | R88's shared ladder — [`push_state_param`] |
/// | `<w> w` | general graphics state (§8.4.3.2) | the recorded [`LineWidth`](crate::text_edit::edit) |
/// | `<c…> RG`/`G`/`K` | colour (§8.6.8) | the recorded stroking [`FillState`] |
///
/// Only the first is text state. The other two are ordinary graphics state
/// shared with **path** painting, and that is exactly why they must be put
/// back: a synthetic-bold run that leaves `2 w` in force changes the weight
/// of every subsequently stroked path in the content stream — content pdfce
/// never claimed to touch (R32/R46). None of the three can be scoped with
/// `q`/`Q`, which is not admitted inside a text object (§8.2 Table 51).
///
/// ## §9.3.6's two traps, both handled here
///
/// 1. **The stroke takes the STROKING colour**, a different graphics-state
///    entry from the fill, whose Table 52 initial value is black. So the
///    stroking colour is set to match whatever fill will be in force over
///    the run — the newly requested one if the same edit changes the
///    colour, else the run's own — and restored afterwards. Without this,
///    red text acquires black outlines.
/// 2. **The width is in user space, not text space.** It is derived from
///    the rendered size scaled by the text matrix, never set as a constant;
///    see [`bold_stroke_width`].
///
/// # Errors
///
/// [`FormatError::AmbientUnrestorable`] if the ambient `Tr` cannot be
/// restored (R88 tier 3 — a run inside a form XObject).
fn plan_synthetic_bold(
    set_ops: &mut Vec<u8>,
    restore_ops: &mut Vec<u8>,
    anchor: &ShowData,
    emitted_size: f64,
    new_fill: Option<&NewFill>,
    restore_narrowed: &mut Vec<TextStateParam>,
    emitted_state: &mut Vec<TextStateParam>,
) -> Result<f64, FormatError> {
    // (1) `2 Tr` — fill then stroke (Table 106), through the shared ladder
    //     so its restore obeys the same four rungs everything else does.
    push_state_param(
        set_ops,
        restore_ops,
        &anchor.text_state,
        TextStateParam::RenderMode,
        2.0,
        restore_narrowed,
        emitted_state,
    )?;

    // (2) the user-space stroke width (§9.3.6). The CTM is not modelled by
    //     this authoring walk, so only the text matrix's scale is applied
    //     here; a page-level `cm` scale is therefore not compensated. That
    //     is disclosed rather than papered over — see
    //     `disclosure_synthetic_bold`.
    let width = derived_operand(bold_stroke_width(
        emitted_size,
        matrix_scale(anchor.text_matrix),
        1.0,
    ));
    // Minimal-diff: a stream that already has exactly this line width in
    // force gets no `w … w` pair added to it. Same guard, same reason, as
    // `push_state_param`'s no-op skip — and it is not merely cosmetic here,
    // because `w` is shared with path painting and every operator pdfce does
    // not need to write is one fewer place for a restore to be wrong.
    if (width - anchor.line_width.value()).abs() > STATE_EPS {
        push_space(set_ops);
        emit_number(set_ops, width);
        set_ops.extend_from_slice(b" w");
        push_space(restore_ops);
        restore_ops.extend_from_slice(&anchor.line_width.restore_bytes());
    }

    // (3) the stroking colour, matched to the fill in force over the run.
    push_space(set_ops);
    match new_fill {
        // The same edit is also changing the fill: match the NEW colour, in
        // the same device space the operator chose (parity-plus — pdfce
        // does not force DeviceRGB here either).
        Some(nf) => push_stroke_fill(set_ops, nf),
        // Otherwise match the run's existing fill colour.
        None => set_ops.extend_from_slice(&stroking_form_of(&anchor.fill_color)),
    }
    push_space(restore_ops);
    restore_ops.extend_from_slice(&anchor.stroke_color.restore_bytes_stroking());

    Ok(width)
}

/// The **stroking** operator sequence that reproduces a recorded *fill*
/// colour — §9.3.6's "match the stroke to the fill" requirement.
///
/// The three cases mirror [`FillState`] itself:
///
/// - `Default` — no fill operator ran, so the fill is §8.6.8's black
///   `DeviceGray 0`; the stroking spelling of that is `0 G`.
/// - `Device` — re-emit the **components** under the uppercase operator.
///   The recorded raw bytes are deliberately *not* case-swapped: they are
///   the fill's own spelling, and this is a newly authored operator rather
///   than a restore, so a canonical spelling is correct and simpler to
///   verify.
/// - `Other` — a colour in a resource-named space (`/CS0 cs … scn`). Here
///   the raw bytes **are** transformed, by uppercasing the operator tokens
///   `cs`→`CS`, `sc`→`SC`, `scn`→`SCN`. That is safe because §8.6.8 gives
///   the stroking operators the same operand grammar and the same
///   `/ColorSpace` resource namespace as their non-stroking twins — the
///   colour-space *name* is unchanged, only which of the two graphics-state
///   entries it lands in. It is the only way to match a spot/ICC colour
///   pdfce cannot itself decode, and the alternative (emitting nothing)
///   would leave the outline black.
fn stroking_form_of(fill: &FillState) -> Vec<u8> {
    match fill {
        FillState::Default => b"0 G".to_vec(),
        FillState::Device { space, comps, .. } => {
            let mut out = Vec::new();
            for (i, c) in comps.iter().enumerate() {
                if i > 0 {
                    out.push(b' ');
                }
                emit_number(&mut out, *c);
            }
            out.push(b' ');
            out.extend_from_slice(match space {
                crate::text_edit::edit::DeviceSpace::Gray => b"G".as_slice(),
                crate::text_edit::edit::DeviceSpace::Rgb => b"RG".as_slice(),
                crate::text_edit::edit::DeviceSpace::Cmyk => b"K".as_slice(),
            });
            out
        }
        FillState::Other { raw } => {
            let text = String::from_utf8_lossy(raw);
            let swapped: Vec<String> = text
                .split_whitespace()
                .map(|tok| match tok {
                    "cs" => "CS".to_owned(),
                    "sc" => "SC".to_owned(),
                    "scn" => "SCN".to_owned(),
                    other => other.to_owned(),
                })
                .collect();
            swapped.join(" ").into_bytes()
        }
    }
}

/// Append the chosen device colour under its **stroking** operator
/// (`G`/`RG`/`K`), for the case where the same edit is also setting a new
/// fill colour.
fn push_stroke_fill(out: &mut Vec<u8>, nf: &NewFill) {
    for (i, c) in nf.components.iter().enumerate() {
        if i > 0 {
            out.push(b' ');
        }
        emit_number(out, *c);
    }
    out.push(b' ');
    out.extend_from_slice(match nf.model {
        FillModel::Gray => b"G".as_slice(),
        FillModel::Rgb => b"RG".as_slice(),
        FillModel::Cmyk => b"K".as_slice(),
    });
}

/// Emit synthetic italic — an oblique shear premultiplied into the run's
/// `Tm` — bracketed by absolute `Tm` operators, returning
/// `(tan θ, Trise · tan θ)` for the report.
///
/// ## Why this is the one mechanism that does NOT ride R88's ladder
///
/// Everything else this module emits is a §9.3 text-state parameter, and
/// text state is scoped by restore-by-value. A `Tm` is not text state at
/// all: §9.4.2 puts it in the *text object* state, `BT` resets it, `q`/`Q`
/// do not touch it, and `Td`/`TD`/`T*` **derive from** it. So it needs its
/// own scoping, and the scoping is: state the matrix absolutely on both
/// sides of the run.
///
/// ```text
///   … pre …   [Tm sheared @ mid-start]   mid   [Tm upright @ mid-end]   … post …
/// ```
///
/// Both matrices are computed from the anchor's own recorded `Tm` by
/// translating along the baseline by the §9.4.4 advance already consumed —
/// `pre`'s advance for the first, `pre + mid` for the second. A shear does
/// not change an advance (it leaves the matrix's `a` and `b` terms alone, so
/// `translate(tx,0) × Tm` displaces the origin identically sheared or not),
/// which is why the same ΔA arithmetic the rest of the planner uses is still
/// correct here.
///
/// ## The three refusals, and why each is a refusal rather than a fix-up
///
/// 1. **The anchor's matrix is not known.** The walk stopped tracking `Tm`
///    (an unresolvable font, or a composite run it does not decode), so
///    there is no matrix to shear. Emitting a guessed one would move the
///    run.
/// 2. **A `Td`/`TD`/`T*` follows inside the same text object.** Any `Tm`
///    pdfce injects overwrites `Tlm` too, so that operator would translate
///    from pdfce's matrix instead of the producer's line origin and the
///    next line would land shifted by this run's advance. Decision 019 §3.6
///    proposes re-emitting such followers as absolute `Tm`s; pdfce instead
///    requires them to *already* be absolute and refuses otherwise, which
///    is the more conservative reading of the same hazard — see the
///    engineer's note in this module's tests.
/// 3. **[`FollowerDisposition::Pin`] was requested.** Pin consumes ΔA with a
///    trailing compensating `TJ`, but the closing absolute `Tm` here already
///    determines where `post` starts; the two mechanisms would compensate
///    twice. Refused rather than silently ignoring one of them.
///
/// ## The `Ts` × oblique interaction (decision 019 §3.6, named test case)
///
/// A shear maps `x' = x + y·tanθ`. The rise is applied *before* `Tm` (it is
/// the `f` term of the §9.4.4 text-space parameter matrix), so a raised run
/// enters the shear at `y = Trise` and is displaced horizontally by
/// `Trise · tan θ`. At the baseline that is zero, which is why the effect
/// only appears on superscripts and only surprises people there. It is
/// **reported**, not compensated: the displacement is what a genuine oblique
/// face would also do, and silently pulling the run back left would make a
/// synthesized italic sit differently from a real one.
///
/// # Errors
///
/// [`FormatError::ShearUnsupported`] for each of the three conditions above.
#[allow(clippy::too_many_arguments)]
fn plan_synthetic_italic(
    set_ops: &mut Vec<u8>,
    restore_ops: &mut Vec<u8>,
    anchor: &ShowData,
    recs: &[OpRec],
    anchor_index: usize,
    disposition: FollowerDisposition,
    pre: &[ShowElem],
    orig_font: &ExtractFont,
    orig_size: f64,
    mid_advance: f64,
    rise: Option<f64>,
) -> Result<(f64, f64), FormatError> {
    if !anchor.matrix_known {
        return Err(FormatError::ShearUnsupported(
            "pdfce could not track this run's text matrix through the content stream (an \
             unresolvable font resource, or a composite run this surgery does not decode), and a \
             shear must be premultiplied into a KNOWN matrix — guessing one would move the run. \
             Nothing was applied."
                .to_owned(),
        ));
    }
    if matches!(disposition, FollowerDisposition::Pin) {
        return Err(FormatError::ShearUnsupported(
            "synthetic italic re-states the run's text matrix absolutely on both sides of the \
             run, which already fixes where the following text starts; combining that with \
             --pin's compensating TJ would consume the advance delta twice. Re-run without --pin \
             (the default reflow disposition). Nothing was applied."
                .to_owned(),
        ));
    }
    // Scan forward to the end of THIS text object. Past `ET` the next `BT`
    // resets both matrices (§9.4.1 Table 107), so nothing carries over and
    // the hazard stops.
    for r in recs.iter().skip(anchor_index + 1) {
        match &r.rec {
            Rec::EndText => break,
            Rec::Boundary => {
                return Err(FormatError::ShearUnsupported(
                    "a Td/TD/T* next-line operator follows this run inside the same text object. \
                     Synthetic italic must inject an absolute `Tm`, and a `Tm` sets the text LINE \
                     matrix as well (§9.4.2 Table 108) — so that operator would derive its line \
                     from pdfce's matrix instead of the producer's line origin, and the following \
                     line would land shifted by this run's advance. pdfce refuses rather than \
                     mis-positioning it. Nothing was applied."
                        .to_owned(),
                ));
            }
            _ => {}
        }
    }

    // Where `mid` starts: the anchor's matrix advanced along the baseline by
    // everything `pre` shows. `pre` runs at the AMBIENT state (it is outside
    // the set/restore wrap), so it is measured at the ambient Tc/Tw/Th and
    // the run's original size and face.
    let mut pre_advance = 0.0;
    for e in pre {
        match e {
            ShowElem::Str(bytes) => {
                for &c in bytes {
                    pre_advance += glyph_advance_with(
                        orig_font,
                        u32::from(c),
                        orig_size,
                        anchor.tc(),
                        anchor.tw(),
                        anchor.th(),
                        true,
                    );
                }
            }
            // §9.4.3: a bare TJ adjustment displaces by −Tj/1000 · Tfs · Th,
            // with no Tc/Tw term "since no glyph was painted".
            ShowElem::Num(v) => pre_advance += (-v / 1000.0) * orig_size * anchor.th(),
        }
    }

    let tm_mid = mat_mul([1.0, 0.0, 0.0, 1.0, pre_advance, 0.0], anchor.text_matrix);
    let tm_post = mat_mul(
        [1.0, 0.0, 0.0, 1.0, pre_advance + mid_advance, 0.0],
        anchor.text_matrix,
    );

    // Both injected matrices are pdfce's OWN arithmetic — a shear product
    // and an accumulated advance — so every operand goes through
    // `derived_operand`. Without it a `tan θ × a` term or a summed advance
    // arrives as sixteen significant digits of `f64` noise, which bloats a
    // stream this project is trying to keep minimal, makes the diff
    // unreadable, and exceeds the ~5 significant digits Annex C records as
    // PDF's traditional real precision. Nothing the operator typed is
    // rounded here; the producer's own matrix terms survive because
    // rounding a value already at six decimal places is the identity.
    let round6 = |m: [f64; 6]| m.map(derived_operand);
    push_space(set_ops);
    set_ops.extend_from_slice(&emit_tm(round6(shear_into(tm_mid))));
    push_space(restore_ops);
    restore_ops.extend_from_slice(&emit_tm(round6(tm_post)));

    let tan = shear_into([1.0, 0.0, 0.0, 1.0, 0.0, 0.0])[2];
    let rise_offset = rise.unwrap_or(anchor.text_state.rise.value) * tan;
    Ok((tan, derived_operand(rise_offset)))
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

/// Disclose a word-spacing change — and say the three surprising things
/// about `Tw` out loud, because every one of them is a thing an operator
/// would otherwise discover by being wrong about it (Pass 19.4, R91).
///
/// ## Why this disclosure is longer than its siblings
///
/// `Tc` and `Tz` do what their names suggest. `Tw` does not, in three
/// separate ways, and the project's own decision record predicted each:
///
/// 1. **Scope.** Decision 015 §3.1 rejected `Tw` as the justification
///    mechanism partly because it "hits **every** code-32, including
///    leading, trailing and doubled spaces". An operator reaching for this
///    control almost always means "the gap between *these two* words", and
///    that is not what they are about to get. Said first, plainly.
/// 2. **Count.** The number of spaces actually affected is quoted, which
///    turns the point above from a caveat into a fact about *their* run.
///    A count of zero is stated as such rather than hidden: a `Tw` on a
///    space-free run is a real state change with no visible effect, and
///    pretending otherwise in either direction would be a lie.
/// 3. **`Th` coupling.** §9.4.4's `tx = ((w0 − Tj/1000)·Tfs + Tc + Tw)·Th`
///    multiplies `Tw` by the horizontal scaling in force, so under a
///    `50 Tz` the visible gap is half the operand. The effective figure is
///    quoted whenever `Th` is not 1, because a silently-halved number is
///    the sort of thing that gets reported as a rendering bug.
///
/// It also states the composite exclusion even on the simple-font path it
/// is reached from: an operator whose next run is composite meets the
/// refusal cold otherwise.
fn disclosure_word_spacing(
    spec: MetricSpec,
    base_size: f64,
    ambient: f64,
    emitted: f64,
    affected: usize,
    th: f64,
) -> String {
    let derivation = match spec {
        MetricSpec::Absolute(_) => String::from(
            "an ABSOLUTE value, written as typed at every font size (R89 MetricSpec::Absolute)",
        ),
        MetricSpec::Relative(per_mille) => format!(
            "a RELATIVE value: {per_mille} thousandths of the run's {base_size} pt base size \
             re-derives to {emitted} unscaled text-space units. Tw is NOT scaled by the font size \
             (§9.3, Table 105), so storing the ratio rather than the operand is what keeps a later \
             resize correct (R89)"
        ),
    };
    let scope = match affected {
        0 => String::from(
            "This run contains NO single-byte code 32, so the change is real state but has no \
             visible effect here — the operator was emitted and restored exactly as asked, and \
             this sentence is the honest answer rather than a silent no-op",
        ),
        1 => String::from("It applies to the 1 space inside the formatted run"),
        n => format!("It applies to ALL {n} spaces inside the formatted run"),
    };
    let scaling = if (th - 1.0).abs() > STATE_EPS {
        format!(
            " Horizontal scaling is in force at Th = {th}, and §9.4.4 multiplies Tw by it \
             (tx = ((w0 - Tj/1000)*Tfs + Tc + Tw)*Th), so the VISIBLE extra gap is {} text-space \
             units, not {emitted}.",
            derived_operand(emitted * th)
        )
    } else {
        String::new()
    };
    format!(
        "word spacing: Tw {ambient} -> {emitted} for the matched run only (§9.3.3, unscaled \
         text-space units). You supplied {} in {} — {derivation}. SCOPE, stated because it is not \
         what most operators expect: Tw applies to EVERY occurrence of the single-byte character \
         code 32 in the run — leading spaces, trailing spaces and both halves of a doubled space \
         included. There is no per-gap word spacing in PDF; per-gap control is what TJ numeric \
         adjustments do, which is why pdfce's own justification distributes slack as TJ and not as \
         Tw (decision 015 §3.1). {scope}.{scaling} Tw also enters the §9.4.4 advance, so the run's \
         width changed and the rest of the line was relaid out. Finally: Tw is void for COMPOSITE \
         (multi-byte) runs per §9.3.3, where pdfce refuses the request by name rather than writing \
         an operator that would do nothing (R91).",
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
             unchanged. This is how an inherited non-zero rise is flattened for one run; for any \
             other baseline offset use the free-form rise control, which writes the number given \
             and does not resize.",
            pos.label()
        ),
    }
}

/// Disclose a free-form baseline rise (Pass 19.2) — the value, its unit, its
/// R89 mode, and the two things about `Ts` that surprise people.
fn disclosure_rise(spec: MetricSpec, base_size: f64, ambient: f64, emitted: f64) -> String {
    let mode = match spec {
        MetricSpec::Absolute(_) => "ABSOLUTE — written exactly as given, in unscaled text-space \
             units, and NOT rounded or re-derived. A later size change will leave it where it is"
            .to_owned(),
        MetricSpec::Relative(pm) => format!(
            "RELATIVE — {pm} thousandths of the {base_size} pt base size, re-derived to {emitted} \
             at emit time and re-derived again if the run is resized (R89)"
        ),
    };
    format!(
        "baseline rise: Ts {emitted} (was {ambient}). The value you supplied is {}, in {}; it is \
         {mode}. Two properties of Ts worth knowing: it is a TRANSLATION in the text rendering \
         matrix (§9.3.7), so it moves the run without changing its advance and without moving \
         anything after it; and it is in unscaled text-space units, so it is NOT scaled by the \
         font size. This control is a deliberate pdfce EXCEED — current Acrobat exposes only a \
         coarse superscript/subscript toggle and dropped free-form baseline offset when text \
         editing was consolidated (decision 019 §1.1/§3.2).",
        spec.raw(),
        spec.unit_label(),
    )
}

/// Disclose a synthetic bold: the mechanism, the width by value, and the one
/// case pdfce knowingly does not compensate for.
fn disclosure_synthetic_bold(width: f64, rendered_size: f64) -> String {
    format!(
        "synthetic bold: the run is painted in text rendering mode 2 (fill, THEN stroke — §9.3.6 \
         Table 106) with a stroke width of {width} and the STROKING colour set to match the \
         run's fill. The width is derived as {BOLD_STROKE_RATIO} x the {rendered_size} pt \
         rendered size because §9.3.6 interprets a stroked-text line width in USER space, not \
         text space — a constant width would look right at one size and wrong at every other. \
         All three operators (Tr, w, and the stroking colour) are RESTORED immediately after the \
         run; the line width and stroking colour especially, because they are shared with path \
         painting and a stale value would change the weight of later strokes. LIMIT, disclosed \
         rather than hidden: pdfce derives the width from the text matrix only, so a page-level \
         `cm` scale is not compensated and the outline will scale with the page. This is a \
         FALLBACK weight, not a real Bold face: the letterforms are the regular face's, thickened."
    )
}

/// Disclose a synthetic italic: the mechanism, the scoping, and the rise
/// interaction — including when the interaction is inert.
fn disclosure_synthetic_italic(tan: f64, rise_offset: f64) -> String {
    let interaction = if rise_offset.abs() > STATE_EPS {
        format!(
            "Because this run is also RAISED, the shear displaces it horizontally by \
             Trise x tan(theta) = {rise_offset} text-space units. That is not a bug and is not \
             compensated: a real oblique face displaces a raised run the same way, and pulling it \
             back would make pdfce's synthesis sit differently from the real thing."
        )
    } else {
        "The run sits on the baseline, where a shear displaces nothing (x' = x + y·tan(theta) and \
         y = 0), so no horizontal offset arises here. It would if the run were raised."
            .to_owned()
    };
    format!(
        "synthetic italic: an oblique shear of tan(theta) = {tan} (12 degrees) premultiplied into \
         the run's text matrix. Unlike the spacing controls this is NOT text state, so it is not \
         covered by the restore ladder — a Tm is not saved by q/Q and IS propagated by \
         Td/TD/T*, which derive the next line by translating the line matrix. pdfce therefore \
         brackets the run with two ABSOLUTE Tm operators: the sheared matrix before it, the \
         original matrix at its end. {interaction} This is a FALLBACK slant, not a real Italic \
         face: the letterforms are the upright face's, leaned."
    )
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
fn disclosure_justify_invalidated(
    h_scale: bool,
    spacing: bool,
    word_spacing: bool,
    delta: f64,
) -> String {
    let cause = match (h_scale, spacing, word_spacing) {
        (true, _, true) => "horizontal scaling and the word-spacing change",
        (true, true, false) => "horizontal scaling and the spacing/size change",
        (true, false, false) => "horizontal scaling",
        (false, _, true) => "the word-spacing change",
        _ => "the spacing/size change",
    };
    // A `Tw` edit on a justified line is the sharpest case in this family
    // and gets one extra sentence, because the two mechanisms are direct
    // rivals for the same physical space (decision 015 §3.1). Without it
    // the operator is told the line broke but not that they now have two
    // things distributing inter-word space.
    let rivalry = if word_spacing {
        " NOTE, specific to word spacing: TJ slack and Tw are two DIFFERENT mechanisms both \
         widening the gaps between words, and they add. The slack numbers on this line were \
         computed by pdfce's justifier on the assumption that Tw was whatever it was before; \
         they are now competing with your Tw over the same space."
    } else {
        ""
    };
    format!(
        "JUSTIFY invalidated (disclosed, with a remedy — never silently left wrong): this run's \
         show operator carries TJ numeric adjustments, which is the shape a JUSTIFIED line's \
         slack takes (it can also be a producer's kerning — pdfce cannot tell the two apart from \
         the bytes alone). {cause} moved the run's width by {delta:.4} text-space units, so slack \
         computed against the OLD width no longer fills the measure and the line's right edge \
         will not align. The surviving TJ numbers were NOT rescaled: they sit outside the \
         restored state wrap and still run at the ambient Tz (§9.3.4 would rescale them only \
         inside it).{rivalry} REMEDY: re-justify the paragraph — `pdfce-cli reflow --page N \
         --block K --align justified` — which recomputes the slack against the new widths."
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
    use crate::text_edit::synth::OBLIQUE_TAN;

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
            FormatRequest::new(0, "hello").rise(MetricSpec::Absolute(3.0)),
            FormatRequest::new(0, "hello").synthetic(StyleSynthesis::Bold),
        ] {
            set_format(&doc, &req, &FormatOptions::default())
                .expect("each 19.1/19.2 control is a formatting operation on its own");
        }
    }

    // ===============================================================
    // Pass 19.2 — free-form `Ts` and synthetic bold/italic
    // ===============================================================

    /// A one-page PDF whose `/Font` carries BOTH a regular and a real Bold
    /// face of the same family — the fixture the R90 fallback-only gate is
    /// measured against.
    fn family_pdf(content: &str) -> Vec<u8> {
        build_pdf(
            content,
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
        )
    }

    // --- free-form `Ts` (decision 019 §3.2's deliberate exceed) ---

    /// An ABSOLUTE rise is written exactly as typed. This is the R89
    /// contract for operator-supplied numbers, and it is the reason
    /// `derived_operand` is deliberately NOT applied to them: rounding a
    /// number a human entered is a silent modification.
    #[test]
    fn a_free_form_absolute_rise_is_written_as_typed_and_restored() {
        let src = fill_pdf("BT /F1 12 Tf 72 700 Td (hello world) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello").rise(MetricSpec::Absolute(3.25)),
            &FormatOptions::default(),
        )
        .unwrap();
        let text = as_text(&out.bytes);
        assert!(text.contains("3.25 Ts"), "written as typed: {text}");
        // The ambient rise was never set, so the restore is the Table 105
        // default made explicit — rung 1 of the ladder.
        assert!(
            text.contains("0 Ts"),
            "restored to the spec default: {text}"
        );
        assert_eq!(out.report.rise_change, Some((0.0, 3.25)));
        // Rise is a Trm translation (§9.3.7): it must not move anything.
        assert_eq!(
            out.report.advance_delta, 0.0,
            "a rise changes position, never advance"
        );
    }

    /// A RELATIVE rise re-derives against the **base** size (R89), which is
    /// the whole point of the discriminated unit model: the same request at
    /// a different size lands at a proportionally different rise.
    #[test]
    fn a_relative_rise_re_derives_against_the_base_size() {
        let src = fill_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        // 250 thousandths of an em at 12 pt = 3.0.
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello").rise(MetricSpec::Relative(250.0)),
            &FormatOptions::default(),
        )
        .unwrap();
        assert_eq!(out.report.rise_change, Some((0.0, 3.0)));

        // The SAME request, with the run resized to 24 pt in the same edit,
        // must resolve against the NEW base size: 250‰ of 24 = 6.0.
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello")
                .size(24.0)
                .rise(MetricSpec::Relative(250.0)),
            &FormatOptions::default(),
        )
        .unwrap();
        assert_eq!(
            out.report.rise_change,
            Some((0.0, 6.0)),
            "a relative rise resolves against the base size in force AFTER the edit (R89)"
        );
    }

    /// Both a free-form rise and a script toggle write `Ts`. pdfce refuses
    /// rather than silently choosing (rule 4).
    #[test]
    fn a_rise_and_a_script_toggle_together_are_refused_not_silently_merged() {
        let src = fill_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let err = set_format(
            &doc,
            &FormatRequest::new(0, "hello")
                .rise(MetricSpec::Absolute(3.0))
                .script(ScriptPosition::Superscript),
            &FormatOptions::default(),
        )
        .unwrap_err();
        assert!(matches!(err, FormatError::ConflictingRise), "{err}");
    }

    /// The ambient-rise restore, on a stream that already had one. Rung 2:
    /// the producer's own bytes come back verbatim, trailing zeros included.
    #[test]
    fn an_ambient_rise_is_restored_byte_faithfully_after_a_free_form_rise() {
        let src = fill_pdf("BT /F1 12 Tf 2.50 Ts 72 700 Td (hello world) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello").rise(MetricSpec::Absolute(6.0)),
            &FormatOptions::default(),
        )
        .unwrap();
        let appended = as_text(&out.bytes[..]);
        assert!(appended.contains("6 Ts"), "new rise: {appended}");
        assert!(
            appended.matches("2.50 Ts").count() >= 2,
            "the producer's own spelling comes back verbatim, not renormalized \
             to `2.5 Ts`: {appended}"
        );
        assert_eq!(out.report.rise_change, Some((2.5, 6.0)));
    }

    // --- (a)/(b)/(c): the R90 fallback-only gate ---

    /// **(a)** A real Bold face resolves on the page, so synthesis is
    /// REFUSED and the operator is pointed at the genuine face. Synthesis is
    /// a fallback, never an alternative to a real typeface.
    #[test]
    fn synthesis_is_refused_when_a_real_bold_face_resolves() {
        let src = family_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let err = set_format(
            &doc,
            &FormatRequest::new(0, "hello").synthetic(StyleSynthesis::Bold),
            &FormatOptions::default(),
        )
        .unwrap_err();
        match err {
            FormatError::RealFaceAvailable {
                ref real_font,
                ref resource,
                ..
            } => {
                assert_eq!(real_font, "Times-Bold");
                assert_eq!(resource, "F2");
            }
            other => panic!("expected the real-face refusal, got {other}"),
        }
        assert!(
            err.to_string().contains("fallback"),
            "the refusal explains WHY: {err}"
        );
    }

    /// **(b)** No real Bold face resolves anywhere on the page, so synthesis
    /// is available — and is applied only because it was asked for.
    #[test]
    fn synthesis_applies_when_no_real_face_resolves_and_only_when_asked() {
        let src = fill_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello").synthetic(StyleSynthesis::Bold),
            &FormatOptions::default(),
        )
        .unwrap();
        assert_eq!(out.report.synthesis, StyleSynthesis::Bold);
        let text = as_text(&out.bytes);
        assert!(text.contains("2 Tr"), "fill-then-stroke mode: {text}");
        assert!(text.contains(" w"), "a stroke width: {text}");
        // The offer is disclosed by name, with the remedies, every time.
        assert!(
            out.report
                .disclosures
                .iter()
                .any(|d| d.contains("SYNTHETIC STYLE") && d.contains("Helvetica")),
            "the offer names the font: {:?}",
            out.report.disclosures
        );
    }

    /// **(c)** Declining is the default. The identical edit without the
    /// synthesis request applies nothing and emits no rendering mode — there
    /// is no global preference that could turn it on behind the operator's
    /// back (deliberately stricter than Acrobat).
    #[test]
    fn declining_synthesis_applies_nothing_at_all() {
        let src = fill_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello").size(14.0),
            &FormatOptions::default(),
        )
        .unwrap();
        assert_eq!(out.report.synthesis, StyleSynthesis::None);
        assert!(out.report.synthetic_bold_width.is_none());
        let appended = as_text(&out.bytes[..]);
        assert!(
            !appended.contains(" Tr"),
            "no rendering mode is written when synthesis was not requested: {appended}"
        );
    }

    // --- (e): §9.3.6's stroking-colour trap ---

    /// **(e)** The named hazard. A faux bold on RED text must produce RED
    /// outlines, not black ones — the stroke takes the *stroking* colour
    /// (§9.3.6), a different graphics-state entry whose initial value is
    /// black. And the previous stroking colour must come back afterwards.
    #[test]
    fn faux_bold_on_coloured_text_matches_and_restores_the_stroking_colour() {
        let src = fill_pdf("BT /F1 12 Tf 1 0 0 rg 0 0 1 RG 72 700 Td (hello) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello").synthetic(StyleSynthesis::Bold),
            &FormatOptions::default(),
        )
        .unwrap();
        let appended = as_text(&out.bytes[..]);
        assert!(
            appended.contains("1 0 0 RG"),
            "the stroking colour is matched to the RED fill, so the outline is \
             red and not black: {appended}"
        );
        assert!(
            appended.contains("0 0 1 RG"),
            "and the page's own BLUE stroking colour is restored afterwards, so \
             a later stroked path is unaffected: {appended}"
        );
    }

    /// The same requirement when the edit *also* changes the fill colour:
    /// the outline must follow the NEW colour, in the same device space.
    #[test]
    fn faux_bold_matches_a_newly_requested_fill_colour_in_its_own_space() {
        let src = fill_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello")
                .fill(NewFill::new(FillModel::Cmyk, vec![0.0, 1.0, 1.0, 0.0]).unwrap())
                .synthetic(StyleSynthesis::Bold),
            &FormatOptions::default(),
        )
        .unwrap();
        let text = as_text(&out.bytes);
        assert!(text.contains("0 1 1 0 k"), "the new fill: {text}");
        assert!(
            text.contains("0 1 1 0 K"),
            "the stroke matches it in the SAME space (no forced DeviceRGB): {text}"
        );
    }

    /// The line width is graphics state shared with **path** painting, so a
    /// synthetic bold that did not restore it would change the weight of
    /// every later stroke in the stream. The leak test for `w`.
    #[test]
    fn faux_bold_restores_the_line_width_it_borrowed() {
        let src = fill_pdf("3 w BT /F1 12 Tf 72 700 Td (hello) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello").synthetic(StyleSynthesis::Bold),
            &FormatOptions::default(),
        )
        .unwrap();
        let appended = as_text(&out.bytes[..]);
        assert!(
            appended.matches("3 w").count() >= 2,
            "the page's own 3 w is restored after the run, byte-faithfully, so \
             later path strokes keep their weight: {appended}"
        );
        assert!(
            appended.contains("0.264 w"),
            "and the synthetic width itself is 2.2% of the 12 pt size: {appended}"
        );
    }

    /// **(f)** The width is derived in USER space from the rendered size, so
    /// a faux bold at 10 pt and at 72 pt look like the same *weight* rather
    /// than the same *thickness*.
    #[test]
    fn faux_bold_width_scales_with_the_rendered_size() {
        let src = fill_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let small = set_format(
            &doc,
            &FormatRequest::new(0, "hello")
                .size(10.0)
                .synthetic(StyleSynthesis::Bold),
            &FormatOptions::default(),
        )
        .unwrap();
        let large = set_format(
            &doc,
            &FormatRequest::new(0, "hello")
                .size(72.0)
                .synthetic(StyleSynthesis::Bold),
            &FormatOptions::default(),
        )
        .unwrap();
        let (sw, lw) = (
            small.report.synthetic_bold_width.unwrap(),
            large.report.synthetic_bold_width.unwrap(),
        );
        assert!((sw - 0.22).abs() < 1e-9, "10 pt ⇒ 0.22, got {sw}");
        assert!((lw - 1.584).abs() < 1e-9, "72 pt ⇒ 1.584, got {lw}");
        assert!(
            (lw / sw - 7.2).abs() < 1e-9,
            "linear in the size, not a constant"
        );
    }

    /// A super/subscript reduces the rendered size, and the stroke width
    /// must follow the size the glyphs are actually painted at — not the
    /// base size. Otherwise a bolded superscript is visibly over-weight.
    #[test]
    fn faux_bold_on_a_superscript_uses_the_reduced_rendered_size() {
        let src = fill_pdf("BT /F1 20 Tf 72 700 Td (hello) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello")
                .script(ScriptPosition::Superscript)
                .synthetic(StyleSynthesis::Bold),
            &FormatOptions::default(),
        )
        .unwrap();
        // 20 pt × 0.60 = 12 pt rendered; 12 × 0.022 = 0.264.
        assert_eq!(out.report.script_size, Some((20.0, 12.0)));
        assert!(
            (out.report.synthetic_bold_width.unwrap() - 0.264).abs() < 1e-9,
            "the width follows the REDUCED size: {:?}",
            out.report.synthetic_bold_width
        );
    }

    // --- synthetic italic and its three refusals ---

    /// The shear is emitted as a bracketing pair of ABSOLUTE `Tm`s: the
    /// sheared matrix before the run, the upright one at its end. That is
    /// what stops the lean propagating into `post` and into any following
    /// show operator.
    #[test]
    fn synthetic_italic_brackets_the_run_with_absolute_text_matrices() {
        let src = fill_pdf("BT /F1 12 Tf 1 0 0 1 72 700 Tm (hello world) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello").synthetic(StyleSynthesis::Italic),
            &FormatOptions::default(),
        )
        .unwrap();
        let appended = as_text(&out.bytes[..]);
        // The run starts at the anchor's own origin (nothing precedes it in
        // the operator), sheared.
        assert!(
            appended.contains("1 0 0.212557 1 72 700 Tm"),
            "sheared matrix before the run: {appended}"
        );
        // …and an UPRIGHT matrix at the run's end, so " world" is not leaned.
        assert!(
            appended.contains(" 1 0 0 1 ") && appended.matches(" Tm").count() >= 2,
            "an upright absolute Tm closes the shear's scope: {appended}"
        );
        assert_eq!(out.report.synthesis, StyleSynthesis::Italic);
    }

    /// **(g)** Decision 019 §3.6's named interaction. A shear maps
    /// `x' = x + y·tanθ`, so a run that is also RAISED is displaced
    /// horizontally by `Trise · tan θ` — and a run on the baseline is not
    /// displaced at all. Both halves are asserted, because a test that only
    /// checked the raised case would pass for an implementation that
    /// displaced everything.
    #[test]
    fn a_raised_sheared_run_is_displaced_by_rise_times_tan_theta() {
        let src = fill_pdf("BT /F1 12 Tf 1 0 0 1 72 700 Tm (hello) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();

        // On the baseline: no horizontal displacement.
        let flat = set_format(
            &doc,
            &FormatRequest::new(0, "hello").synthetic(StyleSynthesis::Italic),
            &FormatOptions::default(),
        )
        .unwrap();
        let (tan, offset) = flat.report.synthetic_italic.unwrap();
        assert!((tan - OBLIQUE_TAN).abs() < 1e-12);
        assert_eq!(offset, 0.0, "a baseline run is not displaced by a shear");

        // Raised 8 units: displaced by 8 × tan 12° ≈ 1.700456.
        let raised = set_format(
            &doc,
            &FormatRequest::new(0, "hello")
                .rise(MetricSpec::Absolute(8.0))
                .synthetic(StyleSynthesis::Italic),
            &FormatOptions::default(),
        )
        .unwrap();
        let (_, offset) = raised.report.synthetic_italic.unwrap();
        assert!(
            (offset - 8.0 * OBLIQUE_TAN).abs() < 1e-6,
            "expected ≈{}, got {offset}",
            8.0 * OBLIQUE_TAN
        );
        assert!(
            raised
                .report
                .disclosures
                .iter()
                .any(|d| d.contains("RAISED") && d.contains("not compensated")),
            "the interaction is disclosed rather than silently corrected: {:?}",
            raised.report.disclosures
        );
    }

    /// The hazard that gets its own refusal: a `Td`/`TD`/`T*` after the run
    /// inside the same text object. Any injected `Tm` sets the text LINE
    /// matrix too, so that operator would translate from pdfce's matrix and
    /// the next line would land shifted by this run's advance.
    #[test]
    fn synthetic_italic_refuses_when_a_next_line_operator_follows_in_the_same_text_object() {
        let src =
            fill_pdf("BT /F1 12 Tf 1 0 0 1 72 700 Tm (hello) Tj 0 -14 Td (second line) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let err = set_format(
            &doc,
            &FormatRequest::new(0, "hello").synthetic(StyleSynthesis::Italic),
            &FormatOptions::default(),
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(matches!(err, FormatError::ShearUnsupported(_)), "{msg}");
        assert!(msg.contains("Td/TD/T*"), "the refusal names why: {msg}");
        assert!(msg.contains("Nothing was applied"), "{msg}");
    }

    /// …and the same run is fine once that follower is in its OWN text
    /// object, because `BT` resets both matrices (§9.4.1 Table 107) so
    /// nothing pdfce injected can reach it. This is the test that keeps the
    /// refusal above honest — without it, "refuse always" would also pass.
    #[test]
    fn synthetic_italic_is_allowed_when_the_next_line_is_a_separate_text_object() {
        let src = fill_pdf(
            "BT /F1 12 Tf 1 0 0 1 72 700 Tm (hello) Tj ET\n\
             BT /F1 12 Tf 1 0 0 1 72 686 Tm (second line) Tj ET\n",
        );
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello").synthetic(StyleSynthesis::Italic),
            &FormatOptions::default(),
        )
        .unwrap();
        assert_eq!(out.report.synthesis, StyleSynthesis::Italic);
        let appended = as_text(&out.bytes[..]);
        assert!(
            appended.contains("1 0 0 1 72 686 Tm"),
            "the following text object's own matrix is untouched: {appended}"
        );
    }

    // --- Pass 19.3: the read-only pre-resolution query (Option B) ---

    /// Resolve a preview against a `&Document`, mirroring what
    /// `EditSession::preview_style_resolution` does for the session path.
    fn preview(doc: &Document, find: &str, want: StyleSynthesis) -> StyleResolution {
        let pages = page_tree::pages(doc).unwrap();
        let page = pages.first().unwrap();
        let stream = ContentStream::from_page(&doc.view(), page).unwrap();
        preview_style_resolution(doc, page, &stream, find, None, want).unwrap()
    }

    /// The preview's whole reason to exist: the operator learns which of the
    /// two outcomes they are heading for BEFORE clicking, and the answer is
    /// the gate's own, not a re-derivation of it.
    #[test]
    fn the_preview_reports_would_synthesize_when_no_real_face_resolves() {
        // Helvetica alone on the page — no Helvetica-Bold sibling.
        let doc = Document::from_bytes(fill_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n")).unwrap();
        let res = preview(&doc, "hello", StyleSynthesis::Bold);
        assert_eq!(res.run_font, "Helvetica");
        assert_eq!(res.combined, Some(StyleOutcome::WouldSynthesize));
        assert_eq!(res.bold_axis, Some(StyleOutcome::WouldSynthesize));
        assert_eq!(res.italic_axis, None, "an unasked axis is not probed");
        assert!(!res.is_mixed());
    }

    /// …and the mirror case names the same resource and `/BaseFont` the
    /// commit path's refusal would name. If these two ever disagreed the
    /// preview would be lying, which is why the query calls `gate_synthesis`
    /// rather than re-implementing the family/name matching (R74).
    #[test]
    fn the_preview_names_the_same_real_face_the_refusal_would() {
        let doc =
            Document::from_bytes(family_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n")).unwrap();
        let res = preview(&doc, "hello", StyleSynthesis::Bold);
        assert_eq!(
            res.combined,
            Some(StyleOutcome::RealFaceResolves {
                real_font: "Times-Bold".to_owned(),
                resource: "F2".to_owned(),
            })
        );
        assert!(!res.is_mixed(), "a single face covers everything asked for");

        // The same request, actually submitted, refuses with those strings.
        let err = set_format(
            &doc,
            &FormatRequest::new(0, "hello").synthetic(StyleSynthesis::Bold),
            &FormatOptions::default(),
        )
        .unwrap_err();
        match err {
            FormatError::RealFaceAvailable {
                ref real_font,
                ref resource,
                ..
            } => {
                assert_eq!(real_font, "Times-Bold");
                assert_eq!(resource, "F2");
            }
            other => panic!("expected the real-face refusal, got {other}"),
        }
    }

    /// **The mixed case the all-or-nothing gate cannot express.** The page
    /// has a real `Times-Bold` but no `Times-Italic` and no
    /// `Times-BoldItalic`. Asking for both:
    ///
    /// - the COMBINED probe says "would synthesize" — because no single face
    ///   covers both — so a naive caller would synthesize BOTH and silently
    ///   pass over a real Bold that is sitting right there;
    /// - the per-axis probes expose that, and `is_mixed()` is the predicate a
    ///   caller uses to refuse and disclose instead.
    #[test]
    fn the_preview_exposes_a_mixed_bold_italic_request() {
        let doc =
            Document::from_bytes(family_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n")).unwrap();
        let res = preview(&doc, "hello", StyleSynthesis::BoldItalic);
        assert_eq!(
            res.combined,
            Some(StyleOutcome::WouldSynthesize),
            "no single face covers Bold AND Italic — this is the trap"
        );
        assert!(
            res.bold_axis
                .as_ref()
                .is_some_and(StyleOutcome::is_real_face),
            "a real Bold exists on its own: {:?}",
            res.bold_axis
        );
        assert_eq!(
            res.italic_axis,
            Some(StyleOutcome::WouldSynthesize),
            "no real Italic exists"
        );
        assert!(
            res.is_mixed(),
            "the caller must be able to recognise this without re-deriving the gate"
        );
    }

    /// Asking about nothing answers nothing — no probe is run, so the query
    /// costs nothing on the frames where no style is ticked.
    #[test]
    fn the_preview_probes_nothing_when_no_style_is_requested() {
        let doc = Document::from_bytes(fill_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n")).unwrap();
        let res = preview(&doc, "hello", StyleSynthesis::None);
        assert_eq!(res.combined, None);
        assert_eq!(res.bold_axis, None);
        assert_eq!(res.italic_axis, None);
        assert!(!res.is_mixed());
    }

    /// Read-only means read-only: the query must not disturb the document it
    /// was asked about, so a commit made afterwards is byte-identical to one
    /// made without the query.
    #[test]
    fn the_preview_has_no_side_effects_on_the_document() {
        let src = fill_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n");
        let doc = Document::from_bytes(src.clone()).unwrap();
        let without = set_format(
            &doc,
            &FormatRequest::new(0, "hello").synthetic(StyleSynthesis::Bold),
            &FormatOptions::default(),
        )
        .unwrap();

        let doc2 = Document::from_bytes(src).unwrap();
        let _ = preview(&doc2, "hello", StyleSynthesis::BoldItalic);
        let _ = preview(&doc2, "hello", StyleSynthesis::Bold);
        let with = set_format(
            &doc2,
            &FormatRequest::new(0, "hello").synthetic(StyleSynthesis::Bold),
            &FormatOptions::default(),
        )
        .unwrap();
        assert_eq!(without.bytes, with.bytes);
    }

    /// Pin and the shear are two mechanisms for the same job. Refused rather
    /// than letting them compensate twice.
    #[test]
    fn synthetic_italic_refuses_the_pin_disposition() {
        let src = fill_pdf("BT /F1 12 Tf 1 0 0 1 72 700 Tm (hello) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let err = set_format(
            &doc,
            &FormatRequest::new(0, "hello").synthetic(StyleSynthesis::Italic),
            &FormatOptions::default().with_disposition(FollowerDisposition::Pin),
        )
        .unwrap_err();
        assert!(matches!(err, FormatError::ShearUnsupported(_)), "{err}");
        assert!(err.to_string().contains("--pin"), "{err}");
    }

    /// The shear must be premultiplied into the run's ACTUAL matrix, not
    /// assumed to be the identity. Here the producer set a 2× scaled matrix;
    /// a naive `c = tanθ` would produce half the intended lean.
    #[test]
    fn the_shear_composes_with_a_non_identity_producer_matrix() {
        let src = fill_pdf("BT /F1 12 Tf 2 0 0 2 72 700 Tm (hello) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello").synthetic(StyleSynthesis::Italic),
            &FormatOptions::default(),
        )
        .unwrap();
        // c' = tanθ·a + c = 0.212557 × 2 = 0.425114
        let appended = as_text(&out.bytes[..]);
        assert!(
            appended.contains("2 0 0.425114 2 72 700 Tm"),
            "the shear scales with the matrix it is premultiplied into: {appended}"
        );
    }

    // --- (d)/(h): tagging, and self-evident persistence ---

    /// **(d)** A synthesized run inside a tagged sequence keeps its
    /// `BDC …/MCID… EMC` wrapper by construction, and the staleness of the
    /// structure tree is disclosed (R73) rather than corrupted — explicitly
    /// unlike Acrobat's documented tag-tree defect for formatting edits.
    #[test]
    fn synthesis_on_a_tagged_run_preserves_the_mcid_wrapper_and_discloses_staleness() {
        let src = fill_pdf("/P << /MCID 4 >> BDC BT /F1 12 Tf 72 700 Td (hello) Tj ET EMC\n");
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello").synthetic(StyleSynthesis::Bold),
            &FormatOptions::default(),
        )
        .unwrap();
        assert_eq!(out.report.tagged_mcid, Some(4));
        let appended = as_text(&out.bytes[..]);
        assert!(
            appended.contains("/P << /MCID 4 >> BDC") && appended.contains("EMC"),
            "the marked-content wrapper survives verbatim: {appended}"
        );
        assert!(
            out.report.disclosures.iter().any(|d| d.contains("MCID")),
            "staleness disclosed: {:?}",
            out.report.disclosures
        );
    }

    /// **(h)** The whole persistence story: save, reload, and re-detect the
    /// synthesis from the bytes — with **no private marker** having been
    /// written into the file.
    ///
    /// The second half of this test is the load-bearing one. It is easy to
    /// build a detector that works because the emitter secretly cooperated;
    /// asserting that the appended revision contains no pdfce-specific key
    /// is what proves the detection is genuinely from the standard bytes,
    /// which is also why it works on other producers' files.
    #[test]
    fn a_synthesized_run_is_re_detected_on_reload_with_no_marker_in_the_file() {
        let src = fill_pdf("BT /F1 12 Tf 1 0 0 1 72 700 Tm (hello) Tj ET\n");
        let doc = Document::from_bytes(src.clone()).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello").synthetic(StyleSynthesis::BoldItalic),
            &FormatOptions::default(),
        )
        .unwrap();

        // No private marker of any kind was written.
        let appended = as_text(&out.bytes[src.len()..]);
        for forbidden in ["PieceInfo", "pdfce", "PDFCE", "Synth"] {
            assert!(
                !appended.contains(forbidden),
                "the saved bytes must carry NO private marker; found {forbidden:?} in {appended}"
            );
        }

        // Reload and re-derive the run's state from the file alone.
        let reloaded = Document::from_bytes(out.bytes.clone()).unwrap();
        let pages = crate::page_tree::pages(&reloaded).unwrap();
        let stream = ContentStream::from_page(&reloaded.view(), &pages[0]).unwrap();
        let mut walk = Walk::new(&reloaded, &pages[0].resources);
        for op in stream.operations() {
            walk.operation(&op, &stream.buf);
        }
        let run = walk
            .recs
            .iter()
            .find_map(|r| match &r.rec {
                Rec::Show(s) if s.text == "hello" => Some(s.clone()),
                _ => None,
            })
            .expect("the synthesized run is found again on reload");

        let detected = crate::text_edit::synth::detect(
            "Helvetica",
            run.text_state.params().render_mode,
            run.line_width.value(),
            run.tf_size,
            run.text_matrix,
        );
        assert_eq!(
            detected,
            StyleSynthesis::BoldItalic,
            "the synthesis is re-detectable from the standard bytes alone"
        );
    }

    /// The restore leak test for synthesis, stated as the property that
    /// matters: a run AFTER the synthesized one must be painted in the
    /// ambient rendering mode, not in mode 2.
    #[test]
    fn synthesis_does_not_leak_into_the_following_run() {
        let src = fill_pdf("BT /F1 12 Tf 1 0 0 1 72 700 Tm (hello) Tj (plain) Tj ET\n");
        let doc = Document::from_bytes(src.clone()).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello").synthetic(StyleSynthesis::Bold),
            &FormatOptions::default(),
        )
        .unwrap();

        // Re-walk the saved result and ask what state the SECOND run is in.
        let reloaded = Document::from_bytes(out.bytes).unwrap();
        let pages = crate::page_tree::pages(&reloaded).unwrap();
        let stream = ContentStream::from_page(&reloaded.view(), &pages[0]).unwrap();
        let mut walk = Walk::new(&reloaded, &pages[0].resources);
        for op in stream.operations() {
            walk.operation(&op, &stream.buf);
        }
        let plain = walk
            .recs
            .iter()
            .find_map(|r| match &r.rec {
                Rec::Show(s) if s.text == "plain" => Some(s.clone()),
                _ => None,
            })
            .expect("the following run survives");
        assert_eq!(
            plain.text_state.params().render_mode,
            0,
            "the following run is back in the ambient fill-only mode — the \
             synthetic bold did not bleed past its restore"
        );
        assert_eq!(
            plain.line_width.value(),
            1.0,
            "and the line width is back at Table 52's initial value"
        );
        assert_eq!(
            crate::text_edit::synth::detect(
                "Helvetica",
                plain.text_state.params().render_mode,
                plain.line_width.value(),
                plain.tf_size,
                plain.text_matrix,
            ),
            StyleSynthesis::None,
            "and the detector agrees the following run is not synthesized"
        );
    }

    /// The `Other`-colour-space case for the stroking match. pdfce cannot
    /// decode an ICC/spot colour, but it can still put the *same* colour on
    /// the stroke by re-stating the space and value through the uppercase
    /// operators — which is the only alternative to a black outline.
    #[test]
    fn faux_bold_matches_an_undecodable_colour_space_by_operator_case() {
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
            &FormatRequest::new(0, "hello").synthetic(StyleSynthesis::Bold),
            &FormatOptions::default(),
        )
        .unwrap();
        let text = as_text(&out.bytes);
        assert!(
            text.contains("/CS0 CS 0.2 0.4 0.6 SCN"),
            "the same space and value, on the STROKING side: {text}"
        );
    }

    /// The family gate must not be fooled by a subset tag or by a comma
    /// style separator — both are ordinary `/BaseFont` spellings (§9.6.2.2,
    /// §9.6.4) and both would otherwise let a real Bold hide from the gate.
    #[test]
    fn the_real_face_gate_sees_through_subset_tags_and_comma_styles() {
        let src = build_pdf(
            "BT /F1 12 Tf 72 700 Td (hello) Tj ET\n",
            &[
                (
                    b"F1".to_vec(),
                    b"<< /Type /Font /Subtype /Type1 /BaseFont /ABCDEF+Arial /Encoding /WinAnsiEncoding >>"
                        .to_vec(),
                ),
                (
                    b"F9".to_vec(),
                    b"<< /Type /Font /Subtype /Type1 /BaseFont /GHIJKL+Arial,Bold /Encoding /WinAnsiEncoding >>"
                        .to_vec(),
                ),
            ],
        );
        let doc = Document::from_bytes(src).unwrap();
        let err = set_format(
            &doc,
            &FormatRequest::new(0, "hello").synthetic(StyleSynthesis::Bold),
            &FormatOptions::default(),
        )
        .unwrap_err();
        match err {
            FormatError::RealFaceAvailable { ref real_font, .. } => {
                assert_eq!(real_font, "GHIJKL+Arial,Bold");
            }
            other => panic!("expected the real-face refusal, got {other}"),
        }
    }

    /// A `Times-Bold` on the page does NOT satisfy a request for synthetic
    /// *italic* — the gate requires the real face to cover every style asked
    /// for, or it is not a substitute.
    #[test]
    fn a_bold_sibling_does_not_satisfy_an_italic_synthesis_request() {
        let src = family_pdf("BT /F1 12 Tf 1 0 0 1 72 700 Tm (hello) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello").synthetic(StyleSynthesis::Italic),
            &FormatOptions::default(),
        )
        .expect("no real Italic resolves, so synthesis is available");
        assert_eq!(out.report.synthesis, StyleSynthesis::Italic);
    }

    // ===============================================================
    // Pass 19.4 — word spacing (`Tw`), the final FF-H control
    // ===============================================================

    /// A one-page PDF whose `/F1` is a **composite** Type 0 / Identity-H
    /// font — the fixture R91's refusal is measured against.
    ///
    /// Written with a DIRECT `/DescendantFonts` array so it needs no extra
    /// indirect objects and the shared [`build_pdf`] helper still applies.
    /// `ExtractFont::resolve` reads `/Encoding /Identity-H` and classifies
    /// the codespace as 2-byte (§9.7.5.2), which is exactly the property
    /// under test — nothing here depends on the descendant's glyph data.
    fn composite_pdf(content: &str) -> Vec<u8> {
        build_pdf(
            content,
            &[(
                b"F1".to_vec(),
                b"<< /Type /Font /Subtype /Type0 /BaseFont /ABCDEF+NotoSans \
                   /Encoding /Identity-H /DescendantFonts [<< /Type /Font \
                   /Subtype /CIDFontType2 /BaseFont /ABCDEF+NotoSans \
                   /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
                   /DW 1000 >>] >>"
                    .to_vec(),
            )],
        )
    }

    /// The byte span of the LAST `Tj`/`TJ` operator token in `content` —
    /// the `GlyphProvenance::operator_span` convention
    /// (`pin_names_operator` accepts it), used to pin a run whose text this
    /// module's own walk cannot decode.
    fn tj_token_span(content: &str, op: &str) -> ByteSpan {
        let at = content
            .rfind(op)
            .expect("the content contains the operator");
        // `ByteSpan::new` takes (start, LENGTH), not (start, end).
        ByteSpan::new(at, op.len())
    }

    /// The headline: `Tw` is emitted for the matched run and RESTORED after
    /// it, so nothing following changes — the same leak gate `Tc`/`Tz`
    /// already pass, applied to the last member of the family.
    #[test]
    fn word_spacing_is_emitted_and_restored_around_the_run() {
        let src = fill_pdf("BT /F1 12 Tf 72 700 Td (a b) Tj 0 -14 Td (follower c d) Tj ET\n");
        let doc = Document::from_bytes(src.clone()).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "a b").word_spacing(MetricSpec::Absolute(2.5)),
            &FormatOptions::default(),
        )
        .unwrap();

        let appended = as_text(&out.bytes[src.len()..]);
        assert!(appended.contains("2.5 Tw"), "set emitted: {appended}");
        assert!(
            appended.contains("0 Tw"),
            "spec-default restore emitted (rung 1): {appended}"
        );
        assert_eq!(out.report.word_spacing_change, Some((0.0, 2.5)));
        // Two spaces in "follower c d" must be untouched: a reader walking
        // the SAVED file sees the ambient back at the Table 105 default.
        let after = ambient_after_reload(&out.bytes, "follower");
        assert_eq!(after.word_spacing.value, 0.0, "Tw leaked past the run");
        // …and the edited run itself really is at the new value.
        assert_eq!(
            ambient_after_reload(&out.bytes, "a b").word_spacing.value,
            2.5
        );
    }

    /// R89, `Tw` edition: `Absolute` is written exactly as typed at any
    /// size; `Relative` is thousandths of the em, re-derived against the
    /// run's BASE size (Amendment B.3) so a resize keeps its meaning.
    #[test]
    fn word_spacing_honours_both_units() {
        // Absolute: the same operand at two different sizes.
        for size in [12.0_f64, 30.0] {
            let src = fill_pdf(&format!("BT /F1 {size} Tf 72 700 Td (a b) Tj ET\n"));
            let doc = Document::from_bytes(src).unwrap();
            let out = set_format(
                &doc,
                &FormatRequest::new(0, "a b").word_spacing(MetricSpec::Absolute(1.75)),
                &FormatOptions::default(),
            )
            .unwrap();
            assert_eq!(
                out.report.word_spacing_change,
                Some((0.0, 1.75)),
                "an absolute Tw is written as typed at {size} pt"
            );
        }
        // Relative: 200 per-mille of the em scales with the size.
        for (size, expected) in [(12.0_f64, 2.4_f64), (30.0, 6.0)] {
            let src = fill_pdf(&format!("BT /F1 {size} Tf 72 700 Td (a b) Tj ET\n"));
            let doc = Document::from_bytes(src).unwrap();
            let out = set_format(
                &doc,
                &FormatRequest::new(0, "a b").word_spacing(MetricSpec::Relative(200.0)),
                &FormatOptions::default(),
            )
            .unwrap();
            let (_, emitted) = out.report.word_spacing_change.unwrap();
            assert!(
                (emitted - expected).abs() < 1e-12,
                "200 per-mille at {size} pt should be {expected}, got {emitted}"
            );
        }
    }

    /// **R91.** A composite (Type 0 / CIDFont) run refuses a word-spacing
    /// request BY NAME, with nothing applied — never a silent no-op, and
    /// never a `Tw` written into a file where §9.3.3 says it cannot apply.
    ///
    /// Pinned by `operator_span` rather than located by find text on
    /// purpose: `Walk::record_show` does not decode a composite run's
    /// string, so its `ShowData::text` is empty and a find-text lookup
    /// could never reach the anchor. Pinning is also exactly how the GUI
    /// addresses a run, so this is the path an operator's click takes.
    #[test]
    fn word_spacing_on_a_composite_run_is_refused_by_name() {
        let content = "BT /F1 12 Tf 72 700 Td <00410020004200430044> Tj ET\n";
        let src = composite_pdf(content);
        let doc = Document::from_bytes(src).unwrap();

        let mut req = FormatRequest::new(0, "irrelevant").word_spacing(MetricSpec::Absolute(2.0));
        req.pinned_span = Some(tj_token_span(content, "Tj"));
        let err = set_format(&doc, &req, &FormatOptions::default()).unwrap_err();

        match err {
            FormatError::WordSpacingComposite { ref base_font } => {
                assert!(
                    base_font.contains("NotoSans"),
                    "the refusal names the font: {base_font}"
                );
                let msg = err.to_string();
                assert!(msg.contains("§9.3.3"), "cite the clause: {msg}");
                assert!(
                    msg.contains("Nothing was applied"),
                    "say nothing happened: {msg}"
                );
                assert!(
                    msg.contains("TJ"),
                    "point at the mechanism that DOES work there: {msg}"
                );
            }
            other => panic!("expected the R91 composite refusal, got {other:?}"),
        }
    }

    /// The other half of the R91 gate: it fires **only** for word spacing.
    /// A composite run with some other formatting request must not acquire
    /// a word-spacing refusal it never asked for.
    #[test]
    fn the_composite_gate_fires_only_for_word_spacing() {
        let content = "BT /F1 12 Tf 72 700 Td <00410020004200430044> Tj ET\n";
        let src = composite_pdf(content);
        let doc = Document::from_bytes(src).unwrap();

        let mut req = FormatRequest::new(0, "irrelevant").char_spacing(MetricSpec::Absolute(0.5));
        req.pinned_span = Some(tj_token_span(content, "Tj"));
        let err = set_format(&doc, &req, &FormatOptions::default()).unwrap_err();
        assert!(
            !matches!(err, FormatError::WordSpacingComposite { .. }),
            "a Tc request must not be refused as a Tw one: {err:?}"
        );
    }

    /// Rung 2 of R88's ladder for `Tw`: an ambient set by its own `Tw`
    /// operator restores that operator's RAW BYTES, so a `1.500 Tw` does
    /// not come back renormalized as `1.5 Tw` (a diff in an operand pdfce
    /// did not logically change — R32/R46).
    #[test]
    fn word_spacing_rung_two_restores_the_raw_operand_bytes() {
        let src = fill_pdf("BT /F1 12 Tf 1.500 Tw 72 700 Td (a b) Tj (tail c) Tj ET\n");
        let doc = Document::from_bytes(src.clone()).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "a b").word_spacing(MetricSpec::Absolute(4.0)),
            &FormatOptions::default(),
        )
        .unwrap();
        let appended = as_text(&out.bytes[src.len()..]);
        assert!(appended.contains("4 Tw"), "set emitted: {appended}");
        assert!(
            appended.contains("1.500 Tw"),
            "the trailing-zero spelling must survive the restore verbatim: {appended}"
        );
        assert!(out.report.restore_narrowed.is_empty());
        assert_eq!(
            ambient_after_reload(&out.bytes, "tail").word_spacing.value,
            1.5
        );
    }

    /// **Rung 3 (`ObservedIndirect`) — the rung `Tw` is the headline case
    /// for.** `"` sets `Tw` *and* `Tc` **while showing a string** (§9.4.3
    /// Table 109), so replaying its bytes as a restore would repaint the
    /// text. The value is re-spelled as `2 Tw`, the narrowing is disclosed,
    /// and the shown string appears exactly once in the appended revision.
    #[test]
    fn word_spacing_rung_three_indirect_ambient_is_respelled_not_replayed() {
        let src = fill_pdf("BT /F1 12 Tf 14 TL 72 700 Td 2 0.25 (lead) \" (a b) Tj ET\n");
        let doc = Document::from_bytes(src.clone()).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "a b").word_spacing(MetricSpec::Absolute(5.0)),
            &FormatOptions::default(),
        )
        .unwrap();

        assert!(
            out.report
                .restore_narrowed
                .contains(&TextStateParam::WordSpacing),
            "an ObservedIndirect Tw restore must be reported as narrowed: {:?}",
            out.report.restore_narrowed
        );
        let appended = as_text(&out.bytes[src.len()..]);
        assert!(appended.contains("5 Tw"), "set emitted: {appended}");
        assert!(
            appended.contains("2 Tw"),
            "the `\"` operand must come back RE-SPELLED as its own operator: {appended}"
        );
        // The trap Amendment A.1 exists for: replaying the `"` bytes would
        // have painted "lead" a SECOND time.
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
        // A reader of the SAVED file agrees the ambient came back.
        assert_eq!(
            ambient_after_reload(&out.bytes, "a b").word_spacing.value,
            5.0
        );
    }

    /// Rung 4: an ambient `Tw` inherited through a form XObject is
    /// unrestorable, so the whole edit refuses rather than guessing `0 Tw`.
    /// Exercised at the ladder's application point for the same reason the
    /// 19.1 twin is — `text_edit::edit::Walk` never descends into a form,
    /// so the tier is not reachable end-to-end today.
    #[test]
    fn word_spacing_rung_four_form_xobject_ambient_refuses() {
        let mut ambient = AmbientTextState::initial();
        ambient.apply_operator(b"Tw", &[1.25], b"1.25 Tw");
        ambient.enter_form(Some(9));

        let mut set_ops = Vec::new();
        let mut restore_ops = Vec::new();
        let mut narrowed = Vec::new();
        let mut emitted = Vec::new();
        let err = push_state_param(
            &mut set_ops,
            &mut restore_ops,
            &ambient,
            TextStateParam::WordSpacing,
            3.0,
            &mut narrowed,
            &mut emitted,
        )
        .unwrap_err();
        match err {
            FormatError::AmbientUnrestorable(inner) => {
                let msg = inner.to_string();
                assert!(msg.contains("word spacing"), "{msg}");
                assert!(msg.contains("form XObject 9"), "{msg}");
            }
            other => panic!("expected an unrestorable-ambient refusal, got {other:?}"),
        }
        assert!(
            set_ops.is_empty() && restore_ops.is_empty() && emitted.is_empty(),
            "a refusal must leave NOTHING partially applied (rule 4)"
        );
    }

    /// A `Tw` change on a justified line reuses 19.1's disclose-and-offer
    /// path — and names the rivalry, because `Tw` and `TJ` slack are two
    /// mechanisms competing for the same inter-word space (decision 015
    /// §3.1).
    #[test]
    fn word_spacing_on_a_justified_line_discloses_and_offers_re_justify() {
        let src = fill_pdf("BT /F1 12 Tf 72 700 Td [(a b) -220 (wide) -220 (world)] TJ ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "a b").word_spacing(MetricSpec::Absolute(3.0)),
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
        assert!(
            d.contains("word-spacing change"),
            "the cause must be named as word spacing: {d}"
        );
        assert!(
            d.contains("competing"),
            "the Tw-vs-TJ rivalry must be stated: {d}"
        );
        assert!(
            d.contains("--align justified"),
            "the remedy must be actionable: {d}"
        );
    }

    /// The scope property that surprises people, asserted rather than only
    /// documented: `Tw` reaches EVERY code-32 in the run, and the report
    /// says how many. A run with none reports `Some(0)` — a real answer,
    /// not a suppressed operation.
    #[test]
    fn the_report_counts_every_space_the_word_spacing_reaches() {
        // Leading, doubled and trailing spaces — all four are code 32 and
        // all four are affected.
        let src = fill_pdf("BT /F1 12 Tf 72 700 Td ( a  b ) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, " a  b ").word_spacing(MetricSpec::Absolute(2.0)),
            &FormatOptions::default(),
        )
        .unwrap();
        assert_eq!(out.report.word_spacing_affected_codes, Some(4));
        let d = out
            .report
            .disclosures
            .iter()
            .find(|d| d.starts_with("word spacing:"))
            .expect("word spacing is disclosed");
        assert!(
            d.contains("ALL 4 spaces"),
            "the count must be quoted by value: {d}"
        );
        assert!(
            d.contains("leading spaces, trailing spaces and both halves of a doubled space"),
            "the scope must be stated plainly: {d}"
        );

        // A space-free run: the operator IS emitted (state was asked for),
        // the count is zero, and the disclosure says so rather than
        // implying something happened.
        let src = fill_pdf("BT /F1 12 Tf 72 700 Td (hello) Tj ET\n");
        let doc = Document::from_bytes(src.clone()).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "hello").word_spacing(MetricSpec::Absolute(2.0)),
            &FormatOptions::default(),
        )
        .unwrap();
        assert_eq!(out.report.word_spacing_affected_codes, Some(0));
        assert_eq!(out.report.advance_delta, 0.0, "no code 32 ⇒ no ΔA (§9.3.3)");
        let appended = as_text(&out.bytes[src.len()..]);
        assert!(appended.contains("2 Tw"), "still emitted: {appended}");
        assert!(
            out.report
                .disclosures
                .iter()
                .any(|d| d.contains("no visible effect here")),
            "a zero count must be said out loud"
        );
    }

    /// `Tw` is a term of §9.4.4, so it moves ΔA — but only through code-32
    /// glyphs, and it is multiplied by `Th`. Both halves asserted, because
    /// both are places the advance would silently drift if the emitted
    /// value were not fed into `A_new`.
    #[test]
    fn word_spacing_enters_the_advance_delta_and_scales_with_th() {
        let src = fill_pdf("BT /F1 12 Tf 72 700 Td (a b) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let plain = set_format(
            &doc,
            &FormatRequest::new(0, "a b").word_spacing(MetricSpec::Absolute(2.0)),
            &FormatOptions::default(),
        )
        .unwrap();
        // One space, Th = 1 ⇒ ΔA is exactly the operand.
        assert!((plain.report.advance_delta - 2.0).abs() < 1e-9);

        // The same Tw under a 50% horizontal scale delivers half of it —
        // §9.3.4: Th "shall also affect the spacing parameters Tc and Tw".
        let scaled = set_format(
            &doc,
            &FormatRequest::new(0, "a b")
                .word_spacing(MetricSpec::Absolute(2.0))
                .h_scale(50.0),
            &FormatOptions::default(),
        )
        .unwrap();
        let d = scaled
            .report
            .disclosures
            .iter()
            .find(|d| d.starts_with("word spacing:"))
            .expect("word spacing is disclosed");
        assert!(
            d.contains("VISIBLE extra gap is 1"),
            "the Th-scaled effective gap must be quoted: {d}"
        );
    }

    /// Minimal diff: a `Tw` already in force emits neither a set nor a
    /// restore, and the request is still reported.
    #[test]
    fn a_word_spacing_already_in_force_emits_no_operator() {
        let src = fill_pdf("BT /F1 12 Tf 3 Tw 72 700 Td (a b) Tj ET\n");
        let doc = Document::from_bytes(src.clone()).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "a b").word_spacing(MetricSpec::Absolute(3.0)),
            &FormatOptions::default(),
        )
        .unwrap();
        let appended = as_text(&out.bytes[src.len()..]);
        assert_eq!(
            appended.matches(" Tw").count(),
            1,
            "only the producer's own re-emitted `3 Tw` should be present: {appended}"
        );
        assert_eq!(out.report.word_spacing_change, Some((3.0, 3.0)));
        assert_eq!(out.report.advance_delta, 0.0);
    }

    /// `Tw` composes with the rest of the family through one save/reload:
    /// every control lands on the edited run and none of them survives past
    /// it.
    #[test]
    fn word_spacing_composes_with_the_other_controls_and_round_trips() {
        let src =
            fill_pdf("BT /F1 12 Tf 0.25 Tc 1 Tw 110 Tz 72 700 Td (x y) Tj (a b) Tj (p q) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        let out = set_format(
            &doc,
            &FormatRequest::new(0, "a b")
                .char_spacing(MetricSpec::Absolute(0.75))
                .word_spacing(MetricSpec::Absolute(4.5))
                .h_scale(85.0),
            &FormatOptions::default(),
        )
        .unwrap();

        let edited = ambient_after_reload(&out.bytes, "a b");
        assert_eq!(edited.char_spacing.value, 0.75);
        assert_eq!(edited.word_spacing.value, 4.5);
        assert_eq!(edited.h_scale.value, 85.0);

        for neighbour in ["x y", "p q"] {
            let ts = ambient_after_reload(&out.bytes, neighbour);
            assert_eq!(ts.char_spacing.value, 0.25, "{neighbour}: Tc not restored");
            assert_eq!(ts.word_spacing.value, 1.0, "{neighbour}: Tw not restored");
            assert_eq!(ts.h_scale.value, 110.0, "{neighbour}: Tz not restored");
        }
    }

    /// Word spacing on its own is a real formatting operation (not a
    /// no-op), and its absence still leaves an empty request empty.
    #[test]
    fn word_spacing_alone_counts_as_a_formatting_operation() {
        let src = fill_pdf("BT /F1 12 Tf 72 700 Td (a b) Tj ET\n");
        let doc = Document::from_bytes(src).unwrap();
        set_format(
            &doc,
            &FormatRequest::new(0, "a b").word_spacing(MetricSpec::Absolute(1.0)),
            &FormatOptions::default(),
        )
        .expect("word spacing alone is a formatting operation");
        assert!(matches!(
            set_format(
                &doc,
                &FormatRequest::new(0, "a b"),
                &FormatOptions::default()
            ),
            Err(FormatError::NoOp)
        ));
    }
}
