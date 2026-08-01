# render-parity — full-page pdfium pixel-parity harness (Pass 11)

The standing **render-fidelity verification gate** (decision 010, candidate
C). It proves pdfce's render stack against an *independent* reference
renderer — pdfium, via `pypdfium2` — at corpus scale, replacing the
self-comparison round-trip oracle ("pdfce agrees with pdfce") with a
measured, bucketed, by-file/by-reason fidelity report.

This file is the **logic**; `render_parity.py` is the syntax that enacts it.
A competent engineer should be able to rebuild the harness from this README.

---

## 1. Why this exists (the forcing consumer)

The self-comparison oracles pdfce already ships — `tools/roundtrip` (object
identity, R34) and `tools/content-identity` (content-stream identity, R46) —
prove pdfce agrees with *itself*. That is sufficient for **additive**
authoring (annotations, form fill, flatten-by-append): the page content
stays byte-verbatim, so a self-comparison holds. It is **not** sufficient
for content-stream **surgery** that re-renders an edited page — the vector /
Inkscape-parity editing arc (candidate A, Pass 9). A's acceptance test is
"does the edited page still render *correctly*?", and *correctly* means
"like an independent production renderer", which pdfce comparing to pdfce
structurally cannot answer.

Decision 010 therefore sequences C (this harness) **before** A, so A's
content-stream edits inherit a standing full-page fidelity gate. This
harness is the generalization of `tools/annot-pdfium-diff.py` (an ink-bbox
differential on 7 annotation fixtures) to full-page, per-channel, per-pixel
comparison over the whole loadable corpus (`fixtures/external`, ~2,914
files).

## 2. What it does

For each `*.pdf` under the corpus dir(s), for each sampled page:

1. Render in **pdfce** via `pdfce-cli render-page --page N --scale S`
   (scale = DPI/72), capturing the stdout **diagnostics tally**.
2. Render in **pdfium** via `pypdfium2` at the same scale.
3. Composite both onto white (normalizing transparency), crop to the common
   top-left extent, and compute the per-pixel **max-channel absolute delta**
   `d ∈ [0,255]`.
4. Reduce to per-page metrics: `mean`, `p95`, `dmax`, and
   `frac_over_T` = fraction of pixels with `d > T` for T ∈ {16, **32**, 64}.
5. Tag the page with pdfce's disclosed gaps (from the diagnostics tally) and
   a file-level DeviceCMYK byte-scan.
6. Classify into one of three buckets (§4).

Outputs (`out/`, deterministic + locale-invariant):

| File | Contents |
|---|---|
| `per-page.tsv` | one row per (file, page): dims, metrics, bucket, reason, gaps |
| `summary.txt` | distribution + three-bucket counts + DeviceCMYK char. + unexplained tail |
| `summary.json` | same, machine-readable (the gate/CI artifact) |
| `diffs/*.png` | `[pdfce ǀ pdfium ǀ 8×-amplified delta]` panels for the worst pages |

## 3. The tolerance band — empirical, never tuned (decision 010 Y1 / W14)

**The central problem.** Two independent renderers *always* differ at the
pixel level: anti-aliasing, font hinting, sub-pixel glyph positioning, and
image interpolation are implementation choices, not bugs. Demanding
pixel-for-pixel agreement is a category error. Separating benign noise from
real divergence **is** the analytical core of this Pass, and there are two
forbidden failure modes:

- **W14** — tuning a threshold until a number turns green; and
- declaring benign anti-aliasing noise a "bug".

**Why `frac_over_32` is the discriminator.** Benign AA/hinting noise is
confined to a *thin sub-pixel band around edges*. Individual edge pixels can
swing the full 0..255 (a glyph edge that is black in one renderer and white
in the other one pixel over), so **max-delta and mean-delta are dominated by
edge noise and are poor discriminators**. But that noise touches only a
*small fraction of the page's area*. A real divergence — a missing shading
fill, a wrong DeviceCMYK colour, a shifted or dropped glyph run — touches a
*large contiguous area*, i.e. a large fraction. So **fraction-of-area over a
moderate per-pixel threshold (32/255 ≈ 12.5%)** is the noise-robust page
metric. (Empirically confirmed: the very first band-edge "unexplained" page
was a blank sheet with a single 1px-stroked square whose *only* divergence
was AA on the four border edges — max-delta 143, but `frac_over_32` ≈ 0.0017.
See `docs/SESSION_LOG` Pass 11 and the diff panel.)

**How the band is derived (not picked).**

1. Every page is tagged **clean-by-construction** iff pdfce discloses **zero**
   gaps for it (no substituted/notdef glyphs, no deferred `sh`/BDC-EMC/Type3
   ops, no unsupported font, no image-codec shortfall, no DeviceCMYK JPEG)
   **and** the file contains no `/DeviceCMYK` **and** the page boxes agree.
   Whatever such a page diverges by *can only be renderer noise*, because
   pdfce itself claims to render it in full.
2. The band is the **p99.9 of `frac_over_32` over the clean-by-construction
   population**. Principle: that population is benign *in full*, so the band
   covers essentially all of it. The percentile is chosen to *cover the
   benign population*, **not** to hit a target unexplained count — that is
   the W14 line. Any page above a percentile of its own benign peers is,
   by construction, anomalous *relative to benign noise* and worth a look.
3. The band is a property of the **known-benign** population, so it **cannot
   be tuned to make a bug pass**: a bug lives either on a page pdfce
   discloses a gap for (bucket ii) or in the residual tail of clean pages
   above their own noise floor (bucket iii) — never below the band.

The report always prints the **distribution** (mean / p50 / p95 / p99 / max
of `frac_over_32`, separately for all / clean / DeviceCMYK pages) — never a
bare pass/fail. The band is re-derived from the data on every run and its
source string is recorded in `summary.txt`/`.json`.

## 4. The three buckets (decision 010 deliverable 3; R20 by-file-and-reason)

Each measured (file, page) is exactly one of:

| Bucket | Definition |
|---|---|
| **(i) benign-renderer-noise** | `frac_over_32 ≤ band`. AA / hinting / sub-pixel / interpolation. Characterized, **not** chased to zero (a non-goal). |
| **(ii) known-disclosed-gap** | `frac_over_32 > band` **and** pdfce disclosed a gap that explains it — cross-referenced against pdfce's **existing** `Diagnostics` tally so an already-counted gap is **subtracted, not re-reported**. Reasons: `font-unsupported` (Type3 / exotic CMap), `font-substituted` (substitute face ≠ embedded shapes), `glyph-notdef`, `deferred-op` (`sh` shading, `/OC` marked content, Type3 procs, clip modes 4–7), `image-*` (codec/feature/geometry), `devicecmyk-*` (colorimetry, §6). |
| **(iii) unexplained-divergence** | `frac_over_32 > band` **and** no disclosed gap explains it. The genuine **bug candidates** — the residual after subtracting (i) + (ii). Every one is enumerated by file + reason and either **fixed** (if cheap and clearly a pdfce render bug) or **filed as a named, counted render-gap** (R20/R27). |

Two side classifications that are **not** pdfce errors:

- **reference-divergence** *(only in `--annots` mode)* — the page carries a
  `/Widget` or a no-`/AP` annotation. pdfium needs `FPDF_FFLDraw` to draw
  widget appearances, and it **synthesizes** some no-`/AP` looks (e.g.
  `/Circle /IC` interior fill) that **R43 makes pdfce correctly refuse**
  (Pass 6.0 finding). Bucketed reference-side so pdfium's own quirks are
  never misattributed to pdfce (deliverable 5 / risk Y2). **The default run
  is content-only** (annotations off on both engines), which structurally
  removes this confounder — the vector-editing oracle cares about page
  *content*, which is exactly what an edit re-renders.
- **skipped** — pdfce could not load/render (e.g. a conformance `fail-*`
  file with a broken header/trailer/stream — legitimately out of scope, as
  in the roundtrip gate), or pdfium could not. Counted with a reason
  histogram, never silently dropped.

## 5. Known reference-divergences encoded (deliverable 5 / Y2)

pdfium quirks that must never be scored against pdfce:

- **`FPDF_FFLDraw` widgets** — a bare `page.render(draw_annots=True)` does
  **not** draw `/Widget` form-field appearances; pdfium needs its form-fill
  environment. So in `--annots` mode a widget-bearing page whose `/AS`-
  selected appearance pdfce *does* paint is a reference-side gap
  (`pdfium-fflodraw-widget`), not a pdfce error.
- **Synthesized no-`/AP` appearances** — pdfium invents a look for some
  annotations that lack an appearance stream (the `/Circle /IC` fill);
  R43 forbids pdfce from inventing one (`pdfium-synthesized-noap`).

Both are detected from pdfce's own annotation diagnostics
(`annots_widget`, `annots_no_ap`) and bucketed `reference-divergence`. The
**default content-only run avoids them entirely**, which is why it is the
primary mode.

## 6. DeviceCMYK colorimetry characterization (deliverable 7)

Decision 006 §3.7 established that pdfce's `Rgb::from_cmyk` is the naive
additive `1 − min(c+k, 1)`, whereas pdfium uses its calibrated
`AdobeCMYK_to_sRGB1` table — a real, systematic, visible divergence
affecting **all** DeviceCMYK fills/strokes (not just JPEGs; measured 37.4%
of pixels >8 delta on the corpus CMYK JPEG). This harness **characterizes
and quantifies it corpus-wide** but does **not** fix it here (that would
confound the colour change with the harness build — Y5; and decision 006
revisit-trigger 7 requires re-pinning the §3.4 polarity matrix *before* any
colour change). It is filed as the harness's **first named residual** — a
follow-up colour Pass, promotable via `pdfce-acrobat-librarian`'s already-
filed "what does Acrobat do for uncalibrated DeviceCMYK→screen" question.

The `summary` reports the `frac_over_32` distribution for **DeviceCMYK-only**
pages (DeviceCMYK present, no *other* disclosed gap) against the clean
baseline, so the colorimetry effect is isolated and sized. DeviceCMYK
presence is detected by a tooling-side **file byte-scan** for `/DeviceCMYK`
(no render-side counter is added — adding render capability is a non-goal;
observing is not applying).

## 7. Usage

```sh
cargo build --release -p pdfce-cli          # prerequisite

# default: content-only, 150 DPI, ≤4 sampled pages/file, full corpus
python tools/render-parity/render_parity.py

# bounded subset with more diff panels
python tools/render-parity/render_parity.py --max-files 200 --emit-diffs 12

# full-corpus breadth, first page of every file (fast sweep)
python tools/render-parity/render_parity.py --pages-per-file 1

# one specific page's diff panel (demo / triage)
python tools/render-parity/render_parity.py --diff "6-2-2-t01" --diff-page 1

# gate mode for a render-touching Pass
python tools/render-parity/render_parity.py --gate --max-unexplained <baseline>
```

Key options: `--dpi` (default 150), `--pages-per-file` (0 = all), `--annots`
(compare with annotations on), `--band` / `--band-pct` (band override /
percentile), `--emit-diffs N`, `--timeout` (per-page pdfce render, default
120 s), `--out DIR`.

## 8. Gate role — required on every render-touching Pass (deliverable 6; R34/R46 pattern)

This is the standing render-fidelity gate. Like `tools/roundtrip` (R34) and
`tools/content-identity` (R46) it is an **out-of-tree local corpus gate** —
it is **not** in `.github/workflows/ci.yml` because pypdfium2 is not a CI
dependency (and pdfce ships no runtime dependency on it). It **MUST be
re-run** on every Pass that touches `pdfce-render`, `pdfce-core`'s content-
stream interpretation, colour, fonts, or images — **especially the vector-
editing Pass (Pass 9)**, whose content-stream edits re-render the very pages
this harness measures.

Procedure for a render-touching Pass:

1. Run the harness over the loadable corpus at a fixed DPI.
2. Confirm the three-bucket counts against the recorded baseline
   (`out/summary.json`): the **unexplained** count must not rise without a
   named, filed reason (a new render-gap item, R20/R27), and the **band**
   derivation must be reported (never a bare pass/fail).
3. `--gate --max-unexplained <baseline>` returns non-zero if the unexplained
   count exceeds the recorded baseline — the mechanical enforcement.

The band is re-derived every run, so it tracks the current renderer; a
regression shows up as a *new* page crossing from benign/known-gap into
unexplained, enumerated by file+reason.

## 9. Dependencies, licensing, invariants

- **No new pdfce runtime dependency.** `pypdfium2` is dev/tooling only,
  invoked out-of-tree exactly like the other corpus harnesses. It does
  **not** enter pdfce's shipped dependency set or `THIRD_PARTY_LICENSES.md`.
  pdfce depends on it **at no point** — the harness shells out to the already-
  built `pdfce-cli` binary and imports pypdfium2 only in this Python script.
  (`pypdfium2` is Apache-2.0/BSD-3-Clause-licensed and bundles the
  BSD-3-Clause PDFium binary; relevant only to whoever *runs the harness*,
  never to a pdfce build or release — LEGAL §6.)
- **GUI-core separation** is untouched — this is tooling, imports nothing
  from `pdfce-gui`, and drives `pdfce-cli` (itself GUI-free) as a subprocess.
- **Determinism / locale-invariance** — files are sorted; DPI is fixed; no
  timestamps or clocks enter the report; both renderers are deterministic.

## 10. Honest scope (decision 010 non-goals — binding)

- **Measurement only.** No new render capability: Type3, `sh`, `/SMask`,
  `/OC`, and DeviceCMYK stay their own filed items — the harness *buckets*
  them, it does not implement them (beyond any cheap, clearly-a-bug fix it
  surfaces). No editing capability of any kind.
- **Benign noise is characterized, not eliminated.** Two independent
  renderers never agree pixel-for-pixel; that is not a defect to chase.
- **Tooling-only.** No GUI visual-diff surface (a natural later addition).
- **Not a "pixel-perfect" claim.** The deliverable is a measured, bucketed
  report with the residual named (R20/R27). Whether the Pass 1.1 remainder
  is reported "closed" depends on the harness genuinely running at full-page
  corpus scale — stated exactly, never overclaimed (the Pass 6.0 caveat).
