# UI Spec — Tabbed dock, layer/object tree, selection legibility, and Measure ▾ discoverability

> ## ⚠ STATUS NOTICE — READ BEFORE IMPLEMENTING (added 2026-08-02, engineer)
>
> **This spec is PARTIALLY SUPERSEDED, and the Pass numbering in its
> filename is stale.**
>
> 1. **§A (the dock shell) is superseded TWICE OVER. Do not build it.**
>    §A specifies a horizontal tab strip. `docs/decisions/017-tabbed-dockable-panel-system.md`
>    first replaced that with a two-compartment vertical row list (the
>    320pt dock cannot fit 10+ horizontal tab labels), and then
>    **Amendment A** to that record replaced *it* again: the operator
>    chose `egui_tiles` on 2026-08-02 — *"Use egui_tiles… has the
>    flexibal docking that works as well as inkscape's"* — firing the
>    decision's own pre-armed adoption trigger. **Build the dock from
>    decision 017 + Amendment A, not from §A here.**
>
>    The one requirement to carry across intact: **Layers and Properties
>    must be visible SIMULTANEOUSLY** (in a vector editor you select in
>    the tree and edit properties without losing sight of the tree).
>    Under `egui_tiles` that is a default vertical split, and it ships in
>    the default layout — not something the operator must discover by
>    dragging a panel out.
>
> 2. **§§B and C are UNAFFECTED and remain the build spec** for the
>    object/layer tree and canvas selection legibility respectively.
>    Note that §C's highest-leverage item — that a selection with no
>    visible counterpart must be explained — was independently confirmed
>    as a real shipped bug and partially addressed in commit `c998521`
>    (the object-edit tool drew *no* selection outline at all, because
>    the drawing lived in an unreachable branch). §C's remaining asks
>    (type badge, invisible/approximate-hit disclosure, status readout)
>    are still owed.
>
> 3. **§D (Measure ▾ discoverability) is CONFIRMED and made more urgent
>    by an observation this spec could not have had:** screenshots of the
>    running app proved the toolbar **silently clips** at the default
>    window size — `Obj`, `Measure ▾`, `Tools` and print were entirely
>    off-screen with no affordance showing anything was missing. The
>    operator's "the dimensioning tool didn't seem to have a way to
>    actually set the dimensions" was, in part, literally that the
>    control was not on screen. Overflow handling is being addressed with
>    the icon-set work.
>
> 4. **Pass numbering:** the librarian assigned live-edit rendering
>    (decision 018) to Pass 17.x, so this document's dock/tree family is
>    **Pass 18.x**. The filename was left unchanged to avoid breaking
>    inbound references; trust this notice over the filename.

> Authored by `pdfce-ui-specialist`, on dispatch from the engineer, in
> direct response to operator (Ken) GUI-usability feedback after running
> the current build (2026-08-02), quoted in full at the top of the
> dispatch and not re-quoted here. The engineer implements this
> verbatim, deviating only with a recorded reason (the standing Pass
> 3.2/6.1/7/8/12.0/14.3/15.2/16.2/12.M2/icon-set spec convention).
>
> Read before implementing: `crates/pdfce-gui/src/object_provider.rs`
> and `crates/pdfce-gui/src/main.rs` (`tools_dock`, `canvas`,
> `run_vector_edit_tool`, `run_measure_tool`, `properties_window`,
> `status_bar_body`) **in full, as actually shipped** — §0 below records
> exactly what exists vs. what this spec adds, confirmed by reading the
> code on 2026-08-02, not assumed from the operator's description alone;
> `crates/pdfce-core/src/vector/decompose.rs` (`PathObject`,
> `TextObject`, `ImageObject`, `VectorObject`) for what the object model
> can and cannot currently describe; `docs/ui_specs/icon-set-and-
> toolbar.md` (the still-unshipped icon pipeline this spec's tab strip
> and type-badges compose with, never duplicate); `docs/ui_specs/
> pass-12.M2-dimension-tools.md` (the Measure ▾ tool this spec's §D
> fixes discoverability for — its interaction design is NOT relitigated
> here, confirmed already implemented by reading `run_measure_tool`/
> `scale_entry_widget`, §0.4).

---

## 0. Grounding — what the operator's four complaints actually are, confirmed by reading the code

The operator's report names four symptoms. Each has a distinct, confirmed
root cause; conflating them would misdiagnose the fix. This section
states each cause precisely so §A–§D fix the right thing.

### 0.1 "I don't seem to be able to click on objects" — a real, zoom-inverted tolerance bug

`ObjectModelProvider::hit_test` (`object_provider.rs:219`) resolves a
click via:

```rust
let pdf = self.canvas_to_pdf(point)?;
let idx = hit_test_point(&self.objects, pdf, SELECT_TOLERANCE)?;
```

where `SELECT_TOLERANCE: f64 = 3.0` (`object_provider.rs:60`) is a
**fixed CANVAS-space** value — and canvas space, per this same file's
own module docs, is "the page's device space at zoom 1.0," i.e.
distance-preserving with PDF page points. The `point` passed in is
already the result of `viewer::screen_to_page(screen_pos, image_rect,
extent, zoom)` (`main.rs:5305`, and identically at the vector-edit-tool
call site, `main.rs:7755`), which — per `canvas.rs`'s own documented
`screen_to_page_distance_scales_as_one_over_zoom` law — divides a
screen-space distance by `zoom`. The composition means: **a fixed
3.0-point catch radius on the page corresponds to an on-screen catch
radius of `3.0 × zoom` pixels.** At "Fit page" on a typical letter-size
sheet (`zoom` often well under 1.0, frequently 0.3–0.6), the grab radius
is under 2 screen pixels — smaller than the pointer hot-spot itself.
Zooming OUT (which is exactly what an operator does to see a whole page
before clicking something on it) makes clicking *harder*, the opposite
of every other viewer's behavior. **This is the primary cause of "I
click and nothing happens."** It affects both the plain-selection path
(`main.rs:5307-5309`) and the Vector-Edit ("edit objects") tool
(`main.rs:7757-7760`) identically, since both call the same
`ObjectModelProvider::hit_test`.

**The fix vector already exists in this codebase and must be reused, not
reinvented.** `canvas.rs:715/732` already defines exactly this problem's
solution for the (already-shipped, not-yet-consumer-wired) snap engine:

```rust
pub const SNAP_SCREEN_TOLERANCE_PX: f32 = 10.0;
pub fn screen_tolerance_to_page(screen_px: f32, zoom: f32) -> f64 {
    if zoom.is_finite() && zoom > 0.0 && screen_px.is_finite() && screen_px >= 0.0 {
        f64::from(screen_px) / f64::from(zoom)
    } else {
        0.0
    }
}
```

`run_vector_edit_tool` (`main.rs:7742`) already calls this exact
function for its own node-drag SNAP tolerance — it is proven, tested
(`viewer`'s own zoom-invariance test), and in active use one call-site
away from the bug. **Binding fix:** `ObjectModelProvider::hit_test`/
`hit_test_rect` must resolve their tolerance the same way — a fixed
SCREEN-space catch radius (reuse `SNAP_SCREEN_TOLERANCE_PX` or a
sibling `SELECT_SCREEN_TOLERANCE_PX` constant, engineer's call whether
selection and snap should share literally the same constant or a
separate but equal one) converted via `screen_tolerance_to_page` at
query time. This requires `zoom` to reach `hit_test`/`hit_test_rect`,
which the `CanvasTargetProvider` trait signature does not currently
carry (`canvas.rs:399-419` takes only `page_index`/`point`/`rect`) —
**the exact shape of that plumbing (widen the trait vs. give
`ObjectModelProvider` a per-frame-refreshed tolerance field set by its
caller before each query) is the engineer's call**, per this project's
standing "name the contract, not the struct" convention; the constraint
this spec makes binding is only the OUTCOME — **on-screen catch radius
must be zoom-invariant**, matching the snap engine's already-proven
behavior, not a new invention.

### 0.2 "A box highlighting that doesn't correspond to anything" — a legibility gap, not a hit-testing bug

The selection outline (`main.rs:5370-5388`) is one uniform 2px accent
rect drawn around `selection_outline_bounds` with **zero accompanying
text anywhere in the app** — no status-bar line, no properties panel
(the existing "Properties" window is *document*-level Info-dict fields
only, `properties_window`, `main.rs:4352` — it has no concept of a
canvas selection at all, confirmed by reading it in full: it edits
`doc.properties_draft`, never `doc.canvas_selection`). So a successful
hit currently has **no way to tell the operator what it hit.** Combined
with §0.1's tolerance bug and `TextObject`'s bbox-only, origin-inflated
approximation (`decompose.rs:305-327`, `approximate: bool` always
`true` — the box is deliberately wider than the visible glyphs), the
most likely real explanation for "a box that doesn't correspond to
anything" is: **the operator hit a text object's inflated bounding box
in the whitespace around/above the actual glyphs, and the app never
said so.** §C is the direct fix; it requires zero core changes for the
Text case specifically, since `TextObject.approximate` already exists
and is already `true` — the fact was always computable, just never
surfaced.

### 0.3 "The Tools dock should have tabs, including a layer tree I can click"

Confirmed: `tools_dock` (`main.rs:2637`) is a flat vertical list —
Merge/Split/Insert/FontFolders as `selectable_label` rows switching
`self.tools_selected`, no tab affordance, and **no object/layer
browsing surface exists anywhere in the app.** §A/§B design this from
scratch onto the existing `egui::Panel::right("tools")` mount point
(`main.rs:3529`).

### 0.4 "The dimensioning tool didn't seem to have a way to actually set the dimensions"

**Confirmed NOT a missing-feature gap.** Reading `run_measure_tool`
(`main.rs:7874`) and `scale_entry_widget` (`main.rs:8325`) in full: the
`Measure ▾` menu (`main.rs:4131`), the two-point pick/live-preview/
status-strip machinery, and the full scale-entry sub-panel (real-length
+ ratio paths, paper-unit-basis caption, live preview, group picker) are
**all already implemented**, matching `pass-12.M2-dimension-tools.md`'s
spec closely. The gap is exactly what the engineer's dispatch framed it
as: **discoverability/affordance, not capability.** Two concrete,
confirmed contributing causes:

1. `Measure ▾` (`main.rs:4131-4176`) is still **plain text** in a
   toolbar row already crowded with `Markup ▾`/`Text ▾`/`Edit Text`/
   `Add Text` — per `icon-set-and-toolbar.md` §0/§4.3, its icon
   (`icon-ruler.svg`) is a **named, reserved, but not-yet-implemented**
   assignment, blocked on that spec's own still-open §7.1 rendering-
   pipeline fork. An un-iconified menu button in a dense row of
   similarly-styled menu buttons is a real, measured discoverability
   failure independent of anything this spec adds.
2. **Confirmed by reading `main.rs:8182-8184`:** when a dimension is
   drawn on a group with no scale set, the status strip correctly shows
   `pdfce_core::dimension::NO_SCALE_DISCLOSURE` as a **plain
   `ui.label`** — there is no button, link, or affordance next to that
   text to actually go set a scale. An operator who drew a linear
   dimension, saw a page-units number with a disclosure sentence, and
   had no obvious next click, could reasonably conclude "there's no way
   to set the dimension" even though the mechanism is one menu-row away.
   §D's fix is exactly this: turn a passive disclosure into an
   actionable one.

---

## A. Tabbed dock system

### A.1 What ships v1 — three fixed tabs, not a rearrangeable/undockable surface

**Tabs, left to right: `Tools` | `Objects` | `Properties`.**

- **Tools** — the existing `tools_dock` body (Merge/Split/Insert/Font
  Folders), moved verbatim into the new tab shell. No behavior change.
- **Objects** — new: the layer/object tree, §B.
- **Properties** — new: the **selection**-scoped readout/edit surface,
  §C.6. **Deliberately NOT the same panel as the existing
  `properties_window`** (§A.2 names this collision explicitly — it is
  the one real naming trap in this spec).

**Recommendation: fixed tab order, no drag-to-reorder, no undock-to-
floating-window, in v1.** This is the same "honest first cut" call
`icon-set-and-toolbar.md` §7.1 made for the SVG pipeline and
`pass-12.0-canvas-substrate.md` made for marquee-vs-pan: egui is
immediate-mode with no built-in docking widget, so a genuinely
rearrangeable/undockable dock (what the operator's "like any other
modern program" phrase evokes — VS Code, Photoshop, etc.) means either
hand-rolling persistent drag-reorder state and a floating-window-per-
panel escape hatch, or pulling in a dependency (`egui_dock` is the
obvious candidate — **flagged, not decided, per rule 13**: any new
Cargo dependency is an operator/engineer decision, never an agent's to
make solo, same posture `icon-set-and-toolbar.md` §7.1 took for
`resvg`). **The operator's literal, actionable ask — "the Tools dock
should be able to have other tools docked in tabs" — is satisfied by
fixed tabs.** Nothing in the verbatim feedback asks for drag-reordering
or floating-out a tab; that is an inference from "like any other modern
program" this spec declines to make binding. Fixed tabs are the
scoped, buildable v1; drag/undock is named as an explicit backlog item
in §A.4, not silently dropped.

### A.2 The Properties-tab / Properties-window name collision — a MUST-fix, not a nice-to-have

The existing toolbar `Properties` button (`ui_text::properties_button`)
opens a **modeless `egui::Window`** editing the document's `/Info`
dictionary fields (Title, Author, etc.) — nothing to do with a canvas
selection. This spec adds a **second, differently-scoped** thing also
naturally called "Properties" (a selection inspector). Having both
live in the app simultaneously, one a floating window and one a dock
tab, both titled "Properties," is exactly the kind of collision
`pass-16.2` §0 flagged for `text_menu_tooltip()` and treated as a
must-fix, not a style nit — an operator who opens the dock's
Properties tab expecting document metadata (or vice versa) will
reasonably conclude the app is broken or duplicated.

**Binding rename:** the existing floating window's title
(`ui_text::properties_window_title()`) and its toolbar button/tooltip
(`ui_text::properties_button()`/`properties_tooltip()`) become
**"Document Properties"** (title-cased consistently with existing
naming, e.g. matching `group_manager_window_title()`'s own "Dimension
Groups" precedent) — narrowing its name to what it has always actually
scoped to. The new dock tab keeps the plain, shorter **"Properties"**
label, since it is the tab an operator reaches *from* a selection (the
more frequent, contextual use of the word) while the document-level
one is the rarer, deliberately-sought one and can afford the longer,
more specific name. This is a one-string rename plus updating the two
call sites — cheap, and it must ship in the SAME pass as the new tab,
not as a follow-up, or the collision exists for however long the gap
lasts.

### A.3 Tab-strip widget, state, and Tab-chain placement

```
egui::Panel::right("tools").show(ui, |ui| {
    ui.horizontal(|ui| {
        for (tab, label) in [
            (DockTab::Tools, ui_text::dock_tab_tools_label()),
            (DockTab::Objects, ui_text::dock_tab_objects_label()),
            (DockTab::Properties, ui_text::dock_tab_properties_label()),
        ] {
            if ui.selectable_label(self.dock_tab == tab, label).clicked() {
                self.dock_tab = tab;
            }
        }
    });
    ui.separator();
    egui::ScrollArea::vertical().id_salt(match self.dock_tab { .. }).show(ui, |ui| {
        match self.dock_tab {
            DockTab::Tools => self.tools_dock(ui, actions),
            DockTab::Objects => self.objects_dock(ui, actions),
            DockTab::Properties => self.properties_dock(ui, actions),
        }
    });
});
```

- **Widget:** `ui.selectable_label` per tab — reuses the existing
  bold-on-selected `toggle_label` convention (rule 6: selection is a
  shape+weight cue, not colour-only) with **zero new widget kind**
  introduced, matching this project's own "reuse the existing widget,
  new dispatch" precedent (`pass-12.M2` §1.2's own framing for
  `Measure ▾`).
- **State:** `dock_tab: DockTab` on `PdfceApp` (or wherever
  `tools_selected`/`properties_open` already live), **session-only** —
  never persisted to the document, never round-tripped. Defaults to
  `DockTab::Objects` on first open of a session where a document is
  open (the operator's own stated priority — troubleshooting selection
  — should be the tab they land on, not the historically-first "Tools"
  bucket which is comparatively rarely used per-session), and to
  `DockTab::Tools` when no document is open (Objects/Properties have
  nothing to show yet — §A.4's empty-state design still applies rather
  than forcing a tab switch, but defaulting to the tab that already has
  useful content when nothing is open is the honest choice).
- **`ScrollArea::id_salt` must vary by active tab** — three previously-
  separate scroll regions (Tools dock, a potentially-long object tree,
  a properties form) sharing one `id_salt` would cross-contaminate
  scroll position across tab switches, a real, easy-to-miss egui
  immediate-mode footgun this spec flags explicitly so the engineer
  does not inherit today's single `"tools-dock"` salt unchanged.
- **Tab order / Tab-chain:** unchanged in kind from the existing,
  audited convention (`main.rs`'s own module-doc discussion, ~line
  117-120) — the dock is still added before `CentralPanel`, so Tab
  still flows: toolbar → rail (if open) → dock (now: tab strip, then
  the active tab's own widgets, in visual top-to-bottom/left-to-right
  order) → canvas. **No panel-order change**, per that same module
  doc's own "reordering purely for Tab polish would regress the
  permanent status-bar-spans-window property" reasoning — this spec
  does not relitigate that trade, it only adds MORE reachable widgets
  inside the existing dock slot.
- **Tab-switch keyboard access (P1, not P0):** `Ctrl+Alt+1/2/3` (or
  similar — verify unclaimed, same discipline `pass-12.M2` §1.2 used
  for `Ctrl+Shift+D`) to jump tabs directly is a genuine accessibility
  win worth naming, but is not mandatory in v1 per rule 7 (keyboard
  shortcuts are **mandatory for destructive actions**; tab-switching is
  reversible, low-stakes, and already Tab/click-reachable) — recommend
  as a P1 follow-up, not a P0 blocker.

### A.4 Contextual relevance — hide vs. disable vs. empty-state, decided per tab

Per rule 3 (progressive disclosure) and this agent's own "never silently
blank" instinct:

- **Tools tab** — always available regardless of document state
  (Merge/Split/Insert already operate across files, not the open
  document; unchanged from today).
- **Objects tab** — **never hidden or disabled; always shows an
  empty-state message when there's nothing to browse**, matching
  `font_folders_empty_hint()`'s own precedent for "the list is
  legitimately empty, say so" rather than presenting a blank scroll
  area an operator might mistake for a loading/broken state:
  - No document open → `ui_text::objects_dock_no_document_hint()`
    ("Open a document to see its objects.").
  - Document open, current page has zero decomposable content (a blank
    page) → `ui_text::objects_dock_empty_page_hint()` ("This page has
    no selectable objects.").
  - Page failed to decompose (the same failure `ObjectModelProvider::
    build` already declines on, `object_provider.rs:80-94`) →
    `ui_text::objects_dock_decompose_failed_hint()` — an honest "this
    page's content could not be analyzed" rather than an empty list
    that looks identical to a genuinely blank page (fuzzy-never-sneaky:
    a failure state must not be visually indistinguishable from a
    success state that happens to have no content).
- **Properties tab** — same "always show, never blank" rule: with zero
  objects selected, shows `ui_text::properties_dock_empty_hint()`
  ("Click an object on the canvas, or a row in the Objects tab, to see
  its properties here.") — this line does double duty as the tab's own
  discoverability hint for BOTH selection surfaces (§C.6 ties them
  together as one source of truth).

### A.5 The "Tools" toolbar toggle now opens more than Tools — rename it

`ui_text::tools_button()`/`tools_tooltip()` currently name a control
that opens "the Tools dock" — after this spec, that same button opens a
panel whose first-shown tab is the operator's own most-requested
feature (the object tree), not the batch-file tools. **Recommend
renaming the toolbar control to something dock-scoped** rather than
tool-scoped — e.g. `ui_text::dock_toggle_button()` reading "Panels" or
"Inspector" (final word choice is `ui_text.rs`'s own voice/the
engineer's call, this spec only asks that "Tools" stop being the name
of a control that no longer only opens Tools). Its icon assignment
(`icon-set-and-toolbar.md` §2 row 22, currently `icon-tool.svg` reused
from `Tools`) should be revisited in the same follow-up to that spec —
flagged here, not decided, since icon selection is that spec's
territory.

### A.6 Backlog, explicitly not v1

- Drag-to-reorder tabs.
- Undock a tab into its own floating `egui::Window` (would need its own
  focus/Tab-chain design, and duplicates state-ownership questions
  `pass-12.M2`'s Group panel already resolved in the *other* direction
  — keep one canonical location per editable record, never two).
  Flag to the operator explicitly: **is this actually wanted**, or does
  "like any other modern program" mean "has tabs" (satisfied by A.1)
  rather than "supports full panel rearrangement" (a materially larger
  build)? This is an operator-preference question, not a UX-rule
  question — **flagged for the operator, not decided here.**
- A second, LEFT-side dock (the rail already occupies that slot;
  nothing in this spec's ask requires one).

---

## B. Layer / object tree panel (`Objects` tab)

### B.1 Node model — flat paint-order list today, no OCG grouping yet

The current object model (`pdfce_core::vector::PageObjects`, confirmed
by reading `decompose.rs` in full) has **no optional-content-group
(OCG) awareness for general page content** — `VectorObject::{Path,
Text, Image}` carry no `/OC` membership at all. (The dimension-group
OCGs `pass-12.M2` introduces are annotation-layer visibility toggles,
a completely separate mechanism from page-content marked-content OCG
membership — do not conflate the two, and do not present dimension
groups inside this tree; they already have their own home, the Group
panel, per that spec's §5.1 taxonomy reasoning.) **The tree's node
model for v1 is therefore a flat list, not a hierarchy:**

```
Page N
├─ (Objects — flat, paint order, §B.2 for ordering direction)
│   ├─ Path · fill #1a73e8 · 4 nodes
│   ├─ Text · approx. bounds (12pt) — no content captured (§B.3)
│   ├─ Image · XObject, 3 form nested
│   └─ …
├─ Annotations
│   ├─ Highlight · "…" (rect summary)
│   └─ FreeText · …
└─ Form fields              (only if the document has an AcroForm AND
                              this page has widget annotations)
    ├─ "applicant_name" (Text)
    └─ "agree_terms" (Checkbox)
```

Three sibling **sections**, not three tree *levels* of the same kind —
Objects/Annotations/Form-fields are fundamentally different data
sources (`PageObjects` vs. `pdfce_core::annot::Annotation` vs.
`pdfce_core::forms::AcroForm`) and forcing them into one uniform tree
would either lie about a shared hierarchy that doesn't exist or require
inventing one. Each section is its own `egui::CollapsingHeader`
(default open for Objects, default open for Annotations only if
non-empty, default open for Form fields only if non-empty — an empty
default-open header for a page with no annotations is clutter the
"always show, never blank" rule does not require; the SECTION itself
still always appears with a "(none on this page)" trailing label if
empty, so its absence is never ambiguous with "the tree failed to
load").

**Named backlog item, not v1:** if/when `decompose_page` grows
`BDC`/`EMC` optional-content-membership tracking, the Objects section
gains a real middle grouping level (by OCG). This spec does not
require that extension — flagged so a future reader does not conclude
OCG grouping was overlooked rather than deliberately deferred pending a
core capability that doesn't exist yet.

### B.2 Paint order presentation — topmost first

**Recommendation: the tree lists objects in REVERSE paint order — the
last-drawn (topmost, frontmost) object at the TOP of the list.**
Justification, not just convention-following: (1) the topmost object is
the one a plain click is most likely to hit (`hit_test_point`'s own
"nearest/topmost wins" tie-break, per `object_provider.rs`'s existing
behavior) — so the object the operator is most likely to be looking
for after a confusing click is at the top of the list, not scrolled to
the bottom; (2) this is the prevailing convention across the class of
"layers/objects panel" tooling generally (top of list = top of
z-order) — cited here strictly as a **metaphor-level convention**, per
this spec's own constraint, not as a copied GUI structure from any
specific competitor. Each row still shows its paint-order INDEX (e.g. a
small trailing `#42`) so an operator cross-referencing with, say, a
disclosure string that names an index (`TokenRange`/`ByteSpan`-derived
diagnostics elsewhere in the app already use raw indices) is not stuck
translating.

### B.3 Row content — built from what the object model can ACTUALLY describe today, with the gaps named as binding core asks

**This is the load-bearing finding of this section.** The task's own
illustrative row examples ("Text · \"Section A-A\" · Helvetica 10pt",
"Image · 640×480") are **not buildable with today's core data model.**
Reading `decompose.rs` in full:

| Object kind | Fields available TODAY | Row content buildable NOW | Gap |
|---|---|---|---|
| `PathObject` | `style: PaintStyle` (fill rule + stroke bool), `line_width`, `fill_color`/`stroke_color: Rgb`, `subpaths` (→ anchor count via `page_subpaths()`/`anchors()`) | **Full detail row, no core change needed:** `Path · stroke 0.5pt #1A73E8 · 4 nodes` or `Path · fill #FFFFFF (nonzero) · 4 nodes` (a no-paint `n`-op path per `PaintStyle::is_invisible()` gets its own explicit label, §C.2) | none |
| `TextObject` | `page_bbox` (origin-inflated approximation), `approximate: bool` (always `true`) | **Only:** `Text · approx. bounds, Npt tall` — no string, no font name, nothing else exists to show | **Binding core ask, §B.4 #1** — no extracted string preview, no resolved font name/size are captured anywhere in the decomposition today |
| `ImageObject` | `source: ImageSource` (Inline/XObject/Form), `ctm`, `page_bbox` | **Only:** `Image · inline`/`Image · XObject`/`Form XObject · not decomposed` (source kind is honestly disclosable now) | **Binding core ask, §B.4 #2** — no pixel width/height, no colorspace captured |

**This gap is not cosmetic.** The tree's stated, operator-requested
purpose is "at least that way we can troubleshoot better what I am
clicking on" — and Text is *exactly* the object kind most likely to be
the invisible-box culprit (§0.2). A tree row that can only say "Text"
with no distinguishing string is a materially weaker troubleshooting
tool than the operator is asking for, for precisely the case that
matters most. §B.4 names the fix; it does not block shipping v1 with
the honest, lesser row content in the meantime — an accurate "Text ·
approx. bounds" row is still strictly better than the status quo (no
tree at all), and per fuzzy-never-sneaky, an honestly-incomplete label
is always preferable to a fabricated one.

### B.4 Binding core asks (naming the contract, not the struct — per this project's standing convention)

1. **Extend `TextObject` (or decomposition's internal walk state) to
   capture, per text object: (a) a short extracted string preview**
   (the first N decoded characters from the `Tj`/`TJ`/`'`/`"` operands
   already being walked to compute the pen-position bbox — reuse
   whatever string-decoding the codebase already applies elsewhere for
   PDF string literals, e.g. the same discipline `decode_text_string`
   applies to `/Info` fields per the Properties-panel lossy-marking
   code, so encoding edge cases are handled once, not twice), and
   **(b) the resolved font resource name + size** from the `Tf`
   graphics-state value already tracked during the same token walk
   (`decompose.rs`'s existing `GraphicsState`-equivalent machinery
   already has this in scope for `line_width`/colors — surfacing it as
   a new field is exposing already-computed internal state, not new
   parsing work). This is the single highest-value core change this
   spec asks for.
2. **Extend `ImageObject` to capture pixel width/height** (from the
   image XObject's `/Width`/`/Height`, or the inline image's own
   dictionary — both already resolved wherever the decomposition reads
   the image dict to establish the unit-square `page_bbox`) and,
   optionally, a colorspace family name (`DeviceRGB`/`DeviceGray`/
   `Indexed`/etc.) for the detail row. Lower priority than #1 — an
   image is rarely the ambiguous-click case the operator hit, since it
   is visually present by definition (unlike a zero-alpha or
   whitespace-bbox text hit).

Neither ask requires `pdfce-core`/`pdfce-render` to gain a GUI
dependency (rule 2) — both are pure additions to an existing,
GUI-free data structure.

### B.5 Bidirectional selection sync

- **Row click → canvas:** `doc.canvas_selection = canvas::
  selection_after_click(&doc.canvas_selection, Some(target_id), shift)`
  — the **exact same selection-mutation function** the canvas click
  path already calls (`main.rs:5311`), so tree-driven and canvas-driven
  selection are provably the same operation, never a second,
  divergent code path. Plain click replaces; **Shift+click toggles
  add/remove** — deliberately mirroring the canvas's own Shift
  convention (`main.rs:5306`) rather than inventing a "range select"
  behavior foreign to the rest of the app. No new modifier convention
  is introduced.
- **Row click → viewport:** if the target object's bounds
  (`ObjectModelProvider::bounds`) fall outside the currently-visible
  canvas viewport at the current zoom/scroll position, scroll/pan the
  canvas so the object is visible (egui's `Response::scroll_to_me` /
  `Ui::scroll_to_rect` is the mechanism — a genuine new wiring point,
  since the canvas viewport today only changes via zoom controls or
  drag-pan, never programmatically; the engineer's call how the canvas
  area's own scroll offset is exposed for this). This is what makes the
  tree an actual troubleshooting instrument rather than a second,
  disconnected list — clicking a row must always make the object
  findable on screen, not just selected out of view.
- **Canvas selection → row:** when `doc.canvas_selection` changes via
  ANY path (plain click, marquee, Vector-Edit tool, a future
  find/search result), the Objects tab — if visible — highlights the
  corresponding row(s) using the SAME `toggle_label` bold+background
  convention every other selected-state control in this app already
  uses, and scrolls the tree's own `ScrollArea` so at least the first
  selected row is visible (`ui.scroll_to_me` inside the tree's row
  loop, gated on "this row's `TargetId` is newly in `canvas_selection`
  this frame" to avoid fighting the operator's own manual tree
  scrolling on every unrelated frame).
- **Multi-select:** N objects selected on canvas (marquee, or repeated
  Shift-click) highlights N rows simultaneously — no additional design
  needed beyond "every row independently checks membership in
  `canvas_selection`," which is already how `selection_outline_bounds`
  works today (`canvas.rs`, consumed at `main.rs:5370`).

### B.6 Large-page behavior — virtualize, never silently truncate

A complex vector drawing can legitimately decompose to tens of
thousands of `VectorObject`s. Two distinct risks, addressed separately:

1. **Per-frame rendering cost of the list itself.** `ScrollArea`'s
   default vertical layout lays out every child every frame regardless
   of visibility — thousands of `selectable_label` calls per frame will
   visibly stall. **Binding: use egui's row-virtualizing API
   (`ScrollArea::show_rows`, fixed row height) rather than a naive
   child loop** — only the rows actually scrolled into view are laid
   out per frame, the standard fix for exactly this class of problem
   and already idiomatic egui, not a new dependency.
2. **Per-frame cost of computing each row's LABEL STRING.** If the
   `Path · stroke 0.5pt #1A73E8 · 4 nodes`-style detail string is
   recomputed from `PathObject` fields inside the render loop every
   frame for every visible row, that is redundant, cheap-but-not-free
   work repeated 60 times a second for no reason. **Recommend:**
   precompute every visible page's row-label strings ONCE, at the same
   moment `ensure_object_provider` (`main.rs:1460`) rebuilds the
   `ObjectModelProvider` for a page change/edit — piggyback the
   existing single-rebuild-per-page-per-edit discipline rather than
   introducing a second, independent cache with its own invalidation
   rules.
3. **The genuinely pathological case (still, after virtualization, the
   list is too large to be USEFULLY browsed — tens of thousands of
   rows a human will never scroll through) gets an honest, DISCLOSED
   cap, never a silent one**, per the project's standing no-silent-
   caps rule: e.g. "Showing the first 2,000 of 47,318 objects on this
   page — use the canvas to click directly, or (future) the Search
   tool, to reach the rest." **This is a fallback for an extreme case,
   not the normal-case design** — normal-case is full virtualization
   with no cap at all, since `show_rows` makes rendering 47,000 rows
   cheap; the cap exists only if some OTHER cost (row-string
   precompute time, memory) proves it necessary at implementation time,
   and if so it must be visible text, never a quietly-shortened list
   the operator has no way to know is incomplete.

### B.7 Search/filter — named, deferred

A filter box at the top of the Objects tab (by object kind, or — once
§B.4 #1 lands — by text content) is a natural, high-value pairing with
this tree and with the reserved `icon-search.svg` assignment
(`icon-set-and-toolbar.md` §8.3). **Explicitly out of scope for this
spec** — flagged as a clean, obvious P1 follow-up so it is not
scope-crept into this already-large Pass, not because it lacks value.

---

## C. Selection feedback on the canvas

### C.1 Per-kind highlight treatment

Today: one uniform 2px accent-color stroke rect for every object kind
(`main.rs:5370-5388`), regardless of whether the hit was a Path, Text,
Image, or (once wired) an annotation. **Keep the outline as the base
treatment — it is cheap, already correct, and already satisfies rule 6
(a real 2px boundary shape, not a colour-only tint).** Add: a small
type-badge glyph drawn at the outline's top-left corner —
`P`/`T`/`I`/`F`/`A` (Path/Text/Image/Form/Annotation) as a plain text
badge in a small filled circle using the SAME accent colour as the
outline, until the icon pipeline (`icon-set-and-toolbar.md` §7.1)
lands, at which point it becomes a tiny rendered icon instead of a
letter — **this spec's badge design must not block on that pipeline
decision**, a text-badge fallback is the honest interim, not a second
thing to redesign later (the badge's POSITION/existence is the durable
part of this design; its exact glyph rendering swaps the same way
every toolbar icon eventually will, §5.3/§6 of the icon spec's own
theming plan applies unchanged here). This directly answers "what did
I just select" at the exact moment of confusion, before the operator
even reads the status readout.

### C.2 Disambiguating a hit on an invisible object — the direct fix for the operator's literal complaint

**The selection outline must NEVER be suppressed just because the
object paints nothing or is visually indistinguishable from its
background** — suppressing it would make the mystery WORSE (a click
that "does nothing at all" is less debuggable than a click that shows
a box with an explanation). Instead, the **Properties tab (§C.6)
becomes the mandatory disclosure surface**, always populated on any
non-empty selection:

- **A no-paint path** (`PaintStyle::is_invisible()`, `decompose.rs:184`
  — an `n`-op clip/no-op path, already a real, cheaply-detectable fact)
  gets an explicit, plain-language line: *"Selected: Path object,
  paints nothing (a clip or no-op path) — 42 × 18pt."* This is the
  single most common cause of a "box over nothing," directly named.
- **A Text object** (always `approximate: true` today) gets: *"Selected:
  Text object — the box shown is an approximate bounding box (based on
  glyph start positions), not the exact visible text extent. pdfce
  does not yet measure exact glyph outlines."* **This is buildable
  TODAY with zero core changes** (`TextObject.approximate` already
  exists and is already always `true`) and is, by a wide margin, **the
  single cheapest, highest-leverage fix in this entire spec** for the
  operator's literal, verbatim complaint — recommend the engineer
  prioritize this one line even ahead of the tolerance fix in §0.1, if
  forced to sequence, since it turns an already-somewhat-fixed click
  into an EXPLAINED one.
- **A same-colour fill/stroke** (e.g. white-on-white) is **not
  reliably detectable** from `PathObject`'s own fields alone (the
  underlying page background may be a separate filled shape, an image,
  or the blank page canvas — there is no single "background colour" to
  diff against). **Named as a real, honest limit, not silently
  papered over:** this spec does not ask for a same-colour heuristic;
  the outline + type badge + fill/stroke colour readout in Properties
  (§C.6, which DOES show the object's own fill colour verbatim, e.g.
  "fill #FFFFFF") already lets an operator work out "oh, it's the same
  colour as the page" themselves from the disclosed facts, without the
  app needing to guess at visual contrast.

### C.3 Overlapping-object cycling

`hit_test_point`'s existing "topmost/nearest wins" behavior means a
tight stack of overlapping objects is only ever reachable one way today
— whichever one is on top. **Binding ask, naming the same shape as
Pass 12.M1's own "return the full tied candidate list, not just the
winner" precedent (`pass-12.M2` §2.4):** a sibling hit-test query that
returns every object within tolerance at a point, sorted nearest/
topmost-first, so **Alt+click** (chosen to avoid colliding with the
canvas's existing Shift-for-additive-selection convention) steps to the
next-topmost object at the same screen point on each repeated Alt+click.
This mirrors the snap engine's Tab-cycle UX (`pass-12.M2` §2.4) closely
enough that an operator who has learned one learns the other for free.
**Exact core/GUI split (a new `hit_test_point`-sibling in
`pdfce_core::vector::hit` vs. a `pdfce-gui`-side re-sort of an existing
`hit_test_rect`-style call) is the engineer's call**, per this
project's "name the contract" convention.

### C.4 Annotation and form-field selection

Annotations (`pdfce_core::annot::Annotation`, which already has
`subtype_label()`) and form-field widgets are not currently part of
`CanvasTargetProvider`'s hit-testable set at all (confirmed:
`ObjectModelProvider` only wraps `PageObjects`, which is page-content-
stream objects only). **This spec does not require wiring annotation/
form-field canvas hit-testing** — that is a materially larger scope
item (annotations already have their own selection/interaction paths
for the markup and form-fill tools, per `pass-6.1`/`pass-7`) and is
named here only so the Objects tree's Annotations/Form-fields SECTIONS
(§B.1) are understood as **browsable, click-to-select-and-reveal**
entries (reusing `doc.canvas_selection` is still correct for them, if
the engineer extends `TargetId`'s encoding to also address annotations
— the same "extend the opaque ID, don't widen the trait" pattern
`pass-12.M2` used for dimension annotations, §2.4's chained-snapping
ask), **without this spec asserting they already have full on-canvas
click-to-select parity with page-content objects.** Flagged as an
open scope boundary, not silently assumed solved.

### C.5 Status-bar readout vs. Properties tab — one, not two

Per rule 4 (status bar is the narrator) there is a real tension: should
the "what did I select" readout live in the persistent status bar
(`status_bar_body`, always visible) or the Properties tab (only visible
when the dock is open and that tab is active)? **Recommendation: the
Properties tab is the primary, detailed surface (§C.6); the status bar
gets ONE short summary line, not the full detail**, e.g. `"Selected: 1
object (Path)"` / `"Selected: 3 objects (2 Path, 1 Text)"`, appended to
`status_bar_body`'s existing narrator channel (`main.rs:4537` region)
alongside the edit-note/save-result/redaction-pending lines already
living there. This satisfies rule 4's "always narrated" requirement
even when the dock is closed (the operator's very first troubleshooting
session may not have the dock open at all) while keeping the FULL
disambiguating detail (§C.2's invisible-object explanations, fill/
stroke colours, node counts) in the Properties tab where there is
actual room for it — a status-bar line has no room for a full
sentence-length disclosure without crowding out the other narrator
lines already documented as non-suppressible (`status_bar_body`'s own
doc comment: "None of these lines may be suppressed").

### C.6 The Properties tab — the selection readout, built from the SAME facts as §B.3's row detail

**Single source of truth:** the row-detail string logic designed in
§B.3 and the Properties-tab body described here must be the SAME
function/data path, not two independently-maintained descriptions of
the same object — a `describe_object(obj: &VectorObject) -> ObjectSummary`
(or similar; exact shape is the engineer's call) computed once and
consumed by both the tree row label and the Properties tab, so a Path's
fill colour is never described one way in the tree and a different way
in Properties.

- **0 selected:** `properties_dock_empty_hint()` (§A.4).
- **1 selected:** kind + full detail (§B.3's row content, at FULL
  length here rather than the tree's necessarily-truncated one-line
  form) + page-space bounds (`x, y, w × h` in PDF points) +, for a
  Path, the fill/stroke/line-width facts verbatim + (§C.2's) any
  invisible-object/approximation disclosure that applies.
- **N > 1 selected:** a summary line — `"3 objects selected (2 Path, 1
  Text)"` — per rule 3, this spec does not ask for a per-object
  breakdown of a multi-select in v1 (that is what the tree's own
  highlighted rows already show); Properties' job for a multi-select
  is orientation, not exhaustive detail.
- Real, Tab-focusable widgets throughout (no painter-drawn text
  standing in for something an operator might want to copy/read via
  assistive tech) — matching every prior tool spec's accessibility
  discipline.

---

## D. Dimension-tool ("Measure ▾") affordance fix

Per §0.4: the tool is real and functional. This section fixes
discoverability and mid-flow guidance, not capability.

### D.1 Icon — align with, don't duplicate, the icon-set spec's own reservation

`icon-set-and-toolbar.md` §8.2 already reserves `icon-ruler.svg` for
`Measure ▾` and names the exact sub-icon assignments for Linear/Radius-
Diameter/Set-Group-Scale/Manage-Groups. **This spec does not re-design
that mapping.** It DOES flag, as a scheduling note for the engineer (not
a UX decision): if this Pass ships before the icon-set spec's §7.1
rendering-pipeline fork is resolved, `Measure ▾` will STILL be
text-only, and the discoverability problem this section is meant to fix
will only be partially addressed by this spec's other changes (D.2-D.3)
until that pipeline lands. **Recommend sequencing the icon-set spec's
implementation in the same work window as this Pass if at all
feasible** — the two specs' fixes for the SAME operator complaint are
more effective together than either alone.

### D.2 Verify the menu button and its tooltip are pulling their weight today

Before adding anything new, the engineer should confirm (grep, per this
project's own "verify before assuming" discipline) that
`ui_text::measure_menu_tooltip()` exists and states clearly WHEN to
reach for Measure ▾ vs. Markup ▾ (`pass-12.M2` §9 already named this as
a required disambiguation) and that the dynamic label
(`"Measure: Linear ▾"` etc., §1.2 of that spec) is actually wired so an
operator who activated a measure tool and got distracted can tell at a
glance, from the closed toolbar alone, that a tool is still active.
This spec does not re-design that string; it asks that its presence be
CONFIRMED, since a spec having designed a fix is not the same fact as
the fix having shipped byte-for-byte.

### D.3 The load-bearing fix: turn the "no scale set" disclosure into an actionable one

Confirmed (§0.4, `main.rs:8182-8184`): the no-scale-set line is a bare
`ui.label`. **Binding UI change:** add a real button immediately
beside that disclosure text — `ui_text::set_scale_action_button()`
("Set Scale…") — that, when clicked:

1. If a `MeasureScale` gesture can reuse the CURRENT in-progress
   reference line (the operator already drew a linear dimension; its
   two picked points are a perfectly valid scale-reference line too),
   route directly into the scale-entry sub-panel (§4.2 of
   `pass-12.M2`) seeded with THAT line's drawn length, sparing the
   operator from re-drawing a second reference line for the same
   physical edge they just measured. **This is a genuine UX
   improvement over `pass-12.M2`'s own original design**, which treated
   `MeasureScale` as always starting its own fresh two-point pick — flag
   this refinement to the engineer as a small, additive change to that
   spec's §4.1, not a contradiction of it (the tri-state/two-path
   scale-entry panel itself is unchanged).
2. If reusing the in-progress line is not feasible (engineer's call,
   depending on `MeasureLinearState`'s exact shape), fall back to simply
   switching `active_tool` to `MeasureScale` and opening the Group
   Manager's inline scale-entry form (§5.2) for the CURRENT group,
   pre-selected — still strictly better than today's dead-end label,
   since the operator lands on a form with an obvious next action
   rather than having to independently rediscover the `Measure ▾` menu
   a second time.

This is the concrete instance of rule 1 (fuzzy, never sneaky — every
algorithmic/disclosed fact should be actionable where a fix exists)
applied to a **stuck-state**, not a destructive one: the disclosure
already correctly tells the operator the truth ("no scale set"); what
it was missing is the one-click path to FIX that truth, which this
spec adds without changing the disclosure's own wording or the tri-
state model (`pass-12.M2` §4.3) it depends on.

### D.4 In-progress gesture feedback — already specced and implemented, verify only

`pass-12.M2` §2.1-2.6's live-preview line, fuzzy snap indicator, and
status/disclosure strip are confirmed present in `run_measure_tool`
(§0.4). This spec does not add anything further here — named only so
the engineer does not re-derive or duplicate machinery that already
exists; if a review of the LIVE build surfaces a gap in this specific
area (e.g. the snap glyph legend from `pass-12.M2` §2.2 not rendering),
that is a bug against the existing spec, not new scope for this one.

---

## Discoverability / accessibility checklist run against this spec

- **Plain-English labels, tooltip states WHEN not just WHAT:** every
  new control named above (`dock_tab_*_label()`, `objects_dock_*_hint()`,
  `properties_dock_empty_hint()`, `set_scale_action_button()`) needs a
  `ui_text.rs` entry following the catalog's existing voice — final
  copy is the engineer's/`ui_text.rs`'s call, per every prior spec's
  convention; this spec names the CONTRACT (what fact each string must
  convey), not the exact sentence.
- **Keyboard shortcut mandatory for destructive actions:** nothing in
  this spec is destructive — tab-switching, tree selection, and the
  Set-Scale action button are all reversible, low-friction operations
  (rule 7's "frictionless trivial ones" half applies throughout; no
  new confirmation dialog is introduced anywhere in this spec).
- **Visible current state:** the tab strip's bold-on-selected
  (`toggle_label`) convention, the tree's highlighted-row convention,
  and the type-badge on canvas selections are all shape/weight cues,
  never colour-only (rule 6), consistent throughout.
- **Tab order / reading order:** §A.3 confirms no panel-order change;
  every new widget (tab buttons, tree rows via `show_rows`, Properties
  fields, the Set-Scale button) must be a real, focusable egui widget —
  `ui.selectable_label`/`ui.button`, never a painter-drawn region
  standing in for one, matching every prior spec's accessibility
  discipline in this project.
- **Known egui/accesskit gap, inherited, not widened:** the canvas
  raster itself remains screen-reader-illegible (the standing note,
  `main.rs` ~191-208) — this spec's whole point is to route every fact
  an operator needs through the tree/Properties tab/status line's real
  text widgets INSTEAD of expecting the canvas image to convey it,
  which actively narrows the practical impact of that gap rather than
  widening it, the same posture `pass-12.M2` §7.2 took for its own
  numeric-entry design.
- **Fuzzy-never-sneaky:** the type badge, invisible-object disclosure,
  and text-approximation disclosure (§C.2) are all newly-surfaced FACTS
  about existing, already-computed data (`PaintStyle::is_invisible()`,
  `TextObject.approximate`) — nothing here is a new inference the
  operator must confirm/override; they are disclosures of ground truth
  already known to `pdfce-core`, just never shown before.

---

## Priority table

| Item | Priority | Note |
|---|---|---|
| Zoom-invariant selection tolerance (§0.1) | **P0** | Root cause of "can't click objects"; reuses `screen_tolerance_to_page`, no new mechanism |
| Text-approximation selection disclosure (§C.2) | **P0** | Zero core changes needed; single highest-leverage fix for the operator's literal complaint |
| Invisible-path (`is_invisible`) selection disclosure (§C.2) | **P0** | Zero core changes needed |
| Tab strip + `Tools`/`Objects`/`Properties` shell (§A.1, A.3) | **P0** | The operator's literal, actionable ask |
| `Document Properties` rename (collision fix, §A.2) | **P0** | Must ship in the same pass as the new Properties tab, not after |
| Objects tree: Path rows (full detail), flat paint-order list, empty states (§B.1-B.3, B.6) | **P0** | Buildable today for Path; Text/Image rows ship with honest, lesser detail pending §B.4 |
| Bidirectional selection sync, tree ↔ canvas (§B.5) | **P0** | The core "troubleshoot what I'm clicking" mechanism |
| Status-bar selection summary line (§C.5) | **P0** | Cheap, closes the "no readout at all" gap even with the dock closed |
| Properties tab selection detail (§C.6) | **P0** | Shares the describe-object logic with the tree, §B.3 |
| `TextObject` string-preview + font-name core extension (§B.4 #1) | **P1** | High value, but v1 tree still functions honestly without it |
| `ImageObject` pixel-dimension core extension (§B.4 #2) | **P1** | Lower urgency than #1 — images are rarely the ambiguous-click case |
| Overlapping-object Alt+click cycling (§C.3) | **P1** | Real usability win, not blocking the core troubleshooting loop |
| Set-Scale inline action button (§D.3) | **P0** | Confirmed dead-end today; small, high-leverage fix |
| Measure ▾ icon (§D.1) | **P1 (tracked elsewhere)** | Owned by `icon-set-and-toolbar.md`; sequencing note only |
| Tree row-string precompute / virtualization (§B.6) | **P0 for `show_rows`; P1 for the disclosed-cap fallback** | Fallback only needed if precompute alone doesn't suffice |
| Drag-reorder / undock tabs (§A.6) | **Not scoped — operator decision needed** | Flagged, not built |
| Objects-tab search/filter (§B.7) | **P1 (backlog)** | Natural pairing with reserved Search icon |

---

## Open items for the librarian / operator

1. **The `properties_window` rename (`Document Properties`, §A.2) is a
   MUST-ship-together change**, not an optional cleanup — worth a
   one-line pointer from wherever this Pass's roadmap entry lands so a
   future session does not ship the new Properties tab first and the
   rename as a "later" follow-up, recreating the exact collision this
   spec calls out.
2. **§B.4's two core-model extensions (`TextObject` string/font
   preview; `ImageObject` pixel dimensions) are named, binding asks
   with a clear priority order** (#1 materially more urgent than #2,
   tied directly to the operator's own complaint) — worth flagging to
   whichever engineer session scopes `pdfce-core` work next, since
   these are the two gaps that keep the Objects tree from fully
   answering "what am I clicking on" for Text specifically.
3. **§A.6's drag-reorder/undock question is an explicit operator
   decision, not resolved here** — the verbatim feedback ("like any
   other modern program") is ambiguous between "has tabs" (satisfied)
   and "supports full panel rearrangement" (a materially larger,
   possibly-new-dependency build). Surface this distinction to the
   operator before committing engineering time either way.
4. **§0.1's fix reuses `SNAP_SCREEN_TOLERANCE_PX`/`screen_tolerance_to_
   page`, both already shipped and tested for the (not-yet-consumer-
   wired-for-selection) snap engine** — worth a one-line note wherever
   this finding is filed so a future audit does not mistake the
   selection-tolerance bug for a hit-testing correctness bug (it is a
   units bug, one call-site fix once the trait plumbing is settled),
   nor mistake the fix as requiring new machinery when it does not.
5. **§C.3's overlapping-object cycling and §C.4's annotation/form-field
   canvas hit-testing are both explicitly named as scope boundaries,
   not silently assumed-included** — worth flagging alongside this
   spec's Shipped entry so a future session scoping "why can't I
   Alt+click through a stack of objects" or "why doesn't clicking an
   annotation select it in the tree" does not treat either as an
   unnoticed regression; both were deliberately deferred here.
