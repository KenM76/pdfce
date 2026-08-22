# gen-scale-demo — one page, six orders of magnitude

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

`Pass 122.x`'s deep-zoom work made region rendering flat across
magnification and pushed the numerical ceiling out past a **trillion
percent**. That is a claim about arithmetic. This file is the claim made
visible: a document you cannot read without exercising it.

It also happens to be the only artefact in this repository that a
non-programmer can look at and immediately understand what the renderer
does.

## 2. The scale chain

Every tier is roughly ten times the zoom of the one above it. Measured
with `pdfce-cli render-page --region`, each view a 1600 × 1000 viewport:

| tier | true size | as points | readable from | render time |
|---|---|---|---|---|
| banana | 153 mm | 433 pt | 100 % | 47 ms |
| cell outlines | 300 µm | 0.85 pt | ~2 000 % | — |
| cell labels, starch grains | 30–45 µm | 0.07–0.13 pt | ~12 000 % | 61 ms |
| organelle labels | 8 µm | 0.023 pt | ~45 000 % | 73 ms |
| chloroplast grana, plasmodesmata | 1–5 µm | 0.003–0.014 pt | ~400 000 % | 46 ms |
| mitochondrial cristae | 0.7 µm | 0.002 pt | ~2 600 000 % | 50 ms |

★ **The render time does not grow.** It cannot: a viewport is a fixed
number of pixels whatever the page behind it is doing. That flatness is
the property the deep-zoom work bought, and this table is the cheapest
demonstration of it the project owns.

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
- Simplified on purpose: no ER network beyond a few strands, no ribosomes,
  no membrane bilayers. **Organelle counts are illustrative** — a real
  section would show far more mitochondria.

## 6. The easter egg, and the arithmetic that said it would fit

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
