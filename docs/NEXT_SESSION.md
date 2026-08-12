# NEXT SESSION — start here

Engineer-owned handoff (this filing written by `pdfce-librarian` at the
engineer's explicit request, same as every prior overwrite of this file).
Read this **before** the librarian's record — `ROADMAP.md` says what
shipped, this says what is in flight and what the next hour should be.
Overwrite it once acted on.

**Written 2026-08-12 (hundred-and-twenty-sixth filing), branch `main`,
finishing at `af5580e55ebd05a67be5d043cf9db8a5ec82da02`**, after **six**
commits `e931836` · `905791f` · `fbcb946` · `9f2af1d` · `e3fb7e0` ·
`af5580e`. The previous version of this file was written at `95c3416`;
**this is a full overwrite, not an amendment.**

> **★★ READ THIS BEFORE ANYTHING ELSE — A COMMIT DISCIPLINE ITEM, AND IT
> IS EVIDENCE FOR ESCALATION 2 (§6).** `af5580e` was committed **at
> 15:41:08, while this filing was being written**, and it **swept this
> filing's uncommitted doc edits into itself**. **Measured:** `af5580e`
> touches 7 files; **two are the engineer's** (`ocr/mod.rs`,
> `ocr/models.rs`) and **five are this filing** — `docs/ROADMAP.md`
> (+802/−5), `docs/ARCHITECTURE.md` (+252), `docs/FEATURES.md` (+5/−3),
> `docs/PRIOR_ART.md` (+1/−1), `CLAUDE.md` (+33/−3). Its ROADMAP diff
> contains *"hundred-and-twenty-sixth"* seven times and **zero** lines
> about the downloader or `ocr::models`. **So the commit message does not
> describe most of its own diff: 1,093 of its 1,371 inserted lines are a
> librarian filing about five other commits, under the title *"withdraw
> the downloader I proposed"*.**
>
> **Nothing is undone** — it is on a public remote and amending after the
> fact is worse than the disorder. **The convention owed is cheap: `git
> status --short` before `git commit -a`, and a librarian filing gets its
> own commit.** The hundred-and-twenty-fifth filing's *"this filing
> touched nothing outside `docs/`"* is the positive instance, and it said
> that **because it checked.**

**★ TERMINOLOGY, binding on every dispatch this file generates (project
rule 15):** items 1 and 3 are about **ce dimensions** — the ones **pdfce
authors** (`/Line` + `/IT /LineDimension` + `/Measure` + the `/PieceInfo`
sidecar, everything under `crates/pdfce-core/src/dimension/`). **pdf
dimensions** — CAD-exported ones already in the file — enter **only as
pickable page geometry**, and pdfce must not alter them. **A subagent
handed the unqualified word writes an entire analysis in it.**

---

## 1. ★★★ `Pass 68.0` IS HALF BUILT. THE OPERATOR'S REQUEST IS NOT YET MET. THIS IS THE TOP OF THE QUEUE.

**Operator request (i), verbatim:**

> *"dimensioning tool should allow the selection of two lines. if those
> lines are parallel it makes a linear dimension between them like
> SolidWorks would, if they are at an angle it makes an angle dimension."*

**WHAT IS BUILT (core only, two commits, both shipped):**

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

**WHAT IS NOT BUILT — and it is the request itself:**

1. **THE GUI GESTURE.** The canvas cannot select two lines and author a ce
   dimension. Everything above is substrate. **This is the work.**
2. **THE CLI.** `dimension-add` has **no angular kind** (rule 11 wants it
   in the same Pass as the GUI flow).

**`FEATURES.md` carries this as core `[x]` · cli `[ ]` · gui `[ ]`** —
`R151`'s exact shape, filed as a signal. **Do not round it up.**

**Where to read:** `ROADMAP.md` *Next up* → `Pass 68.0`, which now leads
with a status block; and the top-of-*Shipped* entry
(hundred-and-twenty-sixth filing) items 1 and 2 for the full delivery
record. **`ARCHITECTURE.md` §4.1 (R)** carries the API contract, including
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

## 7. ★ TAKE A BACKUP — 5 commits owed, and this figure was MEASURED, not inherited

**Measured at filing time**, by `ls -t`, `git bundle list-heads` and
`git rev-list --count`, not by reading any document:

- Newest bundle: **`pdfce-20260812-1356.bundle`**, whose `refs/heads/main`
  is **`e931836ab58cdf7a01ef075bb6086db6044d56f0`**.
- `git rev-list --count e931836..HEAD` = **5**.
- `HEAD` = **`af5580e55ebd05a67be5d043cf9db8a5ec82da02`**.
- `git remote -v` = `https://github.com/KenM76/pdfce.git`.
- **It was 4 an hour earlier, at `e3fb7e0`, and that figure was correct
  when measured.** Both are stated rather than one silently replacing the
  other — which is the whole reason this section exists.

**This ledger has carried a WRONG backup figure twice.** `ls` and
`git bundle list-heads` cost nothing. **Re-run them; do not quote the
number above without re-running it, including when the number above is
this one.**

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
- **★ NEW, from `e3fb7e0` — `tools/check-shipped-assets.py` now gates
  every directory under `crates/*/assets/`.** It requires a
  `PROVENANCE.md`, that it **NAMES A LICENCE**, and that it mentions the
  files present. **If you add a shipped asset directory, write the
  provenance note first**; the gate is wired into CI's `fmt` job.
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

## Release state — `v0.5.1`

Tag `v0.5.1` → `aad48c73…`. **Its CI run is RED on `check-commits-filed`
only** — three commits were tagged and released before the librarian filed
them. **The released CODE is fine and that is proven:** `git diff
--name-only aad48c7 68408f1` returns **only `docs/` paths**, and `68408f1`
passed CI fully. **Binaries fine, ordering wrong.**

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
- **★ NEW — `git status --short` BEFORE `git commit -a`, and give a
  librarian filing its own commit.** `af5580e` swept five files of an
  in-progress filing into a commit about an unrelated withdrawal; its
  message now does not describe most of its own diff. **Run
  `tools/check-commits-filed.py` at the END of a filing as well as at the
  start** — twice in a row now it has named a commit that landed while the
  filing was being written.

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
filings **126** → next free **127** · **Pass families up to 71** → next
free **`Pass 72`** · operator questions **(bl)** → next free **(bm)**.
