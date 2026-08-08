# Decision 033 — the GUI's usability gap, and the workspace that cannot act on what it shows

- **Status:** PROPOSED — §7 carries the questions that are Ken's, not the engineer's. **§7 ANSWERED IN FULL 2026-08-08 (see §8, appended) — Q1 (R124 placeholders) RULED AGAINST §5.1's proposed ribbon clause; Q2–Q5 ACCEPTED, two (Q3, Q4) with a larger operator-added requirement attached. `docs/ROADMAP.md` is CANONICAL for the current, resolved state — this file's §5.1/§6/§7 are the original proposal, annotated in place, not rewritten.**
- **Date:** 2026-08-08
- **Decider:** engineer, on the operator's direct instruction
- **Supersedes:** nothing. **Amends:** R124, R125 (scope note). **Mints:** two proposed rules (§5.3, §5.4), numbers librarian-assigned.
- **Depends on:** decision 017 (docks), 024 (ribbon + accept/reject), 028 (handles/hit priority), Pass 46.0 (core resize).

---

## §0 The request, verbatim, because the wording carries three separable instructions

> "The GUI in its current state is not very user friendly and feels incomplete, however I see that a lot of features are working. I want you to plan out an improved layout and feature set for the GUI. **Rewrite any rules about the GUI that make it less usable and user friendly.** The workspace needs to be able to handle all of the logical commands that it should handle, like resizing form fields, filling out form fields, multi-select of objects and right click context sensitive menus (same idea applied to the object tree view), a similar look and feel and navigation method as MS Office has for its ribbon bar / PDF-XChange Editor."

Three instructions, and they are not the same job:

1. **A layout and feature-set plan** — design work.
2. **A licence to rewrite standing rules** — governance work, and the unusual one. The operator is saying the rules are a *cause*, not merely a constraint on the fix.
3. **A named capability list** the workspace must handle — resize form fields, fill form fields, multi-select objects, right-click context menus on canvas **and object tree**, Office/PDF-XChange ribbon navigation.

The phrase *"a lot of features are working"* is the load-bearing qualifier. This is not a rewrite request. It is a *reachability* complaint: the capability exists and the workspace cannot get at it.

---

## §1 What the running application actually shows

Observed live on 2026-08-08 (release build, `demo-form.pdf`, window 1800×1160, captured by `PrintWindow` per tab). **Not inferred from code.** This section exists because three separate agent reports described the GUI accurately in code terms and none of them conveyed what it looks like.

### §1.1 The ribbon is a toolbar with small grey words in it

The band is **one row, ~26 px of controls**. Group captions render as tiny grey text *inline, to the left of* their buttons — `File  [Open…] [Save a copy…]  Document  [Properties]  Clipboard  [Copy this page's text] …`. Office renders a group caption **centred beneath** a group of mixed-size buttons in a band roughly four times as tall.

The consequence is not aesthetic. **The grouping is invisible**, so the ribbon reads as an undifferentiated row of buttons and the tab strip above it looks like a menu bar that changes the toolbar. Nothing about it signals "ribbon", which is precisely the operator's *"similar look and feel"* complaint.

**Three groups render with no caption at all**, confirmed on screen and in code: `Show` + `Panels` share one un-captioned block (`main.rs:10048`), and `LayoutReset` bypasses the group helper entirely (`main.rs:10781`). On the View tab the four leftmost controls float with no label of any kind.

### §1.2 Zoom, page navigation and undo vanish on four tabs out of six

Confirmed by capturing each tab. `Go to ‹ Page 1 of 1 ›` and the whole `Zoom` group exist **only** on View. Undo/Redo exist **only** on Edit.

So an operator on **Measure** — the operator's own stated primary activity — has no undo, no zoom, and no page controls without leaving the tab they are working in. Decision 024 §3.5(d) specified a Quick Access Toolbar precisely to prevent this, in those words: *"pdfce's zoom/navigation controls are used constantly and must not end up behind a tab switch."* **It was never built** — zero hits for `quick_access|QAT` anywhere in the crate.

### §1.3 A form field is invisible, and the application says so out loud

`demo-form.pdf` has two fields. The canvas shows **one** — the check box. The text field is not faint or unstyled; it is **absent**. The status bar states it:

> *2 annotation(s) on this page (1 painted, 2 form field(s)).*
> *1 annotation(s) have no appearance stream pdfce can paint, so nothing was drawn for them (pdfce never invents a look): Widget ×1.*

The Forms panel simultaneously lists *"Full name (p. 1)"* with a working text box. **The operator can type a value into a field they cannot locate on the page.** There is no highlight, no outline, no link from the panel row to the page position.

This is R43 working exactly as designed — pdfce never synthesises appearance from `/MK` — and it is also, for a form *editor*, close to disqualifying. §5.3 resolves it without touching R43.

### §1.4 The workspace cannot act on what it lets you select

- **Annotations, form-field widgets, redaction marks and links are not canvas-selectable at all.** The hit-tester covers page content objects only (`object_provider.rs:470-508`). "Resize form fields" therefore has *two* prerequisites, not one: the core verb (unbuilt, Pass 46.0) **and** widgets becoming selectable.
- **Multi-select selects, then silently acts on one.** Marquee and Shift-extend genuinely work — but `delete_selected_object` (`main.rs:4174`) and the drag handler (`main.rs:17806`) both read `canvas_selection.iter().next()`. Select five objects, press Delete: **one disappears, no message.** That is a correctness defect, not a missing feature, and it is the single worst thing in this document.
- **There is no right-click menu anywhere.** Zero `context_menu`/`secondary_clicked` hits in the entire crate. egui provides the primitive; pdfce uses it nowhere. Right-click is a reflex within the first minute of use, and it currently does nothing on the canvas, the Objects tree, the thumbnails and the form rows alike.
- **The Objects tree is a read-only view of editable things** — select and expand, nothing else. Same for the Comments panel, which cannot even delete (core has **no `delete_annotation` verb at all**).

### §1.5 The GUI reaches one authoring property out of ~30

Field creation sets **`tooltip` and nothing else** (`commit_field_draft`, `main.rs:15309-15357`). Every other property on all five `New*` specs is unreachable: `value`, `max_len`, `multiline`, `read_only`, `required`, `on_state`, `checked`, `export_value`, `selected`, `no_toggle_to_off`, `radios_in_unison`, `combo`, `editable`, `multi_select`, `sort`.

★ **And one of those is a functional bug:** `main.rs:15352` passes `Vec::new()` for `/Opt` unconditionally, so **every choice field the GUI creates has zero options and can never be filled.** Core discloses this (`has_no_options`); the GUI offers no way to avoid it.

Four core verbs remain GUI-unreachable — `rename_field`, `field_defaults`, `move_widget`, `add_push_button` — plus, newly found, **`move_dimension`, which is orphaned in the GUI *and* the CLI** while `main.rs:18498` claims it is the commit path (it is `place_dimension`).

---

## §2 The five causes, ranked

| # | Cause | Evidence | Cost to fix |
|---|---|---|---|
| 1 | **Verbs silently narrow a multi-selection to one target** | `main.rs:4174`, `:17806` | small |
| 2 | **The QAT was specified and never built**, so constant-use controls are tab-gated | zero `QAT` hits; decision 024 §3.5(d) | small — a placement move of wired controls |
| 3 | **No editor chrome for what R43 declines to paint**, so fields are invisible | §1.3, live | small–medium |
| 4 | **No context menus anywhere** | zero hits | medium |
| 5 | **The ribbon does not read as a ribbon** — one row, inline captions, three groups uncaptioned | §1.1, live | medium |

**What is NOT a cause, so it does not get "fixed" by accident:** the tab taxonomy (operator-authored, decision 024), the two-dock split, the icon set (real SVG), and the Forms panel's *fill* surface — which is better than its reputation and already handles text, check box, **radio**, **single-choice** and **multi-choice**, with rich-text correctly refused.

---

## §3 The shape of the answer

**No shell rewrite.** The complaint is reachability and legibility, and every fix below is additive to a structurally sound shell.

```
┌────────────────────────────────────────────────────────────────────────┐
│ [Open] [Save] │ ↶ ↷ │ ‹ 1/12 › │ − 100% + │        ← QAT, ALWAYS PRESENT │
│ File  Edit  Review  Measure  Tools  View  │  ▸ Measure Tool             │
├────────────────────────────────────────────────────────────────────────┤
│  ┌─ Content ──────┐ ┌─ Forms ─────────┐ ┌─ Pages ──┐                    │
│  │ [Aa] [Aa] [Obj]│ │ [Fill] [Create] │ │ [↺] [↻]  │   ← two-row band,   │
│  │    Content     │ │      Forms      │ │  Pages   │     captions BELOW  │
├────────────────────────────────────────────────────────────────────────┤
│ Pages  │            CANVAS + chrome overlay + context menu   │ Objects  │
│ Tool   │                                                     │ (+ menu) │
├────────────────────────────────────────────────────────────────────────┤
│ status / narrator                                                       │
└────────────────────────────────────────────────────────────────────────┘
```

### §3.1 Quick Access Toolbar — restore what decision 024 already specified

Open, Save, Undo, Redo, page nav, zoom. Promoted **out of** the gated bands, not duplicated into a second location (R123: one command, one place — so `History`, `Navigate` and `Zoom` leave the ribbon). Fixed chrome, emitted every frame; see the R125 scope note in §5.2.

### §3.2 The band becomes legible as a ribbon

Two-row group bodies, **captions centred beneath each group**, vertical separators between groups, and mixed large/small button forms so the primary verb in a group is visually primary. Microsoft's own guidance is explicit that this is what makes a ribbon parse as one; PDF-XChange follows it too. No tab is added and no command moves between tabs — **this is a rendering change to `ribbon_group`, not a taxonomy change.** The three caption-less groups are fixed by construction, because there stops being a code path that skips the helper.

### §3.3 One contextual tab — the reduced form

A single contextual slot at the right of the strip, visually separated, appearing when a tool is armed: `▸ Measure Tool`. Auto-activates on appearance and restores the prior fixed tab on disappearance (decision 024 §3.3(b) rule 1, including its verified egui focus caveat — focus drops to `None` when a focused widget stops being drawn, so it must be re-requested deliberately). Distinguished by a leading `▸` **and** weight **and** tint, never colour alone (R84).

**Its band holds the armed tool's identity and a "bring to front" for the Tool pane — not the tool's controls.** Pass 34.1 already moved every floating property bar into `DockPanel::ArmedTool` (zero `egui::Area` remain), and `ribbon.rs:143-162` records the project rejecting a ribbon-hosted duplicate of exactly this content as the "two mental models" failure. The contextual tab supplies the **navigational signal** Office is recognised by; the dock keeps the controls. Family B (selection-kind tabs) stays blocked on Passes 22.0/23.2, unchanged.

**The six-tab cap survives** — it was always a claim about `RibbonTab::ALL`, the *fixed* strip. The contextual slot is a separate rendering region, not a seventh enum variant, which is what keeps the cap true in code and not merely in a diagram.

### §3.4 Editor chrome — the layer R43 leaves empty

Every form-field widget gets a **dashed, tinted outline drawn by the editor**, plus a type glyph, whenever a form context is active. Never written to any `/AP`; never present in a plain viewing session; never a style the document's own renderer could produce. This makes fields findable, makes the Forms panel's rows locatable on the page, and is the prerequisite for §3.5 — **a resize handle on an invisible rectangle is unusable.** Governed by a new rule (§5.3) rather than by an exception to R43.

### §3.5 The five named capabilities

| Capability | Design | Blocked on |
|---|---|---|
| **Fill form fields** | Already works in the panel for all five fillable types. Add: click a field on the canvas → its panel row scrolls into view and highlights; click a panel row → its widget highlights on the page. Bidirectional, using the chrome layer. | — |
| **Multi-select objects** | Selection already works. Make the **verbs** iterate the whole set — one gesture, one undo entry. Where core has no aggregate verb, N calls collapsed into one `Command` (the `move_nodes` precedent). Mixed-*kind* selection raises no unified verb set and the status bar says why (decision 024 §3.3(b) rule 4, R112). | small core work |
| **Resize form fields** | Eight handles reusing decision 028's handle vocabulary; hit priority from decision 028's stack; live `/Rect` readout in the Tool pane; commit through the wired `GestureInterrupt::Commit`, no floating confirm. | **widgets becoming canvas-selectable** + **Pass 46.0's core resize verb** |
| **Right-click menus (canvas)** | Per selection state; entries map only to verbs that exist. Empty page → Fit/Zoom/Rotate. One object → Delete, Properties. N objects → "Delete selected (N)" — *ship the aggregate verb first*. Widget → Edit in Forms panel, Delete. | §3.5 multi-select row |
| **Right-click menus (object tree)** | The same verb set as the canvas equivalent for that row's type — mirrored, never a diverging second set. Plus "Reveal on canvas", reusing the existing `objects_revealed` field. | — |

**Context-menu honesty differs from ribbon honesty, and the references disagree deliberately.** Microsoft's guidance is that a *ribbon* control should be **disabled, never hidden** ("hiding makes the ribbon presentation unstable"), while a *context menu* should **omit** what does not apply. Both are adopted, per surface. That is the resolution of R124 in §5.1.

### §3.6 Field creation stops being a name-and-tooltip form

The Create Field pane gains the per-type properties that already exist on the specs — and **`/Opt` options for choice fields**, which today is a functional bug (§1.5). This is Pass 20.5's long-owed "per-type detail fields", now with an additional reason to do it.

---

## §4 What this does not attempt

- **No backstage view.** File stays a tab; decision 024 §3.5 settled it and the operator put File in the strip himself.
- **No Alt-key KeyTips.** Real Office parity, genuinely absent, and egui 0.35 has no focus-group primitive (R125's own sourcing) — so it is bespoke Alt-state tracking across every button. P2, and not a driver of "feels incomplete".
- **No ribbon customisation / persistence.** Deliberately out (`ribbon.rs:45-52`).
- **Post-2023 Acrobat is not a ribbon reference** — it replaced its ribbon with an "All tools" left rail. It remains the reference for form-editing *semantics*.

---

## §5 Rule changes

### §5.1 R124 — AMEND. This is the rule the complaint indicts

**Current:** *"Empty space in the command surface stays empty."* No disabled placeholders for unbuilt features; a sparse tab is not a defect.

**Why it was right:** a greyed clone of a working control promises a capability that does not exist.

**Why it must change:** it conflates *unscoped* work with *filed, sequenced* work, and — decisively — **Microsoft's own ribbon guidance says the opposite of R124 for the ribbon specifically**: disable, never hide, because hiding "makes the ribbon presentation unstable". R124 optimised for one kind of honesty and paid for it in the exact symptom the operator reported.

**Proposed:**

> **R124 — Empty space stays empty for UNSCOPED work; scoped work may show a *planned* control; and the rule differs by surface.**
> - **Ribbon / command bands:** a capability with a filed Pass ID may appear as a control in a visually distinct **planned** treatment — reduced opacity **and** a dashed outline **and** disabled, never a plain greyed clone — whose tooltip names the Pass ID and says plainly that it is not yet built. A capability with **no** Pass ID stays absent.
> - **Context menus:** the inverse. Inapplicable entries are **omitted**, not disabled — except the standard editing verbs, which stay present and disabled so their absence is never mistaken for a missing feature.
> - **Mechanically checkable:** a planned-treatment control that cannot cite a Pass ID in its own tooltip is not compliant and must be removed, not left.

This also answers **open operator question (bb)** — *"the operator asked for placeholders, the engineer argued against, and the operator has not ruled"* — which the current instruction rules by a different door.

**★ STATUS 2026-08-08 (operator's direct answer to §7 Q1 — see §8, appended below): the "Proposed" text above is REJECTED for the ribbon bullet.** Operator, verbatim and complete: *"1. no placeholders."* The engineer's original argument against placeholders is the one that stands; the ribbon planned-treatment clause does not ship. The **context-menu bullet is unaffected** — it was never the placeholder question and both Microsoft's guidance and the operator's answer agree on it. This paragraph is left as originally proposed, per the append-only/annotate discipline for this file; `docs/ROADMAP.md`'s R124 entry carries the reversed, currently-governing rule text.

### §5.2 R125 — SCOPE NOTE, not a rewrite

R125 (only the active tab's band is emitted) is correct and load-bearing for keyboard reachability. Nothing in its text exempted the QAT, and that is plausibly *how* decision 024's QAT quietly became tab-gated groups. Append:

> **R125 governs tab-band controls only.** Controls designated Quick Access by decision 024 §3.5(d) — Open, Save, Undo, Redo, page navigation, zoom — are fixed chrome, emitted every frame regardless of active tab.

### §5.3 NEW — editor chrome is not appearance, and R43 does not govern it

**R43 is kept unchanged.** The defect was never R43; it was that nothing filled the gap R43 deliberately leaves.

> **Editor chrome is drawn *about* the document, never *from* it.** An overlay painted each frame to show something R43 declines to synthesise — a dashed outline for a widget with no paintable `/AP`, a type glyph, a resize handle, a selection highlight — is permitted and encouraged, on three conditions: (1) it is never written to any `/AP` or any object; (2) it is unmistakably chrome — a distinct hue and a non-solid stroke, never a style the document's own renderer could produce; (3) it draws only in an editing context, so a plain viewing session shows exactly what R43 already guarantees.

R44 already separates *generated and written to `/AP`* from *displayed*; this separates *displayed from the document* from *drawn by the editor about the document*.

### §5.4 NEW — a verb offered on a multi-selection acts on all of it

> **A verb reachable from an N-target selection acts on the whole selection, or refuses with a stated reason — never a silent subset.** A control that visibly applies to N and affects one misrepresents its scope as surely as R83 forbids misrepresenting its existence. **Checkable at review:** any verb site reading a selection via `.next()`/`.first()` rather than iterating must have its triggering control gated to single-target selections; if the control is offered while N > 1, the mismatch is the defect.

### §5.5 R83, R151, decision 024 §4.4 — KEEP

- **R83** — its 2026-08-07 amendment already gives the checkable test, and §5.4 is an application of it, not a case it misses.
- **R151** — correctly worded; the problem is that it is *cited* at filing time rather than *run*. Fold the call-graph sweep into the Pass-completion checklist beside `cargo tree`/`clippy`.
- **Decision 024 §4.4** (accept/reject narrowing) — confirmed correctly implemented; `GestureInterrupt::Commit` is wired and the floating-Area complaint is closed. The `Finish` control stays in the Tool pane: case-(b) tools still need explicit review, and an explicit "done" remains valuable even where commit also fires implicitly.

---

## §6 Slicing

**P0 — correctness and reachability, GUI-only except where noted**

1. **Multi-target verbs** (§5.4). A silent-data-loss shape; first regardless of anything else.
2. **QAT** (§3.1). Placement move of already-wired controls.
3. **Ribbon group rendering** (§3.2). Captions beneath, separators, two-row bodies; fixes the three caption-less groups by construction.
4. **Editor chrome for form fields** (§3.4, §5.3). Makes the shipped Forms panel usable.
5. **Choice-field `/Opt` in the Create Field pane** (§1.5) — a functional bug, small.

**P1**

6. **Right-click menus**, canvas and Objects tree (§3.5), after 1 and 5 so the menu never shows a verb that narrows.
7. **Widgets become canvas-selectable** — the prerequisite nobody had named for "resize form fields".
8. **One contextual tab** (§3.3).
9. **Per-type field properties** (§3.6) — Pass 20.5's owed remainder.
10. ~~**R124 planned-treatment** applied to the scoped gaps, `add-push-button`'s palette entry first (its core verb shipped today).~~ **RETIRED 2026-08-08 — operator ruled "no placeholders" (§7 Q1, §8). The Pass this item named (`Pass 47.8`) is off the board, not reused.**

**P2** — form-field resize (blocked on Pass 46.0's core verb + item 7), Alt KeyTips, object-tree editing verbs, `delete_annotation` in core (the Comments panel cannot be finished without it).

---

## §7 For Ken — the questions that are yours

1. **R124's planned-treatment (§5.1) is a direct reversal of an engineer argument you never ruled on** (question (bb)). This document reads your instruction as that ruling. Confirm, or say you meant something narrower. **★ ANSWERED 2026-08-08: "1. no placeholders." — the reading above was WRONG; see §8.1.**
2. **The QAT contents.** Open, Save, Undo, Redo, page nav, zoom is decision 024's list. Anything you want added or dropped? **★ ANSWERED 2026-08-08: "your recommendation" — see §8.2.**
3. **Marquee semantics have no industry consensus** — AutoCAD makes direction decide (L→R encloses, R→L crosses), Illustrator touches by default, Inkscape encloses by default. pdfce currently requires **full enclosure**. Keep, or adopt direction-sensitive? **★ ANSWERED 2026-08-08: "your recommendation," plus a new requirement — see §8.3.**
4. **Ribbon density.** A two-row band with captions costs ~60 px of vertical space against a large CAD sheet. Worth it, or do you want a compact mode? **★ ANSWERED 2026-08-08: "your recommendation," plus a new requirement — see §8.4.**
5. **Contextual tabs: one, or the full decision 024 design?** §3.3 proposes the reduced form deliberately. Family B (a tab per selection kind) is a bigger, later thing and stays blocked. **★ ANSWERED 2026-08-08: "your recommendation" — see §8.5.**

**See §8, appended below, for the full answers and their consequences. This numbered list is left exactly as filed, per this file's append-only discipline — the ★ markers point forward, they do not restate the answer.**

---

## §8 Operator answers, appended 2026-08-08 (same-day continuation) — APPEND-ONLY, does not edit §0–§7 above

Ken answered all five §7 questions in one message. This section records
the answers and their consequences; `docs/ROADMAP.md` is canonical
going forward for anything this section and the roadmap might disagree
on (the roadmap gets the fuller, cross-referenced treatment — R124's
own `★★ RE-AMENDMENT` entry under *Standing rules*, the *Open operator
questions* entries for (bb)/(bd)/(be)/(bf)/(bg), and the two new
Backlog buckets in §8.3/§8.4 below).

### §8.1 Q1 — R124 placeholders: ANSWERED, AGAINST §5.1's proposal

Operator, verbatim and complete: **"1. no placeholders."**

§5.1's proposed ribbon planned-treatment clause is **REJECTED**. Only
that clause — the context-menu clause (inapplicable entries omitted,
not disabled) was never the placeholder question and is unaffected.
**(bb) is now resolved the ENGINEER'S original way**: the operator
asked for placeholders when R124 was first written (decision 024 §7);
the engineer argued against them; asked directly here, the operator
ruled for the engineer's argument.

**The governing distinction, now R124's live text (full form in
`ROADMAP.md`):** never show a control for a capability that does not
exist (any surface, not scoped-work-only — this is the original R124
sentence, restored to its full scope); DO show, and disable-with-a-
reason, a control for a capability that exists but does not apply right
now (R83's territory, not R124's — unaffected by this answer, and
already how the Forms panel's read-only/signature/push-button rows
work).

`Pass 47.8` (§6 item 10, R124 planned-treatment styling) is **RETIRED**,
not deferred — the ID is never reused.

**A correction to this document's own §5.1/§0 reading, recorded because
it was wrong.** §5.1 read the operator's *"rewrite any rules about the
GUI that make it less usable and user friendly"* (§0) as itself the
ruling on (bb), in favour of placeholders — filed as
RULED-PENDING-CONFIRMATION, explicitly not RESOLVED, for exactly this
reason. That reading was wrong: the general instruction was about
reachability and the ribbon's undifferentiated appearance, not a
specific ruling on the placeholder question, and the operator's actual
answer, once asked directly, went the other way. **The
PENDING-CONFIRMATION marker is why this cost one filing instead of a
shipped feature** — nothing was built against the wrong reading before
the confirmation arrived.

### §8.2 Q2 — QAT contents: ACCEPTED, with a divergence from §3.1's list

Operator: **"your recommendation."**

QAT = Open, Save a copy…, Undo, Redo. **Page navigation and zoom do NOT
go into the QAT** — they go to the **status bar, bottom-right**,
matching Acrobat/PDF-XChange/Edge/Chrome convention. This diverges from
§3.1's own list (which followed decision 024 §3.5(d) and included page
nav/zoom in the QAT); the divergence is the operator's own
recommendation-request being read against the wider convention
`Ribbon_UX` research had already sourced, not an error in transcription.
`Pass 47.1`'s acceptance criteria change accordingly.

### §8.3 Q3 — Marquee semantics: ACCEPTED as proposed, PLUS a much larger request

Operator: **"your recommendation."** Direction-sensitive selection
ships as proposed in §3.5's sibling material: left-to-right encloses,
right-to-left crosses, **with the marquee rectangle changing appearance
mid-drag** so the active mode is visible while it is being chosen.

**Attached to this answer, substantially larger than the question
asked:** *"all mouse and keyboard features should be customizable with
the ability to save different configurations."* Filed as a new,
UNSCOPED Backlog bucket in `docs/ROADMAP.md` ("Input/keyboard
customisation + saved configurations") — **not** folded into the
marquee item itself, and **not designed by this filing**: a
`pdfce-ui-specialist` dispatch on the design of this bucket (and §8.4's)
is running concurrently.

### §8.4 Q4 — Ribbon density: ACCEPTED as proposed, PLUS a much larger request

Operator: **"your recommendation."** Two-row band with captions
centred beneath, as `Pass 47.2` specifies, **plus an Office-style
collapse toggle** for the compact case.

**Attached to this answer, substantially larger than the question
asked:** *"Ribbon interface should be completely customizable when a
customize option is enabled. layout configurations can be saved and
chosen. should be able to be done with drag and drop and right click to
add tabs, sections etc."* Filed as a new, UNSCOPED Backlog bucket in
`docs/ROADMAP.md` ("Ribbon customisation + persisted layout
configurations") — **not** folded into `Pass 47.2` itself, and **not
designed by this filing** (same concurrent `pdfce-ui-specialist`
dispatch as §8.3).

**This reverses `ribbon.rs`'s own module-doc refusal** (*"What is
deliberately NOT built: reorder/hide UI, and persistence"*, written on
the operator's own then-stated "maybe in the future" framing) — recorded
in `ROADMAP.md` as a reversal, not an oversight being fixed. It also
shares an unnamed prerequisite with §8.3's bucket — a persisted
configuration store — and is adjacent to (but does not itself reopen)
`main.rs:11290`'s dock-layout-is-session-only disclosure; see
`ROADMAP.md`'s Backlog entries for the full cross-referencing.

### §8.5 Q5 — Contextual tabs: ACCEPTED as proposed

Operator: **"your recommendation."** One reduced slot ships now as
`Pass 47.7` (`▸ <armed tool>`, identity + "bring to front" only, tool
controls stay in the dock). §3.3's fuller Family B design (a tab per
selection kind) is **revisited, not retired**, once `Pass 47.6` makes
widgets (and the rest of family (a)) canvas-selectable — that is what
gives Family B a population of selection kinds worth a tab each.

### §8.6 Ledger note

No new Pass IDs, standing rules or decision numbers are minted by this
section. `Pass 47.8` is retired (not renumbered, not reused). Two new
Backlog buckets are filed in `ROADMAP.md` with no Pass ID
(deliberately — `pdfce-ui-specialist`'s concurrent design report comes
before Pass scoping). Full ledger arithmetic, where it matters, lives in
`ROADMAP.md`'s own filing for this continuation, not duplicated here.
