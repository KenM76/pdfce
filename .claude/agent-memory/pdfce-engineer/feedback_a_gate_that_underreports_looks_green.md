---
name: a-gate-that-underreports-looks-green
description: A gate's blind spot is only ever found by forecasting its output and disagreeing; when it does, suspect the gate — and fix the CLASS, not the spelling that failed
metadata:
  type: feedback
---

**A gate that under-reports is byte-indistinguishable from a green one.** The
only detector is an independent forecast — which is the exact labour a gate
exists to remove. So when a gate's output disagrees with what you expected,
**suspect the gate before you suspect your expectation.**

**Why:** four occurrences across two files, and every single one surfaced
because somebody predicted the result, never because a gate reported anything.

- `check-ledger-numbers.py`'s Pass-heading anchor: `(?:★ )?` could not see a
  `★★`/`★★★` heading. Found twice, both times by disagreement.
- `check-string-gaps.sh` (2026-08-20): reported **two of three** gaps one Pass
  introduced. The invisible one was `"...needs at least          {minimum}"` —
  the character class required a *letter* after the run and `{` was not in it.
  Found because I knew there were three and the report listed two.
- `check-ledger-numbers.py`'s decision ceiling (2026-08-20): printed
  `071 -> next free is 072` while decisions **072, 073 and 074 all existed**.
  Found because the librarian reported minting 074 and the gate disagreed;
  neither was lying and only one could be right. This one was a live hazard —
  §12 duplicate detection is deliberately absent, so the printed ceiling was
  the only thing preventing a duplicate.

**The half that keeps biting: fix the CLASS, not the spelling.**
- The first star-anchor fix repaired `★` because that was the one seen; a
  convention using one-to-three stars stayed half invisible.
- The 2026-08-11 decision-ceiling fix added `ARCHITECTURE.md` as a second
  **source** but kept a declaration-shaped **pattern** — so the hole reopened
  the moment the prevailing spelling changed from `### … decision NNN` to
  `- **date — Decision NNN…**`. *Fixing a source while leaving the pattern
  spelling-dependent is a fix that expires.*

**And the first widening is often wrong in the other direction.** Widening
`check-string-gaps.sh`'s class globally took the tree from 0 findings to ~60,
every one a deliberately aligned report column in a dev tool. The
distinguishing property was not the characters: a `thiserror` message is
**prose**, a `println!` in a sweep tool is a **table**. Scope the widening to
where the property is structurally guaranteed, and pin the false-positive shape
in the gate's CLEAN self-test so the next widening cannot re-break it.

**How to apply:** (1) before running a gate you have reason to have tripped,
say out loud how many findings you expect; a mismatch in *either* direction is
the signal. (2) When you fix a gate, ask what OTHER spellings of the same thing
it cannot see, and prefer a rule that cannot under-report over one that matches
the instance you saw. (3) Sabotage-verify: reinstate the real defect and watch
it fire. (4) Add the false-positive case to the clean set, not just the true
positive to the dirty set.

Related: [[run-the-projects-own-gates]] (the gate set is wider than
fmt/clippy/tests), [[gates-i-owe-myself]] (the ones I skip), [[two-modes-one-pattern-is-one-measurement]]
(same-pattern agreement is not verification).
