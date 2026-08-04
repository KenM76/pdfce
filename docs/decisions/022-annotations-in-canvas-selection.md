# Decision 022 — Annotations in canvas selection (dimensions cannot be selected)

**Status:** Decided (consultant recommendation; engineer to schedule, librarian to file)
**Date:** 2026-08-04
**Requested by:** pdfce-engineer
**Decision number:** **022** — verified against the live ceiling, not assumed.
`python tools/check-ledger-numbers.py --stats` reports
`decision records: 021 -> next free is 022`. Same run: `standing rules: R110
-> next free is R111`; `Pass families with headings: up to 21 (highest ID
21.1)`, `CLAIMED BUT NOT YET HEADED: 5, 9, 9c, 10, 13, 20`, so the next free
Pass family is **22**. **Re-run the checker at filing time** — the second
amendment to the numbering rule exists precisely because a family is claimed
in prose before it is headed, and two concurrent sessions have collided on
this project before.

---

## 0. Summary

The engineer's root-cause hypothesis is **correct, and confirmed empirically
rather than by inspection alone.** The GUI's selectable-object model is
`decompose_page` → `ContentStream::from_page`, i.e. page content streams
only. pdfce's dimensions are `/Line` annotations in `/Annots`. Annotations
are a parallel object space the model never enumerates, so they are invisible
to `hit_test`, `hit_test_all` and `hit_test_rect` alike. This is a scope gap
in the selection model, not a tolerance bug.

**The investigation also found a second, larger defect the operator has not
yet hit: there is no way to delete a dimension at all.** Not by canvas
selection, not by the Objects panel, not by any `pdfce-cli` subcommand, not by
any `EditSession` method. `EditSession` exposes `delete_object` (content-stream
surgery), `delete_redaction_mark` (redaction marks only) and `flatten_fields`
(form fields). There is no generic annotation removal. A dimension, once
authored, is permanent for the life of the document. That reframes the slice:
the honest fix for "I can't select my dimensions" is not select-only.

**And a third, latent corruption path** was found while reasoning about what
delete must do — see §5.3. Deleting a dimension's annotation without pruning
its `/PieceInfo` record causes a later `set_group_scale` to write a fresh
`/AP` stream at a *removed* object id, resurrecting an orphan appearance and
reporting a member count that includes a dimension that is not on the page.

---

## 1. Root cause — verified

### 1.1 By code

- `crates/pdfce-gui/src/object_provider.rs:107` —
  `ObjectModelProvider::build` calls `decompose_page(view, page, Matrix::IDENTITY)`.
- `crates/pdfce-core/src/vector/decompose.rs:1077` — `decompose_page` opens
  with `ContentStream::from_page(view, page)?` and never reads the page
  dictionary's `/Annots`.
- `grep -rn "Annots" crates/pdfce-core/src/vector/` returns **nothing**. The
  vector object model has no concept of an annotation.
- `crates/pdfce-core/src/dimension/author.rs:154-159` — a dimension is
  `/Type /Annot /Subtype /Line /IT /LineDimension` with a fully baked `/AP`.
  `EditSession::add_dimension` wires it into `/Annots`.
- `crates/pdfce-render/src/annot.rs:102` `survey_page_annotations` — called
  from `render/lib.rs:245` **after** page content is interpreted. So the
  dimension *is* painted. It is visible and unselectable, which is the exact
  asymmetry the operator is reporting.

### 1.2 By measurement (the CLI oracle, same code path as the GUI provider)

`pdfce-cli object-list` consumes the same `decompose_page`/`hit_test_point_all`
path the GUI provider does, so it is a legitimate headless stand-in where R86
cannot be satisfied. On the committed fixture
`fixtures/synthetic/dimension/linear-dim.pdf`:

```
$ pdfce-cli dimension-list fixtures/synthetic/dimension/linear-dim.pdf
dimension-list ... groups=1 dimensions=1
  group 0 "Default" unit=m scale=0.025 m/pt visible=true members=1
  dim 0 group=0 kind=linear value="5.000 m"

$ pdfce-cli object-list fixtures/synthetic/dimension/linear-dim.pdf --page 1
object page=1 index=0 kind=path bbox=100,200,300,200 subpaths=1 anchors=2 ...
object-list ... page=1 objects=1 paths=1 text=0 images=0 forms=0

$ pdfce-cli object-list fixtures/synthetic/dimension/linear-dim.pdf \
      --page 1 --hit 200,207 --all-hits
hit page=1 at=200,207 tolerance=3 index=none kind=none candidates=0
```

One dimension in the model; **`objects=1`** in the selectable model — the base
line only. A hit at `200,207` (the dimension's label band, clear of the base
line at `y=200`) returns **`candidates=0`**. This is the defect, reproduced
without the screen, and it is the before-half of the slice's acceptance
criterion.

`hit_test_rect` (the marquee the operator actually complained about) reads the
same `PageObjects`, so box-select misses for the same reason. Click-select in
the Obj / `VectorEdit` tool misses too — **the engineer's parenthetical
suspicion is confirmed.** `VectorEdit`'s click path is
`canvas::selection_after_click` over `provider.hit_test_all`, the same query
that returned `candidates=0` above.

---

## 2. Q1 — Should `TargetProvider` enumerate annotations?

### Decision

**Yes, through one composite provider, with `TargetId` widened from a `u64`
newtype into a two-variant enum** — option (a) and option (b) taken together,
because they answer different halves of the question. (a) fixes the *type*;
(b) describes the *provider*. They are not alternatives.

```rust
/// Opaque handle to a hit-testable thing on a page. The KIND partition is
/// the substrate's business (it is what makes verb dispatch exhaustive);
/// the payload inside each variant remains entirely the provider's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TargetId {
    /// Paint-order index into `PageObjects::objects` — unchanged meaning.
    Content(u64),
    /// The annotation object's own identity.
    Annot(ObjId),
}
```

`ObjId` already derives `Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd,
Ord` (`object.rs:61`), so `BTreeSet<TargetId>` and every ordering/liveness
comparison in `canvas.rs` keep working with a derive change and no logic
change.

### Blast radius — measured, not assumed

`TargetId` appears in three files. `canvas.rs` needs **zero semantic
changes**: it genuinely treats the value opaquely (stores, compares, orders,
prunes). The real cost is 10 sites in 2 files:

**Mint sites (5)** — all become `TargetId::Content(..)` or `::Annot(..)`:

| Site | What it mints from |
|---|---|
| `object_provider.rs:266` (`hit_test_all`) | paint-order index |
| `object_provider.rs:281` (`hit_test_rect`) | paint-order index |
| `main.rs:6510` (Objects panel row) | paint-order index |
| `main.rs:11211` (VectorEdit move preview) | `drag.object_index` |
| `main.rs:11464` (MeasureCircular fit outline) | fit-set index |

**Decode sites (5)** — each `.0` access becomes an exhaustive `match`:

| Site | Current code | Consequence of the change |
|---|---|---|
| `object_provider.rs:290` (`bounds`) | `usize::try_from(target.0)` | routes to content bbox or annot `/Rect` |
| `main.rs:2613` (`delete_selected_object`) | `t.0 as usize` | **the load-bearing one** — see Q2 |
| `main.rs:10676` (`selection_readout`) | `usize::try_from(target.0)` | needs an annotation summary |
| `main.rs:10908` (`display_row_for_target`) | `usize::try_from(target.0)` | annotation rows need their own row mapping |
| `main.rs:11086` (VectorEdit selected) | `t.0 as usize` | annotation must not enter the drag path |

Two doc-only references (`object_summary.rs:505`, `ui_text.rs:1980`) need
prose updates, no code.

### Why the enum, and not the cheaper options

**Rejected — (c) a separate selection channel for annotations.** Two
selection sets means two prune paths, two Escape semantics, two "what is
selected" readouts, two Delete owners, and an undefined answer to "what does a
marquee that covers both do?" `canvas.rs` was built explicitly as *one*
substrate (R60 — "exactly ONE canvas-interaction substrate"). A second channel
re-litigates R60 for a feature that does not need it. The one thing (c) buys —
annotation-specific affordances — is available inside a single set by matching
on the variant.

**Rejected — (d) tagged-integer partition of the existing `u64`** (e.g. high
bit set means annotation). Not offered by the engineer, considered and
rejected here so a future reader knows it was weighed: it keeps the type
unchanged, which sounds like a small diff and is actually the worst outcome.
`display_row_for_target` currently does
`usize::try_from(target.0).unwrap_or(last).min(last)` — under a tagged
integer, an annotation target *clamps to the last content row* and scrolls the
Objects panel to a plausible-looking wrong row. `delete_selected_object` would
pass a giant index into `delete_object`, which refuses with
`ObjectOutOfRange { index: 9223372036854775809, count: 3 }` — a refusal whose
message is nonsense. Every site that today "handles" an out-of-range id
gracefully becomes a site that handles an annotation *wrongly but quietly*.
The project's whole posture (rule 4, R83, R93) is against exactly this.

**Rejected — (e) inject annotations into `PageObjects` in core** (a
`VectorObject::Annot` variant), which is the tempting "fix it in one place"
move: `TargetId` unchanged, Objects panel/readout/describe_object all get
annotations for free. **This is the option that would have caused a silent
data-corruption bug, and the reason is worth recording.**
`EditSession::vector_surgery` (`edit.rs:2222-2237`) decomposes the base
content with `crate::vector::decompose(&stream, IDENTITY, &resolver)` — the
**content-only** path — and uses the caller's `object_index` to index
`model.objects`. The GUI provider uses `decompose_page`, which is the same
content-only object list (font resolution changes text *previews*, never the
object list or its order). The two agree by construction today, and that
agreement is the contract that makes `object-move --object 2` and a GUI drag
mean the same thing. Add annotations to `PageObjects` and the GUI's index
space diverges from the surgery's: a drag on object *n* would move a
*different* object. Prefixing vs. appending only changes which files break.
The invariant could be restored with a runtime "index ≥ content count is an
annotation" convention enforced at every core entry point — but that converts
a compile-time-guaranteed shared index space into a cross-crate convention
three functions have to remember. The enum makes the same distinction
unrepresentable-if-wrong instead, and leaves `PageObjects`, R46 byte-inertness
and the existing CLI `--object N` semantics untouched.

### Why `Annot(ObjId)` and not `Annot(index)`

An annotation has a document identity; a content object does not. Using
`ObjId` means an annotation target survives a provider rebuild that shifts
positions, so `prune_selection` drops only genuinely-removed annotations. It
also means the GUI hands `EditSession` exactly the handle its delete method
already takes (`delete_redaction_mark(annot_id: ObjId)`), with no translation
table to keep in sync. The content side lives with index fragility only
because content objects have no identity to use instead; there is no reason to
inherit that constraint where it does not apply.

### The one contract this amends, stated honestly

`canvas.rs`'s spec §4.1 says the substrate "never interprets this value" and
"the concrete representation is deliberately the provider's call." After this
change the substrate knows there are **two kinds**. The amended contract:
*the kind partition belongs to the substrate (that is what makes verb
dispatch exhaustive); the payload inside each kind stays entirely the
provider's.* The substrate still never indexes, decodes, or geometrically
interprets a payload. This should be written into the trait docs in the same
commit, not left for a future reader to infer from the enum.

---

## 3. Q2 — What may the operator DO with a selected annotation?

### Decision

**Verbs are polymorphic in principle, but this slice implements only the one
that exists for both kinds: DELETE. Move and drag-node are not "unimplemented
for annotations" — they are structurally absent from the annotation arm, so
there is no drag affordance to mislead anyone (R83), and no refusal string
that could rot into dead code (R96).**

| Verb | `Content(i)` | `Annot(id)` |
|---|---|---|
| select (click, Alt-cycle, marquee, panel row) | yes | **yes (new)** |
| delete | `delete_object` (content surgery) | **`delete_annotation` (new)** |
| move (drag) | `move_object` | **gesture never starts** |
| drag-node | `move_node` | **no nodes exposed** |

### Why this is R83-clean rather than R83-adjacent

The engineer's framing is exactly right: *a half-polymorphic verb set — select
works, move silently does nothing — is worse than not selecting at all.* The
enum is what prevents that, and it prevents it at compile time, not by
review. `main.rs:11086`'s
`doc.canvas_selection.iter().next().map(|t| t.0 as usize)` **stops compiling**.
The author must write a `match`, and the only honest `Annot` arm in this slice
is "do not enter the drag path." Consequences that follow, and must be
implemented as such:

- **No drag state is created** when the pointer goes down on an annotation with
  the Obj tool active. `vector_drag` stays `None`.
- **No move-preview rectangle** is painted (`main.rs:11211`'s branch is
  content-only).
- **No node handles / node-grab tolerance** apply — `classify_drag` is never
  reached.
- **The status readout says what the tool can do with this selection**, in
  positive terms ("Dimension … — Delete removes it"), not as an apology for a
  missing feature.

### The R83/R96 split, stated precisely

These two rules pull in opposite directions here and the resolution is a
surface split, not a compromise:

- **GUI → R83 governs.** The correct design is *no affordance*: the gesture
  never begins. Consequently there is **no named refusal on the GUI move
  path**, and there must not be — a refusal that can never fire is exactly
  R96's dead code that looks live.
- **CLI → R96 governs.** A CLI has no gesture to suppress; an argument can
  always name a wrong thing. Therefore every refusal the CLI names must be
  reachable and tested as firing. This slice's CLI names **three** refusals,
  all reachable (§6.2).

Naming this split is the substantive answer to "which, and why": the verb set
is polymorphic at the *type* level in both surfaces, but honesty is carried by
absence-of-affordance in the GUI and by named-reachable-refusal in the CLI.

---

## 4. Q3 — Dimensions are not just any annotation

### Decision

**The Measure tool owns dimension geometry. The generic vector tool offers
select + delete only. And "delete a dimension" is a dimension-aware operation,
not a generic annotation removal that happens to hit one.**

### 4.1 What the operator's words should mean for a dimension

The request was "click on individual lines and nodes to move or delete them."
For a dimension the honest answer is: **a dimension is one thing, not a bundle
of lines and nodes.** Its leader, its two extension ticks, its two arrowheads
and its value label are a single baked appearance generated by
`author_dimension` as a pure function of `(kind, scale, format)`
(`author.rs:107`). There is no arrowhead to grab; there is no leader to delete
independently of the label. Selecting a dimension selects *the dimension*, and
deleting it takes the whole appearance and the model record with it.

Stated as the operator will experience it: **on ordinary drawing geometry, the
Obj tool does what they asked (individual lines, individual nodes). On a
dimension it selects the dimension as a unit.** That is not a limitation to
apologize for — it is what a dimension is.

### 4.2 Why node-editing a dimension must NOT be the generic tool's job

`DimensionKind::Linear { a, b, constraint }` is documented in
`group.rs:162` as "The **immutable** geometry." Moving `a` changes
`kind.measured_points()`, which changes `format_measurement(...)`, which
changes the label, `/Contents`, `/Rect` and the baked `/AP`.

**Moving a dimension's endpoint is a re-measure, not a translate** — the
engineer's framing, and it is correct. The consequence is stronger than
"belongs elsewhere": in a *measurement* tool, a drag that silently changes a
reported measurement is the single sneakiest thing the application could do
(rule 4, and the reason `ScaleState` is tri-state rather than
`Option<f64>` — this project already refuses to let a measurement state be
ambiguous). If re-measure is offered, it must be offered where the operator's
mental model is "I am measuring," with the new value shown live before commit,
which is precisely the Measure tool's existing two-point-pick gesture with its
pre-commit preview. Deferred to a Measure-tool slice, not implemented here,
and **explicitly not reachable through the Obj tool at any point.**

### 4.3 Why even a pure TRANSLATE of a dimension is not the generic annotation move

Worth recording because it is counter-intuitive. Translating both `a` and `b`
by the same delta leaves `measured_points()` mathematically invariant, so a
whole-dimension move is semantically honest. But it still cannot be done by
the generic `/Rect`-rewrite annotation move, because `ap_form_dict`
(`author.rs:321-336`) sets `/BBox` to the page-space `/Rect` with an identity
`/Matrix` and `author_dimension` draws every stroke in **absolute page
coordinates**. Moving `/Rect` alone would leave the geometry drawn where it
was and then anisotropically refit it under §12.5.5 step b. A dimension move
must therefore re-run `author_dimension` with translated geometry and replace
the `/AP` stream — mechanically the same machinery `set_group_scale` already
runs, but a different operation from a foreign annotation's move. Recorded so
a future slice does not implement "annotation move" and assume dimensions came
along for free.

### 4.4 The corresponding note for foreign annotations

For an annotation pdfce did not author (an Acrobat `/Line`, a sticky note), a
`/Rect`-only rewrite *is* the correct move — §12.5.5 step b recomputes the fit
matrix from the new `/Rect`, so an `/AP` whose `/BBox` matches the old
`/Rect`'s size translates cleanly. Where `/BBox` and `/Rect` differ in size the
move anisotropically rescales the appearance, which may or may not be what the
author intended. **Confirm this against `PDF_Spec` §12.5.5 before shipping any
annotation move** — it is not needed for this slice, and it is a real reason
move is deferred rather than "just one more `ObjectWrite`."

---

## 5. Q4 — Round-trip, R107, and the forced-full-rewrite family

### 5.1 Exactly which objects change on an annotation delete

Four, at most, and the fourth only for a pdfce dimension:

1. **The `/Annots` container.** `EditSession::remove_from_annots`
   (`edit.rs:4864`) already handles both shapes and already avoids the
   no-op write: when `/Annots` is an **indirect array**, only that array
   object is written and *the page dictionary is untouched*; when it is
   **inline**, the page dictionary is written and the array object does not
   exist. Never both. This discipline is shipped and reused verbatim.
2. **The annotation dictionary** — a `Removal`.
3. **The `/AP` `/N` stream object** — a `Removal`, resolved before any
   mutation (the `delete_redaction_mark` pattern, `edit.rs:3260-3270`).
4. **The catalog `/PieceInfo` sidecar** (dimensions only) — via the existing
   `catalog_dimension_write`, the same object `add_dimension` and
   `set_group_scale` already write.

Everything else is byte-verbatim. **Zero page content streams change** — this
is not content surgery at all, which is a cheap, machine-checkable
distinguishing claim (§6.3).

### 5.2 Page-dictionary modification is already the case for shipped operations

Confirmed, as the engineer asked. `delete_redaction_mark` (Pass 8, shipped)
does exactly this: it removes an annotation from `/Annots`, writes the page
dict or the shared array, and removes the annot plus its `/AP`. `add_markup`,
`add_text_annotation` and `add_dimension` all patch `/Annots` in the additive
direction. `delete_pages` and `reorder_pages` rewrite page-tree nodes. There
is nothing new here.

### 5.3 ★ The corruption path this decision closes (found during review)

`set_group_scale` (`edit.rs:6083-6130`) collects
`model.members(group).filter_map(|d| Some((d.kind, d.annot?, d.ap?)))` and then,
**for every member, unconditionally pushes an `ObjectWrite` replacing the
`/AP` stream object**. The annotation-dict half is guarded
(`if let Some(Object::Dict(old)) = self.value(*annot_id)`); the `/AP` stream
write is not.

So if an annotation is deleted generically while its `DimensionRecord`
survives in the sidecar, the next scale change **writes a fresh `/AP` stream
at a removed object id** — resurrecting an orphan appearance stream that no
`/Annots` entry references — and reports
`CommandKind::SetGroupScale { members: N }` counting a dimension that is not
on the page. `dimension-list` would also keep reporting it (a lie to the
operator, rule 4).

**Therefore: `delete_annotation` must prune the `DimensionRecord` and write
the sidecar in the same undoable command.** This is not a nicety; it is what
makes the operation correct. It is also the concrete, demonstrable reason
Q3's answer is "dimension-aware delete" rather than "generic annotation
delete that happens to work on dimensions."

### 5.4 Not a member of the forced-full-rewrite family — and the rule text that needs fixing

**Incremental save remains the default (R36/R70). This does not join
R35/R58/R67.** The reasoning is already established in the record, for a
different operation: R70 and `ARCHITECTURE.md` §5.11 hold that in-place text
editing is *not* a fourth forced-full-rewrite sibling because "editing is a
content CHANGE, not a removal," and that prior content surviving in history is
a disclosed, accepted consequence rather than a defect. The operative
distinction is not *removal vs. change* but **whose contract is
confidentiality**: R35/R58/R67 exist because a redaction/scrub/recovered-base
save that leaves the old bytes recoverable has failed at its stated purpose.
Deleting an annotation makes no confidentiality promise; its contract is "this
is no longer in the current revision," and the prior revision remaining
reachable is undo/version history working.

**But R58's literal text says "every removal/scrub operation," and that
wording is already wrong today.** `EditSession::delete_object` (Pass 9c-min,
shipped `76485b5`) removes visible page content under incremental save and was
never reconciled with R58 the way text editing was reconciled by R70/§5.11.
`delete_redaction_mark` is a second such case. This decision would be the
third. **Do not quietly make it the third.** §8 proposes the amendment; the
scope of a standing rule is not this consultant's to narrow.

### 5.5 R107's relationship

R107's own subject matter (FF-C only ever *adds* font resources) does not
transfer — this is by definition a removal. What transfers is its **shape**,
and it should be honored: R107 works because it names precisely which objects
are allocated and proves by an object-id-disjointness *test* that nothing else
is touched, rather than by a runtime guard that could not fire. The analogue
here is naming the ≤4 changed objects above and proving the remainder
byte-verbatim with the existing content-identity gate (§6.3) — a test, not a
guard.

---

## 6. Q5 — Scope and slicing

### Decision

The smallest honest slice is **select + delete**, and the engineer's instinct
is right — but not for the reason of minimalism. **Select-only is not
available as an option.** Once an annotation is selectable, the Delete key is
pressed on it. With `delete_selected_object` matching on the enum, the `Annot`
arm must do *something*, and the only alternatives are (a) implement delete,
or (b) make Delete visibly refuse. (b) costs nearly as much as (a) — a named,
reachable, tested refusal plus its UI string — and leaves the operator with a
dimension they still cannot remove by any means whatsoever. (a) is the same
work and closes the second defect. Move stays deferred for the reasons in Q3.

Proposed as **Pass 22.0**, three sub-slices, CLI before GUI so the GUI half is
arguable from a headless oracle where R86 cannot be met.

### 6.1 Pass 22.0a — core

- **`pdfce_core::annot::is_visible_on_screen(graph, &Annotation, &oc_off) -> bool`**
  — extracted from `render/annot.rs:125-145`'s existing sequence (`/Popup`
  skip → `flags.suppressed_on_screen()` → `oc_is_hidden`). `survey_page_annotations`
  is refactored to *call* it. **This is the load-bearing structural point of
  the whole slice:** selection must enumerate through the same predicate the
  renderer paints through, or the two drift and the operator can select
  something they cannot see (or vice versa) — the same class of defect as
  decision 011's Z2 two-decompositions warning, one object space over. One
  definition, two consumers.
- **`pdfce_core::annot::selectable_annotations(graph, page_id) -> Vec<Annotation>`**
  — `page_annotations` filtered by that predicate. GUI-free, in core, per
  invariant 2.
- **`EditSession::delete_annotation(annot_id: ObjId) -> Result<(), EditError>`**
  — modeled directly on `delete_redaction_mark`: locate through the
  **session** graph (never the base), resolve `/AP /N` before any mutation,
  `remove_from_annots`, push `Removal`s, one `CommandKind::DeleteAnnotation`,
  encryption + `check_certification` guards first, every refusal before any
  mutation. **Dimension-aware:** if the id is a `DimensionRecord.annot`, the
  record is removed from the model and `catalog_dimension_write` is included
  in the same command (§5.3).
- **`EditSession::delete_dimension(dim: DimensionId)`** — the dimension-native
  front door; resolves to its `annot` and delegates. Keeps the CLI addressable
  by the stable id `dimension-list` already prints, instead of forcing the
  operator to translate to an object number.

### 6.2 Pass 22.0b — CLI (rule 11, same Pass)

- **`annot-list <pdf> --page N`** — one line per annotation: index, `ObjId`,
  `/Subtype`, `/Rect`, whether visible (and if not, *which* rule hid it —
  flags vs. `/OC`), and whether it is a pdfce-owned dimension. This is the
  headless oracle for 22.0c and the disclosure surface the GUI's Objects panel
  will mirror.
- **`annot-delete <pdf> --annot <num> [--gen N] -o out.pdf [--verify-undo]`**
- **`dimension-delete <pdf> --dim <id> -o out.pdf [--verify-undo]`**
- **`dimension-list` widened to print `annot=<objid>`** so the two listings
  can be cross-referenced (today it prints neither the annot nor the ap id,
  which is why the §5.3 inconsistency would be invisible from the CLI).
- **`object-list` gains `--enclose x0,y0,x1,y1`** reporting `hit_test_rect`
  results — the marquee half currently has *no* headless oracle at all, which
  is a contributing reason this defect was reported by a human rather than
  caught by a gate.

**The three named CLI refusals, all reachable and each owed a test that
asserts it FIRES (R96):**

1. **`NotAnAnnotation { id }`** — the id is not in this page's `/Annots`.
2. **`AnnotationIsWidget { id }`** — `/Subtype /Widget`. Deleting a widget
   without its `/AcroForm /Fields` entry orphans the field; that is form
   surgery (decision 020's R100–R106 family), not annotation surgery.
   **Recommended refusal, but see §9 item 2 — this is an operator call.**
3. **`AnnotationIsPopup { id }`** — a `/Popup` belongs to its parent markup
   annotation (§12.5.6.14) and is deleted with it, never independently.

Each refusal must have a test that reaches it with a fixture that would
otherwise succeed — not merely a test that the error variant exists. R96's
whole point is that a correctly-written, correctly-wired gate can be
structurally unreachable.

### 6.3 Pass 22.0c — GUI

- `TargetId` → enum; trait doc amended per §2's contract note.
- `ObjectModelProvider` becomes composite: content objects (unchanged index
  space) plus `selectable_annotations`. `hit_test_all` returns annotations
  **before** content at a shared point, because annotations paint over content
  and z-order honesty demands it; a content object shadowed by a large
  dimension `/Rect` stays reachable through the existing Alt+click cycling,
  which is exactly what that mechanism was built for. `hit_test_rect`
  enclosure-tests the `/Rect` — exact, cheap, and with no shadowing question
  at all, which is why the operator's literal complaint (box-select) is the
  cleanest part of this to get right.
- `bounds()` returns the `/Rect` mapped through the existing
  `pdf_bounds_to_canvas`. **Annotations are bbox-only targets**, the same
  honest treatment text and image objects already get
  (`a_text_object_is_selectable_by_its_bbox`).
- Objects panel gains annotation rows; `display_row_for_target` grows a
  two-space row mapping. Selection readout describes annotations through
  **one** description path, extending `object_summary::describe_object`'s
  existing single-source discipline rather than adding a second describer.
- `delete_selected_object` matches on the variant. Drag start refuses to
  begin on an annotation (no state, no preview, no handles).
- R84: any new selected-state cue pairs colour with weight/shape.

### 6.4 Acceptance criteria

| # | Criterion | How it is checked |
|---|---|---|
| A1 | The measured before-state inverts | `object-list linear-dim.pdf --page 1 --hit 200,207 --all-hits` goes from `candidates=0` to a candidate naming the dimension annotation. **The "before" is measured in §1.2, not asserted.** |
| A2 | Marquee finds it | `object-list --enclose` over the dimension's `/Rect`, plus a `hit_test_rect` unit test in `object_provider.rs` with an annotation fixture |
| A3 | Selection matches the render exactly | A flag-hidden annotation and an `/OC`-OFF annotation (`ocg-hidden.pdf` already exists) are **both unpainted and unselectable**; one predicate, asserted from both sides |
| A4 | Zero content streams change | `tools/content-identity` reports **0** changed page content streams for `annot-delete` — the claim that distinguishes this from every prior delete |
| A5 | Undo is byte-identical | `--verify-undo` on both new subcommands |
| A6 | R85 | `annot-delete` / `dimension-delete` join the preview-equals-saved oracle's operation list |
| A7 | The model stays consistent | After `dimension-delete`, `dimension-list` reports `dimensions=N-1`, and a subsequent `group-set-scale` reports `members=N-1` **and writes no `ObjectWrite` for the removed `/AP`** — the direct regression test for §5.3 |
| A8 | Refusals fire | One test per §6.2 refusal, each reaching the gate from an input that would otherwise succeed (R96) |
| A9 | Invariants | `cargo tree -p pdfce-core -p pdfce-render` GUI-free; `fmt`/`clippy -D warnings`/`check-ui-strings.sh`/`check-ledger-numbers.py` clean; zero new dependencies expected |

### 6.5 What genuinely needs watching before it ships (R86 cannot be met now)

Most of this slice is arguable from code and tests. Three things are not, and
should be flagged to the operator as "watch these when you're back at the
machine" rather than quietly shipped as verified:

1. **Marquee over a mixed selection** — a box containing both drawing geometry
   and a dimension. Enclosure arithmetic is unit-testable; whether the
   resulting multi-select *reads* correctly (outline count matching the
   readout's census) is visual.
2. **The absence of drag affordance on an annotation.** A test can prove
   `vector_drag` stays `None`; only looking proves the cursor and the canvas
   do not imply otherwise. This is precisely the R86 rationale the project
   sharpened after the "Place point" button incident — a test proves a
   character has a glyph, only looking proves the operator can read the button.
3. **Z-order/shadowing in practice** — whether a long dimension's `/Rect`
   makes underlying geometry annoying to click even with Alt-cycling
   available. If it does, the remedy is a tighter annotation hit shape
   (`/L`-aware for `/Line`), which is a follow-up, not a redesign.

---

## 7. Risks to the two load-bearing invariants

**GUI-core separation** — the live risk is re-deriving annotation visibility
inside the GUI provider "just to avoid touching `pdfce-render`." That would
put spec logic in the shell crate *and* create a second definition that drifts
from the renderer's. Mitigation is §6.1: the predicate is extracted into
`pdfce-core::annot`, `pdfce-render` is refactored to consume it, and A3
asserts both consumers agree. Cost: a small, real refactor of shipped render
code — accepted deliberately, because the alternative is the exact divergence
class this project keeps writing rules about. `cargo tree` is the gate.

**Round-trip / minimal-diff** — the risk is writing the page dictionary when
`/Annots` is an indirect array (a no-op write that inflates the dirty set and
breaks the "only what changed" claim). Already solved and shipped in
`remove_from_annots`; the requirement is to reuse it rather than re-derive.
A4 is the proof. Secondary risk: forgetting the `/AP` `/N` removal, leaving an
orphan stream — `delete_redaction_mark`'s resolve-before-mutate pattern covers
it, and an object-count assertion should catch a regression.

**A third risk, not to an invariant but to the record** — shipping this
without the R58 amendment (§5.4) makes the third unreconciled removal-under-
incremental-save and leaves a standing rule whose literal text three shipped
operations violate.

---

## 8. Proposed standing rules

*Proposed; the librarian assigns numbers. Next free is **R111** per the live
ceiling, but re-read it at filing time.*

- **Selection enumerates exactly what the renderer paints.** Any object space
  the canvas paints must be hit-testable through the *same* visibility
  predicate the renderer uses, defined once in `pdfce-core`. A paint/hit-test
  asymmetry is a defect of the same class as decision 011's Z2 two-decompositions
  divergence — one object space over. **This rule's absence is the whole cause
  of this decision:** annotations were painted from Pass 6.0 and selectable
  from never, and no gate existed to notice.
- **A selectable kind carries its verb set in its type.** Handles like
  `TargetId` are an enum over kinds and every verb dispatch is an exhaustive
  match; a verb that does not apply to a kind is *unrepresentable*, never a
  silent no-op. The structural form of R83 — R83 forbids the affordance, this
  forbids the shape that makes the affordance easy to add by accident.
- **Deleting a pdfce-authored annotation prunes its `/PieceInfo` record in the
  same command.** The sidecar and the document never diverge across one
  undoable command, in either direction. Derived from a concrete corruption
  path (§5.3), not from principle.
- **Every new selection or gesture surface ships a headless oracle in the same
  Pass.** `hit_test` has had `object-list --hit` since Pass 9a; `hit_test_rect`
  has had nothing, and the marquee gap was found by a human. Rule 11's logic
  (each feature ships its CLI surface) applied to *verification* surfaces.

---

## 9. For the operator — not this consultant's call

1. **Is the Obj tool "everything on the page" or "page content only"?** This
   decision assumes the former. If the answer is the latter, the cheaper and
   arguably better first surface is a **Dimensions panel** listing every
   dimension with Delete — it fixes "I can't get rid of this dimension"
   without touching `TargetId`, canvas selection, or the Objects panel at all.
   It does *not* fix "I can't box-select dimensions," which is what was
   literally asked for. **Recommendation: canvas selection, as decided — but
   the panel is a legitimate cheaper answer if the real need is removal rather
   than selection, and it is worth one question before the work starts.**
2. **Widget (form-field) annotations: refuse by name, or cascade into form
   surgery?** §6.2 recommends refuse. Cascading touches decision 020's
   R100–R106 family and changes what "delete" means on a form. Not mine.
3. **R58's scope amendment (§5.4).** Narrowing "every removal/scrub" to
   confidentiality-contract removals is a standing-rule change. Three shipped
   operations already sit outside its literal text; the wording should be
   fixed rather than accumulating a fourth exception. Librarian + operator.
4. **R86 for 22.0c** — the three items in §6.5 need eyes. Whether that blocks
   the Shipped record is the operator's call, and R86 is itself still
   operator-sign-off-pending (open question (e)).
5. **Dimension re-measure (Q3/§4.2)** — deferred here. Whether it is wanted at
   all, and whether it belongs to the Measure tool as recommended, is a
   product question worth asking before a slice is scoped for it.
