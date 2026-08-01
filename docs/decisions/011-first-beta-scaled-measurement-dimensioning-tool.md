# Decision 011 — Architecture and Pass-slicing for the operator's first usable beta: a scaled measurement/dimensioning tool built on vector selection, snapping, and basic vector editing

- **Date:** 2026-07-31
- **Status:** Decided
- **Decider:** KenAgent (autonomous-builder / decision-consultant), per the
  ROADMAP standing rule "KenAgent decision routing (operator process rule,
  2026-07-30)".
- **Question:** How should pdfce's FIRST usable beta — a scaled
  measurement/dimensioning tool (vector selection + snapping + basic vector
  editing + a new dimensioning subsystem) — be architected and sliced into
  buildable Passes, honoring the project invariants and the C→B→A sequence
  decision 010 set?
- **Outcome:** Build the beta as decision 010's Pass 12 R60 substrate (built
  once, not a throwaway) + a NEW measurement/dimensioning subsystem + the
  first minimal slices of Pass 9 vector editing (select / move / delete /
  drag-node). Five slices: **12.0** (substrate), **9a** (object/selection
  model + centerline), **12.M1** (snapping), **12.M2** (dimensioning + scale/
  group + hybrid storage + OCG layer), **9c-min** (basic editing). The
  authorizing lever is decision 010 **revisit-trigger 3** (operator demand
  promotes vector editing after B, with the disclosed spot-check caveat).
- **Amends:** Nothing structurally. Decision 010's **C→B→A destination, Pass
  IDs, and ranking are intact.** This record REPACKAGES the front of that
  path into a shippable beta: C (Pass 11) continues in parallel (already in
  progress); B (Pass 12) leads with its mandatory substrate but its FIRST
  landed tool becomes measurement/dimensioning; the first slices of A
  (Pass 9) are pulled forward. The three deferred editing GUIs and the rest
  of Pass 9 become fast-follow on the SAME substrate.

---

## 1. The authorizing basis and the one insight that shapes everything

The operator has explicitly named this his **first usable beta**. That is,
verbatim, decision 010's revisit-trigger 3: *"Operator wants vector editing
immediately, accepting unverified-render risk → A/Pass 9 can promote after B
alone (B is the hard UI prerequisite; C is the correctness prerequisite).
Brief that this ships vector edits whose visual correctness is spot-checked,
not corpus-measured — the fuzzy-never-sneaky posture applied to a scheduling
decision."* This record does not fight decision 010; it exercises the escape
hatch decision 010 explicitly built for exactly this operator demand.

**The load-bearing insight — the beta cleaves by correctness oracle.**
Decision 010's whole argument was that vector editing is the first subsystem
whose oracle is *independent visual fidelity* (redaction dodged it with a
byte-absence oracle), and that this is why C (render-fidelity verification)
must precede A. That argument applies with a scalpel here:

- The **dimensioning half** of this beta is **purely additive**. A dimension
  is an authored PDF annotation placed by **overlay-append** (§5.8) — the
  same mechanism as flatten and markup authoring. The page's existing content
  stays **byte-verbatim**, so the **existing self-comparison round-trip
  oracle is sufficient** and **Pass 11 (C) is NOT a prerequisite** for it.
- The **editing half** (move object, delete object, drag node) is
  **content-stream surgery** that re-renders the edited page (§5.7, the
  R46/R58 named exception, the mirror of redaction). It **is** the subsystem
  whose oracle is independent visual fidelity, so it **rides Pass 11's (C's)
  render-fidelity gate (R59)** — or, if C has not landed, ships
  spot-checked-with-explicit-disclosure per revisit-trigger 3.

This is not a scheduling convenience; it is the correct application of
decision 010's own principle. It means the **additive measurement beta can
ship the moment the substrate + object model + snapping + dimensioning land,
with zero dependency on C**, while the editing half attaches to C's gate as
it lands (C is already in progress). If C lands first, all five slices ship
together; if not, the beta ships in two honest steps.

---

## 2. Architecture

### 2.1 (A) Vector object/selection model

The model is a **read-only decomposition layer that INDEXES the existing
lossless content-token model** (`pdfce-core` `content.rs`), not a replacement.
It reuses the **same content-token walk + graphics-state tracking**
`pdfce-render` uses, so the object model and the render agree by construction
— the geometry analogue of the R49/R60 "one pipeline" discipline. It lives in
`pdfce-core` (GUI-free) and is consumed by `pdfce-render` (hit-test geometry)
and `pdfce-gui` (via the substrate).

**How objects are built.** Walk the page content token stream tracking
graphics state (CTM from `q`/`Q`/`cm`, line width, dash, stroke/fill colour).
Segment into **path objects** at each painting operator: an object = the run
of path-construction operators (`m l c v y re h`) terminating in a painting op
(`S s f F f* B B* b b* n`), captured with:

1. its subpaths as **node lists** (anchors + Bézier control points) in **user
   space**, transformed by the effective CTM into **page space** for
   hit-test/snap;
2. the **effective graphics state** at paint time;
3. the **content-token index range / ByteSpan** of its defining operators —
   the handle that maps a selection back to the **Pass 8.0 surgery
   interpreter** for editing.

Text objects (`BT..ET`) and image objects (`Do` on an image XObject) are
decomposed as **selectable-for-move/delete** objects but are **not
node-editable** in the beta (dimensioning cares about path geometry).

**The "line center not thickness" requirement.** Dimensions snap to path
**geometry** (the centerline), never stroke edges. Two cases:

- **Stroked path:** the geometry **is** the centerline (the stroke straddles
  the path ±w/2). Snapping to path nodes/segments already yields the
  centerline — no special handling.
- **Line drawn as a thin FILLED rectangle / filled 4-point quad:** DERIVE the
  midline connecting the midpoints of the two short edges, detected by an
  aspect-ratio threshold (long:short over ~8:1). This derivation is a **fuzzy
  inference**: shown as a highlighted candidate centerline with a "centerline
  derived from filled shape" disclosure, confirmed or overridden by the
  operator. **Never auto-committed** (fuzzy-never-sneaky).

### 2.2 (B) Snapping engine

A **shared substrate service** (not dimension-specific — it also serves the
vector-editing move/node-drag). Snap targets, priority high→low:

1. path nodes / anchor vertices
2. explicit segment endpoints
3. circle/arc centers (derived, including best-fit)
4. segment midpoints
5. segment–segment intersections (computed on demand within tolerance)
6. nearest point on a segment centerline (perpendicular projection)
7. page-axis / optional grid

**Tolerance** is a constant **screen-space** value (≈8–12 px) converted to
page space via the current transform, so snap "feel" is **zoom-invariant**.
Ties resolve by priority, then by nearest.

**H/V alignment constraint** — the operator's "snap to horizontally or
vertically aligned" option. A tool toggle (and/or modifier): HORIZONTAL
projects the two points onto the page **X** axis (length = |Δx|), VERTICAL
onto **Y** (=|Δy|), ALIGNED = free Euclidean. Under constraint the second
pick can be constrained to share Y (horizontal) or X (vertical) with the
first.

**Fuzzy indicator.** The current snap candidate is shown **before** the click
commits — a marker glyph + a **type label** ("node", "endpoint", "center",
"midpoint", "intersection", "centerline"). The operator sees exactly what was
inferred and can cycle/override. Snapping is the beta's primary
fuzzy-never-sneaky surface.

### 2.3 (C) Dimensioning subsystem

**Linear dimensions.** Pick point A, pick point B (each snapped), optional H/V
constraint. `measured_pdf_length` = axis-projected or Euclidean **page-space**
distance in PDF default user units (1/72"). Displayed value =
`measured_pdf_length × group.scale`, formatted in `group.units`. Rendered as
an authored `/Line` annotation with `/IT /LineDimension`: a baked `/AP`
leader + extension lines + value text, reusing **Pass 6.1** geometry authoring
and the **Pass 6.2 §12.7.3.3 vartext** label generator.

**Radius/diameter dimensions.**

- From a **circle object** (PDF circles are four ≈0.5523-κ Béziers, or an
  ellipse): fit a circle to the object's node set.
- From **multiple selected nodes** "that make up a circular area (might be
  small line segments)": flatten any Béziers to sample points at tolerance,
  take segment endpoints (+midpoints if segment count is low) as fit samples,
  run a least-squares best-fit circle.

**Best-fit algorithm — Taubin, chosen deliberately.** PRIMARY = **Taubin**
fit (algebraic, closed-form, no iteration), with an optional single
Gauss-Newton geometric-refinement step. Taubin is chosen **specifically
because the operator's stated case is small line segments approximating an
arc** (partial-arc / short-arc data) — the exact regime where the simpler
**Kåsa** fit is strongly **biased in radius**; Taubin is near-unbiased for
partial arcs at nearly the same cost. Hand-rolled ~50–80 lines of linear
algebra (a 3×3 generalized eigenproblem / closed form) — **ZERO new
dependency** (rule 13). The **fit residual** (RMS node-to-circle distance) is
reported so the operator sees fit quality; the fitted circle (center +
radius + residual) is drawn as a **preview** the operator accepts or rejects —
**fuzzy-never-sneaky; the best-fit circle is a reviewable hint, never a
silent auto-apply.** displayed radius = `fitted_radius_pdf × scale`; diameter
= 2×.

**Value data model (the clean part).** GEOMETRY is **immutable** and stored
(measured pdf length / fitted radius). The DISPLAYED value is **derived** =
`geometry × group.scale` in `group.units`, recomputed — and the `/AP` label
regenerated (reusing the **Pass 7.1** regenerate-appearances pattern) —
whenever the group's scale changes. This is what makes "change the group
scale → all member dimensions update" cheap and correct.

### 2.4 (D) Scale + group model and hybrid storage

**Group** = a named entity: `{ name, scale (f64 real-units-per-pdf-unit),
units (mm|cm|m|inch|decimal-ft|ft-in), number-format (precision, suffix;
ft-in gets feet+inches formatting), ocg (the group's OCG ref), members }`. A
default/active group always exists so a dimension has a home; dimensions are
(re)assignable to named groups.

**Scale-dimension workflow (the operator's exact requirement).** The operator
draws a line like the dim tool (pick A, pick B, snapped) → a special **scale
dimension**. Then EITHER (i) types the **scale ratio** directly, OR (ii) types
the **real length + units** of that drawn line and pdfce back-calculates
`scale = real_length / drawn_pdf_length`. The group receives this scale;
editable later; re-propagates. Path (ii) is the RECOMMENDED unambiguous path.
Path (i) needs a paper-unit basis ("1:100" means 1 paper-unit = 100 real-units
of the same kind, and PDF paper units are 1/72") — a definitional detail to
source from **§12.9 NumberFormat** semantics; default basis = inch, disclosed.

**HYBRID storage (binding answer 1) — three coordinated layers:**

1. **Native portable layer.** Each dimension → a **real PDF annotation**
   portable to any reader: `/Line` with `/IT /LineDimension` (linear);
   radius/diameter as a `/Line` from center to rim with `/IT /LineDimension`
   plus the sidecar carrying the radial semantics (the native measurement
   model is weak on radial dims, so the baked `/AP` shows leader+value and the
   sidecar holds the "radius/diameter of fitted circle C" meaning). The baked
   `/AP` shows the drawn dimension + value in **any** reader.
2. **Native scale.** Where a group maps cleanly to a single page region,
   author a native `/Viewport` with a `/Measure /RL` (rectilinear) dict
   carrying `/X /Y /D /A` + **§12.9.3 `/NumberFormat`** (`/U` unit label, `/C`
   conversion factor, `/D` precision) — so the SCALE is portable in
   §12.9-honoring readers. Overlapping groups with different scales on one
   page do NOT map to disjoint `/BBox` viewports, so native `/Measure` is a
   best-effort **portability PROJECTION**, not the authority.
3. **Authoritative sidecar.** The pdfce private model — group defs, membership,
   best-fit params + residual + contributing nodes, centerline-derivation
   provenance, scale-dimension record — is carried **IN-PDF via §14.5
   `/PieceInfo`** (the spec-sanctioned home for private application data that
   survives round-trip through other editors), keyed to a pdfce private key.
   This keeps the hybrid a **single self-contained PDF** (no loose external
   file required; an optional external `.pdfce-dims` export is a fast-follow).
   **The sidecar is authoritative in pdfce; the native `/Measure` is the
   portability projection.** On load, if native and sidecar disagree (a
   third-party editor changed one), **disclose and prefer the sidecar**
   (fuzzy-never-sneaky).

**The OCG layer.** One **§8.11 `/OCG`** per dimension group (so each group
toggles independently), registered in the catalog `/OCProperties` with a `/D`
default config; each dimension annotation carries `/OC` → its group's OCG.
Toggleable in any OCG-honoring reader AND in pdfce. Because pdfce currently
**DEFERS `/OC` rendering** (BDC/EMC + `/OC` visibility is a known render gap —
Pass 6.0 deviation 2, §8.11 RAG gap), the beta adds a **bounded** piece:
honor the visibility of pdfce's **OWN AUTHORED** dimension OCGs —
**annotation-level `/OC`** membership checked against `/OCProperties /D` (the
Pass 6.0 annotation walk already exists; add the `/OC`→OCG visibility test + a
per-group GUI toggle). **FULL content-stream BDC/EMC `/OC` visibility stays
deferred** (a named non-goal); only the authored-layer annotation `/OC` + `/D`
default config are honored. This closes exactly the sliver of the `/OC` gap
the operator's "own toggleable layer" requirement needs, and no more.

### 2.5 (E) Basic vector editing (binding answer 2)

The **minimum** that makes the beta a real editor without becoming full
Pass 9 — exactly three operations:

1. **Move object** — translate all of the object's path-construction operands
   by (dx,dy) in its user space (CTM-aware).
2. **Delete object** — remove the object's construction + painting operators.
3. **Drag node** — rewrite one anchor's coordinate pair in the `m`/`l`/`c`
   operand list (anchor/corner move; adjacent control-point/handle editing is
   a fast-follow).

**Mechanism.** All three are **content-stream surgery** via the Pass 8.0
advance-preserving interpreter: locate the object's operator byte range (from
the object model's token-index range), rewrite/remove operands, re-emit
**ONLY that stream** (R46 named exception, §5.7 — the mirror of redaction's
operator removal). Compressed-object-stream objects are **promoted/decomposed
per §5.7/§5.9**. Each edit is one undoable command (Pass 3.1 command log);
snap-on-drag comes from 12.M1.

**Excluded (stays full Pass 9):** boolean path ops, gradients/shading/
transparency editing, numeric matrix transforms, align/distribute, z-order,
group/ungroup, text-to-path, in-place text edit, image resample.

**Oracle dependency.** Move/delete/drag-node **re-render** the edited page →
this is the first subsystem whose oracle is independent visual fidelity → it
**rides Pass 11's (C's) render-fidelity gate (R59)**. If C has not landed when
the beta ships, 9c-min ships **spot-checked-with-disclosure** per revisit-
trigger 3. The additive dimensioning half has no such dependency.

### 2.6 (F) The canvas substrate — built once, per R60

**Confirmed: the beta builds decision 010's Pass 12 FIRST SLICE — the ONE
R60 canvas-interaction substrate — NOT a throwaway measurement canvas.** It
provides: a **focusable canvas** (`Sense::click_and_drag`, resolving
`main.rs`'s Pass-1 focusable-canvas caveat), `viewer::screen_to_page` /
`page_to_screen` (page-space geometry, rotation-correct via `current_extent`),
a **tool-mode dispatch state machine**, a **hit-test/selection scaffold with a
pluggable target provider** (the object model plugs in as the provider), and a
**live-preview overlay**. It is built generically so the dimension tools, the
vector-editing move/node-drag, AND the three deferred editing GUIs
(markup/form-fill/redaction) all layer onto the SAME widget. A second parallel
canvas-interaction path is forbidden (R60).

This is the beta's structural justification: **the operator's first usable
tool doubles as the substrate every other editing GUI has been waiting for.**

---

## 3. Pass slicing (the buildable plan)

Proposed IDs; the librarian confirms. IDs keep decision 010's families
(Pass 12 = B substrate + tools; Pass 9 = A vector editing, pulled forward,
ID never renumbered).

| Slice | Name | Additive/Surgery | Oracle | R59? | Blocking prereqs |
|---|---|---|---|---|---|
| **12.0** | Canvas substrate (R60) | n/a | n/a | no | ui-specialist (substrate) |
| **9a** | Object/selection model + centerline | read-only | self-compare | no | 12.0; inkscape-librarian |
| **12.M1** | Snapping engine | n/a | n/a | no | 9a; inkscape-librarian |
| **12.M2** | Dimensioning + scale/group + hybrid storage + OCG | **additive** | **self-compare (C NOT required)** | yes | 9a+12.M1; **spec-librarian §12.9/§14.5/§8.11**; acrobat-librarian; ui-specialist |
| **9c-min** | Basic editing: move/delete/drag-node | **surgery** | **independent visual (needs C)** | yes | 9a+12.0+12.M1; **Pass 11 (C)** or disclosed spot-check |

**Shippable beta** = all five slices (answer 2 makes basic editing in-scope).
**Two-step fallback if C is not yet landed:** STEP 1 "measurement beta" =
12.0 + 9a + 12.M1 + 12.M2 (additive, zero C dependency); STEP 2 "editing beta"
= 9c-min riding Pass 11's gate. **Fast-follow (not beta):** the three deferred
editing GUIs on the same substrate; per-group takeoff totals + CSV (answer 4);
associative dimensions; Bézier handle editing; area/perimeter measurement; the
rest of Pass 9.

Full per-slice deliverables, acceptance criteria, non-goals, and risks are in
**Appendix A** (the JSON decision block the engineer implements from).

---

## 4. Prerequisites to dispatch in parallel NOW

- **pdfce-spec-librarian** — **§12.9** (`/Measure` `/Viewport` `/RL`,
  §12.9.3 `/NumberFormat` `/U`/`/C`/`/D`, `/IT /LineDimension` /
  `/PolyLineDimension`), **§14.5** (`/PieceInfo` private app data), **§8.11**
  (`/OCProperties` `/OCG` `/OCMD` `/OC`, `/D` config, BDC/EMC scope).
  **BLOCKING for Pass 12.M2.** Dispatch immediately.
- **pdfce-acrobat-librarian** — the Measuring-Tool / dimension capability
  bucket (distance + scale ratio + snap-to-content + measurement-markup
  persistence + units). Grounds 12.M2 acceptance criteria. Dispatch now.
- **pdfce-inkscape-librarian** — object/node selection + snapping capability
  bucket (node tool, snap targets & priority, bbox-vs-node snap). Grounds 9a +
  12.M1. Agent + `Inkscape_Features` RAG scaffold already exist. Dispatch now.
- **pdfce-ui-specialist** — (i) the R60 substrate interaction taxonomy
  (12.0); (ii) the dimension tool UX — tool modes, snap indicator, group
  panel, scale-dimension entry dialog, best-fit-circle confirm, per-group
  layer toggle (12.M2). Dispatch when 12.0/12.M2 are scoped.
- **Pass 11 (C)** render-fidelity harness — ALREADY IN PROGRESS. No new
  dispatch; 9c-min consumes its R59 gate.

---

## 5. Invariants honored

- **Round-trip / minimal-diff (R46, §5.7/§5.8/§5.9):** dimensioning is
  additive (overlay-append §5.8, zero R46 exception); the object-model
  decomposition (9a) is byte-inert; editing (9c-min) is the R46/§5.7 named
  surgery exception (mirror of redaction) — only the edited stream re-emits,
  compressed containers promoted/decomposed, everything else byte-verbatim.
- **Fuzzy, never sneaky:** every inference is a reviewable hint — the best-fit
  circle (with residual) previewed + confirmed; the snap target shown
  pre-commit with its type label; the filled-rectangle centerline derivation
  disclosed + confirmed; native-vs-sidecar storage disagreement disclosed on
  load. Nothing auto-applies.
- **R59 render-fidelity gate:** re-run on the render-touching slices (12.M2
  authored `/AP` + OCG visibility; 9c-min surgery re-render). Additive-only
  slices (12.0/9a/12.M1) are not render-touching.
- **R60 one canvas substrate:** the beta builds the single shared substrate;
  dimensions, vector-edit, and the three deferred GUIs all layer on it.
- **R61 Inkscape behavioral reference only:** snapping/node-selection behavior
  sourced from the Inkscape capability RAG (behavior/limits only,
  GPL-2.0 — never a dependency, code source, or GUI mimicry); UI designed
  independently by pdfce-ui-specialist.
- **Rule 13 — no new copyleft deps:** ZERO new dependencies across all five
  slices (Taubin, snapping, object model, storage all hand-rolled),
  consistent with the project's ZERO-new-dep posture through Pass 8.
- **GUI-core separation:** object model, snapping math, dimension geometry,
  best-fit, and storage all in `pdfce-core` (GUI-free); only the tool UX +
  substrate in `pdfce-gui`. `cargo tree` core+render stays egui/eframe/winit/
  wgpu-free.

---

## 6. Revisit triggers

1. Pass 11 (C) surfaces a systematic render divergence pdfce cannot cheaply
   reconcile → re-decide before 9c-min ships (the additive measurement half is
   unaffected and can still ship).
2. Operator wants per-group takeoff totals/CSV (answer 4 was "labels only for
   now") → promote the fast-follow totals slice.
3. Operator wants area/perimeter measurement or **associative** (re-measuring)
   dimensions → scope as a follow-on measurement Pass (beta dimensions are
   static).
4. §12.9 native `/Measure` proves too weak for radius/diameter interop → the
   sidecar already carries the semantics; revisit a richer native form.
5. Any beta slice hits the three-attempts wall → decision 010's fallback
   (Pass 5 encryption) remains the designated switch track.

---

## 7. Operator actions owed (resurfaced, not new)

- Encryption-refusal sign-off (oldest owed, now across decisions
  007/008/009/010).
- License decision (LEGAL §1) — gates the public repo/release and the GPL
  Inkscape-reference boundary; this beta remains a **private build** until the
  license lands (rule 8).
- Commit authorization — Passes 0–8 ALL uncommitted; this beta adds more
  unversioned work (W15/X16).
- `/R6` AES-256 sourcing + LEGAL §2 Adobe-supplement copyright contradiction
  (Ken's calls when Pass 5 activates).

## References

- `docs/decisions/010-highest-value-investment-after-the-editing-arc.md` — the
  C→B→A sequence, revisit-trigger 3 (the authorizing lever), standing rules
  R59/R60/R61.
- `docs/decisions/008-next-subsystem-after-extract.md` §5.3 — the vector-
  editing slicing a–g this beta pulls (a) and a minimal (c) forward from.
- `docs/ARCHITECTURE.md` §5.7 (mutation/promotion/stale-copy), §5.8
  (flatten overlay-append — the additive pattern dimensions reuse), §5.9
  (R58 removal/surgery pattern the editing slice mirrors).
- `docs/ui_specs/pass-6.1-markup-tools.md`, `pass-7-form-fill.md`,
  `pass-8-redaction.md` — the three deferred GUIs that fast-follow on the
  same Pass-12.0 substrate.
- ROADMAP standing rules R46 (content-identity), R49 (one appearance
  pipeline), R58 (removal forces full rewrite), R59 (render-fidelity gate),
  R60 (one canvas substrate), R61 (Inkscape behavioral reference).
- Spec prerequisites (to be sourced): ISO 32000 §12.9 (measurement/Viewport/
  NumberFormat), §14.5 (PieceInfo), §8.11 (optional content).

---

## Appendix A — JSON decision block

```json
{
  "decision_id": "011",
  "title": "Architecture and Pass-slicing for the operator's first usable beta: a scaled measurement/dimensioning tool built on vector selection, snapping, and basic vector editing",
  "date": "2026-07-31",
  "status": "Decided",
  "decider": "KenAgent (autonomous-builder / decision-consultant), per the ROADMAP standing rule 'KenAgent decision routing (operator process rule, 2026-07-30)'",
  "supersedes": "nothing",
  "amends": "Nothing structurally — this record USES decision 010's revisit-trigger 3 (operator demand promotes vector editing after B) as its authorizing lever. It does NOT reorder decision 010's C→B→A; it REPACKAGES the front of that path into a shippable first beta: Pass 11 (C) continues in parallel (already in progress); Pass 12 (B) leads with its mandatory substrate slice but its FIRST landed tool becomes the measurement/dimensioning tool instead of the markup/form-fill/redaction GUIs; and the first slices of Pass 9 (A: object model + selection + minimal node/object editing) are pulled forward into the beta. The three deferred editing GUIs (markup, form-fill, redaction) and the rest of Pass 9 become fast-follow on the SAME substrate. Decision 010's Pass IDs, ranking, and the C→B→A destination are otherwise intact.",

  "authorizing_basis": "Decision 010 revisit-trigger 3: 'Operator wants vector editing immediately, accepting unverified-render risk → A/Pass 9 can promote after B alone (B is the hard UI prerequisite; C is the correctness prerequisite). Brief that this ships vector edits whose visual correctness is spot-checked, not corpus-measured — the fuzzy-never-sneaky posture applied to a scheduling decision.' The operator has explicitly prioritized this beta as his first usable deliverable, which is exactly the trigger.",

  "headline": "The beta is the ONE canvas-interaction substrate (decision 010's Pass 12 / R60, built once, not a throwaway) plus a NEW measurement/dimensioning subsystem plus the first, minimal slices of vector editing (select / move / delete / drag-node). The single most important architectural insight is that the beta cleaves cleanly by CORRECTNESS ORACLE: the DIMENSIONING half is purely ADDITIVE (dimensions are authored PDF annotations placed by overlay-append, §5.8 — page content stays byte-verbatim, so the existing self-comparison round-trip oracle is sufficient and Pass 11/C is NOT a prerequisite for it); the EDITING half (move/delete object, drag node) is content-stream SURGERY that re-renders the edited page (§5.7, the R46/R58 named exception, mirror of redaction), and it IS the first subsystem whose acceptance oracle is independent visual fidelity — so it rides Pass 11's (C's) render-fidelity gate (R59) or ships spot-checked-with-disclosure per revisit-trigger 3. This split lets the additive measurement beta ship the moment the substrate + object model + snapping + dimensioning land, independent of C; the editing half attaches to C's gate as it lands (C is already in progress). Storage is HYBRID exactly as the operator scoped it: portable native PDF measurement annotations (§12.9 /Line + /IT /LineDimension, baked /AP leader+value) + a native page /Viewport//Measure //RL scale where it round-trips cleanly, ALL authoritatively backed by a pdfce private sidecar carried in-PDF via §14.5 /PieceInfo (group/scale/units/best-fit-circle/centerline provenance the PDF measurement model cannot express), placed on a per-group Optional Content Group (§8.11 /OCG) whose visibility pdfce honors for its OWN authored layer (closing a bounded slice of the deferred /OC render gap — authored-layer annotation /OC + /D default config only; full content-stream BDC/EMC /OC stays deferred).",

  "beta_definition": {
    "operator_intent_preserved": [
      "Select nodes and objects and snap to them (Inkscape-style).",
      "Add dimensions with an option to snap to horizontal- or vertical-aligned; add them by snapping to an existing line or node.",
      "Diameter and radius dimensions from circles OR from multiple selected nodes making up a circular area (small line segments → LEAST-SQUARES BEST-FIT circle).",
      "Dimensions assigned into named GROUPS; each group carries a SCALE and (binding answer 3) its OWN UNITS.",
      "Group scale set via a SCALE DIMENSION drawn on the PDF like the dim tool, then the operator either types the scale ratio OR types the real length+units and pdfce back-calculates scale = drawn-pdf-length / real-length.",
      "Group scale is editable and re-propagates to all member dimensions (live value recompute).",
      "Dimensions relate to line CENTERS (centerline geometry), NOT stroke thickness — including lines drawn as thin FILLED rectangles (derive the midline).",
      "Basic editing (binding answer 2): move/delete objects and drag nodes.",
      "Hybrid storage on its own OCG layer (binding answer 1). Per-group units (binding answer 3). Labels only, no per-group takeoff totals/CSV yet (binding answer 4)."
    ],
    "shippable_beta_slices": ["Pass 12.0 (substrate)", "Pass 9a (object/selection model + centerline)", "Pass 12.M1 (snapping engine)", "Pass 12.M2 (dimensioning + scale/group + hybrid storage + OCG layer)", "Pass 9c-min (basic vector editing: move/delete/drag-node)"],
    "shippable_beta_note": "Answer 2 makes basic editing IN-scope for the beta, so all five slices constitute the beta. BUT because 12.0+9a+12.M1+12.M2 are additive (oracle-sufficient without C) and 9c-min is surgery (needs C), the beta can ship in two honest steps if C is not yet landed: (STEP 1, 'measurement beta') the four additive slices, zero C dependency; (STEP 2, 'editing beta') 9c-min riding Pass 11's gate. If C lands first (it is already in progress), all five ship together.",
    "fast_follow_not_beta": [
      "The three deferred editing GUIs — markup drawing (pass-6.1-markup-tools.md), form-fill (pass-7-form-fill.md), redaction marking (pass-8-redaction.md) — land as SUBSEQUENT slices on the SAME Pass-12.0 substrate (this is why the substrate is built once, R60).",
      "Per-group takeoff totals + CSV export (binding answer 4: labels only for now).",
      "Associative dimensions (a dimension that re-measures when its underlying object's nodes are moved) — beta dimensions store measured geometry statically at creation, like Acrobat measurement markups.",
      "Bézier control-point (handle) editing, and the rest of Pass 9: boolean path ops, gradients/shading/transparency, numeric transforms, align/distribute, z-order, group/ungroup, text-to-path, FULL content-stream BDC/EMC /OC layer editing.",
      "Area/perimeter measurement (Acrobat's other measuring modes) — beta is linear + radius/diameter only."
    ]
  },

  "architecture": {
    "A_vector_object_selection_model": {
      "principle": "A READ-ONLY decomposition layer INDEXING the existing lossless content-token model (pdfce-core content.rs), NOT a replacement object model. It reuses the SAME content-token walk + graphics-state tracking that pdfce-render uses, so the object model and the render agree by construction (the geometry analogue of R49/R60 'one pipeline'). Lives in pdfce-core (GUI-free); consumed by pdfce-render for hit-test geometry and by pdfce-gui via the substrate.",
      "object_construction": "Walk the page content token stream tracking graphics state (CTM from q/Q/cm, line width, dash, stroke/fill colour). Segment into PATH OBJECTS at each painting operator: an object = the run of path-construction operators (m l c v y re h) terminating in a painting op (S s f F f* B B* b b* n), captured with (i) its subpaths as node lists (anchors + Bézier control points) in USER space, transformed by the effective CTM into PAGE space for hit-test/snap; (ii) the effective graphics state at paint time; (iii) the CONTENT-TOKEN INDEX RANGE / ByteSpan of its defining operators — the handle that maps a selection back to the Pass 8.0 surgery interpreter for editing. Text objects (BT..ET) and image objects (Do on image XObject) are decomposed as selectable-for-move/delete objects but are NOT node-editable in the beta (snapping cares about path geometry).",
      "centerline_requirement": "Dimensions must snap to path GEOMETRY (the centerline), never stroke edges. Two cases: (1) a STROKED path — the geometry IS the centerline (the stroke straddles the path ±w/2), so snapping to path nodes/segments already yields the centerline; no special handling. (2) a line drawn as a THIN FILLED RECTANGLE or filled 4-point quad (aspect ratio of long:short axis over a threshold, e.g. >8:1) — DERIVE the midline connecting the midpoints of the two short edges. This derivation is a FUZZY inference: shown as a highlighted candidate centerline with a 'centerline derived from filled shape' disclosure; the operator confirms or overrides (fuzzy-never-sneaky). Never auto-committed.",
      "surgery_mapping": "Selection → content-token index range → Pass 8.0 advance-preserving surgery interpreter. For editing (E), the object's numeric operands are rewritten/removed in place and ONLY that content stream is re-emitted (R46 named exception, §5.7); if the object lives in a compressed object stream, it is promoted/its container decomposed per §5.7/§5.9. For dimensioning (C/D), NO surgery occurs — dimensions are additive annotations."
    },
    "B_snapping_engine": {
      "targets_priority_high_to_low": ["path nodes / anchor vertices", "explicit segment endpoints", "circle/arc centers (derived, incl. best-fit)", "segment midpoints", "segment–segment intersections (computed on demand within tolerance)", "nearest point on a segment centerline (perpendicular projection)", "page-axis / optional grid"],
      "tolerance": "Constant SCREEN-space tolerance (≈8–12 px) converted to page space via the current screen↔page transform, so snap 'feel' is zoom-invariant. Ties resolved by priority, then by nearest.",
      "hv_constraint": "The operator's 'snap to horizontally or vertically aligned' option. A tool toggle (and/or modifier key) that constrains the dimension's measurement axis: HORIZONTAL dim projects the two points onto the page X axis (measured length = |Δx|); VERTICAL onto Y (=|Δy|); ALIGNED = free direction (Euclidean). Under constraint the second pick can be constrained to share Y (horizontal) or X (vertical) with the first.",
      "fuzzy_indicator": "The current snap candidate is shown BEFORE the click commits — a marker glyph plus a type label ('node', 'endpoint', 'center', 'midpoint', 'intersection', 'centerline'). The operator sees exactly what was inferred and can cycle/override. Snapping is the primary fuzzy-never-sneaky surface of the beta.",
      "reuse": "Built as a shared substrate service (not dimension-specific): the same snap engine serves the dimension tools AND the vector-editing move/node-drag (snap a dragged node to another object's node)."
    },
    "C_dimensioning_subsystem": {
      "linear": "Pick point A, pick point B (each snapped), optional H/V constraint. measured_pdf_length = axis-projected or Euclidean page-space distance in PDF default user units (1/72\"). Displayed value = measured_pdf_length × group.scale, formatted in group.units. Rendered as an authored /Line annotation with /IT /LineDimension: baked /AP leader + extension lines + value text (reuses Pass 6.1 geometry authoring + Pass 6.2 §12.7.3.3 vartext label generator).",
      "radius_diameter": "From a circle OBJECT (PDF circles are 4 kappa≈0.5523 Béziers, or an ellipse): fit a circle to the object node set. From MULTIPLE SELECTED NODES 'that make up a circular area (might be small line segments)': flatten any Béziers to sample points at tolerance, take segment endpoints (+midpoints if segment count is low) as fit samples, run a LEAST-SQUARES BEST-FIT circle.",
      "best_fit_algorithm": "PRIMARY = TAUBIN fit (algebraic, closed-form, no iteration). Chosen SPECIFICALLY because the operator's stated case is small line segments approximating an arc (partial-arc / short-arc data) — the exact regime where the simpler Kåsa fit is strongly biased in radius; Taubin is near-unbiased for partial arcs at nearly the same cost. Optional single Gauss-Newton geometric-refinement step if residual matters. Hand-rolled ~50–80 lines of linear algebra (a 3×3 generalized eigenproblem / closed form) — ZERO new dependency (rule 13). Report the FIT RESIDUAL (RMS node-to-circle distance) so the operator sees fit quality. The fitted circle (center + radius + residual) is drawn as a PREVIEW the operator accepts/rejects — fuzzy-never-sneaky; the best-fit circle is a reviewable hint, never a silent auto-apply. displayed radius = fitted_radius_pdf × scale; diameter = 2×.",
      "value_data_model": "GEOMETRY is immutable and stored (the measured pdf length / fitted radius). The DISPLAYED value is DERIVED = geometry × group.scale, in group.units, recomputed (and the /AP label regenerated, reusing the Pass 7.1 regenerate-appearances pattern) whenever the group's scale changes. This is what makes 'change the group scale → all member dimensions update' clean and cheap."
    },
    "D_scale_group_model_and_storage": {
      "group": "A named entity: { name, scale (f64 real-units-per-pdf-unit), units (mm|cm|m|inch|decimal-ft|ft-in), number-format (precision, suffix; ft-in gets feet+inches formatting), ocg (the group's Optional Content Group ref), members (dimension set) }. A default/active group exists so a dimension always has a home; dimensions are (re)assignable to named groups.",
      "scale_dimension_workflow": "Operator draws a line like the dim tool (pick A, pick B, snapped) → a special SCALE DIMENSION. Then EITHER (i) types the scale ratio directly, OR (ii) types the real length + units of that drawn line and pdfce back-calculates scale = real_length / drawn_pdf_length (the operator's exact words). The group receives this scale. Editable later; re-propagates. The real-length-entry path (ii) is the RECOMMENDED unambiguous path; the ratio path (i) needs a paper-unit basis ('1:100' means 1 paper-unit = 100 real-units of the same kind, and PDF paper units are 1/72\") — a definitional detail to source from §12.9 NumberFormat semantics; default paper-unit basis = inch, disclosed.",
      "hybrid_storage_answer_1": {
        "native_portable_layer": "Each dimension → a REAL PDF annotation portable to any reader: /Line with /IT /LineDimension (linear); radius/diameter as a /Line from center to rim with /IT /LineDimension + the sidecar carrying the radius/diameter semantics (the native measurement model is weak on radial dims, so the baked /AP shows the leader+value and the sidecar holds the 'radius/diameter of fitted circle C' meaning). The baked /AP shows the drawn dimension + measured value in ANY reader.",
        "native_scale": "Where a group maps cleanly to a single page region, author a native /Viewport with a /Measure /RL (rectilinear) dict carrying /X /Y /D /A + /NumberFormat (§12.9.3: /U unit label, /C conversion factor, /D precision) — so the SCALE is portable/interoperable in readers honoring §12.9. Overlapping groups with different scales on one page do NOT map to disjoint /BBox viewports, so native /Measure is a best-effort PORTABILITY PROJECTION, not the authority.",
        "sidecar_authority": "The pdfce private model — group defs (name/scale/units/format), dimension→group membership, best-fit-circle params+residual+contributing nodes, centerline-derivation provenance, scale-dimension record — is carried IN-PDF via §14.5 /PieceInfo (the spec-sanctioned home for private application data that survives round-trip through other editors), keyed to a pdfce private key. This keeps the hybrid a SINGLE self-contained PDF (no loose external file required; an optional external .pdfce-dims export can be a fast-follow). The SIDECAR is authoritative in pdfce; the native /Measure is the portability projection. On load, if native and sidecar disagree (a third-party editor changed one), DISCLOSE and prefer the sidecar (fuzzy-never-sneaky).",
        "ocg_layer": "One Optional Content Group (§8.11 /OCG) PER dimension group (so each group toggles independently), registered in the catalog /OCProperties with a /D default config; each dimension annotation carries /OC → its group's OCG. Toggleable in any OCG-honoring reader AND in pdfce. Because pdfce currently DEFERS /OC rendering, the beta adds a BOUNDED piece: honor the visibility of pdfce's OWN AUTHORED dimension OCGs — annotation-level /OC membership checked against /OCProperties /D (the annotation walk from Pass 6.0 already exists; add the /OC→OCG visibility test + a per-group GUI toggle). FULL content-stream BDC/EMC /OC visibility stays deferred (a named non-goal); only the authored-layer annotation /OC + /D default config are honored."
      }
    },
    "E_basic_vector_editing_answer_2": {
      "scope_minimum": "Exactly three operations, the minimum that makes the beta a real editor without becoming full Pass 9: (1) MOVE object — translate all of the object's path-construction operands by (dx,dy) in its user space (CTM-aware); (2) DELETE object — remove the object's construction+painting operators from the content stream; (3) DRAG NODE — rewrite one anchor's coordinate pair in the m/l/c operand list (anchor/corner move; adjacent control-point 'handle' editing is a fast-follow).",
      "mechanism": "All three are content-stream SURGERY via the Pass 8.0 advance-preserving interpreter: locate the object's operator byte range (from the object model's token-index range), rewrite/remove operands, re-emit ONLY that stream. Every other object stays byte-verbatim (R46 named exception, §5.7 — this is the mirror of redaction's operator removal). Compressed-object-stream objects are promoted/decomposed per §5.7/§5.9. Each edit is one undoable command (Pass 3.1 command log).",
      "excluded": "Boolean path ops, gradients/shading/transparency editing, numeric matrix transforms, align/distribute, z-order, group/ungroup, text-to-path, in-place text edit, image resample — ALL stay full Pass 9.",
      "oracle_dependency": "Move/delete/drag-node RE-RENDER the edited page → this is the first subsystem whose acceptance oracle is independent visual fidelity → it rides Pass 11's (C's) render-fidelity gate (R59). If C has not landed when the beta ships, 9c-min ships spot-checked-with-disclosure per decision 010 revisit-trigger 3 (a real, disclosed risk). The additive dimensioning half has NO such dependency."
    },
    "F_canvas_substrate_R60": {
      "confirm_one_substrate": "YES — the beta builds decision 010's Pass 12 FIRST SLICE, the ONE R60 canvas-interaction substrate, NOT a throwaway measurement-only canvas: a focusable canvas (Sense::click_and_drag, resolving main.rs's Pass-1 focusable caveat), viewer::screen_to_page/page_to_screen (page-space geometry, rotation-correct via current_extent), a tool-mode dispatch state machine, a hit-test/selection scaffold with a PLUGGABLE target provider (the object model plugs in as the provider), and a live-preview overlay. It is built generically so the dimension tools, the vector-editing move/node-drag, AND the three deferred editing GUIs (markup/form-fill/redaction) all layer onto the SAME widget. A second parallel canvas-interaction path is forbidden (R60).",
      "note": "This is the beta's justification for building the real substrate now: the operator's first usable tool doubles as the substrate every other editing GUI has been waiting for. pdfce-ui-specialist designs the substrate interaction taxonomy + the dimension tool UX (it owns the five-way placement taxonomy)."
    }
  },

  "pass_slicing": [
    {
      "pass_id": "Pass 12.0 (proposed; the R60 substrate slice of decision 010's Pass 12 — librarian confirms)",
      "name": "Canvas-interaction substrate (built once, R60)",
      "deliverables": ["Focusable canvas Response (Sense::click_and_drag), resolving main.rs's Pass-1 focusable-canvas caveat", "viewer::screen_to_page / page_to_screen (page-space geometry storage, rotation-correct via current_extent)", "Tool-mode dispatch state machine (pointer events → active tool)", "Hit-test/selection scaffold with a pluggable target provider + a selection set model", "Live-preview overlay painting layer + drag-vs-pan suppression"],
      "acceptance_criteria": ["Canvas focusable + receives click/drag; main.rs Pass-1 caveat closed", "screen↔page transform correct under page rotation + zoom (test at 0/90/180/270°)", "Tool-mode dispatch routes to a no-op default tool with zero behavior change to existing viewer", "Selection scaffold selects nothing until a target provider is attached (9a attaches it)", "Standing gates green (fmt, clippy -D warnings, GUI-free cargo tree core+render, wasm32, --duplicates, no-network, ui-strings); R34/R46 unmoved; ZERO new deps"],
      "non_goals": ["No object model (9a)", "No tools that mutate the document", "Not the three deferred editing GUIs (fast-follow on this substrate)"],
      "prerequisites": ["pdfce-ui-specialist: substrate interaction taxonomy + tool-mode/selection UX (BLOCKING design input)"],
      "render_touching": false,
      "risks": [{"id": "Z1", "risk": "Substrate built too narrow (measurement-only) and later refactored for the deferred GUIs — violates R60's 'build once'.", "mitigation": "ui-specialist designs it generically against ALL four consumers (dimensions, vector-edit, markup, form-fill/redaction) up front; the target provider + tool-mode are the extension seams."}]
    },
    {
      "pass_id": "Pass 9a (decision 008 §5.3 slice (a) / Pass 9 slice, PULLED FORWARD into the beta per decision 010 revisit-trigger 3; keeps the Pass 9 ID family)",
      "name": "Vector object/selection model + centerline derivation",
      "deliverables": ["pdfce-core read-only content-token decomposition → selectable path objects (nodes in user+page space, effective graphics state, content-token index range)", "Hit-test geometry for path/text/image objects; attaches as Pass 12.0's target provider", "Centerline: stroked-path geometry = centerline (no-op); thin-filled-rectangle/quad midline DERIVATION as a fuzzy, disclosed, operator-confirmed hint", "Selection UI on the substrate (click, marquee, add/remove)"],
      "acceptance_criteria": ["Decomposition is byte-INERT (read-only; R46 unaffected — proven by re-running the content-identity gate: no change)", "Object node geometry matches the rendered geometry (cross-check against pdfce-render's own walk on fixtures)", "Filled-rectangle centerline derivation flagged + confirmable, never auto-applied (fuzzy-never-sneaky)", "Fuzz target over the decomposition (malformed/degenerate paths, huge node counts, unbalanced q/Q) 0 crashes", "Standing gates green; ZERO new deps"],
      "non_goals": ["No editing (9c-min)", "No node-editing of text/image objects", "No boolean/gradient decomposition"],
      "prerequisites": ["Pass 12.0", "pdfce-inkscape-librarian: object/node-selection capability bucket (grounds selection semantics) — dispatch now, agent+RAG scaffold already exist"],
      "render_touching": false,
      "risks": [{"id": "Z2", "risk": "Object segmentation disagrees with the render walk (two different decompositions) — the geometry analogue of a second pipeline.", "mitigation": "Reuse the SAME content-token walk + gstate tracking pdfce-render uses; cross-check object geometry against the render path on fixtures as an acceptance gate."}, {"id": "Z3", "risk": "Filled-rectangle centerline false positives on genuinely rectangular fills.", "mitigation": "Aspect-ratio threshold + operator confirmation; never auto-commit; disclose the derivation."}]
    },
    {
      "pass_id": "Pass 12.M1 (proposed; a substrate service slice)",
      "name": "Snapping engine",
      "deliverables": ["Snap targets (nodes, endpoints, centers incl. best-fit, midpoints, intersections, on-segment projection) over 9a's object geometry", "Priority ordering + zoom-invariant screen-space tolerance", "H/V alignment constraint (axis projection)", "Fuzzy snap indicator (marker + type label) shown pre-commit; cycle/override"],
      "acceptance_criteria": ["Snap 'feel' zoom-invariant (screen-space tolerance)", "Priority + nearest tie-breaking deterministic + tested", "H/V constraint projects to page X/Y axis correctly under rotation", "Snap indicator discloses the inferred target before commit (fuzzy-never-sneaky)", "Shared service usable by BOTH the dimension tools and 9c-min node-drag; standing gates green; ZERO new deps"],
      "non_goals": ["No dimension creation (12.M2)", "No object editing (9c-min)"],
      "prerequisites": ["Pass 9a", "pdfce-inkscape-librarian: snapping capability bucket (snap targets/priority) — same dispatch as 9a"],
      "render_touching": false,
      "risks": [{"id": "Z4", "risk": "Intersection computation cost on dense pages.", "mitigation": "Only compute intersections among segments within the screen-space tolerance neighborhood (spatial bucket), not globally."}]
    },
    {
      "pass_id": "Pass 12.M2 (proposed; the beta's headline new capability)",
      "name": "Dimensioning subsystem + scale/group model + hybrid storage + OCG layer",
      "deliverables": ["Linear dimensions (point/node/line to point/node/line, H/V or aligned)", "Radius/diameter from a circle object OR multiple nodes via TAUBIN best-fit circle (+ optional Gauss-Newton refine), residual reported, fitted-circle preview accepted/rejected (fuzzy-never-sneaky)", "Named dimension groups: scale + per-group units (mm/cm/m/inch/decimal-ft/ft-in) + number format; dimension→group assignment", "Scale-dimension workflow: draw line → type ratio OR real-length+units → back-calc scale = pdf-length/real-length; editable group scale re-propagates (regenerate /AP labels, Pass 7.1 pattern)", "HYBRID storage: native /Line + /IT /LineDimension annotations with baked /AP (Pass 6.1 geometry + 6.2 vartext); native /Viewport//Measure //RL + /NumberFormat scale where it round-trips; authoritative §14.5 /PieceInfo sidecar (group/scale/units/best-fit/centerline provenance)", "Per-group §8.11 /OCG layer + /OCProperties /D config; honor authored-layer annotation /OC visibility in pdfce render + per-group GUI toggle", "CLI: dimension-add / dimension-list / group-set-scale / layer-toggle subcommands (parity-plus, same discipline as the GUI)"],
      "acceptance_criteria": ["Value = measured_pdf_length × group.scale in group.units, correct across all six unit modes (ft-in formatting tested)", "Scale-dimension back-calc correct for both entry paths; scale change re-propagates + regenerates labels", "Best-fit circle near-unbiased on a synthetic short-arc fixture (Taubin beats Kåsa — proven by test); residual surfaced; fitted circle is a confirmed hint", "Dimensions ADDITIVE (overlay-append, §5.8): existing page content byte-verbatim → R46 GATE PASS with zero new divergence (self-comparison oracle sufficient; C NOT required)", "Portable: the baked /AP dimension + value render in pdfium (via the reference harness) and the value text is present", "Authored OCG toggles visibility in pdfce render (authored-layer /OC honored) and carries /OC in the file for other readers", "Sidecar survives a save round-trip via /PieceInfo; on native-vs-sidecar disagreement, disclosed + sidecar preferred", "R59 render-fidelity re-run (render-touching: authored /AP + OCG visibility); standing gates green; ZERO new deps (Taubin hand-rolled)"],
      "non_goals": ["Per-group takeoff totals/CSV (answer 4 — fast-follow)", "Area/perimeter measurement (fast-follow)", "Associative dimensions that re-measure on object edit (fast-follow; beta dims are static)", "FULL content-stream BDC/EMC /OC visibility (only authored-layer annotation /OC + /D honored)", "No content-stream surgery (dimensions are additive)"],
      "prerequisites": ["Pass 9a + 12.M1", "pdfce-spec-librarian: §12.9 (/Measure //Viewport //RL //NumberFormat, /IT /LineDimension //PolyLineDimension), §14.5 (/PieceInfo private app data), §8.11 (/OCProperties //OCG //OCMD //OC, /D default config) — BLOCKING for storage+layer design", "pdfce-acrobat-librarian: Measuring-Tool capability bucket (distance/scale ratio/snap-to-content/markup persistence) — grounds acceptance criteria", "pdfce-ui-specialist: dimension tool UX (tool modes, group panel, scale-dimension entry dialog, best-fit-circle confirm, layer toggle)"],
      "render_touching": true,
      "risks": [{"id": "Z5", "risk": "Best-fit circle biased/unstable on very short arcs.", "mitigation": "Taubin (chosen for exactly this regime) + optional geometric refine; report residual; operator confirms the preview."}, {"id": "Z6", "risk": "Scale-ratio entry ('1:100') ambiguous without a paper-unit basis.", "mitigation": "Prefer the real-length-entry path; default ratio basis = inch (1 unit=1/72\"), disclosed; source §12.9 NumberFormat semantics via spec-librarian."}, {"id": "Z7", "risk": "OCG scope creep into full content-stream /OC BDC/EMC (the deferred render gap).", "mitigation": "Bind to authored-layer annotation /OC + /D config only; content-stream /OC stays a named non-goal."}, {"id": "Z8", "risk": "Native /Measure and sidecar diverge after a third-party edit.", "mitigation": "Sidecar authoritative; native is a portability projection; disclose disagreement on load, prefer sidecar (fuzzy-never-sneaky)."}, {"id": "Z9", "risk": "§12.9/§14.5/§8.11 not yet in the spec RAG.", "mitigation": "BLOCKING spec-librarian dispatch NOW, in parallel with 12.0/9a/12.M1 build."}]
    },
    {
      "pass_id": "Pass 9c-min (decision 008 §5.3 slice (c), MINIMAL subset, PULLED FORWARD; keeps the Pass 9 ID family)",
      "name": "Basic vector editing — move / delete object, drag node",
      "deliverables": ["Move object (translate all construction operands, CTM-aware)", "Delete object (remove construction+painting operators)", "Drag node (rewrite one anchor coordinate)", "All via Pass 8.0 advance-preserving surgery interpreter; re-emit ONLY the edited stream; promote/decompose compressed containers (§5.7/§5.9); one undoable command each; snap-on-drag via 12.M1", "CLI: object-move / object-delete / node-move subcommands"],
      "acceptance_criteria": ["Only the edited content stream changes; every other object byte-verbatim (R46 named exception, proven by the content-identity gate showing exactly one changed stream)", "Compressed-object-stream objects promoted/decomposed correctly (no stale copy, §5.7)", "R59 render-fidelity gate PASS on edited pages against pdfium (the surgery re-render is visually correct) — OR, if C not yet landed, SHIP spot-checked-with-disclosure per decision 010 revisit-trigger 3", "Undo restores byte-identical pre-edit state (Pass 3.1 command log)", "Fuzz over operand rewriting (degenerate coords, huge operands) 0 crashes; standing gates green; ZERO new deps"],
      "non_goals": ["Bézier control-point/handle editing (fast-follow)", "Boolean/gradient/transform/align/z-order/group/text-to-path (full Pass 9)", "Text/image object node editing"],
      "prerequisites": ["Pass 9a + 12.0 + 12.M1", "Pass 11 (C) render-fidelity gate — required for the oracle, OR the disclosed spot-check per revisit-trigger 3"],
      "render_touching": true,
      "risks": [{"id": "Z10", "risk": "Node-drag surgery re-renders wrong without an independent oracle (the exact decision-010 'building on sand' risk).", "mitigation": "Ride Pass 11's (C's) R59 gate; if C not landed, ship spot-checked-with-explicit-disclosure (revisit-trigger 3), never silently."}, {"id": "Z11", "risk": "Operand rewrite changes byte length → perturbs downstream offsets in the same stream.", "mitigation": "The Pass 8.0 interpreter already handles operator-level in-stream edits with position preservation; re-emit the whole edited stream (only that one), never patch bytes in place across the file."}]
    }
  ],

  "prerequisites_to_dispatch_in_parallel_now": [
    "pdfce-spec-librarian — §12.9 measurement model (/Measure //Viewport //RL, §12.9.3 /NumberFormat /U /C /D, /IT /LineDimension //PolyLineDimension), §14.5 /PieceInfo (private app data round-trip), §8.11 optional content (/OCProperties //OCG //OCMD //OC annotation entry, /D default config, BDC/EMC scope). BLOCKING for Pass 12.M2. Dispatch immediately.",
    "pdfce-acrobat-librarian — Measuring-Tool / dimension capability bucket (distance + scale ratio + snap-to-content + measurement-markup persistence + units). Grounds Pass 12.M2 acceptance criteria in real Acrobat behavior. Dispatch now.",
    "pdfce-inkscape-librarian — object/node selection + snapping capability bucket (node tool, snap targets & priority, bbox-vs-node snap). Grounds Pass 9a + 12.M1. Agent + Inkscape_Features RAG scaffold already exist. Dispatch now.",
    "pdfce-ui-specialist — (i) the R60 substrate interaction taxonomy (Pass 12.0), (ii) the dimension tool UX (tool modes, snap indicator, group panel, scale-dimension entry, best-fit-circle confirm, per-group layer toggle) (Pass 12.M2). Dispatch when 12.0/12.M2 are scoped.",
    "Pass 11 (C) render-fidelity harness — ALREADY IN PROGRESS. No new dispatch; Pass 9c-min consumes its R59 gate."
  ],

  "invariants_honored": {
    "round_trip_minimal_diff": "Dimensioning is ADDITIVE (overlay-append §5.8, R46 zero-exception). Editing is the R46/§5.7/§5.9 NAMED surgery exception (mirror of redaction): only the edited stream re-emits; compressed containers promoted/decomposed; everything else byte-verbatim. Object-model decomposition (9a) is byte-INERT.",
    "fuzzy_never_sneaky": "Every inference is a reviewable hint: the best-fit circle (with residual) is previewed + confirmed; the snap target is shown pre-commit with its type label; the filled-rectangle centerline derivation is disclosed + confirmed; native-vs-sidecar storage disagreement is disclosed on load. Nothing auto-applies.",
    "R59_render_fidelity_gate": "Re-run on the render-touching slices (12.M2 authored /AP + OCG visibility; 9c-min surgery re-render). Additive-only slices (12.0/9a/12.M1) are not render-touching.",
    "R60_one_canvas_substrate": "The beta builds the single shared substrate; dimensions, vector-edit, and the three deferred GUIs all layer on it. No second interaction path.",
    "R61_inkscape_behavioral_reference_only": "Snapping/node-selection behavior is sourced from the Inkscape capability RAG (behavior/limits only, GPL-2.0 — never a dependency, code source, or GUI mimicry); UI designed independently by pdfce-ui-specialist.",
    "rule_13_no_new_copyleft_deps": "ZERO new dependencies across all five slices. Taubin best-fit, snapping, object model, XML-free storage all hand-rolled (consistent with the project's ZERO-new-dep posture through Pass 8).",
    "gui_core_separation": "Object model, snapping math, dimension geometry, best-fit, storage all in pdfce-core (GUI-free); only the tool UX + substrate in pdfce-gui. cargo tree core+render stays egui/eframe/winit/wgpu-free."
  },

  "revisit_triggers": [
    "Pass 11 (C) surfaces a systematic render divergence pdfce cannot cheaply reconcile → re-decide before 9c-min ships (the additive measurement half is unaffected and can still ship).",
    "Operator wants per-group takeoff totals/CSV (answer 4 was 'labels only for now') → promote the fast-follow totals slice.",
    "Operator wants area/perimeter measurement or associative (re-measuring) dimensions → scope as a follow-on measurement Pass.",
    "The §12.9 native /Measure model proves too weak for radius/diameter interop → the sidecar already carries the semantics; revisit whether to author a richer native representation.",
    "Any beta slice hits the three-attempts wall → decision 010's fallback (Pass 5 encryption) remains the designated switch track."
  ],

  "operator_actions_owed_resurfaced_not_new": [
    "Encryption-refusal sign-off (oldest owed, now across decisions 007/008/009/010).",
    "License decision (LEGAL §1) — gates the public repo/release and the GPL Inkscape-reference boundary; the beta is still a private build until this lands (rule 8).",
    "Commit authorization — Passes 0–8 ALL uncommitted; this beta adds more unversioned work (W15/X16).",
    "/R6 AES-256 sourcing + LEGAL §2 Adobe-supplement copyright contradiction (Ken's calls when Pass 5 activates)."
  ]
}
```

---

## Orchestrator note (2026-08-01, at archival)

Decision 011 archived. It is the architecture + Pass-slicing for the operator's FIRST BETA — a scaled measurement/dimensioning tool. Five slices: 12.0 (canvas substrate, R60 built-once), 9a (object/selection model + centerline), 12.M1 (snapping), 12.M2 (dimensioning + scale/group + hybrid storage + per-group OCG layer), 9c-min (basic editing: move/delete/drag-node). Authorized by decision 010 revisit-trigger 3 (operator-demanded vector editing after B); does NOT reorder decision 010's C->B->A, repackages its front into the beta. KEY SPLIT: dimensioning is additive (overlay-append, oracle-sufficient WITHOUT Pass 11/C); editing is content-stream surgery (needs C's render-fidelity gate R59). Because Pass 11 (C) SHIPPED before this archival (render-fidelity harness complete, R59 gate live), all five slices can ship together — the two-step fallback is moot. Storage = hybrid: native /Line + /IT /LineDimension annotations + best-effort native /Viewport//Measure//RL scale + AUTHORITATIVE pdfce sidecar via §14.5 /PieceInfo (in-PDF, self-contained) + per-group §8.11 /OCG layer with pdfce honoring authored-layer annotation /OC visibility only (full content-stream BDC/EMC /OC stays deferred). Best-fit circle = Taubin (chosen for the short-arc/small-segment regime). Zero new deps (Taubin/snapping/object-model/storage hand-rolled). At archival: the operator was shown the plan for confirmation before the build; the parallel research prerequisites (spec-librarian §12.9/§14.5/§8.11 [blocking for 12.M2], acrobat-librarian measuring-tool bucket, inkscape-librarian selection+snapping bucket) were dispatched; the ui-specialist design + the engineer implementation passes + the ROADMAP filing await the operator's confirmation. Operator answers that shaped it: hybrid-on-own-layer storage; measure+basic-editing scope; per-group units; labels-only (no takeoff totals in the beta).
