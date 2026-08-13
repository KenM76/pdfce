# Pass 68.0 UI Spec — the Two-Line ce-Dimension Gesture (GUI half)

> Authored by `pdfce-ui-specialist`, on dispatch from the engineer. Core
> (`e931836`, `905791f`) and CLI (`bc13a86`) are shipped and tested; **the
> GUI has zero path to this capability today** — no operator working in the
> canvas can produce an Angular ce dimension, which is the literal, verbatim
> form the operator's request took. This spec is that missing slice, and
> only that slice: nothing in §§1–19 below asks for a change to
> `pdfce-core`'s already-shipped classification, authoring, or storage — the
> one binding "core" ask this spec makes (§11.2) is a GUI-side extension to
> an existing drag handler, not new geometry.
>
> Read before implementing: `crates/pdfce-core/src/vector/linepick.rs`,
> `crates/pdfce-core/src/dimension/two_lines.rs` (both **in full** — read
> below, not skimmed, because §11's design leans on exact field semantics
> from both), `crates/pdfce-gui/src/{canvas.rs, measure_tool.rs}`, and
> `crates/pdfce-gui/src/main.rs`'s `run_measure_tool`/`measure_options_ui`/
> `measure_status_ui`/`run_dimension_drag` (all four, in full — this spec
> extends each one by name); `docs/ui_specs/pass-46-canvas-interaction-
> model.md` §1–§2 (the `CanvasTool` contract); `docs/ui_specs/pass-12.M2-
> dimension-tools.md` (the shipped Linear/Circular/Scale precedent this spec
> extends rather than re-derives); `docs/ui_specs/tool-options-dock-and-ce-
> dimension-properties.md`; `UI_PREFERENCES.md` (repo root, §1/§4, the
> chrome-vs-canvas-overlay token distinction); `docs/ARCHITECTURE.md` §12
> (decision 024 §4.4, the narrowed rule-4 disclosure obligation this spec
> is built around).

**Terminology (project rule 15), binding throughout.** Every dimension
object this spec discusses that pdfce authors, edits, or draws is a **ce
dimension** (`/Line` + `/IT /LineDimension` or the new Angular geometry,
plus `/Measure` and the `/PieceInfo` sidecar — everything under
`crates/pdfce-core/src/dimension/`). Pre-existing CAD-exported callouts —
**pdf dimensions** — enter this feature only as the pickable page geometry
`pick_line` reads; nothing here writes to or alters them.

---

## 0. What already exists, and the one function the GUI must call

### 0.1 Shipped, and not re-derived here

Confirmed by reading the code directly, not the ROADMAP's prose about it
(the ROADMAP entry for `bc13a86` describes an earlier, now-superseded
inline CLI implementation; `crates/pdfce-core/src/dimension/two_lines.rs`
is the current, canonical, shell-agnostic surface and is what this spec
designs against):

- **`pdfce_core::vector::linepick`** — `PickedLine`, `ParallelPolicy`,
  `TwoLineRelation`, `pick_line`, `classify_two_lines`,
  `measured_angle_degrees`. The geometric question only: parallel, angled,
  or collinear, and which of the four angles a pick point selects.
- **`pdfce_core::dimension::two_lines`** — `author_from_two_lines`,
  `TwoLinePlacement`, `TwoLineAuthoring`, `TwoLineRefusal`. The *authoring*
  question: what `DimensionKind` a relation becomes, the arc-radius
  fallback, the linear-normal sign convention, and the two named refusals.
  Its own module docs state plainly why this is one function, not one per
  shell: *"That would have been the wrong instrument... Two copies of that
  is how the CLI and the GUI come to author visibly different ce
  dimensions from the same two clicks."* **This spec's single most
  important technical instruction is: call this function. Do not
  re-derive any part of what it does inside `pdfce-gui`.**
- **`DimensionKind::Angular { apex, dir_a, dir_b, radius, text_along }`**
  — the third ce-dimension kind, does not scale, already has its own
  `format_angle_degrees`/display branch, already has a `SIDECAR_VERSION`
  bump so an old build cannot silently drop it.
- **`pdfce-cli dimension-add --kind two-lines`** — the CLI's authoring
  surface, `--treat-as-parallel` for the override, discloses
  `measured_angle`/`forced`/`authored` on every run.

### 0.2 Not shipped, and this spec is exactly the gap

- No `pdfce-gui` code calls `pick_line` or `author_from_two_lines`
  anywhere. `crates/pdfce-gui/src/canvas.rs`'s `CanvasTool` enum has no
  concept of picking a line rather than a point.
- `crates/pdfce-gui/src/main.rs`'s `run_dimension_drag` — the existing
  drag-to-reposition gesture for an already-authored ce dimension — filters
  to `DimensionKind::Linear` only (§11.2 below names this precisely; it
  predates `Angular` by definition, since `Angular` did not exist when it
  was written).
- `FEATURES.md` therefore still reads core `[x]` · cli `[x]` · gui `[ ]`
  for this capability until this spec ships.

### 0.3 `author_from_two_lines`'s contract, restated for this spec's own use

```rust
pub struct TwoLinePlacement {
    pub constraint: AxisConstraint, // ignored for an angular result
    pub offset: f64,                // linear standoff, OR angular arc radius
                                     // (0.0 ⇒ "choose a readable default")
    pub text_along: f64,            // points (linear) or degrees (angular)
}

pub struct TwoLineAuthoring {
    pub kind: DimensionKind,                  // what to hand to add_dimension
    pub relation: TwoLineRelation,             // never Collinear here
    pub measured_angle_degrees: Option<f64>,   // ALWAYS present, even forced
    pub forced_parallel: bool,
}

pub enum TwoLineRefusal { Collinear, Degenerate } // both `thiserror`, both
                                                    // carry a ready operator-
                                                    // facing message

pub fn author_from_two_lines(
    a: &PickedLine, b: &PickedLine,
    policy: ParallelPolicy, placement: TwoLinePlacement,
) -> Result<TwoLineAuthoring, TwoLineRefusal>;
```

Three properties of this contract drive most of this spec's design and are
worth stating up front rather than re-discovering per section:

1. **`TwoLinePlacement::default()` is already a usable result** — `offset:
   0.0, text_along: 0.0`. For a Linear result this reproduces the ordinary
   tool's own "through point A, centred text" default; for an Angular
   result, `offset: 0.0` triggers a **derived** arc radius (half the
   shorter picked arm, floored at 20 pt so a short pick is still
   clickable). **This means the GUI does not need a third-click
   placement gesture to ship a usable result** — Accept can fire the
   instant a valid, non-refused verdict exists. §11 covers what a
   placement gesture would add on top, as a named P1, not a P0
   dependency.
2. **`measured_angle_degrees` is always populated**, forced or not — this
   is what makes the verdict disclosure honest without the GUI
   re-measuring anything itself (§6).
3. **Neither refusal is a diagnostic string invented by this spec** — both
   are `thiserror` messages already written to be operator-facing. §7
   renders them **verbatim**, the same convention `run_measure_tool`
   already uses for Circular/Scale's `Err(err) => ui_text::refusal_line(&
   err.to_string())`.

---

## 1. Decision (a) — a PICK MODE of `CanvasTool::MeasureLinear`, not a new tool

### 1.1 The question, and why it is close enough to argue rather than assert

The brief asks this explicitly, and it deserves a real argument, because
the precedent set in this codebase cuts both ways depending on which axis
you weigh. Three prior decisions bear on it directly:

- **`AddText` vs. a `TextEdit` sub-mode** (Pass 16.2): decided **separate
  tool**, because a plain click's MEANING would otherwise be silently
  repurposed — "sometimes creates page content" — based on invisible
  state.
- **Reflow vs. a `TextEdit` sub-mode** (Pass 15.2): decided **sub-mode**,
  the opposite way, because a plain click's meaning did NOT change enough
  to justify a whole new armed tool.
- **`MarkupKind`'s ten kinds under ONE `CanvasTool::Markup`** (Pass 46):
  decided **one tool, an explicit kind selector**, even though the GESTURE
  SHAPE differs by kind (`Square`/`Circle` drag a rect; `Line` drags
  between two endpoints) — because the operator explicitly picks the kind
  *before* drawing, so nothing is silently repurposed; only the shape of
  an already-declared intent varies.

Two-line picking is the `MarkupKind` case, not the `AddText` case, and the
distinguishing test is the same one Pass 46 already applied: **does the
operator explicitly declare the mode before it changes what a click does,
or does the click's meaning depend on state they cannot see?** A segmented
"Pick: Two Points | Two Lines" control in Tool Options, visible the whole
time the tool is armed, is exactly that explicit declaration — no click
anywhere in this design ever means two different things based on
something the operator would have to infer.

### 1.2 The concrete case for THIS tool, not a new one

- **The operator's own wording is textual evidence, not just a vibe**:
  *"the dimensioning tool should allow the selection of two lines"* —
  singular, "the," referring to the tool that already exists. A brand-new
  armed tool would be answering a request the operator did not make.
- **Every piece of shared plumbing is genuinely shared, not
  coincidentally similar**: the active dimension GROUP, the `snap_master`
  toggle's existence as a concept (even though §3 turns it off for this
  mode specifically — see why below), the Accept/Reject strip, the
  disclosure area, the Tool Options pane it draws in, the `Ctrl+Shift+D`
  discoverability path. Splitting these across two `CanvasTool` variants
  would either duplicate all of it or require the two tools to reach
  into each other's state — worse than the mode switch it would replace.
- **The output is not fixed to one dimension family**, which argues
  AGAINST a name like `MeasureAngle`: the same gesture can author either
  `Linear` or `Angular` depending on the geometry, decided by pdfce, not
  by which tool the operator armed. A tool named for one specific output
  kind would be lying about what it does roughly half the time on a
  drawing with a mix of parallel and angled edges.
- **This directly mirrors how a "smart dimension" tool behaves in the
  reference workflow the operator invoked by name** (SolidWorks): one
  tool, the picked entities decide the dimension type. That is a
  capability fact sourced from the operator's own analogy, not a GUI
  mechanic copied from anywhere (rule 12/R61 stay satisfied — nothing
  here reads Acrobat's or SolidWorks' own screen layout, only the
  behavior the operator named).

### 1.3 Required doc-comment amendment — a naming-honesty fix, not a rename

`canvas.rs`'s `CanvasTool::MeasureLinear` doc comment currently reads:

> *"Linear dimension (Pass 12.M2, ui-spec §1.1): two snapped point picks
> author a scaled measurement. A plain click while this tool is active is
> always a point-pick, never an object-selection click."*

The second sentence becomes false the moment Two-Lines mode ships — a
plain click in that mode is a LINE-pick (`pick_line`), which additionally
can miss (empty space, a curve) in a way a raw point-pick never does. The
variant name `MeasureLinear` does **not** need to change — precisely
because of §1.2's own reasoning, it is the entry point for the whole
"linear dimension family," of which this is now a second pick METHOD, not
a second output kind. But the doc comment needs a paragraph naming the
two pick modes and stating which invariant now holds per-mode rather than
per-tool. Write it; do not leave the stale sentence standing, since it is
exactly the kind of comment a future reader would reason from incorrectly
(the same failure class `Pass 68.0`'s own `905791f` commit fixed once
already for a version-gate comment — "a comment that misdescribes a
compatibility gate is worse than no comment").

`TOOL_PRECEDENCE` needs **no reordering** — the mode lives entirely inside
`MeasureLinear`'s own state, so its position in the array (already correct,
per Pass 46's placement reasoning) is unaffected.

---

## 2. New state — `LinearPickMode` + `TwoLinePick`, and why NOT `pending`

### 2.1 The concrete hazard of the obvious-looking shortcut

`MeasureState::pending: Option<DimensionKind>` looks, at first glance, like
exactly the right home for a completed two-line classification — it is
already "the linear tool's completed-but-not-yet-authored dimension." It
is the **wrong** home, and the reason is a real, already-shipped piece of
machinery this spec would otherwise silently break:

```rust
// main.rs, PdfceApp::committable_gesture — VERBATIM, shipped, tested:
let measure = doc.active_tool() == Some(CanvasTool::MeasureLinear)
    && doc.measure.as_ref().is_some_and(|s| s.pending.is_some());
```

This is decision 031's Pass-34.0 Commit-on-interrupt path: a completed
ordinary two-point pick is safe to commit automatically if some OTHER
action interrupts the gesture (switching tools, closing the document),
because nothing about a raw two-point pick is inferred — it is exactly
what the operator clicked. The SAME function's own comment states the
opposite rule for the tool one line below it: *"Circular (best-fit =
inferred) and scale (blast radius) are deliberately excluded."*

A two-line classification is inferred in exactly Circular's sense —
parallel-vs-angled, which of four angles, a possibly-virtual apex are all
decisions pdfce made by reading geometry, not values the operator typed or
clicked directly. **If a completed two-line verdict were stored in
`pending`, `committable_gesture()`'s existing `is_some()` check would
silently make it interrupt-committable too**, because that check has no
way to distinguish "an ordinary two-point pick happened to land in
`pending`" from "an inferred classification happened to land in
`pending`" — it only ever asked whether the field was populated. That
would reopen, for this one gesture, precisely the hazard decision 031
closed for Circular: an inference committing itself because the operator
happened to alt-tab away mid-review.

### 2.2 The fix — a sibling to `CircularPick`, not a reuse of `pending`

```rust
/// Which geometry `MeasureLinear`'s next commit targets (ui-spec §1).
///
/// A real change in what a click MEANS — `Points` resolves any snap
/// candidate anywhere on the page; `TwoLines` calls `pick_line` and
/// requires landing on straight, already-drawn geometry, refusing curves
/// and misses rather than inventing a point. Same class of mid-tool mode
/// change `MarkupKind` already makes (ui-spec pass-46 §1.2), so switching
/// modes DISCARDS whatever pick is in progress first — free, because
/// nothing has committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LinearPickMode {
    #[default]
    Points,
    TwoLines,
}

/// The two-line pick state (ui-spec §2) — click line A, click line B,
/// review pdfce's classification, then Accept or Reject.
///
/// Deliberately its OWN field, siblings with `linear`/`circular`/`scale`,
/// NOT folded into `MeasureState::pending` — see the module's own §2.1 for
/// why: an inferred classification staying out of `pending` is what keeps
/// `committable_gesture()`'s existing, already-tested rule correct without
/// that function needing to learn a new distinction.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TwoLinePick {
    /// The first picked line, page space. `None` while awaiting it.
    pub first: Option<PickedLine>,
    /// The second picked line, once chosen. Picking a THIRD line while
    /// both are set is ignored (ui-spec §4 — matches `pending`'s own
    /// documented "further picks are ignored, the operator is
    /// reviewing" rule, not a new convention).
    pub second: Option<PickedLine>,
    /// The operator's per-pair "treat as parallel" override (ui-spec §9).
    /// Re-consulted live on every toggle while `second` is `Some` — never
    /// applied to an already-authored ce dimension (§9.3: the source
    /// lines are not retained once authored, so there is nothing later to
    /// re-run this against).
    pub force_parallel: bool,
    /// The live classification result, recomputed whenever `second` or
    /// `force_parallel` changes (never cached stale across a toggle).
    /// `Err` retains `first`, clears `second` (ui-spec §7) so the operator
    /// can re-pick line B without losing line A.
    pub verdict: Option<Result<TwoLineAuthoring, TwoLineRefusal>>,
}

impl TwoLinePick {
    #[must_use]
    pub fn in_progress(&self) -> bool {
        self.first.is_some() || self.second.is_some()
    }
    pub fn clear(&mut self) {
        self.first = None;
        self.second = None;
        self.verdict = None;
        // force_parallel is a per-pick preference the operator likely
        // wants to keep for their next pick in the same session — NOT
        // reset here (mirrors `snap_master`/`group` surviving
        // `clear_gesture`, per `MeasureState::clear_gesture`'s own doc).
    }
}
```

`MeasureState` gains `pub linear_pick_mode: LinearPickMode` and `pub
two_lines: TwoLinePick`. `MeasureState::gesture_in_progress` gains one
disjunct: `|| self.two_lines.in_progress()`.

---

## 3. Tool Options layout — `measure_options_ui`, amended for `MeasureLinear`

The mode toggle goes **first**, immediately under the existing Close-row
and above the group picker — the highest-discoverability position, and the
one control whose state changes what every row below it means:

```
[ Close ]                                              (existing, right-aligned)
Linear Dimension                                        (existing hint header)
Pick:  ( Two Points )  ( Two Lines )                    NEW — selectable_value pair

Group: [ Floor Plan ▾ ]  [ Groups… ]                    (existing, unchanged, both modes)

--- Points mode only, existing, unchanged ---
☑ Snap to content
Alignment:  ( Aligned ) ( Horizontal ) ( Vertical )

--- Two Lines mode only, NEW ---
Click a straight line to pick it. Curves and empty space
aren't picked — pdfce won't guess where the line is.
```

**Why `Snap to content`/Tab-cycle/Alt-suppress/`Alignment` are HIDDEN, not
merely inert, in `TwoLines` mode**: these controls govern
`snap_candidates` — POINT snapping to nodes/endpoints/centers/midpoints/
intersections — an entirely different mechanism from `pick_line`'s
nearest-straight-segment search. Showing a "Snap to content" checkbox
that is never consulted in this mode is a real discoverability defect:
an operator would reasonably read it as governing the very picks they are
making and wonder why toggling it does nothing. Hiding it is the correct
reading of rule 3 for a mode-specific control the way `MeasureCircular`'s
own panel already hides `Alignment` (never shown for that tool at all,
confirmed by the existing `if canvas::tool_builds_measure_linear(active)
|| canvas::tool_builds_measure_scale(active)` gate) — this spec extends
that same gating principle one level deeper, inside `MeasureLinear`
itself, rather than inventing a new one.

The `Two Points`/`Two Lines` `selectable_value` pair uses the identical
widget every other segmented control in this pane already uses
(`Alignment`, `Display: Radius/Diameter`) — no new widget type, per rule
6's Tab-focusability requirement and the project's own consistency
discipline.

---

## 4. The gesture state machine

| Stage | Plain click on a pickable line | Plain click on empty space / a curve | Escape | Toggling `force_parallel` |
|---|---|---|---|---|
| Nothing picked (`first: None`) | Sets `first`. Draws the picked line in preview-orange. | No-op — nothing changes, no message (§4.1). | `resolve_escape`: no gesture in progress → `ExitTool` (disarms `MeasureLinear`, same as every other tool with nothing pending). | N/A — checkbox not shown yet (§9.1). |
| Line A picked, B not (`first: Some, second: None`) | Sets `second`, calls `author_from_two_lines` immediately, sets `verdict`. | No-op — line A stays picked, hover highlight simply shows nothing over the miss. | `CancelGesture`: clears `first` (full reset to "nothing picked"). Stays in `MeasureLinear`, stays in `TwoLines` mode (the mode is a Tool Options setting, not part of the gesture — clearing a gesture never reverts it, matching how `snap_master`/`group` survive `clear_gesture`). | N/A |
| Both picked, verdict is `Ok(...)` (a valid, reviewable dimension) | **Ignored** — matches `pending`'s own documented "further picks are ignored, the operator is reviewing" rule (§2.2). The operator must Accept or Reject/Escape first. | No-op. | `CancelGesture`: clears both picks and the verdict. | Re-runs `author_from_two_lines` with the new policy, live (§9.2). |
| Both picked, verdict is `Err(Collinear)` or `Err(Degenerate)` | Sets `second` to the NEW line (replaces the rejected one), re-classifies. This is the natural "try a different line B" recovery — not a special case, just `second` being reassigned normally. | No-op — line A and the refusal disclosure both stay visible. | `CancelGesture`: clears `first`, `second`, and the verdict (full reset — same outcome as the "Line A picked" row, since a refused pair has nothing worth partially preserving beyond A, which the operator can re-obtain with one click if they want it back). | Re-runs immediately — a refusal can become a valid verdict this way (e.g. two lines the auto-threshold read as angled, forced parallel). |

### 4.1 Why a miss produces no disclosure at all

`pick_line` returning `None` — the pointer is over empty space, or over a
curve — is not treated as a refusal. It produces **no message**, on the
hover pass or the click pass, because a per-attempt "that wasn't a line"
toast for every ordinary miss during normal mouse movement across a
drawing would be exactly the "annoying" outcome the brief asks this spec
to avoid. The single, calm, ALWAYS-visible hint line in Tool Options
(§3's bottom line, matching `measure_linear_hint()`'s own existing
always-shown-while-armed convention) is the whole answer to "how is this
disclosed without being annoying": stated once, standing, never repeated
per click.

### 4.2 Why no special-case guard is needed for "picked the same line twice"

If the operator's second click lands on the identical segment as their
first (same `object_index`/`subpath`/`segment`), `classify_two_lines`
already handles it correctly with no new code: `start`/`end`/`pick` are
identical, the perpendicular distance comes out at (or within floating-
point noise of) zero, and the existing `distance <= scale * 1e-6` test in
`classify_two_lines` reports `Collinear`. The Collinear refusal path
(§4's last row) already recovers gracefully — no bespoke "you picked the
same line twice" message is required, and none should be added; it would
duplicate a case the geometry already answers correctly.

---

## 5. Hover feedback — before any click, per rule 4's "shown before every commit" reading

Every frame `TwoLines` mode is active and the pointer is over the canvas,
call `pick_line` at the pointer position (page-space, tolerance converted
from the same `SNAP_SCREEN_TOLERANCE_PX`/zoom conversion the point-snap
path already uses — `canvas::screen_tolerance_to_page`, reused verbatim,
no new tolerance constant). If it returns `Some`:

- Draw the **whole picked segment** (start→end, not just the click point)
  with a 2.0-pt stroke in `theme.palette.node_mark` (blue — "a point is
  here" generalized to "a pickable edge is here"; the same semantic
  register the existing vector-node marks already use for "this is
  something you could interact with," per `UI_PREFERENCES.md` §4's
  established blue-means-editable convention).
- Draw a small text label `"line"` beside the pick point, via the exact
  same `painter.text(sp + vec2(9.0, -2.0), Align2::LEFT_CENTER, …,
  FontId::proportional(11.0), text_color)` call the existing snap
  indicator already uses (§2.2 of pass-12.M2's spec) — reused pattern, no
  new painter convention.

Once `first` is set, that picked line switches to `theme.palette.preview`
(orange — "part of my in-progress gesture," the same role every other
in-progress-pick preview already uses) and STAYS drawn that color while
the operator hovers for line B; the hover highlight (blue) continues to
track whatever line is currently under the pointer for the second pick,
independently.

**No live "what-if" verdict preview against the hover candidate before an
actual second click.** This would be a genuine nice-to-have (seeing the
parallel/angled reading update as the pointer sweeps past candidate
second lines, before committing to one) but is a real, separate feature —
named here explicitly as a P2 idea, not built into this spec, so a future
reader does not wonder why it is missing rather than assume it was
considered and rejected for a reason.

---

## 6. The verdict + disclosure surface

### 6.1 Where and when

The verdict draws in the **same Tool Options / disclosure area** every
other measure tool already uses (`measure_status_ui`) — never a
floating, page-relative box (the exact thing decision 024 §4.4 forbids by
name, and the exact complaint the operator filed on 2026-08-04). It
appears **immediately** on the second valid pick — there is no reason to
delay it, since `author_from_two_lines` is a pure, cheap function and the
whole point of showing it before Accept is available is rule 4's "visible
before it becomes document state" requirement.

### 6.2 Composed vs. verbatim — a real distinction this spec keeps precise

Two different disclosure classes appear here, and they follow two
different rules that already exist elsewhere in this codebase — this spec
does not invent a third:

- **The refusal messages (`TwoLineRefusal::Collinear`/`Degenerate`) are
  rendered VERBATIM** via `ui_text::refusal_line(&err.to_string())` — the
  exact call already used for Circular's and Scale's `Err` arms in
  `run_measure_tool`'s Phase C. These are `thiserror` strings core
  already wrote to be operator-facing; paraphrasing them would risk
  drifting from the CLI's own identical wording for the identical
  refusal.
- **The successful verdict sentence is GUI-COMPOSED from core's
  structured `Ok` fields** — `measured_angle_degrees`, `relation`,
  `forced_parallel` — the same way `measure_length_readout`/
  `best_fit_circle_disclosure` already compose a sentence from
  `radius`/`residual`/`count` rather than rendering a string core owns.
  Core does not hand back a ready sentence for the success case (only for
  the two refusals), so this spec's new `ui_text` functions build one, in
  the exact voice the brief itself modeled:

```rust
// ui_text.rs — new, composed from TwoLineAuthoring's fields, not a
// verbatim core string (§6.2 distinguishes this from the refusal case).

/// The parallel-reading verdict (ui-spec §6.3). `measured` is the TRUE
/// angle even when `forced` — rule 4's disclosure half applied to an
/// operator override, matching `two_lines.rs`'s own module-doc framing
/// verbatim ("a shell that shows the result and hides the measurement is
/// asking the operator to accept a decision while withholding the fact
/// that makes it one").
pub fn two_line_verdict_linear(measured: f64, forced: bool, distance_text: &str) -> String {
    if forced {
        format!(
            "Treating as PARALLEL (you overrode the measured {measured:.1}°) — \
             distance {distance_text}."
        )
    } else {
        format!("Reading these as PARALLEL ({measured:.1}° apart) — distance {distance_text}.")
    }
}

/// The angled-reading verdict (ui-spec §6.3). `angle_text` is pre-formatted
/// through `format_angle_degrees` against the active group's NumberFormat
/// (decimal-marker-only, per `905791f`'s own display rule for an angle),
/// matching `measure_length_readout`'s existing "caller formats, this
/// composes the sentence" split.
pub fn two_line_verdict_angular(angle_text: &str) -> String {
    format!("Reading these as ANGLED — angle {angle_text}.")
}

/// Appended when `apex_is_real == false` (ui-spec §8) — a fact, not a
/// refusal, so it is its own line rather than folded into the verdict
/// sentence above (mirrors how `two_lines.rs`'s CLI-facing message is a
/// SEPARATE eprintln from the `authored=angular` line it follows).
pub fn two_line_virtual_apex_note() -> &'static str {
    "These two lines do not actually meet — the angle is measured at the point where they \
would cross if extended."
}

pub fn two_line_pick_mode_label() -> &'static str { "Pick:" }
pub fn two_line_pick_mode_points_option() -> &'static str { "Two Points" }
pub fn two_line_pick_mode_lines_option() -> &'static str { "Two Lines" }
pub fn two_line_hint() -> &'static str {
    "Click a straight line to pick it. Curves and empty space aren't picked — pdfce won't \
guess where the line is."
}
pub fn two_line_force_parallel_checkbox() -> &'static str { "Treat these two lines as parallel" }
```

(Final wording is `ui_text.rs`'s own voice/call, per the catalog's
standing convention — the above is illustrative, not to be lifted
character-for-character, matching every prior spec's own disclaimer.)

### 6.3 Accept enablement

`measure_status_ui`'s existing `if canvas::tool_builds_measure_linear
(active) { … }` arm needs to branch on `linear_pick_mode`:

- `Points` (existing behavior, unchanged): `can_accept = st.pending.is_some()`.
- `TwoLines` (new): render the verdict per §6.2, then `can_accept =
  matches!(st.two_lines.verdict, Some(Ok(_)))` — Accept stays disabled
  while nothing is picked, while only one line is picked, and while the
  current verdict is a refusal. This is the same "grey until there is
  something legitimate to commit" discipline every other measure tool's
  Accept button already follows.

---

## 7. Collinear / Degenerate refusal — recovery, not a dead end

On `Err(Collinear)` or `Err(Degenerate)`: `first` is retained, `second` is
cleared, and the refusal's verbatim message (§6.2) draws where the verdict
sentence would otherwise be — colored via `ui.visuals().error_fg_color`
(`theme.palette.danger`'s chrome-side equivalent, matching the existing
convention at `main.rs:23575`'s `ui_text::refusal_line`), paired with the
same error glyph convention used elsewhere (rule 6: color is never the
sole signal). The operator's very next click, on a different line, simply
becomes the new `second` and re-classifies — no Escape required to
recover, because nothing about the refusal state prevents an ordinary
line-pick from proceeding (§4's table, third row).

---

## 8. Virtual apex — disclosed, never refused

When `authored.apex_is_real() == Some(false)`, `two_line_virtual_apex_
note()` draws as its own line beneath the verdict sentence, colored via
`ui.visuals().warn_fg_color` — **`theme.palette.notice`, not `danger`**.
This is a deliberate distinction already established in this project's
palette (`UI_PREFERENCES.md` §3's own framing, extended to canvas overlay
by `theme.rs`'s `notice` doc comment: *"something is worth knowing and
nothing is broken"*): a virtual apex is not an error condition — CAD
drawings dimension a virtual intersection routinely, per the module docs
of `two_lines.rs` itself — it is a fact the operator may not have noticed
about their own drawing.

On the canvas, the live preview for an `Angled` verdict with a virtual
apex draws dashed EXTENSION lines from each picked line's own endpoint
out to the apex point, in `theme.palette.guide` (the weaker, "a hint about
a proposal" relative of `preview` — exactly the role `UI_PREFERENCES.md`
§4 already assigns it), so the operator can see WHERE the virtual
intersection is, not just that one exists. This reuses the identical
extension-line drawing convention `MeasureLinear`'s own Points-mode
already established (`draw_seg(ext_a, dim_a)` / `draw_seg(ext_b, dim_b)`
in `run_measure_tool`) — same visual grammar, new geometry.

---

## 9. The "treat as parallel" checkbox

### 9.1 Where it lives, and why not earlier

The checkbox (`TwoLinePick::force_parallel`) draws **only once both lines
are picked** — inside the verdict disclosure area, immediately below the
verdict sentence, never in the Tool Options panel's standing controls
(§3). This is a direct, load-bearing reading of `two_lines.rs`'s own
stated principle: *"a shell that shows the result and hides the
measurement is asking the operator to accept a decision while withholding
the fact that makes it one."* A checkbox shown BEFORE any measured angle
exists would be the same failure in reverse — a control offering to
override a number that is not yet on screen. Showing it exactly where the
measured angle is displayed is what keeps the pairing rule 4 requires
intact.

### 9.2 It is a live, reversible toggle, not a one-shot input

Ticking or unticking the checkbox while both lines are still picked
re-runs `author_from_two_lines` immediately with the updated
`ParallelPolicy` and replaces `verdict` — the preview geometry, the
verdict sentence, and Accept's enabled state all update in the same
frame. This is genuinely "editing" the not-yet-committed decision, in the
most literal sense available: nothing has been written to `EditSession`
yet (§10 confirms this per-row), so there is nothing destructive about
changing one's mind here, and no confirmation step beyond the ordinary
Accept/Reject pair is warranted (rule 7 — no unnecessary friction for a
reversible, pre-commit choice).

### 9.3 What "editing" does NOT mean here, stated precisely and why

The operator's mid-build request used both words: *"When making OR
EDITING a dimension of this type…"* This spec satisfies "making" in full
and satisfies "editing" for exactly the pre-Accept review window described
in §9.2 — **it does not, and structurally cannot, extend to retroactively
reclassifying an ALREADY-AUTHORED ce dimension**, and this is worth
stating as a finding rather than leaving ambiguous:

`DimensionKind::Angular`'s own doc comment states the reason directly —
*"the two lines bound four angles, and which one the operator meant was
decided at pick time... Storing the lines would throw that decision away
... re-deriving a choice the operator already made is how a dimension
silently becomes a different dimension after a scale change."* The
authored `Linear`/`Angular` geometry retains only the RESOLVED result (two
points, or an apex + two arm directions) — never the two source
`PickedLine`s that produced it. `author_from_two_lines` requires exactly
those two `PickedLine`s as input. **There is therefore no data left,
after Accept, from which "treat this already-authored dimension as
parallel instead" could even be computed** — not a scope decision this
spec is declining to make, but a direct consequence of a data-model
decision core already made and documented for an unrelated, good reason
(protecting scale re-derivation from silently changing what a dimension
means).

**What a future Pass COULD build, if this gap matters in practice**: an
already-authored `Angular` ce dimension's `apex`/`dir_a`/`dir_b` do
contain enough information to compute *what a forced-parallel reading
would look like*, via a NEW, distinct calculation (not `author_from_two_
lines`, which needs `PickedLine`s specifically) — but that is a genuinely
new core capability with its own semantics (what "distance" means when
projecting two direction vectors instead of two finite segments), not a
retroactive rerun of this Pass's function. **Recommend flagging this to
the librarian as an explicitly open, not-yet-scoped question** rather than
silently deciding it either way — see §18.

---

## 10. Commit path — explicit Accept only, never click-out

### 10.1 The precedent this follows, by name

`PdfceApp::committable_gesture`'s own comment: *"Circular (best-fit =
inferred) and scale (blast radius) are deliberately excluded"* from
Pass 34.0's click-out/interrupt-commit convenience. A two-line
classification is inferred in the same sense Circular's best-fit is —
this spec's `TwoLinePick` therefore follows Circular's rule, not ordinary
Linear's: **no click-out commit, no commit-on-interrupt, ever** — only
the explicit Accept button in §6.3 commits it. Because `TwoLinePick`
never touches `MeasureState::pending` (§2.1), `committable_gesture`'s
existing check needs **no code change at all** to keep excluding it —
this is a case where the RIGHT data-model choice makes the correctness
property free rather than something a new `if` has to enforce.

### 10.2 `run_measure_tool`'s Phase C — the new branch

Currently three arms (Linear/Circular/Scale). This spec adds a fourth,
gated on `(tool, pick_mode)` rather than `tool` alone — Linear+Points
keeps calling the existing `commit_measure_linear_draft`; Linear+TwoLines
is new and mirrors Circular's own Accept arm exactly (author from held
state, call `add_dimension`, refresh, set `last_disclosures`, clear):

```rust
} else if canvas::tool_builds_measure_linear(active)
    && st_mode == LinearPickMode::TwoLines
{
    // Mirrors the Circular arm's shape exactly — an inferred result,
    // authored on explicit Accept only (§10.1).
    if let Some(Ok(authored)) = doc.measure.as_ref().map(|s| s.two_lines.verdict.clone()).flatten() {
        match doc.session_mut().add_dimension(page_index, group, authored.kind) {
            Ok(_) => {
                doc.refresh_pages();
                if let Some(st) = doc.measure.as_mut() {
                    st.two_lines.clear();
                    st.last_disclosures = vec![ui_text::measure_dimension_authored(&group_name)];
                }
            }
            Err(err) => {
                if let Some(st) = doc.measure.as_mut() {
                    st.last_disclosures = vec![ui_text::refusal_line(&err.to_string())];
                }
            }
        }
    }
}
```

`ui_text::measure_dimension_authored(&group_name)` is **reused verbatim**
from Circular's existing success path — no new string needed, since the
sentence ("authored into group X") is equally true regardless of which
tool/mode produced the `add_dimension` call.

---

## 11. Where the ce dimension lands, and dragging it into place

### 11.1 P0 — auto-placed, immediately usable, no third click required

Per §0.3's first property, Accept calls `author_from_two_lines(&a, &b,
policy, TwoLinePlacement::default())` — no placement gesture is required
for a usable result. A Linear result appears with its line running
through line A's pick point, text centred; an Angular result appears with
its arc at the derived, floored-for-visibility default radius, text
centred on the arc. Both are **immediately selectable and, for Linear,
immediately draggable** via the existing `run_dimension_drag` gesture
(§11.2 names the one place this is NOT yet true).

### 11.2 ★ A genuine, code-verified gap: Angular ce dimensions cannot be dragged yet

Read directly from `main.rs`'s `run_dimension_drag` (current, shipped
code): the drag's `current` computation is

```rust
let current = doc.session.dimension_model().dimension(id).map(|d| d.kind)
    .filter(|k| matches!(k, pdfce_core::dimension::DimensionKind::Linear { .. }));
```

**This filter predates `DimensionKind::Angular`'s existence** (`905791f`
shipped it; this filter did not change alongside it). Click-selection
still works for an Angular ce dimension — `hit_at`/`dimension_rects`
are not kind-filtered — but the moment a drag is attempted, `current`
resolves to `None`, `placed` resolves to `None`, and **nothing happens**:
no live preview draws, nothing commits on release. An operator who
authors an Angular ce dimension via this spec's gesture and then tries to
drag it — the exact "does the operator drag it into place" question the
brief asks — will find that the drag silently does nothing.

**This is named here as a real gap this spec's own feature surfaces, not
as something this spec is required to close.** Recommend it as a
**named P1**, not a P0 blocker, for two reasons: (a) the default,
auto-computed placement (§11.1) is already a usable, visible, legible
result without dragging — nothing about authoring is broken without this
fix; (b) the fix is real, non-trivial new work, not a one-line filter
change: `DimensionKind::Linear`'s drag decomposes the pointer's delta
into the dimension's own `(u, n)` axis frame (`axis_frame()`), and an
Angular dimension has no equivalent axis frame today — dragging it needs
the delta decomposed into a RADIAL component (grows/shrinks `radius`,
apex-centered) and a TANGENTIAL component expressed in degrees (shifts
`text_along`), which is a different, apex-centered geometry, not a
straight-line one. The exact shape of that computation — a new
`pdfce_core::dimension` helper (e.g. `angular_drag_frame`/an
apex-relative analogue of `axis_frame`) vs. inline trigonometry in
`run_dimension_drag` itself — is the engineer's call, per this project's
standing "name the contract, not the implementation" convention for
core/GUI accessor asks. **What is binding: the contract must produce the
same "what you see mid-drag is what commits" (R85) guarantee
`run_dimension_drag`'s own comment already states as a requirement for
Linear.**

---

## 12. `Settings::parallel_epsilon_degrees` — where it is, and a reachability recommendation

The slider already exists and ships (`settings_panel.rs`'s
`parallel_epsilon_setting`, reachable through the ordinary Settings
window) — **this spec does not need to add it**. The brief's question is
whether it should ALSO be reachable from Tool Options directly.

**Recommendation: a small caption, not a duplicate control.** Show the
current value inline in the `TwoLines`-mode hint area once both lines are
picked and the verdict is `Angled` near the threshold (say, within 2× the
current epsilon) — e.g. appended to `two_line_verdict_angular`'s line:
*"(within N° of counting as parallel — adjust in Settings)"* — so an
operator who is surprised by a borderline reading has an immediate, honest
pointer to WHERE the threshold lives, without this spec inventing a
second slider or a scroll-to-setting mechanism that does not exist
anywhere else in the app yet. A full "Adjust…" deep-link button that opens
Settings pre-scrolled to this one field is a reasonable P2 if the operator
finds the caption insufficient in practice — not built here, since no
existing control in this codebase opens Settings pre-scrolled to a
specific field, and inventing that mechanism for one setting is
disproportionate to what is being asked.

---

## 13. `ui_text.rs` catalog — full list (names, not final copy)

All new; grep before adding, per the standing `pass-16.2`-established
discipline, to confirm none collide with an existing name:

- `two_line_pick_mode_label()`, `two_line_pick_mode_points_option()`,
  `two_line_pick_mode_lines_option()`
- `two_line_hint()`
- `two_line_hover_label()` — the `"line"` hover-glyph caption (§5)
- `two_line_verdict_linear(measured, forced, distance_text)`,
  `two_line_verdict_angular(angle_text)`
- `two_line_virtual_apex_note()`
- `two_line_force_parallel_checkbox()`
- `two_line_epsilon_proximity_caption(epsilon_degrees)` (§12, if the
  proximity caption is built)

Reused, NOT duplicated (confirm at implementation time these are not
re-declared under a new name):

- `ui_text::refusal_line` — both `TwoLineRefusal` variants (§6.2, §7)
- `ui_text::measure_dimension_authored` — the post-Accept success line
  (§10.2)
- `ui_text::measure_group_label`, `measure_open_groups_button` — the
  group picker, unchanged in either mode
- The existing `snap_indicator_label`/painter-text pattern's mechanics
  (§5) — no NEW painter convention, only a new label string and a new
  color role

---

## 14. Disclosures table

| Trigger | Kind | Wording source |
|---|---|---|
| Hover over a pickable line (no pick yet) | Live indicator (canvas glyph + label) | `two_line_hover_label()` |
| Line A picked | Canvas color change only (orange) | — no text disclosure needed, the color IS the state |
| Both lines picked, valid Parallel verdict | Composed sentence | `two_line_verdict_linear` |
| Both lines picked, valid Angled verdict | Composed sentence | `two_line_verdict_angular` |
| …and the apex is virtual | Composed, `notice`-colored, own line | `two_line_virtual_apex_note` |
| Both lines picked, Collinear | **Verbatim core string** | `refusal_line(&TwoLineRefusal::Collinear.to_string())` |
| One picked line is degenerate | **Verbatim core string** | `refusal_line(&TwoLineRefusal::Degenerate.to_string())` |
| `force_parallel` ticked, verdict flips to Parallel | Same composed sentence, `forced: true` branch | `two_line_verdict_linear` |
| Accept succeeds | Composed (reused, not new) | `measure_dimension_authored` |
| Add-dimension itself refuses (rare — e.g. a document-level write failure) | **Verbatim core string** | `refusal_line(&err.to_string())` |

---

## 15. Undo / write-path summary

| Operation | `EditSession` command? | Writes anything? |
|---|---|---|
| Toggling Points ↔ TwoLines mode | No | No |
| Picking line A / line B | No | No — pure `TwoLinePick` state |
| Toggling `force_parallel` (either direction, any number of times) | No | No — re-derives `verdict`, nothing written |
| A Collinear/Degenerate refusal appearing | No | No |
| Escape (any stage) | No | No — falls to the existing four-way precedence, unchanged |
| Reject | No | No — nothing was ever written (rule 7) |
| **Accept, valid verdict** | **Yes, one command** | **Yes — one `add_dimension` call, one authored `Linear` or `Angular` ce dimension + sidecar entry** |
| Dragging an authored Linear result afterward | Yes, one command (existing `run_dimension_drag`) | Yes — existing, unchanged |
| Dragging an authored Angular result afterward | **N/A today (§11.2 — silently does nothing until that gap closes)** | No |

---

## 16. Accessibility

- The `Two Points`/`Two Lines` mode toggle and the `force_parallel`
  checkbox are both real `ui.selectable_value`/`ui.checkbox` widgets —
  Tab-focusable, accesskit-visible, matching every other segmented control
  and checkbox in this pane.
- Accept/Reject are the SAME real `ui.button`s the tool already has —
  no new accept/reject mechanism, no new keyboard gap.
- **A genuine, inherited gap, not created here**: the hover-highlight and
  the picked-line-color feedback (§5) are canvas-drawn and therefore not
  screen-reader legible, matching the standing, already-tracked
  `pass-12.0`/`pass-12.M2` note that the canvas remains a raster image.
  This spec does not widen that gap — every FACT the operator needs
  (which mode is active, what was picked, what pdfce classified it as,
  the measured angle) is ALSO available as real text in the disclosure
  area (§6), never canvas-only. The one thing that is canvas-only is the
  hover preview itself, which is advisory rather than load-bearing — an
  operator who cannot see it can still click a line and read the result
  from the text disclosure that follows.
- No new keyboard chord is bound for the mode toggle — menu/panel-only,
  consistent with `pass-12.M2`'s own "the common tool earns the chord, a
  rarer variant is discoverable, not muscle-memory-optimized" reasoning;
  `Ctrl+Shift+D` continues to arm ordinary `MeasureLinear` (Points mode by
  default — the mode does not reset on re-arming within a session, per
  §2.2's `clear_gesture` note, so a chord-armed tool remembers the
  operator's last-used mode).

---

## 17. Priority table

| Item | Priority | Depends on |
|---|---|---|
| `LinearPickMode` + `TwoLinePick` state (§2) | **P0** | — |
| Mode toggle in Tool Options, mode-conditional hiding (§3) | **P0** | §2 |
| Gesture state machine incl. Collinear/Degenerate recovery (§4, §7) | **P0** | §2 |
| Hover feedback (§5) | **P0** | §2 |
| Verdict disclosure + composed `ui_text` entries (§6, §13) | **P0** | §2 |
| Virtual-apex disclosure + extension-line preview (§8) | **P0** | §6 |
| `force_parallel` checkbox, live re-classify (§9) | **P0** | §6 |
| Explicit-Accept-only commit path, `run_measure_tool` Phase C branch (§10) | **P0** | §2, §6 |
| `parallel_epsilon_degrees` proximity caption (§12) | **P1** | §6 |
| `run_dimension_drag` Angular support (§11.2) | **P1** | Independent of everything above — can ship before, after, or alongside |
| Live "what-if" hover-preview classification (§5, named P2) | **P2** | — |
| Retroactive re-classify of an already-authored dimension (§9.3) | **Not scoped — open question, §18** | New core capability |

**No cut below P0 leaves a coherent, honest feature behind**: every P0
item together is exactly "pick two lines, see what pdfce read them as,
accept or reject it" — the operator's request, verbatim, working
end-to-end. The two P1s are real, named quality gaps (a borderline-angle
caption; dragging an Angular result), not required for the gesture itself
to exist and be trustworthy.

---

## 18. Open items for the librarian

1. **Retroactive re-classification of an already-authored two-line-derived
   ce dimension (§9.3) is explicitly out of scope here, for a data-model
   reason, not a judgment call** — `DimensionKind::Angular`/`Linear` do
   not retain the source `PickedLine`s by design (documented in
   `905791f`'s own doc comment, for an unrelated and good reason: scale
   re-derivation must not silently reinterpret a committed dimension).
   Worth recording as a standing, named, NOT-yet-scoped follow-up
   question rather than something a future session should assume was
   simply forgotten.
2. **`run_dimension_drag`'s Linear-only filter (§11.2) is a genuine,
   code-verified gap this spec's own feature surfaces** — an Angular ce
   dimension can be selected but not dragged, today, independent of
   anything else in this spec. Worth a `ROADMAP.md` note now (a Backlog
   item, or an amendment to the existing R151-shaped "core capability, no
   full shell reach" pattern this project already tracks) rather than
   discovering it only once an operator tries to drag their first
   authored angle.
3. **The "one function, two callers" discipline (`author_from_two_lines`,
   §0.3) is worth naming as a pattern for the standing-rule ledger** —
   this Pass is a clean, already-in-the-codebase example of exactly the
   "duplicated shell logic diverges" hazard several prior specs (12.M2's
   Z2 pattern, the tool-options-dock spec's blast-radius axis) have
   warned about in the abstract; `two_lines.rs`'s own module docs already
   state the principle in almost standing-rule prose. Worth a pointer
   from wherever this project records that class of pattern.

---

## 19. Change-list for the engineer

1. `crates/pdfce-gui/src/canvas.rs`: amend `CanvasTool::MeasureLinear`'s
   doc comment per §1.3 (no variant rename, no `TOOL_PRECEDENCE` change).
2. `crates/pdfce-gui/src/measure_tool.rs`: add `LinearPickMode` and
   `TwoLinePick` (§2.2); add `linear_pick_mode: LinearPickMode` and
   `two_lines: TwoLinePick` to `MeasureState`; extend
   `MeasureState::gesture_in_progress` and `MeasureState::clear_gesture`
   to include `two_lines`.
3. `crates/pdfce-gui/src/main.rs`, `measure_options_ui`: add the mode
   toggle (§3); gate `Snap to content`/Tab/Alt/`Alignment` to
   `Points`-mode only; add the always-visible `TwoLines`-mode hint.
4. `crates/pdfce-gui/src/main.rs`, `run_measure_tool`: implement the
   per-frame hover-highlight (§5); implement the click-handling state
   machine (§4) — calling `pick_line` in `TwoLines` mode instead of the
   existing `snap_candidates` path; call `author_from_two_lines` on the
   second pick and on every `force_parallel` toggle (§9.2); draw the
   Parallel-witness or Angled-arc live preview + virtual-apex extension
   lines (§8) via `theme.palette.{preview, guide}`.
5. `crates/pdfce-gui/src/main.rs`, `measure_status_ui`: branch the
   existing `tool_builds_measure_linear` arm on `linear_pick_mode`; render
   the verdict/refusal per §6.2; compute `can_accept` per §6.3.
6. `crates/pdfce-gui/src/main.rs`, `run_measure_tool` Phase C: add the
   `Linear`+`TwoLines` Accept arm per §10.2, alongside (not replacing) the
   existing three.
7. `crates/pdfce-gui/src/ui_text.rs`: add the §13 entries; grep first to
   confirm no collisions, per the standing discipline.
8. **P1, independent, own commit**: `run_dimension_drag`'s `Angular`
   support (§11.2) — new core or GUI-side apex-relative drag-decomposition
   contract, engineer's call on exact shape.
9. **P1**: the `parallel_epsilon_degrees` proximity caption (§12).
10. File §18's three items with `pdfce-librarian` at Pass completion, per
    the standing rule-5 filing discipline.
