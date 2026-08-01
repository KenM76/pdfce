# Pass 16.2 UI Spec — "Add Page Text" on the Pass 12.0 Canvas (decision 016 / FF-D, final slice)

> Authored by `pdfce-ui-specialist`, on dispatch from the engineer. Implements
> the GUI slice of decision 016 (`docs/decisions/016-ffd-add-new-page-text.md`)
> — the ROADMAP's "16.2 — Add-text UI on the Pass 12.0 canvas." Engineer
> implements this verbatim; deviations are named, not silent (the Pass
> 3.2/6.1/7/8/12.0/14.3/15.2 convention).
>
> Read before implementing: decision 016 in full (§3.1 real-page-content-
> never-FreeText, R78; §3.2 point-then-boxed; §3.3 bundled-Std14 default, R79;
> §3.6 tagged-untagged, R73); `docs/ui_specs/pass-14.3-text-edit-ui.md` in
> full (the preview/Accept-Reject/disclosure conventions this Pass reuses
> verbatim, and the toolbar-taxonomy precedent this Pass extends);
> `docs/ui_specs/pass-15.2-reflow-ui.md` in full (the sub-mode-vs-new-tool
> reasoning this spec explicitly contrasts itself against, §0.1);
> `crates/pdfce-core/src/text_edit/addtext.rs` in full — **this Pass is
> written against the ACTUAL SHIPPED Pass 16.0 API, not an announced
> contract** (unlike 14.3's `EditSession::edit_text`/15.2's
> `EditSession::reflow_block`, both of which had to ask for a core addition —
> 16.0 shipped the free function AND the session-integrated sibling from day
> one, confirmed by reading `crates/pdfce-core/src/edit.rs` lines 1680-1780
> directly); `crates/pdfce-core/src/fontdata/mod.rs` (`Std14`,
> `std14_base_font_name`); `crates/pdfce-cli/src/main.rs`'s `cmd_add_text`
> (the CLI's own `font_env.classify_nonembedded(base_font_name)` idiom this
> spec's property bar reuses verbatim); `crates/pdfce-gui/src/{main.rs,
> canvas.rs,ui_text.rs}` in full — specifically the shipped `CanvasTool`,
> `TextEditState`/`PendingEdit`, `run_text_edit_tool`'s Phase A/B/C structure,
> `current_gesture_interrupt`/`discard_active_gesture`/`resolve_escape`
> wiring, the `"Markup ▾"`/`"Text ▾"`/`"Edit Text"` toolbar cluster (lines
> ~3567-3659), and the Font-folders Tools-dock panel (`font_folders_tool`,
> `font_env`, `rebuild_font_env`).

---

## 0. What this Pass changes, and what it does NOT need to ask for

### 0.1 Add Page Text is a SECOND `CanvasTool` variant, not a `TextEdit` sub-mode — the load-bearing call, decided the opposite way from 15.2's reflow

`CanvasTool` has exactly one inhabitant today — `TextEdit` (Pass 14.3), with
Reflow living inside it as a sub-mode (Pass 15.2). **This Pass adds a genuine
SECOND variant:**

```rust
pub enum CanvasTool {
    TextEdit,
    /// Author brand-new page content at an operator-chosen point or box
    /// (Pass 16.0-16.2 / decision 016 / FF-D): click→point origin, drag→wrap
    /// box (16.1), type→live preview, Accept→one `CommandKind::AddText`,
    /// Reject/Esc→nothing added. NEVER a Pass-6.2 FreeText annotation (R78).
    AddText,
}
```

**Why NOT a `TextEdit` sub-mode, when 15.2 deliberately made reflow one —
three concrete, structural reasons this Pass's target is categorically
different from reflow's:**

1. **Reflow targets EXISTING recognized structure; Add-Text targets
   ARBITRARY, unstructured page-space coordinates.** 15.2 §1's whole
   argument for keeping reflow inside `TextEdit` was that it operates on
   "the block containing the current caret" — a `Block` reached through
   `EditableTextModel`, the SAME model/caret/navigation machinery `TextEdit`
   already owns. Add-Text has no such dependency: placing a new origin needs
   no `EditableTextModel::hit_test`, no caret, no existing-glyph navigation
   at all — only a canvas-space point or rectangle converted to PDF space.
   Forcing it through `TextEditState` would import a whole apparatus
   (model rebuild, caret, selection) this operation never touches.
2. **A plain click's MEANING cannot be safely overloaded inside the SAME
   active tool.** 15.2 §1 explicitly rejected "click a block in the overlay
   to enter reflow" specifically because it would make a plain click mean
   two different things depending on an orthogonal-looking toggle
   (`show_block_overlay`) — the exact "silent mode-shift" fuzzy-never-sneaky
   forbids. If Add-Text were a `TextEdit` sub-mode, EVERY click while
   `TextEdit` is active would need a NEW disambiguation rule ("did this
   click hit existing text — edit it — or miss — start placing new text?"),
   which is the identical hazard one level up: an operator who has spent
   months learning "a miss just clears my caret" (14.3 §3.2, the shipped,
   safe no-op) would have that meaning silently repurposed into "and also
   sometimes now creates new page content," the first time they misclick
   intending only to deselect. **A SEPARATE, deliberately-entered tool**
   avoids this outright: `TextEdit`'s click semantics are completely
   unchanged by this Pass; a click only ever places new text while the
   operator has explicitly switched into `AddText` via its own toolbar
   toggle — exactly the same kind of deliberate, discoverable mode-switch
   that already governs entering `TextEdit` itself in the first place, not
   a new or riskier pattern.
3. **Placement is not restricted to "inside existing text."** Add-Text must
   work identically whether the operator clicks on a blank margin, over a
   diagram, or directly on top of existing body text (e.g. adding a "VOID"
   label across a paragraph) — there is no "hit vs miss" branch to make
   sense of the way `TextEdit`'s click handler has one. A dedicated tool
   with its own click handler (§3) needs no such branch either; it always
   means the same thing.

`active_tool: Option<CanvasTool>` is already a single value (Pass 12.0), so
`TextEdit` and `AddText` are automatically mutually exclusive by
construction — selecting one clears the other, with zero new substrate
plumbing (`canvas_suppresses_pan`, `resolve_escape`,
`current_gesture_interrupt`'s dispatch are already generic over
`tool_active: bool`, not over which `CanvasTool` variant — confirmed by
reading `canvas.rs` and `main.rs`'s `current_gesture_interrupt`/`ui()`
call sites directly; they take a plain bool, computed as
`doc.active_tool.is_some()`, so a second variant costs nothing at the
dispatch layer).

### 0.2 Good news: the P0 commit-path core gap 14.3/15.2 each had to ask for is ALREADY CLOSED

Both 14.3 (`EditSession::edit_text`/`format_text`) and 15.2
(`EditSession::reflow_block`) had to name, as their own spec's #1 priority, a
missing session-integrated sibling of a free function, because the free
function alone eagerly incrementally-saves and cannot back an interactive,
undo-able Accept. **Pass 16.0 shipped BOTH from day one** — confirmed by
reading `crates/pdfce-core/src/edit.rs` lines 1680-1780 directly:

```rust
// crates/pdfce-core/src/text_edit/addtext.rs — the free function (CLI, tests)
pub fn add_text(doc: &Document, req: &AddTextRequest) -> Result<AddTextOutcome, AddTextError>;

// crates/pdfce-core/src/edit.rs — the session-integrated sibling (THIS Pass's Accept)
impl EditSession {
    pub fn add_text(&mut self, req: &AddTextRequest)
        -> Result<AddTextReport, AddTextError>;
}
```

Both share ONE planner (`pub(crate) fn plan_add_text`), so they can never
drift — the session method commits `Command { kind: CommandKind::AddText,
objects: [content_stream, font_dict, page_dict], .. }` as ONE undo entry,
mirroring `edit_text`/`format_text`/`reflow_block`'s established shape line
for line. **There is no §0.2-class gap to flag for point-text Add/Accept —
§4/§7 below wire directly to this shipped method.**

### 0.3 What genuinely IS missing — 16.1 (boxed/wrap add) has not shipped, and this Pass's box-mode half depends on it

Confirmed by grepping `crates/pdfce-core/src` for `wrap_width`/`AddTextBox`/
`with_box`: **only reflow's own (Pass 15.x) wrap-width concept exists.**
`AddTextRequest` (§0.2) has `origin: (f64, f64)` and nothing else — no width,
no box, no multi-line concept at all. Decision 016 §6 slices this
deliberately: 16.0 = point only, 16.1 = boxed + wrap via the 15.x reflow
engine, 16.2 (this spec) = the canvas UI for BOTH.

**This spec designs the FULL click-or-drag interaction decision 016's own
16.2 line calls for (§3), but box-mode's commit path and its live wrap
preview are written against an ANNOUNCED contract, exactly the discipline
14.3 §0.2/15.2 §0.2 already established for their own core-accessor asks —
named explicitly here so 16.1 confirms or corrects, not silently guesses:**

1. **A mutating counterpart, mirroring 16.0's own free-function +
   session-sibling shape exactly** — e.g. `AddTextRequest` grows an optional
   box (`.with_wrap_width(w).with_leading(l)`) that a shared planner
   (`plan_add_text`, extended) routes through the 15.x reflow engine for
   line-breaking, OR a parallel `AddTextBoxRequest`/`add_text_boxed` +
   `EditSession::add_text_boxed` — 16.1's call which shape, not this spec's;
   named so the property bar's one Accept call site (§7) is written to
   change cleanly whichever way 16.1 lands, not guessed at twice.
2. **A PURE, read-only wrap-PREVIEW function, independent of any mutating
   call** — needed for box-mode's live-typing preview (§4.2) the same way
   15.2's live reflow re-preview needed `ReflowEngine::preview` to be a pure,
   cheap, read-only computation recomputable every frame. Decision 016 says
   16.1 "wraps through the already-shipped 15.x reflow engine" — but 15.0's
   `ReflowEngine::preview` takes a `block_index` into an ALREADY-RECOGNIZED
   `EditableTextModel`; it has no entry point for "wrap this literal,
   not-yet-committed STRING into a box of this width, in this font/size,"
   which is what Add-Text's live composition needs framing every keystroke.
   **Required, flagged explicitly:** a pure function shaped roughly
   `text_edit::addtext::preview_wrap(text: &str, wrap_width: f64, leading:
   Option<f64>, font: Std14, size: f64) -> AddTextWrapPreview` (or
   equivalent), returning per-line origins/text/an overflow flag — the
   read-only analogue of `ReflowPreview`, but wrapping a literal string
   instead of re-deriving an existing block. Without this, §4.2's live
   per-keystroke wrap preview cannot be built without the GUI hand-rolling
   its own greedy-wrap approximation — precisely the duplication-drift risk
   named repeatedly elsewhere in this project (14.3 §7's `font_subset_stem`,
   15.2 §0.3's `reflow_recognition_options`) — the wrap decisions the
   operator reviews pre-Accept MUST be the exact same ones the commit
   re-derives, never a GUI-side approximation of them.

**Until 16.1 ships, this Pass's point-mode half (§3.1's click path, §4.1,
§6, §7's `EditSession::add_text` call) is fully buildable today.** Box-mode
(drag path, §3.2, §4.2, the box-Accept call) is specified in full but is
correctly BLOCKED on 16.1 — flag this dependency plainly to the engineer
rather than half-building an unreachable box-mode UI ahead of its backend.

### 0.4 Font enumeration for the property bar — already solved, no new core surface needed

Initial expectation (before reading the shipped code) was that Add-Text's
font choice would need a new "list every registered Supplied face" GUI-side
accessor. **It does not.** `AddTextRequest::base_font: Std14` is constrained
to exactly the 14 canonical §9.6.2.2 faces (`pdfce_core::fontdata::Std14`,
14 variants, no more, no less — the spec defines exactly 14 and this project
never aliases beyond them, confirmed in `fontdata/mod.rs`'s own module docs).
"Supplied" does not mean "any font the operator has" — it means "the
operator has registered, via a font-folder, a face that answers to ONE of
these 14 exact names, so pdfce renders the PREVIEW with that face's real
shapes; the WRITTEN `/BaseFont` name and dict are identical either way
(§9.6.2.2, non-embedded, no `/FontFile`)." This is EXACTLY 14.1/14.3's own
Bundled-vs-Supplied rendering distinction, reused unchanged, not
reinvented — confirmed by reading `cmd_add_text` in `pdfce-cli/src/main.rs`:

```rust
let base_font_name = std14_base_font_name(font);         // one of the 14 exact spellings
let provenance = match font_env.classify_nonembedded(base_font_name) {
    pdfce_render::GlyphSource::Supplied => FontProvenance::Supplied,
    _ => FontProvenance::Bundled,
};
```

The property bar's Font `ComboBox` (§5) does exactly this ONE call per
candidate name, reusing `FontEnvironment::classify_nonembedded` (already
public, already hoisted for 14.3) — **no new core or render accessor is
needed for font trust labeling.** The only small, optional nicety (P1, not
blocking): `Std14` has no public `ALL: [Std14; 14]`/iterator (`ALL_14` at
`fontdata/mod.rs` line 814 is `#[cfg(test)]`-only) — the GUI must otherwise
hardcode the same 14-arm ordered list the test fixture already encodes.
Because the Standard-14 set is spec-frozen (ISO 32000-1 §9.6.2.2 defines
exactly 14, never revised), this is materially lower-risk than the
`reflow_recognition_options`/`font_subset_stem` hoists (those guarded
*tunable* values); still cheap and worth doing in the same Pass — flagged in
§13, not blocking.

---

## 1. Tool discovery and entry

### 1.1 The naming-collision problem this control must solve — read `text_menu_tooltip()` first

The shipped `"Text ▾"` menu (Pass 6.2, `crates/pdfce-gui/src/ui_text.rs`
lines 396-404) is the FreeText/Sticky-note/Stamp **annotation** authoring
menu, and its CURRENT tooltip reads:

> *"Add a text box, sticky note, or stamp to the current page. This changes
> the document and is saved with it — use Undo to reverse it."*

**"Add a text box... to the current page" is precisely the phrase an
operator would reach for to describe what THIS Pass's tool does** — this is
the real, sourced naming collision decision 016 §3.1/the Acrobat-parity
catalog names, now live in pdfce's own shipped copy, not hypothetical. R78
requires this Pass's new control to be unmistakably distinct in label AND
requires `text_menu_tooltip()`'s own wording to be updated (§13 change list)
so an operator who reads it is pointed at the RIGHT tool for "real page
text," not left to guess.

### 1.2 Placement — third occupant of the family, adjacent to (not inside) Edit Text

The existing toolbar cluster (`crates/pdfce-gui/src/main.rs` lines
3567-3659):

```
… | [Markup ▾] [Text ▾] [✎ Aa  Edit Text] | [Undo] [Redo] | …
```

**New button inserted immediately after Edit Text, inside the SAME visual
group** (no new separator before it — the adjacency itself is a
discoverability aid: it signals "these two are the page-content-editing
pair," distinct from the `Text ▾`/`Markup ▾` annotation-authoring cluster
one position to the left):

```
… | [Markup ▾] [Text ▾] [✎ Aa  Edit Text] [+ Aa  Add Text] | [Undo] [Redo] | …
```

- **Icon glyph:** `"+ Aa"` (plus-sign prefix + the SAME "Aa" root Edit Text
  already uses) — deliberately NOT the pencil (`✎`) Edit Text uses (that
  glyph now means "modify what's there"); a real, distinct glyph, not a
  colour variant (rule 6). The shared "Aa" root signals family membership
  ("this is about the page's actual text, like its neighbor"); the prefix
  difference (pencil vs plus) signals the DIFFERENT operation.
- **Widget:** `ui.add_sized(ICON_BUTTON_SIZE, egui::Button::selectable(active,
  ui_text::add_text_tool_button()))` — the IDENTICAL widget/sizing Edit
  Text already uses (`main.rs` line ~3648), never a smaller ad hoc control.
- **Label:** `add_text_tool_button() = "+ Aa"`.
- **Tooltip (proposed, engineer may adjust wording, not the disambiguation
  content):**
  ```rust
  pub fn add_text_tool_tooltip() -> &'static str {
      "Add brand-new text to the page itself — a label, caption, or note \
       that becomes real, permanent page content, exactly like the text \
       already here (Ctrl+Shift+E). This is NOT the same as Text ▾ → Text \
       box (a removable annotation) — for a comment or sticky note instead, \
       use Text ▾. To fix text that's already on the page, use Edit Text."
  }
  ```
  Three sentences, each doing one job: what it does, what it is NOT (the
  R78 disambiguator, naming the EXACT competing control by its own visible
  label), and where the third related-but-different tool is. This is
  longer than Edit Text's own two-sentence tooltip because there are now
  THREE adjacent, easily-conflated controls (`Text ▾`, Edit Text, Add Text)
  where 14.3 only had two to distinguish (§13 also updates Edit Text's own
  tooltip to close the loop the other way).
- **Toggle semantics:** `Action::SelectCanvasTool(Some(CanvasTool::AddText))`
  when off, `Action::SelectCanvasTool(None)` when on and clicked again —
  identical to Edit Text's own wiring (`main.rs` lines 3651-3657), one line
  changed.
- **Keyboard chord:** propose **Ctrl+Shift+E** — the same mnemonic letter as
  Edit Text's `Ctrl+E`, Shift signalling "the bigger/creation variant."
  **Verified free at spec-authoring time** (grepped
  `collect_keyboard_actions` for every `Modifiers::COMMAND.plus(Modifiers::
  SHIFT)` chord: only `Ctrl+Shift+Z`=Redo and one other at line ~3297 are
  bound — `Ctrl+Shift+E` is unclaimed) — **re-verify at implementation time
  before shipping**, per every prior Pass's own instruction on this point;
  do not treat this spec's grep as a substitute for the engineer's own
  check at commit time.

### 1.3 Rule-3 tension, addressed directly

Rule 3 (progressive disclosure, lean primary toolbar) is a real concern
here: this is a FOURTH always-visible control in the edit group (after
`Markup ▾`, `Text ▾`, Edit Text), where `Markup ▾`/`Text ▾` are already
MENUS specifically to avoid exploding into one button per shape/annotation
kind. Two alternatives considered and rejected:

- **Nest Add Text as a secondary item behind a small caret on the Edit Text
  button (a split-button).** Rejected: this recreates §0.1's own rejected
  hazard one level down — a control whose PRIMARY click does one thing
  (toggle Edit Text) and whose caret reveals a DIFFERENT thing (enter
  AddText), for two operations that are frequently alternated between in
  the same authoring session. A bare, always-visible, clearly-labelled
  toggle is more discoverable, not less, for exactly the two highest-
  frequency text operations in the app.
- **Move it into the Tools dock instead of the primary toolbar.** Rejected:
  this would treat "add new text" as an advanced/occasional feature on par
  with Font-folders management or redaction, when decision 016 §2 names it
  "among the most common Acrobat text actions" — rule 3's OWN standard
  ("lean primary toolbar... advanced feature groups... in secondary
  panels") argues FOR keeping a frequent, primary editing action in the
  primary toolbar, not against it. Acrobat's own well-documented
  menu/ribbon-overload complaint is about deeply NESTED chrome for RARE
  features, not about a small number of mutually-exclusive top-level MODE
  toggles for a document's single most common editing surface (text) —
  the same reasoning that already justified Edit Text living as a bare
  toggle rather than inside `Text ▾`.

**Verdict: ship it as a bare toggle, adjacent to Edit Text, per §1.2.** If a
future Pass adds a THIRD or FOURTH sibling text-authoring mode, that is the
point to revisit whether the pair should collapse into a small dropdown —
not yet, with only two.

### 1.4 Disabled state

Grey out (not hide) when no document is open — `ui.add_enabled_ui(!doc.
pages.is_empty(), ..)`, matching Edit Text's own precedent (`main.rs` line
3643) and every other document-scoped toolbar control. No new pattern.

---

## 2. State model

```rust
/// On `OpenDoc`, session-only, exactly like `text_edit`/`active_tool`/
/// `canvas_selection`. `None` whenever `CanvasTool::AddText` is not the
/// active tool.
add_text: Option<AddTextState>,

struct AddTextState {
    /// The page this state targets — rebuilt (not carried) on page
    /// navigation while the tool stays active, mirroring `TextEditState`'s
    /// own §2.1 rule; there is no meaningful "in-progress add" to carry
    /// across a page boundary.
    page_index: usize,
    /// Property-bar working values, seeded on tool entry from the
    /// operator's DEFAULT preference (§5.1's Font-folders-panel control),
    /// then editable per-use without changing that preference.
    prop_font: pdfce_core::fontdata::Std14,
    prop_size: f64,
    prop_model: pdfce_core::text_edit::FillModel,
    prop_components: [f64; 4],
    /// The keyboard-complete manual-entry fields (§3.3) — always present,
    /// editable whether or not a mouse gesture has happened yet.
    manual_origin: [f64; 2],
    /// `Some([w, h])` only once the operator has chosen box mode manually
    /// (§3.3) or dragged a box (§3.2) — `None` means point mode.
    manual_box_size: Option<[f64; 2]>,
    /// The in-progress placement + composition — at most one at a time,
    /// this tool's own discardable `GestureInterrupt` (§8).
    draft: Option<AddTextDraft>,
    /// The most recent ACCEPTED add's disclosures, rendered verbatim until
    /// the next Accept or tool exit (mirrors `TextEditState::
    /// last_disclosures` exactly, §6.1).
    last_disclosures: Vec<String>,
}

/// A fixed origin/box (never repositioned mid-compose — reposition means
/// Reject, then re-place) plus the operator's in-progress new text. Created
/// the MOMENT a placement gesture completes (§3), even before any character
/// is typed, so the insertion caret/box outline appears immediately and
/// Reject/Esc is available right away — Accept stays disabled while
/// `draft_text` is empty (nothing to add yet, §6.3).
struct AddTextDraft {
    placement: Placement,
    /// Empty immediately after placement; grows with typing. No wrapping
    /// logic touches this in point mode — box mode additionally recomputes
    /// `wrap_preview` every frame it changes (§4.2).
    draft_text: String,
    /// Box mode ONLY (16.1-gated, §0.3) — the live, pure wrap preview,
    /// recomputed every frame `draft_text`/the box size/font/size changes.
    /// `None` for point mode (nothing to wrap).
    wrap_preview: Option<Result<AddTextWrapPreview, AddTextWrapError>>, // 16.1 shape TBD
    /// The last Accept ATTEMPT's refusal `Display` text, kept visible while
    /// the operator revises — mirrors `PendingEdit::last_refusal` exactly
    /// (never a silent dead end, §6.4).
    last_refusal: Option<String>,
}

/// PDF-space placement, fixed at creation.
enum Placement {
    /// A single-line run growing rightward from `origin` (16.0, shipped).
    Point { origin: (f64, f64) },
    /// A wrap box, top-anchored (16.1-gated, §0.3).
    Box { origin: (f64, f64), width: f64, height_hint: f64 },
}
```

### 2.1 When the state is (re)built

- **On tool entry** (`Action::SelectCanvasTool(Some(CanvasTool::AddText))`):
  build `AddTextState` for the CURRENT page; seed `prop_font`/`prop_size`/
  `prop_model`/`prop_components` from the operator's persisted default
  preference (§5.1); `draft = None`.
- **On page navigation while the tool stays active:** rebuild for the new
  page, `draft = None` — an in-progress, unaccepted add on page 3 has no
  sensible meaning on page 4 (mirrors `TextEditState`'s identical rule).
- **After an accepted Add (§7):** the tool STAYS active (§7.1) and
  `AddTextState` is rebuilt for the SAME page (cheap; no `EditableTextModel`
  is even involved here, unlike `TextEditState`'s post-edit rebuild, since
  Add-Text does not read page text at all — only `page_index`/prop-bar
  values persist; `draft` clears).
- **Never per-keystroke** for point mode (no core call at all while
  composing, §4.1); box mode DOES recompute a PURE, read-only wrap preview
  every frame `draft_text` changes (§4.2) — this is the same "recompute a
  cheap derivation live, mutate only on commit" precedent 15.2 already
  established for `ReflowEngine::preview`, not a violation of "never call
  core per keystroke" (that rule is about MUTATING calls, not pure,
  read-only ones — see §4.2's explicit framing).

---

## 3. Placement — click for point, drag for box

### 3.1 Point mode (16.0, buildable today)

`image_response.clicked()` while `CanvasTool::AddText` is active and no
`draft` yet exists:

1. Convert the click's canvas-space point to PDF space via
   **`viewer::canvas_to_pdf_space(canvas_point, page)`** — the SAME bridge
   14.3 §3 established as the correct one for canvas-space → PDF-space
   conversion (never `screen_to_page`, which yields device/rotated canvas
   space, not the PDF user-space coordinates `AddTextRequest::origin`
   expects).
2. On success, set `draft = Some(AddTextDraft { placement: Placement::Point
   { origin: (pdf.x, pdf.y) }, draft_text: String::new(), wrap_preview:
   None, last_refusal: None })`.
3. A blinking insertion caret appears at `origin` immediately (§4.1's
   rendering) — Reject/Esc is available at once even before typing starts
   (nothing has been written anywhere yet, so cancelling costs nothing,
   rule 7).

**A click while a `draft` already exists** (the operator clicked again
before Accepting/Rejecting the current one) is this tool's `GestureInterrupt
::Discard` case (§8) — the SAME "click elsewhere discards the in-progress,
uncommitted thing" precedent 14.3/15.2 already use for `PendingEdit`/
`ReflowState`; the click that triggered the discard THEN starts a fresh
placement at its own point, exactly as a `TextEdit` click after a discarded
`PendingEdit` immediately sets a new caret.

### 3.2 Box mode (16.1-gated, §0.3 — specified in full, blocked on backend)

`image_response.drag_started()`/`dragged()`/`drag_stopped()` while
`CanvasTool::AddText` is active — **the FIRST "define a brand-new rectangle
from scratch by dragging on blank canvas" gesture in the app**, worth
flagging as such: 15.2's width-handle drag ADJUSTS one edge of an ALREADY-
known block; 14.3's selection drag EXTENDS a span over EXISTING glyphs;
neither is "sketch an arbitrary new rect from nothing," so this is new
ground, even though the underlying `drag_started`/`dragged`/`drag_stopped`
egui idiom is the exact one 14.3/15.2 already use elsewhere.

```rust
if image_response.drag_started() {
    // Anchor corner, canvas space, converted once at drag start.
    box_anchor_canvas = image_response.interact_pointer_pos();
} else if image_response.dragged() {
    // Live rubber-band: anchor .. current pointer, both converted every
    // frame — no accumulated delta, the same "recompute from the absolute
    // current position" discipline 15.2 §6.1 already uses for its width
    // handle (no drift).
} else if image_response.drag_stopped() {
    // Normalize (min/max so any drag direction works), convert BOTH
    // corners via canvas_to_pdf_space, commit the box — OR fall back to
    // point mode, next paragraph.
}
```

**Live rubber-band rendering (while dragging, before ANY text exists):** a
plain dashed rectangle, `egui::Stroke::new(1.5, egui::Color32::from_rgb(210,
90, 40))` — the IDENTICAL "not yet real" amber/orange `PendingEdit` already
uses for its dashed border, at the SAME 1.5px weight, reused rather than
inventing a new colour for "I am sketching a region" (rule 6 — this project
already reserves this hue family for "reviewable, not-yet-applied"; a
generic blue "selection marquee" colour would be a needless FOURTH visual
vocabulary alongside the caret/selection/preview/refusal set already
established). No "PREVIEW" tag yet at this stage — that appears once text
actually exists (§4.2), matching the same "the tag names a REAL preview,
not an empty gesture" restraint point mode's own caret-before-typing state
already exercises implicitly (no tag on a bare caret either).

**Degenerate-drag handling — a DELIBERATE divergence from Pass 6.1's own
"degenerate-drag discards silently" precedent, named explicitly:** if the
final box (in PDF-space points, or in screen pixels before conversion) is
smaller than a small positive floor (`MIN_ADD_TEXT_BOX_PT`, mirroring
15.2's own `MIN_WRAP_WIDTH_PT = 12.0` naming convention), **do not discard —
fall back to point mode, anchored at the drag's START position.** This is a
deliberate, reasoned divergence from Pass 6.1's shape-drawing precedent
(where a near-zero-size square/circle has no sensible minimal form to keep,
so silently discarding is correct): here, a near-zero drag still names a
perfectly sensible, valid construct — a point-text insertion at the
gesture's start — so falling back preserves the operator's evident intent
(they clearly meant to place SOMETHING) instead of silently eating an
almost-successful gesture. State this reasoning in the shipped code's
comment so a future reader does not "fix" it back to Pass 6.1's discard
convention, mismatching this operation's own honest semantics.

**On a successful (non-degenerate) `drag_stopped()`:** `draft =
Some(AddTextDraft { placement: Placement::Box { origin: (min_x, min_y... —
top-anchored per decision 016 §3.2/15.x's own "top-anchored vertical growth"
convention), width, height_hint: 0.0 /* grows with content, 16.1 */ },
draft_text: String::new(), wrap_preview: None, last_refusal: None })`.

### 3.3 Keyboard-complete alternative entry — named accessibility improvement, not a documented gap

Pointer-first placement (click/drag on the canvas) is, like Pass 8's own
freehand marking, "inherently pointer-first... a real, non-fixable-via-egui
limitation of drag gestures generally" (that spec's own honest framing,
reused here rather than re-litigated). **Rather than merely NAME this gap**
(as Pass 8 did for its own drag-marking), this Pass adds a genuine,
keyboard-reachable alternative: the property bar (§5), shown the MOMENT
`AddText` becomes active (even before any placement gesture), carries real,
Tab-focusable `egui::DragValue` fields:

```
┌─ Add Text ───────────────────────────────────────────────────────┐
│ Origin X, Y (pt): [ 72.0 ▲▼ ] [ 700.0 ▲▼ ]     [ Place point ]     │
│ ☐ Use a box instead:  W (pt): [ 200.0 ▲▼ ]  H (pt): [ 40.0 ▲▼ ]    │
└────────────────────────────────────────────────────────────────────┘
```

Typing exact coordinates then clicking **"Place point"** (or, with the "Use
a box instead" checkbox on, **"Place box"**) creates `draft` exactly as a
mouse gesture would (§3.1/§3.2) — a fully keyboard-operable path to the
SAME state a click/drag reaches, not a lesser fallback. This is a concrete,
buildable accessibility win this Pass can bank (unlike Pass 8's redaction-
marking gap, which had no such natural numeric-entry equivalent for an
arbitrary freehand region) — flag it as such in the shipped code's module
doc so it reads as a deliberate design decision, not an incidental feature.

---

## 4. Composing + live preview

### 4.1 Point mode — no core call per keystroke, reuses `PendingEdit`'s exact visual language

Typing (any character key, Backspace, Delete) while `draft.is_some()` and
`draft.placement` is `Point` appends/removes from `draft_text` directly —
**identical to `PendingEdit`'s own §6.1 rule**: no core call happens per
keystroke; the preview draws `draft_text` using an egui built-in
proportional font sized from `prop_size × zoom` (the SAME approximation
14.3 §6.3 already uses and accepts — real glyph shapes only exist after a
real Accept triggers a real re-render; this Pass adds no new font-shaping
capability either).

**Rendering, reusing 14.3's exact painter conventions (`main.rs` lines
~5202-5251), generalized from "the original run's projected bbox" to "a box
anchored at the placement origin, sized to the current draft":**

1. A translucent mask, `egui::Color32::from_rgba_unmultiplied(250, 250, 250,
   220)` — the IDENTICAL fill `PendingEdit` uses, over a screen-space box
   computed from `origin` (bottom-left) growing rightward by the draft
   text's approximate measured width (egui's own text-measurement for the
   same font/size used to draw it, so the mask always exactly contains what
   is drawn — no guessed padding beyond `PendingEdit`'s own `.expand(2.0)`
   convention).
2. `draft_text` painted via `painter.text(..)` at the box's left-bottom,
   same `egui::FontId::proportional(size)`/`egui::Color32::from_rgb(20, 20,
   20)` as `PendingEdit`.
3. Dashed border (`Stroke::new(1.5, Color32::from_rgb(210, 90, 40))`) plus
   the SAME `ui_text::preview_tag()` ("PREVIEW — not yet applied") corner
   label — shown ONLY once `draft_text` is non-empty (an empty draft shows
   only the blinking caret, §4.3, no tag — there is nothing yet to call a
   preview of).
4. **One semantic nuance worth naming explicitly, not silently reusing
   without comment:** `PendingEdit`'s mask exists to visually SUPPRESS the
   original glyphs underneath it (so old and new text do not collide on
   screen). Add-Text's mask, in point mode, usually covers BLANK space
   (nothing is being suppressed) — except when the operator deliberately
   places new text over existing content (§0.1 point 3), where it plays
   `PendingEdit`'s exact original role. Either way the VISUAL TREATMENT is
   identical (rule 6/the "one visual language for reviewable-not-yet-real"
   discipline) — only the underlying reason the mask exists differs
   case-by-case, which the operator does not need to know to read the
   preview correctly.

### 4.2 Box mode — a PURE, per-frame wrap-preview call, NOT a violation of "no core call per keystroke" (16.1-gated, §0.3)

Once §0.3's pure wrap-preview accessor exists, box-mode composing
recomputes it every frame `draft_text` (or the box width/font/size)
changed — **the same "recompute a cheap, read-only derivation live, commit
only on Accept" precedent 15.2 §6 already established for `ReflowEngine::
preview`, extended from "re-plan an existing block" to "wrap this literal
string."** This is explicitly NOT the thing 14.3 §6.1 forbids ("no core
call happens per keystroke") — that rule targets MUTATING calls that would
force an incremental-save-and-reparse cycle on every character; a pure,
read-only wrap computation over an in-memory string is exactly the
category 15.2 already normalized for live review. Name this distinction in
the shipped code's comment so a future reader does not conflate the two
rules and either (a) wrongly avoid the live wrap preview citing 14.3's rule,
producing an inaccurate/stale preview, or (b) wrongly extend point mode to
also do per-keystroke core calls it does not need.

**Ghost-line rendering** reuses 15.2 §5.2's exact generalized convention
(mask over the box, each wrapped line painted via the SAME egui-approximate
font, dashed PREVIEW border + tag on the box, a wrap-width guide line when
alignment is non-Left if 16.1 exposes an alignment choice for Add-Text —
TBD against 16.1's actual shape, flagged in §0.3) — no new visual language
invented for box mode either.

### 4.3 The insertion caret (point mode) / box outline (box mode) before typing starts

Immediately on placement (§3), before any keystroke: a blinking vertical
caret (point mode) drawn via the SAME technique 14.3 §5 uses (`ui.painter()`
directly, `egui`'s standard blink interval, never a re-raster) — but sized
from `state.prop_size` (the property bar's CURRENT font-size value) rather
than an existing glyph's recorded size, since there is no existing glyph to
read one from. Box mode shows the placed rectangle's outline (solid, thin,
neutral) with no text yet.

---

## 5. Font default + provenance surface (R79)

### 5.1 The PREFERENCE — a new control on the existing Font-folders Tools-dock panel, not a new settings surface

Decision 016 §3.3 requires the bundled-Std14 default to be
"operator-configurable (a preference plus a per-use override)." Rather than
inventing a new settings surface, add ONE new control to the ALREADY-SHIPPED
`Tool::FontFolders` panel (`font_folders_tool`, `main.rs` line ~2389) —
directly adjacent to the font-folder list it is thematically part of:

```
┌─ Font folders ──────────────────────────────────────────────────┐
│ … (existing folder add/remove list, unchanged) …                 │
│                                                                    │
│ Default font for new page text: [ Helvetica ▾ ]                  │
│ ⓘ Helvetica is bundled (no font-dir face registered for this name)│
└────────────────────────────────────────────────────────────────────┘
```

- A `ComboBox` listing the 14 `Std14` variants (§0.4's fixed, hardcoded-but-
  spec-frozen list) by `std14_base_font_name`, each optionally suffixed
  `" (supplied)"` when `font_env.classify_nonembedded(name) ==
  GlyphSource::Supplied` — the IDENTICAL trust computation `cmd_add_text`
  already performs, reused verbatim, never re-derived.
- Persisted on `PdfceApp` as `default_add_text_font: pdfce_core::fontdata::
  Std14` (session-scoped at minimum, matching `font_folders`'s own session
  scope — whether it additionally persists ACROSS sessions via whatever
  config mechanism the engineer chooses for `font_folders` itself, if any,
  is that same mechanism's call, not a new one this Pass invents).
- **Per-use override:** `AddTextState.prop_font` (§2) is SEEDED from this
  preference on tool entry, then freely changed for THIS session's adds via
  the property bar's own Font control (§5.2) without touching the
  preference — exactly the "preference plus per-use override" decision 016
  asks for, and the same seed-then-diverge relationship 15.2 §2.1 already
  uses for `ReflowState.width`/`.leading` seeded from a first preview.

### 5.2 The per-use override — the property bar's Font control

In `AddTextState`'s property bar (shown while `draft.is_some()`, alongside
size/colour, mirroring 14.3 §7's layout):

```
┌─ Add Text ─────────────────────────────────────────────────────┐
│ Font: [ Helvetica ▾ ]  Size: [ 12.0 ▲▼ ] pt                      │
│ Color model: (○ RGB ○ Gray)   [ swatch/sliders ]                 │
└────────────────────────────────────────────────────────────────────┘
```

- Font `ComboBox` — same 14-entry list + trust suffix as §5.1's preference
  control (same code, two call sites — a small, deliberate, cheap
  duplication of RENDERING the same list, not of the underlying trust
  LOGIC, which stays the one `classify_nonembedded` call). Changing this
  selection changes `AddTextState.prop_font` for the CURRENT draft only.
- **Color model** — decision 016's shipped `NewTextColor` only has `Black`/
  `Rgb(r,g,b)` (confirmed in `addtext.rs`, no CMYK/Gray variant exists for
  Add-Text, unlike 14.2's `FillModel` which supports Gray/RGB/CMYK for
  EDITING existing runs). The property bar therefore offers exactly what
  core can express — **do not offer a Gray/CMYK radio that core would
  silently coerce or refuse**; a two-state ("Black" default / "Custom RGB")
  toggle, or a plain `ui.color_edit_button_srgba` defaulting to black and
  read back as `NewTextColor::Rgb` only when changed from pure black, is the
  honest, buildable surface. Flag to the engineer: if 16.1 or a later slice
  widens `NewTextColor`, this control widens with it — do not pre-build UI
  for a colour model core cannot yet accept.
- **Size** — plain `egui::DragValue`, identical convention to 14.3 §7's own
  Size control.

### 5.3 Glyph coverage — refused-and-disclosed at Accept (P0), a proactive live check is a named, optional fast-follow (P1)

Baseline (P0), mirroring 14.3 §6.4/§8.2 exactly: a character the chosen
`Std14` face's encoding cannot represent is caught by `AddTextRequest`'s own
`InverseEncoding` at commit time and surfaces as `AddTextError::Refused
(Refusal { trigger: RInvTrigger, .. })` — handled by §6.4's refusal table,
never silently dropped or substituted.

**Optional, named fast-follow (P1), NOT required for this Pass:** a live,
per-keystroke highlight of exactly which character(s) in `draft_text` the
CURRENT `prop_font` cannot represent, so a long composed sentence is not
refused wholesale only after Accept. This would need a new, cheap, PURE
coverage query (`Std14`/`FontEnvironment`-adjacent, e.g. "does this face's
encoding table have a code for this Unicode scalar" — a simple table lookup,
not a rendering call) that does not exist today; flagged in §13 as a
genuine but non-blocking enhancement, not silently assumed to exist.

---

## 6. Disclosures — rendered verbatim, nothing paraphrased

### 6.1 The disclosure strip (accepted adds)

A slim, non-modal strip in the SAME visual convention as 14.3 §8.1's — a
NEW `egui::Area` (`"pdfce-add-text-status"`, mirroring the naming
convention of `"pdfce-text-edit-status"`), since `AddText` is a distinct
tool with its own state, not a shared strip with `TextEdit`/reflow. On every
successful Accept (§7), render `report.disclosures` VERBATIM, one bullet per
string, in commit order — reading the actual shipped `plan_add_text` source
(`addtext.rs` lines 557-579) shows **the report can carry UP TO THREE
distinct disclosure strings per add, not just the tagged-untagged one** —
this Pass's strip must render ALL of them, not only the R73 one a
first-glance reading of decision 016 §3.6 might suggest is the only one:

1. **Always present** — the font-provenance disclosure: *"new run uses a
   {bundled|supplied} Standard-14 face '{name}' by name+code — no glyph
   embedding (R79 / ISO 32000-1 §9.6.2.2); provenance is disclosed, not the
   document's own."*
2. **Conditional** — the §7.7.3.4 inheritance-trap disclosure, present
   whenever `report.gave_page_own_resources` is true: *"this page INHERITED
   its /Resources from an ancestor /Pages node; pdfce gave the page its OWN
   /Resources... the shared ancestor resources were NOT modified."* This is
   a genuinely new disclosure category this Pass's UI has not needed
   before (14.3/15.2 never touch `/Resources` inheritance) — surfaced with
   the SAME ⓘ icon + verbatim text discipline as every other disclosure,
   not silently dropped for being unfamiliar.
3. **Conditional** — the R73 tagged-untagged disclosure, present whenever
   `report.tagged_untagged` is true: *"new run added as untagged page
   content; the structure tree / reading order was not updated — no tag
   created (R73)."*

Use ⓘ for all three (none of them name a real limitation the way 14.3's
overflow disclosure does — they are informational, not warnings) — pair
icon with text, never color alone (rule 6). Persist until the next Accept
or tool exit, never auto-dismissed (an operator reading slowly must not
have it vanish mid-read — 14.3 §8.1's exact reasoning, reused unchanged).

### 6.2 Refusals — the REAL `AddTextError`/`RInvTrigger` table, not a placeholder

Unlike 15.2 (which had to write a provisional table against an unshipped
error type), 16.0's `AddTextError` is ALREADY SHIPPED — this table is
written against the real variants (`addtext.rs` lines 293-337), not a guess:

| `AddTextError` variant | "What would lift it" framing to append |
|---|---|
| `Refused(Refusal)` where `trigger` is `RInvTrigger::TargetAbsent` (R-INV-1) or `BeyondRepertoire` (R-INV-8) — the two realistically reachable triggers for a fresh Standard-14 encode (`CodeOccupied`/`LigatureOnly`/`Composite`/`SymbolicNoEncoding`/`ToUnicodeOnly` govern EXISTING-font read/re-encode cases 14.1 handles and are not expected to fire from a from-scratch Std-14 build, but are not asserted unreachable — reuse the SAME per-trigger hint text 14.3 §8.2 already authored, keyed by `trigger.id()`, rather than writing a second copy) | "Choose a different Standard-14 face — Symbol/ZapfDingbats cover a different repertoire than the Latin faces — or supply a font via Tools → Font folders for this exact name so pdfce can render more of what it names, though the WRITTEN dict is unaffected either way." |
| `PageIndex(usize)` | (should be unreachable from the GUI — the page index always comes from the currently-open, currently-displayed page; if it fires anyway, treat as an internal-consistency bug, mirroring 15.2 §7.3's identical framing for its own unreachable-in-practice variant) |
| `EmptyText` | Accept is already DISABLED while `draft_text.is_empty()` (§2/§6.3) — this variant should be unreachable from the GUI's own Accept gating; if it fires anyway, same "internal-consistency bug" framing as above. |
| `InvalidSize(f64)` | "Choose a positive font size." (the property bar's `DragValue` should itself clamp to a positive floor — a defence-in-depth refusal, not the primary guard) |
| `Encrypted` | "Adding text to an encrypted document is out of scope for this release." |
| `HiddenObjects { count }` | "This file's cross-reference table currently hides some entries in a way adding new objects would expose — this is a rare structural limitation, not something to work around by retrying." |
| `ObjectNumbersExhausted` | (practically unreachable — extremely large object-number space; no operator action would lift it) |
| `Unsupported(String)` | Render the core's own `String` verbatim (it already names the specific reason, e.g. "the page object is not a dictionary") — no generic GUI-authored hint needed on top. |
| `PageTree(PageTreeError)` / `Write(WriteError)` | These are document-structural/save failures, not Add-Text-specific — render verbatim, same treatment every other command's structural/save error already gets elsewhere in the app (no new convention). |

Styled identically to 14.3 §8.2's refusal treatment: a warning-coloured
border + ✖ glyph (never colour alone), directly beneath the still-visible
Accept/Reject controls, kept visible (not cleared) so the operator can
revise `draft_text`/the font choice and retry, per rule 4's "never a dead
end."

---

## 7. Accept / Reject

### 7.1 Accept — one `EditSession::add_text` call (point mode; box mode per §0.3 once 16.1 ships)

```rust
if do_accept_add_text
    && let Some(page) = doc.pages.get(add.page_index)
    && let Some(draft) = &add.draft
    && let Placement::Point { origin } = draft.placement
{
    let req = AddTextRequest::new(add.page_index, origin, draft.draft_text.clone())
        .with_font(add.prop_font)
        .with_provenance(font_provenance_for(&self.font_env, add.prop_font)) // §0.4's
            // classify_nonembedded call, wrapped
        .with_size(add.prop_size)
        .with_color(color_for_prop(add.prop_model, add.prop_components));

    match doc.session.add_text(&req) {
        Ok(report) => {
            doc.refresh_pages(); // the page's content changed
            if let Some(add) = doc.add_text.as_mut() {
                add.draft = None;
                add.last_disclosures = report.disclosures; // §6.1, verbatim
            }
            // Tool STAYS active (§7.2) — ready for another add.
        }
        Err(err) => {
            if let Some(d) = doc.add_text.as_mut().and_then(|s| s.draft.as_mut()) {
                d.last_refusal = Some(ui_text::refusal_with_hint(
                    &err.to_string(),
                    add_text_refusal_hint(&err), // §6.2's table
                ));
            }
        }
    }
}
```

Keyboard binding: **Enter**, matching `PendingEdit`/reflow's own Accept
binding — consistent muscle memory across every "review, then commit" flow
in the app, regardless of which tool produced the review. **Disabled while
`draft_text.is_empty()`** (§2/§6.2's `EmptyText` row) — tooltip: *"Type some
text first."*

### 7.2 The tool stays active after Accept — continuity without a separate "added-text mode"

Unlike a single-shot tool that exits after one use, `AddText` remains the
active `CanvasTool` after a successful Accept, so the operator can click (or
drag) again to place ANOTHER new run — matching the real, expected workflow
of labelling several blank fields or margins in one pass, and matching
Acrobat's own precedent of its Add Text tool staying engaged across
multiple placements until the operator explicitly switches away. This is
the concrete realization of decision 016 §3.4/task item 6's "no separate
added-text mode": the JUST-ADDED run is immediately, ordinarily editable —
not through some special "added text" state, but simply because switching
to `TextEdit` (Ctrl+E) re-extracts the page's CURRENT content (§2.1 of the
14.3 spec's own tool-entry rule, unchanged by this Pass) and the new run is
there, indistinguishable from any other page text.

**A small, named P1 continuity convenience** (not required to satisfy the
task's "no separate mode" requirement, which is already met structurally
per above): a "Edit this text now →" button/link in the disclosure strip
(§6.1) that switches `active_tool` to `TextEdit` immediately after a
successful Accept. Whether it can ALSO place the caret directly inside the
just-added run depends on `AddTextReport` naming the new run's identity —
it currently does not (only `content_object`/`font_object`, the two new
PDF object numbers, not a `text_edit::TextPosition`). Flagged in §13 as an
optional core-report enhancement; the P0 version of this button simply
switches tools and lets the operator click the (now visibly present, right
where they placed it) new text themselves — fully discoverable without the
extra plumbing.

### 7.3 Reject — no core call

`Esc`, or a small ✕ button — clears `draft` with no core call, exactly
`PendingEdit`/`ReflowState`'s own Reject (§7.1 of 14.3, §7.1 of 15.2).
Nothing was ever written anywhere. Also satisfies Pass 12.0 §3.5's
`EscapeOutcome::CancelGesture` step (cancel the gesture, stay in the tool) —
no new Escape semantics.

---

## 8. `GestureInterrupt` / Escape wiring — one more disjunct on the SAME existing enforcement point

Exactly the pattern 15.2 §1.4 used to add `reflow` alongside `pending` on
the SAME query — this Pass adds `add_text`'s `draft` alongside BOTH:

```rust
fn current_gesture_interrupt(&self) -> GestureInterrupt {
    match &self.status {
        Status::Open(doc)
            if doc.text_edit.as_ref().is_some_and(|s| s.pending.is_some() || s.reflow.is_some())
                || doc.add_text.as_ref().is_some_and(|s| s.draft.is_some()) =>
        {
            GestureInterrupt::Discard
        }
        _ => GestureInterrupt::Nothing,
    }
}

fn discard_active_gesture(&mut self) {
    if let Status::Open(doc) = &mut self.status {
        if let Some(state) = doc.text_edit.as_mut() {
            state.pending = None;
            state.reflow = None;
        }
        if let Some(state) = doc.add_text.as_mut() {
            state.draft = None;
        }
    }
}
```

`canvas_tool_active` (the `ui()` top-level computation feeding
`resolve_escape`/`collect_keyboard_actions`) is ALREADY `doc.active_tool.
is_some()` — generic over which variant — so `AddText` being active
satisfies it with zero changes. **No second enforcement point is added; this
is the SAME one query, one more disjunct** — matching the substrate's own
documented design intent (`canvas.rs`'s own module docs: "the substrate does
not hardcode a policy, it only guarantees there is exactly ONE place the
question is asked").

---

## 9. Keyboard / accessibility

- **Discoverability:** toolbar button + tooltip (§1.2); no hidden-only entry
  point; greyed-not-hidden when no document (§1.4).
- **Keyboard access:** the manual X/Y/W/H origin entry (§3.3) makes
  PLACEMENT itself keyboard-reachable, not just composing/Accept/Reject —
  a genuine improvement over Pass 8's named-but-unsolved pointer-first gap
  for its own drag-marking gesture. Typing, Accept (Enter), Reject (Esc)
  are all standard keyboard input, identical to `TextEdit`'s own §10/§9
  precedent.
- **Real `egui` widgets for every interactive control:** the property bar's
  `ComboBox`/`DragValue`s, the manual-entry fields, "Place point"/"Place
  box," and Accept/Reject are all real `ui.button`/`egui::DragValue`/
  `egui::ComboBox` widgets (never painter-drawn) — banking Pass 12.0 §6.4's
  `accesskit` recommendation from the START, the same discipline 14.3 §10
  had to retrofit onto its own Accept/Reject.
- **Colour never sole signal:** the caret is a real line; the rubber-band/
  preview boxes are dashed-border + text-tag, never tint alone (§3.2/§4.1);
  disclosure vs. refusal uses icon + text, not border colour alone (§6).
- **Click targets:** the toolbar toggle, Accept/Reject, and "Place
  point"/"Place box" buttons all use `ICON_BUTTON_SIZE`/`add_sized` — no
  smaller ad hoc target, matching every prior Pass's own instruction on
  this point.
- **`accesskit`/screen-reader gap — inherited, not introduced:** the
  caret/rubber-band/ghost-preview TEXT painting is still painter-drawn (no
  sensible "real widget" form for a blinking caret or ghost glyphs) — the
  SAME standing gap Pass 12.0 §6.4 named, not a new instance. Every
  INTERACTIVE control (§ above) avoids adding to it.
- **Crash-safe autosave:** an in-progress `AddTextDraft` is never written
  anywhere until Accept (§7) — a crash mid-composition loses only the
  uncommitted new text, the same low-stakes class as `PendingEdit`'s own
  precedent (§10 of the 14.3 spec). No new autosave work; the existing
  post-Accept cadence picks up `CommandKind::AddText` automatically, since
  it is committed through `EditSession` exactly like every other command.

---

## 10. `ui_text.rs` catalog (new entries, proposed wording — engineer may adjust text, not presence)

- `add_text_tool_button() = "+ Aa"` (§1.2)
- `add_text_tool_tooltip()` (§1.2, full three-sentence disambiguator)
- **Updated, not new** — `edit_text_tool_tooltip()` gains a third sentence
  naming Add Text (§13's change list): e.g. *"...To fix existing text, use
  Edit Text (Ctrl+E). To add brand-new page text instead, use Add Text
  (Ctrl+Shift+E)."*
- **Updated, not new** — `text_menu_tooltip()` gains a disambiguating
  clause (§1.1/§13): e.g. *"...This is a removable annotation, not page
  content — for text that becomes a real, permanent part of the page
  itself, use Add Text instead."*
- `add_text_propbar_title() = "Add Text"` (§3.3/§5.2 panel header)
- `add_text_origin_label() = "Origin X, Y (pt):"`
- `add_text_place_point_button() = "Place point"`
- `add_text_use_box_checkbox() = "Use a box instead:"`
- `add_text_place_box_button() = "Place box"`
- `add_text_accept() = "✓ Add"`, `add_text_reject() = "✕ Cancel"` (distinct
  wording from `TextEdit`'s `accept_edit()`/`reject_edit()` and reflow's
  own `reflow_accept()`/`reflow_reject()`, per 15.2 §11's own reasoning:
  several of these could in principle be visible across a session and the
  operator should never have to guess which action a bare "Accept" commits)
- `add_text_empty_tooltip() = "Type some text first."`
- `add_text_default_font_label() = "Default font for new page text:"`
  (§5.1, Font-folders panel)
- One "what would lift it" hint entry per §6.2 row not already covered by
  14.3's existing `r_inv_*_hint()` functions (reused verbatim where the
  trigger id matches) — naming convention `add_text_invalid_size_hint()`,
  `add_text_hidden_objects_hint()`, etc.
- `edit_this_text_now_button() = "Edit this text now →"` (§7.2's P1
  continuity convenience)

No new wording for the disclosure BODIES (§6.1) — those render
`AddTextReport.disclosures` verbatim from core, exactly as every prior
tool-bearing spec's own §11/§8 already establishes for its own report type.

---

## 11. Undo / write-path summary

| Operation | `EditSession` command? | Writes anything? |
|---|---|---|
| Tool entry/exit, page nav while tool active | No | No — state rebuild only, no page-text read at all (unlike `TextEdit`'s model rebuild) |
| Click/drag placement (§3), manual X/Y/W/H entry (§3.3) | No | No — view-state only |
| Typing while composing (§4.1/§4.2) | No | No — `draft_text` only; box mode's live wrap preview is a PURE read-only call, no mutation |
| Accept (§7.1), successful | **Yes — ONE `CommandKind::AddText`** (already-shipped `EditSession::add_text`, §0.2) | Nothing until real Save (incremental, R34/R36) |
| Accept, refused | No | No — `draft` stays, unchanged, for revision |
| Reject / Esc (§7.3) | No | No — `draft` discarded |
| Font-folders "default font" preference change (§5.1) | No | No — a session (or persisted-config) preference, not a document edit |

Exactly one new class of undo-able command is EXERCISED by this Pass
(`CommandKind::AddText` already exists, unlike 14.3/15.2 which had to wait
for their own core addition to land first) — every other row is view-state,
matching every prior tool-bearing spec's own write-path table line for
line.

---

## 12. Priority table

| Item | Priority | Note |
|---|---|---|
| `CanvasTool::AddText` variant, toolbar entry, tool-active wiring (§0.1, §1) | **P0** | |
| `edit_text_tool_tooltip()`/`text_menu_tooltip()` copy updates (§1.1, §10) | **P0** | Cheap, closes the exact naming-collision risk this Pass's own existence creates elsewhere in the toolbar |
| `AddTextState`/`AddTextDraft`, tool-entry/page-nav/post-accept lifecycle (§2) | **P0** | |
| Point-mode click→placement via `canvas_to_pdf_space` (§3.1) | **P0** | Fully buildable today — no core gap |
| Manual X/Y keyboard-entry placement (§3.3, point half) | **P0** | The concrete accessibility win; cheap given the property bar already exists |
| Point-mode composing + preview, reusing `PendingEdit`'s exact visual language (§4.1) | **P0** | |
| Font-folders "default new-text font" preference control (§5.1) | **P0** | Small addition to an existing panel |
| Property-bar Font/Size/Colour override (§5.2) | **P0** | |
| Disclosure strip rendering ALL THREE possible `AddTextReport.disclosures` entries (§6.1) | **P0** | The inheritance-trap disclosure is easy to miss if only skimming decision 016 §3.6 |
| Refusal table against the REAL `AddTextError`/`RInvTrigger` (§6.2) | **P0** | |
| Accept/Reject wiring to the ALREADY-SHIPPED `EditSession::add_text` (§7.1) | **P0** | No core prerequisite, unlike 14.3/15.2's own P0 core asks |
| Tool-stays-active-after-Accept continuity (§7.2) | **P0** | |
| `GestureInterrupt`/Escape one-more-disjunct wiring (§8) | **P0** | |
| Box-mode drag placement, rubber-band, degenerate-drag fallback (§3.2) | **P1 — BLOCKED on 16.1** | Fully specified; cannot ship until 16.1's mutating box API lands (§0.3) |
| Box-mode live wrap preview (§4.2) | **P1 — BLOCKED on 16.1** | Needs the pure wrap-preview accessor named in §0.3 |
| Manual W/H keyboard-entry (box half of §3.3) | **P1 — BLOCKED on 16.1** | Depends on box mode existing at all |
| `Std14::ALL` convenience array (§0.4) | **P2** | Cheap, low-risk (spec-frozen list), not blocking — GUI may hardcode the 14-arm list in the meantime |
| Live per-keystroke glyph-coverage highlight (§5.3) | **Explicitly deferred, named fast-follow** | Needs a new coverage-query accessor; baseline refuse-at-Accept (P0) is fully sufficient and consistent with 14.3's own precedent |
| "Edit this text now →" convenience + `AddTextReport` new-run identity (§7.2) | **P1** | Nice, not required — the P0 version (switch tool, let the operator click the visible new text) already satisfies the "no separate mode" requirement |

---

## 13. Open items for the librarian

1. **This Pass adds a SECOND `CanvasTool` variant** (`AddText`), the first
   time since Pass 14.3 introduced the enum's first variant — worth noting
   in the same taxonomy-history record Pass 12.0 §10, Pass 14.3 §14, and
   Pass 15.2 §13 already maintain, specifically because this Pass's own
   §0.1 makes a DELIBERATE, reasoned call in the OPPOSITE direction from
   15.2's (new tool, not a sub-mode) — a future reader comparing the two
   should see this was a considered choice per operation, not an
   inconsistent precedent.
2. **§0.2 is a genuine, notable GOOD-NEWS finding, the mirror image of
   14.3/15.2's own §0.2/§0.2 findings**: Pass 16.0 shipped BOTH the free
   function and the session-integrated `EditSession::add_text` sibling from
   day one, closing what would otherwise have been this Pass's #1 priority
   core-accessor ask. Worth recording as a positive precedent (the pattern
   14.3/15.2 established — "ship the free function AND the session sibling
   together, sharing one planner" — evidently reached 16.0's own author
   before this UI spec was even written) in whatever decision/ROADMAP entry
   next touches Pass 16.0's completion note.
3. **§0.3 — 16.1 (boxed/wrap add) has two concrete asks, named precisely so
   16.1 confirms rather than guesses**: (a) a mutating box-add API mirroring
   16.0's own free-function + session-sibling shape exactly; (b) a NEW pure,
   read-only wrap-preview function independent of any mutating call, needed
   for box-mode's live-typing feedback — this is genuinely new, not covered
   by 15.0's existing `ReflowEngine::preview` (which previews an EXISTING
   recognized block, not a literal, not-yet-committed string). Flag
   alongside 14.3 §0.2/15.2 §0.2's own precedent for this kind of ask.
4. **§0.4 — font enumeration for the property bar needed NO new core
   surface** (a presumption corrected by reading the shipped code directly):
   `classify_nonembedded` (already hoisted for 14.3) answers Bundled-vs-
   Supplied for each of the 14 fixed Standard-14 names exactly as `pdfce-
   cli`'s own `cmd_add_text` already does it. Only a small, optional,
   spec-frozen `Std14::ALL` convenience remains (P2, §12) — worth noting so
   a future reader does not go looking for a bigger font-registry gap that
   does not exist.
5. **§1.1/§10 — two EXISTING tooltip strings need updating**
   (`edit_text_tool_tooltip()`, `text_menu_tooltip()`) as a direct
   consequence of this Pass shipping a third, easily-conflated control —
   flag this as a required companion change, not an optional nicety, since
   R78's disambiguation requirement is bidirectional (the new control must
   name the old ones, AND the old ones should now be able to name the new
   one, closing the loop rather than leaving a stale one-way pointer).
6. **§7.2's "Edit this text now →" convenience is a named, non-blocking
   P1** gated on `AddTextReport` growing a new-run-identity field — worth
   tracking alongside 14.3 §14 item 5 (block split/merge/reorder) and 15.2
   §13 item 6 (diff-aware recognition-divergence) as another example of
   this project's "named fast-follow, not silently deferred" convention.
7. **§5.3's live glyph-coverage highlight** is a deliberately deferred
   enhancement needing a new coverage-query accessor that does not exist
   today — flag alongside the other named-not-blocking font-tooling
   fast-follows (14.3 §14 item 3's `font_subset_stem` hoist, now
   completed, is the precedent for "flag it, and it does eventually get
   done").
