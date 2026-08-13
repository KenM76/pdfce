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

## Not measured in THIS section

The requesting session's question 3 — the honest upper bound on magnification —
is answered in the second half of this document, below.

---

# The magnification ceiling — measured, and it is not where anyone would guess

**Added 2026-08-13**, answering question 3 of the same request (*"the honest
upper bound on magnification before the output stops being meaningful"*), which
the first version of this document recorded as owed.

```text
cargo run --release -p pdfce-render --example zoom_ceiling
```

**Method.** A 1 pt black bar whose left edge sits at **x = 2999.7373 pt** on a
3370 pt (A0) sheet. At each scale a tight region is rendered around that edge
and the first ink column is compared against where the arithmetic says it must
land. Error is in **device pixels**.

Position matters as much as scale: `f32` carries a 24-bit mantissa, so it is
the **absolute magnitude** of `coordinate × scale` that consumes precision. A
bar near the origin would show nothing.

| scale | region px | error px |
|---:|---|---:|
| 1.3 | 1 × 1 | 0.755 |
| 21 | 3 × 2 | 0.076 |
| 336 | 28 × 15 | 0.222 |
| 672 | 54 × 28 | 1.444 |
| 2,690 | 216 × 107 | 0.775 |
| 5,381 | 431 × 214 | 0.551 |
| 10,762 | 862 × 428 | 0.102 |
| 21,524 | 1724 × 856 | 0.797 |
| **43,047** | 3448 × 1712 | **2.594** |
| 86,095 | 6896 × 3424 | 5.187 |
| 172,189 | 13792 × 6848 | 11.374 |

## Reading it

Below ~5,000× the error is **sub-pixel and non-monotonic** — it wanders
between 0.02 and 1.4 px with no trend. That is anti-aliasing and threshold
rounding on where the ink crosses the detection cut, **not** precision decay.

Beyond ~43,000× it **doubles as the scale doubles** (2.59 → 5.19 → 11.37).
That is the signature of `f32` mantissa exhaustion, and the arithmetic agrees:
`2999.7373 × 43,047 ≈ 1.29 × 10⁸`, well past the 1.677 × 10⁷ above which the
`f32` spacing exceeds 1.0, so the representable gap — and hence the error —
scales with the coordinate from there.

## ★ The conclusion, which is the useful part

**Numerical precision is not the binding constraint on magnification, and is
not close to being one.** On the worst realistic case — a coordinate near
3,000 pt on an A0 sheet — device coordinates stay sub-pixel accurate to roughly
**5,000×**, three orders of magnitude beyond any plausible viewing zoom.

**So `MAX_ZOOM` must be set from performance and usability, not from
numerics.** The real limit is the ~0.7–1.1 s per-render interpretation floor
measured above. Setting it from `f32` would be picking a number for a reason
that does not apply — the same class of error `MAX_PIXMAP_EDGE`'s original
justification made.

## What is still NOT covered here

Hairline strokes (`0 w`), which render at a device-minimum width regardless of
scale and therefore get relatively *thinner* the deeper the zoom, and text
hinting at extreme sizes. Both are **appearance** questions rather than
correctness ones, and neither is measured by this harness. Stated so the
"measured" claim above is not read wider than it is.
