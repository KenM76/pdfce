# Decision 023 — The Obj tool is for everything: level navigation, node-level editing, dimension re-measure, and the missing format surface

**Status:** Decided (consultant recommendation; engineer to schedule, librarian to file)
**Date:** 2026-08-04
**Requested by:** pdfce-engineer
**Amends:** decision **022** (annotations in canvas selection) — see §8. **022 is not
edited.** `docs/decisions/README.md` is explicit: *"Files here are append-only history:
never edit a decision record after the fact — a reversed decision gets a NEW record that
references the old one."* The engineer's instruction to "amend 022" is therefore executed
as *this record*, plus a librarian-owned forward-reference from 022's ledger entry and
`ARCHITECTURE.md` §12. Nothing in `022-annotations-in-canvas-selection.md` is rewritten.

**Decision number:** **023** — verified against the live ceiling at write time, not
assumed. `python tools/check-ledger-numbers.py --stats` reports
`decision records: 022 -> next free is 023`; same run: `standing rules: R110 -> next free
is R111`; `Pass families with headings: up to 21 (highest ID 21.1)`,
`CLAIMED BUT NOT YET HEADED: 5, 9, 9c, 10, 13, 20`. **Pass family 22 is claimed in prose
by decision 022** (its §6 proposes Pass 22.0) but is not yet headed, so this record claims
**Pass family 23**. Re-run the checker at filing time — this project has collided on
numbering repeatedly, which is why 022 carries the same warning.

---

## 0. Summary

The operator's answer to 022's open question is **yes, the Obj tool is for everything**,
plus three additions (level navigation, node-level operations for a group, dimension
re-measure) and one gap report (units and display type unreachable in the GUI).

Five findings drive this record. Each was verified in the code, not inferred:

1. **PDF has no uniform notion of a group, but it does have a uniform notion of
   *containment*, and that containment is always a contiguous interval in paint order or
   a separate content stream.** This is what makes the operator's model buildable at all
   — and it is why the flat `Vec<VectorObject>` must **stay** flat rather than becoming a
   tree. §1, §2.

2. **The paint/select asymmetry decision 022 found in annotations exists a second time,
   in form XObjects, and nobody has noticed.** `pdfce-render`'s interpreter recurses into
   a form (`interpret.rs:47` — *"`Do` on a form is the interpreter's only recursion"*,
   with a cycle set and a depth cap), so a form's contents are **painted individually**.
   `decompose.rs:2236-2241` emits **one opaque `ImageObject`** for the same `Do`. Every
   line inside a title block, a hatch, or a placed CAD block is visible and unselectable —
   the exact defect 022 reported for dimensions, one object space over. 022's proposed
   rule ("selection enumerates exactly what the renderer paints") therefore has **two**
   live violations, not one. §1.2.

3. **Editing inside a form XObject edits every invocation of it.** A form's content stream
   is one object; `Do` paints it N times. This is the single sharpest hazard in the whole
   level-navigation design and it must be a measured, named refusal — not a surprise.
   §1.4, §7.

4. **`NumberFormat::inch_fraction` is dead capability.** It is implemented, documented with
   runnable examples (`units.rs:197-206`), spec-mirrored into `/Measure` (`measure_dict.rs`
   has a test for it), and **constructed from nowhere outside tests**. Neither the GUI nor
   `pdfce-cli` can produce it. The operator asking for "fraction" is asking for something
   pdfce already does and cannot be told to do. This is R83's *inverse* and deserves its
   own rule. §6.

5. **`DimensionKind` is documented as "The **immutable** geometry" (`group.rs:163`).**
   Re-measure makes that comment false. It must change in the same commit — R93's shape
   (a comment asserting a behavior is not evidence of it) applied in reverse.

**Where the operator's model does not fit PDF, stated plainly** (the engineer asked for
this over a design that quietly cannot be built):

- **"Three levels" is not a property of PDF.** It is a property of a *file*. A flattened
  CAD plot has **two** (objects, nodes) — there is nothing to descend into. A tagged Word
  export or a drawing built from nested blocks can have **four or five**. The design must
  be "descend one container per double-click until there is nothing left," with the
  terminal level always being nodes. §1.3.
- **A dimension has no level 2 and no nodes.** It is one baked `/AP` (022 §4.1). Its
  "next level down" is not lines and text — it is *the two measured points*, and reaching
  them is re-measure, not descent. §5.
- **"Nodes for the group visible" does not survive contact with a real CAD page.**
  `MAX_NODES` is 4,000,000 and a plotted drawing routinely carries tens of thousands of
  anchors. Showing every node of a 4,000-object group is not a rendering problem, it is an
  un-usable screen. A ceiling with disclosure is required. §4.6.

---

## 1. Q1 — What IS a "group" for level 1?

### 1.1 Decision

**Level 1 is the smallest *structural container* that encloses the object, where
"structural container" means exactly two things, taken as a union:**

| # | Container | Spec | In the model today | Real? |
|---|---|---|---|---|
| **C1** | A **form XObject invocation** (`Do` on a `/Subtype /Form`) | §8.10 | one opaque `ImageObject { source: Form }` | **Yes — a real container with its own content stream, `/BBox`, `/Matrix`, `/Resources`** |
| **C2** | A **balanced marked-content sequence** (`BDC`…`EMC`, `BMC`…`EMC`) | §14.6 | **discarded** (`decompose.rs:1695`; `interpret.rs:1188` ignores it too) | Yes — semantic, advisory, nestable |

**Plus one level-1 leaf that is not a container:** an **annotation** (decision 022's
`TargetId::Annot`). A dimension sits at level 1 and has no level 2 — §5.

### 1.2 Rejected, with reasons

**Rejected — `/OC` optional content groups as the grouping axis.** An OCG is a
**visibility** axis, not a containment axis. Three concrete reasons, not one:

1. Membership is many-to-many and cross-page. Two objects in the same OCG are not "one
   thing you clicked" — they may be on different pages.
2. pdfce already gives OCGs a UI with *different* semantics: the shipped per-group
   dimension-layer toggle (`toggle_dimension_layer`, Pass 12.M2). If an OCG were also a
   click-group, clicking one dimension would select every dimension in the group. That is
   not what was asked for and it would collide with a shipped feature.
3. Content-stream `/OC` (`/OC /MC0 BDC` … `EMC`) is a **deferred gap in both** decompose
   and the renderer (`annot.rs:114`: *"full content-stream BDC/EMC `/OC` stays deferred"*;
   decision 011 §2.4). Building grouping on it would silently re-scope that gap.

OCGs stay what they are: a layer axis, orthogonal to level.

**Rejected — pdfce-side spatial / heuristic grouping** (proximity clustering, same-style
runs, connected-stroke chains). This is the tempting answer for the flattened-CAD case and
it must be refused. Rule 4 ("fuzzy, never sneaky") permits a *reviewable hint the operator
accepts or overrides*; a hidden heuristic that silently changes what one click selects is
a silent auto-apply wearing a hint's clothes. It is also **unstable under editing** — move
one object and the clusters re-form, so the same double-click means different things
before and after an unrelated edit. If spatial grouping is ever wanted it must be an
*explicit, operator-invoked, reviewable* "group these" command that writes a real
container (a marked-content sequence pdfce authors), not an invisible inference. Named
here as a future capability, refused as an inference.

**Rejected — treating the annotation space as a container.** An annotation is a leaf that
happens to be composite. 022 §4.1 already settled this: *"a dimension is one thing, not a
bundle of lines and nodes."* Descending into `/AP` content would expose the leader, ticks,
arrowheads and label as separately deletable pieces of a measurement — the sneakiest
possible outcome (rule 4). Refused permanently, not deferred.

### 1.3 ★ What happens on a page with no containers at all — the honest answer

**Level 1 collapses to level 2, and the tool must say so.**

A flattened CAD export — SolidWorks/AutoCAD plot-to-PDF, Chrome print-to-PDF, most
scanner-derived vector output — is very often one long list of `re` / `m l l S` with **no
form XObjects and no marked content**. On such a page there is no group, and pretending
otherwise would be fabrication.

Three disclosures, all cheap, make this legible rather than mysterious:

1. **The selection readout names the level and the container kind.** Positive-terms
   phrasing, following the existing readout discipline:
   - with a container: `Group · form XObject "Fig1" · 47 object(s) — double-click to enter`
   - without: `Path · … — this object is not inside a group (already at object level)`
2. **A double-click with nothing to descend into is never a silent no-op.** It reports the
   fact once, in the same readout. (This is the ROADMAP Pass 12.M2c bug #4 shape —
   "post-second-click clicks are dead with no hint" — and repeating it here would be
   repeating a known defect class.)
3. **A headless answer exists before any GUI work**: `pdfce-cli object-list --tree` prints
   the container forest, so "does this page have groups at all?" is answerable from a
   script and provable in a test. §7.2.

**Consequence worth stating to the operator directly:** on the drawings pdfce is most
likely to be used on, the "click once → group, double-click → lines" step will frequently
have no first stage. That is the file's structure, not a pdfce limitation, and the readout
is what makes the difference visible.

### 1.4 ★ The form-XObject trap: one stream, N placements

`decompose.rs:2236` classifies `Do` → `XObjectShape::Form { bbox, matrix }` and emits one
object per **invocation**. But the invocations share **one content stream object**.

Therefore: **editing a path inside a form changes every placement of that form.** Delete a
line inside a title block placed on 12 pages and it vanishes from all 12. Move a node
inside a hatch pattern placed 40 times and all 40 move.

This is not a bug to fix; it is what a form XObject *is* (§8.10 — reuse is the entire
point of the construct). It must be:

- **measured**, not assumed — count the invocations of that stream across the document;
- **refused by name** when the count is > 1, before any byte is planned;
- **disclosed** when the count is 1 (`this form is placed once — edits affect only this
  placement`), so the operator learns the rule from the common case rather than from the
  refusal.

Named refusal: `FormStreamIsShared { form: ObjId, invocations: usize }`. Reachable
trivially (any repeated block), and owed a test that asserts it **fires** (R96).

Whether to offer a "make this placement independent" un-share (duplicate the stream,
rewrite the one `Do`) is an operator question — it deliberately breaks minimal-diff for
that object and inflates the file. §10 item 3.

---

## 2. Q2 — Does decompose need to stop flattening?

### 2.1 Decision

**No. `PageObjects.objects` stays a flat `Vec<VectorObject>` in paint order with its
index space byte-for-byte unchanged. Hierarchy is added as (a) contiguous *ranges* over
that list and (b) a *forest of content streams*, one flat list per stream.**

The load-bearing reason, and it is the same one 022 §2 alternative (e) documented:
`EditSession::vector_surgery` (`edit.rs:2222-2237`) decomposes the page content with the
content-only `decompose(&stream, IDENTITY, &resolver)` and indexes `model.objects` by the
caller's `object_index`; the GUI provider uses `decompose_page`, which produces the same
list in the same order. **They agree by construction, and that agreement is what makes
`object-move --object 2` and a GUI drag mean the same thing.** A tree re-parents the list
and breaks that agreement *silently* — the drag moves a different object. This is the
identical failure 022 refused, and it must be refused identically.

### 2.2 ★ The structural insight that makes this cheap

**A PDF page's structural hierarchy is a laminar family of contiguous intervals over paint
order, plus a forest of content streams.** Two facts, both checkable:

- A `BDC`…`EMC` sequence is by definition a **contiguous token range**, so the objects it
  encloses are a **contiguous index range** in paint order. Nesting produces nested ranges
  — never crossing ones (an unbalanced/crossing sequence is malformed and is counted, not
  honored).
- A form's contents live in a **different content stream**, so they were never in the
  page's index space to begin with. Giving them their own flat list is the *truth*, not a
  workaround — and it is exactly right for surgery, which must target that stream anyway.

So the tree and the flat list are the same data. The tree is a **view**; the list stays
canonical.

### 2.3 The concrete shape

```rust
/// A structural container on a page — a contiguous run of paint-order
/// objects (§14.6 marked content) or a separately-decomposed form
/// XObject stream (§8.10). NEVER a re-parenting of `objects`.
pub struct Container {
    pub kind: ContainerKind,
    /// Index into `PageObjects::containers`, or `None` for a top-level
    /// container. Laminar: a child's range is inside its parent's.
    pub parent: Option<usize>,
    pub page_bbox: Bounds,
}

pub enum ContainerKind {
    /// A balanced `BDC`/`BMC` … `EMC` sequence (§14.6). Its members are
    /// the contiguous paint-order range `members` in the SAME stream —
    /// the index space is unchanged.
    Marked { tag: Vec<u8>, members: std::ops::Range<usize> },
    /// A `Do` on a form XObject (§8.10). `object_index` is the existing
    /// opaque `ImageObject`'s index — UNCHANGED — and `contents` is the
    /// form's OWN decomposition, in its own index space, under
    /// `matrix × ctm`.
    Form { object_index: usize, stream: ObjId, invocations: usize, contents: Box<PageObjects> },
}
```

`PageObjects` gains **one** field (`containers: Vec<Container>`) and is constructed at
exactly **one** site (`decompose.rs:1158` — verified; every other mention is a test helper
or the type/impl). That is the whole additive cost to the type.

### 2.4 Blast radius — measured

| Consumer | Effect |
|---|---|
| `hit_test_point` / `hit_test_point_all` / `hit_test_rect` (`hit.rs:124/171/178`) | **none** — they operate on one flat list; a sub-model is queried by calling them on `contents` |
| `snap_candidates` (`snap.rs`) | **none** at top level. *Open question, §10 item 7: should snapping see inside forms? Today it cannot, which means you cannot dimension to a line inside a placed block — a real limitation the operator will hit.* |
| `centerline` derivation | none |
| `vector/edit.rs` `plan_move` / `plan_delete` / `plan_move_node` | **none** — they take `&PathObject` + the stream it came from. A form's paths come with the form's stream. The signature already generalizes. |
| `EditSession::move_object` / `delete_object` / `move_node` | **none for top-level.** Editing inside a form needs new session methods targeting the form's stream (and the §1.4 refusal). |
| `pdfce-cli` `object-list` / `object-move --object N` / `object-delete` / `node-move` | **none** — `--object N` keeps meaning top-level object N. New addressing is *additive* (`--in-form`, `--tree`). |
| GUI Objects panel, `display_row_for_target`, `describe_object`, `selection_readout` | rows gain indentation + a level; `TargetId::Content`'s **payload** grows to carry a stream path |
| `canvas.rs` substrate | **zero** beyond what 022 already does — see below |
| `PageObjects::page_bbox` | none |

### 2.5 ★ The substrate needs no change beyond 022's

022 §2 established the amended `TargetId` contract: *"the kind partition belongs to the
substrate; the payload inside each kind stays entirely the provider's."* A stream path is
**payload**. So `TargetId::Content(ContentPath)` where
`ContentPath = { stream: SmallVec<[u32; 2]>, index: u32 }` requires **no further
substrate work** — `canvas.rs` still only stores, compares, orders and prunes.

This is a real reduction in cost and it is a direct dividend of 022's enum choice over the
tagged-integer alternative it rejected. Worth recording: had 022 taken the tagged-`u64`
route, level navigation would now require re-litigating the handle type under pressure.

### 2.6 What must be reused, not re-derived

Form recursion needs a **cycle guard and a depth cap**. `pdfce-render` already has both —
`interpret.rs:156-157` (*"unbounded recursion … is caught by `Interpreter::active`'s cycle
set at any depth"*) plus `xobject_depth_overflows` in its diagnostics. Decompose must use
the **same** limits and the **same** cycle-set discipline, and `DecomposeDiagnostics` gains
the matching counters. Two independently-chosen depth caps would mean the renderer paints a
level the selector cannot reach — the paint/select asymmetry this whole record exists to
close, reintroduced at a different depth.

---

## 3. Q3 — Descend and ascend

### 3.1 Decision

**Descend: double-click, one level per double-click. Ascend: Escape (one level per press),
and click-outside (to the common ancestor). "Leave the current group level" goes into
`resolve_escape` between `CancelGesture` and `ExitTool`.**

### 3.2 The amended precedence chain

Today (`canvas.rs:354-368`, call site `main.rs:5190`):

1. `CancelGesture` — tool active with a discardable gesture
2. `ExitTool` — tool active
3. `ClearCanvasSelection` — substrate selection non-empty
4. `FallThroughToRailClear`

After this Pass **and** the in-flight navigation Pass:

| # | Outcome | Owner | Why here |
|---|---|---|---|
| 0 | `DismissContextMenu` | navigation Pass | A menu is an overlay above everything; Escape closes it and nothing else. |
| 1 | `CancelGesture` | shipped | unchanged |
| **2** | **`LeaveGroupLevel`** | **this Pass** | **new — pop one level, stay in the tool** |
| 3 | `ExitTool` | shipped | |
| 4 | `ClearCanvasSelection` | shipped | |
| 5 | `FallThroughToRailClear` | shipped | |

**Why 2 must be above 3.** With the Obj tool active and the operator two levels inside a
form, today's chain hits rule 2 and **exits the tool entirely**, discarding both the level
context and the tool. Escape's established meaning in this chain is *"undo the most recent
narrowing of context, most-recent first."* Depth is narrower than tool-activation, so it
unwinds first. This yields a property worth stating and testing:

> **Escape walks all the way out, one step per press, and never skips a step.**
> Esc → parent level, Esc → top level, Esc → view mode, Esc → clear selection, Esc → rail
> clear. Never two steps at once, never a jump.

### 3.3 ★ These two Passes do not fight — but the signature will

The slots are **disjoint**: context-menu dismissal is strictly above `CancelGesture`
(a menu is not part of a tool's gesture); `LeaveGroupLevel` is strictly between
`CancelGesture` and `ExitTool`. Semantically there is no conflict.

**The hazard is mechanical, not semantic.** Both Passes edit the same function, whose
signature is three positional `bool`s:

```rust
pub fn resolve_escape(tool_active: bool, gesture_discardable: bool, canvas_selection_nonempty: bool) -> EscapeOutcome
```

Two concurrent Passes each appending a `bool` produces
`resolve_escape(true, false, true, false, true)` at the call site — a shape where a
transposed argument compiles, passes type-check, and silently reorders Escape's
precedence. That is a silent-wrong-behavior merge hazard, and this project has a rule
family about exactly this class (R92, R96).

**Recommendation, and it should be done by whichever Pass lands first regardless of which
that is:** change the signature **once**, to a named-field context.

```rust
pub struct EscapeContext {
    pub context_menu_open: bool,   // navigation Pass
    pub tool_active: bool,
    pub gesture_discardable: bool,
    pub group_depth: usize,        // this Pass — 0 = top level
    pub canvas_selection_nonempty: bool,
}
pub fn resolve_escape(cx: &EscapeContext) -> EscapeOutcome
```

Then each Pass **adds a field**, the other Pass's call site keeps compiling, and a
transposition is a name error rather than a behavior change. Proposed as a standing rule
in §9.

### 3.4 Click-outside ascent

Inkscape's second ascent gesture, and the one the operator will reach for without being
told. Well-defined semantics:

- **Click on an object outside the current container** → ascend to the **nearest common
  ancestor** of the current level and the clicked object, and select the clicked object's
  descendant-of-that-ancestor. (Same as Inkscape's behavior; R61 — behavioral reference
  only, designed independently.)
- **Click on genuinely empty canvas** → ascend to top level and clear the selection.
- **Drag on empty canvas** → still a marquee. The substrate already distinguishes click
  from drag, so this is not a new discrimination.

Scoped into slice 23.2b, after Escape (23.2a), so the smaller half is arguable from tests
before the ambiguous half lands.

### 3.5 The breadcrumb is not decoration

The operator cannot navigate a hierarchy they cannot see. A status-line breadcrumb —
`Page › Form "Fig1" › Marked /Span › Path` — is simultaneously the disclosure (§1.3) and
the ascent affordance (clicking a crumb jumps to that level). On a page with no containers
it reads `Page › Path`, which is itself the answer to "why isn't double-click doing
anything."

`pdfce-ui-specialist` owns the visual design. This record owns only the requirement: **the
current level is always visible, and it is always clickable to ascend** (R84 — never
colour alone; a breadcrumb is text + separators, which satisfies it by construction).

---

## 4. Q4 — Node level

### 4.1 What exists today

`vector_edit_tool.rs`: `classify_drag` picks a node within `NODE_GRAB_TOLERANCE` (6.0 pt,
fixed page-space) or falls through to a whole-object move. `VectorDrag { object_index,
node: Option<usize>, start }`. **One** node, transiently, for the duration of one drag.
There is no node selection *set*, no multi-node operation, and no node delete.

`vector/edit.rs` already carries the node-level refusal vocabulary:
`RectangleNode` (an `re` corner has no independent operand), `ImplicitNode` (the reused
start of an `h`-reopened subpath), `MalformedOperand`, `DegenerateCtm`, `NotAPath`,
`ObjectOutOfRange`.

### 4.2 Decision — what is well-defined

| Operation | Well-defined? | Notes |
|---|---|---|
| Show anchors for **every selected path object** | **Yes** | `Subpath::anchors()` exists; purely presentational. **Needs a ceiling — §4.6.** |
| Node **selection set** (click / shift-click / marquee-over-nodes) | **Yes** | New GUI state only: `BTreeSet<(ContentPath, u32)>`. No core change. |
| Node **move, single** | **Shipped** | `plan_move_node` |
| Node **move, multi** (one delta, N nodes, possibly across objects) | **Yes** | **Must be ONE core plan** — `plan_move_nodes`. N sequential `move_node` calls would be N undo steps and N content-stream re-emissions, and would violate 022 §5.5's R107-shape (name the changed objects, prove the rest byte-verbatim). |
| Node **delete** | **Yes, within named bounds** | §4.3 |

### 4.3 Node delete — the well-defined cases

**Semantics: remove the anchor and join its two incident segments with a single straight
segment.** Deterministic, spec-trivial, and visibly the change the operator asked for.

| Case | Result |
|---|---|
| Interior node of an **open** subpath | The two incident segments become one `l` to the successor. |
| **First** anchor of an open subpath | The second anchor becomes the new `m` operand; its incoming segment is dropped. |
| **Last** anchor of an open subpath | The final segment is dropped. |
| Any node of a **closed** subpath | Interior semantics (a closed subpath has no endpoints). |
| Node whose incident segments are **curves** (`c`/`v`/`y`) | Same — replaced by one straight segment. **Disclosed** in the readout, because curvature is lost. |

### 4.4 Node delete — refused by name (R83 / R96)

Each of these is **reachable from an input that would otherwise succeed**, and each is
owed a test that asserts it **fires** — R96's whole point.

| Refusal | Fires when | Why refused |
|---|---|---|
| **`SubpathWouldDegenerate { subpath_index, closed, remaining, minimum }`** *(new)* | deleting would drop an **open** subpath below 2 anchors or a **closed** subpath below 3 | There is no right answer. A 1-anchor open subpath is a bare `m` that paints nothing but is still a selectable object; a 2-anchor closed subpath is a degenerate "closed line." **The operator's route to removing the whole thing already exists and is unambiguous: delete the object.** Trivially reachable — a two-point line is the commonest object on a CAD page. |
| **`RectangleNode`** *(existing, extended to delete)* | the node is a corner of an `re` | Stronger for delete than for move: an `re` has **no per-corner operand at all**, so there is nothing to remove. Removing the whole `re` is object-delete. |
| **`ImplicitNode`** *(existing, extended)* | the node is the reused start of an `h`-reopened subpath | No independent operand. |
| **`NotAPath`** *(existing)* | a text / image / form object | Node editing is path-only (decision 011 §2.1). |
| **`FormStreamIsShared { form, invocations }`** *(new, §1.4)* | the path lives in a form invoked more than once | Editing one placement is not expressible. |

**`DegenerateCtm` must NOT be raised for node delete.** Deletion needs no coordinate
transform, therefore no CTM inverse, therefore no such failure exists. Raising it anyway
would be a refusal that is not required — dishonest in the opposite direction from a
missing one, and it would be a refusal no test could reach honestly.

### 4.5 Refused as capabilities, by name, so nobody builds an affordance for them

- **Node insert** (add an anchor on a segment). Mathematically well-defined (de Casteljau
  subdivision preserves the curve exactly), but not asked for and not in scope. **No
  affordance** — no double-click-on-segment, no `+` cursor (R83).
- **Curve-preserving delete (refit).** Inkscape re-fits a Bézier through the survivors to
  approximate the original shape. That is a *fuzzy* operation producing geometry the
  operator did not draw, so under rule 4 it can only ship as a **reviewable preview with
  accept/reject** — a different, larger feature. Deferred by name; the straight-join
  semantics of §4.3 ship instead, disclosed.
- **Bézier handle (control-point) editing.** `Subpath::anchors()` deliberately excludes
  control points (`decompose.rs:232-234`: *"a snap target is an anchor, not a handle"*),
  and decision 011 lists handle editing as a named fast-follow. **Nodes ≠ handles.** The
  node-level readout must not imply handles exist.

### 4.6 ★ The ceiling "nodes for the group visible" needs

`MAX_NODES` is 4,000,000. A plotted mechanical drawing routinely carries tens of thousands
of anchors, and the operator's request is explicitly to show nodes **for a whole group**.
Rendering 40,000 handles is not slow — it is an unreadable screen and an unclickable one
(handles at 6 pt grab tolerance overlap into a solid mat).

**Required: a node-display ceiling with disclosure, never silent truncation.** Above the
ceiling, show **no** handles and say why:
`Node view off — this selection has 41,208 nodes (limit 2,000). Select fewer objects.`
Silently drawing the first 2,000 would be the worst outcome: the operator would conclude
the rest of the geometry has no nodes.

The ceiling value is a UI judgment (`pdfce-ui-specialist`) and an R86 item — only looking
proves where handles stop being distinguishable.

---

## 5. Q5 — Dimension re-measure

### 5.1 The reconciliation

022 §4.2's argument was: *"in a measurement tool, a drag that silently changes a reported
measurement is the single sneakiest thing the application could do."*

**That argument is about silence, not about capability.** Re-read it and the conclusion it
actually supports is the one it states next: *"If re-measure is offered, it must be offered
where the operator's mental model is 'I am measuring,' with the new value shown live before
commit."* The operator has now said re-measure is wanted. Nothing in 022's reasoning
resists that; what it resists is **mouse-up committing a new number**.

**Decision: re-measure is a first-class, disclosed, two-stage operation. It is never
committed by releasing a drag.**

### 5.2 Where it lives — both surfaces, one implementation

| Surface | What it offers |
|---|---|
| **Measure tool** | Owns the gesture. A selected dimension shows **endpoint grab handles** — an affordance that now has a capability, R83-clean. |
| **Obj tool** | Owns *selection of everything* (the operator's answer). Offers select, delete (022), and a **`Re-measure` verb** that activates the Measure tool with that dimension loaded and its handles live. It does **not** perform the geometry edit itself. |
| **`pdfce-cli`** | `dimension-set-points` — and this is the clean headless oracle. §7.3. |

**This is the precise sense in which "the Obj tool is for everything" is true and
buildable: it is universal at the *selection* layer, not at the *verb* layer.** Verb
ownership stays where the operator's mental model is. Stated this way, 022 §4.2 and the
operator's instruction are both satisfied without either being bent.

### 5.3 The gesture, stage by stage

1. Measure tool active, dimension selected → two endpoint handles drawn (R84: handle shape
   + weight, not colour alone).
2. Pointer down on a handle → **pending re-measure** begins. Snapping applies (12.M1),
   exactly as the original pick did.
3. During the drag the label shows **both** values live: `5.000 m → 6.250 m`. The leader
   previews at the new position.
4. **Mouse-up does NOT commit.** It leaves the pending state on screen with the existing
   Accept / Reject strip (the same two-stage shape the linear-dimension pick already uses
   — ROADMAP 12.M2c bug #4 confirms Accept/Reject is the established next action, and the
   `Accept reflow` / `Reject reflow` buttons are the shipped precedent).
5. **Accept** commits one `EditSession` command. **Reject** or **Escape** reverts
   (`CancelGesture`, chain slot 1) to the pre-drag geometry.

**What must be on screen before Accept can be pressed** (rule 4, non-negotiable):
old value, new value, the delta, and the **group and scale** the new value was computed
under. A re-measure under a mis-set scale that shows only the new number is the exact
failure 022 was guarding against.

### 5.4 Why this cannot be a transform — and what it costs

`author.rs` (`ap_form_dict`, ~321-336) sets `/BBox = /Rect` with an **identity `/Matrix`**,
and `author_dimension` draws every stroke in **absolute page coordinates**. So:

- Rewriting `/Rect` alone leaves the geometry drawn where it was, then anisotropically
  refits it under §12.5.5 step b. Wrong twice.
- The endpoint change must **re-run `author_dimension(kind', scale, format)`** and replace
  `/L`, `/Rect`, `/Contents`, and the `/AP` `/N` stream.

That is mechanically **the same machinery `set_group_scale` already runs per member**
(`edit.rs:6083-6130`), so the cost is a new entry point, not a new subsystem.

```rust
/// Re-author a dimension at new geometry. ONE undoable command:
/// the /AP /N stream, the annotation dict (/L, /Rect, /Contents),
/// and the /PieceInfo sidecar (DimensionRecord.kind) — never a
/// subset. Page content streams: ZERO.
pub fn set_dimension_geometry(&mut self, dim: DimensionId, kind: DimensionKind)
    -> Result<MeasurementDisplay, EditError>
```

Returning `MeasurementDisplay` rather than `()` means the committed value is a
**measurement returned by the operation**, not a number the GUI re-derives and hopes
matches — R98's shape (apply computes, then reports).

### 5.5 Two hazards this inherits, both already documented

1. **022 §5.3's corruption path.** `set_group_scale` pushes an `ObjectWrite` replacing each
   member's `/AP` **unconditionally**, while the annotation-dict half is guarded.
   `set_dimension_geometry` must use the guarded pattern and must write the sidecar in the
   **same** command — otherwise a re-measure followed by a scale change reintroduces
   022's orphan-`/AP` shape from a new direction. Cross-referenced, not re-derived.
2. **`group.rs:163` calls `DimensionKind` "The **immutable** geometry."** This ships false.
   The doc comment must change in the same commit. R93 in reverse: a comment asserting
   immutability adjacent to a method that mutates is precisely the rot the rule names.

### 5.6 The sibling that should ship with it: whole-dimension move

022 §4.3 established that translating both endpoints by the same delta leaves
`measured_points()` mathematically invariant — so a whole-dimension move is semantically
honest — but still cannot use the generic `/Rect`-rewrite path, for the §5.4 reason.

It is therefore **the same function** with `kind.translated(dx, dy)`, and it is what the
operator will actually reach for most often (nudging a dimension off the geometry it is
covering). Ship it in the same slice, with the honesty affordance that falls out for free:

> `Moved — measurement unchanged: 5.000 m`

showing the invariant *held*, rather than leaving the operator to trust that it did.

---

## 6. Q6 — Units and display type in the GUI

### 6.1 Verified state

| Fact | Evidence |
|---|---|
| Format is **per-group** | `group.rs:57` — `Group { scale, format: NumberFormat, ocg, visible, … }` |
| The GUI **never** lets the operator choose | `measure_tool.rs:425` — `commit()` returns `preview.unit.default_format()`. There is no other format-producing site in `pdfce-gui`. |
| Format can only be set **together with scale** | `EditSession::set_group_scale(group, scale, format)` (`edit.rs:6069`). There is **no** `set_group_format`. |
| The CLI exposes only `--precision` | `main.rs:7929-7934` — `Some(p) if unit == FeetInches => feet_inches(p, false)`, `Some(p) => decimal(unit, p)`, `None => unit.default_format()` |
| **`NumberFormat::inch_fraction` is constructed from nowhere outside tests** | grep across `crates/`: only `measure_dict.rs:331` (a test) — so `6 4/8 in` display is **unreachable from both surfaces** |
| **`FractionMode::Fraction { reduce: true }` likewise** | every `feet_inches(_, reduce)` call site passes `false` |

So the operator is right, and it is worse than they said: this is not only a missing GUI
surface, it is a **shipped capability reachable from nothing at all**. Core implements it,
documents it with runnable examples, and mirrors it into `/Measure` for §12.9-honoring
readers. No operator can ask for it.

### 6.2 Decision — where it belongs

**The Dimension-groups panel, on the group row. Not the scale-entry dialog.**

The scale-entry dialog is a **transient, gesture-scoped** surface: draw a reference line →
type a real length → Accept. Putting a *persistent display preference* there means the only
way to change precision is to **re-draw a scale line** — which is why `set_group_scale`
couples scale and format today. The coupling is an artifact of where the control landed,
not a design.

The groups panel already owns the persistent per-group state (scale, units, visibility,
active-group selection). Note: **the ROADMAP already claims format is there** (line 1940:
*"create group / set scale+units+format / toggle per-group layer visibility"*) — a
documented-but-unbuilt claim, worth flagging to the librarian independently of this slice.

### 6.3 Per-group or per-dimension?

**Keep it per-group. With an argument, not an assertion.**

A group **is** the scale-and-unit context. Conventionally, every dimension in one scale
context on an architectural drawing shares one format (all feet-inches to 1/8"). Splitting
format per-dimension costs three real things:

1. a per-dimension override field in `DimensionRecord` **and** a sidecar schema change
   (`sidecar.rs`);
2. a "this one differs from its group" disclosure on every affected dimension — without it,
   an override is invisible and a later group-format change appears to silently fail;
3. an answer to *"does a group format change stomp overrides?"* — and both answers are
   defensible, which is the signature of a question that should be asked rather than
   decided here.

**And there is already an escape hatch that costs nothing:** two groups can share a scale.
If the operator needs decimal inches alongside feet-inches at the same scale, that is two
groups. Surfaced as an operator question (§10 item 1) in case their workflow says
otherwise — if it does, the group model is wrong and that is worth knowing now.

### 6.4 The smallest honest slice

**Core.**
```rust
/// Set a group's display format WITHOUT touching its scale. Regenerates
/// every member's /AP + /Measure, rewrites the sidecar, ONE command.
/// Returns the number of members regenerated (a measurement, R98).
pub fn set_group_format(&mut self, group: GroupId, format: NumberFormat)
    -> Result<usize, EditError>
```
Same guarded-write discipline as §5.5 item 1. Zero page content streams change.

**CLI (rule 11, same Pass).**
```
group-set-format <pdf> --group N [--unit mm|cm|m|in|ft|ft-in]
                       --style decimal|fraction
                       --places N          (style=decimal)
                       --denominator N     (style=fraction: 2|4|8|16|32|64)
                       [--reduce]
                       -o out.pdf [--verify-undo]
```
`--style fraction --unit in --denominator 8` is what finally makes `inch_fraction`
reachable; `--reduce` makes `FractionMode::Fraction { reduce: true }` reachable. **Both
dead capabilities are closed by argument surface, which costs nothing** — this is the
R83/R96 split 022 established, applied in the opposite direction (R96 governs the CLI: a
name must be reachable).

`group-set-scale`'s existing `--precision` stays as the *scale-entry default* and is
documented as such; format changes route to the new subcommand. One way to do each thing
(R92's spirit — a second path would drift).

**GUI.** On each group row in the Dimension-groups panel:
- unit selector (existing model, newly exposed);
- **style** selector: `Decimal` / `Fraction`;
- a precision-or-denominator field whose **label and legal values change with the style**
  (places 0–6 / denominator 2·4·8·16·32·64);
- **a live sample** of one of the group's real dimensions rendered in the *pending* format
  before Apply. `NumberFormat::format` is pure, so this is free — and it is the rule-4
  reviewable preview that makes the control honest rather than a guess-and-check.

R83 detail: when style = `Decimal`, the denominator field is **hidden, not greyed** — the
Pass 19.3 precedent (the free-rise field hidden rather than disabled, because *a greyed
spinner is still an affordance*).

`reduce` is **not** exposed in the GUI (nobody asked; adding an unrequested checkbox is
its own noise). It is disclosed instead — *"denominators are kept: 6/8, not 3/4"* — and it
is reachable from the CLI. §10 item 2 asks whether a GUI toggle is wanted.

### 6.5 ★ A shipped bug that would make this control look broken

**ROADMAP Pass 12.M2c bug #1: "Ratio scale entry silently overwrites the group's display
unit with the paper-basis unit."**

That bug is in the code path this slice touches. Sequence: operator sets feet-inches to
1/16 in the new panel → later draws a ratio scale → the unit silently resets → the format
control appears to have forgotten its setting. The operator would reasonably conclude the
new control is broken.

**Fix 12.M2c #1 in the same slice, or the new control is untrustworthy from day one.**
Named here because the coupling is invisible from either item alone.

---

## 7. Slice plan

Pass family **23** (family 22 is claimed by decision 022's prose). Four slices, ordered so
the two things the operator named explicitly land first and independently of the largest,
riskiest, weakest-oracle work.

### 7.1 Pass 23.0 — Format & units GUI surface (Q6)

**First, deliberately.** Smallest, highest operator value, **zero** hierarchy risk, clean
CLI oracle, no invariant risk, no dependency on 022.

- core `set_group_format`; CLI `group-set-format` with `--style`/`--denominator`/`--reduce`
- GUI group-row controls + live sample
- fix ROADMAP 12.M2c bug #1 (§6.5)

| # | Acceptance criterion | Check |
|---|---|---|
| A1 | The dead capability is reachable | `group-set-format --style fraction --unit in --denominator 8` then `dimension-list` prints `6 4/8 in` — a label string no prior build could produce |
| A2 | `reduce` is reachable | same, with `--reduce` → `6 1/2 in` |
| A3 | Format is decoupled from scale | `group-set-format` then `dimension-list` shows unchanged scale, changed labels |
| A4 | Zero content streams change | `tools/content-identity` reports **0** changed page content streams |
| A5 | Undo is byte-identical | `--verify-undo` |
| A6 | The 12.M2c #1 regression | set format → apply a ratio scale → `dimension-list` shows the format **retained** |
| A7 | R85 | `group-set-format` joins the preview-equals-saved operation list |

### 7.2 Pass 23.1 — Dimension re-measure + whole-dimension move (Q5)

Depends on **22.0** (the sidecar-pruning discipline and the guarded-write fix from 022
§5.3 must exist first, or this re-derives them).

- core `set_dimension_geometry`; fix `group.rs:163`'s "immutable" comment
- CLI `dimension-set-points` / `dimension-move`
- GUI: Measure-tool endpoint handles, two-stage pending + Accept/Reject, old→new readout
- Obj tool: a `Re-measure` verb that hands off to the Measure tool

| # | Acceptance criterion | Check |
|---|---|---|
| B1 | The measurement actually changes | `dimension-set-points --points 100,100,200,100` then `dimension-list` reports the new value; the **before** is measured, not asserted |
| B2 | A pure move leaves the value invariant | `dimension-move --dx 20 --dy 20` → `dimension-list` value **byte-identical**, `/Rect` moved |
| B3 | The §5.3 shape does not return | after re-measure, `group-set-scale` writes **no** `ObjectWrite` for a stale `/AP` and reports the correct member count |
| B4 | Zero content streams change | `tools/content-identity` = 0 |
| B5 | Undo byte-identical | `--verify-undo` on both |
| B6 | R85 | both join the oracle list |
| B7 | **R86 — must be watched** | that mouse-up does **not** commit, and that the old→new readout is legible mid-drag. A test can prove the command is not issued; only looking proves the strip reads as a decision point. |

### 7.3 Pass 23.2 — Level navigation (Q1 / Q2 / Q3)

Depends on **22.0** (`TargetId` enum). **Read-only** — no content is written, which is a
real strength of splitting navigation from editing and should be stated as such.

- **23.2a**: core containers (marked-content ranges + form sub-models, reusing the
  renderer's cycle set and depth cap); CLI `object-list --tree` / `--hit x,y --level N`;
  provider payload → `ContentPath`; double-click descend; Escape ascend;
  `EscapeContext` signature change (§3.3); breadcrumb
- **23.2b**: click-outside ascent to the common ancestor

| # | Acceptance criterion | Check |
|---|---|---|
| C1 | Form contents become reachable | on a fixture with a `Do` on a form containing 3 paths, `object-list --tree` prints the container and its 3 children; today it prints one opaque object |
| C2 | **The paint/select asymmetry closes** | every object the renderer paints inside a form is reachable by `--hit … --level N`; asserted from **both** sides (the renderer's own recursion and the decomposition), one definition of the depth cap |
| C3 | Descent is a headless assertion | `object-list --hit x,y --level 0/1/2` returns container / child / node — descent is a pure function of (point, level, forest), so the marquee's no-oracle problem does **not** extend here |
| C4 | The flat index space is untouched | `object-move --object 2` on a form-bearing fixture moves the **same** object before and after; a golden test on `PageObjects.objects` |
| C5 | Escape walks out one step per press | unit test over `EscapeContext` covering all six slots, including the navigation Pass's slot 0 |
| C6 | No-container pages disclose | `object-list --tree` on a flattened fixture prints `containers=0`; the readout says "already at object level" |
| C7 | Cycle/depth safety | a self-referencing form fixture terminates; `DecomposeDiagnostics` counts the overflow rather than recursing |
| C8 | Invariants | `cargo tree -p pdfce-core -p pdfce-render` GUI-free; **zero** content bytes written by this slice |
| C9 | **R86 — must be watched** | whether the breadcrumb reads as navigation rather than decoration, and whether double-click-to-descend is discoverable at all without it |

### 7.4 Pass 23.3 — Node selection set, multi-node move, node delete (Q4)

Depends on 23.2 (level 3 is the level below level 2).

- core `plan_delete_node` + `plan_move_nodes` (plural, ONE command) + the §4.4 refusals
- CLI `node-delete` / `node-move --nodes n1,n2,…`
- GUI node selection set, node marquee, the §4.6 ceiling

| # | Acceptance criterion | Check |
|---|---|---|
| D1 | Node delete works and is minimal | only the two incident construction operators are respliced; byte-span assertion |
| D2 | **Every refusal fires** | one test per §4.4 refusal, each reached from an input that would otherwise succeed (R96) — especially `SubpathWouldDegenerate` on a two-point line and `FormStreamIsShared` on a twice-placed form |
| D3 | `DegenerateCtm` is **not** raised | a singular-CTM fixture deletes a node successfully — the refusal that must *not* exist |
| D4 | Multi-node move is one command | undo stack depth 1 after moving 5 nodes; one content-stream re-emission |
| D5 | The ceiling discloses | a >ceiling selection shows **no** handles and states the count and the limit; never a silent first-N |
| D6 | Undo byte-identical | `--verify-undo` |
| D7 | **R86 — must be watched** | handle density/legibility at the ceiling, and that a node marquee reads differently from an object marquee |

---

## 8. Amendments owed to decision 022

**Recorded here, not written into 022** (§0 — the directory is append-only). The librarian
should add a forward-reference from 022's ledger/`ARCHITECTURE.md` §12 entry to this
record.

| # | 022 says | Amendment |
|---|---|---|
| 1 | §9 item 1 — *"Is the Obj tool 'everything on the page' or 'page content only'?"* — open, 022 assumes the former | **ANSWERED: everything.** 022's assumption stands; the Dimensions-panel alternative is not taken. **Qualified:** "everything" is answered at the **selection** layer. Verb ownership is unchanged (§5.2). |
| 2 | §9 item 5 — re-measure *"deferred here … a product question worth asking"* | **ANSWERED: wanted.** Pass 23.1. |
| 3 | §4.2 — re-measure is *"explicitly not reachable through the Obj tool at any point"* | **NARROWED.** The prohibition was on the **silent gesture**, not the capability. The Obj tool must not *perform* re-measure as a drag; it **must** expose a route to it (§5.2). 022's own next sentence already contemplates this. |
| 4 | §3 verb table — `Annot` row `move (drag)` = *"gesture never starts"* | **Scoped to the Obj tool.** In the **Measure** tool a dimension has endpoint handles (23.1). A **re-measure** row is added, and a **move** row whose `Annot` cell is `set_dimension_geometry(kind.translated(..))`, not the generic `/Rect` rewrite (022 §4.3's own reasoning). |
| 5 | §6 — *"Move stays deferred"* | **Time-boxed to 22.0.** 23.1 un-defers it for pdfce-authored dimensions specifically. Foreign-annotation move remains deferred with 022 §4.4's §12.5.5 verification still owed. |
| 6 | §2 — `TargetId::Content(u64)`, payload is the provider's | **Payload grows to `ContentPath`** in 23.2. **No further substrate change** — and this is a dividend of 022 choosing the enum over the tagged integer (§2.5). |
| 7 | §8 proposed rule — *"a selectable kind carries its verb set in its type"* | **Strengthened.** The handle must also express **level**, or level becomes a runtime convention of exactly the class 022 §2(d) rejected. |
| 8 | §8 proposed rule — *"selection enumerates exactly what the renderer paints"* | **Second live violation found: form XObjects** (§0 finding 2). The rule's justification is stronger than 022 knew, and its first gate should cover both object spaces. |

---

## 9. Proposed standing rules

*Proposed; the librarian assigns numbers. Next free is **R111** per the live ceiling, but
re-read it at filing time — 022 proposes rules against the same ceiling.*

1. **A PDF page's structural hierarchy is a laminar interval family over paint order plus a
   forest of content streams — never a re-parented object list.** Any container model
   preserves each stream's flat paint-order index space byte-for-byte; hierarchy is an
   index over it. *(Protects the core/GUI index agreement 022 §2(e) identified, at the one
   moment most likely to break it.)*

2. **Editing inside a form XObject is refused while that form is invoked more than once,
   and the invocation count is measured, not assumed.** A shared content stream cannot be
   edited for one placement only; the refusal names the count.

3. **A shipped capability reachable from no surface is a defect of the same class as an
   affordance with no capability** — R83's inverse. Every constructor of an
   operator-visible formatting or behavior variant is reachable from at least one surface,
   or it is deleted. *(Sourced concretely: `NumberFormat::inch_fraction` is live,
   documented, spec-mirrored, tested — and un-askable-for.)*

4. **A display-format preference never rides the gesture that first set it.** Format lives
   on the persistent entity, is settable independently of that gesture, and shows a live
   sample of the operator's own data before Apply. *(Sourced from
   `set_group_scale(scale, format)` making precision changeable only by re-drawing a scale
   line.)*

5. **A geometry change to a measurement is two-stage and disclosed: old → new visible
   before commit, and mouse-up is never the commit.** *(This is 022 §4.2's principle,
   preserved as a rule rather than discarded now that the capability is wanted.)*

6. **A precedence chain resolved from booleans takes a named-field context, not positional
   arguments.** *(Methodology; may be judged too small for a number. `resolve_escape` is
   about to be edited by two concurrent Passes; a fourth and fifth positional `bool` makes
   a transposed argument compile and silently reorder Escape.)*

---

## 10. For the operator — not this consultant's call

1. **Per-group vs per-dimension format.** Recommended: per-group, with "make a second group
   at the same scale" as the escape hatch (§6.3). Confirm that fits — if your drawings mix
   formats inside one scale context, the group model is wrong and it is better to know now
   than after 23.0 ships.
2. **`reduce` (`3/4"` vs `6/8"`).** The GUI ships denominators-kept, disclosed; the CLI
   exposes `--reduce`. Want a GUI toggle?
3. **Form-XObject un-sharing.** A form placed 5 times shares one stream, so editing inside
   it changes all 5 (§1.4). Recommended: refuse by name. Do you also want a *"make this
   placement independent"* command? It duplicates the stream — deliberately breaking
   minimal-diff for that object and growing the file — so it is a real trade, not a freebie.
4. **Fixed three levels, or as deep as the file goes?** Recommended: as deep as the file
   goes, terminating at nodes (§1.3). Fixed-three would either refuse to enter a nested
   block or lie about where you are.
5. **Node delete between two curves → a straight segment, no refit** (§4.3/§4.5).
   Acceptable, or do you want a reviewable curve-refit later?
6. **R86 for 23.1/23.2/23.3.** Each slice has named watch-items (B7, C9, D7). Whether
   unwatched items block the Shipped record is your call, and R86 is itself still
   sign-off-pending (022 §9 item 4, ROADMAP open question (e)).
7. **Should snapping see inside form XObjects?** Today it cannot, which means **you cannot
   dimension to a line inside a placed block.** 23.2 makes the geometry *available*;
   whether the snap engine should consume it is a separate call with a real cost (a
   snap query over a deeply-nested page is much larger).
8. **Decision 022's §9 items 2 and 3 are still open** (widget-annotation refusal; R58's
   scope amendment). Neither is touched by this record; both still need you.

---

## 11. Risks to the two load-bearing invariants

**GUI-core separation.** Two live risks, in opposite directions.

- *Core logic drifting into the GUI:* the provider already owns "which page" and the
  coordinate map, so building the container forest there is the path of least resistance.
  It must not happen — container detection is content-stream parsing, i.e. core.
  `cargo tree -p pdfce-core -p pdfce-render` is the gate.
- *GUI state drifting into core:* the **current level / current container** is transient
  view state and must **not** enter `PageObjects`. `PageObjects` describes the document;
  where the operator is standing is the shell's business. Putting it in core would make the
  WASM fork inherit a UI mode.

**Round-trip / minimal-diff.**

- 23.0 and 23.1 regenerate `/AP` streams and annotation dicts — the same objects
  `set_group_scale` already touches. **Zero page content streams change**, which is a
  cheap, machine-checkable distinguishing claim (`tools/content-identity` = 0).
- **23.2 writes nothing at all.** Navigation is read-only. Splitting it from editing is
  what makes the largest slice the one with no round-trip risk.
- 23.3 is content surgery — R46's named exception, same family as 9c-min's `plan_delete`.
  The specific risk is respliced-too-much: `plan_move_node` already rewrites exactly one
  operator's operands, and `plan_delete_node` must splice only the two incident
  construction operators. A byte-span assertion, not a code review.
- **The sharp one is form aliasing** (§1.4): a correct, minimal, byte-perfect edit to a
  shared stream is still wrong for 11 of 12 placements. Minimal-diff discipline does not
  protect against it — only the measured refusal does.

**A third risk, not to an invariant but to the record.** 023's slices land on top of
022's, and 022 itself is not yet started. Building 23.x before 22.0 would force the
`TargetId` widening and the sidecar-pruning discipline to be re-derived under pressure by
whoever gets there first. **22.0 ships first, or 23.1/23.2 do not start.** 23.0 is the
exception — it depends on neither.

---

## 12. JSON

```json
{
  "decision_number": "023",
  "title": "The Obj tool is for everything: level navigation, node-level editing, dimension re-measure, and the missing format surface",
  "date": "2026-08-04",
  "amends": "022",
  "amendment_mechanism": "new append-only record + librarian forward-reference; 022 is NOT edited (docs/decisions/README.md)",
  "pass_family": 23,
  "confidence": "high",

  "headline": "The operator's model is buildable, but not as three fixed levels. A PDF page's structural hierarchy is a laminar family of contiguous intervals over paint order plus a forest of content streams, so the canonical flat object list must STAY flat and hierarchy is added as an index over it — which reduces the substrate cost to zero beyond what decision 022 already pays. Level 1 is the smallest structural container (form XObject invocation OR balanced marked-content sequence), annotations are level-1 leaves, and on a flattened CAD page level 1 collapses to level 2 and the tool says so. Re-measure is granted but re-framed: it is a two-stage disclosed operation showing old->new before an explicit Accept, owned by the Measure tool, ROUTED TO from the Obj tool — which resolves 022 Q3 without bending either it or the operator, because 'the Obj tool is for everything' is true at the SELECTION layer, not the verb layer. Units/display-type is not a missing capability but a capability reachable from nothing: NumberFormat::inch_fraction is implemented, documented, spec-mirrored and constructed only in tests.",

  "answers": {
    "Q1_what_is_a_group": {
      "decision": "A union of exactly two structural containers: (C1) a form XObject invocation (SS8.10), (C2) a balanced BDC/BMC..EMC marked-content sequence (SS14.6). Plus annotations as level-1 LEAVES (a dimension is one thing, no level 2).",
      "rejected": {
        "oc_layers": "An OCG is a VISIBILITY axis, not containment: many-to-many and cross-page; it already has different shipped semantics (per-group dimension-layer toggle); and content-stream /OC is a deferred gap in BOTH decompose and render (annot.rs:114, decision 011 SS2.4).",
        "spatial_heuristic_grouping": "Rule 4 permits reviewable hints, not silent inference that changes what a click selects. Also unstable under editing — the same double-click means different things before and after an unrelated edit. Allowed only as an explicit operator-invoked command that WRITES a real container.",
        "annotation_as_container": "022 SS4.1 settled it: descending into a dimension's /AP would expose leader/ticks/arrowheads/label as separately deletable pieces of a measurement. Refused permanently, not deferred."
      },
      "no_container_page": "Level 1 COLLAPSES to level 2. Flattened CAD exports commonly have zero containers. Disclosed three ways: (1) readout names the level and container kind, or says 'not inside a group (already at object level)'; (2) a double-click with nothing to descend into reports the fact, never a silent no-op; (3) `object-list --tree` answers it headlessly before any GUI work.",
      "depth_is_not_three": "Depth is a property of the FILE, not of PDF: a flattened plot has 2 levels, a nested-block or tagged export can have 4-5. Design is 'descend one container per double-click until nothing remains', terminal level always nodes.",
      "form_aliasing_trap": "A form's contents are ONE content stream painted N times. Editing inside it changes every placement. Must be MEASURED (count invocations), REFUSED BY NAME when >1 (FormStreamIsShared), and DISCLOSED when ==1."
    },

    "Q2_decompose_tree": {
      "decision": "NO. PageObjects.objects stays a flat Vec<VectorObject> in paint order with its index space byte-for-byte unchanged. Add (a) contiguous RANGES over that list for marked content and (b) a FOREST of content streams (one flat list per form stream).",
      "why": "EditSession::vector_surgery (edit.rs:2222-2237) indexes model.objects by the caller's object_index using the content-only decompose; the GUI provider uses decompose_page. They agree BY CONSTRUCTION and that agreement is what makes `object-move --object 2` and a GUI drag mean the same thing. A tree re-parents the list and breaks it SILENTLY — the identical failure 022 SS2(e) refused.",
      "structural_insight": "A BDC..EMC sequence is a contiguous token range, hence a contiguous index range; nesting is nested, never crossing. A form's contents live in a DIFFERENT stream and were never in the page index space. So tree and list are the same data; the tree is a VIEW.",
      "blast_radius": {
        "zero_change": ["hit_test_point", "hit_test_point_all", "hit_test_rect", "centerline", "vector/edit.rs planners", "EditSession move/delete/move_node (top level)", "CLI --object N semantics", "canvas.rs substrate", "PageObjects::page_bbox"],
        "additive": ["PageObjects gains ONE field; constructed at exactly ONE site (decompose.rs:1158, verified)", "CLI gains --tree / --in-form", "Objects panel gains indentation + level", "DecomposeDiagnostics gains cycle/depth counters"],
        "payload_only": "TargetId::Content payload grows u64 -> ContentPath { stream, index }. NO further substrate change — 022 already established the payload is the provider's. A dividend of 022 rejecting the tagged-integer option.",
        "open": "snap_candidates does not see inside forms today, so you cannot dimension to a line inside a placed block. Operator question 7."
      },
      "reuse_not_rederive": "Form recursion needs a cycle guard + depth cap. pdfce-render ALREADY has both (interpret.rs:47/156-157, xobject_depth_overflows). Two independently-chosen caps would mean the renderer paints a level the selector cannot reach."
    },

    "Q3_descend_ascend": {
      "descend": "double-click, one container per double-click",
      "ascend": ["Escape (one level per press)", "click-outside -> nearest common ancestor (23.2b)", "breadcrumb crumb click"],
      "escape_chain": [
        "0 DismissContextMenu (navigation Pass)",
        "1 CancelGesture (shipped)",
        "2 LeaveGroupLevel (THIS Pass — new)",
        "3 ExitTool (shipped)",
        "4 ClearCanvasSelection (shipped)",
        "5 FallThroughToRailClear (shipped)"
      ],
      "why_level_above_exit_tool": "Today, Escape two levels inside a form hits ExitTool and discards BOTH the level context and the tool. The chain's meaning is 'undo the most recent narrowing of context, most-recent first'; depth is narrower than tool-activation. Yields a testable property: Escape walks all the way out, one step per press, never skipping.",
      "do_the_two_passes_fight": "Semantically NO — the slots are disjoint. MECHANICALLY YES: two Passes each appending a bool to a 3-positional-bool signature produces resolve_escape(true,false,true,false,true), where a transposed argument compiles and silently reorders precedence. Fix: change the signature ONCE to EscapeContext { context_menu_open, tool_active, gesture_discardable, group_depth, canvas_selection_nonempty }, whichever Pass lands first.",
      "breadcrumb": "Required, not decoration: 'Page > Form \"Fig1\" > Marked /Span > Path'. It is simultaneously the level disclosure and the ascent affordance. On a container-free page it reads 'Page > Path', which is itself the answer to 'why isn't double-click doing anything'."
    },

    "Q4_node_level": {
      "well_defined": [
        "show anchors for every selected path object (Subpath::anchors exists) — WITH A CEILING",
        "node selection set (click / shift-click / marquee) — GUI state only, no core change",
        "node move single (SHIPPED: plan_move_node)",
        "node move multi — MUST be one core plan (plan_move_nodes); N sequential calls = N undo steps + N stream re-emissions",
        "node delete: remove the anchor, join the two incident segments with ONE straight segment"
      ],
      "delete_semantics": {
        "interior_open": "two incident segments -> one `l`",
        "first_anchor_open": "second anchor becomes the new `m` operand",
        "last_anchor_open": "final segment dropped",
        "closed_subpath": "interior semantics (no endpoints)",
        "curves": "same — replaced by a straight segment, DISCLOSED because curvature is lost"
      },
      "refused_by_name_reachable_and_tested": [
        "SubpathWouldDegenerate { subpath_index, closed, remaining, minimum } (NEW) — open<2 or closed<3 anchors. No right answer exists; the operator's unambiguous route is object-delete. Trivially reachable (a two-point line is the commonest CAD object).",
        "RectangleNode (existing, extended) — an `re` has NO per-corner operand, so there is nothing to remove. Stronger for delete than for move.",
        "ImplicitNode (existing, extended) — h-reopened start has no independent operand",
        "NotAPath (existing) — text/image/form",
        "FormStreamIsShared { form, invocations } (NEW) — editing one placement is not expressible"
      ],
      "must_NOT_be_raised": "DegenerateCtm. Deletion needs no coordinate transform, hence no CTM inverse, hence no such failure. Raising it would be a refusal no test could honestly reach.",
      "refused_as_capabilities_no_affordance": [
        "node insert (de Casteljau subdivision) — well-defined but not asked for; no `+` cursor, no double-click-on-segment (R83)",
        "curve-preserving refit on delete — a FUZZY operation producing geometry the operator did not draw; under rule 4 it can only ship as a reviewable accept/reject preview. Deferred by name.",
        "Bezier handle (control-point) editing — anchors() deliberately excludes control points (decompose.rs:232-234); nodes != handles; the readout must not imply handles exist"
      ],
      "ceiling": "MAX_NODES is 4,000,000 and a plotted drawing carries tens of thousands of anchors. Above a ceiling, show NO handles and state the count and the limit. Silently drawing the first N is the worst outcome — the operator would conclude the rest has no nodes. Ceiling value is a ui-specialist + R86 call."
    },

    "Q5_dimension_remeasure": {
      "reconciliation": "022 SS4.2's argument is about SILENCE, not capability — its own next sentence already contemplates a disclosed re-measure. Granted, re-framed: first-class, two-stage, disclosed; NEVER committed by mouse-up.",
      "ownership": {
        "measure_tool": "owns the gesture — endpoint grab handles on a selected dimension (an affordance that now HAS a capability, R83-clean)",
        "obj_tool": "owns universal SELECTION; offers select + delete (022) + a `Re-measure` verb that hands off to the Measure tool. Does not perform the geometry edit.",
        "cli": "dimension-set-points / dimension-move — the clean headless oracle"
      },
      "the_key_sentence": "'The Obj tool is for everything' is true at the SELECTION layer, not the verb layer. That is what satisfies both 022 SS4.2 and the operator without bending either.",
      "gesture": ["handles drawn", "pointer-down begins a PENDING re-measure with snapping", "label shows BOTH values live (5.000 m -> 6.250 m)", "mouse-up does NOT commit — Accept/Reject strip (the shipped linear-pick precedent)", "Accept = one EditSession command; Reject/Escape reverts via CancelGesture"],
      "must_be_visible_before_accept": ["old value", "new value", "delta", "the GROUP and SCALE the new value was computed under"],
      "why_not_a_transform": "author.rs ap_form_dict sets /BBox = /Rect with an IDENTITY /Matrix and author_dimension draws in ABSOLUTE PAGE COORDINATES. A /Rect rewrite leaves geometry where it was and then anisotropically refits it under SS12.5.5 step b — wrong twice. The endpoint change must RE-RUN author_dimension and replace /L, /Rect, /Contents and the /AP /N stream — the same machinery set_group_scale already runs per member (edit.rs:6083-6130).",
      "new_api": "EditSession::set_dimension_geometry(dim: DimensionId, kind: DimensionKind) -> Result<MeasurementDisplay, EditError>. Returns the display so the committed value is a MEASUREMENT returned by the operation, not a number the GUI re-derives and hopes matches (R98's shape).",
      "inherited_hazards": [
        "022 SS5.3 — set_group_scale pushes an /AP ObjectWrite UNCONDITIONALLY while the annot-dict half is guarded. set_dimension_geometry must use the guarded pattern AND write the sidecar in the same command.",
        "group.rs:163 documents DimensionKind as 'The immutable geometry'. This ships that comment false. Change it in the same commit — R93 in reverse."
      ],
      "ship_alongside": "Whole-dimension MOVE — the same function with kind.translated(dx,dy). 022 SS4.3 already proved it is semantically honest (measured_points invariant) but cannot use the generic /Rect path. It is what the operator will reach for most. Discloses 'Moved — measurement unchanged: 5.000 m', showing the invariant HELD rather than asking for trust."
    },

    "Q6_units_and_display": {
      "verified_gap": "Not a missing capability — a capability reachable from NOTHING. measure_tool.rs:425 commits preview.unit.default_format(); there is no other format-producing site in pdfce-gui. set_group_scale(group, scale, format) is the ONLY setter, so format cannot be changed without re-entering scale. NumberFormat::inch_fraction is constructed only in a test (measure_dict.rs:331). FractionMode::Fraction{reduce:true} likewise never constructed.",
      "where_it_belongs": "The Dimension-groups panel, on the group row. NOT the scale-entry dialog — that is a transient gesture-scoped surface, so putting a persistent display preference there means the only way to change precision is to RE-DRAW A SCALE LINE. That coupling is an artifact of placement, not design. Note: ROADMAP:1940 ALREADY CLAIMS format is in the groups panel — a documented-but-unbuilt claim worth flagging independently.",
      "per_group_or_per_dimension": "Per-GROUP, kept. A group IS the scale-and-unit context. Per-dimension costs: a DimensionRecord override field + sidecar schema change; a 'differs from its group' disclosure on every override (without it a later group change appears to silently fail); and an answer to 'does a group format change stomp overrides?' where both answers are defensible — the signature of a question to ASK. Escape hatch that costs nothing: two groups can share a scale.",
      "smallest_slice": {
        "core": "EditSession::set_group_format(group, format) -> Result<usize, EditError> — decoupled from scale, regenerates member /AP + /Measure, rewrites the sidecar, ONE command, returns members regenerated (a measurement, R98)",
        "cli": "group-set-format --group N [--unit U] --style decimal|fraction [--places N|--denominator N] [--reduce] -o out.pdf [--verify-undo]. --style fraction makes inch_fraction reachable; --reduce makes Fraction{reduce:true} reachable. BOTH dead capabilities closed by argument surface, which costs nothing (R96 governs the CLI).",
        "gui": "group row: unit selector, STYLE selector (Decimal/Fraction), a precision-or-denominator field whose LABEL AND LEGAL VALUES change with the style, and a LIVE SAMPLE of one of the group's real dimensions in the pending format before Apply (NumberFormat::format is pure, so it is free — and it is the rule-4 reviewable preview).",
        "r83_detail": "When style=Decimal the denominator field is HIDDEN, not greyed — the Pass 19.3 precedent (a greyed spinner is still an affordance).",
        "reduce_posture": "Not exposed in the GUI (nobody asked); disclosed instead ('denominators are kept: 6/8, not 3/4'); reachable from the CLI. Same R83/R96 split 022 established, applied in the opposite direction."
      },
      "coupled_bug": "ROADMAP Pass 12.M2c bug #1 — ratio scale entry SILENTLY OVERWRITES the group's display unit with the paper-basis unit. Same code path. Sequence: set feet-inches 1/16 -> draw a ratio scale -> unit resets -> the new control looks broken. FIX IT IN THE SAME SLICE or the control is untrustworthy from day one."
    }
  },

  "slice_plan": [
    {
      "id": "23.0",
      "name": "Format & units GUI surface",
      "depends_on": [],
      "why_first": "smallest, highest operator value, ZERO hierarchy risk, clean CLI oracle, no invariant risk, independent of 022",
      "acceptance": ["A1 group-set-format --style fraction --unit in --denominator 8 -> dimension-list prints `6 4/8 in`, a label no prior build could produce", "A2 --reduce -> `6 1/2 in`", "A3 format decoupled from scale", "A4 tools/content-identity = 0 changed content streams", "A5 --verify-undo byte-identical", "A6 12.M2c #1 regression: format retained across a ratio scale entry", "A7 R85 oracle list"]
    },
    {
      "id": "23.1",
      "name": "Dimension re-measure + whole-dimension move",
      "depends_on": ["22.0"],
      "acceptance": ["B1 dimension-set-points changes the value, before measured not asserted", "B2 dimension-move leaves the value byte-identical, /Rect moved", "B3 no 022 SS5.3 orphan /AP after re-measure + group-set-scale", "B4 content-identity = 0", "B5 --verify-undo both", "B6 R85", "B7 R86 WATCH: mouse-up does not commit; old->new readout legible mid-drag"]
    },
    {
      "id": "23.2",
      "name": "Level navigation (containers, descend/ascend, breadcrumb)",
      "depends_on": ["22.0"],
      "note": "READ-ONLY — writes nothing. Splitting navigation from editing puts the largest slice at zero round-trip risk.",
      "sub_slices": ["23.2a core containers + object-list --tree/--level + descend + Escape + EscapeContext + breadcrumb", "23.2b click-outside ascent to the common ancestor"],
      "acceptance": ["C1 object-list --tree prints a form's 3 children where today it prints 1 opaque object", "C2 the paint/select asymmetry closes — asserted from BOTH the renderer's recursion and the decomposition, ONE depth cap", "C3 object-list --hit x,y --level N makes DESCENT a headless assertion (descent is a pure function of point+level+forest, so the marquee's no-oracle problem does not extend here)", "C4 object-move --object 2 moves the SAME object before and after (golden test on the flat list)", "C5 Escape walks out one step per press across all six slots", "C6 container-free fixture prints containers=0 and the readout says 'already at object level'", "C7 self-referencing form terminates; the overflow is COUNTED", "C8 cargo tree GUI-free; ZERO content bytes written", "C9 R86 WATCH: is the breadcrumb read as navigation; is double-click-to-descend discoverable without it"]
    },
    {
      "id": "23.3",
      "name": "Node selection set, multi-node move, node delete",
      "depends_on": ["23.2"],
      "acceptance": ["D1 only the two incident construction operators are respliced (byte-span assertion)", "D2 EVERY SS4.4 refusal FIRES from an input that would otherwise succeed (R96) — especially SubpathWouldDegenerate on a two-point line and FormStreamIsShared on a twice-placed form", "D3 DegenerateCtm is NOT raised — a singular-CTM fixture deletes successfully", "D4 5-node move = undo depth 1, one re-emission", "D5 the ceiling discloses count + limit, never a silent first-N", "D6 --verify-undo", "D7 R86 WATCH: handle density at the ceiling; node marquee distinguishable from object marquee"]
    }
  ],

  "headless_oracle": {
    "problem": "022 recorded that the marquee has NO headless oracle; a hierarchy makes that worse.",
    "answer": "object-list --tree (the container forest, per-level indices) + object-list --hit x,y --level N (what a click at level N selects) + object-list --enclose x0,y0,x1,y1 (022's proposal). --level is the load-bearing one: DESCENT is a pure function of (point, level, container forest), so it is fully assertable headlessly even though the marquee's visual reading is not. Re-measure and format both have clean oracles already (dimension-list before/after + --verify-undo)."
  },

  "amendments_to_022": [
    "SS9 item 1 ANSWERED: the Obj tool is everything — but at the SELECTION layer only; verb ownership unchanged",
    "SS9 item 5 ANSWERED: re-measure is wanted (Pass 23.1)",
    "SS4.2 NARROWED: the prohibition was on the silent gesture, not the capability; the Obj tool must expose a ROUTE to re-measure",
    "SS3 verb table: the Annot 'gesture never starts' cell is scoped to the OBJ tool; Measure-tool endpoint handles are added; new re-measure and move rows",
    "SS6 'move stays deferred' time-boxed to 22.0; 23.1 un-defers it for pdfce-authored dimensions; foreign-annotation move still deferred with SS12.5.5 verification owed",
    "SS2 TargetId::Content payload grows to ContentPath — NO further substrate change; a dividend of 022 rejecting the tagged integer",
    "SS8 'a selectable kind carries its verb set in its type' STRENGTHENED: the handle must also express LEVEL or level becomes the runtime convention SS2(d) rejected",
    "SS8 'selection enumerates what the renderer paints' — SECOND live violation found: form XObjects. The renderer recurses (interpret.rs:47) and decompose emits one opaque object (decompose.rs:2236-2241)."
  ],

  "proposed_standing_rules": [
    "A PDF page's structural hierarchy is a laminar interval family over paint order plus a forest of content streams — never a re-parented object list. Each stream's flat index space is preserved byte-for-byte; hierarchy is an INDEX over it.",
    "Editing inside a form XObject is refused while that form is invoked more than once, and the invocation count is MEASURED, not assumed.",
    "A shipped capability reachable from no surface is a defect of the same class as an affordance with no capability (R83's inverse). Sourced from NumberFormat::inch_fraction being live, documented, spec-mirrored, tested, and un-askable-for.",
    "A display-format preference never rides the gesture that first set it; it lives on the persistent entity and shows a live sample of the operator's own data before Apply.",
    "A geometry change to a measurement is two-stage and disclosed: old -> new visible before commit, and mouse-up is never the commit. (022 SS4.2's principle preserved as a rule now that the capability is wanted.)",
    "A precedence chain resolved from booleans takes a named-field context, not positional arguments. (Methodology; may be too small for a number.)"
  ],

  "risks": {
    "gui_core_separation": [
      "core logic drifting INTO the GUI: building the container forest in the provider is the path of least resistance and must not happen (cargo tree is the gate)",
      "GUI state drifting INTO core: current level / current container is transient view state and must NOT enter PageObjects, or the WASM fork inherits a UI mode"
    ],
    "round_trip_minimal_diff": [
      "23.0/23.1 touch the same objects set_group_scale already touches; ZERO page content streams change (content-identity = 0)",
      "23.2 writes nothing at all",
      "23.3 is R46-exception surgery; the risk is respliced-too-much — a byte-span assertion, not a code review",
      "SHARPEST: form aliasing. A correct, minimal, byte-perfect edit to a shared stream is still wrong for 11 of 12 placements. Minimal-diff discipline does not protect against it; only the measured refusal does."
    ],
    "record": "22.0 must ship before 23.1/23.2 start, or the TargetId widening and the sidecar-pruning discipline get re-derived under pressure. 23.0 is the exception — it depends on neither."
  },

  "for_the_operator": [
    "Per-group vs per-dimension format — recommended per-group with 'two groups can share a scale' as the escape hatch. If your drawings mix formats inside one scale context, the group model is wrong and it is better to know before 23.0 ships.",
    "reduce (3/4\" vs 6/8\") — GUI ships denominators-kept + disclosed, CLI exposes --reduce. Want a GUI toggle?",
    "Form un-sharing — a form placed 5 times shares one stream. Refuse by name (recommended), or also offer 'make this placement independent'? The latter duplicates the stream, deliberately breaking minimal-diff for that object.",
    "Fixed three levels or as deep as the file goes? Recommended: as deep as it goes, terminating at nodes.",
    "Node delete between two curves -> a straight segment, no refit. Acceptable, or do you want a reviewable curve-refit later?",
    "R86 watch-items B7 / C9 / D7 — whether unwatched items block the Shipped record is your call (R86 is itself still sign-off-pending, ROADMAP question (e)).",
    "Should snapping see inside form XObjects? Today it cannot — so you cannot dimension to a line inside a placed block. 23.2 makes the geometry available; consuming it is a separate call with a real cost.",
    "022's SS9 items 2 and 3 (widget-annotation refusal; R58's scope amendment) remain open and untouched by this record."
  ]
}
```
