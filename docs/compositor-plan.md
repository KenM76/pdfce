# The colorant compositor — plan of record

**Written 2026-08-18**, engineer-owned, at `2a75be1`+`e618d67`.
Companion to `docs/overprint-architecture-survey.md` (the sourcing record for
the colorant half) and `docs/ghent-patch-reference.md` (the per-patch expected
appearance for the overprint patches).

> ## ★★ AMENDMENT 2026-08-19 — THE DENOMINATOR MOVED AND ONE PROBED PATCH
> ## NOW PASSES. Re-derive the thesis before scoping `Pass 97.x` from it.
>
> This plan is a **live plan**, not a dated record, and `Pass 97.x` is scoped
> from it — so its arithmetic being stale is a live problem rather than a
> historical footnote.
>
> **The standing is now `26 pass · 14 FAIL · 11 UNRESOLVED`**, not the
> `25 · 18 · 8` in §1 below. Two independent things moved it:
>
> 1. **A classifier shift that predates this session.** `18+8 = 15+11 = 26`
>    and `pass` was 25 either way, so **three patches crossed the
>    FAIL/UNRESOLVED line without any patch changing outcome.** Found by
>    building the previous commit in a worktree rather than quoting the board.
>    Filed as a reading, not a verified cause.
> 2. **`1_GWG160` genuinely passes now** — the non-separable blend modes
>    (`Pass 85.4b`, `972ddbb`) shipped, and it was the patch they flipped.
>
> ★ **Point 2 is the one that touches this document's argument, not just its
> numbers.** `1_GWG160` is one of the five transparency-group failures §1.1
> probes cell by cell, and it was resolved by **Table 137 arithmetic, not by
> a compositor** — with no CMYK buffer, no non-transparent initial backdrop
> and no `Pass 97.0`. So the claim that these failures "do not decompose"
> has one counter-example, and **"16 of the 18" cannot be re-stated as
> "16 of the 14"**.
>
> **What that does NOT mean:** the compositor case is not refuted. Overprint
> still needs per-colorant planes, and the measured negative result behind
> that (`ac15158` — a spot-ink multiplier plate built, ablated and reverted)
> stands untouched.
>
> **What it does mean:** the share is unknown until someone re-derives it,
> and this plan should not be quoted for a figure until they have. The
> current fourteen, measured today:
>
> ```text
> overprint / colorant   1_GWG011  1_GWG190  1_GWG191  1_GWG192
>                        2_GWG020  2_GWG030  2_GWG040
> transparency groups    1_GWG161  1_GWG162  3_GWG161  3_GWG164
> soft masks             1_GWG1611
> shading                1_GWG060
> ICC                    3_GWG130
> ```
>
> §1's baseline block and §1.1's `1_GWG160` probe are **left as written** —
> they are the measurement this plan was built on and rewriting them would
> destroy the record of what was true at `e618d67`. Read them as dated.

This document exists to answer one question with evidence rather than
intuition: **what single build clears the largest share of the remaining
Ghent failures, and why is it one build rather than five?**

The answer given here, **at the time of writing and now owed a re-derivation
(see the amendment above)**, is that **16 of the then-18** failures are
downstream of the same missing thing — pdfce has no compositor of its own. It delegates every
per-pixel blend to `tiny_skia`, which composites **8-bit premultiplied sRGB**
with **Porter-Duff over a transparent-initialised buffer**. ISO 32000-1
clause 11 requires compositing **in the group's colour space**, over a
**non-transparent initial backdrop**, with a **backdrop-removal correction**
on the way out, and — for overprint — **per-colorant planes that sRGB cannot
represent**. Every one of those four is a property of the buffer, not of the
call site. That is why the fixes do not decompose.

---

## §1 — The measurement this plan is built on

Baseline re-measured 2026-08-18 at `e618d67`:

```
25 pass · 18 FAIL · 8 UNRESOLVED (reference-strip) · 0 render errors  of 51
```

The five **transparency-group** failures were probed cell by cell with a new
diagnostic (`tools/ghent-cell-probe.py`, §7 below). For each trap X it reports
three numbers: the colour pdfce painted **inside** the X, the colour pdfce
painted in the **surround**, and the colour **Acrobat** painted at the same
place. That triple is decisive, because the suite's trap X is drawn so that a
*correct* engine renders it **the same colour as its surround** — so a
disagreement localises to one of the two, not to "the cell".

### 1.1 What the probe found — `1_GWG160` (DeviceCMYK, non-isolated, non-knockout)

Only **3 of 16** cells fail: the ones governed by **`Hue`**, **`Saturation`**
and **`Color`**, at device x = 204, 266 and 329 on the y = 106 row. Cell
identities resolved by `tools/ghent-cellmap.py` from each form XObject's
`/Matrix` and `/BBox` against its governing `/ExtGState`.

**★ CORRECTED 2026-08-18, later the same day.** This section originally read
that those three are *"exactly the nonseparable blend modes whose K component
is taken from the BACKDROP"*, that `Luminosity` passing was *"a one-bit
discriminator falling on the correct side"*, and therefore that §11.3.5.3's
K-selection rule was the cause. **That inference was wrong, and it was wrong
in the most persuasive direction — it fit.**

What the counters actually say, measured with `pdfce-cli render-page` on all
four transparency patches: **`blend_modes_applied=11, blend_modes_ignored=4`**
in every one. `blend_mode_from_name` returns `None` for all four nonseparable
modes, so pdfce **declines them outright** and composites them as `Normal`.
They never reach the applied path at all, so nothing about them can be
evidence for *how* the applied path computes. Caught by the librarian while
filing, from the code rather than from the pixels.

The corrected reading, and the cell identities are now **resolved** rather
than inferred — `tools/ghent-cellmap.py` walks the content stream, tracks the
CTM through `q`/`Q`/`cm`, and maps each form XObject's `/Matrix` and `/BBox`
into device space against its governing `/ExtGState`:

| patch | `Hue` | `Saturation` | `Color` | `Luminosity` | applied modes |
|---|---|---|---|---|---|
| `1_GWG160` | trap | trap | trap | **clean** | all 11 clean |
| `3_GWG164` | trap | trap | trap | **clean** | **`Difference` traps** |

**Three separate facts fall out, and the first two are different bugs:**

1. **`Hue`/`Saturation`/`Color` fail because they are not implemented** —
   declined, not miscomputed. `Luminosity` is declined *identically* and still
   comes out clean, which means "declined" does not imply "visibly wrong": on
   this artwork its correct result and its `Normal` stand-in coincide closely
   enough to stay under the suite's clear-X threshold. **Do not read that
   clean cell as evidence `Luminosity` works.**
2. **`3_GWG164`'s `Difference` cell is the real §11.3.4 evidence** — an
   *applied*, separable mode, failing on ICCBased CMYK. `Difference` is
   `|cb − cs|`, the mode most sensitive to whether its operands were
   complemented first, so it is where a wrong blending space surfaces
   soonest. One cell, not a cluster — reported as such.
3. §11.3.5.3's K-selection rule is still **required to implement** the
   nonseparable modes correctly. It is just not the explanation for today's
   failures. The clause was right; the attribution was not.

**The methodological lesson, since it cost a wrong entry in three documents
and a commit message:** the original mapping came from cell-pitch arithmetic
(`22.678 pt` squares on a `31.68 pt` pitch at scale 2.0) — one method, no
cross-check — and it produced a story so clean it discouraged looking further.
The pitch arithmetic turned out to be *right about positions* and the
*causal attribution built on it* was wrong, which is the combination that
survives a sanity check.

**The clause that Stage B still owes**, quoted so the implementation has it —
this is what *implementing* the nonseparable modes requires, not a diagnosis
of why they fail today. `iso32000__s__11.3.5.md` §4.8, verbatim:

> "The formulas in this sub-clause apply to **RGB** spaces. Blending in
> **CMYK** spaces (including both `DeviceCMYK` and `ICCBased` calibrated CMYK
> spaces) **shall** be handled in the following way: the C, M and Y components
> **shall** be converted to their complementary R, G and B components in the
> usual way; the preceding formulas **shall** be applied to the RGB colour
> values; the results **shall** be converted back to C, M and Y. For the **K**
> component, the result **shall** be the K component of **Cb** for the `Hue`,
> `Saturation` and `Color` blend modes; it **shall** be the K component of
> **Cs** for the `Luminosity` blend mode."

⇒ **K is not blended, it is selected**, and the selection differs by mode. The
same RAG file anticipated the gap in writing: *"pdfce currently composites in
device RGB. If/when a CMYK path lands, this clause is the whole rule."*

`3_GWG164` (ICCBased **CMYK**) fails **4** cells: the same three declined
nonseparable modes, **plus `Difference`** — and that fourth cell is the only
direct evidence in the corpus that an *applied* mode is computed in the wrong
space.

### 1.2 What the probe found — `1_GWG161` / `3_GWG161` / `1_GWG162` (14 / 15 / 7 cells)

Different shape entirely. In almost every failing cell, **pdfce's surround
agrees with Acrobat** (within a few levels) while **pdfce's X is a saturated
primary** — `[237, 1, 140]`, `[255, 0, 255]`, `[0, 0, 255]`. A saturated
primary is what a blend mode produces when it is applied against **nothing**:
`B(cb, cs)` composited over a transparent backdrop returns `cs` unchanged.

The cause is in `crates/pdfce-render/src/interpret.rs` and is already
documented honestly in its own comment:

```rust
let outer_is_neutral = self.gs.current.blend_mode == tiny_skia::BlendMode::SourceOver
    && self.gs.current.fill_alpha >= 1.0;
let needs_buffer =
    is_transparency_group && (!outer_is_neutral || group_flag(b"I") || is_knockout);
```

A `tiny_skia::Pixmap` starts **transparent**, and transparent-initialised is
**isolated** semantics (§11.4.7). So whenever the outer graphics state is not
neutral, pdfce allocates a buffer — and in doing so **silently converts a
NON-isolated group into an isolated one**. The comment says so: *"Buffering
unconditionally gets those wrong in the opposite direction from flattening."*

The Ghent transparency patches set `/BM` on the graphics state **at the `Do`**,
which makes `outer_is_neutral` false for every cell. So every cell takes the
buffered path, every cell loses the page backdrop, and every interior blend
degenerates to "paint the source". **14 of 16, 15 of 16, 7 of 16** — the
counts are what "essentially all of them" looks like once the handful of cells
whose correct answer *is* the source colour are removed.

This is not fixable by choosing the other branch. Painting inline gets
non-isolated groups right only while the outer state is neutral; a real
non-isolated group needs its buffer **initialised from the backdrop** and then
needs §11.4.4's **backdrop removal** applied to the result, or the backdrop
contributes twice.

### 1.3 Soft masks — same buffer, different clause

Diagnosed last session and unchanged: `/Alpha` and `/Luminosity` masks are
**constructed correctly** (both were dumped to PNG and inspected — correct,
properly placed soft gradients), but they are **applied by folding into the
clip**, which applies them to each element *inside* the group. §11.4.5 applies
the mask to the group's **RESULT**. Four patches (`GWG1610`, `GWG1611`,
`GWG168`, `GWG169`).

There is nowhere to apply a mask to a group result until the group result is a
thing pdfce owns. Same build.

### 1.4 Overprint — the colorant planes

Established three independent ways last session and written up in
`docs/overprint-architecture-survey.md`: by measurement (a spot-ink multiplier
plate was built, ablated, and **reverted** — `ac15158`), by research (seven
engines converge on one architecture; Artifex's colour architect states in a
peer-reviewed paper that collapsing colour before compositing *"is not
possible"* **specifically because of overprint**), and by pdfce's own spec RAG,
which called it *"architectural, a different project"* on 2026-08-08.

Seven patches: `GWG011`, `GWG190`, `GWG191`, `GWG192`, `GWG020`, `GWG030`,
`GWG040`.

### 1.5 The two that are NOT this build

| Patch | Cause | Where it belongs |
|---|---|---|
| `1_GWG060` | Type 6/7 mesh shadings | `Pass 85.1` — **unblocked**; `iso32000__s__8.7.4.5__mesh.md` (1,014 lines, Tables 82–86) landed 2026-08-18 |
| `3_GWG130` | ICC source profile handling | Its own Pass; see §6 on why `lcms2` is not the answer |

---

## §2 — Why one build: the four requirements are all properties of the buffer

| Requirement | Clause | What the buffer must carry |
|---|---|---|
| Blend in the group's colour space, subtractive components complemented before and after | §11.3.4 | **N colorant channels**, not 3 sRGB |
| Nonseparable modes in CMYK: RGB detour, K selected by mode | §11.3.5.3 | the **K channel**, addressable |
| Non-isolated groups: initialise from backdrop, then remove it | §11.4.4 | **un-premultiplied f32** + a second alpha `α_g` |
| Knockout groups: each element composites against the *initial* backdrop | §11.4.8 | **two** buffers per nesting level |
| Soft mask applies to the group RESULT | §11.4.5 | a group result that exists as a value |
| Overprint preserves untouched colorants | §11.7.4.4 / Table 149 | **one plane per colorant** |

`tiny_skia::Pixmap` is RGBA8 premultiplied with a single alpha. It satisfies
none of the right-hand column, and no call-site change makes it. The build is
**a compositor pdfce owns**, with `tiny_skia` demoted from "the thing that
blends" to "the thing that scan-converts".

---

## §3 — The architecture

### 3.1 Coverage and colour are separated

`tiny_skia` remains the rasterizer. It is asked for **coverage only**:

- **Fills** — `tiny_skia::Mask::from_path(path, fill_rule, anti_alias)` gives
  an 8-bit coverage mask.
- **Strokes** — `PathStroker` converts the stroke to a fill path first; then
  as above. (pdfce already does this in the clip path.)
- **Glyphs** — already outlines; same route as fills.
- **Images, shadings, tiling patterns** — cannot be reduced to a single colour
  plus coverage. These rasterize into a scratch `Pixmap` as today, and the
  compositor reads *both* colour and alpha from it. The colour arrives in sRGB
  and is lifted into colorant space by the same route as any other sRGB value,
  which is a documented lossy step, not a silent one (§5).

The compositor then does the per-pixel arithmetic itself. This is the
structure every surveyed engine uses and it is why "SIMD the blend loop"
(a suggestion pdfce received from an outside model) is premature: the loop
does not exist yet, and the failures are arithmetic, not throughput.

### 3.2 The pixel

```
struct Pixel<const N: usize> {
    c:      [f32; N],   // UN-premultiplied colorant values, group colour space
    alpha:  f32,        // α  — the full alpha, including the backdrop's
    alpha_g: f32,       // αg — the group's own alpha, excluding the backdrop
}
```

**f32, not u8**, and this is load-bearing rather than fastidious: §11.4.4's
backdrop-removal correction contains a single `1/α_gn`, which amplifies
quantisation error by that factor. At `α_gn = 0.02` a half-level u8 error
becomes 25 levels — visible, and exactly the magnitude Ghent traps on.

**Un-premultiplied**, because the blend function `B(cb, cs)` is defined on
un-premultiplied values and premultiplying-then-blending is a different
function for every non-linear mode.

`α_i = Union(α_0, α_gi)` is derivable and `α_0` is the parent buffer's alpha,
so **one extra scalar per pixel** is the whole cost over a plain RGBA buffer.

### 3.3 N is chosen per page, not per build

Pre-scan the page's resources for `/Separation` and `/DeviceN` colorant names
and size the plane set exactly: `CMYK + one plane per distinct spot`.
Ghostscript's own documentation notes this pre-scan **is possible in PDF and
impossible in general in PostScript** — it is a structural advantage pdfce
inherits from the format and should take.

**Cap and fall back honestly.** Beyond a configured plane ceiling, revert to
the tint transform — which is precisely pdfce's current behaviour, so the
fallback is already written, already tested and already disclosed by the
existing counters. Rule 4: the fallback **prints what it did**; it does not
quietly produce a different picture.

### 3.4 Scope: not page-wide by default

Poppler bug #1565 (still open) is the warning: enabling overprint preview
routed the whole page through CMYK and **visibly shifted unrelated RGB raster
content**. pdfce should engage the colorant compositor for the **object
subtrees that need it** — transparency groups, and content under an overprint
state — and leave the ordinary sRGB path alone otherwise. A patch that fixes
7 patches and shifts 25 others is not progress.

---

## §4 — Staging

The stages are ordered so each one is independently measurable and each one
ships a number. **Do not skip A to get to B**: A is where the group semantics
get right, and B is a change of pixel type on top of correct semantics. Doing
B first means debugging colorant arithmetic and backdrop removal at the same
time, on the same pixels.

### Stage A — the compositor, RGB only (proposed `Pass 97.0`)

Replace the group-buffer path with pdfce's own f32 un-premultiplied buffer and
pdfce's own composite/blend implementation. **N = 3, sRGB.** No colorant
planes yet.

Delivers:
- **Non-isolated groups**: buffer initialised from the backdrop; §11.4.4
  result-block backdrop removal `C = C_n + (C_n − C_0)·(α_0/α_gn − α_0)`, with
  the single division guarded.
- **Isolated groups**: `α_0 = 0`, which the same code path expresses without a
  branch on `/I`.
- **Knockout groups**: §11.4.8's `b ∈ {0, i−1}` subscript, implemented as
  **two buffers**, not per-element copies. Memory is O(nesting depth).
- **Soft mask on the group result** (§11.4.5), replacing the fold-into-clip
  approximation — including the `/TR` transfer function, which is currently
  read and counted (`soft_mask_tr_ignored`) but not evaluated. `/TR` is where
  a mask gets **inverted**, so an ignored one can leave visible exactly what
  the document meant to hide.
- **`0/0 = 0` by convention**, adopted unconditionally — a `should` in ISO
  32000-1 and a **`shall`** in ISO 32000-2 §11.3.2. Note the `shall` is on
  *robustness*: never emit NaN or Inf.

Expected: `1_GWG161`, `3_GWG161`, `1_GWG162`, `GWG1610`, `GWG1611`, `GWG168`,
`GWG169` — **7 patches**, 25 → up to 32.

### Stage B — colorant planes (proposed `Pass 97.1`)

Make the buffer N-colorant. Same compositor, different pixel.

Delivers:
- **§11.3.4** blending in the group colour space with subtractive complement
  (`blend_subtractive(cb, cs) = 1 − B(1 − cb, 1 − cs)`).
- **§11.3.5.3** nonseparable modes in CMYK: complement CMY to RGB, blend,
  complement back, **select K by mode**.
- **Table 149 overprint** — already written as pure, tested logic in
  `pdfce_render::overprint` (12 tests, the table transcribed cell by cell,
  `bd9d5ef`). It has never had a colorant buffer to run against.
- **Keep the tint transform OUT of the paint path** for any colorant that owns
  a plane; retain it only to derive that colorant's equivalent colour for the
  final collapse.

Expected: `1_GWG160`, `3_GWG164`, and the 7 overprint patches — **9 patches**,
32 → up to 41.

### Stage C — the collapse, and its disclosure (proposed `Pass 97.2`)

Collapse N planes to sRGB **once, at the end**.

**⇢ NOW SOURCED: `docs/collapse-model-survey.md` (2026-08-18).** The
"vendors disagree" claim is no longer an assertion — it is measured against
thirteen engines, and the absence of a specification is itself documented by
an ISO TC171 participant who went looking. Headlines:

- **Harlequin uses per-colorant `max()`; Mako uses multiply-of-complements —
  two products from the same vendor.** There is no consensus formula to find.
- **Acrobat's method has never been published**, and the ICC states Acrobat
  *"should not be used as a guide"* for spot inks. "Match Acrobat" is not an
  available specification.
- **Third independent confirmation of the N-plane architecture**, this time
  from vendor docs: *"overprinting… is disabled"* / *"not allowed"* if spot
  colorants are tint-transformed first. Tint-transforming early and
  overprinting correctly are mutually exclusive.
- **Default decided:** per-colorant `/Separation` tint transform (preferred
  over the DeviceN collective one), accumulate by multiply-of-complements,
  one ICC hop to sRGB through the OutputIntent. Five settings enumerated,
  including a second ambiguity nobody had noticed — **which OutputIntent to
  use when the array has more than one entry** (MuPDF takes `[0]` ignoring
  `/S`; Poppler refuses to act at all).
- **★ THE ICC HOP IS `iccce`'S — corrected 2026-08-18.** This bullet named
  `moxcms` as the ICC candidate. **`ARCHITECTURE.md` decision 064 already
  assigned colour CONVERSION to `iccce`** (the operator's sibling MIT project,
  which names pdfce as its first consumer), and recommending a third-party CMM
  against it was a call made without reading the record. `iccce` already
  ships what Stage C needs: `Chain::with_destination(&src, Destination::None,
  intent)`, whose built-in sRGB destination is **constructed from published
  constants** (BT.709-6, W3C transfer constants, Bradford to D50 per
  ICC.1:2022 Annex E.3) — **no shipped `.icc`, so no redistribution
  question**. Verified by reading `iccce-cmm/src/transform.rs`, not its prose.
- **Two things Stage C must carry from that**, both in
  `docs/collapse-model-survey.md` §7: `Destination::None` is an **assertion**,
  not `Option::None` — a declared-but-unparseable output intent is a
  **refusal to propagate**, never a silent fallback; and the conversion costs
  **~1.4 Mpix/s ≈ 6 s/page against pdfce's ~0.6 s render**, so the collapse
  cannot be an unconditional per-frame step.

### Out of scope for 97.x

`Pass 85.1` mesh shadings (`1_GWG060`) and ICC source profiles (`3_GWG130`).
Both are real, both are separately scoped, neither is blocked on this build.

---

## §5 — Risks, and the honest failure modes

1. **Performance.** Every group becomes an f32 page-sized buffer. Mitigation:
   engage the compositor only for subtrees that need it (§3.4); measure with
   `tools/render-profile` before and after; the render-parity gate
   (`tools/render-parity`, 2840/49/1 buckets) is the regression net for the
   ordinary path.
2. **The render-parity gate is against pdfium**, which does not implement
   overprint and flattens transparency differently. A Stage-B improvement can
   register as a parity *regression*. Read a parity delta on transparency
   content as a question, not a verdict — and record which oracle disagreed.
3. **sRGB → colorant is lossy and ambiguous.** Images and shadings arrive as
   sRGB. Lifting them into CMYK for a CMYK group is a guess. It must be
   **counted and printed**, not silently performed. This is precisely the
   class of inference rule 4 exists for: render it normally, disclose it
   off-canvas.
4. **Scope creep into a rasterizer rewrite.** The line is: `tiny_skia` keeps
   scan conversion, path stroking, glyph outlines and image sampling. pdfce
   takes compositing only. If a change requires re-deriving coverage, it is
   out of scope.
5. **The 8 UNRESOLVED reference-strip patches are not addressed by any of
   this** and must not be quietly counted as headroom. Two of the three cheap
   wins in §7 bear on them instead.

---

## §6 — Two external suggestions, assessed

An outside model (Gemini, via the operator, 2026-08-18) proposed a route to
the remaining patches. Recording the assessment because two of its
recommendations would be **actively wrong for pdfce**, and a future session
that meets the same advice should not have to re-derive why.

**Correct but already held, at lower resolution:** ISO 32000-2 clause 11 as
the governing text; W3C Compositing Level 1 as a cross-check on the separable
blend formulas; isolated-vs-non-isolated as the transparency-group axis;
Porter-Duff alpha weighting; and — independently reaching pdfce's own
conclusion — that overprint requires **preserving the underlying colorant
rather than knocking it out**. That last convergence is worth something: an
outside model with no access to this repo arrived at the same architectural
requirement as the seven-engine survey.

**Rejected — `lcms2`.** It is a binding to Little CMS, a **C** library. It
cannot cross the **wasm32 CI gate** that `pdfce-core` and `pdfce-render` are
held to (`.github/workflows`, `cargo check --target wasm32-unknown-unknown`),
and that gate is not negotiable machinery — it is the enforcement of the
web-fork invariant. The OCR engine decision turned on exactly this constraint.

**★ And the follow-on recommendation was ALSO wrong, which is the part worth
keeping.** This paragraph originally continued *"if ICC handling needs a
library, the candidates are pure Rust (`qcms`, `moxcms`)"*. **There is no
candidate slot to fill**: `ARCHITECTURE.md` decision 064 assigns colour
conversion to **`iccce`**, and that decision predates this document by a day.
Rejecting the wrong crate for the right reason and then proposing a
replacement is a *narrower* failure than adopting `lcms2` would have been, and
it has the same root — the record was not read. See
`docs/collapse-model-survey.md` §7.

**Rejected — `vello`.** GPU, via `wgpu`. `pdfce-render` may not gain a
windowing or GPU surface (`ARCHITECTURE.md` §3, project rule 2). This is
categorical, not a tradeoff. `resvg` is a fair *structural* reference — it is
`tiny_skia`'s own consumer — but it is a reference, not a dependency.

**Premature — SIMD.** The failures are arithmetic correctness. There is no
compositing loop to vectorise until Stage A exists, and vectorising the wrong
formula is not a win. Revisit after §5's measurement, not before.

---

## §7 — Cheap wins that do not wait on this build

Carried forward from `NEXT_SESSION.md` §2, unchanged and still unclaimed:

1. ~~**The trap detector is probably over-counting.**~~ **MEASURED AND FALSE,
   2026-08-18. Do not spend a session on this.** The hypothesis was that
   `CONTRAST_MIN` — calibrated against pdfce's own output rather than against
   GWG's stated *"Faint X does not indicate a failure!"* — was firing on marks
   GWG pre-declares tolerant (**all ten cells of GWG020**, **cell d of every
   DeviceN patch**). The new probe measured the actual X-versus-surround
   contrast on every currently-failing patch those tolerances cover:

   | patch | cell | X | surround | faint? |
   |---|---|---|---|---|
   | `GWG020` | 6 of 7 | `[254,254,253]` white | `[141,197,62]` green | **no — maximal** |
   | `GWG020` | 2 cells | `[196,197,195]` grey | `[146,197,73]` green | no |
   | `GWG190` | d | `[0,0,0]` black | `[0,158,218]` cyan | **no** |
   | `GWG191` | a,b,d | `[0,0,0]` black | green / cyan | no |
   | `GWG191` | c | `[0,240,255]` | `[0,180,241]` | no (~60 levels) |
   | `GWG192` | b | `[255,255,255]` white | `[239,56,62]` red | **no — maximal** |

   GWG's wording for `GWG020` is *"a faint 'X' in **slightly darker green**"*.
   A **white** X on green is not that mark. Every trap still firing on these
   patches is at or near maximal contrast, so **no recalibration consistent
   with GWG's criterion changes a single verdict**. The failures are real
   rendering failures, and they are the ones §4 Stage B addresses.

   One live nuance that survives, and is *not* about contrast: `GWG191` cell
   **c** has **two sanctioned correct outcomes** — GWG states a cross there is
   fine *"if the system performs colour conversion and sets the OPM for this
   patch c to 0"*. pdfce converts but leaves `OPM 1`, so its cross is a
   genuine failure today. If Stage B ever makes pdfce take the
   convert-and-set-OPM-0 route deliberately, the harness must learn that
   cell c is not binary.
2. **The suite ships its own Reference file** —
   `Ghent_PDF-Output-Test-V50_ALL_REFERENCE.pdf`, in the same ZIP, with texts
   in Registration so they appear in every separation. pdfce is not using it
   as an oracle and should. This is the one that bears on the 8 UNRESOLVED.

   **Blocked on an input, checked 2026-08-18:** the file is **not on this
   machine**. `D:\Dev\temp\ghent-patches\` holds the 51 patch PDFs and
   `D:\Dev\temp\ghent-readmes\` the extracted ReadMes, but the Reference PDF
   was not among what was kept from the 126 MB download. Re-fetching it is an
   operator call (a large download, and `LEGAL.md` §5 governs what enters the
   corpus), so this item is **owed, not merely unstarted** — it should not be
   picked up as if it were a free afternoon.
3. **`/Indexed` colorants — MEASURED AND CONFIRMED, 2026-08-18. This is a live
   defect, not a suspicion.** Colorants must be read from the **base** space
   (§8.6.6.3). `overprint::classify` has no `Indexed` arm, so an `/Indexed`
   space falls to `_ => SourceKind::OtherProcess` and its base's colorant list
   is invisible to Table 149. Extracted from the corpus:

   ```
   1_GWG190:  /Indexed [/DeviceN [/Cyan]              /DeviceCMYK ...] 255 <lookup>
   1_GWG190:  /Indexed [/DeviceN [/Cyan /Yellow /Black] /DeviceCMYK ...] 255 <lookup>
   2_GWG020:  /Indexed /DeviceCMYK 255 <lookup>
   ```

   The first two **are** GWG190's documented discriminator — the a/b pair's
   DeviceN **omits** the backdrop's colorants and the c/d pair **includes**
   them at 0%, and *"the colorant LIST — not the tint values — decides what
   survives"*. pdfce cannot see either list. `/Indexed` appears in **4 of the
   7 failing overprint patches** (`GWG190`, `GWG191`, `GWG192`, `GWG020`).

   Two halves to the fix, and only the first is small: `classify` must recurse
   into the base space, **and** the tints handed to `cmyk_group_rules` must be
   the palette-**looked-up** base components rather than the index. The call
   site currently receives `(space, comps)` where `comps` is the raw index.

   **A second, larger gap surfaced while measuring this**, and it is recorded
   rather than fixed because it belongs to Stage B: `overprint::composite` has
   exactly **one** call site, in the path/glyph painter. **Image XObjects do
   not reach it at all** — and `GWG190`'s only failing cell is `d`, an image.
   Per-sample overprint needs per-sample colorants, which is the colorant
   buffer. Before building it, add the counter: an image that skips overprint
   is currently not counted as `overprint_refused`, which is the same
   blind-counter shape as the glyph painter in `bf75351`.

And one new one, produced while writing this document:

4. **`tools/ghent-cell-probe.py`** — the diagnostic §1 is built on. For each
   trap it prints the X colour, the surround colour and Acrobat's colour at
   the same cell. It turned "14 traps on `1_GWG161`" into "the interior blend
   is being applied against a transparent backdrop" in one run. It currently
   lives outside the repo and should be promoted into `tools/` with the cell
   index → blend-mode mapping derived from the content stream rather than from
   the pitch arithmetic in §1.1.
