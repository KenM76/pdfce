# Pass 12.M2 UI Spec — Dimension Tools (scaled measurement/dimensioning)

> Authored by `pdfce-ui-specialist`, on dispatch from the engineer. This is
> the implementable interaction spec for Pass 12.M2 — the headline new
> capability of decision 011's first beta. The engineer implements it
> verbatim, deviating only with a recorded reason (the standing Pass
> 3.2/6.1/7/8/12.0/14.3/15.2/16.2 spec convention).
>
> Read before implementing: `docs/decisions/011-first-beta-scaled-
> measurement-dimensioning-tool.md` §2.2–2.4 and its `pass_slicing`
> Pass-12.M2 JSON block (binding deliverables/acceptance criteria — this
> spec expands them into widget trees, it does not relax them);
> `docs/ui_specs/pass-12.0-canvas-substrate.md` (the substrate this Pass
> layers onto — §1–§6, especially §2.3's two geometry bridges and §3's
> `CanvasTool`/`resolve_gesture_interrupt`/`resolve_escape` machinery);
> `crates/pdfce-core/src/vector/{mod.rs,decompose.rs,hit.rs,centerline.rs}`
> and `crates/pdfce-gui/src/{canvas.rs,object_provider.rs,main.rs}` **in
> full, as actually shipped** — §0.1 below records exactly what already
> exists vs. what this Pass must still add, confirmed by reading the code,
> not assumed from the decision doc alone; `D:\Dev\Rag-Specialized\
> Acrobat_Features\measure__*.md` (grounding — §0.2 below); `D:\Dev\Rag-
> Specialized\PDF_Spec\iso32000\iso32000__s__12.9.md` (the `/Measure`/
> `/Viewport`/`/NumberFormat` model this Pass's storage half authors
> against — storage bytes are the engineer's/spec-librarian's call, not
> named here beyond what the interaction needs to know).

---

## 0. What already exists, what this Pass adds, and the gaps a UI spec must name up front

### 0.1 Shipped substrate this Pass builds on (confirmed by reading the code, 2026-08-01)

- **Pass 12.0** (canvas substrate) — shipped. `CanvasTool` is inhabited by
  `TextEdit` (14.3) and `AddText` (16.2); `active_tool: Option<CanvasTool>`,
  `canvas_selection: BTreeSet<TargetId>`, `resolve_escape`,
  `canvas_suppresses_pan`, the two geometry bridges
  (`viewer::screen_to_page`/`page_to_screen`,
  `viewer::canvas_to_pdf_space`/`pdf_space_to_canvas`) are all real and
  tested (`crates/pdfce-gui/src/canvas.rs`).
- **Pass 9a** (object/selection model + centerline) — shipped.
  `pdfce_core::vector::{decompose_page, hit_test_point, hit_test_rect,
  page_candidates, CenterlineCandidate}` are real; `pdfce-gui`'s
  `ObjectModelProvider` (`crates/pdfce-gui/src/object_provider.rs`) is a
  real `CanvasTargetProvider` over the current page, click/marquee
  object-selection is wired in `main.rs`'s `canvas()` (~lines 4948–5120),
  and `OpenDoc::canvas_selection` is a real, populatable `BTreeSet<TargetId>`.
- **Pass 12.M1** (snapping engine) — **NOT shipped as of this design
  session.** Confirmed by grep: no `snap` module anywhere in
  `pdfce-core/src`. This spec designs the snap-indicator UX against the
  CONTRACT decision 011 §2.2 specifies and names concrete, binding asks
  for 12.M1 in §2.4 below — the same "design against the announced
  contract, name the asks" discipline `pass-14.3`/`pass-15.2` already
  used for their own not-yet-shipped `EditSession` siblings.
- **Pass 12.M2 core** (dimension geometry, best-fit, group model, hybrid
  storage) — **NOT shipped.** This entire subsystem is this Pass's job;
  §0.3 below names where this spec leaves the exact API shape to the
  engineer (per the established convention: `TargetId`'s representation,
  `EditSession` command shapes, etc. are never dictated at the UI-spec
  layer) and where it makes a **binding** interaction-level ask.

### 0.2 Acrobat grounding honored (read `measure__*.md` in full before reviewing)

- **Both scale-entry paths are co-equal** (`measure__scale_and_calibration.md`)
  — direct ratio entry and calibrate-by-known-length are described as
  equally primary in the reference product. This spec still makes the
  real-length path the RECOMMENDED one (decision 011's own instruction,
  and the unambiguous one — a ratio needs a paper-unit basis the operator
  must additionally understand), but never demotes the ratio path to a
  buried fallback.
- **"No scale set" must be its own disclosed state**, not a silent 1:1
  assumption (`measure__scale_and_calibration.md`'s explicit
  recommendation, independent of what Acrobat itself does) — §2.4/§6
  design a **tri-state** (never-set / explicitly-1:1 / calibrated) so a
  legitimate 1:1 scale is never confused with "forgot to calibrate."
- **pdfce EXCEEDS Acrobat** on three named points, and this spec treats
  each as a first-class capability, not an apologetic afterthought:
  radius/diameter dimensions (Acrobat has none — `measure__perimeter_
  and_area_tools.md` — so there is no GUI mechanic to avoid copying,
  only a best-fit-circle interaction to originate honestly); architectural
  feet-inches display (`4'-6"`, a durable, still-open Acrobat feature
  request per `measure__units_and_number_format.md`); named, per-group
  scale+units that are **non-geometric** (any dimension can join any
  group regardless of page position — Acrobat's own `/Viewport` scoping
  is a hard geometric partition, per `measure__scale_and_calibration.md`'s
  2026-08-01 addendum. This is a genuine structural exceed, not a
  restyled parity point, and shapes §5's Group panel design directly.
- **Chained-dimension snapping (snap to a prior dimension's own
  geometry) is a pdfce origination, not a parity claim** — no Acrobat
  source confirms or denies whether Snap-to-Content includes previously-
  authored measurement annotations. §2.4 recommends INCLUDING them
  (lower-risk, more useful, and free given decision 011's object model
  already treats annotations as first-class hit-testable geometry) and
  states this is a design origination.
- **No GUI mechanics were read from Acrobat** — per rule 12/R61, only
  capability/behavior/limits informed this spec; every widget, menu, and
  dialog design below is pdfce's own.

### 0.3 Where this spec deliberately leaves core's exact shape open

Per the established convention (`pass-12.0` §4.1: "concrete representation
… is the engineer's/Pass 9a's call — not dictated here"), this spec does
**not** dictate: the exact `Group`/`Dimension` struct field layout, the
`EditSession` command enum variants, the `/PieceInfo` byte schema, or the
`TargetId` internals for a working best-fit-pick set. It **does** make
binding interaction-level asks (§2.4, §3.4, §4.5, §5.6, §10) — the same
distinction `pass-14.3`'s "core needs `edit_text`/`format_text` siblings"
and `pass-15.2`'s "core needs `reflow_block`" asks made: naming the
CONTRACT the UI needs, not the implementation.

---

## 1. Tool modes

### 1.1 Three new `CanvasTool` variants, not five — a deliberate reduction from decision 011's naming

Decision 011 names "linear," "radius," "diameter," and "scale" dimensions
as separate capabilities. This spec adds **three** `CanvasTool` variants,
not four, because radius and diameter are **the same geometry, displayed
differently** (decision 011 §2.3's own value-data-model: `displayed
radius = fitted_radius × scale; diameter = 2×`) — giving them separate
tools would assert a false identity distinction the data model itself
denies. A fourth, `MeasureScale`, is its own variant rather than a hidden
mode of `MeasureLinear` for the same reason Pass 16.2 rejected folding
`AddText` into `TextEdit` (§0.1 of that spec): a two-point pick meaning
"author an ordinary dimension" vs. "calibrate the active group's scale"
based on invisible state would be exactly the silent mode-shift
fuzzy-never-sneaky exists to prevent. Scale-setting is deliberately,
discoverably its own tool the operator explicitly enters.

```rust
// Added to the existing `pdfce_gui::canvas::CanvasTool` (currently
// TextEdit, AddText — see §0.1). Doc-comment style matches the existing
// two variants' own documentation depth.
pub enum CanvasTool {
    TextEdit,   // existing (14.3)
    AddText,    // existing (16.2)

    /// Linear dimension: two snapped picks (§2), optional H/V/aligned
    /// constraint (§2.3), assigned to a dimension group (§5). A plain
    /// click while this tool is active is ALWAYS a point-pick for the
    /// in-progress dimension — never an object-selection click (mirrors
    /// TextEdit/AddText's own "the active tool owns plain clicks" rule,
    /// Pass 12.0 §4.2's default-provider dispatch applies only when NO
    /// tool is active).
    MeasureLinear,

    /// Radius/diameter dimension from a best-fit circle (§3): clicks
    /// toggle-add/remove OBJECTS (not raw points) into this tool's own
    /// working pick-set, live-refitting a Taubin circle as the set
    /// changes (§3.2). Deliberately reuses object-level `TargetId` picks
    /// via `CanvasTargetProvider::hit_test` — NOT `canvas_selection`
    /// (§3.1 explains why this tool owns its own pick-set, matching
    /// TextEdit/AddText's "each tool owns its own state shape"
    /// precedent, not the substrate's general-purpose selection).
    MeasureCircular,

    /// Scale dimension (§4): draws a reference line exactly like
    /// `MeasureLinear` (same two-point-pick/snap mechanic), then routes
    /// to the scale-entry sub-panel (§4.2) instead of an ordinary value
    /// display. A DISTINCT, deliberately-chosen tool (§1.1) — never a
    /// hidden `MeasureLinear` sub-state.
    MeasureScale,
}
```

`tool_builds_dimension_pick(tool)` / `tool_builds_circular_pick(tool)` /
`tool_builds_scale_pick(tool)` — three small pure predicates mirroring
`tool_builds_text_edit`/`tool_builds_add_text` exactly (same
mutual-exclusion-is-free-because-`Option`-is-a-single-value reasoning,
§0.1 of `pass-16.2`), unit-tested the same way.

### 1.2 Placement — a "Measure ▾" menu, not four new toolbar icons (rule 3 tension, named and resolved)

**The tension:** Edit Text/Add Text each earned a dedicated toolbar toggle
icon because there are exactly two of them and they are the single most
common editing surface (per `pass-16.2`'s own explicit reasoning). Four
new dimension tools as four more toolbar icons would be the FOURTH
addition to an already-growing toolbar (Markup ▾, Text ▾, Edit Text, Add
Text) and is exactly the "Acrobat's own worst habit" rule 3 warns against
— primary-toolbar icon creep for a feature most sessions will use in
short, deliberate bursts (draw some dimensions, then go back to reading),
not continuously like text editing.

**The resolution:** a `ui.menu_button` — reusing the **existing widget**
Markup ▾/Text ▾ already use — labelled dynamically so the active tool is
never hidden by the closed state:

```rust
// ui_text::measure_menu_button(active: Option<MeasureToolLabel>) -> String
// "Measure ▾"                      when no measure tool is active
// "Measure: Linear ▾"              when MeasureLinear is active
// "Measure: Radius/Diameter ▾"     when MeasureCircular is active
// "Measure: Set Scale ▾"           when MeasureScale is active
```

This is a **new combination** of two existing precedents, named
explicitly so a future reader does not mistake it for either verbatim:
the WIDGET is Markup ▾'s (`ui.menu_button`, closes on pick), but the
DISPATCH is Edit Text/Add Text's (`Action::SelectCanvasTool`, a real
`active_tool` toggle) — Markup ▾ itself currently dispatches an
*immediate-author* action (`Action::AddMarkupShape`, a default-rect
placeholder — Pass 6.1's real drawing-tool canvas interaction is still
the deferred slice, confirmed by reading `main.rs`'s `add_markup_shape`),
which is NOT what dimension tools need (they need a live, on-canvas
multi-pick gesture, exactly like `MeasureLinear`/`MeasureCircular`
above). Each menu row is a `ui.selectable_label` (bold when it is the
active tool, per `toggle_label`'s existing bold-active convention — rule
6), not a plain `ui.button`, so the open menu itself shows which tool (if
any) is active. Clicking the already-active row exits the tool
(`SelectCanvasTool(None)`), identical to Edit Text/Add Text's own
toggle-off behavior.

```
Measure ▾
├─ Linear Dimension                      (Ctrl+Shift+D)
├─ Radius / Diameter Dimension
├─ Set Group Scale…
├─ ───────────────
└─ Manage Dimension Groups…              (opens §5's window; does not
                                            change active_tool)
```

**One keyboard chord, not four** — `Ctrl+Shift+D` toggles `MeasureLinear`
specifically (the single most common measure action, mirroring 16.2's
own "the common tool earns the chord, the rarer variants are menu-only"
reasoning for Add Text vs. its own box-mode). Verify `Ctrl+Shift+D` is
unclaimed before binding (grep `collect_keyboard_actions` at
implementation time — `Ctrl+Shift+E`/`Ctrl+Shift+Z`/`Ctrl+Shift+C` are the
only Shift-chords bound as of `pass-16.2`'s own audit). Radius/Diameter
and Set Scale are menu-only — discoverable, not muscle-memory-optimized,
consistent with progressive disclosure for the rarer actions.

**Enablement:** the whole `Measure ▾` button is `add_enabled_ui(!doc.
pages.is_empty(), …)`-gated, matching Edit Text/Add Text's own
"greyed, not hidden, with no pages" rule — there is something to
discover about a measuring tool even on an empty document, unlike a
control that is meaningless without ANY document open.

### 1.3 Gesture-interrupt / Escape wiring — reuses the substrate exactly, no new mechanism

All three new tools plug into the EXISTING `resolve_gesture_interrupt`/
`resolve_escape` machinery (`pass-12.0` §3.3/§3.5) with the SAME
`GestureInterrupt::Discard` policy Pass 6.1's shapes use (nothing is
written to `EditSession` until Accept — §8's undo/write-path table
confirms every measure gesture is discardable, never commit-on-interrupt
like Pass 7's text-field draft). Escape's stage 1 (cancel gesture, stay
in tool) clears the in-progress pick(s) — the first point of a linear
dimension, the working pick-set of a circular fit, the scale line — and
stage 2 (exit tool) behaves identically to Edit Text/Add Text's own exit.
**No new enforcement point is added anywhere** (`pass-12.0`'s explicit
binding instruction).

---

## 2. Placement + snapping feedback

### 2.1 The two-pick model (`MeasureLinear`)

Click 1 resolves the current snap candidate (§2.2) to point A; a
live-preview line follows the cursor to the current candidate for point
B; click 2 commits point B and opens the value/group property bar (§2.5).
This is the SAME "pick A, pick B, live preview between" shape as
`measure__distance_tool.md`'s "two-click model" — a capability fact, not
a copied GUI mechanic (rule 12).

### 2.2 The fuzzy snap indicator — shown BEFORE every commit, never silent

Every frame the `MeasureLinear`/`MeasureCircular`/`MeasureScale` tools are
active and the pointer is over the canvas, the current snap candidate (if
any, within tolerance) is drawn as:

1. a small glyph AT the candidate point (shape distinguishes kind —
   rule 6, colour is never the sole signal):

   | `SnapKind` | Glyph | Label |
   |---|---|---|
   | Node (path anchor) | `◼` filled square | "node" |
   | Endpoint | `●` filled circle | "endpoint" |
   | Center (incl. best-fit) | `⊕` crosshair-in-circle | "center" |
   | Midpoint | `▲` small triangle | "midpoint" |
   | Intersection | `✕` cross | "intersection" |
   | Segment centerline (routine — stroked path) | `┄` dashed tick | "centerline" |
   | Page axis / grid | `⊞` grid glyph | "axis" |

2. a small text pill immediately beside the glyph, e.g. `◼ node`,
   `⊕ center`, drawn via the SAME live-preview-overlay painter
   `pass-12.0` §5 already wires (never a re-raster).

**This is distinct from, and must not be visually confused with, the
filled-shape centerline DERIVATION (§2.3.1)** — a routine "centerline"
snap (a stroked path's own geometry, §2.3.2, zero inference) uses the
plain dashed-tick glyph above; a DERIVED centerline candidate (a filled
thin quad, a real fuzzy inference requiring confirmation) gets its own,
visually distinct treatment (§2.3.1) so the operator is never shown the
same glyph for a routine fact and an inferred one.

### 2.3 What the fuzzy indicator resolves against

#### 2.3.1 Derived centerlines (filled thin quads) — a required, EXTRA confirm step, proportional not gated

When the pointer is near an object `page_candidates()` (already-shipped,
`pdfce_core::vector::centerline`) flags as a `CenterlineCandidate`, the
indicator shows a DIFFERENT glyph (`▤` hatch square) and label: **"derived
centerline (unconfirmed)"** — never the plain "centerline" glyph/label
above, so the operator can tell at a glance that this specific candidate
needs one more look. The candidate's own midline is drawn as a dashed
highlighted overlay (reusing the marked/PREVIEW visual language from
Pass 8/14.3: dashed stroke, never solid, so it reads as "proposed," not
"placed"), alongside a disclosure-strip line:

> "Centerline derived from a filled shape (long:short ≈ 12.3:1) — click
> again to confirm this is a drawn line, not a rectangle."

**The confirm mechanism, proportional to the (non-destructive,
pre-Accept, fully undoable) stakes:** the FIRST click on a derived
candidate only PROMOTES it to "proposed" (highlights it, does not yet set
point A/B); a SECOND click (or an explicit "Use this centerline" button
in the property bar) confirms it as the actual pick. This is deliberately
lighter than Pass 8's refusal-acknowledgement GATE (a checkbox blocking
a destructive Apply) — reflow's own precedent (`pass-15.2`'s "overflow
disclosed calmly, no gate") for the same reason: nothing here is
destructive or post-save-irreversible, so a proportionate two-click
confirm (not a blocking modal, not a separate checkbox) is the correct
weight. **Never auto-applied** — a plain single click near a derived
candidate must NOT silently become the dimension's endpoint.

#### 2.3.2 Routine snap targets (nodes/endpoints/centers/midpoints/
intersections/segment-centerlines/axis) — no extra confirmation, per decision 011's own framing (these are facts about existing geometry, not inferences about operator intent)

Single click commits immediately, exactly like an ordinary pick — the
"fuzzy" surface here is the INDICATOR itself (showing what was inferred
before commit), not a gate on using it. This matches decision 011 §2.2's
own framing: "The operator sees exactly what was inferred and can
cycle/override" — override, not confirm-twice, because these are
deterministic facts about geometry already on the page, not a judgment
call about operator intent the way §2.3.1's derivation is.

### 2.4 Cycle/override — the binding ask for Pass 12.M1

**Decision 011 §2.2 requires cycle/override; this is only possible if
12.M1 returns every tied candidate within tolerance, not just the
winner.** Binding ask:

```rust
// pdfce_core (new, 12.M1) — the contract this spec designs against.
pub enum SnapKind { Node, Endpoint, Center, Midpoint, Intersection, SegmentCenterline, Axis }

pub struct SnapCandidate {
    pub point: Point,        // page space
    pub kind: SnapKind,
    pub source_object: Option<usize>,  // index into PageObjects::objects, for a disclosure
}

/// ALL candidates within `tolerance` of `point`, sorted by (priority
/// ascending, then distance ascending) — decision 011 §2.2's 7-item
/// priority list, ties broken by nearest. Returning the FULL list (not
/// just the top pick) is what makes Tab-cycle possible; the GUI treats
/// index 0 as the default pick and Tab advances through the rest.
pub fn snap_candidates(
    point: Point,
    tolerance: f64,      // PAGE-SPACE, already zoom-converted by the caller
    model: &PageObjects,
) -> Vec<SnapCandidate>;
```

**GUI-side wiring:** the canvas converts a fixed screen-space tolerance
(8–12 px, decision 011's own number) to page-space via `doc.view.zoom`
each frame (the same zoom-invariance property `pass-12.0` §2.2 already
tests for `screen_to_page`), calls `snap_candidates`, and:

- shows candidate `[0]`'s glyph/label by default;
- **Tab** (while a measure tool is active and the candidate list is
  non-empty) cycles to the next candidate, wrapping;
- **holding Alt** suppresses snapping entirely for the current pick (a
  transient per-pick override, matching the reference product's
  documented master on/off concept — `measure__snapping_behavior.md` —
  without copying its exact key);
- a property-bar **"Snap to content"** checkbox (default ON) is the
  persistent master toggle Acrobat's own model documents as a
  must_have — off, `snap_candidates` is simply never called and every
  pick is the raw pointer position.
- **Derived centerlines (§2.3.1) participate in this SAME candidate list**
  at whatever priority the engineer assigns (recommend: just above
  segment-centerline, since it is a MORE specific inference about the
  SAME kind of geometry) — but carry the extra confirm step regardless
  of list position; cycling past one shows its distinct glyph/label,
  never the routine one.
- **Chained-dimension snapping (§0.2):** `snap_candidates`'s `model:
  &PageObjects` should be built to include the CURRENT page's
  already-authored dimension annotations' own witness-line geometry
  (endpoints, at minimum) as ordinary `Endpoint`/`Node` candidates —
  the pdfce-origination recommendation from §0.2, flagged here as a
  concrete ask because it changes what `PageObjects` must be assembled
  FROM (page content stream objects only, today, per 9a's own module
  docs — this Pass's dimension annotations are a genuinely new object
  KIND those decomposition inputs do not yet include).

### 2.5 H/V/aligned constraint — a segmented control in the property bar, not a modifier key

Matching `pass-15.2`'s own alignment-picker precedent (real
`selectable_value` buttons, not a `ComboBox` or a hidden modifier-key
convention an operator must be told about out-of-band): the
`MeasureLinear`/`MeasureScale` property bar (a floating top `egui::Area`,
`pdfce-measure-propbar`, identical anchor/frame convention to
`pdfce-text-edit-propbar` — `image_rect.min + (8,8)`, `egui::Frame::
popup`) shows three buttons: **Aligned | Horizontal | Vertical**. Under
Horizontal/Vertical, the second pick's snap candidate is still shown and
still cyclable (§2.4), but the COMMITTED point is the axis-projected
value (`measured_pdf_length = |Δx|` or `|Δy|`, decision 011 §2.2) — a
live preview line shows the ACTUAL projected segment (not the raw
diagonal to the raw candidate), so what is on screen during the pick is
exactly what will be measured, never a surprise at Accept.

```rust
// pdfce_core (12.M2, new) — pure, testable without a live pick.
pub enum AxisConstraint { Aligned, Horizontal, Vertical }
pub fn constrained_second_point(first: Point, raw_second: Point, c: AxisConstraint) -> Point;
```

### 2.6 Property bar + status/disclosure strip — reuses the exact Accept/Reject convention

`MeasureLinear`'s status strip (`pdfce-measure-status`, bottom-left
anchored, identical to `pdfce-text-edit-status`'s `fixed_pos`/`pivot`)
shows, while a dimension is in progress:

- the raw page-space length AND the scaled display value side by side —
  `12.4 pt → 3.10 m` — or, if the active group has never had a scale set
  (§4's tri-state), **"no scale set — showing raw page units"** verbatim
  as the group's own disclosure (never silently presented as a
  real-world number, per §0.2's Acrobat-RAG-sourced recommendation);
- the group picker (a `ComboBox` of existing groups + "New group…",
  seeding a default-named group inline — §5.4);
- Accept/Reject as REAL `ui.button`s (`ICON_BUTTON_SIZE`, accesskit win,
  matching every prior tool's own reasoning);
- the disclosure strip proper (§6) rendering core's disclosures
  VERBATIM, never paraphrased (the standing `pass-14.3` rule).

Reject discards with nothing written (rule 7, no friction for a
reversible in-progress gesture); Accept commits ONE undo-able
`EditSession` command (§8).

---

## 3. Radius/diameter dimensions

### 3.1 Why this tool owns its OWN pick-set, not `canvas_selection`

`canvas_selection` is the substrate's general-purpose, no-tool-active
object selection (Pass 9a). While `MeasureCircular` is active, plain
clicks must mean ONE thing — "add/remove this object from my circle-fit
attempt" — never ambiguously double as "also change what the ordinary
Properties/rail selection sees." This is the same reasoning `pass-14.3`
used to keep text selection out of `CanvasTargetProvider` entirely: each
tool's selection-like state is shaped for what that tool needs, and
conflating it with the substrate's general selection is either wrong (a
circle-fit pick-set has no sensible meaning as a "current object
selection" for Properties) or silently surprising (leaving stale
objects "selected" after the tool exits). `MeasureCircularState.
picked_objects: BTreeSet<TargetId>` is entirely separate from
`OpenDoc::canvas_selection` and is cleared on tool exit, mirroring
`TextEditState`/`AddTextState`'s own "cleared when the tool exits,
never leaked into another tool's state" convention.

### 3.2 Two entry points into the SAME fit

- **From a circle/ellipse object:** a plain click on a single filled-or-
  stroked closed path whose flattened outline is already circle-like
  (the engineer's call whether to special-case detection or simply
  always run the fit on whatever is clicked — decision 011 does not
  require a separate "is this a circle" predicate, only that the FIT
  handle the case) auto-populates the pick-set with that one object and
  immediately shows a fit preview.
- **From multiple selected line-segment objects:** clicking (toggle-add,
  §3.1) several small path objects that together approximate an arc
  builds the pick-set incrementally; the fit re-runs live after every
  toggle. **No new node-level selection primitive is needed for this** —
  a genuinely important finding, worth stating explicitly (mirroring
  `pass-16.2`'s own "font enumeration needs no new accessor" positive
  finding): decision 011's "multiple selected nodes … might be small
  line segments" maps cleanly onto OBJECT-level multi-pick, because
  each such "node" is really a separate small path OBJECT contributing
  its own anchor points to the sample set, and Pass 9a's object model
  already exposes every selected object's full anchor list
  (`PathObject::page_subpaths()` → `Subpath::anchors()`, confirmed by
  reading `centerline.rs`'s own use of exactly this accessor). A single
  polyline object with many anchors (one object, many nodes) is handled
  identically — selecting it alone hands its own full anchor list to
  the fit.

### 3.3 The fit-input accessor gap — a binding ask, not optional wiring

`CanvasTargetProvider` (§4.1 of `pass-12.0`) is deliberately OPAQUE —
`hit_test`/`hit_test_rect`/`bounds` only, no node geometry — which is
correct for the SUBSTRATE's own purposes (selection outlines only need
bounds) but insufficient here: feeding the Taubin fit needs the actual
flattened point samples of every picked object, in PDF space.
Re-decomposing the page a second time (a fresh `decompose_page` call) to
get this would risk exactly the "two decompositions quietly diverge"
Z2 pattern decision 011 itself names and Pass 9a/Pass 12.0 both went out
of their way to avoid (the object model and the render "agree by
construction" specifically because there is only ONE decomposition per
page per frame-state).

**Binding ask:** `ObjectModelProvider` (already the sole owner of the
current page's `PageObjects`) should expose a same-crate accessor the
`MeasureCircular` tool code calls directly — e.g.

```rust
// crates/pdfce-gui/src/object_provider.rs — a NEW, same-crate accessor
// alongside the existing CanvasTargetProvider impl. Does not change the
// trait (the substrate stays opaque, per pass-12.0 §4.1); this is
// pdfce-gui-internal wiring so the dimension tool reuses the SAME
// decomposition the selection provider already built, once per page.
impl ObjectModelProvider {
    pub(crate) fn page_objects(&self) -> &PageObjects { &self.objects }
}
```

`OpenDoc` needs a way to reach the CONCRETE `ObjectModelProvider`, not
just the opaque `Box<dyn CanvasTargetProvider>` `target_provider` holds
— recommend `OpenDoc` retain a second, concretely-typed field (e.g.
`object_model: Option<ObjectModelProvider>`) populated by the SAME
`ensure_object_provider` call that already builds `target_provider`,
rather than attempting a `dyn Any` downcast. This is the SAME "name the
concrete ask, leave the exact field name/shape to the engineer"
discipline as every prior spec's core-accessor flags.

Once the tool has each picked object's PDF-space sample points
(flattening Béziers the SAME way `hit.rs`'s own `flatten()` already
does — reuse it, do not re-derive a second flattening routine, the same
"one pipeline" instinct that already governs this codebase), it calls
12.M2's Taubin fit:

```rust
// pdfce_core (12.M2, new)
pub struct FitCircle { pub center: Point, pub radius: f64, pub residual: f64 }

/// Taubin best-fit over an arbitrary point set (decision 011 §2.3:
/// chosen specifically for partial-arc/short-segment bias resistance).
/// `None` for a degenerate input (fewer than 3 usable points, or a
/// numerically singular fit).
pub fn fit_circle_taubin(points: &[Point]) -> Option<FitCircle>;
```

### 3.4 Best-fit preview — accept/reject, never auto-applied

The live fit (re-run on every pick-set change) draws as a DASHED circle
outline (the same PREVIEW visual language as §2.3.1/Pass 8/14.3 —
dashed, never solid, until Accept) plus a small centre-crosshair glyph,
with the disclosure strip showing:

> "Best-fit circle from 4 objects — radius 18.2 pt, fit residual 0.3 pt
> (RMS)."

The residual is ALWAYS shown, not just on request — decision 011's own
instruction ("the fit residual … is reported so the operator sees fit
quality"), and a poor fit (large residual relative to radius) gets a
`colored_label(warn_fg_color, …)` treatment paired with the SAME text
(rule 6 — colour is never the sole signal; the number itself already
says "this fit is loose"). A **Radius | Diameter** segmented toggle (same
`selectable_value` widget convention as §2.5's H/V/Aligned) picks the
DISPLAY only — `displayed = radius` or `2×radius`, both scaled by the
active group — never a second fit. Accept/Reject strip is identical to
§2.6's.

---

## 4. The scale-dimension workflow (the load-bearing operator requirement)

### 4.1 Entry

`MeasureScale` draws a reference line with the EXACT same two-point-pick/
snap mechanic as `MeasureLinear` (§2.1–2.5, including H/V/aligned and the
fuzzy snap indicator — a scale reference line snaps to real geometry
exactly like an ordinary dimension). On the second pick, instead of an
ordinary value/group property bar, the property bar switches to the
scale-entry sub-panel below.

### 4.2 The scale-entry sub-panel — two co-equal paths, one clearly recommended

```
┌─ Set Group Scale ──────────────────────────────────┐
│ Drawn reference length: 42.3 pt                      │
│                                                       │
│ ○ Real length of this line  (recommended)            │
│     [   25.0  ] [ ft ▾ ]                             │
│     → scale = 25.0 ft / 42.3 pt                      │
│                                                       │
│ ○ Direct ratio                                       │
│     [ 1 ▾ ] paper-unit  =  [ 100 ▾ ] real-unit       │
│     (paper-unit basis: 1 in = 72 pt, disclosed)      │
│                                                       │
│ Apply to group:  [ Floor Plan ▾ ]   [+ New group]    │
│                                                       │
│           [ Accept ]        [ Reject ]               │
└───────────────────────────────────────────────────────┘
```

- **The radio-equivalent is `selectable_value` (matching every prior
  segmented-choice precedent in this project — H/V/Aligned, radius/
  diameter, reflow's alignment picker), not two separate checkboxes** —
  the paths are mutually exclusive, one is always the active choice.
- **"Real length of this line" is pre-selected by default** — the
  recommended path per decision 011 and the Acrobat RAG's own framing
  (unambiguous; the ratio path needs the operator to additionally
  understand the paper-unit basis). This is a DEFAULT, not the only
  option — Acrobat treats both as co-equal (§0.2) and so does this
  panel; the operator can switch to Direct ratio with one click.
- **The paper-unit basis is ALWAYS shown, even when the ratio path is
  not selected** — "(paper-unit basis: 1 in = 72 pt, disclosed)" is a
  small caption under the ratio row regardless of which path is active,
  because an operator who has never touched the ratio path should not
  have to open it to learn the basis exists before deciding which path
  to use.
- **Live preview of the resulting scale**, computed and shown BEFORE
  Accept (`→ scale = 25.0 ft / 42.3 pt`), so the operator can sanity-check
  the number before committing — this is the SAME "preview + real
  EditSession-integrated commit" split `pass-14.3`/`pass-16.2` already
  established for other tools (§4.5's binding ask names the exact core
  functions this needs).
- **Group target** — a `ComboBox` identical to §2.6's, plus an inline
  "+ New group" that seeds a fresh group named "Group N" (matching
  Font Folders' own inline-create convention for a first pass at
  naming, renamable later in §5).

### 4.3 The tri-state, made concrete

A group's scale state is never collapsed to a single "has a scale /
doesn't" boolean — three real states, each with its own disclosure copy:

| State | Group Manager (§5) shows | Property bar (§2.6) shows |
|---|---|---|
| Never set (fresh/default group) | "No scale set" (neutral, not a warning glyph — a fresh group is a normal starting state, not an error) | "no scale set — showing raw page units" |
| Explicitly set to a literal 1:1 | "Scale: 1:1 (set by operator)" | the scaled value (identical to raw, correctly, with no caveat) |
| Calibrated / ratio-set to any other value | "Scale: 1 ft = 4.0 pt" (or whatever the ratio resolves to) | the scaled value |

This distinction matters precisely because a legitimate 1:1 real-world
scale (a full-size detail drawing) must never look, in the UI, like an
operator who simply never got around to calibrating.

### 4.4 Re-propagation

Accepting a scale change re-derives EVERY member dimension's displayed
value and regenerates its baked `/AP` label (decision 011's own
instruction, "reusing the Pass 7.1 regenerate-appearances pattern") as
part of the SAME undo-able command — the operator's mental model is "I
changed the group's scale," one action, one undo step, matching
Properties' own "Apply is one undo step for the whole panel, however
many fields changed" precedent (`main.rs`'s own documented reasoning for
`ApplyProperties`).

### 4.5 Binding core asks

```rust
// pdfce_core (12.M2, new) — the preview/commit split, matching the
// edit_text/format_text and reflow_block precedent exactly.

/// Pure preview: no mutation, no EditSession — just the arithmetic the
/// entry panel shows live. `known_length`/`known_units` for the
/// real-length path; `ratio_num`/`ratio_den` + a disclosed paper-unit
/// basis for the ratio path. Exactly one of the two input shapes is
/// populated per call (an enum, not two optional pairs — the engineer's
/// call on exact shape).
pub fn preview_group_scale(drawn_pdf_length: f64, entry: ScaleEntry) -> ScalePreview;

// EditSession sibling (mirrors edit_text/format_text's own shape):
// takes the SAME entry (re-derived at commit time, not a pre-computed
// preview passed through verbatim — pass-15.2's own binding reasoning
// for reflow_block applies identically here: what the operator reviewed
// pre-Accept must be exactly what commits), regenerates every member
// dimension's baked /AP, returns the disclosures §6 renders verbatim.
```

---

## 5. Group panel

### 5.1 Placement — the existing "edit → window" taxonomy bucket, not a new instance

The five-way placement taxonomy already has an instance for
"edit-type control living in a modeless window" — the Properties panel.
The Group panel is the SAME bucket, not a new taxonomy entry: it is a
live, canvas-tool-scoped, document-internal editing surface, not an
"advanced batch operation on files outside the one you have open" (the
Tools dock's own framing, which is why the Redact review panel needed a
genuinely new seventh instance in `pass-8` — this does not). **Named
explicitly so a future reader does not go looking for (or invent) an
eighth taxonomy instance for this panel.**

A modeless `egui::Window` ("Dimension Groups"), `resizable(true)`,
`default_width(520.0)` (wider than Properties' 420, fewer but denser
rows: name, scale summary, layer toggle, member count), opened from
`Measure ▾`'s "Manage Dimension Groups…" item, and — for workflow
continuity — offered as a one-click link in the scale-entry sub-panel's
post-Accept disclosure ("Scale applied to 'Floor Plan' — [Open Group
Manager]").

### 5.2 List + per-group inline edit — Font Folders' list shape, Properties' Apply/dirty-check shape, combined deliberately

```
┌─ Dimension Groups ─────────────────────────────────────┐
│ [+ New Group]                                            │
│                                                            │
│  👁  Floor Plan          1 ft = 4.0 pt      12 dims   [≡] │
│  👁  Detail Callouts     no scale set        3 dims   [≡] │
│  ⊘   Site Plan (hidden)  1 m = 2.83 pt        7 dims   [≡] │
└────────────────────────────────────────────────────────────┘
```

- **`👁`/`⊘` is the per-group OCG-layer visibility toggle** (decision
  011's "per-group GUI toggle" ask) — a click flips the group's
  authored-layer `/OC` visibility (Pass 12.0's bounded `/OC` honoring,
  decision 011 §2.4). Paired with the row's own greyed styling AND the
  "(hidden)" text suffix when off — rule 6: never rely on the eye glyph
  alone.
- **`[≡]` expands the row into its own inline edit sub-form** — rename,
  scale (re-opens §4.2's SAME entry widget, seeded with the current
  value, so there is only ONE scale-entry UI in the whole app, never a
  second one for "edit an existing scale" vs. "set a new one"), units
  dropdown (mm/cm/m/inch/decimal-ft/ft-in), number-format (precision;
  **and, for ft-in specifically, an operator-selectable fractional-inch
  denominator — 1/8, 1/16, 1/32 — the concrete, well-evidenced
  exceed-Acrobat opportunity `measure__units_and_number_format.md`
  names**), with its OWN Apply/Cancel — matching `properties_window`'s
  dirty-check discipline (`Apply`/`Revert` greyed when the draft already
  matches, per the Pass 3.1-carried rule) but at PER-GROUP granularity,
  since groups are independent records the way font-folder entries are
  independent list items, not one shared record the way `/Info` fields
  are.
- **Deleting a group** requires an explicit confirm (its member count is
  always shown right there, so the operator sees the blast radius before
  confirming) and REASSIGNS members to the default/active group rather
  than silently orphaning them — disclosed in the confirm prompt itself
  ("12 dimensions will move to the default group"), never a silent data
  loss.

### 5.3 A default group always exists

Matching decision 011's own instruction ("A default/active group always
exists so a dimension has a home") — the panel never shows an empty
list; a freshly-opened document with zero authored dimensions still has
one group, named "Default," scale never-set (§4.3's first row),
un-hideable (no `👁`/`⊘` toggle on the one group every dimension could
conceivably fall back to — hiding the ONLY group a stray dimension might
belong to is a foot-gun with no benefit).

### 5.4 Binding core ask

Group CRUD (create/rename/set-scale/set-units/set-format/delete-and-
reassign/toggle-visibility) each needs an `EditSession` command
(mutating the sidecar + regenerating affected `/AP`s + touching
`/OCProperties`) — exact variant shapes are the engineer's call, per
§0.3. The ONE binding interaction-level constraint: **every one of these
is a single undo step**, matching §4.4's reasoning, so the operator's
mental model ("I renamed a group," "I hid a layer") maps to exactly one
`Ctrl+Z`.

---

## 6. Disclosures (rendered verbatim, per the standing rule)

| Trigger | Disclosure (illustrative wording — core owns the exact string) |
|---|---|
| Snap candidate resolved (routine kind) | *(the indicator glyph/label itself — §2.2; no separate disclosure-strip line needed for the routine case, the indicator IS the disclosure)* |
| Snap candidate is a DERIVED centerline | "Centerline derived from a filled shape (long:short ≈ N:1) — click again to confirm this is a drawn line, not a rectangle." |
| Best-fit circle computed | "Best-fit circle from N objects — radius R, fit residual ε (RMS)." + a warn-coloured pairing when residual is large relative to radius |
| Group has never had a scale set | "No scale set — showing raw page units." |
| Group scale explicitly 1:1 | *(no caveat — a deliberate, disclosed operator choice, §4.3)* |
| Scale-dimension ratio path chosen | "Paper-unit basis: 1 in = 72 pt." (always visible, §4.2) |
| Group scale changed, members re-propagated | "Scale applied to '<name>' — N dimension(s) updated." |
| Group deleted | "N dimension(s) moved to '<default group name>'." |
| Native `/Measure` vs. sidecar disagree on load (decision 011 §2.4) | "This document's stored scale for '<name>' does not match its portable measurement data — using the pdfce-authored value." |
| Chained-dimension snap used (§2.4) | *(no extra disclosure beyond the ordinary "endpoint"/"node" label — it is an ordinary snap candidate once included, not a special-cased fact needing its own callout)* |

Every row above the double rule is a REAL algorithmic inference or a
document-state fact (fuzzy-never-sneaky, rule 4/R20's "status bar is the
narrator" principle) — none is a cosmetic string invented at the GUI
layer without a core-owned fact behind it, matching the standing
"disclosures render core's own text verbatim" rule.

---

## 7. Discoverability / accessibility / crash-safety

### 7.1 Discoverability

- `Measure ▾`'s dynamic label (§1.2) always shows the active tool by
  name — an operator returning to the app after a break sees at a
  glance whether a measure tool is still engaged, without opening the
  menu.
- Every menu row and toolbar control has a tooltip stating not just
  what it does but WHEN to use it (per this agent's own discoverability
  checklist) — e.g. the Radius/Diameter row's tooltip: "Fit a circle to
  a drawn circle, or to several selected line-segment objects forming
  an arc." (illustrative; final copy is the engineer's/`ui_text.rs`'s
  call, following the catalog's existing voice).
- The Group panel's per-row member count and scale summary make its
  state legible without opening anything (matching Font Folders' own
  "active indicator" convention).

### 7.2 Keyboard / accessibility

- Accept/Reject/Apply/Cancel are ALL real `ui.button`s (accesskit,
  Tab-focusable) — never painter-drawn, matching every prior tool.
- The H/V/Aligned, Radius/Diameter, and scale-entry-path segmented
  controls are real `selectable_value` widgets (Tab-reachable), not
  hand-rolled painter regions — the SAME accessibility win `pass-12.0`
  §6.4 flagged as forward guidance for exactly this kind of control.
- **Numeric entry (real-length value, ratio numerator/denominator, unit
  dropdowns, precision) is entirely keyboard-driven `DragValue`/
  `ComboBox`** — an operator can set a group's scale WITHOUT ever
  drawing a reference line at all, by opening the Group panel and
  typing directly into a group's inline edit form. This is a genuine,
  buildable accessibility win over the on-canvas-only entry path
  (mirrors `pass-16.2`'s own "typed-coordinate placement, no mouse drag
  required" precedent) — flagged as a deliberate win, not a mitigation
  of an unfixable gap.
- **Known egui/accesskit gap, inherited, not newly created:** the
  canvas itself remains a raster image with no screen-reader-legible
  content (`pass-12.0` §6.4's own standing note) — a dimension's VALUE
  is only available via the property bar/disclosure strip's real text
  widgets (which ARE accessible), never via the canvas drawing alone.
  This Pass does not close that gap; it does not widen it either, by
  routing every number an operator needs through real, accessible text.
- Snap-cycle-via-Tab (§2.4) is itself a keyboard affordance, not a
  mouse-only mechanic — an operator who cannot reliably mouse-hover a
  tight cluster of nearby geometry can still reach every candidate.

### 7.3 Crash-safety of an in-progress dimension or scale calibration

Nothing in §1–5 writes to `EditSession` before Accept (§8 confirms this
per-row) — an in-progress pick, fit, or scale entry is PURE session/view
state, covered by the SAME crash-recovery posture every other in-progress
gesture in this app already has (the standing autosave/recovery
discipline, rule 5): a crash mid-pick loses only the uncommitted gesture,
never corrupts the document, and the next autosave snapshot is
unaffected because nothing changed. No NEW crash-safety mechanism is
needed; this Pass simply must not accidentally become the first tool
that writes early (the same discipline `pass-6.1`'s `GestureInterrupt::
Discard` already enforces structurally).

---

## 8. Undo / write-path summary

| Operation | `EditSession` command? | Writes anything? |
|---|---|---|
| Selecting a Measure tool / opening the menu | No | No |
| In-progress pick (point A, cycling snap candidates, Tab/Alt) | No | No — pure view state |
| Confirming a derived centerline (§2.3.1) | No | No — still pre-Accept |
| Building/re-fitting the circular pick-set | No | No — `MeasureCircularState` only |
| Reject (any of the three tools) | No | No — nothing was ever written (rule 7) |
| **Accept a linear/circular dimension** | **Yes, one command** | **Yes — one authored `/Line` `/IT /LineDimension` annotation + sidecar entry** |
| **Accept a scale entry** | **Yes, one command** | **Yes — the group's scale + every member's regenerated `/AP` + sidecar** |
| Group rename/scale-edit/units/format/delete/visibility-toggle (§5) | **Yes, one command each** | **Yes — sidecar + (for scale/format) regenerated `/AP`s + (for delete) reassigned members** |
| Escape (any stage) | No | No — falls to the existing four-way precedence, unchanged |

Every row above the two bold "Accept" rows and the Group-panel row is
"no edit, nothing written" — matching every prior tool-bearing spec's own
table shape (`pass-16.2` §8, `pass-15.2`'s reflow table).

---

## 9. `ui_text.rs` catalog — new entries (names, not final copy; final wording is the engineer's/`ui_text.rs`'s own voice, per the catalog's existing style)

- `measure_menu_button(active: Option<MeasureToolLabel>) -> String`
- `measure_menu_tooltip() -> &'static str` — states when to reach for
  this vs. Markup ▾ (R78-style disambiguation, since both authoring
  toolbars now touch annotation-shaped output)
- `measure_linear_menu_item()`, `measure_circular_menu_item()`,
  `measure_set_scale_menu_item()`, `measure_manage_groups_menu_item()`
- `snap_indicator_label(kind: SnapKind) -> &'static str` (the seven
  routine labels, §2.2) + `derived_centerline_label() -> &'static str`
  (the distinct eighth, §2.3.1)
- `axis_constraint_label(c: AxisConstraint) -> &'static str` (Aligned/
  Horizontal/Vertical)
- `scale_entry_real_length_label()`, `scale_entry_ratio_label()`,
  `scale_entry_paper_unit_caption()`, `scale_entry_preview(scale: &str)
  -> String`
- `group_picker_new_group_item()`, `group_manager_window_title()`,
  `group_row_summary(name, scale_summary, dim_count) -> String`,
  `group_delete_confirm(name, dim_count) -> String`
- `no_scale_set_disclosure()`, `scale_explicit_one_to_one_note()`
  (illustrative — may not need its own string if §4.3's "no caveat"
  design is followed literally)
- `best_fit_circle_disclosure(count, radius, residual) -> String`,
  `best_fit_residual_high_warning()`
- `fractional_inch_denominator_label()` (§5.2's ft-in exceed-Acrobat
  control)

No entry duplicates or shadows an existing string — verify at
implementation time via the same grep-before-adding discipline
`pass-16.2` §0 used to catch its own `text_menu_tooltip()` collision.

---

## 10. Consolidated core/GUI accessor asks (gathered from §2–§5, for the engineer's tracking)

1. **12.M1 — `SnapKind`/`SnapCandidate`/`snap_candidates`, returning the
   FULL tied candidate list (not just the winner), sorted priority-then-
   nearest** (§2.4). **Binding — cycle/override is impossible without it.**
2. **12.M1/12.M2 — `AxisConstraint`/`constrained_second_point`**, pure,
   recommend placing in 12.M2 alongside the other dimension-geometry
   pure functions since it is a measurement concern, not a snap-target
   concern (§2.5).
3. **12.M2 — `fit_circle_taubin(points: &[Point]) -> Option<FitCircle>`**
   over an arbitrary point set (§3.3) — decision 011 already specifies
   the algorithm; this names the exact call shape the GUI needs.
4. **`pdfce-gui` wiring — `ObjectModelProvider::page_objects()` (or
   equivalent same-crate accessor) + an `OpenDoc`-level concretely-typed
   handle to the current page's decomposition**, so the circular-fit tool
   (and the centerline-candidate overlay, and eventually the snap query)
   reuse the ONE decomposition already built for selection, never a
   second `decompose_page` call per frame (§3.3). **Not a `pdfce-core`
   change — a same-crate wiring fix.**
5. **9a/12.M1 — `PageObjects`'s construction inputs should be extended
   to include the CURRENT page's already-authored dimension annotations'
   own geometry**, if the chained-dimension-snapping recommendation
   (§0.2/§2.4) is adopted — flagged as a real, non-trivial scope item
   (dimension annotations are a new object KIND, not page-content-stream
   geometry), not assumed free.
6. **12.M2 — `preview_group_scale`/the `EditSession` scale-commit
   sibling**, mirroring the `edit_text`/`format_text` and `reflow_block`
   preview-then-commit precedent exactly (§4.5).
7. **12.M2 — the tri-state scale representation** (never-set / explicit-
   1:1 / calibrated) must be a REAL, distinguishable stored state, not
   collapsed to `Option<f64>` where `None` and `Some(1.0)` would be
   indistinguishable from "never touched" vs. "deliberately 1:1" (§4.3).
   **Binding — this is a data-model correctness point, not a display
   nicety**, since §4.3's whole design depends on the two being
   genuinely different facts.
8. **12.M2 — group CRUD commands (create/rename/set-scale/set-units/
   set-format incl. ft-in fractional-inch denominator/delete-and-
   reassign/toggle-visibility), each ONE undo step** (§5.4).

---

## 11. Priority table

| Item | Priority | Note |
|---|---|---|
| `CanvasTool::{MeasureLinear, MeasureCircular, MeasureScale}` + predicates (§1.1) | **P0** | |
| `Measure ▾` menu, dynamic label, `Ctrl+Shift+D` chord for Linear only (§1.2) | **P0** | Verify chord unclaimed at implementation time |
| Fuzzy snap indicator rendering (glyph+label, §2.2) | **P0** | Depends on ask #1 |
| Derived-centerline distinct glyph + two-click confirm (§2.3.1) | **P0** | Fuzzy-never-sneaky centerpiece |
| Cycle (Tab) / override (Alt) / master toggle (§2.4) | **P0** | Depends on ask #1 |
| H/V/Aligned segmented control + `constrained_second_point` (§2.5) | **P0** | Ask #2 |
| Property bar + status/disclosure strip, reusing existing Area convention (§2.6) | **P0** | |
| `MeasureCircular` own pick-set + best-fit preview/Accept (§3) | **P0** | Depends on asks #3, #4 |
| Scale-entry sub-panel, both paths, tri-state (§4) | **P0** | Depends on asks #6, #7 |
| Group panel: list + per-row inline edit + visibility toggle (§5) | **P0** | Depends on ask #8 |
| Fractional-inch denominator control (§5.2) | **P1** | Named exceed-Acrobat opportunity, not required for a functioning beta |
| Chained-dimension snapping (§0.2/§2.4) | **P1** | A pdfce origination, not required for parity; depends on ask #5 |
| `ui_text.rs` catalog entries (§9) | **P0** | Alongside each control they label |

**No cut below P0 leaves a coherent, honest beta behind** — per decision
011's own framing, the additive dimensioning half either ships complete
(linear + circular + scale + groups, since a group without a working
scale-entry path is not a usable feature) or the two P1 items are the
only defensible trim.

---

## 12. Open items for the librarian

1. **Three `CanvasTool` variants, not four** (§1.1) — radius/diameter
   collapsed into one `MeasureCircular` tool with a display-only toggle,
   a deliberate reduction from decision 011's literal four-capability
   list. Worth a one-line note on decision 011's own record so a future
   reader does not go looking for a fourth tool variant that was
   intentionally never built.
2. **The Group panel is the SAME "edit → window" taxonomy bucket as
   Properties, not a new instance** (§5.1) — explicitly recorded so no
   future session invents (or goes looking for) an eighth taxonomy
   entry for it.
3. **`Measure ▾` is a NEW combination of two existing precedents**
   (Markup ▾'s menu widget + Edit Text/Add Text's `SelectCanvasTool`
   dispatch) — flagged so a future reader does not mistake it for either
   verbatim, and so a future Pass 6.1 real-drawing-tool build (which will
   need the SAME live-pick dispatch Markup ▾ currently lacks) can reuse
   this exact combination rather than re-deriving it.
4. **The consolidated core/GUI accessor asks in §10** are the load-
   bearing gaps this spec found beyond decision 011's own Appendix A —
   worth a pointer from decision 011's record to this file, the same
   courtesy `pass-12.0` §10 extended to decision 011 for the
   `canvas_to_pdf_space` finding.
5. **Chained-dimension snapping (§0.2) is a named pdfce origination**,
   not a Acrobat-parity claim — flag to `pdfce-acrobat-librarian` that
   this is now a DECIDED pdfce design point (include prior dimensions'
   geometry as snap targets), not an open question, if that RAG's own
   file is ever revisited.
