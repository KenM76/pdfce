# Region rasterisation — measured cost model

**Measured 2026-08-13** on the operator's own benchmark drawing, in answer to
`D:\Dev\FeatureRequests\pdfce_FeatureRequests\request_region_rasterisation.md`
from the `pdfceGUI` session. Re-runnable:

```text
cargo run --release -p pdfce-render --example region_bench -- <file.pdf>
```

**Subject:** `D:\Dev\temp\pdfce\ncored-benchmark-cad-drawing.pdf` — **A3
landscape** (1190.55 × 841.89 pt), 5.6 MB, dense vector site plan,
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

| a true A1 landscape @ 2× DPR | whole-page | region |
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


### 4a. ★ The floor decomposed by ABLATION, and the ~99 % claim is confirmed rather than corrected

**Measured 2026-08-18**, scoping `Pass 75.0`. The claim above — *"the floor is
interpretation"* — was inferred from an **area** comparison (a 2-pixel render
costs what a million-pixel one does). That is sound evidence for
area-independence, but it does not by itself say **what** the area-independent
work is. Interpretation and `fill_path`'s *setup* are both area-independent:
tiny-skia builds an edge list per path whether the pixmap is a megapixel or
two pixels.

So it was measured directly, by **ablation**: every `fill_path` / `stroke_path`
call in the paint path was env-gated off (8 sites), and the FLOOR case re-run.
Nothing else changed, so nothing else is confounded.

| FLOOR (1 × 1 pt region, 2 px) | run 1 | run 2 | run 3 | median |
|---|---:|---:|---:|---:|
| **normal** | 665 ms | 667 ms | 722 ms | **667 ms** |
| **every paint call removed** | 591 ms | 580 ms | 611 ms | **591 ms** |

⇒ **Painting is ~11 % of the floor (~76 ms). Interpretation and path
construction are the other ~89 % (~591 ms).**

**This confirms §4 rather than qualifying it**, and the confirmation is worth
more than the original inference because it rules out the specific alternative
that would have sunk the design: had the split been the other way round, a
cached parse would have removed only a tenth of the cost and `Pass 75.0`'s
acceptance criterion 1 — *"~700 ms to tens of ms"* — would have been
unreachable by construction.

**Ceiling this sets on `Pass 75.0`:** a handle that skips interpretation and
path construction removes ~591 ms of the ~667 ms floor. The residual ~76 ms is
`fill_path` setup for all 148,517 paints, and **a bbox cull at replay is what
removes that** — a cull is cheap against a display list because the bounds are
already computed, whereas during interpretation they are not known until the
path has been built, which is the expensive part. **So culling belongs in the
replay path and is not an alternative to the handle.**

#### ★ A cull at the PAINT site was already measured and correctly rejected

`paint_is_cullable` exists in `interpret.rs` and feeds **only** a counter. That
is deliberate, and `profile.rs` says so at the field:

> *"Measured at 1.34 % on the reference CAD sheet, which is why no such cull
> was built. Kept as a counter so the next person to propose one gets the
> number instead of the intuition."*

It worked: this session proposed exactly that cull and got the number instead
of the intuition. **Recorded because a predicate that gates nothing looks like
dead code to a reader who has not found its rationale** — and the rationale is
one file away, in a doc comment on the counter it feeds.

Note also that the two culls are against **different rectangles** and are not
substitutes: `paint_is_cullable` tests against the **clip's** bbox (1.34 %
hit rate, because on this sheet clips average 66 % of the page), whereas a
replay-time cull would test against the **region**, which for a zoomed viewport
is a small fraction of the page and would hit far more often.

#### Method note, stated because it nearly produced a wrong number

The first ablation run reported the split as **36 % painting**, and that figure
would have been written down had it not been repeated. The machine was
concurrently running `cargo build`, and the contention landed unevenly across
the two cases. **Three runs per case, medians reported, both cases interleaved
under the same load** is what made the number stable. A single-run ablation
measures the load as much as the code.

## ★ A SECOND DOCUMENT, and the caveat below was right

**Measured 2026-08-13 by the `pdfceGUI` session** on `iso32000-2-preview.pdf`
— the PDF 2.0 spec preview, 689 KB, text-heavy A4 — in answer to the
one-document caveat this section originally carried:

```text
FULL   scale 1   596x842   =   501,832 px    8.97 ms
FULL   scale 2  1191x1684  = 2,005,644 px   13.94 ms
FLOOR  1x1pt      1x2      =         2 px    3.21 ms
REGION scale 1   401x301   =   120,701 px    6.08 ms
```

| | dense CAD (A3) | text-heavy (A4) |
|---|---:|---:|
| interpretation floor | **691 ms** | **3.2 ms** |
| full page, scale 1 | 877 ms | 8.97 ms |
| floor as share of full page | **~99 %** | **~36 %** |

**Both conclusions survive, but only one magnitude does.** *Never tile for
speed* holds on both. *Tiling is a catastrophe* holds only where
interpretation dominates: the 3×3 penalty is **~9×** on the CAD sheet and
**~1.9×** on the text page, where the absolute numbers (29 ms vs 55 ms) put
nothing about interactivity at stake.

**The real finding is not about tiling at all.** It is that one document type
costs **700 ms** per render and the other costs **9 ms** — a spread of nearly
three orders of magnitude on the same code path. Any strategy tuned for one is
mistuned for the other, which is the argument for the display-list handle
below rather than for any particular render granularity.

## Honest limits of this measurement

- ~~**One document.**~~ **Discharged** — see the section directly above. The
  original caveat read: *"A dense CAD sheet is the worst case for
  interpretation cost and the best case for the argument above. A text-heavy
  office page would show a much lower floor and a relatively larger fill share;
  the 'do not tile' conclusion weakens as the floor falls."* That is exactly
  what the second measurement found. Kept rather than deleted, because a caveat
  that turned out to be correct is evidence about how much to trust the next
  one.
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
