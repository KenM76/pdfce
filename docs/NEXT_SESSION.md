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

**`HEAD` is `8567647`, which is also the `v0.10.0` tag. The working tree is
clean, `origin/main` matches, and nothing is blocked on anybody.** Both
FeatureRequests channels were checked; the newest inbound is `iccce`'s
`note_your_name_gate_has_the_two_defects_mine_had.md`, **acted on and closed**
— see §0.

**★ `v0.10.0` IS RELEASED AND VERIFIED.** `python tools/verify-release.py
v0.10.0` → **7 of 7 ok, exit 0**; CI run `32901516410` green at the tagged
commit. Second consecutive release at 7 of 7, and the first to get there
**after** a red run rather than instead of one — see §0a, which is the most
useful thing on this page.

### What this run built (2026-08-25)

Two features and four repairs, in one day.

| commit | what an operator would notice |
|---|---|
| `3681a7f` | **`Pass 125.0` — gradient MESHES render.** Two failing cells in the first box of the print-conformance suite's first page, reported by the operator. Both were `ShadingType 7` tensor-product patch meshes, recognised and refused. All four mesh types now decode and rasterise |
| `1a5fc92` | **`Pass 126.0` — TYPE 3 FONTS render.** Glyphs that are content streams rather than a font program. **49 of 49** corpus files carrying one now show their text; one drew a heading that was an entirely blank page that morning |
| `0f09780` | **`Pass 126.1`** — the bitmap flavour, and the measurement that it is never smoothed at any zoom, which matches Acrobat |
| `4b22c95` / `ccf9ed3` | The fuzz harness had not linked **on Windows** since OCR landed. Repaired — and the second commit corrects the first's stated REASON, which was wrong |
| `525585e` | The suite-name scrub gate published the term it suppresses, and could not see the commit it was gating. Both found by `iccce`. `R218` minted |
| `8567647` | The `v0.10.0` filing, and the tag |

### ★★ THE TWO FEATURES, IN THE NUMBERS THAT MATTER

**`Pass 125.0` — mesh shadings.** `PCS 6.0`'s two type 7 cells, scored against
the reference images the patch prints beside them:

| cell | before | after | Acrobat, same file |
|---|---|---|---|
| `a` | corr **−0.044** | **0.949** | 0.981 |
| `d` | corr **−0.044** | **0.997** | 0.997 |

Cell `d` is closer to its reference than Acrobat's own render is. ★ **The
residual on `a` is NOT a mesh defect** — pdfce's mesh differs from Acrobat's
mesh by the same amount pdfce's *plain raster image* differs from Acrobat's on
the same page (26.5 vs 29.6). It is the `DeviceCMYK`→sRGB path, which is
`iccce`'s by decision 064. **Do not chase it inside `mesh.rs`.**

Suite board `8 FAIL / 27 pass / 16 UNRESOLVED` → **`7 FAIL / 27 pass / 17
UNRESOLVED`**. ★ `PCS 6.0` now reports `ref? strip corr=0.84`; **that is not a
grade** — `PCS 6.1` renders perfectly and scores **0.371** on the same
uncalibrated metric.

**`Pass 126.0`/`126.1` — Type 3 fonts.** All 49 corpus files render,
`unsupported_type3=0`, `type3_glyphs_missing=0`, no panics. The one question
nobody had recorded — whether Acrobat honours Table 113's rule that a `d1`
glyph's own colour operators "shall be ignored" — was **measured in Acrobat
before any code was written**: `d1` blue, `d0` red, image-mask blue. Acrobat is
spec-conformant on every point tested, so **§9.6.5 and Acrobat parity are the
same criterion here**.

### The two ambiguities### The two ambiguities, handled the two different ways they deserve

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

## §0a — ★★★ THE FILING COMMIT MUST BE THE LAST COMMIT BEFORE THE TAG

**Read this before cutting a release. It cost a red CI run and a re-tag.**

`v0.10.0` was tagged, pushed and published, and CI came back **red** on
`check-commits-filed.py`: the version-bump commit was in no filing.

The sequence that produced it: dispatch the librarian → commit the version
bump → commit the librarian's filing on top. **The filing was written before
the commit it needed to cite existed.**

★ **And the gate sweep run beforehand was green, honestly.** At that moment
the bump was the **tip**, and `bb154ed`'s tip-deferral correctly excuses a
commit that cannot cite its own hash. The instant the filing landed on top,
the bump stopped being the tip and became real debt. **The gate went from
green to red without anything about that commit changing.**

The librarian classified this as a fourth occurrence of **`R217`**'s shape
(tip-deferral evaporating once a commit lands on top) rather than `R218`'s,
**correcting the framing I handed it** — `R218` is a tracked-versus-untracked
blind spot with no analog here. Recorded as a named candidate at n = 1, not
minted.

> **The rule: the librarian's filing commit must be the LAST commit before the
> tag.** Any code commit made after the dispatch has, by construction, no
> filing that can name it. `bb154ed` made *tag-on-a-code-commit* safe; nothing
> makes *file-then-append-code* safe except ordering.

**The recovery, which has precedent** (`v0.8.0`, 241st filing): file the
orphan, re-tag at the filing commit, force-push the tag, **rebuild the
package** so its `BUILD-INFO.txt` names the tagged commit rather than a
superseded one, replace the asset with `--clobber`, and **re-run the smoke
test on the new artefact** — a re-cut release is a new artefact and does not
inherit the old one's test.

★ The rebuild is a judgement, not a step. Leaving it would have shipped a
`BUILD-INFO.txt` naming a commit that is not the tag — a small dishonesty no
gate would ever have caught.

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

1. **Type 3 text EXTRACTION and search** — `/ToUnicode`-gated, filed to
   Backlog and **explicitly marked as not requested by the operator**. It is
   first in this queue anyway, and the reason is a support one rather than a
   technical one: `FEATURES.md` now shows Type 3 under *Implemented*, and a
   reader will reasonably assume searching works. It does not. A Type 3 glyph
   name carries no intrinsic Unicode meaning, so extraction depends entirely
   on the file carrying a `/ToUnicode` CMap — which is the same thing Acrobat
   depends on, so this is parity rather than catch-up. Small.
2. **The trap-X cells — `PCS 1.0`'s `a`/`b`/`f`/`g`.** The `/Separation /All`
   hypothesis: §8.6.6.4 says painting with `/All` applies the tint to **all
   available colorants at once**, which a screen neutral cannot do; `122.5`
   gave those pages a colorant buffer, so it is now possible. **Confirm the
   content-stream question first** — that those marks really *are* the `/All`
   paint is still unestablished, and reading a co-occurring counter as a
   cause is what §2a of the previous handoff was about. `125.1` is the next
   free ID.
3. **Mesh shadings in ink — the mesh half of `Pass 97.1k`.** Newly relevant:
   meshes are a **new** population bridging through sRGB. `Patch` corners
   hold a resolved `Shade::Rgb`; carrying authored colorants alongside is the
   same shape `ColorRamp::at_cmyk` already took for analytic ramps in
   `122.6`, and would let a mesh honour overprint.
4. **Mesh cells `e`/`j` on `PCS 1.0`** — the shading colour path
   (`ColorRamp::at` resolves to sRGB when the ramp is *built*). Long-standing
   and structural; unrelated to `125.0`.
5. **Per-image overprint (`Pass 122.1`)** — the image sibling of `122.6`.
   Diagnosed: it is why `PCS 8.2`'s check mark is missing. pdfium fails it too.
6. **A `windows-latest` sibling for CI's fuzz job.** ★ Not "wire fuzzing
   into CI" — **it is already wired**, as `fuzz targets build (nightly)`, and
   it has been green throughout. It runs on `ubuntu-latest`, and the break
   found today was MSVC-only, so the job is structurally blind to the entire
   class. Until the sibling exists, a Windows `cargo +nightly fuzz build` is a
   per-session discipline and not a gate.
7. **Audit the other seventeen `tools/check-*` against `R218`.** Filed as an
   unscoped Backlog note.
8. **Build `R214`'s positional-reference gate** — a grep over a closed
   vocabulary in doc comments. **Measure its baseline first, repair, then
   wire — never wire it red.** `R216`'s companion vocabulary rides the same
   instrument; build **one** script with two vocabularies.
9. **`Pass 122.0`** — multithreading, the operator's own request. ★★ Read §2
   item 5 and `R215` before writing its acceptance table: *"byte-identical
   output at any core count"* is a differential claim over an **unbounded**
   parameter. Sample the switch points (1, 2, `n-1`, `n`, oversubscribed).
   Decision 080 adds a compile-time target gate, because `std::thread` and
   `rayon` both `cargo check` cleanly for `wasm32`.
10. **`Pass 119.1`** — `unshare_form`. Carried unstarted through eleven
   handoffs now.
11. **`Pass 122.3`** — the colorant buffer's byte ceiling. A **full-page**
    render above ~375 DPI refuses the buffer and silently composites in the
    wrong space, so one page can have different colours at different
    resolutions. Interactive use is unaffected.
12. **`R215`'s retro-application** — not started. Any Pass filed with a
    *"required after"* column must be re-read against `R215` before that
    column is used as a gate. Runs over `docs/` **and both RAG tiers**.

---

## §4 — STANDING NOT-DONE LIST, named so it does not read as done

Known gaps in shipped behaviour. None is a regression; each is a capability
pdfce does not have yet.

- **Mesh shadings bridge through sRGB** into the colorant buffer, so their
  overprint is not represented. New this session; queue item 3.
- **Type 3 text cannot be extracted, searched or edited.** Rendering ships;
  extraction is `/ToUnicode`-gated and is queue item 1. Editing is not
  planned and Acrobat has no in-place path for it either.
- **A Type 3 glyph is not culled against `/FontBBox`.** Table 112 makes
  `[0 0 0 0]` a *sentinel* meaning "assume nothing", and a nonzero box that is
  wrong makes the result "unpredictable" rather than clipped, so pdfce uses it
  for nothing. `Type3Font::bbox_is_sentinel` exists so that a future caller
  that wants to cull has to ask rather than discover the sentinel by erasing
  every glyph of a font that used it.
- **Acrobat's behaviour on MALFORMED Type 3 is unmeasured**, and the parity
  corpus says so rather than guessing: a procedure whose first operator is
  neither `d0` nor `d1`; whether Acrobat still advances on a missing
  `/CharProcs` key; a `d1` procedure containing a full-colour image; whether
  its `/Resources` page fallback actually happens; a wrong `/FontBBox`; any
  recursion limit. pdfce implements all of these from the clause plus its own
  robustness posture, which is stated rather than presented as parity.
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

- **PUSHING AND RELEASING — DISCHARGED TWICE ON 2026-08-25, AND NEITHER
  CARRIES FORWARD.** Two separate authorisations, each covering one act:
  *"release when ready"* → **`v0.9.0`** at `a73e3be`; *"release new version
  when done"* → **`v0.10.0`** at `8567647`. Both tagged on a librarian filing
  commit, both verified **7 of 7**.
  **The next push and the next release each need their own go-ahead**
  (`CLAUDE.md` rule 8) — one authorisation covers one act, and two in a day
  does not make a standing one.

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
