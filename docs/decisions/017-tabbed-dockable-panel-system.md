# Decision 017 — Multi-panel tabbed/dockable panel system for pdfce-gui

**Date:** 2026-08-02
**Status:** DECIDED, then **AMENDED the same day — the §6.1 trigger FIRED and
`egui_tiles` IS ADOPTED. See the Amendment at the end of this record; read it
before implementing anything from §3 or §8.**
**Decided by:** KenAgent decision protocol (`docs/decisions/README.md`), with a
`pdfce-ui-specialist` review that **reversed the initial recommendation**.
**Requested by:** operator — "The Tools dock should be able to have other tools
docked in tabs as well, like any other modern program would."
**Cross-references:** `ARCHITECTURE.md` §3, §7, §10, §12 (continuation-19 —
**superseded in part by this record, see §9**); `LEGAL.md` §6.1/§6.2;
`PRIOR_ART.md` ("CLI / native dialogs / egui extras" table); `ROADMAP.md` R11,
R15; decision 002 (R1, all strings via `ui_text.rs`); decision 003 (portable
posture); decision 012 (the precedent for disclosed session-only GUI state);
`docs/ui_specs/pass-12.M2-dimension-tools.md` §5.1;
`docs/ui_specs/pass-17-dock-and-layer-tree.md`;
`docs/ui_specs/icon-set-and-toolbar.md` (icon pipeline, undecided).

---

## 1. Decision

**Hand-roll the panel system as a two-compartment vertical row list inside the
existing right-hand `egui::Panel::right("tools")`. Add no docking dependency.**

**`egui_dock` is rejected permanently.** **`egui_tiles` is fully vetted and
pre-approved behind one named trigger** (§6) — if the trigger fires, adopt it
without a new decision record; file a dated amendment to this one.

This record **reverses an initial recommendation to adopt `egui_tiles` now**.
The reversal is documented rather than erased (§4) because the reasoning that
produced it is the reasoning that will govern the re-adoption.

---

## 2. What the operator asked for, decoded

"Like any other modern program would" is the requirement, and tabs alone do not
satisfy it. Programs in this class provide four things: panels grouped as tabs;
panels rearrangeable by drag; containers splittable so **two panels are visible
simultaneously**; and layout that **survives restart**.

Item 3 is load-bearing and nearly got missed. pdfce is not only an Acrobat
clone — it is also an Inkscape-parity vector editor, and in every vector editor
**Layers and Properties are used together**: you select an object in the layer
tree and edit its properties without losing sight of the tree. Passes 9 and
12.M2 already put that pairing in play. If Layers and Properties are mutually
exclusive tabs, the tool is *worse* than the flat list it replaced.

Item 4 is deferred for a reason external to this decision (§7).

---

## 3. Chosen design

A **vertical, single-column list of full-width selectable rows**, text-labeled
now, in **two independently-selecting compartments** stacked inside the one
right-hand panel:

- **Upper compartment (selection-scoped):** Properties, Comments, Bookmarks
- **Lower compartment (document-scoped):** Layers/OCGs, batch Tools

Each compartment is its own small row list with its own `Option<DockPanel>`
selection and its own content region below it.

**Why vertical, not a horizontal tab strip.** The dock is `default_size(320.0)`
and spans toolbar→status, so it has height to spend and almost none of the
width a horizontal strip needs. Ten text labels do not fit in 320pt; they
truncate to unreadable abbreviations, wrap to multiple rows, or scroll — and a
horizontally-scrolled-off tab is functionally invisible. That is Acrobat's own
worst habit (ribbon/menu overload) reproduced sideways. Vertical scales by
adding rows at zero horizontal cost, and it is literally the pattern
`tools_dock()` already uses internally (`main.rs:2642–2652`), stretched from
accordion-within-a-panel to list-of-sibling-panels. It is also what programs
with narrow docks actually do — Blender's Properties editor is a vertical tab
column, as are Photoshop's and Acrobat's panel rails.

**Why two compartments, in the P0, not later.** This is the entire simultaneity
answer, and it costs two `Option<DockPanel>` fields instead of one. Retrofitting
"actually two of these need to coexist" after operators have built muscle memory
around one flat list is a materially worse migration than drawing the boundary
up front.

**Icons later, deliberately.** An icon rail is the correct end state — narrower
than text, and `icon_button` (`main.rs:3775`) already carries the
accessible-name-via-tooltip pattern one needs. But pdfce ships **zero icon
images today** and the SVG-in-egui pipeline is an open question in
`docs/ui_specs/icon-set-and-toolbar.md`. An icon-only rail before that pipeline
exists is a column of ambiguous glyphs — worse for discoverability than plain
text. Sequence it as an additive follow-up reusing the same row widget.

---

## 4. Why not `egui_tiles` — and why the initial recommendation was reversed

The initial analysis recommended `egui_tiles` 0.16.0, principally because a flat
tab strip cannot show Layers and Properties simultaneously and a real tiling
engine can. The UI review defeated that argument on two fronts:

1. **Form factor (decisive).** `egui_tiles` draws **horizontal** tab bars only.
   Its 0.16.0 answer to overflow is tab-bar scroll arrows — i.e. it *hides*
   tabs, which is the failure mode, not the fix. Overriding `Behavior::tab_ui`
   does not help: the tab **bar** layout is horizontal in the container code, not
   in the per-tab hook. There is no vertical-tab-column mode, and none of the
   egui docking crates has one.
2. **The simultaneity requirement is met without it.** Two fixed compartments
   with independent selection deliver Layers-above-Properties for two extra
   fields and no dependency. Once that is free, `egui_tiles`' residual value is
   drag-to-rearrange and arbitrary splitting *inside a 320pt column* — marginal,
   and it would let an operator build layouts the column is too narrow to render
   usefully.

**This is a rejection on FIT, not on price.** `egui_tiles` is one of the
cleanest dependencies pdfce could take (§6.2). Do not re-litigate it on
dependency-hygiene grounds; that analysis is complete and it passes.

---

## 5. Why `egui_dock` is rejected permanently

`egui_dock` **0.20.1**, released 2026-06-28 — which **closes the open
verification gap** in `PRIOR_ART.md` recording a 0.19.1-vs-0.20.1 disagreement.
MIT, permissive; the license is not the problem.

1. **Binary splits only** — no n-ary column, no grid. pdfce is heading to 10+
   panels.
2. **Zero accessibility instrumentation, repo-wide.** A code search of
   `Adanos020/egui_dock` returns **0** hits for `widget_info`, **0** for
   `accesskit` (the only such string lives in `Cargo.lock`), and **0** for
   `keyboard`. Its tab bar (`src/widgets/dock_area/show/leaf.rs:968`) is a bare
   `ui.interact(tab_rect, id, Sense::click_and_drag())`. Because that sense sets
   egui's `FOCUSABLE` bit (`sense.rs:81`), those tabs *are* Tab-reachable and
   keyboard-activatable — and **unnamed** to AccessKit. A focusable control that
   announces nothing is the worst case, and there is no upstream sign of intent.
   (`egui_tiles` had the same gap at 0.16.0 — verified at the release tag — but
   has **already fixed it on `main`**, with a commit comment stating the problem
   in pdfce's own terms: *"a tab is an unnamed blob to screen readers, and
   cannot be found by name from `egui_kittest`."* One maintainer understands the
   requirement; the other has never mentioned it.)
3. **`paste` carries RUSTSEC-2024-0436** (INFO/unmaintained; author archived the
   repo). `egui_dock` depends on it directly. Adopting it means committing in
   advance to allowlisting an advisory forever, for a docking crate.
4. **Slower egui-tracking, thinner bus factor** — 3 / 22 / 3 / 2 days across
   egui 0.32–0.35, against `egui_tiles`' 1 / 1 / 1 / 1; single community
   maintainer vs Rerun-funded with egui's own author listed.
5. **Three new packages** (incl. two proc-macro crates) vs one.

Its one genuine advantage is `Surface::Window` (undock into a floating window),
which `egui_tiles` lacks (rerun-io/egui_tiles issue #30). pdfce does not need to
buy that from `egui_dock`: once §8's `panel_body` dispatcher exists, hosting any
panel in an `egui::Window` is a few lines of pdfce's own code.

---

## 6. The `egui_tiles` trigger, and its completed vetting

### 6.1 Trigger

**Primary:** Ken answers escalation Q1 (§10) with the VS Code/Blender model —
the engine owns the whole content area with documents as panes. Horizontal tab
bars are *correct* across a wide canvas region; the 320pt objection is specific
to the narrow side dock and evaporates.
**Secondary:** operators explicitly ask to drag panels between compartments or
resize the compartment boundary. Until asked, do not build it.

If either fires, adopt `egui_tiles` **without a new decision record** — file a
dated amendment here.

### 6.2 Vetting (complete; do not redo)

| Fact | Value |
|---|---|
| Version vetted | 0.16.0 (2026-06-26) |
| License | **MIT OR Apache-2.0** — permissive (LEGAL §6.1); §6.2 step 3 → proceed and log. No operator flag. |
| MSRV | 1.92 — pdfce is 1.92, exact match |
| egui requirement | `^0.35.0` — pdfce pins 0.35.0, exact match |
| **New packages in the shipping graph** | **1** |
| Transitive deps | `ahash 0.8.12` (via eframe/egui/epaint), `itertools 0.14.0` (via egui), `log 0.4.33` (via egui/eframe/rfd/tiny-skia) — **all already present at satisfying versions**, verified by `cargo tree` |
| Transitive licenses | all MIT OR Apache-2.0 — zero copyleft, zero FFI, zero build scripts, crate denies `unsafe` |
| WASM | wasm32-clean (`ahash` pinned to `no-rng` → no `getrandom`; no threads/time/fs); production-proven in Rerun's browser viewer |
| Cadence | 1 day behind each of egui 0.32/0.33/0.34/0.35 |
| Persistence | `Tree<Pane>` derives Serialize/Deserialize under the `serde` feature (`src/tree.rs:30`), incl. a JSON-infinity workaround for width/height |

Re-verify only the version-specific facts against whatever release is current
when the trigger fires — `egui_tiles` `main` has already bumped `rust-version`
to 1.95 for a future release.

**Two gotchas recorded now so the future Pass does not rediscover them:**
`Tree<Pane>` derives only `Clone, PartialEq` — **not `Default`**, so
`std::mem::take` will not compile; use
`std::mem::replace(&mut self.dock, Tree::empty("swap"))` for the borrow dance.
And `SimplificationOptions::default()` has `prune_single_child_tabs: true` with
`all_panes_must_have_tabs: false`, which makes **the tab bar vanish when only
one panel is open** — override it to `all_panes_must_have_tabs: true`.

---

## 7. Persistence

**This Pass: session-only, explicitly disclosed.** No serde, no ron, no new
packages.

**Correcting a factual error in the UI review** (recorded because acting on it
would send an engineer hunting for a mechanism that does not exist): the review
states the pinned-panel set should persist "the same way `rail_expanded` /
`tools_open` are already persisted." **They are not persisted.** `main.rs:353–354`
and `:383–384` say the opposite in as many words — *"Session state only —
deliberately not persisted to disk"* and *"**Session state only**, deliberately
not persisted to disk — same stance as `rail_expanded`/`tools_open`."* Nothing
in the GUI is persisted today; there is no convention to reuse.

**Do not enable eframe's `persistence` feature.** It expands to
`["dep:home", "egui-winit/serde", "egui/persistence", "ron", "serde"]` and writes
to a *platform* app-data directory — contradicting decision 003's
single-folder-portable posture and R15, which requires user state in a named
partition of the distribution folder so the update procedure can name which
files to keep.

Layout therefore stays session-only and is disclosed as such — precisely the
precedent decision 012 set for the font-folders setting.

**Contract for the Pass that lands R15:** deserialization is **fail-soft**. File
missing, parse failure, unknown `DockPanel` variant, or a mandatory panel absent
→ fall back to the **default layout**, noted in the status surface. Never an
error dialog, never a lost document session. This is the GUI-layer expression of
the fail-clean invariant (ARCHITECTURE §10): a corrupt *preference* must never
cost the operator their *work*.

---

## 8. Implementation path

1. **`enum DockPanel` + one `panel_body(&mut self, panel, ui, actions)`
   dispatcher.** Store each compartment's pinned set as an **ordered
   `Vec<DockPanel>`**, not a fixed array — that is what makes the later pin/unpin
   affordance additive rather than a rewrite. Engine-agnostic; survives verbatim
   if the §6 trigger ever fires.
2. **Two compartments, two selections, two list-plus-content regions.** Reuse
   `selectable_label` + `toggle_label` (bold-on-selected) from
   `main.rs:2642–2652` / `:3792` verbatim. No new widget type.
3. **Retire `properties_window()`'s floating form**; move its body into the upper
   compartment. **Keep** `properties_button()` (`ui_text.rs:336`, wired at
   `main.rs:3983`) and its shortcut — change only the effect to "open dock +
   select Properties." Same muscle memory, new destination. **No float-OR-dock
   dual mode**: two code paths for the same content, duplicating open-state,
   position/size and focus handling, for zero operator benefit at this scale.
4. **Prerequisite bugfix, mandatory, ships with or before step 3:** `open_path()`
   leaves `properties_draft` stale/empty across a new-document open. Today that
   is masked because operators close a floating window; a **persistent dock row
   makes the blast radius worse**. Reseed `properties_draft` in `open_path()`.
5. **Rename the inner "Tools" row** in `ui_text.rs` (e.g. "Batch Tools" /
   "Document Tools"). A row labeled "Tools" inside a container called "the Tools
   dock" is a real collision an operator trips on.
6. **Accessibility.** Every row: accessible name **plus a purpose tooltip that
   says WHEN to use it**, not a restatement of the label ("Show/hide
   optional-content groups and vector object layers", not "Layers"). Reuse the
   `icon_button` `WidgetInfo` pattern (`main.rs:3775`) even for text rows.
   Selected state **must pair a weight/glyph cue with colour** — the GUI-polish
   audit already flagged colour-fill-only selected state as a *recurring* blind
   spot on Fit Page/Width, the Properties toggle and the annotations toggle. Do
   not let the new list repeat it in its inaugural instance.
7. **Tab chain.** No change to the panel-add order (toolbar → status → rail →
   dock → canvas, `main.rs:87–123`). Within the dock, the picker list Tabs
   **before** the selected panel's widgets (pick-then-fill), which is free if the
   list is drawn first, exactly as today.
8. **Focus on switch stays on the clicked row.** Do **not** auto-focus the new
   panel's first field — an operator arrow-keying between rows would suddenly be
   typing into a document field.
9. **No fake affordances.** No drag cursor, drag highlight, or resize handle for
   an interaction that is not implemented (§R83).
10. **Ctrl+Tab / arrow-key cycling: deferred, explicitly.** No such handling
    exists anywhere in the codebase and it is not an established pdfce rule. Note
    it as a future accelerator; do not ship it silently as if expected.
11. **`"+ More panels…"`** — not needed at 3–5 panels. Once the pinned set
    outgrows the visible column, add it as a fixed last row opening a multi-select
    checklist. **Pin order = append order**, with unpin/repin as the promotion
    mechanism — the only relief for "my most-used panel is buried" before real
    drag-reordering exists.
12. **Ship "Reset panel layout"** in the same Pass.
13. **Rule 11 (CLI parity) does not apply** — pure GUI-shell change, no headless
    equivalent. Say so in the Pass entry so the omission reads as reasoned.
14. **Update `main.rs`'s accessibility doc-comment (191–208):** a hand-rolled row
    list built from `selectable_label` does **not** acquire an AccessKit
    `Tab`/`TabList` semantic **role** just because it looks like tabs — only the
    **name** is supplied. Record it as a tracked gap, in the same honest
    convention the module already uses for the canvas's screen-reader gap.

---

## 9. Decision-log contradiction this Pass must resolve

`ARCHITECTURE.md` §12 continuation-19 records: *"Properties stays the single
legacy floating exception, never to be joined by a second."* **That is now false
on two counts.** Pass 12.M2's Dimension Groups panel already shipped as a second
floating `egui::Window` (`docs/ui_specs/pass-12.M2-dimension-tools.md` §5.1),
and this Pass retires Properties' floating form entirely.

Dispatch `pdfce-librarian` for a **superseding entry** — not a silent
contradiction left standing in the decision log — and name **Dimension Groups as
the remaining floating-window holdout** for a follow-up migration into the same
dock, so it does not quietly become the new "one legacy exception."

---

## 10. Escalated to the operator

**Q1 — Does the panel system own only the right-hand dock, or eventually the
whole content area including the canvas?** *(Promoted from a design detail to
this decision's own trigger.)* Under the narrow model the hand-rolled vertical
row list is right and `egui_tiles`' horizontal tab bars do not fit; under the
wide model horizontal tab bars become correct across the canvas region and
`egui_tiles` is pre-approved for immediate adoption. Building narrow first does
not block wide **provided** `DockPanel` is designed so a payload-carrying
`Document(DocId)` variant is a non-breaking addition — cheap now, expensive to
retrofit through a persisted layout schema. **Default if unanswered:** build
narrow, keep `DockPanel` extensible.

**Q2 — Should panels be undockable into separate OS windows (multi-monitor)?**
Needs egui's multi-viewport machinery and interacts with crash-safe autosave and
the persistence schema. Not something to back into via a library choice.
**Default:** no; docked-only; its own Backlog entry.

**Q3 — Confirm the compartment assignment** (upper = Properties/Comments/
Bookmarks; lower = Layers/OCGs/batch Tools). Product judgment informed by what
Passes 6.1 / 9 / 12.M2 shipped. **Default:** ship the proposed split.

**Not escalated: licensing.** No dependency is added, so no licensing question
arises. The `egui_tiles` pre-vetting (§6.2) is recorded so the future trigger
needs no fresh legal pass.

---

## 11. Proposed standing rules (next free number is R80)

- **R80** — The right-hand dock is a two-compartment, independently-selecting
  panel host. Every dockable surface is a `DockPanel` variant reached through ONE
  `panel_body` dispatcher. No panel is reachable ONLY as a floating window.
- **R81** — Floating windows are for TRANSIENT surfaces only (confirmations,
  blocking questions, modeless references). Anything an operator keeps open while
  working on the document is a dock panel. Supersedes §12 continuation-19's
  "single legacy floating exception."
- **R82** — Panel layout is user state, and user state rides R15. Never persisted
  through eframe's platform-directory Storage.
- **R83** — No affordance without the capability.
- **R84** — Selected state is never colour alone; pair it with a weight or glyph cue.

---

# AMENDMENT A — 2026-08-02 — the §6.1 trigger fired: `egui_tiles` is ADOPTED

**Filed by:** pdfce-engineer, per §6.1's instruction that a fired trigger is
recorded as *"a dated amendment to this one"* rather than a new decision record.
**Status:** operative. Where this amendment and §§1/3/8 differ, **this amendment
wins.**

## A.1 What the operator said

Asked whether the panel system owns only the right-hand dock or eventually the
whole content area (§10 Q1 — the named trigger), Ken answered:

> *"Use egui_tiles. You're building something to compete with Acrobat and is
> open source, and has the flexibal docking that works as well as inkscape's."*

That is the **primary trigger** in §6.1, answered in the widest direction
available: the comparison class is set by Acrobat and Inkscape, and the stated
requirement is *flexible docking*, not merely "more than one panel."

## A.2 What changes

**Adopt `egui_tiles` 0.16.0.** The §6.2 vetting stands and does **not** need
redoing — MIT OR Apache-2.0 (permissive; `LEGAL.md` §6.2 step 3: proceed and
log, no operator flag), **one** new package in the shipping graph, every
transitive dependency (`ahash`, `itertools`, `log`) already present at a
satisfying version, zero copyleft, zero FFI, zero build scripts, crate denies
`unsafe`, wasm32-clean, exact MSRV (1.92) and egui (0.35.0) match.

**Re-verify only the version-specific facts** against whatever release is
current at build time — §6.2 already warns that `egui_tiles` `main` has bumped
`rust-version` to 1.95 for a future release. If the current release demands an
MSRV above pdfce's, pin to the last release that does not.

`THIRD_PARTY_LICENSES.md` must be regenerated via `cargo-about` when this
dependency lands (rule 13 / `LEGAL.md` §6.3).

## A.3 What §4's reversal was about, and why it no longer blocks

§4 rejected `egui_tiles` on **fit at one specific scope**: a 320pt-wide side
dock cannot display 10+ *horizontal* tab labels, and horizontal is all
`egui_tiles` draws. That objection was correct and remains correct **for a
narrow side dock in isolation**. §6.1 named precisely this escape — horizontal
tab bars are the *right* form across a wide content area — so the objection
dissolves once the engine also owns the wide region, which is the model the
operator chose.

**What must NOT be lost from §3.** The two-compartment design solved a real
requirement independent of which engine draws tabs:

> **Layers and Properties must be visible SIMULTANEOUSLY.** In every vector
> editor you select an object in the layer tree and edit its properties without
> losing sight of the tree. Passes 9 and 12.M2 already put that pairing in play.
> If Layers and Properties are mutually exclusive tabs, the tool is *worse* than
> the flat list it replaced.

Under `egui_tiles` this becomes a **vertical split container** with Layers above
Properties instead of two hand-rolled compartments. The requirement survives;
only its mechanism changes. **Ship the default layout with that split already in
place** — do not ship one tab group and leave the operator to discover they must
drag a panel out to see both. §3's argument for drawing the boundary in the P0
applies unchanged.

**Narrow-column overflow remains a real hazard, not a solved one.** §4's
arithmetic is still true. Mitigate rather than ignore:
- Do not pack 10+ panels into one narrow tab group by default.
- Prefer a wider default dock than 320pt now that it hosts real content.
- Treat 0.16.0's tab-bar scroll arrows as a **failure indicator**: if the
  default layout needs them, the default layout is wrong.

## A.4 Guidance that SURVIVES — still binding

From §8, deliberately written engine-agnostic:

1. **`enum DockPanel` + one `panel_body(…)` dispatcher.** §8.1 predicted this
   *"survives verbatim if the §6 trigger ever fires."* It has. Build it as
   specified; it becomes the `egui_tiles` pane payload. Keep it **extensible** —
   a payload-carrying `Document(DocId)` variant must remain a non-breaking
   addition (§10 Q1's caveat), and under the wide model that variant is now
   *expected*, not hypothetical.
2. **§8.3** Properties migration — retire the floating form, keep
   `properties_button()` and its shortcut as the entry point, no float-OR-dock
   dual mode.
3. **§8.4** the `properties_draft` staleness bugfix — still mandatory, ships
   with or before the migration.
4. **§8.5** the "Tools" row naming collision — still needs fixing.
5. **§8.6** accessibility — now *more* important. §5's audit found `egui_dock`'s
   tab bars focusable but unnamed to AccessKit; `egui_tiles` 0.16.0 had the same
   gap at its release tag and **fixed it on `main`**. Verify which side of that
   fix the pinned version falls on; if the unfixed side, either bump or supply
   names via `Behavior::tab_ui`. Adopting a docking engine must not silently
   regress the keyboard/AT story `main.rs`'s accessibility doc-comment commits to.
6. **§8.9** no fake affordances; **§8.12** "Reset panel layout" (now *more*
   necessary — a draggable layout can be wrecked in ways a fixed one cannot);
   **§8.13** rule 11 does not apply; **§8.14** doc-comment honesty.

## A.5 Guidance now SUPERSEDED

- §3's *"vertical, single-column list of full-width selectable rows"* as the
  picking mechanism, and §8.2's two hand-rolled compartments — replaced by
  `egui_tiles` containers. **The simultaneity requirement they served is NOT
  superseded** (A.3).
- §8.10's deferral of Ctrl+Tab cycling — reconsider; a real tab engine makes tab
  cycling an expected accelerator rather than an invented one.
- §8.11's `"+ More panels…"` overflow row — `egui_tiles` owns adding panes.

## A.6 Persistence — §7 stands, with one addition

Layout stays **session-only and disclosed as such** this Pass. §7's prohibition
on eframe's `persistence` feature is unchanged.

Addition: §6.2 notes `Tree<Pane>` derives Serialize/Deserialize under the
crate's `serde` feature. **Do not enable it yet.** When R15 lands, serializing
the tree is the natural mechanism, governed by §7's **fail-soft** contract — a
missing file, parse failure, unknown `DockPanel` variant, or absent mandatory
panel all fall back to the **default layout**, noted in the status surface;
never an error dialog, never a lost document session. A draggable layout makes
that contract *more* load-bearing: the operator can now save a layout a later
build cannot represent.

## A.7 Gotchas — carried forward from §6.2

- `Tree<Pane>` derives only `Clone, PartialEq` — **not `Default`**, so
  `std::mem::take` will not compile. Use
  `std::mem::replace(&mut self.dock, Tree::empty("swap"))`.
- `SimplificationOptions::default()` sets `prune_single_child_tabs: true` with
  `all_panes_must_have_tabs: false`, making **the tab bar vanish when only one
  panel is open**. Override `all_panes_must_have_tabs: true`.

## A.8 Still open

- **§10 Q2 (undock into separate OS windows / multi-monitor)** — NOT answered
  here and NOT granted by adopting `egui_tiles`, which has no `Surface::Window`
  equivalent (rerun-io/egui_tiles issue #30). §5 notes pdfce can host any panel
  in an `egui::Window` itself once `panel_body` exists. Default stands:
  docked-only, its own Backlog entry.
- **§10 Q3 (which panels pair)** — the proposal becomes the **default layout**
  rather than a fixed structure, lowering the stakes since the operator can now
  drag. Ship it.
- **§5's permanent rejection of `egui_dock`** is unaffected and was independent
  of the trigger.
- **`docs/ui_specs/pass-17-dock-and-layer-tree.md` §A** predates both this record
  and this amendment and describes a horizontal tab strip. Superseded twice
  over. Its **object/layer tree** (§B) and **canvas selection feedback** (§C)
  sections are unaffected and remain the build spec for those.
