# GUI Polish Pass — Presentability Audit of the Current Feature Set

> Authored by `pdfce-ui-specialist`, 2026-07-31, on dispatch from the
> engineer, ahead of the operator's **first hands-on use** of pdfce.
> Scope is strictly polish/cohesion/discoverability/presentability of
> what has **already shipped** (Passes 0–8.0) — no new features, no
> canvas-interaction design (that is the coming measurement/
> dimensioning beta's substrate and is out of scope here entirely).
>
> Read in full: `crates/pdfce-gui/src/main.rs` (3,875 lines),
> `crates/pdfce-gui/src/ui_text.rs`, `crates/pdfce-gui/src/viewer.rs`,
> `crates/pdfce-gui/src/raster.rs`; `docs/ARCHITECTURE.md` §12
> continuation-20/23/25 (the placement-taxonomy history and the
> settled **five-way convention**: view-state → toolbar view group;
> edit → toolbar/window; selection-scoped → rail; advanced → Tools
> dock; disclosure → status bar); `.claude/agents/pdfce-ui-specialist.md`
> (standing rules); and this agent's own memory of the Pass 3.1 review,
> the Pass 7 form-fill spec, and the Pass 8 redaction spec, to confirm
> which earlier findings are already fixed in code and which are still
> open.
>
> **Verified against the running code, not assumed:** every claim below
> was checked by reading the actual `main.rs`/`ui_text.rs` source just
> now (2026-07-31 build). Where an earlier review's finding turned out
> to already be fixed, it is listed under §0 rather than repeated as a
> new item — do not re-do that work.

---

## 0. Already fixed / already compliant — do NOT touch

Confirmed in the current source, so these are **not** on the change
list below. Listed so the engineer doesn't spend time re-verifying:

- **Atomic save write.** `write_atomic()` (main.rs) does temp-file +
  `sync_all` + rename, in the destination's own directory. The Pass 3.1
  review's ❌ finding is resolved.
- **Rotate has a keyboard shortcut.** `[` / `]`, bound in
  `collect_keyboard_actions`. The Pass 3.1 review's discoverability gap
  is resolved.
- **Undo/Redo tooltips name the specific operation** via
  `ui_text::command_label`/`undo_tooltip_for`/`redo_tooltip_for`,
  reading `EditSession`'s structured `CommandKind`. Resolved.
- **Per-field lossy marking in the Properties panel** —
  `info_field_lossy_marker()` / `info_field_lossy_tooltip()` mark the
  *specific* field, not just a panel-wide warning. Resolved.
- **Apply/Revert grey-out in the Properties panel** — both buttons are
  `add_enabled(dirty, …)`, with a distinct disabled-state tooltip on
  Apply (`properties_apply_unchanged_tooltip`). Resolved (Revert's own
  disabled tooltip parity is a small residual — see P1-5).
- **Toolbar status-summary right-alignment** —
  `ui.with_layout(egui::Layout::right_to_left(...), …)` is already in
  place. The Pass 3.1 review's "will drift right" nit is resolved.
- **Hide-vs-disable discipline is already consistent and correct.**
  Controls meaningless with no document/no selection are **hidden**
  (Save, the whole nav/zoom/edit/history toolbar cluster, the rail's
  selection action bar, the Combine tool's commit button); controls
  that are always meaningful but sometimes have nothing to do are
  **disabled with an informative tooltip** (Undo/Redo, Properties
  Apply/Revert, the Combine list's per-row move buttons). Do not
  collapse this distinction — it is a real, working pattern; extend it,
  don't rethink it.
- **Toolbar group count is at, not over, the documented cap.** Six
  separator-divided groups (file / view / navigation / zoom / edit /
  history) plus two ungrouped utility controls (Copy-text menu, Tools
  toggle) — exactly the "6 groups + toggles" shape the Pass 3.1 review
  flagged as a future risk. The Tools dock is doing its job as the
  overflow valve. No group needs to be cut or merged; §P1-4 below is a
  pure visual-consistency touch, not a restructuring.
- **The minimal Markup/Text menus already disclose their own
  limitation.** `markup_menu_hint()` and (mostly — see P1-3)
  `text_menu_hint()` already say the shape/text lands at page-centre
  and that on-canvas placement is coming. This is the right pattern;
  the fixes below are wording/consistency refinements, not a missing
  disclosure.
- **Properties floating window is the correct placement**, per the
  settled five-way taxonomy: "edit → toolbar/window" explicitly
  includes windows. This is not the unresolved question the Pass 3.1
  review flagged before the taxonomy existed — that question is
  answered now. Leave it as a floating `egui::Window`.
- **Redaction's one non-negotiable GUI item shipped correctly.** The
  persistent, document-census-driven "unapplied /Redact marks" status
  line (`redaction_marks_pending`) is present, colour-plus-glyph-plus-text
  (rule 6 compliant), and computed from the document every frame rather
  than a session counter. This is the Pass 8 spec's P0 and it is in.

---

## P0 — must-have before the operator's first session

### P0-1. Reset stale narration state when a new document is opened

**What's wrong:** `PdfceApp::open_path` (main.rs, ~line 974) only
clears `self.save_result`. It does **not** clear `self.edit_note`,
`self.copy_result`, `self.copy_detail_expanded`, `self.pending_text_kind`,
or `self.text_input`. An operator who deletes a page or copies text
from document A, then opens document B in the same session (a very
likely first-session action — "open one, glance at it, open another"),
sees document A's leftover narration line (e.g. *"2 page(s) deleted;
14 object(s) removed…"* or a Copy-text result) sitting in the status
bar as if it just happened to document B. This is not merely untidy —
it directly contradicts the "status bar is the narrator" standing rule
and rule 4 (fuzzy, never sneaky): stale narration about the wrong
document is actively misleading, not just missing.

Separately: if the Pass 6.2 text-entry popup happens to be open (it is
**not** part of the blocking pending-gate, so `Action::Open` is not
suppressed while it's showing) and the operator clicks Open anyway, the
popup stays open afterward and "Add to page" would author onto the
*new* document using text typed against the old one.

(Checked and **not** a bug: `self.pending_save` / `self.pending_copy`
cannot be stale here — the `apply()` gate blocks `Action::Open` itself
while either is set, so a new document can never load while one of
those confirmations is outstanding. Don't add a reset for those two;
there is nothing to reset.)

**Fix:** in `open_path`, alongside the existing
`self.save_result = None;`, add:

```rust
self.edit_note = None;
self.copy_result = None;
self.copy_detail_expanded = false;
self.pending_text_kind = None;
self.text_input.clear();
```

**Where:** `PdfceApp::open_path`, main.rs (~line 974–995).

**ui_text.rs entries needed:** none — this is pure state hygiene.

---

### P0-2. Reseed the Properties panel when a new document loads while it's open

**What's wrong:** `properties_open` (a `PdfceApp` field, outlives any
one document) is not reset or reseeded in `open_path`. If the operator
has the Properties panel open and then opens a different file, the new
`OpenDoc` is constructed with `properties_draft: Vec::new()`
(`OpenDoc::new`) — so the still-showing Properties window renders an
**empty grid**: no Title/Author/Subject/Keywords rows at all, until the
operator closes and reopens the panel (which is the only place
`seed_properties_draft()` is currently called). This reads as a broken
panel on exactly the "open a second file" workflow a first session is
likely to exercise.

Checked and confirmed **not** a data-integrity risk: `apply_properties`
iterates the (now-empty) draft, so Apply/Revert are correctly harmless
no-ops against an empty draft — this is a presentability bug, not a
silent-overwrite risk.

**Fix:** in `open_path`, immediately after `self.status` is assigned,
seed the draft if the panel happens to be open:

```rust
if self.properties_open
    && let Status::Open(doc) = &mut self.status
{
    doc.seed_properties_draft();
}
```

**Where:** `PdfceApp::open_path`, main.rs, right after the existing
`self.status = match Document::load(&path) { … };` assignment.

**ui_text.rs entries needed:** none.

---

### P0-3. Window title never reflects the open file

**What's wrong:** `main()` sets `.with_title("pdfce")` once, at
`ViewportBuilder` construction (main.rs ~line 267), and nothing in
`PdfceApp::ui`/`apply` ever updates it. Every other desktop PDF viewer
puts the open file's name in the title bar/taskbar — an operator with
several documents open in several windows, or alt-tabbing back to
pdfce after a distraction, has no way to tell which file this window
is short of looking at the toolbar's status-summary text. This is the
single most conventional piece of "does this look like a finished
desktop app" signal a first-time user will notice, and it currently
never fires.

**Fix:** compute the wanted title each frame from `self.status` and
push it via `egui::ViewportCommand::Title` **only when it changes**
(avoid spamming the platform layer every frame):

```rust
// New PdfceApp field:
last_window_title: String,   // default: ui_text::window_title_idle().to_owned()

// Once per frame, near the top of `eframe::App::ui`, after `actions`
// are applied (so `status_open`'s `modified` flag reflects this
// frame's edits) — or equally correct computed at the very end:
let wanted_title = match &self.status {
    Status::Open(doc) => ui_text::window_title_open(
        &doc.path,
        doc.session.is_modified(),
    ),
    _ => ui_text::window_title_idle().to_owned(),
};
if wanted_title != self.last_window_title {
    ctx.send_viewport_cmd(egui::ViewportCommand::Title(wanted_title.clone()));
    self.last_window_title = wanted_title;
}
```

Deliberately **not** titled for `Status::Failed`/`Status::Unsupported`
— only a document that actually opened earns a place in the window
chrome, matching the convention every other editor uses (a failed open
does not rename the window after the file that failed).

**Where:** `main()` for the initial title (unchanged), `PdfceApp` gains
one `String` field, `eframe::App::ui` gains the block above (near the
end, after `self.apply(...)` runs so `is_modified()` is current for
this frame).

**ui_text.rs entries needed:**

```rust
/// Window/taskbar title while no document is open, or after a failed
/// or unsupported open (a failed open does not rename the window).
pub fn window_title_idle() -> &'static str {
    "pdfce"
}

/// Window/taskbar title once a document is open. `modified` marks
/// unsaved edits with a plain leading asterisk — the same convention
/// most editors use, so it needs no legend.
pub fn window_title_open(path: &Path, modified: bool) -> String {
    if modified {
        format!("{}* — pdfce", file_name(path))
    } else {
        format!("{} — pdfce", file_name(path))
    }
}
```

---

### P0-4. Cap the status bar's height so stacked disclosures can't crowd the canvas

**What's wrong:** `egui::Panel::bottom("status")` (main.rs ~line 2312)
has no height constraint, and `status_bar()`'s body can legitimately
emit a *lot* of simultaneous lines by design (every one of them is a
standing-rule-mandated disclosure that must never be suppressed):
`edit_note`, `save_result` (+ a promoted-objects sub-line),
`copy_result_bar` (result line + collapsing header + up to 3 detail
lines when expanded), the redaction-pending warning, the content
diagnostics header (+ up to ~9 possible detail lines when expanded),
and `annotation_status` (up to 4 more lines). None of these may be
hidden — that is the whole point of rule 4 and R20/R43/R50/R51/R52 —
but a page that happens to trip several of them at once, right after a
delete or a copy, can genuinely push the status area to consume a large
fraction of the window, squeezing the canvas. This is exactly the kind
of thing that reads as "unfinished" on first contact even though every
individual line is correct and required.

**Fix:** wrap the *existing* body of `status_bar()` in a height-capped
scroll area — nothing is hidden, it is simply scrollable once it grows
past a sane cap:

```rust
const STATUS_BAR_MAX_HEIGHT: f32 = 220.0; // ~ 8 lines before it scrolls

fn status_bar(&mut self, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .id_salt("status-bar-scroll")
        .max_height(STATUS_BAR_MAX_HEIGHT)
        .show(ui, |ui| {
            // ...the existing body, unchanged, just re-indented one level...
        });
}
```

**Where:** `PdfceApp::status_bar`, main.rs (~line 2939). Pure
re-indentation of the existing body into the closure — no logic
changes, no line removed, no disclosure suppressed.

**ui_text.rs entries needed:** none.

---

### P0-5. A real empty state: identity, an inline Open affordance, and drop-to-open

**What's wrong:** before a file is opened, the canvas shows one
centred sentence (`canvas_idle_hint`) and nothing else — no app-name
anchor, no way to open a file except the toolbar button up top (which,
on a maximized 1100×800 window, is a long reach from the visually
"empty" centre the operator is looking at), and no drag-and-drop
support at all (confirmed: no `dropped_files`/`hovered_files` handling
anywhere in `main.rs`). This is the operator's literal first impression
of the app and it currently reads as inert rather than "ready."

**Fix, three parts, all scoped to avoid new confirmation machinery:**

1. **App-name heading**, above the existing hint text, in the same
   centred block. Plain text, no slogan, no "open source"/release
   claim (rule 8 / ~~license still undecided~~ **license is MIT since
   2026-08-01; the live reason is the still-ungranted PUBLISH
   authorization, not the licence** — see the correction note at the end
   of this document) — just the name:

   ```rust
   ui.centered_and_justified(|ui| {
       ui.vertical_centered(|ui| {
           ui.heading(ui_text::empty_state_heading());
           ui.add_space(8.0);
           ui.label(ui_text::canvas_idle_hint());
           ui.add_space(12.0);
           if ui.button(ui_text::open_button()).on_hover_text(ui_text::open_tooltip()).clicked() {
               actions.push(Action::Open);
           }
           ui.add_space(6.0);
           ui.label(ui_text::canvas_idle_drop_hint());
       });
   });
   ```

   (Reuses `open_button()`/`open_tooltip()`/`Action::Open` verbatim —
   no duplicate affordance, just a second, more discoverable entry
   point to the same action.)

2. **Drop-to-open**, gated to states where nothing is at risk. Read
   dropped files once per frame near the top of `eframe::App::ui`:

   ```rust
   let dropped_pdf = ctx.input(|i| {
       i.raw.dropped_files.iter()
           .find_map(|f| f.path.clone())
           .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf")))
   });
   if let Some(path) = dropped_pdf
       && matches!(self.status, Status::Idle | Status::Failed { .. } | Status::Unsupported { .. })
   {
       self.open_path(path);
   }
   ```

   **Deliberately restricted to "nothing meaningfully open yet."**
   Accepting a drop while a document with unsaved edits is open would
   silently discard those edits with no confirmation — a real hazard
   this Pass has no infrastructure for and should not improvise one for.
   Extending drop-to-*replace* an open document to reuse the existing
   `pending_save`-style confirmation convention is a reasonable P2/
   backlog follow-up, not part of this fix.

3. **A visible hint that dropping works**, satisfied by
   `canvas_idle_drop_hint()` above. A hover-highlight on the canvas
   while a file is being dragged over it (`ctx.input(|i| i.raw.hovered_files)`
   is non-empty) is a nice-to-have paint detail, not required for
   correctness — leave it if time allows, don't block on it.

**Where:** `PdfceApp::canvas`, `Status::Idle` branch (main.rs ~line
3470–3473); `eframe::App::ui` (main.rs ~line 2302) for the drop-read.
Verify the exact `egui::InputState`/`DroppedFile` field names against
`D:\dev\rag\egui` or the pinned egui docs before implementing — this
spec sketches the well-known stable shape of that API but the engineer
should confirm against the pinned version rather than trust this
document's Rust verbatim.

**ui_text.rs entries needed:**

```rust
/// Heading shown above the empty-canvas hint, before any file is open.
/// Plain app name only — no tagline, no "open source"/release claim
/// (the project's licence is still undecided).
/// [⚠ 2026-08-07: that parenthetical is STALE — the licence is MIT
///  (2026-08-01). The rule still holds; its reason is now the
///  still-ungranted PUBLISH authorization. This spec's text is the
///  UPSTREAM SOURCE of the identical stale comment now in shipped code
///  at crates/pdfce-gui/src/ui_text.rs:1142, which is outside this
///  sweep's docs/-only scope and is flagged as owed.]
pub fn empty_state_heading() -> &'static str {
    "pdfce"
}

/// Second line of the empty-canvas hint, naming the drop affordance.
/// A separate entry from `canvas_idle_hint` (R2: one entry, one
/// complete thought) rather than folding a third sentence into it.
pub fn canvas_idle_drop_hint() -> &'static str {
    "Or drop a PDF file here to open it."
}
```

---

### P0-6. Annotation-visibility toggle's click target is inconsistent with every other icon-only control

**What's wrong:** every other icon-only toolbar control (rail toggle,
prev/next page, zoom in/out, rotate left/right, undo/redo) is wrapped
in `ui.add_sized(ICON_BUTTON_SIZE, egui::Button::new(...))`, giving it
the documented minimum 28×24 click target regardless of glyph width.
The annotation-visibility toggle (main.rs ~line 2493) is the one
exception: `ui.selectable_label(visible, ui_text::annotations_toggle_button())`
with no `add_sized` wrapper — it sizes to its single-emoji content,
which is narrower than every neighbouring control and is the one
icon-only button that does not honour the app's own documented
click-target minimum. This is a small but real, easily-missed
consistency/accessibility gap on a control an operator will use often
(it is the one control whose effect can be visually silent on a
lightly-annotated page, so the operator specifically needs to be able
to hit it reliably).

**Fix:**

```rust
if ui
    .add_sized(
        ICON_BUTTON_SIZE,
        egui::SelectableLabel::new(visible, ui_text::annotations_toggle_button()),
    )
    .on_hover_text(tooltip)
    .clicked()
{
    actions.push(Action::ToggleAnnotations);
}
```

(`egui::SelectableLabel` implements `Widget`, so it drops into
`add_sized` exactly like `egui::Button` does elsewhere in this file —
no new widget type, no new dependency.)

**Where:** `PdfceApp::toolbar`, main.rs (~line 2486–2500).

**ui_text.rs entries needed:** none.

---

## P1 — should-fix in the same Pass if time allows

### P1-1. Selected/active state relies on colour alone for four toolbar toggles

**What's wrong:** `ui.selectable_label(...)` is used for Fit Page, Fit
Width, the Properties toggle, and (after P0-6) the annotation
visibility toggle. In every case the *only* signal that the control is
currently active is `visuals.selection.bg_fill` — a background colour
change. There is no glyph, weight, or text change accompanying it.
This is the one place in the current toolbar that does not follow
rule 6 (colour is never the sole signal) as consistently as the rest
of the app does (every `colored_label` warning elsewhere pairs colour
with a ⚠/✖/✔ glyph and full sentence).

**Fix:** add a non-colour cue alongside the background fill. Cheapest
option, no new glyph asset, no layout-shifting prefix character:

```rust
fn toggle_label(selected: bool, text: &str) -> egui::RichText {
    let rt = egui::RichText::new(text);
    if selected { rt.strong() } else { rt }
}
// ... at each of the four call sites:
ui.selectable_label(selected, toggle_label(selected, label))
```

If the engineer judges bold-weight too subtle in practice, a leading
checkmark (`"✔ Fit page"` vs `"Fit page"`) is the more emphatic
alternative — either satisfies rule 6; pick one and apply it
consistently to all four controls, not a mix.

**Where:** `PdfceApp::toolbar` — Fit Page (~2563), Fit Width (~2570),
Properties toggle (~2614), Annotations toggle (after P0-6 is applied).

**ui_text.rs entries needed:** none (this is a `RichText` styling
change at the call site, not new copy).

---

### P1-2. No single place to see every keyboard shortcut

**What's wrong:** every shortcut is documented, but only inside its own
control's tooltip — there is no single reference, so "what can I do
from the keyboard" requires hovering a dozen separate buttons. The task
brief explicitly invites considering this; the current design has
accreted eleven-plus chords (`PageUp`/`PageDown`/`Home`/`End`,
`Ctrl+O`, `Ctrl+Plus`/`Minus`/`0`, `Ctrl+S`, `Ctrl+Z`, `Ctrl+Y`/
`Ctrl+Shift+Z`, `[`/`]`, `Alt+↑`/`Alt+↓`, `Esc`, `Delete`/`Backspace`,
`Ctrl+Shift+C`) with no map.

**Fix:** a small, modeless `egui::Window` (same convention as the
Properties panel — not a blocking modal, since reading a reference
list while looking at the document is exactly the use case), opened
from a new toolbar entry in the existing **ungrouped-utility** slot
(beside Copy-text/Tools, per the settled taxonomy — this is a
disclosure surface, not an edit or a document-scoped tool). **Reuse
each control's existing tooltip string as the row text** rather than
building a second, parallel copy of the same facts that could drift
from the real bindings:

```rust
fn shortcuts_window(&mut self, ctx: &egui::Context) {
    if !self.shortcuts_open { return; }
    let mut open = true;
    egui::Window::new(ui_text::shortcuts_window_title())
        .open(&mut open)
        .resizable(true)
        .default_width(420.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for line in [
                    ui_text::open_tooltip(),
                    ui_text::prev_page_tooltip(),
                    ui_text::next_page_tooltip(),
                    ui_text::zoom_in_tooltip(),
                    ui_text::zoom_out_tooltip(),
                    ui_text::zoom_100_tooltip(),
                    ui_text::save_tooltip(),
                    // ...one line per bound chord, pulling the SAME
                    // string already shown as that control's tooltip...
                ] {
                    ui.label(line);
                    ui.separator();
                }
            });
        });
    if !open { self.shortcuts_open = false; }
}
```

New toolbar entry (ungrouped utility, beside Tools):

```rust
if ui
    .add_sized(ICON_BUTTON_SIZE, egui::Button::new(ui_text::shortcuts_button()))
    .on_hover_text(ui_text::shortcuts_tooltip())
    .clicked()
{
    self.shortcuts_open = !self.shortcuts_open;
}
```

**Where:** new `PdfceApp` field `shortcuts_open: bool` (default
`false`, same progressive-disclosure default as `properties_open`);
new method beside `properties_window`; new toolbar button beside the
Tools toggle in `PdfceApp::toolbar`; called once from `eframe::App::ui`
alongside `self.properties_window(...)`.

**ui_text.rs entries needed (chrome only — no duplicated shortcut
prose):**

```rust
pub fn shortcuts_button() -> &'static str { "⌨" }
pub fn shortcuts_tooltip() -> &'static str { "Show every keyboard shortcut" }
pub fn shortcuts_window_title() -> &'static str { "Keyboard shortcuts" }
```

---

### P1-3. Text-menu wording, silent per-type colour inconsistency, and same-spot stacking

Three related findings in the Pass 6.1/6.2 minimal authoring menus:

**(a) Ambiguous wording.** `text_menu_hint()` currently reads *"Type
the text, then choose where it goes."* — but there is no placement
choice; the operator chooses which **kind** (text box / sticky note /
stamp), and it is always centred automatically. "Choose where it goes"
reads as if a placement control exists. Reword:

```rust
pub fn text_menu_hint() -> &'static str {
    "Type the text, then pick a kind below. It is placed at the centre \
of the current page; click-to-place editing on the canvas is coming."
}
```

**(b) Silent per-subtype colour inconsistency.** The Markup menu's
colour picker (`self.markup_color`) is read by `add_pending_text` for
**FreeText only** — Sticky notes and Stamps use hard-coded colours
(yellow, red) regardless of what the operator picked. This is
plausibly the *correct* behaviour (a sticky note that isn't
conventionally yellow, or a Draft stamp that isn't conventionally red,
would be less recognisable, not more useful) — but nothing in the UI
says so, and the colour picker itself only appears in the *Markup*
menu, so an operator who authors a FreeText box from the *Text* menu
without ever opening Markup gets an unexplained dark-red default.
Fix: surface the same colour control in the Text menu too, with an
honest one-line note about the exception:

```rust
ui.menu_button(ui_text::text_menu_button(), |ui| {
    ui.label(ui_text::text_menu_hint());
    ui.horizontal(|ui| {
        ui.label(ui_text::markup_color_label());   // reuse, don't duplicate
        ui.color_edit_button_srgba(&mut self.markup_color);
    });
    ui.label(ui_text::text_menu_color_note());
    ui.separator();
    // ...existing three menu items...
});
```

```rust
/// Note under the Text menu's colour picker, disclosing the one
/// exception: Sticky notes and Draft stamps intentionally ignore it.
pub fn text_menu_color_note() -> &'static str {
    "Applies to the text box only — sticky notes and stamps use their \
own standard colours."
}
```

**(c) Repeated adds land in the exact same spot.** `add_markup_shape`
and `add_pending_text` compute the centred rect deterministically from
the page's `MediaBox` every call, with no per-invocation offset. An
operator who clicks "Rectangle" twice authors two squares exactly on
top of each other — indistinguishable on screen, since the canvas has
no drag-to-reposition yet. Given how minimal this affordance already
is, this is a real "did that even work?" moment on a first session.
**Fix (small, contained, no canvas interaction required):** offset each
successive same-page addition by a small fixed step, derived from a
per-page counter (e.g. how many markup/text annotations the current
`EditSession` already holds for this page — data already available
without new state):

```rust
// e.g. a small jitter based on the page's current annotation count,
// applied to cx/cy before building the rect:
let existing = doc.session.annotation_count_for_page(page_index); // or equivalent
let jitter = (existing as f64) * 12.0; // points; small, deterministic, visible
let cx = f64::midpoint(mb.llx, mb.urx) + jitter;
let cy = f64::midpoint(mb.lly, mb.ury) - jitter;
```

If `EditSession` does not currently expose a cheap per-page annotation
count, a session-local `PdfceApp` counter reset per page-navigation is
an acceptable, purely-cosmetic substitute — this does not need to be
exact, only visibly different per click.

**Where:** `ui_text::text_menu_hint` (rewrite); `PdfceApp::toolbar`'s
Text-menu block (~2659); `PdfceApp::add_markup_shape` (~1182) and
`PdfceApp::add_pending_text` (~1253).

**ui_text.rs entries needed:** `text_menu_hint()` rewritten (above);
new `text_menu_color_note()` (above).

---

### P1-4. Visual boundary of the ungrouped-utility cluster

**What's wrong:** small, cosmetic. The Copy-text menu and Tools toggle
(the two "ungrouped utility" controls the Pass 3.2/Pass 4 doc comments
establish as a deliberate category, distinct from the six separator-
bounded groups) sit flush against each other with no space between
them, and — because Copy-text is only shown when a document is open
while Tools is always shown — the visual gap before this cluster
differs depending on whether a document is open. Not a functional bug,
just a small inconsistency in an otherwise carefully-organised toolbar.

**Fix:** `ui.add_space(6.0)` immediately before the Copy-text/Tools
pair, unconditionally (i.e., before the `if self.status_is_open() { … }`
block that may or may not show Copy-text), so the utility cluster
always starts from a consistent visual offset regardless of document
state. Do not add a full `ui.separator()` here — that would visually
promote the pair to a seventh "group," which is exactly what the Pass
3.2 doc comments say not to do.

**Where:** `PdfceApp::toolbar`, main.rs, immediately before the
Copy-text `if self.status_is_open() { … }` block (~2716).

**ui_text.rs entries needed:** none.

---

### P1-5. Properties "Revert" button's disabled tooltip doesn't say why it's disabled

**What's wrong:** Apply's disabled-state tooltip correctly explains
*why* it's greyed out (`properties_apply_unchanged_tooltip`: "Nothing
to apply — these values already match the document."). Revert, right
next to it, always shows the same tooltip (`properties_revert_tooltip`)
whether enabled or disabled, so the disabled Revert offers no
equivalent "there's nothing to revert" confirmation. Small, but the two
buttons sit side by side and an inconsistent level of explanation
between neighbours reads as unfinished.

**Fix:**

```rust
.on_hover_text(if dirty {
    ui_text::properties_revert_tooltip()
} else {
    ui_text::properties_revert_unchanged_tooltip()
})
```

**Where:** `PdfceApp::properties_window`, main.rs (~2889–2895).

**ui_text.rs entries needed:**

```rust
/// Tooltip on the Revert button when the draft already matches the
/// document — the disabled-state counterpart to
/// `properties_apply_unchanged_tooltip`.
pub fn properties_revert_unchanged_tooltip() -> &'static str {
    "Nothing to revert — these values already match the document."
}
```

---

### P1-6. Icon-only glyph buttons and the accessible-name gap

**What's wrong:** roughly a dozen toolbar controls (`◀ ▶ − + ↺ ↻ ↶ ↷
▲ ▼ ▤ 🗩`) carry their entire visible label as a single Unicode glyph,
with the meaningful name living only in `.on_hover_text(...)`. The
module docs already state, honestly, that "screen-reader support …
has not been tested" — this finding is the concrete mechanism behind
that honest gap, not a claim that it's untested-but-fine. In most
GUI-accessibility integrations (egui's `accesskit` included, to the
best of this specialist's non-code-writing knowledge — **verify
against `D:\dev\rag\egui` or the pinned egui/accesskit docs before
relying on this**), the accessible *name* a screen reader announces is
derived from the widget's own visible text/label, not from its hover
tooltip. If that holds for the pinned egui version, every glyph-only
button here announces as a raw character or its Unicode name to a
screen reader, not as "Rotate page clockwise."

**Fix (contingent on the verification above):** if egui exposes a way
to set an accessible name independent of the visible glyph (check for
something like a `Response`/`Ui` accesskit-label hook, or a
`Button::new(glyph).accesskit_name(text)`-shaped API in the pinned
version), apply it to every glyph-only control above, sourcing the name
from the *same* `ui_text` tooltip string already in hand (no new
catalog entries — reuse `rotate_left_tooltip()` etc. as the accessible
name too). If no such mechanism exists in the pinned egui, **do not
invent a workaround** — instead, add one sentence to this crate's
existing accessibility doc comment (main.rs, "Accessibility status —
stated honestly, not implied") naming this specific gap explicitly
("icon-only controls' accessible names are currently their raw glyph,
not their tooltip text — a tracked upstream limitation, not an
oversight"), so it stays an honestly-disclosed gap rather than a silent
one. Either outcome is an acceptable resolution to this item; producing
neither is not.

**Where:** verification step first (engineer + `D:\dev\rag\egui`); then
either a mechanical per-control change across `PdfceApp::toolbar`/
`thumbnail_rail`, or a one-sentence doc-comment addition in `main.rs`'s
module docs.

**ui_text.rs entries needed:** none (reuses existing tooltip strings,
or is a code-comment-only change).

---

## P2 — nice-to-have, defer without concern

- **P2-1. Recent-files list in the empty state.** The task brief
  mentions this as a "maybe." It genuinely needs real preference
  persistence (a settings file with a format and a migration story) —
  exactly the thing `rail_expanded`'s own doc comment already declines
  to grow ad hoc. Do not build a one-off `Vec<PathBuf>` in `PdfceApp`
  for this; it is the same anti-pattern the existing code comments
  warn against. File as backlog, gated on a real settings-persistence
  design, not on this polish Pass.
- **P2-2. Window/taskbar icon asset.** No `.with_icon(...)` is set
  anywhere; the app currently runs under a generic default icon. This
  needs actual artwork (an `.ico`/`.png` asset), which is out of scope
  for a text-only session — flag for follow-up once an icon exists,
  don't block this Pass on it.
- **P2-3. Light/dark theme verification.** No explicit `Visuals`/theme
  code exists anywhere in `main.rs` — the app relies entirely on
  eframe's default OS-theme behaviour. Recommend a quick manual check
  on a light-mode Windows machine before calling the app "finished
  looking" — if eframe already follows the OS theme correctly (likely,
  but unverified here), no code change is needed; if it doesn't, that's
  a real P1/P0-worthy finding for a *future* review, not invented here
  without evidence.
- **P2-4. Markup colour-picker tooltip.** `color_edit_button_srgba` in
  the Markup menu has no `.on_hover_text(...)` of its own. Low priority
  — the adjacent "Colour:" label plus the menu's own hint already carry
  the context — but a one-line tooltip ("Changing this is not an edit —
  only adding a shape below is.") would reinforce fuzzy-never-sneaky
  for a control that otherwise looks like an immediate action.
- **P2-5. A visual/spacing QA pass once built.** Everything in this
  document was found by reading source, not by running the app —
  padding, alignment, and exact spacing can't be verified from code
  alone. Recommend one screenshot-driven pass (light + dark, a few
  window sizes) after the above lands, specifically looking for
  anything that still reads as "accreted" rather than "designed."

---

## Out of scope for this Pass — flag to the operator/engineer separately

These are real, standing-rule-relevant gaps this audit surfaced while
reading the code, but they are **not** UI polish — they are data-safety
or persistence features that need actual engineering, not a widget
change, and including them in a "polish" Pass would misrepresent their
size:

- **No autosave/crash-recovery scratch file exists anywhere in
  `pdfce-gui`.** Standing rule 5 ("crash-safe autosave, always-on") is
  currently satisfied only for the *write* half (P0's atomic-write fix
  is real and already shipped); the *autosave* half — a periodic
  recovery snapshot of in-progress edits — does not exist. This was
  flagged in the Pass 3.1 review and remains open. It must land before
  any true in-place Save ships (in-place Save without a recovery file
  would be strictly more dangerous than today's copy-only model), but
  it is a feature, not a polish item, and does not belong in this Pass.
- **True in-place Save** (rebinding `Ctrl+S` away from "Save a copy…")
  is explicitly gated on the autosave item above per the Pass 3.1
  review's own ordering — still correctly deferred, not a regression to
  flag now.

---

## Summary for the engineer

Six P0 items, all small and independently landable:
correctness/honesty fixes for cross-document stale state (P0-1, P0-2),
one structural presentability fix each for the window chrome (P0-3),
the status bar's growth (P0-4), the empty state (P0-5), and one
click-target consistency fix (P0-6). None of them touch
`pdfce-core`/`pdfce-render`, none of them add a new save mode, and none
of them require the canvas-interaction infrastructure this Pass is
explicitly not designing. P1 items are genuine but lower-stakes; take
as many as time allows in priority order. P2 and the two out-of-scope
items are recorded so they aren't lost, not because they belong in this
Pass.

---

## ⚠ CORRECTION FOOTER — added 2026-08-07 by `pdfce-librarian` (licence sweep)

**This spec was authored 2026-07-31 and is not otherwise re-opened.** Two
places in it state that pdfce's own licence is *"still undecided"* (§P0-5's
app-name-heading item and the `ui_text.rs` doc comment it specifies). **That
was true when written and became false on 2026-08-01, when the operator
chose MIT** (`LEGAL.md` §1).

**Both are marked in place rather than rewritten** — the spec is a dated
design artifact, and its reasoning at the time is part of the record.

**The design decision itself is UNCHANGED and still correct: no "open
source" or release claim in the empty state.** Only its *reason* moves.
The gate was never really the licence — it is **project rule 8's publish
authorization**, which is separate from the licence, was never granted, and
is still not granted today. A future reader who sees "MIT" and concludes the
tagline restriction lapsed would be wrong.

**Downstream consequence, flagged not fixed:** this document's Rust snippet
is the **upstream source** of the doc comment now shipping at
`crates/pdfce-gui/src/ui_text.rs:1142`, which carries the stale
parenthetical verbatim. That file is outside this sweep's `docs/`-only
scope. Full swept set, method, and the other out-of-`docs/` residue:
`LEGAL.md` §7's **2026-08-07 second entry**.
