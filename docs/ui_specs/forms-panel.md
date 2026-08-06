# Forms panel — interactive AcroForm fill, flatten, and data interchange

> Authored by `pdfce-ui-specialist`, 2026-08-05, on dispatch from the
> engineer. **This supersedes `docs/ui_specs/pass-7-form-fill.md` for
> placement and interaction-shape decisions.** That spec was correct about
> almost everything at the field-interaction level (the per-type behavior in
> its §3, the two mandatory disclosures in its §4, the certification-gate
> finding in its §6.2) and this document reuses those parts by reference
> rather than re-deriving them. What it got wrong is not a mistake — it
> predates four shell changes that did not exist when it was written and
> that make its central architectural call (a mode-less **canvas overlay**:
> real `egui::TextEdit`/`ComboBox` widgets projected onto each widget's
> screen rect, hit-tested against `ui.interact` on every frame) no longer
> the cheapest or best-fitting answer:
>
> 1. **Pass 12.0** formalized `CanvasTool` as the one substrate every
>    interactive canvas behavior attaches to. Pass 7's own module comment in
>    `canvas.rs` already records "Pass 7's form-fill adds NONE — it needs no
>    tool mode at all" — true, but written before there was a `CanvasTool`
>    enum to not-join, and before the project had ANY working precedent for
>    what building real per-widget canvas hit-testing costs.
> 2. **Pass 24.3** replaced the right dock's four-pane split with
>    `ribbon::PaneSubject` — a single switchable surface in the LEFT Tool
>    Options dock, explicitly built so "the ribbon picks the activity; the
>    sidebar holds its controls." A sustained, document-wide, non-canvas-tool
>    activity (Redact, Batch Tools) now has an established home shape that
>    Pass 7's spec had no equivalent of when it was written.
> 3. **Pass 34.1/34.2** proved out real, working, ALREADY-SHIPPED precedents
>    for exactly the two mechanisms this document needs — implicit
>    commit-on-interaction-end for a value editor (`place_draft_commit`,
>    `crates/pdfce-gui/src/main.rs:15767`), and a list-driven,
>    state-then-action-then-detail panel with row-level navigate buttons
>    (`redact_panel`, `main.rs:4143`). Neither existed in July.
> 4. **Pass 8** built `EditSession::to_full_bytes`/`writer::save_full`
>    (`crates/pdfce-core/src/edit.rs:1797`) for redaction's forced full
>    rewrite. Pass 7's spec named "no full-rewrite save mode exists yet" as
>    the reason Flatten's R48 residual could not be closed — that primitive
>    now exists, changing that finding (§7 below).
>
> **Read first, and take as given rather than re-argued here:**
> `docs/ui_specs/pass-7-form-fill.md` §3 (per-type field behavior), §4 (the
> two mandatory disclosures), §6.2 (the `/P` certification-gate finding —
> **already shipped**, `check_certification_for_fill` in `edit.rs`, permits
> fill at `/P >= 2`, refuses at `/P 1`); `docs/ui_specs/tool-options-dock-
> and-ce-dimension-properties.md` (the current Tool Options / `PaneSubject`
> shape, and the "blast radius" commit-weight test this document also
> applies); `crates/pdfce-gui/src/{ribbon.rs, dock.rs, canvas.rs, main.rs}`
> (`redact_panel`, `tool_options_panel`, `place_draft_commit`);
> `crates/pdfce-core/src/{forms.rs, edit.rs, fdf.rs}` (the ALREADY-SHIPPED
> field model and every fill/flatten/import/export verb — Pass 7.0/7.1 are
> core-complete; this document is GUI-only).
>
> **This spec does not cover:** field *authoring* (creating, moving, or
> deleting fields — a distinct, larger capability, closer in shape to Pass
> 6.1's tools than to this one); executing any `/A`/`/AA` action of any kind
> (unchanged decision 009 posture A — pushbuttons, calculated fields, and
> validation scripts are recognized and disclosed, never run); XFA (tracked,
> unresolved backlog item, `CLAUDE.md`'s outstanding-items list); Reader
> Extensions / usage-rights (an Adobe-proprietary, EULA-gated mechanism —
> confirmed `out_of_scope` by the Acrobat-parity RAG, not a pdfce gap).

---

## 0. Scope table

| Bucket | Contents | Ships this Pass? |
|---|---|---|
| **P0** | New `PaneSubject::Forms`, reached from a new ribbon group; a list-driven field panel — one real egui widget per fillable field, in file order; text fill (single-line + multiline + `/MaxLen`, password-masked, comb as a plain bounded field with no cell dividers); checkbox toggle; the "document is a form" + JS-computed-value + rich-text disclosures; read-only/certification-disabled rows (R83); pushbutton recognize-and-disclose; commit-on-`lost_focus()` for text, immediate commit for checkbox; one `EditSession` command per commit | **Yes, required** |
| **P1** | Radio-group rows (exclusive `ui.radio_value` cluster); choice fields (`egui::ComboBox` for single-select, a checkbox stack + inline "Apply" for multi-select); Flatten (tooltip-only, delete-shaped — §7); Export/Import form data in the existing Batch Tools pane; "Regenerate appearances" action beside the `/NeedAppearances` disclosure; per-row page jump (`Action::GoToPage`) + a passive on-canvas highlight rect for the selected row; a `(required)` label marker; per-page collapsible grouping of the list | Ship if runway allows; a complete, honest P0-only release is acceptable if not |
| **P2 — named, not scoped, and flagged to the engineer as its own decision** | True click-on-the-page-to-focus-and-edit (a new `CanvasTool`-adjacent hit-testing mechanism — see §2.3); comb fixed-pitch cell-divider rendering in the inline editor; a "highlight all fillable fields" canvas-wide toggle (needs the P2 hit-testing infra first); `/Tabs` S/R/C-computed ordering (v1's file-order answer is the honest baseline — §2.4); rich-text field editing (v1 refuses and discloses rather than silently downgrading — §3.1) | No — named residuals, not silent gaps |
| **Not scoped at all, any priority** | Field authoring; `/A`/`/AA` execution; XFA; Reader Extensions/usage rights | No |

---

## 1. Placement — a fifth `PaneSubject`, on its own ribbon group, in the Edit tab

### 1.1 Decision

Add **`PaneSubject::Forms`**, a sibling of `Properties`/`BatchTools`/`Redact`
in `ribbon::PaneSubject` (`ribbon.rs:429`), dispatched from
`tool_options_panel` (`main.rs:8176`) exactly like the three existing
pinned subjects: `PaneSubject::Forms => { self.forms_panel(ui, actions);
return; }`.

Reached from a **new `RibbonGroup::Forms`**, placed on **`RibbonTab::Edit`**
("What am I CHANGING about what is already there?", `ribbon.rs:200`), not
on `Tools`. The group holds one button in P0 (`forms_panel_button()` —
*"Fields"* or *"Fill Form"*, naming is the engineer's call) that sets
`pane_subject = PaneSubject::Forms`, mirroring exactly how `Batch`/`Protect`
buttons set `BatchTools`/`Redact` today.

### 1.2 Why not `ActiveTool`

`PaneSubject::ActiveTool` **follows `doc.active_tool: Option<CanvasTool>`**
(`ribbon.rs` doc comment, `main.rs:8199`) — it shows the empty state when no
tool is armed. Form filling has, correctly, no `CanvasTool` variant (the
`canvas.rs` module comment already states this and this document does not
change it — see §2). A pane whose visibility is *conditioned on* a tool
being armed cannot host an activity that, by design, arms no tool. This
alone rules `ActiveTool` out; it is not a close call.

### 1.3 Why not fold into the widened `Properties` pane

`Properties` was JUST widened at Pass 34.2 to three scope-named sections
(selected ce dimension / ce-dimension groups / document `/Info`), and its
own doc comment states the reasoning for stopping at three: "the first
thing an operator does after changing a group's units is look at what it
did to the ce dimension they have selected, and two panes cannot show that
in one view." Forms fill is not a *property of the currently selected
thing* — there is no "selected field" concept elsewhere in the app for it
to key off, and unlike the ce-dimension/group relationship, nothing else in
the UI produces a "selected field" as a side effect of another action. It
is its own sustained, document-wide activity, structurally the same shape
as Redact ("the redaction review surface") and Batch Tools ("operations
across whole files"), not a properties-of-a-selection surface. Folding it
in would also make an already-three-way pane four-way for an activity that
shares no state with the other three.

### 1.4 Why `Edit`, not `Tools` (where `Redact` lives)

`Redact` sits under `Tools` > `Protect` for a **named, single reason**: "grouped
apart for that reason" — Redact's own group doc comment ties its placement
to its danger, isolating an irreversible-after-Apply operation from
everything else. That reasoning does not transfer to form-fill: per rule 7
and per `pass-7-form-fill.md` §3.2, filling is **reversible pre-save and
low-stakes** — no confirmation, atomic commits, exactly the profile of an
ordinary edit. Placing it beside Redact would misrepresent its risk level
to an operator scanning the ribbon for "the dangerous stuff." `Edit`'s own
one-line test — "What am I CHANGING about what is already there?" — is a
direct, literal match: filling in a field that already exists in the
document is changing what is already there, the same description that puts
page rotation and Undo/Redo on that tab today.

### 1.5 Discoverability — the "document is a form" disclosure gets an action button

Per `pass-7-form-fill.md` §4.3 (still correct, unchanged): a document-level
status-bar line, *"This document is a form: {fillable} of {total} field(s)
can be filled in,"* shown whenever `parse_acroform` returns `Some` with at
least one terminal field. **New in this version:** put a button directly
beside it — `open_forms_panel_button()`, *"Fill in form fields"* — that sets
`pane_subject = PaneSubject::Forms` the same way `Action::GoToPage` is
pushed from a redact-panel row. This reuses the exact "the fact and its
remedy stay adjacent" locality principle the codebase already applies to
`annotations_need_appearances()`/`regenerate_appearances_button()`, and it
gives the panel a **second, contextual** entry point without inventing a
second convention — the ribbon button remains the primary, always-visible
one; this is the "I just opened a form and it told me it's fillable" path.

**No auto-switch.** Opening a document that happens to be a form does
**not** programmatically change `pane_subject`. An operator who has
`Properties` or `BatchTools` pinned open for an unrelated reason should not
have it silently replaced by something they did not ask for — that is
exactly the "the ribbon (or an explicit button) picks the activity"
discipline the whole `PaneSubject` arc converged on. The status-bar line is
the disclosure; the button beside it is the invitation; nothing is forced.

---

## 2. List-driven vs. canvas-driven — list-driven for P0, and why this is not merely the cheaper answer

### 2.1 The honest tension in the task's own framing

The task names the real trade correctly: Acrobat's actual behavior is
click-the-widget-on-the-page, and pdfce's canvas has **zero widget
hit-testing today** — confirmed by reading `canvas.rs` in full.
`CanvasTargetProvider` (the one hit-testing trait that exists) is for
**discrete-object selection** in the vector/measure tools' sense (Pass
12.0 module docs), and a form widget's `/Rect` is not one of the objects
that trait was built to test against. Building real per-widget hit-testing,
a drag-vs-pan conflict resolution scoped to individual rects, and projected
`TextEdit`/`ComboBox` overlays (the old spec's §2.2–§2.6) is genuinely
substantial, first-of-its-kind canvas work — not a small extension of
something that already exists for a different purpose.

### 2.2 Decision: list-driven for P0, with a passive canvas highlight in P1, full click-to-edit named as P2

**A list-driven panel is not the frustrating fallback here — it is the
better-fitting answer for THIS shell, for a reason independent of cost.**
Every accessibility win the old spec argued for canvas overlays ("the
single biggest, cheapest accessibility win available in this Pass": real
Tab-focus, real `accesskit` exposure, native Space/Enter handling) is
obtained **more directly** by a list of ordinary `egui::TextEdit`/
`egui::Checkbox`/`egui::ComboBox` rows in a normal dock panel than by the
same widgets *projected onto a raster and hit-tested per frame* — a list
row needs no geometry bridge, no per-frame `page_to_screen` projection, and
no "which overlapping `interact` wins" verification the old spec had to
flag as unresolved (§2.4 of that document). Tab order **is** the panel's
row order, natively, with no custom focus-cycling code at all. This is the
`redact_panel`/`properties_panel` precedent extended to a new subject, not
a new pattern.

What the list-driven design does **not** give an operator is "click the
field where I see it on the page" — the single most natural gesture for a
form, and the thing the task rightly worries a list-only release would
lose. Two mitigations, at two different costs:

- **P1 — passive highlight, no hit-testing needed.** Clicking a field's row
  pushes `Action::GoToPage(page_index)` (exactly the `redact_panel` row
  precedent, `main.rs:4291`) **and** sets a new, small piece of view state
  — `doc.view.highlighted_widget: Option<(ObjId, pdfce_core::geom::Rect)>`
  or equivalent. The ordinary `CentralPanel` render pass draws a 2px
  outline (not a color wash — rule 6) around that rect, projected via the
  EXISTING `viewer::page_to_screen` (`viewer.rs:427`), whenever it is
  `Some` and the current page matches. This needs **no** `CanvasTool`
  variant, no hit-testing, and no interaction at all on the canvas side —
  it is a read of state that a list click set, the same shape as how
  `redact::redaction_marks` are drawn every frame regardless of any tool
  being armed. Cheap, and it directly answers "where is the field I just
  selected."
- **P2 — true click-to-edit — named, not scoped, flagged as its own
  decision.** Making a plain click **on the page** focus the matching
  field (the old spec's full ambition) needs real hit-testing against
  widget rects, and now that `CanvasTool` is an inhabited enum with four
  real variants and established conventions for gesture ownership,
  building it is a well-shaped, schedulable Pass — but it is a
  **different, larger** piece of work than anything else in this
  document, and P0's list already makes a fillable PDF fillable without
  it. Recommend the engineer treat this as its own future Pass rather
  than a P1 stretch goal for this one. **This is the one item in this
  spec where the honest answer is "this needs new canvas-substrate
  infrastructure first," flagged per the task's own request.**

### 2.3 What "canvas-driven or list-driven" actually turned on

Not cost alone — the task asked to argue it. The deciding fact is that
**the current shell already has an established, working home for
exactly this activity shape** (`PaneSubject`, `redact_panel`'s
list-with-per-row-navigation), and the old spec's canvas-overlay design
was reasoned entirely against a shell that had neither a `PaneSubject`
taxonomy nor a shipped commit-on-interaction-end precedent to build on. A
spec written today, for the shell as it exists today, reaches for the
tool that already fits rather than the one that was the best available
answer in July.

### 2.4 Field order — the file's own order, and why that is the honest v1 answer, not a placeholder

The list is populated from `AcroForm::fields`, in the order
`parse_acroform` already produces: field-tree DFS order, i.e. the order
`/Fields`/`/Kids` presents them (`forms.rs:516` doc comment). Per the
Acrobat-parity RAG's `forms__tab_order.md`, Acrobat's own **"Unspecified"**
tab-order state — the one a page carries before any of Structure/Row/
Column is actively chosen — falls back to the raw `/Annots` array order,
which is the file's own order. **v1's field-list order is therefore not an
approximation of Acrobat's behavior; it is Acrobat's own documented
fallback**, applied honestly rather than guessed at. Computing Structure/
Row/Column ordering is real, spec-governed work (`/Tabs`, tag-tree
traversal) that belongs with field *authoring*, not fill — named as a P2
residual, not silently absent.

---

## 3. The panel — widget tree

```
PdfceApp::forms_panel(ui, actions)
├─ ui.heading(forms_pane_title())                       // "Fill Form Fields"
├─ ui.label(forms_panel_intro())                         // one sentence, what this pane is for
├─ ui.separator()
│
├─ [state] — mirrors redact_panel's "state → action → detail" order (R86,
│            applied proactively here rather than rediscovered)
│  ├─ Status::Open(doc) guard; else forms_panel_no_document_hint()
│  ├─ let form = parse_acroform(&doc.session.graph())
│  │  else → forms_panel_not_a_form_hint()  // honest empty state, not blank
│  ├─ ui.label(form_document_summary(fillable, total))          // §4.3 of pass-7 spec, verbatim
│  ├─ [P0] if any field.has_additional_actions with a calc/CO hook:
│  │      ui.label(form_js_computed_fields_note(count))          // §4.2, document-level
│  ├─ [P1] if form.need_appearances || any !field.has_appearance():
│  │      ui.horizontal(|ui| {
│  │        ui.colored_label(warn_fg_color, annotations_need_appearances());
│  │        if ui.button(regenerate_appearances_button())
│  │             .on_hover_text(regenerate_appearances_tooltip())
│  │             .clicked() { actions.push(Action::RegenerateFieldAppearances); }
│  │      })
│  │      // placed ABOVE the list — the R86 lesson applied in advance,
│  │      // not rediscovered on a narrow dock the way redact_panel's was.
│  ├─ [P1] ui.horizontal(|ui| {
│  │        if ui.button(flatten_button())
│  │             .on_hover_text(flatten_tooltip())    // §7 below — verbatim
│  │             .clicked() { actions.push(Action::FlattenForm); }
│  │      })
│
├─ ui.separator()
│
└─ [detail] — the field list, in the pane's OWN ScrollArea (no nested
│            ScrollArea — the same lesson redact_panel's comment names
│            explicitly for a ~250pt pane)
   └─ for field in &form.fields (file order, §2.4):
        forms_field_row(ui, field, doc, actions)   // §3.1–§3.6 dispatch by type
```

### 3.1 Row shape, common to every field type

- **Visible label:** `field.alternate_name` (`/TU`) decoded, if present,
  **falling back to** `field.fully_qualified_name`. This is a genuine,
  RAG-grounded improvement over showing the raw dotted PDF name by
  default: `forms__field_property_model.md` documents `/TU` as the
  practical, load-bearing **accessible name** for forms specifically
  (screen readers read the interactive-field layer through `/TU`, not the
  tag tree) — showing the same string as the row's visual label means an
  operator reads exactly what a screen reader announces, rather than two
  different names for the same field. The row's tooltip always states the
  raw fully-qualified name too (`form_field_row_tooltip(fqn)`), because an
  operator diagnosing an FDF import mismatch (§6) needs the technical name,
  and `/TU` may be absent.
- **Page suffix (P0):** `" (p. {n})"` appended to the label when
  `widget.page` resolves to a page index (a small `ObjId → page index`
  lookup the engineer builds over `doc.pages` — flagged in §9 as a
  concrete, small implementation task, not dictated further here). When
  `widget.page` is `None` or does not resolve, the suffix is omitted and
  any P1 "jump to page" control on that row is disabled with a tooltip
  naming why (R83) rather than silently doing nothing.
- **`(required)` marker (P1):** appended to the label, as **text**, when
  `field.flags.required()` — never a color-only cue (rule 6). Cheap,
  deferred to P1 only to keep P0's cut minimal, not because it is
  difficult.
- **Disabled, not hidden, when unfillable** (R83, `redact_panel`'s
  `ui.add_enabled_ui` convention, reused verbatim):
  - `field.flags.read_only()` → the row's control is disabled with
    `form_field_readonly_tooltip()`.
  - The certification gate refuses fill on this document (`/P 1`, or a
    `/FieldMDP` lock naming this field — see `forms__permissions_
    signature_interaction.md`'s FieldMDP note) → disabled with
    `form_field_certification_disabled_tooltip()` (`pass-7-form-fill.md`
    §6.3, unchanged).
  - `field.field_type == Signature` → disabled, `form_field_signature_
    not_supported_note()`: *"Signature field — pdfce does not create or
    verify signatures for this field yet."* Recognition-only, matching
    core's own Pass 7 scope; this is a disabled ROW, not a missing one —
    an operator who scrolls past a signature field should see that pdfce
    knows it is there.
  - **New finding, P0-required:** a **text** field with `field.flags.has
    (FieldFlags::RICH_TEXT)` set → disabled, `form_field_rich_text_not_
    supported_note()`: *"This field stores rich (formatted) text. pdfce
    can only edit it as plain text, which would discard its formatting —
    not supported yet."` **Why this is P0, not a nicety:** `fill_text_
    field` today does not special-case the `RichText` bit — nothing in
    `edit.rs` refuses or warns before overwriting a rich-text field's `/V`
    with plain decoded text and regenerating a plain (non-rich) appearance
    via `vartext.rs`. Left unguarded, an operator editing a rich-text
    field through this panel would silently lose whatever formatting was
    there — a direct rule-4 violation (a lossy conversion presented as an
    ordinary edit) waiting to happen the first time a real-world rich-text
    field reaches this panel. The GUI-side disable in this row is the
    cheap, immediate fix; whether `pdfce-core` should *also* refuse
    `fill_text_field` on a `RichText`-flagged field (a defense-in-depth
    guard, so `pdfce-cli fill-field` gets the same protection) is a
    `pdfce-core` question flagged to the engineer in §9, not decided here.
    **Caution:** the `RichText` bit (26, value `33554432`) is the *same
    bit value* as `RadiosInUnison` on a button field (`forms.rs`'s own
    doc comment: "Shares its value with `RadiosInUnison` — decode against
    the resolved `/FT`") — this check must be gated on
    `field.field_type == Some(FieldType::Text)` before testing the bit, or
    it will misfire on radio groups.

### 3.2 Text field (`/FT /Tx`, not read-only, not rich-text)

- `field.max_len` present → an `egui::TextEdit` with a character-count
  caption below it (`"{len}/{max_len}"`), truncating input at the limit
  live rather than discovering it at commit (`pass-7-form-fill.md` §3.1's
  reasoning, unchanged).
- `field.flags.has(FieldFlags::MULTILINE)` → `TextEdit::multiline()`;
  Enter inserts a newline, does not commit. Otherwise `TextEdit::
  singleline()`; Enter both commits (via the same `lost_focus()` path,
  §5) and defocuses, which is `TextEdit::singleline`'s native behavior —
  no custom Enter handling needed, unlike the old spec's canvas overlay,
  which had to special-case Enter itself.
- `field.flags.has(FieldFlags::PASSWORD)` → `TextEdit::password(true)`.
  Masking is display-only (the RAG's own caution, `forms__text_fields.md`)
  — the row's tooltip states this plainly:
  `form_field_password_tooltip()` — *"Entered text is masked on screen.
  This does not encrypt the value — it is stored as plain text inside the
  PDF."* (fuzzy-never-sneaky: a masked field reads as "secure" to an
  operator who has not been told otherwise).
- **Comb (`/Ff` bit 25) — P0 renders as a plain bounded `TextEdit` with
  `/MaxLen` enforced, no cell-divider ticks.** The old spec's live
  divider-line overlay (§3.1 of that document) is a real, cheap-to-add
  visual nicety, deferred to P2 rather than P0 or P1 because it changes
  nothing about correctness or fillability — named explicitly so a future
  reader does not read its absence as an oversight.
- Placeholder/label layout: **label above, editor below** (vertical
  stacking), not label-beside-editor. `dock.rs`'s own module doc already
  names the generalizable caution — a narrow column clips a two-column
  layout — and this panel lives in the same dock at the same default
  width; stacking avoids re-learning that lesson a second time.

### 3.3 Checkbox (`/FT /Btn`, `ButtonKind::Check`)

- A real `ui.checkbox(&mut checked, "")`, bound to `widget.appearance_
  state == on_state` (the field's single on-state name, from `widget.
  on_states`). Click or Space (native to `egui::Checkbox`, no custom
  handling) toggles.
- **Commit: immediate**, one `EditSession::set_button_state` call per
  toggle — no draft state, matching `pass-7-form-fill.md` §3.2's reasoning
  unchanged (atomic, like a markup shape commit).
- **No confirmation** (rule 7 — reversible pre-save, low-stakes).
- The honest no-visual-change disclosure (`pass-7-form-fill.md` §3.2, a
  one-shot `edit_note`) carries over unchanged: when the widget's `/AP`
  lacks a sub-stream for the state being toggled INTO, the value changes
  but nothing paints differently — disclosed, not left as an apparent
  no-op.

### 3.4 Radio group (`/FT /Btn`, `ButtonKind::Radio`) — P1

All widgets sharing one `fully_qualified_name` render as **one row**: a
horizontal (or, on a narrow dock, vertical — engineer's layout call)
cluster of `ui.radio_value(&mut selected, widget_on_state, label)`, one per
sibling widget, `label` being the on-state name decoded as text (no `/MK`
caption is modeled in `Widget`, so the on-state name is the only string
available — named as the honest current ceiling, not a simplification to
fix here). Selecting any member is **one** `EditSession::set_button_state`
call — `pdfce-core` already applies the whole-group `/AS` flip and `/V`
update as one command (`forms.rs`/`edit.rs`), so the GUI does not need to
special-case "clear the siblings" itself; it just sends the click.
Re-clicking the selected member when `NoToggleToOff` is set is a
`pdfce-core` no-op by construction (per `pass-7-form-fill.md` §3.3's
"the GUI always sends a click-on-this-widget request; `pdfce-core` decides
whether it is honored" reasoning — unchanged, still correctly keeps the
rule in exactly one place).

### 3.5 Choice field (`/FT /Ch`) — P1

- **Single-select** (list box or combo, `MultiSelect` clear): an
  `egui::ComboBox` populated from `field.options` (display strings shown,
  export values sent on commit). Commit **on selection change**, one
  `EditSession::set_choice_value` call — no separate "Apply" needed, since
  a `ComboBox` selection is itself a discrete, unambiguous act (unlike a
  multi-select stack, below).
- **Editable combo** (`Combo` + `Edit` bits both set): pair the
  `ComboBox` with a `TextEdit` the operator can type a value into that is
  not in `field.options` — commits via the same `lost_focus()` path as an
  ordinary text field (§5).
- **Multi-select list** (`MultiSelect` set): a small vertical stack of
  `ui.checkbox` rows, one per `field.options` entry, in a bounded
  height (its own tiny inner scroll only if `field.options.len()` is
  large — the one deliberate exception to "no nested `ScrollArea`," scoped
  to a single field's own option list rather than the whole panel). An
  explicit small **"Apply"** button commits the whole set as **one**
  `EditSession::set_choice_value` call — kept from `pass-7-form-fill.md`
  §3.4's reasoning (avoid N undo entries for N checkbox toggles), adapted
  from "commits when the transient overlay closes" (no such moment exists
  in a persistent list row) to "commits on an explicit small button,"
  which is the natural translation of the same rule into this shape.

### 3.6 Pushbutton (`/FT /Btn`, `ButtonKind::Push`) — recognize, disclose, do nothing

Unchanged from `pass-7-form-fill.md` §3.6 in substance, adapted to a row:
the row renders as a disabled-LOOKING but actually-clickable button (so
the click itself can be disclosed — an inert row an operator cannot even
press would hide the "nothing happens" fact rather than state it), with
`pushbutton_tooltip()` always present on hover (*"This is a button. pdfce
does not run button actions yet... clicking it does nothing to the
document."*) and `pushbutton_clicked_note()` as a one-shot narrator line on
click, reusing the existing `edit_note` channel — no new UI surface for a
control that, by design, changes nothing.

---

## 4. Commit semantics — reuse `place_draft_commit`'s exact three-condition shape

### 4.1 The precedent, cited directly rather than re-derived

Pass 34.2's `place_draft_commit` (`main.rs:15767`) already answers "when
does a draft become a command" for exactly this shape of problem (an
`egui` value editor whose live-drag/live-type state must NOT become an
`EditSession` command on every frame). Its own doc comment names the three
conditions and why each is load-bearing:

1. **`ended`** — the interaction is over, not merely "changed."
2. **the draft belongs to the currently-relevant target** — a stale draft
   from a previously-focused field must never commit onto whatever is
   focused now.
3. **the value actually differs from the document** — a focus/defocus with
   no real edit must not manufacture an undo entry.

### 4.2 Applied to a text field

`ended` = `response.lost_focus()` (there is no drag component for a text
field, so `drag_stopped()` is not part of this condition, unlike the
numeric-spinner case). Recommend a sibling pure function,
`form_field_commit(ended: bool, draft: Option<(String, String)>, fqn: &str,
current: &str) -> Option<String>`, mirroring `place_draft_commit`'s
signature shape and unit-testability (the same "cannot be reached by the
scripted-input harness, so a pure function with a unit test is the honest
substitute" reasoning applies here too — `lost_focus()` is exactly as
unreachable from `tools/gui-drive.ps1`'s synthetic input as `drag_stopped()`
is, per the same `D:/dev/rag/egui/eframe_035_raw_input_hook_synthetic_
event_injection.md` finding).

**Flag, don't assume (the same discipline `pass-7-form-fill.md` used
repeatedly, still correct practice):** verify in the pinned egui version
that `lost_focus()` fires reliably on every path that should count as
"the operator left this field" — Tab, Shift-Tab, a click on another row's
control, Ctrl+S while focused, and a `pane_subject` switch away from
`Forms` while a field is focused. The last two are genuinely uncertain:
does egui guarantee focus is dropped (and `lost_focus()` observed by the
row's own code) in the SAME frame the keyboard shortcut or panel switch is
processed, or could a draft be silently discarded because the row that
would have read `lost_focus()` never draws again? This is worth a
concrete test before shipping, not an assumption — if the guarantee does
not hold, the fallback is an explicit flush-on-save/flush-on-pane-switch
call the same way `pass-7-form-fill.md` §2.6 specified for its canvas
version of this problem ("Save/Undo/navigation... first COMMITS the
current draft").

### 4.3 Checkbox / radio / choice — no draft state, so this section does not apply

Per §3.3–§3.5: these commit immediately (checkbox, radio, single-select
choice) or on an explicit small "Apply" (multi-select choice) — there is
no keystroke-vs-commit distinction to manage, so `place_draft_commit`'s
three-condition shape is not needed for them at all. Stated explicitly so
a future reader does not go looking for a draft mechanism that was
deliberately not built for these types.

### 4.4 Escape

While a text field row has focus, Escape reverts to the value captured at
the moment focus was gained (not to an earlier Undo-reachable state — Undo
remains the mechanism for reversing an already-committed change) and
defocuses without producing a command. Single-stage, per `pass-7-form-fill.md`
§2.6's reasoning (there is no "tool" to partially exit — unchanged, still
true here).

---

## 5. `/NeedAppearances` and appearance regeneration

Unchanged from `pass-7-form-fill.md` §4.1, relocated: the disclosure line
(`annotations_need_appearances()`, already shipped since Pass 6.0) and its
new `regenerate_appearances_button()` remedy live in the Forms panel's
**state** section (§3, above the list — the R86 placement lesson applied
proactively). One batched `EditSession::regenerate_appearances` call per
click, Undo reverses the whole regeneration in one step, narrator line
verbatim reuses *"Use Undo … until you save."* No confirmation (rule 7 —
reversible, non-destructive).

**New, minor finding worth naming:** the Acrobat-parity RAG's
`forms__appearance_generation_and_needappearances.md` documents pdfce's
existing posture (always bake a correct `/AP`, never rely on
`/NeedAppearances` for pdfce's own output) as the **correct, RAG-endorsed**
design — third-party SDK vendors converge on exactly this as the safer
strategy, and Acrobat's own handling of the flag is undocumented even to
competing engine vendors. Nothing to change here; cited so the engineer has
the RAG's corroboration on record rather than treating the existing
core-side choice as merely a project preference.

---

## 6. Export / Import form data (FDF/XFDF) — unchanged placement, in Batch Tools

`pass-7-form-fill.md` §5.2's placement reasoning is unchanged and still
correct under the current shell: both operations' argument is a file
**outside** the open document, which is `tools_dock_intro()`'s own defining
sentence. `PaneSubject::BatchTools` (reached via `RibbonTab::Tools` >
`RibbonGroup::Batch`) is where this lives — `tool_export_form_data_label()`
/ `tool_import_form_data_label()`, reusing the existing `rfd::FileDialog` +
atomic-write pattern already used for Save/Extract/Combine. Import's
named-mismatch narrator line (`import_form_data_succeeded(matched,
unmatched)`) is unchanged. No new dialog convention.

---

## 7. Flatten — still delete-shaped, now with a concretely-verified reason why, plus a new finding

### 7.1 The weight, re-argued against the ACTUAL shipped mechanism (not a hypothetical one)

The task asked this to be argued, not copied from precedent — and there is
now a real, shipped precedent to argue against rather than the speculative
one `pass-7-form-fill.md` had to reason about in the abstract. Read
`crates/pdfce-gui/src/redact_apply.rs`'s own module doc: redaction's Apply
pipeline is built around **exactly two, and only two, full rewrites**
(`EditSession::to_full_bytes` called twice, once to materialize pending
marks and once inside `redact::apply_redactions` itself) — "there is
deliberately no `to_incremental_bytes` call anywhere in this file, and no
fallback that could introduce one." Redaction earns its heavy modal +
checkbox-acknowledgement gate (`redaction_apply_confirmation`,
`main.rs:4575`) because its Apply step **unconditionally, structurally**
destroys the superseded bytes — there is no code path where an ordinary
Save leaves anything recoverable.

**Flatten's own shipped implementation does not have this property.** Per
the Pass 7.1 ROADMAP entry: flatten **appends** a new overlay content
stream and invokes the widget's existing `/AP` as a page XObject — "existing
content streams stay byte-verbatim... R48 verified: incremental flatten
leaves the field dict recoverable in the prior revision." Under the
**default** save mode (incremental), a flattened field's prior `/V`/`/AP`
genuinely IS still forensically present in the file, exactly like page
deletion. Flatten's irreversibility is **conditional on which save mode is
used**, not unconditional the way redaction's is — this is the precise
distinction rule 2 draws between "genuinely irreversible once saved" and
everything else, and it is why Flatten earns `selection_delete_tooltip`'s
weight (a rich, honest tooltip, no blocking modal), not
`redaction_apply_confirmation`'s.

- `flatten_button()` in the Forms panel's state section (§3, above the
  list — placed alongside the Regenerate-appearances button, per the same
  R86 reasoning).
- `flatten_tooltip()` — carries `pass-7-form-fill.md` §5.1's full wording
  forward, with one clause updated per §7.2 below.
- **Commit:** one `EditSession::flatten_fields(None)` call (all fields),
  batched, Undo reverses the whole flatten in one step. Flatten uses the
  **strict** certification gate — already shipped (`check_certification`
  in `flatten_fields`, distinct from fill's `/P >= 2` gate) — so a
  certified document refuses flatten by name where it would still permit
  ordinary fill. This distinction is already correctly implemented in
  `pdfce-core`; the GUI needs only to surface whichever `EditError` comes
  back through the existing `save_result = Some(SaveOutcome::Failed(...))`
  channel, no new refusal UI.

### 7.2 New finding: R48's "offer a true removal" residual may no longer be unbuildable

`pass-7-form-fill.md` §5.1 named the full-rewrite half of R48 as "not
buildable yet" because, at the time, pdfce had no full-rewrite save mode at
all. **That is no longer true.** `EditSession::to_full_bytes`/
`writer::save_full` (`edit.rs:1797`) now exists, built for Pass 8's
redaction pipeline. It is a general session method, not something
redaction-specific — its own doc comment states only two caveats
(destroys existing signatures; does not by itself clear a promoted
object's stale compressed copy), neither of which is Flatten-specific.

**This does not change §7.1's recommendation** — Flatten still ships as a
tooltip-only, no-modal action, because the DEFAULT save path remains
incremental and the danger being weighed is about the default, not about
what is theoretically possible. What it changes is the **residual's
status**: where `pass-7-form-fill.md` had to say "not buildable yet, name
it and move on," this document can say "buildable now, at a real but
knowable cost (reuse `to_full_bytes` the way `redact_apply.rs` does),
worth a fast-follow decision rather than an indefinitely-deferred one."
Recommend the flatten tooltip's wording change from *"pdfce does not yet
offer a way to force that"* to something that does not overstate the
remaining gap — e.g. *"If you need the fields' data truly gone, use
Flatten together with a full-rewrite save (not yet exposed as an operator
option here)"* — and flag to the engineer in §9 whether a small "Flatten,
then save as a full rewrite" affordance is worth building this Pass or the
next one. **Not decided here** — this is a scheduling call, not a UI-design
one, per this agent's own charter.

---

## 8. Read-only / required / permissions — summary (detail in §3.1)

Per §3.1: a read-only field, a certification-refused field, a signature
field, and (new finding) a rich-text field all render as a **disabled row
with a named tooltip**, never a hidden one and never a silently-inert
control — the R83/R145 convention this codebase already applies
everywhere else ("a control that vanishes teaches nothing; a disabled one
with a tooltip teaches what would enable it" / "a silent refusal reads as a
broken control"). `Required` gets a P1 text marker, never color alone.

---

## 9. Items for the engineer — flagged, not decided here

1. **`ObjId → page index` lookup** for `widget.page` (§3.1's page suffix
   and §2.2's `GoToPage`/highlight). Small, but a real implementation task
   — a linear scan over `doc.pages` per lookup is almost certainly fine at
   real-world form sizes; a cached map is the engineer's call if profiling
   says otherwise.
2. **Whether `pdfce-core::fill_text_field` should itself refuse a
   `RichText`-flagged text field** (§3.1), as a defense-in-depth guard so
   `pdfce-cli fill-field` gets the same protection this panel's disabled
   row gives the GUI. Not required for this Pass's GUI acceptance criteria
   (the GUI-side disable is sufficient on its own), but worth a decision
   before `pdfce-cli`'s own forms surface is next touched, so the gap does
   not sit open indefinitely on the CLI side only.
3. **`lost_focus()` reliability across Ctrl+S / pane-subject-switch**
   (§4.2) — verify against the pinned egui version before relying on it as
   the sole commit trigger; build the explicit flush-on-save/flush-on-
   switch fallback if the guarantee does not hold.
4. **Flatten + full-rewrite fast-follow** (§7.2) — a scheduling decision,
   not a this-document decision: is closing R48's residual (now cheaper
   than it was) worth doing in this Pass or the next.
5. **P2 canvas click-to-edit** (§2.2) — its own future Pass, not a stretch
   goal for this one; flagged so it is scheduled deliberately rather than
   informally expected to "just get added later" to what ships here.
6. **Naming**: `forms_panel_button()`'s exact ribbon-button label ("Fields"
   vs. "Fill Form" vs. "Form Fields") and the multi-select choice field's
   "Apply" button's exact label — cosmetic, left to the engineer/`ui_text.rs`
   catalog author.

---

## 10. `ui_text.rs` catalog — new entries this document adds

Reuses `pass-7-form-fill.md` §9's catalog for every string that document
already named (`form_document_summary`, `form_js_computed_fields_note`,
`regenerate_appearances_button/tooltip/succeeded`,
`form_field_js_computed_tooltip`, `form_field_no_appearance_note`,
`form_field_certification_disabled_tooltip`, `pushbutton_tooltip/
_clicked_note`, `text_field_multiline_hint`, `flatten_button/tooltip/
_succeeded`, `tool_export/import_form_data_label`,
`import_form_data_succeeded`) — do not re-litigate or rename these; carry
them forward verbatim into the panel-based implementation.

**New this document**, under a new `// Forms panel (this Pass)` header:

- `forms_pane_title()` — *"Fill Form Fields"*.
- `forms_panel_intro()` — one sentence, what the pane is for.
- `forms_panel_no_document_hint()` / `forms_panel_not_a_form_hint()` —
  R83-style empty states (mirrors `redact_panel_no_document_hint()`).
- `forms_panel_button()` — the ribbon button label (§9 item 6).
- `open_forms_panel_button()` — the status-bar adjacency button (§1.5).
- `ribbon_group_forms()` — the new `RibbonGroup::Forms` caption.
- `form_field_row_tooltip(fqn: &str) -> String` — always states the raw
  fully-qualified name (§3.1).
- `form_field_readonly_tooltip()`.
- `form_field_signature_not_supported_note()`.
- `form_field_rich_text_not_supported_note()` — the new §3.1 disclosure.
- `form_field_password_tooltip()` — the new §3.2 masking-is-not-encryption
  disclosure.
- `form_field_required_marker()` — the P1 text marker (§3.1/§8).
- `form_multiselect_apply_button()` — §3.5.
- `form_field_max_len_caption(len: usize, max: i64) -> String` — §3.2.

Every one of these is a new `pub fn` in `ui_text.rs`'s existing catalog
discipline (R1) — no operator-visible string is written inline in
`main.rs`, per the standing rule and `tools/check-ui-strings.sh`.

---

## 11. Priority table (consolidated)

| Item | Priority |
|---|---|
| `PaneSubject::Forms` + `RibbonGroup::Forms` on `RibbonTab::Edit` | **P0** |
| List-driven field panel, file-order, state→action→detail layout | **P0** |
| Text fill (single-line + multiline + `/MaxLen` + password mask) | **P0** |
| Checkbox toggle + honest no-appearance disclosure | **P0** |
| Pushbutton recognize-and-disclose | **P0** |
| The three mandatory disclosures: document-is-a-form, JS-computed, rich-text-unsupported | **P0** |
| Read-only / certification-refused / signature-field disabled rows | **P0** |
| `form_field_commit`, commit-on-`lost_focus()` | **P0** |
| Status-bar `open_forms_panel_button()` adjacency | **P0** |
| Radio-group rows | **P1** |
| Choice fields (combo, list, editable combo, multi-select) | **P1** |
| Flatten | **P1** |
| Export/Import form data (Batch Tools) | **P1** |
| Regenerate appearances | **P1** |
| Row page-jump + passive canvas highlight | **P1** |
| `(required)` marker | **P1** |
| Per-page collapsible grouping | **P1** |
| Comb cell-divider live rendering | **P2** |
| True click-on-canvas-to-edit | **P2 — separate future Pass** |
| `/Tabs` S/R/C computed ordering | **P2** |
| Rich-text field editing (currently refused+disclosed) | **P2** |
| Flatten + full-rewrite affordance | **P2 — scheduling call, §7.2/§9** |

**If cut further: ship P0 alone.** Text + checkbox fill, all three
disclosures, disabled-not-hidden rows for everything else, reached from a
real ribbon entry point — a complete, honest first release, matching the
same "ship P0 alone" fallback both `pass-6.1` and `pass-7-form-fill.md`
anticipated for themselves.
