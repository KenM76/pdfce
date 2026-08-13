# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. Overwrite it once acted on.

**State at handoff (2026-08-13):** branch `main`, three code/doc commits
past `v0.5.3`, **not pushed, not tagged**. 3,667 tests, 0 failures. Every
gate clean. Nothing half-built.

---

## ⇢ ★ THE STANDING CONSTRAINT — READ THIS BEFORE PLANNING ANYTHING

**The operator paused ALL GUI work on 2026-08-13**, verbatim:

> *"continue the planned work except for gui related, don't do any more
> work on the gui until I say so."*

He gave no reason and was not asked for one. **Do not infer one, and do not
record an inferred one as fact.** It is unambiguous as it stands.

- Paused: `crates/pdfce-gui/`, `tools/gui-drive.ps1`, `tools/gui-shot.ps1`.
- Not paused: core, render, CLI, print, docs, RAGs, tests, fuzz, tooling.
- A Pass whose GUI half is deferred ships `core [x] · cli [x] · gui [ ]`
  and the ROADMAP entry records it as an **operator instruction, not an
  engineering shortfall**. `Pass 69.0`/`69.1` are the worked precedent.
- **If he asks for GUI work, he has lifted it.** Do not quote the pause
  back at him.

---

## ⇢ IF THE OPERATOR JUST SAID "CONTINUE"

**Ask `(bl)` again — it has now gone four sessions unasked-then-asked and
is still the only thing blocking a whole feature from reaching him.** Then
take §2 (`Pass 67.0` phase C) without waiting, since it needs nothing from
him.

---

## What just shipped, so you do not redo it

`Pass 69.0` **and** `Pass 69.1` — the ce-dimension **style cascade** and
**tolerance** — both **core + CLI complete, GUI deferred**.

| commit | what |
|---|---|
| `d5431a4` | `Pass 69.0` — `dimension::style`, the three-tier per-property cascade + `group-style` / `dimension-style` / `dimension-list --style` |
| `be41d75` | the hundred-and-thirty-fourth filing (decision **056**) |
| `c057682` | `Pass 69.1` — `dimension::tolerance`, seven notation types, as the cascade's tenth property |

The model, in one box:

```text
factory (StyleDefaults::FACTORY) -> group (Group::style)
    -> ce dimension (DimensionRecord::style)
```

**Eleven properties, each an independent `Option` — that `Option` IS the
operator's requested checkbox.** `resolve_style()` is the single resolution
point; `style_provenance()` answers *which tier supplied this*, and
`StyleSource::follows_group()` answers the question a panel actually asks
(*will a group edit move this?*) — **`true` for `Factory` as well as
`Group`**, which is the easy thing to get wrong when deriving it by hand.

### Three things worth carrying, all about tests

1. **★ An ABSENCE assertion on PDF bytes is vacuous under an incremental
   save**, because the superseded object is still in the file. Both
   load-bearing tests here save with `--mode full` for that reason. Filed
   as a lesson in `C:\personal_rag\pdf\`.
2. **★ It COMPOSES with the `Pass 68.0` octal-escaping lesson, and the pair
   met for real here.** The `±` sign is the **second** non-ASCII character
   this writer has ever emitted; the first shipped broken. Get the encoding
   right but the save mode wrong, or the reverse, and the test still cannot
   fail. Both halves, every time.
3. **The sabotage check was run**: the appearance test was **seen to fail**
   (cascade replaced by `From<&Group>`) before being trusted. A cascade
   that resolves correctly in memory and is discarded on the way to the
   document has exactly one symptom — it works in the panel and vanishes in
   the file — and only a test that reads the **baked appearance** sees it.

### One trap the GUI will hit when it is un-paused

**`EditSession::set_group_style` returns the number REGENERATED, not the
number that will visibly MOVE.** Those differ whenever a member overrides
the edited property. The operator's *"cannot change one and be surprised 40
others changed or didn't"* is asking for the second number, and it must be
computed (via `style_provenance` per member) **before** the edit is applied
if it is to be disclosed before the edit is applied. Written up in
`docs/ui_specs/tool-options-dock-and-ce-dimension-properties.md`
**Amendment B**, which also records what else the GUI owes.

---

## 1. ★★ `(bl)` — STILL UNANSWERED, STILL BLOCKING OCR

**Nothing here is an engineering blocker. Raise it; do not resolve it.**

May a **CC-BY-SA-4.0 model file** ship inside pdfce's **MIT** portable
folder?

| candidate | wasm32 | languages | weights |
|---|---|---|---|
| `ocrs`/`rten` | **the ONLY passing route** | Latin only | **CC-BY-SA-4.0** |
| `ocr-rs` / PaddleOCR | none | **50+** | Apache-2.0 |

*"Copyleft weights and a web future"* versus *"permissive weights, more
languages, no web future"* — **not** *"one is clean."* Do **not** resolve it
by picking the permissive engine and calling it moot: that forfeits the
WASM route, an `ARCHITECTURE.md` §3 commitment. *Default if unanswered:
ship neither model set.* Engine question already answered (**both**, behind
Cargo features, `fbcb946`). Sourcing: `docs/ocr-engine-survey.md` — **Surya
is a recorded trap** (Open RAIL-M, $5 M revenue cap); **Tesseract's default
Windows build ships LGPL binaries**.

**Also still owed:** confirm the model-**downloader** withdrawal
(`af5580e`) — he agreed on a wrong estimate, so the agreement was
uninformed.

## 2. `Pass 67.0` phase C — re-subset fonts — the best non-GUI next move

Lowest-risk of the three remaining phases, needs nothing from the operator,
entirely core + CLI. D (text to outlines) is irreversible and needs an
inline disclosure; F (replace font X with Y) has **no Acrobat equivalent**,
so its acceptance criteria are a design question, not a parity one. **Ask
which he wants rather than guessing** — but C is the one to start if he
does not answer.

## 3. Other non-GUI work available

- **Imposition has no GUI** — but the right first step is core anyway:
  extract sheet composition into `pdfce-print` so both shells can share one
  implementation. Unblocked by the pause.
- **`v0.1.0` / `v0.4.0` / `v0.5.0` have no release record.** Measured
  figures are already filed in `ROADMAP.md`'s hundred-and-thirty-third
  filing; backfilling is cheap and needs no operator input. Cause is
  `R192`'s exact shape: `check-commits-filed.py` counts *commits*, each
  version-bump commit *was* filed, so nothing watches for a release with no
  filing.
- **`R192` is PROPOSED, NOT MINTED** — *an obligation that falls between
  two correct tools is enforced by neither.* The engineer's ruling is owed.
- **Two dead/stale printing items** (Backlog, deliberately unfixed):
  `DeviceSettings::pick_tray_by_page_size` sets no `DEVMODE` field;
  `build_devmode`'s doc claims a driver-default start the code does not do.

## 4. Deferred BY THE PAUSE — not forgotten, not startable

- The GUI half of `Pass 69.0` + `69.1` (group-tier controls, the
  per-ce-dimension section, the follows-group disclosure).
- `Pass 46` slices 2–4 — post-hoc select/move/resize a placed annotation.
  Gates click-a-comment-to-select and canvas selection of ce dimensions.
- The GUI attachments surface (core + CLI finished in `95c3416`).

## 5. ★ TWO ESCALATIONS STILL AWAITING THE OPERATOR — raise, don't resolve

1. **The broken no-git convention** (`iccce`).
2. **Agents' in-progress files swept into a public repo.**

Carried across five filings now with **no supporting detail supplied**.
**Recorded so a compaction does not lose them — not as established
findings.** Get the actual statement. `af5580e` remains the one measured
instance of (2). `D:\Dev\iccce\` **does** contain a `.git` directory, so
whatever (1) is, it is not "that project has no repository."

The convention it bought is holding: every commit this session was staged
**by name**, `git status --short` run first, and
`tools/render-profile/Cargo.lock` — dirty since before the session — was
left alone in all three.

---

## 6. Release state

**`v0.5.3` is the current tag and is clean** (all seven `verify-release.py`
checks, CI green at the tagged commit). Three commits sit past it,
**unpushed and untagged**. Nothing about `Pass 69.0`/`69.1` requires a
release; the operator's standing authorisation covers **builds for his own
testing**, **not** publishing.

Keep the order: **FILE → LET CI GO GREEN → TAG**, run
`tools/verify-release.py` *before* tagging, and **bump the version before
the tag** (`--version` prints `CARGO_PKG_VERSION`, so tagging a version the
binary does not report ships a false claim where a user checks it).

## 7. Backup — re-measure, do not quote

Last measured 2026-08-12 ~22:05:
`D:\Dev\pdfce-backups\pdfce-20260812-2205.bundle`. **That predates this
session's three commits** — a fresh bundle is owed. This ledger has carried
a wrong backup figure twice; re-run `ls -t` and `git bundle list-heads`,
including when the number above is this one.

---

## Tooling — corrections that cost time

- **★ NEW: a quoted heredoc (`<<'PYEOF'`) through the Bash tool failed
  twice on large Rust/Python payloads**, with `unexpected EOF while looking
  for matching '`. Same content written via the Write tool and run as a
  script worked every time. **For any multi-line patch, write a file and
  run it** — do not fight the heredoc.
- **★ NEW: a `str.replace()` in a patch script with no `assert` is a
  silent no-op.** One tolerance listing line went missing exactly that way
  after `cargo fmt` had reflowed the anchor text. **Assert every anchor.**
- **`cargo fuzz` on this machine needs the MSVC ASan DLL on PATH** or the
  binary dies with `STATUS_DLL_NOT_FOUND`. The path has spaces — set it
  **literally**, do not build it from `find` output (word-splitting turns it
  into `/c`). Filed in `D:\dev\rag\rust\`.
- **`gui-shot.ps1` and `gui-drive.ps1` cannot share coordinates** — two
  independent reasons (different default window sizes; `gui-shot` must run
  on-screen). Moot while the GUI is paused; do not delete the note.
- **The diag script separator is `;`, not `,`.** Click steps are
  `move:`/`down:`/`up:` — there is no `click:`.
- **`git show <sha> -- <path> | grep` SEARCHES THE COMMIT MESSAGE TOO.**
  Use `git diff A^ A -- <path>`.
- **`gh run list --commit <SHORT-SHA>` returns an EMPTY LIST, not an
  error.** Always pass a full 40-char SHA.
- **Resolve every short hash yourself** — librarians have no shell in some
  dispatches; paste real hashes.
- **A gate's DOCSTRING is not the gate** — verify by making the hazard
  occur, in three states (passing, genuinely-failing, correctly-exempt).
- **The CI job's NAME does not name the gate**, demonstrated in both
  directions. Rename or split — small and actionable, not GUI work.

`tools/splice.py` · `tools/check-fmt-excluded.py` (run **beside**
`cargo fmt --all --check`) · `tools/check-shipped-assets.py` ·
`tools/verify-release.py <tag>` — **before** tagging ·
`tools/check-commits-filed.py` — **file the commit; do NOT extend the
baseline** · `tools/check-ledger-numbers.py` · `tools/gen-embed-fixtures.py`
/ `tools/gen-unembed-fixtures.py` · `tools/package-portable.py --note "..."`.

**Live ceilings — re-run `check-ledger-numbers.py`, do not trust this
line.** After the hundred-and-thirty-fourth filing: rules **R191** → next
free **R192** (**claimed by an unruled PROPOSAL**) · decisions **056** →
next **057** · filings **134** → next **135** (the hundred-and-thirty-fifth
was in flight as this was written — verify) · Pass families to **71** →
next **`Pass 72`** · operator questions **(bl)** → next **(bm)**.
