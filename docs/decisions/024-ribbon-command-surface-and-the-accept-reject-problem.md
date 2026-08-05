# Decision 024 — A ribbon command surface, and the end of the floating Accept/Reject box

**Status:** Decided (consultant recommendation; engineer to schedule, librarian to file).
Two halves, decided independently and shippable independently — §4 (the confirm model)
does **not** depend on §2/§3 (the ribbon), and is the half that answers the operator's
sharpest complaint first.
**Date:** 2026-08-04
**Requested by:** operator, verbatim (§1.1), relayed through `pdfce-engineer`.
**Amends:** nothing. **Extends** decision 017 + its Amendment A (the `egui_tiles` dock) —
this record deliberately leaves the dock **untouched** and explains why (§3.4).
`docs/decisions/README.md` is explicit that the directory is append-only, so nothing in
017, 018, 022 or 023 is rewritten here; every interaction with them is a forward reference.

**Decision number:** **024** — verified against the live ceiling at write time, not assumed
(R106). `python tools/check-ledger-numbers.py --stats` at draft time reports
`decision records: 023 -> next free is 024`; same run: `standing rules: R120 -> next free
is R121`; `Pass families with headings: up to 21 (highest ID 21.1)`,
`Pass families MENTIONED: up to 23 (highest ID 23.3)`,
`CLAIMED BUT NOT YET HEADED: 5, 9, 9c, 10, 13, 20, 22, 23`. This record therefore claims
**Pass family 24**, as the engineer's dispatch instructed. Open-operator-question letters
run to **(ab)**; this record claims **(ac)–(aj)**. **Re-run the checker at filing time** —
this project has collided on numbering five documented times, which is why every recent
record carries this paragraph.

**Terminology (binding, operator, 2026-08-04; CLAUDE.md rule 15).** This record never
writes bare "dimension". **pdf dimensions** are dimensions already present in a file
exported by CAD or another authoring tool — pdfce reads and measures against them and must
not silently alter them. **ce dimensions** are the dimension objects pdfce itself authors
(`/Line` + `/IT /LineDimension` with a baked `/AP`, their groups, scale, `/Measure` dict
and `/PieceInfo` sidecar). The Measure tool authors **ce dimensions**; the contextual tab
§3.3 proposes for a selected ce dimension is a **ce-dimension** tab and is named that way
in every string it would ship (§3.3's label note is where rule 15 does real work in this
record, not decoration).

**Cross-references:** `CLAUDE.md` rules 4, 6, 12, 15; `ARCHITECTURE.md` §3 (crate layout,
the GUI-core separation invariant), §11.4 (widget → `Action` → session, never widget →
`Document`), §12 (decision log); `ROADMAP.md` standing rules **R1–R3** (single string
catalog, no sentence assembly, no English-width layout), **R80–R84** (dock host, floating
windows are transient-only, layout is user state, no affordance without capability,
selected state is never colour alone), **R85** (preview-equals-saved), **R86** (observed
working in the running application), **R99** (state → action → detail ordering in a bounded
container), **R111/R112** (selection enumerates what the renderer paints; a selectable kind
carries its verb set in its type), **R117** (R83's inverse — a capability reachable from no
surface), **R118/R119** (format does not ride the gesture; a geometry change to a
measurement is two-stage and disclosed), **R120** (a precedence chain takes a named-field
context); decisions **010** (the canvas foundation is built once), **011** (the
scaled-measurement / ce-dimension subsystem), **017 + Amendment A** (the dock),
**018** (the canvas renders the edited document), **022** (annotations in canvas
selection), **023** (level navigation, ce-dimension re-measure, the format surface);
`docs/ui_specs/icon-set-and-toolbar.md`, `docs/ui_specs/menu-affordance-and-glyph-coverage.md`,
`docs/ui_specs/gui-polish-current-featureset.md`, `docs/ui_specs/pass-12.M2-dimension-tools.md`,
`docs/ui_specs/pass-14.3-text-edit-ui.md`, `docs/ui_specs/pass-16.2-add-text-ui.md`,
`docs/ui_specs/pass-19.3-text-formatting-surface.md`.

---

## 0. Summary

**Decision, in one line each:**

1. **Yes — adopt a ribbon-style command surface**, replacing the single wrapping icon
   toolbar. Six fixed tabs plus a `File` menu button, contextual tabs for the active tool
   and for the selected object. §2, §3.
2. **No — do not replace the `egui_tiles` dock.** The dock is not what the operator
   complained about, both reference products he named have a ribbon *and* side panels, and
   the two surfaces are orthogonal. §3.4.
3. **Yes — kill the floating Accept/Reject box**, and replace it with a **fixed-anchor
   confirm** plus **Enter/Escape as universal commit/cancel**, tiered by whether the
   gesture carries an inference. §4.
4. **Rule 4 ("fuzzy, never sneaky") needs narrowing, not repeal** — and the narrowing is
   the load-bearing intellectual move in this record. §4.4 proposes exact wording. It is
   flagged for the operator because rule 4 lives in `CLAUDE.md`, which is his file, not the
   engineer's. §8 item 1.
5. **Hand-build it. No new dependency.** **No ribbon crate exists for egui in any state of
   repair** — verified against crates.io, the GitHub repo/issue/discussion APIs and the
   third-party-crates wiki (§5.1). But egui 0.35 supplies more of the parts than expected —
   a real `MenuBar`/`MenuButton`/`SubMenu` API whose own doc comment says it *"goes well in
   a `Panel::top`"*, **built-in `Panel::show_switched` / `show_collapsible` animation that
   is exactly the ribbon collapse/expand primitive**, and spatial arrow-key focus
   navigation. pdfce already owns six members of the button-widget family a ribbon needs a
   seventh of. §5.
6. **Six Passes, 24.0 → 24.5**, ordered so the operator sees the *confirm* fix first, in a
   Pass that does not touch the ribbon at all. §6.

**Six findings drive this record. Each was verified in the code or in the pinned crate
source, not inferred from memory:**

1. **The complaint is about PLACEMENT, not about the existence of a confirm step — and one
   of the two products the operator named to hold pdfce against has an Accept/Reject pair
   of its own.** SolidWorks' PropertyManager carries a ✓ / ✗ pair at its top-left for every
   modal feature command. What it does *not* do is float that pair over the graphics area
   at a position derived from the page image. pdfce's does exactly that. §4.1. *This is a
   claim about a product the operator uses daily and can falsify in two seconds — it is
   stated so it can be checked, and if it is wrong, §4.2's tier model still stands but
   §4.1's argument for it does not.*

2. **The Accept/Reject controls are `egui::Area`s pinned to the PAGE IMAGE's bottom-left
   corner, not to the window.** `main.rs:9366`, `:10244`, `:11942` are all
   `.fixed_pos(egui::pos2(image_rect.min.x + 8.0, image_rect.max.y - 8.0))` with
   `.pivot(Align2::LEFT_BOTTOM)`. So the confirm control **moves when the operator scrolls,
   zooms or changes page**, and lands under whatever page content happens to be at the
   bottom-left. That is not merely unconventional; it is a control whose position is a
   function of the document. §1.2.

3. **Enter already commits in exactly one of the three tools, and nobody noticed the
   inconsistency.** `main.rs:9959-9970`: Add Text takes Enter (point mode) / Ctrl+Enter
   (box mode) as Accept. The Measure tool and the Text Edit tool have **no** keyboard
   commit at all — a repo-wide grep for `Key::Enter` in `pdfce-gui` returns that one site.
   So one third of the answer is already built, in one place, undocumented as a convention.
   §4.3.

4. **egui 0.35's `WidgetType` has no `Tab`, `TabList`, `Toolbar` or `MenuBar` role — and
   the AccessKit mapping is worse than "missing", it is actively collapsing.** Verified
   twice: the enum at `egui-0.35.0/src/lib.rs:623-668` has exactly twenty variants
   (`Label, Link, TextEdit, Button, Checkbox, RadioButton, RadioGroup, SelectableLabel,
   ComboBox, Slider, DragValue, ColorButton, Image, CollapsingHeader, Panel,
   ProgressIndicator, Window, ResizeHandle, ScrollBar, Other`), and the mapping at
   `src/response.rs:914-936` sends `Button | CollapsingHeader | SelectableLabel` **all to
   `accesskit::Role::Button`**. AccessKit itself *has* `Role::Tab`/`Role::TabList`; the
   ceiling is egui's mapping, not the backend. So ribbon tabs announce as **buttons that
   correctly report their pressed state** (egui 0.35 added that — upstream PR #8130) and
   nothing more. This is the **identical, already-documented** gap `dock.rs` records for
   `egui_tiles`' tabs. Not a new debt; the same debt at a second surface. §5.3.

5. **egui 0.35 already does spatial arrow-key focus navigation, and pdfce does not consume
   plain arrow keys globally.** `egui-0.35.0/src/memory/mod.rs` defines `FocusDirection`
   with seven variants and `:575-581` maps bare ArrowUp/Right/Down/Left onto the cardinal
   ones; `end_pass` resolves them against widgets' actual screen rects. pdfce's
   `collect_keyboard_actions` binds only **Alt**+ArrowUp/Down (`main.rs:5315-5316`), and
   the Text Edit tool consumes arrows only while the canvas image has focus
   (`main.rs:8848-8880`). **So ribbon-band arrow-key navigation is free** — it is the
   behaviour a ribbon is expected to have, and it costs nothing but not breaking it.
   Recorded here because a future Pass adding a global bare-arrow binding would silently
   destroy it, app-wide, with no visual symptom.

6. **egui has NO focus-group / roving-tabindex concept, and there is a focus dead-man's
   switch that a tab switch will trip.** `focus_group`, `roving`, `tabindex` return zero
   hits across egui 0.35's source. Tab order is one flat global sequence in
   widget-registration order, and `skip_ahead_auto_ids` (`src/ui.rs:898`) is an **ID
   allocation** tool, not a focus-order tool — a correction to the natural assumption.
   Separately, `memory/mod.rs:617-624` **drops focus to `None` when a focused widget stops
   being drawn**, which is precisely what happens when the operator switches ribbon tabs
   while keyboard-focused inside a band. Both facts are load-bearing for Pass 24.5 and are
   named here so they are constraints rather than surprises. §5.3.

**Where the operator's model does not fit, stated plainly** (the engineer's dispatch asked
for concrete tradeoffs, not an agreeable yes):

- **A ribbon costs canvas height, permanently.** Today's wrapping toolbar is roughly one
  24 pt button row plus frame padding. A two-row ribbon (command band + tab strip) at
  Office proportions is ~110 pt. On the 1100 × 800 default window
  (`INITIAL_WINDOW_SIZE`, `main.rs:255`) that is ~10 % of the window height moved from
  document to chrome, every frame, forever. Collapse (§3.6) mitigates it; nothing removes
  it. This is the single largest thing given up and it must be a conscious trade, not a
  surprise.
- **A ribbon makes a sparse application look sparse.** Office's ribbon is dense because
  Office has thousands of commands. pdfce has ~30 toolbar commands today. Six tabs over 30
  commands will look empty — and **R83 forbids the obvious fix** (greying in placeholders
  for unbuilt features). §7 proposes a rule making the empty space deliberate rather than
  an embarrassment to be padded. The honest framing: a ribbon is the *right shape for where
  pdfce is going* and a slightly loose fit for where it is.
- **There is no prior art. pdfce would be first.** The research sweep found **zero** GitHub
  repositories matching "egui ribbon", **zero** issues in `emilk/egui` mentioning a ribbon,
  and **zero** discussions. Nobody has built one, and nobody has even asked upstream for
  one. There is no reference implementation to crib overflow maths, contextual-tab state
  machines or collapse behaviour from. §5.1. Budget accordingly; this is why §6 has six
  Passes and not two.
- **"Contextual tabs" require a selection model rich enough to key them on.** pdfce's is —
  `TargetId` is already an enum over kinds and decision 022's R112 requires every verb
  dispatch on it to be an exhaustive match, which is exactly the property a contextual-tab
  dispatcher needs. But **Pass 22.0 (annotations in canvas selection) and Pass 23.2
  (`TargetId::Content` → `ContentPath`) are both unbuilt.** The ce-dimension contextual tab
  and the page-object contextual tab therefore **cannot ship before those**, and Pass 24.3
  is sequenced behind them rather than pretending otherwise. §6.4.
- **A ribbon does not, by itself, fix "doesn't match other software."** §1.3 enumerates six
  more divergences found in the code, one of which (drag-on-canvas is a marquee, not a pan)
  carries a comment in `main.rs` admitting the UX review on it *"was owed and never
  happened."* Shipping a ribbon over those leaves them shipped, in a nicer frame.

---

## 1. What the operator said, decoded

### 1.1 Verbatim (2026-08-04)

> *"The interface is a bit weird where there is a separate accept / reject box somewhere on
> the screen to click - I've never seen any other software operate that way. Zoom is also
> jarring because cntrol-scroll doesn't treat the cursor as the center of the zoom, but the
> workspace area instead. There's quite a few weird gui setup decisions that don't match up
> to anything I've seen in other software. This should look and feel like the ribbon
> interface in MS Office, solidworks, and other modern software."*

Four statements, of which the zoom-to-cursor one is already fixed and is out of scope for
this record by the engineer's instruction. The remaining three are **not** three complaints;
they are one complaint stated at three altitudes:

| Altitude | The statement | What it actually asks for |
|---|---|---|
| **Specific defect** | "a separate accept / reject box somewhere on the screen" | Commands must commit where the operator is working, from a control whose position is predictable. §4. |
| **Pattern** | "quite a few weird gui setup decisions" | An audit, not a feature. §1.3 supplies one. |
| **Target** | "look and feel like the ribbon interface in MS Office, solidworks" | A named, industry-standard command-surface architecture, adopted deliberately rather than converged on accidentally. §2, §3. |

**The altitudes must be answered in that order, not in the order stated.** Building the
ribbon first and the confirm model second would ship a beautiful frame around the exact
behaviour that prompted the message. §6 sequences accordingly: **Pass 24.0 is the confirm
fix and contains no ribbon.**

### 1.2 "Somewhere on the screen" is literally true — measured

The three tool status strips are `egui::Area`s positioned against `image_rect`, the
rectangle of the **rasterized page**, not against the window:

| Tool | Site | Position |
|---|---|---|
| Text Edit (+ reflow) | `main.rs:9364-9367` | `fixed_pos(image_rect.min.x + 8.0, image_rect.max.y - 8.0)`, `pivot LEFT_BOTTOM` |
| Add Text | `main.rs:10242-10245` | identical |
| Measure (ce-dimension authoring) | `main.rs:11940-11943` | identical |

Consequences, none of which are hypothetical:

- **Zooming moves the Accept button.** `image_rect` is derived from the current raster
  scale, so a Ctrl+wheel gesture slides the confirm control across the window.
- **Scrolling moves it.** Panning the canvas moves `image_rect`; the confirm goes with it,
  and on a page taller than the viewport it can leave the visible area entirely.
- **Changing page moves it**, because a different page has a different extent.
- **It sits ON the document, over content**, at the one corner of a CAD drawing most likely
  to hold a title block.

The Measure tool's *property* bar already had precisely this problem diagnosed and
half-fixed: `main.rs:11811-11823` records that it was `.fixed_pos(...)` pinned to the
page's top-left *"with no way to shift it or dismiss it. On a drawing whose dimensions sit
under that corner, the box covers exactly the geometry the operator is trying to pick."*
The fix applied there was `default_pos` + `movable(true)` — i.e. **the operator was made
responsible for dragging the application's own controls out of his way.** That is the
correct emergency patch and the wrong permanent answer, and its sibling — the *status*
strip carrying Accept/Reject — never even got the patch.

**Stated as the finding it is:** the codebase already contains a written diagnosis of this
exact failure mode, at the neighbouring control, dated the same day as the operator's
message. The pattern was visible from inside; what was missing was the decision to change
the shape rather than to make the shape draggable.

### 1.3 "Quite a few weird gui setup decisions" — the audit the phrase asks for

The operator named two. The engineer's dispatch names two. Here is what a read of
`pdfce-gui` finds, so the list is a list rather than an ellipsis. **This record does not
decide any of items 3–8** — they are surfaced with a recommendation each, for the operator
and the `pdfce-ui-specialist` to take separately (§8).

1. **Floating Accept/Reject.** Named. Decided in §4.
2. **Ctrl+scroll zoom ignores the cursor.** Named. Already fixed; out of scope.
3. **Drag on the canvas is a rubber-band marquee, not a pan.** Every PDF reader in
   existence pans on drag (Acrobat's Hand tool is its *default* tool). Pass 9a changed this
   deliberately and `main.rs:182-189` records the admission verbatim: *"its own comment
   records that a UX review was owed on that default and never happened. The operator
   asking for middle-drag panning IS that review arriving."* **Recommendation:** space-drag
   and middle-drag pan universally (the CAD/Illustrator convention, and SolidWorks' own
   middle-drag idiom), marquee stays on left-drag. Not decided here.
4. **Tool options float over the page.** No mainstream application does this. Photoshop has
   a fixed options bar under the menu; Office has a contextual ribbon tab; SolidWorks has
   the PropertyManager docked left. pdfce floats them at the page's top-left corner and, as
   of 2026-08-04, lets you drag them. **Decided here**, as part of §3.3 / Pass 24.2.
5. **No menu bar at all.** Every application the operator named has File/Edit/View menus.
   pdfce has zero: `menu_button_labeled` is used for four *dropdown buttons* (Markup, Text,
   Copy, Measure), which is not a menu bar. A ribbon's `File` button (§3.5) answers part of
   this; whether pdfce should *also* carry a conventional menu bar is a real question with
   a real answer in Office's own history (it removed one). **Recommendation: no separate
   menu bar** — one command surface, not two. Named as operator question (ae).
6. **Two mental models for the same side region.** The toolbar has a `Tools` toggle that
   opens/closes the whole dock, *and* a `Properties` toggle and a `Redact` toggle that each
   open the dock and activate one tab. So "the panel button" means two different things
   depending on which button it is. The ribbon's View tab (§3.2) normalises this: every
   panel gets one toggle with identical semantics, and the region-level show/hide becomes a
   separate, clearly-labelled control.
7. **Escape means six different things by precedence and nothing announces which.**
   `canvas::resolve_escape` today resolves four outcomes; decision 023 §3.2 and the
   in-flight navigation Pass take it to six. The chain is well-designed and completely
   invisible. **Recommendation:** the status surface names what the next Escape will do,
   the way the breadcrumb decision 023 §3.5 argues for names the current level. Not decided
   here; noted as a cheap, high-value follow-up.
8. **`Fit Page` / `Fit Width` are sticky *modes* rendered as selected toggles.** This one is
   **correct** and matches Acrobat; it is listed so it does not get "fixed" during a
   ribbon migration by someone tidying toggles into buttons.

### 1.4 What "ribbon" means, decomposed

"Ribbon" is not a look; it is four separable mechanisms, and pdfce can adopt them
independently. Naming them separately is what makes §6's staging possible:

| # | Mechanism | Present in pdfce today |
|---|---|---|
| **M1** | **Tabbed command bands** — commands grouped by task phase into named tabs, one band visible at a time | No. One flat wrapping row. |
| **M2** | **Named groups inside a band**, with a visible group caption and size-by-importance (a large icon-over-label button for the primary command, small rows for secondary) | Partially — `ui.separator()`-divided groups exist and are documented as a deliberate structure, but they are **unnamed** and every control is the same size. |
| **M3** | **Contextual tabs** — a tab that appears only while a given object kind is selected or a given tool is armed, and auto-activates | No. Tool properties float; selection properties are in the dock. |
| **M4** | **Quick Access + File/Backstage + keytips** — an always-visible micro-toolbar, a file-scope menu, and Alt-key command access | No QAT, no File menu, no keytips. |

**M1 + M2 + M3 are what the operator is asking for.** M4 is partly cheap (QAT, File menu)
and partly expensive-and-unsupported (keytips — §5.3). The record adopts M1, M2, M3, and
the cheap half of M4, and **refuses keytips by name**.

---

## 2. Q1 — Should pdfce adopt a ribbon? The decision, and what is given up

### 2.1 Decision

**Yes.** `pdfce-gui` replaces its single wrapping icon toolbar with a ribbon: a `File` menu
button and six fixed tabs across the top, one command band visible at a time, contextual
tabs appearing for the armed tool and for the selected object kind.

**The `egui_tiles` dock is NOT replaced** and is not part of this decision beyond gaining
one toggle per panel on the View tab. §3.4.

**The floating property bars and floating Accept/Reject strips ARE replaced** — but by a
mechanism (§4) that ships **before** the ribbon and stands on its own if the ribbon were
abandoned.

### 2.2 Why yes — five reasons, in decreasing order of weight

1. **The operator asked, by name, twice, with two reference products.** This is a product
   owner specifying a UI architecture for his own tool. The consultant's job is to say what
   it costs and how to get there, not to relitigate whether it is the best of all possible
   command surfaces. The one thing that *would* justify pushing back is a technical
   impossibility, and there is none (§5).

2. **pdfce's command count is on a trajectory the flat toolbar cannot absorb, and the
   codebase already says so in three places.** `main.rs:5688-5703` documents a real,
   observed failure — at ~640 pt window width the toolbar wrapped *inside* a button and
   rendered "Measure ▾" as a one-character-per-line vertical column, inflating the whole
   toolbar and pushing the History group out of the panel. The fix (`TextWrapMode::Extend`)
   moves the break to the control boundary; it does not create width. `main.rs:6254-6257`
   states the intent to cap the toolbar *"at its existing six groups plus this."* That cap
   has since been broken by Add Text, Edit Objects, Measure and Redact.
   `icons.rs:1618-1624` records the chevron being shrunk from 16 pt to 10 pt specifically
   because *"toolbar width is no longer free... a 16pt chevron on four buttons is 64pt of
   width buying nothing."* Three separate, dated, independent admissions that the surface is
   full. The Backlog (forms, OCR, Bates, signing, PDF/A, page-organise, Inkscape-parity
   vector tools) is several times the current command count. **A flat row cannot hold it,
   and the alternative to tabs is nested menus, which is Acrobat's own worst habit and the
   thing pdfce exists to be better than.**

3. **It removes an entire class of placement argument.** Read `main.rs:6125-6159` — nine
   paragraphs of genuinely careful reasoning about where one Redact button goes, reconciling
   "standing rule 3 says keep it off the primary toolbar" against "rule 7 wants destructive
   actions discoverable," and landing on "one icon+label control at the END of the edit
   group is the minimum weight that satisfies both." That reasoning is good, and it is
   reasoning that **should not have had to happen**, because the surface offered exactly one
   shelf. With a `Protect` tab the question answers itself. Every future feature Pass
   currently pays this tax; a ribbon retires it.

4. **It gives contextual verbs a home, at exactly the moment pdfce is growing them.**
   Decisions 022 and 023 are, between them, building a selection model with per-kind verb
   sets (R112: *"a verb that does not apply to a kind is UNREPRESENTABLE"*) — page objects,
   ce dimensions, foreign annotations, text runs, nodes, containers, each with a different
   and growing command list. **A per-kind verb set is precisely what a contextual tab is
   for.** Without one, those verbs go into either a right-click menu (invisible, and pdfce
   has no context menu today beyond the one an in-flight Pass is adding), the dock (already
   at four panels and A.3-capped at two tabs per default group), or the flat toolbar
   (full). The ribbon arrives with a use case waiting for it rather than as a reskin.

5. **It is the right shape for the eventual web fork.** `ARCHITECTURE.md` §3's whole point
   is that `pdfce-web` is a shell-crate swap. A ribbon is a `Panel::top` full of ordinary
   egui widgets — it compiles to WASM exactly as the toolbar does. Nothing in this decision
   costs the fork anything, and a tabbed command surface is *more* portable to a narrow
   browser window than a wrapping row is, because the tab set degrades to a dropdown while
   a wrapping row degrades to five stacked rows.

### 2.3 What is given up — named, not minimised

1. **Canvas height, permanently.** ~110 pt for tab strip + band, versus ~34 pt for today's
   single row. On the 1100 × 800 default window that is ~10 % of the height. Collapse
   (§3.6) makes it recoverable, not free, and a collapsed ribbon that must be re-expanded
   to reach a command is *slower* than a toolbar. **This is the real price.**

2. **One click for a command that used to be zero.** Any command not on the active tab
   costs a tab click first. Office solves this by putting genuinely-everyday commands on
   `Home` and on the QAT. pdfce's zoom/navigation controls are used constantly and
   **must not** end up behind a tab switch — which is why §3.2 puts them on both the QAT
   row and the View tab, and §8 flags the alternative (a permanent, un-tabbed zoom cluster
   in the status bar, the way every browser and most viewers do it) as a real option.

3. **A visibly sparse surface, with the obvious remedy forbidden.** R83 forbids greying in
   placeholders for unbuilt features; §7's proposed R124 makes that explicit for the ribbon
   specifically, because a ribbon's free space *invites* the violation in a way a full
   toolbar row does not.

4. **A second hand-rolled layout surface to maintain, with no prior art anywhere.**
   `dock.rs` is 599 lines wrapping one dependency that did the hard part. A ribbon module
   wraps nothing, and §5.1's sweep found no implementation in any Rust GUI ecosystem to
   learn from. Every egui upgrade now has two bespoke surfaces to re-verify instead of one.

5. **~60 new `ui_text.rs` entries** (tab names, group captions, every command's label and
   purpose tooltip), all subject to R1's single-catalog rule and the `check-ui-strings.sh`
   gate, and all subject to R3's +50 % width budget as fixed-position text. Not hard; not
   nothing.

6. **The accessibility role gap, doubled — and it is a collapse, not an omission.** §5.3.
   Ribbon tabs will reach a screen reader as `Role::Button` with a correct pressed state,
   exactly as `egui_tiles`' dock tabs do, for the same verified reason.

### 2.4 Alternatives considered and rejected

**(a) Keep the flat toolbar; fix only the Accept/Reject box.** Rejected as the *whole*
answer, adopted as the *first* answer. It is Pass 24.0. It addresses the sharpest complaint
and none of the trajectory problem in §2.2 item 2, and it does not answer what the operator
actually asked for twice. Shipping only this and calling the request served would be the
kind of partial compliance that produces the same message again in a month.

**(b) A conventional menu bar + a small toolbar (the pre-2007 Office model).** Genuinely
viable, cheaper than a ribbon, and egui 0.35 ships a real `MenuBar`/`MenuButton`/`SubMenu`
API to build it with (`egui-0.35.0/src/containers/menu.rs:217-426`), whose own doc comment
recommends putting it in a `Panel::top`. Rejected because the operator named the ribbon
specifically, and because a menu bar hides every command behind a label — which for a
*drawing* tool, where the operator's hands are on the page, is a worse trade than a band of
visible buttons. Recorded as the fallback if §6 stalls: Pass 24.1's `File` menu is built on
this API anyway, so a partial retreat to (b) is cheap.

**(c) A vertical icon rail (the VS Code / Blender-sidebar model).** Rejected on the same
grounds decision 017 §3 originally rejected horizontal tabs *for the dock*, in reverse: a
vertical rail spends width, and pdfce already spends 380 pt on the dock
(`DOCK_DEFAULT_WIDTH_PTS`, `main.rs:326`) plus the thumbnail rail on the left. A CAD drawing
is landscape; horizontal chrome is the correct axis to spend on. It also is not what was
asked for.

**(d) Adopt a ribbon *and* move the dock into it (one `egui_tiles` tree owning everything).**
Rejected — §3.4. Also structurally unavailable: `egui_tiles` 0.16.0 has **no command-surface
concept at all** — a grep of its source for `toolbar`/`ribbon`/`menubar` returns zero hits,
and it does not even reference `egui::Panel`. It is a pane-tiling tree and nothing else.

**(e) Make the ribbon optional, with a "classic toolbar" mode.** Rejected firmly. Decision
017 §8.3 already refused a float-OR-dock dual mode with the exact reasoning that applies
here: *"two code paths for the same content, duplicating open-state, position/size and
focus handling, for zero operator benefit at this scale."* Two command surfaces is that,
squared — every future feature Pass would have to place its command twice, and the two
would drift. If the ribbon is wrong, revert it; do not keep both.

---

## 3. Q2 — The concrete target shape

### 3.1 The layout, whole

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ [File ▾] │ ⌸ ⌷ ↶ ↷ │  Home  Insert  Edit  Measure  Protect  View │ ‹ce dim›  │  ← row 1: File + QAT + tab strip (+ contextual tabs, right)
├──────────────────────────────────────────────────────────────────────────────┤
│  ┌─ Clipboard ─┐ ┌─ Select ─┐ ┌──── Pages ────┐ ┌─ History ─┐                │  ← row 2: the active tab's BAND
│  │   [Copy▾]   │ │  [Obj]   │ │ [↺] [↻] [Del] │ │ [↶] [↷]   │                │     named groups, captions beneath
│  └─────────────┘ └──────────┘ └───────────────┘ └───────────┘                │
├──────────────────────────────────────────────────────────────────────────────┤
│ ▸ Measure · Linear    47.25 mm  (1:50)          [ ✔ Accept ] [ ✘ Reject ]     │  ← row 3: TOOL STRIP — only while a tool is armed (Pass 24.0)
├────────┬────────────────────────────────────────────────────┬────────────────┤
│ thumbs │                      canvas                        │  egui_tiles    │
│  rail  │                                                    │  dock          │
│        │                                                    │  (unchanged)   │
├────────┴────────────────────────────────────────────────────┴────────────────┤
│ status bar (unchanged)                                                        │
└──────────────────────────────────────────────────────────────────────────────┘
```

**Panel add order becomes:** `ribbon` (top) → `toolstrip` (top, conditional) → `status`
(bottom) → `thumbnails` (left) → `dock` (right) → `CentralPanel`. This preserves the
load-bearing property `main.rs:87-123` documents at length: **the full-width bottom panel is
added before any side panel**, so the status bar still spans the window. The tool strip is a
*second* top panel added immediately after the ribbon, which is the only position that makes
it span the full width and sit directly under the ribbon. egui's own `Panel` doc comment
states the governing invariant — *"The first panel you add will always be the outermost…
⚠ Always add any `CentralPanel` last"* — which the existing order already honours and this
one continues to.

**Height budget, stated so it can be measured against (R86):** tab strip ≈ 28 pt, band
≈ 68 pt (one 44 pt large-button row + a 14 pt caption row + padding), tool strip ≈ 30 pt
when present. Total ≈ 96 pt idle, ≈ 126 pt with a tool armed, against today's ≈ 34 pt.
**These are estimates, not measurements** — the first Pass must report the real numbers,
because the whole §2.3 item 1 trade turns on them.

### 3.2 (a) The fixed tabs, and what lives on each

**The tab taxonomy is derived from pdfce's own settled placement question — *what does this
command act on?* — and NOT from any other product's menu structure.** CLAUDE.md rule 12
forbids the Acrobat RAG from informing GUI structure; R61 forbids the same for Inkscape.
This is the ribbon's expression of that rule and §7 proposes it as R123.

The organising axis already exists in the codebase. `main.rs:5734-5736` and `:5850-5855`
name it explicitly: view-state commands ("govern what is on screen rather than the
document") are one group, and edit commands are another because *"mixing the two would make
'does this button change my file?' unanswerable at a glance."* The tabs are that question,
asked with more room.

| Tab | Organising question | Groups → commands (shipped today unless marked) |
|---|---|---|
| **File** *(a menu button, not a tab — §3.5)* | "What do I do with the file as a whole?" | Open · Save · Save As… · *(Recent — not built)* · Combine files… · Split document… · Insert pages from file… · Font folders… · Keyboard shortcuts · Exit |
| **Home** | "What do I do most, on any document?" | **Clipboard**: Copy text ▾ (page / document) — **Select**: Obj tool *(the pointer; its verbs live on contextual tabs, §3.3)* — **Pages**: Rotate ↺ / ↻, Delete, Extract, Move up/down *(today only on the thumbnail rail — promoting them is a real gain, not a move)* — **History**: Undo, Redo |
| **Insert** | "What am I ADDING to the document?" | **Text**: Add Text — **Markup**: Rectangle, Ellipse, Arrow, Highlight, pen colour — **Notes**: FreeText box, Sticky note, Stamp — **Pages**: Insert pages from file… — *(future: Image, Link, Form fields, Bates)* |
| **Edit** | "What am I CHANGING that is already there?" | **Text**: Edit Text, Reflow *(reflow is currently only reachable from inside the Text Edit floating bar)* — **Objects**: Edit Objects (vector move / delete / drag-node) — *(future: Crop, OCR, Optimise)* |
| **Measure** | "What am I measuring, and in what units?" | **Tools**: Linear, Circular, Set scale — **Groups**: active ce-dimension group picker, Manage groups…, layer visibility toggle — **Format**: unit, style, denominator, reduce *(Pass 23.0 — currently unreachable capability, R117)* — **Snapping**: snap master toggle, alignment constraint |
| **Protect** | "What am I removing or restricting?" | **Redact**: Mark page, Search & mark…, Review & apply… — *(future: Encrypt, Permissions, Sanitise, Sign, Certify)* |
| **View** | "What is on my screen?" | **Zoom**: −, %, +, Fit page, Fit width, 100 % — **Show**: thumbnail rail, annotations, layers/OCG — **Panels**: Objects, Properties, Batch Tools, Redact, *Reset panel layout* — **Ribbon**: collapse |

**Why `Measure` earns a whole fixed tab** — and this is a deliberate, arguable promotion.
Decision 011 made the scaled-measurement / ce-dimension subsystem the **first beta**, and
decision 023 is currently adding re-measure and the format surface to it. It is pdfce's
genuine parity-plus feature and the operator's own primary use case (CAD drawings). Today
it is one dropdown button competing for width with Copy-text.
`docs/ui_specs/pass-12.M2-dimension-tools.md` §1.2 chose that dropdown with explicit
reasoning — *"dimensioning is used in short deliberate bursts, so it earns a menu, not
primary-icon creep"* — and that reasoning was correct **given a flat toolbar with no room**.
A tab has room, and "short deliberate bursts" is exactly what a tab is for: you go to the
tab, you work, you leave. The prior reasoning is not overturned; its premise is.

**Why `Protect` rather than putting Redact on `Edit`.** §1.3 item 1's nine-paragraph
placement argument. Redaction is not an edit — it is the one operation the round-trip
invariant carves out an exception for (`ARCHITECTURE.md` §5.2, R35), it forces a full
rewrite, and it is irreversible after save. Putting it in the same band as "move this line"
mis-states its weight. It also gives encryption, permissions and signing an obvious home
before they are built, which is what stops the next feature from paying §2.2 item 3's tax.

**Rejected tab schemes, for the record:**
- **By document lifecycle (`Review` / `Prepare` / `Share`)** — Acrobat's own shape. Refused
  on rule 12 grounds *and* on merit: it groups by workflow the operator may not be in, so
  "where is rotate?" has no answer derivable from first principles.
- **One tab per tool.** Contextual tabs already do this, correctly, and only when the tool
  is armed.
- **A `Tools` tab holding the batch operations.** They are file-scope, they are already a
  dock panel with multi-file forms, and duplicating their entry points is exactly the
  two-mental-models problem §1.3 item 6 identifies. They live on `File`, opening the
  existing `BatchTools` dock panel.

### 3.3 (b) Contextual tabs

Two families, keyed on two different pieces of state, and **the distinction matters** —
conflating them is how contextual tabs become unpredictable:

**Family A — TOOL tabs, keyed on `doc.active_tool: Option<CanvasTool>`.** These replace the
floating property bars entirely. One tab per armed tool; it appears when the tool is armed,
auto-activates, and disappears when the tool is put away. Its rightmost group is always
**Finish** — the fixed-anchor Accept/Reject (§4.2) — so the confirm is in the same place for
every tool, forever.

| `CanvasTool` | Contextual tab | Groups (all content already exists, floating) |
|---|---|---|
| `MeasureLinear` / `MeasureCircular` / `MeasureScale` | **Measure Tool** | Constraint (aligned/H/V) · Snapping · ce-dimension group · Display (radius/diameter) · Scale entry · **Finish** |
| `AddText` | **Add Text** | Font family/size · Colour · Placement mode (point/box) · **Finish** |
| `TextEdit` | **Edit Text** | Font family/size/style (Pass 19.3's whole surface) · Spacing (character/word/horizontal scale/rise) · Block overlay toggle · Reflow · **Finish** |
| `VectorEdit` | **Edit Objects** | Level (breadcrumb + up/down, Pass 23.2) · Node ops (Pass 23.3) · **Finish** |

**Family B — SELECTION tabs, keyed on the kind of `TargetId` in `doc.canvas_selection`.**
These carry the per-kind verb set R112 requires. **All of Family B is blocked on Passes 22.0
and 23.2** and is sequenced accordingly (§6.4).

| Selection kind | Contextual tab | Groups |
|---|---|---|
| `TargetId::Content` (path / text run / image / form XObject) | **Object** | Arrange: move, delete *(shipped, Pass 9c-min)* · Level: enter / leave / breadcrumb *(Pass 23.2)* · Nodes: select, move, delete *(Pass 23.3)* · Info: read-only fill/stroke/bbox readout *(R83 — read-only until an edit path exists)* |
| `TargetId::Annot` on a **pdfce-authored ce dimension** | **Dimension (pdfce)** — see the label note below | Measurement: re-measure, move *(Pass 23.1)* · Group: active group, scale, layer visibility · Format: unit / style / denominator / reduce *(Pass 23.0)* · Delete *(Pass 22.0)* |
| `TargetId::Annot` on a **foreign / markup annotation** | **Annotation** | Colour · Delete *(Pass 22.0)* · *(move: still deferred, decision 022 §6)* |
| A text run selected inside the Text Edit tool | *(no separate tab — it is the `TextEdit` tool tab, Family A)* | — |

**On the ce-dimension tab's LABEL, and rule 15.** A contextual tab reading just
"Dimension" is precisely the ambiguity rule 15 exists to prevent: on a CAD drawing the
operator is looking at dozens of **pdf dimensions** and a handful of **ce dimensions**, and
a tab that appears when he clicks one and says "Dimension" tells him nothing about which
kind he has hold of — while its verbs (re-measure, re-scale, delete) apply *only* to ce
dimensions and would be alarming if he believed they applied to the pdf dimensions printed
on the sheet. **The tab must name its provenance.** Recommended string:
`Dimension (pdfce)`, with the purpose tooltip stating in full that these commands act on
dimensions pdfce authored and never on dimensions already drawn into the page. `ui_text.rs`
is the single catalog for it (R1). Selecting a **pdf dimension** — which is page content,
not an annotation — correctly raises the **Object** tab instead, and that asymmetry is
itself the disclosure: the two kinds get visibly different verb sets, which is the clearest
possible statement that they are different things.

**Contextual-tab discipline — four rules that make them predictable rather than jumpy:**

1. **A contextual tab auto-activates on appearance, and restores the previously-active fixed
   tab on disappearance.** Office's behaviour, and the only one that does not lose the
   operator's place. **Caveat verified in the crate source:** egui drops keyboard focus to
   `None` when a focused widget stops being drawn (`memory/mod.rs:617-624`), so an operator
   keyboard-focused inside a band that disappears loses focus entirely. Pass 24.5 must
   re-request focus deliberately rather than let it fall on the floor.
2. **A contextual tab is visually distinguished** from a fixed tab — and per **R84**, never
   by colour alone. Recommended: a leading `▸` marker plus a bold label plus a tint. The
   marker is what survives greyscale and colour-vision deficiency.
3. **At most ONE tool tab and at most ONE selection tab exist at a time.** A tool and a
   selection can coexist (you can have the Obj tool armed with something selected), so two
   contextual tabs may be present; three may not. Enforced structurally by keying them on
   `Option<CanvasTool>` and on one selection *kind*, not by discipline.
4. **A mixed-kind selection raises NO selection tab**, and the status readout says why.
   R112's exhaustive-match discipline means a verb set for "some paths and an annotation"
   does not exist; inventing a lowest-common-denominator tab would be the half-polymorphic
   verb set R112 exists to forbid.

### 3.4 (c) Where the dock panels go: **nowhere. They stay.**

**The `egui_tiles` dock is untouched by this decision.** Objects, Properties, Batch Tools
and Redact remain exactly where they are, in the same tree, with the same default layout and
the same R80–R84 discipline. Four reasons:

1. **Nothing in the operator's message is about the dock.** He asked for a ribbon and
   complained about a floating box. Decision 017's Amendment A records that he chose the
   *widest* docking model available and named Inkscape's flexibility as the target. Tearing
   that out three days later, unasked, would be the engineer substituting his own taste for
   a stated preference.

2. **Both reference products have a ribbon AND persistent side panels.** Office has the
   ribbon plus the Navigation/Styles/Comments panes. SolidWorks has the CommandManager
   (ribbon) plus the FeatureManager tree and the PropertyManager, docked left, permanently.
   **A ribbon is not an alternative to a dock; they are orthogonal surfaces** — the ribbon
   carries *commands*, the dock carries *state you keep looking at*. R81 already draws
   exactly this line ("anything an operator keeps open while working on the document is a
   dock panel"), and it survives this decision unchanged.

3. **The dock earns its keep in ways a band cannot.** The Objects tree is unbounded (a page
   can decompose to thousands of rows), the Properties form is a multi-field form, the
   Redact panel is a scrollable mark list with an R99-ordered action. None of those fit in a
   ~68 pt horizontal band. Amendment A's simultaneity requirement — Objects/Layers visible
   *at the same time* as Properties, because that is how every vector editor works — is a
   vertical-space requirement, and the ribbon has none to give.

4. **Two large surfaces rewritten at once is the classic way this fails.** §6 keeps the dock
   as the stable reference point the ribbon migration is measured against.

**What the dock DOES gain:** one toggle per panel in the View tab's **Panels** group, with
uniform semantics (open the dock if closed, activate that panel), plus `Reset panel layout`.
This normalises §1.3 item 6's two-mental-models problem, and it satisfies R80's spirit at a
second surface — every panel is reachable from the command surface, not just the two that
happened to get toolbar toggles.

**The one thing this record explicitly does NOT decide:** whether the `egui_tiles` tree
should eventually own the *canvas region* too, with open documents as panes — decision 017
§10 Q1's "wide model," which the operator answered *yes* to and which has never been built.
That remains a live, answered, unbuilt item. The ribbon neither helps nor hinders it: a
`DockPanel::Document(DocId)` variant is still a non-breaking addition, exactly as `dock.rs`'s
own doc comment guarantees. Flagged as operator question (af).

### 3.5 (d) Quick Access and file operations

**`File` is a menu button at the far left of the tab strip row — not a tab, and not a
Backstage.**

- **Not a tab**, because a File *band* would be a row of large buttons for commands that are
  used once at the start and once at the end of a session; that is the worst possible use of
  permanent vertical space.
- **Not a Backstage** (Office's full-window takeover). It is a large, expensive, modal
  surface whose value is print preview / account / export panes pdfce does not have. Building
  one now would be affordance without capability at the *screen* level.
- **A menu**, built on egui 0.35's real menu API — `egui::containers::menu::{MenuBar,
  MenuButton, SubMenuButton, SubMenu}`, verified present at
  `egui-0.35.0/src/containers/menu.rs:217-426`, re-exported at the crate root
  (`lib.rs:464`), and carrying a doc comment that literally recommends
  *"The menu bar goes well in a `crate::Panel::top`."* `MenuButton::from_button` lets a
  ribbon-sized button carry menu behaviour, which is also what group-collapse (§3.6) needs.
  pdfce already uses `menu_button_labeled` / `menu_button_atoms` for four dropdowns, so the
  widget idiom, the chevron affordance (`icons::menu_chevron`, and the `▾`-is-tofu finding
  behind it) and the accessible-name pattern all already exist and are reused, not
  re-derived.

**Quick Access Toolbar (QAT): yes, inline on the tab-strip row, to the left of the tabs,
fixed contents.**

Contents: **Open · Save · Undo · Redo**. Four controls, chosen because each is (a) used from
any tab, (b) already keyboard-bound, and (c) meaningless to hunt for. `Save` keeps its
existing hidden-when-no-document behaviour (`main.rs:5717-5731` — *"hidden rather than
disabled when nothing is open — there is nothing to discover about saving with no
document"*), and Undo/Redo keep their existing disabled-not-hidden behaviour
(`main.rs:6173-6180` — *"the absence of an Undo control and a greyed-out one say different
things"*). **Both existing behaviours are preserved verbatim**, because both are reasoned
and dated, and a migration that quietly normalised them would be discarding thought.

**The QAT is NOT user-customisable.** Office's is; pdfce's must not be, yet — customisation
means persisted user state, which rides **R82/R15** (a named partition of the distribution
folder, never eframe's platform `Storage`, per decision 003's portable posture). Until R15
lands there is nowhere legitimate to persist it, and a customisation UI that forgets on
restart is worse than none. Named as a follow-up, refused for now, for a stated reason.

**Where the status summary goes.** Today `toolbar()` pins `status_summary()` to the toolbar
row's right edge via an outer right-to-left layout (`main.rs:5666-5675`, `:6278-6281`), with
a comment explaining that this is deliberate so it *"cannot join the wrap flow and drift
leftward as future Passes append tool groups."* **Keep it, on the tab-strip row's right
edge**, using the same right-to-left outer layout. The reasoning transfers intact and the
tab strip is the row that will never wrap.

### 3.6 Collapse, overflow, and the two failure modes that must not be repeated

**Collapse — and egui gives this away free.** Double-clicking the active tab collapses the
band, leaving the tab strip; a single click on any tab then shows that band as a transient
overlay which auto-collapses after a command. This is Office's exact model and it is the
only mitigation for §2.3 item 1. **`egui::Panel` 0.35 ships `show_switched` (animate between
a thin/collapsed and a thick/expanded panel) and `show_collapsible` (slide animation)** —
verified in `containers/panel.rs` — which is precisely this primitive, already written and
already animated. `Ctrl+F1` toggles it (the Office chord; pdfce binds no F-keys, so there is
no collision). **Collapse state is session-only and disclosed**, the same stance decision
017 §7 / R82 takes for dock layout and decision 012 set for font folders.

*(Implementation note worth recording so it is not rediscovered: `Panel::top` and
`Panel::bottom` default to `.resizable(false)` — `containers/panel.rs:239`. If the ribbon
should be operator-resizable, that must be opted into explicitly. Recommendation: leave it
non-resizable; a ribbon with an operator-chosen height is a layout with two ways to be
wrong.)*

**Overflow: a band that does not fit must NEVER hide commands behind scroll arrows.**
This is not a preference — it is the *named failure mode* decision 017 §4 identified in
`egui_tiles` 0.16.0 and Amendment A elevated to an invariant: *"Treat the appearance of
tab-bar scroll arrows in the DEFAULT layout as a defect report, not as normal,"* enforced by
`dock.rs`'s `no_default_tab_group_holds_more_than_two_panes` test. **The same rule binds the
ribbon.** `ScrollArea::horizontal()` exists and is deliberately **not** used. The correct
degradation is Office's: as width runs out, groups collapse **whole**, right-to-left, each
becoming a single labelled `MenuButton` containing its commands. A collapsed group is still
visible and still named; a scrolled-off command is not.

Until group-collapse is built (Pass 24.4), the interim mitigation is the same shape as
`dock.rs`'s: **a test asserting no tab's band exceeds a stated control budget**, so a future
Pass cannot quietly overfill a band and rediscover the failure. The budget is a measured
number, produced by Pass 24.1 against the real minimum window width, not guessed here.

**The second failure mode, from this project's own history:** `main.rs:5688-5703` and
ROADMAP continuation-76's finding that six buttons using `add_sized(ICON_BUTTON_SIZE, ..)`
rendered "Place point" as `Pla`/`ce`/`poi`/`nt` — *"a test can prove a character has a glyph;
only looking proves the operator can read the button."* A ribbon's large icon-over-label
buttons are **exactly** the widget shape that bites: a two-word label under a 32 pt icon in a
fixed-width cell. `ICON_BUTTON_SIZE`'s doc comment already states the rule
(`.min_size()`, never `add_sized()`); the ribbon's large-button widget must follow it, and
**R86 observation of the widest real label at the narrowest supported width is a ship
condition for Pass 24.1**, not optional polish.

---

## 4. Q3 — The Accept/Reject problem, and the narrowing of rule 4

### 4.1 The complaint is about placement — and SolidWorks proves it

The operator wrote *"I've never seen any other software operate that way."* Taken at face
value that reads as "confirm steps are unusual," which is not true and which, if acted on
naively, would delete disclosures pdfce is obliged to make.

**SolidWorks — one of the two products he named — has an Accept/Reject pair on essentially
every modal feature command:** the PropertyManager's ✓ (OK) and ✗ (Cancel) buttons, at the
**top-left of the PropertyManager panel**, in the same place for every command, docked, with
Enter and Escape as their keyboard equivalents. Office does the same thing in a different
idiom: Track Changes' Accept/Reject live on the **Review ribbon tab**, in a named group, in
the same place every time.

*(Stated so it can be checked: this is a claim about a product the operator uses daily. If
the PropertyManager's ✓/✗ are not where this record says, §4.2's tier model still stands on
its own merits, but this paragraph's argument for it does not. Verify before quoting it
back at him.)*

**So what is actually unprecedented in pdfce is not the confirm — it is that the confirm
box's position is a function of the document.** §1.2 measured it: `image_rect`-relative,
`LEFT_BOTTOM` pivot, moving on zoom, scroll and page change, drawn over page content. No
mainstream application positions a command control against the *document*. That is the
defect, it is narrow, and it is fixable without touching a single disclosure.

**This reframing is what makes the rest of this section possible.** "Remove the confirm" and
"move the confirm" are very different instructions, and only one of them is compatible with
rule 4.

### 4.2 The replacement model — three tiers

The engineer's dispatch listed four candidate mechanisms. The answer uses three of them,
tiered by a single question: **did pdfce infer anything the operator did not directly
specify?**

---

**Tier 1 — Direct manipulation: commit on gesture completion; undo is the escape hatch.**

*Applies when the gesture's result is fully visible on the canvas, fully determined by what
the operator did, and reversible in exactly one undo step.*

Members today: object move, object delete, node drag (Pass 9c-min); markup shape authoring;
page rotate / delete / reorder; annotation delete (Pass 22.0); whole-ce-dimension move
(Pass 23.1's pure translate, whose measured value is **invariant** — decision 023's
acceptance criterion B2 asserts a byte-identical value after a move).

Mechanism: mouse-up commits. No confirm control exists, because there is nothing to review —
the operator specified the result with his hand and can see it. `EditSession`'s command log
(`ARCHITECTURE.md` §11) already makes each of these one undoable command, so Ctrl+Z is a
complete and honest escape hatch.

**This tier is where the "no other software works that way" complaint is fully satisfied**,
because it is how other software works.

---

**Tier 2 — Inference under review: a two-stage commit at a FIXED anchor.**

*Applies when pdfce computed something the operator did not directly specify, or when a
disclosure must be read before the result becomes document state.*

Members today, with what is being reviewed in each:

| Operation | The inference / disclosure under review |
|---|---|
| ce-dimension authoring (linear) | The **snapped** point may not be the clicked point; the measured value and its scale context; `NO_SCALE_DISCLOSURE` when no scale is set |
| ce-dimension authoring (circular) | A **best-fit circle** over N picked objects, with a residual — `best_fit_residual_high()` exists precisely because the fit can be bad |
| Set scale | The derived ratio, previewed before it re-propagates to every member of the group |
| Derived centerline | `measure_confirm_derived_centerline()` — an explicitly fuzzy inference, already flagged in warn colour |
| ce-dimension **re-measure** (Pass 23.1) | **R119 mandates it**: old value, new value, delta, and the group/scale it was computed under, all on screen before commit; mouse-up is never the commit |
| In-place text edit | The **font trust ladder** (R71) — which face pdfce will use, and whether it is synthesising a style |
| Reflow | R72/R75/R76 — a recognised block is a *reviewable hint*; overflow discloses |
| Add Text (box mode) | The live wrap preview and its overflow disclosures |
| Redaction apply | Already correctly a blocking confirmation with a computed result (R98) — **unchanged by this record** |

Mechanism: **the tool strip** (§3.1 row 3) — a full-width `egui::Panel::top` added
immediately after the ribbon, present only while a tool is armed or a pending result exists.
It carries, left to right: the tool/operation name, the live readout and disclosures, and
**at a fixed right anchor, `✔ Accept` and `✘ Reject`.**

Six properties, each load-bearing:

1. **Its position is a function of the WINDOW, never of the page.** This is the entire fix.
2. **It is the same position for every tool**, so the confirm is somewhere the hand learns
   once.
3. **It never covers document content**, because it is a panel, not an overlay — egui's
   panel layout shrinks the central region around it, exactly as the toolbar and status bar
   already do.
4. **It cannot drift off screen** on zoom, scroll or page change.
5. **It orders state → action → detail**, which is **R99** applied at a second surface:
   readout first, Accept/Reject next, verbose disclosures last, so the action is never below
   the fold of a long disclosure list. The Redact panel learned this the hard way in Pass
   8.1; the tool strip inherits the lesson instead of repeating it.
6. **Once contextual tool tabs exist (Pass 24.2), the strip's controls become the tool tab's
   `Finish` group** — same widgets, same anchor rule, one row higher. The strip is therefore
   not throwaway scaffolding; it is the confirm's permanent home, drawn in a temporary
   container until the tab exists.

**Rejected for Tier 2 — on-canvas handles at the gesture (a floating ✓/✗ near the cursor).**
The engineer's dispatch listed it. It is refused: it is a *smaller* floating box in a *less*
predictable place, it occludes the geometry being measured (the exact complaint
`main.rs:11813-11817` already records for the property bar), and it cannot carry the
disclosure text Tier 2 exists to display. **On-canvas handles are right for direct
manipulation of geometry** — drag an endpoint, drag a node — and Pass 23.1 should have them
for that. They are wrong as a *command* surface.

---

**Tier 3 — Keyboard, universal, over both tiers.**

- **Enter commits** the pending gesture wherever one exists. Today this is implemented in
  exactly one tool (Add Text, `main.rs:9959-9970`) and nowhere else. It becomes the rule.
- **Escape steps back**, through the existing `canvas::resolve_escape` precedence chain —
  which is already correct, already tested, and about to become a named-field `EscapeContext`
  under **R120**. Nothing here changes its semantics; this record only requires that every
  tool's Reject route through it rather than each tool having a private reject path.
- **Ctrl+Enter commits** where plain Enter is meaningful inside the gesture (Add Text's box
  mode already does exactly this — the existing convention is promoted, not invented).

Tier 3 is what makes Tier 2's mouse trip **optional**. An operator who never wants to look
at the tool strip never has to: measure, glance at the readout, Enter. That is the workflow
SolidWorks users have in their fingers, and it is currently impossible in pdfce for two of
the three tools.

### 4.3 What this changes in the code, concretely

| Today | After Pass 24.0 |
|---|---|
| `egui::Area` "pdfce-text-edit-status" at `image_rect`-relative pos (`main.rs:9364`) | removed; content moves to the tool strip |
| `egui::Area` "pdfce-add-text-status" (`main.rs:10242`) | removed; ditto |
| `egui::Area` "pdfce-measure-status" (`main.rs:11940`) | removed; ditto |
| `egui::Area` "pdfce-text-edit-propbar" (`main.rs:8854`) | stays floating until Pass 24.2, then becomes a contextual tab |
| `egui::Area` "pdfce-add-text-propbar" (`main.rs:10120`) | ditto |
| `egui::Area` "pdfce-measure-propbar" (`main.rs:11825`, movable + closable) | ditto |
| Enter commits in Add Text only | Enter commits in all three |
| Reject is a per-tool `do_reject` flag | Reject routes through `resolve_escape`'s `CancelGesture` |

**Note the split:** Pass 24.0 moves the **status/confirm** strips (the operator's actual
complaint) and leaves the **property** bars floating. That is deliberate — the property bars'
correct destination is a contextual ribbon tab, which does not exist yet, and moving them to
an interim second location would be two migrations instead of one. The property bars are
already `movable(true)` as of 2026-08-04, so they are survivable in the interim; the confirm
strips are not, and they go first.

### 4.4 Reconciling with rule 4 — the narrowing, proposed in full

**Rule 4 as written** (`CLAUDE.md`):

> **Fuzzy, never sneaky.** Every algorithmic suggestion (OCR text, auto-detected form
> fields, suggested Bates ranges) is a reviewable hint the operator accepts or overrides —
> never a silent auto-apply.

**The rule is right. Its application is what drifted**, in a way worth naming precisely
because it is a general failure mode:

Rule 4 says a *suggestion* must be *reviewable*. Somewhere between Pass 12.M2, Pass 14.3 and
Pass 16.2, three tools independently converged on the same implementation of "reviewable" —
a floating box with two buttons — and each cited the tool before it as precedent. The
ROADMAP records this convergence approvingly: *"Placement answered by PRECEDENT — the
TextEdit tool already has a floating tool-scoped property bar, and AddText and Measure
independently converged on the identical pattern."* **Convergence by precedent is not the
same as convergence on the right answer**; three tools agreeing does not make a floating box
correct, it makes it habitual. The rule never asked for a box. It asked that the operator be
able to see and reject what pdfce inferred.

Two distinct over-applications resulted:

- **Over-application A — Tier-1 operations got a confirm they never needed.** Nothing about
  a straight click-click-Accept linear measure with snapping *off* is an inference. The
  operator picked two points; pdfce reported the distance between them. Requiring a trip to
  a floating box to bless arithmetic he specified is friction rule 4 does not ask for.
- **Over-application B — the confirm CONTROL was conflated with the review CONTENT.** The
  review content (the value, the scale disclosure, the residual, the trust ladder) is what
  rule 4 requires. The two buttons are one possible commit mechanism among several, and the
  weakest one — because a keyboard commit is *more* deliberate than a mouse click, not less,
  and both are deliberate.

**Proposed narrowed wording** (for `CLAUDE.md` rule 4 — **the operator's file, so this is a
proposal, not an edit**; §8 item 1):

> **4. Fuzzy, never sneaky.**
>
> Anything pdfce **inferred** — a value, a boundary, a classification, a correction the
> operator did not directly specify (OCR text, auto-detected form fields, recognized text
> blocks, snapped points, best-fit geometry, derived centerlines, reflow results, suggested
> Bates ranges, substituted or synthesised fonts) — is **visible before it becomes document
> state**, and the operator can reject it without undoing anything else.
>
> This is a requirement on **disclosure**, not on any particular widget. It is satisfied by
> the inferred value being on screen and the commit being a deliberate act — a key press or
> a click on a control at a fixed, predictable position. It is **not** satisfied by a
> control whose position is derived from the document, and it does **not** require a
> two-click confirmation for a direct manipulation whose result is fully visible on the
> canvas and reversible in one undo.
>
> Where an inference is *inherently* uncertain (a best-fit residual, a font-trust
> downgrade, a reflow that overflows), the uncertainty is stated in the disclosure, not
> merely implied by the presence of a confirm button.

**What this narrowing does NOT weaken — checked, item by item, against every rule that
depends on rule 4:**

| Rule | Depends on rule 4 for | Still holds? |
|---|---|---|
| **R119** (ce-dimension geometry change is two-stage; mouse-up is never the commit) | The entire rule | **Yes, unchanged.** Re-measure is a Tier-2 inference; old→new→delta→scale all display in the tool strip, and Enter or a fixed-anchor Accept is the commit. R119's own words are about *silence*, not about a floating box: decision 023 §9 item 5 says explicitly *"the argument was always about SILENCE, not about whether re-measure should exist."* |
| **R72 / R75 / R76** (recognized blocks and reflow are reviewable hints; overflow discloses) | Review of a recognition | **Yes.** Reflow stays Tier 2; its diagnostics move to the strip and gain R99 ordering. |
| **R71** (font-on-edit trust ladder) | Disclosure before commit | **Yes.** Tier 2. |
| **R90** (synthetic bold/italic is per-use, declinable) | A declinable offer | **Yes.** Decision 019 already found the refusal strip *is* an honest declinable offer; it moves, unchanged. |
| **R98** (compute the real result and present it as the confirmation content) | Redaction apply | **Yes, untouched.** Redaction apply is a blocking modal and stays one. Tier 2 is not a downgrade path for it. |
| **R118** (a display-format preference never rides the gesture that first set it) | Decoupling | **Yes** — and the ribbon *helps*: format lives in the Measure tab's Format group and on the ce-dimension contextual tab, not in the scale-entry dialog. |
| **R83** (no affordance without capability) | — | **Yes**, and §7's proposed R124 extends it to the ribbon's empty space. |
| **R85** (preview-equals-saved) | — | **Untouched.** A pure GUI-shell change writes no bytes. |

**And what it deliberately DOES weaken, stated so nobody is surprised:** a Tier-1 operation
that today shows an Accept button will stop showing one. If the operator's mental model is
"pdfce asks before it changes my file," that model changes to "pdfce asks before it applies
something it guessed." That is a real behavioural change, it is the change he asked for, and
it is undo-backed. **Pass 24.0's tier assignment table must be reviewed by the operator, not
inferred by the engineer** — the boundary between "I specified this" and "pdfce guessed
this" is a judgement about *his* work, not a technical fact. Named as operator question (ac).

### 4.5 Sequencing note — this half does not wait for the ribbon

§4's model is implementable **today**, against the current flat toolbar, with no ribbon, no
new dependency, and no dependence on Passes 22.0 or 23.x. The tool strip is one
`egui::Panel::top` and three deletions. That is why it is **Pass 24.0** and why it is the
smallest Pass in this record. If the ribbon were abandoned entirely, §4 would still be the
right answer and would still ship.

---

## 5. Q4 — Feasibility in egui 0.35

### 5.1 Is there a credible crate? — **No. Verified, and the negative is unusually clean.**

| Probe | Result |
|---|---|
| crates.io API, `egui_ribbon` | **404 — does not exist** |
| crates.io API, `egui_toolbar` | **404 — does not exist** |
| crates.io full-text `ribbon` (86 results) | Zero egui-related — Bloom/ribbon-*filter* crates, an iterator crate, a healthcare API client, an iOS icon tool |
| crates.io `keyword=egui`, top 100 by downloads | No ribbon / toolbar / command-bar crate |
| crates.io `q=command bar egui ribbon tabbed` | **0 results** |
| GitHub repo search `egui ribbon` | **total_count = 0** |
| GitHub repo search `ribbon ui language:Rust` | **total_count = 0** |
| GitHub issue search `repo:emilk/egui ribbon` | **total_count = 0** — nobody has even asked upstream |
| GitHub Discussions search, `repo:emilk/egui ribbon` | **discussionCount = 0** |
| `emilk/egui` wiki, "3rd-party egui crates" | No ribbon / toolbar / command-bar entry |

**The one crate that looks like a hit is not one.** `fluent-ribbon` 2.0.2 (Apache-2.0)
advertises exactly the right feature list — *"Ribbon bar component: tabs, groups, contextual
tabs, overflow"* — and targets **GPUI** (Zed's framework), not egui. Its dependency list is
`fluent-core` / `fluent-primitives` / `gpui`. Unusable, and its provenance (published and
last touched inside a single day, 150 downloads) would not clear decision 017 §6.2's vetting
bar anyway.

Adjacent crates that exist and are *not* ribbons: `egui_tabs` 0.2.1 (MIT, but pinned to
**egui ^0.29.1** — six versions stale, does not build against 0.35), `egui_flex` 0.7.0 and
`egui_taffy` 0.13.0 (MIT layout helpers), `egui-elegance`, `egui-material3`, `egui-antd`
(widget sets, no ribbon/toolbar/app-bar), `egui-menu` 0.1.0 (2023, superseded by egui's own
0.32+ menu API), `egui_dock` 0.20.1 (docking — and permanently rejected by decision 017 §5).

**Licence sweep result: nothing GPL/AGPL/LGPL appeared anywhere in this survey.** Every
crate touched is MIT or MIT-OR-Apache-2.0. There is therefore **nothing to flag under
CLAUDE.md rule 13** — which is close to moot anyway, since the answer is "build it," and a
hand-rolled ribbon adds **zero** new packages.

**Recommendation: hand-build, no new dependency.** This is consistent with how this project
has actually decided such things: `egui_tiles` was adopted because tiling with drag, split
and simplification is genuinely hard and easy to get subtly wrong. A tab strip plus a row of
grouped buttons is neither. **And note the corollary of the zero-prior-art finding: pdfce
would be the first ribbon in the egui ecosystem.** There is no reference implementation to
crib overflow maths, contextual-tab state machines or collapse behaviour from. That is the
strongest argument for §6's six-Pass staging over a two-Pass big bang.

### 5.2 What egui 0.35 actually gives you — verified against the pinned source

Verified by reading `~/.cargo/registry/src/*/egui-0.35.0/`, not from memory:

| Primitive | Status | Ribbon use |
|---|---|---|
| `egui::Panel::{top,bottom,left,right}` (`containers/panel.rs:180`, ctors `:223-248`) | present. **`TopBottomPanel`/`SidePanel` were REMOVED — not deprecated — in egui 0.34** (upstream PR #5659); pdfce already uses `Panel::top`, so this is a warning about copied snippets, not a migration | the tab strip + band row, and the separate tool strip |
| `Panel::show_switched` / `show_collapsible` | present — animates between a thin/collapsed and a thick/expanded panel | **the ribbon collapse/expand primitive, already written and already animated.** Do not hand-roll it |
| `Panel::top` defaults to `.resizable(false)` (`panel.rs:239`) | present | recommendation: leave it non-resizable (§3.6 note) |
| `egui::containers::menu::{MenuBar, MenuButton, SubMenuButton, SubMenu, MenuConfig, MenuState}` (`containers/menu.rs:65-426`, re-exported `lib.rs:464`) | present; its doc comment recommends `Panel::top` | the `File` menu; group-collapse dropdowns at overflow, via `MenuButton::from_button` |
| `ui.selectable_label` / `Button::selectable` | present (`ui.rs:1928`); pdfce wraps both already (`toggle_label`, `icon_toggle`, `icon_text_toggle`) | tab strip; toggle-style commands |
| `egui::Frame::group` / `Frame::side_top_panel` (`containers/frame.rs:178`, `:185`) | present; pdfce uses `Frame::popup` already | the group boxes |
| `egui::Grid` (`grid.rs:327`), `ui.vertical`, `ui.allocate_ui_with_layout` (`ui.rs:1321`) | present | small-button stacks inside a group; per-group sizing |
| `egui::Image` at arbitrary size | present; `icons::image_tinted` rasterizes SVG paths at `ICON_PTS * pixels_per_point` and is trivially parameterised to a second size | the 32 pt large-button glyph — **no new art required**, the assets are vector |
| `Response::widget_info` (`response.rs:849`) | present; pdfce uses the exact pattern in `icon_button`, `labeled_icon_button` and `dock.rs::on_tab_button` | tab accessible names |
| `FocusDirection` + bare-arrow mapping (`memory/mod.rs:151-175`, `:575-581`), `Memory::move_focus` (`:918`) | present | **arrow-key navigation inside a band, for free** |
| `TextWrapMode::Extend` | present; pdfce already sets it on the toolbar for exactly this reason | break at the control boundary, never inside a label |
| `ScrollArea::horizontal()` (`scroll_area.rs:369`) | present | **deliberately NOT used** — §3.6 |
| `Ui::skip_ahead_auto_ids` (`ui.rs:898`) | present — **but it is an ID-ALLOCATION tool, not a focus-order tool.** A natural and wrong assumption, corrected here | id stability across a variable-length band; **not** a way to skip the ribbon in the Tab chain |

**What has to be written by hand, and it is a short list:**

1. **`ribbon.rs`** — the tab-strip row, the band, the group frame + caption, and the
   `RibbonTab` enum + `tab_band(&mut self, tab, ui, actions)` dispatcher. The dispatcher is
   deliberately the same shape as `dock.rs`'s `DockPanel` + `panel_body` — one enum, one
   exhaustive match, so a new tab is a compiler error until it is placed. That pattern is
   proven in this codebase and is reused rather than re-derived.
2. **A large icon-over-label button.** `PdfceApp` already has `icon_button`,
   `labeled_icon_button`, `icon_toggle`, `icon_text_toggle`, `icon_text` and
   `menu_button_atoms`. This is a seventh member of an existing family: a vertical layout of
   a 32 pt image over a wrapped-to-two-lines label, honouring `ICON_BUTTON_SIZE`'s
   `.min_size()`-not-`add_sized()` rule and publishing a `WidgetInfo` name.
3. **The contextual-tab dispatcher** — a pure function from
   `(Option<CanvasTool>, selection kind)` to `Vec<RibbonTab>`. **Pure, therefore headlessly
   testable**, which matters: it is the one part of the ribbon whose *logic* can be wrong in
   a way a human would notice, and `main.rs`'s module docs state the project's rule for that
   (*"every piece of logic that could be wrong in a way a human would notice is pushed into
   [a module] where it is a pure function with a unit test"*). It belongs in `ribbon.rs` with
   tests, not inline in the draw call.
4. **Group-collapse-on-overflow** (Pass 24.4) — measure each group's desired width, collapse
   right-to-left into `MenuButton`s until the row fits. This is the only genuinely fiddly
   piece, it has no prior art anywhere to copy, and that is why it is its own Pass rather
   than a footnote in 24.1.

### 5.3 What breaks — honestly

**Accessibility role: the same gap, at a second surface, and it is a collapse rather than an
omission.** Verified at `egui-0.35.0/src/response.rs:914-936`: the `WidgetType` →
`accesskit::Role` mapping sends `Button | CollapsingHeader | SelectableLabel` **all to
`Role::Button`**. A grep for `Role::Tab`, `Role::TabList`, `Role::TabPanel` or
`Role::Toolbar` across egui 0.35 **and** eframe 0.35 returns zero hits — neither crate ships
any tab or toolbar role. AccessKit itself has `Role::Tab`/`Role::TabList`, so the ceiling is
egui's mapping, not the backend, and `Response::widget_info` cannot reach past it because
`WidgetInfo.typ` is a `WidgetType`.

Net: ribbon tabs announce as **buttons that correctly report their pressed state** (egui
0.35 added that — upstream PR #8130, *"Announce pressed state of selectable buttons to
screen readers"*) with the accessible name pdfce supplies, and no tab role. That is
byte-for-byte the situation `dock.rs`'s module docs already record for `egui_tiles`' tabs
(*"only the name is supplied; the role cannot be, short of an upstream change"*). **The
ribbon must state this in the same honest convention**, and `main.rs`'s accessibility
doc-comment (lines 196-213) must be updated in the same commit — decision 017 §8.14 set that
precedent and it binds here. The `WidgetType::Other` doc comment explicitly invites the
upstream request (*"If this is something you think should be added, file an issue"*); filing
it is a cheap, honest contribution and is named as a follow-up, not a blocker.

**Focus chain: better than today, but for a reason that must be protected.** `main.rs:87-123`
documents that panel-add order is the Tab order and that the canvas is deliberately last.
Naively one fears a ribbon puts 50–150 focusable widgets ahead of the canvas — and **egui
has no focus-group, roving-tabindex or tab-index mechanism of any kind** (verified: zero
hits for `focus_group`, `roving`, `tabindex` across 0.35's source; Tab order is one flat
global sequence in widget-registration order). The rescue is immediate mode itself:
**widgets that are not drawn are never registered**, so emitting only the active tab's band
puts one tab's worth (~12 controls) plus the tab strip (~7) in the chain — *fewer* than
today's flat toolbar, which emits all ~25 unconditionally. Collapsed, it is ~7. This is a
real ergonomic improvement and §7 proposes making it a rule (R125), because it is a property
that a future "render all tabs and just hide the inactive ones" optimisation would silently
destroy.

**The focus dead-man's switch.** `memory/mod.rs:617-624` drops focus to `None` when the
focused widget stops being drawn. Switching ribbon tabs, or a contextual tab disappearing on
deselection, does exactly that. **Focus must be re-requested deliberately** (Pass 24.5's F1),
or the operator's keyboard position vanishes on every tab change. Named here because it is
invisible in testing with a mouse.

**Arrow-key navigation is free but fragile.** §0 finding 5. It works because pdfce binds
only Alt+arrows globally. A future Pass adding a bare-arrow global binding (nudge selection
by 1 pt is the obvious candidate) would break ribbon navigation, dock navigation and every
list in the app simultaneously, with no visual symptom. Named so it is a known constraint
rather than a regression.

**Keytips (Alt → H → V → P) are NOT buildable at reasonable cost, and are refused by name.**
Verified: `access_key`, `accesskey`, `key_tip`, `keytip` and `mnemonic` return **zero hits**
across egui 0.35's source. There is no access-key registry, no mnemonic underlining, no
Alt-overlay. Implementing keytips means intercepting Alt, entering a modal input state,
painting badge overlays over every control, and maintaining a unique-prefix assignment
across a variable command set — and it would fight egui's own focus system. **Out of scope,
permanently unless upstream adds support.** Stated explicitly because "ribbon" implies
keytips to anyone who uses Office heavily, and an unstated omission reads as a bug.

**`egui_tiles` and the dock: zero interaction, structurally.** The ribbon is a top panel; the
dock is a right panel added later, and egui resolves panels sequentially with each shrinking
the remaining rect. Independently confirmed from the other side: `egui_tiles` 0.16.0's
source has **zero** occurrences of `toolbar`, `ribbon`, `command_bar` or `menubar`, and
**zero** references to `egui::Panel` at all — it is a pure tiling tree with a ten-item public
surface. The one contact point is that the View tab pushes `Action`s the existing
`toggle_dock_panel` / `dock::activate` already handle.

**Strings and layout: R1 and R3 bite harder.** ~60 new catalog entries, all whitespace-bearing
literals, all caught by `tools/check-ui-strings.sh` if they land outside `ui_text.rs`. And
R3 (no English-width layout) applies to every fixed-position tab label and group caption:
the +50 % budget must be documented in a comment at each fixed extent, as R3 requires.

### 5.4 Cost estimate, stated as a range with its basis

`dock.rs` is 599 lines including ~130 lines of module docs and ~130 lines of tests, wrapping
a dependency that did the hard part. `ribbon.rs` does more layout and less delegation, and
has no prior art to shorten it: **expect 700–1,100 lines** for Pass 24.1 (shell + Home/View
bands), plus ~60 `ui_text.rs` entries, plus the large-button widget (~60 lines with docs) —
and a **reduction** in `main.rs`, since `toolbar_controls`' ~600 lines of interleaved layout
and reasoning move into per-tab functions. The net line count is roughly flat; the net
*comprehensibility* improves, because each tab function answers one question.

This is an estimate from comparable in-repo work, not a measurement. It is stated so the
first Pass can falsify it.

---

## 6. Q5 — The staged migration plan

Six Passes. **Pass family 24** (families up to 23 are claimed — R106 checked at draft time;
re-check at filing). Ordered so the operator sees the thing he complained about fixed first,
in a Pass that contains no ribbon at all.

**Rule 11 (CLI parity) does not apply to any Pass in this family.** Every one is a pure
GUI-shell change with no headless equivalent — there is no `pdfce-cli` surface for "where
the Accept button is." **Stated explicitly in each Pass entry so the omission reads as
reasoned rather than missed**, exactly as decision 017 §8.13 required.

**R85 (preview-equals-saved) is likewise untouched**: no Pass in this family writes a byte.
`tools/content-identity` must report **0** changed content streams for the whole family, and
that is a cheap, mechanical guard against a "while I'm in here" edit sneaking in.

### 6.1 Pass 24.0 — The confirm leaves the page: fixed-anchor tool strip + universal Enter/Escape

**First, deliberately.** Smallest Pass in the family. **No ribbon.** No dependency on 22.0,
23.x, or on anything else in this record. Answers the operator's sharpest complaint on its
own.

- New `egui::Panel::top("toolstrip")`, added immediately after the existing toolbar, shown
  only while a tool is armed or a pending result exists. Content ordered **state → action →
  detail** (R99).
- Delete the three `image_rect`-anchored status `Area`s (`main.rs:9364`, `:10242`, `:11940`);
  move their content into the strip verbatim — **every disclosure string survives byte-for-byte**,
  which is the acceptance criterion that keeps this a *move* and not a *trim*.
- Enter commits in all three tools; Ctrl+Enter where plain Enter is meaningful in-gesture
  (Add Text box mode's existing behaviour is the model, not an exception).
- Reject routes through `resolve_escape`'s `CancelGesture` rather than a per-tool flag.
- Tier assignment (§4.2) applied: Tier-1 operations lose their confirm; the table ships in
  the Pass entry for the operator to review (question (ac)).
- Property bars stay floating (and stay `movable`) — §4.3.

| # | Acceptance criterion | Check |
|---|---|---|
| A1 | **No control's position derives from the page** | zero `egui::Area` remain in `pdfce-gui` whose position is computed from `image_rect`; grep-assertable |
| A2 | **The Accept control is in the same place for every tool** | a test asserting all three tools' confirm is emitted by the one strip function; R86 observed across all three |
| A3 | **Enter commits, Escape cancels, in all three tools** | one test per tool over the keyboard path; the Measure and Text-Edit cases are new capability, not regression cover |
| A4 | **No disclosure is lost in the move** | every `ui_text` entry referenced by the three deleted `Area`s is referenced by the strip; a test enumerating them, so a silently-dropped disclosure fails the build |
| A5 | **R99 ordering** | the Accept control renders above the disclosure list, asserted; and observed at a short window height where the list would otherwise push it under |
| A6 | **Zero bytes written** | `tools/content-identity` = 0; `cargo tree -p pdfce-core -p pdfce-render` GUI-free |
| A7 | **R86 — must be watched** | the strip does not jump in height as disclosures appear mid-gesture (a strip that grows under the cursor is a new version of the old problem); and that Accept is reachable without leaving the drawing |

### 6.2 Pass 24.1 — Ribbon shell: tab strip, `File` menu, QAT, and the Home/View bands

**Zero new commands. Zero behaviour change.** Every control that exists today still exists,
with the same label, tooltip, shortcut, enabled/disabled/hidden logic and selected-state
derivation. This Pass is a *relocation* and is measured as one.

- `ribbon.rs`: `RibbonTab` enum + `tab_band` dispatcher (the `DockPanel`/`panel_body` shape).
- Tab strip row: `File ▾` menu button (egui's `MenuBar`/`MenuButton`), QAT (Open/Save/Undo/
  Redo), the six tab labels, status summary pinned right by the existing right-to-left outer
  layout.
- The large icon-over-label button widget, honouring `.min_size()`.
- **Home** and **View** bands populated from today's toolbar. The other four tabs exist with
  their bands populated by relocation only (Insert, Edit, Measure, Protect all have shipped
  commands today).
- `main.rs`'s accessibility doc-comment updated with the tab-role gap.
- The band's control budget test (§3.6).

| # | Acceptance criterion | Check |
|---|---|---|
| B1 | **Every command survives** | a test enumerating every `Action` reachable from the old toolbar and asserting each is reachable from exactly one ribbon location; **exactly one**, so relocation cannot silently duplicate an entry point |
| B2 | **No shortcut changes** | `collect_keyboard_actions` diff is empty except the new `Ctrl+F1` |
| B3 | **Selected-state derivation preserved** | the Properties and Redact toggles still derive selected state from `dock::panel_is_active`, never a private boolean |
| B4 | **Hidden-vs-disabled decisions preserved** | Save hidden with no document; Undo/Redo disabled not hidden; page/zoom controls hidden with no document — each asserted, because each is a dated reasoned decision |
| B5 | **Focus chain is not worse** | a test counting focusable widgets emitted by the top panels: ribbon (active tab) ≤ today's toolbar |
| B6 | **No overflow hiding** | at the minimum supported window width, no band scrolls and no command is unreachable; **R86 observed at that width**, not just asserted |
| B7 | **Height budget reported** | the Pass entry states the measured chrome height before and after; §2.3 item 1's trade is quantified, not estimated |
| B8 | **R86 — must be watched** | the widest real label under a 32 pt icon at the narrowest supported width, against the `Pla`/`ce`/`poi`/`nt` failure; and whether six tabs read as navigable or as clutter on a 1100 pt window |
| B9 | Invariants | `cargo tree -p pdfce-core -p pdfce-render` GUI-free; **zero new Cargo dependencies**; `content-identity` = 0; `check-ui-strings.sh` clean |

### 6.3 Pass 24.2 — Tool contextual tabs: the property bars come off the page

Depends on 24.0 (the strip) and 24.1 (the tab machinery).

- The three floating property `Area`s (`main.rs:8854`, `:10120`, `:11825`) become contextual
  tool tabs (§3.3 Family A).
- The 24.0 tool strip's confirm becomes each tool tab's rightmost **Finish** group; the strip
  itself is retired.
- Auto-activate on arm, restore the prior fixed tab on disarm, **and re-request focus
  deliberately** (§5.3's dead-man's switch).
- The Measure tool's `movable`/`close` affordances retire with the floating box — closing the
  tool is what the tab's own tool toggle already does, and keeping a second close path would
  be two mental models again.

| # | Acceptance criterion | Check |
|---|---|---|
| C1 | **Nothing floats over the page any more** | zero tool-owned `egui::Area` remain; the only `egui::Window`s left are the R81-legitimate transients (signature, copy, redact-apply confirmations, text-entry popup, shortcuts, ce-dimension group manager) |
| C2 | **Every property control survives** | enumerated test, as B1 |
| C3 | **Tab activation is deterministic** | pure-function test over `(active_tool, selection kind) → Vec<RibbonTab>`, all `CanvasTool` variants |
| C4 | **Arming a tool never loses the operator's fixed tab, or his focus** | disarm restores the prior tab, asserted; focus is re-requested rather than dropped |
| C5 | **The ce-dimension group manager stays a window or becomes a panel — decided, not drifted** | decision 017 §9 named it *"the remaining floating-window holdout"*; this Pass states which it is and why |
| C6 | **R86 — must be watched** | that a tool tab appearing does not feel like the window jumped; and that the Finish group is findable without being told |

### 6.4 Pass 24.3 — Selection contextual tabs

**Depends on Pass 22.0** (annotations enter canvas selection — without it there is no
`TargetId::Annot` to key a ce-dimension or annotation tab on) **and on Pass 23.2**
(`TargetId::Content` → `ContentPath` — without it the Object tab has no level verbs). It
also *consumes* Passes 23.0/23.1's format and re-measure surfaces. **It cannot be pulled
forward**, and pretending otherwise is how a ribbon Pass would end up re-deriving decision
022's selection model under pressure.

- The four Family-B tabs (§3.3): **Object**, **Dimension (pdfce)**, **Annotation**, and the
  mixed-selection no-tab case.
- Verb sets sourced from R112's exhaustive match — **a verb absent from a kind is absent
  from its tab**, never greyed (R83).
- The ce-dimension tab's provenance-bearing label and tooltip (rule 15).

| # | Acceptance criterion | Check |
|---|---|---|
| D1 | **Verb sets are exhaustive-matched, not hand-listed** | adding a `TargetId` kind fails to compile until its tab is placed |
| D2 | **No verb appears for a kind that cannot perform it** | one test per kind × verb (R83); the inverse (R117) also checked — every shipped verb is on some tab |
| D3 | **The ce-dimension tab names its provenance** | its label and tooltip both distinguish ce dimensions from pdf dimensions; a string test, because rule 15 is a mutual-intelligibility rule and a silent regression here is expensive |
| D4 | **Selecting a pdf dimension raises the Object tab, not the Dimension tab** | fixture with a CAD-drawn pdf dimension; asserted |
| D5 | **Mixed selection raises no selection tab and says why** | asserted |
| D6 | **R86 — must be watched** | whether contextual tabs appearing/disappearing on every selection change reads as helpful or as flicker. **This is the single highest-risk R86 item in the family** — Office's contextual tabs work because selection changes are deliberate; a marquee over a CAD page changes selection continuously. Mitigation ready: defer contextual-tab changes until the gesture ends |

### 6.5 Pass 24.4 — Collapse, overflow, and the group-collapse degradation

- Double-click-to-collapse and `Ctrl+F1`, built on `Panel::show_switched` /
  `show_collapsible` rather than hand-rolled; session-only + disclosed (R82's stance).
- Group-collapse-on-overflow, right-to-left, into `MenuButton`s.
- `Reset ribbon` alongside `Reset panel layout` (decision 017 §8.12's reasoning: a
  rearrangeable surface can be wrecked in ways a fixed one cannot — and a *collapsed* ribbon
  is exactly the state a confused operator gets stuck in).

| # | Acceptance criterion | Check |
|---|---|---|
| E1 | **No command is ever unreachable at any window width** | a sweep from the minimum supported width to 2560 pt, asserting every command is reachable in ≤ 2 interactions |
| E2 | **A collapsed group is named** | never an unlabelled `»` |
| E3 | **Scroll arrows never appear** | asserted; treated as a defect report per §3.6 / decision 017 A.3 |
| E4 | **Collapse is disclosed as session-only** | visible text, per decision 012's precedent |
| E5 | **R86 — must be watched** | that the collapsed → transient-band → auto-collapse cycle does not strand the operator |

### 6.6 Pass 24.5 — Keyboard, focus, and the honest accessibility statement

- Arrow-key navigation verified working inside bands and the tab strip (it should be free —
  §0 finding 5 — so this Pass mostly *proves and protects* it).
- Focus re-request on tab switch (§5.3's dead-man's switch).
- `Ctrl+Page Up/Down` cycles tabs (or another chord — must not collide;
  `collect_keyboard_actions` is the authority).
- Every tab and every command publishes a `WidgetInfo` name plus a purpose tooltip that says
  **when to reach for it**, not a restatement of the label — decision 017 §8.6's rule, and
  `dock.rs` already enforces it with `every_panel_tooltip_adds_information_beyond_its_label`.
  The ribbon gets the same test.
- `main.rs`'s accessibility doc-comment finalised: names yes, roles no (`SelectableLabel` →
  `Role::Button`), keytips never, screen-reader still untested.
- File the upstream `WidgetType::Tab`/`TabList` request the enum's own doc comment invites.

| # | Acceptance criterion | Check |
|---|---|---|
| F1 | Arrow navigation works and is protected | test + a comment at every global key binding warning that a bare-arrow binding breaks it app-wide |
| F2 | Focus survives a tab switch | asserted against the `memory/mod.rs:617-624` drop behaviour |
| F3 | Every tab/command has a name and a purpose tooltip | the `dock.rs` test, generalised |
| F4 | The role gap is stated, not implied | doc-comment review; upstream issue filed and linked |
| F5 | No chord collisions | asserted against the full binding table |

---

## 7. Proposed standing rules

Next free number at draft time is **R121** (verified; re-verify at filing — R106).

- **R121 — A tool's confirm lives at a fixed, window-relative anchor; never at a position
  derived from the document.** A control that commits or cancels an operation must be at the
  same screen location for every tool and must not move when the operator zooms, scrolls or
  changes page. Sourced concretely: `main.rs:9366`, `:10244`, `:11942` all position
  Accept/Reject at `image_rect.min.x + 8, image_rect.max.y - 8` with a `LEFT_BOTTOM` pivot,
  so the confirm control tracks the page image and is drawn over page content — the operator
  reported this as *"a separate accept / reject box somewhere on the screen"* and it is
  literally somewhere. The neighbouring property bar had the same defect diagnosed in its own
  comment (`main.rs:11813-11817`) and was patched by making it *draggable*, which makes the
  operator responsible for moving the application's controls out of his way.

- **R122 — Every gesture that has a commit has a keyboard commit.** Enter commits the pending
  result; Escape resolves through `canvas::resolve_escape`'s precedence chain (never a
  private per-tool reject path); Ctrl+Enter where plain Enter is meaningful inside the
  gesture. Sourced concretely: this was shipped in exactly one tool of three (Add Text,
  `main.rs:9959-9970`) and in no other, so the Measure tool and the Text Edit tool could only
  be committed with the mouse, at the moving target R121 describes.

- **R123 — The command surface's structure is derived from what pdfce can do, never from
  another product's menus.** The extension of CLAUDE.md rule 12 (the Acrobat RAG catalogs
  capability, never GUI mechanics) and R61 (Inkscape is a behavioural reference only) to the
  ribbon's own taxonomy: tab names, group names and command placement are decided from
  pdfce's own organising question — *what does this command act on?* — which the codebase
  already uses (`main.rs:5734-5736`, `:5850-5855`). A tab set copied from a competitor's
  ribbon is trade dress, and it also produces placements no operator can derive from first
  principles.

- **R124 — Empty space in the command surface stays empty.** A ribbon group is never padded
  with disabled placeholders for unbuilt features, and a sparse tab is not a defect. R83's
  application to a surface that, unlike a full toolbar row, has visible free space and
  therefore actively invites the violation. A greyed control for a feature that does not
  exist promises a capability; a gap promises nothing, which is the truth.

- **R125 — Only the active tab's band is emitted.** Inactive tabs' controls are not built,
  not laid out, and not in the focus chain. Verified as the *only* available mechanism: egui
  0.35 has no focus group, no roving tabindex and no tab-index concept (zero source hits for
  all three), and `skip_ahead_auto_ids` allocates IDs rather than ordering focus — so
  immediate-mode non-emission is the sole way a six-tab ribbon keeps a shorter Tab chain
  (~12 + 7) than the flat toolbar it replaces (~25). A future "render all tabs and hide the
  inactive ones" optimisation would turn a keyboard improvement into a keyboard regression
  with no visual symptom. *(Flagged as possibly too small for a number; R120 is direct
  precedent that a methodology rule earns one when it prevents a concrete, named
  regression.)*

**Not proposed as a new rule: the narrowed rule 4.** §4.4's wording amends a rule in
`CLAUDE.md`, which is the operator's file. It is proposed there, for him, and if accepted the
librarian mirrors it into `ROADMAP.md` as a numbered rule that *cites* rule 4 rather than
duplicating it. The engineer must not edit `CLAUDE.md`.

---

## 8. What this record does NOT decide — for the operator

Filed as open operator questions **(ac)–(aj)** (letters run to (ab) at draft time; re-verify).

1. **(ac) — The rule-4 narrowing, and the Tier-1/Tier-2 boundary.** §4.4. Two things, and
   the second is the one that touches his hands: *which operations stop asking.* The
   engineer's proposal is that a click-click linear measure with snapping off, an object
   move, a node drag and a page rotate all commit on completion with undo as the escape
   hatch, while anything carrying a snapped point, a best-fit, a scale, a font-trust
   decision or a reflow keeps its two-stage confirm. **That boundary is a judgement about
   his work, not a technical fact**, and the engineer should not draw it alone. *Default if
   unanswered:* ship the proposed table, and make it easy to move an operation between tiers.

2. **(ad) — The permanent height cost.** ~96 pt idle versus ~34 pt today, on an 800 pt
   window. Collapse recovers it at the cost of a click. Acceptable? *Default:* ship it and
   report the measured number in Pass 24.1 (B7) so the question can be re-asked with real
   figures rather than estimates.

3. **(ae) — Also a conventional menu bar, or `File ▾` alone?** Every product he named has
   File/Edit/View menus; Office removed its menu bar precisely to have a ribbon.
   *Recommendation: `File ▾` alone* — one command surface, not two. *Default:* as
   recommended.

4. **(af) — Decision 017 §10 Q1's "wide model" is answered but unbuilt.** He said yes to
   `egui_tiles` owning the whole content area with documents as panes. That has never been
   built and this record does not build it. Still wanted? *Default:* leave it as a Backlog
   item; the ribbon neither helps nor hinders it.

5. **(ag) — Drag on the canvas: marquee or pan?** §1.3 item 3. `main.rs`'s own comment
   admits the UX review on this default was owed and never happened, and this is the review
   arriving. *Recommendation:* middle-drag and space-drag pan universally, left-drag stays
   marquee. Not decided here because it is a canvas-gesture decision, not a
   command-surface one, and it deserves its own record if he wants it changed.

6. **(ah) — Zoom/navigation on the View tab, or permanently visible?** §2.3 item 2. Zoom is
   used constantly and putting it behind a tab switch is the classic ribbon regression.
   Options: keep it on the QAT row; put it in the status bar (browser convention); or accept
   the tab. *Recommendation:* status-bar zoom cluster **plus** the View tab, since the status
   bar is already permanent and currently carries no controls. *Default:* View tab only,
   revisit after use.

7. **(ai) — Contextual tab flicker on marquee selection.** §6.4's D6, the highest-risk R86
   item in the family. If dragging a marquee across a CAD page makes a tab appear and
   disappear continuously, contextual tabs are wrong for pdfce even though they are right for
   Office. There is a cheap mitigation (defer contextual-tab changes until the gesture ends)
   and it should probably ship in 24.3 by default. *Default:* defer-until-gesture-end.

8. **(aj) — The ce-dimension contextual tab's exact label.** `Dimension (pdfce)` is the
   proposal (§3.3). Rule 15 makes this his call by construction — it is a
   mutual-intelligibility rule, and the string is the point of contact. *Default:* as
   proposed.

**Also not decided here, and deliberately left where they are:**

- **The dock's structure** — untouched (§3.4). Decisions 017/A and R80–R84 stand.
- **Redaction apply's blocking confirmation** — untouched. R98 governs it; it is not a
  Tier-2 candidate and this record does not make it one.
- **§1.3 items 5, 6, 7** (menu bar, two side-region mental models, Escape's invisible
  precedence) — surfaced with recommendations, decided by nobody yet. Item 6 is partially
  fixed as a side effect of the View tab's Panels group; items 5 and 7 are not.
- **Icon art for future commands.** The existing 38 SVG-path icons render at any size, so
  large-button variants are free — but Insert/Edit/Protect will want glyphs for commands
  that do not exist yet. `docs/ui_specs/icon-set-and-toolbar.md` owns that pipeline;
  R83/R124 mean no icon is needed until its command is.
- **`pdfce-ui-specialist` owns the visual design** of the tab strip, the group frame, the
  contextual-tab distinction marker and the large-button proportions. This record owns the
  *architecture and the rules*; the specialist owns how it looks. Dispatch it before Pass
  24.1 draws a pixel.

---

## 9. Risks to the two load-bearing invariants

**GUI-core separation (`ARCHITECTURE.md` §3, CLAUDE.md rule 2): no risk, and one thing to
watch.**

Every Pass in this family is confined to `pdfce-gui`. `pdfce-core` and `pdfce-render` gain
nothing; `cargo tree -p pdfce-core` / `-p pdfce-render` remain the standing gate and are
listed in every Pass's criteria. **The one thing to watch** is the same one decision 023 §11
named from the other direction: *GUI state must not drift into core.* A ribbon introduces
new transient view state — active tab, collapse state, contextual-tab set — and the path of
least resistance for "which tab should be active for this selection" is to ask a core type.
It must not. The contextual-tab dispatcher is a pure function **in `pdfce-gui`** from
`(Option<CanvasTool>, selection kind)` to tabs; `CanvasTool` is already a `pdfce-gui` type
(`canvas.rs:86`) and stays one. If a core type ever grew a `fn ribbon_tab(&self)`, the WASM
fork would inherit a desktop UI mode.

**Round-trip / minimal-diff (§5): no risk, structurally.**

No Pass in this family writes a byte to any document. `tools/content-identity` reporting
**0** changed content streams is listed as an acceptance criterion for the family, which
makes this a checked property rather than an assumed one. The `Action` → `apply()` →
`EditSession` path is unchanged; the ribbon pushes the **same** `Action` values the toolbar
pushes today, which is B1's "exactly one location" criterion stated as an invariant.

**One genuine hazard, and it is not an invariant hazard — it is a disclosure hazard.**
Pass 24.0 moves ~40 disclosure strings from three floating strips into one panel. Rule 4,
R71, R72, R75, R76, R90 and R119 all bottom out in "the operator can see X before commit."
**A migration that drops one of those strings is a rule-4 violation with no test to catch
it and no visible symptom** — the tool still works, it just stops saying something. That is
exactly the `flatten_fields` failure shape this project has recorded before: correct
counters, wrong artifact. **A4's enumerated-string test is the guard**, and it is the single
most important acceptance criterion in this record.

---

## 10. JSON

```json
{
  "decision_id": "024",
  "title": "A ribbon command surface, and the end of the floating Accept/Reject box",
  "date": "2026-08-04",
  "status": "decided",
  "confidence": "high on the confirm model (S4), medium-high on the ribbon shape (S3) — the shape is product judgement over a stated operator preference, and S8 lists what is genuinely his call",

  "decisions": {
    "ribbon": "ADOPT. Six fixed tabs (Home, Insert, Edit, Measure, Protect, View) + a File MENU button + an inline Quick Access row (Open/Save/Undo/Redo) + contextual tabs. Replaces the single wrapping icon toolbar.",
    "dock": "UNCHANGED. egui_tiles dock stays exactly as decision 017 Amendment A shipped it. A ribbon and a dock are orthogonal surfaces; both reference products (MS Office, SolidWorks) have both. Structurally confirmed from the other side: egui_tiles 0.16.0 has zero occurrences of toolbar/ribbon/menubar and zero references to egui::Panel. The dock gains only one uniform toggle per panel on the View tab.",
    "floating_property_bars": "RETIRED into contextual TOOL tabs (Pass 24.2), keyed on active_tool.",
    "accept_reject": "REPLACED by a three-tier model: Tier 1 direct manipulation commits on gesture completion with undo as the escape hatch; Tier 2 inference-under-review commits through a FIXED, window-relative anchor carrying the readout and disclosures; Tier 3 Enter commits and Escape steps back, universally.",
    "dependency": "NONE. No ribbon crate exists for egui — verified against crates.io (egui_ribbon and egui_toolbar both 404), the GitHub repo/issue/discussion APIs (total_count 0 on every probe) and the 3rd-party-egui-crates wiki. MIT licensing forbids GPL/AGPL sources categorically, but nothing GPL/AGPL/LGPL appeared in the survey at all. Hand-build one module (ribbon.rs) reusing the DockPanel/panel_body dispatcher shape.",
    "keytips": "REFUSED BY NAME. egui 0.35 has zero source hits for access_key/accesskey/key_tip/keytip/mnemonic. Implementing Alt-key keytips means a modal input state plus badge overlays plus unique-prefix assignment, fighting egui's own focus system."
  },

  "verified_findings": [
    "The three tool status strips are egui::Area at fixed_pos(image_rect.min.x + 8, image_rect.max.y - 8) with LEFT_BOTTOM pivot (main.rs:9366, :10244, :11942) — the confirm control's position is a function of the DOCUMENT, moving on zoom, scroll and page change, drawn over page content.",
    "Enter already commits in exactly ONE of three tools (Add Text, main.rs:9959-9970); a repo-wide grep for Key::Enter in pdfce-gui returns that one site. Measure and Text Edit have no keyboard commit at all.",
    "egui 0.35's WidgetType has NO Tab/TabList/Toolbar/MenuBar variant (lib.rs:623-668, 20 variants), and the AccessKit mapping at response.rs:914-936 sends Button | CollapsingHeader | SelectableLabel ALL to Role::Button. AccessKit itself has Role::Tab/TabList, so the ceiling is egui's mapping, not the backend. egui 0.35 did add pressed-state announcement for selectable buttons (upstream PR #8130), which is the best available fidelity.",
    "egui 0.35 DOES do spatial arrow-key focus navigation (FocusDirection, memory/mod.rs:151-175 and :575-581, resolved against real screen rects in end_pass), and pdfce binds only Alt+arrows globally (main.rs:5315-5316) — so ribbon arrow navigation is free, and a future bare-arrow global binding would silently destroy it app-wide.",
    "egui has NO focus-group / roving-tabindex / tabindex concept (zero source hits for all three); Tab order is one flat global sequence in widget-registration order. Ui::skip_ahead_auto_ids (ui.rs:898) is an ID-ALLOCATION tool, NOT a focus-order tool — a natural and wrong assumption. Immediate-mode non-emission of inactive tabs is therefore the SOLE available mechanism for bounding the Tab chain.",
    "egui drops keyboard focus to None when a focused widget stops being drawn (memory/mod.rs:617-624) — exactly what a ribbon tab switch or a contextual tab disappearing does. Focus must be re-requested deliberately.",
    "egui 0.35 ships a real menu API (containers/menu.rs:65-426: MenuBar, MenuButton, SubMenuButton, SubMenu, MenuConfig, MenuState; re-exported lib.rs:464) whose doc comment says 'The menu bar goes well in a Panel::top'. MenuButton::from_button lets a ribbon-sized button carry menu behaviour.",
    "egui::Panel ships show_switched and show_collapsible (animate between thin/collapsed and thick/expanded) — precisely the ribbon collapse/expand primitive, already written and animated. Panel::top defaults to .resizable(false) (containers/panel.rs:239). TopBottomPanel/SidePanel were REMOVED (not deprecated) in egui 0.34 (upstream PR #5659) — a warning about copied snippets, not a migration, since pdfce already uses Panel::top.",
    "ZERO prior art: GitHub repo search 'egui ribbon' total_count 0; repo search 'ribbon ui language:Rust' total_count 0; issue search repo:emilk/egui ribbon total_count 0; Discussions discussionCount 0. pdfce would be the first ribbon in the egui ecosystem — no reference implementation for overflow maths, contextual-tab state machines, or collapse behaviour.",
    "The codebase contains three dated, independent admissions that the flat toolbar is full: the ~640pt per-character label wrap (main.rs:5688-5703), the 'capped at its existing six groups' intent since broken four times (main.rs:6254-6257), and the chevron shrunk 16pt to 10pt because 'toolbar width is no longer free' (icons.rs:1618-1624).",
    "The Measure property bar had this exact defect diagnosed in its own comment (main.rs:11813-11817) and was patched by making it DRAGGABLE — making the operator responsible for moving the application's controls out of his way. Its sibling status strip never even got that patch."
  ],

  "rule_4_narrowing": {
    "why_needed": "Rule 4 requires an INFERENCE to be REVIEWABLE. Three tools independently converged on the same implementation of 'reviewable' — a floating box with two buttons — each citing the one before it as precedent. Convergence by precedent is not convergence on the right answer. Two over-applications resulted: (A) Tier-1 operations that infer nothing got a confirm anyway; (B) the confirm CONTROL was conflated with the review CONTENT.",
    "proposed_wording": "Anything pdfce INFERRED — a value, boundary, classification or correction the operator did not directly specify — is visible before it becomes document state, and the operator can reject it without undoing anything else. This is a requirement on DISCLOSURE, not on any particular widget. It is satisfied by the inferred value being on screen and the commit being a deliberate act (a key press, or a click on a control at a fixed predictable position). It is NOT satisfied by a control whose position is derived from the document, and it does NOT require a two-click confirmation for a direct manipulation whose result is fully visible on the canvas and reversible in one undo. Where an inference is inherently uncertain, the uncertainty is stated in the disclosure, not merely implied by the presence of a confirm button.",
    "rules_checked_unaffected": ["R119", "R72", "R75", "R76", "R71", "R90", "R98", "R118", "R83", "R85"],
    "what_it_does_weaken": "A Tier-1 operation that shows an Accept button today will stop showing one. The operator's model shifts from 'pdfce asks before it changes my file' to 'pdfce asks before it applies something it guessed.' Undo-backed, requested, and flagged as operator question (ac) because the tier boundary is a judgement about his work.",
    "ownership": "CLAUDE.md is the operator's file. This is a PROPOSAL to him, never an engineer edit."
  },

  "passes": [
    {
      "id": "24.0",
      "name": "Fixed-anchor tool strip + universal Enter/Escape (NO ribbon)",
      "depends_on": [],
      "acceptance": [
        "A1 zero egui::Area in pdfce-gui positioned from image_rect (grep-assertable)",
        "A2 the Accept control is at one position for all three tools; R86 observed across all three",
        "A3 Enter commits / Escape cancels in all three tools (new capability for Measure and Text Edit)",
        "A4 every ui_text disclosure referenced by the three deleted Areas is referenced by the strip — enumerated test; THE most important criterion in this record",
        "A5 R99 state-then-action-then-detail ordering, asserted AND observed at a short window height",
        "A6 content-identity 0; cargo tree -p pdfce-core -p pdfce-render GUI-free",
        "A7 R86 WATCH: the strip must not jump height mid-gesture; Accept reachable without leaving the drawing"
      ]
    },
    {
      "id": "24.1",
      "name": "Ribbon shell: tab strip, File menu, QAT, Home/View bands. Zero new commands.",
      "depends_on": ["24.0"],
      "acceptance": [
        "B1 every Action reachable from the old toolbar is reachable from EXACTLY ONE ribbon location",
        "B2 no shortcut changes except the new Ctrl+F1",
        "B3 Properties/Redact selected state still derived from dock::panel_is_active, never a private boolean",
        "B4 hidden-vs-disabled decisions preserved verbatim (Save hidden, Undo/Redo disabled, page/zoom hidden)",
        "B5 focusable widgets emitted by the top panels <= today's toolbar",
        "B6 at minimum supported width no band scrolls and no command is unreachable; R86 observed AT that width",
        "B7 measured chrome height before/after stated in the Pass entry",
        "B8 R86 WATCH: widest real label under a 32pt icon at narrowest width (the Pla/ce/poi/nt failure)",
        "B9 zero new Cargo dependencies; content-identity 0; check-ui-strings clean"
      ]
    },
    {
      "id": "24.2",
      "name": "Tool contextual tabs — the property bars come off the page",
      "depends_on": ["24.0", "24.1"],
      "acceptance": [
        "C1 zero tool-owned egui::Area remain; only R81-legitimate transient Windows survive",
        "C2 every property control survives (enumerated test)",
        "C3 pure-function test over (active_tool, selection kind) -> Vec<RibbonTab>, all CanvasTool variants",
        "C4 disarming a tool restores the prior fixed tab AND re-requests focus rather than dropping it (memory/mod.rs:617-624)",
        "C5 the ce-dimension group manager's fate (window or panel) is DECIDED in this Pass, per decision 017 §9's 'remaining floating-window holdout'",
        "C6 R86 WATCH: a tool tab appearing must not feel like the window jumped; the Finish group findable without being told"
      ]
    },
    {
      "id": "24.3",
      "name": "Selection contextual tabs (Object / Dimension (pdfce) / Annotation)",
      "depends_on": ["22.0", "23.2", "24.2"],
      "acceptance": [
        "D1 verb sets exhaustive-matched — a new TargetId kind fails to compile until its tab is placed",
        "D2 no verb for a kind that cannot perform it (R83) AND every shipped verb is on some tab (R117)",
        "D3 the ce-dimension tab's label and tooltip distinguish ce dimensions from pdf dimensions (rule 15) — string test",
        "D4 selecting a pdf dimension raises the Object tab, not the Dimension tab (CAD fixture)",
        "D5 mixed-kind selection raises no selection tab and says why",
        "D6 R86 WATCH — HIGHEST RISK IN THE FAMILY: contextual tabs appearing/disappearing during a marquee over a CAD page. Mitigation ready: defer contextual-tab changes until the gesture ends."
      ]
    },
    {
      "id": "24.4",
      "name": "Collapse, overflow, group-collapse degradation, Reset ribbon",
      "depends_on": ["24.1"],
      "acceptance": [
        "E1 width sweep (minimum to 2560pt): every command reachable in <= 2 interactions",
        "E2 a collapsed group is NAMED, never an unlabelled chevron",
        "E3 scroll arrows never appear — a defect report per decision 017 A.3, not normal",
        "E4 collapse disclosed as session-only (decision 012 precedent); built on Panel::show_switched/show_collapsible, not hand-rolled",
        "E5 R86 WATCH: the collapsed -> transient band -> auto-collapse cycle must not strand the operator"
      ]
    },
    {
      "id": "24.5",
      "name": "Keyboard, focus, and the honest accessibility statement",
      "depends_on": ["24.1"],
      "acceptance": [
        "F1 arrow navigation works AND is protected by a comment at every global key binding (a bare-arrow global binding breaks it app-wide)",
        "F2 focus survives a tab switch, asserted against egui's drop-focus-when-not-drawn behaviour",
        "F3 every tab and command has a WidgetInfo name plus a purpose tooltip saying WHEN to use it (dock.rs's test, generalised)",
        "F4 the tab-ROLE gap is stated in main.rs's accessibility doc-comment, not implied away; the upstream WidgetType::Tab issue the enum's own doc comment invites is filed and linked",
        "F5 no chord collisions against the full binding table"
      ]
    }
  ],

  "rule_11_cli_parity": "DOES NOT APPLY to any Pass in family 24 — pure GUI-shell changes with no headless equivalent. Stated in each Pass entry so the omission reads as reasoned (decision 017 §8.13's precedent).",

  "proposed_standing_rules": [
    "R121 — A tool's confirm lives at a fixed, window-relative anchor; never at a position derived from the document.",
    "R122 — Every gesture that has a commit has a keyboard commit (Enter commits, Escape resolves through resolve_escape, Ctrl+Enter where plain Enter is in-gesture).",
    "R123 — The command surface's structure is derived from what pdfce can do, never from another product's menus (rule 12 / R61 extended to the ribbon's taxonomy).",
    "R124 — Empty space in the command surface stays empty; a ribbon group is never padded with disabled placeholders for unbuilt features (R83 applied to a surface with visible free space).",
    "R125 — Only the active tab's band is emitted; inactive tabs' controls are not built, laid out, or in the focus chain. Verified as the SOLE available mechanism — egui has no focus group, no roving tabindex, and skip_ahead_auto_ids allocates IDs rather than ordering focus. (Flagged as possibly too small for a number; R120 is precedent.)"
  ],

  "operator_questions": {
    "ac": "The rule-4 narrowing AND the Tier-1/Tier-2 boundary — which operations stop asking. A judgement about his work, not a technical fact.",
    "ad": "The permanent height cost (~96pt idle vs ~34pt today on an 800pt window). Collapse recovers it at the cost of a click.",
    "ae": "Also a conventional menu bar, or File-menu-button alone? Recommendation: alone — one command surface, not two.",
    "af": "Decision 017 §10 Q1's 'wide model' (egui_tiles owning the canvas region, documents as panes) is ANSWERED YES and UNBUILT. Still wanted?",
    "ag": "Drag on canvas: marquee or pan? main.rs admits the UX review was owed and never happened. Recommendation: middle/space-drag pans, left-drag stays marquee. Deserves its own record.",
    "ah": "Zoom/navigation on the View tab, or permanently visible? Recommendation: a status-bar zoom cluster PLUS the View tab.",
    "ai": "Contextual-tab flicker during marquee selection (D6). Mitigation ready: defer changes until the gesture ends.",
    "aj": "The ce-dimension contextual tab's exact label. Proposal: 'Dimension (pdfce)'. Rule 15 makes this his call by construction."
  },

  "not_decided_here": [
    "The dock's structure — untouched; decisions 017/A and R80-R84 stand.",
    "Redaction apply's blocking confirmation — untouched; R98 governs it and it is not a Tier-2 candidate.",
    "§1.3 items 5, 6, 7 (menu bar, two side-region mental models, Escape's invisible precedence) — surfaced with recommendations, decided by nobody.",
    "Icon art for future commands — icons.rs renders SVG paths at any size so large variants are free, but new commands need new glyphs; R83/R124 mean none is needed until its command is.",
    "Visual design — pdfce-ui-specialist owns the tab strip's look, the group frame, the contextual-tab distinction marker and large-button proportions. Dispatch before 24.1 draws a pixel."
  ],

  "risks": {
    "gui_core_separation": [
      "No risk structurally — every Pass is confined to pdfce-gui and the cargo tree gate is in every Pass's criteria.",
      "WATCH: GUI state drifting INTO core. The path of least resistance for 'which tab for this selection' is to ask a core type. It must stay a pure function in pdfce-gui over (Option<CanvasTool>, selection kind); CanvasTool is a pdfce-gui type (canvas.rs:86) and stays one, or the WASM fork inherits a desktop UI mode."
    ],
    "round_trip_minimal_diff": [
      "No risk — no Pass writes a byte. content-identity = 0 is a family-wide acceptance criterion, making it checked rather than assumed.",
      "The ribbon pushes the SAME Action values the toolbar pushes; B1's 'exactly one location' states that as an invariant."
    ],
    "sharpest_non_invariant_hazard": "Pass 24.0 relocates ~40 disclosure strings. Rule 4, R71, R72, R75, R76, R90 and R119 all bottom out in 'the operator can see X before commit.' A migration that drops one is a rule-4 violation with NO test to catch it and NO visible symptom — the flatten_fields shape: correct counters, wrong artifact. A4's enumerated-string test is the guard and is the most important criterion in this record."
  },

  "what_is_given_up": [
    "Canvas height, permanently (~96pt idle vs ~34pt; ~10% of an 800pt window). Collapse mitigates; nothing removes it.",
    "One click for any command not on the active tab.",
    "A visibly sparse surface, with the obvious remedy (greyed placeholders) forbidden by R83/R124.",
    "A second hand-rolled layout surface to re-verify on every egui upgrade — with ZERO prior art in any Rust GUI ecosystem to learn from.",
    "~60 new ui_text.rs entries under R1's catalog rule and R3's +50% width budget.",
    "The accessibility role gap, now at two surfaces instead of one — and it is a COLLAPSE (SelectableLabel -> Role::Button), verified, not feared."
  ]
}
```

## 11. SUPERSESSION 2026-08-05 — §3.3 Family A superseded by decision 031 / Pass 34.1; Family B unaffected

**Filed by:** `pdfce-librarian`, on `pdfce-engineer`'s explicit dispatch, following
`pdfce-ui-specialist`'s recommendation in
`docs/ui_specs/ribbon-groupings-and-customization-architecture.md` §2 ("This
is a decision-log-worthy correction... I recommend the engineer dispatch
`pdfce-librarian` to file it that way"). **Original §3.3 text above is left
unedited** — the directory is append-only (`docs/decisions/README.md`) and
the same discipline decision 031 §7 already used for its own correction.

**What §3.3 decided (2026-08-04):** **Family A — TOOL tabs, keyed on
`doc.active_tool: Option<CanvasTool>`**, one contextual ribbon tab per armed
tool (Measure Tool / Add Text / Edit Text / Edit Objects), each ending in a
fixed **Finish** group carrying the Accept/Reject pair, replacing the three
floating property-bar `egui::Area`s entirely. Pass 24.2 was scoped to build
it.

**That is not what shipped, on the operator's own later instruction.** On
2026-08-05 — one day after this record was written — the operator gave a
*more specific* instruction than the one §3.3 had to work from, verbatim:
*"When I select a tool like the edit text one all of the options should be
shown in a side bar tab docked with the page navigation tab,"* and later,
*"add text and measure tools should integrated into the context sensitive
sidebar tab."* The result, decided in decision 031 §4 (corrected §7) and
built as Pass 34.1: a left-hand `egui_tiles::Tree<DockPanel>` with tabs
`Pages | Tool Options`, and `DockPanel::ToolOptions` — a **dock panel**, not
a ribbon tab — now hosts each armed tool's live controls and its Pass 34.0
commit/discard contract. Three slices shipped it: slice 1 (`e15f55b`, the
dock scaffold), slice 2 (`fae916d`, Edit Text's property bar), slice 3
(`13f3c0b`, Add Text and Measure). See `ROADMAP.md`'s Shipped entry for the
full build record.

**Family A (§3.3, this record) is SUPERSEDED for tool-options content.**
`DockPanel::ToolOptions` already does the job Family A specified — auto-
raise on tool-arm, no forced return on disarm, a fixed predictable location
for a tool's live controls and its commit/reject — just on the other side of
the window, in the dock rather than the ribbon, and already shipped before
Pass 24.2 (ribbon contextual tabs) was ever started. Building Family A on
top of an already-shipped, already-working equivalent would be two
mechanisms doing one job for the same tools — precisely the failure §3.4
point 4 of this same record already warns against ("two large surfaces
rewritten at once"). **What survives from Family A is the *principle*** — a
fixed, predictable home for a tool's live options and its commit/reject,
never a floating box — realized in the dock instead of the ribbon, because
that is where the operator's own later, more specific instruction put it.

**Family B (§3.3's second table — SELECTION tabs keyed on `TargetId` kind:
Object / Dimension (pdfce) / Annotation) is UNAFFECTED and still stands.**
Nothing in Pass 34.x touches canvas selection; Family B remains correctly
blocked on Passes 22.0 and 23.2, exactly as this record originally scoped it
(§3.3, §6.4).

**Consequence, stated plainly for the next reader.** If and when Pass 24.2
(or its successor) is built, the **Measure** and **Edit** (and Add Text /
Edit Objects) contextual ribbon tabs it produces are **thinner than this
record pictured**: their job becomes *invocation* — arming the sub-tool,
managing ce-dimension groups, document-scope commands available whether or
not a tool is armed — never the armed tool's live controls, which live in
`DockPanel::ToolOptions` and must not be duplicated onto a ribbon tab. A
future session reading §3.3's table alone, without this section, would
build a second home for controls that already have one.

**Independently confirmed** by
`docs/ui_specs/ribbon-groupings-and-customization-architecture.md` §2/§3
(`pdfce-ui-specialist`, 2026-08-05), which also gives Measure its own fixed
ribbon tab rather than a group under Insert — the tie broken cleanly
*because* this supersession leaves that tab's body thin (invocation only,
no live controls competing for the same space) — and specifies the future
ribbon-customization architecture (`RibbonCommandId`/`RibbonCommand`/
`RibbonGroupDefault`, deliberately naming the group-identity type
`RibbonGroupId` rather than `GroupId` to avoid collision with
`pdfce_core::dimension::GroupId`) that decision 024 §8 explicitly deferred.
Not yet built; §5.4 of that document recommends against building
reorder/hide/reset UI or persistence now.

**No standing rule or Pass ID is spent by this section.** It corrects the
record, not the ledger. See `ROADMAP.md`'s "★ Pass 24.0–24.5" Next-up entry
and Standing rules for the cross-filed process observation (R155) about how
this near-miss was caught.
