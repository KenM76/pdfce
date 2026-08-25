//! # Content-stream interpreter → tiny-skia rasterization (Pass 1 slice)
//!
//! Executes the semantic projection of `pdfce_core::content` against a
//! [`tiny_skia::Pixmap`]. Spec sources: `iso32000__s__8.2.md` (operator
//! categories/state machine), `iso32000__s__8.3.md` (CTM, row-vector
//! convention), `iso32000__s__8.4.md`/`8.4.3.md` (graphics state,
//! caps/joins/dash), `iso32000__s__8.5.md` (paths, painting, clipping),
//! `iso32000__s__8.6.md` (device colours), `iso32000__s__7.8.md`
//! (operand rules, `BX`/`EX`) in the PDF-spec RAG.
//!
//! ## Pass 1 first-slice coverage
//!
//! Implemented: `q`/`Q`, `cm`, line-state ops (`w J j M d i ri`),
//! device colours (`g G rg RG k K`), all Table 59 construction ops
//! (`m l c v y h re`), all Table 60 painting ops
//! (`S s f F f* B B* b b* n`), clipping (`W W*` with the deferred-
//! application rule), `gs` (the LW/LC/LJ/ML/D subset of Table 58),
//! `BX`/`EX` compatibility sections, and the **complete text operator
//! set** — `BT`/`ET`, the seven text-state operators
//! (`Tc Tw Tz TL Tf Tr Ts`), all four positioning operators
//! (`Td TD Tm T*`) and all four showing operators (`Tj TJ ' "`).
//! Text spec sources: `iso32000__s__9.3.md`, `iso32000__s__9.4.md`,
//! `iso32000__s__9.6.6.md`, `iso32000__s__9.7.md`; scope per
//! `docs/decisions/004-text-rendering-fonts.md` §4.3. The glyph
//! machinery itself lives in [`crate::text`] and [`crate::font`]; this
//! module owns the operator dispatch and the painting.
//!
//! Also implemented: the **`Do` operator and inline images** — form
//! XObjects (§8.10's five-step procedure), image XObjects (§8.9), and
//! `BI`/`ID`/`EI` (§8.9.7), all through [`crate::image`]. Spec sources:
//! `iso32000__s__8.8.md` (dispatch on `/Subtype`, Table 87),
//! `iso32000__s__8.10.md` (Table 95, form space, `BBox` clipping,
//! `Resources` scoping), `iso32000__s__8.9.md` (Table 89, the
//! unit-square mapping), `iso32000__s__7.9.5.md` (rectangle corner
//! normalization).
//!
//! Also implemented: the **full §8.6 colour-space set** — `cs`/`CS` and
//! `sc`/`scn`/`SC`/`SCN` over `DeviceGray`/`DeviceRGB`/`DeviceCMYK`,
//! `CalGray`, `CalRGB`, `Lab`, `ICCBased` (through §8.6.5.5's own
//! `/Alternate`-or-`/N` fallback), `Indexed`, `Separation` and `DeviceN`
//! (parsed, with the tint transform **disclosed as unevaluated**), and
//! `Pattern` (recognised, unpainted, counted). The model lives in
//! [`crate::color`]; this module owns only the operator arms and the
//! paint-suppression check. Until 2026-08-10 those six operators were
//! deferred, which meant a stream that selected a space and set a colour
//! painted in whatever colour was previously in force — see
//! [`crate::color`]'s module docs for why that was worse than a gap.
//!
//! Recognized-but-deferred (counted in [`Diagnostics`], never silent —
//! "fuzzy, never sneaky"): shading (`sh`), marked content, Type 3 glyph procedures
//! (`d0`/`d1`), and text **clipping** modes `Tr` 4–7 (their fill/stroke
//! half is painted; the clip is not applied). Unknown operators outside
//! `BX`/`EX` are — per the RAG's tolerance note — logged and skipped
//! rather than hard-failing the page (§7.8.2 calls them an error; a
//! viewer that abandons a page over one is conformant but useless; the
//! diagnostic keeps divergence visible).
//!
//! ## `Do` on a form is the interpreter's only recursion
//!
//! §8.10's procedure is *"save state, concat `/Matrix`, clip to
//! `/BBox`, run the form's content stream with the form's own
//! `/Resources`, restore state"* — i.e. this module calling itself.
//! Three guards make that safe (ARCHITECTURE.md §10.1):
//!
//! 1. **[`MAX_XOBJECT_DEPTH`]** bounds nesting.
//! 2. **A cycle set keyed on the XObject's object number**, not its
//!    resource name — the same stream can be reached under different
//!    names, so name-keyed tracking would miss `/A Do` → `/B Do` → the
//!    same object.
//! 3. The nested run gets a **fresh [`Interpreter`]** over a *clone* of
//!    the current graphics state, so steps (a) and (e) are structural:
//!    an unbalanced `Q` inside a form cannot corrupt the caller's
//!    stack, and the form's own state changes simply die with its
//!    interpreter.
//!
//! That fresh interpreter also starts with `text: None`, which is how
//! §9.4.1's "`Tm`/`Tlm` belong to one `BT`…`ET`" is honoured across the
//! boundary: a form invoked *inside* a caller's text object (ill-formed
//! per §8.2's Figure 9, common in the wild) neither sees nor moves the
//! caller's pen, and a form containing its own `BT`…`ET` works
//! normally. The text *state* (font, size, spacing — §9.3 graphics-state
//! parameters) IS inherited, because §8.10.1 says the form's initial
//! graphics state is the caller's.
//!
//! Each form also gets its **own font cache**, which is a correctness
//! requirement rather than an optimization detail: the cache is keyed by
//! resource *name*, and `/F1` in a form's `/Resources` is a different
//! font from `/F1` on the page.
//!
//! ## Correctness details this module owes to the spec
//!
//! - **`cm` PRE-multiplies** (`CTM′ = M × CTM`, row-vector convention,
//!   §8.3.4) — the classic works-on-translations, breaks-on-rotations
//!   bug lives here; pinned by a test.
//! - **`W`/`W*` are deferred**: they mark the pending path; the paint
//!   op paints under the OLD clip, and only afterwards does the clip
//!   tighten (§8.5.4 verbatim rule).
//! - **`f` implicitly closes; `S` does not** (§8.5.3) — tiny-skia's
//!   fill already treats contours as implicitly closed, and stroking
//!   leaves them open, matching exactly; the close-variants (`s b b*`)
//!   close explicitly first.
//! - **The path lives in user space** and is painted through the CTM
//!   captured at the path's FIRST construction op. A `cm` in the
//!   middle of path construction (legal, vanishingly rare) is
//!   diagnosed and approximated with that first CTM — a documented
//!   Pass 1 simplification.
//! - **Stroke geometry is computed in user space** (width, dash, caps)
//!   and transformed to device space afterwards — exactly PDF's model
//!   (§8.4.3.2 "line width in user space units"), and exactly what
//!   `tiny_skia::Pixmap::stroke_path(path, …, transform, …)` does.
//!   Glyph outlines take the same route for the same reason: §9.3.6
//!   says stroked text's line width "shall be interpreted in USER SPACE
//!   rather than in text space", so a glyph path is transformed to user
//!   space and the CTM is passed separately.
//! - **Text state is graphics state; the text matrices are not.** §9.3
//!   puts `Tc Tw Th Tl Tf Tfs Tmode Trise` in the graphics state, so
//!   `q`/`Q` save and restore them; §9.4.1 confines `Tm`/`Tlm` to one
//!   `BT`…`ET`, so they live in [`Interpreter::text`] and a `q`/`Q`
//!   pair inside a text object leaves the pen where it was.
//! - **Every glyph advances, even the ones that paint nothing** —
//!   rendering mode 3 (the invisible OCR text layer), a `.notdef`
//!   fallback, and a space all move `Tm` (§9.4.4).

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use pdfce_core::content::{ContentStream, ContentTokenKind, Operation};
// decision 018: read paths take a `DocumentView` (graph + byte source), so
// the same code renders a loaded file or an editing session's unsaved state.
use pdfce_core::filters;
use pdfce_core::graph::ObjectGraph;
use pdfce_core::object::{Dict, ObjId, Object, Stream};
use pdfce_core::span::ByteSpan;
use pdfce_core::view::DocumentView;
use tiny_skia::{
    FillRule, FilterQuality, LineCap as SkCap, LineJoin as SkJoin, Mask, Path, PathBuilder, Pixmap,
    Rect, Stroke, StrokeDash, Transform,
};

use crate::cancel::RenderCancel;
use crate::canvas::{BrushSpec, Canvas, ClipRef, LayerPaint};
use crate::display_list::{ClipDef, PoisonReason};
use crate::font::program::FontProgram;
use crate::font::{FontEnvironment, RenderPolicy};
use crate::gstate::{GStateStack, GraphicsState, LineCap, LineJoin, Mat64, Rgb};
use crate::image::{self, ImageError, ImageNotes, ImageOrigin};
use crate::text::{LoadedFont, TextObject};
use pdfce_core::settings::MinifyFilter;

/// Maximum nesting of `Do`-invoked form XObjects (pdfce policy,
/// ARCHITECTURE.md §10.1).
///
/// **There is no spec limit to inherit here.** Annex C (itself
/// informative) lists no form-XObject nesting bound at all, and PDF/A
/// §6.1.12 positively requires a reader *not* to impose Annex C's
/// implementation limits — so this number is pure pdfce policy and has
/// to be justified on its own.
///
/// It is set from corpus measurement rather than intuition. Ordinary
/// documents nest two or three deep (a page invokes a template, which
/// invokes a logo, whose annotation appearance stream invokes a shared
/// field appearance), which is what makes a small value *look* safe.
/// But veraPDF's PDF/A-1b §6.1.12 implementation-limits suite contains
/// `6-1-12-t08-pass-*.pdf`, a **conformant** file with a deliberate
/// chain of **32** nested form XObjects — a reader that refuses it is
/// wrong, in exactly the way the 8 KiB `MAX_TOKEN_LEN` guard was wrong
/// against `6-1-12-t02-pass-k.pdf`. 64 is 2× the deepest conformant
/// structure in the corpus.
///
/// This bound is a backstop, not the real defence: the attack it would
/// have to stop is unbounded *recursion*, and that is caught by
/// [`Interpreter::active`]'s cycle set at any depth. What this value
/// actually bounds is the linear memory a legitimate-but-absurd chain
/// can pin — one cloned [`GraphicsState`] (including its page-sized
/// clip mask) per live level — and 64 keeps that comfortably below the
/// 256 levels [`crate::gstate::MAX_Q_DEPTH`] already permits per level.
pub const MAX_XOBJECT_DEPTH: usize = 64;

/// The width, in device pixels, below which
/// [`RenderOptions::subpixel_culling`] considers a form invisible.
///
/// Half a pixel, in BOTH axes, so a form has to be smaller than the
/// sampling grid in every direction before it is dropped — a thin but
/// long form still paints, because a hairline is exactly the kind of
/// thing an operator would notice missing.
///
/// It is a threshold on the OBJECT, not on its contribution: a form this
/// small can still tint the pixel it sits in, and dropping it is a
/// visible change on a page carrying hundreds of them. That is why the
/// option defaults off; the constant only says where "too small to see"
/// begins once the operator has accepted the trade.
pub const SUBPIXEL_CULL_PX: f32 = 0.5;

/// Cap on the number of distinct sample strings retained in
/// [`Diagnostics::sample_ops`] / [`Diagnostics::image_notes`].
///
/// Diagnostics are shown to a human and shipped in a CLI batch report;
/// an unbounded list from a hostile page is both useless and an
/// allocation vector.
const MAX_SAMPLES: usize = 12;

/// Diagnostics from interpreting one page — every divergence from
/// full rendering is COUNTED here, never silently absorbed
/// ("fuzzy, never sneaky": the operator can see exactly how honest
/// the raster is).
#[derive(Debug, Default, Clone)]
pub struct Diagnostics {
    /// `Contents` entries on this page that named an object the file does
    /// not contain, and so contributed no content stream to the raster
    /// (mirrors [`pdfce_core::page_tree::Page::contents_unresolved`]).
    ///
    /// This is the one diagnostic here that is decided **before**
    /// interpretation starts — it is a property of the page dictionary,
    /// not of any operator — so it is copied in by the render entry point
    /// rather than accumulated by the interpreter. It belongs in this
    /// struct anyway, because from the operator's side it is the same kind
    /// of fact as an unsupported image: the raster is *incomplete*, not
    /// *wrong*, and a page that comes out emptier than expected has a
    /// named reason (§7.3.10 + Table 30 — a dangling reference is the null
    /// object, and an absent `Contents` is an empty page).
    pub contents_streams_unresolved: usize,
    /// Operators recognized but not yet implemented (XObjects,
    /// shading, marked content, Type 3 glyph procedures, and `Tr`'s
    /// clipping modes 4–7), with occurrence counts folded into one
    /// number.
    pub deferred_ops: usize,
    /// `/OC` marked-content sections that were HIDDEN, and so not drawn
    /// (§8.11.3.2).
    ///
    /// Counted per section entered, not per operator suppressed, because
    /// the question an operator asks is "is something on this page not
    /// being shown?" — and one section can hide a whole drawing. A
    /// non-zero value with a page that looks empty is the difference
    /// between a layer turned off and a render that failed, which is
    /// exactly the distinction the diagnostics exist to make.
    pub oc_sections_hidden: usize,
    /// Operators not recognized at all (outside `BX`/`EX`).
    pub unknown_ops: usize,
    /// Operators skipped inside `BX`/`EX` compatibility sections
    /// (spec-sanctioned skips, §7.8.2 Table 32).
    pub compat_skipped: usize,
    /// `gs` operators that selected a **non-Normal blend mode** (`/BM`,
    /// ISO 32000-1 §11.3.5) which pdfce APPLIED.
    ///
    /// **★ "Applied" is not "applied correctly", and this doc said "A
    /// census, not a shortfall" until 2026-08-18.** **§11.3.4** requires
    /// blending in the **group's colour space**, with subtractive
    /// components complemented before and after
    /// (`blend_subtractive(cb, cs) = 1 − B(1 − cb, 1 − cs)`).
    ///
    /// ★ **Corrected 2026-08-21 (`Pass 97.1e`/`97.1f`): this said "pdfce
    /// blends in device sRGB, so a CMYK group's blends are computed in the
    /// wrong space. `Pass 97.1` is the fix", in the future tense, and it
    /// is now the past.** A page whose group declares a subtractive
    /// blending space composites in a colorant buffer, and the blends
    /// inside it go through §11.3.4's complement. What remains true is the
    /// distinction this paragraph was written to make: **applied is still
    /// not applied correctly** — an *additive* page's blends are computed
    /// in device sRGB, which is what §8.6.6.4 specifies for an additive
    /// device, and a mode this counter records as applied can still be
    /// wrong for reasons that have nothing to do with the space.
    ///
    /// **Measured evidence, and it is one cell rather than a cluster:**
    /// Ghent `3_GWG164` (ICCBased CMYK) fails its **`Difference`** cell —
    /// an applied, separable mode. `Difference` is `|cb − cs|`, the mode
    /// most sensitive to whether its operands were complemented first, so
    /// it is exactly where a wrong blending space shows up soonest. The
    /// cell was identified by resolving the form XObject's `/Matrix` and
    /// `/BBox` into device space against the governing `/ExtGState`, not
    /// by cell-pitch arithmetic.
    ///
    /// **★ CORRECTION, 2026-08-18.** An earlier revision of this doc also
    /// blamed §11.3.5.3 here and cited `1_GWG160`'s `Hue`/`Saturation`/
    /// `Color` failures as proof. Those did not reach this counter: at the
    /// time [`crate::gstate::blend_mode_from_name`] returned `None` for all
    /// four nonseparable modes, so they landed in
    /// [`Self::blend_modes_ignored`]. Caught by the librarian on filing.
    /// The evidence was real and the counter it was attached to was wrong,
    /// which is the harder error to notice: a citation makes a claim look
    /// checked.
    ///
    /// **★★ AND THAT CORRECTION IS ITSELF SUPERSEDED, 2026-08-19.** It said
    /// *"**Those cannot reach this counter**"* in the present tense, and
    /// they can now: the four nonseparable modes ship
    /// ([`crate::blend_nonsep`]), `blend_mode_from_name` is no longer the
    /// route they take, and they DO increment this counter — with their
    /// composites additionally counted on
    /// [`Self::nonseparable_composited`]. `1_GWG160` now passes.
    ///
    /// Two corrections deep on one paragraph, so the shape is worth naming
    /// rather than just fixing again: **a correction written in the present
    /// tense about a mechanism becomes false when the mechanism changes,
    /// and it is MORE durable than the error it fixed**, because it reads
    /// as settled. Dating each layer is what keeps the third reader from
    /// trusting the second.
    ///
    /// Separate from [`Self::blend_modes_ignored`] because those two
    /// numbers answer different questions and used to be one. Before
    /// blend modes were implemented, "ignored" and "used" were the same
    /// quantity; implementing them made that reading false, and silently
    /// redefining the counter would have left an operator diffing two runs
    /// unable to tell which number moved or why.
    pub blend_modes_applied: usize,
    /// `gs` operators naming a blend mode pdfce did NOT apply. Those marks
    /// were composited as `Normal`.
    ///
    /// **One reason reaches this counter**: a `/BM` name outside ISO
    /// 32000-1 Tables 136 and 137 — a typo, or a mode from an extension
    /// pdfce does not know. Those marks composite as `Normal`.
    ///
    /// Counted because the result is WRONG rather than missing, and
    /// wrongness of this shape is invisible: a blend that composited as
    /// Normal produces a perfectly ordinary-looking opaque overlay. A
    /// missing image leaves a hole somebody notices; a wrong compositing
    /// rule just looks like a different document.
    ///
    /// # ★ THIS COUNTER USED TO MEAN TWO THINGS, and the second is gone
    ///
    /// Until 2026-08-19 it also counted the four **non-separable** modes of
    /// Table 137 (`Hue`, `Saturation`, `Color`, `Luminosity`), which pdfce
    /// recognised and **deliberately declined** to composite because the
    /// rasteriser's implementations are measurably wrong. This block said
    /// *"Reason 2 is by far the likelier on a real print-oriented file"* —
    /// and that was true right up until those modes shipped.
    ///
    /// They now render, computed by pdfce against the clause
    /// ([`crate::blend_nonsep`]) and counted on
    /// [`Self::nonseparable_composited`]. **A non-zero value here no longer
    /// implicates them.**
    ///
    /// # ★★ AND THIS BLOCK NAMED A BLOCKER THAT SHIPPING FALSIFIED
    ///
    /// It closed: *"Implementing them needs Table 137's formulas **and**
    /// §11.3.5.3's CMYK detour, in which K is selected by mode rather than
    /// blended … Both need a backdrop this crate can read, which is
    /// `Pass 97.0`'s buffer."*
    ///
    /// **They shipped with no CMYK buffer and no `Pass 97.0`.** The backdrop
    /// a non-separable blend needs is the destination pixmap, which
    /// `Pass 85.5` had already made readable per paint — the same machinery
    /// overprint composites through. The CMYK detour governs a `DeviceCMYK`
    /// *blending colour space*, which is a separate obligation and remains
    /// `Pass 97.x`'s.
    ///
    /// Worth leaving on the page rather than deleting, because the shape
    /// recurs: a blocker recorded once, in a doc comment, outlives the
    /// design that motivated it and then argues against work that has
    /// become cheap. The `Pass 85.5` row carried the same defect — *"gated
    /// on `iccce`"* — and was falsified the same way, by someone trying it.
    ///
    /// The per-cell trap table that used to sit here (`1_GWG160` and
    /// `3_GWG164`, three of four modes trapping) is **deleted rather than
    /// corrected**: `1_GWG160` now PASSES, so a table of which of its cells
    /// trap describes a document state that no longer exists.
    pub blend_modes_ignored: usize,
    /// Form XObjects carrying `/Group << /S /Transparency >>` (§11.4.7,
    /// Table 147) that pdfce painted **straight onto the page** instead of
    /// compositing as a unit.
    ///
    /// # Why this is its own counter and not folded into the blend one
    ///
    /// A transparency group is a *compositing scope*: its contents are
    /// rendered into a separate buffer, and the group's RESULT is then
    /// composited onto the backdrop with the blend mode, alpha and soft
    /// mask in force at the `Do`. Painting the contents directly applies
    /// those to each object INSIDE the group instead — which gives the
    /// same answer for a group holding one opaque object and a different
    /// answer for almost anything else.
    ///
    /// That distinction is invisible in every other counter. It was found
    /// only by rendering the operator's Ghent X-4 file and seeing its
    /// blend-mode panel still show the suite's failure crosses AFTER blend
    /// modes were implemented and verified correct both in isolation and
    /// against a coloured backdrop. The page carries 148 form XObjects;
    /// `/Group` was never read.
    ///
    /// Non-zero here means the page's transparency is approximate in a way
    /// no blend-mode counter can express.
    pub transparency_groups_flattened: usize,
    /// `gs` operators that turned OVERPRINT on (`/OP` or `/op`, §8.6.7).
    ///
    /// ★ This sentence ended "**while pdfce does not simulate it**" while
    /// the section immediately below it corrected that at length. A head
    /// sentence is what a hover tooltip and a doc-search snippet show, so
    /// the correction was invisible exactly where the claim was loudest.
    ///
    /// # Why this is counted, and what it does NOT mean
    ///
    /// **★ This heading used to read "Why this is counted and not applied",
    /// and the section under it argued that pdfce "composites in additive
    /// RGB — so on the shipped path there is no per-colorant state for
    /// overprint to preserve". `bf75351` made that false and did not
    /// revise it.** Fourth copy of the same stale narrative found on
    /// 2026-08-18; the others were the stdout note, the comment beside its
    /// format arguments, and [`crate::gstate::GraphicsState::overprint_mode`].
    /// One shortfall, written in four places, corrected in one.
    ///
    /// Overprint is a SUBTRACTIVE-device behaviour: an overprinting object
    /// leaves the backdrop's other colorants in place instead of replacing
    /// them, which only means anything if the device HAS separable
    /// colorants. pdfce gives it some, per pixel, by reconstructing CMYK
    /// from the raster, applying `CompatibleOverprint` (§11.7.4.3, Table
    /// 149) and converting back — an exact round trip by construction, so
    /// only the components Table 149 actually alters can move.
    ///
    /// ISO 32000-1 never describes overprint PREVIEW on a non-separating
    /// device (`overprint preview`: 0 hits in the 756-page source), so
    /// simulating it is a product decision rather than a conformance
    /// obligation. Acrobat enables Overprint Preview automatically for
    /// PDF/X files, so a PDF/X-4 document's EXPECTED appearance includes it.
    ///
    /// **What is still missing**, and why this counter is not a success
    /// measure: the four PROCESS colorants survive, but a SPOT colorant has
    /// no plane of its own and is flattened through its tint transform, so
    /// it cannot be left standing the way a press leaves it. Image
    /// XObjects do not reach the overprint composite at all — they are
    /// **counted** now, on [`Self::overprint_images_unsupported`], which
    /// this sentence said they were not until `Pass 97.1b`. Both remain
    /// `Pass 97.1`'s n-channel compositing buffer to FIX; only the
    /// blind-counter half is closed.
    ///
    /// ★ Note how that stale half survived a sweep: it named
    /// `overprint_refused` in order to say the situation did **not** reach
    /// it, so a grep for the *new* counter could not find it and a grep
    /// for the old one found a sentence that looked deliberate. A claim
    /// phrased as an absence is invisible to a search for the thing that
    /// now exists.
    pub overprint_requested: usize,
    /// Of those, the ones that also selected **overprint mode 1** (`/OPM 1`,
    /// the "nonzero overprint mode").
    ///
    /// Separated because mode 1 is where overprint stops being a
    /// component-set question and becomes a per-component VALUE question:
    /// a zero DeviceCMYK component leaves the backdrop unchanged. It is
    /// also the mode §8.6.7 explicitly makes inert off DeviceCMYK, so a
    /// non-zero count here is the clearest signal that a document expects
    /// an ink model pdfce is not providing.
    pub overprint_mode1_requested: usize,
    /// Paints made while overprint was ON **and where honouring it would
    /// have changed the result** — the subset of `overprint_requested` that
    /// is a real visual difference rather than a no-op.
    ///
    /// # Why the distinction is the whole point of counting
    ///
    /// Overprint is enabled far more often than it matters. §11.7.4.3's
    /// `CompatibleOverprint` picks the SOURCE component for every component
    /// the current colour space specifies and the BACKDROP component for
    /// the rest — so a DeviceCMYK fill over a DeviceCMYK backdrop at
    /// overprint mode 0 specifies all four components, selects the source
    /// for all four, and is **identical to Normal**. Producers set `/OP
    /// true` across whole documents as a default; most of those paints are
    /// that case.
    ///
    /// What is NOT a no-op is a source space that specifies FEWER
    /// components than the backdrop has — a `Separation` or a one-component
    /// `DeviceN` over CMYK, where the unspecified process components must
    /// survive — or overprint mode 1, where a zero-valued component leaves
    /// the backdrop alone.
    ///
    /// So this counter, not `overprint_requested`, is the honest measure of
    /// which paints need `CompatibleOverprint` rather than `Normal`.
    ///
    /// **Amended when overprint simulation shipped.** This doc previously
    /// ended "…and it is the number that says whether the n-channel buffer
    /// is worth building for a given document", which was true while the
    /// counter was purely diagnostic. It now counts the paints that ARE
    /// composited through Table 149 rather than the ones that are missed,
    /// so `overprint_effective` and `overprint_composited` should agree
    /// except where a composite was refused. A disagreement is the signal
    /// worth chasing.
    pub overprint_effective: usize,
    /// Paints actually composited through `CompatibleOverprint`
    /// (§11.7.4.3, Table 149) rather than the ordinary `Normal` blend.
    ///
    /// Should equal `overprint_effective` minus `overprint_refused`. Kept
    /// as its own counter rather than derived, because a derived number
    /// cannot disagree with reality and therefore cannot report a bug.
    pub overprint_composited: usize,
    /// Paints composited through one of §11.3.5.3's four **non-separable**
    /// blend modes (`Hue`/`Saturation`/`Color`/`Luminosity`).
    ///
    /// Counted separately from [`Self::blend_modes_applied`] because these
    /// four take a **different code path** — pdfce computes Table 137 per
    /// pixel rather than handing the mode to the rasteriser, whose
    /// implementations are measurably wrong (`ARCHITECTURE.md` §12 decision
    /// 066). A single counter would hide which of the two paths a page
    /// actually exercised, and they can fail independently.
    pub nonseparable_composited: usize,
    /// Pixels changed by those composites.
    ///
    /// The companion to the count, for the same reason `overprint_pixels`
    /// exists: a composite that ran on zero pixels and one that repainted a
    /// swatch are both "1" on the counter above, and only this distinguishes
    /// them.
    pub nonseparable_pixels: u64,
    /// Paints where overprint applied but the composite could not run, so
    /// the paint fell back to a normal blend.
    ///
    /// **Non-zero means the operator is seeing knocked-out backdrops where
    /// a press would show overprinted ink.** Disclosed rather than silently
    /// tolerated: rule 4's whole point is that an inference or a shortfall
    /// the operator cannot see by looking is the kind that must be said out
    /// loud.
    pub overprint_refused: usize,
    /// **Images painted while overprint was in force**, which
    /// `CompatibleOverprint` was never offered.
    ///
    /// # Why this is a separate counter and not more `overprint_refused`
    ///
    /// Because they are different failures and a reader acts on them
    /// differently. `overprint_refused` means *"the composite was offered
    /// this paint and could not run it"* — a recording canvas, a
    /// degenerate region, a mask that would not allocate. This one means
    /// *"the composite was never offered this object at all"*, because
    /// `overprint::composite` has exactly one call site, in the path and
    /// glyph painter, and an image XObject does not reach it.
    ///
    /// Widening `overprint_refused` to cover both would have made a
    /// whole missing object class look like a run of ordinary failures,
    /// and would have moved a number an operator may already be diffing
    /// between runs.
    ///
    /// # Why it cannot be fixed where it is counted
    ///
    /// Per-sample overprint needs per-sample colorants, and pdfce's buffer
    /// is device sRGB — the image's own colorant identity is gone by the
    /// time its texels are composited. That is `Pass 97.1`'s colorant
    /// buffer, not a call-site change. Counting it first is deliberate:
    /// **a counter blind to a whole object class reports a smaller problem
    /// than exists**, which this project has already paid for once in the
    /// glyph painter (`bf75351`).
    ///
    /// ★ It is also what makes the `/Indexed` classification fix
    /// measurable. Ghent's `1_GWG190`, `1_GWG191`, `1_GWG192` and
    /// `2_GWG020` all carry `/Indexed [/DeviceN …]` spaces and every one
    /// of them uses that space **only** for an image — so
    /// `ColorSpace::indexed_entry` is correct, cited, and **inert on the
    /// whole corpus** until this number can go down.
    pub overprint_images_unsupported: usize,
    /// Transparency groups (and the page itself) whose **blending colour
    /// space is SUBTRACTIVE** — `DeviceCMYK`, `Separation`, `DeviceN`, or
    /// a four-component `ICCBased` resolving to one (§11.3.4).
    ///
    /// # Why this is counted at all, now that it IS honoured
    ///
    /// ★ **This section was headed "Why this is counted before it is
    /// honoured" and was itself the stale claim it warned about.** Until
    /// `Pass 97.1e` pdfce blended in device sRGB throughout, so every
    /// blend inside one of these groups was computed on the wrong side of
    /// §11.3.4's complement — the marks landing in the right places and
    /// the picture staying plausible, which is why a counter was the only
    /// way to see it.
    ///
    /// That is no longer true, and the counter survives with a **changed
    /// job**: it is now a CENSUS of exposure rather than a measure of
    /// shortfall. Non-zero says *this page's compositing is governed by
    /// §11.3.4*; whether it was HONOURED is
    /// [`Diagnostics::cmyk_buffer_engaged`], and what it cost when it was
    /// not is [`Diagnostics::blends_in_wrong_space`]. Three numbers, three
    /// questions, and reading any one of them alone gets a wrong answer.
    ///
    /// **It is not a small class.** Every patch in the Ghent transparency
    /// panel declares `/Group /CS /DeviceCMYK` on the PAGE, including the
    /// one whose own objects are `ICCBased` RGB, because a non-isolated
    /// group inherits its blending space rather than choosing one
    /// (Table 147's `/CS` row).
    ///
    /// Zero means every group on the page blends additively and pdfce's
    /// answer is the standard's answer. Non-zero means it is not, and
    /// [`Self::blends_in_wrong_space`] says how often that mattered.
    pub blend_space_subtractive: usize,
    /// **Where the page's blending colour space came from** — `page_group`,
    /// `device_native` or `output_intent`.
    ///
    /// # Why a provenance and not just a space
    ///
    /// Because the space alone cannot be checked. `blend_space_subtractive`
    /// says a page composited in ink; it does not say whether the file
    /// asked for that or whether pdfce inferred it from an output intent
    /// under `PageBlendSpaceSource::OutputIntentIfSubtractive`. Those are
    /// different facts and only one of them is pdfce's guess.
    ///
    /// This is project rule 4 applied to the least visible inference the
    /// renderer makes: a blending space changes **every colour on the
    /// page** and draws nothing to say so, so two files rendering
    /// differently for this reason are otherwise indistinguishable from a
    /// bug. The disclosure is off-canvas by construction — a string on the
    /// metrics line — and nothing is drawn on the page.
    ///
    /// `""` when no page content was painted and the question never arose.
    pub blend_space_from: &'static str,
    /// `1` when the page's blending space was **inferred from the document's
    /// output intent** rather than declared by the page group or defaulted
    /// to the device's native space; `0` otherwise.
    ///
    /// The numeric twin of [`Self::blend_space_from`], and the one that
    /// reaches `pdfce-cli`'s metrics line. That line's values are all
    /// non-negative integers — a property its own test pins, and one
    /// downstream consumers rely on — so the provenance is reported there as
    /// a flag and in prose in the operator note.
    ///
    /// Only the **inference** needs a flag: `page_group` means the file said
    /// so and `device_native` means the standard said so, and neither is
    /// something pdfce guessed.
    pub blend_space_from_output_intent: usize,
    /// Blend-mode applications performed **additively while the blending
    /// colour space was subtractive** — §11.3.4 violations, counted where
    /// they happen.
    ///
    /// # Why this is separate from [`Self::blend_space_subtractive`]
    ///
    /// Because a subtractive blending space with nothing but `Normal`
    /// inside it is **not** wrong: §11.3.4's complement is applied to the
    /// blend function, and `Normal` is `c_s` on either side of it
    /// (`1 − (1 − c_s) = c_s`). A page can be entirely `DeviceCMYK` and
    /// entirely correct.
    ///
    /// So this is the number that says a rendering is actually affected,
    /// and the pair reads as *"N groups are in the risky space, and M
    /// blends inside them were computed the wrong way"*. Reporting only
    /// the first would overstate; only the second would hide the exposure.
    ///
    /// The worked case is Ghent `1_GWG162`'s `Difference` cell: magenta
    /// under black gives `DeviceCMYK 1 0 1 0` — the green the patch is
    /// authored around — under §11.3.4, and `(237, 1, 140)` without it.
    ///
    /// # ★ NARROWED BY `Pass 97.1e`, AND THE NARROWING IS THE POINT
    ///
    /// This used to increment whenever the blending space was subtractive,
    /// full stop, because pdfce had no way to honour §11.3.4 and every such
    /// blend really was computed additively. It now increments only when the
    /// paint target is **not** a colorant buffer.
    ///
    /// Leaving it un-narrowed was tried and rejected in the same session:
    /// `tools/measure-blend-space.py` went on reporting **107 of 107 wrong**
    /// on the Ghent suite after the buffer landed and two of its patches
    /// started passing. A shortfall counter that cannot see the fix reports
    /// the fix as a no-op — and it is the only instrument anyone runs at
    /// corpus scale for this question, so it would have said so
    /// indefinitely.
    ///
    /// The companion `cmyk_buffer` key still exists and still matters: it
    /// says *which* of the two regimes produced the zero.
    pub blends_in_wrong_space: usize,
    /// The page was composited in a **subtractive colorant buffer**
    /// (`Pass 97.1e`) rather than in sRGB.
    ///
    /// The answer to [`Self::blends_in_wrong_space`]. When this is `true`
    /// that counter's meaning changes and its name becomes misleading if
    /// read alone: the blends it counted at `/BM`-selection time were
    /// **performed** subtractively, because the buffer they landed in was.
    /// The counter is left as it is rather than suppressed, because it
    /// still measures a real quantity — how many blends on this page were
    /// exposed to §11.3.4 — and a number that changes meaning silently is
    /// worse than one that needs a companion.
    pub cmyk_buffer_engaged: bool,
    /// The page declared a subtractive blending space and pdfce composited
    /// it in sRGB **anyway**, because the colorant buffer could not be
    /// allocated.
    ///
    /// A refusal, and the honest outcome of a page-size ceiling: see
    /// [`crate::cmyk_buffer::MAX_CMYK_BUFFER_BYTES`]. Non-zero means the
    /// render is the pre-`Pass 97.1e` approximation and says so, rather
    /// than failing.
    pub cmyk_buffer_refused: usize,
    /// Pixels that entered the colorant buffer through the **sRGB bridge**
    /// rather than as authored ink — images, shadings, and the results of
    /// transparency groups.
    ///
    /// A disclosure, not a shortfall. An image's samples were flattened to
    /// sRGB inside the decode loop long before any canvas saw them, so
    /// bridging is the only information that reaches the compositor. The
    /// count exists so that "this page composited in ink" cannot be read
    /// as "every colour on this page was authored ink".
    pub cmyk_bridged_pixels: u64,
    /// Transparency groups on a subtractive page that could **not** be
    /// composited natively in ink.
    ///
    /// Two cases, both genuine shortfalls and both `Pass 97.1f`'s work: a
    /// **knockout** group, whose §11.4.6 semantics are preserved but whose
    /// interior runs in sRGB; and a **non-isolated** group, composited as
    /// if isolated with §11.4.4's backdrop removal skipped.
    ///
    /// An ordinary isolated group is **not** counted — it gets a child
    /// colorant buffer and crosses no conversion at all.
    pub cmyk_groups_approximated: u64,
    /// Image brushes that reached a subtractive paint through a path with
    /// no bridge, and were therefore **not painted at all**.
    ///
    /// Should always be zero: the only route to one is a replayed display
    /// list, and a subtractive page is refused for recording outright
    /// ([`crate::display_list::PoisonReason::ColorantBuffer`]). It is
    /// counted rather than asserted because a claim of unreachability
    /// decays as the code around it changes, and a counter that stays zero
    /// costs one `u64` and one line of output.
    pub cmyk_unbridged_images: u64,
    /// Pixels whose value the overprint composites actually changed.
    ///
    /// The measurement that distinguishes "overprint ran and mattered" from
    /// "overprint ran and was a no-op on this geometry" — two different
    /// facts that a paint count alone conflates.
    pub overprint_pixels: u64,
    /// Transparency groups rendered into their own buffer and composited
    /// as a UNIT — the §11.4.5 behaviour.
    ///
    /// **★ NOT a clean census, despite what this doc said until 2026-08-18
    /// ("A census, not a shortfall.").** A [`tiny_skia::Pixmap`] starts
    /// TRANSPARENT, and a transparent initial backdrop **is** isolated
    /// semantics (§11.4.7). A buffer is allocated whenever the outer
    /// graphics state is non-neutral — so a **non-isolated** group under a
    /// `/BM` silently becomes an isolated one, and every blend inside it
    /// composites against nothing, returning the source colour unchanged.
    ///
    /// Measured on the Ghent transparency patches, which set `/BM` at every
    /// `Do`: **14, 15 and 7** wrong cells out of 16, each counted here as a
    /// success. That measurement predates `Pass 97.0`, which **shipped on
    /// 2026-08-21**; this doc still said the number "over-reports until
    /// `Pass 97.0` lands" the day after it landed. The over-report is
    /// resolved; the figures above are kept as the record of what it was,
    /// labelled as history rather than as current behaviour.
    ///
    /// Recorded shape, because it is the fifth time on this one narrative:
    /// the print-site comment beside this counter was corrected in
    /// `3e3019a` and **this field's own doc was not** — inside the very
    /// commit written to fix "a claim duplicated across sites, corrected at
    /// one of them". Found by the librarian on filing, not by the sweep.
    pub transparency_groups_composited: usize,
    /// **Elements inside a knockout group** (`/K true`, Table 147) that
    /// could not be given §11.4.6 semantics, because they read the
    /// destination back.
    ///
    /// # ★ THIS COUNTER CHANGED MEANING IN `Pass 97.0`, and the old
    /// # wording is quoted rather than deleted
    ///
    /// It used to read: *"Groups carrying `/K true` that were composited
    /// as ordinary groups… Compositing the group as a unit gets its outer
    /// boundary right and its internal occlusion order wrong."* That was
    /// true while knockout was unimplemented. §11.4.6 now has a real
    /// implementation (`crate::canvas::KnockoutTarget`), so a knockout
    /// group pdfce renders exactly reports **zero** here.
    ///
    /// A silently redefined counter is worse than a renamed one: an
    /// operator diffing two runs would watch this fall to zero and read it
    /// as an improvement in the wrong thing.
    ///
    /// # What still lands here
    ///
    /// Exactly three operator kinds, and all three for one reason — they
    /// read the destination back, and there is no formulation of *"read
    /// what is already there"* that also yields the element's own shape in
    /// isolation, which §11.4.8 needs because it scales the destination by
    /// `(1 − f_s)` rather than `(1 − α_s)`:
    ///
    /// * a shading (`sh`, or a shading pattern),
    /// * an overprint composite (§11.7.4.3),
    /// * a per-paint non-separable blend (§11.3.5.3).
    ///
    /// Those elements layer instead of knocking out — the answer a
    /// **non**-knockout group would have given, which is also the answer
    /// every element gives at `q_s = 1`, so the shortfall is bounded and
    /// nameable rather than uncharacterised.
    ///
    /// # What this counter has NEVER measured, and it is the larger half
    ///
    /// Only **explicit `/K true`** groups. Four clauses establish a
    /// knockout group with no `/K` key anywhere in the file, and three of
    /// them are non-isolated:
    ///
    /// * **§9.3.8's `/TK`**, whose **initial value is `true`** — every text
    ///   object is an implicit knockout group;
    /// * **§11.6.7** — shading patterns are knockout (tiling patterns are
    ///   not);
    /// * **§11.7.4.4** — `B`/`B*`/`b`/`b*` and text rendering modes 2 and
    ///   6. Its NOTE 2 names the visible symptom outright: the **double
    ///   border** on a semi-transparent fill-then-stroke is missing
    ///   knockout.
    ///
    /// Ranked by likely frequency in real files that is `B`/`b` ≫ `/TK` >
    /// explicit `/K`. **None of them is treated as knockout today**, and
    /// none of them reaches this counter either — so a zero here does not
    /// mean a page had no knockout semantics to get wrong.
    pub transparency_groups_knockout_approximated: usize,
    /// **Non-isolated** transparency groups whose content stream was walked
    /// a **second time**, over a copy of their own backdrop, so §11.4.4's
    /// element formula and backdrop removal could be computed against it
    /// (`crate::canvas::Canvas::group`).
    ///
    /// # Why this is a cost counter rather than a shortfall counter
    ///
    /// Every other counter here names something pdfce did *not* do. This
    /// one names something it did, and the reason it is disclosed is that
    /// it is the only place in the renderer where a page's content stream
    /// is interpreted more than once. A document whose groups all blend
    /// pays roughly double for those subtrees, and an operator comparing
    /// two render timings has no other way to see it.
    ///
    /// A zero here does **not** mean non-isolated groups were mishandled:
    /// §11.4.4 NOTE 5 makes the single-run answer *exact* whenever the
    /// group's interior composites `Normal` throughout, which is the
    /// overwhelming majority of real content. The second walk is taken
    /// only when the interior actually blended against something.
    pub transparency_groups_backdrop_reruns: usize,
    /// Transparency groups whose **soft mask was applied to the group's
    /// RESULT** (§11.4.5) rather than folded into its contents' clip.
    ///
    /// # Why this is counted separately from `soft_masks_applied`
    ///
    /// Because they answer different questions and the difference is a
    /// correctness one. `soft_masks_applied` counts masks pdfce **built**;
    /// this counts the ones that reached the place §11.4.5 puts them.
    /// Folding a mask into the clip is exactly right for an elementary
    /// object (§11.6.4.1 makes the mask value that object's `q_m`) and
    /// exactly wrong for a group, where it multiplies once per object
    /// inside instead of once on the composite — visible wherever two of
    /// those objects overlap.
    ///
    /// A group whose mask could **not** be lifted out of the clip — a
    /// `W n` intervened between the `gs` that set it and the `Do`, so the
    /// pre-mask clip no longer describes the geometry — keeps the old
    /// behaviour and is counted on `soft_masks_reset_stale` instead. That
    /// counter already existed for the same underlying limit; this Pass
    /// gave it a second way to fire rather than inventing a third name for
    /// one condition.
    pub soft_masks_on_group_result: usize,
    /// Transparency groups that are **isolated** (`/I true`) or
    /// **knockout** (`/K true`) — Table 147. Counted at every such group,
    /// whether or not it was flattened.
    ///
    /// ★ This opened "**Of those**, the ones that are..." — a back
    /// reference to whichever field happened to precede it. Two things
    /// were wrong with that and only one is obvious. The obvious one:
    /// `soft_masks_on_group_result` was later declared in between, so the
    /// antecedent silently became a different counter. The one worth
    /// remembering: **the original antecedent was wrong too.** The
    /// increment site is `is_transparency_group && (knockout || isolated)`
    /// with no flattening condition at all, so this was never a subset of
    /// the flattened count. The first repair of this comment restored the
    /// "obvious" antecedent and had to be corrected again by reading the
    /// increment site — a doc that cites a NEIGHBOUR rather than a NAME
    /// gives no way to tell a broken reference from a wrong one.
    ///
    /// NOT Table 96, which is the COMMON group-attributes table; `/K`, `/I`
    /// and `/CS` belong to the transparency-group subtype's own table.
    /// Clause-11 table numbers also shift by −2 across editions — this is
    /// Table 145 in ISO 32000-2 — so the number alone is ambiguous without
    /// the edition.
    ///
    /// Tracked separately because these two flags are exactly where
    /// flattening stops being a good approximation. An ISOLATED group
    /// blends against a transparent initial backdrop rather than the page,
    /// so flattening it makes every non-Normal blend inside it composite
    /// against the wrong thing. A KNOCKOUT group has each element
    /// composite against the group's *initial* backdrop rather than the
    /// accumulated result, so flattening reverses the intended occlusion.
    pub transparency_groups_special: usize,
    /// `gs` operators that selected a **soft mask** (`/SMask`, §11.6.5)
    /// which pdfce does not implement — the marks were painted
    /// unmasked.
    ///
    /// The failure direction matters and is why this is separate from
    /// `blend_modes_ignored`: an ignored soft mask paints MORE than the
    /// document asked for, so content the author faded out or masked away
    /// appears at full strength. On a page whose design relies on a mask
    /// to hide something, that is the difference between a rendering
    /// artefact and showing what was meant to be hidden.
    pub soft_masks_ignored: usize,
    /// Soft masks BUILT and applied (§11.6.5).
    ///
    /// **★ "Applied" used to overstate it, and no longer does.** This doc
    /// said, until `Pass 97.0`: *"Application is not [right]: §11.4.5
    /// applies the mask to a transparency group's RESULT, whereas pdfce
    /// folds it into the clip, which applies it to each element inside the
    /// group."* That was true and is now false — see
    /// [`Self::soft_masks_on_group_result`], which counts the groups where
    /// the mask reached the place §11.4.5 puts it.
    ///
    /// **Folding into the clip is still what happens to an ELEMENTARY
    /// object, and that is correct rather than a leftover**: §11.6.4.1
    /// makes the mask value that object's `q_m`, and a `q_m` multiplies
    /// coverage exactly as a clip does. The two behaviours are one
    /// implementation of two different clauses, not a fix half-applied.
    ///
    /// Measured on Ghent, reference-strip correlation on the same strip
    /// before and after: `1_GWG1610` 0.576 → **0.962**, `1_GWG168`
    /// 0.725 → **0.978**, `1_GWG169` 0.905 → **0.986**. Still counted as
    /// UNRESOLVED by `tools/ghent-check.py`, which has no calibrated
    /// threshold for reference-strip patches — that is an instrument gap,
    /// not a render result.
    ///
    /// See also [`Self::soft_mask_transfer_ignored`], which is the other
    /// half and is **still owed**: `/TR` is read, counted and never
    /// evaluated.
    pub soft_masks_applied: usize,
    /// Soft masks carrying a `/TR` transfer function that was not applied.
    ///
    /// A shortfall the operator cannot see by looking: `/TR` is the natural
    /// place to INVERT a mask, so an ignored one can show exactly the
    /// content that should have been hidden.
    pub soft_mask_transfer_ignored: usize,
    /// `gs /SMask /None` resets that could not restore the pre-mask clip
    /// exactly, because a `W n` intervened while the mask was in force.
    ///
    /// See `GraphicsState::clip_before_smask` for why this case exists and
    /// why it is counted rather than silently mis-clipped.
    pub soft_masks_reset_stale: usize,
    /// Structural oddities tolerated (unbalanced `Q`, missing current
    /// point, mid-path `cm`, operand type/count mismatches).
    pub tolerated: usize,
    /// First few distinct unknown/deferred operator names, for the
    /// diagnostics panel.
    pub sample_ops: Vec<String>,
    /// Glyphs painted as `.notdef`, or not painted at all because no
    /// glyph could be selected (§9.6.6.2, §9.7.6.3 — the fallback
    /// ladders in `iso32000__ref__text_pipeline.md`).
    pub glyphs_notdef: usize,
    /// Glyphs painted from a **bundled** Foxit Base-14 substitute face
    /// rather than the document's own embedded program (rule R20/R63).
    /// Positions are still exact — they come from the PDF's own widths —
    /// but the shapes are pdfce's, not the document's. Since decision 012
    /// this counts the BUNDLED level ONLY; operator-supplied faces are
    /// counted in [`Diagnostics::glyphs_supplied`].
    pub glyphs_substituted: usize,
    /// Glyphs painted from an **operator-supplied** face (decision 012 —
    /// [`crate::font::GlyphSource::Supplied`]), matched by name through
    /// the `FontEnvironment` seam the shell filled from a font folder.
    /// Distinct from [`Diagnostics::glyphs_substituted`] (bundled): both
    /// are substitutes with exact positions, but a supplied glyph is the
    /// operator's own deliberate shape, never pdfce's guess — and, being
    /// machine-dependent, it is outside the R19 determinism guarantee, so
    /// it must be disclosed on its own (R63/R64).
    pub glyphs_supplied: usize,
    /// Fonts whose machinery this Pass does not implement (Type 3,
    /// non-`Identity-H` CMaps, `Identity-V`, unparseable programs).
    /// Their text was **skipped**, not approximated. Counted once per
    /// distinct font resource, not per glyph.
    pub fonts_unsupported: usize,
    /// [`Diagnostics::fonts_unsupported`] broken down by REASON, keyed
    /// by [`crate::text::UnsupportedFont::reason_key`] (`"Type3"`,
    /// `"NonIdentityCmap"`, `"VerticalWriting"`,
    /// `"CompositeNotEmbedded"`, `"UnknownSubtype"`, `"UnusableProgram"`).
    ///
    /// The lump counter answers "was any text skipped?"; this answers
    /// "*why*?" without re-instrumenting the loader (rule R20). A
    /// `CompositeNotEmbedded` or `NonIdentityCmap` count means "supply
    /// the font / this needs a CMap Pass"; an `UnusableProgram` count
    /// means "an embedded program pdfce could not parse" — historically
    /// the signal that caught the `0x00010000`-sfnt misroute that made
    /// every embedded TrueType land here. Summing the values equals
    /// [`Diagnostics::fonts_unsupported`].
    ///
    /// A `BTreeMap` (like [`Diagnostics::codec_feature_unsupported`]) so
    /// a batch report's key order is deterministic and diffable.
    pub fonts_unsupported_by_reason: BTreeMap<&'static str, usize>,
    /// `BaseFont` names that fell to a **bundled** substitute, for the
    /// diagnostics panel — so an operator can name the fonts they may
    /// want to supply from a font folder (rule R20/R63). Bundled-only
    /// since decision 012; supplied faces are named in
    /// [`Diagnostics::supplied_fonts`].
    pub substituted_fonts: Vec<String>,
    /// `BaseFont` names that resolved to an **operator-supplied** face
    /// (decision 012), for the diagnostics panel — so an operator can
    /// confirm which of the fonts they supplied actually drew, and see
    /// (via the name pdfce reports) whether a supplied file matched the
    /// reference it intended. Distinct from
    /// [`Diagnostics::substituted_fonts`] (rule R63).
    pub supplied_fonts: Vec<String>,
    /// Sampled images actually rasterized onto the page (image
    /// XObjects + inline images).
    pub images_rendered: usize,
    /// Images that could **not** be drawn at all and are therefore
    /// simply missing from the raster — an unimplemented codec
    /// (`DCTDecode`, `JPXDecode`, `CCITTFaxDecode`, `JBIG2Decode`,
    /// `LZWDecode`), an out-of-scope colour space, a malformed
    /// dictionary, or a size past the guard. This is the image-side
    /// twin of [`Diagnostics::fonts_unsupported`]: nothing was
    /// approximated, so the page is *incomplete*, not *wrong*.
    pub images_unsupported: usize,
    /// Of [`Diagnostics::images_unsupported`], those refused because
    /// the **codec itself** is unimplemented in this build, or because
    /// §8.9.7 forbids it in an inline image.
    ///
    /// From Pass 2.3 all four codecs are implemented, so the only
    /// remaining source is the inline-image refusal (`JBIG2Decode` and
    /// `JPXDecode` inside `BI`/`ID`/`EI`, which §7.4.7 and §8.9.7
    /// forbid). The counter is kept so an unimplemented-codec regression
    /// stays visible (decision 005 §6.4).
    pub images_codec_unsupported: usize,
    /// Images refused because a codec **sub-feature** is unimplemented,
    /// keyed by a stable name: `"DCT/arithmetic"`, `"DCT/lossless"`,
    /// `"DCT/12-bit"`, `"DCT/adobe-transform-3"`,
    /// `"CCITT/damaged-rows"` (Table 11's `DamagedRowsBeforeError`
    /// resynchronization, which pdfce does not implement — named only
    /// when the file actually asked for it and the stream then failed),
    /// `"JPX/progression-order-change"` and `"JPX/unsupported-marker"`
    /// (an unhandled T.800 marker segment; the former is
    /// `hayro-jpeg2000`'s own documented gap, distinguished from the
    /// latter by a marker walk on the error path), `"JPX/bit-depth"`
    /// (a component depth outside the 1..=31 pdfce will scale from) …
    /// An operator must be
    /// able to tell *which* feature is missing without reading the code
    /// (rule R27), which a single lumped counter cannot express.
    ///
    /// A `BTreeMap` rather than a `HashMap` so a batch report's key
    /// order is deterministic and diffable across runs.
    pub codec_feature_unsupported: BTreeMap<&'static str, usize>,
    /// Images whose codestream geometry disagreed with the image
    /// dictionary (`/Width`, `/Height`, `/BitsPerComponent`, component
    /// count). For JPX the codestream wins wherever Table 89 says it
    /// does — colour when `/ColorSpace` is absent, and bit depth
    /// always; for DCT a mismatch is a producer bug. Counted either
    /// way, never silent — the image still drew.
    ///
    /// A JPX image that carries `/BitsPerComponent` at all lands here
    /// even when the value is honest, because that is the one entry a
    /// reader is actively told to ignore.
    pub codec_geometry_mismatch: usize,
    /// 4-component DCT images in YCCK storage (effective transform
    /// 1/2) — decision 006 §4.4's **benign census**. The mandated
    /// YCCK→CMYK inverse carries no polarity ambiguity and pdfce
    /// pixel-matches pdfium on every such corpus file (9 as of
    /// 2026-07-31), so this is volume, not shortfall: no warning
    /// attaches. (The pre-006 doc here claimed zero existed and
    /// treated all 4-component JPEGs as suspect — both halves wrong.)
    pub dct_cmyk_images: usize,
    /// 4-component DCT images with effective transform **0** and
    /// **no `/Decode`** — the one genuinely polarity-ambiguous shape
    /// (decision 006 rule **R30**): the undocumented Photoshop
    /// inverted-storage convention could make such an image render as
    /// its own negative, and nothing in the codestream or dictionary
    /// disambiguates it. Drawn from the raw samples (matching all four
    /// reference engines) and WARNED about by name. Zero exist in the
    /// conformance corpus; any sighting is a decision 006 §9 revisit
    /// trigger.
    pub dct_cmyk_polarity_unverifiable: usize,
    /// JPX images declaring `/SMaskInData 2` — Table 89's colour
    /// channels "preblended with a background" plus an opacity channel
    /// that would need a `Matte` entry to undo.
    ///
    /// Recognized and deferred (decision 005 §7 assigns clause 11's
    /// transparency model to `ROADMAP.md` Pass 1.1 item 6.3), so the
    /// image IS drawn — from the preblended channels exactly as stored,
    /// which is what it looks like over that backdrop. Counted and
    /// named so the approximation is never silent.
    pub jpx_smask_in_data_preblended: usize,
    /// LZW streams that did not begin with a `ClearCode`, or that ended
    /// without an `EndOfInformation`. Both are recovered, both are
    /// non-conformant, both are reported.
    pub lzw_framing_anomalies: usize,

    // ---- image transparency (§8.9.6, §11.6.5.3) ----------------------
    //
    // A census pair plus two texture counters. `images_masked` counts
    // success and deliberately prints no warning — decision 006 §4.4's
    // lesson, that a note on verified-correct volume trains an operator
    // to ignore the channel. `images_mask_unsupported` is the shortfall
    // twin and always names its reason.
    /// Sampled images whose transparency pdfce **composited**: an
    /// `/SMask`, an explicit `/Mask` stencil, a colour-key `/Mask`, or a
    /// JPX in-codestream opacity channel. A subset of
    /// [`Diagnostics::images_rendered`].
    pub images_masked: usize,
    /// Of [`Diagnostics::images_masked`], the breakdown by mechanism,
    /// keyed by [`crate::image::MaskApplied::key`] (`"smask"`,
    /// `"stencil"`, `"colour-key"`, `"jpx-embedded-alpha"`). A
    /// `BTreeMap` so a batch report's key order is deterministic.
    ///
    /// Which mechanism a corpus actually uses is the measurement that
    /// should drive any future optimisation here, so it is recorded
    /// rather than guessed at.
    pub mask_applied: BTreeMap<&'static str, usize>,
    /// Images carrying a `/SMask` or `/Mask` that pdfce could **not**
    /// turn into alpha, so the base image was drawn **fully opaque** —
    /// visually wrong wherever the mask would have hidden something.
    /// The image-transparency twin of
    /// [`Diagnostics::images_unsupported`], separate because the
    /// operator's question differs: "is this picture missing?" versus
    /// "is this picture too solid?"
    pub images_mask_unsupported: usize,
    /// Of [`Diagnostics::images_mask_unsupported`], the breakdown by
    /// reason, keyed by [`crate::mask::MaskRefusal::key`] (rule R27:
    /// "the mask is 40 gigapixels" and "the mask is in a colour space
    /// pdfce refuses" lead to different next actions).
    pub mask_refused: BTreeMap<&'static str, usize>,
    /// Masks whose pixel dimensions differed from their base image's and
    /// were therefore point-sampled across it (§8.9.6.3: "need not have
    /// the same resolution … their boundaries on the page will
    /// coincide"). Conformant and common; counted so a pixel-parity
    /// investigation can tell a resampling difference from a decode one
    /// without re-deriving why.
    pub masks_resampled: usize,
    /// `/SMask`s carrying `/Matte` whose preblend pdfce **undid**
    /// (§11.6.5.3's `c = m + (c′ − m)/α`). Census, not shortfall — but
    /// worth counting, because the reconstruction amplifies quantisation
    /// error by `1/α` in near-transparent regions and a parity
    /// investigation should know when it is looking at one.
    pub mattes_undone: usize,
    /// `/SMask`s carrying `/Matte` whose preblend pdfce did **not** undo
    /// — a dimension mismatch (Table 145 makes equality a `shall` when
    /// `/Matte` is present), an `Indexed` parent (spec ambiguity
    /// `SM-A4`), or a wrong-length array. The alpha is applied either
    /// way; only the colour correction is missing, and the reason is in
    /// [`Diagnostics::image_notes`].
    pub mattes_not_undone: usize,
    /// First few distinct reasons behind `images_unsupported`, plus the
    /// softer per-image divergences ([`crate::image::ImageNotes`]:
    /// deferred `/SMask`, truncated samples, short palette). Named
    /// separately from [`Diagnostics::sample_ops`] because "which codec
    /// do I need?" and "which operator is missing?" are different
    /// operator questions with different answers.
    pub image_notes: Vec<String>,
    /// Form XObjects executed (§8.10). Counted after the recursion
    /// guards, so it is the number of forms actually painted.
    pub forms_rendered: usize,
    /// `Do` invocations skipped because the form's `/BBox`, mapped through
    /// the CTM, lands entirely outside the canvas or outside the clip in
    /// force. **Lossless**, and that is a consequence of the spec rather
    /// than an assumption: §8.10.1 makes `/BBox` a *clip* on the form's
    /// contents, so nothing inside a form can paint outside it, so a form
    /// whose box misses the viewport cannot contribute a pixel.
    ///
    /// Counted separately from [`Diagnostics::forms_rendered`] rather than
    /// folded into it, because "342 forms executed" and "342 forms in the
    /// page, 1 of them on screen" are different answers to an operator
    /// asking why a render was slow — and the first is what pdfce used to
    /// report while doing the second's work.
    pub forms_culled: usize,
    /// `Do` invocations skipped because the form was smaller than
    /// [`SUBPIXEL_CULL_PX`] in both axes and
    /// [`RenderOptions::subpixel_culling`] was on.
    ///
    /// **LOSSY**, and counted separately from [`Self::forms_culled`] for
    /// exactly that reason: that one is an exact consequence of §8.10.1's
    /// `/BBox` clip and changes no pixel, this one drops coverage the
    /// document asked for. Reporting them as one number would let a
    /// fidelity trade hide inside a correctness optimisation.
    ///
    /// Emitted on the metrics line whether the option is on or off, so a
    /// raster carries the count of what it left out rather than the
    /// operator having to remember which flags produced it (rule 4).
    pub subpixel_culled: usize,
    /// `Do` invocations refused because they would have exceeded
    /// [`MAX_XOBJECT_DEPTH`] **or** re-entered a form already on the
    /// stack (a cycle). Their content is missing from the raster.
    pub xobject_depth_overflows: usize,

    // ---- annotation appearances (Pass 6.0, ISO 32000-1 §12.5) --------
    //
    // These count what the annotation-painting pass (`crate::annot`,
    // gated by `RenderOptions::effective_annotation_scope`) did on this
    // page — the census (`annotations_total`, `annotations_widget`,
    // `annotations_without_ap`) is taken under EVERY scope, so a narrowed
    // or suppressed render still discloses what it is not showing. They are
    // page-level: nested form XObjects never evaluate annotations, so a
    // merged child contributes zero to them. Every counter is APPENDED
    // to the CLI stable-line contract, never reordered (module docs of
    // `pdfce-cli`).
    /// Annotations found in the page's `/Annots` array and modelled
    /// (every subtype, every disposition — the census denominator).
    pub annotations_total: usize,
    /// Annotations whose selected `/AP` `/N` appearance was actually
    /// painted onto the page (a §12.5.5 placement succeeded).
    pub annotations_painted: usize,
    /// Annotations with **no usable appearance** ([`Appearance::None`]),
    /// keyed by `/Subtype`: no `/AP`, no `/N`, or an `/N` that is neither
    /// stream nor subdictionary. Under R43 these are named-not-painted
    /// and never synthesised; the count is the measured demand signal for
    /// the later appearance-generation Passes.
    ///
    /// A `BTreeMap` so a batch report's key order is deterministic.
    ///
    /// [`Appearance::None`]: pdfce_core::annot::Appearance::None
    pub annotations_without_ap: BTreeMap<String, usize>,
    /// Annotations suppressed from on-screen display by the Hidden or
    /// NoView flag (§12.5.3, Table 165). Honoured (not painted) AND
    /// counted (R50): content the operator cannot see is still disclosed.
    pub annotations_hidden: usize,
    /// Annotations whose `/AP` `/N` is a state subdictionary but whose
    /// state could not be selected — `/AS` missing against a multi-entry
    /// subdictionary, or `/AS` naming an absent state (§12.5.5 NOTE 3).
    /// Displayed as nothing, never guessed.
    pub annotations_appearance_state_missing: usize,
    /// Of [`Diagnostics::annotations_total`], those whose `/Subtype` is
    /// `Widget` (§12.5.6.19). A census signal — widgets are ~88 % of
    /// organic annotations, so their share drives forms prioritisation.
    pub annotations_widget: usize,
    /// Annotations carrying an `/AP` `/N` that could **not be placed**:
    /// a missing `/Rect` or `/BBox` (the §12.5.5 placement inputs), or a
    /// **degenerate transformed appearance box** (zero width or height,
    /// making the step-b fit matrix singular). A named refusal, never a
    /// divide-by-zero and never a fabricated placement (risk X2). The
    /// specific reason is in [`Diagnostics::annotation_notes`].
    pub annotations_placement_degenerate: usize,
    /// Annotations withheld because the render's
    /// [`AnnotationScope`](crate::AnnotationScope) does not paint their
    /// class — a `/Highlight` under "Document", a sticky note under
    /// "Document and Stamps", any annotation at all under
    /// [`ContentOnly`](crate::AnnotationScope::ContentOnly).
    ///
    /// # Why this is its own counter and not folded into `annotations_hidden`
    ///
    /// They answer different questions and have different owners. A hidden
    /// annotation was hidden **by the document** (§12.5.3's flags, or an
    /// OFF optional-content group); an out-of-scope one was withheld **by
    /// the caller**, and can be brought back by changing one option. Summing
    /// them would tell an operator "six annotations are not shown" while
    /// destroying the only information that says which of those six they
    /// can do anything about.
    ///
    /// The two are independent, not exclusive: an annotation that is both
    /// out of scope and Hidden increments both counters, because both
    /// statements about it are true.
    ///
    /// Zero under the default scope, where every class is painted — so this
    /// counter is also the answer to "did a narrowed scope actually change
    /// what this page shows?"
    pub annotations_out_of_scope: usize,
    /// The page's own content streams were **not painted** because the
    /// render's [`AnnotationScope`](crate::AnnotationScope) was
    /// [`FormFieldsOnly`](crate::AnnotationScope::FormFieldsOnly) — the
    /// print-onto-pre-printed-paper scope, where the page background is the
    /// physical paper and drawing it again would double-print it.
    ///
    /// The one diagnostic here that reports a **deliberate** omission the
    /// caller asked for, rather than a shortfall. It exists because the
    /// resulting raster is indistinguishable by inspection from a page
    /// whose content failed to decode, and a caller handed a nearly-blank
    /// pixmap must be able to tell the two apart without knowing which
    /// options it passed three layers up.
    ///
    /// While this is `true`, [`Diagnostics::contents_streams_unresolved`]
    /// stays `0`: pdfce never looked at the content streams, so it has
    /// nothing to report about them, and reporting an incompleteness it did
    /// not measure would be an invented fact.
    pub page_content_suppressed: bool,
    /// First few distinct annotation-handling reasons (degenerate box,
    /// missing `/Rect`/`/BBox`, deferred NoZoom/NoRotate adjustment),
    /// for the diagnostics surfaces. Kept separate from
    /// [`Diagnostics::sample_ops`] and [`Diagnostics::image_notes`]
    /// because "why was this annotation not placed?" is a distinct
    /// operator question (R27).
    pub annotation_notes: Vec<String>,
    /// Colour-space disclosures (ISO 32000-1 §8.6) — an unresolvable
    /// space, the `ICCBased` fallback, an unevaluated `Separation`/
    /// `DeviceN` tint transform, an unpainted pattern.
    ///
    /// Nested rather than flattened into a dozen more fields here because
    /// they are one subsystem's story and [`crate::color`] owns their
    /// definitions and their wording; this struct owns the page-level
    /// aggregation. See [`crate::color::ColorDiagnostics`].
    pub color: crate::color::ColorDiagnostics,
    /// Images suppressed entirely because their colour space was
    /// `/Separation /None` or an all-`/None` `/DeviceN` (§8.6.6.4/.5).
    ///
    /// Census of pdfce obeying the standard, not a shortfall — but it
    /// belongs on the machine line, because a reference renderer that
    /// paints such an image (pdfium paints it BLACK, measured
    /// 2026-08-17) will diverge maximally and a parity harness needs to
    /// know the divergence is pdfce's correctness.
    pub images_colorant_none: usize,
    /// Images converted through pdfce's OWN XYZ→sRGB colorimetry rather
    /// than a colour-management engine — `Lab`, `CalGray`, `CalRGB`.
    ///
    /// On the stdout line rather than only in a note, because that is the
    /// only channel a parity harness reads. A `Lab` image landed in the
    /// harness's *unexplained* bucket while a perfectly good stderr
    /// sentence explained it — the third instance today of a disclosure
    /// that reached a human and not a machine.
    pub images_uncalibrated_colorimetry: usize,
    /// §8.7.4 shadings — the gradient inventory.
    ///
    /// Nested for the same reason `color` is: the counters are defined and
    /// documented next to the code that increments them, and this struct
    /// owns the page-level aggregation.
    ///
    /// **Every counter in it is currently a "found, not painted" census.**
    /// The model slice resolves and classifies shadings; the geometry slice
    /// paints them. That is worth stating here as well as there, because a
    /// non-zero `encountered` beside a zero `painted` is otherwise easy to
    /// read as a bug rather than as the honest report it is.
    pub shading: crate::shading::ShadingDiagnostics,
}

impl Diagnostics {
    /// Record a distinct operator/construct name for the sample list.
    fn note(&mut self, name: &[u8]) {
        push_sample(&mut self.sample_ops, &String::from_utf8_lossy(name));
    }

    /// Record a distinct image reason/divergence for the sample list.
    fn note_image(&mut self, reason: &str) {
        push_sample(&mut self.image_notes, reason);
    }

    /// Record a distinct annotation-handling reason (degenerate box,
    /// missing `/Rect`/`/BBox`, deferred flag adjustment) for the
    /// [`Diagnostics::annotation_notes`] list. Called by [`crate::annot`].
    pub(crate) fn note_annotation(&mut self, reason: &str) {
        push_sample(&mut self.annotation_notes, reason);
    }

    /// Record the soft divergences of one successfully drawn image.
    fn note_image_divergence(&mut self, notes: ImageNotes) {
        // Transparency census (Pass 1.1 item 6.3). `images_masked` is
        // volume, not shortfall — the same treatment decision 006 §4.4
        // gave the benign YCCK census, and for the same reason: a note
        // on every correctly-composited transparent image would cry wolf
        // on known-good files. Only the refusals below get a note.
        if let Some(kind) = notes.mask_applied {
            self.images_masked += 1;
            *self.mask_applied.entry(kind.key()).or_insert(0) += 1;
        }
        if let Some(reason) = notes.mask_refused {
            self.images_mask_unsupported += 1;
            *self.mask_refused.entry(reason).or_insert(0) += 1;
            self.note_image(&format!(
                "/SMask or /Mask present but not applied ({reason}); base image drawn opaque"
            ));
        }
        if notes.mask_resampled {
            self.masks_resampled += 1;
        }
        if notes.matte_undone {
            self.mattes_undone += 1;
        }
        if let Some(reason) = notes.matte_not_undone {
            self.mattes_not_undone += 1;
            self.note_image(&format!(
                "/SMask carries /Matte but the preblend was NOT undone ({reason}); \
alpha applied, colours stay shifted toward the matte colour"
            ));
        }
        if notes.jpx_smask_in_data_preblended {
            self.jpx_smask_in_data_preblended += 1;
            self.note_image(
                "/SMaskInData 2: JPX colour channels are preblended with a backdrop; Matte un-premultiplication deferred",
            );
        }
        if notes.truncated {
            self.note_image("sample data shorter than Width x Height (padded with 0)");
        }
        if notes.palette_out_of_range {
            self.note_image("/Indexed lookup table shorter than hival (painted black)");
        }
        // R183: a picture that is correctly absent is otherwise
        // indistinguishable from one that failed to decode. This one is
        // pdfce obeying §8.6.6.4/.5, not falling short — and the note
        // carries the pdfium disagreement, because a parity run will show
        // a maximal divergence here that is pdfce's correctness.
        if notes.colorant_none_suppressed {
            self.images_colorant_none += 1;
            self.note_image(
                "image colour space is /Separation /None or an all-/None /DeviceN: NOTHING painted, per 8.6.6.4/.5 (note: pdfium paints such an image black)",
            );
        }
        if let Some(space) = notes.uncalibrated_colorimetry {
            self.images_uncalibrated_colorimetry += 1;
            self.note_image(&format!(
                "{space} image converted by pdfce's own XYZ->sRGB (Bradford to D65, no colour management, no rendering intent): defensible, not colour-managed"
            ));
        }
        if notes.decode_array_ignored {
            self.note_image("/Decode array had the wrong length (default used)");
        }
        if notes.codec_geometry_mismatch {
            self.codec_geometry_mismatch += 1;
            self.note_image("codestream geometry disagrees with the image dictionary");
        }
        // Decision 006 §4.4: the YCCK census is deliberately note-less
        // — it is verified-correct volume, and a per-image note would
        // re-create the cried-wolf warning the split exists to retire.
        // Only the R30 shape gets a named note.
        if notes.dct_cmyk_image {
            self.dct_cmyk_images += 1;
        }
        if notes.dct_cmyk_polarity_unverifiable {
            self.dct_cmyk_polarity_unverifiable += 1;
            self.note_image(
                "4-component CMYK JPEG with ColorTransform 0 and no /Decode: \
polarity unverifiable (decision 006 R30)",
            );
        }
        if notes.lzw_framing_anomalies > 0 {
            self.lzw_framing_anomalies += notes.lzw_framing_anomalies;
            self.note_image("LZW stream missing its ClearCode or EndOfInformation");
        }
    }

    /// Fold a nested form XObject's diagnostics into this one.
    ///
    /// Every counter is additive because every counter answers a
    /// "how many, on this page" question — and the page includes
    /// whatever its forms painted. The two sample lists merge with the
    /// same dedup-and-cap policy as direct notes.
    pub(crate) fn merge(&mut self, other: Self) {
        self.deferred_ops += other.deferred_ops;
        self.oc_sections_hidden += other.oc_sections_hidden;
        self.unknown_ops += other.unknown_ops;
        self.compat_skipped += other.compat_skipped;
        self.overprint_requested += other.overprint_requested;
        self.overprint_effective += other.overprint_effective;
        self.overprint_composited += other.overprint_composited;
        self.nonseparable_composited += other.nonseparable_composited;
        self.nonseparable_pixels += other.nonseparable_pixels;
        self.overprint_refused += other.overprint_refused;
        self.overprint_images_unsupported += other.overprint_images_unsupported;
        self.blend_space_subtractive += other.blend_space_subtractive;
        // A provenance is a per-page FACT, not a tally, so merging takes
        // the first non-empty rather than summing or overwriting. A page
        // that painted no content contributes nothing and must not erase
        // the answer a sibling already established.
        if self.blend_space_from.is_empty() {
            self.blend_space_from = other.blend_space_from;
        }
        self.blend_space_from_output_intent += other.blend_space_from_output_intent;
        self.cmyk_buffer_engaged |= other.cmyk_buffer_engaged;
        self.cmyk_buffer_refused += other.cmyk_buffer_refused;
        self.cmyk_bridged_pixels += other.cmyk_bridged_pixels;
        self.cmyk_groups_approximated += other.cmyk_groups_approximated;
        self.cmyk_unbridged_images += other.cmyk_unbridged_images;
        self.blends_in_wrong_space += other.blends_in_wrong_space;
        self.overprint_pixels += other.overprint_pixels;
        self.overprint_mode1_requested += other.overprint_mode1_requested;
        self.transparency_groups_composited += other.transparency_groups_composited;
        self.transparency_groups_backdrop_reruns += other.transparency_groups_backdrop_reruns;
        self.soft_masks_on_group_result += other.soft_masks_on_group_result;
        self.transparency_groups_knockout_approximated +=
            other.transparency_groups_knockout_approximated;
        self.transparency_groups_flattened += other.transparency_groups_flattened;
        self.transparency_groups_special += other.transparency_groups_special;
        self.blend_modes_applied += other.blend_modes_applied;
        self.blend_modes_ignored += other.blend_modes_ignored;
        self.soft_masks_ignored += other.soft_masks_ignored;
        self.soft_masks_applied += other.soft_masks_applied;
        self.soft_mask_transfer_ignored += other.soft_mask_transfer_ignored;
        self.soft_masks_reset_stale += other.soft_masks_reset_stale;
        self.tolerated += other.tolerated;
        self.glyphs_notdef += other.glyphs_notdef;
        self.glyphs_substituted += other.glyphs_substituted;
        self.glyphs_supplied += other.glyphs_supplied;
        self.fonts_unsupported += other.fonts_unsupported;
        for (reason, count) in other.fonts_unsupported_by_reason {
            *self.fonts_unsupported_by_reason.entry(reason).or_insert(0) += count;
        }
        self.images_rendered += other.images_rendered;
        self.images_unsupported += other.images_unsupported;
        self.images_codec_unsupported += other.images_codec_unsupported;
        self.codec_geometry_mismatch += other.codec_geometry_mismatch;
        self.dct_cmyk_images += other.dct_cmyk_images;
        self.dct_cmyk_polarity_unverifiable += other.dct_cmyk_polarity_unverifiable;
        self.jpx_smask_in_data_preblended += other.jpx_smask_in_data_preblended;
        self.lzw_framing_anomalies += other.lzw_framing_anomalies;
        self.images_masked += other.images_masked;
        self.images_mask_unsupported += other.images_mask_unsupported;
        self.masks_resampled += other.masks_resampled;
        self.mattes_undone += other.mattes_undone;
        self.mattes_not_undone += other.mattes_not_undone;
        for (kind, count) in other.mask_applied {
            *self.mask_applied.entry(kind).or_insert(0) += count;
        }
        for (reason, count) in other.mask_refused {
            *self.mask_refused.entry(reason).or_insert(0) += count;
        }
        for (feature, count) in other.codec_feature_unsupported {
            *self.codec_feature_unsupported.entry(feature).or_insert(0) += count;
        }
        self.forms_rendered += other.forms_rendered;
        self.forms_culled += other.forms_culled;
        self.subpixel_culled += other.subpixel_culled;
        self.xobject_depth_overflows += other.xobject_depth_overflows;
        // Annotation counters are page-level (a nested form never sets
        // them), so in practice `other` contributes zero here — but the
        // merge is written out in full so it stays correct if that ever
        // changes, rather than silently dropping a counter.
        self.annotations_total += other.annotations_total;
        self.annotations_painted += other.annotations_painted;
        self.annotations_hidden += other.annotations_hidden;
        self.annotations_appearance_state_missing += other.annotations_appearance_state_missing;
        self.annotations_widget += other.annotations_widget;
        self.annotations_placement_degenerate += other.annotations_placement_degenerate;
        self.annotations_out_of_scope += other.annotations_out_of_scope;
        // A `bool`, so the merge is OR rather than `+`: page-content
        // suppression is a property of the whole render, and a merged child
        // can only ever confirm it, never retract it.
        self.page_content_suppressed |= other.page_content_suppressed;
        for (subtype, count) in other.annotations_without_ap {
            *self.annotations_without_ap.entry(subtype).or_insert(0) += count;
        }
        self.images_colorant_none += other.images_colorant_none;
        self.images_uncalibrated_colorimetry += other.images_uncalibrated_colorimetry;
        self.color.merge(other.color);
        self.shading.merge(other.shading);
        for s in other.annotation_notes {
            push_sample(&mut self.annotation_notes, &s);
        }
        for s in other.sample_ops {
            push_sample(&mut self.sample_ops, &s);
        }
        for s in other.image_notes {
            push_sample(&mut self.image_notes, &s);
        }
        for s in other.substituted_fonts {
            if self.substituted_fonts.len() < 32 && !self.substituted_fonts.contains(&s) {
                self.substituted_fonts.push(s);
            }
        }
        for s in other.supplied_fonts {
            if self.supplied_fonts.len() < 32 && !self.supplied_fonts.contains(&s) {
                self.supplied_fonts.push(s);
            }
        }
    }
}

/// Append `value` to a diagnostics sample list if it is new and the
/// list is not already at [`MAX_SAMPLES`].
fn push_sample(list: &mut Vec<String>, value: &str) {
    if list.len() < MAX_SAMPLES && !list.iter().any(|s| s == value) {
        list.push(value.to_owned());
    }
}

/// Interpret `content` onto `pixmap` starting from `initial` (the
/// device CTM etc. already set by the caller from page geometry).
///
/// `resources` is the page's resolved resource dictionary — `gs` reads
/// `/ExtGState` from it and `Tf` reads `/Font` (§7.8.3 Table 33). `doc`
/// is needed because those resource entries are almost always indirect
/// references, and a font dictionary's descriptor, encoding, widths and
/// embedded program are each another hop; `fonts` supplies the
/// substitute faces for any font that carries no program (decision 004
/// §6.3 — the renderer never goes looking for one itself, R19).
// Eight parameters, one over clippy's bound, and grouping them would make
// this worse rather than better. They are not a cohesive value — they are
// `RenderOptions` already DECOMPOSED into the pieces the interpreter
// actually uses, plus the two things options cannot carry (the document
// view and the target pixmap). A `RunArgs` struct here would exist purely
// to satisfy a count, and would have to be built at every call site from
// the same fields.
#[allow(clippy::too_many_arguments)]
pub fn run(
    doc: &DocumentView<'_>,
    content: &ContentStream,
    resources: &Dict,
    fonts: &FontEnvironment,
    initial: GraphicsState,
    pixmap: &mut Pixmap,
    cancel: Option<&RenderCancel>,
    policy: RenderPolicy<'_>,
) -> Diagnostics {
    run_on(
        doc,
        content,
        resources,
        fonts,
        initial,
        &mut Canvas::paint(pixmap),
        cancel,
        policy,
        // This entry point is handed a bare content stream and no page, so
        // there is no `/Group /CS` to read. Additive is 11.4.7's own answer
        // for an RGBA8 output device, not a shrug.
        crate::compositor::BlendSpace::Additive,
    )
}

/// §11.4.7 / §11.3.4 — the **page's** blending colour space.
///
/// A page is a transparency group (§11.4.7: *"all of the elements painted
/// directly onto a page … shall be treated as if they were contained in a
/// transparency group P"*), so it has a blending colour space, and every
/// element on it — including every non-isolated group inside it, which
/// **inherits** rather than choosing (Table 147's `/CS` row) — composites
/// in that space.
///
/// # Why this is the number that matters
///
/// **Every patch in the Ghent transparency panel declares
/// `/Group /CS /DeviceCMYK` here**, including the one whose own objects
/// are `ICCBased` RGB. So the space is subtractive for all of them
/// regardless of what the artwork is coloured in, and §11.3.4's complement
/// governs every blend on the page. That is why the transparency panels
/// could not be fixed by the group model alone.
///
/// # The default, and why it is `Additive` rather than a guess
///
/// §11.4.7 says an unspecified page group colour space *"shall be
/// inherited from the native colour space of the output device"*. pdfce's
/// output device is an `RGBA8` pixmap, so `Additive` is not a fallback
/// here — it is the clause's own answer for this renderer. A file that
/// says nothing gets the right answer, not a shrug.
pub(crate) fn page_blend_space(
    doc: &DocumentView<'_>,
    page_id: ObjId,
    resources: &Dict,
    diag: &mut crate::color::ColorDiagnostics,
    source: pdfce_core::settings::PageBlendSpaceSource,
) -> (crate::compositor::BlendSpace, BlendSpaceFrom) {
    use crate::compositor::BlendSpace;
    let Some(dict) = doc.value(page_id).and_then(Object::as_dict) else {
        return (BlendSpace::Additive, BlendSpaceFrom::DeviceNative);
    };
    // The DECLARED case first, and it is unconditional: a page group that
    // names its `/CS` is answered by Table 147, and no setting reaches it.
    // `PGB-7a` notes that ISO 32000-2's Annex P routes a declared page group
    // through the same "device or output intent" node, but only to supply a
    // space it did not declare -- a declared one is still its own answer.
    if let Some(sp) = dict
        .get(b"Group")
        .map(|o| doc.resolve(o))
        .and_then(Object::as_dict)
        .and_then(|g| g.get(b"CS"))
        .and_then(|cs| crate::color::resolve_object(doc, doc.resolve(cs), resources, 0, diag))
    {
        return (BlendSpace::of(&sp), BlendSpaceFrom::PageGroup);
    }
    // UNDECLARED. §11.4.7 and §11.6.3 both say the device's native space,
    // and for pdfce that is an `RGBA8` pixmap. Everything below is ISO
    // 32000-2 Annex P territory and is therefore opt-out-able.
    match source {
        pdfce_core::settings::PageBlendSpaceSource::DeviceNative => {
            (BlendSpace::Additive, BlendSpaceFrom::DeviceNative)
        }
        pdfce_core::settings::PageBlendSpaceSource::OutputIntentIfSubtractive => {
            match output_intent_blend_space(doc) {
                Some(BlendSpace::Subtractive) => {
                    (BlendSpace::Subtractive, BlendSpaceFrom::OutputIntent)
                }
                // ★ An additive output intent is NOT reported as
                // `OutputIntent` provenance, because under this setting it
                // did not decide anything -- the answer is the device's
                // native space either way, and claiming the intent supplied
                // it would make the disclosure say something false on every
                // ordinary RGB file.
                _ => (BlendSpace::Additive, BlendSpaceFrom::DeviceNative),
            }
        }
        pdfce_core::settings::PageBlendSpaceSource::OutputIntentAlways => {
            output_intent_blend_space(doc)
                .map_or((BlendSpace::Additive, BlendSpaceFrom::DeviceNative), |sp| {
                    (sp, BlendSpaceFrom::OutputIntent)
                })
        }
        // `PageBlendSpaceSource` is `#[non_exhaustive]`, so a variant added
        // in `pdfce-core` compiles here before anyone teaches this match
        // about it. The fallback is the ISO 32000-1 answer -- the device's
        // native space -- because that is the direction that cannot invent
        // a subtractive page out of a setting this build does not
        // understand. A new variant therefore renders as today's `1.7`
        // behaviour until wired, which is visible and conservative rather
        // than silently colourful.
        _ => (BlendSpace::Additive, BlendSpaceFrom::DeviceNative),
    }
}

/// Where a page's blending colour space came from — for **disclosure**, not
/// for behaviour.
///
/// Project rule 4 requires an inference pdfce made to be reported
/// off-canvas, and a blending space is the extreme case of an invisible
/// inference: it changes every colour on the page and leaves no mark saying
/// so. Two files that render differently for this reason would otherwise be
/// indistinguishable from a bug.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlendSpaceFrom {
    /// The page group declared `/CS`. Table 147; nothing was inferred.
    PageGroup,
    /// Inherited from the output device, which for pdfce is sRGB.
    /// ISO 32000-1 §11.4.7 / §11.6.3.
    DeviceNative,
    /// Taken from the document's output intent — ISO 32000-2 Annex P,
    /// which is **informative** and does not rank this against the device.
    /// Only reachable when the page group declared no `/CS`.
    OutputIntent,
}

impl BlendSpaceFrom {
    /// The token used on `pdfce-cli`'s metrics line.
    pub(crate) const fn token(self) -> &'static str {
        match self {
            Self::PageGroup => "page_group",
            Self::DeviceNative => "device_native",
            Self::OutputIntent => "output_intent",
        }
    }
}

/// The blending space implied by the document's `/OutputIntents`, if one can
/// be determined at all.
///
/// # How the colour class is decided, and why it is `/N` rather than a name
///
/// The output intent's `/DestOutputProfile` is an ICC profile stream, and
/// §8.6.5.5 / Table 66 give `/N` as the number of colour components — the
/// same key `ICCBased` uses. Four or more components is a subtractive
/// device class; one or three is not. Reading `/N` rather than parsing the
/// profile header keeps this to one dictionary lookup and uses the value
/// the writer was already obliged to make correct.
///
/// # What returns `None`, and why that is not a failure
///
/// A file may carry an output intent with **no** `/DestOutputProfile` —
/// PDF/X permits identifying a registered printing condition by name alone.
/// pdfce cannot resolve a name to a colorant count without a registry it
/// does not ship and must not fetch (`ARCHITECTURE.md` §1.1 forbids a
/// network client in the engine, permanently). `None` therefore means *"not
/// determinable here"*, and the caller falls back to the device's native
/// space — the ISO 32000-1 answer, which is the safe direction.
///
/// # Multiple output intents
///
/// The first one that yields a determinable space wins. ISO 32000-2 does
/// not say which intent governs when several are present — recorded as
/// `PGB-A2` in the spec corpus, and deliberately NOT solved differently
/// from the existing `SEP-A1` question of the same shape. First-wins is
/// stated here so that the choice is visible rather than emergent.
fn output_intent_blend_space(doc: &DocumentView<'_>) -> Option<crate::compositor::BlendSpace> {
    let catalog = doc
        .catalog_id()
        .and_then(|id| doc.value(id))
        .and_then(Object::as_dict)?;
    let entry = catalog.get(b"OutputIntents")?;
    let items = doc.resolve(entry).as_array().map(<[Object]>::to_vec)?;
    items.iter().find_map(|item| {
        let intent = doc.resolve(item).as_dict()?.clone();
        let profile = intent.get(b"DestOutputProfile")?;
        // `/DestOutputProfile` is an ICC profile STREAM, so its `/N` lives on
        // the stream's dictionary. `Object::as_dict` matches only `Dict`, by
        // design, so the stream arm is written out rather than papered over
        // with a helper that would blur the two.
        let n = match doc.resolve(profile) {
            Object::Stream(st) => st.dict.get(b"N").map(|o| doc.resolve(o)),
            _ => None,
        }
        .and_then(Object::as_int)?;
        Some(if n >= 4 {
            crate::compositor::BlendSpace::Subtractive
        } else {
            crate::compositor::BlendSpace::Additive
        })
    })
}

/// [`run`], against an arbitrary drawing target rather than a pixmap.
///
/// This is the form the renderer itself uses. It exists separately from
/// [`run`] for one reason worth stating: `Canvas` is a crate-internal
/// type, and making the public entry point demand one would drag the
/// display-list machinery into every caller's field of view — including
/// this crate's own integration tests, which legitimately want "render
/// this content stream into these pixels" and nothing more.
///
/// So the public signature stays the one it has always been, and the
/// extra capability arrives as an additional door rather than as a
/// breaking change to the existing one.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_on(
    doc: &DocumentView<'_>,
    content: &ContentStream,
    resources: &Dict,
    fonts: &FontEnvironment,
    initial: GraphicsState,
    canvas: &mut Canvas<'_>,
    cancel: Option<&RenderCancel>,
    policy: RenderPolicy<'_>,
    // §11.4.7 makes the PAGE a transparency group, and §11.3.4 makes its
    // `/CS` the blending colour space every element on the page composites
    // in. Read from the page dictionary by the caller, because this
    // function is handed a content stream and never sees one.
    blend_space: crate::compositor::BlendSpace,
) -> Diagnostics {
    run_nested(
        doc,
        content,
        resources,
        fonts,
        initial,
        canvas,
        0,
        Vec::new(),
        cancel,
        policy,
        // A page's own content stream has nothing above it to inherit
        // hiddenness from; `/OC` sections inside it start the stack.
        false,
        blend_space,
    )
}

/// One painted path recorded by [`trace_paths`], in the renderer's own
/// terms: the finished path's nodes in **user space** (as the interpreter's
/// `PathBuilder` built them, before any transform) plus the CTM captured at
/// the path's first construction op (`path_ctm`).
///
/// This exists solely so Pass 9a's `pdfce_core::vector` object model can be
/// cross-checked against the renderer's ACTUAL construction walk — not a
/// second copy of it — on the fixtures (decision 011's Z2 "agree by
/// construction" acceptance gate). Transform each node's endpoint by
/// [`TracedPath::ctm`] to get the page-space geometry the object model
/// stores in `PathObject::page_subpaths`.
#[derive(Debug, Clone)]
pub struct TracedPath {
    /// The finished path's segments, in construction order, user space.
    pub nodes: Vec<TracedNode>,
    /// The CTM captured at the path's first construction op.
    pub ctm: Transform,
    /// Whether the terminating operator filled.
    pub fill: bool,
    /// Whether the terminating operator stroked.
    pub stroke: bool,
}

/// One node of a [`TracedPath`], mirroring `tiny_skia`'s own path segments
/// (all PDF path construction lowers to moves, lines, and cubics — a PDF
/// content stream never emits a quadratic).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TracedNode {
    /// A `move_to` (subpath start), user space.
    Move(f32, f32),
    /// A `line_to`, user space.
    Line(f32, f32),
    /// A cubic `curve_to` — two control points then the endpoint, user
    /// space.
    Cubic(f32, f32, f32, f32, f32, f32),
    /// A subpath close.
    Close,
}

/// Trace the paths the interpreter builds for `content`, WITHOUT caring
/// about the pixels — the geometry oracle for the Pass 9a object-model
/// cross-check (see [`TracedPath`]).
///
/// It runs the **real** interpreter (the same `paint` path the renderer
/// uses), recording each finished path's nodes and captured CTM instead of
/// forking a second decomposition. Pass [`GraphicsState::default_with_ctm`]
/// with `Transform::identity()` as `initial` to trace in PDF user space,
/// matching `pdfce_core::vector::decompose(_, Matrix::IDENTITY, _)`.
///
/// Painting still happens (onto a throwaway pixmap) so the trace reflects
/// exactly what the renderer would draw; only top-level paths are traced
/// (a nested form's paths are the form's own concern and are not part of
/// this page-level cross-check).
#[must_use]
pub fn trace_paths(
    doc: &DocumentView<'_>,
    content: &ContentStream,
    resources: &Dict,
    fonts: &FontEnvironment,
    initial: GraphicsState,
    policy: RenderPolicy<'_>,
) -> Vec<TracedPath> {
    // A tiny throwaway target: we discard the pixels, so its size only has
    // to be non-zero for `Pixmap::new` to succeed.
    let Some(mut pixmap) = Pixmap::new(8, 8) else {
        return Vec::new();
    };
    // §8.7.2 PM3/PM5: pattern space is anchored to the DEFAULT coordinate
    // system of the page (or, for a pattern used inside a form XObject, of
    // the form) — NOT to the CTM in effect where `scn` or the fill occurs.
    // `initial.ctm` is exactly that space at every entry point: `run` is
    // handed the page's device transform, and `run_form_at` is handed the
    // form's, so the form case (PM4) comes out right without a second rule.
    //
    // Captured BEFORE the stack is built, because the first `cm` in the
    // stream would otherwise be indistinguishable from the page transform.
    let base_ctm = initial.ctm;
    let mut interp = Interpreter {
        policy,
        base_ctm,
        gs: GStateStack::new(initial),
        diag: Diagnostics::default(),
        // Geometry only — `trace_paths` records paths and
        // composites nothing, so §11.3.4 cannot apply. Additive
        // is the value that changes no behaviour rather than a
        // claim about the document.
        blend_space: crate::compositor::BlendSpace::Additive,
        path: PathBuilder::new(),
        subpixel_culling: policy.subpixel_culling,
        path_precise: false,
        path_ctm64: Mat64::IDENTITY,
        path_origin: None,
        path_ctm: None,
        current: None,
        subpath_start: None,
        needs_move: false,
        pending_clip: None,
        compat_depth: 0,
        mc_stack: Vec::new(),
        hidden_depth: 0,
        oc_off: None,
        resources,
        doc,
        fonts,
        text: None,
        clip_cache: crate::clip_cache::ClipCache::new(),
        font_cache: HashMap::new(),
        color: crate::color::ColorState::new(),
        depth: 0,
        active: Vec::new(),
        trace: Some(Vec::new()),
        // `trace_paths` is a diagnostic walk with no pixels and no
        // operator waiting on it; there is nothing to cancel.
        cancel: None,
    };
    for op in content.operations() {
        interp.execute(&op, content, &mut Canvas::paint(&mut pixmap));
    }
    interp.trace.unwrap_or_default()
}

/// The recursive body of [`run`]: interpret one content stream at
/// XObject nesting `depth`, with `active` naming the form XObjects
/// currently being executed further up the stack (the cycle guard).
///
/// A page's content stream is just the `depth == 0`, `active == []`
/// case — there is deliberately no separate "page" code path, because
/// §8.10 defines a form XObject as "a PDF content stream" and any
/// divergence between the two would be a bug waiting to happen (the
/// most likely one: a rule enforced for pages but not for the
/// appearance streams that annotations are made of).
#[allow(clippy::too_many_arguments)]
fn run_nested(
    doc: &DocumentView<'_>,
    content: &ContentStream,
    resources: &Dict,
    fonts: &FontEnvironment,
    initial: GraphicsState,
    canvas: &mut Canvas<'_>,
    depth: usize,
    active: Vec<ObjId>,
    cancel: Option<&RenderCancel>,
    policy: RenderPolicy<'_>,
    // `true` if the `Do` that invoked this form sits inside a hidden
    // `/OC` section, or the form's own `/OC` is off.
    //
    // Hiddenness is INHERITED and cannot be revoked from inside: a
    // visible `/OC` section nested within a hidden one stays hidden
    // (§8.11.3.1 — hidden content shall not be drawn, full stop). The
    // stream is still WALKED rather than skipped, so its fonts, images
    // and structural oddities are still counted; a form that vanishes
    // from the diagnostics is a form nobody can tell was there.
    hidden: bool,
    // §11.3.4's blending colour space for THIS stream — see
    // `Interpreter::blend_space`. Passed rather than derived because it is
    // decided at the group boundary the caller owns, and a callee that
    // re-derived it would have to re-read the isolation rule.
    blend_space: crate::compositor::BlendSpace,
) -> Diagnostics {
    // §8.7.2 PM3/PM5: pattern space anchors to this stream's DEFAULT
    // coordinate system, not to the CTM where the fill occurs. Captured
    // before the stack is built — after the first `cm` it is unrecoverable.
    let base_ctm = initial.ctm;
    let mut interp = Interpreter {
        policy,
        base_ctm,
        gs: GStateStack::new(initial),
        diag: Diagnostics::default(),
        blend_space,
        path: PathBuilder::new(),
        subpixel_culling: policy.subpixel_culling,
        path_precise: false,
        path_ctm64: Mat64::IDENTITY,
        path_origin: None,
        path_ctm: None,
        current: None,
        subpath_start: None,
        needs_move: false,
        pending_clip: None,
        compat_depth: 0,
        mc_stack: Vec::new(),
        hidden_depth: usize::from(hidden),
        oc_off: None,
        resources,
        doc,
        fonts,
        text: None,
        clip_cache: crate::clip_cache::ClipCache::new(),
        font_cache: HashMap::new(),
        color: crate::color::ColorState::new(),
        depth,
        active,
        trace: None,
        cancel,
    };
    for op in content.operations() {
        // THE CANCELLATION POLL. One relaxed load per operator — see
        // `crate::cancel`'s module docs for why that ordering is correct
        // rather than merely cheap, and why between-operators is the
        // right granularity (a single clip costs ~360 us, so the worst
        // case latency is a third of a millisecond, not the render).
        //
        // Breaking rather than returning leaves `interp.diag` intact, so
        // the caller still learns what was attempted. The half-painted
        // pixmap is the caller's to discard — `render_page_with_view`
        // turns a set flag into `RenderError::Cancelled` rather than
        // handing anyone a partial picture.
        if interp.cancel.is_some_and(RenderCancel::is_cancelled) {
            break;
        }
        interp.execute(&op, content, canvas);
    }
    interp.diag
}

/// Paint one annotation **appearance stream** (a form XObject, §12.5.5)
/// at a caller-computed placement, through the **existing** §8.10.1
/// form-execution path.
///
/// This is Pass 6.0's single seam between the annotation-placement code
/// ([`crate::annot`]) and the renderer. It exists so appearances go
/// through [`Interpreter::do_form`] — the *same* code the page's own
/// forms use — rather than a second, shorter copy. Routing through
/// `do_form` inherits, for free and correctly:
///
/// - **the `/AP` stream's OWN `/Resources`** (risk X8): `do_form` resolves
///   `Do`/`Tf`/`Cs` names against the appearance stream's resource
///   dictionary, never the page's — the correctness fix continuation-9
///   already paid for (a form's `/F1` is a different font than the
///   page's `/F1`). `resources_fallback` is used *only* for the §7.8.3
///   case-3 legacy fallback when the appearance has no `/Resources` of
///   its own; pass the page's resources.
/// - **the object-number cycle guard and [`MAX_XOBJECT_DEPTH`]**: a
///   self-referential or pathologically deep `/AP` is bounded exactly as
///   a page form is.
/// - **the per-interpreter font cache** and the fresh-state / discard
///   semantics of §8.10.1 steps (a)/(e).
///
/// ## The placement contract (§12.5.5, computed by [`crate::annot`])
///
/// `initial`'s CTM must already be **`A × base_device_ctm`**, where `A`
/// is the §12.5.5 step-b matrix mapping the transformed appearance box to
/// the annotation `/Rect`. `do_form` then concatenates the appearance's
/// own `/Matrix` on top (step b of §8.10.1), yielding the effective
/// transform **`AA = Matrix × A × base`** exactly as §12.5.5 requires
/// (`AA = Matrix × A`, then page geometry). The `/BBox` clip `do_form`
/// applies is therefore the appearance box mapped all the way to device
/// space — the correct clip. Do **not** fold `/Matrix` into `A` here; that
/// would apply it twice (the second-most-common annotation-render bug per
/// the §12.5.5 RAG).
///
/// Returns the appearance's own [`Diagnostics`] (form/glyph/image counters
/// for its content), which the caller merges into the page's. `forms_
/// rendered` is incremented by `do_form`, so an appearance is also counted
/// as a form — correct, because an appearance *is* a form XObject.
///
/// Over clippy's argument bound by one, since 2026-08-07's cancellation
/// parameter — the same `#[allow]` [`run_nested`] already carries, for
/// the same reason: these are the renderer's internal recursion seams,
/// and bundling their arguments into a struct would put a layer of
/// indirection between `do_form` and the state it is threading.
#[allow(clippy::too_many_arguments)]
pub fn run_form_at(
    doc: &DocumentView<'_>,
    stream: &Stream,
    id: Option<ObjId>,
    resources_fallback: &Dict,
    fonts: &FontEnvironment,
    initial: GraphicsState,
    pixmap: &mut Pixmap,
    cancel: Option<&RenderCancel>,
    policy: RenderPolicy<'_>,
) -> Diagnostics {
    run_form_at_on(
        doc,
        stream,
        id,
        resources_fallback,
        fonts,
        initial,
        &mut Canvas::paint(pixmap),
        cancel,
        policy,
    )
}

/// [`run_form_at`], against an arbitrary drawing target — see
/// [`run_on`] for why the pair exists.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_form_at_on(
    doc: &DocumentView<'_>,
    stream: &Stream,
    id: Option<ObjId>,
    resources_fallback: &Dict,
    fonts: &FontEnvironment,
    initial: GraphicsState,
    canvas: &mut Canvas<'_>,
    cancel: Option<&RenderCancel>,
    policy: RenderPolicy<'_>,
) -> Diagnostics {
    // §8.7.2 PM3/PM5: pattern space anchors to this stream's DEFAULT
    // coordinate system, not to the CTM where the fill occurs. Captured
    // before the stack is built — after the first `cm` it is unrecoverable.
    let base_ctm = initial.ctm;
    let mut interp = Interpreter {
        policy,
        base_ctm,
        gs: GStateStack::new(initial),
        diag: Diagnostics::default(),
        // §12.5.5: an appearance stream with no `/Group` is a
        // NON-ISOLATED group, and a non-isolated group INHERITS
        // its blending space (Table 147's `/CS` row). This entry
        // point is handed no parent to inherit from, so it takes
        // the value that changes nothing. Threading the page's
        // space in here is owed when annotations composite in
        // ink rather than on screen.
        blend_space: crate::compositor::BlendSpace::Additive,
        path: PathBuilder::new(),
        subpixel_culling: policy.subpixel_culling,
        path_precise: false,
        path_ctm64: Mat64::IDENTITY,
        path_origin: None,
        path_ctm: None,
        current: None,
        subpath_start: None,
        needs_move: false,
        pending_clip: None,
        compat_depth: 0,
        mc_stack: Vec::new(),
        hidden_depth: 0,
        oc_off: None,
        resources: resources_fallback,
        doc,
        fonts,
        text: None,
        clip_cache: crate::clip_cache::ClipCache::new(),
        font_cache: HashMap::new(),
        color: crate::color::ColorState::new(),
        depth: 0,
        active: Vec::new(),
        trace: None,
        // Threaded so an annotation appearance stops with the page it is
        // being painted onto. `do_form` recurses into `run_nested`, which
        // is where the poll actually lives.
        cancel,
    };
    interp.do_form(id, stream, canvas, false);
    interp.diag
}

/// Interpreter state for one content stream.
struct Interpreter<'a> {
    /// The CTM this content stream STARTED with — pattern space's anchor
    /// (§8.7.2 PM3/PM5, NOTE 1). Never changed by `cm`, which is the whole
    /// point: a pattern is immune to transformations in the stream that
    /// uses it.
    base_ctm: Transform,
    gs: GStateStack,
    diag: Diagnostics,
    /// §11.3.4's **blending colour space**, for the group this stream is
    /// the contents of.
    ///
    /// On the interpreter rather than on [`GraphicsState`] deliberately:
    /// it is a property of the GROUP NESTING, not of the graphics state,
    /// and `q`/`Q` must not be able to change it. A group establishes one
    /// at its boundary and every element inside it blends in that space
    /// until the next boundary.
    ///
    /// Inherited, not chosen, unless the group is isolated — Table 147's
    /// `/CS` row: *"if the group is non-isolated, `CS` shall be ignored
    /// and the colour space shall be inherited"*. ISO 32000-2 states the
    /// same rule from the other side in §11.6.6 (*"non-isolated groups
    /// shall inherit their colour space from the nearest ancestor isolated
    /// parent group"*), and cites the reason: converting the backdrop into
    /// a different space is not always possible, and would be an excessive
    /// number of conversions where it is.
    blend_space: crate::compositor::BlendSpace,
    /// The path under construction — in USER space normally, in DEVICE
    /// space when [`Self::path_precise`] is set (module docs).
    path: PathBuilder,
    /// CTM captured at the path's first construction op.
    path_ctm: Option<Transform>,
    /// Is the path under construction being built in DEVICE space?
    ///
    /// The second of `Pass 74.7`'s two algorithms, and the one with a
    /// per-point cost, so it is decided once per path in
    /// [`Interpreter::capture_path_ctm`] and never re-asked.
    ///
    /// Set when the CTM's translation is large enough that narrowing a
    /// page coordinate to `f32` before transforming it would move the
    /// point by more than 1/20 of a device pixel
    /// ([`Mat64::needs_precise_paths`]).
    ///
    /// When set, coordinates are stored RELATIVE to [`Self::path_origin`]
    /// and the difference is taken in `f64` — so the path holds small
    /// offsets rather than nearly-equal large numbers, and `tiny_skia`
    /// still receives the CTM's linear part and can stroke normally.
    path_precise: bool,
    /// The `f64` CTM captured with [`Self::path_ctm`], used to place the
    /// path when [`Self::path_precise`] is set.
    path_ctm64: Mat64,
    /// The path's own FIRST POINT, in user space, when
    /// [`Self::path_precise`] is set — the origin every later coordinate
    /// is expressed relative to.
    path_origin: Option<(f64, f64)>,
    /// Current point (§8.5.2.1), `None` = undefined.
    ///
    /// In the SAME space as [`Self::path`] — user space normally, device
    /// space under [`Self::path_precise`]. It is only ever fed back into
    /// the path (as `v`'s implicit control point, as `h`'s subpath
    /// return, as the move a segment operator needs), so keeping it in
    /// path space keeps it consistent by construction; a copy in user
    /// space would need converting at every one of those uses.
    current: Option<(f32, f32)>,
    /// Start point of the current subpath (for `h` and the
    /// after-`h`-new-subpath rule).
    subpath_start: Option<(f32, f32)>,
    /// After `h`/`re`, the next segment op must open a new subpath.
    needs_move: bool,
    /// Deferred clip rule set by `W`/`W*`, applied after the next
    /// paint op (§8.5.4).
    pending_clip: Option<FillRule>,
    /// `BX`/`EX` nesting depth (§7.8.2 Table 32; may nest).
    compat_depth: usize,
    /// One entry per open `BMC`/`BDC`, `true` if THAT level opened a
    /// hidden `/OC` section (§8.11.3.2).
    ///
    /// A stack rather than a counter because `EMC` closes the innermost
    /// section and only the stack knows whether that particular one was
    /// the hiding one. Marked content nests freely and mixes tags, so
    /// non-`/OC` levels are pushed as `false` — dropping them would make
    /// `EMC` pop the wrong entry, which is the difference between
    /// hiding one layer and hiding the rest of the page.
    ///
    /// Unbalanced streams are real: a surplus `EMC` pops nothing rather
    /// than underflowing, and levels left open at end of stream simply
    /// end with it.
    mc_stack: Vec<bool>,
    /// How many enclosing sections are hidden; `> 0` suppresses PAINT
    /// and nothing else (§8.11.3.1: hidden content "shall not be drawn",
    /// but colour, CTM, clip and text advance still apply).
    ///
    /// Derived from `mc_stack` but kept alongside it so the per-operator
    /// check is a comparison rather than a scan.
    hidden_depth: usize,
    /// OCGs that are OFF in the default configuration, computed lazily.
    ///
    /// `None` until the first `/OC` marked content or `/OC` XObject is
    /// met. Most content streams have neither, and the set costs a
    /// catalog walk — so a page without optional content pays nothing,
    /// and a nested form XObject inside one does not recompute it per
    /// invocation beyond its own first use.
    oc_off: Option<std::collections::BTreeSet<ObjId>>,
    resources: &'a Dict,
    /// The document, for resolving indirect resource/font entries.
    doc: &'a DocumentView<'a>,
    /// Substitute faces available to `Tf` (R19: supplied, never found).
    fonts: &'a FontEnvironment,
    /// The operator's rendering choices for the questions ISO 32000-1
    /// leaves open (R169) — the DeviceCMYK conversion (§8.6.4.4), the
    /// mask resampling filter (`SM-A1`), the minification filter
    /// (`IM-A1`) and the CMYK-JPEG polarity rule (`DCT-A1`).
    ///
    /// Carried **per render**, never read from a global: two renders of
    /// the same page must not be able to differ for a reason invisible at
    /// the call site. See `RenderPolicy`'s own docs.
    policy: RenderPolicy<'a>,
    /// `Tm`/`Tlm`, live only between `BT` and `ET` (§9.4.1). `None`
    /// outside a text object — which is how the positioning and showing
    /// operators detect the "shall only appear within text objects"
    /// violation without a separate flag.
    text: Option<TextObject>,
    /// The caller's cancellation flag, threaded down so a form XObject
    /// nested inside the page stops with it rather than running to
    /// completion inside an abandoned render.
    cancel: Option<&'a RenderCancel>,
    /// Already-built clip masks, so an identical clip applied again
    /// costs a pointer comparison instead of ~362 µs. Scoped to this
    /// content stream: masks are keyed partly on device size, and a
    /// cache outliving one render would be a leak and a hazard.
    clip_cache: crate::clip_cache::ClipCache,
    /// `Tf` results keyed by resource name.
    ///
    /// Loading a font walks the whole §9.6.6 encoding ladder over 256
    /// codes, and a content stream re-selects the same few fonts
    /// constantly (once per style run). Caching also makes
    /// `fonts_unsupported` and `substituted_fonts` count DISTINCT
    /// fonts, which is what those diagnostics mean.
    ///
    /// Scoped to ONE content stream, never shared with a nested form
    /// XObject: the key is a resource *name*, and names are scoped to
    /// the resource dictionary they came from (module docs).
    font_cache: HashMap<Vec<u8>, Option<Arc<LoadedFont>>>,
    /// How many form XObjects deep this interpreter is (0 = the page's
    /// own content stream). Bounded by [`MAX_XOBJECT_DEPTH`].
    depth: usize,
    /// Object numbers of the form XObjects currently executing, this
    /// one's callers included. Keyed on identity rather than resource
    /// name so a cycle reached through two different names is still
    /// caught (module docs).
    active: Vec<ObjId>,
    /// The §8.6 colour-space half of the graphics state: which space each
    /// of `sc`/`scn`'s operand runs is to be interpreted in, and whether
    /// painting in it marks the page at all. Carries its own `q`/`Q` stack
    /// (Table 52 makes the colour space graphics state), pushed and popped
    /// in lockstep with [`Interpreter::gs`]. See [`crate::color`].
    color: crate::color::ColorState,
    /// When `Some`, [`Interpreter::paint`] records each finished path here
    /// (nodes + captured CTM) instead of only painting it — the Pass 9a
    /// object-model cross-check oracle ([`trace_paths`]). `None` for
    /// ordinary rendering, so the render path is byte-for-byte unchanged.
    trace: Option<Vec<TracedPath>>,
    /// [`RenderOptions::subpixel_culling`], carried down so `do_form` can
    /// consult it without reaching back for the options.
    subpixel_culling: bool,
}

impl Interpreter<'_> {
    fn execute(&mut self, op: &Operation<'_>, content: &ContentStream, canvas: &mut Canvas<'_>) {
        let Some(name) = op.operator_name(&content.buf) else {
            // The only non-operator "operation" the projection yields is
            // a complete inline image (§8.9.7) — one indivisible
            // graphics object, params already normalized out of the
            // Table 93/94 abbreviations by `pdfce_core::content`, so it
            // takes exactly the same rendering path as an image
            // XObject.
            if let ContentTokenKind::InlineImage { params, data } = &op.operator.kind {
                match data.slice(&content.buf) {
                    // `ImageOrigin::Inline` is not cosmetic: §7.4.7
                    // forbids JBIG2 data in an inline image and §8.9.7
                    // gives no inline form for JPX, so the same
                    // dictionary that is legal as an XObject is not
                    // legal here.
                    Some(raw) => self.draw_image(params, raw, canvas, ImageOrigin::Inline),
                    None => self.diag.tolerated += 1,
                }
            } else {
                self.diag.tolerated += 1;
            }
            return;
        };

        // Operand accessors (tolerant: wrong types/counts are
        // diagnosed and the op skipped — never a panic, never an
        // abort of the whole page).
        let nums: Vec<f32> = op
            .operands
            .iter()
            .filter_map(|t| match &t.kind {
                ContentTokenKind::Operand(o) => o.as_number().map(|v| v as f32),
                _ => None,
            })
            .collect();

        match name {
            // ---- graphics state (Table 57) ----
            b"q" => {
                // The colour SPACE is a graphics-state parameter (Table
                // 52) and lives in its own stack, so it is pushed here —
                // gated on the same success, or the two stacks desync and
                // a `Q` restores one half of the state.
                if self.gs.push() {
                    self.color.push();
                } else {
                    self.diag.tolerated += 1;
                }
            }
            b"Q" => {
                if self.gs.pop() {
                    self.color.pop();
                } else {
                    self.diag.tolerated += 1; // unbalanced Q, tolerated
                }
            }
            b"cm" => {
                // ★ Composed in `f64`, from the operands' OWN `f64` values
                // rather than from the `f32` copies in `nums`.
                //
                // Both halves matter and the second is easy to miss. A
                // `cm` on a deep-zoom page routinely carries a page
                // coordinate — `540` — and composing it with a base CTM
                // whose scale is millions produces a translation that is
                // the difference of two ~4e9 numbers. In `f32`, whose
                // spacing there is 512, the answer came out quantised in
                // 512-pixel steps; see `Mat64`'s docs for the measured
                // consequence. Reading `nums` here would also narrow `e`
                // and `f` to `f32` BEFORE the multiply, which reinstates
                // the problem from the other end for a page whose `cm`
                // needs more than seven digits.
                if nums.len() == 6
                    && let Some(v) = operand_f64s(op, 6)
                {
                    let m = Mat64::from_row(v[0], v[1], v[2], v[3], v[4], v[5]);
                    // PRE-multiply: CTM' = M × CTM (§8.3.4).
                    self.gs
                        .current
                        .set_ctm64(m.post_concat(self.gs.current.ctm64));
                } else {
                    self.diag.tolerated += 1;
                }
            }
            b"w" => {
                if let &[lw] = nums.as_slice() {
                    self.gs.current.line_width = lw.max(0.0);
                }
            }
            b"J" => {
                self.gs.current.line_cap = match nums.first().copied() {
                    Some(v) if v as i32 == 1 => LineCap::Round,
                    Some(v) if v as i32 == 2 => LineCap::Square,
                    _ => LineCap::Butt,
                };
            }
            b"j" => {
                self.gs.current.line_join = match nums.first().copied() {
                    Some(v) if v as i32 == 1 => LineJoin::Round,
                    Some(v) if v as i32 == 2 => LineJoin::Bevel,
                    _ => LineJoin::Miter,
                };
            }
            b"M" => {
                if let &[ml] = nums.as_slice() {
                    self.gs.current.miter_limit = ml;
                }
            }
            b"d" => self.set_dash(op),
            b"i" | b"ri" => {} // flatness / rendering intent: recognized no-ops in Pass 1
            b"gs" => self.apply_ext_gstate(op, canvas),

            // ---- device colours (Table 74, §8.6.4) ----
            //
            // Each of these sets the colour SPACE as well as the colour —
            // Table 74's wording is "set the … colour space to DeviceGray
            // … AND set the gray level". The `set_device` call is what
            // makes a following `sc` read its operands in the space the
            // document just chose rather than one three operators
            // upstream. (§8.6.5.6's `DefaultGray`/`DefaultRGB`/
            // `DefaultCMYK` redirection is not implemented; see
            // `crate::color`'s module docs.)
            b"g" => {
                if let &[v] = nums.as_slice() {
                    self.gs.current.fill_color = Rgb::from_gray(v);
                    self.color
                        .set_device(crate::color::DeviceSpace::Gray, &[v], false);
                }
            }
            b"G" => {
                if let &[v] = nums.as_slice() {
                    self.gs.current.stroke_color = Rgb::from_gray(v);
                    self.color
                        .set_device(crate::color::DeviceSpace::Gray, &[v], true);
                }
            }
            b"rg" => {
                if let &[r, g, b] = nums.as_slice() {
                    self.gs.current.fill_color = Rgb::from_rgb(r, g, b);
                    self.color
                        .set_device(crate::color::DeviceSpace::Rgb, &[r, g, b], false);
                }
            }
            b"RG" => {
                if let &[r, g, b] = nums.as_slice() {
                    self.gs.current.stroke_color = Rgb::from_rgb(r, g, b);
                    self.color
                        .set_device(crate::color::DeviceSpace::Rgb, &[r, g, b], true);
                }
            }
            b"k" => {
                if let &[c, m, y, kk] = nums.as_slice() {
                    self.gs.current.fill_color =
                        Rgb::from_cmyk(self.policy.cmyk_intent, c, m, y, kk);
                    self.color
                        .set_device(crate::color::DeviceSpace::Cmyk, &[c, m, y, kk], false);
                }
            }
            b"K" => {
                if let &[c, m, y, kk] = nums.as_slice() {
                    self.gs.current.stroke_color =
                        Rgb::from_cmyk(self.policy.cmyk_intent, c, m, y, kk);
                    self.color
                        .set_device(crate::color::DeviceSpace::Cmyk, &[c, m, y, kk], true);
                }
            }

            // ---- colour spaces and non-device colour (Table 74, §8.6) ----
            //
            // These six were "recognized, deferred" until 2026-08-10, and
            // the consequence was not a missing feature but WRONG PIXELS: a
            // stream that selected a space and set a colour kept whatever
            // colour was previously in force and painted with it, silently.
            // Uppercase is stroking, lowercase non-stroking, universally.
            // `SC`/`SCN` (and `sc`/`scn`) are handled by one arm because
            // `SCN` is a strict superset and accepting its semantics for
            // `SC` is harmless on read (`iso32000__s__8.6.md`).
            b"cs" | b"CS" => {
                let stroking = name == b"CS";
                let initial = self.color.select(
                    self.doc,
                    self.resources,
                    last_name(op).as_deref(),
                    stroking,
                    self.policy.cmyk_intent,
                    &mut self.diag.color,
                );
                if let Some(rgb) = initial {
                    self.set_current_color(rgb, stroking);
                }
            }
            b"sc" | b"scn" | b"SC" | b"SCN" => {
                let stroking = matches!(name, b"SC" | b"SCN");
                // The trailing name is `scn`'s pattern operand (Table 74);
                // its PRESENCE, not the operand count, distinguishes the
                // two `scn` shapes, because an uncoloured tiling pattern
                // takes numbers *and* a name.
                let set = self.color.set(
                    &nums,
                    last_name(op).as_deref(),
                    stroking,
                    self.policy.cmyk_intent,
                    &mut self.diag.color,
                );
                if let Some(rgb) = set {
                    self.set_current_color(rgb, stroking);
                }
            }

            // ---- path construction (Table 59) ----
            b"m" => {
                if nums.len() == 2 {
                    self.capture_path_ctm();
                    let Some(v) = self.path_coords(op, &nums, 2) else {
                        self.diag.tolerated += 1;
                        return;
                    };
                    let (x, y) = (v[0], v[1]);
                    // Consecutive-`m` override (Table 59): PathBuilder
                    // naturally collapses a move_to followed by
                    // another move_to into the latter (no empty
                    // contour is emitted), matching the rule.
                    self.path.move_to(x, y);
                    self.current = Some((x, y));
                    self.subpath_start = Some((x, y));
                    self.needs_move = false;
                }
            }
            b"l" => {
                if nums.len() == 2 && self.begin_segment() {
                    let Some(v) = self.path_coords(op, &nums, 2) else {
                        self.diag.tolerated += 1;
                        return;
                    };
                    self.path.line_to(v[0], v[1]);
                    self.current = Some((v[0], v[1]));
                }
            }
            b"c" => {
                if nums.len() == 6 && self.begin_segment() {
                    let Some(v) = self.path_coords(op, &nums, 6) else {
                        self.diag.tolerated += 1;
                        return;
                    };
                    self.path.cubic_to(v[0], v[1], v[2], v[3], v[4], v[5]);
                    self.current = Some((v[4], v[5]));
                }
            }
            b"v" => {
                // First control point = CURRENT POINT (the v/y trap,
                // §8.5.2.2 Table 59).
                if nums.len() == 4
                    && self.begin_segment()
                    && let Some((cx, cy)) = self.current
                {
                    let Some(v) = self.path_coords(op, &nums, 4) else {
                        self.diag.tolerated += 1;
                        return;
                    };
                    // `cx, cy` is already in path space, which is why
                    // `current` is kept there — an affine map takes
                    // control points to control points, so a Bezier
                    // mapped point-by-point is the mapped Bezier.
                    self.path.cubic_to(cx, cy, v[0], v[1], v[2], v[3]);
                    self.current = Some((v[2], v[3]));
                }
            }
            b"y" => {
                // Second control point = ENDPOINT.
                if nums.len() == 4 && self.begin_segment() {
                    let Some(v) = self.path_coords(op, &nums, 4) else {
                        self.diag.tolerated += 1;
                        return;
                    };
                    self.path.cubic_to(v[0], v[1], v[2], v[3], v[2], v[3]);
                    self.current = Some((v[2], v[3]));
                }
            }
            b"h" => {
                self.path.close();
                // §8.5.2.1: `h` terminates the subpath; the current
                // point becomes the subpath start, and any following
                // segment op opens a NEW subpath there.
                self.current = self.subpath_start;
                self.needs_move = true;
            }
            b"re" => {
                if let &[_x, _y, _w, _h] = nums.as_slice() {
                    self.capture_path_ctm();
                    // `re`'s operands are an ORIGIN AND A SIZE, not two
                    // corners, so its four corner points have to be
                    // formed by addition BEFORE the transform — the sum
                    // is what carries the page magnitude, and adding
                    // after narrowing would put the quantisation back.
                    let Some(c) = (if self.path_precise {
                        operand_f64s(op, 4).map(|v| {
                            let (x, y, w, h) = (v[0], v[1], v[2], v[3]);
                            let (ox, oy) = *self.path_origin.get_or_insert((x, y));
                            #[allow(clippy::cast_possible_truncation)]
                            [(x, y), (x + w, y), (x + w, y + h), (x, y + h)]
                                .map(|(a, b)| ((a - ox) as f32, (b - oy) as f32))
                        })
                    } else {
                        let (x, y, w, h) = (nums[0], nums[1], nums[2], nums[3]);
                        Some([(x, y), (x + w, y), (x + w, y + h), (x, y + h)])
                    }) else {
                        self.diag.tolerated += 1;
                        return;
                    };
                    // Table 59's defined expansion: m, l, l, l, h — a
                    // COMPLETE subpath (a following segment op starts
                    // a new subpath at (x, y) per the h rule).
                    self.path.move_to(c[0].0, c[0].1);
                    self.path.line_to(c[1].0, c[1].1);
                    self.path.line_to(c[2].0, c[2].1);
                    self.path.line_to(c[3].0, c[3].1);
                    self.path.close();
                    self.current = Some(c[0]);
                    self.subpath_start = Some(c[0]);
                    self.needs_move = true;
                }
            }

            // ---- path painting (Table 60) + clipping (Table 61) ----
            b"S" => self.paint(canvas, false, true, None),
            b"s" => {
                self.path.close();
                self.paint(canvas, false, true, None);
            }
            b"f" | b"F" => self.paint(canvas, true, false, Some(FillRule::Winding)),
            b"f*" => self.paint(canvas, true, false, Some(FillRule::EvenOdd)),
            b"B" => self.paint(canvas, true, true, Some(FillRule::Winding)),
            b"B*" => self.paint(canvas, true, true, Some(FillRule::EvenOdd)),
            b"b" => {
                self.path.close();
                self.paint(canvas, true, true, Some(FillRule::Winding));
            }
            b"b*" => {
                self.path.close();
                self.paint(canvas, true, true, Some(FillRule::EvenOdd));
            }
            b"n" => self.paint(canvas, false, false, None),
            b"W" => self.pending_clip = Some(FillRule::Winding),
            b"W*" => self.pending_clip = Some(FillRule::EvenOdd),

            // ---- compatibility sections (Table 32) ----
            b"BX" => self.compat_depth += 1,
            b"EX" => self.compat_depth = self.compat_depth.saturating_sub(1),

            // ---- text objects (Table 107) ----
            b"BT" => {
                if self.text.is_some() {
                    // "Text objects shall not be nested; a second `BT`
                    // shall not appear before an `ET`" — real files do
                    // it anyway. §9.4's tolerance note: treat the inner
                    // `BT` as a re-initialization of Tm/Tlm.
                    self.diag.tolerated += 1;
                    self.diag.note(b"BT(nested)");
                }
                self.text = Some(TextObject::new());
            }
            b"ET" => {
                if self.text.take().is_none() {
                    self.diag.tolerated += 1;
                }
            }

            // ---- text state (Table 105) ----
            // These may legally appear OUTSIDE a text object and persist
            // across text objects (§9.3's scope rule), so none of them
            // checks `self.text`.
            b"Tc" => {
                if let &[v] = nums.as_slice() {
                    self.gs.current.text.char_spacing = v;
                }
            }
            b"Tw" => {
                if let &[v] = nums.as_slice() {
                    self.gs.current.text.word_spacing = v;
                }
            }
            b"Tz" => {
                // The operand is a PERCENTAGE; `Th` is the ratio.
                if let &[v] = nums.as_slice() {
                    self.gs.current.text.horizontal_scale = v / 100.0;
                }
            }
            b"TL" => {
                if let &[v] = nums.as_slice() {
                    self.gs.current.text.leading = v;
                }
            }
            b"Ts" => {
                if let &[v] = nums.as_slice() {
                    self.gs.current.text.rise = v;
                }
            }
            b"Tr" => {
                if let &[v] = nums.as_slice() {
                    let mode = v as i32;
                    // Modes 4–7 add glyphs to the clipping path, which
                    // this Pass defers (decision 004 §4.3). Their
                    // fill/stroke half IS honored — dropping that too
                    // would hide text that a conforming reader paints —
                    // but the clip is not applied, so the divergence is
                    // counted the first time it is requested.
                    if (4..=7).contains(&mode) {
                        self.diag.deferred_ops += 1;
                        self.diag.note(b"Tr(clip 4-7)");
                    }
                    self.gs.current.text.render_mode = u8::try_from(mode).unwrap_or(0);
                }
            }
            b"Tf" => self.select_font(op),

            // ---- text positioning (Table 108) ----
            // "The text-positioning operators shall only appear within
            // text objects" — outside one there is no Tlm to move.
            b"Td" => {
                if let &[tx, ty] = nums.as_slice() {
                    self.with_text_object(|t| t.next_line_offset(tx, ty));
                }
            }
            b"TD" => {
                // "the same effect as: −ty TL, then tx ty Td" — note
                // the NEGATION (`−15 TD` sets the leading to 15).
                if let &[tx, ty] = nums.as_slice() {
                    self.gs.current.text.leading = -ty;
                    self.with_text_object(|t| t.next_line_offset(tx, ty));
                }
            }
            b"Tm" => {
                if let &[a, b, c, d, e, f] = nums.as_slice() {
                    let m = Transform::from_row(a, b, c, d, e, f);
                    self.with_text_object(|t| t.set_matrix(m));
                }
            }
            b"T*" => self.next_line(),

            // ---- text showing (Table 109) ----
            b"Tj" => {
                if let Some(s) = last_string(op) {
                    self.show_string(&s, canvas);
                }
            }
            b"TJ" => self.show_array(op, canvas),
            b"'" => {
                // "the same effect as: T*, then string Tj".
                if let Some(s) = last_string(op) {
                    self.next_line();
                    self.show_string(&s, canvas);
                }
            }
            b"\"" => {
                // "aw ac string" — aw Tw, ac Tc, then `'`. The spacing
                // assignments PERSIST (§9.4.3): this is not a scoped
                // override.
                if let (&[aw, ac], Some(s)) = (nums.as_slice(), last_string(op)) {
                    self.gs.current.text.word_spacing = aw;
                    self.gs.current.text.char_spacing = ac;
                    self.next_line();
                    self.show_string(&s, canvas);
                } else {
                    self.diag.tolerated += 1;
                }
            }

            // ---- external objects (Table 87, §8.8) ----
            b"Do" => self.do_xobject(op, canvas),

            // ---- marked content, for optional content only (§14.6) ----
            //
            // pdfce does not build a marked-content TREE — structure,
            // artifacts and tagging are later work. It tracks marked
            // content for exactly one reason: `/OC` sections decide what
            // gets drawn (§8.11.3.2), and that cannot be answered without
            // knowing which sections are open.
            b"BDC" => self.begin_marked_content(op),
            b"BMC" => {
                // No property list, so it can never be an `/OC` section —
                // but it MUST be stacked, or the matching `EMC` closes
                // someone else's section.
                self.mc_stack.push(false);
                self.diag.deferred_ops += 1;
                self.diag.note(name);
            }
            b"EMC" => self.end_marked_content(),

            // `sh` — paint a shading directly in current user space
            // (§8.7.4.2, Table 77). The MODEL is resolved here and nothing
            // is painted yet; see `crate::shading`'s module docs for why a
            // non-painting slice ships on its own.
            //
            // It is lifted out of the deferred group below because
            // "deferred=52, names BDC/sh/BMC" cannot answer how many
            // gradients a page has, of which types, or whether the next
            // slice will fix it. Resolving the dictionary answers all
            // three, and costs one dictionary walk per `sh`.
            b"sh" => self.shading_operator(op, canvas),

            // ---- recognized, deferred to later slices ----
            b"MP" | b"DP" | b"d0" | b"d1" => {
                self.diag.deferred_ops += 1;
                self.diag.note(name);
            }

            // ---- unknown ----
            _ => {
                if self.compat_depth > 0 {
                    // Inside BX/EX: spec-sanctioned silent skip
                    // (operands were already consumed by the
                    // projection).
                    self.diag.compat_skipped += 1;
                } else {
                    self.diag.unknown_ops += 1;
                    self.diag.note(name);
                }
            }
        }
    }

    /// Write a colour into the stroking or non-stroking half of the
    /// graphics state (§8.6: the uppercase/lowercase operator pairs map
    /// one-for-one onto the two independent colour parameters).
    fn set_current_color(&mut self, rgb: Rgb, stroking: bool) {
        if stroking {
            self.gs.current.stroke_color = rgb;
        } else {
            self.gs.current.fill_color = rgb;
        }
    }

    /// Run `f` against the live [`TextObject`], or diagnose the §9.4.2
    /// violation of using a positioning operator outside `BT`…`ET`.
    fn with_text_object(&mut self, f: impl FnOnce(&mut TextObject)) {
        match self.text.as_mut() {
            Some(t) => f(t),
            None => self.diag.tolerated += 1,
        }
    }

    /// `T*` — "the same effect as the code `0 −Tl Td`" (Table 108). The
    /// leading is negated because going to the next line DECREASES y.
    fn next_line(&mut self) {
        let leading = self.gs.current.text.leading;
        self.with_text_object(|t| t.next_line_offset(0.0, -leading));
    }

    /// `Tf` — "`font` shall be the name of a font resource in the
    /// `Font` subdictionary of the current resource dictionary; `size`
    /// shall be a number representing a scale factor" (Table 105).
    ///
    /// A name that resolves to no font resource leaves `Tf` unset:
    /// §9.3 gives the font no initial value, so the honest behavior is
    /// to skip the text and diagnose, never to quietly pick a face.
    fn select_font(&mut self, op: &Operation<'_>) {
        let mut name: Option<Vec<u8>> = None;
        let mut size: Option<f32> = None;
        for tok in op.operands {
            if let ContentTokenKind::Operand(o) = &tok.kind {
                match o {
                    Object::Name(n) => name = Some(n.as_bytes().to_vec()),
                    other => {
                        if let Some(v) = other.as_number() {
                            size = Some(v as f32);
                        }
                    }
                }
            }
        }
        let (Some(name), Some(size)) = (name, size) else {
            self.diag.tolerated += 1;
            return;
        };
        self.gs.current.text.font_size = size;
        self.gs.current.text.font = self.load_font(&name);
    }

    /// Resolve a `/Font` resource name to a [`LoadedFont`], memoized
    /// (see [`Interpreter::font_cache`]). Every failure mode is counted
    /// exactly once per distinct resource name.
    fn load_font(&mut self, name: &[u8]) -> Option<Arc<LoadedFont>> {
        if let Some(hit) = self.font_cache.get(name) {
            return hit.clone();
        }
        let dict = self
            .resources
            .get(b"Font")
            .map(|o| self.doc.resolve(o))
            .and_then(Object::as_dict)
            .and_then(|fonts| fonts.get(name))
            .map(|o| self.doc.resolve(o))
            .and_then(Object::as_dict);

        let loaded = match dict {
            None => {
                // §7.8.3: the resource is simply not there. Not a font
                // problem — a structural one.
                self.diag.tolerated += 1;
                self.diag.note(b"Tf(missing resource)");
                None
            }
            Some(d) => match crate::text::load(self.doc, d, self.fonts) {
                Ok(font) => {
                    // R63: name the substitute in the bucket matching its
                    // trust level — bundled and supplied are disclosed
                    // separately, never conflated. Embedded fonts name
                    // nothing (the document's own program is exact).
                    let list = match font.source {
                        crate::font::GlyphSource::Bundled => Some(&mut self.diag.substituted_fonts),
                        crate::font::GlyphSource::Supplied => Some(&mut self.diag.supplied_fonts),
                        crate::font::GlyphSource::Embedded => None,
                    };
                    if let Some(list) = list
                        && list.len() < 32
                        && !list.contains(&font.base_font)
                    {
                        list.push(font.base_font.clone());
                    }
                    Some(Arc::new(font))
                }
                Err(reason) => {
                    // The whole font is out of this Pass's scope: its
                    // text is skipped, never approximated. The lump
                    // counter AND its by-reason bucket both advance (R20)
                    // so a batch report can say *why*, not just *that*.
                    self.diag.fonts_unsupported += 1;
                    *self
                        .diag
                        .fonts_unsupported_by_reason
                        .entry(reason.reason_key())
                        .or_insert(0) += 1;
                    None
                }
            },
        };
        self.font_cache.insert(name.to_vec(), loaded.clone());
        loaded
    }

    /// `TJ` — "each element of `array` shall be either a string or a
    /// number. If a string, show it. If a number, adjust the text
    /// position by that amount" (Table 109).
    fn show_array(&mut self, op: &Operation<'_>, canvas: &mut Canvas<'_>) {
        let items = op.operands.iter().rev().find_map(|t| match &t.kind {
            ContentTokenKind::Operand(Object::Array(a)) => Some(a.clone()),
            _ => None,
        });
        let Some(items) = items else {
            self.diag.tolerated += 1;
            return;
        };
        for item in &items {
            match item {
                Object::String(s) => self.show_string(s, canvas),
                other => {
                    if let Some(tj) = other.as_number() {
                        let tx = self.gs.current.text.adjustment(tj as f32);
                        self.with_text_object(|t| t.advance(tx, 0.0));
                    }
                }
            }
        }
    }

    /// Show one string: decode it to character codes, paint each
    /// glyph through `Trm × CTM`, and advance `Tm` (§9.4.3, §9.4.4).
    ///
    /// The font program is parsed ONCE per shown string rather than per
    /// glyph. It cannot be cached across calls because it borrows the
    /// `Arc`-held bytes, and skrifa's parse is lazy/zero-copy, so this
    /// is the cheap end of the tradeoff — the expensive part of font
    /// setup (the §9.6.6 encoding ladder) already happened at `Tf`.
    fn show_string(&mut self, string: &[u8], canvas: &mut Canvas<'_>) {
        // Cheap early outs, in the order that keeps the diagnostics
        // meaningful.
        let Some(font) = self.gs.current.text.font.clone() else {
            // §9.3: `Tf` has no initial value, so this is undefined
            // content, not a rendering shortfall.
            self.diag.tolerated += 1;
            self.diag.note(b"Tj(no font)");
            return;
        };
        if self.text.is_none() {
            // "The text-showing operators shall only appear within text
            // objects" (§9.4.3).
            self.diag.tolerated += 1;
            return;
        }

        // `data` must outlive `program`, which borrows it — declaration
        // order gives the reverse drop order that guarantees it.
        let data = font.data.clone();
        let program = FontProgram::parse(data.bytes()).ok();
        if program.is_none() {
            // The program parsed at `Tf` time (or `load` would have
            // failed) but not now — treat as an unusable program, the
            // same reason bucket `load` would have used.
            self.diag.fonts_unsupported += 1;
            *self
                .diag
                .fonts_unsupported_by_reason
                .entry(crate::text::UnsupportedFont::UnusableProgram.reason_key())
                .or_insert(0) += 1;
        }

        for code in font.codes(string) {
            let gid = match font.gid(code.value, program.as_ref()) {
                Some(g) => g,
                None => {
                    // §9.6.6.2 / §9.7.6.3: substitute `.notdef`, which
                    // in every well-formed program is GID 0 and usually
                    // empty or a hollow box. Either way the advance
                    // still happens, so the rest of the line stays put.
                    self.diag.glyphs_notdef += 1;
                    0
                }
            };
            self.paint_glyph(&font, program.as_ref(), gid, canvas);

            // §9.4.4's advance, applied whether or not anything was
            // painted — mode 3 (invisible) and a missing glyph both
            // still move the pen.
            let w0 = font.width(code.value) / 1000.0;
            let tx = self
                .gs
                .current
                .text
                .advance_for(w0, 0.0, code.word_spacing_applies);
            self.with_text_object(|t| t.advance(tx, 0.0));
        }
    }

    /// Paint one glyph per the current rendering mode (Table 106).
    ///
    /// The outline arrives in FONT units (rule R18: unhinted, y-up) and
    /// is transformed to USER space before painting, not to device
    /// space — because §9.3.6 requires stroked text to take its line
    /// width from the graphics state "in user space rather than in text
    /// space", and tiny-skia computes stroke geometry in the path's own
    /// coordinate system. The `CTM` is then handed to
    /// `fill_path`/`stroke_path` exactly as the path painter does.
    fn paint_glyph(
        &mut self,
        font: &LoadedFont,
        program: Option<&FontProgram<'_>>,
        gid: u32,
        canvas: &mut Canvas<'_>,
    ) {
        let ts = &self.gs.current.text;
        // Mode 3 (invisible — the OCR text-layer mode) and mode 7
        // (clip-only) paint nothing. Skipping the outline lookup here
        // is safe ONLY because this Pass does not implement text
        // clipping; when modes 4–7 land, mode 7 must still compute
        // outlines (§9.3.6's named trap).
        if !ts.fills() && !ts.strokes() {
            return;
        }
        let (Some(program), Some(tobj)) = (program, self.text) else {
            return;
        };
        let Ok(Some(path)) = program.outline(gid) else {
            // An empty outline is legitimate (a space); a draw failure
            // is not, but both are already reflected in what is on the
            // page, and `glyphs_notdef` covers selection failures.
            return;
        };
        let Some(path) = path.transform(ts.glyph_to_user(tobj.tm, program.upem())) else {
            self.diag.tolerated += 1;
            return;
        };

        // R63: count each painted substitute glyph at its own trust
        // level. Both bundled and supplied have exact positions (from
        // `/Widths`); only the shapes differ, and an operator needs to
        // tell pdfce's guess from their own supplied face.
        match font.source {
            crate::font::GlyphSource::Bundled => self.diag.glyphs_substituted += 1,
            crate::font::GlyphSource::Supplied => self.diag.glyphs_supplied += 1,
            crate::font::GlyphSource::Embedded => {}
        }
        let ctm = self.gs.current.ctm;
        // BORROWED, never cloned — see `paint_path`'s note. A glyph is
        // one more paint under the same page-sized mask.
        let clip = self.gs.current.clip_ref();
        crate::profile::note_paint(
            clip.mask.is_some(),
            paint_is_cullable(&path, ctm, self.gs.current.clip_bbox),
        );
        // MEASUREMENT ABLATIONS — see `paint_path`. A glyph is one more
        // paint under the same mask, so it must honour the same switches
        // or the floor would silently include text rasterization.
        // The ablation drops the MASK and only the mask. A recorded clip
        // id costs nothing per pixel, so removing it would not isolate
        // sampling cost — it would change what a recording MEANS.
        let clip = if crate::profile::skip_clip_sample() {
            ClipRef { mask: None, ..clip }
        } else {
            clip
        };
        // A glyph inside a hidden `/OC` section is not drawn — but the
        // caller has already advanced the text position, which is what
        // §8.11.3.1's "text advance still applies" requires. Suppressing
        // the ADVANCE would reflow the visible text around the hidden
        // run, so a layer toggle would move the rest of the line.
        let skip_paint = crate::profile::skip_paint() || self.oc_hidden();
        // §11.7.4.3 lists the elementary graphics objects overprint applies
        // to: "fills, strokes, TEXT, images, and shadings". Text is decided
        // here, before the paints, so the counter and the behaviour come
        // from one predicate — the same arrangement as `paint_path`.
        let op_fill = !skip_paint
            && self.gs.current.text.fills()
            && self.color.paints(false)
            && self.overprint_would_change(false);
        let op_stroke = !skip_paint
            && self.gs.current.text.strokes()
            && self.color.paints(true)
            && self.overprint_would_change(true);
        if op_fill {
            self.diag.overprint_effective += 1;
        }
        if op_stroke {
            self.diag.overprint_effective += 1;
        }

        // See the note above `paint_nonseparable`: text takes the same
        // composite as a path, and forgetting it is a documented past defect
        // rather than a hypothetical one.
        let ns_glyph = if skip_paint {
            None
        } else {
            self.gs.current.nonseparable
        };
        if !skip_paint
            && ns_glyph.is_none()
            && self.gs.current.text.fills()
            && self.color.paints(false)
            && !op_fill
        {
            let paint = self.solid_authored(
                false,
                self.gs.current.fill_alpha,
                self.gs.current.blend_mode,
            );
            // Glyph outlines are filled with the NONZERO winding rule
            // (§9.3.6: filling has "the same effects for a text object
            // as… for a path object"; counters in `o`/`e` are wound in
            // the opposite direction by the font, not by even-odd).
            canvas.fill(&path, &paint, FillRule::Winding, ctm, clip);
        }
        if !skip_paint
            && ns_glyph.is_none()
            && self.gs.current.text.strokes()
            && self.color.paints(true)
            && !op_stroke
        {
            let paint = self.solid_authored(
                true,
                self.gs.current.stroke_alpha,
                self.gs.current.blend_mode,
            );
            canvas.stroke(&path, &paint, &self.stroke_params(), ctm, clip);
        }

        if let Some(mode) = ns_glyph {
            let mut done = false;
            if self.gs.current.text.fills()
                && self.color.paints(false)
                && self.paint_nonseparable(&path, mode, Some(FillRule::Winding), false, canvas)
            {
                done = true;
            }
            if self.gs.current.text.strokes()
                && self.color.paints(true)
                && self.paint_nonseparable(&path, mode, None, true, canvas)
            {
                done = true;
            }
            if done {
                return;
            }
            // Fell through: the composite could not run. Paint normally and
            // disclose, never paint nothing.
            self.diag.blend_modes_ignored += 1;
        }

        // After the `clip` borrow ends, for the same reason `paint_path`
        // defers its composites: `paint_overprint` needs `&mut self`.
        if op_fill && !self.paint_overprint(&path, Some(FillRule::Winding), false, canvas) {
            self.diag.overprint_refused += 1;
            let paint = self.solid_authored(
                false,
                self.gs.current.fill_alpha,
                self.gs.current.blend_mode,
            );
            let clip = self.gs.current.clip_ref();
            canvas.fill(&path, &paint, FillRule::Winding, ctm, clip);
        }
        if op_stroke && !self.paint_overprint(&path, None, true, canvas) {
            self.diag.overprint_refused += 1;
            let paint = self.solid_authored(
                true,
                self.gs.current.stroke_alpha,
                self.gs.current.blend_mode,
            );
            let clip = self.gs.current.clip_ref();
            canvas.stroke(&path, &paint, &self.stroke_params(), ctm, clip);
        }
    }

    /// Capture the CTM at the path's first construction op; diagnose a
    /// mid-path `cm` (module docs).
    fn capture_path_ctm(&mut self) {
        match self.path_ctm {
            None => {
                self.path_ctm = Some(self.gs.current.ctm);
                self.path_ctm64 = self.gs.current.ctm64;
                // DECIDED ONCE, HERE, and not re-asked per coordinate.
                //
                // One condition, and it is about MAGNITUDE, not about
                // the shape of the transform: any affine CTM is
                // admissible, because the path keeps its user-space
                // linear part and only its ORIGIN moves. `path_origin` is
                // cleared here so the next path picks its own.
                self.path_precise = self.path_ctm64.needs_precise_paths();
                self.path_origin = None;
            }
            Some(m) if m != self.gs.current.ctm => {
                // Path already begun under a different CTM.
                self.diag.tolerated += 1;
            }
            Some(_) => {}
        }
    }

    /// Path-operator coordinates, in whatever space the current path is
    /// being built in.
    ///
    /// Normally this is just the already-narrowed `nums` — no work, no
    /// allocation, and the fast route is the default. Under
    /// [`Self::path_precise`] it re-reads the operands as `f64` and maps
    /// each `(x, y)` pair through the `f64` CTM, so the point reaches
    /// `tiny_skia` as a DEVICE coordinate that was never rounded at page
    /// magnitude.
    ///
    /// Re-reading rather than widening `nums` is deliberate: `nums` has
    /// already lost the digits this exists to keep. A point near
    /// `x = 540` has an `f32` spacing of `6.1e-5 pt` — 21.5 µm — so a
    /// 30 µm feature written in page coordinates is barely more than one
    /// representable step wide before any matrix touches it.
    fn path_coords(&mut self, op: &Operation<'_>, nums: &[f32], n: usize) -> Option<Vec<f32>> {
        if nums.len() != n {
            return None;
        }
        if !self.path_precise {
            return Some(nums.to_vec());
        }
        let v = operand_f64s(op, n)?;
        let (ox, oy) = *self.path_origin.get_or_insert((v[0], v[1]));
        #[allow(clippy::cast_possible_truncation)]
        Some(
            v.chunks_exact(2)
                .flat_map(|p| [(p[0] - ox) as f32, (p[1] - oy) as f32])
                .collect(),
        )
    }

    /// Common preamble for segment operators (`l c v y`): a segment
    /// with an undefined current point is a spec error (§8.5.2.1) —
    /// tolerated by skipping (diagnosed); after `h`/`re`, open the new
    /// subpath at the recorded point per the `h` rule.
    fn begin_segment(&mut self) -> bool {
        let Some((cx, cy)) = self.current else {
            self.diag.tolerated += 1;
            return false;
        };
        self.capture_path_ctm();
        if self.needs_move {
            self.path.move_to(cx, cy);
            self.subpath_start = Some((cx, cy));
            self.needs_move = false;
        }
        true
    }

    /// `d array phase` — dash pattern (§8.4.3.6). The array is ONE
    /// operand (an array object).
    fn set_dash(&mut self, op: &Operation<'_>) {
        let mut it = op.operands.iter().filter_map(|t| match &t.kind {
            ContentTokenKind::Operand(o) => Some(o),
            _ => None,
        });
        let (Some(arr), Some(phase)) = (it.next(), it.next()) else {
            self.diag.tolerated += 1;
            return;
        };
        let (Some(items), Some(phase)) = (arr.as_array(), phase.as_number()) else {
            self.diag.tolerated += 1;
            return;
        };
        let dashes: Vec<f32> = items
            .iter()
            .filter_map(|o| o.as_number().map(|v| v as f32))
            .collect();
        // §8.4.3.6: all-zero or negative entries are invalid; empty =
        // solid. Guard the degenerate cases tiny-skia would reject.
        if dashes.iter().any(|&v| v < 0.0)
            || (!dashes.is_empty() && dashes.iter().all(|&v| v == 0.0))
        {
            self.diag.tolerated += 1;
            return;
        }
        self.gs.current.dash = (dashes, phase as f32);
    }

    /// `gs` — apply an ExtGState by name from the resource dictionary
    /// (Table 58; the honored subset per the RAG's triage: LW, LC, LJ,
    /// ML, D; everything else recognized-and-deferred).
    fn apply_ext_gstate(&mut self, op: &Operation<'_>, canvas: &mut Canvas<'_>) {
        let name = op.operands.iter().rev().find_map(|t| match &t.kind {
            ContentTokenKind::Operand(Object::Name(n)) => Some(n.as_bytes()),
            _ => None,
        });
        // BOTH lookups resolve. Neither did until 2026-08-08, and the
        // consequence was that `gs` was a SILENT NO-OP on essentially
        // every real file.
        //
        // §7.3.10 lets any value be an indirect reference, and producers
        // overwhelmingly write `/ExtGState << /GS0 12 0 R >>` — the named
        // entry is a reference far more often than not. `as_dict()` on a
        // `Reference` returns `None`, so the whole chain collapsed to the
        // `tolerated += 1` arm below: no line width, no line cap, no dash
        // pattern, and no diagnostic an operator would recognise as "the
        // graphics state you asked for was ignored".
        //
        // Every other resource lookup in this file already resolved —
        // `/Font` at ~1428, `/XObject` at ~1799 — which is what makes this
        // a slip rather than a policy. Found by the benign-bucket audit
        // (`tools/render-parity/benign_structure.py`) on
        // `pdfium/testing/resources/multiple_graphics_states.pdf`, where
        // pdfium and Acrobat agree with each other and pdfce painted an
        // opaque rectangle with a 1 px stroke instead of a 25%-alpha one
        // with a 4-unit stroke.
        let ext = name
            .and_then(|n| {
                self.resources
                    .get(b"ExtGState")
                    .map(|o| self.doc.resolve(o))?
                    .as_dict()?
                    .get(n)
            })
            .map(|o| self.doc.resolve(o))
            .and_then(Object::as_dict);
        let Some(ext) = ext else {
            self.diag.tolerated += 1;
            return;
        };
        if let Some(v) = ext.get(b"LW").and_then(Object::as_number) {
            self.gs.current.line_width = (v as f32).max(0.0);
        }
        if let Some(v) = ext.get(b"LC").and_then(Object::as_int) {
            self.gs.current.line_cap = match v {
                1 => LineCap::Round,
                2 => LineCap::Square,
                _ => LineCap::Butt,
            };
        }
        if let Some(v) = ext.get(b"LJ").and_then(Object::as_int) {
            self.gs.current.line_join = match v {
                1 => LineJoin::Round,
                2 => LineJoin::Bevel,
                _ => LineJoin::Miter,
            };
        }
        if let Some(v) = ext.get(b"ML").and_then(Object::as_number) {
            self.gs.current.miter_limit = v as f32;
        }
        // D = [[dashes] phase]
        if let Some([arr, phase]) = ext
            .get(b"D")
            .and_then(Object::as_array)
            .and_then(|a| <&[Object; 2]>::try_from(a).ok())
            && let (Some(items), Some(phase)) = (arr.as_array(), phase.as_number())
        {
            let dashes: Vec<f32> = items
                .iter()
                .filter_map(|o| o.as_number().map(|v| v as f32))
                .collect();
            self.gs.current.dash = (dashes, phase as f32);
        }
        // §11.6.4.4 constant alpha. `/ca` is non-stroking, `/CA` is
        // stroking, both in 0..1 and both initially 1.0.
        //
        // These were "deferred" with NO COUNTER until 2026-08-09, which
        // made them invisible twice over: the page rendered fully opaque,
        // and nothing told the operator a transparency instruction had
        // been dropped. The benign-bucket audit found the cluster —
        // ~120 flagged pages across the veraPDF PDF/A transparency and
        // colour-space suites, where the fixture's own bookmark says
        // "The ExtGState contains the /ca key with value 0.5" and pdfce
        // painted the glyph solid black while pdfium and Acrobat painted
        // it 50% grey.
        //
        // Clamped rather than refused: §11.6.4.4 gives the range but a
        // malformed file is not a reason to abandon the page, and a
        // value outside 0..1 has an obvious intended reading at each end.
        if let Some(v) = ext.get(b"ca").and_then(Object::as_number) {
            self.gs.current.fill_alpha = (v as f32).clamp(0.0, 1.0);
        }
        // §11.3.5 `/BM` and §11.6.5 `/SMask` — clause 11 transparency.
        //
        // NEITHER IS IMPLEMENTED, and until this was added neither was
        // even COUNTED: `apply_ext_gstate` read `LW`, `LC`, `LJ`, `ML`,
        // `D`, `ca` and `CA` and silently dropped the rest. A page asking
        // for a Multiply blend or a soft mask rendered as though it had
        // asked for neither, with nothing on the result line to say so.
        //
        // `/BM` may be a name or an ARRAY of names (Table 58: "the first
        // blend mode in the array that the conforming reader supports").
        // `Normal` and `Compatible` are what pdfce actually does, so
        // selecting either is not a shortfall and is not counted — this
        // counts only the modes whose absence changes the picture.
        if let Some(bm) = ext.get(b"BM").map(|o| self.doc.resolve(o)) {
            let first = match bm {
                Object::Name(n) => Some(n.as_bytes().to_vec()),
                Object::Array(a) => a
                    .first()
                    .map(|o| self.doc.resolve(o))
                    .and_then(Object::as_name)
                    .map(|n| n.as_bytes().to_vec()),
                _ => None,
            };
            if let Some(name) = first {
                // ★ THE FOUR NON-SEPARABLE MODES ARE RESOLVED FIRST, and
                // they never reach `blend_mode_from_name` — which cannot
                // express them, by design (decision 066). pdfce computes
                // these itself in `crate::blend_nonsep` because the
                // rasteriser's are wrong by up to 107/255.
                if let Some(ns) = crate::blend_nonsep::NonSeparableBlend::from_name(&name) {
                    self.gs.current.nonseparable = Some(ns);
                    // Left at `SourceOver` on purpose — see the field docs.
                    self.gs.current.blend_mode = tiny_skia::BlendMode::SourceOver;
                    self.diag.blend_modes_applied += 1;
                    // §11.3.5.3 makes these FOUR worse than the separable
                    // ones in a subtractive space, not merely different:
                    // K is SELECTED rather than blended, from the backdrop
                    // for Hue/Saturation/Color and from the source for
                    // Luminosity. Computed additively they blend all four
                    // channels, which is plausible and non-conforming.
                    if self.blend_space.is_subtractive() && canvas.cmyk_mut().is_none() {
                        self.diag.blends_in_wrong_space += 1;
                    }
                } else {
                    self.gs.current.nonseparable = None;
                    match crate::gstate::blend_mode_from_name(&name) {
                        Some(mode) => {
                            self.gs.current.blend_mode = mode;
                            // Census counts the modes that CHANGE something.
                            // Counting `Normal` too would put a large number on
                            // ordinary documents — producers emit `/BM /Normal`
                            // constantly to reset inherited state — and train
                            // every reader to ignore the counter.
                            if mode != tiny_skia::BlendMode::SourceOver {
                                self.diag.blend_modes_applied += 1;
                                // §11.3.4: a non-`Normal` mode selected
                                // while the blending space is subtractive
                                // will be computed on the wrong side of
                                // the complement. `Normal` is deliberately
                                // NOT counted — it is `c_s` in either
                                // space, so a `DeviceCMYK` page that only
                                // ever composites Normal is correct.
                                if self.blend_space.is_subtractive() && canvas.cmyk_mut().is_none()
                                {
                                    self.diag.blends_in_wrong_space += 1;
                                }
                            }
                        }
                        None => {
                            // An unknown name is NOT a reason to refuse the
                            // paint: the marks belong on the page and only the
                            // compositing rule is in doubt. Fall back to
                            // Normal, and say so.
                            self.gs.current.blend_mode = tiny_skia::BlendMode::SourceOver;
                            self.diag.blend_modes_ignored += 1;
                            self.diag.note(
                                format!(
                                    "gs /BM /{}: blend mode not applied; composited as Normal",
                                    String::from_utf8_lossy(&name)
                                )
                                .as_bytes(),
                            );
                        }
                    }
                }
            }
        }
        // §11.6.5's soft mask. `/None` is the RESET (Table 58), and it is
        // a real state change now that a mask can actually be in force.
        if let Some(sm) = ext.get(b"SMask").map(|o| self.doc.resolve(o)) {
            let is_none = matches!(sm, Object::Name(n) if n.as_bytes() == b"None")
                || matches!(sm, Object::Null);
            if is_none {
                // Restore the clip as it stood before the mask was folded
                // in. See `GraphicsState::clip_before_smask`: the mask was
                // MULTIPLIED into the clip, so it cannot be divided back
                // out and the pre-multiplication value has to be kept.
                if let Some(saved) = self.gs.current.clip_before_smask.take() {
                    if self.gs.current.clips_since_smask > 0 {
                        // A `W n` landed while the mask was in force, so
                        // the snapshot predates it and restoring would
                        // discard that clip. Counted rather than silently
                        // producing the wrong clip region.
                        self.diag.soft_masks_reset_stale += 1;
                        self.diag.note(
                            b"gs /SMask /None after an intervening clip; \
                              pre-mask clip not restored exactly",
                        );
                    } else {
                        self.gs.current.clip = saved;
                        self.gs.current.clip_bbox = None;
                    }
                }
                self.gs.current.clips_since_smask = 0;
                self.gs.current.soft_mask = None;
            } else if let Some(dict) = sm.as_dict().cloned() {
                match self.build_soft_mask(&dict, canvas) {
                    Some(mask) => {
                        // Fold into the clip, which every paint site in
                        // this renderer already honours. A soft mask
                        // multiplies coverage exactly as a clip does, so
                        // this is the operation, not an approximation of
                        // it — §8.5.4 NOTE 2 guarantees a clip only ever
                        // shrinks, and a mask only ever shrinks too.
                        if self.gs.current.clip_before_smask.is_none() {
                            self.gs.current.clip_before_smask = Some(self.gs.current.clip.clone());
                            self.gs.current.clips_since_smask = 0;
                        }
                        // Keep the mask ITSELF, un-folded: §11.4.5 needs
                        // it as a value a group composite can apply once,
                        // not as a coverage multiplier already fused into
                        // the clip. See `GraphicsState::soft_mask`.
                        self.gs.current.soft_mask = Some(std::sync::Arc::new(mask.clone()));
                        let combined = match self.gs.current.clip.as_deref() {
                            Some(old) => {
                                let mut m = mask;
                                let old_data = old.data().to_vec();
                                for (n, o) in m.data_mut().iter_mut().zip(old_data.iter()) {
                                    *n = ((u16::from(*n) * u16::from(*o)) / 255) as u8;
                                }
                                m
                            }
                            None => mask,
                        };
                        self.gs.current.clip = Some(std::sync::Arc::new(combined));
                        // The mask covers the whole page, so any cached
                        // clip bbox is now wrong in the direction that
                        // culls too much. Dropping it costs a cull, which
                        // is the safe side.
                        self.gs.current.clip_bbox = None;
                        self.diag.soft_masks_applied += 1;
                    }
                    None => {
                        self.diag.soft_masks_ignored += 1;
                        self.diag.note(
                            b"gs /SMask: mask group could not be built; marks painted unmasked",
                        );
                    }
                }
            } else {
                self.diag.soft_masks_ignored += 1;
                self.diag
                    .note(b"gs /SMask: not a dictionary; marks painted unmasked");
            }
        }
        if let Some(v) = ext.get(b"CA").and_then(Object::as_number) {
            self.gs.current.stroke_alpha = (v as f32).clamp(0.0, 1.0);
        }
        // §8.6.7 OVERPRINT. Table 58's own rule, which is the easy thing to
        // get wrong: `/OP` sets BOTH the stroking and non-stroking overprint
        // parameters, UNLESS `/op` appears in the SAME dictionary — in which
        // case `/op` takes the non-stroking one. So the order below is
        // load-bearing, not stylistic.
        let bool_of = |k: &[u8]| {
            matches!(
                ext.get(k).map(|o| self.doc.resolve(o)),
                Some(Object::Boolean(true))
            )
        };
        let has = |k: &[u8]| ext.get(k).is_some();
        if has(b"OP") {
            let v = bool_of(b"OP");
            self.gs.current.overprint_stroke = v;
            if !has(b"op") {
                self.gs.current.overprint_fill = v;
            }
        }
        if has(b"op") {
            self.gs.current.overprint_fill = bool_of(b"op");
        }
        if let Some(v) = ext
            .get(b"OPM")
            .map(|o| self.doc.resolve(o))
            .and_then(|o| match o {
                Object::Integer(i) => Some(*i),
                _ => None,
            })
        {
            self.gs.current.overprint_mode = v;
        }
        if self.gs.current.overprint_stroke || self.gs.current.overprint_fill {
            self.diag.overprint_requested += 1;
            if self.gs.current.overprint_mode == 1 {
                self.diag.overprint_mode1_requested += 1;
            }
        }
        // Remaining Table 58 keys (BM/SMask/Font/…): still deferred.
    }

    // -----------------------------------------------------------------
    // External objects — `Do` (§8.8) and inline images (§8.9.7)
    // -----------------------------------------------------------------

    /// `name Do` — "paint the specified XObject" (Table 87).
    ///
    /// Dispatch is on **`/Subtype`**, not `/Type`: Table 87 says the
    /// stream's `Type` entry is checked only "if present", while both
    /// Table 89 (image) and Table 95 (form) mark `Subtype` Required.
    /// The three subtypes behave completely differently:
    ///
    /// - `Image` → rasterize through the unit-square mapping (§8.9.4).
    /// - `Form`  → recursive content-stream execution (§8.10.1).
    /// - `PS`    → **silent no-op.** §8.8.1 says PostScript XObjects
    ///   "should not be used" and a non-PostScript conforming reader
    ///   ignores them; that is correct behaviour, not a shortfall, so it
    ///   is deliberately not counted as a deferral.
    ///
    /// An unresolvable `name`, a non-stream target, and a missing
    /// `Subtype` are all spec-undefined or malformed; each is a no-op
    /// plus a `tolerated` diagnostic rather than a failed page.
    /// Is painting currently suppressed by an enclosing hidden `/OC`
    /// section?
    ///
    /// The ONLY thing this may gate is the blit. §8.11.3.1 is explicit
    /// that hidden content still participates in everything else —
    /// colour, CTM, clip and text advance persist — so a hidden run of
    /// text must still move the text position, and a hidden `W n` must
    /// still tighten the clip for the visible content that follows.
    /// Gating anything wider makes the page LAYOUT depend on layer
    /// state, which is the one thing toggling a layer must not do.
    fn oc_hidden(&self) -> bool {
        self.hidden_depth > 0
    }

    /// The default configuration's OFF set, computed on first use.
    ///
    /// Lazy because most content streams contain no optional content at
    /// all, and the set costs a walk of `/OCProperties` (§8.11.4.2).
    fn oc_off_set(&mut self) -> &std::collections::BTreeSet<ObjId> {
        // An operator override REPLACES the document's default
        // configuration; it is not merged with it. See
        // `crate::layer_state`'s module docs for why — every merge rule
        // would be a rendering decision invisible at the call site.
        if let Some(v) = self.policy.layers {
            return v.hidden_set();
        }
        let doc = self.doc;
        let magnification = self.policy.view_magnification;
        self.oc_off.get_or_insert_with(|| {
            let mut off = pdfce_core::annot::optional_content_default_off(doc);
            // §8.11.4.5: only a VIEWER examines `/AS`. `None` here is a
            // print or aggregate render, for which the `/D`-initial state
            // is not just adequate but required.
            if let Some(magnification) = magnification {
                // The notes are dropped HERE on purpose: this is the
                // render path, and it has no channel to report them on.
                // The shells surface them from their own call.
                let _ = pdfce_core::annot::apply_view_usage(doc, &mut off, magnification);
            }
            off
        })
    }

    /// `BDC` — open a marked-content section, and decide whether it hides.
    ///
    /// # Only the `/OC` tag is interpreted
    ///
    /// Every `BDC` is stacked so `EMC` stays balanced, but only tag
    /// `/OC` can hide. `/Span`, `/Artifact`, `/P` and the rest are
    /// structure and tagging — later work, and irrelevant to painting.
    ///
    /// # The property list must be an INDIRECT resource
    ///
    /// §8.11.3.2: the operand "shall be a named resource in the
    /// `/Properties` subdictionary" precisely because OCGs and OCMDs are
    /// indirect objects, and visibility is keyed on object identity. An
    /// inline dictionary therefore has no identity to key on, so it
    /// cannot be resolved against the OFF set and the section is treated
    /// as VISIBLE and counted as tolerated.
    ///
    /// Showing content pdfce could not classify is the right way to be
    /// wrong here: a hidden-by-mistake region is content silently
    /// missing from the page with nothing on screen to suggest it, while
    /// a shown-by-mistake region is visible and therefore arguable.
    fn begin_marked_content(&mut self, op: &Operation<'_>) {
        let mut names = op.operands.iter().filter_map(|t| match &t.kind {
            ContentTokenKind::Operand(Object::Name(n)) => Some(n.as_bytes().to_vec()),
            _ => None,
        });
        let tag = names.next();
        if tag.as_deref() != Some(b"OC".as_slice()) {
            self.mc_stack.push(false);
            self.diag.deferred_ops += 1;
            self.diag.note(b"BDC");
            return;
        }
        // The second name is the /Properties key. Absent = an inline
        // dictionary operand, which §8.11.3.2 forbids for /OC.
        let Some(key) = names.next() else {
            self.mc_stack.push(false);
            self.diag.tolerated += 1;
            self.diag.note(b"BDC(/OC without a /Properties name)");
            return;
        };
        let doc = self.doc;
        let id = self
            .resources
            .get(b"Properties")
            .map(|o| doc.resolve(o))
            .and_then(Object::as_dict)
            .and_then(|props| props.get(&key))
            .and_then(|entry| entry.as_reference());
        let Some(id) = id else {
            self.mc_stack.push(false);
            self.diag.tolerated += 1;
            self.diag
                .note(b"BDC(/OC name not an indirect /Properties entry)");
            return;
        };
        let hidden = {
            let off = self.oc_off_set().clone();
            pdfce_core::annot::oc_is_hidden(doc, id, &off)
        };
        self.mc_stack.push(hidden);
        if hidden {
            self.hidden_depth += 1;
            self.diag.oc_sections_hidden += 1;
        }
    }

    /// `EMC` — close the innermost marked-content section.
    ///
    /// A surplus `EMC` (more closes than opens) pops nothing and is
    /// counted. Real files do this, and the alternative — underflowing
    /// the hidden depth — would un-hide content that is still inside an
    /// open hidden section, which is strictly worse than ignoring a
    /// stray operator.
    fn end_marked_content(&mut self) {
        match self.mc_stack.pop() {
            Some(true) => self.hidden_depth = self.hidden_depth.saturating_sub(1),
            Some(false) => {}
            None => {
                self.diag.tolerated += 1;
                self.diag.note(b"EMC(without a matching BMC/BDC)");
            }
        }
    }

    /// `sh` — paint a shading directly in current user space
    /// (ISO 32000-1 §8.7.4.2, Table 77).
    ///
    /// # What this does today: resolve and report, do not paint
    ///
    /// The shading MODEL is built (`crate::shading::Shading::load`) and
    /// classified; nothing reaches the pixmap. The geometry that would
    /// turn a device point into a parametric coordinate — §8.7.4.5, ISO
    /// 32000-1 Tables 79–84 — is in the spec corpus for the analytic types
    /// (1, 2, 3) and **not** for the meshes (4–7), so the painting slice
    /// is scoped to the former. Project rule 1 forbids writing the rest
    /// from recall. `crate::shading`'s module docs carry the full
    /// rationale for shipping the model without the paint.
    ///
    /// # Why resolving is worth doing before painting can
    ///
    /// This operator was previously grouped with `MP`/`DP`/`d0`/`d1` as
    /// "recognized, deferred", which produced exactly one fact —
    /// `deferred=52, first names BDC, sh, BMC` — and that fact cannot
    /// distinguish an axial gradient pdfce is about to be able to draw
    /// from a type 7 tensor-patch mesh that is a much larger job. Walking
    /// the dictionary answers it, and the cost is one lookup per `sh`.
    ///
    /// # The anchoring note, recorded where the mistake would be made
    ///
    /// When this does paint, it paints in **current user space** — the CTM
    /// in effect right here — and it fills the **current clip region**,
    /// not a path. That is the opposite of a `PatternType 2` fill, whose
    /// coordinates are pattern space and therefore immune to a `cm` in
    /// this stream (§8.7.2 NOTE 1). Table 77 states the contrast itself,
    /// and also that this route **ignores `/Background`**. The route is
    /// carried into the model as [`crate::shading::PaintRoute::ShOperator`]
    /// rather than assumed at the paint site.
    fn shading_operator(&mut self, op: &Operation<'_>, canvas: &mut Canvas<'_>) {
        let doc = self.doc;
        let resources = self.resources;

        self.diag
            .shading
            .reached(crate::shading::PaintRoute::ShOperator);

        let Some(name) = last_name(op) else {
            self.diag.shading.refused += 1;
            self.diag.tolerated += 1;
            self.diag.note(b"sh(no shading name operand)");
            return;
        };
        let entry = resources
            .get(b"Shading")
            .map(|o| doc.resolve(o))
            .and_then(Object::as_dict)
            .and_then(|shadings| shadings.get(&name));
        let Some(entry) = entry else {
            self.diag.shading.refused += 1;
            self.diag.tolerated += 1;
            self.diag.note(b"sh(missing Shading resource)");
            return;
        };

        let intent = self.policy.cmyk_intent;
        let shading = crate::shading::Shading::load(
            doc,
            entry,
            resources,
            intent,
            &mut self.diag.color,
            &mut self.diag.shading,
        );
        let Some(shading) = shading else {
            return;
        };
        if shading.is_paintable() {
            self.diag.shading.paintable += 1;
        }

        // §8.11.3.1: hidden optional content is not drawn, and everything
        // else still runs — the shading is still resolved and still
        // counted above, exactly as a hidden path is still consumed.
        if self.oc_hidden() || crate::profile::skip_paint() {
            return;
        }

        // Table 77: `sh` takes no path and "applies the corresponding
        // gradient fill directly to current user space". So the paint AREA
        // is the current clip region, and the ANCHORING is the current
        // CTM — inverted here, because the painter walks device pixels and
        // needs to ask where each one lands in the shading's own space.
        //
        // A non-invertible CTM is a degenerate transform (a zero scale)
        // under which nothing has any area to be painted in; skipped
        // rather than approximated.
        let ctm = self.gs.current.ctm;
        let Some(to_target) = ctm.invert() else {
            self.diag.tolerated += 1;
            self.diag.note(b"sh(non-invertible CTM)");
            return;
        };

        // The region: the clip's own device-space bounds when there is a
        // clip, the whole pixmap when there is not. §8.7.4.2's `should`
        // that `sh` "be applied only to bounded or geometrically defined
        // shadings" is exactly this case — an unbounded shading with no
        // clip legitimately fills the page.
        #[allow(clippy::cast_possible_truncation)]
        let region = match self.gs.current.clip_bbox {
            Some((l, t, r, b)) => (
                l.floor() as i32,
                t.floor() as i32,
                r.ceil() as i32,
                b.ceil() as i32,
            ),
            None => (0, 0, canvas.width() as i32, canvas.height() as i32),
        };

        let clip = self.gs.current.clip.as_deref();
        let alpha = self.gs.current.fill_alpha;
        // A shading is evaluated PER DESTINATION PIXEL, so it needs the
        // destination itself rather than a recordable draw. See
        // `Canvas::pixmap_mut`: a target that cannot hand one over is one
        // that cannot reproduce this operator, and the honest answer there
        // is to refuse the whole recording rather than to drop the shading.
        canvas.refuse(PoisonReason::Shading);
        // ★ BRIDGED, NOT NATIVE, AND THE REASON IS UPSTREAM OF THIS CALL.
        // `ColorRamp::at` resolves a shading's colour to three-channel
        // sRGB when the ramp is BUILT, so by the time the pixel loop runs
        // there are no colorants left to composite. Evaluating the ramp in
        // ink is `Pass 97.1g`; until then the shading paints into a
        // transparent scratch with the same evaluator, and its RESULT
        // crosses into the colorant buffer -- so a shading on a
        // subtractive page composites against the page in ink even though
        // it is not authored in ink.
        if let Some(buf) = canvas.cmyk_mut() {
            let (w, h) = (buf.width(), buf.height());
            let Some(mut scratch) = tiny_skia::Pixmap::new(w, h) else {
                self.diag.shading.refused += 1;
                return;
            };
            if shading
                .paint(to_target, region, clip, alpha, &mut scratch)
                .is_some()
            {
                buf.composite_srgb(
                    &scratch,
                    clamp_region(region, w, h),
                    1.0,
                    crate::compositor::Blend::Normal,
                );
                self.diag.shading.painted += 1;
            }
            return;
        }
        let Some(dest) = canvas.pixmap_mut() else {
            self.diag.shading.refused += 1;
            return;
        };
        if let Some(pixels) = shading.paint(to_target, region, clip, alpha, dest) {
            // `painted` counts SHADINGS drawn, not pixels — the pixel
            // count is the return value and is used only to decide whether
            // anything landed. A shading that resolved and painted zero
            // pixels (fully clipped, or `/Extend` false at both ends and
            // the geometry missing the region) is still a shading pdfce
            // drew correctly, so it counts.
            let _ = pixels;
            self.diag.shading.painted += 1;
        }
    }

    /// Paint `path` with the `/Pattern` the current colour selects
    /// (§8.7.2, §8.7.4.3). Returns `true` if pixels were laid down.
    ///
    /// # The anchoring rule, which is the whole difficulty
    ///
    /// A pattern's coordinates are **pattern space**, mapped to the
    /// *default* coordinate space of the content stream by the pattern's
    /// own `/Matrix` — NOT by the CTM in effect at the fill. §8.7.2 NOTE 1
    /// states it plainly, and PM5's `shall` is the binding form: "the
    /// pattern matrix maps pattern space to the default coordinate system
    /// of the pattern's parent content stream". So the transform is
    /// `base_ctm x /Matrix`, and a `cm` between selecting the pattern and
    /// filling with it must not move the gradient.
    ///
    /// That is the exact opposite of the `sh` operator, which paints in
    /// CURRENT user space (Table 77). The two routes share this crate's
    /// painter and differ only in the matrix handed to it and in the area
    /// painted — `sh` fills the clip region, a pattern fills the path.
    /// Getting the two confused produces a gradient that is in the right
    /// place until the page is scaled, which is why the routes are carried
    /// explicitly as [`crate::shading::PaintRoute`] rather than inferred.
    ///
    /// # What is not done here
    ///
    /// `PatternType 1` (tiling) is counted and not painted. It needs the
    /// pattern's own content stream run into a tile and replicated on
    /// `/XStep`/`/YStep`, which is a different job from evaluating an
    /// analytic function per pixel.
    /// Would honouring overprint have changed this paint?
    ///
    /// Implements the §11.7.4.3 selection rule as a PREDICATE rather than
    /// as a blend, because pdfce cannot yet perform the blend: the answer
    /// is "yes" exactly when `CompatibleOverprint` would have chosen the
    /// backdrop component for at least one component of the destination.
    ///
    /// Two ways that happens, and both are checked here:
    ///
    /// 1. **Overprint mode 1** with a DeviceCMYK source — any component
    ///    whose value is zero leaves the backdrop unchanged (§8.6.7). A
    ///    DeviceCMYK fill with a zero in it is the common case, so this is
    ///    the branch that fires on real prepress files.
    /// 2. **A source space specifying fewer components than DeviceCMYK** —
    ///    a `Separation`, or a `DeviceN` with fewer than four colorants.
    ///    The process components the source does not name must survive, and
    ///    an RGB composite cannot let them.
    ///
    /// A full DeviceCMYK source at mode 0 is deliberately NOT counted: it
    /// specifies all four components, selects the source for all four, and
    /// is identical to Normal. Counting it would put a large number on
    /// ordinary documents and hide the cases that matter.
    fn overprint_would_change(&self, stroking: bool) -> bool {
        let on = if stroking {
            self.gs.current.overprint_stroke
        } else {
            self.gs.current.overprint_fill
        };
        if !on {
            return false;
        }
        let Some((space, comps)) = self.color.device_color(stroking) else {
            // No resolvable source colour — a pattern, or a refused
            // operand count. Not classifiable, so not counted.
            return false;
        };
        // §8.6.6.3, the same resolution `paint_overprint` does — and it has
        // to be done in BOTH, because this predicate is what decides
        // whether that one is called at all. An `Indexed` space that fell
        // to the `_ => false` arm below was never even counted as an
        // effective overprint, so the disclosure under-reported by exactly
        // the set the renderer then failed to honour.
        let resolved = space.indexed_entry(comps);
        let (space, comps) = resolved
            .as_ref()
            .map_or((space, comps), |(b, c)| (*b, c.as_slice()));
        match space {
            crate::color::ColorSpace::DeviceCmyk => {
                // Mode 1 only: at mode 0 all four components are specified
                // and the blend degenerates to Normal.
                self.gs.current.overprint_mode == 1 && comps.contains(&0.0)
            }
            // Fewer components than the process set: the ones the source
            // does not name must survive from the backdrop.
            crate::color::ColorSpace::Separation { .. } => true,
            crate::color::ColorSpace::DeviceN { names, .. } => names.len() < 4,
            // DeviceGray and DeviceRGB over a CMYK backdrop are a
            // colour-space question this predicate deliberately does not
            // answer: §11.7.4.3 is about the components of the CURRENT
            // space, and pdfce has no group colour space to compare
            // against until the n-channel buffer exists. Not counted, and
            // that is a known under-count rather than a claim of zero.
            _ => false,
        }
    }

    /// Build the soft mask a `gs` `/SMask` dictionary describes
    /// (§11.6.5), or [`None`] if it cannot be built.
    ///
    /// # The contract, clause by clause
    ///
    /// * **§11.5.3** — a `/Luminosity` mask renders its group `/G` over a
    ///   **fully opaque** backdrop of colour `/BC` (the spec's `α₀ = 1`),
    ///   then takes the luminosity of the composite. The opacity of that
    ///   backdrop is easy to miss and changes the answer for a
    ///   non-isolated mask group.
    /// * **Table 144** — `/BC`'s default is *"the colour space's initial
    ///   value, representing black"*. Table 74 gives that value per space,
    ///   and the trap is that it is all-zeros for RGB and Gray but
    ///   `[0 0 0 1]` for `DeviceCMYK`. A renderer that defaults to "all
    ///   zeros" therefore gets **black in RGB and pure WHITE in CMYK** — a
    ///   mask wide open exactly where it should be shut. pdfce defaults to
    ///   black *as a colour*, so the CMYK case is right by construction
    ///   rather than by a special case that could be forgotten.
    /// * **§11.6.5.2** — outside the group's bounding box the mask value is
    ///   neither 0 nor 1: it is `TR(lum(BC))` for `/Luminosity` and
    ///   `TR(0.0)` for `/Alpha`. Implemented by pre-filling the buffer with
    ///   the backdrop *before* rendering the group into it, so "outside the
    ///   bbox" is simply "where the group did not paint" and needs no case
    ///   of its own. Under the defaults that value is 0 — everything
    ///   outside a defaulted-`/BC` luminosity mask is fully masked out.
    /// * **§11.5.3 NOTE 3** — device-space luminosity is
    ///   `0.30 R + 0.59 G + 0.11 B`, with **no** gamma compensation. It is
    ///   deliberately *not* Rec.709 (`0.2126/0.7152/0.0722`) and the values
    ///   are deliberately *not* linearised first; the standard says so in
    ///   as many words, and both "corrections" are tempting.
    /// * **§11.6.5.2** — the mask's coordinate system is `/Matrix`
    ///   concatenated with the CTM **at the moment the mask is established
    ///   by `gs`**, not the CTM at paint time. Building the mask eagerly,
    ///   here inside the `gs` handler, is what makes that true by
    ///   construction. A lazily-evaluated mask would silently use the wrong
    ///   matrix under any later `cm`.
    /// * **Table 147** — `/CS` is *Required* in a luminosity mask group's
    ///   attributes, precisely because such a group is rootless: it does
    ///   **not** inherit the page group's colour space, and making it do so
    ///   "for consistency" is a named error.
    ///
    /// # Not yet honoured, and counted rather than assumed away
    ///
    /// `/TR` (Table 144) is read and, when it is anything other than the
    /// name `/Identity`, counted and disclosed. Evaluating it needs the
    /// function machinery threaded into the render crate; until then a
    /// document that inverts its mask through `/TR` gets the un-inverted
    /// mask and **says so** rather than looking correct.
    fn build_soft_mask(&mut self, sm: &Dict, canvas: &mut Canvas<'_>) -> Option<Mask> {
        let doc = self.doc;
        let subtype = match doc.resolve(sm.get(b"S")?) {
            Object::Name(n) => n.as_bytes().to_vec(),
            _ => return None,
        };
        let luminosity = subtype == b"Luminosity";
        if !luminosity && subtype != b"Alpha" {
            return None;
        }

        // Table 144: `/TR` is applied LAST, after the luminosity or alpha
        // computation, once. Not implemented; disclosed.
        if let Some(tr) = sm.get(b"TR").map(|o| doc.resolve(o))
            && !matches!(tr, Object::Name(n) if n.as_bytes() == b"Identity")
            && !matches!(tr, Object::Null)
        {
            self.diag.soft_mask_transfer_ignored += 1;
            self.diag
                .note(b"gs /SMask /TR: transfer function not applied; mask used untransformed");
        }

        let g_obj = doc.resolve(sm.get(b"G")?);
        let Object::Stream(gs_stream) = g_obj else {
            return None;
        };
        let g_dict = gs_stream.dict.clone();
        let raw = doc.slice(gs_stream.data_span)?;
        let bytes = filters::decode_stream(&g_dict, raw).ok()?;
        let content = ContentStream::parse(bytes).ok()?;

        let mut buf = Pixmap::new(canvas.width(), canvas.height())?;
        if luminosity {
            // §11.5.3's opaque backdrop. This fill is ALSO what makes
            // "outside the BBox" correct, so it is not merely an
            // initialisation.
            buf.fill(self.soft_mask_backdrop(&g_dict, sm));
        }

        // §11.6.6: inside a group the blend mode is Normal, both alpha
        // constants are 1.0 and the soft mask is None — "to ensure that
        // they are not applied twice". That applies with particular force
        // here, where applying the mask under construction to its own
        // construction would be circular.
        let mut inner = GraphicsState::default_with_ctm(self.gs.current.ctm);
        // §8.10.2 step (b): concatenate /Matrix, THEN clip to /BBox in the
        // already-transformed space. Same order as `do_form`.
        if let Some(m) = matrix_entry64(doc, &g_dict) {
            inner.set_ctm64(m.post_concat(inner.ctm64));
        }
        if let Some(rect) = rect_entry(doc, &g_dict, b"BBox") {
            if rect.width() <= 0.0 || rect.height() <= 0.0 {
                // A degenerate BBox paints nothing, so the mask is the
                // backdrop everywhere — which `buf` already holds.
                return Self::mask_from_buffer(&buf, luminosity, canvas);
            }
            let path = PathBuilder::from_rect(rect);
            let form_ctm = inner.ctm;
            intersect_clip(
                &mut inner,
                &path,
                FillRule::Winding,
                form_ctm,
                canvas,
                &mut self.clip_cache,
            );
        }

        let resources = match g_dict
            .get(b"Resources")
            .map(|o| doc.resolve(o))
            .and_then(Object::as_dict)
        {
            Some(own) => own,
            None => self.resources,
        };

        // §11.6.5 / Table 147: a luminosity mask group's `/CS` is
        // **Required**, precisely because such a group is rootless — it
        // does NOT inherit the page's blending space, and making it do so
        // "for consistency" is a named error. So the space is read from
        // the mask group's own dictionary, and a mask group missing `/CS`
        // (already counted above) falls to additive, which is what pdfce
        // does everywhere today.
        let mask_space = g_dict
            .get(b"Group")
            .map(|o| doc.resolve(o))
            .and_then(Object::as_dict)
            .and_then(|g| g.get(b"CS"))
            .and_then(|cs| {
                crate::color::resolve_object(
                    doc,
                    doc.resolve(cs),
                    resources,
                    0,
                    &mut self.diag.color,
                )
            })
            .map_or(crate::compositor::BlendSpace::Additive, |sp| {
                crate::compositor::BlendSpace::of(&sp)
            });
        let nested = run_nested(
            doc,
            &content,
            resources,
            self.fonts,
            inner,
            &mut Canvas::paint(&mut buf),
            self.depth + 1,
            // The mask group's own recursion guard set. Cloned rather than
            // borrowed because `self` is already mutably borrowed here; a
            // mask group is built once per `gs`, not per paint, so the
            // clone is not on a hot path. It must still be a REAL copy of
            // the active set, not an empty one, or a form that invokes
            // itself through a soft mask would recurse unguarded.
            self.active.clone(),
            self.cancel,
            self.policy,
            false,
            mask_space,
        );
        self.diag.merge(nested);
        Self::mask_from_buffer(&buf, luminosity, canvas)
    }

    /// Turn a rendered mask-group buffer into per-pixel mask coverage.
    ///
    /// Split out so the degenerate-`/BBox` path and the ordinary path
    /// cannot disagree about the conversion — the two differ only in
    /// whether anything was painted into the buffer.
    fn mask_from_buffer(buf: &Pixmap, luminosity: bool, canvas: &Canvas<'_>) -> Option<Mask> {
        let mut mask = Mask::new(canvas.width(), canvas.height())?;
        let data = mask.data_mut();
        for (i, px) in buf.pixels().iter().enumerate() {
            let a = f32::from(px.alpha()) / 255.0;
            let v = if luminosity {
                // Un-premultiply, then §11.5.3 NOTE 3's device formula.
                // Weights 0.30/0.59/0.11, values NOT linearised — the
                // standard forbids gamma compensation here explicitly.
                let (r, g, b) = if a <= 0.0 {
                    (0.0, 0.0, 0.0)
                } else {
                    (
                        f32::from(px.red()) / 255.0 / a,
                        f32::from(px.green()) / 255.0 / a,
                        f32::from(px.blue()) / 255.0 / a,
                    )
                };
                0.30_f32.mul_add(r, 0.59_f32.mul_add(g, 0.11 * b))
            } else {
                a
            };
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            {
                *data.get_mut(i)? = (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            }
        }
        Some(mask)
    }

    /// The opaque backdrop colour a `/Luminosity` mask composites over.
    ///
    /// Table 144's `/BC`, defaulting to the group colour space's initial
    /// value — black in every space Table 74 defines, which is why the
    /// fallback here is literally black rather than "all components zero".
    /// Those are the same thing in RGB and Gray and **opposite** things in
    /// `DeviceCMYK`, where all-zeros is white.
    fn soft_mask_backdrop(&mut self, g_dict: &Dict, sm: &Dict) -> tiny_skia::Color {
        let doc = self.doc;
        let black = tiny_skia::Color::BLACK;
        let Some(arr) = sm
            .get(b"BC")
            .map(|o| doc.resolve(o))
            .and_then(Object::as_array)
        else {
            return black;
        };
        let comps: Vec<f32> = arr
            .iter()
            .filter_map(|o| doc.resolve(o).as_number())
            .map(|v| v as f32)
            .collect();
        if comps.is_empty() {
            return black;
        }
        // Table 147 makes `/CS` Required for a luminosity mask group, and
        // it is the space `/BC`'s components are expressed in. A mask group
        // does NOT inherit the page group's space — it is rootless, which
        // is exactly why `/CS` is Required rather than Optional.
        //
        // Resolved by COMPONENT COUNT rather than by loading the space.
        // `/BC` is optional and rare, its default (black) is already
        // correct for every space Table 74 lists, and the only thing that
        // matters when it IS present is the polarity trap: four components
        // are subtractive, so `[0 0 0 1]` is black and `[0 0 0 0]` is
        // white. One and three components are additive and zero is black
        // in both. Anything else is disclosed rather than guessed.
        let has_cs = g_dict
            .get(b"Group")
            .map(|o| doc.resolve(o))
            .and_then(Object::as_dict)
            .is_some_and(|g| g.contains_key(b"CS"));
        if !has_cs {
            self.diag.tolerated += 1;
            self.diag
                .note(b"gs /SMask /BC without group /CS (Required, Table 147)");
        }
        // THE SAME CONVERSION ROUTE THE PAINT PATH USES, and this is a
        // correction: `/BC` originally went through an inline naive
        // complement `(1−c)(1−k)`, justified by a comment claiming the
        // naive form "is right for the polarity question, which is all
        // this is used for".
        //
        // That justification was FALSE, and falsely reassuring, which is
        // the worse half. §11.6.5.2 makes this backdrop's *magnitude* the
        // mask value everywhere outside the group's `/BBox` — that is
        // exactly what the pre-fill in `build_soft_mask` implements — so
        // the number matters, not just its polarity. Meanwhile painted
        // content inside the same mask group reached luminosity through
        // `Rgb::from_cmyk`'s calibrated 6⁴ grid. One mask, one luminosity
        // computation, two different CMYK→sRGB routes feeding it.
        //
        // Found by the librarian while filing the commit that introduced
        // it. The lesson is not "use the calibrated table" — it is that a
        // comment asserting why a shortcut is safe is a claim, and this
        // one was never checked against the clause it was standing in.
        let Rgb { r, g, b } = match *comps.as_slice() {
            [v] => Rgb::from_gray(v),
            [r, g, b] => Rgb::from_rgb(r, g, b),
            [c, m, y, k] => Rgb::from_cmyk(self.policy.cmyk_intent, c, m, y, k),
            _ => {
                self.diag.tolerated += 1;
                self.diag
                    .note(b"gs /SMask /BC with unhandled component count - assuming black");
                return black;
            }
        };
        tiny_skia::Color::from_rgba(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0), 1.0)
            .unwrap_or(black)
    }

    /// The current paint colour as **authored subtractive tints**, or
    /// `None` when the source colour space does not state any.
    ///
    /// # Why the interpreter is the only place that can answer this
    ///
    /// Because the answer dies one call later. `ColorState` keeps the
    /// operands of the last colour-setting operator, un-converted and
    /// `q`/`Q`-correct; `GraphicsState::fill_color` and `stroke_color` are
    /// plain `Rgb`. Every paint site in this crate reads the second and
    /// never consults the first, so the colorants are present in the
    /// graphics state and absent from the paint — which is exactly the gap
    /// `BrushSpec::cmyk` closes.
    ///
    /// # The `Indexed` resolution, which is not optional
    ///
    /// §8.6.6.3: an `Indexed` operand is an **index**, not a colour. It is
    /// resolved to its palette entry in the base space before anything asks
    /// a colorant question, because `overprint::classify` recurses into the
    /// base independently and the row and the tints agree only if this
    /// happens too.
    ///
    /// # `None` is a real answer
    ///
    /// It means "this space states no tints", not "something failed". The
    /// colorant buffer then converts the resolved sRGB with
    /// `overprint::rgb_to_cmyk`, which is §11.6.6's required conversion —
    /// and is measurably not the same thing as an authored value, which is
    /// why the two are distinguished all the way down to the paint.
    fn authored_cmyk(&self, stroking: bool) -> Option<[f32; 4]> {
        let (space, comps) = self.color.device_color(stroking)?;
        let resolved = space.indexed_entry(comps);
        let (space, comps) = resolved
            .as_ref()
            .map_or((space, comps), |(b, c)| (*b, c.as_slice()));
        let kind = crate::overprint::classify(space, false)?;
        crate::overprint::authored_tints(&kind, comps)
    }

    /// A solid paint at `alpha`, carrying its authored colorants when the
    /// file stated any.
    ///
    /// Replaces ten call sites that read `gs.current.{fill,stroke}_color`
    /// and built a `BrushSpec` from it alone. Taking `stroking` rather than
    /// the colour means the colour and the colorants are read from the same
    /// half of the graphics state by construction — passing the colour in
    /// separately is how a fill paint would end up carrying the stroke's
    /// ink, and that mistake would be invisible on every additive page.
    ///
    /// The quantisation itself is untouched: `BrushSpec::solid` still
    /// produces the same bytes, so the sRGB path does not move.
    fn solid_authored(&self, stroking: bool, alpha: f32, blend: tiny_skia::BlendMode) -> BrushSpec {
        let colour = if stroking {
            self.gs.current.stroke_color
        } else {
            self.gs.current.fill_color
        };
        let spec = BrushSpec::solid(colour, alpha, blend);
        match self.authored_cmyk(stroking) {
            Some(cmyk) => spec.with_cmyk(cmyk),
            None => spec,
        }
    }

    /// Paint one path with `CompatibleOverprint` instead of `Normal`.
    ///
    /// Called only where [`Self::overprint_would_change`] is true, which is
    /// the same predicate that has been *counting* effective overprints
    /// since the disclosure Pass. That symmetry is deliberate: the number
    /// the operator was already being shown is exactly the set of paints
    /// this now renders differently, so the disclosure and the behaviour
    /// cannot drift apart.
    ///
    /// # Returns
    ///
    /// `true` if the overprint composite ran and the caller should skip its
    /// ordinary paint; `false` if it could not run, in which case the caller
    /// paints normally. A `false` return is **disclosed** by the caller —
    /// silently falling back to a normal paint is precisely the "sneaky"
    /// failure rule 4 forbids.
    fn paint_overprint(
        &mut self,
        path: &Path,
        rule: Option<FillRule>,
        stroking: bool,
        canvas: &mut Canvas<'_>,
    ) -> bool {
        use crate::overprint;

        let Some((space, comps)) = self.color.device_color(stroking) else {
            return false;
        };
        // §8.6.6.3: an `Indexed` operand is an INDEX, not a colour. Resolve
        // it to the palette entry in the base space before anything asks a
        // question about colorants — see `ColorSpace::indexed_entry`, and
        // note that `overprint::classify` recurses into the base
        // independently, so the row and the tints agree only if this
        // happens too.
        let resolved = space.indexed_entry(comps);
        let (space, comps) = resolved
            .as_ref()
            .map_or((space, comps), |(b, c)| (*b, c.as_slice()));
        let Some(kind) = overprint::classify(space, false) else {
            return false;
        };

        // The source colour as SUBTRACTIVE TINTS, which is what Table 149
        // is written in. The rule lives in `overprint::authored_tints`
        // because the colorant buffer needs the same answer and the two
        // must not be able to disagree -- see that function for why a
        // `DeviceCMYK` or `Separation`/`DeviceN` source is READ rather than
        // converted, and for the Ghent patch that discriminates.
        //
        // `None` means the space states no tints of its own, and the only
        // thing left is to reconstruct them from the paint colour the
        // pipeline already resolved. The recovery is exact under
        // `rgb_to_cmyk`/`cmyk_to_rgb` (see their docs), and which CHANNELS
        // that source is entitled to paint is decided from the colorant
        // names by `cmyk_group_rules`, not from these numbers.
        let source_cmyk: [f32; 4] = overprint::authored_tints(&kind, comps).unwrap_or_else(|| {
            let c = if stroking {
                self.gs.current.stroke_color
            } else {
                self.gs.current.fill_color
            };
            overprint::rgb_to_cmyk(c.r, c.g, c.b)
        });

        let op = if stroking {
            self.gs.current.overprint_stroke
        } else {
            self.gs.current.overprint_fill
        };
        let rules = overprint::cmyk_group_rules(
            &kind,
            source_cmyk,
            op,
            // `/OPM` is stored as the i64 the file carried, because
            // OP-N2 records that values other than 0 and 1 have NO
            // specified behaviour and pdfce keeps what it read rather
            // than normalising it away. Anything that is not exactly 1
            // is mode 0 -- the conservative reading, pinned by a test.
            u8::from(self.gs.current.overprint_mode == 1),
        );

        // Coverage: the path, rasterised exactly as tiny_skia would have
        // rasterised it for a normal paint, then intersected with the clip.
        // Using the same rasteriser is what keeps an overprinted edge
        // identical in shape to a non-overprinted one.
        let ctm = self.gs.current.ctm;
        let Some(mut coverage) = Mask::new(canvas.width(), canvas.height()) else {
            return false;
        };
        if let Some(r) = rule {
            coverage.fill_path(path, r, true, ctm);
        } else {
            let Some(stroked) = path.clone().stroke(&self.stroke_params(), 1.0) else {
                return false;
            };
            coverage.fill_path(&stroked, FillRule::Winding, true, ctm);
        }
        if let Some(old) = self.gs.current.clip.as_deref() {
            let old_data = old.data().to_vec();
            for (n, o) in coverage.data_mut().iter_mut().zip(old_data.iter()) {
                *n = ((u16::from(*n) * u16::from(*o)) / 255) as u8;
            }
        }

        // Restrict the scan to the path's device-space bounds; outside them
        // coverage is zero and the per-pixel CMYK round trip would be pure
        // waste. A full-page scan is ~8x slower on a typical patch.
        let Some(device_path) = path.clone().transform(ctm) else {
            return false;
        };
        let b = device_path.bounds();
        // A stroke extends beyond the path's own bounds by half the line
        // width, and the join/cap can add more. Padding by the full width
        // is cheap and cannot under-cover.
        let pad = if rule.is_some() {
            1.0
        } else {
            self.gs.current.line_width.mul_add(0.5, 2.0)
        };
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let region = (
            (b.left() - pad).floor().max(0.0) as u32,
            (b.top() - pad).floor().max(0.0) as u32,
            ((b.right() + pad).ceil().max(0.0) as u32).min(canvas.width()),
            ((b.bottom() + pad).ceil().max(0.0) as u32).min(canvas.height()),
        );
        if region.0 >= region.2 || region.1 >= region.3 {
            // Entirely off-page. The composite ran correctly and touched
            // nothing, which is a success, not a fallback.
            return true;
        }

        let alpha = if stroking {
            self.gs.current.stroke_alpha
        } else {
            self.gs.current.fill_alpha
        };
        // §11.7.4.3's composite reads the destination back, so like `sh`
        // it needs real pixels. `false` here means "could not run", and the
        // caller's documented response is to paint normally AND disclose —
        // never to paint nothing.
        canvas.refuse(PoisonReason::Overprint);
        // ★ THE SUBTRACTIVE PATH IS THE ONE THIS OPERATOR WAS ALWAYS
        // WRITTEN FOR. Table 149 selects per COLORANT, and until the
        // colorant buffer existed the backdrop's colorants had to be
        // reconstructed from an sRGB composite on every pixel. Here they
        // are simply read. Same rules, same coverage, same rasteriser --
        // only the backdrop stops being a guess.
        if let Some(buf) = canvas.cmyk_mut() {
            let changed = buf.composite_overprint(
                &coverage,
                region,
                rules,
                source_cmyk,
                alpha.clamp(0.0, 1.0),
            );
            self.diag.overprint_composited += 1;
            self.diag.overprint_pixels += u64::from(changed);
            return true;
        }
        let Some(dest) = canvas.pixmap_mut() else {
            return false;
        };
        let changed = overprint::composite(
            dest,
            &coverage,
            rules,
            source_cmyk,
            alpha.clamp(0.0, 1.0),
            region,
        );
        self.diag.overprint_composited += 1;
        self.diag.overprint_pixels += u64::from(changed);
        true
    }

    /// Composite a path through one of §11.3.5.3's four **non-separable**
    /// blend modes, per pixel.
    ///
    /// # Why this is a separate path and not a `tiny_skia::BlendMode`
    ///
    /// The rasteriser's four are measurably wrong (decision 066), so pdfce
    /// computes Table 137 itself in [`crate::blend_nonsep`]. The shape here
    /// is [`Self::paint_overprint`]'s, deliberately: **rasterise the paint
    /// to a coverage mask with the SAME rasteriser a normal paint uses**,
    /// intersect the clip into it, then blend per pixel inside the path's
    /// own device bounds. Sharing the rasteriser is what keeps a
    /// non-separably-blended edge the same SHAPE as an ordinary one.
    ///
    /// That machinery did not exist when these modes were refused. It
    /// arrived with `Pass 85.5`, which is why this became cheap rather than
    /// architectural.
    ///
    /// # Returns
    ///
    /// `true` if the composite ran. `false` if it could not, in which case
    /// the caller paints normally **and discloses** — never paints nothing,
    /// the same contract [`Self::paint_overprint`] has.
    fn paint_nonseparable(
        &mut self,
        path: &Path,
        mode: crate::blend_nonsep::NonSeparableBlend,
        rule: Option<FillRule>,
        stroking: bool,
        canvas: &mut Canvas<'_>,
    ) -> bool {
        let colour = if stroking {
            self.gs.current.stroke_color
        } else {
            self.gs.current.fill_color
        };
        let ctm = self.gs.current.ctm;

        // Coverage, rasterised exactly as a normal paint would be.
        let Some(mut coverage) = Mask::new(canvas.width(), canvas.height()) else {
            return false;
        };
        if let Some(r) = rule {
            coverage.fill_path(path, r, true, ctm);
        } else {
            let Some(stroked) = path.clone().stroke(&self.stroke_params(), 1.0) else {
                return false;
            };
            coverage.fill_path(&stroked, FillRule::Winding, true, ctm);
        }
        if let Some(old) = self.gs.current.clip.as_deref() {
            let old_data = old.data().to_vec();
            for (n, o) in coverage.data_mut().iter_mut().zip(old_data.iter()) {
                *n = u8::try_from((u16::from(*n) * u16::from(*o)) / 255).unwrap_or(255);
            }
        }

        // Restrict the scan to the path's device bounds — outside them the
        // coverage is zero and the per-pixel work is waste.
        let Some(device_path) = path.clone().transform(ctm) else {
            return false;
        };
        let b = device_path.bounds();
        let pad = if rule.is_some() {
            1.0
        } else {
            self.gs.current.line_width.mul_add(0.5, 2.0)
        };
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let region = (
            (b.left() - pad).floor().max(0.0) as u32,
            (b.top() - pad).floor().max(0.0) as u32,
            ((b.right() + pad).ceil().max(0.0) as u32).min(canvas.width()),
            ((b.bottom() + pad).ceil().max(0.0) as u32).min(canvas.height()),
        );
        if region.0 >= region.2 || region.1 >= region.3 {
            // Entirely off-page: ran correctly, touched nothing.
            return true;
        }

        let alpha = if stroking {
            self.gs.current.stroke_alpha
        } else {
            self.gs.current.fill_alpha
        };

        // Reads the destination back, so it needs real pixels — a recording
        // canvas cannot reproduce it and must refuse BY NAME rather than
        // silently record a normally-blended paint.
        canvas.refuse(PoisonReason::NonSeparableBlend);
        // The subtractive path, and note it is SHORTER than the additive
        // one rather than an extra case: `composite_element_cmyk` already
        // dispatches Table 137 through `Blend::apply_subtractive`, which
        // performs §11.3.4's complement around C/M/Y and then §11.3.5.3's
        // K SELECTION -- K takes the backdrop's value for Hue, Saturation
        // and Color and the source's for Luminosity, and is never put
        // through the formula. A CMYK buffer that ran all four channels
        // through Table 137 would produce entirely plausible output and be
        // non-conforming, which is why the selection lives in the shared
        // arithmetic and not at this call site.
        if let Some(buf) = canvas.cmyk_mut() {
            let source = self
                .authored_cmyk(stroking)
                .unwrap_or_else(|| crate::overprint::rgb_to_cmyk(colour.r, colour.g, colour.b));
            let changed = buf.composite_mask(
                &coverage,
                region,
                source,
                alpha.clamp(0.0, 1.0),
                crate::compositor::Blend::NonSeparable(mode),
            );
            self.diag.nonseparable_composited += 1;
            self.diag.nonseparable_pixels += u64::from(changed);
            return true;
        }
        let Some(dest) = canvas.pixmap_mut() else {
            return false;
        };
        let changed = crate::blend_nonsep::composite(
            dest,
            &coverage,
            mode,
            [colour.r, colour.g, colour.b],
            alpha.clamp(0.0, 1.0),
            region,
        );
        self.diag.nonseparable_composited += 1;
        self.diag.nonseparable_pixels += u64::from(changed);
        true
    }

    fn paint_with_pattern(
        &mut self,
        path: &Path,
        rule: FillRule,
        stroking: bool,
        canvas: &mut Canvas<'_>,
    ) -> bool {
        let Some(name) = self.color.pattern(stroking).map(<[u8]>::to_vec) else {
            return false;
        };
        let doc = self.doc;
        let resources = self.resources;
        let entry = resources
            .get(b"Pattern")
            .map(|o| doc.resolve(o))
            .and_then(Object::as_dict)
            .and_then(|patterns| patterns.get(&name));
        let Some(entry) = entry else {
            // §8.6.6.2: an unresolvable pattern name has no defined
            // recovery. Counted as unpainted, not as a paint.
            self.diag.color.patterns_unpainted += 1;
            self.diag.tolerated += 1;
            self.diag.color.note(&format!(
                "scn /{}: no such /Pattern resource",
                String::from_utf8_lossy(&name)
            ));
            return false;
        };
        let resolved = doc.resolve(entry);
        // Table 75/76: a tiling pattern is a STREAM, a shading pattern is a
        // plain dictionary. Both carry their entries in a dictionary, so
        // read that and branch on `/PatternType` rather than on the shape.
        let dict = match resolved {
            Object::Dict(d) => d,
            Object::Stream(st) => &st.dict,
            _ => {
                self.diag.color.patterns_unpainted += 1;
                self.diag.tolerated += 1;
                return false;
            }
        };
        let pattern_type = dict
            .get(b"PatternType")
            .map(|o| doc.resolve(o))
            .and_then(Object::as_number)
            .unwrap_or(0.0);
        #[allow(clippy::float_cmp)]
        if pattern_type != 2.0 {
            // Tiling (1), or a value the standard does not define.
            self.diag.color.patterns_unpainted += 1;
            self.diag.color.note(&format!(
                "scn /{}: PatternType {} not painted (only shading patterns are drawn this build)",
                String::from_utf8_lossy(&name),
                pattern_type as i64
            ));
            return false;
        }
        let Some(shading_entry) = dict.get(b"Shading") else {
            self.diag.color.patterns_unpainted += 1;
            self.diag.tolerated += 1;
            self.diag.color.note(&format!(
                "scn /{}: PatternType 2 with no /Shading",
                String::from_utf8_lossy(&name)
            ));
            return false;
        };

        self.diag
            .shading
            .reached(crate::shading::PaintRoute::ShadingPattern);
        let intent = self.policy.cmyk_intent;
        let Some(shading) = crate::shading::Shading::load(
            doc,
            shading_entry,
            resources,
            intent,
            &mut self.diag.color,
            &mut self.diag.shading,
        ) else {
            self.diag.color.patterns_unpainted += 1;
            return false;
        };
        if shading.is_paintable() {
            self.diag.shading.paintable += 1;
        }
        if self.oc_hidden() || crate::profile::skip_paint() {
            // Consistent with every other paint: hidden content is
            // resolved and counted, just not drawn. Not a shortfall, so
            // `patterns_unpainted` is deliberately NOT incremented.
            return false;
        }

        // pattern space -> default space (/Matrix) -> device (base_ctm).
        // `pre_concat` applies its argument FIRST, which is the order the
        // sentence above reads in.
        let matrix = pattern_matrix(doc, dict);
        let to_device = self.base_ctm.pre_concat(matrix);
        let Some(to_target) = to_device.invert() else {
            // A degenerate pattern matrix collapses pattern space to a
            // line or a point; there is no sensible colour for any pixel.
            self.diag.color.patterns_unpainted += 1;
            self.diag.tolerated += 1;
            self.diag.color.note(&format!(
                "scn /{}: non-invertible pattern matrix",
                String::from_utf8_lossy(&name)
            ));
            return false;
        };

        // The paint AREA is the path, so the path becomes a mask —
        // intersected with any clip already in force, because a pattern
        // fill is still subject to the clip like any other paint.
        let ctm = self.gs.current.ctm;
        let Some(mut mask) = Mask::new(canvas.width(), canvas.height()) else {
            return false;
        };
        mask.fill_path(path, rule, true, ctm);
        if let Some(old) = self.gs.current.clip.as_deref() {
            // Per-pixel coverage multiply, the same operation
            // `intersect_clip` performs — a clip and a path mask combine
            // by multiplication, never by a path boolean, because §8.5.4
            // NOTE 2 guarantees a clip only ever shrinks.
            //
            // Done over the whole buffer rather than over the path's
            // bounds as `intersect_clip` does: the saving there is worth
            // the extra bookkeeping because clips run tens of thousands of
            // times on a real sheet, whereas a pattern fill is rare.
            let old_data = old.data().to_vec();
            for (n, o) in mask.data_mut().iter_mut().zip(old_data.iter()) {
                *n = ((u16::from(*n) * u16::from(*o)) / 255) as u8;
            }
        }
        // The region is the path's DEVICE-space bounds: outside them the
        // mask is zero, so evaluating the shading there is wasted work on
        // a per-pixel analytic function.
        let Some(device_path) = path.clone().transform(ctm) else {
            return false;
        };
        let b = device_path.bounds();
        #[allow(clippy::cast_possible_truncation)]
        let region = (
            b.left().floor().max(0.0) as i32,
            b.top().floor().max(0.0) as i32,
            b.right().ceil().min(canvas.width() as f32) as i32,
            b.bottom().ceil().min(canvas.height() as f32) as i32,
        );
        let alpha = if stroking {
            self.gs.current.stroke_alpha
        } else {
            self.gs.current.fill_alpha
        };
        canvas.refuse(PoisonReason::Shading);
        // Bridged for the same reason the `sh` operator is: the ramp is
        // already sRGB by the time it is sampled. See that site.
        if let Some(buf) = canvas.cmyk_mut() {
            let (w, h) = (buf.width(), buf.height());
            let Some(mut scratch) = tiny_skia::Pixmap::new(w, h) else {
                self.diag.color.patterns_unpainted += 1;
                return false;
            };
            return if shading
                .paint(to_target, region, Some(&mask), alpha, &mut scratch)
                .is_some()
            {
                buf.composite_srgb(
                    &scratch,
                    clamp_region(region, w, h),
                    1.0,
                    crate::compositor::Blend::Normal,
                );
                self.diag.shading.painted += 1;
                true
            } else {
                self.diag.color.patterns_unpainted += 1;
                false
            };
        }
        let Some(dest) = canvas.pixmap_mut() else {
            self.diag.color.patterns_unpainted += 1;
            return false;
        };
        if shading
            .paint(to_target, region, Some(&mask), alpha, dest)
            .is_some()
        {
            self.diag.shading.painted += 1;
            true
        } else {
            // Modelled but not drawable — a mesh, or a type 1 function
            // shading. A real shortfall, so it counts.
            self.diag.color.patterns_unpainted += 1;
            false
        }
    }
    fn do_xobject(&mut self, op: &Operation<'_>, canvas: &mut Canvas<'_>) {
        // Copy the two shared references out before any `&mut self`
        // call so the borrow checker sees them as independent of `self`
        // (both are `&'a`, i.e. tied to the document, not to the
        // interpreter).
        let doc = self.doc;
        let resources = self.resources;

        let Some(name) = last_name(op) else {
            self.diag.tolerated += 1;
            return;
        };
        let entry = resources
            .get(b"XObject")
            .map(|o| doc.resolve(o))
            .and_then(Object::as_dict)
            .and_then(|xobjects| xobjects.get(&name));
        let Some(entry) = entry else {
            // §8.8: an unresolvable name in the XObject subdictionary is
            // spec-undefined. No-op + diagnostic.
            self.diag.tolerated += 1;
            self.diag.note(b"Do(missing XObject resource)");
            return;
        };
        // Capture the identity BEFORE resolving — this is the cycle
        // guard's key, and it only exists on the reference.
        let id = entry.as_reference();
        let Object::Stream(stream) = doc.resolve(entry) else {
            self.diag.tolerated += 1;
            self.diag.note(b"Do(XObject is not a stream)");
            return;
        };

        // §8.11.3.3: a form or image XObject may carry its OWN `/OC`, and
        // its visibility is that group's state AND the visibility where
        // the `Do` occurs. The second half is why this ORs with the
        // marked-content state rather than replacing it — an ON XObject
        // invoked inside a hidden section is still hidden.
        //
        // For a form this is threaded into the nested run (so the stream
        // is still walked and still counted); for an image there is
        // nothing to walk, so it is simply not drawn.
        //
        // NOTE the division of labour, because getting it wrong is what
        // let images paint inside hidden sections: this flag covers the
        // XObject's OWN `/OC` only. The "current visibility at the place
        // the `Do` occurs" half is enforced inside `draw_image` and
        // `do_form`, at the blit, where inline images are also covered.
        let oc_hidden_here = match stream.dict.get(b"OC").and_then(|o| o.as_reference()) {
            Some(oc) => {
                let off = self.oc_off_set().clone();
                pdfce_core::annot::oc_is_hidden(doc, oc, &off)
            }
            None => false,
        };
        if oc_hidden_here {
            self.diag.oc_sections_hidden += 1;
        }

        let subtype = stream
            .dict
            .get(b"Subtype")
            .map(|o| doc.resolve(o))
            .and_then(Object::as_name)
            .map(|n| n.as_bytes());
        match subtype {
            Some(b"Image") => {
                if !oc_hidden_here {
                    self.do_image(&stream.dict, stream.data_span, canvas);
                }
            }
            Some(b"Form") => self.do_form(id, stream, canvas, oc_hidden_here),
            // §8.8.2: ignored by a conforming non-PostScript reader.
            Some(b"PS") => {}
            _ => {
                // `Subtype` is Required in both tables, so this file is
                // malformed. Structural inference (`Width`+`Height` ⇒
                // image, `BBox` ⇒ form) is a repair heuristic, NOT spec
                // (`iso32000__s__8.8.md`), so it is counted.
                self.diag.tolerated += 1;
                self.diag.note(b"Do(XObject without /Subtype)");
                if stream.dict.contains_key(b"Width") && stream.dict.contains_key(b"Height") {
                    if !oc_hidden_here {
                        self.do_image(&stream.dict, stream.data_span, canvas);
                    }
                } else if stream.dict.contains_key(b"BBox") {
                    self.do_form(id, stream, canvas, oc_hidden_here);
                }
            }
        }
    }

    /// `Do` on a form XObject — §8.10.1's five-step procedure, verbatim:
    ///
    /// > a) save the current graphics state, as if by `q`;
    /// > b) concatenate the matrix from the form dictionary's `Matrix`
    /// >    entry with the current transformation matrix;
    /// > c) clip according to the form dictionary's `BBox` entry;
    /// > d) paint the graphics objects specified in the form's content
    /// >    stream;
    /// > e) restore the saved graphics state, as if by `Q`.
    ///
    /// **Order matters and is not negotiable.** `Matrix` is concatenated
    /// *before* the `BBox` clip, so the box is clipped in the
    /// transformed space — a form whose `Matrix` scales by 2 has a
    /// `BBox` twice as large on the page, not the same size.
    ///
    /// Steps (a) and (e) are implemented structurally rather than with
    /// `q`/`Q`: the nested interpreter runs over a *clone* of the
    /// current state and its stack is discarded, so an unbalanced `Q`
    /// inside the form cannot pop the caller's state (§8.4.2's balance
    /// requirement is per content stream, and producers break it).
    fn do_form(
        &mut self,
        id: Option<ObjId>,
        stream: &Stream,
        canvas: &mut Canvas<'_>,
        oc_off: bool,
    ) {
        // --- recursion guards (module docs, ARCHITECTURE.md §10.1) ---
        if self.depth >= MAX_XOBJECT_DEPTH {
            self.diag.xobject_depth_overflows += 1;
            self.diag.note(b"Do(form nesting past MAX_XOBJECT_DEPTH)");
            return;
        }
        if let Some(id) = id
            && self.active.contains(&id)
        {
            self.diag.xobject_depth_overflows += 1;
            self.diag.note(b"Do(form invokes itself - cycle)");
            return;
        }

        let doc = self.doc;

        // --- VIEWPORT CULL, and it has to be HERE, before the decode ---
        //
        // WHAT IT IS. §8.10.1: "the form XObject's bounding box ... shall
        // be used to clip its contents". That is a clip, not a hint, so no
        // operator inside a form can paint a pixel outside its transformed
        // `/BBox`. A form whose box misses the canvas therefore cannot
        // change one pixel of the raster, and skipping it is EXACT — the
        // output is byte-identical either way. Nothing here is a fidelity
        // trade.
        //
        // WHY THE POSITION IS THE WHOLE POINT, and it was got wrong once.
        // The first version of this cull sat below, next to the `/BBox`
        // clip in step (c), which reads as the natural home for it. It
        // reported the same counts — 339 of 342 forms culled on the test
        // page — and bought almost nothing, because by the time control
        // reaches step (c) the stream has already been sliced, FLATE-
        // DECODED and parsed into a `ContentStream`. On the gen-scale-demo
        // banana page that is ~110 kB inflated per instance, 342 times:
        // ~37 MB of inflate for content that was about to be discarded.
        // Measured: 802 ms with the late cull, 8× less with this one. A
        // cull is only worth what it skips, and the counter looks
        // identical in both cases, which is exactly why the wrong version
        // was convincing.
        //
        // WHY THE CLIP BBOX IS ONLY CONSULTED WITHOUT A SOFT MASK. Further
        // down, a transparency group with a soft mask has `inner.clip`
        // REPLACED by the wider pre-mask clip (§11.4.5/§11.6.6). Testing
        // against the narrower clip in force here could then cull a form
        // that the widened clip would have shown. Canvas bounds are always
        // safe; the clip box is only used when no such widening can
        // happen.
        //
        // THE ONE-PIXEL MARGIN is deliberate. A shape whose edge lands on
        // the boundary can still tint the boundary pixel through
        // anti-aliasing. Being a pixel too generous costs one form; being
        // a pixel too eager costs a seam at a tile edge — the signature
        // artefact of a culling bug, and among the hardest to attribute
        // months later.
        if let Some(bbox) = rect_entry(doc, &stream.dict, b"BBox")
            && bbox.width() > 0.0
            && bbox.height() > 0.0
        {
            // The `/Matrix` concat is repeated in step (b) below rather
            // than hoisted: this probe must not touch `inner`, which does
            // not exist yet, and duplicating one `post_concat` is cheaper
            // than restructuring the soft-mask handling that sits between.
            // In `f64`, for the same reason the real composition below
            // is: a probe that disagrees with the CTM the form will
            // actually be painted under would cull a form that renders,
            // or keep one that does not, and both failures look like a
            // renderer bug rather than an arithmetic one.
            let mut probe = self.gs.current.ctm64;
            if let Some(m) = matrix_entry64(doc, &stream.dict) {
                probe = m.post_concat(probe);
            }
            if let Some(dev) = bbox.transform(probe.to_f32()) {
                #[allow(clippy::cast_precision_loss)]
                let (cw, ch) = (canvas.width() as f32, canvas.height() as f32);
                let (mut l, mut t) = (dev.left() - 1.0, dev.top() - 1.0);
                let (mut r, mut b) = (dev.right() + 1.0, dev.bottom() + 1.0);
                if self.gs.current.soft_mask.is_none()
                    && let Some((cl, ct, cr, cb)) = self.gs.current.clip_bbox
                {
                    l = l.max(cl);
                    t = t.max(ct);
                    r = r.min(cr);
                    b = b.min(cb);
                }
                if r.min(cw) <= l.max(0.0) || b.min(ch) <= t.max(0.0) {
                    self.diag.forms_culled += 1;
                    return;
                }
                // The OPT-IN, lossy half. Tested after the exact cull, and
                // counted separately, so a fidelity trade can never be
                // mistaken for the correctness optimisation above.
                //
                // Both axes, so a hairline -- thin but long -- still
                // paints. `dev` is used rather than the clipped `l/t/r/b`
                // because the question is how big the FORM is, not how
                // much of it survives the clip: a large form mostly
                // clipped away is not a small form.
                if self.subpixel_culling
                    && dev.width() < SUBPIXEL_CULL_PX
                    && dev.height() < SUBPIXEL_CULL_PX
                {
                    self.diag.subpixel_culled += 1;
                    return;
                }
            }
        }

        // `doc.slice`, not `span.slice(doc.bytes())` (decision 018 §4): a
        // form XObject authored this session — every dimension and markup
        // annotation appearance is one — has its content stream in the R45
        // staging half, which is precisely why authored annotations never
        // appeared on the canvas before Pass 17.0.
        let Some(raw) = doc.slice(stream.data_span) else {
            self.diag.tolerated += 1;
            return;
        };
        let Ok(bytes) = filters::decode_stream(&stream.dict, raw) else {
            // A form whose content stream needs an unimplemented filter
            // is content pdfce cannot show — same honesty posture as an
            // undecodable image.
            self.diag.tolerated += 1;
            self.diag.note(b"Do(form content stream undecodable)");
            return;
        };
        let Ok(content) = ContentStream::parse(bytes) else {
            self.diag.tolerated += 1;
            self.diag.note(b"Do(form content stream unparseable)");
            return;
        };

        // --- (a) save: work on a clone, never on `self.gs` ---
        let mut inner = self.gs.current.clone();

        // --- (b) concatenate /Matrix (Table 95; default identity) ---
        if let Some(m) = matrix_entry64(doc, &stream.dict) {
            inner.set_ctm64(m.post_concat(inner.ctm64));
        }

        // §11.4.5 / §11.6.6 — THE SOFT MASK, DECIDED BEFORE THE CLIP
        //
        // A soft mask in force at a `Do` applies to the group's RESULT,
        // not to the objects inside it, and §11.6.6 says the second half
        // out loud: inside the group the mask "shall be" None, "to ensure
        // that they are not applied twice".
        //
        // pdfce folds a mask into the clip, which is correct for an
        // elementary object (§11.6.4.1: the mask value is that object's
        // `q_m`, and a `q_m` multiplies coverage exactly as a clip does)
        // and wrong for a group. So for a transparency group the mask is
        // LIFTED BACK OUT here — `inner.clip` is restored to what it was
        // before the fold — and handed to `Canvas::group` to apply once,
        // to the composite.
        //
        // This has to happen BEFORE step (c) so the form's own `/BBox`
        // clip lands on the mask-free clip rather than on top of the fold.
        //
        // The one case it cannot be lifted: a `W n` established between
        // the `gs` that set the mask and this `Do`. The pre-mask clip
        // predates that clip, so restoring it would UNCLIP content
        // (`GraphicsState::clip_before_smask`'s own documented limit).
        // Those groups keep the old behaviour and are counted on
        // `soft_masks_reset_stale`, which is the counter that limit
        // already had.
        let is_group_here = stream
            .dict
            .get(b"Group")
            .map(|o| doc.resolve(o))
            .and_then(Object::as_dict)
            .is_some_and(|g| {
                g.get(b"S")
                    .map(|o| doc.resolve(o))
                    .and_then(Object::as_name)
                    .is_some_and(|n| n.as_bytes() == b"Transparency")
            });
        let group_mask = if is_group_here && self.gs.current.soft_mask.is_some() {
            match (
                self.gs.current.clip_before_smask.clone(),
                self.gs.current.clips_since_smask,
            ) {
                (Some(pre), 0) => {
                    let m = self.gs.current.soft_mask.clone();
                    inner.clip = pre;
                    inner.clip_bbox = None;
                    m
                }
                _ => {
                    self.diag.soft_masks_reset_stale += 1;
                    self.diag.note(
                        b"Do(group): a clip was established after the soft mask, so the \
                          mask stays folded into the contents' clip; 11.4.5 not applied \
                          to the group result)",
                    );
                    None
                }
            }
        } else {
            None
        };
        // §11.6.6: whatever happened above, the group's own contents run
        // with NO soft mask. When one was lifted it is applied to the
        // result; when it could not be, it is already inside `inner.clip`
        // and must not be applied a second time from the state.
        inner.soft_mask = None;
        inner.clip_before_smask = None;
        inner.clips_since_smask = 0;

        // --- (c) clip to /BBox, expressed in FORM space ---
        match rect_entry(doc, &stream.dict, b"BBox") {
            Some(rect) => {
                // A zero-width or zero-height BBox is legal and means
                // "paint nothing" (§8.10 gotchas) — `PathBuilder::
                // from_rect` cannot represent it, so short-circuit
                // rather than skipping the clip and painting everything.
                if rect.width() <= 0.0 || rect.height() <= 0.0 {
                    self.diag.forms_rendered += 1;
                    return;
                }

                let path = PathBuilder::from_rect(rect);
                // The BBox is in FORM space, so it is clipped through
                // the ALREADY-Matrix-concatenated CTM (step b before
                // step c — see the fn docs).
                let form_ctm = inner.ctm;
                // NOTE the ORDER: `group_mask` above has already replaced
                // `inner.clip` with the pre-mask clip when this form is a
                // transparency group, so the BBox intersects the
                // MASK-FREE clip. Doing it the other way round would
                // leave the mask inside the group's clip and the BBox
                // outside it — the BBox is geometry and belongs with the
                // geometric clip.

                intersect_clip(
                    &mut inner,
                    &path,
                    FillRule::Winding,
                    form_ctm,
                    canvas,
                    &mut self.clip_cache,
                );
            }
            None => {
                // `BBox` is Required (Table 95). Painting unclipped is
                // the lenient reading every viewer takes; it is counted.
                self.diag.tolerated += 1;
                self.diag.note(b"Do(form without /BBox)");
            }
        }

        // --- (d) paint, with the form's OWN resources ---
        // §7.8.3 case 2. The fallback to the *calling* stream's
        // resources is case 3 — a construct §7.8.3 calls obsolete
        // (PDF ≤ 1.1) but does not forbid reading. Note the two
        // dictionaries are never MERGED: §8.10's PDF 1.2+ rule
        // explicitly forbids promoting a form's resources outward.
        let form_resources = match stream
            .dict
            .get(b"Resources")
            .map(|o| doc.resolve(o))
            .and_then(Object::as_dict)
        {
            Some(own) => own,
            None => {
                self.diag.tolerated += 1;
                self.diag
                    .note(b"Do(form without /Resources - using caller's)");
                self.resources
            }
        };

        // §11.4.7 / Table 147: a `/Group` with `/S /Transparency` makes this
        // form a COMPOSITING SCOPE, not merely a reusable content stream.
        // Its contents are rendered into their own buffer, and the GROUP'S
        // RESULT is composited with the blend mode, constant alpha and soft
        // mask in force at the `Do` — §11.4.5. Painting the contents
        // straight onto the page applies those to each object INSIDE
        // instead, which is the same answer only for a group holding one
        // opaque object.
        let group_dict = stream
            .dict
            .get(b"Group")
            .map(|o| doc.resolve(o))
            .and_then(Object::as_dict);
        let is_transparency_group = group_dict.is_some_and(|g| {
            g.get(b"S")
                .map(|o| doc.resolve(o))
                .and_then(Object::as_name)
                .is_some_and(|n| n.as_bytes() == b"Transparency")
        });
        let group_flag = |k: &[u8]| {
            group_dict.is_some_and(|g| {
                matches!(
                    g.get(k).map(|o| doc.resolve(o)),
                    Some(Object::Boolean(true))
                )
            })
        };
        // `/K` (knockout, Table 147). ★ IMPLEMENTED SINCE `Pass 97.0` —
        // `crate::canvas::KnockoutTarget` — and this comment used to open
        // "is NOT implemented", with three sentences explaining that
        // compositing a knockout group as an ordinary one "gets its outer
        // boundary right and its internal occlusion order wrong". Deleted
        // rather than kept, because it described the code that is no
        // longer here.
        //
        // What follows is the part that SURVIVES the implementation, and
        // both paragraphs are still load-bearing: the first because pdfce
        // still honours only the EXPLICIT `/K` slice, and the second
        // because it is the trap any test of this feature falls into.
        //
        // ★ IT IS NOT A CORNER CASE, and the explicit `/K` groups pdfce
        // now renders correctly are the SMALLEST of the four populations.
        // FOUR clauses establish a knockout group
        // with no `/K` key anywhere in the file: §9.3.8's `/TK`, whose
        // INITIAL VALUE IS `true`, making every text object one; §11.6.7,
        // which makes shading patterns knockout (tiling patterns are not);
        // and §11.7.4.4, which makes `B`/`B*`/`b`/`b*` and text render
        // modes 2 and 6 knockout. §11.7.4.4's NOTE 2 names the visible
        // symptom outright: the DOUBLE BORDER on a semi-transparent
        // fill-then-stroke is missing knockout. Ranked by likely frequency
        // that is `B`/`b` >> `/TK` > explicit `/K` — so the one pdfce
        // implements is the rarest, and `transparency_groups_knockout_
        // approximated` reading zero says nothing about the other three.
        //
        // ★ AND THE FIXTURE WARNING, because it is the degenerate-fixture
        // trap again: knockout and non-knockout are IDENTICAL when every
        // element is opaque (`q_s = 1` implies `α_s = f_s`). A fixture of
        // opaque fills cannot tell a correct implementation from a wrong
        // one. Any knockout test must set `/ca < 1` — which is what
        // `tests/transparency_is_disclosed.rs`'s
        // `knockout_erases_an_earlier_element_where_a_normal_group_layers`
        // does, and why it does it.
        let is_knockout = group_flag(b"K");
        if is_transparency_group && (is_knockout || group_flag(b"I")) {
            self.diag.transparency_groups_special += 1;
        }

        // §11.3.4 / Table 147 — THE GROUP'S BLENDING COLOUR SPACE.
        //
        // Table 147's `/CS` row is the whole rule, and its second half is
        // the one an implementation skips: *"if the group is NON-ISOLATED,
        // `CS` shall be IGNORED and the colour space shall be inherited
        // from the group's parent"*. ISO 32000-2 §11.6.6 states it from
        // the other side — *"non-isolated groups shall inherit their
        // colour space from the nearest ancestor isolated parent group"* —
        // and gives the reason: converting the backdrop into a different
        // space is not always possible (some conversions run one way
        // only), and would be an excessive number of conversions where it
        // is.
        //
        // ★ Which is why every Ghent transparency patch blends in
        // `DeviceCMYK` even though one of them draws in `ICCBased` RGB:
        // the declaration is on the PAGE group, and the cell groups either
        // inherit it or restate it.
        //
        // A group that is not a transparency group is not a compositing
        // scope at all and changes nothing.
        let group_space = if is_transparency_group {
            let declared = group_flag(b"I")
                .then(|| {
                    group_dict
                        .and_then(|g| g.get(b"CS"))
                        .and_then(|cs| {
                            crate::color::resolve_object(
                                doc,
                                doc.resolve(cs),
                                form_resources,
                                0,
                                &mut self.diag.color,
                            )
                        })
                        .map(|sp| crate::compositor::BlendSpace::of(&sp))
                })
                .flatten();
            let space = declared.unwrap_or(self.blend_space);
            if space.is_subtractive() {
                self.diag.blend_space_subtractive += 1;
            }
            space
        } else {
            self.blend_space
        };

        let mut active = self.active.clone();
        if let Some(id) = id {
            active.push(id);
        }
        // A transparency group gets its OWN page-sized buffer so its result
        // can be composited as a unit. Anything else paints straight into
        // the caller's pixmap, exactly as before.
        //
        // The buffer is page-sized rather than BBox-sized on purpose: the
        // group's contents are drawn under the SAME CTM as the page, so a
        // smaller buffer would need its own translation threaded through
        // every paint site and the clip mask. Page-sized costs ~4 bytes per
        // pixel per nesting level and needs no coordinate change at all,
        // which is the difference between a correct implementation and a
        // subtly-misaligned one.
        //
        // WHEN a buffer is needed, and why "always" is both slower AND less
        // correct. A buffer starts TRANSPARENT, which is ISOLATED semantics
        // (§11.4.7). `/I` defaults to FALSE, so most groups are
        // NON-isolated and their contents are meant to blend against the
        // page backdrop — precisely what painting inline does. Buffering
        // unconditionally gets those wrong in the opposite direction from
        // flattening, and costs a page-sized allocation to do it.
        // ★ `nonseparable` MUST be part of this test. It was not when the
        // field was added, and the consequence was silent: a non-separable
        // outer mode parks `blend_mode` at `SourceOver` (see the field docs),
        // so a group under `/BM /Hue` LOOKED neutral, skipped its buffer, and
        // painted inline — the one path where the outer mode can never be
        // applied to the group's result at all.
        //
        // The general shape is worth naming, because it is how the same bug
        // arrives twice: a new graphics-state field has to be added to every
        // predicate that asks "is the state still default?", and nothing
        // makes those sites findable from the field.
        // ★ AND THE SOFT MASK, which is the SAME BUG A THIRD TIME and the
        // comment above predicted it: "a new graphics-state field has to be
        // added to every predicate that asks 'is the state still default?',
        // and nothing makes those sites findable from the field."
        //
        // A group under a soft mask with a `Normal` blend and alpha 1 looked
        // neutral, took the inline path, and had its mask applied to every
        // object inside it — §11.4.5 says it applies to the group's result.
        // The inline path has no result to apply it to.
        let outer_is_neutral = self.gs.current.blend_mode == tiny_skia::BlendMode::SourceOver
            && self.gs.current.nonseparable.is_none()
            && self.gs.current.soft_mask.is_none()
            && self.gs.current.fill_alpha >= 1.0;
        // KNOCKOUT is unconditional, and the neutral-outer-state shortcut is
        // not available to it. §11.4.4 NOTE 5's inline fast path requires
        // the group to have "the same knockout attribute as its parent" —
        // a knockout group has no initial backdrop to composite against
        // when painted inline, which is the whole of what makes it a
        // knockout group. Painting it inline is not a cheaper route to the
        // same picture; it is the non-knockout picture.
        let needs_buffer =
            is_transparency_group && (!outer_is_neutral || group_flag(b"I") || is_knockout);
        // A group that needs its own buffer is drawn through
        // `Canvas::layer` — which allocates the buffer, runs the contents
        // into it and performs §11.4.5's composite. Buffer and composite
        // used to be written out here; they moved because an annotation's
        // `/CA` compositing is the SAME operation, and two copies of it are
        // two places for the "no mask on the composite" rule to be
        // forgotten.
        //
        // Everything the nested run needs is read out of `self` FIRST, so
        // the closure below captures plain values rather than a borrow of
        // an interpreter whose `diag` the caller is about to write to.
        let fonts = self.fonts;
        let depth = self.depth + 1;
        let cancel = self.cancel;
        let policy = self.policy;
        let hidden_here = self.oc_hidden() || oc_off;
        // §11.4.5: the blend mode, constant alpha and soft mask in force at
        // the `Do` apply to the GROUP'S RESULT, not to the objects inside
        // it. So the group's own contents start with the initial values —
        // otherwise the blend is applied twice, once per object and again
        // to the composite. `inner` is CLONED rather than mutated in place
        // because the could-not-start fallback below must run with the
        // ORIGINAL state, exactly as the previous code did.
        let layered = if needs_buffer {
            let mut group_state = inner.clone();
            group_state.blend_mode = tiny_skia::BlendMode::SourceOver;
            group_state.fill_alpha = 1.0;
            group_state.stroke_alpha = 1.0;
            // ★ AND THE NON-SEPARABLE MODE, for the same §11.4.5 reason as
            // the three above — a bug the moment `nonseparable` was added,
            // because the reset list is the kind of thing a new field is
            // silently absent from. Contents that inherited it would blend
            // against the group's own TRANSPARENT buffer, which is not what
            // the outer mode means: the outer mode applies to the group's
            // RESULT.
            group_state.nonseparable = None;
            let paint = LayerPaint {
                opacity: self.gs.current.fill_alpha.clamp(0.0, 1.0),
                blend: self.gs.current.blend_mode,
                // §11.4.5 through Table 137 when the outer mode is one of
                // the four -- `Canvas::layer` composites the group RESULT
                // per pixel, because `draw_pixmap` cannot carry these.
                nonseparable: self.gs.current.nonseparable,
            };
            let nested_active = active.clone();
            // §11.4.4 / §11.4.5, through `Canvas::group` rather than
            // `Canvas::layer`: an ISOLATED group's initial backdrop is
            // transparent and `layer`'s buffer already is that, but a
            // NON-isolated group's elements are entitled to see the
            // backdrop beneath them and `layer` cannot give them one.
            //
            // The closure is called ONCE for an isolated group and for a
            // non-isolated group whose interior never blends, and TWICE
            // otherwise — see `Canvas::group` for why the second run is
            // both necessary and conditional. Only the first run's
            // diagnostics are returned, so nothing here double-counts.
            canvas.group(
                paint,
                group_flag(b"I"),
                is_knockout,
                group_mask.as_deref(),
                |sub| {
                    let nested = run_nested(
                        doc,
                        &content,
                        form_resources,
                        fonts,
                        group_state.clone(),
                        sub,
                        depth,
                        nested_active.clone(),
                        cancel,
                        policy,
                        hidden_here,
                        group_space,
                    );
                    // "Did anything inside this group need to read what was
                    // underneath it?" — §11.4.4 NOTE 2's condition, answered
                    // from the run rather than guessed from the dictionary.
                    //
                    // Deliberately CONSERVATIVE in the safe direction: a
                    // nested *isolated* child's own interior blends also land
                    // in these counters, so the outer group can be re-run when
                    // it did not strictly need to be. Over-triggering costs a
                    // second walk; under-triggering silently renders a blend
                    // against nothing, which is the failure this whole path
                    // exists to end.
                    let backdrop_dependent = nested.blend_modes_applied > 0
                        || nested.nonseparable_composited > 0
                        || nested.overprint_composited > 0
                        || nested.transparency_groups_backdrop_reruns > 0;
                    (nested, backdrop_dependent)
                },
            )
        } else {
            None
        };
        match layered {
            Some(outcome) => {
                if outcome.backdrop_rerun {
                    self.diag.transparency_groups_backdrop_reruns += 1;
                }
                // §11.4.6 is IMPLEMENTED now, so this counter changed
                // meaning: it used to count every `/K true` group (the
                // whole feature was an approximation) and now counts only
                // the ELEMENTS inside one that could not be given knockout
                // semantics, because they read the destination back. A
                // knockout group that renders exactly reports zero.
                self.diag.transparency_groups_knockout_approximated +=
                    outcome.knockout_approximated;
                if group_mask.is_some() {
                    self.diag.soft_masks_on_group_result += 1;
                }
                self.diag.merge(outcome.result);
                self.diag.transparency_groups_composited += 1;
            }
            None => {
                // Either no buffer was wanted, or one was wanted and the
                // layer could not be started. In both cases the contents
                // paint inline, under the UNMODIFIED outer state.
                let nested = run_nested(
                    doc,
                    &content,
                    form_resources,
                    fonts,
                    inner,
                    canvas,
                    depth,
                    active,
                    cancel,
                    policy,
                    hidden_here,
                    group_space,
                );
                self.diag.merge(nested);
                if needs_buffer {
                    // The layer could not be started. Painting inline rather
                    // than dropping the content, and counting it as the
                    // shortfall it is.
                    self.diag.transparency_groups_flattened += 1;
                } else if is_transparency_group {
                    // A non-isolated group under a neutral outer state.
                    // Painting inline IS the §11.4.5 answer here, not an
                    // approximation of it, so this counts as composited
                    // rather than flattened.
                    self.diag.transparency_groups_composited += 1;
                }
            }
        }
        self.diag.forms_rendered += 1;

        // --- (e) restore: `self.gs` was never touched ---
    }

    /// `Do` on an image XObject: pull the still-encoded sample bytes out
    /// of the file and hand them to the shared image path.
    fn do_image(&mut self, dict: &Dict, data: ByteSpan, canvas: &mut Canvas<'_>) {
        // See `run_form`: resolved through the view, so an image XObject
        // staged this session resolves too (decision 018 §4).
        let doc = self.doc;
        let Some(raw) = doc.slice(data) else {
            self.diag.tolerated += 1;
            return;
        };
        self.draw_image(dict, raw, canvas, ImageOrigin::XObject);
    }

    /// Decode and paint one sampled image — the single path shared by
    /// image XObjects and inline images (§8.9.7: "the key-value pairs
    /// appearing between `BI` and `ID` are analogous to those in the
    /// dictionary portion of an image XObject").
    ///
    /// `fill_color` is passed through for the stencil-mask case
    /// (§8.9.6.2: an image mask "designates places where the current
    /// colour shall be painted"), which is why the decode cannot be
    /// cached across graphics states without keying on the colour.
    fn draw_image(
        &mut self,
        dict: &Dict,
        raw: &[u8],
        canvas: &mut Canvas<'_>,
        origin: ImageOrigin,
    ) {
        // ★ THE IMAGE GATE, WHICH SHIPPED MISSING.
        //
        // §8.11.3.1's "shall not be drawn" is not media-typed, but the
        // first cut of optional content gated only the PATH blit and the
        // GLYPH blit. An image inside a hidden `/OC` section still
        // painted — and so did an inline `BI`/`ID`/`EI` image, which has
        // no XObject dictionary and therefore no `/OC` of its own to
        // check. Found by rendering a fixture and reading the pixel,
        // after `do_xobject`'s comment claimed an OR that only ever
        // reached the FORM branch.
        //
        // Here rather than at the call sites because this is the one
        // place every image path converges: the XObject branch, the
        // no-`/Subtype` repair heuristic, and the inline arm.
        //
        // Returning BEFORE the decode, not after: an image that is not
        // drawn does not need to be decoded, and on a drawing with
        // hidden raster layers that is the difference between skipping
        // the work and doing all of it invisibly. The cost is that a
        // hidden image's codec diagnostics are absent — which is honest
        // (`images_rendered` means rendered), and `oc_sections_hidden`
        // is what discloses that something was withheld.
        if self.oc_hidden() {
            return;
        }
        let doc = self.doc;
        let resources = self.resources;
        let fill = self.gs.current.fill_color;
        match image::decode(doc, dict, raw, resources, fill, origin, self.policy) {
            Ok(decoded) => {
                self.diag.note_image_divergence(decoded.notes);
                // §8.9.5.3: `/Interpolate` asks for smoothing on
                // scaling. Default false → nearest-neighbour, which is
                // what the spec's "no interpolation" means and what
                // keeps a 2×2 test image's pixels exactly assertable.
                let interpolate = matches!(
                    dict.get(b"Interpolate").map(|o| doc.resolve(o)),
                    Some(Object::Boolean(true))
                );
                // §11.7.4.3 has no image path in this renderer — see
                // `Diagnostics::overprint_images_unsupported`. Counted
                // BEFORE the paint so a paint that panics or is clipped
                // away still leaves the shortfall on the record.
                if self.gs.current.overprint_fill {
                    self.diag.overprint_images_unsupported += 1;
                    self.diag.note(
                        b"image painted under /OP true: CompatibleOverprint has no \
                          per-sample path; painted normally",
                    );
                }
                self.paint_image(&decoded.pixmap, interpolate, canvas);
                self.diag.images_rendered += 1;
            }
            Err(err) => {
                // Nothing drawn. Counted, named, never approximated.
                // The three codec-specific buckets are counted
                // SEPARATELY as well as in the headline number, because
                // "pdfce has no JPEG 2000 decoder", "pdfce has a JPEG
                // decoder but not the arithmetic-coded variant" and
                // "these bytes are broken" lead an operator to three
                // different next actions (decision 005 §6.4, R27).
                self.diag.images_unsupported += 1;
                match &err {
                    ImageError::CodecUnsupported(_) => self.diag.images_codec_unsupported += 1,
                    ImageError::CodecFeature(feature) => {
                        *self
                            .diag
                            .codec_feature_unsupported
                            .entry(feature)
                            .or_insert(0) += 1;
                    }
                    _ => {}
                }
                self.diag.note_image(&err.to_string());
            }
        }
    }

    /// Composite decoded texels onto the page through the CTM.
    ///
    /// ## The mapping (§8.9.4), and why it is a pattern shader
    ///
    /// "The unit square of user space, bounded by user coordinates
    /// (0, 0) and (1, 1), corresponds to the boundary of the image in
    /// image space… The implicit transformation from image space to
    /// user space, if specified explicitly, would be described by the
    /// matrix `[1/w 0 0 −1/h 0 1]`." The `−1/h` is the y-flip: image
    /// space is y-down with the origin at the upper-left, user space is
    /// y-up. Omitting it renders every image upside down.
    ///
    /// So placement is *entirely* the CTM's job, and the CTM may be an
    /// arbitrary affine transform — rotated, skewed, mirrored, and with
    /// a non-uniform scale that deliberately distorts the aspect ratio
    /// (§8.9.4's own EXAMPLE does exactly that). That rules out
    /// `Pixmap::draw_pixmap`, whose integer `x`/`y` origin makes it a
    /// blit-with-a-transform rather than a general mapping.
    ///
    /// The route taken instead — and the reason it is correct under any
    /// CTM — is a **[`Pattern`] shader over the unit-square path**:
    ///
    /// - the pattern's own transform is `[1/w 0 0 −1/h 0 1]`, i.e.
    ///   image space → user space;
    /// - `fill_path`'s `transform` argument is the CTM, and tiny-skia
    ///   *post-concatenates* it into the shader's transform, giving
    ///   image → user → device in one matrix;
    /// - the geometry filled is the user-space unit square, transformed
    ///   by that same CTM, so the painted region and the sampled region
    ///   coincide exactly by construction.
    ///
    /// [`SpreadMode::Pad`] keeps edge sampling inside the texels when
    /// anti-aliased coverage lands a fraction outside the square.
    ///
    /// ## Sampling, and the one direction the spec never legislated
    ///
    /// *Up*-scaling follows §8.9.5.3 literally: `/Interpolate` false — the
    /// default — means nearest-neighbour, which is right for magnification
    /// and is what makes a 2×2 test image's pixels exactly assertable.
    ///
    /// *Down*-scaling is a different question, and it took until the R169
    /// ambiguity sweep to see why. §8.9.5.3 defines interpolation **only**
    /// for magnification — *"when the resolution of a source image is
    /// significantly **lower** than that of the output device"* — and the
    /// standard never mentions minification at all (`minif` 0 hits,
    /// `mipmap` 0, `decimat` 0, `down-sampl` 0 over the whole source). So
    /// `/Interpolate false` does not in fact ask for point-sampling on the
    /// way down; it switches off the *up*-scaling feature the clause
    /// actually defines, and a reader minifying an image is unconstrained.
    ///
    /// That makes it spec ambiguity `IM-A1` and therefore the operator's
    /// choice ([`MinifyFilter`]). The default remains **point-sample**, the
    /// shipped behaviour, at **evidence tier (d)** — a guess. This comment
    /// used to assert that *"most production viewers smooth on
    /// minification regardless of `/Interpolate`"*; that assertion was
    /// never verified against anything, so it is not being used to move a
    /// default. It is restated here as the open question it is: a
    /// viewer-behaviour check filed to `C:\personal_rag\pdf\` would raise
    /// this to tier (c), and if it confirms, the default flips.
    ///
    /// ## Detecting minification
    ///
    /// The CTM maps the unit square to the painted region, so
    /// `|CTM.sx|` and `|CTM.sy|` are the device-space extents of the whole
    /// image in each axis. Dividing by the texel counts gives device
    /// pixels per texel; **below one in either axis** the image is being
    /// squeezed and texels are being skipped. A skew or rotation makes
    /// those extents approximate rather than exact, which is acceptable
    /// here: the consequence of being slightly wrong at the boundary is
    /// one filter rather than another on an image that is very nearly
    /// 1:1, where the two agree anyway.
    fn paint_image(&self, texels: &Pixmap, interpolate: bool, canvas: &mut Canvas<'_>) {
        let (w, h) = (texels.width(), texels.height());
        if w == 0 || h == 0 {
            return;
        }
        let image_to_user =
            Transform::from_row(1.0 / w as f32, 0.0, 0.0, -1.0 / h as f32, 0.0, 1.0);
        // `IM-A1`: smoothing on the way DOWN is a separate question from
        // `/Interpolate`, and only the operator's setting can turn it on.
        // `/Interpolate true` still wins outright — a document that asked
        // for smoothing gets it in both directions, whatever this is set
        // to, because that request IS spec-governed.
        let smooth_minified =
            matches!(self.policy.image_minify, MinifyFilter::Smooth) && self.is_minified(w, h);
        let quality = if interpolate || smooth_minified {
            FilterQuality::Bilinear
        } else {
            FilterQuality::Nearest
        };
        // §11.3.5 applies to an IMAGE exactly as it does to a path
        // fill — Table 58's `/BM` is a graphics-state parameter, not a
        // path-painting one. This was hard-coded `SourceOver` when
        // blend modes first landed, and the symptom was precise and
        // misleading: the operator's Ghent page 2 reported 76 blend
        // modes APPLIED while only 0.37% of its pixels changed, because
        // the marks those modes govern are drawn by images, not paths.
        // A counter said the feature worked; the pixels said otherwise.
        let blend = self.gs.current.blend_mode;
        // ★ NOT unconditionally true — see `image_edge_needs_antialiasing`.
        // An image's edge is a SAMPLING boundary, not a shape edge, and
        // antialiasing it is what bands abutting tiles.
        let anti_alias = image_edge_needs_antialiasing(self.gs.current.ctm);
        let Some(unit) = Rect::from_ltrb(0.0, 0.0, 1.0, 1.0) else {
            return;
        };
        let path = PathBuilder::from_rect(unit);
        canvas.fill_image(
            &path,
            texels,
            quality,
            image_to_user,
            blend,
            anti_alias,
            self.gs.current.ctm,
            self.gs.current.clip_ref(),
        );
    }

    /// Whether a `w × h` image is being drawn smaller than its own pixel
    /// grid under the current CTM — the trigger for `IM-A1`.
    ///
    /// The image occupies §8.9.4's unit square, so the CTM's x- and y-
    /// scale factors ARE the device-space width and height of the whole
    /// image. Dividing each by the texel count in that axis gives device
    /// pixels per texel; strictly below one means texels are being
    /// discarded, which is the only case where a minification filter can
    /// change anything.
    ///
    /// Returns `false` for a degenerate or non-finite CTM: an image with
    /// no extent paints nothing, and choosing a filter for it is not a
    /// decision worth making from a NaN.
    fn is_minified(&self, w: u32, h: u32) -> bool {
        let ctm = self.gs.current.ctm;
        let (sx, sy) = (ctm.sx.abs(), ctm.sy.abs());
        if !sx.is_finite() || !sy.is_finite() || sx <= 0.0 || sy <= 0.0 {
            return false;
        }
        sx < w as f32 || sy < h as f32
    }

    /// The graphics state's stroke geometry (§8.4.3), shared by path
    /// painting and by stroked text — which is exactly the sharing
    /// §9.3.6 mandates: "the graphics state parameters affecting those
    /// operations, such as line width, shall be interpreted in USER
    /// SPACE rather than in text space", i.e. a 12 pt and a 72 pt glyph
    /// stroked at the same `w` have the same stroke thickness.
    fn stroke_params(&self) -> Stroke {
        // A ONE-DEVICE-PIXEL FLOOR, computed through the CTM.
        //
        // `tiny_skia::stroke_path` is handed the CTM and applies it, so
        // `Stroke::width` is in USER space. The old code mapped `0 w` to a
        // fixed `0.1` user units, which is not what §8.4.3.2 asks for and
        // is wrong in both directions: at low zoom 0.1 user units is a
        // fraction of a pixel and anti-aliases to near-invisible, and at
        // high zoom it becomes a visibly thick line. "The thinnest line
        // the device can render" is a statement about DEVICE space, so it
        // has to be converted into user space through the current scale.
        //
        // The same floor also rescues thin-but-nonzero widths. Measured by
        // the benign-bucket audit: `0.1 w` lands at 0.17 device pixels and
        // pdfce anti-aliased it to grey 233 — about 9% contrast — across
        // nine qpdf `form-*.pdf` files with an identical 482-pixel
        // signature, silently. pdfium and Acrobat both draw a solid ~1 px
        // line.
        //
        // §8.4.3.2 mandates the minimum only for `0 w`, so clamping a
        // NON-ZERO sub-pixel width is a product choice rather than a
        // requirement — the standard is silent, and rendering it literally
        // is a defensible reading. It is not offered as a setting yet;
        // both reference renderers clamp, so clamping is the right
        // default, and the knob is owed rather than skipped.
        let scale = {
            let t = self.gs.current.ctm;
            // Geometric mean of the transform's singular values — the
            // scale-invariant "how much does this CTM magnify area", which
            // behaves correctly under rotation and shear where taking
            // `sx` alone would not.
            let det = t.sx.mul_add(t.sy, -(t.kx * t.ky)).abs();
            if det.is_finite() && det > 0.0 {
                det.sqrt()
            } else {
                1.0
            }
        };
        let min_user_width = if scale > 0.0 { 1.0 / scale } else { 0.0 };
        Stroke {
            // §8.4.3.2: width 0 means "thinnest line the device can
            // render", which is exactly the floor; a non-zero width takes
            // the floor only when it would otherwise land under a pixel.
            width: if self.gs.current.line_width <= 0.0 {
                min_user_width
            } else {
                self.gs.current.line_width.max(min_user_width)
            },
            miter_limit: self.gs.current.miter_limit,
            line_cap: match self.gs.current.line_cap {
                LineCap::Butt => SkCap::Butt,
                LineCap::Round => SkCap::Round,
                LineCap::Square => SkCap::Square,
            },
            line_join: match self.gs.current.line_join {
                LineJoin::Miter => SkJoin::Miter,
                LineJoin::Round => SkJoin::Round,
                LineJoin::Bevel => SkJoin::Bevel,
            },
            dash: {
                let (dashes, phase) = &self.gs.current.dash;
                if dashes.is_empty() {
                    None
                } else {
                    // tiny-skia requires an even count; PDF allows odd
                    // (repeats to even) — normalize.
                    let mut d = dashes.clone();
                    if d.len() % 2 == 1 {
                        d.extend_from_slice(dashes);
                    }
                    StrokeDash::new(d, *phase)
                }
            },
        }
    }

    /// Terminate the current path object with the requested painting
    /// (§8.5.3), then apply any pending clip (§8.5.4's deferred rule).
    fn paint(
        &mut self,
        canvas: &mut Canvas<'_>,
        fill: bool,
        stroke: bool,
        fill_rule: Option<FillRule>,
    ) {
        let builder = std::mem::replace(&mut self.path, PathBuilder::new());
        // A device-space path has already been through the CTM, so the
        // transform handed to `tiny_skia` is the IDENTITY. Taken rather
        // than peeked so the flag cannot leak into the next path, which
        // would paint it unscaled at the canvas origin.
        let precise = std::mem::take(&mut self.path_precise);
        let origin = self.path_origin.take();
        let captured = self.path_ctm.take();
        let ctm = match (precise, origin) {
            // The path was built RELATIVE to its own first point, so the
            // transform keeps the CTM's linear part unchanged and carries
            // a translation of where that point lands -- computed in
            // `f64`, so the cancellation happens before the narrowing.
            (true, Some((ox, oy))) => {
                let m = self.path_ctm64;
                let (dx, dy) = m.map(ox, oy);
                #[allow(clippy::cast_possible_truncation)]
                Transform::from_row(
                    m.sx as f32,
                    m.ky as f32,
                    m.kx as f32,
                    m.sy as f32,
                    dx as f32,
                    dy as f32,
                )
            }
            _ => captured.unwrap_or(self.gs.current.ctm),
        };
        self.current = None;
        self.subpath_start = None;
        self.needs_move = false;
        let pending_clip = self.pending_clip.take();

        let Some(path) = builder.finish() else {
            // Empty/degenerate path: nothing to paint, and a pending
            // clip over an empty path clips everything out — model
            // that with an empty mask.
            if pending_clip.is_some()
                && let Some(mask) = Mask::new(canvas.width(), canvas.height())
            {
                // Recorded as a clip with NO path, which is what an empty
                // clip IS: it admits nothing, and it does not multiply by
                // whatever was in force before it. See `ClipDef::path`.
                self.gs.current.clip_id = canvas.record_clip(ClipDef {
                    path: None,
                    rule: FillRule::Winding,
                    ctm,
                    parent: self.gs.current.clip_id,
                });
                self.gs.current.clip = Some(std::sync::Arc::new(mask));
                // An all-zero mask admits nothing, so the bbox is EMPTY —
                // not `None`, which means "no clip at all" and is the
                // opposite. Left > right, so every paint tests as outside,
                // which is exactly what an everything-clipped-out state is.
                self.gs.current.clip_bbox = Some((f32::MAX, f32::MAX, f32::MIN, f32::MIN));
                crate::profile::note_clip(0.0, 0.0);
            }
            return;
        };

        // Pass 9a cross-check: record this finished path's nodes + captured
        // CTM for the object-model geometry oracle (module docs of
        // `trace_paths`), before painting, using the SAME `path`/`ctm` the
        // renderer is about to draw. `None` in ordinary rendering.
        if let Some(trace) = self.trace.as_mut() {
            let mut nodes = Vec::new();
            for seg in path.segments() {
                nodes.push(match seg {
                    tiny_skia::PathSegment::MoveTo(p) => TracedNode::Move(p.x, p.y),
                    tiny_skia::PathSegment::LineTo(p) => TracedNode::Line(p.x, p.y),
                    tiny_skia::PathSegment::CubicTo(a, b, c) => {
                        TracedNode::Cubic(a.x, a.y, b.x, b.y, c.x, c.y)
                    }
                    // A PDF content stream never emits a quadratic; if a
                    // future path source ever did, lower it to its
                    // endpoint so the anchor cross-check still holds.
                    tiny_skia::PathSegment::QuadTo(_, p) => TracedNode::Line(p.x, p.y),
                    tiny_skia::PathSegment::Close => TracedNode::Close,
                });
            }
            trace.push(TracedPath {
                nodes,
                ctm,
                fill,
                stroke,
            });
        }

        // Paint under the CURRENT clip (the deferred-W rule: the new
        // clip must NOT affect this paint).
        //
        // # BORROWED, never cloned
        //
        // `clip` is an `Option<tiny_skia::Mask>` holding a **page-sized**
        // coverage buffer — one byte per device pixel. This used to be a
        // `.clone()`, which meant every fill and every stroke memcpy'd the
        // whole page before painting anything.
        //
        // Measured on a 129,515-path CAD drawing (2026-08-07): ~114,000
        // paints × a 1 MB mask at scale 1 is ~108 GB of pointless memory
        // traffic for a single page, and it scales with page area, so the
        // cost of drawing one hairline grew with the size of the paper it
        // was drawn on. Nothing needed the copy — `fill_path`/`stroke_path`
        // take `Option<&Mask>`, and the clip is not mutated until
        // `intersect_clip` below, which is after the last use.
        let clip = self.gs.current.clip_ref();
        crate::profile::note_paint(
            clip.mask.is_some(),
            paint_is_cullable(&path, ctm, self.gs.current.clip_bbox),
        );
        // MEASUREMENT ABLATIONS — both fold away without `profile`.
        // `clip-sample` keeps the mask built and drops only the
        // per-pixel sampling, which is what isolates sampling cost from
        // construction cost; skipping construction cannot.
        // The ablation drops the MASK and only the mask. A recorded clip
        // id costs nothing per pixel, so removing it would not isolate
        // sampling cost — it would change what a recording MEANS.
        let clip = if crate::profile::skip_clip_sample() {
            ClipRef { mask: None, ..clip }
        } else {
            clip
        };
        // Hidden content is not drawn, and everything else in this
        // function still runs: the path is consumed, the CTM is taken,
        // and the pending clip below is applied exactly as if it had
        // been painted (§8.11.3.1).
        let skip_paint = crate::profile::skip_paint() || self.oc_hidden();
        // ★ A NON-SEPARABLE BLEND MODE REPLACES the ordinary paint, exactly
        // as overprint does — it is a different compositing rule, not a
        // post-pass over a normal one. Painting normally first would knock
        // out the backdrop the blend function needs to read.
        let nonsep = if skip_paint {
            None
        } else {
            self.gs.current.nonseparable
        };
        // `self.color.paints(_)` is false only where the standard or
        // pdfce's own limits say nothing is drawn: a `Pattern` space
        // (unpainted, and counted), `Separation /None` and an all-`/None`
        // `DeviceN` ("shall have no effect on the current page", §8.6.6.4).
        // Painting white instead would erase the backdrop those cases
        // require to show through.
        // Would honouring overprint have changed either half of this
        // paint? Asked here, once per painted object, because that is the
        // granularity §11.7.4.3 works at — "elementary graphics objects
        // (fills, strokes, text, images, and shadings)".
        let mut overprint_fill = false;
        let mut overprint_stroke = false;
        if !skip_paint {
            if fill && fill_rule.is_some() && self.overprint_would_change(false) {
                self.diag.overprint_effective += 1;
                overprint_fill = true;
            }
            if stroke && self.overprint_would_change(true) {
                self.diag.overprint_effective += 1;
                overprint_stroke = true;
            }
        }
        // Deferred for the same borrow reason as `pattern_fill` below: the
        // composite needs `&mut self` while `clip` is an immutable borrow
        // of the same graphics state.
        let mut overprint_fill_pending = false;
        let mut overprint_stroke_pending = false;
        // Deferred to after the `clip` borrow ends — see below.
        let mut pattern_fill: Option<FillRule> = None;
        if !skip_paint
            && fill
            && let Some(rule) = fill_rule
        {
            if self.color.paints(false) {
                // Overprint replaces the ordinary paint entirely — it is a
                // different blend mode (§11.7.4.3), not a post-pass over a
                // normal one. Painting first and overprinting after would
                // have already knocked out the backdrop this must preserve.
                if overprint_fill {
                    overprint_fill_pending = true;
                } else if nonsep.is_some() {
                    // Deferred below with the other composites.
                } else {
                    let paint = self.solid_authored(
                        false,
                        self.gs.current.fill_alpha,
                        self.gs.current.blend_mode,
                    );
                    canvas.fill(&path, &paint, rule, ctm, clip);
                }
            } else {
                // `paints` is false for a Pattern space as well as for
                // `Separation /None` — and those want opposite things. The
                // colorant cases must paint NOTHING (§8.6.6.4); a pattern
                // wants its own painter. `paint_with_pattern` returns
                // immediately unless a pattern name is actually selected,
                // so the colorant cases still fall through to nothing.
                pattern_fill = Some(rule);
            }
        }
        if !skip_paint && stroke && self.color.paints(true) {
            if overprint_stroke {
                overprint_stroke_pending = true;
            } else if nonsep.is_some() {
                // Deferred below with the other composites.
            } else {
                let paint = self.solid_authored(
                    true,
                    self.gs.current.stroke_alpha,
                    self.gs.current.blend_mode,
                );
                canvas.stroke(&path, &paint, &self.stroke_params(), ctm, clip);
            }
        }

        // The non-separable composites, after the `clip` borrow ends — same
        // reason the overprint ones are here. Fill before stroke, matching
        // the order the ordinary paints above would have used.
        if let Some(mode) = nonsep {
            if fill
                && let Some(rule) = fill_rule
                && self.color.paints(false)
                && !self.paint_nonseparable(&path, mode, Some(rule), false, canvas)
            {
                // Could not composite — paint normally rather than paint
                // nothing, and SAY SO. A silent fallback would leave the
                // operator with a Normal-blended mark and a diagnostic
                // claiming the mode was applied.
                self.diag.blend_modes_ignored += 1;
                let paint = self.solid_authored(
                    false,
                    self.gs.current.fill_alpha,
                    tiny_skia::BlendMode::SourceOver,
                );
                let clip = self.gs.current.clip_ref();
                canvas.fill(&path, &paint, rule, ctm, clip);
            }
            if stroke
                && self.color.paints(true)
                && !self.paint_nonseparable(&path, mode, None, true, canvas)
            {
                self.diag.blend_modes_ignored += 1;
                let paint = self.solid_authored(
                    true,
                    self.gs.current.stroke_alpha,
                    tiny_skia::BlendMode::SourceOver,
                );
                let clip = self.gs.current.clip_ref();
                canvas.stroke(&path, &paint, &self.stroke_params(), ctm, clip);
            }
        }

        // The pattern fill runs HERE rather than in the branch above
        // because it needs `&mut self` (it resolves resources and records
        // diagnostics) while `clip` above is an immutable borrow of the
        // same graphics state. It reads that clip itself, so nothing is
        // lost by the move; the ordering against the stroke is unchanged
        // in every case that can arise, since a path filled with a pattern
        // and stroked in a solid colour paints fill-then-stroke either way.
        if let Some(rule) = pattern_fill {
            self.paint_with_pattern(&path, rule, false, canvas);
        }

        // The overprint composites run here, after the `clip` borrow ends,
        // for the same reason the pattern fill does. Fill before stroke,
        // matching the order the ordinary paints above would have used.
        if overprint_fill_pending
            && let Some(rule) = fill_rule
            && !self.paint_overprint(&path, Some(rule), false, canvas)
        {
            // Could not composite -- paint normally rather than paint
            // nothing, and SAY SO. A silent fallback here would leave the
            // operator with a knocked-out backdrop and a diagnostic
            // claiming overprint was honoured.
            self.diag.overprint_refused += 1;
            let paint = self.solid_authored(
                false,
                self.gs.current.fill_alpha,
                self.gs.current.blend_mode,
            );
            let ctm = self.gs.current.ctm;
            let clip = self.gs.current.clip_ref();
            canvas.fill(&path, &paint, rule, ctm, clip);
        }
        if overprint_stroke_pending && !self.paint_overprint(&path, None, true, canvas) {
            self.diag.overprint_refused += 1;
            let paint = self.solid_authored(
                true,
                self.gs.current.stroke_alpha,
                self.gs.current.blend_mode,
            );
            let ctm = self.gs.current.ctm;
            let clip = self.gs.current.clip_ref();
            canvas.stroke(&path, &paint, &self.stroke_params(), ctm, clip);
        }

        // NOW tighten the clip (§8.5.4: after the path is painted).
        if let Some(rule) = pending_clip {
            intersect_clip(
                &mut self.gs.current,
                &path,
                rule,
                ctm,
                canvas,
                &mut self.clip_cache,
            );
        }
    }
}

/// Intersect `state`'s clipping path with `path` (given in the space
/// `ctm` maps to device space), per §8.5.4.
///
/// Shared by `W`/`W*` and by a form XObject's `/BBox` (§8.10.1 step c),
/// which is the same operation on the same representation — a form's
/// bounding box is not a special kind of clip, it is just a clip whose
/// rectangle happens to come from a dictionary instead of the content
/// stream.
///
/// The intersection is a **per-pixel multiply** of coverage masks. That
/// is sound only because PDF clips never grow: §8.5.4 NOTE 2 — "the
/// clipping path can only be reduced in size; it can never be
/// enlarged" — so there is no need for path booleans, and `q`/`Q` (or,
/// for a form, discarding the nested state) is the only way back.
///
/// A failure to allocate the mask leaves the clip unchanged, which
/// paints *more* than it should rather than less; the alternative
/// (treating it as "clip everything") would silently blank content.
/// Would a bounding-box cull skip this paint?
///
/// True when a clip is in force and the paint's device bounds miss the
/// clip's bbox entirely. **Reporting only** — no cull is performed,
/// because the answer on the reference CAD sheet is 1.34% of clipped
/// paints (2026-08-07) and a cull that skips one paint in seventy-five
/// costs more in branches than it saves in fills. The counter exists so
/// the next proposal starts from the number.
fn paint_is_cullable(path: &Path, ctm: Transform, bbox: Option<(f32, f32, f32, f32)>) -> bool {
    let Some((l, t, r, b)) = bbox else {
        return false;
    };
    let Some(pb) = path.bounds().transform(ctm) else {
        return false;
    };
    pb.right() < l || pb.left() > r || pb.bottom() < t || pb.top() > b
}

/// A pattern's `/Matrix` (Table 75/76), defaulting to the identity.
///
/// Six numbers `[a b c d e f]` in the usual §8.3.3 order. A malformed or
/// short array falls back to the identity rather than refusing: the entry
/// is optional, its default IS the identity, and a pattern drawn unmoved
/// is far more recoverable than one not drawn at all.
fn pattern_matrix(doc: &DocumentView<'_>, dict: &Dict) -> Transform {
    let Some(arr) = dict
        .get(b"Matrix")
        .map(|o| doc.resolve(o))
        .and_then(Object::as_array)
    else {
        return Transform::identity();
    };
    let nums: Vec<f32> = arr
        .iter()
        .filter_map(|o| doc.resolve(o).as_number())
        .map(|n| n as f32)
        .collect();
    match nums.as_slice() {
        &[a, b, c, d, e, f] => Transform::from_row(a, b, c, d, e, f),
        _ => Transform::identity(),
    }
}

fn intersect_clip(
    state: &mut GraphicsState,
    path: &Path,
    rule: FillRule,
    ctm: Transform,
    canvas: &mut Canvas<'_>,
    cache: &mut crate::clip_cache::ClipCache,
) {
    // RECORDING: a clip becomes a DEFINITION, not a mask.
    //
    // A `tiny_skia::Mask` is device-sized, so a recorded one would be valid
    // only for the geometry that built it — and surviving a change of
    // viewport is the entire point of a display list
    // (`crate::display_list` module docs §2.2). The bounding box below is
    // still maintained, in exactly the arithmetic the painting path uses
    // further down, because it is graphics state `q`/`Q` must carry either
    // way.
    //
    // This branch is also why recording is CHEAPER than rendering rather
    // than an extra pass on top of one: ~24,000 mask builds and multiplies
    // do not happen.
    if let Some(id) = canvas.record_clip(ClipDef {
        path: Some(std::sync::Arc::new(path.clone())),
        rule,
        ctm,
        parent: state.clip_id,
    }) {
        state.clip_id = Some(id);
        #[allow(clippy::cast_precision_loss)]
        let (w, h) = (canvas.width() as f32, canvas.height() as f32);
        if let Some(b) = path.bounds().transform(ctm) {
            let (nl, nt) = (b.left().max(0.0), b.top().max(0.0));
            let (nr, nb) = (b.right().min(w), b.bottom().min(h));
            let accum = match state.clip_bbox {
                Some((pl, pt, pr, pb)) => (nl.max(pl), nt.max(pt), nr.min(pr), nb.min(pb)),
                None => (nl, nt, nr, nb),
            };
            state.clip_bbox = Some(accum);
        }
        return;
    }
    // MEASUREMENT ABLATION — always false without the `profile` feature,
    // where this folds away entirely.
    //
    // Returning here leaves `state.clip` at `None`, which is exactly the
    // confound that produced the day's worst number: it suppresses not
    // only mask construction but clip SAMPLING in every later paint and
    // the `Arc` clone in every `q`. `Ablation::confounds` names all
    // three so a delta measured this way cannot be read as the cost of
    // construction alone (R164).
    if crate::profile::skip_clip_build() {
        return;
    }
    // Sub-phase timing. `timing_enabled()` is a compile-time constant, so
    // without the `profile` feature every `Instant::now()` below folds
    // away and a shipping render pays nothing.
    //
    // Timed rather than ablated ON PURPOSE — see `profile::note_clip_phases`.
    // An ablation of one phase removes others with it and yields an upper
    // bound (R164); a timer removes nothing and confounds nothing. Clips
    // run 24,128 times over ~350 µs each, so a ~25 ns timer is ~1e-4 of
    // the measured quantity.
    //
    // This comment used to add that `render-profile` "prints the
    // un-instrumented total beside it so the overhead is shown, not
    // argued". **It cannot** — `timing_enabled()` is
    // `cfg!(feature = "profile")`, a compile-time constant, so one
    // invocation only ever produces one of the two totals. The claim was
    // unimplementable rather than merely stale, and the same sentence
    // was already corrected in `profile.rs`; this copy survived, which
    // is the single-location-amendment failure again.
    //
    // The overhead was measured instead, across builds: three
    // instrumented runs at 9.49 / 9.52 / 10.04 s (5.8% spread) against
    // 9.28 s un-instrumented — **below this machine's noise**, so the
    // ~1e-4 the arithmetic predicts is not resolvable here and is not
    // claimed to be.
    // Census of how often the same mask gets rebuilt. Keyed on the
    // BUILD inputs only — see `profile::note_clip_identity` for why the
    // clip already in force is excluded — so it measures what a cache of
    // `Mask::new` + `fill_path` could serve, not the whole operation.
    crate::profile::note_clip_identity(
        path,
        matches!(rule, FillRule::EvenOdd),
        ctm,
        canvas.width(),
        canvas.height(),
        state.clip.as_ref().map(std::sync::Arc::as_ptr),
    );

    // Has this exact mask already been built under this exact incoming
    // clip? On the reference CAD sheet the answer is yes 99.83% of the
    // time — 24,128 applications over 40 distinct masks, one path alone
    // accounting for 97.3% — and a hit returns the already-intersected
    // `Arc`, skipping `Mask::new`, `fill_path` AND the multiply.
    //
    // The bbox is taken from the cache rather than recomputed below
    // because it is a function of the same inputs: `clip` and
    // `clip_bbox` are only ever written as a pair, so a given mask
    // always carries the same bbox. See `ClipCache::get`.
    let key =
        crate::clip_cache::ClipCache::build_key(path, rule, ctm, canvas.width(), canvas.height());
    if let Some((cached, bbox)) = cache.get(key, state.clip.as_ref()) {
        // Still counted: the census measures how often a clip is
        // APPLIED, and a served application is an application. Leaving
        // it out would make the very repetition this cache exploits
        // vanish from the instrument that found it.
        if let Some((l, t, r, b)) = bbox {
            let (w, h) = (canvas.width() as f32, canvas.height() as f32);
            let page_area = w * h;
            let indiv = ((r - l).max(0.0) * (b - t).max(0.0)) / page_area;
            crate::profile::note_clip(indiv, indiv);
        }
        state.clip_bbox = bbox;
        state.clip = Some(cached);
        return;
    }
    let incoming = state.clip.clone();

    let timed = crate::profile::timing_enabled();
    let t0 = timed.then(std::time::Instant::now);
    let Some(mut mask) = Mask::new(canvas.width(), canvas.height()) else {
        return;
    };
    let t1 = timed.then(std::time::Instant::now);
    mask.fill_path(path, rule, true, ctm);
    let t2 = timed.then(std::time::Instant::now);
    if let Some(old) = &state.clip {
        // Multiply ONLY inside the new path's device-space bounds.
        //
        // Outside them `fill_path` wrote nothing and `Mask::new` zeroed
        // the buffer, so `new_px` is 0 there and `0 × old / 255 == 0` —
        // the multiply is provably a no-op. Restricting the loop is an
        // identity, not an approximation.
        //
        // CORRECTED 2026-08-07, same day it was written. This comment
        // originally read "clips in real drawings are SMALL relative to
        // the paper … rectangles that mostly cover a few percent of it".
        // **That is false**, and the error was a fraction printed as a
        // percent: the reference CAD sheet's mean clip bbox is 66.36% of
        // the page, not 0.663%. Measured concretely, its first clips
        // cover 87%, 65%, 100%, 81% and 95% of the sheet.
        //
        // The bound is still an identity and still worth keeping — it
        // skips the ~34% of the page that lies outside the new path, and
        // the tail of small clips (0.98%, 2.58%) benefits a lot. But it
        // is a third off the work, not the two orders of magnitude the
        // original wording implied, and no optimization should be scoped
        // on the premise that clips are tiny. `tools/render-profile`
        // reports this figure so the claim stays checkable; it prints an
        // explicit note when clips are large.
        let w = mask.width() as usize;
        let h = mask.height() as usize;
        let (x0, y0, x1, y1) = match path.bounds().transform(ctm) {
            // Clamp to the mask; a clip may legitimately hang off-page.
            Some(b) => (
                (b.left().floor().max(0.0) as usize).min(w),
                (b.top().floor().max(0.0) as usize).min(h),
                ((b.right().ceil().max(0.0) as usize) + 1).min(w),
                ((b.bottom().ceil().max(0.0) as usize) + 1).min(h),
            ),
            // No usable bounds: fall back to the whole page rather than
            // silently skipping the intersection.
            None => (0, 0, w, h),
        };
        // Row SLICES, not indexing. An indexed inner loop costs a bounds
        // check per pixel and does not autovectorize; slicing the row once
        // and zipping keeps the SIMD the full-page version got for free.
        // Measured: the indexed form was SLOWER than the whole-page loop
        // it replaced at 0.25x and 0.5x, because a vectorized pass over
        // the whole page beats a scalar pass over part of it.
        let old_data = old.data();
        let new_data = mask.data_mut();
        for y in y0..y1 {
            let row = y * w;
            let new_row = &mut new_data[row + x0..row + x1];
            let old_row = &old_data[row + x0..row + x1];
            for (n, o) in new_row.iter_mut().zip(old_row.iter()) {
                *n = ((u16::from(*n) * u16::from(*o)) / 255) as u8;
            }
        }
    }
    if let (Some(t0), Some(t1), Some(t2)) = (t0, t1, t2) {
        let t3 = std::time::Instant::now();
        crate::profile::note_clip_phases(
            (t1 - t0).as_nanos() as u64,
            (t2 - t1).as_nanos() as u64,
            (t3 - t2).as_nanos() as u64,
        );
    }
    // Maintain the bbox alongside the mask. `state` is the live graphics
    // state, so `q`/`Q` carry this exactly as they carry the mask — see
    // `GraphicsState::clip_bbox` for why anywhere else is wrong.
    let (w, h) = (mask.width() as f32, mask.height() as f32);
    let page_area = w * h;
    if let Some(b) = path.bounds().transform(ctm) {
        let (nl, nt) = (b.left().max(0.0), b.top().max(0.0));
        let (nr, nb) = (b.right().min(w), b.bottom().min(h));
        let indiv = ((nr - nl).max(0.0) * (nb - nt).max(0.0)) / page_area;
        let accum = match state.clip_bbox {
            Some((pl, pt, pr, pb)) => (nl.max(pl), nt.max(pt), nr.min(pr), nb.min(pb)),
            None => (nl, nt, nr, nb),
        };
        state.clip_bbox = Some(accum);
        let accum_area = ((accum.2 - accum.0).max(0.0) * (accum.3 - accum.1).max(0.0)) / page_area;
        crate::profile::note_clip(indiv, accum_area);
    }
    let built = std::sync::Arc::new(mask);
    // Cached AFTER intersection, keyed on what was intersected WITH, so
    // a hit can hand back this exact `Arc` rather than rebuilding and
    // re-multiplying. `incoming` was cloned before `state.clip` was
    // overwritten, and holding it keeps its address pinned so pointer
    // identity stays sound (`clip_cache`'s ABA note).
    cache.insert(
        key,
        incoming,
        std::sync::Arc::clone(&built),
        state.clip_bbox,
    );
    state.clip = Some(built);
}

/// Read a six-number `/Matrix` entry (Table 95), in `f64`.
///
/// Returns `None` when absent or malformed, which the caller treats as
/// Table 95's documented default — the identity matrix. Note this is an
/// **array** operand, unlike `cm`/`Tm`, whose six numbers are loose
/// operands.
///
/// A form's `/Matrix` is the other place a page coordinate enters the CTM
/// (§8.10.1 step b), and it composes with the same deep-zoom base a `cm`
/// does — so it loses precision the same way and is repaired the same way.
/// See [`Mat64`].
///
/// **There is deliberately no `f32` sibling.** One existed, for about ten
/// minutes, as a narrowing wrapper — and `-D dead-code` observed that
/// nothing called it: every consumer wants the `f64` value and narrows at
/// its own leaf, which is the discipline `Mat64` exists to impose. A
/// convenience wrapper would have been a second, lossier reader of one
/// dictionary entry, sitting there to be picked by accident.
fn matrix_entry64(doc: &DocumentView<'_>, dict: &Dict) -> Option<Mat64> {
    let items = doc.resolve(dict.get(b"Matrix")?).as_array()?;
    let n: Vec<f64> = items.iter().filter_map(Object::as_number).collect();
    match n.as_slice() {
        &[a, b, c, d, e, f] => Some(Mat64::from_row(a, b, c, d, e, f)),
        _ => None,
    }
}

/// The first `n` operands of an operation as `f64`.
///
/// `Interpreter::apply` narrows every operand to `f32` up front, which is
/// right for colours, widths and dash phases and wrong for a matrix: `cm`
/// and `/Matrix` carry page coordinates that then get multiplied by a
/// deep-zoom scale, and seven digits is not enough to survive the
/// cancellation that follows. This reads the same operands again without
/// that narrowing, and is called only from the two matrix sites.
fn operand_f64s(op: &Operation<'_>, n: usize) -> Option<Vec<f64>> {
    let v: Vec<f64> = op
        .operands
        .iter()
        .filter_map(|t| match &t.kind {
            ContentTokenKind::Operand(o) => o.as_number(),
            _ => None,
        })
        .collect();
    (v.len() == n).then_some(v)
}

/// Read a four-number rectangle entry, **normalized** per §7.9.5.
///
/// §7.9.5: a rectangle is written `[llx lly urx ury]` but "the two
/// corners may be given in either order", so both axes are sorted here.
///
/// This is the exact opposite of how a `/Decode` pair must be handled
/// (`crate::image`): there, `Dmin > Dmax` is §8.9.5.2's *inversion*
/// idiom and normalizing destroys it. Two arrays of numbers, opposite
/// rules — worth naming at both sites so neither gets "fixed" to match
/// the other.
fn rect_entry(doc: &DocumentView<'_>, dict: &Dict, key: &[u8]) -> Option<Rect> {
    let items = doc.resolve(dict.get(key)?).as_array()?;
    let n: Vec<f32> = items
        .iter()
        .filter_map(|o| doc.resolve(o).as_number().map(|v| v as f32))
        .collect();
    let &[x0, y0, x1, y1] = n.as_slice() else {
        return None;
    };
    Rect::from_ltrb(x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1))
}

/// Clamp a shading's signed device-space region to a pixel rectangle the
/// colorant buffer can scan.
///
/// `Shading::paint` works in `i32` because a shading's geometry can extend
/// off the page in either direction and the sign carries meaning while the
/// region is being derived. A buffer scan cannot use a negative bound, and
/// silently casting one to `u32` would wrap to four billion — which reads
/// as "scan the whole page" on a good day and as a panic on a bad one.
///
/// Returns an **empty** rectangle (`x0 == x1`) when the region lies wholly
/// off-page, which the scan loops treat as "nothing to do".
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn clamp_region(region: (i32, i32, i32, i32), width: u32, height: u32) -> (u32, u32, u32, u32) {
    let (l, t, r, b) = region;
    let x0 = l.max(0) as u32;
    let y0 = t.max(0) as u32;
    let x1 = (r.max(0) as u32).min(width);
    let y1 = (b.max(0) as u32).min(height);
    if x0 >= x1 || y0 >= y1 {
        return (0, 0, 0, 0);
    }
    (x0, y0, x1, y1)
}

/// The last name operand of an operator (`Do`, `gs`, `sh`, …).
///
/// Taken from the END of the operand run for the same reason
/// [`last_string`] is: §7.8.2 says no operands are left over, producers
/// disagree, and junk is far likelier to precede the real operand than
/// to follow it.
fn last_name(op: &Operation<'_>) -> Option<Vec<u8>> {
    op.operands.iter().rev().find_map(|t| match &t.kind {
        ContentTokenKind::Operand(Object::Name(n)) => Some(n.as_bytes().to_vec()),
        _ => None,
    })
}

/// The last string operand of a text-showing operator.
///
/// Taken from the END of the operand run because `"` puts two numbers
/// before its string, and because a malformed stream with leftover
/// operands (§7.8.2 says there shall be none; producers disagree) is
/// far more likely to have junk before the real operand than after it.
fn last_string(op: &Operation<'_>) -> Option<Vec<u8>> {
    op.operands.iter().rev().find_map(|t| match &t.kind {
        ContentTokenKind::Operand(Object::String(s)) => Some(s.clone()),
        _ => None,
    })
}

/// Whether an image's own outline should be antialiased when it is filled.
///
/// # The defect this exists to fix
///
/// An image is painted as a fill of its unit square (§8.9.4), and that fill
/// was unconditionally antialiased. Where two images **abut**, the shared
/// boundary pixel is partially covered by each, and source-over compositing
/// of two partial coverages does not reach 1:
///
/// ```text
/// coverage = a + b·(1 − a)      a = b = 0.5  ⇒  0.75
/// ```
///
/// so a quarter of the background shows through a join that should be
/// seamless. This is **conflation**: antialiased abutting edges failing to
/// sum to full coverage.
///
/// **Measured on a real drawing** (`pdfceGUI` report, 2026-08-18, reproduced
/// here before changing anything): a SolidWorks shaded view is dozens of
/// small masked image XObjects laid edge to edge — `images=92`,
/// `images_masked=92`, `shadings=0` — and a flat `RGB(195,38,38)` panel was
/// crossed by single rows of `RGB(206,78,78)` and `RGB(210,93,93)` every
/// ~30.5 device pixels, i.e. **19 %–25 % of the white page bleeding through
/// an opaque fill**.
///
/// **The decisive measurement was that it is device-space, not content.**
/// Re-rendered at half scale, the seams kept the *same colours* and the
/// *same one-pixel thickness* while their spacing halved. Lighter rows in
/// the source images could not do that — texels would have merged and both
/// colour and thickness would have moved. The seam is created at composite
/// time.
///
/// # Why turning it off is CORRECT here and not merely convenient
///
/// §8.9.4 maps the unit square through the CTM and samples the image at
/// device pixels whose centres fall inside it. **The edge of an image is
/// where sampling stops, not a curve that needs smoothing.** Two images that
/// abut therefore tile the device grid exactly, which is why Acrobat has no
/// seam. Antialiasing an image's *outline* is a rasterizer's embellishment
/// that the imaging model never asked for.
///
/// # The two conditions, and why each is needed
///
/// 1. **Axis-aligned CTM.** With rotation or skew the outline really is a
///    diagonal edge across the pixel grid, and a hard edge there looks worse
///    than a seam — visible stair-stepping on every rotated image, to cure an
///    artefact that needs *abutting* rotated tiles to appear at all. Both the
///    unrotated case (`kx = ky = 0`) and the quarter-turn case
///    (`sx = sy = 0`) map the unit square to an axis-aligned device
///    rectangle, so both qualify.
/// 2. **At least one device pixel in each axis.** Without antialiasing, a
///    shape covering no pixel centre paints **nothing**. A sub-pixel image
///    would vanish outright rather than render as a faint smear — a worse
///    failure than the seam this function exists to remove, and one that
///    would be silent. Such an image keeps its antialiasing and, being
///    sub-pixel, cannot show a visible seam anyway.
///
/// Both conditions are deliberately checked on the **CTM alone**: the
/// decision must not depend on the image's texel count, or two abutting
/// tiles of different resolutions would disagree about whether their shared
/// edge is antialiased and the seam would come back on exactly the boundary
/// that matters.
fn image_edge_needs_antialiasing(ctm: Transform) -> bool {
    // Exact zero rather than an epsilon: these come from concatenated `cm`
    // operands, and a CTM that is *nearly* axis-aligned is one that will
    // produce a *nearly* horizontal edge — precisely the case that still
    // wants smoothing. Only an exactly axis-aligned mapping tiles the
    // device grid exactly.
    let upright = ctm.kx == 0.0 && ctm.ky == 0.0;
    let quarter_turn = ctm.sx == 0.0 && ctm.sy == 0.0;
    if !upright && !quarter_turn {
        return true;
    }
    // Device extent of the unit square along each axis.
    let (wide, tall) = if upright {
        (ctm.sx.abs(), ctm.sy.abs())
    } else {
        (ctm.kx.abs(), ctm.ky.abs())
    };
    wide < 1.0 || tall < 1.0
}

#[cfg(test)]
mod image_edge_antialiasing_tests {
    use super::image_edge_needs_antialiasing;
    use tiny_skia::Transform;

    /// An ordinary upright image tile: no antialiasing, so abutting tiles
    /// tile the device grid exactly.
    #[test]
    fn an_upright_tile_of_several_pixels_is_not_antialiased() {
        let ctm = Transform::from_row(30.5, 0.0, 0.0, 30.5, 100.0, 200.0);
        assert!(!image_edge_needs_antialiasing(ctm));
    }

    /// A quarter-turn still maps the unit square to an axis-aligned device
    /// rectangle, so it qualifies too. Missing this case would leave every
    /// rotated-90° page seamed while the upright ones were clean — the kind
    /// of partial fix that reads as "the bug is back" on one document.
    #[test]
    fn a_quarter_turn_is_still_axis_aligned() {
        let ctm = Transform::from_row(0.0, 20.0, -20.0, 0.0, 50.0, 50.0);
        assert!(!image_edge_needs_antialiasing(ctm));
    }

    /// ★ Rotation KEEPS antialiasing. The outline is a genuine diagonal
    /// across the pixel grid there, and a hard edge would stair-step on
    /// every rotated image — a visible cost paid on common content to cure
    /// an artefact that needs abutting ROTATED tiles to appear at all.
    #[test]
    fn a_rotated_image_keeps_its_antialiasing() {
        let ctm = Transform::from_row(20.0, 5.0, -5.0, 20.0, 0.0, 0.0);
        assert!(image_edge_needs_antialiasing(ctm));
    }

    /// Skew without rotation is equally not axis-aligned.
    #[test]
    fn a_skewed_image_keeps_its_antialiasing() {
        let ctm = Transform::from_row(20.0, 0.0, 7.0, 20.0, 0.0, 0.0);
        assert!(image_edge_needs_antialiasing(ctm));
    }

    /// ★ THE GUARD THAT STOPS A SILENT DISAPPEARANCE. Without
    /// antialiasing, a shape covering no pixel centre paints NOTHING, so a
    /// sub-pixel image would vanish outright rather than render faintly.
    /// That is a worse failure than the seam, and a silent one.
    #[test]
    fn a_sub_pixel_image_keeps_its_antialiasing_so_it_cannot_vanish() {
        let thin = Transform::from_row(40.0, 0.0, 0.0, 0.4, 0.0, 0.0);
        assert!(
            image_edge_needs_antialiasing(thin),
            "a 0.4px-tall image must keep AA or it disappears"
        );
        let narrow = Transform::from_row(0.6, 0.0, 0.0, 40.0, 0.0, 0.0);
        assert!(image_edge_needs_antialiasing(narrow));
    }

    /// The boundary from both sides, so the threshold is pinned to a
    /// number rather than to "small".
    #[test]
    fn the_one_pixel_threshold_holds_from_both_sides() {
        let under = Transform::from_row(10.0, 0.0, 0.0, 0.999, 0.0, 0.0);
        let over = Transform::from_row(10.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        assert!(image_edge_needs_antialiasing(under));
        assert!(!image_edge_needs_antialiasing(over));
    }

    /// A negative scale is a flip, not a rotation — still axis-aligned.
    /// `/Decode` arrays and flipped CTMs are ordinary in real files, and
    /// testing on the sign rather than the magnitude would seam every one
    /// of them.
    #[test]
    fn a_flipped_image_is_still_axis_aligned() {
        let ctm = Transform::from_row(-30.0, 0.0, 0.0, -30.0, 0.0, 0.0);
        assert!(!image_edge_needs_antialiasing(ctm));
    }
}
