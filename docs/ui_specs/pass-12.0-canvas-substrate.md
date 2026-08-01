# Pass 12.0 UI Spec — The Canvas-Interaction Substrate (R60, built once)

> Authored by `pdfce-ui-specialist`, on dispatch from the engineer/
> orchestrator. This is the implementation spec for Pass 12.0 — the
> **foundation slice** of decision 011's first beta and decision 010's
> Pass 12 — and the engineer implements it verbatim, deviating only with
> a recorded reason (the Pass 3.2/6.1/7/8 spec convention: deviations are
> named, not silent).
>
> Read before implementing: `docs/decisions/011-first-beta-scaled-
> measurement-dimensioning-tool.md` §2.6 ("F_canvas_substrate_R60") and
> its `pass_slicing` Pass-12.0 JSON block (deliverables/acceptance
> criteria are binding — this spec expands them into an implementable
> widget-tree/type spec, it does not relax them); `docs/decisions/
> 010-highest-value-investment-after-the-editing-arc.md` §4.3/§6 (R60,
> why the substrate is built once); `crates/pdfce-gui/src/{main.rs,
> viewer.rs, ui_text.rs}` in full; `docs/ui_specs/pass-6.1-markup-
> tools.md` §1–§2 (the tool-mode state machine and `screen_to_page`/
> `page_to_screen` this spec *generalizes* — read this section closely,
> §0 below states precisely what changes and why); `pass-7-form-
> fill.md` §1–2 (the case where NO tool-mode applies at all — the
> substrate must not force one); `pass-8-redaction.md` §1.1, §2.2–2.4
> (the load-bearing dependency flag that made this Pass necessary in
> the first place); `docs/ARCHITECTURE.md` §12 continuation-23(b) (the
> five-way placement taxonomy) and continuation-24/25 (§8.10.2 form-
> XObject ordering, R44–R52); `crates/pdfce-render/src/lib.rs`
> `page_device_geometry` (the rotation-bearing device transform this
> spec's geometry bridge reuses — do not re-derive it).
>
> **This spec does not cover:** any tool that mutates the document. No
> markup shapes, no redaction marks, no dimensions, no vector-object
> move/delete/drag-node. Those are Pass 6.1, Pass 8, Pass 12.M1/12.M2,
> and Pass 9a/9c-min respectively, and every one of them **layers onto
> what this Pass ships** rather than building its own copy of it (R60).
> This Pass proves the mechanism with **zero real tools attached** — a
> deliberately, verifiably no-op default state — and every acceptance
> criterion below is written to be checkable without any tool existing.

---

## 0. What this Pass actually changes, stated precisely up front

Three UI specs already exist — `pass-6.1-markup-tools.md` §1–2,
`pass-7-form-fill.md` §1–2, `pass-8-redaction.md` §1.1/§2 — and each
independently designed a slice of "the canvas becomes interactive."
Between them they already contain the right *ideas*; what none of them
could do, because none was written with the others in front of it, is
factor the ideas into **one** substrate that all three (and the beta's
new dimension/vector-edit tools) share without contradiction. This Pass
does that factoring. Concretely, three renames/generalizations happen
relative to what `pass-6.1`/`pass-8` already wrote — **read every
future Pass's use of the old names as the new ones below; this is not
new design, it is the same design given its permanent name**:

| Old name (pass-6.1/pass-8, unshipped) | New name (this Pass) | Why renamed |
|---|---|---|
| `MarkupTool` (enum) | `CanvasTool` (enum, **uninhabited at this Pass** — §3) | Pass 8 already had to bolt `Redact` onto `MarkupTool`; the beta needs `Dimension*`/`VectorEdit*` variants too. A tool-mode enum that is not just about markup needs a name that says so — same enum, same one-substrate discipline, honest name. |
| `Action::SelectMarkupTool(Option<MarkupTool>)` | `Action::SelectCanvasTool(Option<CanvasTool>)` | Follows the enum rename; one dispatch action for every tool family, not one per feature. |
| `active_tool: Option<MarkupTool>` (field, on `OpenDoc`) | `active_tool: Option<CanvasTool>` | Same. |
| (not previously named) | `CanvasTargetProvider` trait + `provider: Option<Box<dyn CanvasTargetProvider>>` | §4 — new in this Pass; Pass 9a is the first real implementor. |
| (not previously named) | `viewer::screen_to_page` / `page_to_screen` **plus** a new, previously-missing second bridge, `viewer::canvas_to_pdf_space` / `pdf_space_to_canvas` | §2.3 — closes a real correctness gap none of the three prior specs noticed (see §2.3's framing). |

Everything else in `pass-6.1`/`pass-7`/`pass-8` — the property-bar
placement, the per-feature keyboard chords, the drag-vs-pan reasoning,
the accessibility framing — stands unchanged; those specs simply now
build **on top of** the substrate this Pass ships instead of each
re-deriving its own copy of the state machine underneath it.

---

## 1. Focusable interactive canvas

### 1.1 The change

`main.rs`'s `canvas()` method currently allocates the page `Image`
inside a `ScrollArea` with `egui::Sense::hover()` when no texture is
ready yet, and an implicit (non-interactive) `Image` widget otherwise.
Neither carries `Sense::click_and_drag()`. Change the image allocation
to carry it explicitly, in both branches:

```rust
// Where the texture exists:
let image_response = ui.add(
    egui::Image::from_texture(&texture)
        .fit_to_exact_size(display_size)
        .sense(egui::Sense::click_and_drag()),
);
// Where it does not exist yet (first frame after Open):
let image_response = ui.allocate_rect(
    egui::Rect::from_min_size(ui.cursor().min, display_size),
    egui::Sense::click_and_drag(),
);
```

This `image_response` — not the outer `ScrollArea`'s own response — is
the substrate's canonical **canvas response**: every piece of §2–§5
below reads pointer/keyboard state relative to it, and its `.rect` is
the `image_rect` every geometry function in §2 takes.

### 1.2 Coexisting with pan/zoom — the one real risk, and how it's resolved

Today, panning is the `ScrollArea`'s own internal drag-to-scroll
(`ScrollSource::ALL`), and it operates on drags anywhere over its
content — including, after this change, over a widget (the `Image`)
that now *also* claims `Sense::click_and_drag()` for itself. Two
widgets claiming the same drag gesture is exactly the ambiguity
`pass-6.1` §2.3 already flagged for its own drawing tools ("a
click-drag while, say, Square is active would ambiguously both draw a
rectangle preview *and* pan the view"). This Pass generalizes that
fix rather than special-casing it per tool:

```rust
// One substrate-owned flag, computed once per frame, BEFORE the
// ScrollArea is built (egui's ScrollArea reads its builder options at
// construction, not reactively):
let suppress_pan = self.canvas_suppresses_pan();   // see §1.3 — a pure fn

let response = egui::ScrollArea::both()
    .id_salt("page-canvas")
    .scroll_source(egui::scroll_area::ScrollSource::ALL)
    .drag_to_scroll(!suppress_pan)
    .show(ui, |ui| { /* image_response as in §1.1 */ })
    .inner;
```

**At this Pass, `suppress_pan` is always `false`** (§1.3's pure
function returns `false` whenever no tool is active, which is the only
real state this Pass has) — so `drag_to_scroll(true)` is passed
unconditionally in practice, and plain-drag-pans-the-canvas is
**byte-for-byte unchanged** from today. The wiring exists so Pass 6.1
(whole-canvas suppression while any drawing tool is active) and Pass 7
(narrower, per-focused-field-rect suppression) each set the *input* to
this same function differently — neither invents its own
`drag_to_scroll` call site.

⚠️ **Verify, do not assume, before shipping:** which of the `Image`
widget's own `Sense::click_and_drag()` response and the `ScrollArea`'s
internal drag-to-scroll wins a given drag gesture in the pinned egui/
eframe version, and whether `drag_to_scroll(false)` is sufficient on
its own or whether the inner widget's response must *also* be consulted
(e.g., only start a canvas-response-driven drag if the pointer's
press-origin was not already claimed by the scroll area). This is the
same "which overlapping `interact` wins" question `pass-3.2` §3.1,
`pass-6.1` §6.2, and `pass-7` §2.4 each already had to flag for their
own overlapping-gesture cases — it is not resolved by inspection here,
it is a pinned-version fact to check once, centrally, for this Pass,
so `pass-6.1`/`pass-7`/`pass-8` do not each have to re-verify it.

### 1.3 `canvas_suppresses_pan` — a pure, testable function

```rust
/// Whether the canvas's own click-drag should win over the ScrollArea's
/// pan-by-drag for this frame. A pure function of two independently
/// meaningful facts, not of `CanvasTool`'s (currently nonexistent)
/// variants — see §3.4 for why every state-machine decision in this
/// Pass is written this way, never as a `match` on tool identity.
#[must_use]
pub fn canvas_suppresses_pan(tool_active: bool, narrow_suppression_rect: Option<Rect>) -> bool {
    // Pass 6.1's whole-canvas suppression: true whenever a drawing
    // tool is active and it did not opt into the narrower form.
    // Pass 7's narrower suppression: only the drag's start position
    // falling inside `narrow_suppression_rect` (a focused text
    // overlay's own rect) matters; the tool_active flag is irrelevant
    // to it. This Pass ships with tool_active always false and
    // narrow_suppression_rect always None, so this always returns
    // false — proven by the unit tests below, with no real tool
    // required to exercise every branch.
    tool_active && narrow_suppression_rect.is_none()
    // (Pass 7 replaces the call site's second argument with the real
    // hit-test; it does not change this function's signature.)
}
```

Unit-test every branch with plain `bool`/`Option` inputs — no
`CanvasTool` variant is needed to exercise the logic, because the logic
never inspects one.

### 1.4 Focus

On `image_response.clicked() || image_response.drag_started()`, request
focus for the canvas's own persistent `egui::Id`:

```rust
if image_response.clicked() || image_response.drag_started() {
    ui.memory_mut(|m| m.request_focus(image_response.id));
}
```

This is what makes the canvas a real stop in the Tab cycle rather than
an inert image — see §6.2 for the Tab-order consequence and the
`main.rs` module-doc update this closes.

---

## 2. `screen ↔ page` transform, and the second, previously-missing bridge

### 2.1 The two spaces in play — named precisely, because the prior specs did not have to distinguish them and a future Pass will get this wrong if it is not written down once, here

`pass-6.1` §2.1 designed `screen_to_page`/`page_to_screen` against
`doc.current_extent()` — which is `viewer::page_extent_pts`, i.e.
`pdfce_render::page_device_geometry(page, 1.0)`'s **device**-space
width/height (Y-down, top-left origin, `/Rotate` already resolved into
a possibly-swapped width/height). That choice is correct **for the
substrate's own purposes** (hit-testing against what is on screen,
drawing a live-preview overlay that must track the raster pixel-for-
pixel) and this Pass keeps it unchanged. But it is a **different**
coordinate space from the one `pdfce-core`'s existing authoring APIs
consume: `add_markup_shape`'s current code builds every `Rect` directly
from `page.media_box` — genuine **PDF user-space** (Y-**up**, origin at
the *un-rotated* MediaBox's lower-left corner, exactly what an
annotation's `/Rect` or a content-stream operand is expressed in).

**No prior spec noticed this gap because none of them has shipped a
real commit path yet** — Pass 6.1's shipped GUI is the minimal
default-rect affordance (`add_markup_shape`, `GuiMarkupKind`), which
never calls `screen_to_page` at all; it builds PDF-space rects
directly. The moment any tool-bearing Pass (6.1's real drawing tools,
8's redact marks, 12.M2's dimension picks, 9c-min's node drags) tries
to take a **canvas-space** point produced by `screen_to_page` and hand
it to an authoring API that expects **PDF user-space**, it needs a
second conversion — and if four different Passes each write that
conversion independently, that is exactly the "two decompositions
quietly diverge" risk decision 011 §2.1 named for the object model
(risk Z2) applied one layer down, to geometry instead of hit-test
targets. **This Pass builds that second bridge once, so no later Pass
has to.**

### 2.2 `screen_to_page` / `page_to_screen` — unchanged from `pass-6.1` §2.1, now formalized as substrate API

```rust
/// Canvas-space (Y-down, top-left origin, page-device points at
/// zoom 1.0, `/Rotate` already resolved via `extent`'s possibly-swapped
/// width/height) ← screen point.
///
/// `image_rect` is the canvas Response's own `.rect` (§1.1) for THIS
/// frame; `extent` is `doc.current_extent()`; `zoom` is `doc.view.zoom`.
/// Pure arithmetic — no rotation logic lives in this function itself;
/// rotation-correctness comes entirely from `extent` already carrying
/// the rotated width/height (see `page_extent_pts`'s own doc comment,
/// unchanged). Baking rotation into THIS function as well would be a
/// double-application bug — do not add rotation-aware branches here.
pub fn screen_to_page(pos: Pos2, image_rect: Rect, extent: (f32, f32), zoom: f32) -> Pos2;

/// The exact inverse of `screen_to_page`. Needed because a live preview
/// (§5) must project already-stored page-space geometry back to screen
/// space every frame, and (once Pass 9a exists) a hit-tested object's
/// page-space bounds must be projected to draw its selection outline.
pub fn page_to_screen(page_pt: Pos2, image_rect: Rect, extent: (f32, f32), zoom: f32) -> Pos2;
```

**Contract (binding acceptance criteria, decision 011's "test at
0/90/180/270°" requirement made concrete):**

1. **Round-trip.** For any `pos`, `image_rect`, `extent`, `zoom` with
   `zoom > 0` and both `extent` components `> 0`:
   `page_to_screen(screen_to_page(pos, ..), ..) == pos` within a few
   ULPs of `f32` error. Same in the other direction. This is the
   single property every consumer (hit-testing, snapping, live preview)
   depends on implicitly; test it as its own property test, not just
   spot values.
2. **Zoom-invariance of page-space quantities.** A fixed **screen**-
   space distance maps to a page-space distance that scales as `1/zoom`
   — this is what makes Pass 12.M1's screen-space snap tolerance
   zoom-invariant once converted. Test: `screen_to_page` of two points
   `d` screen-pixels apart yields page-space points
   `d/zoom` apart, for a spread of zoom values including the
   `ZOOM_LADDER` extremes.
3. **Rotation correctness is `extent`'s job, not this function's.**
   Test this precisely as: construct a page whose `page_extent_pts`
   returns `(w, h)` at `/Rotate 0` and the **swapped** `(h, w)` at
   `/Rotate 90`/`270` (already true and tested today, in `viewer.rs`'s
   existing test module) — then confirm `screen_to_page`/`page_to_screen`
   at each of 0/90/180/270°, called with the extent that
   `page_extent_pts` actually returns for that rotation, still satisfy
   property 1 above with no special-casing of the rotation value inside
   the function bodies. The four angles are testing that **nothing
   rotation-specific leaks into this function** — a passing test here
   is a test that the function is agnostic to rotation, not that it
   "handles" it.
4. **Degenerate inputs never panic or produce NaN/∞** — mirror
   `viewer.rs`'s existing `fit_scale`/`clamp_zoom` degenerate-input
   discipline (zero/negative extent, zero zoom) and fall back the same
   way those functions do (a finite, harmless value — `Pos2::ZERO` is
   the reasonable choice here since there is no sensible page-space
   coordinate for a degenerate page).

### 2.3 The second bridge — canvas-space ⟷ true PDF user-space

```rust
/// Converts a CANVAS-space point (§2.1's device/Y-down/rotated
/// convention — the space `screen_to_page` produces) into genuine PDF
/// user-space (Y-up, un-rotated MediaBox/CropBox-relative) — the space
/// every existing and future `pdfce-core` authoring API (`Rect`,
/// annotation `/Rect`, content-stream operands, the Pass 9a object
/// model's node coordinates) is expressed in.
///
/// Implemented by INVERTING the SAME transform `pdfce-render` already
/// computes for rasterizing this exact page — reusing
/// `pdfce_render::page_device_geometry(page, 1.0).2` (a
/// `pdfce_render::tiny_skia::Transform`, already re-exported by
/// `pdfce-render` — no new dependency, rule 13) and its own
/// `.invert()`. This is the geometry analogue of decision 011 §2.1's
/// "reuses the SAME content-token walk pdfce-render uses, so the
/// object model and the render agree by construction" — applied to
/// coordinate transforms instead of content decomposition. DO NOT
/// hand-derive a second rotation-undo formula in `pdfce-gui`; every
/// future Pass that needs this conversion (6.1's shape commit, 8's
/// redact-mark commit, 12.M2's dimension pick, 9c-min's node drag)
/// calls THIS function, once.
///
/// Returns `None` only for a genuinely non-invertible transform (a
/// degenerate page whose CropBox has zero extent — the same condition
/// `page_device_geometry` itself already guards elsewhere). Callers
/// treat `None` the way `add_markup_shape` already treats "no current
/// page": decline the commit rather than authoring garbage geometry.
pub fn canvas_to_pdf_space(point: Pos2, page: &Page) -> Option<Pos2>;

/// The exact inverse — PDF user-space → canvas-space. Needed by any
/// FUTURE consumer that receives geometry already in true PDF space
/// (the primary case: Pass 9a's object-model provider hands back a
/// hit-tested object's bounds in PDF space, and the substrate's
/// selection-outline overlay, §4, must project them to screen via
/// `page_to_screen(pdf_space_to_canvas(bounds, page), ..)`).
pub fn pdf_space_to_canvas(point: Pos2, page: &Page) -> Option<Pos2>;
```

⚠️ **Verify, do not assume:** confirm `tiny_skia::Transform::invert()`
exists and returns `Option<Transform>` in the pinned `tiny-skia`
version (re-exported today as `pdfce_render::tiny_skia`) before wiring
this — the same "flag it, verify once" discipline as §1.2's drag-vs-
scroll question, not asserted here as already confirmed.

**Test plan, mirroring §2.2's:** round-trip
(`pdf_space_to_canvas(canvas_to_pdf_space(p, page).unwrap(), page)
≈ p`) at `/Rotate` 0/90/180/270°, cross-checked against
`pdfce_render::page_device_geometry`'s own already-tested forward
transform on the SAME fixture pages `pdfce-render`'s test module
already uses — this is what proves "agrees with the renderer by
construction" rather than merely "internally self-consistent."

**This Pass builds and tests both bridges; it does not call
`canvas_to_pdf_space` from any real commit path, because it ships no
commits.** The function exists, is fully tested against
`page_device_geometry`, and is ready the moment Pass 6.1/8/12.M2/
9c-min need it.

---

## 3. Tool-mode dispatch state machine

### 3.1 `CanvasTool` — deliberately uninhabited at this Pass

```rust
/// Which interactive canvas tool is active, if any. `None` (via
/// `Option<CanvasTool>`) is the substrate's own no-op default: ordinary
/// pan/zoom (§1.2) plus click-to-select against whatever provider is
/// attached (§4) — currently no provider, so click-to-select is a
/// no-op too. Every tool-bearing Pass that lands after this one adds
/// ITS variants directly to this enum, in the SAME `pdfce-gui` crate,
/// rather than inventing a parallel `active_tool`-shaped field — R60's
/// "one substrate" binds the DISPATCH TYPE itself, not just the
/// widget. Expected future variants, for orientation only (not
/// authored here): Pass 6.1's `Ink`/`Square`/`Circle`/`Line`/`Polygon`/
/// `PolyLine`/`Highlight`/`Underline`/`StrikeOut`/`Squiggly`; Pass 8's
/// `Redact`; Pass 12.M2's dimension-pick tools; Pass 9c-min's
/// vector-edit tools (move/drag-node). (Pass 7's form-fill deliberately
/// adds NONE — `pass-7-form-fill.md` §1 already decided form-filling
/// needs no tool mode at all, and that decision is unaffected by this
/// Pass; form-fill uses `screen_to_page`/`page_to_screen` and the
/// focusable canvas from §1–2 directly, with `active_tool` staying
/// `None` throughout a fill session.)
///
/// Uninhabited (zero variants) is deliberate, not a placeholder to be
/// embarrassed about: it is the type-level proof that this Pass ships
/// no tool. `Option<CanvasTool>` remains a normal, sized, niche-
/// optimized type with an uninhabited payload — `self.active_tool` can
/// only ever observably be `None` until a future Pass adds a variant,
/// which is exactly the "zero behavior change to existing viewer"
/// acceptance criterion made structural rather than merely tested.
pub enum CanvasTool {}
```

`OpenDoc` gains one field: `active_tool: Option<CanvasTool>` (always
`None` this Pass; not persisted, matching `rail_expanded`/`tools_open`'s
existing session-state-only precedent).

### 3.2 `Action::SelectCanvasTool`

```rust
/// Enter (`Some`) or exit-to-view-mode (`None`) a canvas tool.
/// Generalizes `pass-6.1`'s proposed `Action::SelectMarkupTool` — one
/// dispatch action shared by every tool family, per §0's rename table.
SelectCanvasTool(Option<CanvasTool>),
/// Cancel the active tool's in-progress gesture WITHOUT exiting the
/// tool (the first stage of `pass-6.1`'s two-stage Escape, §3.5).
/// Generalizes `pass-6.1`'s proposed `Action::CancelDraw`.
CancelToolGesture,
```

Since `CanvasTool` has no variants yet, `Action::SelectCanvasTool(Some(_))`
is **uninhabited at the call-site level too** — there is no live code
path that constructs it this Pass. What IS live and tested: entering/
exiting via `None`, and the auto-cancel/enforcement-point wiring in
§3.3, all of which is exercised via the `tool_active: bool`-shaped pure
functions in §3.4, never via a real `Some` value.

### 3.3 One enforcement point, generalizing `pass-6.1` §1.5's auto-cancel rule

`pass-6.1` §1.5 specified: any action that is not itself part of
continuing/cancelling/committing the in-progress shape silently
discards it, enforced at one point in `apply()`. `pass-7` §2.6
correctly found the OPPOSITE policy is needed for form-fill's draft
text (commit, don't discard) — proving the *specific* policy
(discard vs. commit) is a per-tool decision, while the *mechanism*
(one enforcement point, not N) is the substrate's job. This Pass ships
the mechanism, generically:

```rust
/// What happens to a tool's in-progress, uncommitted gesture when an
/// unrelated action (Undo, Save, page navigation, opening Properties/
/// Tools-dock/…) is about to happen. The active tool's OWN gesture
/// state decides which of these applies each time this is consulted —
/// the substrate does not hardcode a policy, it only guarantees there
/// is exactly one place this question is ever asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GestureInterrupt {
    /// Nothing is in progress — the common case, and this Pass's ONLY
    /// reachable case (no tool exists to have a gesture).
    Nothing,
    /// Discard the in-progress gesture with no further action —
    /// pass-6.1's shapes: nothing has been written to the EditSession,
    /// so there is nothing to lose (rule 7's "no unnecessary friction").
    Discard,
    /// Commit the in-progress gesture as one EditSession command before
    /// the interrupting action proceeds — pass-7's text-field draft:
    /// the operator typed something with clear intent to keep it.
    Commit,
}

/// The ONE enforcement point (lives at the top of `apply()`, exactly
/// where `pass-6.1` §1.5 already placed its own version):
fn resolve_gesture_interrupt(&mut self, incoming: Action) {
    if matches!(
        incoming,
        Action::SelectCanvasTool(_) | Action::CancelToolGesture /* | a future
            Action::CommitToolGesture(_) each tool-bearing Pass adds its own
            commit action to this allow-list */
    ) {
        return; // this action IS the gesture continuing/committing itself
    }
    match self.current_gesture_interrupt() {   // queries whatever tool state exists
        GestureInterrupt::Nothing => {}
        GestureInterrupt::Discard => self.discard_active_gesture(),
        GestureInterrupt::Commit => self.commit_active_gesture(),
    }
}
```

**This Pass's `current_gesture_interrupt()` always returns
`GestureInterrupt::Nothing`** (there is no tool, hence no gesture) —
the function exists, is called from the one real enforcement point in
`apply()`, and is unit-tested to be a no-op today. Pass 6.1 and Pass 7
each replace its body with a query against their own gesture-state
shape (`DrawState`, a text-field draft flag) and return `Discard`/
`Commit` respectively — **neither adds a second enforcement point**.

### 3.4 Why every decision function above takes `bool`, never a `CanvasTool` match

`CanvasTool` is uninhabited this Pass. A function written as
`match self.active_tool { Some(CanvasTool::X) => .., None => .. }`
would have **zero live branches to test** until a real variant exists —
an entire class of substrate behavior (pan suppression's `true` branch,
the two-stage-Escape's "cancel gesture" branch, §5's overlay-painting
branch) would ship untested and only be exercised for the first time
when Pass 6.1 lands, which is exactly the wrong moment to discover a
substrate bug. **Every state-machine decision in this Pass is instead
written as a pure function over `bool`/`Option<T>`-shaped inputs**
(`tool_active: bool` derived from `active_tool.is_some()`,
`GestureInterrupt` as an explicit enum a future tool's own state
supplies) — never over `CanvasTool`'s variants directly. This is a
binding implementation instruction, not a style preference: it is what
makes 100% of this Pass's logic unit-testable today, with the uninhabited
enum proving "no tool" at the type level while the boolean-driven pure
functions prove every *branch* of the future dispatch is exercised now.

### 3.5 Two-stage Escape — the precedence chain, decided once

`pass-6.1` and `pass-8` each independently specified a two-stage Escape
(cancel gesture, then exit tool) that falls through to the *existing*
`ClearSelection` binding when idle; `pass-7` correctly found it needed
only a *single*-stage Escape for its own draft-commit model. Rather
than let each tool-bearing Pass improvise Escape's exact precedence
against whatever else Escape already means, **this Pass fixes the
canonical order every future Pass slots into**:

```
Esc pressed:
  1. Active tool has an in-progress gesture (GestureInterrupt::Discard
     policy tools only — pass-7's Commit-policy fields use their OWN
     single-stage Esc-reverts-to-gesture-start-value rule, §2.6 of
     that spec, which is NOT this chain)
       → cancel the gesture, STAY in the tool. Consumed; stop here.
  2. A tool is active, no gesture in progress
       → Action::SelectCanvasTool(None) (exit to view mode). Consumed;
         stop here.
  3. No tool active, the substrate's OWN selection set (§4) is
     non-empty
       → clear it. Consumed; stop here. (Unreachable until Pass 9a
         attaches a real provider; this Pass's selection set is
         always empty, so this branch never fires yet — tested via
         the pure `resolve_escape` function below with a
         non-empty-selection input, not via a real selection.)
  4. Otherwise → falls through to the EXISTING `Action::ClearSelection`
     (rail page-selection) binding, completely unchanged.
```

```rust
/// Pure precedence function — testable today with plain bools/options,
/// no real tool or selection required.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscapeOutcome { CancelGesture, ExitTool, ClearCanvasSelection, FallThroughToRailClear }

#[must_use]
pub fn resolve_escape(
    tool_active: bool,
    gesture_discardable: bool,   // GestureInterrupt::Discard, specifically
    canvas_selection_nonempty: bool,
) -> EscapeOutcome {
    if tool_active && gesture_discardable { EscapeOutcome::CancelGesture }
    else if tool_active { EscapeOutcome::ExitTool }
    else if canvas_selection_nonempty { EscapeOutcome::ClearCanvasSelection }
    else { EscapeOutcome::FallThroughToRailClear }
}
```

Document this precedence chain in `collect_keyboard_actions`'s own doc
comment (the exact place `pass-6.1` §6.2 and `pass-7` §7 each already
asked their own Escape special-cases to be documented) — as ONE
four-way priority note, not scattered across four future Passes' own
doc comments each partially restating it.

---

## 4. Hit-test / selection scaffold with a pluggable target provider

### 4.1 `CanvasTargetProvider` — lives in `pdfce-gui`, not `pdfce-core`

```rust
/// Opaque handle to a hit-testable thing on a page — a vector object
/// (Pass 9a), eventually a Bézier node (Pass 9c-min), eventually a
/// placed dimension (Pass 12.M2). The substrate never interprets this
/// value; it only stores it in a selection set and hands it back to
/// the SAME provider for bounds/details. Concrete representation
/// (an object index? a `pdfce-core` object number? a small enum
/// distinguishing object-vs-node?) is the engineer's/Pass 9a's call —
/// not dictated here, per the established `pass-6.1`/`pass-8`
/// precedent of leaving core-API shape unnamed at the UI-spec layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TargetId(/* engineer's choice */);

/// The seam a hit-testable content model plugs into. Defined HERE, in
/// `pdfce-gui`, deliberately — this is a GUI-interaction concept
/// (click/marquee hit-testing FOR SELECTION), and putting the trait in
/// `pdfce-core` would give the engine a GUI-shaped dependency exactly
/// backwards from the invariant (`ARCHITECTURE.md` §3/§11.4; this
/// project's load-bearing GUI-core separation rule). Pass 9a's real
/// implementation is a thin `pdfce-gui`-side adapter that CALLS INTO
/// `pdfce-core`'s read-only object model (which stays GUI-free) — the
/// adapter owns this trait impl, the object model owns none of it.
pub trait CanvasTargetProvider {
    /// Topmost/nearest target at a canvas-space point (§2.1's space —
    /// NOT PDF user-space; the provider is responsible for its own
    /// internal PDF-space object geometry, converting via
    /// `pdf_space_to_canvas`/`canvas_to_pdf_space` (§2.3) as needed to
    /// compare against this point).
    fn hit_test(&self, page_index: usize, point: Pos2) -> Option<TargetId>;

    /// Every target fully or partially enclosed by a canvas-space
    /// marquee rectangle (enclosure policy — fully vs. partially — is
    /// the provider's call, matching how Inkscape's own convention is
    /// itself a documented, non-obvious choice per the Inkscape
    /// capability RAG; this substrate does not dictate one).
    fn hit_test_rect(&self, page_index: usize, rect: Rect) -> Vec<TargetId>;

    /// The target's own canvas-space bounds, for drawing a selection
    /// outline (§4.3) — `None` if the target no longer exists (the
    /// document changed underneath a stale `TargetId`; the substrate
    /// treats this as "silently drop from selection," never a panic —
    /// see §4.4).
    fn bounds(&self, page_index: usize, target: TargetId) -> Option<Rect>;
}
```

### 4.2 Selection-set model

```rust
/// On `OpenDoc`, session-only, matching `selected_pages`'s existing
/// `BTreeSet` precedent (ordered — "the selection" is always a
/// deterministic, iterable list, never a HashSet's arbitrary order).
canvas_selection: BTreeSet<TargetId>,
/// The attached provider, if any. `None` until Pass 9a attaches its
/// adapter — the ENTIRE reason this Pass's selection behavior is
/// verifiably a no-op: every hit-test call below short-circuits to
/// "no target" when this is `None`.
target_provider: Option<Box<dyn CanvasTargetProvider>>,
```

Operations, all routed through the canvas response from §1:

- **Click** (`image_response.clicked()`, no drag): `hit_test` at the
  click's canvas-space point (via `screen_to_page`). A hit **replaces**
  the selection with that one target; a miss **clears** it (clicking
  empty canvas deselects — the universal convention, and the same
  "clicking away" behavior the Properties/Tools-dock panels already
  rely on implicitly).
- **Shift+click**: toggles the hit target's membership without
  disturbing the rest of the selection (add if absent, remove if
  present) — the standard multi-select convention across virtually
  every graphics editor (Inkscape included, per the capability RAG),
  distinct from the page-rail's own Shift-click **range**-select
  (`SelectRangeTo`), which is a different, page-ORDER-based semantic
  that does not apply to a set of arbitrary canvas objects with no
  inherent linear order.
- **Marquee** (drag starting on empty canvas — i.e., the drag's
  **start** point misses `hit_test`): `hit_test_rect` over the live
  rubber-band rectangle (§5), replacing the selection on
  `drag_stopped()`. Shift-held marquee **adds** the hit set to the
  existing selection rather than replacing it.
- **Clear**: Esc, per §3.5's precedence chain, step 3.

**With `target_provider == None` (this Pass's only real state):** every
`hit_test`/`hit_test_rect` call is skipped entirely (no provider to
call), so click always clears, marquee never selects anything, and
`canvas_selection` never becomes non-empty. This is the acceptance
criterion "Selection scaffold selects nothing until a target provider
is attached" made structural, the same way §3.1's uninhabited enum
makes "no tool" structural.

**Marquee-vs-pan disambiguation — flagged, not resolved, deliberately.**
A drag starting on empty canvas is, today, an ordinary pan gesture
(§1.2 — `suppress_pan` is always `false` this Pass). The instant a real
provider exists and marquee-select becomes meaningful (Pass 9a), a
drag-starting-on-empty-canvas becomes ambiguous between "pan" and
"marquee-select" in exactly the way a click-drag while a drawing tool
is active was ambiguous between "pan" and "draw" (§1.2) — **and this
Pass deliberately does NOT resolve that ambiguity**, because doing so
now would mean guessing at a UX decision (a modifier key? a distinct
"Select" tool the operator must explicitly enter, Inkscape-style,
rather than an always-on default? some other convention?) with no real
selection to validate it against. **This is Pass 9a's decision to
make, named here so it is made once, deliberately, rather than
discovered as a surprise mid-Pass 9a.** Until then, `hit_test_rect` is
built and tested against a stub marquee rectangle in isolation (proving
the function works), never wired to a live drag gesture that would
conflict with pan.

### 4.3 Selection-outline overlay — the rendering half

Once a real provider exists, `canvas_selection`'s members are drawn as
outlines via the SAME live-preview overlay mechanism §5 defines (never
a re-raster) — a 2px rectangle outline per selected target's
`bounds()`, projected via `pdf_space_to_canvas` → `page_to_screen`.
**Shape, not color, is the signal** (rule 6) — an outline with a real
boundary, matching `pass-7` §2.5's focus-ring precedent exactly (same
visual vocabulary, different trigger). This Pass paints nothing here
(`canvas_selection` is always empty), but the paint call itself is
written and covered by a test that asserts it is a no-op over an empty
selection set — not simply omitted pending Pass 9a.

### 4.4 Stale `TargetId`s

Any edit (Undo, Redo, an authored shape, a future vector-edit
move/delete) can invalidate a previously-selected `TargetId`. On every
`refresh_pages()` call (the existing function every edit already
funnels through), re-validate `canvas_selection` against the current
provider — `bounds()` returning `None` for a member removes it from the
set silently (mirroring `OpenDoc::clamp_selection`'s existing "drop any
selection entry past the end" precedent for the page-selection rail,
applied to canvas targets instead of page indices). No confirmation, no
narrator line — an edit that happens to deselect something it also just
changed is not a fact the operator needs disclosed, any more than
`clamp_selection` announces itself today.

---

## 5. Live-preview overlay

### 5.1 Mechanism — unchanged from `pass-6.1` §2.8, generalized to any tool

```rust
/// Called once per frame, immediately after the page Image is drawn
/// (§1.1), painting directly via `ui.painter()` on top of the already-
/// rasterized texture — NEVER a re-raster, exactly `pass-6.1` §2.8's
/// "don't re-rasterize for something the operator hasn't committed
/// yet" principle, generalized from markup-specific shapes to
/// whatever the active tool (or the selection outline, §4.3) wants
/// drawn this frame.
fn paint_canvas_overlay(&self, painter: &egui::Painter, image_rect: Rect) {
    // §4.3's selection outlines (always empty this Pass — tested as
    // a no-op, per §4.3).
    // A future tool's own in-progress-gesture preview (nothing to
    // call this Pass — CanvasTool is uninhabited).
}
```

This Pass ships the **call site** (wired into `canvas()`, right after
the image is drawn, before the response's hover/zoom handling) and the
**empty-selection no-op test** for §4.3's half of it. There is no
tool-gesture half to test yet, and none is invented speculatively.

### 5.2 Drag-vs-pan suppression

Already specified generically in §1.2/§1.3 — the overlay and the pan-
suppression are two sides of the same coin (a tool that draws a live
preview is, definitionally, a tool that also needs
`canvas_suppresses_pan` to return `true` while it drags), and neither
needs its own separate mechanism.

---

## 6. Placement, keyboard, and accessibility

### 6.1 Placement — this Pass adds NO new operator-visible surface

Every prior tool-bearing spec (`pass-6.1` §4.1, `pass-8` §3.1) needed a
placement decision because each introduces a new, visible control
(a toolbar menu, a side panel). **This Pass introduces none.** There is
no new toolbar button, no new panel, no new menu — the only observable
effect of Pass 12.0 shipping, from the operator's chair, is that the
canvas can now be Tab-focused (§6.2) and, per §1.2, still pans exactly
as it did before. This is deliberate and is itself worth stating
plainly rather than silently implying a placement decision was skipped:
**a pure-substrate Pass has nothing to place.** The five-way taxonomy's
"edit, transient/tool-scoped → dedicated top panel" (its sixth
instance, from `pass-6.1` §4.1) and "dedicated secondary panel for a
document-internal, multi-step review-then-commit workflow" (its
seventh, from `pass-8` §3.1) both remain exactly as recorded; this Pass
adds no eighth instance, because it adds no panel.

### 6.2 Tab order — closing `main.rs`'s Pass-1 caveat

`main.rs`'s own module doc (§"Panel order is load-bearing") has carried,
since Pass 1: *"Since the canvas holds no focusable widgets at Pass 1
… the focus-order cost is currently zero … Revisit this when the canvas
gains focusable content."* **This Pass is what makes that condition
false, and per `pass-6.1` §2.2's own forecast ("Pass 6.1 is the first
Pass to make that true") — that forecast is corrected here: it is
THIS Pass, not Pass 6.1's drawing tools, because the substrate (and
therefore the caveat's actual resolution) had to exist before any tool
could attach to it.**

**Decision: keep the existing panel-add order unchanged**
(toolbar → status → rail → Tools dock → CentralPanel/canvas →
Properties window) — do not reorder panels to put the canvas earlier
in the Tab chain. Per `main.rs`'s own reasoning for the current order
(status bar must span full window width, which requires it to be added
before any side panel), reordering purely for Tab-order polish would
regress a real layout property to fix a cosmetic one. What changes is
narrower and sufficient: **the canvas is now a genuine, reachable stop
at the end of the existing chain** (Tab from the Tools dock's last
widget — or the rail's last thumbnail, if the dock is closed — now
lands on the canvas itself, which shows a visible focus rectangle per
egui's default focus styling, rather than skipping straight to
wrap-around). This is the exact resolution `pass-6.1` §4.1 already
anticipated for its own (superseded, see §0) design: *"Tab order
becomes toolbar → property bar → rail → dock → canvas — the canvas is
still last, and now actually participates in it instead of being a
dead stop."*

**Required doc update — replace `main.rs`'s existing caveat text**
(the "Revisit this when the canvas gains focusable content" sentence
and its surrounding paragraph) with a note stating: this was resolved
by Pass 12.0's canvas-interaction substrate, not by any single drawing
feature; the canvas is now Tab-reachable at the end of the existing
panel-add chain; the layout-vs-focus-order tradeoff that paragraph
originally flagged was evaluated and the existing order was kept
deliberately (status bar full-width requirement), with the canvas's
new reachability closing the cost side of that tradeoff rather than
its layout side.

### 6.3 No new keyboard chords this Pass

Nothing in `collect_keyboard_actions` gains a new chord — there is no
tool to bind one to. What DOES change in that function: the Escape
handler gains the §3.5 four-way precedence dispatch (with branches 1–3
provably unreachable this Pass, per §3.4's testing discipline, and
branch 4 — fall through to the existing `ClearSelection` — remaining
the ONLY live path, unchanged from today). Document the full four-way
precedence in that function's doc comment now, per §3.5's closing
instruction, so Pass 6.1/7/8 each *read* the precedence rather than
each writing their own partial version of it.

**Flagged for a future Pass, not solved here:** `collect_keyboard_actions`
today reads `ctx.input()` unconditionally every frame — a **global**
keyboard dispatch model, not a focused-widget one. Pass 7's real
`TextEdit` overlay (§3.1 of that spec) needs focused-widget key
handling (typing into a field must not also fire, say, `Ctrl+Shift+Z`'s
global Redo binding, or worse, an unrelated single-letter chord meant
for a tool). This Pass's canvas becoming focusable does not yet change
how `collect_keyboard_actions` reads input — that reconciliation
(global chords vs. a focused text/interactive widget's own key
consumption) is real, needed, and explicitly **Pass 7's problem to
solve**, not this Pass's; naming it here so it is not rediscovered as a
surprise.

### 6.4 Accessibility

- **Color never the sole signal:** N/A this Pass (nothing is painted —
  §5.1) but the pattern is set for every future consumer: §4.3's
  selection outline is a shape, not a tint, mirroring `pass-7` §2.5's
  focus-ring precedent exactly.
- **Click-target sizing:** N/A — no new clickable controls this Pass
  (§6.1). The canvas response itself covers the whole displayed page,
  never a sub-minimum target.
- **Tab order:** resolved, §6.2. ✅
- **`accesskit` gap, forward guidance for Pass 9a, not a Pass-12.0
  finding:** `pass-7` §2.2 already established the load-bearing
  recommendation that four of six form-field kinds use REAL egui
  widgets (`ui.interact`, `TextEdit`, `ComboBox`) rather than hand-
  rolled `painter()` hit-testing, specifically because real widgets get
  Tab-focus and `accesskit` exposure for free. **The same
  recommendation applies to Pass 9a's selection targets**: where a
  target's hit-test region can reasonably be represented as a real
  `ui.interact(rect, id, Sense::click())` per visible object (as
  opposed to a single canvas-wide hand-rolled hit-test), doing so buys
  the same free accessibility win `pass-7` already banked for form
  fields. This Pass does not decide FOR Pass 9a (a page can hold
  thousands of vector objects — one `ui.interact` call per object
  every frame is a real performance question `pass-7`'s bounded,
  small field count never had to face), but the option is named here
  so Pass 9a evaluates it deliberately rather than defaulting to
  painter-based hit-testing by inertia.
- **Screen-reader gap:** unchanged and not newly created by this Pass
  — the underlying page raster is still an image with no text
  alternative (the standing, project-wide gap named in `main.rs`'s
  own module docs since Pass 1). This Pass adds no new instance of the
  gap (it draws nothing); it is recorded here only so a future reader
  does not have to check whether "the canvas is now interactive" opened
  a new screen-reader hole — it did not, because there is still
  nothing on the canvas for a screen reader to describe.

---

## 7. `ui_text.rs` catalog

**No new entries.** This Pass introduces no operator-visible string —
no new button, tooltip, menu item, or status-bar line. Stating this
explicitly (per `pass-7` §7's own "no chord is needed… worth stating
explicitly rather than silently omitting" convention) rather than
silently having an empty section: a pure-infrastructure Pass that adds
zero user-facing copy is a real, correct outcome, not an oversight.

---

## 8. Undo / write-path summary

| Operation | `EditSession` command? | Writes anything? |
|---|---|---|
| Canvas gains/loses focus | No | No — pure UI focus |
| `Action::SelectCanvasTool(None)` (a no-op, since it is already `None`) | No | No |
| Click/marquee against the (absent) provider | No | No — selection is view-state, never an edit, exactly like `selected_pages`'s existing precedent |
| Esc (falls through to existing `ClearSelection`) | No | No — unchanged existing behavior |

Every row is "no edit, nothing written" — this Pass cannot possibly
touch `EditSession`, because it ships no action that calls into one.

---

## 9. Priority table

| Item | Priority | Note |
|---|---|---|
| Focusable canvas (`Sense::click_and_drag`), focus-request wiring (§1) | **P0** | Resolves `main.rs`'s Pass-1 caveat |
| `screen_to_page`/`page_to_screen`, contract + tests incl. 0/90/180/270° (§2.2) | **P0** | |
| `canvas_to_pdf_space`/`pdf_space_to_canvas`, contract + tests (§2.3) | **P0** | New finding beyond decision 011's literal deliverables list — flag to the librarian (§10) as a scope refinement, not a deviation |
| `CanvasTool` (uninhabited), `active_tool` field, `Action::SelectCanvasTool`/`CancelToolGesture` (§3.1–3.2) | **P0** | |
| One `resolve_gesture_interrupt` enforcement point + `GestureInterrupt` (§3.3) | **P0** | |
| `canvas_suppresses_pan`, `resolve_escape` — pure, bool-driven, fully unit-tested (§1.3, §3.4–3.5) | **P0** | The testability discipline itself is P0, not a nicety — see §3.4 |
| `CanvasTargetProvider` trait, `TargetId`, `canvas_selection`, click/shift-click/marquee dispatch against an absent provider (§4.1–4.2) | **P0** | Selects nothing until Pass 9a attaches a provider — the acceptance criterion |
| Selection-outline overlay call site + empty-selection no-op test (§4.3) | **P0** | |
| Stale-`TargetId` cleanup on `refresh_pages()` (§4.4) | **P0** | Cheap, prevents a real future bug (dangling selection after Undo) |
| Live-preview overlay call site (§5.1) | **P0** | Empty this Pass; wired and tested as a no-op |
| `main.rs` module-doc update closing the Pass-1 focusable-canvas caveat (§6.2) | **P0** | Documentation-first rule; the caveat must not be left stale once resolved |
| Escape four-way precedence documented in `collect_keyboard_actions` (§3.5, §6.3) | **P0** | Cheap; prevents four future Passes each writing a partial version |
| Marquee-vs-pan disambiguation decision | **Explicitly NOT this Pass — Pass 9a's call** | Named, not silently deferred (§4.2) |
| Global-vs-focused keyboard dispatch reconciliation | **Explicitly NOT this Pass — Pass 7's call** | Named, not silently deferred (§6.3) |
| Real-egui-widget-per-target accessibility recommendation | **Forward guidance for Pass 9a, not a Pass-12.0 deliverable** | §6.4 |

**There is no P1/P2 cut for this Pass.** Every item above is
foundation the other four beta slices (9a, 12.M1, 12.M2, 9c-min) and
the three deferred editing GUIs (6.1, 7, 8) depend on directly; there
is no "ship the easy half, defer the rest" cut that leaves a coherent
substrate behind. If schedule pressure forces a cut, the cut is at the
BETA level (ship fewer of the five slices, per decision 011 §3's own
two-step fallback), not inside this one.

---

## 10. Open items for the librarian

1. **`canvas_to_pdf_space`/`pdf_space_to_canvas` (§2.3) is a genuine new
   finding, not present in decision 011's literal Pass-12.0 deliverables
   list** — a real correctness gap none of the three prior UI specs
   (`pass-6.1`, `pass-7`, `pass-8`) surfaced, because none of them has a
   shipped commit path that exercises it yet. Worth a one-line addition
   to decision 011's Pass-12.0 record (or a note at the next
   `ARCHITECTURE.md`/`ROADMAP.md` touch) so a future reader sees this
   was found and closed here, not overlooked.
2. **`CanvasTool` renamed from `pass-6.1`/`pass-8`'s proposed
   `MarkupTool`** (§0's rename table) — when Pass 6.1's and Pass 8's GUI
   slices are actually implemented, every `MarkupTool`/`Action::
   SelectMarkupTool` reference in those two spec files should be read as
   `CanvasTool`/`Action::SelectCanvasTool`. Worth a corrective pointer
   note on both spec files (or their ROADMAP entries) the next time
   either is touched, so a future reader is not confused by the name
   mismatch between an old spec file and the type that actually exists.
3. **Marquee-vs-pan disambiguation is explicitly Pass 9a's decision**
   (§4.2) — not resolved here, not silently assumed. Flag it as a
   required design question at Pass 9a kickoff, alongside the
   already-known object-model prerequisites decision 011 names.
4. **Global-vs-focused keyboard dispatch is explicitly Pass 7's
   decision** (§6.3) — `collect_keyboard_actions`'s current
   unconditional-every-frame model will need reconciling with a real
   focused `TextEdit` overlay. Flag at Pass 7 kickoff.
5. **Real-egui-widget-per-selectable-target, weighed against per-object
   `ui.interact` cost at real page densities, is Pass 9a's call**
   (§6.4) — named as an option to evaluate deliberately, not decided
   here and not silently defaulted to painter-based hit-testing.
6. **This Pass adds no new instance of the five-way placement
   taxonomy** (§6.1) — the taxonomy's six and seven recorded instances
   (`pass-6.1` §4.1, `pass-8` §3.1) are unchanged; worth noting in the
   same taxonomy-history record so a future reader does not go looking
   for an eighth instance that does not exist.
