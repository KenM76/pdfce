# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**: no
*"this paragraph read X until…"*. What is true now, plus a pointer.
Corrections and their prior wording live in the **append-only** record —
`ROADMAP.md` and `SESSION_LOG.md` — where a claim is dated and no later edit
can falsify it.

---

## §A — COLD START: everything you need, in one screen

**`HEAD` is `a73e3be`, which is also the `v0.9.0` tag. The working tree is
clean, `origin/main` matches, and nothing is blocked on anybody.** Both
FeatureRequests channels were checked; the newest inbound is
`iccce`'s `note_your_name_gate_has_the_two_defects_mine_had.md`, which was
**acted on and closed** this session — see §0.

**★ `v0.9.0` IS RELEASED AND VERIFIED.** `python tools/verify-release.py
v0.9.0` → **7 of 7 ok, exit 0**; CI run `32888354436` green on all ten jobs at
the tagged commit. The previous release's own filing recorded **6 of 7, exit
1**. This is the first release since `v0.7.0` whose tagged commit's CI run is
green, and it is green **without anyone remembering an ordering discipline** —
which is what `bb154ed` was for, now demonstrated on a real release rather
than argued for. Both operator items in §5 are discharged **for this act
only**.

### What this run built (2026-08-25, afternoon)

| commit | what an operator would notice |
|---|---|
| `3681a7f` | **`Pass 125.0` — gradient MESHES render.** The operator's own report: two failing cells in the first box of the print-conformance suite's first page. Both were `ShadingType 7` tensor-product patch meshes, recognised and refused. All four mesh types (4/5/6/7) now decode and rasterise |
| `4b22c95` | **The fuzz harness had not linked ON WINDOWS since OCR landed.** Repaired, plus a `mesh_shading` target: 1 107 957 runs in 91 s, no crash. ★ **Its own stated reason was wrong and `ccf9ed3` corrects it — read that one too** |
| `525585e` | **The suite-name scrub gate published the term it suppresses, and could not see the commit it was gating.** Both found by `iccce`. `R218` minted |
| `3016641` | A test assertion shipped with a ten-space hole — the second heredoc-eaten line continuation this session |
| `42bcea0` | The `Pass 125.0` filing. ★ **Its own first draft breached the scrub rule in four lines, two of them inside a verbatim quotation of the operator** |
| `ccf9ed3` | ★★ **CI DOES run `cargo fuzz build` — on Linux.** The break was MSVC-only, so a green CI and a broken local build were never a contradiction. Also: `ci.yml` claimed "the eight cargo-fuzz targets"; there are 24 |
| `e115947` / `a73e3be` | **`v0.9.0`** — the version bump and the release filing. The tag is on the second of these |

### ★★★★★ `Pass 125.0` — the numbers, because they are the evidence

`PCS 6.0`'s four cells each pair a live shading with a bitmap of what a
correct render produces — the patch's own printed criterion is *"The shadings
should look like the reference image"* — so scoring it costs **no external
reference at all**. Mean absolute per-channel difference and Pearson
correlation, shading versus its own reference image:

| cell | type | before | after | Acrobat, same file |
|---|---|---|---|---|
| `a` | 7 (mesh) | 125.9 / **−0.044** | 10.0 / **0.949** | 7.6 / 0.981 |
| `b` | 3 (control) | 6.2 / 0.9985 | 6.2 / 0.9985 | 4.5 / 0.9979 |
| `c` | 2 (control) | 0.9 / 0.9997 | 0.9 / 0.9997 | 1.1 / 0.9998 |
| `d` | 7 (mesh) | 128.6 / **−0.044** | 1.9 / **0.997** | 3.4 / 0.997 |

Cell `d` is **closer to its reference than Acrobat's own render is**.

★ **THE RESIDUAL ON CELL `a` IS NOT A MESH DEFECT, and this forecloses a
Pass.** pdfce's *mesh* differs from Acrobat's *mesh* by essentially the same
amount pdfce's *plain raster image* differs from Acrobat's *plain raster
image* on the same page — **26.5 vs 29.6** on cell `a`, **27.1 vs 26.7** on
cell `d`. The reference image contains no mesh. ⇒ the residual is the
`DeviceCMYK`→sRGB path, which is `iccce`'s by decision 064 and is already
routed there as `Pass 122.7`. Do not chase it inside `mesh.rs`.

**Suite board `8 FAIL / 27 pass / 16 UNRESOLVED` → `7 FAIL / 27 pass / 17
UNRESOLVED`.** `PCS 6.0` leaves the trap-X population and lands in the
reference-strip bucket the harness cannot adjudicate (`ref? strip corr=0.84`).
★ **Do not read 0.84 as a grade** — `PCS 6.1` renders perfectly and scores
**0.371** on the same metric. The strip correlation is uncalibrated; the
harness says so itself. The cell table above is the evidence.

**Blast radius, measured:** 1 of 51 suite patches contains a mesh; **0 of
3 735** external-corpus PDFs contain one on page 1. Nothing without a mesh
reaches the new code — entry is gated on `Geometry::Mesh`.

### The two ambiguities, handled the two different ways they deserve

This contrast is the transferable part.

- **`MSH-A1`** — the byte-padding unit for a type 6/7 *patch* record.
  §8.7.4.5.5 scopes its rule to a **vertex** and a patch has none; ISO
  32000-2 repeats the sentence word for word, so it is **permanent**. Two
  defensible answers ⇒ **a setting**: `mesh_patch_padding` ∈ `per_record` |
  `none`, defaulting to `per_record`. Threaded Settings → `RenderOptions` →
  `RenderPolicy` → parser, with a settings-window row and a fixture
  (`type6_unaligned.pdf`, widths 4/12/4) that makes it observable.
- **`MSH-A3`** — the subdivision density for a patch. Also unspecified, and
  **deliberately NOT a setting**: the right value is a function of zoom, so a
  knob is one the operator cannot set correctly. Taken from the patch's
  device-space hull (~4 px per cell).

### ★★ THE THREE THINGS MOST LIKELY TO BE MISREAD

1. **A mesh still bridges through sRGB.** It resolves its colour before
   compositing, so on an ink page it is bridged like any other shading
   (`cmyk_bridged_pixels`) and **its overprint is not represented**. That is
   the mesh half of `Pass 97.1k`. A row that lists "images and ramps" without
   naming meshes now under-reports.
2. **`ShadingType 1` is still not painted.** `FEATURES.md`'s old *Planned*
   row covered types 1 **and** 4–7; it was split, and the type 1 half stands.
3. **The crack margin is restricted to unpainted pixels, and that
   restriction is load-bearing.** Applied unconditionally it closed the crack
   and moved the cell's correlation the **wrong way** (0.9485 → 0.9347),
   because primitives paint in stream order and each then overwrote a third
   of a pixel of its predecessor — shifting every interior colour boundary
   downstream. Do not "simplify" it.

---

## §0 — THE SCRUB RULE CAN BE BREACHED BY QUOTING THE OPERATOR. Read once.

`iccce` reported two defects in `tools/check-suite-name-absent.py`, both
found on its own copy of the same gate. Both were real; both are fixed
(`525585e`), and `R218` was minted:

> **`R218` — a gate whose input set is "what is already committed" cannot see
> the commit you are about to make.**

Run locally *before staging* — which is when anyone naturally runs a
verification — `git ls-files` and a bare `git grep` exclude precisely the
files the session has just written, **the only files that could have
introduced a new violation**. A/B'd with a probe file: pre-fix **exit 0,
"clean"**, on a file violating the rule twice.

★★ **Then the gate caught this session's own filing.** Four lines, unstaged,
two of them inside a **verbatim quotation of the operator's own words** —
which is how the whole Pass started and is worth recording. Nobody did
anything wrong: the engineer quoted him exactly, and the librarian filed what
it was handed. ⇒ **The elision has to happen at the DISPATCH boundary**,
where operator speech enters a tracked document. Not minted (n = 1); recorded
as a named candidate in `SESSION_LOG.md`'s 254th filing, so a second
occurrence triggers it.

**Owed, filed as an unscoped Backlog note:** audit the other seventeen
`tools/check-*` scripts against `R218`. It is a shape, not a property of one
script.

★ Note also what the gate **cannot** see: a `.gitignore`d file. A stale
`tools/__pycache__/*.pyc` carried the term in its own filename and needed an
eye, not a gate.

---

## §1 — THE PRE-FLIGHT CHECKS

**1. `ls` BOTH FeatureRequests channels.** They are outside this repository,
so **no gate will ever contradict a stale sentence about them — including
this one.**

```
D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\
D:\Dev\FeatureRequests\iccce_FeatureRequests\open\
```

**2. Run the gates — `ls tools/check-*`, do not trust any list.** `R209`:
*"all gates green" names a set, and the set somebody runs is not the set CI
runs.* At the time of writing: **18 on disk, 17 runnable as bare gates, all
17 exit 0.** The 18th, `check-image-colorspace-truth.py`, takes a
fixture-directory argument and is **not** a gate. Count them; do not quote a
count.

**3. ★★ RUN `cargo +nightly fuzz build` ON WINDOWS. CI CANNOT DO IT FOR
YOU, AND `cargo check --bins` IS NOT A SUBSTITUTE EITHER.** Two separate
traps, and each defeats the obvious escape from the other:

- **CI's `fuzz targets build (nightly)` job runs on `ubuntu-latest`.** It has
  been green throughout, including while the harness was completely
  unbuildable here — `rten` declares `crate-type = ["lib", "cdylib"]`, and
  only Windows hands that cdylib libFuzzer's `/include:main`. A green CI does
  **not** absolve a red local; on this job the local run is the stricter one.
  A `windows-latest` sibling is filed to Backlog and does not exist yet.
- **`check-ci-parity.py --list` offers `cd fuzz && cargo check --bins` as the
  local stand-in.** A/B'd 2026-08-25: **it passes in BOTH states**, because
  `cargo check` never links and the break was a link break. **A cheap proxy
  for a gate is a proxy for the part of the gate that is cheap.**

Run the real thing at least once per session that touches `fuzz/` **or any
crate's `Cargo.toml`** — a dependency change *is* a fuzz-harness change,
which is how this went unnoticed. About 5–6 minutes warm for all 24 targets.

**4. Read `docs/compositor-plan.md`** before scoping anything in `97.x`.

**5. ★ BEFORE ANY PACKAGING OR RELEASE BUILD, CHECK FREE SPACE.**
`df -h . && du -sh target target/debug`. On 2026-08-23 a packaging run died
on a full disk with `target/debug` at 103 GB. Delete **`target/debug` only**
— never `cargo clean`, which also drops the warm `target/release`. At the
time of writing: **88 % used, 124 GB free**, `target/debug` 44 GB,
`target/release` 4.6 GB.

**6. The suite corpus is at `$PDFCE_SUITE_DIR` (51 patches) and Acrobat
reference renders of all 51 at `$PDFCE_SUITE_REFS`.** ★ **The private map's
`manifest.json` names a `suite_dir` that does not exist** — the corpus is one
directory over. Neither variable is set in a fresh shell; read the manifest,
then verify the path with `ls` before believing it.

---

## §2 — ★★★★★ THE METHODOLOGY LESSONS FROM THIS RUN

**1. An equivalence the STANDARD requires beats a blessed screenshot.** A
rendered gradient has no obviously-right answer, and a committed "known good"
PNG pins whatever the code did that day (`R215`). The mesh tests instead pair
files that must agree **by different code paths**: a Coons patch and the
tensor patch its boundary implies (`MSH30` — and the fixture generator's own
independent transcription of those equations is what makes agreement a check
of the renderer's), and a type 4 flag sequence against a type 5 lattice
(`MSH19` vs `MSH22`, two parsers, one with no flag field at all). **And the
guard that keeps that from being vacuous is asserted too:** a bilinear patch
interior and a Gouraud triangle pair must **differ**, or both equivalences
are comparing two renders of nothing.

**2. ★ A fixture generator is a second implementation, and it gets things
wrong the same way.** Its first draft authored continued patches as if the
inherited edge arrived forward; flags 2 and 3 hand it over **reversed**. The
result was a pair of lens shapes with a hole between them **that looked
enough like "a gradient" at a glance to need a second look**.

**3. ★ A test's expected value can be derived from the wrong model and look
plausible.** The parametric test first expected greys of 85 and 117 and would
have **failed a correct renderer**: it applied an sRGB transfer curve to the
function's output, but a `DeviceRGB` component in a PDF **is** the device
value (§8.6.4.3). Correct: 28 (right order) versus 47 (wrong order). The
failure arrived from the direction of the *assertion*, not the code.

**4. A defect can present as something else entirely.** One unpainted pixel
in a 60×60 cell — and what showed through was the suite's own failure marker,
drawn **underneath** the shading. A crack in a gradient does not read as a
crack; it reads as a stray black speck in the artwork.

**5. Sample across the PREDICATE.** The patch subdivision density is chosen
from device size, so it **changes with scale**. Watertightness was verified at
1, 2, 4, 8 and 16 — not at "a reasonable range" (`R211` clause (e)).

**6. Verify each INSTANCE, not the class.** Eleven of the twelve mesh tests
reached the mesh by `sh`. A shading arrives two ways, and they anchor in
**opposite** coordinate spaces; the twelfth test covers `PatternType 2` and
was added for exactly that reason.

**7. Never put Rust through a shell heredoc.** Two line continuations lost
their backslash this session; `tools/check-string-gaps.sh` caught both.
★ The second survived a full gate sweep because the sweep ran **before** the
file's last edit — `R218` one scale down. Every edit made through a script
**Written to a file** was fine.

---

## §3 — THE QUEUE, in the order I would take it

**Nothing in this queue is blocked on anybody.**

1. **The trap-X cells — `PCS 1.0`'s `a`/`b`/`f`/`g`.** The `/Separation /All`
   hypothesis: §8.6.6.4 says painting with `/All` applies the tint to **all
   available colorants at once**, which a screen neutral cannot do; `122.5`
   gave those pages a colorant buffer, so it is now possible. **Confirm the
   content-stream question first** — that those marks really *are* the `/All`
   paint is still unestablished, and reading a co-occurring counter as a
   cause is what §2a of the previous handoff was about. `125.1` is the next
   free ID.
2. **Mesh shadings in ink — the mesh half of `Pass 97.1k`.** Newly relevant:
   meshes are a **new** population bridging through sRGB. `Patch` corners
   hold a resolved `Shade::Rgb`; carrying authored colorants alongside is the
   same shape `ColorRamp::at_cmyk` already took for analytic ramps in
   `122.6`, and would let a mesh honour overprint.
3. **Mesh cells `e`/`j` on `PCS 1.0`** — the shading colour path
   (`ColorRamp::at` resolves to sRGB when the ramp is *built*). Long-standing
   and structural; unrelated to `125.0`.
4. **Per-image overprint (`Pass 122.1`)** — the image sibling of `122.6`.
   Diagnosed: it is why `PCS 8.2`'s check mark is missing. pdfium fails it too.
5. **Wire `cargo +nightly fuzz build` into CI** (build only, ~4 min warm).
   The structural fix for §1 item 3; a reminder is not one, for the same
   reason the release-ordering discipline was replaced structurally in
   `bb154ed`.
6. **Audit the other seventeen `tools/check-*` against `R218`.** Filed as an
   unscoped Backlog note.
7. **Build `R214`'s positional-reference gate** — a grep over a closed
   vocabulary in doc comments. **Measure its baseline first, repair, then
   wire — never wire it red.** `R216`'s companion vocabulary rides the same
   instrument; build **one** script with two vocabularies.
8. **`Pass 122.0`** — multithreading, the operator's own request. ★★ Read §2
   item 5 and `R215` before writing its acceptance table: *"byte-identical
   output at any core count"* is a differential claim over an **unbounded**
   parameter. Sample the switch points (1, 2, `n-1`, `n`, oversubscribed).
   Decision 080 adds a compile-time target gate, because `std::thread` and
   `rayon` both `cargo check` cleanly for `wasm32`.
9. **`Pass 119.1`** — `unshare_form`. Carried unstarted through eleven
   handoffs now.
10. **`Pass 122.3`** — the colorant buffer's byte ceiling. A **full-page**
    render above ~375 DPI refuses the buffer and silently composites in the
    wrong space, so one page can have different colours at different
    resolutions. Interactive use is unaffected.
11. **`R215`'s retro-application** — not started. Any Pass filed with a
    *"required after"* column must be re-read against `R215` before that
    column is used as a gate. Runs over `docs/` **and both RAG tiers**.

---

## §4 — STANDING NOT-DONE LIST, named so it does not read as done

Known gaps in shipped behaviour. None is a regression; each is a capability
pdfce does not have yet.

- **Mesh shadings bridge through sRGB** into the colorant buffer, so their
  overprint is not represented. New this session; queue item 2.
- **`ShadingType 1`** (function-based) is modelled, ramped and **not
  painted**. Zero occurrences measured in the suite corpus.
- **Mesh anti-aliasing**: `/AntiAlias` is a hint and defaults false, so a
  mesh's outer silhouette is hard-edged. Interior edges are seamless.
- **`resolve_indexed`** builds its palette with a **scratch
  `ColorDiagnostics` that is discarded**, so a tint failure inside a palette
  never reaches the operator.
- **Implicit knockout**: only explicit `/K true` is honoured. `/TK` defaults
  true (every text object), and `B`/`b` and shading patterns are knockout.
  **The one pdfce implements is the rarest.**
- **`/TR` on a soft mask** is read, counted, never evaluated.
- **`/AIS true`** is not distinguished from `/AIS false`.
- **Spot colorants** — four planes, not runtime `N`. Every remaining
  trap-criterion suite FAIL is in this bucket.
- **Rendering intent** (§11.7.5.3, §8.6.5.8) — **pdfce carries NONE.** The
  `ri` operator is an explicit no-op and `/RI` in an `/ExtGState` is not read
  at all. `settings::CmykIntent` is **not** an ICC rendering intent: it
  selects which fixed `DeviceCMYK`→sRGB table is used, per-invocation.
  Measured (`tools/intent-census/`): 12 of 51 suite print-production files
  name an intent, 1 names two; a byte-grep that does not inflate streams
  finds 0, so any future scan must inflate first. Still unmeasured: whether a
  single *page* switches intent mid-stream.
- **A non-isolated group whose SECOND buffer cannot be allocated** still
  falls back to isolated semantics — counted, disclosed, and now the *only*
  way a non-isolated group reaches `cmyk_groups_approximated`.
- **No GUI code path reads** `forms_culled`, `subpixel_culled`,
  `annots_out_of_scope`, `page_content_suppressed`, `render_page_region`, the
  display list, or any of the three new `mesh_*` counters, **and no GUI
  exposes `--fast-subpixel`.** GUI work is paused; recorded so the `[ ] gui`
  boxes in `FEATURES.md` are not mistaken for oversights.

---

## §5 — THE TWO OPERATOR ITEMS

- **PUSHING AND RELEASING — BOTH DISCHARGED 2026-08-25, AND NEITHER CARRIES
  FORWARD.** The operator said **"release when ready"**, unprompted, after
  being briefed on `Pass 125.0`. `v0.9.0` was tagged at `a73e3be` (a librarian
  filing commit, deliberately), `main` pushed `0d4165e..a73e3be` as a clean
  fast-forward, and the release published with one asset,
  `pdfce-v0.9.0-windows-x64-portable.zip` (11,180,078 B).
  **The next push and the next release each need their own go-ahead**
  (`CLAUDE.md` rule 8) — one authorisation covers one act.

  ★ **Only ONE asset, and the reason is a licence one.** `v0.8.0` shipped a
  demonstration PDF beside the zip. This release's headline artefact is a
  render of a **licensed** conformance patch whose artwork must not be
  redistributed (`docs/LEGAL.md` §5, plus the 2026-08-25 name ruling), so
  none was attached. The twelve **synthetic** mesh fixtures that demonstrate
  the same capability are MIT-clean and in the repository at
  `fixtures/synthetic/mesh/`. **Do not attach a render of a suite patch to a
  release, ever** — the temptation is real, because it is the best picture
  of what shipped.

  ★★ **The packaging smoke test was run on the ZIP, not on the build
  folder**, and that distinction is the point: the zip was extracted to a
  fresh path and `pdfce-cli.exe` run from the extraction, producing a render
  **byte-identical** (same MD5) to the development build's. The artefact
  people download is verified, not merely built.
- **CUTTING A BACKUP — STILL NOT DISCHARGED.** A GitHub release is an
  offsite copy of one build and one PDF; it is **not a `git bundle`** and
  does not contain the history. **Re-measure before quoting** —
  `ls D:/Dev/pdfce-backups/`, `git bundle list-heads <newest>`,
  `git rev-list --count <bundle-head>..main`. Do not carry a number forward
  from any previous handoff; each was true at a different `HEAD`.
