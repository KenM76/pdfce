# Pass 3.2 UI Spec — Structural Page Operations

> Authored by pdfce-ui-specialist, 2026-08-01, on dispatch from the
> engineer/orchestrator. This is the implementation spec for Pass 3.2's
> GUI surfaces; the engineer implements it verbatim, deviating only with
> a recorded reason. The specialist read: pdfce-gui main.rs / ui_text.rs
> / viewer.rs, pdfce-core document.rs + writer/mod.rs doc comments,
> ARCHITECTURE.md §5.4–5.7 + §11.1–11.5, ROADMAP.md Pass 3.2 entries,
> D:\dev\rag\egui\index.md.
>
> RECORD DEFECT FLAGGED (for the librarian, not the engineer):
> ARCHITECTURE.md §5.4 attributes **R36** to the
> linearization-never-repaired rule, but document.rs and writer/mod.rs
> both attribute **R36** to the signature/redaction either-or rule
> (decision 007 W7). Two different rules share one number. This spec is
> written against the writer/mod.rs usage (R36 = signature either-or).

---

## 1. Secondary-surface convention (one-time decision, settled here)

**Decision: docked, not floating, for everything new in this Pass.**
Formalize main.rs's "right side reserved for a future contextual/tool
panel" comment into an actual panel now, rather than adding a fourth
surface type.

**The rule, stated once, for every future Pass to apply without
re-litigating it:**

> If the operator's argument is a set of pages already visible in the
> open document, the control lives on the **thumbnail rail** (selection
> + contextual action bar). If the operator's argument is something from
> outside the currently open document — a file, a folder, a set of
> files — the control lives in the **Tools dock**, a persistent
> right-side panel toggled from the toolbar. Floating windows are
> retired for new tool surfaces; Properties (Pass 3.1) stays as the one
> legacy exception because it's a short, occasional, spatially
> unanchored metadata edit — don't migrate it, and don't add a second
> one of its kind.

Concretely: reorder/delete/extract/rotate-batch → rail;
insert/merge/split → Tools dock. Nothing in Pass 3.2 needs a new
floating window.

*(Extension, 2026-08-01, Pass 4: this binary is now a THREE-way
taxonomy — a read-only snapshot action over the whole open document
(first instance: Copy-text) fits neither rail nor dock and lives as an
**ungrouped toolbar menu button**, under the same toolbar-cap
discipline as the Tools toggle. The two rules above stand unchanged;
the third pattern's discriminator is stated once for reuse in
`pass-4-text-extraction.md` §1, and the audit trail is
`ARCHITECTURE.md` §12, continuation-20 entry (b).)*

**Panel add-order (load-bearing, per main.rs's own documented
reasoning):** the bottom status panel must stay full-width, so anything
that claims left OR right space must be added to the `Ui` **before**
it, exactly like the existing rail. New order:

```
1. toolbar          (top)
2. status           (bottom)
3. thumbnail rail   (left)   — if rail_expanded
4. Tools dock       (right)  — if tools_open
5. CentralPanel     (canvas)
6. properties_window(ctx, …)   — last, floating-over-everything, unchanged
```

Tab order becomes toolbar → rail → dock → canvas. The canvas still
holds no focusable widgets in Pass 3.2 (page-ops live in rail/dock, not
canvas), so the existing "revisit when the canvas gains focusable
content" caveat carries forward unchanged — inherited debt, not new
debt.

**This IS the rule-3 escape valve, generalized:** the Tools dock is
pdfce's one "more tools" secondary surface. Future advanced buckets —
Bates stamping, OCR, redaction, forms, portfolios, PDF/A conversion —
become entries in this same dock's tool list, not each a new floating
window or a new toolbar group. (Decision-log-worthy pattern; librarian
captures it with the Pass 3.2 filing.)

---

## 2. Toolbar growth strategy

The toolbar today already has 6 separator-divided groups (file / view /
navigation / zoom / edit / history) plus the right-aligned status
summary. Pass 3.2's naive ~6 buttons would make it 7+ groups and start
wrapping/crowding.

**Concrete fix: add exactly one new toolbar control, a single toggle
button, same pattern as the existing rail toggle.**

```rust
ui.separator();
if ui.add_sized(ICON_BUTTON_SIZE, egui::Button::new(ui_text::tools_button()))
    .on_hover_text(ui_text::tools_tooltip())
    .clicked()
{ actions.push(Action::ToggleTools); }
```

Net toolbar change for the whole Pass: **+1 button**, not +6.
Everything else lives off-toolbar:

| Feature | Surface | Why it's not a toolbar button |
|---|---|---|
| Delete, Extract, Rotate-batch | Rail selection action bar | Meaningless with 0 pages selected — a toolbar button would be permanently disabled-and-mysterious (violates discoverability) instead of appearing right where the pages are |
| Reorder | Rail drag + keyboard | Not a "click a button" action at all |
| Insert / Merge / Split | Tools dock | Argument is external to the open document |

**Standing rule for future Passes** (librarian captures): the toolbar is
capped at its current 6 groups + the Tools toggle. Any future feature
either fits one of those 6 groups, becomes a rail-contextual control,
or becomes a Tools-dock entry. No feature gets a 7th toolbar group
without a fresh review.

---

## 3. Page-ops interactions

### 3.1 Rail multi-select model

Extend `thumbnail_rail` (currently single-select-via-click). Two
coexisting affordances, so the feature is discoverable both by seeing
it and by knowing the accelerator:

- **A small checkbox** (18×18 min, matching ICON_BUTTON_SIZE's
  click-target discipline) painted in each thumbnail's top-left corner,
  allocated and interacted **separately** from the thumbnail's own
  click-`Sense`, so clicking the checkbox toggles selection without
  navigating, and clicking the thumbnail body still navigates (Pass 1
  behavior unchanged). ⚠️ Implementation note: verify in egui 0.35
  which of two overlapping `interact`s wins the hit-test when the
  checkbox rect is inside the thumbnail rect — don't assume, test it.
- Checkbox glyph: filled + a check-mark glyph when selected,
  outlined-empty when not — never color alone (rule 6). Reuse the
  existing selection-ring pattern (stroke, not fill-color-only) for
  "this is the currently viewed page," which is already correct.
- **Keyboard:** Space toggles the focused thumbnail's selection;
  Shift+click / Shift+Arrow does a contiguous range select from the
  last-clicked "anchor" page. Esc or a "Clear selection" click empties
  it.
- **Selection action bar** — a new `ui.horizontal` row pinned above the
  rail's `ScrollArea`, present **only when `selected.len() >= 1`**
  (hidden, not disabled, matching the existing Save-button precedent —
  nothing to discover about a batch action with zero pages picked):

```
[ ui_text::selection_bar_summary(n) ]   ↺   ↻   Delete   Extract…   Clear selection
```

Rotate/Delete/Extract here are the SAME glyphs/buttons already shipped
for single-page rotate — reused, not duplicated, and their tooltip text
switches on `count` (catalog entries below).

### 3.2 Delete

- Trigger: selection-bar "Delete" button, right-click context menu
  "Delete page(s)", or Delete/Backspace when the rail has focus and ≥1
  page selected.
- **No confirmation dialog.** Per rule 7, this is reversible pre-save
  (undo). Matches the working Pass 3.1 pattern (rotate's tooltip states
  "use Undo to reverse it" instead of a modal) — replicate it, don't
  invent a new confirmation style for a reversible action.
- **Two honesty additions are mandatory** (both new applications of
  fuzzy-never-sneaky, extensions of rule 2):
  1. **Dangling-reference disclosure.** If any bookmark/outline entry
     or link annotation targeted a deleted page, a status-bar narrator
     line appears immediately after the delete commits (same
     channel/style as the save-result line, not a new one): *"N page(s)
     deleted. M bookmark(s)/link(s) pointed at a deleted page and now
     point nowhere — nothing else was changed."* **This needs a small
     pdfce-core addition**: the delete-page command's result must
     report a dangling-reference count. The GUI cannot compute this
     itself without independently walking the outline/annotation graph
     (would duplicate core logic) — a core dependency, not a GUI fake.
  2. **"Delete ≠ redaction" framing.** An operator who deletes a page
     believing the content is now gone (privacy/confidentiality) is
     wrong under the default incremental save mode — §5's
     non-destructive-by-default model means the removed page's objects
     are *omitted from the new page tree*, not *scrubbed from the
     file's bytes*. The Delete button's tooltip must say so:

     > "Remove the selected page(s) from this document. This can be
     > undone until you save. Note: like every edit here, this does not
     > erase the removed page's data from the file — the previous
     > version can still contain it. If you need to permanently destroy
     > content, use Redaction (not available yet), not Delete."

### 3.3 Reorder

- Drag a selected thumbnail (or the whole current selection) to a new
  slot; a thin insertion-line indicator between two thumbnails tracks
  the pointer; dragged item(s) render ghosted/semi-transparent
  following the cursor. On drop: **one** EditSession command for the
  whole move, however many pages moved — exactly §11.3's named
  allowance ("reordering 50 pages in one drag operation" → one coarser
  before/after page-order snapshot command). Don't build a
  per-page-move command stack.
- **Mandatory keyboard equivalent** (rule 6 — drag-and-drop is not
  keyboard-operable; egui's AT support is a known, tracked gap, so this
  is the compensating path, not optional): "Move up" / "Move down" in
  the selection bar, plus a chord (e.g. Alt+↑ / Alt+↓) that moves the
  current selection one slot when the rail has focus.
- Implementation flag: needs a persisted `dragged_index: Option<usize>`
  / `drop_target: Option<usize>` (or the `egui_dnd` crate, if the
  engineer wants to add it — a dependency-licensing call per CLAUDE.md
  rule 13, not the specialist's). Either way this is egui::Id-scoped
  session state per frame, not derivable positionally.

### 3.4 Rotate-in-batch

- Reuses the existing rotate glyphs/buttons; when `count == 1` and that
  page is the currently viewed page, keep the exact Pass 3.1 wording
  unchanged (no regression). When `count > 1`, switch to the new batch
  wording (catalog below).
- **Fix the Pass 3.1 gap while here:** rotate still has no keyboard
  shortcut at all. Pick an unclaimed chord (e.g. `[` / `]`, common in
  image viewers, not an Acrobat binding) and verify against
  `collect_keyboard_actions` for collisions.
- Undo: one command per click, list of `(page, old_rotate, new_rotate)`
  pairs — same §11.3 pattern as reorder.

### 3.5 Extract

- Selection-bar "Extract…" button. **This is NOT an EditSession edit** —
  it doesn't mutate the open document at all, so it creates no undo
  entry and doesn't touch the "unsaved changes" marker. It behaves
  exactly like Pass 3.1's "Save a copy," restricted to the selected
  page subset: click → native save dialog (suggested name e.g.
  `{stem} (pages 3-5).pdf`) → `write_atomic` → the same
  SaveOutcome-style persistent status-bar line. Do not invent a second
  save-flow style for this.
- If the open document carries a signature, add one informational
  (non-blocking) note after a successful extract: *"This extracted file
  does not carry the original document's signature — the file you
  opened is untouched."* Expectation-setting, not a warning — nothing
  about the source document changed.

### 3.6 Insert (Tools dock)

Widget tree inside the dock when "Insert pages…" is selected:

```
1. Button: ui_text::insert_pages_choose_file_button()  → rfd::FileDialog::pick_file (PDF filter)
2. (once a file is picked) mini thumbnail strip of the SOURCE document
   — same ThumbnailCache/checkbox-select pattern as the main rail, reused
   — default: all pages selected
3. ComboBox / segmented control: ui_text::insert_pages_position_label()
   [Before current page] [After current page] [At start] [At end]
   default = "After current page"
4. Button: ui_text::insert_pages_commit_button()
   — disabled + hint (ui_text::insert_pages_none_selected_hint()) while 0 source pages picked
```

One EditSession command for the whole insert (N pages, position P),
regardless of N. pdfce-core does cross-document renumbering; the GUI
only passes source path + selected indices + target position.

### 3.7 Merge (Tools dock)

```
1. Button: ui_text::merge_add_files_button()  → rfd::FileDialog::pick_files (multi-select)
2. Ordered list (one row per file):
   [drag handle / ▲▼] name — ui_text::merge_pending_page_count(n)  [✕ remove]
   — if a document is open, it appears pre-populated as the first row,
     labeled via ui_text::merge_current_document_label(path); operator may remove it
3. Button: ui_text::merge_commit_button() — disabled + ui_text::merge_needs_two_files_hint()
   while fewer than 2 files are listed
   → native save dialog (suggested name via ui_text::suggested_merge_name)
   → write_atomic → status line via ui_text::merge_succeeded(...)
```

No EditSession involvement — Merge produces a brand-new file from N
untouched source files; nothing to undo. If any source carries a
signature, one informational note (`ui_text::merge_note_unsigned_output`)
after success, same non-blocking framing as Extract's.

### 3.8 Split (Tools dock)

```
1. Radio group, ui_text::split_criteria_label():
   ○ ui_text::split_every_n_label()  [stepper: N]  ui_text::split_every_n_suffix(n)
   ○ ui_text::split_after_pages_label()  [TextEdit, hint: split_after_pages_hint()]
       — inline validation per entry; invalid tokens shown in error_fg_color with
         ui_text::split_after_pages_invalid(entry, page_count) — never silently
         clamped or dropped
   ○ ui_text::split_at_selection_label()  — uses the RAIL's current multi-select
       as break points (ties this criterion back to the rail per the hybrid rule)
2. TextEdit: ui_text::split_naming_pattern_label(), hint: split_naming_pattern_hint()
   (template placeholders {stem} {n} {start} {end}; operator-edited value always wins)
3. Read-only preview list, live-updating as criteria/pattern change:
   Heading: ui_text::split_preview_heading()  ("nothing is written until you click Split")
   Row per output: ui_text::split_preview_row(name, first_page, last_page)
4. Button: ui_text::split_commit_button()
   → rfd::FileDialog::pick_folder
   → if any generated name collides with an existing file in that folder:
       a real confirmation (below) — NOT reversible via Undo (files, not
       edits), so it earns rule 7's "discoverable destructive" treatment
   → write each output atomically → one status line: ui_text::split_succeeded(count, folder)
```

Collision confirmation (only when needed, not by default):
```
egui::Modal — ui_text::split_collision_warning(names)
[ split_collision_cancel_button() ]   [ split_collision_confirm_button() ]
```
No EditSession involvement, same as Merge.

### 3.9 Undo semantics — consolidated table

| Operation | EditSession command? | Undo granularity |
|---|---|---|
| Delete | Yes | 1 command for the whole selection |
| Reorder | Yes | 1 command (before/after snapshot, §11.3) |
| Rotate-batch | Yes | 1 command for the whole selection |
| Insert | Yes | 1 command for the whole insert |
| Extract | **No** | nothing to undo — new file, source untouched |
| Merge | **No** | nothing to undo — new file, sources untouched |
| Split | **No** | nothing to undo — new files, source untouched |

Do NOT wire Extract/Merge/Split into the undo stack "for consistency" —
they genuinely have no undo story; forcing one would be ceremony
without meaning.

---

## 4. Signed-document interplay

**Load-bearing framing:** per §11.1, the dirty set (and therefore
whether a save can stay incremental) is a **diff computed at save
time**, not known at edit-commit time. So the confirmation cannot
correctly live on the Delete/Reorder/Insert buttons themselves; it can
only be correct at the **Save** action — which is also exactly where
rule 2/§11.2 draw the real "can't undo this after all" line (the
redaction precedent: *"the operator needs to understand before
saving"*).

**Required core-API dependency:** pdfce-core doesn't currently expose
signature-presence detection (writer/mod.rs's own R36/W7 note says as
much). Requirement 4 needs a query the GUI calls right before Save:

```rust
enum SignatureImpact { None, PreservedIncremental, Invalidated }
fn EditSession::signature_impact_of_save(&self) -> SignatureImpact
```

This keeps the GUI's logic trivial (ask core, render one of three
states) rather than the GUI inferring "full-rewrite-ness + signature
presence" from two booleans — which would be GUI reasoning about PDF
structure, a GUI-core-separation violation. Whether an incremental save
after a page-tree change keeps a signature *valid in Acrobat's DocMDP
sense* (vs merely byte-range-preserved per §12.8.1 NOTE 1) is a real
spec question — **flagged to pdfce-spec-librarian**, do not guess.

**UI flow, gated on SignatureImpact, at the moment Save is invoked,
BEFORE the native file dialog opens** (so the operator isn't asked to
pick a destination pointlessly if they'll back out):

- `None` → straight to the existing save flow, zero added friction.
- `PreservedIncremental` → proceed, plus one non-blocking status-bar
  narrator line after success — informational, not a gate.
- `Invalidated` → **the one real, blocking confirmation in this Pass**,
  `egui::Modal` (verify availability against docs.rs for the pinned
  0.35 before implementing — if unavailable, fall back to a
  non-collapsible egui::Window added last, same layering trick as
  Properties, with explicit input-blocking):

```
Title:  ui_text::signature_invalidation_title()
Body:   ui_text::signature_invalidation_body()
[ ui_text::signature_invalidation_cancel_button() ]   [ ui_text::signature_invalidation_confirm_button() ]
```

Cancel returns with nothing written (identical semantics to today's
save-dialog cancel). Confirm proceeds to the existing native-dialog →
atomic-write → SaveOutcome flow, with one addition: a persistent,
honest status-bar line after success (`save_signature_invalidated_note`),
so the operator has a durable record, not just a modal they might
forget.

**Fallback if the SignatureImpact API isn't buildable this Pass** (ship
the honest degraded version rather than nothing — silence here would be
the sneaky option): a single generic warning fires whenever a save is
*known* to require full rewrite, without claiming to know signature
presence:

> `ui_text::full_rewrite_signature_unknown_warning()` — "This save
> requires pdfce to rewrite the whole file. pdfce cannot yet tell
> whether this document is digitally signed — if it is, rewriting will
> invalidate that signature. If you're not sure, keep a copy of the
> original file."

Same confirm/cancel button pair. The fallback shipping in Pass 3.2
beats the whole feature deferring.

---

## 5. New ui_text.rs catalog entries

All follow R2 (one complete templated sentence), R3 (no hard-sized
layout to English length), R6 (numeric/path formatting lives here).
Grouped to match the file's existing section style.

```rust
// -- Toolbar — tools dock toggle --------------------------------------
fn tools_button() -> &'static str;                    // "🧰  Tools"
fn tools_tooltip() -> &'static str;                   // names what's inside + toggle nature

// -- Thumbnail rail — selection ---------------------------------------
fn select_checkbox_tooltip(selected: bool) -> &'static str;
fn selection_bar_summary(count: usize) -> String;     // "N page(s) selected"
fn selection_clear_button() -> &'static str;          // "Clear selection"
fn selection_clear_tooltip() -> &'static str;         // names Esc
fn batch_rotate_left_tooltip(count: usize) -> String;
fn batch_rotate_right_tooltip(count: usize) -> String;
fn selection_delete_button() -> &'static str;         // "Delete"
fn selection_delete_tooltip(count: usize) -> String;  // the §3.2 wording, count-templated
fn selection_extract_button() -> &'static str;        // "Extract…"
fn selection_extract_tooltip(count: usize) -> String;
fn dangling_references_after_delete(bookmarks: usize, links: usize) -> String;
fn extract_note_unsigned_output() -> &'static str;

// -- Reorder ------------------------------------------------------------
fn thumbnail_drag_tooltip(number: usize) -> String;   // names drag AND the Alt+arrow alternative
fn move_selection_up_tooltip() -> &'static str;
fn move_selection_down_tooltip() -> &'static str;

// -- Tools dock shell -----------------------------------------------------
fn tools_dock_title() -> &'static str;                // "Tools"
fn tool_insert_pages_label() -> &'static str;
fn tool_merge_label() -> &'static str;
fn tool_split_label() -> &'static str;

// -- Insert pages ---------------------------------------------------------
fn insert_pages_choose_file_button() -> &'static str;
fn insert_pages_choose_file_tooltip() -> &'static str;
fn insert_pages_source_hint() -> &'static str;
fn insert_pages_position_label() -> &'static str;
fn insert_position_before() -> &'static str;
fn insert_position_after() -> &'static str;
fn insert_position_start() -> &'static str;
fn insert_position_end() -> &'static str;
fn insert_pages_commit_button() -> &'static str;
fn insert_pages_commit_tooltip(count: usize) -> String;
fn insert_pages_none_selected_hint() -> &'static str;

// -- Merge ------------------------------------------------------------------
fn merge_add_files_button() -> &'static str;
fn merge_add_files_tooltip() -> &'static str;
fn merge_current_document_label(path: &Path) -> String;
fn merge_remove_file_tooltip() -> &'static str;
fn merge_pending_page_count(count: usize) -> String;
fn merge_needs_two_files_hint() -> &'static str;
fn merge_commit_button() -> &'static str;
fn merge_commit_tooltip() -> &'static str;
fn suggested_merge_name(first: &Path) -> String;
fn merge_succeeded(path: &Path, files: usize, pages: usize, objects: usize) -> String;
fn merge_note_unsigned_output() -> &'static str;

// -- Split ------------------------------------------------------------------
fn split_criteria_label() -> &'static str;
fn split_every_n_label() -> &'static str;
fn split_every_n_suffix(n: usize) -> String;
fn split_after_pages_label() -> &'static str;
fn split_after_pages_hint() -> &'static str;
fn split_after_pages_invalid(entry: &str, page_count: usize) -> String;
fn split_at_selection_label() -> &'static str;
fn split_naming_pattern_label() -> &'static str;
fn split_naming_pattern_hint() -> &'static str;
fn split_preview_heading() -> &'static str;
fn split_preview_row(name: &str, first_page: usize, last_page: usize) -> String;
fn split_commit_button() -> &'static str;
fn split_commit_tooltip(parts: usize) -> String;
fn split_collision_warning(names: &str) -> String;
fn split_collision_confirm_button() -> &'static str;
fn split_collision_cancel_button() -> &'static str;
fn split_succeeded(count: usize, folder: &Path) -> String;

// -- Signature-invalidation confirmation (the one real modal) --------------
fn signature_invalidation_title() -> &'static str;
fn signature_invalidation_body() -> &'static str;
fn signature_invalidation_confirm_button() -> &'static str;   // "Save without the signature"
fn signature_invalidation_cancel_button() -> &'static str;    // "Don't save yet"
fn full_rewrite_signature_unknown_warning() -> &'static str;  // fallback, see §4
fn save_signature_invalidated_note() -> &'static str;         // post-save narrator line
```

---

## 6. Priority / deferral

**P0 (this Pass, blocking):**
1. Tools-dock scaffold + toolbar toggle (the container, regardless of
   what ships inside it first).
2. Rail multi-select model, Delete, Reorder, Rotate-batch, Extract —
   the page-scoped ops, all reusing existing patterns (rail,
   EditSession, save_dialog/write_atomic) most directly.
3. The SignatureImpact gate on Save — ship at minimum the **fallback**
   wording (`full_rewrite_signature_unknown_warning`) if the precise
   `signature_impact_of_save` API can't land this Pass. Shipping
   *nothing* here is the one place this Pass would violate rule 2/R36
   outright.
4. Rotate's missing keyboard shortcut (carried from the Pass 3.1
   review, now more urgent with batch rotate).

**P1 (ship if the Pass has room; otherwise a clean fast-follow):**
5. Insert-from-file (needs the dock scaffold anyway, natural to bundle).
6. The precise `signature_impact_of_save` core API, replacing the
   fallback warning.
7. The dangling-bookmark/link core count for Delete's disclosure, if
   not ready alongside Delete itself — ship Delete without it only as a
   last resort, flagged loudly as a rule-4 gap to close same-Pass if at
   all possible.

**P2 (larger scope, reasonable to split into a follow-up slice):**
8. Merge, Split GUI — multi-file assembly and
   folder-output-with-naming-preview are meaningfully bigger builds
   than the rail-based ops. If deferred, settle the Tools-dock
   convention and entry list now (placeholder rows are fine) so it
   isn't designed twice.
