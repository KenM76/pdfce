---
name: gui-work-paused
description: 2026-08-13 — pdfce-gui is paused because it was unusable; a replacement is being built at D:\dev\pdfceGUI in a separate session and may replace/merge into this repo
metadata:
  type: project
---

**The instruction, 2026-08-13, verbatim:** *"continue the planned work except
for gui related, don't do any more work on the gui until I say so."*

**The reason, given later the same day, verbatim:** *"I paused GUI production
in this branch because it was unusable and I realised it needed a separate
project plan rather than the current method which just seems to be low
priority and a patchwork things stuck together as they are added. The new one
is being built in d:\dev\pdfceGUI in another session and if successful will
likely replace the current one and may have its dev folder merged into this
one."*

**★ This file previously asserted "he gave no reason and none was asked for —
do not infer one." That was true when written and is now wrong.** Keeping the
correction visible rather than silently overwriting it, because the shape
matters: an absence recorded as a *property of the operator's instruction*
("there is no reason") reads identically to an absence that was simply not
volunteered yet. The safe form is *"no reason has been given"*, which invites
the update; *"he gave no reason"* quietly forecloses it.

**What the reason changes, engineering-wise:**

1. **`crates/pdfce-gui` may be REPLACED WHOLESALE.** Do not invest in it. Any
   refactor, polish or new panel there is potentially throwaway, and worse,
   raises the cost of the swap. The critique is explicitly about *method* —
   patchwork accreted feature-by-feature at low priority — so adding one more
   well-built panel does not answer it.
2. **The GUI-core separation invariant is now being TESTED FOR REAL, by
   someone else.** `D:\dev\pdfceGUI` consuming `pdfce-core`/`pdfce-render`
   from outside this repo is exactly the scenario §3's invariant was written
   for (the "fork to a web app later" goal). If that project needs nothing to
   move in core, the separation is real. **Anything it does need is a place
   the boundary was drawn wrong** — treat such a request as a finding about
   this repo, not as an accommodation.
3. **`pdfce-core`'s public API is now a consumed boundary with a real external
   consumer**, not a hypothetical one. Rust API Guidelines compliance and
   doc-comment completeness stopped being hygiene and became someone else's
   unblocking. Where a core verb has a trap (e.g. `set_group_style` returns
   the count REGENERATED, not the count that will visibly MOVE), that trap is
   now reachable by a session that cannot ask me.
4. **A merge of `D:\dev\pdfceGUI` into this repo is possible.** Keep the
   workspace layout mergeable; do not spread GUI assumptions into core crates.
5. **`gui [ ]` rows in `FEATURES.md` are MORE accurate now, not less** — and
   will need re-basing against the new shell if it lands. Do not pre-tick
   anything in anticipation.

**How to apply:** core / render / CLI / print / docs / RAGs / tests / fuzz /
tooling all continue normally. `crates/pdfce-gui/`, `tools/gui-drive.ps1` and
`tools/gui-shot.ps1` stay untouched. A Pass whose GUI half is deferred ships
`core [x] · cli [x] · gui [ ]` with the instruction recorded as an **operator
instruction, not an engineering shortfall** — `Pass 69.0`/`69.1` are the
worked precedent.

**This expires when he lifts it**, and asking for GUI work IS lifting it. Do
not quote this file back at him. Note also that *"keep going"* (2026-08-13,
answering the OCR licence question) did **not** lift it — a different sentence
answering a different question.

Related: [[launch-on-completion]] is partially suspended — a GUI window cannot
be launched for a Pass with no GUI half; launch the CLI demonstration instead.
