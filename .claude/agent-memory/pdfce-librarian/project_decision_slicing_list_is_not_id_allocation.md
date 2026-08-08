---
name: project-decision-slicing-list-is-not-id-allocation
description: 2026-08-08 — two Pass-ID collisions in one session (Pass 47.5→47.11, Pass 47.7→48.3), both minted from memory of decision 033 §6's slicing list rather than the live ceiling checker; exposed a 3rd, distinct check-passes-filed.py blind spot; R106 got a fourth amendment.
metadata:
  type: project
---

**What happened (2026-08-08, thirty-fifth and thirty-sixth filings).**
`docs/decisions/033`'s §6 laid out a P0/P1/P2 work-breakdown list for the
GUI-usability Pass family (47). Twice in one session, the dispatching
engineer minted a Pass sub-ID by reading a position in that list from
memory rather than re-running `tools/check-ledger-numbers.py`/
`check-passes-filed.py`. Both times the position was already claimed by
something else filed by an intervening session:

- `Pass 47.5` minted for a mid-session text-edit retarget bug fix
  (commit `5ceb5f8`) — `47.5` was already right-click context menus.
  Corrected to `Pass 47.11`.
- `Pass 47.7` minted for the GUI image-drop gesture (commit `e6ad48c`) —
  `47.7` was already the contextual ribbon tab. Corrected to `Pass 48.3`
  (which was, separately, already the correct reserved ID for this exact
  capability, filed by the thirty-fifth filing's own Backlog entry).

**Why:** the engineer's own account named the mechanism both times — the
decision record's numbered slicing list was read as though it allocated
IDs. It never does; it only proposes scope and rough order. See
`D:\dev\rag\rust\a_decision_records_proposed_slicing_is_a_proposal_not_an_id_allocation.md`
for the full derivation and `D:\Dev\pdfce\docs\ROADMAP.md`'s `R106`
(Standing rules), fourth amendment, for the project-side record.

**Secondary finding, from the second collision specifically:** because
`e6ad48c`'s wrong subject line (`Pass 47.7: …`) is now permanent (commit
unpushed but citations to its hash already exist in prior filings, so
rewriting was rejected — see `ROADMAP.md`'s `Pass 48.3` Shipped entry),
this exposed a THIRD, distinct blind spot in `tools/check-
passes-filed.py`: its join key is hash-presence in `ROADMAP.md`, never a
comparison of the commit's subject-claimed ID against the ID of the entry
the hash is filed under. Not fully silent — a real future `Pass 47.7`
commit would trigger the checker's existing collision-note mechanism —
but the note cannot distinguish "legitimate multi-commit Pass" from
"stale mistaken subject line," per the checker's own documented
limitation. **Left as an open engineer decision, not resolved by this
filing**: whether to add a docstring `KNOWN WEAKNESS` bullet, extend the
collision note to diff subject text, or accept the residual risk.

**Why this is worth a project memory, not just the RAG file:** the RAG
file is the generalizable engineering finding (any project with a
decision-record-driven numbering scheme). This memory is the pdfce-
specific state: `R106` now has a fourth amendment, the Pass-ID collision
count is six (enumerable: 13.x, 18.4/18.2, 19.4/18.7, 24.0a/24-family,
47.5/47.11, 47.7/48.3), and the broader "numbering collisions" tally
(last stated as six, 2026-08-04) was deliberately left unrecomputed
pending a full re-audit — do not assume it is "eight" without doing that
audit; it may be higher if any other, non-Pass-ID collision also went
untallied in the same window.

**How to apply:** before minting any Pass/rule/decision number sourced
from a decision record's own proposed list (not just from a remembered
ceiling), run the live checker anyway — the record's list is never
sufficient evidence of what is free, no matter how carefully re-read.
