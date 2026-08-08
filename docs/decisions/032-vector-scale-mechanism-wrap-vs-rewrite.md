# Decision 032 — The vector-scale mechanism: wrap in `cm` versus rewrite operands, and the companion-behaviour set that rides on the answer

**Status: ⚠ OPEN. NOT DECIDED. Awaiting the operator's ruling.**

This record **opens** the question, lays out the options and their
consequences, and states the project-rule tension that makes it a decision
rather than a default. **It rules exactly one thing** (§7, the rounded-corner
toggle, ruled by the operator at dispatch time) and deliberately rules nothing
else. A future dated amendment to this file records the ruling when it comes.

**Date opened:** 2026-08-07
**Opened by:** `pdfce-librarian`, on `pdfce-engineer`'s explicit dispatch.
**Blocks:** **`Pass 46.1`** (*A vector object can be resized*) — its
`ROADMAP.md` entry already says *"Do not start the core verb before that is
ruled; it determines the data model, not just the implementation."* This
record is that ruling's container.
**Does not block:** `Pass 46.0` (the `/Rect` family). §8 explains why — it is a
third back-end with no stake in this question.
**Sources the research:** the `transforms__*` bucket of
`D:\Dev\Rag-Specialized\Inkscape_Features\`, filled 0 → 4 files on 2026-08-07
by `pdfce-inkscape-librarian`. **This record CITES those files; it does not
copy them.** They live outside the repo by design (R61 — Inkscape is a
behavioural reference only, never a dependency or a code source), and a
roadmap that copied them would go stale against them silently.
**Grounded in:** `ROADMAP.md`'s `Pass 46.0–46.1` entry **§4** (*"FAMILY (b) —
resize is genuinely harder, and LINE WIDTH is why"*), which raised the two
mechanisms and their spec consequences and then said, correctly, *"Which
mechanism is right is an ENGINEER DECISION and is NOT made here."*
**Amends:** nothing. **Extends:** `ROADMAP.md` §4 by adding the companion-
behaviour set, the SVG→PDF non-transfers, and the constraint in §4 below that
`ROADMAP.md` §4 did not have.

---

## 0. Number verification (R106 — verified against the live ledger, not assumed)

`tools/check-ledger-numbers.py`, run immediately before this file was written,
printed *"decision records : 031 -> next free is 032"*. `git ls-files
'docs/decisions/*.md'` returns **32 files** = `001`–`031` plus `README.md`,
**no gaps**. **032 is therefore the next free number and this record spends
it.**

**Nothing else is minted by this record.** **No Pass ID** — `Pass 46.1`
already exists and this is its gate, not a new arc; the ceiling stays at
**Pass 46** (headings ceiling `46.0`, mentioned ceiling `46.2`, the two
disagreeing by design as the twenty-ninth filing documented). **No standing
rule** — the ceiling stays **R166**, **R167** next free, third consecutive
filing to leave it free.

---

## 1. The question, in one sentence

**When pdfce scales a vector object in a page content stream, does it wrap the
object in `q <a b c d e f> cm … Q`, or does it rewrite the object's
path-construction operands (`m l c v y re`) in place?**

Everything else in this record is a consequence of that answer, or a
constraint on it.

---

## 2. ★★ Why this is a decision record and not a default — it lands on project rule 3

**The Inkscape research surfaced the finding that makes this unavoidable:**

> **Wrap-in-`cm` versus rewrite-operands is the same trade-off as Inkscape's
> *Preserved* versus *Optimized* transform storage.** Same two choices, same
> trade-off, same irreversibility.
> — `transforms__bbox_basis_and_transform_storage.md`, § SVG→PDF mapping

That file states outright that this *"lands directly on a load-bearing pdfce
invariant"* and that **neither option is universally right**, and it flags the
choice to `pdfce-engineer` as needing an explicit `ARCHITECTURE.md` record
rather than a default that emerges from whichever is easier to code. This
record is that flag being honoured.

**Project rule 3 (round-trip / minimal-diff editing)** says objects pdfce did
not logically touch are re-emitted byte-identical or omitted. **The rule does
not settle this question, because both options touch the object** — they
differ in *how much of it* they touch and *what they leave behind*:

| | **Wrap in `cm`** (Preserved-like) | **Rewrite operands** (Optimized-like) |
|---|---|---|
| Diff size for the object | **Tiny** — original operands stay byte-identical; two lines added | **Largest possible** — every coordinate in the object changes |
| Rule-3 posture | Honours byte-identical re-emission of the operand text | **Destroys** byte-identical re-emission of the operand text |
| Artefact | **Introduces an editor-shaped artefact** into a stream pdfce did not otherwise touch | **None** — output looks like it was authored that way |
| Repeated edits | **Nests** — a `cm` per scale unless coalesced | Flat — no accumulation, ever |
| Inspectability | The operation is recorded and invertible at the file level | The operation is **gone**; only its result survives, irreversibly |
| Stroke | **Scales with the geometry** (§8.4.3.2 — line width is in user space), so *"don't scale the line weight"* needs the **compensating-`w` path with all its non-uniform problems** | **Does not scale** — exact stroke independence **for free**, no compensation, no division |

**The tension, stated plainly:** the option that is *best* for rule 3's diff
size is the option that is *worst* for stroke control and cleanliness, and
vice versa. **A project whose stated corpus is CAD output** — where hairlines
and stroke weight carry drafting meaning — **may reasonably weight stroke
exactness above diff size.** That is a judgement about what pdfce is for, not
about which code is easier, and it is the operator's to make.

---

## 3. The option set

Four options, not two. **None is recommended here.**

### Option A — Always wrap in `q … cm … Q`

- **For:** minimal diff; the operation stays inspectable and invertible; it is
  the *native* mechanism for image XObjects already (§8.9.5 — an image is
  drawn through its `cm`, so for images this is not a wrapper hack at all, and
  `ROADMAP.md` §4 already names images as the cheapest member on exactly that
  ground).
- **Against:** nests on repeated edits; introduces an artefact; **forces the
  compensating-`w` path**, which is where every documented Inkscape defect in
  this area lives (non-uniform distortion, filter/bbox miscomputation).
- **Note:** the `q`/`Q` balance discipline already recorded in `PDF_Spec` is a
  real-world failure mode, not theoretical, and applies to every emission.

### Option B — Always rewrite operands

- **For:** exact stroke independence *for free* — no compensating division, no
  retained matrix; no nesting; no artefact; the cleanest possible saved file.
  **pdfce has a structural advantage Inkscape lacks here**: the Pass-8.0
  surgery interpreter already rewrites path-construction operands in place, so
  this is not new machinery (`transforms__stroke_width_scaling_semantics.md`,
  § SVG→PDF mapping — *"Inkscape's compensating-`stroke-width` approach exists
  because SVG editors reach for the transform attribute first; pdfce should
  not inherit that shape"*).
- **Against:** **largest possible diff for the object**; irreversible (the
  pre-transform coordinates are unrecoverable); and — the caveat that bounds
  it — **operand rewriting is only clean while the object's own CTM is
  unshared.** If the path sits under an enclosing `cm` shared with other
  objects, a nested `q … cm … Q` is required anyway **and the compensating-`w`
  path returns.** So Option B is not actually always available.

### Option C — Hybrid, with a stated switch rule

Rewrite operands when the object's operand run is small and unshared; wrap in
`cm` when the object is large, shared, or already under a non-identity CTM.

- **For:** takes each option where it is strongest.
- **Against:** **the emission strategy becomes data-dependent and therefore
  hard for an operator to predict**, and rule 3's contract becomes conditional.
  If this is chosen, *the switch rule itself must be documented and stable* —
  `transforms__bbox_basis_and_transform_storage.md` offers this shape but
  explicitly labels it *"a starting point and not an Inkscape-sourced fact"*.

### Option D — Operator-facing choice

Expose the emission mode the way Inkscape exposes *Store transformation*, per
operation or as a document preference.

- **For:** does not guess.
- **Against:** **Inkscape's own version of this is a hidden global mode and it
  is the direct cause of its most-asked transform question** (see §11). A
  choice the operator cannot see at the moment it applies is worse than a
  default they can read about.

---

## 4. ★★ THE CONSTRAINT THAT BOUNDS WHAT PDFCE CAN OFFER AT ALL

**`vector-effect: non-scaling-stroke` has NO PDF EQUIVALENT.**

This is the single most important SVG→PDF divergence in the research, and it
is filed prominently because **it is the sentence that stops someone
promising it.**

- In SVG it is **the mechanism that actually solves the non-uniform case** —
  it moves stroke-outline generation out of the transformed space entirely
  rather than trying to cancel the transform with a scalar. SVG2, verbatim:
  stroke width becomes independent of the element's transforms *"including
  non-uniform scaling and shear transformations"*.
- **PDF's only device-space-referenced stroke width is `0 w`** — *"the
  thinnest line that can be rendered at device resolution: 1 device pixel
  wide"* (§8.4.3.2). **That is a hairline, not an arbitrary constant width.**
  There is no PDF construct that says *"stroke this at 2 pt regardless of the
  CTM."*
- **Therefore: pdfce can offer non-scaling-stroke as an EDITING BEHAVIOUR,
  never as a DOCUMENT PROPERTY.** Rewriting `w` at edit time is exactly the
  toggle and is fully available. **The saved file cannot carry the intent** —
  a later session, or any other tool, has no way to know the operator meant
  *"line weights are fixed here."*

**An engineer scoping this from Inkscape alone would assume the escape hatch
exists.** It does not. `transforms__stroke_width_scaling_semantics.md` records
it as a GAP with the spec citation.

**Consequence for the option set:** no option in §3 can be justified on the
grounds that *"we can always fall back to non-scaling-stroke."* There is no
fallback. The choice in §1 is the whole answer.

**Consequence already recorded as settled by the research, and carried here so
a future pass does not re-investigate:** *a persisted non-scaling-stroke
document property* is **`out_of_scope`** — not "we haven't looked", but "no
PDF construct exists."

---

## 5. ★★ THE UNSATISFIABLE CASE — non-uniform scale with stroke-scaling OFF

**No scalar stroke width can cancel a non-uniform matrix.** This is
arithmetic, not an implementation gap, and **it transfers from SVG to PDF
unchanged** because both formats put line width in user space and both
generate the stroke outline before applying the transform.

**The reference fails silently at it, twice over:**

- **Launchpad #1335376** was closed **Invalid** on exactly this ground —
  the transform is applied as a single matrix *after* stroking (correct
  behaviour), so non-uniform scaling *"cannot be replicated by adjusting
  stroke-width alone."* Declared correct-by-spec rather than fixable.
- **Inkscape 1.4 distorts without warning or refusal** (UX issue **#339**) —
  in the mode where the operator *explicitly asked* for constant line weight
  and did not get it. The distortion is live during the drag and persists
  after commit. Its own users file bugs about it.

**⚠ Under project rule 4 this is a place to be BETTER than the reference, not
equal to it.** A width pdfce *chose* — via any mean of `sx` and `sy` — is
**inferred state**, and rule 4 requires it be visible before it becomes
document state. **pdfce must not silently pick a fudge factor.**

Three honest answers exist, and choosing among them is part of what this
record is open for:

1. **Refuse** the combination for non-uniform scale and say why — a named
   refusal is a legitimate outcome under **decision 027** (*refuse what has no
   good reading, disclose what has one*), and `ROADMAP.md` §4 already put this
   on the table.
2. **Offer it and state the residual anisotropy** at the moment of the
   operation — disclosure rather than refusal.
3. **Offer an explicit stroke-to-outline conversion** (stroke → filled path)
   as the escape hatch, since PDF offers no non-scaling-stroke property. This
   destroys the object's editability *as a stroke*, which is why it must be
   explicit. Same offset-curve machinery as the boolean/path-ops slice.

**Related spec consequences that make the non-uniform case worse than it
looks, already citable from `ROADMAP.md` §4:** §8.4.3.5's miter limit is a
**ratio to line width** and §8.4.3.4's join angle is measured **in user
space**, so an anisotropic scale can visibly change corner shape with nothing
about the miter limit edited. And **a line width of `0` is not a number a
scale can act on** — `0 × 2 = 0` — which is *preserved* under both mechanisms
but **destroyed by any scheme that bakes the CTM into an explicit `w`.**
Hairlines are ubiquitous in CAD output, which is this project's actual corpus.

**One more constraint on any compensating implementation, derived from the
specs rather than observed:** compensation must use `|s|`, **never `s`**. A
signed division yields a negative stroke width, which §8.4.3.2 forbids (*"a
non-negative number"*). Negative scale factors are legitimate — they mean
mirroring about the bounding-box edge — and a pure flip leaves rendered
thickness identical, since stroke width is a distance.

---

## 6. ★★ THREE INVERSIONS AND ONE NON-TRANSFER — each would mislead someone scoping from the reference

These are recorded because the natural move — *"do what Inkscape does"* — is
**wrong in a different direction each time.**

### (a) Patterns invert: OFF is PDF's structural default, ON is the branch requiring work

In SVG, *"transform the pattern with the object"* is the branch that needs an
edit (`patternTransform`). **In PDF it is the opposite.** A PDF tiling
pattern's `/Matrix` maps pattern space to the **default** coordinate space of
the page — **not** to the CTM in effect at paint time. So *"don't transform
the pattern"* is what PDF does if pdfce writes nothing, and **ON is the branch
requiring work.**

**⚠ OWED, carried forward rather than assumed:** the researcher flagged the
`/Matrix` anchoring clause **itself** as needing a re-check against
`PDF_Spec` §8.7.3 — it was not re-fetched during the research session. **This
record inherits that debt and does not discharge it.** Dispatch
`pdfce-spec-librarian` before this clause grounds an acceptance criterion.

### (b) Group-vs-each inverts: per-object is the cheap one in PDF

In SVG, scaling a group is one matrix on the `<g>` and per-object is the
expensive path. **In PDF it is reversed.** pdfce's object model already
segments a content stream into *"run of path-construction operators terminated
by one painting operator"* objects (decision 011 §2.1), so **per-object
scaling is N independent operand rewrites with no grouping construct needed**,
and **as-a-group scaling is the one that needs a shared `q … cm … Q` wrapper**
or a coordinated N-way rewrite about a common origin.

This is the single most consequential option on a multi-object selection, and
the difference is invisible until after the operation — so it must be an
explicit mode with a stated default, in the GUI and in `pdfce-cli` alike.

### (c) Rounded-corner radii do not transfer at all — **RULED, see §7**

### (d) Markers have no PDF construct — do not replicate the coupling

SVG's default `markerUnits="strokeWidth"` means **marker size follows the
stroke width**, so a stroke-scaling toggle silently governs arrowhead size too
without saying so. **PDF has no marker or arrowhead primitive** — an arrowhead
in page content is ordinary baked filled geometry. **The coupling has no PDF
analogue and must not be replicated.**

**Do not conflate this with annotation `/LE` line endings** — that is a
different, annotation-only mechanism belonging to family (a).

---

## 7. ✅ RULED — the rounded-corner-radii toggle is `out_of_scope`

**This is the one thing this record decides.** Ruled by the operator at
dispatch, 2026-08-07.

**Why it is un-implementable rather than unbuilt:** PDF's `re` operator draws
a **sharp** rectangle. A rounded rectangle in a PDF content stream arrives as
**already-flattened Bézier geometry (`c` segments) with no surviving radius
parameter.** There is nothing for a *"scale radii"* toggle to act on. Radii
scale with the geometry, unconditionally, and **that is the only available
behaviour.**

**Recorded so a future pass does not re-investigate:** the answer is *"PDF
flattened it before pdfce ever saw it"*, **not** *"we haven't looked."*

**Narrow caveat, stated so the ruling is not over-read:** the ruling is about
**page content pdfce did not author.** If pdfce ever introduces its own
parametric-shape sidecar (a `/PieceInfo`-style record, as the ce-dimension
model already does), a radius could survive *for pdfce's own shapes*. That is
a different feature, not a reopening of this one.

---

## 8. ★★ FORM FIELDS ARE A THIRD BACK-END, WITH NO INKSCAPE ANALOGUE WHATSOEVER

**One operator gesture — *"resize this"* — has two mechanisms behind it, and
the second one has no bearing on §1 at all.**

Resizing an annotation or form widget is **not** a content-stream edit. It is
a `/Rect` edit that must keep the appearance stream's `/BBox` and `/Matrix`
consistent (§12.5.2, §12.5.5). **`/Rect` is neither *geometric* nor *visual*
bbox in the SVG sense** — it is a **declared box the appearance is fitted
into**, a third kind of extent that Inkscape has no concept of.

**⚠ Do NOT scope widget resize from the Inkscape RAG.** Scope it from
`Acrobat_Features` + `PDF_Spec` §12.5.5. The research files say this
themselves, in two separate places, unprompted.

**This is why `Pass 46.0` is not blocked on this record** — family (a) never
reaches the wrap-vs-rewrite question. It has its own harder problem, already
recorded in the `Pass 46.0` entry: §12.5.5's placement algorithm step (b) maps
the appearance box's corners onto `/Rect`'s **independently in x and y**, so
enlarging `/Rect` **anisotropically stretches the artwork** rather than
revealing more of it, which makes a resize a **regenerate**, not an array
write.

**The shared obligation across both back-ends:** a typed absolute size and a
dragged resize must produce the same result, and **the operator must be told
which extent a number refers to.** Both routes should go through one
`pdfce-core` operation taking the companion choices as explicit parameters —
which also makes `pdfce-cli` a first-class caller of the same code (project
rule 11) rather than a parallel implementation.

---

## 9. What DOES transfer verbatim — the core stroke model

Recorded because it is the part that needs **no** translation, and because a
record full of non-transfers could leave the impression that nothing carries
over.

**ISO 32000-1 §8.4.3.2**: line width is *"a non-negative number expressed in
**user space units**; stroking a path shall entail painting all points whose
perpendicular distance from the path **in user space** is less than or equal
to half the line width."* **That is precisely SVG's model.** A `cm` scale
therefore scales the stroke exactly as an SVG `transform` does.

And the consequence is already recorded on the PDF side independently: the
spec RAG's own derived note states that **an anisotropic CTM makes stroke
thickness orientation-dependent** — the same fact Inkscape's users report as a
bug. **Every stroke-model finding in the research transfers to pdfce
unchanged.** There is no SVG-only wrinkle to discount.

---

## 10. GAPs — recorded as GAPs, not as defaults

**Each of these is something the research deliberately declined to assert.
None may be quoted as fact, and none may be quietly turned into a default by
whoever implements `Pass 46.1`.**

1. **The non-uniform compensation formula is UNKNOWN.** Three sources describe
   the **symptom** without stating the **arithmetic**. Candidate conventions
   (geometric mean, `sqrt(|det|)`, arithmetic mean, dominant axis) are all
   plausible — **and `sqrt(|det|)` was DELIBERATELY NOT recorded as fact**,
   even though the SVG "expansion factor" convention would suggest it. **R61
   bars reading Inkscape's source** to settle it. **Do not encode any specific
   formula into acceptance criteria as an "Inkscape does X" fact.** If pdfce
   needs a formula, it must derive and justify its own — and disclose it under
   rule 4, because a chosen width is inferred state.
2. **Whether the companion toggles govern the NUMERIC route as well as the
   drag is unconfirmed.** No reachable source states outright that a typed
   scale honours the stroke setting. **It matters** — an operator who sets a
   mode and then types a size must get the same stroke result as one who
   drags. **pdfce should make the answer *"yes, identically"* by
   construction** rather than inherit an unverified one.
3. **Set exhaustiveness is unconfirmed. ⚠ Do not say "Inkscape has exactly
   four."** Four companion behaviours are **confirmed present** — scale stroke
   width · scale rounded-corner radii · transform gradients with object ·
   transform patterns with object — plus *Store transformation*
   (Optimized | Preserved) and a visual-vs-geometric **bbox basis**, and
   **neither of those last two is a companion toggle**: they change *how the
   scale is recorded* and *what the scale is measured against*, not *what
   scales*. **There is no filter toggle.** That no fifth companion behaviour
   exists in current Inkscape was **not** establishable from a reachable
   primary source (the relevant manual chapter returned HTTP 522 twice).
   **State the four that are verified and state that the enumeration is
   unconfirmed.**
4. **Which bbox basis is the reference's default was not stated** by the
   retrieved source. Do not assert one. (pdfce is *freer* here regardless —
   page content has no stored bbox in PDF at all, both extents are computed,
   so pdfce can offer both bases with no storage consequence, and can compute
   the true stroked extent rather than the round-join approximation the
   reference uses for its own performance reasons.)
5. **Version pin.** The verbatim option wording comes from an older-edition
   manual; current-era sources corroborate the **behaviours** in 1.x but the
   **exact current strings** were not re-verified.

---

## 11. ⚠ OWED — two dispatches this record raises and does not discharge

### (a) To `pdfce-spec-librarian` — the `/Matrix` anchoring clause

Re-check **§8.7.3**: that a tiling pattern's `/Matrix` maps pattern space to
the page's **default** coordinate space rather than to the CTM at paint time.
**§6(a)'s entire inversion rests on it**, and it was not re-fetched during the
research session. Flagged by the researcher, inherited here, **not
discharged.**

### (b) To `pdfce-ui-specialist` — the hidden-global-mode failure

**The reference's stroke toggle is a hidden global mode**, and *"why did my
stroke change?"* is one of its most-asked transform questions — **the mode is
invisible at the moment it matters.**

**Under project rule 4 that is a failure, and pdfce would be committing it in
the same shape:** the scale factor is something the operator *performed* and
can see, but **the stroke consequence is something pdfce would choose on their
behalf from off-screen state.** Rule 4 as narrowed by decision 024 §4.4 does
**not** require a confirm step for a visible, undoable direct manipulation —
and none is being asked for here. What is required is that **the state be
legible at the point of the resize**, in the tool's dock compartment at a
fixed anchor (§4.4's placement constraint: nothing floats over the canvas at a
document-derived position).

**This record asserts the behavioural requirement, not the widget.** The
widget is the specialist's call.

---

## 12. What this record does NOT claim

- **It does not decide §1.** Options A–D are laid out; **none is
  recommended.** The operator's ruling is awaited, and arrives as a dated
  amendment to this file plus a matching `ARCHITECTURE.md` §12 entry.
- **It mints nothing** — no Pass ID (ceiling stays **Pass 46**), no standing
  rule (ceiling stays **R166**, **R167** next free).
- **It does not restate the research.** The four `transforms__*` files are the
  source; this record points at them. If they and this record disagree, **the
  RAG files are the sourced side** for Inkscape behaviour and `PDF_Spec` is
  the sourced side for PDF behaviour.
- **It ships no code and claims no gate.** No build, no test, no render was
  run by the filing that produced it.
- **It does not scope text or images.** Both are named in `ROADMAP.md` §4 as
  open *inside* family (b) — text has four candidate mechanisms (`Tf` size /
  `Tz` / `Tm` rewrite / wrapping `cm`) that interact with shipped Pass 19.x
  work, and an image XObject is already drawn *through* its `cm` (§8.9.5),
  making it the cheapest member. **The §1 ruling constrains both but settles
  neither.**
- **It does not touch `Pass 46.0`.** See §8.
- **It does not discharge §11.** Both dispatches remain owed.
