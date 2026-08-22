# gen-scale-demo — one page, eight orders of magnitude

Generates a letter-size PDF holding a **banana at life size** and **two of
its cells at that same scale**. Nothing on the page is enlarged for
clarity, which is the entire point: the cells really are 300 µm and 60 µm
across, so at a zoom where the page fits a screen they cover about one
pixel between them.

```
python tools/gen-scale-demo/gen_banana.py <out.pdf>
```

Nothing ships. This is a demonstration and a stress test, not a fixture —
it is not in `fixtures/`, no test loads it, and `cargo test` never runs it.

---

## 1. Why it exists

`Pass 74.1`/`74.2`'s deep-zoom work made region rendering flat across
magnification and pushed the numerical ceiling out past a **trillion
percent**. That is a claim about arithmetic. This file is the claim made
visible: a document you cannot read without exercising it.

It also happens to be the only artefact in this repository that a
non-programmer can look at and immediately understand what the renderer
does.

## 2. The scale chain

Every tier is roughly ten times the zoom of the one above it. Measured
with `pdfce-cli render-page --region`, each view a ~1600 × 1000 viewport:

| tier | true size | as points | readable from | render time |
|---|---|---|---|---|
| banana | 153 mm | 433 pt | 100 % | 1 490 ms |
| cell outlines | 300 µm | 0.85 pt | ~2 000 % | — |
| cell labels, starch grains | 30–45 µm | 0.07–0.13 pt | ~12 000 % | 1 600 ms |
| organelle labels | 8 µm | 0.023 pt | ~45 000 % | 1 420 ms |
| chloroplast grana, plasmodesmata | 1–5 µm | 0.003–0.014 pt | ~400 000 % | 120 ms |
| whole mitochondria | 1.2–3 µm | 0.003–0.009 pt | ~2 600 000 % | 243 ms |
| cristae, junctions, nucleoids | 18–210 nm | 5e-5 – 6e-4 pt | ~16 000 000 % | 182 ms |
| ATP synthase particles | 10 nm | 2.8e-5 pt | ~35 000 000 % | 190 ms |

★ **Render time still does not grow with magnification — it falls**, and
the reason is worth stating plainly. A viewport is a fixed number of
pixels whatever the page behind it is doing, so the top rows and the
bottom rows differ only in how much of the document is *in* it. The deep
tiers are the fast ones because most of the page is off-screen and now
gets skipped (§7).

★ **Every tier is ten to twenty times slower than the previous version of
this page, and that is the honest price of the detail.** The page went
from ~3 000 path operators to roughly **one million**: 342 mitochondria,
each now a full section with two membranes, 6–15 cristae, and 60–360 ATP
synthase particles. At page-fit zoom every one of those paths is
rasterised into about a hundredth of a pixel. §7 says what was fixed and
what was deliberately left alone.

## 3. The arithmetic the request imposed

The brief was specific, and two of its constraints turned out to be the
interesting ones.

- **Labels under each cell at 1/10 the height of the largest cell.**
  300 µm / 10 = **30 µm = 0.085 pt**. That is the smallest type most
  people have ever deliberately set.
- **The arrow ends two cell lengths above the cells.** 2 × 300 µm =
  **600 µm = 1.70 pt**. The arrow's point is at `y = 562.125984` and the
  cell's top edge is at `560.425197`; the difference is exactly that.
- ⇒ **and therefore the arrow could not have a conventional head.** One
  large enough to see at page scale is about 8 pt across — *nine times
  wider than the thing it points at*, which would bury the cells at the
  only zoom where they matter. It is a tapered dart with a **250 µm**
  head: an arrow from across the room, and an arrowhead under
  magnification.

## 4. Files

| file | holds |
|---|---|
| `gen_banana.py` | page layout, the banana's Béziers, scale bars, the dart, and the PDF writer |
| `cells_detail.py` | both cell interiors, authored in **micrometres** — a `Pen` converts to points at the last moment, so no drawing code ever sees a conversion factor |
| `easter_egg.py` | a stroke font and a path-to-mitochondria chainer |
| `mitochondrion.py` | **one mitochondrion, drawn to its real anatomy**, authored in **nanometres** and emitted as a shared Form XObject. Every mitochondrion on the page comes from here |

## 5. The biology, and where it is a simplification

Representative values, not measurements of a particular fruit. Both
figures are printed on the page beside the thing they describe, so the
drawing states its own assumptions.

- **Pulp** is parenchyma: thin primary wall, a vacuole filling most of the
  volume, cytoplasm squeezed to a peripheral band, the nucleus pushed
  against the wall — and the signature of an unripe banana, amyloplasts
  packed with large **concentrically layered starch grains with an
  eccentric hilum**.
- **Peel** epidermis is a brick under a waxy **cuticle**, its outer wall
  far thicker than its side walls, with **chloroplasts** (so: unripe;
  they become carotenoid chromoplasts as it ripens).
- Simplified on purpose: no ER network beyond a few strands, no cytosolic
  ribosomes, no membrane bilayers. **Organelle counts are illustrative** —
  a real section would show far more mitochondria.
- **The mitochondrion is the exception, and is not simplified.** §6.

## 6. The mitochondrion, the one thing on this page drawn in full

Every mitochondrion used to be an ellipse with a chord or three ruled
across it. That reads correctly at the zoom where the organelle is a few
pixels wide and is **wrong at every zoom below it**, because a crista is
not a chord. It is an invagination *of the inner membrane*, joined to it
through a narrow neck, and the space inside a crista is continuous with
the space between the two membranes — **not** with the matrix. Drawing
cristae as chords gets the compartment topology backwards, which is the
one thing a section view exists to show.

`mitochondrion.py` now draws, outside in:

| feature | size | first readable at |
|---|---|---|
| outer membrane | 5 nm | ~2 600 000 % |
| inner boundary membrane | 4.5 nm | ~2 600 000 % |
| crista junction (the neck) | 18 nm | ~4 000 000 % |
| intermembrane space | 24 nm | ~3 000 000 % |
| crista lumen | 26 nm | ~3 000 000 % |
| mitoribosome | 26 nm | ~3 000 000 % |
| matrix granule | 36 nm | ~2 000 000 % |
| mtDNA nucleoid | 210 nm | ~400 000 % |
| **ATP synthase F1 head** | **10 nm** | **~35 000 000 %** |

Three details are worth knowing, because they are the ones a diagram
usually gets wrong.

- **The inner membrane is ONE path.** Boundary arc, dive in to form a
  crista, round the blind end, come back out, continue along the
  boundary — a single closed curve for the whole organelle. Filling it
  with the matrix colour therefore leaves each crista lumen showing the
  intermembrane colour laid down beneath it, so the compartments come out
  right *by construction*, instead of by drawing the lumen as a separate
  object that could drift out of register with its own membrane.
- **A self-crossing at the junction is invisible until you fill it.** The
  first version dived in on the wrong lateral side, so the path crossed
  its own junction mouth twice — a bow tie. Every membrane was still
  exactly the right shape and the stroked outline looked perfect, but
  nonzero winding then filled the lumen with matrix colour: the same
  compartment error, one layer down and much harder to see. Catching it
  took a **160 000 000 %** render of a single crista, because at anything
  less the lumen is too narrow to read a colour from.
- **ATP synthase density follows the real gradient.** Dimer rows crowd the
  highly curved crista rims and faces — that curvature is largely *caused*
  by them — while the flat boundary membrane carries far fewer. Spaced
  11.5 nm on cristae and 62 nm on the boundary, so the gradient is
  visible rather than asserted.

All four populations share this module: the 14 in the pulp cytoplasm, the
3 in the skin cell, and the 325 beaded into the easter egg. Before, they
were three copy-pastes that had drifted — three cristae, one or two
cristae, and **none at all** in the skin cell. The beads in the letters
are smaller organelles now, not simpler ones.

**They are Form XObjects**, twelve of them, placed 342 times. Two reasons,
and the second is the one that made it non-optional:

1. **Size.** 342 inline copies of ~2 500 path operators would be megabytes
   of content stream on a page whose entire previous content was 53 kB.
2. **Precision.** A 10 nm feature is 2.8e-5 pt. Written as an absolute
   page coordinate near `x = 540` that needs eleven significant figures —
   past the five decimal digits ISO 32000-1 Annex C says a conforming
   reader need only honour. Inside a form it is the literal `5.000`, and
   one matrix per instance carries the magnitude.

A side effect worth having: the page is now a deep-zoom stress test of the
renderer's Form XObject path under an extreme CTM, not only of its path
rasteriser.

## 7. What this cost the renderer, and what got fixed

Building this page found a real defect in `pdfce-render`, which is most of
the value of having built it.

**Every `Do` executed its form in full, however far off-screen it was.**
Rendering the deepest tier — a viewport 0.09 pt wide, holding parts of
maybe three organelles — decoded, parsed and rasterised all 342 of them,
then discarded ~700 000 paths against the clip. §8.10.1 makes `/BBox` a
*clip* on a form's contents, so a form whose transformed box misses the
viewport **cannot** contribute a pixel, and skipping it is exact rather
than approximate. `do_form` now culls on that and reports `forms_culled`
beside `forms` on the `render-page` metrics line.

**Where the cull sits is the whole point, and the first attempt got it
wrong.** Placed next to the `/BBox` clip — the natural-looking home — it
reported the *same counts* and bought almost nothing, because by then the
stream had already been sliced, inflated and parsed. Moved ahead of the
decode, the same tier went **802 ms → 120 ms**. A cull is worth only what
it skips, and the counter looked identical either way, which is exactly
why the wrong version was convincing.

**What was deliberately NOT done.** At page-fit zoom all 342 forms *are*
on the canvas, each about 1/70th of a pixel across, and pdfce still
rasterises every path inside them — about 1.5 s. Skipping sub-pixel
geometry would fix that and would be **lossy**: those paths do contribute
anti-aliased coverage, and pdfce does not silently trade fidelity for
speed. That is an operator decision, not an engineering one, and it is
left open rather than quietly taken.

## 8. The easter egg, and the arithmetic that said it would fit

Inside the pulp cell's vacuole: a heart drawn from mitochondria, with
`KEN ♥ EMILY` inside it also drawn from mitochondria — the ♥ is itself a
little heart of mitochondria — and an anniversary line beneath.

The question was whether mitochondria are too big to fit it. They are not,
and it is not close:

```
cell          300 µm            = 200 mitochondria wide
outer heart   140 × 125 µm      ≈ 190 mitochondria around its curve
capitals      12 µm tall        ≈ 8 mitochondria per vertical stroke
"KEN ♥ EMILY" 9 glyphs, 81 µm   ≈ 210 mitochondria
```

★ **The binding constraint is legibility, not space.** A 12 µm capital
built from 1.5 µm beads has about eight per stroke — enough to read, not
enough to look smooth. Below that the letters become dotted lines, which
is why the anniversary text underneath is ordinary type: at 7 µm a beaded
letter would be four mitochondria tall and stop being a letter.

The starch grains were moved into a ring around the vacuole to clear the
centre. That is the only change the egg required, and it costs nothing
biologically — grains cluster wherever they cluster.

Every bead in the heart and the letters is now a **fully detailed
mitochondrion** (§6), with its own cristae, junctions, ATP synthase and
nucleoid. Three interior variants and a mirror give six apparent
organelles before the pattern repeats, which at this bead pitch is far
enough apart that the eye reads tissue rather than a stamp.
