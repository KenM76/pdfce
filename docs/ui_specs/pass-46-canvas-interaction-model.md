# Pass 46 UI Spec — Canvas Interaction Model: Draw-Where-You-Point,
Select, Move, Resize

> Authored by `pdfce-ui-specialist`, 2026-08-12, on dispatch from the
> engineer, off the operator's direct complaint (quoted in full below).
> This spec **supersedes `docs/ui_specs/pass-6.1-markup-tools.md`** —
> see §0 for exactly what survives, what is amended, and what is
> dropped. Do not re-read pass-6.1 as authoritative for anything this
> document says otherwise about; do not delete pass-6.1 either — it is
> historical record and §0's table is the map between the two.
>
> Read before implementing: `crates/pdfce-gui/src/{canvas.rs, main.rs,
> object_provider.rs, dock.rs, viewer.rs, canvas_overlay.rs}` in full;
> `crates/pdfce-core/src/{annot.rs, annot_author.rs, edit.rs}`;
> `UI_PREFERENCES.md` (repo root, §4 canvas-overlay palette); this
> project's `.claude/agent-memory/pdfce-ui-specialist/` entries on
> Pass 12.0, Pass 14.3, Pass 12.M2, the gesture-commit audit, and the
> tool-options-dock spec — all five are load-bearing precedent this
> spec builds on rather than re-derives.

---

## 0. Supersession — what changes from pass-6.1-markup-tools.md

Pass 6.1's spec (2026-07-31) predates the `CanvasTool` gesture
framework by one day of engineering time but a full architectural
generation: it designed its own parallel `MarkupTool`/`DrawState`
state machine because `CanvasTool` did not exist yet. `CanvasTool`
shipped with Pass 12.0 the same week and every tool built since
(TextEdit, AddText, PlaceField, the three Measure tools, VectorEdit)
was built on it. Pass 6.1's GUI layer was **never implemented** — what
shipped instead was a minimal placeholder,
`PdfceApp::add_markup_shape` (`main.rs:5548`), which drops a
Square/Circle/Line/Highlight annotation at a fixed, jittered position
at the page centre with no drawing gesture at all. This is precisely
what the operator is describing.

| Pass 6.1 §/content | Status here | Why |
|---|---|---|
| §1 (`MarkupTool`/`DrawState` state machine) | **Dropped.** Replaced by §1 below (`CanvasTool::Markup`) | `CanvasTool` did not exist when §1 was written. A second, parallel gesture state machine sharing one canvas with the seven that already exist is exactly the inconsistency the operator is reporting — see §1.1 for the full argument. |
| §2.1 (geometry: `screen_to_page`/`page_to_screen`, page-space storage) | **Survives verbatim, as a citation, not a re-derivation.** | Correct then, correct now, and superseded in practice by the *better* bridge Pass 12.0 built one day later: `viewer::canvas_to_pdf_space`/`pdf_space_to_canvas`, which — unlike the pair §2.1 asked for — is inverted from the SAME transform the renderer uses (module docs, `object_provider.rs`). Use the Pass 12.0 bridge, not a new one; the reasoning for storing points in page-space is unchanged and still correct. |
| §2.2 (canvas focusability) | **Already done, by Pass 12.0/9a**, not by this Pass or by 6.1. | `main.rs`'s own module doc caveat this section named is already closed; confirmed by reading the current canvas image `Response` construction (`main.rs` ~16366-16400), which already senses click-and-drag conditionally on tool state. |
| §2.3–§2.9 (drag mechanics, live preview, commit-to-`EditSession`) | **Amended, not dropped.** The MECHANICS (page-space storage, overlay-not-reraster preview, one command per shape, `refresh_pages`, the `edit_note` narrator line) are correct and reused. The STATE CARRIER changes: from `DrawState`/`active_tool: Option<MarkupTool>` to a `MarkupToolState` living inside the `CanvasTool::Markup` arm, following the "each tool owns its own state shape" convention `TextEditState`/`MeasureCircularState` already established (Pass 14.3, Pass 12.M2). | The gesture logic pass-6.1 designed was sound; only its home changed. |
| §3 (quad-point rectangle fallback) | **Survives, folded into the unified `MarkupKind`.** The CCW-vs-Z-order dependency (§3.5 item 1) and the appearance-generation dependency (§3.5 item 2) are **both resolved** — confirmed by reading `pdfce-core/src/annot_author.rs` directly: `MarkupSpec::TextMarkup` and `build_appearance` for all four subtypes already ship. This spec's gating on P1 no longer applies; treat quad-point markup as P0-buildable. | A genuine, good-news correction: two "before P1 can ship" blockers pass-6.1 named are already closed. Flag to the librarian — pass-6.1's own open item is stale. |
| §4 (property bar placement: "a new transient top panel") | **Dropped, replaced by the shipped shell.** | `DockPanel::ArmedTool` (Pass 34.1, the shell redesign) is now the ONE home for every tool's property bar — a top panel was the right call in July, and the shell that made it unnecessary shipped in August. §2 below places Markup's property bar there, like every other tool. |
| §5 (signature/certification interaction) | **Survives verbatim.** | Nothing about the certification-refusal architecture changed; re-verify the `/P` census call sites still exist as described, but do not re-derive the reasoning. |
| §6 (discoverability, keyboard map, toolbar entry) | **Amended.** The `ui.menu_button` Draw▾ menu this section specifies is retired — `main.rs`'s own dead-code removal note (~L14030) records that ALL FOUR toolbar dropdown menus (Markup▾, Text▾, Measure▾, Copy▾) were flattened into per-ribbon-tab button rows in Pass 24.4, for reasons unrelated to this Pass. The **existing** flat `RibbonGroup::Markup` buttons (`ribbon.rs:187`) are the real toolbar entry point; §5 below re-specifies their wiring, not their shape. The `Alt+`-letter keyboard map is unverified against the shipped `collect_keyboard_actions` bindings and must be re-checked at implementation, per pass-6.1's own flag. | Toolbar architecture moved twice since this section was written (dropdown→flat rows, then flat rows reorganized into ribbon tabs); the content (what commands exist, what they're called) is unaffected. |
| §7 (accessibility) | **Survives, extended.** §6 below adds the new selection/resize-handle gap to the same tracked `accesskit` list this section already opens. | No regression, a genuine new instance of an existing named gap. |
| §8–§9 (`ui_text.rs` catalog, undo/write-path table) | **Amended.** The catalog entries are still needed; several need re-authoring against `MarkupKind` instead of `MarkupTool`, and new entries are needed for selection/move/resize (§7 below). The undo/write-path table gains new rows for move/resize. | Mechanical extension, not a re-design. |
| §10 (priority table) | **Superseded by §8 (slicing) below**, which reorders around what is independently visible to an operator testing builds — pass-6.1 was never scoped against that constraint because it was written before this Pass's post-hoc-editing requirement existed. | |
| §11 (open items for the librarian) | **Partly resolved, partly carried forward** — see §9 below, which restates only what's still open. | |
| The header's own "does not cover... selecting, moving, resizing" scope note | **This is now half of the operator's literal request and is filed here as §3-§4**, not deferred further. | The operator asked for exactly this, in these words: *"I can't drag or resize them."* |

**One correction to the dispatch brief itself**, found while reading
the code rather than assumed: the brief's fact (5) says grep found "no
`CanvasSelection` enum, no handle/corner/resize types" and flagged this
as inconclusive. It was right to flag it as inconclusive — the actual
substrate is considerably richer than that grep suggested. Pass 9a
already shipped a full `CanvasTargetProvider`/`TargetId`/
`canvas_selection: BTreeSet<TargetId>` general object-selection model
(click, Shift-click, marquee, front-to-back click-cycling through
stacked objects, per-frame pruning of stale targets after an edit) that
already runs live in "no tool armed" view mode for page **content-
stream** objects (paths, text runs) — confirmed by reading
`crates/pdfce-gui/src/{canvas.rs, object_provider.rs}` and the
`no tool answers a click` / `plain-selection path` code and tests
directly. **This substrate does not cover annotations at all** — see
§3.1 for why, and why that gap, not a missing selection concept, is
the real one. VectorEdit's whole-object/subpath/node drag-to-move is
also already live, implicit-commit-on-mouse-release, no separate
Accept step (`main.rs::run_vector_edit_tool`) — this is the *third*
"already-working, no-floating-box" precedent to cite (after
VectorEdit and the ce-dimension drag the tool-options-dock spec
already named), not a design to invent.

---

## 1. Decision (a) — Markup joins `CanvasTool`, as ONE new variant

### 1.1 Why it joins, not a parallel machine

The dispatch brief's own prior is correct, and the case for it is
stronger now that the actual substrate has been read in full: seven
tools already share one `active_tool`/`TOOL_PRECEDENCE` dispatch, one
Escape-precedence chain (`resolve_escape`), one pan-suppression rule
(`canvas_suppresses_pan`), one commit-interrupt policy
(`GestureInterrupt`), and — as of the 2026-08-06 independent-toggles
ruling — one `BTreeSet<CanvasTool>` "which tools are armed" model with
an explicit, arguable precedence order. Markup staying on its own
`MarkupTool`/`DrawState` machine outside all of that is not a neutral
design choice sitting beside the others; it is the literal, confirmed
root cause of the complaint. `add_markup_shape` never calls
`screen_to_page`, never checks `active_tool`, never participates in
Escape, pan-suppression, or the precedence ladder — it is invisible to
every one of the substrate's own rules, which is exactly why it reads
to the operator as the application "operating weirdly."

### 1.2 One variant, not six (or ten)

```rust
pub enum CanvasTool {
    TextEdit,
    AddText,
    PlaceField,
    MeasureLinear,
    MeasureCircular,
    MeasureScale,
    VectorEdit,
    /// Author a new geometric- or text-markup annotation (Pass 46,
    /// superseding the unbuilt pass-6.1 `MarkupTool`): click/drag draws
    /// the CURRENT `MarkupKind` at the pointer; commit is one
    /// `EditSession` `add_markup` command. A plain click's meaning here
    /// ("start drawing THIS kind of mark") must not be silently
    /// repurposed from the pan/select default — same argument that made
    /// `AddText`, `PlaceField`, and `VectorEdit` each their own tool
    /// rather than a sub-mode.
    ///
    /// One variant carries all ten markup kinds (Square, Circle, Line,
    /// Ink, Polygon, PolyLine, Highlight, Underline, StrikeOut,
    /// Squiggly) in `MarkupToolState::kind`, exactly the shape
    /// `MeasureCircular` already uses for Radius/Diameter (one tool, a
    /// display/mode toggle over it) and `PlaceField` already uses for
    /// its four field types (one tool, a type selector in Tool
    /// Options) — NOT ten `TOOL_PRECEDENCE` entries that are mutually
    /// exclusive in practice but independently toggleable in the type
    /// system, which `PlaceField`'s own doc comment names as the wrong
    /// shape for exactly this situation ("a set of enabled tools where
    /// three of the four combinations mean nothing coherent").
    ///
    /// Unlike `MeasureCircular`'s radius/diameter toggle (one geometry,
    /// two displays), switching `MarkupKind` mid-tool is a real mode
    /// change in what a click means (a Line click is one point of a
    /// two-point drag; a Polygon click adds a vertex to a running list;
    /// a Square click is a drag-rect corner) — so unlike
    /// `MeasureCircular`, changing `kind` **discards** any in-progress
    /// gesture first (pass-6.1 §1.4's "entering a tool discards any
    /// in-progress shape" rule, narrowed to "changing kind" since the
    /// tool itself does not change). Free, because nothing has
    /// committed yet (rule 7).
    Markup,
}
```

`MarkupKind` is pass-6.1 §1.2's `MarkupTool` enum, renamed and
unchanged in content (its `is_quad_point`/`has_stroke`/
`has_fill_option`/`is_multi_click` helper predicates all survive
verbatim — they were never wrong, only homed on the wrong carrier
type):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkupKind {
    Ink, Square, Circle, Line, Polygon, PolyLine,
    Highlight, Underline, StrikeOut, Squiggly,
}
```

`GuiMarkupKind` (`main.rs:1954`, today's shipped 4-variant Square/
Circle/Line/Highlight placeholder enum) is **retired and replaced** by
`MarkupKind` — not extended in place, because its whole reason for
existing (the payload of the immediate-author `Action::AddMarkupShape`)
is itself retired by §2 below. `Action::AddMarkupShape` and
`PdfceApp::add_markup_shape` are **deleted**, not deprecated-beside;
there is no case where the centered-jitter placement is still the
right behavior once a real gesture exists, and keeping it as a second
path would be exactly the "two ways to do the same thing, one of them
worse" defect this project consistently refuses to ship.

### 1.3 `TOOL_PRECEDENCE` placement

```rust
const TOOL_PRECEDENCE: [CanvasTool; 8] = [
    CanvasTool::TextEdit,
    CanvasTool::AddText,
    CanvasTool::Markup,       // NEW — see reasoning below
    CanvasTool::PlaceField,
    CanvasTool::MeasureLinear,
    CanvasTool::MeasureCircular,
    CanvasTool::MeasureScale,
    CanvasTool::VectorEdit,
];
```

Placed immediately after `AddText`, for the identical reason
`PlaceField` already sits there (`main.rs:3469-3474`, quoted in full
because the reasoning transfers exactly): AddText and Markup both mean
"a click ALWAYS creates something here," so both must outrank the
picking/measuring tools below them (a Markup click must never be
misread as a measurement point-pick or a snap query) — and Markup
sits below AddText for the same "AddText can be mid-composition with
typed-but-uncommitted text, which stranding would be worse" reasoning.
Markup sits above `PlaceField` because the two are peers in kind (both
"place a new thing") and the existing order already had a rule for
peers-of-this-kind before Markup existed (declaration order was
arbitrary among them); no operator-visible behavior depends on Markup
vs. PlaceField order specifically, so this is the engineer's to
adjust if a real conflict surfaces, unlike the AddText/measure
boundary, which is load-bearing.

---

## 2. Decision (b) — the universal contract, and who already meets it

### 2.1 The contract, stated once, formally

> **Arming a `CanvasTool` immediately populates `DockPanel::ArmedTool`
> with that tool's property bar and status, with no further click
> needed. A pointer gesture on the canvas — click, drag, or a click
> sequence for a multi-click tool — always targets the location the
> operator is pointing at, expressed by converting through
> `viewer::screen_to_page` → `viewer::canvas_to_pdf_space` (never a
> document-derived, fixed, or centred position). The gesture completes
> with either an implicit commit (mouse-up / Enter, for a plain
> authored value the operator can see and undo) or an explicit
> Accept/Reject pair inside the SAME `ArmedTool` pane (for a value
> pdfce inferred — rule 4) — never both, and never a control whose own
> screen position is derived from the page.**

This is exactly `Self::tool_builds_text_edit`/`tool_builds_add_text`/
`tool_builds_measure`/`tool_builds_vector_edit`'s existing dispatch
pattern in `canvas.rs`, generalized into one sentence, plus the
`DockPanel::ArmedTool` placement the shell redesign already settled.

### 2.2 Compliance audit — every existing `CanvasTool`, checked, not assumed

| Tool | Arming shows options immediately? | Gesture targets the pointer? | Commit model | Verdict |
|---|---|---|---|---|
| `TextEdit` | Yes (`tool_options_panel`, `main.rs` ~14543) | Yes (`canvas_to_pdf_space` via `EditableTextModel::hit_test`) | Explicit Accept/Reject (rule-4 reviewable draft, Pass 14.3 spec §0.2) | **Compliant** |
| `AddText` | Yes | Yes (`resolve_drag_placement`, PDF-space) | Explicit Accept/Reject | **Compliant** |
| `PlaceField` | Yes | Yes (`run_place_field_tool`, `main.rs:19361` — click/drag builds the box exactly where the pointer is, confirmed by reading the function in full) | Explicit Accept/Reject (name + type in Tool Options before commit) | **Compliant** |
| `MeasureLinear`/`MeasureCircular`/`MeasureScale` | Yes | Yes (snapped point-picks, `canvas_to_pdf_space`) | `MeasureCircular`'s derived centerline gets an explicit confirm (fuzzy inference); plain linear picks are a candidate for implicit commit per the gesture-commit audit's own recommendation (unresolved as of that audit — not this Pass's job to change) | **Compliant** |
| `VectorEdit` | Yes | Yes | **Implicit**, direct commit on `drag_stopped()` — no Accept button at all, one `EditSession` command per completed drag | **Compliant, and the cleanest existing model** — cite this one first when building Markup's own commit, not the Accept/Reject tools |
| `Markup` (today's placeholder, pre-this-Pass) | **No** — `add_markup_shape` bypasses `active_tool`/Tool Options entirely | **No** — computes a rect from `page.media_box`'s centre plus a jitter counter | Immediate, on menu click, with no gesture at all | **The one and only non-compliant instance in the entire application** |

This is worth stating plainly because it directly answers the
dispatch brief's own §(b) question: **no**, this is not a systemic
"nail down the basic tool" problem — six of seven shipped tools
already do exactly what the operator expects, built correctly, on the
first attempt, by prior Passes. Markup is the single holdout, and it
predates the framework the other six were built on. Nothing else in
this Pass needs to "bring an existing tool into line" — §5 below wires
Markup onto the SAME pattern the other six already prove works.

---

## 3. Decision (c) — post-hoc selection, drag, resize

### 3.1 Why annotations need a new provider, not a bigger `ObjectModelProvider`

`ObjectModelProvider` (`object_provider.rs`) is built from
`pdfce_core::vector::decompose_page`, which walks the page's **content
stream** — paint operators (paths, text runs, images, form XObjects).
Annotations are a structurally separate part of the PDF object model
(`/Annots`, outside the content stream, ISO 32000-1 §12.5) and
`decompose_page` does not see them — confirmed by reading
`pdfce_core::vector::VectorObject`'s variants directly (`Path`,
`Text`, and image/form-XObject kinds only). Folding annotations into
this provider would misdescribe what it decomposes and would also be
architecturally wrong: an annotation's edit verbs (move whole,
reshape) share nothing with a path's (move/subpath/node rungs) or a
text run's (no move verb at all today) — trying to force one
`TargetId` space and one selection set across three kinds this
different is exactly the premature unification this project has
consistently avoided (each selectable kind already "owns its own
state shape" — `TextEditState`, `MeasureCircularState.picked_objects`,
and `doc.selected_dimension` for ce dimensions are three existing,
independent precedents for this same call).

**Decision: a new `AnnotationTargetProvider`** (same crate,
`pdfce-gui`, same shape as `ObjectModelProvider` — a thin adapter that
calls into `pdfce_core::annot`'s read-only model and owns only the
coordinate bridge and ID-minting), and **a new, independent selection
field**, `doc.selected_annotation: Option<AnnotationId>`
(`AnnotationId` = the annotation's index into `page.annots` — the
existing `Annotation.id: Option<ObjId>` is the PDF object identity for
diagnostics, not a stable UI selection key across an edit; the paint-
order index is what `ObjectModelProvider`'s `TargetId` already does
for the same reason, so this mirrors an established pattern rather
than inventing one) — **not** folded into `canvas_selection`, for the
same "each kind owns its shape" reasoning `doc.selected_dimension`
already established, one level up. `AnnotationTargetProvider`
implements the SAME `CanvasTargetProvider` trait `ObjectModelProvider`
does (hit_test/hit_test_rect/bounds), so selection-outline drawing,
marquee handling, and pruning-after-edit all reuse the substrate's
existing, tested functions (`canvas::selection_after_click`,
`selection_after_marquee`, `prune_selection`) rather than
reimplementing them a second time for annotations specifically — the
NEW code is the provider, not the selection mechanics around it.

### 3.2 No tool required — selection is a base-view-mode action

Selecting an already-placed annotation must **not** require arming any
`CanvasTool` first. This mirrors the two precedents already live in
the app today, checked directly rather than assumed: with **no**
`CanvasTool` armed, the canvas already supports click/marquee
selection of content-stream objects (`canvas_selection`, Pass 9a) and
drag-to-reposition of ce dimensions (`run_dimension_drag`,
`main.rs:22452`, called unconditionally per frame, not gated on
`active_tool`). Gating annotation selection behind an armed tool would
be the operator's exact complaint in a new shape: "I have to arm a
special tool just to touch the thing I already drew." A plain click on
the canvas in view mode, with nothing armed, must find the annotation
under the pointer.

### 3.3 Click priority when several kinds of thing sit under one point

Three independently-selectable kinds of "thing under the pointer" now
coexist in base view mode: a ce dimension, an annotation, and a
content-stream object. The tool-options-dock spec already recorded
that ce dimensions win over content objects (`run_dimension_drag`'s
own "a ce dimension takes priority over object selection under the
pointer" rule). **This spec extends that to a three-way chain: ce
dimension → annotation → content-stream object**, checked in that
order, first hit wins. Reasoning: an annotation is painted on top of
page content in every conformant renderer (§12.5.2), so it is the more
"front" thing at that point and the more likely target of an
operator's click; a ce dimension outranks even that because it is the
thing pdfce itself just drew as an overlay ON TOP of everything,
including other annotations, in its own authored workflow. This is a
genuinely new decision (not previously named anywhere in the app) and
should be recorded as such, not silently folded into the existing
one-line rule.

### 3.4 What "selected" looks like

Reuses the SAME visual vocabulary `canvas_overlay::draw_selection_outlines`
already establishes for content objects (a 2px outline rect, drawn by
the painter directly, never a re-raster) — **not** a new color from
thin air. Per `UI_PREFERENCES.md` §4's already-consolidated palette:
content-object selection currently reads through `ui.visuals()
.selection.stroke.color` (theme-aware — the general "this is selected"
signal); ce-dimension selection uses the theme-**invariant**
`OVERLAY_CE_DIMENSION_SELECTED` (teal). An annotation selection needs
its own value on the same theme-invariant "canvas overlay" axis
(§4's whole point — canvas overlays must read consistently regardless
of light/dark theme, since they sit on top of rendered page content
whose own colors don't change with the app's theme). **Recommended:**
a new named token, e.g. `OVERLAY_ANNOTATION_SELECTED`, distinct from
both the teal ce-dimension color and the blue
`OVERLAY_NODE_STROKE`/`OVERLAY_TEXT_SELECTION_FILL` family (which
already means "editable point/text," a different concept) — exact hue
is the engineer's call, but it must be added to `UI_PREFERENCES.md`
§4's table as an 8th named role when this Pass ships, not left as a
fresh, undocumented `Color32::from_rgb` literal (the precise defect
§4's audit was written to prevent).

### 3.5 Resize handles — two genuinely different geometry families, and why

Reading `pdfce_core::annot::Annotation`'s doc comment directly
surfaces the load-bearing constraint for this whole section: **the
read model deliberately does not carry per-subtype geometry keys**
(`/L`, `/Vertices`, `/InkList`, `/QuadPoints`) — only `/Rect`. Its own
comment names why: *"under R43 they are neither painted nor, in Pass
6.0, generated from."* This is a bigger, more foundational core gap
than the dispatch brief anticipated, and it splits annotation editing
into two families with genuinely different reach this Pass:

**Family A — Rect-bounded (Square, Circle, the quad-family drawn as an
axis-aligned rectangle, and by extension Stamp/FreeText/Widget even
though this Pass does not touch their content).** A PDF viewer places
an annotation's appearance by mapping the appearance stream's own
`/BBox` (through its `/Matrix`) into the annotation's `/Rect` — ISO
32000-1/2 §12.5.5's placement algorithm, already the mechanism every
conformant reader uses to paint ANY annotation appearance. This means
**moving or resizing a Rect-bounded annotation is, in general, a pure
`/Rect` edit that never touches `/AP` at all** — the existing
appearance auto-fits into the new rectangle, for pdfce-authored AND
foreign annotations alike, because the mapping is a property of the
PDF spec's own placement rule, not of who authored the content.
**⚠️ Flagged, not asserted as settled:** this is spec-governed
behavior (rule 1) and this agent has not verified it against
`D:\Dev\Rag-Specialized\PDF_Spec\` — the engineer should dispatch
`pdfce-spec-librarian` to confirm §12.5.5's exact placement algorithm
and any edge cases (a `/Matrix` other than identity, an `/AP` with no
`/BBox`) before building `set_annotation_rect` on this premise. If
confirmed, this is a **small, safe, single core verb** —
`EditSession::set_annotation_rect(page_index, annot_index, new_rect)`
— that covers move AND resize for the entire Rect-bounded family in
one command, with no appearance regeneration and no per-origin
distinction (foreign annotations are just as safely resizable as
pdfce's own).

**Family B — point-list-defined (Line, Polygon, PolyLine, Ink, and
QuadPoints when not axis-aligned).** Here the visual content of the
appearance stream literally draws the point geometry — a Line's `/AP`
is a content stream containing the actual line-drawing operators at
the actual coordinates. Moving or reshaping one of these means editing
the point list AND regenerating `/AP` to match — there is no
`/Rect`-only shortcut. This is where the annotation-geometry read gap
above becomes load-bearing: **`pdfce-core` needs a new read accessor**
exposing `/L`/`/Vertices`/`/InkList`/real `/QuadPoints` (not just
`/Rect`) before the GUI can even DRAW a selection outline that follows
the true shape rather than its bounding box, and a new reshape verb
(`EditSession::reshape_annotation`, taking a per-subtype geometry
update — translate-all-points for a move, one-point-moved for a
vertex/endpoint drag) that regenerates `/AP` via the SAME
`build_appearance` function `annot_author.rs` already has for
authoring. **Honest scope note, disclosed rather than silently
accepted:** regenerating `/AP` for a FOREIGN Family-B annotation
discards any cosmetic property beyond what `MarkupSpec` models today
(a dash pattern, a cloudy border `/BE`, a non-default line ending not
in pdfce's current `LineEnding` set) — this is real, disclosed loss
for a foreign annotation of these subtypes specifically, and the
Tool Options pane must say so BEFORE the operator drags one, not
after (§4.3). A pdfce-authored Family-B annotation has nothing to lose
this way, since `build_appearance` is exactly what produced it in the
first place.

**Handle geometry, per family:**
- Family A: 8 handles (4 corners + 4 edge midpoints) on the selection
  rect, standard drag-to-resize convention across essentially every
  graphics editor; corner handles preserve aspect only if the operator
  holds a modifier (P2, not required this Pass — plain corner drag
  resizes freely, matching how `PlaceField`'s own drag-to-size already
  behaves with no aspect lock).
- Family B (Line): 2 endpoint handles.
- Family B (Polygon/PolyLine): one handle per vertex — reuses the
  SAME visual glyph the Node rung already established
  (`OVERLAY_NODE_STROKE`/`OVERLAY_NODE_FILL`, filled/hollow circles)
  since a vertex handle is, conceptually, the same kind of thing a
  path node already is, and the tool-options-dock spec's own reasoning
  for why extension-line handles needed a NEW glyph (avoiding
  collision with the Node rung's existing one) does not apply here —
  there IS no simultaneous Node-rung display for an annotation, so
  reusing the glyph does not collide with anything on screen.
- Family B (Ink): **no per-point reshape handles this Pass** — an Ink
  stroke's point list can run into the hundreds for a real freehand
  gesture, and offering all of them as individually draggable handles
  is exactly the R83 hazard decision 028 already found and fixed for
  path nodes on a dense CAD object. Ink gets **whole-annotation move
  only** in this Pass (drag the body, all points translate together);
  point-level ink reshaping is a named P2 follow-up, not silently
  attempted.

### 3.6 Undo shape

One `EditSession` command per completed drag (move OR resize),
committed **implicitly on mouse-release** — no Accept/Reject pair, no
new floating or docked confirm control. This is a direct manipulation
whose result is fully visible on the canvas the instant the drag ends
and fully reversible in one Undo — squarely the case rule 4's 2026-08-05
narrowing (decision 024 §4.4) describes as needing no confirm click,
and it is the SAME commit shape `run_vector_edit_tool` already ships
for exactly this kind of gesture (whole-object/subpath/node drag,
commit on `drag_stopped()`). Do not build a fourth confirm-dialog
convention for this; there already isn't a third.

---

## 4. Decision (d) — selection-driven Tool Options / Properties

### 4.1 Where it lives

`DockPanel::ArmedTool`'s `properties_panel` function (`main.rs:14831`)
already has an established, working shape for exactly this problem:
`selected_dimension_section(doc, ui)` renders a contextual section
when `doc.selected_dimension` is `Some`, ahead of the document-level
`/Info` grid. **A new `selected_annotation_section(doc, ui)` joins it
as a sibling section, in the same pane, using the same tiering
convention** ("the thing the operator was just looking at on the
canvas" first, the document's own least-frequently-touched properties
last) — not a new taxonomy bucket, not a new dock panel. This directly
answers the operator's own words: *"the appropriate things should show
in the tool options box"* — Properties already lives inside
`DockPanel::ArmedTool` (via `pane_subject`, §"Where these three still
share a pane" doc comment, `main.rs:14392` onward), so this is
literally the tool options box already.

### 4.2 What it shows for a selected, editable annotation

Reuses the SAME control set the draw-time property bar uses (§5.3
below) — color/width/opacity/fill, per `MarkupKind`'s existing
`has_stroke`/`has_fill_option`/`is_quad_point` predicates — but now
each control edits the COMMITTED annotation directly rather than the
"next shape" pen. **Commit model for property edits:** one
`EditSession` command per control interaction, committed on the same
event every other property-style control in this app already commits
on (a color picker's popup closing, a slider's `drag_stopped()`) — NOT
per-frame while a slider is being dragged, matching the coalescing
`Slider`-driven edits elsewhere in the app already use (the reflow
width-handle is the direct precedent, per `UI_PREFERENCES.md` §4's own
`OVERLAY_PREVIEW_FILL` note). Also shown: `/Contents` (the annotation's
note text, if present) as a read-only display this Pass — editing an
existing annotation's note text is real, wanted, and out of scope
here; name it as a named follow-up rather than silently omit the
field.

### 4.3 What it shows for a selected, non-editable annotation

Per the dispatch brief's own explicit ask, this must not read as a
bug. §3.5 already names the two ways an annotation can be
non-editable this Pass: (1) its subtype is entirely outside
`MarkupKind`'s coverage (Popup, 3D, Widget — Widget specifically
already has its OWN edit surface via Pass 7's form-fill work, which
this spec does not touch or duplicate), or (2) it is a Family-B
subtype whose reshape would discard cosmetic properties `MarkupSpec`
doesn't model. For (1): show the subtype name, `/Contents` if present,
and one line, matching the honest phrasing convention `dock_panel_*`
tooltips already use (R83's "say WHY, don't just show nothing"): *"This
annotation type is not editable here."* For (2): show the same
controls §4.2 offers, but with a persistent, non-dismissible caption
above them — not a one-time toast — stating the disclosed loss:
*"Moving or resizing this will regenerate its appearance from these
properties. Details beyond color, width, and points will not be
preserved."* This is exactly the "quality disclosure, not a data-
provenance one" weight class pass-6.1 §3.3 already established for the
rectangle-fallback highlight tool — reused, not reinvented, for a
structurally similar honesty problem.

### 4.4 What it shows for a selected ce dimension or content object

Unchanged — this Pass adds a sibling section, it does not touch
`selected_dimension_section` or any future content-object properties
section. Only one selection-kind's section renders at a time (they are
mutually exclusive by construction, since only one of `doc
.selected_dimension`/`doc.selected_annotation`/`doc.canvas_selection`
non-empty can be the ONE thing a single click just picked, per §3.3's
priority chain).

---

## 5. Building the draw gesture (Decisions a+b, applied)

### 5.1 `MarkupToolState`

```rust
struct MarkupToolState {
    kind: MarkupKind,
    draw: DrawGesture,       // pass-6.1 §1.3's DrawState, renamed, unchanged shape
    color: egui::Color32,    // "current pen," session-only, matches pass-6.1 §1.1
    width: f32,
    opacity: f32,
    fill: Option<egui::Color32>,
}
```

`DrawGesture` is pass-6.1 §1.3's `DrawState` verbatim (`Idle`,
`Ink{points}`, `Dragging{anchor, current}`, `Polygon{vertices,
cursor}`) — the state SHAPE was never the problem, only where it lived.

### 5.2 Wiring, reusing `run_place_field_tool` as the direct template

`run_place_field_tool` (§0's table already cites it) is the closest
already-shipped analogue for the drag-rect kinds (Square, Circle,
Highlight-as-rect, and the Underline/StrikeOut/Squiggly quad family):
read it before writing `run_markup_tool` — the bridge calls
(`canvas_to_pdf_space`/`screen_to_page`/`pdf_space_to_canvas`/
`page_to_screen`), the live rubber-band paint (`ui.painter_at
(image_rect).rect_stroke`, theme's preview color), the `MIN_DRAG`
click-vs-drag threshold, and the drag-anchor state field are all
directly reusable patterns, not just prior art to consult. Line and
Ink extend the same skeleton (two-point drag; continuous point capture
while dragging, respectively — pass-6.1 §2.4/§2.6 describe both
correctly, only the carrier changed). Polygon/PolyLine's click-to-add-
vertex/double-click-or-Enter-to-commit state machine (pass-6.1 §2.7) is
unaffected by the carrier change and reused verbatim, including the
context-sensitive Backspace-removes-last-vertex rule and its explicit
dispatcher-ordering note.

### 5.3 Property bar

Renders inside `tool_options_panel` exactly where `text_edit_options_ui`/
`run_place_field_tool`'s type selector already do, gated by a new
`canvas::tool_builds_markup(tool)` predicate following the established
`tool_builds_*` family. Content is pass-6.1 §4.2's control layout
verbatim (kind-conditional stroke/fill/opacity controls, the always-
visible hint label) — that layout was never wrong, only its container
(a `TopBottomPanel`) was, and that container is retired per §0's table.
The **kind selector itself** (which of the ten `MarkupKind` values is
active) is a new control this section adds that pass-6.1 didn't need
(that spec chose the kind via the pre-armed toolbar menu item; here,
since Markup is now ONE tool per §1.2, switching kind mid-tool needs
its own in-pane control — a row of small icon toggles reusing
`GuiMarkupKind::icon`'s already-existing per-kind glyphs, extended to
all ten `MarkupKind` values, six of which need NEW icons authored
under R124 since their commands did not exist as reachable UI before
this Pass: Ink, Polygon, PolyLine, Underline, StrikeOut, Squiggly).

### 5.4 Toolbar entry

The existing `RibbonGroup::Markup` flat button row is NOT replaced —
its four buttons (Square/Circle/Line/Highlight) simply change what
they DO: instead of dispatching `Action::AddMarkupShape(kind)`
(deleted, §1.2), each becomes an `icon_toggle`/`icon_text_toggle`
following the EXACT pattern `Edit Text`/`Add Text`/`Edit Objects`
already use for their own single-tool-arm buttons — `selected` reads
`doc.tool_enabled(CanvasTool::Markup) && doc.markup.kind == kind`,
`clicked()` dispatches `Action::SelectCanvasTool(Some(CanvasTool::Markup))`
plus a kind-set action if not already that kind. Six new buttons join
the row for Ink/Polygon/PolyLine/Underline/StrikeOut/Squiggly — net
ribbon growth of six controls in one tab that decision 024/031 already
gave its own room to (Edit tab, per the ribbon-groupings memory), not
six controls added to a flat, crowded toolbar the way the pre-ribbon
"9-control edit group" finding warned against.

---

## 6. Accessibility

- Selection-outline color for annotations (§3.4) is a new, distinct,
  theme-invariant hue — color is never the sole signal, since the
  outline shape itself (a rect or the true point-geometry outline for
  Family B) is the primary cue, matching every other selection
  convention already in the app.
- Resize handles are small filled/hollow circles or squares (§3.5),
  sized to the existing node-mark convention, which is already sized
  for a reasonable click target — do not shrink them for a denser
  look.
- **New, honest `accesskit` gap, joining the three already tracked
  against Pass 3.2's drag-reorder, pass-4's deferred text-selection,
  and pass-6.1's own shape-drawing gesture**: a resize-handle drag is,
  like the others, inherently a pointer-first gesture with no
  keyboard-only equivalent this Pass builds. State this plainly rather
  than promise a fix; the librarian should link this as the FOURTH
  occurrence of the one gap, not file a new entry.
- Every non-drag step remains fully keyboard-operable: arming Markup,
  switching kind, adjusting the property bar, committing a Polygon via
  Enter, Undo/Redo, Delete-to-remove a selected annotation (reusing
  whatever general delete-annotation verb exists or is built — see §9).

---

## 7. `ui_text.rs` catalog — delta from pass-6.1 §8

Every entry in pass-6.1 §8 whose name embeds `markup_tool_*`/
`markup_menu_*`/`markup_hint_for` etc. is re-authored against
`MarkupKind` (a rename of the type the function takes, not a rewording
of the string content — the tooltips/labels themselves are unchanged
and should be lifted verbatim). **New entries this Pass needs, not in
pass-6.1 because post-hoc editing wasn't in scope there:**

- `annotation_selected_heading(subtype: &str) -> String`
- `annotation_not_editable_here() -> &'static str` — §4.3(1)'s line.
- `annotation_reshape_will_regenerate_appearance() -> &'static str` —
  §4.3(2)'s persistent caption.
- `annotation_move_committed(subtype: &str) -> String` — the
  `edit_note` narrator line, matching `markup_added`'s existing
  phrasing convention ("Use Undo to reverse it until you save.").
- `annotation_resize_committed(subtype: &str) -> String` — same shape.
- Tooltip text for the six new toolbar buttons (Ink, Polygon,
  PolyLine, Underline, StrikeOut, Squiggly) — pass-6.1 §8 already
  authored all six; lift verbatim.

---

## 8. Decision (e) — slicing, ordered by what the operator can DO after each one

**Slice 1 — fix the draw gesture for the four kinds already shipped
(Square, Circle, Line, Highlight). No new `pdfce-core` work at all**
(`MarkupSpec`/`add_markup`/`build_appearance` already cover all four).
Ships §1 (`CanvasTool::Markup`, one kind at a time — the other six
kinds can wait for Slice 3), §2's compliance fix, §5 minus the six new
toolbar buttons, §7's draw-time-only entries. **After this slice, the
operator can click Square/Circle/Line/Highlight in the ribbon and draw
each one exactly where they point, with live preview, exactly the
behavior they described every other program having.** This is the
single highest-value, lowest-risk slice — it is a pure GUI rewire of
an already-working core call, and it is the exact complaint quoted at
the top of the dispatch. Ship it first, alone if necessary.

**Slice 2 — Family A post-hoc select/move/resize** (§3.1–§3.4, §3.5's
Family A, §3.6, §4 in full for Family A). Needs ONE new core verb
(`set_annotation_rect`, pending spec-librarian confirmation of the
§12.5.5 premise) and the new `AnnotationTargetProvider`. **After this
slice, the operator can click any Square/Circle/Highlight annotation
already on the page — one they just drew OR one that was already in
the file — drag its body to move it, and drag a corner/edge handle to
resize it, with its properties visible and editable in the same Tool
Options pane.** This is the slice that answers "I can't drag or resize
them" for the majority of what pass-6.1 shipped and what most real
PDFs' existing annotations already are (Square/Circle/Stamp/FreeText
boxes are the common case; freehand Ink and multi-point Polygon
annotations are comparatively rare in practice).

**Slice 3 — extend the draw gesture to the remaining six kinds** (Ink,
Polygon, PolyLine, Underline, StrikeOut, Squiggly). No new core work
(confirmed, §0's table). Mechanical extension of Slice 1's
infrastructure plus the six new icons/toolbar buttons. **After this
slice, every one of pass-6.1's originally-scoped ten markup kinds
draws correctly.**

**Slice 4 — Family B post-hoc select/move/resize** (Line endpoint
drag, Polygon/PolyLine vertex drag, Ink whole-move). Needs the new
`pdfce-core` annotation-geometry read accessor and the
`reshape_annotation` verb — the largest core lift in this spec.
**After this slice, every markup annotation pdfce can draw can also be
selected, moved, and (where geometrically meaningful) reshaped.**

Slices are independently shippable in this order; do not block Slice 1
or 2 on Slice 3/4's larger core work. If time runs out after Slice 2,
the operator already has a materially complete answer to both halves
of their complaint for the common case, with the remainder named and
scoped rather than silently dropped.

---

## 9. Open items for the librarian

1. **Two closed pass-6.1 blockers, found while reading code for this
   spec, not previously recorded as closed**: §3.5's QuadPoints
   ordering question and appearance-generation dependency are both
   resolved in shipped `pdfce-core` (`annot_author.rs`). Pass 6.1's
   own §11 item 2 (the `/P 3` certification-precision gap) is also
   already resolved in that file's history per pass-6.1 §5.4's own
   "Decision X11" — re-confirm this landed as described when Pass 46
   ships, since this spec did not re-audit it.
2. **A new, fourth core-level annotation gap, larger than the
   dispatch brief anticipated**: `pdfce_core::annot::Annotation` does
   not model per-subtype geometry (`/L`/`/Vertices`/`/InkList`/
   `/QuadPoints`) at all — only `/Rect`. This blocks Slice 4 entirely
   and should be scoped as its own `pdfce-core` task before that
   slice starts.
3. **New `UI_PREFERENCES.md` §4 palette entry** — `OVERLAY_ANNOTATION_SELECTED`
   (§3.4), an 8th theme-invariant canvas-overlay role, needs adding to
   that table when Slice 2 ships.
4. **Fourth occurrence of the tracked `accesskit`/pointer-first-gesture
   gap** (§6) — link to the existing three rather than filing new.
5. **A `pdfce-spec-librarian` confirmation is owed** before Slice 2's
   `set_annotation_rect` is built: verify ISO 32000-1/2 §12.5.5's
   appearance-placement algorithm covers the "resize via `/Rect` alone,
   `/AP` untouched" premise §3.5 Family A relies on, including the
   `/Matrix`-not-identity and missing-`/BBox` edge cases.
6. **`GuiMarkupKind` retirement and `Action::AddMarkupShape` deletion**
   (§1.2) touch existing, shipped code and existing tests — the
   engineer should grep for every remaining reference before deleting,
   not assume the type is otherwise unused.
