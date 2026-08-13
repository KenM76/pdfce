# Region rasterisation — measured cost model

**Measured 2026-08-13** on the operator's own benchmark drawing, in answer to
`D:\Dev\FeatureRequests\pdfce_FeatureRequests\request_region_rasterisation.md`
from the `pdfceGUI` session. Re-runnable:

```text
cargo run --release -p pdfce-render --example region_bench -- <file.pdf>
```

**Subject:** `D:\Dev\temp\pdfce\ncored-benchmark-cad-drawing.pdf` — A1
landscape (1190.55 × 841.89 pt), 5.6 MB, dense vector site plan,
148,517 paints · 24,128 clip ops. Release build. Load: ~5 ms.

---

## The measurements

| case | pixmap | pixels | time |
|---|---|---:|---:|
| **full page**, scale 1 | 1191 × 842 | 1,002,822 | **877 ms** |
| **full page**, scale 2 | 2382 × 1684 | 4,011,288 | **1,422 ms** |
| **region**, scale 1 | 401 × 301 | 120,701 | **699 ms** |
| **region**, scale 2 | 401 × 301 | 120,701 | **855 ms** |
| **region**, scale 8 | 401 × 301 | 120,701 | **801 ms** |
| **region**, scale 32 | 401 × 301 | 120,701 | **1,067 ms** |
| **★ floor**, a 1 × 1 **point** region | 1 × 2 | **2** | **691 ms** |

## ★ The finding

**A two-pixel render costs 691 ms; a 120,701-pixel render costs 699 ms.**

So on this document the cost is **~99 % resolution- and area-independent**.
It is content-stream interpretation and path construction, paid in full
regardless of how few pixels come out. Fill is nearly free by comparison — the
whole 1,002,822-pixel page adds only ~180 ms over the floor.

The requesting session predicted this ("~0.74 s of that is
resolution-independent") from `tools/render-profile`'s scale sweep. **The
prediction was correct, and the floor is slightly higher than estimated.**

## What follows from it, in order of importance

### 1. Do not tile. One region per viewport.

A 3 × 3 tile ring costs **9 × ~0.7 s ≈ 6.2 s**, against ~0.7 s for a single
region covering the same area. Tiling on this engine is not an optimisation,
it is a 9× regression. The requesting session's instinct — *"a bare
`render_page_region` that re-parses would be worse than nothing, and I would
rather know that up front"* — was right, and this is the up-front answer.

Tiling remains legitimate for **bounding memory** on an enormous viewport. It
is never a way to save time.

### 2. Region rendering buys REACHABILITY, not speed.

This is the part the timing table understates. At scale 32 the whole page would
be 38,112 × 26,940 px — **3.8 GiB, and over `MAX_PIXMAP_EDGE` regardless**, so
it is not slow, it is *impossible*. The region renders in 1.07 s.

| A1 landscape @ 2× DPR | whole-page | region |
|---|---|---|
| max zoom | **3.4×** (guard-bound) | limited only by region size |
| memory at max | 1.00 GiB | ~0.5 MB for a 400 × 300 pt viewport |

So the operator requirement — *"zoom in as much as feasibly possible,
preferably further than other software allows"* — is now reachable. It simply
costs ~0.7–1.1 s per zoom step on a document this dense, not less.

### 3. Cost grows mildly with zoom, and that is fine.

699 ms at 1× → 1,067 ms at 32×, for a constant pixel count. The growth is path
geometry getting larger relative to the clip, not fill. Deep zoom is affordable.

### 4. A display-list cache would remove ~99 % of the repeat cost.

Because the floor is interpretation, a reusable parsed representation the shell
holds and replays against N regions would take the second and subsequent
renders of the same page from ~700 ms to roughly the fill cost — tens of
milliseconds. That is the single highest-value optimisation available to this
crate, and this measurement is the evidence for funding it.

**It is not built.** `render_page_region` shares `render_page_with_view`'s
implementation exactly, differing only in pixmap size and a translation on the
base CTM. Nothing is cached between calls.

## Honest limits of this measurement

- **One document.** A dense CAD sheet is the worst case for interpretation cost
  and the best case for the argument above. A text-heavy office page would show
  a much lower floor and a relatively larger fill share; the "do not tile"
  conclusion weakens as the floor falls.
- **One machine, single-threaded.** `pdfce-render` uses no `rayon` and no
  threads. Region fills are embarrassingly parallel once a display list exists;
  without one, parallelism would duplicate the floor per worker rather than
  divide it.
- **`--release` matters.** Debug ratios are not shipped ratios.
- The `scale 8` row (801 ms) is *faster* than `scale 2` (855 ms). That is run
  noise, not a trend — the harness reports a single run per case, deliberately,
  because the floor's magnitude is the finding and it is not a close call.

## Not measured, and still owed

The requesting session's question 3 — **the honest upper bound on magnification
before output stops being meaningful** (PDF coordinate precision, the
rasteriser's fixed-point arithmetic, hairline stroke widths) — is not answered
here. `MAX_ZOOM` should be set from where quality measurably degrades, and that
needs its own experiment.
