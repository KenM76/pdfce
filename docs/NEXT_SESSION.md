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

**`v0.11.0` is released and verified. The working tree is clean, `origin/main`
matches, and nothing is blocked on anybody.** Both FeatureRequests channels
were checked; nothing new inbound. One note went **out** to pdfceGUI — see §5.

### What this run built (2026-08-25, evening — the same day as v0.10.0)

One Pass, and a defect it flushed out that had nothing to do with it.

| commit | what an operator would notice |
|---|---|
| `2104d38` | **`Pass 127.0` — Type 3 text SEARCHES and COPIES**, wherever the file carries a `/ToUnicode` CMap. And where it does not, pdfce **says so, per font, by name** |
| `35fce5f` | The three CLI tests that prove the disclosure reaches the operator, not just the counter |
| `768e934` | `v0.11.0` |

### ★★ THE SHAPE OF THIS PASS IS THE LESSON

The queue item read *"Type 3 text EXTRACTION and search"*. **The extraction
half was already shipped and unnamed.** A probe fixture built **before any
code was written** extracted `HI!` at `via_tounicode=3`, `sourced_pct=100.0`,
`failed=0` — because `ExtractFont::resolve` has routed `Type3` through the
simple-font path since Pass 4, and §9.10.2 rung 1 is font-subtype-agnostic.

**Checking first turned a feature into a sentence.** What actually shipped is
a *disclosure* Pass. Take the measurement before the scope.

### What the disclosure is, and why it is not cosmetic

A Type 3 glyph is a content stream named by an **arbitrary `/CharProcs`
key** (§9.6.5). `/g13` means nothing outside its own document, so §9.10.2
method 2's precondition is false **by construction** and `/ToUnicode` is the
font's only route. Without it the text **renders perfectly** and cannot be
searched, copied or extracted.

So `matches=0` has two causes and one appearance: *the needle is absent*, and
*this document's text was never recoverable as Unicode*. Nothing in
`TextMatch` can tell them apart, because the second case produces no
`TextMatch` to carry the news.

- `EditSession::search_text(needle, &opts) -> TextSearch { matches,
  diagnostics }` — new public verb, 140 now. Identical scan, identical hits;
  `find_text_with` delegates to it. It simply does not throw half of it away.
- `TextDiagnostics::type3_fonts_without_to_unicode`, the simple-font twin of
  `identity_fonts_without_to_unicode`, raised from the **font dictionary
  alone** — not from whether a code happened to fail, or a document whose
  Type 3 text is on page 40 would report nothing for the first 39.
- `extract-text` prints `type3_no_tounicode=`; `find-text` prints
  `unreadable_codes=`, `type3_no_tounicode=`, `identity_no_tounicode=` and
  prose on stderr.

★ **Acrobat has the identical limit and gives up silently.** Matching the
limit is parity; saying so is not. The Acrobat RAG
(`type3fonts__extraction_editing_and_tagging.md`) recommended this posture by
name.

### ★★★ THE SECOND DEFECT — the one worth carrying forward

The per-font diagnostic de-duplicated on `/BaseFont`. **ISO 32000-1 Table 112
has NO `/BaseFont` ENTRY** — a conformant Type 3 font has no name — so every
unnamed font on a page shared one slot.

**A/B'd rather than reasoned about, and the measurement was worse than the
prediction.** The expectation written down first was that N unnamed dead ends
would report `1`. Reverted to the old key, the fixture reported **`0`**:
`/TA` is unnamed too, is resolved **first**, has a `/ToUnicode` and therefore
no note to emit — it claimed the empty key and both fonts behind it were
skipped before their notes were ever read. **One clean unnamed font silenced
every unnamed font behind it.**

The prediction was recorded, then corrected in the doc comment, the test
rationale *and* the fixture's `PROVENANCE.md`. Write the measured value, not
the one you expected.

★ **This was never Type-3-specific.** It suppressed `UnknownSubtype`,
`BuiltinEncodingUnreadable` and every other per-font note for any font with a
missing or malformed `/BaseFont`. It surfaced only because Type 3 makes
namelessness the **conformant** case.

---

## §0 — ★★ A DISPATCHED LIBRARIAN SEES A SNAPSHOT, AND YOU KEEP TYPING

Read this before dispatching a filing agent and then continuing to work.

The librarian reported, from its own shell and correctly:

> *"`crates/pdfce-cli/tests/find_text.rs` is dirty and holds this Pass's own
> CLI tests… They are not in `2104d38`. So the commit ships the `find-text`
> disclosure **without the tests that prove it**."*

**True when it looked. False by the time it said so.** The tests were written
after the dispatch and committed as `35fce5f` while it was still working.
Nobody was wrong: it read the tree it was given.

This is `R218`'s shape one scale up — *a check whose input is "what exists
now" cannot see what you are about to do* — and it is the reason underneath
§0a below. It cost one amendment dispatch. It could just as easily have cost
a filing that names two of three commits, which is what turned CI red at
`v0.10.0`.

**The mitigation is ordering, not vigilance:** finish the code, *then*
dispatch, *then* commit the filing last.

---

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

**4. ★ NEVER PATCH RUST THROUGH A NON-RAW PYTHON STRING.** A trailing
backslash before a newline is a **Python** line continuation: it eats the
backslash *and* the newline, silently turning a wrapped Rust string literal
into one line with the next line's indentation baked in. **It compiles.**
Four literals shipped that way this session and `check-string-gaps.sh` caught
all four — a re-read did not. Use `r'''…'''`, or write the file and use
`Edit`. This is the second occurrence of the wrapped-literal class in this
project; `D:\dev\rag\rust\` has the file, amended.

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

1. **`Pass 127.1` — redaction-by-search discloses too.** The sibling defect,
   and **more dangerous than the search case**: `mark_redactions_by_search`
   over unsearchable text marks **nothing** and says **nothing**, while the
   operator's mental model is *"I redacted everything that matched."*
   `scan_text_matches` already returns the diagnostics — the drop is a named,
   commented line in `edit.rs` — so this is a return-type change on that verb
   plus a CLI line. Small, and it is first for the same reason `127.0` was.
2. **The trap-X cells — `PCS 1.0`'s `a`/`b`/`f`/`g`.** The `/Separation /All`
   hypothesis: §8.6.6.4 says painting with `/All` applies the tint to **all
   available colorants at once**, which a screen neutral cannot do; `122.5`
   gave those pages a colorant buffer, so it is now possible. **Confirm the
   content-stream question first** — that those marks really *are* the `/All`
   paint is still unestablished. `127.2` is the next free ID.
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

- **PUSHING AND RELEASING — DISCHARGED ONCE, FOR `v0.11.0`.** The
  authorisation was *"once complete release to git"*, given with this
  session's task. **One authorisation covers one act** (`CLAUDE.md` rule 8).
  The next push and the next release each need their own go-ahead. Three
  releases in one day does not make a standing one.

  ★ **Asset policy, unchanged and worth re-reading before the temptation
  arrives:** **never attach a render of a suite patch to a release.** Its
  artwork is licensed and must not be redistributed (`docs/LEGAL.md` §5, plus
  the 2026-08-25 name ruling). Synthetic fixtures from
  `fixtures/synthetic/` are MIT-clean and *are* attachable.

  ★★ **Smoke-test the ZIP, not the build folder.** Extract to a fresh path
  and run `pdfce-cli.exe` from the extraction; the artefact people download
  is what needs verifying, not what a build produced.

- **CUTTING A BACKUP — STILL NOT DISCHARGED, and now three releases stale.**
  A GitHub release is an offsite copy of one build; it is **not a `git
  bundle`** and contains no history. Measured this session: the newest bundle
  `D:\Dev\pdfce-backups\pdfce-20260825-0218-full.bundle` holds `main` at
  `81e5aab`. **Re-measure before quoting** — `ls D:/Dev/pdfce-backups/`,
  `git bundle list-heads <newest>`, `git rev-list --count <head>..main`. Do
  not carry a number forward from any handoff; each was true at a different
  `HEAD`.

- **OUTBOUND TO pdfceGUI, sent this session, needs no reply from Ken:**
  `D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\2026-08-25-a-zero-result-search-is-not-proof-the-word-is-absent.md`
  — tells that project to swap its Find bar from `find_text_with` to
  `search_text`, points at `docs/core-api/` §8.5 as the real documentation
  (not the note), and warns explicitly that `mark_redactions_by_search` does
  **not** disclose yet. Per the standing discipline: a cross-project
  deliverable recorded only in the producing tree has been *filed*, not
  *handed off*, so `ROADMAP.md`'s filing names that file too.
