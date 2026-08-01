# Pass 14.3 UI Spec — Edit UI on the Pass 12.0 Canvas (Acrobat Text-Edit Parity)

> Authored by `pdfce-ui-specialist`, on dispatch from the engineer. Implements
> the sole remaining slice of decision 014
> (`docs/decisions/014-acrobat-text-editing.md` §5.2's "13.3"/ROADMAP's
> "14.3"): click→caret, type→edit, drag→select, live preview accepted by the
> operator, block-boundary review overlay, the three-trust-level + tagged
> disclosures. Engineer implements this verbatim; deviations are named, not
> silent (the Pass 3.2/6.1/7/8/12.0 convention).
>
> Read before implementing: `docs/decisions/014-acrobat-text-editing.md` in
> full (especially §5.1's six standing rules, R69–R74); `docs/ui_specs/
> pass-12.0-canvas-substrate.md` in full — this spec is the FIRST real
> inhabitant of that substrate (`CanvasTool` has shipped as `enum
> CanvasTool {}` since Pass 12.0 and is **still uninhabited today** —
> confirmed by reading `crates/pdfce-gui/src/canvas.rs` at spec-authoring
> time; no Pass 6.1/7/8 GUI slice has added a variant yet, despite those
> Passes' own content-authoring capability having shipped at the core/CLI
> level). `crates/pdfce-core/src/text_edit/{mod,model,edit,encoding,
> format}.rs` in full — the shipped 14.0–14.2 API this spec wires to, not a
> hypothetical one. `crates/pdfce-core/src/edit.rs`'s `EditSession` (undo/
> redo, `dirty_set`, `to_incremental_bytes`) — §0.2 below names a real
> mismatch between it and `text_edit`'s free-function shape that gates
> everything else in this spec. `crates/pdfce-gui/src/{main.rs,viewer.rs,
> canvas.rs,ui_text.rs}` in full.

---

## 0. What this Pass changes, and the one thing it cannot design around

### 0.1 First real `CanvasTool` variant

Every prior tool-bearing spec (6.1/7/8) was written against a `CanvasTool`
that either didn't exist yet or has since shipped uninhabited. This Pass
adds the enum's first variant:

```rust
pub enum CanvasTool {
    /// Acrobat-style in-place page-text editing (Pass 14.3). Caret/selection
    /// state lives in TextEditState (§2), not in canvas_selection/TargetId —
    /// see §0.3 for why this tool does NOT use CanvasTargetProvider.
    TextEdit,
}
```

This is the first time `canvas_suppresses_pan`, `resolve_gesture_interrupt`,
and `resolve_escape` (all shipped since Pass 12.0, all unit-tested only
against synthetic bools) get a REAL call site with `tool_active == true`.
**Verify, do not assume, before shipping:** re-run the existing Pass 12.0
unit tests, then confirm by hand that `Action::SelectCanvasTool(Some(
CanvasTool::TextEdit))` actually reaches `resolve_gesture_interrupt`'s
allow-list check and `apply()`'s tool-set branch — this is the substrate's
first live exercise of code paths that were, until now, structurally
unreachable.

### 0.2 The load-bearing core-accessor gap — flag to the engineer before writing any GUI code

`pdfce-core`'s `EditSession` (`crates/pdfce-core/src/edit.rs`) is the ONE
in-memory command-log undo/redo abstraction every other mutating operation
in pdfce goes through: `add_markup`, `add_redaction`, `fill_text_field`,
`delete_pages`, `rotate_pages`, `set_info_field`, etc. all take
`&mut EditSession`, mutate the session's in-memory `Document`, and push one
command onto its undo stack; `to_incremental_bytes()`/`to_full_bytes()` is
called only at actual Save time. This is exactly the "command-log undo
stack over the in-memory document" rule 2 of the standing UX rules
describes.

**`text_edit::edit_text` and `text_edit::format::set_format` do not do
this.** Both are free functions —

```rust
pub fn edit_text(doc: &Document, req: &EditRequest, opts: &EditOptions)
    -> Result<EditOutcome, EditError>;   // EditOutcome { bytes: Vec<u8>, report }
pub fn set_format(doc: &Document, req: &FormatRequest, opts: &FormatOptions)
    -> Result<FormatOutcome, FormatError>;
```

— that take an immutable `&Document` and return **already incrementally-
saved bytes**. There is no staged, undo-able, in-memory mutation step; a
successful call is, architecturally, "produce a new saved file," full stop.
This is the right shape for `pdfce-cli edit-text`/`format-text` (a one-shot
batch tool with no interactive session), but it is the **wrong** shape for
an interactive GUI that must let the operator make five text edits and then
Ctrl+Z them one at a time, exactly like five markup annotations.

**Do not work around this in the GUI** by reloading the whole `Document`/
`EditSession` from `outcome.bytes` after every accepted keystroke-edit. That
would: (a) force a full incremental-save + re-parse + Pass-4 re-extraction +
Pass-14.0 re-recognition on every single accepted edit, when every other
operation in the app does none of that until real Save; (b) require the
GUI's undo stack to hold two structurally different kinds of undo entry
(EditSession commands vs. whole-document byte-snapshots) and silently
splice between them — a correctness hazard, not just an inefficiency, since
`EditSession::undo_depth`/`dirty_set`/`can_undo` all assume ONE command log;
(c) throw away every other command's undo entry the instant a text edit
lands, if the reload replaces the `EditSession` outright.

**Required core addition for this Pass (flag to the engineer, do not defer
silently):** `EditSession` needs a session-integrated sibling of each
free function, reusing the SAME locate/re-encode/relayout logic
`text_edit::edit`/`text_edit::format` already implement, applied against
the session's own in-memory content-stream object instead of a borrowed
`&Document`, recorded as one undo-able command:

```rust
impl EditSession {
    pub fn edit_text(&mut self, req: &EditRequest, opts: &EditOptions)
        -> Result<EditReport, EditError>;
    pub fn format_text(&mut self, req: &FormatRequest, opts: &FormatOptions)
        -> Result<FormatReport, FormatError>;
}
```

Both return the SAME `EditReport`/`FormatReport` (disclosures, advance_delta,
followers_repositioned, tagged_mcid, glyph_source, …) the free functions
already produce — nothing in §7–§8 below changes shape — the only
difference is where the mutation lands (the session's object graph, one
undo-stack command) versus what it returns (a report, not pre-saved bytes;
`to_incremental_bytes()` is called once, later, at real Save, exactly like
every other command). **The free functions stay exactly as they are** for
`pdfce-cli`; this is an *addition*, not a breaking change. This is the
single prerequisite that gates every "Accept" affordance in this spec —
everything else here (hit-test, caret rendering, selection, preview,
formatting, disclosures) is buildable today against the shipped 14.0–14.2
read/report API; only the commit step needs this.

### 0.3 Why this tool does NOT use `CanvasTargetProvider`/`canvas_selection`

Pass 12.0's selection scaffold (`TargetId`, `canvas_selection: BTreeSet
<TargetId>`, `hit_test`/`hit_test_rect`) models **discrete-object**
selection — the right shape for Pass 9a's vector objects, each an opaque,
independently-selectable thing. Text selection is categorically different:
a single contiguous `(anchor, active)` pair of caret positions over a
continuous glyph stream, with its own navigation semantics (Left/Right steps
a glyph, not "select the next object"). Forcing it through `TargetId`/
`BTreeSet` would be a genuine type mismatch, not just an awkward fit.
**`TextEdit` gets its own parallel state** (§2), living alongside
`active_tool`/`canvas_selection` on `OpenDoc`, exactly as `GestureInterrupt`'s
own doc comment already anticipated ("Pass 6.1 replaces the query's result
with `DrawState`... Pass 7... a text-field draft flag" — this Pass supplies
its own third shape, never touching `canvas_selection`). `CanvasTargetProvider`
is untouched by this Pass; `target_provider` stays `None` until Pass 9a.

---

## 1. Tool discovery and entry

### 1.1 Placement — reuses, does not invent, the five-way taxonomy

Per `ARCHITECTURE.md`'s settled taxonomy (view-state→toolbar view group;
edit→toolbar/window; selection-scoped→rail; advanced→Tools dock;
disclosure→status bar) plus the two named-but-never-actually-built
instances from `pass-6.1`/`pass-8`'s own (still-unshipped-as-GUI) specs —
"edit, transient/tool-scoped → dedicated top panel" and "dedicated
secondary panel for document-internal multi-step review." **This Pass is
the FIRST to actually build the "dedicated top panel, tool-scoped" instance**
(6.1's markup tools shipped as a menu-driven default-rect placement with no
property bar at all — confirmed by reading `main.rs`'s `add_markup_shape`/
`GuiMarkupKind` menu, which never touches `CanvasTool`). Do not treat this
as an eighth taxonomy instance; it is the first real occupant of the sixth.

**New toolbar group, distinct from the existing Markup/Text groups**
(deliberately — decision 014 §1 draws a hard line between page-content
editing, Pass 6.2's FreeText/Stamp annotation authoring, and Pass 4
extraction; conflating them in the UI would re-introduce the confusion the
decision explicitly resolved):

```
[Open] [Save] | [Rail] [Annotations toggle] | ◄ ► [page] [zoom−][zoom+] |
[Markup ▾] [Text ▾] | [Edit Text] | ...
```

- Button: an "Aa" pencil-style icon (a real, non-color-only glyph — rule 6),
  `add_sized(ICON_BUTTON_SIZE, ...)` matching every other toolbar control's
  click-target convention (per the polish-audit finding: `selectable_label`
  without sizing is the app's one existing inconsistency — do not repeat it
  here; this control MUST use the same wrapper every neighbor uses).
- Label/tooltip (proposed `ui_text.rs` copy, engineer may adjust wording,
  not scope): `edit_text_tool_button() = "Edit Text"`;
  `edit_text_tool_tooltip() = "Edit the words already on this page — fix a \
  typo, resize, or recolor existing text. To add a NEW comment or sticky \
  note instead, use Markup or Text."` — the tooltip's second sentence is the
  disambiguator decision 014 §1 requires; do not ship this control without it.
- A selectable-style toggle (`egui::Button::selectable(active, label)`, the
  SAME widget already used for the annotation-visibility toggle — reuse,
  don't invent a third look for "is this tool active") wired to
  `Action::SelectCanvasTool(Some(CanvasTool::TextEdit))` when off,
  `Action::SelectCanvasTool(None)` when on and clicked again.
- **Keyboard chord:** assign one, but **verify no collision first** — grep
  `collect_keyboard_actions` for existing single-key/`Ctrl+`-key bindings
  (rotate `[`/`]`, `Delete`, `Ctrl+Z`/`Y`, arrow-key page nav, etc.) before
  picking. A reasonable, common-convention candidate is `Ctrl+E`; do not
  ship it without confirming it is free. This is destructive-in-the-loose
  sense that it changes page content, but per rule 7 entering/exiting the
  TOOL itself is not the destructive act (an accepted edit is, and that
  already goes through `EditSession`'s ordinary undo) — a chord is
  appropriate and expected for a repetitive power-user action, unlike Pass
  8's deliberate no-chord-on-Apply.

### 1.2 Disabled state

Grey out (not hide — there is something to discover: "this exists, here's
why you can't use it right now") when no document is open, mirroring every
other document-scoped toolbar control's existing convention. No new
disabled-state pattern needed.

---

## 2. State model

```rust
/// On OpenDoc, session-only (matches active_tool/canvas_selection's own
/// non-persisted precedent). None whenever CanvasTool::TextEdit is not the
/// active tool, or between pages.
text_edit: Option<TextEditState>,

struct TextEditState {
    /// Which page this state was built against. Rebuilt (not incrementally
    /// patched) whenever the page changes or the underlying document
    /// content changes (after an accepted edit — see §6.4).
    page_index: usize,
    /// Pass 14.0's model, built with capture_provenance = true. Rebuilding
    /// this is the one real per-entry cost this tool has — see §2.1's note
    /// on when to pay it.
    model: pdfce_core::text_edit::EditableTextModel<'static>, // engineer's
        // choice how the borrow is managed (owned PageText alongside it,
        // most likely, since EditableTextModel borrows from PageText) —
        // not dictated here; the shape (rebuilt-per-entry, not persisted
        // across app sessions) is what matters.
    /// The caret — always present once inside a text run; None only before
    /// the first click.
    caret: Option<pdfce_core::text_edit::TextPosition>,
    /// The OTHER end of a selection; None means "just a caret, no
    /// selection" (anchor == caret is likewise a collapsed/no selection).
    anchor: Option<pdfce_core::text_edit::TextPosition>,
    /// An in-progress, UNCOMMITTED edit the operator is composing —
    /// Some() is exactly this tool's "discardable gesture" for
    /// GestureInterrupt (§6.2).
    pending: Option<PendingEdit>,
    /// Whether the block-boundary review overlay (§9) is shown.
    show_block_overlay: bool,
}

struct PendingEdit {
    /// The run being edited (a selection may only span ONE run — §4.4).
    run: usize,
    /// The run's original text, for the "reject" revert and for building
    /// the eventual EditRequest::find.
    original_text: String,
    /// The operator's in-progress replacement (what the live preview draws).
    draft_text: String,
    /// The last commit ATTEMPT's outcome, if the operator has tried Accept
    /// at least once and it was refused — kept so the refusal stays
    /// visible while they revise draft_text, never silently cleared.
    last_refusal: Option<String>, // Refusal/FormatError Display text, verbatim
}
```

### 2.1 When the model is (re)built

- **On tool entry** (`Action::SelectCanvasTool(Some(CanvasTool::TextEdit))`
  while a document is open): build `TextEditState` for the CURRENT page,
  calling `text_extract::extract_pages(doc, &[page_index],
  &ExtractOptions::default().with_provenance())` (the provenance flag is
  mandatory — without it, `EditableTextModel::provenance()` returns `None`
  everywhere and §7's anchor-pinning cannot work) then
  `EditableTextModel::recognize(&page_text, &BlockRecognitionOptions::default())`.
- **On page navigation while the tool stays active:** rebuild for the new
  page (do not attempt to carry caret/selection across a page boundary —
  there is no meaningful position to carry).
- **After an accepted edit (§6.4):** rebuild for the SAME page — the
  content stream changed, so every `TextPosition`/`GlyphRef` the old model
  produced is stale. This is the cost the §0.2 core-accessor gap need not
  make worse: rebuilding the READ-ONLY model (Pass 4 extraction + Pass 14.0
  recognition) after a committed edit is unavoidable and cheap relative to
  a full incremental-save-and-reparse; it is NOT the same cost as reloading
  the whole `Document`/`EditSession`, which §0.2 specifically rules out.
- **Never per-keystroke.** The live preview (§6) never touches the model;
  it draws `draft_text` directly.

---

## 3. Click → caret

1. `image_response.clicked()` while `CanvasTool::TextEdit` is active: take
   the click's canvas-space point (already computed by the substrate,
   §1–2 of the Pass 12.0 spec), convert to PDF user-space via
   **`viewer::canvas_to_pdf_space(point, page)`** (NOT `screen_to_page` —
   `EditableTextModel::hit_test(x, y)` takes page-space coordinates over
   `PageText`'s bboxes, which are genuine PDF user-space `lly/ury/llx/urx`
   rects, confirmed by reading `model.rs`'s `hit_test`/`hit_in_line`;
   `screen_to_page` produces the OTHER, device/rotated space — using the
   wrong bridge here is exactly the kind of divergence Pass 12.0 §2.3 was
   built to prevent. This is the first real consumer of
   `canvas_to_pdf_space` since it shipped fully tested but uncalled.).
2. `model.hit_test(x, y)` → `Option<TextPosition>`. `None` (page has no
   clustered glyph at all, or the click missed every line): clear
   `caret`/`anchor`, no pending edit — the same "click empty canvas
   deselects" convention Pass 12.0 already established for object
   selection, applied to text.
3. `Some(pos)`: set `caret = Some(pos)`, `anchor = None` (a plain click
   collapses any selection) — unless a `PendingEdit` is in progress, in
   which case §6.2's gesture-interrupt rule fires FIRST (discard the
   pending edit, then process the click normally).
4. **Pre-emptive editability signal (first-cut scope note):** this Pass
   does NOT proactively caption a run as Embedded/Bundled/Supplied/
   embedded-subset-limited before the operator attempts an edit — trust-
   level disclosure fires at commit time (§8.1), reusing `EditReport`/
   `FormatReport`'s existing `disclosures` verbatim. A proactive per-run
   caption (grey the caret in an embedded-subset run, e.g.) is a clean,
   named fast-follow, not this Pass's scope — it would need a NEW
   read-only core query (font-embedded-ness at a page-space point,
   independent of attempting an edit) that does not exist today. Flagged,
   not silently deferred.

---

## 4. Selection gestures

### 4.1 Drag-to-select

Drag starting inside a text run while `TextEdit` is active (this is exactly
`canvas_suppresses_pan`'s `true` branch — the FIRST real exercise of it):
`anchor` is set on `drag_started()` (via `hit_test` at the drag's start
point), `caret` tracks `hit_test` at the current pointer position on every
frame the drag continues, and `model.resolve_range(anchor, caret)` is
recomputed each frame purely for the RENDERING half (§5) — never for a
commit, which only happens on Accept (§6).

### 4.2 Shift+click

Extends the selection from the EXISTING `caret` (which becomes `anchor` if
not already set) to the click's new hit position — the standard word-
processor convention, distinct from Pass 12.0's own Shift+click TOGGLE
semantics for object selection (§4.2 of that spec) — this is a real,
necessary divergence: text selection has no "membership," only a span. Name
this in the code's doc comment so a future reader does not assume Pass
12.0's toggle rule applies here too.

### 4.3 Double-click (word) / triple-click (line)

Both need a boundary the shipped model does not directly expose — flag to
the engineer as a small, well-motivated core addition (same "core owns the
derived structure" principle as decision 014 §4.1's whole argument for why
word/line/block recognition lives in `pdfce-core`, not reimplemented ad hoc
per consumer):

```rust
impl<'a> EditableTextModel<'a> {
    /// The Line index containing `pos` (reverse of hit_test's internal
    /// line-then-glyph walk). None only if pos.run is out of range.
    pub fn line_at(&self, pos: TextPosition) -> Option<usize>;
    /// Word boundaries around `pos`, split on Unicode whitespace within
    /// the run's own text — a DERIVED judgement (word boundaries do not
    /// exist in an untagged content stream any more than lines do, S1-S9),
    /// same honesty posture as every other derived boundary in this model.
    pub fn word_range_at(&self, pos: TextPosition) -> (TextPosition, TextPosition);
    /// The first/last caret position of the Line containing `pos`.
    pub fn line_range_at(&self, pos: TextPosition) -> Option<(TextPosition, TextPosition)>;
}
```

`line_at` is a straightforward reverse-lookup reusing the exact matching
logic `hit_test`/`hit_in_line` already contain (do not hand-roll a second,
possibly-diverging version in `pdfce-gui` — this is precisely the
duplication risk Pass 12.0 §2.3 named for geometry, now recurring one layer
up for text structure). `word_range_at`/`line_range_at` are then thin. Wire:
double-click → `word_range_at(hit)`, `(anchor, caret) = result`;
triple-click → `line_range_at(hit)`.

### 4.4 Cross-run selections — a named first-cut limit, not a silent gap

`EditRequest`/`FormatRequest` anchor to **one show operator** (`find`
matched within it, optionally pinned by `operator_span`) — there is no
multi-operator batch-edit primitive in 14.1/14.2, and Pass 14.1's own
non-goals list already names "cross-`TJ`-element matches" as refused BY
NAME. A selection whose `resolve_range` result spans more than one
distinct `GlyphRef.run` **cannot be committed as a single edit** under the
shipped core API.

**Design:** do not silently clamp or silently attempt a doomed core call
that will refuse anyway. The moment a selection's covered glyphs span >1
run (checked client-side, cheaply, on every selection-extending frame),
show a small inline notice at the selection's trailing edge — proposed copy
`ui_text::cross_run_selection_notice() = "This selection spans more than \
one text run (e.g., a formatting change partway through) — pdfce's \
first-cut editor edits one run at a time. Narrow the selection to edit or \
format it."` — and disable Accept (§6) for that selection. This is a
refusal-with-reason, exactly rule 4/§4 of the task's ask, delivered BEFORE
a wasted core round-trip rather than after. Typing while such a selection is
active does nothing (no `PendingEdit` is created) rather than silently
replacing only part of the selection.

### 4.5 Arrow / Home / End caret navigation

- **Left/Right:** step to the adjacent glyph boundary within the current
  Line's `glyphs` order (`Line.glyphs: Vec<GlyphRef>` is public — walk it
  directly); at a line's start/end, move to the adjacent Line's end/start
  (`Block.line_indices`/`EditableTextModel::lines()` give the ordering).
  This is ordinary GUI-side traversal over already-public model fields —
  no new core accessor needed here, unlike §4.3's word/line-boundary
  lookups (which needed the SAME matching logic `hit_test` already encodes
  internally, not merely public data to walk).
- **Home/End:** `line_range_at(caret)`'s two ends (§4.3's new accessor).
- **Up/Down:** find the Line immediately above/below in the SAME column
  (`Line.column`/`Line.block` public fields), then re-hit-test at the
  current caret's approximate x within that line (`model.hit_in_line`'s
  logic is private, but `hit_test(x, target_line.baseline_y)` achieves the
  same result through the public API — reuse it rather than reimplementing
  nearest-glyph matching a third time). Shift held extends the selection
  exactly as a mouse-driven Shift+click would (§4.2).
- All of the above are plain keyboard input consumed ONLY while the canvas
  has focus AND `TextEdit` is active — this is exactly the "focused-widget
  key consumption" reconciliation Pass 12.0 §6.3 flagged as "explicitly
  Pass 7's problem." **If Pass 7's reconciliation has not landed yet when
  this Pass is built, it becomes THIS Pass's problem too** (a global
  `collect_keyboard_actions` that reads `ctx.input()` unconditionally every
  frame cannot safely also bind arrow keys to page navigation elsewhere) —
  verify which is true at implementation time and flag accordingly; do not
  silently assume Pass 7 solved it.

---

## 5. Caret + selection rendering in canvas space

Reuses Pass 12.0 §5's live-preview overlay mechanism verbatim — paint
directly via `ui.painter()` on top of the already-rasterized texture, never
a re-raster:

1. For each frame `TextEdit` is active with a caret set: convert the
   caret's PDF-space position to canvas space via
   **`pdf_space_to_canvas(pos, page)` then `page_to_screen(.., image_rect,
   extent, zoom)`** (the exact inverse chain of §3's forward path — the
   two Pass-12.0 bridges composed, exactly as their own docs anticipated:
   "a hit-tested object's bounds ... projected to screen via
   `page_to_screen(pdf_space_to_canvas(bounds, page), ..)`" — this Pass is
   the first real consumer of that composition too). Draw a 1–2px vertical
   line spanning the glyph's ascent-to-descent box, blinking on the
   standard OS-independent interval `egui` already provides for `TextEdit`
   widgets elsewhere in the app (reuse, don't hand-roll a blink timer).
2. For a non-empty selection (`anchor.is_some()`): `resolve_range(anchor,
   caret)` → per-glyph boxes → project each the same way → paint a single
   translucent highlight rectangle spanning their union per line (multiple
   rects for a multi-line selection, one per line, never one box spanning
   the inter-line gap).
3. **Shape/pattern, not color alone (rule 6):** the caret is a real line
   (not a color change to existing content) and the selection highlight is
   paired with the existing status-line disclosure the moment it spans
   >1 run (§4.4) — satisfies "color never the sole signal" the same way
   Pass 12.0 §4.3's selection-outline precedent already does for objects.

---

## 6. Live preview, accept/reject — the operator-confirmed edit

### 6.1 Entering compose mode

Typing (any character key, Backspace, Delete) while a caret or a
single-run selection exists creates or extends `PendingEdit` if none
exists: `original_text` = the target run's full text (`TextRun.text`, via
`model.sourced_view().runs[run].text` — public field, no new accessor
needed), `draft_text` = `original_text` with the typed edit applied at
the caret/selection's byte offsets. Every subsequent keystroke updates
`draft_text` only — **no core call happens per keystroke.**

### 6.2 The pending edit IS the tool's `GestureInterrupt`

`TextEditState.pending.is_some()` is exactly this tool's answer to
`current_gesture_interrupt()` (Pass 12.0 §3.3's one enforcement point).
**Policy: `GestureInterrupt::Discard`, the SAME class as Pass 6.1's
half-drawn shape, NOT Pass 7's Commit-policy form-field draft.** This is a
deliberate, defensible call: an uncommitted text edit is, by rule 4's own
framing, a reviewable draft the operator has explicitly not yet accepted —
discarding it on an unrelated interrupt (Undo, Save, page nav, opening a
dock panel) loses nothing that was ever written anywhere, exactly rule 7's
"no unnecessary friction for a reversible, low-stakes action" (there is
nothing to be friction-ful ABOUT; nothing happened yet). Pass 7's Commit
policy is right for its own case (a form value typed with clear intent to
keep) precisely because form-fill has no separate "Accept" gesture the way
this tool does — text-edit's whole design IS the separate accept gesture,
so Discard-on-interrupt is what makes "operator-accepted, never silent"
actually mean something.

### 6.3 The preview render itself — same visual language as redaction's marked-vs-applied split

Never re-raster to show `draft_text`. Instead, per frame with a
`PendingEdit`:

1. Paint a translucent mask over the ORIGINAL run's projected bbox (so the
   old glyphs don't visually collide with the new ones — a temporary
   paint-layer effect, never a raster edit).
2. Draw `draft_text` as plain `painter.text()` using an egui built-in font
   at a size approximated from the run's `Tf`/zoom (NOT the document's own
   font — real glyph shapes only exist after a real commit triggers a real
   re-render, exactly as decision 014 §4.7 accepts no new dependency and
   this Pass adds no font-shaping capability of its own).
3. **Dashed border + a small corner tag reading "PREVIEW — not yet
   applied"** around the affected region. This deliberately reuses Pass
   8's own established convention (hatch pattern + "MARKED" tag for
   pre-apply redaction, distinguished sharply from the seamless post-apply
   result) — same principle (a draft state must never be visually
   confusable with the committed result), same "never the same code path"
   discipline, applied to a different operation. Do not invent a fourth
   visual language for "this isn't real yet" when the app already has one.

### 6.4 Accept / Reject

A small, non-modal control pair appears anchored near the pending edit's
bounding box (floating, not a blocking dialog — this is a frictionless,
undoable-up-to-save action per rule 7, not a destructive one requiring
Pass 8's confirmation weight):

- **Accept** (proposed: `Enter` key, or a small ✓ button) — calls
  `EditSession::edit_text(EditRequest::find_replace(page_index,
  &original_text, &draft_text).with pinned_span from
  model.provenance(GlyphRef::new(run, 0)).map(|p| p.operator_span))`
  (§0.2's required core addition). Two outcomes:
  - **`Ok(report)`:** clear `pending`; rebuild `TextEditState.model` for
    the page (§2.1); surface `report.disclosures` verbatim in the
    disclosure strip (§8); re-place the caret at the edit's new end
    position (best-effort — an exact re-derivation of the post-edit
    `TextPosition` from `report.advance_delta`/the new run text is a
    reasonable engineer call, not dictated further here).
  - **`Err(e)`:** do NOT clear `pending` — set
    `pending.last_refusal = Some(e.to_string())` (the `Refusal`/
    `EditError` `Display` impl already produces the exact verbatim R-INV
    message) and keep the preview + Accept/Reject controls on screen so
    the operator can revise `draft_text` and retry, or Reject. **A
    refusal is never a dead end** — §8.2 designs the "what would lift it"
    framing this must carry.
- **Reject** (proposed: `Esc`, which ALSO satisfies Pass 12.0 §3.5's
  Escape precedence step 1 "cancel the gesture, stay in the tool" — no new
  Escape semantics needed, this tool's gesture literally IS the thing step
  1 already describes) or a small ✕ button — clears `pending` with no
  core call, reverting to the plain caret/selection.

---

## 7. Formatting controls — the property bar (the sixth taxonomy instance, first real build)

Shown only while `TextEdit` is active AND a selection spans exactly one run
(§4.4) — a "dedicated top panel" per the taxonomy, appearing/disappearing
with the tool exactly like the never-shipped Pass 6.1 property bar was
designed to (reuse that placement idea; this Pass is what actually builds
an instance of it):

```
┌─ Edit Text ──────────────────────────────────────────────────┐
│ Size: [ 12.0 ▲▼] pt   Color model: (○ RGB ○ CMYK ○ Gray)      │
│ [ swatch/sliders per model ]   Font: [ dropdown: page fonts ▾]│
└────────────────────────────────────────────────────────────────┘
```

- **Size** — `egui::DragValue` bound to a local float, committed on
  Enter/focus-loss via `EditSession::format_text(FormatRequest::new(..)
  .size(pt))`. Same accept/refuse/disclosure path as §6.4 (no separate
  mechanism — `FormatOutcome`/`FormatError` are shaped identically to
  `EditOutcome`/`EditError`).
- **Color model** — three real radio-style controls (`ui.selectable_value`
  or explicit labeled buttons — NOT a single dropdown defaulting to one
  value), because the MODEL choice (RGB/CMYK/Gray) is itself the
  parity-plus disclosure surface: pdfce stores whichever the operator
  picks, never force-converting like Acrobat. Label this plainly:
  proposed `ui_text::format_color_model_label() = "Store color as:"`, each
  option's tooltip stating the operator (`rg`/`k`/`g`) so this reads as a
  real, meaningful choice, not decoration. Component sliders/spinners
  below change shape with the model (1 for Gray, 3 for RGB, 4 for CMYK) —
  reuse `egui::color_picker` widgets where the model is RGB (a real color
  swatch is more usable there); Gray/CMYK get plain numeric sliders since
  `egui` has no native CMYK widget (this is a new but small piece of UI,
  not a new interaction pattern).
- **Font family/style** — a `ComboBox` populated ONLY from the page's
  actual `/Resources /Font` entries (per 14.2's scope: "an existing page
  font resource... no new embedding") — never a system-font list that
  would silently promise more than pdfce can do. Each entry's label states
  its trust level using the SAME three-way vocabulary as the disclosure
  strip: `"Times-Bold (embedded)"` / `"Helvetica (bundled)"` /
  `"Calibri (supplied — --font-dir)"` — computed via
  `self.font_env`/`self.font_folders` (already on `PdfceApp`, built for
  decision 012's Font-folders tool) checked against the target's stripped
  base-font name. **Reuse, do not re-derive, the subset-stripping logic**:
  `pdfce-cli`'s `font_subset_stem` (`crates/pdfce-cli/src/main.rs`) already
  does exactly this string operation for the identical CLI-side
  Bundled-vs-Supplied classification (§0's "shell" refinement, per Pass
  14.1's judgment call #1: core reports Embedded/NonEmbedded only, the
  shell refines NonEmbedded). **Flag to the engineer:** hoist this helper
  somewhere both `pdfce-cli` and `pdfce-gui` can call (e.g. a method on
  `pdfce_render::FontEnvironment` itself, which already owns the registry
  being consulted) rather than let a second copy of the same six-letter-
  subset-tag-stripping regex drift between the two shells — a small,
  cheap fix now versus a real future divergence risk.
- Coverage failure (the target font can't render every selected character)
  is refused-and-disclosed exactly per §6.4/§8.2 — the ComboBox selection
  itself is not blocked from being MADE, only from being ACCEPTED, so the
  operator can see the option, try it, and get a named reason if it fails,
  rather than a font silently missing from the list with no explanation.

---

## 8. Disclosure surfacing — the honesty surface

### 8.1 The disclosure strip (accepted edits)

A slim, non-modal strip anchored at the bottom of the property bar (§7) —
NOT the global status bar, which is already at real risk of unbounded
height (a confirmed prior finding: the status bar can legitimately stack
8+ simultaneous lines). Text-edit disclosures are per-edit and
tool-scoped, so they belong with the tool, not the document-wide status
surface. On every successful Accept (§6.4) or format commit (§7):

```
┌─ Last edit ──────────────────────────────────────────────────┐
│ ⓘ font: 'Helvetica' is NON-embedded; a bundled Base-14        │
│   substitute renders the edited glyphs...                     │
│ ⓘ save: this edit was written INCREMENTALLY (R34/R70); the    │
│   prior text survives in the document's revision history...   │
│   To truly remove content, use redaction — a distinct,        │
│   security operation.                                         │
│ ⚠ relayout: the edited line was shifted and MAY now overflow  │
│   the original right margin; block re-wrap is deferred...     │
└──────────────────────────────────────────────────────────────┘
```

**Render `EditReport.disclosures`/`FormatReport.disclosures` VERBATIM,
one bullet per string, in commit order.** These are already
operator-facing, already reviewed prose (see `edit.rs`/`format.rs`'s
`trust_disclosure`/`disclosure_save`/`disclosure_size`/`disclosure_fill`/
`disclosure_narrowing`/`disclosure_reflow`/`disclosure_tagged` functions —
this Pass does not re-author or paraphrase a single word of them; it is a
pure render). Use ⓘ for informational disclosures and ⚠ for ones naming a
real limitation (overflow-not-reflowed, colour-narrowing, tagged
staleness) — pair the icon with the text, never rely on a border color
alone (rule 6). Persist the LAST edit's disclosures until the next
accepted edit or tool exit — do not auto-dismiss after N seconds (an
operator reading slowly must not have the disclosure vanish mid-read).

### 8.2 Refusals — never a dead end

On `Err` from `EditSession::edit_text`/`format_text` (§6.4), render the
`Display` text (already verbatim, e.g. `"R-INV-1: character U+0064 'd' \
does not already carry on this page's embedded SUBSET of 'ABCDEF+Calibri' \
— Acrobat's own 'embedded-but-not-local' floor..."`) in the SAME strip,
styled distinctly (a warning-colored border + a ✖ glyph, never color
alone) from a success disclosure, directly beneath the still-visible
Accept/Reject controls. **Every refusal message pdfce's core already
produces states the cause; pair it in the UI with one more sentence
naming what would lift it, using this fixed mapping** (proposed
`ui_text` entries, one per `RInvTrigger`/`FormatError` family — the
engineer may adjust exact wording, the MAPPING is the binding part):

| Trigger | "What would lift it" framing to append |
|---|---|
| R-INV-1 (embedded-subset floor) | "Supply this font via a font folder (Tools → Font folders) so pdfce can use its full character set, or keep this edit to characters already on the page." |
| R-INV-2/3/4 (symbolic/ToUnicode-only/composite) | "This font's encoding can't be safely inverted for character-level editing — not yet supported for this font type." |
| R-INV-5 (ambiguous, soft) | (already a disclosure, not a refusal — no "lift it" framing needed, it already applied the edit) |
| R-INV-6 (ligature-only) | "This character exists only as part of a ligature in this font — try a font where it has its own glyph." |
| R-INV-7 (code occupied) | (rare, internal encoding conflict) "This exact substitution isn't representable in this run's encoding." |
| R-INV-8 (beyond repertoire) | "This character is outside what a simple (non-Unicode-wide) font can address here." |
| `FormatError::CoverageFailure` | "Choose a font that includes every character in this selection, or supply one via Tools → Font folders." |
| `FormatError::TargetFontMissing` | "This page has no resource for that font — pdfce edits existing page fonts only in this first cut; embedding a new one is a planned fast-follow (FF-C)." |
| Cross-run selection (§4.4) | "Narrow the selection to one text run." |

This table is the concrete instantiation of the task's "here's why, and
what would lift it (supply the font / FF-C planned), never a dead-end"
requirement — do not ship a refusal with only the core's own cause text
and no next-step sentence.

### 8.3 Tagged-run staleness

`disclosure_tagged`'s string already fires automatically whenever
`report.tagged_mcid.is_some()` — no separate UI state needed; it appears
in the same strip (§8.1), worded by core, not re-derived by the GUI.

---

## 9. Block-boundary review overlay

### 9.1 Scope call for THIS Pass — read-only visualization, split/merge/reorder deliberately NOT wired

Decision 014's task framing describes the block model as operator
split/merge/resize/reorder-able; the shipped 14.0 core, however, has **no
persistence mechanism for an operator's correction** — `BlockRecognitionOptions`
is four GLOBAL threshold ratios (affecting the whole page's recognition
pass, not one specific misjudged paragraph), and no consumer of the block
model exists yet that a correction would even affect (single-line
edit/format doesn't consult `Block`; reflow, which WOULD care, is FF-A,
deferred). **This Pass ships the overlay as toggleable, read-only
visualization only** — the honest, buildable slice — and names
split/merge/resize/reorder as deferred rather than half-building
uncommitted-nowhere-to-go controls:

- Toggle: `show_block_overlay: bool` on `TextEditState`, a small toolbar
  icon within the property bar (§7) — proposed
  `ui_text::block_overlay_toggle_tooltip() = "Show/hide the recognized \
  paragraph and column boundaries pdfce inferred (not stated by the PDF \
  itself — a reviewable hint)."`
- When on: outline each `Block.bbox` (dashed, distinct from the caret/
  selection's solid rendering — a different visual vocabulary for "this is
  a structural guess" versus "this is your cursor") projected via the same
  `pdf_space_to_canvas`→`page_to_screen` chain as §5. Hovering a block
  shows its `BlockDiagnostics` counts in a tooltip (lines/columns/blocks
  recognized) — the "counted, not hidden" half of rule 4, made visible
  without a click.
- **Do not add split/merge/resize/reorder buttons this Pass.** State this
  explicitly in the shipped code's module doc (per the project's own
  "state the absence explicitly" convention, e.g. Pass 7 §7/Pass 12.0
  §7) — a future reader must see this was a deliberate scope line, not an
  oversight, and see exactly what unblocks it (a core persistence API +
  FF-A's reflow actually consuming a correction).

---

## 10. Keyboard / accessibility

- **Discoverability:** toolbar button + tooltip (§1.1); no hidden-only
  entry point.
- **Keyboard access:** full caret/selection navigation is keyboard-driven
  (§4.5); Accept/Reject both have keyboard bindings (Enter/Esc, §6.4);
  Tab order is unaffected beyond what Pass 12.0 already resolved (the
  canvas itself is the Tab stop; this tool doesn't add new Tab-reachable
  widgets to the canvas itself, only the property bar, which is an
  ordinary `egui` panel already in the Tab chain like the Tools dock).
- **Color never sole signal:** caret is a real line; selection highlight
  is paired with the cross-run notice (§4.4) when relevant; preview vs.
  accepted state is dashed-border + text-tag, not tint alone (§6.3);
  disclosure vs. refusal uses icon + text, not border color alone (§8).
- **Click targets:** Accept/Reject buttons and the toolbar tool-button all
  use the app's existing `ICON_BUTTON_SIZE`/`add_sized` convention — do
  not introduce a smaller ad hoc hit target for the floating Accept/Reject
  pair just because it's non-modal.
- **`accesskit`/screen-reader gap (named, not solved here):** the canvas
  is still a raster image with no text alternative; this Pass's caret/
  preview are painter-drawn overlays, not real `egui` widgets, so they
  carry no `accesskit` exposure either — consistent with, not a new
  instance beyond, the standing gap Pass 12.0 §6.4 already named. Flag
  for a future pass: the property bar's OWN controls (DragValue, radio
  buttons, ComboBox, the Accept/Reject buttons if built as real
  `ui.button`s rather than painter-drawn) DO get real accesskit exposure
  for free, per Pass 12.0 §6.4's "prefer real egui widgets" recommendation
  — build Accept/Reject as real `ui.button`s anchored via `egui::Area`,
  NOT painter-drawn, specifically to bank this win (unlike the caret/
  selection/preview-text, which have no sensible "real widget" form).
- **Crash-safe autosave:** an in-progress `PendingEdit` is never written
  anywhere (§6.1–6.2) — a crash mid-composition loses only the
  uncommitted keystrokes, the same low-stakes loss as a half-drawn Pass
  6.1 shape or a half-typed Pass 7 field before ITS OWN commit point. No
  new autosave work is needed for the compose phase; verify the existing
  autosave cadence picks up promptly after each Accept (§6.4), same as
  every other `EditSession` command — this Pass does not change that
  cadence, only adds another kind of command to it (once §0.2 lands).

---

## 11. `ui_text.rs` catalog (new entries, proposed wording — engineer may adjust text, not presence)

- `edit_text_tool_button`, `edit_text_tool_tooltip` (§1.1)
- `cross_run_selection_notice` (§4.4)
- `format_color_model_label` (§7)
- `block_overlay_toggle_tooltip` (§9.1)
- One string per §8.2 "what would lift it" row (8 R-INV rows + 2
  FormatError rows + 1 cross-run row = 11 entries) — naming convention
  e.g. `r_inv_1_hint()`, `format_coverage_failure_hint()`, etc.
- Disclosure strip section headers: `disclosure_strip_title() = "Last \
  edit"`, `refusal_strip_title() = "Not applied"` — no new wording for the
  disclosure/refusal BODIES themselves (§8.1/§8.2 render core's own
  strings verbatim — do not add `ui_text` entries that would duplicate or
  paraphrase them).

---

## 12. Undo / write-path summary

| Operation | `EditSession` command? | Writes anything? |
|---|---|---|
| Tool entry/exit, page nav while tool active | No | No — model rebuild is a read-only re-extraction |
| Click/drag/shift-click/double/triple-click (caret+selection) | No | No — view-state only, exactly `canvas_selection`'s existing precedent |
| Typing while composing (`PendingEdit` update) | No | No — draft state only |
| Accept (§6.4), successful | **Yes — ONE command** (§0.2's `EditSession::edit_text`/`format_text`) | Nothing until real Save (incremental, per R70) |
| Accept, refused | No | No — `pending` stays, unchanged, for revision |
| Reject / Esc (§6.4) | No | No — `pending` discarded |
| Block-overlay toggle (§9) | No | No — view-state only |

Exactly one new class of undo-able command enters `EditSession`'s existing
stack (once §0.2 ships) — everything else in this Pass is view-state,
matching Pass 12.0's own §8 table precedent line for line.

---

## 13. Priority table

| Item | Priority | Note |
|---|---|---|
| §0.2 `EditSession::edit_text`/`format_text` (core addition) | **P0 — gates everything else committing** | Not a nicety; the free functions alone cannot back an interactive Accept without breaking the undo model |
| `CanvasTool::TextEdit` variant, toolbar entry, tool-active wiring (§0.1, §1) | **P0** | |
| `TextEditState`, model (re)build on entry/page-nav/post-edit (§2) | **P0** | |
| Click→caret via `canvas_to_pdf_space`→`hit_test` (§3) | **P0** | First real use of that bridge |
| Drag/shift-click selection + rendering (§4.1–4.2, §5) | **P0** | |
| `line_at`/`word_range_at`/`line_range_at` (core addition, §4.3) | **P0** | Needed for double/triple-click |
| Cross-run-selection detection + notice (§4.4) | **P0** | The load-bearing refusal-before-core-call case |
| Arrow/Home/End navigation (§4.5) | **P1** | Usable without it (mouse-only), materially worse without |
| Live preview + Accept/Reject + GestureInterrupt wiring (§6) | **P0** | |
| Property bar: size/color-model/font (§7) | **P0** | 14.2's whole capability is otherwise unreachable from the GUI |
| Disclosure strip (§8.1) | **P0** | Rule 4/2's honesty surface — non-negotiable per every prior spec's own precedent |
| Refusal strip + "what would lift it" table (§8.2) | **P0** | Same |
| `font_subset_stem`-equivalent hoisted to a shared location (§7) | **P1** | Cheap now, real divergence risk later |
| Block-boundary review overlay, READ-ONLY (§9) | **P1** | Named non-goal (split/merge/reorder) makes the cut line explicit |
| Pre-emptive per-run trust captioning before an edit attempt | **Explicitly deferred — fast-follow** | Needs a new core query; not this Pass (§3 item 4) |
| Real-`egui`-widget Accept/Reject (accesskit win) | **P1** | Cheap, banks Pass 12.0 §6.4's own recommendation |

---

## 14. Open items for the librarian

1. **§0.2 is a genuine, load-bearing core-API gap** discovered while
   designing this Pass, not named in decision 014's original record —
   worth a one-line addition to that decision or the next
   `ARCHITECTURE.md`/`ROADMAP.md` touch, exactly the "found and closed
   here, not overlooked" convention Pass 12.0 §10 already set for its own
   `canvas_to_pdf_space` finding.
2. **§4.3's `line_at`/`word_range_at`/`line_range_at`** are a second,
   smaller core addition in the same "core owns the derived structure"
   spirit as decision 014 §4.1's whole argument — flag alongside §0.2, not
   as a separate future decision.
3. **§7's `font_subset_stem` duplication** between `pdfce-cli` and the new
   `pdfce-gui` consumer — flag as a small hoist-to-`pdfce-render` cleanup,
   cheap now, worth doing in the SAME Pass rather than letting a second
   copy exist even briefly.
4. **This Pass is the first real inhabitant of `CanvasTool`** — the
   `MarkupTool`→`CanvasTool` rename note Pass 12.0 §10 already flagged for
   Pass 6.1/8's eventual GUI slices still applies; this Pass does not
   change that pointer, just confirms Pass 6.1/8's canvas-tool GUI still
   had not landed as of this spec's authoring (`canvas.rs` read directly,
   confirmed uninhabited).
5. **Block split/merge/resize/reorder is a named non-goal for THIS Pass**
   (§9.1), gated on a future core persistence API + FF-A actually
   consuming a correction — worth tracking as a fast-follow alongside
   FF-A itself rather than as its own independent backlog item.
