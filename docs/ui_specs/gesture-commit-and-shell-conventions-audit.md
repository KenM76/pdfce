# Gesture-commit and shell-conventions audit — the Accept/Reject box, the
# status-bar jump, and the ribbon question (ui-spec)

**Author:** `pdfce-ui-specialist` · **Date:** 2026-08-04 · **Drives:**
operator feedback, verbatim, 2026-08-04:

> "The interface is a bit weird where there is a separate accept / reject
> box somewhere on the screen to click - I've never seen any other software
> operate that way. Zoom is also jarring because cntrol-scroll doesn't
> treat the cursor as the center of the zoom, but the workspace area
> instead. There's quite a few weird gui setup decisions that don't match
> up to anything I've seen in other software. This should look and feel
> like the ribbon interface in MS Office, solidworks, and other modern
> software."

The zoom-to-cursor defect is already fixed and is out of scope here. This
document covers: (1) an honest, code-grounded divergence audit of the
current shell; (2) a redesign of the Accept/Reject pattern, reconciled with
rule 4 (fuzzy, never sneaky); (3) the status-bar/canvas-jump defect found
today, with a recommended fix; (4) a professional read on the ribbon
request; (5) a prioritized change list.

**Terminology note, per project instruction (2026-08-04):** every use of
"dimension" below is qualified. **ce dimensions** are pdfce-authored
`/Line`+`/IT /LineDimension` annotation objects (`crates/pdfce-core/src/
dimension/`); **pdf dimensions** are pre-existing dimension callouts already
present in an imported CAD-exported page. The Measure ▾ tool family
discussed in §1/§2 authors **ce dimensions**; nothing here concerns pdf
dimensions.

**I do not write code.** Everything below is a critique and a concrete
change list, cited against the actual running source
(`crates/pdfce-gui/src/main.rs`, `dock.rs`, `canvas.rs`), for the engineer
to implement or push back on.

---

## 0. What was actually read

`crates/pdfce-gui/src/main.rs` in full for: the toolbar module docs
(L85–150), `PdfceApp::toolbar`/`toolbar_controls` (L5665–6224), the three
tool "propbar"/"status" `egui::Area` pairs — TextEdit (L8854, L9364),
AddText (L10120, L10242), Measure (L11825, L11940) — `status_bar`/
`status_bar_body` (L6831–6960+), `selection_readout` (L10971), `canvas`
(L7496–7760), the panel-add order (L5123–5147), `resolve_gesture_interrupt`/
`current_gesture_interrupt`/`discard_active_gesture`/`commit_active_gesture`
(L4796–4951), `collect_keyboard_actions` (L5239–5370). `crates/pdfce-gui/
src/canvas.rs` (module docs, `GestureInterrupt`, `EscapeOutcome`,
`resolve_escape`, `CanvasTargetProvider`). `crates/pdfce-gui/src/dock.rs`
(module docs, `DockPanel`). `docs/ROADMAP.md`'s Standing rules R60, R61,
R80–R85, R98 for citation. This agent's own memory of the icon-set/toolbar
spec, the GUI-polish audit, and the 19.3 text-formatting spec, re-verified
against current code rather than trusted from memory (per this agent's own
brief).

---

## 1. Honest audit — where the shell diverges from Word / SolidWorks /
   Illustrator / Acrobat, cited

Ten findings, D1–D10, ordered roughly by how directly each explains the
operator's "doesn't match anything I've seen" verdict.

### D1. The toolbar is a flat, uncaptioned row wearing a "grouped" costume

The module doc (L125–128) states the design intent honestly: "The toolbar
is built as `ui.separator()`-divided *groups* ... rather than one flat row,
so the Passes that add Edit/Comment/Sign clusters insert a group instead of
rewriting the row." That intent is sound and IS what happened mechanically
— six separator-divided groups exist (file / view / navigation / zoom /
edit / history) plus an ungrouped-utility cluster. But **no group ever
received a visible caption.** A `ui.separator()` is a thin vertical rule; it
tells the eye "something changed here," not "this is the View group." Every
reference app in the operator's list — the Office ribbon, SolidWorks's
CommandManager, Illustrator's Properties/Control panel — labels its groups
with a caption underneath or beside the icon cluster. pdfce's groups are
real in the code and invisible on screen. This is the single largest
contributor to "an amalgam that doesn't match anything": the operator is
looking at correctly-organized data with no organizational chrome on top of
it.

### D2. The "edit" group silently grew to nine controls with no re-balancing pass

Walking `toolbar_controls` (L5850–6203) in the order it renders: rotate
left, rotate right, Properties toggle, Markup ▾ (menu), Text ▾ (menu), Edit
Text toggle, Add Text toggle, Edit Objects toggle, Measure ▾ (menu), Redact.
**Nine controls, three of them menus, in one unlabeled cluster.** Each
addition was reasoned correctly and locally at the time (the Pass 6.1 doc
comment, the Pass 14.3 comment, the Pass 16.2 comment, the Pass 12.M2
comment, the Pass 8.1 comment at L6125–6170 all individually justify why
their control belongs in "edit") — but nobody re-balanced the *group* as it
grew, which is exactly the accretion failure mode standing rule 3 exists to
warn against, arriving here from the bottom up (Pass-by-Pass local
correctness) rather than the top down (one bad initial menu design, which is
Acrobat's own failure mode). A ribbon's tab system is precisely the
mechanism that prevents this — a tab has a natural capacity signal (does it
fit in one or two rows?) that a `ui.separator()`-bounded cluster inside an
infinitely wrapping row does not.

### D3. Inline dropdown menus mixed with plain toggles inside one row

Markup ▾, Text ▾, and Measure ▾ (`Self::menu_button_labeled`/
`menu_button_atoms`, L5893–6123) are click-to-open menus sitting between
plain click-to-toggle controls (Edit Text, Add Text, Edit Objects,
L5983–6048). Visually these read as the *same kind of control* — same
size, same icon+text shape — but behave differently: one toggles a tool
immediately, the other opens a submenu requiring a second click. The only
visual differentiator is a small "▾" glyph. This is a legitimate,
ubiquitous pattern in isolation (browser toolbars do this constantly) but
it is specifically **not** the ribbon/gallery pattern the operator named —
a ribbon gallery either shows the options directly as a grid (Word's Style
gallery) or opens a large, visually distinct dropdown panel, never a
thin inline menu the same size as a plain button.

### D4. Floating, draggable "tool palette" property bars — a fourth GUI family the operator has not named directly but is almost certainly reacting to

Three independent `egui::Area`s — TextEdit's propbar (L8854), AddText's
(L10120), Measure's (L11825) — all share:

```rust
egui::Area::new(egui::Id::new("pdfce-…-propbar"))
    .order(egui::Order::Foreground)
    .movable(true)
    .default_pos(image_rect.min + egui::vec2(8.0, 8.0))
```

`.movable(true)` at a **fixed default position (canvas top-left corner)**,
independent of where the operator clicked, what they selected, or where the
caret is. This is the Photoshop-CS2 / classic-GIMP **floating tool
palette** pattern — a window that starts somewhere arbitrary and that the
operator is expected to drag to a convenient spot and leave there. None of
the operator's three named reference apps use this pattern for per-
selection controls:

- **Word** shows an inline mini-toolbar *at the selection itself* and a
  docked-right Properties/Styles pane in a *fixed* screen location.
- **SolidWorks** replaces the FeatureManager tree with a *fixed,
  pinned-left* PropertyManager pane — never a floating movable window.
- **Illustrator** uses a fixed-position, dockable Properties panel (right
  side), contextual content swaps inside it, the panel itself never moves
  on selection change.

All three keep the property surface in one predictable place. pdfce's is
the only one of the four families (flat toolbar, ribbon, docked panel,
floating palette) that is a floating palette, and it's the one family none
of the operator's reference apps use for this purpose.

### D5. Accept/Reject: a fully independent, differently-anchored THIRD floating panel — the operator's named complaint, confirmed exactly

Each tool's `…-status` `egui::Area` is a **separate** floating panel from
its `…-propbar`, anchored to a **different corner**:

```rust
egui::Area::new(egui::Id::new("pdfce-text-edit-status"))   // and add-text-status, measure-status
    .order(egui::Order::Foreground)
    .fixed_pos(egui::pos2(image_rect.min.x + 8.0, image_rect.max.y - 8.0))
    .pivot(egui::Align2::LEFT_BOTTOM)
```

Propbar = canvas top-left. Status/Accept-Reject = canvas bottom-left. The
actual gesture (typing, clicking, dragging) can be **anywhere on the page**
— e.g. text edited in the lower-right corner of a landscape page commits via
a button rendered in the diagonally opposite corner. That is **three
independent points of attention for one editing action** — exactly the
"somewhere on the screen" the operator named, and it is structurally
identical across all three shipped tools (TextEdit L9364, AddText L10242,
Measure L11940), so the operator would have hit it on the very first
gesture they tried in any of the three. No reference app puts a commit
control in a third, independently-anchored location: Word/PowerPoint commit
on click-away with no dialog at all; SolidWorks's OK/Cancel checkmarks sit
at the **top of the same PropertyManager pane** the feature's own inputs
are in; Illustrator commits a shape on Enter or click-away, silently.

### D6. Escape cancels; nothing commits — an asymmetric keyboard story

`resolve_escape`'s four-way precedence chain (canvas.rs L458) is fully
built and wired for **cancel** (`EscapeOutcome::CancelGesture`), consumed
centrally in `collect_keyboard_actions` (main.rs L5366). There is **no**
matching universal "commit" chord. The one exception is AddText's own
private composing loop (L9959–9970: plain Enter accepts in point mode,
Ctrl+Enter in box mode) — a good, correct instinct, implemented once,
locally, and not extended to TextEdit or Measure. Half of the natural
keyboard commit/cancel pair exists app-wide; the other half exists in one
tool only. Every reference app treats Enter-commits/Escape-cancels as a
matched pair, never one without the other.

### D7. (today's defect) Selecting an object shrinks and re-fits the canvas — a second "the UI reacted to something I didn't ask to resize" symptom

Covered in full, with root cause, in §3. Named here because it belongs in
the same complaint family as the already-fixed zoom bug: a UI surface
visibly moving/resizing in response to something the operator did not
intend to be a resize event.

### D8. Icon-only vs. icon+text is applied per-control, not per-group

Nav/zoom/rotate/undo/redo are icon-only (name lives in the tooltip only).
Fit Page/Fit Width/Properties/Redact/Edit Text/Add Text/Edit Objects are
icon+text. The split appears to track "how often is this clicked" (a
reasonable heuristic, and each individual choice is separately defensible
— e.g. the icon-set spec's own P1-recorded rationale for leaving "100%" as
plain text) but there is no VISUAL signal marking which family a control
belongs to before you've learned it by trial. A ribbon (and, less
formally, a captioned toolbar group per D1) makes this legible by grouping:
within one labeled group, the convention is consistent, so the operator
only has to learn the rule once per group, not once per control.

### D9. No persistent mode cue beyond the toolbar button's own highlight

Entering a canvas tool (TextEdit, AddText, VectorEdit, any Measure
variant) changes nothing about the canvas itself except what the active
tool's own interaction code does — no cursor change, no persistent
"Tool: Edit Text" banner. The ONLY indicator that a tool is active at all
is the toolbar button's own selected-state styling (bold + outline ring,
per R84), which is easy to lose track of once the operator has scrolled or
zoomed and is looking only at the page. SolidWorks and Illustrator both
give the operator a persistent, glanceable mode cue independent of the
toolbar (cursor shape at minimum; SolidWorks additionally shows a
feature-in-progress banner). Smaller finding than D1–D7; recorded for
completeness (§5, P2).

### D10. The dock — the more expected "contextual info surface" — starts closed, so selection feedback is status-bar-only on a first session

`tools_open` defaults to `false` (main.rs L877). `selection_readout`
(L10971) renders **only** into the status bar. An operator's very first
selection click in a first session — which is exactly what triggered D7 —
therefore has its ONLY feedback channel be a status-bar text line, in a
panel that (per D7) is also the thing whose growth just made the canvas
jump. Two separately-real findings compounding at the exact same
interaction moment a first-time operator is most likely to hit.

---

## 2. The Accept/Reject redesign, reconciled with rule 4

### 2.1 The direct answer to the brief's question: yes, rule 4 is currently over-applied to case (a)

The code says so itself. `current_gesture_interrupt`'s doc comment
(main.rs L4872–4878), explaining why TextEdit chose `GestureInterrupt::
Discard` over `GestureInterrupt::Commit`:

> "a text edit's whole design is its separate Accept gesture, so
> Discard-on-interrupt is what makes 'operator-accepted, never silent' mean
> something"

That is citing rule 4 — "operator-accepted, never silent" is this
project's own paraphrase of fuzzy-never-sneaky — to justify gating a **plain
typed find/replace edit** behind an explicit Accept click. But rule 4's own
text is scoped to **algorithmic suggestions**: "Every algorithmic
suggestion — OCR text, an auto-detected form field, suggested Bates
ranges..." A find/replace edit where the operator typed both the search
text and the replacement text is not an algorithmic suggestion. It is
authored content, exactly as authored as a sentence typed into Word. Word
does not ask "Accept this typed sentence?" after every commit; Illustrator
does not ask "Accept this shape?" after you finish drawing it; both rely on
the same mechanism rule 2 already gives pdfce — undo, available until save.
The Pass 19.3 spec (this agent's own prior work, `docs/ui_specs/pass-19.3-
text-formatting-surface.md` §4.3) already reasoned its way to exactly this
conclusion for a narrower case (synthetic bold/italic) and retracted its
own draft confirm-dialog design as a result: "a synthetic-style Apply is
not \[in the blocking-question class\]: it is undoable pre-save... the
brief's own warning against 'a modal interrogation on every bold click' is
exactly right." This spec extends the same reasoning one layer up, from
"which strip renders a refusal" to "should a commit gate exist before the
strip is ever reached at all" — and the answer for **deliberate, literally-
authored content** is no.

**Distinguish precisely, per the brief's own framing:**

- **(a) Deliberate operator gesture — no confirmation needed, undo covers
  it.** TextEdit's plain find/replace (the operator typed both sides).
  AddText's authored content (the operator typed the text and chose the
  placement). MeasureLinear's plain two-point pick, **when the picked
  points are literal — not snapped to an inferred feature** (see below).
- **(b) Algorithmic guess — review-before-commit still required.**
  MeasureCircular's best-fit circle (`st.circular.fit()`, a Taubin fit
  computed FROM the operator's clicked points, not identical to any single
  click — the residual/radius the operator is being asked to accept is the
  algorithm's inference about where the true circle is). The **already-
  named** "derived-centerline confirm" (`ui_text::measure_confirm_derived_
  centerline`, L12000–12002, tagged in-code "fuzzy inference, §2.3.1") —
  a snap to an *inferred* centerline rather than a literal clicked point.
  Reflow's recomputed wrap/leading (already its own accept/reject
  sub-flow, L9401–9447, because the committed geometry is the *engine's*
  layout, not literally what the operator typed). Style-synthesis fallback
  (already resolved with no gate at all, per Pass 19.3 — a different
  resolution of the SAME distinction, arrived at independently).

**The pattern was already half-built into this codebase before this
spec.** The Measure tool already distinguishes `active_is_derived` from an
ordinary pick and shows a DIFFERENT confirm line for it (L12000–12002); it
just currently renders that distinction inside the same undifferentiated
Accept/Reject box as every plain pick. Reflow already keeps its own
separate accept/reject sub-flow for exactly the "this is the engine's
computed layout, not your literal input" reason. **Extend the distinction
that already exists; do not invent a new one.**

### 2.2 The mechanism already exists, unused, since Pass 12.0 — wire it, don't build a new one

`GestureInterrupt` (canvas.rs L412) has three variants: `Nothing`,
`Discard`, `Commit`. `Commit` was purpose-built for exactly this case —
"the active tool's typed draft with clear intent to keep it" (canvas.rs
L419–422) — and its consumer, `PdfceApp::commit_active_gesture` (main.rs
L4945–4950), is **still an empty stub**: `fn commit_active_gesture(&mut
self) {}`. Every tool shipped since Pass 12.0 (TextEdit, AddText, Measure)
chose `Discard` + an explicit Accept button instead, each independently
re-deriving a justification (L4876–4878's comment is the clearest one on
record). The substrate was designed to support both models; the app has
only ever exercised one of them.

**The fix wires `Commit` for the first time**, for exactly the case-(a)
family named in §2.1:

1. `current_gesture_interrupt` returns `GestureInterrupt::Commit` (not
   `Discard`) when the live gesture is TextEdit's plain pending edit,
   AddText's draft, or Measure's plain (non-`active_is_derived`) linear
   pick.
2. `commit_active_gesture` gains a real body: for each of those three
   cases, build the SAME request `do_accept`'s current code already builds
   (main.rs L9559–9584 for TextEdit; the AddText/Measure equivalents), and
   submit it through the SAME `EditSession` call already in use. **No new
   core API, no new command kind** — this reuses the existing Accept code
   path verbatim; only the *trigger* moves from "operator clicked a
   button" to "an interrupting action fired, or Enter was pressed, or the
   operator switched tools/selected something else."
3. **On refusal, the interrupt does NOT proceed and the draft is NOT
   discarded.** This is the one place the auto-commit design must be more
   careful than a plain "always commit on interrupt," and it matters for
   the same reason rule 2 (non-destructive by default) matters everywhere
   else: a spec-governed refusal (a font-trust-ladder refusal, an
   `EditError`) must never silently eat typed content the operator has not
   yet been told didn't take effect. Concretely: `commit_active_gesture`
   attempts the commit; on `Err`, it re-arms the SAME pending draft with
   the refusal text attached (exactly the existing `pending.last_refusal =
   Some(msg)` assignment at L9578–9581, unchanged) and the calling
   `resolve_gesture_interrupt` treats this as if `Discard` had been
   requested but nothing was discarded — the gesture stays open, the
   refusal renders inline (§2.4's merged panel), and the interrupting
   action itself is effectively deferred until the operator resolves it
   (either fixes the draft and lets it commit, or explicitly cancels with
   Escape). This is a genuine, if small, new piece of control flow — not
   a reuse of an existing branch — and should be reviewed as such by the
   engineer, not assumed trivial.
4. **Escape remains the explicit, sole "reject" mechanism**, using the
   `CancelGesture` outcome that already exists and is already wired
   (canvas.rs L463–464, consumed at main.rs L5366+). No new keybinding, no
   new dialog. This closes D6 for TextEdit/Measure at the same time (only
   AddText currently has a matching Enter-to-commit; §5 P1 extends Enter
   to TextEdit as the explicit "commit now" trigger, same as AddText's
   existing convention).
5. **Case (b) — MeasureCircular's best-fit, the derived-centerline confirm,
   Reflow — keep an explicit review step.** They are NOT wired to `Commit`;
   they keep exactly the accept/reject buttons they have today. What
   changes for them is only *where* those buttons render (§2.4).

### 2.3 Why "undo already exists" is a sufficient answer for case (a), and why it *isn't* for case (b)

Undo (`EditSession`'s command-log stack, `ARCHITECTURE.md` §11) covers "I
committed something and changed my mind" identically for a button-click
Accept and for an interrupt-triggered auto-commit — the committed command
is undoable either way, so removing the button changes *when* the commit
happens, not *whether* it can be reversed. That is what makes case (a)
safe to auto-commit: the operator authored the content directly, so there
is nothing to "catch" that the operator themselves didn't put there, and
undo is the same safety net Ctrl+Z already is everywhere else in the app.
Case (b) is different in kind, not degree: the content about to be written
is NOT what the operator directly specified — it's the algorithm's
inference from what they specified (a fitted circle's center/radius, a
re-flowed block's new line breaks). Undo still protects against a bad
outcome, but rule 4 additionally requires the operator see and
consciously accept the *specific inferred values* before they're written,
precisely because the operator cannot have "authored" a number they never
typed. This is the same distinction the standing UX rules already draw for
OCR text and auto-detected form fields — nothing new is being invented
here, only extended to a control family (tool commits) it had not yet
reached.

### 2.4 Even where a review step is kept, stop putting it in a third box

Whether or not `Commit` is wired for a given tool, **the propbar and
status Areas should merge into one floating panel per tool** — controls
on top, disclosures/refusal/(where kept) Accept-Reject at the bottom of
the SAME panel, matching SolidWorks's own PropertyManager convention (its
OK/Cancel checkmarks sit at the top of the same pane as the feature's
inputs, never in a separate window). This is a small, mechanical,
low-risk change independent of §2.1–2.3's taxonomy decision — it can ship
even if the engineer defers the auto-commit change — and it already fixes
the "two disconnected floating boxes" half of D5 on its own. Concretely:
delete `pdfce-text-edit-status`/`pdfce-add-text-status`/`pdfce-measure-
status` as separate `Area`s; append their content (cross-run notice,
Accept/Reject where still kept, refusal strip, disclosure strip) to the
bottom of `pdfce-text-edit-propbar`/`pdfce-add-text-propbar`/`pdfce-
measure-propbar` respectively, inside the same `egui::Frame::popup`.

### 2.5 Checklists (per this agent's own brief's mandatory format)

**Discoverability.** Case-(a) auto-commit needs no new affordance to
discover — it reuses the SAME visible live-preview the operator is already
looking at while typing/dragging; the "control" is simply "stop, and it's
done," which is the universal convention every reference app already
teaches. Case-(b)'s kept Accept/Reject buttons remain real `ui.button`s
(accesskit-visible, per the existing L9362 comment), now co-located with
the controls that produced the value being reviewed rather than in a
separate box — strictly more discoverable, not less. Escape-cancels /
Enter-commits should each get one line in whatever keyboard-shortcut
surface exists (P1-2 from the earlier GUI-polish audit, if built) —
tooltips alone are not enough for a chord with no visible button.

**Accessibility.** No new widget types. Merging two `Area`s into one
(§2.4) is strictly fewer Tab-stops to traverse per tool, not more. Removing
a button for case (a) removes one Tab-stop entirely for the common path,
which is a net accessibility win, not a regression — a screen-reader/
keyboard-only operator no longer needs to locate a spatially-disconnected
control to finish an edit.

**Fuzzy-never-sneaky.** §2.1–2.3 is precisely the exercise of getting this
checklist right at the CATEGORY level: algorithmic content (case b) keeps
full visible marking + explicit accept, unchanged; authored content (case
a) was never actually a case rule 4 was written to cover, so removing its
gate does not violate the rule — it stops mis-applying it. The refusal-
reopens-the-gesture behavior in §2.2 step 3 is itself a fuzzy-never-sneaky
guarantee at the failure-mode level: a refused commit never silently
discards operator-typed content.

**Immediate-mode fit.** No retained-mode assumption introduced. `Commit`'s
wiring is a pure state-transition function exactly like `Discard`'s
existing one (`discard_active_gesture`, L4910–4934) — same shape, same
per-frame-rebuilt `PdfceApp` fields, no new persistence pattern.

---

## 3. The status-bar/canvas-jump defect — root cause and recommended fix

### 3.1 Root cause, confirmed by reading the code, not inferred from the symptom

`egui::Panel::bottom("status")` (main.rs L5126) has **no fixed height** —
it auto-sizes to its content's rendered height, up to `status_bar`'s own
internal `egui::ScrollArea::vertical().max_height(STATUS_BAR_MAX_HEIGHT)`
cap (220.0 pt, L338, added by the earlier GUI-polish Pass's P0-4 fix — that
fix correctly capped runaway growth but did not make the panel's height
**invariant**, which is the actual property needed here). `selection_
readout` (L10971) returns immediately, rendering **nothing**, whenever
`doc.canvas_selection` is empty — so the ONLY moment its content (and
therefore the status panel's height) changes is the instant an object goes
from unselected to selected. Meanwhile `canvas()` computes its fit-mode
viewport **unconditionally, every frame**, directly from whatever space is
left after every panel above it has claimed its share:

```rust
let viewport = (
    (ui.available_width() - CANVAS_MARGIN).max(1.0),
    (ui.available_height() - CANVAS_MARGIN).max(1.0),
);
...
doc.view.apply_fit(extent, viewport, max_zoom);   // L7567, no gate, no "did this actually change" check
```

`FitMode::Page`/`FitMode::Width` re-derive zoom from `viewport` on every
single frame with no memoization and no check for *why* the viewport
changed. So: select an object → `status_bar_body` grows by one row →
the bottom `Panel`'s auto-height grows by that row's pixel height →
`CentralPanel`'s `available_height()` shrinks by the same amount →
`apply_fit` reads the smaller viewport and recomputes a smaller zoom →
the whole page visibly shrinks and re-centers, in the same frame the click
was processed. The measured numbers in the task brief (canvas rect
`[[313.5 71.0]-[1466.5 962.0]]` at zoom 0.7279 → `[[325.1 71.0]-[1454.9
944.0]]` at zoom 0.7132`, purely from a selection click) are exactly the
signature of this mechanism — a status-panel-height delta, not a genuine
window resize.

### 3.2 Why this belongs in the same complaint-family as the already-fixed zoom bug

Both are cases of a canvas-geometry computation reacting to a cause the
operator did not associate with "resize the view": scroll-wheel-under-
Ctrl in the zoom bug's case, a plain object-selection click here. An
operator who has just been told (by themselves) "this feels off, in ways I
can't fully name" is disproportionately likely to notice a second,
adjacent instance of the same *category* of jarring behavior, even though
the two are unrelated in code. Fixing this alongside §2's changes is worth
doing as one coherent "make the shell hold still unless I asked it to
move" pass, not filed as an unrelated cosmetic nit.

### 3.3 Recommended fix: reserve a FIXED height for the status panel, not an auto/capped one

Give `egui::Panel::bottom("status")` an explicit, constant height
(`.exact_height(…)` or the pinned `egui::Panel` API's equivalent —
**verify the exact builder name against the pinned egui version**, the
same hedge every prior spec in this project applies to unpinned-API
claims), sized to comfortably show the common case (2–3 lines, roughly
80–100 pt) with the EXISTING internal `ScrollArea` (already correctly
non-suppressing, per P0-4) absorbing anything beyond that inside the
fixed budget. This is a **one-line change in kind** (swap an implicit
content-driven height for an explicit constant) but it is the property
that actually matters: the panel's contribution to `CentralPanel`'s
available space becomes **invariant across frames regardless of what the
status bar has to say**, which is what stops `apply_fit` from ever seeing
a viewport change that wasn't caused by something the operator did on
purpose (a real window resize, an explicit Fit-mode click, opening/closing
the rail or dock — all of which SHOULD re-fit, because the operator caused
them directly and a re-fit in response is expected, not jarring).

**Why not the alternatives, considered and rejected:**

- **Move `selection_readout` out of the status bar entirely** (e.g. into
  the Objects/Properties dock panel) fixes only the ONE trigger observed
  today. `edit_note`, `save_result`, `copy_result_bar`, and the render-
  diagnostics header can ALL independently grow/shrink the same auto-
  height panel (a delete, a save, a copy-text run), so this alone leaves
  the underlying defect live for every other disclosure event — it treats
  the symptom's one observed trigger, not the mechanism. Worth doing
  ANYWAY as a D10 fix (§5, P1) — surfacing selection info in the dock too
  — but not sufficient on its own.
- **Decouple `FitMode::Page` from firing every frame** (recompute only on
  a genuine window resize or an explicit Fit-mode click, not on every
  frame's `ui.available_size()` reading) is the more thoroughly "correct"
  architectural fix in spirit — a fit mode's contract arguably SHOULD mean
  "stay fit to genuine layout changes," not "recompute unconditionally from
  whatever this frame's panel layout produced" — but it requires tracking
  *why* the viewport changed (window resize vs. sibling-panel content
  churn), which egui does not hand you for free; it is real, valuable,
  future-Pass-sized work, not a P0 surgical fix. Recorded as a P1/P2
  option (§5) rather than the recommended P0.

A fixed-height reservation costs a small amount of permanently-visible
vertical space even when the status bar has nothing to say — an
entirely conventional trade (this is how a status bar/terminal panel in
most desktop IDEs and editors already behaves) and strictly preferable to
a page that visibly jumps on click.

**No standing rule is weakened by this fix.** Rule 4's "no disclosure line
may be suppressed" is unaffected — the existing `ScrollArea` already
guarantees every line is still reachable by scrolling; only the OUTER
panel's height stops being reactive to how many lines happen to be
present this frame. This is the same non-suppression argument the earlier
P0-4 fix already made for its own 220pt cap, applied one level up.

---

## 4. The ribbon question

### 4.1 Is a ribbon right for pdfce?

**Directionally, yes — for a specific, rule-grounded reason, not because
the operator asked for it by name.** Standing rule 3 ("progressive
disclosure over Acrobat's own worst habit... lean primary toolbar...
advanced feature groups in secondary panels") describes almost exactly
what a well-built ribbon's tab system IS FOR: a "Home" tab carries the
lean, always-relevant set; less-frequent feature families (redaction,
Bates stamping, forms, OCR) live behind their own named tab, present but
not cluttering the default view. D1/D2/D8 above are, structurally, the
exact failure mode rule 3 warns about — arriving bottom-up rather than
top-down, but landing in the same place: one overloaded, uncaptioned
cluster. A ribbon's CONTEXTUAL tabs additionally solve D4/D5's placement
problem directly and permanently: instead of a floating palette that
starts in an arbitrary corner, a "Text Format" contextual tab appears
**in the same fixed ribbon location every time**, the moment TextEdit has
a caret in a run, and disappears the moment it doesn't — exactly
SolidWorks's own PropertyManager convention, generalized into the
ribbon's own idiom rather than a separate side pane.

**But a ribbon is necessary, not sufficient, for the operator's actual
complaint.** §2 and §3 above are real, independently-shippable defects
that a ribbon wrapped around them would not fix by itself — a beautifully
captioned, tabbed ribbon that STILL opens a floating Accept/Reject box in
the opposite corner from the gesture, or STILL lets the canvas jump on
selection, would still feel foreign. Ship §2/§3 first regardless of the
ribbon decision; they are cheap, surgical, and independently
operator-visible. Treat the ribbon as the larger, separate initiative
this section scopes.

### 4.2 Cost, honestly

egui has no first-party ribbon widget. Building one — a multi-row,
group-captioned, tabbed control strip with a contextual-tab mechanism — is
custom `Ui` composition, entirely feasible in egui's immediate-mode model
(it's fundamentally nested layout + conditional rendering, nothing egui
can't do), but it is genuine new UI infrastructure, not a restyle of the
existing `toolbar_controls` function. It is a Pass-sized initiative with
its own spec, not a same-session change. Scoping it honestly here so the
engineer can weigh it against §5's cheaper wins rather than either
under- or over-committing to it in one sitting.

### 4.3 A cheaper, ribbon-*adjacent* interim, if the full ribbon is deferred

Three changes, all mechanical, all shippable against the EXISTING flat
toolbar with no new widget architecture, each closing part of D1/D2/D8
without committing to the tab system:

1. **Visible captions under each existing separator-divided group**
   (e.g. a small `ui.label` reading "File" / "View" / "Navigation" /
   "Zoom" / "Edit" / "History" beneath or before each cluster). Directly
   closes D1.
2. **Split the nine-control "edit" group into two labeled groups** — e.g.
   "Page" (rotate ×2, Properties) and "Markup & Tools" (Markup ▾, Text ▾,
   Edit Text, Add Text, Edit Objects, Measure ▾, Redact) — a pure
   re-grouping, no behavior change. Directly closes D2.
3. **One icon-style rule per group, applied consistently** (either every
   control in a group carries a visible caption, or none do — never
   mixed within one group). Directly closes D8.

This is explicitly a **stopgap**, honestly labeled as such — it will
narrow the gap to "modern software" without fully closing it, because the
operator specifically named the ribbon paradigm (tabs + contextual
surfaces), which this interim does not attempt. Whether to ship the
interim, go straight for the full ribbon, or both in sequence is a
scheduling call for the engineer (per this agent's own brief's boundary),
not decided here.

### 4.4 Proposed tabs, if/when the full ribbon is built

A first sketch, reusing pdfce's own existing groupings as the seed rather
than inventing new categories:

- **Home** (default tab). File (Open/Save), Navigation (page/prev/next),
  Zoom (in/out/fit page/fit width/100%), History (Undo/Redo), the rail
  toggle, the annotation-visibility toggle. Everything currently in the
  file/view/navigation/zoom/history groups, unchanged in substance.
- **Insert.** Markup shapes, Text/Sticky/Stamp, the Add Text tool, the
  Measure ▾ family (linear/circular/scale — authors **ce dimensions**).
  Today's Markup ▾/Text ▾/Add Text/Measure ▾ cluster, given a home that
  matches "things that add new content to the page" as the group's
  organizing question.
- **Edit** (or "Page Content"). Edit Text tool, Edit Objects tool, rotate,
  and — a genuine improvement over today's placement — the page-structure
  operations currently buried in the Batch Tools dock panel (Combine /
  Split / Insert pages), since those act on the open document exactly as
  directly as rotate does.
- **Protect.** Redact, alone today. The dedicated-tab answer to the
  tension the existing L6125–6170 code comment already names explicitly
  (rule 3 wants it off the primary surface; rule 7 wants it discoverable)
  — a named tab is "present, one click away, never primary-toolbar
  clutter," which is a strictly better resolution than the current
  stopgap (one icon+label control at the end of an already-overloaded
  group). Room to grow into Bates stamping/PDF-A conversion/OCR as those
  ship, per rule 3's own backlog list.
- **Contextual: "Text Format"** — appears only while `CanvasTool::TextEdit`
  has an active caret/selection. Hosts exactly what today's floating
  TextEdit propbar hosts (Size/Colour/Font, the 19.3 spacing/style rows)
  — this is the surface that makes D4's floating-palette problem
  disappear for TextEdit specifically, by giving it a fixed, predictable,
  always-in-the-same-place home instead of a draggable window.
- **Contextual: "Object"** — appears while `CanvasTool::VectorEdit` is
  active or a canvas object is selected. Candidate future home for the
  rail's selection action bar.
- **Contextual: "Dimension"** — appears while any Measure tool is active
  or a **ce dimension** is selected. Hosts the Measure propbar's controls
  AND, per §2.2 step 5, the case-(b) review controls that legitimately
  keep an explicit accept step (MeasureCircular's best-fit, the
  derived-centerline confirm) — co-located in the tab's own fixed
  location, closing the rest of D5 for the cases §2 does not auto-commit.

**Where the existing dock panels go: nowhere new.** A ribbon replaces
`toolbar_controls`'s single-row implementation; it does not replace the
dock. R80/R81/R82 already govern the dock as a distinct, correctly-placed
architecture, and every one of the operator's three reference apps ALSO
keeps a persistent side dock alongside its ribbon (SolidWorks's
FeatureManager tree beside its CommandManager ribbon; Word's Navigation
Pane beside its ribbon) — this is not an either/or, and scoping the ribbon
as "toolbar-widget replacement only" keeps the blast radius much smaller
than "rebuild the whole shell." Objects tree, Properties (`/Info`
metadata), Batch Tools, and Redact's own review surface all stay exactly
where R80's `DockPanel` enum already puts them.

**Rule-12 note, named explicitly because it is adjacent to a real
boundary.** Project rule 12 forbids sourcing pdfce's GUI structure from
Acrobat's actual menu paths/panels/dialogs — capability parity only, never
GUI-mechanics copying. Adopting the ribbon PARADIGM (tabs + captioned
groups + contextual tabs) does not breach this: it is a widely-shared,
non-proprietary interaction pattern the operator explicitly named from
THREE different applications, not a lift of Acrobat's specific ribbon
layout. The tab captions and groupings sketched above are pdfce's own,
designed independently from what any of the three reference apps actually
put on their own tabs (e.g. "Protect" is a plain English word describing
what the tab contains, chosen because it is the honest name for the
content — not because Acrobat also uses that word). Flagging this
distinction explicitly so the engineer can make the same call
consciously if the tab set is refined later, not because anything sketched
here crosses the line.

---

## 5. Prioritized change list

Ordered by (operator-visible improvement) / (implementation cost). P0 =
ship first.

### P0 — cheap, surgical, directly answers today's feedback

1. **Fix the status-bar/canvas jump** (§3.3): give `egui::Panel::bottom
   ("status")` a fixed height; keep the existing internal `ScrollArea`
   for overflow. Independent of everything else on this list; ship
   first, it's the smallest, most isolated change here.
2. **Merge each tool's `…-status` Area into its `…-propbar` Area**
   (§2.4): one floating panel per tool instead of two, regardless of the
   Accept/Reject taxonomy decision. Mechanical, low-risk, already closes
   half of D5 on its own.
3. **Wire `GestureInterrupt::Commit` for case-(a) deliberate content**
   (§2.2): TextEdit's plain find/replace, AddText's authored content,
   MeasureLinear's plain (non-derived) pick. Fill in the currently-stub
   `commit_active_gesture`. Keep Escape as the sole reject mechanism
   (already built). Handle the refusal-reopens-the-gesture case
   (§2.2 step 3) carefully — it is new control flow, not a reuse.
4. **Keep and relocate (not remove) the case-(b) review step**
   (§2.2 step 5): MeasureCircular's best-fit, the derived-centerline
   confirm, Reflow's accept/reject — unchanged in mechanism, now
   rendering inside the merged panel from item 2 instead of a separate
   box.

Suggested order: 1 and 2 first (fully independent of each other and of
3/4, lowest risk), then 3/4 together (the actual interaction-model
change, needs the case-(a)/(b) taxonomy settled).

### P1 — clear net improvement, moderate cost

5. **Ribbon-adjacent interim on the flat toolbar** (§4.3): group
   captions, split the overloaded edit group, consistent icon-style per
   group. Narrows the "doesn't match other software" gap without
   committing to the full ribbon.
6. **Enter-commits for TextEdit**, matching AddText's existing
   point-mode/box-mode convention (§2.2 step 4's note; closes D6 fully).
7. **Anchor the merged property panel near the gesture/selection**
   rather than a fixed canvas-corner default (still movable). Reduces,
   without a ribbon, the "where do I look" cost of D4 — genuinely harder
   than items 1–4 (needs per-frame anchor tracking against the caret/
   selection position), hence P1 not P0.
8. **Surface selection feedback in the dock too**, and consider
   defaulting the dock open on first use or the first time a selection
   is made (§3.3's rejected-alternative note; D10). Complements item 1,
   does not replace it.

### P2 — larger, valuable, not required to answer today's feedback

9. **The full ribbon** (§4.4): tabs, contextual tabs. A genuine,
   separate Pass/spec-sized initiative; the tab sketch above is a
   starting point, not a final design.
10. **A persistent tool-mode cursor/banner cue** (D9), independent of
    the ribbon question.
11. **Redact's own "Protect" ribbon tab**, once the ribbon exists —
    today's toolbar placement (end of the edit group) is a reasonable,
    already-justified interim per the existing code comment; no urgency
    to change it before the ribbon does.
12. **Decouple `FitMode::Page` from firing every frame** (§3.3's
    rejected-alternative note) — the more architecturally thorough fix
    for the canvas-jump family of defects, valuable as a hardening pass
    once item 1 has removed the only currently-known trigger.

### Standing-rule interactions, flagged explicitly per this agent's brief

- **§2's removal of Accept/Reject for case (a) does not weaken rule 4.**
  It corrects an over-application (§2.1); the algorithmic-inference cases
  (b) keep their explicit review step unchanged. Read this as "the rule
  was being applied to the wrong set of things," not "the safety rail
  was cut" — the engineer should frame it the same way if it needs
  explaining later (e.g. to the operator, or in a decision-log entry).
- **§2's merged floating panel does not create a new instance of the
  floating-window pattern R81 restricts.** It reduces the count of
  simultaneous per-tool floating panels from two to one, staying inside
  the already-precedented, already-shipped "tool-scoped floating Area,
  visible only while that tool is active" category R81 already tolerates
  (transient, not "kept open while working" in the dock sense). No rule
  is being stretched to accommodate this change.
- **§3's fixed-height status panel does not suppress any disclosure
  line** — same non-suppression argument the existing P0-4 fix already
  established for its own 220pt scroll cap, applied to the outer panel
  too. Cite this explicitly if anyone questions whether reserving a
  fixed height conflicts with "every disclosure line may not be
  suppressed."
- **§4's ribbon paradigm does not breach rule 12** (Acrobat GUI-mechanics
  never copied) — see §4.4's rule-12 note for the reasoning; flagged
  there in detail because it is the one place in this whole document
  that sits close enough to that boundary to be worth naming
  consciously rather than assuming clear.

---

## 6. Items for the engineer, not mine to decide

- **Whether §5's P0 items 3/4 (the `GestureInterrupt::Commit` wiring)
  ship in the same session as items 1/2 (the mechanical merge/fix), or
  in a follow-up.** Items 1/2 are safe, small, and independently
  valuable; item 3/4 is a genuine interaction-model change that touches
  three tools' commit paths and deserves its own careful review pass —
  a scheduling call, not mine.
- **Whether the §4.3 ribbon-adjacent interim ships at all**, versus
  going straight to scoping the full ribbon (§4.4) as its own Pass. Both
  are legitimate; I've scoped both so the choice is informed either way.
- **The exact fixed height for §3.3's status panel** (I suggested
  80–100 pt / 2–3 lines as a starting point) — a real product/spacing
  judgment best made by looking at the running app, not derived from
  reading source alone, per this agent's own P2-5 precedent in the
  earlier GUI-polish audit ("everything in this document was found by
  reading source, not by running the app").
