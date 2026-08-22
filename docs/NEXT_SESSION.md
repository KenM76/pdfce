# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. Overwrite it once acted on.

**Written 2026-08-22 (evening), by `pdfce-librarian` at the engineer's
request** during the 228th filing, replacing the 2026-08-22 (afternoon)
handoff. **Amended 2026-08-22 (229th filing):** §0.5's two doc repairs are
**discharged** by `f9dc007`; what stands there now is the 28-key gap that
checking the repair uncovered, plus the gate to wire after it. **`(br)` in §0
is the only thing awaiting Ken.**

---

## §0 — THE BANANA IS DONE. GO LOOK AT IT, THEN ANSWER ONE QUESTION

The operator's stated first task — *"the first thing I am going to do is
refine the banana.pdf"* — **shipped** as `Pass 74.4` + `Pass 74.5`
(`e36f96e`).

### What to look at, first thing

**`C:\Users\Ken\OneDrive\pdfTests\`** — `banana-at-scale.pdf` (**684 kB**,
was 53 kB) and **NINE numbered renders**, `_1-whole-page` through
`_9-one-crista-atp-synthase`. The old set had six; **tiers 7, 8 and 9 are
new**, and they are the ones worth opening first.

The scale chain went **from six tiers to eight**. Smallest feature is a
**10 nm ATP synthase F1 head = 2.835 × 10⁻⁵ pt**, first readable at
**~35 000 000 %**.

### What changed in the drawing

Every mitochondrion is now drawn to its **real internal anatomy** instead
of an ellipse with a chord or three ruled across it — because a crista is
not a chord, it is an **invagination of the inner membrane** joined through
a narrow neck, and the space inside it is continuous with the intermembrane
space rather than the matrix. **Chords get the compartment topology
backwards**, which is the one thing a section view is for.

`tools/gen-scale-demo/mitochondrion.py` (603 lines, authored in
**nanometres**, emitted as shared Form XObjects): outer membrane 5 nm,
intermembrane space 24 nm, an inner membrane that is **one closed path**
running the boundary and diving in through each 18 nm crista junction to a
26 nm lumen, ATP synthase F1 heads at the real density gradient (11.5 nm
pitch on crista faces and rims, 62 nm on the flat boundary), mitoribosomes
26 nm, matrix granules 36 nm, mtDNA nucleoids 210 nm as tangled loops.

**All four populations share the module** — 14 in the pulp cytoplasm, 3 in
the skin cell, 325 beaded into the easter-egg heart and letters; **342
instances from 12 shared forms, 7 571 F1 heads** across the library.
Before this they were **three copy-pastes that had drifted**: three cristae
in the pulp cell, one or two in the easter egg, **none at all in the skin
cell**.

### ★★ THE ONE THING WAITING ON KEN — open question `(br)`

**May pdfce skip geometry smaller than a pixel?**

- Deep zoom is now **fast**: the `/BBox` viewport cull skips forms that
  cannot mark a pixel, and the deepest tier went **802 ms → 120 ms**. That
  cull is **exact** — ISO 32000-1 §8.10.1 makes `/BBox` a clip, and the
  test asserts the raster is byte-identical to the same page with the `Do`
  removed.
- **Page-fit zoom is the slow case now**: all 342 forms *are* on the canvas
  at ~1/70th of a pixel each, every path inside them is still rasterised,
  **~1.5 s** (was ~110 ms before the page gained ~1 000 000 path
  operators).
- Skipping sub-pixel geometry would fix that and **would be lossy** —
  those paths contribute anti-aliased coverage. **Decision 082** says a
  render may skip work only where skipping is exact, so this is **his
  call, not the engineer's.** Default if unanswered: **never skip**.

**Do not take this as a performance ticket.** It is `ROADMAP.md`'s open
operator question `(br)`, with three costed answers written out there.

### What is still on the imperfections list

Untouched by this Pass, and still worth doing if he wants more refinement:
pulp-cell **label collisions** (`nucleus` / `nucleolus + chromatin`
leaders converge; `central vacuole` sits near a starch grain); the
banana's **stem and blossom scar** read as placeholders; the **dart's
tail** is a blunt cut; `easter_egg.FONT` is **all-caps only** (lower case
needs curves, and curves from 1.5 µm beads stop reading as letters — the
honest route is *larger letters, not smaller beads*); the **heart glyph
sits low** on its line; **standard-14 Helvetica, not embedded** (what stops
this being PDF/A); **no `/Info` dictionary**; the page's **lower third is
empty**.

Two constraints that are load-bearing rather than decorative: **labels are
1/10 the largest cell's height** (30 µm = 0.085 pt, so cell size drives
label size), and **the arrow is a tapered dart rather than a shaft-and-head
because a conventional arrowhead is ~8 pt across — nine times wider than
the thing it points at.** The easter egg's binding constraint is
**legibility, not space**: ~8 beads per stroke is enough to read and not
enough to look smooth, which is why the anniversary line is ordinary type.

### How to look at any tier

`--region` takes **two corners in PDF user space**, not an origin and a
size. The viewport is what costs time, so keep the region small and put the
magnification in `--scale`. `tools/gen-scale-demo/README.md` carries the
current tier table with timings and the regions for all eight tiers; it was
rewritten in `e36f96e` and is accurate.

```
python tools/gen-scale-demo/gen_banana.py <out.pdf>
pdfce-cli render-page banana.pdf --page 1 --scale 1.6 -o page.png
```

---

## §0.5 — THE TWO DOC REPAIRS ARE **DONE**. WHAT REPLACED THEM IS TWENTY-EIGHT TIMES BIGGER.

**Discharged by `f9dc007`** (2026-08-22, `crates/pdfce-cli/src/main.rs` only,
+24 / −5), filed as the 229th filing: the `render-page:` template carries
`forms_culled=<S2>`, and the *"existing keys never move"* sentence now
promises **name**-stability rather than **ordinal** stability and names
`crates/pdfce-cli/tests/render_page.rs` as the enforcement. **Do not redo
this.**

### ★★★★ What the 229th filing found while checking that repair

It built the gate predicate the engineer asked about and **ran** it. The
published doc template shows **59 of the 87 keys** the `render-page`
`println!` emits. **28 are missing**, contiguous at the tail —
`blend_modes_applied` → `cmyk_unbridged_images` — and the per-key table
further down the same doc block is missing the same 28.

- Template's last extension: **`1e7a0be`, 2026-08-17** (the colour slice,
  ending at `img_uncalibrated`).
- The **next** slice, `bd244d9`, the **same day**, added keys and skipped it.
- **16 commits have edited `crates/pdfce-cli/tests/render_page.rs` since; none
  edited the template.** Five days, 28 keys, sixteen opportunities, zero
  propagations.

**Why only one direction:** the test asserts the key list by equality, so an
addition that skips the test **goes red**. The published copy has nothing that
goes red. That is standing rule **`R212`**, minted by this filing.

### What is owed, and the ORDER matters

1. **Repair the template** — 28 names, already in the right order in the
   test's assertion list. A copy, not a derivation.
2. **Repair the per-key table** in the same doc block — same 28, one row each.
3. **THEN** wire `tools/check-metrics-line-contract.py`. The predicate is
   written out in `ROADMAP.md`'s `f9dc007` entry, ready to take. **It is RED at
   baseline** — wiring it before steps 1–2 makes it a warning nobody reads
   inside a week
   (`D:/dev/rag/rust/ci_gate_red_at_baseline_enforces_nothing.md`).

**Take these in their OWN commit, not bundled into anything else.** A
filing commit that touches `crates/` or `tools/` becomes a code commit in
no filing and turns `check-commits-filed.py` red on the commit meant to
make it green — that is `d4721d8`'s memory file, and it is why the
librarian reports these instead of fixing them.

★ **The shape worth carrying:** the change that invalidated the original two
was made **deliberately, with an argument, in a test comment.** The reasoning
was written down — just not where the contract is published. **A well-argued
change is not a self-propagating one**, and a test edit in a diff is
**compulsion**, not evidence of diligence.

---

## §1 — THE THREE PRE-FLIGHT CHECKS, unchanged and still earning their place

**1. `ls` BOTH FeatureRequests channels.** They are outside this
repository, so **no gate will ever contradict a stale sentence about them —
including this one.**

```
D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\
D:\Dev\FeatureRequests\iccce_FeatureRequests\open\
```

Three sessions running, that `ls` found something a document said was not
there. On 2026-08-21 it found two `iccce` notes that had landed overnight:
one corrected a design sentence before it was implemented, and another —
*"a `(C, α)` buffer is a lossy representation of the model"* — was worth
**thirteen trap marks**. A reply is filed in that channel.

**2. Run the gates — `ls tools/check-*`, do not trust any list.**
`python tools/check-ci-parity.py --list` prints the local stand-ins.
**`R209`:** *"all gates green" names a set, and the set somebody runs is
not the set CI runs; a CI job with no local runner is UNOBSERVED, not
passing.*
*(All four gates the 228th filing ran — `check-commits-filed`,
`check-passes-filed`, `check-ledger-numbers`, `check-core-api-verbs` —
were green at exit 0 with only that filing's `docs/` edits in the tree.
**The 229th filing ran FIVE** — those four plus `check-ui-strings.sh` —
all exit 0. Note the set grew by one between two filings on the same day,
which is `R209`'s point: **"all gates green" names a set, and the set
somebody runs is not the set CI runs.**)*

**3. Read `docs/compositor-plan.md`** before scoping anything in `97.x`.

---

## §2 — ★★ THE GHENT PASS COUNT IS AN OVER-COUNT. Read this before quoting any figure.

`tools/ghent-check.py` implements **one of the suite's two pass criteria**.
It hunts for a **cross that should not be there**; seven patches instead
mark failure by the **absence of a check mark** (GWG 050, 080, 081, 082,
150, 151, 152) and have scored `clean` since the harness was written. At
least three of them are failures. A second fault — the contrast floor has
no **area** term — puts `GWG 1.0` in the same category.

**Corrected standing: 26 at most, not 29.**

⇒ **THE DELTAS SURVIVE, THE LEVELS DO NOT.** Every board ever filed is
over-counted by the same family, so a before/after comparison is sound and
any absolute "N of 51" is not.

The operator's cell-by-cell judgements are in
`docs/ghent-operator-review-2026-08-21.md` — **the only independent check
this harness has ever had**, and the calibration set for fixing it
(`Pass 122.2`).

---

## §3 — WHAT SHIPPED 2026-08-21/22

**`Pass 74.4` — Form XObject viewport culling** (`e36f96e`). `do_form`
skips a `Do` whose `/BBox`, mapped through the CTM, misses the canvas or
the clip, **without decoding its stream**. Exact by §8.10.1. New counter
`forms_culled`, printed beside `forms`.

★ **The finding is not the cull, it is where the cull went.** The first
version sat beside the `/BBox` clip — the natural-looking home — reported
the **same `339 of 342`** and bought almost nothing, because the stream had
already been sliced, flate-decoded and parsed: **~110 kB × 342 ≈ 37 MB of
inflate per render for content about to be discarded.** Hoisted above the
decode: **802 ms → 120 ms**. **A cull is worth only what it skips, and a
counter that reads identical either way is what makes the wrong version
convincing.** Accept an early-out on a wall clock, never on a hit rate.

**`Pass 74.5` — `tools/gen-scale-demo/mitochondrion.py`** (`e36f96e`) —
§0. Two silent bugs found by **rendering, not reading**: `PT_PER_NM`
written as a division where it needed the reciprocal (**8.03× too small**,
everything correctly placed and shaped, which reads as a drawing choice),
and a crista path that dived in on the wrong lateral side and **crossed its
own junction mouth twice** — stroked perfectly, while nonzero winding
filled every lumen with matrix colour. Catching the second took a
**160 000 000 % render of a single crista**.

**Deep zoom** (`bd9844d`, `71f7055` — `Pass 74.1`/`74.2`): `--region` on
the CLI, and a region's device geometry computed in `f64`. A requested
800×600 viewport used to come back **800×512** at 215 million percent; it
now holds to a trillion.

**The colorant buffer** (`97.1e`, `97.1f`): a page whose group declares a
subtractive blending space composites in four ink planes end to end. Ghent
wrong-space blends **107/107 → 0/107**. Corpus A/B: **3 731 identical of
4 023, 4 changed**, all four prepress conformance files.

**Two performance repairs** (`97.1i`, `97.1j`) — the first fixes a
regression `97.1e` shipped the same morning:

| page | pre-Pass | after `97.1e` | now |
|---|---|---|---|
| 1 | 632 ms | 3 713 ms | **320 ms** |
| 2 | 4 086 ms | 7 689 ms | **2 486 ms** |

**An `/Indexed`-over-`/DeviceN` palette bug** (`97.1h`), found by the
operator reading a test patch's caption: a two-colorant tint transform was
handed four inputs, refused, and fell back to a neutral — a grey palette,
with no counter anywhere. pdfium now agrees with us pixel for pixel.

**Pass IDs, and this is the sentence that matters when you commit:**
`74.1`–`74.5` were all **minted by filings**, not by commit messages.
**The next commit in this family must start at `74.6`.** Next free
elsewhere: `122.4`; `97.1g` is reserved and unbuilt. Decisions next free
**083**; standing rules next free **`R212`**; open operator questions next
free **`(bs)`**.

---

## §4 — THE QUEUE, in the order I would take it

0. **Answer-dependent:** if Ken answers `(br)` with "skip them" or "skip
   them behind a setting", that becomes the top item and it is a
   `pdfce-render` Pass with a **disclosure obligation** (a counter on the
   metrics line, off by default if it is a setting).
1. **The two `crates/` doc repairs in §0.5** — one commit, five minutes,
   and they are the freshest stale claims in the tree.
2. **`Pass 97.1g`** — non-isolated ordinary groups on a subtractive page
   are composited as if isolated. The arithmetic exists
   (`remove_backdrop_cmyk`); the second content walk does not. **A port of
   the additive path, not a design.**
3. **`Pass 97.1k`** — native colorant paths for images and shadings, which
   bridge through sRGB today (`cmyk_bridged_pixels`).
4. **`Pass 122.2`** — teach `ghent-check.py` the check-mark criterion and
   give its contrast floor an area term. §2 says why this is not optional.
5. **`Pass 122.1`** — per-sample image overprint. ★ Now **diagnosed**: it
   is why `GWG 8.2`'s check mark is missing. The mark is painted in yellow
   *underneath* the images, which overprint it so cyan-over-yellow reads
   green; pdfce paints the image normally and covers it
   (`overprint_images_unsupported = 2`). pdfium fails it too.
6. **`Pass 122.0`** — multithreading, the operator's request. His design
   (a runtime max-cores setting) is right and is kept; decision 080 adds a
   **compile-time target gate**, because `std::thread` and `rayon` both
   `cargo check` cleanly for `wasm32` and the CI wasm job therefore cannot
   catch a threading regression. Hard acceptance criterion: **byte-identical
   output at any core count.** ★ **Note the interaction with `Pass 74.4`:**
   the banana page is now a genuinely useful threading benchmark, because
   its cost is ~1 000 000 path operators in 342 forms rather than one big
   image.
7. **`Pass 119.1`** — `unshare_form`. Carried unstarted through six
   handoffs now.
8. **`Pass 122.3`** — the colorant buffer's byte ceiling. Interactive use
   is unaffected, but a **full-page** render above ~375 DPI refuses the
   buffer and silently composites in the wrong space, so one page can have
   different colours at different resolutions.

---

## §5 — STILL NOT DONE, named so it does not read as done

- **Sub-pixel geometry is still rasterised** at page-fit zoom — ~1.5 s on
  the banana page. **Blocked on `(br)`, deliberately.**
- `resolve_indexed` builds its palette with a **scratch
  `ColorDiagnostics` that is discarded**, so a tint failure inside a
  palette never reaches the operator.
- **Implicit knockout**: only explicit `/K true` is honoured. `/TK`
  defaults true (every text object), and `B`/`b` and shading patterns are
  knockout. **The one pdfce implements is the rarest.**
- **`/TR` on a soft mask** is read, counted, never evaluated.
- **`/AIS true`** is not distinguished from `/AIS false`.
- **Spot colorants** — four planes, not runtime `N`. Every remaining
  trap-criterion Ghent FAIL is in this bucket.
- **Per-paint rendering intent** (§11.7.5.3) — pdfce carries one per page.
  `iccce` costed the alternative and asked for the consumer fact; **no
  corpus measurement of mid-page intent switching has been taken.**
- **No GUI code path reads `forms_culled`**, `render_page_region` or the
  display list. GUI work is paused; recorded so the `[ ] gui` boxes in
  `FEATURES.md` are not mistaken for oversights.

---

## §6 — FOUR LESSONS FROM THIS RUN THAT ARE CLASSES, NOT INCIDENTS

1. **A cull is worth only what it skips.** Its hit counter is **invariant
   to its placement**, so the instrument you add to prove the optimisation
   works cannot detect that it barely works.
   `D:/dev/rag/rust/a_cull_placed_after_the_decode_reports_the_same_count_and_buys_almost_nothing.md`
2. **A unit constant inverted by a non-round factor renders as a drawing
   choice.** PDF user space has no declared unit, so nothing in the file
   and no validator can contradict a wrong scale. **Assert one known length
   at author time** — one line turns an invisible render defect into a
   build failure.
3. **A self-crossing subpath strokes perfectly and fills wrong.** Stroke
   and fill read one path through two unrelated rules, so **reviewing the
   outlined drawing is not evidence about the filled drawing** — and the
   outline is the one that looks diagnostic.
4. **Three copies of one drawing drift silently**, because a duplicated
   *drawing* has no compiler, no test and no diff anybody reads, and no two
   copies are ever on screen at the same magnification at the same time.

---

## §7 — HOUSEKEEPING

- **`origin/main` is at `c24ad7a`, THREE COMMITS BEHIND `HEAD`.**
  `d4721d8`, `dd47cd0` and `e36f96e` are **local only**. Pushing is the
  operator's act and needs a current go-ahead (`CLAUDE.md` rule 8). The
  repository is public, so **anything committed is published by default.**
  *(Measured 2026-08-22, 228th filing: `git remote -v`,
  `git rev-parse --short origin/main`.)*
- **Backups are 196 commits behind** — newest bundle
  `pdfce-20260817-v060.bundle` (2026-08-17 20:34) at `3c4c00e`. `v0.7.0`'s
  tag is in no bundle on disk. **Cutting one is the operator's call.**
  *(Measured 2026-08-22, 228th filing: `ls -lt D:\Dev\pdfce-backups\`,
  `git bundle list-heads`, `git rev-list --count 3c4c00e..HEAD`.)*
- **If `check-commits-filed.py` is red when you start, READ ITS OUTPUT FOR
  THE HASH.** Do not assume which commit it means, and **never extend
  `tools/commits-filed-baseline.txt`** — that file is pre-existing debt,
  not an allowlist. The 2026-08-22 afternoon handoff carried a conditional
  instruction that would have bought a duplicate filing, because its
  condition came true for a cause its author had not enumerated.
- ★ **A claim in `bd9844d`'s commit message is WRONG and stands corrected
  only in the filing.** It says `tools/check-string-gaps.sh` *"has caught
  it every time"*. It has not: `ae06440` (2026-08-20) is titled *"the
  string-gap gate reported two of three"*, and that miss was found by a
  human who knew there were three. The ordinal ("third time") survives; the
  "every time" does not.
- **Two measurement worktrees may still be on disk** under `%TEMP%`
  (`pdfce-base`, `pdfce-head`). `git worktree list` is authoritative.
