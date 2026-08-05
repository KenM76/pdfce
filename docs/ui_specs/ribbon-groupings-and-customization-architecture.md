# Ribbon command groupings + the architecture for future customizability (ui-spec)

**Author:** `pdfce-ui-specialist` · **Date:** 2026-08-05 · **Drives:** the
operator's verbatim ruling, relayed by `pdfce-engineer`:

> "For 1. and 2. just make the ribbon command groupings make sense, they
> might be similar to acrobat's but if it makes more organizational sense
> to have them a different way then do so. we might want to make these
> customizable in the future like you can with solidworks and ms office."

**I do not write code.** Everything below is a critique, an audit against
already-decided work, and a concrete architecture the engineer implements
or pushes back on.

**Terminology (CLAUDE.md rule 15, binding throughout).** Every dimension
object named below is a **ce dimension** — a `/Line` + `/IT
/LineDimension` annotation pdfce itself authors (`crates/pdfce-core/src/
dimension/`) — never a **pdf dimension** (a pre-existing CAD-exported
callout pdfce did not create). The Measure tab discussed in §4 authors
**ce dimensions**; nothing here concerns pdf dimensions.

---

## 0. What was actually read

`docs/decisions/024-ribbon-command-surface-and-the-accept-reject-problem.md`
in full (1,629 lines) — this is the load-bearing document; §3.2/§3.3
already contain a complete, reasoned ribbon tab/group taxonomy and §6
already stages six Passes (24.0–24.5) to build it. `docs/ROADMAP.md`'s
Pass 24.0–24.5 entry, R121–R125, open operator questions (ac)–(aj) and
(ax)/(ay)/(az), and the Pass 34.0/34.1 Shipped entries (decision 031).
`crates/pdfce-gui/src/main.rs`'s current `toolbar_controls`
(L6882–7495), `action_preserves_gesture` (L5502–5529), `thumbnail_rail`'s
selection-action bar (L8586–8688), `tool_options_panel` (L7589–7650).
`crates/pdfce-gui/src/dock.rs` in full — `DockPanel`, `default_tree`,
`default_left_tree` (the left dock, Pass 34.1, already shipped and
already doing part of what decision 024 planned for the ribbon — see §2).
`crates/pdfce-gui/src/icons.rs`'s `Icon` enum and
`crates/pdfce-gui/assets/icons/PROVENANCE.md`. `docs/UI_PREFERENCES.md`
(this agent's own prior deliverable, 2026-08-05) for the token/component
vocabulary a ribbon must draw from. This agent's own memory of the
gesture-commit audit and the icon-set spec, re-verified against current
code rather than trusted from memory.

**The headline finding: this is mostly an audit, not a fresh design.**
Decision 024 already answers almost everything the brief asks for
(Deliverable 1's grouping, most of the Measure-placement argument, most of
the R84/icon question's shape). Re-deriving any of that from scratch
would be redundant, more expensive than reading it, and risks silently
drifting from a document the operator has not been told is wrong. What
this document adds: (a) the operator's ruling formally closes the one
thing decision 024 could not decide for itself (§1); (b) a real
architectural fork opened *after* decision 024 was written, which changes
what "the Measure tab" and "the Edit tab" actually contain (§2); (c) named
deltas between decision 024's snapshot (2026-08-04) and today's shipped
state (§3); (d) the customization architecture decision 024 explicitly
deferred, which is the genuinely new work here (§5); (e) an icon-coverage
audit scoped to *today's* commands only, per R124's own rule (§6).

---

## 1. The operator's ruling — three consequences, confirmed

### 1.1 The rule-12 conflict is dissolved, not amended — (ax) closes

Open question **(ax)** (`ROADMAP.md`) asked whether `CLAUDE.md` rule 12
should be amended so `pdfce-acrobat-librarian` could audit Acrobat's GUI
structure "so pdfce can intentionally differ." The operator did not grant
that amendment. He answered the *organizing* question directly instead —
"make the groupings make sense... if it makes more sense to have them a
different way, do so" — which is precisely what R123 (decision 024 §7,
already a standing rule) already requires: tab names, group names and
command placement are decided from pdfce's own organising question, *what
does this command act on?*, never from another product's menu structure.
**Recommendation, stated so nobody re-opens it:** `pdfce-acrobat-librarian`
is not dispatched for this. Rule 12 stands untouched. (ax) should be
recorded **closed** by the engineer/librarian — the operator resolved it
by answering the question underneath it, not by granting the audit.

### 1.2 (ay) is, in substance, also answered

Open question **(ay)** asked how close pdfce's ribbon should read to
Acrobat's actual layout — a product-identity call this agent's own
charter says only the operator can make. His words — "they might be
similar to acrobat's but if it makes more organizational sense to have
them a different way then do so" — state a *decision procedure*
(organizational sense governs) rather than a target resemblance
(neither "match Acrobat" nor "differ from Acrobat" is the goal in either
direction). That is a complete answer to (ay) even though it does not
name a specific resemblance level, because it removes resemblance as a
criterion at all. **Recommendation:** record (ay) **answered**, not left
open, with this framing quoted. This is the engineer's/librarian's ledger
to update, not mine — but I'd close it; the ruling is unambiguous.

### 1.3 Customizability is a future direction that changes today's architecture, not today's scope

Decision 024 §3.5 already refused to make the QAT customizable, for a
reason that still holds verbatim: customization means persisted user
state, which rides **R82/R15** (a named partition of the distribution
folder, never eframe's platform `Storage`), and R15 does not exist yet.
**That refusal is not overturned by the operator's ruling** — he said "in
the future," not "now" — but it does mean today's ribbon work should be
*structured* so that reorder/hide/reset are a data change away, not a
rewrite, once R15 lands. §5 is the concrete answer to that ask. The
operator named SolidWorks and Office specifically — §5.1 decomposes what
that reference frame concretely obligates the architecture to make cheap
later, and what it does not.

---

## 2. A fork opened after decision 024 was written — resolve this before trusting any tab content

**Decision 024 §3.3 Family A** (2026-08-04) proposed that the three
floating tool property bars (TextEdit, AddText, Measure) become
**contextual ribbon tabs** — one tab per armed tool, auto-activating,
ending in a fixed **Finish** group (the Accept/Reject pair). Pass 24.2 was
scoped to build this.

**One day later, decision 031 / Pass 34.0–34.1 (2026-08-05, shipped
`b84fd53` + `e15f55b`)** did something materially different, driven by a
*more specific* operator instruction than the one decision 024 had to
work from: *"all of the options should be shown in a side bar tab docked
with the page navigation tab."* The result is a **left-hand
`egui_tiles::Tree`** (`dock.rs` L334–382) with tabs **Pages | Tool
Options**, and `DockPanel::ToolOptions` (`dock.rs` L211–219) now hosts the
armed tool's identity, its commit/discard contract and its disclosures —
in the **dock**, not the ribbon.

**These are not the same mechanism**, and decision 024's Family A was
written before the second, more specific instruction existed. Family A's
"auto-activate a contextual tab on tool-arm, restore the prior fixed tab
on disarm" is *exactly* the job `DockPanel::ToolOptions`'s
arm-raises/disarm-does-not-lower behaviour already does (`dock.rs`
L369–374), just on the other side of the window and already shipped.
Building Family A on top of an already-shipped, already-working
equivalent would be two mechanisms doing one job — the same failure mode
decision 024 §3.4 point 4 itself warns against ("two large surfaces
rewritten at once").

**Recommendation, and why this matters for the rest of this document:**
**Pass 24.2 / Family A is superseded for tool-options content.** Tool
Options stays a **dock panel** (left dock), not a ribbon contextual tab.
What survives from Family A is the *principle* — a fixed, predictable
place for a tool's live options and its commit/reject, never a floating
box — just realized in the dock instead of the ribbon, which is where the
operator's own later, more specific instruction actually put it. Family B
(selection contextual tabs, Pass 24.3 — Object / Dimension (pdfce) /
Annotation) is **unaffected**: nothing in Pass 34.1 touches selection, and
Family B still correctly waits on Passes 22.0 and 23.2 as decision 024
scoped it.

**This is a decision-log-worthy correction**, the same shape as the Pass
27.2 tick-2 reversal already on record — I recommend the engineer dispatch
`pdfce-librarian` to file it that way (Family A superseded by Pass
34.0/34.1, Family B unaffected and still blocked) rather than let the two
records silently disagree for whoever reads them next.

**Consequence for "the Measure tab" (this document's §4) and "the Edit
tab" (§3):** the ribbon's **Measure** tab and **Edit** tab are about
*invocation* — arming a sub-tool, managing ce-dimension groups, the
document-scope commands that exist whether or not a tool is armed. The
*armed tool's own controls* (font/size/colour while Edit Text is armed;
constraint/snapping/scale-entry while a Measure tool is armed) live in the
**Tool Options dock**, not on the tab. This is a clean, already-precedented
split — R81's own line, "the ribbon carries commands, the dock carries
state you keep looking at" — and it resolves what would otherwise read as
two competing homes for the same controls.

---

## 3. The grouping — confirmed against decision 024, with deltas (Deliverable 1)

The full taxonomy is decision 024 §3.2/§3.3; reproducing all 1,629 lines
here would be exactly the redundant re-derivation §0 warns against. What
follows is the **organizing question** per tab (so the "does this button
change my file?" test the codebase already uses, `main.rs:5734-5736`, is
visible at a glance), confirmed against current shipped state, with every
delta named.

| Tab | Organising question | Status vs. decision 024 (2026-08-04) |
|---|---|---|
| **File** *(menu, not a tab)* | "What do I do with the file as a whole?" | **Unchanged.** Open · Save · Save As… · Combine/Split/Insert-pages (opens `BatchTools` dock) · Font folders · Keyboard shortcuts · Exit. |
| **Home** | "What do I do most, on any document?" | **Delta flagged, §3.1 below** — the Pages group's promotion (Delete/Extract/Move) needs a hidden-vs-disabled decision decision 024 didn't examine. |
| **Insert** | "What am I ADDING to the document?" | Unchanged. |
| **Edit** | "What am I CHANGING that is already there?" | **Delta — Reflow reasoned out, §3.2 below.** |
| **Measure** | "What am I measuring, and in what units?" | Unchanged in shape; **thinner than 024 pictured**, per §2's fork — its body is tool-arming + group management only. |
| **Protect** | "What am I removing or restricting?" | Unchanged. |
| **View** | "What is on my screen?" | **Two deltas, §3.3 below** — Show Points (Pass 36.3, postdates 024) and the two-dock Reset problem (Pass 34.1, postdates 024). |

**The tail (Copy▾ / Tools▾ / Keyboard) — confirmed fully resolved.**
Copy-text → **Home › Clipboard**. Combine/Split/Insert-pages/Font-folders
→ **File** menu, opening the existing `BatchTools` dock panel (decision
024 §3.2's rejected-schemes note explicitly refuses a separate `Tools`
tab for exactly this reason — it's file-scope, it already has a dock
panel, duplicating the entry point is the two-mental-models problem
again). Keyboard shortcuts → **File** menu. Every one of the brief's
"nine-control edit group" plus the "ungrouped tail" now has exactly one
organizing-question home — the dumping-ground critique in the brief is
fully answered by decision 024's existing taxonomy, not by anything new
in this document.

### 3.1 Home › Pages — a genuine open question decision 024 didn't examine

Delete / Extract / Move-up / Move-down (`thumbnail_rail`'s selection
action bar, `main.rs` L8607–8687) are **hidden entirely** today when
`selected_count == 0` — the exact "nothing to discover about a batch
action with zero pages picked" reasoning that also governs Save. Decision
024 §3.2 promotes these to a permanent **Home › Pages** ribbon group,
correctly calling this "a real gain, not a move" — but a *permanent* tab
group cannot use "hidden with nothing selected" the way a *selection-
scoped rail* can, because a ribbon group that is sometimes entirely blank
reads as broken, not as absent (this is the same principle R124 states
for unbuilt features, applied here to *selection* state instead). Two
honest options, not resolved here:

- **(a)** Show the group always, with Delete/Extract/Move **disabled**
  (not hidden) when `selected_count == 0`, tooltip stating "select one or
  more pages on the Pages panel to enable" — consistent with how Undo/
  Redo already distinguish "nothing to undo" from "control doesn't
  exist."
- **(b)** Do not promote them to Home at all; leave them rail-only, as
  today. Decision 024's "real gain" framing would then not apply to these
  four commands specifically, only to Rotate (which already works
  document-wide, unconditioned on selection).

**Recommendation:** (a) — it matches the Undo/Redo precedent the codebase
already trusts, and disabled-with-a-tooltip is exactly the "discoverable"
half of the discoverability checklist a permanently-hidden control fails.
Not asserting it as decided; flagging it because decision 024 promoted
these four commands without examining this question, and it is
genuinely new to this document.

### 3.2 Edit › Text — Reflow reasoned out of the tab, into Tool Options

Decision 024 §3.2 listed Reflow under **Edit › Text**, parenthetically
noting it is "currently only reachable from inside the Text Edit floating
bar." Given §2's fork, that parenthetical is the tell: Reflow is not a
document-scope command reachable independent of tool state — it requires
the Edit Text tool armed *and* a text-block selection, exactly like a ce
dimension's re-measure requires the Measure tool armed and a ce dimension
selected. Both belong in **Tool Options**, not on a permanent tab button,
for the same reason: a "Reflow" button sitting on the Edit tab with
nothing armed would either be permanently disabled (R83 — no affordance
without capability, since there is nothing to reflow) or, worse, would
invite arming the tool as a side effect of clicking it, which is a
surprising thing for a tab button to do. **Recommendation:** Reflow stays
inside Tool Options' Edit-Text body (where it already lives, Pass 15.2)
and does **not** get a permanent Edit-tab button. This is a delta from
024's literal wording, reasoned from the fork in §2, not from anything
024 got wrong on its own terms — it simply predates the left dock.

### 3.3 View — two deltas, both because Pass 34.1 postdates decision 024

1. **Show Points** (Pass 36.3, shipped 2026-08-05, after decision 024 was
   written) belongs in **View › Show**, beside the rail toggle and the
   annotation-visibility toggle — it is the same kind of command by
   decision 024's own organizing question ("what is on my screen," never
   the document). No argument needed; it is a straightforward addition to
   an existing group, not a new group.
2. **"Reset panel layout" must now reset TWO trees, not one.** Decision
   024 §3.4 named a single `Reset panel layout` control for the (then
   singular) right-hand dock. Pass 34.1 added a *second*, independent
   `egui_tiles::Tree` (the left dock, `dock.rs` L334–345) — `dock.rs`'s
   own module doc is explicit that the two trees are deliberately
   non-interoperable ("a pane dragged out of the left dock must not be
   droppable into the right one"). A single "Reset panel layout" control
   that only resets one of them would silently leave the other wrecked,
   which is worse than no reset at all for an operator who doesn't know
   there are now two trees to reset. **Recommendation:** either one
   control that resets both (simplest, and matches the operator's mental
   model of "put the panels back"), or two clearly-labelled controls
   ("Reset right panels" / "Reset left panels") if the engineer judges
   that operators will want to reset one without disturbing the other.
   Flagging the *problem* — decision 024 didn't know about the second
   tree yet — not asserting which fix.

---

## 4. The Measure verdict (Deliverable 2)

**Confirmed: Measure keeps its own fixed ribbon tab, not a group folded
into Insert or a contextual "Annotate" surface.** Decision 024 §3.2
already made this call and argued it correctly; restated here because the
brief asks for the argument, not because it needs re-deriving:

1. **On strict organizing-question grounds alone, "Insert" is
   defensible.** A ce dimension is authored content — rule 15's own
   framing — so "what am I adding to the document?" genuinely fits. This
   is not a case where the brief's worry (burying the operator's main
   tool inside Markup/sticky-notes) is avoidable by organizing-question
   logic alone; the two candidate homes are both *legitimate* answers to
   that question, which is exactly when usage pattern should break the
   tie.
2. **Usage pattern breaks the tie, and decision 024 already made this
   argument correctly:** dimensioning is the operator's own stated
   primary use case (CAD drawings), it earns "the room a tab gives you"
   in exactly the way `pass-12.M2-dimension-tools.md`'s own reasoning
   anticipated when it chose a dropdown *"given a flat toolbar with no
   room"* — a tab has room, and short deliberate bursts are exactly what
   a tab is for: go to the tab, work, leave. The dropdown's prior
   reasoning is not overturned; its premise (no room) is.
3. **§2's fork makes this cleaner, not murkier.** With Tool Options
   hosting the armed tool's live controls (constraint, snapping, scale
   entry, display mode), the Measure **tab**'s own body is genuinely thin
   — arm Linear / Circular / Set Scale, pick the active ce-dimension
   group, Manage Groups… — which is correctly sparse (R124: a sparse tab
   is not a defect) rather than crowded with controls that only apply
   once a sub-tool is armed anyway.
4. **This does not bury the operator's main tool anywhere.** A dedicated
   tab is the *opposite* of burying — it is the most prominent placement
   available short of the Home tab itself, which would wrongly imply
   Measure is used on every document rather than the deliberate,
   CAD-specific activity it is.

---

## 5. Architecture for future customizability (Deliverable 2, the new work)

### 5.1 What "customizable like SolidWorks and Office" concretely obligates, scoped

The operator named two specific reference products. Decomposed into the
gestures both actually offer at the **command-surface** level (as
distinct from macro recording or per-document scripting, which neither
product conflates with ribbon/toolbar customization and which nothing in
the operator's ruling asks for):

1. **Move a command** to a different group, or a different tab.
2. **Remove/hide a command** from its default location (declutter a
   crowded group without deleting the *capability* — the command still
   exists, reachable elsewhere).
3. **Reorder** commands within a group.
4. **Reset to default** — a single action, always available, that
   discards every override in one step.

**Not in scope**, because neither reference product does this at the
command-surface level either: renaming a command's label, reassigning a
command's icon, recording a macro, or building a *new* command from
existing ones (Office's "New Group" with hand-picked commands is the one
partial exception — noted as a possible future extension in §5.6, not
committed to here).

### 5.2 The data model — a command registry, the load-bearing structural change

Today, `toolbar_controls` (`main.rs` L6882–7495) is roughly 600 lines of
inline widget calls; "the grouping" exists only as source-code proximity
plus a `ui.separator()` — there is no data structure a future
reorder/hide/reset mechanism could operate on without editing Rust source
per change. The single structural change that makes customization cheap
later, without building it now, is extracting that inline code into a
**static registry** the render loop walks, keyed by a **stable command
identity** rather than by source position.

```rust
/// A command's STABLE identity — what a future override keys on.
/// Stable across releases: adding a variant is fine; renumbering or
/// removing one silently is the exact failure `dock.rs`'s own
/// unknown-`DockPanel`-variant fail-soft contract already guards
/// against for panels, and this needs the identical guard for commands.
enum RibbonCommandId {
    Open, Save, SaveAs,
    RotateLeft, RotateRight, DeletePages, ExtractPages, MovePagesUp, MovePagesDown,
    CopyTextPage, CopyTextDocument,
    // ... one variant per command in §3's confirmed taxonomy, no more,
    // no fewer — this enum's own exhaustiveness is what future work
    // checks against, mirroring R112's exhaustive-match discipline for
    // TargetId kinds.
}

/// One command's presentation + behaviour. Everything here is a Rust
/// value or function pointer — nothing is user-authored data yet (§5.4).
struct RibbonCommand {
    id: RibbonCommandId,
    icon: Option<icons::Icon>,
    label: fn() -> &'static str,       // routes through ui_text.rs, R1
    tooltip: fn() -> &'static str,      // routes through ui_text.rs, R1
    shortcut: Option<KeyChord>,         // cross-checked against collect_keyboard_actions
    kind: RibbonCommandKind,
    // `Option`, not a bare closure with a default: a command with no
    // selected-state concept (Open, Save) is genuinely different from
    // one that has a selected state and it happens to be false — R84's
    // renderer needs to tell those apart to decide whether to apply the
    // toggle convention at all.
    selected: Option<fn(&OpenDoc) -> bool>,
    enabled: fn(&OpenDoc) -> bool,
    emits: fn(&OpenDoc) -> Action,
}

enum RibbonCommandKind {
    Button,
    Toggle,
    /// A gallery/menu — its OWN contents are `RibbonCommandId`s, not a
    /// closure, precisely so an individual gallery item can later be
    /// promoted to a standalone button without re-typing it (§5.3).
    Menu(&'static [RibbonCommandId]),
    /// The escape hatch for controls that are not (icon, label, action)
    /// shaped at all — see §5.3.
    Custom(fn(&mut egui::Ui, &mut OpenDoc, &mut Vec<Action>)),
}

/// A group's DEFAULT membership and order. `RibbonGroupId`, deliberately
/// NOT named `GroupId` — `pdfce_core::dimension::GroupId` already exists
/// (a ce-dimension group's identity) and reusing the name in
/// `pdfce-gui` for an unrelated ribbon-UI concept is exactly the kind of
/// collision rule 15 exists to prevent one level up (two different
/// "GroupId"s in play, one core, one chrome, both real). Caught by
/// grepping the crate before naming anything, not assumed.
struct RibbonGroupDefault {
    tab: RibbonTab,
    group: RibbonGroupId,
    group_label: fn() -> &'static str,   // R1
    commands: &'static [RibbonCommandId], // DEFAULT order — a future
                                           // override replaces this Vec
                                           // for THIS group only
}
```

`tab_band(tab)` becomes: look up every `RibbonGroupDefault` for `tab`,
render each through **one shared function** that dispatches on
`RibbonCommandKind` — replacing ~600 lines of hand-written per-control
code with a loop plus a handful of `Custom` escape hatches. This is not
speculative infrastructure sitting unused: **it is how Pass 24.1 should
be implemented**, not a separate initiative bolted on afterward. Decision
024 §6.2 already scoped Pass 24.1 as "zero new commands, zero behaviour
change — a relocation, measured as one," with acceptance criterion B1
demanding "every command reachable from exactly one ribbon location."
That criterion is *exactly* what a registry with one entry per
`RibbonCommandId` proves by construction (an exhaustive match over the
enum), where a hand-written relocation would need to prove it by manual
enumeration. Building the registry IS the cheapest way to satisfy B1, not
an added cost on top of it.

### 5.3 The awkward cases, named one by one

- **Menu buttons (Markup ▾ / Text ▾ / Measure ▾) are galleries, not
  single commands.** `RibbonCommandKind::Menu(&[RibbonCommandId])` types
  the gallery's own contents (Rectangle / Ellipse / Arrow / Highlight,
  say) as first-class commands from day one — so the day someone wants
  "put Rectangle directly on Home" (exactly Office's own "promote a
  gallery item to the QAT" gesture), it is a data change (move
  `RibbonCommandId::MarkupRectangle` from one group's list to another),
  not a re-typing of something that was previously only a closure inside
  a menu-building callback.
- **Non-button controls (zoom % field, page indicator, the pen-colour
  swatch) carry live, continuously-edited state, not a fire-and-forget
  action.** `RibbonCommandKind::Custom` is the honest escape hatch — the
  registry can still say WHICH group a `Custom` command lives in and
  whether it is user-removable, even though it cannot describe the
  control generically. Forcing these into the (icon, label, action) shape
  would either lose real functionality (a live-typed zoom percentage
  cannot be represented as "emits one Action on click") or bloat `Action`
  with variants that are not actually discrete commands. Not every
  control reduces cleanly, and the registry says so honestly rather than
  pretending otherwise.
- **Enablement predicates stay plain Rust functions, not data.** Only
  *position* (which group, which order, hidden or not) is ever
  user-configurable under §5.1's scoped gesture list — *whether* a
  command is currently clickable is a correctness question about
  document state, never something an operator overrides. Keeping
  `enabled: fn(&OpenDoc) -> bool` as code (not a serializable rule
  language) means the eventual persistence format (§5.5) only needs to
  serialize `RibbonCommandId` values and group membership, never
  predicate logic — a much smaller, much safer surface.
- **Toggle rendering needs a selected-check, not just an enabled-check.**
  The `selected: Option<fn(&OpenDoc) -> bool>` field is what lets ONE
  shared render function apply R84's existing bold + outline-ring
  convention (`toggle_label`/`icon_toggle`) uniformly, instead of every
  toggle re-implementing it by hand at its call site as today's code
  does. This is a robustness gain independent of customization — a
  registry-driven renderer cannot forget the convention the way a new
  hand-written call site could.

### 5.4 What to build now versus what is premature — the honest boundary

**Build now, as part of Pass 24.1 (not a separate initiative):** the
`RibbonCommandId`/`RibbonCommand`/`RibbonGroupDefault` types and tables,
populated with **today's exact command set**, and `tab_band` rewritten to
walk them. Zero behaviour change, per decision 024's own Pass 24.1 scope
— this IS the relocation, implemented in the shape that makes the next
step cheap instead of the shape that makes it expensive.

**Do not build yet:** any reorder/hide/reset **UI**, and any
**persistence** of a customized layout. Two independent reasons, both
already on record and both still binding:

1. **Persistence needs R15**, which does not exist. Decision 024 §3.5
   already refused QAT customization for exactly this reason — "until R15
   lands there is nowhere legitimate to persist it, and a customisation
   UI that forgets on restart is worse than none." The same refusal
   applies to the full ribbon, verbatim, for the same reason.
2. **Building the interaction before the storage exists is the
   SolidWorks/Office failure mode inverted, not a feature.** A
   drag-to-reorder gesture that resets on every restart teaches the
   operator that customizing pdfce doesn't work, which is a worse first
   impression than no customization UI at all.

The registry (item 1) is valuable **on its own terms**, independent of
whether customization ever ships — it is what makes B1's "every command
reachable from exactly one location" a compile-time-checkable property
instead of a manually-maintained one, and it is what makes Pass 24.4's
group-collapse-on-overflow (decision 024 §6.5) implementable as "render
this same group's commands inside a `MenuButton` instead of inline"
rather than a second hand-written code path.

### 5.5 Reset-to-default and persistence, once R15 lands

The natural shape, once R15 exists, deliberately mirrors `dock.rs`'s
already-established fail-soft contract (module docs §7) rather than
inventing a new persistence convention:

- **Persist a flat override, not a serialized tree.** Unlike
  `egui_tiles::Tree` (an arbitrary split/tab structure the dock needs
  fully serialized), a ribbon's structure is a **fixed tab set** whose
  *contents* are reorderable/hideable — a much smaller surface. The
  natural shape: `Vec<(RibbonGroupId, Vec<RibbonCommandId>)>` for
  reordered/moved commands, plus a `HashSet<RibbonCommandId>` for
  hidden ones.
- **Fail-soft on load, same contract as the dock's.** An unknown
  `RibbonCommandId` (from a build that has since removed a command) is
  dropped, disclosed in the status line, never a crash. A
  `RibbonCommandId` that exists in the current build but is **absent**
  from a saved override (a command added in a later release than the
  saved layout) is **appended at its `RibbonGroupDefault` position** —
  unlike the dock's "missing mandatory panel" case, a ribbon command has
  no reasonable "just don't show it" fallback, because unlike a
  workspace panel a command is something the operator may need *right
  now* and has no other way to discover is missing.
- **"Reset to default"** — same convention as "Reset panel layout" —
  clears the override entirely and re-derives straight from
  `RibbonGroupDefault`. One control, always available, matching §5.1's
  scoped gesture list.
- **Disclosure**, per R82's existing stance: session-only and stated in
  visible text until R15 ships ("your ribbon layout is not saved between
  sessions"), then "restored from your saved layout" once it is —
  exactly the same two-state disclosure `dock.rs`'s own header already
  uses for panel layout.

### 5.6 One extension named, not committed to

Office's "New Group — pick commands from any tab" is a partial fifth
gesture beyond §5.1's four (it lets an operator build a group that
doesn't correspond to any single existing `RibbonGroupDefault`). Nothing
in today's registry design forecloses it — a user-defined group is just
another `RibbonGroupId` with an operator-chosen `Vec<RibbonCommandId>`
drawn from the full `RibbonCommandId` enum instead of one
`RibbonGroupDefault`'s fixed list — but it is **not** part of §5.1's
scoped ask and I am not recommending it be built. Noted only so a future
"can we also do custom groups" question doesn't require reconsidering the
data model from scratch — the answer would be yes, cheaply, if the
operator ever asks.

---

## 6. R84 / icon-coverage audit (Deliverable 3)

Scoped strictly per **R124** (decision 024, already a standing rule): "a
ribbon group is never padded with disabled placeholders for unbuilt
features... no icon is needed until its command is." The gaps below are
every case where a command **already exists and is already being
relocated** by §3's confirmed taxonomy, but has no `icons::Icon` variant
today. Nothing for backlog/future features (Encrypt, Sign, Certify, OCR,
Crop, Optimise, Image, Link, Form fields, Bates, the Format surface) is
listed — per R124, and per decision 024 §8's own explicit statement that
icon art for those is deliberately not decided yet.

| Gap | Where it surfaces | Recommendation |
|---|---|---|
| **Delete / Extract / Clear-selection** (page ops) | `thumbnail_rail`'s selection bar today — plain `ui.button(...)`, no icon. If §3.1's promotion to a permanent **Home › Pages** group proceeds (still an open question), these need icons before that ships: a large icon-over-label button with no icon renders the label alone at roughly twice today's button size, which is a worse regression than the small text button it replaces. | New icon-authoring work, **conditional on §3.1's promotion being approved** — not needed if these stay rail-only. |
| **Measure sub-tools** (Linear / Circular / Set Scale) | Plain-text `selectable_label` rows inside the `Measure ▾` menu today (`main.rs` L7295–7320) — no icons, only the shared `Measure` (ruler) glyph on the menu button itself. If the Measure tab's own body (§4) renders these as three separate large buttons rather than reusing the existing menu, each needs an icon distinct from the shared ruler glyph. | New icon-authoring work for whoever builds Pass 24.1's Measure tab — three new concepts (two-point-with-dimension-line for Linear; circle-with-radius-tick for Circular; a calibration/ruler-with-check glyph for Set Scale), none colliding with `ruler.svg`'s existing meaning. |
| **Redact sub-verbs** (Mark page / Search & mark… / Review & apply…) | Only the top-level `Redact` icon (the deliberate solid-fill exception, PROVENANCE.md §3) exists; these three are plain buttons inside the Redact dock panel today. If the Protect tab's Redact group renders them as three distinct large buttons, they need icons. | New icon-authoring work. Recommend visual kinship with the solid-fill `Redact` glyph (not the scissors/Split family — PROVENANCE.md's named collision must not recur here) since all three are steps in the *same* irreversible-once-applied feature. |
| **Objects** dock panel | `DockPanel::label()` returns text only; no icon exists for the object/layer tree. | New icon-authoring work if the View tab's Panels group renders icon+label buttons rather than a plain menu. |
| **Pages / Tool Options** dock panels | **Already shipped** (Pass 34.1, 2026-08-05) — genuinely current, not future. `PROVENANCE.md`'s file-by-file table predates both panels and assigns them no icon. | Flag as a real, present gap independent of the ribbon work — `PROVENANCE.md` needs a follow-up entry regardless of whether/when the View tab's Panels group goes icon+label. |
| **Reset panel layout** / future **Reset ribbon** | No icon exists. | **Exempt under R124** — not yet a ribbon-reachable control (the View tab hasn't shipped) and the dock's own existing control is text-only today. |

**Confirmed NOT gaps.** Every File/Home/Insert/Edit/Protect/View command
that exists in today's flat toolbar already has an icon per
`PROVENANCE.md`'s complete mapping — the relocation itself (Pass 24.1)
introduces **zero** new icon debt for those, which is worth stating
explicitly since it's the overwhelming majority of the command set.

---

## 7. Checklists (per this agent's own brief's mandatory format)

**Discoverability.** Every command in §3's taxonomy keeps its existing
label/tooltip verbatim (decision 024 B1/B4) — nothing here removes an
affordance. The Home › Pages open question (§3.1) is explicitly about
*improving* discoverability (disabled-with-a-reason beats hidden-with-no-
explanation) rather than the reverse.

**Accessibility.** No new widget types are introduced by the registry
(§5.2) — it changes how existing widgets are *reached from code*, not
what they render as. R84's toggle convention becomes *more* consistently
applied under a registry-driven renderer than under today's per-call-site
implementation, which is a net accessibility gain, not a neutral change.

**Fuzzy-never-sneaky.** Not directly engaged by this document — no
algorithmic inference is introduced. The one adjacent point: §5.5's
fail-soft load behaviour (an unknown `RibbonCommandId` dropped, a missing
one re-appended) must be **disclosed**, not silent, the same standing
requirement `dock.rs`'s own fail-soft contract already states for panels.

**Immediate-mode fit.** The registry (§5.2) is pure data plus function
pointers walked once per frame inside `tab_band` — no persistent widget
identity is assumed, no retained-mode state is introduced. `RibbonCommandId`
is exactly the same kind of stable-identity-over-position pattern
`DockPanel` already uses successfully for the dock; this is not a new
idiom for the codebase, it is the same one applied one level down.

---

## 8. Items for the engineer, not mine to decide

- **§2's fork (Family A superseded by Pass 34.1)** needs a decision-log
  correction. I recommend dispatching `pdfce-librarian` to record it, but
  the exact form (an amendment to decision 024, a new short decision, or
  a ROADMAP status annotation like the Pass 27.2 tick-2 reversal) is the
  engineer's/librarian's call.
- **§3.1 (Home › Pages hidden-vs-disabled)** is a genuine product
  judgment, not an engineering fact — I've named both options and their
  cost, not picked one for the record.
- **§3.3 item 2 (one Reset control vs. two, for two dock trees)** —
  likewise a real UX judgment best made by looking at the running app,
  per this agent's own standing precedent against inventing pixel-level
  answers from source alone.
- **Whether the §5.2 registry refactor ships AS Pass 24.1 itself, or as a
  preparatory tick immediately before it** — a scheduling call. My
  recommendation is to fold it in (Pass 24.1 has to touch every call site
  anyway to relocate it; writing each one twice, once inline and once
  into a registry, is strictly more work than doing the registry first),
  but sequencing inside a Pass is the engineer's call per this agent's
  charter.
- **(ax)/(ay)'s formal closure** — I've recommended closing both; the
  librarian's ledger update is the engineer's dispatch to make, not mine.
- **§6's conditional icon-authoring items** — contingent on §3.1's and
  §4's downstream implementation choices (whether Measure sub-tools and
  Redact sub-verbs get their own buttons vs. stay menu-shaped); the
  engineer decides the rendering shape, which decides whether the icons
  are needed at all.
