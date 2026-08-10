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

use eframe::egui::{Color32, PointerButton, Pos2, Rect, Response, Shape, Stroke};
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
/// `Ord`/`Hash` are derived so a `BTreeSet<CanvasTool>` can hold the SET of
/// enabled tools (2026-08-06, the operator's independent-toggles ruling). The
/// derived order is declaration order and is **not** the dispatch precedence —
/// that lives in `OpenDoc::TOOL_PRECEDENCE`, stated explicitly there so it can
/// be argued about rather than accidentally inherited from where a variant
/// happens to sit in this enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    /// Place a new interactive-form field (decision 020's F5): click → a
    /// type-dependent default box, drag → an explicit box, then name it and
    /// Accept in the Tool Options pane.
    ///
    /// # Why a tool at all, when Pass 7's form FILLING deliberately is not one
    ///
    /// The comment above says Pass 7 "adds NONE — it needs no tool mode at
    /// all", and `RibbonGroup::Forms`'s own doc comment gives filling's whole
    /// reason for being its own group: it never touches canvas gesture state.
    /// Creating a field is the opposite kind of act. It is a rectangle placed
    /// on the page by a pointer gesture, so it must own what a click MEANS,
    /// which is exactly what a `CanvasTool` is for.
    ///
    /// Same argument that made [`CanvasTool::AddText`] a separate tool from
    /// [`CanvasTool::TextEdit`] rather than a sub-mode: a plain click's
    /// existing meaning must not be silently repurposed into "sometimes
    /// creates a form field."
    ///
    /// # One tool for four field types, not four tools
    ///
    /// The TYPE (text / check box / radio / choice) is a control in the Tool
    /// Options pane, not a separate armed tool. Four tools would put four
    /// entries on [`OpenDoc::TOOL_PRECEDENCE`] that are mutually exclusive in
    /// practice but expressible as simultaneous — a set of enabled tools where
    /// three of the four combinations mean nothing coherent. One tool with a
    /// type selector also matches how the field is actually placed: arm once,
    /// place several.
    PlaceField,
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

// ---------------------------------------------------------------------------
// RETIRED — `tool_strip_anchor` / `StripCorner` (Pass 34.1 slice 4)
// ---------------------------------------------------------------------------
//
// The three tools each drew two floating `egui::Area`s: a property bar and a
// status/commit strip. This function decided where they hung.
//
// Its own docs recorded the defect that created it. The strips were originally
// positioned at `image_rect.min.x + 8, image_rect.max.y - 8` — a function of
// the PAGE's drawn rectangle — so the Accept/Reject controls moved on every
// zoom step, every scroll, every page change, and at high zoom left the
// viewport entirely. The operator, 2026-08-04: *"there is a separate accept /
// reject box somewhere on the screen to click - I've never seen any other
// software operate that way."* The word doing the work there is SOMEWHERE.
//
// Decision 024 answered it by anchoring to the VIEWPORT instead of the page, so
// the strips stopped moving. That was the right fix for the position and left
// the category alone: they still floated over the drawing.
//
// Pass 34.1 finished the job the operator actually asked for — *"all of the
// options should be shown in a side bar tab docked with the page navigation
// tab"* — by moving all six into `DockPanel::ToolOptions`. Slice 3 emptied the
// top-left corner and removed `StripCorner::TopLeft`; slice 4 emptied the
// bottom-left and removes the rest.
//
// Deleted rather than kept "in case something floats again". A helper for
// hanging things off canvas corners is an invitation to hang something off a
// canvas corner, and the whole point of the dock is that tool controls have
// one fixed home. If a genuinely transient overlay is ever needed, it should
// be designed as one, not inherited from the mechanism this replaced.

/// Which object the operator has **entered**, and which of its subpaths is
/// selected inside it.
///
/// A PDF path object can hold an entire drawing view — one measured CAD export
/// has 1194 subpaths in a single object — so "the object under the pointer" is
/// often not the thing the operator means. Entering an object switches
/// selection to the level below it, the way entering a group does in a vector
/// editor (R61: Inkscape as a behavioural reference, never a code source).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnteredObject {
    /// Paint-order index of the object that was entered.
    pub object: usize,
    /// The selected subpath within it, if any. `None` means "inside this
    /// object, nothing picked yet" — a real state, reached by entering an
    /// object at a point where no subpath is close enough.
    pub subpath: Option<usize>,
    /// The selected anchor within `subpath`, if the operator has descended one
    /// rung further — the **Node** rung (decision 025's `k+3`, designed in
    /// decision 028).
    ///
    /// # The index is OBJECT-scoped, not subpath-scoped
    ///
    /// Decision 025 §1.3(b) settled this and it is load-bearing rather than a
    /// convention: it is the space `vector::anchor_count` reports and the space
    /// `pdfce-cli node-move --node N` addresses. A subpath-scoped index would
    /// be a second numbering for the same points, and the number pdfce shows
    /// the operator would then disagree with the number they can act on.
    ///
    /// `Some` implies `subpath.is_some()` — there is no way to select a point
    /// without being inside the part that holds it. Nothing in the type
    /// enforces that; [`depth_after_click`] is the only constructor that
    /// matters and it maintains it.
    ///
    /// # Why this exists as a rung rather than as a free gesture
    ///
    /// A node drag was ALREADY reachable before this field existed, from any
    /// selected path object, against that object's whole flat anchor list — up
    /// to 6,681 anchors on one measured CAD object, with nothing drawn before
    /// the grab to say which one was about to move. Decision 028 calls that an
    /// R83 hazard already in production and requires the rung to REPLACE it,
    /// not sit beside it: two routes to the same edit, one of them invisible,
    /// is the failure decision 025 §2.1 diagnosed for descent.
    pub node: Option<usize>,
}

/// What a canvas click does to the selection **depth**.
///
/// Separated from `selection_and_cycle_after_click`, which decides WHICH
/// objects are selected, because depth is an orthogonal question: the same
/// click can change one, the other, or both. Keeping them apart is also what
/// lets this be tested exhaustively with plain `Option<usize>` inputs, with no
/// document, no geometry and no egui frame.
///
/// # The rules, and why each one
///
/// - **Double-click on an object → enter it**, selecting the nearest subpath.
///   This is the operator's own stated model ("double-click to get to the next
///   level down") and the vector-editor convention.
/// - **Single click while inside, on a subpath → re-pick** that subpath. Once
///   you are inside, ordinary clicking works at that level; requiring a
///   double-click for every subsequent pick would make the level feel modal in
///   a way no editor is.
/// - **Single click while inside, hitting NO subpath → leave.** Clicking away
///   is how every editor exits a group. The alternative — staying inside
///   forever until Escape — strands an operator who has forgotten they
///   descended, which is the failure mode a depth model has to avoid above all.
/// - **Double-click on empty space → leave**, for the same reason; a
///   double-click is also a click.
/// - **Entering a DIFFERENT object** replaces the entry rather than nesting.
///   PDF path objects do not nest, so a stack would be depth for its own sake.
/// - **Double-click while at the Subpath rung → enter the Node rung**, per
///   decision 025's table row `k+2`. The anchors of the entered subpath
///   become selectable, and `node_hit` picks one.
/// - **Single click at the Node rung → re-pick a point**; a click that misses
///   every point but lands on a part ascends one rung and re-picks that part.
///   That is the table's `k+3` "ascend to Subpath and re-pick".
/// - **Double-click at the Node rung does nothing** — there is nothing below a
///   point. The state is returned unchanged; disclosing that to the operator
///   is the caller's job (023 §1.3 disclosure 2 — reported, never a silent
///   no-op).
///
/// # One deliberate deviation from 025's table
///
/// The table says a click outside at the Node rung ascends "to Object". This
/// returns `None` (leave entirely), matching what the Subpath rung has always
/// done for the same gesture. Making the two rungs disagree about what
/// clicking-nothing means would be a worse inconsistency than the deviation,
/// and the full `LevelPath`/`Rung` type 025 §3.2 specifies — which gives
/// "Object rung" its own representable state instead of encoding it as
/// `Some { subpath: None }` — is where that is properly resolved.
#[must_use]
pub fn depth_after_click(
    entered: Option<EnteredObject>,
    double: bool,
    object_hit: Option<usize>,
    subpath_hit: Option<usize>,
    node_hit: Option<usize>,
) -> Option<EnteredObject> {
    match (entered, double) {
        // ---- At the NODE rung ------------------------------------------
        // Nothing is below a point, so a double-click changes no depth.
        (Some(e), true) if e.node.is_some() => Some(e),
        // Ordinary click: re-pick a point, else fall back one rung onto a
        // part, else leave.
        (Some(e), false) if e.node.is_some() => match (node_hit, subpath_hit) {
            (Some(n), _) => Some(EnteredObject { node: Some(n), ..e }),
            // Missed every point but landed on a part: ascend one rung and
            // re-pick, rather than stranding the operator at a rung whose
            // targets they keep missing.
            (None, Some(sp)) => Some(EnteredObject {
                object: e.object,
                subpath: Some(sp),
                node: None,
            }),
            (None, None) => None,
        },

        // ---- At the SUBPATH rung, double-click: descend to NODE ---------
        // Only when the click stayed inside the SAME object. A double-click on
        // a different object is an entry into that object (below), not a
        // descent — PDF path objects do not nest, so carrying a node index
        // across would address a point in a different anchor space.
        (Some(e), true) if e.subpath.is_some() && object_hit == Some(e.object) => {
            Some(EnteredObject {
                object: e.object,
                // Descend into the part under the pointer if there is one,
                // else the part already selected.
                subpath: subpath_hit.or(e.subpath),
                node: node_hit,
            })
        }

        // Already inside an object, ordinary click: re-pick within it, or
        // leave if the click landed on nothing in it.
        (Some(e), false) => subpath_hit.map(|sp| EnteredObject {
            object: e.object,
            subpath: Some(sp),
            node: None,
        }),
        // Double-click: enter whatever object is under the pointer (possibly a
        // different one), or leave if there is none.
        (_, true) => object_hit.map(|object| EnteredObject {
            object,
            subpath: subpath_hit,
            node: None,
        }),
        // Not inside anything, ordinary click: nothing to do at this level.
        (None, false) => None,
    }
}

/// The scroll offset a middle-drag pan should move to, clamped to what the
/// canvas can actually show.
///
/// # Why the clamp is not optional
///
/// The offset is subtracted, so the content follows the hand. Without a clamp
/// an unscrollable canvas — the page fitted inside the viewport, offset pinned
/// at zero — still accepts a negative target for one frame, so the page slides
/// with the pointer and then snaps back the instant the drag ends. Observed
/// exactly that on 2026-08-04: a 50 px slide and a 50 px jump back. Refusing to
/// move at all is the honest response to "there is nothing to pan to".
///
/// # Known limitation, deliberately left
///
/// This clamps to the PAGE, so the page edges cannot be dragged inward past the
/// viewport edge. The operator asked to "navigate beyond the page's edges",
/// which needs reserved space around the page rather than a different clamp —
/// a change to how the canvas reserves its content area, with a visible
/// consequence (scrollbars present at every zoom). That is a UX call, referred
/// to `pdfce-ui-specialist` rather than decided here, and this function is the
/// one place it would need to change.
#[must_use]
pub fn pan_offset(
    last: (f32, f32),
    pan: (f32, f32),
    display: (f32, f32),
    viewport: (f32, f32),
) -> (f32, f32) {
    fn axis(last: f32, pan: f32, d: f32, v: f32) -> f32 {
        if !(last.is_finite() && pan.is_finite() && d.is_finite() && v.is_finite()) {
            return last;
        }
        (last - pan).clamp(0.0, (d - v).max(0.0))
    }
    (
        axis(last.0, pan.0, display.0, viewport.0),
        axis(last.1, pan.1, display.1, viewport.1),
    )
}

/// Whether a canvas gesture is starting — **primary button only**.
///
/// # Why every canvas gesture must ask this instead of `drag_started()`
///
/// `Response::drag_started()` is button-agnostic: it is true for a middle-drag
/// and a right-drag as well as a left one. That was harmless while those two
/// buttons did nothing on the canvas. Once the middle button became the pan
/// gesture it stopped being harmless — a middle-drag over a selected object
/// would have been read by the object-edit tool as "move this object", so
/// panning across a drawing would silently rewrite it.
///
/// The right button is reserved for the operator's requested context menus and
/// is excluded for the same reason, before it can acquire the same defect.
///
/// These three helpers exist so the constraint is stated once with its reason,
/// rather than as a dozen scattered `..._by(PointerButton::Primary)` calls that
/// a later gesture is free to forget.
#[must_use]
pub fn primary_drag_started(r: &Response) -> bool {
    r.drag_started_by(PointerButton::Primary)
}

/// Whether a canvas gesture is in flight — primary button only. See
/// [`primary_drag_started`].
#[must_use]
pub fn primary_dragged(r: &Response) -> bool {
    r.dragged_by(PointerButton::Primary)
}

/// Whether a canvas gesture just ended — primary button only. See
/// [`primary_drag_started`].
///
/// A gesture that STARTS on the primary button can only stop on it, so in
/// practice this differs from `drag_stopped()` only for a drag that was never
/// ours to begin with — which is exactly the case that must not commit an edit.
#[must_use]
pub fn primary_drag_stopped(r: &Response) -> bool {
    r.drag_stopped_by(PointerButton::Primary)
}

/// Where the canvas must be scrolled to so the page point under the pointer
/// stays under the pointer across a zoom step — "zoom to cursor".
///
/// # Why this exists
///
/// Ctrl+wheel previously called `zoom_by` and nothing else. The scroll offset
/// was left alone, so the *viewport centre* was the fixed point of the zoom and
/// whatever the operator was pointing at slid away — worse the further from
/// centre they were pointing, which is exactly where a person zooms in on a
/// drawing detail. Every other application that zooms a canvas (browsers, CAD,
/// Inkscape, Office) anchors on the cursor, and the operator reported ours as
/// "jarring" on 2026-08-04 for that reason.
///
/// # The geometry
///
/// The page is drawn at `display` pixels inside a scroll-area content box of
/// `outer = max(display, viewport)` — the `max` is what lets the area still
/// scroll when the page is bigger AND centre the page when it is smaller (see
/// the reservation comment in `main.rs`). So the page's top-left sits at
/// `margin = (outer - display) / 2` in content coordinates, and a point at
/// fraction `anchor_frac` of the page appears on screen at
///
/// ```text
///     screen = viewport_origin + margin + anchor_frac * display - offset
/// ```
///
/// Holding `screen` fixed across the step and solving for the new offset gives
///
/// ```text
///     offset₁ = offset₀ + anchor_frac * (display₁ - display₀) + (margin₁ - margin₀)
/// ```
///
/// which needs no knowledge of where the viewport is on screen — only sizes.
/// The margin term is not a refinement: while the page is smaller than the
/// viewport the offset is pinned at zero and *all* of the movement is the
/// margin shrinking, so dropping it would make zoom-to-cursor do nothing at
/// precisely the "fit page" zoom an operator starts from.
///
/// # Contract
///
/// - `anchor_frac` is the pointer's position as a fraction of the page's drawn
///   size, `(pointer - page_top_left) / display₀`. Values outside `0..=1` are
///   meaningful (the pointer may be in the centring margin) and are not clamped.
/// - The result is clamped to the scrollable range `0 ..= max(0, display₁ -
///   viewport)`, so a caller may hand it straight to `ScrollArea::scroll_offset`
///   without producing an offset the area would fight back against.
/// - Non-finite inputs yield `offset_before` unchanged: refusing to move is the
///   only safe answer, since a NaN offset would blank the canvas.
#[must_use]
pub fn zoom_anchor_offset(
    offset_before: (f32, f32),
    display_before: (f32, f32),
    display_after: (f32, f32),
    viewport: (f32, f32),
    anchor_frac: (f32, f32),
) -> (f32, f32) {
    /// Centring margin on one axis: half the slack when the page is smaller
    /// than the viewport, zero once it is larger.
    fn margin(display: f32, viewport: f32) -> f32 {
        (display.max(viewport) - display) / 2.0
    }

    /// One axis of the solve above, plus the scrollable-range clamp.
    fn axis(off0: f32, d0: f32, d1: f32, v: f32, u: f32) -> f32 {
        let off1 = off0 + u * (d1 - d0) + (margin(d1, v) - margin(d0, v));
        off1.clamp(0.0, (d1 - v).max(0.0))
    }

    let finite = [
        offset_before.0,
        offset_before.1,
        display_before.0,
        display_before.1,
        display_after.0,
        display_after.1,
        viewport.0,
        viewport.1,
        anchor_frac.0,
        anchor_frac.1,
    ]
    .iter()
    .all(|f| f.is_finite());
    if !finite {
        return offset_before;
    }

    (
        axis(
            offset_before.0,
            display_before.0,
            display_after.0,
            viewport.0,
            anchor_frac.0,
        ),
        axis(
            offset_before.1,
            display_before.1,
            display_after.1,
            viewport.1,
            anchor_frac.1,
        ),
    )
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
    /// Leave the object the operator has entered, returning to the level
    /// above it — WITHOUT clearing the selection or exiting the tool
    /// (Pass 28.0). One press, one rung.
    LeaveLevel,
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
    inside_object: bool,
) -> EscapeOutcome {
    if tool_active && gesture_discardable {
        EscapeOutcome::CancelGesture
    } else if inside_object {
        // Pass 28.0, decision 025's L1: one Escape pops ONE rung, never
        // several. Placed ABOVE `ExitTool` deliberately — with the object tool
        // armed and the operator two levels inside a drawing, Escape used to
        // exit the tool outright, discarding the level AND the tool in a single
        // press, which is the collapse decision 025 named.
        //
        // Pass 25.1 shipped Escape as "clear everything", which had the same
        // effect from the other direction: leaving a part also left the object,
        // so an operator who descended two rungs to reach a line and pressed
        // Escape once found themselves back at the page.
        EscapeOutcome::LeaveLevel
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
    ///
    /// `tolerance` is the canvas-space slack the click may miss an object's
    /// edge by. It is a **parameter, not a provider constant**, because the
    /// only honest source for it is the live zoom: the pointer has already
    /// been divided by `zoom` on its way here
    /// ([`crate::viewer::screen_to_page`]), so a tolerance fixed in canvas
    /// units would shrink on screen as the operator zooms out — at "Fit
    /// page" it collapses to under two pixels and thin geometry becomes
    /// effectively unclickable. Callers pass
    /// [`screen_tolerance_to_page`]`(`[`SELECT_SCREEN_TOLERANCE_PX`]`, zoom)`
    /// so the catch radius is a constant number of SCREEN pixels at every
    /// zoom — the same zoom-invariance law the snap engine already uses.
    ///
    /// **A provided method, not a required one.** It is defined as the head
    /// of [`Self::hit_test_all`], which is what makes "the first click and
    /// the cycling clicks agree about what is under the pointer" a
    /// structural property rather than a convention two implementations have
    /// to keep. An implementor supplies only the all-hits query.
    fn hit_test(&self, page_index: usize, point: Pos2, tolerance: f64) -> Option<TargetId> {
        self.hit_test_all(page_index, point, tolerance)
            .first()
            .copied()
    }

    /// **Every** target at a canvas-space `point` within `tolerance`,
    /// **topmost/front-most first**.
    ///
    /// The required half of the point query (see [`Self::hit_test`] for why
    /// the topmost one is derived from this rather than the reverse), and
    /// the input to click-through cycling: an object entirely covered by
    /// another can only ever be selected by stepping past the cover, and
    /// with a topmost-only query no click can do that. ui-spec
    /// `pass-17-dock-and-layer-tree.md` §C.3.
    ///
    /// Empty for a miss and for a query on another page.
    fn hit_test_all(&self, page_index: usize, point: Pos2, tolerance: f64) -> Vec<TargetId>;

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
    fn hit_test_all(&self, _page_index: usize, _point: Pos2, _tolerance: f64) -> Vec<TargetId> {
        Vec::new()
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

/// How far, in canvas units, the pointer may drift between two clicks and
/// still count as "the same point" for click-through cycling.
///
/// **A cycle that never resets is a trap**, and so is one that resets on a
/// one-pixel tremor. This is the tolerance between those failures: a mouse
/// held still while double-clicking wanders a pixel or two, so 4.0 absorbs
/// hand shake; a deliberate move to a different object is far larger than
/// that, and lands the operator back at "select the topmost thing here",
/// which is what an unmodified click has always done.
///
/// Canvas units rather than screen pixels: the recorded point is canvas
/// space, so comparing there costs no zoom bookkeeping. The consequence —
/// the on-screen slack grows as the operator zooms in — is the *safe*
/// direction, because zoomed in, the pointer covers less document per pixel
/// and two clicks a pixel apart are even more obviously meant to be the
/// same point.
pub const CYCLE_SAME_POINT_CANVAS: f32 = 4.0;

/// Where a click-through cycle currently stands (ui-spec §C.3).
///
/// One click at one point can mean two different things — *select what is
/// here* and *select what is UNDER what is here* — and the difference is
/// history, so it has to be state. This is the whole of that state.
///
/// ## What resets it, and why each reset exists
///
/// A cycle that outlives its context is worse than no cycle: the operator
/// clicks expecting the topmost object and gets the third one down, with no
/// way to tell why. So a cycle is *derived-live*, checked on every use by
/// [`Self::continues`], and stale state simply stops applying:
///
/// | Reset trigger | Why |
/// |---|---|
/// | The pointer moved more than [`CYCLE_SAME_POINT_CANVAS`] | A different point is a different question. |
/// | The page changed | `TargetId`s are per-page indices; the same number means a different object. |
/// | The selection is no longer exactly the object this cycle produced | Something else selected — a tree row, a marquee, Escape, an edit's prune. The operator's mental "current object" moved, so the cycle's position is meaningless. |
/// | The hit list no longer contains that object | The document changed under it. |
///
/// The tool changing needs no rule of its own: every tool switch either
/// clears the selection or leaves it, and in the second case the operator
/// clicking the same point with a different tool active is still asking
/// about the same stack — the cycle staying live is the friendly answer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClickCycle {
    /// The page the cycle is anchored to.
    pub page_index: usize,
    /// The canvas-space point the cycle is anchored to.
    pub point: Pos2,
    /// 0-based position in the front-to-back hit list that the LAST click
    /// resolved to.
    pub ordinal: usize,
    /// How many objects were under the pointer at that click — the `3` in
    /// "2 of 3 at this point".
    pub total: usize,
    /// The target that click selected. The cycle only continues while this
    /// is still exactly the selection (table above).
    pub produced: TargetId,
}

impl ClickCycle {
    /// Whether this cycle still applies to a click at `point` on
    /// `page_index` with `current` selected.
    #[must_use]
    pub fn continues(self, page_index: usize, point: Pos2, current: &BTreeSet<TargetId>) -> bool {
        self.page_index == page_index
            && self.point.distance(point) <= CYCLE_SAME_POINT_CANVAS
            && current.len() == 1
            && current.contains(&self.produced)
    }

    /// The 1-based position for display ("**2** of 3 at this point").
    #[must_use]
    pub fn position(self) -> usize {
        self.ordinal.saturating_add(1)
    }

    /// Whether this cycle is the live description of `current` on
    /// `page_index` — the guard the status readout applies before printing
    /// "n of m at this point" beside a selection.
    ///
    /// Deliberately NOT position-dependent: the readout describes the
    /// selection, and the selection does not move when the pointer does. A
    /// cycle whose object is still the one selected still truthfully says
    /// how many objects were stacked where that selection came from.
    #[must_use]
    pub fn describes(self, page_index: usize, current: &BTreeSet<TargetId>) -> bool {
        self.page_index == page_index && current.len() == 1 && current.contains(&self.produced)
    }
}

/// Resolve a canvas click into a new selection **and** the cycle state that
/// click leaves behind (ui-spec §C.3).
///
/// The one place click-through cycling is decided, shared by every canvas
/// click path so a plain-selection click and an object-edit-tool click
/// cannot cycle differently. Pure — no egui, no provider — so every branch
/// below is unit-testable with fabricated hit lists.
///
/// `hits` is the provider's front-to-back list at the point
/// ([`CanvasTargetProvider::hit_test_all`]).
///
/// ## The rules
///
/// - **Miss** (empty `hits`): exactly [`selection_after_click`]'s miss —
///   plain click clears, `Shift`+click leaves the selection alone — and the
///   cycle is dropped. There is no stack here to step through.
/// - **`Shift`+click**: toggles the TOPMOST hit, unchanged from before, and
///   drops the cycle. Additive selection and cycling are different
///   questions, and a `Shift` that both added an object and advanced a
///   hidden cursor would be unpredictable.
/// - **`Alt`+click** (`alt`): steps the cycle. The starting position is the
///   next one after wherever the selection currently sits:
///   - a live cycle (see [`ClickCycle::continues`]) advances from its own
///     ordinal, wrapping at the end back to the topmost;
///   - otherwise, if the current selection is exactly one object that IS in
///     the hit list, the step starts from *there* — so the natural gesture
///     "click to select, then Alt+click to go deeper" works without the
///     first click having been an Alt+click;
///   - otherwise it starts at the topmost, i.e. an Alt+click into a stack
///     with nothing selected behaves like a plain click.
/// - **Plain click**: selects the topmost hit — completely unchanged
///   behaviour — but RECORDS the cycle at ordinal 0. That record is what
///   lets the status readout disclose "1 of 3 at this point" on an ordinary
///   click, which is how the operator learns there is anything to cycle
///   through at all (rule 4: the capability is disclosed, not hidden behind
///   a modifier nobody discovers).
#[must_use]
pub fn selection_and_cycle_after_click(
    hits: &[TargetId],
    current: &BTreeSet<TargetId>,
    cycle: Option<ClickCycle>,
    page_index: usize,
    point: Pos2,
    shift: bool,
    alt: bool,
) -> (BTreeSet<TargetId>, Option<ClickCycle>) {
    let Some(&topmost) = hits.first() else {
        return (selection_after_click(current, None, shift), None);
    };
    if shift {
        return (selection_after_click(current, Some(topmost), shift), None);
    }

    let resume = cycle
        .filter(|c| c.continues(page_index, point, current))
        .map(|c| c.ordinal)
        .or_else(|| {
            // No live cycle: start from wherever the current selection sits in
            // this stack, if it sits in it at all.
            (current.len() == 1)
                .then(|| hits.iter().position(|t| current.contains(t)))
                .flatten()
        });
    let ordinal = cycle_ordinal(alt, resume, hits.len());

    let target = hits.get(ordinal).copied().unwrap_or(topmost);
    (
        selection_after_click(current, Some(target), false),
        Some(ClickCycle {
            page_index,
            point,
            ordinal,
            total: hits.len(),
            produced: target,
        }),
    )
}

/// Which ordinal in a front-to-back hit stack a click resolves to.
///
/// The click-through rule, in one place: a plain click always takes the
/// front-most; an Alt+click steps one deeper, wrapping; and an Alt+click with
/// nowhere to resume from behaves as a plain click, because "step from
/// nothing" has no meaningful answer and silently landing somewhere in the
/// middle of a stack would be worse than landing on top.
///
/// `resume_from` is where the click is stepping FROM — a live cycle's ordinal,
/// or the position of the current selection within this stack.
///
/// Extracted because two levels of the selection ladder need identical
/// behaviour: objects, and the subpaths inside an entered object. Alt+click
/// meaning "the next one down" at one level and something subtly different at
/// the other is exactly the kind of divergence an operator experiences as the
/// application being inconsistent, and exactly what R92 warns duplicated
/// predicates drift into.
#[must_use]
pub fn cycle_ordinal(alt: bool, resume_from: Option<usize>, len: usize) -> usize {
    if !alt || len == 0 {
        return 0;
    }
    resume_from.map_or(0, |i| (i + 1) % len)
}

/// A live click-through cycle at the **subpath** level — the part-level twin
/// of [`ClickCycle`].
///
/// Separate from `ClickCycle` rather than generic over the target type,
/// because the two anchor to different things: an object cycle is anchored to
/// a page and a `TargetId`, a subpath cycle to the entered OBJECT and a plain
/// index. Forcing them into one type would need a page/object union that only
/// exists to satisfy the abstraction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SubpathCycle {
    /// The entered object the cycle belongs to.
    pub object: usize,
    /// The canvas-space point it is anchored to.
    pub point: Pos2,
    /// 0-based position in the nearest-first hit list the last click took.
    pub ordinal: usize,
    /// How many parts were under the pointer — the `5` in "part 2 of 5 here".
    pub total: usize,
    /// The subpath index that click selected; the cycle only continues while
    /// that is still what is selected.
    pub produced: usize,
}

impl SubpathCycle {
    /// Whether this cycle still applies to a click at `point` inside `object`
    /// with `current` selected.
    #[must_use]
    pub fn continues(self, object: usize, point: Pos2, current: Option<usize>) -> bool {
        self.object == object
            && self.point.distance(point) <= CYCLE_SAME_POINT_CANVAS
            && current == Some(self.produced)
    }

    /// The 1-based position for display ("part **2** of 5 here").
    #[must_use]
    pub fn position(self) -> usize {
        self.ordinal.saturating_add(1)
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
///
/// **Each rect is paired with the [`TargetId`] it came from.** The overlay
/// needs to know WHICH object each box belongs to, so it can pick a per-kind
/// treatment and draw a type badge (ui-spec §C.1) — a bare `Vec<Rect>` could
/// not be zipped back to the selection set, because the `filter_map` above
/// drops stale targets and so breaks positional correspondence. Returning the
/// pair is the only way to keep that association honest.
#[must_use]
pub fn selection_outline_bounds(
    selection: &BTreeSet<TargetId>,
    provider: Option<&dyn CanvasTargetProvider>,
    page_index: usize,
) -> Vec<(TargetId, Rect)> {
    let Some(provider) = provider else {
        return Vec::new();
    };
    selection
        .iter()
        .filter_map(|&target| Some((target, provider.bounds(page_index, target)?)))
        .collect()
}

/// The minimum on-screen extent, in egui logical points, that a selection
/// outline is guaranteed to have on each axis.
///
/// Sized to be unmistakably visible without materially misreporting where the
/// object is: at 6 pt a horizontal rule's outline reads as a thin band centred
/// on the rule, and the readout states the true size (`… — 200.0 × 0.0 pt …`)
/// so the enlargement can never be mistaken for the object's real extent.
pub const MIN_OUTLINE_EXTENT_PX: f32 = 6.0;

/// Grow a degenerate outline rect, about its own centre, until it is at least
/// `min_extent` on both axes — **the fix for a selection that is correct and
/// paints nothing.**
///
/// # The bug this closes
///
/// A horizontal rule (`100 200 m 300 200 l S`, which is the only object in
/// `fixtures/synthetic/dimension/linear-base.pdf`) has the page bbox
/// `100,200 → 300,200`: real, finite, and **exactly zero high**. It hit-tests
/// correctly, it selects correctly, it appears in the Objects panel — and its
/// outline rect has zero height, so `Painter::rect_stroke` with
/// `StrokeKind::Inside` has no interior band to fill and puts **nothing** on
/// the screen. The operator's click was right, the selection state was right,
/// and the feedback was a blank page: exactly the "a correct action with no
/// feedback is indistinguishable from a broken one" failure that
/// `draw_selection_outlines`' own doc comment was written about, in a second
/// guise.
///
/// # Why in SCREEN space, and why symmetric
///
/// Applied after the canvas→screen projection, so the guaranteed thickness is
/// a constant number of on-screen points at every zoom — the same
/// zoom-invariance discipline [`screen_tolerance_to_page`] applies to the
/// catch radius. Growing symmetrically about the centre keeps the band
/// straddling the rule rather than sitting to one side of it, so the outline
/// still says truthfully *the object is here*.
///
/// # Not a silent widening
///
/// Rule 4 (fuzzy, never sneaky) is satisfied by disclosure, not by declining
/// to draw: `object_summary::describe_object` emits
/// `ObjectNote::DegenerateBounds` for exactly these objects, and the status
/// readout says in words that the object is a rule and that its outline has
/// been thickened on screen. The picture is legible AND the truth is stated.
///
/// A non-finite rect is returned unchanged — there is no meaningful centre to
/// grow about, and a NaN box is a bug to leave visible upstream, not to repair
/// here.
#[must_use]
pub fn visible_outline_rect(rect: Rect, min_extent: f32) -> Rect {
    if !rect.min.x.is_finite()
        || !rect.min.y.is_finite()
        || !rect.max.x.is_finite()
        || !rect.max.y.is_finite()
        || !min_extent.is_finite()
        || min_extent <= 0.0
    {
        return rect;
    }
    // `Rect::from_two_pos` normalises a rect whose corners arrived in either
    // order — the canvas→screen projection includes a Y flip, so `min` is not
    // guaranteed to be the smaller corner by the time it gets here.
    let rect = Rect::from_two_pos(rect.min, rect.max);
    let grow = |lo: f32, hi: f32| -> (f32, f32) {
        let extent = hi - lo;
        if extent >= min_extent {
            return (lo, hi);
        }
        let pad = (min_extent - extent) / 2.0;
        (lo - pad, hi + pad)
    };
    let (x0, x1) = grow(rect.min.x, rect.max.x);
    let (y0, y1) = grow(rect.min.y, rect.max.y);
    Rect::from_min_max(Pos2::new(x0, y0), Pos2::new(x1, y1))
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

/// The screen-space catch radius for **object selection**, in egui logical
/// points, converted to a canvas/page-space tolerance each query by
/// [`screen_tolerance_to_page`].
///
/// Deliberately a sibling of [`SNAP_SCREEN_TOLERANCE_PX`] rather than the
/// same constant: snapping and selection answer different questions and are
/// allowed to drift apart. Selection is set slightly TIGHTER (6 px vs 10 px)
/// because a snap that grabs a nearby vertex is a helpful correction the
/// operator can see and cycle through, whereas a selection that grabs a
/// neighbouring object is a silent wrong answer — the failure modes are not
/// symmetric, so the tolerances should not be either.
///
/// The old behaviour this replaces was a fixed 3.0 **canvas-space** value,
/// which is `3.0 × zoom` pixels on screen: 3 px at 100%, 1.5 px at 50%,
/// 0.75 px at 25%. That is the bug this constant exists to close — a
/// constant screen radius means the click target feels identical at every
/// zoom level.
pub const SELECT_SCREEN_TOLERANCE_PX: f32 = 6.0;

/// The side length, in SCREEN pixels, of an unselected node mark at the Node
/// rung (decision 028 §Q1).
///
/// Screen-space, not page-space, for the same reason
/// [`SELECT_SCREEN_TOLERANCE_PX`] is — and here it is not merely consistency:
/// the mark and `NODE_GRAB_SCREEN_TOLERANCE_PX` must agree about where a point
/// is, or the operator aims at a square and grabs something else.
pub const NODE_MARK_PX: f32 = 6.0;

/// The side length of the SELECTED node mark — larger and filled, so selection
/// is carried by size and fill rather than by colour alone (R84).
pub const NODE_MARK_SELECTED_PX: f32 = 8.0;

/// The diameter, in SCREEN pixels, of an unselected Bézier handle mark.
///
/// Smaller than [`NODE_MARK_PX`] on purpose, and hit-tested FIRST for the same
/// reason: a handle sits closest to its node exactly when the curve is nearly
/// flat there, so if the node won that contest the handle would be
/// unreachable precisely when the operator most wants it — to pull a flat
/// segment into a curve (decision 028 §Q3).
pub const HANDLE_MARK_PX: f32 = 5.0;

/// The diameter of the SELECTED handle mark.
pub const HANDLE_MARK_SELECTED_PX: f32 = 7.0;

/// The grab radius, in SCREEN pixels, for a Bézier handle.
///
/// Deliberately tighter than [`NODE_GRAB_SCREEN_TOLERANCE_PX`](crate::vector_edit_tool::NODE_GRAB_SCREEN_TOLERANCE_PX):
/// handles are checked first, so a generous handle radius would start eating
/// presses meant for the node underneath. The asymmetry is the point — losing
/// a handle grab costs one retry, while stealing a node grab moves a point of
/// the drawing.
pub const HANDLE_GRAB_SCREEN_TOLERANCE_PX: f32 = 5.0;

/// The most anchors drawn for one entered subpath before the Node rung stops
/// drawing them and says so.
///
/// **Provisional (R86 — only looking proves it.)** The reasoning is recorded
/// so it can be revised rather than re-derived: decision 025's own measurement
/// puts the common case at ~6 anchors per part, and 300 six-pixel squares is
/// still legible on a typical canvas view, while the pathological case this
/// project actually has — 6,681 anchors in one object — is nowhere near
/// drawable. Above the ceiling nothing is drawn and the count is disclosed;
/// a silent first-N would let an operator believe a 1,200-point part has 300.
pub const MAX_DRAWN_NODES: usize = 300;

/// The side length, in SCREEN pixels, of a node mark belonging to a part the
/// operator has NOT selected — drawn only while the "show points" view option
/// is on (Pass 36.3).
///
/// Smaller than [`NODE_MARK_PX`] deliberately. These marks answer "where would
/// I aim if I wanted that other line", so they must be visible; they are not
/// aim-able targets right now, because a click at the Node rung picks within
/// the selected part. Size is the cue that separates the two populations, and
/// it is a cue that survives a colourblind operator and a greyscale screenshot
/// alike (R84) — both populations share the part-outline hue.
pub const NODE_MARK_OTHER_PART_PX: f32 = 4.0;

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

    /// **One Escape, one rung** (Pass 28.0, decision 025's L1).
    ///
    /// Being inside an object outranks both exit-tool and clear-selection. Two
    /// separate bugs collapse into this: with a tool armed, Escape used to exit
    /// the TOOL while the operator was two levels inside a drawing; with no
    /// tool, Pass 25.1's Escape cleared the entered object AND the selection in
    /// one press. Either way a single press dropped more than one rung.
    #[test]
    fn escape_leaves_the_entered_object_before_it_touches_the_tool_or_selection() {
        // Inside an object, tool armed: the LEVEL goes, the tool stays.
        assert_eq!(
            resolve_escape(true, false, true, true),
            EscapeOutcome::LeaveLevel
        );
        // Inside an object, no tool, selection present: the level still wins.
        assert_eq!(
            resolve_escape(false, false, true, true),
            EscapeOutcome::LeaveLevel
        );
        // An in-flight GESTURE still outranks it — a half-drawn dimension is
        // more transient than a navigation level, and losing it silently to a
        // level pop would be the worse trade.
        assert_eq!(
            resolve_escape(true, true, true, true),
            EscapeOutcome::CancelGesture
        );
        // And once out, the chain behaves exactly as before.
        assert_eq!(
            resolve_escape(false, false, true, false),
            EscapeOutcome::ClearCanvasSelection
        );
    }

    // ---- click-through ordinal ----------------------------------------

    #[test]
    fn a_plain_click_always_takes_the_front_most() {
        assert_eq!(cycle_ordinal(false, None, 5), 0);
        assert_eq!(
            cycle_ordinal(false, Some(3), 5),
            0,
            "a plain click must RESET the cycle, not continue it"
        );
    }

    #[test]
    fn alt_steps_one_deeper_and_wraps() {
        assert_eq!(cycle_ordinal(true, Some(0), 3), 1);
        assert_eq!(cycle_ordinal(true, Some(1), 3), 2);
        assert_eq!(cycle_ordinal(true, Some(2), 3), 0, "must wrap, not stick");
    }

    #[test]
    fn alt_with_nowhere_to_resume_from_behaves_as_a_plain_click() {
        // Landing somewhere in the middle of a stack the operator has not
        // stepped into would be worse than landing on top.
        assert_eq!(cycle_ordinal(true, None, 4), 0);
    }

    #[test]
    fn an_empty_stack_never_indexes() {
        assert_eq!(cycle_ordinal(true, Some(2), 0), 0);
        assert_eq!(cycle_ordinal(false, None, 0), 0);
    }

    #[test]
    fn a_subpath_cycle_only_continues_for_the_same_object_point_and_selection() {
        let c = SubpathCycle {
            object: 7,
            point: Pos2::new(100.0, 100.0),
            ordinal: 1,
            total: 3,
            produced: 42,
        };
        assert!(c.continues(7, Pos2::new(100.0, 100.0), Some(42)));
        assert!(
            !c.continues(8, Pos2::new(100.0, 100.0), Some(42)),
            "a different object is a different stack"
        );
        assert!(
            !c.continues(7, Pos2::new(400.0, 400.0), Some(42)),
            "a click somewhere else starts over"
        );
        assert!(
            !c.continues(7, Pos2::new(100.0, 100.0), Some(9)),
            "if the selection changed underneath, the cycle is stale"
        );
        assert_eq!(c.position(), 2, "1-based for display");
    }

    // ---- selection depth ----------------------------------------------

    /// Shorthand so the table below reads as rules rather than as struct
    /// literals.
    fn ent(object: usize, subpath: Option<usize>) -> Option<EnteredObject> {
        Some(EnteredObject {
            object,
            subpath,
            node: None,
        })
    }

    /// The same shorthand for the Node rung.
    fn nod(object: usize, subpath: usize, node: usize) -> Option<EnteredObject> {
        Some(EnteredObject {
            object,
            subpath: Some(subpath),
            node: Some(node),
        })
    }

    #[test]
    fn a_double_click_enters_the_object_under_the_pointer() {
        assert_eq!(
            depth_after_click(None, true, Some(5870), Some(667), None),
            ent(5870, Some(667)),
            "double-clicking a many-subpath object must descend into it"
        );
    }

    #[test]
    fn entering_where_no_subpath_is_close_still_enters() {
        // A real state: inside the object, nothing picked. Collapsing this to
        // "not entered" would make a double-click near the middle of a sparse
        // view silently do nothing.
        assert_eq!(
            depth_after_click(None, true, Some(3), None, None),
            ent(3, None)
        );
    }

    #[test]
    fn once_inside_an_ordinary_click_re_picks_within_the_same_object() {
        assert_eq!(
            depth_after_click(ent(5870, Some(667)), false, Some(5870), Some(12), None),
            ent(5870, Some(12)),
            "a second pick inside must not require a second double-click"
        );
    }

    // ---- the Node rung (decision 028) --------------------------------

    /// **Descend Subpath → Node.** A double-click inside the entered object
    /// selects a point.
    #[test]
    fn a_double_click_at_the_subpath_rung_descends_to_the_node_rung() {
        assert_eq!(
            depth_after_click(
                ent(5870, Some(667)),
                true,
                Some(5870),
                Some(667),
                Some(1204)
            ),
            nod(5870, 667, 1204),
            "double-clicking inside the entered part must reach its points"
        );
    }

    /// Descending into a DIFFERENT part of the same object picks a node of
    /// THAT part.
    ///
    /// The bug this rules out is carrying the old part forward while taking
    /// the new part's node index — which would select a point that is not on
    /// the part the operator is looking at.
    #[test]
    fn descending_into_a_different_part_takes_that_parts_node() {
        assert_eq!(
            depth_after_click(ent(5870, Some(667)), true, Some(5870), Some(12), Some(40)),
            nod(5870, 12, 40)
        );
    }

    /// **A double-click on a DIFFERENT object enters that object — it does not
    /// descend.**
    ///
    /// PDF path objects do not nest, so a node index means nothing across the
    /// boundary: anchor numbering is per-object. Carrying one over would
    /// address a point in a different space entirely.
    #[test]
    fn a_double_click_on_another_object_enters_it_rather_than_descending() {
        assert_eq!(
            depth_after_click(ent(5870, Some(667)), true, Some(11), Some(3), Some(99)),
            ent(11, Some(3)),
            "entering a different object must not carry a node index across"
        );
    }

    /// **Nothing is below a point.** A double-click at the Node rung is a
    /// no-op on depth, not a descent into nowhere and not an exit.
    ///
    /// Returned unchanged so the caller can SAY so (023 §1.3 disclosure 2 —
    /// reported, never a silent no-op).
    #[test]
    fn a_double_click_at_the_node_rung_changes_nothing() {
        assert_eq!(
            depth_after_click(nod(5870, 667, 1204), true, Some(5870), Some(667), Some(9)),
            nod(5870, 667, 1204),
            "there is nothing below a point, so depth must not move"
        );
    }

    /// Once at the Node rung, an ordinary click re-picks a point.
    #[test]
    fn an_ordinary_click_at_the_node_rung_re_picks_a_point() {
        assert_eq!(
            depth_after_click(
                nod(5870, 667, 1204),
                false,
                Some(5870),
                Some(667),
                Some(1205)
            ),
            nod(5870, 667, 1205),
            "picking a second point must not require a second double-click"
        );
    }

    /// **Missing every point but hitting a part ascends one rung**, rather
    /// than stranding the operator at a rung whose targets they keep missing.
    #[test]
    fn missing_every_point_but_hitting_a_part_ascends_one_rung() {
        assert_eq!(
            depth_after_click(nod(5870, 667, 1204), false, Some(5870), Some(12), None),
            ent(5870, Some(12)),
            "a missed point that lands on a part falls back to the part"
        );
    }

    /// Clicking nothing at the Node rung leaves, exactly as it does at the
    /// Subpath rung.
    ///
    /// A documented deviation from decision 025's table (which says "ascend to
    /// Object"): the two rungs disagreeing about what clicking-nothing means
    /// would be a worse inconsistency, and the `LevelPath` type 025 §3.2
    /// specifies is where "Object rung" gets its own representable state.
    #[test]
    fn clicking_nothing_at_the_node_rung_leaves() {
        assert_eq!(
            depth_after_click(nod(5870, 667, 1204), false, None, None, None),
            None
        );
    }

    /// **Re-picking a part clears the node.** Ascending must not leave a stale
    /// point selected on a part that no longer holds it — the index is
    /// object-scoped, so it would still resolve, to the wrong point.
    #[test]
    fn re_picking_a_part_clears_the_selected_point() {
        assert_eq!(
            depth_after_click(nod(5870, 667, 1204), false, Some(5870), Some(12), None)
                .and_then(|e| e.node),
            None
        );
    }

    /// Entering an object fresh never starts at the Node rung — descent is one
    /// rung per double-click, with no shortcut.
    #[test]
    fn entering_an_object_never_lands_directly_on_a_point() {
        assert_eq!(
            depth_after_click(None, true, Some(5870), Some(667), Some(1204)).and_then(|e| e.node),
            None,
            "the first double-click reaches parts, never points"
        );
    }

    #[test]
    fn clicking_nothing_while_inside_leaves_rather_than_stranding_the_operator() {
        assert_eq!(
            depth_after_click(ent(5870, Some(667)), false, None, None, None),
            None
        );
        // Even if some OTHER object is under the pointer: it is not a subpath
        // of the entered one, so the click is outside, and outside means out.
        assert_eq!(
            depth_after_click(ent(5870, Some(667)), false, Some(11), None, None),
            None
        );
    }

    #[test]
    fn a_double_click_on_a_different_object_replaces_rather_than_nests() {
        assert_eq!(
            depth_after_click(ent(5870, Some(667)), true, Some(42), Some(0), None),
            ent(42, Some(0)),
            "PDF path objects do not nest, so neither should the entry"
        );
    }

    #[test]
    fn a_double_click_on_empty_space_leaves() {
        assert_eq!(
            depth_after_click(ent(5870, Some(667)), true, None, None, None),
            None
        );
        assert_eq!(depth_after_click(None, true, None, None, None), None);
    }

    #[test]
    fn an_ordinary_click_at_object_level_changes_no_depth() {
        assert_eq!(depth_after_click(None, false, Some(7), Some(1), None), None);
    }

    // ---- middle-drag pan ----------------------------------------------

    #[test]
    fn panning_moves_the_content_opposite_the_offset_so_the_page_follows_the_hand() {
        // Page twice the viewport, so there is room to move.
        let out = pan_offset(
            (500.0, 500.0),
            (30.0, -20.0),
            (1600.0, 1600.0),
            (800.0, 800.0),
        );
        assert_eq!(
            out,
            (470.0, 520.0),
            "dragging right must DECREASE the offset, or the page moves against the hand"
        );
    }

    #[test]
    fn an_unscrollable_canvas_refuses_to_pan_rather_than_rubber_banding() {
        // The fit-page case: page smaller than the viewport, offset pinned.
        // Before the clamp this returned -50 and the page visibly slid, then
        // snapped back when the drag ended.
        let out = pan_offset((0.0, 0.0), (50.0, 50.0), (600.0, 600.0), (800.0, 800.0));
        assert_eq!(out, (0.0, 0.0));
    }

    #[test]
    fn panning_stops_at_the_far_edge() {
        let out = pan_offset(
            (700.0, 0.0),
            (-500.0, 0.0),
            (1000.0, 1000.0),
            (800.0, 800.0),
        );
        assert_eq!(out.0, 200.0, "must not scroll past the end of the page");
    }

    // ---- zoom to cursor -----------------------------------------------

    /// The whole point, stated as the invariant rather than as an offset:
    /// re-derive where the anchored page point lands on screen after the step
    /// and assert it has not moved.
    ///
    /// Screen position is `margin + frac * display - offset` relative to the
    /// viewport origin, which is the same expression the doc comment solves —
    /// so this checks the solve, not merely that the code agrees with itself
    /// about arithmetic (R93: assert the outcome, not the intent).
    fn anchored_screen_x(off: f32, d: f32, v: f32, u: f32) -> f32 {
        (d.max(v) - d) / 2.0 + u * d - off
    }

    #[test]
    fn the_point_under_the_cursor_stays_under_the_cursor() {
        // A page larger than the viewport, pointer three quarters across —
        // i.e. far from centre, where the old centre-anchored behaviour was
        // most visibly wrong.
        let (v, u) = (800.0_f32, 0.75_f32);
        let (d0, d1) = (1200.0_f32, 1800.0_f32); // a 1.5x zoom in
        let off0 = 300.0_f32;
        let before = anchored_screen_x(off0, d0, v, u);

        let off1 = zoom_anchor_offset((off0, 0.0), (d0, d0), (d1, d1), (v, v), (u, u)).0;
        let after = anchored_screen_x(off1, d1, v, u);

        assert!(
            (after - before).abs() < 0.01,
            "the anchored point moved {} px across the zoom (before {before}, after {after})",
            after - before
        );
    }

    #[test]
    fn zooming_in_from_fit_page_moves_the_view_even_though_the_offset_starts_pinned() {
        // The case the margin term exists for: at "fit page" the page is
        // SMALLER than the viewport, so offset is 0 and cannot go lower.
        // Zooming past the viewport must start scrolling toward the anchor.
        let (v, u) = (800.0_f32, 0.9_f32); // pointer near the right edge
        let (d0, d1) = (600.0_f32, 2000.0_f32);
        let off1 = zoom_anchor_offset((0.0, 0.0), (d0, d0), (d1, d1), (v, v), (u, u)).0;
        assert!(
            off1 > 0.0,
            "zooming in past the viewport with the pointer off-centre must scroll toward it, \
             got {off1}"
        );
        let before = anchored_screen_x(0.0, d0, v, u);
        let after = anchored_screen_x(off1, d1, v, u);
        assert!(
            (after - before).abs() < 0.01,
            "the anchored point moved {} px",
            after - before
        );
    }

    #[test]
    fn the_offset_never_leaves_the_scrollable_range() {
        // Zooming OUT far enough that the page no longer fills the viewport
        // must land at 0 rather than at a negative offset the scroll area
        // would silently fight.
        let out = zoom_anchor_offset(
            (900.0, 900.0),
            (2000.0, 2000.0),
            (400.0, 400.0),
            (800.0, 800.0),
            (0.1, 0.1),
        );
        assert_eq!(
            out,
            (0.0, 0.0),
            "zoomed-out offset must clamp to the origin"
        );

        // And never past the far edge.
        let (v, d1) = (800.0_f32, 1000.0_f32);
        let out = zoom_anchor_offset((900.0, 0.0), (500.0, 500.0), (d1, d1), (v, v), (5.0, 0.0));
        assert!(
            out.0 <= d1 - v + 0.01,
            "offset {} exceeds the maximum scroll {}",
            out.0,
            d1 - v
        );
    }

    /// **The clamp wins over the anchor, deliberately.** Documented as its own
    /// test because it is the one case where zoom-to-cursor visibly does not
    /// hold the point still, and a future reader could mistake that for the
    /// bug this feature fixes.
    ///
    /// Anchoring near an edge can demand an offset past the end of the page.
    /// Honouring it would scroll blank space into view; every other canvas
    /// application saturates instead, so the anchored point drifts by exactly
    /// the amount the range was short. Found by a test that first asserted
    /// exact preservation here and failed by 60 px — the assertion was wrong,
    /// not the code.
    #[test]
    fn anchoring_past_the_page_edge_saturates_rather_than_scrolling_into_blank_space() {
        let (v, u) = (800.0_f32, 0.9_f32);
        let (d0, d1) = (600.0_f32, 1000.0_f32);
        let off1 = zoom_anchor_offset((0.0, 0.0), (d0, d0), (d1, d1), (v, v), (u, u)).0;
        let want = 0.9 * (d1 - d0) - 100.0; // 260: the unclamped solve
        let max = d1 - v; // 200: all the range there is
        assert!(
            want > max,
            "this case must actually be over-range to test it"
        );
        assert_eq!(off1, max, "the offset must saturate at the page edge");
    }

    #[test]
    fn a_non_finite_input_refuses_to_move_rather_than_blanking_the_canvas() {
        // `anchor_frac` divides by the drawn page size, which is zero for one
        // frame after an open — so NaN really can reach here.
        let off0 = (120.0, 45.0);
        assert_eq!(
            zoom_anchor_offset(
                off0,
                (0.0, 0.0),
                (100.0, 100.0),
                (800.0, 800.0),
                (f32::NAN, 0.5)
            ),
            off0
        );
    }

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
            resolve_escape(true, true, true, false),
            EscapeOutcome::CancelGesture
        );
        // Priority 2: tool active, no gesture → exit the tool.
        assert_eq!(
            resolve_escape(true, false, true, false),
            EscapeOutcome::ExitTool
        );
        // Priority 3: no tool, non-empty canvas selection → clear it.
        assert_eq!(
            resolve_escape(false, false, true, false),
            EscapeOutcome::ClearCanvasSelection
        );
        // Priority 4: nothing else → the existing rail-clear (this Pass's
        // only live outcome).
        assert_eq!(
            resolve_escape(false, false, false, false),
            EscapeOutcome::FallThroughToRailClear
        );
        // A discardable-gesture flag with no active tool is meaningless and
        // must not fire CancelGesture.
        assert_eq!(
            resolve_escape(false, true, false, false),
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
        // Only the all-hits query is implemented: `hit_test` is the trait's
        // provided method over it, which is exactly the guarantee under
        // test — a provider cannot make the two disagree.
        fn hit_test_all(&self, _page_index: usize, point: Pos2, tolerance: f64) -> Vec<TargetId> {
            // The stub honours `tolerance` by inflating its boxes, so the
            // substrate's own tests exercise the parameter rather than
            // ignoring it.
            #[allow(clippy::cast_possible_truncation)]
            let pad = tolerance.max(0.0) as f32;
            self.boxes
                .iter()
                .filter(|(_, r)| r.expand(pad).contains(point))
                .map(|(id, _)| *id)
                .collect()
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
        assert_eq!(p.hit_test(0, Pos2::new(5.0, 5.0), 6.0), None);
        assert!(p.hit_test_rect(0, r(0.0, 0.0, 100.0, 100.0)).is_empty());
        assert_eq!(p.bounds(0, TargetId(1)), None);
    }

    // ---- click-through cycling (ui-spec §C.3) --------------------------

    /// The stack every cycling test clicks into: three overlapping objects,
    /// front-most first, exactly as a provider reports them.
    fn stack() -> Vec<TargetId> {
        vec![TargetId(7), TargetId(4), TargetId(2)]
    }

    const P: Pos2 = Pos2::new(50.0, 50.0);

    /// One click, no modifiers: unchanged behaviour — the topmost object —
    /// and a recorded cycle so the readout can DISCLOSE that two more
    /// objects are underneath. A capability nobody can tell exists is not a
    /// capability (rule 4).
    #[test]
    fn a_plain_click_selects_the_topmost_and_records_the_stack() {
        let (sel, cycle) =
            selection_and_cycle_after_click(&stack(), &BTreeSet::new(), None, 0, P, false, false);
        assert_eq!(sel, ids(&[7]));
        let cycle = cycle.expect("a plain click into a stack records it");
        assert_eq!(cycle.ordinal, 0);
        assert_eq!(cycle.position(), 1); // "1 of 3"
        assert_eq!(cycle.total, 3);
        assert_eq!(cycle.produced, TargetId(7));
    }

    /// Repeated Alt+clicks at the same point walk DOWN the stack and wrap —
    /// the whole point of the feature: without it, `TargetId(4)` and
    /// `TargetId(2)` are unselectable by any click at this point.
    #[test]
    fn repeated_alt_clicks_step_through_the_stack_and_wrap() {
        let hits = stack();
        let mut sel = BTreeSet::new();
        let mut cycle = None;
        let mut seen = Vec::new();
        for _ in 0..5 {
            let (next_sel, next_cycle) =
                selection_and_cycle_after_click(&hits, &sel, cycle, 0, P, false, true);
            sel = next_sel;
            cycle = next_cycle;
            seen.push(sel.iter().next().copied().expect("one object selected"));
        }
        // First Alt+click with nothing selected behaves as a plain click
        // (topmost); then it walks and wraps.
        assert_eq!(
            seen,
            vec![
                TargetId(7),
                TargetId(4),
                TargetId(2),
                TargetId(7),
                TargetId(4),
            ]
        );
        assert_eq!(cycle.map(ClickCycle::position), Some(2));
    }

    /// The natural gesture: plain-click to select, THEN Alt+click to go
    /// deeper. The Alt+click must not restart at the top just because the
    /// first click was not itself an Alt+click.
    #[test]
    fn alt_click_continues_from_whatever_is_already_selected() {
        let hits = stack();
        // Selection arrived from somewhere else entirely (a tree row click),
        // so there is no cycle at all — only a selection.
        let selected = ids(&[4]);
        let (sel, cycle) =
            selection_and_cycle_after_click(&hits, &selected, None, 0, P, false, true);
        assert_eq!(
            sel,
            ids(&[2]),
            "steps past the selected object, not to the top"
        );
        assert_eq!(cycle.map(ClickCycle::position), Some(3));
    }

    /// Moving the pointer resets the cycle — a cycle that never resets is a
    /// trap, because the operator's next click at a NEW place would silently
    /// select the third object down.
    #[test]
    fn moving_the_pointer_beyond_the_threshold_restarts_the_cycle() {
        let hits = stack();
        let (sel, cycle) =
            selection_and_cycle_after_click(&hits, &BTreeSet::new(), None, 0, P, false, true);
        let cycle = cycle.expect("a cycle");
        assert_eq!(sel, ids(&[7]));

        // A hand tremor within the threshold still counts as the same point.
        let jitter = Pos2::new(P.x + CYCLE_SAME_POINT_CANVAS - 0.5, P.y);
        assert!(cycle.continues(0, jitter, &sel));

        // A real move does not — and the resulting Alt+click starts over at
        // the topmost, exactly as an unmodified click would.
        let elsewhere = Pos2::new(P.x + CYCLE_SAME_POINT_CANVAS * 10.0, P.y);
        assert!(!cycle.continues(0, elsewhere, &sel));
        let (moved, _) =
            selection_and_cycle_after_click(&hits, &sel, Some(cycle), 0, elsewhere, false, true);
        // `sel` is TargetId(7) = position 0 in the stack, so the step from
        // "where the selection sits" goes to 1 — NOT to the stale ordinal.
        assert_eq!(moved, ids(&[4]));
    }

    /// A page change resets the cycle: a `TargetId` is a per-page index, so
    /// the same number is a different object on another page.
    #[test]
    fn a_page_change_resets_the_cycle() {
        let (sel, cycle) =
            selection_and_cycle_after_click(&stack(), &BTreeSet::new(), None, 0, P, false, true);
        let cycle = cycle.expect("a cycle");
        assert!(cycle.continues(0, P, &sel));
        assert!(!cycle.continues(1, P, &sel));
        assert!(!cycle.describes(1, &sel));
    }

    /// A selection change from anywhere else (a tree row, a marquee, an
    /// edit's prune) invalidates the cycle's position, so the readout stops
    /// claiming to describe it.
    #[test]
    fn a_selection_change_from_elsewhere_invalidates_the_cycle() {
        let (_, cycle) =
            selection_and_cycle_after_click(&stack(), &BTreeSet::new(), None, 0, P, false, true);
        let cycle = cycle.expect("a cycle");
        // Something else selected a different object…
        assert!(!cycle.continues(0, P, &ids(&[4])));
        assert!(!cycle.describes(0, &ids(&[4])));
        // …or added to the selection (a multi-selection is not this cycle's
        // object, so "2 of 3" would be a lie about which one).
        assert!(!cycle.describes(0, &ids(&[7, 4])));
        // …or cleared it.
        assert!(!cycle.describes(0, &BTreeSet::new()));
    }

    /// Shift keeps its additive meaning and never cycles: two different
    /// questions, and a modifier that answered both would be unpredictable.
    /// A miss keeps its clearing meaning and drops the cycle.
    #[test]
    fn shift_still_toggles_and_a_miss_still_clears() {
        let hits = stack();
        let (sel, cycle) =
            selection_and_cycle_after_click(&hits, &ids(&[2]), None, 0, P, true, false);
        assert_eq!(
            sel,
            ids(&[2, 7]),
            "Shift adds the topmost, never a deeper one"
        );
        assert_eq!(cycle, None);

        // Alt is ignored under Shift for the same reason.
        let (sel, _) = selection_and_cycle_after_click(&hits, &ids(&[2]), None, 0, P, true, true);
        assert_eq!(sel, ids(&[2, 7]));

        // A miss: plain clears, Shift leaves alone, neither leaves a cycle.
        let (cleared, cycle) =
            selection_and_cycle_after_click(&[], &ids(&[7]), None, 0, P, false, true);
        assert!(cleared.is_empty());
        assert_eq!(cycle, None);
        let (kept, _) = selection_and_cycle_after_click(&[], &ids(&[7]), None, 0, P, true, false);
        assert_eq!(kept, ids(&[7]));
    }

    /// A single object under the pointer is not a stack: cycling it is a
    /// no-op that keeps selecting the same thing, and the readout has
    /// nothing to disclose (`total == 1`).
    #[test]
    fn cycling_a_single_hit_is_a_stable_no_op() {
        let hits = vec![TargetId(9)];
        let (sel, cycle) =
            selection_and_cycle_after_click(&hits, &ids(&[9]), None, 0, P, false, true);
        assert_eq!(sel, ids(&[9]));
        let cycle = cycle.expect("a cycle");
        assert_eq!(cycle.total, 1);
        assert_eq!(cycle.position(), 1);
    }

    /// The trait's provided `hit_test` IS the head of `hit_test_all` — the
    /// invariant `pdfce_core::vector::hit` guarantees in page space, carried
    /// through the provider seam so a first click and a cycling click can
    /// never disagree about what is under the pointer.
    #[test]
    fn the_providers_topmost_query_is_the_head_of_its_all_hits_query() {
        let provider = StubProvider {
            boxes: vec![
                (TargetId(1), r(0.0, 0.0, 100.0, 100.0)),
                (TargetId(2), r(20.0, 20.0, 30.0, 30.0)),
            ],
        };
        for x in [-5.0_f32, 0.0, 25.0, 60.0, 99.0, 200.0] {
            for y in [-5.0_f32, 0.0, 25.0, 60.0, 99.0, 200.0] {
                for tol in [0.0_f64, 3.0, 20.0] {
                    let p = Pos2::new(x, y);
                    assert_eq!(
                        provider.hit_test(0, p, tol),
                        provider.hit_test_all(0, p, tol).first().copied(),
                        "{p:?} at tolerance {tol}"
                    );
                }
            }
        }
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
        // rect — the stale one is silently skipped (spec §4.4 posture) — and
        // it is PAIRED with the target it came from, which is what lets the
        // overlay pick a per-kind treatment for it.
        let selection = ids(&[2, 99]);
        let rects = selection_outline_bounds(&selection, Some(&provider), 0);
        assert_eq!(rects, vec![(TargetId(2), r(20.0, 20.0, 5.0, 5.0))]);
    }

    /// The degenerate-outline fix: a zero-height rect (a horizontal rule)
    /// must come back thick enough to stroke, centred on the rule.
    #[test]
    fn a_zero_height_outline_is_grown_symmetrically_about_the_rule() {
        let flat = r(100.0, 200.0, 200.0, 0.0);
        let out = visible_outline_rect(flat, 6.0);
        assert_eq!(out.height(), 6.0);
        // Grown about the centre: the rule's y is still the band's middle.
        assert_eq!(out.center().y, 200.0);
        // The non-degenerate axis is untouched.
        assert_eq!(out.min.x, 100.0);
        assert_eq!(out.max.x, 300.0);
    }

    #[test]
    fn a_zero_width_outline_is_grown_symmetrically_too() {
        let out = visible_outline_rect(r(200.0, 100.0, 0.0, 200.0), 6.0);
        assert_eq!(out.width(), 6.0);
        assert_eq!(out.center().x, 200.0);
        assert_eq!(out.height(), 200.0);
    }

    /// A single-point object is degenerate on both axes at once.
    #[test]
    fn a_point_outline_is_grown_on_both_axes() {
        let out = visible_outline_rect(r(50.0, 50.0, 0.0, 0.0), 6.0);
        assert_eq!(out.width(), 6.0);
        assert_eq!(out.height(), 6.0);
        assert_eq!(out.center(), Pos2::new(50.0, 50.0));
    }

    /// A rect that is already big enough must come back BYTE-identical: the
    /// outline of an ordinary object must keep reporting that object's real
    /// extent, or every selection box would start lying by a few points.
    #[test]
    fn an_ordinary_outline_is_returned_unchanged() {
        let big = r(10.0, 10.0, 80.0, 40.0);
        assert_eq!(visible_outline_rect(big, 6.0), big);
        // Exactly at the threshold is "already big enough", not "grow".
        let exact = r(0.0, 0.0, 6.0, 6.0);
        assert_eq!(visible_outline_rect(exact, 6.0), exact);
    }

    /// A rect whose corners arrived in the wrong order (the canvas→screen
    /// projection includes a Y flip) is normalised, not turned inside out.
    #[test]
    fn an_inverted_outline_is_normalised_before_growing() {
        let flipped = Rect::from_min_max(Pos2::new(300.0, 200.0), Pos2::new(100.0, 200.0));
        let out = visible_outline_rect(flipped, 6.0);
        assert_eq!(out.min.x, 100.0);
        assert_eq!(out.max.x, 300.0);
        assert_eq!(out.height(), 6.0);
    }

    /// A non-finite rect is left alone — there is no centre to grow about,
    /// and repairing a NaN box here would hide the upstream bug that made it.
    #[test]
    fn a_non_finite_outline_is_left_alone() {
        let nan = Rect::from_min_max(Pos2::new(f32::NAN, 0.0), Pos2::new(10.0, 10.0));
        let out = visible_outline_rect(nan, 6.0);
        assert!(out.min.x.is_nan());
        // A degenerate minimum is refused rather than dividing by zero.
        let flat = r(0.0, 0.0, 10.0, 0.0);
        assert_eq!(visible_outline_rect(flat, 0.0), flat);
        assert_eq!(visible_outline_rect(flat, f32::NAN), flat);
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
        assert_eq!(
            provider.hit_test(0, Pos2::new(5.0, 5.0), 0.0),
            Some(TargetId(1))
        );
        assert_eq!(provider.hit_test(0, Pos2::new(50.0, 50.0), 0.0), None);
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
            // NOT A THEME COLOUR: an arbitrary argument; this asserts geometry.
            assert!(!snap_marker_shapes(Pos2::new(10.0, 10.0), k, Color32::RED, 4.0).is_empty());
        }
        // The derived centerline's glyph must not be visually confused with the
        // routine centerline tick (§2.3.1) — here proven by a different shape
        // composition (a hatched square vs. two dashes).
        let derived =
            // NOT A THEME COLOUR: an arbitrary argument; this asserts geometry.
            snap_marker_shapes(Pos2::ZERO, SnapKind::DerivedCenterline, Color32::RED, 4.0);
        let routine =
            // NOT A THEME COLOUR: an arbitrary argument; this asserts geometry.
            snap_marker_shapes(Pos2::ZERO, SnapKind::SegmentCenterline, Color32::RED, 4.0);
        assert_ne!(derived.len(), routine.len());
    }
}
