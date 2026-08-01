# Pass 6.1 UI Spec — Markup Annotation Authoring

> Authored by `pdfce-ui-specialist`, 2026-07-31, on dispatch from the
> engineer/orchestrator. This is the implementation spec for Pass 6.1's
> drawing/authoring GUI surfaces; the engineer implements it verbatim,
> deviating only with a recorded reason (per this project's own
> Pass 3.2/Pass 4 spec convention — deviations get named, not silent).
>
> Read: `crates/pdfce-gui/src/{main.rs, ui_text.rs, viewer.rs}` in full;
> `.claude/agents/pdfce-ui-specialist.md`; `docs/ui_specs/pass-3.2-page-ops.md`
> §1–3; `docs/ui_specs/pass-4-text-extraction.md` §4 (the already-written,
> never-shipped canvas text-selection design — reused here, not
> re-derived); `docs/ARCHITECTURE.md` §12 continuation-19/20/23 (the
> placement-taxonomy history and its final five-way form); `docs/ROADMAP.md`
> Pass 6.1 entry; `crates/pdfce-core/src/{signature.rs, annot.rs}`.
>
> **This spec does not cover:** selecting, moving, resizing, or
> recoloring an *already-placed* annotation. Pass 6.1 is authoring-only —
> the operator draws new markup; Undo is the correction mechanism for a
> mistake, exactly as it already is for every other edit in this crate.
> A post-hoc annotation-editing model (click an existing annotation,
> drag a handle, reopen its property bar) is real, wanted, out of scope
> here, and should be filed as a Pass 6.1 follow-up bucket if not already
> tracked.

---

## 0. Scope decided in this spec — read this first

| Bucket | Contents | Ships this Pass? |
|---|---|---|
| **P0** | Ink, Square, Circle, Line, Polygon, PolyLine — full tool-mode state machine, live preview, one `EditSession` command per completed shape, the property bar (color/width/opacity/fill), the full keyboard map, draw-time certification refusal (conservative), save-time signature-impact reuse | **Yes, required** |
| **P1** | Quad-point family (Highlight/Underline/StrikeOut/Squiggly) via a **rectangle-marquee fallback** — NOT glyph- or text-selection-aware | Ship if the two named core dependencies (§3.5) land in time; otherwise ship the Insert/Split-style CLI-placeholder (§3.6) and carry the GUI slice forward |
| **P2 — explicit follow-up, not this Pass** | (a) Glyph-accurate, text-selection-driven quad-point generation — reuses `pass-4-text-extraction.md` §4's design **in full**, unchanged; (b) post-hoc selection/editing of already-placed annotations; (c) Shift-constrain (square/circle/45°-line), multi-stroke Ink, per-subtype "current pen" memory | **No** |

If P1 does not make the Pass, **ship the P0 shape tools alone** — this
mirrors exactly how Pass 3.2 shipped Merge in the GUI and named Split/
Insert as CLI-only placeholders in the same dock, rather than blocking
the whole Pass on every bucket landing simultaneously.

---

## 1. Tool-mode data model & state machine

### 1.1 New state (on `OpenDoc`, session-only — not persisted, matching
`rail_expanded`/`tools_open`'s existing precedent)

```rust
/// Which drawing tool is active, if any. `None` = ordinary view/pan mode —
/// there is no separate "annotation select" tool this Pass (§ header note).
active_tool: Option<MarkupTool>,

/// The in-progress shape, if any. Cleared on commit, cancel, or any of the
/// auto-cancel triggers in §1.5. Lives here (not as an `Action`) for the
/// same reason `dragged_page`/`drop_target` do in the existing rail
/// drag-reorder code: per-frame pointer state that only produces ONE
/// discrete `Action` on completion.
draw_state: DrawState,

/// The "current pen" — persists across shapes within a session, exactly
/// like a paint program's active tool settings. NOT part of `EditSession`;
/// changing it is not an edit, only USING it to complete a shape is
/// (mirrors the Pass 3.1 properties-draft rule: typing/adjusting a
/// control is not an edit; committing is).
markup_color: egui::Color32,
markup_width: f32,      // points; stroke-based tools only
markup_opacity: f32,    // 0.0–1.0
markup_fill: Option<egui::Color32>,  // Square/Circle only; None = no fill
```

### 1.2 `MarkupTool`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MarkupTool {
    Ink, Square, Circle, Line, Polygon, PolyLine,
    Highlight, Underline, StrikeOut, Squiggly,
}
impl MarkupTool {
    fn is_quad_point(self) -> bool { matches!(self, Self::Highlight | Self::Underline | Self::StrikeOut | Self::Squiggly) }
    fn has_stroke(self) -> bool { !matches!(self, Self::Highlight) }        // every quad tool except Highlight draws a line
    fn has_fill_option(self) -> bool { matches!(self, Self::Square | Self::Circle) }
    fn is_multi_click(self) -> bool { matches!(self, Self::Polygon | Self::PolyLine) }
}
```

### 1.3 `DrawState`

```rust
#[derive(Debug, Clone)]
enum DrawState {
    Idle,
    /// Ink: continuously appended while the mouse button is held.
    /// Points are in PAGE-SPACE (§2.1) so they stay correct across a
    /// zoom change mid-... — see §2.1's note on why this is stored this
    /// way rather than in screen pixels.
    Ink { points: Vec<egui::Pos2> },
    /// Square/Circle/Line/quad-rectangle: one anchor, one live corner.
    Dragging { anchor: egui::Pos2, current: egui::Pos2 },
    /// Polygon/PolyLine: committed vertices plus the live rubber-band
    /// point tracking the pointer.
    Polygon { vertices: Vec<egui::Pos2>, cursor: egui::Pos2 },
}
```

### 1.4 Entry / exit / cancel

- **Enter a tool**: `Action::SelectMarkupTool(Some(tool))` — from the
  toolbar menu (§6.1) or its keyboard chord (§6.2). Sets `active_tool`,
  resets `draw_state = Idle`. Entering a tool while another is active
  **discards** any in-progress shape first (§1.5) — free, because
  nothing has committed yet (rule 7: no friction for a reversible,
  zero-cost action).
- **Exit to view mode**: `Action::SelectMarkupTool(None)` — clicking the
  active tool's own row again, or the toolbar's ✕ pill, or `Esc` (§6.2).
  Also discards any in-progress shape.
- **Cancel without exiting the tool**: `Esc` while a shape is mid-drag or
  mid-polygon discards the in-progress shape but **stays** in the same
  tool, ready to start another one — this matches how a real drawing
  tool behaves (aborting one stroke doesn't mean "put the pencil down").
  Only a second `Esc` (now with `draw_state == Idle`) exits to view mode.
  This two-stage Escape is a genuine, deliberate UX decision, not free —
  flag it to the operator via the property-bar hint text (§4.2) so it's
  discoverable rather than surprising ("Esc: cancel this shape · Esc
  again: stop drawing").

### 1.5 Auto-cancel triggers — the general rule

**Any action that is not itself part of continuing, cancelling, or
committing the in-progress shape silently discards it first, with no
confirmation, then proceeds.** Concretely: page navigation
(`PrevPage`/`NextPage`/`GoToPage`/a rail thumbnail click), `Undo`/`Redo`,
`Save`, opening Properties or the Tools dock, all fall through to this
rule.

This is deliberately the **opposite polarity** from the pending-
confirmation gate already in `apply()` (§ "The pending-confirmation
gate" doc comment): that gate *blocks* everything else because the
operator is being asked a real yes/no question with a real consequence.
An in-progress shape has no consequence at all — nothing has been
written to the `EditSession` — so blocking navigation while a rectangle
is half-drawn would trap the operator for no reason; silently discarding
it and letting the requested action through is strictly friendlier and
is exactly rule 7's "no unnecessary friction for reversible, low-stakes"
half (there is nothing *to* reverse — it never existed as a fact about
the document).

One enforcement point, in `apply()`, mirroring the pending-gate's own
"one place, not a dozen" reasoning:

```rust
// at the top of apply(), before the pending-confirmation gate or after —
// order doesn't matter, the two gates are orthogonal
if !matches!(action, Action::SelectMarkupTool(_) | Action::CancelDraw
                    | Action::MarkupPointerEvent(_) | Action::CommitMarkup(_))
{
    if let Status::Open(doc) = &mut self.status {
        doc.draw_state = DrawState::Idle; // no-op if already Idle
    }
}
```

**Special case — page navigation specifically.** Vertices/points are
stored in page-space (§2.1) of *whatever page was current when drawing
started*. Navigating away mid-shape doesn't just interrupt the gesture,
it invalidates the coordinate system the in-progress geometry is
expressed in relative to what's on screen — so this is not merely a
convenience discard, it is a correctness requirement, and must not be
skipped even if a future engineer is tempted to "preserve" the
in-progress shape across a page turn.

---

## 2. Canvas interaction

### 2.1 Geometry — reuse `pass-4-text-extraction.md` §4.1, don't rebuild it

That spec already designed and justified `viewer::screen_to_page(pos,
image_rect, extent, zoom) -> Pos2`, built from exactly the geometry
`viewer.rs` already exposes (`doc.current_extent()`, `doc.view.zoom`,
the `Image` response's `rect`). **Add it now, for this Pass** — Pass 6.1
is what actually needs it first (text-selection, the feature that
originally motivated it, is still deferred). Add its natural inverse
alongside it:

```rust
/// Companion to `screen_to_page` — needed here because, unlike the
/// deferred text-selection slice (which only ever *reads* screen
/// position to find a `(run, offset)`), a live shape preview must
/// project PAGE-space geometry back to screen space every frame to draw
/// the in-progress outline.
pub fn page_to_screen(page_pt: Pos2, image_rect: Rect, extent: (f32, f32), zoom: f32) -> Pos2
```

**Why points are stored in page-space, not screen-space:** a zoom change
(ctrl+scroll) must not corrupt an in-progress Ink stroke or Polygon's
already-placed vertices — those are facts about *where on the page* the
operator clicked, and should reproject correctly at any zoom. Only the
*rendering* of the live preview needs a screen-space value, recomputed
fresh via `page_to_screen` every frame. This is the same reasoning
pass-4 §4.1 already gives for glyph hit-testing; it applies identically
here.

**Same open question carries over, and is now load-bearing sooner:**
pass-4 §4.1 flagged confirming whether `/Rotate` is baked into the
extent-vs-glyph coordinate systems consistently. For this Pass the
question is simpler and must be resolved regardless: `current_extent()`
already has `/Rotate` applied (§ `viewer::page_extent_pts`'s own doc
comment says so), so `screen_to_page`/`page_to_screen` built against it
are automatically rotation-correct for *drawing* — there is no glyph
coordinate system involved here at all, so Pass 6.1 does **not** inherit
pass-4's open question. State this explicitly in the code comment so a
future reader doesn't assume the two slices share the same caveat.

### 2.2 Making the canvas focusable — this is the Pass that actually
triggers it, not the deferred text-selection slice

`main.rs`'s own module docs have carried this caveat since Pass 1:
*"Revisit this when the canvas gains focusable content — text
selection, form fields."* Pass 6.1 is the first Pass to make that true.
Resolve it here:

- Change the canvas image `Response` to carry `Sense::click_and_drag()`
  (an `Image` alone is not interactive).
- On click/drag-start, `ui.memory_mut(|m| m.request_focus(id))` so Tab
  reaches the canvas at the position `pass-3.2` §1 already reserved for
  it (toolbar → rail → dock → canvas — order unchanged, the canvas
  simply now participates in it instead of being a dead stop).
- **Update `main.rs`'s module doc** to close this caveat and note it was
  resolved by Pass 6.1, not by the text-selection slice that originally
  named it — the engineer should build the interactive-canvas plumbing
  generically enough (`canvas_response.dragged()`/`clicked()` dispatched
  to whichever mode — draw tool vs. eventual text-selection — is
  currently active) that the deferred slice can layer onto the *same*
  focusable widget later rather than re-doing this step.

### 2.3 Cursor, and the drag-vs-pan conflict

While `active_tool.is_some()`:
- Cursor over the canvas becomes `egui::CursorIcon::Crosshair` (via
  `ui.output_mut(|o| o.cursor_icon = …)` scoped to the canvas response's
  hover), reverting to the default arrow when `active_tool` is `None`.
  This is the single most important "is a mode active" signal a drawing
  tool can give — cite it in the discoverability checklist as the thing
  that answers "what happens if I click right now?" without reading a
  tooltip first.
- **The existing `ScrollArea`'s drag-to-pan must be suppressed while a
  tool is active** — set `.drag_to_scroll(!active_tool.is_some())` (or
  equivalent) on the canvas's scroll area. Without this, a click-drag
  while, say, Square is active would ambiguously both draw a rectangle
  preview *and* pan the view, which is a genuine, easy-to-miss
  implementation bug specific to this Pass (Pass 1 never had two
  consumers of the same drag gesture). Plain wheel-scroll and
  ctrl+scroll-zoom are **untouched** — only click-drag pan is
  superseded, and only while a tool is active. Surface this trade-off in
  the property bar's hint text ("Scroll to pan, drag to draw.") so an
  operator who reflexively tries to drag-pan mid-tool understands why
  the view didn't move.

### 2.4 Ink

- Drag starts capture: `response.drag_started()` → `draw_state =
  DrawState::Ink { points: vec![screen_to_page(pos, ...)] }`.
- Every frame while `response.dragged()`: append the current pointer
  position (in page-space) if it differs from the last captured point
  by more than a small on-screen epsilon (avoids an absurdly dense point
  list from a stationary-but-still-"dragging" pointer).
- `response.drag_stopped()`: if `points.len() < 2` (a click with no real
  movement), discard silently — **the property bar's hint already said
  "drag to draw a freehand stroke," so no reactive message is owed** (a
  no-op needs no narrator line; the guidance was visible before the
  click happened, which is the cheaper, less chatty way to satisfy
  discoverability than firing a message after every accidental click).
  Otherwise, push `Action::CommitMarkup(Ink { points, color, width,
  opacity })`.
- **Multi-stroke Ink** (one Ink annotation covering several separate
  pen-lifts, which real Ink annotations support via multiple `InkList`
  subpaths) is **P2** — one drag = one stroke = one annotation for this
  Pass, which is simpler, correct, and matches "one command per
  completed shape" without needing a "hold to add another stroke before
  committing" gesture this Pass doesn't need to invent yet.

### 2.5 Square / Circle

- `drag_started()` → `DrawState::Dragging { anchor, current: anchor }`.
- Live preview: an axis-aligned rectangle (Square) or the ellipse
  inscribed in it (Circle) from `anchor` to `current`, updated every
  frame from the pointer.
- `drag_stopped()`: if the resulting `/Rect` is degenerate (near-zero
  width or height — the same "click, don't drag" case as Ink), discard
  silently. Otherwise commit one `Action::CommitMarkup(Square { rect,
  stroke_color, width, opacity, fill })` (or `Circle`).
- **Shift-to-constrain** (perfect square / perfect circle) is a common,
  reasonable convenience but is **P2** — not required, and not an
  Acrobat-mechanics question either way (it's a drawing-tool convention
  found across many unrelated apps, not something borrowed from
  Acrobat's own GUI).

### 2.6 Line

- Identical drag model to §2.5; commits `/L [x1 y1 x2 y2]` from anchor
  to release point. Degenerate (near-zero-length) drags discard
  silently, same reasoning.
- Shift-to-constrain-angle (0/45/90°) is the same **P2** convenience as
  above.

### 2.7 Polygon / PolyLine

- **Click** (not drag) adds a vertex: on `response.clicked()` while
  `active_tool` is `Polygon`/`PolyLine`, push the clicked page-space
  point into `DrawState::Polygon.vertices`.
- **Live rubber-band**: every frame, draw the committed vertex chain
  plus one more segment from the last vertex to the current pointer
  position (`response.hover_pos()`), so the operator always sees exactly
  what the *next* click will add.
- **Backspace** while mid-construction removes the most recently placed
  vertex (an interactive-state undo, not an `EditSession` undo — nothing
  has committed yet, so this must not create or consume an undo-stack
  entry). This is a genuinely different meaning for Backspace than its
  existing binding (`Action::DeleteSelection` on the rail) — the
  dispatcher must check `draw_state` **first** and only fall through to
  the rail-delete meaning when no polygon is in progress. Name this
  explicitly in `collect_keyboard_actions`'s doc comment so a future
  reader doesn't "simplify" it into one binding.
- **Commit**: double-click, or `Enter` (unbound elsewhere — free), pushes
  `Action::CommitMarkup(Polygon { vertices, .. })` /
  `PolyLine { vertices, .. }`. Minimum vertex count to commit: **3** for
  Polygon (a closed shape needs at least a triangle to mean anything),
  **2** for PolyLine (a single open segment is a legitimate PolyLine).
  Below the minimum, double-click/Enter is a no-op — again, no reactive
  message needed, the property bar's hint already states the running
  vertex count and the action that finishes the shape (§4.2).
- **Cancel**: `Esc` discards the whole in-progress vertex list (first
  `Esc` per §1.4's two-stage rule).
- One `EditSession` command per completed polygon, **regardless of
  vertex count** — exactly the task's own framing, and exactly the
  §1.3.3-in-`pass-3.2` precedent of "one coarser before/after snapshot
  command, not a per-vertex command stack."

### 2.8 Preview rendering — an overlay, never a re-raster

The in-progress shape is drawn directly with `ui.painter()` calls
(`line_segment`, `rect_stroke`, `circle_stroke`, or a manual polyline via
repeated `line_segment`s) **on top of** the already-rasterized page
`Image`, every frame, using `page_to_screen` (§2.1) to project the
stored page-space geometry. This is the same "don't re-rasterize for
something the operator hasn't committed yet" principle `main.rs`'s own
module docs already establish for zoom debouncing — a live preview is
cheap vector painting, never a trip through `pdfce-render`.

**Committed** shapes, by contrast, only become visible by re-rasterizing
the page (§2.9) — there is no separate "draft annotation" rendering path
that would have to be reconciled with the real one later.

### 2.9 Commit → `EditSession`, and making it visible

`Action::CommitMarkup(shape)` in `apply()`:

1. Calls the new `pdfce-core` API (name/shape is the engineer's and
   `pdfce-spec-librarian`'s call, not dictated here) to add one
   annotation dictionary + one generated appearance stream to the
   current page, via the session's staging buffer (R45).
2. On `Ok`: calls `doc.refresh_pages()` — the **existing** function
   already used after rotate/delete/reorder/undo/redo, which
   unconditionally drops the cached page texture and thumbnails. No new
   invalidation path is needed; this is exactly what already happens
   after every other edit that changes what a page looks like.
3. Sets a narrator line via the **existing** `edit_note` channel (same
   one Delete/Reorder/Extract already use), e.g. *"Added an Ink
   annotation. Use Undo to reverse it until you save."* — new
   `ui_text` entries, same channel, no new UI surface.
4. On `Err` (a certification refusal, §5.2): routes through the
   **existing** `save_result = Some(SaveOutcome::Failed(...))` channel —
   the identical pattern `delete_selection`/`rotate_selection` already
   use for the same class of refusal. No new error-presentation surface.

---

## 3. Quad-point markup — rectangle fallback, explicitly, with the
glyph-accurate version explicitly deferred

### 3.1 Why a non-text-aware rectangle still ships a real, useful feature

The saved annotation is a genuine `/Highlight`/`/Underline`/`/StrikeOut`/
`/Squiggly` dictionary with real `/QuadPoints` and a spec-conventional
generated appearance (Multiply-blended yellow fill for Highlight; a
thin line at the appropriate vertical position in the quad for the other
three). Any other PDF viewer — including a real Acrobat reviewer this
document might travel to — will recognize it correctly as markup on
that region of the page. What's missing is only the *convenience* of
"drag over some words and the exact glyph bounds are found for you" —
the interoperability value of the annotation itself is not diminished by
the operator having drawn the box by hand.

### 3.2 Interaction

Identical drag model to Square (§2.5): drag a rectangle; on release,
commit one `Action::CommitMarkup(quad_tool, rect, color, ...)` whose
`/QuadPoints` are the four corners of the dragged rectangle — **one
quad**, not one derived per line of text (there is no text awareness to
derive lines from). Live preview is tinted per subtype so the operator
sees roughly what they'll get: a translucent yellow fill for Highlight;
a thin horizontal line preview at the bottom edge of the drag rectangle
for Underline; at the vertical midpoint for StrikeOut; a wavy line at
the bottom edge for Squiggly.

**Per-subtype property-bar contents differ** (§4.2): Highlight exposes
color only (no width — it's a filled region, not a stroke; opacity is
the spec-conventional default, not operator-adjustable this Pass).
Underline/StrikeOut/Squiggly expose color **and** a thickness control,
because — unlike a real text-derived quad, which could derive a
sensible line weight from font size — a hand-drawn rectangle has no font
size to derive one from, so the operator needs a manual control the
glyph-accurate version (§3.4) would not.

### 3.3 The mandatory honesty disclosure — fuzzy, never sneaky

An operator's mental model of "Highlight" from any other PDF tool is
"select text, then highlight it." pdfce's rectangle model is genuinely
different and lower-fidelity, and that gap must be disclosed **every
time the tool is reachable**, not once as a dismissible tip (rule 4's
status-bar-narrator principle: a fact the operator needs is stated
persistently, not as a toast that can be missed):

- The tool's menu-item tooltip (§6.1) states it plainly: *"Drag over the
  area you want to mark. This does not detect the words underneath yet —
  it marks whatever region you draw."*
- The property bar's hint label repeats a short form while the tool is
  active: *"Drag over the text to mark — not yet text-aware."*

This is a *quality* disclosure (rule 1's "fuzzy" framing), not a
data-provenance one like OCR confidence — there is no operator override
to offer here beyond "draw it more carefully," so the disclosure's job
is purely to set the correct expectation before the operator drags, not
to gate a commit behind a confirmation. No modal is warranted; this is
squarely a tooltip/hint-text situation, same weight class as
`copy_page_text_tooltip`'s existing "pdfce works it out from letter
position" framing.

### 3.4 The deferred, glyph-accurate version — pointer, not a re-design

`pass-4-text-extraction.md` §4 already designed the real version of this
feature in full: logical-order anchor/focus text selection over
`PageText::runs`, click/double-click/triple-click granularity,
`Ctrl+Shift`-arrow keyboard extension, and (per that spec's own §4.11)
sized it as **a dedicated Pass**, larger than Pass 3.2's Merge/Split
follow-up. Converting a completed *text* selection into `/QuadPoints`
once that slice exists is a small, mechanical addition on top of it (one
quad per selected line, taken from the union of covered glyph rects
already computed for the highlight-rendering step that spec's §4.5
already specifies) — **do not re-derive any of this now.** When that
Pass is scheduled, its acceptance criteria should explicitly include
"wire the existing quad-point authoring commands (§2.9's API) to a
completed text selection" as the last step, rather than reopening the
selection-model design.

### 3.5 Dependencies before P1 can ship (both already named in
`ROADMAP.md`, repeated here because they gate this GUI slice directly)

1. **The QuadPoints CCW-vs-Z-order question** (`ROADMAP.md` Pass 6.1
   entry: "§12.5.6 says CCW, real producers/Acrobat emit Z/reading
   order") — this determines the *order* the four corners of the
   dragged rectangle must be written in. This is a spec/interop
   question for `pdfce-spec-librarian`, not a UI question; the GUI's
   drag-rectangle interaction is correct regardless of which order wins,
   but the commit call cannot be wired until it's settled.
2. **Per-subtype appearance generation** for the four quad subtypes
   (Multiply-blend fill for Highlight; line-drawing for the other
   three) needs to exist in `pdfce-core`/the appearance-generation path
   before `Action::CommitMarkup` has anything real to call for these
   four tools.

If either is not ready when the rest of Pass 6.1 is, **ship the P0 shape
tools and gate the four quad-point toolbar entries behind the same
placeholder pattern Insert/Split already use** (§3.6) rather than
blocking the whole Pass.

### 3.6 CLI-placeholder fallback, if P1 slips

Reuse `ui_text::tool_available_in_cli` verbatim (it already exists, and
its own doc comment already states the principle this needs: "a
placeholder that says 'coming soon' wastes the operator's time; one that
hands them a working command does not"). If the quad-point GUI tools
aren't ready, their toolbar menu rows are simply omitted (not shown
disabled — there's no "coming soon" state worth occupying a menu row
for when the CLI is a strictly better answer), and the CLI equivalent
(whatever `pdfce-cli`'s Pass 6.1 subcommand surface turns out to be) is
what the engineer names in the Pass's ship notes.

---

## 4. Appearance properties — the property bar

### 4.1 Placement — a new transient top panel, not a floating window, not the dock

Per the durable **five-way placement taxonomy** (`ARCHITECTURE.md` §12
continuation-23(b)): *view-state → toolbar view group; edit → toolbar/
window; selection-scoped → rail; advanced → Tools dock; disclosure →
status bar.* Choosing a shape/color/width for the **next** shape you're
about to draw is an **edit-adjacent, transient control**, not a
disclosure and not a rail-selection (the rail's selection is pages, not
drawing state) and not a Tools-dock entry (the dock is explicitly
"things that act on files outside the one you have open" per its own
intro sentence, `tools_dock_intro()`).

The taxonomy's "edit → toolbar/window" branch technically permits a
floating window (Properties, Pass 3.1's one legacy exception per
`pass-3.2` §1). **Don't use one here.** A floating window would be a
**second** exception to the "floating windows retired for new tool
surfaces" decision, and this control is used far more continuously than
Properties (it's live for the whole duration of a drawing session, not
an occasional metadata edit) — exactly the shape `pass-3.2` §1
anticipated when it said don't migrate Properties and don't add a
second one of its kind.

**Decision: a new `TopBottomPanel::top`, shown only while `active_tool`
is `Some`** (hidden, not disabled — matching the rail selection bar's
existing "hidden until relevant" convention, not the grayed-out
"disabled" convention used for finer-grained per-control gates like
zero-page rotate). This is a genuinely new placement case the five-way
taxonomy didn't need to name explicitly (nothing before Pass 6.1 needed
a transient, tool-scoped settings strip), so it is recorded here as a
sixth concrete instance of the "edit" bucket rather than silently
extended — flag this to the librarian as worth a one-line addition to
the taxonomy's own record ("edit, transient/tool-scoped → a dedicated
top panel shown while the tool is active" is a distinct enough shape
from "edit → toolbar/window" to be worth naming explicitly next time the
taxonomy doc is touched).

**Panel-add order** (extends `pass-3.2` §1's table; the bottom status
panel must still be added before any side panel to stay full-width —
unchanged rule, new row inserted):

```
1. toolbar               (top)
2. markup property bar   (top, second — stacks below the toolbar; shown
                           only when active_tool.is_some())
3. status                (bottom)
4. thumbnail rail         (left)   — if rail_expanded
5. Tools dock             (right)  — if tools_open
6. CentralPanel           (canvas)
7. properties_window(ctx, …)  — last, floating-over-everything, unchanged
```

Tab order becomes toolbar → property bar → rail → dock → canvas — the
canvas is still last, and (§2.2) now actually participates in it instead
of being a dead stop.

### 4.2 Per-tool contents

```
ui.horizontal(|ui| {
    ui.label(markup_property_bar_tool_label(tool));       // "Ink:" / "Highlight:" / …

    if tool.has_stroke() {
        color_edit_button_srgba(ui, &mut markup_color)
            .on_hover_text(markup_property_bar_color_tooltip());
        ui.label(markup_property_bar_width_label());
        ui.add(Slider::new(&mut markup_width, 1.0..=12.0).suffix(" pt"));
    } else {
        // Highlight only: fill colour, no stroke width.
        color_edit_button_srgba(ui, &mut markup_color)
            .on_hover_text(markup_property_bar_color_tooltip());
    }

    if tool.has_fill_option() {
        ui.checkbox(&mut markup_fill_enabled, markup_property_bar_fill_checkbox_label());
        if markup_fill_enabled {
            color_edit_button_srgba(ui, &mut markup_fill_color);
        }
    }

    if !tool.is_quad_point() {
        ui.label(markup_property_bar_opacity_label());
        ui.add(Slider::new(&mut markup_opacity, 0.0..=100.0).suffix("%"));
    }
    // Quad tools: opacity is the spec-conventional per-subtype default
    // (§ task framing: "Highlight yellow /Multiply") and not exposed —
    // deliberately less than full manual control, consistent with
    // "parity-default appearances the operator can override" reading as
    // overriding the COLOUR, not necessarily every appearance parameter,
    // for the four subtypes whose whole visual identity depends on a
    // specific blend mode.

    ui.separator();
    ui.label(markup_hint_for(tool, &doc.draw_state));   // always-visible instruction text, §2.4–2.7/§3.3
});
```

### 4.3 Persistence model this Pass

One shared "current pen" (`markup_color`/`markup_width`/`markup_opacity`/
`markup_fill`) across **all** tools, reset to a sensible default at
document-open (not remembered across documents or app restarts — no
settings-file work this Pass, matching `rail_expanded`'s own "session
state only" precedent). **Per-subtype separate memory** ("Ink defaults
to blue, Highlight remembers yellow independently") is a reasonable
enhancement but is **P2** — do not build it speculatively; a shared pen
is simpler, correct, and matches how the very first version of nearly
every drawing tool works.

Changing the property-bar controls is never an `EditSession` edit —
exactly Pass 3.1's "typing in a text box is not an edit" rule, extended
to sliders and color pickers. Only completing a shape (§2.9) produces a
command.

---

## 5. Signature / certification interaction

### 5.1 Two distinct moments — reuse both existing patterns, invent nothing

| Moment | Trigger | Existing pattern reused | New dialog? |
|---|---|---|---|
| **Hard refusal** | The document's certification **forbids the change outright** (a P=1-equivalent DocMDP enforcement) | The `EditError` → `save_result = Some(SaveOutcome::Failed(...))` status-bar channel `delete_selection`/`rotate_selection` already use for "a certification signature that forbids the change" | **No** |
| **Soft warning** | The change is allowed, but **saving** would invalidate an existing signature | `Pass 3.2`'s `signature_confirmation` blocking `egui::Window`, fired from `begin_save`, unchanged | **No** — reused verbatim |

**Both fire at the moment the task specifies, and it's worth stating
explicitly why they fire at *different* moments rather than both at
save:** the hard refusal is a property of the *document*, knowable the
instant the operator tries to commit a shape — there is no reason to let
them draw an entire multi-vertex polygon only to discover at save time
that it was never going to be allowed. The soft warning is a property of
the *save*, per `pass-3.2`'s own already-established finding: "§11.1
makes the dirty set a diff computed **at save time**, so whether this
save is structural is not knowable when the edit is committed." Nothing
about annotation authoring changes that reasoning — it's the same
architecture, applied to a new edit kind.

### 5.2 Draw-time hard refusal

`Action::CommitMarkup`'s core call (§2.9) can return `Err`. When the
underlying `EditError` is the certification-forbids-change kind, it
routes through the **existing** `save_result` channel exactly like
`delete_selection`'s existing handling — no new presentation code, a new
`ui_text` message naming annotation authoring specifically (so the
operator reads "adding this annotation was refused" rather than a
generic message that reads like a save failure they didn't ask for).

### 5.3 Save-time soft warning

Unchanged. `begin_save` already asks `session.signature_impact_of_save`
and shows `signature_confirmation` for `SignatureImpact::Invalidated`.
Nothing about this Pass touches that function; a document carrying newly
-authored annotations reaches it exactly the way a document carrying a
rotated page already does.

### 5.4 Recommended P1 improvement, and a resolved spec-precision gap —
**ship conservative, name the residual (coordinator decision X11)**

**The improvement:** proactively gray out the Draw▾ toolbar control
(§6.1) with an explanatory tooltip when `doc.session.signature_census()
.forbids_structural_change()` is true, rather than only refusing at
commit time. This is strictly better UX (rule 5's "discoverable
destructive [refusal], frictionless [everything else]" reasoning applies
in reverse here too — better to know before spending effort drawing a
shape than after) and needs **no new `pdfce-core` API**:
`EditSession::signature_census()` and `SignatureCensus::
forbids_structural_change()` already exist and are already called
elsewhere in this crate (`extract_selection`). This is P1, not P0 — §5.2
alone is a complete, honest, already-correct-by-construction refusal
path; graying out proactively is a nicety on top of it, not a
requirement.

**The gap, now resolved against the spec (was an open question in the
first draft of this section; no longer):** `SignatureCensus::
forbids_structural_change()`'s current body is `self.perms_enforced &&
self.signatures > 0` — it does **not** consult `certification_permission`
(the actual DocMDP `/P` value, 1/2/3, which the struct already carries as
a separate field). Its own doc comment explains why this was fine when
written: *"no `P` value's permitted list contains any operation pdfce can
currently perform"* — true at Pass 3.2 time, when the only structural
operations were page delete/rotate/reorder and none of DocMDP's three
permission levels grant any of those. **That premise no longer holds now
that annotation authoring is a real capability.** Table 254's VALIDATION
MODEL (`iso32000__s__12.8.md`, already sourced in
`D:\Dev\Rag-Specialized\PDF_Spec\`) is now confirmed: **`/P 3` permits
annotation addition**, **`/P 1` forbids it outright**, and **`/P 2` is
form-fill-in-and-signing only — it does not grant annotation addition**.
So the existing coarse boolean, blind to which `/P` value is present,
**over-refuses specifically at `/P 3`**: a document whose certification
explicitly permits adding annotations is still hard-refused today,
because the check predates annotation authoring and has never
distinguished `/P` levels.

**Decision (X11): ship the conservative existing check as the P0 hard
refusal this Pass. Do not build a precise per-`/P` annotation-permission
gate now.** The direction of the error is what makes shipping it
acceptable rather than merely expedient: over-refusing declines an edit
DocMDP would actually have permitted — fail-clean and disclosed (the
operator sees a named refusal and can investigate) — never the reverse
mistake of *permitting* something the spec forbids, which is the one
direction a signature/certification boundary cannot tolerate. Ship the
conservative check as-is; do not gate this Pass on refining it.

**The residual, named precisely for a follow-up (no longer an open spec
question, now a scoped `pdfce-core` task):** a `/P 3`-certified document
will have annotation authoring refused (§5.2's hard refusal, and §5.4's
proactive gray-out once built) even though its own certification permits
it. The fix, when scheduled, is narrow and fully specified: distinguish
operation kind in the gate — page-structural edits stay refused at `/P`
≤ 2 exactly as today, but annotation-adding should additionally consult
`certification_permission == Some(3)` and permit it in that one case.
No further spec research is owed before that follow-up is built; the
boundary is known.

---

## 6. Discoverability + keyboard map

### 6.1 Toolbar entry

One new toolbar group, **"Markup"**, inserted after the existing
"history" group (Undo/Redo) — net toolbar growth **+1 button** in the
steady state (the ✕ cancel pill is hidden, not always-present, matching
the rail selection-bar's own "hidden until relevant" convention flagged
as the right model in the Pass 3.1 review).

```
ui.separator();
// "Markup" group.
ui.menu_button(markup_menu_button_label(self.active_tool), |ui| {
    for (tool, label) in [/* Ink, Square, Circle, Line, Polygon, PolyLine */] {
        if ui.selectable_label(self.active_tool == Some(tool), label)
            .on_hover_text(markup_tool_tooltip(tool))
            .clicked()
        {
            actions.push(Action::SelectMarkupTool(Some(tool)));
            ui.close_menu();
        }
    }
    ui.separator();
    for (tool, label) in [/* Highlight, Underline, StrikeOut, Squiggly */] {
        // same pattern — separated visually so the two families read as
        // "shapes" vs. "text markup" without needing two menu buttons
    }
});
if self.active_tool.is_some()
    && ui.add_sized(ICON_BUTTON_SIZE, egui::Button::new(markup_cancel_button()))
        .on_hover_text(markup_cancel_tooltip())
        .clicked()
{
    actions.push(Action::SelectMarkupTool(None));
}
```

`markup_menu_button_label` returns a **different string** depending on
whether a tool is active (`"✏  Draw ▾"` vs. `"✏  Drawing: Ink ▾"`, etc.)
— this is the "visible current state" requirement from the
discoverability checklist, and it means the operator never has to open
the menu just to check what mode they're in.

### 6.2 Keyboard map — new chords, all unclaimed against
`collect_keyboard_actions`'s existing bindings (verified against the
full existing list: `Ctrl+O`, `Ctrl+Plus/Equals/Minus/0`, `Ctrl+S`,
`Ctrl+Shift+Z`, `Ctrl+Y`, `Ctrl+Z`, `[`/`]`, `Alt+↑/↓`, `Esc`, `Delete`,
`Backspace`, `Ctrl+Shift+C`)

| Chord | Action | Note |
|---|---|---|
| `Alt+I` | Enter Ink | |
| `Alt+S` | Enter Square | |
| `Alt+C` | Enter Circle | |
| `Alt+L` | Enter Line | |
| `Alt+P` | Enter Polygon | |
| `Alt+O` | Enter PolyLine | "O" for "open" polyline vs. closed polygon; no collision with `Ctrl+O` (different modifier — egui's `consume_key` matches modifiers exactly) |
| `Alt+H` | Enter Highlight | P1 |
| `Alt+U` | Enter Underline | P1 |
| `Alt+K` | Enter StrikeOut | P1 — "K" chosen over "S" (already Square) or "T" (ambiguous with "text") |
| `Alt+Q` | Enter Squiggly | P1 |
| `Enter` | Commit Polygon/PolyLine | Only meaningful mid-construction; unbound elsewhere, free |
| `Backspace` | Remove last polygon vertex **while mid-construction**; falls through to the existing `DeleteSelection` (rail) meaning otherwise | Context-sensitive — see §2.7's note; must be checked in this order in the dispatcher, not the other way around |
| `Esc` | Two-stage: cancel in-progress shape, then (if already idle) exit to view mode; falls through to the existing `ClearSelection` (rail) meaning only when **no tool is active and no shape is in progress** | Three-way context-sensitive priority; document this explicitly in `collect_keyboard_actions`'s doc comment the same way the Ctrl+Shift+Z-before-Ctrl+Z ordering note already explains its own priority reasoning |

All ten tool-entry chords use the **same modifier** (`Alt`) so they read
as one coherent family rather than an arbitrary scatter — a small,
free discoverability win once an operator learns one of them.

⚠️ **Implementation note, flagged rather than assumed:** verify in the
pinned egui/winit version that `Alt`+letter chords are not intercepted
at the OS/window level (Windows historically treats `Alt` as a menu-
mnemonic modifier for apps with a native menu bar; eframe/winit windows
typically have none, so this is very likely fine, but "very likely" is
not "verified" — check it before shipping, the same discipline
`pass-3.2` §3.1 already applied to its own "which overlapping `interact`
wins" uncertainty).

### 6.3 Certified/refused-document parity with the encrypted-document pattern

pdfce already has one fully honest "tools are absent, and the canvas
says exactly why" pattern: an encrypted document currently fails to
open at all (`Status::Unsupported`), the **entire** toolbar edit/view/
nav/zoom group set (nested inside `if let Status::Open(doc) = ...`)
simply does not render, and `ui_text::canvas_unsupported` explains the
gap in plain language on the canvas itself. The new Markup group
inherits this for free — it lives inside the same `Status::Open`
conditional, so it vanishes along with every other editing control when
there is no open document to draw on, exactly matching existing
behavior, no new logic needed.

**Within** an open document, follow the **existing, finer-grained**
convention (disabled-but-visible, not vanished) for the two real
in-document gates:
- **Zero pages** (`doc.pages.is_empty()`): wrap the whole Markup group in
  `ui.add_enabled_ui(!doc.pages.is_empty(), |ui| { ... })`, the same
  guard already used for Rotate.
- **Certification forbids the change** (§5.4's proactive P1
  improvement): gray the Draw▾ button specifically, with a tooltip
  naming the reason plainly (`markup_certification_disabled_tooltip`) —
  "grayed + a reason in the tooltip" is the honest form rule 5 asks for,
  not a vanished control (an operator who doesn't understand *why*
  markup is unavailable on this one document is left guessing) and not
  a silent failure discovered only after drawing (§5.4 already covers
  why the commit-time refusal alone remains acceptable if this
  improvement doesn't make P0).

---

## 7. Accessibility

- **Color never the sole signal:** the active tool is shown via **text**
  (`markup_menu_button_label`'s changing string), not a color highlight
  alone. The in-progress preview is a **shape** (an outline, a rubber-
  band line), never a color wash with no boundary. ✅
- **Click-target sizing:** the ✕ cancel pill uses the existing
  `ICON_BUTTON_SIZE` constant, matching every other icon-only control in
  the toolbar. ✅
- **Tab order:** resolved by §2.2/§4.1 — toolbar → property bar → rail →
  dock → canvas, the canvas now genuinely reachable and interactive
  rather than a dead stop. ✅
- **Screen-reader gap, named plainly, not papered over:** everything
  drawn by these tools is a hand-crafted vector shape with no text
  alternative pdfce generates — a screen-reader user gets no
  announcement of "a highlight was added covering this region" any more
  than a sighted-but-blind-to-color user would get one from color alone.
  This is the same named, tracked `accesskit` gap already recorded
  against Pass 3.2's drag-reorder and pass-4's deferred text-selection —
  a third occurrence of one gap, not a new one; the librarian should
  link all three rather than filing separately.
- **Keyboard-operability, stated honestly rather than over-promised:**
  every *non-drawing* step in this feature is fully keyboard-operable —
  entering/exiting/canceling a tool, adjusting the property bar's
  sliders and color pickers, committing a Polygon, Undo/Redo, Save. The
  **freehand/arbitrary-geometry drawing gesture itself is not**, and
  this is not an egui-specific shortfall to promise a future fix for: an
  Ink stroke or a rectangle at an arbitrary screen position is
  inherently a pointer-first task in essentially every real-world
  implementation of this kind of tool, keyboard-only included — there is
  no realistic "draw a freehand curve with the keyboard" affordance to
  build. State this distinction plainly in the shipped feature's
  documentation (this is a difference in *kind* from the `accesskit`
  gap above, which is a real, fixable-someday tooling shortfall; this
  one is closer to "a hammer needs a hand," and conflating the two would
  misrepresent both).

---

## 8. `ui_text.rs` catalog

New section header: `// Markup annotation authoring (Pass 6.1)`. Every
entry below is one complete templated message (R2); tooltips name *when*
to use the tool, not just what it draws, per the discoverability
checklist.

**Toolbar / menu:**
- `markup_menu_button_label(active: Option<MarkupTool>) -> String` —
  `"✏  Draw ▾"` when `None`; `"✏  Drawing: {tool} ▾"` when `Some` (the
  per-tool display name reused from `markup_tool_label`).
- `markup_menu_tooltip() -> &'static str` — "Draw markup on this page:
  freehand ink, shapes, or a highlight/underline/strikeout over text."
- `markup_tool_label(tool: MarkupTool) -> &'static str` — one arm per
  tool: `"Ink"`, `"Square"`, `"Circle"`, `"Line"`, `"Polygon"`,
  `"PolyLine"`, `"Highlight"`, `"Underline"`, `"Strikeout"`,
  `"Squiggly underline"`.
- `markup_tool_tooltip(tool: MarkupTool) -> &'static str` — one arm per
  tool, naming *when* to reach for it, e.g. Ink: "Draw a freehand
  mark — drag to write, circle, or sketch anything a straight shape
  can't."; Polygon: "Outline an irregular area with straight edges —
  click each corner, double-click or Enter to close it."; Highlight:
  "Mark a passage as important. Drag over the area you want to mark —
  this does not detect the words underneath yet, it marks whatever
  region you draw." (quad tools carry the §3.3 disclosure inline, once,
  here).
- `markup_cancel_button() -> &'static str` — `"✕"`.
- `markup_cancel_tooltip() -> &'static str` — "Stop drawing and discard
  anything in progress (Esc)."
- `markup_certification_disabled_tooltip() -> &'static str` — "This
  document's certification does not allow adding markup." (P1, §5.4.)

**Property bar:**
- `markup_property_bar_tool_label(tool: MarkupTool) -> String` —
  `"{tool}:"`.
- `markup_property_bar_color_tooltip() -> &'static str` — "Color for the
  next shape you draw. Changing this does not affect shapes already on
  the page."
- `markup_property_bar_width_label() -> &'static str` — "Width".
- `markup_property_bar_opacity_label() -> &'static str` — "Opacity".
- `markup_property_bar_fill_checkbox_label() -> &'static str` — "Fill
  interior".
- `markup_hint_for(tool: MarkupTool, state: &DrawState) -> String` —
  one arm per (tool, state) combination that needs distinct wording,
  e.g. `Ink, Idle` → "Drag to draw a freehand stroke."; `Polygon,
  Polygon { vertices, .. }` → "{n} point(s) placed — click to add
  another, double-click or Enter to finish, Esc to cancel."; quad tools,
  `Idle` → the short disclosure form from §3.3.

**Status-bar narrator (`edit_note` channel, same as Delete/Reorder/
Extract):**
- `markup_added(tool: MarkupTool) -> String` — "Added {a/an} {tool}
  annotation. Use Undo to reverse it until you save." (Note: this
  matches `delete_succeeded`'s "Use Undo … until you save" phrasing
  verbatim — reuse the exact clause, don't rephrase it, so the operator
  learns one sentence pattern for "this was reversible pre-save," not
  several near-identical variants.)
- `markup_commit_refused(message: &str) -> String` — reuses the
  `save_failed`-style two-part shape: a plain-English headline ("This
  annotation could not be added.") plus the technical detail, matching
  `save_failed`'s own "the technical detail is included because the
  operator's next step is usually to report it" reasoning.

**Undo/redo labels** — extend `command_label`'s existing match:
```rust
CommandKind::AddAnnotation(subtype) => format!("add {} annotation", subtype_display_name(subtype).to_lowercase()),
```
(`subtype_display_name` can be `markup_tool_label` reused directly if
the new `CommandKind` variant carries a `MarkupTool`-shaped value, or a
small `pdfce-core`-side equivalent if it carries the core's own subtype
representation instead — engineer's call, not dictated here.) This gets
"Undo: add Ink annotation" in the Undo button's tooltip for free, via
the exact mechanism `undo_tooltip_for`/`redo_tooltip_for` already use.

---

## 9. Undo / write-path summary (matching the `pass-3.2`/`pass-4` convention)

| Operation | `EditSession` command? | Writes anything? |
|---|---|---|
| Enter/exit/cancel a tool | No | No — pure UI mode, not an edit |
| Adjust color/width/opacity/fill in the property bar | No | No — "current pen," not a document fact until used |
| Complete an Ink/Square/Circle/Line/Polygon/PolyLine shape | **Yes — one command** | Not until Save (staged in the session, per R45) |
| Complete a Highlight/Underline/StrikeOut/Squiggly rectangle (P1) | **Yes — one command** | Not until Save |
| Save (with any newly-authored annotations pending) | N/A | Yes — incremental, exactly Pass 3.1/3.2's existing Save path; no new save mode |

---

## 10. Priority table (consolidated)

| Item | Priority | Note |
|---|---|---|
| Ink, Square, Circle, Line — tool-mode infra, live preview, one-command commit | **P0** | |
| Polygon/PolyLine — multi-click state machine, Enter/double-click commit | **P0** | Same order of engineering effort as the drag tools; no glyph/text reasoning needed |
| Property bar (color/width/opacity/fill), shared "current pen" | **P0** | |
| Keyboard map (§6.2), draw-time certification hard refusal (§5.2), save-time signature reuse (§5.3) | **P0** | |
| Canvas made focusable/interactive (§2.2) — resolves the long-standing `main.rs` caveat | **P0** | Prerequisite for everything else in this spec |
| Quad-point family via rectangle-marquee (§3) | **P1** | Gated on the two named core dependencies (§3.5); ship the CLI-placeholder pattern (§3.6) if either slips |
| Proactive certification gray-out (§5.4) | **P1** | The commit-time refusal (P0) is already correct and sufficient without it |
| Shift-to-constrain (square/circle/45° line), multi-stroke Ink, per-subtype pen memory | **P2** | Common conveniences, not required for a first shipped version |
| Glyph-accurate, text-selection-driven quad-point generation | **P2 — a dedicated follow-up Pass** | Reuses `pass-4-text-extraction.md` §4 in full, unchanged; do not re-design |
| Post-hoc selection/editing of already-placed annotations (move/resize/recolor/delete-by-click) | **P2 — a dedicated follow-up bucket** | Undo is this Pass's correction mechanism; there is no annotation-selection model yet |

**If the engineer must cut this Pass down further: ship P0 alone.** The
task's own framing already anticipates this ("If canvas text-selection
is P2, say so clearly so the engineer ships the shape tools first") —
the same logic extends one level further down: if time runs out before
the quad-point family's core dependencies land, ship the six shape tools
alone and carry Highlight/Underline/StrikeOut/Squiggly forward as a
named, scoped follow-up rather than shipping a rushed, spec-uncertain
version of them.

---

## 11. Open items for the librarian

1. **New sixth instance of the placement taxonomy's "edit" bucket**
   (§4.1): "edit, transient/tool-scoped → a dedicated top panel shown
   while the tool is active" is a distinct enough shape from "edit →
   toolbar/window" to be worth naming explicitly the next time
   `ARCHITECTURE.md` §12's taxonomy record is touched.
2. **The `SignatureCensus::forbids_structural_change()` precision gap**
   (§5.4) — **resolved against the spec (X11), not still open.** Table
   254's VALIDATION MODEL (`iso32000__s__12.8.md`) is sourced and
   confirms `/P 3` permits annotation addition, `/P 1` forbids it, `/P 2`
   is form-fill-in-and-signing only. Decision taken: ship the existing
   coarse check (conservative, safe — worst case is a disclosed
   over-refusal, never a permitted-but-forbidden change) as the P0 hard
   refusal for this Pass; do **not** build the precise per-`/P` gate now.
   Record as a scoped `pdfce-core` follow-up item (not a spec-research
   item — the boundary is known): teach the gate to consult
   `certification_permission == Some(3)` specifically for
   annotation-adding, while page-structural edits keep the existing
   `/P` ≤ 2 refusal unchanged.
3. **Third occurrence of the `accesskit`/egui-AT gap** (§7) — link
   alongside the two already recorded against Pass 3.2's drag-reorder
   and pass-4's deferred text-selection, rather than filing a third
   independent entry.
4. **`main.rs`'s module doc "revisit this when the canvas gains
   focusable content" caveat is resolved by *this* Pass** (§2.2), not by
   the text-selection slice that originally named it — worth a
   corrective note the next time that history is summarized, the same
   way this spec corrects its own earlier forecast in §2.2's heading.
