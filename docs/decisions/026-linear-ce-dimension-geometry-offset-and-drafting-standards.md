# Decision 026 — Linear ce-dimension geometry: the axis-aligned dimension line, the offset that makes extension lines possible, and ANSI-vs-ISO drafting standards

**Status:** Decided in part; **one question is deliberately NOT decided and is put to the
operator** (§4.7). Consultant recommendation; engineer to schedule, librarian to file.
**Date:** 2026-08-04
**Requested by:** `pdfce-engineer`, on the operator's report of 2026-08-04 (§1.0, verbatim).

**Amends:** decision **011** (§2.3's linear-dimension value model — *extends* it with a
placement term; the geometry-stored/display-derived split is preserved intact, not
reversed) and decision **022** (§4.2/§4.3 — this record adds a *third* class of dimension
gesture that 022's two-way split did not contemplate: a drag that changes neither the
measured points nor the measured value). **Neither file is edited** —
`docs/decisions/README.md` is explicit that records are append-only history. The
amendment table is §9.3. The librarian owes forward-references from 011's and 022's
`ARCHITECTURE.md` §12 ledger entries to this record.

---

## Ledger verification (R106)

**This worktree is NOT stale, and that is worth stating because the last two records were.**
Decision 025 opened with a section on the checker reporting ceilings that had already moved.
That failure mode was checked for here and did not occur:

```
worktree HEAD          : b3474b8  "Pass 25.6: delete ce dimensions"
pass-8-redaction HEAD  : b3474b8  "Pass 25.6: delete ce dimensions"   <- identical
```

The isolated worktree `agent-a92b79fb0e8196d2b` is checked out at the *same commit* as the
live branch, so `python tools/check-ledger-numbers.py --stats` run inside it reads the
live `docs/ROADMAP.md`. Its output, verbatim:

```
  Pass families with headings : up to 25 (highest ID 25.1)
  CLAIMED BUT NOT YET HEADED  : 5, 9, 9c, 10, 13, 20, 22, 23, 24
  standing rules      : R129  -> next free is R130
  decision records    : 025 -> next free is 026
```

`docs/ROADMAP.md`'s open-operator-question letters run to **(aj)**.

**The one thing the checker cannot see** is what unfiled decision records have already
*claimed*. Decision 025 (`docs/decisions/025-*.md`, written earlier today, not yet filed by
the librarian) claims **Pass family 26**, standing rules **R130–R134**, and open questions
**(ak)–(ap)**. Decision 024 claims **Pass 24.0–24.5**. Both were read directly to avoid a
collision the checker would not have caught.

| Ledger | Live ceiling | Claimed by unfiled records | **This record claims** |
|---|---|---|---|
| Decision records | 025 | — | **026** |
| Pass families | headed to 25; 24 claimed-not-headed | **26** (decision 025) | **family 27** — Passes 27.0–27.3 |
| Standing rules | R129 | **R130–R134** (decision 025) | proposes **R135–R139** (librarian assigns finally) |
| Open operator questions | (aj) | **(ak)–(ap)** (decision 025) | claims **(aq)–(au)** |

---

## 0. Summary

The operator reports that a ce dimension set to the **Horizontal** constraint reads the
correct horizontal value but is *drawn at an angle*, that dragging one does not keep it
aligned with what it measures, and that ce dimensions should follow ANSI (his default) or
ISO drafting practice.

**The defect is real, it is three lines of code, and its shape is worse than "the drawing
is wrong": pdfce shows the operator a correct preview and then bakes a different thing.**

Seven findings drive this record. Findings 1–6 were verified in the shipped code at
`b3474b8`, not inferred; finding 7 was researched from live standards sources with every
claim confidence-marked (§5.0).

1. **`author.rs::leader_endpoints` returns the two picked points verbatim, and
   `draw_linear` strokes straight between them** (`author.rs:228-236`, `author.rs:239-266`).
   The `AxisConstraint` is consumed by `measured_length` and by nothing else in the
   appearance path. §1.1.

2. **★ The root cause is not "the drawer forgot the constraint" — it is a deliberate,
   documented preview/commit divergence.** `LinearPick::preview_segment` draws the
   *constrained* segment (`measure_tool.rs:146-149`), while `LinearPick::commit_point`
   stores the **raw** second pick (`measure_tool.rs:112-127`), justified in its own module
   doc as "byte-equivalence" with the CLI. The operator is shown a horizontal line, clicks,
   and gets a diagonal one. The stored-raw decision is **correct and is kept** — the raw
   point is the *anchor of the second extension line*, which is exactly what the fix needs
   — but the appearance path was never taught that `b` is a measured point rather than a
   dimension-line endpoint. §1.2.

3. **There are no extension lines. There never were.** `draw_linear` emits a 4-point
   perpendicular *tick* centred on each endpoint (`author.rs:252-261`, `tick = 4.0`). A
   tick is not a witness line: it does not reach anything, it cannot span a gap, and it
   makes the second half of the operator's sentence ("extend the lines to the connection
   points") unimplementable without new stored state. §1.4.

4. **The missing state is one signed scalar.** `DimensionKind::Linear` stores where the
   dimension *was measured* and nothing about where the dimension *line sits*. Adding
   `offset: f64` — signed, page-space points, along a canonicalised normal, based at `a` —
   is sufficient for every case in the report. Critically, **`offset = 0.0` reproduces
   today's committed geometry exactly for an already-axis-aligned pick, and reproduces the
   PREVIEW the operator was shown for every pick**, which is what makes the migration free
   and closes finding 2 in the same stroke. §3.

5. **★ `sidecar.rs` has a latent data-loss cliff that this Pass must not walk into.**
   `deserialize_model` gates on `Version == SIDECAR_VERSION` with **exact equality**
   (`sidecar.rs:67-69`); any mismatch returns `None`, the caller starts a fresh model, and
   **every group, every calibrated scale, every membership is silently gone** while the
   `/Line` annotations remain on the page looking fine. A naive "bump to version 2" would
   destroy the calibration work of every existing dimensioned file. The offset is therefore
   added as an **optional key at version 1** (additive, exactly the forward-compat pattern
   the module's own header claims), and the gate is separately widened to a range so the
   *next* change has somewhere to go. §3.6.

6. **The drag semantics cannot be decided from the operator's sentence, because its two
   halves point opposite ways.** "Only be able to drag it horizontally" and "so it stays in
   line with what it is actually measuring" are, under the model he is asking for, mutually
   exclusive: the along-axis drag is a geometric no-op, and the perpendicular drag is the
   one that sets the offset. Both readings are laid out, a recommendation is given with its
   reasoning, and **the exact question is written out for the operator rather than
   silently resolved** (§4.7). This is the one item this record refuses to decide alone.

7. **★ "ANSI" is the wrong standard for most of what pdfce draws, and ISO mandates more
   than expected.** The natural assumption — ASME Y14.5 — is wrong: Y14.5 is the
   GD&T/tolerancing standard, and arrowheads, line conventions and lettering live in
   **ASME Y14.2-2014**, whose §4/§5/§6 structure was verified from ASME's own published
   front matter. On the ISO side, three rules that could easily have been treated as
   convention are verified **"shall"** clauses of ISO 129-1:2018: text above an *unbroken*
   line (cl. 4.1.1), vertical text reads *from the right* with orientation *"determined at
   the centre of the dimension"* (cl. 4.1.1 — and the widely-taught "30° ambiguous zone" is
   **not in the standard**), and **a comma as the decimal marker** (cl. 4.1.1). The comma
   forced a design revision: it is expressible portably via §12.9 Table 263's `/RD`, so it
   is governed by the standard *without* breaking the `/Measure` agreement contract — but
   only if `/RT` is set alongside it, or every grouped number reads `1,234,56`. §5.

Plus one side-finding outside the report's scope, recorded because it is in the same
family and would otherwise stay invisible: **pdfce's own dimension label and a
§12.9-honouring reader can already disagree today** — pdfce prints `3.10 m` (fixed places,
`units.rs:283-285`) while the mirrored `/Measure` dict omits `/FD`, whose default `false`
permits a conforming reader to truncate the low-order zero to `3.1 m`
(`measure_dict.rs:115-123`; ISO 32000-1 §12.9 Table 263). The doc comment on
`NumberFormat::format` claims the two "agree by construction." They do not. §5.5.

---

## 1. Root cause — verified, not inferred

### 1.0 The operator's report, verbatim

> "Also when I mve a ce dimension it should extend the lines to the connection points and
> stay algined with them - ie if it is a horizontal dimension I should only be able drag it
> horizontally so it stays in line with what it is actually measuring. Also when horizontal
> or vertical is selected the dimension line should show in the appropriate direction. It
> looks like it give me the correct horizontal or vertical dimension but it shows at an
> angle. These should be able to follow ANSI and ISO standards. My default is ANSI, but ISO
> should be an option too."

Four distinct requests are in there, and they are not equally hard:

| # | Request | Difficulty |
|---|---|---|
| R1 | A Horizontal/Vertical ce dimension must be **drawn** along that axis | Three lines. §2. |
| R2 | Moving one must **extend the lines to the connection points** | Needs new stored state. §3. |
| R3 | Dragging must **stay aligned with what it measures** | Semantics ambiguous. §4. |
| R4 | ANSI (default) and ISO must both be available | New per-group field + a real drawing rework. §5, §6. |

### 1.1 The three-line chain

```
author.rs:146   let (l0, l1) = leader_endpoints(kind);

author.rs:228   fn leader_endpoints(kind: &DimensionKind) -> (Point, Point) {
author.rs:230       DimensionKind::Linear { a, b, .. } => (a, b),      // <- constraint discarded

author.rs:159   draw_linear(&mut b, &mut bounds, l0, l1);

author.rs:239   fn draw_linear(b, bounds, a: Point, c: Point) {
author.rs:246       b.move_to(a.x, a.y);
author.rs:247       b.line_to(c.x, c.y);                                // <- straight a -> b
```

`leader_endpoints` pattern-matches `DimensionKind::Linear { a, b, .. }`. **The `..` is the
bug**: it discards `constraint`. Note also `author.rs:157-161`, where the `Linear` arm
binds `a` only to immediately `let _ = a;` — a dead binding that is itself a small signal
that this arm was written around the constraint rather than through it.

The `/L` annotation array (`author.rs:191-199`) is populated from the same `(l0, l1)` pair,
so the *machine-readable* leader is wrong in the same way as the drawn one. A
`/IT /LineDimension` consumer reading `/L` gets the diagonal too.

### 1.2 ★ The framing that matters: this is a preview/commit divergence

It would be easy to file this as "the appearance drawer ignores the constraint" and patch
`leader_endpoints`. That reading misses the more serious property of the defect.

`measure_tool.rs` deliberately keeps two different segments:

```
measure_tool.rs:146   pub fn preview_segment(&self, raw: Point) -> Option<(Point, Point)> {
measure_tool.rs:147       self.first
measure_tool.rs:148           .map(|a| (a, constrained_second_point(a, raw, self.constraint)))

measure_tool.rs:112   pub fn commit_point(&mut self, p: Point) -> Option<DimensionKind> {
measure_tool.rs:120           Some(DimensionKind::Linear { a, b: p, constraint: self.constraint })
                                                              ^^^^ RAW, not projected
```

and says so in its own module doc (`measure_tool.rs:75-81`):

> **Raw-second-point storage (byte-equivalence, module docs):** the authored `b` is the RAW
> snapped second pick, exactly as the CLI stores it; […] The on-canvas preview segment is
> the *constrained* line — display only, so "what you see is what's measured" (ui-spec §2.5)
> without diverging the stored geometry from the CLI's.

The stated goal — "what you see is what's measured" — **is met for the value and violated
for the drawing**, and the module doc does not notice the second half. The operator moves
the pointer, sees a horizontal preview line, clicks, and a diagonal line appears. In a
*measurement* product that is the same category of harm as rule 4's "fuzzy, never sneaky":
the operator is shown one thing and given another.

**The raw-`b` decision is nonetheless correct and this record keeps it.** Under the
corrected model, `a` and `b` are the **measured points** — the anchors the extension lines
run back to — and the dimension line is *derived*. Projecting `b` at commit time would
throw away exactly the information R2 needs. The bug is not that `b` is raw; the bug is
that the appearance path treated `b` as if it were a dimension-line endpoint.

### 1.3 What is NOT wrong

The **value** is correct and stays correct. `measured_length` (`snap.rs:405-411`) honours
the constraint (`Horizontal` ⇒ `|Δx|`, `Vertical` ⇒ `|Δy|`, `Aligned` ⇒ Euclidean), and
`DimensionKind::measured_points` (`group.rs:153-164`) routes through it. The operator's own
observation — *"it gives me the correct horizontal or vertical dimension but it shows at an
angle"* — is exactly right, and no part of this record changes the value model.

Nothing in decision 011 §2.3 is reversed. Geometry is stored; the displayed value is
derived; a scale change re-runs `author_dimension` for every member. The offset added in §3
is stored geometry of the same kind and participates in the same regeneration path.

### 1.4 There are no extension lines

```
author.rs:252   // Extension ticks perpendicular at each end.
author.rs:253   for &end in &[a, c] {
author.rs:254       let t0 = Point::new(end.x - px * tick, end.y - py * tick);
author.rs:255       let t1 = Point::new(end.x + px * tick, end.y + py * tick);
```

with `tick = 4.0` (points, `author.rs:243`). These are 8-pt cross-ticks straddling each
endpoint, perpendicular to whatever direction `a → b` happened to run. They are not witness
lines in any drafting sense:

- they do not **reach** anything (fixed 4 pt, not spanning to the measured point);
- they are **centred on the endpoint**, so there is no gap from the object and no
  overshoot past the dimension line — both of which are defining features of an extension
  line under either standard;
- with offset 0 and an axis-aligned pick they are visually indistinguishable from a
  correct-but-tiny witness line, which is why this has not been noticed before now.

R2 ("extend the lines to the connection points") is therefore **not a tuning change**. It
requires (a) a dimension line that is somewhere other than through the measured points, and
(b) real extension lines with a gap and an overshoot. (a) is the new stored state; (b) is
new drawing code. Both are §2 and §3.

---

## 2. Q1 — The corrected geometry

### 2.1 The model

Four kinds of line participate, and pdfce currently conflates the first two:

| Element | What it is | Today |
|---|---|---|
| **Measured points** `a`, `b` | Where the operator picked. Anchors of the extension lines. Determine the value. | stored ✓ |
| **Dimension line** | The line carrying the terminators and the value text. Parallel to the measurement axis, offset from the feature. | conflated with `a → b` ✗ |
| **Extension (witness) lines** | Thin lines from near each measured point out to (and slightly past) the dimension line. | 8-pt ticks, not witness lines ✗ |
| **Terminators** | Arrowheads or oblique strokes at the dimension line's two ends. | present, outward-pointing ✓ |

### 2.2 Definitions

All page space (PDF default user space, 1/72", y increasing upward). The GUI feeds page
space through `viewer::canvas_to_pdf_space`, so everything below is `/Rotate`-correct by
construction — the same argument `snap.rs:336-343` already makes for the constraint.

Given `DimensionKind::Linear { a, b, constraint, offset }` and `w = b − a`:

**Axis unit vector `u`** — the direction the measurement runs:

| `constraint` | `u` |
|---|---|
| `Aligned` | `normalize(w)`; `(1, 0)` if `\|w\| ≈ 0` (matches `unit_vector`'s existing degenerate answer, `author.rs:331-340`) |
| `Horizontal` | `(1, 0)` — always, independent of pick order |
| `Vertical` | `(0, 1)` — always, independent of pick order |

**Canonical offset normal `n`** — `perp_ccw(u) = (−u.y, u.x)`, then **negated if it points
into the lower half-plane**, i.e. if `n.y < 0`, or `n.y == 0 && n.x < 0`.

> Why canonicalise. Without it, `n` for `Aligned` flips when the operator picks
> right-to-left, so the *sign* of a stored offset would depend on pick order and an
> otherwise-identical pair of ce dimensions would place their lines on opposite sides.
> Canonicalising also makes the two axis cases read the way a human expects: **positive
> offset is "up" for a horizontal ce dimension and "right" for a vertical one.** For
> `Vertical`, `perp_ccw((0,1)) = (−1,0)` points left and is negated to `(1,0)`; this is the
> one place where `n ≠ perp_ccw(u)`, and it is deliberate. It must be a named function with
> its own test, not an inline expression, because "which way is positive" is exactly the
> kind of thing that gets re-derived inconsistently in two places (R92).

**Offset vector** `O = offset · n`. Note `O ⟂ u` always.

**Signed axis span** `t = w · u`. (`Horizontal` ⇒ `b.x − a.x`; `Vertical` ⇒ `b.y − a.y`;
`Aligned` ⇒ `|w|`.) Let `su = sign(t)`, with `su = 0` treated as degenerate (§2.5).

**Signed normal separation** `h = w · n` — how far `b` sits off the axis through `a`.
(`Horizontal` ⇒ `b.y − a.y`; `Vertical` ⇒ `b.x − a.x`; `Aligned` ⇒ `0`.)

### 2.3 The corrected geometry, compact

| Element | Formula | `Aligned` | `Horizontal` | `Vertical` |
|---|---|---|---|---|
| Dimension-line end at `a` | `Pa = a + O` | `a + offset·n` | `(a.x, a.y + offset)` | `(a.x + offset, a.y)` |
| Dimension-line end at `b` | `Pb = a + O + t·u` | `b + offset·n` | `(b.x, a.y + offset)` | `(a.x + offset, b.y)` |
| Dimension line | `Pa → Pb`, broken for ANSI (§5) | along `w` | horizontal at `y = a.y + offset` | vertical at `x = a.x + offset` |
| Extension line at `a` | signed span `sa = offset` along `n`, from `a` | ✓ | vertical, from `a` up/down to the line | horizontal, from `a` left/right to the line |
| Extension line at `b` | signed span `sb = offset − h` along `n`, from `b` | span `= offset` (since `h = 0`) | span `= offset − (b.y − a.y)` | span `= offset − (b.x − a.x)` |
| Terminator at `Pa` | points along `−su·u` (outward) | ✓ | ✓ | ✓ |
| Terminator at `Pb` | points along `+su·u` (outward) | ✓ | ✓ | ✓ |
| Text anchor | `Mid = midpoint(Pa, Pb)`; placement/orientation per standard (§5.2) | ✓ | ✓ | ✓ |

**The invariant this buys, and it is the acceptance criterion:**

```
|Pb − Pa| == kind.measured_points()      for every (a, b, constraint, offset)
```

Proof: `Pb − Pa = t·u`, so `|Pb − Pa| = |t| = |w·u|`, which is `|Δx|` for `Horizontal`,
`|Δy|` for `Vertical`, and `|w|` for `Aligned` — precisely `measured_length`'s three cases.
**The drawn dimension line is exactly as long as the number printed on it.** Today it is
not: for a Horizontal pick with `Δy ≠ 0`, the drawn line is `√(Δx² + Δy²)` long while the
label says `Δx`. That is the operator's complaint, stated as a testable property, and it
should ship as a `proptest`-style property test over random point pairs and constraints —
not three hand-picked examples.

### 2.4 Extension lines, drawn

For a measured point `m` with signed span `s` along `n` (so `s = sa = offset` for `a`, and
`s = sb = offset − h` for `b`):

```
if !s.is_finite() || |s| <= EXT_GAP  -> draw NOTHING for this extension line
else:
    dir   = sign(s) · n
    start = m + EXT_GAP           · dir       // stand off from the object
    end   = m + (|s| + EXT_OVER)  · dir       // overshoot past the dimension line
    stroke start -> end
```

Three things this handles that are easy to get wrong:

- **`offset = 0`** ⇒ `sa = 0` ⇒ `a`'s extension line is omitted entirely, which is correct:
  the dimension line already passes through `a`, and drawing a gap-plus-overshoot stub there
  would put a mark on the wrong side of the line.
- **`|s| <= EXT_GAP`** ⇒ omitted, not clamped. Clamping produces a segment that starts past
  the dimension line and runs backwards.
- **The `a` and `b` extension lines can point in opposite directions** — e.g. a Horizontal
  pick with `b` above `a` and the dimension line placed between them. `sign(s)` is computed
  per point, so this falls out; it must have a test, because an implementation that computes
  one `dir` from `offset` alone gets it wrong.

### 2.5 Degenerate cases (`ARCHITECTURE.md` §10 — total, never a panic)

| Case | Behaviour |
|---|---|
| `t == 0` (zero measured length) | `Pa == Pb`. Draw both extension lines and the text; **no terminators** (there is no direction to point them). `/Rect` still guaranteed positive-area by `BoundsAcc::into_rect`'s existing 1-pt floor (`author.rs:406-425`). |
| `a == b` exactly | as above, plus both extension spans equal `offset`. |
| Non-finite `a`, `b`, or `offset` | `BoundsAcc::add` already skips non-finite points (`author.rs:397-404`); the drawing calls must additionally guard so no `NaN` reaches the content stream. A `NaN` in an `/AP` stream is a malformed stream, which the existing `appearance_content_reparses_as_a_content_stream` test would catch — extend it to the degenerate inputs. |
| `\|w\| ≈ 0` under `Aligned` | `u = (1, 0)` per `unit_vector`'s existing rule; then `t = 0` and the row above applies. |

### 2.6 Small-space handling (both standards)

When the dimension line is too short to contain the terminators and the text, both
standards move things outside. The rule pdfce adopts:

```
extent_u = |text_w · u.x| + |text_h · u.y|      // text bbox projected onto the axis
needed   = extent_u + 2·TEXT_PAD + 2·ARROW_LEN
if |t| < needed:
    - terminators flip to point INWARD, placed OUTSIDE Pa and Pb
    - text is placed outside the extension lines, past the `+su` end,
      offset by TEXT_PAD + ARROW_LEN from Pb
    - ANSI: the dimension line is NOT broken in this case (there is nothing to break)
    - the dimension line is extended by ARROW_LEN past each end so the
      outside-pointing terminators have a line to sit on
```

This is real drafting behaviour under both standards and it is not optional polish: on a
CAD drawing, short dimensions are the common case, and a label overhanging its own
arrowheads is the most obviously-wrong thing a dimensioning tool can draw. Scoped into
Pass 27.0 rather than deferred, because the geometry rework is already touching every line
of `draw_linear`.

### 2.7 What this does NOT change

- `DimensionKind::Circular` keeps its centre→rim leader unchanged. The standards also
  govern radial/diameter leaders (jogged leaders, centre marks, the `⌀` prefix) — that is a
  separate, larger slice and is named as out of scope in §9.
- `measured_points()`, `format_measurement`, the `/Measure` mirror's *scale* content, the
  OCG layer model, and the group model are all untouched by §2.
- `AUTHORED_ANNOT_KEYS` is unchanged: `/Rect`, `/L` and `/Contents` are already owned by
  authoring, and `/L` simply gets the corrected `(Pa, Pb)` instead of `(a, b)`.

> **`/L` semantics, checked against the spec rather than assumed.** ISO 32000-1 §12.5.6.7
> defines `/L` as the line annotation's two endpoints in default user space, and for
> `/IT /LineDimension` those endpoints are the *dimension line*, not the measured feature.
> Writing `(Pa, Pb)` is therefore both the fix and the spec-correct reading. The measured
> points `a`/`b` have no native home in the `/Line` dictionary and live only in the
> `/PieceInfo` sidecar — which is one more concrete instance of why the sidecar is
> authoritative (decision 011 §2.4) and why §3.6's cliff matters.

---

## 3. Q2 — The offset, and the model change it forces

### 3.1 Decision

**Add one field: `offset: f64` to `DimensionKind::Linear`.** Signed, page-space points,
measured along the canonical normal `n` (§2.2), **based at the measured point `a`**.
Default for a newly authored ce dimension: **exactly `0.0`**.

```rust
DimensionKind::Linear {
    a: Point,
    b: Point,
    constraint: AxisConstraint,
    /// Signed perpendicular displacement of the DIMENSION LINE from the measured
    /// point `a`, in page-space points, along the canonical normal (§2.2).
    /// `0.0` puts the dimension line through `a` — the geometry the two-click
    /// preview showed at commit time.
    offset: f64,
}
```

### 3.2 Why a scalar and not a third point

| Option | Verdict |
|---|---|
| **`offset: f64`** (chosen) | Exactly one degree of freedom, which is exactly how many the placement has (the component along `u` is meaningless — the dimension line's extent is pinned by the extension lines). Translation-invariant: `translated()` leaves it alone, no code. Zero default = today's geometry. |
| `line_point: Point` (AutoCAD's "dimension line location" pick) | Two stored numbers for one degree of freedom. The component along `u` is dead state that a drag would write and nothing would read — two representations of one truth, which drift (R92). Would also need translating in `translated()`, adding a way to get it wrong. |
| `offset` on `DimensionRecord` instead of on the kind | Breaks the contract that `author_dimension` is *"a pure function of `(kind, scale, format)`"* (`author.rs:28-33`) — the property `move_dimension` and `set_group_scale` both rely on. Would force a wider signature change on `author_dimension` for no gain. |
| Derive it from `/Rect` on load | The appearance is generated *from* the model, never parsed back into it. Inverting that is a new, fragile direction of data flow. |

The one thing `line_point` would buy is a future **oblique/rotated** dimension kind (a
dimension line at an arbitrary angle to the measured pair). That is a *new variant*, not a
change to this one, and it can carry its own representation. Recorded so the choice is not
re-litigated: `offset: f64` is right for `Linear`, and does not foreclose an oblique kind.

**`Circular` gets no offset.** Its leader runs centre→rim by construction; there is nothing
to stand off from. Adding a field to only one variant of an enum is normal and is what
enums are for.

### 3.3 Why the base is `a`, and why the default is exactly 0.0

Base candidates were `a`, `midpoint(a, b)`, and "the point farther along `+n`."

- **`midpoint`** makes `offset = 0` place the line *between* the two measured points when
  they differ in height — a position that matches neither the preview nor anything a
  drafter would draw, and one that changes if `b` moves.
- **"farther point"** makes the base *switch identity* as the offset is dragged across
  zero, so the stored value jumps discontinuously at the crossing. Unacceptable for a
  drag.
- **`a`** is stable, is the anchor of the first extension line, and — decisively — makes
  `offset = 0.0` reproduce `preview_segment`'s output exactly.

That last point is the argument that settles the default. `preview_segment` draws
`(a, constrained_second_point(a, raw, constraint))`, which for `Horizontal` is
`(a, (raw.x, a.y))` — i.e. **a horizontal segment at `y = a.y` spanning to the pointer's
x**. Under §2.3 with `offset = 0.0` and `b = raw`, `Pa = (a.x, a.y)` and
`Pb = (b.x, a.y)`. **Identical.** So:

> **The default offset of 0.0 makes the baked appearance byte-for-byte the same geometry
> as the preview line the operator was looking at when they clicked.** Finding 2's
> divergence does not get narrowed; it closes.

A nonzero default (a "standoff" of ~10× text height, which is what a CAD package would do)
is therefore **rejected as a default** and offered instead as a preference, deferred:
choosing a nonzero default means choosing a *sign*, and pdfce has no information about
which side of the feature is clear space. Guessing would be wrong roughly half the time and
would break preview/commit convergence to do it. Open question **(ar)**, §10.

### 3.4 Blast radius, measured

`DimensionKind::Linear` is constructed at **23 sites**:

| File | Sites |
|---|---|
| `crates/pdfce-gui/src/measure_tool.rs` | 6 |
| `crates/pdfce-core/src/dimension/author.rs` | 4 (all tests) |
| `crates/pdfce-core/src/dimension/group.rs` | 3 (all tests) |
| `crates/pdfce-core/src/dimension/sidecar.rs` | 3 |
| `crates/pdfce-cli/src/main.rs` | 2 |
| `crates/pdfce-core/tests/dimension_roundtrip.rs` | 2 |
| `crates/pdfce-gui/src/main.rs` | 1 |
| `crates/pdfce-render/tests/{edited_view_is_what_renders,preview_equals_saved}.rs` | 2 |

Adding a field to a struct-like enum variant is a **compile error at every construction
site** — the change cannot be half-done and cannot be silently missed. That is the safe
kind of breaking change and it is worth saying out loud: the alternative designs in §3.2
that avoid the 23 edits do so by making the state *invisible to the compiler*, which trades
23 mechanical fixes for an unbounded number of runtime ones.

`DimensionKind::translated` (`group.rs:132-147`) needs **one line**: carry `offset` through
unchanged. Its existing doc comment — *"A translation is a rigid motion: every distance it
preserves"* — remains true and now covers the offset too, since `offset` is a distance
along a direction that translation does not rotate.

### 3.5 Sidecar serialisation

`serialize_dimension`'s `Linear` arm gains one key; `deserialize_dimension`'s gains one
tolerant read:

```rust
// serialize_dimension, Linear arm (sidecar.rs:188-196) — ADD:
d.insert(Name::from(b"Offset"), Object::Real(offset));

// deserialize_dimension, "linear" arm (sidecar.rs:219-223) — ADD:
offset: d.get(b"Offset").and_then(Object::as_number).unwrap_or(0.0),
```

`unwrap_or(0.0)`, **not** `?`. A sidecar written before this Pass has no `/Offset`; it must
deserialise to `0.0`, which §3.3 established is exactly its shipped geometry. The
migration is therefore not merely lossless — **it is a visual no-op for every existing ce
dimension whose pick was already axis-aligned, and for a non-axis-aligned pick it changes
the drawing to what the operator was shown at commit time.** No file needs a conversion
step; no operator sees anything move that they did not already expect to be where it now is.

`SIDECAR_VERSION` **stays at 1**. This is a purely additive optional key, which is exactly
the case the module header already documents:

> `sidecar.rs:26-28` — "Deserialisation is **total and lenient** […] and unknown keys are
> ignored (forward-compat, §14.5's `PictureEdit`/`PictureEditExtended` pattern)."

A version-1 pdfce reading a version-1-with-`/Offset` sidecar ignores the key and renders
offset 0 — degraded, not broken, and it round-trips the rest of the model intact. That is
the correct behaviour for an optional placement hint and it is only available because the
version was *not* bumped.

### 3.6 ★ The latent data-loss cliff in `sidecar.rs`, and why this Pass must widen the gate

```rust
sidecar.rs:67   if d.get(b"Version").and_then(Object::as_int) != Some(SIDECAR_VERSION) {
sidecar.rs:68       return None;
sidecar.rs:69   }
```

Exact equality, in both directions. Consider what `None` costs. `deserialize_model`'s
caller starts a fresh `DimensionModel::new()` — one default group, no dimensions. But the
`/Line` annotations are still in the file and still render, because their `/AP` is baked.
So the operator opens a dimensioned drawing and sees:

- every ce dimension still drawn, still with the right numbers;
- the group panel showing one empty "Default" group;
- **every calibrated scale gone** — the `real_length ÷ drawn_length` work from decision 011
  §2.4's headline workflow, silently discarded;
- every group name, membership and layer visibility gone;
- and on the next save, `catalog_dimension_write` writes the *fresh* model over the old
  sidecar, making the loss permanent.

Nothing is disclosed. This directly violates decision 011 §2.4's own binding instruction
for the analogous case — *"On load, if native and sidecar disagree […] **disclose** and
prefer the sidecar"* — because a version mismatch is not even reached by that rule.

The trigger today is narrow (only a pdfce build that bumped the constant), which is exactly
why it is worth fixing *now*, while the trigger is still hypothetical: **the first person to
bump `SIDECAR_VERSION` for a genuinely breaking change destroys every dimensioned file
produced before it**, and will not find out from a test, because every test in
`sidecar.rs` constructs and parses at the same version.

**Decision: widen the gate to a range in the same Pass, as a separate commit with its own
tests.**

```rust
/// The oldest sidecar layout this build can still read. Bumped ONLY when a
/// change genuinely cannot be expressed additively — and then the deserializer
/// keeps a per-version path, it does not drop the old one.
pub const MIN_READABLE_SIDECAR_VERSION: i64 = 1;

let version = d.get(b"Version").and_then(Object::as_int)?;
if !(MIN_READABLE_SIDECAR_VERSION..=SIDECAR_VERSION).contains(&version) {
    return None;      // genuinely unreadable: newer major, or pre-history
}
```

and, for the newer-than-us case, the disclosure rather than the silent discard: a sidecar
whose `/Version` **exceeds** `SIDECAR_VERSION` means a newer pdfce wrote this file. Silently
starting fresh and then overwriting it is the worst available option. The correct behaviour
is to refuse to *write* the sidecar and tell the operator, which is a new refusal:
**`SidecarWrittenByNewerBuild`** (proposed R138, §8). This is not scope creep — it is the
same class of guard as decision 025 §5.5's `DeleteWouldMoveNextSubpath`: a byte-minimal
operation that quietly destroys work the operator cannot get back, catchable only by a
named refusal.

### 3.7 The migration criterion, stated so it cannot be skipped

Pass 27.0 does not ship without this test, by name:

> **`a_pre_offset_sidecar_deserialises_with_offset_zero_and_loses_nothing`** — construct
> the version-1 sidecar `Object` **without** any `/Offset` key (built by hand, not by
> calling `serialize_model`, so the test cannot rot into vacuity when the serializer
> changes), deserialise it, and assert: every group survives with its exact id, name,
> scale, format, visibility and `/Ocg`; every dimension survives with its exact id, group,
> points, constraint, `/Annot` and `/Ap`; and every `Linear` has `offset == 0.0`.

Plus its partner:

> **`a_sidecar_with_an_unknown_future_key_still_round_trips`** — inject a nonsense key into
> a serialised dimension dict and assert deserialisation is unaffected. The module header
> *claims* this property; nothing tests it.

---

## 4. Q3 — Drag semantics: two readings, a recommendation, and the question for the operator

### 4.1 The sentence

> "when I mve a ce dimension it should extend the lines to the connection points and stay
> algined with them - **ie if it is a horizontal dimension I should only be able drag it
> horizontally so it stays in line with what it is actually measuring**"

The clause after "ie" is offered as a *restatement* of the clause before it. Under the
model the first clause describes, it is not one. That is the whole problem.

### 4.2 What is shipped today (Pass 25.5), for contrast

`run_dimension_drag` (`main.rs:12250-12403`) does a **free two-dimensional translate**:
`move_dimension(id, dx, dy)` → `DimensionKind::translated(dx, dy)` → both measured points
move by the same delta. The value is preserved (a rigid motion preserves distances) and
`group.rs:119-130` argues that correctly. But **the ce dimension detaches from the feature
it measures**: `a` and `b` are no longer on the drawing. There are no extension lines to
stretch, because there is nothing left behind to stretch back to. The operator's first
clause is a direct report of this: *there should be lines going back to the connection
points, and there aren't.*

### 4.3 Reading A — the literal one: drag constrained to the dimension's own axis

"Horizontal dimension ⇒ drag vector constrained to horizontal."

**Under the corrected offset model this is a geometric no-op.** The dimension line's
position along `u` is not a free parameter: `Pa` and `Pb` are pinned to the measured points'
projections (§2.3). Sliding "along the axis" changes nothing that is drawn. The gesture
would consume a drag and produce no visible result, which is worse than refusing it.

**Under the shipped whole-translate model it is actively harmful.** Dragging a Horizontal
ce dimension horizontally moves `a` and `b` in x — off the very features whose x-separation
is being reported — while the label keeps reading the old, now-unanchored value. This is
the *opposite* of "stays in line with what it is actually measuring," and it is close to
the sneakiest thing a measurement tool can do: a number that still looks authoritative
while pointing at nothing.

So Reading A is either inert or wrong, depending on which model it is applied to. It cannot
be what was wanted.

### 4.4 Reading B — the CAD one: drag constrained perpendicular, changing the offset

"Horizontal dimension ⇒ drag vector constrained to *vertical*; the measured points stay
pinned; the offset changes; the extension lines grow to span the new gap."

This is the AutoCAD/SolidWorks/Inkscape-measure convention and it satisfies every clause of
the first half of the sentence:

- *"extend the lines to the connection points"* — the extension lines are precisely the
  lines that get longer as the offset grows. This is the only reading under which that
  phrase describes anything at all.
- *"stay aligned with them"* — `Pa` and `Pb` remain the perpendicular projections of `a`
  and `b`, so the dimension line stays in registration with the feature for the whole drag.
- *"stays in line with what it is actually measuring"* — the drawn line stays exactly
  `measured_points()` long and stays parallel to the measurement axis. §2.3's invariant.

Mechanically: `offset_new = offset_old + ((cur − down) · n)`, where `cur` and `down` are
the pointer's page-space positions. The along-`u` component of the drag is discarded. The
live preview during the drag shows the ce dimension redrawn at the trial offset (not just a
bbox outline, which is what `main.rs:12362-12367` draws today — a bbox is inadequate
feedback for an offset drag because the *shape* changes, not just the position).

### 4.5 Why Reading B is almost certainly what was meant

1. **"Extend the lines to the connection points" is unimplementable under Reading A.** The
   first clause is unambiguous and describes a mechanism; the "ie" clause is an
   *explanation* of it. When an explanation and the thing it explains conflict, the thing
   being explained is the more reliable signal.
2. **Reading A is a no-op under the very model the first clause requests.** The operator is
   describing a behaviour he wants, not a null gesture.
3. **The two readings produce nearly the same picture from different sides.** "Whole-move,
   but constrained to the perpendicular, plus lines back to where the points were" *is* the
   offset drag. It is entirely plausible to describe the offset drag as constrained
   movement and then name the wrong axis while thinking about which axis the dimension
   *is*, rather than which axis the *drag* runs along. "It is a horizontal dimension"
   → "so drag it horizontally" is a natural slip; "it is a horizontal dimension" → "so drag
   it vertically" requires holding two axes in mind at once.
4. **The domain convention is unanimous.** Every CAD package the operator uses places the
   dimension line by dragging perpendicular to the measurement axis. This is a CAD-drawing
   product for a CAD user; the convention is evidence.

### 4.6 The synthesis — which honours both halves without guessing

There *is* one legitimate along-axis drag in drafting, and it is not the dimension line: it
is the **text position**. Both standards permit sliding the value text along the dimension
line, and pulling it outside the extension lines when it will not fit (§2.6 automates the
"will not fit" case; a manual override is the drafter's escape hatch).

So both halves of the sentence can be true of different grabs:

| Grab | Drag direction | Effect |
|---|---|---|
| The **dimension line** or an extension line | perpendicular (`n` only) | changes `offset` — Reading B |
| The **value text** | along the axis (`u` only) | slides the text along the line — Reading A's direction, on the thing it makes sense for |

This is offered as the *destination*, not as Pass 27.1's scope. Text position needs a
second stored field (`text_along: f64`) and is deferred (§9). It is recorded here because
it is the reading under which the operator's sentence is entirely correct with no slip at
all — and if that is what he meant, the answer to the question below is "both, on different
handles."

### 4.7 ★ The exact question for the operator

> **Your sentence has two halves that point opposite ways, and I do not want to guess.**
>
> You wrote: *"it should extend the lines to the connection points and stay aligned with
> them — ie if it is a horizontal dimension I should only be able drag it horizontally so
> it stays in line with what it is actually measuring."*
>
> For a **horizontal ce dimension**, dragging **horizontally** slides it left/right *along*
> its own length — which moves it out of registration with the two points it measures.
> Dragging **vertically** moves the dimension line up/down away from the drawing while
> staying in registration, and the extension lines grow to reach back to the measured
> points. That second one is the AutoCAD/SolidWorks convention and it is the only one under
> which "extend the lines to the connection points" describes anything.
>
> **My reading is that you meant the vertical (perpendicular) drag — i.e. a horizontal ce
> dimension can only be pushed up or down, a vertical one only left or right — and that
> "horizontally" was about which kind of dimension it is, not which way the drag goes.**
> Three questions:
>
> **(1)** Is that right? Perpendicular-only drag, measured points pinned, extension lines
> stretch?
>
> **(2)** Or did you mean something I have not thought of — for instance, sliding the
> **number** along the dimension line (that one *is* an along-the-axis drag, and both
> drafting standards allow it)? If you want both, they can live on different grabs: drag
> the *line* to move it off the drawing, drag the *text* to slide the number along.
>
> **(3)** Today's drag (Pass 25.5) moves the **whole** ce dimension — both measured points
> travel with it, so it comes off the feature entirely. Under (1) that gesture goes away as
> the default. Do you want to keep it anywhere — say on Ctrl+drag, or on grips at the
> measured points — for the case where you dimensioned the wrong feature and want to shift
> the whole thing? Or is "pick it again" the right answer and the whole-move should just go?

### 4.8 Why this is not decided solo

Decision 022 §4.2 is binding here and it draws the line in the right place:

> "**Moving a dimension's endpoint is a re-measure, not a translate** […] in a *measurement*
> tool, a drag that silently changes a reported measurement is the single sneakiest thing
> the application could do (rule 4)."

022 split dimension gestures into two classes — translate (safe, value-preserving) and
re-measure (dangerous, deferred to the Measure tool with a live pre-commit preview). **The
offset drag is a third class 022 did not contemplate, and it is safer than either**: it
changes neither `a`, nor `b`, nor `constraint`, nor `measured_points()`. It is provably
value-preserving *by construction* rather than by arithmetic coincidence, because it
touches a field the value function does not read.

That makes the offset drag the safest possible default drag — which is an argument for
Reading B, not an argument for deciding Reading B without asking. Rule 4's "reviewable hint
the operator accepts or overrides" applies to *pdfce's* inferences about operator intent
just as much as to OCR output. The recommendation is made loudly; the choice is his.

### 4.9 What Pass 27.1 does under each answer

| Operator's answer | Pass 27.1 scope |
|---|---|
| (1) yes — perpendicular | Constrain the drag to `n`; live-redraw the ce dimension at the trial offset; commit `set_dimension_offset`. Whole-move demoted per (3). |
| (2) both, on different grabs | 27.1 ships the perpendicular line-drag; text-along-axis becomes Pass 27.3 with a `text_along: f64` field, same additive sidecar pattern. |
| (3) keep whole-move | Retain `move_dimension` on a modifier; the CLI keeps whatever it has, since a CLI has no gesture ambiguity. |
| Something else | 27.1 does not start. The geometry Pass (27.0) is independent of the answer and ships regardless. |

**Pass 27.0 is deliberately not gated on this question.** The corrected geometry, the
offset field, and the sidecar migration are correct under every reading — including under
Reading A, and including under "no drag at all, set the offset in a properties field."

---

## 5. Q4 — ANSI vs ISO

### 5.0 Sourcing discipline, stated before any fact

The drafting standards are **not** PDF specifications and are **not** in
`D:\Dev\Rag-Specialized\PDF_Spec\`. Rule 1's "never from training-data memory" applies with
no RAG to fall back on, so this section was researched from live sources and every claim
below carries an explicit confidence marker. **No clause number appears here unless it was
read from a document.** Where a number could not be verified, the substance is given and
the gap is named — per the global claim-bearing-copy rule, a plausible-looking invented
citation in an engineering record is worse than an honest "unverified."

| Marker | Meaning |
|---|---|
| **VERIFIED** | Read verbatim from the standard's own text or ASME's own published front matter. |
| **HIGH** | Multiple independent reliable secondaries agree (university engineering course material, CAD-vendor documentation naming the standard). |
| **MEDIUM** | One secondary source. |
| **UNVERIFIED** | Could not source. Named as such; not implemented as if settled. |

Two access facts shape what follows. **ISO 129-1:2018's official free preview covers
clauses 1 through 5.3 verbatim** — so ISO's text-placement, orientation, decimal-marker and
unit rules below are primary-sourced. **Clauses 5.4 (Terminators) and 5.5 (Extension line)
are past the preview cut**: their numbers and titles are verified from the table of
contents, their *contents* are secondary-sourced only. **ASME Y14.5 and Y14.2 are entirely
paywalled**; full-text copies were found in search results and **refused** — one z-lib
sourced, the rest unauthorized re-hosts (`LEGAL.md` §2's spirit: pdfce does not build on
material it has no right to). **No ASME clause number is cited anywhere in this record.**

### 5.1 ★ The first correction: ASME Y14.5 is the wrong standard for most of this

The request said "ANSI standards," and the natural assumption is ASME Y14.5. **That is
wrong for everything pdfce draws.** Y14.5 is the **dimensioning-and-tolerancing/GD&T**
standard — feature control frames, datums, tolerance zones. The geometry of the *marks* —
arrowheads, line conventions, lettering — lives in a different document:

> **ASME Y14.2-2014, "Line Conventions and Lettering."** Structure confirmed from ASME's
> own published front matter: **§4 Line Conventions, §5 Arrowheads, §6 Lettering**, with
> **Fig. 4-7 "Arrowhead Placement on Dimension Lines," Fig. 4-8 "Special Applications of
> Extension Lines," Fig. 5-1 "Arrowhead Styles," Table 6-1 "Minimum Letter Height
> Proportions."** Issued 30 January 2015. **VERIFIED** (asme.org front matter PDF).

ISO consolidates: **ISO 129-1** carries terminators, extension lines, placement *and*
units in one document.

**Consequence for pdfce.** The operator-facing label must not say "ASME Y14.5" — the
`DimStandard::Ansi` variant implements **ASME Y14.2 line/arrowhead practice plus Y14.5-era
dimensioning convention**, and the operator-facing string should read plainly ("ANSI /
ASME (US)" vs "ISO (international)") rather than cite a standard number pdfce cannot fully
claim conformance to. Naming a specific standard in the UI is a claim under the global
claim-bearing-copy rule; pdfce has not read Y14.2's normative text and must not imply it
has. **Proposed criterion E9, §7.3.**

### 5.2 The difference table — what actually changes on the page

| # | Aspect | **ANSI / ASME** | **ISO** | Confidence |
|---|---|---|---|---|
| 1 | **Text vs dimension line** | Line **breaks**; value centred in the gap. The governing convention is stated as *"a dimension line is never broken except for insertion of the dimension"* — i.e. the break exists **only** to hold the value. | Value sits **above an unbroken line**. **Mandated.** ISO 129-1:2018 **cl. 4.1.1**: *"The text of all dimensions, graphical symbols and annotations **shall be indicated above the dimension line** and read from the bottom."* Reinforced **cl. 5.3**: *"Where the feature is shown broken, the corresponding dimension line shall be shown unbroken."* | ANSI **HIGH** (clause unverified) · ISO **VERIFIED** |
| 2 | **Text orientation** | **Unidirectional** — all text horizontal, read from the bottom of the sheet. | **Aligned** with the dimension line. ISO 129-1:2018 **cl. 4.1.1**: *"…read from the bottom. When the text of a dimension, symbol or annotation is presented vertically, **it shall read from the right**. **The determination of orientation is based on the centre of the dimension**, symbol or annotation."* | ANSI **HIGH** · ISO **VERIFIED** |
| 3 | **The "ambiguous zone"** | n/a (text is always horizontal). | **★ The 30°-either-side-of-vertical rule is NOT in ISO 129-1:2018.** It appears only in teaching material and is classically an *angular*-dimension convention from the superseded ISO 129:1985. ISO's actual tie-break is the sentence in row 2: **orientation is resolved at the centre of the dimension** — a deterministic rule that is directly implementable. | folklore **MEDIUM**; ISO's real rule **VERIFIED** |
| 4 | **Terminator forms** | Filled arrowhead, **3:1 length-to-width**, one style throughout the drawing. Governing home is **ASME Y14.2 §5 "Arrowheads"** (clause number VERIFIED, text paywalled) — whether Y14.2 words 3:1 as *shall* or *should* **could not be verified**. Fig. 5-1 is titled "Arrowhead **Styles**" (plural), so more than one is sanctioned. | Closed-filled 30°, closed-blank 30° (engineering default); open 30°/90°; **oblique stroke at 45°**; **dot**; **origin symbol** (circle) — the last VERIFIED as **term 3.1.4**, *"circle indicating the start of running dimensioning or coordinate dimensioning"*, with **cl. 5.4.2 "Origin presentation."** Governing clause **5.4.1 "Terminators."** | ANSI 3:1 **HIGH**, mandate status **UNVERIFIED** · ISO list **HIGH**, clause numbers **VERIFIED**, clause text paywalled |
| 5 | **Is the oblique stroke "architectural"?** | — | **Yes, by discipline, not by permission.** CAD-vendor documentation names closed-filled and closed-blank as *"typically used in engineering"* and oblique as *"favored by architects."* But ISO 129-1:2018 **Scope §1** says it *"applies to 2D technical drawings in **all disciplines and trades**"* — so oblique is fully legal ISO everywhere. | discipline split **HIGH** · scope **VERIFIED** |
| 6 | **Extension-line gap from the feature** | A gap is **required**, value **not specified**: *"There should be a visible gap between an extension line and the feature to which it refers."* Conventional magnitude **1.5 mm**; architectural practice **1/16″–1/8″ (1.6–3 mm)**. **Absolute lengths.** | **Permissive, not required**, and field-dependent: *"It is permissible to have a gap (approximately **8 × the line width**) between the feature and the beginning of the extension line **in certain technical fields**."* **A multiple of line width.** Governing cl. **5.5**. | gap-exists **HIGH** · magnitudes **MEDIUM**, convention not mandate |
| 7 | **Extension-line overshoot past the dimension line** | *"about **1 mm (1/32 inch)** beyond the last dimension line"* (mechanical); architectural sources give **~3 mm (1/8″)**. The two US traditions genuinely differ. | *"approximately **8 × the line width** beyond the respective dimension line."* | **MEDIUM** both |
| 8 | **★ The structural difference in 6–7** | **Absolute lengths.** | **Multiples of line width** — self-scaling with line weight. For a 0.25 mm narrow line, ISO's 8× ≈ 2 mm, which sits inside the ANSI 1–3 mm range, so the *rendered* results are similar. **Implementing ANSI as absolute and ISO as line-width-relative reproduces both traditions from one model** — this is the single cleanest structural difference in the whole section and pdfce should adopt it verbatim. | **HIGH** (analysis) |
| 9 | **Extension-line crossings** | Same rule both: extension lines are **not** broken where they cross object lines or other extension lines, **but are** broken at or adjacent to arrowheads. | same | **HIGH** |
| 10 | **First dimension line offset from the outline** | **10 mm (3/8″) minimum.** One outlier source gives 1/2″ / 15 mm; architectural gives 3/8″. | **10 mm** (ISO/BIS teaching sources). | **MEDIUM**; the *existence* of a minimum is **HIGH** |
| 11 | **Spacing between successive parallel dimension lines** | **6 mm (1/4″) minimum.** | **6 mm.** ISO 129-1:2018 **cl. 8** covers arrangement (**8.2 Chain, 8.3 Parallel, 8.4 Running, 8.5 Coordinate, 8.6 Combined**) — clause numbers VERIFIED, spacing values not in the preview. | **MEDIUM** |
| 12 | **★ Decimal marker** | **Point.** | **Comma. Mandated.** ISO 129-1:2018 **cl. 4.1.1**: *"Dimensional values indicated in decimal notation, **shall use a comma as the decimal marker**."* Worth flagging: a genuine "shall" that is **widely violated in practice** — much ISO-region CAD output uses a point. | ANSI **HIGH** · ISO **VERIFIED** |
| 13 | **Leading zero** | **Inch: SUPPRESSED** (`.500`, not `0.500`). **Metric: PRESENT** (*"a zero precedes the decimal point where the dimension is less than one millimetre"*). | **Present** (`0,5`) — **inferred from ISO 80000-1 numeric convention; NOT found in ISO 129-1:2018's readable text.** | ASME **HIGH** · ISO **MEDIUM (inferred)** |
| 14 | **★ Real-world conformance to 13** | **Not uniform.** University of Florida's own teaching rules state *"Decimal dimensions less than 1.0 should be preceded with a leading zero (i.e. 0.375)"* — **directly contradicting the ASME inch rule.** Inbound drawings cannot be assumed conformant. | — | **HIGH** |
| 15 | **Trailing zeros** | **Inch: REQUIRED**, to make the dimension's and the tolerance's decimal-place counts match. **Metric: NOT added**, except to pad a non-symmetric tolerance's lower limit to match the upper. | **Optional.** ISO 129-1:2018 **cl. 4.1.7**: *"**Trailing zeros may or may not be presented**."* ISO instead mandates **alignment**: *"the decimal marker of the upper and lower shall be aligned. When a tolerance limit is not shown with a decimal marker, the remaining digits shall be aligned as if the decimal marker had been displayed."* | ASME **HIGH** · ISO **VERIFIED** |
| 16 | **Unit symbol on each dimension** | **Omitted** — stated once for the drawing. *"In machine drawing, all unit marks should be omitted, except when necessary for clarity."* | **Omitted for linear, always shown for angular.** ISO 129-1:2018 **cl. 4.3**: *"For linear units, the predominant unit… may be specified on the drawing or in an associated document and **the unit omitted from the individual dimensions**… Any dimensions expressed in a different unit of measure shall indicate that unit of measure."* and *"For angular dimensions, the units **shall always** be specified with the individual dimensions."* | ANSI **HIGH** · ISO **VERIFIED** |
| 17 | **Reversed terminators when cramped** | Same both. | ISO 129-1:2018 **cl. 5.3**: *"Where space is limited, dimension lines may be extended past the extension lines and the arrowheads placed outside of the extension lines and reversed."* | ISO **VERIFIED** · ANSI **HIGH** |
| 18 | **Single text height per drawing** | Convention. | **Mandated.** ISO 129-1:2018 **cl. 4.1.7**: *"There shall be only one character height for dimension and tolerance presentation for a specific drawing."* | ISO **VERIFIED** |
| 19 | **Out-of-scale values** | Underlined (practice). | **Underlined** — cl. 4.1.3. | ISO **VERIFIED** · ANSI **MEDIUM** |
| 20 | **Auxiliary / reference values** | "REF" or parentheses. | **Parentheses `( )`** — cl. 4.1.4. Theoretically-exact values get a rectangular frame per ISO 1101 — cl. 4.1.5. | ISO **VERIFIED** · ANSI **MEDIUM** |
| 21 | **Property indicators (`⌀`, `R`, `□`, `S⌀`, `SR`)** | — | ISO 129-1:2018 **cl. 5.2**: indicators **precede the value with no space**. A plain linear dimension between two parallel planes or lines takes **no indicator** — it is bare. | ISO **VERIFIED** |
| 22 | **Line class of the dimension line** | Y14.2 §4 "Line Conventions." | **Continuous NARROW line** per ISO 128-20, cl. 5.3. Line *weight class* matters, not just colour. | ISO **VERIFIED** |

### 5.3 What the standard governs in pdfce, and what it does not

**R139's boundary, stated precisely, and it is not quite where §6.4's first draft put it.**
The research forced one revision: **the decimal marker is mandated by ISO** (row 12,
VERIFIED "shall"), so it cannot simply be waved off as locale. But it *is* expressible in
the portable projection (§5.4), which means it can be governed by the standard **without**
breaking the one contract that would otherwise forbid it.

| Aspect | Governed by | Ships in |
|---|---|---|
| Dimension line broken vs unbroken (row 1) | **`DimStandard`** | 27.2 |
| Text orientation and side (rows 2–3) | **`DimStandard`** | 27.2 |
| Terminator form (row 4) | **`DimStandard`** | 27.2 |
| Extension gap + overshoot, and whether they are absolute or line-width-relative (rows 6–8) | **`DimStandard`** | 27.2 |
| Cramped-space reversal (row 17) | neither — **always on**, both standards agree (§2.6) | 27.0 |
| **Decimal marker (row 12)** | **`NumberFormat::decimal_marker`, whose default is SET from `DimStandard`** and which the operator may then override per group | 27.2 |
| Unit, precision, fraction denominator | **`NumberFormat`** — unchanged, as today | — |
| The measured value itself | **nothing** — `measured_points()` is geometry | — |

**Three things are deliberately NOT implemented, and each is a named honest limit rather
than an oversight:**

1. **Leading-zero suppression for ANSI inch (row 13).** ASME's inch rule is well-attested
   (HIGH), and pdfce will not implement it. **Reason: it is not expressible in ISO 32000-1
   §12.9 Table 263.** There is no key for "suppress the leading zero" — `/RD`, `/RT`, `/PS`,
   `/SS`, `/F`, `/D`, `/FD` are the whole vocabulary (§5.4). Implementing it would put
   pdfce's baked label and every conforming reader's computation of the *same file* into
   permanent, invisible disagreement — the exact defect §5.5 documents as already shipped
   once. Row 14 makes the case weaker still: real-world US practice is not uniform, and a
   respected engineering-school rule sheet teaches the opposite. If the operator wants it,
   it needs its own decision about which side of the divergence to accept. Question **(at)**.

2. **Trailing-zero rules (row 15).** pdfce shows fixed places (`3.10`), which matches ASME
   inch (required) and is permitted by ISO (optional). ASME *metric*'s "do not add trailing
   zeros" is not implemented, for the same expressibility reason plus §5.5's pre-existing
   `/FD` problem, which must be fixed first.

3. **Unit-symbol omission (row 16).** **Both** standards state the unit once for the
   drawing and omit it per dimension; pdfce prints it on every label. This is a real
   divergence from both, and it is kept deliberately: pdfce has no drawing-level title-block
   note to carry the statement, `/Measure`'s `/U` is a **required** key regardless, and a ce
   dimension that reads `3.10` with no unit in a document that also contains **pdf
   dimensions** in some other unit is an ambiguity a measurement tool should not create. A
   per-group "omit unit" toggle is a reasonable future option; it is not this Pass. Recorded
   as a named deviation so it is not later "discovered" as a bug.

**pdfce already satisfies row 18 trivially** (`LABEL_SIZE` is a single constant, 10.0 pt) —
recorded because it becomes a *constraint* the moment anyone adds a per-dimension text-size
control: ISO mandates one character height per drawing.

### 5.4 ★ The `/Measure` agreement contract, and why the comma survives it

`units.rs:225-227` states a contract that this section must not break:

> "This is the exact string pdfce's live readout and each baked `/AP` label show, and it is
> the value an ISO §12.9-honouring reader computes for the same file from the mirrored
> `/Measure` dict (**the two agree by construction** — same algorithm)."

The question is whether an ISO comma can be expressed portably. **It can.** From
`D:\Dev\Rag-Specialized\PDF_Spec\iso32000\iso32000__s__12.9.md` (ISO 32000-1 §12.9,
Table 263 — read, not recalled):

| Key | Type | Meaning | Default |
|---|---|---|---|
| `/RT` | text string | Text between orders of thousands (thousands separator). Empty = none. | **COMMA (`2Ch`)** |
| `/RD` | text string | **Text used as the decimal point.** Empty = default. | **PERIOD (`2Eh`)** |
| `/PS` | text string | Text concatenated to the **left** of the `/U` label. | single ASCII SPACE |
| `/SS` | text string | Text concatenated **after** the `/U` label. | single ASCII SPACE |
| `/O` | name | `S` = label is suffix, `P` = prefix. | `S` |

So the ISO presentation is fully portable: emit `/RD (,)` and — because ISO's default
thousands separator would otherwise *also* be a comma, producing `1,234,56` — emit
`/RT (\040)` (a space, the ISO 80000-1 group separator) or `/RT ()` (none) alongside it.
**That interaction is the kind of thing that gets missed**: switching only `/RD` to a comma
while leaving `/RT` at its comma default yields an unreadable number in any reader that
groups thousands. Proposed criterion **E10**.

This is why the decimal marker belongs on **`NumberFormat`**, not on `DimStandard`:

```rust
pub struct NumberFormat {
    pub unit: Unit,
    pub fraction: FractionMode,
    /// The decimal marker (§12.9 Table 263 `/RD`) and, paired with it, the
    /// thousands separator (`/RT`). ISO 129-1:2018 cl. 4.1.1 mandates a comma
    /// for ISO drawings; ASME practice uses a point.
    pub decimal_marker: DecimalMarker,   // Point | Comma
}
```

`NumberFormat` is precisely the structure `measure_dict` projects from, so putting the
marker there makes the "agree by construction" claim **structurally true instead of
aspirationally true**: the same field feeds `format()` and feeds `/RD`. Selecting ISO on a
group *sets* `format.decimal_marker = Comma` as a disclosed side effect, which the operator
may then override — so question **(at)** is answerable either way without a code change.
`format_measurement` and `author_dimension`'s **value** path do not need `DimStandard` at
all; only the **drawing** path does. R139's boundary holds, with the marker as its one
carefully-argued exception.

### 5.5 ★ Side-finding: pdfce's label and its own `/Measure` mirror can already disagree

Not caused by this record; found while establishing §5.4's contract; recorded because it
would otherwise stay invisible.

```
units.rs:283   fn trim_or_fixed(value: f64, places: u32) -> String {
units.rs:284       format!("{value:.*}", places as usize)      // 3.1 -> "3.10", always
```

against Table 263's `/D` and `/FD` rules, verbatim from the spec RAG:

> `/D` — "When `F=D`: precision of decimal display, **shall be a multiple of 10** (default
> 100 = two decimals; **low-order zeros truncated unless `FD` true**)."
> `/FD` — "If **true**, the `D`-formatted fractional value **may not** have its denominator
> reduced or **low-order zeros truncated**. **Default false.**"

and against what pdfce emits:

```
measure_dict.rs:115   (unit, FractionMode::Decimal { places }) => Object::Array(vec![nf_dict(
measure_dict.rs:118       Some(FracKeys { fraction: false, d: pow10(places), fd: false })])
                                                                              ^^^^^^^^^^
measure_dict.rs:171   if f.fd { d.insert(Name::from(b"FD"), Object::Boolean(true)); }
```

`fd: false` ⇒ `/FD` is **omitted** ⇒ default `false` ⇒ **a conforming reader is permitted
to truncate the low-order zero and display `3.1 m` where pdfce's baked label says
`3.10 m`.** The doc comment claims they agree by construction. They do not, and no test
catches it because nothing compares the two.

**The fix is one word** — `fd: true` for the decimal arm, since pdfce deliberately does not
trim (`units.rs:281-282`: *"no trimming — pdfce shows the requested precision verbatim so
`3.10` stays `3.10`"*). **It is not folded into Pass 27**: it is a separate defect with a
separate blast radius (it changes the `/Measure` bytes of every existing ce dimension the
next time one is regenerated), and bundling an unrelated byte-level change into a geometry
Pass is how a regression gets attributed to the wrong commit. Filed for the librarian as a
Backlog item, **and it must land before §5.3's item 2 (trailing-zero rules) is ever
attempted**, since that work depends on this contract being real.

### 5.6 The pdfce constants table — and which numbers are honest

Per row 8, ANSI values are absolute and ISO values are multiples of `LINE_WIDTH`
(`author.rs:54`, currently `0.75` pt). All lengths in PDF points (1/72″); 1 mm = 2.8346 pt.

| Constant | ANSI | ISO | Status |
|---|---|---|---|
| `EXT_GAP` (feature → extension-line start) | **4.25 pt** (1.5 mm) | **8 × `LINE_WIDTH`** = 6.0 pt at 0.75 | ANSI: **convention, not mandated** — sources range 1.5–3 mm. ISO: **MEDIUM**, and ISO's own wording is *permissive*, not required. |
| `EXT_OVERSHOOT` (past the dimension line) | **2.8 pt** (1 mm) | **8 × `LINE_WIDTH`** = 6.0 pt | ANSI: **convention**; mechanical sources say ~1 mm, architectural ~3 mm — the traditions genuinely differ, so this is a *default*, not a rule. |
| `ARROW_LEN` | keep **7.0 pt** (`author.rs:57`) | same | Arrowhead size relative to text height is **UNVERIFIED** in both systems. ISO defers to its **Annex A**, which scales to lettering height *h* — **paywalled** and not obtainable without buying the standard. 7.0 pt against a 10 pt label ≈ 0.7 × text height, inside the plausible range. **Explicitly a pdfce choice, not a standard's.** |
| Arrowhead width | **`ARROW_LEN / 3`** (3:1) | 30° included angle ⇒ half-width = `ARROW_LEN · tan 15°` ≈ `0.268 · ARROW_LEN` | ANSI 3:1 **HIGH**, mandate status **UNVERIFIED**. Note the two are nearly identical (0.333 vs 0.268 half-width ratio) — the visible difference between the standards is *not* the arrowhead proportion. Current code uses `half = ARROW_LEN * 0.35` (`author.rs:301`), close to ANSI 3:1 already. |
| `TEXT_PAD` (break clearance around ANSI text) | **0.5 × text height** | n/a (no break) | pdfce choice. **UNVERIFIED** in any source. |
| `TEXT_GAP` (ISO text baseline above the line) | n/a | **0.25 × text height** ≈ 2.5 pt at 10 pt | **UNVERIFIED as a standards value.** No ISO source gives a number; the value is CAD-template convention (ISO-25 style: 2.5 mm text, 0.625 mm gap). **Named as a pdfce choice.** |
| First-line standoff, chain spacing (rows 10–11) | 10 mm / 6 mm | 10 mm / 6 mm | **Not implemented in Pass 27** — these govern *automatic placement of multiple* dimensions (baseline/chain), which is out of scope (§9.1). Recorded so the numbers are on file when that Pass arrives. |

**These constants live in one place with their provenance in the doc comment**, and the
"convention, not mandated" status is written *into the code*, not just here. Proposed
criterion **E11**: a reviewer reading `author.rs` must be able to tell which numbers pdfce
would have to defend against a standard and which are its own taste.

### 5.7 The drawing rules, implementable

**Text orientation and side** — the two normals must not be conflated (§2.2's canonical
offset normal `n` is *not* the ISO text-side normal):

```
ANSI:
    text matrix = [1, 0, 0, 1, tx, ty]                  // always horizontal
    tx = Mid.x - text_w/2
    ty = Mid.y - 0.35 * LABEL_SIZE                      // cap-height centred on the line
    dimension line drawn as TWO segments, gap centred at Mid, half-width
        hw = (|text_w * u.x| + |text_h * u.y|)/2 + TEXT_PAD
    (the projected-extent formula is constraint-independent: it gives text_w/2 for a
     horizontal dimension and text_h/2 for a vertical one, which is correct because the
     text stays horizontal while the LINE rotates)

ISO:
    u_text = u  if angle(u) in (-90 deg, +90 deg]  else  -u     // never upside-down;
                                                               // ISO cl. 4.1.1's
                                                               // "determined at the centre"
    n_text = perp_ccw(u_text)
    org    = Mid + TEXT_GAP * n_text - (text_w/2) * u_text
    text matrix = [u_text.x, u_text.y, -u_text.y, u_text.x, org.x, org.y]
    dimension line drawn as ONE unbroken segment Pa -> Pb
```

Worked check, `Vertical` constraint, `u = (0, 1)`: `angle(u) = +90°`, which is *inside* the
half-open interval, so `u_text = u` — text reads bottom-to-top. `n_text = perp_ccw((0,1)) =
(−1, 0)` — text sits to the **left** of the line. **That is exactly ISO's "vertical text
reads from the right"** (a reader tilts their head to the right; the text is on the left of
the line). Meanwhile §2.2's canonical **offset** normal for `Vertical` is `(+1, 0)`. **The
two normals point opposite ways for the same dimension.** They are different quantities
answering different questions and must be separate functions with separate tests; writing
one and reusing it is the obvious bug and it would be invisible under ANSI (where text
orientation ignores the normal entirely).

**Terminators:**

```
ANSI:  filled triangle, length ARROW_LEN, half-width ARROW_LEN/3      (3:1)
ISO :  filled triangle, length ARROW_LEN, half-width ARROW_LEN*tan(15 deg)   (30 deg closed-filled)
       (open, oblique-45 deg and dot forms are ISO-legal and are NOT offered in 27.2 -
        one terminator per standard; a terminator picker is a later option, §9.1)
```

**Extension lines** — §2.4's algorithm, with the gap and overshoot resolved per standard
from §5.6. The ANSI/ISO difference is entirely in *how the two constants are computed*
(absolute vs `8 × LINE_WIDTH`), not in the algorithm — which is what row 8's structural
observation buys.

### 5.8 Standard designations, for the record and for the UI

| Designation | Status | Confidence |
|---|---|---|
| **ASME Y14.5-2018 (R2024)** | Current. On **stabilized maintenance**. | **VERIFIED** (asme.org product page) |
| Lineage | **ANSI Y14.5-1973 → ANSI Y14.5M-1982 → ASME Y14.5M-1994 → ASME Y14.5-2009 → ASME Y14.5-2018 (R2024)**. The **"M"** denoted the metric edition and was **dropped from 2009 onward**; the ANSI→ASME prefix change reflects the publishing body, not scope. A title block citing "ANSI Y14.5M-1982" or "ASME Y14.5M-1994" is on a **pre-2009** revision. | **HIGH** |
| **ASME Y14.2-2014** | Current; revision of Y14.2-2008; issued 30 Jan 2015. **The arrowhead / line-convention / lettering authority** (§5.1). | **VERIFIED** |
| **ISO 129-1:2018** (2nd ed., 2018-02) | *"Technical product documentation (TPD) — Presentation of dimensions and tolerances — Part 1: General principles."* **Cancels and replaces ISO 129-1:2004**, *"which has been technically revised."* 2004 in turn descends from ISO 129:1985. **Does not cover application of dimensional tolerances** — that is ISO 14405. | **VERIFIED** (Foreword, Scope §1); 1985 lineage **MEDIUM** |
| European adoption | **EN ISO 129-1:2019**; **BS EN ISO 129-1:2019+A1:2021**. | **MEDIUM** |
| **ISO 128-2** "Basic conventions for lines" | The line-type authority. Current edition **2020**; a **2022 revision is reported but could not be confirmed** (iso.org returned 403). It **amalgamated and superseded ISO 128-20:2001, -21, -22:1999, -23, -24:2014, -25**. | edition year **MEDIUM**; supersession **HIGH** |
| ⚠️ **Staleness trap** | **ISO 129-1:2018's own normative references (cl. 2) still cite ISO 128-20, ISO 128-22 and ISO 128-24:2014 — all three now withdrawn.** Anyone following 129-1's pointer lands on a dead standard. Follow through to **ISO 128-2**. | **VERIFIED** (129-1 cl. 2) + **HIGH** (supersession) |
| **ISO 3098 (all parts)** — Lettering | **Normative for ISO drawings.** ISO 129-1:2018 **cl. 4.1.7**: *"Characters on drawings **shall** be in accordance with the ISO 3098 series."* Parts: 3098-1:2015 (general), 3098-0:1997, 3098-2:2000 (Latin/numerals), 3098-5:1997 (CAD lettering, confirmed 2025). **ISO 129-1 Annex A scales symbol geometry to lettering height *h*, defined as "lettering B vertical according to ISO 3098-0"** — so **ISO 3098 class B is the sizing datum for ISO terminator geometry**, and that is the table pdfce would need for defensible ISO conformance. | clause **VERIFIED** · parts **MEDIUM** |
| **ISO 129-1:2018 Annex C** | *"Former practice"* (informative) — the named home for legacy ISO rendering, should it ever be wanted. | **VERIFIED** (ToC) |

### 5.9 ★ What pdfce cannot claim, and the one thing money would buy

**ISO 129-1:2018 Annex A (normative), "Relations and dimensions of graphical symbols," is
the table that would make pdfce's ISO terminator geometry defensibly conformant, and it is
behind the paywall.** Every ISO numeric proportion in §5.6 is therefore a pdfce choice
informed by CAD-template convention, not a standard's value.

Two consequences, both binding:

1. **pdfce must not claim ISO 129-1 conformance** in any user-facing copy — not in the UI,
   not in the README, not in release notes. The honest phrasing is "ISO-style" or "ISO
   (international) convention." This is the global claim-bearing-copy rule applied exactly:
   a conformance claim is a claim, and pdfce has read five clauses of a document whose
   normative annex it has never seen. Criterion **E9**.
2. **If defensible ISO conformance is ever wanted, buying ISO 129-1:2018 is the only clean
   path.** Recorded as open question **(as)**'s companion — a purchasing decision, which is
   the operator's, not the engineer's. Roughly CHF 150–200 at list.

**A terminology note that generalises beyond this record.** Everything in §5 governs how
**ce dimensions** are drawn. **None of it licenses altering pdf dimensions** already present
in CAD-exported page content — and rows 12–14 are the sharp edge of that: real-world PDFs
routinely violate the very leading-zero and separator rules above, so a **pdf dimension**'s
printed text must never be assumed standard-conformant when parsing it, measuring against
it, or comparing it to a **ce dimension** pdfce authored beside it. A future "does this ce
dimension agree with that pdf dimension?" check must treat `.500`, `0.500`, `0,500` and
`.5` as the same number.

---

## 6. Q5 — Where the drafting standard is chosen

### 6.1 Decision

**Per group**, as a new `Group` field, **defaulted from an application preference whose
factory value is ANSI**.

```rust
/// The drafting standard governing how this group's ce dimensions are DRAWN
/// (§5): terminator form, text placement and orientation, extension-line gap
/// and overshoot. Does NOT govern the numeric string (§5.3).
pub standard: DimStandard,      // on Group

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DimStandard {
    /// ASME/ANSI Y14.5 practice. pdfce's factory default (operator, 2026-08-04).
    #[default]
    Ansi,
    /// ISO 129-1 practice.
    Iso,
}
```

`Group::new` takes it from the preference; the sidecar stores it per group as an optional
`/Standard` name (`/ansi` | `/iso`) defaulting to `ansi` when absent — the same additive,
no-version-bump pattern as `/Offset` (§3.5), for the same reason.

### 6.2 The four options, argued

| Scope | For | Against | Verdict |
|---|---|---|---|
| **Per ce dimension** | maximal flexibility | Nobody wants a drawing where dimension #3 follows ISO and #4 follows ANSI. The flexibility is a foot-gun with no use case, and it multiplies the property-bar surface. | rejected |
| **Per group** ✔ | The group **already** owns every other display-governing property: scale, unit, number format, layer. The standard is the same class of thing. Regeneration machinery already exists and is already per-group (`set_group_scale` → `regenerate_dimension_writes` over `members(group)`). Mixed-standard documents are possible but never accidental. | Slightly more state than a document-wide setting. | **chosen** |
| **Per document** | Matches how drawings are actually produced (one drawing, one standard). | pdfce has no document-level dimension settings *at all* — the model is `DimensionModel { groups, dimensions }` and nothing else. Inventing a document tier for one field means inventing merge semantics for "document says ISO, group says ANSI." A "set all groups" action gets the same result with no new tier. | rejected |
| **App preference only** | Simplest. | The setting would not travel with the file. Open the drawing on another machine, regenerate for a scale change, and the ce dimensions silently restyle. Unacceptable — the file must carry its own appearance. | rejected **as the storage**, adopted as the **default source** |

The preference-supplies-the-default arrangement is what makes the operator's exact sentence
true: *"My default is ANSI, but ISO should be an option too."* The word **default** is doing
work there — he is describing a preference, not a per-file choice, and per-group storage
with a preference-sourced default gives him both.

**The unit interaction seals it.** §5.3's decimal conventions are unit-dependent (the
leading-zero rule is an *inch* rule). `format.unit` is per group. A standard stored at any
other altitude than the unit's would create combinations no single code path owns.

### 6.3 What happens to existing ce dimensions when the standard changes

**All members of that group regenerate, immediately, exactly like a scale change.** No
opt-in, no "apply to new only."

This follows the precedent already set and tested by `set_group_scale`
(`edit.rs:6181`) and its test `changing_group_scale_regenerates_all_member_labels`. The
argument is the same argument decision 011 §2.3 made for scale: a group exists so that its
members agree. A group whose members are drawn to two different standards is not a group
with a setting — it is a group with a history, and the operator would have to remember
which dimensions predate the change. That is precisely the state the group model exists to
prevent.

Mechanically it is nearly free: `regenerate_dimension_writes` already re-runs
`author_dimension` for a list of ids and already reads the group for `scale` and `format`;
it gains `standard` from the same `group` binding. The new edit-session verb
`set_group_standard(GroupId, DimStandard)` is a near-copy of `set_group_scale` and is one
undoable command.

**It is not silent.** The regeneration changes what every member looks like, which is a
larger visible change than a scale edit (a scale edit changes numbers; this changes shapes).
The group panel discloses the member count before the change is applied, in the same idiom
`delete_group`'s reassignment count already uses. Proposed criterion, §7.3 C6.

### 6.4 The API changes this forces, and how to keep them non-breaking

Two public signatures are affected. Both are `pdfce-core` `pub` items, so rule 10 applies
(`D:\dev\rag\rust\rust-style-guide-and-api-guidelines.md`).

**`author_dimension`** — currently `(kind, scale, format)`. The clean, guideline-conformant
move for a function that has reached four parameters is to take a struct:

```rust
/// Everything the appearance of one ce dimension depends on, besides its geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DimensionStyle {
    pub scale: ScaleState,
    pub format: NumberFormat,
    pub standard: DimStandard,
}

pub fn author_dimension(kind: &DimensionKind, style: DimensionStyle) -> AuthoredDimension;
```

This is a breaking change to a `pub fn`, with **6 call sites** (4 in `author.rs`'s own
tests, 1 in `edit.rs:6087`, 1 in `edit.rs:6494`) — all inside the workspace. Take it: the
alternative is a five-positional-parameter function whose call sites read
`author_dimension(&k, s, f, std)` with three same-shaped arguments in a fixed order, which
is the exact pattern the API guidelines warn about. The purity contract in `author.rs:28-33`
is *strengthened*, not weakened: it becomes "a pure function of `(kind, style)`," and
`DimensionStyle` is trivially derivable from a `Group`, so `impl From<&Group> for
DimensionStyle` keeps every call site to one line.

**`format_measurement` and `NumberFormat`** — the research revised this. ISO 129-1:2018
cl. 4.1.1 **mandates** a comma decimal marker (§5.2 row 12, VERIFIED), so the numeric string
is not entirely standard-independent after all. The resolution keeps the value path free of
`DimStandard` anyway:

```rust
pub struct NumberFormat {
    pub unit: Unit,
    pub fraction: FractionMode,
    pub decimal_marker: DecimalMarker,   // NEW. Point | Comma. Default Point.
}
```

`NumberFormat` is exactly what `measure_dict` projects into a §12.9 `NumberFormat` dict, and
`/RD` is exactly that dict's decimal-marker key — so the marker's home in pdfce mirrors its
home in the spec, and the "agree by construction" claim becomes structurally true rather
than aspirational (§5.4). `format_measurement`'s **signature is unchanged**; it reads the
marker off the `NumberFormat` it already receives. `set_group_standard` sets
`format.decimal_marker` as a **disclosed** side effect, which the operator may override —
so question **(at)** is answerable in either direction with no further code change.

Blast radius: the three `NumberFormat` constructors (`decimal`, `inch_fraction`,
`feet_inches`) absorb the default; the one struct literal outside them is
`sidecar.rs:172`, which gains a tolerant read of an optional `/Marker` key defaulting to
`Point` — the same additive pattern as `/Offset` and `/Standard`, for the same reason.
`measure_dict::nf_dict` gains `/RD` **and** `/RT` (§5.4: setting `/RD` to a comma without
also setting `/RT` produces `1,234,56`).

**`DimStandard` therefore never reaches the value path at all** — only the drawing path.
That is what preserves R139's boundary with the marker as its single, argued exception.

---

## 7. Pass plan — family 27

Four Passes. **27.0 and 27.2 are independent of the operator's answer to §4.7; 27.1 is
gated on it; 27.3 exists only under one answer.** Rule 11 applies to each: the `pdfce-cli`
subcommand ships in the same Pass as the GUI flow.

### 7.1 Pass 27.0 — Correct linear ce-dimension geometry: axis-aligned dimension line, real extension lines, the offset field (core + CLI)

**Not gated. Ships first. Fixes the reported defect.**

Scope: §2 (all of it, including §2.6 small-space), §3 (the field, `translated`, the sidecar
key), §3.6 (the version-gate widening + `SidecarWrittenByNewerBuild`), and replacing
`estimate_text_width` with real AFM widths (`fontdata::std14_width` + `encoding_glyph_name`,
both already present) — required because ANSI's broken dimension line needs the gap to match
the text, and a 0.5-em estimate visibly does not.

CLI: `dimension-add --offset <pt>` (default `0`); a new `dimension-offset --dimension <id>
--offset <pt>`; `dimension-list` prints each linear ce dimension's offset.

| # | Acceptance criterion |
|---|---|
| C1 | **Property test**, randomised over point pairs × 3 constraints × random offsets: `\|Pb − Pa\| == kind.measured_points()` to within 1e-9. |
| C2 | A `Horizontal` ce dimension with `a.y ≠ b.y` bakes a dimension line whose two `/L` points share a `y`, and whose `y` equals `a.y + offset`. Vertical: same for `x`. |
| C3 | Both extension lines are emitted with the correct `gap` and `overshoot`, and each is **omitted** when its span `≤ gap`. A case where the two extension lines point in *opposite* directions has its own test. |
| C4 | **`a_pre_offset_sidecar_deserialises_with_offset_zero_and_loses_nothing`** — §3.7, verbatim, including the hand-built (not serializer-built) version-1 dict. |
| C5 | **`a_sidecar_with_an_unknown_future_key_still_round_trips`** — §3.7's partner. |
| C6 | A sidecar with `/Version` **greater** than `SIDECAR_VERSION` produces `SidecarWrittenByNewerBuild` on any write attempt, and the write is refused **before** any mutation (rule 4). The existing model is not overwritten. |
| C7 | `offset = 0.0` on an already-axis-aligned pick produces a **byte-identical** `/AP` stream to `b3474b8`'s output for the same input — the "no unintended visual change" guard. (Ticks→extension lines will differ for the non-axis-aligned case by design; C7 pins only the case that must not move.) |
| C8 | Degenerate inputs (`a == b`, `t == 0`, non-finite offset) produce a re-parseable `/AP` and a positive-area `/Rect`; extends `appearance_content_reparses_as_a_content_stream`. |
| C9 | `cargo tree -p pdfce-core` unchanged (no new dependency; the AFM widths are already in-crate). |
| C10 | `cargo fmt --check` + `cargo clippy -- -D warnings` clean workspace-wide. |
| C11 | CLI round-trip: `dimension-add --constraint horizontal` on two points at different heights, then `dimension-list`, then reload — offset survives; `--verify-undo` passes. |

### 7.2 Pass 27.1 — The offset drag (GUI) — **GATED on §4.7**

Scope under answer (1): constrain `run_dimension_drag` to the canonical normal; replace the
bbox-outline preview with a live redraw of the ce dimension at the trial offset; commit via
a new `EditSession::set_dimension_offset`. Whole-move demoted per the operator's answer to
(3). Dispatch `pdfce-ui-specialist` for the grab-target design (line vs text vs extension
line) and for the cursor/affordance — this is a new interaction pattern, not a tweak.

| # | Acceptance criterion |
|---|---|
| D1 | Dragging a `Horizontal` ce dimension moves the dimension line only along `y`; `a`, `b`, `constraint` and `measured_points()` are bit-identical before and after. Vertical: `x`. Aligned: along `n`. |
| D2 | The drag preview shows the **redrawn ce dimension**, not a bbox — the shape changes during the drag and the feedback must show it. |
| D3 | One undoable command; Ctrl+Z restores the prior offset exactly. |
| D4 | Pointer leaving the window mid-drag abandons rather than commits (preserves `main.rs:12383-12387`'s existing, correct behaviour). |
| D5 | Whatever §4.7(3) decides about whole-move is implemented **and** its disposition is recorded in `ROADMAP.md` — including "removed," if that is the answer. |

### 7.3 Pass 27.2 — ANSI and ISO drafting standards (core + CLI + GUI)

**Not gated.** Scope: §5's drawing rules, §6's per-group `standard` field + preference-
sourced default, `DimensionStyle`, `set_group_standard` with full-group regeneration.

CLI: `group-set-standard --group <id> --standard <ansi|iso>`; `group-add --standard`;
`dimension-list` prints each group's standard.

| # | Acceptance criterion |
|---|---|
| E1 | A horizontal ce dimension under **ANSI** bakes a dimension line in **two** stroked segments with a gap centred on the text; under **ISO**, **one** unbroken segment with the text above it. Asserted on the content stream, not on a rendered image. |
| E2 | A **vertical** ce dimension under ANSI bakes text with an identity text matrix (horizontal, unidirectional); under ISO, a rotated text matrix, with the rotation chosen so the text is never upside-down. Both assertions on the `Tm` operands. |
| E3 | Terminator form follows §5.2 for each standard. |
| E4 | `set_group_standard` regenerates **every** member of the group in one undoable command (mirrors `changing_group_scale_regenerates_all_member_labels`). |
| E5 | A group with no `/Standard` key in its sidecar dict deserialises to `Ansi` and loses nothing else — the §3.7 migration test, repeated for the group half. |
| E6 | Changing a group's standard discloses the member count before applying. |
| E7 | The factory default for a new install is **ANSI** (operator, 2026-08-04), asserted in a test so a later refactor cannot flip it silently. |
| E8 | **The §5.3 boundary, pinned:** for the same `(kind, scale, format)`, the baked label text is identical under ANSI and ISO **except** for the decimal marker. `DimStandard` appears nowhere in the value path — asserted by a test that formats under both standards with the marker held fixed and gets byte-identical strings. |
| E9 | **No conformance claim ships.** The operator-facing strings read "ANSI / ASME (US)" and "ISO (international)", never "ASME Y14.5", "ASME Y14.2" or "ISO 129-1". §5.1, §5.9. Asserted against `ui_text`, so a later copy edit cannot reintroduce one. |
| E10 | Selecting ISO emits **both** `/RD (,)` **and** a non-comma `/RT` in the `/Measure` dict; a test asserts a four-digit value does not render `1,234,56`. §5.4. |
| E11 | Every geometry constant in §5.6 carries its provenance in its doc comment — **which standard, which confidence, and "convention, not mandated" where that is the truth.** A reviewer must be able to tell pdfce's taste from a standard's requirement by reading `author.rs` alone. |

### 7.4 Pass 27.3 — Text position along the dimension line — **exists only under §4.7 answer (2)**

`text_along: f64` on `DimensionKind::Linear`, same additive sidecar pattern, drag on the
text grab. Not scoped further until the answer is in.

### 7.5 Sequencing

```
27.0  (independent)  ──┬──> 27.1  (gated on §4.7)  ──> 27.3 (only under answer 2)
                       └──> 27.2  (independent)
```

27.0 must land before either 27.1 or 27.2 — both depend on the offset field and on the
corrected `draw_linear`. 27.1 and 27.2 do not depend on each other and can be taken in
either order; taking **27.2 first** is recommended if the operator's answer is slow, since
it is unblocked and delivers the second half of the report.

**Interaction with decision 024.** Pass 24.2 migrates the three floating property bars into
ribbon contextual tabs. The standard picker (§6) lands in the measure property bar at
`main.rs:12672-12691` and in the group panel; if 24.2 lands first, it lands in the
contextual tab instead. Flagged so the two Passes do not both invent a home for it.

---

## 8. Proposed standing rules (librarian assigns final numbers; R130–R134 are claimed by decision 025)

**R135 — A ce dimension's drawn dimension line is exactly as long as its printed value.**
`|Pb − Pa| == kind.measured_points()` for every linear ce dimension, under every constraint
and every offset. Any change to the appearance path carries a property test asserting it.
*Why:* this is decision 026's whole defect, stated as an invariant. A measurement drawing
whose line length contradicts its own number is worse than no drawing.

**R136 — What the pick preview shows is what authoring bakes.** Any divergence between a
tool's on-canvas preview geometry and the geometry its commit produces is a defect, not an
optimisation, regardless of how the stored representation is justified. *Why:* the shipped
divergence (§1.2) was introduced for a good reason (CLI byte-equivalence), documented
honestly, and still shipped a bug — because the justification covered the *value* and
nobody checked the *drawing*.

**R137 — Sidecar schema changes are additive at the current version by default.** A new
field goes in as an optional key with a documented absent-value default, and
`SIDECAR_VERSION` is bumped only when a change genuinely cannot be expressed additively.
Every such addition ships a test that deserialises a **hand-built** older dict — never one
produced by the current serializer — and asserts nothing else is lost. *Why:* §3.6. The
version gate discards the *entire* model on mismatch, and the discard is invisible because
the annotations keep rendering.

**R138 — A sidecar written by a newer build is a refusal, never a silent reset.** When
`/Version` exceeds what this build writes, pdfce refuses to write the sidecar
(`SidecarWrittenByNewerBuild`) and discloses, rather than starting a fresh model and
overwriting on save. *Why:* same family as R-`DeleteWouldMoveNextSubpath` (decision 025
§5.5) — a byte-minimal operation that destroys unrecoverable operator work, catchable only
by a named refusal.

**R139 — The drafting standard governs how a ce dimension is DRAWN, never what it MEASURES;
and any presentation rule it does reach must be expressible in the portable `/Measure`
projection.** Terminators, text placement/orientation, extension-line gap and overshoot
follow `DimStandard`. The numeric string stays governed by `NumberFormat` — including ISO's
mandated comma, which lives there because that is where §12.9's `/RD` lives, so selecting
ISO changes one field that feeds *both* the printed label and the portable dict. A
presentation rule that **cannot** be expressed in Table 263 (`/U /C /F /D /FD /RT /RD /PS
/SS /O`) is not implemented — ANSI inch leading-zero suppression is the live example.
*Why:* §5.3/§5.4. A rule pdfce can print but cannot project makes its own label and its own
portable mirror disagree about the same file, permanently and invisibly — §5.5 shows that
has already happened once.

---

## 9. What this record does NOT decide

### 9.1 Explicitly out of scope

| Item | Why deferred |
|---|---|
| **The drag semantics** | §4.7. The one item put to the operator rather than decided. |
| **Text position along the dimension line** | Pass 27.3, and only under one answer to §4.7. |
| **`DimensionKind::Circular` geometry** | The standards also govern radial/diameter leaders (jogged leaders, centre marks, `⌀` placement, the "leader outside the circle" case). A real slice, not a footnote. The `standard` field applies to circular *text* placement in 27.2 where it is cheap; the leader geometry rework is not scoped here. |
| **Baseline / chain / ordinate dimensioning** | Multiple dimensions sharing a datum, with automatic offset stepping. Wants the offset field this record adds, and is a natural follow-on, but is a new authoring gesture. |
| **Tolerances, limits, GD&T feature control frames** | ASME Y14.5's actual subject matter. Enormous. Not requested. |
| **Dimension text overrides** (typing a value that differs from the measurement) | Deliberately not offered. Rule 4 — a measurement product that lets the printed number diverge from the measured one is the sneakiest possible feature. If ever requested, it needs its own decision. |
| **A nonzero default offset ("auto standoff")** | §3.3. Rejected as a default because the sign cannot be inferred; offered as a preference, question **(ar)**. |
| **Editing `a`/`b` (re-anchoring)** | Decision 022 §4.2 already deferred this to a Measure-tool slice with a live pre-commit preview. Unchanged by this record. |
| **The `/FD` decimal divergence** (§5.5) | Real, found here, but a separate defect with a separate blast radius (it changes the `/Measure` bytes of every regenerated ce dimension). Filed for the librarian as a Backlog item, not folded into Pass 27 — and it must land **before** any trailing-zero work. |
| **ANSI leading-zero suppression, ASME trailing-zero rules** | §5.3 items 1–2. Not expressible in ISO 32000-1 §12.9 Table 263, so implementing them would create a permanent invisible divergence between pdfce's label and every conforming reader's computation of the same file. Needs its own decision about which side of that divergence to accept. Question **(at)**. |
| **Unit-symbol omission** | §5.3 item 3. Both standards omit the unit per dimension and state it once for the drawing; pdfce prints it on every label and keeps doing so, deliberately — there is no drawing-level note to carry the statement, `/Measure`'s `/U` is required regardless, and a bare `3.10` beside **pdf dimensions** in another unit is an ambiguity a measurement tool should not create. A per-group toggle is a reasonable future option. |
| **A terminator picker** (open arrowhead, oblique 45°, dot, ISO origin circle) | All ISO-legal (§5.2 row 4). Pass 27.2 ships **one** terminator per standard. A picker is a later option and needs the discipline conventions (§5.2 row 5) to inform its default, not just its list. |
| **ISO 129-1:2018 Annex A conformance** | §5.9. The normative symbol-proportion table is paywalled; every ISO numeric proportion pdfce uses is a CAD-convention-informed pdfce choice. Buying the standard is a purchasing decision, not an engineering one. |

### 9.2 What needs the operator's own call

1. **§4.7's three questions** — the drag semantics. Blocking for Pass 27.1 only.
2. **Question (ar)** — should newly authored ce dimensions get a nonzero default standoff?
3. **Question (as)** — is a mixed-standard document (per-group) acceptable, or should the
   standard be forced document-wide? §6.2 chose per-group on structural grounds; if the
   operator has a strong preference for one-drawing-one-standard, a "set all groups" action
   plus a warning is a small change.
4. **Question (at)** — the ANSI **inch** number conventions. ISO's mandated comma is
   handled (§5.4), but two well-attested ASME inch rules are **not** implemented and the
   reason is a real trade-off, not laziness: **leading-zero suppression** (`.500`) and
   **trailing-zero padding** are not expressible in ISO 32000-1 §12.9 Table 263, so
   implementing them makes pdfce's printed label and every conforming reader's computation
   of the *same file* disagree permanently and invisibly. Against that: real US practice is
   not uniform — a respected engineering-school rule sheet teaches the opposite (§5.2 row
   14). Does the operator want `.500` on his ANSI drawings badly enough to accept that
   divergence?
5. **Question (au)** — should ISO 129-1:2018 be **purchased**? Its normative Annex A is the
   symbol-proportion table that would make pdfce's ISO terminator geometry defensibly
   conformant instead of convention-informed (§5.9). Until then pdfce must say "ISO-style,"
   not "ISO 129-1 conformant" (criterion E9). ~CHF 150–200. A purchasing decision, and
   therefore not the engineer's.

### 9.3 Amendment table

| Record | Statement | Status after 026 |
|---|---|---|
| 011 §2.3 | "GEOMETRY is **immutable** and stored […] the DISPLAYED value is **derived**" | **Intact.** `offset` is stored geometry participating in the same regeneration path; the value is still derived from `(a, b, constraint)` alone and is untouched by `offset`. |
| 011 §2.3 | "Rendered as an authored `/Line` annotation […] a baked `/AP` leader + **extension lines** + value text" | **Not yet true; 026 makes it true.** The shipped code emits 8-pt ticks, not extension lines (§1.4). 011 specified the right thing; the implementation under-delivered and nothing caught it. |
| 022 §4.2/§4.3 | Dimension gestures split two ways: translate (safe) vs re-measure (deferred) | **Extended, not reversed.** A third class exists: the offset drag, which changes neither the measured points nor the value, and is safer than translate (§4.8). 022's refusal to let the generic Obj tool touch dimension geometry stands unchanged. |
| 022 §4.3 | "a whole-dimension move is semantically honest" | **Still true arithmetically, now questioned operationally** (§4.2): it is value-preserving *and* detaches the ce dimension from the feature it measures. Whether it survives as a gesture is §4.7(3). |
| `author.rs:75-81` module doc | the raw-`b` storage rationale | **Incomplete, not wrong.** The rationale covers the value and omits the drawing. 026 keeps raw `b` (it is the extension-line anchor) and adds the missing half. |
| `units.rs:225-227` doc on `NumberFormat::format` | pdfce's label and a §12.9 reader's "agree by construction" | **False today** (§5.5), independently of this record. |

---

## 10. Open operator questions claimed

| Letter | Question |
|---|---|
| **(aq)** | The drag semantics — §4.7's three-part question. **Blocking for Pass 27.1.** |
| **(ar)** | Default standoff for newly authored ce dimensions: keep `0.0` (dimension line through the first picked point, matching the preview), or apply a nonzero default and pick a side? |
| **(as)** | Is per-group drafting standard right, or should it be forced document-wide? |
| **(at)** | Implement ANSI **inch** leading-zero suppression (`.500`) and trailing-zero padding, accepting a permanent invisible divergence between pdfce's label and any §12.9-honouring reader? Or keep `0.500` and stay portable? |
| **(au)** | Purchase ISO 129-1:2018 (~CHF 150–200) for its normative Annex A symbol proportions, so pdfce's ISO geometry can be defensibly conformant rather than convention-informed? |

---

## 11. Risks to the two load-bearing invariants

**GUI-core separation (rule 2).** Zero risk. Every new type (`DimStandard`,
`DimensionStyle`), every new function (the canonical normal, the extension-line builder, the
standard-specific text placement) is `pdfce-core`. The GUI contributes a drag vector and a
picker; `pdfce-render` is untouched. `cargo tree -p pdfce-core` is criterion C9 anyway.

**Round-trip / minimal-diff (rule 3).** Low, and the shape is already established. Every
operation here is a regeneration of objects pdfce authored: the `/AP` stream and the
annotation dict (starting from the existing dict and overwriting only
`AUTHORED_ANNOT_KEYS`, `edit.rs:6511-6519`), plus the catalog `/PieceInfo`. No page content
stream byte is touched — the R46 zero-exception overlay-append discipline is unchanged. The
one thing to watch is C7: a regeneration triggered for an *unrelated* reason (a scale
change) must not silently restyle ce dimensions authored before Pass 27.2, which is why the
`standard` field is per group with an explicit `Ansi` default on absence rather than "the
current preference at regeneration time." **A preference read at regeneration time would be
a round-trip violation with a moving cause** — the file would render differently on two
machines — and is rejected for exactly that reason.

**Spec fidelity (rule 1).** `/L` semantics (§2.7) and the `/Measure` `NumberFormat` keys
(§5.4) were read from `D:\Dev\Rag-Specialized\PDF_Spec\iso32000\iso32000__s__12.9.md` and
cited, not recalled. The drafting standards themselves are **not** PDF specs and are not in
that RAG; §5.1 states exactly which of their facts are verified and which are not.

---

## 12. JSON

```json
{
  "decision": "Split the linear ce-dimension model into MEASURED POINTS (a, b - already stored, kept raw) and a DERIVED DIMENSION LINE, by adding one signed scalar `offset: f64` to DimensionKind::Linear. Draw the dimension line along the constraint axis at that offset, with real extension lines (gap + overshoot) reaching back to each measured point. Store the offset as an OPTIONAL sidecar key at version 1 (no version bump) defaulting to 0.0, which reproduces both today's committed geometry for axis-aligned picks and the two-click PREVIEW for every pick. Add a per-group `standard: DimStandard` (Ansi | Iso) defaulted from an app preference whose factory value is ANSI, governing how a ce dimension is DRAWN. Its one reach into the numeric string is ISO's MANDATED comma decimal marker (ISO 129-1:2018 cl. 4.1.1, verified 'shall'), which is placed on `NumberFormat` - not on `DimStandard` - because that is what projects into §12.9's `/RD`, keeping the label and the portable mirror structurally in agreement. Do NOT decide the drag semantics - put the operator's contradictory sentence back to him as a written question.",
  "confidence": "high on the geometry, the offset model, and the sidecar migration (all verified in shipped code at b3474b8); high on per-group standard scoping; DELIBERATELY UNDECIDED on drag semantics",
  "reasoning": "author.rs::leader_endpoints discards the AxisConstraint via a `..` pattern and draw_linear strokes straight between the two picked points, so a Horizontal ce dimension reports |dx| and draws sqrt(dx^2+dy^2). The deeper defect is a preview/commit divergence: measure_tool.rs::preview_segment draws the CONSTRAINED segment while commit_point stores the RAW second pick, so the operator is shown one line and given another. Keeping raw b is correct - it is the anchor of the second extension line - so the fix is to teach the appearance path that b is a measured point, not a dimension-line endpoint. That requires exactly one new degree of freedom (where the dimension line sits perpendicular to the axis), which is one signed scalar. Basing it at `a` with default 0.0 makes the baked appearance identical to the preview, closing the divergence rather than narrowing it. The standard belongs on the group because the group already owns scale, unit, number format and layer - every other display-governing property - and because the ANSI decimal conventions are unit-dependent and the unit is per group.",
  "alternatives": [
    {"option": "Project b at commit time (store the constrained point)", "tradeoff": "Fixes the angle and destroys the information the extension lines need. The second measured point would be lost, so 'extend the lines to the connection points' becomes unimplementable forever.", "when_use": "Never."},
    {"option": "Store a third Point (AutoCAD dimension-line-location model)", "tradeoff": "Two stored numbers for one degree of freedom; the along-axis component is dead state that a drag writes and nothing reads. Needs translating in translated(). Two representations of one truth drift (R92).", "when_use": "If an oblique/rotated dimension KIND is added later - and that is a new variant with its own representation, not a change to this one."},
    {"option": "Bump SIDECAR_VERSION to 2 for the offset", "tradeoff": "deserialize_model gates on EXACT version equality and returns None on mismatch, so every pre-existing dimensioned file would silently lose all groups, all calibrated scales and all membership while its /Line annotations kept rendering correctly. Then overwrite on next save.", "when_use": "Never for an additive field. Only for a change that genuinely cannot be expressed additively, and then only with a per-version read path."},
    {"option": "Drafting standard per document or as an app preference only", "tradeoff": "Per-document invents a tier the DimensionModel does not have, plus merge semantics. Preference-only means the setting does not travel with the file, so the same PDF restyles on another machine at the next regeneration - a round-trip violation with a moving cause.", "when_use": "The preference is retained, but as the SOURCE OF THE DEFAULT for new groups, not as the storage."},
    {"option": "Put the decimal marker on DimStandard rather than on NumberFormat", "tradeoff": "ISO 129-1:2018 cl. 4.1.1 MANDATES a comma (verified 'shall'), so the marker must be governed by the standard somehow. But NumberFormat is what measure_dict projects into a §12.9 NumberFormat dict, and /RD IS that dict's decimal-marker key. Putting the marker on DimStandard would mean two structures owning one fact and would force DimStandard into the value path.", "when_use": "Never. Put it on NumberFormat; have set_group_standard SET it as a disclosed side effect the operator can override. That makes the 'agree by construction' claim structurally true and answers question (at) in either direction with no code change."},
    {"option": "Implement ANSI inch leading-zero suppression (.500) and trailing-zero padding", "tradeoff": "Well-attested ASME rules, but NOT expressible in ISO 32000-1 §12.9 Table 263 - there is no key for suppressing a leading zero. Implementing them makes pdfce's printed label and every conforming reader's computation of the SAME FILE disagree permanently and invisibly. Real US practice is also not uniform: a respected engineering-school rule sheet teaches the opposite.", "when_use": "Only if the operator answers question (at) that he wants .500 badly enough to accept the divergence - and then the divergence gets disclosed, not hidden."}
  ],
  "implementation_guidance": "Pass 27.0 first and ungated: canonical-normal helper (named fn + test - 'which way is positive' must have one home), corrected leader/extension/terminator geometry, small-space flip, `offset: f64` on DimensionKind::Linear (23 compile-enforced construction sites), one-line carry in translated(), optional /Offset sidecar key read with unwrap_or(0.0), widen the sidecar version gate to a range, add SidecarWrittenByNewerBuild, and replace estimate_text_width with fontdata::std14_width + encoding_glyph_name (already in-crate, needed for ANSI's break gap). CLI: dimension-add --offset, dimension-offset, dimension-list prints offset. Then Pass 27.2 (also ungated): DimStandard (Ansi | Iso) per Group with a preference-sourced ANSI factory default; DimensionStyle replacing author_dimension's three trailing params (6 call sites, all in-workspace); NumberFormat gains decimal_marker (Point | Comma) which measure_dict projects to /RD - and /RT MUST be set alongside it or grouped numbers read 1,234,56; set_group_standard regenerates every member in one undoable command and discloses the count first. ANSI draws a BROKEN dimension line with horizontal text centred in the gap; ISO draws an UNBROKEN line with text rotated to align, never upside-down, resolved at the dimension's centre (ISO cl. 4.1.1 - NOT the 30-degree folklore, which is not in the standard). Extension gap/overshoot are ABSOLUTE for ANSI and 8x line-width for ISO - one structural difference reproducing both traditions. The canonical OFFSET normal and the ISO TEXT-SIDE normal point OPPOSITE ways for a vertical dimension; they must be separate functions with separate tests. Pass 27.1 (the drag) does not start until the operator answers §4.7.",
  "risks_gotchas": [
    "The sidecar version gate is an exact-equality check whose failure mode is TOTAL SILENT MODEL LOSS with the annotations still rendering. Do not bump the version. Widen the gate in the same Pass.",
    "The two extension lines can point in OPPOSITE directions (dimension line placed between two measured points at different heights). An implementation that computes one direction from `offset` alone gets this wrong and it will not show up in an axis-aligned test.",
    "The canonical offset normal and the ISO text-side normal are DIFFERENT for a vertical dimension (offset normal is canonicalised to +x; ISO text sits on the -x side). Conflating them is the obvious bug.",
    "estimate_text_width's 0.5-em guess is adequate for centring and NOT adequate for ANSI's broken dimension line, where the gap must match the text. Real AFM widths are already available in-crate.",
    "Regeneration must read the standard from the GROUP, never from the live app preference - otherwise the same file renders differently on two machines, which is a round-trip violation with a moving cause.",
    "author_dimension's purity contract (`a pure function of (kind, scale, format)`) is load-bearing for move_dimension and set_group_scale. Keep it by widening to (kind, style), not by reading anything ambient.",
    "ANSI is NOT ASME Y14.5 for anything pdfce draws - Y14.5 is GD&T/tolerancing. Arrowheads, line conventions and lettering are ASME Y14.2-2014 (S4 Line Conventions, S5 Arrowheads, S6 Lettering, Fig 5-1 'Arrowhead Styles', Table 6-1). Verified from ASME's own front matter.",
    "The widely-taught ISO '30 degrees either side of vertical' ambiguous-zone rule is NOT in ISO 129-1:2018. The standard's actual tie-break is 'the determination of orientation is based on the centre of the dimension' (cl. 4.1.1, verified). Implement that, not the folklore.",
    "Setting /RD to a comma for ISO without also setting /RT breaks grouped numbers: /RT's spec default is ALSO a comma, so 1234.56 renders as 1,234,56.",
    "The canonical offset normal and the ISO text-side normal point OPPOSITE ways for a vertical ce dimension (offset normal canonicalised to +x; ISO text sits on the -x side, which is what 'vertical text reads from the right' means). One function reused for both is the obvious bug, and it is invisible under ANSI where text orientation ignores the normal entirely.",
    "pdfce must not claim ISO 129-1 conformance in any user-facing copy: its normative Annex A symbol-proportion table is paywalled and unread, so every ISO numeric proportion pdfce uses is a CAD-convention-informed pdfce choice. 'ISO-style', not 'ISO 129-1 conformant'.",
    "ISO 129-1:2018's own normative references still cite ISO 128-20, 128-22 and 128-24:2014 - all three withdrawn and superseded by ISO 128-2. Following 129-1's pointer lands on a dead standard.",
    "SIDE FINDING, pre-existing: pdfce prints fixed decimal places (3.10 m) while the mirrored /Measure dict omits /FD, whose default false permits a conforming reader to truncate to 3.1 m. NumberFormat::format's doc claims they agree by construction. They do not. Fix is `fd: true` on the decimal arm, but it changes the /Measure bytes of every regenerated ce dimension, so it is a separate Backlog item - and it must land before any trailing-zero work.",
    "Real-world pdf dimensions do NOT reliably follow the leading-zero or decimal-separator rules above (a respected engineering-school rule sheet teaches the opposite of ASME's inch rule). Any future 'does this ce dimension agree with that pdf dimension?' check must treat .500, 0.500, 0,500 and .5 as the same number."
  ],
  "rationale_for_docs": "The defect the operator reported is three lines of code, but its shape is a preview/commit divergence - pdfce showed a correct horizontal preview and baked a diagonal line - which is the same category of harm as rule 4's fuzzy-never-sneaky. Fixing it properly requires separating what was measured from where the dimension line is drawn, which is one signed scalar, and that scalar is also the state the operator's 'extend the lines to the connection points' request needs. The offset defaults to zero specifically so the migration is free and the baked geometry converges on the preview. The drafting standard is a per-group property for the same reason scale and units are: a group exists so its members agree.",
  "not_decided": {
    "drag_semantics": "The operator's sentence contains two clauses that point opposite ways: 'only be able to drag it horizontally' (along the axis - a geometric no-op under the offset model, and a detach-from-the-feature under the shipped whole-move model) versus 'so it stays in line with what it is actually measuring' plus 'extend the lines to the connection points' (perpendicular - the CAD convention, and the only reading under which extension lines are a thing that happens). Recommendation is PERPENDICULAR with high confidence, but the question is written out and put to him verbatim in §4.7 rather than resolved silently, including a third possibility (that he meant sliding the NUMBER along the line, which IS an along-axis drag both standards allow) and a question about whether today's whole-move gesture should survive."
  },
  "ledger": {
    "worktree_stale": false,
    "worktree_head": "b3474b8",
    "branch_head": "b3474b8",
    "decision_number": 26,
    "pass_family": 27,
    "passes": ["27.0", "27.1", "27.2", "27.3"],
    "proposed_rules": ["R135", "R136", "R137", "R138", "R139"],
    "open_questions": ["(aq)", "(ar)", "(as)", "(at)", "(au)"]
  }
}
```
