# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**: no
*"this paragraph read X until…"*. What is true now, plus a pointer.
Corrections and their prior wording live in the **append-only** record —
`ROADMAP.md` and `SESSION_LOG.md` — where a claim is dated and no later edit
can falsify it.

---

## §0 — TWO THINGS OWED IMMEDIATELY

1. **A backup is six filings overdue.** The newest bundle is ~35 commits
   behind `HEAD`. Everything is on `origin`, so this is redundancy rather than
   exposure — but it is the operator's act and it has now been reported six
   times.
2. **`check-cited-commits-exist.py` degrades to silence at exactly the wrong
   moment**, and the direction matters — this corrects the version in the
   302nd filing, which predicted the opposite. Read from source: an
   unreachable-but-still-present commit is detected (`cat-file -t` says
   `commit`, so it reaches the staleness report). After a `git gc` prunes it,
   `cat-file -t` fails and the token falls through the *"not a hash: a colour,
   a length, an offset"* branch — **silently skipped.**

   So an *explained* orphan citation stays green either way (fine), and an
   **unexplained** one goes from **RED to GREEN the moment the damage becomes
   permanent.** The gate stops reporting precisely when the citation can no
   longer be repaired from anything but its subject line.

   ⇒ The fix is not a warning; it is that a 7+ hex token which is *not* an
   ancestor and *not* a known object cannot be told apart from a colour code,
   so the gate needs a named list of deliberately-cited orphans and should
   treat any *other* unresolvable hash-shaped token as suspicious rather than
   as prose.

   ★ **This paragraph demonstrated the problem while describing it.** Naming
   the two orphans here turned the gate **red** — correctly, because a bare
   orphan hash is indistinguishable from a citation somebody forgot to
   repair. The gate already has the right rule: an orphan is *explained*, and
   accepted, when the document also names the commit that replaced it. So:
   **`eab7da4` was replaced by `36e7b66`**, and **`fa16819` by `0d20861`**.
   That is not appeasing the gate — it is the sentence a future reader needs,
   and writing it is what makes the citation repairable after a `git gc`
   removes the objects.

---

## §A — RUN `bash tools/run-gates.sh`, NOT A HAND-TYPED GATE LOOP

**New this session, and it exists because the old habit put a red run on the
public repository.** It derives its command list from
`check-ci-parity.py --list` (which derives it from `.github/workflows/`), runs
the **filing gates last** by construction, names anything it skips, and does
not stop at the first failure.

```
bash tools/run-gates.sh          # 26 commands (1 pre-flight + 23 + 2 filing)
bash tools/run-gates.sh --list   # what it would run, in order, incl. skips
bash tools/run-gates.sh --full   # add --all-features testing
```

★★ **It now runs one PRE-FLIGHT check CI structurally cannot provide.**
`check-history-not-rewritten.py` asks whether `origin/main` is still an
ancestor of `HEAD` — i.e. whether published history has been rewritten. Every
other gate asks about the *tree*, which the server re-checks; by the time CI
runs, the push has happened. **A subagent amended an already-pushed commit
during this session — TWICE**, and nothing announced either. Both were
harmless (identical trees, metadata only). The first was found by accident;
**the second landed on the librarian's own filing commit 37 seconds after it
was pushed**, and its recovery was this gate's printed remedy executed 71
minutes before the gate existed. The knowledge was never missing. Only the
announcement. **If it ever fires,
the fix is `git reset --mixed origin/main`, never `--force`.**

★ **Do not go back to `for g in tools/check-*`.** A hand-typed sweep ran green
thirteen times this session while omitting **five** of CI's commands —
including **both filing gates** — and CI went red on one of them. A sweep that
omits a gate is byte-indistinguishable from a green one.

★★ **And the ordering rule the gates enforce, restated because it was broken
twice tonight:** commit the code → dispatch the librarian → commit the filing
→ **then** push. `check-commits-filed.py` defers exactly **one** commit at the
tip (a commit cannot cite its own hash) and demands every commit below it.
**One unfiled commit may sit at the tip. Never two.** Decision 090's *"always
push"* grants the push; it does not grant pushing twice before the librarian
has run.

---

## §B — WHAT TO PICK UP

**Check the channel with `ls -lt` and open every file newer than the last
check** — `D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\` and the
`iccce_FeatureRequests` sibling. It moved **three times** during this session,
twice underneath a check that had already run, and one of those files was a
live defect report against a Pass shipped four hours earlier. **A directory
listing is not a reading.**

As of this writing all five inbound asks are shipped and the open files are
pdfce's own replies plus `pdfceGUI`'s consumption notes — but that sentence
has a shelf life measured in minutes and has already expired once.

| # | Pass | what | where |
|---|---|---|---|
| 1 | **`143.0`** | a `DeviceGray` fill overprinting a spot backdrop knocks it out; Acrobat preserves it | *Backlog*, fully diagnosed |

**`Pass 143.0` is the pick-up item**, and it was the pick-up item before the
channel pre-empted it. It is **a spec reading, not a bug** — ISO 32000-1
§8.6.7 scopes `OPM 1` to a tint of 0.0 *"in a `DeviceCMYK` colour space"*, and
`DeviceGray` is not one, so pdfce's literal reading is defensible and
Acrobat's convert-to-K-then-apply reading is the other defensible one. **Ship
both, default to Acrobat's** (`R169`; Ken 2026-08-08 *"make spec ambiguity a
setting"*), because this is a print-conformance axis and the suite is authored
to press behaviour.

**Scoping it as a bug is the mistake to avoid.** Read the `Pass 143.0` Backlog
entry before starting: it carries the `overprint::classify` →
`SourceKind::OtherProcess` → `[ComponentRule::Source; 4]` chain, the sourcing
(`iso32000__s__8.6.7.md`, whose `OPM-2` row already records the neighbouring
CIE-based case), and the three acceptance criteria owed at scoping time
(the setting's name and values; whether it covers `DeviceGray` only or every
non-`DeviceCMYK` process space `classify` calls `OtherProcess`; whether the
CLI gets a flag).

**Expected result:** the print-conformance suite goes back to **5 FAIL or
better** of 51, undoing the `6 FAIL` that `Pass 140.1` correctly produced by
removing a false pass. **That is not `140.1` being reverted** — the trap X
stays correct either way; this fixes the surround it was hiding behind.

`Pass 142.0` remains **de-prioritised, not closed**, in *Backlog*.

### ★ Two things measured while scoping `143.0`, before any code was written

**1. The filed scoping is CORRECT — I checked and it survived.** The spec RAG
confirms §8.6.5.7's implicit-conversion clause covers **CIE-based** spaces
only, so `DeviceGray` is genuinely outside `OPM 1`'s literal scope and pdfce's
reading is defensible. Acrobat's grey→K-only-then-`OPM 1` reading is the other
defensible one. **Ship both, default to Acrobat's**, exactly as filed. The
spot backdrop's ink is *already in the four CMYK planes* by paint time, so
preserving it IS expressible with the four-component rules — no new plane
needed for `143.0`.

**2. A SEPARATE finding, and it is not what `143.0` is about — do not
conflate them.** Table 149's **entire spot-component row family is
implemented, tested, documented and UNREACHABLE**:

- `overprint::compatible_overprint` has **zero callers outside its own
  module** — the renderer only ever calls `cmyk_group_rules`, which returns
  **four** rules;
- `Component::Spot` is **only ever matched, never constructed** outside tests;
- `CmykBuffer` has exactly **four** colorant planes, and its own module
  comment says so: *"the next buffer (spot planes) is runtime-N"*.

So the rule that would preserve a **named** spot component under a process
paint (`(OtherProcess, Spot) → Backdrop`) is correct and can never fire.
That is `R151`'s shape at the scale of a table row family, and it belongs to
the **`Pass 97.x` CMYK+N compositor**, not to `143.0`. Worth its own Backlog
item; the librarian has it.

★ I nearly filed (2) as a *correction to `143.0`'s scoping*, which would have
been wrong — the spot ink being pre-converted into the four planes is exactly
why the filed fix works. Stated here as two findings because they are two.

**3. THE OPEN ACCEPTANCE CRITERION IS ANSWERED BY MEASUREMENT — scope the
setting to `DeviceGray` ONLY.** The entry asked *"whether the conversion
applies to `DeviceGray` only or to every non-`DeviceCMYK` process space
`classify` currently calls `OtherProcess`"*. Measured over the 53-file
conformance corpus (set `PDFCE_SUITE_DIR`; the private map names the
directory): of the sixteen patches combining a `Separation`/`DeviceN` with
`/OP true`, **not one carries a `DeviceRGB` fill** — every count is **0**.

⇒ There is **no measured Acrobat behaviour** for RGB-over-spot under
overprint, so widening the setting to cover it would be an **unmeasured
parity claim**. `DeviceGray` is the only non-CMYK process space the corpus
puts in this position, and it is the one Acrobat was measured on.

★★ **And the corpus supplies its own control**, which is the second
acceptance criterion and a better one than anything I would have invented:
**`PCS2_031`** is a grey **IMAGE** overprint, where §8.6.7's `OPM-3` says
`OPM 1` **shall not** apply. So the setting must move the fill patch —
**`PCS2_030`**, the one the entry measured at 84,120,34 against pdfce's
127,127,127 — and must **NOT** move the image one. `classify` already takes
`in_image_sample`, so the distinction is expressible; **a fix that moves both
is over-broad, and the corpus says so without anyone having to notice.**

---

## §C — WHAT SHIPPED, AND THE TWO NUMBERS WORTH CARRYING

`142.1` (`2e6235c`), `144.0` (`cfa2c44`), `145.0` (`0c48bbf`), `146.0`
(`9f6e732`), `147.0` (`8aa9cea`), `148.0` (`f1a88e6`), filings `0c254f8` /
`54a3e01` / `0c24cad` / the 300th, and `1e62715` (the gate runner). **Five**
channel items — `pdfceGUI` consumed three the same night and filed a defect
against one (`147.0`), and the librarian's sweep then found that defect still
live one function over (`148.0`).

Two measurements this session that are now **published guarantees**, not
observations, and both live in `ARCHITECTURE.md` §4.2 / `docs/core-api/`:

- **The `operator_span`-slice invariant HOLDS.** 29,246 operator groups over
  4,289 files, **0** non-contiguous, **0** unmatchable. A consuming project had
  already shipped against it and could not check it. A `text_extract::layout`
  refactor that breaks it now turns a test red.
- **13 % of runs (2,420 of 18,559) carry glyphs from more than one show
  operator.** That confirms a claim `ROADMAP.md` had carried as *unverified*.
  A `TextRun` is **not** a show operator.

---

## §D — VERIFIED FROM A SHELL AT WRITE TIME — re-run, do not copy forward

| fact | value | command |
|---|---|---|
| `HEAD` | `f1a88e6` **plus the 300th filing and this file's own commit** | `git rev-parse --short HEAD` |
| tests | **4,493 passing, 0 failing** | `cargo test --workspace` |
| gates | **26 commands green** | `bash tools/run-gates.sh` |
| fuzz | all targets build; `text_extract` 44,918 runs, `load_document` 100,363 runs, **0 artifacts** | `cargo +nightly fuzz build` |
| wasm32 / GUI-core | both clean | in the sweep |
| **CI** | `0c48bbf` and `0c254f8` were **RED** (filing order, both repaired); `54a3e01` is **green**, read from GitHub. Everything after: **read it from GitHub too.** | `gh run list --branch main --limit 3` |
| disk | 182 GB free of 954 GB | `df -h /d` |

★ **Fuzzing needs the MSVC ASan DLL on `PATH`**, else `0xc0000135`:
`find "/c/Program Files (x86)/Microsoft Visual Studio" -iname "clang_rt.asan_dynamic-x86_64.dll"`.

★★ **Pushing needs no asking** (decision 090, *"always push"*). **Not**
covered: tags, releases, `--force`, or any branch but `main`. **Scrub
`tools/check-suite-name-absent.py` green before every push** — the repo is
public and that gate scans untracked files too, which is why scratch renders
belong in `D:\Dev\temp\pdfce\`.

---

## §E — HABITS THAT PAID, AND ONE THAT COST

### Run the binary. Unit tests cannot reach the shell-to-engine wiring

Three defects in this session's own new code were invisible to a green
`cargo test --workspace` and took one terminal command each:

- `edit-text --pin-span` was **declared, parsed, validated — and never
  attached to the request.** The refusal read *"empty find text"*, the exact
  message the feature existed to remove. Every core test passed; they
  construct `EditRequest` themselves.
- A shipped refusal carried **fourteen baked-in spaces**, on which
  `check-string-gaps.sh` reported PASS.
- The font pre-flight, given a pinned empty `find`, surveyed **zero
  characters** and reported **every face on the page as accepted** — found by
  `pdfceGUI` four hours after the Pass that made callers write it that way.
  The failure presents as a **richer list**, not an error.

⇒ When a Pass adds anything an operator types or reads, drive **every** new
path — success, each refusal, a malformed input — and read the output as text.

### Reproduce a report before believing it, and measure its mechanism separately

All three of `pdfceGUI`'s `gate_synthesis` commands were re-run before the
defect was accepted. That is what made it *confirmed* rather than triage — and
the same discipline then **falsified their stated mechanism** for a different
report (0 of 256 fixtures) while leaving the headline true. **A bug report's
SYMPTOM is evidence; its MECHANISM is a hypothesis.** They arrive in the same
file in the same confident voice.

### Verify each instance, not the class — TWICE, on one defect

**One `find`-resolution defect took three Passes to close, each fixing one
route:**

| Pass | fixed | left broken |
|---|---|---|
| `145.0` | `plan_format`, `plan_edit` | both preview queries |
| `147.0` | `preview_font_resources` | `preview_style_resolution` |
| `148.0` | `preview_style_resolution` | — |

`147.0` was reported by `pdfceGUI`. `148.0` was found by the **librarian's**
rule-11 sweep asking *"what else decides from the caller's string?"* — not by
me, four hours earlier, editing the same file. And within `147.0` I fixed the
pinned half and **assumed** the unpinned half already errored; it did not
(`s.text.contains("")` is true of every run), and a test I wrote expecting it
to pass is what caught it.

⇒ **Enumerating routes from the function you are fixing enumerates the
instance.** `crates/pdfce-core/tests/route_enumeration.rs` now asserts, as a
**source scan**, that every function calling `find_anchor` also calls
`effective_find` — the only assertion in that file that can fail on a fifth
route added next month. Its own first cut (a 30-line window) reported all four
known-good sites as violations, which is the right way for a bad heuristic to
fail.

### Sabotage every suite before trusting it

Used seven times, all caught: dropping the acceptance filter (3 red), dropping
the cross-family branch (2 red, and the two that stayed green were exactly the
right two), excluding a glyph from the coverage computation (1 red), returning
a default for an absent border (2 red), reverting the pre-flight to the
caller's string (2 red, with the unpinned test correctly staying green), and
removing the resolution from `preview_style_resolution` (3 of 4 red, including
the source scan). `cp x /tmp/x.bak` first; `git checkout --` is not an undo.

### A doc comment's claim about its own callers is `R223` now

Two more instances this session, both in the function pair `Pass 144.0` fixed,
neither reported by anything. One was **true when written and falsified by a
later caller**; the other was **wrong on the day it was written**. `cargo doc`
cannot check either. Old wording kept **struck**, not replaced — the failure
mode is worth more than the correction.

---

## §F — OPEN, UNCHANGED

- **`(bl)`** — may a **CC-BY-SA-4.0** OCR model ship inside pdfce's **MIT**
  portable folder? Ken's call; default if unanswered is **ship neither model
  set**.
- **`R13` vs "download addin capability"** — downloading is permitted;
  **executing** what was downloaded is not, and that ruling is owed from the
  operator. **No add-in Pass can be scoped until it lands.**
- **`(p)`** — whether to narrow the XFA item to read/fill only, or retire it.
- **An observation, not a decision:** `tools/check-image-colorspace-truth.py`
  writes a `_truth/` subdirectory beside the images it scores, and pointing it
  at `fixtures/synthetic/images` broke a `pdfce-core` test. Whether that gate
  should write into a fixture directory at all is an open engineer question.
- **Backups are behind.** The librarian measured the newest bundle at 22
  commits back before this session's work; nothing from tonight is in one,
  though all of it is on `origin`.
