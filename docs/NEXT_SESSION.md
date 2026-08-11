# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** the librarian's record —
`ROADMAP.md` says what shipped, this says what is in flight and what the
next hour should be. Overwrite it once acted on.

Written 2026-08-11 at `cfc20dd`, branch **`main`**.

---

## ★ Read this first: the branch moved and a release went out

**`v0.3.0` is tagged and released** at `cfc20dd`, 35 commits after `v0.2.0`.
Operator gave the explicit go-ahead (project rule 8 requires one per
release, and it is not standing).

**You are on `main`, not `post-v0.2.0`.** Those two refs are identical at
`cfc20dd`; `main` was fast-forwarded and pushed, and `origin/main` is AT
the tagged commit. Do not resume work on `post-v0.2.0` — it is a stale
duplicate name for the same commit, and the next branch should be cut
from `main`.

`tools/verify-release.py v0.3.0` passes all six checks. This was its
**first real outing** — it was written after the `v0.2.0` near-miss, where
a tag pointed at the right commit while `git push origin main` pushed a
local `main` 36 commits behind and *reported success*. The check that
would have caught that (`origin/main` is AT the tagged commit) is the one
that matters; run the script before believing any future release.

**Open with the operator, not resolved:** he reported *"my mouse
navigation is getting screwy"* right after a session that drove
`tools/gui-shot.ps1` about eight times. That harness moves the REAL
cursor and synthesises Ctrl+scroll and click-drag. No harness process was
running and nothing was held when checked, and all buttons/modifiers were
defensively released — but the cause was never confirmed. **Treat the GUI
harness as an input-hijacking tool and say so before running it while the
operator is at the machine.**

---

## Verified state

Measured this session, not relayed:

- `cargo test --workspace` — **3,353 passing / 0 failing** (3,311 at the
  start of the session).
- `cargo clippy --workspace --all-targets --all-features -D warnings` — 0.
- `cargo fmt --check`, `check-ui-strings.sh`, `check-theme-colors.sh`,
  `check-ledger-numbers.py`, `check-passes-filed.py`,
  `check-bypass-paths.sh` — clean.
- `cargo tree -p pdfce-core` / `-p pdfce-render` name no GUI crate.
- Portable build **`D:\builds\pdfce-20260811-1322-cfc20dd`** — this is the
  RELEASED build; the same bytes are the `v0.3.0` asset. Smoke-tested
  by copying to a fresh folder and running both binaries there — the AES
  render matched `bc2dfede94ef290e7c7a7f7e509fea98` from the packaged
  binary, and `print-preview` reported `auto` at `0.9725` against
  `portrait` at `0.7515`.

## ★ CI IS GREEN — for the first time in the project's history

All **10 jobs pass** at `10e9b0c`. Before 2026-08-11 CI had failed on
**every push ever made**, including both prior releases, and nobody was
looking. Two independent causes, both fixed today:

1. **`pdfce-print` did not compile for any non-Windows target**
   (`Pass 66.0`). A `#[cfg(windows)]` on a plain-data error type that the
   file's own non-Windows stubs returned. `cargo tree` proved no GUI crate
   was linked; nothing proved the crate *built* — the GUI-separation
   invariant failing quietly one crate over.
2. **`check-commits-filed.py` was reading a one-commit repository.**
   `actions/checkout` defaults to `fetch-depth: 1`, and a shallow boundary
   commit has no parent, so git reports it as adding every file — making
   docs-only filing commits look like unfiled code. It printed a
   confident, specific, **wrong** list for as long as that job existed.

**Two things to know before you touch this.** The filing check runs *in*
CI, so **every CODE commit leaves CI red until its filing lands** — that
is by design here, not a defect, and a session that pushes code and stops
will always leave a red build behind. And `check-passes-filed.py` has the
identical latent shallow-clone flaw; it is not run in CI today, so the
risk is theoretical, and it is filed in Backlog rather than fixed.

**Check CI after pushing.** `gh run list --limit 1`. Green local gates
stood in for a green build for the entire project's history.

Filing gate: `check-commits-filed.py` is **clean** (5 known-unfiled carried
in the baseline as pre-existing debt). Everything this session is filed.

**`check-ledger-numbers.py` was itself wrong until `e293143` and is worth
re-reading.** It printed `clean` while reporting two ceilings that were
false: decision numbers counted only `docs/decisions/*.md` files (missing
034-036, 039, 040, which live only in ARCHITECTURE §12), and its ordinal
vocabulary stopped at "ninety" on the very day `SESSION_LOG.md` reached
its hundredth filing. **Run it; do not infer the ceilings from anywhere
else.** As of `cfc20dd` it reads: Pass **65.0**, decisions **042 → next
free 043**, rules **R186 → next free R187**, filings **104 → next free
105**, questions next free **(bk)**.

---

## What shipped

Four pieces, all verified in a running build rather than by compiling.

### Encryption increment 2 — AES-128 (`f7aee60`, `74e54a5`)

`/CFM /AESV2` decrypts in core, CLI and GUI. `FileKey::object_key` needed
no change — increment 1 had already written the `sAlT` variant (T1).
**Decision 039** records the `aes`/`cbc` dependency and the R24 exception
(the backend is cfg-selected, so the usual `default-features = false`
lever does not exist; bounded in CI instead).

`74e54a5` closed a hole found by asking what the fixtures *cannot* fail
on: every `enc-*.pdf` has zero object streams, and pypdf flattens them on
clone, so the commonest real-world AES shape was untested. Covered with
PDFium's `encrypted.pdf` (a third independent implementation).

**Still refused:** `/AESV3` keys off Algorithm 2.A, not Algorithm 1 — the
block cipher bought nothing there. `/R 6` stays unsourced. Writing an
encrypted document is unimplemented in all three shells.

### Print dialog — `Pass 63.0` (`5d2b19b`, `483cb4d`)

Tabs, `min_size` + one `ScrollArea::both()`, variable-height preview,
zoom/pan, Ctrl+P, and a preview that renders the page instead of a flat
rectangle. Two bugs fixed that nobody asked about: `pending_print` was
missing from `apply()`'s one-question gate (reachable via the ribbon
alone), and `spool_print` built render options **without** the operator's
CMYK intent.

### Landscape orientation — `Pass 64.0` (`d1756e5`, `290aef9`, `4837009`)

**Orientation turned the paper but not the placement.** `printer_caps` is
read before any DEVMODE exists, `plan_job` never saw orientation, and
`build_devmode` then told the driver to turn the sheet. A landscape page
printed at ~77% of size.

★ **The diagnosis that reached this file first was half wrong, and the
correction is the better finding.** It did NOT fire at pure defaults:
`build_devmode` returned `None` when `settings == default`, so no DEVMODE
was sent and planner and driver agreed *by accident*. The real shape:
the mismatch fired whenever **any** setting differed from default —
changing duplex alone was enough — and, worse, because `Auto` **is** the
default, **auto-orientation never turned anything**. A "disturb nothing by
default" guard had disabled the behaviour it was guarding.

`From<&PrinterCaps> for DeviceGeometry` was **deleted**: an infallible
conversion that silently gives the wrong answer for a landscape job gets
reached again. `DeviceGeometry::from_caps(caps, requested, first_page_pt)`
is now the only route, so the un-turned view is unreachable.
**Decision 041**; **R171 widened in place** (third instance in
`print_flow.rs` alone of two copies of one derivation drifting).

Measured in the packaged CLI: portrait `0.7515`, landscape `0.9725`,
**auto `0.9725`** — auto matching landscape is the inert-Auto bug closed.

### Escape cancels every dialog — `Pass 65.0` (`4ddd6c4`)

Operator question **(bj)**, answered by Ken: *"escape should work like it
does for any other program."* Escape was bound on **none** of the five
confirmation dialogs and fell through to the canvas ladder, so it acted on
the document *underneath* the question. One new top rung, above the
password prompt AND above view mode — the latter because read mode and
full screen hide the ribbon, so a view-mode win would drop the operator
out of full screen and leave the question sitting there. Every arm returns
a **Cancel**, never a Confirm (redaction's confirmed branch is the only
irreversible operation pdfce has). **Decision 042.**

Two things the tests found, both recorded because they are the reusable
part. My own doc comment claimed the resolver's match order was
load-bearing; measured, it is not — the five are mutually exclusive, so
the tiebreak is never reached, and the comment was corrected in place
rather than left standing. And a **latent deadlock**: with two questions
somehow set, *neither* can be answered, because `apply()`'s gate checks
run in sequence and each dialog's Cancel is dropped by the other's gate.
Unreachable today only because every pending state is set from inside
`apply()`. That invariant is now held by
`at_most_one_confirmation_question_is_ever_up`, driven through the real
`CloseDocument` path — **a test, not a comment.**

### The ledger gate (`e293143`) — see "Verified state" above.

---

## ★ Start here: pick one

1. **Encryption increment 3 — AES-256 `/R` 5.** Sourced (Algorithm 2.A /
   3.2a: SHA-256 over password+salt, unwrap `UE`/`OE`, **key used as-is**,
   no per-object step). The block cipher exists; new is the derivation and
   the three AES *modes* `/R 5` uses (**T25**: CBC+random-IV+padding for
   data, CBC+zero-IV+**no** padding for `UE`/`OE`, **ECB — no IV at all**
   for `Perms`). `enc-aes-256-r5.pdf` is already a fixture. **`/R 6` stays
   blocked**; `enc-aes-256-r6.pdf` is a refusal fixture on purpose.
2. **Two dead/stale printing items found and deliberately not fixed**
   (both filed to Backlog): `DeviceSettings::pick_tray_by_page_size` sets
   no `DEVMODE` field at all — `DM_DEFAULTSOURCE` is never written, so the
   control does nothing; and `build_devmode`'s doc claims it "starts from
   the driver's own default rather than zeroed" while the code builds a
   zeroed `DEVMODEW` and leaves `_printer_wide` unused.
3. **Imposition has no GUI.** Extract sheet composition into `pdfce-print`
   FIRST so both shells share one implementation.
4. **No open operator questions.** `(bj)` was answered and closed
   2026-08-11; next free is `(bk)`.
5. Static hybrid XFA read/fill · wide-shape CSV · colour management
   (`D:\Dev\iccce\`, planned, no code).
6. **Ledger-accuracy defect** (librarian-reported, not fixed): filings
   ninety-two through ninety-five cite `(bh)`/`(bi)` as if `(bi)` had not
   been minted.
7. **Spec-librarian flag**: confirm the eight-item never-encrypted list
   (E1–E9) is in the §7.6 corpus rather than only in pdfce's code.

---

## Live decisions worth not re-litigating

- **`R186` — now FIVE instances**, and the newest is the sharpest.
  Increment 1 guarded the in-buffer write-back on
  `plain.len() == span.len` and asserted "RC4 must preserve length". True
  then. AES plaintext is **strictly shorter**, so the equality is *always*
  false, the copy *always* skipped, and every stream in an AES document
  would silently stay ciphertext. Nothing failed; no test went red.
- **A limit set before there is a case to argue is worth more than one set
  during the argument.** `crypto/md5.rs` recorded in increment 1 why MD5
  and RC4 were hand-rolled AND that the reasoning "does not extend to
  AES". Increment 2 honoured it without re-opening it.
- **`DocError::PasswordRequired` is not a capability gap.**
- **A derived value with one producer cannot drift** (`149fd03`).
- **Decision 037** — `/BaseState /OFF` applies to registered groups only.
- **Decision 038** — cite **both** loci; `Table 101` is 1.7-only
  (ISO 32000-2 renumbers it to **Table 99**).

---

## Tooling — three corrections that cost time this session

- **`PDFCE_DIAG_VIEWPORT`**, not `PDFCE_VIEWPORT`. Four comma-separated
  numbers: `x,y,w,h`.
- **The diag script separator is `;`, not `,`.** A comma-separated script
  parses as ONE unparseable step, is silently skipped, and the run then
  looks like a *feature* failure. The trace says
  `script-step-UNPARSEABLE` — read it.
- **`gui-shot.ps1` and `gui-drive.ps1` default to different window
  sizes.** Read the trace's own `rect=`, never a screenshot's pixels.

`tools/splice.py` — anchored substitution, all-or-nothing.
`tools/verify-release.py <tag>` · `tools/gen-encryption-fixtures.py`
(no arguments needed) · `tools/package-portable.py --note "..."`.

---

## What the operator can try

`D:\builds\pdfce-20260811-1322-cfc20dd\pdfce-gui.exe` — or the released
`v0.3.0` asset, which is the same bytes:

- **`enc-aes-128.pdf`** — prompts; `userpw` or `ownerpw` open it.
- **`enc-emptyuser.pdf`** — AES-128, opens with **no prompt at all**, and
  the status bar says why. Save is greyed out with its reason stated at
  OPEN, not sprung at Ctrl+S.
- **Ctrl+P**, or the ribbon Print button. Tabs; drag the window smaller
  and both scrollbars appear; Ctrl+wheel over the preview zooms; drag
  pans; **the sheet now turns with the Orientation radio.**
- **`enc-aes-256-r5.pdf`** — still refused, **by cipher name**.
- **Escape** now closes any confirmation dialog — print, close, copy,
  save-conflict, redaction-apply — and always takes the safe branch.

CLI: `pdfce-cli --open-password userpw <cmd> <file>`;
`print-preview --orientation portrait|landscape|auto <file>` reports the
turned sheet and the scale the job would use.

---

## The habit worth carrying

Unchanged, and it paid four times today: **prove a guard by making it
fail.** The old length guard was reinstated (render still exited 0 and
wrote a plausible PNG — only a byte comparison caught it); the ObjStm
test's password was changed to `wrong`; `hazmat` was temporarily enabled
to see the new CI gate fire; the `pending_print` gate was deleted to see
its test go red. A test that has never been seen to fail is a test nobody
has tested.

**And the newest one, which is about *searching* rather than testing.**
I reported `UI_PREFERENCES.md` as never having existed, from
`ls docs/UI_PREFERENCES.md` and `git log --all -- docs/UI_PREFERENCES.md`.
It exists — **at the repo root**. `--all` reads as exhaustive (all
branches, all refs) but is still **path-scoped**, so a wrong path returns
silence indistinguishable from a true negative. `pdfce-ui-specialist` hit
the identical blind spot the same day by globbing. **A negative result
from a path-scoped query is a fact about the path, not the repository** —
use `git ls-files | grep`, `git log --all -- '*name*'`, or `find -iname`
before calling anything absent.

A near-miss from the same family, caught only by comparing rather than
concluding: the packaging smoke test appeared to show a stale 2.7 MB CLI
with no `--open-password` flag. The package was fine; `ls …-smoke-* |
head -1` had picked up a **leftover folder from a previous session**.
One `stat` separated "product regression" from "my glob".
