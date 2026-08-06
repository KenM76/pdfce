# Shell redesign — persistent panels, density, the page rail, adjacency, and a first Comments surface

**Author:** `pdfce-ui-specialist` · **Date:** 2026-08-06 · **Drives:** five
operator-confirmed UI PROPERTIES (2026-08-06), converted from a PDF-XChange-
Editor resemblance request per **R123** — see the dispatch brief for the
exact property wording and the binding constraint that this document is
designed from pdfce's own organising question, not from having looked at any
competitor's interface. **I have not opened, described, or reasoned from
PDF-XChange Editor anywhere below**, and I flag one place (§3.4) where I
believe the design would genuinely be easier to get right with a reference
screenshot in front of me — and stop there rather than go get one.

**Terminology (`CLAUDE.md` rule 15, binding throughout).** Where this
document discusses dimension objects (§3.5's exclusion of the Measure
surface from the new Comments list), every one is a **ce dimension** — a
`/Line`+`/IT /LineDimension` annotation pdfce itself authors — never a **pdf
dimension**. Nothing here concerns pdf dimensions.

**I do not write code.** Everything below is critique + a concrete change
list for the engineer to implement, push back on, or take back to the
operator where I've flagged a genuine judgment call.

---

## 0. What already exists — read before designing anything new

Confirmed by reading `crates/pdfce-gui/src/{dock.rs, ribbon.rs}` in full,
`crates/pdfce-core/src/{annot.rs, annot_author.rs, edit.rs}`, `docs/
ROADMAP.md`'s Pass 24.1/24.3/34.1/34.2 entries, `docs/FEATURES.md`, and this
agent's own two most recent specs (`tool-options-dock-and-ce-dimension-
properties.md`, `forms-panel.md`) — not assumed from memory.

**The current shell, precisely, as of this session:**

- **Ribbon** (`ribbon.rs`, Pass 24.1): six fixed tabs — File / Edit / Review
  / Measure / Tools / View — each gating a fixed set of `RibbonGroup`
  bands. This is R123-compliant, already shipped, and **this document does
  not touch the tab taxonomy** — none of the five properties are about
  ribbon groupings.
- **Right dock** (`dock.rs`, Pass 24.3): exactly one panel, `DockPanel::
  Objects` — the page's object/layer tree. Deliberately narrowed to one
  thing on the operator's own instruction ("the only thing that should be
  visible in the right side panel is the Objects tree"). **Untouched by
  this document** — none of the five properties ask to touch the right
  dock, and I have no reason to reopen that decision.
- **Left dock** (`dock.rs`, Pass 34.1): a second, independent `egui_tiles::
  Tree`, one tab group, two labels: **`Pages | Tool Options`**. `Pages` is
  the page-thumbnail rail. `Tool Options` hosts `ribbon::PaneSubject` — a
  **five-way content mux**: `ActiveTool` (the armed canvas tool's controls),
  `Properties` (selected ce dimension / ce-dimension groups / document
  `/Info`, Pass 34.2), `BatchTools`, `Redact`, `Forms` (Pass 37.2). Exactly
  one of these five is visible at a time; reaching another replaces
  whichever was showing. Arming a tool auto-raises the `Tool Options` tab
  over `Pages` (`dock::activate`); disarming does not auto-lower it.
- **The load-bearing reason `Tool Options` is docked, not floating**
  (`dock.rs` L180-188, `dock.rs` module docs): the pre-Pass-34.1 shell drew
  three tools' property bars and status strips as floating `egui::Area`s
  pinned to canvas corners. Before decision 024 they were pinned to the
  **page**, so they moved with every zoom/scroll/page change — this is
  *exactly* the operator's own words, quoted verbatim in `dock.rs`: *"a
  separate accept / reject box somewhere on the screen to click — I've
  never seen any other software operate that way."* Pass 34.1 fixed this by
  anchoring the content to the viewport (the dock), not the page. **This is
  the fact property 4's binding constraint exists to protect, and this
  document does not put anything back over the canvas.**
- **Annotation read model** (`annot.rs`): `Annotation` carries `id`,
  `subtype`, `rect`, `flags`, `appearance`, `is_popup`, `oc` — deliberately,
  per its own module doc, **not** a faithful echo of the annotation
  dictionary. It does **not** model `/Contents`, `/T` (author), `/M`
  (mod date), or `/C` (colour). This is a real, load-bearing gap for §3.5,
  found by reading the struct definition directly, not assumed.
- **No general annotation-delete verb exists** (`edit.rs` L3663-3689,
  `EditSession::remove_redaction_mark`'s own doc comment): *"It is
  deliberately **not** a general `delete_annotation`... this command cannot
  become the back door through which a UI deletes an operator's highlights
  or a form's widgets without those features designing their own deletion
  semantics (dangling `/AcroForm` `/Fields`, `/Popup` companions, `/IRT`
  reply chains — none of which a redaction mark has)."* This is the exact
  caution the engineer needs when the Comments panel eventually grows a
  Delete action (§3.5, P1).
- **Annotation authoring today has no note-text UI for geometric shapes.**
  Per `FEATURES.md`'s Annotations row: "Author geometric markup: Ink/
  Square/Circle/Line/Polygon (Pass 6.1) — minimal menu affordance only."
  Confirmed by reading `annot_author.rs`: only the text-bearing family
  (`Text`, `FreeText`) sets `/Contents`; the geometric shapes never do.
  This bounds what §3.5's list can honestly show on day one — flagged there,
  not glossed over.
- **Canvas-side annotation selection does not exist yet.** `FEATURES.md`'s
  *Planned* table: "Pass 22.0 — make ce dimensions and foreign annotations
  selectable, marqueeable, and deletable from the canvas" is unstarted.
  This bounds what a Comments-list row can do (§3.5).
- **The working precedents this redesign reuses rather than reinvents:**
  `redact_panel`'s state→action→detail list shape and per-row
  `Action::GoToPage` navigation (cited directly by `forms-panel.md` §2.2/
  §3, itself cited here); `place_draft_commit`'s three-condition commit
  test; and — most load-bearing for §7 below — **R125's non-emission
  mechanism** (`ROADMAP.md`: "only the ACTIVE tab's band is emitted;
  inactive tabs are not built, laid out, or in the focus chain... egui 0.35
  has no focus groups or tab-index, so non-emission is the only mechanism
  keeping the Tab chain short").

---

## 1. The organising question — unchanged, and why this document doesn't need a new one

R123 requires structure to derive from *what pdfce can do*, not from a
copied taxonomy. The organising question for the ribbon (*"what does this
command act on?"*, `ribbon.rs` module docs) and the rule the whole Pass
24.3/34.1 arc converged on — **"the ribbon picks the activity; the sidebar
holds its controls"** — are both already pdfce's own, already shipped, and
none of the five properties ask to change them. This document is scoped to
**how the sidebar holds its controls** (persistence, density, the rail,
adjacency) and **one new activity the sidebar has never held** (Comments).
Nothing below touches which ribbon tab a command lives under, except adding
one new button to the *already-existing* `RibbonTab::Review` (§3.5) — the
same tab `Markup`/`Notes` already live on, for the same reason.

---

## 2. Property-by-property analysis

### 2.1 Property 1 — persistent panels that change content, not panels you open and close

**The complaint is real, but narrower than "make every `PaneSubject` value
simultaneously visible."** Read literally, that reading is a **bad idea**
(flagged per the brief's explicit request) — `BatchTools`, `Redact`,
`Forms` are genuinely alternate document-wide *workflows*: an operator
running a batch job, reviewing redaction marks, and filling a form are not
things they do in the same eye-movement, and Pass 34.1's own dock.rs
comment makes exactly this argument for `Pages`/`Tool Options`: *"an
operator reads page thumbnails to navigate, and reads tool options while
working the canvas, and they are never doing both in the same second."*
Forcing all five `PaneSubject` values into permanent simultaneous view would
cost real vertical space (fighting property 2, §2.2) for a simultaneity
nobody asked for between, say, Batch Tools and Redact.

**What the complaint correctly diagnoses: `ActiveTool` and `Properties`.**
This is the *exact same relationship* decision 017 §3/Amendment A.3 built
the original right-dock vertical split to solve — *"select an object in
the layer tree and edit its properties without losing sight of the
tree... if Layers and Properties are mutually exclusive tabs, the tool is
worse than the flat list it replaced."* Pass 24.3 retired that split on the
**right** dock because the premise (Objects paired with the document `/Info`
form) had gone stale — but the underlying need (select something, see its
properties, without losing what you're doing) didn't disappear; it moved.
Today it lives one level down: an armed tool's controls (`ActiveTool`) and
a selected object's properties (`Properties`) are exactly that pairing,
now inside `Tool Options`, still mutually exclusive. **This is the one
place property 1 names a genuine, previously-solved-and-then-retired
problem recurring in a new location, and it is where I recommend spending
the structural change.**

**Recommendation:** `ActiveTool` and `Properties` stop being `PaneSubject`
values reached by switching. They become two **always-visible, standalone
dock panes**, stacked vertically. `BatchTools`/`Redact`/`Forms` (plus the
new `Comments`, §2.5) stay switched — but reached through an
**always-visible, in-panel control** rather than exclusively through a
ribbon click, which is itself a real, independent win on property 1 even
without full simultaneity: an operator can see *that* three other
activities exist and pick one without leaving the dock, instead of the
panel silently having no visible trace of what else it could show. See
§4 for the concrete tree.

### 2.2 Property 2 — higher information density

Recommend a **first, structural, no-risk slice** (§6 slice 1): a named
density convention, appended to `UI_PREFERENCES.md` alongside its existing
§6 type scale and §9 component patterns — not invented per-panel. Concretely:

- **Row height.** `egui::Grid::new(id).num_columns(2)` (already
  `UI_PREFERENCES.md` §9's property-row convention) gets an explicit,
  tightened default row spacing — today's grids inherit whatever
  `Style::spacing.item_spacing` happens to be, which was never tuned for a
  *dense property surface* specifically (it's egui's general-purpose
  default, shared with every other spacing decision in the app).
- **One control per row is not a rule to defend.** Several existing panel
  sections (`redact_panel`'s state section, `tool_options_panel`'s per-tool
  header) put one labelled control on its own line where two would fit —
  fine when there's only one, wrong when a numeric field and its unit
  ComboBox (an existing pattern — scale entry, decimal places) get stacked
  instead of paired on one row. Recommend a pass over the *shipped* panels
  (Properties/BatchTools/Redact/Forms/Tool Options) auditing exactly this,
  before any new panel is built — cheaper to fix once, in the token module,
  than to re-derive per new surface.
- **Heading role is doing real work today; keep it, don't shrink it
  further.** `UI_PREFERENCES.md` §6 already sized `Heading` at 17pt
  specifically for panel section titles. Density should come from spacing
  and row layout, not from shrinking text below the 11pt `Caption` role
  already established for secondary lines — shrinking body text to gain
  density is exactly the kind of trade this project's own accessibility
  standing rule (rule 6, plus this agent's charter §6) argues against
  without a name for what's gained.

**This slice is genuinely independent of every other property** — it
touches spacing constants and existing widget calls, not panel shape or
placement. Ship it first (§6) so every later structural slice inherits the
tighter baseline instead of the redesign shipping loose and needing a
second density pass afterward.

### 2.3 Property 3 — the page-thumbnail rail: placement and behaviour

**Two separable asks inside one property, both real.**

**Placement.** Today `Pages` shares a tab group with `Tool Options` — the
exact pairing the operator asked for on 2026-08-05 (*"docked with the page
navigation tab"*, `dock.rs` L176-178). Property 3, filed a session later,
revisits it. I recommend treating this as new evidence superseding the
earlier request, not silently overridden — **flagged explicitly for the
engineer to confirm rather than assumed**: `Pages` becomes its own
standalone stacked compartment (§4), no longer sharing a tab group — or a
mux — with anything. This is consistent with §2.1's finding (`ActiveTool`/
`Properties` needed the same promotion) and gives `Pages` the property the
2026-08-05 request didn't have language for yet: **always visible, never
displaced by arming a tool.**

**Behaviour.** The specific defect this fixes: today, arming ANY tool
auto-raises `Tool Options` over `Pages` (`dock::activate` on the rising
edge `active_tool: None → Some`), so an operator measuring a drawing loses
their page thumbnails the instant they pick up the Measure tool. Under §4's
stacked layout this auto-raise mechanic is **deleted outright**, not
disabled — there is no longer a "raise" to do, because nothing is hidden.
This is a genuine simplification the engineer should register as removed
code, not dead code kept `#[allow(dead_code)]`.

### 2.4 Property 4 — tool options adjacent to the canvas, not floating over it

**Read narrowly (docked, positioned immediately beside the canvas, never
detached into a window or pinned to page coordinates): already satisfied.**
The left dock's physical position — flush against the canvas's left edge,
part of the same window, moving only when the operator resizes the dock —
*is* "adjacent to the canvas." Pass 34.1 exists specifically because the
**floating**, page-pinned version of this content was the operator's
original complaint. Nothing in this document reopens that. §2.1's change
(always-visible instead of tab-switched) makes `Tool Options` **more**
adjacent in the sense that matters — reachable with zero navigation, not
one click away behind `Pages` — without moving it back over the canvas.

**Read maximally (a fly-out or overlay that visually hugs the currently
selected object, closer than a fixed-width dock column) — this is the
reading that would conflict, and I recommend against it, not silently
route around it.** That shape is structurally the floating `egui::Area`
Pass 34.1 deleted, however it's styled — it would still be positioned
relative to something on the canvas, still move on zoom/scroll/pan, and
still risk reproducing the *"a separate accept / reject box somewhere on
the screen"* complaint that shipped its own standing rule (decision 024
§4.4, `CLAUDE.md` rule 4's narrowing) specifically to close.

**This is the one place in this document where I believe I'd design better
with a reference screenshot in front of me, and where I'm stopping instead
of going to get one, per the binding constraint.** I don't know from the
neutral property wording alone which reading the operator meant. **Flagged
plainly for the engineer to confirm which reading was intended before
building §4** — if it's the narrow reading, §4 already answers it in full;
if it's the maximal reading, that is a request to reopen Pass 34.1's fix
and needs to go back to the operator by name, not be inferred from five
words in a property list.

### 2.5 Property 5 — the comment/annotation list

The largest section; see §3 below in full.

---

## 3. The annotation list — design

### 3.1 What it is not, decided by exclusion first

- **Not ce dimensions.** `ribbon.rs`'s own `RibbonTab::Measure` doc comment
  already reasons through this exact question for a sibling case: ce
  dimensions are technically `/Line`+`/IT /LineDimension` annotations, and
  the ribbon *could* have folded them into `Review`, but didn't, because
  measuring is the operator's primary CAD activity and giving it a second,
  duplicate home would be "exactly the two-mental-models duplication
  decision 024 §3.2 already ruled out." The same reasoning applies here by
  direct citation, not re-derivation: ce dimensions keep their existing
  home (`Measure` tab, `Properties` pane, §2.1); this list excludes them.
- **Not `/Widget` annotations.** Form fields have their own first-class
  surface (`forms-panel.md`, shipped Pass 37.2). `annot::Annotation::
  is_widget()` already exists as the exact filter predicate — reuse it,
  don't re-derive a second one.
- **Not `/Popup`.** `annot::Annotation::is_popup` already exists (`annot.rs`
  L235-238, "never page content"). A `/Popup` is a reader-UI window
  attached to a `Text`/`FreeText` annotation, not an independent comment —
  one list row per real annotation, its popup is implementation detail.

### 3.2 What it lists

Every non-widget, non-popup annotation the document carries, in the same
deterministic order `pdfce-cli list-annotations` already uses (page order,
then `/Annots`-array order) — reuse that ordering rule by name rather than
inventing a second one for the GUI.

**Row content, per row:**

- **Subtype label** (`Annotation::subtype_label()`, already exists) —
  "Square markup", "Line markup", "Note" (Text), "Free text", "Stamp".
- **Page number** — the same `ObjId → page index` lookup `forms-panel.md`
  §9 item 1 already names as an owed, small implementation task; reuse the
  same lookup rather than building a second one, since both panels need
  exactly the same mapping.
- **Note/content preview** — the annotation's `/Contents`, truncated, for
  the text-bearing subtypes that carry one. **For the geometric subtypes
  (Ink/Square/Circle/Line/Polygon), an honest caption instead of blank
  space** — `annotation_no_note_caption()`, e.g. *"No note text — this
  markup has no attached comment."* This is a real, named limit, not a
  polish gap: per §0's finding, Pass 6.1's authoring UI never sets
  `/Contents` on a geometric shape, so on real-world documents authored by
  pdfce itself, most rows in this list will show that caption rather than
  reviewer prose, at least until markup authoring grows a note field. Say
  this plainly to the operator before shipping (§5) so the first release
  doesn't read as under-delivered against "comment list."

### 3.3 What a row does

- **Click → navigate + highlight, no hit-testing needed.** Exactly
  `forms-panel.md` §2.2's P1 mechanism, cited directly rather than
  re-derived: push `Action::GoToPage(page_index)` and set a small piece of
  view state (`doc.view.highlighted_annotation: Option<(ObjId, Rect)>` —
  same shape as forms-panel's `highlighted_widget`, a sibling field, not a
  new mechanism), drawn as a 2px outline via the existing `viewer::
  page_to_screen` whenever it's `Some` and the current page matches. **This
  is P0 here, not P1 the way it was in forms-panel** — forms-panel had a
  P0 fill-focused release that P1's highlight was added *alongside*; the
  Comments panel has no equivalent P0 "fill" activity, so navigate+
  highlight IS its core interaction and belongs at P0.
- **No select-on-canvas yet, named honestly.** Making the annotation the
  *selected* object (enabling move/delete/property-edit from the canvas
  itself) needs Pass 22.0 (annotation selection — unstarted per
  `FEATURES.md`). This is the exact same shape of gap `forms-panel.md`
  named for its own P2 click-to-edit — cite the same posture: a real,
  scoped, later Pass, not a silently-missing corner of this one.
- **Delete — P1, blocked on a real new core verb, not GUI-only.**
  `edit.rs` L3663's own doc comment is explicit that no general
  `delete_annotation` exists, and names exactly what a general version must
  handle that `remove_redaction_mark` didn't need to: a `Text` annotation's
  `/Popup` companion (clean it up too, or it orphans), and a guard against
  ever being asked to delete a `Widget` (this list already excludes them by
  construction, §3.1, but the core verb should refuse one anyway as
  defence-in-depth, the same posture `forms-panel.md` §9 item 2 recommends
  for its own core-side rich-text guard). **Per R83, the Delete control
  does not render at all until this verb exists** — this is the "capability
  doesn't exist yet" case, not the "capability exists but this row doesn't
  qualify" case forms-panel's disabled rows handle; R83's rule is
  "disabled-and-explained beats hidden for things that EXIST but do not
  currently apply" — a capability that doesn't exist anywhere in pdfce yet
  gets omitted, not greyed out promising a future click.

### 3.4 The core-model gap this surface actually needs first

Restated from §0 because it is the load-bearing P0 dependency: `annot::
Annotation` does not carry `/Contents`, `/T`, or `/M` today. **Recommend
extending it** with `contents: Option<String>`, `author: Option<String>`,
`mod_date: Option<String>` (raw §7.9.4 date string; a parsed/validated
date type is a separate, later decision if a "sort by most recent" feature
is ever wanted — not needed for P0's page-order list) — all `Option`,
read-only, additive, no writer changes, the same "core decodes and models,
render paints" axis (R26) every other panel in this app already reads
through rather than poking raw dictionaries from `main.rs`. This is
core-touching work, same class as forms-panel's `/TU` finding and the
tool-options-dock spec's tolerance-zero-representation finding — flagged
plainly as new `pdfce-core` work, not GUI-only, and small enough (three
optional fields, populated from keys `page_annotations` already has the
dictionary in hand to read) to ship in the same session as the list panel
itself rather than needing its own separate Pass.

**CLI parity (rule 11).** `pdfce-cli list-annotations` already exists
(Pass 6.0) and already prints one `annot …` line per annotation. Recommend
extending its output line with `contents=<token>` and `author=<token>` once
the core fields exist — cheap, and gives an operator a scriptable way to
grep a document's comments without opening the GUI, which the CLI-parity
rule (rule 11) asks for by default alongside every GUI feature. **P1**, not
blocking the GUI panel.

### 3.5 Where it lives, and the ribbon entry point

Placed as the fourth member of the Activities compartment (§4) —
`PaneSubject::Comments` (renaming the narrowed enum, see §4). Reached from
a **new button in the already-existing `RibbonTab::Review`**, alongside
`Markup`/`Notes` — not a new tab, not a new taxonomy decision. `ribbon.rs`'s
own doc comment for `Review` already states the organising question this
answers without needing to be re-derived: *"What am I adding for someone
else to read?"* — browsing what's already been added is the same question
asked backwards, exactly the move `ribbon.rs` already names for why Undo/
Redo sits on `Edit` ("the same question answered backwards"). New
`RibbonGroup::CommentsList` (name pending `ui_text.rs` authoring), gated to
`RibbonTab::Review`; `RibbonTab::groups()` for `Review` grows from
`[Markup, Notes]` to `[Markup, Notes, CommentsList]` — three ribbon
*groups* under one tab, which is unconstrained (the ≤2 cap in `dock.rs`
is an `egui_tiles` **tab-group** limit; ribbon groups are flat bands within
one tab, a different mechanism entirely, and `RibbonTab::File` already
carries five).

---

## 4. Target layout — concrete widget tree

### 4.1 Proposed `DockPanel` shape (left tree)

```text
LEFT_TREE_ID root: vertical egui_tiles::Container::Linear, 4 children,
no Tabs container anywhere in this tree.

├── DockPanel::Pages           (unchanged content; standalone, not
│                                tab-paired with anything — §2.3)
├── DockPanel::ArmedTool        (was the ActiveTool case of PaneSubject;
│                                renamed because it no longer also has to
│                                explain hosting Properties/Activities —
│                                the empty-state caption from
│                                tool-options-dock-and-ce-dimension-
│                                properties.md §A.3 carries forward
│                                unchanged: "No tool armed — choose Edit
│                                Text, Add Text, a Measure tool, or Edit
│                                Objects...")
├── DockPanel::Properties       (was the Properties case of PaneSubject;
│                                content UNCHANGED from Pass 34.2's
│                                three-section design — selected ce
│                                dimension / ce-dimension groups / document
│                                `/Info` — only its ALWAYS-VISIBLE status
│                                is new)
└── DockPanel::Activities       (new host pane; internal state is the
                                 NARROWED `PaneSubject` — now exactly
                                 {BatchTools, Redact, Forms, Comments},
                                 four values, chosen via an always-visible
                                 in-panel segmented control, not egui_tiles
                                 tabs — see §4.2)
```

**Why no `Tabs` container at all, not even a 2-label one for `Pages`/
something:** with `ActiveTool` and `Properties` promoted out, and `Pages`
promoted out (§2.3), there is nothing left that benefits from being
tab-paired — every remaining relationship between these four is
"always visible together," which `egui_tiles::Container::Linear` expresses
directly with **zero** tab-bar machinery, zero R84 bold-on-active
bookkeeping, and zero exposure to the "no default tab group holds more
than two" invariant (which doesn't apply here — there are no tab groups
to violate it). This is a genuine simplification over today's shape, not
just a rearrangement.

**Right dock: unchanged.** `DockPanel::Objects` alone, exactly as Pass 24.3
left it. None of the five properties touch it.

### 4.2 The Activities pane's internal segmented control

Not `egui_tiles` tabs (that mechanism's ≤2-label cap exists because
`egui_tiles` 0.16.0 answers overflow by **hiding** tabs behind scroll
arrows — a real risk for a *dock* tab bar an operator might resize
narrow). A plain in-panel button row does not carry that risk: it **wraps**
onto a second line in a narrow column rather than hiding anything, so a
four-item segmented control here is not the same hazard a four-label
`egui_tiles::Container::Tabs` would be. Reuses the existing `icon_button`/
`toggle_label` accessible-name and bold-on-selected conventions (this
agent's icon-set-and-toolbar memory) rather than inventing a fourth
selection-state visual language.

```text
DockPanel::Activities body:
├── ui.heading(activities_pane_title())          // "Activities"
├── [segmented control, 4 toggle buttons — R84: bold + outline-ring on
│    selected, never colour alone]
│    [ Batch Tools ]  [ Forms ]  [ Redact ]  [ Comments ]
├── ui.separator()
└── [content of whichever PaneSubject value is selected — UNCHANGED
     bodies from today's batch_tools_panel / redact_panel / forms_panel,
     plus the new comments_panel (§3), each in ITS OWN ScrollArea per the
     existing "no nested ScrollArea" lesson]
```

Reaching an Activities member from the ribbon (`Batch`/`Protect`/`Forms`
buttons, plus the new Comments button, §3.5) sets the narrowed
`PaneSubject` exactly as it does today — the mechanism is unchanged, only
its **destination** changes, from "raise a hidden tab" to "select a
segment inside an already-visible pane." **One real implementation nuance
to flag:** if the operator has collapsed the Activities compartment (§4.3)
down to its header, a ribbon button setting its subject should also
re-expand it — otherwise "click Forms" silently does nothing visible,
which would be exactly the "hidden desync" class of defect this project's
own tests (`a_backgrounded_panel_can_be_brought_forward`) exist to catch
for the old mechanism. Not a pixel-level spec — flagged as a behaviour the
new code needs, left to the engineer to wire.

### 4.3 Per-compartment collapse — the density/simultaneity lever

Each of the four stacked panes gets a small collapse toggle (a chevron) in
its header, reusing `UI_PREFERENCES.md` §9's existing "Panel section
header: Heading role + one plain `ui.separator()`" pattern, with the
chevron prefixed to the heading rather than a new header shape. Collapsed,
a compartment renders **only its header row** — its body is not drawn at
all that frame, not drawn-and-hidden. This is the same **non-emission**
mechanism R125 already uses for inactive ribbon tabs, applied to a new
location for the same two reasons R125 states: it keeps the frame cheap,
and — more importantly here — **it keeps the Tab focus chain short** (§7).

This is the honest answer to the property-2-vs-property-1 tension named in
§5: pdfce cannot make four compartments both maximally dense AND maximally
simultaneous at a fixed window height — something has to give, and the
collapse toggle hands that decision to the operator at runtime rather than
pdfce guessing a fixed compromise. Default state: all four expanded
(matches the "put it all back" bias `ResetScope::default()` already uses
for the layout-reset chooser); `Action::ResetPanelLayout`'s left-panels
scope should reset this state too, once it's built — a small addition to
an existing action, not a new one.

---

## 5. Tensions and bad ideas found — named explicitly, per the brief's request

1. **Property 1, read as "all five `PaneSubject` values simultaneously
   visible," is a bad idea.** §2.1 already argues this at length; restated
   here as the first tension because it's the one most likely to be
   over-read from the property's own short phrasing. `BatchTools`/`Redact`/
   `Forms`/`Comments` are workflows, not properties-of-a-selection; forcing
   simultaneity between them buys nothing and costs real density.
2. **Property 2 (density) and property 1/3 (persistent, always-visible
   panels) are in genuine, structural tension, not a wording accident.**
   Four always-visible compartments cost more fixed vertical space than a
   2-tab mux, full stop — no interpretation of either property removes
   this trade. §4.3's per-compartment collapse is the mitigation, not a
   resolution; the operator should expect to occasionally collapse
   `Pages` or `Activities` on a short window, not see everything maximally
   dense and maximally simultaneous at once. Say this plainly rather than
   implying the redesign solves both fully.
3. **Property 4, read maximally, directly fights the reason Pass 34.1
   exists.** §2.4 already covers this — flagged again here because it's
   the property most likely to get silently "split the difference" if
   nobody names the conflict out loud, which the dispatch brief explicitly
   asked not to happen.
4. **Property 5's honest P0 ceiling is lower than "comment review
   workspace."** No select-on-canvas (blocked on Pass 22.0), and — a
   finding, not merely a scoping choice — most of what pdfce itself can
   currently author into this list (geometric markup) has **no note text
   to show at all** (§0, §3.2). Set this expectation with the operator
   before shipping; a list of mostly-untitled "Square markup, page 3" rows
   could read as a broken feature rather than an honestly-scoped first
   slice if nobody says so first.
5. **A new accessibility cost, not previously present, worth naming on its
   own (not just folded into §7):** promoting four panes to always-visible
   necessarily lengthens the operator's Tab chain through the left dock
   before reaching the canvas, on every frame, for every operator — R125's
   own stated reason for non-emitting inactive ribbon tabs is exactly "keep
   the Tab chain short," and this redesign trades some of that back for
   property 1's win. §4.3's collapse mechanism is the mitigation (a
   collapsed pane is NOT emitted, so it also drops out of the focus
   chain) — but an operator who wants maximum density AND a short tab
   chain has to actively collapse panes to get it; the default (all
   expanded) does not get it for free. Named so it isn't discovered later
   as a surprise regression.

---

## 6. Migration path — shippable slices

**Slice 1 — density convention, no structural change.** Append a `§11
Density` section to `UI_PREFERENCES.md` (row-spacing constant, an audit of
existing one-control-per-row panels worth pairing). Apply it to the
*existing* panels (Properties/BatchTools/Redact/Forms/Tool Options) as they
stand today, before any panel moves. **Buys:** an immediate, low-risk
tightening (property 2, in isolation) and a tighter baseline for every
later slice to inherit, so the redesign doesn't ship loose and need a
second density pass afterward. Zero interaction with dock shape or
`PaneSubject`.

**Slice 2 — the left-dock restructure.** Promote `ActiveTool` → `DockPanel::
ArmedTool` and `Properties` → `DockPanel::Properties`, both standalone,
non-tab-paired panes; promote `Pages` to its own standalone pane; delete
the `dock::activate`-on-tool-arm auto-raise mechanic outright (§2.3); narrow
`PaneSubject` to `{BatchTools, Redact, Forms}` (Comments arrives in slice
4) hosted inside a new `DockPanel::Activities` behind the segmented control
(§4.2); add the per-compartment collapse chevron (§4.3). **Buys:** the
whole of properties 1 and 3, and — with no further code — resolves property
4's narrow reading (§2.4) by construction, since `ArmedTool` is now
*more*, not less, adjacent than it was. **This is the one slice that
directly reverses specific claims in shipped doc comments** — `dock.rs`
L308-335's "Why one group and not a vertical split" reasoning, and its
"nothing here needs simultaneity" argument for `Pages`/`Tool Options`
specifically — and those comments need the same kind of correction this
project already has a convention for (Pass 24.3's own "the pairing was
reasonable and the premise was wrong" framing, reused rather than a bare
deletion). Also touches `FEATURES.md`'s Shell & UX row and several
ce-dimension rows that cite "the Properties dock pane" — still accurate in
substance (the pane still exists, still holds the same three sections),
wording only.

**Slice 3 — property 4 confirmation, folded into slice 2's commit, not a
separate slice.** No new code beyond slice 2's own result. The only
deliverable is the doc-comment correction (§2.4) plus the flagged
confirmation question to the operator (which reading was meant) — resolve
before or immediately after slice 2 ships, since the narrow reading is
already answered by slice 2 and the maximal reading is a refusal that
needs to be said out loud rather than silently avoided.

**Slice 4 — the Comments panel.** The `annot::Annotation` core-model
extension (§3.4, `contents`/`author`/`mod_date`, read-only, additive); the
new `PaneSubject::Comments` (widening the segmented control from three
members to four); the new `RibbonGroup::CommentsList` button on
`RibbonTab::Review`; the list-driven panel itself (§3.2/§3.3) — browse,
page order, honest no-note captions, navigate + highlight (P0, no
hit-testing). **Independently shippable** after slice 2's `Activities`
compartment exists to receive it; could in principle ship *before* slice 2,
temporarily hosted in today's unnarrowed `PaneSubject`, if scheduling needs
that order instead — flagged as a real option, not a requirement to
sequence it exactly as listed.

**Slice 5 (P1, later, no rush) —** the general `delete_annotation` core
verb (§3.3, scoped per `edit.rs` L3663's own named cautions) and the
Delete row action it unlocks; `pdfce-cli list-annotations`'s
`contents=`/`author=` fields (§3.4); real default-height tuning for the
four stacked compartments, done against the running app per this agent's
standing practice of not inventing exact pixel numbers from source alone.

---

## 7. Accessibility — the checklist, walked

- **Tab order matches reading order?** Yes, by construction: the proposed
  top-to-bottom compartment order (`Pages` → `ArmedTool` → `Properties` →
  `Activities`) reads as "where am I → what am I doing → what did I select
  → what workflow am I running," a defensible narrative order, and
  `egui_tiles::Container::Linear`'s own child order drives both visual
  layout and Tab traversal identically — no separate focus-order
  bookkeeping needed.
- **Colour never the sole signal?** The segmented control's selected state
  (§4.2) explicitly specifies bold + outline-ring, reusing the icon-set
  spec's already-named fix for icon-only toggles — not a new exception.
- **The Tab-chain-length cost is real and named, not hidden** (§5 item 5,
  §4.3) — this is the one place this redesign makes accessibility
  *measurably* more expensive by default, and the honest answer is "the
  operator can collapse a pane to shorten it," not "it isn't a cost."
- **Known egui gap, restated rather than re-discovered:** `dock.rs`'s own
  module docs already record that `egui_tiles` 0.16.0 tabs are
  AccessKit-unnamed by default and pdfce supplies names itself via
  `on_tab_button`'s `WidgetInfo` pattern. This redesign removes tabs from
  the left dock almost entirely (§4.1) — which, if anything, **shrinks**
  this particular gap's surface area, since standalone panes read through
  ordinary heading/label text rather than through the tab mechanism that
  needed the workaround in the first place.

---

## 8. Prioritized change list

### P0 — the shell restructure and the minimal honest Comments surface

1. Density convention (`UI_PREFERENCES.md` §11) — slice 1.
2. Promote `ArmedTool`/`Properties`/`Pages` to standalone, always-visible
   left-dock panes; delete the tool-arm auto-raise mechanic; narrow
   `PaneSubject`; new `DockPanel::Activities` with the 3-member (pre-
   Comments) segmented control — slice 2.
3. Per-compartment collapse chevron (§4.3) — ships with slice 2, not
   deferred, since it's the accessibility mitigation for item 2's own cost
   (§5 item 5, §7).
4. Doc-comment corrections in `dock.rs` (§6 slice 3) + `FEATURES.md`
   wording — same commit as item 2.
5. `annot::Annotation` core-model extension: `contents`/`author`/
   `mod_date` (§3.4) — slice 4.
6. `PaneSubject::Comments`, new `RibbonGroup::CommentsList` on
   `RibbonTab::Review`, the list panel itself: browse, page order, subtype
   label, honest no-note caption, navigate + highlight — slice 4.

### P1 — real new capability, named, scoped as its own follow-on work

7. General `delete_annotation` core verb (scoped per `edit.rs` L3663) +
   the Comments panel's Delete row action — slice 5.
8. `pdfce-cli list-annotations` `contents=`/`author=` fields — slice 5.
9. Real default-height tuning for the four stacked left-dock compartments,
   verified against the running app.
10. `ResetScope`'s left-panels reset extended to also reset the four
    compartments' collapse state (§4.3).

### P2 — valuable, not required to answer today's request

11. A "select on canvas from a Comments row" mechanism, once Pass 22.0
    ships (annotation selection) — named, not scoped (§3.3).
12. Note-text authoring for geometric markup shapes (`/Contents` on
    Ink/Square/Circle/Line/Polygon) — the fix to §5 item 4's honest ceiling,
    a Pass 6.1-adjacent capability, not this document's to scope further.
13. A parsed/validated `/M` mod-date type, if a "most recently commented"
    sort ever gets requested — the raw string is sufficient for P0's
    page-order list.

### Items for the engineer, not mine to decide

- **Property 4's exact intended reading (§2.4)** — confirm with the
  operator before or alongside slice 2; the narrow reading needs no further
  work beyond slice 2, the maximal reading is a refusal that needs to be
  said by name.
- **Whether slice 4 (Comments) ships before or after slice 2** (§6) — both
  orders work; scheduling call, not a design one.
- Exact default heights/proportions for the four stacked left-dock
  compartments (§8 item 9) and the segmented control's exact button
  labels/icon coverage — spacing and naming judgments best made against
  the running app, per this agent's standing practice.
- Whether `RibbonGroup::CommentsList`'s button reads "Comments" or "Notes
  list" or similar — cosmetic, left to the `ui_text.rs` catalog author,
  same posture `forms-panel.md` §9 item 6 already took for its own naming
  question.

---

## 9. `ui_text.rs` catalog — new entries this document requires (R1)

- `dock_panel_armed_tool_label()` / `_tooltip()` — replaces
  `dock_panel_tool_options_label()`/`_tooltip()` (renamed, not merely
  reused, since the pane's job is now narrower and more honest).
- `dock_panel_properties_label()` / `_tooltip()` — new, since `Properties`
  is now a `DockPanel` variant with its own tab-free identity, not a
  `PaneSubject` value.
- `dock_panel_activities_label()` / `_tooltip()`.
- `activities_pane_title()` — the segmented-control pane's own heading.
- `activities_segment_batch_label()` / `_forms_label()` / `_redact_label()`
  / `_comments_label()` — the four segmented-control button labels.
- `panel_collapse_toggle_tooltip()` / `panel_expand_toggle_tooltip()` (§4.3)
  — one pair, reused by all four compartments' chevrons, not four separate
  pairs (R2: same meaning, same string).
- `comments_pane_title()`, `comments_panel_intro()`,
  `comments_panel_no_document_hint()`, `annotation_no_note_caption()` (§3.2),
  `ribbon_group_comments_list()` (§3.5).
- `annotation_row_page_suffix(page: usize) -> String` — reuse the exact
  wording pattern `forms-panel.md`'s row page suffix already uses, not a
  second convention.

Every entry above is a new `pub fn` in `ui_text.rs`'s existing catalog
discipline (R1) — no operator-visible string written inline in `main.rs`,
per `tools/check-ui-strings.sh`.
