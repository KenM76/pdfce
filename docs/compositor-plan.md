# The colorant compositor — plan of record

**Written 2026-08-18**, engineer-owned, at `2a75be1`+`e618d67`.
Companion to `docs/overprint-architecture-survey.md` (the sourcing record for
the colorant half) and `docs/ghent-patch-reference.md` (the per-patch expected
appearance for the overprint patches).

This document exists to answer one question with evidence rather than
intuition: **what single build clears the largest share of the 18 remaining
Ghent failures, and why is it one build rather than five?**

The answer is that **16 of the 18** failures are downstream of the same
missing thing — pdfce has no compositor of its own. It delegates every
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

Only **3 of 16** cells fail, and they are contiguous:

| trap x | cell index | blend mode |
|---|---|---|
| 204 | 3 | **Hue** |
| 266 | 4 | **Saturation** |
| 329 | 5 | **Color** |

Cell index derived from the patch's own content stream: the row-2 labels are
`Hard Light, Difference, Exclusion, Hue, Saturation, Color, Luminosity,
Opacity (0%)`, the cells are `22.678 pt` squares on a `31.68 pt` pitch, and
the render is at scale 2.0.

Those three are **exactly the nonseparable blend modes whose K component is
taken from the BACKDROP**. `Luminosity` — the fourth nonseparable mode, whose
K comes from the **source** — **passes**. That is not a coincidence and it is
not a coarse signal; it is a one-bit discriminator falling on the correct
side.

`iso32000__s__11.3.5.md` §4.8 quotes the governing `shall` verbatim:

> "The formulas in this sub-clause apply to **RGB** spaces. Blending in
> **CMYK** spaces (including both `DeviceCMYK` and `ICCBased` calibrated CMYK
> spaces) **shall** be handled in the following way: the C, M and Y components
> **shall** be converted to their complementary R, G and B components in the
> usual way; the preceding formulas **shall** be applied to the RGB colour
> values; the results **shall** be converted back to C, M and Y. For the **K**
> component, the result **shall** be the K component of **Cb** for the `Hue`,
> `Saturation` and `Color` blend modes; it **shall** be the K component of
> **Cs** for the `Luminosity` blend mode."

⇒ **K is not blended, it is selected**, and the selection differs by mode.
pdfce composites in device RGB, so it has no K to select and no CMY to
complement. The same RAG file already anticipated this in writing: *"pdfce
currently composites in device RGB. If/when a CMYK path lands, this clause is
the whole rule."*

`3_GWG164` (ICCBased **CMYK**) fails **4** cells and is the same defect — the
clause names calibrated CMYK explicitly.

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

Collapse N planes to sRGB **once, at the end**. §6 of the overprint survey
records that this step is **not standardised**, that vendors disagree
materially, and that Acrobat does not document its method. That makes it a
**settings-shaped ambiguity** under the standing rule (never hard-code a
choice the standard leaves open): pick a default deliberately, expose it, and
disclose which model produced the pixels.

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
If ICC handling needs a library, the candidates are **pure Rust** (`qcms`,
`moxcms`) and each still needs a licence classification under rule 13 before
it enters a `Cargo.toml`.

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
