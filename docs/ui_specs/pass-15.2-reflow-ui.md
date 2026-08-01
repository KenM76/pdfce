# Pass 15.2 UI Spec — Reflow on the Pass 14.3 Text-Edit Tool (decision 015 FF-A, slice 3 of 3)

> Authored by `pdfce-ui-specialist`, on dispatch from the engineer. Implements
> the GUI slice of decision 015
> (`docs/decisions/015-ffa-within-block-offline-reflow.md`) — the ROADMAP's
> "★ Pass 15.x" entry, "15.2 — Reflow UI." Engineer implements this verbatim;
> deviations are named, not silent (the Pass 3.2/6.1/7/8/12.0/14.3 convention).
>
> Read before implementing: decision 015 in full (§3.4 preview→command, §3.5
> +R76 overflow, §3.6+R77 alignment); `docs/ui_specs/pass-14.3-text-edit-ui.md`
> in full — this spec is an EXTENSION of that Pass's shipped tool, not a new
> one (§0 below); `crates/pdfce-gui/src/main.rs`'s shipped `TextEditState`,
> `PendingEdit`, and `run_text_edit_tool` (read at spec-authoring time —
> §0.1 quotes the exact structure this spec adds onto); `crates/pdfce-gui/src/
> canvas.rs`'s shipped `CanvasTool`/`GestureInterrupt`/`resolve_escape`;
> `crates/pdfce-core/src/text_edit/reflow.rs` in full (the shipped 15.0
> `ReflowEngine`/`ReflowPreview`/`ReflowRequest`/`BlockAlignment`/
> `AlignmentSource` — this spec wires to the ACTUAL shipped API, not a
> hypothetical one); `crates/pdfce-core/src/edit.rs`'s shipped
> `EditSession::edit_text`/`format_text` (§0.2's precedent for the
> `EditSession::reflow_block` contract this spec requires); `crates/pdfce-cli/
> src/main.rs`'s `cmd_inspect_reflow_preview`/`reflow_recognition_options`
> (§0.3 — the load-bearing recognition-options finding).

---

## 0. What this Pass changes, and two things it cannot design around

### 0.1 Reflow is a sub-mode of the existing `CanvasTool::TextEdit`, not a new tool

`CanvasTool` has exactly one inhabitant today — `TextEdit` (Pass 14.3). This
Pass adds **no second variant**. Reflow is entered, reviewed, and
accepted/rejected entirely from inside the already-active `TextEdit` tool's
existing floating property bar and status strip
(`crates/pdfce-gui/src/main.rs`'s `run_text_edit_tool`, the
`"pdfce-text-edit-propbar"` / `"pdfce-text-edit-status"` `egui::Area`s).

**Why (R60 — extend, don't fork):**

1. Reflow operates on the exact same page, the exact same `TextEditState`
   (`page_text`, the model built from it), the exact same
   `canvas_to_pdf_space`/`pdf_space_to_canvas` bridges, and the exact same
   `EditSession` a second `CanvasTool` variant would have to re-derive or
   duplicate access to. A parallel tool would duplicate the model-rebuild-
   on-page-nav logic, the Escape-precedence wiring, and the tool-entry
   toolbar affordance for zero benefit — precisely the duplication-drift
   risk Pass 12.0 §0 named its own rename table to prevent, one layer up.
2. A recognized `Block` is not a `CanvasTargetProvider` target. 14.3 §0.3
   already established, for exactly this reason, that text selection (a
   contiguous caret/anchor pair) is categorically different from
   `CanvasTargetProvider`'s discrete, provider-agnostic `TargetId` model
   (Pass 9a's future vector objects) and does not belong in
   `canvas_selection`. The same reasoning applies one level up: a `Block`
   is a `pdfce-core::text_edit` structure, addressed by an index into
   `EditableTextModel::blocks()`, not an opaque hit-testable object a
   generic provider would hand back. Reflow's block-targeting stays inside
   `TextEditState`, exactly where 14.0/14.1/14.2/14.3's own block/run/glyph
   addressing already lives.
3. `GestureInterrupt` only needs ONE more `Discard`-class gesture shape
   (`ReflowState`, §2 below, parallel to `PendingEdit`) — not a second
   tool's worth of enforcement-point wiring, escape-precedence duplication,
   or a second toolbar button an operator has to separately discover and
   switch into and out of for what is, conceptually, still "editing the
   text on this page."

**Concretely:** the existing `run_text_edit_tool` gains one more piece of
state (`TextEditState.reflow: Option<ReflowState>`, §2), one more property-bar
control (a "Reflow paragraph…" button, §4), a conditional property-bar
content swap (§4.3), new painter overlays (§5), and one more `EditSession`
commit path (§7) — the same shape of addition 14.3 itself made onto Pass
12.0's substrate, applied one Pass later onto 14.3's own tool.

### 0.2 The load-bearing core-accessor gap — `EditSession::reflow_block`, flagged for 15.1 to confirm

Decision 015 §6 names the contract precisely: *"15.1 adds
`EditSession::reflow_block` applying an accepted preview as ONE undo-able
`CommandKind::ReflowBlock` (mirrors 14.3's `edit_text`/`format_text`
integration)."* At this spec's authoring time, `crates/pdfce-core/src/
edit.rs` does **not** yet have `ReflowBlock` in `CommandKind` or a
`reflow_block` method (verified by reading the file directly) — Pass 15.1
is in progress. This spec is written against the **announced contract**,
mirroring `EditSession::edit_text`/`format_text`'s actual shipped shape
exactly (same file, same pattern: re-derive the session's own current page
content, call the same `plan_*` surgery the free function would, commit ONE
`CommandKind` variant, return the SAME report type a read-only preview
already produces):

```rust
impl EditSession {
    /// Apply an accepted `ReflowPreview` (decision 015 §3.4) as ONE
    /// undo-able `CommandKind::ReflowBlock` command — the session-integrated
    /// sibling of `ReflowEngine::preview`, mirroring `Self::edit_text`/
    /// `Self::format_text` (this file, current shipped code): re-derives
    /// text extraction + block recognition from the SESSION'S OWN current
    /// page content (never a GUI-cached model), re-runs `ReflowEngine::
    /// preview` against `req`, then emits the 14.1-style advance-preserving
    /// surgery for the resulting lines/origins. Returns the SAME
    /// `ReflowPreview`-shaped diagnostics as a `ReflowReport` (disclosures
    /// verbatim, §8) — nothing here changes shape from what 15.0's
    /// read-only preview already computes; only the mutation + one
    /// undo-stack command are new.
    pub fn reflow_block(
        &mut self,
        page_index: usize,
        block_index: usize,
        req: &pdfce_core::text_edit::ReflowRequest,
    ) -> Result<pdfce_core::text_edit::ReflowReport, pdfce_core::text_edit::ReflowSessionError>;
}
```

**Two things this spec asks the engineer to confirm/settle when 15.1 lands
(not designed around, flagged so the GUI wiring in §7 is not written against
a guess):**

1. **`req: &ReflowRequest` in, not a pre-computed `ReflowPreview` in.** This
   spec deliberately chooses the *request*-shaped call (mirroring
   `edit_text(req: &EditRequest, ..)`/`format_text(req: &FormatRequest, ..)`,
   which likewise re-locate and re-plan at commit time rather than taking a
   pre-computed plan) over passing the GUI's already-rendered
   `ReflowPreview` value verbatim. `ReflowEngine::preview` is a pure,
   read-only computation (§0.3's model note aside) — recomputing it once
   more at commit time, from the session's own state, is cheap and is what
   makes "what you see is what you get" a structural guarantee (the exact
   bytes committed are freshly derived from the exact request the operator
   last adjusted) rather than a hope that a separately-carried `Preview`
   value never silently drifted from the session's actual current content.
   If 15.1 instead ships a `Preview`-in shape, the GUI wiring in §7 changes
   its one call site accordingly — flagged here so that is a **confirmed
   choice**, not a silent mismatch discovered at integration time.
2. **`page_cropbox` on the passed `ReflowRequest`.** The GUI (§6) always
   builds its live-preview requests with `.with_page_cropbox(page.crop_box)`
   (matching the CLI's own `cmd_inspect_reflow_preview` convention exactly —
   confirmed by reading `pdfce-cli/src/main.rs`). `EditSession::reflow_block`
   should accept the SAME already-populated `req` verbatim (not silently
   re-fill `page_cropbox` from its own page lookup) so the overflow
   disclosure the operator reviewed pre-accept is exactly what gets computed
   at commit time — no second, independently-sourced cropbox value.

### 0.3 A second, equally load-bearing gap — `reflow_recognition_options()` must be hoisted to `pdfce-core`, or the GUI's block numbering silently disagrees with itself

This is a **new finding**, not previously flagged anywhere, discovered by
reading both the shipped 14.3 GUI code and the shipped 15.0 CLI code
side by side:

- 14.3's `run_text_edit_tool` builds its block-overlay model with
  **`BlockRecognitionOptions::default()`** (confirmed: `crates/pdfce-gui/src/
  main.rs`, Phase A — `EditableTextModel::recognize(&state.page_text,
  &BlockRecognitionOptions::default())`).
- 15.0's own reflow-preview path — **both** `pdfce-cli`'s
  `cmd_inspect_reflow_preview` and `crates/pdfce-core/src/text_edit/
  reflow.rs`'s own test-suite convention (`recognise_relaxed`) — uses a
  **relaxed** recognition (`indent_ratio` pushed out of practical reach) for
  exactly one documented reason: a right-, centre-, or justified-aligned
  paragraph has *ragged left edges by definition*, which the DEFAULT
  recognition's first-line-indent rule misreads as a new paragraph starting
  on every line, fragmenting the whole paragraph into single-line blocks —
  which then makes alignment auto-detection see only single-line blocks
  (always `SingleLineDefault`, never `Detected`), defeating R77 precisely
  for the alignments R77 is supposed to differentiate on.
- `reflow_recognition_options()` (the function that builds this relaxed
  config) is, today, a **private function in `pdfce-cli/src/main.rs`**
  (confirmed by reading it directly) — not exported by `pdfce-core`, not
  reachable from `pdfce-gui` at all.

**The consequence if Pass 15.2 is built without addressing this:** the GUI
would have to choose between (a) reusing 14.3's existing default-recognition
`model` to resolve "which block is the caret in" for the Reflow button —
which will pick the WRONG (fragmented, single-line) block for exactly the
right/centre/justified paragraphs this feature is supposed to shine on — or
(b) hand-rolling a SECOND copy of the same relaxed-threshold constants inside
`pdfce-gui`, which is the identical duplication-drift risk 14.3 §7 already
flagged for `font_subset_stem` (two copies of the same tuning constant,
one crate each, guaranteed to drift the first time either is corpus-tuned
per decision 015 §10 revisit trigger 2).

**Required core addition, flagged to the engineer (small, mechanical, same
class as 14.3's `font_subset_stem` hoist):**

```rust
// crates/pdfce-core/src/text_edit/reflow.rs (or mod.rs) — made public,
// the CLI's private fn deleted and replaced with a call to this one.
/// The block-recognition options reflow uses (decision 015 §3.2's engine +
/// this function): the [`BlockRecognitionOptions`] defaults with
/// first-line-indent paragraph splitting effectively disabled
/// (`indent_ratio` pushed out of practical reach). A right/centre/justified
/// paragraph's ragged left edges would otherwise be misread as per-line
/// indents and fragment the paragraph into single-line blocks, defeating
/// alignment auto-detection (R77). Leading-gap paragraph splitting is
/// UNCHANGED — this only relaxes the indent rule. Every consumer that
/// targets or previews a reflow (the CLI, the GUI, this module's own tests)
/// calls THIS function, once, so "how reflow recognizes paragraph
/// boundaries" cannot drift into two independently-tuned copies.
#[must_use]
pub fn reflow_recognition_options() -> BlockRecognitionOptions { .. }
```

**This is a genuine trade-off, not a strict improvement — say so to the
operator, do not paper over it (§3 below).** Relaxing `indent_ratio` fixes
ragged-left (right/centre/justified) fragmentation, but the SAME relaxation
also loses a *different*, real signal: a traditional flush-left,
first-line-indented paragraph style (no blank line between paragraphs, each
new paragraph signalled only by an indented first line) will, under the
relaxed config, **merge** what should be two separate paragraphs into one
block for reflow's purposes — the exact opposite failure mode. Because of
this, §3 below does **not** recommend silently switching 14.3's whole
general block overlay over to the relaxed config; it recommends keeping
BOTH recognitions and disclosing when they disagree for the block the
operator is about to target.

---

## 1. Invoking reflow — the property bar, operating on the block the caret is in

**Decision:** reflow is invoked via one new button in the EXISTING property
bar (`"Edit Text"` panel, §7 of the 14.3 spec), labelled **"Reflow
paragraph…"**, which targets **the block containing the current caret** —
never a separately-clicked block, never a new hit-testing mode layered onto
the canvas.

**Why not "click a block in the overlay to pick it":** that design would
make plain clicks mean two different things depending on whether
`show_block_overlay` happens to be toggled on — a silent mode-shift the
fuzzy-never-sneaky discipline exists to prevent (turning on the "show
paragraph guides" checkbox purely to *look* at boundaries, as 14.3's own
tooltip explicitly invites an operator to do, would ALSO quietly repurpose
every subsequent click). Caret-driven targeting reuses navigation the
operator can already do with the keyboard alone (14.3 §4.5's Left/Right/Up/
Down/Home/End caret movement, all shipped) and needs **zero new hit-testing
code** — resolving "which block is the caret in" is one line over already-
public API (§2.1).

### 1.1 Resolving the caret's block — no new core accessor needed for this part

`EditableTextModel::line_at(pos) -> Option<usize>` (public, shipped 14.3) and
`Line.block: usize` (public field, shipped 14.0) already compose to exactly
what is needed:

```rust
fn caret_block_index(model: &EditableTextModel<'_>, caret: TextPosition) -> Option<usize> {
    let line_idx = model.line_at(caret)?;
    model.lines().get(line_idx).map(|l| l.block)
}
```

**Optional core nicety (P1, not blocking):** hoist this one-liner as
`EditableTextModel::block_at(pos: TextPosition) -> Option<usize>`, mirroring
`line_at`'s own existence as sugar over a walk the model already does
internally — cheap, avoids the same three-line composition appearing at
every future call site (CLI, GUI, tests), same "core owns the derived
structure" principle 14.3 §4.3 already invoked for `line_at`/`word_range_at`/
`line_range_at` itself.

### 1.2 Which model resolves it — the RELAXED one (§0.3), not 14.3's default one

`caret_block_index` above must be called against a model built with
`reflow_recognition_options()` (§0.3), **not** the `BlockRecognitionOptions::
default()` model 14.3's Phase A already builds for the general overlay.
Concretely, `run_text_edit_tool`'s Phase A gains a SECOND, parallel
recognition pass:

```rust
let model = EditableTextModel::recognize(&state.page_text, &BlockRecognitionOptions::default());
// NEW for 15.2 — same page_text, the reflow-specific relaxed recognition:
let reflow_model = EditableTextModel::recognize(&state.page_text, &reflow_recognition_options());
```

Both are "cheap, index-only" per `EditableTextModel`'s own module docs
(no glyph data is copied, only indices/boxes over the borrowed `PageText`) —
building it twice per frame is an accepted, deliberate cost of this Pass,
not an oversight; name it as such in the shipped code's module doc so a
future reader does not "simplify" it back down to one model and
reintroduce §0.3's bug.

`caret_block_index(&reflow_model, caret)` is what the "Reflow paragraph…"
button's enabled state and target `block_index` are computed from,
end-to-end — never `model` (the default-recognition one), which stays
exactly as 14.3 shipped it, unchanged, for the plain overlay/caret/format
path.

### 1.3 Button placement, label, tooltip, disabled state

In the property bar, directly below the existing font row and above the
existing block-overlay checkbox (the property bar's normal-mode content —
§4.3 covers what replaces it once reflow is actually active):

```rust
ui.separator();
let target = caret.and_then(|c| caret_block_index(&reflow_model, c));
let enabled = target.is_some() && state.pending.is_none(); // §1.4
ui.add_enabled_ui(enabled, |ui| {
    if ui.button(ui_text::reflow_button_label()).clicked() {
        // §2 — enter reflow, seed ReflowState from an initial preview.
    }
}).response.on_hover_text(if enabled {
    ui_text::reflow_button_tooltip()
} else if state.pending.is_some() {
    ui_text::reflow_disabled_pending_tooltip()
} else {
    ui_text::reflow_disabled_no_block_tooltip()
});
```

Grey, not hidden (14.3's own §1.2 precedent for the tool-toggle button
itself, applied here) — there is something to discover ("this exists, place
your cursor in a paragraph to use it").

### 1.4 Mutual exclusion with `PendingEdit` — at most one derived, uncommitted state at a time

`TextEditState` gains `reflow: Option<ReflowState>` (§2) as a THIRD parallel
state, alongside `pending` — never both at once:

- The "Reflow paragraph…" button is **disabled** while `pending.is_some()`
  (§1.3) — the operator must Accept or Reject the in-flight single-run edit
  first. Tooltip explains why (`reflow_disabled_pending_tooltip`).
- Typing (which creates/extends a `PendingEdit`, §6.1 of the 14.3 spec) is
  **suppressed** while `reflow.is_some()` — the exact same "cross-run
  selection suppresses typing" precedent (14.3 §4.4/Phase B: `if
  image_response.has_focus() && !cross_run`), extended to `&& state.reflow.
  is_none()`.
- A plain click elsewhere on the canvas **discards** an in-progress
  `ReflowState` exactly as it already discards a `PendingEdit` today (14.3
  Phase B: *"The click resolves AFTER render: an in-progress pending edit is
  this tool's discardable `GestureInterrupt` — a click discards it"* — this
  Pass extends that same line to check `reflow` too). Nothing was ever
  written (rule 7 — no friction for a reversible, unstarted action).
- `GestureInterrupt`'s query (`current_gesture_interrupt`, Pass 12.0 §3.3's
  ONE enforcement point) returns `Discard` when **either** `pending.
  is_some()` **or** `reflow.is_some()` — one more disjunct on an existing
  query, not a second enforcement point.
- `resolve_escape`'s `gesture_discardable` input is likewise
  `pending.is_some() || reflow.is_some()` — Esc cancels whichever is active
  (§7.4 covers Reject's own binding, which is the same action under a
  different name).

---

## 2. State model

```rust
/// An in-progress, operator-reviewed reflow of ONE recognized block —
/// never written anywhere until Accept (§7). A crash mid-review loses only
/// the uncommitted width/alignment/leading adjustments — the same low-stakes
/// class as a half-drawn Pass 6.1 shape or a half-composed `PendingEdit`
/// (§10 of the 14.3 spec; this Pass changes nothing about that discipline).
struct ReflowState {
    /// Which block (an index into the RELAXED-recognition model's
    /// `blocks()`, §1.2 — never the default-recognition model's index
    /// space, which may number blocks differently for the same paragraph).
    block_index: usize,
    /// The engine's own auto-detected alignment for this block, computed
    /// ONCE at entry (`ReflowEngine::detect_alignment(block_index)`) and
    /// held fixed for the rest of the review — the stable reference point
    /// the "Detected: X" / "you changed this" caption (§6.2) compares
    /// against, independent of whatever the operator later picks.
    detected_alignment: pdfce_core::text_edit::DetectedAlignment,
    /// Current working wrap width, pt — seeded from the FIRST preview's
    /// `wrap_width` (the block's own bbox width) on entry, then edited
    /// directly by the width `DragValue`/drag-handle (§6.1). Always a
    /// concrete number; never blank.
    width: f64,
    /// Current working alignment — seeded from `detected_alignment.
    /// alignment` on entry, then edited by the alignment picker (§6.2).
    alignment: pdfce_core::text_edit::BlockAlignment,
    /// Whether the operator has picked a DIFFERENT alignment than
    /// `detected_alignment.alignment` at least once. Gates whether the
    /// live-preview request sends an explicit `.with_alignment(..)`
    /// override at all (§6.2's "don't always send Some" rule) — needed
    /// because `ReflowEngine::preview` reports `AlignmentSource::Overridden`
    /// the instant ANY explicit alignment is supplied, even one that
    /// happens to equal the detected value, which would otherwise make the
    /// "Detected" / "Overridden" caption lie the moment the operator so
    /// much as re-clicks the already-selected option.
    alignment_is_override: bool,
    /// Current working leading, pt — seeded from the first preview's
    /// (measured or estimated) `leading` on entry, then edited by the
    /// leading `DragValue` (§6.3).
    leading: f64,
    /// The most recently computed live preview, recomputed every frame
    /// any of `width`/`alignment`/`leading` changed (§6, mirrors 12.0/14.3's
    /// own "recompute a pure read-only derivation every frame during a
    /// live gesture" precedent — `resolve_range` during a text drag, the
    /// marquee rect during a canvas drag). `Err` is shown, not silently
    /// dropped (§6.4) — a bad width mid-drag is still informative.
    preview: Result<pdfce_core::text_edit::ReflowPreview, pdfce_core::text_edit::ReflowError>,
    /// The last Accept ATTEMPT's refusal `Display` text (+ appended hint,
    /// §7.3), kept visible while the operator revises inputs — never a
    /// silent dead end, mirroring `PendingEdit::last_refusal` exactly.
    /// Cleared the moment any of width/alignment/leading changes (a
    /// revision is in progress; the stale refusal would otherwise imply a
    /// still-current problem it may no longer describe).
    last_refusal: Option<String>,
}
```

### 2.1 Entering reflow (the button's click handler, §1.3)

```rust
if let (Some(block_index), Some(page)) = (target, doc.pages.get(page_index)) {
    let engine = ReflowEngine::new(&reflow_model);
    let detected = engine.detect_alignment(block_index).unwrap_or(/* Left/SingleLineDefault fallback — unreachable in practice since `target` already proved the index valid */);
    let seed_req = ReflowRequest::new().with_page_cropbox(page.crop_box);
    let preview = engine.preview(block_index, &seed_req);
    let (width, leading) = match &preview {
        Ok(p) => (p.wrap_width, p.leading),
        Err(_) => (0.0, 0.0), // shown as an error immediately, §6.4
    };
    state.reflow = Some(ReflowState {
        block_index,
        detected_alignment: detected,
        width,
        alignment: detected.alignment,
        alignment_is_override: false,
        leading,
        preview,
        last_refusal: None,
    });
    state.show_block_overlay = true; // convenience — see §5.1
}
```

`ReflowEngine::preview`/`detect_alignment` are pure, read-only, and take
`&EditableTextModel` — this call needs no `EditSession`, no mutation, and no
extra borrow-scoping beyond what Phase A already does for `model`/
`reflow_model` (§1.2's parallel construction).

---

## 3. The recognition-divergence disclosure (§0.3's honesty surface)

Because `reflow_model` (relaxed) and `model` (default, the general overlay)
can genuinely disagree about where paragraph boundaries fall — most visibly
for right/centre/justified text — this Pass must not let that divergence go
unstated the moment it becomes operationally relevant (fuzzy-never-sneaky,
applied to the tool's OWN two internal derivations, not just to core's
output).

**Rendered once, as a static caption, whenever the "Reflow paragraph…"
button is enabled** (i.e., whenever `target.is_some()` — cheap, always
correct, and does not require diffing the two models' line sets frame to
frame, which would be a fussier, error-prone refinement left as a named
future nicety, not required for this Pass):

```
ⓘ Reflow may group these paragraph guides slightly differently than the
  boundaries shown above, to keep centred, right-aligned, or justified
  paragraphs whole.
```

(`ui_text::reflow_recognition_note()`, §11.) This is GUI-authored copy (not
a verbatim core disclosure — core's own `ReflowDiagnostics.disclosures`
knows nothing about "the boundaries shown above," a purely GUI-side
juxtaposition), shown as an ⓘ-prefixed caption directly under the "Reflow
paragraph…" button, not as an alarming warning — it is background context,
not a decision the operator must act on.

**The targeted block itself is additionally highlighted** distinctly from
the general overlay the moment `show_block_overlay` is on and a target
resolves (§5.1) — so the operator can *see*, not just be told, exactly what
"this paragraph" refers to before or after clicking Reflow, including cases
where it visibly spans more than one of the general overlay's own dashed
boxes.

---

## 4. Property bar — content swap while reflow is active

The property bar (`"pdfce-text-edit-propbar"` `egui::Area`, unchanged
position/frame) shows one of two mutually exclusive bodies:

### 4.1 Normal body (`state.reflow.is_none()`) — unchanged from 14.3

Size/colour-model/components/font rows, the new "Reflow paragraph…" button +
divergence caption (§1.3/§3), then the block-overlay checkbox — exactly
14.3's shipped layout with one button and one caption appended.

### 4.2 Reflow body (`state.reflow.is_some()`) — replaces the size/colour/font rows

```
┌─ Reflow ¶ ─────────────────────────────────────────────────────┐
│ ⓘ Detected: Left (from the original layout)                     │
│                                     [or, once overridden:]       │
│ Left was detected — you changed this to Center                  │
│                                                                   │
│ Width (pt): [ 180.0 ▲▼ ]      (or drag the right-edge handle)    │
│ Align: (○ Left) (○ Center) (○ Right) (○ Justify)                 │
│ Line spacing (pt): [ 14.0 ▲▼ ]                                   │
│                                                                   │
│ ☐ Show paragraph guides            [unchanged checkbox]         │
└──────────────────────────────────────────────────────────────────┘
```

Rationale for the swap (not an additive stack of both bodies at once):
size/colour/font apply to "the run at the caret" — a different, now-
suspended editing mode while a block-level re-wrap is under review; showing
both simultaneously would let an operator click "Apply size" mid-reflow-
review with no clear meaning (does it apply to the pre-reflow or post-reflow
text?). Swapping avoids the ambiguity outright rather than disabling half
the panel with an explanation.

### 4.3 Alignment picker — real `selectable_value` buttons, not a dropdown

Same precedent as 14.3 §7's colour-model radios (a real, meaningful choice
the operator should see all four options of, not one defaulting to a value
picked from a closed dropdown):

```rust
ui.horizontal(|ui| {
    ui.label(ui_text::reflow_alignment_label());
    for (val, label) in [
        (BlockAlignment::Left, "Left"),
        (BlockAlignment::Center, "Center"),
        (BlockAlignment::Right, "Right"),
        (BlockAlignment::Justified, "Justify"),
    ] {
        if ui.selectable_value(&mut r.alignment, val, label).clicked()
            && val != r.detected_alignment.alignment
        {
            r.alignment_is_override = true;
        } else if val == r.detected_alignment.alignment {
            r.alignment_is_override = false; // clicking BACK to the
                                              // detected value un-overrides
                                              // it, §2's rule
        }
    }
});
```

### 4.4 Width — `DragValue` + a real drag-handle on the canvas (§6.1)

### 4.5 Leading — a plain `DragValue`, no on-canvas handle (no natural single
drag axis on the canvas maps to "leading" the way the right edge maps to
width — a numeric field alone is sufficient and matches 14.3's own Size
control's own precedent of a bare `DragValue`, no on-canvas equivalent).

---

## 5. The ghost preview — reusing 14.3's marked/applied visual language, generalized from one run to one block

Painted in Phase A, immediately after the existing block-overlay/caret/
`PendingEdit`-preview painting 14.3 already does (same painter, same
`to_screen` closure, no re-raster) — **only** while `state.reflow.is_some()`:

### 5.1 The targeted block's highlight (independent of the ghost itself)

Whenever `show_block_overlay` is on AND a target block resolves (§1.2,
§1.4's "entering reflow sets `show_block_overlay = true`" convenience makes
this always true the moment reflow is entered) — draw that block's
`reflow_model` bbox with a **solid** (not dashed) stroke in the SAME amber
family the general overlay already uses (`rgb(200, 120, 40)`), at 2.5px
instead of the general overlay's 1px: shape (solid vs. dashed) plus weight
is the signal, not a new colour (rule 6). This may visibly cover/exceed
several of the general overlay's own dashed single-line boxes for a
right/centre/justified paragraph — expected, per §3's disclosure.

### 5.2 The ghost lines — old mask, new text, generalized from `PendingEdit`'s per-run mask to a per-block one

1. **Mask the OLD block.** A translucent rectangle over `preview_old_bbox`
   (the `Err`/`Ok` preview's `old_bbox`, converted `to_screen` the same way
   the block-overlay boxes already are) using the EXACT same fill 14.3
   already uses for `PendingEdit` (`rgba(250, 250, 250, 220)`) — so the old
   glyphs stop visually competing with the new ghost text, never a raster
   edit.
2. **Draw each new line's text.** For `Ok(preview)`, iterate `preview.lines`
   (each a `ReflowLine { text, origin_x, baseline_y, .. }`) and
   `painter.text()` each at `to_screen(pos2(origin_x, baseline_y))` using an
   egui built-in proportional font sized from the block's own representative
   size × zoom (same approximation 14.3 already uses for `PendingEdit`'s
   draft — real glyph shapes only exist after a real Accept triggers a real
   re-render; this Pass adds no new font-shaping capability, same as 14.3
   §6.3's own reasoning). `ReflowLine::is_overflowing_word` lines are drawn
   the same way (their text is real content, per R76 — never hidden).
3. **Dashed border + "PREVIEW — not yet applied" tag on the NEW `new_bbox`**
   — verbatim reuse of 14.3's `preview_tag()`/dashed-border convention
   (`rgb(210, 90, 40)`), now sized to the whole block instead of one run.
4. **The OLD `old_bbox`, for comparison, drawn thin and muted** (e.g.
   `ui.visuals().weak_text_color()`, 1px, dashed at a shorter dash length
   than the PREVIEW border so the two are visually distinguishable by
   pattern, not merely by the fact that one happens to be a different
   colour) — labelled with a small **"current"** tag at its own top-left
   corner, mirroring the PREVIEW tag's placement/size convention on the
   opposite corner so the two labels never overlap.
5. **A wrap-width guide.** Whenever `preview.alignment.alignment !=
   BlockAlignment::Left`, draw a thin vertical dashed guide at
   `to_screen(pos2(new_bbox.urx, ..))` spanning the new block's height, with
   a small **"wrap width"** label — this is the "re-flushed right edge" the
   task calls out for justified, generalized usefully to right/centre too
   (all three place text relative to that same right-hand boundary).
6. **`Err(ReflowError)` case (bad width mid-drag, an emptied block, etc.):**
   skip the ghost-text/new-bbox painting entirely (nothing sensible to
   draw); keep the OLD block's mask/muted outline so the canvas does not
   flicker to a blank state, and surface the error in the status strip
   (§6.4) exactly like a refusal — never silently disappear the whole
   preview area.

---

## 6. Adjustable inputs, live re-preview

Recomputed **every frame** any of `width`/`alignment`/`leading` changed this
frame (never per-keystroke-throttled — `ReflowEngine::preview` is a pure,
cheap, read-only computation over an already-built model, the same "recompute
live, mutate only on commit" precedent 12.0/14.3 already established for
`resolve_range` and the marquee rect):

```rust
let req = ReflowRequest::new()
    .with_wrap_width(r.width)
    .with_leading(r.leading)
    .with_page_cropbox(page.crop_box)
    .with_alignment_opt(r.alignment_is_override.then_some(r.alignment));
r.preview = ReflowEngine::new(&reflow_model).preview(r.block_index, &req);
```

`.with_alignment_opt(r.alignment_is_override.then_some(r.alignment))` is the
concrete form of §2's rule: omit the override entirely unless the operator
has genuinely deviated from the detected value, so `preview.alignment.
source` stays a trustworthy `Detected`/`SingleLineDefault`/`AmbiguousDefault`
until it truly is `Overridden`.

### 6.1 Width — `DragValue` + a real drag-handle on the block's right edge

The `DragValue` (§4.4) is the PRIMARY, keyboard-reachable control (Tab-
focusable, arrow-key/type-to-edit like every other `DragValue` in the app).
The on-canvas handle is a mouse CONVENIENCE layered on top of it, never the
only way to change the width — the same "real widget is primary, painter
convenience is secondary" split this spec already uses for block-invocation
(§1) and 14.3 already used for Accept/Reject (real buttons) versus the
caret (painter-drawn, no real-widget form makes sense for a blinking caret
line).

```rust
// Phase A, only while `state.reflow.is_some()` and `preview` is `Ok`:
if let Ok(p) = &r.preview {
    let mid_y = (p.new_bbox.lly + p.new_bbox.ury) / 2.0;
    if let Some(handle_screen) = to_screen(egui::pos2(p.new_bbox.urx as f32, mid_y as f32)) {
        let handle_rect = egui::Rect::from_center_size(handle_screen, egui::vec2(10.0, 24.0));
        // No smaller than the app's own ICON_BUTTON_SIZE (28x24) in its
        // shorter dimension — a real click/drag target, not a pixel-hunt.
        let resp = ui.interact(handle_rect, egui::Id::new("pdfce-reflow-width-handle"), egui::Sense::drag());
        if resp.dragged()
            && let Some(sp) = resp.interact_pointer_pos()
        {
            let canvas = viewer::screen_to_page(sp, image_rect, extent, zoom);
            if let Some(pdf) = viewer::canvas_to_pdf_space(canvas, page) {
                let new_width = (f64::from(pdf.x) - p.old_bbox.llx).max(MIN_WRAP_WIDTH_PT);
                r.width = new_width; // re-preview computed at the top of §6
            }
        }
        resp.on_hover_cursor(egui::CursorIcon::ResizeHorizontal)
            .on_hover_text(ui_text::reflow_width_handle_tooltip());
    }
}
```

Recomputed from the **absolute** current pointer position each dragged
frame (`pdf.x − old_bbox.llx`, `old_bbox.llx` never moves — the block is
always left-anchored per the engine's own `align_origin_x`/top-anchored
placement, confirmed in `reflow.rs`), not accumulated from a delta — the
same "no drift" discipline Pass 12.0's own marquee-rect recomputation
already uses. `MIN_WRAP_WIDTH_PT` is a small positive floor (e.g. a few
points) purely to avoid feeding a degenerate width into the engine mid-drag
and flashing a `BadWidth` error on every frame the pointer is briefly past
the left edge — the engine's own `ReflowError::BadWidth` still fires (and is
shown, §5.2 item 6) for any width that manages to reach it non-positive; the
floor is a UI-side smoothing, not a silent clamp hiding a real refusal.

### 6.2 Alignment — the "Detected" / "you changed this" caption

Rendered directly above the picker (§4.2's mock), computed from
`r.detected_alignment` (fixed at entry) versus `r.alignment` (live):

```rust
let caption = if !r.alignment_is_override {
    match r.detected_alignment.source {
        AlignmentSource::Detected => ui_text::reflow_detected_caption(r.alignment.as_str()),
        AlignmentSource::SingleLineDefault | AlignmentSource::AmbiguousDefault =>
            ui_text::reflow_ambiguous_caption(), // ⚠, not ⓘ — an honest limitation, rule 6 icon discipline
        AlignmentSource::Overridden => unreachable!("detect_alignment never returns Overridden"),
    }
} else {
    ui_text::reflow_overridden_caption(r.detected_alignment.alignment.as_str(), r.alignment.as_str())
};
```

This is a short, GUI-authored UI cue (§11) — distinct from, and shown
ALONGSIDE, the full verbatim `ReflowDiagnostics.disclosures` sentence the
engine itself produces for the SAME fact (§8.1's live-diagnostics list
already includes the engine's own longer, ragged-edge-measurement-bearing
alignment disclosure string) — the caption is the at-a-glance version, the
disclosure list is the full, authored-by-core, cited-decision version. Never
paraphrase one into the other; render both, each once, in its own place.

### 6.3 Leading — plain `DragValue`, no special handling beyond §6's live re-preview.

---

## 7. Accept / Reject

Reuses the EXISTING `"pdfce-text-edit-status"` `egui::Area` (bottom-left,
unchanged position) — the SAME strip `PendingEdit`'s Accept/Reject and
disclosure/refusal rendering already occupies, since only one of
`{pending, reflow}` is ever `Some` at a time (§1.4):

```rust
if let Some(r) = &state.reflow {
    ui.horizontal(|ui| {
        if ui.add_sized(ICON_BUTTON_SIZE, egui::Button::new(ui_text::reflow_accept())).clicked() {
            do_accept_reflow = true;
        }
        if ui.add_sized(ICON_BUTTON_SIZE, egui::Button::new(ui_text::reflow_reject())).clicked() {
            do_reject_reflow = true;
        }
    });
    // §8 — live diagnostics + overflow, ALWAYS shown while reviewing,
    // rendered BEFORE the operator decides, not just after.
    match &r.preview {
        Ok(p) => for d in &p.diagnostics.disclosures {
            ui.label(ui_text::disclosure_bullet(d)); // verbatim, §8.1
        },
        Err(e) => ui.colored_label(ui.visuals().error_fg_color, ui_text::refusal_line(&e.to_string())),
    }
    if let Some(text) = &r.last_refusal {
        ui.separator();
        ui.label(ui_text::refusal_strip_title());
        ui.colored_label(ui.visuals().error_fg_color, ui_text::refusal_line(text));
    }
}
```

### 7.1 Reject — no core call

```rust
if do_reject_reflow {
    state.reflow = None; // nothing mutated — mirrors PendingEdit's Reject exactly
    return;
}
```

Keyboard binding: **Esc** (§1.4 — the same `resolve_escape`
`CancelGesture` branch `PendingEdit`'s Reject already satisfies, no new
Escape semantics).

### 7.2 Accept — one `EditSession::reflow_block` call, one undo-able command

```rust
if do_accept_reflow && let Some(r) = &state.reflow {
    let req = ReflowRequest::new()
        .with_wrap_width(r.width)
        .with_leading(r.leading)
        .with_page_cropbox(page.crop_box)
        .with_alignment_opt(r.alignment_is_override.then_some(r.alignment));
    match doc.session.reflow_block(page_index, r.block_index, &req) {
        Ok(report) => {
            doc.refresh_pages();
            doc.build_text_edit_state(); // page_text/model rebuilt — the
                // §2.1-of-14.3-spec rule ("after an accepted edit, rebuild
                // for the SAME page — the content stream changed") applies
                // to ReflowBlock exactly as it already does to EditText/
                // FormatText; no new rebuild rule needed.
            if let Some(state) = doc.text_edit.as_mut() {
                state.reflow = None;
                state.last_disclosures = report.disclosures; // verbatim, §8.1
            }
        }
        Err(err) => {
            if let Some(r) = doc.text_edit.as_mut().and_then(|s| s.reflow.as_mut()) {
                r.last_refusal = Some(ui_text::refusal_with_hint(&err.to_string(), reflow_refusal_hint(&err)));
            }
        }
    }
}
```

Keyboard binding: **Enter**, matching `PendingEdit`'s own Accept binding
exactly (14.3 §6.4) — consistent muscle memory for "the thing I'm reviewing,
commit it," regardless of which sub-mode produced the review.

### 7.3 Refusal hint table (§8.2 of the 14.3 spec's own convention, extended)

`ReflowSessionError`'s exact variants are 15.1's decision (§0.2); this table
covers what is ALREADY named in the shipped, read-only `ReflowError` (§0
above) and should be extended, not replaced, once 15.1's real session-error
type ships:

| Trigger | "What would lift it" framing to append |
|---|---|
| `ReflowError::BlockIndexOutOfRange` | (should be unreachable from the GUI — `target`/`r.block_index` are only ever set from a just-computed valid index; if it fires anyway, treat as an internal-consistency bug, not an operator-actionable refusal) |
| `ReflowError::EmptyBlock` | "This paragraph has no measurable text to reflow (it may be entirely invisible or `/ActualText`-only) — nothing to re-wrap here." |
| `ReflowError::BadWidth` | "Choose a positive width — drag the handle back onto the page, or type a value above zero." |
| *(15.1 TBD — e.g. an encrypted-document or content-parse refusal, mirroring `edit_text`'s own `Encrypted`/`Content` cases)* | Extend this table with one row per new variant 15.1 actually ships, same discipline as 14.3 §8.2 — no refusal ships without a next-step sentence. |

---

## 8. Disclosure surfacing — two distinct moments, one shared convention

### 8.1 Live diagnostics (while reviewing, before Accept) — §7's rendering, verbatim

`ReflowPreview.diagnostics.disclosures` — the engine's own already-authored
strings (alignment-detection rationale, leading/space-width
estimated-vs-measured, oversized-word overflow, page-bottom overflow) —
rendered ⓘ/⚠-prefixed **exactly as computed**, one bullet per string, in the
status strip WHILE the operator is still deciding (§7). This is what lets
the overflow condition (R76) inform the decision itself rather than
surprise the operator only after they have already clicked Accept.
**Overflow is disclosed here calmly — not as a Pass-8-style confirmation
gate.** Reflow overflow is neither destructive nor irreversible (it is a
pre-save, undo-able-via-Ctrl+Z content REPOSITIONING, and R76 already
guarantees the content itself is never lost) — Accept stays a plain, single
click, no extra acknowledgement checkbox. Do not import Pass 8's
refusal-acknowledgement gate here; that heavier control exists specifically
for genuinely irreversible-once-saved content REMOVAL (redaction/sanitize),
a materially different risk class this feature does not share.

### 8.2 Post-accept report (after commit) — `state.last_disclosures`, unchanged mechanism

Exactly 14.3's existing `last_disclosures` rendering (§8.1 of that spec) —
this Pass adds no new mechanism here, only a new source (`ReflowReport.
disclosures` replaces `EditReport`/`FormatReport.disclosures` in the SAME
field, at the SAME call site pattern, §7.2 above).

---

## 9. Keyboard / accessibility

- **Discoverability:** the "Reflow paragraph…" button lives in the ALREADY-
  discovered property bar (no new toolbar surface, no new panel to find);
  greyed-not-hidden when no target resolves, with a tooltip naming why
  (§1.3) — matching every other disabled-state convention in the app.
- **Keyboard access:** width `DragValue`, alignment `selectable_value`
  buttons, leading `DragValue`, and Accept/Reject are ALL real `egui`
  widgets, Tab-reachable in the property-bar/status-strip chain exactly as
  14.3's own controls already are. The on-canvas width-drag handle (§6.1) is
  a mouse CONVENIENCE — the `DragValue` is the keyboard-complete primary
  control; a keyboard-only operator loses nothing by never touching the
  handle.
- **Keyboard bindings:** Enter = Accept, Esc = Reject (§7.1/§7.2) — reused
  from `PendingEdit`, no new chord for ENTERING reflow (a rarer, more
  deliberative per-paragraph action than the tool-entry chord itself;
  mouse/Tab discovery via the property bar is sufficient, consistent with
  rule 7 cutting the other way too — not every action needs a keyboard
  shortcut, only the repetitive/destructive ones).
- **Colour never sole signal:** the targeted-block highlight is solid-vs-
  dashed + weight (§5.1), never colour alone; the old/new ghost boxes are
  dashed-pattern + corner-tag-text distinct, never colour alone (§5.2); the
  alignment caption uses ⓘ/⚠ icon + text exactly like 14.3's own disclosure
  icon discipline (§6.2); the wrap-width guide carries a text label, not a
  bare line (§5.2 item 5).
- **Click-target sizing:** Accept/Reject reuse `ICON_BUTTON_SIZE` (28×24,
  unchanged); the width-drag handle's hit rect is 10×24 — matched to
  `ICON_BUTTON_SIZE`'s shorter dimension, not an ad hoc smaller target,
  per the app's existing convention (14.3 §10's own instruction, reused
  verbatim here).
- **Crash-safe autosave:** `ReflowState` is never written anywhere until
  Accept (§7.2) — a crash mid-review loses only the uncommitted width/
  alignment/leading tweaks, the same low-stakes class as `PendingEdit`'s own
  §10 precedent. No new autosave work; the existing post-Accept cadence
  (already exercised by `EditText`/`FormatText`) picks up `ReflowBlock` the
  moment 15.1 lands, unchanged.
- **`accesskit`/screen-reader gap — inherited, not introduced:** the ghost
  preview's per-line TEXT (§5.2) is still painter-drawn, same as
  `PendingEdit`'s own draft-text rendering (14.3 §10) — no new instance of
  the standing gap, only a bigger area of the same kind. The interactive
  CONTROLS (the button, `DragValue`s, `selectable_value`s, Accept/Reject,
  the drag handle) all use real `egui` widgets and get real `accesskit`
  exposure for free — banking Pass 12.0 §6.4's recommendation exactly as
  14.3 §10 already did for its own Accept/Reject.

---

## 10. Undo / write-path summary

| Operation | `EditSession` command? | Writes anything? |
|---|---|---|
| "Reflow paragraph…" click (enter reflow, §2.1) | No | No — read-only `ReflowEngine::preview` call |
| Width drag / `DragValue` edit, alignment pick, leading edit (§6) | No | No — recomputes a read-only preview live |
| Accept, successful (§7.2) | **Yes — ONE `CommandKind::ReflowBlock`** (§0.2's `EditSession::reflow_block`) | Nothing until real Save (incremental, R34/R36) |
| Accept, refused | No | No — `reflow` stays, unchanged, for revision |
| Reject / Esc (§7.1) | No | No — `reflow` discarded |
| Click elsewhere while reviewing (§1.4) | No | No — discards `reflow`, proceeds as an ordinary caret click |

Exactly one new class of undo-able command enters `EditSession`'s existing
stack (once §0.2 ships) — every other row is view-state, matching 12.0's and
14.3's own write-path tables line for line.

---

## 11. `ui_text.rs` catalog (new entries, proposed wording — engineer may adjust text, not presence)

- `reflow_button_label() = "Reflow paragraph…"`
- `reflow_button_tooltip() = "Re-wrap this paragraph to a new width, \
  alignment, or line spacing — review the result before it's applied."`
- `reflow_disabled_no_block_tooltip() = "Click into a paragraph first — \
  Reflow works on the paragraph containing your cursor."`
- `reflow_disabled_pending_tooltip() = "Finish or cancel the current edit \
  first (Accept/Reject below)."`
- `reflow_recognition_note() = "Reflow may group these paragraph guides \
  slightly differently than the boundaries shown above, to keep centred, \
  right-aligned, or justified paragraphs whole."` (§3)
- `reflow_alignment_label() = "Align:"`
- `reflow_detected_caption(align: &str) -> String = "Detected: {align} \
  (from the original layout)"`
- `reflow_ambiguous_caption() = "⚠ No clear alignment signal in this \
  paragraph — defaulted to Left"`
- `reflow_overridden_caption(detected: &str, chosen: &str) -> String = \
  "{detected} was detected — you changed this to {chosen}"`
- `reflow_width_handle_tooltip() = "Drag to change how wide this \
  paragraph re-wraps — or type an exact width above."`
- `reflow_accept() = "✓ Accept reflow"`, `reflow_reject() = "✕ Reject \
  reflow"` (distinct wording from the plain-edit `accept_edit()`/
  `reject_edit()`, since both could in principle render in the same strip
  across a session and the operator should never have to guess which
  action a bare "Accept" commits)
- `reflow_refusal_hint(err: &ReflowError) -> &'static str` — the §7.3 table,
  one arm per variant, extended as 15.1's real error type ships (mirrors
  14.3's `edit_refusal_hint`/`format_refusal_hint` dispatch functions
  exactly).

No new wording for the disclosure BODIES themselves (§8) — those render
`ReflowDiagnostics.disclosures`/`ReflowReport.disclosures` verbatim from
core, exactly as 14.3 §11 already established for `EditReport`/
`FormatReport`.

---

## 12. Priority table

| Item | Priority | Note |
|---|---|---|
| §0.2 `EditSession::reflow_block` (core, gates Accept) | **P0 — confirm/land with 15.1** | The `req`-in-not-`Preview`-in shape and the `page_cropbox`-passthrough are this spec's two asks to confirm, not silent guesses |
| §0.3 hoist `reflow_recognition_options()` to `pdfce-core`, public | **P0** | Without this, the GUI either targets the WRONG block for right/centre/justified paragraphs or duplicates a corpus-tunable constant in a second crate — both bad |
| `TextEditState.reflow: Option<ReflowState>`, entry via the property-bar button (§1–§2) | **P0** | |
| Parallel `reflow_model` build + `caret_block_index` (§1.1–§1.2) | **P0** | |
| Mutual exclusion with `PendingEdit` (§1.4) | **P0** | The load-bearing correctness rule preventing two simultaneous uncommitted derived states |
| Recognition-divergence caption (§3) | **P0** | Cheap; the honesty requirement §0.3's finding demands |
| Property-bar content swap (§4) | **P0** | |
| Ghost preview: mask + new-line text + PREVIEW tag + muted old outline + wrap-width guide (§5.2) | **P0** | Reuses 14.3's exact visual language, generalized to block scope |
| Targeted-block solid-highlight (§5.1) | **P0** | The visible link between "the block" and "this control" |
| Width `DragValue` + drag-handle (§6.1) | **P0** (`DragValue`) / **P1** (handle) | `DragValue` alone is fully functional and keyboard-complete; the handle is a banked convenience |
| Alignment picker + Detected/Overridden caption (§4.3/§6.2) | **P0** | R77's differentiator, made visible |
| Leading `DragValue` (§6.3) | **P0** | |
| Accept/Reject + refusal hint table (§7) | **P0** | |
| Live diagnostics rendering, pre-accept (§8.1) | **P0** | Lets overflow inform the decision, not just follow it |
| `EditableTextModel::block_at` convenience (§1.1) | **P1** | Cheap sugar, not blocking — the three-line composition works today |
| Diff-aware (not always-on) recognition-divergence caption | **Explicitly deferred, named nicety** | §3 — correctness/clutter tradeoff not worth the added complexity this Pass |

---

## 13. Open items for the librarian

1. **§0.2 — `EditSession::reflow_block`'s exact call shape** (`req`-in vs
   `Preview`-in; `page_cropbox` passthrough) is this spec's own design
   choice, offered for 15.1 to confirm or correct — worth a one-line note
   on whichever of decision 015 / the 15.1 ROADMAP entry is touched next,
   the same "found and closed here, not overlooked" convention Pass 12.0
   §10 and Pass 14.3 §14 already set for their own core-accessor findings.
2. **§0.3 is a genuine, previously-unflagged finding**: `pdfce-cli`'s
   private `reflow_recognition_options()` must be hoisted to `pdfce-core`
   (public) before Pass 15.2 can correctly resolve "which block is the
   caret in" — otherwise the GUI either targets the wrong (fragmented)
   block for right/centre/justified paragraphs or duplicates the tuning
   constant in a second crate. Same duplication-drift shape as 14.3 §7's
   `font_subset_stem` finding, one Pass later, on a different symbol. Worth
   recording alongside that precedent.
3. **The recognition-divergence disclosure (§3) is new UI-authored copy**,
   not sourced from any core disclosure string — flag if a future reader
   wonders why it isn't listed among the "render core's disclosures
   verbatim" set: it describes a purely GUI-internal juxtaposition (two
   models built by the SAME tool), which core has no way to know about.
4. **`EditableTextModel::block_at` (§1.1)** is a small, optional core
   nicety in the same "core owns the derived structure" spirit as 14.3
   §4.3's `line_at`/`word_range_at`/`line_range_at` — flag alongside those,
   not as a separate future decision.
5. **This Pass adds no new `CanvasTool` variant and no new placement-
   taxonomy instance** (§0.1, §1) — reflow lives entirely inside the
   existing "dedicated top panel, tool-scoped" property bar 14.3 already
   built; worth noting in the same taxonomy-history record Pass 12.0 §10
   and Pass 14.3 §14 already maintain, so a future reader does not go
   looking for an eighth/ninth instance that does not exist.
6. **Diff-aware recognition-divergence detection (§3, §12's deferred row)**
   is a named, deliberately-deferred refinement — worth tracking as a
   fast-follow alongside FF-A's other named non-goals (Knuth-Plass,
   hyphenation) rather than as its own independent backlog item.
