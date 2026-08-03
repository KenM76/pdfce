//! # In-place text FORMATTING surgery (Pass 14.2)
//!
//! This module changes the **size**, **fill colour**, and
//! **font-family/style** of text already on a page — in place, minimal-diff
//! — by reusing Pass 14.1's advance-preserving content-stream surgery
//! (`crate::text_edit::edit`) with different operators changed. It is the
//! third slice of decision 014's Acrobat-text-editing family
//! (`docs/decisions/014-acrobat-text-editing.md` §3 "Formatting", §5.2's
//! "13.2"; R32/R34/R47/R69/R70/R72).
//!
//! ## The three operations, and what each rewrites
//!
//! | Op | Operator changed | Advance affected? | Font gate? |
//! |---|---|---|---|
//! | **size** | `Tf` size operand | yes (ΔA) | no — never needs new glyphs |
//! | **fill colour** | `rg`/`g`/`k` (the chosen space) | no | no |
//! | **font family/style** | `Tf` font resource | yes (ΔA) | YES — re-encode + coverage gate |
//!
//! All three are the SAME surgery as 14.1 REPLACE — locate the anchor show
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
use crate::writer::content::emit_number;

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

    /// Whether any formatting operation was requested.
    #[must_use]
    const fn is_empty(&self) -> bool {
        self.set_size.is_none() && self.set_fill.is_none() && self.set_font.is_none()
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
    /// No formatting operation was requested (`--set-size`/`--set-color`/
    /// `--set-font` all absent).
    #[error("no formatting operation was requested (need one of size, colour, or font)")]
    NoOp,
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

    let new_size = req.set_size.unwrap_or(orig_size);
    let new_codes: Vec<u8> = match &font_plan {
        Some(plan) => plan.new_codes.clone(),
        None => m.old_codes.clone(),
    };
    let advance_font: &ExtractFont = font_plan.as_ref().map_or(&orig_font, |p| &p.font);

    // --- advance delta (§9.4.4): only size and font change it ---
    let a_old: f64 = m
        .old_codes
        .iter()
        .map(|&c| glyph_advance_with(&orig_font, c, orig_size, anchor.tc, anchor.tw, anchor.th))
        .sum();
    let a_new: f64 = new_codes
        .iter()
        .map(|&c| glyph_advance_with(advance_font, c, new_size, anchor.tc, anchor.tw, anchor.th))
        .sum();
    let delta = a_new - a_old;

    // --- build the state-set / state-restore operator sequences ---
    let size_changed = req.set_size.is_some();
    let font_changed = font_plan.is_some();
    let tf_touched = size_changed || font_changed;

    let set_font_name: &[u8] = font_plan
        .as_ref()
        .map_or(anchor.font_name.as_slice(), |p| p.resource.as_slice());

    let mut set_ops: Vec<u8> = Vec::new();
    let mut restore_ops: Vec<u8> = Vec::new();
    if tf_touched {
        push_tf(&mut set_ops, set_font_name, new_size);
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
                if let Some(n) = compensating_tj(delta, orig_size, anchor.th)
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
        disclosures.push(disclosure_size(orig_size, new_size));
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
        size_change: size_changed.then_some((orig_size, new_size)),
        fill_space: req.set_fill.as_ref().map(|nf| nf.model.operator()),
        fill_narrowed,
        font_change: font_plan
            .as_ref()
            .map(|p| (orig_font.base_font.clone(), p.font.base_font.clone())),
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
}
