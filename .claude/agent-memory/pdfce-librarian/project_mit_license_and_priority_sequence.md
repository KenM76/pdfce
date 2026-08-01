---
name: project-mit-license-and-priority-sequence
description: pdfce's OSS license is DECIDED (MIT, 2026-08-01) and the operator set a four-item work-priority sequence the same continuation — dimensioning tool (active) → GUI icons → finish text-handling → form-building tools.
metadata:
  type: project
---

**2026-08-01, SESSION_LOG continuation 50.** Two facts landed together
in one operator instruction:

1. **`LEGAL.md` §1 license decision: MIT.** Implemented same session —
   repo-root `LICENSE` (standard text, "Copyright (c) 2026 Ken
   Mantle"), `license = "MIT"` in `Cargo.toml` `[workspace.package]`,
   `license.workspace = true` on all four member crates. Dependency
   audit: 100% permissive, zero copyleft — MIT requires no dependency
   rework. Consequence: GPL/AGPL prior art (MuPDF, Poppler,
   Ghostscript) is now categorically, permanently excluded as a real
   dependency (was already the practical posture, now locked in).
   Project rule 8's license precondition is satisfied — but **this
   does NOT authorize pushing** the existing local commit (`d8b3903`)
   or publishing; that's a separate, still-open operator item.

2. **Four-item priority sequence** (verbatim: *"get the dimensioning
   tool completely functional in the gui interface. add
   d:/dev/scriptree style icons for all gui features. finish off all
   the text handling stuff. work on form building tools after if that
   makes sense."*):
   1. Dimensioning tool → completely functional in GUI. Promoted to
      **ACTIVE**. State at the time: only Pass 12.0 (canvas substrate,
      uninhabited) shipped; decision 011's remaining slices — 9a, 12.M1,
      12.M2, 9c-min — not built. Pass 9a dispatched to build.
   2. ScripTree-style SVG icons for all GUI features (styled after
      `D:\Dev\ScripTree\icons\*.svg`) — new, unscoped Backlog item,
      queued behind #1.
   3. Finish text-handling: FF-B, FF-H, FF-C all now schedulable
      (FF-C's license/rule-8 gate specifically lifted by item 1 above).
      **List-authoring is a SEPARATE, still-unanswered scope question**
      — this instruction does NOT resolve it; don't conflate the two.
   4. Form-building tools (field CREATION/authoring — distinct from the
      shipped Pass 7.0/7.1 fill/flatten subsystem) — queued last,
      operator's own hedge ("if that makes sense") noted verbatim.

**Why this matters:** this is the current top-level work-order for the
whole project as of 2026-08-01 — any future librarian dispatch
("what's next", "roadmap update", "pre-compaction capture") should
check `ROADMAP.md`'s "★★★ Operator priority sequence" block (top of
"Next up") for the live, authoritative version of this before assuming
anything from an older session's framing (e.g. the pre-2026-08-01
"text-parity arc awaits an operator decision" framing is now
superseded — the decision arrived).

**How to apply:** when asked to add a new Backlog/Pass entry or judge
sequencing, respect this four-item order unless the operator gives a
new explicit steer. Don't let a lower-priority item (icons, text-
handling, forms) get scheduled ahead of the dimensioning tool without
a fresh operator instruction to reorder. See also
[[project-loop-throttled-awaiting-steer]] (the steer this sequence
represents) and [[project-uncommitted-repo-worktree-risk]] (the
license-vs-push distinction this decision sharpened).
