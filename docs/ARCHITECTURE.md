# pdfce — Architecture

This document is the logic. The Rust code is the syntax that enacts it.
Per the user's standing global rule: a competent engineer (human or LLM)
should be able to reconstruct pdfce's design from this file (plus
`ROADMAP.md`, `LEGAL.md`, the PDF-spec RAG, and the Acrobat feature-
parity RAG) without reading a line of code.

## 1. Project goal (verbatim framing from the founding conversation, 2026-07-23)

An open-source, non-monetized, full-feature-for-feature replacement for
**Adobe Acrobat Pro**. The initial application is a native desktop GUI
that does **not** rely on running a web server, a browser runtime, or
any local network listener — everything happens in one native process.
It must run from a single folder, including all of its dependencies
(no installer, no registry writes, no system-wide runtime dependency).

pdfce also ships **CLI capabilities** from the start (`pdfce-cli`, see
§3 and §7) — batch/scriptable operations (merge, split, stamp, convert,
sign, validate) invokable without opening the GUI at all. This is
addressed by the user (2026-07-23) as an explicit project requirement,
not just a developer convenience: Acrobat Pro itself has no equivalent
first-class CLI (only in-GUI Action Wizard batch sequences), so a real
CLI is a genuine parity-plus feature for anyone scripting document
workflows.

A **later fork** (not this codebase's job yet, but a design constraint
on this codebase **today**) will turn the same core logic into a web
application. Every architectural decision below is chosen to keep that
fork cheap when the time comes, without over-building for it now.

**Competitive/prior-art landscape confirmed clear** (see
`docs/PRIOR_ART.md`, researched 2026-07-23): no existing open-source
project, web or desktop, currently combines pdfce's full target
feature breadth in one native application. The closest attempts (Open
PDF Studio, KillerPDF) each have confirmed major gaps and neither uses
a native Rust PDF engine. This validates the project's premise.

### 1.1 Privacy posture (explicit, binding — not merely implicit in "no web server")

pdfce makes **no network calls of any kind by default**: no telemetry,
no usage analytics, no crash reporting, no update-check phone-home, no
license-verification callback. Every document a user opens is
processed entirely locally, in-process, with no data ever leaving the
machine unless the user explicitly initiates it themselves (e.g.
emailing a file — an action pdfce doesn't perform on their behalf
anyway). If a future feature ever genuinely needs network access (an
opt-in update checker, say), it must be **off by default and
explicitly opted into**, disclosed plainly in the UI and in
`README.md`, never silently enabled. This is a load-bearing part of
the project's value proposition ("not a web app" is a promise about
data handling, not just deployment topology) and is treated with the
same weight as the GUI-core-separation and round-trip invariants —
don't add a network call without flagging it to the user first.

**Precision clause (2026-07-30, decision 003 §3.4 — a correction of
wording, not a weakening):** pdfce makes no network requests and
contains **no HTTP client and no TLS stack** — verifiable by any
reader of the generated `THIRD_PARTY_LICENSES.md` — but the shipped
GUI binary does link the `webbrowser` crate (and its `url` parser
dependency), because eframe 0.35 hardcodes egui-winit's `links`
feature and it cannot be disabled downstream. That code opens the OS
default browser and makes no request itself; it is inert unless pdfce
emits an `OpenUrl` event. When it fires, the request belongs to the
user's browser, not to pdfce. State the posture in exactly these terms
(decision 003 §6.3's copy) — "no network code at all" would be false.
Enforcement is the fail-closed `no-network` CI job (decision 003 R12):
no HTTP/TLS/socket client crate may enter any pdfce crate without a
new decision record.

## 2. Language & toolkit decision (made 2026-07-23, by the user)

| Decision | Choice | Why |
|---|---|---|
| Systems language | **Rust** | Single self-contained native binary (no runtime to bundle), memory safety for a file-format parser that will be fed adversarial/malformed input from the public internet, first-class WASM target for the future web fork, mature crate ecosystem for compression (`flate2`, `weezl` for LZW), fonts (`ttf-parser`, `allsorts`), image codecs, and crypto (`rsa`, `aes`, `sha2`, `rustls`-adjacent primitives) that pdfce will need anyway. |
| GUI toolkit | **egui + eframe** (recommended default — see §2.1) | `eframe` is egui's application shell and already targets **both native (winit+wgpu/glow) and WASM+canvas from the same codebase** — this is the single biggest lever for making the later web fork cheap. Immediate-mode fits a tool-heavy, many-panel editor (canvas + thumbnails + inspector + toolbars) well; prior art includes rerun.io and many CAD-adjacent Rust tools built the same way. |
| Rendering backend | `wgpu` (falls back to `glow`/OpenGL if needed) | Cross-platform, matches eframe's default, no separate native-toolkit dependency to bundle. |

### 2.1a — Toolchain pin & lockfile policy (Pass 0 task)

- **Toolchain**: pin a specific stable Rust release via `rust-toolchain.toml`
  at the workspace root, created at Pass 0. Don't float on "whatever
  stable is installed" — reproducibility matters for a project other
  people will eventually build. Bump deliberately (dated decision-log
  entry), not silently.
- **MSRV** (minimum supported Rust version): not yet decided — set it
  at Pass 0 once the toolchain is pinned, document it in `Cargo.toml`'s
  `rust-version` field, and re-check it against `docs/PRIOR_ART.md`'s
  candidate dependencies (some crates there have their own MSRV floors
  that could force pdfce's own MSRV higher than expected).
- **`Cargo.lock`**: **commit it.** This is an application workspace
  (produces `pdfce-gui`/`pdfce-cli` binaries), not a pure library —
  the Rust ecosystem convention for binaries is to commit the lockfile
  for reproducible builds, unlike libraries which typically don't.
  Don't `.gitignore` it.

### 2.1 — egui vs iced: still confirm at Pass 0

The user's decision was "Rust core + native GUI (egui/iced)" — the
specific pick between the two was left to engineering judgment. This
document recommends **egui/eframe** for the WASM-parity reason above.
**pdfce-engineer**: treat this as a strong default, not yet a closed
decision — confirm it explicitly with the user at the start of Pass 0
(the first real coding session) before the workspace is scaffolded,
since reversing it later means rewriting the entire GUI crate.

## 3. Workspace layout (Cargo workspace, to be created at Pass 0)

```
D:\Dev\pdfce\
  Cargo.toml                  <- workspace root, [workspace] members below
  crates\
    pdfce-core\                <- COS object model, tokenizer, xref (table + stream),
                                   object streams, incremental-update writer, filters,
                                   fonts, color spaces, encryption/decryption, digital
                                   signature verification, content-stream interpreter
                                   (produces a display-list / draw-op stream, NOT pixels).
                                   ZERO windowing/GUI/rendering-backend dependencies.
                                   THIS is the crate that forks to WASM later.
                                   **`font_embed.rs` (Pass 21.0, FF-C, decision 021,
                                   commit `48c6b77`; body-section sync 2026-08-04
                                   continuation 77):** plain-data contract
                                   (`FontEmbedPlan`/`SubsetGlyph`/`DescriptorMetrics`/
                                   `OutlineKind`) plus `build_objects` — emits the new
                                   `/Type0`+`/CIDFontType2`+`/FontDescriptor`+
                                   `FontFile2`+`/ToUnicode` PDF objects from a plan
                                   `pdfce-render` fills in. No font-PROGRAM parser
                                   lives here — `fontdata/` stays metrics-only, even
                                   after this Pass. See §4 for the full contract and
                                   why the split runs this way.
    pdfce-render\               <- Takes pdfce-core's draw-op stream + resources
                                   (fonts, images, color spaces) and rasterizes to an
                                   in-memory pixel buffer via `tiny-skia` (CPU-only,
                                   pure Rust, no GPU/windowing context — see
                                   docs/PRIOR_ART.md, resolved 2026-07-23).
                                   Depends on pdfce-core. Still GUI-framework-agnostic
                                   (no egui/eframe dependency) — a headless render
                                   (e.g. "render page 3 to PNG") must work with zero
                                   windowing system present, which is also what makes
                                   the eventual web fork (canvas-based rendering) and
                                   any future CLI/batch tooling possible.
                                   **Implementation note (Pass 1, amended 2026-07-30):**
                                   the content-stream *interpreter* (`gstate`/
                                   `interpret` modules, incl. §8.10.1 Form-XObject
                                   execution and §8.9 image drawing) lives HERE, not
                                   in pdfce-core as this diagram's original wording
                                   ("content-stream interpreter... produces a
                                   display-list/draw-op stream") implied — pdfce-core
                                   supplies the lossless content-token model only
                                   (`content.rs`); pdfce-render walks those tokens and
                                   paints directly, with no separate draw-op IR
                                   in between. Recursive `Do` dispatch into nested
                                   Form XObjects is therefore a pdfce-render-time
                                   concern (`MAX_XOBJECT_DEPTH`, §10.1), distinct from
                                   pdfce-core's parse-time recursion guards (page-tree
                                   depth, xref/ObjStm cycles).
                                   **★ Implementation note (measured 2026-08-07,
                                   `76200e9`) — THE CLIP'S REPRESENTATION IS THIS
                                   CRATE'S COST CENTRE.** The graphics-state clip is
                                   an `Option<tiny_skia::Mask>`: a PAGE-SIZED coverage
                                   buffer, one byte per device pixel. On a
                                   129,515-path CAD sheet the clip machinery was
                                   **95% of render time** — painting every path costs
                                   **0.87 s** against an 18.04 s total — while read +
                                   parse + page tree together were **~0.005%**. Two
                                   semantics-preserving fixes landed in
                                   `interpret.rs`: a per-paint `clip.clone()` became a
                                   borrow (~108 GB of memcpy for one page, scaling
                                   with page AREA), and `intersect_clip`'s multiply is
                                   bounded to the path's device bounds — an IDENTITY,
                                   since outside them the fresh mask is zero. Output
                                   byte-identical (SHA-256). ~~**`Mask::new` alone
                                   remains 10.1 s of the remaining ~18 s**; reducing
                                   it is a REPRESENTATION change (most clips are
                                   `re W n` rectangles needing no mask at all) and was
                                   deliberately not folded into that commit.~~
                                   **★ CORRECTED 2026-08-07 (`4475fe6`) — BOTH CLAUSES
                                   IN THAT STRUCK SENTENCE WERE WRONG.** `Mask::new`
                                   is **1.02 s, not 10.1 s** (the 10.1 s came from an
                                   ablation that measured construction PLUS use — an
                                   **R164** instance), and only **612 of 24,128 clips
                                   — 2.5% — are rectangles**, so the rectangle
                                   special-case was **declined on measurement, not
                                   built**. The real 1× distribution was **`q`/`Q`
                                   gstate clone 6.80 s**, `mask.fill_path` 5.24 s,
                                   multiply 2.26 s, `Mask::new` 1.02 s. **The clone
                                   was the cost, and `4475fe6` removed it by making
                                   the clip an `Arc<Mask>`** — sound because a clip is
                                   never mutated in place (`intersect_clip` builds a
                                   FRESH mask and assigns it; the old one is only
                                   read), so `q` needs a reference, not a buffer, and
                                   no copy-on-write is required because there is no
                                   write. **`Arc` rather than `Rc` is a deliberate
                                   architectural choice: it keeps
                                   `GraphicsState: Send`**, which is what leaves
                                   off-thread page rendering reachable without a
                                   second type change; the cost is one atomic
                                   increment per `q`. Result: `q`/`Q` clone
                                   **6.80 s → 0.01 s**, 1× **17.47 → 10.18 s**, 2×
                                   **214.71 → 51.52 s**, and the 1×→2× cache cliff
                                   **14.1× → 5.1×** (reduced, not gone). Output
                                   byte-identical on the CAD sheet **and on 52
                                   synthetic fixtures** — that page has zero images
                                   and 242 text elements, so it cannot witness a
                                   regression in image sampling, glyph rasterization
                                   or annotation appearance, and a
                                   "no pixel anywhere changes" claim needs witnesses
                                   spanning the surfaces that produce pixels.
                                   ~~**What remains is still a REPRESENTATION change,
                                   but a differently-shaped one:** clips are 100%
                                   single-subpath, mean 7 segments, mean bounding box
                                   **0.663% of the page**, so the mismatch is between
                                   clip EXTENT and mask EXTENT — not between clip
                                   SHAPE and mask SHAPE.~~
                                   **★★ CORRECTED 2026-08-07 (`6b33789`) — THAT
                                   STRUCK SENTENCE IS WRONG BY 100×, AND IT IS THE
                                   SECOND WRONG FIGURE IN THIS SAME BLOCK.** Mean
                                   clip bounding box is **66.36% of the page, not
                                   0.663%** — a fraction printed as a percent. The
                                   sheet's first clips cover **87%, 65%, 100%, 81%,
                                   95%**; individual and accumulated bboxes both give
                                   66.36%, so it is not an accumulation artifact.
                                   **There is no EXTENT mismatch to exploit** — a
                                   mask sized to a 66%-of-page clip is a page-sized
                                   mask in all but name — and the follow-on
                                   optimisation this sentence was the premise for is
                                   **RETIRED** in `ROADMAP.md`'s *Next up*, not
                                   merely annotated. **Two further, independent
                                   refutations, either fatal on its own:** tiny-skia
                                   requires the clip mask and the pixmap to be the
                                   SAME SIZE and **enforces it SILENTLY** —
                                   `RasterPipelineBlitter::new` returns `None` on a
                                   mismatch (`pipeline/blitter.rs:36-44`), a
                                   `log::warn!` and a **dropped paint**, so a smaller
                                   mask produces WRONG output rather than fast output;
                                   and the saving does not exist anyway —
                                   `Mask::fill_path` costs **10.3 µs on a 64×64 mask
                                   vs 8.3 µs page-sized**, being dominated by three
                                   raster-pipeline compilations per call rather than
                                   by rasterization, while `scan::path_aa::fill_path`
                                   **already** bounds itself to `path.bounds()`.
                                   `Mask::new` at page size is **24.6 µs**, so its
                                   ~1.02 s is real and **irreducible without changing
                                   the representation**. **The clip-representation
                                   line of attack is CLOSED**; what survives of the
                                   census is SHAPE (single-subpath, 7 segments), not
                                   SIZE. The `intersect_clip` doc comment that
                                   asserted clips *"mostly cover a few percent"* was
                                   **corrected in place the same day it was written**,
                                   and **the bound it justifies remains an IDENTITY
                                   worth keeping** — it skips the ~34% outside the new
                                   path, a third of the work rather than two orders of
                                   magnitude. `clip_bbox` is a **`GraphicsState`
                                   field** rather than a thread-local for a reason
                                   found the hard way: any clip-derived quantity
                                   tracked outside the graphics state is monotonically
                                   wrong, because **`Q` reinstates a LARGER clip** and
                                   a tracker that only ever shrinks never widens on it.
                                   See §12's 2026-08-07 twenty-second entry.
                                   **Consequence for anyone optimising here: tiling
                                   and threading would today be aimed at 5% of the
                                   cost.** See §12's 2026-08-07 twentieth and
                                   twenty-first entries and `ROADMAP.md`'s two
                                   *fix — RENDER PERFORMANCE* entries.
                                   **★ AND THERE IS NOW A MEASURED FLOOR UNDER ALL
                                   OF IT (2026-08-07, `fa17d54`,
                                   `render-profile --ablate-sweep`): 0.49–0.53 s
                                   while pixels vary 64×.** The floor is
                                   **SCALE-FLAT**, therefore **per-operation** — it
                                   is the cost of walking **148,517 content-stream
                                   operators** and building their paths, and it is
                                   identical at every scale. **Pixels are
                                   essentially free here.** The complete map at 1×:
                                   interpreter floor **0.5 s** · painting
                                   **~0.8 s** · mask sampling **free, at the noise
                                   floor** · **clip construction ~8.4 s = 86%** —
                                   the last of which **reproduces the per-phase sum
                                   above (5.24 + 2.26 + 1.02 = 8.52 s) within 4% by
                                   a DIFFERENT METHOD**, which is the second
                                   measurement **R166** requires before a figure may
                                   order work. **The binding constraint on how this
                                   crate may be optimised is therefore stronger than
                                   "tiling addresses 5%": tiling and threading
                                   cannot go BELOW the floor at all**, because they
                                   render fewer pixels and not fewer operators.
                                   **A low-resolution proxy is likewise bounded
                                   below by ~2.6 s** — at 0.25× the full render is
                                   **2.57 s, not 0.67 s**, because clip construction
                                   drops only ~4× for a 16× pixel reduction. Anyone
                                   reaching for a proxy or progressive refinement as
                                   the answer to interactive speed should read that
                                   sentence first. See §12's twenty-fourth entry.
                                   **★★ AND THE 86% IS NOW BROKEN DOWN
                                   (2026-08-07, `110b8c9`, per-phase timing
                                   rather than ablation — a timer removes
                                   nothing, an ablation removes other things
                                   with it, R164). At 1× over 24,128 clips:
                                   `Mask::new` 1.03 s (42.7 µs, 11.8%) ·
                                   `fill_path` 5.22 s (216.4 µs, 59.9%) ·
                                   the multiply 2.46 s (102.0 µs, 28.3%) =
                                   8.72 s (361.2 µs per clip). Sum + floor =
                                   9.26 s against a 9.49 s render — THE
                                   ARITHMETIC CLOSES**, and the ~0.23 s
                                   residual is the corrected painting figure.
                                   **★ TWO FIGURES IN THIS BLOCK ARE
                                   CORRECTED BY THAT RUN.** (i) `fill_path`
                                   at **10.3 µs / 8.3 µs** above is what a
                                   SMALL path costs; in this workload it is
                                   **216.4 µs — wrong by ~22×**. **The
                                   conclusion that pair supported is still
                                   TRUE and still fatal to the clip-sized-mask
                                   idea:** buffer size does not drive the
                                   cost. What drives it is the PATH — **an
                                   anti-aliased scanline fill costs what the
                                   path's EDGES cost, not what the buffer
                                   costs** — and the original experiment
                                   varied the buffer while holding the path
                                   fixed, so it was right about the ratio and
                                   wrong about the magnitude. (ii) **Painting
                                   is ~0.27 s, not ~0.8 s**: the 0.81 s was
                                   the whole `clip-build`-ablated render,
                                   floor PLUS painting (**R164**, third
                                   instance that day); ablating `paint` alone
                                   moves the total 9.28 → 9.32 s, inside
                                   noise. **Consequence: tiling and threading
                                   address UNDER 3%, not 5% — the ordering is
                                   unchanged and the margin grew.**
                                   **★★ THE CONSTRAINT THAT SHAPES ANY FUTURE
                                   OPTIMISATION OF THIS CRATE: THE PER-CLIP
                                   COST DISTRIBUTION IS UNIFORM.** 85.0% of
                                   clips fall in 256–512 µs and 14.4% in
                                   512–1024 µs; **p90 and p99 are both under
                                   1024 µs**; only **108 of 24,128** exceed a
                                   millisecond and only **36** fall below
                                   256 µs — **99.85% inside a single 4× band.
                                   There is no tail and no head.** So there is
                                   **no pathological special case to find and
                                   fix**, and **anything that helps must
                                   change the work done for ALL 24,128
                                   clips.** That forecloses fast paths as a
                                   category (items 1 and 1′ in `ROADMAP.md`
                                   were both special-case proposals, both
                                   killed by a census; this is the third
                                   census and it kills the category), and it
                                   is why the live candidate is
                                   **deduplication of already-built clip
                                   masks — BLOCKED, deliberately, on a census
                                   of how many of the 24,128 are
                                   re-applications of an already-built clip
                                   path. Measure the repetition BEFORE
                                   building anything** (`R166` applied
                                   prospectively).
                                   **★★ THE CENSUS IS RUN AND THE BLOCK IS
                                   DISCHARGED 2026-08-07 (`1992d13`) — AND
                                   THE PREMISE SURVIVED, THE FIRST OF THREE
                                   THAT HAS.** **24,128 applications over 40
                                   distinct build keys = 603.20 per key;
                                   24,088 repeats = 99.83%.** **But the mean
                                   hides the shape: top-1 = 97.3%, top-2 =
                                   99.8%, and 37 of the 40 keys are applied
                                   EXACTLY ONCE**, so **a 2-entry cache
                                   serves 99.8% over ~1.9 MiB** (38.3 MiB ÷
                                   40 = 0.958 MiB per mask), not the 40-entry
                                   38.3 MiB the working set implies. **And a
                                   hit is worth the WHOLE operation, not 72%
                                   of it: a second key including the INCOMING
                                   clip returns 40 distinct (path, incoming
                                   clip) pairs — IDENTICAL to the 40 build
                                   keys — so every re-application is under
                                   the SAME incoming clip, the FINAL mask is
                                   identical, and a hit can SHARE THE
                                   EXISTING `Arc`: 361 µs/clip (8.72 s ÷
                                   24,128), not the 259 µs (6.25 s ÷ 24,128)
                                   a build-only cache would save.** `q`/`Q`
                                   was checked and does **not** already solve
                                   it — restore is free since `4475fe6`, but
                                   **every `W`/`W*` calls `intersect_clip`
                                   regardless**. **Both identity choices
                                   UNDERSTATE repetition by construction**
                                   (bit-exact coordinates; `Arc`-pointer
                                   incoming identity), so **99.83% and 40 are
                                   LOWER BOUNDS**. ⚠ **The ~10 s → ~1.7 s
                                   projection is ARITHMETIC over separately
                                   measured parts (99.83% × 345.6 µs mean =
                                   8.34 s over 24,128 removed, against 1×
                                   totals of 9.28–10.18 s), one instrument
                                   each, and is EXPLICITLY UNVERIFIED — no
                                   cache exists and nothing has been
                                   re-rendered** (`R166` still governs).
                                   **★★ AND THE SCALING LAW EXPLAINS WHY
                                   FEWER PIXELS BUY SO LITTLE.** Per 4×
                                   pixels: `Mask::new` 4.3×, 7.9×
                                   (**superlinear**) · `fill_path` 1.98×,
                                   2.11× (**~2× — it tracks the LINEAR
                                   dimension**) · the multiply 4.0×, 4.4×
                                   (**area-bound**). **The scanline
                                   converter's cost follows the path's
                                   PERIMETER and the number of scanlines it
                                   spans, not the buffer it writes into** —
                                   which is why `fill_path` dominates at every
                                   scale and is **still 56% of the entire
                                   render at 0.25×**. **This is the MEASURED
                                   mechanism behind the "proxies underdeliver"
                                   claim above, which until now rested on a
                                   total rather than on a law.** ⚠ One figure
                                   is **UNRECONCILED**: 56% at 0.25× with
                                   `fill_path` = 1.25 s implies a 0.25× total
                                   of ~2.23 s, against the 2.57 s recorded
                                   above — **13% apart, outside this machine's
                                   5.8% spread, and no denominator was
                                   stated.** Neither is retired here; the
                                   qualitative conclusion (both are ~3.5×
                                   above the 0.67 s naive pixel scaling
                                   predicts) holds either way.
                                   See §12's twenty-fifth entry.
                                   **★★ THE CACHE IS BUILT, AND THIS
                                   CRATE IS NOW FLOOR-BOUND (2026-08-07,
                                   `ce57ed5`, **`Pass 45.0`**).**
                                   `crates/pdfce-render/src/clip_cache.rs`
                                   (414 lines) caches the mask **AFTER**
                                   intersection, keyed on the build inputs
                                   **plus which clip it is intersected
                                   with**, bounded to **4 entries, LRU**,
                                   and **owned by the `Interpreter`** so it
                                   dies with the content stream —
                                   deliberately **not global and not
                                   `thread_local`**, because rendering moved
                                   to a worker in **Pass 44.0** and masks are
                                   keyed partly on device size.
                                   **Result, two instruments, both filed:**
                                   engineer end-to-end **1× 32,313 →
                                   907 ms (35.63×)** and **2× 447,862
                                   → 1,425 ms (314.3×)**; the
                                   `render-profile` harness, render phase
                                   only, **1× 10.68 → 0.79 s
                                   (13.52×)** and **2× 58.52 →
                                   1.30 s (45.02×)**. **The first pair is
                                   DAY-CUMULATIVE over three fixes; the second
                                   is THIS COMMIT ALONE**, and they differ by
                                   a near-constant **117 ms at 1× / 125 ms
                                   at 2×** of process start and PNG encode.
                                   **Output BYTE-IDENTICAL — SHA-256
                                   `9250a89f…`, the SAME hash as the
                                   32.3 s render, plus an unchanged aggregate
                                   over 115 synthetic fixtures.**
                                   **★★ THE ABA HAZARD IS THE
                                   LOAD-BEARING DESIGN DETAIL, not the
                                   speed-up:** incoming-clip identity is
                                   **pointer identity**, which can lose hits
                                   and cannot invent one — but a **bare**
                                   pointer would be **unsound**, since a
                                   dropped mask's address can be reused and a
                                   stale entry would then match a pointer that
                                   means something else, **returning the wrong
                                   clip and painting a silently wrong
                                   picture**. **Each entry holds a strong
                                   `Arc` to the incoming mask**, pinning the
                                   address for as long as the entry can match.
                                   **No timing would have shown that failure
                                   — a wrong-mask hit is FASTER.**
                                   **Measured hit rate 24,087 + 41 = 24,128 =
                                   99.83%, EXACTLY the census ceiling**, the
                                   41st build being the single eviction 4
                                   slots make over 40 distinct keys; residual
                                   clip cost **41 × 362 µs = 14.8 ms
                                   = 1.9% of the render**, down from
                                   **8.72 s = 86%**.
                                   **★★ SO THE BINDING CONSTRAINT ON
                                   THIS CRATE CHANGES: the floor is now the
                                   COST.** 0.49–0.53 s against a 0.79 s
                                   render — **62–67% of what is left**,
                                   **maximum further speed-up at 1× =
                                   0.79 ÷ 0.51 = 1.55×**, and the only
                                   remaining target is the **operator walk**
                                   (148,517 operators = **3.43 µs each**).
                                   **★ AND THE ~1.7 s PROJECTION ABOVE IS
                                   DISCHARGED BY MEASUREMENT, WHICH ALSO
                                   ADJUDICATED AN EARLIER CORRECTION:** floor
                                   0.51 + painting **0.27** = **0.78 s**
                                   against **0.79 s** measured (**1.3% apart,
                                   CONFIRMED**), while floor 0.51 + painting
                                   **0.87** = **1.38 s** (**75% high,
                                   REFUTED**) — so the **`R164` painting
                                   correction, made on reasoning alone and
                                   never independently measured, is now
                                   confirmed**, and the projection was
                                   conservative by 2.2× because it rested
                                   on the uncorrected residual.
                                   See §12's twenty-sixth entry.
                                   **`font\subset.rs` (Pass 21.0, FF-C, decision 021,
                                   commit `48c6b77`; body-section sync 2026-08-04
                                   continuation 77):** `plan_subset` parses an
                                   operator-supplied donor face via the existing
                                   skrifa parser (no second font-program parser — R21
                                   unchanged), checks OpenType `OS/2 fsType` embedding
                                   permission (R109) BEFORE calling `subsetter::subset`
                                   (`subsetter` strips `OS/2`), and produces a
                                   plain-data `FontEmbedPlan` for `pdfce-core::
                                   font_embed` to emit. `SubsetError` (R27-shaped,
                                   one variant per distinct cause) and
                                   `MAX_DONOR_BYTES` (64 MiB, a judgement call, not a
                                   corpus measurement — the census that measured real
                                   embedded font programs cannot apply to an
                                   operator-supplied donor face; see the constant's
                                   own doc comment) live here. P0 floor: `glyf`
                                   (TrueType-outline) donors only — CFF donors are
                                   refused by name (decision 021 §10, C-3).
    pdfce-gui\                  <- The native desktop shell. egui/eframe application,
                                   window chrome, file dialogs (rfd crate), menus,
                                   docking layout (egui_dock or hand-rolled), the
                                   `fn main()` entry point and packaged executable.
                                   Depends on pdfce-core + pdfce-render.
                                   **★ THREADING LIVES HERE, AND ONLY HERE
                                   (Pass 44.0, 2026-08-07, `7926a78`).**
                                   `render_worker.rs` (503 lines) owns the
                                   background rasterization thread, the
                                   channel, the `RenderCancel` token and the
                                   generation counter that discards a
                                   superseded result. **`pdfce-core` and
                                   `pdfce-render` remain thread-AGNOSTIC** —
                                   they gained only the PROPERTY that makes
                                   this legal (`ObjectGraph: Send + Sync`) and
                                   the MECHANISM it needs (`RenderCancel`, a
                                   plain `Arc<AtomicBool>`); neither spawns a
                                   thread, owns a runtime, or knows a worker
                                   exists. **That split is deliberate and is
                                   what keeps the wasm fork a shell swap:** the
                                   web target has no `std::thread`, so a core
                                   crate that spawned one would not compile
                                   there, while a core crate that is merely
                                   `Send + Sync` compiles unchanged and lets
                                   the web shell reach for a Web Worker
                                   instead. **The session is
                                   `Arc<EditSession>`** because a worker must
                                   outlive the call that started it and
                                   `DocumentView` borrows its graph; every
                                   mutation passes through
                                   `OpenDoc::session_mut`, which cancels the
                                   in-flight render and JOINS the thread before
                                   handing out `&mut`, making `Arc::get_mut`
                                   infallible by construction. **The one place
                                   the UI thread blocks on rendering is a
                                   12 ms bounded wait in `spawn`** — 72% of one
                                   16.7 ms frame at 60 Hz — so a page that
                                   rasterizes in milliseconds returns inline and
                                   never touches the asynchronous path.
    pdfce-cli\                  <- The command-line batch shell. Subcommand parsing
                                   (clap crate), one subcommand per batch operation
                                   (merge/split/rotate/extract, Bates stamp, convert
                                   to PDF/A, sign, validate PDF/A or PDF/UA conformance
                                   and print a report, render-page-to-PNG for scripted
                                   thumbnailing). `fn main()` entry point, packaged as
                                   its own executable alongside pdfce-gui in the same
                                   single-folder distribution. Depends on pdfce-core +
                                   pdfce-render, same as pdfce-gui — ZERO GUI/windowing
                                   dependencies of its own, see §7. Doubles as a fast,
                                   windowless way to exercise pdfce-core in tests.
    pdfce-web\ (future, not     <- The web fork. Same pdfce-core + pdfce-render,
      built in this phase)         compiled to wasm32-unknown-unknown, eframe's
                                   web target, served as static files (no server-side
                                   PDF processing — everything still runs in-browser,
                                   preserving the "not a web app in spirit" privacy
                                   posture even in the fork).
  docs\                        <- This file, ROADMAP.md, LEGAL.md, SESSION_LOG.md
  .claude\agents\               <- pdfce-engineer, pdfce-librarian,
                                   pdfce-spec-librarian, pdfce-ui-specialist
  tests\                       <- Integration tests: parse→render→compare fixture PDFs
  fixtures\                    <- ONLY synthetic or clearly-licensed-for-redistribution
                                   test PDFs (see LEGAL.md §Test corpus sourcing).
                                   Never a scanned/downloaded real-world PDF of unknown
                                   provenance.
```

**Invariant (do not violate):** `pdfce-core` and `pdfce-render` must
compile with zero GUI/windowing crates in their dependency tree. This
is checked, not just hoped for — `cargo tree -p pdfce-core` and
`cargo tree -p pdfce-render` should never show `egui`, `eframe`,
`winit`, `wgpu` (a headless CPU rasterizer like `tiny_skia` in
`pdfce-render` is fine; a *windowing* dependency is not). This is the
single invariant that keeps the future web fork a "swap the shell
crate" job instead of a rewrite.

## 4. Core data model (target contract — implemented incrementally per ROADMAP)

This is what `pdfce-core` will expose once Pass 1+ lands. Written now
as the target so early implementation work has a north star; update
this section the moment the real API diverges (the doc is the logic —
if code and doc disagree, that's a bug in one of them, fix it same-day).

**Current state as of Pass 0 (2026-07-23):** `pdfce-core` exposes ONLY
the header-probe surface — `PdfVersion { major, minor }`, `PdfError`
(`thiserror`, `#[non_exhaustive]`), `probe_header(&[u8])`,
`probe_file(&Path)`, and the `HEADER_SCAN_WINDOW` const (1024, the
byte window the `%PDF-` marker is scanned within). None of the
`Document`/`Object`/`Page`/`StreamData` model below exists yet; it is
the Pass 1+ target. The user deliberately kept Pass 0's core this thin
to defer the from-scratch-vs-`oxidize-pdf` foundation decision (§12
entry (b), 2026-07-23) — do not treat the contract below as implemented.

**Forward pointer (2026-07-30):** that decision is now CLOSED — build
from scratch (§12 entry 2026-07-30,
`docs/decisions/001-oxidize-pdf-adopt-vs-build.md`) — and it binds six
Pass-1 obligations on this model: `ByteSpan` provenance, a lossless
content-stream token model, the ONE-object-model invariant, fail-clean
filter contract, unwrap-deny lints, and no output fingerprint. The
engineer integrates the full design text here at Pass 1.

- `Document` — owns the COS object graph, trailer, xref, and the
  original byte buffer (for lazy/unmodified-object passthrough on
  write — see the round-trip invariant below).
- `ObjectId(u32 /* number */, u16 /* generation */)`
- `Object` — enum: `Null | Bool | Integer | Real | String(Str) | Name |
  Array(Vec<Object>) | Dict(Dictionary) | Stream(Dictionary, StreamData) |
  Reference(ObjectId)`
- `StreamData` — lazy: holds the raw (still-encoded) bytes plus the
  filter chain; decoding happens on demand and is cached, never eager
  for every stream in a large document.
- `Page` — resolved view over a page dictionary: `MediaBox`, `Resources`,
  content stream(s) concatenated, inherited attributes resolved per
  §7.7.3.4 of the spec (page tree attribute inheritance).
- `Document::open(path) -> Result<Document, PdfError>`
- `Document::save(path) -> Result<(), PdfError>` — full rewrite.
- `Document::save_incremental(path) -> Result<(), PdfError>` — **the
  default save mode.** Appends a new xref section + updated/new objects
  only; every object pdfce did not touch is left byte-identical in the
  file. This is not an optimization, it's a correctness requirement:
  Acrobat's own digital-signature model depends on incremental updates
  (a signature covers a byte range; anything after that range is a
  later revision). pdfce must support this from day one, not bolt it on
  after signatures are implemented.
- `Document::render_page(index, dpi) -> Pixmap` (in `pdfce-render`,
  takes a `&Document`).

**IMPLEMENTED (2026-08-02, Pass 17.0, commit `3a56b55` — was a forward
pointer, now current reality):** `render_page`/`render_page_with` stay
`&Document`-taking thin wrappers (unchanged signatures), but
`pdfce-render`'s real internal surface is generalized to accept
`&pdfce_core::view::DocumentView` (a promoted, top-level home for the
former `pageops::assemble::DocumentView`) so it can render either a
plain `Document` or a live `EditSession` overlay — this is what the
canvas now actually renders (`self.session.view()`, not
`self.session.document()`). Full design, including the `StreamSource`
byte-source abstraction (`Contiguous | Split { base, staged }`) and the
two implementation deviations found while building it
(`image_codec::decode_image` generalization; `DocumentView::bytes()` is
`Option<&[u8]>`, not `&[u8]`): §12 entries "2026-08-02 — Decision 018"
and its same-day continuation-56 follow-up, below.

**IMPLEMENTED (2026-08-03, Pass 18.5, commit `9998a6b`):** the vector
object model (`pdfce_core::vector`, introduced incrementally from
decision 011/Pass 9a onward, not otherwise itemized in this section)
gains two hit-testing/content-detail additions. **Invariant:**
`hit_test_point` is defined as the structural head
(`hits_front_to_back(..).next()`) of the new
`hit_test_point_all -> Vec<HitResult>` (`hits_front_to_back(..).collect()`)
— the two cannot disagree because there is one private iterator
underneath both; see §12's continuation-60 entry for the full rationale
and the cross-project generalization filed to
`D:\dev\rag\rust\define_singular_query_as_head_of_plural_query.md`.
**`TextObject`** gains `preview: TextPreview` (sourced-text-only, no
derived spacing; four-variant enum, not `Option<String>`) and
`font: Option<TextFont>` (`size` is the literal `Tf` operand, not the
rendered glyph size) via a new `FontResolver` seam
(`NoFonts`/`DocumentFonts`, zero GUI dependency, `decompose(...)`'s
public signature unchanged). **`ImageObject`** gains `pixel_size`. Full
build record: `ROADMAP.md`'s Pass 18.5 Shipped entry.

**IMPLEMENTED (2026-08-03, Pass 18.6, commit `1b38e34`):** `TextObject`'s
bounding box is no longer the pen-start point of each show operator
inflated symmetrically by the largest `Tf` size in the run (a square
centred on the run's start, for the common single-`Tj` case). It is now
the summed §9.4.4 advance widths across the run for the horizontal
extent, and the resolved font's ascent/descent for the vertical extent,
computed via a new `TextBoundsBasis` four-variant enum
(`FontMetrics | MetricAdvancesNominalHeight | EstimatedAdvances |
EmBox`) — deliberately four bases, not the two the originating ui-spec
(§E) asked for, because a Type 3 or descriptor-less CIDFont has real
advances but only a guessed height, and a non-standard-14 font with no
`/Widths` has estimated advances; collapsing either into `FontMetrics`
would silently misrepresent the confidence of the box. `EmBox` is the
prior (pre-Pass-18.6) geometry, kept as the guaranteed fallback for
`NoFonts`/unresolvable-font/non-finite-`Tf`-size objects — never
silently upgraded to a basis the data doesn't support. New `Vertical`
ascent/descent resolver, a four-rung fallback ladder: `/Ascent`+
`/Descent` (§9.8 Table 122) → `/FontBBox` `ury`/`lly` (§7.9.5) →
compiled-in standard-14 descriptor metrics (§9.6.2.2) → nominal 1.0/
−0.25 em, flagged. Composite (Type 0) fonts resolve through the
**descendant** font's descriptor (§9.8.1 — a Type 0 dict itself never
carries one); Type 3 always takes the nominal rung (its own descriptor
numbers, when present, live in `/FontMatrix` glyph space, not text
space). **Invariant:** `advance_tx(w0, tfs, tc, tw, th)` is now the ONE
implementation of §9.4.4's displacement formula, shared verbatim by
`text_extract::page::show_code`, `redact::glyph`, and this bbox
computation — a fourth call site was about to become a third
independent implementation before this consolidation. Two latent
decompose-walk bugs, invisible under the prior ±1-em-inflated geometry,
were found and fixed in the same Pass: `'`/`"` did not perform their
`T*` line move (§9.4.3 Table 109), and `Tc`/`Tw`/`Tz`/`Ts` were not
tracked in the decomposer's `GState` at all. Zero new Cargo
dependencies — reuses `text_extract::font::ExtractFont`'s existing
dictionary-only resolver (rule R21: no glyph-shaping crate in
`pdfce-core`, a hit-test runs per click). Full build record:
`ROADMAP.md`'s Pass 18.6 Shipped entry (top of Shipped).

**IMPLEMENTED (2026-08-03, Pass 21.0, FF-C, decision 021, commit
`48c6b77`; §3/§4 body-section sync filed 2026-08-04, continuation 77 —
flagged owed at ship, discharged here):** `pdfce-core` gains
`font_embed.rs` — the FIRST pdfce-core surface that emits a *new*,
operator-supplied font program into a PDF, distinct from every prior
font-touching module which only ever READ existing font resources.
Public: `FontEmbedPlan` (plain-data contract: `SubsetGlyph`,
`DescriptorMetrics`, `OutlineKind` — `TrueType` emittable at P0,
`Cff` refused by name, decision 021 §10 C-3), `build_objects(&plan) ->
Result<EmbeddedFontObjects, FontEmbedError>` (allocates a `/Type0`
font dict + `/CIDFontType2` descendant + `/FontDescriptor` +
`FontFile2` stream + `/ToUnicode` CMap — always `Identity-H`, forced
independently by both `subsetter` stripping `cmap` and ISO 32000-1
§9.9's `shall`). **Round-trip: R107 — this module only ever
ALLOCATES fresh object ids, never rewrites an existing `/FontFile*`/
`/FontDescriptor`/`/Font` dict**, so FF-C needs no new §5 exception;
incremental save stays the default.

`pdfce-render` gains `font::subset` — `plan_subset(donor_bytes, ...)
-> Result<FontEmbedPlan, SubsetError>`, parsing the donor via the
existing skrifa parser (no second font-program parser added anywhere
— R21 unchanged) and calling `subsetter::subset`. Reads the donor's
`OS/2 fsType` BEFORE subsetting, since `subsetter` strips `OS/2`
(R109: `SubsettingNotPermitted` on bit 8, `EmbeddingNotPermitted` on
bit 9, both correctly inert on `OS/2` v0/v1). `MAX_DONOR_BYTES` = 64
MiB, a judgement call rather than a corpus measurement (`ARCHITECTURE.md`
§10.1 wants a bound on attacker-influenced bytes; the project's own
`tools/fontfile-census` measured EXISTING embedded font programs, which
ISO 32000-1 §9.9 forbids reusing as an FF-C donor — so that census
cannot justify this constant, and the number is stated as argued, not
measured; see the constant's own doc comment).

**Why the split runs core/render rather than living entirely in one
crate (decision 021 §3.2):** subsetting is a *write* concern, so
`pdfce-core` looks like its natural home — but producing a subset
first requires *parsing* the donor (coverage from `cmap`, advances
from `hmtx`, descriptor metrics, the `fsType` bits), and that parser
already exists in `pdfce-render`. Putting `subsetter` in `pdfce-core`
would give a crate with no font-program parser two of them, purely to
avoid a plain-data seam. So the seam **is** the design: `pdfce-render`
parses and subsets, `pdfce-core` emits the PDF objects, and
`pdfce-core` gains **zero** new dependencies from this Pass.
`pdfce-core` still has no font-program parser after Pass 21.0 —
`fontdata/` (§4, standard-14 metrics) remains compiled-in metrics
only, unchanged.

**Composite-run editability is explicitly NOT part of this contract.**
Pass 21.0 can only ADD composite text; R110 (Standing rules) governs
whether an already-present composite run can be EDITED, and as of this
entry `ShowSlot::code` (§4's `Page`/text-decode model) is still `u8`
and cannot hold a multi-byte CID — composite runs Pass 21.0 adds are
locatable and correctly refused (R-INV-4), not yet rewritable. Do not
read this §4 entry as FF-C being complete; see `ROADMAP.md`'s Pass
21.1 In-progress entry.

> **[SUPERSEDED 2026-08-05 by §4.1(F) below — read the two together.**
> `ShowSlot::code` has been **`u32` since Pass 21.1**, and **Pass 29.0
> (`a104536`) lifted the blanket composite refusal entirely**. The
> paragraph above is retained because it states *why* the narrowing
> existed, and because §4.1(F) records that the reason had become
> **self-justifying** — the refusal was cited as the ground for keeping
> the types single-byte, and the single-byte types were cited as the
> ground for keeping the refusal. **R-INV-4 still exists and still
> fires**, but only on the two font properties no amount of pdfce work
> can fix. **]**

---

## 4.1 §4 SYNC — the ACTUAL `pdfce-core` surface, read from the crate on 2026-08-05

**Why this subsection exists, and what it supersedes.** Everything above
in §4 is a chain of dated *target* and *IMPLEMENTED* blocks, last extended
at **Pass 21.0 (2026-08-03)**. Between then and 2026-08-05 the crate
shipped Passes **25.0–25.6, 26.0–26.1, 27.0–27.2, 28.0, 29.0, 30.0,
30.1**, decisions **026** and **027**, and the fix `82228b1` — and **two
of those changed contracts §4 had already documented**, so §4 was not
merely incomplete, it was **wrong as written** for anyone reading it as
the current API.

**This subsection was produced by reading the `pub` items in
`crates/pdfce-core/src/`, not by reconstructing them from `ROADMAP.md`.**
That distinction is the method, and it is deliberate: **the roadmap
records intent; the crate records truth**, and where they disagree the
crate wins. Where a statement below could not be verified in the source
it is marked **UNVERIFIED** rather than asserted or quietly dropped.

**How to read the two halves of §4.** Everything above §4.1 is the
**audit trail** — what was targeted, when, and why. §4.1 is the **living
truth**. When they conflict, §4.1 governs, and every conflict this sync
found is named below rather than papered over.

---

### (A) ★★ BREAKING — the ORIGINAL §4 bullet list is a Pass-0 TARGET, and the shipped names differ

The bullets near the head of §4 (`ObjectId`, `Object::Bool`, `StreamData`,
`Document::open`, `Document::save`) were written on **2026-07-23 as a
north star before any of it existed**. They were never renamed as the code
landed. **A reader treating them as the API will not compile.** The
correspondence, verified in `object.rs` / `document.rs`:

| §4 target bullet (2026-07-23) | What actually ships (2026-08-05) | Where |
|---|---|---|
| `ObjectId(u32, u16)` — tuple struct | **`ObjId { num: u32, generation: u16 }`** — named fields | `object.rs:62` |
| `Object::Bool` | **`Object::Boolean(bool)`** | `object.rs:260` |
| `Object::String(Str)` | **`Object::String(Vec<u8>)`** — raw bytes, escapes/hex already applied; §7.9.2 text interpretation is a later layer | `object.rs` |
| `Object::Dict(Dictionary)` | **`Object::Dict(Dict)`** | `object.rs` |
| `Object::Stream(Dictionary, StreamData)` — two fields | **`Object::Stream(Stream)`** — one aggregate | `object.rs` |
| `Object::Reference(ObjectId)` | **`Object::Reference(ObjId)`** | `object.rs` |
| **`StreamData`** — the named lazy-decode type | **does not exist under that name.** The lazy/cached decode contract it described is real and honoured; it is not carried by a type called `StreamData` | — |
| `Document::open(path)` | **`Document::load(&Path) -> Result<Self, DocError>`** and **`Document::from_bytes(Vec<u8>) -> Result<Self, DocError>`** | `document.rs:251`, `:261` |
| `Document::save(path)` — full rewrite | **`Document::save_full(..)`** | `document.rs:770` |
| `Document::save_incremental(path)` | **`Document::save_incremental(..)`** — name survives; **still the default save mode**, per §5 | `document.rs:739` |
| `PdfError` | **`DocError`** on the document surface. **UNVERIFIED whether `PdfError` still exists elsewhere as the Pass-0 header-probe error** — not re-derived in this sync, and not asserted either way | — |

**The `Page` bullet is substantially right and its shape is confirmed:**
`page_tree::Page` carries `id: ObjId` plus resolved inherited attributes
(§7.7.3.4). **Not field-by-field audited in this sync** — flagged as
partially verified rather than claimed complete.

> **★ The lesson worth keeping, because it is the general case, not this
> file's accident: a section written as a TARGET and never re-labelled
> becomes indistinguishable from a section written as a RECORD.** Both
> read as statements of fact to someone who arrives after the fact. The
> five *IMPLEMENTED (date, Pass, commit)* blocks that follow the bullets
> are dated and therefore self-locating; the **bullets are not**, which
> is exactly why they drifted for six weeks without anyone noticing. Date
> and label every contract statement, or it will be read as current.

---

### (B) ★★ BREAKING — `Subpath` gained two fields (Pass 28.0, `d8b9735`)

**This is a data-model change, so §4's prior description of `Subpath` is
wrong, not merely short.** Verified in `vector/decompose.rs:224`:

```rust
pub struct Subpath {
    pub start: Point,
    pub segments: Vec<Segment>,
    pub closed: bool,
    pub tokens: TokenRange,          // NEW — Pass 28.0
    pub starts_implicitly: bool,     // NEW — Pass 28.0
}
```

- **`tokens: TokenRange`** — the content-token range of the operators
  that construct this subpath: its opening `m`/`re` through its last
  segment, including a closing `h`. `TokenRange { start: usize, end:
  usize }` with `as_range() -> std::ops::Range<usize>`
  (`decompose.rs:323`).

  **Why it is the load-bearing addition.** Without it, per-subpath
  **editing is not expressible**: an editor had to re-walk the operator
  bytes and *hope* its walk agreed with the decomposer's about how many
  subpaths there are. `plan_delete_subpath` shipped exactly that, with a
  count guard (`SubpathStructureMismatch`) that refused the **whole
  object** whenever the two walks disagreed. Recording the range on the
  walk that already knows it makes the agreement **structural instead of
  checked** — the same shape as R92 (two derivations of one fact are two
  definitions of it).

- **`starts_implicitly: bool`** — true when the subpath was opened by a
  segment operator **after `h`**, which reopens at the closed subpath's
  start point (ISO 32000-1 §8.5.2.1) **with no operator of its own saying
  where**.

  **Why it must be RECORDED rather than inferred at edit time.** Such a
  subpath's start is **inherited and carried by no operand**, with two
  consequences that are otherwise silent: (1) it **cannot be moved** —
  there is no coordinate pair to rewrite; (2) **the subpath BEFORE it
  cannot be deleted** — excising those operators changes the current point
  the implicit one starts from, so *a byte-minimal edit that passes every
  round-trip check still moves a line the operator never touched*. That
  second case is the `DeleteWouldMoveNextSubpath` refusal.

**Migration note for a reader of an older checkout:** a `Subpath` struct
literal written before Pass 28.0 will not compile. **UNVERIFIED whether
`Subpath` is `#[non_exhaustive]`** — not checked in this sync; assume it
is not and treat the addition as breaking.

---

### (C) ★★ BREAKING — decision 027: `PlannedEdit::disclosures`, five changed `EditSession` signatures, two REMOVED error variants

**Three separate breaking changes, all from decision 027, all verified.**

**(C.1) `vector::PlannedEdit` gained `disclosures: Vec<String>`**
(`vector/edit.rs:249`). Full struct as shipped:

```rust
pub struct PlannedEdit {
    pub content: Vec<u8>,          // rewritten decoded content-stream bytes
    pub operators_touched: usize,
    pub disclosures: Vec<String>,  // NEW — decision 027
}
```

`disclosures` is *"what the operator must be told about **HOW** the edit
was expressed"*, in operator-facing prose, **empty in the common case**.
It is populated when the surgery had to change **the FORM of an operator**
to express what was asked, because **some shapes in PDF cannot say what
the operator just asked for**. The canonical case: dragging one corner of
an `re` rectangle. `re` carries an origin **and a size**, so a
four-sided shape that is not a box **has no `re` spelling at all**
(§8.5.2.1) and the operator must be expanded to four lines. The **drawing
is unchanged** — but the bytes are **not recoverable by dragging back**,
and an operator who cares about minimal diffs (**R46**) is owed that fact
rather than left to find it in a diff.

> **This is CLAUDE.md rule 4 applied to REPRESENTATION rather than to
> value.** pdfce may reshape *how* a thing is written in order to do what
> was asked — and says so when it does. Nothing about the rendered page
> changed; something about the file's recoverability did, and that is
> exactly the class of fact rule 4 exists to surface.

**(C.2) The five `EditSession` vector methods changed return type** —
from `Result<(), EditError>` to **`Result<Vec<String>, EditError>`**,
propagating (C.1)'s disclosures to the caller. Verified in `edit.rs`:

| Method | Line | Signature as shipped |
|---|---|---|
| `move_object` | `2170` | `(&mut self, page_index: usize, object_index: usize, dx: f64, dy: f64) -> Result<Vec<String>, EditError>` |
| `delete_object` | `2220` | `(&mut self, page_index: usize, object_index: usize) -> Result<Vec<String>, EditError>` |
| `delete_subpath` | `2290` | `(&mut self, page_index: usize, object_index: usize, subpath_index: usize) -> Result<Vec<String>, EditError>` |
| `move_subpath` | `2342` | `(&mut self, page_index: usize, object_index: usize, subpath_index: usize, dx: f64, dy: f64) -> Result<Vec<String>, EditError>` |
| `move_node` | `2406` | `(&mut self, page_index: usize, object_index: usize, node_index: usize, to: vector::Point) -> Result<Vec<String>, EditError>` |

All five route through one private `vector_surgery(CommandKind, page_index, ..)`
helper, which is why the change landed as one edit across five methods
rather than five independent ones.

**A `let _ = session.move_object(..)?;` written before decision 027 still
compiles and silently discards the disclosures.** That is the migration
hazard worth naming: **the breakage is not uniformly a compile error**.
Call sites that bound the result (`let () = ...`) break loudly; call sites
that ignored it break **silently and lose a rule-4 disclosure**.

**(C.3) TWO `VectorEditError` variants were REMOVED** —
`VectorEditError::RectangleNode` and `VectorEditError::ImplicitNode`.
Verified **absent** from `vector/edit.rs:120`'s enum as of this read.
Decision 027's pattern is **materialize rather than refuse**: instead of
refusing to move an `re` corner or an implicitly-started subpath's anchor,
the planner now **materializes the missing operand** (expanding `re` to
four lines; writing an explicit `m`) and **discloses** it via (C.1).

> **★ The removal had a consequence in a crate this section does not
> cover, and it is recorded here because §4 is where a reader will look
> for the blast radius.** R144 fired a second time on exactly this: **the
> refusals were incidentally GATING a `pdfce-gui` drag gesture** that ran
> over the whole object's flat anchor list with no rung state. Removing
> them turned a drag that was *refused on release* into a drag that
> *succeeds and edits the drawing*. **The protection a refusal provides is
> felt where it is INVOKED, and core-side reasoning cannot see it**
> (**R147**). Closed in Pass 26.0 (`c62c4d0`). See `ROADMAP.md`'s ⚠ block
> at the head of *Shipped*.

**The current `VectorEditError` variant set, verified in full**
(`vector/edit.rs:120`, `thiserror`, one variant per refusable condition):
`ObjectOutOfRange { index, count }`, `NotAPath { .. }`, `DegenerateCtm`,
`MalformedOperand`, `DeleteWouldMoveNextSubpath { .. }`,
`SubpathOutOfRange { index, count }`, `ClippingPath`,
`SubpathStructureMismatch { .. }`, `NoHandleHere { .. }`,
`NodeOutOfRange { index, count }`.

---

### (D) Pass 30.1 / 26.1 — Bézier handles: `plan_move_handle`, `Handle`, `NoHandleHere`, `EditSession::move_handle`

**Additive; nothing previously documented changed.**

- **`vector::Handle`** (`vector/edit.rs:1011`) — a two-variant, `Copy`,
  `Hash` enum: **`Incoming`** (the control point governing the curve as it
  **ARRIVES** at the node — the *second* control point of the segment that
  ends there) and **`Outgoing`** (the curve as it **LEAVES** — the *first*
  control point of the segment that starts there).

  > **★ Named for DIRECTION OF TRAVEL, not "first/second", and the
  > reasoning is an API-design argument worth preserving:** first-and-second
  > are properties of an **operator**, and an operator says nothing about
  > which node a front end has selected. A caller holding a node index
  > needs to name a handle *relative to that node*; `First`/`Second` would
  > have forced every caller to re-derive which operator the node sits in.
  >
  > **The corollary that bit in `pdfce-gui` (Pass 26.1): a cubic's two
  > control points belong to DIFFERENT nodes.** Segment *k* runs anchor *k*
  > → anchor *k+1*, so `c1` is anchor **k**'s *outgoing* handle and `c2`
  > is anchor **k+1**'s *incoming* handle. **Assigning both to one node is
  > the plausible-looking wrong answer** — it draws two handles in roughly
  > the right place and then makes every handle drag deform the *far* end
  > of the curve, with no artefact, no error and no refusal to signal it.

- **`vector::plan_move_handle(content, obj, node_index, handle, to_page)
  -> Result<PlannedEdit, VectorEditError>`** (`vector/edit.rs:1076`) —
  moves one control point, **leaving the on-curve node exactly where it
  is**. This is the operation that changes a curve's **SHAPE**; without it
  `plan_move_node` could only move the points a curve passes *through*, so
  **curvature was not editable at all**.

  **The `v`/`y` promotion, which is the spec-governed part.** ISO 32000-1
  §8.5.2.1 Table 59 gives cubic segments three spellings, two of which
  **omit a control point by making it equal to a point they already
  have**. So the handle the operator grabs on a `v` or `y` **has no
  operand of its own to overwrite**, and the operator must be **promoted
  to `c`** before it can be moved. Same root cause and same remedy as Pass
  30.0's (C.3) materialization.

- **`VectorEditError::NoHandleHere { .. }`** — the refusal for a node
  whose requested side has no curve at all (a line endpoint has no
  handles).

- **`EditSession::move_handle(&mut self, page_index, object_index,
  node_index, handle: vector::Handle, to: vector::Point) ->
  Result<Vec<String>, EditError>`** (`edit.rs:2463`) — **the sixth member
  of (C.2)'s family**, `Vec<String>`-returning from birth rather than
  migrated, routing through the same `vector_surgery` helper with
  `CommandKind::MoveHandle`.

**Also verified present, and previously undocumented in §4:**
`vector::anchor_count(content, obj) -> usize` (`edit.rs:1232`) — the
flattened per-object anchor total that the GUI's object-scoped node
numbering is defined against.

---

### (E) Pass 25.x — the subpath-level vector surface (`hit_test_subpaths`, `subpath_bounds`, `plan_delete_subpath`, `plan_move_subpath`)

**Additive.** Verified in `vector/hit.rs` and `vector/edit.rs`:

| Item | Signature | Notes |
|---|---|---|
| `hit_test_subpaths` | `(model: &PageObjects, object_index: usize, point: Point, tolerance: f64) -> Vec<usize>` | `hit.rs:277`. Returns **every** matching subpath index, not just the front one — the plural query, per the R92/`hit_test_point` head-of-plural invariant already recorded in §4's Pass 18.5 block |
| `subpath_bounds` | `(model: &PageObjects, object_index: usize, subpath: usize) -> Option<Bounds>` | `hit.rs:329`. `None` for a non-path object or an out-of-range index; a subpath whose every vertex is non-finite yields the `EMPTY` seed rather than a garbage box |
| `plan_delete_subpath` | `(content, obj: &PathObject, subpath_index: usize) -> Result<PlannedEdit, VectorEditError>` | `edit.rs:473`. **Three refusals that ARE the design:** `ClippingPath` (checked first — deleting part of a clip changes what is painted *elsewhere*), `SubpathOutOfRange`, `SubpathStructureMismatch` |
| `plan_move_subpath` | `(content, obj: &PathObject, subpath_index: usize, dx: f64, dy: f64) -> Result<PlannedEdit, VectorEditError>` | `edit.rs:687`. Pass 28.0; expressible **only because** (B)'s `tokens` range exists |

**Full current `vector` re-export surface** (`vector/mod.rs:61-83`), given
in full because the module's public face is what a §4 reader is looking
for and it is now much wider than any prior block described:

- from **`centerline`**: `CENTERLINE_ASPECT_THRESHOLD`,
  `CenterlineCandidate`, `derive_from_path`, `page_candidates`
- from **`decompose`**: `DecomposeDiagnostics`, `DocumentFonts`,
  `DocumentXObjects`, `FillRule`, `FontResolver`, `ImageObject`,
  `ImageSource`, `MAX_FONT_NAME_BYTES`, `MAX_NODES`, `MAX_OBJECTS`,
  `MAX_TEXT_PREVIEW_CHARS`, `NoFonts`, `NoXObjects`, `PageObjects`,
  `PaintStyle`, `PathObject`, `Segment`, `Subpath`, `TextBoundsBasis`,
  `TextFont`, `TextObject`, `TextPreview`, `TokenRange`, `VectorObject`,
  `XObjectResolver`, `XObjectShape`, `decompose`, `decompose_page`,
  `decompose_with_fonts`
- from **`edit`**: `Handle`, `PlannedEdit`, `VectorEditError`,
  `anchor_count`, `plan_delete`, `plan_delete_subpath`, `plan_move`,
  `plan_move_handle`, `plan_move_node`, `plan_move_subpath`
- from **`geometry`**: `Bounds`, `Matrix`, `Point`, `Rgb`,
  `cubic_from_v`, `cubic_from_y`, `rect_corners`
- from **`hit`**: `FLATTEN_STEPS`, `MarqueeMode`, `hit_test_point`,
  `hit_test_point_all`, `hit_test_rect`, `hit_test_subpaths`,
  `subpath_bounds`
- from **`snap`**: `AxisConstraint`, `MAX_CANDIDATES`,
  `MAX_NEIGHBOURHOOD_SEGMENTS`, `SNAP_FLATTEN_STEPS`, `SnapCandidate`,
  `SnapConfig`, `SnapKind`, `constrained_second_point`,
  `measured_length`, `snap_candidates`

---

### (F) Pass 29.0 — composite (`/Type0` / CIDFont) text is EDITABLE; R-INV-4 narrowed to two font properties

**This supersedes the Pass 21.0 block's closing paragraph** (see the
inline `[SUPERSEDED]` note there).

- **`ShowSlot::code` is `u32`, not `u8`** — since **Pass 21.1**
  (`text_edit/edit.rs:498`). It is `pub(crate)`, so this is an **internal**
  correction to a claim §4 made, not a change to the public surface. A
  simple font's code **is** one byte and always will be (§9.4.3); the
  width is carried **per slot** as `width: u8` (1 for simple, 2 for
  `Identity-H`) because *a code's value does not tell you how many bytes
  it occupied*, and every byte-range calculation needs that (**R92** —
  deriving it twice would drift).

- **`text_edit::encoding::CompositeEncoding`** — the multi-byte sibling of
  `InverseEncoding`. Where that one inverts a simple font's
  code→glyph-name table, this inverts a **`/ToUnicode` CMap**, and only
  where doing so is **sound**. Public API: `build(base_font: &str, cmap:
  &ToUnicodeCMap) -> Result<Self, NotInjective>`, `encode_str(&self,
  target: &str) -> Result<CompositeEncodeResult, Refusal>`,
  `covers(&self, ch: char) -> bool`.

  **`CompositeEncodeResult { pub cids: Vec<u16> }`** with
  **`to_bytes(&self) -> Vec<u8>`** — big-endian per §9.7.6.2, because
  `Identity-H` codes are two bytes most-significant-first. *"Producing the
  bytes here rather than at the splice keeps the byte order in one place —
  the failure mode for getting it wrong is not a crash but a page of
  plausible, entirely different glyphs."*

  > **★ API-GUIDELINES OBSERVATION, flagged not fixed (CLAUDE.md rule 10).**
  > `CompositeEncoding`, `CompositeEncodeResult` and `NotInjective` are
  > `pub` in their modules but are **NOT in `text_edit/mod.rs`'s
  > `pub use encoding::{...}` list**, which re-exports only
  > `CharEncoding`, `EncodeResult`, `InverseEncoding`, `RInvTrigger`,
  > `Refusal`. So a caller reaches them at
  > `pdfce_core::text_edit::encoding::CompositeEncoding` while their
  > simple-font siblings are available at `pdfce_core::text_edit::`. **The
  > asymmetry is real and is either an oversight or a deliberate
  > staging — this sync could not determine which, and does not guess.**
  > Owed: an engineer decision to re-export or to document the omission.
  > (`NotInjective` additionally lives in `text_extract::cmap`, so a
  > `text_edit` re-export would also be a cross-module lift.)

- **`RInvTrigger::Composite` (R-INV-4) now fires ONLY on two font
  properties** (`text_edit/edit.rs:1810`), where it previously refused
  **every** composite run:
  1. **`/ToUnicode` absent** — nothing in the file says which code
     produces a given character;
  2. **`/ToUnicode` non-injective** — a ligature destination or a
     collision, so the inverse **is not a function**.

  **Both are properties of the FONT, and no amount of pdfce work fixes
  them** — the information is not in the file. That is R110's distinction,
  now load-bearing rather than descriptive.

  > **★ The finding worth carrying, because it is a REVIEW HEURISTIC, not
  > a fact about fonts.** The old blanket refusal was justified on the
  > grounds that *"the re-encoding path is single-byte end to end."*
  > Surveyed again before starting, that was **substantially untrue**, and
  > the parts that were true **were already built**: `ExtractFont::width`
  > already read `Widths::Composite` (`/W`, `/DW`, §9.7.4.3);
  > `emit_literal_string` already escaped arbitrary bytes as three-digit
  > octal, so a two-byte code needs no hex-string emitter (§7.3.4.2);
  > `CompositeEncoding` was already written and tested **and nothing
  > called it**; `ShowSlot` already carried `code: u32` + `width`.
  >
  > **The refusal had become SELF-JUSTIFYING**: it was cited as the reason
  > those types could stay single-byte, and their being single-byte was
  > cited as the reason the refusal had to stay. **When a limitation's
  > stated cause is a second limitation, check whether either is still
  > true independently** (**R143**).

- **Full current `text_edit` re-export surface** (`text_edit/mod.rs:72-99`),
  previously not enumerated in §4 at all: `addtext::{AddTextError,
  AddTextOutcome, AddTextReport, AddTextRequest, AddTextWrapPreview,
  FontProvenance, NewTextColor, WrapPreviewLine, add_text, preview_wrap}`;
  `edit::{EditError, EditGlyphSource, EditOptions, EditOutcome,
  EditReport, EditRequest, FollowerDisposition, edit_text}`;
  `encoding::{CharEncoding, EncodeResult, InverseEncoding, RInvTrigger,
  Refusal}`; `format::{FillModel, FontSelector, FormatError,
  FormatOptions, FormatOutcome, FormatReport, FormatRequest, MetricSpec,
  NewFill, SUBSCRIPT, SUPERSCRIPT, ScriptMetrics, ScriptPosition,
  StyleOutcome, StyleResolution, set_format}`; `model::{Block,
  BlockDiagnostics, BlockKind, BlockRecognitionOptions, EditableTextModel,
  GlyphRef, Line, TextPosition}`; `reflow::{AlignmentSource,
  BlockAlignment, DetectedAlignment, PageOverflow, ReflowDiagnostics,
  ReflowEngine, ReflowError, ReflowLine, ReflowPreview, ReflowRequest,
  reflow_recognition_options}`; `reflow_apply::{ReflowApplyError,
  ReflowApplyReport, ReflowOutcome, apply_reflow}`;
  `synth::{BOLD_STROKE_RATIO, OBLIQUE_TAN, StyleSynthesis, SynthesisOffer,
  SynthesisPath, bold_stroke_width, detect as detect_style_synthesis}`.

---

### (G) `82228b1` — `text_edit::reflow::PageOverflow::past_right_pt`

**Additive and non-breaking: `PageOverflow` is `#[non_exhaustive]`**, so
no downstream struct literal existed to break. Current shape
(`text_edit/reflow.rs:303`):

```rust
#[non_exhaustive]
pub struct PageOverflow {
    pub past_bottom_pt: f64,
    pub lines_outside: usize,
    pub past_right_pt: f64,   // NEW — 82228b1
}
```

`past_right_pt` is how far the new block box extends past the cropbox
**RIGHT** edge, `0.0` when it does not. **Disclosed, never clamped** —
matching the module's existing posture for the vertical case (decision 015
§3.5, **R76**).

> **★★ THE CONTRACT CHANGE THAT IS NOT VISIBLE IN THE STRUCT, and is the
> reason this item is in §4 at all.** `Option<PageOverflow>` carried an
> **undocumented invariant that lived in its CALLERS, not in its type**:
> *present ⇒ the BOTTOM overflowed*. Making it `Some` for a
> right-edge-only case invalidated every reader that had narrowed it by
> convention — the apply stage began emitting *"grows the block 0.0pt past
> the page bottom"*, **false in its letter**, while the true horizontal
> disclosure was **swallowed** by a downstream filter keyed on the
> **prose** of the note (`contains("not applied")`).
>
> **A coupling expressed as string content is invisible to every
> signature and every type check.** The rule this yields for any future
> §4 contract: **when a sum/option type's inhabited case gains a new
> cause, audit each reader for the narrower meaning it had assumed, and
> guard each cause independently rather than at the `Some`.** Recorded as
> an amendment to **R148**.
>
> **This does NOT close Pass 33.0.** It implements option **(c)** —
> disclose — **only**. The auto-detected wrap width is still measured from
> a bounding box a prior `edit-text` may have widened, so the *inference*
> is still wrong; it is merely no longer silent.

---

### (H) Decision 026 — the ce-dimension model (`pdfce_core::dimension`)

**Terminology, binding per CLAUDE.md rule 15: everything in this
subsection is about *ce dimensions* — the `/Line` + `/IT /LineDimension`
annotations pdfce AUTHORS, with their baked `/AP`, groups, scale,
`/Measure` dict and `/PieceInfo` sidecar. It says nothing about *pdf
dimensions* (dimensions a CAD tool already exported into the file), which
pdfce reads and measures against but must not silently alter.**

Full current re-export surface (`dimension/mod.rs:62-76`), verified:

- **`author::{AUTHORED_ANNOT_KEYS, AUTHORED_MEASURE_KEY,
  AuthoredDimension, DimensionStyle, author_dimension}`**
- **`fit::{FitCircle, fit_circle_taubin, fit_circle_taubin_refined}`**
- **`group::{DEFAULT_GROUP_ID, DimStandard, DimensionId, DimensionKind,
  DimensionModel, DimensionRecord, Group, GroupId}`**
- **`length_parse::{LengthParseError, ParsedLength, parse_length}`**
- **`measure_dict::{build_measure_dict, build_ocg, build_ocproperties}`**
- **`sidecar::{SIDECAR_VERSION, deserialize_model, serialize_model,
  sidecar_version}`** — `SIDECAR_VERSION: i64 = 1`
- **`units::{DecimalMarker, FractionMode, MeasurementDisplay,
  NO_SCALE_DISCLOSURE, NumberFormat, ScaleEntry, ScalePreview, ScaleState,
  Unit, format_measurement, preview_group_scale}`**

**`DimStandard`** (`group.rs:96`) — the drafting standard a ce-dimension
group is authored to. **ANSI is the factory default** (operator,
2026-08-04: *"My default is ANSI, but ISO should be an option too"*).
Recorded in the type's own doc comment and worth repeating here because it
is routinely miscited: the ANSI line/arrowhead/lettering conventions are
**ASME Y14.2**, **not Y14.5** — Y14.5 is the GD&T/tolerancing standard.

**`EditSession::set_group_standard(&mut self, group: GroupId, standard:
DimStandard) -> Result<usize, EditError>`** (`edit.rs:6645`) — sets a
group's standard and returns the count of affected ce dimensions.
**Refuses with `EditError::DocumentEncrypted` when the trailer carries
`/Encrypt`**, checked first.

**`NumberFormat`** (`units.rs:178`) — `{ unit: Unit, fraction:
FractionMode, .. }` plus the **Pass 27.2 decimal-marker field**.
**`DecimalMarker`** (`units.rs:205`) is `Point` (default — `1.5`, ANSI
practice) or `Comma` (`1,5` — **mandated by ISO 129-1:2018 cl. 4.1.1, and
widely violated in practice, which is why it is overridable rather than
implied**). **`FractionMode::Decimal { places: u32 }`** shows fixed full
precision so `3.10 m` reads consistently.

**The `/RD` + `/RT` half of the §12.9 `/Measure` mirror is closed; the
`/FD` half is NOT.** pdfce prints a fixed number of decimal places in its
**baked** label (`3.10 m`) while the mirrored `/Measure` dict **omits
`/FD`**, whose spec default of `false` **permits a conforming reader to
print `3.1 m`**. Any doc comment claiming the two *"agree by
construction"* is **wrong as written**. Tracked as an open item under
`ROADMAP.md` *Next up*.

> **★ Why pdfce cannot see this failure from inside itself, which is the
> generalizable part:** **pdfce's label is baked into the appearance
> stream, and the dict is what everyone ELSE computes from.** A test that
> asserts on pdfce's own rendering will always agree with itself. Any real
> fix needs **a reader that is not pdfce**, or a test that asserts on the
> **dict's semantics** rather than on pdfce's rendering of it.

**Sidecar validation (`3a23694`) — a behavioural contract with no public
symbol.** `dimension/sidecar.rs` validates every file-supplied placement
scalar before it is drawn: `usable_page_value(v) = v.is_finite() &&
v.abs() <= MAX_PAGE_VALUE` with `MAX_PAGE_VALUE = 1.0e7`. **Both are
PRIVATE** (`sidecar.rs:352`, `:371`) — so this is not a `pub` item to
document, but it **is** a contract on `deserialize_model`: an unusable
scalar **defaults to `0.0` rather than dropping the record**, on the
ground that a standoff is a presentation detail and losing the whole ce
dimension over one bad number is the worse failure. Recorded here because
a §4 reader needs the *behaviour* even though there is no signature to
point at.

---

### (J) Pass 38.4 — `annot::Annotation` gains `contents`, `title`, `modified` (ADDITIVE, read-only) — 2026-08-06, `8228f44`

**Filed on the Pass, not in a later sync**, because §4 lagging several
filings behind the shipped core surface is the failure §4.1 exists to
correct — repeating it on the very next core change would be the whole
lesson unlearned.

**Before** (`pdfce_core::annot::Annotation`): `id`, `subtype`, `rect`,
`flags`, `appearance`, `is_popup`, `is_widget`, `oc`. **No note text, no
author, no modification date** — a struct describing *shapes*.

**After**, three additions, all `Option`, all read-only, all populated
from dictionary keys `model_annotation` **already had in hand** (this is
a widening of what is *surfaced*, not new parsing):

| Field | Key | Type | The contract, and why it is that contract |
|---|---|---|---|
| `contents` | `/Contents` | `Option<String>` | Decoded through the **§7.9.2 text-string decoder** every other text-string consumer in the crate uses — **not** a private byte-to-char conversion, which would disagree with the rest of pdfce on non-Latin input. Pinned by a **UTF-16BE test asserting `"Ré"`**, which a naive conversion cannot pass |
| `title` | `/T` | `Option<String>` | **ISO 32000-1 Table 170 — a MARKUP-annotation key, not Table 164.** Legitimately **absent** on a Link or a Stamp. `None` therefore means *"this subtype has no such concept"*, **NOT "anonymous"** — documented on the field, because a UI that conflates those two lies about the document |
| `modified` | `/M` | `Option<String>` | **Stored RAW — deliberately NOT parsed to a date type.** §12.5.2 types `/M` as *"date **or** text string"* and **requires readers to "accept and display a string in any format."** A date parser would have to reject or mangle values the standard obliges a reader to accept. **A test pins a `/M` of `(last Tuesday)` surviving verbatim**, so a later "improvement" to parse it fails loudly |

**Documented rather than silently half-applied:** §12.5.6.2 NOTE 2 says a
markup annotation carrying an `/IRT` parent has **its own `/Contents`
ignored** in favour of the thread. Honouring that needs `/IRT` modelling
this struct does not have, so the raw value is surfaced **and the caveat
is stated on the field**. Surfacing it silently would imply a threading
model that does not exist.

**Breaking? NO.** Three added fields on a struct consumers construct only
by reading a document. **`ARCHITECTURE.md` §3's GUI-core invariant was
re-verified at this commit specifically** (`cargo tree -p pdfce-core` /
`-p pdfce-render`: zero GUI-dependency matches) — this was the first Pass
in several to touch `pdfce-core` at all, so the check was load-bearing
rather than ceremonial.

**Still absent, named so the edge is honest:** no `delete_annotation`
verb (`edit.rs` L3664's three hazards — dangling `/AcroForm /Fields`,
`/Popup` companions, `/IRT` reply chains); no `/IRT` field; no
`/CreationDate`; no `/RC` rich-text contents. The **CLI** has not yet been
widened to print these (`list-annotations` gains `contents=`/`author=` in
Pass 38.5) — a live instance of the **R151** core-ahead-of-shell pattern,
recorded rather than rounded up.

### (K) `e4256f2` — ★★ BREAKING for downstream implementors: `ObjectGraph` gains `Send + Sync`; `pdfce-render` gains `RenderCancel` and `RenderOptions.cancel` — 2026-08-07

**Two public-surface changes, in two crates, taken together because the
first exists only to enable the second.**

**1. `pdfce_core::graph::ObjectGraph: Send + Sync` (supertrait added).**

**Breaking in the semver sense** — a supertrait on a public trait is a new
obligation on every downstream implementor — **and additive in practice**,
because it does not add a requirement so much as stop **erasing** one.
`Document`, `EditSession`, `SessionGraph`, `PendingGraph` and the test
graphs were all already thread-safe. The obstacle was **only the trait
object**: `DocumentView` holds `&dyn ObjectGraph`, and **an unbounded
`dyn Trait` is neither `Send` nor `Sync` regardless of what implements
it.**

**Established by compile probe before and after**, not by the argument
above: `DocumentView<'static>` went from failing both bounds to satisfying
both, **zero errors across the workspace**.

**Why NOW is the whole justification, and it is a timing argument rather
than a design one.** With **no git remote and no release**, the affected
implementor set is **exactly this repository's, and it is empty**. That is
the cheapest this bound will ever be, and the cost grows monotonically
from here. Rust API guideline **`C-SEND-SYNC`** asks for `Send`/`Sync`
where possible and for exceptions to be documented — **the rationale lives
on the trait** (`CLAUDE.md` rule 10, §8).

**2. `pdfce_render::RenderCancel` + `RenderOptions.cancel: Option<RenderCancel>`.**

A plain **`Arc<AtomicBool>`** (`crates/pdfce-render/src/cancel.rs`). **No
windowing, no async runtime, no executor** — so **§3's GUI-core separation
invariant holds** (verified: `cargo tree` reports **0 GUI matches** for
both `pdfce-core` and `pdfce-render` at this commit), and it compiles and
behaves identically under **wasm**, which is the property that keeps the
eventual web fork a shell-crate swap.

**`cancel` defaults to `None`, and that default is a SAFETY property, not
ergonomics.** Every existing caller keeps a render that cannot be
interrupted, so **the CLI, the round-trip oracle and the R85 harness
cannot acquire a new failure mode from this field existing.** For a caller
passing no token the check is `Option::is_some_and` on a `None` — one
always-false branch, **no atomic executed at all**.

The poll sits **between content-stream operators**, one **relaxed** load.
Relaxed rather than acquire because **the flag carries no data and guards
no memory**; a late answer costs one more operator. Cancellation
granularity is bounded by a single operation — at **~360 µs per clip ×
24,128 clips** on the CAD reference sheet — so **worst-case latency is
about a third of a millisecond, not the render.**

**Measured** (engineer's figures, relayed under R87): pre-cancelled
**322 ms** against **10,367 ms** uncancelled; **28.9 ms** from `cancel()`
to thread exit **mid-render**.

**What this does NOT do, stated because the numbers invite the opposite
reading: the GUI still freezes.** These are layers 1 and 2 of off-thread
rasterization. **Layer 3 — `Arc<EditSession>`, a worker thread, a channel
and a generation counter — does not exist**, and lives in `pdfce-gui`.
Nothing here makes a render faster; it makes one **interruptible**.

**Consequent design ruling, recorded here because it constrains layer 3.**
On an edit arriving while a background render holds the graph: **cancel the
render, wait, then mutate — one choke point.** The measured **28.9 ms**
wait is what makes this a ruling rather than a preference; the
alternatives were being weighed against an unknown that turned out to be
**three orders of magnitude below ~58 s of blocking.** Rejected:
snapshotting the session (**`EditSession` is not `Clone`** — this would
need a new public deep-copy on the crate's largest type, a public API
addition taken to avoid a 28.9 ms wait) and **`Arc::get_mut` at ~40
sites** (spreads a concurrency concern across the whole mutation surface
to serialise what one flag already serialises).

~~**No Pass ID assigned**~~ — the argument both ways is recorded in
`ROADMAP.md`'s *Shipped* entry §6; **the librarian's non-binding reading
is one ID covering all three layers, minted when layer 3 lands.** This
subsection exists regardless of that ruling, because **the public API
changed and §4.1 is the living truth.**

> **[★ AMENDED 2026-08-07 (`7926a78`) — TWO FACTS IN THIS SUBSECTION HAVE
> CHANGED, AND A THIRD HAS NOT.**
>
> **1. The ID is minted: this is `Pass 44.0`**, covering all three layers,
> with `e4256f2` recorded retroactively — the recommendation above,
> accepted in full. **The blocking condition CLEARED when layer 3 landed**
> (a Pass whose criterion is *"the GUI no longer freezes"* would have been
> failed by `e4256f2` alone; it is not failed by the pair).
>
> **2. *"The GUI still freezes"* is TRUE OF `e4256f2` AND NO LONGER TRUE OF
> THE PROJECT.** Layer 3 exists: `crates/pdfce-gui/src/render_worker.rs`,
> **503 lines**, holding the worker thread, the channel, the cancellation
> token and the generation counter. **`Arc<EditSession>` landed with it**,
> and every mutation now passes through `OpenDoc::session_mut`, which
> cancels and **joins** before handing out `&mut` — so **`Arc::get_mut` is
> infallible by construction**, not by hope.
>
> **3. NOT CHANGED: `pdfce-core`'s public API.** `7926a78` touches
> **`pdfce-gui` only** (4 files, +840 / −132; `main.rs`, `raster.rs`,
> `render_worker.rs`, `ui_text.rs`) — **no core or render file, no
> manifest**, so this subsection's API description remains the complete
> and current one, and the GUI-core separation invariant (§3) is untouched
> by it. **`cargo tree` was re-run anyway and reports 0 GUI matches for
> `pdfce-core`.**
>
> **One correction to the ruling's own arithmetic, filed rather than
> quietly fixed:** the rejected `Arc::get_mut` alternative is described
> above as ***"~40 sites"***. **The real count is 51** — established by
> performing the change and letting the **compiler** count, against a
> static pre-count of 46 and 49 compiler-visible borrow errors (the two it
> missed are in test code `cargo build` does not reach). **51 mutating +
> 45 read-only = 96 total `session` call sites; 51 / 96 = 53%.** The
> ruling is **unaffected** — 51 sites is a stronger argument against that
> alternative than 40 — but the figure was wrong, it was relayed, and this
> project has had **five wrong relayed figures in one day**. See the
> `Pass 44.0` *Shipped* entry §4.]**

### (I) What this sync did NOT cover — stated so the edges are honest

**A partial sync that names its edges is worth more than a
complete-looking one that does not.** Not audited in this pass, and
therefore **neither claimed current nor claimed stale**:

1. **`pdfce-cli`'s argument/output surface.** CLAUDE.md rule 10 puts it
   under the same API-guidelines check as `pdfce-core`'s `pub` items, and
   rule 11 means every Pass since 21.0 added subcommands. **§7 (CLI
   capabilities) has not been synced and is presumed to lag by the same
   several Passes §4 did.** Owed: its own dispatch.
2. **`EditSession`'s other 57 `pub fn`s.** The crate exposes **63** at
   `edit.rs`; this sync verified the **six** vector methods and
   `set_group_standard`. The rest — text editing, annotations, redaction,
   forms, page ops, undo/redo — are **not** enumerated in §4 anywhere, and
   that gap predates this sync.
3. **`Page`'s field list** — shape confirmed, fields not audited (see (A)).
4. **Whether `PdfError` still exists** alongside `DocError` (see (A)).
5. **Whether `Subpath` is `#[non_exhaustive]`** (see (B)).
6. **The `pub` surfaces of `annot`, `forms`, `redact`, `signature`,
   `recover`, `writer`, `pageops`, `view`, `text_extract`, `filters`,
   `image_codec`, `fontdata`, `linearization`, `objstm`, `vartext`,
   `text_state`, `span`, `graph`, `lexer`, `parser`, `page_tree`,
   `xref`** — 27 `pub mod`s exist in `lib.rs`; §4 describes a handful.
   **§4 has never been a complete API index and this sync did not make it
   one.** It brought the *changed and breaking* surface current, which was
   the dispatch's stated priority.

**Recommended follow-on, in the order the value falls:** (1) sync **§7**
for the CLI; (2) decide the **`CompositeEncoding` re-export** question in
(F); (3) enumerate **`EditSession`** — the single largest undocumented
`pub` surface in the crate.

## 5. Round-trip / non-destructive-editing invariant

Analogous to the tail-bytes / lazy-round-trip discipline the user's
other format-RE project (SWFormat) established for SOLIDWORKS files —
same principle, different format:

- Any object pdfce did not logically modify is re-emitted **byte
  identical** (for full rewrite) or **omitted entirely** (for
  incremental save, since the old bytes are simply not touched).
- Never "normalize" a PDF's internal structure as a side effect of an
  unrelated edit (e.g. don't silently rewrite every xref table to xref
  streams just because pdfce opened the file). Minimal-diff editing is
  a hard requirement — Acrobat users expect that adding one comment to
  a 400-page contract does not perturb the other 399 pages' bytes,
  and forensic/signature-validity expectations depend on it.
- Corollary: **redaction is the one deliberate exception.** True
  redaction must actually remove the covered content from the object
  stream (not just draw a black box on top) — see ROADMAP backlog
  item "Redaction — true content removal". This is a documented,
  intentional violation of the minimal-diff rule for exactly the
  objects the user asked to redact, and only those.
- **Forward pointer (2026-07-30):** the mechanical enactment of this
  invariant — `ByteSpan` provenance on every parsed object and a
  lossless, span-provenanced content-stream token model — is specified
  by the six Pass-1 obligations in the §12 entry of 2026-07-30
  (decision record `docs/decisions/001-oxidize-pdf-adopt-vs-build.md`
  §6.1); full design text lands in this document with Pass 1.
- **PDF-1.5 extension (2026-07-30, Pass 1.1 item 1 — continuation
  8):** objects parsed out of object streams (§7.5.7) carry
  `Provenance::ObjectStream { container, index }` rather than a file
  `ByteSpan` — for these, byte-identical passthrough is
  **expressible-or-consciously-absent**: a compressed object has no
  contiguous file bytes to re-emit, so any writer that touches one
  must either promote it to an uncompressed object or rewrite its
  container stream. The contract is documented on the `Provenance`
  type itself in `pdfce-core`; `file_span()` returns `Some` only for
  `Provenance::File`. See the §12 continuation-8 entry of 2026-07-30.

### 5.1 The invariant stated precisely — three contracts, never one

*(Added 2026-07-31, Pass 3.0. Before this Pass §5 was prose; it is now
a measured gate. Decision 007 W1/R32 names conflating these "the single
likeliest source of a false green or a false red".)*

The invariant is **not** one claim. It is three, and each save mode
promises exactly one of them:

| Save mode | What is byte-identical | Assertion shape |
|---|---|---|
| `save_incremental`, **empty dirty set** | the **whole file** — output *is* input | `output == input` |
| `save_incremental`, non-empty dirty set | **every byte below the original EOF** | `output.starts_with(input)` |
| `save_full` | **every object definition** of a `Provenance::File` object | per object, **never** per file |

A full rewrite **cannot** be byte-identical file-wide: object offsets
move, so the cross-reference section must differ. A test asserting
file-level identity for `save_full` fails universally; a test asserting
only reloadability passes vacuously. Both mistakes look like diligence.

Two corollaries that are easy to get wrong and expensive to discover
late:

- **Zero edits means zero bytes.** An empty dirty set produces the
  input file, not "the input plus an empty revision". Appending a
  revision to a document the operator did not change is itself a §5
  violation.
- **The dirty set is a save-time diff against the base revision**,
  never the union of every command run (§11.1). §7.5.6 requirement 1
  is the spec-side reason: an update section *"shall contain entries
  **only for** objects that have been changed, replaced, or deleted"*
  — a restriction, not merely permission to omit.

**Measured, not asserted** (Pass 3.0, 2,914-file corpus): whole-file
identity 2,898/2,898 loadable files (100%); prior-bytes-intact on
append 2,898/2,898 (100%); per-object verbatim on full rewrite
2,897/2,898, the one exception being a hybrid file pdfce **refuses by
name** (see below). `tools/roundtrip` is the executable gate; it
re-runs on every writer-touching Pass.

### 5.2 Redaction forbids incremental save

*(Added 2026-07-31, Pass 3.0, closing decision 007 W2. This was
trust-critical and undocumented, and incremental is the DEFAULT mode.)*

**Incremental save structurally preserves superseded content.** §7.5.6
requires that *"changes shall be appended to the end of the file,
leaving its original contents intact"* — so the old bytes of every
replaced object remain in the file **by construction**. A redaction
saved incrementally therefore leaves the redacted content trivially
recoverable by anyone who reads the earlier revision.

Binding rule (**R35**): redaction — and any operation whose contract is
*removal* — **must force a full rewrite and must refuse incremental
save.** This is enforced in the writer, not left to the Redaction Pass
to remember, and the Redaction Pass owes a test that greps the saved
bytes for the removed content.

See also §11.2, which covers the *undo* half of the same exception:
once a redaction is written, no later session can undo it, because
there is no data left in the file to restore.

**Correction (2026-07-31, Pass 3.1):** this section's original framing
implied that forcing a full rewrite closes the stale-copy path for
**promoted compressed objects**. It does not — object streams carry
through **verbatim in both save modes** (§5.6), so a promoted object's
superseded value survives inside its untouched container even after a
full rewrite. R35's refusal of incremental save is necessary but NOT
sufficient for redaction; see §5.7 for the full amendment and the
binding consequence for the Redaction Pass (container
rewrite/decomposition).

### 5.3 `/ID` discipline on save

*(Added 2026-07-31, Pass 3.0, closing decision 007 W6/R39.)*

§14.4 says `ID[0]` *"shall not change when the file is incrementally
updated"* and `ID[1]` is *"a changing identifier based on the file's
contents at the time it was last updated."* Read naively, the second
half conflicts head-on with byte-identical round-tripping.

It does not actually conflict, and the reasoning matters because it
will be re-litigated:

1. **If nothing changed, nothing was "updated"** — §14.4's trigger
   never fired.
2. `/ID` is `should`-strength for unencrypted files, and **no `shall`
   anywhere requires regeneration**. §14.4 states what `ID[1]` *is*,
   not when a writer must recompute it.

Binding rule: pdfce regenerates `ID[1]` **exactly when a save writes at
least one changed object**, and never otherwise. `ID[0]` changes only
when pdfce creates a document it regards as new (a from-scratch write,
or an explicit "Save As new document") — never on incremental save or
plain full rewrite. This is also an R41 matter: a gratuitously
regenerated `/ID` is an observable *pdfce touched this file* signal on
a file pdfce did not change.

Load-bearing beyond tidiness: `/ID[0]` is an input to §7.6.3.3's
encryption-key derivation, so an error here becomes a Pass 5 decryption
failure that presents as a crypto bug.

### 5.4 Linearization is invalidated by any save, and never repaired

*(Added 2026-07-31, Pass 3.0, closing decision 007 W5. Citation
corrected 2026-07-31, Pass 3.2 filing: this section's rule is
**R42** — the original "R36" citation collided with decision 007's
R36, "save mode is chosen by contract and disclosed," which the
writer/`document.rs`/`linearization.rs` code comments cite and keep.
See the dated reconciliation note at R42 in `ROADMAP.md` Standing
rules. The warn-before-save behavior below remains ALSO covered by
R36's disclosure clause; the never-repair/never-strip/never-patch-`L`
rule is R42.)*

Annex F.1 is normative and blunt: *"Incremental update shall still be
permitted, but the resulting PDF is **no longer linearized** and
subsequently shall be treated as ordinary PDF."* An append lands past
the first-page cross-reference table and the hint streams, so the
linearization is stale afterwards.

That is spec-sanctioned and unavoidable — but it is an observable
property change the operator did not ask for (the file opens more
slowly over a network). Under the *fuzzy, never sneaky* rule, pdfce:

- **detects** linearization on load (Annex F.3.3's 1024-byte parameter
  dictionary, with `L`-versus-file-length as the liveness check);
- **warns** before a save that would spend a live Fast Web View
  property;
- **never strips** a stale `/Linearized` dictionary (that would be a
  normalization, and Annex G.7's reader-side revalidation depends on
  it being present);
- **never patches `L`.** `L` is not the property — the object ordering
  and hint validity are. A file whose `L` was "fixed" after an append
  *claims* to be linearized while its hints point into a stale layout,
  which is strictly worse for a network reader than an honestly
  de-linearized one.

Re-linearization belongs to the Optimization backlog bucket, not to any
save path.

### 5.5 Signatures and the redaction conflict

*(Added 2026-07-31, Pass 3.0, closing decision 007 W7.)*

§12.8.1 NOTE 1: *"If a signed document is modified and saved by
incremental update, the data corresponding to the byte range of the
original signature is preserved."* A **full rewrite destroys every
existing signature**, because a signature covers a byte range that a
full rewrite necessarily disturbs.

So signature presence forces incremental — which collides head-on with
§5.2's rule that redaction forces a full rewrite. **"Redact a signed
document" is a genuine either/or, not an oversight**, and it must be
surfaced to the operator as an explicit choice, never resolved
silently. Naming it here means neither the Redaction Pass nor the
Signatures Pass can claim surprise.

Structural consequence already in force: pdfce never re-serializes a
signature dictionary, *even identically*. Its `/Contents` is a
fixed-width placeholder referenced by byte offsets, so re-emitting it
is a hazard regardless of whether the bytes come out the same. The
answer is structural rather than a special case — signed objects are
`Provenance::File` objects and ride the verbatim copy path like any
other.

### 5.6 Never normalize — the rule that has no spec backing, and needs none

*(Added 2026-07-31, Pass 3.0. Decision 007 R33/W4.)*

§7.5.6 contains **no requirement** that an appended update section
match the form of the section it supersedes. That is a recorded
NEGATIVE RESULT in the spec RAG, not an oversight — which is precisely
why the rule has to be pdfce's own.

pdfce emits whatever the base file's **newest** cross-reference section
already used, and never chooses:

- a classic §7.5.4 table stays a classic table;
- a §7.5.8 cross-reference stream stays a stream;
- a §7.5.8.4 **hybrid** file is appended to as a classic section
  carrying `/XRefStm` **forward** (form A — the only shape that
  satisfies §7.5.6 requirement 3's *"all the entries except the `Prev`
  entry … whether modified or not"*, since `/XRefStm` is such an
  entry);
- object streams are carried through a full rewrite **intact, with
  zero promotions**: a type-2 entry names a container and an index,
  neither of which is a byte offset, so re-emitting the container
  verbatim leaves every type-2 entry still correct;
- the `%PDF-M.N` header line and its §7.5.2 binary-comment line are
  copied byte-for-byte, so no save can raise a file's version —
  **copied FROM THE `%PDF-` MARKER since 2026-08-07; any bytes BEFORE
  it are dropped by a full rewrite. See the narrowing at the end of
  this section — it is the one deliberate exception §5.6 has.**

**A full rewrite of a hybrid file is refused by name** rather than
flattened. §7.5.8.4 describes a hybrid as a three-part unit a writer
creates *"at the same time"*; rebuilding it from a merged view requires
re-deriving the hidden-object set and re-checking the clause's
recursive visibility rule. Normalizing it to a single section instead
would silently destroy the file's pre-1.5 readability. Refusing is the
R27 fail-clean posture applied to the write side: name it, count it,
do not guess.

#### ★ 5.6.1 THE ONE DELIBERATE EXCEPTION — a full rewrite DROPS bytes before `%PDF-` (added 2026-08-07, `fa4f83c`)

**§5.6 stands, and is narrowed at exactly one point.** A **full
rewrite** emits `%PDF-` at **byte 0** and discards any preamble — the
BOM, whitespace or junk that pdfce's 1 KiB header probe tolerates on
the way in. `save_incremental` and identity-append are **unchanged**
and still carry a preamble through; they promise whole-file identity
and a byte-prefix respectively (§5.1) and **do not call `header_span`
at all**, which an identity assertion in the same test pins.

**This reverses a tested contract, so the reasoning is recorded in
full rather than summarised.**

**What §5.6 said, and it was not wrong.** *Do not normalize what the
operator did not ask about.* A leading preamble is such a thing, the
probe tolerates it, and pdfce's emitted offsets were **absolute from
byte 0 exactly as §7.5.4/§7.5.5 require** (*"the byte offset … from
the beginning of the file"*). Every offset in the output was verified
to match its true position. **pdfce's writer was correct.**

**What overturned it, and it is a MEASUREMENT, not a preference.** A
minimal 3-object file with **correct absolute offsets** and 19 bytes
of junk before `%PDF-` is unreadable to veraPDF — *"can not locate
xref table"* — and the identical file with the junk removed parses
clean (`failedToParse="0"`). **veraPDF reads offsets as
HEADER-RELATIVE whenever a preamble exists**, whatever the producer
intended. So the property is not a quirk of one corpus file's
convention: **every preamble-preserving file pdfce ever wrote was
unreadable to an independent conformance reader.**

**Why dropping is the right answer rather than a capitulation.** The
spec RAG (`iso32000__s__7.5.md`) records the offset base as *"a real,
load-bearing ambiguity"* that **ISO 32000-1 does not resolve** — the
spec position is byte 0, and it gives readers on the other side no
guidance at all. Preserving the preamble **picks pdfce's side of an
unsettled argument** and ships files only that side can open.
**Dropping it makes the two readings COINCIDE**: with the header at
byte 0, *absolute* and *header-relative* are **the same number**, and
the output is unambiguous to every reader. It also stops re-emitting
a **§7.5.2 violation** (*"The first line of a PDF file shall be a
header"*) the operator never asked pdfce to keep.

**Why only a full rewrite may do it.** §5.1's table is the whole
licence: `save_full` promises per-object-definition byte identity, a
reloadable file and an identical raster — **explicitly not whole-file
identity**, because offsets legitimately move. Removing a preamble is
inside that promise and outside the other two.

**The generalisable form, stated because the next ambiguity will not
be about headers:** *where a format spec leaves a question genuinely
unresolved, emit the form under which the competing readings coincide
— not the form under which your own reading is correct.* Put to the
engineer as a candidate standing rule (**R165**) and **deliberately
not minted** by the filing that found it; see `ROADMAP.md`'s *third
defect the veraPDF gate found* entry, *Ledger*.

**[★ AMENDED 2026-08-07, fifteenth filing — `R165` IS MINTED. The
clause above is left exactly as filed; it is no longer current
status.]** The generalisable form quoted above is now **standing rule
`R165`**, ruled in by the operator against the filing librarian's own
recommendation and on that librarian's own counter-argument. **Ceiling
R164 → R165; R166 next free.** Two consequences bind **this section**:

- **§5.6.1 is R165's WORKED EXAMPLE, and is named as such in the rule.**
  The conflict resolved here — R165 pulling one way, §5.6's *do not
  normalize what the operator did not ask about* pulling the other — is
  the model for the next such conflict. **§5.6 remains the default;
  R165 is the exception that must be ARGUED for**, per case, with the
  ambiguous clause cited and the §5.1 save-mode contract checked.
- **R165 does not widen this exception by one byte.** Its own limit
  binds it to cases where the spec is **genuinely silent or
  self-contradictory**, and the paragraph below (*what §5.6 is NOT
  narrowed toward*) is unaffected: the trailing-space header, the
  cross-reference form, object streams and the hybrid refusal are all
  still outside it. **Binding text: `ROADMAP.md`, *Standing rules*,
  `R165`.**

**Also note what §5.6 is NOT narrowed toward.** No other normalisation
is licensed by this: not the `%PDF-1.4 ` with a trailing space (still
copied verbatim), not the cross-reference form, not object streams,
not the hybrid refusal. The exception is **the preamble, on the full
rewrite, only.**

### 5.7 The mutation writer, promotion, and the stale-copy reality

*(Added 2026-07-31, Pass 3.1 — the first Pass with real mutations.
Records both the mutation-writer design and a CRITICAL correction to
§5.2's original framing and decision 007 W3's mitigation. Corrections
are recorded forward; the archived 007 record is not edited.)*

**Design: one writer path, dirty set as an argument.** Pass 3.1
extended the Pass 3.0 writer rather than adding a mutation sibling:
`save_full` (like `save_incremental`) now takes a `&DirtySet` —
replacements (object number → new definition) plus a trailer patch,
with `changes_content()` distinguishing content-bearing edits from
metadata-only ones. `DirtySet::empty()` reproduces Pass 3.0's identity
behavior exactly, making identity a **strict pinned subset** of the
mutation writer, not a parallel code path that could drift. The dirty
set itself is produced by `EditSession` (§11.5) as a save-time diff
against the base revision, per §11.1. `/ID[1]` is derived per §14.4 in
`writer/fileid.rs`, exactly when a save writes at least one changed
object (§5.3); `/ID` is **never synthesised when absent**, in either
mode — the spec RAG's synthesise-on-full-rewrite recommendation was
declined (R41: stamping an `/ID` into a file that never had one is an
observable "pdfce touched this" signal); a real Save-As path may
revisit.

**Promotion (R38) in practice.** A touched
`Provenance::ObjectStream` object is promoted to an uncompressed
object superseded by a type-1 xref entry; its container is left
byte-untouched. Coverage honesty: promotion is **fixture-covered, not
corpus-covered** — 75 corpus files hold 2,197 compressed objects, but
page objects are uncompressed in all of them, so the corpus rotation
gate never exercises promotion; the round-trip harness reports both
numbers so the gap cannot silently pass for coverage.

**The stale-copy reality — CORRECTION.** Decision 007 W3's mitigation
and §5.2's original framing claimed a full rewrite "closes the
stale-copy path" for promoted compressed objects. **FALSE.** Object
streams carry through **verbatim in BOTH save modes** — incremental
save never touches them by construction, and `save_full` re-emits
containers intact with zero promotions (§5.6, deliberately, because
rewriting a container perturbs every other object inside it). So a
promoted object's old value survives inside its untouched container
under *either* save mode. Binding consequence (documented at the
creating code as well): **the Redaction Pass must rewrite or decompose
every container stream that holds a redacted object.** R35 (refuse
incremental) is necessary but not sufficient; the redaction test that
greps saved bytes for removed content (§5.2) is what will hold this
honest, provided its fixtures include object-stream-compressed
content.

**Object creation and `/Size` suppression.** The Pass 3.1 fuzzer found
a real bug class here: creating a new object by raising `/Size`
**resurrected** xref entries that the base trailer's `/Size` was
suppressing (§7.5.4/§7.5.8: entries beyond `/Size` shall be ignored —
and real chains carry such entries, which then fail to parse when
exposed). Fix: `next_object_number` allocates above the **unfiltered**
chain maximum (never reusing a suppressed number), and creation is
refused by name when `/Size` suppresses entries
(`EditError::ObjectCreationWouldExposeHiddenObjects`, CLI exit 9);
editing existing objects still works on such files. Lesson:
`C:\personal_rag\pdf\lesson_20260731_xref_size_suppresses_trailing_entries_raising_resurrects.md`.

### 5.8 Flatten burns in by overlay-APPEND, not content-stream surgery

*(Added 2026-08-01, Pass 7.1 — the first operation that makes an
authored appearance part of a page's rendered content. Records the
design and why it is MORE minimal-diff than the in-place rewrite the
Pass scope anticipated.)*

**The problem.** Flattening a form field removes the interactive widget
and bakes its current appearance into the page so it renders identically
in a non-form-aware viewer. The obvious implementation — splice the
widget's appearance operators into the existing page content stream —
would rewrite that stream, which under §5.6 (never normalize) and the R46
identity discipline is exactly the destructive re-emission pdfce avoids on
every object it did not logically change.

**The design pdfce adopted.** Flatten does NOT touch the existing page
content stream. It:

1. builds a one-line overlay content stream that sets the widget's
   placement matrix (the §12.5.5 `fit_matrix_for` `/Rect`→`/BBox`
   transform) and `Do`-invokes the widget's existing `/AP` `/N` form
   XObject by name (`ContentBuilder::invoke_xobject` — `/Name Do`);
2. APPENDS that new stream to the page's `/Contents` array (promoting a
   single-stream `/Contents` to an array as needed);
3. registers the `/AP` `/N` XObject under the page's `/Resources`
   `/XObject` (`add_page_xobjects`, merging into the page's effective
   resources); and
4. removes the widget from `/Annots` and the field from `/AcroForm`
   `/Fields` (`remove_from_annots` / `remove_fields_from_form`), clearing
   `/NeedAppearances` if it was set.

The pre-existing page content bytes are never re-serialized. The only new
bytes are the appended overlay stream and the dict edits.

**Consequence — R46 keeps ZERO flattened-page exceptions.** Because the
existing content stream passes through byte-verbatim (§5.6 span
re-emission), the R46 re-emit-everything identity gate finds no new
divergence on a flattened page: GATE PASS over `fixtures/synthetic` +
`fixtures/external`, all divergences the known value-preserving `-0`→`0`
number re-spellings, zero corruptions. In-place surgery would have put
every flattened page's content stream through the canonical serializer,
surfacing (harmlessly, but noisily) the number-respelling class §5.6/R46
document — and, worse, would have been a genuine rewrite of content the
operator did not ask to reformat.

**R48 (flatten discloses its destructiveness) is still honored.** Flatten
is destructive in the sense R48 means — the interactive field is gone. But
under incremental save the field dict survives in the PRIOR revision
(recoverable), which flatten discloses; a `--full-rewrite` save produces a
file with no `/FT`/`/Tx` that still renders the burned value. Flatten uses
the STRICT certification gate (refused on any enforced `/DocMDP`, including
`/P 2` certified — proven by test), NOT the fill path's `/P >= 2` permit,
because flatten is a STRUCTURAL change to the page/annotation/field
structure, not a value fill.

**General pattern (recorded for future Passes).** Overlay-APPEND beats
content-stream-surgery whenever the goal is ADDITIVE burn-in (make
something already-authored part of the rendered page). Reserve true
in-place content-stream surgery for the one operation whose goal is
REMOVAL, not addition: **Redaction (Pass 8)** — the R46 named exception,
where covered operators must actually be deleted from the content stream
(and containers decomposed, §5.7), because visual masking is not removal.
The two operations are mirror images: flatten adds without rewriting;
redaction removes and must rewrite. This finding is escalated as a
`personal_rag/pdf` lesson
(`lesson_20260801_flatten_overlay_append_beats_content_stream_surgery.md`).

### 5.9 Every removal/scrub operation forces a full rewrite (R58 — generalizes §5.2's R35)

*(Added 2026-08-01, Pass 8.0 — Redaction landed, and the
`pdfce-ui-specialist` review generalized R35 into a standing rule that
binds every future scrub operation, not just redaction-apply.)*

§5.2 established **R35** for redaction specifically: because incremental
save structurally preserves superseded content (§7.5.6 requires the
original contents be left intact and changes appended), a removal saved
incrementally leaves the removed content trivially recoverable in the
prior revision. The remedy — force a full rewrite, refuse incremental
save, drop `/Prev` so prior revisions are gone — is not unique to
redaction. It is the correct posture for **any** operation whose contract
is *removal or scrubbing of content*.

**Binding rule (R58):** every removal/scrub operation rides the same
forced full rewrite as redaction-apply. This includes, prospectively, any
**Sanitize / Remove-Hidden-Information / metadata-scrub** Pass pdfce may
add. Three obligations travel with the rule:

1. **Force full rewrite, refuse incremental save.** The R35 mechanism,
   enforced in the writer, not left to each scrub Pass to remember.
2. **Decompose every object-stream container holding a scrubbed object**
   (§5.7). Refusing incremental save (R35) is necessary but NOT sufficient:
   object streams carry through verbatim in BOTH save modes, so a scrubbed
   object's old value survives inside its untouched container unless the
   container is rewritten/decomposed. Pass 8.0 proved this concretely — a
   redacted `/Info` compressed in an `/ObjStm` survives without §7.5.7
   Strategy B decomposition (`containers_decomposed >= 1`).
3. **Owe an absence test.** The scrub Pass greps the whole saved output —
   raw bytes AND every decoded content stream — for the removed content
   and asserts zero occurrences. This is R46 inverted: R46 proves presence
   (untouched content re-emitted byte-identical); the absence test proves
   deletion (removed content gone from the entire file). Pass 8.0's
   headline embodied it: `redact-apply` on `demo-secret.pdf` →
   `grep "SECRET" redacted.pdf` = 0 (control `marked.pdf` = 3).

The general framing (§5.8): flatten and redaction are mirror images —
flatten ADDS without rewriting (overlay-append), redaction/scrub REMOVES
and must rewrite (content-stream surgery + container decomposition). R58
is the standing-rule form of "removal is never additive, and never
incremental."

**Staleness flagged, text NOT changed (decision 022 §5.4, filed
`pdfce-librarian` continuation 80, 2026-08-04 — full text:
`ROADMAP.md` Standing rules R58 and Open operator question (v)).**
R58's binding text above ("every removal/scrub operation forces a full
rewrite") is already contradicted by two shipped operations that
correctly stay under the project's default incremental save:
`EditSession::delete_object` (Pass 9c-min, `76485b5`, content-stream
surgery removing visible page geometry) and `delete_redaction_mark`
(Pass 8). Neither operation's contract is confidentiality — see §5.11,
below, which already established that distinction for in-place text
editing (a change, not a removal) and whose reasoning applies here by
extension (a removal whose contract is "no longer in the current
revision," not "provably unrecoverable"). Decision 022's own proposed
`EditSession::delete_annotation` (Pass 22.0, unbuilt) would be a THIRD
such exception if shipped without a wording fix. **The correction this
rule needs — narrowing "every removal/scrub operation" to "every
operation whose contract is CONFIDENTIALITY" (redaction, scrub, a
recovered-base save) — is deliberately not made in this entry.**
Decision 022 explicitly declines to narrow a standing rule's scope
solo, asking for operator confirmation first (`ROADMAP.md` Open
operator question (v)); this section records the discrepancy rather
than resolving it unilaterally. See §5.12, below, for the settled
(non-wording) part of this same finding: whether annotation deletion
joins the forced-full-rewrite family at all.

### 5.10 A cross-reference-recovered document forces a full rewrite (R67 — third sibling of §5.2/R35 and §5.9/R58)

*(Added 2026-07-31, decision 013. **FLIPPED TO SHIPPED/ACTIVE 2026-08-01**
— Pass 13b (rebuild-by-scan recovery) shipped this session; the contract
below is now enforced code, not a forward-looking design note. R67 is now
IN FORCE. See `ROADMAP.md` Shipped, Pass 13b, for the acceptance numbers:
566 previously-failing real-world files now open (1,109-file corpus), zero
regression on the 2,907-file veraPDF corpus, `*-fail-*` reconciliation
complete.)*

§5.2 (R35, redaction) and §5.9 (R58, all removal/scrub) force a full rewrite
because incremental save structurally *preserves* superseded content.
Cross-reference recovery forces a full rewrite for a **different but equally
structural** reason: a document loaded via rebuild-by-scan had an **invalid
base cross-reference table**. An incremental append onto it would write a new
section whose `/Prev` points at a cross-reference section that does not
correctly exist — the appended file would be self-inconsistent and would fail
to reload. **Incremental-append onto a broken base is structurally
impossible, not merely undesirable.**

**Binding rule (R67):** a recovered document's save is a **mandatory full
rewrite** (`save_full`) emitting a fresh valid classic xref/trailer/
`startxref`. `save_incremental` on a recovered document is **refused by
name** (`WriteError::RecoveredBaseForbidsIncremental`). The recovered/rebuilt
status is flagged on the `Document` (a `recovery: Option<RecoveryReport>`
field), disclosed in the CLI + GUI, and counted (R20) — recovery is a
reviewable fact, never a silent repair (fuzzy-never-sneaky).

**Interaction with §5.6 "never normalize" (stated explicitly so a future
reader does not think recovery breaks R33):** §5.6 governs *clean
passthrough* objects — it forbids reformatting a file the operator loaded
intact. It does **not** bind a recovered file: the base was invalid, so
emitting a fresh normalized classic xref (`SectionShape::Classic { xref_stm:
None }` — the most compatible form) is the correct, honest output, not a
normalization violation.

**Why this never perturbs a clean file:** recovery triggers **exclusively on
the strict-load error path** (`document.rs::from_bytes` only invokes it when
`load_xref_chain` / `probe_header` returned `Err`). A file that loads cleanly
never enters recovery code, so the round-trip/minimal-diff invariant for
clean files (§5.1) is preserved **by construction**, not by policy. Full
record: `docs/decisions/013-xref-recovery.md`; standing rule R67.

**★ AMENDED 2026-08-07 — R67 IS UNCHANGED AND WAS NOT VIOLATED. The
failure was UPSTREAM of it, and the distinction is the useful part.**
`49dfe81` fixed a case where a recovered document's save produced a file
naming a `/Pages` object that was **not in it** — which looks at first
glance like a §5.10 breach and is not. R67 did exactly what it promises:
the save was a full rewrite emitting a **fresh, valid** classic
xref/trailer/`startxref`. **The xref was valid over an INVENTORY THAT WAS
SHORT.** `parse_object_at` requires `endobj` (§7.3.10), so an object whose
only damage was a missing four-byte keyword was never registered by
`confirm_candidates`, and R67 then faithfully emitted a correct table of
everything recovery had — including a catalog pointing at something it did
not.

**Stated as the reusable sentence, because it generalises past this
defect:** ***a valid cross-reference table over an incomplete inventory is
still a broken document.*** R67 guarantees the table is well-formed and
self-consistent; **it guarantees nothing about completeness**, and cannot,
because completeness is decided one level up in `recover.rs` /
`parser.rs`. Any future recovery work should read R67 as a **write-side**
contract only. Full record: §12's fifteenth 2026-08-07 entry;
`ROADMAP.md` *Shipped*, the `first defect the veraPDF gate found` entry.

### 5.11 In-place text editing is surgery-under-incremental-save, NOT a fourth forced-full-rewrite sibling (decision 014 — SHIPPED 2026-08-01, Pass 14.0–14.3 all COMPLETE)

*(Added 2026-08-01 as a forward-looking design note ahead of Pass 14.1;
FLIPPED to shipped/active 2026-08-01 on decision 015's filing — all four
Pass 14.x slices are now shipped (see `ROADMAP.md` Shipped). This section
records the actual module layout, mirroring how §5.10 was rewritten on
Pass 13b's ship.)*

§§5.2/5.9/5.10 (R35/R58/R67) form a **forced-full-rewrite family**: every
member exists because incremental save structurally *preserves* superseded
content, which is disqualifying for redaction, scrub, and a recovered-base
save alike. **In-place text editing is confirmed NOT a fourth member of
that family.** Editing is a content *change*, not a removal or a
recovery — it uses the project's **default** incremental save (R36/R70),
and prior text surviving in history is a disclosed, accepted consequence,
not a defect. Truly removing text remains Redaction's job (§5.2/R35);
conflating the two would either weaken redaction's absence guarantee or
force every keystroke through a full rewrite that drops revision history
for no security reason.

**Shipped module layout.** `crates/pdfce-core/src/text_edit/`:

- **`model.rs`** — the derived Run→Line→Block hierarchy over Pass 4's
  extraction (`Block`/`Line` with union `bbox`, `line_indices`, `column`;
  `BlockRecognitionOptions` — `column_overlap_ratio`,
  `paragraph_leading_ratio`, `indent_ratio`, `line_baseline_ratio`;
  `BlockDiagnostics` counting every inference, R72). `line_at`/
  `word_range_at`/`line_range_at`/`word_bounds` accessors (added Pass 14.3
  for caret/selection navigation).
- **`edit.rs`** — the advance-preserving REMOVE→REPLACE content-stream
  surgery (extends Pass 8.0's `redact.rs` interpreter, R69/R47), the
  inverse-encoding builder (Unicode→code, inverting Pass 4's §9.10.2 decode
  ladder), `FollowerDisposition` (same-line relayout past the original
  margin, disclosed), `EditReport.disclosures` (verbatim-surfaced
  refusals/warnings), and the R-INV-1..8 font-on-edit gate (R71) keyed on
  `GlyphSource` + glyph presence (decision 012). Also owns `EditSession` —
  the undo/redo command log — split as `plan_edit(...) -> EditPlan` /
  `plan_format(...) -> FormatPlan` (shared by both the free-function path
  and the session path) + `write_incremental`; `CommandKind::{EditText,
  FormatText}` (Pass 14.3 addition; `ReflowBlock` is Pass 15.1's addition,
  see §12's decision-015 entry) apply as ONE undo-able command each over
  the session's in-memory object graph, proven byte-identical to the free
  function for a single edit.
- **`format.rs`** — formatting-on-selection (Pass 14.2): size (`Tf`), fill
  colour (`rg`/`g`/`k`, storing the operator's actual chosen colour space —
  RGB/CMYK/gray — unlike Acrobat, which always stores `DeviceRGB`
  regardless of the picker mode shown), gated font-family/style change
  (re-encode into an available covering face, else refuse-and-disclose).
- **`vartext.rs`** — reused verbatim for reflow line-breaking (Pass 15.x);
  not itself part of the 14.x edit path but the shared line-breaking
  substrate.

**GUI (`pdfce-gui`, Pass 14.3):** `CanvasTool::TextEdit` — click→caret,
Shift-click→extend, double-click→word, drag→select; `TextEditState`/
`PendingEdit` in `main.rs`; live preview (mask + draft text + a dashed
"PREVIEW — not yet applied" tag), Accept/Reject buttons, the verbatim
disclosure/refusal strips, a read-only block-boundary review overlay, and
the property bar (size / colour-model / font, trust-labelled per R63).
`ui_text.rs` carries the ~30 new user-facing strings. Deferred, named
non-goals: triple-click/arrow-Home-End caret nav (accessor plumbing already
shipped, wiring deferred), split/merge/reorder of recognized blocks,
commit-on-focus-loss for the property bar (an explicit Apply button is used
instead).

The edit mechanism **is** content-stream surgery — the second sanctioned
page-content-rewriting operation after Pass 8.0's redaction interpreter
(R47's surgery-vs-overlay line), extended from REMOVE to REPLACE. It reuses
Pass 8.0's §9.4.4 advance-preservation machinery so un-edited same-line
text does not slide. The crux design call is **font-on-edit**: a keystroke
is applied only when the run's font can already supply the glyph (an
embedded program's existing glyphs, or a non-embedded font's full
bundled/supplied coverage per decision 012); a glyph an embedded *subset*
lacks is refused-and-disclosed by name, never faked or silently substituted
(R71). Block recognition is derived, counted, reviewable structure over
Pass 4's extraction output — never authoritative, never a silent re-layout
(R72; reflow itself is Pass 15.x, see below). An edit inside a
marked-content sequence preserves its BDC/EMC + MCID wrapper and discloses
staleness rather than corrupting the structure tree the way Acrobat's own
in-place edit is known to (R73) — minimal-diff turned into an
accessibility guarantee.

**Not a fourth forced-full-rewrite sibling, confirmed by the shipped
gates.** Every Pass 14.x ship re-verified `cargo tree -p pdfce-core` /
`-p pdfce-render` zero egui/eframe/winit/wgpu/glow (GUI-core separation
intact); the round-trip/R46 gate stays green for untouched objects; only
the edited content stream(s) (+ changed resource/font dict) are re-emitted.

**Forward pointer — reflow (FF-A) is a separate Pass family, not an
extension of 14.x's module boundary.** Decision 015 (2026-08-01) scopes
within-block offline reflow as `ROADMAP.md`'s ★ Pass 15.x, building a
`ReflowEngine`/`ReflowPreview` on top of this same `text_edit::model`/
`edit` substrate (15.0 read-only engine, 15.1 surgery +
`CommandKind::ReflowBlock`, 15.2 canvas UI). Reflow remains an *opt-in*
beside — never a replacement for — the default single-line relayout
described above (R75). Full design: `docs/decisions/015-ffa-within-block-offline-reflow.md`;
decision-log entry below.

**Forward pointer — FF-H (direct text-state formatting) re-scoped and
sliced by decision 019 (2026-08-03), a shared prerequisite for FF-C and
FF-B, not a peer extension of this section's surgery model.** FF-H's
own emission mechanism reuses this section's `set_ops`/`restore_ops`
pattern (Pass 14.2's `format.rs`) directly — `Tc`/`Tz`/`Ts` slot into
the existing `pre | set_ops | mid | restore_ops | post` splice with no
structural change. Two new architectural facts this decision
establishes, both binding on any future text-state-emitting code in
`pdfce-core`: (1) **`q`/`Q` are illegal inside `BT…ET`** (ISO 32000-1
§8.2 Table 51/Figure 9) — ambient text-state restoration after a
formatted run is therefore always **restore-by-value**, resolved by a
ladder in the same family as `TextColor::restore_bytes` (fill colour)
but with one more rung than that ladder needed (R88, corrected by
Amendment A below — see the decision-log entry for why a third
"available" case exists between "observed raw bytes" and "refuse"); (2)
**`Tc`/`Ts` are unscaled text-space quantities (§9.3) and are not
rescaled by `Tfs`** — pdfce's model stores them as a discriminated
`Absolute | Relative` quantity so a font-size change cannot silently
mis-scale a stored rise or tracking value (R89 — `Tf`/`Tfs` themselves
are explicitly OUT of this unification, per Amendment A item 3, to
avoid perturbing already-published glyph positions). A third finding
was a code-hygiene one rather than a spec fact: ambient
`Tc`/`Tw`/`Tz`/`Ts` state was independently tracked three times in
three different modules (`text_extract::page::TextState`,
`text_edit::edit::Walk`/`reflow_apply::BlockTextState`,
`vector::decompose::GState`) with zero shared publication.

**Narrowed by decision 019 Amendment C (Pass 19.2, `ebe35d8`):** the
"one shared consolidation" claim above is specifically about **the six
§9.3 text-state parameters** (`Tc`/`Tw`/`Tz`/`TL`/`Ts`/`Tr`) that R88's
ladder covers. Synthetic bold (§3.6/R90) introduced two more tracked
quantities — stroke line width and stroking colour — that are
**ordinary graphics state shared with path painting, not text state**,
and are tracked and restored separately from the `TextStateParams`
model rather than folded into it; a synthetic-bold run's stroke
settings would otherwise leak into later stroked *paths* on the page,
not just later text. Pass 19.2 also added `Tm`/`Tlm` tracking to
`text_edit::edit::Walk` (`BT` reset, `Td`/`TD`/`T*` derivation,
§9.4.4 advance accumulation, a `matrix_known` honesty flag, and a new
`Rec::EndText` variant) — needed for the absolute-`Tm`-required-for-
followers refusal gate (see below), and not anticipated by the
original decision text or by Amendment A's `Tf`/`Tfs` exclusion. So:
exactly one definition of the six text-state parameters in
`pdfce-core`, plus two separately-tracked shared-graphics-state
parameters, plus a separately-tracked text matrix — three distinct
things, not one, and the distinction is deliberate rather than an
oversight.

**Pass 19.0 SHIPPED (2026-08-03, `38fffad`) — this consolidation is now
built, not merely planned.** New `pdfce-core/src/text_state.rs` in two
layers: `TextStateParam`/`TextStateParams` (parameter identity +
resolved values, for arithmetic-only consumers) and
`AmbientValue`/`AmbientOrigin`/`AmbientTextState`/`AmbientRestoreError`
(values plus restore provenance, four-rung — see Amendment A below).
One `apply_operator` update rule is now shared by all three walks.
`GlyphProvenance` gains `text_state` (the resolved ambient parameters at
the glyph's show point) and `composite` (whether this glyph came from a
composite/synthesized run) fields, published for the first time —
previously dropped at provenance-construction time. `Tw` is tracked and
preserved but still **not** promoted to a direct authoring control by
this decision — its inter-word-distribution job stays with 15.1's
`TJ`-based reflow design, and any future promotion is gated behind a
corpus census (R91), never built speculatively. Synthetic bold/italic
(Tr 2 + `Tm` shear, R90) is new authoring surface, not a data-model
change, and does not warrant its own subsection here. `cargo test
--workspace` 1613 → 1643; zero new Cargo dependencies;
`fixtures/synthetic` roundtrip byte-identical (verified from a genuine
pre-change worktree build, not a `git stash` on an already-clean tree —
see `D:\dev\rag\rust\git_stash_on_clean_tree_makes_before_after_comparison_vacuous.md`
for why the first comparison attempt was vacuous). Full design:
`docs/decisions/019-ffh-spacing-scaling-synthetic-styles.md` +
Amendment A; decision-log entries below; Pass slicing (19.0
consolidation SHIPPED → 19.1 `Tc`/`Tz`/super-subscript IN PROGRESS →
19.2 `Ts`/synthesis → 19.3 GUI → 19.4 `Tw` conditional) in
`ROADMAP.md`'s ★ Pass 19.x entry.

**Pass 19.1 SHIPPED (2026-08-03, `603b051`) — `Tc`/`Tz`/superscript/
subscript authoring now built, not merely planned.** Rides the existing
`pre | set_ops | mid | restore_ops | post` splice with no structural
change; new `MetricSpec`/`ScriptPosition`/`ScriptMetrics` types,
`push_state_param` (the R88 ladder's application point). CLI:
`format-text --char-spacing`/`--h-scale`/`--superscript`/`--subscript`/
`--no-script`. Superscript/subscript ratios (0.60× size, +0.34×/−0.18×
rise, both of the BASE size per decision 019 Amendment B item B.3) are
pdfce's own choice, not an Acrobat parity claim (Acrobat's own values
are an unsourced gap in the parity catalog). **Decision 019 Amendment
B, filed same day, corrects three things found while building this
slice** — see the decision-log entry immediately below for the full
account: (1) the `Tz`×justify disclosure named the wrong mechanism (the
real cause is the formatted run's width delta, not a `TJ`-adjustment
rescale — the rescaled-`TJ` premise is true in general but the specific
`TJ` numbers carrying justify slack sit outside the edit's set/restore
wrap); (2) the `Ts`-rise spec-citation flag was verified NOT to be an
error in this document (only in `text_state.rs`, already fixed); (3)
R89's "`Tfs`" is now stated explicitly as the BASE size. Also fixed in
this slice: a live defect where `EditSession::format_text`'s own
hand-listed no-op predicate had drifted out of sync with the `FormatRequest`
fields Pass 19.1 added, making a spacing-only request a phantom no-op on
the GUI-facing `EditSession` path specifically (the CLI's `set_format`
path was unaffected) — replaced with `req.is_empty()` so the predicate
cannot drift again. Second occurrence of the same bug shape as Amendment
A.4's missing `q`/`Q` arms (a hand-maintained check mirroring a
structure's shape, rather than derived from it) — see `ROADMAP.md`'s new
standing rule R92.

**Pass 19.2 SHIPPED (2026-08-03, `ebe35d8`) — free-form `Ts` and
synthetic bold/italic now built.** New
`crates/pdfce-core/src/text_edit/synth.rs`: `StyleSynthesis` (the shared
policy type used by both `format.rs` in-place edit and `addtext.rs`
Add-Text), `SynthesisPath` (the *only* asymmetry between the two paths
is remedy *order*, per decision 019 §3.6), `SynthesisOffer`,
`OBLIQUE_TAN`/`BOLD_STROKE_RATIO` constants, `shear_into` (a true
matrix premultiplication, not a naive single-component overwrite —
tested against a pre-rotated matrix, where overwriting just the shear
component loses the lean entirely), `matrix_scale` (determinant-based,
so a shear does not perturb the derived bold stroke width), and
`detect` (reload-time re-detection of synthetic styles by byte
inspection, pdfce's own and other producers'). CLI: `--rise`,
`--bold-synthetic`, `--italic-synthetic`. The render-honours-`Tr
2`-and-sheared-`Tm` prerequisite named in the decision was confirmed
**empirically, by mutation testing** — a new
`crates/pdfce-render/tests/synthetic_style_render.rs` rasterizes built
fixtures and interrogates pixels, then deliberately breaks the renderer
three separate ways (drop mode-2 stroking, zero the `Tm` shear
component, zero the rise) and re-runs to confirm each mutation fails
exactly the tests it should — the standard the original by-inspection
prerequisite check should have met (see the decision-log entry for the
general methodology finding). **Decision 019 Amendment C filed**
(six corrections found while building this slice — the wrong restore
set named for stroking colour/line width, a narrower-than-written
absolute-`Tm`-required-for-followers refusal, a disclosed two-of-three-
factor bold-width formula, unanticipated `Tm`/`Tlm` tracking needed in
the authoring walk, two named unhandled conflicts refused by name
(rise-vs-toggle, synthetic-italic-vs-`--pin`), and Add-Text synthesis
flagged as not wired despite the shared type existing) — see the
decision-log entry immediately below for the full account, and
`docs/decisions/019-ffh-spacing-scaling-synthetic-styles.md` Amendment
C for the complete record. **No GUI code and no GUI verification this
Pass** (slice 19.3, the property surface, is a separate
`pdfce-ui-specialist` dispatch) — verified via the CLI oracle and a new
R85 case, exercising the same `EditSession` path the GUI will use.

**Pass 19.3 SHIPPED (2026-08-03, `74052d3`) — the GUI property surface
is now built, AND a defect that had silently disabled every property-
bar Apply since Pass 14.3 is fixed.** GUI slice: Option-B wrapper
(`StyleOutcome`/`StyleResolution`/`probe_synthesis`/
`preview_style_resolution` in `pdfce-core`, read-only and side-effect-
free — `preview_style_resolution` calls `gate_synthesis` up to three
times rather than re-deriving, proven byte-equal to a non-previewed
commit) plus the `pdfce-gui` property tree (`MetricUnit`/
`BaselineChoice`/`AmbientSnapshot`, 11 new `TextEditState` fields, five
`FormatOp` variants). **The headline finding is a data-contract defect
predating this decision entirely, exposed only because this slice
stopped discarding failed anchor lookups with `.ok()`.**
`GlyphProvenance::operator_span` (§9.4, published by the extraction
walk) names the span of the operator token ALONE; `text_edit::edit`'s
`OpRec` (the authoring walk's own record) names the OPERAND-INCLUSIVE
extent of the same operation. `find_anchor`'s pinned-request path
(`pin_names_operator`) compared the two spans for EXACT EQUALITY —
since the GUI always pins from published provenance, and the authoring
walk always records the wider span, **the two never matched, and every
GUI-issued `format-text`/`edit-text` Apply since Pass 14.3 refused
with `NoMatch` before reaching the surgery**, invisible in the running
application until this slice made the failure visible instead of
swallowing it. **Fix:** `pin_names_operator` now accepts either
convention — `pin.end() == r.end && pin.start >= r.start` — since two
operations in one content stream cannot share an end offset; a
regression test proves the relaxed match still DISCRIMINATES a
near-miss span (does not degrade into false-positive editing of the
wrong run). **Verified by mutation:** reverting to exact-equality
matching makes a new regression test fail; restoring the fix makes it
pass. Both doc comments that had independently asserted the two
conventions already agreed (`EditRequest::pinned_span`'s "matches the
same span," `text_edit/page.rs`'s "the surgery locates the operator by
exactly this span") are corrected in place — this is the architectural
fact this section previously stated incorrectly, now fixed at the
source. `cargo test --workspace` 1708 → 1722, 0 failed; `cargo tree`
re-verified clean; zero new Cargo dependencies. Full record: the Pass
19.3 Shipped entry, `ROADMAP.md` (top of Shipped), and the new standing
rule R93 (methodology: a code comment asserting a cross-module contract
is a claim, not evidence, even when two independent comments on both
ends of the contract agree — third occurrence of this failure shape in
this project, after decision 018's `refresh_pages` comment and the
`.gitattributes` ordering incident).

**The `Tw` census (decision 019 §3.3, gating slice 19.4) has been RUN
(2026-08-03) — Amendment E.** New out-of-workspace crate
`tools/tw-census` measured reachability, keyed by show operator
(`GlyphProvenance`'s `(ContentStreamRef, ByteSpan)`), over the Pass-11
render-fidelity corpus (4,012 files; 1,224 text-bearing after excluding
627 unloadable + 2,172 zero-show-operator files): **91.6% of show
operators / 97.4% of shown glyphs are on a simple (non-composite)
font** — the BUILD band (≥60%), not marginal. Slice 19.4 is cleared to
build but has **not started**; the engineer prioritized a real
document-loading defect this same census sweep found (see below).
**§3.2 reason 2 — that Type0/Identity-H composite embedding is "a
large and growing share" of documents, which would make a `Tw` control
inert on most files — is FALSIFIED on this corpus**: 81.2% of
text-bearing documents contain no composite run at all. The "growing"
half of that claim is untestable on this corpus (its files are older
PDF-tooling test suites, not a sample of recently-produced documents).
Full numeric record, sub-corpus breakdown, and both caveats (corpus
vintage; corpus composition — PDF-tooling test suites, not organic
documents): `docs/decisions/019-ffh-spacing-scaling-synthetic-styles.md`
Amendment E; `ROADMAP.md`'s continuation-67 In-progress entry.

**Same sweep found a pdfce document-loading defect, engineer-verified —
FIXED 2026-08-03, committed `409a6b5`:** 341 corpus files (8.5%)
refused to open at all with "page /Contents is neither a stream nor an
array of streams." Hand-verified NOT a correct rejection —
`fixtures/external/qpdf/qpdf/qtest/qpdf/add-contents.pdf` is a legal
file per ISO 32000-1 (`/Contents [ 4 0 R 5 0 R 6 0 R ]`, all eight
objects present, three intact text-bearing content streams) that pdfce
refused outright. **The originally-filed diagnosis was wrong in
mechanism**, not just incomplete: Pass 13b's rebuild-by-scan recovery
does not undercount objects — the scan correctly proposes all 8
headers, but object 5 was dropped at the strict-confirmation step with
"endstream not found where /Length points." The real cause:
`add-contents.pdf` is an **LF file converted to CRLF**, so every
`/Length` (measured on the LF form) is now short by one byte per
internal line, and the declared extent lands mid-content — the same
CRLF shift that broke `startxref`/`xref` in the first place (why
recovery engaged at all) also silently ate the content stream
recovery existed to save. One damage event, two symptoms.

**Two fixes, kept deliberately separate (both new, both opt-in/scoped
rather than changing default strict parsing):**
1. **`StreamLengthPolicy`** (`Strict` default, unchanged;
   `RecoverFromEndstream` re-derives a stream's extent from the
   `endstream` keyword — reachable only from existing recovery paths).
   This is not a heuristic: §7.3.8.2 *defines* `/Length` as the byte
   count "to the last byte just before the keyword `endstream`," so
   deriving the extent from the keyword reads the same normative
   sentence from its other end.
2. **Per-element `/Contents` degradation.** A `/Contents` array
   reference resolving to null contributes nothing and is dropped
   (§7.3.10's dangling-reference-is-null-object rule + Table 30's
   `/Contents`-is-optional rule — degrade the one element, not the
   document); a genuine *type* error (a non-reference array element,
   or a reference resolving to the wrong object type) is still
   `BadContents`, unchanged. A direct `null` (not an unresolved
   reference) is treated as absent per §7.3.9 and deliberately excluded
   from the `contents_unresolved` disclosure count, which is reserved
   for content that should have been present and was not. Counted and
   surfaced, never silent: `RecoveryReport.stream_lengths_recovered`
   (CLI + GUI recovery banner) and `Page.contents_unresolved` →
   `render::Diagnostics.contents_streams_unresolved` /
   `TextDiagnostics.contents_unresolved` (CLI stable line, GUI
   "unsupported items" detail list).

**★ AMENDED 2026-08-07 — A THIRD OPT-IN RECOVERY POLICY NOW EXISTS. The
numbered pair above is UNCHANGED and still describes the `/Contents`
defect's two fixes; this marker exists so a reader looking for *"which
parser policies can the recovery path turn on?"* does not stop at two.**

3. **`TerminatorPolicy`** (`Strict` default, unchanged;
   `RecoverAtNextHeader` accepts a definition whose body parsed cleanly
   but whose `endobj` is missing) — added `49dfe81`, reachable **only**
   from the rebuild-by-scan recovery path, exactly like
   `StreamLengthPolicy::RecoverFromEndstream`. **It is that policy's
   sibling by construction, not by analogy:** both are cases where the
   file contradicts §7.3, **which of two readings to believe is a POLICY
   choice rather than a spec choice**, and pdfce makes it an explicit
   parameter instead of a hidden default. The leniency accepts **only
   when the terminator is an integer**, so it cannot swallow trailing
   garbage, and the object's provenance is **`RecoveredFile`** — **R94's
   second instance**, and for the identical reason the `/Length` repair
   above needed the variant: the source bytes no longer agree with the
   value, so verbatim re-emission would carry the malformation into the
   saved file. Counted and surfaced, never silent:
   `RecoveryReport.missing_endobj_recovered` → CLI
   `missing-endobj-recovered=N` plus a prose NOTE citing §7.3.10.

**Why a third one was needed at all, in one sentence:** the `/Contents`
work fixed the case where a recovered object's **extent** was wrong;
`49dfe81` fixed the case where a recovered object's **terminator** was
missing and the object was therefore **never registered** — different
failure, same requirement that the repair be explicit, bounded, counted,
and provenance-invalidating. Full record: §12's fifteenth 2026-08-07
entry.

**The round-trip gate caught a bug in the fix itself.** The first
repair attempt corrected the recovered object's byte span but left its
stale `/Length` untouched; because the writer copies `Provenance::File`
objects verbatim, `save_full` produced a file pdfce itself could not
reload — a self-inflicted §5.10 round-trip violation, caught by the
gate that contract exists to enforce. Resolved by adding a third
`Provenance::RecoveredFile` variant to the already-`#[non_exhaustive]`
`Provenance` enum, meaning "bytes exist but no longer agree with the
value" — objects in this state are always re-serialized (recomputing
`/Length`) rather than copied verbatim. §5.10 is not weakened: the
mutation is deliberate, disclosed via the existing `RecoveryReport`
channel, and both existing verbatim-passthrough call sites already
excluded non-`File` provenance via `let-else`, so both were correct by
construction against the new variant. **Generalized as standing rule
R94** (`ROADMAP.md`, Standing rules): a repair that mutates a value
must invalidate any "these-bytes-are-verbatim" provenance attached to
it, or a downstream verbatim-copy path re-emits stale bytes beside a
corrected value. **R95** states the per-element `/Contents`-degrade
rule as binding (extends the R67 forced-full-rewrite-on-recovery
family with a read-side sibling: dangling optional/array-valued
content degrades in place, it never condemns the whole document).

**Result:** 289 of the 341 files now open with real content (verified
independently by re-running `tools/tw-census`: text-bearing documents
1,224 → 1,513, page-tree load failures 497 → 163, `BadContents` 341 →
1, zero regressions). Full numbers, sub-corpus breakdown, and gates:
`ROADMAP.md`'s `/Contents`-defect-fix Shipped entry (top of Shipped).

**Pass 19.4 SHIPPED (2026-08-03, `a1638f4`) — `Tw` direct-authoring
control now built; decision 019 / FF-H is COMPLETE end-to-end (all five
slices 19.0–19.4 shipped).** Rides the existing `push_state_param`
four-rung ladder and `pre | set_ops | mid | restore_ops | post` splice —
no new authoring path. `FormatRequest::set_word_spacing` shares the same
`MetricSpec::{Absolute, Relative}` model `Tc` uses (Pass 19.1), resolved
against the BASE font size per Amendment B item B.3; `FormatError::
WordSpacingComposite`; `FormatReport::word_spacing_change` +
`word_spacing_affected_codes`. `Tw` enters the §9.4.4 advance via
`eff_tw` and joins the existing justify-invalidation trigger set
(`disclosure_justify_invalidated`, Pass 19.1's mechanism, not a second
path). CLI `--word-spacing V[pt|em]`, generalizing `parse_char_spacing`
into `parse_text_metric` so `Tc`/`Tw`/`Ts` share one grammar and one
error voice. GUI row live for simple-font runs; the composite strip
stays the existing read-only R83 presentation.

**Amendment F filed** (three findings this slice's build surfaced,
none anticipated by the original decision or Amendments A–E — full
account in `docs/decisions/019-ffh-spacing-scaling-synthetic-styles.md`
Amendment F): (1) **the composite refusal (R91) was UNREACHABLE as
originally implemented** — `match_run` filters every composite run to
`NoMatch` (its decoded text is always empty) before the font-aware gate
ever runs, so R91 would have shipped as referenced, documented, never-
executed dead code; fixed by hoisting font resolution above `match_run`,
verified by a test proving the gate now fires AND a second test proving
the OTHER three controls stay live on the same composite run (a
specific capability gate, not a blanket composite refusal). Generalized
as `D:\dev\rag\rust\dead_guard_clause_behind_a_filter_the_guarded_case_cannot_pass.md`.
(2) **A named limit:** the fixed refusal is reachable via the pinned-span
path but not via CLI `--find` (composite-run text search finds nothing,
so the CLI reports "not found in an editable run," a less specific
message than the decision describes) — closing this needs composite
decoding in the authoring walk, FF-E's scope, not this slice's. (3)
**`Tw` is multiplied by `Th`** (§9.4.4, same basis as `Tc`) — the
decision names this only as a reason `Tw` is an awkward control, never
as something needing disclosure; the word-spacing disclosure now quotes
the effective delivered value whenever `Th ≠ 1`. Filed to
`C:\personal_rag\pdf\lesson_20260803_word_spacing_multiplied_by_horizontal_scaling.md`.
Also recorded, not a correction: `Some(0)` affected-spaces is emitted
and disclosed as a real answer (a `Tw` set on a code-32-free run is
genuine, legitimate state), and Amendment A.1's fourth restore rung
needed no change to correctly handle `"` setting `Tw`/`Tc` as a
side-effect of showing text — its first concrete, load-bearing test.
`cargo test --workspace` 1738 → 1756, 0 failed; zero new Cargo
dependencies; round-trip proven non-vacuous by two binaries differing in
both MD5 and size. Full record: `ROADMAP.md`'s Pass 19.4 Shipped entry
(top of Shipped).

**MILESTONE — decision 019 / FF-H is COMPLETE end-to-end.** This closes
item #3 ("finish off all the text handling stuff") of the operator's
four-item priority sequence as far as FF-H's own scope goes (FF-C and
FF-B remain unscheduled, per this decision's own Q3 build order).

Full design, the four-case font-on-edit matrix, the fast-follow ladder
(FF-A offline reflow ladder through FF-H spacing/synthetic-styles — FF-A/
FF-B boundary amended by decision 015, FF-H re-scoped by decision 019,
see below), and the six standing rules (R69–R74) are in
`docs/decisions/014-acrobat-text-editing.md`; Pass slicing (14.0
read-only model → 14.1 edit+relayout+font-gate → 14.2 formatting → 14.3
canvas UI) and its Shipped records are in `ROADMAP.md`.

### 5.12 Annotation deletion is surgery-under-incremental-save, NOT a fifth forced-full-rewrite sibling (decision 022 — DECIDED, Pass 22.0 unbuilt)

*(Added 2026-08-04, `pdfce-librarian` continuation 80, as a
forward-looking design note ahead of Pass 22.0's build — same
disposition §5.11 had ahead of Pass 14.1, per §5.11's own header note
above. This section records the SETTLED half of decision 022 §5.4's
finding — family membership — and separates it from the UNSETTLED half
— R58's exact wording — which stays flagged, not fixed, at §5.9,
above, pending Open operator question (v).)*

§§5.2/5.9/5.10 (R35/R58/R67) form a **forced-full-rewrite family**:
every member exists because incremental save structurally *preserves*
superseded content, which is disqualifying wherever the operation's own
contract is confidentiality. **Deleting a pdfce-authored annotation
(`EditSession::delete_annotation`/`delete_dimension`, decision 022 §6.1,
Pass 22.0) is confirmed NOT a fifth member of that family**, by the same
reasoning §5.11 already applied to in-place text editing: deleting an
annotation is a removal, but its contract is "this is no longer in the
current revision," never "this must be provably unrecoverable." The
prior revision remaining reachable through undo/version history is that
mechanism working as intended, not a defect the way a redaction that
leaves the redacted text recoverable would be. Truly making content
unrecoverable remains Redaction's job (§5.2/R35) and, where it applies,
Sanitize/scrub's (§5.9/R58); conflating annotation deletion with either
would force a routine "remove this ce dimension" action through a full
rewrite that drops revision history for no security reason the
operation ever promised.

**Exactly which objects change on an annotation delete, per decision
022 §5.1 — at most four, and the fourth only for a pdfce-authored ce
dimension:**

1. The `/Annots` container — indirect-array XOR inline, never both
   (`EditSession::remove_from_annots`, already shipped and reused
   verbatim, no new logic).
2. The annotation dictionary — a `Removal`.
3. The `/AP` `/N` stream object — a `Removal`, resolved BEFORE any
   mutation (the `delete_redaction_mark` pattern).
4. The catalog `/PieceInfo` sidecar (ce dimensions only) — via the
   existing `catalog_dimension_write`, in the SAME command (R113).

**Zero page content streams change** — this is not content surgery at
all, which is a cheap, machine-checkable distinguishing claim
(`tools/content-identity` reporting 0 for `annot-delete`/
`dimension-delete`, decision 022's acceptance criterion A4). This
places annotation deletion architecturally closer to the R107 family
(precisely-named object allocation/removal, proven by
object-id-disjointness, not a runtime guard) than to R35/R58/R67's
content-stream-rewrite family, despite both being "delete" operations
in the colloquial sense.

**What this section does NOT settle:** whether R58's own binding TEXT
should be corrected to name this exception explicitly (`ARCHITECTURE.md`
§5.9, above; `ROADMAP.md` Standing rules R58; Open operator question
(v)) — that is a standing-rule-wording call decision 022 itself declines
to make solo, and this librarian is not making it here either. This
section settles only the underlying architectural question (family
membership), which is not in genuine dispute — three independent
instances (`delete_object`, `delete_redaction_mark`, and now
`delete_annotation`) already agree.

## 6. Packaging: single-folder portable

- **Platform scope (decided 2026-07-30, decision 003 §4.1 — no longer
  a default):** v1 ships **Windows 10/11 x86_64 only**, as a
  deliberate scope decision. The codebase stays platform-clean at all
  times (no `#[cfg(target_os)]` in `pdfce-core`/`pdfce-render`, rule
  R10), verified continuously by cross-target `cargo check` CI for
  macOS-arm64 and wasm32 — a compile signal, never a support claim
  (rule R9). See `docs/decisions/003-distribution-posture.md` for the
  full reasoning, the macOS/Linux gating triggers, and the
  CLI-first-via-musl rule if Linux ever ships.
- No installer. Build produces `pdfce.exe` (Windows first target) plus
  whatever DLLs/assets are needed, all in one output folder.
- **Payload/user-state partition (decision 003 R15, binding from the
  first Pass that persists anything):** the distribution folder is
  split into replaceable payload (binaries, assets,
  `THIRD_PARTY_LICENSES.md`, README) and user state (settings,
  recents, later OCR data) in a clearly named location — because the
  documented update procedure is "replace the folder," and replacing a
  folder destroys whatever the user kept in it. User state never sits
  loose among the binaries; the update instructions name exactly which
  files to keep. The packaging smoke test verifies the partition.
- No registry writes, no `%APPDATA%` requirement for the app to run
  (per-user settings/recents may still use a conventional config dir,
  but the app must run read-only-folder-clean with no config present).
- Verify every packaging pass with a **real smoke test**: zip the
  output folder, unzip it to an unrelated path (e.g. a fresh temp
  dir), launch from there with no prior install step, confirm it
  opens and renders a fixture PDF. This is the packaging equivalent of
  MatExtractor's "smoke-import MainWindow" rule — don't claim a
  packaging pass done without actually running the copied folder.

## 7. CLI capabilities (`pdfce-cli`)

pdfce ships a real command-line interface alongside the GUI, not as a
debug afterthought. Design points:

- **Same crate-separation discipline as the GUI.** `pdfce-cli` depends
  on `pdfce-core` + `pdfce-render` exactly like `pdfce-gui` does, and
  is held to the same zero-GUI-dependency-in-core invariant (§3) — the
  CLI's existence is itself proof that invariant is doing its job:
  two completely different front ends, one shared core, no logic
  duplicated.
- **Subcommand shape** (`clap`-based, final surface scoped alongside
  each feature's own Pass — see `docs/ROADMAP.md`): one subcommand per
  batch operation, e.g. `pdfce-cli merge a.pdf b.pdf -o out.pdf`,
  `pdfce-cli extract-pages in.pdf 3-7 -o out.pdf`, `pdfce-cli
  bates-stamp *.pdf --start 1 --format "DOC-{:06}"`, `pdfce-cli
  to-pdfa in.pdf --level 2b -o out.pdf`, `pdfce-cli validate-pdfa
  in.pdf` (prints a conformance report, non-zero exit on failure —
  scriptable in CI/document-pipeline contexts), `pdfce-cli sign in.pdf
  --cert cert.p12 -o out.pdf`, `pdfce-cli render-page in.pdf 3 -o
  page3.png --dpi 150`.
- **Exit codes matter.** Since this is meant to be genuinely scriptable
  (unlike Acrobat, which has no real CLI), follow normal Unix
  conventions: `0` success, non-zero on any failure, with a specific,
  documented meaning per non-zero code where it's useful for a calling
  script to distinguish failure modes (e.g. "input not found" vs
  "encrypted, no password given" vs "PDF/A validation failed").
- **Same round-trip / redaction / fuzzy-never-sneaky invariants apply.**
  A CLI redact command must truly remove content, same as the GUI
  path (§5); a CLI OCR command's output is still a hint the caller
  chooses to apply, not silently baked into the saved file, unless an
  explicit `--apply` (or similarly unambiguous) flag says otherwise.
- **Packaging**: `pdfce-cli.exe` ships in the same single output
  folder as `pdfce-gui`'s executable — one portable folder, two
  entry-point binaries, both zero-install. The packaging smoke test
  (§6) covers both.

## 8. Code style & public API design

`pdfce-core`'s public API (and, downstream of it, `pdfce-cli`'s
argument/output design) follows the official Rust ecosystem
conventions, not an invented house style:

- **Formatting** — the Rust Style Guide, enforced via `cargo fmt`.
- **API design** — the Rust API Guidelines checklist (naming
  conventions, trait derives, error-type design, documentation,
  predictability, type safety).

Full condensed reference, kept up to date as a cross-project resource
(useful to any future Rust project, not just pdfce):
`D:\dev\rag\rust\rust-style-guide-and-api-guidelines.md`. This is a
binding engineering discipline, not a style preference — see
`.claude\agents\pdfce-engineer.md` §"Code style & API design
discipline" for the enforcement mechanics (`cargo fmt --check` and
`cargo clippy -- -D warnings` clean before any Pass ships).

## 9. Open-source dependencies & attribution

pdfce builds on the existing Rust/OSS ecosystem rather than
reinventing every primitive — see `docs/PRIOR_ART.md` for the
survey/decision record and `docs/LEGAL.md` §6 for the binding
licensing discipline (permissive-vs-copyleft classification, the
mandatory per-dependency license check, and why pdfce's own license
gates which prior art is even usable). Attribution for whatever's
actually adopted is **generated**, not hand-maintained — `cargo-about`
produces `THIRD_PARTY_LICENSES.md` from the real `Cargo.lock`,
regenerated at every packaging pass (§6).

**pdfce's own license is MIT (decided 2026-08-01, `LEGAL.md` §1; see
§12 decision log).** `LICENSE` (repo root) + `license = "MIT"` in
`Cargo.toml` `[workspace.package]`, inherited by all four member
crates via `license.workspace = true`. Every current dependency is
permissive (verified against `THIRD_PARTY_LICENSES.md`), so this
decision required no dependency rework. **Consequence: GPL/AGPL prior
art (MuPDF, Poppler, Ghostscript) is now categorically, permanently
excluded as a real dependency** — reference-only (architecture/
algorithms studied, never linked or copied), per `LEGAL.md` §6.1.

**First conjunctive-attribution dependency: `jpeg-encoder` 0.7.1
(2026-08-08, `Pass 48.2`, see §12's thirty-fifth-filing entry).**
`(MIT OR Apache-2.0) AND IJG` — permissive at the grant level, with an
IJG attribution NOTICE condition that applies unconditionally alongside
it (an `AND`, not a caller's choice between license terms). Accepted on
direct operator ruling; the attribution sentence is generated into
`about.hbs`, never hand-written. pdfce's first shipped image ENCODER,
under standing rule R28's own named exception (`ROADMAP.md`).

## 10. Adversarial input hardening & fuzzing

`pdfce-core` parses files from the public internet by design — every
PDF it opens must be treated as **untrusted, potentially adversarial
input**, not just "possibly malformed." This is a real gap identified
2026-07-23: the project justified choosing Rust partly on this basis
(§2) but had never written down what that actually requires structurally.

### 10.1 Resource-limit guards (decompression-bomb defense)

Every filter decoder (`FlateDecode`, `LZWDecode`, `CCITTFaxDecode`,
`JBIG2Decode`, `DCTDecode`, `JPXDecode`) **must** enforce a maximum
decoded-output-size cap before/while decoding, not just check the
result afterward — a few KB of compressed input can expand to
gigabytes (classic zip-bomb pattern), and PDF's filter chaining
(e.g. `ASCII85Decode` → `FlateDecode` → raw image data) can compound
this. Concretely:

- Every decoder takes an explicit output-size ceiling (a sane default,
  overridable) and returns an error rather than continuing once
  exceeded — never silently truncate, never allocate unbounded.
- Object/dictionary nesting (page tree, `Kids` arrays, `Resources`
  inheritance chains, annotation appearance-stream references) needs
  cycle detection and a depth cap — a maliciously crafted circular
  reference must fail cleanly, not hang or stack-overflow.
- Content-stream interpretation (path construction, clipping, nested
  `Form XObject`/`q`/`Q` graphics-state pairs) needs an operation-count
  or time budget per page — pathological but syntactically valid
  content streams (e.g. millions of degenerate path segments) must not
  be able to hang the renderer indefinitely.
- Object counts / xref table size get a sanity ceiling too (a
  100 MB file claiming 500 million objects is lying).
- **Concrete instance (added 2026-07-30, Pass 1.1 slice): recursive
  Form-XObject execution (`Do`) gets `MAX_XOBJECT_DEPTH` = 64,
  corpus-measured, not guessed.** An initial guard of 16 (intuition)
  overflowed on exactly one of 2,914 veraPDF/PDF-Association corpus
  files — a **conformant** 32-deep chain
  (`veraPDF-corpus/PDF_A-1b/6.1 File structure/6.1.12 Implementation
  limits/veraPDF test suite 6-1-12-t08-pass-c.pdf`, objects 19–50).
  Annex C sets no form-nesting limit and PDF/A §6.1.12 forbids a
  reader from imposing Annex C limits anyway. Raised to 64 (2× the
  deepest conformant structure measured); corpus-wide overflows are
  now 0. This is the SECOND guard in this project caught by the
  veraPDF §6.1.12 implementation-limits suite (the first was
  `MAX_TOKEN_LEN`) — see the `ROADMAP.md` standing rule requiring
  every new resource guard to be run against that suite specifically
  before shipping.
- **★ Concrete instance (added 2026-08-07, `0df6158`) — the first guard
  in this codebase motivated by a SPEC OBLIGATION rather than by a bomb,
  and the first on the WRITE side: `save::MAX_REWRITE_OBJECT_NUMBER` =
  8,388,607.** §7.5.4 obliges a single-section full rewrite to emit one
  cross-reference entry per object number **from 0 to the file's
  maximum**, so `save_full`'s hole-filling loop is **O(largest object
  NUMBER), not O(object count)** — and the largest number is chosen by
  whoever wrote the input. pdfium's **1.2 KB** `bug_455199.pdf` names
  `2147483648 0 obj` (2³¹) and therefore asks for **2,147,483,649**
  entries: measured at **~27 MB/s of steady allocation, CPU pinned** —
  about an hour and 40 GB. **Not an infinite loop, which is what made it
  survive:** it looks like progress the whole way down, so a liveness or
  progress check cannot detect this class and **only a wall-clock budget
  can**. In the GUI it is an unrecoverable freeze with no error, no
  cancel and no save. **Refused by name (R27)** rather than complied with
  — a sparse table would be malformed (§7.5.4) and compact renumbering
  would break §5's per-object byte-identity contract. The value is
  **sourced from Annex C Table C.1's maximum indirect objects (2²³ − 1)**,
  not guessed, and **deliberately not clamped to the object COUNT**
  (a sparse-but-small file with one enormous number is exactly the
  adversarial shape). The same table caps a PDF **integer** at
  2,147,483,647, so that file's object number is **one more than the spec
  permits** — unrepresentable, not merely improbable, which is why the
  guard refuses nothing a conforming producer can write. **Reading is
  unaffected**; `inspect` and `extract-text` both succeed on the file.
  ~~**The §6.1.12 implementation-limits run this bullet's own standing rule
  requires is OWED for this guard**~~ — **★ DISCHARGED 2026-08-07: the run
  was performed.** All **44** files of the four `*6.1.12*` directories
  (`Isartor test files/PDFA-1b`, `PDF_A-1b`, `PDF_A-2b`, `PDF_A-4`) swept at
  `--mode full`: **0 hangs, 0 regressions, 0 REFUSED.** **Shown non-vacuous
  in the same breath, which is the half that matters:** *"0 refused"* reads
  identically to *"the guard cannot fire"*, so the guard was separately
  verified **firing** on `bug_455199.pdf`. **Fires on a real file, silent
  across all 44 — two-sided.** The third validation this suite has produced
  and the first that **passed** rather than exposing a bad bound (the two
  before it, `MAX_TOKEN_LEN` and `MAX_XOBJECT_DEPTH`, were intuition-chosen;
  this one came from Annex C Table C.1). See §12's seventeenth 2026-08-07
  entry for the owed record and the eighteenth for the discharge.

### 10.2 Fuzz-testing (required, not optional, before Pass 1 ships)

Set up a `cargo-fuzz` target against the tokenizer/object-parser as
part of Pass 1 (add explicitly to its acceptance criteria in
`docs/ROADMAP.md` if not already there by the time Pass 1 starts).
Minimum scope for the first fuzz target: raw byte-stream → tokenizer
→ COS object parser, asserting only "never panics, never hangs past a
bounded timeout, never allocates past a bounded ceiling" — not
semantic correctness (that's what the fixture-based tests in §5/§9
cover). Expand fuzz targets to each filter decoder as they're
implemented. Treat any fuzz-discovered crash as a release blocker for
the Pass that introduced the vulnerable code path, not a "file it and
move on" backlog item.

### 10.3 Where this lives in the codebase

Guard logic (size ceilings, depth counters, timeouts) belongs in
`pdfce-core` itself — not bolted on as a wrapper in `pdfce-gui`/
`pdfce-cli` — so both front ends (and the future WASM fork) inherit
the same hardening automatically. Document each guard's default limit
and rationale in the doc comment of the function it guards, per the
documentation-first rule; a reader should understand *why* the number
is what it is (e.g. "1 GiB default output cap — larger than any
legitimate single decoded PDF stream this project has seen, small
enough that hitting it can't exhaust a typical machine's memory").

**Amendment (2026-07-30, Pass 1.1 slice):** the principle stated above
("guards belong in `pdfce-core`") is precise for **parse-time**
recursion (page-tree walk, xref/ObjStm cycles) but not for
**render-time** recursion — a Form XObject's recursive `Do` execution
happens inside `pdfce-render`'s content-stream interpreter (§3's
implementation note), which `pdfce-core` has no visibility into (it
only ever sees one content stream's tokens at a time, never resolves
`Do` itself). `MAX_XOBJECT_DEPTH` therefore lives in `pdfce-render`.
Both front ends (and the WASM fork) still inherit it automatically,
because both depend on `pdfce-render` for any rendering at all — the
"automatically inherited by every front end" property is what actually
matters, not which of the two GUI-agnostic crates holds the constant.
General rule going forward: a guard against adversarial input lives in
whichever of `pdfce-core`/`pdfce-render` actually performs the
recursive/expanding operation being guarded.

## 11. Undo/redo architecture

Identified as a real design gap 2026-07-23: the UI standing rule
"every edit is undoable" (see `pdfce-ui-specialist.md`) was never
reconciled with the round-trip/minimal-diff invariant (§5) — the two
interact in a way that needs an explicit mechanism, not just a UX
promise.

### 11.1 The core design: command log over the in-memory object graph, diffed at save time

- Every edit the user makes is represented as a small command object
  (`PendingEdit` or similar) with `apply()` and an inverse/`revert()`,
  operating on `pdfce-core`'s **in-memory** `Document` object graph —
  never on file bytes directly, and never on the saved file at all.
  The undo stack holds these commands.
- The **original loaded byte buffer / object graph is retained
  unmodified** as the "base revision" for the life of the open
  document (this is already required by §5 for lazy round-trip
  passthrough — undo reuses the same retained state, doesn't add a
  new one).
- Undo/redo operates **entirely pre-save**: hitting Undo reverts the
  in-memory graph via the command's inverse. It has no relationship to
  what's on disk until the user actually invokes Save.
- **Critical rule**: the "dirty set" (which objects actually differ
  from the base revision, i.e. what an incremental save must include)
  is computed as a **structural diff against the base revision at save
  time** — it is *not* the union of every object any command ever
  touched during the session. If a user edits an object and then
  undoes that specific edit before saving, that object must **not**
  appear in the incremental update, because compared to the base
  revision nothing net changed. Tracking "was this object touched
  by history" instead of "does this object currently differ from
  base" would silently violate the minimal-diff promise the moment
  undo is involved — this is exactly the subtle bug this section
  exists to prevent someone from introducing.
- Redo stack is invalidated (cleared) the instant a new edit is made
  after an undo — standard editor behavior, stated here for
  documentation-first completeness, not because it's subtle.
- Bound the undo history (a configurable max operation count) rather
  than keeping it unbounded — large documents with long editing
  sessions shouldn't accumulate unbounded command-object memory.
  Acrobat itself bounds undo; matching that expectation is fine.
- **CORRECTION (2026-08-03, Pass 17.1) — at most one `ObjectWrite` per
  object id per command.** The command model above assumes a command's
  writes compose; in practice `EditSession` applies a command's
  `ObjectWrite`s in sequence against the PRE-command state, and nothing
  commits mid-command — so a SECOND whole-dictionary `ObjectWrite` to an
  object id already written earlier in the SAME command **replaces**
  the first rather than merging with it. Found via `flatten_fields`,
  which issued three whole-dictionary writes to the SAME page object in
  one command (`/Contents`, `/Resources /XObject`, `/Annots`), each
  cloned from the identical pre-command page dict; the `/Annots` write
  landed last and silently discarded the `/Contents`/`/Resources`
  changes, so every flattened form lost its burned-in visible values
  while still reporting correct counters (`fields_flattened`/
  `widgets_burned`/`pages_touched`). No existing test caught this
  because none rendered the result — R85 (`ROADMAP.md` Standing rules)
  closed exactly this class of gap. **Binding rule going forward:** a
  command that needs to touch the same object's `/Contents`,
  `/Resources`, AND `/Annots` (or any other combination) in one step
  must accumulate ONE merged dictionary write per id, never issue N
  separate whole-dict writes to the same id within a command. Other
  multi-write commands are owed the same audit — not yet performed
  exhaustively as of this entry. Full record: `ROADMAP.md`'s Pass
  17.1/17.2 Shipped entry.

### 11.2 Redaction is the deliberate exception, and only after save

Redaction's true-content-removal behavior (§5 corollary) is
undo-able **like any other edit, right up until the document is
saved**. Once a redaction has actually been written to disk (the
underlying content genuinely gone from the saved bytes), that save is
not reversible by "Undo" in a later session — there is no data left
in the file to restore. This matches real-world expectation (redact +
save = permanent) and is exactly why the UI standing rule requires an
explicit, honest confirmation dialog for redaction specifically
(`pdfce-ui-specialist.md`) — the operator needs to understand *before*
saving that this is the one edit type Undo can't rescue them from
after the fact.

**Cross-reference added 2026-07-31 (Pass 3.0, decision 007 W2/R35):**
the *save-side* half of this exception is now specified in **§5.2**.
Redaction must force a **full rewrite** and must **refuse incremental
save**, because incremental save structurally preserves superseded
content — the old bytes of every replaced object stay in the file by
construction (§7.5.6). Without that rule, a redaction saved in pdfce's
*default* mode would leave the redacted content trivially recoverable,
and the confirmation dialog this section describes would be promising
something the writer did not deliver. §5.5 records the resulting
conflict with signed documents, which is a genuine operator either/or.

**Correction cross-reference added 2026-07-31 (Pass 3.1):** forcing a
full rewrite is NOT by itself enough to make redacted content
"genuinely gone from the saved bytes" when the redacted object lives
in an object stream — containers carry through verbatim in both save
modes, so the Redaction Pass must also rewrite/decompose the
containers holding redacted objects. See §5.7.

### 11.3 Snapshot fallback for bulk structural edits

The command-pattern model is the default for content-level edits (text,
annotations, form fields, single-page operations). For bulk structural
operations where per-item commands would be awkward (e.g. reordering
50 pages in one drag operation), a coarser "before/after page-order
snapshot" command is an acceptable specialization of the same pattern
— still one undo-stack entry, still diffed against the base revision
at save time via the same mechanism. Don't invent a second, parallel
undo system for this case.

### 11.4 Scope for Pass planning

Read-only Passes (Pass 1) need none of this. **The first Pass that
introduces any editing capability must build the command-log/undo-
stack mechanism as part of that Pass, not after** — retrofitting undo
onto edit code that was written assuming direct mutation is
significantly more expensive than designing it in from the first edit
feature. Flag this explicitly when `docs/ROADMAP.md` scopes the first
editing Pass.

### 11.5 Implementation record — the overlay design (Pass 3.1, 2026-07-31)

*(§11.4's obligation bound at Pass 3.1 — the first editing Pass — and
was honored: the mechanism below shipped in that Pass, not after.
This section records the shape actually built, so §11.1's design
prose and the code stay reconcilable.)*

- **`EditSession` command log** (`crates/pdfce-core/src/edit.rs`,
  1,608 lines): every edit is a command with apply/revert, exactly as
  §11.1 specifies — operating on an **overlay** above the base
  revision, never on the base object graph and never on file bytes.
  The base revision (buffer + parse) stays untouched for the life of
  the open document; the overlay holds only the objects that
  currently differ.
- **The dirty set is derived, not accumulated:** at save time the
  overlay yields a `DirtySet` (replacements + trailer patch +
  `changes_content`) as a diff against the base revision. An
  edit that has been undone leaves no trace in the overlay, so it
  cannot appear in the save — §11.1's "union of every command ever
  run" bug is structurally unexpressible, and executably pinned:
  **edit → undo → save is byte-identical, 2,897/2,897 corpus files
  (100%)**, plus dedicated fixture tests including a 12-command
  history and undo → redo → save.
- **One writer path:** both save modes take `&DirtySet`;
  `DirtySet::empty()` is Pass 3.0's identity writer as a strict
  pinned subset (§5.7).
- **Undo granularity matches operator intent:** the GUI applies edits
  on button press, not per keystroke — one undo step per intent, so
  the stack holds meaningful operations (a deliberate Pass 3.1
  decision, §12 continuation-18 entry).
- Redo invalidation on new-edit-after-undo behaves as §11.1 states.

## 12. Decision log

Append-dated entries here whenever an architectural decision is made
or revised. Don't rewrite history — if a decision changes, add a new
dated entry noting the change and why; leave the old entry in place
with a forward pointer.

- **2026-07-23** — Project bootstrap. Language: Rust. GUI: egui/eframe
  recommended (confirm at Pass 0). Target parity product: Adobe
  Acrobat Pro. PDF-spec RAG location: `D:\Dev\Rag-Specialized\PDF_Spec\`.
  Agents created: pdfce-engineer, pdfce-librarian, pdfce-spec-librarian,
  pdfce-ui-specialist.
- **2026-07-23 (same-session amendment)** — Added `pdfce-cli` as a
  first-class crate (§3, §7) per explicit user request: pdfce ships
  CLI batch capabilities from the start, not just a GUI. Added Rust
  Style Guide + API Guidelines as a binding, cross-project-referenced
  engineering discipline (§8), backed by a new reference file at
  `D:\dev\rag\rust\rust-style-guide-and-api-guidelines.md`. Corrected
  the cross-project-knowledge-base plan: Rust/egui/wgpu findings
  belong in the existing `D:\dev\rag\rust\` / `D:\dev\rag\egui\`
  Cross-project Tool RAG (registered in the user's global CLAUDE.md),
  not a new `personal_rag/rust` subject — `personal_rag/pdf` remains
  correct as-is for PDF-domain-specific empirical findings.
- **2026-07-23 (same-session amendment 2)** — Added a second reference
  RAG, `D:\Dev\Rag-Specialized\Acrobat_Features\`, cataloging Adobe
  Acrobat Pro's feature set (capability/behavior/edge-cases/limits)
  to ground `ROADMAP.md` acceptance criteria — explicitly excludes
  Acrobat's GUI mechanics, since pdfce's UI is designed independently
  by `pdfce-ui-specialist`. New agent `pdfce-acrobat-librarian` owns
  it. Also established a binding cross-RAG format rule: every RAG
  this project builds or writes to (`PDF_Spec`, `Acrobat_Features`,
  `D:/dev/rag/rust`, `D:/dev/rag/egui`, the future `personal_rag/pdf`)
  is written for LLM consumption only, not human reading — dense,
  schema-consistent, no narrative padding.
- **2026-07-23 (same-session amendment 3)** — Added §9, open-source
  dependencies & attribution, per user request to survey existing OSS
  prior art and ensure proper crediting. New `docs/PRIOR_ART.md`
  (research/decision record) + `LEGAL.md` §6 (binding licensing
  discipline: permissive-vs-copyleft classification, per-dependency
  license check before any `Cargo.toml` addition, generated
  `THIRD_PARTY_LICENSES.md` via `cargo-about` rather than hand-
  maintained attribution). Three research passes launched to survey
  core PDF crates, supporting codec/font/crypto crates, and existing
  full OSS PDF tools; findings pending synthesis into `PRIOR_ART.md`.
- **2026-07-23 (same-session amendment 4)** — Research synthesized
  into `docs/PRIOR_ART.md`. **Open question flagged, not yet decided:**
  `oxidize-pdf` (MIT) may already cover most of pdfce-core's target
  scope — needs a dedicated audit (round-trip fidelity, signature-safe
  incremental saves, PAdES signing) before Pass 1 locks in a
  from-scratch `pdfce-core`. Confirmed: pure-Rust filter answers now
  exist for JBIG2/CCITT/JPX (`hayro-*` crate family) — the "problem
  filter" set from §7's plan is resolved. Confirmed gap: no Rust crate
  does signature-safe incremental saves or PAdES signing — build from
  `cms`+`x509-cert`+RustCrypto. `tiny-skia` selected as the
  `pdfce-render` rasterizer. MuPDF and Ghostscript flagged as AGPL-3.0
  licensing landmines — never link without a deliberate, user-
  confirmed decision. Competitive landscape confirmed clear (§1).
- **2026-07-23 (Pass 0 — workspace bootstrap)** — Pass 0 shipped (see
  `ROADMAP.md` Shipped, `SESSION_LOG.md` 2026-07-23 Pass 0). Decisions
  recorded this Pass:
  - **(a) GUI toolkit CONFIRMED: egui/eframe over iced** (user
    decision). Closes the §2.1 open question — egui/eframe was a strong
    default there, now a closed decision. Reversal cost (rewriting the
    GUI crate) no longer looms over subsequent Passes.
  - **(b) `oxidize-pdf`: decision DEFERRED, gate unchanged.** The user
    chose a thin, header-probe-only `pdfce-core` for Pass 0 (just the
    `%PDF-` version probe, no COS parser), specifically to defer the
    build-from-scratch-vs-adopt-`oxidize-pdf` foundation decision. That
    audit remains the gate BEFORE Pass 1 (see `ROADMAP.md` Pass 1 GATE
    note and `PRIOR_ART.md`'s OPEN QUESTION) — Pass 0 did not resolve it
    and did not lock in a from-scratch core.
  - **(c) Rendering backend = GLOW, not wgpu.** eframe 0.35's default
    backend is now wgpu, but the current wgpu 29.0.4 stack FAILS TO
    COMPILE on `x86_64-pc-windows-msvc`: `wgpu-hal` 29.0.4 depends on the
    `windows` crate 0.61.2 while `gpu-allocator` 0.28.0 depends on
    `windows` 0.62.2, and their D3D12 `ID3D12Heap` types are mutually
    incompatible across the two `windows-core` versions
    (`windows_core::imp::CanInto` trait mismatch in
    `CreatePlacedResource`). §2's stack table already specified "`wgpu`,
    falls back to `glow`/OpenGL if needed" — choosing glow here
    exercises that documented fallback, so this is a **routine
    engineering call within the pre-authorized §2 design, not a
    reversal**. glow is also lighter for single-folder packaging.
    Revisit wgpu when the upstream `windows`-crate versions realign.
    (Full version-stamped finding: `D:\dev\rag\egui\`.)
  - **(d) Toolchain pinned: 1.97.1; edition 2024; resolver 3; MSRV
    (`rust-version`) = 1.92; `Cargo.lock` committed.** The MSRV of 1.92
    is driven up from the edition-2024 language floor of 1.85 by
    `eframe` 0.35, which requires rustc 1.92 — exactly the "candidate
    dependencies may force pdfce's MSRV higher than the edition floor"
    re-check that §2.1a anticipated. Sets the §2.1a toolchain/lockfile
    policy into concrete values.
  - **(e) Static CRT on Windows.** `.cargo/config.toml` sets
    `[target.x86_64-pc-windows-msvc] rustflags =
    ["-C","target-feature=+crt-static"]` so the binaries statically link
    the MSVC CRT and need no VC++ redistributable — directly serves §6's
    single-folder, no-system-wide-runtime-dependency requirement.
    Verified via `dumpbin /dependents` (only OS DLLs remain). (Full
    finding: `D:\dev\rag\rust\`.)
  - **(f) `.gitattributes` hardened for binary fixtures.** `*.pdf`,
    `*.bin`, `*.png`, etc. marked `binary` so Git's EOL normalization
    cannot corrupt byte-offset-sensitive binary PDF fixtures on
    checkout — a real risk for the §5 round-trip / byte-exact invariant
    once fixtures land. (Full finding: `D:\dev\rag\rust\`.)
- **2026-07-30 — `oxidize-pdf` gate CLOSED: decision (c)
  reference-only; `pdfce-core` is built from scratch.** Decided via
  the KenAgent decision protocol; full record at
  `docs/decisions/001-oxidize-pdf-adopt-vs-build.md` (audit of
  `bzsanti/oxidizePdf` HEAD `5f3e8b3`, v4.2.1, MIT). Closes the §12
  entry (b) of 2026-07-23 ("decision DEFERRED") and `ROADMAP.md`
  Pass 1's GATE. `oxidize-pdf` becomes MIT prior art plus an
  out-of-tree differential test oracle (`tools/difftest/`,
  `[workspace]`-excluded, pinned version, advisory-never-authoritative,
  fixtures never sourced from its repo) — never a shipping dependency
  (`cargo tree` on all four crates must never show it), no fork or
  vendor, zero literal ports planned. Maintained permissive crates are
  preferred over any vendoring: `hayro-jbig2`, `hayro-ccitt`,
  `subsetter` (fallback `allsorts`), RustCrypto stack; `flate2`
  (`miniz_oxide`/`zlib-rs` backend only) + `tiny-skia` adopted at
  Pass 1 (decision 001 §6.2). The decision creates **six Pass-1
  architecture obligations** (decision 001 §6.1 — all land in Pass 1
  even though Pass 1 is read-only, because each is
  expensive-to-impossible to retrofit):
  1. **`ByteSpan` provenance is a first-class field** on every parsed
     indirect object (retained source buffer; a span-backed object
     structurally equal to the base revision re-emits its source bytes
     verbatim on full rewrite, or is omitted on incremental save).
  2. **Lossless content-stream token model with per-token byte
     spans**; the semantic operator view is a *projection* over the
     tokens, never the primary representation.
  3. **ONE object model** — exactly one `Document` type that is
     simultaneously the parse result and the write source, to be
     recorded in §4 as a named invariant.
  4. **Fail-clean filters as a type-level contract**: every decoder
     returns `Result<Vec<u8>, FilterError>`; no code path returns
     undecoded or partial bytes on failure; one
     corrupted-stream→`Err` regression test per filter.
  5. **Lint policy**: `#![deny(clippy::unwrap_used,
     clippy::expect_used, clippy::panic, clippy::indexing_slicing)]`
     in `pdfce-core` (allowed under `#[cfg(test)]`).
  6. **No output fingerprint**: `/Info` untouched on incremental save
     unless the operator actually changed metadata; on full rewrite
     only, `/Producer` is set to `pdfce <version>`, documented, and
     overridable via API and CLI flag.
  The engineer integrates the full design text into §4/§5 when
  implementing Pass 1; until then the decision record is the
  authoritative design source for these obligations.
- **2026-07-30 — i18n/l10n architecture decided (decision 002; second
  use of the KenAgent protocol).** Full record:
  `docs/decisions/002-i18n-timing.md`. Resolves (and supersedes) the
  "Internationalization/localization" deferred bullet from
  `ROADMAP.md`'s product-scope list (2026-07-23), before the first
  real UI strings are written (Pass 1 — the record's stated point of
  no return). Outcome:
  - **Centralized, zero-dependency, function-based string catalog**:
    `crates/pdfce-gui/src/ui_text.rs` is the single home of every
    user-facing GUI string from Pass 1 onward; entries are `pub fn`
    (never `pub const` — the function signature is what makes a future
    catalog-backed retrofit a one-file, zero-call-site change).
    English-only; no locale detection, no i18n crate. Enforced by a
    new `ui-strings` CI job (whitespace-bearing string literals in
    `pdfce-gui` outside `ui_text.rs` fail the build unless
    `// ui-text-exempt: <reason>`). **UPDATE (2026-08-03, continuation
    59):** this job was found red at baseline on 140 hits — i.e. not
    actually enforcing R1 — and was hiding a real violation (three
    Measure sub-tool names as bare literals in `main.rs`). Fixed and
    relocated to `tools/check-ui-strings.sh` (runnable locally, not only
    in CI), rewritten as a character-level quote-tracking scanner
    instead of a whole-line regex (the regex had been mis-parsing
    `"svg" | "?xml"`-style adjacent literals as one literal spanning
    both). See `ROADMAP.md`'s `ui-strings` CI gate Shipped entry
    (2026-08-03) for the full account, including the "verify a gate by
    making it fail" methodology finding.
  - **Eight standing rules R1–R8** added to `ROADMAP.md`'s Standing
    rules — the discipline (no sentence assembly, no English-width
    layout, formatting helpers, no i18n dep without a §9 trigger,
    `gettext-rs` pre-disqualified for LGPL-static-link-on-Windows) is
    the actual deliverable; the catalog module is the cheap part.
  - **`pdfce-cli` is English-only PERMANENTLY, by design — not
    deferred** (R5): clap 4.6 hardcodes its own headings/error prose
    with no i18n API (clap-rs/clap#380, open since 2016), and a
    localized scripting interface is a hazard. The binding contract:
    **stdout is locale-invariant machine output, permanently** — never
    varies with `LANG`/`LC_ALL`; human diagnostics go to stderr; the
    §7 exit-code contract is likewise locale-invariant.
  - **`pdfce-core`/`pdfce-render` errors are never localized (R4)** —
    and in exchange every error variant must carry the **structured
    data** the message is rendered from, never pre-formatted prose
    `String`s. This is the record's one genuinely irreversible item
    and binds the substantial new error variants decision 001's §6.1
    obligations add in Pass 1 (`FilterError`, xref/object-model parse
    failures). `Display` stays English/diagnostic/stable per
    C-GOOD-ERR (§8); front ends own presentation.
  - **Non-Latin text *inside* PDF documents is NOT deferred (R7)** —
    `pdfce-render`'s own text stack, entirely separate from epaint's
    (which lacks bidi and CJK fonts; see the related Backlog entry
    "UI font coverage for non-Latin file paths and document metadata"
    and `D:\dev\rag\egui\epaint_0.35_text_stack_i18n_limits.md`).
  The engineer implements the record's §10 items 1–5 in Pass 1
  (ui_text.rs, CI job, CLI module-doc paragraph, R4 adherence); until
  §4/§7 body text is updated the decision record is the authoritative
  design source, same convention as decision 001 above.
- **2026-07-30 — Distribution posture decided (decision 003; third use
  of the KenAgent protocol).** Full record:
  `docs/decisions/003-distribution-posture.md`. Resolves (and
  supersedes) the final two `ROADMAP.md` deferred bullets
  ("Cross-platform scope beyond Windows first" and "Update/release
  mechanism") — that list is now **empty**. Outcome:
  - **v1 ships Windows 10/11 x86_64 and nothing else — a deliberate
    scope decision, not an accident of where the project started**
    (§6's "Windows first" parenthetical is now a decision with this
    record as authority, not a default). The codebase stays
    platform-clean at all times (R10: no `#[cfg(target_os)]` in
    `pdfce-core`/`pdfce-render`, ever), verified continuously by
    **cross-target `cargo check` CI on the existing ubuntu runner
    instead of new runners**: `aarch64-apple-darwin` (32 s, no SDK
    needed — `check` type-checks, never links) plus a **positive
    web-fork invariant check** — `pdfce-core` + `pdfce-render` must
    compile for `wasm32-unknown-unknown` (6.5 s; §3's crate split
    checked positively for the first time, not just by absence of GUI
    crates). No macOS runner (unactionable red CI with no Mac
    hardware; macOS's real cost is Gatekeeper/notarization/$99-year/
    hardware, not the build). If Linux ever ships: `pdfce-cli` first
    as static musl, `pdfce-gui` separately as glibc-dynamic — Linux is
    the *heaviest* dependency target (237 crates vs Windows 147) and
    the GUI cannot be musl-static (all Linux windowing bindings are
    `dlopen`-based; musl has no `dlopen` in static builds).
  - **R12 — no network client in the tree, fail-closed.** New
    `no-network` CI job (cargo-tree denylist against the SHIPPED
    Windows target); unlocking requires a new decision record;
    `pdfce-core`/`pdfce-render` may never contain network code under
    any future decision. §1.1's privacy posture stops being a promise
    and becomes a build gate, verifiable by a skeptical reader via
    `THIRD_PARTY_LICENSES.md`.
  - **R13 — pdfce never self-updates. Permanent.** Manual
    replace-the-folder is the only update mechanism; discovery is
    delegated to Scoop-then-WinGet manifests (gated on `LEGAL.md` §1;
    see `ROADMAP.md`'s "Release & distribution channel" Backlog
    entry). An in-app *checker* is deferred behind the record's
    complete §6.4 spec, requiring a new decision record (needs an
    HTTP client, which R12 forbids).
  - **R15 — the distribution folder is partitioned** (replaceable
    payload vs user state) from the first Pass that persists anything,
    because folder-replace destroys everything in the folder — decided
    now while it costs zero, expensive to retrofit after users have
    state. Amends §6's packaging contract + smoke-test procedure
    (engineering item 6, pending).
  - **`webbrowser`-in-eframe finding (record §3.4):** eframe 0.35
    hardcodes egui-winit `features = ["clipboard", "links"]`, so
    `webbrowser` 1.2.1 (+ its `url` dep — the lockfile's ONLY hit on a
    network-crate grep) is unconditionally linked in the shipping
    Windows binary and cannot be feature-disabled downstream. §1.1
    stays true — `webbrowser` makes no request itself, it hands a URL
    to the OS default browser, and pdfce never emits `OpenUrl` today —
    but **§1.1 needs a precision correction** ("no HTTP client/TLS
    stack", not "no code that can reach the network") — engineering
    item 5, pending. Free dividend: a zero-dependency Help-menu
    "open releases page" item is available at first release.
  - **Two latent CI defects found (record §3.5), fixes pending
    engineering:** (1) the `gui-core-separation` job runs `cargo tree`
    with no `--target` on ubuntu, so it checks the 237-crate **Linux**
    graph, not the 147-crate shipped **Windows** graph — a
    Windows-only GUI dep creeping into `pdfce-core` would pass CI; fix
    is one `--target x86_64-pc-windows-msvc` flag (`cargo tree
    --target` is metadata-only, needs no installed target). (2) every
    job uses `dtolnay/rust-toolchain@stable` while
    `rust-toolchain.toml` pins 1.97.1 — rustup honors the file, so a
    second toolchain downloads silently; pin the action to 1.97.1.
  - Eight standing rules **R9–R16** added to `ROADMAP.md` (condensed;
    the record's §6.1 is the full-text authority). The engineer
    implements the record's §10 items 1–7 (two new CI jobs, two CI
    fixes, §1.1 + §6 amendments, README copy per §6.3 verbatim) in the
    next Pass touching CI or packaging; until §1.1/§6 body text is
    amended the decision record is the authoritative source, same
    convention as decisions 001/002 above.
- **2026-07-30 — Pass 1 text-rendering font strategy decided (decision
  004; fourth use of the KenAgent protocol).** Full record:
  `docs/decisions/004-text-rendering-fonts.md`. Resolves the three
  referred sub-questions — read-path font parser, standard-14 glyph
  shapes, Pass 1 composite/shaping scope. Outcome:
  - **`skrifa` 0.42, pinned to epaint's resolved version, is the sole
    read-path font parser** for `pdfce-render` (never `pdfce-core`).
    Already in `Cargo.lock` via `epaint` 0.35 and `vello_common`, so
    the dependency adds **zero new lock packages** and zero
    `THIRD_PARTY_LICENSES.md` entries. Its `raw` re-export of
    `read-fonts` covers all four PDF font-program cases from the one
    dependency — including **bare Type 1 and bare CFF via
    `raw::ps::{type1, cff}`**, which eliminates `PRIOR_ART.md`'s
    "Type1 is the weakest link ecosystem-wide" risk outright (PFB
    segment tags, PFA hex `eexec`, raw binary `eexec` and `lenIV` all
    verified at source in `read-fonts` 0.39.2). The version pin is
    load-bearing: declaring upstream 0.45 would link a second
    semver-incompatible fontations stack beside epaint's — guarded by
    a `cargo tree --duplicates` CI check (R21).
  - **The Foxit base-14 faces are bundled** (BSD-3-Clause via Google's
    pdfium grant, 264,741 bytes, all 14 including byte-exact-metric
    Symbol and ZapfDingbats; provenance-verified per R22 —
    source URL, upstream commit, SHA-256, extraction method, license
    text). The obvious alternative, URW/Nimbus, was identified as
    **`AGPL-3.0-only WITH PS-or-PDF-font-exception-20170817`** — the
    exception covers embedding in a PS/PDF *document* only, not
    application bundling — i.e. a copyleft trap. Rejected on the
    merits and **never escalated to Ken because it was unneeded**:
    Foxit is better on every remaining axis, so the `LEGAL.md` §6.2
    copyleft-escalation protocol was never triggered and the §1
    license decision stays untouched and unconstrained.
  - **`FontEnvironment`/`RenderOptions` API seam** (record §6.3): the
    renderer never touches the filesystem, environment, or OS font
    store — the shell supplies any additional/replacement faces
    through the public API. R10 platform-cleanliness holds unchanged;
    the WASM fork supplies nothing and gets the bundled set.
  - **The render path never shapes (R17)** — content streams carry
    already-positioned glyphs; shaping belongs only to a future
    text-authoring path (`harfrust`) and reading-order recovery only
    to text extraction (`unicode-bidi`). **No hinting, ever (R18).**
    **Rendering is font-deterministic by default (R19).**
    **Substitution is always disclosed in `Diagnostics` (R20).**
  - **Pass 1 scope grows to `Identity-H`/`Identity-V` composite fonts**
    (`CIDFontType2` + `CIDFontType0`), Type 3, and the full simple-font
    §9.6.6 chain — a documented departure from the spec RAG's ladder
    (steps 4–6 were "the natural Pass 2" only while the parsing
    question was open; with `skrifa` in they collapse to "pick a GID,
    hand an outline to a pen", and steps 1–3 alone would render no
    text in most modern subsetted PDFs). Non-Identity CMaps, vertical
    metrics beyond `Identity-V` decoding, and `Tr` 4–7 clipping defer
    with diagnostics.
  - Six standing rules **R17–R22** added to `ROADMAP.md` (condensed;
    the record's §6.1 is the full-text authority). The engineer
    implements the record's §10 items 1–8 (skrifa dep + duplicates
    guard, `tools/extract-base14/`, `.gitattributes` font formats,
    manual attribution entry, `pdfce-core` std-14 data vs
    `pdfce-render` outline split, `src/font/` modules + seam +
    diagnostics, the four test families) in the Pass 1 font slice;
    until §4/§10 body text absorbs the design, the decision record is
    the authoritative source, same convention as decisions 001–003
    above.
- **2026-07-30 (continuation 8) — Pass 1.1 item 1 shipped
  (xref/object/hybrid streams); decision 001 §6.3 harvest gate CLOSED
  BY MEASUREMENT; `Provenance` API evolution; encryption-refusal
  scope addition.**
  - **(a) The conditional oxidize-pdf xref-recovery-harvest gate
    (decision 001 §6.3) is now CLEARED BY MEASUREMENT, permanently
    closed.** The gate's trigger was pdfce's own parser scoring <~95%
    on the veraPDF / PDF-Association corpora. Continuation 7's
    baseline measurement (2,914 files, `tools/corpus-report`) was
    82.4%, with `RefusedXrefStream` (489 files) accounting for 97.8%
    of all failures — i.e. the shortfall was one unimplemented
    feature, not parser weakness. After implementing cross-reference
    streams (§7.5.8, Tables 17/18, W defaults, entry types 0/1/2),
    object streams (§7.5.7, new `objstm.rs`), and hybrid files
    (§7.5.8.4, `/XRefStm` before `/Prev`), the re-run measured
    veraPDF Ok **2,395 → 2,884 = 99.2%**, `RefusedXrefStream` 489 →
    0, with ALL 24 remaining non-Ok files across both corpora being
    deliberate `*-fail-*` conformance files (correct rejections),
    zero panics/timeouts, 12,927 pages rendered. 99.2% actual vs the
    <~95% trigger: **no oxidize-pdf harvest was ever needed.** The
    conditional-harvest lesson
    (`C:\personal_rag\pdf\lesson_20260730_oxidize_pdf_xref_recovery_conditional_harvest.md`)
    stands as methodology; its pdfce-specific condition is resolved.
  - **(b) `Provenance` API evolution — the §5 round-trip contract
    extended to PDF-1.5 compressed objects.**
    `IndirectObject.span: ByteSpan` (decision 001 §6.1 obligation 1)
    became `provenance: Provenance`, an enum of `File(ByteSpan)` |
    `ObjectStream { container, index }`, with a `file_span()`
    accessor (`Some` only for `File`). Rationale: an object parsed
    out of an object stream has no contiguous file bytes, so
    byte-identical passthrough is not merely unimplemented for it —
    it is *inexpressible*, and the type now says so. The §5 contract
    is thereby **expressible-or-consciously-absent**: any future
    writer touching a compressed object must either promote it to an
    uncompressed object or rewrite its container stream; that
    obligation is documented on the `Provenance` type itself.
    `XrefEntry` is now `#[non_exhaustive]` with a new `InStream`
    variant. §5 body amended same-day (see the dated bullet there).
  - **(c) Scope addition, engineer judgment, flagged to the
    operator:** `XrefErrorKind::EncryptionUnsupported` — encrypted
    PDFs (§7.6) are refused up front rather than silently rendering
    ciphertext (an honesty call in the same spirit as the render
    path's "unsupported ≠ broken" state). GUI `Status::Unsupported`
    repointed; 4 corpus files reclassified `RefusedEncrypted`.
    Supersedes nothing — encryption support proper remains the
    Backlog "Encryption" bucket.
  - **(d) Tolerance postures chosen** (auditable, each with its spec
    anchor): `/Type` absent tolerated on XRef/ObjStm streams,
    present-but-wrong refused; a malformed individual xref-stream
    row is skipped per §7.5.8.3's unknown-type posture; a broken
    `/XRefStm` degrades to the classic xref view (safe by
    §7.5.8.4's completeness guarantee for pre-1.5 readers); ObjStm
    `/N` and `/First` must be direct objects. Corpus-verified facts
    behind these:
    `C:\personal_rag\pdf\lesson_20260730_xref_stream_w_default_hybrid_fallback_objstm_drop.md`.
- **2026-07-30 (continuation 9) — Pass 1.1 slice shipped: Form and
  Image XObjects (`Do`) + inline images. Five decisions; §3/§10 body
  amended same-day (see the dated notes there).**
  - **(a) Nested form execution uses a fresh `Interpreter` over a
    clone of the current `GraphicsState`, not `q`/`Q` on the shared
    stack.** Rationale: this makes §8.10.1's steps (a) ("save the
    current graphics state") and (e) ("restore the graphics state to
    what it was before step a") **structural, not conventional** — an
    unbalanced `Q` inside a form's own content stream provably cannot
    pop the caller's graphics state, because the caller's state was
    never pushed onto the same stack the form is mutating. A second,
    non-optional consequence: each nested interpreter gets its own
    font cache, which is a **correctness** requirement, not a
    performance optimization — a font cache keyed only by resource
    name would silently satisfy a form's `/F1` lookup with the page's
    `/F1` glyph program if the two happen to share a cache, and
    `/F1` in a form's own `/Resources` dictionary is legitimately a
    *different* font object than `/F1` on the page (§7.8.3 — resource
    names are scoped to the dictionary that defines them, not global).
  - **(b) The XObject cycle guard is keyed on the XObject's object
    number, not its resource name.** Rationale: the same stream object
    can legitimately be referenced under different resource names in
    different `/Resources` dictionaries (e.g. `/X1` on the page and
    `/Im1` inside a form both pointing at object 42) — a name-keyed
    guard would miss a real cycle reachable only through the second
    name, and could also false-positive on two different objects that
    happen to share a name in unrelated resource dictionaries.
  - **(c) Text objects do not cross the form boundary — the nested
    interpreter's text state starts as `text: None` — while
    text-relevant graphics state (current font, size, character/word
    spacing, `Tz`/`Tr`/leading) IS inherited from the caller.**
    Rationale: §9.4.1 defines `BT`/`ET` as delimiting exactly one text
    object with its own `Tm`/`Tlm`; a form boundary is not a `BT`/`ET`
    pair, so a `BT` that never closes before `Do` (or a form that
    itself never opens one) must not silently inherit or leak a text
    matrix across the boundary. But §9.3 defines font/size/spacing/
    render-mode as **graphics state**, not text-object state, and
    graphics state IS supposed to flow into a form per step (a)'s
    state-save semantics — so these two are deliberately handled
    differently rather than uniformly "reset everything" or
    "inherit everything." Pinned by a regression test.
  - **(d) Images are painted via a tiny-skia `Pattern` shader over the
    user-space unit square, never `Pixmap::draw_pixmap`.** Rationale:
    `draw_pixmap` takes an integer `(x, y)` destination origin plus a
    transform — it is fundamentally a *blit*, unable to express
    §8.9.4's requirement that an image map onto an **arbitrary
    affine** region of the page (rotation, skew, deliberate aspect
    distortion, all legal under the image-space-to-user-space CTM).
    The `Pattern`'s own transform is set to image-space→user-space
    (`[1/w 0 0 −1/h 0 1]`, carrying the mandatory y-flip since PDF
    image space has the origin at the top-left row while user space
    doesn't); `fill_path`'s transform argument (the CTM) is then
    POST-concatenated by tiny-skia into that pattern transform
    (`Shader::transform` semantics: `a.post_concat(b)` = "apply a,
    then b"), yielding image→user→device in one composed matrix. The
    filled geometry is the same unit square under the same CTM, so
    the painted region and the sampled region coincide by
    construction rather than by two independently-computed transforms
    needing to agree. Full write-up (ecosystem-wide tiny-skia finding,
    not pdfce-specific):
    `D:\dev\rag\rust\tiny_skia_0.11_pattern_shader_arbitrary_affine_image_placement.md`.
  - **(e) `MAX_XOBJECT_DEPTH` raised 16 → 64, corpus-corrected mid-slice.**
    See §10.1's dated amendment for the full rationale — briefly: 16
    was intuition and overflowed on one of 2,914 corpus files, a
    **conformant** 32-deep form-XObject chain (veraPDF
    `6-1-12-t08-pass-c.pdf`, objects 19–50); Annex C has no
    form-nesting limit and PDF/A §6.1.12 forbids imposing one. Raised
    to 64 (2× measured depth); corpus-wide overflows now 0. **Second
    incident of the identical bug shape** (first: `MAX_TOKEN_LEN`,
    continuation 7) — prompted a new `ROADMAP.md` Standing rule
    requiring every new resource guard to be run against veraPDF's
    §6.1.12 suite specifically before shipping, not just the general
    corpus.
  - **Corpus delta** (same 2,914-file corpus, isolated by reverting
    only the `Do`/inline-image arms): deferred ops 7,347 → 6,079
    (−17.3%); images rendered 0 → 76; images unsupported 0 → 137;
    forms rendered 0 → 1,168; glyphs substituted +37 (text inside
    forms now paints); xobject depth overflows 0 (was 1 at the old
    depth-16 guard); zero panics/timeouts/hangs. `images_unsupported`
    (137) now EXCEEDS `images_rendered` (76) — makes `DCTDecode`
    (baseline JPEG) the corpus-measured next priority, recorded in
    `ROADMAP.md` Pass 1.1 item 6.
  - New crates/modules: `pdfce-core/src/filters/ascii.rs`
    (`ASCIIHexDecode`/`ASCII85Decode`, §7.4.2/§7.4.3 — required by this
    slice, not deferred, because they're the only two filters that
    make an inline image's data length unambiguous per §8.9.7);
    `pdfce-render/src/image.rs` (image XObjects + inline images → RGBA
    pixmaps, full §8.9.5.2 pipeline, `MAX_IMAGE_PIXELS` = 32 Mpx
    guard); `pdfce-render/src/interpret.rs` gained `Do` dispatch and
    inline-image routing.
  - RAG escalations: `C:\personal_rag\pdf\lesson_20260730_max_xobject_depth_verapdf_32_deep_conformant_chain.md`,
    `C:\personal_rag\pdf\lesson_20260730_corpus_image_codec_priority_dct_first.md`,
    `D:\dev\rag\rust\tiny_skia_0.11_pattern_shader_arbitrary_affine_image_placement.md`.
- **2026-07-30 — Image-codec strategy decided (decision 005; fifth use
  of the KenAgent protocol).** Full record:
  `docs/decisions/005-image-codecs.md`. Resolves `ROADMAP.md` Pass 1.1
  item 6's deferred sub-order for the remaining unimplemented PDF
  filters (DCT/LZW/CCITT/JBIG2/JPX). Outcome:
  - **Two-tier codec architecture, both tiers in `pdfce-core` (R23,
    record §4.6/§6.3).** Image codecs (DCT, CCITT, JBIG2, JPX) are a
    **terminal stage**, not byte-stream filters:
    `filters::decode_stream` never decodes them — it returns a new
    `FilterError::ImageCodec(String)` variant, and a new
    `pdfce_core::image_codec` module
    (`decode_image(doc, dict, raw) -> Result<CodedImage,
    ImageCodecError>`) runs the byte-stream *prefix* of the `/Filter`
    chain through tier 1 and dispatches the single terminal codec.
    Codec output crosses the API as a `CodedImage` — samples plus
    **codec-declared** geometry and colour model — never a bare
    `Vec<u8>`, because §8.9.5 Table 89 (JPX codestream overrides the
    dictionary) and §7.4.8 (DCT colour model depends on the JPEG's own
    APP14 marker) make that declaration unrecoverable otherwise.
    `LZWDecode` alone stays a byte-stream filter in the cascade
    (bytes-in/bytes-out, `/Predictor` composes over it unchanged).
    Placement in `pdfce-core` (not `pdfce-render`) is set by the
    consumer set — `pdfce-cli extract-images`/optimize and the
    round-trip writer need to understand image streams without a
    rasterizer: **core decodes and models, render paints** (R26: only
    `pdfce-render` applies `/Decode` and resolves `/ColorSpace`).
  - **Five crate selections, all permissive, all pure-Rust, all
    `forbid(unsafe_code)` in the configuration pdfce builds:**
    DCT = `zune-jpeg 0.5` (`default-features = false` drops the
    `x86`/`neon` SIMD features, which is what activates its
    `cfg_attr` `forbid(unsafe_code)` — all 96 unsafe occurrences live
    in SIMD files that don't compile in this configuration);
    LZW = `weezl 0.2`; CCITT = `hayro-ccitt 0.3` (`fax` the named
    fallback/differential oracle); JBIG2 = `hayro-jbig2 0.3` and
    JPX = `hayro-jpeg2000 0.4` (both `default-features = false,
    features = ["std"]` — `simd` off drops `fearless_simd`, `image`
    off keeps the `image` crate/`moxcms` out of `pdfce-core`).
    The SIMD-off posture is a standing rule (R24), and CI asserts the
    **feature state** (`cargo tree -e features`), not just the
    dependency set, because feature unification is transitive and
    silent. The GPL C alternatives the obvious answers would have
    required (`jbig2dec`, OpenJPEG wrappers, mozjpeg) were made moot —
    zero `LEGAL.md` §6.2 copyleft escalations needed; §1 stays open
    and unconstrained. Named risks: three of five codecs are one
    author's project; `hayro-*` MSRV = 1.92 = pdfce's exactly, zero
    headroom.
  - **A correction worth recording (record §3.6): `Cargo.lock`
    presence ≠ build-graph presence.** The lockfile already listed
    `zune-jpeg`/`weezl`/`fax`/`tiff` as **unenabled optional
    dependencies** of `image` (whose `jpeg`/`tiff` features are off),
    so the tempting "zero-cost like `skrifa` in decision 004"
    conclusion was WRONG — verified by `cargo tree -i` ("nothing to
    print") and by `THIRD_PARTY_LICENSES.md` (generated from the real
    build graph, no entries). Honest cost: six new packages, six new
    attribution entries. Generalized to
    `D:\dev\rag\rust\cargo_lock_unenabled_optional_deps_not_build_graph.md`.
  - **Priority order, set by measurement** (record §3.1/§3.2; 2,914
    corpus files): **Pass 2.1 = DCT + LZW** (82.3% and 10.4% of
    unimplemented-filter occurrences), **Pass 2.2 = CCITT + JBIG2**
    (zero corpus presence *by corpus construction* — conformance
    corpora contain no scanned documents; priority set by the OCR/
    scanned-document Backlog dependency; one vendor, `hayro-jbig2`
    depends on `hayro-ccitt`), **Pass 2.3 = JPX** (rarest, largest
    codec surface, most unverified spec surface). Full measurement
    tables recorded in the `ROADMAP.md` Pass 2.1 entry.
  - **Three BLOCKING spec verifications dispatched to
    `pdfce-spec-librarian`** — each gates its Pass, none is
    ceremonial: §7.4.8 **Table 13** `/ColorTransform` wording +
    defaults (blocks Pass 2.1; `filter__dct.md` marks it unverified
    and the colour-routing table rests on it); **Table 11**
    `/Columns`/`/EndOfBlock`/`/BlackIs1` defaults (blocks Pass 2.2;
    `BlackIs1` is a polarity flag — a wrong default inverts every fax
    image plausibly); §8.9.5 **`/SMaskInData` + Table 89** JPX
    overrides, then an audit of `pdfce-render`'s image path for hard
    requirements Table 89 makes optional (blocks Pass 2.3).
  - Six standing rules **R23–R28** added to `ROADMAP.md` (condensed;
    the record's §6.1 is the full-text authority). The engineer
    implements the record's §10 engineering items 5–14 across Passes
    2.1–2.3; until §4/§10 body text absorbs the design, the decision
    record is the authoritative source, same convention as decisions
    001–004 above.
- **2026-07-30 (continuation 12) — Pass 2.1 shipped (DCT + LZW +
  RunLength; two-tier `image_codec` landed); three additive API
  deviations from decision 005 §6.3; decision 005 §3.2 measurement
  corrected; decision 006 dispatched.**
  - **(a) Three engineer deviations from decision 005's §6.3 API
    sketch, all additive, none contradicting a standing rule:**
    (1) `CodedImage::codec` is an `Option<Codec>` with an
    `Unspecified` variant rather than a bare enum — a codestream can
    fail to declare what the sketch assumed it always would;
    (2) `decode_image` takes an `inline: bool` parameter — the
    inline-image path has different legality rules (e.g. JBIG2
    forbidden per §7.4.7/§8.9.7) and the seam is where that belongs;
    (3) `RunLengthDecode` truncation (data ends mid-run, no EOD) is a
    strict `Err`, consistent with the fail-clean filter contract
    (decision 001 §6.1 obligation 4), not a tolerance.
  - **(b) CORRECTION to decision 005 §3.2 — the "0 four-component
    JPEGs in the corpus" measurement was WRONG: 12 exist**, in
    veraPDF's "6.2.4.3 Uncalibrated -Device colour spaces" section
    (the scan missed them; the record's method caveats anticipated
    the failure mode). **Revisit trigger 2 (§9) is LIVE** —
    `6-2-4-3-t02-pass-a.pdf` is `/DeviceCMYK` `/DCTDecode`, Adobe
    APP14 transform 2 (YCCK), NO `/Decode` array (relies on the bare
    Adobe convention); pdfce passes raw samples per §5.5's deliberate
    no-guess posture, so these 12 likely render inverted today.
    Filed as a dated addendum at the END of
    `docs/decisions/005-image-codecs.md` (the record is not
    rewritten). **Decision 006 dispatched** for the sourced
    inversion rule, per §5.5's file-the-answer-then-implement order.
  - **(c) Pass 1 bugfix (content.rs):** `ID` followed by CRLF is ONE
    white-space character (§8.9.7 + §7.2.2's CRLF-is-one-EOL);
    consuming only the CR left a stray `\n` corrupting 4 corpus
    inline DCT images. Lesson:
    `C:\personal_rag\pdf\lesson_20260730_inline_image_id_crlf_single_whitespace.md`.
  - Ship stats: corpus Ok 99.2% → 99.3% (2,886); images rendered
    74 → 201; unsupported 135 → 8 (7 JPX for Pass 2.3 + 1 deliberate
    `/Lzw`-misspelling fail-file); 412 workspace tests; 4 fuzz
    targets zero crashes; `THIRD_PARTY_LICENSES.md` +3 permissive
    entries; all gates green incl. the new R24 feature assertion.
    Full entry: `ROADMAP.md` Shipped, Pass 2.1.
- **2026-07-31 — CMYK/YCCK JPEG inversion rule decided (decision 006;
  sixth use of the KenAgent protocol).** Full record:
  `docs/decisions/006-cmyk-jpeg-inversion.md`. Closes decision 005
  §5.5's deliberately-open question and its §9 revisit trigger 2;
  corrects the 005 Addendum of 2026-07-30 (second dated addendum
  appended there). Outcome:
  - **The rule is the null rule: pdfce NEVER applies an "Adobe CMYK
    inversion" (R29).** Not on APP14 presence, not on transform-byte
    value, not on component count, not on producer sniffing. The APP14
    transform byte is consumed for exactly one purpose, already
    correctly implemented: selecting the ISO 32000-1 Table 13 colour
    transform. `/Decode` is the sole polarity control —
    `/Decode [1 0 1 0 1 0 1 0]` IS the sanctioned mechanism by which a
    producer declares inverted storage. **No behavioral change was
    needed**: the 005 Addendum's "these files likely render inverted
    today" premise was FALSIFIED — pdfce pixel-matches pdfium on all 9
    real four-component corpus JPEGs (count corrected from 12 by two
    independent scans), and matches on all six controlled variants
    (transform 2/0/no-marker × with/without `/Decode`).
  - **Sourced by four-engine consensus + a revert trail:** pdf.js,
    pdfium, MuPDF (PDF path) and Poppler all implement exactly
    never-invert (actual conditions read at source, not paraphrased);
    marker-gated inversion was shipped and reverted twice upstream
    (cairo issue 156, Firefox bug 674619). ImageMagick/libvips/
    standalone pdf.js DO invert unconditionally — recorded so they are
    never mistaken for PDF-reader precedent.
  - **Adobe TN #5116 negative result:** the normative-by-reference
    primary (ISO 32000-1 §7.4.8 footnote a) was obtained and read. The
    word "invert" appears zero times; §13.1's only `255 −` is the
    reversible CMYK→YCCK definition (forward transform defined in
    terms of TRUE ink values, so the inverse recovers true ink
    directly — no further step exists). §18's APP14 layout does NOT
    enumerate transform values 0/1/2 (those are de facto, from libjpeg
    `jdapimin.c` + Table 13). The inverted-CMYK storage convention is
    undocumented Photoshop behavior, absent from the canonical source;
    Adobe's own products compensate out of band via the container's
    decode array.
  - **The Pillow trap → R31.** The first reference consulted (Pillow)
    reported the exact complement of libjpeg's answer — because
    `PIL.JpegImagePlugin` applies rawmode `CMYK;I` to EVERY
    four-component JPEG unconditionally ("assume adobe conventions",
    no marker test). Trusting it would have "fixed" a non-bug, broken
    all 9 files, and produced a green test suite (fixtures built
    against the same wrong reference). Hence R31: a reference decoder
    is evidence only after its own conventions are verified; prefer a
    production-engine page render (pdfium/`pypdfium2`), and a
    source-level read of the condition, over a bare image-library
    decode.
  - **R26 clarified, not changed: observing is not applying.** The
    codec adapter may OBSERVE the image dictionary to classify
    diagnostics (`dct::decode` already receives `dict`) while
    remaining forbidden to APPLY `/Decode` or any polarity flip. R26's
    anti-inversion clause graduates from provisional to
    permanent-and-sourced. Diagnostics split accordingly:
    `dct_cmyk_images` (benign YCCK census — 9 in corpus, verified
    correct, no warning) vs `dct_cmyk_polarity_unverifiable` (R30 —
    4-component AND effective transform 0 AND no `/Decode`, the one
    genuinely ambiguous shape; 0 in corpus; named warning, and any
    future repair is an operator-reviewable per-image toggle, never a
    default).
  - **Separate colorimetry gap found in passing, deliberately NOT
    decided here (006 §3.7):** pdfce and pdfium agree on polarity but
    disagree on colour — `Rgb::from_cmyk` (`gstate.rs:112`) is naive
    additive vs pdfium's calibrated `AdobeCMYK_to_sRGB1` table (37.4%
    of pixels >8 Δ on the corpus CMYK image; max Δ `[11,37,30]`).
    Affects every `DeviceCMYK` fill/stroke, not just images. Filed as
    its own `ROADMAP.md` Backlog entry ("DeviceCMYK→RGB colorimetry"),
    to be scoped via `pdfce-acrobat-librarian`.
  - Three standing rules **R29–R31** + the R26 clarification added to
    `ROADMAP.md` (condensed; the record's §6.1 is the full-text
    authority). Engineering follow-ups (006 §10 items 2–6: dct.rs doc
    rewrite, counter split, CLI/GUI note text, six §6.4 regression
    fixtures asserting sample values at named pixels) are docs/
    diagnostics/fixtures only — no behavioral change, per the record.
- **2026-07-31 (continuation 15) — Pass 2.3 shipped (JPXDecode via
  `hayro-jpeg2000 0.4`); Pass 2 / decision 005 COMPLETE as planned;
  six engineer deviations recorded; a new guard CLASS
  (declared-work amplification, `jpx::MAX_TILES`) from a live fuzz
  find.**
  - **(a) Table 89 precedence — the dispatch brief stated it
    BACKWARDS; the verified rule is implemented:** a PRESENT
    `/ColorSpace` **wins** over the codestream ("any colour space
    specifications in the JPEG2000 data shall be ignored"); the
    codestream wins only when `/ColorSpace` is absent.
    `/BitsPerComponent` and `/Decode` are ignored as briefed. Pinned
    by test `jpx_present_colour_space_still_wins`.
  - **(b) `/Width`/`/Height` are NOT a Table 89 override** — the
    dict-for-placement / codestream-for-stride split is retained,
    divergence counted; a per-filter dimension-policy contrast table
    added to the `image_codec` `mod.rs` docs.
  - **(c) Bit-depth normalization is full-range scale to 8**
    (`round(v/(2^d−1)×255)`), not high-byte truncation — Table 89
    leaves depth handling to the conforming reader; the 16-bit
    fixture's `0x00FF` discriminator pixel distinguishes the two
    choices.
  - **(d) `/SMaskInData` 2 is recognize-and-defer** — preblended
    colour returned as stored, alpha not exposed; new counter
    `jpx_smask_in_data_preblended` → CLI key `jpx_preblended`
    (appended; stable-line contract kept) + a GUI line.
  - **(e) EXTRA Table 89 gap found in the audit and closed:**
    `decode_stencil` hard-required 1-bit data and would have sheared
    a JPX `/ImageMask`'s 8-bit samples 8× — the stencil path now
    takes stride/depth from the codec and thresholds against zero,
    `/Decode` still honoured (the §7.4.9 exemption).
  - **(f) hayro `data_u8()` deliberately unused** — it interleaves
    alpha AND computes `1 << bit_depth` on a palette-box depth that
    may be 128, a shift-overflow panic reachable from fuzzed input;
    pdfce interleaves itself and refuses depths outside `1..=31`
    (named diagnostic `JPX/bit-depth`). Lesson:
    `C:\personal_rag\pdf\lesson_20260731_hayro_jpeg2000_data_u8_shift_overflow_palette_depth.md`.
  - **(g) `jpx::MAX_TILES = 4096` — a new guard class:
    declared-work amplification.** A 310-byte codestream declaring a
    65,536-tile grid over 512×1024 pixels cost 32 s to decode; the
    tile grid is declared independently of image size, so no
    pixel/byte ceiling saw it. Third guard-by-intuition encounter
    (after MAX_TOKEN_LEN, MAX_XOBJECT_DEPTH) but the FIRST found by
    the fuzzer rather than a rejected real file. 8× the most
    aggressive real tiling; 32 Mpx can still tile 91×91; same input
    now 3 ms; the input kept as fuzz corpus seed + an accept-side
    test pins the ceiling from below. Lesson:
    `C:\personal_rag\pdf\lesson_20260731_jpx_max_tiles_declared_work_amplification.md`.
  - Ship stats: corpus Ok holds 2,892 (99.2%); images rendered
    204 → 210, unsupported 9 → 3; codec-unsupported 7 → 0;
    codec-FEATURE-unsupported 0 → 1 (NAMED:
    JPX/enumerated-colour-space — CIEJab, space 19,
    §7.4.9-permitted, unimplemented upstream; was a generic
    corrupt-file error). 487 workspace tests (was 457); final fuzz
    campaign 15,694 runs / 60 s / zero crashes;
    `THIRD_PARTY_LICENSES.md` +1 permissive entry (Apache-2.0 OR
    MIT); all gates green incl. MSRV 1.92 core+render (no bump —
    decision 005 §3.7's zero-headroom risk did not bite). GUI
    launched on `jpx-rgba-smaskindata1.pdf`. Full entry:
    `ROADMAP.md` Shipped, Pass 2.3.
- **2026-07-31 (continuation 16) — Next subsystem decided (decision
  007; seventh use of the KenAgent protocol): the incremental-save
  writer, sliced 3.0 → 3.1 → 3.2, the first slice with NO editing
  capability.** Full record:
  `docs/decisions/007-next-subsystem-after-read-stack.md` (the
  effective JSON is its Appendix A — base block plus the
  final-message patch; a reconciliation note at archival grounds its
  housekeeping items against Continuation 15). Candidate ranking
  A ≫ D > B > C. Pass 3.0 ships a serializer plus
  `save_full`/`save_incremental` whose entire acceptance bar is a
  corpus-wide executable proof of the §5 round-trip/minimal-diff
  invariant — per-object-definition byte identity for full rewrite,
  whole-file identity for empty-dirty-set incremental (R32) — BEFORE
  any mutation code exists, so §11.4's undo obligation does not bind
  until Pass 3.1. Adds standing rules **R32–R41** (condensed in
  `ROADMAP.md`; the record's §6 is the authority): never normalize;
  the round-trip gate guards every writer Pass; redaction forbids
  incremental save; save-mode disclosure; the object-encoder seam
  for the Pass-5 crypt stage; compressed-object promote-not-rewrite;
  `/ID` discipline; fuzz + differential coverage; no output
  fingerprint. Pass 3.0 also owes THIS document an amendment —
  §5 gains R35/R36/R39 and §11.2 a cross-reference to R35 (decision
  007 deliverable 9); the body-section update is deferred to that
  Pass, when the writer's actual shape is known, and this entry is
  the audit-trail pointer until then. Blockers live now: the
  `pdfce-spec-librarian` write-direction audit of §7.5.4/.5/.6/.8 +
  §14.4 (dispatched, in flight) and the engineer's first-action
  re-check of `hayro-write`'s changelog for byte-preserving
  incremental append (decision 001 §9 trigger 2).
- **2026-07-31 (continuation 17) — Pass 3.0 shipped (identity writer
  + round-trip proof harness): the §5 invariant is now an executable,
  green, corpus-wide gate; the §5.1–5.6 + §11.2 body amendments
  LANDED in-Pass (deliverable 9 — closing the deferral the
  continuation-16 entry carried); six engineer deviations recorded;
  the `/Encrypt` census returned — promotion trigger NOT met, Pass 5
  stays behind Pass 4.**
  - **Blocker (b) resolved first, NEGATIVE:** `hayro-write` 0.7.0
    (2026-05-27) self-describes as an internal `pdf-writer`
    converter, ~580 LoC, no incremental append — decision 001 §9
    trigger 2 does not fire; depend-or-contribute stays closed.
  - **Gate results (2,898 loadable of 2,914; the 16 NotLoadable are
    deliberate `*-fail-*` files):** empty-dirty-set
    `save_incremental` whole-file byte identity **2,898/2,898 =
    100.00%**; append identity (prior bytes intact) 2,898/2,898;
    `save_full` per-object-definition verbatim 2,897/2,898 = 99.97%,
    the single miss a CORRECT named refusal (hybrid "Isartor test
    suite manual.pdf" → `WriteError::HybridFullRewrite`, CLI exit 8,
    R33/R27 posture; incremental works on it via form A); raster
    self-oracle 5,783/5,783; 0 objects re-serialized under
    `SaveOptions::identity()`; 0 panics/timeouts; W14's ~98% STOP
    threshold never approached. The two identity assertions were kept
    distinct per R32 (W1's named confusion did not occur).
    Structural census byproduct: 2,410 classic / 487 xref-stream /
    1 hybrid / 36 live-linearized.
  - **(a) `ProducerPolicy::Set` never CREATES a missing `/Info`** —
    stamping a producer into a file that had no `/Info` would be the
    exact fingerprinting behavior R41 / decision 001 §6.1 obligation
    6 exists to prevent; `Set` only rewrites an `/Info` that already
    exists.
  - **(b) `save_full` carries object streams intact — zero
    promotions.** Type-2 xref entries name container+index, not byte
    offsets, so verbatim re-emission of the container keeps them
    valid; W3 (compressed-object offset drift) is structurally
    avoided rather than handled.
  - **(c) Hybrid full-rewrite refused BY NAME**
    (`WriteError::HybridFullRewrite`, CLI exit 8) — a full rewrite of
    a §7.5.8.4 hybrid cannot preserve both xref views without
    normalizing one away (forbidden by R33); incremental save remains
    available on hybrids via form A.
  - **(d) No predictor on emitted xref streams** — §7.5.8 never
    mentions predictors on the write side (negative result from the
    write-direction audit); reading predictored streams is
    unaffected.
  - **(e) No wildcard match arms anywhere in the writer** —
    `#[non_exhaustive]` does not bind inside the defining crate, so
    wildcard-free matches make a future `Object` variant a compile
    error at every serializer decision point instead of a silent
    null/fallback emission. Finding escalated:
    `D:\dev\rag\rust\non_exhaustive_no_effect_defining_crate_wildcard_free_match.md`.
  - **(f) A NUL-bearing Name emits `#00` and fails reload
    deliberately** — §7.3.5 forbids NUL in a name; emitting the
    escape and letting the strict reader refuse it is the honest
    posture (never silently dropping or mangling the byte).
  - **`/Encrypt` census (decision 007 parallel cheap task, run by a
    parallel agent):** 19,940 organic PDFs (20k cap, read-only,
    aggregates only — LEGAL §5): 134 = 0.67% carry `/Encrypt`;
    26 R2 / 30 R3 / 67 R4 / 10 R6 / 1 undetermined-R (FOPN FileOpen
    DRM, non-Standard handler); 92.5% legacy R≤4; empty-vs-real
    password not determinable pre-Pass-5. Promotion trigger NOT met.
  - Ship stats: new `pdfce-core` writer module
    (`mod`/`serialize`/`encoder`/`xref_out`/`save`),
    `linearization.rs`, `equivalent_across_buffers` on `object.rs`
    (lesson:
    `C:\personal_rag\pdf\lesson_20260731_span_backed_stream_derived_partialeq_cross_buffer.md`),
    `SectionShape` + `LoadedXref.startxref` on `xref.rs`,
    `tests/writer_roundtrip.rs`, `tools/roundtrip`, fuzz target
    `writer_roundtrip` (661,190 ASan execs / 61 s, zero crashes),
    CLI `round-trip` subcommand with documented exit-code contract.
    585 workspace tests (was 487); veraPDF §6.1.12 suite 44/44
    against the new writer-side guards; dependency set UNCHANGED (no
    `cargo-about` regeneration owed); all other standing gates green.
    GUI opened blank — `pdfce-gui` still lacks a file argument (open
    Pass 1.1 remainder); rendering verified via CLI `render-page`.
    Full entry: `ROADMAP.md` Shipped, Pass 3.0. Pass 3.1 engineer
    dispatched same day, in flight.
- **2026-07-31 (continuation 18) — Pass 3.1 shipped (mutation writer
  + dirty-set diff + undo/redo command log): §11.4's undo obligation
  bound and was honored in-Pass; §11.1's union-bug is now an
  executable gate (edit → undo → save byte-identical 2,897/2,897);
  §5.7 + §11.5 body amendments landed; and a CRITICAL correction to
  decision 007 W3 / §5.2 is filed forward.**
  - **CRITICAL correction (recorded forward — the archived 007
    decision file is NOT edited):** decision 007 W3's mitigation and
    §5.2's original framing claimed R35's full rewrite "closes the
    stale-copy path" for promoted compressed objects. **FALSE** —
    object streams carry through verbatim in BOTH save modes (§5.6),
    so a promoted object's old value survives inside its untouched
    container. Documented at the creating code; §5.2 carries a dated
    correction footer, §5.7 the full amendment, `ROADMAP.md` a dated
    note at R38. Binding consequence: **the Redaction Pass must
    rewrite/decompose every container stream holding a redacted
    object** — R35 is necessary but not sufficient.
  - **(a) One writer path — `save_full` takes `&DirtySet`**
    (deviation 1): `DirtySet::empty()` makes Pass 3.0's identity
    behavior a strict pinned subset of the mutation writer, not a
    parallel path that could drift.
  - **(b) `/ID` never synthesised when absent, either mode**
    (deviation 2, R41): the spec RAG's synthesise-on-full-rewrite
    recommendation was DECLINED — stamping an `/ID` into a file that
    never had one is an observable fingerprint; deferred to a real
    Save-As path.
  - **(c) Rotate-to-base-value writes nothing** (deviation 3, R33):
    the exact base spelling is restored, 4 quarter-turns net to
    zero, and `/Rotate 450` is NOT normalised.
  - **(d) Text-string encoding is ASCII-or-UTF-16BE+BOM only**
    (deviation 4): §7.9.2/Annex D.3 PDFDocEncoding is a RECORDED RAG
    GAP (a `pdfce-spec-librarian` item); undecodable bytes decode to
    U+FFFD with `exact: false` surfaced in the GUI — fuzzy, never
    sneaky.
  - **(e) GUI applies on button press, not per keystroke**
    (deviation 5): one undo step per operator intent; the undo stack
    holds meaningful operations.
  - **Fuzz find + fix (real bug):** object creation raised `/Size`
    and RESURRECTED xref entries the base `/Size` was suppressing
    (they then failed to parse). Fix: `next_object_number` allocates
    above the UNFILTERED chain maximum (was reusing live numbers) +
    creation refused by name when `/Size` suppresses entries
    (`EditError::ObjectCreationWouldExposeHiddenObjects`, CLI exit
    9; editing existing objects still works). Post-fix 408,886
    runs / 91 s zero crashes; `load_document` 681,645 / 61 s clean.
    Lesson:
    `C:\personal_rag\pdf\lesson_20260731_xref_size_suppresses_trailing_entries_raising_resurrects.md`.
  - **R38 coverage honesty:** promotion is fixture-covered, NOT
    corpus-covered — 75 corpus files hold 2,197 compressed objects
    but page objects are uncompressed in all (corpus rotation never
    promotes); the harness reports both numbers.
  - Ship stats: `EditSession` command log
    (`crates/pdfce-core/src/edit.rs`, 1,608 lines),
    `writer/fileid.rs` (§14.4 `/ID[1]` derivation), `DirtySet`
    (replacements + trailer patch + `changes_content`), CLI
    `set-info` / `rotate-page` / `--verify-undo` / exit 9 /
    appended `promoted=` key, GUI properties panel + rotate +
    undo/redo + "Save a copy…", `tools/roundtrip` mutation mode,
    fuzz edit-history extension. Key test edit → undo → save
    byte-identical 2,897/2,897 (100%) + 6 fixture tests (incl.
    object-stream file, 12-command history, undo → redo → save).
    Pass 3.0 identity gate UNPERTURBED per R34 (2,892/2,892 + 6/6;
    full-rewrite 2,891/2,892, the miss the same correct hybrid named
    refusal; raster 5,783/5,783; 0 re-serialized). Mutation gate:
    edit applied + reloaded 100%; all other objects byte-verbatim
    100%. 52 new tests (32 core + 20 CLI) over the 585 baseline;
    fmt/clippy clean; GUI-core separation verified; dependency set
    UNCHANGED; nothing committed. UI follow-ups with
    `pdfce-ui-specialist` (in flight); Pass 3.2 promoted to In
    progress, blocked on the `pdfce-acrobat-librarian` "Core
    document ops" dispatch (in flight). Full entry: `ROADMAP.md`
    Shipped, Pass 3.1.
- **2026-07-31 (continuation 19) — Pass 3.2 shipped (structural page
  operations — the first operator-visible editing feature): seven
  ops in two shapes; deletion writes a real free list (decision 007
  W9); signature awareness shipped as a real API with a
  DocMDP-grounded rename; the R36 rule-number collision reconciled
  (R42); the Tools-dock/toolbar-cap UI conventions adopted as
  standing decisions.**
  - **UI-surface conventions ADOPTED as architectural decisions**
    (authored by `pdfce-ui-specialist` in
    `docs/ui_specs/pass-3.2-page-ops.md` §1–2, which remains the
    living spec; this entry is the audit trail):
    (1) **The Tools dock is pdfce's ONE "more tools" secondary
    surface** — a persistent right-side panel toggled from the
    toolbar. Future advanced buckets (Bates stamping, OCR,
    redaction, forms, portfolios, PDF/A conversion) become entries
    in this dock's tool list, never new floating windows;
    Properties (Pass 3.1) stays the single legacy floating
    exception, never to be joined by a second.
    *(Superseded in part 2026-08-02 — see the dated §12 entry filed
    that day. This claim is now FALSE on two counts: Pass 12.M2's
    Dimension Groups panel already shipped as a second floating
    `egui::Window`, and decision 017 retires the Properties floating
    form entirely, replacing this convention with R80/R81's
    two-compartment dock. Dimension Groups is named there as the
    remaining floating-window holdout for a follow-up migration.
    **FURTHER AMENDED same day, continuation 57 — decision 017
    AMENDMENT A:** the "two-compartment dock" MECHANISM above is itself
    superseded by `egui_tiles` containers (the §6.1 trigger fired,
    operator chose the whole-content-area/Inkscape-flexible-docking
    model); the underlying SIMULTANEITY requirement R80/R81 encode
    does not lapse — it becomes a vertical split pane instead of two
    fixed compartments. See the continuation-57 §12 entry below. Pass
    18.1, which builds this, SHIPPED 2026-08-03 — see the
    continuation-58 §12 entry below.)*
    (2) **The toolbar is CAPPED at its 6 groups + the Tools
    toggle.** Any future feature fits an existing group, becomes a
    rail-contextual control, or becomes a Tools-dock entry; no 7th
    toolbar group without a fresh review.
    (3) **The rail-vs-dock rule:** if the operator's argument is a
    set of pages already visible in the open document, the control
    lives on the thumbnail rail (selection + contextual action
    bar); if the argument comes from outside the open document (a
    file, a folder, a set of files), it lives in the Tools dock.
    Panel add-order stays load-bearing: toolbar, status, rail,
    dock, canvas, floating-last.
    *(Extended 2026-08-01, continuation 20 — the placement taxonomy
    is now THREE-way: a snapshot action on the whole open document
    (copy-text) fits neither rail nor dock and lives as a toolbar
    menu button. See the continuation-20 entry, item (b). This is an
    extension, not a rewrite — rules (1)–(3) stand unchanged.)*
  - **R36 collision reconciled (record defect flagged in the UI
    spec's header):** §5.4's linearization-never-repaired rule is
    now **R42**; decision 007's R36 ("save mode is chosen by
    contract and disclosed" — the signature either-or) keeps the
    number, as the code comments already use it. §5.4's citation
    line corrected; full dated note at R42 in `ROADMAP.md` Standing
    rules; no rule content changed.
  - **(a) `SignatureImpact::ByteRangePreserved`** — renamed from the
    UI spec's `PreservedIncremental` per the mid-Pass DocMDP relay
    from `pdfce-spec-librarian`: §12.8.1 NOTE 1 guarantees only
    that the signed BYTE RANGE is preserved; whether the signature
    remains *valid* in the DocMDP sense is a separate verdict, and
    the variant name no longer overclaims. Classification walks
    `/Reference` → `/TransformMethod` (`/DocMDP` lives in the
    signature's reference array, never `/Perms`; `/P` defaults 2);
    a certification whose `/P` forbids the change is a NAMED
    refusal, `EditError::CertificationForbidsChange` (Table 258);
    `/FieldMDP` recognized. `signature_impact_of_save(mode)` takes
    the save mode as a parameter (deviation from the spec's
    zero-arg sketch). Spec closure: `PDF_Spec`
    `iso32000__s__12.8.md` now 689 lines, a/b/c verdicts + the
    ByteRangePreserved-never-reported-alone rule.
  - **(b) Insert is a producer, not an `EditSession` command**
    (deviation from the UI spec §3.6/§3.9): in-place overlay insert
    requires per-object SOURCE buffers plus an overlay-aware
    renderer — deferred rather than half-built. GUI Insert
    deferred; the Tools dock names the CLI `insert` command; the
    producer path (`assemble()`) ships it complete.
  - **(c) One shared `assemble()` for all four producers**
    (extract/merge/split/insert) over one shared `ObjectGraph`
    walk (`graph.rs` — works over the loaded file OR the
    `EditSession` overlay; `edit.rs`'s Pass-3.1 comment predicted
    the need). Carryover policy table documented + cited in
    `pageops/`: outline subset+repoint / per-source-top-level
    merge / target-only insert; `/Dests` never carried (carried
    bookmarks rewritten explicit); `/PageLabels` stale-for-insert
    with named diagnostic, dropped-for-subsets; `/StructTreeRoot`
    dropped + counted; form fields `Doc<N>_` auto-rename,
    straddling fields dropped whole + counted; barrier hits
    counted.
  - **(d) Deletion free-list (W9):** `DirtySet::delete` +
    `apply_free_list` — type-0 entries, generation+1 saturating at
    65,535, front-spliced onto the existing free chain;
    pre-existing detached free entries untouched (R33); a
    two-closure sweep proves shared objects are never freed.
  - **(e) Remaining spec deviations:** rail checkbox is one
    interaction + position test (not two overlapping `interact`s);
    `egui::Window` not `egui::Modal` (the spec's own named
    fallback); split's file-size criterion deferred + named.
  - **Two bugs caught by tests (both filed as `personal_rag/pdf`
    lessons):** reorder lost inherited rotation (`materialize_for`
    was one-directional; `preserve_inherited` now writes
    §7.7.3.4's default when the NEW parent chain supplies a value
    the old chain didn't); extract left `/Dest [null /Fit]`
    (reference barriers must propagate through WHOLE arrays, not
    per-element).
  - **Open residuals (named):** `/Info` edits not
    certification-gated (`/P 1` strict reading — owed decision,
    recorded at `check_certification`);
    `PermissionGate::NotApplicableYet` awaits Pass 5; delete corpus
    coverage thin (23 multi-page files — fixtures + fuzz carry
    it); `qpdf` not on PATH (R40's external oracle unused —
    operator-installable).
  - Ship stats: `graph.rs`, `signature.rs` (810 lines), `pageops/`
    (2,833), `tests/page_ops.rs` (967). **769 workspace tests (was
    707)**; 3.0/3.1 gates UNMOVED per R34 (identity 2,892/2,892;
    full-rewrite 2,891/2,892 same hybrid refusal; edit → undo →
    save 2,891/2,891; raster 5,771/5,771); corpus page-op sweep
    2,892 extract-ok + 23/23 delete-ok, 0 failures; §6.1.12 40
    files clean with guard headroom MEASURED (outlines 10 vs 200k,
    dests 62 vs 100k, depth 3 vs 64, pages 10k vs 1M); fuzz
    `pageops_sequence` 130,400 / 61 s zero crashes +
    `writer_roundtrip` clean; GUI-free both targets; wasm32 +
    aarch64 clean; `--duplicates` + R24 clean; **`ui-strings` R1
    gate clean for the FIRST time** (3 pre-existing false positives
    fixed — evidence CI has never run, W15); no new dependencies.
    Carried items applied: Apply/Revert grey-out, per-field lossy
    marking, command-named undo tooltips, **GUI file argument
    (Pass 1.1 remainder CLOSED)**, rotate shortcut `[`/`]`. GUI
    launched (PID 23332); CLI demo split-by-bookmark →
    reverse-merge → render. Pass 4 (text extraction) promoted to
    In progress; `pdfce-spec-librarian` §9.10 sourcing dispatched,
    in flight. Full entry: `ROADMAP.md` Shipped, Pass 3.2.
- **2026-08-01 (continuation 20) — Pass 4 shipped (text extraction /
  structured content): the §9.10.2 ladder verbatim with rung 3
  structural+named, PDFDocEncoding built from Annex D.3's structural
  rules (4 source-table typos caught), the plain/sourced dual API
  adopted as the extraction-feature pattern, and the UI placement
  taxonomy extended to three-way.**
  - **(a) Confirmation-dialog convention is now a STANDING pattern**
    — two independent uses exist (Pass 3.2's signature-impact
    confirmation; Pass 4's pre-copy reliability gate, firing on
    `identity_fonts_without_to_unicode > 0 || sourced < 50%` —
    deliberately not a low threshold): a single centre-anchored,
    one-question, input-blocking `egui::Window`, resolved before any
    other pending question is posed. Enforcement lives in the action
    dispatcher, not the window code — the Ctrl+S bug (below) is why
    that placement is load-bearing.
  - **(b) Placement taxonomy is now THREE-way** (dated extension of
    the continuation-19 conventions record, which stands unchanged):
    **rail** — argument is pages in the open document; **Tools
    dock** — argument comes from outside the open document; **toolbar
    menu button** — a snapshot action over the whole open document
    (copy-text is the first instance). The rail-vs-dock binary didn't
    cover the third case; copy-text is deliberately NOT a Tools-dock
    entry. Designed in `docs/ui_specs/pass-4-text-extraction.md`
    (573 lines), which remains the living spec.
    *(Superseded/extended 2026-08-01, continuation 23 — the Pass 6.0
    ui-specialist delivered a five-way placement taxonomy that resolves
    the X14 drift and subsumes this three-way rule. See the
    continuation-23 entry, item (b). This bullet stays as the audit
    trail of the intermediate three-way form.)*
  - **(c) `plain_text()` vs `sourced_text()` dual API adopted as THE
    fuzzy-never-sneaky pattern for extraction-like features** (OCR is
    next): spec-sourced characters and derived judgments (spaces,
    line breaks, ordering) are separate API surfaces, the derived
    layer isolated (`text_extract/layout.rs`) and every derived
    insertion labelled. The Drucker `/ActualText` example is the
    pinned verification: sourced "Drucker", plain "Druc\nker" with
    one labelled derived break.
  - **(d) Deviations, both additive and counted:** per-code
    fallthrough (§9.10.3 NOTE 4 — universal practice, unsourced,
    counted); glyph-name extension for fonts failing method 2's
    whole-array precondition. `FontNote::BuiltinEncodingUnreadable`
    names the one R21-unreachable case (embedded symbolic built-in
    encoding → StandardEncoding fallback — counted as extension,
    never sourced). Rung-3 gaps are structural+named
    (`Rung3Gap::{IdentityNoToUnicode, Ucs2NotBundled,
    PredefinedCmapNotBundled}`), never silently skipped.
  - **(e) Bidi deferred-not-half-done:** RTL presence detected +
    counted; `unicode-bidi` NOT added (B1–B3 would make reordering
    wholly derived). Extraction diagnostics are a SNAPSHOT surface,
    separate from the per-frame render header (merging would lie on
    navigation).
  - **Real pre-existing GUI bug found by the ui-specialist's
    verification and fixed:** Ctrl+S fired through a live signature
    confirmation — the doc comment claimed a guard that didn't
    exist; Pass 4's second centre-anchored window made the collision
    reachable. Fix: one-question gate at the top of `apply()`, doc
    comments corrected; `status_is_open()` now requires a page
    (`/Count 0` nit). Escalated as an egui-tier RAG lesson
    (pending-state gates belong in the action dispatcher).
  - **Open residuals (named):** `/Alt`/`/E` counted-not-substituted;
    nested `/ActualText` outermost-wins; artifacts
    excluded-by-policy but present in runs; structure-tree order
    recognition-only; derived layout assumes axis-aligned text
    (rotated text over-produces line breaks — cannot affect sourced
    chars); canvas text-selection deferred WITH its spec written
    (verified needing no core addition — `ExtractedGlyph` already
    carries per-glyph `LadderRung` + geometry).
  - Ship stats: 5,469 new `pdfce-core` lines (`textstring.rs`,
    `text_extract/{cmap,font,page,layout,mod}.rs`). Corpus
    measurement: 2,907 files, 281,516 codes, 0 panics/timeouts —
    rung 1 78,101 (27.74%), rung 2 202,793 (72.04%), rung 3 zero,
    extension 39 (0.01%, almost all Isartor 6-3-7), failed 583
    (0.21%); **sourced total 99.78%**; derived 752 spaces / 1,905
    line breaks. **875 workspace tests (was 769)**; Pass 3.x gates
    UNMOVED; §6.1.12 44/44 with measured headroom (1,674 CMap
    singles vs 500k, 2,044 ranges vs 100k); fuzz `text_extract`
    50,215 / 61 s zero crashes, 10 targets build; NO new deps;
    `cargo-about` byte-identical. Demo: 20-page tagged manual 34,037
    codes 100% sourced in 66 ms (background-extraction concern
    measured-and-unneeded); GUI PID 41588. Pass 5 (encryption)
    promoted to In progress by decision-007 sequence;
    `pdfce-spec-librarian` §7.6 corpus session dispatched, in
    flight. Full entry: `ROADMAP.md` Shipped, Pass 4.
- **2026-08-01 (continuation 21) — Decision 008: next subsystem after
  the decision-007 read→write→edit→extract stack = Annotations &
  markup (candidate A), sliced.** Full record:
  `docs/decisions/008-next-subsystem-after-extract.md` (archived in
  parallel by another agent). Ranking across candidates
  **A ≫ B > C > E > D > F** (A = Annotations & markup,
  B = Forms/AcroForm, C = Redaction, E = Vector/Inkscape-parity,
  D = Text-&-object editing, F = Signatures/PAdES).
  - **The slice:** Pass 6.0 = annotation & widget appearance rendering
    (read-side) — IN PROGRESS, blocked on the `pdfce-spec-librarian`
    §12.5 dispatch (in flight); Pass 6.1 = authored streams + the
    project's first content-stream serializer + geometric markup
    authoring (Ink/Square/Circle/Line/Polygon/quad-point); Pass 6.2 =
    text-bearing annotations + §12.7.3.3 variable text (no `harfrust`
    per R17 — Base-14 + embedded widget widths); then Pass 7 (Forms,
    B, second overall), Pass 8 (Redaction, C), Pass 9+ (Vector, E,
    sliced a–g), Pass 5 (Encryption — repositioned to the
    fallback/interleave track AFTER Pass 7, retaining its
    decision-007 ID), Pass 10 (Signatures, F, last). Sequence recorded
    in `ROADMAP.md` "Next up → Decision 008 sequence."
  - **Census (read-only, pypdf, aggregates only, LEGAL §5 posture):**
    conformance corpus 338/2,914 files (11.6%) have annotations,
    228 with `/AP`, 127 `/AcroForm`, 4 `/XFA`; organic sample
    2,500/25,203 Dropbox files — 814 (32.6%) annotations, 753 (30.1%)
    `/AcroForm`, 43,508/55,545 annots have `/AP` (78.3%), `/Widget`
    87.8% of annots, `/Tx` 99.8% of 47,868 fields, `/SigFlags` 16
    (0.64%), `/XFA` 2 (0.08%). **Per-file figures are robust; the
    per-annotation figures are concentration-skewed and must be
    re-measured with pdfce's own tooling before any becomes a gate
    denominator** (decision 008 caveat W16). The 0.64% `/SigFlags` and
    0.08% `/XFA` shares are recorded against the Signatures and XFA
    Backlog buckets respectively; the XFA measurement answers the
    demand half only and does NOT close the standing "verify Adobe's
    XFA deprecation status" open item.
  - **Structural findings F1–F4:** **F1 — pdfce renders NO annotations
    and does not even COUNT them: an UNDISCLOSED shortfall, unique in
    the project** (everything else unsupported is R20/R27-counted;
    annotations are the one gap filed nowhere — a new "Annotation
    display (read-side)" Backlog bucket was created for it, exactly as
    text extraction was unfiled pre-decision-007). **F2 — the §8.10.1
    form-XObject execution path already shipped in Pass 1.1, and an
    `/AP` `/N` IS a form XObject** — the rendering primitive for 6.0
    already exists. **F3 — there is no content-stream writer yet and
    `Stream` cannot hold authored bytes** — Pass 6.1 builds the first
    one. **F4 — the pageops/assemble staging-buffer pattern is the
    model for authored bytes, and `DocumentView` carries a written
    assertion "a Pass that authors stream bytes must revisit this
    type"** — discharged in 6.1 by deliberately amending the type
    (R45).
  - **Standing rules added: R43–R52** (see `ROADMAP.md` Standing
    rules): R43 render-from-`/AP`-or-not-at-all (display sibling of
    R29); R44 generated appearances are written to the file, never
    rendered from a private buffer; R45 authored bytes in a session
    staging buffer, `Stream` keeps its span model, the `DocumentView`
    assertion discharged by amending the type; R46 content-stream
    serializer proven by a corpus identity gate before it authors;
    R47 an annotation edit never touches the page content stream;
    R48 flatten is destructive and discloses incremental-save
    recoverability (R35 sibling); R49 a widget is an annotation first
    (one appearance pipeline); R50 hidden annotations honored AND
    counted (forensics — the F1 fix); R51 `/NeedAppearances` disclosed,
    never silent auto-generate; R52 redaction mark and apply are
    separate operations with separate confirmations.
  - **Pass-5 repositioning:** the continuation-20 promotion of Pass 5
    (Encryption) to In progress was by the decision-007 SEQUENCE;
    decision 008 supersedes that sequencing and moves Pass 5 to the
    fallback/interleave track after Pass 7. Pass 5 keeps its 007 ID
    (never renumbered); its scope and the 0.67% `/Encrypt` census are
    unchanged. A dated append-only correction note is at the
    `ROADMAP.md` In-progress (Pass 6.0) entry; the Pass 4 Shipped entry
    is NOT rewritten.
  - **Owed (logged, NOT fixed this filing) — two §4 staleness items
    surfaced by decision 008:** (1) `ARCHITECTURE.md` §4 still
    describes the Pass-0 header-probe state ("Current state as of
    Pass 0") — decisions 001, 004, and 005 owe their §4 core-data-model
    integration (the real `Document`/`Object`/`StreamData`/font/codec
    surface shipped across Passes 1–2 is not yet reflected in the §4
    body). (2) A consolidation session is owed to integrate that
    accumulated reality into §4 **before** the annotation data model is
    documented there — so the annotation types (§12.5 annotation dict,
    `/AP` appearance selection, the flag set, the authored-stream
    staging buffer of R45) land in a §4 that already reflects Passes
    1–5, not on top of a Pass-0 stub. Neither is fixed here; both are
    recorded as owed.
- **2026-08-01 (continuation 22) — §7.6 encryption spec-corpus session
  complete; Pass 5 spec-unblocked (still queue-deferred behind Pass 7
  per decision 008).** Spec-corpus work, not a code Pass — pointer only.
  `pdfce-spec-librarian` built the §7.6 corpus at
  `D:\Dev\Rag-Specialized\PDF_Spec\` (7 new + 2 updated files;
  `iso32000__s__7.6.1`–`7.6.5`, new `security__aes256_r5_r6.md` under a
  new `security__` prefix, `iso32000__ref__encryption_impl.md` derived
  checklist, `filter__crypt.md` de-stubbed; Adobe ExtensionLevel 3
  supplement staged). Closes the "§7.6 largest spec gap" prerequisite
  the Encryption Backlog bucket named. **`/R 6` (AES-256, Acrobat X+)
  could NOT be sourced** (ISO 32000-2 paywalled; no public
  ExtensionLevel 8; pdfa.org 403) — the agent correctly REFUSED to
  reconstruct Algorithm 2.B from memory. Consequence + the three
  AES-256-write options are a Pass-5 open sub-decision, recorded at the
  `ROADMAP.md` Encryption Backlog bucket; two operator decisions
  (LEGAL.md §2 Adobe-supplement copyright contradiction; `/R 6` sourcing
  method) at SESSION_LOG continuation 22's operator-items list. Full
  record: SESSION_LOG continuation 22.
- **2026-08-01 (continuation 23) — Pass 6.0 shipped (annotation &
  widget appearance rendering, read-side): render every existing
  `/AP` `/N`, count every annotation, author NOTHING (R43).** The
  direct remedy for decision 008 finding F1 (annotations were the
  project's one undisclosed, uncounted shortfall). F2 confirmed in
  code: an `/AP` `/N` IS a form XObject, so §12.5.5 placement routes
  through the EXISTING Pass 1.1 `interpret::run_form_at` → `do_form`
  path — X8 resource scoping, cycle guard, `MAX_XOBJECT_DEPTH`, and the
  per-form font cache all inherited unchanged, pinned by
  `appearance_uses_its_own_resources_not_the_page_font`. New surface:
  `pdfce-core` `annot.rs` (§12.5 walk/model/select, `AnnotFlags`
  §12.5.3, the `/AP`→`/N` + `/AS` `Appearance` taxonomy, the
  document-scoped `need_appearances` query); `pdfce-render` `annot.rs`;
  `RenderOptions.annotations` + `RenderOptions::with_annotations`;
  Diagnostics +8 keys; CLI `render-page --no-annotations` +
  `list-annotations`; GUI toolbar visibility toggle + status-bar
  disclosure; fuzz target 11 `annot_walk.rs` (1.1M runs, 0 crashes).
  - **(a) Census baseline PINNED (pdfce-native; supersedes decision
    008's pypdf conformance figures per W16, which is now DISCHARGED
    for the conformance corpus):** 2,914 files, ZERO panics — 338 with
    annotations / 429 annotations / 224 USABLE `/AP` `/N` / 127
    `/AcroForm` / 34 `/Popup` / 87 `/Widget`. The per-file 338 and 127
    match pypdf exactly. **The 224-vs-228 (pypdf) `/AP` gap is
    DEFINITIONAL, not an error:** pdfce counts a *usable* `/AP` `/N`
    (resolvable stream / selectable `/AS` state), pypdf counts raw
    `/AP`-key presence; pdfce's predicate is stronger. Filed as a
    `personal_rag/pdf` finding.
  - **(b) Durable FIVE-way GUI placement taxonomy (ui-specialist
    deliverable — THE settled convention, resolves the X14 drift;
    supersedes/extends the continuation-20 three-way rule):**
    **view-state → toolbar view group; edit → toolbar/window;
    selection-scoped → rail; advanced → Tools dock; disclosure →
    status bar.** All future GUI placement decisions follow this. The
    Pass-6.0 GUI (visibility toggle = view-state → toolbar view group;
    annotation diagnostics = disclosure → status bar) is the first
    instance built to it.
  - **(c) Deviations, all named/counted (fuzzy-never-sneaky):**
    (1) `/NoZoom`/`/NoRotate` post-annotation-matrix transform DEFERRED
    — counted + named (`annotation_notes`); rare, near-exclusively on
    icon subtypes lacking `/AP` anyway; a wrong post-transform is worse
    than a disclosed omission. (2) `/OC` optional-content visibility
    test not implemented — consistent with the renderer implementing NO
    optional content anywhere (BDC/EMC deferred; §8.11 is a RAG GAP);
    an OC-off annotation currently paints, named. (3)
    `need_appearances_documents` is a document-scoped query, not folded
    into per-page render `Diagnostics` (inherently document-level). (4)
    GUI diagnostics are a separate always-evaluated status line below
    the content-diagnostics header, NOT folded into the content
    unsupported-tally (chosen to avoid destabilizing the tested content
    clean-return path; still honest R50/R27/R51; flagged for future
    ui-specialist refinement).
  - **(d) Placement correctness (X2), NOT a pixel-parity close:**
    `tools/annot-pdfium-diff.py` (pypdfium2, decision 006 §3.2
    precedent) — 7/7 pure-geometry placement fixtures agree with pdfium
    within 4 px, 0 mismatches; 6 blank-expected cases correctly blank.
    This is an ink-bbox differential on the annotation subset ONLY; the
    Pass 1.1 full-page pixel-parity remainder stays OWED — explicitly
    NOT claimed closed.
  - **(e) Guards:** new `MAX_ANNOTS_PER_PAGE = 1_000_000` (pure memory
    backstop — Annex C imposes no limit, §6.1.12 forbids imposing one;
    busiest corpus page ≪100, >10,000× headroom); `/AP` recursion
    reuses `MAX_XOBJECT_DEPTH = 64` unchanged. **R34 holds
    STRUCTURALLY** — no pinned reference raster exists; the round-trip
    oracle is a runtime self-comparison, so painting annotations
    perturbs nothing the Pass 3.x/4 gates measure.
  - **RAG escalations (`C:\personal_rag\pdf\`):** (a) pdfium requires
    `FPDF_FFLDraw` to render `/Widget` appearances — a differential-
    harness gotcha (the two apparent pdfium diffs were REFERENCE
    divergences, not pdfce errors: pdfium SYNTHESIZES the no-`/AP`
    `/Circle` `/IC` fill that R43 makes pdfce refuse); (b) QuadPoints
    CCW-vs-Z-order unresolved (§12.5.6 says CCW, real producers/Acrobat
    emit Z/reading order — only bites Pass 6.1 generation).
  - Ship stats: **901 workspace tests (was 875)**; fmt/clippy clean;
    GUI-free host + msvc; wasm32; `--duplicates`; `ui-strings`;
    `no-network` all clean. Pass 6.1 (authored streams + content-stream
    serializer + geometric markup) promoted to In progress, blocked on
    the §8.10.2 form-XObject WRITE-direction audit
    (`pdfce-spec-librarian` dispatched, in flight); the "Comments &
    markup" acrobat bucket is complete. Full entry: `ROADMAP.md`
    Shipped, Pass 6.0.
- **2026-08-01 (continuation 24) — Pass 6.1 shipped (authored streams +
  content-stream serializer + geometric markup authoring): the
  project's FIRST content-stream authoring path.** Discharges decision
  008 findings **F3** (there was no content-stream serializer, and
  `Stream` could not hold authored bytes) and **F4/R45** (authored
  bytes are staged, not stored by mutating the span-provenanced `Stream`
  type). Authors the pure-geometry markup annotations (Ink, Square,
  Circle, Line, Polygon, PolyLine + the quad-point family
  Highlight/Underline/StrikeOut/Squiggly); text-bearing annotations and
  §12.7.3.3 variable text are deferred to Pass 6.2 (one appearance
  pipeline). New surface: `writer/content.rs` (`ContentBuilder` — the
  §8.2 path/paint/graphics-state/colour operator set + the §8.10 WF6
  form-XObject ordering from the unblocking WRITE-direction audit),
  `annot_author.rs` (`MarkupSpec`/`Color`/`Quad`/`LineEnding`/
  `TextMarkupKind`; `build_appearance` → `AuthoredAppearance` =
  annotation dict + `/AP` `/N` form XObject + content bytes). Modified:
  `writer/serialize.rs` primitives promoted to `pub(crate)`;
  `DirtySet` gains the R45 staging buffer + `combined_source()`;
  `writer/save.rs` serializes against base++staging; `edit.rs` gains
  `EditSession::add_markup` + `authored_source()` + COW `/Annots`
  patching + `AnnotKind` + `CommandKind::AddAnnotation` + three named
  `EditError`s; `pageops/assemble.rs`'s `DocumentView` doc comment is
  **amended to discharge — not delete —** the R45 written assertion.
  CLI `annotate`; GUI minimal "Markup ▾" menu; fuzz target 12.
  - **(a) The content-stream serializer is proven before it authors
    (R46) — this is the load-bearing architectural fact of the Pass.**
    The R46 corpus identity gate re-serializes EVERY existing content
    stream and requires byte-faithful reproduction before the writer is
    trusted to author: **12,936 streams / 2,898 files → 12,854
    byte-identical (99.37%) / 82 non-identical (0.63%) / 0 corrupted →
    PASS.** The 82 are all spec-legal, VALUE-PRESERVED number
    re-spellings, enumerated by file+reason (R20): `.05`→`0.05` (20×),
    `-0`→`0` (18×), one 300-digit pathological real, `1.`→`1.0`.
    **Architectural framing (records why this is NOT a §5 round-trip
    violation):** R46 is a SERIALIZER-correctness test that deliberately
    re-emits every stream. §5's minimal-diff invariant means pdfce
    **never re-serializes untouched page content in normal save** —
    span re-emission passes it through byte-verbatim — and authoring
    writes only NEW streams. So these 82 divergences are structurally
    unreachable in production save; X6 (silent normalization of content
    pdfce claims to preserve) is caught mechanically.
  - **(b) R45 staging-buffer reality — the F4 assertion discharged by
    amendment, not deletion.** `Stream` keeps its (offset, len) span
    model; authored bytes accumulate in a per-`DirtySet` staging buffer
    (the `pageops/assemble` pattern generalized), and `save` serializes
    replacement/created objects against `combined_source()` =
    base++staging. The `DocumentView` "a Pass that authors stream bytes
    must revisit this type" assertion (F4) is discharged by a named,
    reviewed doc-comment amendment — the deliberate change R45
    anticipated, never a silent widening of `Stream` into a
    bytes-owning type.
  - **(c) R44 authored-appearance identity holds end-to-end.** Author →
    save → reload → paint round-trips: an authored square/highlight/ink
    reloads and renders through Pass 6.0's read path (annots=3/painted=3/
    forms=3; red square paints red), `undo_identical=1` on the first
    author (minimal-diff), and extract-from-session (X5) resolves the
    authored appearance BYTE-EXACT via `authored_source()`. Every
    authored look is a real baked `/AP` `/N`; there is no second private
    render path (R44).
  - **(d) QuadPoints authoring convention DECIDED — Z / reading order
    (UL, UR, LL, LR).** Closes the continuation-23 carried open item
    (§12.5.6 spec's CCW vs the Z/reading order real producers/Acrobat
    emit). pdfce authors in Z order for maximum third-party interop
    (Acrobat/PDFBox/pdf.js), documented in `annot_author.rs`. Because
    pdfce's own render paints the baked `/AP` and never re-derives
    geometry from QuadPoints, the choice is an interop decision, not a
    correctness one — render is convention-independent.
  - **(e) Deviations/residuals (fuzzy-never-sneaky, all named):** X11
    certification gating is CONSERVATIVE (reuses `check_certification()`,
    over-refuses annotation-add that `/DocMDP` `/P 3` permits;
    fail-clean-safe; per-`/P` refinement scoped, §12.8 already sourced);
    X10 encryption guard is a forward-compat R37 seam (encrypted files
    refused at LOAD, so `DocumentEncrypted` in `add_markup` is
    unreachable until Pass 5); no `/M`//`CreationDate` on authored
    annotations (avoids clock non-determinism in byte-compare tests);
    line-ending set limited to None/OpenArrow/ClosedArrow; default
    colours are pdfce's own except the sourced Highlight yellow+Multiply.
  - **Ship stats: 939 workspace tests (was 901)**; fmt/clippy clean;
    **R34 re-runs GREEN** (identity + raster unperturbed — authoring
    touches no existing object); GUI-free host + msvc; wasm32;
    `--duplicates`; `ui-strings`; `no-network` all clean; fuzz target 12
    696,098 runs / 61 s, 0 crashes; **ZERO new dependencies** (hand-
    rolled, no `harfrust`; `THIRD_PARTY_LICENSES.md` unchanged). Pass
    6.2 (text-bearing annotations + §12.7.3.3 variable text) promoted to
    In progress, blocked on the §12.7.3.3 variable-text spec
    (`pdfce-spec-librarian` dispatched, in flight). Full entry:
    `ROADMAP.md` Shipped, Pass 6.1.
- **2026-08-01 (continuation 25) — Pass 6.2 shipped (text-bearing
  annotations + §12.7.3.3 variable-text appearance generation): the
  decision-008 6.x annotation arc is COMPLETE.** 6.0 (display) → 6.1
  (geometry) → 6.2 (text) are all shipped; In progress advances to Pass 7
  (Forms/AcroForm). Adds the text-bearing annotation subtypes Pass 6.1
  deferred — FreeText, Text (sticky note), Stamp — plus the shared
  §12.7.3.3 variable-text pipeline. New surface: **`vartext.rs`** — the
  §12.7.3.3 variable-text pipeline (`/DA` default-appearance parsing, the
  auto-font-size `0` rule, field-value → appearance-stream layout with
  line breaking / `/Q` quadding / baseline placement). Modified:
  `writer/content.rs` (`ContentBuilder` gains the text/marked-content/
  clip/matrix operator set BT/ET/Tf/Td/TD/TL/Tj/Tc/Tw/Tz/q/Q/BMC/EMC/W/cm
  + `emit_literal_string`); `annot_author.rs` (`TextAnnotSpec` +
  `StickyIcon`/`StampName`/`AuthoredTextAnnot`/`build_text_annotation`);
  `edit.rs` (`EditSession::add_text_annotation` + `AnnotKind::{FreeText,
  Text,Stamp}` + `EditError::VariableText`). CLI `annotate --type
  freetext|text|stamp`; GUI "Text ▾" menu + modeless text-entry popup.
  - **(a) `vartext.rs` is the ONE appearance generator Pass 7 reuses —
    the load-bearing architectural fact of the Pass.** The §12.7.3.3
    variable-text procedure is written once, here, as the shared FreeText
    + widget-field appearance generator. Pass 7 (Forms) wires it to the
    `/AcroForm` field model rather than re-implementing appearance
    generation (R49 — a widget is an annotation first; one appearance
    pipeline for widgets and annotations alike). This is why 6.2 precedes
    7 in the decision-008 sequence: the appearance half is earned before
    the field model needs it.
  - **(b) The content-stream operator additions are PURELY ADDITIVE —
    the R46 identity result is preserved by construction.**
    `ContentBuilder` gains text/clip/matrix/marked-content emit methods,
    but the R46 re-emission path (`reemit_canonical` /
    `emit_token_canonical` / `number_divergence_reason` / `emit_number`)
    is byte-unchanged. The orchestrator's full-corpus R46 re-run
    (2026-08-01, over `fixtures/external` — 3,020 files, veraPDF-corpus +
    pdf20examples) is **GATE PASS, zero corruptions**, all divergences the
    same value-preserving number re-spellings Pass 6.1 catalogued — so the
    additive-only claim is confirmed BY MEASUREMENT, not merely by
    inspection. (The engineer's earlier "corpus not present" was a
    path-resolution miss; the corpus is present and runnable at
    `fixtures/external`, a standing note for future Passes.)
  - **(c) The bare-Base-14 modality choice.** A FreeText appearance is
    authored against a Base-14 font dict with **no embedded font program**
    — no `/FontDescriptor`, no `/Widths` — relying on §9.6.2.1's
    reader-shall-supply-standard-metrics rule (the PDF-1.5 should-embed
    deprecation is a *should*, honoured as a named modality choice, not a
    *shall*). The one deviation from a literal 3-key dict is `+/Encoding
    /WinAnsiEncoding` (4-key), added for deterministic Latin byte→glyph so
    the pipeline can assert real glyph pixels
    (`authored_freetext_paints_glyph_pixels_after_reload_r44`: >100 dark
    glyph pixels through the Pass 6.0 read path) and measure `/Q` against
    AFM widths ("AV" Helvetica = 13.34 pt). The dict stays program-free —
    the gate's real meaning. Base-14 is LATIN-only (no `harfrust`, R17;
    non-WinAnsi chars → "?" counted as `unencodable_chars`).
  - **(d) The auto-size VT1 heuristic is implementation-defined, counted,
    never presented as spec-mandated.** §12.7.3.3's auto-font-size (`/DA`
    text size `0`) has no spec formula (S-class spec silence). pdfce uses
    `auto_size(rect_h) = ((rect_h − 2·PAD)/1.15).clamp(4.0,12.0)`
    (`PAD = 2`, line-factor 1.15); every generated appearance reports
    `applied_autosize` so the operator sees the derived value. This is the
    general pattern for spec-silent layout parameters — pick a reviewable
    heuristic, count it, surface it (fuzzy, never sneaky).
  - **(e) Deviations/residuals (all named):** text specs live in a
    SEPARATE `TextAnnotSpec` enum, NOT folded into `MarkupSpec`, so the
    R46/R34-proven geometric `add_markup` path + its exhaustive match arms
    stay byte-unchanged (text needs `/DA`, popup, `/NoZoom`/`/NoRotate`);
    `/M`//`/CreationDate` still omitted (clock non-determinism in
    byte-compare tests); `/RC` rich text recognition-only (VT3 non-goal);
    no comb fields (Pass 7; comb = field-flag bit 25 = 16777216); X11
    certification gating still conservative; X10 encryption refusal still
    the load-time R37 seam.
  - **Ship stats: 971 workspace tests (was 939)**; fmt/clippy clean;
    GUI-free core+render (zero egui/eframe/winit/wgpu); wasm32;
    `--duplicates`; `ui-strings`; `no-network` all clean; fuzz
    `annot_author` extended (`/DA` parsing + text-appearance gen) 13,871
    runs / 61 s, 0 crashes; **no new §6.1.12 guards**; **ZERO new
    dependencies** (Base-14 only, no `harfrust`; `THIRD_PARTY_LICENSES.md`
    unchanged). **Pass 7 (Forms/AcroForm) promoted to In progress**,
    blocked on two prerequisites both dispatched in parallel (the
    §12.7.1–12.7.4 form-field spec via `pdfce-spec-librarian` + the "Forms
    (AcroForm)" acrobat parity bucket); the embedded-JavaScript posture is
    an open Pass-7 security sub-decision (recommend never-execute —
    recognize + disclose). Full entry: `ROADMAP.md` Shipped, Pass 6.2.
- **2026-08-01 (continuation 26) — Pass 7.0 shipped (AcroForm field model
  + text/checkbox fill: the forms FOUNDATIONAL SLICE) AND decision 009
  (embedded form/document JavaScript posture) filed.** Pass 7 was split on
  ship: 7.0 = the field-model read path + the dominant fill path; the
  residuals become Pass 7.1 ("completes the forms subsystem", now In
  progress). This is NOT "Forms shipped."
  - **(a) `forms.rs` — the `/AcroForm` field model (~1,050 lines, 13
    tests).** `parse_acroform(graph)` walks `/AcroForm` → `/Fields` DFS
    with §12.7.3.1 inheritance of `/FT`//`/V`//`/DV`//`/Ff`//`/DA`//`/Q`
    down `/Kids` via `/Parent`, building the dotted fully-qualified field
    name (§12.7.3.2). **Generic over `ObjectGraph`** so it runs against
    both a loaded `Document` and an `EditSession` overlay — the same
    graph-abstraction Pass 3.2 introduced.
  - **(b) The field-vs-widget MERGE is the load-bearing model fact
    (R49).** *Shape A* — a terminal field with a single associated widget
    merges field dict + widget dict into ONE dictionary (empirically ~88%
    of real fields). *Shape B* — a field carrying a `/Kids` array of
    widget annotations keeps field and widgets separate. A reader that
    always expects `/Kids` widgets breaks on the Shape-A common case; this
    is escalated as a `personal_rag/pdf` parsing lesson. `FieldFlags` bits
    are pinned verbatim by test (§12.7.4.2 Table 226 / §12.7.4.2.1: Radio
    32768, Pushbutton 65536, NoToggleToOff 16384, RadiosInUnison
    33554432; Multiline 4096, Comb 16777216; Combo 131072, MultiSelect
    2097152). XFA is **detect-only** (`XfaPresence` — recognized, never
    parsed).
  - **(c) Fill reuses the ONE §12.7.3.3 appearance generator (R49).**
    `fill_text_field` sets `/V` and regenerates `/AP` for every widget via
    Pass 6.2's `vartext.rs`, wrapped by
    `annot_author::build_field_text_appearance` (the `/DA` font resolved
    from `/DR` via `basefont_to_std14`). `set_button_state` selects
    checkbox/radio `/V` + `/AS` with no regen (on/off appearances already
    exist in the widget `/AP` sub-dict), honoring RadiosInUnison and the
    `/Off` convention. There is no second widget-only appearance path —
    the appearance half was earned in 6.2 before the field model needed
    it. R44 form-fill proof: reload → `render-page` paints 11 real glyphs,
    `annots_painted=2 forms=2`.
  - **(d) The `/P`-aware fill certification gate.**
    `check_certification_for_fill` permits fill at `/DocMDP` `/P >= 2`
    (including absent = 2 by §12.8.1 default), refuses by name at `/P 1`,
    and refuses on any `/FieldMDP` — the structural gate stays STRICT.
    Proven by `certification_p2_permits_fill_p1_refuses`. This is the
    per-`/P` refinement the Pass 6.1/6.2 X11 residual scoped, now applied
    to the fill path (annotation-add gating stays conservative until its
    own refinement Pass).
  - **(e) Decision 009 honored structurally — fill never touches the
    AcroForm dict.** Fill mutates only `/V`//`/AP`//`/AS`, so `/CO`//`/AA`
    //`/Names /JavaScript` re-emit byte-verbatim under incremental save.
    `has_additional_actions` + `calc_order_count` are surfaced
    recognition-only; the full JS-disclosure histogram is Pass 7.1.
    **FORWARD POINTER (added 2026-08-03, decision 020 §1.2.6/§7.2):**
    this guarantee is **structural to fill, not to the AcroForm subsystem
    as a whole** — it holds because fill never writes the `/AcroForm`
    dict, not because of any test. Field **creation** (decision 020's
    Pass 20.x family, F1) must append to `/AcroForm/Fields`, which is a
    write to that same dict, so the guarantee **stops holding the moment
    that family starts** — and because it held by construction rather
    than by assertion, no test existing today will go red when it does.
    Decision 020 requires F1 to add a dedicated byte-grep test proving
    `/CO`//`/AA`//`/Names /JavaScript` still re-emit verbatim after a
    field is authored, and requires this very note to be updated when
    F1 ships so a future reader does not inherit the stronger,
    now-partial claim. See the decision-020 entry below for the full
    finding.
    **★ THE FORWARD POINTER HAS FIRED — 2026-08-07, `8e799e9`. Read the
    paragraph above as NO LONGER FULLY TRUE.** `EditSession::add_text_field`
    ships (core + CLI, text fields; **no Pass ID assigned yet** — see
    `ROADMAP.md`'s ⚠ IDENTITY UNRESOLVED entry) and it **does append to
    `/AcroForm /Fields`**, and creates the `/AcroForm` dict outright when
    the document has none. **So the write decision 020 predicted has
    happened.**
    **The guarantee splits in two, and only one half survives:**
    - **"Fill never touches the AcroForm dict" — STILL TRUE**, and still
      structural. `fill_text_field` mutates `/V`//`/AP`//`/AS` only;
      nothing in `8e799e9` changed it.
    - **"`/CO`//`/AA`//`/Names /JavaScript` re-emit byte-verbatim" — NOW
      UNTESTED after an authoring write.** Decision 020 §1.2.6/§7.2
      required F1 to add a dedicated byte-grep test proving exactly this.
      **That test is absent.**
    **Instrument named, per R87:** `crates/pdfce-core/tests/form_field_authoring.rs`
    at `8e799e9` was grepped for `/CO`, `/AA`, `Names` and `JavaScript` —
    **no match**; its eleven test functions cover parse-back, registration,
    additivity, properties, fill-through, undo, append-to-existing-form and
    four refusals, and **none names this concern**. This is a **read of the
    test file, not a run of the code** — the guarantee may well still hold
    in fact. **What is established is that nothing checks it**, which is
    the precise thing decision 020 wrote this forward pointer to prevent:
    *"because it held by construction rather than by assertion, no test
    existing today will go red when it does [stop holding]."*
    **Owed:** the byte-grep test, against a fixture that actually carries
    `/CO`//`/AA`//`/Names /JavaScript`. Until it exists, do not cite
    decision 009's structural honouring as covering the authoring path.
    **★ RE-MEASURED 2026-08-07 at `bca60c9` (Pass 20.2 + 20.3) — STILL
    ABSENT, AND THE EXPOSURE HAS TRIPLED.** `add_check_box` and
    `add_choice_field` ship, and **both append to `/AcroForm /Fields`**,
    so **three** verbs now write the dict that was previously written by
    none. `crates/pdfce-core/tests/form_field_authoring.rs` was re-grepped
    at `bca60c9` for `/CO`, `/AA`, `Names` and `JavaScript` across all
    **34** `#[test]` functions — **no match**. Same instrument, same
    caveat (**a read of the test file, not a run of the code**), same
    conclusion, three times the surface. **This paragraph is updated here
    rather than only in `ROADMAP.md` under the *Update protocol*'s
    same-filing propagation duty** — a forward pointer that fired once and
    was then left to age is the exact failure the duty exists to prevent.
    **★ DISCHARGED 2026-08-07 (Pass 20.0 + Pass 20.1 (completion),
    `a3d885b` + `f809857`) — the owed byte-grep test now exists, WITH A
    NARROWING this paragraph must carry alongside it.** New fixture
    `js-carriers-form.pdf` + test
    `authoring_a_field_leaves_the_javascript_carriers_intact` proves
    `/CO`, the field `/AA`, and both `/Names /JavaScript` streams
    re-emit byte-identical after a field is authored. **The narrowing:**
    when `/AcroForm` is a **direct dict inside the catalog** — the common
    shape, and every fixture this project owns — the object pdfce
    actually rewrites to append `/Fields` is the **catalog**, not
    `/AcroForm` in isolation, so the catalog's OTHER sibling entries
    re-serialize too and whitespace normalizes (`/Names << /JavaScript
    7 0 R >>` becomes `/Names <</JavaScript 7 0 R>>`). **No JavaScript
    content is altered and no reference breaks** — the JS streams
    themselves are never rewritten, and the name tree still names the
    same object. Decision 020 §7.2 asked for the AcroForm dict to be
    re-emitted with only `/Fields` changed; what actually holds is the
    same guarantee **one object up**, at the catalog, with whitespace
    (not structure, not content) as the disclosed exception. **Read the
    guarantee, going forward, as: decision 009's byte-verbatim JS-carrier
    promise is now TEST-ENFORCED for the authoring path, not merely
    structural-by-construction as it was for fill alone** — the
    distinction this whole forward pointer exists to track. Full build
    record: `ROADMAP.md`'s *Shipped* entry, same date.
    **§4.1's "read from the crate on 2026-08-05" snapshot is now one
    filing further stale** — `Field.parent`, `forms_author.rs` and the
    resolver types are not reflected there. Flagged rather than resynced
    in this filing; a full §4.1 resync was not part of this dispatch's
    scope.
    **★ RE-MEASURED 2026-08-07 at `817b268` (Pass 20.2 COMPLETE) — the
    discharge above HOLDS, and the surface is now FOUR authoring verbs
    plus TWO deletion verbs.** `EditSession::add_radio_button` joins the
    three `add_*` verbs and writes `/AcroForm /Fields` through the same
    `resolve_field_path` path, so it is covered by the same
    `authoring_a_field_leaves_the_javascript_carriers_intact` test and
    the same catalog-whitespace narrowing — **no new exposure, because no
    new write path.** **`delete_field` / `delete_widget` are a genuinely
    new shape and are stated as such:** they are the first verbs that
    *remove* entries from `/AcroForm /Fields` rather than appending to
    it, and they prune grouping nodes left childless. **The JS-carrier
    guarantee is not re-derived for them by the append-path test** — what
    covers them instead is the deletion suite's own **raw-byte**
    dangling-reference assertion (deliberately not routed through
    `parse_acroform`, per **R159**), which reads `/Fields` and every
    page's `/Annots` from the serialized document and **first proves its
    own instrument** by re-deriving the pre-deletion state and asserting
    three widgets were named there (**R162**). **Named precisely so a
    future reader does not read the 2026-08-07 discharge above as
    covering the removal path**; it covers the append path, and the
    removal path is covered by a different, byte-level oracle.
    **New `pub` surface this filing, for §4.1's eventual resync:**
    `NewRadioButton` (+ `selected`, `with_tooltip`, `declining_tooltip`,
    `with_group_flags`, `with_flags`), `build_radio_button_appearances`,
    `FieldDeletion` (`widgets_removed`, `field_removed`,
    `selection_cleared`, `emptied_parents`), and the three
    `EditSession` methods. All checked against
    `D:\dev\rag\rust\rust-style-guide-and-api-guidelines.md` (rule 10).
  - **(f) Additivity preserves R34/R46.** A new module + additive
    methods/variants + one new `pub fn`; the re-emission path and
    `add_markup`/`add_text_annotation` are byte-unchanged. Full-corpus R46
    re-run 2026-08-01 post-7.0 over `fixtures/external` = **GATE PASS,
    additivity confirmed by measurement** (fill authors new `/AP` streams
    via the proven §12.7.3.3 generator; the re-emission path is
    byte-unchanged), discharging the "R46 re-run owed" residual. R34 (Pass
    3.0 roundtrip) accepted as additivity-preserved, not separately re-run.
  - **DECISION 009 (embedded form/document JavaScript) — the security
    posture, filed this continuation.** Archived at
    `docs/decisions/009-forms-javascript-posture.md`; discharges the
    decision-008 §5.1 embedded-JS scope trap and the Pass 6.2 open
    sub-decision. **Outcome: NEVER execute embedded PDF JavaScript** —
    field `/AA`, document `/AA`, `/OpenAction`, `/Names /JavaScript`,
    built-in or custom, on load or interaction. This is fully
    ISO-conformant: §12.6.4.16 is a **"hollow shall"** — it mandates
    execution but defines no JS semantics/API/DOM/security model
    (deferring to two external non-ISO documents), specifying only the
    carrier (Table 217) and hook points (§12.6.3, `/AA`, `/CO`, `/Names
    /JavaScript`); there is no normative JS behavior to conform to.
    **Phased hybrid:** posture A (recognize + classify + disclose +
    byte-exact round-trip, zero execution) is the mandatory floor and
    Pass 7's entire JS scope; posture B (native Rust recompute of an
    exact-match whitelist — `AFSimple_Calculate` SUM/AVG/PRD/MIN/MAX
    changes `/V`, `AF*_Format` changes display only — opt-in,
    off-by-default per document, every recompute a reviewable/undoable
    `EditSession` edit leaving the source script in place) is deferred to
    a demand-driven Pass 7.x; posture C (a sandboxed JS engine) is
    REJECTED and made a standing prohibition (re-imports the attack
    surface Adobe's broker process contains; hook points reference
    `/URI`//`/SubmitForm`//`/ImportData`//`/Launch` which R12/R13 forbid;
    nothing to conform to). Adds **standing rules R53–R57** (decision
    009's R-JS-1…R-JS-5, renumbered next-free after R52). Spec
    prerequisites (verify §12.6 carrier/hook coverage + formalize the
    hollow-shall finding via `pdfce-spec-librarian`; source the `AF*`
    helper shapes via `pdfce-acrobat-librarian`; confirm PDF/A forbids JS
    actions) are queued for Pass 7.x, non-blocking for posture A.
  - **Ship stats: 601 `pdfce-core` lib tests (was 582; +13 model +6
    fill)** + integration green; fmt/clippy clean (core + cli); GUI-free
    core+render (zero egui/eframe/winit/wgpu); **ZERO new dependencies**;
    fuzz target 13 `form_model` 1,306,476 runs / 61 s, 0 crashes;
    real-corpus `list-fields` clean on all `/AcroForm` files; veraPDF
    §6.1.12 two new guards (`MAX_FORM_FIELDS = 500_000` /
    `MAX_FIELD_TREE_DEPTH = 64`) are pure memory backstops (corpus max ≈
    63 fields/file). **Pass 7.1 promoted to In progress.** Full entries:
    `ROADMAP.md` Shipped Pass 7.0 + Standing rules R53–R57.
- **2026-08-01 (continuation 27) — Pass 7.1 shipped (form flatten +
  FDF/XFDF + choice fields + regenerate-all): the AcroForm subsystem CORE
  is COMPLETE.** 7.0 (field model + text/checkbox fill) + 7.1 (flatten +
  data interchange + choice fields + regenerate-all) finish the forms
  core; the remaining forms items (GUI form-fill slice, field
  auto-detection, posture-B native recompute) are FOLLOW-UP SLICES tracked
  in Backlog, not core. New surface: **`fdf.rs`** (~700 lines — FDF §12.7.7
  reusing `crate::parser::Parser`; XFDF via a hand-rolled ~200-line scoped
  XML reader, ZERO new deps per rule 13); `edit.rs`
  (`set_choice_value`/`regenerate_appearances`/`flatten_fields`/
  `export_form_data`/`import_form_data` + flatten helpers + §12.5.5
  `fit_matrix_for`; `RegenOutcome`/`ImportOutcome`/`FlattenOutcome`);
  `forms.rs` (`scan_javascript` + `FormJavaScript` histogram);
  `writer/content.rs` (`ContentBuilder::invoke_xobject`, `/Name Do`); CLI
  `regenerate-appearances`/`flatten`/`export-data`/`import-data` + choice
  routing + `|`-multi-select fill.
  - **(a) Flatten burns in by overlay-APPEND, not content-stream surgery —
    the load-bearing design fact of the Pass (full record §5.8).** Flatten
    appends a NEW overlay content stream to the page `/Contents` array that
    `Do`-invokes the widget's existing `/AP` `/N` as a page XObject
    (`invoke_xobject`), rather than rewriting the existing page content
    stream. The pre-existing content bytes are never re-serialized, so
    **the R46 re-emit-everything gate finds ZERO flattened-page
    exceptions** (GATE PASS over `fixtures/synthetic` + `fixtures/external`,
    all divergences the known value-preserving `-0`→`0` re-spellings, 0
    corruptions). This is MORE minimal-diff than the in-place surgery the
    Pass scope anticipated. **General pattern:** overlay-append beats
    content-stream-surgery for ADDITIVE burn-in; reserve in-place surgery
    for REMOVAL (Redaction, Pass 8 — the R46 named exception). The two are
    mirror images: flatten adds without rewriting; redaction removes and
    must rewrite (and decompose containers, §5.7).
  - **(b) R48 honored, STRICT cert gate for flatten.** Incremental flatten
    leaves the field dict recoverable in the prior revision (disclosed);
    `--full-rewrite` output has no `/FT`/`/Tx` yet renders the burned
    value. Flatten refuses on ANY enforced `/DocMDP` (incl. `/P 2`
    certified, by test) — the STRICT gate, NOT the fill path's `/P >= 2`
    permit, because flatten is a STRUCTURAL change, not a value fill.
  - **(c) FDF/XFDF interchange with ZERO new dependencies.** FDF is
    PDF-syntax, so the reader reuses `crate::parser::Parser`; XFDF gets a
    hand-rolled ~200-line scoped XML reader (5 predefined entities, numeric
    char refs, comments, `<?xml?>`/DOCTYPE skip, MAX_XML_DEPTH-guarded) —
    classified per rule 13, `quick-xml`/`roxmltree` declined for a reader
    this small and scoped. Round-trip: fill → export FDF+XFDF → re-import →
    identical `/V` + regenerated appearances; import SKIPS fields the doc
    lacks (counted, never an error).
  - **(d) Choice-field value matrix.** Single-select combo → `/V` = EXPORT
    value, `/I=[idx]`, appearance shows DISPLAY value; multi-select list →
    `/V` array + `/I` array; single-value on a multiselect-required field →
    `ChoiceRequiresMultiSelect` refusal; unknown value on a non-editable
    field → `ChoiceValueNotInOptions`; editable combo (`Combo|Edit`)
    accepts free text with no `/I`.
  - **(e) JS-disclosure histogram = posture A only (decision 009).**
    `scan_javascript` + the `FormJavaScript` histogram COUNT all
    field-level JS actions (recognition-only) with a loud stderr flag on
    any network/launch `/AA` action; NO whitelist recompute — posture B
    stays a demand-driven Pass 7.x follow-up.
  - **(f) Deviations/residuals (named):** flatten overlay-append is a
    POSITIVE deviation (more minimal-diff than scoped); list-box
    multi-select appearance is a simplified display-text newline-join, not
    the §12.7.4.4 highlight-rectangle rendering; corpus flatten-burn
    coverage thin (synthetic fixtures + unit tests carry it); import
    applies as per-field undoable commands, not one atomic
    `ImportFormData`.
  - **Windows toolchain gotcha (RAG-escalated `D:\dev\rag\rust\`):** adding
    the CLI subcommands overflowed the DEBUG `pdfce-cli` main-thread stack
    (clap's `debug_assert` command-tree recursion vs the MSVC ~1 MB main
    stack), surfacing as `TryFromIntError(NegOverflow)` in integration
    tests; fixed by running `main()` on a 16 MB worker thread.
  - **Ship stats: 1,010 workspace tests (core lib 620, was 601)**;
    fmt/clippy clean; GUI-free core+render (zero egui/eframe/winit/wgpu);
    wasm32; `--duplicates`; `no-network` clean; `ui-strings` N/A (no GUI
    changes); R34 (Pass 3.0 identity) green + R46 re-emit-everything GATE
    PASS (additive — flatten appends); fuzz target 14 `fdf_parse` 624,202
    runs / 61 s, 0 crashes; veraPDF §6.1.12 N/A (MAX_XML_DEPTH is
    XFDF-only, outside PDF-conformance scope); **ZERO new dependencies**,
    `THIRD_PARTY_LICENSES.md` unchanged. **AcroForm CORE COMPLETE; Pass 8
    (Redaction) promoted to In progress** — the standing R35 obligation and
    the one truly destructive op, blocked on two prerequisites dispatched
    in parallel (Redaction acrobat-parity bucket + a redaction spec
    dispatch for container-decomposition + `/Redact`-apply semantics). Full
    entry: `ROADMAP.md` Shipped Pass 7.1; the flatten design: §5.8.
- **2026-08-01 (continuation 28) — Pass 8.0 shipped (Redaction — mark +
  apply, text + region): the highest-stakes Pass, and the cardinal rule
  held — never claim redacted what isn't.** This discharges the standing
  **R35** obligation and is the ONE operation whose contract is genuine
  REMOVAL (§5's sole deliberate exception; R46's one named content-stream-
  surgery exception). New surface: **`redact.rs`** (the self-contained
  advance-preserving content-stream surgery interpreter + apply
  orchestration + carrier sweep + container decomposition + `RedactionReport`
  + `count_redaction_marks`); `edit.rs` (`add_redaction`,
  `mark_redactions_by_search`/`_by_pattern`, `find_matches`/
  `find_pattern_matches`); `annot_author.rs` (`RedactSpec` +
  `build_redact_mark` — RED-OUTLINE preview, never a solid fill);
  `text_extract/font.rs` (exposed per-code width/codes/to_unicode as
  `pub(crate)` for the advance computation); CLI `redact-mark` / `redact-apply`
  / `list-redactions`; fuzz target 15 `redact_apply`.
  - **(a) Redaction is the MIRROR IMAGE of Pass 7.1's flatten (§5.8/§5.9).**
    Flatten ADDS by overlay-append and never touches page content; redaction
    REMOVES and is the one op that DOES rewrite existing page content (R46's
    named exception). The surgery interpreter is a NEW code path — the R34/R46
    identity/re-emission paths (`writer/` + `content.rs`) are byte-unchanged,
    so additive preservation holds.
  - **(b) Advance-preserving content-stream surgery — the load-bearing
    correctness fact.** Deleting a text-showing run does NOT shift surviving
    same-line text: the removed `Tj` is replaced by a `TJ` offset consuming
    the exact advance `N = −Σtx·1000/(Tfs·Th)`. Proven visually (redacted
    "SECRET" is a baked black box; "dossier"/"PUBLIC" sit exactly where they
    were) AND numerically (survivor x-origin moved <1.0 pt). Escalated as a
    `personal_rag/pdf` lesson.
  - **(c) The absence-proof acceptance gate (R46 INVERTED).** Redaction's
    four-shalls are embodied as an executable gate: grep the WHOLE saved
    output — raw bytes AND every decoded content stream — for the redacted
    bytes → zero. Demo on `demo-secret.pdf`: `redact-apply` →
    `glyphs_removed=21 info_strings_scrubbed=1`; `grep "SECRET" redacted.pdf`
    = **0** (control `marked.pdf` = 3). R46 proves presence for untouched
    content; the absence test proves deletion for redacted content. Codified
    as standing rule R58 obligation 3 and §5.9. Escalated as a
    `personal_rag/pdf` lesson.
  - **(d) Container decomposition (§7.5.7 Strategy B) is necessary — R35 is
    not sufficient (§5.7).** A redacted `/Info` compressed in an `/ObjStm`
    survives verbatim in BOTH save modes unless its container is decomposed;
    the test asserts absence AND `containers_decomposed >= 1` (promote
    survivors, drop the container). Forced full rewrite (R35): output has no
    `/Prev`, prior revisions dropped, every carrier scrub rides `save_full`.
  - **(e) Refuse-not-false-redact posture.** Images in a redaction region are
    REFUSED by name (`RedactError::ImageRegion`, NO output written) rather
    than overlay-and-leave-pixels — never falsely claim a raster region
    redacted when only a masking box covers intact pixels. Form-XObject
    content in-region is NOT surgically redacted and is disclosed loudly
    (`form_intersect`), never claimed removed. XFA / structure-tree
    `/ActualText` / attachments are detect + disclose (`DISCLOSED_NOT_SCRUBBED`)
    this cut, gated by `--acknowledge-residuals` (CLI exit 10
    `REDACTION_RESIDUALS` otherwise — ui-spec §4.4).
  - **(f) Carrier sweep / report.** `/Info` + XMP SCRUBBED (asserted absent);
    object-streams + prior-revisions DROPPED-BY-REWRITE; OCG
    REDACTED-BY-GEOMETRY (ignores `/OC` visibility); XFA/struct-tree/
    attachments DETECTED + DISCLOSED. The "diligence carriers" a naive
    region-redact misses are escalated as a `personal_rag/pdf` lesson.
  - **(g) GUI — the ONE non-negotiable item shipped:** a persistent status-bar
    disclosure of unapplied `/Redact` marks, computed from the document's own
    annotations — targeting the #1 real-world redaction failure (saving a
    marked-but-not-applied document believing it is redacted). The GUI
    apply-button + canvas marking are DEFERRED to the named GUI follow-up
    (they depend on the Pass 6.1 canvas tool-mode that never shipped; the
    engineer correctly did NOT build a parallel drag tool).
  - **NEW STANDING RULE R58 (generalizes R35 — the ui-specialist finding).**
    Every removal/scrub operation rides R35's forced FULL REWRITE, including
    any future Sanitize / Remove-Hidden-Information Pass, because an
    incremental save leaves the "removed" content recoverable in the prior
    revision. Full text: §5.9 (which generalizes §5.2's R35); ROADMAP Standing
    rules R58.
  - **(h) Deviations/residuals (all disclosed, none silent):** image
    refuse-not-clear (named safe choice); `/RO`+`/OverlayText` burn-in
    deferred — apply draws the `/IC`/default-black fill (Acrobat default),
    overlay-text LABEL not drawn (COSMETIC only, content removed regardless,
    disclosed at mark time); form-XObject in-region not surgically redacted
    (disclosed); XFA/struct-tree/attachments detect+disclose not scrubbed;
    GUI apply-button + canvas marking deferred.
  - **Ship stats: 1,018 workspace tests (+8)**; fmt/clippy clean workspace-
    wide; GUI-free core+render (zero egui/eframe/winit/wgpu); wasm32;
    `--duplicates`; `no-network`; `ui-strings` clean; R34/R46 additive-
    preserved (re-emission paths + gates byte-unchanged); fuzz target 15
    `redact_apply` 9,262 runs / 61 s, 0 crashes (multi-byte CID, nested q/Q,
    overlapping/degenerate quads, all/none covered); **ZERO new
    dependencies**, `THIRD_PARTY_LICENSES.md` unchanged; GUI PID 40828.
    **MILESTONE: read → write → edit → extract → annotations → forms →
    redaction ALL shipped. In progress advances to decision 010** (post-
    redaction priority — KenAgent consultation IN FLIGHT: vector/Inkscape
    editing vs GUI-editing consolidation vs render-fidelity verification vs
    encryption). Full entry: `ROADMAP.md` Shipped Pass 8.0; the surgery/scrub
    design: §5.9; the mirror-image framing: §5.8.
- **2026-08-01 — Post-redaction priority decided (decision 010; the
  KenAgent consultation of continuation-28 RETURNED).** Full record:
  `docs/decisions/010-highest-value-investment-after-the-editing-arc.md`.
  Consulted + archived; scopes Pass 11 (dispatched) and the forward
  sequence.
  - **(a) DESTINATION UNCHANGED, PATH AMENDED — the framing.**
    Vector/Inkscape editing (decision 008's candidate **E** / **Pass 9**)
    remains the highest-value major investment AND pdfce's distinctive
    purpose — that destination does not move. What the accumulated
    GUI-editing + render-verification debt changes is the PATH to it. The
    build order becomes the three-Pass sequence **C → B → A**: **Pass 11**
    (render-fidelity verification) → **Pass 12** (canvas-interaction
    foundation + editing-GUI consolidation) → **Pass 9** (vector editing,
    repositioned onto C+B, keeping its decision-008 Pass ID). Decision
    010's candidate letters **A–E are LOCAL to that record and DIFFER from
    decision 008's A–F** — do not conflate (010-A = 008-E = vector; 010-B =
    GUI consolidation; 010-C = render-fidelity; 010-D = encryption; 010-E =
    signatures).
  - **(b) AMENDS decision 008's revisit-trigger-7.** Decision 008 named a
    clean jump straight to Pass 9 after Pass 6.1; decision 010 amends that
    trigger into the C→B→A sequence, because the render-verification and
    GUI-editing debt must be discharged before the vector-editing surface
    is built on top of it. **Decision 008's ranking and Pass IDs are
    otherwise intact.**
  - **(c) Pass 11 = render-fidelity verification (candidate C), DISPATCHED
    — no blocking spec, pure measurement.** Generalize
    `tools/annot-pdfium-diff.py` to full-page pdfium/pypdfium2 pixel-parity
    over the loadable corpus; a DOCUMENTED justified tolerance band
    reporting DISTRIBUTIONS (never a bare pass/fail); a three-bucket
    classification (benign-renderer-noise / known-disclosed-gap
    [cross-ref the Diagnostics unsupported-tally — Type3 // `sh` // SMask //
    OC // DeviceCMYK — SUBTRACT, do not re-report] / unexplained),
    enumerated by file + reason (R20); triage+fix cheap bucket-(iii) pdfce
    bugs, file the rest as counted named render-gaps; encode the known
    pdfium reference-divergences (`FPDF_FFLDraw` widgets + pdfium's
    synthesized no-`/AP` appearances that R43 makes pdfce refuse); WIRE
    into the standing gate set (re-run on every render-touching Pass);
    DeviceCMYK colorimetry characterized corpus-wide, fixed only if bounded
    (else the first named residual — re-pin decision 006 §3.4's polarity
    matrix before any colour change). **DISCHARGES the long-owed full-page
    pixel-parity remainder (Pass 1.1)** — conditionally, only if the
    harness genuinely generalizes to full-page corpus scale (not a
    "pixel-perfect" claim).
  - **(d) Pass 12 = one canvas-interaction substrate (candidate B).** The
    three accumulated named GUI follow-up slices (Pass-6.1 markup-drawing
    state machine; Pass-7 form-fill GUI; Pass-8 redaction-marking GUI) are
    RECONCILED as SLICES on ONE shared substrate — focusable canvas +
    screen↔page transform + tool-mode dispatch + hit-test/selection +
    live-preview overlay, built once, resolving `main.rs`'s Pass-1
    focusable-canvas caveat — NOT three independent buckets. Pass 9 vector
    editing later layers on the same substrate.
  - **(e) THREE new standing rules added to `ROADMAP.md` (R59/R60/R61):**
    **R59** render-fidelity gate (prove against an independent renderer at
    corpus scale before any subsystem edits content it re-renders;
    self-comparison proves agreement-with-self, not correctness; re-run on
    every render-touching Pass; residual enumerated by file+reason, never a
    threshold tuned to pass — W14); **R60** one-canvas-interaction-substrate
    (exactly one focusable-canvas/transform/tool-mode/hit-test/selection/
    overlay; markup/form-fill/redaction/vector all layer on it; a second
    parallel path forbidden — R49 applied to interaction); **R61**
    Inkscape-is-behavioral-reference-only (GPL-2.0-or-later, never a
    dependency/code-source/GUI-mimicry; `pdfce-inkscape-librarian` catalogs
    capability/behavior/limits, `pdfce-ui-specialist` designs the UI
    independently — formalizes the prior binding ROADMAP note).
  - **(f) `pdfce-inkscape-librarian` + `Inkscape_Features` RAG
    COMMISSIONED 2026-08-01** (in parallel, another agent creating the
    agent file + scaffold at `D:\Dev\Rag-Specialized\Inkscape_Features\`) —
    closes decision 008 §11.4's previously-unowned Inkscape-catalog item,
    so the capability catalog exists before Pass 9 is scoped. Registered in
    the project agent roster (`CLAUDE.md`'s "Project agents" table). It is a
    private development-reference corpus (same posture as the Acrobat
    Features RAG) — never shipped, never committed to the pdfce repo.
  - **(g) Encryption (Pass 5) = candidate D**, stays fallback/interleave
    (unchanged by decision 010, retains its decision-007 ID);
    **signatures (Pass 10) = candidate E, unchanged-last.** Full entry:
    `ROADMAP.md` In progress (Pass 11) + Next up (Pass 12 → Pass 9);
    standing rules R59–R61.

- **2026-08-01 — Pass 11 SHIPPED (render-fidelity verification harness) +
  operator reprioritization to a measurement/dimensioning beta (decision
  011 in flight).** Pass 11 (decision 010's candidate C) shipped as PURE
  MEASUREMENT — zero Rust touched, zero new pdfce dependency (pypdfium2
  dev-tooling only, out-of-tree, not vendored, absent from
  `THIRD_PARTY_LICENSES.md`). Full record: `ROADMAP.md` Shipped (Pass 11).
  - **(a) The harness.** `tools/render-parity/` (out-of-tree, mirroring
    `tools/content-identity/`) drives `pdfce-cli render-page` + pypdfium2,
    aligns rasters, computes per-channel per-pixel deltas over the full
    loadable corpus (2,914 files → 2,890 pages at 125 DPI, content-only;
    ZERO panics/timeouts; 24 skips = unloadable `fail-*` files). Replaces
    the self-comparison round-trip oracle (which proves pdfce agrees with
    *itself*, not that it matches an independent renderer) with a measured,
    bucketed, by-file/by-reason fidelity report — the correctness oracle
    Pass 9 vector editing newly requires (first subsystem whose acceptance
    test is independent *visual* fidelity).
  - **(b) The area-fraction tolerance band (the analytical core).** Metric
    `frac_over_32` = fraction of pixels with max-channel |delta| > 32/255.
    Benign AA/hinting/sub-pixel noise is confined to a thin edge band
    (small AREA) even where edge pixels swing full-range, so the
    noise-robust discriminator is AREA-fraction, not max per-pixel delta.
    Band = p99.9 of `frac_over_32` over the 1,728 clean-by-construction
    pages (zero disclosed gaps + no DeviceCMYK) = a property of the
    known-benign population, so it CANNOT be tuned to make a bug pass (W14
    structurally satisfied). This run: band 0.0294; clean-floor mean
    0.00096 / p95 0.0022 / p99 0.0098. The report prints the distribution,
    never a bare pass/fail.
  - **(c) Three buckets.** (i) benign-renderer-noise 2,840; (ii)
    known-disclosed-gap 49 (cross-referenced against the EXISTING
    Diagnostics tally so already-counted gaps are SUBTRACTED, not
    re-reported); (iii) **unexplained-divergence 1** = `A019-pdfa2-pass-a.pdf`
    (a form-XObject triangle vertex at x ~= `f32::MAX` under the CTM ->
    pdfce rasterizes a spurious cyan bar, pdfium clips it). Filed as a
    named counted render-gap (R20/R27), NOT fixed — the clamp/reject-policy
    call is a `pdfce-render` R34-risk decision, Pass-9-adjacent; the
    measurement-only non-goal binds.
  - **(d) DeviceCMYK = FIRST named residual (NOT fixed).** DeviceCMYK-only
    pages diverge 3.0x the clean-page mean; the delta lights the whole
    filled area uniformly with polarity IDENTICAL (§3.4 / R29 holds) — the
    naive additive `Rgb::from_cmyk` vs pdfium `AdobeCMYK_to_sRGB1` gap.
    Filed as a follow-up colour Pass; decision 006 §3.4 polarity matrix
    must be re-pinned BEFORE any colour change (006 revisit-trigger 7;
    don't confound colour with the harness build). Both residuals filed as
    Backlog items.
  - **(e) R59 discharged for the first time.** `--gate --max-unexplained
    <baseline>` returns non-zero when the unexplained count rises; baseline
    = 1 (the A019 file), verified PASS. Documented as a REQUIRED re-run on
    every render-touching Pass (the R34/R46 pattern), especially Pass 9.
    Local-corpus gate (pypdfium2 not in CI, like content-identity /
    roundtrip). Reference-side pdfium quirks (`--annots` mode:
    `FPDF_FFLDraw` widgets + synthesized no-`/AP` looks R43 refuses) are
    bucketed reference-side (Y2), never charged against pdfce.
  - **(f) Pass 1.1 pixel-parity remainder DISCHARGED.** The harness
    genuinely generalizes to full-page corpus scale (per-channel per-pixel;
    full loadable corpus; first-page coverage of every file; multi-page via
    `--pages-per-file 0`, demonstrated) — meeting decision 010's exact bar.
    Scope named precisely (first-page corpus coverage + a multi-page knob),
    NOT overclaimed as exhaustive-multi-page or pixel-perfect. Struck from
    the SESSION_LOG "still open" lists going forward.
  - **(g) Reprioritization — measurement/dimensioning beta (decision 011,
    IN FLIGHT).** Operator requested a beta (scaled dimensions + vector
    selection/snapping + basic vector editing) as his first usable
    deliverable; its architecture is being decided via KenAgent as decision
    011. The beta PULLS FORWARD decision 010's Pass 12 (candidate B) + the
    first slices of Pass 9 (candidate A) and adds a new dimensioning
    subsystem — the mechanism is decision 010 revisit-trigger 3 (operator
    wants vector editing sooner, now on a *corpus-measured* render rather
    than merely spot-checked). Decision 010's C -> B -> A sequence CONTINUES
    after the beta; Pass 11 (C) is now shipped so the render is verified for
    the editing work. The beta's Pass IDs/slices are defined by decision
    011, not here.
  - **Gates:** `cargo fmt --check` clean; `cargo tree` core+render GUI-free;
    ZERO Rust delta -> clippy/test/R34/R46 unmoved by construction; no
    `Cargo.toml` change -> `THIRD_PARTY_LICENSES.md` unchanged;
    deterministic/locale-invariant (sorted files, fixed DPI, no clocks).
    RAG escalations: `C:\personal_rag\pdf\` (area-fraction-not-max-delta
    tolerance-band methodology); `D:\dev\rag\rust\` (`nohup`-detach
    background-sweep gotcha).
- **2026-08-01 (GUI-polish interlude + launcher)** — An operator-requested
  GUI polish + launcher interlude shipped (see `ROADMAP.md` Shipped,
  `SESSION_LOG.md` continuation 31). NOT a feature Pass — `pdfce-gui` +
  `ui_text.rs` only, ZERO new deps. Two items with lasting architectural
  weight: **(a) canonical run entrypoint** — `D:\Dev\pdfce\pdfce.bat` +
  `pdfce.ps1` at the repo root are now the double-clickable / drag-a-PDF /
  `pdfce.bat [file]` launchers (each `cd`s to repo root, `cargo build
  --release -p pdfce-gui` as a freshness check, then `Start-Process` the exe
  detached). **(b) A named data-safety relationship, now formally tracked as
  a standing-UX-rule gap (Backlog):** true **in-place Save** stays
  deliberately GATED on an **autosave / crash-recovery scratch file**
  existing; until that lands, "Save a copy" is the only save affordance —
  the conservative, non-destructive-by-default posture (§5's spirit applied
  at the GUI layer: never overwrite the source without a recovery net). This
  is NOT cosmetic polish; it is an open crash-safety obligation surfaced,
  named, and filed rather than silently deferred.
- **2026-07-31 — Root-cause font fix (NUL-misroute) + operator-supplied
  fonts (decision 012) SHIPPED.** Two coupled font-layer changes. Full
  records: `ROADMAP.md` Shipped (Font-fix; Operator-supplied fonts);
  `docs/decisions/012-operator-supplied-fonts.md`.
  - **(a) The root-cause fix.** A subset CIDFontType2 (embedded TrueType, no
    `cmap`, legal per §9.7.4.2) was misrouted to the CFF parser because
    font-program format detection trimmed leading whitespace **including
    NUL** before magic-sniffing — stripping the leading NUL of the sfnt
    magic `0x00010000` so `01 00 …` matched bare-CFF magic. Fix: match binary
    magics on RAW bytes; trim only on the Type 1 `%!` text path; never NUL.
    Class impact: all embedded TrueType from SolidWorks/AutoCAD/Office CAD.
    **skrifa stays 0.42.1 pinned — the bug was pdfce-side routing, no bump.**
    Verified corpus-wide by the R59 render-parity gate: font-unsupported gap
    **7→0**, unexplained **1→1** (no regression), band re-derived
    0.02942→0.02963. New `Diagnostics::fonts_unsupported_by_reason`
    (Type3/NonIdentityCmap/VerticalWriting/CompositeNotEmbedded/
    UnknownSubtype/UnusableProgram). This graduates the R68 standing rule
    (embedded font programs route to the correct parser or fail clean; a
    magic/variant disagreement is a gate failure) + the `tools/font-parity/`
    harness that guards it.
  - **(b) Operator-supplied fonts (decision 012 first cut).** Non-embedded,
    non-Base-14 SIMPLE fonts render from an operator-supplied folder, riding
    the `FontEnvironment.named` seam decision 004 §5.3 built for exactly
    this. `LoadedFont.substituted: bool` → `GlyphSource {Embedded, Bundled,
    Supplied}` (three trust levels, R63); `Diagnostics.glyphs_supplied` /
    `supplied_fonts` distinct from the bundled counters; `substitute_face`
    retries after `strip_subset_tag`; `face_names()` on the one skrifa parser
    (R21). The **shell** (`pdfce-gui`/`pdfce-cli`) owns the `std::fs` folder
    walk and the setting; `pdfce-render` stays **bytes-in** (R62) so R10
    (platform-clean core/render), R11 (wasm32), and R19 (deterministic-by-
    default) all hold; `pdfce-core` untouched. Positions still come from
    `/Widths` — supplied improves *shapes*, not *positions* (R63). Adds
    standing rules **R62–R66** (renumbered from the record's proposed R61–R65;
    R61 was taken by decision-010's Inkscape rule). Named fast-follows: FF1
    OS-font enumeration (R66 opt-in), FF2 composite/CID via the Unicode route
    (R65), FF3 descriptor auto-routing. Composite non-embedded stays a hard
    skip (`CompositeNotEmbedded`). ZERO new deps. **Recorded connection:**
    decision 012 is the enabler for the ★ NEXT MAJOR FOCUS Acrobat
    text-editing subsystem — a typed/edited glyph run needs the font
    available to draw it.
  - **OWED code follow-up:** the operator-supplied-fonts `pdfce-render` doc
    comments cite the record's proposed R61/R62/R63; they must be updated to
    the assigned R62/R63/R64 (recorded in ROADMAP Standing rules + SESSION_LOG).
- **2026-07-31 — Cross-reference recovery decided (decision 013); Pass 13a
  SHIPPED (negative result), Pass 13b IN PROGRESS.** The #1 real-world
  robustness fix: 605/712 (85%) of real-file load failures are a missing
  rebuild-by-scan xref-recovery path. Full record:
  `docs/decisions/013-xref-recovery.md`; `ROADMAP.md` Shipped (Pass 13a) +
  In progress (Pass 13b).
  - **(a) Headline finding (Pass 13a, negative result).** pdfce's classic
    xref-table parser is already CRLF-correct for all three §7.5.4 EOL forms
    (SP CR / SP LF / CR LF); the strong CRLF failure correlation is
    **offset-shift corruption** (LF→CRLF text-mode conversion invalidating
    every stored byte offset incl. `startxref`), **NOT a parser bug**. 9
    synthetic legal-EOL fixtures all parse; 547/567 sampled real failures are
    offset-shift; 0 genuine parser bugs. So rebuild-by-scan (Pass 13b) carries
    essentially all the recovery, and Pass 13a is a cheap disambiguation. No
    parser code changed (tests + fixtures + tools only).
  - **(b) Pass 13b design (in progress).** A `pdfce-core` recovery module
    firing ONLY on the strict-load error path (clean files untouched by
    construction); two-phase scan (file-level `N G obj` last-wins, then ObjStm
    pair-table); trailer from last `trailer` or synthesized from `/Type
    /Catalog`; re-checks `/Encrypt` after rebuild and still refuses; subsumes
    the offset-start header case (decision 007 §10 item 6) for free. Non-
    normative reader-robustness policy (no ISO clause defines a recovery
    algorithm) grounded in universal reader behavior; bounded (R25), fail-
    clean (R27), disclosed + counted (R20). NO new dependency (reuses
    `parser.rs` + existing filters/objstm).
  - **(c) The §5 interaction → R67.** A recovered document forces a full-
    rewrite save (`save_incremental` refused by name); §5.10 records the full
    contract, marked pending Pass-13b ship. Standing rule **R67** (renumbered
    from the record's proposed R59, which was taken by decision-010's render-
    fidelity gate) — the third sibling of R35 (redaction) and R58
    (removal/scrub) in the forced-full-rewrite family.
- **2026-07-31 — Acrobat-style in-place text editing decided (decision
  014); Pass 13.x renumbered to Pass 14.x.** The operator's ★ NEXT MAJOR
  FOCUS directive (Backlog, filed 2026-08-01) is now a full architecture
  decision, archived `docs/decisions/014-acrobat-text-editing.md`. Full
  record: `ROADMAP.md` "Next up" (Pass 14.x) + Standing rules (R69–R74);
  `ARCHITECTURE.md` §5.11 (new).
  - **Renumber, recorded explicitly.** The record proposes "Pass 13.x
    (13.0–13.3)"; 13.x was already assigned to xref recovery (decision 013,
    Pass 13a + 13b) by the time this decision was filed. Librarian assigned
    the next free MAJOR number, **Pass 14.x** (14.0 read-only model + block
    recognition; 14.1 in-place edit + single-line relayout + font-on-edit
    gate + CLI `edit-text`; 14.2 formatting on selection; 14.3 edit UI on
    the Pass 12.0 canvas).
  - **Model — M-hier (Run→Line→Block), derived and reviewable.** A NEW
    `pdfce-core` module clusters Pass 4's positioned-glyph extraction into a
    hierarchy (baseline-Y lines, x-band columns, indent/leading
    paragraphs), reusing `layout.rs`'s three ratios. Every Run/Glyph gains
    provenance (source show-operator identity, byte span, full text-state)
    — the substrate the surgery needs. Everything is DERIVED (§14.8
    S1-S9), counted, and reviewable (rule 4) — never silently
    authoritative.
  - **Edit mechanism — E-surgery, not overlay.** Extends Pass 8.0's
    advance-preserving REMOVE interpreter to REPLACE: locate the show
    operator(s), re-encode via an inverse of the §9.10.2 decode ladder,
    re-emit, preserve the §9.4.4 advance. Only edited content stream(s) (+
    changed resource/font dict) are re-emitted; R47's surgery-vs-overlay
    line gets its second sanctioned member (redaction was the first).
  - **Font-on-edit — F-refuse primary; F-substitute only as an explicit
    disclosed choice; F-embed (subsetting) DEFERRED as FF-C.** A keystroke
    applies only when the run's font can already provide the glyph:
    embedded-full (free edit), embedded-subset (edit within existing
    glyphs, refuse-and-disclose a missing one), non-embedded named simple
    (edit bounded by bundled/supplied coverage — decision 012's
    `--font-dir` is pdfce's "local font," reused verbatim), non-embedded
    composite/CID (deferred, FF-E). Ships real editing for three
    high-coverage cases WITHOUT a font subsetter; names the one refusal
    case precisely instead of faking a glyph.
  - **Relayout — RL-line first cut; reflow (FF-A/FF-B) is the
    exceed-Acrobat play.** Acrobat's offline reflow is within-block only
    and its cross-block reflow is cloud-gated + English-only; pdfce's
    offline cross-block reflow (FF-B) is a genuine capability lead, not
    parity. First cut ships single-line advance-preserving relayout only
    (line may overflow the margin, disclosed).
  - **Save mode — default INCREMENTAL (R36), explicitly NOT a fourth
    forced-full-rewrite sibling.** See `ARCHITECTURE.md` §5.11 (new) — this
    is the key structural distinction from R35/R58/R67. Truly removing
    text stays Redaction's job.
  - **Tagged PDFs — T-disclose, the Acrobat-beating property.** Preserves
    BDC/EMC + MCID wrappers around edited operators (structure-tree
    references stay valid) and discloses `/ActualText`/reading-order
    staleness, rather than corrupting the tree the way Acrobat's own
    in-place edit is known to.
  - **Standing rules R69–R74 filed** (decision 014 §5.1's six proposed
    rules, in order, no collisions against R68): text-edit-is-surgery-not-
    overlay; text-edit-is-incremental-not-a-scrub; font-on-edit-trust-
    ladder; recognized-blocks-and-reflow-are-reviewable-hints;
    tagged-edits-disclose-never-corrupt; text-model-in-core-edit-UI-in-gui.
  - **Zero new dependency for 14.0–14.2** (reuses Pass 4, Pass 8.0,
    `vartext.rs`, decision 012's `GlyphSource`, the one skrifa parser
    R21). Only FF-C (font subsetting) would add a crate, gated
    permissive-only (rule 13) with its own dependency-licensing
    escalation, flagged early.
  - **Timing.** All four gating items the operator's directive named
    (font-supply/decision 012, Pass 12.0 canvas, xref-recovery/decision
    013, the beta's Pass-12.0 foundation) are now SHIPPED — see the
    following entry. Starting Pass 14.0 is an engineering scheduling call,
    not a blocked prerequisite.
- **2026-08-01 — Pass 13b (rebuild-by-scan xref recovery) SHIPPED;
  decision 013 CLOSED.** The #1 real-world robustness fix lands: 566
  previously-strict-failing real-world files now open (1,109-file corpus:
  qpdf 639 / pdfium 331 / PDFBox 139), reason-bucketed
  (`NotAnXrefSection` 417 / `TrailerParse` 99 / `BadEntry` 20 /
  `BadXrefStream` 13 / `StartxrefNotFound` 7 / `BadStartxrefOffset` 7 /
  `MissingHeader` 3); **zero regression** on the 2,907-file veraPDF corpus
  (0 clean files diverted into recovery, verified by object-outcome
  tally); the `*-fail-*` reconciliation gate is COMPLETE (all 5 veraPDF
  status changes are PDF/A-conformance files failing a header/colour-space
  rule, never an xref-parse bug — defensible reader recovery, qpdf/pdfium
  agree). 53 real-world files with object-level corruption after a clean
  xref recovery are a named non-goal (new Backlog item, `ROADMAP.md`).
  Fuzz 21,595 runs / 0 crashes; ZERO new dependency; full record:
  `ROADMAP.md` Shipped (Pass 13b). **`ARCHITECTURE.md` §5.10 FLIPPED from
  "pending Pass-13b ship" to shipped/active** (see §5.10 above) — R67 is
  now IN FORCE, not merely filed. Two engineer-flagged deviations recorded
  at the Shipped entry: a code-comment number lag (R59→R67 in
  `recover.rs`, being discharged this session) and a deliberate,
  defensible `gen-65536` deviation (rebuild-by-scan opens some gen-65536
  files via the `BadEntry` trigger — a decision-013 target bucket, NOT the
  separate strict-parser gen-65536 tolerance question Pass 13a flagged,
  which remains open and unaffected).
- **2026-08-01 — FF-A within-block offline reflow decided (decision 015);
  AMENDS decision 014; `ARCHITECTURE.md` §5.11 FLIPPED to shipped.** Full
  record: `docs/decisions/015-ffa-within-block-offline-reflow.md`;
  `ROADMAP.md` "Next up" (★ Pass 15.x) + Standing rules (R75–R77);
  `docs/decisions/014-acrobat-text-editing.md` (amended §3/§5.3/§6, see its
  dated footnotes).
  - **Trigger.** Decision 014's Pass 14.0–14.3 in-place-editing family
    shipped complete (2026-08-01); FF-A (the reflow ladder's first rung)
    is the active thread, and `pdfce-acrobat-librarian`'s scoping surfaced
    one genuinely open question — see next bullet.
  - **The settled call: justified alignment relocates FF-B → FF-A.**
    Acrobat exposes **Justify** on its BASE (non-cloud) Edit-Text panel —
    proof it is a classic-engine, single-block capability, not a
    cross-block/cloud one. Justified is therefore a within-block alignment
    mode, a peer of left/center/right (all three already FF-A), not a
    reflow *scope*; shipping 3-of-4 alignment modes in FF-A while gating
    the fourth behind an unrelated cross-block engine would be incoherent.
    FF-B's headline narrows to cross-block + cross-page reflow only — the
    genuine exceed-Acrobat axis (Acrobat's cross-block reflow is
    cloud-gated + English-only).
  - **Line-breaking — greedy/first-fit, `vartext.rs`'s packing core
    factored (not reused as-is).** Acrobat publishes no line-breaking
    algorithm, so greedy is a free, honest, low-cost choice (Knuth-Plass
    deferred, named non-goal). The greedy packing core is factored into a
    shared breaker taking a width-measuring closure: `vartext.rs` keeps its
    Std14-AFM-width Std14 path; FF-A supplies a provenance-§9.4.4-advance
    measurer over the block model's runs. Break opportunities are
    whitespace-only — no hyphenation, no CJK breaking (FF-E).
  - **Trigger + scope — explicit operator action, exactly one recognized
    `Block`.** Reflow is derived layout the file never stated (§14.8
    S1-S9), so rule 4 requires an accept/reject step; it never fires
    automatically on edit. It coexists with, and never supersedes, Pass
    14.1's single-line relayout (the default post-edit behavior). Reflow
    never crosses into a sibling block or column band; wrap width is the
    block's own detected `bbox` width, operator-adjustable.
  - **One derived preview, one undo-able command.** A `ReflowPreview`
    (new break points, per-line `Tm`/`TD` origins, alignment, new `bbox`,
    disclosures) is an accept/reject overlay; the operator adjusts
    width/alignment/leading, re-previewing live; on accept, 14.1's
    advance-preserving surgery re-emits the block's show operators and the
    whole thing lands as ONE `CommandKind::ReflowBlock` on `EditSession`
    (undo/redo atomic, sibling to `EditText`/`FormatText`). Reject mutates
    nothing.
  - **Page overflow — disclose-and-allow, never silent-disappear, never
    hard-refuse.** A block grows top-anchored downward as lines are
    added/removed; content pushed past the page cropbox is disclosed
    ("reflow grows the block N pt past the page bottom; M line(s) fall
    outside the visible page") and, on accept, emitted as real, recoverable
    off-page content — never clipped-to-invisible, never dropped. This is
    a deliberate divergence from Acrobat, whose own documentation says
    overflow "disappears" — reproducing silent loss is exactly what rule 4
    forbids; a hard refuse would lose legitimate operator work.
  - **Alignment auto-detect + preserve — the differentiator.** A block's
    left/center/right/justified alignment is inferred from glyph
    x-positions (reusing the Pass 14.0 x-band/column geometry) and
    preserved by default through re-wrap; every inference is counted
    (`BlockDiagnostics`) and operator-overridable; a single-line block
    defaults to left + disclosed ambiguity. Acrobat has no documented
    auto-detect/preserve — a re-wrap there risks a silent left-align. This
    is a named, evidenced exceed-Acrobat property, not incidental.
  - **Minimal-diff confirmed.** Reflow re-emits only the reflowed block's
    own content-stream object via the 14.1 surgery machinery; unchanged
    lines byte-identical where provable; default save stays INCREMENTAL
    (R34/R36) — not a forced full rewrite (redaction's R35 alone keeps
    that). Tagged-block MCID/BDC/EMC wrapper preserved, staleness
    disclosed (R72), exactly as 14.1 does.
  - **Pass numbering — assigned fresh Pass 15.x (librarian's call, per
    decision 015 §6's explicit delegation).** Rather than folding into
    14.4–14.6, keeps "Pass 14.x = in-place editing" and "Pass 15.x =
    reflow" as two coherent, separately-citable families — the same
    precedent as 14.x itself being assigned fresh once 13a/13b had already
    taken 13.x. 15.0 (engine, read-only) is DISPATCHED TO BUILD NOW; 15.1
    (surgery + `ReflowBlock` + CLI) and 15.2 (canvas UI,
    `pdfce-ui-specialist` first) follow.
  - **Standing rules R75–R77 filed** (decision 015 §5, in order, no
    collisions against R74): reflow-is-explicit-reviewable-single-block-
    one-undo-command; reflow-overflow-discloses-never-disappears;
    alignment-auto-detected-and-preserved-through-rewrap. Kept as three
    rules (not folded per the decision's discretion note) — matches the
    granularity of decision 014's six rules for the same family.
  - **`ARCHITECTURE.md` §5.11 FLIPPED** from "pending Pass 14.0–14.3 ship"
    to the shipped module layout (see §5.11 above), since all four Pass
    14.x slices are complete; §5.11 also gained a forward pointer to this
    Pass 15.x reflow family.
  - **Zero new dependency.** Reuses Pass 14.0's model/geometry, Pass 14.1's
    surgery, `vartext.rs`'s packing core (factored), decision 012's
    `GlyphSource`. `pdfce-spec-librarian` is dispatched for §9.4.3 `TJ` /
    §9.3.3 `Tw` ahead of 15.1.
- **2026-08-01 — Next text-parity step prioritized + FF-D scoped
  (decision 016).** After the Pass 14.x in-place-editing family and the
  15.x reflow family shipped, KenAgent ranked the remaining fast-follows
  and scoped the top solo-startable one. Full record:
  `docs/decisions/016-ffd-add-new-page-text.md`.
  - **Prioritization:** FF-D (add NEW text as page content) is the
    recommended next build — highest product of frequency × substrate
    leverage × low-risk × solo-startable. FF-C (font subsetting) ranks
    higher on ceiling value but is operator-gated (rule 13 copyleft-dep
    + the then-open license); FF-B (cross-block reflow) is the largest
    new subsystem and least frequent; FF-H (spacing) is `should_have`.
    List-authoring needs an operator scope call.
  - **FF-D design:** add-new-text synthesizes a new `BT…ET` object
    APPENDED to the page `/Contents` array (§7.7.3.3 — original stream
    byte-verbatim), defaulting to a bundled Standard-14 face (§9.6.2.2 —
    no embedding, so FF-D needs no FF-C), routed into the same 14.x
    model/edit/format + 15.x reflow pipeline as existing page text; it
    is page-content surgery, NEVER a Pass-6.2 FreeText annotation.
    Sliced as Pass 16.0 (point-insert engine+CLI) / 16.1 (boxed+wrap) /
    16.2 (canvas UI) — all shipped this session.
  - **Standing rules added:** R78 (add-new-text is page-content
    surgery, never FreeText, one undo-able `CommandKind::AddText`); R79
    (new text uses a bundled/supplied face by name+code, no embedding,
    disclosed provenance). **Amends** decision 014 §5.3 (schedules the
    named fast-follow FF-D into the concrete Pass 16.x family).
- **2026-08-01 — License = MIT (operator decision).** `docs/LEGAL.md`
  §1 flipped from "OPEN DECISION" to **DECIDED: MIT**, as part of a
  combined operator instruction that also set the project's next work
  focus (dimensioning tool → GUI-complete; ScripTree-style icons for
  all GUI features; finish text-handling fast-follows; form-building
  tools after — see `ROADMAP.md` "In progress"/"Next up" for the full
  sequencing record). **Rationale:** MIT is maximally permissive
  (easiest third-party adoption/embedding) and is fully compatible with
  the existing dependency set — a per-dependency audit against
  `THIRD_PARTY_LICENSES.md` found every current dependency permissive
  (MIT/Apache-2.0/BSD/ISC/Zlib/Unicode), zero copyleft, so the decision
  requires no dependency rework. Implemented same-session: `LICENSE`
  file at repo root, `license = "MIT"` in `Cargo.toml`
  `[workspace.package]`, `license.workspace = true` on all four member
  crates (`cargo metadata`-confirmed). **Consequence (§9 below,
  amended):** GPL/AGPL prior art — MuPDF, Poppler, Ghostscript (see
  `docs/PRIOR_ART.md`) — is now categorically and permanently excluded
  as a real dependency; it was already reference-only in practice, but
  the license decision now forecloses the alternative (an AGPL pdfce
  unlocking them) for good. Project rule 8's license precondition for a
  public-facing commit posture is now satisfied, but pushing the
  existing local commit (`d8b3903`) or publishing a release still
  requires its own, separate operator go-ahead — not implied by this
  decision. Full record: `docs/LEGAL.md` §1/§6.1/§7.
- **2026-08-02 — Decision 018: the canvas renders the edited document
  (`pdfce_core::view::DocumentView` + `StreamSource` generalize
  `pdfce-render` over `ObjectGraph`).** Root-caused an operator
  usability report ("I don't seem to be able to click on objects," "the
  dimensioning tool didn't seem to have a way to actually set the
  dimensions") to a single shared read-path defect, not fourteen broken
  features: `pdfce-gui`'s `OpenDoc::rasterize_current` and
  `ensure_object_provider` both read `session.document()` — the BASE
  revision — so raster, hit-test, selection, snapping, annotation
  survey, and OCG-visibility all see only base geometry, while every
  editing feature shipped Pass 3.1–16.2 writes into `EditSession`'s
  in-memory overlay. **Decision:** promote the existing
  `pdfce_core::pageops::assemble::DocumentView` (already `{ graph: &dyn
  ObjectGraph, bytes: &[u8], version }`) to a top-level
  `pdfce_core::view` module; add `StreamSource { Contiguous(&[u8]) |
  Split { base: &[u8], staged: &[u8] } }` for zero-copy dispatch of a
  `ByteSpan` against either a plain buffer or an `EditSession`'s
  base+staging pair (spans provably never straddle the boundary, by
  construction of `stage_bytes`'s offset scheme); `impl ObjectGraph for
  DocumentView`. Measured, not assumed: `pdfce-render`'s entire
  `Document`-typed surface is 3 methods across 50 call sites, of which
  45 compile unchanged once the parameter type widens (`doc.resolve`/
  `doc.resolved` are already on `ObjectGraph`; only `doc.bytes()`'s 5
  sites need `StreamSource`). `pdfce_core::vector::decompose_page` and
  `ContentStream::from_page` generalize the same way, which is why
  hit-testing and the Pass 12.M1 snap engine become edit-aware from the
  identical change (they already share one `ObjectModelProvider`
  decomposition per page — no second path to diverge). **Rejected:**
  (b) re-serialize-then-reparse after each edit — not on performance,
  but because it routes VIEWING through the WRITER, so the viewer
  inherits every refusal the writer is contractually obliged to make
  (R67 refuses incremental save on a recovered document; §5.6 refuses a
  full-rewrite fallback on a hybrid file) — a recovered-and-hybrid
  document could display NOTHING, and a viewer must never be less
  capable than the parser (Pass 13b exists precisely to make such files
  openable at all). (c) GUI-side overlay compositing — cannot represent
  content-stream surgery (most of what shipped: `edit-text`, `reflow`,
  `object-move`/`-delete`, `node-move`, `redact-apply`), cannot fix the
  hit-test half at all, and would create a second appearance-painting
  path inside `pdfce-gui` that must agree with `pdfce-render`
  pixel-for-pixel — the "two decompositions quietly diverge" pattern
  `object_provider.rs`'s own doc comment already cites decision 011
  warning against, and it would erode §3's GUI-core separation in
  spirit. **Invariant impact:** §3 (GUI-core separation) — none;
  `DocumentView` moves between `pdfce-core` modules, no crate gains a
  dependency, the standing `cargo tree` gate is unaffected — the actual
  separation risk in this decision space was option (c), which was
  rejected. §5 (round-trip/minimal-diff) — none from the change itself
  (pure read path, cannot perturb saved bytes), but two hazards are
  named in the type's own doc comment so they can't be introduced
  later: `DocumentView` must never become the writer's input (the
  writer's source of truth stays `&Document` + `DirtySet`); and `Page`
  is a commit-time snapshot that must stay one (audit that every
  session-commit path funnels through `refresh_pages`, canvas vector
  edits and text-edit Accept in particular). Sliced as **Pass 17.0**
  (the core+render generalization + the two-line `pdfce-gui` fix)
  **17.1** (finish the audit of remaining `session.document()` call
  sites — confirmed live bug: `main.rs:4606`'s `count_redaction_marks`
  undercounts marks added in the current session) **17.2** (CLI parity
  + a headless preview-equals-saved oracle harness). **Proposed
  standing rules R85 (preview-equals-saved — headless oracle,
  reusing the Pass 11 raster oracle, now in force per `ROADMAP.md`
  Standing rules) and R86 (a Pass does not ship until observed working
  in the running application — PROPOSED, pending explicit operator
  sign-off, not yet binding).** Full record: `docs/decisions/
  018-edited-state-is-what-the-canvas-renders.md`; `ROADMAP.md` ★★★★
  HEADLINE FINDING note and ★ Pass 17.x entry; §4 above (forward
  pointer). Not yet built as of this entry.
- **2026-08-02 — Decision 017: two-compartment vertical panel list for
  the right-hand dock; `egui_dock` rejected permanently, `egui_tiles`
  pre-vetted and pre-approved behind a named trigger; supersedes
  continuation-19's "Properties is the single legacy floating
  exception."** Answers the operator's ask for tabbed/dockable panels
  plus a clickable layer/object tree. **Decision:** hand-roll a
  two-compartment (upper: Properties/Comments/Bookmarks; lower:
  Layers/OCGs/batch Tools) vertical row list inside the existing
  `egui::Panel::right("tools")` — no new Cargo dependency. **This
  record reverses an initial recommendation to adopt `egui_tiles`
  immediately**, kept rather than erased because the reasoning that
  produced the reversal is the reasoning that will govern a future
  re-adoption: a `pdfce-ui-specialist` review found `egui_tiles` draws
  **horizontal** tab bars only, and the dock is `default_size(320.0)`
  — ten text labels do not fit a 320pt-wide horizontal strip (they
  truncate, wrap, or scroll-hide, reproducing Acrobat's own worst
  ribbon-overload habit sideways); vertical scales by adding rows at
  zero horizontal cost and is already the pattern `tools_dock()` uses
  internally. **`egui_dock` 0.20.1 rejected permanently** (closes
  `PRIOR_ART.md`'s open 0.19.1-vs-0.20.1 version-gap question): binary
  splits only (no n-ary column/grid, and pdfce is heading to 10+
  panels); zero accessibility instrumentation repo-wide (0 hits for
  `widget_info`/`accesskit`/`keyboard`; its tab-bar `Sense` sets
  egui's `FOCUSABLE` bit, so tabs are keyboard-reachable and
  **unnamed** to AccessKit — the worst case); depends on `paste`,
  which carries RUSTSEC-2024-0436 (unmaintained); slower egui-tracking
  cadence and thinner bus factor than `egui_tiles`. **`egui_tiles`
  0.16.0 fully vetted and PRE-APPROVED** behind one named trigger (Ken
  answers decision 017 §10 Q1 with the VS Code/Blender whole-content-
  area model — MIT OR Apache-2.0, 1 new package, all transitive deps
  already present at satisfying versions, wasm32-clean, exact MSRV/egui
  match) — if the trigger fires, adopt without a new decision record,
  just a dated amendment to `docs/decisions/017-...md`. **Persistence:**
  session-only this Pass, explicitly disclosed (same posture decision
  012 set for the font-folders setting) — do NOT enable eframe's
  `persistence` feature (writes to a platform app-data directory,
  contradicting §6's single-folder-portable posture and the new R15/R82
  pairing). **Correction to §12 continuation-19:** "Properties stays
  the single legacy floating exception, never to be joined by a second"
  is now FALSE on two counts — Pass 12.M2's Dimension Groups panel
  already shipped as a second floating `egui::Window`
  (`docs/ui_specs/pass-12.M2-dimension-tools.md` §5.1), and this
  decision retires the Properties floating form entirely, folding its
  body into the new dock's upper compartment. **Dimension Groups is
  named here as the remaining floating-window holdout**, owed a
  follow-up migration into the same dock so it does not quietly become
  the new "one legacy exception" (see the inline forward-pointer added
  to the continuation-19 entry, above). Sliced as **Pass 18.1**
  (tabbed/panel shell + Objects tree + Properties selection panel +
  canvas selection feedback, bundling the mandatory
  `properties_window` → "Document Properties" rename in the same
  slice) **18.2** (`object-list` CLI subcommand) **18.3** (Measure ▾
  affordance fix). Pass 18.0 (an unrelated but adjacent zoom-invariant
  selection-tolerance + gesture-preservation bug-fix, root-caused by
  the same UI-usability review) already shipped, uncommitted — see
  `ROADMAP.md` Shipped. **New standing rules R80 (dock is a
  two-compartment host reached through one `panel_body` dispatcher),
  R81 (floating windows are transient-only — the continuation-19
  supersession), R82 (panel layout rides R15, never eframe Storage),
  R83 (no affordance without the capability), R84 (selected state is
  never colour alone)** — all in force, `ROADMAP.md` Standing rules.
  Full record: `docs/decisions/017-tabbed-dockable-panel-system.md`;
  `ROADMAP.md` ★ Pass 18.x entry. Not yet built (except 18.0) as of
  this entry. **CORRECTION (2026-08-02, same-day continuation 56):**
  Pass 18.0 is committed (`9a68d6f`), not uncommitted as stated above —
  see the follow-up entry below. Pass 18.2 has also since shipped
  (`dae0139`); 18.1 and 18.3 remain unbuilt.
- **2026-08-02 (same-day continuation 56) — Decision 018 implementation
  update: Pass 17.0 SHIPPED (`3a56b55`), two deviations from the plan
  recorded above.** Confirms the decision-018 entry above was built
  largely as designed, with two deviations discovered during
  implementation (neither invalidates the decision; both are additive
  corrections to its plan, recorded here rather than silently folded
  in, per the "decisions get dated entries, not silent edits" rule this
  §12 itself follows):
  1. **`image_codec::decode_image` also threads a `&Document` parameter**
     and needed the same delegating-wrapper generalization as
     `pdfce-render`'s other `Document`-typed call sites. The original
     decision's "3 methods / 50 call sites" measurement did not separately
     enumerate this path; it is functionally identical to the other 45
     unchanged-signature call sites (delegates through `ObjectGraph`),
     just discovered mid-build rather than during the original audit.
  2. **`DocumentView::bytes() -> Option<&[u8]>`, not `&[u8]` as the
     original design implied.** A `Split { base, staged }` view has no
     single contiguous buffer to hand back; returning either half under
     a non-`Option` `&[u8]` signature would silently return a
     PLAUSIBLE-LOOKING but WRONG slice (base-only or staged-only, with
     no compiler signal that the caller needs to handle the split case).
     `Option` forces every caller to acknowledge the split case exists.
  Additionally, **decision 018 §10 hazard 2 was CONFIRMED REAL** by the
  Pass 17.0 build's own commit-site audit (not merely a named risk any
  longer): `Commit::Move`, `Commit::Node`, and `delete_selected_object`
  were all performing genuine content-stream surgery while calling
  `ensure_object_provider` instead of going through `refresh_pages` —
  the provider therefore never rebuilt and stale `page_texture` was
  never dropped, invisible before Pass 17.0 (the canvas drew the base
  regardless of provider staleness) but would have made Pass 17.0 LOOK
  broken on the canvas specifically had it shipped unfixed. Fixed in the
  same Pass. **§4/§5 forward-pointer notes above are now updated to
  reflect implemented (not merely planned) status.** Gates: workspace
  1474 tests passing/0 failed; `cargo tree -p pdfce-core`/`-p
  pdfce-render` GUI-dep-free; roundtrip corpus 4,023 files unchanged;
  raster oracle 6566/6566; zero new Cargo dependencies. Full record:
  `ROADMAP.md`'s Pass 17.0 Shipped entry (top of Shipped).
- **2026-08-02 (same-day continuation 57) — Decision 017 AMENDMENT A:
  the §6.1 trigger FIRED — `egui_tiles` 0.16.0 is ADOPTED, superseding
  §3/§8's hand-rolled two-compartment vertical row list AS A MECHANISM
  (the requirement it served does NOT lapse).** Asked §10 Q1 (does the
  panel system own only the right-hand dock, or eventually the whole
  content area, VS Code/Blender-style), the operator answered: *"Use
  egui_tiles. You're building something to compete with Acrobat and is
  open source, and has the flexibal docking that works as well as
  inkscape's."* That is the primary trigger fired in the widest
  direction available (whole-content-area ownership, flexible docking
  as the explicit bar, not merely "more than one panel"). Per §6.1's own
  instruction, this is filed as a dated amendment to decision 017, NOT a
  new decision record — full text at the end of
  `docs/decisions/017-tabbed-dockable-panel-system.md` ("AMENDMENT A").
  **What changes:** `egui_tiles` 0.16.0 becomes a real dependency (one
  new package; MIT OR Apache-2.0, permissive — `LEGAL.md` §6.2 step 3
  applies, proceed-and-log, no operator flag needed; all transitives
  already present at satisfying versions; wasm32-clean; exact MSRV
  1.92/egui 0.35.0 match). §6.2's vetting stands and is NOT redone;
  re-verify only version-specific facts (MSRV, license, transitive set)
  against whatever `egui_tiles` release is current when Pass 18.1 is
  actually built, since `main` has already bumped `rust-version` toward
  1.95 for a future release — pin to the last release that doesn't
  exceed pdfce's own MSRV if so. `THIRD_PARTY_LICENSES.md` regeneration
  via `cargo-about` is owed when this dependency actually lands in
  `Cargo.toml` (rule 13 / `LEGAL.md` §6.3) — NOT yet, since Pass 18.1 is
  still unbuilt as of this entry. **What is superseded:** §3's vertical
  single-column two-compartment row list as the PICKING MECHANISM, and
  §8.2's "two hand-rolled compartments" implementation step — both
  replaced by `egui_tiles` containers (a vertical split pane, Layers
  above Properties by default). **What survives unchanged:** the
  underlying requirement §3 was solving — Layers and Properties (or any
  two dock panels) must be visible SIMULTANEOUSLY, because pdfce is also
  an Inkscape-parity vector editor where selecting in a layer tree and
  editing properties without losing sight of the tree is a core
  workflow (Passes 9/12.M2 already put that pairing in play). Under
  `egui_tiles` this becomes a vertical split container instead of two
  fixed compartments — same requirement, different mechanism, and the
  default layout must ship with that split already in place (do not
  make an operator discover they must drag a panel out to see both).
  `enum DockPanel` + one `panel_body(...)` dispatcher (§8.1) SURVIVES
  VERBATIM as originally designed — it becomes the `egui_tiles` pane
  payload; keep it extensible (a `Document(DocId)` payload variant must
  stay a non-breaking addition, now expected rather than hypothetical
  under the wide-content-area model). §5's PERMANENT rejection of
  `egui_dock` is unaffected — independent of this trigger, still stands
  on its own accessibility/dependency-hygiene grounds. **AccessKit
  caution carried forward:** `egui_dock`'s tab bars are Tab-focusable
  but unnamed to AccessKit (§5); `egui_tiles` 0.16.0 had the identical
  gap at its release tag but has since fixed it on `main` — the Pass
  that actually adopts the dependency must verify which side of that
  fix the pinned release falls on and supply names via
  `Behavior::tab_ui` if not. **Persistence unchanged:** session-only,
  disclosed, per §7/R82 — do not enable `egui_tiles`' `serde` feature or
  eframe's `persistence` feature yet; when R15 lands, `Tree<Pane>`'s
  own Serialize/Deserialize under fail-soft rules (missing/corrupt →
  default layout, never an error dialog or lost session) is the
  intended mechanism. **Two engine gotchas recorded for whoever builds
  Pass 18.1** (both already in the decision record, restated here so
  §12 alone is a sufficient audit trail): `Tree<Pane>` derives only
  `Clone, PartialEq` — NOT `Default`, so `std::mem::take` will not
  compile, use `std::mem::replace(&mut self.dock, Tree::empty("swap"))`;
  and `SimplificationOptions::default()`'s `prune_single_child_tabs:
  true` + `all_panes_must_have_tabs: false` makes the tab bar vanish
  when only one panel is open — override
  `all_panes_must_have_tabs: true`. **Still open, NOT answered by this
  amendment:** §10 Q2 (undock into separate OS windows/multi-monitor) —
  `egui_tiles` has no `Surface::Window` equivalent (rerun-io/egui_tiles
  issue #30); default stands, docked-only, own Backlog entry.
  **Consequence for `docs/ui_specs/pass-17-dock-and-layer-tree.md`:**
  its §A (horizontal tab-strip dock shell) is now superseded TWICE OVER
  — once by decision 017's original two-compartment design, again by
  this amendment's `egui_tiles` containers — and must not be built as
  written; its §B (object/layer tree), §C (canvas selection feedback),
  and §D (Measure ▾ affordance fix) are unaffected and remain the
  binding design for those parts regardless of shell mechanism. A
  status notice to this effect was added to the top of that ui-spec
  file itself (commit `f9bb560`). **Not yet built:** Pass 18.1 (the
  tabbed/panel shell + Objects tree + Properties selection panel) is
  still unbuilt as of this entry — this amendment changes WHAT it will
  be built with, not whether it has shipped. Full record: `docs/
  decisions/017-tabbed-dockable-panel-system.md` ("AMENDMENT A"
  section, filed by `pdfce-engineer` per §6.1's own instruction);
  `ROADMAP.md` ★ Pass 18.x entry.
- **2026-08-03 (same-day continuation 58) — Decision 018 follow-up:
  Pass 17.1 SHIPPED (`437a6f7`), finishing the `session.document()`
  audit and confirming the read-path bug class had more instances than
  the two named at Pass 17.0's ship.** `count_redaction_marks`
  (`main.rs:4606`), `need_appearances` (`main.rs:4598`), and
  `page_font_entries` (`main.rs:6078`) were all reading the base
  revision instead of `session.view()`; all three fixed, all now
  edit-aware. Two further bugs found by the same audit sweep, distinct
  in KIND from the read-path class (not "reading the wrong revision,"
  but "resolving through the wrong index space" and "pairing the wrong
  bytes with the wrong graph"):
  1. **Search-redaction resolved against the wrong page after any
     delete/reorder.** `author_text_matches` extracted match geometry
     using BASE page indices, then fed that geometry straight to
     `add_redaction`, which resolves page numbers through SESSION
     `page_slots`. After any page delete/reorder earlier in the same
     session, a search-driven redaction mark silently lands on the
     WRONG page, with fully plausible-looking geometry — nothing about
     the result looks wrong on inspection. Fixed.
  2. **Content authored this session could be extracted as empty.**
     `extract_selection` paired a SESSION object graph with BASE bytes.
     A stream authored during the current session has no corresponding
     bytes in `base`, so extraction silently returned empty content
     instead of erroring or reading the session's own staged bytes.
     Fixed.
  See §11.1's addendum above for a THIRD, structurally distinct bug (the
  `flatten_fields` multi-`ObjectWrite`-per-id overwrite) found by the
  same effort — all three were found by the SAME oracle, on its FIRST
  run, none of them hypothetical or anticipated by the original
  decision-018 design.
  **Pass 17.2 SHIPPED the same commit** — the R85 preview-equals-saved
  oracle, built as a `tools/` harness (no new public CLI surface, per
  rule 11), now covers 11 of the 12 named R85 operations. **`redact-
  apply` is the one operation R85 cannot cover, structurally, not by
  omission or oversight:** applying redaction is not an `EditSession`
  operation — it consumes a `Document` and emits a file directly (§11.2
  — redaction stops being an in-memory, undo-able command at exactly
  the point a save actually happens), so "preview equals saved" has no
  live-session left-hand side to compare against for that one
  operation. R85's own text (`ROADMAP.md` Standing rules) lists
  `redact-apply` in its operation coverage; that listing is now known
  to be aspirational for that one entry, not achieved, and is a
  structural fact about the redaction design rather than a gap in the
  oracle's implementation. **Consequence worth naming architecturally:
  there is currently no GUI flow for redaction apply at all** —
  mark-and-disclose only; applying is CLI-only
  (`pdfce-cli redact-apply`). Filed as a real product/feature gap to
  `ROADMAP.md` Backlog, not merely an oracle-coverage note. Gates:
  workspace 1521 tests passing (from the continuation-57 baseline of
  1504), 0 failed; fmt/clippy clean; `cargo tree -p pdfce-core`/`-p
  pdfce-render`/`-p pdfce-cli` GUI-dep-free; zero new Cargo
  dependencies. **Decision 018 (live-edit rendering) is now COMPLETE
  end-to-end** — Pass 17.0 (canvas renders the edited view), 17.1
  (every remaining base-read site triaged and fixed), and 17.2 (the
  oracle that proves it, 11/12 operations) have all shipped. Full
  record: `ROADMAP.md`'s Pass 17.1/17.2 Shipped entry;
  `docs/decisions/018-edited-state-is-what-the-canvas-renders.md`.
- **2026-08-03 (same-day continuation 58) — Decision 017 Amendment A,
  BUILD CONFIRMATION: Pass 18.1 SHIPPED (`f963895`), `egui_tiles`
  0.16.0 actually lands in `Cargo.toml`, `pdfce-gui` only, WITH
  `default-features = false`.** **CORRECTION to the continuation-57
  entry's vetting:** that entry instructed *"do not enable `egui_tiles`'
  `serde` feature ... yet,"* but did not separately record that `serde`
  is ON BY DEFAULT for this crate — §6.2's vetting table recorded
  license/MSRV/transitive-set facts but no default-feature-set check.
  Had Pass 18.1 added the dependency with default features left on (the
  common case when a developer doesn't think to check), the
  continuation-57 instruction would have been silently violated by
  omission, not intent. **General rule filed to
  `D:\dev\rag\rust\crate_default_features_can_silently_contradict_project_policy.md`:**
  check `default-features` explicitly against project policy as its own
  checklist item, not an emergent property of the license/MSRV/
  package-count checks. Exactly **1** new package (`egui_tiles` itself;
  all transitives already present at satisfying versions, as
  predicted); MIT OR Apache-2.0; `THIRD_PARTY_LICENSES.md` regenerated
  via `cargo-about 0.9.1` (generated, not hand-edited, per rule 13);
  `cargo tree -p pdfce-core`/`-p pdfce-render`/`-p pdfce-cli` verified
  clean. **AccessKit gap resolved in the WORSE direction than
  continuation-57 left open:** that entry noted 0.16.0 had the tab-
  naming gap AT ITS RELEASE TAG but had since been fixed on `main`, and
  asked whoever builds Pass 18.1 to check which side of that fix the
  pinned release falls on. It falls on the UNFIXED side — zero
  `widget_info`/`accesskit` hits in the pinned release's source. Names
  are supplied via `Behavior::on_tab_button` (the continuation-57 note's
  `Behavior::tab_ui` reference was slightly off — `on_tab_button` is the
  actual hook used) rather than a fork. **A further gap that cannot be
  closed downstream at all, regardless of `egui_tiles` version:** egui
  0.35's `WidgetType` enum has no `Tab`/`TabList` member, so a tab can
  be given the correct accessible name and selected state but not the
  correct semantic ROLE short of an upstream egui change — filed to
  `D:\dev\rag\egui\egui_035_no_tab_tablist_widgettype.md`. New
  `crates/pdfce-gui/src/dock.rs` (~510 lines): `enum DockPanel` + one
  `panel_body` dispatcher (§8.1) survives verbatim as the `egui_tiles`
  pane payload; default layout ships Objects ABOVE Properties as a
  vertical split, BOTH simultaneously visible per the amendment's own
  requirement, pinned by a unit test asserting both panels are present
  in `active_tiles()`. `properties_open` (a second source of truth for
  panel visibility) is deleted; `properties_window()` is retired — no
  more float-or-dock dual mode, per R80/R81. Both engine gotchas
  recorded at continuation-57 were real and handled as predicted:
  `Tree<Pane>` derives `Clone, PartialEq` but not `Default`
  (`std::mem::replace`, not `std::mem::take`); `SimplificationOptions::
  default()` needed `all_panes_must_have_tabs: true` overridden or the
  tab bar vanishes with one pane open. **Mandatory bugfix caught before
  ship:** `open_path()` did not invalidate `properties_draft`; for a
  now-persistently-mounted Properties panel (it no longer needs opening
  to appear), the failure mode would have been an operator opening a
  NEW document, seeing a stale or EMPTY metadata form left over from the
  PREVIOUS document, and clicking Apply — silently overwriting the new
  document's real `/Info` with leftover or blank values. Fixed with two
  regression tests. Object tree reuses `canvas::selection_after_click`
  verbatim for bidirectional sync and is pinned against
  `pdfce-cli object-list` (Pass 18.2) by a same-indices/same-kinds
  regression test. **Deliberately NOT done, and said so in code:** the
  dock still starts CLOSED by default; the original justification for
  that default (Properties as the sole legacy floating exception, R81)
  is now false, but flipping a startup default is a product call left
  to the operator, not taken unilaterally. §10 Q2 (multi-monitor
  undock) remains unanswered and unbuilt — `egui_tiles` grants no
  `Surface::Window` equivalent (unchanged from the continuation-57
  entry). Gates: workspace 1538 tests passing (from 1521), 0 failed;
  fmt/clippy clean; `cargo tree` invariant intact; exactly one new
  dependency, license-classified and attributed. Full record:
  `ROADMAP.md`'s Pass 18.1 Shipped entry;
  `docs/decisions/017-tabbed-dockable-panel-system.md` ("AMENDMENT A").
- **2026-08-03 (same-day continuation 60) — Decision 017 Amendment A
  follow-up: Pass 18.5 SHIPPED (`9998a6b`), delivering §B.4's core
  `pdfce-core` additions and the `hit_test_point_all` Alt+click-cycling
  API named as owed at Pass 18.4's ship. Two API-design decisions and
  one product/memory-tradeoff decision recorded, none of them reversals
  of anything, all extending `pdfce-core`'s public surface.**
  1. **New invariant: a singular "best match" query is defined as the
     structural HEAD of its plural "all matches" sibling, never a
     second parallel implementation.** `hit_test_point` is now
     `hits_front_to_back(..).next()`; `hit_test_point_all` is
     `hits_front_to_back(..).collect()` over the SAME private iterator
     — `hit_test_point(..) == hit_test_point_all(..).first().copied()`
     is therefore provably true, not a convention two future edits
     could silently break. The same shape is applied one layer up at
     the `pdfce-gui` trait boundary: `CanvasTargetProvider::hit_test_all`
     is the REQUIRED method, `hit_test` a PROVIDED default defined as
     its `.first()`. Recorded as a **general API-design rule for any
     future singular/plural query pair added to `pdfce-core`** (hit-
     testing, snap-candidate resolution, field-lookup-with-fallbacks),
     not a one-off. Full pattern + generalization:
     `D:\dev\rag\rust\define_singular_query_as_head_of_plural_query.md`.
  2. **`FontResolver` seam added to the vector-decomposition path**
     (`pdfce-core`, zero GUI dependency): `NoFonts` (prior behavior,
     unchanged, still the default `decompose(...)` delegates to) and
     `DocumentFonts` (memoizing — one `ExtractFont::resolve` per
     distinct font resource per PAGE, not per text object). Reuses
     `text_extract::ExtractFont::{codes, to_unicode}` directly — the
     same §9.10.2 simple/composite-font decode ladder `extract-text`
     already climbs — rather than standing up a second, parallel
     decoder for the identical problem. `TextObject` gains
     `preview: TextPreview` (a four-variant enum —
     `Decoded{text,truncated,lossy}` / `Undecodable` / `Unavailable` /
     `Empty` — deliberately NOT `Option<String>`, because "no text to
     show" has four semantically distinct causes and only one of them
     is actually a fact about the document) and `font: Option<TextFont>`
     (`size` is the raw `Tf` operand as the content stream states it,
     NOT the rendered glyph size — folding the text matrix's scale in
     would produce a number disagreeing with the content stream itself,
     and this layer has no glyph-metrics access to defend a "measured"
     alternative). `ImageObject` gains `pixel_size`.
  3. **Memory/work-bound decision: the text preview is capped AT
     DECOMPOSITION TIME (`MAX_TEXT_PREVIEW_CHARS = 64`), not at GUI
     display time, and owned `String`s are stored, not borrowed
     spans.** The decode loop physically stops at the cap, so a large
     `Tj` string is never fully decoded and then discarded — the cap
     bounds decode WORK, not merely result memory (worst case ≈450 B
     per text object, ≈100 B realistic; a 50,000-object page tops out
     ≈22 MB worst case / ≈5 MB realistic). Owned strings were chosen
     over spans specifically because a span-based design would need
     the source `ContentStream` kept alive for the `VectorObject`'s
     whole lifetime AND the font-decode ladder re-run on every row
     redraw — and Objects-tree rows redraw every frame in an
     immediate-mode GUI, so that cost would be paid continuously. A
     separate, smaller GUI-layer display cap (`ROW_TEXT_CHARS = 32`)
     sits on top of the core 64-char cap so a future change to either
     cap cannot silently retypeset the other.
  Also this continuation: the Pass 18.4 `ApproximateTextBounds`
  disclosure text (§4/decision-017-Amendment-A lineage, selection
  legibility) was found to be itself inaccurate — it repeated the same
  wrong text-bbox model (§0.2/§B.3 of the ui-spec) that Pass 18.4's own
  Finding 1 had already flagged as wrong, and reassured the operator a
  surprising selection was "correct" while disclosing nothing about the
  opposite, worse failure (a click on visible glyphs can MISS the
  object). Fixed (`d296666`) — disclosure only, the underlying
  hit-target geometry fix is IN PROGRESS (ui-spec §E authored, a
  builder implementing it). Gates: `cargo test --workspace` 1559 →
  1599, 0 failed; doc-tests 69 passed; fmt/clippy clean; `cargo tree -p
  pdfce-core`/`-p pdfce-render` GUI-dep-free; zero new Cargo
  dependencies. Full record: `ROADMAP.md`'s Pass 18.5 Shipped entry
  (top of Shipped) and the Pass 18.4 entry's dated correction footer.
- **2026-08-03 (same-day continuation 61) — Decision 017 Amendment A /
  ui-spec §E follow-up: Pass 18.6 SHIPPED (`1b38e34`), replacing
  `TextObject`'s glyph-origin-inflated bbox with one derived from font
  metrics. Closes the FOURTH and last named contributing cause of the
  operator's "can't click on objects" report. Two decisions recorded,
  both extensions of `pdfce-core`'s public data model, neither a
  reversal.**
  1. **`TextBoundsBasis` ships as a FOUR-variant enum
     (`FontMetrics | MetricAdvancesNominalHeight | EstimatedAdvances |
     EmBox`), not the two-basis design the ui-spec (§E) specified.**
     Judged necessary, not scope creep: a Type 3 or descriptor-less
     CIDFont has real advance widths but only a guessed height (no
     `/FontDescriptor` to source ascent/descent from); a non-standard-14
     font with no `/Widths` array has estimated, not measured, advances.
     Collapsing either case into `FontMetrics` would misrepresent the
     box's own confidence — exactly the "sentence that no longer matches
     the box" failure the ui-spec's own §E was written to prevent. Not
     hypothetical: the project's own `text/identity-h-no-tounicode.pdf`
     fixture (pre-existing, not added for this Pass) naturally exercises
     the `EstimatedAdvances` case. `EmBox` is the pre-Pass-18.6 geometry,
     preserved verbatim as the fallback for `NoFonts`/unresolvable-font/
     non-finite-`Tf`-size objects, pinned by a regression test — an
     unresolvable font is never silently upgraded to a stronger-sounding
     basis than the data actually supports.
  2. **New four-rung font-metrics fallback ladder for vertical extent
     (ascent/descent), recorded as a general resolution-order pattern
     for any future font-dependent measurement in `pdfce-core`:**
     `/Ascent`+`/Descent` (§9.8 Table 122, the spec's own Required
     fields) → `/FontBBox` `ury`/`lly` (§7.9.5, always present on any
     descriptor that exists at all) → compiled-in standard-14 descriptor
     metrics (§9.6.2.2's no-descriptor-permitted case) → nominal 1.0/
     −0.25 em, explicitly flagged as a guess. Composite (Type 0) fonts
     resolve through the descendant font's descriptor, never the Type 0
     dict's own (§9.8.1 forbids a descriptor there). Type 3 always takes
     the nominal rung regardless of whether a descriptor happens to be
     present, because its numbers, if any, live in `/FontMatrix` glyph
     space, not text space. **PDF-domain empirical finding filed to**
     `C:\personal_rag\pdf\lesson_20260803_cidfont_descriptor_ascent_descent_often_absent.md`
     **— real subsetted CIDFonts frequently omit `/FontDescriptor`
     entirely despite Table 122 marking Ascent/Descent Required, making
     this ladder load-bearing in practice, not merely defensive.**
  3. **Invariant: `advance_tx(w0, tfs, tc, tw, th)` is now the single
     shared implementation of §9.4.4's text-displacement formula**,
     used by `text_extract::page::show_code`, `redact::glyph`, and this
     Pass's bbox-computation decompose walk — consolidating what was
     about to become a third, independently-drifting implementation of
     the same formula. Recorded here because it is the same
     "define-the-shared-thing-once" discipline as continuation 60's
     singular/plural query invariant, applied to a different kind of
     duplication (a formula, not a query shape).
  Two latent decompose-walk correctness bugs, invisible under the prior
  geometry, were found and fixed in passing: `'`/`"` did not perform
  their `T*` line move and `"` did not set `Tw`/`Tc` (§9.4.3 Table 109);
  `Tc`/`Tw`/`Tz`/`Ts` were not tracked in the decomposer's `GState` at
  all (now carried with Table 105 initial values, saved/restored across
  `q`/`Q`). Zero new Cargo dependencies. Gates: `cargo test --workspace`
  1599 → 1613, 0 failed; fmt/clippy clean; `cargo tree -p pdfce-core`/
  `-p pdfce-render` GUI-dep-free. Full record: `ROADMAP.md`'s Pass 18.6
  Shipped entry (top of Shipped); `ARCHITECTURE.md` §4's own
  "IMPLEMENTED (2026-08-03, Pass 18.6...)" paragraph (above) for the
  body-section update this decision requires.
- **2026-08-03 (same-day continuation 60) — Documentation-process
  finding, recorded as methodology rather than a product/architecture
  decision: doc-writing agents have no shell, so hashes/counts handed
  to them are filed as fact with no independent verification.** This is
  the SECOND filing error this project's own "verify against `git`"
  habit has caught (the first: commit `7274fdd` went missing from its
  own chain listing at continuation 59; the second: that same listing's
  hash set and commit-vs-branch count conflation, corrected this
  continuation, `25b4783`). New standing rule `ROADMAP.md` R87 records
  the resulting discipline: hashes and commit/test counts are always
  produced by the engineer directly from `git`/`cargo test`, never
  recalled from memory or a prior summary, and spot-checked after
  filing. No `pdfce-core`/`pdfce-render`/`pdfce-gui` architecture is
  affected by this entry — recorded here per this file's practice of
  logging any decision-shaped finding the project produces, not only
  ones that touch shipped code.
- **2026-08-03 — Decision 019: FF-H re-scoped to direct text-state
  formatting (`Tc`/`Tz` + free-form `Ts` + synthetic bold/italic), `Tw`
  evidence-gated, StructTree/`/ActualText` CUT and re-filed as FF-I.
  Amends decision 014 §5.3 (FF-H's original bundle) and decision 016 §2
  (FF-H's "defer" verdict, superseded by the operator's priority-#3
  directive; §2's underlying reasoning about StructTree is upheld and
  acted on, not overturned). Full record:
  `docs/decisions/019-ffh-spacing-scaling-synthetic-styles.md`.**
  The premise changed before any code was written: Acrobat-parity
  research (`D:\Dev\Rag-Specialized\Acrobat_Features\text_edit__spacing_and_scaling_controls.md`,
  sourced to Dov Isaacs, former Adobe Principal Scientist) establishes
  Acrobat itself dropped `Tw` and free-form baseline offset when text
  editing consolidated into the single Edit Text & Images tool — FF-H's
  own name lists four operators, parity covers two. Three architectural
  facts, all binding on future `pdfce-core` text-state work (see §5.11's
  new forward-pointer paragraph, above, for the body-section update this
  entry requires):
  1. **`q`/`Q` are illegal inside `BT…ET`** (§8.2 Table 51/Figure 9), so
     ambient text-state restoration is always restore-by-value via the
     `TextColor::restore_bytes` three-tier ladder (spec default →
     observed raw bytes → refuse-and-disclose), never `q`/`Q` scoping —
     new standing rule R88.
  2. **`Tc`/`Ts` are unscaled text-space quantities (§9.3), not scaled
     by `Tfs`** — stored as `Absolute | Relative` so a font-size change
     cannot silently mis-scale a stored rise/tracking value — R89.
  3. **Ambient `Tc`/`Tw`/`Tz`/`Ts` state was tracked three times,
     privately, with zero shared publication** (`text_extract::page::TextState`,
     `text_edit::edit::Walk`/`reflow_apply::BlockTextState`,
     `vector::decompose::GState`) — Pass 19.0 (IN PROGRESS) consolidates
     these and publishes `Ts`/`Tr` through `GlyphProvenance` for the
     first time (today dropped at provenance-construction time). Pass
     18.6's own `GState` tracker (a reading-path hit-test aid) is
     explicitly NOT groundwork for this consolidation — it is the THIRD
     private tracker, which is what makes the consolidation case
     unarguable, not a head start on building it.
  **Decision, not merely a fact:** `Tw` does NOT become a direct
  authoring control — its inter-word-distribution job stays with
  decision 015's `TJ`-based reflow design, and any future promotion is
  gated behind a corpus census with explicit decision bands (R91: ≥60%
  build, ≤25% close, 25–60% escalate to the operator). Free-form `Ts`
  DOES ship, as a deliberate exceed rather than a parity feature — the
  emission/restore/tracking/test mechanism is forced anyway by
  superscript/subscript (a genuine parity requirement), so withholding
  the raw number would mean building the mechanism and hiding it, and
  it works identically on every font model with no void case (R89
  covers its ratio-storage requirement). Synthetic bold/italic (`Tr 2`
  stroke+fill with a user-space-derived, fill-matched stroke, plus a
  `Tm` shear for oblique, never double-strike) is one shared policy for
  both the in-place-edit (14.x) and add-text (16.x) paths — R90.
  StructTree/`/ActualText` is CUT from FF-H entirely (a partial
  structure-tree writer judged worse than none) and re-filed as its own
  ungated Backlog item, FF-I, with no Pass number assigned. Build order
  is FF-H → FF-C → FF-B, decided on Pass 19.0 being a shared correctness
  prerequisite the other two inherit, not on FF-H's own value (judged
  lowest of the three). Zero new Cargo dependencies; no license/rule-13
  question arises for this decision itself (a future FF-C dependency
  pick still needs its own rule-13 check — flagged to the operator, see
  `ROADMAP.md` Open operator questions item (h)). New standing rules
  R88–R91 (ceiling was R87); Pass family: `ROADMAP.md`'s ★ Pass 19.x
  (19.0 consolidation IN PROGRESS → 19.1 `Tc`/`Tz`/super-subscript → 19.2
  `Ts`/synthesis → 19.3 GUI → 19.4 `Tw` conditional).
- **2026-08-03 (same-day, Amendment A to decision 019) — Pass 19.0
  SHIPPED; three deviations from decision 019/R88-R89 as originally
  written, all now binding, plus a live defect in shipped Pass-14.2
  code found by this slice. Full record:
  `docs/decisions/019-ffh-spacing-scaling-synthetic-styles.md`
  Amendment A.**
  1. **R88's three-tier restore ladder needed a FOURTH rung — its
     wording is corrected below.** "Observed raw operand bytes when
     set" assumed a setter's bytes are either available or absent;
     there is a third case, available and POISONOUS. `TD` sets `TL` as
     a documented side effect of moving the line, and `"` sets
     `Tw`/`Tc` while SHOWING A STRING (§9.4.3 Tables 108/109) — replaying
     either's raw bytes as a spacing-only restore re-executes the side
     effect. Resolved by a new `AmbientOrigin::ObservedIndirect { setter
     }` rung: the value is known but its source operator did more than
     set it, so the restore RE-SPELLS the value in its own dedicated
     operator rather than replaying captured bytes, and
     `is_byte_faithful()` reports `false` for disclosure. **R88's
     wording is corrected to:** restore from raw bytes where they are a
     faithful and side-effect-free record → re-spell where the value is
     known but its source operator did more than set it → refuse where
     unobservable. (Filed as its own generalizable finding:
     `C:\personal_rag\pdf\lesson_20260803_quote_operator_side_effect_poisons_raw_byte_restore.md`.)
  2. **§3.4's tier-3 case (i) (multi-stream `/Contents`) is
     architecturally UNREACHABLE today, not merely rare.**
     `ContentStream::from_page` concatenates the entire `/Contents`
     array before any operator walk begins, and a decode failure on any
     element fails the whole page rather than yielding a partial
     prefix — so "unobservable because the byte stream ends mid-array"
     cannot currently occur. Recorded with the condition that would
     make it reachable again (lazy/per-element concatenation), rather
     than manufacturing an untestable trigger.
  3. **`Tf`/`Tfs` are NOT unified into the shared `TextStateParam`
     model — R89's "exactly one definition" claim is narrowed to the
     six single-operand parameters R88 covers.** The extraction walk
     narrows `Tfs` to `f32` to publish `GlyphProvenance::tf_size`, then
     re-widens it for the §9.4.4 advance computation; unifying to `f64`
     throughout would perturb already-published glyph positions (bit-
     for-bit, not just semantically) — the same narrow-then-divide vs.
     divide-then-narrow trap applies to `Tz`. "Exactly one definition"
     is therefore true of `pdfce-core` only, by design:
     `pdfce-render::text::TextState` remains a deliberate FOURTH
     tracker, kept independent on purpose because render-parity wants
     an implementation that cannot share a bug with the authoring-side
     model.
  **Live defect found and fixed by this slice, in already-shipped Pass
  14.2 code, not a decision-019 design question:** `text_edit::edit::
  Walk` had **no `q` and no `Q` arm at all** (engineer-verified 0 → 1
  occurrences before/after). Text state AND fill colour leaked past a
  `Q` in the in-place-edit model — shipped Pass 14.2 behavior could
  re-emit a fill colour a `Q` had already discarded. Decision 019 §1.2's
  own audit of missing arms reported the missing `Ts`/`Tr` cases and
  missed this one; both facts are recorded together deliberately — the
  audit was otherwise the strongest part of the decision, and "the
  audit was thorough" is exactly the belief that let this gap through.
  Fixed with two new regression tests in the same Pass. No
  `pdfce-core`/`pdfce-render` GUI-dependency change; `cargo tree`
  re-verified clean.
- **2026-08-03 (same-day, Amendment B to decision 019) — Pass 19.1
  SHIPPED (`603b051`); mechanism correction (`Tz`×justify), a spec-
  citation flag verified closed, and R89's base-size ambiguity
  resolved. Full record:
  `docs/decisions/019-ffh-spacing-scaling-synthetic-styles.md`
  Amendment B.**
  1. **The `Tz`×justify disclosure named the wrong mechanism.** §19.1's
     scope note and §3.1's options table both say "`Th` rescales every
     `TJ` numeric adjustment (§9.3.4), so a `Tz` change invalidates a
     justified line's slack." `Th` genuinely does rescale `TJ`
     adjustments per §9.3.4 — but the `TJ` numbers carrying a
     15.1-justified line's slack sit in `format.rs`'s `pre`/`post`
     splice segments, OUTSIDE the `set_ops`/`restore_ops` wrap that
     scopes a `Tz` edit — they run at ambient (unchanged) `Th` and are
     NOT rescaled. **The conclusion survives (a `Tz` edit does
     invalidate a justified line and needs a re-justify offer); the
     cause does not** — it is the formatted run's changed rendered
     WIDTH (`ΔA`, §9.4.4's `tx` formula) making the pre-computed slack
     wrong for the run's new width, not any `TJ` value itself changing.
     `ROADMAP.md`'s ★ Pass 19.x §19.1 slice bullet corrected to match.
     Filed as a general finding:
     `C:\personal_rag\pdf\lesson_20260803_tz_th_rescales_tj_adjustments_not_slack_outside_wrap.md`.
  2. **The flagged `Ts`=§9.3.6 spec-citation error was verified NOT to
     exist in the decision document** — only in `text_state.rs` (three
     comment citations, already fixed by the engineer in this slice).
     The document's own §1.3 item 6 carries no clause citation, its
     "(§1.3.6)" cross-reference at §3.2 is internal numbering (not an
     ISO clause), and §12 References already correctly says "§9.3.7
     rise." No document edit needed; recorded so the flag closes with
     an explanation.
  3. **R89's "`Tfs`" is now stated explicitly as the BASE size** — the
     size in effect for the run at the point of formatting (i.e. the
     size the operator is setting the run TO, if size and
     superscript/subscript are edited in the same request), not any
     other candidate value. Previously ambiguous in the decision text;
     the implementation had already chosen base and now the record
     says so.
  Also: the engineer's fourth flagged item (R88's four-rung wording in
  `ROADMAP.md`'s Standing Rules) was checked and found **already
  correct** — no edit needed, recorded in Amendment B so the item
  closes. **New standing rule R92** (methodology, no decision number):
  a predicate that hand-duplicates the shape of a data structure it
  inspects (an exhaustive field-by-field no-op check, a hand-listed
  operator-arm list) drifts silently the moment the structure gains a
  field or case. Second occurrence of this exact bug shape — the first
  was Amendment A.4's missing `q`/`Q` arms in `text_edit::edit::Walk`;
  this time it was `EditSession::format_text`'s own hand-listed no-op
  predicate (`set_size.is_none() && set_fill.is_none() &&
  set_font.is_none()`), which Pass 19.1's new `FormatRequest` fields
  bypassed entirely, making a spacing-only request a phantom `NoOp` on
  the GUI-facing `EditSession` path specifically (the CLI's
  `set_format` path, which used the real `FormatRequest`, was
  unaffected). Fixed by replacing the hand-list with `req.is_empty()`.
  General rule: derive such predicates from the structure itself, never
  hand-maintain a mirror of it.
- **2026-08-03 (same-day, Amendment C to decision 019) — Pass 19.2
  SHIPPED (`ebe35d8`); free-form `Ts` + synthetic bold/italic. Six
  corrections found while building this slice. Full record:
  `docs/decisions/019-ffh-spacing-scaling-synthetic-styles.md`
  Amendment C.**
  1. **§3.6 named the wrong restore set.** Stroking colour and the
     derived stroke line width are ordinary graphics state **shared
     with path painting**, not text state scoped to `Tc`/`Tw`/`Tz`/
     `Ts`/`Tr` — §3.6 described both only as things to *set* correctly
     and never as things to *restore*, so a synthetic-bold run would
     leak its stroke settings into every later stroked *path* on the
     page. Two restore obligations added; R88's ladder is amended to
     cover them alongside the six text-state parameters.
  2. **§3.6's "re-emit followers with an absolute `Tm`" is narrower in
     practice.** The builder deliberately did **not** convert a
     producer's own `Td`/`T*` into an absolute `Tm` — doing so rewrites
     the producer's own line structure (exceeding R32/R46 minimal-diff)
     and cascades to every subsequent relative move. pdfce instead
     **requires** the follower already be absolute and **refuses,
     disclosed, otherwise** — a twin test proves the refusal is not
     unconditional (the same run succeeds when the next line opens its
     own `BT…ET`).
  3. **The bold-width formula ships two of its three factors,
     disclosed rather than dropped.** §3.6 specifies `Tfs × |Tm scale|
     × |CTM scale|`; the authoring walk models the first two and has
     no `cm` model, so a page-level CTM scale is not compensated. Named
     as a LIMIT in the builder's own report text, per rule 4/R73, not
     found later as a silent gap.
  4. **Neither the decision nor Amendment A anticipated that synthetic
     italic needs text-matrix tracking in the authoring walk at all.**
     Amendment A.3 scoped the shared hoist to the six text-state
     parameters and said nothing about `Tm`/`Tlm` — but item 2's
     refusal gate cannot be evaluated without knowing whether a
     follower is already absolute. Pass 19.2 built `Tm`/`Tlm` tracking
     into `text_edit::edit::Walk` (`BT` reset, `Td`/`TD`/`T*`
     derivation, §9.4.4 advance accumulation, a `matrix_known` honesty
     flag) plus a new `Rec::EndText` variant.
  5. **Two unnamed conflicts, both refused by name rather than silently
     merged:** free-form rise (19.2) vs. the superscript/subscript
     toggle (19.1) — both write `Ts`; and synthetic italic vs. a
     `--pin` follower-positioning mode — the closing absolute `Tm` and
     `--pin`'s compensating `TJ` adjustment would each consume the same
     positional delta, double-consuming it if composed.
  6. **Add-Text synthesis is flagged as NOT wired, not implied
     shipped.** The shared `StyleSynthesis` type, gate, and wording
     exist and `SynthesisPath::AddText` is implemented and tested, but
     `addtext.rs` has no bold/italic request surface, so the offer is
     currently unreachable from that path — matching the decision's own
     prediction that the gate "will rarely even open here" (R79's
     Standard-14 default has real Bolds) but distinct from "cannot be
     reached." Wiring it needs new request/CLI surface, not scoped here.
  **Verification-method finding, itself worth recording:** the render-
  honours-mode-2/sheared-`Tm` prerequisite (named in the original
  decision) was confirmed **by mutation testing, not by inspection** —
  a new `pdfce-render` test suite rasterizes fixtures, passes, then
  deliberately breaks the renderer three ways (drop mode-2 stroking,
  zero the `Tm` shear component, zero the rise) and re-runs to confirm
  each mutation fails exactly the tests it should. This is the standard
  a by-inspection prerequisite check should itself have met; escalated
  as a general methodology finding (see the RAG-escalation note in
  `SESSION_LOG.md`). No `pdfce-core`/`pdfce-render` GUI-dependency
  change; `cargo tree` re-verified clean; zero new Cargo dependencies.
- **2026-08-03 (same-day, Amendment D to decision 019) — Pass 19.3
  SHIPPED (`74052d3`); the GUI property surface, AND a project-wide
  data-contract defect that had silently disabled every property-bar
  Apply since Pass 14.3. Full record: the Pass 19.3 Shipped entry,
  `ROADMAP.md` (top of Shipped), and §5.11's new Pass 19.3 paragraph,
  above.** Not a decision-019 design question — a live defect the
  slice exposed, in the same spirit as Amendment A's `q`/`Q`-arm
  finding: `GlyphProvenance::operator_span` (extraction walk) and
  `text_edit::edit::OpRec` (authoring walk) publish two DIFFERENT
  conventions for "the span of this operation" — operator-token-only
  vs. operand-inclusive — and `find_anchor`'s pinned-request matcher
  required exact equality between them. Since the GUI always pins from
  published provenance and the authoring walk always records the wider
  span, the two spans never matched: every GUI-issued formatting/edit
  request since Pass 14.3 refused with `NoMatch` before reaching the
  surgery, invisible in the running application because the failure was
  discarded with `.ok()` rather than surfaced. Fixed by relaxing
  `pin_names_operator` to accept either convention
  (`pin.end() == r.end && pin.start >= r.start` — two operations sharing
  one content stream cannot share an end offset), verified by mutation
  (revert the relaxation → a new regression test fails; restore it →
  passes) and by a second regression test proving the relaxed matcher
  still discriminates a near-miss span rather than degrading into
  false-positive misattribution. **Two doc comments, on both the
  publisher and the consumer side of the contract, had independently
  asserted the conventions already agreed** — `EditRequest::
  pinned_span`'s "matches the same span" and `text_edit/page.rs`'s "the
  surgery locates the operator by exactly this span" — both corrected
  in place. **New standing rule R93** (`ROADMAP.md`, ceiling was R92):
  a code comment asserting a cross-module contract holds is a claim,
  not evidence, even when two independent comments on both ends of the
  contract agree with each other — third occurrence of this exact
  failure shape in this project, after decision 018's `refresh_pages`
  "the base revision has not changed" comment (true through Pass 3.1,
  silently false from Pass 6.1) and the `.gitattributes` ordering
  incident (the file's own `*.pdf binary` rule silently overridden by a
  catch-all placed below it). No `pdfce-core`/`pdfce-render`
  GUI-dependency change; `cargo tree` re-verified clean; zero new Cargo
  dependencies; `cargo test --workspace` 1708 → 1722, 0 failed.
  **RAG escalation filed to `D:\dev\rag\rust\`, not `personal_rag/pdf`**
  (a deliberate librarian judgment call, deviating from the suggested
  location — the lesson generalizes to any editor publishing byte spans
  for later re-location, not to PDF-domain producer-divergence
  behavior): `byte_span_convention_must_live_in_the_type_not_matching_doc_comments.md`
  and `trust_but_verify_doc_comments_are_not_evidence.md`.
- **2026-08-03 (same-day, Amendment E to decision 019) — the §3.3 `Tw`
  census has been RUN; slice 19.4's BUILD/close/escalate gate is
  resolved (BUILD), and one of the decision's own supporting arguments
  is shown wrong by the result.** New out-of-workspace crate
  `tools/tw-census` (zero new Cargo dependencies, root `exclude`-list
  convention; commits `359d486`/`5387699`, verified `git cat-file -t`)
  measured reachability keyed by show operator
  (`GlyphProvenance`'s `(ContentStreamRef, ByteSpan)` — §3.3's own named
  unit) over the Pass-11 corpus: 1,224 text-bearing documents / 23,144
  show operators / 620,858 glyphs, after excluding 627 unloadable and
  2,172 zero-show-operator files. **Result: 91.6% of show operators /
  97.4% of glyphs are on a simple font — the ≥60% BUILD band, not
  marginal** (weakest denominator 86.7% by-document; survives removing
  the four most-glyph-heavy files at 87.3%). **§3.2 reason 2 falsified
  on this corpus:** 81.2% of text-bearing documents contain no
  composite run at all, contradicting the "large and growing share"
  claim that partly justified withholding `Tw`; the "growing" half is
  separately recorded as untestable on this corpus (PDF-tooling test
  suites, not a sample of recently-produced documents — vintage as old
  as Isartor 2008). A strict variant (simple AND contains code 32)
  lands in the 25–60% escalate band but is flagged fragile (12-point
  swing on removing four files) and asymmetric (composite runs cannot
  carry code 32 in an Identity-H subset) — reported as context, not
  acted on; the decision's bands are written against, and satisfied by,
  the loose metric. **Slice 19.4 is cleared to build but has NOT
  started**: this same corpus sweep found an independent pdfce defect
  (341 corpus files, 8.5%, refuse to open at all on a `/Contents`
  array element that resolves to Null — a fail-clean violation, since
  §7.3.10/Table 30 make a dangling `/Contents` element degradable, not
  document-fatal) that the engineer prioritized fixing first — see
  §5.11's new paragraph and `ROADMAP.md`'s "★ pdfce defect" In-progress
  entry. Full record: `docs/decisions/019-ffh-spacing-scaling-
  synthetic-styles.md` Amendment E. **RAG escalations:**
  `C:\personal_rag\pdf\lesson_20260803_tw_reachability_census_show_operator_91pct.md`
  (the reachability finding, vintage/corpus-bias caveats prominent) and
  `D:\dev\rag\rust\state_every_denominator_a_census_could_report.md`
  (methodology: this census's three denominators differ by 11 points —
  document/operator/glyph — a single headline figure would have been
  actionable-looking and wrong).
- **2026-08-03 (same-day, decision-013 addendum, no new decision
  number) — the `/Contents`-defect fix: `StreamLengthPolicy` +
  `Provenance::RecoveredFile`, closing the pdfce defect Amendment E
  found.** Extends decision 013 (xref recovery, §5.10/R67) with a
  read-side sibling and a write-side correctness fix, both landed the
  same continuation, committed `409a6b5`. **(a) Corrected mechanism:**
  the defect Amendment E reported (rebuild-by-scan undercounting
  objects) was wrong — the scan is correct; the failure was at
  strict-confirmation, root-caused to `add-contents.pdf` being an
  LF-to-CRLF-converted file that invalidated every `/Length` in the
  same damage event that broke `startxref`. **(b) `StreamLengthPolicy`**
  (`Strict` default unchanged; `RecoverFromEndstream`, reachable only
  from recovery paths, re-derives a stream's extent from the
  `endstream` keyword per §7.3.8.2's own definition of `/Length`).
  **(c) Per-element `/Contents` degradation**, not whole-document
  refusal, for a dangling array reference (§7.3.10 + Table 30) —
  counted via `Page.contents_unresolved`/`RecoveryReport.
  stream_lengths_recovered`, disclosed in CLI + GUI, never silent.
  **(d) `Provenance::RecoveredFile`** — a third variant on the
  `#[non_exhaustive]` `Provenance` enum for "bytes exist but no longer
  agree with the value," forcing re-serialization instead of the
  verbatim-copy fast path `Provenance::File` objects otherwise take.
  Added because the round-trip gate (§5.10) caught the first repair
  attempt producing a file pdfce itself could not reload — it had
  corrected the byte span but left the stale `/Length` for the writer
  to copy verbatim beside it. §5.10's contract is not weakened: the
  mutation is deliberate and disclosed through the existing
  `RecoveryReport` channel, and both pre-existing verbatim-passthrough
  sites already excluded non-`File` provenance via `let-else`, so both
  were correct by construction against the new variant without
  modification. **New standing rules R94–R95** (`ROADMAP.md`): R94
  generalizes (d) — a repair that mutates a value must invalidate any
  verbatim-bytes provenance attached to it; R95 states (c) as binding,
  a read-side sibling of R67's forced-full-rewrite family. **Result:**
  289 of 341 previously-unopenable corpus files now open with real
  content (independently re-measured: `BadContents` 341 → 1, zero
  regressions, raster oracle 174 → 178 compared/all identical). Full
  numbers and gates: `ROADMAP.md`'s `/Contents`-defect-fix Shipped
  entry (top of Shipped). **RAG escalations:**
  `C:\personal_rag\pdf\lesson_20260803_crlf_conversion_invalidates_every_length.md`
  and
  `D:\dev\rag\rust\repair_that_mutates_a_value_must_invalidate_verbatim_provenance.md`.
- **2026-08-03 (same-day, Amendment F to decision 019) — Pass 19.4
  (`Tw` direct-authoring control) SHIPPED, `a1638f4`; decision 019 /
  FF-H is COMPLETE end-to-end (all five slices 19.0–19.4 shipped).**
  Rides the existing `push_state_param` restore ladder and
  `pre|set_ops|mid|restore_ops|post` splice, no new authoring path;
  `MetricSpec::{Absolute,Relative}` shared with `Tc` (Pass 19.1),
  resolved against BASE size per Amendment B item B.3; CLI
  `--word-spacing V[pt|em]` via a generalized `parse_text_metric`
  (was `parse_char_spacing`). **Three findings, none anticipated by the
  original decision or Amendments A–E:** (1) **R91 (the composite
  structural-void refusal) was UNREACHABLE as originally implemented**
  — `match_run` decodes a candidate run's text and filters every
  composite run to `NoMatch` (composite `ShowData::text` is always
  empty) BEFORE the font-aware refusal gate runs, so R91 would have
  shipped as code that is referenced in three documents and never once
  executes. Fixed by hoisting font resolution above `match_run`;
  verified by a test proving the gate now fires plus a second test
  (`the_composite_gate_fires_only_for_word_spacing`) proving the OTHER
  three controls (`Tc`/`Tz`/superscript-subscript) stay live on the
  same composite run — a specific capability gate, not an accidental
  blanket composite refusal. **(2) A named limit:** the fixed refusal
  is reachable through the pinned-span path (GUI, core tests) but NOT
  through CLI `--find` — composite-run text search finds nothing, so
  `--find` reports "not found in an editable run," a less specific
  message than the decision describes; closing this needs composite
  decoding in the authoring walk, FF-E's scope. **(3) `Tw` is
  multiplied by `Th`** (§9.4.4, same basis as `Tc`) — the decision
  names this only as a reason `Tw` is awkward to expose as a control,
  never as something needing disclosure; the word-spacing disclosure
  now quotes the effective delivered value whenever `Th ≠ 1`. Also
  recorded (not a defect): `Some(0)` affected spaces is emitted and
  disclosed as a genuine answer rather than suppressed as a no-op; and
  Amendment A.1's fourth restore rung needed no code change to
  correctly handle `"` setting `Tw`/`Tc` as a side effect of showing
  text — its first concrete, load-bearing test, worth recording as a
  design choice that held rather than only recording corrections.
  `cargo test --workspace` 1738 → 1756, 0 failed; fmt/clippy clean;
  `cargo tree` clean; zero new Cargo dependencies; R85 21/21;
  round-trip proven non-vacuous by two binaries differing in both MD5
  and size (3,396,096 vs 3,394,048 bytes). Full record: `ROADMAP.md`'s
  Pass 19.4 Shipped entry (top of Shipped); `ARCHITECTURE.md` §5.11.
  **MILESTONE:** closes item #3 ("finish off all the text handling
  stuff") of the operator's four-item priority sequence as far as
  FF-H's own scope goes — FF-C and FF-B remain unscheduled, per this
  decision's own Q3 build order (FF-H → FF-C → FF-B), unaffected by
  this milestone beyond clearing FF-H's own slot. **RAG escalations:**
  `D:\dev\rag\rust\dead_guard_clause_behind_a_filter_the_guarded_case_cannot_pass.md`
  (finding 1, the unreachable-gate methodology) and
  `C:\personal_rag\pdf\lesson_20260803_word_spacing_multiplied_by_horizontal_scaling.md`
  (finding 3, the `Tw`×`Th` coupling).
- **2026-08-03 (same-day continuation 71) — Decision 020 filed: form
  field AUTHORING (field-identity model, XFA scope, slice order, tab
  order, tagging, four research conflicts). Status: DECIDED, SCOPED,
  NOT STARTED — Pass 20.x is unbuilt.** Archived at
  `docs/decisions/020-form-field-authoring.md`. Requested by
  `pdfce-engineer` scoping operator priority item #4 ("form building
  tools"); depends on decision 009 (JS posture) and decision 019 §3.7
  (the FF-I cut). **No body-section update this entry** — nothing has
  shipped, so `ARCHITECTURE.md` §4/§5/§12.7 describe no new reality yet;
  §4 (core data model) and this file's forms-model description get
  updated when Pass 20.x's F0/F1 actually land, not now. The one
  exception is the forward pointer added to the decision-009 entry
  above (§1.2.6/§7.2), because decision 020 changes what that
  **already-shipped** guarantee will mean once authoring exists — that
  is a change to a currently-true statement, not a future one.
  - **The single sharpest finding: decision 009's byte-verbatim
    JS-carrier guarantee (fill never touches `/AcroForm`) does not
    survive field creation, and no existing test will notice.** Field
    creation must write `/AcroForm/Fields`. Full reasoning at the
    decision-009 entry's forward pointer, above; F1 owes the byte-grep
    test that makes the guarantee test-enforced again rather than
    silently partial.
  - **Data model (Q1): the shipped flat `AcroForm.fields: Vec<Field>`
    read projection is CORRECT and STAYS — it is not a graph-vs-flat
    question at all.** `Field.widgets: Vec<Widget>` already carries the
    one-to-many that matters, and every write path already fans out
    over it. What is missing is everything *above* the terminal field.
    The fix is a **write-side-only** resolver
    (`resolve_field_path(graph, fqn) -> FieldPath`) that walks the raw
    `/Kids` object graph — never a rewrite of the fuzz-tested,
    corpus-proven read path. `Field` gains one new additive `pub` field,
    `parent: Option<ObjId>`. New standing rule **R100**: field identity
    is the FQN, derived from the graph not stored, so every authoring
    write resolves against the graph before it writes.
  - **Collision branch is FOUR-WAY, not two.** Vacant → create; Terminal
    + type-match → merge (Shape A→B promotion, R101/R102); Terminal +
    type-mismatch → refuse (`FieldTypeCollision`); and a fourth branch
    the research and Acrobat both lack because neither authors
    hierarchy — **Grouping** (a non-terminal node, which Table 220 says
    has no type of its own) → refuse (`NameIsGroupingNode`). New
    standing rule **R101**: a widget kid pdfce authors carries no `/T`,
    `/FT`, or `/Kids` — pdfce's own `kid_is_field` heuristic promotes
    any such kid to a separate terminal field, silently destroying group
    semantics. New standing rule **R102**: pdfce never normalizes field
    shape — Shape A→B promotion happens only when Table 220 makes the
    merged form illegal, and Shape B never collapses back to A
    (`ARCHITECTURE.md` §5.6 "never normalize," applied to a shape that
    section did not originally anticipate).
  - **XFA: dynamic stays `out_of_scope` (unchanged). Static-XFA-hybrid
    field creation is REFUSED BY NAME, decided here rather than left as
    an Acrobat-parity GAP** — pdfce can write the AcroForm half of a
    hybrid but not the XFA half, so a one-sided add makes an XFA-aware
    viewer and a non-XFA viewer show two different field counts for the
    same document. Decided from pdfce's own capability boundary, not
    from an unresolvable question about what Acrobat does.
  - **Slice order: F0 (correctness-only substrate, no operator surface,
    rule-11 exempt on the Pass 19.0 precedent) → F1 (text-field creation
    THROUGH the resolver, all four collision outcomes live and
    tested — the P0 floor, not a lone text field) → F2 (checkbox/radio +
    deletion) → F3 (choice + push button) → F4 (tab order, BLOCKED on a
    `pdfce-spec-librarian` dispatch — Table 30's `/Tabs` row and the ISO
    32000-2 delta are verified absent from the spec RAG) → F5 (GUI,
    `pdfce-ui-specialist` dispatch required first). F6/F7 fast-follows,
    non-gating.** Signature-field creation deferred to Pass 10
    (Signatures); barcode fields cut outright (JS-driven population,
    permanently forbidden by decision 009).
  - **Tab order: new standing rule R104 — `/Tabs` is a MODE, not a
    snapshot.** Under `/R`/`/C`/`/S` pdfce reorders nothing on field
    insertion (there is no stored sequence to maintain); under an
    explicit order the new field is appended to the end and that fact
    is disclosed; `/Tabs` is never written as a side effect of creation.
    Corrects the parity research's "always correctly re-sorted"
    recommendation, which read literally would re-sort `/Annots` on
    every insertion — a minimal-diff violation (R32/R46) that also
    changes annotation paint order. Missed case the research did not
    name: `/Tabs /S` + an untagged new field has **no defined tab
    position at all**, not "last," because structure order derives from
    the tag tree and pdfce-authored fields are untagged; F1 must detect
    and disclose this, not defer it to F4.
  - **Tagging: split, not a re-opening of FF-I.** New standing rule
    **R105** — every field pdfce authors carries `/TU`
    (mandatory-or-explicitly-declined; omitting both `--tooltip` and
    `--no-tooltip` is an error, never a silent default), because for
    form fields specifically `/TU` — not the structure tree — is what
    assistive technology actually reads (WebAIM, sourced;
    screen readers read fields through the interactive-field layer,
    bypassing the tag tree). Writing `/StructElem`/`/ParentTree` stays
    cut with FF-I, unchanged rationale — the test applied: does the
    proposal build a partial structure-tree writer? `/TU` does not;
    `/StructElem` does.
    **★ IMPLEMENTED 2026-08-07, `50a5461`.** `TooltipChoice { Undecided,
    Text(String), Declined }` replaces the bare `Option<String>` `/TU`
    parameter on `NewTextField`/`NewCheckBox`/`NewChoiceField`; `Undecided`
    is refused with `EditError::TooltipDecisionRequired`; CLI is
    `--tooltip <text>` XOR `--no-tooltip` (clap `conflicts_with`).
    **Declining writes NO `/TU` key, deliberately not an empty one** — an
    empty `/TU` would make a screen reader announce an empty accessibility
    name instead of falling back to `/T`, so writing `()` on decline would
    be worse than writing nothing. The §3.4.3/§3.5.3 disclosures (this
    bullet's own R104 paragraph above, and R105 here) also ship in the same
    commit, unified across all three authoring verbs as
    `FieldAuthorOutcome { field_id, merged, disclosures:
    FieldAuthorDisclosures { tooltip_declined, tagged_document,
    structure_tab_order, has_no_options } }`, retiring the choice-only
    `ChoiceAuthorOutcome` Pass 20.2/20.3 had shipped. New fixture
    `tagged-struct-tabs.pdf` makes both disclosures reachable at once (R96).
    **Not implemented by this commit: `/Tabs` tab-order AUTHORING (F4) —
    only the disclosure that a field has no defined tab position.** Full
    build record: `ROADMAP.md`'s `Pass 20.0` *Shipped* entry, THIRD
    addendum, same date.
    **★ AMENDED 2026-08-07 (`69ab966`, then `8a8678e`) — THE STRUCT HAS
    FIVE FIELDS, NOT FOUR, AND ITS PREDICATE IS NOW A DESTRUCTURING.**
    The radio slice added **`group_flags_ignored`** (a joining member's
    `NoToggleToOff`/`RadiosInUnison` are disclosed, not applied). The
    four-field shape written above is the shape at `50a5461` and is left
    as filed. **`FieldAuthorDisclosures::any()` had no arm for the new
    field**, so the natural gate for a GUI disclosure block answered
    `false` for a radio merge whose **only** disclosure was that pdfce had
    overridden the operator's flags — **rule 4 failing closed**, and the
    **second** instance of an omission whose first instance
    (`report_field_disclosures`) had already been fixed. **`any()` is now
    a DESTRUCTURING of the struct: adding a field without handling it here
    is a compile error.** **Anyone adding a sixth field must expect the
    compiler to stop them, and must not "fix" that by reintroducing a
    `||` chain or a wildcard arm** — the exhaustiveness is the point.
    Full build record: `ROADMAP.md`'s `Pass 20.5 (PARTIAL)` *Shipped*
    entry.
  - **Dead-guard debt, the prospective form of R96 — new standing rule
    R103.** The parity research's `must_have` (a `/P` bit-6
    permission gate on field creation) would be dead code today: every
    `EditSession` authoring path already refuses `/Encrypt` documents
    outright, unconditionally, before any forms code runs. Decision: do
    not build the bit-6 gate now; record it as owed to Pass 5
    (Encryption), where it becomes reachable and provable. What ships
    instead is a DocMDP/FieldMDP certification gate **stricter than
    fill's** — creation refuses at any `/DocMDP` tier and on any
    `/FieldMDP`, vs. fill's `/P >= 2` permit.
  - **Standing rules R100–R105 filed (six rules; RENUMBERED from the
    decision document's own original R97–R102 — see below).** Full
    text: `ROADMAP.md` Standing rules.
  - **Renumbering note, filed here for the audit trail.** The decision
    document, drafted concurrently with continuation 70's Pass 8.1
    filing, proposed its six rules as R97–R102 against a believed
    ceiling of R96. Continuation 70 had already claimed R97–R99 for
    three unrelated Pass 8.1 findings by the time both filings landed.
    `pdfce-librarian` renumbered all six of decision 020's rules to
    R100–R105 in both the decision document (prose and Appendix A JSON,
    in place, with a machine-readable mapping added) and `ROADMAP.md`,
    rather than leaving a collision on disk. No rule's *substance*
    changed, only its number.
  - **Four unresolved research items closed, none escalated:**
    Combine-Files auto-rename-vs-link is one behavior with a documented
    fallback, not a contradiction (fast-follow F7: make it an operator
    choice, `--on-field-collision rename|link|refuse`); the encrypted-
    document conflict is structurally inapplicable to pdfce's
    architecture (Layer 1) and R96-verified dead code today if built as
    the research proposed (Layer 2, see R103 above); the two radio-
    deletion GAPs are reframed out of existence — decided as pdfce's own
    documented, test-provable rule rather than deferred to an empirical
    check against a real Acrobat install.
  - **Five open items filed for the operator, not decided solo** (full
    text: `ROADMAP.md`'s Open operator questions and the Forms/AcroForm
    Backlog entry): (1) whether item #4 should start at all, given item
    #3 (FF-C/FF-B) is not yet closed; (2) whether "add a signature field
    for someone else to sign" (no signing subsystem required) should be
    pulled into F3; (3) confirm the barcode-field parity subtraction is
    acceptable; (4) retire or re-scope the standing XFA-deprecation
    open item; (5) — resolved, not open — §10.2's competing claim about
    an unfinished GUI redaction-apply flow was written against a stale
    tree and is corrected in place in the decision document itself
    (Pass 8.1 shipped `9a68999` before this decision was filed).

- **2026-08-03 (same-day) — Decision 021 filed: FF-C, font subsetting
  and glyph embedding. Status: DECIDED, SCOPED, NOT STARTED — Pass
  21.x is unbuilt.** Archived at
  `docs/decisions/021-ffc-font-subsetting-and-glyph-embedding.md`.
  Requested by `pdfce-engineer` (operator priority #3, decision 019 §3.8
  build order FF-H → FF-C → FF-B, FF-H complete at `a1638f4`). **No
  §3/§4 body-section update this entry** — nothing has shipped, same
  disposition as decision 020's entry above; §3 (workspace layout) and
  §4 (core data model) get their `pdfce-render::font::subset`/
  `pdfce-core::font_embed` module rows added when Pass 21.0 actually
  lands, not now.
  - **The headline finding: FF-C as filed everywhere else in this
    project (`ROADMAP.md` R71, decision 014 §5.3, and the spec RAG's
    `font__subsetting_ffc_queue.md`) is not implementable.** All three
    describe FF-C as extending the document's own embedded font in
    place — but a subset font by definition does not contain the glyph
    being added; there is no operation on an existing `FontFile2` alone
    that produces missing outline data, only a donor face on disk or
    nothing. Verified at source, not relayed: `subsetter 0.2.6`'s
    `src/lib.rs:20–21` states embedding forces `/Type0`+`Identity-H`
    because it strips `cmap`; the spec-RAG stub's own line 42 names the
    now-wrong "add outline to `glyf`" mechanism. **FF-C is re-scoped,
    add-only: it adds a new, subsetted font resource from a donor face
    and never modifies an existing font program or font dictionary.**
    The spec-RAG stub's rewrite is dispatched to `pdfce-spec-librarian`
    before any Pass 21.0 code, per decision 021 §9.
  - **Crate boundary: `subsetter` goes in `pdfce-render`, zero new
    `pdfce-core` dependencies.** `pdfce-render::font` gains a
    `SubsetPlan` producer (parses the donor via the existing skrifa
    parser, calls `subsetter::subset`); `pdfce-core::font_embed`
    defines the plain-data contract and emits the PDF objects
    (`/Type0`+CIDFont dict+`FontFile2`/`3`+`/W`+`/ToUnicode`). This
    does not re-open standing rule **R21** (one font parser in the
    *read* path) — R21's own escape clause ("no second parser without a
    new decision record") is what this record discharges;
    `subsetter`'s internal reader never renders a glyph or reaches
    `Diagnostics`, and the `cargo tree --duplicates` guard is unchanged.
    `default-features = false` on `subsetter` (dropping the
    `variable-fonts` feature, unneeded at P0) cuts the net new-package
    cost from 2 (`subsetter`+`write-fonts`) to **1** (`subsetter`
    alone) — `PRIOR_ART.md`'s "FF-C dependency classification" section
    amended accordingly (below).
  - **Round-trip: no new exception needed, because FF-C never earns
    one.** New standing rule **R107** — FF-C only ever ADDS font
    resources, never rewrites an existing `/FontFile`/`/FontFile2`/
    `/FontFile3`/`/FontDescriptor`/`/Font`/CIDFont dictionary.
    Original content streams stay byte-identical (§5.1/R32/R46);
    incremental save (§5, R36/R70) stays the default; FF-C joins
    neither the §5.9/R58 nor §5.10/R67 forced-full-rewrite family.
    Enforced by an object-id-disjointness corpus test (the R97 shape),
    not a runtime guard — a guard in an emitter that can only allocate
    fresh object ids is unreachable by construction (R96's dead-code
    shape).
  - **Disclosure (fuzzy-never-sneaky, rule 4): new standing rules R108
    and R109.** **R108** — embedding is an explicit, per-action operator
    choice, never a default or a silent upgrade of the existing R79
    no-embed path; because subsetting is pure, the confirmation shows
    the real computed subset byte count and covered/uncovered character
    list (R98 applied), never an estimate. **R109** — font-embedding
    permission (OpenType `OS/2` `fsType`, which `subsetter` strips) is
    read from the donor *before* subsetting and disclosed; absent or
    unparseable data is disclosed as unknown, never silently treated as
    "permitted." The bit semantics themselves are deliberately not
    stated in the decision record — sourced from the OpenType spec by
    `pdfce-spec-librarian`, never from recall (rule 1); the accept/
    refuse *policy* for a forbidding/unparseable `fsType` is Ken's own
    call, filed as new Open operator question (r), below.
  - **Composite-run editability: new standing rule R110.** A composite
    run is editable only where its `/ToUnicode` is VERIFIED injective,
    per font, per session — checked against the data, never inferred
    from pdfce having authored the font (R93's own discipline, applied
    here). `Identity-H` with no `/ToUnicode` remains a permanent hard
    skip; **R65 is untouched.** Named because shipping Pass 21.0 (the
    adder) without 21.1 (the editor) would ship a capability
    *regression* against the already-shipped Std-14 add-text path — text
    pdfce can add but never again edit — while every existing counter
    (including the R85 raster oracle) reports success; this is the
    `flatten_fields`-shaped failure (correct counters, wrong artifact)
    and needs a deliberate acceptance criterion at ship, not merely a
    gate.
  - **Standing rules R107–R110 filed (four rules). Ceiling is now
    R110.** Full text: `ROADMAP.md` Standing rules. Numbering
    corrected by the engineer before filing against a
    `ROADMAP.md` that had moved underneath the scoping session (three
    librarian filings landed the same day) — see the decision
    document's own "NUMBERING CORRECTION" section and
    `tools/check-ledger-numbers.py`'s companion fix (below).
  - **Amendments filed in the same session, all cross-referenced from
    the decision document §4.2:** `ROADMAP.md` R21 (scope note,
    discharging its own escape clause), R71 (FF-C ceases to be "a
    deferred writer subsystem," now scoped Pass 21.x, trust ladder
    gains a fourth rung: refuse → offer embed (R108) → embed on
    accept), R79 ("no embedding" → "no embedding **by default**");
    `docs/decisions/012-operator-supplied-fonts.md` §6 ("the write side
    — unrelated" corrected — FF-C is the write-side consumer of
    decision 012's `--font-dir` supply mechanism); `PRIOR_ART.md`'s
    "FF-C dependency classification" section (net-cost refinement, 1
    package not 2, at `default-features = false`).
  - **`tools/check-ledger-numbers.py` companion fix, same session
    (commit `d30842c`, alongside this decision's filing).** The
    checker's ceiling report had scanned only `### Pass N` headings —
    but decision 020 claims Pass 20.0–20.7 in Backlog *prose* with no
    heading yet, so the checker reported "highest Pass family: 19,"
    true and useless, and this decision's own scoping session
    independently made the identical mistake (proposing Pass 20.x for
    FF-C before the engineer caught and corrected it to 21.x). Fixed:
    the checker now reports mentioned-but-unheaded Pass families by
    name as "CLAIMED BUT NOT YET HEADED." Folded into standing rule
    R106 as a dated amendment (below) rather than filed as a new rule,
    since it is the same subject (ledger-ceiling reads must not
    under-report).
  - **Two items filed for the operator, not decided solo** (full text:
    `ROADMAP.md` Open operator questions (r)/(s)): font-EULA policy for
    a donor face whose `OS/2` `fsType` forbids embedding/subsetting, or
    is absent/unparseable (refuse / disclose-and-acknowledge /
    disclose-and-proceed — a legal call per `docs/decisions/README.md`,
    not an engineering one); and whether Pass 21.0 refuses complex
    scripts (Arabic/Devanagari/Thai) by name given **R17 (no shaping,
    ever)** means they would embed but render wrong — recommendation is
    refuse-by-name, but it caps a headline capability so it is Ken's
    call.

- **2026-08-03 (same-day) — Decision 021 AMENDED after
  `pdfce-spec-librarian`'s dispatch: FF-C's P0 floor narrowed to `glyf`
  donors; R109 split into two refusals. Status unchanged — DECIDED,
  SCOPED, NOT STARTED, Pass 21.x still unbuilt.** Forward pointer:
  full eight-finding record archived at
  `docs/decisions/021-ffc-font-subsetting-and-glyph-embedding.md` §10
  ("Spec review (2026-08-03)"). The prior entry above (filed the same
  day, before the dispatch returned) is **not retracted** — it stands
  as the record of what was decided from crate source before the spec
  was read; this entry is the correction, per the same append-only-
  with-forward-pointer discipline this section already uses for
  superseded decisions.
  - **Scope-changing finding (C-3): `subsetter`'s CFF output cannot be
    emitted conformantly under the plan filed above.** Verified at
    source (`lib.rs:492`, `FontFlavor::Cff => 0x4F54544F`, the `OTTO`
    tag): `subsetter` wraps CFF donors in an `OTTO` sfnt. ISO 32000-1
    §9.9 Table 126 requires a `cmap` table for CFF-outline `OpenType`
    programs (the `glyf` row does not), and `subsetter` strips `cmap`
    unconditionally — so the CFF path can satisfy neither
    `/FontFile3 /Subtype /OpenType` (needs `cmap`) nor
    `/CIDFontType0C` (needs a bare CFF program, not an OTTO container).
    The prior entry's claim that *"`subsetter` absorbs the TrueType/CFF
    split entirely"* is true for simple-vs-composite dispatch, false
    for the descriptor-key choice.
  - **Scope call (librarian, recorded here as the decision this
    finding produced): Pass 21.0's P0 floor is restricted to `glyf`
    (TrueType-outline) donors; CFF donors are refused by name
    (`DonorUnsupported`, extending the same diagnostic already used for
    CFF2) until a later slice.** Not a new decision record — a
    narrowing of the already-decided Pass 21.x on a sourced constraint.
    L1 (the headline non-Latin capability) survives intact: Noto Sans
    JP/CJK, DejaVu, and most Google Fonts are TrueType `glyf`. The
    alternative — shipping a non-conformant `/FontFile3` at P0 — would
    surface late, expensively, and only under veraPDF. Flagged to Ken
    as a narrowing, not a silent cut.
  - **R109 amended: fsType is not one gate, split into two named
    refusals.** Bit 8 (`0x0100`, *No subsetting*) forbids the one thing
    FF-C ever does while still permitting whole-face embedding —
    `SubsettingNotPermitted`. Bit 9 (`0x0200`, *Bitmap embedding only*)
    is the specification's own "unembeddable" case for outline-program
    embedding — `EmbeddingNotPermitted`. Absent/unparseable `OS/2` is
    disclosed as unknown and must never be treated as bit-0 `0x0000`
    (*Installable*, the MOST permissive value) — that asymmetry is the
    reason "absent" cannot default to "permitted." Full bit table
    (`0x000F` usage sub-field 0/2/4/8, bit 0 reserved, `0x00F0`/`0xFC00`
    reserved, bits 8–9 ignored on `OS/2` v0/v1) now sourced; `ROADMAP.md`
    Open operator question (r) narrowed accordingly — the accept/refuse
    *policy* for absent/unparseable `OS/2` (and the spec-silent
    `fsType == 1`) remains Ken's call, everything else is no longer
    open.
  - **A second, independent argument for the already-decided add-only
    (W2) call (C-8), not previously cited.** ISO 32000-1 §9.9's opening
    paragraph: embedded font programs *"shall be used only to view and
    print the document"* absent contrary information, and creating new
    text needs *"a licensed copy of the font program, not a copy
    extracted from the PDF file."* An existing document's `/FontFile*`
    is therefore not an admissible FF-C donor — independent of §1.2's
    "the bytes don't exist" reason, this is "and you may not reuse them
    even where they do." Modality note: the producer-side sentence is a
    `should`, not a `shall` — do not overstate this into a blanket
    embedding prohibition. Filed as a candidate for standing-rule status
    (donor provenance), not yet assigned a number.
  - **Two favourable corrections (C-1, C-2/C-6): the prior entry
    understated its own case.** The emitted-table list omitted `HHEA`/
    `CVT`/`FPGM`/`PREP`, which §9.9 requires when present in the
    original and `subsetter` does emit (via `hmtx::subset` and, absent
    the `interjector.is_skrifa()` hinting-skip path, the hint tables).
    And `cmap` removal plus the `/Type0`+`Identity-H` choice are not
    merely `subsetter` behavior pdfce happens to inherit — §9.9 states
    both as `shall`s on conforming writers. M2 (§3.4 of the decision
    document) is spec-directed, not crate-forced.
  - **Two citation corrections (C-4, C-5), fixed in place in the
    decision document's §4.2 dispatch table:** `/CIDSet` is §9.8.3
    Table 124, not §9.7.4.2; the subset-tag prefix rule is §9.6.4, not
    §9.8.1 (which has no subset rule).
  - **No further §3/§4 body-section update this entry** — same
    disposition as both decision 020's and decision 021's original
    entries above: nothing has shipped, so the workspace-layout and
    core-data-model tables stay untouched until Pass 21.0 actually
    lands.

- **2026-08-04 (continuation 76) — Decision 021 implementation update:
  R109's fsType read SHIPPED (`58fe3f6`); an interim default policy is
  now live for two of the three previously-open sub-cases of Open
  operator question (r); the R110 primitive SHIPPED (`c0ed638`); and a
  reachability defect in R-INV-4's enforcement was found and fixed
  (`8e08e80`+`87d3cb0`+`6b69956`). Status: decision 021 unchanged
  (DECIDED, SCOPED) — this entry records IMPLEMENTATION against an
  already-decided design, not a new decision.** Full build record:
  `ROADMAP.md`'s Pass 21.1 In-progress entry and the R109/R110
  Standing-rules bullets.
  - **R109 shipped as three named refusals, read before subsetting**
    (forced by `subsetter` stripping `OS/2`): `SubsettingNotPermitted`
    (bit 8), `EmbeddingNotPermitted` (bit 9), and bits 8–9 correctly
    ignored on `OS/2` v0/v1 (the `nosubset`/`nosubset-v1` fixture pair
    — byte-identical bits, different enforcement — is what proves the
    version gate is consulted, not merely present in the code).
  - **Two of the three previously-open sub-cases of (r) now ship an
    interim disclose-and-proceed default — NOT a resolution of (r).**
    Absent/unparseable `OS/2` proceeds, disclosed as unknown (never
    modelled as bit-0 `0x0000` Installable, per the asymmetry already
    on record). `fsType == 4` (Preview & Print) also proceeds — it
    permits the embed; the additional obligation it imposes (the
    *document*, not just the font, must stay read-only afterward) has
    no PDF field to carry it and pdfce cannot enforce it, so "proceed"
    here is a pragmatic default under a named limit, not a claim the
    obligation is met. `ROADMAP.md` Open operator question (r) carries
    the same dated amendment; Ken can still choose a different policy
    for either case without any code-shape change (R109 was written to
    accept whichever policy is chosen).
  - **R110's primitive, `ToUnicodeCMap::injective_inverse()`, shipped.**
    Three named disqualifying obstructions (ligature, many-to-one
    collision, empty map); ranges materialised for this check
    specifically (ordinary `/ToUnicode` lookup stays lazy) so a
    range/single collision is not invisible to it.
  - **Reachability defect found and fixed: R-INV-4's composite-run
    refusal was unreachable from `edit-text`.** `edit.rs` asserted, in
    a comment, that composite runs are refused later by R-INV-4 — false
    by construction, because the preceding text-match stage returned
    `NoMatch` on every composite run (decoded or not) before
    `classify_font` (R-INV-4's home) could ever run. Same shape,
    different code path, as the Pass 19.4 `Tw`/R91 finding already on
    record (`ROADMAP.md` standing rules; RAG file
    `D:\dev\rag\rust\dead_guard_clause_behind_a_filter_the_guarded_case_cannot_pass.md`,
    now carrying this as a second occurrence). Fix: classify the font
    BEFORE matching text, since the refusal is a property of the run,
    not of whether the sought text is inside it — no invariant
    definition changed, R-INV-4 now simply fires when it was always
    supposed to. **No §3/§4/§5 body-section rewrite needed this
    entry** — R-INV-4's definition in §5's text-editing invariant list
    (if present) already stated the intended behavior correctly; the
    defect was in enforcement ordering, not in what was documented as
    true.
  - **Outstanding gap, flagged not fixed this entry:** Pass 21.0's own
    ship (`48c6b77`, continuation 75) never received a §3/§4
    body-section update for the new `pdfce-render::font::subset`/
    `pdfce-core::font_embed` modules, despite this section's own
    "both need to change together" discipline — carried forward as
    still-owed, not absorbed silently into this filing.

- **2026-08-04 (continuation 77) — §3/§4 body-section sync for Pass
  21.0's `pdfce-render::font::subset`/`pdfce-core::font_embed`
  DISCHARGED. No new decision — this entry closes the gap flagged in
  continuation 76's entry above and in continuation 75's original
  Pass 21.0 ship.** §3 (workspace layout) now documents both modules
  under their owning crates (`font_embed.rs`'s plain-data contract +
  `build_objects` under `pdfce-core`; `font\subset.rs`'s `plan_subset`,
  `SubsetError`, `MAX_DONOR_BYTES` under `pdfce-render`), and §4 (core
  data model) gains a full IMPLEMENTED entry for Pass 21.0 recording
  the public surface, R107's allocate-only round-trip guarantee, and
  the crate-split rationale (decision 021 §3.2) in the same terms as
  the module docstrings themselves — subsetting is a *write* concern
  and looks like `pdfce-core`'s job, but producing a subset first
  requires *parsing* the donor, and that parser already lives in
  `pdfce-render`; putting `subsetter` in `pdfce-core` would give a
  crate with no font-program parser two of them purely to avoid a
  plain-data seam, so the seam is the design and `pdfce-core` gains
  zero new dependencies. Also recorded in §4: `pdfce-core` still has no
  font-program parser after Pass 21.0 — `fontdata/` remains compiled-in
  metrics only — and Pass 21.0's contract is explicitly scoped to
  ADDING composite text, not editing it; R110/Pass 21.1 governs
  editability and is unbuilt as of this entry, so this sync must not be
  read as FF-C being complete. Sourced from the modules' own doc
  comments and public signatures (`plan_subset`, `SubsetError`,
  `MAX_DONOR_BYTES` in `crates/pdfce-render/src/font/subset.rs`;
  `FontEmbedPlan`, `SubsetGlyph`, `DescriptorMetrics`, `OutlineKind`,
  `build_objects`, `EmbeddedFontObjects`, `FontEmbedError` in
  `crates/pdfce-core/src/font_embed.rs`), not re-derived from the
  decision document. Full text: §3 and §4, above.

- **2026-08-04 (continuation 80) — Decision 022 filed: annotations in
  canvas selection (ce dimensions cannot be selected or deleted by any
  surface). Status: DECIDED, SCOPED, NOT STARTED — Pass 22.0 is
  unbuilt.** Archived at
  `docs/decisions/022-annotations-in-canvas-selection.md`. Requested by
  `pdfce-engineer`, triggered by the operator's box-select complaint on
  a CAD drawing. **No §3/§4 body-section update this entry** — nothing
  has shipped, same disposition as decisions 020's and 021's own
  entries above (§4 gets its `TargetId`/`ObjectModelProvider` reality
  update when Pass 22.0 actually lands). **The one exception, same
  logic as decision 020's decision-009 forward-pointer exception: §5.9
  gains a staleness flag (not a rewrite) and a new §5.12**, because
  the underlying finding — R58's literal text already contradicted by
  shipped code — is true today, independent of whether Pass 22.0 ever
  ships.
  - **Root cause, verified empirically, not by inspection alone.** The
    GUI's selectable-object model (`decompose_page` → `ContentStream::
    from_page`) is page content streams ONLY. ce dimensions are `/Line`+
    `/IT /LineDimension` annotations in `/Annots`, a parallel object
    space `decompose_page` never reads. `pdfce-render`'s
    `survey_page_annotations` paints them anyway (after content
    interpretation), so a ce dimension is visible and unselectable —
    confirmed on the committed fixture
    `fixtures/synthetic/dimension/linear-dim.pdf`: `object-list --hit`
    at the ce dimension's label band returns `candidates=0` although
    `dimension-list` reports one ce dimension present.
  - **A second, more severe defect found during the investigation, not
    the one reported:** there is NO way to delete a ce dimension at
    all — not by canvas selection, not by the Objects panel, not by
    any `pdfce-cli` subcommand, not by any `EditSession` method. A ce
    dimension, once authored, was permanent for the life of the
    document until this decision closed the gap.
  - **A third, latent corruption path found while reasoning about what
    delete must do:** `set_group_scale` pushes an `ObjectWrite`
    replacing every member's `/AP` stream UNCONDITIONALLY, while the
    annotation-dict half of the same loop is guarded. Deleting an
    annotation generically without pruning its `/PieceInfo` record
    would let a later scale change resurrect an orphan `/AP` at a
    removed object id. New standing rule **R113** closes this by
    requiring the prune in the SAME command as the delete.
  - **`TargetId` widens from a `u64` newtype to a two-variant enum,
    `{Content(u64), Annot(ObjId)}`** — chosen over three rejected
    alternatives (a second parallel selection channel, re-litigating
    R60's one-substrate rule for no reason; a tagged-integer partition
    of the existing `u64`, which would make every existing "handles an
    out-of-range id gracefully" site handle an annotation WRONGLY but
    QUIETLY instead; injecting annotations into `PageObjects` in core,
    which would silently diverge the GUI provider's index space from
    `EditSession::vector_surgery`'s content-only decompose — the
    identical class of failure `ARCHITECTURE.md`/decision 011's Z2
    finding already named). `canvas.rs` needs ZERO semantic changes —
    it already treats the value opaquely. New standing rule **R111**
    (selection enumerates exactly what the renderer paints) and **R112**
    (a selectable kind carries its verb set in its type, so
    move/drag-node are structurally ABSENT from the `Annot` arm, never
    a silent no-op) are the two structural rules this choice earns.
  - **Verb scope, this slice: select + delete only.** Move and
    drag-node are not "unimplemented for annotations," they are
    unrepresentable for them — R112's whole point. A ce dimension is
    one baked `/AP` appearance, not a bundle of independently-editable
    lines/nodes (§4.1) — decision 023 later builds on this exact
    finding to permanently refuse descending into a ce dimension's
    `/AP` content (R115/R116's territory).
  - **Not a fifth forced-full-rewrite family member.** Zero page
    content streams change; incremental save (R36/R70) stays the
    default. Full reasoning: `ARCHITECTURE.md` §5.12, new above. R58's
    own binding TEXT is flagged as already-stale by this finding (two
    shipped operations, `delete_object`/`delete_redaction_mark`,
    already sit outside its literal scope) but is NOT rewritten by this
    entry — decision 022 explicitly declines to narrow a standing
    rule's scope solo; `ROADMAP.md` Open operator question (v).
  - **Four standing rules filed: R111–R114.** Full text:
    `ROADMAP.md` Standing rules. Slice plan: Pass 22.0a (core) →
    22.0b (CLI, rule 11) → 22.0c (GUI). Full acceptance criteria:
    decision 022 §6.4.
  - **Five items filed for the operator, not decided solo:** whether
    the Obj tool is "everything" or "page content only" (ANSWERED by
    decision 023, below — everything, at the selection layer); whether
    a widget annotation's delete refuses by name or cascades into form
    surgery (`ROADMAP.md` Open operator question (u)); R58's wording
    fix (question (v), above); R86 sign-off itself (already-open
    question (e), not new); whether ce-dimension re-measure is wanted
    at all (ANSWERED by decision 023, below — yes).

- **2026-08-04 (continuation 80) — Decision 023 filed: the Obj tool is
  for everything — level navigation, node-level editing, ce-dimension
  re-measure, and the missing format surface. AMENDS decision 022 (see
  below; 022 is NOT edited, per `docs/decisions/README.md`'s
  append-only rule — this is a new record with a librarian-owned
  forward reference). Status: DECIDED, SCOPED, NOT STARTED — Pass
  family 23 is unbuilt.** Archived at
  `docs/decisions/023-object-tool-level-navigation-and-dimension-authoring-controls.md`.
  Requested by `pdfce-engineer`, answering decision 022's own open
  question 1 plus three operator-added scope items (level navigation,
  node-level operations, ce-dimension re-measure) and one gap report
  (units/display-type unreachable from any surface). **No §3/§4
  body-section update this entry** — same disposition as decision
  022's entry above; §4 gets its `PageObjects.containers`/`Container`/
  `ContainerKind` addition when Pass 23.2 actually lands.
  - **The operator's answer: yes, the Obj tool is for everything —
    but this is true at the SELECTION layer, not the verb layer, and
    that distinction is what makes the answer buildable without
    reopening decision 022 §4.2's anti-silent-re-measure argument.**
    The Obj tool selects everything (content, annotations, ce and pdf
    dimensions alike) and offers a `Re-measure` verb that HANDS OFF to
    the Measure tool, which owns the gesture. Recorded as a durable
    project principle in `ROADMAP.md`'s Glossary ("Obj-tool
    universality"), not just in this decision's own text, because it
    resolves a real tension (operator instruction vs. decision 022's
    silence argument) in a way future scoping calls will need again.
  - **A second, independent live violation of decision 022's own
    proposed R111 was found: form XObjects.** `pdfce-render`'s
    interpreter recurses into a `Do` on a form (§8.10, its own cycle
    set + depth cap already exist, `interpret.rs`), painting the
    form's contents individually, while `decompose.rs` emits ONE
    opaque object for the same `Do`. Every line inside a placed CAD
    block — a title block, a hatch, a nested drawing block — is
    painted and unselectable, the exact defect class decision 022
    found for ce dimensions, one object space over. **This is almost
    certainly the defect behind the operator's ORIGINAL report** — a
    CAD-exported drawing's "dimension lines" are very likely pdf
    dimensions living in exactly this kind of placed block, not ce
    dimensions at all. See `ROADMAP.md` Open operator question (t) for
    the full scoping finding and the recommendation to confirm before
    treating Pass 22.0's ship as closing the original complaint.
  - **The core object model stays flat.** `PageObjects.objects` remains
    a `Vec<VectorObject>` in paint order, index space byte-for-byte
    unchanged — the same load-bearing reason decision 022 §2 option (e)
    already established (`EditSession::vector_surgery`'s content-only
    decompose and the GUI provider's `decompose_page` must keep
    agreeing by construction, or `object-move --object N` and a GUI
    drag silently stop meaning the same thing). Hierarchy is added as
    (a) contiguous RANGES over that list for marked-content sequences
    and (b) a FOREST of content streams, one flat list per form stream
    — new standing rule **R115**. `TargetId::Content`'s payload grows
    from `u64` to `ContentPath { stream, index }` — payload only, no
    further `canvas.rs` substrate change, a direct dividend of decision
    022 choosing the enum over a tagged integer.
  - **Form-XObject aliasing is the sharpest hazard in the whole
    design.** A form's content stream is ONE object invoked N times;
    editing inside it edits every placement. Refused by name
    (`FormStreamIsShared`), the invocation count MEASURED not assumed
    — new standing rule **R116**.
  - **`NumberFormat::inch_fraction` and `FractionMode::Fraction{reduce:
    true}` are shipped, documented, spec-mirrored capabilities
    reachable from NOWHERE outside a unit test** — no operator can ask
    for a fraction-formatted ce dimension today. New standing rule
    **R117**, R83's inverse: a shipped capability reachable from no
    surface is a defect of the same class as an affordance with no
    capability. Closed by Pass 23.0's `group-set-format` CLI/GUI
    surface (new standing rule **R118** — format is decoupled from the
    scale-entry gesture that currently co-owns it).
  - **Ce-dimension re-measure is granted, re-framed as two-stage and
    disclosed — new standing rule R119**, preserving decision 022
    §4.2's silence-not-capability argument as a rule now that the
    capability is wanted. Owned by the Measure tool (endpoint handles
    on a selected ce dimension); the Obj tool routes to it, never performs
    the edit itself. `set_dimension_geometry` re-runs `author_dimension`
    and replaces `/L`/`/Rect`/`/Contents`/`/AP` — the same machinery
    `set_group_scale` already runs per member — and inherits R113's
    guarded-write discipline. `group.rs:163`'s "The immutable geometry"
    doc comment on `DimensionKind` ships FALSE once this lands and must
    be corrected in the same commit (R93 in reverse).
  - **Escape's precedence chain gains a new slot, `LeaveGroupLevel`,
    between `CancelGesture` and `ExitTool`** (Escape walks out one
    level per press, never skipping). Because a second, concurrently
    in-flight Pass also edits `resolve_escape`'s signature, the
    function's three positional `bool`s are replaced with a named-field
    `EscapeContext` struct BEFORE either Pass's slot is added — new
    standing rule **R120**, numbered despite the decision's own hedge
    that it might be "too small," on the R106 precedent that a
    methodology rule earns a number when it closes a concrete,
    identified collision risk.
  - **Node-level editing is well-defined within named bounds.** Node
    delete = remove the anchor, join the two incident segments with one
    straight segment (curvature loss disclosed, never silently
    refit — a refit would be a FUZZY operation under rule 4 and can
    only ship as a reviewable preview, deferred by name). Multi-node
    move must be ONE core plan (`plan_move_nodes`), never N sequential
    `move_node` calls. `DegenerateCtm` must NOT be raised for node
    delete — deletion needs no coordinate transform, so the refusal
    would be unreachable by construction (R96's inverse: a refusal that
    fires when it structurally cannot be wrong is as dishonest as one
    that never fires when it should).
  - **Six standing rules filed: R115–R120** (R111–R114 belong to
    decision 022, above; R112 is STRENGTHENED and R111 gains a second
    documented violation by this decision — both amendment notes are on
    their own Standing-rules bullets). Full text: `ROADMAP.md` Standing
    rules.
  - **Slice plan: Pass 23.0 (format/units, zero dependency) → Pass
    23.1 (re-measure + move, depends on 22.0) → Pass 23.2 (level
    navigation, READ-ONLY, depends on 22.0) → Pass 23.3 (node
    selection/move/delete, depends on 23.2).** 22.0 must ship before
    23.1/23.2 start, or their inherited guarded-write/sidecar-pruning
    discipline gets re-derived under pressure; 23.0 is the sole
    exception. Full acceptance criteria: decision 023 §7.
  - **Eight items filed for the operator, not decided solo** — see
    `ROADMAP.md` Open operator questions (w)–(ab), plus 022's still-open
    (u)/(v). Full text: decision 023 §10.

- **2026-08-04 (continuation 81) — Decision 024 filed: a ribbon command
  surface, and the end of the floating Accept/Reject box.** Source:
  `docs/decisions/024-ribbon-command-surface-and-the-accept-reject-problem.md`,
  written against the operator's verbatim 2026-08-04 request, with the
  supporting audit `docs/ui_specs/gesture-commit-and-shell-conventions-audit.md`
  (divergences D1–D10, the Accept/Reject redesign, the status-panel root
  cause, a ribbon assessment, and a P0/P1/P2 change list). **Two halves,
  decided independently and shippable independently** — the confirm model
  does not depend on the ribbon and ships first. Extends decision 017 +
  Amendment A (the `egui_tiles` dock); amends nothing.
  - **The reframing that drives the whole record: the operator's
    Accept/Reject complaint is about PLACEMENT, not about the confirm
    step existing.** SolidWorks — one of the two products he named —
    carries a ✓/✗ pair on essentially every modal PropertyManager
    command; what it does not do is float that pair over the graphics
    area at a position derived from the drawing. pdfce's is an
    `egui::Area` at
    `.fixed_pos(image_rect.min.x + 8.0, image_rect.max.y - 8.0)` with a
    `LEFT_BOTTOM` pivot, at three sites (`main.rs:9366`, `:10244`,
    `:11942`) — **its position is a function of the DOCUMENT**, so it
    moves on zoom, on scroll and on page change, and it sits over page
    content at the corner of a CAD drawing most likely to hold a title
    block. Read naively the complaint would license deleting disclosures
    pdfce is obliged to make; read correctly it asks for the same
    confirm at a fixed, window-relative anchor.
  - **Adopted: a hand-built ribbon — six fixed tabs (Home, Insert, Edit,
    Measure, Protect, View) plus a `File ▾` menu button, a fixed
    four-control Quick Access row (Open/Save/Undo/Redo), and contextual
    tabs keyed on the armed tool and on the selected object's kind.
    ZERO new dependencies.** No ribbon crate exists for egui in any state
    of repair (verified against crates.io and the GitHub repo/issue/
    discussion APIs — zero repos, zero issues, zero discussions), so
    pdfce would be first and there is no reference implementation to crib
    from. egui 0.35 nonetheless supplies more of the parts than expected:
    a real `MenuBar`/`MenuButton`/`SubMenu` API whose own doc comment
    recommends a `Panel::top`, and `Panel::show_switched` /
    `show_collapsible`, which is exactly the ribbon collapse/expand
    primitive already written and already animated.
  - **The `egui_tiles` dock is explicitly NOT replaced.** Nothing in the
    operator's message is about the dock; both reference products have a
    ribbon **and** persistent side panels; the two surfaces are
    orthogonal (the ribbon carries commands, the dock carries state you
    keep looking at — the line R81 already draws); and an unbounded
    Objects tree, a multi-field Properties form and a scrollable Redact
    mark list do not fit in a ~68 pt horizontal band. The dock gains one
    uniform toggle per panel on the View tab, which also normalises the
    two-mental-models problem where `Tools` opens the whole dock while
    `Properties`/`Redact` each open it *and* activate one tab.
  - **The confirm model becomes three tiers, keyed on one question — did
    pdfce infer anything the operator did not directly specify?** Tier 1
    (direct manipulation: object move, node drag, page rotate, markup
    shape authoring, whole-ce-dimension move) commits on gesture
    completion with undo as the escape hatch and shows no confirm
    control at all. Tier 2 (inference under review: snapped points,
    best-fit circles, scale derivation, derived centerlines, ce-dimension
    re-measure, the font trust ladder, reflow, Add-Text box wrap)
    commits through a **fixed-anchor tool strip** — a full-width
    `Panel::top` whose position is a function of the WINDOW, identical
    for every tool, that cannot cover document content because a panel
    shrinks the central region rather than overlaying it, and that
    orders state → action → detail (R99 at a second surface). Tier 3
    (keyboard, universal): Enter commits, Escape resolves through
    `canvas::resolve_escape`, Ctrl+Enter where plain Enter is meaningful
    in-gesture — today Enter commits in **exactly one tool of three**
    (Add Text, `main.rs:9959-9970`), which is why two of three tools can
    only be committed with the mouse at the moving target above.
    **Rejected for Tier 2: on-canvas ✓/✗ handles at the gesture** — a
    smaller floating box in a less predictable place, occluding the
    geometry being measured, unable to carry the disclosure text Tier 2
    exists to display.
  - **Rule 4 ("fuzzy, never sneaky") is proposed for NARROWING, not
    repeal — and no agent may apply it.** `CLAUDE.md` is the operator's
    file. The proposal (decision 024 §4.4, full wording there) makes rule
    4 bind **disclosure** rather than any particular widget: satisfied by
    the inferred value being on screen and the commit being a deliberate
    act at a fixed predictable position; not satisfied by a control whose
    position is derived from the document; and explicitly not requiring a
    two-click confirmation for a direct manipulation that is fully
    visible and reversible in one undo. The record checks the narrowing
    item-by-item against every rule that depends on rule 4 (R119, R71,
    R72/R75/R76, R90, R98, R118, R83, R85) and none weaken. Diagnosis
    worth keeping: three tools independently converged on a floating
    box, each citing the one before it as precedent, and the ROADMAP
    recorded the convergence approvingly — **convergence by precedent is
    not convergence on the right answer**; the rule never asked for a
    box, it asked that the operator be able to see and reject what pdfce
    inferred. Filed as open operator question **(ac)**.
  - **Five standing rules filed: R121–R125** (fixed-anchor confirm;
    keyboard commit for every gesture that has a commit; the command
    surface's taxonomy derived from pdfce's own capabilities and never
    from a competitor's menus — the ribbon-scoped extension of rule 12
    and R61; empty ribbon space stays empty, R83 at a surface that
    invites the violation; only the active tab's band is emitted, which
    is the *only* mechanism available because egui 0.35 has no focus
    group, no roving tabindex and no tab-index concept). Full text:
    `ROADMAP.md` Standing rules.
  - **Slice plan: Pass 24.0** (confirm leaves the page — fixed-anchor
    tool strip + universal Enter/Escape; **no ribbon**, no dependency on
    anything) **→ 24.1** (ribbon shell: tab strip, `File ▾`, QAT,
    Home/View bands; zero new commands, zero behaviour change) **→ 24.2**
    (tool contextual tabs; property bars come off the page) **→ 24.3**
    (selection contextual tabs; **depends on Pass 22.0 and Pass 23.2**
    and cannot be pulled forward) **→ 24.4** (collapse + whole-group
    overflow degradation; scroll arrows treated as a defect report per
    decision 017 Amendment A) **→ 24.5** (keyboard, focus, and an honest
    accessibility statement). Rule 11 does not apply to any Pass in the
    family — there is no `pdfce-cli` surface for "where the Accept button
    is" — and R85 is untouched, with `tools/content-identity` = 0 as the
    mechanical guard for the whole family.
  - **Accessibility, stated rather than implied:** egui 0.35's
    `WidgetType` has no `Tab`/`TabList`/`Toolbar`/`MenuBar` role and its
    AccessKit mapping sends `Button | CollapsingHeader | SelectableLabel`
    all to `Role::Button`. AccessKit itself has `Role::Tab`/`Role::TabList`
    — the ceiling is egui's mapping, not the backend. Ribbon tabs will
    therefore announce as buttons with a correct pressed state and
    nothing more: **the same already-documented debt `dock.rs` records
    for `egui_tiles` tabs, at a second surface.** Keytips are refused by
    name. Already captured ecosystem-wide at
    `D:\dev\rag\egui\egui_035_no_tab_tablist_widgettype.md`.
  - **What is given up, recorded so it is a conscious trade:** ~96 pt of
    chrome idle (~126 pt with a tool armed) against today's ~34 pt, i.e.
    ~10% of an 800 pt window moved from document to chrome permanently;
    one extra click for any command not on the active tab; a visibly
    sparse surface with the obvious remedy forbidden by R83/R124; a
    second hand-rolled layout surface with no prior art anywhere; and
    ~60 new `ui_text.rs` entries.
  - **Eight items filed for the operator, not decided solo** — see
    `ROADMAP.md` Open operator questions (ac)–(aj). Full text: decision
    024 §8.
  - **No `ARCHITECTURE.md` §3/§4 body-section update this filing** —
    same disposition as decisions 020–023: nothing has shipped, and
    every Pass in the family is confined to `pdfce-gui`, so §3's crate
    layout and §4's core data model describe no new reality. The one
    thing §9-adjacent to watch, named by the decision itself: a ribbon
    introduces new transient view state (active tab, collapse state,
    contextual-tab set) and the path of least resistance for "which tab
    should be active for this selection" is to ask a core type. **It must
    not** — the contextual-tab dispatcher is a pure function in
    `pdfce-gui`.

- **2026-08-04 (continuation 82) — Decision 025 filed: the subpath rung
  and the unified level ladder.** Source:
  `docs/decisions/025-the-subpath-rung-and-the-unified-level-ladder.md`
  (1,744 lines), written against the `ROADMAP.md` "Level-model
  reconciliation" flag raised at continuation 81. **Resolves that flag.**
  Extends decision 023 (the Obj tool's level navigation); **corrects
  five factual statements in it** without editing it (append-only, per
  `docs/decisions/README.md`); adopts 023's container half unchanged.
  - **The decision, in one sentence:** the **subpath is a genuine rung**
    and it goes where the measurement put it — **between object and
    node** — and the double-click/Escape collision between decision 023
    and Pass 25.1 **dissolves the moment descent stops being two
    mechanisms and becomes one state variable**. The ladder is
    page → container(s) → object → subpath → node, walked by a single
    `LevelPath` standing position.
  - **The measurement the argument had to survive, and it is the whole
    reason the rung exists.** On the operator's own SolidWorks export,
    page 1 carries ~5,900 objects and object **5870 is one stroked path
    holding 1,194 subpaths / 6,681 anchors** — a 550 × 500 pt isometric
    view that is a single object. Per-object hit testing selects a whole
    drawing view for any click. Decision 023 §0 asserted that a
    flattened CAD plot has **two** levels below the page (*"objects,
    nodes — there is nothing to descend into"*); it has **three**, and
    the omitted one is the only one that makes the file selectable.
  - **Five statements in decision 023 are now factually WRONG** — not
    incomplete; wrong in the sense that a reader acting on them would
    build the wrong thing. Summarised here because the decision log is
    the audit trail a future session reads first: (1) §1.3's specified
    readout string *"this object is not inside a group (already at
    object level)"* and §7.3's criterion **C6** that requires it — C6
    would **certify a false statement** on the operator's own files;
    (2) §0's two-levels-on-a-CAD-plot enumeration; (3) §1.3's *"that is
    the file's structure, not a pdfce limitation"* — it **was** a pdfce
    limitation, the third instance of the R111 paint/select asymmetry,
    and Passes 25.0/25.1 removed it; (4) §4.6's node-ceiling arithmetic
    (superseded rather than wrong in itself — the node rung shows **one**
    entered subpath's anchors, ≈6 per part on the measured page, not
    41,208 handles for a whole group); (5) `LeaveGroupLevel` renamed
    **`LeaveLevel`**, a misnomer at three of five rungs. Two further
    items are **narrowed rather than wrong**: open question (z)'s "three
    levels" is four-plus, and §1.2's heuristic-grouping refusal stands
    and is upheld — the subpath rung passes all three of its own grounds.
  - **The concrete fix, and it is a deletion.**
    `Action::ClearCanvasSelection` (`main.rs:4878-4886`) clears the
    selection set **and** sets `entered = None` — two rungs, or five,
    collapsed into one Escape press, contradicting decision 023 §3.2's
    own testable property that *"Escape walks all the way out, one step
    per press, and never skips a step."* Delete `doc.entered = None`;
    add `Action::LeaveLevel` at **Escape slot 2**, above `ExitTool`.
  - **The sharpest new hazard: `DeleteWouldMoveNextSubpath` (§5.5) —
    decision 023 §1.4's form-aliasing trap, one object space down.**
    After `h`, `close_subpath` sets `pa.current = pa.subpath_start`; a
    following `l`/`c`/`v`/`y` then opens a subpath whose `start` is
    **inherited from the closed subpath and carried by no operand of its
    own**. Excise the predecessor and the follower's start point
    **silently moves**. The edit is byte-minimal, byte-verifiable, and
    passes every round-trip check — **§5's minimal-diff discipline
    cannot catch it.** Only a named refusal can. Pass 25.2 covers it
    conservatively via its structure guard; the named refusal is still
    owed (question (ap)).
  - **A core-model gap this record names and pdfce has not closed:
    `Subpath` carries no byte span.** `PathObject` has `tokens:
    TokenRange` and `bytes: ByteSpan`; `Subpath` has `start`,
    `segments`, `closed`. So `move_subpath` is **not expressible**
    against today's core model — not hard, *not expressible*. Filed as
    standing rule **R134**, with Pass 25.2's re-derivation guard
    recorded as a deviation that satisfies the intent for excision only.
  - **Pass plan:** 26.0 (the ladder, node rung, Escape fix, readout
    matrix, subpath click-cycle), 26.1 (subpath verbs), 26.2 (level
    survival across an edit). **Pass 23.2 SPLITS** — core+CLI half
    stands, GUI half superseded by 26.0, criterion C6 amended. **Pass
    23.3's dependency changes from 23.2 to 26.0**, a genuine unblocking.
    22.0 still first.
  - **Standing rules R130–R134; open operator questions (ak)–(ap).**
  - **Shipped-ahead-of-filing deviation, recorded rather than
    smoothed:** Passes 25.2 and 25.4 delivered parts of 26.1 and 26.0
    respectively **before this record was registered** — see the
    `ROADMAP.md` Pass 26.0–26.2 entry's deviation table.
  - **No §3/§4 body-section update this filing.** Nothing of the ladder
    has shipped; `LevelPath` is `pdfce-gui` state and correctly never
    reaches §4. The one §4 obligation this record creates is R134's span
    on `Subpath`, filed as **owed** rather than written into §4 as if it
    existed — §12 is the audit trail, §4 describes shipped reality.
    Pass 25.0's `vector::hit_test_subpaths`/`subpath_bounds` and Pass
    25.2's `plan_delete_subpath` **are** shipped core surface and remain
    owed a §4 line at the next §3/§4 sync — carried forward from
    continuation 81, now with more on it.
  - **★ FORWARD POINTER, added 2026-08-05 (continuation 85): this record
    is AMENDED by decision 028**
    (`docs/decisions/028-the-node-rung-marks-handles-hit-priority-and-the-clip-gate.md`).
    **025 is not edited.** 028 answers the three things 025 §11 left
    open (the node ceiling, the breadcrumb's design, the node marks) and
    **finds three defects in 025 as written** — the Node readout row
    needs a **handle-presence clause** (decision 023 §4.5's *"the
    node-level readout must not imply handles exist"* went false when
    Pass 30.1 shipped handle editing); the **already-live, ungated
    node-drag gesture must be REPLACED AND GATED by the rung, not
    shipped alongside it** (else the ladder ships the very
    two-mechanisms failure 025 §2.1 refused for descent); and the
    **Subpath readout row omits the descent disclosure the Object row
    has**. **Read 025 and 028 together.**

- **2026-08-04 (continuation 82) — Decision 026 filed: linear
  ce-dimension geometry, the offset model, and drafting standards.**
  Source:
  `docs/decisions/026-linear-ce-dimension-geometry-offset-and-drafting-standards.md`
  (1,529 lines), written against the operator's verbatim 2026-08-04
  report that a **ce dimension** constrained to Horizontal *"shows at an
  angle."* Extends decision 011 (the scaled-measurement/dimensioning
  tool); amends nothing. **Rule 15 applies throughout: this record and
  this entry concern ce dimensions — the ones pdfce authors — never pdf
  dimensions.**
  - **★ The root cause is architecturally interesting and is what R136
    was written from: a deliberate, documented PREVIEW/COMMIT
    DIVERGENCE, not a forgotten parameter.**
    `LinearPick::preview_segment` drew the **constrained** segment while
    `LinearPick::commit_point` stored the **raw** second pick —
    justified in the module's own doc comment as byte-equivalence with
    the CLI. The stored-raw decision is **correct and was kept** (the
    raw point anchors the second extension line); what was never done is
    teach the appearance path that `b` is a *measured* point rather than
    a *dimension-line endpoint*. The operator was shown a horizontal
    line, clicked, and got a diagonal one. **The justification covered
    the value; nobody checked the drawing.**
  - **The model change is one signed scalar, chosen so the migration is
    free.** `offset: f64` on `DimensionKind::Linear` — signed,
    page-space points, along a canonicalised normal, based at `a`,
    default **`0.0`**. `0.0` reproduces the previously committed
    geometry exactly for an already-axis-aligned pick **and** reproduces
    the preview the operator was shown for every pick. Pass 27.1 added
    the parallel half, `text_along`, on the same additive pattern.
  - **★ A latent DATA-LOSS CLIFF in `dimension/sidecar.rs`, named by the
    record and initially shipped past by the Pass.** `deserialize_model`
    gated on `Version == SIDECAR_VERSION` with **exact equality**; any
    mismatch returned `None`, the caller started a **fresh model**, and
    every group, every calibrated scale and every membership was
    **silently gone** — while the `/Line` annotations kept rendering, so
    nothing looked wrong until the next save made the loss permanent.
    Reading is now a **range**; writing over a newer version is a
    **named refusal** (`EditError::SidecarWrittenByNewerBuild`) checked
    at all seven mutation sites **before** any mutation. Filed as
    **R138**, with scope generalised past sidecars to any versioned
    private data pdfce writes. Same family as
    `DeleteWouldMoveNextSubpath` above: a byte-minimal operation that
    destroys unrecoverable operator work, catchable only by a named
    refusal.
  - **★ A standards misattribution corrected before any code was
    written.** The natural assumption — **ASME Y14.5** — is **wrong**:
    Y14.5 is the GD&T/tolerancing standard. Arrowheads, line conventions
    and lettering live in **ASME Y14.2**. On the ISO side, three rules
    that could easily have been treated as convention are verified
    *"shall"* clauses of **ISO 129-1:2018 cl. 4.1.1**: text above an
    **unbroken** line; vertical text read from the right with
    orientation determined at the centre of the dimension; and **a comma
    as the decimal marker**. The widely-taught **"30° ambiguous zone" is
    NOT in the 2018 edition** — folklore carried from ISO 129:1985.
  - **The comma forced a design revision, and the fix is the interesting
    part.** ISO's mandated comma is expressible **portably** via ISO
    32000-1 §12.9 Table 263's `/RD`, so it can be governed by the
    drafting standard **without** breaking the `/Measure` agreement
    contract — provided `/RT` is set alongside it, **because `/RT`'s own
    spec default is also a comma** and every grouped number would
    otherwise render `1,234,56`. Generalised into **R139**: the standard
    governs how a ce dimension is **drawn**, never what it **measures**,
    and a presentation rule that cannot be projected into Table 263 is
    not implemented (ANSI inch leading-zero suppression is the live
    example, question (at)).
  - **★ Side-finding, same family, otherwise invisible: pdfce's own
    ce-dimension label and its own `/Measure` mirror can ALREADY
    disagree.** pdfce prints `3.10 m` (fixed places, `units.rs:283-285`)
    while the mirrored dict omits `/FD`, whose default `false` permits a
    conforming reader to print `3.1 m` (`measure_dict.rs:115-123`).
    `NumberFormat::format`'s doc comment claims the two *"agree by
    construction."* **They do not** — an instance of R93, found by
    reading rather than by a failure.
  - **What pdfce may NOT claim — the claim-bearing-copy discipline
    applied to a drafting claim.** ISO 129-1:2018's normative **Annex A
    is paywalled**, so pdfce's ISO geometry is convention-informed.
    Operator-facing strings read **"ANSI / ASME (US)"** and **"ISO
    (international)"** — never "ASME Y14.5", "ASME Y14.2", "ISO 129-1",
    never "conformant" — asserted against `ui_text` so a copy edit
    cannot reintroduce a claim. Whether to purchase the standard is
    question (au).
  - **Pass plan:** 27.0 (geometry + offset + the sidecar gate,
    **SHIPPED** `5e93bec` → `104162d`), 27.1 (placement drag, **SHIPPED**
    `7ed90a2`), 27.2 (ANSI/ISO standards, **NOT STARTED — the
    outstanding half of the report**), 27.3 (text-along, **largely
    ABSORBED by 27.1**; see question (aq)).
  - **Standing rules R135–R139; open operator questions (aq)–(au), of
    which (aq) is ANSWERED — SolidWorks semantics** (both halves of the
    placement from one drag, measured points pinned; sourced from the
    operator's own SolidWorks API RAG, `IModelDoc2.AddDimension2`'s
    *"text-placement point"*).
  - **A process failure worth the decision log's space, because R141 was
    written from it:** Pass 27.0 was **declared shipped with criterion
    C6 unmet** — the very sidecar gate this record had already named as
    a latent data-loss cliff. Caught by an autonomous post-ship check,
    not by review; completed the same day (`104162d`) and filed as
    *"Pass 27.0 completing C6"*, not as a new Pass.
  - **§4 body-section obligation, owed and named:**
    `DimensionKind::Linear` has gained `offset` and `text_along`, and
    `EditError` has gained `SidecarWrittenByNewerBuild` — **shipped core
    surface**, so §4 is now behind on the dimension model as well as on
    Pass 25.x's vector surface. Both go in at the next §3/§4 sync.

- **2026-08-04 (continuation 82) — Forward pointer on decision 023's
  ledger entry (above, continuation 80).** Decision 023's entry is **not
  edited** (append-only); readers of it must also read **decision 025
  §9**, which corrects **five** of its statements as factually wrong —
  including its acceptance criterion **C6** and a specified UI string
  that would ship a false claim on the operator's own files — and
  narrows two more. Decision 023 remains authoritative for its
  **container half**, which decision 025 adopts unchanged.

- **2026-08-05 (continuation 83) — Decision 027: REFUSE what has no good
  reading; DISCLOSE what has one. Plus the API change that makes
  disclosure possible at all.** Filed by `pdfce-librarian` at the
  engineer's referral on Pass 30.0's ship (`a56bdd7`); scope is the
  **vector-edit planner surface** in `pdfce-core`, with consequences for
  `pdfce-cli` and `pdfce-gui`. Full record:
  **`docs/decisions/027-refuse-what-has-no-good-reading-disclose-what-has-one.md`**
  — filed **post-hoc against shipped code** and **not** consultant
  output, which that file states up front; it exists because
  `tools/check-ledger-numbers.py` derives the live decision ceiling from
  `docs/decisions/NNN-*.md` **only**, so a decision recorded solely here
  would be invisible to the R133 guard and the next session would be told
  `next free is 027`.

  - **The decision, in one line.** Moving a clipping path is **disclosed,
    not refused**; deleting part of one **stays refused**. The
    distinguishing test is **whether a legitimate operator intent
    exists** for the gesture — not how dangerous it is, and not whether
    the consequence is local.
  - **Why that test and not "danger".** Both gestures are dangerous in
    the same way: a `W`/`W*` clip governs what **other** content is
    visible, so editing it changes the page **somewhere other than where
    the operator is looking**. Ranking by danger gives no separation.
    Ranking by intent does: **resizing a crop region is a real task** a
    draughtsman performs on purpose, and refusing it leaves clip geometry
    **permanently uneditable** — a refusal with no path to "yes", which
    is a capability hole disguised as safety. **"Delete part of a clip"
    has no reading worth guessing at** — there is no operation the
    operator could have meant that pdfce could then perform correctly.
    Refuse the one with no good reading; disclose the one that has one.
  - **How this sits with rule 4, precisely.** The narrowed rule 4
    (decision 024 §4.4) removes the confirm step from **direct
    manipulations the operator performed** whose result is visible and
    reversible in one undo. A clip drag is a direct manipulation, so the
    narrowing applies — but it is the case where the narrowing's own
    premise (*"fully visible on the canvas"*) is **weakest**, because the
    visible result and the material result are in different places. That
    tension is not resolved by fiat: it is **open operator question
    (av)**, with disclose-and-proceed as the shipped default and a
    **fixed-anchor confirm** (R121) named as the alternative if the
    operator disagrees. A refusal is explicitly **not** among the
    alternatives, for the permanent-uneditability reason above.
  - **★ The API change, which is the durable half.**
    `vector::PlannedEdit` gained **`disclosures: Vec<String>`**, and five
    `EditSession` methods — `move_object`, `delete_object`,
    `move_subpath`, `delete_subpath`, `move_node` — changed from
    `Result<(), EditError>` to **`Result<Vec<String>, EditError>`**.
    **The rationale generalizes past clipping paths and is why this is a
    decision rather than a bug fix:** `Result<(), _>` has **no channel**
    for "succeeded, *and* here is what you need to know." With no
    channel, every caller drops that information **by default rather
    than by decision**, and no review catches it because there is nothing
    at the call site to see. This is the **same** failure `pending_note`
    was added to the GUI to fix — now confirmed in two layers. Recorded
    as **standing rule R145**.
  - **Routing, both halves load-bearing.** CLI prints disclosures to
    **stderr**, keeping the stdout record machine-parseable (**pinned by
    a test**); GUI routes them through **`pending_note`**. A disclosure
    only a human at a terminal can see is not delivered to a batch
    caller; a disclosure on stdout breaks every script that parses it.
  - **Two error variants REMOVED as a consequence:**
    `VectorEditError::RectangleNode` and `::ImplicitNode` have no
    remaining producers — Pass 30.0 materializes the operands whose
    absence they reported (`re` → `m`/`l`/`l`/`l`/`h` per ISO 32000-1
    §8.5.2.1 Table 59; an inherited subpath start → an explicit `m`).
    Their tests were **rewritten, not deleted**; the rectangle one became
    an **undo** test and is the stronger case, because undo must restore
    a stream **shorter** than the one it undoes (five operators back to
    one) — a length change a same-length rewrite never exercises.
  - **Two standing rules originate here besides R145: R143** (a refusal's
    stated reason is re-verified before it is used to scope work — from
    Pass 29.0's self-justifying R-INV-4) and **R144** (removing a refusal
    can remove an unrelated protection the refusal was incidentally
    providing — from this Pass's clipping-path discovery, and the reason
    a newly-lifted gesture is run against a **real file**, not only a
    fixture, before shipping).
  - **§4 body-section obligation — now THREE deep, and named again.**
    §4's API contract is behind on: Pass 25.x's vector surface and the
    ce-dimension model (`DimensionKind::Linear`'s `offset`/`text_along`,
    `EditError::SidecarWrittenByNewerBuild` — both owed since decision
    026), **plus** this decision's `PlannedEdit::disclosures`, the five
    changed `EditSession` signatures, and the two removed
    `VectorEditError` variants. All are **shipped core surface**. This is
    the second consecutive filing to name the §4 debt without clearing
    it; the next §3/§4 sync should be scheduled as work, not assumed to
    happen incidentally.
  - **Same-day follow-up (Pass 30.1, `d025c1a`), appended here rather
    than filed as a new decision, because it changes nothing — it
    CONFIRMS.** Pass 30.1's Bézier-handle drag re-spells a `v`/`y`
    segment as the `c` that states both control points (§8.5.2.1
    Table 59's implicit control points cannot both stay implicit and
    move) and **discloses it through this decision's channel unchanged**
    — the first consumer beyond clipping paths, and the evidence that
    `PlannedEdit::disclosures` was an API-shape fix rather than a
    one-off. It refuses a straight segment (`NoHandleHere`) on the same
    discriminator: an intent exists for reshaping a curve, none for
    silently promoting a line the operator never drew as a curve. **One
    cost recorded:** the `# Returns` blocks added by this decision's
    signature change landed **above** each summary line on all five
    methods, making *"# Returns"* the rustdoc one-liner for the family;
    fixed in `d025c1a`. A uniform mechanical edit across a verb family
    fails uniformly — worth expecting next time. **§4 is therefore also
    behind on `plan_move_handle`, the `Handle` enum and
    `VectorEditError::NoHandleHere`**, i.e. the debt named above grew in
    the same hour it was recorded. Full text: the amendment section of
    `docs/decisions/027-refuse-what-has-no-good-reading-disclose-what-has-one.md`.

- **2026-08-05 (continuation 85) — Decision 028 filed: the node rung made
  visible — marks, handles, hit priority, the breadcrumb, and the clip
  gate.** Source:
  `docs/decisions/028-the-node-rung-marks-handles-hit-priority-and-the-clip-gate.md`.
  Returned by `pdfce-ui-specialist`, dispatched to design the Node rung
  before Pass 26.0 is built. **A design REVIEW of decision 025, not a new
  architectural choice** — 025 §11 explicitly deferred the ceiling value,
  the breadcrumb's visual design and the operator-facing noun for a node
  to a specialist. **025 is amended, never edited** (append-only, per
  `docs/decisions/README.md`); a forward pointer is on 025's own ledger
  entry above. **No new Pass family** — 26.0–26.2 were already claimed by
  025. **No new operator question** — the posture recommendation is filed
  as an amendment to the existing **(av)**.
  - **★ The one REQUIRED item, and it is a shipping-ORDER requirement,
    not a cosmetic one.** There is **already a blind, ungated node-drag
    gesture in the shipped GUI**: `vector_edit_tool::classify_drag` runs
    over `object_provider::object_sample_points`, **the whole object's
    flat anchor list**, consulted with no rung state and with no marks
    drawn beforehand. It **must be REPLACED AND GATED** by the Node rung
    **in Pass 26.0's head slice** — shipping the rung *alongside* it
    would give the ladder **a second, unscoped, invisible route to the
    same edit**, which is precisely the two-mechanisms failure **decision
    025 §2.1 diagnosed for descent and refused** (*"Two predicates
    answering 'descend?' is R92's shape"*), reproduced one rung down.
  - **★ This is R144's SECOND firing, on the same Pass (30.0), and it
    yields standing rule R147.** `Subpath::anchors()` yields `start`
    **plus each segment end**, so **`re` corners and `h`-reopened
    inherited starts have ALWAYS been candidates for that gesture**.
    Before Pass 30.0 the drag was **refused on release**
    (`VectorEditError::RectangleNode` / `::ImplicitNode`); after Pass
    30.0 the **identical drag succeeds**, so **on a clipping path,
    content elsewhere on the page can now change from a gesture that
    previously did nothing**. The engineer did not check it **because he
    was reasoning about the core and the protection lived in the
    callers** — which is R147 exactly: *when a refusal is removed, audit
    its CALLERS, not just its own module.* **R144 says the protection can
    vanish; R147 says where to look for it.**
  - **A ROADMAP claim is corrected by this, and the correction is
    recorded rather than quietly patched.** The Pass 26.0–26.2 entry's
    *"every anchor on a page is addressable by the core planner and
    **none** of it is addressable by hand"* is **imprecise**: `re`
    corners and reused starts were already reachable by hand. **Only
    Bézier handles (Pass 30.1) are genuinely unreachable without the
    CLI.** The **R117 framing of the priority raise survives intact** —
    it is *more* justified, not less.
  - **Two further defects in 025 as written.** (1) **Decision 023 §4.5 is
    now factually wrong** — *"Nodes ≠ handles … the node-level readout
    must not imply handles exist"* went false when **Pass 30.1 shipped
    handle editing the same day** (`d025c1a`). 025's Node readout row
    needs a **handle-presence clause**, and that clause is the **only way
    an operator ever learns handles exist**, making it an **R83**
    obligation. (2) **025's Subpath readout row omits the descent
    disclosure the Object row has** — at the rung immediately above the
    node rung, the operator is never told that double-clicking descends.
  - **The design answers, each reusing an existing vocabulary rather than
    minting a fourth.** **Node marks:** hollow 6×6 unselected / filled
    8×8 selected, in **SCREEN** space, `SUBPATH_OUTLINE_COLOR` + the app
    accent — **square vs. circle carries node-vs-handle without relying
    on colour (R84 satisfied by construction)**; ceiling **300 nodes per
    subpath**, **provisional under R86**, with an explicit *"points not
    shown"* string — **never silent truncation**. **Handles:** hollow
    5×5 / filled 7×7, shown for **every node of the ENTERED subpath that
    has one**, not selected-node-only, **because handles are what let the
    operator decide WHICH node to pick**; tied to their node by a
    **dashed 1.0 px arm reusing `APPROXIMATE_OUTLINE_DASH`**'s existing
    *"this is not a measured edge"* signal; a straight segment's
    **absent** handle is disclosed **in the status line**, never as a
    ghost widget (R83; the core already refuses by name via
    `NoHandleHere`). **Hit priority: handle (5 px) → node (6 px) →
    subpath body → nothing — the SMALLER target first**, because a handle
    sits close to its node **exactly when the curve is nearly flat**, and
    node-priority would make it unreachable precisely then; **handle drag
    is Node-rung only**. **Keyboard:** Tab/Shift+Tab cycle nodes in
    **OBJECT-scoped** order (**R92** — so what Tab lands on and what
    `node-move --node N` addresses never disagree), arrows nudge 1 pt,
    Shift+arrow 10 pt. **Breadcrumb** (net new — nothing exists today):
    `Page › Path #5870 › Part #667 › Point #1,204`, each segment
    clickable to ascend; **its growth after the first double-click is
    itself the confirmation that the gesture did something**, which is
    how 025 §3.5's *"inside-with-nothing-selected looks identical to
    outside"* hazard is actually discharged. **Pass 28.0's subpath move
    gets its GUI gesture here** — a plain drag on the entered subpath's
    body, which falls out of the hit-priority ladder's third rank for
    free.
  - **Recommendation on open question (av) — RECORDED AS A
    RECOMMENDATION, NOT A RESOLUTION; shipped behaviour is unchanged.**
    Post-hoc disclosure is **SUFFICIENT** for `re`-corner expansion and
    `v`/`y` → `c` handle promotion (**Tier 1** — the picture is
    **byte-identical**, nothing changed where the operator is not
    looking, Ctrl+Z is a complete escape hatch; **gating these would be
    decision 024's "Over-application A"**), and **INSUFFICIENT for a clip
    move** (**Tier 2**). The reasoning is argued from the narrowing's own
    text: the Tier-1 carve-out is *"a direct manipulation whose result is
    **fully visible on the canvas**"*, and a clip's consequence **lands
    elsewhere on the page, possibly outside the viewport**. **★ It meets
    Tier 2's test even though NOTHING WAS INFERRED in the fuzzy sense** —
    the uncertainty is about **WHERE THE CONSEQUENCE LANDS**, not about a
    guessed value, so it is Tier 2's *second* limb (*a disclosure must be
    read before the result becomes document state*) that is satisfied,
    not its first. **The mechanism introduces nothing new:** route
    clip-gated drags to the **EXISTING window-anchored tool strip**
    (decision 024 §4.2's Tier 2 mechanism), Accept/Reject at its **fixed
    right anchor** — satisfying the operator's hard constraint (**R121**)
    that a confirm must not be positioned relative to the page — Enter
    accepts, **Escape rejects through the existing `resolve_escape`
    chain** (**R122**), and the strip shows the **CORE-AUTHORED
    disclosure string verbatim, never a `ui_text` paraphrase**. **One new
    symbol only:** a read-only provider predicate
    `object_is_clipping_path(index) -> bool`, **mirroring the core's
    existing `is_clipping_path` (`edit.rs:595`) rather than re-deriving
    it** (**R92**).
  - **No §3/§4 body-section update this filing.** Nothing of the node
    rung has shipped; the marks, handles, breadcrumb and hit priority are
    all `pdfce-gui` view state and correctly never reach §4. The **one**
    §4 obligation this record would create — `object_is_clipping_path` —
    is a **`pdfce-gui` provider predicate**, not core surface, and it is
    **conditional on Ken's answer to (av)**, so it is filed as *owed if
    adopted* rather than written into §4 as if it existed. **The standing
    §4 debt is unchanged and still unpaid** (Pass 25.x's vector surface,
    decision 026's ce-dimension model, Pass 28.0's `Subpath` data-model
    change, decision 027's `disclosures` + five changed signatures + two
    removed variants, Pass 30.1's `plan_move_handle` / `Handle` /
    `NoHandleHere`) — **this is the third consecutive filing to name it
    without clearing it**, and it remains a scheduled *Next up* item that
    must be dispatched as its own task with its own read of the crates.

- **2026-08-05 (continuation 85) — Two correctness fixes to shipped GUI
  surface, filed with no Pass ID** (`5b2682b`, `075e8f8`). Recorded in
  §12 rather than only in `ROADMAP.md` because both carry a reusable
  finding. **(1) `5b2682b` — a refused page rotation told the operator
  nothing.** The GUI rotate buttons discarded `rotate_pages`'s `Result`
  behind a comment asserting *"A refusal is impossible for a ±90 turn on
  a page the view is already displaying"* — while the same comment named
  the certification gate. `rotate_pages` opens with
  `check_certification()?`, so on an enforced DocMDP document
  (**§12.8.4 Table 258**) it refuses, and the operator got a button that
  did nothing and said nothing. **The impossibility claim was already
  contradicted by `pdfce-core`'s own
  `an_enforced_certification_refuses_structural_edits_by_name`.** Fixed
  through `pending_note`, **quoting the engine's reason verbatim** — a
  fixed *"could not rotate"* tells an operator holding a signed document
  nothing about **why**. **★ The finding: a discard WITH a justification
  is harder to find than a bare one, and no more correct.** The bare
  `let _ =` instances were fixed in `d8b9735` **precisely because they
  looked like shortcuts**; this one **read as considered and survived**.
  Filed as an audit-methodology corollary to **R145**, not a new rule —
  its actionable content is **R143**'s (*a stated reason is a claim to
  test*) pointed at a different construct. **The audit's other candidate
  was deliberately LEFT**: `refresh_pages`'s
  `if let Ok(pages) = self.session.pages()` **is a read**, and keeping
  the previous page list is reasonable refresh degradation — recorded so
  a future audit does not convert a sound degradation into a spurious
  error surface. **(2) `075e8f8` — the node-grab radius is a screen
  measure, not a page measure.** `NODE_GRAB_TOLERANCE` was a fixed `6.0`
  in **page** space, its own doc comment calling zoom-invariance *"a
  follow-up refinement"* — the defect was **known, named and deferred in
  writing**, which is why it did not read as a bug. Renamed
  `NODE_GRAB_SCREEN_TOLERANCE_PX` and converted through
  `canvas::screen_tolerance_to_page` like every sibling tolerance;
  `classify_drag` now **takes the tolerance as an argument** so the pure
  classifier stays free of view state. **Swelling is the dangerous
  direction on this project's files**: a measured CAD export holds
  **6,681 anchors in one path object** and the grab searches the whole
  object's list, so at high zoom the radius sweeps up anchors from
  subpaths the operator is not pointing at. **★ A test self-correction
  worth keeping:** the first fixture placed anchors **10 pt apart** and
  **failed at zoom 0.25** — **correctly**, because at that zoom they are
  2.5 screen px apart and a 6 px grab genuinely cannot distinguish them.
  That is what *"zoomed too far out to aim at individual points"* means.
  The fixture was corrected to separate the anchors **on screen at every
  tested zoom**, so the test asserts the **operator-experienced
  property** rather than restating the implementation.

- **2026-08-05 (continuation 87) — decision 029: development stays ONE
  session over the whole workspace; sessions are NOT partitioned by
  crate.** Source:
  `docs/decisions/029-single-session-vs-crate-partitioned-sessions.md`.
  **The outcome is NO CHANGE**, and it is recorded anyway because a
  "no change" with reasoning behind it is the easiest thing to lose —
  **the next person to notice the clean crate split will propose the same
  thing.** The operator proposed **three parallel sessions, one per crate
  (`pdfce-core` / `pdfce-cli` / `pdfce-gui`)**, to keep a usable GUI while
  features were built; the engineer argued against; **the operator agreed
  to keep things as they are, on his own stated ground: he did not want
  to do anything that reduces the ability to catch errors.**
  - **★ The load-bearing distinction: a crate boundary is a DEPENDENCY
    DIRECTION, not a work boundary.** §3's layout says what may reference
    what; it says nothing about who works where. **The work crosses it by
    rule** — CLAUDE.md rule 11 requires each feature Pass to ship its
    `pdfce-cli` subcommand alongside the GUI flow in the same session —
    and **crosses it in fact**: measured on this session's commits
    (engineer's figures), the substantive feature Passes were
    **cross-crate** (Pass 30.0 touched **all four crates in one commit**,
    30.1 touched two) while the **single-crate commits were mostly
    fixes**. The proposal would partition sessions along an axis the work
    does not follow.
  - **★ The decisive argument, and it answers the operator's own
    criterion: this session's three most valuable findings were all
    BETWEEN crates.** (1) **R144's second firing** — a refusal removed in
    `pdfce-core` silently un-gated a `pdfce-gui` drag that relied on it,
    which is **R147** exactly (*the protection is felt where it is
    INVOKED*). (2) The **clipping-path gap**, whose consequence lands on
    page content the operator may not be able to see. (3) The **reflow
    auto-width defect** (Pass 33.0 / **R148**) — a composition of **two
    individually-correct operations** that **neither module's tests can
    catch**, because each is correct by its own lights. **All three are
    seam defects, and a crate-partitioned setup optimises for exactly
    that blindness** — a complete view of one module, no view of the
    composition.
  - **The ledger argument.** Four hand-maintained numbered ledgers (Pass
    IDs, standing rules, decision records, operator questions) are
    primary keys. **Pass ID 31.0 was burned this session by ONE librarian
    racing ONE engineer**; five collisions were found on 2026-08-03 alone
    (the reason **R106** and `tools/check-ledger-numbers.py` exist).
    Three concurrent number-minting sessions would make collisions
    routine, and the checker detects them **after** the fact.
  - **The single-writer argument.** `ROADMAP.md` and `SESSION_LOG.md` are
    the highest-churn files in the repo (**5 of the last 12 commits**,
    engineer's figure), are **append-only by rule**, and have **one
    writer by deliberate design**. Three sessions means three concurrent
    writers to append-only history.
  - **★ The better answer to the operator's ACTUAL goal, recorded as an
    OPTION and explicitly NOT as a decision.** The goal was *"a GUI in a
    stable state I can use"* — a statement about having a working build,
    not about session topology. **A release build to its own folder**
    (§6's single-folder-portable target, already a project requirement)
    **decouples "usable app" from "how many sessions are running"
    entirely.** The operator **declined for now**; it is recorded so it
    is available, not so a future session treats it as authorised work.
  - **No §3 body change.** The crate split, its dependency direction and
    its `cargo tree` verification are **unchanged and unchallenged** by
    this decision — what was decided is who works where, not what depends
    on what.
  - **Ledger note:** this record spends decision **029**, which
    continuation 86 had deliberately left free for a possible Pass 33.0
    fix record. **That record, if written, takes 030.** The superseding
    note is appended to R148 in `ROADMAP.md`. The checker was not run at
    filing time (librarian hard rule 8); **re-run it after committing.**

- **2026-08-05 (continuation 88) — §4 SYNCED against the crate for the
  first time since Pass 21.0, and it was WRONG rather than merely
  incomplete. NOT a decision — a correction, logged here because §12 is
  the audit trail and a body section that changes this much should be
  findable from it.** New subsection **§4.1**, produced by reading the
  `pub` items in `crates/pdfce-core/src/` rather than reconstructing them
  from `ROADMAP.md` — **the roadmap records intent, the crate records
  truth, and where they disagree the crate wins.** No prior text was
  deleted; the superseded claims carry inline `[SUPERSEDED]` notes so a
  reader of an older checkout can still tell what moved.
  - **Three BREAKING contract changes are now recorded AS changes**, per
    the scheduled item's own acceptance criterion: **§4.1(B)** `Subpath`
    gained `tokens: TokenRange` + `starts_implicitly: bool` (Pass 28.0);
    **§4.1(C)** decision 027's `PlannedEdit::disclosures`, the five
    `EditSession` vector methods moving from `Result<(), EditError>` to
    `Result<Vec<String>, EditError>`, and the **removal** of
    `VectorEditError::RectangleNode` / `::ImplicitNode`; **§4.1(D)** Pass
    30.1/26.1's `plan_move_handle` / `Handle` / `NoHandleHere` /
    `EditSession::move_handle` (additive, but the sixth member of the
    changed family).
  - **★ The migration hazard worth surfacing at §12 level, because it is
    not a compile error: (C.2)'s return-type change breaks LOUDLY only at
    call sites that BOUND the result.** A pre-027
    `session.move_object(..)?;` that discarded `()` still compiles and now
    **silently drops a rule-4 disclosure**. A breaking change that some
    callers absorb silently is worse than one that breaks all of them.
  - **★ The finding that generalizes past this file: a section written as
    a TARGET and never re-labelled becomes indistinguishable from a
    section written as a RECORD.** §4's opening bullets were a Pass-0
    north star from 2026-07-23 (`ObjectId`, `Object::Bool`, `StreamData`,
    `Document::open`/`save`) and had drifted from every shipped name
    (`ObjId`, `Boolean`, no `StreamData`, `load`/`from_bytes`/`save_full`)
    for six weeks without anyone noticing — **because, unlike the dated
    `IMPLEMENTED` blocks below them, they carried no date to locate them
    in time.** §4.1(A) tabulates the correspondence. **Date and label
    every contract statement, or it will be read as current.**
  - **The sync is deliberately PARTIAL and says so** — §4.1(I) enumerates
    six things it did not audit, chief among them **§7 (CLI capabilities),
    which is presumed to lag by the same several Passes** and needs its
    own dispatch, and **`EditSession`'s other 57 `pub fn`s**, which §4 has
    never enumerated at all. **A partial sync honest about its edges is
    worth more than a complete-looking one that is not.**
  - **One open API-guidelines question raised, not answered** (CLAUDE.md
    rule 10): `CompositeEncoding` / `CompositeEncodeResult` /
    `NotInjective` are `pub` but **absent from `text_edit`'s re-export
    list** while their simple-font siblings are re-exported. Oversight or
    deliberate staging could not be determined from the source; flagged
    for an engineer decision rather than guessed.
  - **No `ROADMAP.md` Pass ID, standing rule, decision number or operator
    question was minted by this sync** — it is documentation catching up
    to shipped code, not new scope.

- **2026-08-05 — Decision 030: preserving the option of a future plugin
  system — build nothing, name what would foreclose it. Logged here for
  the first time; the record itself was filed earlier the same day and
  owed this entry (`docs/decisions/030-preserving-the-option-of-a-future-plugin-system.md`
  §9 item 1).** Filed by `autonomous-builder` (KenAgent), dispatched by
  `pdfce-engineer`, at the operator's request that decisions taken now
  not foreclose an *optional, bulky* plugin add-on later. **Outcome: no
  plugin system is built. No host, no ABI, no `[features]`, no new trait,
  no new `pub` item.** The record's value is almost entirely in what it
  found would DESTROY the option if built carelessly, not in anything it
  built. Three properties currently keep the option open, each verified
  in-tree and each true for reasons **unrelated** to plugins: (1)
  `EditSession::undo` is data-driven, not type-driven — `CommandKind` is
  popped and never inspected (`edit.rs:1304-1318`); (2) `CommandKind` is
  `#[non_exhaustive]`, though this is weaker than it looks — the
  `Command`/`ObjectWrite`/`Removal` types around it are private, and a
  plugin cannot construct an authored `Stream` at all (`stage_bytes` is
  private); (3) `pdfce-core` is deliberately WASM-portable, so a plugin
  host must live in a shell crate, never in core, on pain of nested WASM.
  **The inversion that changes the framing:** the request assumed the
  `EditSession` road was the only mutation path and it was closed. It
  is not the only one — `DirtySet` (`writer/mod.rs:214`) and
  `writer::save_full`/`save_incremental` are **fully `pub`** today, so an
  external crate can already mutate a `Document` with no undo, no
  disclosure (R145), and no save-mode obligation. **The greater risk is
  not foreclosure of a future plugin system; it is that the open,
  undisciplined road gets used by default because it is the only one
  available**, before anyone decides that on purpose. Recommended
  guardrail (not yet accepted or acted on): one standing rule naming
  `EditSession`-bypass as a named exception (redaction is the only
  sanctioned traveller, R35) plus a small grep gate
  (`tools/check-bypass-paths.sh`) on the five `DirtySet`/`writer::save_*`
  symbols — **not built by this record**, and its two candidate standing-
  rule numbers (§6.2(a), §4.5's core/shell plugin-host analogue to
  CLAUDE.md rule 2) remain **unminted pending the operator's acceptance**
  (see the continuation-89 entry below for the numbering consequence).
  Position on Cargo `[features]`: **not now** — would encode `[cfg]`
  boundaries that do not match where the real coupling lives (`edit.rs`'s
  8,880-line, 67-method convergence point), and would trade error
  detection for an unbuilt-combination risk, the same tradeoff the
  operator already declined for parallel sessions (decision 029). No
  licence question arises from this record; the one that would (an
  in-process dynamic-library plugin host vs. a WASM-sandboxed one) is
  named for later, not resolved now. **No body-section change required**
  — nothing in `pdfce-core`'s actual public surface changed; this is a
  preservation-of-optionality analysis, not an implemented decision.
  Reference: `docs/decisions/030-preserving-the-option-of-a-future-plugin-system.md`
  (full text, all measurements and the four foreclosure-risk scenarios in
  §5).

- **2026-08-05 (continuation 89) — Decision 031: where implicit commit
  stops — operator manipulation commits, pdfce inference stays reviewed,
  and `MeasureScale` is a named exception on a third, blast-radius axis.**
  Filed by `pdfce-engineer`'s classification, confirmed and extended (one
  addition) by `pdfce-ui-specialist`, recorded by `pdfce-librarian` at the
  engineer's explicit dispatch, alongside `ROADMAP.md` Pass 34.0's
  `GestureInterrupt::Commit` wiring. **Restates CLAUDE.md rule 4 as
  narrowed by decision 024 §4.4 against the full 2026-08-05 gesture
  inventory** (TextEdit, AddText, MeasureLinear, VectorEdit, ce-dimension
  position drag — all operator-authored, all commit implicitly on
  click-out; Reflow, MeasureCircular best-fit, derived-centerline —
  all pdfce-inferred, all keep an explicit Accept). **Adds one
  classification decision 024 did not need to make:** `MeasureScale`'s
  back-calculated scale is authored, not inferred, and would read as an
  ordinary implicit-commit case by rule 4's letter alone — recommended to
  keep its explicit confirm anyway on a **third axis, blast radius**: a
  scale commit changes the displayed value of every other ce dimension in
  the group at once, including ones off-screen, which is a materially
  different risk from any other single-object case-(a) commit
  (`main.rs:13413`'s own comment already names this). **This is a
  deviation from the operator's own literal instruction** ("this goes the
  same with all tools") and is surfaced rather than decided silently — see
  `ROADMAP.md` open operator question **(aw)**. Also decided: the new
  left-hand Tool Options dock (`ROADMAP.md` Pass 34.1) reuses the existing
  `DockBehavior` mechanism via a SEPARATE `Tree<LeftPanel>` instance rather
  than genericizing `DockBehavior` over a pane type or writing an
  independent sibling `Behavior` impl — `egui_tiles::Tree` instances are
  independent with no cross-tree pane migration wanted, so there is no
  cross-tree behavior to unify, and the existing `DockPanel`/`panel_body`
  dispatcher pattern already survived one prior library-adoption change
  verbatim (the continuation-57 entry above), which two small dispatchers
  extend rather than risk against `egui_tiles`'s own generic trait bounds.
  **[CORRECTED 2026-08-05 — pdfce-librarian.]** `Tree<LeftPanel>` names a
  type that does not exist and was never buildable as decision 031 §4
  specified it (`Tree<LeftPanel>` needs `impl Behavior<LeftPanel>`; the
  same `DockBehavior` cannot be "reused as-is" against a different pane
  type without genericizing it — decision 031 §4's own header already
  said the coherent thing: "one `DockPanel` enum, two `Tree` instances,
  one `DockBehavior`"). What shipped (Pass 34.1 slice 1, `e15f55b`): no
  `LeftPanel` type anywhere in `crates/`; `DockPanel` widened with two
  variants (`Pages`, `ToolOptions`); a second `Tree<DockPanel>`
  (`dock::default_left_tree()`, tree id `"pdfce-dock-left"`); the SAME
  `DockBehavior` genuinely reused, unchanged, across both trees. See
  decision 031 §7 for the full correction.
  **Numbering consequence for decision 030's still-unminted rule
  proposals:** decision 030 §9 items 3–4 remain pending the operator's
  acceptance, and are not the only contingent candidate on record — the
  continuation-88 documentation-discipline observation was floated as
  "R150's to claim" but never itself minted. **R150 was spent this same
  filing on the unrelated `GestureInterrupt::Commit` gesture-asymmetry
  process finding** (`ROADMAP.md` standing rules), filed first, so R151
  is next free for whichever of the three contingent candidates is
  accepted or promoted first — not pre-allocated among them here. No
  `pdfce-core`/`pdfce-render`/`pdfce-gui` body-section text is corrected
  by this entry beyond what Pass 34.x's own shipping will do — this is a
  classification governing acceptance criteria, not yet an implemented
  change. Reference: `docs/decisions/031-implicit-commit-boundary-and-the-measurescale-blast-radius-exception.md`.

- **2026-08-05 (same-day continuation 92) — Decision 031, BUILD
  CONFIRMATION: Pass 34.1 (slice 1 of 2) SHIPPED (`e15f55b`), the left
  dock mechanism decided above is now real, not yet fully populated.**
  A second, independent `egui_tiles::Tree<LeftPanel>` mounts on the left
  with tabs `Pages | Tool Options`, exactly the SEPARATE-tree-not-
  genericized-`DockBehavior` shape this decision recorded ahead of the
  build (continuation 89, above) — the existing right-hand dock's
  `DockPanel`/`panel_body` dispatcher pattern is reused verbatim for the
  new tree, confirming R80's one-dispatcher rule holds across two
  independent `egui_tiles::Tree` instances, not just within one.
  **[CORRECTED 2026-08-05 — pdfce-librarian.]** No `LeftPanel` type was
  built; `grep -rn "LeftPanel" crates/` returns nothing. The "second
  `egui_tiles::Tree<LeftPanel>`" above is actually a second
  `Tree<DockPanel>` (`dock::default_left_tree()`) — `DockPanel` itself
  was widened with the `Pages`/`ToolOptions` variants rather than a new
  pane enum being introduced, and the type-level per-tree guarantee a
  dedicated enum would have given is instead a test,
  `no_panel_is_mounted_in_both_docks`, sweeping `DockPanel::ALL` against
  both default trees. Decision 031 §4's `LeftPanel` mechanism was not
  implementable as specified; see decision 031 §7. The page-thumbnail
  rail is now `DockPanel::Pages`; `DockPanel::ToolOptions`
  is new and currently surfaces the armed tool's identity, its Pass-34.0
  commit/discard contract, and refusal/disclosure text — **not yet** the
  property-bar controls (font/size/colour/spacing), which still draw in
  the pre-existing floating `egui::Area` strips. That relocation is
  slice 2, unshipped, and is the literal remainder of the operator's
  original ask; see `ROADMAP.md`'s annotated Pass 34.1 Next-up entry for
  the itemized gap. **No further body-section change required** — as
  with the continuation-58 (decision 017 Amendment A) build-confirmation
  entry, pdfce's GUI-dock architecture is documented in this decision log
  rather than in a dedicated numbered `ARCHITECTURE.md` section, so this
  entry IS the body-section update for this decision. `cargo tree -p
  pdfce-core` / `-p pdfce-render` re-verified clean (no GUI deps pulled
  in by the second `Tree`). Full record: `ROADMAP.md`'s "Pass 34.1
  (slice 1 of 2)" Shipped entry.

- **2026-08-05 (continuation 94) — Pass 34.1 slices 2–3 SHIPPED
  (`fae916d`, `13f3c0b`); decision 024 §3.3 Family A SUPERSEDED by
  decision 031 / Pass 34.1, on the operator's own later instruction.**
  Filed by `pdfce-librarian` at the engineer's dispatch.
  - **Build confirmation.** `DockPanel::ToolOptions` (`crates/pdfce-gui/
    src/dock.rs`) now hosts all three tools' full property surfaces:
    Edit Text's (slice 2, `fae916d`, ~500 lines of plumbing — nine new
    booleans/queue fields on its edit state, the largest of the three
    migrations), and Add Text's and Measure's (slice 3, `13f3c0b` — Add
    Text needed zero new plumbing, every control already wrote a
    `prop_*` field or queued a placement that survives to the canvas
    pass; Measure needed two booleans, `queued_close_tool` and
    `queued_open_groups`, the same one-frame set-in-dock/drain-in-canvas
    contract Pass 34.0 established). The floating `egui::Area` property
    bars for all three tools are deleted, not retained as fallback.
    `StripCorner::TopLeft` is **removed** (not `#[allow(dead_code)]`d) —
    nothing anchors to the canvas top-left any more; `BottomLeft`
    remains until the status/commit strips move (slice 4, not yet
    built — this is a stated, not silent, remainder, per R150). A
    generalised `migrated_options_tool` heading rule replaces the
    text-edit-specific one from slice 2, since every migrated bar now
    carries its own `*_propbar_title()` and a stacked pane heading would
    duplicate it.
  - **§3's own body-section note is now superseded in one respect: this
    IS a `pdfce-gui`-only change** (unchanged crate boundary — `cargo
    tree -p pdfce-core` / `-p pdfce-render` re-verified clean, no GUI
    dependency introduced), so no §3/§4 edit beyond this entry is
    required, consistent with the continuation-89/92/93 disposition for
    this same dock architecture.
  - **Decision 024 §3.3 Family A (TOOL contextual ribbon tabs, keyed on
    `doc.active_tool`) is SUPERSEDED for tool-options content, on the
    operator's own later, more specific instruction** — not an engineer
    or specialist judgment call. §3.3's original text is left unedited
    (append-only); the supersession is recorded as a new §11 in
    `docs/decisions/024-ribbon-command-surface-and-the-accept-reject-problem.md`,
    filed at `pdfce-ui-specialist`'s own recommendation
    (`docs/ui_specs/ribbon-groupings-and-customization-architecture.md`
    §2). **Family B (SELECTION tabs, keyed on `TargetId` kind) is
    UNAFFECTED** — still correctly blocked on Passes 22.0/23.2, exactly
    as decision 024 scoped it. **Consequence for any future Pass 24.2
    build:** the Measure/Edit/Add-Text/Edit-Objects contextual ribbon
    tabs, if built, carry *invocation* only (arm the tool, manage
    ce-dimension groups) — the armed tool's live controls stay in
    `DockPanel::ToolOptions` and must not be duplicated onto a ribbon
    tab.
  - **Open operator questions (ax), (ay), (aw) CLOSED this filing.**
    (ax) — DISSOLVED, not amended: the operator answered the
    organising question directly ("make the groupings make sense...
    if it makes more organizational sense to have them a different
    way, do so") rather than granting the Acrobat-GUI-audit amendment
    to `CLAUDE.md` rule 12; rule 12 stands untouched, and
    `pdfce-acrobat-librarian` was not dispatched. (ay) — CLOSED:
    "organizational sense governs; resemblance either way is not the
    goal" is a complete answer, naming no specific closeness on
    purpose. (aw) — CONFIRMED: the operator ratified keeping
    `MeasureScale`'s explicit Accept/Reject
    (*"good choice. we need to enter a value for it anyway"*),
    appended to decision 031 §3 as a second, independent justification
    (a typed value is already required, so the commit point is free)
    alongside the blast-radius argument decision 031 already recorded.
    Also on record for the future: ribbon groupings are meant to become
    operator-customizable ("like SolidWorks and MS Office") — not
    scoped now, but it is why the customization architecture in
    `docs/ui_specs/ribbon-groupings-and-customization-architecture.md`
    §5 (a static `RibbonCommandId`/`RibbonCommand`/`RibbonGroupDefault`
    registry keyed on stable identity, deliberately named
    `RibbonGroupId` rather than `GroupId` to avoid colliding with
    `pdfce_core::dimension::GroupId`) is being laid down now, without
    building reorder/hide/reset UI or persistence yet.
  - **Process finding, filed as standing rule R155** (`ROADMAP.md`
    Standing rules): the engineer dispatched a fresh ribbon-grouping
    design task without first checking that decision 024 already
    contained a complete, decided taxonomy plus standing rules
    R121–R125 covering the same territory. The specialist caught this
    on its own initiative and audited the existing record instead of
    re-deriving a competing one, so nothing was lost — but the near-miss
    is the second instance this session of the same recurring shape (a
    thing already exists, and nothing pointed at it before the dispatch
    went out; the first was decision 031 §4's `LeftPanel`/`DockBehavior`
    contradiction, R154). Distinct from R151–R154: those audit an
    artifact against current reality *after* something has shipped or
    been written; R155 is upstream of all four — a dispatch-time check,
    before a design brief is written, against the existing decision log.
    Full text and numbering note: `ROADMAP.md` Standing rules.
  - No new decision record minted (031's ratification and 024's §11 are
    both appended corrections to existing records, not new ones);
    decision-record ceiling stays **031** (**032** next free). No new
    Pass family minted (Pass 34.1's slice count is not a family count;
    ceiling stays **37 next free**). Full record: `ROADMAP.md`'s new
    Pass 34.1 Shipped entry and `SESSION_LOG.md` continuation 94.
- **2026-08-06 (continuation 105)** — **The left dock's shape is decided
  by *watched vs entered*, and `PaneSubject` now means "workflow" and
  nothing else.** Shipped as **Pass 38.2** (`aa48167`); design record
  `docs/ui_specs/shell-redesign.md` (723 lines); rule minted as
  **R157**.
  - **What changed structurally.** `DockPanel::ToolOptions` →
    `ArmedTool`; `Properties` and a new `Activities` promoted to peers;
    `Pages` removed from the tab pairing. The left dock is a vertical
    `egui_tiles::Container::Linear` of **four always-visible
    compartments** with **no tab container anywhere** (shares
    0.7/0.7/1.3/1.3, tuned against the running app, draggable —
    defaults, not constraints). `PaneSubject` narrows to
    `{BatchTools, Redact, Forms}` behind an in-panel segmented control.
    The `dock::activate`-on-tool-arm **auto-raise is deleted**.
  - **Why this is a decision and not just a layout change.**
    `PaneSubject::ActiveTool` and `::Properties` were the exact *"select
    above, edit below"* relationship **decision 017 §A.3** built the
    original right-dock vertical split for. **Pass 24.3 retired that
    split correctly** — its specific pairing had gone stale — but the
    underlying need **recurred one level down**, inside Tool Options,
    where nothing was looking for it. The decision is to state the rule
    at a level that survives the next such retirement: **selection state
    is watched and gets a permanent compartment; workflows are entered
    and share one.** §12's earlier decision-017 and Pass-24.3 entries
    stand; this one supersedes neither, it generalises both.
  - **No decision record minted.** The 723-line spec **is** the record,
    and a `docs/decisions/032-*.md` would duplicate it. **Decision-record
    ceiling stays 031 (032 next free).**
  - **Origin, recorded because the derivation is the compliance-relevant
    part.** The operator asked for resemblance to **PDF-XChange Editor**.
    The engineer **declined to copy it** (**R123**, trade dress) and
    converted the request into **five neutral UI properties**, which the
    operator confirmed; `pdfce-ui-specialist` designed against the
    properties under an explicit instruction not to examine PDF-XChange.
    That conversion is filed as a **proposed R123 amendment** — proposed,
    not minted.
  - **A refusal, stated rather than silently avoided.** Property 4's
    *maximal* reading — tool options as a fly-out hugging the selection —
    is **refused**: it would structurally reintroduce the floating
    `egui::Area` behind the operator's *"separate accept/reject box
    somewhere on the screen"* complaint (§ CLAUDE.md rule 4's 2026-08-05
    narrowing; **R81**). Open operator question **(ba)**.
- **2026-08-06 (continuation 105, second entry)** — **The canvas claims
  the arrow keys while a text caret is live, and Tab/Escape deliberately
  stay unclaimed.** `egui-0.35.0/src/memory/mod.rs`'s `Focus::begin_pass`
  routes a **bare** arrow key to `FocusDirection` **unless the focused
  widget's `EventFilter` matches it**; pdfce's canvas declared none, so
  `ArrowLeft` moved focus into the left dock and the caret block stopped
  running entirely. Fixed in `7d368e6` with `set_focus_lock_filter`
  claiming `horizontal_arrows` + `vertical_arrows`.
  **The two deliberate non-claims are the contract:** `tab: false`, so a
  keyboard-only operator is never trapped in the canvas; `escape: false`,
  because Escape is pdfce's **pop-one-level-rung / cancel-gesture** verb
  (decision 025's ladder, **R130–R132**) and claiming it would break
  both. **Consequence to know, read from egui's source:** with
  `escape: false`, a bare Escape also sets egui's `focused_widget = None`
  — pdfce's rung pop and egui's focus surrender happen on the same press.
  Escalated to `D:\dev\rag\egui\`.
- **2026-08-06 (continuation 105, third entry)** — **Retained
  position that is addressed by INDEX is re-validated for KIND and
  addressability, never for identity.** Shipped as **Pass 26.2**
  (`50ab8ec`). `EnteredObject` is positional indices only, so
  `prune_canvas_selection`'s re-validation **cannot** prove the slot
  holds the same object; it proves the slot holds **the same kind with
  enough structure to address**, and truncates **one rung at a time,
  deepest first**, when it does not. The stronger claim is refused **in
  the code**, not merely omitted. The residual risk (a same-kind object
  arriving at the same index) is **accepted knowingly**, licensed by an
  asymmetry rather than by the strength of the check: **the outline is
  drawn**, so a wrong retention is visible immediately, while a lost
  place is silently expensive. The rule (`truncate_entered`, three
  numbers) is **split from the lookup** (`revalidate_entered`) so the
  decidable half is testable without constructing a decomposed page.
  Filed as a standing-rule **proposal**, not minted.
- **2026-08-06 (continuation 107, first entry)** — **When widening a
  singular piece of state to a collection, COUNT THE READERS FIRST, and
  let the count pick the shape.** Two Passes the same day faced the same
  question at two rungs of the same ladder and answered it **differently
  on the evidence, not by template**.
  **Pass 39.0** (`d92c1b3`, multiple open documents): **76 call sites**
  destructure `Status::Open(doc)`. A `Vec<OpenDoc>` + `active: usize`
  would make each of those 76 a chance to silently address the wrong
  document — **a defect class with NO on-screen symptom**, which is the
  worst kind this application can carry. Chosen instead: a **parked list
  beside `status`**, so `Status::Open` keeps meaning *"the document in
  front of the operator"* and **all 76 sites stay correct BY
  CONSTRUCTION rather than by review.** The bad class becomes
  *unrepresentable*, not merely *avoided*. Costs accepted and stated: a
  switch is a **move** not an index change, and the list is in
  most-recently-parked order. **It was affordable only because
  per-document state was ALREADY isolated on `OpenDoc`** — the property
  that usually makes multi-document a rewrite.
  **Pass 41.0** (`66cee16`, node multi-select): **13** genuine
  `EnteredObject.node` readers (27 grep hits, most lookalikes). 13 is
  reviewable where 76 was not — yet the same answer was reached, by two
  *different* arguments: (a) `EnteredObject` is **`Copy` and passed by
  value everywhere**, so a `BTreeSet` field would ripple through every
  such site for reasons unrelated to node selection; and (b) **the
  codebase already answers the same question one rung up** —
  `canvas_selection: BTreeSet<TargetId>` sits **beside** `entered`, not
  inside it. So `selected_nodes: BTreeSet<usize>` **beside** it, with
  `EnteredObject::node` retained as the **primary** anchor, leaving all
  13 readers untouched.
  **The generalisable rules, both stated because they are cheap and were
  each load-bearing once today:** *count the readers before choosing the
  shape — the count is what makes the argument decidable rather than
  aesthetic*; and *look one rung up before designing the rung below — a
  codebase that has already answered this question has also already paid
  for the answer.*
- **2026-08-06 (continuation 107, second entry)** — **Selection and
  editing are MODELESS; creation verbs keep explicit arming.** Shipped as
  **Pass 40.0** (`b6fcf27`), resolving the operator's *"we should be able
  to activate any and all of the edit options at once."* **The boundary
  is not caution, it is decidability:** a verb that acts on **what
  exists** can take its meaning from what is under the pointer; a verb
  that **creates where nothing exists** cannot, because *"place new text
  here"* and *"start a marquee"* are the same gesture over the same empty
  paper. So: click-select, double-click-descend, drag-to-move and
  node-drag are modeless; **Add Text and the measure tools stay armed.**
  Implemented by the no-tool branch **calling `run_vector_edit_tool`
  itself** with a `Modeless` flag — *the same function*, not a thinner
  copy, which is what stops the two drifting (**R92**). They **had**
  already drifted: `apply_click_depth` was once wired into only one of
  the two paths.
  **The one deliberate behavioural difference:** modeless, a drag must
  start **on the selection's BOUNDS** (grown by the selection tolerance)
  or it stays a marquee — otherwise selecting an object and
  rubber-banding elsewhere would **silently drag the selection across the
  page**. Bounds, not anchors: a press inside a large filled shape is
  plainly *on it* while far from every anchor.
  **Deliberately NOT included, and it is a rule-4 question rather than a
  wiring gap:** clicking a text object with no tool armed does **not**
  begin text editing — entering `TextEdit` sets `active_tool`, runs a
  full page-text re-extraction and tears down other tools' state, i.e.
  **a mode change triggered by a click**. Filed as open operator question
  **(bc)**.
- **2026-08-06 (continuation 107, third entry)** — **A confirmation
  belongs in the EXISTING pending-gate, never as a new modal — and
  "save-and-close" must not close on a FAILED save.** Shipped as **Pass
  39.1** (`1c48cbd`). `pending_close` joins `pending_save`,
  `pending_copy` and `pending_redaction_apply` under the **single guard
  at the top of `apply`** that refuses every unrelated action while any
  question is outstanding. That guard exists because **two
  centre-anchored `egui::Window`s render over each other with only the
  later one clickable** — a defect this project has already had once
  (RAG:
  `D:\dev\rag\egui\egui_0.35_two_center_anchored_windows_pending_state_gate_dispatcher.md`).
  **A fourth ad-hoc modal would sit outside the very mechanism that
  prevents it**, so joining the family is a *correctness* decision, not a
  tidiness one. Proven by a test that drives `Action::SwitchDocument`
  while the close question is outstanding and asserts refusal.
  **Three sub-rules, each with its own argument:** (1) **three real
  answers**, because offering two forces one of the three things an
  operator wants to be expressed by *dismissing* a dialog; (2)
  **save-and-close closes ONLY if the save succeeded** — a cancelled
  dialog or refused write otherwise discards exactly the work just asked
  to be kept; (3) **a clean document closes with no prompt**, because a
  confirmation whose answer is always *"yes, obviously"* teaches people
  to dismiss prompts unread, which is what makes the one that matters
  dangerous.
- **2026-08-06 (continuation 107, fourth entry)** — **UI density comes
  from the GAP between controls, never from the controls.** Shipped as
  **Pass 38.1** (`9de335f`); written into `UI_PREFERENCES.md` **§11**
  (**repo root**, not `docs/`). Measured first: egui 0.35's
  `item_spacing.y = 3.0` and `interact_size.y = 18.0`
  (`egui-0.35.0/src/style.rs:1449,1454`) give a 21 px row pitch, and the
  `/Info` grid was quietly running **25 px** against everything else's 21
  — so the airiness was **the gap stacked on a widget height already near
  its floor**, plus one grid on a second rhythm. Fixed with one constant
  (`DENSE_ROW_SPACING_Y = 2.0`) applied **once**, in `panel_body`, the
  single chokepoint every dock pane renders through — so a future panel
  inherits it without knowing it exists. **Scoped to the dock, NOT
  `configure_context`**: the same 3.0 does a different job in a toolbar
  row, where controls must be separable at a glance.
  **Three refusals are the durable half, and are recorded as a numbered
  convention so a later pass cannot quietly spend them:**
  **`interact_size.y` stays 18.0** — shrinking the click target is the
  biggest density win available and it trades pointer accuracy, which
  costs most for the operators least able to spare it; **no text gets
  smaller**; **no explanatory line is deleted to save a row**, because
  the canvas raster is screen-reader-illegible and those text widgets
  **are** the accessibility surface. Verified as **content-neutral by
  measurement**: 285 label rows before and after, 11 px reclaimed.
- **2026-08-06 (continuation 108, first entry)** — **"ENABLED" and
  "HANDLES THIS CLICK" are two different questions, and conflating them
  is what made every edit toggle a radio button.** Shipped as **Pass
  42.0** (`871c868`). `active_tool: Option<CanvasTool>` could hold
  **exactly one** tool, so arming `Obj` disarmed `Edit Text` — which from
  outside is **indistinguishable from a tool switching itself off**, and
  that is what the operator reported. **The state is now
  `enabled_tools: BTreeSet<CanvasTool>`, and `active_tool()` becomes a
  METHOD** answering the second question: *which enabled tool owns this
  click*. **~20 existing readers stay correct untouched**, because they
  all wanted the second question all along — the same
  leave-callers-correct-by-construction move Pass 39.0 made at 76 call
  sites the same day, reached here by a different route.
  **The precedence ladder is ordered MOST-SPECIFIC-CLAIM FIRST, and its
  order is a property of the ladder, never of enable history:**
  `TextEdit` (acts only on text under the pointer) → `AddText` (claims a
  click **anywhere**, so it must sit below the tool that claims only
  text, or it swallows every caret placement) → measure (a deliberate
  multi-click gesture a stray selection must not interrupt) →
  `VectorEdit` (the most general claim, therefore the floor). **A test
  pins enable-order independence**, because the alternative is *the same
  click doing different things depending on history the operator cannot
  see*. **The ladder is invisible, so it is DISCLOSED** — the Tool pane
  names which tool has the canvas and which others are on.
  **One deliberate asymmetry:** the three measure tools stay exclusive
  **among themselves** — they share one state struct and dispatch on a
  single value, so two-on is a **state with no meaning**, not extra
  capability. A test pins the asymmetry so it cannot be mistaken for a
  leftover of the radio-button model.
  **The master switch is gated at ONE chokepoint (`tool_enabled()`) as a
  correctness decision, not a tidiness one:** the failure mode a
  per-dispatch-site gate invites is **one tool still editing in review
  mode**, which is precisely what review mode exists to make impossible.
  It covers the non-canvas authoring surfaces too — ce-dimension drag,
  form filling, redaction marking — each **disabled-and-explained rather
  than hidden** (R83), so a document still READS while nothing can change
  it. Turning editing off **keeps** the tool set.
  **This ANSWERS open operator question (bc)** by rejecting its framing:
  the rule-4 concern (pdfce silently changing the operator's mode) is
  dissolved by a **shape change**, not by a confirmation step — arming
  stays a deliberate act, so nothing changes mode on a click.
- **2026-08-06 (continuation 108, second entry)** — **A rule stated as a
  DISTINCTION survives a mis-sorted instance; a rule stated as an
  INVENTORY would have had to be deleted.** Shipped as **Pass 43.0**
  (`37a49e6`), which **partially reverses Pass 38.2** (`aa48167`) of the
  same morning. **R157** (*selection state is WATCHED, workflows are
  ENTERED; watched things get a compartment, entered things share one*)
  classified **`Properties` as watched**. **It is not** — Properties is
  consulted **in bursts, deliberately, when there is something to
  change**, which is the shape of an *entered* workflow. So
  `DockPanel::Properties` and `::Activities` are **retired**; both are
  activated from the **ribbon** and render in the **Tool** compartment,
  which becomes the universal options surface (named subject label,
  "Back to tools" exit). **The left dock is Pages + Tool.**
  **R157 SURVIVES the reversal, and that is the transferable finding.**
  Because the rule names a *distinction* rather than an inventory of
  panes, **the rule itself decided the correction** — Properties moved
  because entered things multiplex. Had 38.2 written *"Properties gets a
  compartment"*, the rule would have been deleted three commits after
  being minted. **Both corollaries held throughout:** no tab bar (a tab
  bar is the mechanism that hides things), and an always-visible
  compartment is never auto-raised.
  **The change was ROUTING, not new controls** — every ribbon entry point
  already existed (Properties/File, Forms/Edit, Comments/Review,
  Batch+Redact/Tools), which is why the reversal was cheap. **The
  `Activities` segmented control is DELETED**: with each activity on the
  ribbon, a second switch beside it is **two controls for one choice**.
  **Recorded in Pass 24.3's convention** — the specific pairing went
  stale, the underlying need did not — rather than by deleting 38.2's
  reasoning, which stands untouched with a dated pointer.
- **2026-08-06 (continuation 108, third entry)** — **A tree renders the
  nesting the MODEL has, and refuses the nesting it does not.** Shipped
  as **Pass 43.0** (`37a49e6`). The object sidebar nests **object →
  subpath → node** — the level ladder the canvas already walks (R130) —
  and the operator confirmed that ladder verbatim. **Tree and canvas
  agree BY CONSTRUCTION rather than by care:** a row's
  `(object, subpath, node)` triple **IS an `EnteredObject`**, so clicking
  a row *sets the canvas level*; there is no second representation to
  keep in sync.
  **Marked-content / OCG grouping was REFUSED, and the refusal was
  RE-VERIFIED rather than inherited.** The old flat panel's doc comment
  claimed inventing that grouping *"would be a lie about the document's
  structure"*; that claim was checked against the code (R143's shape) —
  **no `--tree`/`--level` on `object-list`, no `ContentPath` in
  `decompose.rs`; Pass 23.2's core half is planned, not built** — and the
  stale half of the comment (which described a *flat* panel) was
  corrected rather than deleted. **A doc comment that is still right
  about the refusal and wrong about the shipped widget is exactly the
  R93 failure**, caught here by reading the code the comment was about.
  **Cost held flat:** a flattened per-frame display list keeps `show_rows`
  virtualization, so a fully-collapsed tree costs exactly the object
  count. **And a retired helper's tests MOVED rather than dying with
  it** — `display_row_for_target`'s front-most-first assertions now sit
  on `build_object_tree_rows`, which owns that ordering.

- **2026-08-07 (first entry)** — **A certified document's signature
  freezes STRUCTURE, not USE: fill-shaped operations take the `/P`-aware
  gate, structure-shaped operations take the strict one.** Established by
  `8e799e9` (form-field creation, core + CLI — see `ROADMAP.md`'s
  ⚠ IDENTITY UNRESOLVED entry at the head of *Shipped*; **no Pass ID
  assigned yet**).
  **The distinction, and why it is not a matter of taste.** `EditSession`
  now has two certification gates and they are not interchangeable.
  `fill_refusal` is `/P`-aware and **permits** editing a certified
  document at `/P >= 2` — correctly, because §12.8.2.2's `/DocMDP`
  transform parameter **exists to say "filling is allowed"**; a form
  certified for completion that refused to be completed would be useless.
  `check_certification` is strict and refuses at **any** `/DocMDP` tier
  and on any `/FieldMDP`. `add_text_field` takes the **strict** one,
  joining `add_markup` and `flatten_fields`.
  **The generalisation, which is the reason this is a decision-log entry
  and not a code comment:** *"does this change what the document SAYS, or
  what the FORM IS?"* is the only question to ask when choosing a gate.
  Setting `/V` on an existing field is a use of the form the certifier
  anticipated and priced in. **Adding a field changes the set of things
  the document can say** — which is precisely what a certification
  signature is for. **Every remaining authoring verb in the decision-020
  family (checkbox/radio/choice creation, field deletion, rename, move,
  resize, tab-order rewrite) is structure-shaped and takes the strict
  gate**; the only 20.x verbs that could take the `/P`-aware one are
  value-setting verbs, and those already exist.
  **Where this could go wrong later, stated now:** the two gates are
  distinguished by which function is called, not by a type. Nothing stops
  a future authoring verb calling `fill_refusal` because it was written by
  copying a fill path — the same **R92**-shaped hazard (one behaviour,
  two implementations) that the shared appearance builder avoids by
  construction. **If a third authoring verb picks the wrong gate, promote
  the choice into the type system rather than fixing the call site.**
  > **★ FORWARD POINTER, 2026-08-07 (tenth filing) — THE HAZARD
  > MATERIALISED IN A DIFFERENT SHAPE THAN THIS PARAGRAPH PREDICTED, and
  > the difference is instructive.** No verb called the wrong gate. **The
  > wrong gate was the only one a SHELL COULD ASK**: `fill_refusal` was
  > `pub`, the strict gate `deletion_preflight` uses had no public query,
  > so the GUI could render a delete control it could not gate correctly
  > — and on a certified fillable form at **`/P 2`**, the ordinary case,
  > the two answers differ. Closed by `EditSession::deletion_refusal()`
  > (`fc51786`). **See the thirteenth entry this day** for the
  > generalisation (*every gate in a divergent set needs a public query*)
  > and for the `/P 1`-only fixture-corpus finding.

- **2026-08-07 (second entry)** — **A form field is THREE writes and they
  are ONE undoable command.** Also from `8e799e9`.
  §12.7.2 requires the field in `/AcroForm /Fields`; §12.5.6.19 requires a
  widget annotation in the page's `/Annots` for it to appear on a page at
  all; and pdfce's own **R43** requires a baked `/AP`, because a widget
  with `/MK` and no `/AP` is this project's canonical named-not-painted
  case. **Registered-but-not-annotated is invisible. Annotated-but-not-
  registered is not a form field.** So all three writes go in one
  `Command`: **an undo must not be able to reach either half-state.**
  **This is the general shape for every authoring verb whose product is
  defined by entries in two or more dictionaries** — the ce-dimension
  authoring path (annotation + `/AP` + `/PieceInfo` sidecar, deleted
  together at Pass 25.6) is the same rule already in force elsewhere, and
  the two should be read as one convention rather than as two local
  choices. **The test for it is not "did the write succeed" but "what does
  ONE Ctrl+Z produce"** — a command boundary in the wrong place yields a
  document that is valid PDF and is not a form, which no round-trip or
  parse test would catch.
  **`/AcroForm` is created with `/DR /Font /Helv` when the document has
  none, and that is a correctness requirement, not a nicety:** §12.7.3.3
  requires the `/DA`'s font to resolve in `/DR`. Without it, a viewer
  regenerating the appearance from `/DA` cannot — and pdfce, which bakes
  its own `/AP`, **would never notice**. That is the failure mode this
  entry most wants recorded: **a document that works in the tool that
  wrote it and nowhere else, invisible to that tool's own tests.**

- **2026-08-07 (third entry)** — **When a decision record REJECTS a write
  model, a refusal that makes the rejected shape UNREACHABLE is a
  legitimate cap — but only when the shape becomes IMPOSSIBLE, not merely
  unlikely.** Established by `bca60c9` (Pass 20.2 + Pass 20.3, check-box
  and choice-field creation — see `ROADMAP.md`'s *Shipped* entry).
  **The situation, because the rule only makes sense against it.**
  Decision 020 §3.3.1 rejected the flat-append authoring model — outcome
  **O1** — in unusually strong terms: *"Even one slice of that authors
  documents that cannot be un-authored."* Pass 20.1 shipped that model
  anyway, and it was **emitting O1 in fact**, not in theory: two
  same-named `add-text-field`/`add-check-box` calls produced two
  top-level fields sharing one fully-qualified name, which §12.7.3.2
  makes the field's **identity**. Verified by reproduction before any
  code changed.
  **The choice was pivot or cap.** Pivoting meant stopping to build the
  write-side field-path resolver decision 020 §6 F1 specifies. Capping
  meant keeping the flat model and **refusing** every same-name add, in
  one shared preflight, for all three authoring verbs.
  **Capping was chosen, and the test that justifies it is the one worth
  carrying forward: with the refusal in place, O1's rejection ground goes
  to ZERO.** Flat-append **plus a total refusal** cannot emit a
  duplicate-identity document *at all* — not rarely, not usually-not.
  What remains is a **missing capability**, and a missing capability and a
  deferred corruption are categorically different objects: the first
  produces an error message, the second produces files in someone's
  archive that cannot be repaired, because nothing records which of two
  identically-named fields the operator meant.
  **The two conditions this rule carries, so it is not read as a licence
  to defer any hard branch:**
  1. **The refusal must be TOTAL and must sit in ONE place.** Here it is a
     single `field_authoring_preflight` called by all three verbs. A
     per-verb refusal would be a claim about three code paths rather than
     a property of the subsystem, and the next verb would forget.
  2. **Nothing built may become waste.** The keyed `/AP /N` sub-dictionary,
     the button appearance generator, `/Opt` encoding and the flag mapping
     are all required whatever shape the resolver eventually takes,
     because none of them sits on the resolver's side of the design. If
     capping had meant building throwaway structure, pivoting would have
     been correct.
  **The corollary that makes this safe to reuse:** a cap converts a
  correctness debt into a **capability** debt, and capability debt must
  then be tracked as owed work with its blocker named. Here the resolver
  is owed, **radio groups are BLOCKED on it** (decision 020 F2 requires
  them built from the merge primitive), and the engineer's response to
  this entry's own filing was to make F0+F1 the priority. **A cap that is
  not followed by that re-prioritisation is just a nicer-looking
  deferral.**
  **A second, smaller rule from the same commit, filed here because it
  generalises past forms:** `ChoiceOptionDuplicate` refuses a repeated
  `/Opt` export value **even though §12.7.4.4 permits it**, because
  pdfce's own fill verb resolves a requested value to the **first** match
  — so a duplicate authors an option the operator can see and can never
  choose. **An authoring verb must be checked against what the project's
  OWN consuming paths can address, not only against what the spec
  permits.** Rule 1 (spec fidelity) says what is legal; this says what is
  reachable, and the two can disagree in the direction of pdfce being
  stricter.

- **2026-08-07 (fourth entry) — the write-side field-path resolver
  ships; the O1 rejection ground the previous cap depended on is now
  CLOSED, not merely capped.** Established by `a3d885b` (F0) + `f809857`
  (F1 completion) — see `ROADMAP.md`'s `Pass 20.0 + Pass 20.1
  (completion)` *Shipped* entry.
  **The third 2026-08-07 entry above accepted a cap** — a total refusal
  making decision 020's rejected O1 outcome unreachable, in place of
  building the resolver — on the explicit condition that the resolver
  remained owed and that capping not be mistaken for solving. **This
  entry records that the resolver is now built**, and the safety property
  the cap protected is now delivered by the mechanism decision 020
  originally specified rather than by a refusal standing in for it:
  `forms_author::resolve_field_path` classifies every add against the
  live graph as Vacant → CREATE, same-type Terminal → MERGE (with Shape
  A→B promotion), different-type Terminal → refuse
  (`FieldTypeCollision`), or Grouping → refuse (`NameIsGroupingNode`),
  **before** any write happens, across all three authoring verbs.
  **What changed is the mechanism, not the guarantee.** Before this
  filing, a same-name/same-type add was *reachable and stopped by a
  guard* (Pass 20.2/20.3's shared preflight). After it, the duplicate-
  identity document decision 020 rejected is *unreachable by
  construction* — there is no code path left that could emit it, because
  every write now resolves the name first. This is the distinction a
  future reader should draw between "capped" and "closed": a cap makes a
  bad outcome unreachable from where it currently sits; closing the gap
  makes the surrounding mechanism incapable of producing it regardless of
  how the call site changes later.
  **This also converts §12.7.3.2's merge semantics — one field, one `/V`,
  shared across widgets/pages — from a spec fact pdfce refused to
  implement into a spec fact pdfce implements.** A radio group and a
  page-number field repeated on every page are the two cases §12.7.3.2
  names this mechanism for; both are now reachable through the same
  primitive, though radio itself is not yet built on top of it (F2,
  unblocked, unbuilt).
  **The corollary from the third entry — "a cap not followed by
  re-prioritisation is just a nicer-looking deferral" — is discharged by
  this entry's own existence.** The cap was followed by exactly the
  re-prioritisation it was conditioned on.

- **2026-08-07 (fifth entry) — two defects found building the resolver
  are RESTATEMENTS of already-filed rules, not new ones, and both
  restatements are worth keeping because they occurred in code written
  AFTER the rule was known.** Also from `a3d885b` + `f809857`.
  **(1) N whole-object writes to one object in one command do not
  compose — the SECOND occurrence of the 2026-08-07 (second entry) rule
  above, this time inside the resolver itself.** Shape A→B promotion
  retargets a page's `/Annots` to the promoted widget, then appends the
  new widget: two whole-page-dict writes in one command, each computed
  from pre-command state, so the append silently discarded the retarget.
  **This is not a new failure mode** — it is the identical shape the R85
  byte-level oracle caught in `flatten_fields` (F0, same filing), applied
  to a different dictionary. **Worth recording precisely because of the
  timing**: the rule was already filed in this same document before the
  code that violates it a second time was written, which means "know the
  rule" was not sufficient to avoid it — the rule needs a mechanical
  check (a lint, a code-review question, or a type that forces a single
  write), not only documentation. Fixed by folding retarget and append
  into one write. Escalated as a Rust/state-management finding to
  `D:\dev\rag\rust\n_sequential_whole_object_writes_in_one_command_do_not_compose.md`.
  **(2) A model-level test assertion is blind to a defect the model
  itself normalizes away — found in F0, not F1, but filed here alongside
  its sibling because both are "the test suite agreed with the bug"
  findings from the same filing.** `flatten_fields` left `/AcroForm
  /Fields` naming two deleted objects; every existing forms test asserts
  through `parse_acroform`, which resolves each `/Fields` entry and
  silently drops what it cannot — so the test suite's own model-level
  view of the document was a smaller, coincidentally-correct-looking form
  with no evidence the file itself was wrong. **The generalisation:** any
  test that asserts through a parser/projection which tolerates malformed
  input inherits that tolerance as a blind spot; a defect that produces
  exactly the input class the parser was built to shrug off is invisible
  to every test built on top of it, however many such tests exist. The
  regression test for this defect asserts on **bytes**, deliberately
  bypassing the tolerant projection. Escalated to
  `D:\dev\rag\rust\model_level_assertion_blind_to_normalized_away_defect.md`.
  **Neither finding changes any standing rule number** — both are
  read as sharpened restatements of the existing R85 oracle discipline
  and the existing "second entry" command-boundary rule, not as new
  architectural decisions in their own right.

  **⚠ AMENDED 2026-08-07, same day, by a SECOND librarian filing the same
  commits — the last sentence above is PARTLY SUPERSEDED. Finding (2)
  DID mint a standing rule: R159.** The two filings ran concurrently and
  reached different judgements on the same finding; both judgements are
  recorded rather than one being quietly replaced, because the
  disagreement is the useful part.

  **The case for "no new number" (this entry's original position):**
  finding (2) is the R85 byte-oracle discipline applied to a new
  subsystem, and the project already knows to prefer byte-level proof.

  **The case for minting, which was accepted and is now `ROADMAP.md`'s
  R159:** R85 is a *specific instrument* — a preview-equals-saved oracle
  for the renderer — not a general statement about **what your test
  oracle is allowed to be**. Nothing in the rule set said *a lenient
  parser used as a test oracle inherits its own leniency as a blind
  spot*. **R87** governs whether the instrument was pointed at the thing;
  R159 governs an instrument that **was** and **corrected the reading on
  the way out**. **R92** and **R96** each explain one of the two defects'
  *proximate* causes and neither covers the shared blindness that let
  both ship. And the project's own minting bar is met on the face of it:
  **the same pattern occurred twice in one commit and had already shipped
  two defects** — R92 was minted on its second occurrence, R96 on its
  first.

  **The stronger form of this entry's own argument, which R159 adopts
  rather than discards:** this entry observes that knowing a rule was not
  sufficient to avoid violating it, and asks for a *mechanical* check.
  R159 supplies the mechanical form — *name the repairs your parser
  performs on the way in, and assert on bytes for every defect class it
  would absorb* — which is checkable at review time in the way R87's
  adopted wording is and its rejected wording was not.

  **Also, a pointer correction:** the `ROADMAP.md` *Shipped* entry this
  and the preceding entry cite as `Pass 20.0 + Pass 20.1 (completion)` is
  now headed **`Pass 20.0 — …`**. The original heading declared a Pass ID
  already headed elsewhere in *Shipped* and made
  `check-ledger-numbers.py` fail; the content is unchanged. **R160** was
  also minted the same day, from a process finding neither this entry nor
  the filing that wrote it contains — see `ROADMAP.md` *Standing rules*.

- **2026-08-07 (sixth entry) — R105 (`/TU` mandatory-or-declined) and the
  §3.4.3/§3.5.3 disclosures SHIP, closing two of the third-entry's own
  "still owed" list; the CLI verb-name question is the one item that does
  not close.** Established by `50a5461`, engineering work that was already
  live in `crates/pdfce-core/src/edit.rs` at the moment the fifth entry's
  own concurrency note observed it.
  **The mechanism is a sum type standing in for a boolean-shaped question
  that is not actually boolean.** `Option<String>` on `/TU` cannot
  distinguish "operator declined" from "operator was never asked," and for
  this one field the distinction is load-bearing rather than cosmetic: per
  decision 020 §3.5 (sourced to WebAIM), `/TU` — not the structure tree —
  is what a screen reader reads for a form field, bypassing `/StructTree`
  entirely. `TooltipChoice::Undecided` is therefore refused
  (`EditError::TooltipDecisionRequired`), not defaulted and not merely
  warned about; the asymmetry argument (the missing case is invisible to
  the sighted operator and load-bearing for the blind one) is the same
  shape rule 4's fuzzy-never-sneaky asymmetry already uses, applied here
  to an omission rather than to an inference.
  **A second, smaller finding worth keeping precisely because it is easy
  to get backwards:** declining the tooltip writes **no** `/TU` key, not
  an empty one. An empty `/TU ()` is a *worse* accessibility state than a
  missing key, because several screen readers announce an empty name
  rather than falling back to `/T` — so the "safe-looking" choice
  (write something, even if blank) is the wrong one, and the correct
  choice (write nothing) requires the declination to be recorded
  somewhere else, which is what `FieldAuthorDisclosures.tooltip_declined`
  is for.
  **Surface consolidation, not scope growth:** all three authoring verbs
  now share one outcome type, `FieldAuthorOutcome { field_id, merged,
  disclosures }`, and the choice-only `ChoiceAuthorOutcome` Pass 20.2/20.3
  shipped is retired into it. The CLI gained one `report_field_disclosures`
  printer in place of three would-be copies — the same "a third hand-copied
  block is where a disclosure silently goes missing" reasoning R159 already
  states for a different subsystem (a lenient-parser test oracle), applied
  here to a lenient-review-surface risk instead.
  **What does NOT close:** the CLI verb-name question (`add-text-field`/
  `add-check-box`/`add-choice-field` vs decision 020's `forms add-field
  --type …`) is untouched, and Pass 20.1 stays PARTIAL on that basis alone.
  **✅ AMENDED 2026-08-07 — that question is now CLOSED by the next entry
  below: the flat shape is ruled to stand and decision 020's nested form
  is superseded. Pass 20.1's PARTIAL status no longer rests on it.**
  **Nor does `/Tabs` tab-order authoring** — F4 remains blocked on the
  `pdfce-spec-librarian` dispatch named in the F0/F1 slice-order bullet
  above; only the disclosure that a field lacks a defined tab position
  ships here, not the computation of tab order itself. Full build record:
  `ROADMAP.md`'s `Pass 20.0` *Shipped* entry, THIRD addendum.
  **No standing rule minted** — R105 was already filed by this same
  decision record; this entry is R105 reaching implementation, not a new
  rule being written.

- **2026-08-07 (seventh entry) — the CLI verb shape for field creation is
  ruled FLAT; decision 020's nested `forms add-field --type …` is
  SUPERSEDED.** Ruled by `pdfce-engineer`; filed by `pdfce-librarian` the
  same day. Decision 020 §6 specified field creation as a nested surface
  (`pdfce-cli forms add-field --type text|checkbox|choice`, with sibling
  `forms remove-field`, `forms set-tab-order`, `forms rename-field`). What
  shipped across Passes 20.1/20.2/20.3/20.0 is flat: `add-text-field`,
  `add-check-box`, `add-choice-field`. The divergence was flagged four
  times without being settled, which meant it was being settled by
  accretion — one more verb per slice.
  **The flat shape stands.**
  **The basis is a measurement of the surface the new verbs had to join,
  not a preference.** `crates/pdfce-cli/src/main.rs`'s `Command` enum
  (lines 381–2414) carries **52 subcommands; every one is flat**, and
  `#[command(subcommand)]` occurs **zero** times in it. Two flat naming
  conventions already coexist — verb-first (`list-fields`, `fill-field`,
  `extract-text`, `regenerate-appearances`) and noun-prefixed
  (`dimension-add`, `group-set-scale`, `object-move`, `node-delete`) — but
  **nothing is nested under a noun subcommand.** `forms add-field` would
  have been the only nested verb in the entire CLI.
  **The reasoning worth preserving is not "the shipped thing wins because
  it shipped."** Decision 020 specified the nested form *before the
  surface it had to join was examined*; consistency with 52 shipped flat
  verbs beats consistency with a prediction made in the abstract. Had
  `pdfce-cli` been nested elsewhere, the ruling would have gone the other
  way. This is the same failure mode as a spec written against a
  remembered codebase rather than a read one — cheap to avoid by
  measuring, expensive once three verbs have accreted.
  **Reversible, and the operator's to overturn.** There is no release, no
  git remote and no users (project rule 8 — publishing still awaits Ken's
  go-ahead), so if Ken prefers the nested form the rename is three clap
  variants plus their tests: hours, not a migration. Recorded explicitly
  so this is not read as settled beyond appeal.
  **Consequences filed with it:** decision 020 gains a **§0 amendment
  placed ahead of its TL;DR** (so no reader meets the superseded shape
  first), with a per-slice mapping table and inline `[SUPERSEDED]` markers
  at every prescriptive CLI line in §6, including the machine-readable
  JSON block's `F1.cli` field. Decision 020 §11's *"should the shipped
  forms subcommands move under a `forms` parent"* bullet and
  `ROADMAP.md` open-operator-question **(q)** are the same question and
  are **CLOSED: no** — with no `forms` parent created, the six shipped
  forms subcommands have nothing to migrate toward.
  **Deliberately NOT ruled, and must not be inferred:** F2's radio verb
  name (decision 020 §6 authors one radio **member** per call through the
  F1 merge primitive, so `add-radio-group` would misdescribe the
  operation), F3's push-button verb name, and whether `remove-*` or
  `delete-*` is the house word for deletion (§6 says `remove-field` /
  `remove-widget`; the CLI elsewhere says `object-delete` / `node-delete`
  / `dimension-delete`). Those are picked when F2/F3 are scoped, against
  §6's own text.
  **✅ AMENDED 2026-08-07 — two of those three are now RULED by the
  eighth entry below: the radio verb is `add-radio-button`, and deletion
  is `delete-field` / `delete-widget` (`delete` is the house word). Only
  F3's push-button verb name remains NOT RULED, and still must not be
  inferred.**
  **Nothing minted** — no Pass ID, no standing rule, no new decision
  record. This is an amendment to decision 020, and the ceilings stay
  where they were (R160, decisions 031, Pass family 43).

- **2026-08-07 (eighth entry) — the radio verb is RULED
  `add-radio-button`; deletion is RULED `delete-field` / `delete-widget`,
  superseding decision 020 §6's `remove-*`; and `pdfce-cli`'s word order
  is found to be DOMAIN-PARTITIONED, not inconsistent.** Ruled by
  `pdfce-engineer`; filed by `pdfce-librarian` the same day, into
  decision 020's existing **§0.1** rather than a parallel record — the
  seventh entry above ruled the CLI *shape*, this one rules the *words*
  it left open.

  **Ruling 1 — `add-radio-button`.** Grounds, which matter more than the
  name: it matches its three shipped siblings' `add-<thing>` shape
  (`add-text-field`, `add-check-box`, `add-choice-field`), and **`button`
  rather than `group` is faithful to decision 020 §6's one-call-per-member
  design** — §6 authors one radio MEMBER through the F1 merge primitive
  and the group is what the merge produces, so `add-radio-group` would
  misdescribe the operation.

  **It is ratified on the MERITS, not because an in-flight fork wrote
  it — and that is the reusable part of this entry.** The prior
  librarian found `add_radio_button` already live in `crates/` and
  **refused to bless the name from source**, leaving `NOT RULED`
  standing. That refusal was **correct**, and it is why this is a
  decision rather than a fait accompli: adopting whatever a fork happens
  to have typed is exactly the accretion the seventh entry's ruling
  exists to stop — one more verb per slice, the convention settled by
  arriving rather than by being chosen. Had the name failed the two
  tests above, the fork would have been renamed to match the ruling.

  **Ruling 2 — `delete`, not `remove`; and verb-first.** Measured from
  `pdfce-cli --help`: **five shipped verbs use *delete*** —
  `delete-pages`, `dimension-delete`, `object-delete`, `subpath-delete`,
  `node-delete` — and ***remove* appears only in prose descriptions,
  never as a verb name.** Confirmed independently at filing time against
  `crates/pdfce-cli/src/main.rs`'s `Command` enum: **51 variants, five
  carrying `Delete`, ZERO named `Remove*`.** So `delete-field` /
  `delete-widget`, not `field-delete`.

  **The second-order finding, which is the one worth preserving: the CLI
  is not inconsistent about word order — it is DOMAIN-PARTITIONED, and
  each domain is internally consistent.** vector/dimension is
  **noun-first** (`object-list`, `object-delete`, `node-move`,
  `node-delete`, `subpath-delete`, `dimension-add`, `group-add`,
  `layer-toggle`); page is **verb-first** (`extract-pages`,
  `insert-pages`, `delete-pages`, `reorder-pages`); forms is
  **verb-first** (`list-fields`, `fill-field`, `add-text-field`,
  `export-data`, `import-data`). Forms being a verb-first domain is what
  makes `delete-field` right and `field-delete` wrong, and what made
  `add-radio-button` right without a separate argument. **The general
  rule: a new verb takes its word order from the DOMAIN it joins, not
  from the CLI as a whole** — recorded here so the next naming question
  needs no ruling, and so nobody "fixes" the cross-domain variation by
  normalising away the consistency that actually exists.

  **Still NOT ruled:** F3's push-button verb name. The flat pattern
  implies `add-push-button` and Ruling 1's sibling-shape argument would
  likely reach it, but F3 was not the question in front of the engineer
  and a name arrived at by inference is what both rulings refuse.

  **Nothing minted** — no Pass ID, no standing rule, no new decision
  record; ceilings stay R160 / decisions 031 / Pass family 43. **Whether
  the domain-partition finding deserves a standing rule of its own is
  flagged to the operator, not decided here.** No Pass is shipped or
  implied by this filing, and it carries no commit.

  **✅ AMENDED 2026-08-07 (later the same day) — IT DOES, and it is now
  STANDING RULE R161.** The paragraph above is left as filed. The
  operator ruled that the domain-partition finding is minted; the two
  verb-NAME rulings this entry records still mint nothing. **R161's
  binding statement:** *"A new verb takes its word order from the domain
  it joins, not from the CLI as a whole"* — with the corollary that the
  CLI's apparent cross-domain inconsistency **is not a defect and must
  not be "corrected."**
  **The deciding argument, recorded because it is the reusable half:
  there is a LIVE failure mode the rule guards against.** A finding that
  only *describes* the CLI could have lived in decision 020's amendment;
  one that must **stop** a plausible, well-intentioned future action — a
  tidy-minded rename of `object-delete` to `delete-object` — needs a
  number, **because the person about to take that action will not be
  reading decision 020.** Two supporting arguments also held: it is a
  standing constraint on all future work, and it would be rediscovered or
  contradicted if it lived only inside one decision record's amendment.
  **A three-way count discrepancy about the size of the CLI surface
  ("45+" / "52 subcommands" / "51 `Command` variants") was RECONCILED at
  the same filing rather than resolved by picking**: **52** is correct at
  every commit in the window in which both rulings were filed, **51** is
  a miscount (the true sequence is 50 → 52 → 53), and **"45+"** is a
  floor rather than a count. The load-bearing claims — zero
  `#[command(subcommand)]` inside the enum, zero `Remove*` variants,
  five `Delete` variants — held at all eleven commits measured. Method,
  per-commit table and the residual line-range imprecision are recorded
  in R161's own entry.
  **Ceiling is now R161; R162 is next free.** Decision records stay
  **031**, Pass family stays **43**. **F3's push-button verb name is
  still NOT RULED** and must not be inferred from R161 — R161 supplies
  the shape, not the word. Full entry: `ROADMAP.md` *Standing rules*,
  R161.

- **2026-08-07 (ninth entry this day) — Pass 20.2 COMPLETE: radio groups
  are built OUT OF the F1 merge primitive rather than by a path of their
  own, deletion implements decision 020 §3.6.3, positional-`/Opt` is
  REFUSED by name, and standing rule R162 is minted.** Commits `69ab966`,
  `834d256`, `817b268`.

  **The architectural claim worth recording is a NEGATIVE one: almost
  nothing in pdfce knows what a radio group IS.** `add_radio_button` sets
  `/Ff` bit 16, draws a round widget, and hands the name to
  `forms_author::resolve_field_path`. **Grouping falls out of §12.7.3.2
  meaning what it says a shared fully-qualified name means** — three calls
  with one `--name` produce ONE field with THREE widgets via the existing
  MERGE outcome, including Shape A→B promotion, with **no radio-specific
  grouping code and no `add-radio-group` verb.** Mutual exclusion likewise
  required no code: the already-shipped `set_button_state` sets each
  widget's `/AS` to the requested state when that widget offers it and
  `/Off` otherwise — **which IS radio behaviour**, written before radio
  authoring existed.

  **Why this belongs in the decision log and not only in the roadmap:** it
  is the second time this family has produced the same architectural
  result — a capability delivered by a *general* primitive rather than a
  *specific* path (the first being F1's four collision outcomes serving all
  authoring verbs identically). **The standing consequence is a
  constraint on future forms work:** a new button-family capability should
  be attempted through `resolve_field_path` and the existing state setters
  first, and a proposal that needs its own grouping/exclusion mechanism is
  a signal to re-read §12.7.3.2 before writing it. **A second mechanism
  for something the format already defines is the failure mode R92 names
  for appearance generators, here applied to STRUCTURE.**

  **Deletion is the first REMOVAL verb in the authoring family, and its
  §3.6.3 rule set is a design commitment, not an implementation detail.**
  Deleting the widget whose on-state equals the field's `/V` leaves `/V`
  naming a state no remaining widget can display — **a malformed field
  that parses perfectly**. pdfce sets `/V` and every surviving kid's `/AS`
  to `/Off` **and discloses it** (rule 4): §3.6.3 is explicit that both
  silences — leaving the dangling value, and clearing it quietly — are the
  sneaky outcome. **R102 holds on the way down** (a 3→1 group keeps its
  `/Kids` parent; both shapes are legal, so a deletion has no business
  rewriting object identities nobody asked it to change), and last-member
  `delete_widget` **delegates to `delete_field`** so the two paths cannot
  come to disagree about what *gone* means. **Two verbs at the surface,
  one function beneath**, because an optional `--index` whose absence
  silently means *delete everything* is a footgun.

  **Positional-`/Opt` radio authoring is REFUSED BY NAME, discharging
  decision 020 §8.3.** Table 227 lets a button's `/Opt` supply export
  values positionally, which makes the `/AP /N` keys array **indices**.
  pdfce parses `/Opt` but has never consulted it on the write side, so it
  can compute neither a new member's index nor the existing members'
  exports. §8.3 required *"either implement or explicitly refuse"*; **this
  is the refusal, chosen and reasoned**, and it is unreachable on
  pdfce-authored groups, which are always named. **Recorded as a decision
  so a future reader does not re-file it as a gap.**

  **Group flags are DISCLOSED, not applied and not refused** —
  `NoToggleToOff` and `RadiosInUnison` live on the **field**, so a joining
  member honouring its own would silently rewrite how every existing
  member behaves, while refusing outright would break the obvious script
  that passes the same flags to every call in a loop. **Bit 26 is read
  only through the type-gated `Field::radios_in_unison()`** — the same raw
  bit means `RichText` on a text field.

  **The round widget is pdfce's own stated design choice, not a parity
  claim.** §12.7.4.2 distinguishes radio from check box by bit 16 alone.
  But the convention is load-bearing: check box means *toggle me
  independently*, radio means *choose one of these*, and the difference is
  invisible until you click. **A group drawn as squares is a form that
  lies about its own behaviour.**

  **Standing rule R162 MINTED** — *an assertion that something is ABSENT
  proves nothing until the container has been shown capable of holding
  it.* Derived from a deletion test that was vacuous twice: first reading
  the bytes **after** the final `startxref` (an offset and `%%EOF`), then,
  once corrected to read `/Fields` and `/Annots` raw, **looping over a
  possibly-empty array**. It now re-derives the pre-deletion document and
  asserts three widgets were named there before asserting the
  post-deletion silence. **It is a peer of R87 and R159, not an amendment
  to either:** R87 asks *did I look in the right place?* (the instrument
  here was pointed correctly); R159 asks *did my reader lie to me?* (no
  lenient parser was involved); **R162 asks *could my assertion ever have
  come out false?*** **Ceiling is now R162; R163 is next free.**

  **R160 AMENDED IN PLACE (no number claimed) — a fork commits only the
  paths it authored.** `69ab966` carries five `pdfce-librarian` docs files
  — including **decision 020's own CLI verb-shape amendment** — inside a
  commit whose subject line describes radio groups, because the fork used
  `git add -A` despite an explicit warning. Nothing lost; **findability**
  lost. **The honest limit is recorded with the amendment:** a verbal
  warning was already given and ignored, so a rule number would have been
  ignored the same way — **the working mitigation is mechanical**, and is
  flagged to the engineer as tooling work rather than claimed as done.

  **Nothing else minted** — no Pass ID (20.2 was filed 2026-08-03 and
  headed by `bca60c9`'s entry; this completes it under hard rule 2), no
  new decision record. Decision records stay **031**, Pass family stays
  **43**, operator questions stay **(bb)**. ~~**F3's push-button verb name
  is still NOT RULED and must not be inferred from `add-radio-button`.**~~
  **[RULED 2026-08-08 — see the 2026-08-08 (thirty-second filing) entry
  below: `add-push-button`.]**
  Full build record: `ROADMAP.md`'s COMPLETION ADDENDUM on the
  `Pass 20.2 + Pass 20.3` *Shipped* entry.

- **2026-08-07 (tenth entry this day) — Pass 20.5 PARTIAL: the GUI can
  CREATE form fields, and an exhaustiveness predicate becomes a
  DESTRUCTURING so the next omission is a compile error**
  (`8a8678e`+`165dd49`). Four decisions and one deliberate cut.

  **`FieldAuthorDisclosures::any()` is now a destructuring, not a `||`
  chain — and that is the architectural change, not the bug fix.** The
  predicate omitted `group_flags_ignored`, the newest field, so the
  natural gate for a GUI disclosure block answered **`false`** for a radio
  merge whose **only** disclosure was that pdfce had overridden the
  operator's flags — **project rule 4 failing closed.** It was the
  **SECOND instance of the same omission, one field away**: the F2 fork
  fixed `report_field_disclosures` and the fix did not reach `any()`,
  which is **R162's shape** (the second instance survives the fix for the
  first). **No existing test caught it because the one test producing
  `group_flags_ignored` also calls `.declining_tooltip()`, so `any()` came
  out true through a DIFFERENT field** — vacuous **by coincidence rather
  than by construction**, which is why no reviewer would have seen it.
  **The fix moves the discipline from remembering to the type system:
  adding a struct field without handling it here no longer compiles.**
  **This is the second time in one day a structural fix beat a procedural
  one**, and the pattern — *prefer making the omission a compile error
  over writing a rule that asks a human to remember* — is **named as a
  standing-rule candidate and DELIBERATELY NOT MINTED**; **R163 is left
  free for the engineer to rule.** **✅ RULED THE SAME DAY — see the
  twelfth entry below: the candidate IS `R163`, and the ceiling is now
  R163 (R164 next free).**

  **The authoring tool joins a NEW ribbon group beside `Forms`, not
  `Forms` itself, because `Forms` documents its own reason for existing as
  arming no tool.** `RibbonGroup::Forms` states in its own doc comment
  that filling *"works with no tool armed and never touches the canvas
  gesture state"* and that `ContentTools` *"would promise a mode change
  that does not happen"*. **Creating a field IS that mode change**, so
  joining `Forms` would falsify the sentence justifying `Forms`. Not
  `ContentTools` either: that group is page **content**, while a form
  field is an `/AcroForm` entry that survives content edits and is removed
  by flattening. **One tool for four types**, with the type a Tool-Options
  control, because four tools would put four mutually-exclusive-in-
  practice entries on `TOOL_PRECEDENCE` where three of the four
  enabled-combinations mean nothing.

  **★ THE AUTO-NAME SCANS THE EXISTING NAMESPACE RATHER THAN COUNTING —
  a deliberate divergence from Acrobat, forced by pdfce's OWN merge
  semantics.** Acrobat's auto-name is a session counter that does not
  rescan (rename a box to `Check Box1`, make another, get `Check Box21`).
  **Copying that would be actively unsafe HERE specifically**, because
  **pdfce MERGES same-name same-type fields** (§12.7.3.2, shipped in F1):
  a colliding stub would turn a click the operator believes creates a
  **new** field into a **silent extra widget on an existing one**, arrived
  at through a name **pdfce chose itself** — rule 4's exact failure mode.
  **A parity claim would have been the wrong instinct**; the correct unit
  of comparison was pdfce's own write-side behaviour, not Acrobat's UI.
  Stub shape `Text1`/`CheckBox1`/`Radio1`/`Choice1` is pdfce's own pick,
  **not** a parity claim — Acrobat's corpus disagrees with itself
  (`Checkbox1` vs `Check Box1`).

  **The merge disclosure can only appear AFTER a successful Accept, and no
  preview is offered rather than an invented one.** `resolve_field_path`
  runs **inside** the core verb, so whether an add creates or merges **is
  not knowable until the call returns**. That is a genuine departure from
  every other reviewable action in decision 024's table, recorded as such.
  A GUI-side predicted "this will merge" would be a **second, divergent
  implementation of the resolver** — R92's shape. Every control and
  disclosure renders at a **fixed anchor inside the dock, never over the
  page** (decision 024 §4.4). An **untouched** auto-name is disclosed as
  an inference; an **edited** one is not, because that is direct
  manipulation — and a **dirty bit is the only way to tell them apart
  after the fact**, since the resulting `/T` is identical either way.

  **Push button is ABSENT rather than greyed** — ~~its verb name is still
  NOT RULED, so there is no capability to point a control at~~ **[the verb
  is RULED as of 2026-08-08 (`add-push-button`, thirty-second filing) —
  the palette omission's recorded reason has EXPIRED; it is now a cheap,
  un-taken follow-up, not a standing refusal]** (**R83**),
  and **R124** says an empty control teaches nothing its absence does not.
  **Default box sizes carry three different confidences, stated in the
  code**: text `150x22` and check box / radio `18x18` are community-
  sourced from the Acrobat RAG; **the choice default is DERIVED from the
  text field and says so** — no figure exists at any confidence, and a
  guess recorded as a guess is worth more than one dressed as parity.

  **PARTIAL BY CHOICE.** Field **deletion** in the Forms panel was
  core-complete (`817b268`) and scoped into this Pass, and was **cut**:
  *"half of both is worse than all of one."* The per-type detail fields
  (multiline, initial state, the choice option editor) were cut on the
  same reasoning — an empty `/Opt` is an allowed, disclosed state. Both
  stay owed **under Pass 20.5's own ID**, per hard rule 2.
  > **✅ AMENDED 2026-08-07 (tenth filing) — the DELETION half is now
  > BUILT** (`fc51786`+`69db1c6`; see the **thirteenth** entry this day).
  > **Only the per-type detail fields stay owed**, and **Pass 20.5 is
  > still PARTIAL**. The paragraph above is left as filed; it was correct
  > when written.

  **★ AN OWED VERIFICATION IS RECORDED RATHER THAN SMOOTHED OVER.** The
  Pass was driven in the running application (**R86**) via
  `tools/gui-drive.ps1`, and the traces establish the geometry
  (`llx=76.8 lly=73.7 w=150 h=22`), the R105 gate (`can=false` → `can=true`),
  the commit report (`name=Text1 merged=false notes=2`), the
  anti-collision guarantee end to end (a second placement offers `Text2`,
  unmerged), and that the pane's controls fit the compartment
  (`x=8-149`). **The rendered appearance was NOT visually verified** — the
  operator was at the machine with a file dialog open, the first capture
  photographed his desktop, and continuing would have stolen his focus.
  **That was the right call and is still a real gap**: three defects that
  session were caught **only by looking**.

  **Two harness findings, both about the observation harness attributing
  its own defects to the application:** an **unparseable diag step is
  SILENTLY DROPPED** (the absent trace reads as an application defect
  until a known-good sibling is checked — an R87-family silence in the
  *instrument*; generalizable half escalated to `D:\dev\rag\rust\`, and
  the mechanical `gui-drive.ps1` fix is **flagged as owed tooling work,
  not claimed as done**), and **the Edit tab is not active by default**,
  so tool traces need `tab:edit` first.

  **A process gap, named as such: `pdfce-ui-specialist` was dispatched and
  filed NO spec document**, so its design exists only in a conversation
  that compaction will discard. **`docs/ui_specs/` is where such a design
  belongs** — twenty-one precedents including `forms-panel.md` for this
  very feature family. **Not a criticism of the specialist**, which was
  asked for a critique and gave one; **the filing obligation belongs in
  the dispatch or in the agent file, and that is the engineer's call.**

  **Nothing minted.** No Pass ID (20.5 was filed 2026-08-03 by decision
  020's Backlog amendment and is headed here for the first time), **no
  standing rule — R163 deliberately left free** for the compile-error
  candidate above. Decision records stay **031**, Pass family stays **43**,
  operator questions stay **(bb)**. Full build record: `ROADMAP.md`'s
  `Pass 20.5 (PARTIAL)` *Shipped* entry.

  > **✅ AMENDED 2026-08-07 (same day) — the "nothing minted" clause above
  > is superseded in ONE respect: `R163` IS MINTED**, by the engineer's
  > ruling on the candidate this entry named. **Ceiling R162 → R163; R164
  > is next free.** Pass family (43), decision records (031) and operator
  > questions ((bb)) are unchanged. See the twelfth entry below.

- **2026-08-07 (eleventh entry this day) — veraPDF is ELECTED UNDER
  MPL-2.0, not GPL-3.0, and is an ARMS-LENGTH DEV-TIME TOOL.** Recorded in
  full in `LEGAL.md` §6.5 and §7; summarized here because it is the first
  time pdfce has taken a **licence election** under a dual-licensed
  artifact, and because the six operational rules it produces constrain
  packaging and `Cargo.toml` for the life of the project. **Every veraPDF
  component is dual-licensed GPLv3+ / MPLv2+**; under a dual licence the
  **recipient chooses**, and **an undocumented choice is an ambiguous
  one**. MPL-2.0 is **file-level** weak copyleft and **§3.3 expressly
  permits combination into a "Larger Work" under other terms** — no
  propagation path to pdfce's MIT licence. **A second, independent
  protection also holds**: the usage pattern triggers nothing even on the
  GPL branch (GPL §0 affirms unlimited permission to **run** an unmodified
  program; copyleft attaches to **distributing** a combined or derivative
  work; the GPL reaches neither a program's output nor a program that
  merely consumes it). **The enforceable rules:** never vendor/bundle/
  redistribute; never link or embed; never copy source, validation
  profiles or model files; separate process over the documented CLI
  consuming XML; **dev-time only, never in any `Cargo.toml`** (so it will
  correctly never appear in `THIRD_PARTY_LICENSES.md` — **do not "fix"
  that**); and **the gate SKIPS rather than fails when veraPDF is absent**,
  because a required gate would make it a de facto build dependency. **The
  veraPDF CORPUS (already in use under `LEGAL.md` §5) is a separate
  artifact with separate terms — do not conflate the two.**

- **2026-08-07 (twelfth entry this day) — `R163` IS MINTED: prefer making
  the omission a COMPILE ERROR over writing a rule that asks a human to
  remember. Plus a documentation-correctness sweep: every editable
  location still calling pdfce's licence UNDECIDED is corrected, six days
  after the MIT decision.** No code changed; no Pass ID, no decision
  record, no operator question minted. **Ceiling R162 → R163; R164 is next
  free.**

  **R163, and specifically its LIMIT, is the architectural content.** The
  rule says that when a defect's shape is *"someone will forget to update
  site B after changing site A,"* the first question is whether the
  language can be made to **refuse** the omission — exhaustive `match`,
  destructuring, newtype, `const` assertion — and that only when no such
  construction exists does the obligation become a rule. **It is the
  COMPLEMENT of the R87/R159/R162 family, not a fourth member:** those
  three ask *is my evidence real?* after the fact; R163 removes the class
  of omission before evidence is needed. **The evidence is two instances
  in one day** — `FieldAuthorDisclosures::any()` becoming a destructuring
  (`8a8678e`, the tenth entry above), and the `git add -A` case
  whose only working mitigation was agreed to be mechanical — plus, and
  this is the strongest item, **R160's own staging amendment already
  concedes the weaker form in writing**: *"a verbal warning was already
  given, in the dispatch, and was ignored. A rule number would have been
  ignored the same way."* **R163 generalises that concession from dispatch
  discipline to code structure.** ★ **The limit is inside the rule, not
  beside it**: R163 binds only where a compiler or equivalently mechanical
  gate *can* carry the obligation, and dispatch discipline, screenshot
  verification (R86) and corpus sourcing (rule 7) stay rule-shaped —
  **R163 is not a licence to skip writing a rule there.** Full text and
  the falsifiable *name-the-construction* test: `ROADMAP.md` *Standing
  rules*, `R163`.

  **The licence sweep — a same-filing-propagation failure with a
  measurable age.** `LEGAL.md` **§6's opening paragraph** still read
  *"pdfce's own license (§1, still undecided)"* **six days after
  2026-08-01's MIT decision**, which §1, §6.1 and §7's own 2026-08-01
  entry all record correctly. **§6.1 carries the correction inline, so no
  careful reader was misled — but a careless one was, and this is the
  fourth time this project has hit a single-location amendment.** Swept
  with three independent patterns over **tracked files only**
  (`git grep`), and corrected in every **editable** `docs/` location:
  `LEGAL.md` §6 opening, `PRIOR_ART.md`'s egui font-licence note, and
  `docs/ui_specs/gui-polish-current-featureset.md` (twice). `LEGAL.md`
  §7's two 2026-07-23 log lines were **correct at their date** and get
  forward-pointer markers rather than edits, per the append-only
  discipline; `docs/decisions/` is append-only by hard rule and is not
  touched at all. **Six stale statements OUTSIDE `docs/` are reported,
  not edited** (`README.md`, `about.toml`, `crates/pdfce-gui/src/
  ui_text.rs`, `crates/pdfce-core/src/image_codec/jbig2.rs`, and two agent
  files) — the dispatch scoped this filing to `docs/`. `README.md`'s is
  the one that matters most: it is the first document a reader opens.
  **Full record and the exact patterns used: `LEGAL.md` §7's 2026-08-07
  second entry.**

  **A reading-hazard correction is attributed to its real cause.**
  `LEGAL.md` **§6.5.5** already recorded that veraPDF's CLI banner names
  **both** licence branches and is not misleading. What it did not say is
  **whose hazard it was**: the earlier GPL-only reading came from a
  **`head -5`** invocation that cut at exactly line 5 and dropped line 6,
  leaving *"Released under the GNU General Public License v3"* as a
  complete, plausible, wrong sentence. **The truncation was self-inflicted
  by the tool invocation, not by veraPDF's wording**, and §6.5.5 now says
  so. **The generalizable shape — a truncated read of a WRAPPED sentence
  yields a complete, plausible, WRONG sentence, and the truncation is
  usually the reader's own `head`/`-n`/`--max-count` — is escalated to**
  `C:\personal_rag\claude_code\` (tool-invocation methodology, neither
  PDF- nor Rust-specific).

- **2026-08-07 (thirteenth entry this day) — A GATE THAT DIVERGES FROM
  ITS SIBLING MUST BE ASKABLE, OR THE DIVERGENCE IS A TRAP FOR EVERY
  SHELL. Plus: a fixture corpus that cannot distinguish two gates makes
  every test of the distinction vacuous.** Established by `fc51786` +
  `69db1c6` (the Pass 20.5 deletion surface — see `ROADMAP.md`'s
  `★ ADDENDUM` on the `Pass 20.5 (PARTIAL)` entry). **No Pass ID, no
  decision record, no rule number minted; R83 gains an in-place
  amendment.**

  **The first entry of this day established the RULE** — fill-shaped
  operations take the `/P`-aware certification gate, structure-shaped
  operations take the strict one — **and named where it could go wrong
  later: a future authoring verb calling `fill_refusal` because it was
  written by copying a fill path.** The real manifestation was **a
  different shape, and a more expensive one**: nobody called the wrong
  gate. **The wrong gate was the only one a SHELL could ask.**
  `EditSession::fill_refusal()` was `pub`; `deletion_preflight`'s strict
  gate had **no public query at all**. So a GUI panel could render a
  delete control it had **no way to gate correctly** — and on the
  ordinary real-world shape (**a certified fillable form at `/P 2`**) the
  correct answers *differ*: fill is permitted, deletion is refused.

  **The generalisation, which is why this is an architecture entry and
  not a commit note:** *when two operations take different preflight
  gates, EVERY gate in the divergent set needs a public per-frame query —
  not just the first one a shell happened to need.* A gate with no query
  is not merely inconvenient; it **forces the shell to pick the query it
  can reach**, which is the wrong one, and the resulting control is
  enabled and useless. `deletion_refusal()` returns the **`EditError`**,
  not a bool, for the same reason `fill_refusal()` does: **a shell forced
  to invent its own wording is how the engine's message and the surface's
  message drift apart** (R92).

  **★ The fixture finding is the durable half, and it generalises past
  forms.** **Every certification fixture in the corpus was `/P 1`**
  (*"no changes permitted"*), which **refuses BOTH operations** — so any
  test written against it **passes whether or not the two gates differ,
  and would keep passing if someone collapsed one into the other.** That
  is **R162 hiding in a FIXTURE CHOICE rather than in a test body**,
  which is materially harder to see: the test reads correctly, and the
  vacuity lives in a file it merely opens. **`/P 2` is the only level at
  which the two gates disagree**, so `certified-p2-form.pdf` was
  byte-authored to carry exactly it, with three tests blocking three
  different vacuous passes (divergence; the split is caused by the LEVEL;
  the gate is not simply stuck on). **The reusable question: for any rule
  with N outcomes, does the corpus contain an input for each — or does
  every fixture land on the same outcome?**

  **R83 is amended, not extended.** The same commit found the Forms panel
  `continue`ing past read-only, signature and push-button rows and
  thereby withholding the **delete** control those rows were entitled to
  (`deletion_preflight` checks encryption and certification and nothing
  else). **R83 forbids offering what you cannot do; it does not license
  withholding what you can.** Filed inside R83 — see `ROADMAP.md`
  *Standing rules* — because it is a scope clarification with no meaning
  apart from the rule it qualifies.

  **★ And the disclosure channel itself was unobservable.** `edit_note`
  is how **every** rule-4 disclosure in the GUI reaches the operator, and
  it emitted **no diagnostic trace** — so **a disclosure that silently
  stopped firing was indistinguishable from one that fired.** It is now
  traced at its **single drain point** rather than per producer, which is
  R163's construction applied: the next panel to add a note cannot forget
  it. **This is R87's family and arguably its worst instance to date** —
  the channel that exists to make pdfce honest was the one channel no
  behavioural harness could see.

  **Honest edge:** §4's API sync does not list **either** refusal query —
  `fill_refusal` and `deletion_refusal` are both inside §4(I)'s
  *"`EditSession`'s other 57 `pub fn`s"* gap, which predates this entry
  and is unchanged by it. Recorded so the addition is not mistaken for a
  §4 update.

- **2026-08-07 (fourteenth entry this day) — VALIDATION AGAINST A
  CONFORMANCE CORPUS IS COMPARATIVE, NOT ABSOLUTE: the question is "did
  pdfce make it worse", never "is the output perfect". Plus: pdfce
  acquires its first OUTSIDE reader.** Established by `9dcab62` →
  `b4d6d61` → `6ca17e1` → `1f5f1e5` (`tools/verapdf-parse-gate.py`).
  **No Pass ID — verification infrastructure, the same class as
  `tools/check-passes-filed.py`.**

  **The architectural gap it closes.** **Every test pdfce has reads
  pdfce's output with pdfce's own parser.** Round-trip reloads through
  `pdfce-core`; the forms tests assert through `parse_acroform`;
  redaction reads back with the lexer that wrote it. **A closed loop
  cannot see a defect both halves share** — and that is measured, not
  feared: **R159 exists because flatten left `/AcroForm /Fields` naming
  deleted objects and every forms test passed**, since `parse_acroform`
  drops entries that no longer resolve. **The model looked right while
  the file was wrong.** An external parser is therefore a **distinct
  verification tier**, not a redundant one.

  **★ The criterion is the design content.** The gate first asked *"does
  pdfce's output parse?"* — wrong, because **a conformance corpus is full
  of deliberately broken files** (that is its purpose), so an absolute
  reading **blames pdfce for damage it faithfully preserved.**
  `PDFBOX-6040-nodeloop.pdf` settles it with opposite verdicts under the
  two readings: veraPDF **cannot open the ORIGINAL** (*"can not locate
  xref table"*), while **pdfce's rewrite opens fine** and reaches only the
  page-tree loop **the file genuinely contains** — pdfce recovered the
  xref, and inventing a page tree would have been wrong. **Absolute:
  failure. Comparative: improvement.** Every input is now scanned as well
  as every output; the **only** failure condition is a file that came out
  **worse than it went in**, with improvements and preserved defects
  counted separately.

  **First real result — 115 pdfbox files: 0 regressions, 3 IMPROVED, 24
  refused by pdfce.** "Improved" = veraPDF reads pdfce's output and could
  not read the input. **That is `recover.rs` confirmed by an independent
  implementation** — evidence pdfce's own parser is structurally incapable
  of producing.

  **Three tool defects, recorded because each would have produced a gate
  that passes forever.** (1) **`--off` returns exit 0 for a file with no
  xref table** — valid → 0, garbage → 0; the verdict is only in the XML
  body. That is **R162 on the day it was written**, hence `--self-test`,
  which **fails if the gate does not fail**, and now also asserts the
  **tier**. (2) **veraPDF has TWO parse-failure tiers** — a
  `taskException type="PARSE"` it does **not** count in
  `batchSummary/@failedToParse` — and the tool's own cross-check **caught
  the engineer's conflation on the first real corpus run**, refusing to
  let either number be believed. (3) **`subprocess(text=True)` decodes
  with the platform locale**, so byte `0x8f` from a real corpus file
  killed the sweep with a traceback naming `threading.py` — the decode
  happens in a **reader thread**, so the tool was never mentioned. **A
  sweep whose purpose is running bytes from producers we do not control
  must not assume its own codepage can spell them.**

  **A build-collision property worth knowing before writing any other
  sweep:** `cargo run` per file holds `target/debug/pdfce-cli.exe` for the
  whole sweep, so a concurrent `cargo test` dies with
  **`Access is denied (os error 5)`** — an error naming a *file
  permission* problem with **no hint another job caused it**. The gate now
  **builds once and runs a private copy from its own temp dir.**
  Escalated to `D:\dev\rag\rust\`.

  **Scope boundary, stated so the green is not overread: this is the
  PARSE gate only.** The **PDF/A conformance gate remains unscoped
  Backlog** and will be a **separate tool** when `to-pdfa` ships, because
  a PDF/A profile run against non-PDF/A output reports failures that are
  not defects. Licensing posture (MPL-2.0 elected, separate process,
  never linked or redistributed, **skips rather than fails when absent**):
  `LEGAL.md` §6.5.

  > **★ AMENDED 2026-08-07 (fifteenth entry this day) — the TWO-TIER
  > design described in defect (2) is REMOVED, and the gate produced its
  > first real defect.** The *measurement* stands (veraPDF does emit a
  > `PARSE` exception it does not count); **modelling the tiers was a
  > false-positive generator**, because the count carries no job
  > attribution and a file's tier therefore depended on its 32-file
  > batch. Collapsed to a boolean in `23a5812`; the `batchSummary`
  > cross-check survives as a sanity check only. **The sentence above
  > reading *"now also asserts the tier"* is stale**, and the assertion it
  > describes is now unfalsifiable — see the fifteenth entry, below.
  > **The first real defect** (a catalog naming an absent `/Pages`
  > object) is fixed; qpdf corpus regressions **10 → 0**, improvements
  > **273 → 275**.

- **2026-08-07 (fifteenth entry this day) — A MISSING `endobj` COSTS THE
  WHOLE OBJECT, AND THE OBJECT IS NEVER REGISTERED RATHER THAN DROPPED;
  `round-trip`'s reload check is a SURVIVORSHIP TEST that §7.3.10 makes
  structurally blind to a dangling reference; and a PER-ITEM VERDICT
  DERIVED FROM A BATCH AGGREGATE IS BATCH-DEPENDENT.** Established by
  `49dfe81` (the parser) and `23a5812` (both instruments). **No Pass ID,
  no decision record, no rule minted** — `R164` is **REQUESTED and left
  free**; the argued case is at the end of `ROADMAP.md`'s *Standing
  rules*. **[★ AMENDED 2026-08-07, sixteenth entry — `R164` IS NOW
  MINTED. Ceiling R163 → R164; R165 next free. The clause above is left
  as filed.]** This is **the first defect the veraPDF parse gate found**, end
  to end: found by an independent parser, fixed, and confirmed by
  re-running the sweep. **Improvements went UP** — qpdf corpus, 560
  files: regressions **10 → 0**, improved **273 → 275**, preserved
  **7 → 15**, refused **79** unchanged; the ten reconcile exactly (2
  became genuine improvements, 8 became correctly-classified preserved
  damage).

  **(a) The parser, not the writer — and *never registered* is not
  *registered then dropped*.** qpdf's `bad6.pdf` omits **exactly one**
  `endobj`, the one after the `/Pages` node. `parse_object_at` requires
  the terminator (§7.3.10), so `confirm_candidates` **never registered
  object 2 at all**: recovery reported `file-level-objects=5`, the
  output contained 1, 3, 5, 6, 7, and the catalog still said
  `/Pages 2 0 R`. **pdfce wrote a document strictly worse than the one it
  read** — veraPDF could recover the original and could not recover
  pdfce's rewrite of it. **The distinction matters architecturally**: a
  writer-side guard (*"do not emit a catalog naming an object you do not
  have"*) would have produced a **different broken file**, not a working
  one, and would have concealed that a recoverable object was discarded.

  **`TerminatorPolicy` is `StreamLengthPolicy`'s sibling by construction,
  not by analogy** (see §5.11's `/Contents`-defect record, which now
  carries a third-policy marker). Same kind of decision: **the file
  contradicts the spec and which of two readings to believe is a POLICY
  choice, not a spec choice**, so pdfce makes it an explicit parameter
  rather than a hidden default. `Strict` is the default; **the clean load
  path is unchanged**, because accepting an inferred extent there would
  put a guess into the byte-identical re-emission path and break §5.
  Two load-bearing details: the leniency accepts **only when the
  terminator is an INTEGER** (`2 0 obj << … >> 3 0 obj` recovers; a body
  followed by an unexpected keyword still fails — **it cannot swallow
  trailing garbage**), and provenance is **`RecoveredFile`, not `File`**,
  because these bytes contain no `endobj` and re-emitting them verbatim
  would copy the malformation forward. **That is R94's second instance**,
  the first being the `/Length` repair in `409a6b5` — which had already
  documented the trap when this one was built. Counted and disclosed
  (R20): `missing_endobj_recovered` → CLI `missing-endobj-recovered=N`
  **plus a prose NOTE** naming §7.3.10, because a count with no sentence
  beside it states a number and not a fact.

  **(b) `reload_ok` asks "did every object I HAVE survive?" — and
  §7.3.10 is why that can never be enough.** A dangling reference
  **resolves to null rather than an error**, so a saved file that
  *references* something absent reads **clean** through the model. **The
  verb whose entire job is verifying the save invariant reported SUCCESS
  on the broken file.**

  **★ The engineer's first framing of this was WRONG and is corrected
  here rather than quietly dropped.** He held that `round-trip`'s check
  *should have caught* `bad6`. **It would not have.** pdfce could not
  resolve the page tree on the **input** either, so a comparative check
  correctly files `bad6` under **preserved damage**, not **destroyed** —
  and a **non-comparative** check would false-fire on **every**
  legitimately broken corpus file, where preserving the damage is
  correct. **Comparative is right; it was simply not SUFFICIENT.** The
  fix is therefore two things: **check 2b** (`page_tree_kept`, a hard
  `RELOAD_FAILED` when the source resolved a page tree and the output
  does not — §7.7.2 Table 28 makes `/Pages` required) **and a NOTE that
  fires whenever the OUTPUT lacks a page tree, stating whether the damage
  is NEW or INHERITED.** **Only the NOTE would have caught the defect
  that prompted it**, and it was verified by **reverting the recovery fix
  and watching it fire with the correct classification**. Fixing only the
  dropped object would have left the check just as blind to the next one
  — **R163 applied the day after it was minted: strengthen the gate, do
  not write a note asking future authors to look harder.**

  **(c) `c-empty.pdf` was the GATE's bug, and its mechanism is an
  aggregate attributed to an individual.** `/Type /Pages /Count 0
  /Kids []`, valid xref, both objects terminated — a **valid zero-page
  document** veraPDF grades identically on input and output, which the
  gate accused pdfce of breaking. veraPDF reports
  `batchSummary/@failedToParse` as a **count with no job attribution**,
  so the two-tier promotion had to guess from `counted == len(results)`,
  which made **a file's tier depend on what else was in its 32-file
  batch**. Input and output scans batch **differently**, so the same file
  could grade `WARNED` on one side and `FAILED` on the other and surface
  as a regression **purely from batching**. **Collapsed to a boolean** —
  batch-independent, and the only question a comparative gate asks. The
  `batchSummary` cross-check **stays, demoted**: veraPDF must never count
  more failures than we found exceptions for, because a counted failure
  with no exception attached is one the gate would report as clean.

  **Recorded against the gate's own author, at his instruction:** *some
  of the ten regressions reported to the operator were batching
  artifacts.* The count was stated **with more confidence than the
  instrument had earned**, even though the reconciliation is exact and
  the underlying defect was real.

  **★ A vacuity the CORRECT fix created, found while filing.**
  `--self-test` still asserts `tier != FAILED`, and **`FAILED` is now the
  only tier that can enter the results dict** — established by exhaustive
  grep of assignments into the container (`results[` → one hit,
  `results[name] = (FAILED, msg)`), not by reading the happy path. **R162
  exactly: the assertion cannot come out false.** The guard above it
  (`if not failures`) still carries weight; the tier branch is dead code
  that reads as a live check. **When three tiers became two, an
  assertion that had been meaningful became a tautology, and nothing in
  the project would have noticed.** Owed to the engineer as tooling —
  restore a second distinguishable outcome, or delete the branch and say
  why.

  > **✅ DISCHARGED 2026-08-07 (sixteenth entry), `b92c313`.** Neither
  > option was taken and the third answer is better than both — see the
  > sixteenth entry below. **The architectural content is the provenance
  > of the defect, not its repair: an R162 violation created BY THE FIX
  > FOR AN R162 FINDING**, which is how the class survives review.

  **Verification discipline worth carrying forward.** The regression test
  asserts on **bytes** (R159 — §7.3.10 makes a model-level check unable
  to distinguish *"object 2 is present"* from *"object 2 is missing and
  reads as null"*), asserts the catalog **references object 2 first**
  (R162's positive control), and its non-vacuity was proven by
  **reverting `TerminatorPolicy` to `Strict` and watching it fail**, with
  **paired parser unit tests** attributing the difference to the policy
  rather than to something incidental about the input. **A revert that
  makes a test fail proves sensitivity to something; the paired tests are
  what prove sensitivity to the thing.**

  **Gates at `23a5812`, each read by its own exit code (R87):**
  `cargo test` **2148 passed / 0 failed**; `cargo fmt --check`,
  `cargo clippy -- -D warnings`, `tools/check-ui-strings.sh`,
  `tools/check-bypass-paths.sh`, `tools/check-ledger-numbers.py` — all
  **exit 0**; the 560-file qpdf sweep **re-run by the engineer** rather
  than accepted from the fork that did the work.

- **2026-08-07 (sixteenth entry this day) — `R164` IS MINTED: A VERDICT
  WHOSE VALUE DEPENDS ON ANYTHING OTHER THAN ITS SUBJECT IS NOT EVIDENCE
  ABOUT THAT SUBJECT; the mechanism to look for is AN AGGREGATE
  ATTRIBUTED TO AN INDIVIDUAL. And the owed `--self-test` vacuity is
  DISCHARGED by `b92c313` — a fix that supplies the missing
  discrimination where it actually exists rather than reviving a fake
  tier.** Librarian-proposed at the fifteenth entry, **engineer-ruled**
  here. No Pass ID, no decision record, no operator question. **Ceiling
  R163 → R164; R165 is next free**, and decision 030's three contingent
  candidates plus the cross-RAG-handoff proposal **transfer R164 → R165**
  (the third such transfer this session, stated rather than performed
  silently). Pass family stays **43**; decision records stay **031**;
  operator questions stay **(bb)**. **[★ AMENDED 2026-08-07, nineteenth
  entry — `R165` IS NOW MINTED. Ceiling R164 → R165; R166 next free, and
  the contingent claims transfer R165 → R166 — the FOURTH such transfer
  this session. The clause above is left as filed.]**

  **Why this is architecture and not bookkeeping: it changes what the
  project will accept as a per-item measurement.** pdfce's verification
  strategy is increasingly **corpus-shaped** — the 560-file qpdf sweep,
  the 115-file pdfbox sweep, and the PDF/A conformance gate that will
  arrive with `to-pdfa`. Every one of those runs a third-party tool over
  **batches**, and `R164` fixes the contract for reading their output:
  **an aggregate that does not name the items it covers may CROSS-CHECK a
  per-item extraction and may never ASSIGN one.** The gate's own
  `batchSummary` cross-check survives on exactly that footing — demoted,
  not deleted, because *veraPDF must never count more failures than we
  found exceptions for*.

  **The family this joins, and the sentence it cost.** `R87` (did I look
  in the right place?), `R159` (did my reader repair the reading?),
  `R162` (could my assertion have come out false?) and now `R164` (does
  my verdict depend on anything other than its subject?) are one family
  of *is my evidence real?* questions. **Its unifying sentence had to
  widen to admit the fourth member**, from *"evidence that could not have
  come out differently is not evidence"* to ***"evidence whose value is
  not a function of its subject alone is not evidence about that
  subject."*** The engineer ruled that widening in **knowingly**, on the
  ground that both are failures of the **link between measurement and
  conclusion** rather than two unrelated hazards. **`R163` is unaffected
  and remains a complement rather than a member** — it operates before
  any evidence is needed.

  **The one-occurrence bar was overruled with its reason, which is worth
  recording as precedent.** This project promotes on **two** occurrences.
  R164 has **one**. It was minted because **that one produced a false
  report to the operator** — *"ten regressions"*, some of which were
  batching artifacts — **and because batched measurement is becoming more
  common here, not less.** *A rule whose first instance already cost a
  wrong statement to the operator does not need a second.*

  **The limit is part of the rule.** R164 binds where a **per-item verdict
  is DERIVED from a group-level result**. **Legitimate aggregate reporting
  — *"15 of 560 failed"* — is not the failure**; attributing the group's
  property to a **member** is.

  **★ The `--self-test` discharge, and its architectural content is the
  PROVENANCE of the defect.** `--self-test` asserted the broken file's
  tier was `FAILED`, which became **unreachable** the moment the two-tier
  model collapsed to a boolean — `FAILED` is the only value that can enter
  the results dict. **This was an R162 violation created BY THE FIX FOR AN
  R162 FINDING**: the dead branch was produced by a **correct** change (the
  collapse that is now R164) and **inherited its author's confidence.**
  That is how the class survives review — **nobody re-audits the guard
  they just wrote correctly**, and a repair is precisely the moment R162's
  question stops being asked.

  **A tier assertion could not be revived honestly**, because reinstating
  a second tier means reinstating the batch-dependent promotion R164
  forbids — *fabricating a distinction so a test can assert on it*. The
  fix instead supplies the discrimination **where it actually exists**:
  the gate's real claim is ***broken files appear, sound files do not***,
  and **a gate reporting EVERY file as unreadable would pass a
  one-directional test.** `--self-test` now scans a known-good document
  (`fixtures/synthetic/forms/demo-form.pdf`) as well as the deliberately
  broken one and fails if the good one comes back dirty. ***A detector
  that never says no is not a detector.*** Two stale docstrings were
  corrected in the same commit — the removed `WARNED` tier (kept as a
  record of **why** it was removed, which is the durable fact) and the
  module-level sentence describing the self-test as one-directional.

  **Two open items were RULED and are recorded as closed-by-decision, not
  as gaps.** (1) **The corpus arithmetic stays as filed** — the four
  buckets account for **369 of 560** and the remaining 191 were not
  measured; *do not backfill a number nobody measured.* (2) **The
  per-file split of the ten** between *"fixed by `49dfe81`"* and
  *"batching artifact"* **stays flagged as an INFERENCE** — it is exactly
  the R164 hazard (a per-file cause attributed from an aggregate
  reconciliation), and **labelling it honestly is the right answer, not a
  gap to close.**

  **This filing edited `docs/` and the RAG trees only**; the `b92c313`
  commit and every gate result quoted here are the engineer's.
- **2026-08-07 (seventeenth entry this day) — THE FULL-REWRITE WRITER
  GAINS A SPEC-SOURCED BOUND, AND THE CHOICE IS *REFUSE*, NOT *REPAIR*
  — a 1.2 KB file used to cost an hour and 40 GB** (`0df6158`; harness
  half `8cb779f`). **The second defect the veraPDF parse gate found, and
  the first one that was a HANG.** Full record: the *second defect the
  veraPDF gate found* entry at the top of `ROADMAP.md`'s *Shipped*.

  **The architectural content, stated as the invariant it touches.**
  `save_full` fills cross-reference holes with `for num in 0..=highest`
  where `highest` is the largest object **NUMBER** in the file, so the
  **writer's cost is O(largest object number), not O(object count)** —
  and the largest number is **chosen by whoever wrote the input**.
  pdfium's `bug_455199.pdf` names `2147483648 0 obj` (2³¹), asking for
  2,147,483,649 `BTreeMap` entries. Measured: **~27 MB/s of steady
  allocation with the CPU pinned**, i.e. roughly an hour and 40 GB before
  the allocator gives up. **Not an infinite loop** — which is what made
  it survive: *it looks like progress the whole way down*, so a liveness
  or progress check cannot see it and only a **wall-clock budget** can.

  **The loop is §7.5.4's completeness requirement and was NOT removed.**
  A single-section full rewrite must carry one entry per object number
  from 0 to the file's maximum, *"even if one or more of the object
  numbers in this range do not actually occur"*. Both ways of complying
  cheaply are worse, and each breaks a different thing this document
  guarantees:

  - **A sparse table** (`build_runs` would emit one) **trades a hang for
    a malformed file** — §7.5.4 is not satisfied by a table that skips
    numbers.
  - **Compact renumbering breaks §5's per-object byte-identity
    contract** — an object pdfce did not logically touch must come back
    byte-identical, and its number is part of what identifies it.

  **So the path is BOUNDED and REFUSED BY NAME (R27), and the bound is
  SOURCED:** `save::MAX_REWRITE_OBJECT_NUMBER = 8_388_607`, from **ISO
  32000-1 Annex C Table C.1's maximum indirect objects (2²³ − 1)** —
  the same construction as `MAX_FORM_FIELDS` / `MAX_ANNOTS_PER_PAGE` /
  `MAX_XOBJECT_DEPTH` (§10.1). New variant
  `WriteError::ObjectNumberTooLarge { num, max }`. **Deliberately not
  clamped to the object COUNT**: a sparse-but-small file with one
  enormous number is precisely the adversarial shape, and a count-based
  bound would pass it through.

  **★ The fact that makes the refusal correct rather than merely
  conservative.** Table C.1 caps a PDF **integer** at 2,147,483,647, so
  the file's 2,147,483,648 is **one MORE than the largest integer the
  spec permits**. The object number is **unrepresentable as a conforming
  PDF integer** — the guard therefore refuses **nothing a conforming
  producer can write**, and costs the corpus nothing.

  **The refusal bounds the WRITER only. Reading is untouched** — `inspect`
  and `extract-text` were both run on the file and both succeed. §10.1's
  guard family gains a member whose motivation is a **spec obligation**
  rather than a decompression bomb, which is a new reason for a guard in
  this codebase and worth noting as such.

  **★ An OWED verification, opened by this filing.** The *Standing rules*
  resource-guard bullet requires every new depth/count/size guard to get a
  run against veraPDF's **§6.1.12 implementation-limits** suite before it
  ships. **What exists is an ARGUMENT for headroom, in the constant's own
  doc comment — not the run.** The argument is strong (the bound comes
  from Annex C, not from intuition, which is the exact defect that rule was
  written after), but **R162's question applies to a rule's discharge as
  much as to a test**: the claim *"§6.1.12 has comfortable headroom"* has
  not been shown capable of coming out false. **Engineer: run it, or rule
  the argument sufficient.**

  > **[★ DISCHARGED 2026-08-07 — the engineer RAN it rather than waiving
  > it. 44 files, 0 hangs, 0 regressions, **0 refused**, and shown
  > non-vacuous by verifying the guard fires on `bug_455199.pdf`. The
  > paragraph above stays as the record of why it was owed; the full
  > discharge is the **eighteenth** 2026-08-07 entry, and §10.1's own
  > bullet carries it too.]**

  **Harness half — the gate stops lying about how far it got.**
  `tools/verapdf-parse-gate.py` gains **`--timeout` (default 120 s)**; a
  hang is a **reported finding naming the file, RANKED ABOVE
  REGRESSIONS**, because *a bad file can be inspected but a
  non-terminating save is an unrecoverable GUI freeze*. Before this, a
  stalled run **reported nothing at all** — the pdfium sweep stopped at
  file **87 of 331** and the remaining 244 were silently never tested,
  indistinguishable from a clean pass. **R162 at the harness level.**
  Verified in both directions (0.01 s budget → three known-good files
  report as hangs, exit 1; default → clean, exit 0), on the
  *a-detector-that-never-says-no-is-not-a-detector* discipline set the
  previous day. **Both corpora now complete for the first time:**
  pdfium **288 files, 0 hangs, 0 regressions, 223 improved**; qpdf
  **560 files, 0 hangs, 0 regressions, 275 improved**.

  **Nothing minted, and one candidate was refused a number deliberately**
  — *"bound an input-VALUE-driven cost before incurring it"* is already
  carried by R27 plus the resource-guard rule, has one occurrence against
  a two-occurrence bar, and its transferable content is a **technique**
  (wall-clock budget over liveness check) filed in `D:\dev\rag\rust\`
  rather than a rule. Per **R163**, an obligation a mechanical guard
  already carries does not also need a rule asking a human to remember it.

  **This filing edited `docs/` and the RAG trees only**; both commits and
  every gate result quoted here are the engineer's.

- **2026-08-07 (eighteenth entry this day) — A FULL REWRITE DROPS BYTES
  BEFORE `%PDF-`: §5.6 GAINS ITS ONE DELIBERATE EXCEPTION, BECAUSE BEING
  SPEC-LITERAL WAS NOT ENOUGH TO BE READABLE. Plus the §6.1.12 run owed by
  the seventeenth entry, DISCHARGED.** (`fa4f83c`; the third defect the
  veraPDF gate found.)

  **The decision, in one line:** `header_prefix_len -> usize` becomes
  `header_span -> Range<usize>` returning `marker..end`, so `save_full`
  emits `%PDF-` at **byte 0** and discards any preamble. Full narrowing
  and its boundaries: **§5.6.1**, added this day.

  **★ pdfce'S OUTPUT WAS NEVER WRONG, and that is what makes this entry
  worth reading.** Every one of the 17 cross-reference offsets and
  `startxref` in the output matched its **true absolute byte position** —
  checked exhaustively, not sampled — and *absolute from the beginning of
  the file* is precisely what §7.5.4/§7.5.5 require. **The corpus INPUT
  (`6-1-2-t01-fail-a.pdf`) was the malformed party**, carrying
  header-relative entries with an absolute `startxref`. The fix is
  therefore **not a repair of a writer defect**; it is a change of
  representation forced by a reader population.

  **The measurement that generalised it, and it is the substance of the
  decision.** A minimal 3-object file with **correct absolute offsets** and
  19 bytes of junk before `%PDF-` fails in veraPDF with *"can not locate
  xref table"*; the identical file with the junk removed parses clean.
  **veraPDF reads offsets as HEADER-RELATIVE whenever a preamble exists**
  — so this was never about one corpus file's convention, and **every
  preamble-preserving file pdfce had ever written was unreadable to it.**

  **Why dropping rather than preserving.** `iso32000__s__7.5.md` records
  the offset base as *"a real, load-bearing ambiguity"* that **ISO 32000-1
  does not resolve**; pdfce's own 1 KiB header tolerance is **Acrobat
  practice, not normative** (the same RAG file flags the ISO 32000-2
  citation once used for it as NEEDS VERIFICATION). Preserving the preamble
  **picks pdfce's side of an unsettled argument**. Dropping it makes the
  two readings **coincide** — header at byte 0 ⇒ absolute and
  header-relative are the same number — and stops re-emitting a §7.5.2
  violation the operator never asked pdfce to keep. **Licensed by §5.1:**
  `save_full` promises per-object identity, **not** whole-file identity;
  the incremental and identity-append paths are unchanged and never call
  `header_span`.

  **A REVERSED contract, handled as a reversal.**
  `a_file_with_leading_junk_keeps_it_through_a_full_rewrite` was a
  reasoned, tested §5.6 contract. It was **inverted in place, carrying its
  original reasoning and the measurement that killed it**, rather than
  deleted — because §5.6 is still in this document and still correct in
  general, so a deleted test leaves the next author free to re-derive the
  old conclusion with full confidence.

  **R162 at two levels, plus a self-catch.** With `marker..pos` reverted
  the integration test fails on its byte assertion and the new unit test
  fails on the span — **but the pre-existing shape test passes either
  way**, because all four of its cases put the marker at byte 0, where the
  two forms are indistinguishable. **That is why the second unit test had
  to exist.** One of the new expected values was also wrong on first
  writing (BOM case `3..18`, not `3..21`) — **the test caught its author,
  not the code**, which is the strongest evidence available that its value
  space is real. All new assertions are on **bytes, not through pdfce's
  reader (R159)**: pdfce parses its own preamble-bearing output perfectly,
  `object_count()` and `catalog()` both pass in the broken state, and that
  is exactly why an outside judge was needed.

  **The fixture is deliberately UNDAMAGED and deliberately ON DISK.**
  `fixtures/synthetic/xref-recover/header-preamble.pdf` is the only valid
  file in a directory of intentionally broken ones; its 12-byte preamble
  sits **inside** the probe window so it loads **strict** (unlike
  `offset-start.pdf`, which routes through recovery), isolating the writer
  from the recovery machinery. On disk rather than inline **so the gate
  keeps watching it**: it reports `improved`, and a regression restoring
  preamble preservation flips it.

  **Sweeps:** PDF/A-1b **569 files, 1 regression → 0**; qpdf (560) and
  pdfium (288) **re-swept and unchanged**; synthetic `xref-recover` (11)
  **0 regressions, 8 improved**. **2150 tests**; fmt, clippy, ui-strings,
  bypass-paths and both `cargo tree` invariants each read by their own
  exit code.

  **★ SECOND HALF — THE §6.1.12 RUN OWED BY THE SEVENTEENTH ENTRY IS
  DISCHARGED.** All **44** files from the four `*6.1.12*` directories
  (`Isartor test files/PDFA-1b`, `PDF_A-1b`, `PDF_A-2b`, `PDF_A-4`) swept
  at `--mode full`: **0 hangs, 0 regressions, 0 preserved, 0 REFUSED.**
  **`0 refused` is the number the rule wanted** — the new guard rejects
  nothing in the implementation-limits suite. **And the result is not
  vacuous, which is the part to keep:** a sweep showing *"0 refused"* would
  look identical if the guard could never fire, so the guard was verified
  **firing** on `bug_455199.pdf` separately. **Fires on a real file, silent
  across all 44 — two-sided, which is what the standing rule was asking
  for and what an argument in a doc comment could never supply.**

  **Nothing minted; one candidate PUT TO THE ENGINEER rather than taken** —
  *"where a spec leaves a question unresolved, emit the form under which
  the competing readings coincide"*. One occurrence against a
  two-occurrence bar, **but not carried by any existing rule** (§5.6
  pointed the other way and had to be narrowed; R27 is about refusing, not
  representing). Recommendation and the counter-arguments: `ROADMAP.md`,
  *third defect the veraPDF gate found*, *Ledger*.

  > **[★ RULED 2026-08-07, nineteenth entry — `R165` IS MINTED, against
  > the filing librarian's recommendation and on that librarian's own
  > counter-argument. The paragraph above stays as the record of the
  > candidate as PUT; it is no longer current status.]** The deciding
  > asymmetry is the one the paragraph above already names: **the
  > candidate refused two entries earlier was carried by R27 plus the
  > resource-guard rule; this one is carried by nothing.** Full reasoning
  > and the rule's LIMIT: the **nineteenth** 2026-08-07 entry, and
  > `ROADMAP.md`'s *Standing rules*, `R165`.

  **This filing edited `docs/` and the RAG trees only**; the commit and
  every gate result quoted here are the engineer's.

- **2026-08-07 (nineteenth entry this day) — `R165` IS MINTED: WHERE A
  FORMAT SPEC LEAVES A QUESTION GENUINELY UNRESOLVED, EMIT THE FORM UNDER
  WHICH THE COMPETING READINGS COINCIDE — not the form under which pdfce's
  own reading is correct. And THE FULL EXTERNAL CORPUS IS SWEPT END TO END
  AT ONE COMMIT: nine corpora, 3,759 files, 0 regressions, 0 hangs, 502
  improved.** Librarian-proposed **with a recommendation AGAINST** at the
  eighteenth entry, **OPERATOR-RULED IN** here. No Pass ID, no decision
  record, no operator question, **no commit** — the sweep is a measurement
  record. **Ceiling R164 → R165; R166 is next free**, and decision 030's
  three contingent candidates plus the cross-RAG-handoff proposal
  **transfer R165 → R166** (the **fourth** such transfer this session,
  stated rather than performed silently). Pass family stays **43**;
  decision records stay **031**; operator questions stay **(bb)**.

  **Why `R165` is architecture and not bookkeeping: it constrains what
  pdfce is allowed to EMIT, which no other rule in the project does.** The
  existing instruments all govern something else. **R27** governs whether
  pdfce **acts** when an input has no good reading. The
  R87/R159/R162/R164 family governs whether a **measurement** is real.
  **§5.6 governs not touching what the operator did not ask about** — and
  it pointed **the wrong way** at the one case where the question arose,
  which is the whole reason a rule was needed. R165 fills the gap between
  *refusing* and *preserving*: **choosing a representation when the
  standard declines to.**

  **The rule, and its LIMIT, which is part of it.** *When the standard does
  not settle how a value is interpreted and real readers are known to
  differ, ship — where one exists — the encoding under which the competing
  readings produce the same answer.* It binds **only** where the spec is
  **genuinely silent or self-contradictory**; it is **not** licence to
  deviate from a **clear** requirement because an implementation disagrees
  (that stays R27's disclose-or-refuse), **not** engaged by pdfce merely
  preferring a reading, and **not** a general normalisation licence — the
  coinciding form must be reachable inside the **§5.1 contract of the save
  mode in use**, which is exactly why the preamble drop is legal on
  `save_full` and illegal on `save_incremental` and identity-append.

  **★ IT CONFLICTS WITH §5's MINIMAL-DIFF POSTURE, AND §5.6.1 IS THE WORKED
  EXAMPLE OF RESOLVING THAT CONFLICT.** §5.6.1 now carries that framing in
  its own text: **§5.6 stays the default; R165 is the exception that must
  be argued for, per case, with the ambiguous clause cited.** A rule that
  can override a documented contract is exactly the kind that needs a
  worked example attached, or the next invocation will be a preference
  wearing a rule's number.

  **★ THE ONE-OCCURRENCE BAR WAS OVERRULED — for the SECOND time in one
  day — and the two reasons are different, which is why the precedent is
  recorded rather than left to accumulate.** `R164` was minted on one
  occurrence because **that occurrence had already produced a false report
  to the operator**. `R165` is minted on one occurrence for a different
  reason: **the single occurrence affected EVERY preamble-carrying file
  pdfce has ever written.** A two-occurrence bar guards against minting
  from a **coincidence**; a general principle about representing ambiguity
  is not a coincidence awaiting corroboration. **The count of occurrences
  and the reach of the occurrence are different measurements**, and the bar
  reads the first while the ruling read the second.

  **The disposition is the durable part.** The librarian **proposed the
  candidate, recommended AGAINST it, and then supplied the argument that
  overturned its own recommendation** — stating plainly that unlike the
  candidate refused two entries earlier, this one *"is not already carried
  elsewhere"*. The operator ruled it in **on that argument**, and named the
  counter to the librarian's own R163 objection as the strongest part of
  the filing: **neither §5.6.1 nor the `personal_rag/pdf` lesson is
  CONSULTED when a new ambiguity is met in a different clause**, which is
  precisely when the rule binds. R163 prefers a mechanical carrier over a
  rule; **here there is no mechanical carrier** — §5.6.1 documents one
  resolved case and cannot refuse a wrong choice in the next one.

  **★ SECOND HALF — THE FULL EXTERNAL CORPUS, SWEPT AT ONE COMMIT.** Nine
  corpora, measured post-fix **by the operator** at `fa4f83c`:
  PDF/A-2b **982**, PDF/A-1b **569**, PDF/A-4 **484**, qpdf **560**,
  PDF/UA-1 **295**, pdfium **288**, Isartor+PDF/UA-2+TWG+PDF/A-1a **459**,
  pdfbox **115**, pdf20examples **7**. **3,759 files, 0 regressions, 0
  hangs, 502 improved**, with roughly **157 refusals**, which are the
  designed outcome (R27) and not failures.

  **Two constraints on how that number may be read, both filed BEFORE the
  number because it will be quoted later.**

  1. **It is a PARSE gate, not a CONFORMANCE gate.** It proves an
     independent implementation can **read** what pdfce writes. **It says
     nothing about PDF/A conformance** — least of all for the **2,330**
     files drawn outright from named PDF/A and PDF/UA conformance suites.
     The conformance gate stays parked until `to-pdfa` and remains a
     **separate tool**.
  2. **The total is a SUM and is filed as one. R164 applies to it.** No
     individual corpus's verdict may be read off the aggregate, and no
     individual file's verdict off its corpus row. The librarian
     **re-derived both totals** rather than inheriting them — 3,759 and 502
     are **exact**, not approximate. **R164 being applied to the operator's
     own headline number the day after it was minted is evidence the rule
     is live rather than filed.**

  **★ THE ARCHITECTURAL FINDING SITS ABOVE THE NUMBERS: all three defects
  the gate found shared ONE shape — pdfce read its own broken output
  perfectly.** `round-trip` reported **success**, rasters **matched**,
  `extract-text` **worked**, and on the preamble defect `object_count()`
  and `catalog()` **both passed** while the file was unopenable to an
  outside reader. **Every in-house instrument agreed the files were fine.**
  That is **the closed-loop argument this gate was built on, demonstrated
  three times in one day on code that had already shipped** — not argued
  from first principles. **It is the standing justification for keeping an
  outside judge in the verification strategy**, and a future sweep that
  finds nothing does not weaken it: a clean sweep is a fact about today,
  the shape is a fact about the method.

  **What this librarian did NOT establish, stated so the entry's evidence
  has a visible edge:** every sweep figure above is the **operator's**,
  run by him; **no sweep was run here**, and **no claim is made about the
  working tree, the index, or any remote** — the `737f14a` tree state is
  his report. The **only** number this filing owns is the arithmetic.

  **This filing edited `docs/` ONLY** — no `crates/`, no `tools/`, no
  `fixtures/`, and **no RAG tree**, by the dispatch's explicit scope.

- **2026-08-07 (twentieth entry this day) — THE RENDERER'S COST CENTRE IS
  NAMED, AND IT IS pdfce'S OWN CLIP REPRESENTATION, NOT ITS PARSER AND NOT
  A DEPENDENCY: painting all 129,515 paths of a CAD sheet costs 0.87 s;
  the clip machinery was 95% of render time** (`76200e9`; 1× 32,313 →
  18,870 ms, 2× 447,862 → 214,714 ms, **output byte-identical**).
  **NOTHING MINTED — no Pass ID, no decision record, no operator question,
  and one standing-rule candidate PUT AND DECLINED.** Ceilings unchanged:
  Pass family **43**, standing rules **R165** (**R166** next free, still
  considered-and-refused for its own separate candidate), decision records
  **031** (**032** next free), operator questions **(bb)**. Body-section
  update filed in the SAME edit: §3's `pdfce-render` block. Full record:
  `ROADMAP.md`'s *fix — RENDER PERFORMANCE* Shipped entry.

  **Why this is architecture and not a bug note.** The finding is about a
  **representation**, and the representation is pdfce's own: the
  graphics-state clip is an `Option<tiny_skia::Mask>`, a page-sized
  coverage buffer at one byte per device pixel. Both fixes are local —
  a per-paint `.clone()` became a borrow (≈108 GB of memcpy for a single
  page, scaling with page **area**, so drawing one hairline cost more on
  bigger paper), and `intersect_clip`'s multiply is now bounded to the
  path's device bounds (**an identity, not an approximation**: outside them
  `Mask::new` had zeroed the buffer and `0 × old / 255 == 0`). **What
  remains is not local.** ~~`Mask::new` alone is **10.1 s of the remaining
  ~18 s** — 24,142 allocate-and-zero passes over a page-sized buffer — and
  removing it means changing what a clip *is*, since most PDF clips are
  `re W n` rectangles that need no mask at all.~~ **★ CORRECTED
  2026-08-07 by the twenty-first entry (`4475fe6`): `Mask::new` is
  1.02 s, not 10.1 s (an R164 instance), and 2.5% of clips are
  rectangles, not most. The struck sentence is preserved because the
  correction is the record. See the entry below.**

  **★ THE ORDER IS THE DURABLE PART, AND IT INVERTS THE OBVIOUS ONE.**
  With painting measured at **0.87 s**, tiling and threading — the two
  moves a renderer-performance discussion reaches for first — would today
  be optimising **5%** of the cost. The sequence is therefore: (1) stop
  allocating a page per clip; (2) **re-measure** the cache cliff, which may
  vanish for free; (3) tiling and threading, **last**. Recorded in
  `ARCHITECTURE.md` because it is a constraint on how this crate is allowed
  to be optimised, not a to-do.

  **★ THE MEASUREMENT METHOD IS WHY THE CLAIM IS MAKEABLE — ABLATION, NOT
  ATTRIBUTION.** Baseline 18.04 s → allocation only 10.99 s →
  `intersect_clip` skipped entirely **0.87 s**. A profiler attributes
  samples to frames; an ablation removes the suspect and measures what
  remains, which yields a **floor**, and only a floor bounds what any
  candidate optimisation can be worth. **Parse was split out the same way
  and eliminated first**: read ~3 ms, parse ~1.7 ms, page tree ~17 µs —
  **~0.005% of 32 s**, four orders of magnitude from mattering. Had that
  been assumed rather than measured, the work would have gone into the
  tokenizer and produced nothing.

  **★ THE SHAPE OF THE SCALING CURVE NAMED THE MECHANISM, and the
  conclusion is less transferable than the reasoning.** 0.25×→0.5×→1×
  cost 3.23× and 3.14× per step; 2× cost **14.1×**. *A quadratic term
  would have shown at every step.* Superlinearity appearing once, at one
  boundary, after three constant-ratio steps, is a **threshold**, not a
  complexity class — working set ~6 MB → ~24 MB, past L3. **Reading it as
  "quadratic in area" would have prescribed tiling**, i.e. the 5% the
  ablation had already excluded.

  **A PREDICTION IS RECORDED BESIDE ITS RESULT: the hypothesis was RIGHT IN
  MECHANISM AND WRONG IN LOCATION.** *Full-canvas masks, cost = elements ×
  page area* — correct, and it aimed the ablation correctly. It placed the
  cost **inside `tiny-skia`**. **No dependency was at fault and none was
  changed.** Filed because a correct mechanism attached to the wrong owner
  sends the next investigator upstream to read a crate's source and find
  nothing wrong with it.

  **THE RULE CANDIDATE, AND WHY IT WAS DECLINED RATHER THAN LEFT
  UNMENTIONED.** *"Before optimising a subsystem, establish its floor by
  ablation."* One occurrence against a two-occurrence bar; it **commissions
  work rather than constraining care**, which is the exact ground `R166`
  was refused on the entry before; and **R163 prefers a mechanical
  carrier** — a profiling harness reporting a floor beside a total would
  carry it without a rule. That harness **does not exist and never did**
  (`git log --all --diff-filter=A -- '*profile_render*'` returns nothing on
  any branch), so it is filed as owed under `ROADMAP.md` *Next up* —
  **build the artifact, do not mint the rule.**

  **What this filing did NOT establish.** Every performance number above is
  the **engineer's**, measured at `76200e9` and relayed (R87); **no render
  and no benchmark was run here.** The measurement input —
  `D:\Dev\temp\pdfce\ncored-benchmark-cad-drawing.pdf`, 5,724,699 bytes —
  is a **MEASUREMENT INPUT, NOT A FIXTURE**: outside the repository tree,
  untracked, and inadmissible under rule 7 / `LEGAL.md` §5, so **every
  number is reproducible only on the operator's machine**. An engineering
  fork was live in `crates/` at filing time; **this entry describes the
  state at `76200e9` and nothing about that fork's work**, and **no claim
  is made about the working tree, the index, or any remote.**

  **This filing edited `docs/` plus the cross-project Rust RAG and
  `C:\personal_rag\pdf\`** — no `crates/`, no `tools/`, no `fixtures/`.

- **2026-08-07 (twenty-first entry this day) — THE CLIP BECOMES SHARED
  RATHER THAN COPIED (`Arc<Mask>`), AND THE PREMISE THE PRECEDING ENTRY
  HANDED FORWARD IS FALSIFIED BY MEASUREMENT** (`4475fe6`; `q`/`Q` clone
  **6.80 s → 0.01 s**, 1× **17.47 → 10.18 s**, 2× **214.71 → 51.52 s**,
  **~3× and ~8.7× from the original baseline**, output **byte-identical on
  the CAD sheet and on 52 synthetic fixtures**). **NOTHING MINTED — no Pass
  ID, no decision record, no operator question.** Ceilings unchanged and
  **re-measured by running `tools/check-ledger-numbers.py`**: Pass family
  **43**, standing rules **R165** (**R166** next free, still
  considered-and-refused for its own separate candidate), decision records
  **031** (**032** next free), operator questions **(bb)**. Body-section
  update filed in the SAME edit: §3's `pdfce-render` block, which also
  carries the correction below. Full record: `ROADMAP.md`'s *fix — RENDER
  PERFORMANCE (second)* Shipped entry.

  **Why this is architecture and not a bug note.** It is a **type change
  that encodes a mutation contract**. `GraphicsState.clip` was an owned
  `Option<Mask>` and is now shared behind an `Arc`. **Sharing is sound
  because a clip is never mutated in place**: `intersect_clip` builds a
  **fresh** mask and **assigns** it, and the old mask is only ever **read**.
  `q` therefore needs a **new reference, not a new buffer**, and — the part
  worth keeping — **no copy-on-write machinery is required, because there
  is no write**. The `Arc` is the type finally admitting what the code
  always did.

  **★ `Arc`, NOT `Rc`, AND THE REASON IS A FUTURE INVARIANT.** `Rc` is
  cheaper (non-atomic refcount) and would have measured the same today.
  `Arc` keeps **`GraphicsState: Send`**, which is the precondition for
  rendering pages off the main thread — the last item on this crate's
  optimisation order. **Picking `Rc` would have made that item require a
  second type change**, so a marginal present saving would have bought a
  future migration. Cost of the choice: **one atomic increment per `q`**,
  against a 6.79 s saving.

  **★ A DIRECTIVE WAS DECLINED ON MEASUREMENT, AND THE FLAG THAT MADE THAT
  POSSIBLE WAS FILED ONE COMMIT EARLIER.** The twentieth entry handed
  forward *"most PDF clips are `re W n` rectangles needing no mask at
  all"*, and the same filing **flagged that sentence as UNCENSUSED** — the
  census behind it had counted the marking **operator** (`W` vs `W*`), not
  the clip **geometry**. **Measured: 612 of 24,128 clip operations — 2.5% —
  are axis-aligned rectangles.** A rectangle special-case would have
  optimised one clip in forty; **the work was reported rather than built.**

  **★ THE FAILURE MODE IS "SOUND REASONING, UNMEASURED POPULATION", AND IT
  IS DISTINCT FROM "WRONG REASONING".** The spec half of the argument
  **held on checking**: ISO 32000-1 **§8.5.3.3.2** (nonzero) and
  **§8.5.3.3.3** (even-odd) agree on a single closed convex subpath, and
  **§8.5.2 Table 59** makes `re` *"a rectangle as a complete subpath"*.
  Every step is correct. **What was never true is that real files contain
  such clips in quantity.** A valid derivation over an unmeasured
  population yields a correct theorem about an almost-empty set — and it
  passes every review that checks derivations. **Recorded here because it
  is a constraint on how this project is allowed to justify an
  optimisation: a spec citation licenses the transformation, it does not
  license the schedule.**

  **★ AN R164 INSTANCE, CAUGHT SIX HOURS AFTER R164 WAS MINTED, BY THE SAME
  INVESTIGATION.** The twentieth entry's **`Mask::new` = 10.1 s** is
  **wrong; it is 1.02 s**. The 10.1 s was read off an ablation that skipped
  `intersect_clip` **entirely** — which also leaves the clip `None`, making
  every `q` clone cheap and letting tiny-skia skip mask sampling. **It
  measured construction PLUS use and attributed all of it to
  construction**, which is exactly what R164 forbids. True 1× distribution:
  **`q`/`Q` gstate clone 6.80 s**, `mask.fill_path` 5.24 s, multiply
  2.26 s, **`Mask::new` 1.02 s** (15.32 s of 17.47 s accounted). **The
  figure was the ranking key for the whole remaining work order**, so the
  error did not merely misstate a cost — it misdirected the plan.
  **Amended in every location it reached** (`ROADMAP.md` ×2,
  `FEATURES.md` ×2, `ARCHITECTURE.md` ×2, `SESSION_LOG.md` ×1).

  **★ WHAT THAT INSTANCE ARGUES FOR, AND IT IS AN ARTIFACT, NOT A RULE.**
  R164 worked — it caught its own author's error. But the error **survived
  six hours and reached four documents**, and it survived **only because no
  standing instrument existed to re-check it against**: the ablation died
  with the working tree that produced it. The owed render-profiling harness
  (`ROADMAP.md` *Next up*) is therefore **promoted from convenience to
  consequence**, and gains a fourth required capability — **per-phase
  timing, not only a total and an ablation floor** — because phase timing
  produced the correct number and difference-of-ablations produced the
  wrong one. **The harness must make the right method the cheap one.**

  **★ THE STANDING-RULE CANDIDATE IS NOW FORMALLY REFUSED.** *"Before
  optimising a subsystem, establish its floor by ablation."* **Declined**,
  on the second of the three grounds the twentieth entry gave: **it
  commissions WORK, not CARE** — the exact ground `R166` was refused on one
  filing earlier, and **minting this after refusing that would be
  incoherent**. `R166` **remains free**. The other two grounds (a
  one-occurrence claim against a two-occurrence bar; **R163** prefers a
  mechanical carrier) stand as support, not as the decision.

  > **[★ AMENDED 2026-08-07 (twenty-third entry) — the ablation refusal
  > STANDS; `R166` is no longer free.** *"Ablate before optimising"* remains
  > **declined**, on this ground, unchanged. **`R166` was minted the same
  > day for a third candidate** — *a number whose instrument no longer
  > exists is not evidence* — which passes the **work-versus-care** test in
  > the **opposite** direction: it is honoured by *not acting*, at zero
  > cost. **Live ceiling R166; R167 next free.** The ablation error that
  > produced the 10.1 s figure is an **instance** of R166, but its fix is
  > the harness this paragraph asked for, not an admonition.]**

  **★ BYTE-IDENTITY IS A CLAIM ABOUT ALL PIXELS, SO ITS WITNESSES MUST SPAN
  ALL PIXEL-PRODUCING SURFACES.** The CAD sheet has **zero images and 242
  text elements**: it is an excellent instrument for path-and-clip cost
  *because* it is narrow, and that same narrowness makes it **unable to
  witness** a regression in image sampling, glyph rasterization or
  annotation appearance. Identity was therefore carried by **52 synthetic
  fixtures** spanning JPX, bilevel, annotations, text, vector and CMYK, in
  addition to the sheet. **The benchmark proves the optimisation; the
  corpus proves the absence of collateral damage. Two claims, two
  witnesses.**

  **★ A MEASUREMENT-SPREAD DISCIPLINE, RECORDED AS A QUOTING RULE.** Four
  figures now exist for `76200e9` at 1× — **17.47 / 18.04 / 18.87 /
  19.28 s**, about a **10% spread**, from a mix of instrumented and plain
  builds that **nobody has separated and this entry does not pretend to**.
  Consequence: **at 1×, quote one significant figure.** *"About −40%"* and
  *"about 3× from the original"* are supportable; a trailing digit is
  noise, and a later filing measuring 11 s has not regressed. The 2×
  figures move by margins far larger than the spread and are safe as
  stated.

  **What this filing did NOT establish.** Every performance number above is
  the **engineer's**, measured at `4475fe6` and relayed (R87): the census,
  the phase timings, the scale sweep, the gates (`cargo test` **2150 / 0
  failed**, `cargo clippy -- -D warnings` **0**) and every SHA-256. **No
  render, no benchmark and no census was run here.** The measurement input
  is unchanged and is still a **MEASUREMENT INPUT, NOT A FIXTURE** —
  outside the repository tree, untracked, inadmissible under rule 7 /
  `LEGAL.md` §5 — so **every number remains reproducible only on the
  operator's machine**. **An engineering fork is live in `crates/` on the
  next render step; this entry describes `4475fe6` and nothing beyond it.**
  Checked rather than inferred (hard rule 8, as amended 2026-08-07):
  `git remote -v` is **empty — no remote configured**, and the newest
  bundle in `D:\Dev\pdfce-backups\` is `pdfce-20260806-1646.bundle`, whose
  `pass-8-redaction` head is **43 commits behind** this branch's `HEAD`
  (`git rev-list --count a8d74b7..HEAD`).

  **This filing edited `docs/` ONLY** — no `crates/`, no `tools/`, no
  `fixtures/`, by the dispatch's explicit scope.

  > **★ AMENDED 2026-08-07 (twenty-second entry, below) — THE FIRST
  > PARAGRAPH OF THIS ENTRY'S *What remains* ARGUMENT IS VOID.** The
  > "extent, not shape" reframing rested on a **mean clip bbox of 0.663% of
  > the page**, which is **66.36%** — a fraction printed as a percent. The
  > entry is preserved unedited above; the correction is the next entry.
  > **The backup figure quoted in this paragraph is also superseded** —
  > re-checked at `6b33789`, the newest bundle is
  > `pdfce-20260807-1509.bundle`, **one commit behind**.

- **2026-08-07 (twenty-second entry this day) — THREE FIGURES WRONG BY TWO
  ORDERS OF MAGNITUDE IN ONE DAY, AND THE CONDITION THEY SHARE IS NOT A
  METHOD BUT A LIFETIME: EACH WAS PRODUCED ONCE, BY AN INSTRUMENT THAT DID
  NOT OUTLIVE THE QUESTION.** Commit `6b33789`. No behaviour changed, no
  Pass minted, no decision record minted, **`R166` NOT MINTED — it is
  RECOMMENDED and left to the operator** (see *the rule judgement* below).
  **Ceiling unchanged: Pass 43, R165, decision 031, question (bb)** —
  re-measured by running `tools/check-ledger-numbers.py`.

  **The three errors, and the only difference between them:**

  | # | figure as produced | actual | mechanism |
  |---|---|---|---|
  | 1 | `Mask::new` = **10.1 s** of an 18 s render | **1.02 s** | an ablation that moved three things and attributed all of it to one (**R164**) |
  | 2 | mean clip bbox = **0.663% of the page** | **66.36%** | a fraction printed as a percent |
  | 3 | clip-bbox cull hit rate = **73.71%** | **1.34%** | a clip bbox tracked in a thread-local that only ever **shrank**, never **widened on `Q`** — and `Q` reinstates a **larger** clip |

  **The mechanisms are three different defects** — a contaminated ablation,
  a unit error, a wrong state scope. **No single rule names all three**, and
  that is the operator's own objection to minting one, correctly stated.
  **But the mechanisms are not what they have in common.** Errors 1 and 2
  were **believed and acted on for hours**; error 3 **was caught inside the
  fork before it was reported**. The difference is **not care and not
  skill** — it is that **error 3 was measured a second time and the other
  two were not**, because by then a committed harness existed and the
  probes behind 1 and 2 had already been deleted with the working tree that
  produced them.

  **The line worth preserving verbatim, from the fork's own summary:**

  > *Two produced figures wrong by two orders of magnitude that were
  > believed and acted on. Neither survived a second measurement — both
  > survived because there was no second measurement to make.*

  **★ CONSEQUENCE FOR THE RENDERER'S DESIGN, not only for its history.**
  Error 3 is why **`clip_bbox` is a `GraphicsState` field** and not a
  thread-local: `q` and `Q` must carry it **exactly as they carry the
  mask**. Any clip-derived quantity tracked outside the graphics state is
  **monotonically wrong**, because `Q` restores a *wider* clip and a
  shrink-only tracker never widens. This generalises past clips — **any
  cached summary of a stacked, save/restore-scoped state must live in that
  state**, or it silently diverges at the first restore.

  **★ THE RETIRED OPTIMISATION HAD THREE INDEPENDENT REFUTATIONS, AND THAT
  IS THE ARCHITECTURAL CONTENT.** *"Size the mask to the clip, not the
  page"* was **retired, not annotated**, because a struck-but-plausible item
  gets rebuilt:
  1. **Size** — 66.36%. A mask sized to a 66%-of-page clip **is** a
     page-sized mask.
  2. **API, and it fails SILENTLY** — tiny-skia requires clip-mask size to
     equal pixmap size; `RasterPipelineBlitter::new` returns `None` on a
     mismatch (`pipeline/blitter.rs:36-44`), producing a `log::warn!` and a
     **dropped paint**. A smaller mask yields **wrong output, not fast
     output**, detectable only in a log line. **This is the load-bearing
     one for anyone touching tiny-skia here: a size contract enforced by a
     returned `None` is a contract you will violate without a test.**
  3. **Cost** — `Mask::fill_path` is **10.3 µs on 64×64 vs 8.3 µs
     page-sized**, dominated by **three raster-pipeline compilations per
     call** rather than by rasterization; `scan::path_aa::fill_path`
     **already** bounds itself to `path.bounds()`. `Mask::new` at page size
     is **24.6 µs**, so its ~1.02 s is real and **irreducible without
     changing the representation**.

  **Any ONE of the three would have killed the item, and one measurement
  would have found any one of them.** None was made before it was scoped,
  dispatched, and filed in four documents as the ranking premise for the
  renderer's entire remaining work order.

  **★ THE COMMENT WAS WRONG AND THE CODE IT JUSTIFIED IS RIGHT — keep
  them apart.** `intersect_clip`'s doc comment claimed clips *"mostly cover
  a few percent"* of the paper; it was **corrected in place the same day it
  was written**, the shortest life of the three errors and the only one
  caught by its own author. **The bound it justifies remains an IDENTITY
  and stays**: bounding the multiply to the new path's device bounds is
  exact — outside them the fresh mask is zero — and it skips the **~34%**
  of the page outside the new path. **A third of the work, not two orders
  of magnitude.** A correct optimisation with a 100×-too-generous stated
  motivation needs its *sentence* repaired, not its code reverted; and the
  repair must restate the real benefit, or the next reader deletes the
  bound for underdelivering against a number it never claimed.

  **★ THE RULE JUDGEMENT — RECOMMENDED, NOT MINTED, AND THE OPERATOR'S OWN
  READ IS TESTED RATHER THAN ADOPTED (as he asked).** Candidate text:
  ***a number whose instrument no longer exists is not evidence — it may be
  reported, but nothing may be scoped, ordered or built on it until a
  second measurement is possible.***

  The operator's read was: *the mechanism differs each time, so there is no
  single defect to name; but the condition is constant — a number produced
  once, by instrumentation that does not outlive the question.* **The
  condition half is right and the objection half does not survive testing.**
  Standing rules in this project name **conditions**, not mechanisms:
  `R162` asks *could my assertion ever have come out false?* across every
  mechanism by which an absence can be vacuous; `R164` asks *does this
  verdict depend on its neighbours?* across every kind of batch. **A rule
  that names a condition is not weakened by mechanism diversity — that is
  what makes it a rule rather than a bug report.**

  Tested against the three grounds the ablation candidate was refused on
  one filing ago, because that is the nearest precedent and the operator
  named it:

  1. **Occurrence bar — PASSES, and by a wider margin than R164 did.**
     The ablation candidate had **one** occurrence against a two-occurrence
     bar. This has **three in a single day**, and `R164` itself was minted
     on **one**.
  2. **Work versus care — PASSES, and this is the ground that actually
     decided the refusal.** The ablation candidate was refused because
     *"ablate before optimising"* **commissions WORK**, and work is
     scheduled rather than remembered. **This candidate commissions
     CARE**: it constrains what you may *believe* and *act on*, and it is
     honoured by **not acting**, at zero cost. It tells nobody to run
     anything. **The operator's read treats the R163-carrier argument as
     the same ground the ablation candidate was refused on — it was not.
     R163 was recorded there as "support, not as the decision."** The
     decisive ground comes out the other way here.
  3. **R163 prefers a mechanical carrier — PARTIALLY, AND ONLY LOCALLY.**
     Unlike one filing ago, **the carrier now exists**:
     `tools/render-profile` is committed and even **prints an explicit
     note when clips cover a large share of the page**, so *this* premise
     cannot be silently re-adopted. **But R163's own stated limit is that
     it binds where a compiler or equivalently mechanical check does the
     work**, and a harness is neither: **it does not run unless someone
     runs it**, and it guards **render** numbers only. The next
     load-bearing figure produced by a deleted probe will be in the text
     pipeline, the writer, or the parser, and `render-profile` will be
     silent about it. **R163 discharges the instance, not the class.**

  **Tested for redundancy against the family, which is the strongest
  argument against minting anything here — and it fails to reach:**
  `R164` covers a verdict **contaminated by its neighbours**, and would
  have caught error 1 and **neither** error 2 nor error 3. `R162` covers
  **absence** assertions and their positive controls; none of the three was
  an absence claim. `R87` requires that a claim be **established rather
  than inferred**, and all three **were** established — by measurement,
  once, wrongly. **The uncovered ground is exactly the operator's phrase:
  a verdict with no second opinion available at all**, as against `R164`'s
  verdict contaminated by the company it kept.

  **Judgement: rule-shaped, and recommended.** **NOT MINTED — `R166`
  remains free pending the operator's ruling**, per the dispatch's
  constraint that nothing is minted unless he rules it. If he declines, the
  honest record is that **the class is then carried by nothing**, since
  `render-profile` binds one subsystem and no rule binds the rest — and
  that is the bet, stated so it can be checked later rather than
  rediscovered.

  **What this filing did NOT establish (R87).** **Every measurement above
  is the ENGINEER's**, taken at `6b33789` and relayed: the 66.36% census,
  the 87/65/100/81/95% first clips, the 1.34% cull rate, the 10.3 µs /
  8.3 µs / 24.6 µs microbenchmarks, the gates (`cargo test` **2153 passed,
  0 failed**; `clippy` **0**; `clippy --features profile` **0**), the
  SHA-256 identity on the CAD render and the **59** synthetic fixtures
  hashed against a binary built at `4475fe6` in a worktree. **This filing
  ran no build, no test, no render and no census.** The measurement input
  remains a **measurement input, not a fixture** — outside the repository
  tree, untracked, inadmissible under rule 7 / `LEGAL.md` §5 — so the
  figures stay reproducible **only on the operator's machine**, which is
  precisely the condition the candidate rule is about.

  **Git and backup state — CHECKED, not inferred (hard rule 8 as amended
  today, `b1368ed`; this filing has a shell and says so rather than
  repeating the retired no-shell disclaimer).** `git rev-parse HEAD` →
  **`6b33789`**; `git status --porcelain` → **empty**; `git remote -v` →
  **empty, no remote configured**; `git bundle list-heads
  D:\Dev\pdfce-backups\pdfce-20260807-1509.bundle` → `pass-8-redaction` at
  **`7867ec4`**, and `git log --oneline 7867ec4..HEAD` → **one commit**.
  **The bundle is ONE commit behind, and the missing commit is `6b33789`
  itself.** **No engineering fork is live** — stated by the operator and
  flagged as the one claim here resting on his word rather than on a
  command.

  **This filing edited `docs/` ONLY** — no `crates/`, no `tools/`, no
  `fixtures/`, by the dispatch's explicit scope. The `crates/` and
  `tools/` corrections for the same 100× error arrived earlier, inside
  `6b33789`.

  > **[★ RULED 2026-08-07 (twenty-third entry, below) — `R166` IS MINTED,
  > AND THE OPERATOR REVERSED HIMSELF ON ALL FIVE OF HIS OWN OBJECTIONS.**
  > The candidate recommended in *the rule judgement* above was ruled **IN**
  > by the engineer. **Ceiling R165 → R166; R167 is next free**, and
  > decision 030's three still-unminted contingent candidates plus the
  > cross-RAG-handoff proposal **transfer R166 → R167** — the **fifth**
  > such transfer this session. **Nothing is renumbered**; the entry above
  > is left exactly as filed. Binding text and the full ruling live in
  > `ROADMAP.md`'s *Standing rules*, `R166`, which is canonical.
  >
  > **The reversal that matters, recorded here because this entry is where
  > the objections were tested:** the ground the operator had used to refuse
  > *both* prior candidates — ***"it commissions work, not care"*** — is the
  > one that **decided this candidate the other way**, and he said so
  > himself. **He applied his own test and got the SIGN wrong.** A test with
  > a direction, applied without checking the direction, will misfire again;
  > it is filed as a checklist item rather than as a confession.]**

- **2026-08-07 (twenty-third entry this day) — `R166` IS MINTED: A NUMBER
  WHOSE INSTRUMENT NO LONGER EXISTS IS NOT EVIDENCE. It may be REPORTED,
  but nothing may be SCOPED, ORDERED or BUILT on it until a second
  measurement is possible.** No commit — a `docs/`-only filing over
  `6502d51`. No Pass minted, no decision record, no operator question.
  **Ceiling R165 → R166; R167 is next free**, and decision 030's three
  contingent candidates plus the cross-RAG-handoff proposal **transfer
  R166 → R167** (the **fifth** such transfer this session). Re-measured by
  **running** `tools/check-ledger-numbers.py` (**exit 0**) and
  `tools/check-passes-filed.py` (**exit 0**) before and after, per
  R106/R133 — not read from prose.

  **Why `R166` is architecture and not bookkeeping.** It constrains what
  may enter a **work order** — which options get closed, which Passes get
  sized, which subsystems get optimised. The renderer's entire remaining
  plan was ranked, for a day, on a figure produced by a probe that no
  longer existed; **item 1′ was scoped, dispatched and filed in four
  documents on a number that was wrong by 100×.** A rule that governs which
  measurements may rank work is a rule about how this project decides what
  to build, which is §12's subject.

  **The three occurrences** — `Mask::new` **10.1 s → 1.02 s**, mean clip
  bbox **0.663% → 66.36%**, cull rate **73.71% → 1.34%** — are tabulated in
  the twenty-second entry above. **The third never reached a document**,
  because by then `tools/render-profile` existed and the fork caught it.
  **Nothing distinguishes the three except whether a second measurement was
  cheap at the moment the number was produced**, which is the rule stated
  as an experiment rather than as an argument.

  **★ THE ARCHITECTURAL CONSEQUENCE, AND IT IS THE REASON THE RULE POINTS
  AT ARTIFACTS.** R166 is discharged, permanently and per-subsystem, by a
  **standing instrument** — not by care. `tools/render-profile` discharges
  it for `pdfce-render` and **for nothing else**: it only runs when someone
  runs it, and it reports render numbers only. **The next load-bearing
  figure produced by a deleted probe will be in the text pipeline, the
  writer or the parser.** The design instruction that follows is concrete:
  **when a subsystem's performance or census figures start ranking work, it
  earns a committed harness before the ranking is acted on**, and the
  harness is the cheaper answer than the rule every time (R163). **R166 is
  written to be hollowed out by its own carriers**, and that is the
  intended end state rather than a weakness.

  **Its limit, because it is the part most likely to be over-read.** It
  does **not** forbid reporting a one-off number, and it does **not**
  require re-measurement before every mention. **The scope is three verbs
  — scope, order, build.** A figure may be stated with its provenance
  named; it may not be the *reason* for a decision.

  **Where it sits in the *is-my-evidence-real?* family.** R87 asks whether
  a claim was established at all — **all three occurrences were**. R159
  asks whether a lenient reader repaired the reading — **no leniency was
  involved**. R162 governs **absence** claims — **none of the three was
  one**. R164 catches a verdict **contaminated by its neighbours** — it
  catches occurrence 1 and **neither of the other two**. The uncovered
  ground, and R166's subject, is **a verdict with no second opinion
  available at all**.

  **The proposal was filed with a recommendation TO MINT and the ruling
  agreed** — the inverse of R165, which was minted against its proposer's
  recommendation. Both are recorded because **the disposition is the
  point**: a close call handed over with both sides argued is what this
  project wants from a librarian, in either direction. Full text:
  `ROADMAP.md` *Standing rules*, `R166`. Measurements:
  `D:\dev\rag\rust\tiny_skia_mask_pixmap_size_mismatch_drops_the_paint_silently.md`
  and
  `C:\personal_rag\pdf\lesson_20260807_cad_clip_geometry_census_66pct_page_bbox_single_subpath.md`.

  **Git and backup state — CHECKED, not inferred (hard rule 8 as amended,
  `b1368ed`).** `git rev-parse HEAD` → **`6502d51`**; `git status
  --porcelain` → **empty**, and **no engineering fork is live** — measured
  by that command at dispatch time rather than asserted from memory, which
  is the correction this session was owed twice. `git remote -v` →
  **empty**; the bundle remains the only copy. A **fresh bundle was
  created and verified** by this filing — see the **twentieth filing's**
  `SESSION_LOG.md` entry for the tip check, which is the part
  `git bundle verify` does **not** establish.

  > **[★ AMENDED 2026-08-07 (twenty-fourth entry, below) — `R166`'s NAMED
  > CARRIER IS NOW COMPLETE.** This entry said `tools/render-profile`
  > discharges `R166` for `pdfce-render`; at the time it was **missing the
  > ablation-and-floor half** that the owed-tooling item had carried unmet
  > for four filings. **`fa17d54` built it**, and the first thing it did
  > was **confirm** an existing figure by a second method within 4% rather
  > than correct one. **The rest of this entry is unchanged**, including
  > the carrier gap: `render-profile` still guards **render only**, and the
  > text pipeline, the writer and the parser still have no standing
  > instrument.]**

- **2026-08-07 (twenty-fourth entry this day) — THE RENDERER HAS A
  MEASURED FLOOR, AND IT IS SCALE-FLAT: 0.49–0.53 s WHILE PIXELS VARY 64×.
  CLIP CONSTRUCTION IS ~8.4 s = 86% OF THE RENDER, CONFIRMED BY A SECOND
  METHOD WITHIN 4%. TILING AND LOW-RESOLUTION PROXIES ARE REFUTED AS
  ANSWERS, NOT MERELY DEPRIORITISED.** `fa17d54` — `--ablate` and
  `--ablate-sweep` in `tools/render-profile`, plus the `Ablation` type and
  its confound model in the feature-gated `pdfce-render/profile`. **NOTHING
  MINTED — no Pass ID, no decision record, no operator question, and TWO
  standing-rule candidates weighed and BOTH declined.** Ceilings unchanged
  and **re-measured by running** `tools/check-ledger-numbers.py`
  (**exit 0**) and `tools/check-passes-filed.py` (**exit 0**): Pass family
  **43**, standing rules **R166** (**R167** next free), decision records
  **031** (**032** next free), operator questions **(bb)**. Body-section
  update filed in the SAME edit: §3's `pdfce-render` block. Full record:
  `ROADMAP.md`'s *tooling — THE FLOOR…* Shipped entry.

  **Why this is architecture and not a tooling note.** A floor is a
  **constraint on what any optimisation of this crate can be worth**, and
  it is the kind of fact that closes options permanently. Three of them
  close here:

  - **Tiling and threading cannot go below the floor at all.** The floor is
    **per-operation**, not per-pixel — it is the cost of walking **148,517
    content-stream operators** and constructing their paths, and it is the
    same at 0.25× as at 2×. **Tiles render fewer PIXELS, not fewer
    OPERATORS.** The prior entries ranked tiling last because it addressed
    5%; this one says something stronger and more durable — **it addresses
    a term that does not shrink.**
  - **A low-resolution proxy is bounded below by ~2.6 s.** At 0.25× the
    **full** render is **2.57 s, not 0.67 s**: clip construction drops only
    about **4×** for a **16×** pixel reduction, because it is dominated by
    per-clip fixed costs (three raster-pipeline compilations per
    `Mask::fill_path`; `scan::path_aa::fill_path` already self-bounds to
    `path.bounds()`). **Progressive refinement and proxies help less than
    pixel count suggests, and clips bind either way.** Anything in these
    documents implying otherwise is corrected by this entry.
  - **Mask sampling is not a cost centre.** Measured **free, at the noise
    floor**, by the one ablation that isolates it without confounds.

  **The complete map at 1×, in one place:** interpreter floor **0.5 s**
  (scales with nothing) · painting **~0.8 s** · mask sampling **free** ·
  **clip construction ~8.4 s, 86% of the render.** **Clip construction IS
  the render.**

  **★ THE SECOND METHOD, AND WHY THIS ENTRY IS `R166`'s FIRST EXERCISE.**
  Clip construction at **~8.4 s by ablation** reproduces the earlier
  **per-phase sum of 8.52 s** (`fill_path` 5.24 + multiply 2.26 +
  `Mask::new` 1.02) **within 4%** — two routes, one answer. `R166` was
  minted one filing ago on three figures each wrong by two orders of
  magnitude, and it says the obligation is discharged the moment a second
  measurement is possible. **Its first exercise returns a CONFIRMATION
  rather than a correction**, which is the outcome worth recording: a rule
  that only ever fires on errors never demonstrates what a healthy number
  looks like. **The renderer's remaining work order is now `R166`-clean**
  where for four filings it was not.

  **★ THE DESIGN FINDING, AND IT IS THE MOST TRANSFERABLE THING HERE: A
  TOOL BUILT TO DEFEAT A SPECIFIC PAST ERROR IS A DIFFERENT ARTIFACT FROM
  ONE THAT MERELY MEASURES.** `R164` was minted this morning and violated
  six hours later by this project — `Mask::new` filed at 10.1 s, actually
  1.02 s, because the ablation that produced it skipped clip construction
  and thereby **also** stripped mask sampling from every later paint and
  made every `q` clone cheap. **Construction plus use, attributed to
  construction.** The harness does not warn about that class of error; it
  **carries the rule structurally**:

  1. **Every ablation implements `confounds()`**, and the tool prints the
     list **beside the delta** on both output paths.
  2. **Only rows with an EMPTY confound list are marked `attributable`** —
     the output says *"no other cost centre changes with this."*
  3. **`clip-sample` exists SOLELY to be such a row.** It builds the clip
     and paints without sampling it, isolating sampling **the one way
     skipping construction never can**. A unit test asserts its confound
     list is empty and fails if a future change gives it a side effect.
  4. **An unknown `--ablate` token EXITS 2.** Ignoring a typo would render
     un-ablated, report a **zero delta**, and read as *"this centre is
     free"* — **a wrong answer wearing the shape of a finding.**

  **The generalisation, for the next harness this project builds:** a
  measuring tool encodes a theory of how its reader will misread it.
  Encoding the *specific* misreading that already cost this project a day
  is cheap — a few dozen lines — and it is the difference between an
  instrument and a number generator. **R163's preference for a mechanical
  carrier over a rule is the same instinct one level down.**

  **★ A COLD-START ARTIFACT LANDS ENTIRELY IN THE DELTA.** `clip-build`
  read **1.17 s at `--repeat 1`** against **0.74 s at `--repeat 3`** — a
  **58%** inflation. A delta is `baseline − ablated`; cold start is paid in
  the first render only, so **it does not cancel between the terms**, and a
  one-shot ablation **systematically overstates whatever it ablated**, in
  the direction that always flatters the finding. The tool now warns below
  `--repeat 2`, with those two figures in the warning text. **Filed with
  its limit: this is plausibly a CONTRIBUTING mechanism in the 10.1 s
  error and is NOT claimed as its cause** — the established cause is
  confound contamination (R164), and the cold-start contribution to that
  specific run is unquantifiable now because the probe is gone, which is
  `R166`'s own subject.

  **★ A REPORTING-DESIGN FINDING: NAME THE CONDITION, DO NOT PRINT A
  NEGATIVE DELTA.** Mask sampling reports as **`NOT RESOLVABLE at this
  sample size`**, with the delta and base beside it and the knob that
  tightens the bound named, rather than as *"removes AT MOST −0.01 s"*.
  **A negative delta reads as a broken row and buries the finding** — the
  reader concludes the tool is miscounting and stops, losing the one useful
  fact in the row. Naming the condition makes a null result **legible as a
  result**.

  **★ AND THE NULL RESULT OVERTURNED THE EXPECTATION OF THE PEOPLE WHO
  WANTED IT.** A per-pixel multiply against a page-sized coverage buffer on
  every paint is exactly the shape of thing that looks expensive. It is
  free. **The attributable row's first job was to be wrong about its
  author's guess**, which is the same property the `6b33789` entry praised
  in the large-clip note.

  **★ THE OPERATOR-FACING SUM, WITH ITS STATUS ATTACHED RATHER THAN
  STRIPPED.** The non-clip work sums to about **1.3 s**, inside the
  **~1.6 s** the reference product's authors report as cold-to-sharp.
  **That is ARITHMETIC OVER SEPARATELY MEASURED PARTS, NOT A MEASUREMENT** —
  `floor 0.5 + painting 0.8`, each measured in a different configuration,
  summed; **nobody has rendered this file in 1.3 s.** `R164` applies to it,
  and the operator said so himself when he reported it. The parts **do**
  reconcile with the ~10 s total, which is real support. It is recorded
  with the qualification **because *"pdfce is 1.3 s away"* is precisely the
  sentence that will be quoted onward**, and because a sum of ablated runs
  is the same reasoning shape that produced the 10.1 s error. **The
  reference remains uninstrumented; ~1.6 s is context, not an acceptance
  criterion.**

  **★ NOTHING MINTED — two candidates weighed, both declined.**
  **(A)** *"A one-shot measurement overstates whatever it ablated."* A fact
  about a **method**, not a condition on care, and it **already has a
  mechanical carrier** in the `--repeat < 2` warning — **R163 is
  decisive**; the rule would be redundant on the day it was written.
  **(B)** *"Report a null result by naming the condition, not by printing a
  negative delta."* One occurrence against a two-occurrence bar,
  **reporting-design craft** that no gate can check. Both are recorded as
  findings so a future filing starts from the argument rather than from
  scratch. **`R167` remains free.**

  **What this filing did NOT establish.** Every performance number above is
  the **operator's**, measured at `fa17d54` and relayed (R87) — the floor
  table, the scale sweep, the 8.4 s clip figure, the 1.17/0.74 s repeat
  comparison, and the gates (`cargo test` **2157 / 0 failed**; `clippy`,
  `clippy --features profile`, `fmt`, ui-strings and bypass-paths all
  **0**). **No render, no build and no test was run here.** The measurement
  input is unchanged and remains a **MEASUREMENT INPUT, NOT A FIXTURE** —
  outside the repository tree, untracked, inadmissible under rule 7 /
  `LEGAL.md` §5 — so **every number is reproducible only on the operator's
  machine, though now by a COMMITTED instrument rather than a deleted
  one.** The tool's four behaviours listed above were verified here **by
  reading the COMMITTED BLOBS** (`git show fa17d54:…`), deliberately not
  the working tree, because a live fork can change a working copy under a
  reader.

  **Git and backup state — CHECKED, not inferred (hard rule 8 as amended,
  `b1368ed`).** `git rev-parse HEAD` → **`fa17d54`**;
  `git status --porcelain | wc -l` → **0**. **The dispatch stated that an
  engineering fork IS live in `crates/` and `tools/` and that those trees
  should be expected dirty; at the instant these commands ran, they were
  clean.** Both are recorded, and **the clean reading is a snapshot, not a
  claim that no fork is live** — a fork between edits shows clean.
  **This entry describes `fa17d54` and nothing beyond it (R87).**
  `git remote -v` → **empty**; the bundle is still the only copy. The
  newest bundle is **`pdfce-20260807-1552.bundle`** — *not* the
  `…-1557.bundle` the dispatch named, which does not exist — with
  `refs/heads/pass-8-redaction` at **`34c676d`**, **one commit behind
  `HEAD`** (`git bundle list-heads` + `git log --oneline 34c676d..HEAD`).
  **A tip matching `HEAD` would still say nothing about uncommitted work:
  a bundle captures committed history only, and this filing's own `docs/`
  edits are uncommitted and in no bundle.**

  **This filing edited `docs/` ONLY** — no `crates/`, no `tools/`, no
  `fixtures/`, by the dispatch's explicit scope.

  > **[★ AMENDED 2026-08-07 (twenty-fifth entry, below) — ONE FIGURE IN
  > THIS ENTRY IS CORRECTED, ONE IS FLAGGED UNRECONCILED, AND THE REST IS
  > CONFIRMED BY A THIRD ROUTE.**
  > **CORRECTED:** *painting **~0.8 s*** in this entry's complete map is
  > **~0.27 s**. The 0.81 s was the whole `clip-build`-ablated render —
  > **floor PLUS painting** — which is **`R164` a third time in one day**,
  > the same shape as the 10.1 s error this entry was partly written about.
  > Ablating `paint` alone moves the total 9.28 → 9.32 s, **inside noise**.
  > **Consequence: the tiling/threading item addresses under 3%, not 5% —
  > direction unchanged, margin grew.**
  > **CONFIRMED:** the per-phase sum this entry validated by ablation
  > re-measures directly at **5.22 + 2.46 + 1.03 = 8.72 s**, and **sum +
  > floor = 9.26 s against a 9.49 s render.** A **third** independent route
  > to the same 86%.
  > **⚠ UNRECONCILED:** this entry's **2.57 s at 0.25×** — the basis of
  > *"a low-resolution proxy is bounded below by ~2.6 s"* — against the
  > **~2.23 s** implied by `110b8c9`'s *"`fill_path` is 56% of the whole
  > render at 0.25×"* with `fill_path` = 1.25 s. **13% apart, outside the
  > 5.8% spread this entry itself measured.** No denominator was stated;
  > **neither figure is retired here and the engineer should say which
  > stands.** The conclusion is unaffected — **2.2 s and 2.6 s are both
  > ~3.5× above the 0.67 s naive pixel-count scaling predicts.**
  > **STRENGTHENED:** this entry argued that proxies underdeliver from a
  > *total*; the twenty-fifth supplies the **law** —
  > **`fill_path` grows ~2× per 4× pixels because the scanline converter
  > follows the path's PERIMETER.** **Everything else in this entry stands
  > unchanged**, including both `R164`-carrier design findings and both
  > declined rule candidates.]**

- **2026-08-07 (twenty-fifth entry this day) — THE 86% IS BROKEN DOWN AND
  THE ARITHMETIC CLOSES: `fill_path` IS 59.9% OF CLIP CONSTRUCTION AND WAS
  FILED 22× TOO LOW. THE PER-CLIP DISTRIBUTION IS UNIFORM — NO TAIL, NO
  HEAD — SO NO FAST PATH CAN EXIST AND ANY FIX MUST CHANGE THE WORK FOR ALL
  24,128 CLIPS. `fill_path` TRACKS THE LINEAR DIMENSION, WHICH IS THE
  MEASURED REASON PROXIES AND CULLING UNDERDELIVER.** `110b8c9` — per-phase
  clip timing behind `#[cfg(feature = "profile")]` in
  `crates/pdfce-render/src/interpret.rs`, a per-clip histogram in
  `crates/pdfce-render/src/profile.rs`, and their reporting in
  `tools/render-profile/src/main.rs`. **NOTHING MINTED — no Pass ID, no
  decision number, no operator question, and THREE standing-rule candidates
  weighed and ALL THREE declined.** Ceilings unchanged and **re-measured by
  running** `tools/check-ledger-numbers.py` (**exit 0**) and
  `tools/check-passes-filed.py` (**exit 0**): Pass family **43**, standing
  rules **R166** (**R167** next free), decision records **031** (**032**
  next free), operator questions **(bb)**. Body-section update filed in the
  SAME edit: §3's `pdfce-render` block. Full record: `ROADMAP.md`'s
  *tooling — `fill_path` IS THE MISSING TWO-THIRDS…* Shipped entry.

  **Why this is architecture and not a measurement note.** The twenty-fourth
  entry established a **floor** — a bound on what any optimisation of this
  crate can be worth. This one establishes a **distribution and a scaling
  law**, and between them they close a *category* of fix rather than an
  instance of one. That is the kind of fact that survives every future
  refactor of this crate, because it is a property of the algorithm
  tiny-skia implements, not of pdfce's arrangement of calls to it.

  **THE BREAKDOWN, at 1× over 24,128 clips:**

  | phase | total | per clip | share |
  |---|---|---|---|
  | `Mask::new` | 1.03 s | 42.7 µs | 11.8% |
  | **`fill_path`** | **5.22 s** | **216.4 µs** | **59.9%** |
  | the multiply | 2.46 s | 102.0 µs | 28.3% |
  | **sum** | **8.72 s** | **361.2 µs** | |

  **Sum + floor (0.54 s) = 9.26 s against a 9.49 s render.** The arithmetic
  closes to **97.6%**, and the **0.23 s residual is filled by the corrected
  painting figure of ~0.27 s** — so the complete 1× map now reconciles to
  **about half a percent**. **That closure is the check that makes the table
  worth believing**, and it is precisely what the previous numbers lacked:
  `Mask::new` 24.6 µs + `fill_path` 8–10 µs + multiply 94 µs summed to
  **~130 µs** against a **348 µs** mean. **A per-phase table that does not
  sum to the whole is not a breakdown; it is three numbers next to each
  other.**

  **★★ CORRECTION ONE, AND IT IS A FAILURE SHAPE THIS PROJECT HAS NOT SEEN
  BEFORE: THE EXPERIMENT VARIED THE WRONG DIMENSION.** `Mask::fill_path` was
  filed at **8.3 µs page-sized / 10.3 µs on a 64×64 mask**; here it is
  **216.4 µs — wrong by ~22×**. The original measurement compared a **small
  path** in a 64×64 mask against a **small path** in a page-sized one. **It
  varied the BUFFER and held the PATH fixed.** In this file the path covers
  **66% of a megapixel page**, and:

  > **An anti-aliased scanline fill costs what the PATH'S EDGES cost, not
  > what the BUFFER costs.**

  **Distinguish this from the day's other three errors, because filing it
  under one of them loses the lesson.** It is not a miscalculation (the
  0.663% fraction-as-percent), not a confounded ablation (**R164**), and not
  a vanished instrument (**R166**). It is an **experimental-design** failure:
  the hypothesis concerned the cost of *filling this path*, and the
  experiment manipulated *the size of the thing filled into*.

  **★ AND THE CONCLUSION THAT MEASUREMENT SUPPORTED IS TRUE — THE
  CLIP-SIZED-MASK DEAD END STAYS CLOSED.** *Buffer size does not drive
  `fill_path`'s cost* and *cost follows the path's edges* are **the same
  fact from two sides**. Refutation (c) of `ROADMAP.md`'s retired item `1′`
  therefore stands unchanged. **What does not transfer is the absolute
  figure** — 8.3 µs is what a *small* path costs, quoted as though it were
  what *this file's* clips cost. **A number can be right about a ratio and
  wrong about a magnitude, and the two travel together in one sentence.**

  **★★ AND THE CONTRADICTION WAS ALREADY FULLY PRESENT IN THIS PROJECT'S
  OWN FILED NUMBERS.** `mask.fill_path` **5.24 s** ÷ **24,128** clips =
  **217 µs**, filed in `ROADMAP.md`'s `4475fe6` entry; **8.3 µs** was filed
  one entry later, **217 lines away in the same file**, and
  24,128 × 8.3 µs = **0.20 s**. **A 26× internal contradiction carried
  openly for two filings, because nobody ever put the two figures in the
  same expression.** The transferable instruction is not *"measure more"*
  but **"divide the totals you already have by the counts you already
  have"** — a consistency check with no measurement cost at all, for which
  this project had every input. Note that **5.24 s was correct all along**
  and re-measures at **5.22 s**: the phase *totals* were never wrong.

  **★ CORRECTION TWO — PAINTING IS ~0.27 s, AND THE 0.87 s HEADLINE WAS
  MISLABELLED FROM ITS FIRST FILING.** Painting was filed at **~0.8 s**
  (twenty-fourth entry) and before that as *"painting all 129,515 paths
  costs 0.87 SECONDS"* (`76200e9`, including in that entry's `ROADMAP.md`
  heading). **It is ~0.27 s**; ablating `paint` alone moves the total
  9.28 → 9.32 s, **inside noise**. Both older figures were **floor PLUS
  painting** — **`R164`, third instance in one day.** **The `76200e9` case
  is the instructive one: that entry's own ablation table labels the row
  "`intersect_clip` skipped entirely"**, which is by construction the whole
  render minus clips. **The number was right; the sentence attached to it
  was wrong, and the correct reading was printed three lines below the
  incorrect one for four filings.** So *"0.87 s is the answer to how fast
  could this page possibly get"* **remains TRUE** (it claims a floor, and a
  clips-off total is a floor) while *"painting costs 0.87 s"* is **false**.
  **Consequence: tiling and threading address under 3%, not 5%** — and the
  twenty-fourth entry already showed they cannot reach below the floor at
  all, so the case against them is over-determined.

  **★★ THE FINDING THAT CHANGES THE PLAN: THE PER-CLIP DISTRIBUTION IS
  UNIFORM.** 36 clips below 256 µs (**0.15%**) · **20,512 in 256–512 µs
  (85.0%)** · 3,472 in 512–1024 µs (14.4%) · **108 above 1024 µs (0.4%)**.
  p50 under 512 µs; **p90 and p99 both under 1024 µs**; **99.85% of all
  24,128 clips sit inside a single 4× band.**

  > **This is not 24,000 cheap clips plus 128 catastrophic ones. There is no
  > tail, and no head. THEREFORE THERE IS NO PATHOLOGICAL SPECIAL CASE TO
  > FIND AND FIX, AND ANYTHING THAT HELPS MUST CHANGE THE WORK DONE FOR ALL
  > 24,128 CLIPS.**

  **That is a standing constraint on every future proposal for this crate**,
  and it is why the live candidate is deduplication rather than a fast path:
  **a fast path's target population does not exist.** Note the pattern —
  `ROADMAP.md`'s items `1` (rectangle special-case) and `1′` (clip-sized
  mask) were **both special-case proposals, and both were killed by a
  census rather than by an implementation attempt.** This is the third
  census, and it kills the **category** rather than one member of it. **A
  distribution is a cheaper thing to measure than any of the three
  optimisations it forecloses.**

  **★★ THE SCALING LAW, AND IT REPLACES A HAND-WAVE WITH A MECHANISM.** Per
  4× pixels: `Mask::new` **4.3×, 7.9×** (superlinear) · **`fill_path` 1.98×,
  2.11×** · the multiply **4.0×, 4.4×** (area-bound).

  > **`fill_path` grows ~2× per 4× pixels. It tracks the LINEAR dimension,
  > because the scanline converter's cost follows the path's PERIMETER and
  > the number of scanlines it spans — not the area of the buffer.**

  **This is why it dominates at every scale, and at 0.25× it is still 56% of
  the entire render.** The twenty-fourth entry asserted that culling and
  low-resolution proxies underdeliver and supported it with a *total*; this
  supplies the **law** that makes the assertion predictive rather than
  observed. **Anything in these documents still resting on the hand-waved
  version is corrected by this entry.**

  **⚠ ONE FIGURE IS UNRECONCILED AND IS FLAGGED RATHER THAN SMOOTHED.**
  *"56% at 0.25×"* with `fill_path` = 1.25 s implies a 0.25× total of
  ~**2.23 s**; the only 0.25× total on record is **2.57 s** (`fa17d54`),
  which supports *"a proxy is bounded below by ~2.6 s"*. **13% apart,
  outside the 5.8% spread measured on this machine.** Both may be right —
  different builds, possibly different repeat counts, and
  0.03 + 1.25 + 0.14 + floor 0.49 + painting ~0.27 = **2.18 s** supports the
  lower one — **but no denominator was stated, so neither is retired here.**
  **The engineer should state which 0.25× total stands.** The conclusion
  holds either way: **2.2 s and 2.6 s are both ~3.5× above the 0.67 s a
  naive pixel-count scaling predicts.**

  **★ METHOD — TIMED, NOT ABLATED, AND THAT IS THE LOAD-BEARING DECISION.**

  > **An ablation answers *"what stops happening"*, and removes other things
  > with it (`R164`). A timer removes nothing.**

  With **four figures corrected in one day and three of them ablation
  artifacts**, the choice of instrument is the substance of this commit, not
  a detail of it.

  **★ AND A MEASUREMENT POLICY INHERITS THE REGIME IT WAS WRITTEN FOR.**
  `profile.rs` carried a **blanket** refusal to time sub-phases — *"timer
  calls inside a loop that runs 148,517 times perturb the thing being
  measured"*. **That was correct for the regime it was written about: the
  per-paint loop, 148,517 iterations of sub-microsecond work.** **Clip
  construction is the opposite regime** — 24,128 iterations of **~360 µs**,
  where a ~25 ns timer is ~1e-4 of the measured quantity. **The policy was
  sound and its SCOPE was unstated**, so it read as universal and would have
  forbidden the one measurement that closed the arithmetic. **A blanket rule
  written inside an instrument is a rule about a regime; write the regime
  down or it will be applied to the wrong one.**

  **★ AN INSTRUMENT DEFECT THAT FIRED UNDER THE TOOL'S OWN RECOMMENDED
  SETTING.** Counters were reset **once per scale**, not **once per
  repeat**, so `--repeat 3` — the setting the tool itself recommends, and
  which the previous entry added a warning to encourage — reported
  **445,551 paints and 72,384 clips** instead of **148,517 and 24,128**.
  **The reason it went unnoticed is the finding: derived percentages
  survived it**, because numerator and denominator both scaled. **Wrong
  numbers sitting beside right ones, in one output block, with nothing to
  tell them apart.** And the compounding shape: *the advice given for
  accurate timings silently corrupted the content block* — a recommendation
  that improves one half of an output while corrupting the other is worse
  than either a plain defect or plain bad advice, because the improvement is
  what gets followed. Fixed in this commit.

  **★ THE INSTRUMENTATION OVERHEAD WAS MEASURED, NOT ASSERTED — AND ONE
  PAIR WOULD HAVE BEEN WRONG.** Three instrumented invocations at 1× gave
  **9.49 / 9.52 / 10.04 s** — a **5.8% spread** — against **9.28 s**
  un-instrumented, which is **2.2% from the instrumented best and therefore
  INSIDE the spread**. **So the honest claim is "not distinguishable from
  variance", not a percentage**; the arithmetic predicts ~1e-4, the
  measurement can only say *below this machine's noise*, and **the
  measurement is what stands.** **A single before/after pair would have read
  "6% overhead" (10.04 vs 9.28) and been wrong.** **That is `R166`'s
  cold-start lesson recurring in a different guise** — there a one-shot
  *ablation* inflated a delta by 58%; here a one-shot *comparison* invents
  an overhead that does not exist. **Same root: one measurement of a noisy
  quantity, treated as the quantity.** Cross-referenced deliberately,
  because the two look unrelated until they are set side by side. A doc
  comment written the same session claiming the harness *"reports the
  un-instrumented total beside the instrumented one"* was **false** and was
  corrected in place in `profile.rs`.

  **★ THE RECOMMENDATION, WITH ITS GUARD, FILED TOGETHER.** **Avoid
  rebuilding masks at all** — deduplicate/cache already-built clip masks,
  because uniform per-clip cost means that is the only lever reaching all
  24,128. **BUT: MEASURE HOW MANY OF THE 24,128 ARE RE-APPLICATIONS OF AN
  ALREADY-BUILT CLIP PATH BEFORE BUILDING ANYTHING.** If repetition is high
  this is a large win; **if it is low the idea dies exactly the way the
  rectangle premise did.** **This is `R166` applied PROSPECTIVELY, and the
  second time in one day a premise has been required to be censused before
  being built on.** Ranked after that census, and read as a shape rather
  than a queue because **the biggest number is the least reachable**:
  `Mask::new` (**11.8%**, superlinear, ~24 GB of memset at 1× — but masks
  are `Arc`'d since `4475fe6`, so **lifetime needs care**) · the multiply
  (**28.3%**, already bbox-bounded by `76200e9`, **limited headroom**) ·
  **`fill_path` itself (59.9% — hardest, because the cost sits inside
  tiny-skia's scanline converter rather than in pdfce code).**

  **★ NOTHING MINTED — THREE CANDIDATES WEIGHED, ALL THREE DECLINED.**
  **(C)** *"An experiment must vary the dimension the hypothesis is
  about."* **The closest call.** A genuinely new failure shape, distinct
  from `R164` and `R166`, and **no mechanical carrier can check it**, so
  `R163` does not dispose of it. **It fails on the two-occurrence bar** —
  the same bar candidate (B) was declined on one entry ago, and declining
  (C) on it is what keeps the bar meaningful. **The second-occurrence
  trigger is named so the next filing need not re-derive it: any future
  figure refuted by re-running its own experiment with a DIFFERENT variable
  held fixed. If that happens, mint it.** **(D)** *"A measurement policy
  inherits the regime it was written for."* One occurrence, and it is advice
  about writing doc comments — craft, not a condition on care. **(E)** *"A
  tool's own recommended setting must be exercised by its own tests."*
  **`R163` is decisive** — the carrier is a test asserting the counts at
  `--repeat 3` equal those at `--repeat 1`, so the rule would be redundant
  the day it was written; **filed as owed tooling instead.** All three are
  recorded so a future filing starts from the argument. **`R167` remains
  free.**

  **★ STILL OWED, AND ONE ITEM IS A FALSE SENTENCE IN THE SHIPPED CRATE.**
  The corrected doc comment was fixed in **one of two places**. The claim
  that *"`render-profile` prints the un-instrumented total beside it so the
  overhead is shown, not argued"* **survives at `110b8c9` in
  `crates/pdfce-render/src/interpret.rs`, lines 2181–2183.** Established by
  reading **committed blobs**, not the working tree:
  `git show 110b8c9:tools/render-profile/src/main.rs` contains **no
  occurrence of "instrument" at all**, and none of its ~60 `println!` sites
  emits such a total; and `profile.rs`'s `timing_enabled()` is
  **`cfg!(feature = "profile")`, a compile-time constant**, so **a single
  invocation cannot produce both an instrumented and an un-instrumented
  total** — the 9.28 s figure came from a separately-built binary. **The
  surviving sentence is not merely stale; it is structurally impossible, and
  it is in the shipped crate rather than in the out-of-tree tool.** **Not
  fixed here** — this filing's scope is `docs/` only and a fork is live in
  `crates/`. Also owed: the `--repeat` regression test named in (E), and a
  ruling on which 0.25× total stands.

  **What this filing did NOT establish.** Every performance number above is
  the **operator's/engineer's**, measured at `110b8c9` and relayed (R87) —
  the phase table, the histogram, the scale sweep, the three instrumented
  invocations, and the gates (`cargo test` **2157 passed / 0 failed**,
  `clippy` **0**). **No render, no build and no test was run here.** The
  measurement input remains a **MEASUREMENT INPUT, NOT A FIXTURE** —
  outside the repository tree, untracked, inadmissible under rule 7 /
  `LEGAL.md` §5 — so **every number is reproducible only on the operator's
  machine, though by a committed instrument.** `cargo tree` was **not**
  re-run: `110b8c9` touches **three files and no manifest**
  (`git show --stat 110b8c9`), so no dependency changed and the
  GUI-core separation invariant is untouched.

  **Git and backup state — CHECKED, not inferred (hard rule 8 as amended,
  `b1368ed`).** `git rev-parse HEAD` → **`110b8c9`**;
  `git status --porcelain` → **two lines**,
  `crates/pdfce-core/src/graph.rs` modified and
  `crates/pdfce-render/src/cancel.rs` untracked. **AN ENGINEERING FORK IS
  LIVE in `crates/pdfce-gui`, `pdfce-core` and `pdfce-render`, building
  off-thread rasterization, exactly as the dispatch stated — and unlike the
  previous two filings the dirty state was visible at FIRST look.** **This
  entry describes `110b8c9` and nothing beyond it (R87)**, established from
  committed blobs. `git remote -v` → **empty**; the bundle is still the only
  copy. The newest bundle is **`pdfce-20260807-1635.bundle`** — **the
  filename the dispatch named, and this time it exists** — with
  `refs/heads/pass-8-redaction` at **`1f63aff`**, **one commit behind
  `HEAD`** (`git bundle list-heads` + `git log --oneline 1f63aff..HEAD`).
  **`110b8c9`, the commit this entry is about, is in NO bundle**, and
  neither are this filing's `docs/` edits. **A tip matching `HEAD` would
  still say nothing about uncommitted work: a bundle captures committed
  history only, and there is uncommitted work in `crates/` right now.**

  **This filing edited `docs/` ONLY** — no `crates/`, no `tools/`, no
  `fixtures/`, by the dispatch's explicit scope.

- **2026-08-07 (twenty-sixth entry this day) — A RENDER CAN BE STOPPED, AND
  STOPPING IT STOPS THE WORK. `ObjectGraph` GAINS `Send + Sync` AND
  `pdfce-render` GAINS `RenderCancel` — LAYERS 1 AND 2 OF OFF-THREAD
  RASTERIZATION, WITH LAYER 3 DELIBERATELY CUT, SO THE GUI STILL FREEZES.
  ★ THE FIRST OF THE RECENT NO-ID COMMITS THAT ACTUALLY CHANGES
  `pdfce-core`'s PUBLIC API. ★★ A GREEN TEST SURVIVED ITS OWN FEATURE BEING
  DISABLED — 32× SLOWER, SAME ASSERTION, SAME RESULT — SO THE LOAD-BEARING
  ASSERTION MOVED TO PIXELS.** — **[★ HEADING AMENDED 2026-08-07, twenty-
  seventh entry (`7926a78`): THIS COMMIT IS NOW `Pass 44.0`, retroactively.
  *"LAYER 3 DELIBERATELY CUT, SO THE GUI STILL FREEZES"* is TRUE OF THIS
  COMMIT and NO LONGER TRUE OF THE PROJECT — layer 3 landed and the freeze
  is gone. The *"NOTHING MINTED"* sentence at the end of this entry is
  likewise true of that filing and is NOT a live ceiling claim; the live
  Pass ceiling is family **44**. Heading otherwise left as filed.]** —
  `e4256f2` — `crates/pdfce-core/src/graph.rs`,
  `crates/pdfce-render/{cancel.rs,lib.rs,interpret.rs,annot.rs,font/mod.rs}`,
  and `crates/pdfce-render/tests/cancel_stops_the_work.rs`. **7 files,
  +477 / −4.** **Full API description: §4.1 subsection (K)**, added this
  filing, because the public surface changed and §4.1 is the living truth.

  **The decisions, as decisions.**

  **(1) Take the `Send + Sync` bound NOW, on a timing argument.** A
  supertrait on a public trait is breaking for downstream implementors.
  **With no remote and no release, the affected implementor set is exactly
  this repository's, and it is empty.** That is the cheapest the bound will
  ever be, and the cost grows monotonically. Only the **trait object** was
  ever the obstacle — every implementor was already thread-safe, and
  `&dyn ObjectGraph` erased that. **Verified by compile probe before and
  after** (R87: the argument predicts, the probe establishes), rationale on
  the trait per **`C-SEND-SYNC`**.

  **(2) `RenderCancel` is an `Arc<AtomicBool>` — no runtime, no executor.**
  §3 holds (`cargo tree`: **0 GUI matches** for core and render) and it
  works unchanged under wasm. **`RenderOptions.cancel` defaults to `None`
  as a safety property**: the CLI, the round-trip oracle and the R85
  harness **cannot acquire a new failure mode from the field existing.**

  **(3) The edit-collision shape is RULED — cancel, wait, mutate.** One
  choke point, **28.9 ms measured against ~58 s** for blocking. Rejected:
  snapshotting (`EditSession` is not `Clone`; needs a new public deep-copy
  to avoid a 28.9 ms wait) and `Arc::get_mut` at ~~~40~~ **[★ CORRECTED
  2026-08-07, twenty-seventh entry (`7926a78`): 51 — static pre-count 46,
  compiler borrow errors 49, sites actually moved 51; 51 mutating + 45
  read-only = 96 total, 53%. The ruling is unaffected; the figure was
  relayed and wrong, the fifth such today.]** sites (wrong shape —
  spreads concurrency across the mutation surface to serialise what one
  flag already serialises). **The number is what makes it a ruling rather
  than a judgement call.**

  **★★ (4) THE TEST FINDING, which is the durable part.** Disabling the
  `break` left `a_pre_cancelled_render_returns_cancelled` **passing
  identically** — same `Some(Cancelled)` — while the render took
  **10,227 ms instead of 322 ms**. `lib.rs` checks the flag **after** the
  loop, so the error variant is right whether or not any work was skipped;
  **three of the four tests had this property.** The assertion therefore
  moved to **PIXELS** — a cancelled render leaves the paper blank, with a
  **companion assertion that the fixture paints when NOT cancelled**,
  without which the claim is trivially true of an empty page. Verified
  failing without the `break` and passing with it. **Pixels rather than
  elapsed time DELIBERATELY: a timing assertion needs a page slow enough to
  clear the noise, and the only such page is an UNCOMMITTED benchmark file,
  so the test would silently not run on a fresh clone.** That is **`R162`
  and the fixture-provenance rule (`CLAUDE.md` rule 7 / `LEGAL.md` §5)
  applied together** — and the pixel assertion is **stronger**, not a
  compromise: it tests what the feature claims to do rather than a proxy.

  **(5) One doc comment deleted — the single-location-amendment failure
  INSIDE a crate.** The claim that `render-profile` *"prints the
  un-instrumented total beside it"* is **unimplementable, not stale**
  (`timing_enabled()` is `cfg!(...)`, a compile-time constant), and **the
  same sentence had already been corrected in `profile.rs`.** This
  **discharges the first of the twenty-second filing's two open engineer
  rulings**; the second — **which 0.25× total stands, 2.57 s or ~2.23 s** —
  **remains open.**

  **NOTHING MINTED — but this one carries a Pass-ID ARGUMENT rather than a
  Pass-ID refusal**, unlike `110b8c9`/`fa17d54`/`6b33789`, whose shared
  justification (*"out-of-tree tool plus an off-by-default feature flag; no
  shipped behaviour changes"*) **does not reach this commit**: the public
  API changed. Against: **no operator-visible capability changed**, and a
  Pass with the acceptance criterion *"the GUI no longer freezes"* would be
  **failed by this commit**. **Librarian's non-binding reading: one ID
  covering all three layers, minted when layer 3 lands, `e4256f2` recorded
  against it retroactively** (the Pass 20.2 PARTIAL→COMPLETE shape). **The
  ID is the engineer's.** Ceilings unchanged and **re-measured by running**
  `tools/check-ledger-numbers.py` and `tools/check-passes-filed.py` (exit
  codes in the *Shipped* entry): Pass **43**, **R166** (R167 next free),
  decision **031** (032 next free), operator question **(bb)**.

  **★ LEDGER CORRECTION FILED THIS ENTRY — and the correction to it was
  ALSO wrong.** The twenty-second filing declared **four** RAG findings owed
  and **none written**; **one was already on disk when that sentence was
  committed.** The dispatched correction then said **two** were already
  written, *"at 16:37 and 16:38"*; **the directory says one.** Established
  by **`stat -c '%w %n' *.md` across `D:\dev\rag\rust\`, `D:\dev\rag\egui\`
  and `C:\personal_rag\pdf\`** (NTFS creation times) against
  **`git log --format=%cI 78ca1bf` → `17:06:24`**:
  `name_the_null_result_…` born **16:38:35** (28 min BEFORE — the ledger was
  false); `one_shot_ablation_…` born **17:18:02** (12 min AFTER — the ledger
  was true and the correction false); `an_experiment_must_vary_…`
  **17:15:56** and `counters_reset_per_outer_loop_…` **17:16:37**. **`16:37`
  is no file's birth time in any of the three trees.** **All four are now
  written and indexed; the debt is closed.** **Limits, because they bound
  the claim:** `%w` survives in-place overwrite and rename, so a 17:18 birth
  means first creation or delete-then-recreate; and **`D:\dev\rag\` is not a
  git repository**, so the filesystem is the only witness. **The
  transferable half — *naming the failure does not perform the check; a
  correction is a claim and must name its world-source* — is filed as
  instance (4) on the existing RAG finding
  `a_record_can_carry_its_own_refutation_…`, and its two practical
  conventions are ADOPTED** into `ROADMAP.md`'s *Update protocol* (*"How a
  figure is filed"*) and `.claude/agents/pdfce-librarian.md` **hard rule
  10**: *file a total beside its per-item form*, and *put the qualifier in
  the table label*. **NO CHECKER WAS BUILT, deliberately** — extraction is
  the whole job, cross-configuration comparison manufactures contradictions,
  the identity set is not enumerable in advance, and a checker commissions
  work rather than care. **The conventions are the carrier.**

  **What this filing did NOT establish.** Every gate and every performance
  figure above is the **engineer's**, measured at `e4256f2` and relayed
  (**R87**): `cargo test` **2166 / 0 failed**, `clippy` **0**, `cargo tree`
  **0 GUI matches** for core and render, **R85 24 cases green**, and the
  322 / 10,367 / 28.9 ms measurements. **No build, no render and no test was
  run here.** The benchmark page remains a **MEASUREMENT INPUT, NOT A
  FIXTURE** — outside the tree, untracked, inadmissible under rule 7 /
  `LEGAL.md` §5 — which is precisely the constraint that forced (4)'s pixel
  assertion.

  **Git and backup state — CHECKED, not inferred (hard rule 8 as amended,
  `b1368ed`).** `git rev-parse HEAD` → **`e4256f2`**. `git remote -v` →
  **empty**; bundles remain the only copy. `git status --porcelain` →
  **three lines, all in `crates/pdfce-gui`** (`main.rs`, `raster.rs`
  modified; `render_worker.rs` untracked) — **the live layer-3 fork, as the
  dispatch stated.** Newest bundle (`ls D:\Dev\pdfce-backups\`):
  **`pdfce-20260807-1706.bundle`**, `refs/heads/pass-8-redaction` at
  **`78ca1bf`** — **one commit behind `HEAD`** (`git bundle list-heads` +
  `git log --oneline 78ca1bf..HEAD`, which returns exactly `e4256f2`).
  **`e4256f2` is in NO bundle**, and neither is the live fork — a bundle
  captures committed history only.

  **This filing edited `docs/` and `.claude/agents/pdfce-librarian.md`
  only** — the agent file by the dispatch's explicit permission, for hard
  rule 10. No `crates/`, no `tools/`, no `fixtures/`.

- **2026-08-07 (twenty-seventh entry this day) — THE RENDER MOVES TO A
  WORKER, AND THE WINDOW STOPS DYING WITH IT. LAYER 3 LANDS, AND `Pass 44.0`
  IS MINTED OVER ALL THREE LAYERS WITH `e4256f2` RECORDED RETROACTIVELY.
  ★★ THE HEADLINE IS A LIVELOCK FOUND BY REASONING BEFORE IT COULD BE
  OBSERVED — THE FIX FOR THE FREEZE WOULD OTHERWISE HAVE SHIPPED A WORSE
  FREEZE WEARING THE SAME FACE. ★ THE "~40 SITES" FIGURE IN THE TWENTY-SIXTH
  ENTRY IS WRONG; IT IS 51 — THE FIFTH WRONG RELAYED FIGURE IN ONE DAY.**
  `7926a78` — `crates/pdfce-gui/{main.rs,raster.rs,render_worker.rs,ui_text.rs}`.
  **4 files, +840 / −132 = net +708**, of which `render_worker.rs` is
  **+503 / −0 (59.9% of insertions, whole file new)** and `raster.rs` is the
  only file that **shrank** (+29 / −42, the dead synchronous path leaving).
  **`pdfce-gui` ONLY — no core, no render, no manifest**, so §4.1 (K)
  remains the complete API record and §3's invariant is untouched.
  **Body sections updated in this same filing: §3's `pdfce-gui` block**
  (threading lives in the shell and only in the shell) **and §4.1 (K)'s
  amendment block** (the ID, the freeze, and the 51).

  **The decisions, as decisions.**

  **★★ (1) `RenderKey` EXISTS BECAUSE OF A LIVELOCK THAT WAS REASONED OUT,
  NOT OBSERVED — and this is the durable half of the commit.** The shell
  re-runs its staleness check **every frame**. While a render is in flight
  the texture **has not been replaced**, so the check keeps saying *stale*
  and keeps requesting the same render. **Without recognising that the
  running job IS the request being asked for again, every frame would cancel
  the previous render and start an identical one — and any page slower than
  ONE FRAME would never finish at all. Not slow: impossible.** It would have
  presented as **exactly the freeze this Pass exists to remove**. The first
  draft had no guard; **nothing observed the defect, and no test would have
  been written for it**, because the state it needs (a render outstanding
  across frames) did not exist until the same commit created it. **This is
  the "green is not evidence" family pointed one step earlier than
  `e4256f2`'s test finding.**

  **(2) A BOUNDED 12 ms IN-FRAME WAIT, AND IT IS VERIFIED NOT FREE.**
  `IN_FRAME_BUDGET` = **12 ms**, deliberately under one 16.7 ms frame at
  60 Hz (**72%**), so the worst-case wait **cannot itself drop a frame**. A
  page that beats it returns pixels **inline** and never touches the async
  path. **Without it, EVERY render costs a frame of staleness** — including
  microsecond ones. **That is how "nothing regresses when fast" is
  satisfied: BY CONSTRUCTION, not by hoping the `ZOOM_SETTLE` debounce
  covers it** (a debounce delays a *request*; it does nothing about a result
  arriving a frame after it was ready). Scale: 12 ms against a 10,000 ms CAD
  render is **0.12%** — a page must be ~**830×** faster than that sheet to
  return inline. **This is the ONE place the UI thread blocks on rendering,
  it is bounded by a constant, and the bound is the point.**

  **(3) SINGLE-SLOT WORKER, NOT A QUEUE.** A second concurrent render is
  **always a superseded first one** — the shell only ever wants the picture
  it wants *now*. **A queue would mean choosing which stale result to
  paint**, a question with no good answer and no caller who wants it asked.

  **(4) CANCEL BEFORE SPAWN, NOT AFTER.** Two CAD rasterizations competing
  for cores **make both slower**, and the old output **is already unwanted**.
  This is also why `e4256f2`'s cancellation had to **stop work** rather than
  discard a result: a cancelled render that ran to completion would turn
  cancel-before-spawn into *blocking* wearing cancellation's name.

  **(5) THE EDIT COLLISION IS IMPLEMENTED EXACTLY AS THE TWENTY-SIXTH ENTRY
  RULED IT — cancel, join, mutate, at ONE choke point** (`session_mut`).
  **`Arc::get_mut` is therefore INFALLIBLE BY CONSTRUCTION, so the `expect`
  is HONEST rather than OPTIMISTIC**: the worker is the only other holder of
  the `Arc` and has been joined two statements above. **58,000 / 28.9 =
  2,007× — three orders of magnitude** against blocking, which is why this
  was a ruling and not a preference. **★ CORRECTION TO THAT ENTRY: the
  rejected `Arc::get_mut` alternative was filed at "~40 sites"; it is
  51.** Static pre-count **46 (5 short, 90%)**; compiler borrow errors
  **49 (2 short, 96% — the two are in test code `cargo build` does not
  reach)**; **sites actually moved 51**. **51 mutating + 45 read-only = 96
  total `session` call sites; 51 / 96 = 53%** (reads needed no change —
  `Deref` still yields `&EditSession`). **This is the FIFTH wrong relayed
  figure in one day** (after `Mask::new` 10.1 s → 1.02 s; clip bbox 0.663% →
  66.36%; `fill_path` 8–10 µs → 216 µs; painting 0.87 s → ~0.27 s) and the
  **mildest — 28% low, not 22× or 100×** — filed at the same weight anyway,
  because **the failure mode is the FREQUENCY, not the magnitude**. **What
  broke the streak is METHOD: it let the compiler count, then checked the
  compiler's own blind spot.**

  **★ (6) THE STALENESS DISCLOSURE IS DECIDED AND SPLIT BY CASE — recorded
  because TWO forks deferred it.** **A zoom already discloses itself:**
  `TextureOptions::LINEAR` makes the scaled previous texture read as
  **soft** — visibly not final, **and free**. **An edit does not:** the page
  renders **sharp and simply wrong**, and nothing distinguishes that from
  the edit having **failed**. So the **status bar** says the canvas is
  behind — **gated at 150 ms** (`CANVAS_BEHIND_NOTICE_AFTER = ZOOM_SETTLE`,
  **9 frames at 60 Hz**, so it cannot flicker), **worded as a fact about the
  PICTURE rather than as progress** (the operator's question is *"did my
  edit take?"*, and the answer is **yes — the drawing has not caught up**;
  *"Rendering…"* answers a question nobody asked), and **fixed position,
  never page-relative — decision 024 §4.4**, which exists because the
  operator objected to controls whose position derived from the document.
  **This is rule 4 applied to a PICTURE rather than to a value**, and §4.4's
  narrowing is exactly what keeps it a status line and not a confirm button:
  nothing here was **inferred**, so nothing needs accepting — what is owed
  is **disclosure**.

  **(7) `R162` DISCHARGED, AND THE FIRST ATTEMPT WAS TOO WEAK.** Removing
  the `PartialEq` **derive** is a **compile error** — which proves the
  derive is **needed**, not that the comparison is **right**. The probe that
  works is a **hand-written `PartialEq` ignoring `raster_scale`**: it
  **compiles**, and fails **exactly 2 of the module's 4 tests (50%)** — on
  the assertion labelled *"raster scale must be compared"* and on the
  one-bit-scale test — **leaving the other two green**, which is the point:
  those two are blind to this defect by construction. **Why the field
  matters both ways:** a key ignoring scale would make the livelock guard
  **swallow a genuine new request**, so changing the zoom would stop
  re-rendering entirely — **the same field prevents one hang and causes a
  different one if compared wrongly**, and only a compilable probe tells
  them apart. **`R162`'s restated lesson: the probe must be able to
  COMPILE. A mutation the type system refuses is a dependency check, not a
  mutation.**

  **(8) TWO SMALLER ITEMS.** `tools/check-ui-strings.sh` **caught a
  worker-failure literal**, now `ui_text::canvas_render_worker_stopped()` —
  and the choice it encodes is deliberate: **a render FAILURE rather than
  silence**, because a canvas waiting forever for a message that will never
  arrive **presents as a hang**, the exact failure this Pass removes. And
  the dead synchronous `render_page_texture` was **REMOVED rather than left
  with a stale justification** — **the same lesson `e4256f2` filed one
  commit earlier** about a doc comment surviving in a second location; a
  dead function with a correct-sounding comment is that failure with a
  longer fuse.

  **⚠ ONE VERIFICATION IS OWED, AND IT IS NOT A FORMALITY: THE RENDERED
  BEHAVIOUR IS UNVERIFIED BY SCREENSHOT.** The operator was at the machine
  and `gui-shot.ps1` **takes the foreground**, so a capture would have
  photographed his desktop. **Three defects on this date were caught only by
  looking**, and every one was green under `cargo test` while wrong. **A
  status line that never appears, appears at the wrong moment, or appears
  and never clears is precisely that class.** Owed under `Pass 44.0`,
  alongside Pass 20.5's own outstanding rendered-appearance check — **two
  now, and the same kind.**

  **TWO ITEMS CARRIED, NEITHER RETIRED.** **(a) Which 0.25× total stands —
  2.57 s or ~2.23 s — STILL OPEN**; it is the second of the twenty-second
  filing's two engineer rulings, the first having been discharged by
  `e4256f2`. **(b) Whether a sharp-but-stale canvas deserves more than a
  status line** — judged a **DESIGN** question rather than an engineering
  one by the fork, **concurred by the engineer and by this filing**, and
  routed to **`pdfce-ui-specialist`**. **The status line ships meanwhile, so
  the gap is a REFINEMENT, NOT A HOLE** — nothing is undisclosed today; what
  is open is whether *telling* is the best treatment, not whether the
  obligation is met.

  **`Pass 44.0` IS MINTED, AND THE RESTRAINT IS WHY IT MEANS SOMETHING.**
  The three no-ID entries below `e4256f2` (`110b8c9`, `fa17d54`, `6b33789`)
  share one justification — *"an out-of-tree tool plus an off-by-default
  feature flag; no shipped behaviour changes"* — **true of all three, and
  FALSE here**: `e4256f2` changed `pdfce-core`'s **public API** and
  `7926a78` changed **the GUI's behaviour**. **Four consecutive commits were
  weighed against the same test; three were refused.** The fourth being
  minted is therefore **a statement about this commit, not a loosening of
  the bar** — the refusals are the calibration, and the precedent must not
  be read as inconsistency. **Ceilings, re-measured by RUNNING
  `tools/check-ledger-numbers.py` (exit 0) and `tools/check-passes-filed.py`
  (exit 0):** Pass family **44** (highest ID **44.0**), standing rules
  **R166** (**R167** next free), decision records **031** (**032** next
  free), operator questions **(bb)**. **Nothing but the Pass ID is minted.**

  **What this filing did NOT establish.** Every gate above is the
  **engineer's**, measured at `7926a78` and relayed (**R87**): `cargo test`
  **2170 passed / 0 failed** (**+4 over `e4256f2`'s 2166 — all four are
  `render_worker`'s own tests, 4 of 4 new tests in 1 new module**), `clippy`
  **0**, `cargo tree` **0 GUI matches for `pdfce-core`**, working tree
  **clean**. **No build, no render and no test was run here.** The
  livelock, the 12 ms budget's necessity, the 150 ms gate and the disclosure
  wording were **read from the committed source and its doc comments**, not
  observed running. The benchmark page remains a **MEASUREMENT INPUT, NOT A
  FIXTURE** — outside the tree, untracked, inadmissible under rule 7 /
  `LEGAL.md` §5.

  **Git and backup state — CHECKED, not inferred (hard rule 8 as amended,
  `b1368ed`).** `git rev-parse HEAD` → **`cf656a8`**. `git remote -v` →
  **empty**; bundles remain the only copy. `git status --porcelain` → **0
  lines** — **and the dispatch's own framing is adopted rather than
  improved on: that is a SNAPSHOT, not a standing fact**, true when looked
  at and not thereafter.

  **★★ AND THE SNAPSHOT EXPIRED DURING THIS FILING — WHICH IS THE POINT,
  DEMONSTRATED RATHER THAN ARGUED.** `git status --porcelain` re-run at the
  END of the filing returns **7 lines**: this filing's **4 `docs/` files**,
  plus **3 files this filing did not touch** —
  `crates/pdfce-render/src/interpret.rs`,
  `crates/pdfce-render/src/profile.rs` and
  `tools/render-profile/src/main.rs`. **A fork went live in `crates/` and
  `tools/` while these documents were being written.** `HEAD` is unchanged
  at `cf656a8`, so **nothing described in this entry is affected** — every
  claim here is about committed blobs. **Recorded rather than quietly
  updated**, because the whole content of hard rule 8's amendment is that a
  checked fact decays: the clean tree was **checked, reported, and then
  false**, inside one filing. **The fix is not to check harder; it is to
  say WHEN.** Newest bundle (`ls D:\Dev\pdfce-backups\`, filename
  READ rather than composed): **`pdfce-20260807-1736.bundle`**, with
  `refs/heads/pass-8-redaction` at **`cf656a8`** (`git bundle list-heads`)
  — **EQUAL TO `HEAD`, so `7926a78` IS IN THE BUNDLE**, which is the first
  filing this day able to say so. **This filing's own `docs/` edits are
  not**, and a matching tip says nothing about uncommitted work: a bundle
  captures committed history only.

  **This filing edited `docs/` ONLY** — no `crates/`, no `tools/`, no
  `fixtures/`, no agent files, by the dispatch's explicit scope.

- **2026-08-07 (twenty-eighth entry this day) — ONE CLIP PATH IS 97.3% OF
  THE WORK AND IT IS REBUILT EVERY TIME. THE THIRD OPTIMISATION PREMISE
  CENSUSED BEFORE BEING BUILT ON, AND THE FIRST TO SURVIVE. ★★ THE MEAN HID
  THE SHAPE (603.20 APPLICATIONS PER PATH READS AS BROAD REUSE; IT IS ONE
  PATH AND 37 SINGLETONS), AND THE HISTOGRAM COULD NOT HAVE REVEALED IT
  BECAUSE ITS LAST BUCKET IS UNBOUNDED. ★★ A HIT CAN SHARE THE `Arc`, SO IT
  IS WORTH THE WHOLE 361 µs, NOT THE 259 µs A BUILD-ONLY CACHE WOULD SAVE.
  ⚠ THE ~10 s → ~1.7 s PROJECTION IS ARITHMETIC, NOT A MEASUREMENT.**
  `1992d13` — `crates/pdfce-render/{profile.rs,interpret.rs}`,
  `tools/render-profile/src/main.rs`. **3 files, +374 / −0**, of which
  `profile.rs` is **+255 (68.2%)**, the harness **+107 (28.6%)** and the
  rasterizer **+12 (3.2%) — one feature-gated call.** **No Pass ID** — the
  fourth consecutive application of the *out-of-tree tool plus an
  off-by-default feature flag* precedent (`110b8c9`, `fa17d54`, `6b33789`),
  and the restraint `Pass 44.0`'s entry names as the calibration that makes
  that ID mean something. **No body section of this document changes**: no
  public API moved (§4.1 stands), no crate boundary moved (§3 stands),
  nothing in `pdfce-core` was touched, and the render invariant is
  untouched — **the commit adds counters, not behaviour.**

  **The decisions, as decisions.**

  **★★ (1) IDENTITY IS THE *BUILD* KEY, AND THE INCOMING CLIP IS
  DELIBERATELY EXCLUDED FROM IT.** The key is *path verbs + points, fill
  rule, CTM, mask dimensions* — the tuple that determines the freshly filled
  mask **before** intersection. Including the clip already in force would
  measure the **intersected** result, which **chains**: identical paths under
  different accumulated clips give different final masks, so a key that
  included it would answer a question about history rather than about what a
  cache of `Mask::new` + `fill_path` can serve. **The exclusion is what makes
  the number an upper bound on addressable repetition rather than an
  estimate of a particular cache's hit rate.**

  **★★ (2) A SECOND KEY *INCLUDING* THE INCOMING CLIP WAS TAKEN ANYWAY, AND
  IT ENLARGED THE PRIZE RATHER THAN QUALIFYING IT.** It returned **40
  distinct (path, incoming clip) pairs — identical to the 40 build keys.**
  Every re-application therefore happens under the **same** incoming clip,
  the **final** mask is identical, and a hit can **share the existing
  `Arc`**: no allocation, no copy, no `fill_path`, no multiply. **The whole
  361 µs per clip (8.72 s ÷ 24,128), not the 259 µs (6.25 s ÷ 24,128 =
  72%)** a build-only cache would save; the remaining **102 µs (2.46 s ÷
  24,128)** is the multiply, and **259 + 102 = 361**, which closes against
  the twenty-fifth entry's timed phases by division rather than by new
  measurement. **The reusable shape: a disambiguating second key can turn out
  to be the more valuable measurement, so take it even when the first key
  already answers the question you asked.**

  **★★ (3) THE MEAN WAS THE TRAP, AND SO WAS THE HISTOGRAM.** **24,128
  applications over 40 distinct paths = 603.20 per path; 24,088 repeats =
  99.83%.** But **top-1 = 97.3%, top-2 = 99.8%, and 37 of 40 paths are
  applied exactly once** — so **a 2-entry cache serves 99.8% for ~1.9 MiB**
  (38.3 MiB ÷ 40 = 0.958 MiB per mask; × 2 = 1.92 MiB), where the mean
  implies a 40-entry 38.3 MiB structure. **Those are different engineering
  problems and the mean is compatible with both.** `clip_reuse_hist` could
  not have separated them either: **its final bucket is `65+`, so *"2 paths
  applied 65 or more times"* is equally consistent with 130 applications and
  with 24,000** — a 185× span, and the span **is** the question. **That is
  why `clip_top_counts` carries the raw top-8 counts beside the histogram,
  and the counter's own doc comment says so.** **A bucketed distribution with
  an open final bucket cannot answer a concentration question.**

  **★★ (4) THE METHODOLOGY WAS STACKED AGAINST THE ANSWER IT RETURNED, AND
  THAT IS WHY THE ANSWER IS LOAD-BEARING.** Coordinates compare
  **bit-exactly** (`f32::to_bits`) because two points differing in the last
  ulp produce different coverage, and **treating them as equal would
  OVERSTATE repetition — the direction that would wrongly justify building.**
  Incoming-clip identity is the **`Arc` pointer**, **stricter** than value
  equality, so it **UNDERSTATES** hits and **cannot manufacture one**.
  **Both choices point away from the conclusion the fork reached**, which
  makes **99.83% and 40 LOWER BOUNDS**, not estimates. **A census whose
  tolerances are tuned toward its hypothesis proves nothing; one tuned
  against it and still returning 99.83% has answered the objection before it
  was raised.** *(This was weighed as a standing-rule candidate and
  DECLINED — see the `1992d13` ROADMAP entry §7. `R167` stays free and the
  candidate is recommended for re-put on its next independent instance.)*

  **(5) `q`/`Q` WAS CHECKED AND DOES NOT ALREADY SOLVE IT — the question was
  asked, not assumed away.** Since `4475fe6` made `GraphicsState.clip` an
  `Arc<Mask>`, a `Q` **restore** is free. But `q`/`Q` dedupes **restores**
  and **no rebuilds**: **every `W`/`W*` operator calls `intersect_clip`
  regardless**, so all 24,128 applications are genuine, unavoided
  `Mask::new` + `fill_path` + multiply. **The addressable repetition is the
  full naive rate, not a reduced remainder.** Recorded because it was the
  plausible reason the entire idea might have been redundant.

  **(6) THREE SELF-CHECKS, ALL RECORDED.** **(a)** A degenerate 64-bit key
  would present as one entry with 24,128 hits and **no singletons**; **37
  singletons** are the observable a collapsing hash destroys first. **(b)**
  Counts were **identical at `--repeat 1` and `--repeat 3`**, testing by name
  for the counter-reset defect found earlier the same day, which produced
  right-looking percentages beside wrong counts. **(c) ★ AN INDEPENDENT
  FIGURE FROM A DIFFERENT FILING CORROBORATES THIS ONE**: the mean
  **individual** clip bbox (**66.36%**) equals the mean **accumulated** bbox,
  a coincidence noted and unexplained in the nineteenth entry. **top-1 =
  97.3% explains it** — one dominant clip applied at or near base state makes
  the accumulated clip *be* that individual clip almost everywhere. **Two
  measurements taken days apart, by different code, for unrelated reasons,
  are consistent only under this census's answer.**

  **⚠ (7) THE PROJECTION IS FILED WITH ITS QUALIFICATION INSEPARABLE.**
  **Roughly 10 s → ~1.7 s at 1×**, made of **99.83%** (this census, one
  instrument) and **345.6 µs** mean clip cost (**8.34 s over 24,128**, the
  `fa17d54` ablation, one instrument) = **~8.33 s removed**, against 1×
  totals on record between **9.28 and 10.18 s** — a band the *"~1.7 s"* sits
  inside. **It is arithmetic over separately measured parts, NOT a
  measurement: no cache exists, nothing has been re-rendered, and nobody has
  observed this file render in 1.7 s.** **`R166` governs it** — it may be
  reported and may scope nothing. **Unmodelled second-order costs**, stated
  so it is not read as a floor: a hit still pays a hash and a map probe
  24,128 times; `Mask::new`'s memset disappears only if the cached `Arc`
  genuinely outlives its uses; and **the 40-key census is ONE FILE** —
  whether the shape generalises is not measured here and is not claimed.

  **(8) THE SEQUENCE IS THE FINDING, NOT THIS RESULT.** Three premises
  censused before being built on, same day, same file: **rectangles 2.5%
  (DECLINED, `4475fe6`)** · **clip bbox 66.36% not 0.663%, 100× wrong across
  four documents (RETIRED, `6b33789`)** · **repetition 99.83% (SURVIVES,
  `1992d13`)**. **The first two create the value of the third.** And the
  third was **blocked by name, in writing, before it was run** — the
  twenty-fifth entry's *"measure how many of the 24,128 are re-applications
  BEFORE building anything; if it is low, the idea dies exactly the way the
  rectangle premise did"*. **A prospective census requirement was written
  down, survived a filing, and was then actually performed.** The two prior
  censuses were run *after* their premises had been filed as fact.

  **★ (9) A GATE BLIND SPOT, FOUND BY RUNNING IT.**
  `tools/check-passes-filed.py` exited **1**, reporting `e7e74f2` (**the
  docs commit that FILED `Pass 44.0`**) as UNFILED. **Not a filing gap: the
  checker's join key is "this commit's short hash appears in
  `docs/ROADMAP.md`", and `e7e74f2` IS the commit that writes
  `docs/ROADMAP.md`** — it would have to contain its own hash. **A gate
  joining a commit to the document that commit edits is unsatisfiable by the
  editing commit and can only go green on the NEXT filing**, which is what
  this filing performed by citing `e7e74f2` in the `Pass 44.0` entry. It
  surfaced now only because that commit's subject was `Pass 44.0: …` rather
  than the customary `docs: …`. **The fix — an exemption for docs-only
  commits, or a convention that a filing commit is cited by the following
  filing — is a TOOLING RULING, flagged to the engineer, not taken by this
  filing.** Both checkers exit **0** afterwards.

  **Git and backup state — CHECKED, not inferred (hard rule 8 as amended,
  `b1368ed`), and TIMESTAMPED because a checked fact decays.** At the start
  of this filing: `git rev-parse HEAD` → **`1992d13`**;
  `git status --porcelain` → **0 lines**; `git remote -v` → **empty**,
  bundles remain the only copy. Newest bundle (`ls D:\Dev\pdfce-backups\`,
  filename READ rather than composed): **`pdfce-20260807-1751.bundle`**,
  `refs/heads/pass-8-redaction` at **`e7e74f2`** (`git bundle list-heads`) —
  **ONE COMMIT BEHIND `HEAD`** (`git log --oneline e7e74f2..HEAD` → one
  line). **`1992d13` is in NO bundle**: it was committed **17:56** and the
  bundle taken **17:51**. **The clean tree is a SNAPSHOT** — the dispatch
  states an engineering fork is live in `crates/pdfce-render` and
  `tools/render-profile` building the cache this census justifies, and the
  previous entry recorded a tree going dirty *during* a filing. **Every
  claim in this entry is read from committed blobs** (`git show 1992d13:…`,
  `git diff e7e74f2..1992d13`), never the working tree.

  **★★ AND THE SNAPSHOT EXPIRED AGAIN, INSIDE THIS FILING, FOR THE SECOND
  CONSECUTIVE TIME — WHICH MAKES IT A PATTERN RATHER THAN AN INCIDENT.**
  `git status --porcelain` re-run at the **end** returns **9 lines**: this
  filing's **4 `docs/` files**, plus **5 files it never touched** —
  `crates/pdfce-render/src/{interpret.rs,lib.rs,profile.rs}` and
  `tools/render-profile/src/main.rs` modified, and
  **`crates/pdfce-render/src/clip_cache.rs` UNTRACKED.** **The cache this
  census justifies is being written as this entry is filed**, and the new
  module's name says so plainly. `HEAD` is **unchanged at `1992d13`**, so
  **nothing in this entry is affected** — it describes committed blobs only.
  **Recorded rather than quietly corrected**, because the entire content of
  hard rule 8's amendment is that a checked fact decays: the clean tree was
  checked, reported, and then false, **within one filing, twice running.**
  **`clip_cache.rs` is on disk, in no commit, and in no bundle.**

  **This filing edited `docs/` ONLY** — no `crates/`, no `tools/`, no
  `fixtures/`, no agent files, by the dispatch's explicit scope. **THREE RAG
  findings are consequently OWED and NOT written** (the unbounded-final-bucket
  reporting lesson; the error-direction-in-identity-choice methodology; the
  self-referential gate join). **Their absence from `D:\dev\rag\rust\` was
  established by `ls` on the directory and `grep -ril` over it, not by
  consulting a ledger** — the last version of that ledger was wrong twice,
  and the correction to it was also wrong. Full table with nearest-neighbour
  files: §12 of the `1992d13` ROADMAP entry.

- **2026-08-07 (twenty-sixth filing, `ce57ed5` + `c3d8853`) — THE CLIP-MASK
  CACHE, `Pass 45.0`, AND A GATE THAT ASKED A COMMIT TO CONTAIN ITS OWN
  HASH.** Two commits: the render optimisation the three-premise census
  sequence was built to justify, and a ledger-gate fix.

  **(1) ★★ AN `Arc` PINS THE KEY, AND THAT IS THE ENTRY'S MOST IMPORTANT
  LINE.** `crates/pdfce-render/src/clip_cache.rs` keys the incoming clip by
  **pointer identity** (`Arc::as_ptr`) — stricter than value equality, so it
  can lose hits and cannot invent one. **A BARE POINTER WOULD BE UNSOUND:**
  drop the incoming mask, let a later allocation reuse the address, and a
  stale entry matches a pointer that now means something else — **ABA,
  returning the wrong clip and painting a silently wrong picture.** Each
  entry therefore holds a **strong `Arc` to the incoming mask**, pinning that
  address for as long as the entry can be matched. **No timing number would
  have shown this failure (a wrong-mask hit is FASTER), and none of the tests
  the dispatch specified exercise address reuse.** Same class as
  `6b33789`'s `RasterPipelineBlitter::new` returning `None` and dropping the
  paint: **wrong output that no assertion in the render path is looking at.**
  **The architectural rule this makes explicit: pointer-as-identity is a
  correct optimisation and an incorrect key — the difference is ownership.**

  **(2) ★★ WHAT IS CACHED IS THE MASK *AFTER* INTERSECTION**, so a hit skips
  `Mask::new`, `fill_path` **and** the multiply — **362 µs each, not the
  259 µs a build-only cache would save** (8.72 s vs 6.24 s over 24,087 hits).
  **Sound only because `1992d13`'s census measured BOTH identities and found
  40 of each** — build key (geometry + fill rule + CTM + mask size) and full
  key (build key + incoming clip). **That equality is a property of THIS
  document, not of PDF**; a file clipping one path under different
  accumulated clips gets **fewer** hits and **cannot get WRONG ones**,
  because the incoming clip is in the key.

  **(3) ★★ THE RESULT, TWO INSTRUMENTS, NEITHER RECONCILED AWAY.**
  ENGINEER, end to end (incl. process start + PNG encode): **1× 32,313 →
  907 ms (35.63×)**, **2× 447,862 → 1,425 ms (314.3×)**. FORK's
  `render-profile`, render phase only: **1× 10.68 → 0.79 s (13.52×)**, **2×
  58.52 → 1.30 s (45.02×)**. **35.6×/314× are DAY-CUMULATIVE over three
  fixes; 13.5×/45× are THIS COMMIT ALONE.** The after-figures differ by
  **117 ms at 1× and 125 ms at 2×** (derived here, not relayed) — near
  constant while pixels quadruple, consistent with **process start**
  dominating rather than encode. **Output BYTE-IDENTICAL: SHA-256
  `9250a89f…`, the same hash as the 32.3 s render this morning, plus an
  unchanged aggregate over 115 synthetic fixtures** (the CAD sheet has zero
  images and 242 text elements and cannot carry that claim alone).
  **2178 tests / 0 failed = 2170 + the module's exactly 8 `#[test]`
  functions.**

  **(4) ★★ THE RENDER IS FLOOR-BOUND, WHICH CHANGES WHAT `pdfce-render`
  OPTIMISATION MEANS FROM HERE.** Measured floor **0.49–0.53 s** (scale-flat
  over a 64× pixel span; **148,517 operators = 3.43 µs each**) against
  **0.79 s at 1×** — the floor is **62–67% of what remains**, and **the
  maximum conceivable further speedup at 1× is 1.55×**. Clip construction,
  86% of the render this morning, is now **41 rebuilt masks × 362 µs =
  14.8 ms = 1.9%**. **Any future work must attack the operator walk itself,
  and the headroom is small and known.**

  **(5) ★ THE HIT RATE IS THE CENSUS CEILING, TO THE UNIT.** **24,087 hits +
  41 builds = 24,128 = 99.830%** against the census ceiling **24,088 /
  24,128 = 99.834%**; **the one-unit shortfall IS the single eviction a
  4-slot cache makes over 40 distinct keys.** A predicted number confirmed to
  the unit means the **model** of the workload was right, not merely its
  magnitude.

  **(6) Bounding and lifetime, with the reasons in the module's own docs.**
  **`CAPACITY = 4`, LRU** — two entries serve 99.8%, so four is double the
  measured need; **an entry pins ~2 MB at 1× and ~8 MB at 2×** (result *and*
  incoming, one byte per device pixel), so four entries is **≤ ~8 MB at 1×
  and ~32 MB at 2×**, against **38.3 MiB for all 40 (= 0.958 MiB each) for
  ~0.2% more hits**. **Owned by the `Interpreter`**, so it dies with the
  content stream — **not global and not `thread_local`**, because rendering
  moved to a worker in **Pass 44.0**, masks are keyed partly on device size,
  and **nothing outside one render should observe another render's masks**.
  **LFU would also work; LRU was chosen for a hot-path-changes case that has
  NOT been measured**, and the code says so rather than implying it.

  **(7) ★★ A MEASUREMENT ADJUDICATED BETWEEN A FIGURE AND ITS OWN
  CORRECTION, AND THE CORRECTION WON.** *floor 0.51 + painting **0.27*** =
  **0.78 s** against the measured **0.79 s** (**1.3% apart, CONFIRMED**);
  *floor 0.51 + painting **0.87*** = **1.38 s** (**75% high, REFUTED**);
  adding the 0.015 s residual closes it to **0.795 s**. **The `R164` painting
  correction was made on reasoning alone four filings ago and had never been
  independently measured.** It now has been — and it also explains why the
  previous filing's **~1.7 s projection was conservative by 2.2×**: that
  projection rested on the uncorrected residual.

  **(8) The gate fix, `c3d8853`.** `tools/check-passes-filed.py` joins on
  *"this commit's short hash appears in `docs/ROADMAP.md`"*, which **a commit
  that WRITES `ROADMAP.md` cannot satisfy** — the hash does not exist until
  the commit does. **The defect was as old as the gate and had been latent
  behind a NAMING HABIT**: filing commits are customarily subjected
  `docs: …`, which the Pass-claim regex never matches, and it surfaced only
  because `e7e74f2` was subjected `Pass 44.0: …`. **A habit was doing a
  guard's job.** The exemption keys on the **diff** (`is_docs_only`), not the
  subject — a commit touching only `docs/` cannot be a Pass's code half
  whatever its subject says, while keying on the subject would rebuild the
  same fragility. **Verified narrow rather than merely quiet:** `e7e74f2` and
  `e6574b7` exempt at 4 files each, **`7926a78` NOT exempt at 4 files.**
  **This answers the tooling ruling the twenty-fifth filing asked for, in
  code, taking the exemption over the cite-it-next-filing convention on
  `R163` grounds — a mechanical carrier beats a remembered obligation.**

  **(9) `Pass 45.0` IS MINTED, scoped to `ce57ed5` only.** Family 45 was free
  — **established by `git grep -n -E "Pass 45(\.|\b)"` over all tracked files
  returning exit 1**, and by the ledger checker's ceiling of 44. **Every
  clause of the four prior refusals' shared reason is false here**: in the
  shipped crate, not behind a feature flag, behaviour changed materially, and
  the GUI benefits with no change of its own. ~~**One scope question is
  FLAGGED for the engineer, not decided:** whether the ID should also cover
  `76200e9` and `4475fe6` retroactively, on Pass 44.0's precedent.~~
  **★★ RULED 2026-08-07 (twenty-seventh filing): THE WIDER SCOPE. `Pass 45.0`
  COVERS `76200e9` + `4475fe6` + `ce57ed5` — one arc, all three attacking
  clip cost, all three byte-identical, cumulatively 1× 32,313 ms → 907 ms
  (35.63×). `4475fe6` is what made the cache EXPRESSIBLE — a hit hands back a
  SHARED mask, which requires a shareable clip, which is exactly what
  `Arc<Mask>` built. NOT widened to `1992d13` or `fa17d54` (instrumentation).
  The widening MINTED NOTHING: an existing ID growing to cover more commits
  consumes no number.** **Widening later is a free amendment; narrowing is
  not.** **Rule candidate (ii)
  (*choose a measurement's error direction against your own hypothesis*) is
  HELD, NOT MINTED — `R167` stays free and reserved for it**, the trigger
  being one independent second episode; the unbounded-final-bucket candidate
  stays declined on `R163`.

  **Git and backup state — CHECKED, not inferred (hard rule 8), and
  TIMESTAMPED because a checked fact decays.** At the start of this filing:
  `git rev-parse HEAD` → **`c3d8853`**; `git status --porcelain` → **0
  lines**; `git remote -v` → **empty**. Newest bundle
  (`ls -la D:\Dev\pdfce-backups\`): **`pdfce-20260807-1818.bundle`**, whose
  `refs/heads/pass-8-redaction` is **`c3d8853…` = `HEAD` exactly**
  (`git bundle list-heads`), with **`ce57ed5` an ancestor**
  (`git merge-base --is-ancestor` → exit 0). **Both commits described here
  are backed up.** **The clean tree remains a snapshot** — this filing reads
  committed blobs only. **`docs/` ONLY was edited.** **SIX RAG findings are
  OWED and unwritten** (three carried, three new: the ABA key, the
  hash-over-printed-output contamination, and the gate precondition satisfied
  by an unwritten convention); **their absence from `D:\dev\rag\rust\` was
  established by `ls` and `grep`, with nearest-neighbour files named in §12
  of the `ce57ed5` ROADMAP entry.**

  **★★ THE SNAPSHOT EXPIRED INSIDE THIS FILING, FOR THE THIRD CONSECUTIVE
  FILING.** `git rev-parse HEAD` re-run at the **end** returns **`9681112`**
  — ***"the render worker starts saying what it did"***, 1 file,
  `crates/pdfce-gui/src/render_worker.rs` **+23 / −0**, committed **18:33**,
  landed mid-filing. `git status --porcelain` shows **exactly this filing's
  four `docs/` files**. **Nothing in this entry is affected** — it describes
  committed blobs, and neither `ce57ed5` nor `c3d8853` moved. **★ AND IT
  CORROBORATES THE HEADLINE FIGURE FROM THE INSTRUMENT THE OPERATOR ACTUALLY
  USES:** that commit records a live GUI trace on the same sheet —
  `render-async-started gen=1 budget_ms=12` → **`render-async-done gen=1
  ms=907 outcome=done`**, *"with the UI thread processing frames
  throughout"*. **The 907 ms above was measured through the CLI; this is the
  same figure through `pdfce-gui`'s worker**, which is the joint claim of
  `Pass 44.0` and `Pass 45.0`. ~~**`9681112` is OUT OF SCOPE here and remains
  UNFILED — owed to the next filing**~~ **★ THE DEBT IS PAID: `9681112` IS
  FILED IN FULL by the twenty-seventh filing, at the top of `ROADMAP.md`'s
  *Shipped*, with no Pass ID (GUI instrumentation, +23 / −0, no behaviour and
  no timing change — the same ground as `1992d13` and `fa17d54`).** — and
  **the 18:18 bundle is therefore one commit behind again.** **Both checkers
  were re-run after `HEAD` moved: exit 0 and exit 0.**

- **2026-08-07 (twenty-seventh filing, `9681112`) — THE RENDER WORKER STARTS
  SAYING WHAT IT DID; `Pass 45.0` IS WIDENED TO THE WHOLE CLIP ARC BY
  ENGINEER RULING; AND THE SIX-DEEP RAG DEBT IS DISCHARGED IN FULL.**
  One commit filed, one ruling recorded, seven cross-project findings
  written. **Nothing minted** — Pass family stays **45**, rules stay **R166**
  (**R167** free and reserved), decisions stay **031**.

  **(1) ★★ THE RENDER PATH HAD NO INSTRUMENT, AND ITS SILENCE WAS READ AS A
  RESULT.** `crates/pdfce-gui/src/render_worker.rs` emitted **no diagnostic
  trace at all**. That was discovered by pointing a filter at it and reading
  the empty output as confirmation the render had worked — **it was not the
  null result of an experiment, it was the absence of an experiment.** Three
  traces now: `render-inline` (beat the 12 ms budget, never went async — the
  *"nothing regresses when fast"* claim made checkable), `render-async-started`
  (**carrying `budget_ms=12`, so the threshold is in the record rather than
  in a constant someone must look up**), and `render-async-done` (**elapsed
  ms + `done`/`cancelled`/`failed` — a distinction the shell ALREADY made and
  could not be seen making**). **+23 / −0 across exactly 3 emit sites = 7.7
  lines per trace.**

  **(2) ★★ THIRD INSTANCE IN ONE DAY OF AN OBSERVABILITY CHANNEL BEING
  ITSELF UNOBSERVABLE**, and this is the architectural point rather than the
  anecdote: `edit_note` (the single choke point every rule-4 disclosure
  reaches the operator through), `gui-drive`'s trace file (which could not
  observe **which binary it was observing**), and now the worker built in
  **Pass 44.0** specifically to make a slow render observable and
  interruptible. **A component built to make something else observable
  acquires NO observability from that purpose — and is less likely to get
  any, because its own reason for existing reads as coverage.** Every claim
  resting on such a channel is a claim about **code reading**, not about
  behaviour.

  **(3) ★★ THE FIRST IN-GUI MEASUREMENT OF THE ORIGINAL COMPLAINT.**
  `render-async-started gen=1 budget_ms=12` →
  **`render-async-done gen=1 ms=907 outcome=done`**, with the UI thread
  processing frames throughout. **907 ms through `pdfce-gui`'s worker against
  907 ms through the CLI**, on the sheet that took **32,313 ms with a frozen
  window** that morning — **35.6×, and this is the instrument the operator
  actually uses.** `outcome=done` carries the `Pass 44.0` half (the render
  completed on a worker and the window lived); **907 ms against
  `budget_ms=12` is 75.6× over budget**, so the async path is the one that
  ran, by design. **⚠ Pass 44.0's owed SCREENSHOT verification is UNTOUCHED
  — a trace proves the worker ran and finished; it does not prove the pixels
  are right.**

  **(4) ★★ `Pass 45.0` IS WIDENED, BY EXPLICIT ENGINEER RULING, TO
  `76200e9` + `4475fe6` + `ce57ed5`.** One arc: the per-paint clip clone,
  `Arc<Mask>`, the cache. **Per-commit factors 1.71× · 1.72× · 13.52×; the
  ID's own cumulative figure is 32,313 ms → 907 ms = 35.63×** — and
  **1.71 × 1.72 × 13.52 = 39.8 is NOT the ID's figure**, because the three
  "before" values come from two instruments and three tree states (an `R164`
  shape, named rather than smoothed). **`4475fe6` is what made the cache
  expressible** — a hit hands back a shared mask, which requires a shareable
  clip. **NOT widened to `1992d13` or `fa17d54`**: instrumentation, and the
  four consecutive ID refusals over them are the calibration that gives
  `Pass 44.0` and `Pass 45.0` their meaning. **Widening consumed no number.**
  Precedent now supported by **two independent episodes** (`Pass 44.0` over
  `e4256f2`, `Pass 45.0` over two): *an ID covers the ARC that produced the
  operator-visible change, not the commit that happened to finish it* —
  **recorded, deliberately NOT minted as a rule.**

  **(5) THE RAG DEBT — SIX OWED, SEVEN WRITTEN, THREE FILES AMENDED.** All
  to `D:\dev\rag\rust\`: the unbounded final bucket · error-direction against
  your own hypothesis · the self-referential gate join · **the ABA pointer
  key** · **hash-over-printed-output contamination** · **a gate precondition
  satisfied by an unwritten naming convention** · **plus a seventh, added on
  the engineer's question and judged IN: a measurement taken for an unrelated
  purpose can adjudicate a dispute it had no stake in.** **Amended rather
  than duplicated (hard rule 4):** the absent-trace chain gained a **zeroth
  link**, the disclosure-channel file was **generalised to any observability
  channel**, and `personal_rag\claude_code\`'s pipeline-exit-status lesson
  gained its **fourth** instance. **Nothing went to `C:\personal_rag\pdf\`** —
  all seven are ecosystem/methodology, checked per finding; the ABA one is
  Rust `Arc` identity, not PDF. **Absence re-established from the DIRECTORY,
  not from the debt table** (that table has been wrong twice, and its
  correction was wrong once): 75 entries before, **82 after, 81 index
  bullets, orphan sweep empty in both directions.**

  **(6) TWO PROCESS FAILURES FILED WITH THE COMMIT**, because they are the
  day's own lesson committed while writing about it: **`build rc=0` read as a
  successful compile when it was `tail`'s exit status** — the **FOURTH**
  wrong reading from that idiom in one day, one of which made a hang look
  like a pass — and **an empty trace filter read as evidence the render
  worked**, which happened to the person writing the fix for it, in the same
  ten minutes. **That frequency is the argument that the class is not solved
  by knowing about it; it is solved by the instrument existing.**

  **(7) ⚠ NO GATE FIGURES EXIST FOR `9681112`, AND THAT IS STATED RATHER
  THAN PAPERED OVER.** `git show -s --format=%B 9681112` contains **no test,
  `fmt`, `clippy` or gate line**; the dispatch relayed none; **this filing
  ran no build, no test and no render.** The commit's message records that
  its **first compile FAILED** and that the failure was read as success from
  `build rc=0`. `git status --porcelain` → **0 lines**, which says the tree
  is clean, **not that it builds. A green build for `9681112` is UNVERIFIED
  and is not claimed.**

  **Git and backup state — CHECKED by this filing, not inferred (hard rule
  8), and timestamped because a checked fact decays.** At the start:
  `git rev-parse HEAD` → **`6efb9b3`**; `git status --porcelain` → **0
  lines**; `git remote -v` → **empty, exit 0 — bundles remain the only copy.**
  Newest bundle (`ls -la D:\Dev\pdfce-backups\`, filename **read**, tip
  **read by `git bundle list-heads`, not inferred from the filename's
  timestamp**): **`pdfce-20260807-1838.bundle`**, whose
  `refs/heads/pass-8-redaction` is
  **`9681112d43cb34d2ed4151099a60323f733189a8`** — **exactly `HEAD~1`.**
  **★ SO THE COMMIT THIS FILING DESCRIBES *IS* BACKED UP**, which supersedes
  the previous filing's *"`9681112` is in no bundle"* — true at 18:18, and
  the 18:38 bundle was taken after it. **★ AND THE DISPATCH'S FIGURE IS
  CORRECTED BY ONE, IN THE SAFE DIRECTION:** it stated the bundle was *"two
  commits behind now"*; **at this instant it is ONE behind** (`6efb9b3`, the
  `Pass 45.0` filing commit, is not in it — `git bundle list-heads` names
  `9681112`). It becomes **two** the moment this filing is committed, which
  is what the dispatch was anticipating, **and the engineer said he will
  bundle after this filing.** Recorded because hard rule 8's whole content is
  that a checked figure beats an inferred one **in both directions** — the
  dispatch's number was a projection, and projections about disk are exactly
  what this rule exists to keep out of the record. **Checkers re-run before
  and after:
  `check-ledger-numbers.py` exit 0 / exit 0, `check-passes-filed.py` exit 0 /
  exit 0.**

- **2026-08-07 (twenty-eighth filing, `3d345aa`) — THREE DECISIONS ON FIELD
  RENAMING, ONE OF WHICH IS A SCOPE CORRECTION TO DECISION 020 ITSELF.**
  Filed as `ARCHITECTURE.md` §12 entries rather than as a new numbered
  decision record: **decision records stay at 031 (032 next free), and
  nothing is minted.** The first two are rulings on an open question inside
  an existing decision record; the third corrects that record's reading, not
  its content.

  **(1) A rename into an occupied name REFUSES — it does not merge, and it
  does not auto-suffix.** Decision 020 named `rename-field` (§6's F6) and
  §0.1 ruled its flat spelling, but **never decided the collision case.**
  The fork implemented refuse and argued for it; the engineer confirms.
  **The argument is the asymmetry with creation, and it turns on what the
  caller supplied.** A same-type `add-*` onto an existing name **merges**,
  because the caller supplied a *type and a name* — they asked for a field
  of that name, and §12.7.3.2 makes same-FQN nodes representations of one
  field, so merging is the spec's own answer to the request as stated. A
  `rename-field` supplied **two identities** — an existing field to keep and
  a new name to move it to — so **merging would destroy an identity that was
  never offered up**, with nothing in the request saying how to reconcile
  two sets of `/V`, `/Ff` and `/Kids`. **Auto-suffixing (`Name_2`) is
  rejected separately and on rule 4:** it hands the operator a name they did
  not choose, silently, which is worse than a refusal because the document
  then contains a name nobody typed. **The refusal is one `if`**, recorded
  because a ruling that is cheap to overturn deserves less scepticism than
  one that is not. Implemented as `FormAuthorError::RenameCollision`.

  **(2) A dotted PATH and a MALFORMED name are separate refusals, because
  the reader's next move differs.** `FormAuthorError::DottedPartialName` was
  added after reusing `PeriodInPartialName` for `A.B` produced *"contains an
  empty name segment"* — **which `A.B` does not have.** `A..B` is malformed
  and there is a typo to find; `A.B` is a **well-formed two-level path that
  simply is not a PARTIAL name** (one segment by construction) and there is
  no typo at all. **An error message that is confidently wrong about the
  cause is worse than a generic one**, because it spends the reader's time
  in the wrong place, and the more they trust the tool the more it costs. A
  test asserts the two messages differ, since an untested distinction is one
  refactor from collapsing back.

  **(3) SCOPE CORRECTION — decision 020's F6 is TWO items, and this
  project's own documents have been reading it as four.** §6's F6 bullet
  names `--defaults-from <field>` and `rename-field`. **`move`, `resize` and
  `re-flag` appear nowhere in decision 020** — established by
  `git grep -n -iE "resize|reposition|move.*field|re-flag|change.*flag" -- docs/decisions/020-form-field-authoring.md`,
  which returns **16 hits, every one of them** the `remove-*`→`delete-*`
  house-word supersession, the Shape A→B promotion's *"move the annotation
  keys off the field dict"* step, or a mid-group/last-member **deletion**
  rule. The four-item phrasing originated in `ROADMAP.md` (three places) and
  propagated to `FEATURES.md` (one); all four are amended. **The correction
  matters because *deferred* and *UNSCOPED* are indistinguishable to a
  reader of the Backlog and are opposite states of the world** — deferred
  means someone decided it and put it later; unscoped means nobody ever
  decided it. With both F6 items built, *"field property editing is done"*
  would be **true of F6 and false of the capability**, and no roadmap entry
  anywhere would explain why nudging a field 3 pt left does nothing.
  **Move / resize / re-flag are therefore filed as owed a SCOPE DECISION —
  a new slice, an amendment to decision 020, or a named refusal — not as
  owed an implementation.**

  **The structural fact underneath all three, which belongs in the record
  because decision 020 states its opposite in passing.** §12.7.3.2's
  fully-qualified name is **derived, not stored** — there is no `/FQN` key;
  a viewer descends from `/AcroForm /Fields` joining each node's `/T`. So
  **rewriting one node's `/T` re-derives that node's identity AND its entire
  subtree's, and the subtree's objects are byte-unchanged** — they were
  never storing the thing that changed. **A subtree rename is ONE object
  write.** Decision 020's F6 bullet says the rename *"needs `Field.parent`
  from F0 for subtree renames"*, which reads as *the write walks down and
  fixes each child*. **It does not.** `Field.parent` is what lets the verb
  **count** the affected descendants so rule 4 can disclose them
  (`FieldRename::descendants_renamed`) — required by the **disclosure**, not
  by the **mutation**. **The requirement was real and its stated reason was
  wrong**, which is the same shape as this librarian's hard rule 8 amendment
  the same day: an obligation that stayed correct while its reason went
  stale. Verified end to end: `Personal.Address` → `Location` yields
  `Personal.Location.Zip` and `Personal.Location.City`, `Personal.Name`
  untouched, **`descendants_renamed=2`** — 2 descendants re-identified over
  1 object written.

  **Body sections touched by this entry:** none. §4's API contract is not
  restated here because `rename_field` is an `EditSession` verb of the same
  shape as the shipped `delete_field`/`delete_widget`, and §4's forms
  paragraph already describes that surface generically; **the capability-
  level record is `FEATURES.md`'s new *RENAME a form field* row and
  `ROADMAP.md`'s `Pass 20.6 (PARTIAL)` entry.**

  **Git and backup state — CHECKED, not inferred (librarian hard rule 8).**
  `git rev-parse HEAD` → **`3d345aa`**; `git status --porcelain` → **0
  lines**; `git remote -v` → **empty, exit 0.** Newest bundle
  (`ls -la D:\Dev\pdfce-backups\`) **`pdfce-20260807-1859.bundle`**, tip
  **read by `git bundle list-heads`** → `refs/heads/pass-8-redaction` =
  **`02a789d…`** = **exactly `HEAD~1`. The dispatch's "one behind" figure is
  CONFIRMED, and `3d345aa` is in no bundle.** **Checkers re-run before and
  after: `check-ledger-numbers.py` exit 0 / exit 0, `check-passes-filed.py`
  exit 0 / exit 0.** **Gates relayed from the engineer, not measured here
  (R87): 2181 tests / 0 failed, clippy 0, both ledger checkers 0.**

### 2026-08-07 (twenty-ninth filing) — **geometry manipulation is its own capability, not a forms one: `Pass 46.0` + `Pass 46.1` filed on the operator's request; re-flag stays unscoped; `R167` stays free**

**Operator request, verbatim:** ***"form fields and everything else should be
draggable and resizeable."*** It arrived **minutes after** `ROADMAP.md`'s
twenty-eighth filing recorded field move / resize / re-flag as **UNSCOPED —
not deferred, not refused, not planned** — and it is **wider than the gap as
filed**, naming *everything else* rather than only fields.

**RULING 1 — this is a NAMED plan entry, NOT an amendment to decision 020.**
Decision 020 governs **authoring**: bringing a field into existence, its
name-collision resolver, its per-type minimums. **Changing a placed widget's
`/Rect` is geometry manipulation.** The same verb that moves a widget moves a
highlight, an ink stroke and a redaction mark — **§12.5.2 Table 164 makes
`/Rect` Required on every annotation subtype** — and decision 020 has no
authority over any of those. **Filing it inside 020 would put it where nobody
scoping *editing* would look**, which is the failure the twenty-eighth filing
was written to prevent. Filed instead as a *Next up* entry and as two Pass
IDs.

**⚠ THE NARROWING: RE-FLAG IS NOT COVERED AND STAYS UNSCOPED.** The operator's
sentence is about **geometry**. **`/Ff` bit editing** (read-only, required,
multiline, comb, `RadiosInUnison`) **has no geometric component** and belongs
to the forms domain in a way `/Rect` does not. **Folding it into a geometry
Pass because it sat beside move and resize in one earlier sentence would be
scope drift, not scoping.** It remains owed a decision — a new F-slice under
decision 020, or a named refusal. Three items went in as unscoped; **two came
out scoped and one did not**, which is the state the record must preserve.

**RULING 2 — `R167` stays free.** The byte-identity candidate the
twenty-eighth filing wrote up is **a RAG finding, not a standing rule**. Its
four grounds hold and **the third decides it: *"validate every baseline"
commissions WORK, not CARE*** — the exact ground the **ablation candidate**
was refused on **the same day**. **Refusing it there and minting it here would
make the bar decorative**, and a bar that bends to whoever happens to be
filing enforces nothing. **Ceiling stays `R166`; `R167` next free.** Second
consecutive filing to leave it free, both for the same stated reason.

**THE ARCHITECTURAL CONTENT — why this is two Passes and not one, and it is
not "fields vs. everything else".** The boundary is **where the geometry is
stored**:

- **Family (a) — dictionary-`/Rect`.** Four numbers in an annotation
  dictionary. **`Pass 46.0`.** Members: form-field widgets, markup
  annotations, redaction marks, links, ce dimensions, popups.
- **Family (b) — content-stream.** Coordinate operands inside a page content
  stream. **`Pass 46.1`.** Members: vector paths, text, images, form XObjects.

**A form-field widget is family (a) and so is a highlight; a vector path is
family (b). The operator's "everything else" straddles both.**

**THE FINDING THAT MAKES FAMILY (a) NOT THE EASY ONE — normative, not an
implementation detail.** **§12.5.5's placement algorithm, step (b)** computes a
matrix **A** that maps the transformed appearance box's lower-left and
upper-right corners onto `/Rect`'s corners **independently in x and y**.
Therefore **enlarging `/Rect` does not reveal more artwork — it
ANISOTROPICALLY STRETCHES the artwork already present.** The spec RAG
(`PDF_Spec/iso32000/iso32000__s__12.5.5.md`) states it outright: *"aspect
ratio is not preserved. This is normative, not a bug."* Because **R43** paints
an annotation from its appearance stream **or not at all**, every
pdfce-authored annotation carries a baked `/AP` — so **a resize is a
REGENERATE, not a `/Rect` write.** A `/Rect`-only resize yields stretched
arrowheads, stretched glyphs and stretched border strokes: **plausible at
1.05×, obviously broken at 3×**, which passes a casual test and fails the
operator's real one. **The regeneration path is the bulk of `Pass 46.0`, not
the array write** — and `annot_author`'s builders being pure and deterministic
(the property `move_dimension` already relies on) is the asset it spends.

**THE FINDING THAT MAKES FAMILY (b) HARD — line width, checked against the
spec rather than assumed.** A resize is either an **operand rewrite**
(geometry scales, **the stroke does not**) or a **wrapping `q … cm … Q`**
(**the stroke scales too**), and the two are **not interchangeable**.
§8.4.3.2 (quoted in `PDF_Spec/iso32000/iso32000__s__8.4.md`) puts line width
in **user-space units**, so: a `cm` scales every stroke; an **anisotropic**
`cm` makes stroke thickness **orientation-dependent**; and **a line width of
0** — *"the thinnest line that can be rendered at device resolution: 1 device
pixel wide"*, ubiquitous in the CAD output that is this project's real
corpus — **is not a number a scale can act on at all.** §8.4.3.5's miter limit
is a *ratio to line width* and §8.4.3.4's join angle is measured *in user
space*, so a non-uniform scale can visibly change corner shape with nothing
about the miter limit edited. **A named refusal of non-uniform scale on
stroked content is a legitimate third outcome** (decision 027). **The
mechanism choice determines the data model, not just the implementation, and
must be ruled before `Pass 46.1`'s core verb starts.**

**THE GUI IS AN AXIS, NOT A THIRD FAMILY — and a drag needs NO confirm step.**
Recorded so nobody rebuilds the box decision 024 §4.4 removed. **Rule 4 as
narrowed by §4.4 explicitly permits direct manipulation whose result is
visible and undoable**; §4.4's own words: it *"does not require a two-click
confirmation for a direct manipulation whose result is fully visible on the
canvas and reversible in one undo."* **What still binds:** nothing floats over
the canvas (readouts, disclosures and refusals live in the tool's dock
compartment at a fixed anchor), and **an INFERRED resize — a snap, a
match-to-neighbour, an aspect-lock nobody asked for — is rule 4's territory in
full.** The **GUI half ships WITH each family**, per **R151**, which exists
because `move_subpath` sat callerless in core from Pass 28.0 until Pass 36.0.

**⚠ ONE OWED RULING RAISED, NOT ADJUDICATED.**
`EditSession::move_dimension`'s doc comment says patching `/Rect` alone *"would
slide the box and leave the drawing inside it exactly where it was"*. **Read
against §12.5.5(b) that appears too strong** — with `/BBox = /Rect_old` and
identity `/Matrix`, translating `/Rect` makes **A** a pure translation.
**The verb's BEHAVIOUR is not in question** (regeneration is correct either
way: `/L`'s endpoints and the `/PieceInfo` sidecar go stale, and §12.5.6.7
makes `/L` authoritative for any viewer that regenerates). **Its stated REASON
is** — the recurring shape of *an obligation that stayed correct while its
reason went stale*, the same shape as this librarian's own hard-rule-8
amendment.

**Body sections touched by this entry:** none. §4's API contract is not
restated, because **no verb exists yet** — these are plan entries and ticking
§4 for them would be the over-optimism `FEATURES.md`'s own maintenance
contract forbids. **The capability-level record is `FEATURES.md`'s two new
`Pass 46.0` / `Pass 46.1` *Planned* rows and `ROADMAP.md`'s *OPERATOR REQUEST
2026-08-07* entry at the head of *Next up*.**

**Git and backup state — CHECKED, not inferred (librarian hard rule 8).**
`git rev-parse HEAD` → **`247b8fa`** — **not the `8689f76` the dispatch
named**, because the `--defaults-from` fork committed mid-filing; the
dispatch's figure was correct when written and **went stale during the work**,
which is precisely why the rule forbids inferring disk state from documents.
`git status --porcelain` → **0 lines**; `git remote -v` → **empty, exit 0.**
Newest bundle (`ls -la D:\Dev\pdfce-backups\`) **`pdfce-20260807-1941.bundle`**
(8,483,256 bytes), tip **read by `git bundle list-heads`** →
`refs/heads/pass-8-redaction` = **`8689f764…`** = **exactly `HEAD~1`; the
dispatch's bundle-tip figure is CONFIRMED, and `247b8fa` is in no bundle.**
**Checkers re-run before and after: `check-ledger-numbers.py` exit 0 / exit 0,
`check-passes-filed.py` exit 0 / exit 0.** **Ceilings after, quoted from the
checker rather than restated:** *"Pass families with headings: up to **46**
(highest ID **`46.0`**)"* and *"Pass families MENTIONED: up to **46** (highest
ID **`46.2`**)"* · **`R166`** (**`R167`** next free) · decisions **`031`**
(**`032`** next free). **Those two lines disagree by design:** `46.1` is filed
but the heading parser reads only the first ID of a `Pass 46.0–46.1` range
(same as the existing `Pass 24.0–24.5`), and **`46.2` is not filed at all** —
it is named in prose as *free*, which reserves the number against reuse
(project rule 2) without assigning it. **`46.2` remains available and
deliberately unminted.** **A gate fix was made in passing and measured:** the
entry was first written with the project's usual `### ★ Pass …` decoration,
the checker proved it **invisible** (headings ceiling stuck at **45**, `46`
listed as claimed-but-unheaded — the ten-heading `collect_passes` defect
already in *Backlog*), the heading was rewritten to lead with `Pass`, and the
same checker then read **46**. **The Backlog defect is not fixed; these two
IDs are simply not an eleventh instance of it.** **No gates measured by this
filing — it ran no build and no test, and claims none.**

### 2026-08-07 (thirtieth filing) — **decision record `032` is OPENED, NOT DECIDED: the vector-scale mechanism (wrap in `cm` vs rewrite operands) becomes a recorded question rather than an emergent default; `Pass 46.1` is blocked on it; ONE thing is ruled and everything else is left open**

**`docs/decisions/032-vector-scale-mechanism-wrap-vs-rewrite.md` — STATUS
OPEN.** This filing **opens** a decision record and deliberately does not
close it. The twenty-ninth filing raised the mechanism question inside
`ROADMAP.md`'s `Pass 46.0–46.1` §4 and said, correctly, *"Which mechanism is
right is an ENGINEER DECISION and is NOT made here."* **This filing gives that
ruling a container.**

**WHY IT IS A RECORD AND NOT A DEFAULT — it lands squarely on project rule 3.**
The Inkscape RAG's `transforms__*` bucket went **0 → 4 files** on 2026-08-07
(`D:\Dev\Rag-Specialized\Inkscape_Features\`) and surfaced the finding that
fixes the question's *shape* without answering it: **wrap-in-`cm` versus
rewrite-operands is the same trade-off as Inkscape's *Preserved* versus
*Optimized* transform storage** — *"the closest structural correspondence in
the whole corpus"*, in that file's own words, and one that *"lands directly on
a load-bearing pdfce invariant."* **Wrapping** = tiny diff, but an editor
artefact, **nesting on repeated edits**, and **the compensating-`w` path with
all its non-uniform problems**. **Rewriting operands** = exact stroke
independence **for free**, but the **largest possible diff for that object**
and the end of byte-identical operand re-emission. **Neither is universally
right, which is the definition of a decision record.** Four options are laid
out (always-wrap · always-rewrite · hybrid-with-a-stated-switch-rule ·
operator-facing choice); **none is recommended.**

**⚠ THE CONSTRAINT THAT BOUNDS EVERY OPTION, filed prominently because it is
the sentence that stops someone promising it: `vector-effect:
non-scaling-stroke` HAS NO PDF EQUIVALENT.** It is the mechanism SVG uses to
solve the non-uniform case, and **the escape hatch an engineer scoping from
Inkscape would assume exists.** **PDF's only device-space-referenced stroke
width is `0 w`** — a one-device-pixel hairline, **not an arbitrary constant
width**. **pdfce can offer non-scaling-stroke as an EDITING BEHAVIOUR, never
as a DOCUMENT PROPERTY** — the saved file cannot carry the intent, so a later
session or another tool has no way to know *"line weights are fixed here."*
**No option may be justified on the grounds that a fallback exists. There is
none.** Consequence recorded as settled: a *persisted* non-scaling-stroke
document property is **`out_of_scope`** — *"no PDF construct exists"*, not
*"we haven't looked."* **Absence established (R87), not assumed:**
`git grep -c -i "non-scaling-stroke"` over tracked files → **exit 1, zero
matches** — the concept had never entered this repository before this filing.

**⚠ NON-UNIFORM + STROKE-SCALING-OFF IS UNSATISFIABLE, AND THE REFERENCE FAILS
SILENTLY AT IT.** No scalar width cancels a non-uniform matrix — arithmetic,
and it transfers to PDF unchanged because both formats put line width in user
space and generate the stroke outline *before* transforming. **Launchpad
#1335376 closed *Invalid*** on exactly that ground (*"cannot be replicated by
adjusting stroke-width alone"*), and **Inkscape 1.4 distorts without warning
or refusal** (UX #339) **in the very mode where the operator asked for
constant line weight.** **Under rule 4 this is a place to be BETTER than the
parity reference, not equal to it** — a width pdfce *chose* is inferred state
and must be visible before it becomes document state. Three honest answers:
refuse (decision 027's named-refusal branch, already on the table), disclose
the residual anisotropy, or offer stroke-to-outline. **Silently fudging a
factor is not among them.**

**THREE INVERSIONS OR NON-TRANSFERS, each of which would mislead someone
scoping from the reference. (a) PATTERNS INVERT** — a PDF pattern `/Matrix`
maps to the page's **default** space, not the CTM at paint time, so *"don't
transform the pattern"* is PDF's **structural default** and **ON is the branch
requiring work**. **(b) GROUP-VS-EACH INVERTS** — SVG group scaling is one
matrix; in PDF **per-object is cheap** (N operand rewrites over decision 011
§2.1's already-segmented objects) and **as-a-group needs a shared wrapper.**
**(c) MARKERS HAVE NO PDF CONSTRUCT** — arrowheads are baked geometry, so
SVG's `markerUnits="strokeWidth"` coupling **must not be replicated** (not to
be conflated with annotation `/LE`).

**✅ THE ONE RULING, MADE BY THE OPERATOR: the rounded-corner-radii toggle is
`out_of_scope` — UN-IMPLEMENTABLE, not unbuilt.** PDF's `re` is a **sharp**
rectangle and rounded corners arrive as **already-flattened Bézier geometry
with no surviving radius parameter.** There is nothing for the toggle to act
on; radii scale with the geometry unconditionally, and that is the only
available behaviour. Recorded so a future pass does not re-investigate: the
answer is *"PDF flattened it before pdfce ever saw it."*

**FORM FIELDS ARE A THIRD BACK-END WITH NO INKSCAPE ANALOGUE — ONE OPERATOR
GESTURE, TWO MECHANISMS.** Widget resize is `/Rect` + `/BBox` + `/Matrix`
consistency (§12.5.2, §12.5.5), and **`/Rect` is neither the geometric nor the
visual bbox** — it is a *declared box the appearance is fitted into*, a third
kind of extent Inkscape has no concept of. **Scope it from `Acrobat_Features`
+ `PDF_Spec` §12.5.5, NOT from the Inkscape RAG** (the research files say so
themselves, twice, unprompted). **This is why `Pass 46.0` is NOT blocked by
this record** — family (a) never reaches the wrap-vs-rewrite question; it has
its own harder problem (§12.5.5(b)'s independent-in-x-and-y corner mapping,
which makes a resize a *regenerate*).

**WHAT DOES TRANSFER VERBATIM — the core stroke model**, recorded so a page of
non-transfers does not leave the impression that nothing carries over.
**§8.4.3.2**: line width is *"expressed in **user space units**"* and stroking
paints all points within **half that width in user space** — precisely SVG's
model — and `PDF_Spec` **already records independently** that an anisotropic
CTM makes stroke thickness orientation-dependent. **There is no SVG-only
wrinkle here to discount.**

**GAPs FILED AS GAPs, NOT DEFAULTS, and none may be quoted as fact:** the
**non-uniform compensation formula is UNKNOWN** — three sources describe the
symptom, **none the arithmetic**; **R61 bars reading the source**; and
**`sqrt(|det|)` was DELIBERATELY NOT recorded as fact** despite the SVG
expansion-factor convention suggesting it. Whether the toggles govern the
**numeric route** as well as the drag is unconfirmed — **pdfce should make the
answer *"yes, identically"* by construction** rather than inherit an
unverified one. And **⚠ set exhaustiveness: DO NOT SAY "Inkscape has exactly
four."** Four companion behaviours are confirmed (**stroke width · rounded-
corner radii · gradients-with-object · patterns-with-object**), plus *Store
transformation* (Optimized | Preserved) and a **visual-vs-geometric bbox
basis** — **neither of those last two a companion toggle** — and **there is no
filter toggle**; that no *fifth* companion behaviour exists was not
establishable from a reachable primary source.

**⚠ TWO ITEMS RAISED AND OWED, NOT DISCHARGED.** **(1) To
`pdfce-spec-librarian`:** the pattern-`/Matrix` anchoring clause **itself**
needs a **§8.7.3** re-check — the researcher flagged it as not re-fetched, and
**the whole pattern inversion rests on it.** **(2) To `pdfce-ui-specialist`:**
Inkscape's toggle is a **hidden global mode**, which is why *"why did my
stroke change?"* is one of its most-asked transform questions; **under rule 4
pdfce would be choosing the stroke consequence from off-screen state**, so the
state must be **legible at the point of the resize** (dock compartment, fixed
anchor). **No confirm step is being asked for** — decision 024 §4.4 already
settles that a visible, undoable direct manipulation needs none.

**Body sections touched by this entry: none.** §4's API contract is not
restated, because **no verb exists** — `Pass 46.1` is a plan entry and is now
explicitly blocked. Ticking anything for it would be the over-optimism
`FEATURES.md`'s maintenance contract forbids. **The capability-level record is
`FEATURES.md`'s existing `Pass 46.1` *Planned* row, amended in place to carry
the block**, and `ROADMAP.md`'s `Pass 46.0–46.1` §4 amendment block.

**Git state — CHECKED, not inferred (librarian hard rule 8).** `git status
--porcelain` at dispatch → **0 lines, clean** — so the `247b8fa`/`fd6eadd`
filing the dispatch warned might still be in flight was **already committed**,
and there was **no race to lose.** `git log --oneline -1` → **`fd6eadd`**;
both named hashes resolve (`git log --oneline -1 <hash>` for each). `git
remote -v` → **empty.** **Backup currency was NOT checked by this filing and
no figure is claimed for it** — this filing had no reason to take a bundle and
will not infer one from documents.

**Checkers re-run, before and after: `check-ledger-numbers.py` exit 0 / exit
0; `check-passes-filed.py` exit 0 / exit 0.** **Ceilings after, quoted from
the checker rather than restated:** *"Pass families with headings: up to
**46** (highest ID **`46.0`**)"* · *"Pass families MENTIONED: up to **46**
(highest ID **`46.2`**)"* · standing rules **`R166`** (**`R167`** next free) ·
decision records **`032`** (**`033`** next free). **`032` is the only number
this filing minted.** **No Pass ID and no standing rule** — the Pass ceiling
stays **46** and **`R167` stays free for a third consecutive filing.**
**No gates measured — this filing ran no build, no test and no render, and
claims none.**

### 2026-08-07 (thirty-first filing, `247b8fa` + `fd6eadd`) — **F6 CLOSES, `Pass 20.6` STOPS BEING PARTIAL, AND `Pass 46.0` DELIVERS ITS FIRST SLICE — plus TWO CORRECTIONS TO REASONS THAT WERE STALE WHILE THEIR CONCLUSIONS STAYED RIGHT**

**Filed as `ARCHITECTURE.md` §12 entries rather than as a numbered decision
record. NOTHING MINTED BY THIS FILING** — no Pass ID (both commits belong to
already-assigned IDs), no standing rule, no decision record. **The
decision-record ceiling nevertheless moved from `031` to `032` during this
filing, and NOT because of it** — see *Concurrency* at the end.

**Why this filing is the THIRTY-FIRST and not the thirtieth:** a second
`pdfce-librarian` ran **concurrently** on the vector-scale question and had
already claimed *thirtieth* in this very section. **Filing ordinals are minted
by hand with no checker behind them**, so they are not collision-safe under
concurrency — the finding is recorded in `SESSION_LOG.md`'s *Filing hygiene*,
and the ordinal was ceded rather than contested because the other entry was
already on disk.

#### (1) `--defaults-from` ships on all four creation verbs — F6 is CLOSED

`247b8fa`. The flag takes a **template field** and copies **`/MaxLen`**
(text), **`/Opt`** export↔display pairs (choice), and the **on-state read from
`widgets[0]`** (check box). **A radio template copies nothing** — on-states
live per **widget**, while the flag names a **field**, so there is no single
value it could refer to.

**Gates, engineer-measured at `247b8fa` and relayed (R87):** 2187 tests / 0
failed (**+6** over `3d345aa`'s 2181, all six in
`crates/pdfce-core/tests/form_field_authoring.rs`, **33 → 39** by
`git show <rev>:… | grep -c '^#\[test\]'`), clippy 0, fmt / ui-strings /
bypass-paths each by its own exit code, `pdfce-core` and `pdfce-render`
GUI-free. **+635 / −5 over 3 files = 158.8 insertions per creation verb**, the
flag attaching to all four being why one idea costs four verbs of wiring.

**★★ THE ARCHITECTURAL CORRECTION, AND IT IS THE ENGINEER'S OWN RULING BEING
CORRECTED BY ITS OWN IMPLEMENTATION.** Ruling 3 said ***"copy the common
subset and disclose the drop."*** The build makes that sentence collapse:

- **every property SHARED across the four field types is a BOOLEAN** (multiline, read-only, required, combo, sort, comb …);
- **every boolean is EXCLUDED**, because the CLI's flags are **presence flags** — *absent* and *explicitly false* are one token, so a copy could **ADD** a property but **never turn one off**, and a single-line field could not be created from a multiline template. That is a one-way, operator-facing trap, expensive to reverse once scripts depend on it, and it is a property of **the flag shape**, not of the idea (revisit if `--no-*` pairs are ever added);
- **every remaining copyable property is TYPE-SPECIFIC.**

**Therefore there is no common subset.** The rule reduces to ***"copy nothing
and disclose it"***, and a text template contributes **literally nothing** to a
choice field. **The shipped behaviour is coherent — the mismatch is disclosed
rather than silently producing a bare field. What was wrong was the RULING'S
DESCRIPTION**, which named a mechanism (*take the intersection*) with no
members. **It only became visibly wrong when someone tried to implement the
sentence** — a rule that reads as a procedure but is not one survives review
indefinitely, because review checks whether a claim is *true*, not whether it
is *executable*. **This is the same shape as hard rule 8's own amendment and
as (3) below: an obligation that stayed correct while its stated reason went
stale.**

**`/TU` is excluded and that exclusion is load-bearing.** **R105** exists so an
accessibility name is never a silent default — *"I never considered it"* must
not be able to happen quietly. **A copied tooltip satisfies R105's mechanism
while defeating its purpose**, and so does a copied *declination*: inheriting
*"no tooltip"* is still a decision the operator never made. **`/AA` is
excluded** because decision 020 **F3** rules that push-button creation authors
no action, and copying it would author actions through the back door.
**Values are excluded** because a value is content, not a default. **A
non-UTF-8 `/Opt` entry copies nothing** rather than being lossily converted —
*a mangled export value is a value the form would submit*.

**The two disclosures are STRUCT FIELDS**, so `any()`'s destructuring makes an
unhandled one a **compile error**; it fired immediately on the preflight
initializer. **A disclosure that can be forgotten will be** — the type system
enforcing it is the same discipline as the `edit_note` trace, moved off review
and onto the compiler.

#### (2) `move_widget` ships — and a MOVE needs no appearance regeneration, which is a fact about §12.5.5 rather than an optimisation

`fd6eadd`, filed as **`Pass 46.0` (PARTIAL)**. **Gates, engineer-measured and
relayed (R87):** 2191 tests / 0 failed (**+4** over 2187, all four in
`form_field_hierarchy.rs`, **19 → 23**), clippy 0, the rest by exit code,
`cargo tree` 0 GUI matches for core and render. **+395 / −0 over 3 files =
131.7 lines per file, and 395 over one delivered verb.**

**The derivation.** §12.5.5 step (a) transforms `/BBox` by `/Matrix`; step (b)
computes **A** mapping that transformed box's corners onto `/Rect`'s corners,
so the per-axis scale is `Rect_extent / box_extent`. **pdfce authors every
appearance with `/BBox = /Rect`, identity `/Matrix`, absolute page
coordinates** (`annot_author.rs`'s module header states it as the placement
discipline; `dimension/author.rs:592` does the same for ce dimensions), so both
factors are exactly **1** before any edit. **A translation does not change
either extent**, so both stay 1 and **A degenerates to a pure translation**:
the artwork is carried, unscaled, **by the algorithm every conforming reader
already runs.** Regenerating would rewrite a stream to produce bytes the format
supplies for free. **One object is written.**

**★★ THE SPLIT THAT IS THE SHAPE OF THE REST OF `Pass 46.0`, and it is an
ARCHITECTURAL fact about how pdfce authors appearances, not a spec fact:**

| family | appearance geometry | re-runnable at a new size? |
|---|---|---|
| **form-field widgets** (text, check box, radio, choice) | drawn **at origin** into `[0 0 w h]`, or reading `(w, h)` off `widget.rect` | **YES, already** — `build_field_text_appearance(w, h, …)` (reached from `regen_field_appearance`, `edit.rs:8097`), `build_check_box_appearances`, `build_radio_button_appearances`. All four size-parameterised and position-independent **today**. |
| **markup annotations** (Square, Circle, Line, Polygon, Ink, FreeText, redaction marks, ce dimensions) | `/BBox == /Rect` with **ABSOLUTE** page geometry (`annot_author.rs:33` states it, `:458` writes it) | **NO** — the drawing commands hold page coordinates, so a resize means **rescaling the `MarkupSpec`** and re-authoring, a different operation rather than a different argument. |

**So resize is two jobs of very different size**, and `ROADMAP.md`'s
`Pass 46.0–46.1` §3 table does not show this because it groups by
`/Rect`-carrying-ness rather than by appearance construction. **Resize is
REACHABLE, not blocked** — it was cut deliberately: the dispatch needs 1–2
`/AP` streams per type with merged-vs-`/Kids` handling, **R92** forbids a
second generator so the shared one is the target, and §12.5.5's own table calls
the anisotropic result *"stretched to fill Rect exactly … normative, not a
bug"* — meaning **a half-built resize does not fail loudly, it silently ships
distorted ticks and stretched glyphs.**

**Rule 4 in the verb's return type:** a field may own widgets on several pages
(§12.7.3.2), so `move_widget` moves **one** and returns
`siblings_left_behind`, which the CLI states in prose when non-zero. **A
`/Rect` that is absent or malformed is REFUSED, not fabricated** — §12.5.2
requires the entry, so the annotation is broken, and inventing an origin would
place the widget somewhere the file never said.

#### (3) ★★ RULING — `move_dimension`'s doc comment is WRONG ABOUT ITS REASON. **AMEND THE REASON, NOT THE CONCLUSION.** *(OWED — this filing does not hold `crates/`.)*

**The twenty-ninth filing raised this and declined to adjudicate. It is now
decided.** `EditSession::move_dimension`'s *"# Why regeneration rather than
patching `/Rect`"* block (`edit.rs:10933–10942`) says nudging `/Rect` alone
*"would slide the box and leave the drawing inside it exactly where it was —
visibly wrong at the first pixel."*

**That is FALSE, by (2)'s derivation.** A ce dimension's `/AP` carries
`/BBox = /Rect` and identity `/Matrix` (`dimension/author.rs:592`, `:607`) —
the same discipline the widgets use — so translating `/Rect` gives equal
extents, factors of 1, and a **pure translation**. **The drawing WOULD be
carried.**

**THE BEHAVIOUR IS UNCHANGED AND STAYS CORRECT**, for the reasons the
twenty-ninth filing already identified and which have nothing to do with the
appearance: **`/L`'s endpoints go stale** (§12.5.6.7 makes `/L` authoritative
for any viewer that regenerates the appearance itself) and **the `/PieceInfo`
sidecar's stored geometry goes stale**, which is what pdfce's own later edits
read. **A `/Rect`-only patch would produce a file that LOOKS right and IS
wrong** — strictly the worst outcome available, and a strictly better argument
than the one in the comment. **Owed edit: `edit.rs:10937–10939`.**

**★ The meta-observation, recorded because it is the SECOND instance in one
day.** The §12.5.5 analysis was performed to justify **not** regenerating on a
widget move. It had no stake in `move_dimension`. It nonetheless settled the
question — **the exact shape of
`D:\dev\rag\rust\a_measurement_taken_for_an_unrelated_purpose_can_adjudicate_a_dispute_it_had_no_stake_in.md`**,
whose first instance (the painting-cost adjudication) was filed earlier the
same day. **That RAG file is amended by this filing to carry the second
instance and to widen from *measurement* to *derivation*, with the honest
caveat that the two instances have different evidential strength** — the first
closed on margins (1.3% vs 75%), the second on a proof, which is only as strong
as the derivation.

#### Body sections touched

**§4 (API contract) — YES, and this is the first `Pass 46` verb to reach it.**
`EditSession` gains **`move_widget`**, returning
`WidgetMove { siblings_left_behind, … }` — an `EditSession` verb of the same
shape as the shipped `delete_field` / `delete_widget` / `rename_field`, and
§4's forms paragraph already describes that surface generically, **but
`move_widget` is NOT a forms verb** — it is the first **geometry** verb over
the annotation `/Rect` family, and that distinction is the whole reason
`Pass 46.0` was filed outside decision 020. **`--defaults-from` adds no core
type**; it is a CLI-side argument feeding the four existing creation verbs.
**Nothing is ticked for resize, for any non-widget family, or for any GUI
surface** — none exists. **The capability-level record is `FEATURES.md`'s new
*MOVE a form-field widget* row plus its amended *CREATE* / *RENAME* /
`Pass 46.0` rows, and `ROADMAP.md`'s `Pass 46.0 (PARTIAL)` entry and
`Pass 20.6` COMPLETION ADDENDUM.**

#### Concurrency, git and backup state — CHECKED, not inferred (librarian hard rule 8)

**⚠ THE DISPATCH'S OWN PREMISE WENT STALE WHILE IT WAS BEING ACTED ON, AND
THIS IS THE THIRD CONSECUTIVE FILING TO RECORD THAT.** The dispatch said *"no
engineering fork is live in `crates/`; an agent is writing to
`D:\Dev\Rag-Specialized\Inkscape_Features\`, which is **outside both your tree
and the repo**"* and gave `git status --porcelain` = **0 lines**. **By
mid-filing that was false:** `git status --porcelain` returned **5 lines**,
including `M docs/ARCHITECTURE.md`, `M docs/FEATURES.md`, `M docs/ROADMAP.md`
and an untracked `docs/decisions/032-vector-scale-mechanism-wrap-vs-rewrite.md`
— **a concurrent `pdfce-librarian` writing into `docs/`, this role's own
primary tree.** No work was lost (every edit here was an anchored insertion,
never a whole-file write), but **the dispatch's isolation claim did not hold**
and the decision-record ceiling moved underneath it.

`git rev-parse HEAD` → **`fd6eadd`**; `git remote -v` → **empty, exit 0 —
bundles remain the only copy.** Newest bundle by `ls -la D:\Dev\pdfce-backups\`
→ **`pdfce-20260807-1941.bundle`** (8,483,256 bytes, 19:41), tip **READ by
`git bundle list-heads`, not inferred from the filename** →
`refs/heads/pass-8-redaction` = **`8689f7645e262bc31cb9b09b9aaada9b48ef6466`**.
**`git rev-list --count 8689f76..HEAD` → 3.** **So the bundle is THREE behind
— the dispatch's figure is CONFIRMED rather than corrected — and `247b8fa`,
`a9f36bc` and `fd6eadd` are in NO bundle.** It becomes four behind the moment
this filing is committed.

**Checkers re-run before and after: `check-ledger-numbers.py` exit 0 / exit 0;
`check-passes-filed.py` exit 0 / exit 0.** **Ceilings after, quoted from the
checker's own output rather than restated:** *"Pass families with headings: up
to **46** (highest ID **`46.0`**)"* · *"Pass families MENTIONED: up to **46**
(highest ID **`46.2`**)"* · standing rules **`R166`** (**`R167`** next free —
**a third consecutive filing to leave it free**) · decision records **`032`**
(**`033`** next free) — **`032` was minted by the concurrent filing, not by
this one.** **No gates measured by this filing: it ran no build, no test and no
render, and every figure above is relayed from the engineer as HIS (R87).**

### 2026-08-08 (thirty-second filing, `baeb624`) — **`Pass 20.3` COMPLETES and F3 CLOSES: three engineer rulings on push buttons, and a correction to a plan that said a key was MISSING when it was WRONG**

**Nothing is minted.** `Pass 20.3` was assigned by decision 020's Backlog
amendment on 2026-08-03; this is its unbuilt remainder arriving. No
standing rule, no decision record. The rulings below sit **inside** decision
020 §F3's already-decided scope — they answer questions that record left
open, and answering an open question inside a decided scope is a ruling,
not a new decision.

**Filed by the engineer.** ~~The session forbids subagent dispatch, so~~
`pdfce-librarian`'s cross-document sweep did not happen. See
`SESSION_LOG.md`'s thirty-second filing for which locations were walked.

> **⚠ CORRECTED SAME DAY BY THE OPERATOR — the struck clause was the
> engineer's INFERENCE about its own session, not a fact, and subagents
> were in use in this window.** The consequence is unchanged and still
> true (no librarian sweep for this filing); only the reason was wrong,
> and it was wrong in a way nobody but the operator could have checked.
> **Standing consequence for this log: a constraint an agent infers about
> its own environment is not a fact and does not get recorded as one.**
> Full form in `ROADMAP.md`'s `Pass 20.3` COMPLETION ADDENDUM.

---

#### Ruling 1 — the CLI verb is `add-push-button`

The `Pass 20.2 + Pass 20.3` *Shipped* entry's own *Still owed* block said
the name *"remains NOT RULED, deliberately, and must not be inferred from
`add-radio-button`"*, on the grounds that **R161 supplies the shape and not
the word**. That caution was right and is honoured: the word is chosen, and
here is the choosing.

- ***"push button"* is the spec's own two-word term** — §12.7.4.2.2's
  heading, and Table 226's `Pushbutton` flag name. It is also Acrobat's
  label for the type.
- **`add-button` is actively wrong**, not merely shorter: a check box and a
  radio button are *also* `/FT /Btn` fields, so the unqualified noun names
  three things and creates one.
- **Hyphenating the spec's noun phrase is what the two sibling verbs
  already do** — `add-check-box`, `add-radio-button`. Consistency here is
  not decoration; the verb list is the operator's mental model of the type
  system, and a fourth verb formed differently would imply a fourth kind of
  thing.

**Cheap to overturn** — one `clap` variant name and one `println!` prefix.

#### Ruling 2 — a push button has no `Required`, and the state is UNREPRESENTABLE rather than refused

`/Ff` bit 2 (`Required`, Table 221) means *the field shall have a value at
the time it is exported*. §12.7.4.2.2 says a push button *"retains no
permanent value"* and *"shall not use the `V` and `DV` entries"*. So a
required push button asserts a condition **no operator action can ever
satisfy** — a form that can never be submitted, for a reason no viewer will
explain. Decision 027's rule is *refuse what has no good reading*; this has
none.

**★ The mechanism is the part worth recording.** The obvious implementation
is an `EditError` variant. The chosen one is a **struct with no such
field**: `NewPushButton::with_flags` takes ONE boolean where the other four
creation specs take two. Consequences:

- no runtime check, no error message to write, no branch to test;
- **the state cannot be reached by a future caller who never read this
  ruling**, which an error variant does not achieve — it only tells them
  afterwards;
- the type documents the constraint by its shape, so the doc comment
  explains *why the field is absent* rather than *why a value is rejected*.

**The discriminator for when this generalises**, since it does not always:
the state must be **nonsensical**, not merely **invalid**. An invalid state
is one an operator might legitimately attempt and deserves an explanation
of — an `Off` check-box on-state, `Edit` without `Combo`. A nonsensical one
is a shape the type should not have had. Only the second should be designed
out; designing out the first replaces a good error message with a compile
error the operator never sees.

**`read_only` IS kept**, because on a button bit 1 has a real reading: the
control renders and cannot be activated, which is exactly what one wants
for a button whose action is not yet bound.

#### Ruling 3 — a merged push-button widget keeps its OWN caption

`/MK` is a **widget** key (Table 189), so a second `add_push_button` under
an existing name gives the second view its own `/CA`. One field, one
action, two plates that may read *Submit* in a header and *Send* in a
footer.

**★ This is deliberately the OPPOSITE of the on-state rule**, and the two
are worth stating together because they look symmetrical and are not:

| | what the widgets are views of | must they agree? |
|---|---|---|
| check-box on-state | one **exported value** | **yes** — `add_check_box`'s merge branch strips `/V` and `/AS` from the incoming widget for exactly this reason |
| push-button caption | one **action** | **no** — nothing about the field's behaviour depends on the label |

The same asymmetry decides a disclosure: `field_defaults` reports an
on-state disagreement (`defaults_on_state_ambiguous`) and **deliberately
does not report a caption disagreement**. Widgets exporting different data
for one field is a defect; widgets labelled differently is a supported
arrangement, and disclosing it would train the operator to dismiss the
class the on-state disclosure belongs to.

---

#### A correction to the record, not to the code: the plan said `/I` was MISSING; it was WRONG

The `Pass 20.2 + Pass 20.3` entry listed *"**`/I` and `/TI`** (choice
selection indices) — **Pass 20.3, still PARTIAL**"*, which reads as *neither
key is written*. `/TI` genuinely was not. **`/I` had been written on every
choice fill since Pass 7.1** — in the caller's selection order where Table
231 requires ascending, and on single-select fields where Table 231 scopes
it to `MultiSelect` and adds that `/V` wins on conflict.

**★ The two states have opposite risk profiles**, which is why the
distinction is worth a decision-log entry rather than a footnote. An absent
key degrades gracefully — a reader falls back to `/V`, which is what Table
231 tells it to do anyway. A **present and wrong** key is data a conforming
reader may act on.

**How the record came to be wrong**: the plan was written from the SLICE's
acceptance criteria, and the slice had no reason to mention a key an
*earlier* slice had already touched. **A per-slice plan describes what a
slice will ADD; it is not a statement about what the codebase currently
DOES**, and it will be read as one.

---

#### The verification note, recorded because it bounds a habit the project relies on

pdfce's strongest verification is *render it and look*. It is **worth
nothing** for `/I` and `/TI`: pdfce's list-box appearance paints the
selected values, while Acrobat renders a live scrollable control from
`/Opt` **regardless of `/AP`**. Neither key changes a pixel pdfce draws.

**The general form: a verification technique's coverage is bounded by what
the verifying program consumes — and the parity target consuming MORE than
pdfce does is exactly the case where pdfce's own instruments go blind.**
Byte assertions against the saved dictionary are the only instrument that
works here (R159, arriving from a direction it was not written for).

Not minted as a standing rule on one occurrence; flagged in
`SESSION_LOG.md` as a RAG candidate awaiting a second instance.

---

#### What §12 does NOT record from this filing

The push button's structural facts — no `/V`/`/DV`/`/AS`, `/AP /N` as a
plain stream rather than a state-keyed sub-dictionary, `/MK /CA` as the
caption — are **canonical spec content**, not architectural decisions.
They live in the code's doc comments with their §-citations and in
`ROADMAP.md`'s COMPLETION ADDENDUM. The `/TI` derivation rule is likewise
an implementation of Table 231, not a choice between architectures; it is
stated in full at `EditSession::derive_top_index` and summarised in the
addendum.

---

### 2026-08-08 (thirty-fifth filing) — pdfce's FIRST image encoder ships (`jpeg-encoder` 0.7.1, R28's first exception); write-side CMYK/YCCK polarity RULED to warrant its OWN decision record (034, CLAIMED, NOT YET AUTHORED)

**Filed by `pdfce-librarian`, no shell available — relaying the
dispatching engineer's account, not independently re-run.** Closes out
the largest filing debt this project has carried: nine shipped commits
across two Pass families (`47.0`–`47.4` + `47.11`; `48.0`–`48.2`), none
previously recorded anywhere in `docs/`. Full detail in `ROADMAP.md`'s
`Pass 48.0–48.2` and `Pass 47.0–47.4 + Pass 47.11` Shipped entries and
`SESSION_LOG.md`'s thirty-fifth filing; this entry records only the
architectural decision content.

**Decision 1 — `jpeg-encoder` 0.7.1 accepted as pdfce's first image
encoder, under R28's own escape hatch.** `docs/ROADMAP.md` standing rule
R28 ("Read-compat only... no image encoder enters any pdfce crate without
a new decision record") has, since decision 005 (2026-07-30), forbidden
pdfce from WRITING any of the image codecs it reads. `Pass 48.2`
(`889566f`, 2026-08-08) exercises R28's own named exception for the first
time: `jpeg-encoder` 0.7.1, license `(MIT OR Apache-2.0) AND IJG` —
**pdfce's first conjunctive-attribution dependency** (the `AND` means
both the permissive grant and the IJG attribution condition apply
together, not a caller's choice). `simd` feature left OFF, so
`forbid(unsafe_code)` stays project-wide unbroken. **Accepted on direct,
explicit operator ruling**, verbatim and complete: *"accept the ijg
attribution line."* The attribution sentence is generated into
`about.hbs` (`cargo-about`), never hand-written, matching every other
attribution in `THIRD_PARTY_LICENSES.md`.

**§9 body update (this filing):** `docs/ARCHITECTURE.md` §9 should note
this as the second exception to "every current dependency is permissive"
kept simple by classification (MIT OR Apache-2.0 is permissive; IJG's
attribution condition is a NOTICE obligation, not copyleft) — see the
inline addition below §9's existing MIT-license paragraph.

**Decision 2 (RULED by this filing, NOT yet a full decision record) —
the write-side CMYK/YCCK polarity technique needs its OWN decision
record, not an amendment to decision 006.** The write path feeds the
**complement of true-ink CMYK samples** into `jpeg-encoder`'s
`CmykAsYcck` mode. Because that crate performs its own internal
Adobe-style inversion on encode (verified this session — a
previously-relayed characterization that it "does not invert" was WRONG,
see `D:\dev\rag\rust\jpeg_encoder_crate_inverts_cmyk_despite_doc_claim.md`),
the composition of (complement) ∘ (crate's internal inversion) reduces to
**exactly Adobe TN #5116 §13.1's forward CMYK→YCCK transform** — the SAME
transform decision 006 §4.1 already established as definitional on the
READ side (`ycck_to_cmyk_in_place`'s inverse). The written stream carries
APP14 transform byte 2; **no `/Decode` array is emitted**, because none
is needed — a reader applying decision 006's own R29 (never invert; trust
the transform byte) recovers correct true-ink CMYK with zero special
handling, identical to how it already treats every other transform-2 file
in the corpus.

**Why this is ruled as its own record rather than a 006 amendment, three
independent reasons:** (1) R28's own text makes a NEW decision record a
PRECONDITION for any encoder entering a pdfce crate at all — this is not
discretionary, R28 already says so. (2) Decision 006 §7 item 2 explicitly
listed "whether pdfce should ever write CMYK JPEGs" as future scope
**reserved**, distinct from anything §1–§6 of 006 itself decided —
treating this as a 006 amendment would conflate an exhaustively-closed
read-side record (§3's corpus survey, §3.6's TN #5116 primary read, R29–
R31) with genuinely new write-side content that never went through that
same evidentiary process. (3) The bundle here — encoder selection and its
license, the write-side transform argument, and the interaction with R29
— is substantial enough on its own terms to be independently reviewable,
the same bar every other pdfce decision record meets.

**Status: CLAIMED as decision number 034, NOT YET AUTHORED.** Decision
records in this project are produced via the `autonomous-builder`/
KenAgent protocol (`docs/decisions/README.md`) — `pdfce-librarian` records
the ruling and the ledger claim here and in `ROADMAP.md`, but does not
author `docs/decisions/034-*.md` itself. **`docs/decisions/` remains at
033 files on disk as of this filing** — 034 is a claim recorded in prose
across `ROADMAP.md`, this entry, and a `personal_rag/pdf` lesson, not yet
a file. The next engineer session should either dispatch
`autonomous-builder` to author 034 formally from this content, or
explicitly release the claim if this ruling is overturned, before any
unrelated work takes the number 034.

**What this filing does NOT decide:** whether pdfce should ever write a
transform-0 (raw CMYK, no YCCK) JPEG — the complement technique recorded
here is specific to the YCCK write path and to this specific encoder's
internal behaviour; a future transform-0 write path would need its own
polarity argument. Also not decided: any GUI-facing quality control for
`Jpeg{quality}` (folds into `Pass 48.3`'s scope when that Pass is
started).

**Cross-references:** `C:\personal_rag\pdf\lesson_20260808_write_side_cmyk_ycck_polarity_feed_complement_for_forward_transform.md`
(the empirical/technique writeup); `D:\dev\rag\rust\jpeg_encoder_crate_inverts_cmyk_despite_doc_claim.md`
(the crate-behaviour correction); `docs/ROADMAP.md`'s R28 annotation and
`Pass 48.0–48.2` Shipped entry (the ledger record).
