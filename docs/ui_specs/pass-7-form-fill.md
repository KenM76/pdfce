# Pass 7 UI Spec — Interactive Form Filling (AcroForm)

> Authored by `pdfce-ui-specialist`, 2026-07-31/08-01, on dispatch from the
> engineer/orchestrator. This is the implementation spec for Pass 7's form-
> **filling** GUI surface; the engineer implements its P0 verbatim, deviating
> only with a recorded reason (the Pass 3.2/6.1 spec convention — deviations
> are named, not silent). If schedule pressure forces a cut, ship P0 alone
> and defer the rest as a named follow-up, exactly as Pass 6.1 anticipated
> for its own quad-point family.
>
> Read: `crates/pdfce-gui/src/{main.rs, ui_text.rs, viewer.rs}` in full;
> `.claude/agents/pdfce-ui-specialist.md`; `docs/ui_specs/pass-6.1-markup-tools.md`
> (the tool-mode state machine, `screen_to_page`/`page_to_screen`, the
> property-bar placement precedent, the drag-vs-pan conflict, the auto-cancel
> rule — this spec reuses the *canvas plumbing* and explicitly does **not**
> reuse the *tool-mode* shape, see §1); `docs/ARCHITECTURE.md` §12
> continuation-23(b) (the five-way placement taxonomy) and continuation-24/25
> (R43–R52, the appearance-pipeline and flatten/NeedAppearances rules this
> spec is built on top of); `docs/ROADMAP.md` Pass 7 entry (scope, the two
> dispatched prerequisites, the open embedded-JS decision); `crates/pdfce-core/
> src/{annot.rs, vartext.rs, signature.rs}` (existing widget/appearance/
> census machinery this Pass wires into, not rebuilds).
>
> **This spec does not cover:** the field *model itself* (§12.7 field
> dictionary, terminal/non-terminal fields, `/AcroForm` traversal) — that is
> `pdfce-core`'s job, blocked on the dispatched §12.7.1–12.7.4 spec sourcing,
> and this GUI spec is written against its *shape* (four field types, a
> field↔widget merge, field flags) rather than its exact Rust types, exactly
> as `pass-6.1`'s spec left `pdfce-core`'s markup-command API unnamed. It also
> does **not** cover form *creation* (drawing a new field onto a page) —
> Pass 7 fills fields that already exist in the file; authoring new fields is
> a distinct, larger capability (closer in shape to Pass 6.1's authoring
> tools than to this one) and should be scoped separately if wanted. It does
> **not** cover executing any action dictionary (`/A`, `/AA`) of any kind —
> JavaScript, `/ResetForm`, `/SubmitForm`, `/GoTo`, `/URI` — all alike; see
> §3.6 for why pushbuttons are recognized-and-disclosed rather than partially
> implemented.

---

## 0. Scope decided in this spec — read this first

| Bucket | Contents | Ships this Pass? |
|---|---|---|
| **P0** | Text-field fill (single-line + multiline, `/MaxLen`), checkbox toggle, pushbutton disclosure, the two honesty disclosures (JS-computed value, `/NeedAppearances`), field focus ring, cursor affordance, Tab/Shift-Tab navigation via egui's native focus order, one `EditSession` command per committed value, draw-time certification hard refusal (reusing the existing coarse gate), save-time signature reuse (unchanged), the document-is-a-form status line | **Yes, required** |
| **P1** | Radio-group exclusivity, list box (single/multi-select), combo box (incl. editable-combo free text), comb fields (fixed-pitch cell rendering in both the live overlay and the generated appearance), Flatten (+ its tooltip disclosure), Export/Import form data (FDF/XFDF) in the Tools dock, the "Regenerate appearances" action, proactive certification gray-out, multi-page Tab-overflow auto-navigation | Ship if the field-model prerequisite lands with enough runway; otherwise ship P0 alone and carry these forward as a named follow-up, exactly as pass-6.1's quad-point family was allowed to slip |
| **Not scoped to Pass 7 at all** | Executing any `/A`/`/AA` action (JavaScript, `/ResetForm`, `/SubmitForm`, `/GoTo`, `/URI`, `/Named`); authoring new fields; XFA forms (separate, unresolved-deprecation-status backlog item per `CLAUDE.md`'s outstanding-items list) | No — not a Pass 7 follow-up bucket, a genuinely separate future decision |

Given census data already measured for decision 008 (`/AcroForm` in 30.1% of
the organic sample; `/Tx` the overwhelming majority field type), **P0's
text-field + checkbox coverage alone already serves the large majority of
real forms.** This is the concrete justification for cutting radio/list/
combo to P1 rather than treating all six field-interaction kinds as one
inseparable unit: unlike Pass 6.1's shape tools (all roughly equal
implementation cost, no one shape dominating real usage), form field types
are **not** equally load-bearing, and the cut follows the measured demand
curve rather than an arbitrary implementation-convenience split.

---

## 1. Why this is *not* a tool-mode state machine — the mode-model decision

### 1.1 The question, answered up front

**Decision: no separate "fill mode." Widgets on an open document are
directly clickable in ordinary view mode, exactly as they display today.**
There is no `active_tool: Option<FormTool>`, no toolbar mode-toggle, no
crosshair cursor for the canvas as a whole, and no "enter fill mode" chord.

### 1.2 Reasoning — why this is the opposite call from Pass 6.1's markup tools

Pass 6.1 needed a tool-mode state machine because drawing is **ambiguous
without one**: a click-drag on a blank canvas could mean "pan the view,"
"select something," or "draw a shape," and nothing about the gesture itself
disambiguates — hence a mode, a crosshair cursor as the "what happens if I
click now" signal, and an explicit enter/exit lifecycle (`pass-6.1` §1.4,
§2.3).

Form-field interaction has **no such ambiguity**. A widget's `/Rect` is a
small, bounded region that the file itself marks as interactive — the
widget's own appearance stream *already looks like* an input control (a
bordered box, a checkbox glyph, a dropdown arrow), because that is what the
document's own author or producing application drew. Clicking inside that
specific rectangle can only sensibly mean "interact with this field"; there
is nothing else at that exact location competing for the gesture. This is
squarely rule 3's "a form is self-evidently fillable" framing from the task,
and it is also the stronger, load-bearing reason: **gating filling behind a
mode would hide the primary reason most form documents exist** at all,
which is the opposite of progressive disclosure's actual goal (rule 3 exists
to hide *advanced, occasional* capability, not a document's central,
common-case purpose). A markup pen is an advanced authoring capability on
top of any document; filling in a form *is* what a form document is *for*.

### 1.3 What this decision does NOT eliminate

Removing the tool-mode does not remove every piece of Pass 6.1's canvas
machinery — it removes exactly the parts that existed to resolve ambiguity
that forms do not have:

| Pass 6.1 concept | Needed here? | Why / why not |
|---|---|---|
| `screen_to_page`/`page_to_screen` (§2.1) | **Yes, reused verbatim** | Every widget rect must still be projected screen↔page each frame; this is pure geometry, orthogonal to the mode question |
| Canvas made focusable (§2.2) | **Yes, reused** — this Pass is a **second**, independent reason the same caveat needed resolving | Text-field editing needs real keyboard focus on the canvas; Pass 6.1 already resolved the underlying plumbing generically enough (per its own §2.2 note) for this Pass to layer onto the same focusable widget rather than redoing the step |
| Crosshair cursor / "what happens if I click" signal | **No — narrower, per-widget signal instead** | See §2.3: the cursor changes only when *hovering a specific widget*, never for the canvas as a whole, because there is no ambiguous canvas-wide mode to signal |
| Drag-to-pan suppression | **Yes, but scoped to individual widget rects, not global** | See §2.4 — the conflict is real but local, unlike Pass 6.1's whole-canvas suppression |
| Auto-cancel-on-navigation rule (§1.5) | **Partially — reasoning differs, see §2.6** | A half-drawn shape has no operator-legible value to preserve; a half-typed field value does, so this Pass's equivalent rule is "commit, don't discard" |
| The two-stage Escape | **No** | There is no in-progress *shape* to cancel independently of exiting a mode there is no mode to exit |

---

## 2. Canvas interaction

### 2.1 Geometry — reuse, do not rebuild

`viewer::screen_to_page`/`page_to_screen` (added in Pass 6.1, §2.1 of that
spec) are reused unchanged. Every widget's `/Rect` is projected to a screen
rectangle each frame via `page_to_screen` applied to its two corners; that
screen rectangle is what every interaction in this spec hit-tests against
and draws its focus ring/overlay relative to.

### 2.2 The architectural recommendation: real egui widgets over the raster, not hand-rolled `painter()` hit-testing

Pass 6.1's shapes are drawn with raw `ui.painter()` calls because there is
no natural egui widget that "is" an in-progress polygon. Forms are
different: **egui already has widgets that are a near-exact behavioral
match for four of the six field kinds**, and using them is not merely
convenient — it is the single biggest, cheapest accessibility win available
in this Pass, because it gets real Tab-focus participation, real
`accesskit` exposure, and real keyboard operability **for free**, in
contrast to Pass 6.1's hand-rolled shapes (which the accessibility section
of that spec named as a real, tracked, pointer-only gap). Recommend:

| Field kind | Interaction primitive | Why |
|---|---|---|
| Text (single-line, multiline, comb) | A **real** `egui::TextEdit` overlaid exactly on the widget's screen rect, shown only while that field has focus | Live text editing genuinely needs a real text-input widget; there is no way to fake this with painter calls without reinventing cursor/selection/IME handling badly |
| Checkbox | `ui.interact(rect, id, Sense::click())` — a transparent hit/focus target with **no egui-drawn visual** | The correct visual (checked/unchecked glyph) already exists in the widget's own `/AP` and is what R43's existing render path already paints; drawing a *second*, egui-styled checkbox glyph on top would be a visible double-render that does not match the document's actual appearance. Focus/hover/Tab-participation is what we want from `ui.interact`; the checkbox *look* stays exactly what pdfce would render for that document regardless |
| Radio (each widget in the group) | Same as checkbox — `ui.interact`, transparent | Same reasoning; exclusivity is a **value-level** rule (§3.3), not a rendering one |
| Pushbutton | Same as checkbox — `ui.interact`, transparent, for the click-to-disclose behavior (§3.6) | No value to hold; only needs a click target |
| List box | A **real** overlay list widget (`egui::ScrollArea` + selectable rows, or `egui::Checkbox`-per-row for `MultiSelect`) shown only while focused | Hit-testing the *raster's* row layout is unreliable (pdfce cannot always know the exact row geometry an arbitrary producer's appearance stream used); an interactive overlay sidesteps that entirely — same principle as the text overlay |
| Combo box | A **real** `egui::ComboBox` (or, for the editable-combo flag, a `TextEdit` + dropdown button pair) overlaid while focused | Same reasoning as list box |

For checkbox/radio/pushbutton, the **visual never changes until commit** —
the operator sees the document's own real appearance the whole time, and
only the invisible interactive layer sits on top of it. For text/list/
combo, the overlay **replaces** the visual while focused (matching how a
spreadsheet cell shows its raw formula/value while being edited and its
rendered result otherwise) and reverts to the (re-rasterized) real
appearance the moment focus leaves and the commit lands.

### 2.3 Cursor affordance — the per-widget signal that replaces Pass 6.1's crosshair

Because there is no canvas-wide mode, the "what happens if I click here"
signal is scoped to individual widgets, via `ui.output_mut(|o| o.cursor_icon
= …)` on hover:

- Hovering a text field → `CursorIcon::Text` (I-beam).
- Hovering a checkbox, radio button, pushbutton, list box or combo box →
  `CursorIcon::PointingHand`.
- Hovering anything else on the page (ordinary content, or a widget that is
  read-only/hidden per its own annotation flags) → the default arrow,
  unchanged from today.

This is a near-universal input-device convention for interactive form
controls (browsers, native OS dialogs, every desktop toolkit), not an
Acrobat-GUI borrowing, and it satisfies the discoverability checklist's
"visible current state" bar without needing a tooltip to be read first.

### 2.4 The drag-vs-pan conflict — scoped, not global

Pass 6.1 had to suppress the canvas's whole drag-to-pan behavior while any
tool was active (its §2.3). This Pass has a **narrower** version of the
same conflict: a click-drag that **starts inside a focused text field's own
rect** (selecting a run of text within the field) must not also pan the
view. Resolve it exactly the way pass-6.1 flagged its own equivalent
uncertainty — as a "verify against the pinned egui version" item, not an
assumption:

- Hit-test the drag's **start** position against the currently-open text
  overlay's rect (if any) before the `ScrollArea` claims the gesture. If it
  falls inside, the whole drag belongs to the `TextEdit` (its own internal
  text-selection dragging); otherwise, canvas pan is completely unaffected.
- ⚠️ **Verify in the pinned egui/eframe version which widget's `interact`
  wins when a `TextEdit` sits inside a `ScrollArea`'s response area** — the
  same "which overlapping `interact` wins" discipline `pass-3.2` §3.1 and
  `pass-6.1` §6.2 both already flagged for their own overlapping-gesture
  questions. This is very likely already correct by construction (egui
  widgets consume their own drag events before a parent `ScrollArea` sees
  them), but "very likely" is not "verified."
- Every other canvas gesture (plain wheel-scroll, ctrl+scroll-zoom, page-
  turning, panning anywhere **outside** a widget rect) is **completely
  unaffected** — unlike Pass 6.1, there is no whole-canvas suppression here.

### 2.5 Focus ring — how "the currently-focused field" is shown

A **shape**, not a color wash (rule 6): draw a 2px rectangle outline around
the screen rect of whichever field currently holds focus, in a color
visually distinct from the current markup pen color (so it can never be
mistaken for an authored annotation preview) and distinct from the
selection-rail's own checkbox glyph. The outline's **presence** is the
signal — no other field ever gets one — so this remains legible to an
operator who cannot distinguish the specific hue, exactly the same
reasoning `pass-6.1` §7 used to clear its own in-progress-preview shape
against rule 6.

No focus ring is drawn when nothing has focus (e.g., the operator just
opened the document and has not yet clicked a field) — an absent ring is
not ambiguous here the way an absent-vs-present annotation toggle state
would be, because "no field selected yet" is the obvious, expected initial
state of any form, needing no disclosure of its own.

### 2.6 Why the Pass 6.1 auto-cancel rule does NOT carry over unchanged

Pass 6.1 §1.5's rule is: any action that isn't part of continuing/cancelling/
committing an in-progress shape **silently discards it**, because a half-
drawn shape has no operator-legible value distinct from finishing the drag —
there is nothing to lose.

**A half-typed field value is different: the operator typed something with
clear intent to keep it.** Silently discarding a draft text-field value on
page navigation (or Undo, or Save) would be a real, surprising data loss —
the opposite of what rule 7 ("frictionless for reversible, low-stakes;
no *unnecessary* friction," which cuts both ways: an *unnecessary loss* is
also friction) argues for. So Pass 7's equivalent rule is:

**Any action that would move focus away from a field with an uncommitted
draft (Tab, Shift-Tab, a click on another field, a page-navigation action,
Undo/Redo, Save, opening Properties or the Tools dock) first COMMITS the
current draft as one `EditSession` command, then proceeds** — never a
silent discard. This is the same "commit-and-advance" behavior a
spreadsheet cell edit uses, and it is *why* text-field commit is defined at
the field-navigation boundary (§3.2) rather than only on an explicit
Enter/click-elsewhere.

The one genuine cancel path is **Escape**, which reverts the field to its
value as of the start of *this* editing gesture (not to some earlier
Undo-reachable state — Undo remains the mechanism for reversing an already-
committed change) and defocuses without committing. This is a **single-
stage** Escape, unlike Pass 6.1's two-stage rule — there is no "cancel the
gesture but stay in the tool" concept to preserve, because there is no
tool.

---

## 3. Field interaction per type

### 3.1 Text field (`/FT /Tx`)

- **Enter/focus:** click inside the widget rect (or Tab into it, §5) opens
  a real `egui::TextEdit` overlaid exactly on the widget's screen rect,
  pre-filled with the field's current `/V` (decoded via the existing
  PDFDocEncoding/UTF-16 machinery `text_extract`/`textstring.rs` already
  built for Pass 4 — reused, not reimplemented).
- **Multiline** (`/Ff` bit 13): a multi-line `TextEdit`; **Enter inserts a
  newline** and does **not** commit. Commit happens only via Tab/click-
  elsewhere/Save/etc. (§2.6) or an explicit "Done" affordance — the
  property-bar-style hint text under the field (or in the status bar while
  editing) states this plainly: *"This field accepts multiple lines — press
  Tab or click elsewhere to save your entry, Esc to cancel it."*
- **Single-line:** a single-line `TextEdit`; **Enter commits and defocuses**
  (matches task item 6's "commit (Enter)"; ⚠️ verify the pinned egui
  version's exact `TextEdit::singleline` Enter-vs-`lost_focus` API shape
  before wiring this — same "flag, don't assume" discipline as §2.4).
- **`/MaxLen`:** enforced live in the overlay (egui's character-limit
  facility, or a manual truncating `change_filter`) — the operator cannot
  type past the limit rather than discovering a silent truncation at
  commit.
- **Comb** (`/Ff` bit 25, the field this Pass adds over Pass 6.2's deferral):
  the live overlay draws `/MaxLen` evenly-spaced vertical cell dividers via
  `ui.painter()` **on top of** the `TextEdit` (a cheap vector overlay,
  exactly `pass-6.1` §2.8's "don't re-raster for a live preview" principle),
  so the operator sees the comb-cell layout while typing even though the
  backing widget is an ordinary `TextEdit`. The generated appearance (on
  commit) is what must be pixel-accurate per-cell — that is `vartext.rs`'s
  job (`build_variable_text` needs a comb-aware code path this Pass adds;
  not dictated here, per the pass-6.1 precedent of leaving `pdfce-core`
  API shape to the engineer) — the live overlay's divider lines are a
  **visual approximation for editing feedback**, not a promise of exact
  character-per-cell alignment while typing.
- **Commit:** one `EditSession` command per commit (Tab, click-elsewhere,
  Enter on a single-line field, or Save/Undo/navigation per §2.6) —
  never per keystroke, exactly Pass 3.1's properties-draft precedent
  extended to fields. On success, `doc.refresh_pages()` (the existing
  function, unchanged) makes the regenerated appearance visible.
- **Cancel:** Escape reverts to the gesture-start value and defocuses,
  without producing a command.

### 3.2 Checkbox (`/FT /Btn`, no Radio/Pushbutton flags)

- **Interaction:** `ui.interact(rect, id, Sense::click())`, transparent
  (§2.2). Click **or** Space-while-focused toggles the field's `/AS`
  between its on-state name and `/Off` — task item 6's "toggle (Space for
  checkbox)."
- **Commit:** immediate, one `EditSession` command per click/Space-press —
  there is no draft state to hold, so this is atomic like Pass 6.1's markup
  shape commit, not staged like the text field.
- **No confirmation** — matches rule 7 exactly: a checkbox toggle is
  reversible pre-save (Undo) and low-stakes; gating it behind any
  confirmation would be the friction rule 7 explicitly forbids.
- **The honest "no visual for this state" case:** because R43 (render from
  `/AP` or not at all) still applies, a checkbox whose `/AP`/`/N` lacks an
  appearance sub-stream for the state it is being toggled *into* will
  toggle the stored value correctly but paint nothing different on screen.
  This must be disclosed, not left as an apparent no-op: a one-shot
  `edit_note` line, *"The value changed, but this control has no stored
  appearance for its new state, so nothing will look different on screen
  until it is viewed elsewhere or a full appearance is generated."* — the
  same "missing, not approximated" honesty framing `diagnostics_fonts_
  unsupported` already uses for the render-diagnostics header.

### 3.3 Radio group (`/FT /Btn`, Radio flag set) — **P1**

- Each widget in the group is a separate annotation but shares one parent
  field (or the group is modeled as siblings under a common `/Parent`,
  per whatever `pdfce-core`'s field model settles on — GUI-agnostic to
  which). Clicking **any** widget in the group:
  1. Sets that widget's own `/AS` to its on-state name.
  2. Sets **every sibling widget's** `/AS` to `/Off`.
  3. Sets the **field's** `/V` to the clicked widget's on-state name.
  4. All of the above is **one** `EditSession` command covering the whole
     group flip — Undo reverses the entire group change in a single step,
     not N widget-level steps (the same "one coarser command, not a
     per-element stack" precedent `pass-6.1` §2.7 already established for
     Polygon vertex commits).
- **Re-clicking the already-selected radio:** whether this can turn the
  whole group off (`NoToggleToOff` field-flag semantics) is a **field-model
  rule**, not a GUI one — the GUI always sends a click-on-this-widget
  request; `pdfce-core` decides whether it is honored or is a no-op. This
  keeps the GUI dumb and the rule enforced in exactly one place, matching
  the GUI-core-separation invariant.
- Interaction primitive, cursor, and honest no-appearance disclosure: all
  identical to checkbox (§3.2).

### 3.4 List box (`/FT /Ch`, Combo flag clear) — **P1**

- **Interaction:** clicking anywhere inside the widget rect (or Tab-focus)
  replaces the visual with a real overlay list (§2.2) built from the
  field's `/Opt` array (export/display value pairs) and current `/V`
  (or `/I` for indices, per the field model). Single-select: click a row
  to select and commit immediately (one command), overlay closes. Multi-
  select (`/Ff` bit 22, `MultiSelect`): each row is independently
  toggleable (a checkbox-per-row visual), with an explicit "Done" action
  (a small button, or click-elsewhere) that commits the whole set as one
  command — not one command per row toggled, mirroring the radio-group
  reasoning in §3.3.
- **Commit → regenerate:** on commit, the field's stored appearance (which
  visually shows the selection, e.g., a highlighted row) is regenerated by
  the same appearance pipeline as everything else in this Pass, and
  `refresh_pages()` shows it once the overlay closes.

### 3.5 Combo box (`/FT /Ch`, Combo flag set) — **P1**

- **Interaction:** a real `egui::ComboBox` overlaid on the widget rect while
  focused, populated from `/Opt`. For the **editable** variant (`/Ff` bit
  19, `Edit`), pair it with a `TextEdit` so the operator can type a value
  not in the list, exactly matching what that flag means in the field
  model.
- **Commit:** one command on selection/text-commit, same reasoning as
  every other type.

### 3.6 Pushbutton (`/FT /Btn`, Pushbutton flag set) — recognize, disclose, do nothing

Per the task's framing, a pushbutton **has no value** — it is a trigger for
an action dictionary (`/A` on the widget, or `/AA` entries), and **executing
any action of any kind is out of scope for this entire Pass**, not merely
the JavaScript subset of it (`/ResetForm`, `/SubmitForm`, `/GoTo`, `/URI`
are equally unimplemented, even though none of them require running a
script). This is a **fuzzy-never-sneaky** situation distinct from the
markup tools' honest disclosures: a control that visually invites a click
(often styled to look exactly like a real button, via `/MK`) and then does
*nothing at all* is silently confusing unless pdfce says so.

- **Cursor:** `PointingHand` on hover (§2.3), same as every other
  interactive widget — the operator should not be misled into thinking
  the button is inert *before* they try it.
- **Tooltip (always present on hover, proactive, not just reactive):**
  *"This is a button. pdfce does not run button actions yet (including
  form submission, reset, or navigation), so clicking it does nothing to
  the document."*
- **On click:** a one-shot `edit_note`-channel status line — non-blocking,
  no modal, because there is genuinely no consequence to gate (nothing
  happened, so there is nothing for the operator to confirm or undo):
  *"Nothing happened — pdfce does not run this button's action yet."*
  Reusing the existing narrator channel means no new UI surface is
  introduced for a control that, by design, changes nothing.
- **Future-worth-naming, not scoped here:** `/GoTo`-only pushbutton actions
  (in-document navigation) are a meaningfully lower-risk subset than
  script execution or form submission — worth flagging to the librarian as
  a *possible* narrowly-scoped future Backlog idea, distinct from (and much
  smaller than) full action-dictionary support. Not asserted as planned;
  only as worth recording as an idea.

---

## 4. The two mandatory disclosures — fuzzy, never sneaky

### 4.1 `/NeedAppearances` — reuse the existing Pass 6.0 line, add the missing action

`ui_text::annotations_need_appearances()` **already ships** (Pass 6.0) and
already states the R51-compliant fact: pdfce shows the file's stored
appearances as-is and does not silently rewrite them. What Pass 6.0 did
**not** need, and Pass 7 does, is an actual operator-triggered remedy —
R51's other half ("generates appearances only as a reviewable, operator-
visible action") has had nothing to attach to until there is a real
appearance-generation pipeline for *field* values (Pass 6.2 built it for
markup/text annotations; Pass 7 extends it to widgets).

**Add one button directly beside the existing disclosure line** (same
status-bar row, not a new panel — the fact and its remedy stay adjacent,
the same locality principle the toolbar's annotation-visibility toggle
already demonstrates for its own disclosure):

- `regenerate_appearances_button()` — *"Regenerate appearances"*.
- On click: **one** `EditSession` command that regenerates every widget's
  appearance stream from its current value (batched, not per-field — Undo
  reverses the whole regeneration in one step, matching the radio-group and
  list-box multi-change precedent).
- Narrator line on success: *"Regenerated {N} field appearance(s). Use Undo
  to reverse this until you save."* — the same "Use Undo … until you save"
  clause verbatim, per `pass-6.1`'s own explicit instruction to reuse that
  exact phrase rather than rewording it.
- **No confirmation dialog** — this is a reversible, whole-document
  regeneration with no destructive consequence pre-save, so rule 7 applies
  the same way it does to the checkbox toggle.

### 4.2 The JS-computed-value disclosure — a genuinely new disclosure this Pass introduces

A field whose value is normally **computed** by an embedded calculation
script (an `/AA`/`/C` action on the field, or the field's presence in the
AcroForm's `/CO` calculation-order array) is, per the pending decision-008
continuation (never-execute), never recalculated by pdfce. The operator
sees the value **last stored in the file** — which is fine and honest as
long as it is disclosed; presented silently, it would read as "this field
updates live" when it does not, which is exactly what rule 1 forbids.

- **Per-field, on hover/focus (proactive, before the operator relies on
  it):** a tooltip on any such field, *"This value is normally calculated
  automatically by a script in this document. pdfce does not run scripts,
  so this shows the value stored in the file and will not update as you
  fill in other fields."*
- **Document-level status-bar count (disclosure → status bar, reusing the
  existing collapsing-diagnostics-detail convention):** a new line in the
  same family as `annotations_no_appearance`/`diagnostics_fonts_
  unsupported` — *"{N} field(s) in this document have their value computed
  by a script pdfce does not run; their stored values are shown as-is."*
  This is a **document-scoped** fact (a calculation script can reference
  fields anywhere in the form, not just the current page), so it belongs
  with `annotations_need_appearances` in the document-level disclosure
  area, not folded into the per-page `annotations_painted_summary` line.

### 4.3 "This document is a form" — the onboarding-level disclosure

A new document-scoped status line, shown whenever `/AcroForm` is present
with at least one terminal field, in the same disclosure area as
`annotations_need_appearances`:

- `form_document_summary(total_fields: usize, fillable_fields: usize) ->
  String` — *"This document is a form: {fillable_fields} of {total_fields}
  field(s) can be filled in."* (The difference between the two counts —
  read-only fields, per `/Ff` bit 1, and hidden widgets per `/F` — is
  itself worth a plain-English word rather than silently absorbed into one
  number; a form with 20 fields where only 12 are editable is a materially
  different situation for the operator to understand.)
- This is the answer to the task's "how is 'this document is a form / has N
  fillable fields' surfaced" question: **status-bar disclosure**, not a
  toolbar affordance — a toolbar control would need to *do* something
  (per rule 3's own bucket boundaries, a toolbar entry is view-state or
  edit, not a passive fact), and this is purely informational. No new
  panel, no new dialog style.

---

## 5. Flatten, and export/import form data

### 5.1 Flatten — **P1**, a delete-shaped disclosure, NOT a redaction-shaped confirmation

R48 records flatten as "destructive... like redaction it must disclose that
an incremental save leaves the pre-flatten annotation recoverable in the
prior revision" — but this is explicitly a **sibling of R35** (the delete-
pages disclosure), not of redaction's genuine post-save irreversibility.
The task's own instruction #4 asks precisely this question — "reuse the
SignatureImpact-style confirmation **only where a real irreversible-after-
save boundary exists**" — and the honest answer for flatten is: **that
boundary does not exist unconditionally.** Flatten is:

- Reversible pre-save via ordinary Undo, exactly like every other edit in
  this crate.
- **Still forensically recoverable after an incremental save**, because the
  flattened-away field/widget objects remain in the file's earlier
  revision even though the current page no longer references them —
  precisely the same situation `selection_delete_tooltip` already
  discloses for page deletion ("this does not erase the removed page's
  data from the file — the previous version can still contain it").
- Only genuinely, irrecoverably gone if a **full-rewrite save** is chosen —
  and pdfce has no full-rewrite save mode yet (`main.rs`'s own module docs:
  "A full-rewrite option belongs with the optimization feature that gives
  it a reason to exist"). So R48's "offer/force a full rewrite" half is
  **not buildable yet** — name this explicitly as a residual rather than
  silently dropping it (§7's open items).

**Decision: Flatten gets the exact `selection_delete_tooltip` treatment —
a rich, honest tooltip on the toolbar button, no blocking modal** — this
is the correctly-weighted answer, not merely the cheaper one, because the
"genuinely irreversible once saved" bar (rule 2) is conditioned on a save
mode this Pass does not offer.

- `flatten_button()` — a new toolbar button in the existing edit group
  (not a new "Form ▾" menu — see §5.3 for why a whole menu is not
  warranted yet).
- `flatten_tooltip()` — *"Bake the current field values into the page and
  remove the interactive form fields. This changes the document and is
  saved with it — use Undo to reverse it until you save. Note: like
  deleting a page, this does not erase the fields' data from the file — an
  ordinary save can still leave them recoverable in an earlier revision. If
  you need the fields truly gone, pdfce does not yet offer a way to force
  that (a planned full-rewrite save option would)."*
- **Commit:** one `EditSession` command (batched — every field flattened in
  one step, Undo reverses the whole flatten). Same certification hard-
  refusal path as markup authoring (§6).

### 5.2 Export / import form data (FDF/XFDF) — **P1**, Tools dock

Both operations' argument is a **file outside the currently open
document** (export writes one; import reads one) — this is exactly the
Tools dock's own defining sentence, `tools_dock_intro()`: *"These tools work
with files outside the one you have open."* No new placement rule is
needed; this is the existing rail-vs-dock argument-source test (`pass-3.2`
§1, carried into the five-way taxonomy's "advanced → Tools dock" bucket)
applied without modification.

- `tool_export_form_data_label()` — *"Export form data…"*. Writes an FDF or
  XFDF file (format choice: engineer's/`pdfce-spec-librarian`'s call) of
  the document's current field values. **Not an edit** — reads the open
  document, writes an unrelated file, creates no `EditSession` command,
  exactly like Extract/Combine already work.
- `tool_import_form_data_label()` — *"Import form data…"*. Reads an
  operator-picked FDF/XFDF and applies its values to matching fields **as
  one batched `EditSession` command** (this direction genuinely is an edit
  — it changes the open document's field values) — Undo reverses the
  whole import in one step. A field named in the imported file that does
  not exist in the open document is a **named, counted** mismatch in the
  success narrator line (*"Imported {n} field value(s); {m} name(s) in the
  file did not match any field in this document and were skipped."*),
  never a silent partial success.
- Both reuse the existing native-dialog (`rfd::FileDialog`) + atomic-write
  patterns already established for Save/Extract/Combine — no new dialog
  style, per the task's instruction #4.

### 5.3 Why no "Form ▾" menu yet

Unlike Markup/Text, which needed a menu because the operator first *picks
a tool* before drawing, form filling has **no tool-selection step at all**
(§1) — every interaction in §3 happens directly on the canvas with no
menu involved. The only toolbar-level, whole-document action this Pass
introduces is Flatten, so a single button is the right amount of new UI —
inventing a one-item dropdown menu "for future growth" would be
premature structure with nothing yet to justify it. **If a later Pass adds
a second whole-document form action** (e.g., "clear all field values"),
promote Flatten into a "Form ▾" menu at that point, exactly mirroring how
Markup/Text menus themselves came into being — flag this evolution path
for the librarian now rather than silently deciding it later.

---

## 6. Signature / certification interaction

### 6.1 Two distinct moments — reused verbatim from Pass 6.1, no new dialog

Identical table to `pass-6.1` §5.1: a **hard refusal** at commit time
(the existing `EditError` → `save_result = Some(SaveOutcome::Failed(...))`
channel) for a certification that structurally forbids the change, and the
**unchanged** `signature_confirmation` soft warning at save time for
"saving would invalidate an existing signature." Nothing new is invented
for either moment.

### 6.2 The load-bearing finding: form-fill's refusal gate must NOT be the same coarse check Pass 6.1 shipped

**This is the most important cross-cutting finding in this spec, and it is
a `pdfce-core` change, not a pure-GUI one — flagging it here because the
GUI's correctness depends on it and because it directly answers the task's
own item 5.**

`SignatureCensus::forbids_structural_change()` today (unchanged since
Pass 3.2) is:

```rust
pub const fn forbids_structural_change(&self) -> bool {
    self.perms_enforced && self.signatures > 0
}
```

This is **coarse by design** — it does not consult `certification_
permission` (`/P`, 1/2/3) at all, and `pass-6.1` §5.4 already found and
named the resulting over-refusal for annotation-adding at `/P 3`. **Form-
fill makes the same coarse check wrong in a much more consequential way,**
because of where `/P`'s default sits: **`/P` is optional with default
`2`, and Table 254's VALIDATION MODEL defines `/P 2` as *"filling in
forms... and signing"* — i.e., `/P 2` is not merely one tier that happens
to permit form-fill, it is the tier whose PRIMARY documented purpose IS
form-fill.** A document certified with the ordinary, default permission
level — "let people fill this in and sign it," almost certainly the single
most common reason anyone certifies a fillable PDF at all — is hard-
refused for the one operation its certification was created to allow, under
today's check.

Where Pass 6.1's residual (`/P 3` blocking annotation-adding) was a real
but narrow edge case worth shipping-conservative-and-naming, **this one is
not narrow: it would make the certified-form-fill case, arguably the
headline use case for this entire Pass, refuse itself on a large share of
real certified documents.** The over-refusal direction is still the safe
one (declining something DocMDP would have permitted, never the reverse),
so this is not a security defect — but shipping it unfixed would make Pass
7's flagship scenario visibly, immediately broken for the exact documents
operators are most likely to test it against.

**Recommendation: this Pass's `pdfce-core` work must extend the gate to
consult `certification_permission` for form-fill specifically** — at
minimum: `/P 1` still refuses (no changes permitted at all); `/P 2` and
`/P 3` both **permit** form-fill (per Table 254, `/P 3`'s permission list is
cumulative with `/P 2`'s). This is **not** the same scope as the annotation-
adding refinement `pass-6.1` §5.4 deferred to a follow-up — that refinement
was optional polish on an already-safe, if imprecise, P1 nicety. Here, doing
nothing produces a **P0-blocking** correctness gap for the Pass's core
scenario. State this to the engineer as a **required `pdfce-core` task for
this Pass**, not a P1/P2 GUI refinement.

### 6.3 GUI consequences, once the gate is fixed

- A `/P 1`-certified document: form-fill widgets are grayed with a named
  tooltip (`form_field_certification_disabled_tooltip()` — *"This
  document's certification does not allow any changes, including filling
  in this form."*), mirroring `pass-6.1` §6.3's existing disabled-not-
  vanished convention for its own zero-pages/certification gates.
- A `/P 2`- or `/P 3`-certified document: form-fill works exactly as if
  uncertified — no warning, no tooltip caveat, because the operation is
  genuinely, unconditionally permitted. **Do not add a defensive
  disclosure here "just in case"** — a caveat on a permitted, ordinary
  action is the over-warning the task explicitly asks this Pass to avoid,
  and it would train operators to distrust a message that is describing
  the normal case.
- Flatten (§5.1) is a **structural** change in the DocMDP sense (it removes
  interactive form fields from the page), so it stays subject to the
  existing coarse `forbids_structural_change()` check unchanged — the
  §6.2 fix is scoped to *value-setting* (fill), not to flatten, and this
  distinction should be named explicitly in the `pdfce-core` change so a
  future reader does not conflate the two operations' permission
  boundaries.

---

## 7. Keyboard map

| Chord | Action | Note |
|---|---|---|
| `Tab` | Move focus to the next field, in field-model tab order | **Not** a new custom `Action` — this falls out of egui's own native focus-cycling for free, provided fields are registered as real `ui.interact`/widget responses in the correct per-frame order (§2.2, §8). A half-typed field commits first (§2.6) |
| `Shift+Tab` | Move focus to the previous field | Same mechanism, reversed |
| `Space` | Toggle a focused checkbox or radio button | Checked via `response.has_focus() && key_pressed(Space)`, since the transparent `ui.interact` target has no built-in Space handling of its own |
| `Enter` | Commit a focused **single-line** text field and defocus | **Does not** commit a multiline field (inserts a newline instead, §3.1) — a genuinely different meaning depending on field type, name this explicitly in `collect_keyboard_actions`'s doc comment the same way `pass-6.1` named its own context-sensitive Backspace/Esc bindings |
| `Esc` | Cancel the current field's uncommitted draft, revert, defocus | Single-stage (§2.6) — **not** the two-stage rule Pass 6.1 needed, and not the same meaning as the rail's `ClearSelection` binding; falls through to the existing `ClearSelection` meaning only when no field is currently focused |

**No chord is needed to "enter fill mode"** — task item 6 asks for one
"if modal," and this design is deliberately not modal (§1), so there is
nothing to enter. This is worth stating explicitly rather than silently
omitting the chord, so a future reader does not assume it was forgotten.

All five bindings above are either **already-native egui behavior** (Tab/
Shift-Tab) or **new, unclaimed** against the full existing chord list
carried over from `pass-6.1` §6.2 (`Ctrl+O`, `Ctrl+Plus/Equals/Minus/0`,
`Ctrl+S`, `Ctrl+Shift+Z`, `Ctrl+Y`, `Ctrl+Z`, `[`/`]`, `Alt+`{ten letters},
`Alt+↑/↓`, `Esc`, `Delete`, `Backspace`, `Ctrl+Shift+C`, `Enter` — unbound
before this Pass). Space is unclaimed. Enter is unclaimed (Pass 6.1
reserved it for Polygon/PolyLine commit, but only "while mid-construction,"
which cannot overlap with a focused text field — the two contexts are
mutually exclusive by construction, since a markup tool being active is
itself one of the auto-cancel triggers, `pass-6.1` §1.5, that would already
have discarded any in-progress polygon before a field could gain focus).

---

## 8. Accessibility

- **Real Tab-focus and `accesskit` exposure for four of six field types
  (checkbox, radio, list, combo) and the active text overlay, for free** —
  the single biggest accessibility improvement this Pass makes over Pass
  6.1's necessarily pointer-only shapes (§2.2). This is a genuine
  capability gain, not a promise: real egui widgets publish real
  accessibility-tree nodes.
- **Color never the sole signal:** the focus ring is a shape (an outline
  with a real boundary), not a color highlight (§2.5); the cursor-icon
  affordance (§2.3) is an icon change, and every disclosure in §4 pairs
  its message with the full sentence, never a color-only cue.
- **Click-target sizing:** widget rects come from the document itself and
  may occasionally be small (a checkbox drawn at a genuinely tiny size by
  its original producer) — pdfce does not enlarge the *visual*, but should
  ensure the **hit-test** rect is never smaller than a reasonable minimum
  (reuse `ICON_BUTTON_SIZE`'s existing minimum-click-target reasoning,
  expanding the interactive hit-area slightly beyond a sub-minimum `/Rect`
  without changing what is drawn) — flagged as a concrete, cheap fix worth
  including even in P0.
- **Keyboard-operability, stated honestly:** every field-value interaction
  in P0 (text entry, checkbox toggle) is fully keyboard-operable end to
  end (focus via Tab, edit/toggle via typing/Space, commit via Enter/Tab).
  P1's list/combo overlays should be checked against the same bar before
  shipping (egui's native `ComboBox`/selectable-row widgets are themselves
  keyboard-operable, so this should hold by construction, but verify
  rather than assume, per this spec's own repeated "flag, don't assume"
  discipline).
- **Screen-reader gap, unlike Pass 6.1's, is narrower here, and this is
  worth stating precisely rather than lumping it in with the existing
  tracked gap:** the underlying **page raster itself** is still an image
  with no text alternative (the standing, project-wide gap named in
  `main.rs`'s own module docs since Pass 1), so a screen-reader user
  cannot yet discover *which fields exist and where* by reading the page.
  But **once a field has focus** (by Tab, or by some other means), the
  real overlay widget backing it (`TextEdit`/`ComboBox`/interact-target)
  **is** a normal accesskit-exposed control with whatever label the field
  model resolves (`/TU`, the field's user-facing alternate name, §12.7.3.3,
  if `pdfce-core`'s field model surfaces it — flag to the engineer that
  `/TU` should be threaded through as the accessible name specifically
  *because* this Pass can finally make use of it, unlike every prior Pass
  which had no real focusable form-field widget to attach a name to).

---

## 9. `ui_text.rs` catalog

New section header: `// Form field filling (Pass 7)`.

**Document-level disclosure (status bar, alongside `annotations_need_
appearances`):**
- `form_document_summary(fillable: usize, total: usize) -> String` — §4.3.
- `form_js_computed_fields_note(count: usize) -> String` — §4.2's
  document-level count.
- `regenerate_appearances_button() -> &'static str` — *"Regenerate
  appearances"*.
- `regenerate_appearances_tooltip() -> &'static str` — names what it does
  and that it is reversible.
- `regenerate_appearances_succeeded(count: usize) -> String` — §4.1's
  narrator line, verbatim reuse of the "Use Undo … until you save" clause.

**Per-field tooltips:**
- `form_field_js_computed_tooltip() -> &'static str` — §4.2's per-field
  form.
- `form_field_no_appearance_note() -> &'static str` — §3.2's honest
  no-visual-change disclosure.
- `form_field_certification_disabled_tooltip() -> &'static str` — §6.3.
- `pushbutton_tooltip() -> &'static str` / `pushbutton_clicked_note() ->
  &'static str` — §3.6.
- `text_field_multiline_hint() -> &'static str` — §3.1's Tab-to-commit
  reminder.

**Flatten:**
- `flatten_button() -> &'static str`.
- `flatten_tooltip() -> &'static str` — §5.1's full delete-shaped
  disclosure, verbatim.
- `flatten_succeeded(count: usize) -> String`.

**Tools dock:**
- `tool_export_form_data_label() -> &'static str`.
- `tool_import_form_data_label() -> &'static str`.
- `import_form_data_succeeded(matched: usize, unmatched: usize) ->
  String` — §5.2's named-mismatch line.

**Undo/redo labels** — extend `command_label`'s existing match, one arm per
new `CommandKind` variant (`SetFieldValue`, `ToggleCheckbox`,
`SetRadioGroup`, `SetListSelection`, `SetComboSelection`, `FlattenForm`,
`RegenerateAppearances`, `ImportFormData` — exact core-side naming is the
engineer's call, not dictated here, per the `pass-6.1` precedent) — each
gets a plain-English label the same way `CommandKind::RotatePages` already
does, so Undo/Redo tooltips name what they would reverse rather than
falling back to the generic "the last change."

---

## 10. Undo / write-path summary

| Operation | `EditSession` command? | Writes anything? |
|---|---|---|
| Focus/defocus/Tab between fields | No | No — pure UI focus, not an edit |
| Typing in a focused text field before commit | No | No — draft, not fact (Pass 3.1's "keystroke ≠ edit" rule, extended) |
| Commit a text field's value (Tab/Enter/click-elsewhere/Save-triggered flush) | **Yes — one command** | Not until Save (staged, R45) |
| Toggle a checkbox / click a radio in a group | **Yes — one command per click** (radio: covers the whole group flip) | Not until Save |
| Commit a list/combo selection | **Yes — one command** (multi-select: the whole set, not per-row) | Not until Save |
| Flatten | **Yes — one command**, batched across every field | Not until Save |
| Regenerate appearances | **Yes — one command**, batched | Not until Save |
| Import form data | **Yes — one command**, batched | Not until Save |
| Export form data | No — reads the open document, writes an unrelated file | Yes — the exported file, immediately (not staged, not undoable, exactly like Extract/Combine) |
| Save (with any pending form edits) | N/A | Yes — incremental, unchanged Save path |

---

## 11. Priority table (consolidated)

| Item | Priority | Note |
|---|---|---|
| Text field fill (single-line + multiline + `/MaxLen`), one-command commit at the field-navigation boundary | **P0** | Dominant real-world field type by measured share |
| Checkbox toggle (click + Space), honest no-appearance disclosure | **P0** | Cheap, high-value, no draft-state complexity |
| Pushbutton recognize-and-disclose | **P0** | Nearly free; prevents a silently-broken-looking control from shipping |
| The two mandatory disclosures (§4.2 JS-computed, existing §4.1 NeedAppearances line) | **P0** | Fuzzy-never-sneaky is a hard bar, not a nicety, for a first form-fill release |
| Document-is-a-form status line (§4.3) | **P0** | Cheap, answers the task's own discoverability question directly |
| Focus ring, cursor affordance, canvas-focusable plumbing, Tab/Shift-Tab via native egui focus | **P0** | Prerequisite for everything else in this spec |
| The `forbids_structural_change` `/P 2`/`/P 3` fix (§6.2) | **P0 — `pdfce-core`, required, not optional** | Without it, the Pass's headline scenario (fill a certified form) is broken on the most common certification permission level |
| Radio-group exclusivity | **P1** | Needs group-sibling resolution in the field model |
| List box, combo box (incl. editable-combo) | **P1** | Overlay-widget design work, lower measured prevalence than text/checkbox |
| Comb-field fixed-pitch rendering (both live overlay and generated appearance) | **P1** | `vartext.rs` needs a comb-aware code path |
| Flatten | **P1** | Genuinely separate capability from filling; ships its own tooltip disclosure once ready |
| Export/Import form data (FDF/XFDF) | **P1** | Tools dock, reuses existing file-dialog/atomic-write patterns |
| Regenerate appearances (the R51 remedy button) | **P1** | The existing NeedAppearances disclosure line ships in P0 without an action attached; this pairs an action with it |
| Proactive certification gray-out (beyond the required §6.2 fix) | **P1** | Nicety on top of an already-correct commit-time refusal, mirroring `pass-6.1` §5.4's own P1/P0 split |
| Multi-page Tab-overflow auto-navigation (Tab past the last field on a page advances to the next page's first field) | **P1** | Genuinely useful for real multi-page forms; not required for a first usable single-page-at-a-time fill experience |
| `/TU` threaded through as the accessible name for focused field overlays | **P1** | Depends on the field model surfacing it; flag to the engineer as worth doing as soon as it's available, not deferred indefinitely |

**If the engineer must cut this Pass down further: ship P0 alone** —
text-field + checkbox fill, both disclosures, the document-is-a-form line,
and the `/P 2`/`/P 3` core fix. This is a complete, honest, genuinely useful
first form-fill release on its own, exactly mirroring how `pass-6.1`
anticipated shipping its own six shape tools without the quad-point family
if time ran out.

---

## 12. Open items for the librarian

1. **The `forbids_structural_change` `/P 2`/`/P 3` fix (§6.2) is a required
   `pdfce-core` task for THIS Pass, not a deferred P1/P2 refinement** —
   distinct in urgency from `pass-6.1`'s own `/P 3`-annotation residual
   (which was correctly shippable-conservative). Record this distinction
   explicitly so the two "the coarse gate over-refuses at some `/P`
   value" findings are not conflated into one follow-up item with one
   priority — they have different urgency for different reasons.
2. **R48's "offer/force a full rewrite" half remains unbuildable** until
   pdfce has a full-rewrite save mode at all (still true as of this Pass,
   carried from Pass 3.1's own module-doc note). Flatten ships with the
   delete-shaped tooltip disclosure only; the full-rewrite affordance is a
   named residual, not silently dropped.
3. **A fourth occurrence of the `accesskit`/egui-AT gap, but narrower than
   the prior three** — this Pass is the first to actually *close* part of
   it (real focus/AT exposure for checkbox/radio/list/combo/text-overlay),
   while the underlying page-raster-has-no-text-alternative half remains
   open. Worth a corrective note distinguishing "gap closed here" from
   "gap still open there" rather than another blanket "same gap, fourth
   time" entry.
4. **`/TU` as the accessible name for focused field overlays** (§8) is
   contingent on the field model surfacing it — flag as a live dependency
   to check off once `pdfce-core`'s Pass 7 field model lands, not an
   independent follow-up item.
5. **Evolution path for a future "Form ▾" menu** (§5.3) — if a later Pass
   adds a second whole-document form-level action beyond Flatten, promote
   from a single toolbar button to a menu at that point, mirroring how
   Markup/Text menus themselves emerged from single-purpose beginnings.
6. **`/GoTo`-only pushbutton action support** (§3.6) — named as a possible,
   meaningfully-lower-risk future Backlog idea distinct from full action-
   dictionary execution. Not asserted as planned.
