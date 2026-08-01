# Pass 4 UI Spec — Text Extraction ("Copy text")

> Authored by pdfce-ui-specialist, 2026-07-31, on dispatch from the
> engineer/orchestrator, as a **retrospective design record**, not a
> pre-implementation spec: unlike `pass-3.2-page-ops.md`, this Pass's
> P0/P1 items (1–7) were already applied in code before this document
> was written. Its job is (a) to capture the reasoning behind what
> shipped so a future session does not have to re-derive it by reading
> diffs, (b) to name and generalize the placement pattern this Pass
> discovered, and (c) to give the deferred canvas-selection slice a
> real design instead of a one-line backlog note. The specialist read:
> `crates/pdfce-gui/src/main.rs` (the copy-text state, gate, status-bar
> surface, toolbar menu, keyboard bindings, and both confirmation
> windows), `crates/pdfce-gui/src/ui_text.rs` (the full "Copy text
> (Pass 4)" catalog section and the existing Tools-dock/toolbar
> sections it had to fit beside), `crates/pdfce-gui/src/viewer.rs` (the
> page↔screen geometry the deferred slice must reuse),
> `crates/pdfce-core/src/text_extract/mod.rs` and `font.rs` (the
> sourced/derived model, `TextOrigin`, `ExtractedGlyph`, `LadderRung`),
> and `docs/ui_specs/pass-3.2-page-ops.md` (the rail/dock convention
> and the confirmation-dialog precedent this Pass extends).
>
> **No defects found in the design decisions themselves** — every
> claim in the engineer's summary checked out against the code exactly
> as described. One implementation defect **was** found while reading,
> orthogonal to the design: see §3.1.

---

## 1. The third placement pattern: read-only in-document utility

`pass-3.2-page-ops.md` §1 states the rule as a binary:

> If the operator's argument is a set of pages already visible in the
> open document, the control lives on the **thumbnail rail**. If the
> operator's argument is something from outside the currently open
> document, the control lives in the **Tools dock**.

Copy-text is neither. Its argument is the open document (or its
current page), which sounds like the rail's condition — but the rail's
whole apparatus (checkbox multi-select, anchor+range, a selection
action bar that only appears once something is checked) exists to let
an operator pick a **subset** of pages to act on. Copy-text has no
subset to pick: its two scopes are "the page already on screen" and
"all of them," neither of which needs a selection model at all. And
it is obviously not dock material — the dock's own intro sentence,
verbatim in `ui_text::tools_dock_intro`, is *"These tools work with
files outside the one you have open."* Putting Copy-text there would
make that sentence false the moment an operator read it.

**The discriminator that resolves this, stated once for reuse:**

> If the control's argument is the open document as a whole (or its
> currently-displayed page), the action is **read-only** — it cannot
> write to the file, full stop, not "not yet wired to undo" — and it
> needs no rail-style selection model to express its scope, it is an
> **ungrouped toolbar utility button**, governed by the same "toolbar
> capped at N groups + K utility buttons" discipline `pass-3.2`
> established for the Tools toggle. A plain button when there is
> exactly one reasonable scope; a small menu (`egui::menu_button`) when
> the operator must pick between more than one, same as Copy-text's
> page-vs-document choice.

This is a **founding default**, not yet battle-tested against a third
occurrence — flagging per the standing-rules note at the top of this
agent's brief.

**Applying the test forward, so it earns its keep immediately:**

- **Bates preview.** The *preview* step (show what a proposed numbering
  range would look like before committing) is read-only and could, in
  isolation, pass this test. But Bates stamping as a *feature* is not
  read-only — the commit step writes a stamp onto every page, is a
  batch operation over the whole document, and needs a settings form
  (prefix, start number, position/format) before the preview can even
  render. That shape is `Split`'s shape (`pass-3.2` §3.8: criteria →
  live preview → commit), not a toolbar-button shape. **Verdict: Bates
  stamping belongs in the Tools dock, reusing Split's
  settings-then-preview-then-commit convention** — the read-only
  preview lives *inside* that dock flow, not as its own toolbar entry.
- **OCR "recognize this page."** This one **fails** the discriminator
  outright: it writes an invisible text layer onto the page. It is not
  external-file scoped either, so it isn't dock material by the
  original rule. What it actually resembles is single-page rotate
  before Pass 3.2 batch-generalized it — a per-page write action whose
  natural home is the rail (a page, or a rail multi-select of pages,
  is exactly "a set of pages already visible in the open document").
  Whether OCR ships as a rail action on the current/selected page(s)
  or needs its own batch-settings dock flow (language, DPI, whether to
  replace an existing text layer) is a real question, but it is **not
  this pattern's question** — the discriminator's job here is telling
  you *not* to reach for the toolbar-utility answer, which is itself
  useful: it heads off a category error before a session burns time on
  it. Route OCR's actual placement to a dedicated
  `pdfce-ui-specialist` review when that Pass is scoped, per
  `CLAUDE.md`'s own dispatch rule.

---

## 2. The shipped surface, and why each piece is shaped the way it is

### 2.1 Toolbar placement

One new control: `ui.menu_button(ui_text::copy_text_button(), …)` (label
`"📋 ▾"`), placed immediately left of the Tools toggle, as a second
**ungrouped** utility button rather than a seventh toolbar group.
Verified in `main.rs`: it sits in neither the view group nor the edit
group, and the code comment at the call site states the reasoning
identically to §1 above — it belongs to neither because "does this
button change my document?" answers *No, structurally* for Copy-text,
which would make either group's own organizing question unanswerable
at a glance if it were forced in.

The trailing chevron is doing real discoverability work, not
decoration: it tells the operator *before* they click that this opens
a choice rather than acting immediately — a plain button here would
have to guess a scope, which is exactly the guess this feature exists
not to make.

### 2.2 Status-bar surface: its own header, deliberately not merged

`copy_result_bar` renders in its own `CollapsingHeader`
(`id_salt("copy-diagnostics")`), positioned with `save_result`/
`edit_note` at the top of `status_bar`, and explicitly **not** folded
into the render-diagnostics header directly below it. The reasoning,
recorded in both the `copy_result` field doc and the function doc, is
a lifecycle distinction worth generalizing beyond this one Pass:

> Render diagnostics are re-derived from the current page's cached
> texture **every frame** and are therefore always a claim about what
> is on screen right now. A copy result is a **snapshot of a
> completed, operator-triggered action**, possibly against a different
> page or the whole document, that must persist after the operator
> navigates away. Merging the two would make the merged header start
> lying the instant the operator turned the page.

Generalization for future Passes: **any status-bar fact whose truth
value can change just by turning the page must not share a header
with any status-bar fact that is pinned to the moment it was
produced.** This is the same "did my last requested action work"
family as `save_result`/`edit_note`, and Copy-text correctly joins it
rather than starting a fourth notification style.

The summary line names its scope explicitly (`"…from page 4"` /
`"…from all 12 page(s)"`) for the identical reason — a summary that
just said "text copied" would become ambiguous, then wrong, the moment
the operator moved to a different page.

### 2.3 The pre-copy reliability gate

`begin_copy_text` gates on:

```rust
d.identity_fonts_without_to_unicode > 0 || d.sourced_fraction().unwrap_or(1.0) < 0.5
```

Confirmed as two *different kinds* of signal, not one threshold
duplicated:

- `identity_fonts_without_to_unicode > 0` is **structural** — pdfce
  knows, before counting a single failed code, that ISO 32000-1
  §9.10.2 itself leaves no rung available for an `Identity-H`/
  `Identity-V` font with no `/ToUnicode`. This is a `shall`-adjacent
  fact about the file, not a magnitude judgement.
- `sourced_fraction() < 0.5` is a **magnitude backstop** for every
  other failure cause, deliberately loose. The code comment records
  the corpus measurement backing the 0.5 choice: 99.78% sourced across
  the measured corpus, so a tighter trigger (5–10%) would fire on
  ordinary documents carrying one odd symbol font — and a confirmation
  that fires on ordinary documents is one operators learn to
  reflexively click through, which defeats the entire point of having
  it. **This is the correct application of "discoverable destructive,
  frictionless trivial" (rule 7) to a fuzzy/never-sneaky gate**: the
  gate must be rare enough that seeing it means something.

The confirmation itself (`copy_confirmation`) is asked **before** the
clipboard write, matching the exact reasoning already established for
`pending_save`: a clipboard write destroys whatever the operator had
copied previously, and an operator backing out must not first lose it.
`PendingCopy` stores the extracted text and its counters rather than
re-extracting on confirm — same reasoning as `PendingSave` storing its
verdict rather than recomputing it: the question the operator answered
was about *this* result, and a re-extraction after some unrelated frame
(the operator may have turned the page while the dialog sat open)
could silently answer a different one.

### 2.4 Two-tier disclosure

The detail expander separates two facts that are **not a severity
ranking** — they are two different *kinds* of claim:

- **Tier 1 (uncoloured): derived whitespace.** Stated plainly, no
  colour. ISO 32000-1 §14.8.2.5/S1–S4 guarantee **no** inter-word
  signal at all outside a Tagged PDF, so nearly every real document
  needs some derived spacing — colouring this as a caution would teach
  operators to distrust something true of almost every PDF they will
  ever open. This is the correct generalization of rule 4
  ("fuzzy, never sneaky"): visibly marking a guess does not require
  visually *alarming* about it when the guess is the normal case, not
  the exceptional one.
- **Tier 2 (warn-coloured + marked): ladder failures.** Genuinely
  uncertain content — U+FFFD replacement characters the file gave
  pdfce no way to read. Warn-coloured **and** carries the `⚠` mark in
  the headline text itself, satisfying rule 6 (colour is never the
  sole signal) the same way `diagnostics_summary` already does for
  unsupported render items — reusing an established convention rather
  than inventing a second "this needs your attention" visual language.

`codes_total == 0` (a page with genuinely no extractable text — the
scanned-page-with-no-OCR-layer case) is handled as its own named
sentence (`copy_text_no_extractable_text`) rather than falling through
to a "successful" copy of nothing: an operator who pastes nothing and
is told nothing cannot distinguish "no text here" from "broken
button." Verified in `main.rs`; the message names the likely cause
(scanned page, no OCR yet) rather than stopping at "nothing found,"
which is the actionable half of the sentence.

### 2.5 Keyboard binding

`Ctrl+Shift+C` → page-scope copy. Plain `Ctrl+C` is **deliberately
left unbound**, with an explicit comment reserving it for the deferred
canvas-selection slice (§4). This is the right call: binding `Ctrl+C`
to page-copy now, then re-binding it to selection-copy once the canvas
slice ships, would mean taking a muscle-memory chord back from
operators after they'd learned it — worse than the chord not existing
yet. Whole-document copy is intentionally menu-only, not chorded — it
is both rarer and the one with a visible, undisclosed-until-you-hover
delay on a long file (§2.6 and §5, item 9).

### 2.6 `ui_text.rs` catalog — recorded for completeness

18 entries under `// Copy text (Pass 4 — ISO 32000-1 §9.10 text
extraction)`, with a header comment recording exactly the dock-vs-
toolbar reasoning of §1 and the two-tier disclosure reasoning of §2.4
— worth calling out because that header comment is itself doing the
job this document is doing at smaller scale, in the one place a future
reader is guaranteed to look before adding a ninth entry to the same
section. All entries follow R2 (one complete templated sentence per
entry), R3 (no hard-sized layout — the detail panel is a
`CollapsingHeader`, not a fixed-height box), R6 (the one percentage
figure, `copy_text_sourced_percent`, formats its own number rather than
an inline `format!` at the call site).

---

## 3. The confirmation-dialog convention: now a pattern, not a one-off

`copy_confirmation` is byte-for-byte the same shape as
`signature_confirmation`: a plain `egui::Window`, `.collapsible(false)`,
`.resizable(false)`, `.anchor(Align2::CENTER_CENTER, Vec2::ZERO)`,
`ui.set_max_width(520.0)`, added last in frame order, Cancel-then-
Confirm button pair. The `copy_confirmation` doc comment already notes
this explicitly ("this is now the app's *second* blocking confirmation
and a second dialog style would be a second thing for the operator to
learn").

**This is owed a `docs/decisions/` entry.** With one use, the shape was
implicit in `pending_save`'s own reasoning; with two, it is a
convention an operator will start to *recognize* ("oh, this is one of
those stop-and-ask windows"), which is exactly the kind of pattern this
project's decision log exists to pin down before a third use invents a
slightly different variant by accident. The engineer relays this to
`pdfce-librarian`; recorded here only so the obligation isn't lost.

### 3.1 An implementation defect found while verifying this section

The shared convention's own doc comment (both copies, verbatim in
`signature_confirmation`) claims: *"added last so it draws over every
panel, **with the rest of the UI disabled underneath**."* This is not
implemented. I searched `main.rs` for any input-disabling gate
(`ui.disable`, `add_enabled_ui` keyed on `pending_save`/`pending_copy`,
or an early-return guard in `apply()`) and found none. Concretely:

- Every toolbar button, rail action, and dock control remains fully
  clickable while either confirmation window is on screen — only the
  screen region the ~520px-wide centred window physically covers is
  blocked by paint order; everything else is live.
- `collect_keyboard_actions` is a free function taking only `&egui::
  Context`, with no visibility into `self.pending_save`/`pending_copy`
  at all, so **every keyboard shortcut fires regardless of a pending
  confirmation** — `Ctrl+S` or `Ctrl+Shift+C` are consumed and
  dispatched into `apply()` unconditionally, every frame, whether or
  not a confirmation is currently blocking that same action's earlier
  invocation.
- Concrete reachable failure: an operator opens the unreliable-copy
  confirmation (`pending_copy = Some`), then presses `Ctrl+S` before
  answering it. `begin_save` runs, and if the save also turns out to
  invalidate a signature, `pending_save` becomes `Some` too. Both
  windows are centre-anchored at the same size, so they render
  **exactly on top of each other** — the later-painted one
  (`copy_confirmation`, called after `signature_confirmation` in `ui()`)
  is the only one that can receive clicks; the earlier one is
  invisible-in-practice and unanswerable, sitting in `self.pending_save`
  until something else happens to overwrite or clear it.

This did not originate in Pass 4 — the doc-comment overclaim and the
missing guard both predate it, in `pending_save`. Pass 4 makes it a
live bug rather than a latent one, because it introduces the second
independent pending-state that can now collide with the first. **Fix
recommendation for the engineer:** gate the top of `apply()` (or the
action-collection step) on `self.pending_save.is_some() ||
self.pending_copy.is_some()` and only let the matching
`Confirm*`/`Cancel*` action for whichever is set through — everything
else drops for that frame. This one guard closes both the click-through
gap and the keyboard-bypass gap, and should land before the confirmation
convention gets its decision-log entry, since the entry should describe
what the convention actually does, not what its comment currently
(incorrectly) claims it does.

**Minor, non-blocking, found in the same pass:** a document with
`/Count 0` (a real, spec-legal degenerate case `viewer.rs`'s own
comments already anticipate) reaches `begin_copy_text` with
`page_index == 0` against zero pages, fails extraction, and surfaces
`copy_text_no_extractable_text` — a message written for "this page has
no text" rather than "this document has no pages." Cosmetic; fix
whenever this section is next touched, not before.

---

## 4. The deferred canvas text-selection slice — a real design

This is the largest remaining gap against Acrobat parity for a feature
literally named "Copy text": there is no click-drag-select-then-`Ctrl+C`
path yet, only the two whole-scope menu actions. It is out of scope for
this Pass by size alone (§4.12), but it should not be re-designed from
scratch next time it comes up — every piece below was reasoned through
against the actual code that will carry it.

### 4.1 Geometry reuse — no new transform, extend `viewer.rs`

`viewer.rs` already fully defines the page↔screen mapping this slice
needs:

- `doc.current_extent()` — the page's on-screen extent in PDF
  user-space points, **with `/Rotate` already applied** (delegates to
  `pdfce_render::page_device_geometry`, per the module's own stated
  reason for not reading `page.crop_box` directly: one function, so the
  GUI's idea of page size and the renderer's idea of pixmap size cannot
  drift apart).
- `doc.view.zoom` — device pixels (well, logical points) per user-space
  unit.
- The canvas paints via `egui::Image::from_texture(&texture)
  .fit_to_exact_size(display_size)` where `display_size = extent *
  zoom`, inside a `ScrollArea`; the `Response` from that `ui.add(...)`
  carries the image's on-screen `rect`.

**Do not build a second transform.** Add one pure, unit-testable
function to `viewer.rs` — `screen_to_page(pos: Pos2, image_rect: Rect,
extent: (f32, f32), zoom: f32) -> Pos2` — that inverts exactly this:
subtract `image_rect.min`, divide by `zoom`, and flip the Y axis (PDF
user space is Y-up; egui screen space is Y-down: `page_y = extent.1 -
screen_y / zoom`). This belongs in `viewer.rs`, not `main.rs`, for the
same testability reason every other geometry function in that module
does (per its own module doc: "a windowed UI cannot be exercised
headlessly on a CI runner, but \[this kind of] arithmetic ... exactly
where an off-by-one would show up as a user-visible bug").

**Open question, flagged rather than guessed:** confirm whether
`ExtractedGlyph.x`/`.y` ("default user space" per the field doc) are in
the page's *raw* coordinate system or already reflect the same
`/Rotate` transform `page_device_geometry` bakes into `extent`. If they
are pre-rotation and `extent` is post-rotation, hit-testing a rotated
page needs the *same* rotation step `pdfce-render` already applies —
reuse whatever function it uses internally rather than re-deriving a
second rotation matrix in the GUI, which would be exactly the kind of
"GUI reasoning about PDF structure" `pass-3.2` §4 already flagged as
out of bounds for the signature-impact question. Verify against
`pdfce-render`'s transform code before writing the hit-test, don't
assume either direction.

### 4.2 Hit model: approximate glyph rects, named as an approximation

`ExtractedGlyph` gives an origin (`x`, `y`), `advance`, and `size` —
enough to build a synthetic per-glyph rect (`[x, y − size, x + advance,
y + size]` as a first approximation, or `TextRun::bbox` as a coarser
per-run fallback), but **not** a true ascent/descent box: pdfce-core
cannot read font-program metrics (R21, same limitation already named
for `fonts_with_estimated_widths`). Document the glyph-rect
approximation as exactly that in the code that builds it — the same
family of named limitation as R21, not a new kind of guess needing a
fresh operator-facing disclosure (this is internal hit-test geometry,
not an operator-visible claim about the document, so rule 4's
"visible and overridable" bar doesn't apply here — an honest code
comment is the right-sized disclosure).

### 4.3 Selection model: logical order, not screen order — `rtl_runs` is load-bearing

The standard drag-selection bug is computing "everything whose glyph
x-coordinate falls between the drag's start-x and end-x." That breaks
the instant a right-to-left run or a reordered/derived line appears,
because screen-left-to-right is **not** logical reading order the
moment either condition holds. `TextDiagnostics::rtl_runs` already
counts exactly this condition on every extraction — its presence on a
page is the live signal that a screen-x-ordered selection would
silently scramble the copied text for that page.

**Design: selection is defined and extended in logical order only.**
`PageText::runs` is already in page-content order (`TextRun`'s own doc
comment: "the sequencing of graphics objects in the content stream ...
the only \[ordering] available without a structure tree"). Represent a
selection as an anchor/focus pair of `(run_index, offset_within_run)`
over that existing sequence — never by re-sorting glyphs spatially. A
drag gesture maps each screen point to the nearest `(run, offset)` via
§4.1/§4.2's geometry, and the selection is the **logical span** between
the two anchors, so a drag that starts inside an RTL run and ends after
it still copies that run in its stored, correct order — exactly what
plain page-copy already does today. This is the same anchor+focus
interaction pdfce already shipped for rail multi-select
(`pass-3.2` §3.1's Shift+click anchor-page model) — reused, not
reinvented, just over runs/offsets instead of page indices.

**Consequence for rendering (§4.5):** because the selection is logical
rather than spatial, its highlight is the **union of each covered run's
on-screen rect**, which may not form one contiguous rectangle and, for
an RTL run, may not even progress left-to-right on screen. Document
this now so a future session doesn't "fix" the visually-scattered
highlight as a bug — it is the textually-correct rendering of a
logically-correct selection.

### 4.4 Click granularity

- **Single click** — places a caret at the nearest `(run, offset)`, no
  selection yet.
- **Double click** — selects the "word" at that offset. Reuse
  `layout.rs`'s existing `DerivedWordSpace` run boundaries as the word
  boundary rather than writing a second word-segmentation pass — this
  is a second GUI-side consumer of a signal `pdfce-core` already
  computes and exposes via `plain_text()`, so no core change is needed.
  Inside an `ActualText` run (atomic — §14.9.4 N4, already documented on
  `TextOrigin::ActualText`), double-click selects the **whole run**;
  anything finer is meaningless by the spec's own admission.
- **Triple click** — selects the "line," bounded by the nearest
  `DerivedLineBreak` runs on either side. Same reuse principle.

### 4.5 Highlight rendering

Paint semi-transparent filled rects over the rendered page texture
(`ui.painter().rect_filled`, at the geometry from §4.2/§4.3 mapped to
screen space via §4.1), reusing egui's own selection-color visual
token rather than introducing a new one, for OS/theme consistency. Not
a rule-6 violation to use colour alone here: rule 6 is about
information conveyed *only* through colour (e.g. "this is an error");
a selection highlight's meaning ("this text is selected") is redundant
with its own shape and presence on screen, not colour-dependent. **Do**
flag contrast: verify the highlight is visible against a dark scanned
page image, not only against typical white-background text — test
against a dark-fixture PDF before shipping, since a fixed low-alpha
blue that reads fine on white can vanish on a black-and-white scan
inverted at capture time.

### 4.6 Keyboard path — the canvas becomes focusable for the first time

Rule 6 requires a keyboard-operable equivalent to any drag gesture
(egui's AT support is a named, tracked gap — same reasoning already
applied to Pass 3.2's reorder chords). Shift+Left/Right extends the
selection by one logical glyph (or the atomic whole of an `ActualText`
run when the focus sits inside one); Shift+Up/Down by one derived
line, using the nearest-offset logic from §4.1 to handle differing
line lengths; Ctrl+Shift+Left/Right/Home/End for word/document-level
extension. None of these chords collide with Pass 3.2's Alt+arrow
reorder bindings (different modifier).

**This is the trigger `main.rs`'s own module doc has been waiting to
name.** It already flags: *"revisit \[the tab-order exception] when the
canvas gains focusable content — text selection, form fields."* This
slice is that moment for the first named case. What's actually required
is not a re-flag but a resolution: the canvas needs a real interactive
`Response` (an `Image` alone is not focusable — `Sense::click_and_drag`
plus explicit focus-request on click via `ui.memory_mut(|m|
m.request_focus(id))`), placed so Tab reaches it in the position
`pass-3.2` §1 already reserved for it (toolbar → rail → dock → canvas —
the canvas is already last, so the *order* needs no change, only that
the canvas widget itself now participates in it).

### 4.7 `Ctrl+C` binding — finally given real, unambiguous meaning

Plain `Ctrl+C` copies the current selection's text (same `plain_text`
semantics — sourced characters plus derived whitespace — restricted to
the selected runs/offsets) **only when a selection is non-empty.** With
no selection, `Ctrl+C` does **nothing** — it must never silently fall
back to "copy the whole page," which would violate predictability the
same way a Copy button that guessed a scope would (§2.1): an operator
expecting "copy what I selected" must never receive "copy everything"
instead, silently. This resolves the reservation `main.rs`'s existing
comment already names for this exact chord.

### 4.8 Right-click override — `sourced_text()`, deferred item 10, now placed

A context menu on an active selection (or, with no selection, on the
caret — matching the page/whole-document scope split already shipped)
adds one item beside the default copy: `plain_text()` semantics is
the click-drag/`Ctrl+C` default; the right-click menu additionally
offers a `sourced_text()`-scoped copy — every `DerivedWordSpace`/
`DerivedLineBreak` run in the span dropped — for when derived spacing
is producing a bad paste. Tooltip must name the operator-visible
symptom, not the ISO clause, matching `copy_page_text_tooltip`'s
existing plain-English convention: something like *"words or lines run
together in your paste? try this."* This is P1 item 10, and it is
correctly deferred together with the rest of this slice rather than
being a standalone gap — it is meaningless without a selection to scope
it to, and there is no right-click surface on the canvas at all today.

### 4.9 The per-selection reliability gate — `pdfce-core` API gap: **verified closed**

The brief asked this to be checked against `text_extract/mod.rs` before
being written up as open. It is closed. `ExtractedGlyph` already
carries `rung: LadderRung` **per glyph** (`font.rs`) alongside full
per-glyph geometry (`x`, `y`, `advance`, `size`, `text_start`,
`text_len`). `TextRun::origin: TextOrigin` is per-run. Together this is
exactly the granularity a per-selection failure rate needs: given a
selection's `(run, offset)`-to-`(run, offset)` span, walk the covered
glyphs in every `Glyphs`-origin run inside that span and tally `rung`
values directly — an aggregation over data `pdfce-core` already
exposes, needing **no core API addition.** The one real caveat: a
selection that includes any part of an `ActualText` run can only be
scored at the run level (the run is atomic by the spec's own
admission — no glyph-level mapping exists to score more finely), so
such a run contributes "1 unit, sourced" to the tally rather than N
characters. Coarser, but correct, and already anticipated by the
module's own docs. **Conclusion for the engineer: build the per-
selection gate as a pure GUI-side (or `pdfce-core`-adjacent free
function operating on borrowed `TextRun`/`ExtractedGlyph` data) tally —
do not file this as a `pdfce-core` feature request.**

### 4.10 Accessibility gap to record

egui cannot today expose a custom-painted, geometry-based text
selection as selectable/announceable text to a screen reader — the
same named, tracked gap already recorded against Pass 3.2's drag-and-
drop reorder (rule 6's "note explicitly when egui's current
accessibility support ... can't yet deliver something"). The mouse and
Shift+Arrow paths (§4.6) give a sighted keyboard-only operator full
access; an AT user gets no equivalent of "announce the current
selection" from egui itself. Record this as a second occurrence of one
tracked gap, not a new gap — the librarian should link the two rather
than filing twice.

### 4.11 Priority

By component count alone — a new hit-test transform, a logical-order
selection state machine, three click granularities, highlight
rendering with a contrast check, a first-ever focusable canvas widget,
two copy semantics, and a context menu — this slice is larger than
Pass 3.2's Merge/Split GUI, which that Pass's own priority table
already flagged as "reasonable to split into a follow-up slice." Same
call here: **a dedicated Pass**, not a tail addition to whatever ships
next.

---

## 5. Residual P1 items — status

| Item | Status | Note |
|---|---|---|
| 8. `copy_text_pages_without_text_note` for document scope | **Done** | Verified wired in `copy_result_bar`, gated on `scope == Document && pages_without_text > 0`. Remove from backlog; it was never actually deferred. |
| 9. Background-thread whole-document extraction | **Ship if room; recommend a cheaper interim fix first** | Not benchmarked in-process. The disclosed-delay tooltip (`copy_document_text_tooltip`) is an honest interim per rule 4, but a frozen window with no visible activity reads as a crash to an operator regardless of what the tooltip said five seconds earlier. Before reaching for a background thread (a bigger change — needs a cancel path, a result racing against a page-navigation, etc.), add a busy cursor/spinner for the duration of `extract_document`, which closes most of the "did it crash" risk for a fraction of the engineering cost. Escalate to P0 fast-follow only if in-process measurement on a real 400+-page fixture shows a multi-second freeze. |
| 10. Right-click "Copy without inferred spacing" | **Correctly deferred into §4.8** | Not a standalone gap — meaningless without a selection to scope it to, and the canvas has no right-click surface yet. Do not re-scope as an isolated P1; it ships with the selection slice or not at all. |

---

## 6. Undo/write-path summary (for consistency with `pass-3.2`'s table)

| Operation | EditSession command? | Writes anything? |
|---|---|---|
| Copy this page's text | No | No — read-only, structurally cannot touch the file |
| Copy the whole document's text | No | No |
| (deferred) Copy selection | No | No |

Do not wire any Copy-text variant into the undo stack "for
consistency" — same instruction `pass-3.2` §3.9 gives for Extract/
Merge/Split, for the identical reason: there is genuinely nothing to
undo, and forcing an undo entry onto a read-only action would be
ceremony without meaning.
