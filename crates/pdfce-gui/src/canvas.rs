//! # canvas — the one shared canvas-interaction substrate (Pass 12.0)
//!
//! Implements `docs/ui_specs/pass-12.0-canvas-substrate.md`: the single
//! focusable, interactive page canvas that every future editing tool
//! (Acrobat-style text edit, the measurement/dimensioning tools, vector
//! editing) layers onto, **built once** (decision 010 R60, decision 011
//! §2.6 "F_canvas_substrate_R60"). This Pass ships the mechanism with
//! **zero real tools attached** — a deliberately, verifiably no-op default
//! state — so every rule below is exercised and unit-tested today without a
//! single document-mutating tool existing.
//!
//! ## Why the type that carries "which tool" is uninhabited on purpose
//!
//! [`CanvasTool`] has no variants. That is not a placeholder: it is the
//! type-level proof that this Pass adds no tool. `Option<CanvasTool>` is a
//! normal, sized type whose payload can never be constructed, so
//! `active_tool` can only ever observably be `None` until a future Pass adds
//! a variant — the "zero behaviour change to the existing viewer"
//! acceptance criterion made *structural* rather than merely tested.
//!
//! ## Why every state-machine decision here is a pure `bool`/`Option`
//! function, never a `match` on [`CanvasTool`]
//!
//! Because [`CanvasTool`] is uninhabited, a decision written as
//! `match self.active_tool { Some(tool) => .., None => .. }` would have zero
//! live branches to test until a real variant exists — an entire class of
//! substrate behaviour (pan suppression's `true` branch, the two-stage
//! Escape's "cancel gesture" branch, the overlay-paint branch) would ship
//! untested and first execute only when Pass 6.1 lands, exactly the wrong
//! moment to discover a substrate bug. So every decision is a pure function
//! over `bool`/`Option<T>`-shaped inputs a future tool's own state supplies
//! ([`canvas_suppresses_pan`], [`resolve_escape`], [`GestureInterrupt`]),
//! and 100% of the logic is unit-testable now (spec §3.4).
//!
//! ## GUI-core separation
//!
//! [`CanvasTargetProvider`] lives here, in `pdfce-gui`, deliberately —
//! click/marquee hit-testing *for selection* is a GUI-interaction concept,
//! and putting the trait in `pdfce-core` would give the engine a GUI-shaped
//! dependency exactly backwards from the load-bearing invariant
//! (`ARCHITECTURE.md` §3). Pass 9a's real provider is a thin `pdfce-gui`
//! adapter that CALLS INTO `pdfce-core`'s read-only object model (which
//! stays GUI-free); the adapter owns the trait impl, the object model owns
//! none of it.

use std::collections::BTreeSet;

use eframe::egui::{Color32, Pos2, Rect, Shape, Stroke};
use pdfce_core::text_edit::{GlyphRef, TextPosition};
use pdfce_core::vector::{SnapCandidate, SnapKind};

// ---------------------------------------------------------------------------
// Tool-mode dispatch (spec §3)
// ---------------------------------------------------------------------------

/// Which interactive canvas tool is active, if any — **uninhabited at this
/// Pass** (see the module docs for why zero variants is a deliberate,
/// load-bearing choice).
///
/// The substrate's own no-op default is `None` (via `Option<CanvasTool>`):
/// ordinary pan/zoom plus click-to-select against whatever
/// [`CanvasTargetProvider`] is attached. Every tool-bearing Pass that lands
/// after this one adds ITS variants directly to THIS enum, in THIS crate,
/// rather than inventing a parallel `active_tool`-shaped field — R60's "one
/// substrate" binds the dispatch *type* itself, not merely the widget.
/// Expected future variants, for orientation only (not authored here):
/// Pass 6.1's `Ink`/`Square`/`Circle`/`Line`/`Polygon`/`PolyLine`/
/// `Highlight`/`Underline`/`StrikeOut`/`Squiggly`; Pass 8's `Redact`; Pass
/// 12.M2's dimension-pick tools; Pass 9c-min's vector-edit tools. (Pass 7's
/// form-fill adds NONE — it needs no tool mode at all and drives the
/// focusable canvas with `active_tool` staying `None` throughout a fill.)
///
/// ## First real inhabitant (Pass 14.3)
///
/// [`CanvasTool::TextEdit`] is the enum's FIRST variant — the Acrobat-style
/// in-place page-text editor (`docs/ui_specs/pass-14.3-text-edit-ui.md`
/// §0.1). It is what finally exercises `canvas_suppresses_pan`'s `true`
/// branch, `resolve_escape`'s `CancelGesture`/`ExitTool` branches and the
/// [`GestureInterrupt::Discard`] path with a live `tool_active == true`;
/// until this Pass those were unit-tested against synthetic `bool`s only.
/// The tool's caret/selection state lives in `pdfce-gui`'s own
/// `TextEditState`, NOT in `canvas_selection`/[`TargetId`] — text selection is
/// a contiguous `(anchor, active)` caret span, categorically different from
/// the discrete-object selection [`CanvasTargetProvider`] models (spec §0.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasTool {
    /// Acrobat-style in-place page-text editing (Pass 14.3): click→caret,
    /// type→edit (a reviewable `PendingEdit` preview), drag/double/triple-click
    /// selection, size/colour/family formatting, all accepted or rejected by
    /// the operator before committing through `EditSession` (rule 4).
    TextEdit,
    /// Author brand-new page content at an operator-chosen point or box
    /// (Pass 16.0-16.2 / decision 016 / FF-D): click→point origin, drag→wrap
    /// box (16.1), type→live preview, Accept→one `CommandKind::AddText`,
    /// Reject/Esc→nothing added. NEVER a Pass-6.2 FreeText annotation (R78).
    ///
    /// A genuine SECOND variant, deliberately NOT a `TextEdit` sub-mode —
    /// decided the OPPOSITE way from 15.2's reflow (Pass 16.2 spec §0.1): a
    /// plain click's meaning inside `TextEdit` ("a miss clears the caret") must
    /// not be silently repurposed into "sometimes creates page content," so a
    /// separate, deliberately-entered tool owns the "a click always places new
    /// text" semantics. `active_tool: Option<CanvasTool>` is a single value
    /// (Pass 12.0), so `TextEdit` and `AddText` are mutually exclusive by
    /// construction — [`tool_builds_text_edit`]/[`tool_builds_add_text`] make
    /// that invariant a headless-tested predicate.
    AddText,
    /// Linear dimension (Pass 12.M2, ui-spec §1.1): two snapped point picks
    /// author a scaled measurement. A plain click while this tool is active is
    /// always a point-pick, never an object-selection click.
    MeasureLinear,
    /// Radius/diameter dimension from a best-fit (Taubin) circle (Pass 12.M2):
    /// object/node picks build the fit set; a display toggle picks radius vs
    /// diameter on the SAME geometry (ui-spec §1.1 — one tool, not two).
    MeasureCircular,
    /// Scale dimension (Pass 12.M2, ui-spec §4): draw a reference line, then
    /// enter a real length + units OR a ratio to back-calc the active group's
    /// scale. A distinct, deliberately-entered tool (never a hidden linear
    /// sub-mode — ui-spec §1.1).
    MeasureScale,
    /// Basic vector editing (Pass 9c-min, decision 011 §2.5): click selects
    /// an object; **dragging** a selected object translates it (move);
    /// dragging an anchor of the selected object relocates that node
    /// (drag-node); **Delete** removes the selected object. The three
    /// operations are one deliberately-entered tool because a plain click's
    /// meaning here ("select / start an edit gesture") must not be silently
    /// repurposed from the pan/marquee default — the same reasoning that made
    /// `AddText` a separate tool from `TextEdit`. Each committed gesture is
    /// one undoable `EditSession` command (`move_object`/`move_node`/
    /// `delete_object`), snapped via the 12.M1 engine, previewed before
    /// commit (fuzzy-never-sneaky).
    VectorEdit,
}

impl CanvasTool {
    /// Whether this is one of the three Pass 12.M2 measure tools (they share
    /// the snap-indicator overlay and the two-point-pick gesture family —
    /// ui-spec §1.1/§2.2).
    #[must_use]
    pub fn is_measure(self) -> bool {
        matches!(
            self,
            CanvasTool::MeasureLinear | CanvasTool::MeasureCircular | CanvasTool::MeasureScale
        )
    }
}

/// A resolved add-text placement from a pointer gesture (Pass 16.2 §3), in PDF
/// **default user space** — the pure geometry the canvas handler turns into an
/// in-progress draft.
///
/// Kept egui-free (plain `f64`) so the point-vs-box decision and the
/// degenerate-drag→point fallback are unit-tested here, exactly like every
/// other canvas state transition in this module (the reason `canvas.rs`/
/// `viewer.rs` exist: `main.rs` is not headlessly testable, these are).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AddTextPlacement {
    /// A single-line run growing rightward from `(x, y)` (16.0, point mode).
    Point {
        /// Origin x, PDF user space.
        x: f64,
        /// Origin (baseline) y, PDF user space.
        y: f64,
    },
    /// A wrap box, lower-left `(llx, lly)`, `width`×`height` (16.1, box mode).
    Box {
        /// Lower-left x, PDF user space.
        llx: f64,
        /// Lower-left y, PDF user space.
        lly: f64,
        /// Box width, points.
        width: f64,
        /// Box height, points.
        height: f64,
    },
}

/// Resolve a rubber-band drag from `start` to `end` (both PDF-space points)
/// into a Point or a Box placement (Pass 16.2 §3.2).
///
/// The corners are normalised (min/max) so a drag in ANY direction yields the
/// same box. **Degenerate-drag handling is a DELIBERATE divergence from Pass
/// 6.1's "degenerate-drag discards silently" rule, named here so a future
/// reader does not "fix" it back:** if the box is smaller than `min_box_pt` in
/// width OR height, this returns a `Point` anchored at the drag's START — a
/// near-zero drag still names a perfectly valid construct (a point insertion),
/// so falling back preserves the operator's evident intent to place *something*
/// rather than eating an almost-successful gesture (unlike a shape draw, where
/// a zero-size square has no sensible minimal form to keep). The `!(… && …)`
/// shape also routes a non-finite drag to the Point fallback.
#[must_use]
pub fn resolve_drag_placement(
    start: (f64, f64),
    end: (f64, f64),
    min_box_pt: f64,
) -> AddTextPlacement {
    let llx = start.0.min(end.0);
    let lly = start.1.min(end.1);
    let urx = start.0.max(end.0);
    let ury = start.1.max(end.1);
    let width = urx - llx;
    let height = ury - lly;
    if !(width >= min_box_pt && height >= min_box_pt) {
        return AddTextPlacement::Point {
            x: start.0,
            y: start.1,
        };
    }
    AddTextPlacement::Box {
        llx,
        lly,
        width,
        height,
    }
}

/// Whether entering `tool` builds the Pass 14.3 `TextEdit` per-page state — the
/// TextEdit half of the mutual-exclusion invariant (Pass 16.2 §0.1).
#[must_use]
pub fn tool_builds_text_edit(tool: Option<CanvasTool>) -> bool {
    matches!(tool, Some(CanvasTool::TextEdit))
}

/// Whether entering `tool` builds the Pass 16.2 `AddText` state — the AddText
/// half of the mutual-exclusion invariant. Because `active_tool` is a single
/// `Option<CanvasTool>`, this and [`tool_builds_text_edit`] are never both
/// true (the two tools' states are torn down for each other), which is exactly
/// what makes a second `CanvasTool` variant cost nothing at the dispatch layer.
#[must_use]
pub fn tool_builds_add_text(tool: Option<CanvasTool>) -> bool {
    matches!(tool, Some(CanvasTool::AddText))
}

/// Whether entering `tool` builds a Pass 12.M2 measure-tool state (linear /
/// circular / scale). Like the text-tool predicates, this is a headless-tested
/// projection of `active_tool` — a measure gesture and a text-edit gesture are
/// mutually exclusive by the single-value `Option<CanvasTool>` (ui-spec §1.3).
#[must_use]
pub fn tool_builds_measure(tool: Option<CanvasTool>) -> bool {
    tool.is_some_and(CanvasTool::is_measure)
}

/// Whether entering `tool` builds the Pass 9c-min `VectorEdit` object-edit
/// state (move / drag-node / delete). Like the other tool predicates, a
/// headless-tested projection of `active_tool`: a vector-edit gesture and a
/// text/measure gesture are mutually exclusive by the single-value
/// `Option<CanvasTool>` (decision 011 §2.5, R60 one substrate).
#[must_use]
pub fn tool_builds_vector_edit(tool: Option<CanvasTool>) -> bool {
    matches!(tool, Some(CanvasTool::VectorEdit))
}

/// Whether `tool` specifically builds the linear-dimension pick (ui-spec §2.1).
#[must_use]
pub fn tool_builds_measure_linear(tool: Option<CanvasTool>) -> bool {
    matches!(tool, Some(CanvasTool::MeasureLinear))
}

/// Whether `tool` builds the circular (radius/diameter) best-fit pick
/// (ui-spec §3).
#[must_use]
pub fn tool_builds_measure_circular(tool: Option<CanvasTool>) -> bool {
    matches!(tool, Some(CanvasTool::MeasureCircular))
}

/// Whether `tool` builds the scale-dimension pick (ui-spec §4).
#[must_use]
pub fn tool_builds_measure_scale(tool: Option<CanvasTool>) -> bool {
    matches!(tool, Some(CanvasTool::MeasureScale))
}

/// Whether the canvas's own click-drag should win over the `ScrollArea`'s
/// pan-by-drag for this frame (spec §1.3).
///
/// A pure function of two independently meaningful facts, never of
/// [`CanvasTool`]'s (currently nonexistent) variants:
///
/// - Pass 6.1's whole-canvas suppression: `true` whenever a drawing tool is
///   active and it did not opt into the narrower form.
/// - Pass 7's narrower suppression: only whether the drag's start position
///   falls inside `narrow_suppression_rect` (a focused text overlay's own
///   rect) matters; `tool_active` is irrelevant to it.
///
/// This Pass calls it with `tool_active == false` and
/// `narrow_suppression_rect == None` always, so it always returns `false` —
/// proven by the unit tests below with no real tool required. Pass 7
/// replaces the call site's second argument with a real hit-test; it does
/// not change this signature.
#[must_use]
pub fn canvas_suppresses_pan(tool_active: bool, narrow_suppression_rect: Option<Rect>) -> bool {
    tool_active && narrow_suppression_rect.is_none()
}

/// What happens to a tool's in-progress, uncommitted gesture when an
/// unrelated action (Undo, Save, page navigation, opening a panel…) is
/// about to happen (spec §3.3).
///
/// The active tool's OWN gesture state decides which of these applies each
/// time it is consulted — the substrate does not hardcode a policy, it only
/// guarantees there is exactly ONE place the question is asked (a single
/// enforcement point in `PdfceApp::apply`), so Pass 6.1 (discard its
/// half-drawn shape) and Pass 7 (commit its typed draft) each replace the
/// query's *result* without adding a second enforcement point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "Discard/Commit are the substrate's future-tool contract, constructed by Pass 6.1/7 gesture state; Nothing is this Pass's only reachable case (spec 3.3)" // ui-text-exempt: clippy lint justification, never displayed
)]
pub enum GestureInterrupt {
    /// Nothing is in progress — the common case, and this Pass's ONLY
    /// reachable case (no tool exists to have a gesture).
    Nothing,
    /// Discard the in-progress gesture with no further action — Pass 6.1's
    /// shapes, where nothing has been written to the `EditSession` yet.
    Discard,
    /// Commit the in-progress gesture as one `EditSession` command before
    /// the interrupting action proceeds — Pass 7's text-field draft, typed
    /// with clear intent to keep it.
    Commit,
}

// ---------------------------------------------------------------------------
// Two-stage Escape precedence (spec §3.5)
// ---------------------------------------------------------------------------

/// The single, canonical outcome of pressing Escape over the canvas — the
/// four-way precedence chain every future tool-bearing Pass slots into
/// rather than each improvising Escape's meaning against whatever else
/// Escape already does (spec §3.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscapeOutcome {
    /// A tool has an in-progress, discardable gesture → cancel the gesture,
    /// STAY in the tool. (Highest precedence.)
    CancelGesture,
    /// A tool is active with no gesture in progress → exit to view mode
    /// (`Action::SelectCanvasTool(None)`).
    ExitTool,
    /// No tool active, the substrate's OWN canvas selection is non-empty →
    /// clear it. (Unreachable until Pass 9a attaches a real provider that
    /// can make the selection non-empty.)
    ClearCanvasSelection,
    /// Nothing above applies → fall through to the EXISTING page-rail
    /// `Action::ClearSelection` binding, completely unchanged. (This Pass's
    /// only live outcome.)
    FallThroughToRailClear,
}

/// Resolve the Escape-key precedence chain (spec §3.5) — pure, testable
/// today with plain `bool`s, no real tool or selection required.
///
/// `gesture_discardable` is specifically [`GestureInterrupt::Discard`]:
/// Pass 7's commit-policy fields use their own single-stage
/// Esc-reverts-to-gesture-start rule, which is NOT this chain.
#[must_use]
pub fn resolve_escape(
    tool_active: bool,
    gesture_discardable: bool,
    canvas_selection_nonempty: bool,
) -> EscapeOutcome {
    if tool_active && gesture_discardable {
        EscapeOutcome::CancelGesture
    } else if tool_active {
        EscapeOutcome::ExitTool
    } else if canvas_selection_nonempty {
        EscapeOutcome::ClearCanvasSelection
    } else {
        EscapeOutcome::FallThroughToRailClear
    }
}

// ---------------------------------------------------------------------------
// Hit-test / selection scaffold with a pluggable target provider (spec §4)
// ---------------------------------------------------------------------------

/// Opaque handle to a hit-testable thing on a page — a vector object
/// (Pass 9a), eventually a Bézier node (Pass 9c-min), eventually a placed
/// dimension (Pass 12.M2).
///
/// The substrate never interprets this value; it only stores it in a
/// selection set and hands it back to the SAME provider for bounds/details.
/// The concrete representation is deliberately the provider's/Pass 9a's
/// call — a `u64` here is the minimal placeholder the substrate needs to
/// order and compare targets; the substrate never MINTS a `TargetId`, so
/// its meaning stays entirely the provider's business.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[allow(
    dead_code,
    reason = "minted by Pass 9a's provider and by tests; the substrate only stores/compares TargetIds, never constructs them (spec 4.1)" // ui-text-exempt: clippy lint justification, never displayed
)]
pub struct TargetId(pub u64);

/// The seam a hit-testable content model plugs into (spec §4.1).
///
/// Defined HERE, in `pdfce-gui` — see the module docs' GUI-core-separation
/// note. All geometry the provider exchanges with the substrate is in
/// **canvas space** (`viewer`'s Y-down/rotated device convention), NOT PDF
/// user space; the provider owns its internal PDF-space object geometry and
/// converts via `viewer::pdf_space_to_canvas`/`canvas_to_pdf_space` as
/// needed.
pub trait CanvasTargetProvider {
    /// The topmost/nearest target at a canvas-space `point`, or `None` for a
    /// miss.
    fn hit_test(&self, page_index: usize, point: Pos2) -> Option<TargetId>;

    /// Every target enclosed by a canvas-space marquee `rect`. Whether
    /// enclosure means fully or partially contained is the provider's call
    /// (matching that Inkscape itself treats this as a documented,
    /// non-obvious convention); the substrate does not dictate one.
    #[allow(
        dead_code,
        reason = "the marquee half of selection; wired by Pass 9a once marquee-vs-pan is decided (spec 4.2), exercised via the stub provider now" // ui-text-exempt: clippy lint justification, never displayed
    )]
    fn hit_test_rect(&self, page_index: usize, rect: Rect) -> Vec<TargetId>;

    /// The target's own canvas-space bounds, for drawing a selection
    /// outline — `None` if the target no longer exists (a stale `TargetId`
    /// after the document changed underneath it; the substrate silently
    /// drops it from the selection, never panics — spec §4.4).
    fn bounds(&self, page_index: usize, target: TargetId) -> Option<Rect>;
}

/// The shippable no-op provider: hits nothing, encloses nothing, has no
/// bounds. This is what makes "the selection scaffold selects nothing until
/// a real provider is attached" hold by construction — Pass 9a replaces it
/// with an adapter over `pdfce-core`'s object model.
#[derive(Debug, Default, Clone, Copy)]
pub struct EmptyTargetProvider;

impl CanvasTargetProvider for EmptyTargetProvider {
    fn hit_test(&self, _page_index: usize, _point: Pos2) -> Option<TargetId> {
        None
    }

    fn hit_test_rect(&self, _page_index: usize, _rect: Rect) -> Vec<TargetId> {
        Vec::new()
    }

    fn bounds(&self, _page_index: usize, _target: TargetId) -> Option<Rect> {
        None
    }
}

/// The selection set after a plain/`Shift` **click** resolved to `hit`
/// (spec §4.2), computed purely so every branch is testable today with
/// fabricated `TargetId`s and no live provider.
///
/// - Plain click, hit: **replace** the selection with the one target.
/// - Plain click, miss: **clear** (clicking empty canvas deselects).
/// - `Shift`+click, hit: **toggle** that target's membership, leaving the
///   rest of the selection alone.
/// - `Shift`+click, miss: unchanged (there is nothing to toggle).
#[must_use]
pub fn selection_after_click(
    current: &BTreeSet<TargetId>,
    hit: Option<TargetId>,
    shift: bool,
) -> BTreeSet<TargetId> {
    match (shift, hit) {
        (false, Some(target)) => BTreeSet::from([target]),
        (false, None) => BTreeSet::new(),
        (true, Some(target)) => {
            let mut next = current.clone();
            if !next.insert(target) {
                next.remove(&target);
            }
            next
        }
        (true, None) => current.clone(),
    }
}

/// The selection set after a **marquee** enclosed `hits` (spec §4.2).
///
/// Plain marquee **replaces** the selection with the hit set; a `Shift`-held
/// marquee **adds** the hit set to the existing selection. Built and tested
/// now, but deliberately NOT wired to a live drag this Pass: a
/// drag-starting-on-empty-canvas is ambiguous between pan and marquee-select
/// in exactly the way a tool-active drag is ambiguous between pan and draw,
/// and resolving that ambiguity is explicitly Pass 9a's decision (spec §4.2)
/// — so this function exists and is proven correct in isolation, ready for
/// Pass 9a to wire once it decides the disambiguation.
#[must_use]
#[allow(
    dead_code,
    reason = "wired by Pass 9a once marquee-vs-pan disambiguation is decided (spec 4.2); proven in isolation now" // ui-text-exempt: clippy lint justification, never displayed
)]
pub fn selection_after_marquee(
    current: &BTreeSet<TargetId>,
    hits: &[TargetId],
    shift: bool,
) -> BTreeSet<TargetId> {
    if shift {
        let mut next = current.clone();
        next.extend(hits.iter().copied());
        next
    } else {
        hits.iter().copied().collect()
    }
}

/// The canvas-space outline rects to stroke for the current selection
/// (spec §4.3) — the testable seam of the live-preview overlay's
/// selection-outline half.
///
/// One rect per still-existing selected target's [`CanvasTargetProvider::
/// bounds`]; a target the provider no longer knows is silently skipped. With
/// an empty selection (this Pass's only real state) the result is empty and
/// the overlay strokes nothing — the "paints nothing this Pass" acceptance
/// criterion, verified by the empty-selection test below rather than by
/// omitting the call.
#[must_use]
pub fn selection_outline_bounds(
    selection: &BTreeSet<TargetId>,
    provider: Option<&dyn CanvasTargetProvider>,
    page_index: usize,
) -> Vec<Rect> {
    let Some(provider) = provider else {
        return Vec::new();
    };
    selection
        .iter()
        .filter_map(|&target| provider.bounds(page_index, target))
        .collect()
}

/// The selection set with every target the provider can no longer resolve
/// removed (spec §4.4) — run on every `refresh_pages` so an edit that
/// invalidates a selected `TargetId` (Undo, Redo, a future move/delete)
/// leaves no dangling entry. Mirrors `OpenDoc::clamp_selection`'s existing
/// "drop any stale index" precedent, applied to canvas targets. Silent: an
/// edit that deselects something it also changed is not a fact the operator
/// needs disclosed.
#[must_use]
pub fn prune_selection(
    selection: &BTreeSet<TargetId>,
    mut is_valid: impl FnMut(TargetId) -> bool,
) -> BTreeSet<TargetId> {
    selection
        .iter()
        .copied()
        .filter(|&target| is_valid(target))
        .collect()
}

// ---------------------------------------------------------------------------
// Text-edit caret/selection state machine (Pass 14.3, spec §3/§4.2/§4.4)
// ---------------------------------------------------------------------------
//
// The text-edit tool's caret/selection transitions are pure functions over
// `pdfce_core::text_edit` positions, unit-tested here exactly as the object-
// selection transitions above are — so the interaction logic is proven
// without a live egui frame (the reason viewer.rs and this module exist:
// `main.rs` is not headlessly testable, these are). `main.rs` calls these
// with the hit position it computed via `viewer::canvas_to_pdf_space` →
// `EditableTextModel::hit_test`; nothing here touches egui or the model.

/// The `(caret, anchor)` after a click resolved to `hit` while the text-edit
/// tool is active (spec §3 item 2–3, §4.2).
///
/// Text selection is a contiguous `(anchor, active)` caret span, so Shift
/// **extends** from the existing caret — a deliberate, named divergence from
/// [`selection_after_click`]'s object-set **toggle** semantics (spec §4.2):
/// discrete objects have membership, a text span has only two ends.
///
/// - Plain click, hit: caret = hit, anchor cleared (a plain click collapses
///   any selection).
/// - Plain click, miss: both cleared (clicking empty canvas deselects — the
///   same convention Pass 12.0 established for objects, applied to text).
/// - `Shift`+click, hit: caret = hit; anchor = the existing anchor if a
///   selection was already open, else the pre-existing caret (the span's new
///   fixed end), else the hit itself (a Shift+click with no prior caret is a
///   collapsed caret, not an error).
/// - `Shift`+click, miss: unchanged (nothing to extend to).
#[must_use]
pub fn text_caret_after_click(
    current_caret: Option<TextPosition>,
    current_anchor: Option<TextPosition>,
    hit: Option<TextPosition>,
    shift: bool,
) -> (Option<TextPosition>, Option<TextPosition>) {
    match (shift, hit) {
        (false, Some(pos)) => (Some(pos), None),
        (false, None) => (None, None),
        (true, Some(pos)) => {
            let anchor = current_anchor.or(current_caret).or(Some(pos));
            (Some(pos), anchor)
        }
        (true, None) => (current_caret, current_anchor),
    }
}

/// Whether the glyphs a selection covers touch more than one distinct run —
/// the client-side cross-run refusal check (spec §4.4).
///
/// `EditRequest`/`FormatRequest` anchor to ONE show operator, so a selection
/// whose `resolve_range` result spans >1 `GlyphRef.run` cannot be committed
/// as a single edit under the shipped core API. The GUI checks this cheaply
/// on every selection-extending frame and refuses BEFORE a doomed core call
/// (disabling Accept, suppressing `PendingEdit` creation, showing a
/// refusal-with-reason) rather than silently clamping or attempting a call
/// that would only refuse anyway.
#[must_use]
pub fn selection_spans_multiple_runs(covered: &[GlyphRef]) -> bool {
    let mut first: Option<usize> = None;
    for g in covered {
        match first {
            None => first = Some(g.run),
            Some(run) if run != g.run => return true,
            Some(_) => {}
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Selection-replace-on-type (Pass 14.4, spec §6.1)
// ---------------------------------------------------------------------------
//
// These two are the model-FREE half of Pass 14.4: pure string / position
// arithmetic with no `EditableTextModel`, so they are unit-tested here in
// `pdfce-gui`. The model-DEPENDENT completions (Left/Right/Up/Down/Home/End
// caret navigation) live as `EditableTextModel` accessors in `pdfce-core`
// instead — `PageText`/`TextRun`/`ExtractedGlyph` are `#[non_exhaustive]` and
// cannot be constructed in a `pdfce-gui` test, so core is the only place those
// can be headless-tested (see `model.rs`'s "Caret navigation geometry" note).

/// The single-run byte range a selection covers, as `(run, lo, hi)` with
/// `lo < hi`, or `None` when there is no editable selection: a collapsed caret,
/// a missing anchor, or a caret/anchor in DIFFERENT runs (a cross-run
/// selection — refused separately by [`selection_spans_multiple_runs`] and
/// never type-replaced, spec §4.4). Pass 14.4 selection-replace-on-type spine.
///
/// Because a [`TextPosition`] is `(run, byte_offset)`, a caret and anchor in
/// the SAME run always bound a single-run span — so this is the exact predicate
/// the GUI uses to decide "typing REPLACES the selection" versus "typing
/// inserts at the caret." Order-insensitive in the two ends.
#[must_use]
pub fn single_run_selection_range(
    caret: Option<TextPosition>,
    anchor: Option<TextPosition>,
) -> Option<(usize, usize, usize)> {
    let (c, a) = (caret?, anchor?);
    if c.run != a.run || c.byte_offset == a.byte_offset {
        return None;
    }
    let lo = c.byte_offset.min(a.byte_offset);
    let hi = c.byte_offset.max(a.byte_offset);
    Some((c.run, lo, hi))
}

/// The draft text and cursor after typing `typed` over the byte range
/// `[lo, hi)` of `original` (Pass 14.4 selection-replace-on-type /
/// selection-delete, spec §6.1). Removes the span and inserts `typed` at its
/// start; the returned cursor lands immediately after the insertion
/// (`lo + typed.len()`). Passing `typed == ""` expresses a bare selection
/// DELETE (Backspace/Delete over a selection): the span is removed and the
/// cursor rests at `lo`.
///
/// Panic-free by the crate's checked-access posture: `lo`/`hi` are clamped into
/// range and, if somehow off a char boundary, snapped OUTWARD to the enclosing
/// boundaries (callers always pass glyph boundaries, which are char boundaries,
/// so this is a safety net, not a normal path). The font-on-edit gate is NOT
/// applied here — this only shapes the draft string; a replacement character
/// the run's font cannot provide is still refused-and-disclosed at Accept time
/// by `EditSession::edit_text`, exactly as Pass 14.1 specifies.
#[must_use]
pub fn selection_after_type(original: &str, lo: usize, hi: usize, typed: &str) -> (String, usize) {
    let len = original.len();
    let mut lo = lo.min(len);
    let mut hi = hi.min(len);
    if lo > hi {
        std::mem::swap(&mut lo, &mut hi);
    }
    while lo > 0 && !original.is_char_boundary(lo) {
        lo -= 1;
    }
    while hi < len && !original.is_char_boundary(hi) {
        hi += 1;
    }
    let mut out = String::with_capacity(len - (hi - lo) + typed.len());
    out.push_str(&original[..lo]);
    out.push_str(typed);
    out.push_str(&original[hi..]);
    (out, lo + typed.len())
}

// ---------------------------------------------------------------------------
// Fuzzy snap indicator — GUI-side logic + rendering primitives (Pass 12.M1)
// ---------------------------------------------------------------------------
//
// The snap MATH lives in `pdfce_core::vector::snap` (GUI-free). This half is
// the GUI's own two responsibilities the engine deliberately does NOT own
// (`snap.rs` module docs): (1) converting a fixed SCREEN-space tolerance into
// the page-space `SnapConfig::tolerance` via the current zoom — the
// zoom-invariance mechanism — and gating the query on the master toggle / Alt
// override, and (2) rendering the fuzzy indicator (a distinct marker glyph per
// snap kind + a type label) BEFORE the pick commits.
//
// Everything here is pure/testable now and, like `selection_after_marquee`
// and the `viewer` PDF-space bridges before it, is WIRED to the live measure
// tools in Pass 12.M2 (which own the tool-mode frame the indicator draws in).
// Building + unit-testing the logic against the announced contract now is the
// substrate's established "design against the contract, name the asks" idiom
// (the same way pass-14.3/16.2 built their not-yet-shipped siblings). The
// per-frame paint call is a one-liner 12.M2 adds in the measure-tool handler:
//   painter.extend(canvas::snap_marker_shapes(screen_at, kind, color, size));
//   painter.text(label_at, .., ui_text::snap_indicator_label(kind), ..);

/// The default screen-space snap catch radius, in egui logical points
/// (decision 011 §2.2: "≈8–12 px"). Converted to a page-space tolerance each
/// frame by [`screen_tolerance_to_page`] so the snap "feel" is zoom-invariant.
#[allow(
    dead_code,
    reason = "Pass 12.M1 snap default; consumed by the Pass 12.M2 measure tools that own the tool-mode frame (spec 2.4)" // ui-text-exempt: clippy lint justification, never displayed
)]
pub const SNAP_SCREEN_TOLERANCE_PX: f32 = 10.0;

/// Convert a fixed SCREEN-space pixel tolerance into a **page-space** snap
/// tolerance for `zoom` (device px per PDF user-space unit) — the
/// zoom-invariance mechanism (decision 011 §2.2; the page-space value
/// `snap_candidates` takes). A constant on-screen catch radius maps to a
/// *shrinking* page-space tolerance as the operator zooms in, so the "feel"
/// stays constant. This is the exact `/ zoom` distance law
/// [`crate::viewer::screen_to_page`] uses (proven zoom-invariant in `viewer`'s
/// `screen_to_page_distance_scales_as_one_over_zoom` test). A non-finite or
/// non-positive `zoom` yields `0.0` (snapping disabled) rather than a NaN/∞
/// tolerance the engine would reject anyway.
#[allow(
    dead_code,
    reason = "Pass 12.M1 zoom-invariance conversion; called by the Pass 12.M2 measure tools each frame (spec 2.4)" // ui-text-exempt: clippy lint justification, never displayed
)]
#[must_use]
pub fn screen_tolerance_to_page(screen_px: f32, zoom: f32) -> f64 {
    if zoom.is_finite() && zoom > 0.0 && screen_px.is_finite() && screen_px >= 0.0 {
        f64::from(screen_px) / f64::from(zoom)
    } else {
        0.0
    }
}

/// Whether a snap query should run for the current pick (ui-spec §2.4): the
/// persistent master "Snap to content" toggle is ON **and** the transient Alt
/// override is NOT held. With snapping disabled either way, the pick is the raw
/// pointer position — no candidates queried, no indicator drawn.
#[allow(
    dead_code,
    reason = "Pass 12.M1 master-toggle + Alt-override gate; consumed by the Pass 12.M2 measure tools (spec 2.4)" // ui-text-exempt: clippy lint justification, never displayed
)]
#[must_use]
pub fn snap_query_enabled(master_on: bool, alt_held: bool) -> bool {
    master_on && !alt_held
}

/// The Tab-cycle index after advancing over a candidate list of `len`
/// (ui-spec §2.4), wrapping to `0` past the end. `len == 0` stays `0` (nothing
/// to cycle). Index 0 is the engine's default pick (highest priority, nearest);
/// Tab steps through the tied/competing candidates the engine returned.
#[allow(
    dead_code,
    reason = "Pass 12.M1 Tab-cycle advance; driven by the Pass 12.M2 measure tools' key handling (spec 2.4)" // ui-text-exempt: clippy lint justification, never displayed
)]
#[must_use]
pub fn next_snap_index(current: usize, len: usize) -> usize {
    if len == 0 { 0 } else { (current + 1) % len }
}

/// The active snap candidate for a Tab-cycle index, wrapped into range
/// (ui-spec §2.4). Returns `None` for an empty list — no candidate within
/// tolerance, so the indicator is hidden and the pick is the raw pointer
/// position. A stale `cycle` past the list length wraps rather than panicking
/// (the list can shrink between frames as the pointer moves).
#[allow(
    dead_code,
    reason = "Pass 12.M1 active-candidate selection; read by the Pass 12.M2 measure tools each frame (spec 2.4)" // ui-text-exempt: clippy lint justification, never displayed
)]
#[must_use]
pub fn active_snap_candidate(cands: &[SnapCandidate], cycle: usize) -> Option<SnapCandidate> {
    if cands.is_empty() {
        None
    } else {
        Some(cands[cycle % cands.len()])
    }
}

/// How many clicks confirm a pick on a candidate of `kind` (ui-spec §2.3): TWO
/// for a derived centerline — the one fuzzy inference, where the first click
/// only *promotes* the candidate to "proposed" and a second confirms it (a
/// proportionate, non-modal two-click gate, never an auto-apply) — and ONE for
/// every routine kind, a deterministic geometry fact that commits on the single
/// pick. This is the fuzzy-never-sneaky gate (rule 4) encoded for the Pass 12.M2
/// pick handler; it reads `SnapKind::is_derived` so the policy lives in one place.
#[allow(
    dead_code,
    reason = "Pass 12.M1 two-click-confirm policy; enforced by the Pass 12.M2 measure-tool pick handler (spec 2.3)" // ui-text-exempt: clippy lint justification, never displayed
)]
#[must_use]
pub fn snap_commit_clicks(kind: SnapKind) -> u8 {
    if kind.is_derived() { 2 } else { 1 }
}

/// The egui shapes that draw the distinct marker glyph for a snap candidate of
/// `kind` at screen position `at` (ui-spec §2.2). **Shape distinguishes the
/// kind — colour is never the sole signal** (rule 6): a node is a filled
/// square, an endpoint a filled circle, a center a crosshair-in-circle, a
/// midpoint a triangle, an intersection a cross, a routine centerline a dashed
/// tick, an axis a grid glyph, and the DERIVED centerline a **hatched square**,
/// visually unmistakable from the routine centerline tick so the extra-confirm
/// candidate always reads differently (§2.3.1). `size` is the marker half-extent
/// in points; `color` tints every stroke/fill. The measure tool paints these
/// via the live-preview overlay painter (never a re-raster) and draws the label
/// text ([`crate::ui_text::snap_indicator_label`]) as a separate galley beside
/// them.
#[allow(
    dead_code,
    reason = "Pass 12.M1 indicator rendering primitive; painted by the Pass 12.M2 measure tools' overlay (spec 2.2)" // ui-text-exempt: clippy lint justification, never displayed
)]
#[must_use]
pub fn snap_marker_shapes(at: Pos2, kind: SnapKind, color: Color32, size: f32) -> Vec<Shape> {
    let s = size.max(1.0);
    let stroke = Stroke::new(1.5, color);
    let sq = |half: f32| -> Vec<Pos2> {
        vec![
            Pos2::new(at.x - half, at.y - half),
            Pos2::new(at.x + half, at.y - half),
            Pos2::new(at.x + half, at.y + half),
            Pos2::new(at.x - half, at.y + half),
        ]
    };
    match kind {
        SnapKind::Node => {
            // ◼ filled square.
            vec![Shape::convex_polygon(sq(s), color, Stroke::NONE)]
        }
        SnapKind::Endpoint => {
            // ● filled circle.
            vec![Shape::circle_filled(at, s, color)]
        }
        SnapKind::Center => {
            // ⊕ crosshair in a circle.
            vec![
                Shape::circle_stroke(at, s, stroke),
                Shape::line_segment(
                    [Pos2::new(at.x - s, at.y), Pos2::new(at.x + s, at.y)],
                    stroke,
                ),
                Shape::line_segment(
                    [Pos2::new(at.x, at.y - s), Pos2::new(at.x, at.y + s)],
                    stroke,
                ),
            ]
        }
        SnapKind::Midpoint => {
            // ▲ up-pointing triangle.
            let tri = vec![
                Pos2::new(at.x, at.y - s),
                Pos2::new(at.x + s, at.y + s),
                Pos2::new(at.x - s, at.y + s),
            ];
            vec![Shape::convex_polygon(tri, color, Stroke::NONE)]
        }
        SnapKind::Intersection => {
            // ✕ diagonal cross.
            vec![
                Shape::line_segment(
                    [Pos2::new(at.x - s, at.y - s), Pos2::new(at.x + s, at.y + s)],
                    stroke,
                ),
                Shape::line_segment(
                    [Pos2::new(at.x - s, at.y + s), Pos2::new(at.x + s, at.y - s)],
                    stroke,
                ),
            ]
        }
        SnapKind::SegmentCenterline => {
            // ┄ dashed tick: two short colinear dashes.
            vec![
                Shape::line_segment(
                    [Pos2::new(at.x - s, at.y), Pos2::new(at.x - s * 0.25, at.y)],
                    stroke,
                ),
                Shape::line_segment(
                    [Pos2::new(at.x + s * 0.25, at.y), Pos2::new(at.x + s, at.y)],
                    stroke,
                ),
            ]
        }
        SnapKind::DerivedCenterline => {
            // ▤ hatched square — a square OUTLINE plus two diagonal hatch
            // lines, deliberately distinct from the routine centerline tick so
            // the extra-confirm candidate is unmistakable (§2.3.1).
            vec![
                Shape::convex_polygon(sq(s), Color32::TRANSPARENT, stroke),
                Shape::line_segment(
                    [Pos2::new(at.x - s, at.y + s), Pos2::new(at.x + s, at.y - s)],
                    stroke,
                ),
                Shape::line_segment(
                    [Pos2::new(at.x - s, at.y), Pos2::new(at.x, at.y - s)],
                    stroke,
                ),
            ]
        }
        SnapKind::Axis => {
            // ⊞ grid glyph: a square outline crossed by one H and one V line.
            vec![
                Shape::convex_polygon(sq(s), Color32::TRANSPARENT, stroke),
                Shape::line_segment(
                    [Pos2::new(at.x - s, at.y), Pos2::new(at.x + s, at.y)],
                    stroke,
                ),
                Shape::line_segment(
                    [Pos2::new(at.x, at.y - s), Pos2::new(at.x, at.y + s)],
                    stroke,
                ),
            ]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- pan suppression (spec §1.3) ----------------------------------

    #[test]
    fn pan_is_suppressed_only_when_a_tool_is_active_without_a_narrow_rect() {
        // Every branch, with plain bool/Option inputs — no CanvasTool
        // variant needed, because the logic never inspects one.
        assert!(!canvas_suppresses_pan(false, None)); // this Pass's only real state
        assert!(canvas_suppresses_pan(true, None)); // Pass 6.1 whole-canvas
        let rect = Rect::from_min_size(Pos2::ZERO, eframe::egui::vec2(10.0, 10.0));
        assert!(!canvas_suppresses_pan(true, Some(rect))); // Pass 7 defers to its rect
        assert!(!canvas_suppresses_pan(false, Some(rect)));
    }

    // ---- Escape precedence (spec §3.5) --------------------------------

    #[test]
    fn escape_precedence_is_gesture_then_tool_then_selection_then_rail() {
        // Priority 1: a discardable gesture wins over everything.
        assert_eq!(
            resolve_escape(true, true, true),
            EscapeOutcome::CancelGesture
        );
        // Priority 2: tool active, no gesture → exit the tool.
        assert_eq!(resolve_escape(true, false, true), EscapeOutcome::ExitTool);
        // Priority 3: no tool, non-empty canvas selection → clear it.
        assert_eq!(
            resolve_escape(false, false, true),
            EscapeOutcome::ClearCanvasSelection
        );
        // Priority 4: nothing else → the existing rail-clear (this Pass's
        // only live outcome).
        assert_eq!(
            resolve_escape(false, false, false),
            EscapeOutcome::FallThroughToRailClear
        );
        // A discardable-gesture flag with no active tool is meaningless and
        // must not fire CancelGesture.
        assert_eq!(
            resolve_escape(false, true, false),
            EscapeOutcome::FallThroughToRailClear
        );
    }

    // ---- selection model (spec §4.2) ----------------------------------

    fn ids(raw: &[u64]) -> BTreeSet<TargetId> {
        raw.iter().copied().map(TargetId).collect()
    }

    #[test]
    fn plain_click_replaces_on_hit_and_clears_on_miss() {
        let current = ids(&[1, 2]);
        assert_eq!(
            selection_after_click(&current, Some(TargetId(9)), false),
            ids(&[9])
        );
        assert_eq!(
            selection_after_click(&current, None, false),
            BTreeSet::new()
        );
    }

    #[test]
    fn shift_click_toggles_membership_and_leaves_the_rest_alone() {
        let current = ids(&[1, 2]);
        // Absent target is added.
        assert_eq!(
            selection_after_click(&current, Some(TargetId(3)), true),
            ids(&[1, 2, 3])
        );
        // Present target is removed.
        assert_eq!(
            selection_after_click(&current, Some(TargetId(2)), true),
            ids(&[1])
        );
        // Shift+miss is a no-op.
        assert_eq!(selection_after_click(&current, None, true), current);
    }

    #[test]
    fn marquee_replaces_plain_and_adds_with_shift() {
        let current = ids(&[1, 2]);
        assert_eq!(
            selection_after_marquee(&current, &[TargetId(5), TargetId(6)], false),
            ids(&[5, 6])
        );
        assert_eq!(
            selection_after_marquee(&current, &[TargetId(2), TargetId(3)], true),
            ids(&[1, 2, 3])
        );
    }

    #[test]
    fn prune_drops_targets_the_provider_no_longer_resolves() {
        let current = ids(&[1, 2, 3]);
        let pruned = prune_selection(&current, |t| t != TargetId(2));
        assert_eq!(pruned, ids(&[1, 3]));
    }

    // ---- provider + overlay (spec §4.1, §4.3) -------------------------

    /// A single-page stub provider over a fixed set of canvas-space bounds,
    /// proving the trait and the overlay seam work before Pass 9a's real
    /// implementation exists.
    struct StubProvider {
        boxes: Vec<(TargetId, Rect)>,
    }

    impl CanvasTargetProvider for StubProvider {
        fn hit_test(&self, _page_index: usize, point: Pos2) -> Option<TargetId> {
            self.boxes
                .iter()
                .find(|(_, r)| r.contains(point))
                .map(|(id, _)| *id)
        }

        fn hit_test_rect(&self, _page_index: usize, rect: Rect) -> Vec<TargetId> {
            self.boxes
                .iter()
                .filter(|(_, r)| rect.intersects(*r))
                .map(|(id, _)| *id)
                .collect()
        }

        fn bounds(&self, _page_index: usize, target: TargetId) -> Option<Rect> {
            self.boxes
                .iter()
                .find(|(id, _)| *id == target)
                .map(|(_, r)| *r)
        }
    }

    fn r(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(Pos2::new(x, y), eframe::egui::vec2(w, h))
    }

    #[test]
    fn empty_target_provider_never_hits_encloses_or_bounds() {
        let p = EmptyTargetProvider;
        assert_eq!(p.hit_test(0, Pos2::new(5.0, 5.0)), None);
        assert!(p.hit_test_rect(0, r(0.0, 0.0, 100.0, 100.0)).is_empty());
        assert_eq!(p.bounds(0, TargetId(1)), None);
    }

    #[test]
    fn overlay_bounds_are_empty_for_an_empty_selection() {
        // The "paints nothing this Pass" criterion: an empty selection
        // yields no outline rects, with a real provider AND with none.
        let provider = StubProvider {
            boxes: vec![(TargetId(1), r(0.0, 0.0, 10.0, 10.0))],
        };
        assert!(selection_outline_bounds(&BTreeSet::new(), Some(&provider), 0).is_empty());
        assert!(selection_outline_bounds(&BTreeSet::new(), None, 0).is_empty());
    }

    #[test]
    fn overlay_bounds_project_each_still_existing_target() {
        let provider = StubProvider {
            boxes: vec![
                (TargetId(1), r(0.0, 0.0, 10.0, 10.0)),
                (TargetId(2), r(20.0, 20.0, 5.0, 5.0)),
            ],
        };
        // A selection with one live and one stale target yields exactly one
        // rect — the stale one is silently skipped (spec §4.4 posture).
        let selection = ids(&[2, 99]);
        let rects = selection_outline_bounds(&selection, Some(&provider), 0);
        assert_eq!(rects, vec![r(20.0, 20.0, 5.0, 5.0)]);
    }

    // ---- text-edit caret/selection (Pass 14.3, spec §3/§4.2/§4.4) -----

    fn tp(run: usize, off: usize) -> TextPosition {
        TextPosition::new(run, off)
    }

    #[test]
    fn plain_click_sets_caret_and_collapses_selection() {
        // Hit: caret = hit, anchor cleared. Miss: both cleared.
        assert_eq!(
            text_caret_after_click(Some(tp(0, 0)), Some(tp(0, 3)), Some(tp(1, 2)), false),
            (Some(tp(1, 2)), None)
        );
        assert_eq!(
            text_caret_after_click(Some(tp(0, 0)), None, None, false),
            (None, None)
        );
    }

    #[test]
    fn shift_click_extends_the_span_rather_than_toggling() {
        // With an open selection, Shift keeps the existing anchor and moves
        // the active end — never toggles membership (the object-selection rule).
        assert_eq!(
            text_caret_after_click(Some(tp(0, 5)), Some(tp(0, 2)), Some(tp(0, 8)), true),
            (Some(tp(0, 8)), Some(tp(0, 2)))
        );
        // With only a caret, that caret becomes the anchor.
        assert_eq!(
            text_caret_after_click(Some(tp(0, 5)), None, Some(tp(0, 8)), true),
            (Some(tp(0, 8)), Some(tp(0, 5)))
        );
        // With nothing prior, the hit anchors itself (collapsed, not an error).
        assert_eq!(
            text_caret_after_click(None, None, Some(tp(0, 8)), true),
            (Some(tp(0, 8)), Some(tp(0, 8)))
        );
        // Shift+miss is a no-op.
        assert_eq!(
            text_caret_after_click(Some(tp(0, 5)), Some(tp(0, 2)), None, true),
            (Some(tp(0, 5)), Some(tp(0, 2)))
        );
    }

    #[test]
    fn cross_run_detection_flags_only_multi_run_spans() {
        let one_run = [
            GlyphRef::new(3, 0),
            GlyphRef::new(3, 1),
            GlyphRef::new(3, 5),
        ];
        assert!(!selection_spans_multiple_runs(&one_run));
        assert!(!selection_spans_multiple_runs(&[]));
        assert!(!selection_spans_multiple_runs(&[GlyphRef::new(7, 0)]));
        let two_runs = [GlyphRef::new(3, 9), GlyphRef::new(4, 0)];
        assert!(selection_spans_multiple_runs(&two_runs));
    }

    // ---- selection-replace-on-type (Pass 14.4, spec §6.1) -------------

    #[test]
    fn single_run_selection_range_only_for_a_real_same_run_span() {
        // A real same-run span (order-insensitive) yields (run, lo, hi).
        assert_eq!(
            single_run_selection_range(Some(tp(2, 7)), Some(tp(2, 3))),
            Some((2, 3, 7))
        );
        assert_eq!(
            single_run_selection_range(Some(tp(2, 3)), Some(tp(2, 7))),
            Some((2, 3, 7))
        );
        // A collapsed caret (equal offsets) is no selection.
        assert_eq!(
            single_run_selection_range(Some(tp(2, 3)), Some(tp(2, 3))),
            None
        );
        // A missing anchor is no selection.
        assert_eq!(single_run_selection_range(Some(tp(2, 3)), None), None);
        // A cross-run span is refused here (edited one run at a time, §4.4).
        assert_eq!(
            single_run_selection_range(Some(tp(2, 3)), Some(tp(3, 0))),
            None
        );
    }

    #[test]
    fn selection_after_type_replaces_and_deletes_a_span() {
        // Replace "cat" (bytes [4,7)) in "the cat" with "dog".
        let (draft, cur) = selection_after_type("the cat", 4, 7, "dog");
        assert_eq!(draft, "the dog");
        assert_eq!(cur, 7); // just after the inserted "dog"
        // A bare delete (typed == "") removes the span, cursor at lo.
        let (draft, cur) = selection_after_type("the cat", 4, 7, "");
        assert_eq!(draft, "the ");
        assert_eq!(cur, 4);
        // Order-insensitivity: swapped bounds behave identically.
        assert_eq!(
            selection_after_type("the cat", 7, 4, "dog"),
            ("the dog".to_string(), 7)
        );
    }

    #[test]
    fn selection_after_type_is_panic_free_and_boundary_snapping() {
        // Out-of-range bounds clamp to the string length (no panic).
        let (draft, cur) = selection_after_type("hi", 5, 9, "X");
        assert_eq!(draft, "hiX");
        assert_eq!(cur, 3);
        // An offset INSIDE a multi-byte char (byte 2 splits "é", which occupies
        // bytes [1,3)) snaps OUTWARD to the enclosing boundaries rather than
        // panicking on a slice: [2,2) becomes [1,3), removing the whole "é".
        let (draft, cur) = selection_after_type("aéb", 2, 2, "");
        assert_eq!(draft, "ab");
        assert_eq!(cur, 1);
    }

    #[test]
    fn stub_hit_test_and_marquee_feed_the_selection_functions() {
        // hit_test_rect proven in isolation against a stub marquee rect
        // (spec §4.2 — built and tested, never wired to a live drag).
        let provider = StubProvider {
            boxes: vec![
                (TargetId(1), r(0.0, 0.0, 10.0, 10.0)),
                (TargetId(2), r(100.0, 100.0, 10.0, 10.0)),
            ],
        };
        assert_eq!(provider.hit_test(0, Pos2::new(5.0, 5.0)), Some(TargetId(1)));
        assert_eq!(provider.hit_test(0, Pos2::new(50.0, 50.0)), None);
        let enclosed = provider.hit_test_rect(0, r(0.0, 0.0, 30.0, 30.0));
        assert_eq!(enclosed, vec![TargetId(1)]);
        assert_eq!(
            selection_after_marquee(&BTreeSet::new(), &enclosed, false),
            ids(&[1])
        );
    }

    // ---- measure-tool predicates (Pass 12.M2 ui-spec §1.1) ------------

    #[test]
    fn measure_tool_predicates_are_mutually_exclusive_with_the_text_tools() {
        // The three measure tools are recognised, and none of the text-tool
        // predicates fire for them (single-value `Option<CanvasTool>` invariant).
        for tool in [
            CanvasTool::MeasureLinear,
            CanvasTool::MeasureCircular,
            CanvasTool::MeasureScale,
        ] {
            assert!(tool.is_measure());
            assert!(tool_builds_measure(Some(tool)));
            assert!(!tool_builds_text_edit(Some(tool)));
            assert!(!tool_builds_add_text(Some(tool)));
        }
        assert!(tool_builds_measure_linear(Some(CanvasTool::MeasureLinear)));
        assert!(tool_builds_measure_circular(Some(
            CanvasTool::MeasureCircular
        )));
        assert!(tool_builds_measure_scale(Some(CanvasTool::MeasureScale)));
        // The text tools are not measure tools.
        assert!(!CanvasTool::TextEdit.is_measure());
        assert!(!tool_builds_measure(Some(CanvasTool::AddText)));
        assert!(!tool_builds_measure(None));
    }

    // ---- add-text placement resolution (Pass 16.2 §3.2) ---------------

    #[test]
    fn a_real_drag_resolves_to_a_normalized_box_in_any_direction() {
        // Top-right → bottom-left and bottom-left → top-right both name the
        // SAME box: lower-left (10,20), 100×80.
        let expect = AddTextPlacement::Box {
            llx: 10.0,
            lly: 20.0,
            width: 100.0,
            height: 80.0,
        };
        assert_eq!(
            resolve_drag_placement((10.0, 20.0), (110.0, 100.0), 4.0),
            expect
        );
        assert_eq!(
            resolve_drag_placement((110.0, 100.0), (10.0, 20.0), 4.0),
            expect
        );
    }

    #[test]
    fn a_degenerate_drag_falls_back_to_a_point_at_the_drag_start() {
        // Below the floor in height → point at the START (not the min corner,
        // not discarded — the deliberate divergence from Pass 6.1).
        assert_eq!(
            resolve_drag_placement((72.0, 700.0), (180.0, 701.0), 12.0),
            AddTextPlacement::Point { x: 72.0, y: 700.0 }
        );
        // Below the floor in width → point at the START.
        assert_eq!(
            resolve_drag_placement((72.0, 700.0), (73.0, 500.0), 12.0),
            AddTextPlacement::Point { x: 72.0, y: 700.0 }
        );
        // A pure click (start == end) is the extreme degenerate case → point.
        assert_eq!(
            resolve_drag_placement((72.0, 700.0), (72.0, 700.0), 12.0),
            AddTextPlacement::Point { x: 72.0, y: 700.0 }
        );
        // A non-finite drag routes to the point fallback, never a NaN box.
        assert_eq!(
            resolve_drag_placement((72.0, 700.0), (f64::NAN, 500.0), 12.0),
            AddTextPlacement::Point { x: 72.0, y: 700.0 }
        );
    }

    #[test]
    fn the_two_tools_states_are_mutually_exclusive() {
        // For every possible active_tool, TextEdit and AddText are never both
        // built — the §0.1 invariant a single Option<CanvasTool> guarantees.
        for tool in [None, Some(CanvasTool::TextEdit), Some(CanvasTool::AddText)] {
            assert!(!(tool_builds_text_edit(tool) && tool_builds_add_text(tool)));
        }
        assert!(tool_builds_text_edit(Some(CanvasTool::TextEdit)));
        assert!(tool_builds_add_text(Some(CanvasTool::AddText)));
        assert!(!tool_builds_text_edit(Some(CanvasTool::AddText)));
        assert!(!tool_builds_add_text(Some(CanvasTool::TextEdit)));
        assert!(!tool_builds_text_edit(None));
        assert!(!tool_builds_add_text(None));
    }

    // ---- snap indicator logic + rendering (Pass 12.M1) ----------------

    #[test]
    fn screen_tolerance_converts_inversely_with_zoom() {
        // A fixed 10px catch radius is 10 page units at 100%, 5 at 200%, 20 at
        // 50% — the zoom-invariance the snap "feel" depends on.
        assert_eq!(screen_tolerance_to_page(10.0, 1.0), 10.0);
        assert_eq!(screen_tolerance_to_page(10.0, 2.0), 5.0);
        assert_eq!(screen_tolerance_to_page(10.0, 0.5), 20.0);
        // Degenerate zoom disables snapping (0 tolerance, which the engine
        // rejects) rather than yielding a NaN/inf.
        assert_eq!(screen_tolerance_to_page(10.0, 0.0), 0.0);
        assert_eq!(screen_tolerance_to_page(10.0, f32::NAN), 0.0);
    }

    #[test]
    fn snap_is_enabled_only_with_master_on_and_alt_up() {
        assert!(snap_query_enabled(true, false));
        assert!(!snap_query_enabled(false, false)); // master toggle off
        assert!(!snap_query_enabled(true, true)); // Alt transiently suppresses
        assert!(!snap_query_enabled(false, true));
    }

    #[test]
    fn tab_cycle_wraps_and_handles_empty() {
        assert_eq!(next_snap_index(0, 3), 1);
        assert_eq!(next_snap_index(2, 3), 0); // wraps past the end
        assert_eq!(next_snap_index(0, 0), 0); // nothing to cycle
        assert_eq!(next_snap_index(5, 0), 0);
    }

    #[test]
    fn active_candidate_indexes_and_wraps() {
        let c = |k| SnapCandidate {
            point: pdfce_core::vector::Point::new(0.0, 0.0),
            kind: k,
            source_object: None,
        };
        let list = [c(SnapKind::Node), c(SnapKind::Midpoint)];
        assert_eq!(
            active_snap_candidate(&list, 0).unwrap().kind,
            SnapKind::Node
        );
        assert_eq!(
            active_snap_candidate(&list, 1).unwrap().kind,
            SnapKind::Midpoint
        );
        // A stale index past the end wraps (3 % 2 == 1) rather than panicking.
        assert_eq!(
            active_snap_candidate(&list, 3).unwrap().kind,
            SnapKind::Midpoint
        );
        assert!(active_snap_candidate(&[], 0).is_none());
    }

    #[test]
    fn derived_centerline_needs_two_clicks_others_one() {
        // The fuzzy-never-sneaky gate: the derived centerline confirms in two
        // clicks; every deterministic kind commits on one.
        assert_eq!(snap_commit_clicks(SnapKind::DerivedCenterline), 2);
        assert_eq!(snap_commit_clicks(SnapKind::Node), 1);
        assert_eq!(snap_commit_clicks(SnapKind::SegmentCenterline), 1);
    }

    #[test]
    fn every_snap_kind_has_a_non_empty_marker_and_the_derived_one_is_distinct() {
        let kinds = [
            SnapKind::Node,
            SnapKind::Endpoint,
            SnapKind::Center,
            SnapKind::Midpoint,
            SnapKind::Intersection,
            SnapKind::DerivedCenterline,
            SnapKind::SegmentCenterline,
            SnapKind::Axis,
        ];
        for k in kinds {
            assert!(!snap_marker_shapes(Pos2::new(10.0, 10.0), k, Color32::RED, 4.0).is_empty());
        }
        // The derived centerline's glyph must not be visually confused with the
        // routine centerline tick (§2.3.1) — here proven by a different shape
        // composition (a hatched square vs. two dashes).
        let derived =
            snap_marker_shapes(Pos2::ZERO, SnapKind::DerivedCenterline, Color32::RED, 4.0);
        let routine =
            snap_marker_shapes(Pos2::ZERO, SnapKind::SegmentCenterline, Color32::RED, 4.0);
        assert_ne!(derived.len(), routine.len());
    }
}
