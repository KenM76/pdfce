# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** the librarian's record —
`ROADMAP.md` says what shipped, this says what is in flight and what the
next hour should be. Overwrite it once acted on.

Written 2026-08-11 at `483cb4d`, branch `post-v0.2.0`.

---

## Verified state

Measured this session, not relayed:

- `cargo test --workspace` — **3,338 passing / 0 failing** (3,311 at the
  start of the session).
- `cargo clippy --workspace --all-targets --all-features -D warnings` — 0.
- `cargo fmt --check`, `check-ui-strings.sh`, `check-theme-colors.sh`,
  `check-ledger-numbers.py`, `check-passes-filed.py`,
  `check-bypass-paths.sh` — clean.
- `cargo tree -p pdfce-core` / `-p pdfce-render` name no GUI crate.
- Portable build **`D:\builds\pdfce-20260811-1116-483cb4d`**, smoke-tested
  by copying to a fresh folder and running both binaries there.

Filing gate: a librarian dispatch covering `74e54a5`, `5d2b19b`, `483cb4d`
was in flight when this was written. **Run `check-commits-filed.py` and
file whatever it names before new work.**

---

## What shipped

### Encryption increment 2 — AES-128 (`f7aee60`, `74e54a5`)

`/V 4` + `/CFM /AESV2` decrypts in core, CLI and GUI. An AES-128 file now
asks for a password (or opens silently when the user password is empty)
instead of reporting an unimplemented cipher.

New: `crates/pdfce-core/src/crypto/aes.rs`. `FileKey::object_key` needed
**no** change — increment 1 had already written the `sAlT` variant (T1)
while AES was still refused.

**Still refused, and the reason is not the cipher.** `/AESV3` keys off
**Algorithm 2.A**, not Algorithm 1, so having the block cipher bought
nothing there. `/R 6` stays blocked as unsourced past step (a).
Writing an encrypted document remains unimplemented in all three shells.

**Decision 039** — `aes 0.9.2` + `cbc 0.2.1`, all-permissive, and the
R24 exception: `aes` selects its intrinsic backends on a **cfg**
(`aes_backend = "soft"`), not a feature, so the project's usual
`default-features = false` lever does not exist. Forcing soft globally
would buy a guarantee true for pdfce's own builds and **false for anyone
consuming `pdfce-core` as a library**, because `.cargo/config.toml` is not
inherited. Hardware backend accepted; exception **bounded in CI**
(`hazmat`/`zeroize` pinned off on four targets). On wasm32 no
`cpufeatures` is pulled at all, so the web-fork target keeps zero-unsafe.

### Print dialog (`5d2b19b`, `483cb4d`)

Operator request. Tabs (Pages & Layout / Copies & Finishing / Comments &
Resolution), `min_size` + one `ScrollArea::both()`, variable-height
preview canvas, zoom/pan (Ctrl+wheel, drag, Fit/−/+/100%), Ctrl+P, and
the preview now **renders the page** instead of filling a flat rectangle.

Two bugs fixed that nobody had asked about:
- **`pending_print` was missing from `apply()`'s one-question gate** while
  the print window is centre-anchored. Reachable today via the ribbon
  alone — clicking Print over an open copy/save/redaction confirmation
  stacked a second centre-anchored window on an unanswerable one.
- **`spool_print` built render options without the operator's CMYK
  intent** while the canvas uses it, so a document proofed under
  `Calibrated` printed under `NeutralBlack`, silently. Found only by
  extracting the shared builder.

---

## ★ Start here: pick one

Nothing is half-finished. These are the live candidates, roughly ordered:

1. **Encryption increment 3 — AES-256 `/R` 5.** Sourced (Algorithm 2.A /
   3.2a: SHA-256 over password+salt, unwrap `UE`/`OE`, **key used as-is**,
   no per-object step). The block cipher already exists; what is new is
   the derivation and the three AES *modes* `/R 5` uses (**T25**:
   CBC+random-IV+padding for data, CBC+zero-IV+**no** padding for
   `UE`/`OE`, and **ECB — no IV at all** for `Perms`; ISO 32000-2's
   errata strike "with an initialization vector of zero" from Algorithms
   2.A(f), 10(f), 13(a)). `enc-aes-256-r5.pdf` is already a fixture.
   **`/R 6` stays blocked** and `enc-aes-256-r6.pdf` is a refusal fixture
   on purpose — deriving 2.B from another implementation and then testing
   against that implementation could not fail.
2. **Imposition has no GUI at all.** Extract sheet composition into
   `pdfce-print` FIRST so both shells share one implementation. The three
   modes are mutually exclusive and a GUI must express that as a *choice*.
3. **The print preview ignores the Orientation radio** (new, pre-existing,
   now filed). `pdfce_print::printer_caps` reports the device's default
   sheet and nothing rotates the previewed geometry. Confirmed this
   session: `plan_job` never reads `DeviceSettings`; orientation reaches
   the job only as `DEVMODE::dmOrientation` in `spool`.
4. **Escape-to-cancel is bound on none of the five gated dialogs.** Worth
   ONE decision covering all five rather than a fifth convention. Filed as
   an open operator question — **Ken's call, do not settle it solo.**
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

`D:\builds\pdfce-20260811-1116-483cb4d\pdfce-gui.exe`:

- **`enc-aes-128.pdf`** — prompts; `userpw` or `ownerpw` open it.
- **`enc-emptyuser.pdf`** — AES-128, opens with no prompt at all.
- **Ctrl+P** anywhere, or the ribbon Print button. Tabs across the top of
  the options column; drag the window smaller and both scrollbars appear;
  Ctrl+wheel over the preview zooms; drag pans.
- **`enc-aes-256-r5.pdf`** — still refused, **by cipher name**.

CLI: `pdfce-cli --open-password userpw <cmd> <file>`, or
`--open-password-file <path>` (`-` reads stdin).

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
