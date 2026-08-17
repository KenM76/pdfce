# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. Overwrite it once acted on.

---

## ★★ THE HEADLINE: THE GHENT QUEUE WAS ORDERED ON AN ASSUMPTION, AND THE MEASUREMENT REVERSED IT

The previous handoff's queue said: shading patterns → tiling patterns →
clause-11 transparency → overprint. **That ordering was never measured
against the operator's file.** I measured it, and:

**Not one of the Ghent X-4 file's six pages uses a `scn` pattern at all.**
`pattern_spaces=0` on every page. The shading-pattern Pass shipped this
session (`5df75dd`) does not move that file, and **tiling patterns will not
either.**

What the file actually uses is **transparency**, and pdfce could not see it
because `apply_ext_gstate` read seven keys and silently dropped `/BM` and
`/SMask`. With counters added (`ae46e82`):

| page | 1 | 2 | 3 | 4 | 5 | 6 | **total** |
|---|---|---|---|---|---|---|---|
| `/BM` ignored | 1 | **76** | 1 | 1 | 31 | 3 | **113** |
| `/SMask` ignored | 1 | **31** | 1 | 1 | 1 | 1 | **36** |

**Page 2 is the worst page in the file and had looked CLEAN** — no
unsupported images, no unpainted patterns, no refused shadings. It was
compositing 76 blend modes by the wrong rule the whole time.

### ⇢ SO: DO CLAUSE-11 TRANSPARENCY NEXT. It is the largest remaining visual gap on the file the operator is watching, by a wide margin.

The revised queue, measured rather than assumed:

1. **Clause-11 transparency** — `/BM` blend modes (§11.3.5) and `/SMask`
   soft masks (§11.6.5) in an ExtGState. **113 + 36 occurrences.** The
   disclosure half is shipped; the implementation half is not started.
2. **Mesh shadings, types 4–7** — 2 on Ghent page 1, currently
   `shadings_mesh=2`, painted 10 of 12. **★ NOT IN THE SPEC CORPUS**, and
   the index carries an explicit *"do not answer from recall"* marker.
   **Dispatch `pdfce-spec-librarian` for §8.7.4.5.5–.8 + Tables 82–84
   before writing any of it.**
3. **`/Separation /All`** — 62 conversions on page 1, 52 on page 4,
   rendered as a neutral of luminance 1−tint. That is pdfce's own
   documented choice, not a bug; whether it should change is a question,
   not a defect.
4. **Tiling patterns** (`PatternType 1`) — real work, but **now known not
   to be Ghent-driven.** Demote it accordingly.
5. **Overprint** LAST. Architectural (compositing into a CMYK buffer) and
   genuinely gated on `iccce` supplying a credible CMYK→display conversion.

Also present on Ghent and already disclosed, not defects: 6 CMYK JPEGs with
unverifiable polarity (decision 006 / R30), 1 Widget annotation with no
`/AP` (R43 refuses to synthesise), and 40/81/15 deferred ops that are all
`BDC`/`BMC`/`MP` marked content.

**The general lesson, which cost nothing to learn and would have cost a
whole Pass to learn later: a queue item's PRIORITY is a measurement, not a
reading. `pattern_spaces=0` took thirty seconds to check.**

---

## STATE AT HANDOFF — 2026-08-17

**`HEAD` = `ae46e82`. Working tree CLEAN. 3,770 tests, 0 failures.**
`fmt`/`clippy` clean; all scripted gates clean (`check-fmt-excluded`,
`check-ui-strings`, `check-shipped-assets`, `check-ledger-numbers`);
`cargo tree` shows no GUI dependency in `pdfce-core` or `pdfce-render`.
**Nothing half-built.** Everything is filed — `check-commits-filed.py` will
name `ae46e82` only, which is this session's last commit and is owed a
filing (see *Owed* below).

**Local is 16 commits ahead of `origin/main` (`718d1e9`). Not pushed, not
tagged.** Stated as a fact — pushing is the operator's act.

**Backup:** `D:\Dev\pdfce-backups\pdfce-20260817-final.bundle`,
`git bundle verify` okay, `refs/heads/main` = `ae46e82` = `HEAD`, and
`refs/tags/v0.5.3` (`b2d0595`) is inside it. **Re-measure anyway** — this
ledger has carried a wrong backup figure twice.

**Live ceilings**, from `check-ledger-numbers.py` this session — but see the
trap below: Pass families to **89** → next free **90** · standing rules
**R195** → next free **R196** · decisions **065** → next **066** · filings
**155** → next **156**.

> **★ THE `R193` TRAP, which bit the librarian this session and it caught
> itself.** `check-ledger-numbers.py`'s next-free rule output is
> **known-wrong**: a `### PROPOSAL … claiming R19x` heading is invisible to
> its pattern, so declined-but-intact proposals do not register.
> **R193 and R194 are both claimed.** Read the *Standing rules* section's
> own closing ledger line, not the checker, before minting a rule.

### What shipped this session

| commit | what |
|---|---|
| `1345663` | the closed-form colour oracle — **pdfium, not pdfce, is wrong on Lab/CalGray/CalRGB** |
| `a705d14` | `Pass 89.0` — the redaction overlay ladder (Table 192) |
| `a7210a4` | `Pass 89.1` — `RedactAppearance`, reaching the find-and-mark path |
| `5df75dd` | `Pass 85.2` slice 1 — `PatternType 2` shading-pattern fills |
| `ae46e82` | clause-11 transparency **disclosure** (the measurement above) |

Plus filings 152–155. Both `pdfceGUI` channel requests are closed with
replies in `archive/`.

### Owed

- **`ae46e82` is not filed.** Dispatch `pdfce-librarian` with its full
  commit message. It needs a Shipped entry, a `FEATURES.md` note that
  clause-11 transparency is **disclosed but not implemented** (do not let
  that read as support), and the queue re-ordering above recorded in the
  Ghent gap inventory.
- **Ask the librarian to rule** whether "a queue item's priority was never
  measured" is a rule-shaped finding. I think it may be — it is adjacent to
  `R180` but distinct: R180 is about a claim going stale, this is about a
  claim never having been checked.

---

## ⇢ ★ FIRST, EVERY SESSION: CHECK BOTH REQUEST CHANNELS

**`D:\Dev\FeatureRequests\pdfce_FeatureRequests`** — the `pdfceGUI` channel.
**`D:\Dev\FeatureRequests\iccce_FeatureRequests`** — the `iccce` channel,
which is **two-way**; expect `request_*.md` written by them.

`open/` in the GUI channel currently holds **five**, all print/annotation,
none touched this session:

- `request_devicesettings_pick_tray_is_never_read.md`
- `request_no_paper_size_selection_in_the_print_path.md`
- `request_no_verb_modifies_an_existing_annotation.md`
- `request_no_verb_sets_a_pages_media_box.md`
- `request_orientation_auto_is_per_job_not_per_page.md`

The two redaction requests were **closed this session** — archived with
replies as `2026-08-17-redaction-overlay-text-*` and
`2026-08-17-redaction-fill-search-path-*`.

**A request is a FINDING, not a favour** (decision 058). Both of this
session's were: one was operator-authored content silently dropped, the
other an unreachable API. **Both were reported by a project that read our
source, found the surface missing, and STOPPED rather than working around
it.** That is expensive for them and cheap for us; honour it by fixing
rather than deflecting.

**The `iccce` channel binds pdfce to THEIR evidence standard inside that
folder**: a colour claim carries its oracle and its number, and pdfce's
`cmyk_table.rs` is fitted to pdfium, which makes it a **cross-check, not
ground truth** — say so when quoting it.

---

## ⇢ ★ THE STANDING CONSTRAINT — GUI WORK IS PAUSED

Operator, 2026-08-13: *"continue the planned work except for gui related,
don't do any more work on the gui until I say so."* Reason given later:
the shell *"was unusable and I realised it needed a separate project plan
rather than the current method which just seems to be low priority and a
patchwork."* A new GUI is being built at `d:\dev\pdfceGUI` and **may replace
`crates/pdfce-gui` wholesale.**

- Paused: `crates/pdfce-gui/`, `tools/gui-drive.ps1`, `tools/gui-shot.ps1`.
- Not paused: core, render, CLI, print, docs, RAGs, tests, fuzz, tooling.
- A Pass whose GUI half is deferred ships `core [x] · cli [x] · gui [ ]`,
  recorded as an **operator instruction, not an engineering shortfall**.
- **The objection is to the METHOD, not the priority** — "a well-built new
  panel" does not answer it. **Do not invest in the current shell.**
- **If he asks for GUI work, he has lifted it.** Do not quote the pause back
  at him.

---

## ★ THE METHOD THAT KEEPS PAYING — four saves this session

1. **The sabotage check found holes nothing else could, three times.**
   - A symmetric `/Range [-100 100 -100 100]` made an a/b transposition a
     **no-op**, so the Lab oracle stayed GREEN under sabotage. Fixed by
     making the range asymmetric.
   - Two shading-pattern tests looked like near-duplicates and **only one
     was load-bearing**: `a_later_solid_colour_replaces_the_pattern` passes
     with the fix removed. Only the `/Separation /None` sibling sees it.
   - Every pattern fixture puts a `cm` between `scn` and the fill, because
     **without one, base-CTM and current-CTM anchoring are arithmetically
     identical** and the suite would pass while testing nothing.

   These are three instances of ONE shape: **a fixture whose parameters are
   degenerate cannot distinguish implementations that differ only in those
   parameters.** Filed in `D:\dev\rag\rust\`.

2. **A green suite is not evidence until you have seen it go red.** Every
   Pass this session was sabotaged first.

3. **Reading the actual output caught four false claims no gate can see** —
   `R180`'s third AND fourth instances, both this session. `--help` text,
   stderr notes, doc comments and even a TEST NAME went false the moment the
   feature shipped. **`check-ui-strings.sh` reads `pdfce-gui`'s strings
   module and structurally cannot see `pdfce-cli`'s clap help.** R180 is now
   firing about once per feature-completing Pass.

4. **Looking at the render, not the counter.** `shadings_painted=1` says
   nothing about whether the gradient is in the right place. The anchoring
   was confirmed by rendering, seeing the gradient span the full page under
   a 0.5× `cm`, and cross-checking against pdfium (mean 1.31, max 2).

---

## THE COLOUR ORACLE — a tool that outranks the parity harness for three spaces

`tools/check-image-colorspace-truth.py` computes `/Lab`, `/CalGray` and
`/CalRGB` in closed form (ISO 32000-1 §8.6.5.2–.4 + the IEC 61966-2-1 sRGB
encode), sharing no code with either renderer. Use it, **not** the parity
harness, when the question is whether those conversions are correct.

    python tools/gen-image-colorspace-fixtures.py D:\Dev\temp\img-fixtures
    python tools/check-image-colorspace-truth.py D:\Dev\temp\img-fixtures

Measured 2026-08-17, 3,600 interior texels per space:

| space | pdfce mean / max | pdfium mean / max |
|---|---|---|
| lab | 0.019 / **1** | 40.854 / **152** |
| calgray | 0.000 / **0** | 2.000 / 9 |
| calrgb | 0.030 / **1** | 3.012 / 9 |

**pdfce is exact; pdfium is not.** The previous handoff had assumed the
opposite. `img_uncalibrated` and `img_colorant_none` are therefore
classified as **reference**-divergences in `render_parity.py`, not as pdfce
gaps — and the cost is stated in that file: it removes those pages from the
bug-candidate pool, which is exactly why the oracle exists as a separate,
non-comparative control.

**`pdfium` paints a `/Separation /None` image SOLID BLACK** in violation of
§8.6.6.4. pdfce paints nothing and is right.

---

## Tooling — corrections that cost time

- **★ A quoted heredoc (`<<'PYEOF'`) through the Bash tool mangles
  backslashes.** It bit me three times this session: a Rust string
  continuation `\` got eaten, and a `\\\n` became a literal `\n` that broke
  the CLI's one-line stdout contract. **For any multi-line patch, Write a
  script file and run it, or use the Edit tool.** Do not fight the heredoc.
- **★ `git checkout <file>` to undo a SABOTAGE reverts the whole file to
  `HEAD`**, including the work you were sabotaging. Copy the file aside
  first (`cp x D:/Dev/temp/x_backup`) and restore from that copy. I lost all
  of `color.rs`'s changes this way and had to re-apply them.
- **A `str.replace()` in a patch script with no `assert` is a silent
  no-op.** Assert every anchor.
- **Inserting a method by anchoring on `fn foo(`** lands it between `foo`'s
  DOC COMMENT and `foo` — the docs then document your new function. clippy
  catches it as *"empty line after doc comment"*; the real damage is silent.
- **A malformed fixture does not fail safe** — it fails somewhere else,
  convincingly. A `/Separation` array wrapped in `<< >>` left the space
  unresolved with `paints = true`, so the page filled with default BLACK and
  the test looked like a genuine failure of the code under test.
- **A hand-counted `/Length` is a `StreamExtentMismatch`** and the test then
  measures pdfce's refusal of your fixture. Compute it.
- **A stream inlined in a dictionary or array is invalid** (§7.3.8.1) and
  pdfce correctly refuses it. This bit me twice in one session — once in the
  image fixtures, once in a tiling-pattern test.
- `cargo fuzz` needs the MSVC ASan DLL on PATH; set it **literally**.
- **`git show <sha> -- <path> | grep` searches the commit MESSAGE too.** Use
  `git diff A^ A -- <path>`.
- **`gh run list --commit <SHORT-SHA>` returns an EMPTY LIST, not an error.**
  Pass a full 40-char SHA.
- **Resolve every short hash yourself** — librarians have no shell.

`tools/splice.py` · `tools/check-fmt-excluded.py` (run **beside**
`cargo fmt --all --check`) · `tools/check-shipped-assets.py` ·
`tools/verify-release.py <tag>` **before** tagging ·
`tools/check-commits-filed.py` — **file the commit; do NOT extend the
baseline** · `tools/check-ledger-numbers.py` ·
`tools/gen-image-colorspace-fixtures.py` + `tools/check-image-colorspace-truth.py` ·
`tools/render-parity/render_parity.py` · `tools/package-portable.py`.

---

## Release state

**`v0.5.3` is the current tag and is clean.** Sixteen commits sit past it,
unpushed and untagged. Nothing this session requires a release.

Keep the order: **FILE → LET CI GO GREEN → TAG**, run
`tools/verify-release.py` *before* tagging, and **bump the version before
the tag** (`--version` prints `CARGO_PKG_VERSION`, so tagging a version the
binary does not report ships a false claim exactly where a user checks it).

---

## Still open, carried forward

- **`Pass 71.0` OCR** — engine bound (`ocrs`, default-ON feature), sandwich
  writer shipped, `(bl)` answered YES. Still owed: the ~12 MB of `.rten`
  weights (**permanent in a PUBLIC repo's history — worth one sentence to
  the operator before the commit**), `pdfce-cli ocr`, and the second engine
  (PaddleOCR, Apache-2.0, 50+ languages, no WASM).
- **`Pass 67.0` phase C** — re-subset fonts. Lowest-risk remaining phase,
  needs nothing from the operator.
- **Backlog, from this session**: overlay-caption legibility — the authored
  `/DA` hard-codes BLACK text, so a caption on a dark `/IC` fill is
  illegible. Found by looking at a render. `pdfceGUI` told by name.
- **A `/Rotate` fixture gap** — no file in `fixtures/synthetic/` carries a
  `/Rotate` key, so four rotation branches have zero file-level coverage.
- **A display-list cache in `pdfce-render`** — measured: ~99 % of a render's
  cost is interpretation, not fill. Highest-value optimisation this crate
  has (decision 060).
- **Two escalations still awaiting the operator**, carried with no
  supporting detail: the broken no-git convention (`iccce`), and agents'
  in-progress files swept into a public repo. **Get the actual statement;
  do not resolve them.**
