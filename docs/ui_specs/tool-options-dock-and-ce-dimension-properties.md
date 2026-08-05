# Tool Options dock + ce-dimension property surface (ui-spec)

**Author:** `pdfce-ui-specialist` · **Date:** 2026-08-05 · **Drives:**
operator feedback, verbatim, 2026-08-05:

> "let's just focus on making the gui more user friendly for now. When I
> select a tool like the edit text one all of the options should be shown in
> a side bar tab docked with the page navigation tab. There should be no
> separate accept/reject when editing the text - if I click out of where I
> am editing that should just accept the edits. this goes the same with all
> tools. the ce dimensions i add need to be editable as well. they should
> give me an option to change the units, number of decimal places shown,
> fractions, tolerance and tolerance types like solidworks has, drag to
> extend and retract the extension lines and the position."

**Terminology (CLAUDE.md rule 15, binding throughout):** every dimensioning
object discussed below is a **ce dimension** — a `/Line`+`/IT
/LineDimension` annotation pdfce itself authors
(`crates/pdfce-core/src/dimension/`), never a **pdf dimension** (a
pre-existing CAD-exported callout). Nothing in this document concerns pdf
dimensions.

**I do not write code.** Everything below is critique + a concrete change
list for the engineer to implement or push back on.

---

## 0. Relationship to the existing, unshipped 2026-08-04 spec — read this first

Roughly half of this request (§B below) is **already fully specified** in
`docs/ui_specs/gesture-commit-and-shell-conventions-audit.md` (this agent's
own prior work) and **has not yet been implemented**. Confirmed by reading
the current source, not assumed:

- `commit_active_gesture` (main.rs L5299) is still the literal empty stub
  `fn commit_active_gesture(&mut self) {}` the prior spec found.
- TextEdit/AddText/Measure still render **two independent floating
  `egui::Area`s each** — `pdfce-text-edit-propbar`/`-status` (L9459/9973),
  `pdfce-add-text-propbar`/`-status` (L10733/10859), `pdfce-measure-
  propbar`/`-status` (L13077/13196) — the merge the prior spec's P0 item 2
  recommended has not shipped.
- The status panel's fixed-height fix (that spec's P0 item 1) **has**
  shipped (`STATUS_PANEL_HEIGHT_PTS = 92.0`, L266, `.exact_size(...)` at
  L5586) — so that part of the operator's original "weird GUI" complaint is
  already resolved and is not revisited here.

**Practical consequence for the engineer:** treat §B of this document as
"confirm, extend, and re-prioritize the existing spec," not as new design
work to redo from scratch. The two documents should be read together; where
they agree I cite the older one rather than repeat its reasoning.

What genuinely **is** new here, not covered by the 2026-08-04 spec: the
dock relocation (§A), VectorEdit's status as an already-compliant precedent
(§B.7), MeasureScale's explicit case-(a)/(b) classification (§B new
finding), and the entire ce-dimension property surface (§C, all new).

---

## A. The Tool Options panel

### A.1 Resolution: a second, independent left-hand dock — not a right-dock merge, not a hand-rolled tab bar

Today's left rail is `egui::Panel::left("thumbnails")` (main.rs L5589), a
plain fixed panel gated on `rail_expanded` — not a dock tab of any kind.
The right side is `dock.rs`'s `egui_tiles::Tree<DockPanel>` (Objects/
Properties/BatchTools/Redact). Three ways to give Tool Options a home
beside page navigation, in order of how strongly I recommend them:

1. **Recommended: a second, independent `egui_tiles::Tree` on the left**,
   with its own small pane enum (e.g. `LeftPanel::{Pages, ToolOptions}`),
   its own `Behavior` impl reusing `DockBehavior`'s exact mechanism —
   `on_tab_button`'s `WidgetInfo` pattern, `tab_title_for_tile`'s
   bold-on-active (R84), `simplification_options`'s
   `all_panes_must_have_tabs: true` gotcha (dock.rs module docs, gotcha 2)
   — either by genericizing `DockBehavior<'a>` over the pane type or by a
   sibling ~40-line module. **This is the one implementation-shape
   question I leave to the engineer** (generic vs. duplicate); both are
   legitimate, and duplication is not shameful at this size if genericizing
   turns out awkward against `egui_tiles`' own trait bounds.
2. **Rejected: fold Tool Options into the existing RIGHT-hand `DockTree`**
   as a fifth `DockPanel` variant. Wrong side of the window — page
   navigation and its now-paired options belong where the operator's eye
   already is when navigating, not across the canvas from it — and it would
   put a fifth label into groups that A.3's own invariant caps at two,
   forcing a fresh re-balancing of a layout that currently satisfies its own
   test (`no_default_tab_group_holds_more_than_two_panes`) cleanly.
3. **Rejected: a second `Panel::left`, hand-rolled tab bar (Objects-panel
   style, pre-`egui_tiles`).** Decision 017's whole point was replacing a
   hand-rolled two-compartment row list with `egui_tiles` because the
   operator asked for "flexible docking that works as well as Inkscape's."
   Building a *second*, differently-styled tab convention on the left,
   right after the right side got the real thing, reintroduces exactly the
   "doesn't match anything I've seen" inconsistency the 2026-08-04 audit's
   D1–D3 already diagnosed for the toolbar. Consistency compounds (this
   agent's own brief, §2.2); don't invent a fourth tab convention.

Why a genuinely SEPARATE tree rather than widening the one dock to span
both sides: the existing right dock's A.3 vertical-split invariant
(Objects/Properties visible simultaneously) is untouched by this — zero
risk of breaking a test that already exists and passes. `Pages` is a
scrolling thumbnail strip with its own persistent scroll position, a
different widget shape from every existing `DockPanel` (which are forms/
trees/lists, not an image-thumbnail rail) — reusing the *tabbing mechanism*
is right; treating it as one more member of the SAME tree as Objects/
Properties/Redact is not required by anything the operator asked for and
adds risk for no benefit.

### A.2 The new default layout

Right dock: **unchanged** — Objects/Redact over Properties/BatchTools,
exactly as `dock.rs`'s `default_tree()` builds it today. Ken did not ask to
touch this; no reason to re-decide A.3.

New left dock, default:

```text
tabs [ Pages | Tool Options ]
```

One tab group, two labels — trivially satisfies the "no default tab group
holds more than two labels" invariant (dock.rs L266-269) without needing to
re-litigate it; there is nothing to split because there are only ever two
panes on this side. `Pages` is the front tab at launch (it's the
general-purpose, always-relevant surface — no tool is armed on a fresh
session). `Tool Options` comes forward automatically the moment a tool is
armed (A.3), using the SAME `dock::activate`/`make_active` mechanism
already built and tested for the right dock (dock.rs L339-341) — reused
verbatim, not reinvented, and it is a pure tab-selection walk with no
widget-focus side effect (confirmed by reading `make_active`), so bringing
Tool Options forward does not also steal keyboard focus from wherever the
operator was typing.

### A.3 Tool Options when no tool is armed, auto-raise, and the panel title

**Not literally empty.** A blank tab is a worse discoverability failure
than a slightly early one — first-session operators would have no reason
to ever look at it. Show one caption line, drawn from `ui_text` (rule R1,
"through the catalog"), naming the tools that populate it: e.g. *"No tool
armed — choose Edit Text, Add Text, a Measure tool, or Edit Objects from
the toolbar to see its options here."* This IS a discoverability control in
the checklist sense (§1 of this agent's brief): it teaches the pairing
between toolbar and dock before the operator has discovered it by trial.

**Auto-raise: yes, on ARM only, never forced back on disarm.** The rising
edge `active_tool: None → Some(tool)` (or a tool swap) calls
`dock::activate(&mut self.left_dock, LeftPanel::ToolOptions)`. This is not
a focus-steal hazard per A.2's note. Disarming a tool (Escape, explicit
tool deselect) does **not** snap the tab back to Pages — it leaves Tool
Options showing the empty-state caption from above. Forcing a snap-back
would itself be an unrequested motion of the kind the 2026-08-04 audit's
D7 already diagnosed as jarring ("the shell should hold still unless the
operator asked it to move") — here applied to a tab flip instead of a
canvas re-fit, same principle. The operator can click back to Pages
themselves if they want it.

**Panel title: the TAB label stays constant ("Tool Options"), the BODY's
first line changes per tool.** Renaming the tab caption itself per active
tool would fight `DockBehavior`'s architecture, which expects `label()` to
be a pure function of the pane enum (dock.rs L232-239), not of transient
`PdfceApp` state — and a tab whose printed name keeps changing is a worse
target to relocate by muscle memory (R84's whole point is a STABLE,
learnable visual identity for a tab). Instead, reuse the EXACT existing
per-tool title convention each floating propbar already has — e.g.
`ui.label(ui_text::text_edit_propbar_title())` (L9550) — just relocate that
label to the top of the docked pane's body. No new naming pattern; the
same one, moved.

### A.4 Where the disclosure surfaces go — enumerated per surface, because a wrong answer here silently suppresses one

None of these are "options"; all are R20/rule-4 disclosure surfaces and
must not be quietly dropped in the relocation.

- **Refusal strip** (a spec-governed `EditError` — encoding, missing
  glyph): moves into the docked Tool Options pane, at the bottom, verbatim
  from core (unchanged content, new host). See §B.8 for the NEW tab-badge
  behavior this panel adds on top.
- **Disclosure strip** (e.g. `measure_dimension_authored`,
  `measure_scale_applied`, refusal lines): same — bottom of the docked
  pane.
- **Cross-run notice** (TextEdit's cross-run-selection refusal, per this
  agent's Pass 14.3 memory): same — bottom of the docked pane, not the
  status bar. Reason for NOT the status bar: that panel is now a small,
  FIXED 92pt height (main.rs L5586) shared by save/delete/copy narrator
  lines project-wide; stuffing tool-specific disclosures into it either
  crowds that general-purpose convention or overflows its own scroll cap
  the moment a tool has anything non-trivial to say. The docked pane has
  no such shared-budget constraint.
- **Live reflow diagnostics** (Pass 15.2's overflow-disclosed-calmly line):
  same placement, same reasoning — it is TextEdit's own sub-mode disclosure,
  belongs with TextEdit's other content in the same pane.
- **Case-(b) explicit Accept/Reject buttons** (MeasureCircular's best-fit,
  the derived-centerline confirm, Reflow's own accept/reject) — per §B,
  these are KEPT, not removed. They render at the bottom of the docked
  pane, same content and mechanism as today, new host only.

Net shape of the Tool Options pane body, top to bottom: **[per-tool title]
→ [tool's own controls] → [separator] → [disclosure/refusal strip,
verbatim from core] → [case-(b) accept/reject buttons, only for the tools
that still keep them]**. This is exactly the 2026-08-04 spec's §2.4 merge
recommendation, with the destination changed from "one merged floating
Area" to "the docked pane" — cheaper for the engineer to implement now that
a dock exists to receive it, since it removes the floating-Area code
entirely rather than building a new merged Area first and then migrating it
later.

### A.5 The floating Areas: delete, do not keep as a fallback

Delete `pdfce-{text-edit,add-text,measure}-{propbar,status}` once the
docked version ships. Precedent directly on point, cited rather than
re-derived: Pass 18.4 retired the floating `properties_window` for
exactly this reason once `DockPanel::Properties` existed, and decision 017
A.4 #2 is explicit that a **float-OR-dock dual mode is deliberately not
supported** — "two code paths for the same content, each duplicating
open-state, position/size and focus handling, for zero operator benefit at
this scale." Keeping the floating Areas "as a fallback" would recreate
precisely that duplication for these three tools; there is no scenario
where an operator needs the floating version once the dock exists, since
the left dock is present in the default layout (unlike an optional pane the
operator might have closed).

---

## B. Implicit commit — confirming, extending, and one new finding

### B.6 The line rule 4 draws: confirmed, with the rule-4 text as the argument

The engineer's read is correct, and it is exactly what the 2026-08-04 spec
already concluded (§2.1, quoting rule 4's own text: "Every algorithmic
suggestion... is a reviewable hint the operator accepts or overrides").
Rule 4 as narrowed 2026-08-05 (decision 024 §4.4, `CLAUDE.md` rule 4) makes
this explicit: the obligation is on things pdfce **guessed**, not on a
"direct manipulation whose result is fully visible on the canvas and
reversible in one undo." Typing a replacement, dragging a node, placing a
ce dimension by two literal picks are all authored content — there is
nothing in them for the operator to "review" that they didn't put there
themselves. I have nothing to add to that reasoning; it is settled and
correctly scoped.

### B.7 Per-tool/gesture enumeration — including VectorEdit, and one positive finding

**VectorEdit already does exactly what Ken is asking for, project-wide.**
This was not named in the 2026-08-04 spec's taxonomy sweep (it predates
this operator request), but confirmed by reading the code: VectorEdit's
node drag, subpath move, and object delete have **no** floating propbar/
status Area at all, and `run_dimension_drag` (main.rs L12649-12835, which
governs a ce dimension's position drag — tool-independent, not even gated
to a Measure tool being armed) commits directly `on release`, with a live
preview during the drag and one `EditSession::place_dimension`/
`move_object`/`move_node` call on interrupt — no button, no separate box.
**Cite this as the working precedent the TextEdit/AddText/MeasureLinear
Commit-wiring should match**, not a hypothetical design — it is already
running in this app today, for a different tool family.

Per-tool table:

| Tool / gesture | Case | Commit trigger |
|---|---|---|
| TextEdit plain find/replace | (a) | click-out / tool-swap / Enter (extends AddText's existing convention) |
| TextEdit Reflow draft | (b) — engine-computed line breaks, kept | explicit Accept, unchanged |
| AddText authored run/box | (a) | click-out, Enter (point mode)/Ctrl+Enter (box mode) already exist; extend "click elsewhere on canvas" as an additional trigger for mouse-only operators |
| MeasureLinear plain 2-point pick | (a) | click-out, once BOTH points are picked (see mid-gesture below) |
| MeasureCircular best-fit | (b) — Taubin fit has a real residual | explicit Accept, unchanged |
| Derived-centerline confirm | (b) — fuzzy inference, in-code tagged | explicit Accept, unchanged |
| MeasureScale back-calc | **see new finding below** | recommend: KEPT explicit Accept |
| VectorEdit node/subpath/object drag | (a) — **already shipped this way** | commit on release, no change needed |
| ce-dimension position drag (27.0/27.1) | (a) — **already shipped this way** | commit on release, no change needed |

**Mid-gesture state that is not yet a complete edit (one of two
MeasureLinear picks taken): commit is genuinely impossible, and that's
fine — it isn't a commit decision at all.** Nothing has been added to the
document yet, so there is nothing for undo to protect and nothing rule 4
governs. Distinguish two triggers:

- Clicking elsewhere **on the canvas** (not the second point) — not an
  interrupt at all under the current design; it simply restarts the pick.
- Clicking **off the canvas** (toolbar, dock, another tool) mid-pick — this
  IS an interrupt, and the honest behavior is **discard**, disclosed with
  one line ("point 1 discarded — tool re-armed"), not silent. This is not
  a rule-4 violation: nothing was ever authored OR inferred into a
  committable state, so there is nothing to protect by keeping it — same
  category as closing a dialog with one field half-typed.

Concretely, this means `current_gesture_interrupt` needs to distinguish
"no committable value yet" (→ `Discard`) from "a complete, committable
draft exists" (→ `Commit`) — which is exactly what checking
`pending.is_some()` (or the tool's equivalent) already gives it; not new
complexity, just the right branch condition.

**New finding: MeasureScale needs its own explicit classification, and I
recommend it stays case (b) — for a reason distinct from "is it inferred."**
The prior spec did not classify it. A back-calculated scale
(`real_length / drawn_pdf_length`, or a typed ratio) is deterministic and
exactly reproducible from what the operator typed — by the "authored vs.
inferred" test alone it reads as case (a). But it fails a THIRD test worth
naming explicitly: **blast radius**. Every other case-(a) commit changes
exactly the one object the operator is looking at. A scale commit
changes the DISPLAYED VALUE OF EVERY OTHER MEMBER of the group
simultaneously — dimensions elsewhere on the page, possibly off-screen,
that the operator is not looking at right now (the existing code comment
at L13413 already says this out loud: "a calibration silently rescales
every dimension in the group"). Auto-committing that on a stray click-out
is a materially different risk from auto-committing one text edit or one
new ce dimension. **Recommend: keep MeasureScale's explicit Accept, named
as a deliberate exception to the case-(a) default on blast-radius grounds,
not as an unresolved case-(b) classification** — flag this for the
engineer (and, if it matters, the operator) to confirm, since it is a
judgment call I'm making rather than a mechanical application of the
already-settled rule-4 line.

### B.8 The refusal case — surfaced, retained, and now badgeable

Generalizes the 2026-08-04 spec's §2.2 step 3 across every case-(a) tool,
plus one genuinely new recommendation this dock relocation enables.

On an interrupt-triggered commit attempt that core refuses: (1) the draft
is **retained**, never discarded; (2) the refusal renders verbatim from
core in the docked pane's bottom disclosure strip (§A.4) — which is now a
**fixed, always-visible location**, a real improvement over the floating-
Area version, since a refusal in a permanent dock tab cannot be scrolled
off-screen or missed the way a floating box the operator just clicked away
from could be; (3) the interrupting action itself is deferred — the tool
does not actually disarm and focus does not move to whatever was clicked —
until the operator resolves it (fix-and-commit, or explicit Escape-discard).

**New recommendation, not in the prior spec (it had no dock tab to badge):
while an unresolved refusal is pending, mark the Tool Options TAB itself**
— a bold weight or a small glyph badge, paired per R84 (never colour
alone) — so an operator whose eye is on the canvas, not the dock, still
sees "something needs your attention here" from the tab strip. Without
this, moving the refusal into a dock tab that starts hidden-behind-Pages
(if the operator manually switched back) would be a regression versus the
always-foreground floating Area it replaces; the badge is what keeps it a
strict improvement.

### B.9 Escape/commit discoverability without a button pair

Escape remains the sole, universal discard-without-commit chord for a
case-(a) tool's current draft — unchanged from the prior spec. To teach
"click-out commits / Escape discards" with no visible button pair:

- The docked pane shows **one static caption line**, from `ui_text`, ONLY
  while a committable draft is pending (`pending.is_some()`) — e.g. "Esc
  discards this edit; clicking elsewhere keeps it." Conditional, so it
  isn't permanent clutter, but present at exactly the moment it's
  actionable — the first time a first-session operator types a change.
- Whatever keyboard-shortcut reference surface exists or gets built (prior
  spec's P1-2) gets one line each for "commit on click-away" and "Esc —
  discard."

### B.10 A gesture where implicit commit is genuinely wrong

Two, both worth naming by name:

1. **An empty AddText draft.** Operator arms AddText, clicks a point,
   types nothing, clicks away. Auto-committing here would add a real but
   invisible, zero-content annotation/content object to the document for
   no operator-visible reason — a foot-gun a "click-out=accept" design
   must not create. `current_gesture_interrupt`'s AddText branch must
   check for non-empty content before returning `Commit`; empty content
   stays `Discard`.
2. **MeasureScale's commit** (§B.7's new finding) — wrong to auto-commit
   not because it's inferred, but because of its blast radius across the
   whole group.

---

## C. The ce-dimension property surface

### C.11 What already exists — read before designing anything new

Confirmed by reading `crates/pdfce-core/src/dimension/{group,units}.rs` and
`crates/pdfce-gui/src/main.rs`'s `run_dimension_groups_panel`/
`run_dimension_drag`, **most of Ken's list already exists, at the GROUP
level, already wired to real controls** (Pass 25.5/27.0/27.1/27.2):

- **Units** — `Unit` (six values incl. feet-inches), selectable today in
  both the MeasureScale propbar and the Group Manager window.
- **Decimal places** — `FractionMode::Decimal { places }`, a `DragValue`
  0–6, already in `scale_entry_widget` (main.rs L13496-13501).
- **Fractions** — `FractionMode::Fraction { denominator, reduce }`, a
  power-of-two denominator ComboBox + a "reduce" checkbox, same widget
  (L13502-13526).
- **Position** — `offset` (standoff) + `text_along` (label position along
  the line), a single SolidWorks-style drag (`run_dimension_drag`,
  L12649-12835) that commits `place_dimension` on release, **already
  matching exactly the implicit-commit model §B recommends generalizing**
  — this is the second working precedent (alongside VectorEdit) for that
  design.
- **Drafting standard** (ANSI/ISO) — `DimStandard`, a two-way choice in the
  Group Manager (L13620-13655).

**What is genuinely missing:**

1. **Tolerance + tolerance type** — absent from `pdfce-core` entirely; no
   type, no field, no rendering. This is real new core work.
2. **A selection-driven property surface for an ALREADY-PLACED ce
   dimension.** `doc.selected_dimension` exists (main.rs L1758) but today
   drives exactly two things: the drag-to-reposition gesture and Delete-key
   removal (L2955-2965, L4766-4779). There is **no** panel today that shows
   a selected ce dimension's own properties — not its group, not its
   radius/diameter toggle, nothing. An operator who placed a Radius ce
   dimension and later wants Diameter has **no way to change it** without
   deleting and redrawing, because that toggle only exists in the
   tool-armed propbar at DRAW time (L13152-13166), never against a
   selection afterward. This is the concrete gap Ken's request is actually
   naming when he says "the ce dimensions I add need to be editable as
   well" — the group-level controls exist; the PER-DIMENSION,
   after-the-fact editing surface does not.
3. **Extension-line drag-to-extend/retract** — the extension-line gap and
   overshoot are currently `DimStandard`-derived constants (group.rs
   L64-81's doc comment: "whether the extension-line gap and overshoot are
   absolute or line-width-relative"), not per-dimension, not
   drag-adjustable. New core fields needed.

### C.11.1 Panel design: group-level vs. per-dimension, and the inheritance disclosure

**Group-level (apply to every member; unchanged from today):** unit,
decimal places/fraction, decimal marker, drafting standard, scale, layer
visibility. **Recommend these STAY group-only for v1** — do not add a
per-dimension override toggle for unit/format yet. Reasoning: SolidWorks
itself sets precision/units at the document level by default, with
per-dimension override a secondary, lower-frequency feature; rule 3
(progressive disclosure) argues for shipping the group-only surface
(already built, just needs relocating per §C.12) and deferring a full
per-field override mechanism until an operator actually asks for it after
using the group-only version. Flagged as P2 in the change list, not
refused outright.

**Per-dimension (this ce dimension only):**

- **Position** (offset/text_along) — already works via drag; recommend
  ALSO exposing two numeric `DragValue` fields in the property panel for
  keyboard-precise entry, mirroring the drag rather than replacing it —
  cheap, since the fields write the exact same `place_dimension` call the
  drag already commits to.
- **Radius/diameter display toggle** (MeasureCircular members only) —
  currently draw-time-only; move it (or duplicate it) into the
  selection-driven panel so it's reachable after placement. This is a real,
  named usability gap, not a nicety.
- **Tolerance + tolerance type** (new) — per-dimension by real-world
  convention, same reasoning SolidWorks itself uses: two features on the
  same drawing routinely carry different manufacturing tolerances even
  though they share a document's units and precision. Recommended shape,
  for the engineer to evaluate against `pdfce-core`'s own conventions
  (**not mine to design in Rust, but sketched so the ask is concrete**):

  ```text
  enum ToleranceType {
      None,
      Symmetric { value: f64 },              // "± value"
      Deviation { plus: f64, minus: f64 },    // "+plus / -minus"
      Limit { upper: f64, lower: f64 },       // stacked max/min
  }
  ```

  Naming it honestly: **pdfce should draw "SolidWorks-style" tolerance
  notation, never claim "SolidWorks-conformant"** — the same epistemic
  discipline `DimStandard`'s own doc comment already applies to ISO 129-1
  ("pdfce draws ISO-style, never ISO 129-1 conformant... paywalled and not
  obtained"). SolidWorks's tolerance dialog has more variants (Fit, Fit
  with Tolerance, Bilateral, etc. — GD&T territory); starting with these
  four common ones and naming the boundary honestly avoids an unverifiable
  parity claim, exactly the pattern this project already uses elsewhere.
  Lives on `DimensionRecord`, not `DimensionKind` — it is a display/
  documentation property layered on top of the measured geometry, exactly
  parallel to how `Group` layers a `NumberFormat` on top of
  `measured_points` (decision 011 §2.3's stored-geometry/derived-display
  split); the immutable `DimensionKind` enum should stay pure geometry.
  Regenerating the `/AP` for a tolerance change touches only the ONE
  member (unlike a group scale change, which regenerates every member) —
  cheaper, and worth stating so nobody assumes it needs the group's
  broadcast-update machinery.

  **A document-level DEFAULT tolerance** (new ce dimensions inherit it at
  creation time, overridable per dimension) is a reasonable P1 addition
  once the per-dimension mechanism exists — mirrors the group's own
  default-format-at-creation pattern (`Unit::default_format`), so it is a
  consistent extension, not a novel one.

**The inheritance-disclosure mechanism Ken explicitly asked for** ("cannot
change one and be surprised 40 others changed or didn't"): for any field
that is genuinely group-level with no override (v1's unit/format/standard),
the panel simply doesn't offer a per-dimension control for it at all — no
ambiguity possible, because there is nothing to accidentally scope wrong.
For tolerance (genuinely per-dimension from day one), no inheritance
question arises either, EXCEPT for the P1 document-default case — if that
ships, the panel must show, next to the tolerance controls, whether the
value shown is "the document default" (greyed, with a caption naming it as
such) or "set on this dimension" (editable, with a caption "this dimension
only"), following the same two-state convention (inherited-and-greyed vs.
overridden-and-editable) any future per-field group override would also
need — so the pattern only needs to be designed once, even though it isn't
needed until the P1 default-tolerance feature ships.

### C.12 Where the panel lives: `DockPanel::Properties`, contextually — and yes, its "nothing competed for the word" premise is now false

Not the Tool Options panel (§A) — that's for an ARMED TOOL's controls, and
per-dimension editing must work with **no tool armed**, exactly like the
existing position-drag already does (`run_dimension_drag`'s own doc
comment: "not gated behind a mode"). Not a new taxonomy bucket either —
this agent's own Pass 12.M2 memory already reasoned that a canvas-tool-
scoped, document-internal editing surface belongs in the SAME bucket as
Properties, not an eighth taxonomy instance; a SELECTION-scoped
per-dimension panel is the same kind of thing, one step further down
(properties of the selected OBJECT, not properties of the armed tool).

**Recommend folding it into `DockPanel::Properties`, as a contextual
section that appears above the existing `/Info` form whenever
`doc.selected_dimension.is_some()`.** Transient, "what I'm looking at
right now" content goes first, above the persistent document-metadata
form below it — the same top-first-bottom-persistent ordering A.3's
Objects/Properties split already establishes in spirit.

**Answering the question directly: yes, the "nothing else competed for the
word Properties" premise (`dock.rs` L173, "the body that used to live in
the floating `properties_window`") is now false, and both the doc comment
and the tab's tooltip need updating.** A ce-dimension selection is a real
second claimant on that word. The fix is to broaden the panel's stated
purpose — "the document's `/Info` metadata, OR the properties of whatever
is currently selected on the canvas" — not to invent a new panel or
rename the tab. Flagging explicitly per this agent's brief's instruction
to note when a placement premise stops holding.

### C.13 Extension-line handles vs. Bézier/node handles vs. the position drag — visual disambiguation

Three handle families will coexist in the same canvas once this ships, and
an operator must be able to tell which is which BEFORE grabbing one
(discoverability checklist, §1 of this agent's brief):

| Family | Shape | When drawn | Colour |
|---|---|---|---|
| Position drag (27.0/27.1) | none — grabs anywhere in the dimension's own rect | `doc.selected_dimension` set | `DIMENSION_DRAG_COLOR` outline on drag |
| Bézier/node handles (26.1) | filled circle (anchor) / hollow circle (control point) | VectorEdit tool active + that path entered/selected | path-editing selection colour |
| **NEW: extension-line length handles** | small perpendicular tick/hash mark at the outer (overshoot) end of each witness line | `doc.selected_dimension` set (same gate as the position drag) | `DIMENSION_DRAG_COLOR`, matched to the position-drag's own colour since they're the same object's furniture |

Recommend the **tick/hash shape**, not a circle or square, specifically so
it reads as "part of this dimension's own drafting geometry" rather than
"a path node" — a circle would visually collide with 26.1's anchor glyph
the moment an operator has both a ce dimension and a path selected in
quick succession, even though the two selections are mutually exclusive at
any GIVEN instant (see below).

**Why the two families never appear simultaneously for the same object,
already true today:** `run_dimension_drag`'s own doc comment states it
takes priority over object selection when the pointer is over a ce
dimension, "because a dimension sits ON TOP of the drawing it measures."
This existing priority rule is exactly what keeps the ce-dimension handle
vocabulary and the VectorEdit node-handle vocabulary from ever needing to
render at the same screen location for the same click — cite it, don't
re-derive a new disambiguation rule.

**New core field needed** (same class of ask as tolerance, not GUI-only):
`ext_a_overshoot: Option<f64>` / `ext_b_overshoot: Option<f64>` on
`DimensionKind::Linear` — `None` = the `DimStandard`-derived default
(today's only behavior), `Some(v)` = an operator override for THIS
dimension's witness line length. This mirrors `offset`/`text_along`'s own
migration-safety pattern exactly (group.rs L152-167's doc comment: "The
default is exactly zero... an existing ce dimension deserialised without
this key looks identical to how it looked before the field existed") — a
`None` default costs nothing on deserialize for every ce dimension
authored before this field exists. **Fuzzy-never-sneaky nuance for
whoever implements this:** once a witness line is overridden, THIS
dimension's line length no longer matches the group's `DimStandard`-
derived proportion the rest of the group uses — that divergence should be
disclosed with a small "custom witness length" caption, the same way a
future per-dimension format override would need one (§C.11.1).

---

## Prioritized change list

### P0 — directly answers the operator's literal request

1. **Wire `GestureInterrupt::Commit`** for TextEdit's plain edit, AddText's
   authored content, MeasureLinear's plain pick (§B.7) — already fully
   specified in the 2026-08-04 spec; this is "go implement it," not new
   design. Handle the mid-gesture-discard distinction (§B.7) and the empty-
   AddText-draft refusal (§B.10) as part of the same change.
2. **Build the new left-dock** (Pages | Tool Options), per §A.1/A.2.
3. **Relocate every propbar/status Area's content into the docked pane**
   per tool (§A.4), deleting the floating Areas outright (§A.5) — do this
   directly into the dock rather than building the 2026-08-04 spec's
   "merge into one floating Area" as an intermediate step; the dock now
   exists to receive the content directly.
4. **Auto-raise Tool Options on tool-arm** (§A.3), with the empty-state
   caption when no tool is armed.
5. **Handle the refusal-retains-draft path + the new tab-badge** (§B.8).
6. **Classify MeasureScale explicitly** (§B.7's new finding) — recommend
   keeping its explicit Accept; flag for engineer/operator confirmation
   since it's a judgment call layered on top of the settled rule-4 line.
7. **Build the selection-driven per-dimension section in
   `DockPanel::Properties`** (§C.12): radius/diameter toggle reachable
   after placement (currently draw-time-only — a real, named gap), numeric
   offset/text_along fields alongside the existing drag. Update the
   panel's doc comment/tooltip per §C.12's premise correction.
8. **Relocate the Group Manager's existing content** (unit/format/
   standard/scale/visibility — all already built) out of its floating
   `egui::Window` (main.rs L13552) into the dock, closing R81's own
   already-named "remaining floating-window holdout for a follow-up
   migration into the dock."

### P1 — real new capability, explicitly requested by name, scoped as its own work

9. **Tolerance + tolerance type** (§C.11.1) — new `pdfce-core` data model,
   GUI controls, `/AP` rendering, and a `pdfce-cli` subcommand (rule 11).
   Recommend `pdfce-librarian` files this as its own Backlog/Next-up
   Pass rather than folding it into this dock-relocation Pass — it's
   materially larger and independent of everything else in this document.
10. **Extension-line drag-to-extend/retract** (§C.13) — new core fields
    (`ext_a_overshoot`/`ext_b_overshoot`) + new drag handles + the
    divergence-from-standard disclosure. Same scoping note as item 9.
11. **Numeric offset/text_along fields** alongside the existing drag
    (§C.11.1) — cheap, since it reuses the existing `place_dimension` call.
12. **Extend AddText's commit trigger** to plain click-elsewhere (not just
    Enter), for mouse-only operators (§B.7).
13. **Keyboard-shortcut reference entries** for commit/discard (§B.9),
    reusing whatever surface the 2026-08-04 spec's own P1-2 builds.

### P2 — valuable, not required to answer today's request

14. **Per-dimension unit/format override toggle** (§C.11.1) — deferred
    deliberately per rule 3; revisit only if an operator asks for it after
    using the group-only version.
15. **Document-level default tolerance**, inherited at creation time
    (§C.11.1) — natural P1-of-item-9, sequenced after tolerance itself
    ships.
16. **Skip no-op TextEdit commits** (identical text, no diff) rather than
    emitting an empty undo-stack entry — a correctness nicety, not a
    safety issue.
17. **A persistent tool-mode cursor/banner cue** (the 2026-08-04 spec's
    D9) — still open, independent of this document.

### Items for the engineer, not mine to decide

- Whether the new left-`egui_tiles::Tree` genericizes `DockBehavior<'a>`
  over its pane type or duplicates a sibling `Behavior` impl (§A.1) — an
  implementation-shape call best made against `egui_tiles`' actual trait
  bounds, not from reading source alone.
- Whether items 9/10 (tolerance, extension-line drag) ship as part of the
  same session as the P0 dock relocation, or as separate, later-scheduled
  Passes — they are real, scoped, `pdfce-core`-touching work, not GUI
  polish, and deserve their own review pass regardless of how eager the
  operator's phrasing ("they should give me an option to...") might read
  as "all in one go."
- The exact fixed pixel/point sizing of the new left dock's default width
  and the Tool Options pane's minimum height — a spacing judgment best
  made by looking at the running app, per this agent's own standing
  precedent for not inventing exact numbers from source alone.
