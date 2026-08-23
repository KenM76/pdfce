# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. Overwrite it once acted on.

**REWRITTEN 2026-08-23 by `pdfce-librarian` (two-hundred-and-thirty-second
filing).** **`PASS 74.7` and `PASS 74.9` both shipped** (`1d6db9e`, `5b0d885`,
`296a23e`). **Open operator question `(br)` is CLOSED** — Ken answered it, and
**both of his rulings are discharged**. **`R215` and `decision 083` minted.**
**Nothing is now awaiting Ken except pushing and cutting a backup**, and both of
those are his acts, not an agent's.

---

## §0 — BOTH OF KEN'S RULINGS ARE DISCHARGED. THERE IS NO OPEN QUESTION.

**This section used to say *"the one thing waiting on Ken."* There is no longer
one.** He answered on 2026-08-23, verbatim and in full:

> *"we'll make the image quality trade an optional option. also if you can fix
> the drawing precision without affecting speed where that percsion isn't
> needed then we should do it. this might require 2 algorithms though depending
> on what is needed."*

**That sentence contained three things and all three shipped the same day.**

1. **`(br)` answered — the third of the three costed options.** A switch, off by
   default, disclosed as a counter. Shipped as **`PASS 74.9`** (`296a23e`):
   **`--fast-subpixel`** on the CLI, **`RenderOptions::subpixel_culling`** in
   the library, **OFF by default**.
2. **`PASS 74.7` confirmed as the engineer's to take** — *"if you can fix the
   drawing precision … then we should do it."* The 230th filing had already
   ruled that; this is the operator agreeing with a ruling never put to him.
3. **The constraint that shaped the whole Pass** — *"without affecting speed
   where that precision isn't needed."* Both algorithms are **gated on
   magnitude and decided once**, not per pixel, and the measurements below say
   ordinary rendering paid nothing.

**★★ AND HIS "THIS MIGHT REQUIRE 2 ALGORITHMS" WAS A CORRECT TECHNICAL
DIAGNOSIS, not a hedge.** Read it that way. The fix genuinely decomposed into
two mechanisms with different triggers, different costs and different failure
modes.

### What `PASS 74.7` did — the content ceiling, in two algorithms

**`Pass 74.2` raised the ceiling for the VIEWPORT; the CONTENT still sat under
three `f32` limits.** All three are now raised.

- **`Mat64`** — the CTM carried in `f64` through composition, narrowed only at
  the leaf. Discharges limits **2 and 3** (the placement matrix; the quantised
  device position). **Three sites had to change together, each load-bearing
  alone**: `cm`, a form's `/Matrix`, and **the BASE transform**, which
  `RegionGeometry` was handing over **already narrowed**. ⇒ **Widening an `f32`
  back to `f64` carries its rounding forward** — keep the `f64` coefficients.
- **Path differencing** — when `needs_precise_paths` fires, the interpreter
  builds the path **relative to its own first point**, differencing in `f64`.
  Discharges limit **1**, which the Backlog entry had explicitly recorded as
  out of `74.7`'s scope. An `f32` near `x = 90 pt` has a spacing of
  `7.6 × 10⁻⁶ pt`, so an `8 × 10⁻⁶ pt` rectangle is **one representable step**
  and no CTM precision recovers it.

**Measured: a single water molecule renders sharply at `scale` 1.6 × 10⁹ —
≈ 190 billion percent** — where before everything above ≈ 5 × 10⁶ vanished.
**Ordinary rendering pays nothing**: banana page **1430/1458 ms against
1487/1532 ms before**; CAD at scale 1.0 **1086 ms against a 980–1040 ms
baseline**.

**★★★ AND `23.8×` FELL OUT OF IT.** The same stroke-heavy CAD region went
**`31 s → 1.3 s`**. Deep zoom was never slow **because** it was imprecise; it
was slow **for the same reason** — large magnitudes reaching a rasteriser that
reasons in **relative** tolerances. See §6 lesson 1.

### What `PASS 74.9` did — the opt-in lossy cull

A Form XObject whose transformed `/BBox` is under **half a device pixel in BOTH
axes** is not executed — both axes, so a **hairline** still paints. Counted on
**its own** key, **`subpixel_culled`**, which **prints whether or not the flag
is set**.

| measurement | speed | fidelity |
|---|---|---|
| whole banana page, page-fit | **`1 468 ms → 108 ms` = `13.6×`** | **0 of 1 242 640 px differ (0.0 %)** |
| 1 pt window, scale 20 | small | 18 of 400 px (4.5 %), worst channel delta 16 of 255 |
| 1 pt window, scale 35 | small | 47 of 1 296 px (3.6 %), worst delta 54 of 255 |
| 1 pt window, scale 60 | small | 82 of 3 600 px (2.3 %), **worst delta 62 of 255 — a quarter of a channel** |

⇒ **genuinely lossy, and the loss is largest exactly where the speed-up is
smallest.** That anti-correlation is why it is **a switch and not a heuristic**:
a heuristic must pick a threshold and every threshold sits somewhere on that
curve.

### ★★ THE OFF-CENTRE RENDER IS NO LONGER EVIDENCE. Its framing is SPENT.

**This file previously said "the render is off-centre ON PURPOSE — do not fix
it", because a `--region` nudge moved the viewport and left the content where it
was.** **That is no longer true.** `PASS 74.7` fixed exactly this, and a nudge
now moves the content.

⇒ **`tools/gen-scale-demo/README.md` and the saved render in
`C:\Users\Ken\OneDrive\pdfTests\` still carry the old framing**, and it needs
one sentence saying it is historical. **This is an owed item, not a defect** —
see §5.

### What to look at

**`C:\Users\Ken\OneDrive\pdfTests\`** — `banana-at-scale.pdf` and eleven
numbered renders, `_1-whole-page` through `_11-ten-molecules`. Tiers 10 and 11
(`_10-molecule-box-pointer`, `_11-ten-molecules`) are the ones the deep-zoom
work was built for. **Re-render them now that `74.7` has landed** — the deepest
tiers were showing content that had drifted hundreds of pixels or vanished
entirely, and those renders predate the fix.

### The banana page's remaining imperfections — unchanged, still worth doing

Pulp-cell **label collisions** (`nucleus` / `nucleolus + chromatin` leaders
converge; `central vacuole` sits near a starch grain); the banana's **stem and
blossom scar** read as placeholders; the **dart's tail** is a blunt cut;
`easter_egg.FONT` is **all-caps only** (lower case needs curves, and curves from
1.5 µm beads stop reading as letters — the honest route is *larger letters, not
smaller beads*); the **heart glyph sits low** on its line; **standard-14
Helvetica, not embedded** (what stops this being PDF/A); **no `/Info`
dictionary**; the page's **lower third is empty**.

Two constraints that are load-bearing rather than decorative: **labels are 1/10
the largest cell's height** (30 µm = 0.085 pt, so cell size drives label size),
and **the arrow is a tapered dart rather than a shaft-and-head because a
conventional arrowhead is ~8 pt across — nine times wider than the thing it
points at.**

### How to look at any tier

`--region` takes **two corners in PDF user space**, not an origin and a size.
The viewport is what costs time, so keep the region small and put the
magnification in `--scale`. `tools/gen-scale-demo/README.md` carries the tier
table with timings and regions.

```
python tools/gen-scale-demo/gen_banana.py <out.pdf>
pdfce-cli render-page banana.pdf --page 1 --scale 1.6 -o page.png
pdfce-cli render-page banana.pdf --page 1 --scale 60 --fast-subpixel -o fast.png
```

---

## §0.5 — ★★★★★ READ THIS BEFORE WRITING ANY "REQUIRED AFTER" COLUMN — `R215`

**The acceptance oracle this project filed in advance for `PASS 74.7` demanded
the WRONG ANSWER, and a fix that satisfied it would have been a bug.**

The 230th filing opened `Pass 74.7` in *Backlog* with a column headed *"required
after `74.7`"* reading **`11 of 11` forms at every scale**. **The correct
post-fix answer is `2`** from `1.25 × 10⁷` up — the box plus the one molecule
actually in frame.

**Two mechanisms were reducing that count** and the oracle named only one:

- the **precision defect** being hunted; and
- **`Pass 74.4`'s `/BBox` viewport cull** — **exact, correct, ISO 32000-1
  §8.10.1, and shipped by this same project five commits earlier** — dropping
  forms that genuinely left the viewport as the zoom rose.

The engineer measured `11 → 7 → 3 → 1` and read it as **one curve from one
defect**. It was **two curves superimposed**, and the healthy row (`11 of 11`)
got extrapolated to every other row.

**★★ What made it authoritative is the part to carry forward.** The table was
introduced as *"ACCEPTANCE BASELINE — ALREADY MEASURED, WHICH IS RARE ENOUGH TO
NAME"*. **That praise is what made it hard to question.** Every number in it was
a correct observation; **the claim was in the column heading.**

**What `R215` obliges:** every row of an acceptance table states its expected
value **and the mechanism that produces it**; before writing a *"required
after"* column, **enumerate every mechanism that moves the measured quantity —
especially your own recent, correct ones**; and where they cannot be
decomposed, **give the oracle a weaker form** (a direction, a bound) rather than
a precise number derived by extrapolation. **A weak oracle that is right beats a
precise one that is wrong**, because only the second can fail a correct fix.

**No harm resulted here** — the engineer reasoned about what `2` meant instead
of matching the table. **That is care, not machinery**, which is why the rule
exists.

---

## §1 — THE THREE PRE-FLIGHT CHECKS, unchanged and still earning their place

**1. `ls` BOTH FeatureRequests channels.** They are outside this repository, so
**no gate will ever contradict a stale sentence about them — including this
one.**

```
D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\
D:\Dev\FeatureRequests\iccce_FeatureRequests\open\
```

Three sessions running, that `ls` found something a document said was not there.

**2. Run the gates — `ls tools/check-*`, do not trust any list.**
`python tools/check-ci-parity.py --list` prints the local stand-ins.
**`R209`:** *"all gates green" names a set, and the set somebody runs is not the
set CI runs; a CI job with no local runner is UNOBSERVED, not passing.*
*(★★ **Measured 2026-08-23, 232nd filing, by `ls tools/check-*` and by running
each: 17 scripts on disk (12 `.py` + 5 `.sh`), 16 runnable as bare gates, ALL 16
EXIT 0.** The 17th, `check-image-colorspace-truth.py`, exits 1 on a bare
invocation **because it takes a fixture-directory argument** and is not a gate.
**Count them; do not quote a count.**)*

**3. Read `docs/compositor-plan.md`** before scoping anything in `97.x`.

---

## §2 — ★★ THE GHENT PASS COUNT IS AN OVER-COUNT. Read this before quoting any figure.

`tools/ghent-check.py` implements **one of the suite's two pass criteria**. It
hunts for a **cross that should not be there**; seven patches instead mark
failure by the **absence of a check mark** (GWG 050, 080, 081, 082, 150, 151,
152) and have scored `clean` since the harness was written. At least three of
them are failures. A second fault — the contrast floor has no **area** term —
puts `GWG 1.0` in the same category.

**Corrected standing: 26 at most, not 29.**

⇒ **THE DELTAS SURVIVE, THE LEVELS DO NOT.** Every board ever filed is
over-counted by the same family, so a before/after comparison is sound and any
absolute "N of 51" is not.

The operator's cell-by-cell judgements are in
`docs/ghent-operator-review-2026-08-21.md` — **the only independent check this
harness has ever had**, and the calibration set for fixing it (`Pass 122.2`).

---

## §3 — WHAT SHIPPED 2026-08-22/23

| commit | Pass | what |
|---|---|---|
| `e36f96e` | `74.4` + `74.5` | Form XObject viewport culling (exact); mitochondria drawn to real anatomy |
| `ea413a4` + `950e3af` + `65e1910` | — / `74.6` / `74.6b` | metrics contract whole and gated; ten molecules at 1:1 |
| `eca07ee` | `74.8` | `--no-annotations` says how many it withheld; nine stale doc claims discharged |
| **`1d6db9e` + `5b0d885`** | **`74.7`** | **the content-side `f32` ceiling raised, in two algorithms; `23.8×` on deep-zoom CAD as a side effect** |
| **`296a23e`** | **`74.9`** | **`--fast-subpixel`, the opt-in lossy cull; closes `(br)`** |

**Ledger after this filing:** next free Pass in the `74` family **`74.10`**
(precedent `Pass 47.10`/`47.11`); decisions **083**, next free **084**; standing
rules **`R215`**, next free **`R216`**; operator questions ceiling **`(br)`,
CLOSED**, next free **`(bs)`**; **`render-page` metrics line at `90` keys**,
measured by running `tools/check-metrics-line-contract.py`.

---

## §4 — THE QUEUE, in the order I would take it

**Item 0 is gone — it was answer-dependent on `(br)`, and `(br)` is answered and
shipped.** Nothing in this queue is blocked on anybody.

1. **★ The DISPLAY-LIST replay path's precision — check this FIRST, because it
   is the only item that could be a live defect rather than new work.**
   `PASS 74.7` changed the **interpreter**; a recorded list replays through
   `Canvas`. **If recorded ops carry `f32` coordinates, the list is a second
   rendering path at a different precision** — `R211`'s exact subject (*two
   paths that owe each other byte-identical output*) and decision **081**'s.
   Either it is already handled and owes one line in the display-list doc block,
   or it is a real gap. **`FEATURES.md` row 202 is deliberately left unchanged
   until someone can tell**, so nothing in the docs will contradict you either
   way. Cheap to answer: replay a deep-zoom region from a list and diff it
   against the fresh render.
2. **Build `R214`'s positional-reference gate** — a grep over a closed
   vocabulary (*of those*, *the above*, *this slice*, *the next slice*, *the
   former*, *the latter*, *as above*, *see below*, *the previous field*) in doc
   comments. **Measure its baseline first, repair, then wire** — never wire it
   red (`ci_gate_red_at_baseline_enforces_nothing.md`). Small, and it is the
   only defence against the one class of stale claim that contains no stale
   word.
3. **`Pass 97.1g`** — non-isolated ordinary groups on a subtractive page are
   composited as if isolated. The arithmetic exists (`remove_backdrop_cmyk`);
   the second content walk does not. **A port of the additive path, not a
   design.**
4. **`Pass 97.1k`** — native colorant paths for images and shadings, which
   bridge through sRGB today (`cmyk_bridged_pixels`).
5. **`Pass 122.2`** — teach `ghent-check.py` the check-mark criterion and give
   its contrast floor an area term. §2 says why this is not optional.
6. **`Pass 122.1`** — per-sample image overprint. Now **diagnosed**: it is why
   `GWG 8.2`'s check mark is missing. The mark is painted in yellow *underneath*
   the images, which overprint it so cyan-over-yellow reads green; pdfce paints
   the image normally and covers it (`overprint_images_unsupported = 2`).
   pdfium fails it too.
7. **`Pass 122.0`** — multithreading, the operator's request. His design (a
   runtime max-cores setting) is right and is kept; decision 080 adds a
   **compile-time target gate**, because `std::thread` and `rayon` both
   `cargo check` cleanly for `wasm32` and the CI wasm job therefore cannot catch
   a threading regression. Hard acceptance criterion: **byte-identical output at
   any core count.** ★ **The banana page is a genuinely useful threading
   benchmark**, because its cost is ~1 000 000 path operators in 342 forms
   rather than one big image. ★★ **And read `R215` before writing its
   acceptance table** — "byte-identical at any core count" is a *bound*, which
   is the right shape; a per-tier timing target would not be.
8. **`Pass 119.1`** — `unshare_form`. Carried unstarted through seven handoffs
   now.
9. **`Pass 122.3`** — the colorant buffer's byte ceiling. Interactive use is
   unaffected, but a **full-page** render above ~375 DPI refuses the buffer and
   silently composites in the wrong space, so one page can have different
   colours at different resolutions.

---

## §5 — STILL NOT DONE, named so it does not read as done

- **`tools/gen-scale-demo/README.md` and the off-centre saved render still carry
  SPENT EVIDENCE.** They describe the off-centre framing as deliberate proof
  that a `--region` nudge does nothing. **The nudge works now.** One sentence
  saying the framing is historical, or the next session preserves it for a
  reason that no longer exists. **Owed, in a code commit, not a filing.**
- **`5b0d885`'s `93 s` needs one sentence of provenance.** The figure appears
  twice in that commit message with two possible referents — the rejected
  device-space attempt's time (*"three times slower … 93 s against 31 s"*) and a
  before-figure at `100 000×` (*"at 100 000×, 93 s → 1.3 s"*). **The 232nd
  filing deliberately did not decide which**, and filed both ratios (`23.8×`,
  `71.5×`) beside their pairs. The `23.8×` headline is unaffected either way.
- **Whether `RenderOptions` is `#[non_exhaustive]`** — and therefore whether
  `subpixel_culling` is source-breaking for a downstream struct-literal
  construction. Per §4.1 (R)'s rule an optional-with-default field owes no bump;
  per (T)'s refinement that holds only where the struct cannot be exhaustively
  constructed downstream. **Not verified by the librarian** (docs-only filing).
- **`R215`'s retro-application** — any Pass filed with a *"required after"*
  column should be re-read against the rule before that column is used as a
  gate.
- `resolve_indexed` builds its palette with a **scratch `ColorDiagnostics` that
  is discarded**, so a tint failure inside a palette never reaches the operator.
- **Implicit knockout**: only explicit `/K true` is honoured. `/TK` defaults true
  (every text object), and `B`/`b` and shading patterns are knockout. **The one
  pdfce implements is the rarest.**
- **`/TR` on a soft mask** is read, counted, never evaluated.
- **`/AIS true`** is not distinguished from `/AIS false`.
- **Spot colorants** — four planes, not runtime `N`. Every remaining
  trap-criterion Ghent FAIL is in this bucket.
- **Per-paint rendering intent** (§11.7.5.3) — pdfce carries one per page.
  `iccce` costed the alternative and asked for the consumer fact; **no corpus
  measurement of mid-page intent switching has been taken.**
- **No GUI code path reads** `forms_culled`, `subpixel_culled`,
  `annots_out_of_scope`, `page_content_suppressed`, `render_page_region` or the
  display list, **and no GUI exposes `--fast-subpixel`.** GUI work is paused;
  recorded so the `[ ] gui` boxes in `FEATURES.md` are not mistaken for
  oversights. ★ **The new `--fast-subpixel` row's `gui` box is `[ ]` and was
  deliberately not rounded up** — unlike row 161, where the row's *subject* is
  reachable and only its counters are not, **here the switch IS the subject**.

---

## §6 — LESSONS FROM THIS RUN THAT ARE CLASSES, NOT INCIDENTS

1. **★★★★ A PRECISION DEFECT AND A PERFORMANCE DEFECT WITH ONE CAUSE LOOK
   UNRELATED UNTIL ONE OF THEM IS FIXED — and the performance one is the half
   nobody attributes correctly.** *"Deep zoom is slow"* sounds like an inherent
   cost, and nobody opens a ticket against physics. Here one cause — large
   magnitudes reaching a rasteriser that reasons in **relative** tolerances —
   produced both, and fixing where the numbers are large fixed both: `31 s →
   1.3 s` = `23.8×`, as a **side effect** of a precision fix.
   `D:/dev/rag/rust/a_precision_defect_and_a_performance_defect_with_one_cause_look_unrelated_until_one_is_fixed.md`
2. **★★★ `tiny_skia` FLATTENS CURVES TO A TOLERANCE IN THE PATH'S OWN UNITS.**
   The rejected first attempt built the path in **device space**: correct, and
   `3×` **slower** at extreme zoom, because million-magnitude coordinates get
   subdivided accordingly. **Move the SUBTRACTION into the wide type; do not
   move the COORDINATES into a big space.**
   `D:/dev/rag/rust/tiny_skia_flattens_curves_to_a_tolerance_in_the_paths_own_units.md`
3. **★★★★ AN ACCEPTANCE ORACLE BUILT FROM THE BROKEN SYSTEM'S OWN OUTPUT
   ENCODES THE DEFECT AS THE REQUIREMENT.** §0.5 has the instance. `R215`.
   `D:/dev/rag/rust/an_acceptance_oracle_built_from_the_broken_systems_output_encodes_the_defect_as_the_requirement.md`
4. **★★ A FLOAT-PRECISION TEST WITH ROUND-NUMBER OPERANDS CANNOT FAIL.** Two
   **equal** large `f32` values cancel **perfectly**; a cancellation only loses
   precision when the operands are large and merely **NEARLY** equal. The
   `Mat64` unit test's first fixture used round numbers and `f32` got the right
   answer.
   `D:/dev/rag/rust/a_float_precision_test_with_round_number_operands_cannot_fail_because_equal_values_cancel_exactly.md`
5. **★★ EVERY TEST IN THIS BATCH HAD TO BE ARGUED INTO BITING, AND ALL FOUR
   WERE CAUGHT BY DELIBERATE SABOTAGE** (restoring from file copies, not
   `git checkout`): a round-number fixture; a sampling test that passed with
   **half** the fix removed (10 px of error does not flip a sample 160 px from
   an edge); a sub-ulp test that sampled the **middle row** while the rectangle
   sat in a **corner**; and a counter assertion where a **raster diff** was
   needed. **A standing-rule mint was DECLINED for this** — `R87`, `R162` and
   `R164` already name the *conditions*, and *"watch it fail"* names an
   **action**, which reads as *sabotage everything*.
   `D:/dev/rag/rust/prove_test_suite_non_vacuous_by_deliberately_breaking_the_thing_it_tests.md`
6. **★★ AN EXACT SKIP AND A LOSSY SKIP MUST NOT SHARE A COUNTER** — decision
   **083**. `forms_culled` changes no pixel; `subpixel_culled` does. One number
   summing both is unanswerable for the only question these counters exist to
   answer: *did this render change my picture?*
7. **★★ A ceiling is a claim about ONE quantity**, and the quantity is the half
   that gets lost. `R213`.
   `D:/dev/rag/rust/a_ceiling_is_a_claim_about_one_quantity.md`
8. **★★ A reference by POSITION cannot tell you whether it is BROKEN or WRONG.**
   `R214`. Name the referent; repair at the referent.
9. **★ A gate's FIRST LIVE CATCH is separate evidence from its founding case —
   and `check-metrics-line-contract.py` has now had its SECOND and THIRD**
   (`subpixel_culled`, after `eca07ee`'s two annotation keys), on a gate four
   commits old, both on work it was not built for. **`89 → 90` keys.**
   `D:/dev/rag/rust/a_gates_first_live_catch_is_the_first_evidence_it_generalises_beyond_its_founding_debt.md`
10. **★ A cull is worth only what it skips.** Its hit counter is **invariant to
    its placement**, so the instrument you add to prove the optimisation works
    cannot detect that it barely works.
    `D:/dev/rag/rust/a_cull_placed_after_the_decode_reports_the_same_count_and_buys_almost_nothing.md`
11. **★ A limitation justified by "beyond any plausible use" is a PREDICTION,
    not a fact** — `R193`. `docs/render-region-measurements.md` (2026-08-13)
    concluded *"numerical precision is not the binding constraint on
    magnification … three orders of magnitude beyond any plausible viewing
    zoom"*, and **the banana page needed `scale` 1.6 × 10⁹ eleven days later.**
    Every number in that document is correct; its **conclusion** was refuted. A
    dated amendment is appended. **An error curve measured over the range you
    consider plausible cannot tell you the range is plausible.**

---

## §7 — HOUSEKEEPING

**All four figures below were MEASURED on 2026-08-23 by the commands named
beside them (hard rule 8). Re-run them; do not quote these lines.**

- **`origin/main` is at `c24ad7a`, and `main` is `15` commits ahead** —
  `git remote -v` → `origin  https://github.com/KenM76/pdfce.git`;
  `git rev-list --count origin/main..main` → **`15`**. **No `git fetch` was
  run**, so that is the local remote-tracking ref's position, **not a live query
  of the remote.** Pushing is the operator's act and needs a current go-ahead
  (`CLAUDE.md` rule 8). **The repository is public, so anything committed is
  published by default.**
- **Backups are `208` commits and six days behind `HEAD`.** Newest bundle
  `pdfce-20260817-v060.bundle` (2026-08-17 20:34) with `refs/heads/main` at
  **`3c4c00e`**; `git rev-list --count 3c4c00e..main` → **`208`**;
  `git merge-base --is-ancestor` confirms **`HEAD` is NOT in that bundle**.
  `v0.7.0`'s tag is in no bundle on disk. **Cutting one is the operator's
  call** — and it is now the *only* outstanding item that is his rather than an
  agent's, alongside pushing.
- **Worktrees: `9` entries**, by `git worktree list`. `D:/Dev/pdfce` (main),
  `%TEMP%\pdfce-head` (detached at `0eff831`), and **seven agent worktrees**
  under `.claude/worktrees/`. **`git worktree list` is authoritative — do not
  quote this line, re-run it.**
- **If `check-commits-filed.py` is red when you start, READ ITS OUTPUT FOR THE
  HASH.** Do not assume which commit it means, and **never extend
  `tools/commits-filed-baseline.txt`** — that file is pre-existing debt, not an
  allowlist.
- **A commit may bundle doc-comment repairs; a FILING may not bundle code.** The
  rule `d4721d8` established has **two directions**, and the second is easy to
  miss: a librarian filing that carries a code change cannot file itself and
  manufactures one more unfiled commit. The reverse — doc repairs inside a code
  commit — costs nothing mechanically and is fine.
- ★ **A claim in `bd9844d`'s commit message is WRONG and stands corrected only
  in the filing.** It says `tools/check-string-gaps.sh` *"has caught it every
  time"*. It has not: `ae06440` (2026-08-20) is titled *"the string-gap gate
  reported two of three"*, and that miss was found by a human who knew there
  were three. The ordinal ("third time") survives; the "every time" does not.
