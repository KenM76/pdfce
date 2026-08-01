# 004 — Pass 1 text-rendering font strategy for `pdfce-render`

**Date:** 2026-07-30
**Status:** Decided
**Decided by:** KenAgent (`autonomous-builder`), per the
`docs/decisions/README.md` protocol
**Requested by:** `pdfce-engineer`
**Supersedes:** nothing
**Amends:** `docs/PRIOR_ART.md` §Fonts (four verdict changes);
`D:\Dev\Rag-Specialized\PDF_Spec\fonts\font__std14_afm_licensing.md`
(URW licensing correction);
`D:\Dev\Rag-Specialized\PDF_Spec\iso32000\iso32000__ref__text_pipeline.md`
(Pass 1 ladder scope)
**Does not touch:** `LEGAL.md` §1 (the license decision stays open —
see §5.4)

---

## 1. Context

`pdfce-render` renders vector paths through `tiny-skia` and ten pixel
tests are green. Text operators — the whole `BT`…`ET` family plus `Tj`,
`TJ`, `'`, `"` — are recognized-but-deferred: they are counted in
`interpret::Diagnostics` and skipped. To paint them we need glyph
**outlines**, which means a font-program parser and a source of shapes
for the fonts that carry no program.

Three sub-questions were referred:

- **(a)** which crate parses embedded font programs (§9.8/§9.9) inside
  `pdfce-render`;
- **(b)** where the glyph shapes for the standard 14 come from — those
  fonts are mandatory per §9.6.2.2 and carry no embedded program;
- **(c)** how much of the composite-font and shaping ladder Pass 1
  attempts.

### 1.1 The constraints this decision has to serve

1. **GUI-core separation** (`ARCHITECTURE.md` §3, decision 001).
   `pdfce-render` may never gain a GUI/windowing dependency. Verified
   by `cargo tree` in CI, not hoped for.
2. **wasm32 cleanliness** (decision 003 R11). The
   `wasm32-unknown-unknown` cross-check of `pdfce-core` +
   `pdfce-render` is a first-class invariant guard, ranked above the
   macOS check, because it protects the web-fork premise the crate
   split exists to serve.
3. **Platform-clean by construction** (decision 003 R10). No
   `#[cfg(target_os)]` in `pdfce-core` or `pdfce-render`, ever. This
   is what makes runtime system-font discovery *inside the renderer*
   structurally unavailable, not merely unattractive.
4. **Permissive licensing only** (`LEGAL.md` §6.1/§6.2). pdfce's own
   license is undecided (§1); a copyleft dependency is never decided
   solo, it is escalated to the user every time.
5. **Pure Rust, no C toolchain.** Consistent with the `flate2`-backend
   rule from decision 001 §6.2 and the `ring` avoidance in
   `PRIOR_ART.md`.
6. **Fuzzy, never sneaky** (`CLAUDE.md` rule 4). Every divergence from
   faithful rendering must be counted and surfaced.
7. **Determinism.** `pdfce-cli render-page` and the differential/golden
   regression tests pdfce needs for its own development require that
   the same input produce the same pixels on any machine.

### 1.2 The distinction this decision must preserve

Decision 001 §6.2 recorded `subsetter` (with `allsorts` as fallback) as
the chosen answer for later **font-embedding** Passes. That is the
**write** side: taking a font, cutting it down, and emitting it into a
PDF. This decision is the **read** side: taking a font program out of a
PDF and turning a code into a painted outline. They are different
problems, they may legitimately use different crates, and nothing here
decides the write side. §7 says what is deliberately left open.

A second distinction runs through §4.2 and must not be collapsed:
**metrics** (advance widths — how far the pen moves) are a separate
sourcing question from **shapes** (glyph outlines — what gets painted).
The spec RAG's `font__std14_afm_licensing.md` answers the metrics
question definitively (APAFML, embeddable, first-party). It does not
answer the shapes question, and its comparison table was built for
metrics — which is why its conclusions there do not transfer.

---

## 2. Options considered

### Decision A — font-program parsing crate

- **A1 — `skrifa`** (Google `fontations`). Already present in the
  workspace lock via `epaint` 0.35.
- **A2 — `ttf-parser`** (now HarfBuzz org). `PRIOR_ART.md`'s standing
  recommendation, flagged there as a watch item.
- **A3 — `allsorts`** (Yeslogic, extracted from Prince).
- **A4 — a `hayro` font crate**, if separable.
- **A5 — a combination**, e.g. `skrifa` for sfnt plus something else
  for bare CFF and Type 1.

### Decision B — standard-14 glyph shapes

- **B1 — bundle metrically-compatible free faces.** Sub-options: the
  URW/Nimbus Core-35 set, the Liberation family, the
  Arimo/Tinos/Cousine (Croscore) family, TeX Gyre, DejaVu, STIX Two,
  the Foxit set that pdfium and pdf.js ship.
- **B2 — runtime system-font discovery.** Windows always has Arial,
  Times New Roman and Courier New. Structurally requires the shell to
  do the discovering, since R10 forbids platform code in the renderer.
- **B3 — hybrid:** bundle for determinism, allow shell-supplied
  overrides.

### Decision C — shaping and composite-font scope for Pass 1

- **C1 — the RAG ladder as written:** simple fonts and standard-14
  only (steps 1–3); defer embedded TrueType, Identity-H and CFF to
  Pass 2.
- **C2 — full simple + composite Identity-H**, deferring only
  non-Identity CMaps.
- **C3 — C2 plus complex shaping** (GSUB/GPOS, bidi).

---

## 3. Evidence

Everything in this section was verified today. The crate-API claims
were checked **against the pinned source in the local cargo registry**
(`read-fonts-0.39.2`, `skrifa-0.42.1`), not against documentation —
which matters, because the single most consequential finding is a
module that the surrounding ecosystem commentary does not mention.

### 3.1 The finding that decides Decision A

**`read_fonts::ps` is a public module that parses bare PostScript
Type 1 and bare CFF blobs and interprets their charstrings to
outlines.** There is no Type 1 gap in Rust.

`skrifa` re-exports `read-fonts` wholesale — `skrifa/src/lib.rs:29`
reads `pub extern crate read_fonts as raw;` — so the entire low-level
surface is reachable from the one dependency. `read_fonts::ps`
(module doc: *"PostScript fonts."*) contains:

| Submodule | What it gives us |
|---|---|
| `ps::type1` | `Type1Font::new(&[u8])`, `.draw(gid, ppem, pen)`, `.encoding()`, `.glyph_name(gid)`, `.matrix()`, `.upem()` |
| `ps::cff` | `CffFontRef::new_cff(data, top_dict_index, upem)`, `.draw(subfont, gid, coords, ppem, pen)`, `.charset()`, `.is_cid()`, `.encoding()`, `.matrix()` |
| `ps::cff::charset` | `Charset::glyph_id(Sid)` / `.string_id(GlyphId)` — the CID↔GID mapping §9.7.4.2 needs |
| `ps::encoding` | `PredefinedEncoding::{Standard, Expert, IsoLatin1}` with `.name(code)` and `.sid(code)` |
| `ps::cs` | the shared charstring evaluator (Type 1 and Type 2) |
| `ps::agl` | the Adobe Glyph List, feature-gated (`agl`) |

The four PDF font-program cases map cleanly onto one dependency:

| PDF font program | §  | API |
|---|---|---|
| `FontFile2` — sfnt TrueType/OpenType | 9.9 | `skrifa::FontRef` → `outline_glyphs()` → `OutlineGlyphCollection::get(GlyphId)` |
| `FontFile3` `/Type1C`, `/CIDFontType0C` — bare CFF | 9.9 | `raw::ps::cff::CffFontRef::new_cff(..)` |
| `FontFile3` `/OpenType` | 9.9 | `skrifa::FontRef` |
| `FontFile` — bare Type 1, PFB or PFA | 9.9 | `raw::ps::type1::Type1Font::new(..)` |

`skrifa`'s own `outline` module cannot do the bare-blob cases —
`OutlineGlyphCollection` only constructs from a `&FontRef`, and
`skrifa::outline::cff` is a private `mod`. The route is through
`skrifa::raw::ps`, which is exactly what **hayro** does: its
`hayro-interpret/src/font/blob.rs` imports
`skrifa::raw::ps::type1::Type1Font` and
`skrifa::raw::ps::cff::{CffFontRef, Subfont, charset::Charset, v1::Cff}`.
`krilla` 0.8.2 independently uses `skrifa ^0.42` + `subsetter ^0.2.6`.
Two Rust PDF projects converged on this stack without coordination.

**One flagged unknown, resolved by reading the source.** The research
pass could not confirm from docs whether `Type1Font::new` handles PFB
segment headers, PFA hex encoding, and `eexec` decryption internally,
and correctly called it the highest-value open question. `RawDicts::new`
in `read-fonts-0.39.2/src/ps/type1.rs:402` does all three: it sniffs
the PFB text-segment tag `0x8001`, walks `0x8002` binary segments,
falls through to a PFA path that locates `eexec` with a real tokenizer
(deliberately, "to avoid catching `eexec` in a comment or string"),
hex-decodes when the following four bytes are all hex digits, decrypts
with the standard seed, and discards the four random lead bytes.
`lenIV` is honored. **The gap is closed, verified at the byte level.**

One real trap comes with it: `verify_header` requires the data to begin
with `%!PS-AdobeFont` or `%!FontType`. A `FontFile` stream that opens
with whitespace or a comment will return `InvalidFontFormat`. That is a
normalization detail, not a capability gap — recorded in §10 as an
empirical finding to file.

### 3.2 The rest of Decision A, measured

| Crate | Version | Released | License | `unsafe` | `no_std`/wasm | Direct deps | Bare Type 1 | Bare CFF |
|---|---|---|---|---|---|---|---|---|
| **skrifa** | 0.42.1 (lock) / 0.45.1 (upstream) | 2026-07-23 | `MIT OR Apache-2.0` | `#![forbid(unsafe_code)]` | yes | `read-fonts`, `bytemuck` | ✅ via `raw::ps` | ✅ via `raw::ps` |
| **read-fonts** | 0.39.2 (lock) / 0.42.1 | 2026-07-23 | `MIT OR Apache-2.0` | `#![forbid(unsafe_code)]` | yes | `font-types`, `bytemuck` | ✅ | ✅ |
| `ttf-parser` | 0.25.1 | **2024-11-29** | disputed¹ | forbids | yes | 0 | ❌ | ✅ |
| `allsorts` | 0.17.0 | 2026-05-13 | **`Apache-2.0` only** | no forbid; `libc`, `ouroboros` | **no** | **23** | ❌ | ⚠️ |
| `postscript` (bodoni) | 0.19.0 | 2025-05-14 | `Apache-2.0/MIT` | — | — | — | **structure only, no outlines** | — |
| `font` (pdf-rs) | 0.1.0 | — | **no `license` field** | — | — | **8 git deps** | claimed | claimed |
| `hayro-font` | 0.4.0 | 2026-01-08 | `Apache-2.0 OR MIT` | — | — | — | **orphaned²** | — |

¹ crates.io says `MIT OR Apache-2.0`; the repo's license field says
`Apache-2.0`. Unresolved.
² Removed from the hayro workspace; its role was absorbed in-tree and
now delegates to `skrifa`'s `ps` module.

**`ttf-parser` has decayed since `PRIOR_ART.md` recorded it.** No
release in 20 months, no commit since 2025-11-22, and the five newest
open items are all unmerged PRs — four filed 2026-07-20, one on
2026-07-22 — several security-flavored: capping COLRv1 paint-graph node
visits, capping `glyf`/`gvar` composite component visits, guarding the
CFF2 `BLEND` operator against an empty argument stack, promoting
`avar::map_value` arithmetic to `i32` to avoid `i16` overflow. Those
are exactly the classes of bug that matter for a tool whose whole
threat model is adversarial input (`ARCHITECTURE.md` §10). Nobody is
landing them. `rustybuzz` is the same org, same story (0.20.1,
2024-11-12).

`allsorts` does have real outline extraction (`allsorts::outline`,
`OutlineBuilder`/`OutlineSink`). It is disqualified on three
independent grounds: `Apache-2.0` with **no MIT arm** (narrows options
while §1 is open), 23 normal dependencies including `libc`,
`ouroboros`, `brotli-decompressor`, `encoding_rs` and
`pathfinder_geometry`, and **no `no_std`** — which puts the wasm32
invariant at risk for a capability we do not need. Its
`Type1Data`/`Type1DataOffsets` types are CFF's Type1-compat plumbing,
not a bare-Type 1 parser.

`oxidize-pdf` uses **no font-parsing crate at all** — hand-rolled
throughout. A footnote to decision 001, and one more reason the
oracle is advisory-only.

### 3.3 The API surface actually needed, confirmed present

§9.6.6.4 requires selecting a **specific** cmap subtable by
`(platform, encoding)` — `(3,1)` Microsoft Unicode, `(1,0)` Mac Roman,
`(3,0)` Microsoft Symbol with its `0xF000` page. A crate offering only
an auto-chosen "best" charmap is insufficient. `skrifa`'s own
`Charmap` is exactly that insufficient auto-chooser — but
`raw::tables::cmap::Cmap` exposes `encoding_records()` and
`subtable(index)`, giving explicit platform/encoding selection.
`raw::tables::post::Post::glyph_name(GlyphId16)` supplies §9.6.6.4's
last-resort `post` fallback.

The pen bridges to the rasterizer with no impedance:

| `read_fonts::model::pen::OutlinePen` | `tiny_skia::PathBuilder` |
|---|---|
| `move_to(x, y)` | `move_to(x, y)` |
| `line_to(x, y)` | `line_to(x, y)` |
| `quad_to(cx0, cy0, x, y)` | `quad_to(x1, y1, x, y)` |
| `curve_to(cx0, cy0, cx1, cy1, x, y)` | `cubic_to(x1, y1, x2, y2, x, y)` |
| `close()` | `close()` |

And `draw(.., ppem: Option<f32>, ..)` with `None` applies the font's
own `FontMatrix` but no ppem scale, yielding outlines in font units.
That silently resolves a trap worth naming: **CFF and Type 1 fonts may
carry a non-standard `FontMatrix`** — the 1/1000 assumption is a
default, not a guarantee — and `Type1Font::upem()` returns
`matrix.scale` so the correct divisor is always available.

### 3.4 The dependency cost is zero, and the version pin is load-bearing

`Cargo.lock` already contains `skrifa 0.42.1`, `read-fonts 0.39.2`,
`font-types 0.11.3` and `bytemuck 1.25.2`, pulled in twice over — by
`epaint` 0.35 and by `vello_common`. Adding `skrifa` to
`pdfce-render` adds **zero new packages to the lock** and **zero new
entries to `THIRD_PARTY_LICENSES.md`**. The GUI binary gains nothing at
all; the CLI binary gains the code it did not previously link.

This makes the version pin a real decision rather than a formality.
Upstream is at `skrifa 0.45.1` / `read-fonts 0.42.1` (2026-07-23). If
`pdfce-render` declared `"0.45"`, Cargo would link a **second,
semver-incompatible copy** of the whole fontations stack beside
`epaint`'s — duplicated code, duplicated attribution, two parsers with
divergent behavior in one binary. **Pin to `"0.42"`**, and add a
`cargo tree --duplicates` guard so the day `egui` moves, CI says so
instead of quietly doubling.

### 3.5 Decision B: the obvious answer is a copyleft trap, and the best answer is 16× smaller

The URW/Nimbus Core-35 set is the reflexive choice — it is what
Ghostscript ships, it is designed as metric clones of the Core 14, and
it covers Symbol and ZapfDingbats. Its actual license, read verbatim
from `ArtifexSoftware/urw-base35-fonts/LICENSE`, is:

> The font and related files in this directory are distributed under
> the GNU AFFERO GENERAL PUBLIC LICENSE Version 3 (see the file
> COPYING), with the following exemption:
>
> As a special exception, permission is granted to include these font
> programs in a Postscript or PDF file that consists of a document that
> contains text to be displayed or printed using this font, regardless
> of the conditions or license applying to the document itself.

SPDX: **`AGPL-3.0-only WITH PS-or-PDF-font-exception-20170817`**. The
exception is scoped to inclusion in a PostScript or PDF *document* and
to nothing else. Bundling the files with an application — and
especially `include_bytes!`-ing them into `pdfce.exe` — is outside it,
and pulls AGPL §13 network copyleft into scope.

Two corrections follow. First, sources describing this set as "AFPL" or
"Aladdin" are describing the pre-2017 situation and are wrong today.
Second, **`font__std14_afm_licensing.md`'s fallback table is wrong on
this point** — it records "relicensed by URW in 2015 to (a)
GPL-with-font-exception and (b) AGPL, dual." It is 2017, it is not
dual, and the exception is narrower than "font exception" suggests.
§10 dispatches the correction.

The measured field, all sizes and metrics taken from the real binaries:

| Set | License | Bytes, all 14 | Symbol | Dingbats | Bundle-safe? |
|---|---|---:|---|---|---|
| **Foxit / pdfium** | **`BSD-3-Clause`** | **264,741** | ✅ 16,729 B | ✅ 29,513 B | **yes** |
| URW Nimbus (OTF) | `AGPL-3.0 WITH PS-or-PDF-font-exception` | 1,154,500 | ✅ | ✅ | **no** |
| TeX Gyre | `LPPL-1.3c` (GUST) | 1,459,028 (12) | ❌ | ❌ | yes |
| Liberation 2.1.5 | `OFL-1.1` | 4,359,164 (12) | ❌ | ❌ | yes |
| Arimo/Tinos/Cousine | `OFL-1.1` | ~4,513,000 | ❌ | ❌ | yes |
| DejaVu 2.37 | `Bitstream-Vera` | 5,349,360 (12) | partial | partial | yes |
| STIX Two | `OFL-1.1` | 2,050,732 (5) | math only | ❌ | yes |

Advance widths against the Core-14 AFMs over the 228-glyph
WinAnsi-relevant set:

| Candidate | vs Helvetica | vs Times-Roman | vs Courier | Symbol | Dingbats |
|---|---|---|---|---|---|
| URW Nimbus | 1 differ | 1 differ | **0** | 190/190 | 202/202 |
| **Foxit** | 4 differ (Δ≤111) | 1 differ (Euro, Δ222) | **0** | **190/190** | **202/202** |
| TeX Gyre | 7 differ, 6 missing | 1 differ | **0** | — | — |
| Liberation | 5 differ (Δ≤219) | 5 differ (Δ≤167) | **0** | — | — |
| Arimo/Tinos/Cousine | 4 differ (Δ≤277) | 3 differ (Δ≤36) | **0** | — | — |
| DejaVu | **301/314 differ** | **294/314** | **313/313** | — | — |

### 3.6 The metric-mismatch question, answered with a measurement

The referral asked whether the Windows-metric families matter, "given
we position with the CORRECT sourced AFM widths and only need plausible
shapes." The reasoning is right and the answer can be measured
directly, so it was: `fontTools` against the staged Core-14 AFMs and
the real `arial.ttf`, `times.ttf`, `cour.ttf` on this machine, `hmtx`
advances normalized to a 1000-unit em.

| Comparison | printable ASCII (95 glyphs) | accent/symbol tail (200) | worst |
|---|---|---|---|
| Helvetica / Arial | **95 exact (100%)** | 188 exact (94.0%) | `macron` 333→552 |
| Times-Roman / Times New Roman | **95 exact (100%)** | 188 exact (94.0%) | `macron` 333→500 |
| Courier / Courier New | **95 exact (100%)** | **200 exact (100%)** | none |

Across the whole 295-name shared set: 283–286 exact for the
proportional families, **295/295 for Courier**. Every divergence sits
in the periphery — `macron`, `summation`, `radical`, `periodcentered`,
`divide`, `plusminus`, `mu`, and the Czech/Slovak caron composites.
**Not one letter, digit, or punctuation mark differs**, in any family.

This is an independent second measurement of the same underlying fact
the research pass found over the WinAnsi set, arrived at from different
data with different tooling. It matters because it establishes *why*
substitution is safe at all: text is positioned from the PDF's own
widths, so inter-glyph positions are exact regardless of which face
draws them; the only artifact is the difference between a glyph's own
natural advance and the slot pdfce advances by, which shows up as
sidebearing, not as displacement. Over the glyphs that carry real
text, that difference is **zero**.

It also means the decision was not forced. Liberation would have
worked. Foxit is chosen because it is better on every remaining axis,
not because the alternatives were disqualified.

### 3.7 What the reference implementations do

- **pdfium** compiles 16 Foxit CFF blobs into the binary as C arrays
  under `core/fxge/fontdata/chromefontdata/`. Every file carries
  `// Original code copyright 2014 Foxit Software Inc.` and is governed
  by pdfium's BSD-style LICENSE. Chrome has shipped them since 2014.
- **pdf.js** extracts the same blobs as `.pfb` under
  `external/standard_fonts/` — but substitutes Liberation Sans for the
  four sans faces, so it ships two licenses side by side.
- **Ghostscript** ships the URW Type 1 files under the same
  document-embedding-only AGPL exemption quoted above.
- There is **no standalone Foxit-published font package** and no
  Foxit-authored grant. The redistribution right traces entirely
  through Google's BSD-3-Clause grant in pdfium.

---

## 4. Decision

### 4.1 Decision A — **A1, `skrifa`, pinned at `0.42`**

One crate in `pdfce-render` (never in `pdfce-core`), covering all four
PDF font-program cases through `skrifa` for sfnt and `skrifa::raw::ps`
for the bare CFF and bare Type 1 blobs. No second parser.
`ttf-parser`, `allsorts`, `postscript` and `font` are rejected on the
§3.2 evidence.

All `raw::ps::*` use is confined to a single module,
`crates/pdfce-render/src/font/program.rs`, so that a break in that
lower-level surface is a one-file fix rather than a scattered one.

### 4.2 Decision B — **B3, hybrid: bundle the Foxit base-14, expose an override seam**

`pdfce-render` bundles all 14 Foxit faces as bare CFF —
**264,741 bytes, `BSD-3-Clause`** — and exposes a public API through
which the shell may supply additional or replacement faces. The
renderer itself never touches the filesystem, the environment, or the
OS font store.

Sourced from **pdfium** rather than pdf.js: one license, all 14 faces
from one origin, and exact-metric Symbol and ZapfDingbats. Extraction
from the C arrays is a one-shot tool that also records provenance.

No cargo feature flag. At 258 KiB against a 7.27 MB GUI binary — 3.5% —
the size escape hatch does not justify a `--no-default-features` CI
matrix entry.

### 4.3 Decision C — **C2, with a deliberate departure from the RAG ladder**

Pass 1 renders:

- **simple fonts** (`Type1`, `MMType1`, `TrueType`) through the full
  §9.6.6 chain — Annex D base tables, `/Differences`, the implicit-base
  rules, Branch A `(3,1)`-via-AGL and `(1,0)`-via-Mac-Roman, Branch B
  `(3,0)` with the `0xF000` page, and the `post` last resort;
- **standard-14 and every other non-embedded font**, via a bundled
  substitute chosen by `BaseFont` name and then by `FontDescriptor`
  `Flags` (Serif, FixedPitch), `ItalicAngle` and `StemV`;
- **Type 3** (§9.6.5) — no new dependency, reuses the interpreter,
  needs `FontMatrix`, `d0`/`d1`, the `Resources` fallback and a
  recursion bound;
- **composite `Identity-H`/`Identity-V`** with **`CIDFontType2`**
  (`/CIDToGIDMap` `/Identity` or stream) **and `CIDFontType0`**
  (CID→GID through the CFF charset), `/W` and `/DW` advances, and `Tw`
  correctly inert on multi-byte codes.

Pass 1 defers, **with diagnostics**: non-Identity CMaps (predefined CJK
and embedded CMap streams), vertical metrics beyond `Identity-V` code
decoding, and text clipping modes `Tr` 4–7.

Pass 1 — and every later Pass — performs **no shaping**. See §5.5.

**The departure, stated plainly.** `iso32000__ref__text_pipeline.md`
scopes Pass 1 to ladder steps 1–3 and calls steps 4–6 "the natural
Pass 2." That ranking is correct about prerequisite order and wrong
about cost, because it was written before the parsing question was
settled. With `skrifa` in, steps 4, 5 and 6 collapse to roughly "pick a
GID, hand an outline to a pen." Shipping steps 1–3 alone would produce
a viewer that renders no text in most modern PDFs — which are
subsetted and `Identity-H` — and that reads as broken rather than
staged. §10 dispatches the amendment.

---

## 5. Rationale

### 5.1 The Type 1 gap was the whole problem, and it does not exist

`PRIOR_ART.md` states: *"Type1 is the weakest link ecosystem-wide — no
fully-verified, actively-maintained pure-Rust Type1 decoder confirmed.
Budget for possible partial custom implementation."* That was the
single largest risk in this decision. Bare Type 1 `FontFile` streams
are not exotic; they are what older documents, and a great many
generated appearance streams, actually contain.

`read_fonts::ps::type1` closes it — completely, with PFB, PFA, `eexec`
and `lenIV` all handled, verified by reading the implementation rather
than trusting a claim. The budget line for a hand-rolled Type 1
charstring interpreter is released.

Everything else about Decision A follows from there, because once the
hardest case is covered by the crate that also covers the easiest, a
second parser buys nothing and costs the surface area of a second
parser.

### 5.2 Why the staleness of `ttf-parser` is disqualifying rather than merely regrettable

`ttf-parser` is a genuinely excellent piece of engineering with a nicer
cmap API than read-fonts' and a convenient `glyph_index_by_name`. If
maintenance were equal it would be a defensible choice.

It is not equal, and the specific shape of the neglect is what decides
it. Four unmerged PRs filed on one day in July 2026 — capping COLRv1
paint-graph recursion, capping composite-glyph component visits,
guarding a CFF2 stack underflow, fixing an `i16` overflow — are not
feature requests. They are hardening fixes against malformed font data,
which in pdfce's threat model arrives inside an untrusted PDF.
`ARCHITECTURE.md` §10 commits pdfce to fuzzing its own parsers; taking
a dependency whose equivalent fixes are sitting unreviewed would
undercut that on the very input class it exists to defend.

`PRIOR_ART.md` already anticipated this: *"no release since Nov 2024
(~20mo stale at verification). Not archived, not yet a red flag —
re-verify before Pass 1 font work starts."* This is that re-verify. The
answer is that it aged another eight months without a commit and grew a
queue of unlanded safety fixes. The verdict flips.

### 5.3 Why bundling beats system discovery, and why the seam exists anyway

The referral framed B2 (system-font discovery) as constrained by R10 —
no platform code in the renderer — and correctly asked for the seam
design if it were chosen. Three arguments make bundling the default
rather than a fallback:

**Determinism is a testing requirement, not a preference.** pdfce needs
golden-pixel regression tests and a differential oracle. Both are
worthless if the pixels depend on which fonts the machine happens to
have. A CI runner, a developer laptop, and a user's Windows install
would all render the same standard-14 document differently, and every
diff would need adjudicating before it could be trusted. Bundling makes
"same input, same pixels" a property of the code.

**The WASM fork has no system fonts at all.** Decision 003 R11 ranks
the wasm32 check above macOS precisely because the crate split exists
to serve the web fork. A renderer whose text output depends on OS font
enumeration is a renderer that produces different results in the fork —
which is the failure the split was designed to prevent.

**The cost is 258 KiB.** The tradeoff that would have made this
interesting — 4 MB of Liberation, or 15.7 MB of CJK — does not arise.

The seam exists anyway, for three reasons that are real. `ROADMAP.md`
already carries a live CJK/Arabic/Hebrew tofu bug whose better fix is
runtime discovery of the system CJK face; that fix belongs in the
shell, and it needs somewhere to hand the bytes to. Users will
eventually want to point pdfce at a specific font for a specific
document. And an override seam is the difference between a renderer
that can be told things and one that cannot.

### 5.4 Why the license question stays closed, and what would reopen it

`LEGAL.md` §6.2 requires stopping and asking the user before adding any
copyleft dependency, "even if pdfce's current license would technically
allow it." The URW set is `AGPL-3.0-only WITH
PS-or-PDF-font-exception-20170817`. That is not a call this protocol
may make — §1 of `LEGAL.md` reserves license decisions to Ken, and
`docs/decisions/README.md` says so explicitly.

The decision avoids the question rather than escalating it, because it
does not need to be asked. Foxit is `BSD-3-Clause`, which is already in
`about.toml`'s accept-list (via `tiny-skia`), imposes no reciprocal
obligation beyond reproducing the notice, and is compatible with every
outcome §1 could reach. **This decision therefore does not gate on the
license decision and does not constrain it.**

One thing is worth Ken's attention before first release, and it is
recorded rather than resolved: the redistribution right traces through
*Google's* BSD-3-Clause grant over *Foxit-origin* code, with no
Foxit-authored grant anywhere. Chrome has shipped it for twelve years,
which is about as good as provenance gets short of a first-party
license file — but it is a chain rather than a direct grant, and
`LEGAL.md` §1's "no publishing until §1 is settled" is the right moment
to look at it once.

### 5.5 Why the render path must never shape — and why that is not a limitation

C3 is not a deferral. It is a category error, and writing it down now
prevents a plausible future mistake.

A PDF content stream does not contain text. It contains **glyph codes
at computed positions**. The producer already ran shaping: it selected
the ligature, applied the kern, reordered the bidi run, positioned the
mark, and then emitted the resulting glyphs with explicit `TJ`
adjustments. §9.4.3's adjustment numbers *are* the baked-in kerning.
The spec RAG makes the same point about AFM kerning — "a PDF consumer
must never apply AFM kerning… applying `KPX` on top double-counts and
corrupts layout" — and the principle generalizes to every layout
feature, not just kerning.

So running HarfBuzz between a `Tj` and a painted glyph would not
improve fidelity. It would substitute our layout decisions for the
producer's, and produce output that differs from every other conforming
reader. **Correct rendering requires not shaping.**

This sharpens decision 002's two-text-stacks separation rather than
contradicting it. That decision established that pdfce eventually owns
a `harfrust` path distinct from epaint's. It does — in **text
authoring**, where pdfce is the producer and must shape, and where
`unicode-bidi` will also be needed. A third, separate place needs
reading-order logic: **text extraction**, where recovering logical
order from positioned glyphs is a real problem. Three text paths, three
different jobs. None of them is the renderer. R17 makes that
permanent so the distinction survives the session that made it.

### 5.6 Two smaller calls, recorded so they are not re-litigated

**No hinting, ever (R18).** Hinting grid-fits outlines to a specific
pixel raster. PDF rendering is resolution-independent and applies an
arbitrary `Trm × CTM` that may rotate, skew, and non-uniformly scale;
grid-fitting before that transform is meaningless, and grid-fitting
after it is not what hinting does. It would also make golden-pixel
tests depend on the raster grid. Always
`DrawSettings::unhinted(Size::unscaled(), ..)`, and let pdfce's own
matrix pipeline — which already handles `cm` pre-multiplication and
user-space stroke geometry correctly — do the transform.

**No `read-fonts` `agl` feature.** `read_fonts::ps::agl` exists and is
tempting. But `pdfce-core` already needs the Adobe Glyph List for
`ToUnicode` and `extract-text`, sourced BSD-3-Clause from the staged
RAG copy, and enabling the crate feature would put a second AGL in the
binary with no guarantee the two agree. One AGL, in `pdfce-core`, used
by both extraction and rendering.

---

## 6. What this decision produces

### 6.1 Standing rules (binding; add verbatim to `ROADMAP.md`, continuing R1–R16)

- **R17 — The render path never shapes.** No GSUB/GPOS, no bidi, no
  script itemization, no ligature substitution, no mark positioning, no
  kern application anywhere between a `Tj` and a painted glyph. PDF
  content streams carry already-positioned glyphs; shaping them
  corrupts correct output. `harfrust` may only ever enter a future
  text-**authoring** path; `unicode-bidi` only a text-**extraction**
  reading-order path. Neither may become a `pdfce-render` dependency.
- **R18 — No hinting, ever, in `pdfce-render`.** Always
  `DrawSettings::unhinted(Size::unscaled(), ..)`; outlines are taken in
  font units and transformed by pdfce's own `Trm × CTM`.
- **R19 — Rendering is font-deterministic by default.**
  `pdfce-render` never discovers, opens, or reads a font from the
  filesystem, the environment, or the OS. Its default `FontEnvironment`
  is the bundled 14 and nothing else. Same input → same pixels on every
  machine, in the CLI, and in the WASM fork. Additional faces arrive
  only through the public API, supplied by the shell.
- **R20 — Substituted glyph shapes are always disclosed.** Any glyph
  painted from a substitute face rather than the document's own
  embedded program is counted in `Diagnostics` and surfaced in the GUI
  diagnostics panel and the CLI summary.
- **R21 — One font parser in the read path.** `skrifa` (with its `raw`
  re-export of `read-fonts`) is the single font-program parser for
  `pdfce-render`. No second parser enters without a new decision
  record. Its version tracks whatever `epaint` resolves to;
  `cargo tree --duplicates` must never show two `skrifa` or
  `read-fonts` majors.
- **R22 — Bundled font provenance is verified, not asserted.** Every
  bundled face carries a recorded source URL, upstream commit,
  SHA-256, extraction method and license text, and a test asserts each
  face's advance widths against the APAFML-sourced width tables, with
  the known exceptions (`Euro`, guillemets) enumerated explicitly
  rather than tolerated silently.

### 6.2 The crate change

```toml
# crates/pdfce-render/Cargo.toml
#
# skrifa: the single font-program parser for the READ path (decision
# 004 R21). License MIT OR Apache-2.0, #![forbid(unsafe_code)], pure
# Rust, no_std-capable — wasm32-clean per decision 003 R11, and with
# no GUI/windowing surface per ARCHITECTURE.md §3.
#
# Covers all four PDF font-program cases through ONE dependency:
#   FontFile2  (sfnt)        -> skrifa::FontRef + OutlineGlyphCollection
#   FontFile3  (bare CFF)    -> skrifa::raw::ps::cff::CffFontRef
#   FontFile   (bare Type 1) -> skrifa::raw::ps::type1::Type1Font
#   explicit cmap / post     -> skrifa::raw::tables::{cmap, post}
#
# VERSION PIN IS LOAD-BEARING. skrifa 0.42.1 / read-fonts 0.39.2 /
# font-types 0.11.3 are ALREADY in Cargo.lock via epaint 0.35 and
# vello_common, so "0.42" adds ZERO packages. Upstream is at 0.45;
# declaring that would link a SECOND semver-incompatible copy of the
# whole stack beside epaint's. When egui bumps skrifa, bump this in the
# same commit — the `cargo tree --duplicates` CI guard will say so.
#
# Do NOT enable read-fonts' `agl` feature: pdfce-core owns the single
# Adobe Glyph List, sourced BSD-3-Clause (decision 004 §5.6).
skrifa = "0.42"
```

### 6.3 The API seam

```rust
// crates/pdfce-render/src/font/mod.rs

/// Shared, immutable font-program bytes.
///
/// `Arc`-backed so a face parsed once can be reused across pages and
/// across threads without copying. The renderer never *obtains* bytes —
/// it only ever receives them (R19).
#[derive(Clone)]
pub struct FontData(Arc<dyn AsRef<[u8]> + Send + Sync>);

/// Which substitute a document font falls back to when it carries no
/// embedded program: the standard-14 slot, or a descriptor-derived
/// class for everything else (§9.8.1 Table 123 `Flags`).
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
#[non_exhaustive]
pub enum FallbackKey { /* Sans/Serif/Fixed × R/B/I/BI, Symbol, Dingbats */ }

/// The set of faces available to the renderer.
///
/// `Default` == [`FontEnvironment::bundled`]: the 14 Foxit faces and
/// nothing else. This is what makes rendering deterministic (R19) and
/// what makes the WASM fork work with no shell support at all.
pub struct FontEnvironment { /* … */ }

impl FontEnvironment {
    /// The bundled standard-14 substitutes. Infallible, no I/O.
    pub fn bundled() -> Self;
    /// Replace a fallback slot with a caller-supplied face.
    pub fn insert_fallback(&mut self, key: FallbackKey, data: FontData);
    /// Offer a face by BaseFont name (e.g. a system CJK face the
    /// shell discovered), consulted before the descriptor-derived
    /// fallback.
    pub fn insert_named(&mut self, base_font: &str, data: FontData);
}

/// Per-render knobs. `#[non_exhaustive]` so later Passes can add
/// image/annotation/overprint options without a breaking change.
#[non_exhaustive]
#[derive(Default)]
pub struct RenderOptions { pub fonts: FontEnvironment, /* … */ }
```

```rust
// crates/pdfce-render/src/lib.rs
pub fn render_page_with(
    doc: &Document, page: &Page, scale: f32, opts: &RenderOptions,
) -> Result<RenderedPage, RenderError>;

/// Convenience shim over [`render_page_with`] with
/// [`RenderOptions::default`] — the bundled faces, nothing else.
pub fn render_page(
    doc: &Document, page: &Page, scale: f32,
) -> Result<RenderedPage, RenderError>;
```

`pdfce-render` performs no filesystem access, no environment reads, no
OS font enumeration, and gains no `cfg(target_os)`. R10 holds
unchanged. The shell owns discovery; the WASM fork supplies nothing and
gets the bundled set.

### 6.4 Diagnostics additions (R20)

`interpret::Diagnostics` gains, alongside the existing counters:

```rust
/// Glyphs painted as `.notdef` or not painted at all (§9.6.6.2,
/// §9.6.5 step b, §9.7.6.3 — the four fallback ladders).
pub glyphs_notdef: usize,
/// Glyphs painted from a bundled substitute rather than the
/// document's own embedded program.
pub glyphs_substituted: usize,
/// Fonts whose machinery this Pass does not implement (non-Identity
/// CMaps, vertical metrics) — the text was skipped, not approximated.
pub fonts_unsupported: usize,
/// BaseFont names that were substituted, for the diagnostics panel.
pub substituted_fonts: Vec<String>,
```

An operator must be able to tell, without reading the code, whether the
page they are looking at was drawn with the document's own glyphs.

### 6.5 The bundled asset

`crates/pdfce-render/assets/fonts/` — 14 bare-CFF faces, 264,741 bytes,
`BSD-3-Clause`, plus `PROVENANCE.md` recording source URL, upstream
commit, per-file SHA-256, byte size, extraction method, and the
verbatim pdfium LICENSE.

`cargo-about` is blind to these — they are not Cargo dependencies —
so they join APAFML and the AGL in the hand-maintained "embedded data"
supplementary section of `THIRD_PARTY_LICENSES.md` that `ROADMAP.md`'s
"Manual-attribution obligation" entry already schedules. No
`about.toml` change is needed: `BSD-3-Clause` is already accepted.

---

## 7. What this decision explicitly does NOT decide

- **The write side.** Font embedding and subsetting remain as decision
  001 §6.2 left them (`subsetter`, with `allsorts` as fallback). §9
  carries a trigger noting that this decision's evidence weakens the
  fallback, but the write-side choice is not made here.
- **`pdfce-core`'s font module.** `pdfce-core` gains std-14 metrics,
  descriptors and the Annex D encoding machinery — data, not a parser.
  Whether it ever needs `skrifa` (it would, for subsetting) is a
  write-side question.
- **`ToUnicode` and text extraction.** Ladder step 7 is cheap and
  independently valuable, and the RAG recommends pulling it forward.
  That is an extraction feature, not a rendering one, and it is the
  engineer's scoping call.
- **The CJK/Arabic/Hebrew UI tofu bug.** `ROADMAP.md` carries it
  separately. This decision builds the seam that its better fix
  (shell-side system-font discovery) will need, and does nothing more.
- **pdfce's own license.** Untouched, unconstrained. See §5.4.
- **Whether `Tr` 4–7 text clipping ships in Pass 1.** Listed as
  deferred-with-diagnostics; promoting it is a scoping call, not a
  dependency question.

---

## 8. Consequences

**Good.**

- The largest identified risk in Pass 1's font work — a possible
  hand-rolled Type 1 charstring interpreter — is eliminated outright.
- Zero new packages in `Cargo.lock`; zero new entries in
  `THIRD_PARTY_LICENSES.md`; the "158 crates, all permissive, ZERO
  copyleft" property from Pass 0 survives intact.
- 258 KiB buys complete standard-14 coverage **including Symbol and
  ZapfDingbats with byte-exact metrics** — the two fonts that every
  cheaper option would have left uncovered, and the two that AcroForm
  checkbox appearances depend on.
- Pass 1 renders text in modern subsetted `Identity-H` documents, which
  is what most real PDFs are. The viewer will look finished rather than
  staged.
- Rendering is bit-reproducible across machines, which the golden-pixel
  and differential-oracle testing pdfce needs for everything after this
  actually depends on.
- One parser, one AGL, one font stack. Small surface, small audit.

**Costs and risks, honestly stated.**

- `skrifa::raw::ps::*` is a **lower-level surface** than `skrifa`'s own
  documented API, on a 0.x crate. It may churn on a version bump. This
  is a real exposure, mitigated by confining it to one module and by
  the fact that hayro carries the same exposure — a break would be
  noticed upstream quickly.
- The version pin couples `pdfce-render` to whatever `epaint` resolves
  to. Deliberate, and guarded by CI, but it is a coupling between two
  crates that the architecture otherwise keeps apart. If `pdfce-gui`
  ever drops `egui`, the pin becomes free.
- The Foxit provenance is a chain (Google's grant over Foxit-origin
  code), not a direct grant. Solid, twelve years shipped, but worth one
  look before release.
- `Euro` and the guillemets have small width deltas against the AFM
  tables. Named in the conformance test rather than silently tolerated.
- Pass 1 grows relative to the RAG's ladder. Type 3, `CIDFontType0`,
  and the Branch A/B TrueType chain are each modest, but they are not
  free, and the honest characterization is that this decision makes
  Pass 1 bigger in exchange for making it useful.

---

## 9. Revisit triggers

1. **`epaint`/`egui` bumps `skrifa` to a new minor.** Bump
   `pdfce-render`'s pin in the same commit. The
   `cargo tree --duplicates` guard fails loudly if not.
2. **`read-fonts`' `ps` module changes shape on a version bump.** It is
   a lower-level surface with less stability guarantee than `skrifa`'s.
   Re-verify `program.rs` on every bump.
3. **A corpus measurement shows a material share of documents hitting
   the non-Identity-CMap deferral.** Schedule predefined-CMap and
   embedded-CMap-stream work, including the Adobe CMap resource
   licensing check the ladder's step 8 flags.
4. **`Type1Font::new` fails on real `FontFile` streams at a material
   rate.** Check the `%!PS-AdobeFont` / `%!FontType` header requirement
   first (§3.1) — a leading-whitespace normalizer is the likely fix.
   Do not hand-roll an interpreter before exhausting that.
5. **Ken selects AGPL-3.0 for pdfce.** URW/Nimbus becomes bundleable
   (one width delta vs Helvetica, against Foxit's four). Marginal;
   recorded so it is not re-derived.
6. **Anyone proposes subsetting the bundled faces to save space.** It
   would create a *modified font*, contradicting the reasoning already
   committed in `about.toml` ("pdfce does not create modified fonts; it
   only embeds them") and triggering renaming obligations for
   OFL/Vera-family sets. At 258 KiB there is no case for it.
7. **The text-authoring Pass begins.** That is where shaping enters, in
   a new module, under R17.
8. **The write-side embedding/subsetting Pass begins.** This decision's
   evidence — `allsorts` is Apache-2.0-only, 23 deps, `libc` +
   `ouroboros`, no `no_std` — should feed decision 001 §6.2's fallback
   choice. Not decided here.

---

## 10. Follow-up actions

**Engineering**

1. Add `skrifa = "0.42"` to `crates/pdfce-render/Cargo.toml` with the
   §6.2 comment. Verify: `cargo tree -p pdfce-render` shows no
   `egui`/`eframe`/`winit`/`wgpu`/`glow`/`rfd`; `cargo tree
   --duplicates` shows one `skrifa` and one `read-fonts`; the wasm32
   cross-check stays green; `Cargo.lock` gains zero packages.
2. Add a `cargo tree --duplicates` guard for `skrifa`/`read-fonts` to
   the CI invariant job — new, implements R21.
3. Write `tools/extract-base14/`: pull the 14 Foxit CFF C arrays from
   pdfium's `core/fxge/fontdata/chromefontdata/`, emit `.cff` files
   into `crates/pdfce-render/assets/fonts/`, write `PROVENANCE.md`.
4. Add to `.gitattributes`: `*.cff binary`, `*.ttf binary`,
   `*.otf binary`, `*.pfb binary`. The existing file covers
   `.pdf/.bin/.png/.jpg` but no font formats, and `* text=auto` would
   otherwise be free to misdetect one.
5. Add the bundled-font `BSD-3-Clause` notice to the
   `THIRD_PARTY_LICENSES.md` manual supplementary section already
   scheduled for APAFML + AGL.
6. `pdfce-core`: generate std-14 widths/descriptors and the Annex D
   encoding tables from the staged RAG sources; own
   code → glyph-name → Unicode. `pdfce-render` owns
   glyph-name/GID → outline. Keep the split — extraction needs the
   former without the latter.
7. `pdfce-render`: add `src/font/{mod,program,bundled,select}.rs`, the
   §6.3 seam, and the §6.4 diagnostics.
8. Tests: (a) advance-width conformance for all 14 bundled faces
   against the AFM tables with `Euro`/guillemet exceptions named (R22);
   (b) golden-pixel tests, one fixture per font path — Type 1
   `FontFile`, Type1C `FontFile3`, TrueType `FontFile2`, `Identity-H`
   `CIDFontType2`, `CIDFontType0`, Type 3, non-embedded std-14;
   (c) a determinism test rendering one fixture twice and comparing
   hashes (R19); (d) a Type 1 fixture exercising both PFB-tagged and
   raw-`eexec` `FontFile` layouts.

**Librarian dispatches**

9. `pdfce-spec-librarian` — correct `font__std14_afm_licensing.md`'s
   URW row: it reads "relicensed by URW in 2015 to (a)
   GPL-with-font-exception and (b) AGPL, dual." Actual, read verbatim
   from the LICENSE file: `AGPL-3.0-only WITH
   PS-or-PDF-font-exception-20170817` — 2017, not dual, and the
   exception covers document embedding only, not application bundling.
   Add the Foxit/pdfium set as the **shapes** source, kept explicitly
   distinct from that file's **widths** analysis.
10. `pdfce-spec-librarian` — amend
    `iso32000__ref__text_pipeline.md`'s Pass 1 ladder: steps 4, 5 and 6
    are no longer "the natural Pass 2" once `skrifa` is in.
11. `pdfce-librarian` — file R17–R22 in `ROADMAP.md`'s standing rules;
    add the `ARCHITECTURE.md` §12 dated entry cross-referencing this
    record.
12. `pdfce-librarian` — record in `C:\personal_rag\pdf\` that
    `read_fonts::ps::type1::Type1Font::new` handles PFB segment tags
    (`0x8001`/`0x8002`), PFA hex-encoded `eexec`, raw binary `eexec`
    and `lenIV`, **but requires the data to begin with
    `%!PS-AdobeFont` or `%!FontType`** — a `FontFile` stream opening
    with whitespace returns `InvalidFontFormat`.
13. `pdfce-librarian` — record in `D:\dev\rag\rust\` the general
    finding that `skrifa` re-exports `read-fonts` as `skrifa::raw`
    (`pub extern crate read_fonts as raw`), making
    `read_fonts::ps::{type1, cff}` reachable from the one dependency —
    the pure-Rust answer to bare Type 1 and bare CFF parsing, which
    ecosystem commentary consistently reports as a gap.

**Documentation**

14. Update `docs/PRIOR_ART.md` §Fonts: `ttf-parser` "adopt (read) —
    watch item" → **do not adopt (read path)**; `read-fonts`/`skrifa`
    "reference/alternative" → **adopt (read path)**; `postscript`
    (bodoni) "evaluate" → **reject** (no outline API); `font` (pdf-rs)
    "evaluate, flagged" → **reject** (unpublished, no `license` field,
    8 git deps); annotate `allsorts` with Apache-2.0-only + 23 deps +
    no `no_std`; **strike** the standing note "Type1 is the weakest
    link ecosystem-wide" — `read_fonts::ps::type1` resolves it. Add a
    bundled-fonts row for the Foxit set.

---

## 11. References

**Project documents**
`docs/ARCHITECTURE.md` §3 (crate split), §6 (packaging), §10
(hardening) · `docs/ROADMAP.md` Pass 1, standing rules R1–R16,
"Manual-attribution obligation" · `docs/LEGAL.md` §1, §6.1–§6.3 ·
`docs/PRIOR_ART.md` §Fonts, §Rasterization ·
`docs/decisions/001-oxidize-pdf-adopt-vs-build.md` §6.2 ·
`docs/decisions/002-i18n-timing.md` §3.2, §4 ·
`docs/decisions/003-distribution-posture.md` §6.1 R9–R16 · `about.toml`

**Spec RAG**
`iso32000__ref__text_pipeline.md` (the ladder, the four fallback
ladders, the ten traps) · `iso32000__s__9.6.md`, `__9.6.5.md`,
`__9.6.6.md`, `__9.7.md`, `__9.7.5.md`, `__9.8.md`,
`iso32000__annex__d.md` · `fonts/font__std14_afm_licensing.md`,
`font__std14_descriptors.md`, `font__std14_widths__*.md`,
`font__agl.md`

**Verified crate source** (local registry, pinned versions)
`skrifa-0.42.1/src/lib.rs:29` (`pub extern crate read_fonts as raw`),
`src/outline/mod.rs` (`OutlineGlyphFormat`, `DrawSettings::unhinted`,
`OutlineGlyphCollection::get`) · `read-fonts-0.39.2/src/ps.rs`,
`src/ps/type1.rs` (`Type1Font`, `RawDicts::new`, `verify_header`,
`decode_pfb_tag`, `find_eexec_data`), `src/ps/cff/font.rs`
(`CffFontRef`), `src/ps/cff/charset.rs`, `src/ps/encoding.rs`,
`src/ps/string.rs`, `src/tables/cmap.rs`, `src/tables/post.rs`,
`src/model/pen.rs` · `tiny-skia-path-0.11.4/src/path_builder.rs`

**Font licensing, primary sources**
pdfium LICENSE — `https://pdfium.googlesource.com/pdfium/+/refs/heads/main/LICENSE` ·
pdf.js `LICENSE_FOXIT` —
`https://raw.githubusercontent.com/mozilla/pdf.js/master/external/standard_fonts/LICENSE_FOXIT` ·
urw-base35-fonts LICENSE —
`https://raw.githubusercontent.com/ArtifexSoftware/urw-base35-fonts/master/LICENSE` ·
SPDX `PS-or-PDF-font-exception-20170817` —
`https://spdx.org/licenses/PS-or-PDF-font-exception-20170817.html` ·
GUST Font License —
`https://www.gust.org.pl/projects/e-foundry/licenses/GUST-FONT-LICENSE.txt` ·
Liberation LICENSE —
`https://raw.githubusercontent.com/liberationfonts/liberation-fonts/main/LICENSE` ·
DejaVu LICENSE —
`https://raw.githubusercontent.com/dejavu-fonts/dejavu-fonts/master/LICENSE` ·
STIX OFL — `https://raw.githubusercontent.com/stipub/stixfonts/master/OFL.txt`

**Prior art in Rust**
hayro `hayro-interpret/src/font/blob.rs` (uses `skrifa::raw::ps`) ·
krilla 0.8.2 (`skrifa ^0.42` + `subsetter ^0.2.6`) · pdfium
`core/fxge/fontdata/chromefontdata/` · pdf.js
`external/standard_fonts/`

**Measurements performed for this decision**
`fontTools` 4.61.1 against `D:\Dev\Rag-Specialized\PDF_Spec\_sources\core14_afm\`
and `C:\Windows\Fonts\{arial,arialbd,times,timesbd,timesi,cour}.ttf` —
per-glyph `hmtx` advances normalized to a 1000-unit em, compared to AFM
`WX` by glyph name (§3.6). Scripts were temporary and not retained; the
method is stated in full so the result can be reproduced.
