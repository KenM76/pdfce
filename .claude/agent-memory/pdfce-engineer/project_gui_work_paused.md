---
name: gui-work-paused
description: 2026-08-13 — Ken paused ALL pdfce-gui work until he says otherwise; core/CLI/docs continue normally
metadata:
  type: project
---

**2026-08-13, verbatim:** *"continue the planned work except for gui
related, don't do any more work on the gui until I say so."*

**Why:** not stated. He gave no reason and none was asked for — the
instruction is unambiguous on its own and a reason is not needed to obey
it. Do **not** infer one (an inferred constraint recorded as a fact is
the exact mistake the global CLAUDE.md's subagent rule was written after).

**How to apply:**
- `pdfce-core`, `pdfce-render`, `pdfce-cli`, `docs/`, RAGs, tests, fuzz,
  tooling — all continue normally. Only `crates/pdfce-gui/` and the
  GUI-driving harnesses (`tools/gui-drive.ps1`, `tools/gui-shot.ps1`) are
  paused.
- A Pass whose GUI half is deferred by this instruction ships as
  `core [x] · cli [x] · gui [ ]` in `docs/FEATURES.md`, with the
  instruction recorded in the ROADMAP entry as an **operator instruction**,
  not as an engineering shortfall. `Pass 69.0` and `Pass 69.1` are the
  worked precedent (2026-08-13).
- The pause is **not** a licence to stop designing for the GUI: write the
  core API so the eventual panel has the data it needs (see
  `StyleProvenance` / `StyleSource::follows_group`, built precisely so a
  future panel renders inheritance rather than recomputing it), and record
  what the GUI owes in the relevant `docs/ui_specs/` file.
- **This memory expires the moment he lifts it.** Check for a newer
  instruction before treating it as current; if he asks for GUI work, he
  has lifted it — do not quote this file back at him.

Related: [[launch-on-completion]] is partially suspended for the duration —
a GUI window cannot be launched for a Pass with no GUI half. Launch the CLI
demonstration instead, which is what was done for `Pass 69.0`/`69.1`.
