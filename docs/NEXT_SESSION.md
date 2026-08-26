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

**`v0.12.0` is released and verified 7 of 7.** Both FeatureRequests channels
were checked; pdfceGUI has consumed everything sent, wired the newest verb and
the standards selector, and sent findings back.

★ **`main` is AHEAD OF `origin/main` and that is deliberate.** Work continued
after the release under an autonomous loop, and **pushing needs its own
go-ahead** (`CLAUDE.md` rule 8) — the *"release and commit"* authorisation was
spent on `v0.12.0`. Do not push to clear the divergence; ask.

★ **A version bump may be owed before the next tag, and it is an open call.**
`Pass 129.1` changed a shipped default's VALUE (`ocr --dpi` 300 → 150), which a
scripted caller can observe, while adding no public core item. The `768e934`
precedent (*"a new callable verb is not a patch"*) settles the additive case
and does not settle this one. Decide before tagging, not after.

### What this run built (2026-08-25 evening → 2026-08-26)

| commit | what an operator would notice |
|---|---|
| `2104d38` `35fce5f` | **`Pass 127.0`** — Type 3 text searches and copies; where it cannot, each font is **named** |
| `de2d93c` | **`Pass 128.0`** — image quality matches Acrobat again, and a Type 3 stencil is exempted from the change |
| `1f79cc1` | **`Pass 128.1`** — render presets per subset standard, each value carrying its evidence tier |
| `181d9bd` | **`Pass 129.0`** — **OCR works.** It never had |
| `9b941b9` | **`Pass 127.1`** — "redact every match" stops being silent about text it could not read |
| `81d1e30` `b6f8cd5` | `v0.12.0` — **briefly tagged at a commit CI rejected**; see below |
| `a72a89b` `d68c621` | the OCR downloader made **opt-in**; a default build now carries **no network code at all** |
| `7ad8b00` | **`Pass 129.1`** — `ocr --dpi` default 300 → 150. The old default was the **worst of five** |
| `a3185ba` | two doc findings back from pdfceGUI, one about how a doc comment gets misread |

★ **`v0.12.0` was tagged twice.** The first tag went on `b6f8cd5`, where CI
failed the macOS cross-compile check: the OCR downloader was a DEFAULT feature
and pulled a TLS stack that cannot cross-compile. Re-cut per §0a's recovery —
re-tagged at the later filing commit, force-pushed, package rebuilt so
`BUILD-INFO` names the tag, asset replaced, **smoke test re-run on the new
artefact** (a re-cut release does not inherit the old one's test).

### ★★★ THE ONE THING TO CARRY OUT OF THIS RUN

**pdfce shipped an OCR engine on 2026-08-12 that produced garbage on every
page, and nothing noticed for two weeks.** The cause was one wrong model file.
What made it survivable was that it was *never measured end to end* — there
was no `pdfce-cli ocr`, so the only way to run it was a GUI, and the GUI's own
end-to-end test was `#[ignore]`d because the weights were not beside its
binary.

Three separate absences, each individually reasonable, and together they made
a completely broken feature indistinguishable from a working one:

1. no shell surface ⇒ nobody could run it cheaply;
2. no shipped weights ⇒ the one surface that existed refused before it ran;
3. no ground truth ⇒ even a run that happened produced a *count*, and a count
   cannot tell reading from hallucinating.

**A feature with no cheap way to run it is a feature nobody has run.** When a
capability lands with its surface deferred, the surface is not polish — it is
the only thing that will ever tell you the capability works.

### The measurement, and how to get one when licensing forbids the input

`ROADMAP.md` had recorded that OCR accuracy could not be measured because the
project may not check in a real scan (`LEGAL.md` §5, rule 7). **So the scan was
manufactured**: render a vector page pdfce authored, degrade it the way a
sheet-fed scanner does (200 dpi, box blur, 0.35° skew, deterministic noise,
paper grey), wrap it as an image-only PDF.

★ **The ground truth falls out of the generator.** The vector original says
where each word really is, via `find-text` and real font metrics; the OCR'd
copy says where OCR put it. Two rectangles, **two completely different
routes**, which must agree — and that catches the failure a word count cannot:
a layer that reads every word perfectly and lands in the wrong place.

    degraded  47/47 words, 100% content, median offset 2.56 pt
    clean     43/47 words,  91.5%,       median offset 0.90 pt

The clean control's **0.90 pt** is the true positional accuracy. The degraded
figure is dominated by the deliberate skew, not by OCR error — the truth is
the *unskewed* original. **The control is what makes that readable**, and it is
the load-bearing half of the fixture.

★ Reported honestly and still unexplained: the **clean** control scores worse
on recall (91.5 % vs 100 %). Four words missed on crisp text and found on
degraded text. Reproducible. Nobody knows why.

### What else is worth knowing before you start

- **`/Rotate` was ignored by the whole OCR chain** while the rasteriser
  honoured it — a transposed invisible layer on a page that still looks
  perfect. Fixed via `PagePlacement` + `words_to_page_space_on`. Scanner
  drivers write `/Rotate` rather than re-imaging, so this is the norm in the
  one population OCR exists for.
- **`MinifyFilter::default()` moved to `Smooth`**, and it took a Type 3 bitmap
  stencil with it — caught by yesterday's Acrobat-measured test. The exclusion
  is scoped to exactly what was measured; page-content `/ImageMask` is
  deliberately NOT covered.
- **Render presets carry an evidence tier per value.** For PDF/X-4, one of six
  is a claim about the standard at all. `only_sourced_cells_may_claim_to_be_
  sourced` is the guard that stops a guess being relabelled.
## §0 — ★★ A DISPATCH IS A SET OF CLAIMS, AND YOURS WILL BE WRONG

Read this before dispatching any agent that files, records, or decides.

The 263rd filing dispatch carried three factual premises. **All three were
false**, and the librarian checked rather than filed them:

1. an outbound note was "at" a path pdfceGUI had already consumed and renamed;
2. a `FEATURES.md` box was asserted `[ ]` when the other project had wired the
   verb **six hours earlier**;
3. ★ **a feature was credited to the wrong commit** — `pdfce-cli ocr` was
   attributed to `de2d93c`; it is present at `1f79cc1` and `181d9bd` only made
   it *work*. Established by measuring four trees, not by reading a message.

None was careless. Each was a reasonable inference from what the engineer
remembered doing. **Memory of one's own session is exactly the kind of source
that feels like a fact.**

Two consequences worth carrying:

- **Write dispatches so a premise is checkable**, and expect the agent to
  check. A dispatch that says "X is at path P" invites verification; one that
  says "as we discussed" does not.
- **The prior session's hazard still holds** (a dispatched agent sees a
  *snapshot* and you keep typing). Both failure modes point the same way:
  **finish the code, then dispatch, then commit the filing last.**
## §0a — ★★★ THE FILING COMMIT MUST BE THE LAST COMMIT BEFORE THE TAG

Held this release, deliberately, after it cost a red CI run and a re-tag at
`v0.10.0`.

`check-commits-filed.py` counts commits that no filing names. `bb154ed`'s
tip-deferral excuses a commit that cannot cite its own hash — but **only
while it is the tip**. The instant a filing lands on top, the excuse
evaporates and the gate flips red without anything about that commit
changing.

> **The rule: dispatch the librarian LAST, and commit its filing LAST.** Any
> code commit made after the dispatch has, by construction, no filing that can
> name it. Nothing makes *file-then-append-code* safe except ordering.

The recovery, if it happens anyway (precedent: `v0.8.0`, `v0.10.0`): file the
orphan, re-tag at the filing commit, force-push the tag, **rebuild the
package** so its `BUILD-INFO.txt` names the tagged commit rather than a
superseded one, replace the asset with `--clobber`, and **re-run the smoke
test on the new artefact** — a re-cut release is a new artefact and does not
inherit the old one's test.

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

★ `check-string-gaps.sh` earned its keep again this session — see §2 item 4.

**3. ★★ RUN `cargo +nightly fuzz build` ON WINDOWS. CI CANNOT DO IT FOR
YOU, AND `cargo check --bins` IS NOT A SUBSTITUTE EITHER.** Two separate
traps, each defeating the obvious escape from the other:

- **CI's `fuzz targets build (nightly)` job runs on `ubuntu-latest`.** It was
  green throughout the window in which the harness was completely unbuildable
  here — `rten` declares `crate-type = ["lib", "cdylib"]`, and only Windows
  hands that cdylib libFuzzer's `/include:main`. A green CI does **not**
  absolve a red local. A `windows-latest` sibling is filed to Backlog and does
  not exist yet.
- **`check-ci-parity.py --list` offers `cd fuzz && cargo check --bins` as the
  local stand-in.** A/B'd 2026-08-25: **it passes in BOTH states**, because
  `cargo check` never links and the break was a link break. **A cheap proxy
  for a gate is a proxy for the part of the gate that is cheap.**

Ran this session: **exit 0, 25 targets, 4m20s.**

**4. Read `docs/compositor-plan.md`** before scoping anything in `97.x`.

**5. ★ BEFORE ANY PACKAGING OR RELEASE BUILD, CHECK FREE SPACE.**
`df -h . && du -sh target/debug target/release`. This session the volume was
**98 % full with `target/debug` at 92 GB** — one packaging run away from the
2026-08-23 failure. Delete **`target/debug` only** — never `cargo clean`,
which also drops the warm `target/release`. After deletion: 104 GB free.

**6. The suite corpus is at `$PDFCE_SUITE_DIR` (51 patches) and Acrobat
reference renders of all 51 at `$PDFCE_SUITE_REFS`.** ★ **The private map's
`manifest.json` names a `suite_dir` that does not exist** — the corpus is one
directory over. Neither variable is set in a fresh shell; read the manifest,
then verify the path with `ls` before believing it.

---

## §2 — ★★★★★ THE METHODOLOGY LESSONS FROM THIS RUN

**1. Measure the gap before scoping the Pass.** The queue item named a
feature that was already shipped. One probe fixture, built before any code,
turned a multi-day-shaped item into a disclosure change. See §A.

**2. A/B a fix's own oracle, and record the MEASURED value.** The prediction
(`1`) and the measurement (`0`) differed, and the measurement told a strictly
better story — that a *clean* font silences the dirty ones behind it. A
`> 0` assertion would have passed on neither code and proved nothing; the
exact `2` is what makes the test a claim about the de-duplication key.

**3. A fixture needs its CONTROL in the same file.** `tounicode_gate.pdf`
carries `/TA` **with** a `/ToUnicode` precisely so that every assertion about
the two fonts without one cannot also pass for a reader that extracts nothing
from any Type 3 font at all. And its glyphs are named `/ga1`, `/gb1`, `/gc1`
— **deliberately not standard names** — because pdfce's counted AGL extension
would otherwise resolve them by luck and paper the dead end over.

**4. ★★★ WRITE THE PATCH SCRIPT TO A FILE. DO NOT PIPE IT THROUGH A
HEREDOC. FOUR TIMES TODAY.**

This item said *"use `r'''…'''`, or write the file"* and offered them as
equivalent. **They are not, and the difference cost three further failures
after that sentence was written.** The escape layers are independent and each
one bites differently:

| layer | what it eats | how it shows up |
|---|---|---|
| **shell heredoc** | a backslash, before Python ever sees it | `re.error: incomplete escape \u` — **loud**, costs a minute |
| **non-raw Python string** | a trailing `\` + newline (a Python line continuation) | a wrapped Rust literal silently becomes one line with the next line's indentation baked in. **It compiles.** Only `check-string-gaps.sh` catches it |
| **raw Python string** | nothing — *and that is the trap* | `\uXXXX` is **not** processed, so `\u00a7` lands in the file literally. A document full of `\u2605` where the stars should be |

★ **The third row is the one that gets you after you have learned the second**,
and it did: the raw string was the correct fix for the backslash problem and
introduced the escape problem, in the same file, ten minutes later.

**The rule that survives all three:** write the script to a **file**, as a
**raw** string, containing **literal Unicode characters** (§, —, ★) typed
directly. Never `\u` escapes inside `r'''…'''`. Never a heredoc for anything
containing a backslash.

Today's tally: four wrapped Rust literals (caught by `check-string-gaps.sh`,
not by re-reading), one `re` pattern killed by a heredoc, and 43 escape
sequences written literally into this very file by a raw string. `D:\dev\rag\rust\`
has the amended finding, and it now records that **`r'''...'''` is not the fix
that works.**

**5. Two layers of test, because a correct model can sit behind a silent
front end.** The core tests prove the counter reaches `2`; they prove nothing
about whether it ever leaves the process. The three CLI tests assert *where*
each half goes — counters on stdout's summary line so a script can branch
without parsing prose, prose on stderr so it cannot contaminate
`find-text > hits.txt` — and one of them is a **control** asserting a clean
document gets **no** warning, because boilerplate is a warning nobody reads.

---

## §3 — THE QUEUE, in the order I would take it

**Nothing in this queue is blocked on anybody.**

1. **`Pass 127.2` — the redaction disclosure's MACHINE-READABLE half.**
   `Pass 127.1` shipped the prose on stderr and **did not ship scope item
   (b)**: `redact-mark`'s stdout summary line still carries no diagnostics
   field, while `find-text`'s does (`unreadable_codes=`,
   `type3_no_tounicode=`, `identity_no_tounicode=`).

   ★ **A script parsing stdout therefore gets IDENTICAL output for a clean
   redaction and one over text that could not be read** — and a script is the
   exact caller the Backlog entry used to justify ranking the work in the
   first place. The human half landed and the automatable half did not, which
   is the more easily-missed direction: the feature demonstrates correctly by
   hand.

   Found by the librarian reading `127.1`'s own scope list against the
   shipped code, not by a gate. Small: append to one `println!`, extend the
   CLI test.
2. **The trap-X cells — `PCS 1.0`'s `a`/`b`/`f`/`g`.** The `/Separation /All`
   hypothesis: §8.6.6.4 says painting with `/All` applies the tint to **all
   available colorants at once**, which a screen neutral cannot do; `122.5`
   gave those pages a colorant buffer, so it is now possible. **Confirm the
   content-stream question first** — that those marks really *are* the `/All`
   paint is still unestablished. **`127.3` is the next free ID** (`127.2` is
   taken by item 1 above).
3. **Mesh shadings in ink — the mesh half of `Pass 97.1k`.** Meshes are a
   population bridging through sRGB. `Patch` corners hold a resolved
   `Shade::Rgb`; carrying authored colorants alongside is the same shape
   `ColorRamp::at_cmyk` already took for analytic ramps in `122.6`, and would
   let a mesh honour overprint.
4. **Mesh cells `e`/`j` on `PCS 1.0`** — the shading colour path
   (`ColorRamp::at` resolves to sRGB when the ramp is *built*). Long-standing
   and structural; unrelated to `125.0`.
5. **Per-image overprint (`Pass 122.1`)** — the image sibling of `122.6`.
   Diagnosed: it is why `PCS 8.2`'s check mark is missing. pdfium fails it too.
6. **A `windows-latest` sibling for CI's fuzz job.** ★ Not "wire fuzzing into
   CI" — **it is already wired** and has been green throughout. It runs on
   `ubuntu-latest`, and the class that broke it was MSVC-only, so the job is
   structurally blind to the whole class.
7. **Audit the other seventeen `tools/check-*` against `R218`.** Filed as an
   unscoped Backlog note. ★ And now against §0 above too — the snapshot
   hazard is the same shape with an agent instead of a script.
8. **Build `R214`'s positional-reference gate** — a grep over a closed
   vocabulary in doc comments. **Measure its baseline first, repair, then
   wire — never wire it red.** `R216`'s companion vocabulary rides the same
   instrument; build **one** script with two vocabularies.
9. **`Pass 122.0`** — multithreading, the operator's own request. ★★ Read §2
   item 2 and `R215` before writing its acceptance table: *"byte-identical
   output at any core count"* is a differential claim over an **unbounded**
   parameter. Sample the switch points (1, 2, `n-1`, `n`, oversubscribed).
   Decision 080 adds a compile-time target gate, because `std::thread` and
   `rayon` both `cargo check` cleanly for `wasm32`.
10. **`Pass 119.1`** — `unshare_form`. Carried unstarted through twelve
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

- **Redaction-by-search does not disclose unreadable text.** Queue item 1.
- **Type 3 text cannot be EDITED.** Extraction, search and copy ship as of
  `Pass 127.0`, gated on `/ToUnicode` exactly as Acrobat is. Editing is not
  planned and **Acrobat has no in-place path for it either** — an evidenced
  "nothing to copy", not an oversight.
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
  recursion limit.
- **Mesh shadings bridge through sRGB** into the colorant buffer, so their
  overprint is not represented. Queue item 3.
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
  finds 0, so any future scan must inflate first.
- **A non-isolated group whose SECOND buffer cannot be allocated** still
  falls back to isolated semantics — counted, disclosed, and now the *only*
  way a non-isolated group reaches `cmyk_groups_approximated`.
- **No GUI code path reads** any of the text diagnostics, `forms_culled`,
  `subpixel_culled`, `annots_out_of_scope`, `page_content_suppressed`,
  `render_page_region`, the display list, or the three `mesh_*` counters, and
  **no GUI exposes `--fast-subpixel`.** GUI work is paused; recorded so the
  `[ ] gui` boxes in `FEATURES.md` are not mistaken for oversights.

---

## §5 — THE OPERATOR ITEMS

- **PUSHING AND RELEASING — DISCHARGED FOR `v0.12.0` AND SPENT.** The
  authorisation was *"release and commit"*, given 2026-08-26. **One
  authorisation covers one act** (`CLAUDE.md` rule 8). The next push and the
  next release each need their own go-ahead.

  ★ Asset policy unchanged: **never attach a render of a suite patch** — its
  artwork is licensed (`LEGAL.md` §5). Synthetic fixtures are MIT-clean and
  *are* attachable; `v0.12.0` ships `fixtures/synthetic/ocr/scan.pdf` as the
  OCR demo for exactly that reason.

  ★★ **Smoke-test the ZIP, not the build folder** — extract to a fresh path and
  run from the extraction. For `v0.12.0` that test did the real work: it
  confirmed OCR runs **with no `--model-dir`**, which is the thing that was
  broken in every release build before this one.

- **CUTTING A BACKUP — DISCHARGED 2026-08-26, and the habit is what matters.**
  `pdfce-20260826-0630-full.bundle`, `git bundle verify` reports a complete
  history, `main` at `81d1e30`, **zero commits behind** at cut time. The
  previous bundle was **35 behind**.

  ★★ **DO NOT PICK THE NEWEST BUNDLE BY FILENAME. SORT BY MTIME.** On
  2026-08-26 two bundles cut hours apart were hand-named with times that did
  not match when they were written, so **alphabetical order was the REVERSE of
  chronological order** — `ls | tail -1` returned the OLDER file and would have
  reported the tree 6 commits stale when it was 2. Found by the librarian
  cross-checking mtime against name, not by any gate.

  Today's two were renamed to carry **the head commit in the filename**
  (`pdfce-<date>-<HHMM>-<shorthash>-full.bundle`), which makes a bundle
  self-describing and the mistake unavailable. **Name new ones the same way.**
  Older files keep their historical names and still disagree; that is left
  alone deliberately rather than mass-renamed, and it is why the rule is "sort
  by mtime" rather than "trust the names now".

  **Re-measure before quoting**, and check both orderings agree:

  ```
  ls -1rt D:/Dev/pdfce-backups/*.bundle | tail -1     # newest, by TIME
  git bundle list-heads <newest> | grep refs/heads/main
  git rev-list --count <head>..main
  ```

  Every number in every handoff was true at a different `HEAD`.

- **OUTBOUND TO pdfceGUI — all consumed, nothing owed.** Three notes went out
  (the zero-result search disclosure, the X-4 preset vector, and the OCR
  DPI-curve warning). pdfceGUI has renamed all three to `done_*_CONSUMED.md`,
  **retracted its DPI curve rather than adjusting it**, rewritten its pinning
  test to solve the fitting equation instead of asserting a value, fixed its
  own `/Rotate` handling, and wired `search_and_mark_redactions_styled`.

  ★ A note is *filed* when you write it and *handed off* when the other side
  names it. All three are handed off; check the channel rather than assuming
  either way.

- **`git ls-remote --tags | tail` DOES NOT SHOW YOU THE NEWEST TAGS.** It
  sorts **lexicographically**, so `v0.10.0` sorts before `v0.5.0` and sits at
  the *head* of the list. The moment a minor number reaches two digits, a
  `tail` on a version list hides exactly the versions you are looking for. Use
  `gh release list`, or `git tag --sort=-v:refname`.
