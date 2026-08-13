# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. Overwrite it once acted on.

**State at handoff (2026-08-12, evening):** branch `main`, `HEAD` =
`5b54a1eca2b28b3848fa7438586b909fcedff183`, tag **`v0.5.2`** at HEAD, **CI GREEN**, working tree clean, every
commit filed (`tools/check-commits-filed.py` clean over 388 commits).
Nothing is broken and nothing is half-committed.

---

## ⇢ IF THE OPERATOR JUST SAID "CONTINUE", DO THIS

**Build the GUI gesture for two-line ce dimensioning — §1 below.**

That is the top of the queue because it is the operator's own request, its
core and CLI halves are already built and tested, and every design decision
it needs is settled and written down in §1 (classification, the override, the
threshold setting, the four-angle pick rule, virtual-apex disclosure). The
canvas does not need to re-derive any of it — it needs a gesture wired to
`pdfce_core::vector::linepick::pick_line`, which today has **no caller**.

Before writing code, in this order:
1. `git log --oneline -8` — see what actually landed last.
2. Read §1 in full. It names the exact functions and the settled decisions.
3. Read `docs/ui_specs/pass-46-canvas-interaction-model.md` §2 for the
   canvas tool contract every gesture obeys (arm → options in the pane →
   gesture targets the pointer → commit).
4. Grep `D:\devag\egui\` before driving the GUI harness — **not only
   before writing code**. Standing rule R172 exists because that step was
   skipped twice in one session, both times at harness-driving time.

**If instead the operator names something else, §§2–6 are the queue in
priority order** and §§7–8 are standing obligations that apply regardless.

**Do NOT start by asking him what to work on.** He said continue; §1 is the
answer.

---

> **★★ CI WAS RED ON `main` AND THE HUNDRED-AND-TWENTY-SEVENTH FILING WAS
> THE FIX.** Four consecutive runs failed, **all on `check-commits-filed`
> and nothing else** — **no code failure**. `1496e13`, `0d7c1bd` and
> `bc13a86` were in no filing; they are now. **Re-run the gate before
> assuming the branch is green**, and note that `1496e13` was flagged
> despite touching `docs/`: the exclusion is **docs-ONLY**, not
> docs-touching, so a commit mixing a filing with a source change is
> visible to the gate exactly as it should be.

> **★★ READ THIS TOO — A COMMIT DISCIPLINE ITEM, AND IT IS EVIDENCE FOR
> ESCALATION 2 (§6).** `af5580e` was committed **while the previous filing
> was being written** and **swept that filing's uncommitted doc edits into
> itself**. **Measured by `git diff --numstat af5580e^ af5580e`: 1,371
> insertions, of which 281 are the withdrawal** (`ocr/models.rs` 276 +
> `ocr/mod.rs` 5) **and 1,090 are a librarian filing about five OTHER
> commits** (`ROADMAP.md` 802, `ARCHITECTURE.md` 252, `CLAUDE.md` 32,
> `FEATURES.md` 3, `PRIOR_ART.md` 1). **So the commit titled *"withdraw
> the downloader I proposed"* is 20% about the downloader.** The
> restricted diff contains **zero** *"downloader"* and **seven**
> *"hundred-and-twenty-sixth"*.
>
> **A measurement trap, because it looks like a refutation:** `git show
> af5580e -- docs/ROADMAP.md | grep -i downloader` returns **four** hits
> and **all four are in the commit-message header `git show` prints above
> the diff.** *Restrict the diff (`git diff A^ A -- path`), do not filter
> the show.*
>
> **Nothing is undone** — it is on a public remote and amending after the
> fact is worse than the disorder. **The convention owed is cheap: `git
> status --short` before `git commit -a`, and a librarian filing gets its
> own commit.** The hundred-and-twenty-fifth filing's *"this filing
> touched nothing outside `docs/`"* is the positive instance, and it said
> that **because it checked.** The hundred-and-twenty-seventh filing was
> told explicitly by the engineer that he would stage `docs/` by name —
> **that is the convention taking hold, and it should keep holding.**

**★ TERMINOLOGY, binding on every dispatch this file generates (project
rule 15):** items 1 and 3 are about **ce dimensions** — the ones **pdfce
authors** (`/Line` + `/IT /LineDimension` + `/Measure` + the `/PieceInfo`
sidecar, everything under `crates/pdfce-core/src/dimension/`). **pdf
dimensions** — CAD-exported ones already in the file — enter **only as
pickable page geometry**, and pdfce must not alter them. **A subagent
handed the unqualified word writes an entire analysis in it.**

---

## 1. ★★★ `Pass 68.0` HAS CORE **AND CLI**. THE GUI GESTURE IS THE REMAINING HALF, AND IT IS THE OPERATOR'S ACTUAL REQUEST. TOP OF THE QUEUE.

**Operator request (i), verbatim:**

> *"dimensioning tool should allow the selection of two lines. if those
> lines are parallel it makes a linear dimension between them like
> SolidWorks would, if they are at an angle it makes an angle dimension."*

**★ WHAT LANDED SINCE THE LAST HANDOFF — THE CLI HALF IS REAL
(`bc13a86`).** `dimension-add --kind two-lines` takes **four points, two
per line**, and reads the geometry:

| relation | authored |
|---|---|
| **parallel** | a **LINEAR** ce dimension of the **perpendicular** distance |
| **angled** | an **ANGULAR** ce dimension of the angle between them |
| **collinear** | **REFUSED BY NAME** — a zero-length ce dimension is a mark that is present and invisible, and the operator would hunt for it |

**Measured on the release binary, all three:**

```
100,100 300,100 100,140 300,140  ->  two_lines authored=linear  distance=40.0000
100,100 300,100 100,100 273,200  ->  two_lines authored=angular degrees=30.029
0,0 100,0 200,0 300,0            ->  refused, "COLLINEAR"
```

- **`--treat-as-parallel` is the CLI form of the operator's checkbox and
  STILL PRINTS THE ANGLE IT OVERRODE** — `measured_angle=5.001 forced=1
  authored=linear`. A control hiding the number it overrides asks for a
  decision while withholding the fact that makes it one.
- **The threshold reads `Settings::parallel_epsilon_degrees`, never a
  literal**, so the CLI and the GUI slider **cannot come to disagree**
  about when two lines count as parallel.
- **The pick point defaults to each segment's MIDPOINT, and says so.**
  Two crossing lines bound **four** angles; a GUI operator picks by
  clicking, **someone typing coordinates has no click**, so the default is
  unavoidable and the only question is whether it is written down.
- **A virtual apex is dimensioned AND disclosed** — `apex_is_real=0` plus
  a sentence. Refusing would be wrong (CAD dimensions virtual
  intersections routinely); staying quiet would be too.
- **The tests read results back through `dimension-list`**, not from the
  authoring command's own stdout: *what pdfce decided* and *what is in the
  file* are different claims and **only the second is what the operator
  ends up with.** `kind=angular value="30.0°"` and `kind=linear
  value="40.00 pt"` both verified from the saved document. **3,587 → 3,593
  tests.**

**WHAT IS BUILT (core, two commits, both shipped):**

- **`e931836`** — **`crates/pdfce-core/src/vector/linepick.rs`** (new).
  Returns a **line as two endpoints**, which nothing in the codebase could
  do (`snap_candidates` → points, `hit_test_subpaths` → indices,
  `centerline` → thin filled quads only). **`PickedLine` records WHERE ON
  THE LINE the operator clicked** — load-bearing, because **two crossing
  lines bound FOUR angles** and SolidWorks' own `AddDimension2` docs say
  the result depends on which endpoints were selected. Test: the same two
  lines give **60° or 120°** purely from which arms were clicked. Curves
  are **skipped, not chorded**.
  Plus the two things the operator asked for mid-build:
  **`Settings::parallel_epsilon_degrees`** (default **0.5°**, range
  **0–45**, GUI slider — `R169`, a choice no standard makes is a setting)
  and **`ParallelPolicy::force_parallel`**, **checked BEFORE the
  threshold** so a forced pair never depends on the global value, and
  **without faking the measurement** (`measured_angle_degrees` still
  reports the truth, so a shell can disclose *"these are 0.8° apart"*).
- **`905791f`** — **`DimensionKind::Angular`**, the third variant (apex,
  a unit direction per arm pointing INTO the wedge, arc radius, position
  along the arc). Arms stored **resolved**, not as the source lines.
  **`display()` branches on `is_angular()` BEFORE applying scale** — an
  angle is invariant under uniform scaling, and 30° through the length
  path in a 1:50 group renders **`1500`, with no unit and no disclosure**.
  `format_angle_degrees` takes only the **decimal marker** from the
  group's `NumberFormat` (**ISO 129-1 cl. 4.1.1**). **`SIDECAR_VERSION`
  1 → 2**; old builds still READ, the session refuses to WRITE.

**WHAT IS NOT BUILT — one item, and it is the request itself:**

1. **THE GUI GESTURE.** The canvas cannot select two lines and author a ce
   dimension. **This is the whole remaining Pass.** The request was phrased
   as a *dimensioning tool* gesture — *"should allow the selection of two
   lines"* — so a scriptable subcommand does not discharge it.

**★ ALREADY SETTLED BY THE CLI HALF, SO THE GUI DOES NOT RE-LITIGATE IT:**
the classification, the override, the threshold source, the four-angle
disambiguation and the virtual-apex disclosure are **decided and tested**.
**The GUI owes a gesture and a disclosure surface, not a second reading of
the geometry.** Building a second classifier at the canvas is how the two
shells acquire the disagreement `Settings::parallel_epsilon_degrees` was
introduced to prevent.

**`FEATURES.md` carries this as core `[x]` · cli `[x]` · gui `[ ]`** —
`R151`'s shape one rung up, filed as a signal. **Do not round it up.**

**Where to read:** `ROADMAP.md` *Next up* → `Pass 68.0`, which now leads
with **two** status blocks (hundred-and-twenty-sixth, then
hundred-and-twenty-seventh); the top-of-*Shipped* entry
(hundred-and-twenty-sixth filing) items 1 and 2 for the core delivery
record, and the hundred-and-twenty-seventh filing item 3 for the CLI. **`ARCHITECTURE.md` §4.1 (R)** carries the API contract, including
**the rule for when a `SIDECAR_VERSION` bump is owed** (optional-with-
default field: no. New variant: yes, because an old build **drops the
record and then saves without it**).

**Dispatch `pdfce-ui-specialist` before building the gesture** — a
two-click pick with a disclosed parallel/angled verdict and an override
tick is a non-trivial UI decision, and rule 4 (narrowed by decision 024
§4.4) governs how the verdict is disclosed: **the inference is the
parallel-vs-angled classification**, not the operator's own click, so it
needs disclosure but **not** a floating confirm box positioned relative to
the page. That placement is the exact thing the operator complained about.

---

## 2. ★★★ OCR — THE OPERATOR SAID **BUILD FOR BOTH**. THE SUBSTRATE EXISTS. NO ENGINE IS WIRED.

**Operator decision, 2026-08-12, verbatim:**

> *"use whichever one is best for everyone including other languages, or
> heck, just build for both."*

**This answers the engine question** — both, behind Cargo features — and
**implicitly prioritises multi-language coverage**. Filed as `Pass 71.0`
(*Next up*), promoted from the Backlog bucket.

**READ FIRST: `docs/ocr-engine-survey.md`** (116,991 bytes, written
2026-08-12). It is the sourcing record for everything below, and it is
**named in `ROADMAP.md` deliberately** so it cannot go untracked the way
`comparison__pdfce_feature_column.md` did.

**What shipped (`9f2af1d`, slice 1): the half that does not depend on
which engine wins.** `pdfce-core::ocr` turns recognised words with page
positions into an **invisible, selectable layer over an untouched scan**.
Sourced to **ISO 32000-1 §9.3.6 Table 106 mode 3**, which the spec corpus
names as the OCR mechanism by name. **The y-flip has exactly one home**
(`words_to_page_space`) — engines are y-DOWN, PDF is y-UP, and backwards
means **every word mirrored on a page that still looks perfect**.
**`confidence: Option<f32>`, `None` load-bearing** — an unscored word
needs review exactly like a low-scored one, and no confidence yields
`None`, never `0.0`.

**★ ALSO SHIPPED, mid-filing (`af5580e`): a WITHDRAWAL you should confirm
with the operator, and `ocr::models`.** The engineer had described an
optional-**download** path for model files as *"a download prompt and a
bit of plumbing"*, **the operator agreed on that basis**, and **the
estimate was wrong — so the agreement was uninformed.** He withdrew it
rather than build it, because `ARCHITECTURE.md` §1.1 states in writing
that **pdfce contains no HTTP client and no TLS stack**, verifiable from
`THIRD_PARTY_LICENSES.md`, and the **fail-closed no-network CI job
(`R12`)** would have blocked it anyway. **Confirm the withdrawal with
him** — he agreed to the thing that was withdrawn, and only he can say
whether he still wants it.

The replacement is **`ocr::models`** (`crates/pdfce-core/src/ocr/models.rs`),
the **`--font-dir` pattern again**: operator-named path → `models/<engine>`
beside the executable → user data. **A named path that does not exist is
an ERROR, never a silent fallback** (a fallback runs a *different* model
while reporting success, and **the output is text either way**). **When
nothing is found, every searched path is printed.** **Per-engine
directories**, because the two engines' weights carry different licences.
**3,582 → 3,587 tests.**

**★★ THE BLOCKER, AND IT IS NOT AN ENGINEERING ONE: open operator
question `(bl)` — NARROWED by `af5580e`, NOT CLOSED.** An operator
supplying his own weights is not pdfce redistributing them, so the
copyleft question does not arise on that path — **but on that path OCR
does nothing out of the box until he goes and fetches a model set, which
is a trade he has not been asked about.** The question stays live for any
build that **bundles** a model set. May a **CC-BY-SA-4.0 model file** ship inside pdfce's
**MIT** portable folder? **"Build for both" selects engines; it does not
accept share-alike terms on a bundled asset.** The trade is real in both
directions:

| candidate | wasm32 | languages | weights | confidence |
|---|---|---|---|---|
| `ocrs`/`rten` (pure Rust, ~12 MB) | **the ONLY passing route** | **Latin only** | **CC-BY-SA-4.0** | **none, at any level** |
| `ocr-rs` / PaddleOCR on static MNN (3.2 MB models, zero DLLs on MSVC) | **none** | **50+** | **Apache-2.0** | yes |

So it is *"copyleft weights and a web future"* versus *"permissive
weights, more languages, no web future"* — **not** *"one is clean."*
**Raise it; do not resolve it** (`pdfce-engineer.md`: legal decisions get
surfaced, not decided). **Do not resolve it by picking the permissive
engine and calling it moot** — that silently forfeits the WASM route,
which is an `ARCHITECTURE.md` §3 commitment.

**Recorded traps, so nobody re-derives them:**
- **Surya** — Apache-2.0 code, **modified Open RAIL-M weights with a $5 M
  revenue cap**. Field-of-use restrictions **cannot** be bundled in an MIT
  app. Do not re-evaluate it on its (genuinely good) accuracy numbers.
- **Tesseract's default Windows build ships LGPL binaries**
  (`libunistring`, `libiconv`, `libintl`, via the libcurl branch) — the
  "obvious" choice is weak-copyleft in its shipped form.

**Also already captured, so it survives a compaction:**
`C:\personal_rag\pdf\lesson_20260812_ocr_text_layer_bt_et_per_line_poppler_tz.md`
— OCRmyPDF emits one `BT…ET` **per LINE**, not per word, as a documented
**poppler** workaround (`Tz` is not carried across `BT`/`ET`, though §9.3
says text state persists). **Decide pdfce's emission granularity with that
constraint visible**, not after a corpus of sandwich PDFs exists.

---

## 3. `Pass 69.0` — ce-dimension style + tolerance — UNSTARTED

Operator requests (ii) and (iii), verbatim:

> *"groups of dimensions should have a default dimensioning and tolerance
> style that can be set for the group, but these should have a checkbox to
> override and set differently."*

> *"they should have the same options as SolidWorks does for dimensions."*

**Read `D:\Dev\Rag-Specialized\SolidWorks_Dimensions\` first** (measured:
exists, carries `index.md` and
`solidworks__dimension_and_tolerance_options.md`). Acceptance criteria are
drafted in the `Pass 69.0` *Next up* entry and are to be **firmed against
that RAG**, not against recall. **SolidWorks is the FLOOR, not the
ceiling** (user memory *exceed the parity reference when you can*); record
any deliberate divergence.

**`68.0` and `69.0` are independent.** `69.0` already has a written spec
(`docs/ui_specs/tool-options-dock-and-ce-dimension-properties.md` — update
it, do not shadow it with a second spec). Splitting into `69.0` (style) +
`69.1` (tolerance) is fine; `69.1` is free.

**One thing `68.0` has now proved for `69.0`:** the `SIDECAR_VERSION` rule.
`69.0`'s criterion 4 says every new field must be optional-with-default and
version-gated — **that is now a stated rule with a worked example**, not a
hope. See `ARCHITECTURE.md` §4.1 (R).

---

## 4. `Pass 46` slices 2–4 — the operator's *"I can't drag or resize them"* is still HALF unanswered

Slice 1 (`7825424`) fixed the half he described first: **markup is now
drawn where you point**, not dropped in the page centre. **Slice 2 — post
hoc select, move and resize a placed annotation — is the other half and is
NOT built.** Slice 3 is the remaining six markup kinds; slice 4 is Family B
reshape. Spec: `docs/ui_specs/pass-46-canvas-interaction-model.md`.

**This also gates** click-a-comment-to-select and canvas selection of ce
dimensions and foreign annotations.

---

## 5. The GUI has NO attachments surface at all

Core and CLI finished in `95c3416` (`extract` / `attach` / `detach`, with
detach removing the stream, not just the name-tree entry, and a multi-node
`/Kids` name tree refused by name). **`gui [ ]` — recorded, not rounded
up.**

---

## 6. ★ TWO ESCALATIONS STILL AWAITING THE OPERATOR — raise them, don't resolve them

1. **The broken no-git convention** (`iccce`).
2. **Agents' in-progress files swept into a public repo.**

**Carried from the engineer's context across two filings now, with NO
supporting detail supplied either time, and the filing librarian could
find NO written record of either anywhere in `docs/`.** Recorded so a
compaction does not lose them — **not** as established findings. **The
exact content of both must come from the operator or the engineer.**

**★★ ESCALATION 2 NOW HAS ONE CONCRETE, MEASURED INSTANCE — see the block
at the top of this file.** `af5580e` swept five files of an in-progress
librarian filing into a commit about something else, on a public remote.
**That is EVIDENCE FOR the escalation, not its resolution.** The
escalation as relayed may concern something broader — agent worktrees,
scratch files, `.claude/` state. **Do not close it on the strength of this
instance; ask him what he meant.**

One check that bears on the first: **`D:\Dev\iccce\` DOES contain a `.git`
directory** (verified by `ls`), so whatever the claim is, it is **not**
"that project has no repository." Get the actual statement before acting.

---

## 7. ★ BACKUP — re-measure it, do not quote this number

**Measured 2026-08-12 ~19:5x**, after the `v0.5.2` release:

- Newest bundle: **``**
- `HEAD` = **`5b54a1eca2b28b3848fa7438586b909fcedff183`**, branch `main`, tag `v0.5.2` at HEAD.
- The bundle above predates the `v0.5.2` version-bump commit, so **a fresh
  bundle is owed** — `git bundle create <path> --all` then
  `git bundle verify <path>`.

**This ledger has carried a WRONG backup figure twice.** `ls -t` and
`git bundle list-heads` cost nothing. **Re-run them; do not quote the number
above without re-running it, including when the number above is this one.**

---

## 8. Carried forward, unchanged in substance

- **★ NEW, from `fbcb946` — strippable capabilities now EXIST, and the
  convention is in `crates/pdfce-core/Cargo.toml`'s `[features]` header.**
  Adding one: **default ON**, **forward it from EVERY shell**, **both CI
  gates**, **refuse by name when stripped**. **Forgetting to forward does
  not break the build — it removes a capability**, silently. Decision 055
  in `ARCHITECTURE.md` §12 has the full reasoning; the ecosystem-wide half
  is at
  `D:\dev\rag\rust\workspace_feature_strip_needs_root_default_features_and_every_shell_forwards_or_the_capability_vanishes_silently.md`.
- **★ from `e3fb7e0`, COMPLETED BY `0d7c1bd` —
  `tools/check-shipped-assets.py` now gates every directory under
  `crates/*/assets/`.** It requires a `PROVENANCE.md`, that it **NAMES A
  LICENCE**, that it mentions the files present, **and (check 4) that the
  directory is cited in `about.hbs`.** **If you add a shipped asset
  directory, write the provenance note AND the `about.hbs` section
  first**; the gate is wired into CI's `fmt` job.
  - **★ WHY CHECK 4 EXISTS: `PROVENANCE.md` DOES NOT SHIP.** The portable
    package carries `LICENSE`, `THIRD_PARTY_LICENSES.md` and `README.md`
    and nothing else, so an asset documented only in the source tree
    reaches end users **with no notice attached**. **`about.hbs` is the
    `cargo-about` TEMPLATE**, so a hand-written section there renders into
    the generated file and travels with every release. **The bundled Foxit
    faces are the worked example to copy.**
  - **Own work is EXEMPT** — art under pdfce's own licence needs no
    `about.hbs` entry, because the shipped `LICENSE` already covers it and
    there is no third-party grant to reproduce. Check 4 flagged the GUI
    icon set on its first run, before the exemption existed; **that was a
    false positive, and a gate that fires on a correct state is one people
    learn to skip.**
  - **This is where `(bl)` lands.** Bundled CC-BY-SA-4.0 weights would be
    third-party, so check 4 is what forces their notice to travel with the
    binaries. **The gate exists before the decision, deliberately.**
- **`R192` is PROPOSED, NOT MINTED** (end of *Standing rules*) — *an
  obligation that falls between two correct tools is enforced by neither*.
  **Third instance recorded.** The engineer's ruling is owed.
- **`Pass 67.0` phases C, D and F** — unstarted, none blocking. C =
  re-subset (lowest risk, no visual change, works where B must refuse).
  D = text to outlines (universal escape hatch; irreversible — disclose
  the cost inline). F = replace font X with Y (hardest; Acrobat has **no
  equivalent** — parity-plus). **Ask the operator which he wants rather
  than guessing an order.** Reusable substrate:
  `FontEnvironment::resolve_for_embedding`'s four-rung donor ladder,
  `fontinfo::Removability`'s nine-verdict classifier, and the shared
  descriptor-reachability code.
- **`/R` 6 encryption still parked** at the operator's explicit
  instruction. The sourcing blocker is gone — ISO 32000-2 is at
  `D:\Dev\Rag-Specialized\PDF_Spec\_sources\ISO_32000-2_sponsored_EC3.pdf`,
  **SINGLE USER ONLY, watermarked to the operator by name: never
  committed, never shipped, never a release asset — paraphrase and cite
  only.** The repository is public, so this is not hypothetical.
- **The CI job's NAME does not name the gate that fails.** Every red X
  renders as **`verify pdfce-gui strings live in ui_text.rs`**, but that
  job runs **three** unrelated gate steps. Rename or split it. Small and
  actionable — and it now hosts a **fourth** gate
  (`check-shipped-assets.py` is in the `fmt` job, not this one, but the
  same naming problem applies there).
- Two dead/stale printing items, filed to Backlog, deliberately not fixed:
  `DeviceSettings::pick_tray_by_page_size` sets no `DEVMODE` field at all;
  `build_devmode`'s doc claims a driver-default start the code does not do.
- **Imposition has no GUI** — extract sheet composition into `pdfce-print`
  first so both shells share one implementation.
- Static hybrid XFA read/fill · wide-shape CSV · colour management
  (`D:\Dev\iccce\`, planned, no code).
- **Ledger-accuracy defect, still not fixed:** filings ninety-two through
  ninety-five cite `(bh)`/`(bi)` as if `(bi)` had not been minted.
- **Spec-librarian flag, still open:** confirm the eight-item
  never-encrypted list (E1–E9) is in the §7.6 corpus rather than only in
  pdfce's code.
- **`CLAUDE.md` rule 8's literal per-release wording is stale** against the
  operator's 2026-08-11 standing release authorisation — flagged across
  several filings, not yet amended by him; **not the librarian's or the
  engineer's file to edit.**

---

## Open operator questions — `(bl)` is NEW; `(bk)` is ANSWERED, do not re-surface it

- **`(bl)` NEW, OPEN** — may a CC-BY-SA-4.0 model file ship inside pdfce's
  MIT portable folder? See §2. *Default if unanswered: ship neither model
  set.* **It blocks OCR from reaching an operator.**
- **`(bk)` ANSWERED** (options A and B together — the bundled-font licence
  now travels inside the document). **Do not raise it again.**
- Ceiling `(bk)` → `(bl)`; **next free `(bm)`**.

---

## Release state — `v0.5.2` (2026-08-12), and it is CLEAN

Tag `v0.5.2` → the version-bump commit, **CI GREEN at the tagged commit,
verified by `tools/verify-release.py v0.5.2` reporting all seven checks ok**
— including the new `CI is GREEN at the tagged commit` check. Asset
`pdfce-v0.5.2-portable-win64.zip` (10,290,964 B). Packaging smoke test run
from a fresh folder: two-line dimensioning and the attachment round trip both
exercised on the copied binaries.

**This is the first release in this project's history tagged at a commit CI
had accepted.** The three before it were not:

| tag | CI at the tagged commit |
|---|---|
| `v0.5.2` | **green** |
| `v0.5.1` | red — `check-commits-filed` only; code proven fine |
| `v0.5.0` | red — same gate |
| `v0.4.0` | green |
| `v0.3.0` | red — `cargo test` + `clippy` + cross-target |

`v0.3.0` is the serious one and stays on the record: a published release
whose tests did not pass. It was invisible locally because those jobs run on
ubuntu/macOS/wasm32 while the engineer verifies on Windows — **cross-platform
breakage is invisible to local gate runs by construction**, which is the
standing argument for consulting CI rather than re-deriving confidence.

**★ THE ORDERING RULE, now enforced by the tool: FILE, LET CI GO GREEN,
THEN TAG.** Run `tools/verify-release.py <tag>` **before** tagging.
History: **3 of the last 4 releases (75%) were tagged at a commit CI had
rejected** — `v0.5.1` and `v0.5.0` on the filing gate only, `v0.4.0`
green, and **`v0.3.0` on `cargo test` + `cargo clippy` + the
`aarch64-apple-darwin` cross-check simultaneously — a published release
whose tests did not pass.** Those jobs run on Linux/macOS/wasm while local
verification happens on Windows: **cross-platform breakage is invisible to
local gate runs by construction.**

**★ NEW HAZARD FOR THE NEXT RELEASE:** `fbcb946` added a **lite build job**
and a **capability-presence job**. Neither has been through a release
cycle yet. **A red X on either is a real capability regression, not a
flake** — the capability job exists precisely because a missing forwarding
block compiles cleanly.

**The version bump comes BEFORE the tag, deliberately** — `--version`
prints `CARGO_PKG_VERSION`, so tagging a version the binary does not report
would ship a false claim in the one place a user checks it.

**Standing release authorisation is in force** (operator, 2026-08-11:
*"please continue to post the latest versions to git so I can try them on
my laptop at home"*): build, tag, publish the asset, run
`tools/verify-release.py`, report what went out. **Scope is narrow** —
pdfce builds for the operator's own testing. **NOT** blanket publishing
authority, **NOT** licence to treat repository visibility as an agent's
decision, **NOT** permission to skip verification.

---

## Tooling — corrections that cost time in prior sessions

- **★ NEW — `cargo build --no-default-features` on ONE member proves
  NOTHING.** Cargo unifies features across the graph; a sibling re-enables
  what you turned off, and the binary comes out **byte-identical**
  (10,438,656 B, measured, twice). **Verify with a size diff or
  `cargo tree -p <crate> -i <dep>`, never with "it built".**
- **★ NEW — `cargo tree -p <shell> -i <dep>` returning EMPTY is how a lost
  capability looks.** No compile error, no test failure. Run it per shell
  after any feature-manifest change.
- **`gui-shot.ps1` / `gui-drive.ps1` no longer leak a live `pdfce-gui`
  process** on non-happy paths (fixed in `ee4e1e4`: `try/finally`,
  pre-launch PID snapshot so it cannot kill an instance the operator
  opened, a verified kill with a 5 s poll). **If the operator reports a
  fighting mouse again, this is no longer the explanation.**
- **`observe-gui.ps1` had `try/finally` around bitmap disposal only, not
  the process.** **When you find a guard on one sibling, check what it
  actually wraps before crediting the others with it.**
- **`PDFCE_DIAG_VIEWPORT`**, not `PDFCE_VIEWPORT`. Four comma-separated
  numbers: `x,y,w,h`.
- **The diag script separator is `;`, not `,`.** A comma-separated script
  parses as ONE unparseable step and is silently skipped — the trace says
  `script-step-UNPARSEABLE`. **`tool:markup` is in the vocabulary.**
- **`gui-shot.ps1` and `gui-drive.ps1` default to different window sizes.**
  Read the trace's own `rect=`, never a screenshot's pixels.
- **Both scripts move the REAL cursor** and synthesise input on the live
  desktop. Say so before running one while the operator is at the machine.
- **A GUI control's traced rect describes the layout *request*, not what
  survived clipping.**
- **`gh run list --commit <SHORT-SHA>` returns an EMPTY LIST, not an
  error.** **Always pass a full 40-character SHA** (`git rev-parse <ref>`).
- **Resolve every short hash yourself with `git rev-parse`.** A fabricated
  full hash reached a filing once already and had to be corrected.
- **★ `git status --short` BEFORE `git commit -a`, and give a librarian
  filing its own commit.** `af5580e` swept five files of an in-progress
  filing into a commit about an unrelated withdrawal; its message does not
  describe most of its own diff (**281 withdrawal lines vs 1,090 filing
  lines, measured**). **Run `tools/check-commits-filed.py` at the END of a
  filing as well as at the start** — three filings running, it has named a
  commit that landed while the filing was being written.
- **★ NEW — `git show <sha> -- <path> | grep …` SEARCHES THE COMMIT
  MESSAGE TOO.** `git show` prints the message header above the restricted
  diff, so a grep for a term that appears in the subject line returns hits
  that are **not in the diff at all** — which reads exactly like a
  refutation of a claim about the diff's contents. **Use `git diff A^ A --
  <path>` when the question is "what does this diff contain".** Cost the
  hundred-and-twenty-seventh filing a near-miss on a measured claim.
- **★ NEW — a gate's DOCSTRING is not the gate.** `check-shipped-assets.py`
  documented a check 4 whose code was never inserted (the string
  replacement did not match; the docstring landed anyway). **Verify a gate
  by making the hazard occur, not by reading its source** — and verify it
  in **three** states (passing, genuinely-failing, and correctly-exempt),
  because a gate proven only on its passing case has demonstrated that it
  can return zero, which is also what a gate with no code in it does.

`tools/splice.py` — anchored substitution, all-or-nothing ·
`tools/check-fmt-excluded.py` (no arguments; the fmt gate for the 12 crates
`cargo fmt --all` cannot see — run it **beside** `cargo fmt --all --check`,
never instead of) · **`tools/check-shipped-assets.py` — NEW; requires a
licence-naming `PROVENANCE.md` per shipped asset directory** ·
`tools/verify-release.py <tag>` — **run it BEFORE tagging; it consults
CI** · `tools/check-commits-filed.py` — run it before assuming a dispatch
listed every unfiled commit. File the commit, **do not** add it to
`tools/commits-filed-baseline.txt` · `tools/check-ledger-numbers.py` — the
live ceilings · `tools/gen-embed-fixtures.py` /
`tools/gen-unembed-fixtures.py` · `tools/package-portable.py --note "..."`.

**Live ceilings at this filing** (by `check-ledger-numbers.py`, measured):
standing rules **R191** → next free **R192** (**claimed by an unruled
PROPOSAL**) · decision records **055** → next free **056** · SESSION_LOG
filings **127** → next free **128** · **Pass families up to 71** → next
free **`Pass 72`** · operator questions **(bl)** → next free **(bm)**.
**The hundred-and-twenty-seventh filing minted no Pass ID, no rule and no
decision** — `bc13a86` is an existing Pass's CLI half, `0d7c1bd` is a gate
fix (no-ID precedent: `e3fb7e0`, `b902ea0`+`b1ee1cf`), `1496e13` is a
filing plus a doc correction.
