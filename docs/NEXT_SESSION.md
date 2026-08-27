# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**: no
*"this paragraph read X until…"*. What is true now, plus a pointer.
Corrections and their prior wording live in the **append-only** record —
`ROADMAP.md` and `SESSION_LOG.md` — where a claim is dated and no later edit
can falsify it.

---

## §A — COLD START

**The previous session took `Pass 140.0` — the one item the last handoff said
was blocked on nothing but effort — and it grew into three Passes and opened a
fourth.** A five-colorant `DeviceN` photograph was rendering visibly
desaturated; it now matches Acrobat closely.

- **`Pass 140.0` + `Pass 140.1`** (`70c5919`) — a `Separation`/`DeviceN`
  **image**, and then a `Separation`/`DeviceN` **path fill**, keep their ink
  on a subtractive page instead of round-tripping through sRGB.
- **`Pass 140.2`** (`25d73d7`) — an image's own colour-conversion diagnostics
  reached **nothing at all**, so a broken `/tintTransform` painted a neutral
  stand-in in silence. A **rule 4** violation, found because a claim in
  `70c5919`'s own commit message was wrong. Read `§D`.
- **`Pass 143.0`** filed to *Backlog*, fully diagnosed — the `DeviceGray`
  overprint spec reading. Read `§E`; it is the reason the suite reads 6.
- **`R221`** minted (probe, not structural predicate).

### ★ Verified from a shell at write time — re-run, do not copy forward

| fact | value | command |
|---|---|---|
| `HEAD` | `25d73d7` **plus this file's own commit, and the 295th filing** | `git rev-parse --short HEAD` |
| `origin/main` | **level at `25d73d7`** — everything through `140.2` is pushed | `git rev-list --count origin/main..main` |
| tests | **4,419 passing, 0 failing** | `cargo test --workspace` |
| gates | 18 run bare, all green; `check-image-colorspace-truth.py` needs a fixture-dir argument and is green with one | `ls tools/check-*` |
| `fmt` / `clippy` | clean | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` |
| fuzz | all targets build; `image_import` 53,595 runs and `load_document` 165,969 runs, no artifacts | `cargo +nightly fuzz build` |
| wasm32 | `pdfce-core` + `pdfce-render` compile | `cargo check -p pdfce-core -p pdfce-render --target wasm32-unknown-unknown` |
| GUI-core invariant | no GUI dep in either engine crate | `cargo tree -p pdfce-core` |
| print-conformance suite | **6 FAIL / 51** — up from 5, and `§E` explains why that is correct | see `§E` |
| **CI at `HEAD`** | **READ IT FROM GITHUB.** `25d73d7`'s run was still in progress at write time and is deliberately not recorded here. | `gh run list --branch main --limit 3` |

★★★ **PUSHING NEEDS NO ASKING — decision `090`.** *"always push."* Ordinary
fast-forward pushes of `main` are standing-authorized. **Not** covered: cutting
a tag or release, `git push --force` or anything rewriting published history,
and any branch other than `main`.

★ **Scrub `tools/check-suite-name-absent.py` green before every push.** The
repository is public, so a push publishes. That gate scans **untracked** files
too — scratch renders belong in `D:\Dev\temp\pdfce\`, never in the tree.

★ **Fuzzing needs the MSVC ASan DLL on `PATH`** or every target dies at launch
with `0xc0000135`. Locate it with
`find "/c/Program Files (x86)/Microsoft Visual Studio" -iname "clang_rt.asan_dynamic-x86_64.dll"`.

---

## §B — THE DISK, WHICH IS NO LONGER THE NEAREST LIMIT

**202 GB free of 954 GB (79 % used)** at write time. The last handoff recorded
14 GB and treated it as the binding constraint; it is not any more. A session
with six release builds of `pdfce-cli` and a full fuzz build stayed
comfortable. `cargo clean` still recovers ~21 GB if needed.

---

## §C — WHAT IS OPEN AND ACTIONABLE

**If you want one thing to pick up: `Pass 143.0`.** It is fully diagnosed, has
a named wrong line, needs no operator decision, and closing it should take the
print-conformance suite from 6 FAIL back to 5 or better.

### `Pass 143.0` — `DeviceGray` under overprint knocks out the backdrop

A 50 % `DeviceGray` fill overprinting a spot backdrop erases it; Acrobat
preserves it. `overprint::classify` maps `ColorSpace::DeviceGray` to
`SourceKind::OtherProcess`, which `cmyk_group_rules` gives
`[ComponentRule::Source; 4]` — the source in all four components.

★ **This is a SPEC READING, not a bug, and that is why it is its own Pass.**
ISO 32000-1 §8.6.7 scopes `OPM 1` to *"a tint value of 0.0 for a colour
component **in a `DeviceCMYK` colour space**"*, and `DeviceGray` is not one, so
pdfce's literal reading is defensible. Acrobat converts grey to K-only CMYK
**first** and then applies `OPM 1`. Sourced:
`PDF_Spec\iso32000\iso32000__s__8.6.7.md`, whose `OPM-2` row already records
the neighbouring case (a CIE-based space *implicitly converted* to
`DeviceCMYK` **does** get `OPM 1`) — which is the strongest argument that the
grey case was an omission rather than an exclusion.

**Shape of the fix**, per the standing *"two defensible answers? ship both,
pick the default"* rule: an ambiguity setting, defaulting to Acrobat's
behaviour, because this is a print-conformance axis and the suite is authored
to press behaviour. **Do not ask the operator** — he has refused to be asked
twice about exactly this class.

### `Pass 142.0` / `142.1` — a REAL bold/italic face, and the missing pre-flight

Unchanged and still waiting on `pdfceGUI`. **Bold and italic already work on
existing text** via `FormatRequest::set_synthetic`; what `142.0` adds is a
*real* typographic face. The question with them is *"is a disclosed synthetic
weight enough, or do your operators need a real face?"* — if synthetic
suffices, `142.0` drops down the queue. **Read
`correction_bold_and_italic_DO_work_…` on their channel, not the longer reply
beside it.**

### The other four print-conformance failures

Spot-colour overprint (6 traps), white overprint (3, down from 5), ICC source
profile (4), blend modes in an ICC RGB group (12). Run:
`python tools/suite-check.py <corpus> --reference-dir <refs>` — the private map
at `D:\Dev\pdfce-private\suite\` names both directories, and set
`PYTHONIOENCODING=utf-8` or its `--help` dies on a star character. **Patch
stems are deliberately not listed here.**

### Owed, small, unstarted

`--in-place` on the other `pdfce-cli` editing subcommands. Grep `main.rs` for
`Never the input path by default`.

---

## §D — THE FINDING, AND IT IS ABOUT HOW I MEASURE, NOT ABOUT COLOUR

`70c5919`'s commit message contained a number I had not measured: *"292
distinct tuples behind 25,870 texels"*, attributing `tint_applied=292` to that
image's `TintCache`. **The counter had no image contribution at all** — it was
the page's path fills, because the decode's diagnostics reached nothing. I
attributed a counter to a producer that could not have reached it, and the
reason I could not tell is the same defect `140.2` then fixed.

> **A census counter that omits one producer is not a smaller number. It is a
> different question, and nothing in the name says which.**

★★ **And the fixture had to be image-ONLY to see it.** Every other page in the
new fixture set carries a fill beside its image, and a fill's conversions *are*
counted — so on those pages the counter reads a plausible non-zero number
whether or not the image contributes. **A page with a second producer cannot
detect a missing producer.**

★★★ **The other correction is the one to carry forward.** Ablated on a debug
build: the `/Indexed`-over-`DeviceN` route — the one that was **not in the bug
report**, added only because `R219` says enumerate every route — accounts for
**the entire fix on the reported patch** (25,870 bridged → 0 with it reverted;
0 → 25,870 native with it restored). **Had I implemented only the route the
report named, the reported defect would not have been fixed at all.**

---

## §E — WHY THE SUITE READS 6 AND SHIPPING WAS STILL RIGHT

The print-conformance suite went **5 FAIL → 6 FAIL** and the rise is a **false
pass being removed**. Established by measurement, not argument, and the method
is the transferable part:

1. **Ablate.** `140.0` alone scores identically to the baseline. Every
   movement is `140.1`'s.
2. **Segment the trap box by exact grey level**, rather than comparing means:

   | object | Acrobat | before | after |
   |---|---|---|---|
   | surround | `84,120,34` | `127,127,127` | `127,127,127` |
   | the trap X | `84,120,34` | `128,128,128` | `76,117,31` |

   Acrobat paints both the same colour, so its X is invisible. pdfce painted
   both the same **wrong** colour, so its X was invisible too and the patch
   scored clean. `140.1` made the X correct and left the surround.
3. **Run the detector on the reference itself.** Zero traps on Acrobat's
   render, three real 49×49 marks at diagonality 1.00 on pdfce's — so the
   marks are adjudicated, not detector noise.
4. **Measure distance to the oracle.** On the 20,790 pixels that changed:
   **108.6 → 25.0**. The other two affected patches also improved.

> **A rising failure count can mean a false pass was removed. Measure
> distance-to-oracle before reverting.**

Reverting `140.1` would have restored a matched pair of errors, re-opened an
8–9 level fill-vs-image disagreement that `140.0` itself creates, and made two
patches worse. The surround is `Pass 143.0`.

---

## §F — HABITS THIS SESSION PAID FOR

### Sabotage caught a false claim in a COMMENT, not a defect in the code

Three sabotages of `140.2`; two failed tests, one **failed nothing**. Deleting
the `scratch_diag` merge changes no test — and the code comment beside it
claimed that merge kept "every non-`Special` image" from being silent. That was
**false**. The comment now records the line as *deliberately uncovered* and
says exactly why. Sabotage is not only a test-quality instrument.

### Run the gate, then check what the gate wrote

`tools/check-image-colorspace-truth.py` writes a `_truth/` subdirectory beside
the images it scores. Pointing it at `fixtures/synthetic/images` — the obvious
thing — made a `pdfce-core` test die on `std::fs::read` of a **directory**,
which on Windows is `PermissionDenied` / "Access is denied.", so the panic
sends the reader after their antivirus. **One of this project's own gates broke
one of its own tests.** Fixed in `25d73d7`; `.gitignore` would not have helped,
because the failure is on disk and has nothing to do with what is tracked.

### Grep the format strings in the same change as the doc comment

The stale-claim sweep found **five** sites, not the four the librarian's sweep
reported. The fifth was a `println!` format string, which no gate reads for
implementation-state claims. It was stale in the **opposite** direction from
the doc comments: the runtime note had been *correct* for ten Passes while the
doc comment was wrong, and `140.0` made the note false and the doc comment
accidentally true again — **repairing a stale claim by a code change rather
than an edit, leaving no trace of it ever having been wrong.**

> **When two copies of a population claim disagree, neither being newer nor
> better-placed is evidence of either being right.**

### `cp x /tmp/x.bak` before every sabotage

Used eight times this session with no loss. `git checkout --` is not an undo —
it reverts the whole file.

---

## §G — OPEN, UNCHANGED

- **`(bl)`** — may a **CC-BY-SA-4.0** OCR model ship inside pdfce's **MIT**
  portable folder? Ken's call; default if unanswered is **ship neither model
  set**. `docs/ocr-engine-survey.md`.
- **`R13` vs "download addin capability"** — downloading is permitted;
  **executing** what was downloaded is not, and that ruling is owed from the
  operator. **No add-in Pass can be scoped until it lands.**
- **`(p)`** — whether to narrow the XFA item to read/fill only, or retire it.

## §H — THE CHANNELS, AND CHECK THEM ANYWAY

Both `D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\` and
`iccce_FeatureRequests\open\` were clear at session start and nothing new
arrived. **They live outside the repository, so no gate can contradict a stale
"it's empty" claim** — list them yourself.

Replies still awaiting the other side:

| file | awaiting |
|---|---|
| `note_the_writing_direction_is_published_and_your_400_lines_can_go.md` | pdfceGUI deleting `canvas::textsel::writing` |
| `note_opacity_at_author_time_shipped_and_it_was_an_undo_bug.md` | pdfceGUI wiring `add_markup_with` |
| `reply_restyle_…` **+ `correction_bold_and_italic_DO_work_…`** | **a decision from them**: is a disclosed SYNTHETIC weight enough, or do their operators need a real face? |
| `iccce/reply_the_profile_census_and_your_33_node_constant.md` | iccce re-examining their 33-node constant |

★ **One disagreement there still needs resolving rather than averaging.**
`iccce` names `USWebCoatedSWOP.icc` as carrying a 33-node `lut8`; the
`U.S. Web Coated (SWOP) v2` in this corpus reports **9**. Either they are
different files or one of us reads the grid byte from a different offset.
pdfce's is byte 10 of the tag per ICC.1:2010 §10.10/§10.11 and is
sabotage-tested — but a disagreement about a specific named file is the kind of
thing to settle, not split.
