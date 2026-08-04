# Decision 025 — The subpath rung, and the unified level ladder: reconciling decision 023's container model with what Pass 25.0/25.1 shipped

**Status:** Decided (consultant recommendation; engineer to schedule, librarian to file).
**Date:** 2026-08-04
**Requested by:** `pdfce-engineer`, on the `ROADMAP.md` *Next up* flag
*"Level-model reconciliation (decision 023 × Pass 25.x) — FLAG, needs a decision before
Pass 23.2 is scheduled."*

**Amends:** decision **023** (object-tool level navigation). **023 is not edited.**
`docs/decisions/README.md` is explicit: *"Files here are append-only history: never edit a
decision record after the fact — a reversed decision gets a NEW record that references the
old one."* The corrections 023 needs — and this record finds **five statements in 023 that
are now factually wrong, not merely incomplete** — live in §9 of this record, in the
amendment-table style records 022→023 already established. The librarian owes a
forward-reference from 023's `ARCHITECTURE.md` §12 ledger entry to this record.

**Decision number:** **025** — verified against the live ceiling at write time, not
assumed (R106). `python tools/check-ledger-numbers.py --stats` reports
`decision records: 024 -> next free is 025`.

**★ The checker's answer for Pass families and standing rules was STALE when read from
this worktree, and the staleness is exactly the R106 limitation the Pass 25.0 entry
names.** This record is being written in an isolated git worktree pinned at `5df8f26`
(Pass 25.1). The librarian's filing commit `cce2d30` — which heads Pass families **25**,
assigns standing rules **R121–R129**, and claims open-operator-question letters
**(ac)–(aj)** — landed on `pass-8-redaction` *after* that pin, so the checker run inside
this worktree reported `Pass families with headings: up to 21`,
`standing rules: R120 -> next free is R121`. Both were wrong by the time they were read.
The filed `ROADMAP.md` was therefore read directly out of git
(`git show pass-8-redaction:docs/ROADMAP.md`) and the ceilings taken from **it**:

| Ledger | Stale worktree answer | Actual, from `cce2d30` | This record claims |
|---|---|---|---|
| Decision records | 024 → next free 025 | same | **025** |
| Pass families | headed to 21 | headed to **25**; claimed-not-headed 5, 9, 9c, 10, 13, 20, 22, 23, **24** | **family 26** |
| Standing rules | R120 → next free R121 | **R129** → next free R130 | proposes **R130–R134** (librarian assigns) |
| Open operator questions | to (ab) | to **(aj)** | claims **(ak)–(ap)** |

This is the seventh numbering hazard on this project and the second in two days, and it is
worth stating in the record rather than only in a commit message: **R106's checker reports
the ceiling of what is written in the tree it is run in.** A worktree is a tree that can be
behind. The mitigation that actually worked here was reading the ceiling out of the branch
rather than the working copy. Proposed as **R134**, §10.

**Terminology (binding — operator, 2026-08-04; `CLAUDE.md` rule 15).** This record never
writes bare "dimension." **pdf dimensions** are dimensions already present in a
CAD-exported (or otherwise foreign-authored) file — page content or foreign annotations
that pdfce reads, measures against, and must not silently alter. **ce dimensions** are the
dimension objects pdfce itself authors (`/Line` + `/IT /LineDimension` with a baked `/AP`,
their groups, scale, `/Measure` dict and `/PieceInfo` sidecar). Rule 15 does real work in
this record in two places, and both are load-bearing rather than decorative:

- §3's ladder must state what a **ce dimension** does at each rung (it is a level-1 leaf —
  023 §5 — so it has no subpath rung and no node rung, and the ladder must refuse to
  descend into it *by name*, not by silently doing nothing).
- §5's subpath-delete surgery operates on **page content**, which on the operator's own
  files is where the **pdf dimensions** live. The `55 5/8"` printed on a SolidWorks
  drawing is a pdf dimension drawn as glyphs plus witness-line and arrow subpaths inside
  the same 1,194-subpath path object as everything else in that view. **A subpath delete
  can silently destroy half of a pdf dimension** — delete one witness line and the number
  is still printed but no longer points at anything. That is not a hypothetical; it is the
  single most likely first use of the verb on the operator's actual files, and §5.8 makes
  it a disclosure obligation rather than a discovery.

**Cross-references:** `CLAUDE.md` rules 2 (GUI-core separation), 3 (round-trip /
minimal-diff), 4 (fuzzy never sneaky), 6 (documentation-first), 11 (CLI parity), 15
(dimension terminology); `ARCHITECTURE.md` §3, §5 (round-trip), §5.9/§5.11, §12;
`ROADMAP.md` standing rules **R35/R46/R58/R67** (save-mode / surgery exception family),
**R61** (Inkscape behavioural reference only), **R83** (no affordance without capability),
**R84** (never colour alone), **R85** (preview-equals-saved), **R86** (observed working),
**R92** (no hand-duplicated predicates), **R96** (a named refusal must be reachable and
tested firing), **R98** (apply computes, then reports), **R106** (re-read the ceiling),
**R111** (selection enumerates exactly what the renderer paints), **R112** (a selectable
kind carries its verb set — and, per 023 §8 item 7, its **level** — in its type),
**R120** (`EscapeContext`, not positional bools), **R127** (a trace reports committed
state), **R128** (fixed-size panel feeding a fit computation); decisions **011** (§2.1,
§2.5 — the vector-edit cut), **018** (edited state is what the canvas renders),
**022** (`TargetId` enum, annotation selection), **023** (this record's subject),
**024** (ribbon; owns Pass family 24 and the `DismissContextMenu` Escape slot).

---

## 0. Summary

Decision 023 designed a three-rung ladder — **structural container → object → node** —
with double-click to descend and Escape to ascend. Pass 23.2 was to build it. It has not
been built. In the meantime Passes 25.0 and 25.1 shipped a **fourth rung that 023's model
does not contain: the subpath, between object and node** — with the same two gestures. The
`ROADMAP.md` flag correctly says these are not compatible as written.

**This record's answer, in one sentence: the subpath is a genuine rung and it goes exactly
where the measurement put it — between object and node — and the collision dissolves the
moment descent stops being two mechanisms and becomes one state variable.**

Seven findings drive the record. Each was verified in the shipped code, not inferred.

1. **The subpath rung is outside 023 §1.2's heuristic-grouping refusal, on all three of
   that refusal's own grounds** — subpaths are *in the file* (one `Subpath` per
   `m`/`re`/`h`-reopen, produced by a deterministic parse with no thresholds and no
   parameters), *stable* (an ordinal is a position in construction-operator order), and
   *disclosed* (the readout already names the count). §1.4.

2. **But it smuggles two smaller instances of the same problem back in, and both were
   missed.** (a) `hit_test_subpaths` orders by **outline distance**, with an interior fill
   hit promoted to 0.0 — a metric choice, not a fact in the file — and **there is no
   subpath click-cycle**, so the second-nearest part under the pointer is unreachable at
   all. The object rung has had `ClickCycle` and its *"2 of 3 at this point"* disclosure
   since Pass 9a; the subpath rung shipped without the twin. `main.rs:2176` takes
   `.first().copied()` and `depth_after_click` accepts a single `Option<usize>`, so this
   is structural, not a missing branch. §1.4.2. (b) Subpath ordinals are **unstable under
   an edit to their own object** — delete part #3 and parts #4.. renumber. §1.4.3, §6.

3. **Five statements in decision 023 are now factually wrong**, not merely incomplete —
   including one acceptance criterion (C6) and one shipped-string specification that would
   ship a lie on the operator's own files. §9.

4. **`Subpath` carries no token range and no byte span.** `PathObject` has `tokens:
   TokenRange` and `bytes: ByteSpan`; `Subpath` has `start`, `segments`, `closed` and
   nothing else (`decompose.rs:224-232`). So **neither subpath verb the roadmap owes is
   expressible against today's model** — not "hard," *not expressible*. This is the single
   largest unstated cost in the Next-up entry. §5.2.

5. **★ The sharpest new hazard, and it is 023 §1.4's form-aliasing trap one object space
   down: `DeleteWouldMoveNextSubpath`.** After `h`, `close_subpath` sets
   `pa.current = pa.subpath_start` and `needs_move = true`
   (`decompose.rs:1859-1868`); a following `l`/`c`/`v`/`y` then opens a new `Subpath`
   whose `start` is **inherited from the closed subpath's start**, carried by no operand
   of its own (`current_for_segment`, `decompose.rs:1803-1824`). Excise the preceding
   subpath and the follower's start point silently changes. The edit is byte-minimal,
   byte-verifiable, passes every round-trip check — and moves geometry the operator did
   not select. **Minimal-diff discipline cannot catch this. Only the named refusal can.**
   §5.5.

6. **Pass 25.1's Escape is wrong under any ladder, and the fix is a deletion.**
   `Action::ClearCanvasSelection` (`main.rs:4878-4886`) does two things: clears the
   selection set *and* sets `entered = None`. That is two rungs (or five) collapsed into
   one press, and it directly contradicts 023 §3.2's stated, testable property — *"Escape
   walks all the way out, one step per press, and never skips a step."* The correction is
   to delete line 4885 and add a new `LeaveLevel` outcome above `ExitTool`. §3.5.

7. **The node ceiling problem 023 §4.6 spent a section on largely evaporates.** 023
   reasoned about showing anchors for *"a whole group"* — 41,208 handles, a 2,000 ceiling,
   an unclickable mat. Under the ladder the node rung shows the anchors of **one entered
   subpath**: on the measured SolidWorks page that is 6,681 anchors ÷ 1,194 subpaths ≈ **6
   per part**. The ceiling stays (a hatch boundary can be one long subpath) but it stops
   being the common case. §4.6.

**What this record does not do.** It does not redesign 023's container half — §2's laminar
interval model, §2.3's `Container`/`ContainerKind`, the cycle guard and depth cap reused
from `pdfce-render`, and the `ContentPath` payload are all adopted unchanged. It does not
decide the ceiling value, the breadcrumb's visual design, or the operator-facing noun for
a node. §11.

---

## 1. Q1 — Does the subpath rung belong in the ladder, and where?

### 1.1 The measurement the argument has to survive

Recorded this session at
`C:\personal_rag\pdf\lesson_20260804_cad_export_one_path_object_per_drawing_view.md` and
reproduced in the Pass 25.0 roadmap entry:

| Fact | Value |
|---|---|
| Form XObjects on page 1 of the operator's SolidWorks export | **0** |
| Marked-content sequences (023's other container kind) | **0** observed; `decompose.rs:1695` discards them anyway |
| Objects on page 1 | ~5,900 |
| Object 5870 | **one** stroked path — **1,194 subpaths, 6,681 anchors**, bbox `590.7,500.2 → 1140.9,1000.3` (a 550 × 500 pt isometric view) |
| The other three views | 950, 881, 742 subpaths — also one object each |

The corroborating pre-existing finding
(`C:\personal_rag\pdf\lesson_20260729_solidworks_pdf_publisher_flat_vector_no_xobjects.md`)
says the same about this producer generally, so this is a producer property, not one
file's accident.

**Two consequences, and the second is the one 023 did not see.** First, 023's own §1.3 is
right that container descent has nothing to descend into here. Second — and this is what
makes the record necessary — **the object rung is not the bottom of the useful ladder on
this file either.** Per-object hit-testing selects an entire drawing view for a click
anywhere in it. The operator's literal report was *"how do I click on individual lines and
nodes to move or delete them?"* Without the subpath rung the answer is: you cannot, at
all, for any line on the page.

So the design question is not "is a fourth rung nice." It is: **the only rung that makes
the operator's primary file selectable is one the level model omits.**

### 1.2 Decision

**The subpath is a fourth rung, inserted between object and node — option (a). The rung is
UNCONDITIONAL: it exists for every path object, including one whose only member is a
single subpath. It is never skipped, and the count of what is below it is disclosed at the
rung above, so descent is never a leap in the dark.**

```
Page ─▶ Container … Container ─▶ Object ─▶ Subpath ─▶ Node
 0        1     …     k            k+1       k+2       k+3
       (variable depth, 0..k)     (always)  (paths)   (paths)
```

Container depth `k` is a property of the file (023 §1.3 — this stays true and is the part
of 023 this record most agrees with). Everything from `Object` down is fixed. On the
operator's files `k = 0`, so the ladder they will actually walk is
**Page → Object → Subpath → Node**: four rungs, three double-clicks.

### 1.3 The four options, argued — and what each costs an operator who does not know how
their file is structured

That last clause is the whole test. The operator cannot see, before clicking, whether a
page has form XObjects, whether an object holds 1 subpath or 1,194, or whether the thing
under the cursor is a path at all. **A ladder design is good exactly to the degree that it
does not require that knowledge.**

#### (a) Insert as a fourth rung — container → object → subpath → node ✅ **CHOSEN**

*Consequence for the uninformed operator:* one rule covers everything — *one double-click
goes one level down, one Escape comes one level up, and the readout tells you which level
you are on and what is below it.* They never have to know the file's structure, because
the tool states it as they walk. On a flat CAD page they do three double-clicks to reach a
point; on a nested title-block page they do five; in both cases they did the same thing and
the breadcrumb showed them where they went.

*Cost, named honestly:* the ladder is deeper than 023's, so Escape-to-exit-the-tool takes
`k+4` presses instead of `k+3` (023 §3.2 already put `LeaveLevel` above `ExitTool`).
§3.6 addresses this without adding a gesture.

#### (b) Treat subpath as a *variant* of the node rung ❌

The idea: the terminal rung has two modes — "nodes of the object" or "parts of the
object" — selected by something.

*Consequence for the uninformed operator:* the deepest rung means different things on
different objects, and **the selector for which meaning applies is the subpath count** —
i.e. the fact the operator cannot see. Double-click a line: you get points. Double-click a
view: you get parts. Same gesture, same-looking objects, two outcomes, no way to predict
which.

*And it breaks two things structurally, which is why this is rejected on engineering
grounds and not only on UX grounds:*

1. **It makes the terminal rung non-terminal.** After picking a part you still want its
   points, so the "variant" rung has to descend into itself. That is a fourth rung wearing
   a disguise, with a worse state type.
2. **It forces a second node index space.** `plan_move_node`'s `node_index` is into the
   object's anchors in *decomposition order* — flattened across every subpath —
   and that is the space `anchor_count` reports and `node-move --node N` addresses
   (`edit.rs:2129-2145`). A subpath-scoped node index would be a second numbering for the
   same anchors, which must then be kept in agreement with the first. That is R92's exact
   shape (a hand-duplicated derivation drifts) and decision 011's Z2 defect class (two
   traversals that must agree and eventually do not).

   **This is worth stating as a positive commitment of option (a), because it is a real
   dividend:** under the four-rung ladder the node rung's indices stay **object-scoped**.
   The subpath rung *filters which anchors are shown and clickable*; it does not renumber
   them. `node-move --node N` keeps its meaning byte-for-byte — precisely the way 023 §2.1
   kept `object-move --object 2`'s meaning by refusing to re-parent the flat list.

#### (c) Make "enter a container" and "enter an object" different gestures ❌

The idea: double-click descends into containers; some other gesture (Ctrl+double-click,
Enter, a context-menu verb) descends into an object's parts.

*Consequence for the uninformed operator:* they must know **which kind of thing is under
the cursor before choosing the gesture** — and containers are invisible. A form XObject
renders as ordinary artwork; a marked-content sequence renders as nothing at all. The
operator would have to consult the readout to pick a gesture, then perform it, then find
out whether they guessed right. This inverts the relationship the design is for: the tool
should tell them where they are, not require them to tell it.

*It also doubles the gesture vocabulary permanently* for a distinction that has no visual
correlate, and it gives Escape an unanswerable question — if descent used two gestures,
does one Escape undo "a container step" or "an object step," and how does the operator
know which they last took?

#### (d) Make the rung conditional on the object having > 1 subpath ❌

The idea: a 1-subpath object skips the parts rung and double-click goes straight to points.

This is the most defensible rejected option and deserves the longest treatment, because it
is what an implementer will propose on ergonomic grounds.

*Consequence for the uninformed operator:* the same gesture on two objects that look
identical lands them on different rungs. A dimension witness line drawn as one `m l S` and
a leader drawn as `m l m l S` are indistinguishable on screen; double-click the first and
you are editing points, double-click the second and you are picking parts. **The operator
learns a rule that is false half the time.**

*And it has three concrete engineering costs 023's own reasoning already forbids:*

1. **The skip predicate must be duplicated in the ascent direction, exactly.** If descent
   skips a rung, Escape must skip the same rung, or the ladder is not a ladder — press
   Escape and land somewhere you were never standing. Two copies of "does this object have
   more than one subpath," in code paths that are edited by different Passes. R92.
2. **It destroys the testable property 023 §3.2 built the Escape chain around.** *"Escape
   walks all the way out, one step per press, and never skips a step"* becomes *"never
   skips a step, except when it does, based on a count the test has to construct."* The
   property was chosen because it is checkable as one unit test over `EscapeContext`
   (023 C5). Conditional rungs turn that into a matrix.
3. **It buys nothing that disclosure does not buy more cheaply.** The friction (d) removes
   is *one extra double-click on simple objects*. The friction is real only if the
   operator does not know what is down there. **They can know**, for free:
   `path.subpaths.len()` is already read at `main.rs:11205` for the current readout. State
   the count at the object rung — *"drawn as a single part"* vs *"drawn as 1,194 parts"* —
   and the extra press stops being a leap in the dark. It becomes a confirmation of
   something already on screen.

**So the single-subpath case is handled by disclosure, not by a skip**, and at that rung
the sole subpath is auto-selected on entry (there is no choice to make), so the operator's
next double-click reaches points with no intermediate decision. The cost is one press; the
saving is a rule that is always true. See §4's readout matrix row *"drawn as a single
part."*

**Recorded as operator question (ak)** — this is the one place in the record where a
reasonable operator could prefer the other answer, and it is cheap to change later
(`descend()` gains one branch), so it is worth confirming rather than discovering.

### 1.4 ★ Is the subpath rung inside 023 §1.2's heuristic-grouping refusal?

023 §1.2 refused pdfce-side spatial/heuristic grouping in strong terms: *"a hidden
heuristic that silently changes what one click selects is a silent auto-apply wearing a
hint's clothes … also unstable under editing — move one object and the clusters re-form,
so the same double-click means different things before and after an unrelated edit."*

The engineer's belief is that subpaths are outside that refusal. **They are — and the
refusal's own three grounds are the right test to apply, so let us apply all three
explicitly rather than assert the conclusion.**

#### 1.4.1 The three grounds, tested

| 023 §1.2's ground for refusing | Heuristic cluster | Subpath | Verdict |
|---|---|---|---|
| **Is it in the file?** | No — inferred from proximity/style thresholds pdfce chooses | **Yes.** `m x y` opens one; `re x y w h` is a complete closed one; `h` closes one. One `Subpath` per construction-operator group, built by a linear parse (`decompose.rs:1524-1567`, `rect()` at `:1872`) with **no thresholds, no tunable parameters, and no clustering step**. §8.5.2 defines the boundaries; pdfce reads them. | **Outside** |
| **Is it stable under editing?** | No — move one object and clusters re-form, so an *unrelated* edit changes what a double-click means | **Yes, against unrelated edits.** A subpath's ordinal is its position in its own object's construction order. Editing a *different* object cannot change it. (It is not stable against edits to the *same* object — §1.4.3.) | **Outside**, with a named obligation |
| **Is it disclosed, or silent?** | Silent — the operator cannot see the cluster boundary | **Disclosed.** Pass 25.1 already outlines each subpath (`SUBPATH_OUTLINE_COLOR`, deliberately *not* the selection accent) and the readout names the count. R84 satisfied by construction — the state is announced in words, not carried by colour alone. | **Outside** |

**Verdict: the subpath rung is genuinely outside 023 §1.2's refusal.** It is not a
pdfce-invented grouping being presented as structure; it is structure pdfce was previously
throwing away. The correct framing — and it is the same framing 023 §0 finding 2 used for
form XObjects — is that this is **a third instance of R111's paint/select asymmetry**: the
renderer strokes each subpath as a distinct visible mark; selection saw one object. 023
found two instances (ce-dimension annotations; form XObjects). This is the third, it was
the one the operator actually hit, and Pass 25.0/25.1 closed it. R111's justification is
stronger again than 023 knew.

#### 1.4.2 ★ But one thing IS smuggled back, and it was missed: the ordering is a choice, and it has no cycle

`hit_test_subpaths` (`hit.rs:277-318`) sorts candidates by **distance to the subpath's
outline**, and promotes an interior fill hit to **distance 0.0** so it outranks a
neighbouring outline the pointer merely came close to. Pass 25.0's doc comment defends
nearest-first well: subpaths within one object share a single painting operator, so there
is no z-order among them to inherit, and *"any other order would be arbitrary dressed up
as meaningful."* That reasoning is correct and this record adopts it.

**But "nearest first" is still a metric resolving an ambiguity, and the object rung
already has an answer for exactly that situation which the subpath rung did not
inherit.** `canvas::ClickCycle` (`canvas.rs:801-840`) exists because a click at a point
where several objects overlap has several right answers; it resolves them by cycling on
repeated clicks at the same point, with the disclosure *"2 of 3 at this point"* and four
documented staleness resets.

At the subpath rung there is no such thing. The evidence is structural, not a missing
branch:

- `main.rs:2173-2177` — `subpath_hits(o, canvas_pos, tol).first().copied()`. The rest of
  the `Vec` is computed and thrown away.
- `canvas::depth_after_click` (`canvas.rs:260-282`) accepts `subpath_hit: Option<usize>`
  — a single value. There is no place to put an ordinal.

**Consequence on the operator's own files:** in a CAD view, a hatch crossing a line, a
witness line running along an edge, and two coincident view boundaries are all routine.
The part *under* the nearest one is unreachable — clicking again returns the same part
forever. The operator's report will be *"I can't select the line under the hatch,"* and it
will look like a hit-test bug rather than a missing affordance.

There is also an R127 angle: a diagnostic or readout that names the picked part without
naming how many were under the pointer is reporting a resolved value while discarding the
evidence that it was a choice. The object rung says *"2 of 3."* The subpath rung says
nothing.

**Decision: the subpath rung inherits `ClickCycle`.** Not a new mechanism — the same one,
generalised so its `produced` field can name a rung-qualified target rather than only a
`TargetId`. Scoped into **Pass 26.0** with its own acceptance criterion (§7, F6). This is
the *"same problem back"* the engineer asked to be checked for, found, and closed.

#### 1.4.3 The second smuggled item: ordinal stability under an edit to the same object

023 §1.2's second ground was instability. Subpaths pass it against *unrelated* edits and
**fail it against an edit to their own object**: delete part #3 of a 1,194-part object and
parts #4…#1,193 renumber. This is the identical fragility content-object indices already
have, and the shipped mitigation is `prune_canvas_selection` (`main.rs:2199-2216`), which
nukes `entered` and `click_cycle` unconditionally on every edit, undo and redo, with an
explicitly correct reason: *"after a content rewrite the SAME paint-order index can name a
different object."*

That mitigation is right and must not be weakened. But it has a consequence nobody has had
to live with yet, because Pass 25.1 shipped **no verbs**: the moment subpath verbs exist,
**every subpath edit throws the operator back to Page.** They descend three rungs into a
1,194-part view, delete one line, and are ejected to the top with nothing selected. On a
page with 5,900 objects, re-finding their place is the dominant cost of the operation.

This is not a reason to keep the level across an edit naively — the ordinals genuinely may
no longer mean the same thing. It is a reason to make **level survival its own piece of
work with its own failure mode**: re-derive the path against the fresh decomposition,
verify each rung still resolves, and drop to the **nearest surviving ancestor rung** with a
disclosure when it does not. That is Pass **26.2** (§6, §7).

---

## 2. Q2 — The gesture collision, and why it dissolves

### 2.1 The collision is real only under two mechanisms

The `ROADMAP.md` flag states it correctly: *"Pass 23.2 cannot add its own double-click
descend and Escape ascent beside Pass 25.1's without deciding what a double-click means
when both a container and a subpath level are available."*

That framing already contains the answer. **A double-click never has to decide "container
or subpath," because they are not both available at the same moment.** They are at
different rungs, and the current rung is always known — it is one variable (§3). The
collision exists only if descent is implemented twice: once by 23.2 reading *"is there a
container under the cursor to enter,"* once by 25.1 reading *"is there an object under the
cursor to enter."* Two predicates answering "descend?" is R92's shape, and R92 is exactly
why Pass 25.1 already routed both of its own click paths through one
`OpenDoc::apply_click_depth` (`main.rs:2141-2147`, which says so in its own doc comment).
**The fix is to extend that discipline one rung further, not to arbitrate between two
mechanisms.**

### 2.2 The unified ladder — what one double-click does at each rung

| # | Rung | What is selected here | One double-click → | Escape → | Click-outside → |
|---|---|---|---|---|---|
| 0 | **Page** | the outermost container on this page, or a bare object where there is none | descend into the clicked thing: its container if it has one, else the object itself | falls through to slot 3 (`ExitTool`) — nothing to leave | clear selection (stay at Page) |
| 1..k | **Container** | the child container or object at this depth | descend one container; at the innermost container, descend to the clicked **object** | ascend one container | ascend to the **nearest common ancestor** of this rung and the clicked thing, selecting the clicked thing's descendant of that ancestor (023 §3.4) |
| k+1 | **Object** | one path / text / image object | **enter it** → Subpath rung, nearest subpath selected (`hit_test_subpaths`, cycling per §1.4.2) | ascend to the enclosing Container, or Page when `k = 0` | as above; with `k = 0` this is "select the clicked object, stay at Object" |
| k+2 | **Subpath** ("part") | one subpath of the entered object | **enter it** → Node rung, that subpath's anchors shown and clickable | ascend to Object | click on another part of the same object → re-pick (no rung change); click outside the object → ascend to Object and select the clicked object |
| k+3 | **Node** ("point") | one anchor, **object-scoped index** (§1.3(b)) | **nothing to enter — reported, never a silent no-op** (023 §1.3 disclosure 2): *"This is the deepest level — there is nothing below a point."* | ascend to Subpath | ascend to Subpath and re-pick, or to Object if outside |

**Three rules govern the whole table, and they are what make it a ladder rather than a
collection of special cases:**

> **L1. One double-click = exactly one rung down. One Escape = exactly one rung up.
> Neither ever skips.**
>
> **L2. A rung with no members on this file is not entered — it is *reported*.** A page
> with no containers: the Page rung's double-click descends straight to Object, and the
> readout says the page has no groups. A text object: the Object rung's double-click does
> not enter, and the readout says text has no parts. Never a silent no-op (023 §1.3
> disclosure 2, whose original motivation was `ROADMAP.md` Pass 12.M2c bug #4 —
> *"post-second-click clicks are dead with no hint"*).
>
> **L3. A ce dimension is a level-1 leaf and the ladder refuses to descend into it BY
> NAME.** 023 §1.2's third rejection is permanent, not deferred: descending into a ce
> dimension's `/AP` would expose the leader, ticks, arrowheads and label as separately
> deletable pieces of a measurement — *"the sneakiest possible outcome."* Under the
> four-rung ladder that refusal needs restating, because the subpath rung makes "descend
> into the parts of this thing" a *general* gesture, and a `/AP` form stream **does**
> contain subpaths. The refusal is therefore now load-bearing where before it was
> theoretical: `TargetId::Annot` has no subpath rung, and a double-click on a selected ce
> dimension reports *"a measurement is one object — it has no parts."* Its route to a
> geometry change is 023 §5's `Re-measure` verb, not descent. **A pdf dimension is the
> opposite case and gets no exemption at all** — it is page content, its witness lines and
> arrowheads *are* subpaths of some path object, and the ladder descends into them exactly
> as into any other artwork. That asymmetry is correct (provenance, per rule 15) and it is
> the reason §5.8 exists.

### 2.3 What changes about Pass 25.1's shipped behaviour

Three changes, one non-change. Stated at this precision because the flag asked what a
double-click *becomes*, not what it should be in principle.

| Behaviour | Shipped in 25.1 | Under the ladder | Why |
|---|---|---|---|
| Double-click an object at Page | enters it, nearest subpath selected | **unchanged** (with `k = 0`) | this is already rung Page → Object |
| Ordinary click while inside, on a part | re-picks that part | **unchanged**, plus cycling (§1.4.2) | ordinary clicking at the current rung is not a rung change |
| Ordinary click while inside, hitting no part of the entered object | leaves entirely (`depth_after_click` → `None`) — then the object-level path selects whatever was clicked | **unchanged when `k = 0`**, because "leave to Page and select the clicked object" *is* the nearest common ancestor when there are no containers. **Changes when `k > 0`:** ascent stops at the common ancestor, not at Page. | 023 §3.4 |
| Double-click a part | nothing (25.1 has no node rung) | **enters the Node rung** | new |
| Escape | `Action::ClearCanvasSelection` clears the selection **and** sets `entered = None` (`main.rs:4878-4886`) | **pops exactly one rung, touches the selection set not at all** | §3.5 |

---

## 3. Q3 — State representation: one type

### 3.1 The two proposals to reconcile

| Source | Shape | What it gets right | What it cannot express |
|---|---|---|---|
| 023 §3.3 | `group_depth: usize` inside `EscapeContext`, plus a container path implied by §2.5's `ContentPath { stream, index }` | depth as a *number* is what the Escape chain needs; the container path is what the breadcrumb needs | no object rung below the container, no subpath, no node |
| Pass 25.1 (shipped) | `entered: Option<canvas::EnteredObject { object: usize, subpath: Option<usize> }>` (`canvas.rs:225-232`, `main.rs:1729`) | the object→subpath pair, and a genuinely pure `depth_after_click` with 7 tests | no containers, no node, and `Option<…>` conflates "at Page" with "no document" |

### 3.2 Decision — `LevelPath`

**One type, in `pdfce-gui`, carrying the whole path from Page to the current rung. It
replaces `EnteredObject` outright; `EnteredObject` is deleted, not extended.**

```rust
/// WHERE THE OPERATOR IS STANDING in the level ladder, and what is picked at
/// that rung. Decision 025 §3.
///
/// # This is view state and it lives here, deliberately
///
/// Decision 023 §11 states the invariant this type has to honour: `PageObjects`
/// describes the DOCUMENT; where the operator is standing is the shell's
/// business. Putting a rung into core would make the eventual WASM fork inherit
/// a UI mode (`CLAUDE.md` rule 2, `ARCHITECTURE.md` §3).
///
/// # `LevelPath` is not `TargetId`, and merging them would be wrong
///
/// `LevelPath` is where you are; `TargetId` is what is selected. They are
/// genuinely different: a marquee can select forty objects while the operator
/// stands at the Page rung, and standing inside one object does not by itself
/// select anything. Decision 022 §2 owns the selection handle; this type owns
/// the standing position; `canvas_selection: BTreeSet<TargetId>` keeps its
/// meaning unchanged.
///
/// # Prefix invariant
///
/// The four fields form a PREFIX CHAIN: `subpath.is_some()` implies
/// `object.is_some()`; `node.is_some()` implies `subpath.is_some()`. Enforced by
/// construction — the fields are only ever written by `descend`/`ascend`/
/// `Default`, and `debug_assert!(self.is_well_formed())` guards both. A path
/// that skips a rung is UNREPRESENTABLE in practice, which is R112's shape
/// applied to level (023 §8 item 7's strengthening).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LevelPath {
    /// Containers entered, OUTERMOST FIRST. Each entry indexes
    /// `PageObjects::containers` (decision 023 §2.3). Empty on a
    /// container-free page — the common case on CAD exports, and the reason
    /// `Default` is the Page rung rather than a sentinel.
    ///
    /// `Vec<u32>`, not decision 023 §2.5's `SmallVec<[u32; 2]>`: `smallvec`
    /// is NOT a workspace dependency (verified — zero hits across every
    /// `Cargo.toml`), and an empty `Vec` does not allocate, so on the files
    /// this ships for the heap cost is exactly zero. Adding a dependency to
    /// avoid an allocation that never happens would fail rule 13's
    /// cost/benefit on its face.
    pub containers: Vec<u32>,
    /// The object entered within the innermost container (or within the page
    /// when `containers` is empty). Paint-order index in THAT stream's own
    /// flat index space — decision 023 §2.1's byte-for-byte-unchanged index
    /// space, per stream.
    pub object: Option<u32>,
    /// The subpath ("part") entered within `object`.
    pub subpath: Option<u32>,
    /// The node ("point") picked within `subpath`.
    ///
    /// ★ The index is OBJECT-SCOPED — decomposition order across the whole
    /// object, the space `vector::anchor_count` reports and `node-move --node`
    /// addresses (`edit.rs:2129-2145`). It is NOT renumbered per subpath. The
    /// subpath rung FILTERS which anchors are shown and clickable; it does not
    /// create a second numbering. See §1.3(b) — a second numbering would be
    /// decision 011's Z2 defect class and R92's shape.
    pub node: Option<u32>,
}

/// Which rung a `LevelPath` names — the value the readout, the breadcrumb and
/// every verb's applicability test switch on. An exhaustive match here is what
/// makes "this verb does not exist at this rung" a compile error rather than a
/// silent no-op (R112).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rung { Page, Container, Object, Subpath, Node }

/// What one attempted descent actually did — a RETURN VALUE, not a convention.
///
/// Decision 023 §1.3 disclosure 2 requires that a double-click with nothing to
/// descend into is never a silent no-op. Making that a returned outcome rather
/// than a caller-side convention is R98's shape (the operation computes, then
/// reports) and it means the disclosure cannot be forgotten at a call site.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DescendOutcome {
    /// Went down one rung; the payload is the rung now standing on.
    Descended(Rung),
    /// This rung exists but has nothing to enter on THIS file — a page with
    /// no containers, an object with no subpaths (text/image/form), a ce
    /// dimension (L3). The readout names the reason.
    NothingToEnter(Rung),
    /// Already at Node. There is nothing below a point.
    AlreadyDeepest,
}

impl LevelPath {
    /// Rung index: 0 = Page. Each container adds one, then object, subpath,
    /// node. This is the ONE number `EscapeContext` needs (§3.5).
    #[must_use] pub fn depth(&self) -> usize;
    /// Which rung this path names.
    #[must_use] pub fn rung(&self) -> Rung;
    /// Pop exactly one rung. Returns `false` at Page — nothing to pop — which
    /// is how `resolve_escape` knows to fall through to the next slot.
    pub fn ascend(&mut self) -> bool;
    /// Push exactly one rung, given what the click resolved to at the rung
    /// below. Never pushes more than one; never skips.
    pub fn descend(&mut self, into: Descent) -> DescendOutcome;
    /// The nearest ancestor rung that is still valid against a fresh
    /// decomposition, dropping trailing rungs that no longer resolve.
    /// Pass 26.2 (§6) — the honest answer to an edit renumbering ordinals.
    #[must_use] pub fn truncated_to_valid(&self, model: &PageObjects) -> (Self, Option<Rung>);
    #[must_use] fn is_well_formed(&self) -> bool;
}
```

`Descent` is the small resolved-hit payload the caller assembles from the providers —
container index / object index / subpath index / node index as applicable. It exists so
`descend` stays pure and testable with plain integers, which is the property that made
Pass 25.1's `depth_after_click` testable with seven tests and no egui frame. That property
is preserved deliberately; it is the single best thing about the shipped code and losing it
would be the easiest mistake to make in this migration.

### 3.3 What Pass 23.2 (or whichever Pass lands the ladder) owes — the concrete migration

Named files, named symbols, in dependency order.

| # | Site | Today | Becomes |
|---|---|---|---|
| 1 | `canvas.rs:225-232` | `pub struct EnteredObject { object: usize, subpath: Option<usize> }` | **deleted.** Its two fields become `LevelPath::object` / `::subpath` (as `Option<u32>`). |
| 2 | `canvas.rs:260-282` | `depth_after_click(entered, double, object_hit, subpath_hit) -> Option<EnteredObject>` | `next_level_after_click(&LevelPath, double, &Descent) -> (LevelPath, DescendOutcome)` — same purity, same no-egui testability. |
| 3 | `canvas.rs:1489-1553` | 7 tests over `EnteredObject` | port field-for-field (they encode the right rules and must not be rewritten from scratch), **plus** new cases: descend Object→Subpath→Node; ascend one rung at a time; `NothingToEnter` on a text object; `AlreadyDeepest`; `k > 0` container descent; the prefix invariant. |
| 4 | `main.rs:1723-1729` | `entered: Option<canvas::EnteredObject>` | `level: canvas::LevelPath` — **no `Option`.** Page is the zero value and `Default` gives it, which removes the shipped conflation of "at Page" with "no document." |
| 5 | `main.rs:1892` | `entered: None` in the constructor | `level: LevelPath::default()` |
| 6 | `main.rs:2155-2182` | `OpenDoc::apply_click_depth` | **keeps its name and its one-method discipline** (R92, its own doc comment at `:2141-2147`). Body swaps the two-way probe for a rung-aware probe: at Page/Container probe containers then objects; at Object probe subpaths (with cycling, §1.4.2); at Subpath probe that subpath's anchors. |
| 7 | `main.rs:4878-4886` | `Action::ClearCanvasSelection` clears the selection **and** sets `doc.entered = None` | **delete the `entered = None` line.** The action reverts to doing exactly what its name says. §3.5. |
| 8 | new | — | `Action::LeaveLevel` → `doc.level.ascend()`. Never touches `canvas_selection`. |
| 9 | `canvas.rs:606-620` | `resolve_escape(tool_active, gesture_discardable, canvas_selection_nonempty)` | `resolve_escape(&EscapeContext)` with the R120 named-field struct, `depth: usize` field, new `LeaveLevel` outcome at slot 2. §3.5. |
| 10 | `main.rs:2199-2216` | `prune_canvas_selection` sets `self.entered = None` unconditionally | `self.level = LevelPath::default()` — **semantics unchanged**, and the existing doc comment's reasoning is still exactly right. Stated explicitly so nobody "improves" it away; Pass 26.2 changes it, and only with re-validation (§6). |
| 11 | `main.rs:11195-11215` | `selection_readout`'s `if let Some(entered)` block | the rung readout matrix, §4. |
| 12 | `ui_text.rs:2013-2043` | `entered_object_readout` (two-fragment assembly) + `entered_object_tooltip` | one complete catalog string per matrix row, §4.2. |
| 13 | `main.rs:11497-11512` | subpath outline draw, keyed on `entered.subpath` | keyed on `level.rung()`; the Node rung adds anchor handles; a Container rung gets its own treatment (**`pdfce-ui-specialist` owns the visual**, §11). |
| 14 | 022/023 §2.5 | `TargetId::Content(u64)` | `TargetId::Content(ContentPath { stream, index })` — payload only, **no substrate change** (023 §2.5, R112). Adopted unchanged; this record adds nothing to it. |

**Two things this migration must NOT do**, because both are the path of least resistance:

- **It must not put `LevelPath` in `pdfce-core`.** 023 §11 named this risk in advance and
  it is sharper now: the subpath rung is *so* tied to `PageObjects` that "just put the
  current rung next to the objects it indexes" reads as tidy. `cargo tree -p pdfce-core`
  is the gate; the acceptance criteria carry it (§7, F8).
- **It must not build the container forest in the GUI provider.** Also 023 §11. Container
  detection is content-stream parsing; that is core. The provider's job stays what Pass
  25.1 made it — a read-only window (`subpath_hits`, `subpath_bounds_canvas`) onto core
  functions, so no GUI code re-derives geometry.

### 3.4 Why `LevelPath` is one type and not "023's field plus 25.1's field"

Because two variables cannot express the prefix invariant, and the failure is silent.
`group_depth: usize` alongside `entered: Option<EnteredObject>` admits `group_depth = 2`
with `entered = Some(..)` — is the operator inside an object inside two containers, or
inside an object *and* two containers deep in an unrelated part of the tree? Nothing in
the types answers it; the Escape chain reads one, the readout reads the other, and they
disagree the first time one is updated without the other. That is exactly decision 022
§2's rejected option (d) — a partition maintained by convention instead of by type — and
this record refuses it for the same reason 022 did.

### 3.5 The Escape chain, and the one line to delete

Today (`canvas.rs:606-620`, call site `main.rs:5190`):

1. `CancelGesture` — tool active with a discardable gesture
2. `ExitTool` — tool active
3. `ClearCanvasSelection` — substrate selection non-empty
4. `FallThroughToRailClear`

**The unified chain** — 023 §3.2's table, adopted with one rename and one clarification:

| # | Outcome | Owner | Fires when |
|---|---|---|---|
| 0 | `DismissContextMenu` | decision 024 / navigation Pass | a context menu is open |
| 1 | `CancelGesture` | shipped | tool active with a discardable gesture |
| **2** | **`LeaveLevel`** | **this record** | `cx.depth > 0` — pops exactly one rung |
| 3 | `ExitTool` | shipped | tool active |
| 4 | `ClearCanvasSelection` | shipped | `canvas_selection` non-empty |
| 5 | `FallThroughToRailClear` | shipped | otherwise |

```rust
pub struct EscapeContext {
    pub context_menu_open: bool,      // decision 024 / navigation Pass
    pub gesture_discardable: bool,
    pub tool_active: bool,
    pub depth: usize,                 // this record: LevelPath::depth(), 0 = Page
    pub canvas_selection_nonempty: bool,
}
pub fn resolve_escape(cx: &EscapeContext) -> EscapeOutcome
```

The named-field struct is R120, already a standing rule, and it is now **more** necessary
than when 023 proposed it: three Passes are queued to edit this function (024's slot 0,
this record's slot 2, and whatever the navigation Pass turns into), and a fifth positional
`bool` makes `resolve_escape(true, false, true, false, true)` a call where a transposition
compiles, type-checks and silently reorders Escape.

**Rename: `LeaveGroupLevel` → `LeaveLevel`.** 023's name is now a misnomer at three of the
five rungs — a subpath is not a group, a node is not a group, and on the operator's files
there are no groups at all. The name never shipped (23.2 is unbuilt), so the rename costs
one identifier and prevents a name that lies in the common case. R93's shape, caught before
it goes false rather than after.

**★ The concrete change the flag asked for, stated exactly.** `main.rs:4878-4886` today:

```rust
Action::ClearCanvasSelection => {
    doc.canvas_selection.clear();
    // Escape also LEAVES an entered object. …
    doc.entered = None;          // ← THIS LINE IS DELETED
}
```

The comment defending it is honest and, for a two-rung world, was right: *"an operator who
has descended into a drawing view and pressed Escape means to be out of it — not to be left
inside with nothing selected, which looks identical to being outside and behaves
differently."* The observation it rests on — **an inside-with-nothing-selected state that
looks identical to outside is a real hazard** — survives and is discharged properly by the
ladder: the rung is *always* named in the readout and the breadcrumb (§4), so the two
states never look identical again. With that disclosure in place, collapsing rungs on one
press is no longer protection; it is a skip, and L1 forbids it.

So: `ClearCanvasSelection` clears the selection and nothing else. `LeaveLevel` pops one
rung and touches the selection not at all. Two actions, two meanings, no overlap.

`prune_canvas_selection`'s unconditional reset (`main.rs:2208`) is a **different thing and
stays**: an edit or page change *invalidates* the path, it does not *ascend* it. Pass 26.2
changes that line, and only by adding re-validation, never by removing the reset.

### 3.6 The cost of putting `LeaveLevel` above `ExitTool`, named

With the ladder four rungs deep (five with containers), an operator standing at a Node with
the Obj tool armed inside two containers needs **eight** Escape presses to reach a cleared
rail: node → subpath → object → container₂ → container₁ → Page → exit tool → clear
selection. 023 argued for this ordering on the ground that *"depth is narrower than
tool-activation"* and that the chain's meaning is *"undo the most recent narrowing of
context, most-recent first."* That reasoning is right and this record keeps it.

But the cost is real and should not be discovered by the operator. **It is answered without
a new gesture:** 023 §3.5 already requires the breadcrumb to be *clickable to ascend*.
Clicking `Page` in `Page › Path #5870 › Part #667 › Point #1204` is the one-click jump to
any rung. The keyboard equivalent (Shift+Escape, or similar, to leave all rungs at once) is
**not built and not offered** — R83 — and is filed as operator question **(al)** rather
than invented here.

---

## 4. Q4 — The disclosure obligation: the unified readout matrix

### 4.1 What 023 requires and what shipped

023 §1.3 requires three things: the readout **names the level and the container kind, in
positive terms**; a double-click with nothing to descend into **reports the fact, never a
silent no-op**; and a **headless answer exists before any GUI work**.

Pass 25.1 ships (`ui_text.rs:2013-2032`, rendered at `main.rs:11209`):

> *"Inside object #5870, which is drawn as 1194 separate part(s) — part #667 is selected.
> Click another part to pick it, or press Escape to go back to whole objects."*

That is good work and the count is the single most useful number on the screen — it is
what explains why clicking a line selected an entire view. Three things need fixing:

1. It knows nothing about containers, nodes, or the Page rung.
2. **It is assembled from fragments.** `let scope = format!(…)` then
   `format!("Inside {scope} — part #{sp} is selected. …")`. R2 forbids sentence assembly
   precisely because fragment order is not translatable — no language guarantees that
   "inside X" and "part N is selected" compose in that order, and `part(s)` is an English
   pluralisation convention baked into the string. With a ten-row matrix this is the moment
   to fix it: **one complete catalog string per row** (R1's single catalog, R2's no
   assembly).
3. It leaves the flattened-CAD Page rung unaddressed — which is where 023's own specified
   string is wrong (§9 item 1).

### 4.2 The matrix

Each row is **one complete string in `ui_text`**, not a fragment. Wording is illustrative
— `pdfce-ui-specialist` owns final copy (§11) — but the *content* of each row is
specified: it must name the rung, name what is below, and where nothing is below, say so
in positive terms.

| Rung | Situation | Readout must convey |
|---|---|---|
| **Page** | page has **no containers** (the operator's CAD files) | *"Top level. This page has no groups — objects are the outermost level. Double-click an object to work on its parts."* — **positive**, and it **still offers the descent**, which is what 023's own string failed to do (§9 item 1). |
| **Page** | page has containers | *"Top level. 3 groups on this page. Double-click one to work inside it."* |
| **Page** | an object is selected, no containers | *"Path selected (object #11), drawn as 1 part. This object is not inside a group. Double-click it to work on its parts."* |
| **Page** | a **ce dimension** is selected | *"Measurement selected (5.000 m, group 2). A measurement is one object — it has no parts. Use Re-measure to change it."* (L3; 023 §5.2) |
| **Container** | inside a **form XObject**, invoked **once** | *"Inside group “Fig1” (a form XObject) — 47 objects. This group is placed once, so edits here affect only this placement. Double-click an object to work inside it."* — 023 §1.4's disclosure on the common case, so the rule is learned before the refusal. |
| **Container** | inside a form XObject, invoked **> 1** time | *"Inside group “TitleBlock” (a form XObject) — 12 objects. **This group is placed 12 times in this document; editing inside it would change all 12.** Editing is not available here."* — the measured count, stated **before** any verb is attempted (023 §1.4's `FormStreamIsShared`). |
| **Container** | inside a **marked-content sequence** | *"Inside group /Span (a marked-content section) — 12 objects. …"* |
| **Object** | path drawn as **N > 1** parts | *"Inside object #5870 — it is drawn as 1,194 parts. Part #667 is selected. Double-click a part to work on its points."* (plus the cycle disclosure when several parts are under the pointer, §4.3) |
| **Object** | path drawn as **exactly 1** part | *"Inside object #11 — it is drawn as a single part, which is already selected. Double-click it to work on its points."* — ★ **the single-subpath rung is entered and disclosed, not skipped** (§1.3(d)). The operator is told the rung is degenerate rather than silently teleported past it. |
| **Object** | **text / image / form** object (no subpaths) | *"Object #40 is text — it has no parts and no points. Press Escape to go back."* — `NothingToEnter`, reported. Descent stops here for these kinds (decision 011 §2.1: text/image are not node-editable). |
| **Subpath** | anchors **under** the ceiling | *"Inside part #667 of object #5870 — 6 points. Click a point to select it."* |
| **Subpath** | anchors **over** the ceiling | *"Inside part #3 — 41,208 points (limit 2,000). Points are not shown; this part is too complex to edit point by point."* — 023 §4.6: show **no** handles and state the count and the limit. **Never a silent first-N**, because the operator would conclude the rest of the geometry has no points. |
| **Node** | a point is selected | *"Point #1,204 of object #5870 (part #667) selected. This is the deepest level."* — the index is object-scoped and the string says which object it counts within, so the number matches `node-move --node`. |
| **Node** | double-click here | *"This is the deepest level — there is nothing below a point."* (`AlreadyDeepest`) |

### 4.3 The flattened-CAD case, called out because it is the operator's case and 023 gets it wrong

On a page with **no container but a real subpath rung**, the readout must do two things at
once that 023's specified string cannot:

- **say there is no group, in positive terms** — 023 §1.3 is right about this, and
- **still offer the descent**, because there *is* one.

023's specified string is `Path · … — this object is not inside a group (already at object
level)`. The parenthesis is **false on the operator's files**: object level is not where
the ladder ends; there are 1,194 selectable parts and 6,681 points below it. Ship that
string and the tool tells the operator the thing they were most confused about is
impossible. §9 item 1; the corrected row is the third Page row above.

### 4.4 The cycle disclosure the object rung already has

When several parts are under the pointer, the Subpath rung's readout carries the object
rung's existing form — *"part #667 (2 of 5 at this point)"* — with `ClickCycle`'s four
staleness resets applying unchanged (`canvas.rs:788-800`). §1.4.2; acceptance criterion F6.

### 4.5 The breadcrumb, extended

023 §3.5's example, `Page › Form "Fig1" › Marked /Span › Path`, stops one rung short of
023's own node level and two short of this ladder. Under the unified model:

- container-free (the operator's files): `Page › Path #5870 › Part #667 › Point #1,204`
- with containers: `Page › Fig1 › /Span › Path #12 › Part #3 › Point #7`

Every crumb is clickable and ascends to that rung (023 §3.5), which is what makes §3.6's
eight-press Escape walk a one-click jump when the operator wants one. R84 is satisfied by
construction — a breadcrumb is text plus separators, so the rung is never carried by colour
alone.

### 4.6 The ceiling, revisited — 023's arithmetic is superseded by the ladder

023 §4.6 reasoned from *"show nodes for a whole group"*: 41,208 handles, an unclickable mat
at 6 pt grab tolerance, a ceiling around 2,000, and a strong warning against silent
truncation.

Under the ladder the Node rung shows the anchors of **one entered part**. On the measured
page: 6,681 anchors ÷ 1,194 subpaths ≈ **6 points per part**. The ceiling is still needed —
a hatch boundary or a flattened spline can be one very long subpath — but it stops being
the common case, and the disclosure text moves from "select fewer objects" (which is not
the operator's situation at that rung) to "this part is too complex to edit point by
point." The ceiling **value** remains a `pdfce-ui-specialist` + R86 judgment (§11); only
looking proves where handles stop being distinguishable.

### 4.7 The headless half of the obligation

023 §1.3 disclosure 3 requires the answer to exist before any GUI work. Pass 25.0 already
shipped half of it (`object-list --enter INDEX --hit X,Y`). The rest:

```
pdfce-cli object-list <pdf> --page N --tree                    # 023 §7.2: the container forest
pdfce-cli object-list <pdf> --page N --enter INDEX --subpaths  # NEW: every part, bbox, anchor count,
                                                               #      and its OBJECT-SCOPED anchor index range
pdfce-cli object-list <pdf> --page N --hit X,Y --level N       # 023 C3: what a click at rung N selects
```

`--subpaths` is the load-bearing new one, and it is what keeps `node-move --node N`
unchanged: it prints, per part, the **object-scoped** anchor index range belonging to that
part, so a script (and a test) can go from "part #667" to "points #3,204–#3,209" without a
second index space existing anywhere (§1.3(b)).

---

## 5. Q5 — Edit verbs at the subpath rung

Pass 25.1 deliberately shipped selection only, and its tooltip says so in words rather than
showing a disabled control — R83 honoured in the UI text itself, which is R83's own
preferred shape:

> *"Moving and deleting individual parts is not available yet — only selecting them."*

This section specifies what those verbs mean.

### 5.1 What the two verbs are

- **Move a part** — translate that subpath's construction operands by a page-space delta;
  every other subpath of the same object, and every other object, byte-verbatim.
- **Delete a part** — excise that subpath's construction operators; the object's **painting
  operator and every other subpath stay byte-verbatim**.

Both are content-stream surgery inside one object, so both are **R46's named exception**
(the one class of object pdfce re-emits rather than copying verbatim), the same family as
`plan_delete`/`plan_move`/`plan_move_node` from Pass 9c-min.

### 5.2 ★ The blocker nobody has stated: `Subpath` has no span

```rust
pub struct PathObject {          // decompose.rs:312-337
    pub subpaths: Vec<Subpath>,
    …
    pub tokens: TokenRange,      // ← the editing handle
    pub bytes: ByteSpan,         // ← the byte span plan_delete splices
}

pub struct Subpath {             // decompose.rs:224-232
    pub start: Point,
    pub segments: Vec<Segment>,
    pub closed: bool,
}                                // ← no tokens. no bytes. nothing.
```

Every shipped planner works from `PathObject::tokens` / `::bytes`:
`plan_move` iterates `ops_in_range(content, obj.tokens.start, obj.tokens.end)`;
`plan_delete` splices `obj.bytes()`. **There is no way to name a subpath's bytes**, so
neither verb is expressible against today's model. Not difficult — *not expressible*. This
is the largest unstated cost in the `ROADMAP.md` Next-up entry and it should be the first
line of the Pass that builds the verbs.

**Two ways to close it:**

| Option | Cost | Risk |
|---|---|---|
| **(i) Record the span at decomposition time** — `Subpath` gains `tokens: TokenRange` + `bytes: ByteSpan`, mirroring `PathObject` | two index pairs per subpath; on the measured page ~6,700 subpaths across four objects, ~100 KB — trivial. `Subpath::transformed` carries them through unchanged (a page-space transform does not change bytes). Small blast radius: tests that construct `Subpath` literals, and `PartialEq` now compares spans. | none structural |
| **(ii) Re-walk the object's operator range at plan time** and derive boundaries from the ordinal | no model change | **a second traversal that must agree with `decompose`'s.** Decision 011's Z2 defect class and R111's shape: two derivations of the same structure that must agree and eventually will not, silently, at the exact moment `hit_test_subpaths` says "part #667" and the planner counts a different #667. |

**Decision: (i).** The reason is not convenience. `hit_test_subpaths` already returns an
**ordinal**, and an ordinal is only meaningful if exactly one traversal defines it.
Recording the span where the ordinal is minted makes agreement structural rather than
maintained. This is 023 §2.6's principle (*"reuse, don't re-derive"* — the renderer's cycle
guard and depth cap) applied one object space down.

### 5.3 Delete semantics, per case

The object's subpaths share **one painting operator**. So a delete removes construction
operators only, and the object survives with one fewer part.

| Case | Result |
|---|---|
| An interior subpath, opened by `m` | its `m` … through the operator before the next subpath's opener is spliced out |
| A subpath that is a bare `re` | the single `re` operator is spliced out |
| The **first** subpath | same — the following subpath's `m` becomes the first construction operator |
| A subpath followed by `h` | the `h` goes with it (it closes *that* subpath) |
| A **closed** subpath whose follower is `h`-reopened | ★ **REFUSED** — `DeleteWouldMoveNextSubpath`, §5.5 |
| The object's **last remaining** subpath | ★ **REFUSED** — `LastSubpathOfObject`, §5.4 |
| A subpath inside a form XObject invoked > 1 time | **REFUSED** — `FormStreamIsShared` (023 §1.4), inherited unchanged |

Clipping operators (`W`/`W*`) sit between the last construction operator and the painting
operator (§8.5.4), so they are never inside a subpath's span and are never disturbed.

### 5.4 Why deleting the last part is a refusal and not a delegation

Deleting an object's only remaining subpath leaves a painting operator (`S`, `f`, `B`, …)
with **no current path** — §8.5.3's operand-less case, a no-op in permissive consumers and
an error in strict ones. Three ways to handle it:

- silently also remove the painting operator → the object vanishes, and **"delete a part"
  silently became "delete the object,"** which is rule 4's exact prohibition;
- leave the bare operator → a malformed-ish stream and a zero-part object still occupying a
  paint-order index, so every subsequent `--object N` shifts around a ghost;
- **refuse by name** and say what the operator wants instead.

**Decision: refuse.** `LastSubpathOfObject { object, subpaths }`. The operator's route to
removing the whole thing already exists and is unambiguous: `delete_object`. This is
structurally identical to 023 §4.4's `SubpathWouldDegenerate` one rung down (*"There is no
right answer… the operator's route to removing the whole thing already exists"*), and it is
trivially reachable — a two-part object is common on any drawing, so R96's firing test
writes itself.

**For a multi-part delete the check is on the SET, not per item:** deleting parts
{0, 1, 2} of a 3-part object fires once, on the set. Per-item checking would pass each
individually and empty the object.

### 5.5 ★ `DeleteWouldMoveNextSubpath` — the byte-perfect edit that moves geometry you did not select

This is the sharpest finding in the record and it is 023 §1.4's form-aliasing trap one
object space down.

**The mechanism, verified in the decomposer:**

```rust
fn close_subpath(&mut self) {          // decompose.rs:1859-1868
    …
    pa.current = pa.subpath_start;     // current point ← the CLOSED subpath's start
    pa.needs_move = true;
}

fn current_for_segment(&mut self, first: usize) -> Option<Point> {   // :1803-1824
    let cur = self.path.as_ref().and_then(|p| p.current);
    …
    if pa.needs_move {
        pa.open = Some(Subpath { start: cur, segments: Vec::new(), closed: false });
        //                       ^^^^^^^^^^ inherited. NO OPERAND OF ITS OWN.
    }
}
```

So after `h`, a following `l`/`c`/`v`/`y` opens a new subpath whose **start point is
inherited from the preceding closed subpath's start** (§8.5.2.1 — this is the same
construct 023 §4.4 named `ImplicitNode` for the *node* rung). Excise the preceding subpath
and the follower's start changes to whatever the current point then is.

**Why this is worse than an ordinary bug.** The resulting edit is byte-minimal. It passes
`--verify-undo`. It passes `tools/content-identity` for every object except the one
deliberately edited. Every round-trip and minimal-diff check pdfce has says it is correct
— and a line the operator never selected has moved. **Minimal-diff discipline does not
protect against it; only the named refusal does.** That sentence is a direct echo of 023
§11's verdict on form aliasing, and the echo is the point: this is the same failure shape,
found at a new rung, and it must be handled the same way — **measured, refused by name,
tested firing.**

`DeleteWouldMoveNextSubpath { deleted, affected }`. Reachable from any `… h … l …`
sequence, which producers emit routinely.

**In a multi-part delete, evaluate against the SURVIVING set:** deleting both the closer
and its reopened follower is fine (nothing is left to inherit a changed start), so the
check runs after set subtraction, not per item. That detail is not obvious and is owed a
test of its own.

**Operator question (ap):** offer *"materialize the follower's start as an explicit `m`,
then delete"* as a reviewable fix-up? It is well-defined and it is the exact analogue of
023 §10 item 3's form un-sharing question, with the same trade — it writes bytes the
operator did not ask for in order to enable the edit they did ask for. Refuse by name
first; offer the fix-up only if the operator wants it.

### 5.6 The refusal table, whole (R96: each reachable, each owed a firing test)

| Refusal | New? | Fires when | Verb |
|---|---|---|---|
| `LastSubpathOfObject { object, subpaths }` | **new** | the delete (or delete set) would leave the object with zero subpaths | delete |
| `DeleteWouldMoveNextSubpath { deleted, affected }` | **new** | the surviving subpath immediately after the deleted one is `h`-reopened, so its start is inherited | delete |
| `ImplicitSubpathStart { subpath }` | **new** | the subpath being **moved** is itself `h`-reopened — its start carries no operand, so translating its explicit operands alone would tear it | move |
| `MalformedOperand` | existing | a construction operator with spec-violating arity (Table 59) | move |
| `DegenerateCtm` | existing | singular CTM — a page-space delta has no user-space pre-image | move **only** |
| `NotAPath { index, kind }` | existing | text / image / form object | both |
| `ObjectOutOfRange` / new `SubpathOutOfRange { index, count }` | existing / new | index past the end | both |
| `FormStreamIsShared { form, invocations }` | 023 §1.4 | the subpath lives in a form invoked more than once | both |

**`DegenerateCtm` must NOT be raised for subpath delete.** A delete needs no coordinate
transform, therefore no CTM inverse, therefore no such failure exists. This is 023 §4.4's
argument for node delete, verbatim, and it holds for the same reason: raising a refusal that
cannot honestly be reached is dishonest in the opposite direction from a missing one, and no
test could reach it without lying.

### 5.7 The API surface

**Core planners** (`pdfce-core::vector::edit`), alongside `plan_move` / `plan_delete` /
`plan_move_node`:

```rust
/// Translate ONE subpath's construction operands by a page-space delta.
/// Every other subpath of `obj`, and every other object, stays byte-verbatim.
pub fn plan_move_subpaths(
    content: &ContentStream, obj: &PathObject, subpaths: &[usize],
    dx_page: f64, dy_page: f64,
) -> Result<PlannedEdit, VectorEditError>;

/// Excise the named subpaths' construction operators. The object's PAINTING
/// operator and all surviving subpaths stay byte-verbatim.
pub fn plan_delete_subpaths(
    content: &ContentStream, obj: &PathObject, subpaths: &[usize],
) -> Result<PlannedEdit, VectorEditError>;
```

**Plural from the outset, deliberately.** 023 §4.2 established the rule for nodes and it
applies identically here: N sequential single calls would be N undo steps and N
content-stream re-emissions, and would violate 022 §5.5's R107-shape (name the changed
objects, prove the rest byte-verbatim). Selecting a hatch's twelve parts and pressing
Delete is one operation to the operator; it must be one command. A singular convenience
wrapper may exist, but the plural is the primitive.

**`EditSession` methods** (`pdfce-core::edit`), both through the existing `vector_surgery`
skeleton (`edit.rs:2189`) so they inherit for free: the encryption guard, the DocMDP
certification guard, base-decompose indexing (so `subpath_index` lines up with whoever
decomposed the base), `VectorEditNeedsReopen`, and undoable staging.

```rust
pub fn move_subpaths(&mut self, page_index: usize, object_index: usize,
                     subpaths: &[usize], dx: f64, dy: f64) -> Result<(), EditError>;
pub fn delete_subpaths(&mut self, page_index: usize, object_index: usize,
                       subpaths: &[usize]) -> Result<(), EditError>;
```

New `CommandKind::MoveSubpaths` / `CommandKind::DeleteSubpaths`.

**CLI (rule 11 — same Pass, not a follow-up):**

```
pdfce-cli subpath-move   <pdf> --page N --object N --subpath N[,N,…] --dx D --dy D
                               -o out.pdf [--verify-undo]
pdfce-cli subpath-delete <pdf> --page N --object N --subpath N[,N,…]
                               -o out.pdf [--verify-undo]
pdfce-cli object-list    <pdf> --page N --enter N --subpaths      # the oracle (§4.7)
```

`node-move --node N` is **unchanged** — object-scoped indices, §1.3(b). That non-change is
an acceptance criterion (§7, G6), not an omission.

### 5.8 ★ Round-trip, minimal-diff, and the R58/R35/R67 family — plus the pdf-dimension hazard

**Minimal-diff (rule 3, `ARCHITECTURE.md` §5).** Both verbs re-emit exactly one content
stream, spliced at the subpath's byte span. Every other object is byte-verbatim. The
distinguishing claim is machine-checkable and belongs in the acceptance criteria: a
**byte-span assertion** that only the named subpath's operators changed — not a code
review. 023 §11 names the specific risk for node delete (*"respliced-too-much"*) and it is
the same risk here, one granularity up.

**Save mode.** Both verbs are **removals/translations whose contract is NOT
confidentiality**, so they stay under the default incremental save. This is the same
posture `EditSession::delete_object` (Pass 9c-min) and `delete_redaction_mark` (Pass 8)
already have.

**And that makes subpath delete the FOURTH instance of R58's staleness.** R58's literal
text — *"every removal/scrub operation rides R35's forced FULL REWRITE"* — is already
contradicted by `delete_object` and `delete_redaction_mark`, with decision 022's proposed
`delete_annotation` (Pass 22.0) as a third. Open operator question **(v)** proposes
narrowing the wording to *"every operation whose contract is CONFIDENTIALITY (redaction,
scrub, recovered-base save)."*

**This record does not decide (v)** — it is the operator's. What it does is add the fourth
data point and state the consequence plainly: **under R58's literal text as written today,
`delete_subpaths` would be required to force a full rewrite, which would be wrong.**
Deleting one line of a drawing is not a confidentiality operation; the prior revision
remaining reachable through undo and version history is that working as intended, exactly
as §5.11/R70 already established for in-place text editing. Recorded as question **(ao)**
so the fourth instance is visible when (v) is answered rather than discovered afterwards.

**R35** is untouched — confidentiality operations only. **R67** applies unchanged: a
cross-reference-recovered document forces full rewrite regardless of the verb.

**★ The pdf-dimension hazard (rule 15, and the reason this subsection is not routine).** On
the operator's own files the subpaths inside a drawing-view object include the geometry of
its **pdf dimensions** — witness lines, extension lines, arrowheads, leaders — drawn as
ordinary path subpaths in the same object as everything else, with the numeral drawn as
text in a *different* object entirely. So:

> **Deleting one part can silently destroy half of a pdf dimension.** Delete a witness line
> and the `55 5/8"` numeral is still printed, still correct-looking, and no longer points
> at anything. Nothing in the file marks those subpaths as belonging together — there is no
> container (the whole premise of §1.1), no marked content, and no annotation. pdfce
> **cannot** detect this, and must not pretend to: inferring "these three subpaths and that
> text are one dimension" from geometry is precisely the spatial-heuristic grouping 023
> §1.2 refused, and inferring it in order to *block an edit* would be worse than inferring
> it to select — a silent refusal based on a guess.

The honest response is **disclosure at the verb, not detection**: the subpath-delete
confirmation states that parts are geometry only and that pdfce cannot tell whether a part
belongs to a printed dimension. That is a one-line string and it is the difference between
a tool that surprised the operator and one that told them. Acceptance criterion G7.

This is also the clearest illustration of why rule 15 is a rule: a **ce dimension** cannot
be damaged this way (it is one annotation with a baked `/AP`, refused descent by L3), and a
**pdf dimension** is defenceless. Same word, opposite properties, opposite handling.

### 5.9 What subpath verbs do to the operator's standing position

A subpath edit rewrites the object's content stream, so `refresh_pages` →
`prune_canvas_selection` fires and today resets the whole path. The operator is ejected from
three rungs deep to Page, on a page with ~5,900 objects. §1.4.3, §6, Pass 26.2.

---

## 6. Level survival across an edit — Pass 26.2

**The problem.** `prune_canvas_selection`'s unconditional reset is correct as written and
its stated reason is exactly right: after a content rewrite the same index can name a
different object, and a selection that silently changed what it refers to is worse than no
selection. But once subpath verbs exist, that correct behaviour makes every edit eject the
operator from the position they are working in.

**The solution is not to weaken the reset. It is to re-derive and re-validate.**

```rust
/// The nearest ancestor rung that STILL RESOLVES against a fresh
/// decomposition. Rungs that no longer resolve are dropped, and the caller
/// discloses which — never silently kept, never silently reset to Page.
pub fn truncated_to_valid(&self, model: &PageObjects) -> (LevelPath, Option<Rung>);
```

Rules:

- The **object** rung survives if that paint-order index still resolves to an object of the
  same kind *and* the edit was to that object (a same-object edit does not renumber the
  page's object list — `plan_move_subpaths` and `plan_delete_subpaths` splice within one
  object's span, so `objects.len()` and every other index are unchanged; a byte-span
  assertion proves it).
- The **subpath** rung survives a *move* (ordinals unchanged) and survives a *delete* only
  for parts whose ordinal is below the lowest deleted ordinal. Above it, everything
  renumbers — so the honest answer is to drop to the Object rung and say so.
- The **node** rung survives only when its subpath does and the anchor count is unchanged.
- Anything else → drop to the nearest surviving ancestor, **disclosed**:
  *"That part no longer exists — you are back at object #5870."*

**Why this is its own Pass and not a line in 26.1.** It cannot be tested until the verbs
exist; it has its own distinct failure mode (a path that survives when it should not is
precisely the *"selection that silently changed what it refers to"* the current code exists
to prevent); and getting it wrong is worse than not having it. Splitting it means 26.1 ships
with the honest, hostile behaviour (ejected to Page, disclosed) rather than a clever wrong
one.

**Operator question (am):** is being dropped to Page after each part-edit acceptable as an
interim, or should 26.2 be folded into 26.1 and shipped together?

---

## 7. Pass plan

**Family 26** — families up to 25 are headed, 24 is claimed by decision 024, and 22/23 are
claimed by decisions 022/023 (see the ceiling table in the header). Re-run
`tools/check-ledger-numbers.py --stats` **against the branch, not a stale worktree**, at
filing time.

### 7.1 Does Pass 23.2 as scoped in 023 §7.3 still stand?

**It splits.** Precisely:

| 023 §7.3 component | Status |
|---|---|
| Core containers — marked-content ranges + form sub-models, reusing `pdfce-render`'s cycle set and depth cap; `DecomposeDiagnostics` counters | **STANDS UNCHANGED.** Nothing here is affected by the subpath rung. |
| CLI `object-list --tree` / `--hit x,y --level N` | **STANDS**, with `--level` now indexing the four-rung ladder, and joined by `--enter N --subpaths` (§4.7). |
| Provider payload → `ContentPath` (022 §2.5, R112) | **STANDS UNCHANGED.** |
| Double-click descend; Escape ascend; `EscapeContext`; breadcrumb | **SUPERSEDED by Pass 26.0.** These are the ladder mechanics, and building them for containers alone would build the second mechanism this record exists to prevent. |
| Acceptance criterion **C6** (*"the readout says 'already at object level'"*) | **WRONG — must be amended.** §9 item 1. |
| Acceptance criteria C1, C2, C4, C7, C8 | **STAND UNCHANGED.** |
| Acceptance criterion **C5** (*"Escape walks out one step per press across all six slots"*) | **STANDS, WIDENED** to the four-rung ladder — the property is unchanged, the state space is larger. |

**Net: 23.2 stands as a core+CLI Pass and loses its GUI half to 26.0. It is therefore
correctly ordered AFTER 26.0**, which inverts the `ROADMAP.md` Next-up assumption that 23.2
comes first — and the roadmap's own flag already anticipates this, noting that the measured
CAD evidence says *"container descent has nothing to descend into"* on the operator's files.
**23.2 is no longer on the critical path for the operator's own use case at all.** It
remains genuinely wanted for *other* files (it is the only thing that closes R111's
form-XObject violation), which is unchanged and still open as question (t).

### 7.2 Pass 26.0 — The unified ladder: `LevelPath`, the node rung, the fixed Escape, the readout matrix

**GUI-only. Read-only — writes zero content bytes.** Containers are out of scope (23.2
adds them by extending `LevelPath::containers`, which is why that field exists from day
one). Ships on the operator's own files, and it is the Pass that makes 25.1's tooltip stop
saying *"not available yet"* about half of what it says.

- `LevelPath` / `Rung` / `DescendOutcome`; `EnteredObject` deleted; `depth_after_click` →
  `next_level_after_click`, keeping purity and porting all 7 tests
- **Node rung**: anchors of the entered part shown and clickable; single-node move reuses
  the shipped `plan_move_node` drag (`vector_edit_tool.rs`) with **no core change**
- `EscapeContext` (R120) + `LeaveLevel` at slot 2; **delete `main.rs:4885`**
- `ClickCycle` generalised to the subpath rung (§1.4.2)
- the §4.2 readout matrix as one complete catalog string per row (R1/R2); breadcrumb
- CLI: `object-list --enter N --subpaths`

| # | Acceptance criterion (one line) |
|---|---|
| F1 | On the SolidWorks fixture, three double-clicks from Page reach a point of part #667 of object #5870, and the breadcrumb names all four rungs |
| F2 | Escape pops exactly one rung per press — a unit test over `EscapeContext` walking node→subpath→object→Page→ExitTool→ClearCanvasSelection→rail, asserting **no press skips a rung** |
| F3 | `Action::ClearCanvasSelection` no longer changes the rung — a test that clears a selection while inside a part and asserts the rung is unchanged |
| F4 | A single-part object is **entered and disclosed**, not skipped: the readout says "drawn as a single part" and the rung index is `Object`+1 |
| F5 | `NothingToEnter` fires and is reported for a text object and for a ce dimension (L3) — never a silent no-op |
| F6 | With three parts under the pointer, repeated clicks cycle through all three and the readout reads "2 of 3 at this point" |
| F7 | `object-list --enter 5870 --subpaths` prints 1,194 rows whose object-scoped anchor ranges sum to the object's `anchor_count`; `node-move --node N` is unchanged (byte-identical output on a golden fixture) |
| F8 | `cargo tree -p pdfce-core -p pdfce-render` GUI-free; **zero content bytes written** by this Pass |
| F9 | **R86 — must be watched**: does the breadcrumb read as navigation; is three-double-clicks-to-a-point tolerable in practice |

### 7.3 Pass 26.1 — Subpath edit verbs: spans, move, delete, three new refusals, CLI

Depends on 26.0 (the rung must exist before it has verbs). Core + CLI + GUI.

- `Subpath` gains `tokens` + `bytes` (§5.2 option (i)); `Subpath::transformed` carries them
- `plan_move_subpaths` / `plan_delete_subpaths` (plural primitives)
- `EditSession::move_subpaths` / `delete_subpaths` via `vector_surgery`
- CLI `subpath-move` / `subpath-delete` with `--verify-undo`
- GUI verbs at the Subpath rung; the 25.1 tooltip's "not available yet" clause **removed in
  the same commit** (R93 — a statement that has gone false)

| # | Acceptance criterion (one line) |
|---|---|
| G1 | Only the named subpath's construction operators change — a **byte-span assertion**, not a code review |
| G2 | **Every refusal fires** from an input that would otherwise succeed (R96): `LastSubpathOfObject` on a 1-part object, `DeleteWouldMoveNextSubpath` on `… h … l …`, `ImplicitSubpathStart` on a moved `h`-reopened part, `FormStreamIsShared` on a twice-placed form |
| G3 | `DegenerateCtm` is **NOT** raised for delete — a singular-CTM fixture deletes a part successfully |
| G4 | A 12-part delete is **one** command — undo-stack depth 1, one content-stream re-emission |
| G5 | `DeleteWouldMoveNextSubpath` is evaluated against the **surviving** set — deleting both closer and reopened follower **succeeds** |
| G6 | `node-move --node N` and `object-move --object N` produce byte-identical output before and after this Pass on a golden fixture |
| G7 | The delete confirmation discloses that a part is geometry only and that pdfce cannot tell whether it belongs to a printed **pdf dimension** (§5.8) |
| G8 | `--verify-undo` byte-identical on both verbs; save stays **incremental** (not a forced full rewrite) — flagged against R58/question (v), §5.8 |

### 7.4 Pass 26.2 — Level survival across an edit

Depends on 26.1.

- `LevelPath::truncated_to_valid`; `prune_canvas_selection` re-validates instead of resetting
- disclosure when a rung is dropped

| # | Acceptance criterion (one line) |
|---|---|
| H1 | Moving part #667 leaves the operator standing at part #667 |
| H2 | Deleting part #10 of 1,194 leaves the operator at the **Object** rung with a disclosure, never silently on a renumbered part |
| H3 | Deleting part #1,100 leaves a standing position at part #10 intact (below the lowest deleted ordinal) |
| H4 | A page change or an edit to a **different** object still resets to Page (the shipped behaviour is not weakened) |
| H5 | **R86 — must be watched**: does re-validation feel like the tool kept your place, or like it moved you |

### 7.5 Pass 23.2 (amended) — containers join the ladder

Scope as 023 §7.3's core+CLI half; GUI half is 26.0's. C6 amended per §9 item 1.

| # | Acceptance criterion (one line) |
|---|---|
| C1′ | `object-list --tree` prints a form's 3 children where today it prints 1 opaque object (023 C1, unchanged) |
| C2′ | The R111 paint/select asymmetry closes for form XObjects, asserted from **both** the renderer's recursion and the decomposition, one depth cap (023 C2, unchanged) |
| C6′ | **AMENDED** — a container-free fixture prints `containers=0` and the readout says *"this page has no groups — objects are the outermost level"* **and still offers the descent to parts** (§4.3) |
| C10 | Entering a container extends `LevelPath::containers` and the ladder's L1/L2 rules hold unchanged at container rungs — one double-click, one rung |

---

## 8. Sequencing summary

```
26.0  ladder + node rung + Escape fix + readout      ← ships on the operator's OWN files
26.1  subpath verbs (needs 26.0)                     ← removes 25.1's "not available yet"
26.2  level survival (needs 26.1)
23.2  containers, core+CLI (independent; GUI half now 26.0's)   ← for OTHER files
23.3  node selection set / multi-node move / node delete (023 Q4; needs 26.0's node rung)
```

**23.3's dependency changes and improves.** 023 §7.4 says it depends on 23.2 (*"level 3 is
the level below level 2"*). Under the ladder its real dependency is **26.0** (the node rung),
not containers. So 23.3 no longer waits on 23.2 — which is a genuine unblocking, since 23.2
is the larger, container-shaped Pass with no payoff on the operator's own files.

**22.0 first, still.** 023 §11's *"22.0 ships first, or 23.1/23.2 do not start"* extends to
26.x: `TargetId`'s enum widening and 022 §5.3's guarded-write fix are prerequisites for
anything that touches the selection substrate.

---

## 9. ★ What decision 023 now gets factually wrong

Not "incomplete" — wrong, in the sense that a reader acting on the statement would build the
wrong thing. Five items. **023 is not edited** (append-only, `docs/decisions/README.md`);
these corrections live here, and the librarian owes a forward-reference from 023's
`ARCHITECTURE.md` §12 ledger entry.

| # | 023 says | Correction |
|---|---|---|
| **1** | §1.3, the specified readout string: `Path · … — this object is not inside a group (already at object level)`; and §7.3's acceptance criterion **C6** requires that string on a container-free fixture | **WRONG on the operator's own files.** Object level is not where the ladder ends there: object 5870 has 1,194 selectable parts and 6,681 points below it. The parenthesis tells the operator the thing they were most confused about is impossible. C6 is an acceptance criterion that would certify a false statement. Corrected string: §4.2's third Page row; corrected criterion: §7.5 C6′. |
| **2** | §0: *"'Three levels' is not a property of PDF … A flattened CAD plot has **two** (objects, nodes) — there is nothing to descend into."* | **WRONG.** A flattened CAD plot has **three** below the page — object, subpath, node — and on the measured file the subpath rung is the only one that makes it selectable. The first clause (depth is a property of the file, not of PDF) is right and this record adopts it; the enumeration omits a rung. |
| **3** | §1.3: *"a flattened CAD export … is very often one long list of `re` / `m l l S` with no form XObjects and no marked content"* and *"That is the file's structure, not a pdfce limitation."* | **The first half understates; the second half is wrong.** The file's structure had a level pdfce was discarding — the renderer stroked every subpath while selection saw one object. That is the R111 asymmetry 023 §0 finding 2 identified for form XObjects, present a third time and unnoticed. It **was** a pdfce limitation, and Pass 25.0/25.1 removed it. |
| **4** | §4.6's ceiling arithmetic, reasoned from *"show nodes for a whole group"* — 41,208 handles, ceiling ~2,000, an unclickable mat | **Superseded, not wrong in itself.** Under the ladder the node rung shows one part's anchors: ~6 per part on the measured page. The ceiling stays (a long hatch boundary can exceed it) but its premise — that the operator's request is nodes for a whole group — is met a different way, and the disclosure text changes from *"select fewer objects"* to *"this part is too complex to edit point by point."* §4.6. |
| **5** | §3.2's outcome name `LeaveGroupLevel` | **A misnomer at three of five rungs** — a subpath is not a group, a node is not a group, and on the operator's files there are no groups at all. Renamed `LeaveLevel` (§3.5). Naming, not fact — recorded here because the name never shipped and renaming later would be gratuitous churn. |

**Two more items that are *narrowed*, not wrong:**

- §10 item 4 / open question **(z)** (*"Fixed three navigation levels, or as deep as the
  file's own structure goes?"*). The **recommendation** — as deep as the file goes,
  terminating at nodes — stands and is adopted. The **number in the question** does not:
  it is four-plus, not three, and only the container segment is variable-depth. Worth
  restating to the operator so (z) is answered against the right ladder.
- §1.2's refusal of heuristic grouping. **Not wrong, and this record upholds it** — but its
  three grounds are now a *test* something can pass, and the subpath rung passes all three
  (§1.4.1) while carrying two smaller obligations the refusal implies and 023 did not
  anticipate: the ordering's cycle disclosure (§1.4.2) and ordinal stability under
  same-object edits (§1.4.3).

---

## 10. Proposed standing rules

*Proposed; the librarian assigns numbers. Next free is **R130** per `cce2d30`, but re-read
it at filing time against the branch.*

1. **A level ladder is walked by ONE state variable and ONE gesture pair; a second descent
   mechanism is a defect, not a feature.** One double-click goes exactly one rung down, one
   Escape exactly one rung up, neither ever skips, and the rung a gesture acts on is read
   from the single standing-position value — never inferred from what happens to be under
   the pointer. *(Sourced: decision 023 specified double-click/Escape for containers and
   Pass 25.1 shipped double-click/Escape for subpaths, producing a collision that exists
   only because two mechanisms answered the same question.)*

2. **A rung that has no members on this file is REPORTED, never silently skipped and never
   silently entered.** Skipping makes the same gesture mean different things on files the
   operator cannot tell apart, and forces the skip predicate to be duplicated in the ascent
   direction (R92). Disclosure at the rung above — *"drawn as a single part"* / *"this page
   has no groups"* — costs one string and makes the extra press a confirmation rather than a
   leap. *(Sourced: the single-subpath skip proposal, §1.3(d).)*

3. **A finer selection level FILTERS the level below it; it never renumbers it.** Adding a
   rung must not create a second index space for something already numbered, because the two
   spaces must then be kept in agreement by hand (R92, decision 011's Z2). *(Sourced: node
   indices stay object-scoped — `anchor_count`'s decomposition order — so `node-move --node
   N` keeps its meaning when the subpath rung is inserted above it, exactly as decision 023
   §2.1 kept `object-move --object N` by refusing to re-parent the flat list.)*

4. **A structural element pdfce selects must record its own byte span where its ordinal is
   minted.** An ordinal handed out by one traversal and resolved to bytes by a second is two
   derivations that must agree and eventually will not, silently. *(Sourced:
   `hit_test_subpaths` returns subpath ordinals while `Subpath` carries no `tokens`/`bytes`,
   so the verbs the roadmap owes are literally not expressible without one or the other.)*

5. **A checker that reports "the highest number written" must be run against the BRANCH, not
   a working tree that can be behind.** *(Sourced: this record's own header — the ledger
   checker run inside a worktree pinned at `5df8f26` reported ceilings that commit `cce2d30`
   had already moved, for Pass families, standing rules and open-question letters
   simultaneously. R106's stated limitation is that an unlanded draft is invisible to the
   checker; this is the second, distinct case — a landed commit invisible to the checker
   because the tree is pinned. Seventh numbering hazard on this project.)*

---

## 11. What this record does NOT decide

**Not decided here, owned elsewhere:**

- **023's container half.** §2's laminar interval model, §2.3's `Container` /
  `ContainerKind`, the one-field addition to `PageObjects` at its one construction site,
  the cycle guard and depth cap reused from `pdfce-render` (023 §2.6), and `ContentPath`
  (022 §2.5 / R112) are all **adopted unchanged**. This record adds `LevelPath::containers`
  as the place they plug in and changes nothing about them.
- **The node ceiling value.** `pdfce-ui-specialist` + R86. Only looking proves where handles
  stop being distinguishable.
- **Visual design** of the rung feedback, the breadcrumb, and the container outline
  treatment. `pdfce-ui-specialist`. This record owns only the requirement: **the current
  rung is always visible and always clickable to ascend** (023 §3.5), and R84 — never colour
  alone.
- **The operator-facing noun for a node.** 25.1's shipped strings say *"part"* for a subpath;
  `vector_edit_tool` and decision 023 say *"node"*; this record's illustrative strings say
  *"point."* **One word must be chosen and it must not be a third invented here** —
  `pdfce-ui-specialist`'s call. Code-facing names stay `subpath` / `node`.
- **R58's wording fix** (open question (v)). §5.8 adds the fourth data point and states the
  consequence; the wording is the operator's.
- **`pdfce-inkscape-librarian`'s input**, which was not dispatched for this record. R61 means
  Inkscape is a behavioural reference only, and the ladder here was derived from pdfce's own
  measured data rather than from Inkscape's group model. If the click-outside common-ancestor
  rule (023 §3.4) is to match a reference product's behaviour precisely, that dispatch should
  happen before 26.0 — flagged, not done.

**For the operator — questions this record raises, claiming (ak)–(ap):**

- **(ak) Single-part objects: enter the rung and disclose, or skip it?** Recommended: enter
  and disclose (§1.3(d)) — one extra double-click buys a rule that is always true and an
  Escape chain with no skip predicate. Cheap to change later (`descend()` gains one branch),
  which is exactly why it is worth confirming rather than discovering.
- **(al) A key to leave all rungs at once (Shift+Escape or similar)?** Not built, not
  offered (R83). The breadcrumb click already gives a one-click jump to any rung. Wanted as a
  keyboard equivalent?
- **(am) After a part-edit, is being dropped to Page acceptable as an interim** (26.1 ships,
  26.2 follows), or should 26.2 be folded in so the rung survives from the first release of
  the verbs?
- **(an) The subpath click-cycle** (§1.4.2) — recommended and scoped into 26.0, because
  without it the part *under* the nearest part is unreachable and it will read as a hit-test
  bug. Confirm it is wanted, since it makes repeated clicking mean something new at that rung.
- **(ao) `delete_subpaths` is the FOURTH non-confidentiality removal** under R58's literal
  text (§5.8), after `delete_object`, `delete_redaction_mark` and 022's proposed
  `delete_annotation`. Does it fold into open question **(v)**'s wording fix?
- **(ap) `DeleteWouldMoveNextSubpath`** (§5.5) — refuse by name (recommended), or also offer
  *"materialize the follower's start as an explicit `m`, then delete"* as a reviewable
  fix-up? Same trade as 023 §10 item 3's form un-sharing: it writes bytes you did not ask for
  to enable the edit you did.

---

## 12. Risks to the two load-bearing invariants

**GUI-core separation (rule 2, `ARCHITECTURE.md` §3).** Two live risks, in opposite
directions — 023 §11 named both and the subpath rung sharpens the second.

- *Core logic drifting into the GUI.* Container detection is content-stream parsing and
  belongs in core; the provider's job stays what Pass 25.1 made it — a read-only window
  (`subpath_hits`, `subpath_bounds_canvas`) onto core functions, so no GUI code re-derives
  geometry. `cargo tree -p pdfce-core -p pdfce-render` is the gate.
- *GUI state drifting into core — sharper now.* `LevelPath` indexes `PageObjects` at four
  levels, which makes "just store the current rung next to the objects it indexes" read as
  tidy rather than as an invariant violation. It is the latter: `PageObjects` describes the
  **document**; where the operator is standing is the **shell's**. Putting it in core makes
  the WASM fork inherit a UI mode.

**Round-trip / minimal-diff (rule 3, `ARCHITECTURE.md` §5).**

- **26.0 writes nothing at all.** The ladder, the node rung, the Escape fix and the readout
  are pure view state and pure functions. That is a real strength of splitting navigation
  from editing and it is why 26.0 is first.
- **26.1 is R46-exception surgery** at a finer granularity than any shipped verb. The named
  risk is respliced-too-much, and the answer is a byte-span assertion, not a code review
  (G1).
- **The sharp one is `DeleteWouldMoveNextSubpath`** (§5.5), and it is worth restating in the
  same words 023 used for form aliasing because the shape is identical: **a correct,
  minimal, byte-perfect edit that is still wrong.** Minimal-diff discipline does not protect
  against it; only the measured, named refusal does.
- **The second sharp one is not a code risk at all** — it is the **pdf-dimension hazard**
  (§5.8). A part-delete can leave a printed dimension pointing at nothing, and pdfce cannot
  detect it without doing exactly the spatial inference 023 §1.2 refused. Disclosure at the
  verb is the whole mitigation, and it is a one-line string that must not be dropped for
  brevity.
- **26.2 writes nothing** but decides whether a *stale* standing position survives, which is
  the same class of hazard as a stale selection — hence H4's requirement that the shipped
  reset behaviour is not weakened where it is still right.

**A third risk, to the record rather than to an invariant.** This record's Passes and
decision 023's Passes are interleaved by dependency, not by number: 26.0 before 23.2's GUI
half (which no longer exists), 26.0 before 23.3 (which no longer needs 23.2). **A reader who
schedules by Pass number will build them in the wrong order.** §8's sequencing block exists
for that reason and the librarian should carry it into `ROADMAP.md` verbatim rather than
leaving the ordering to be inferred from the IDs.

---

## 13. JSON

```json
{
  "decision_number": "025",
  "title": "The subpath rung, and the unified level ladder: reconciling decision 023's container model with what Pass 25.0/25.1 shipped",
  "date": "2026-08-04",
  "amends": "023",
  "amendment_mechanism": "new append-only record + librarian forward-reference; 023 is NOT edited (docs/decisions/README.md)",
  "pass_family": 26,
  "confidence": "high",
  "written_in_worktree": "D:\\Dev\\pdfce\\.claude\\worktrees\\agent-a32bbd2904a0cb18d",

  "headline": "The subpath is a genuine fourth rung and it goes exactly where the measurement put it, between object and node. The gesture collision dissolves the moment descent stops being two mechanisms and becomes one state variable: LevelPath. One double-click = one rung down, one Escape = one rung up, neither ever skips, and a rung with no members on this file is REPORTED rather than skipped or silently entered. Five statements in decision 023 are now factually wrong, including one acceptance criterion (C6) and one shipped-string specification that would ship a lie on the operator's own files. Two things nobody has stated: Subpath carries no token range or byte span, so neither subpath verb the roadmap owes is EXPRESSIBLE today; and DeleteWouldMoveNextSubpath is decision 023's form-aliasing trap one object space down - a byte-minimal, byte-verifiable edit that silently moves geometry the operator did not select, which minimal-diff discipline cannot catch and only a named refusal can.",

  "answers": {
    "Q1_does_the_subpath_rung_belong": {
      "decision": "YES - option (a), a fourth rung between object and node. UNCONDITIONAL: it exists for every path object including single-subpath ones, is never skipped, and the count of what is below is disclosed at the rung above so descent is never a leap in the dark.",
      "ladder": "Page -> Container(0..k, variable, file-dependent) -> Object -> Subpath -> Node. On the operator's CAD files k=0, so four rungs and three double-clicks.",
      "rejected": {
        "b_subpath_as_node_variant": "The deepest rung would mean different things on different objects, selected by the subpath count - the exact fact the operator cannot see. Structurally worse: it makes the terminal rung non-terminal, and it forces a SECOND node index space alongside anchor_count's decomposition order, which node-move --node N already addresses (R92, decision 011 Z2).",
        "c_different_gestures_for_container_vs_object": "Requires the operator to know whether the thing under the cursor is a container BEFORE choosing a gesture - and containers are invisible (a form renders as ordinary artwork, marked content renders as nothing). Inverts the design's purpose. Also gives Escape an unanswerable question about which kind of step to undo.",
        "d_conditional_rung_when_subpaths_gt_1": "Most defensible rejection. Same gesture on visually identical objects lands on different rungs, so the operator learns a rule that is false half the time. Three engineering costs 023's own reasoning forbids: the skip predicate must be duplicated in the ascent direction (R92); it destroys 023 SS3.2's testable 'never skips a step' property; and it buys only one double-click, which disclosure buys back for free since path.subpaths.len() is ALREADY read at main.rs:11205. Recorded as operator question (ak)."
      },
      "is_it_inside_023_SS1.2s_heuristic_refusal": {
        "verdict": "OUTSIDE, on all three of that refusal's own grounds",
        "in_the_file": "YES - one Subpath per m / re / h-reopen, linear parse (decompose.rs:1524-1567, rect() :1872), zero thresholds, zero tunable parameters, no clustering step. SS8.5.2 defines the boundaries; pdfce reads them.",
        "stable": "YES against UNRELATED edits - an ordinal is a position in its own object's construction order, so editing a different object cannot change it.",
        "disclosed": "YES - Pass 25.1 outlines each subpath in a deliberately non-accent amber and the readout names the count. R84 satisfied by construction.",
        "correct_framing": "This is the THIRD instance of R111's paint/select asymmetry (after ce-dimension annotations and form XObjects): the renderer strokes every subpath as a distinct visible mark, selection saw one object. It is the one the operator actually hit."
      },
      "but_two_things_ARE_smuggled_back": {
        "1_ordering_has_no_cycle": "hit_test_subpaths (hit.rs:277-318) orders by outline distance with an interior fill hit promoted to 0.0 - a metric resolving an ambiguity. The OBJECT rung has had canvas::ClickCycle and its '2 of 3 at this point' disclosure since Pass 9a; the subpath rung shipped without the twin. Structural, not a missing branch: main.rs:2176 takes .first().copied() and depth_after_click accepts a single Option<usize>. Consequence on CAD files: the line UNDER a hatch is unreachable forever and will read as a hit-test bug. DECISION: the subpath rung inherits ClickCycle, scoped into Pass 26.0 (criterion F6).",
        "2_ordinal_instability_under_same_object_edits": "Delete part #3 of 1194 and parts #4.. renumber. prune_canvas_selection's unconditional reset is correct and must not be weakened - but it means every subpath edit ejects the operator from three rungs deep to Page on a 5900-object page. Answered by Pass 26.2 (re-validate and truncate to the nearest surviving ancestor, disclosed), not by weakening the reset."
      }
    },

    "Q2_gesture_collision": {
      "resolution": "A double-click never has to decide 'container or subpath' because they are at different rungs and the current rung is always known. The collision exists ONLY if descent is implemented twice - which is R92's shape, and Pass 25.1 already refused it once by routing both of its click paths through one OpenDoc::apply_click_depth (main.rs:2141-2147). Extend that discipline one rung further; do not arbitrate between mechanisms.",
      "three_governing_rules": [
        "L1 - one double-click = exactly one rung down; one Escape = exactly one rung up; neither ever skips.",
        "L2 - a rung with no members on this file is REPORTED, never silently entered and never a silent no-op (023 SS1.3 disclosure 2; ROADMAP Pass 12.M2c bug #4's shape).",
        "L3 - a ce dimension is a level-1 leaf and descent into it is refused BY NAME (023 SS1.2, now load-bearing because the subpath rung makes 'descend into the parts of this thing' a general gesture and an /AP form stream DOES contain subpaths). Its route to a geometry change is the Re-measure verb. A pdf dimension gets no such exemption - it is page content and the ladder descends into it exactly as into any artwork."
      ],
      "escape_chain": [
        "0 DismissContextMenu (decision 024 / navigation Pass)",
        "1 CancelGesture (shipped)",
        "2 LeaveLevel (THIS record - renamed from 023's LeaveGroupLevel, which is a misnomer at three of five rungs)",
        "3 ExitTool (shipped)",
        "4 ClearCanvasSelection (shipped, CORRECTED)",
        "5 FallThroughToRailClear (shipped)"
      ],
      "the_exact_change_to_pass_25.1": "DELETE main.rs:4885 (`doc.entered = None;` inside Action::ClearCanvasSelection). That action reverts to doing exactly what its name says. Its defending comment's OBSERVATION survives - an inside-with-nothing-selected state that looks identical to outside is a real hazard - and is discharged properly by the ladder, because the rung is always named in the readout and breadcrumb so the two states never look identical again. With that disclosure in place, collapsing rungs on one press is a skip, and L1 forbids it. prune_canvas_selection's unconditional reset (main.rs:2208) is a DIFFERENT thing and STAYS: an edit INVALIDATES the path, it does not ASCEND it.",
      "click_outside": "Ascend to the nearest common ancestor of the current rung and the clicked thing (023 SS3.4). NOTE: Pass 25.1's shipped behaviour is ALREADY correct for k=0, because 'leave to Page and select the clicked object' IS the common ancestor when there are no containers. It changes only when k>0.",
      "cost_named": "With LeaveLevel above ExitTool (023 SS3.2's ordering, kept), a Node rung inside two containers is 8 Escape presses to a cleared rail. Answered WITHOUT a new gesture: 023 SS3.5 already requires clickable breadcrumb crumbs. A keyboard leave-all-rungs key is NOT built and NOT offered (R83) - operator question (al)."
    },

    "Q3_state_representation": {
      "one_type": "canvas::LevelPath { containers: Vec<u32>, object: Option<u32>, subpath: Option<u32>, node: Option<u32> } + enum Rung { Page, Container, Object, Subpath, Node } + enum DescendOutcome { Descended(Rung), NothingToEnter(Rung), AlreadyDeepest }.",
      "prefix_invariant": "The four fields form a PREFIX CHAIN: subpath.is_some() implies object.is_some(); node.is_some() implies subpath.is_some(). Written only by descend/ascend/Default, guarded by debug_assert!(is_well_formed()). R112's shape applied to level (023 SS8 item 7's strengthening).",
      "node_index_is_object_scoped": "NOT renumbered per subpath. It is anchor_count's decomposition order, the space node-move --node N addresses (edit.rs:2129-2145). The subpath rung FILTERS which anchors are shown and clickable. A second numbering would be R92 + decision 011's Z2.",
      "Vec_not_SmallVec": "023 SS2.5 proposed SmallVec<[u32;2]>. smallvec is NOT a workspace dependency (verified: zero hits across every Cargo.toml), and an empty Vec does not allocate, so on the files this ships for the heap cost is exactly zero. Adding a dependency to avoid an allocation that never happens fails rule 13 on its face.",
      "LevelPath_is_not_TargetId": "Where you are STANDING vs what is SELECTED. A marquee can select forty objects while standing at Page; standing inside an object selects nothing by itself. canvas_selection: BTreeSet<TargetId> keeps its meaning unchanged. Merging them would be wrong.",
      "why_one_type_and_not_two_fields": "group_depth: usize alongside entered: Option<EnteredObject> admits group_depth=2 with entered=Some(..) and nothing in the types says whether that is coherent. The Escape chain reads one, the readout reads the other, they disagree the first time one is updated without the other. That is decision 022 SS2's rejected option (d) - a partition maintained by convention rather than by type.",
      "migration_owed": [
        "canvas.rs:225-232 EnteredObject DELETED, not extended",
        "canvas.rs:260-282 depth_after_click -> next_level_after_click(&LevelPath, double, &Descent) -> (LevelPath, DescendOutcome); same purity, same no-egui testability",
        "canvas.rs:1489-1553 the 7 tests port field-for-field (they encode the right rules) plus new rung/invariant cases",
        "main.rs:1723-1729 entered: Option<EnteredObject> -> level: LevelPath (NO Option - Page is the zero value, removing the shipped conflation of 'at Page' with 'no document')",
        "main.rs:1892 constructor -> LevelPath::default()",
        "main.rs:2155-2182 apply_click_depth KEEPS its name and one-method discipline (R92); body becomes a rung-aware probe",
        "main.rs:4878-4886 DELETE the entered=None line",
        "NEW Action::LeaveLevel -> doc.level.ascend(); never touches canvas_selection",
        "canvas.rs:606-620 resolve_escape -> EscapeContext { context_menu_open, gesture_discardable, tool_active, depth, canvas_selection_nonempty } (R120)",
        "main.rs:2208 entered=None -> level = LevelPath::default(); semantics UNCHANGED, doc comment still right; Pass 26.2 changes it only by ADDING re-validation",
        "main.rs:11195-11215 selection_readout -> the rung matrix",
        "ui_text.rs:2013-2043 two-fragment assembly -> one complete catalog string per matrix row (R1/R2)",
        "main.rs:11497-11512 outline draw keyed on level.rung(); Node rung adds anchor handles",
        "TargetId::Content(u64) -> Content(ContentPath { stream, index }) - payload only, no substrate change (022 SS2.5, R112), adopted unchanged"
      ],
      "two_things_the_migration_must_NOT_do": [
        "Must NOT put LevelPath in pdfce-core. 023 SS11 named this and it is sharper now because LevelPath indexes PageObjects at four levels, so 'store it next to the objects it indexes' reads as tidy. cargo tree is the gate (criterion F8).",
        "Must NOT build the container forest in the GUI provider. Container detection is content-stream parsing; that is core. The provider stays a read-only window (subpath_hits, subpath_bounds_canvas) onto core functions."
      ]
    },

    "Q4_disclosure_obligation": {
      "shipped_string_problems": [
        "knows nothing about containers, nodes or the Page rung",
        "ASSEMBLED FROM FRAGMENTS - `let scope = format!(..)` then `format!(\"Inside {scope} - part #{sp} is selected...\")` (ui_text.rs:2013-2032). R2 forbids sentence assembly because fragment order is not translatable, and `part(s)` bakes in an English pluralisation. With a ten-row matrix this is the moment to fix it: ONE complete catalog string per row.",
        "leaves the flattened-CAD Page rung unaddressed - which is where 023's own specified string is factually wrong"
      ],
      "matrix_rows": {
        "Page_no_containers": "'Top level. This page has no groups - objects are the outermost level. Double-click an object to work on its parts.' POSITIVE terms AND still offers the descent.",
        "Page_with_containers": "'Top level. 3 groups on this page. Double-click one to work inside it.'",
        "Page_object_selected_no_containers": "'Path selected (object #11), drawn as 1 part. This object is not inside a group. Double-click it to work on its parts.'",
        "Page_ce_dimension_selected": "'Measurement selected (5.000 m, group 2). A measurement is one object - it has no parts. Use Re-measure to change it.' (L3)",
        "Container_form_invoked_once": "'Inside group \"Fig1\" (a form XObject) - 47 objects. This group is placed once, so edits here affect only this placement.' - 023 SS1.4's disclosure on the COMMON case, so the rule is learned before the refusal.",
        "Container_form_invoked_many": "'... This group is placed 12 times in this document; editing inside it would change all 12. Editing is not available here.' - the MEASURED count, stated before any verb is attempted.",
        "Container_marked_content": "'Inside group /Span (a marked-content section) - 12 objects. ...'",
        "Object_N_gt_1_parts": "'Inside object #5870 - it is drawn as 1,194 parts. Part #667 is selected. Double-click a part to work on its points.' (+ '2 of 3 at this point' when several parts are under the pointer)",
        "Object_exactly_1_part": "'Inside object #11 - it is drawn as a single part, which is already selected. Double-click it to work on its points.' THE SINGLE-SUBPATH RUNG IS ENTERED AND DISCLOSED, NOT SKIPPED.",
        "Object_text_image_form": "'Object #40 is text - it has no parts and no points. Press Escape to go back.' NothingToEnter, reported. Descent stops here (decision 011 SS2.1).",
        "Subpath_under_ceiling": "'Inside part #667 of object #5870 - 6 points. Click a point to select it.'",
        "Subpath_over_ceiling": "'Inside part #3 - 41,208 points (limit 2,000). Points are not shown; this part is too complex to edit point by point.' NO handles, never a silent first-N.",
        "Node_selected": "'Point #1,204 of object #5870 (part #667) selected. This is the deepest level.' - index is object-scoped and the string says which object it counts within, so the number matches node-move --node.",
        "Node_double_click": "'This is the deepest level - there is nothing below a point.' (AlreadyDeepest)"
      },
      "flattened_cad_case": "Must do TWO things at once that 023's string cannot: say there is no group in POSITIVE terms, AND still offer the descent, because there IS one.",
      "breadcrumb": "container-free: 'Page > Path #5870 > Part #667 > Point #1,204'. With containers: 'Page > Fig1 > /Span > Path #12 > Part #3 > Point #7'. Every crumb clickable (023 SS3.5), which turns the 8-press Escape walk into a one-click jump. R84 satisfied by construction - text plus separators.",
      "ceiling_revisited": "023 SS4.6 reasoned from 'nodes for a whole group' (41,208 handles). Under the ladder the Node rung shows ONE part's anchors: 6,681 / 1,194 = ~6 per part on the measured page. The ceiling stays for a long hatch boundary but stops being the common case, and the disclosure changes from 'select fewer objects' to 'this part is too complex to edit point by point'.",
      "headless": "object-list --page N --tree (023 SS7.2) + --enter INDEX --subpaths (NEW: per-part bbox, anchor count, and OBJECT-SCOPED anchor index range - which is what keeps node-move --node unchanged) + --hit X,Y --level N (023 C3)."
    },

    "Q5_subpath_edit_verbs": {
      "blocker_nobody_stated": "Subpath (decompose.rs:224-232) has start/segments/closed and NOTHING ELSE. PathObject has tokens: TokenRange and bytes: ByteSpan; every shipped planner works from them (plan_move iterates ops_in_range(obj.tokens..), plan_delete splices obj.bytes()). So neither subpath verb is EXPRESSIBLE against today's model - not hard, not expressible. Largest unstated cost in the ROADMAP Next-up entry.",
      "fix": "Subpath gains tokens + bytes, recorded AT DECOMPOSITION TIME (option i). NOT re-walking at plan time (option ii), because hit_test_subpaths already hands out an ORDINAL and an ordinal is only meaningful if exactly ONE traversal defines it - a second traversal is decision 011's Z2 class and R111's shape. Cost: two index pairs per subpath, ~6,700 subpaths on the measured page, trivial. Subpath::transformed carries them through unchanged (a page-space transform does not change bytes). Blast radius: tests constructing Subpath literals; PartialEq now compares spans.",
      "delete_semantics": "Subpaths share ONE painting operator, so a delete removes construction operators only and the object survives with one fewer part. Interior m-opened subpath: its m through the operator before the next opener. A bare re: the single operator. The first subpath: same. A trailing h goes with its subpath. W/W* clipping sits between the last construction operator and the painting operator (SS8.5.4), so it is never inside a subpath span.",
      "refusals": {
        "LastSubpathOfObject": "NEW. The delete (or delete SET) would leave zero subpaths, so the painting operator would have no current path (SS8.5.3). Three handlings, two wrong: silently removing the painting operator makes 'delete a part' silently become 'delete the object' (rule 4's exact prohibition); leaving a bare operator leaves a zero-part object occupying a paint-order index. REFUSE - the operator's route already exists and is unambiguous: delete_object. Structurally identical to 023 SS4.4's SubpathWouldDegenerate one rung down. For a multi-part delete the check is on the SET, not per item.",
        "DeleteWouldMoveNextSubpath": "NEW, AND THE SHARPEST FINDING IN THE RECORD - 023 SS1.4's form-aliasing trap one object space down. VERIFIED MECHANISM: close_subpath (decompose.rs:1859-1868) sets pa.current = pa.subpath_start and needs_move = true; a following l/c/v/y then opens a Subpath via current_for_segment (:1803-1824) whose start is INHERITED from the closed subpath's start, carried by NO OPERAND OF ITS OWN (SS8.5.2.1 - the same construct 023 SS4.4 named ImplicitNode at the node rung). Excise the preceding subpath and the follower's start silently changes. The edit is byte-minimal, passes --verify-undo, passes content-identity for every other object - and a line the operator never selected has moved. MINIMAL-DIFF DISCIPLINE CANNOT CATCH THIS; ONLY THE NAMED REFUSAL CAN. Reachable from any '... h ... l ...' sequence. In a multi-part delete, evaluate against the SURVIVING set - deleting both closer and reopened follower is fine.",
        "ImplicitSubpathStart": "NEW. The subpath being MOVED is itself h-reopened: its start carries no operand, so translating its explicit operands alone would tear it.",
        "existing_inherited": ["MalformedOperand (move)", "DegenerateCtm (move ONLY)", "NotAPath (both)", "SubpathOutOfRange (new, both)", "FormStreamIsShared (023 SS1.4, both)"]
      },
      "must_NOT_be_raised": "DegenerateCtm for subpath DELETE. A delete needs no coordinate transform, hence no CTM inverse, hence no such failure exists. 023 SS4.4's argument for node delete, verbatim. Raising it would be a refusal no test could honestly reach.",
      "api": {
        "planners": "plan_move_subpaths(content, obj, subpaths: &[usize], dx_page, dy_page) and plan_delete_subpaths(content, obj, subpaths: &[usize]) - PLURAL FROM THE OUTSET (023 SS4.2's rule for nodes: N sequential calls = N undo steps + N re-emissions, violating 022 SS5.5's R107-shape). Selecting a hatch's twelve parts and pressing Delete is one operation to the operator.",
        "session": "EditSession::move_subpaths / delete_subpaths, both through the existing vector_surgery skeleton (edit.rs:2189) so they inherit the encryption guard, the DocMDP certification guard, base-decompose indexing, VectorEditNeedsReopen and undoable staging for free. New CommandKind::MoveSubpaths / DeleteSubpaths.",
        "cli": "pdfce-cli subpath-move <pdf> --page N --object N --subpath N[,N,..] --dx D --dy D -o out.pdf [--verify-undo]; pdfce-cli subpath-delete <pdf> --page N --object N --subpath N[,N,..] -o out.pdf [--verify-undo]; pdfce-cli object-list --enter N --subpaths (the oracle). node-move --node N UNCHANGED - object-scoped, and that non-change is an acceptance criterion (G6), not an omission."
      },
      "round_trip_and_R58": {
        "minimal_diff": "Both verbs re-emit exactly one content stream, spliced at the subpath's byte span; every other object byte-verbatim. R46's named exception, same family as Pass 9c-min's plan_delete/plan_move/plan_move_node. The distinguishing claim is a BYTE-SPAN ASSERTION, not a code review (023 SS11's 'respliced-too-much' risk, one granularity up).",
        "save_mode": "Both are removals/translations whose contract is NOT confidentiality, so they stay under the default INCREMENTAL save - the same posture delete_object (9c-min) and delete_redaction_mark (Pass 8) already have.",
        "R58_fourth_instance": "Under R58's LITERAL text ('every removal/scrub operation rides R35's forced FULL REWRITE'), delete_subpaths would be required to force a full rewrite, WHICH WOULD BE WRONG. It is the FOURTH contradicting instance after delete_object, delete_redaction_mark, and 022's proposed delete_annotation. This record does NOT decide open question (v)'s wording fix - it adds the fourth data point and states the consequence. Recorded as question (ao).",
        "R35_R67": "R35 untouched (confidentiality only). R67 applies unchanged - a recovered-xref document forces full rewrite regardless of verb."
      },
      "pdf_dimension_hazard": "On the operator's own files the subpaths inside a drawing-view object INCLUDE the geometry of its PDF DIMENSIONS - witness lines, extension lines, arrowheads, leaders - drawn as ordinary path subpaths in the same object, with the numeral in a DIFFERENT object entirely. Deleting one part can silently destroy half of a pdf dimension: delete a witness line and the '55 5/8\"' numeral is still printed, still correct-looking, and no longer points at anything. NOTHING in the file marks those subpaths as belonging together - no container (SS1.1's whole premise), no marked content, no annotation. pdfce CANNOT detect this and must not pretend to: inferring 'these subpaths and that text are one dimension' from geometry is exactly the spatial heuristic 023 SS1.2 refused, and inferring it in order to BLOCK an edit would be worse than inferring it to select. The honest response is DISCLOSURE AT THE VERB, not detection: the delete confirmation states that parts are geometry only and that pdfce cannot tell whether a part belongs to a printed dimension. One string; criterion G7. This is also the clearest illustration of why rule 15 is a rule - a CE DIMENSION cannot be damaged this way (one annotation, baked /AP, descent refused by L3) and a PDF DIMENSION is defenceless.",
      "standing_position_cost": "A subpath edit fires refresh_pages -> prune_canvas_selection, which today resets the whole path, so the operator is ejected from three rungs deep to Page on a ~5,900-object page. Pass 26.2."
    },

    "Q6_pass_plan": {
      "family": 26,
      "ceiling_verified": "decision records 024 -> next free 025; Pass families headed to 25 (24 claimed by decision 024, 22/23 by decisions 022/023) -> next free family 26; standing rules R129 -> next free R130; open questions to (aj) -> this record claims (ak)-(ap). READ FROM THE BRANCH (git show pass-8-redaction:docs/ROADMAP.md), because the checker run inside this pinned worktree reported stale ceilings for all three ledgers simultaneously.",
      "does_23.2_still_stand": "IT SPLITS. Core+CLI half STANDS UNCHANGED (containers, marked-content ranges, form sub-models reusing pdfce-render's cycle set and depth cap, object-list --tree/--level, ContentPath payload, criteria C1/C2/C4/C7/C8). GUI half (double-click descend, Escape ascend, EscapeContext, breadcrumb) is SUPERSEDED by Pass 26.0 - building it for containers alone would build the second mechanism this record exists to prevent. Criterion C6 is WRONG and must be amended. Net: 23.2 is ordered AFTER 26.0 and is no longer on the critical path for the operator's own use case at all; it remains genuinely wanted for OTHER files as the only thing that closes R111's form-XObject violation.",
      "23.3_dependency_improves": "023 SS7.4 says 23.3 depends on 23.2. Under the ladder its real dependency is 26.0 (the node rung), NOT containers. A genuine unblocking - 23.3 no longer waits on the larger container-shaped Pass with no payoff on the operator's own files.",
      "passes": [
        {
          "id": "26.0",
          "name": "The unified ladder: LevelPath, the node rung, the fixed Escape, the readout matrix",
          "depends_on": ["22.0"],
          "note": "GUI-only. READ-ONLY - writes zero content bytes. Containers OUT of scope (23.2 adds them by extending LevelPath::containers, which is why that field exists from day one). Ships on the operator's OWN files.",
          "acceptance": [
            "F1 three double-clicks from Page reach a point of part #667 of object #5870 on the SolidWorks fixture; the breadcrumb names all four rungs",
            "F2 Escape pops exactly ONE rung per press - a unit test over EscapeContext walking node->subpath->object->Page->ExitTool->ClearCanvasSelection->rail, asserting no press skips",
            "F3 Action::ClearCanvasSelection no longer changes the rung",
            "F4 a single-part object is ENTERED AND DISCLOSED, not skipped",
            "F5 NothingToEnter fires and is reported for a text object and for a ce dimension (L3)",
            "F6 with three parts under the pointer, repeated clicks cycle all three and the readout reads '2 of 3 at this point'",
            "F7 object-list --enter 5870 --subpaths prints 1,194 rows whose object-scoped anchor ranges sum to anchor_count; node-move --node N byte-identical on a golden fixture",
            "F8 cargo tree -p pdfce-core -p pdfce-render GUI-free; ZERO content bytes written",
            "F9 R86 WATCH: does the breadcrumb read as navigation; is three-double-clicks-to-a-point tolerable"
          ]
        },
        {
          "id": "26.1",
          "name": "Subpath edit verbs: spans, move, delete, three new refusals, CLI",
          "depends_on": ["26.0"],
          "note": "Removes the 25.1 tooltip's 'not available yet' clause IN THE SAME COMMIT (R93 - a statement that has gone false).",
          "acceptance": [
            "G1 only the named subpath's construction operators change - a BYTE-SPAN assertion, not a code review",
            "G2 EVERY refusal FIRES from an input that would otherwise succeed (R96): LastSubpathOfObject on a 1-part object, DeleteWouldMoveNextSubpath on '... h ... l ...', ImplicitSubpathStart on a moved h-reopened part, FormStreamIsShared on a twice-placed form",
            "G3 DegenerateCtm is NOT raised for delete - a singular-CTM fixture deletes a part successfully",
            "G4 a 12-part delete is ONE command - undo depth 1, one re-emission",
            "G5 DeleteWouldMoveNextSubpath is evaluated against the SURVIVING set - deleting both closer and reopened follower SUCCEEDS",
            "G6 node-move --node N and object-move --object N byte-identical before and after this Pass",
            "G7 the delete confirmation discloses that a part is geometry only and that pdfce cannot tell whether it belongs to a printed pdf dimension",
            "G8 --verify-undo byte-identical on both verbs; save stays INCREMENTAL, flagged against R58 / question (v)"
          ]
        },
        {
          "id": "26.2",
          "name": "Level survival across an edit",
          "depends_on": ["26.1"],
          "note": "Its own Pass because it cannot be tested until the verbs exist, it has a distinct failure mode (a path surviving when it should not IS the 'selection that silently changed what it refers to' the current reset prevents), and getting it wrong is worse than not having it.",
          "acceptance": [
            "H1 moving part #667 leaves the operator standing at part #667",
            "H2 deleting part #10 of 1,194 leaves the operator at the OBJECT rung with a disclosure, never silently on a renumbered part",
            "H3 deleting part #1,100 leaves a standing position at part #10 intact",
            "H4 a page change or an edit to a DIFFERENT object still resets to Page - the shipped behaviour is not weakened",
            "H5 R86 WATCH: does re-validation feel like the tool kept your place, or like it moved you"
          ]
        },
        {
          "id": "23.2 (amended)",
          "name": "Containers join the ladder - core + CLI only",
          "depends_on": ["22.0", "26.0"],
          "acceptance": [
            "C1' object-list --tree prints a form's 3 children where today it prints 1 opaque object (023 C1, unchanged)",
            "C2' the R111 paint/select asymmetry closes for form XObjects, asserted from BOTH sides, one depth cap (023 C2, unchanged)",
            "C6' AMENDED - a container-free fixture prints containers=0 and the readout says 'this page has no groups - objects are the outermost level' AND STILL OFFERS the descent to parts",
            "C10 entering a container extends LevelPath::containers and L1/L2 hold unchanged at container rungs"
          ]
        }
      ],
      "sequencing_warning": "These Passes and decision 023's are interleaved by DEPENDENCY, not by number. A reader who schedules by Pass ID will build them in the wrong order. The librarian should carry the sequencing block into ROADMAP.md verbatim rather than leaving the ordering to be inferred from the IDs."
    }
  },

  "what_023_now_gets_factually_wrong": [
    "SS1.3's specified readout string 'this object is not inside a group (already at object level)' and the SS7.3 acceptance criterion C6 that REQUIRES it: WRONG on the operator's own files. Object level is not where the ladder ends there - 1,194 parts and 6,681 points sit below it. C6 would certify a false statement.",
    "SS0: 'A flattened CAD plot has TWO (objects, nodes)': WRONG - it has THREE below the page (object, subpath, node), and the omitted rung is the only one that makes the file selectable.",
    "SS1.3: 'That is the file's structure, not a pdfce limitation': WRONG. The file's structure had a level pdfce was discarding - the renderer stroked every subpath while selection saw one object. It WAS a pdfce limitation (R111's third instance) and Pass 25.0/25.1 removed it.",
    "SS4.6's ceiling arithmetic reasoned from 'nodes for a whole group' (41,208 handles): SUPERSEDED. The Node rung shows ONE part's anchors, ~6 per part on the measured page. The ceiling stays but stops being the common case, and its disclosure text changes.",
    "SS3.2's outcome name LeaveGroupLevel: A MISNOMER at three of five rungs. Renamed LeaveLevel. Naming, not fact - recorded because the name never shipped."
  ],
  "what_023_gets_NARROWED_not_wrong": [
    "SS10 item 4 / open question (z) 'fixed three levels or as deep as the file goes': the RECOMMENDATION stands and is adopted; the NUMBER in the question does not - it is four-plus, and only the container segment is variable-depth.",
    "SS1.2's heuristic-grouping refusal: NOT wrong and UPHELD - but its three grounds are now a TEST, which the subpath rung passes on all three while carrying two obligations 023 did not anticipate (the ordering's cycle disclosure; ordinal stability under same-object edits)."
  ],

  "proposed_standing_rules": [
    "A level ladder is walked by ONE state variable and ONE gesture pair; a second descent mechanism is a defect, not a feature. One double-click = one rung down, one Escape = one rung up, neither skips, and the rung a gesture acts on is read from the single standing-position value - never inferred from what happens to be under the pointer.",
    "A rung with no members on this file is REPORTED, never silently skipped and never silently entered. Skipping makes the same gesture mean different things on files the operator cannot tell apart, and forces the skip predicate to be duplicated in the ascent direction (R92).",
    "A finer selection level FILTERS the level below it; it never renumbers it. Adding a rung must not create a second index space for something already numbered. (Sourced: node indices stay object-scoped so node-move --node N keeps its meaning, exactly as 023 SS2.1 kept object-move --object N.)",
    "A structural element pdfce selects must record its own byte span WHERE ITS ORDINAL IS MINTED. An ordinal handed out by one traversal and resolved to bytes by a second is two derivations that must agree and eventually will not, silently.",
    "A checker that reports 'the highest number written' must be run against the BRANCH, not a working tree that can be behind. (Sourced: this record's own header - the ledger checker in a pinned worktree reported stale ceilings for Pass families, standing rules AND open-question letters simultaneously. Seventh numbering hazard on this project; a landed commit invisible to the checker, distinct from R106's stated case of an unlanded draft.)"
  ],

  "not_decided_here": [
    "023's container half - SS2's laminar interval model, SS2.3's Container/ContainerKind, the one-field PageObjects addition, the reused cycle guard and depth cap, and ContentPath are ADOPTED UNCHANGED",
    "the node-display ceiling VALUE - pdfce-ui-specialist + R86",
    "visual design of rung feedback, breadcrumb, container outline - pdfce-ui-specialist; this record owns only the requirement that the current rung is always visible and always clickable to ascend, and R84",
    "the operator-facing noun for a node ('part' shipped for subpath; 'node' in 023 and vector_edit_tool; 'point' in this record's illustrative strings). ONE word must be chosen and must not be a third invented here - pdfce-ui-specialist's call. Code names stay subpath/node.",
    "R58's wording fix (open question (v)) - this record adds the fourth data point and states the consequence; the wording is the operator's",
    "pdfce-inkscape-librarian was NOT dispatched. R61 means Inkscape is a behavioural reference only, and this ladder was derived from pdfce's own measured data. If the click-outside common-ancestor rule is to match a reference product precisely, that dispatch should happen before 26.0. Flagged, not done."
  ],

  "for_the_operator": [
    "(ak) Single-part objects: enter the rung and disclose (recommended), or skip it? One extra double-click buys a rule that is always true and an Escape chain with no skip predicate. Cheap to change later, which is why it is worth confirming rather than discovering.",
    "(al) A key to leave all rungs at once (Shift+Escape or similar)? Not built, not offered (R83) - the breadcrumb click already gives a one-click jump to any rung. Wanted as a keyboard equivalent?",
    "(am) After a part-edit, is being dropped to Page acceptable as an interim (26.1 ships, 26.2 follows), or should 26.2 be folded in so the rung survives from the first release of the verbs?",
    "(an) The subpath click-cycle - recommended and scoped into 26.0, because without it the part UNDER the nearest part is unreachable forever and will read as a hit-test bug. Confirm it is wanted, since it makes repeated clicking mean something new at that rung.",
    "(ao) delete_subpaths is the FOURTH non-confidentiality removal under R58's literal text, after delete_object, delete_redaction_mark and 022's proposed delete_annotation. Does it fold into open question (v)'s wording fix?",
    "(ap) DeleteWouldMoveNextSubpath - refuse by name (recommended), or ALSO offer 'materialize the follower's start as an explicit m, then delete' as a reviewable fix-up? Same trade as 023 SS10 item 3's form un-sharing: it writes bytes you did not ask for to enable the edit you did."
  ],

  "risks": {
    "gui_core_separation": [
      "core logic drifting INTO the GUI: container detection is content-stream parsing and belongs in core; the provider stays a read-only window onto core functions (cargo tree is the gate)",
      "GUI state drifting INTO core - SHARPER NOW: LevelPath indexes PageObjects at four levels, so 'store the current rung next to the objects it indexes' reads as tidy rather than as an invariant violation. PageObjects describes the DOCUMENT; where the operator is standing is the SHELL's. In core, the WASM fork inherits a UI mode."
    ],
    "round_trip_minimal_diff": [
      "26.0 writes NOTHING - pure view state and pure functions. That is why it is first.",
      "26.1 is R46-exception surgery at a finer granularity than any shipped verb; the named risk is respliced-too-much and the answer is a byte-span assertion (G1)",
      "SHARPEST (code): DeleteWouldMoveNextSubpath - a correct, minimal, byte-perfect edit that is still wrong. Minimal-diff discipline does not protect against it; only the measured, named refusal does. Same sentence 023 SS11 used for form aliasing, and the echo is the point.",
      "SHARPEST (not code): the pdf-dimension hazard. A part-delete can leave a printed dimension pointing at nothing, and pdfce cannot detect it without doing exactly the spatial inference 023 SS1.2 refused. Disclosure at the verb is the whole mitigation and it must not be dropped for brevity.",
      "26.2 writes nothing but decides whether a STALE standing position survives - the same class of hazard as a stale selection, hence H4's requirement that the shipped reset is not weakened where it is still right."
    ],
    "record": "This record's Passes and 023's are interleaved by dependency, not by number. Scheduling by Pass ID builds them in the wrong order."
  }
}
```
