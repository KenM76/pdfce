# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. Overwrite it once acted on.

**State at handoff (2026-08-12, late evening):** branch `main`, `HEAD` =
`0963b5f2cac7bcbbe5ef2eadea62ecc3f4d63a53`, pushed, **tag `v0.5.3` at HEAD**,
**CI GREEN**, `verify-release` clean on all seven checks, every code commit
filed (`check-commits-filed.py` clean over 391). 3,632 tests, 0 failures.
Nothing is broken and nothing is half-built.

---

## ⇢ IF THE OPERATOR JUST SAID "CONTINUE", DO THIS

**Ask him which he wants first — §1 or §2 — and say why it is a real
choice.** This is the one handoff in a while where the queue does *not*
have an obvious head:

- **§1 `Pass 69.0`** (ce-dimension style + tolerance) is his own outstanding
  request, has a written spec and a RAG to firm it against, and is the
  natural continuation of the dimensioning arc just finished.
- **§2 OCR `Pass 71.0`** is blocked on **his** answer to open question
  `(bl)` — a licence reading only he can make. Raising it costs one
  sentence, and *not* raising it means OCR sits still for another session.

Do **not** silently pick §1 and leave `(bl)` unasked for a fourth session.
Ask both in the same message; start §1 while waiting.

---

## What just shipped, so you do not redo it

`Pass 68.0` is **complete — core, CLI and GUI**. Two-line ce dimensioning
works end to end: pick two lines on the canvas, pdfce reads whether they
are parallel or angled, discloses the reading, and authors a linear or
angular ce dimension on Accept.

| commit | what |
|---|---|
| `bd53ab3` | `pdfce-core::dimension::two_lines` — the shared authoring function; the CLI stopped owning it |
| `c4ec3f5` | the GUI gesture (`LinearPickMode`, `TwoLinePick`), + `docs/ui_specs/pass-68.0-…md` |
| `916da7e` | the hundred-and-twenty-eighth filing |
| `1f7ef59` | **three latent defects the feature exposed** — see below |

**`FEATURES.md`'s two-line row is now `core [x] · cli [x] · gui [x]`.**

### The three defects are the interesting part, and they rhyme

None was introduced by `Pass 68.0`; each was **latent and became reachable**
the moment angular ce dimensions could be authored from the GUI.

1. **`place_dimension` refused Angular** because it tested *"is this
   `Linear`"* — a guard written in Pass 27.1 when the only other kind WAS
   circular, so its name and message coincided. A third variant silently
   widened it. **R186's shape one rung down: a guard keyed on a marker
   rather than on the property it needs.**
2. **`author_dimension` re-derived the display value** and lacked the
   angular branch `DimensionModel::display` had — so the pane read `77.5°`
   while the `/AP` **baked into the document** read `77.47 pt`. One
   producer now (`DimensionKind::display_with`).
3. **The label font is `/WinAnsiEncoding` and the baker wrote UTF-8** →
   `77.5Â°`. Invisible for years because `caption_prefix` deliberately says
   `"DIA "`/`"R "` rather than a symbol: **the degree sign is the first
   non-ASCII character that writer has ever emitted.**

**★ The transferable lesson, and it cost a near-miss:** a test asserting on
**raw bytes** in a content stream can pass **vacuously**. The first version
asserted "`C2 B0` is absent" and passed — but so would the broken build,
because the writer **octal-escapes every high byte**, so neither form ever
appears raw. Assert in the encoding the writer actually emits (`\260`
present, `\302\260` absent). Both regression tests were then **seen to fail**
before being trusted.

---

## 1. `Pass 69.0` — ce-dimension style + tolerance — UNSTARTED

Operator requests (ii) and (iii), verbatim:

> *"groups of dimensions should have a default dimensioning and tolerance
> style that can be set for the group, but these should have a checkbox to
> override and set differently."*

> *"they should have the same options as SolidWorks does for dimensions."*

**Read `D:\Dev\Rag-Specialized\SolidWorks_Dimensions\` first** (exists;
`index.md` + `solidworks__dimension_and_tolerance_options.md`). Firm the
drafted acceptance criteria against that RAG, not against recall.
**SolidWorks is the FLOOR, not the ceiling** — record any deliberate
divergence.

Spec to UPDATE, not shadow:
`docs/ui_specs/tool-options-dock-and-ce-dimension-properties.md`.
Splitting into `69.0` (style) + `69.1` (tolerance) is fine.

**Two things `Pass 68.0` just proved for it:**
- The `SIDECAR_VERSION` rule is now a stated rule with a worked example
  (`ARCHITECTURE.md` §4.1 (R)) — criterion 4's "optional-with-default and
  version-gated" is no longer a hope.
- **The group-wide-default-plus-per-item-override shape already exists**, in
  `TwoLinePick::force_parallel` vs `Settings::parallel_epsilon_degrees`.
  Read how that pair works before designing `69.0`'s override checkbox —
  including the decision that the override **survives** a gesture clear
  (`linepick.rs`: a per-item override that resets is how a setting becomes
  a thing people fight).

---

## 2. ★★ OCR `Pass 71.0` — BLOCKED ON THE OPERATOR, AND HE HAS NOT BEEN ASKED

**Nothing here is an engineering blocker. Do not start building around it.**

He already answered the ENGINE question (2026-08-12, verbatim): *"use
whichever one is best for everyone including other languages, or heck, just
build for both."* → **both, behind Cargo features** (the mechanism shipped
as `Pass 70.0`, `fbcb946`), ranked on multi-language coverage.

**Open question `(bl)`, unanswered:** may a **CC-BY-SA-4.0 model file** ship
inside pdfce's **MIT** portable folder?

| candidate | wasm32 | languages | weights |
|---|---|---|---|
| `ocrs`/`rten` | **the ONLY passing route** | Latin only | **CC-BY-SA-4.0** |
| `ocr-rs` / PaddleOCR | none | **50+** | Apache-2.0 |

So it is *"copyleft weights and a web future"* versus *"permissive weights,
more languages, no web future"* — **not** *"one is clean."* **Raise it; do
not resolve it.** Do **not** resolve it by picking the permissive engine and
calling it moot — that forfeits the WASM route, an `ARCHITECTURE.md` §3
commitment. *Default if unanswered: ship neither model set.*

Shipped meanwhile: `pdfce-core::ocr` (`9f2af1d`, the engine-independent
text-layer substrate, ISO 32000-1 §9.3.6 Table 106 mode 3) and
`ocr::models` (`af5580e`, the `--font-dir` resolution pattern).
**Sourcing record: `docs/ocr-engine-survey.md`.** Recorded traps: **Surya**
(Open RAIL-M weights, $5 M revenue cap — do not re-evaluate on accuracy);
**Tesseract's default Windows build ships LGPL binaries**.

**Also confirm with him:** the model-**downloader** withdrawal (`af5580e`).
He agreed to it on a wrong estimate, so the agreement was uninformed; only
he can say whether he still wants it.

---

## 3. Still open from `Pass 68.0` — small, named, none blocking

- **Re-classifying a COMMITTED two-line ce dimension is structurally
  impossible**, not merely unscoped: `Linear`/`Angular` retain only the
  resolved geometry, never the source `PickedLine`s — deliberately, so a
  scale change cannot silently reinterpret a committed ce dimension. His
  *"when making **or editing**"* is satisfied pre-Accept only. **Ask
  whether that matters in practice** before building a new core capability
  for it.
- The `parallel_epsilon_degrees` **proximity caption** (ui-spec §12) — P1.
- A **live what-if verdict on hover** before the second click (ui-spec §5,
  named P2) — genuinely nice, deliberately not built.

## 4. `Pass 46` slices 2–4 — *"I can't drag or resize them"* still HALF unanswered

Slice 1 (`7825424`) fixed "markup is drawn where you point". **Slice 2 —
post-hoc select/move/resize a placed annotation — is NOT built.** Slice 3 is
the remaining six markup kinds; slice 4 is Family B reshape. Spec:
`docs/ui_specs/pass-46-canvas-interaction-model.md`. Gates
click-a-comment-to-select and canvas selection of ce dimensions.

## 5. The GUI has NO attachments surface

Core + CLI finished in `95c3416`. **`gui [ ]` — recorded, not rounded up.**

## 6. ★ TWO ESCALATIONS STILL AWAITING THE OPERATOR — raise, don't resolve

1. **The broken no-git convention** (`iccce`).
2. **Agents' in-progress files swept into a public repo.**

Carried across three filings now with **no supporting detail supplied**, and
no written record anywhere in `docs/`. **Recorded so a compaction does not
lose them — not as established findings.** Get the actual statement.

`af5580e` remains the one measured instance of (2) — it swept five files of
an in-progress filing into a commit about something else (**281 withdrawal
lines vs 1,090 filing lines**). **Evidence FOR the escalation, not its
resolution.** Note `D:\Dev\iccce\` **does** contain a `.git` directory, so
whatever (1) is, it is not "that project has no repository."

**The convention it bought is holding**: `916da7e` and `1f7ef59` were split
deliberately, `git status --short` run before each, `docs/` staged by name.

---

## 7. Release state — **`v0.5.3` IS OUT, and it is clean**

Tag `v0.5.3` → `0963b5f` (= `HEAD` = `origin/main`). Asset
`pdfce-v0.5.3-portable-win64.zip`, **10,308,179 B**, at
`https://github.com/KenM76/pdfce/releases/tag/v0.5.3`.
`tools/verify-release.py v0.5.3` reports **all seven checks ok**, including
**CI is GREEN at the tagged commit**.

**★ This is only the SECOND release tagged at a commit CI had accepted**
(`v0.5.2` was the first). The full history, **measured this session** with
`gh run list` against each tagged commit — not recalled:

| tag | asset (bytes) | CI at the tagged commit |
|---|---|---|
| `v0.1.0` | 9,590,137 | **red** |
| `v0.2.0` | 9,922,177 | **red** |
| `v0.3.0` | 9,998,582 | **red** — published with failing tests |
| `v0.4.0` | 10,023,075 | green |
| `v0.5.0` | 10,086,482 | **red** |
| `v0.5.1` | 10,244,728 | **red** |
| `v0.5.2` | 10,290,964 | green |
| `v0.5.3` | 10,308,179 | green |

**5 of 8, not the "3 of the last 4" the ledger has been carrying** (that
figure was true of its window and understates the whole). Mitigating
context already on record (`f2ac2af`, hundred-and-seventh filing): CI was
red on **every** push back past `v0.1.0` and nobody was watching, so the
three earliest reds share one cause. The encouraging half: **the last two
are green, and they are the only two ever checked by `verify-release.py`'s
CI gate** — the tool postdates the rest.

Keep the order: **FILE → LET CI GO GREEN → TAG**, and run
`tools/verify-release.py` *before* tagging.

**★ Half the releases have NO release record**, found while filing this
one. `v0.2.0`, `v0.3.0`, `v0.5.3` have dedicated entries; `v0.5.2` now does
(backfilled, `6d21fd1`); `v0.5.1` is recorded inside a work filing;
**`v0.1.0`, `v0.4.0` and `v0.5.0` have none of either kind.** Deliberately
NOT backfilled — filed as an open item with the measured table, for a
session with appetite for it, or for the operator to say the old ones are
not worth reconstructing. **The cause is `R192`'s exact shape:**
`check-commits-filed.py` counts *commits*, and each version-bump commit
*was* filed, so the gate was satisfied while the release record was
missing. Nothing watches for a release with no filing.

**Bump the version BEFORE the tag** (`--version` prints
`CARGO_PKG_VERSION`, so tagging a version the binary does not report ships a
false claim where a user checks it). Done here and verified: the copied
binary printed `pdfce-cli 0.5.3`.

Packaging smoke test run properly: staged folder **copied to a fresh path**,
both binaries launched from it, and the *new feature* exercised — CLI
authored both kinds and they were read back **through `dimension-list` from
the saved files** (`40.00 pt`, `30.0°`), and the GUI reached the `77.5°`
verdict.

**★ CORRECTION TO A CLAIM THIS FILE USED TO MAKE.** It said `fbcb946` added
a **lite-build job** and a **capability-presence job**, and warned to watch
them at release time. They are **not jobs** — they are **steps inside
`cargo test (${{ matrix.os }})`** (`.github/workflows/ci.yml` lines 91 and
105). Consequence worth keeping: they do **not** appear in `gh run view`'s
job list, so looking for them by name suggests they never ran, when in fact
they ran on **both** matrix legs and passed. This is the *same* defect §9
already names from the other direction — **a job whose name describes one of
the several unrelated gates it runs.** It has now produced a false alarm in
both directions, which strengthens the case for splitting it.

Scope of the standing authorisation is narrow: builds for his own testing.
**Not** blanket publishing authority.

## 8. Backup — re-measure, do not quote

Measured 2026-08-12 ~22:05:
**`D:\Dev\pdfce-backups\pdfce-20260812-2205.bundle`**, `git bundle verify`
okay, `refs/heads/main` = `0963b5f…` = `HEAD`, and **`refs/tags/v0.5.3` is
inside the bundle** (`b2d0595…`), so the release is recoverable from backup
alone.

**This ledger has carried a wrong backup figure twice. Re-run `ls -t` and
`git bundle list-heads` — including when the number above is this one.**

---

## 9. Carried forward, unchanged in substance

- **Strippable capabilities exist** (`fbcb946`, decision 055): **default ON**,
  **forward from EVERY shell**, **both CI gates**, **refuse by name when
  stripped**. Forgetting to forward does not break the build — it removes a
  capability, silently.
- **`tools/check-shipped-assets.py` gates every `crates/*/assets/` dir** —
  requires a `PROVENANCE.md` that **names a licence**, and (check 4) that
  the dir is cited in **`about.hbs`**. **`PROVENANCE.md` does not ship**;
  `about.hbs` is the `cargo-about` template, so it travels with releases.
  Own work is **exempt**. **This is where `(bl)` lands** — bundled
  CC-BY-SA-4.0 weights are third-party.
- **`R192` is PROPOSED, NOT MINTED** — *an obligation that falls between two
  correct tools is enforced by neither.* Third instance recorded; the
  engineer's ruling is owed. **`1f7ef59` may be a fourth instance** (a
  refusal correct for its original two variants, wrong for a third) — but
  that reads more like **R186**; decide which, or mint neither.
- **`Pass 67.0` phases C, D, F** unstarted, none blocking. C = re-subset
  (lowest risk). D = text to outlines (irreversible — disclose inline).
  F = replace font X with Y (Acrobat has **no equivalent**). **Ask which he
  wants rather than guessing.**
- **`/R` 6 encryption parked** at his instruction. ISO 32000-2 is at
  `D:\Dev\Rag-Specialized\PDF_Spec\_sources\ISO_32000-2_sponsored_EC3.pdf`
  — **SINGLE USER, watermarked to him by name: never committed, never
  shipped, paraphrase and cite only.** The repo is public.
- **★ The CI job's NAME does not name the gate — now demonstrated in BOTH
  directions, so this is worth actually fixing.** Every red X on one job
  renders as `verify pdfce-gui strings live in ui_text.rs`, though that job
  runs **three** unrelated gates. And `cargo test (${{ matrix.os }})` hosts
  the two **strippable-capability** gates (`ci.yml` lines 91, 105), so they
  are invisible in `gh run view`'s job list and a reader concludes they never
  ran. **A name that hides a failure is bad; a name that hides a success is
  how a real gate gets re-implemented by someone who thinks it is missing.**
  Rename or split — small and actionable.
- Two dead/stale printing items (Backlog, deliberately unfixed):
  `DeviceSettings::pick_tray_by_page_size` sets no `DEVMODE` field;
  `build_devmode`'s doc claims a driver-default start the code does not do.
- **Imposition has no GUI** — extract sheet composition into `pdfce-print`
  first so both shells share one implementation.
- Static hybrid XFA read/fill · wide-shape CSV · colour management
  (`D:\Dev\iccce\`, planned, no code).
- **Ledger-accuracy defect:** filings ninety-two through ninety-five cite
  `(bh)`/`(bi)` as if `(bi)` had not been minted.
- **Spec-librarian flag:** confirm the eight-item never-encrypted list
  (E1–E9) is in the §7.6 corpus, not only in pdfce's code.
- **`CLAUDE.md` rule 8's per-release wording is stale** against his
  2026-08-11 standing authorisation — **his file to amend, not ours.**

## Open operator questions

- **`(bl)` OPEN** — CC-BY-SA-4.0 model file inside an MIT portable folder?
  §2. *Default if unanswered: ship neither model set.* **Blocks OCR reaching
  an operator.**
- **`(bk)` ANSWERED** — do not raise again.
- Ceiling `(bl)`; **next free `(bm)`.**

---

## Tooling — corrections that cost time

- **★ `gui-shot.ps1` and `gui-drive.ps1` cannot share coordinates, for TWO
  independent reasons.** (a) different default window sizes (1760×1150 vs
  1600×1000) — pass matching `-W/-H`; (b) **`gui-shot` must run ON-SCREEN**
  — its blank-capture guard fires at the `-4000,-4000` position `gui-drive`
  uses. Then convert image pixels → egui client coords by subtracting the
  **31 px title bar** (measured, verified by a click landing at image y=441
  when sent as y=410).
- **The diag script separator is `;`, not `,`.** A comma-separated script
  parses as ONE unparseable step and is silently skipped
  (`script-step-UNPARSEABLE`). Click steps are `move:`/`down:`/`up:` —
  **there is no `click:`**. `tool:measure`, `tool:none` are in the vocabulary.
- **`PDFCE_DIAG_VIEWPORT`**, not `PDFCE_VIEWPORT`; four numbers `x,y,w,h`.
- **Both scripts move the REAL cursor.** Say so before running one while he
  is at the machine.
- **★ `LNK1201` on a debug link is not always disk-full.** Hit once here
  with **77 GB free**; a stale `deps/*.pdb` + `.exe` pair for the same hash
  was the cause — delete that pair and re-run. `target/` is **170 GB**;
  a `cargo clean` is overdue and would cost one full rebuild.
- **★ `git show <sha> -- <path> | grep` SEARCHES THE COMMIT MESSAGE TOO** —
  it prints the message above the restricted diff. **Use
  `git diff A^ A -- <path>`** when the question is what the diff contains.
- **`gh run list --commit <SHORT-SHA>` returns an EMPTY LIST, not an error.**
  Always pass a full 40-char SHA (`git rev-parse`).
- **Resolve every short hash yourself** — a fabricated full hash reached a
  filing once. **Librarians have no shell; paste real hashes.**
- **`git status --short` BEFORE `git commit -a`; a librarian filing gets its
  own commit.** Run `tools/check-commits-filed.py` at the **END** of a
  filing as well as the start.
- **A gate's DOCSTRING is not the gate** — verify by making the hazard
  occur, in **three** states (passing, genuinely-failing, correctly-exempt).
- **`cargo build --no-default-features` on ONE member proves NOTHING**
  (Cargo unifies features). Verify with a size diff or
  `cargo tree -p <crate> -i <dep>`; an **empty** result is how a lost
  capability looks.
- **A traced widget rect is the layout *request*, not what survived
  clipping.**

`tools/splice.py` · `tools/check-fmt-excluded.py` (no args; run **beside**
`cargo fmt --all --check`, never instead of) · `tools/check-shipped-assets.py`
· `tools/verify-release.py <tag>` — **before** tagging · 
`tools/check-commits-filed.py` — **file the commit; do NOT extend
`commits-filed-baseline.txt`** · `tools/check-ledger-numbers.py` ·
`tools/gen-embed-fixtures.py` / `tools/gen-unembed-fixtures.py` ·
`tools/package-portable.py --note "..."`.

**Live ceilings — re-run `check-ledger-numbers.py`, do not trust this line.**
At the hundred-and-twenty-eighth filing: rules **R191** → next free
**R192** (**claimed by an unruled PROPOSAL**) · decisions **055** → next
**056** · filings **128** → next **129** · Pass families to **71** → next
**`Pass 72`** · operator questions **(bl)** → next **(bm)**.
