# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. Overwrite it once acted on.

**Written 2026-08-22**, replacing the 2026-08-21 (evening) handoff.

---

## §0 — THE OPERATOR'S FIRST TASK IS THE BANANA, NOT THE COMPOSITOR

He said so directly: *"the first thing I am going to do is refine the
banana.pdf."* Everything from §3 onward is the engineering queue and it can
wait. Start here.

### What it is

`tools/gen-scale-demo/` generates a letter-size PDF holding a **banana at
life size** and **two of its cells at that same scale**, so the deep-zoom
work becomes a document you cannot read without exercising it. Six tiers,
each ~10× the zoom of the one above, all rendering in 46–73 ms.

```
python tools/gen-scale-demo/gen_banana.py <out.pdf>
```

Read `tools/gen-scale-demo/README.md` first — it carries the scale table,
the biology, and the arithmetic behind the easter egg. Three modules:
`gen_banana.py` (page, banana, dart, PDF writer), `cells_detail.py` (both
cell interiors, authored in **micrometres**), `easter_egg.py` (stroke font
plus a path-to-mitochondria chainer).

### Where the outputs are

- **OneDrive**, `C:\Users\Ken\OneDrive\pdfTests\` — `banana-at-scale.pdf`
  plus six numbered renders (`_1-whole-page` … `_6-mitochondria-2600000pct`).
  This is the copy the operator looks at.
- Working copies were in `%TEMP%\bananademo\`, which is **not durable** —
  regenerate rather than assume they are still there.

### How to look at any tier, which is the thing to learn first

`--region` takes **two corners in PDF user space**, not an origin and a
size. The viewport is what costs time, so keep the region small and put
the magnification in `--scale`:

```
# whole page
pdfce-cli render-page banana.pdf --page 1 --scale 1.6 -o page.png

# the two cells                       (~62 000 %)
… --scale 620   --region "539.2,558.9,541.9,563.2"
# the pulp cell and its labels        (~90 000 %)
… --scale 900   --region "538.9,558.6,541.9,561.0"
# the skin cell                       (~420 000 %)
… --scale 4200  --region "541.05,559.85,541.42,560.16"
# the easter egg                      (~360 000 %)
… --scale 3600  --region "539.79,559.76,540.24,560.25"
# single mitochondria in the letters  (~2 600 000 %)
… --scale 22000 --region "539.93,560.00,540.02,560.06"
```

### ★ Known imperfections, so refinement starts from a list rather than a squint

Observed in the rendered output, roughly in the order they annoy:

1. **Pulp-cell label collisions.** `nucleus` and `nucleolus + chromatin`
   have converging leaders and overlapping text; `central vacuole` was
   moved to `(96, 250)` to dodge the easter egg and now sits near the
   top-centre starch grain. The label list is a plain table at the bottom
   of `draw_pulp` — cheap to move.
2. **The banana's ends.** The stem is a flat quadrilateral and the blossom
   scar a small triangle; both read as placeholders next to the body. The
   longitudinal ridge stops slightly short of the right tip.
3. **The dart's tail** is a blunt cut at `(468, 646)`. Fine at page scale,
   crude if anyone zooms the tail.
4. **All-caps names.** `easter_egg.FONT` has no lower case — the operator
   wrote "Ken ♥ Emily" and it renders `KEN ♥ EMILY`. Lower case needs
   curves, and curves built from 1.5 µm beads stop reading as letters; if
   mixed case is wanted, the honest route is **larger letters, not smaller
   beads**.
5. **The heart glyph sits slightly low** relative to the capitals on its
   line.
6. **Fonts are the standard-14 Helvetica, not embedded.** Fine for pdfce
   and any normal viewer; it is what stops this being PDF/A, and it means
   the page depends on the reader having Helvetica metrics.
7. **No `/Info` dictionary** — no title, author or subject. A page this
   presentable should probably have one.
8. **The page's lower third is empty** below the scale-chain text.

### Two constraints that are load-bearing, not decoration

- **Labels under each cell are 1/10 the largest cell's height** — 300/10 =
  30 µm = **0.085 pt**. Change the cell size and the label size follows.
- **The arrow ends two cell lengths above the cells** — 600 µm = 1.70 pt.
  Its point is at `y = 562.125984`; the cell's top edge is at
  `560.425197`. ★ **And that is why it is a tapered dart rather than a
  shaft-and-head**: a conventional arrowhead visible at page scale is
  ~8 pt across, *nine times wider than the thing it points at*, and would
  bury the cells at the only zoom where they matter.

### The easter egg, and the number that governs it

A heart of mitochondria in the vacuole's clear centre, `KEN ♥ EMILY`
inside it also of mitochondria (the ♥ is itself a small heart of them),
anniversary line beneath in ordinary type.

★ **The binding constraint is LEGIBILITY, not space.** The cell is 200
mitochondria wide; there is room for far more. But a 12 µm capital built
from 1.5 µm beads has ~8 per stroke — enough to read, not enough to look
smooth — and below that the letters become dotted lines. That is why the
anniversary text is ordinary type: at 7 µm a beaded letter would be four
mitochondria tall and stop being a letter. **Any "make it smaller" request
runs into this before it runs into space.**

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

**The colorant buffer** (`97.1e`, `97.1f`): a page whose group declares a
subtractive blending space composites in four ink planes end to end. Ghent
wrong-space blends **107/107 → 0/107**. Corpus A/B: **3 731 identical of
4 023, 4 changed**, all four prepress conformance files.

**Two performance repairs** (`97.1i`, `97.1j`) — and the first fixes a
regression `97.1e` shipped the same morning:

| page | pre-Pass | after `97.1e` | now |
|---|---|---|---|
| 1 | 632 ms | 3 713 ms | **320 ms** |
| 2 | 4 086 ms | 7 689 ms | **2 486 ms** |

A page-sized coverage mask per **paint** (every glyph is a paint), and a
page-sized 38.8 MB child buffer per **transparency group** (142 on one
page). Both are now reused.

**An `/Indexed`-over-`/DeviceN` palette bug** (`97.1h`), found by the
operator reading a test patch's caption: a two-colorant tint transform was
handed four inputs, refused, and fell back to a neutral — a grey palette,
with no counter anywhere. pdfium now agrees with us pixel for pixel.

**Deep zoom** (`bd9844d`, `71f7055`): `--region` on the CLI, and a region's
device geometry computed in `f64`. A requested 800×600 viewport used to
come back **800×512** at 215 million percent; it now holds to a trillion.

**`tools/gen-scale-demo/`** (`83409b3`) — §0.

Filed as **`Pass 74.1`/`74.2`/`74.3`**, not `122.x`. The `74.x` family is
`render_page_region` itself (`Pass 74.0`, `2fe6216`) and all three commits
ARE that function; `122.0`–`122.3` are four unrelated items that happen to
share a filing date. Decision **081** and rule **`R211`** were minted with
them.

---

## §4 — THE QUEUE, in the order I would take it

1. **`Pass 97.1g`** — non-isolated ordinary groups on a subtractive page
   are composited as if isolated. The arithmetic exists
   (`remove_backdrop_cmyk`); the second content walk does not. **A port of
   the additive path, not a design.**
2. **`Pass 97.1k`** — native colorant paths for images and shadings, which
   bridge through sRGB today (`cmyk_bridged_pixels`).
3. **`Pass 122.2`** — teach `ghent-check.py` the check-mark criterion and
   give its contrast floor an area term. §2 says why this is not optional.
4. **`Pass 122.1`** — per-sample image overprint. ★ Now **diagnosed**: it
   is why `GWG 8.2`'s check mark is missing. The mark is painted in yellow
   *underneath* the images, which overprint it so cyan-over-yellow reads
   green; pdfce paints the image normally and covers it
   (`overprint_images_unsupported = 2`). pdfium fails it too.
5. **`Pass 122.0`** — multithreading, the operator's request. His design
   (a runtime max-cores setting) is right and is kept; decision 080 adds a
   **compile-time target gate**, because `std::thread` and `rayon` both
   `cargo check` cleanly for `wasm32` and the CI wasm job therefore cannot
   catch a threading regression. Hard acceptance criterion: **byte-identical
   output at any core count.**
6. **`Pass 119.1`** — `unshare_form`. Carried unstarted through five
   handoffs now.
7. **`Pass 122.3`** — the colorant buffer's byte ceiling. Interactive use
   is unaffected, but a **full-page** render above ~375 DPI refuses the
   buffer and silently composites in the wrong space, so one page can have
   different colours at different resolutions.

---

## §5 — STILL NOT DONE, named so it does not read as done

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

---

## §6 — THREE LESSONS FROM THIS SESSION THAT ARE CLASSES, NOT INCIDENTS

1. **A deferral that states only the per-unit cost is a hope, not a
   measurement.** `cmyk_paint.rs` documented the 259 µs page-sized mask and
   deferred it with sound reasoning. Nobody multiplied it by the number of
   marks on a page. 5.9× slower.
2. **A plausible explanation that predicts the right order of magnitude is
   not a diagnosis.** Two hypotheses both predicted "~20 ms per group,
   scaling with page size"; only one survived being asked to predict cost
   versus page **area**.
3. **An identical failure count across three different fixes means you are
   fixing the wrong thing.** 5 841 bytes, three times, unmoved — and the
   test's own message had already said *"the window is OFFSET, not that the
   drawing is wrong"*.

---

## §7 — HOUSEKEEPING

- **`origin/main` is level** and the repository is public. Anything
  committed is published by default.
- **Backups are 193 commits behind** — newest bundle
  `pdfce-20260817-v060.bundle` at `3c4c00e`. `v0.7.0`'s tag is in no bundle
  on disk. **Cutting one is the operator's call.**
  *(Measured 2026-08-22, 227th filing: `git rev-list --count 3c4c00e..HEAD`.
  Was written "~190" here.)*
- ~~A librarian filing for `bd9844d`, `71f7055` and `83409b3` was dispatched
  in the same window as this file. If `check-commits-filed.py` is red when
  you start, that filing did not land — re-dispatch rather than adding to
  the baseline.~~
  **CORRECTED 2026-08-22 (227th filing) — the strikethrough above would have
  cost you a duplicate filing.** That filing **did** land, as `c24ad7a`, and
  it filed all three commits as `Pass 74.1`/`74.2`/`74.3`. The gate was then
  red **for a different reason the sentence never enumerated: `c24ad7a`
  itself**, which bundled three code repairs into a filing commit and so
  became a code commit in no filing. It is filed by the **227th** entry.
  ⇒ **A conditional instruction goes stale when its condition comes true for
  a cause the author did not list.** If `check-commits-filed.py` is red when
  you start, **read its output for the hash** — do not assume which commit it
  means, and never extend `tools/commits-filed-baseline.txt`.
- ★ **A claim in `bd9844d`'s commit message is WRONG and stands corrected
  only in the filing.** It says `tools/check-string-gaps.sh` *"has caught
  it every time"*. It has not: `ae06440` (2026-08-20) is titled *"the
  string-gap gate reported two of three"*, and that miss was found by a
  human who knew there were three. Three episodes, one under-report. The
  ordinal ("third time") survives; the "every time" does not.
- **Two measurement worktrees may still be on disk** under `%TEMP%`
  (`pdfce-base`, `pdfce-head`). `git worktree list` is authoritative.
