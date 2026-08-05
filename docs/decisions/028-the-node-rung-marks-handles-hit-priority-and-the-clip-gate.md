# Decision 028 — The node rung made visible: marks, handles, hit priority, the breadcrumb, and the clip gate

**Status:** Design review returned and filed. **Amends decision 025 (§4, §7.2, §11).
025 is not edited** — `docs/decisions/README.md` is explicit: *"Files here are
append-only history: never edit a decision record after the fact — a reversed decision
gets a NEW record that references the old one."* This record is that new record, in the
same amendment style 025 used for 023 and 023 used for 022.

**Date:** 2026-08-05
**Author:** `pdfce-ui-specialist`, dispatched by `pdfce-engineer` to design the Node rung
before Pass 26.0 is built.
**Filed by:** `pdfce-librarian`.

**Decision number: 028** — read from the live ledger at write time, not assumed (R106).
`python tools/check-ledger-numbers.py --stats` at the tip of `pass-8-redaction`
(after `075e8f8`) reports `decision records: 027 -> next free is 028`,
`standing rules: R146 -> next free is R147`, and Pass families *claimed but not yet
headed* including **26** — so this record spends **no** new Pass family; Passes
26.0–26.2 were already claimed by decision 025.

**★ WHAT THIS RECORD IS, AND WHAT IT IS NOT.** It is a **design review of an existing
decision**, not a new architectural choice. Decision 025 decided *that* there is a Node
rung and *where* it sits in the ladder; it explicitly deferred three things to a
specialist — §11 lists *"the ceiling value, the breadcrumb's visual design, and the
operator-facing noun for a node"* as **not decided**. This record answers those, and in
answering them found **three defects in 025 as written**, one of which is a
**shipping-order requirement** rather than a cosmetic correction. Its one recommendation
about operator posture — on open question **(av)** — is recorded as a **recommendation,
not a resolution**. It is Ken's call and it is not made here.

**Terminology (binding — `CLAUDE.md` rule 15).** This record never writes bare
"dimension." Where it matters: a **pdf dimension**'s witness lines, arrowheads and leader
*are subpaths of some path object* on the operator's CAD exports, so the Node rung
descends into them exactly as into any other artwork, and every mark, handle and nudge
this record specifies will be applied to them. A **ce dimension** is a level-1 leaf
(025 §2.2 rule L3) — it has no subpath rung and no node rung, the ladder refuses to
descend into it **by name**, and nothing in this record creates a back door to that
refusal. The asymmetry is provenance, not representation, and it is deliberate.

**Cross-references:** `CLAUDE.md` rules **2** (GUI-core separation), **4** (fuzzy never
sneaky, as narrowed 2026-08-05 by decision 024 §4.4), **11** (CLI parity), **15**
(dimension terminology); decisions **022** (`TargetId`), **023** (level navigation —
§4.5 is corrected here), **024** (§4.2's three-tier confirm model and the
window-anchored tool strip; §4.4's narrowing of rule 4), **025** (this record's
subject), **027** (`PlannedEdit::disclosures`, `is_clipping_path`); standing rules
**R83** (no affordance without capability), **R84** (never colour alone), **R86**
(observed working), **R92** (no hand-duplicated predicates), **R106** (re-read the
ceiling), **R111** (selection enumerates what the renderer paints), **R117** (a shipped
capability reachable from no surface is a defect), **R121** (a confirm lives at a fixed
window-relative anchor), **R122** (every gesture with a commit has a keyboard commit),
**R144** (removing a refusal can remove an unrelated protection), **R145**
(`Result<(), E>` drops operator-visible information by default), **R147** (this record's
own instance — audit a removed refusal's **callers**).

---

## 0. Summary

Decision 025's ladder is adopted whole. What it lacks is the **bottom rung's visible
form**: 025 §4's readout matrix has a Node row, but nothing in the record says what a
node *looks like*, how a handle is distinguished from a node, which of the two wins a
click when they overlap, how the operator gets from one node to the next without the
mouse, or how they know how deep they are standing. This record supplies those, and
supplies them as a **replacement** for a gesture that is already live and ungated.

Three findings drive it.

1. **Decision 023 §4.5 is now factually wrong**, and 025 inherited the error by not
   correcting it. 023 says *"Nodes ≠ handles … the node-level readout must not imply
   handles exist."* **Pass 30.1 shipped handle editing the same day 025 was being
   scoped** (`d025c1a`, `plan_move_handle`, `pdfce-cli handle-move`). Handles exist. The
   Node readout row therefore needs a **handle-presence clause**, and that clause is
   the **only** way an operator ever learns handles are there — which makes it an R83
   obligation, not a nicety.

2. **★ There is already a blind, ungated node-drag gesture in the shipped GUI, and it
   must be REPLACED AND GATED by the rung — not shipped alongside it.** This is the
   record's one *required* item. Shipping the Node rung beside the existing gesture
   would give the ladder a **second, unscoped, invisible route to the same edit**, which
   is precisely the two-mechanisms failure 025 §2.1 diagnosed for descent and refused.
   Detail in §2; consequences for the roadmap's own claims in §2.3.

3. **025's Subpath-rung readout row omits the descent disclosure the Object row has.**
   The Object row tells the operator that double-clicking descends. The Subpath row does
   not — so at the rung immediately above the node rung, the operator is never told that
   the gesture which got them there works again.

Everything else in this record is answer, not critique.

---

## 1. What decision 025 left open, and the answers

025 §11 named three deferrals. All three are answered below, and the answers are
constrained rather than free: each reuses an existing visual vocabulary instead of
minting a fourth.

### 1.1 Node marks — shape carries the meaning, colour does not

| State | Mark | Space | Colour |
|---|---|---|---|
| Unselected node | **hollow square, 6 × 6** | **screen** | `SUBPATH_OUTLINE_COLOR` (`main.rs:278`, `rgb(210,140,40)`) |
| Selected node | **filled square, 8 × 8** | **screen** | the app accent |

Three properties, each load-bearing:

- **Screen space, not page space.** A mark drawn at a fixed page size shrinks to nothing
  when the operator zooms out to find their place and swells to a blob when they zoom in
  to aim. This is the same coordinate-space error `075e8f8` just fixed for the *grab
  radius*; drawing the mark in the other space from the radius that grabs it would be
  worse than either mistake alone, because the visible target and the actual target
  would disagree by a zoom-dependent factor.
- **Square vs. circle carries node-vs-handle** (§1.2). This satisfies **R84** —
  never colour alone — *by construction*, and it keeps working for an operator who
  cannot distinguish the accent from `SUBPATH_OUTLINE_COLOR`.
- **Selected is bigger AND filled**, two independent channels, again R84.

**Ceiling: 300 nodes per subpath**, **provisional under R86** — the number is a starting
value to be confirmed against the running application on the operator's own files, not a
measured result. 025 §4.6 already established why the ceiling stopped being the common
case: under the ladder the node rung shows the anchors of **one entered subpath**, and on
the measured SolidWorks page that is 6,681 anchors ÷ 1,194 subpaths ≈ **6 per part**. The
ceiling exists for the tail — a hatch boundary can be one long subpath.

**When the ceiling bites, it is stated in words: an explicit "points not shown" string,
never silent truncation.** A silently truncated set of marks is an affordance that lies
about what is selectable (R83's inverse), and the operator's only symptom would be *"some
of my points aren't there"* — indistinguishable from a hit-test bug.

### 1.2 Handles — shown for every node of the entered subpath that has one

| State | Mark |
|---|---|
| Unselected handle | **hollow circle, 5 × 5** (screen) |
| Selected handle | **filled circle, 7 × 7** (screen) |

**★ Shown for every node of the ENTERED subpath that has a handle — not
selected-node-only.** This is the non-obvious call and it has a reason: **handles are
what let the operator decide WHICH node to pick.** Revealing a node's handles only after
it is selected inverts the order of the decision — the operator must commit to a node in
order to see the information that would have told them whether it was the right node. On
a curve, the handle geometry *is* the visible difference between two otherwise identical
anchors.

**Each handle is tied to its node by a dashed 1.0 px arm**, reusing
`APPROXIMATE_OUTLINE_DASH` (`main.rs:11863`, `(6.0, 4.0)`). The reuse is deliberate and
is not a cosmetic economy: that dash pattern already carries the meaning *"this is not a
measured edge"* everywhere else in the canvas. A handle arm is exactly that — a control
vector, not geometry that will be inked. Inventing a fourth visual language for it would
teach the operator two things where one already suffices.

**A straight segment's ABSENT handle is disclosed in the status line, never drawn as a
ghost widget.** A greyed-out or dotted "handle that isn't there" is an affordance without
the capability behind it — R83 in its original form. The core already refuses this case
by name: Pass 30.1's `VectorEditError::NoHandleHere` exists precisely because *"an intent
exists for reshaping a curve, none for silently promoting a line the operator never drew
as a curve"* (`ARCHITECTURE.md` §12, decision 027's Pass 30.1 amendment). The GUI's job
is to **say** that, in the same words, before the operator reaches for something that
isn't there.

### 1.3 Hit priority — the smaller target first

> **handle (5 px) → node (6 px) → subpath body → nothing**

**The smaller target wins, and that ordering is the whole point.** The naive ordering —
node first, because a node is the more important object — fails exactly when it matters
most: **a handle sits close to its node precisely when the curve through that node is
nearly flat.** Under node-priority the handle becomes unreachable in the one case where
the operator most needs it, namely nudging a nearly-straight curve into shape. The
tolerance figures (5 px / 6 px) are screen measures and go through
`canvas::screen_tolerance_to_page` (`canvas.rs:1411`) like every other canvas tolerance
in the codebase.

**Handle drag is Node-rung only.** It is not available at Subpath, not at Object, not at
Page. There is no rung above Node at which "drag this control point" has a defined
meaning, and offering it there would be the second unscoped route this record exists to
prevent.

### 1.4 The breadcrumb — net new; nothing exists today

```
Page  ›  Path #5870  ›  Part #667  ›  Point #1,204
```

Each segment is **clickable to ascend** to that rung. It is net new: there is no
breadcrumb in the shipped GUI at all, and 025 §11 explicitly declined to design one.

**Its most valuable property is the one that is easiest to overlook: its growth after the
first double-click is itself the confirmation that the gesture did something.** Descent
is otherwise a state change with no visible mark — the operator double-clicks, the
picture does not change, and there is nothing on screen that distinguishes "I descended"
from "the click missed." 025 §3.5 identified exactly this hazard when it argued for
deleting `doc.entered = None` from `Action::ClearCanvasSelection`: *"an
inside-with-nothing-selected state that looks identical to outside is a real hazard."*
025 discharged it by promising the rung is *"always named in the readout and the
breadcrumb"* — this record is where the breadcrumb half of that promise gets built.

Number formatting is thousands-separated (`#1,204`, `#5870` per the existing object-index
convention) because these are four-digit ordinals on the operator's real files, not toy
indices.

### 1.5 Keyboard

| Key | Action |
|---|---|
| **Tab** / **Shift+Tab** | cycle to next / previous node |
| **Arrow** | nudge the selected node 1 pt |
| **Shift + Arrow** | nudge 10 pt |

**★ Tab cycles in OBJECT-scoped order.** Not subpath-scoped. This is **R92** applied to
an index space: `plan_move_node`'s `node_index` is into the object's anchors in
decomposition order, flattened across every subpath (`edit.rs:2129-2145`), and that is
the space `anchor_count` reports and `node-move --node N` addresses. If Tab cycled in a
subpath-local order, **what Tab lands on and what `node-move --node N` addresses would
disagree** — two numberings for the same anchors, which must then be kept in agreement,
which is decision 011's Z2 defect class. 025 §1.3(b) already committed to object-scoped
node indices as a positive dividend of the four-rung ladder; this is that commitment
reaching the keyboard.

The subpath rung **filters which anchors are shown and clickable; it does not renumber
them.** Tab therefore visits the entered subpath's anchors *in their object-scoped
order*, and the breadcrumb's `Point #1,204` is the number the CLI would take.

### 1.6 Toolbar tooltip correction

The Obj-tool tooltip is corrected to describe the ladder, so the rung is **discoverable
before trying anything**. Today an operator learns that double-click descends only by
double-clicking and noticing — which, per §1.4, produces no visible change at all until
the breadcrumb exists. Discoverability of a gesture whose success is invisible cannot be
left to experiment.

---

## 2. ★ The required item: the existing gesture must be REPLACED AND GATED, not joined

This is the record's one item marked **REQUIRED**, and it belongs in the **head slice** of
Pass 26.0 — not in a cleanup pass afterwards.

### 2.1 What is already live

`vector_edit_tool::classify_drag` (`vector_edit_tool.rs:86`) runs over
`object_provider::object_sample_points` (`object_provider.rs:200`), which returns **the
whole object's flat anchor list**. A drag that starts within the grab radius of any
anchor in that list classifies as a node grab — at **any** rung, with **no** rung state
consulted, and with **nothing drawn beforehand** to say which node is about to move.

There is no Node rung today. The gesture predates it and is not gated by it.

### 2.2 Why joining is not an option

If Pass 26.0 adds a scoped, marked, disclosed Node rung and leaves this gesture in place,
the ladder ships with **two routes to the same edit**: one scoped to the entered subpath,
marked on screen, addressable by keyboard, breadcrumbed; and one that works anywhere, on
any anchor of the whole object, with no marks and no disclosure. That is not redundancy —
it is the **exact structural failure 025 §2.1 identified and refused for descent**:

> *"The collision exists only if descent is implemented twice … Two predicates answering
> 'descend?' is R92's shape."*

Two mechanisms answering *"is the operator moving a node?"* is the same shape one rung
down, and it fails the same way: the two will disagree, and the one that wins will be
whichever runs first, which is not a decision anybody made.

### 2.3 ★ The R144 second firing, and the roadmap claim it makes imprecise

**This is R144's second instance, on the same Pass, and it is filed here because it was
found by reasoning about the CORE while the defect lived in the core's CALLERS.**

`Subpath::anchors()` yields `start` plus each segment end. So **all four corners of an
`re`**, and **the inherited start of an `h`-reopened subpath**, have **always** been in
`object_sample_points`'s output, and therefore always candidates for this gesture.

| | Before Pass 30.0 | After Pass 30.0 |
|---|---|---|
| Drag such an anchor | classified as a node grab, then **refused on release** (`VectorEditError::RectangleNode` / `::ImplicitNode`) | **succeeds** — the rectangle expands, or the `m` is materialized — with only a **post-hoc note** |

The refusal was the gate. Lifting it in Pass 30.0 (`a56bdd7`) opened a gesture the Pass
was not reasoning about, because the Pass was reasoning about the planner. **On a clipping
path — which is overwhelmingly an `re` rectangle — content elsewhere on the page can now
change from a drag that was previously a no-op.**

**Consequence for the record, stated as a correction:** `ROADMAP.md`'s Pass 26.0–26.2
entry claims that after Pass 30.0 *"every anchor on a page is addressable by the core
planner and **none** of it is addressable by hand."* **That is imprecise.** `re` corners
and reused subpath starts were **already reachable by hand** from this gesture — before
Pass 30.0 as a refused grab, after it as a successful edit. What is genuinely unreachable
without the CLI is **Bézier handles** (Pass 30.1), and nothing else. The R117 framing of
the priority raise survives intact; only the totalizer in it does not.

**The rule this yields is R147** (§4): *when a refusal is removed, audit its **callers**,
not just its own module.* The protection a refusal provides is felt **where it is
invoked**, and core-side reasoning cannot see it.

### 2.4 What "replaced and gated" means concretely

- `classify_drag` consults the current rung. A node grab is possible **only** at the Node
  rung, and only over the **entered subpath's** anchors — not the whole object's list.
- At every other rung the same drag means what that rung's own verb means (at Subpath: a
  subpath move, §3).
- The gesture's candidate set becomes the same set that is **drawn** — which is **R111**,
  paint/select symmetry, applied at the node rung: what the renderer marks is what the
  pointer can grab, and nothing else.

---

## 3. Pass 28.0's subpath move gets its GUI gesture here

Pass 28.0 (`d8b9735`) shipped `plan_move_subpath` and the `Subpath` token spans that made
it expressible, with **no GUI gesture** — another R117 item stacked behind the ladder.

**The gesture is: a plain drag on the entered subpath's body.** It belongs in this record
rather than a separate Pass because it falls out of the hit-priority ladder (§1.3) for
free: `handle → node → subpath body → nothing`. Once the third rank exists as a hit
target, the drag that starts there has exactly one reasonable meaning, and it is the verb
the core already ships.

---

## 4. Standing rule proposed

**R147 — when a refusal is removed, audit its CALLERS, not just its own module.**
Assigned by the librarian at filing; full text in `ROADMAP.md`'s *Standing rules*. It is
R144's caller-side corollary: R144 says lifting a refusal can remove an unrelated
protection; R147 says **where to look for it** — the protection is felt at the call
sites, not in the module that stopped refusing, and a review that stays inside the core
is structurally incapable of seeing it. Instance: §2.3.

**A second, smaller finding is filed with the fixes rather than as a rule** (`ROADMAP.md`,
the `5b2682b` entry): **a discard WITH a justification is harder to find than a bare one,
and no more correct.** The three bare `let _ =` instances fixed in `d8b9735` were found
because they *looked* like shortcuts; the fourth survived an extra day precisely because
a comment made it read as considered. It is filed as an audit-methodology corollary to
R145 rather than as its own rule, because its actionable content — *do not let an
explanatory comment terminate an audit* — is already R143's territory (*a refusal's
stated reason is a claim to test, not a fact*) pointed at a different construct.

---

## 5. ★ Recommendation on OPEN OPERATOR QUESTION (av) — a RECOMMENDATION, not a resolution

**(av) asks: moving a clipping path — is DISCLOSE the posture you want, or should it be a
confirm?** The default shipped today is disclose-and-proceed. **The answer is Ken's; this
section records what the specialist recommends and why, and nothing here changes shipped
behaviour.**

### 5.1 The recommendation, split

| Case | Recommended posture | Tier (024 §4.2) |
|---|---|---|
| `re`-corner expansion | **post-hoc disclosure is SUFFICIENT** | Tier 1 |
| `v`/`y` → `c` handle promotion (Pass 30.1) | **post-hoc disclosure is SUFFICIENT** | Tier 1 |
| **Moving a clipping path** | **post-hoc disclosure is INSUFFICIENT** | **Tier 2** |

### 5.2 Why the first two are Tier 1

The picture is **byte-identical**. Nothing changed anywhere the operator is not looking.
`re` → `m`/`l`/`l`/`l`/`h` and `v`/`y` → `c` are re-spellings that materialize operands the
file left implicit (ISO 32000-1 §8.5.2.1 Table 59); the rendered result before and after
is the same page. Ctrl+Z is a complete escape hatch, because `EditSession`'s command log
makes each of these exactly one undoable command.

**Gating these would be decision 024's "Over-application A"** — putting a confirm in front
of a direct manipulation the operator just performed and can see. That is the failure the
narrowing of rule 4 (024 §4.4) was written to stop, and it is the failure the operator
reported in his own words: *"there is a separate accept / reject box somewhere on the
screen to click — I've never seen any other software operate that way."*

### 5.3 Why a clip move is different, argued from the narrowing's own text

The narrowing's Tier-1 carve-out is explicitly *"a direct manipulation whose result is
**fully visible on the canvas**."* A clip move fails that clause on its face: **the
consequence lands elsewhere on the page, possibly outside the viewport.** The visible
result of the gesture and the material result are in **different places on the screen** —
this is the same fact R144's instance recorded, where dragging a full-page clip's corner
*"rendered correctly and made an unrelated drawing elsewhere on the page vanish, with
nothing at all changing at the cursor."*

**The precise claim, stated carefully because it is the subtle part: this meets Tier 2's
own test even though nothing was INFERRED in the fuzzy sense.** Rule 4 governs things
pdfce *guessed*. pdfce guessed nothing here — the operator dragged a corner and pdfce
moved that corner. **The uncertainty is about WHERE THE CONSEQUENCE LANDS, not about a
guessed value.** Tier 2's stated trigger is *"pdfce computed something the operator did
not directly specify, **or** when a disclosure must be read before the result becomes
document state"* — and it is the second limb, not the first, that a clip move satisfies.

### 5.4 The proposed mechanism introduces nothing new

**Route clip-gated drags to the EXISTING window-anchored tool strip** — decision 024
§4.2's Tier 2 mechanism, already designed, already argued, already the confirm's permanent
home:

- **Accept / Reject at the strip's fixed right anchor.** This satisfies the operator's
  hard constraint (**R121**) that a confirm control must **not** be positioned relative to
  the page. The strip is a `Panel::top`, so its position is a function of the **window**;
  it cannot drift off screen on zoom, scroll or page change, and it never covers document
  content.
- **Enter accepts; Escape rejects through the existing `resolve_escape` chain**
  (`canvas.rs:606-620`), never through a private per-tool reject path. **R122.**
- **The strip shows the CORE-AUTHORED disclosure string verbatim** — the `Vec<String>`
  that Pass 30.0's `PlannedEdit::disclosures` already produces (**R145**) — **never a
  `ui_text` paraphrase.** A GUI-side restatement of a core disclosure is a second
  authority on what the edit did, and the two drift.

**It needs exactly one new symbol**: a read-only provider predicate

```rust
fn object_is_clipping_path(&self, index: usize) -> bool
```

**mirroring the core's existing `is_clipping_path` (`edit.rs:595`) rather than
re-deriving it** — **R92**. A GUI-side re-derivation of "is this a clip" would be a second
definition of the same predicate, maintained by nobody, drifting the first time the core's
`W`/`W*` handling changes.

### 5.5 What Ken is actually being asked

Not *"disclose or confirm"* in general — the recommendation says **disclose** for two of
the three cases and would leave them exactly as shipped. The question is narrower:
**should a clip-geometry drag alone be promoted to the fixed-anchor confirm?** The cost of
yes is one extra deliberate act on an operation that is rare and whose consequence is
off-screen. The cost of no is that a drag can change what is visible somewhere the
operator is not looking, with the notice arriving after the fact.

**Not refusal, in either direction.** Refusing clip-geometry edits reinstates the
permanent-uneditability problem (av)'s own default text names, and resizing a crop region
is a real task.

---

## 6. What this record does NOT decide

- **The operator-facing noun for a node.** 025 §11 deferred it; this record uses
  *"Point"* in the breadcrumb and *"Part"* for a subpath, matching 025 §2.2's own table,
  but does not claim that as a decision.
- **The ceiling value (300).** Provisional under **R86** — confirm against the running
  application on the operator's files.
- **Question (av).** §5 is a recommendation. The shipped posture is unchanged until Ken
  answers.
- **The container rung's visual treatment.** 025 §3.3 item 13 assigned it to
  `pdfce-ui-specialist`; there are no containers on the operator's files (`k = 0`), so it
  is not on this record's critical path.
