# Pass 8 UI Spec — Redaction (Mark → Review → Apply)

> Authored by `pdfce-ui-specialist`, 2026-07-31, on dispatch from the
> engineer/orchestrator. This is the implementation spec for Pass 8's
> GUI surfaces; the Pass 8.0 engineer follows its P0 if present, else
> ships minimal + defers, per this project's own Pass 3.2/6.1 spec
> convention (deviations get named, not silent).
>
> Read: `crates/pdfce-gui/src/{main.rs, ui_text.rs, viewer.rs}` in full;
> `docs/ui_specs/pass-6.1-markup-tools.md` (the tool-mode state machine
> this spec's Mark phase reuses — **see the load-bearing dependency
> flag in §1.1 before assuming this infrastructure exists**);
> `docs/ARCHITECTURE.md` §5.2/§5.7/§5.8/§11.2 and standing rules
> R35/R38/R46/R48/R52; `docs/ROADMAP.md`'s Pass 8 entry (In progress);
> `D:\Dev\Rag-Specialized\Acrobat_Features\redaction__*.md` (all seven
> files, read in full — the mark/apply two-phase model, the content-
> removal scope GAP, the mark-appearance conventions, search-and-redact,
> sanitize's separate scope, the documented failure modes, and the
> permissions/signature interaction); `D:\Dev\Rag-Specialized\PDF_Spec\
> iso32000\iso32000__ref__redaction_removal.md` (the container-
> decomposition mechanics and the carrier-sweep checklist this spec's
> Apply report is a UI rendering of).
>
> **Correctness here IS security.** Every design choice below is
> pressure-tested against one question: *can this UI make an operator
> believe content is gone from the file when it is not?* Where the
> answer was ever "yes, if X," X is closed off explicitly, not left as
> a residual.

---

## 0. Scope decided in this spec — read this first

| Bucket | Contents | Ships this Pass? |
|---|---|---|
| **P0 — minimal, honest, CLI-first** | `pdfce-cli redact mark` / `redact apply` (core's own scope, referenced not designed here) fully functional; GUI ships ONLY the `tool_available_in_cli`-pattern placeholder in the Tools dock (§3.5) **plus** the mandatory, non-negotiable pending-marks status-bar disclosure (§3.4) — the latter ships **regardless of which GUI slice lands**, because it is cheap, and it is the direct fix for the single most-cited real-world redaction failure (a marked-but-never-applied file shared as if finished, per `redaction__limitations_and_failure_modes.md`) | **Yes, required, whichever of P0/P1 the engineer has time for** |
| **P1 — full GUI Mark/Review/Apply** | Canvas drag-rectangle marking (§2), whole-page marking (§2.4), the Tools-dock marks-review panel (§3), literal-text search-to-mark (§2.5), the Apply report modal (§4), Sanitize's own scan+report+apply (§6) | Ship if the two named prerequisites (§1.1's canvas tool-mode infra; a real `ApplyReport`-shaped data contract from `pdfce-core`, §7) are both available; otherwise ship P0 alone and carry this forward as a named follow-up, exactly as Pass 6.1 did for its own canvas state machine |
| **P2 — explicit follow-up, not this Pass** | PII pattern presets + custom regex for search-and-mark (§2.5); redaction codes/code sets (FOIA/Privacy Act presets, custom codes); overlay-text/auto-size/repeat appearance customization (§2.6); post-hoc editing of an already-placed mark (move/resize, not just delete); keyboard-driven navigation/deletion inside the marks-review list | **No** |

**If P1 does not make the Pass, ship P0 alone.** This is not a lesser
spec written to look complete — it is the deliberately conservative
call for the one operation in this entire application where a rushed
GUI is a worse outcome than no GUI. A CLI-only redaction feature with
an honest placeholder and a loud pending-marks warning is trustworthy.
A half-built canvas tool that silently mis-marks a region, or an Apply
button one click away from a signature-style 520px dialog that
undersells what it is about to destroy, is not — and this is the one
feature where "not trustworthy" is the actual security failure mode
this Pass exists to prevent.

---

## 1. Prerequisites and data model

### 1.1 Load-bearing dependency flag: the canvas tool-mode infrastructure may not exist yet

`docs/ui_specs/pass-6.1-markup-tools.md` designed a full canvas
tool-mode state machine — `active_tool: Option<MarkupTool>`,
`draw_state: DrawState`, canvas focusability (`Sense::click_and_drag()`,
`request_focus`), drag-vs-pan suppression, the transient property-bar
top panel — for Pass 6.1's own drag-to-draw shapes. **Per
`ROADMAP.md`'s Pass 6.1 Shipped entry, that full state machine was
NOT what shipped.** Pass 6.1's actual GUI is "a minimal menu affordance
that authors at a default page-centred rect" (`GuiMarkupKind`,
`add_markup_shape` in `main.rs`); the full drag/marquee/live-preview
canvas machine is recorded as a **named, unshipped GUI follow-up
slice**.

**Consequence for this Pass, stated plainly so it is not silently
assumed away:** the Mark phase's canvas drag-rectangle interaction
(§2.2) depends on exactly that same unshipped infrastructure. Before
building it, the Pass 8 engineer must check whether it has landed
(either as the Pass 6.1 follow-up, or by some other Pass). If it has
not:

- **Do not build a second, parallel, one-off drag-rectangle
  implementation just for redaction marks.** That would be the same
  mistake `pdfce-core` guards against with its single-writer-path
  discipline (§5.7), applied to the GUI: two independent drag-tool
  implementations is exactly how the two quietly diverge later.
- **Ship P0 instead** (§0, §3.5) — CLI-first, GUI placeholder, the
  mandatory pending-marks disclosure. This is not a workaround; it is
  the correctly-scoped answer to a real, named blocking dependency,
  exactly the shape of Pass 6.1's own P1-quad-point CLI-placeholder
  fallback (§3.6 of that spec) and Pass 3.2's Split/Insert
  CLI-placeholder precedent.
- If the canvas infra is built as PART of this Pass (absorbing the
  Pass 6.1 debt), that is a legitimate scope call for the engineer to
  make explicitly and name in the ship notes — but it is a bigger Pass
  than "just redaction," and should be sized accordingly, not
  discovered mid-Pass.

### 1.2 New GUI-side state (session-only, on `OpenDoc`, matching the
`rail_expanded`/`markup_color` precedent — none of this is persisted)

```rust
/// Which redaction-workflow surface is open. Distinct from `tools_open`/
/// `tools_selected` — see §3.1 for why this is its own toggle rather
/// than a fourth `Tool` dock entry.
redact_panel_open: bool,

/// The Tools-dock-style running review list, kept in sync with the
/// document's actual `/Redact` annotations (never a private cache that
/// could drift — rebuilt from `doc.session` on every refresh_pages(),
/// exactly like `pages`/`thumbnails` already are).
redaction_marks: Vec<RedactionMarkRow>,

/// A redaction apply the operator has been asked to confirm. `Some`
/// means the Apply Report modal (§4) is on screen and NOTHING has been
/// written — same before-not-after posture as `PendingSave`/
/// `PendingCopy`, with a sharper edge: unlike those two, this one has
/// no cheap undo once its FULL REWRITE actually lands on disk (§5).
pending_redaction_apply: Option<PendingDestructiveApply>,

/// A sanitize run the operator has been asked to confirm — see §6.
/// Deliberately a SEPARATE `Option`, never fused with
/// `pending_redaction_apply` into one enum: task rule #4 requires
/// sanitize to be a genuinely separate operation with its own
/// confirmation, and one shared `Option` would make "which of the two
/// is pending" a runtime question instead of a type-level one.
pending_sanitize_apply: Option<PendingDestructiveApply>,
```

```rust
/// One row in the marks-review list — a display projection of an
/// actual `/Redact` annotation in the document, never a second source
/// of truth. Rebuilt, not incrementally patched, on every
/// `refresh_pages()` — the same "re-derive, don't cache-and-drift"
/// discipline `selected_pages`/`thumbnails` already follow.
struct RedactionMarkRow {
    /// Stable identity for the ✕ (remove) button and jump-to-mark click.
    /// The engineer's/core's call what this actually is (object number,
    /// an index into the page's /Annots — not dictated here).
    id: RedactionMarkId,
    page_index: usize,
    /// How the mark was authored — shown in the row so an operator can
    /// tell "I drew this" from "this came from a search hit" at a
    /// glance, per the fuzzy-never-sneaky principle applied to the
    /// mark's OWN provenance, not just to algorithmic content.
    source: RedactionMarkSource, // Manual | WholePage | SearchHit(String)
}

enum RedactionMarkSource { Manual, WholePage, SearchHit(String) }
```

```rust
/// Shared shape for BOTH the redaction-apply and the sanitize-apply
/// confirmations (§4, §6) — ONE new confirmation-dialog convention,
/// not two. See §4.1 for why this needed a THIRD dialog convention
/// (distinct from the existing 520px-centered-window style) rather
/// than reusing `signature_confirmation`'s shape verbatim.
struct PendingDestructiveApply {
    /// What will be removed — the report's affirmative section (§4.3).
    report: DestructiveApplyReport,
    /// Whether the operator has checked the mandatory acknowledgement
    /// checkbox yet (§4.5) — gates the Apply button itself.
    acknowledged: bool,
}
```

The exact shape of `DestructiveApplyReport` depends on what
`pdfce-core` actually returns (§7) — this spec fixes what the UI must
be able to *render* from it, not the Rust type. Do not let the UI
design wait on a perfect core API; a `Vec<String>`-shaped stand-in that
satisfies §4.3's rendering contract is an acceptable P1 cut, same
spirit as Pass 6.1 leaving `MarkupTool`'s exact core wiring to the
engineer.

---

## 2. MARK phase — reversible, safe, frictionless

### 2.1 The one governing rule

**Nothing in this phase requires a confirmation.** Per R52 and the
Acrobat-parity RAG's own corroboration, marking is non-destructive and
fully reversible right up until Apply — rule 7's "no unnecessary
friction for reversible, low-stakes actions" applies at full force
here, mirroring exactly how Pass 6.1's Square/Circle/Line tools commit
with zero confirmation. The ONLY place friction belongs in this entire
feature is Apply (§4).

### 2.2 Canvas drag-rectangle marking — reuses Pass 6.1's tool-mode
machinery verbatim (subject to §1.1)

Add `Redact` as a variant of the SAME `MarkupTool` enum
`pass-6.1-markup-tools.md` §1.2 defines (do not invent a parallel tool
enum):

```rust
enum MarkupTool {
    Ink, Square, Circle, Line, Polygon, PolyLine,
    Highlight, Underline, StrikeOut, Squiggly,
    Redact, // new
}
```

Interaction is IDENTICAL to Square (`pass-6.1-markup-tools.md` §2.5):
drag anchor→current, live preview, degenerate-drag discards silently,
`drag_stopped()` commits one `Action::CommitRedactionMark { rect }`.
Reuses `screen_to_page`/`page_to_screen`, the crosshair cursor, and the
drag-vs-pan suppression — nothing new to design here, only a new enum
arm and a new commit target.

**Entry point:** from the Tools-dock Redact panel (§3.2), NOT a
toolbar button — see §3.1 for the placement reasoning. Clicking "Draw
a mark…" issues `Action::SelectMarkupTool(Some(MarkupTool::Redact))`,
exactly like any other tool selection; the transient property bar
(§2.6) appears; the dock panel stays open so the operator can watch
the review list grow as they draw multiple marks in one session.

### 2.3 The mark's on-canvas appearance — MUST NOT look like an applied redaction

This is the single most safety-critical rendering decision in this
spec, directly answering the task's "the visual language must clearly
distinguish marked (reversible) from applied (gone)" requirement.

**A pre-apply mark is never a solid black fill.** Per
`redaction__mark_appearance_and_overlay_text.md`, Acrobat itself treats
the pre-apply mark's own appearance and the post-apply redacted-area
fill as two SEPARATE settings — pdfce follows the same split, but goes
further and makes the two visually *incompatible* rather than merely
independently configurable, so an operator cannot mistake one for the
other even glancing quickly:

- **Pre-apply (`/Redact` annotation's own `/AP`, generated by the SAME
  `annot_author`/`vartext` pipeline Pass 6.1/6.2 built — R44's "no
  private render path" applies here too):** a translucent
  **diagonal-hatch pattern in warning red**, a solid red outline, and a
  small corner label reading **"MARKED"** in a bordered tag — never a
  filled block of any color. The hatch pattern is load-bearing for
  rule 6 (color is never the sole signal): a red-tinted but otherwise
  solid rectangle could, at a glance or in a grayscale screenshot, read
  as "this is already redacted" — a hatch pattern reads as
  "provisional" the way construction-zone markings do, and cannot be
  confused with a filled shape at any zoom level.
- **Post-apply (baked directly into page content, §4/§5 — the
  annotation is deleted per spec, this is no longer an annotation at
  all):** the actual configured redacted-area fill (default solid
  black, or an operator-chosen `IC` color, §2.6) — indistinguishable
  from any other black rectangle that was always part of the page,
  which is *correct*: post-apply there is no "mark" left to visually
  distinguish, only content.

**The two states are therefore never rendered by the same code path
and never share a visual vocabulary** — the hatch+outline+tag is
categorically different from a solid fill, not merely a lighter shade
of the same thing.

### 2.4 Whole-page marking

A second dock button, "Mark whole page" (`Action::MarkWholePage`) —
commits one redaction mark spanning the current page's full
`MediaBox`, no drag required. Per the Acrobat RAG, this is a
`must_have` distinct primitive precisely so an operator does not have
to hand-draw a rectangle over an entire page when the intent is "redact
this whole page." No confirmation (§2.1); an `edit_note` line
discloses it happened ("Marked the whole of page {n} for redaction.
Nothing has been removed yet.") because marking a whole page in one
click is easy to do without noticing, and the narrator-surface
principle (rule 4) says that fact belongs on screen even without a
gate.

### 2.5 Mark-by-search (P1: literal text only; P2: patterns/regex)

A search field in the dock panel (§3.2), reusing Pass 4's text-
extraction substrate exactly as the Acrobat RAG recommends ("search
reuses extraction, doesn't reinvent it"):

```
ui.horizontal(|ui| {
    ui.text_edit_singleline(&mut self.redact_search_query);
    if ui.button(redact_search_button()).clicked() {
        actions.push(Action::SearchAndMark(self.redact_search_query.clone()));
    }
});
ui.label(redact_search_hint());  // "Finds this exact text across every
                                  // page and adds a mark over each
                                  // match. This only works on text
                                  // pdfce can extract — a scanned page
                                  // with no text layer will find
                                  // nothing here, which is NOT the same
                                  // as 'nothing sensitive is there.'"
```

The hint text's OCR caveat is mandatory, not decorative — per
`redaction__limitations_and_failure_modes.md`'s "(c) OCR/scanned-
document dependency," a silent zero-match result on a scanned page is
a real, named failure mode (an operator reads "0 matches" as "nothing
sensitive here" rather than "nothing searchable here"). This is
exactly the same shape of disclosure `copy_page_text_tooltip` and
`copy_text_no_extractable_text` already carry for Pass 4's own
extraction feature — reuse that established wording pattern, don't
invent a new one.

**Result handling:** every match becomes one ordinary redaction mark,
added to the SAME review list (§3.2) with `source:
RedactionMarkSource::SearchHit(query)` so each is visually tagged in
the list and individually removable before Apply — per the RAG's "a
search-and-redact pass presents a reviewable batch of proposed marks,
not a silent auto-apply," which is simply R52 applied to a bulk-authored
mark instead of a hand-drawn one. Zero matches gets its own
`edit_note` ("No matches for “{query}”. If this document is a scan
with no text layer, this search cannot find anything on it — mark the
region manually instead."), never a silent no-op.

**P2, not this Pass:** named PII pattern presets (phone/email/SSN/
credit-card), custom regex, and — per the Acrobat RAG's own documented
gap — pdfce's custom-pattern matching should honor case-sensitivity
when the pattern requests it rather than silently forcing case-
insensitive matching the way Acrobat's own regex path does (a
deliberate "match or exceed," flagged exactly as the RAG itself
flags it, for whenever P2 is scoped).

### 2.6 The transient property bar — mark-time appearance controls

Same placement as Pass 6.1 §4.1's transient top panel (the sixth
taxonomy instance, shown only while `active_tool == Some(Redact)`):

```
ui.horizontal(|ui| {
    ui.label(redact_property_bar_label());  // "Redaction mark:"
    color_edit_button_srgba(ui, &mut self.redact_mark_color)
        .on_hover_text(redact_mark_color_tooltip());
        // "Color of the MARK indicator only — this is discarded when
        // you Apply. Set the color of the final redacted area below,
        // in the Apply report."
    ui.separator();
    ui.label(redact_hint_idle()); // "Drag over the content to mark it
                                  // for redaction. This does not
                                  // remove anything yet."
});
```

Note the tooltip's explicit split between the mark-indicator color
(cosmetic, discarded at apply) and the final redacted-area fill color
(configured at Apply time, §4.6) — directly mirroring the Acrobat RAG's
documented convention, and preventing an operator from thinking "I
picked blue for my mark, so the final redaction will be blue."

**P2:** overlay text / auto-size / repeat / redaction codes on the mark
itself — Acrobat's richer per-mark appearance surface
(`redaction__mark_appearance_and_overlay_text.md`). None of this
affects the removal guarantee; it is pure appearance-stream content and
can be added later without touching anything load-bearing in this
spec.

### 2.7 Undo / save behavior

Every commit (`CommitRedactionMark`, `MarkWholePage`, each mark from
`SearchAndMark`) is **one `EditSession` command per mark**, undoable
like any other annotation authoring (R47 lineage) — no new undo
mechanism needed. Marks are **saveable un-applied**: an ordinary
incremental Save persists them exactly like any other annotation,
because pre-apply a `/Redact` annotation is just an annotation — no
special-casing in the save path. See §3.4 for the mandatory disclosure
this triggers.

---

## 3. Marks-review surface

### 3.1 Placement — a new Redact panel, distinct from the Tools dock, and why

The five-way placement taxonomy (`ARCHITECTURE.md` §12
continuation-23): *view-state → toolbar view group; edit → toolbar/
window; selection-scoped → rail; advanced → Tools dock; disclosure →
status bar.* None of these cleanly fits "a live-updating list of
destructive-pending marks on the OPEN document, needing repeated review
across a whole editing session before a single batch commit."

**The Tools dock is the wrong home, and the mismatch is not
cosmetic.** `tools_dock_intro()` states, verbatim, "These tools work
with files outside the one you have open" — the exact sentence Pass
4's copy-text feature was kept OUT of the dock to avoid falsifying
(`ui_text.rs`'s own comment on `copy_text_button`'s placement). A
redaction mark acts directly on the open document's own content —
falsifying that sentence far more than copy-text (which touches
nothing) ever would have. Do not put Redact in the Tools dock list
alongside Merge/Split/Insert.

**Decision: a new toggle, `redact_panel_open`, opening its own
`egui::SidePanel::right`**, entered via ONE new toolbar control — a
single icon-only button (`🛡` shield glyph, matching `ICON_BUTTON_SIZE`),
placed as its own ungrouped control next to the Tools toggle, mirroring
exactly how Pass 4 gave copy-text its own ungrouped toolbar button
rather than forcing it into an existing group ("it belongs to neither
the view group... nor the edit group... and forcing it into either
would make that group's own organizing question unanswerable at a
glance" — the identical reasoning applies here: Redact belongs to
neither the Tools dock's "external files" bucket nor the Markup group's
"non-destructive annotation" bucket).

**Rule-3 tension, addressed directly:** rule 3 names redaction as an
example of what should stay OFF the primary toolbar. Giving it one
icon-only button is the minimum weight that satisfies BOTH rules at
once: rule 3's progressive-disclosure intent (don't add a labeled menu
group cluttering the primary row) and rule 7's "discoverable
destructive actions" (a security-critical feature that is too well
hidden fails its own purpose — an operator who cannot find how to
redact will improvise with the Highlight tool, which is exactly the
overlay-only false-redaction failure mode named in
`redaction__limitations_and_failure_modes.md`). One small icon-only
button, tooltip-labeled plainly, is the resolution: present but not
loud.

**This is a new placement pattern — the taxonomy's sixth-and-seventh
instances are both now on record** (Pass 6.1's transient tool-scoped
top panel was the sixth; this "dedicated secondary panel for a
document-internal, multi-step, review-then-commit destructive
workflow, distinct from the Tools dock's external-files framing" is the
seventh). Flag both to the librarian in one pass — see §11.

**Panel-add order** (extends the existing table; the redact panel
follows the same right-side-panel slot discipline as the Tools dock,
and the two may coexist — an operator with both open simply gets less
canvas width, same as rail+dock today):

```
1. toolbar                (top)
2. markup property bar    (top, second — shown while active_tool.is_some(),
                            unchanged from Pass 6.1, now also covers Redact)
3. status                 (bottom)
4. thumbnail rail          (left)   — if rail_expanded
5. Tools dock              (right)  — if tools_open
6. Redact panel            (right)  — if redact_panel_open (added AFTER
                            the Tools dock in the panel-add order, so if
                            both are open the Redact panel sits closer
                            to the canvas — it is the more actively-used
                            surface during an actual redaction session)
7. CentralPanel            (canvas)
8. properties_window(ctx, …)   — last, floating-over-everything, unchanged
```

### 3.2 Panel content

```
egui::SidePanel::right("redact_panel").show(ctx, |ui| {
    ui.heading(redact_panel_title());          // "Redaction"
    ui.label(redact_panel_intro());
        // "Mark content, then apply to permanently remove it. Marking
        // is reversible; applying is not."
    ui.separator();

    // -- authoring entry points --
    if ui.button(redact_draw_mark_button())      // "Draw a mark…"
        .on_hover_text(redact_draw_mark_tooltip())
            // "Drag over the content you want to redact (Alt+R). This
            // only marks it — nothing is removed until you Apply."
        .clicked()
    { actions.push(Action::SelectMarkupTool(Some(MarkupTool::Redact))); }

    if ui.button(redact_mark_whole_page_button()) // "Mark whole page"
        .on_hover_text(redact_mark_whole_page_tooltip())
        .clicked()
    { actions.push(Action::MarkWholePage); }

    ui.separator();
    ui.horizontal(|ui| {
        ui.text_edit_singleline(&mut self.redact_search_query);
        if ui.button(redact_search_button()).clicked() {  // "Find & mark…"
            actions.push(Action::SearchAndMark(self.redact_search_query.clone()));
        }
    });
    ui.label(redact_search_hint());

    ui.separator();

    // -- the review list itself --
    ui.label(redact_marks_count_label(doc.redaction_marks.len()));
        // 0  -> "No redaction marks yet."
        // N>0 -> "{N} pending redaction mark(s) — nothing removed yet."
    egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
        for row in &doc.redaction_marks {
            ui.horizontal(|ui| {
                if ui.selectable_label(false, redact_mark_row_label(row)).clicked() {
                    actions.push(Action::GoToPage(row.page_index));
                }
                if ui.add_sized(ICON_BUTTON_SIZE, egui::Button::new("✕"))
                    .on_hover_text(redact_mark_remove_tooltip())
                    .clicked()
                { actions.push(Action::RemoveRedactionMark(row.id)); }
            });
        }
    });

    ui.separator();
    let can_apply = !doc.redaction_marks.is_empty();
    ui.add_enabled_ui(can_apply, |ui| {
        if ui.button(redact_review_apply_button())   // "Review & Apply Redactions…"
            .on_hover_text(redact_review_apply_tooltip(can_apply))
            .clicked()
        { actions.push(Action::BeginRedactionApply); }
    });
});
```

`redact_mark_row_label` renders the source tag inline —
`"Page {n} — drawn region"` / `"Page {n} — whole page"` / `"Page {n} —
search: “{query}”"` — so an operator scanning the list can distinguish
how each mark came to exist, per §1.2's `RedactionMarkSource` note.

### 3.3 Jump-to-mark

Clicking a row's label (not its ✕) issues `Action::GoToPage`, reusing
the existing page-navigation action verbatim — no new navigation
mechanism needed. This is the accessibility-relevant path for reviewing
a mark's placement without relying on visually scanning the canvas.

### 3.4 The mandatory pending-marks disclosure — ships even if nothing
else in this spec does

**This is the one item in this entire spec with no priority tier below
P0.** Computed from the document's actual `/Redact` annotation census
(not a session counter — a document opened with marks already
authored, by the CLI or a prior GUI session, must trigger this too),
refreshed alongside `refresh_pages()`:

- Status bar, always visible whenever the count is nonzero, never
  dismissible, colour-paired with a glyph (rule 6):
  `⚠ {N} page(s) carry {M} unapplied redaction mark(s) — the marked
  content has NOT been removed. Open 🛡 to review and apply.`
- On a successful Save while marks are pending, an `edit_note` fires
  in addition to the ordinary save confirmation (never replacing it):
  `This save keeps {M} pending redaction mark(s) in the file — the
  marked content is still there. Nothing is removed until you Apply.`

This is the direct, structural answer to
`redaction__limitations_and_failure_modes.md`'s single most-cited
real-world failure ("Missed-Apply / Phase-1-only file shared as if
finished"). It costs one annotation-count query and one status-bar
line, and it is owed regardless of how much of the rest of this Pass
ships.

### 3.5 P0 fallback — the CLI-placeholder pattern

If §1.1's canvas infra is not ready, the panel above does not ship.
Instead, the toolbar's `🛡` button opens a minimal panel reusing
`ui_text::tool_available_in_cli` **verbatim** (same function, same
doc-comment reasoning — "a placeholder that says 'coming soon' wastes
the operator's time; one that hands them a working command does not"):

```
egui::SidePanel::right("redact_panel").show(ctx, |ui| {
    ui.heading(redact_panel_title());
    ui.label(tool_available_in_cli(
        "pdfce-cli redact mark <file.pdf> --region <page:x,y,w,h> -o <marked.pdf>\n\
         pdfce-cli redact apply <marked.pdf> -o <redacted.pdf> --confirm"
    ));
});
```

The §3.4 disclosure ships unconditionally, in EITHER case — it lives in
the status bar, not inside this panel, and does not depend on which
version of the panel is showing.

---

## 4. APPLY phase — the ONE heavy, non-skippable confirmation

### 4.1 A new, third confirmation-dialog convention — deliberately, not
by accident

pdfce currently has two confirmation styles: the 520px-centered-window
convention (`signature_confirmation`, `copy_confirmation` — fixed
width, non-resizable, short body text) and no confirmation at all
(every reversible edit). **Neither fits Apply.** The report (§4.3) is
the centerpiece the task explicitly demands, and its length genuinely
varies with document size and mark count — cramming it into a fixed
520px box would bury the very thing rule 2 says must be prominent, and
that is a load-bearing reason to deviate, not a style preference.

**Decision:** a THIRD convention — `egui::Window`, **`.resizable(true)`**
(the one deliberate deviation from the existing two dialogs'
`.resizable(false)`), a larger default size (760×560 or similar),
scrollable body, still `.collapsible(false)` and still centre-anchored
on first open. Shared by BOTH the redaction-apply and the sanitize-
apply confirmations (§6) — one new pattern class, not two, and not
three-going-on-four. Flag this to the librarian exactly as Pass 6.1
flagged its own property-bar addition (§11) — a decision-log-worthy
new dialog convention, recorded once, reused twice.

### 4.2 Trigger and the pending-gate extension

`Action::BeginRedactionApply` (from §3.2's "Review & Apply…" button)
computes the report (§7's data contract) and sets
`pending_redaction_apply = Some(...)`. **The existing pending-
confirmation gate at the top of `apply()` must be extended, not
duplicated**, to include this third `Option` alongside `pending_save`/
`pending_copy` — the exact same collision this project already found
once (Pass 4's `pending_copy` vs `pending_save`, documented in
`apply()`'s own doc comment) will recur with a third independent
pending state if the gate is not updated. This is not optional
bookkeeping; it is the mechanism that keeps two blocking dialogs from
silently rendering on top of each other with only the later one
clickable — exactly the bug class the existing doc comment already
warns about.

### 4.3 The report — the centerpiece, not fine print

Rendered as the modal's main body, always shown, always scrolled to
the top on open:

```
ui.heading(redact_apply_report_heading());
    // "What Apply will do to this document"

ui.label(redact_apply_permanence_statement());
    // "Applying is permanent. It rewrites the ENTIRE file and cannot
    // be undone once you save this — even Undo cannot bring it back
    // after that, because there is no data left in the file to
    // restore." (This is the architecture's own §11.2 language,
    // deliberately unsoftened — see §5's wording contract.)

ui.separator();
ui.strong(redact_apply_will_remove_heading());  // "Will be permanently removed:"
egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
    for line in &report.will_remove {   // per-page, per-carrier lines —
        ui.label(line);                 // see §7 for what these lines
    }                                    // are built from
});

if !report.could_not_remove.is_empty() {
    ui.separator();
    ui.colored_label(ui.visuals().warn_fg_color, redact_apply_refused_heading());
        // "⚠ pdfce could NOT remove the following — read this before continuing:"
    for line in &report.could_not_remove {
        ui.colored_label(ui.visuals().warn_fg_color, line);
    }
    // §4.5's mandatory extra checkbox appears ONLY in this branch.
}

ui.separator();
ui.label(redact_apply_scope_reminder());
    // "This does not touch document metadata, embedded files, hidden
    // layers, or bookmarks — those aren't tied to any mark. Use
    // Sanitize for those (see below)."
if ui.button(redact_apply_open_sanitize_button()).clicked() {
    actions.push(Action::OpenSanitizePanel);
}
```

### 4.4 The refusal contract — the part that makes or breaks the whole
feature's honesty

If `report.could_not_remove` is non-empty, the Apply button is
**disabled by default**, and enabling it requires an EXTRA
acknowledgement distinct from the ordinary confirmation checkbox
(§4.5):

```
if !report.could_not_remove.is_empty() {
    ui.checkbox(&mut pending.acknowledged_refusals,
        redact_apply_refusal_acknowledgement_checkbox());
        // "I understand the item(s) above will NOT be removed, and I
        // still want to apply the redactions that CAN be completed."
}
```

**This is the single most important gate in this spec.** Per task
requirement #3, the UI must never let a partial redaction be mistaken
for a complete one. A refused carrier (an image format pdfce cannot
re-encode, an XFA parallel copy it only detects, an embedded file under
the mark) is not a fine-print residual — it is a per-item, explicitly
acknowledged fact the operator affirmatively accepts BEFORE the button
that commits anything becomes clickable. There is no path in this
design where Apply proceeds silently past a refusal.

### 4.5 The confirmation itself

```
ui.separator();
ui.checkbox(&mut pending.acknowledged,
    redact_apply_confirm_checkbox());
    // "I understand this permanently removes the underlying content,
    // not just the visible marks."

ui.horizontal(|ui| {
    if ui.button(redact_apply_cancel_button()).clicked() {
        actions.push(Action::CancelRedactionApply);
    }
    let ready = pending.acknowledged
        && (report.could_not_remove.is_empty() || pending.acknowledged_refusals);
    ui.add_enabled_ui(ready, |ui| {
        if ui.button(redact_apply_confirm_button()).clicked() {
            // "Permanently Remove & Save As…" — the label IS the
            // consequence. Never "OK", never "Yes", never "Apply"
            // alone.
            actions.push(Action::ConfirmRedactionApply);
        }
    });
});
```

**No `Enter`-triggers-default-button behavior on this window.** Verify
in implementation that egui/eframe does not bind `Enter` to whichever
button happened to gain focus first — an operator reading a long
report and pressing Enter out of habit must never accidentally commit
the single most destructive action in the application. This is a
concrete implementation check, not a design nicety; flag it exactly as
Pass 6.1 §6.2 flagged its own "verify Alt+letter isn't intercepted"
implementation risk.

**No keyboard shortcut opens or confirms Apply, anywhere in the app.**
This is a deliberate asymmetry with every other destructive action in
pdfce (Delete has the Delete key, rotate has `[`/`]`) — those are
reversible pre-save; this is not, once saved. Removing the "accidental
keystroke" vector entirely is the correct application of rule 7's
"discoverable destructive actions" read in its stricter direction: the
heaviest action in the app gets zero frictionless paths, not even a
chord, while still being reachable via one clearly-labeled click chain
(`🛡` → "Review & Apply…" → the modal).

### 4.6 Confirm → forced full rewrite, forced save-as, never through the
existing Save path

`Action::ConfirmRedactionApply` calls a NEW method, `begin_redaction_
apply()` — **not** a parameter bolted onto the existing `save_dialog()`
— for exactly the reason `save_dialog()`'s own doc comments make
load-bearing elsewhere in this codebase: conflating two save paths is
how one silently inherits the other's defaults. Concretely:

- Opens the SAME native save dialog (`rfd::FileDialog::save_file`) as
  `save_dialog`, but pre-filled via a NEW `ui_text::suggested_
  redaction_name(path)` — `"{stem} (redacted).pdf"`, matching the
  existing `suggested_save_name`/`suggested_merge_name`/`suggested_
  extract_name` family AND the Acrobat RAG's own documented
  `_Redacted.pdf` convention. **Never** pre-filled with the original
  file's own name.
- Calls whatever `pdfce-core` API performs the apply, requesting
  `SaveMode::Full` **unconditionally** — never `Incremental`, per R35,
  and never operator-selectable. There is no toggle in this dialog
  that could make Apply write incrementally; that would defeat the
  entire operation's purpose (§5.2, §5.7).
- On success: `edit_note` gets the post-apply summary — see §5.
- On failure (a mid-apply refusal the core layer surfaces, e.g. a
  certification gate): routes through the existing `save_result =
  Some(SaveOutcome::Failed(...))` channel, same pattern every other
  refusal in this codebase already uses — no new failure-presentation
  surface needed.

---

## 5. The never-claim-what-isn't-true contract

### 5.1 Wording rules, stated as a checklist the engineer can verify
against

- **Never say "removed" without qualification if any carrier was
  refused.** The post-apply summary line MUST name the residual count
  whenever `could_not_remove` was non-empty at confirm time — never a
  bare "Redaction complete."
- **Never say "verified" unless a real verification step ran.** If
  `pdfce-core`'s apply path re-parses the saved output and greps for
  absence of the removed bytes (the same self-check
  `iso32000__ref__redaction_removal.md` recommends and the Acrobat RAG's
  own parity notes demand as a TEST-SUITE discipline), the summary may
  say "verified removed from the saved file." If that self-check does
  not exist yet, the summary says only "{N} region(s) redacted" — never
  borrow the stronger word for a claim the code did not actually check.
  This is a wording contract tied directly to whatever core capability
  actually ships; flag to the engineer that the exact word choice here
  must be revisited the moment the core-side verification step lands
  or is deferred.
- **Never let "Undo" appear anywhere near a post-apply state.** Unlike
  every other `edit_note` in this codebase ("Use Undo to reverse this
  until you save"), the post-apply summary must say the opposite
  explicitly, because an operator's learned expectation from every
  OTHER feature in this app is that undo is available pre-save — this
  is the one moment that expectation is actively wrong and must be
  corrected on screen, not left to be assumed.

### 5.2 Post-apply status-bar summary (durable, per the `edit_note`/
`save_result` "persists until superseded" precedent)

```
// Clean case (no refusals):
"✔ Redacted and saved to {file}. {N} region(s) across {M} page(s)
removed. This save cannot be undone — the original is unaffected if you
kept a copy."

// Residual case (refusals were acknowledged and proceeded):
"⚠ Redacted and saved to {file}. {N} region(s) removed; {K} region(s)
could NOT be removed and are still in this file — see the report from
your last Apply for what and why. This save cannot be undone."
```

The residual-case line is **warn-colored AND carries the ⚠ glyph**
(rule 6) and is never shortened or omitted — an operator who
acknowledged a refusal during Apply and then closes the app is still
owed a durable record of exactly what remains un-redacted, because a
dismissed modal is forgotten (the same reasoning
`save_signature_invalidated_note`'s own doc comment already states for
signature loss).

---

## 6. Sanitize / Remove Hidden Information — a genuinely separate feature

### 6.1 Why this needs the SAME destructiveness discipline as redaction,
not a lighter one

Per `redaction__sanitize_remove_hidden_information.md`, sanitize's
entire purpose is removing document-wide traces (metadata history,
prior-save artifacts, orphaned objects, cached form data) that would
otherwise be **recoverable**. If pdfce's own sanitize only performed an
ordinary incremental save, the "removed" metadata would still be
trivially recoverable in the file's own prior revision — defeating the
feature's entire reason to exist, in exactly the way §5.2/R35 already
established for redaction. **Sanitize is therefore held to the SAME
forced-full-rewrite, forced-save-as, no-incremental-option discipline
as redaction** (R35's "any operation whose contract is removal" reads
on sanitize as directly as it reads on redaction) — this is a genuine
finding of this spec, not an assumption carried over from Acrobat,
which does not document its own internal save mechanism at this level
of detail.

### 6.2 Placement and entry point

A **separate** Tools-dock entry, `Tool::Sanitize`, alongside
Merge/Split/Insert — this one DOES belong in the dock, unlike Redact
(§3.1): sanitize's scope is explicitly document-wide-and-mark-
independent, closer in spirit to Merge/Split's "act on the document as
a whole unit" shape than to a per-region marking workflow, and its
report is a one-shot scan-then-list-then-commit interaction, not a
running review surface revisited many times per session the way the
Redact panel is. No new placement pattern needed here.

**Never auto-run on redaction Apply** (task requirement #4, explicit):
the Apply report (§4.3) offers a button that opens the Sanitize panel
(`Action::OpenSanitizePanel`) — a suggestion, never an automatic
invocation. Per the sanitize RAG's own documented risk ("sanitize can
remove content an operator wants to keep," specifically bookmarks),
auto-running it would also violate rule 1 (operator confirmation
required) in the most direct possible way.

### 6.3 Panel content

```
ui.heading(sanitize_panel_title());   // "Sanitize document"
ui.label(sanitize_panel_intro());
    // "Scans for hidden, document-wide data that has nothing to do
    // with any redaction mark: metadata, embedded files, scripts,
    // hidden layers, bookmarks, cached form data, comments. This is a
    // separate step and never runs automatically."

if ui.button(sanitize_scan_button()).clicked() {   // "Scan…"
    actions.push(Action::ScanForHiddenData);
}

// After a scan, one checkbox row per found category:
for category in &scan.categories {
    ui.horizontal(|ui| {
        ui.checkbox(&mut category.selected, sanitize_category_label(category));
            // "Document metadata (Title, Author, 2 custom propert(y/ies))"
            // "1 embedded file (invoice.xlsx)"
            // "3 bookmark(s)"
        if category.kind == SanitizeCategory::Bookmarks {
            ui.colored_label(ui.visuals().warn_fg_color,
                sanitize_bookmarks_keep_warning());
                // "⚠ You may want to keep these for navigation."
        }
    });
}
```

**Default selection state, a deliberate call:** every category defaults
CHECKED except Bookmarks, which defaults UNCHECKED with the inline
warning shown above — directly encoding the one named real-world
friction point the Acrobat RAG documents (operators who enable sanitize
and lose navigation bookmarks they wanted). This is a small,
deliberate parity-plus over Acrobat's own undifferentiated default.

### 6.4 Sanitize's own Apply confirmation — reuses §4's modal shape

Clicking "Remove selected…" builds a `DestructiveApplyReport` (same
shape as redaction's, §1.2) listing exactly what will be removed per
selected category, and opens the SAME modal component as §4
(`pending_sanitize_apply`, not `pending_redaction_apply` — a distinct
`Option`, same rendering code path). The permanence statement, the
checkbox-gated confirm button, the forced-full-rewrite-forced-save-as
mechanics (§4.6, with `suggested_sanitize_name` — `"{stem}
(sanitized).pdf"`) and the no-keyboard-shortcut rule all apply
identically. There is no "could not remove" branch analog required
here unless a specific category turns out to be only partially
scrubbable (e.g. XFA detect-only, per Pass 7's existing posture) — if
so, it uses the exact same §4.4 refusal-acknowledgement pattern.

---

## 7. Data contract needed from `pdfce-core` (informational — the
engineer's/`pdfce-spec-librarian`'s call on the exact Rust shape)

The Apply modal (§4.3) needs, at minimum, enough structure to render:

- `will_remove: Vec<String>` — one line per page/carrier combination
  that WILL be removed (e.g. "Page 3: 2 text region(s), 1 image
  region(s)"), grounded in the carrier-sweep checklist
  (`iso32000__ref__redaction_removal.md` §6) — content-stream text,
  image/inline XObjects, object-stream containers being decomposed,
  intersecting annotations, and (if a sweep of document-wide carriers
  was also requested) `/Info` strings, XMP mentions, `/ActualText`
  duplicates.
- `could_not_remove: Vec<String>` — one line per refused item, each
  naming BOTH the location and the reason (e.g. "Page 5: an image using
  a codec pdfce cannot re-encode — this region will remain visible and
  present in the saved file"). This is the field §4.4's mandatory
  acknowledgement gate reads from; it existing and being populated
  honestly is the single load-bearing core-side requirement this
  entire UI design depends on.
- A boolean or count indicating whether a post-save self-verification
  (re-parse and grep for absence) actually ran, so §5.1's wording rule
  can pick the correct word ("verified removed" vs "redacted").

This is deliberately informational rather than prescriptive — same
posture Pass 6.1 §2.9 took toward its own `add_markup`-equivalent core
API ("the engineer's and `pdfce-spec-librarian`'s call, not dictated
here").

---

## 8. Keyboard map

| Chord | Action | Note |
|---|---|---|
| `Alt+R` | Enter the Redact mark tool | Joins the existing Pass 6.1 `Alt`+letter family (`I/S/C/L/P/O/H/U/K/Q` all taken; `R` free) |
| `Alt+Shift+R` | Mark whole page | No drag needed; kept off plain `Alt+R` so the common "draw a mark" chord cannot accidentally mark an entire page |
| `Esc` | Same two-stage cancel/exit convention as every other `MarkupTool` (§1.4 of `pass-6.1-markup-tools.md`, unchanged) | |
| *(none)* | Open the Apply modal | Deliberately no chord — §4.5 |
| *(none)* | Confirm Apply | Deliberately no chord, ever — §4.5 |
| *(none)* | Confirm Sanitize | Same reasoning as Apply — §6.4 |

**No keyboard path exists to remove a mark from the list (§3.2's ✕
button) this Pass.** Named as a P2 accessibility residual (§0), not
papered over: an operator using only the keyboard cannot currently
prune the marks list without a pointer. This is a real, tracked gap,
not a silent omission — flag it exactly as Pass 6.1 §7 named its own
freehand-drawing keyboard gap honestly rather than promising a fix that
does not exist yet.

---

## 9. Accessibility

- **Color never the sole signal:** the mark's hatch pattern + "MARKED"
  tag (§2.3) — not tint alone; the refusal section's ⚠ glyph (§4.3,
  §5.2) alongside warn-color; the pending-marks status line's ⚠ glyph
  (§3.4).
- **Click targets:** the row ✕ buttons use `ICON_BUTTON_SIZE`, matching
  every other icon-only control in this crate.
- **Tab order:** the Redact panel participates in the existing panel-
  order chain (§3.1's table) exactly like the Tools dock does today;
  the Apply modal's checkbox → cancel → confirm sequence is a normal
  tab chain, verified NOT to bind `Enter` to a default action (§4.5).
- **Screen-reader gap, named plainly:** the mark's hatch-pattern
  appearance is a rendered vector shape with no text alternative pdfce
  generates, exactly the same `accesskit` gap already recorded against
  Pass 3.2's drag-reorder, Pass 4's deferred text-selection, and Pass
  6.1's markup tools. This is a fourth occurrence of one gap, not a new
  one — link it in the same chain rather than filing separately.
- **Freehand/drag marking is inherently pointer-first**, same honest
  distinction Pass 6.1 §7 already drew for its own drawing tools (a
  real, non-fixable-via-egui limitation of drag gestures generally, not
  an accesskit shortfall) — restated here rather than silently assumed
  to also apply.

---

## 10. `ui_text.rs` catalog

New section header: `// Redaction (Pass 8)`. Every entry is one
complete templated message (R2); every tooltip on a destructive control
names the consequence, not just the action (discoverability
checklist).

**Toolbar:**
- `redact_panel_toggle_button() -> &'static str` — `"🛡"`.
- `redact_panel_toggle_tooltip() -> &'static str` — "Mark content for
  redaction, review pending marks, and apply — permanently removing
  what's marked."

**Redact panel:**
- `redact_panel_title() -> &'static str` — "Redaction".
- `redact_panel_intro() -> &'static str` — "Mark content, then apply to
  permanently remove it. Marking is reversible; applying is not."
- `redact_draw_mark_button() -> &'static str` — "Draw a mark…".
- `redact_draw_mark_tooltip() -> &'static str` — "Drag over the content
  you want to redact (Alt+R). This only marks it — nothing is removed
  until you Apply."
- `redact_mark_whole_page_button() -> &'static str` — "Mark whole page".
- `redact_mark_whole_page_tooltip() -> &'static str` — "Mark the entire
  current page for redaction (Alt+Shift+R). Nothing is removed until
  you Apply."
- `redact_search_button() -> &'static str` — "Find & mark…".
- `redact_search_hint() -> &'static str` — "Finds this exact text
  across every page and adds a mark over each match. This only works on
  text pdfce can extract — a scanned page with no text layer will find
  nothing here, which is not the same as “nothing sensitive is there.”"
- `redact_no_matches(query: &str) -> String` — "No matches for
  “{query}”. If this document is a scan with no text layer, this search
  cannot find anything on it — mark the region manually instead."
- `redact_marks_count_label(count: usize) -> String` — 0 → "No
  redaction marks yet." / N>0 → "{N} pending redaction mark(s) —
  nothing removed yet."
- `redact_mark_row_label(row: &RedactionMarkRow) -> String` — "Page
  {n} — drawn region" / "Page {n} — whole page" / "Page {n} — search:
  “{query}”".
- `redact_mark_remove_tooltip() -> &'static str` — "Remove this mark.
  It was never applied, so nothing in the document changes."
- `redact_review_apply_button() -> &'static str` — "Review & Apply
  Redactions…".
- `redact_review_apply_tooltip(can_apply: bool) -> String` — enabled:
  "Opens a report of exactly what will be permanently removed. Nothing
  is deleted until you confirm there." / disabled: "Mark at least one
  region first."
- `redact_whole_page_marked(page_number: usize) -> String` — the
  `edit_note` for §2.4: "Marked the whole of page {page_number} for
  redaction. Nothing has been removed yet."

**Property bar:**
- `redact_property_bar_label() -> &'static str` — "Redaction mark:".
- `redact_mark_color_tooltip() -> &'static str` — "Color of the mark
  indicator only — this is discarded when you Apply. Set the color of
  the final redacted area in the Apply report."
- `redact_hint_idle() -> &'static str` — "Drag over the content to mark
  it for redaction. This does not remove anything yet."

**Status-bar disclosure (§3.4, mandatory P0):**
- `redact_pending_marks_status(pages: usize, marks: usize) -> String` —
  "⚠ {pages} page(s) carry {marks} unapplied redaction mark(s) — the
  marked content has NOT been removed. Open 🛡 to review and apply."
- `redact_save_kept_pending_marks(count: usize) -> String` — "This save
  keeps {count} pending redaction mark(s) in the file — the marked
  content is still there. Nothing is removed until you Apply."

**Apply modal (the centerpiece, §4):**
- `redact_apply_report_heading() -> &'static str` — "What Apply will do
  to this document".
- `redact_apply_permanence_statement() -> &'static str` — "Applying is
  permanent. It rewrites the entire file and cannot be undone once you
  save this — even Undo cannot bring it back after that, because there
  is no data left in the file to restore."
- `redact_apply_will_remove_heading() -> &'static str` — "Will be
  permanently removed:".
- `redact_apply_refused_heading() -> &'static str` — "⚠ pdfce could NOT
  remove the following — read this before continuing:".
- `redact_apply_refusal_acknowledgement_checkbox() -> &'static str` —
  "I understand the item(s) above will NOT be removed, and I still
  want to apply the redactions that CAN be completed."
- `redact_apply_scope_reminder() -> &'static str` — "This does not
  touch document metadata, embedded files, hidden layers, or bookmarks
  — those aren't tied to any mark. Use Sanitize for those."
- `redact_apply_open_sanitize_button() -> &'static str` — "Open
  Sanitize…".
- `redact_apply_confirm_checkbox() -> &'static str` — "I understand
  this permanently removes the underlying content, not just the
  visible marks."
- `redact_apply_confirm_button() -> &'static str` — "Permanently Remove
  & Save As…".
- `redact_apply_cancel_button() -> &'static str` — "Don't apply yet".
- `suggested_redaction_name(path: &Path) -> String` — `"{stem}
  (redacted).pdf"`, same family as `suggested_save_name`.

**Post-apply disclosure (§5.2, durable):**
- `redact_apply_succeeded_clean(path: &Path, regions: usize, pages: usize)
  -> String` — "✔ Redacted and saved to {file}. {regions} region(s)
  across {pages} page(s) removed. This save cannot be undone — the
  original is unaffected if you kept a copy."
- `redact_apply_succeeded_residual(path: &Path, removed: usize, kept: usize)
  -> String` — "⚠ Redacted and saved to {file}. {removed} region(s)
  removed; {kept} region(s) could NOT be removed and are still in this
  file — see the report from your last Apply for what and why. This
  save cannot be undone."

**Sanitize (§6):**
- `sanitize_panel_title() -> &'static str` — "Sanitize document".
- `sanitize_panel_intro() -> &'static str` — "Scans for hidden,
  document-wide data that has nothing to do with any redaction mark:
  metadata, embedded files, scripts, hidden layers, bookmarks, cached
  form data, comments. This is a separate step and never runs
  automatically."
- `sanitize_scan_button() -> &'static str` — "Scan…".
- `sanitize_category_label(category: &SanitizeCategoryKind, count: usize,
  detail: &str) -> String` — one templated line per category, e.g.
  "Document metadata ({detail})" / "{count} embedded file(s) ({detail})"
  / "{count} bookmark(s)".
- `sanitize_bookmarks_keep_warning() -> &'static str` — "⚠ You may want
  to keep these for navigation.".
- `sanitize_remove_selected_button() -> &'static str` — "Remove
  selected…".
- `suggested_sanitize_name(path: &Path) -> String` — `"{stem}
  (sanitized).pdf"`.
- `sanitize_apply_succeeded(path: &Path, categories: usize) -> String`
  — "✔ Sanitized and saved to {file}. {categories} categor(y/ies) of
  hidden data removed. This save cannot be undone."

**P0 fallback (§3.5, reuses existing catalog):**
- No new entries — `tool_available_in_cli` is reused verbatim with a
  Pass-8-specific command string, exactly as specified.

---

## 11. Open items for the librarian

1. **Two new instances of the placement taxonomy**, both from this
   spec: (a) the Redact panel (§3.1) — "a dedicated secondary panel for
   a document-internal, multi-step, review-then-commit destructive
   workflow, distinct from the Tools dock's external-files framing" —
   the taxonomy's seventh recorded instance (Pass 6.1's transient
   tool-scoped top panel was the sixth); (b) the new resizable,
   larger-report confirmation-dialog convention (§4.1) — a third dialog
   style alongside the existing 520px-centered-window convention,
   deliberately shared by redaction-apply AND sanitize-apply rather
   than invented twice.
2. **The Pass 6.1 canvas tool-mode infrastructure's shipped-vs-spec'd
   status is directly load-bearing for Pass 8** (§1.1) — if it still
   has not landed by the time this Pass is worked, that is the single
   fact that decides whether Pass 8 ships P0 (CLI-first) or P1 (full
   GUI). Worth a pointed check at Pass 8 kickoff rather than discovered
   mid-Pass.
3. **Sanitize is now scoped to the SAME forced-full-rewrite discipline
   as redaction** (§6.1) — a genuine finding of this spec (R35's
   "any operation whose contract is removal" read onto sanitize, which
   the existing standing rules do not yet say explicitly). Worth a
   one-line addition to the R35/R38 record the next time
   `ARCHITECTURE.md` §5 or `ROADMAP.md`'s standing-rules section is
   touched, so a future reader does not have to re-derive it from this
   UI spec.
4. **Fourth (not third) occurrence of the `accesskit`/egui-AT gap**
   (§9) — link alongside the three already recorded against Pass 3.2's
   drag-reorder, Pass 4's deferred text-selection, and Pass 6.1's
   markup tools, rather than filing a fresh entry.
5. **The wording contract tied to core self-verification** (§5.1) —
   "verified removed" vs "redacted" is a phrase choice that depends on
   a specific core capability (post-save re-parse-and-grep) existing.
   Worth flagging to whoever scopes the exact Pass 8 core acceptance
   criteria so the UI wording and the core's actual guarantee are
   decided together, not independently.
