---
name: feedback-dispatch-should-carry-git-evidence-when-no-shell
description: When this librarian is dispatched without a shell and a filing turns on git history (which commit shipped what, whether two commits are ancestors of each other), the dispatch should carry the raw git/grep output directly rather than leaving the librarian to flag the question as unresolvable.
metadata:
  type: feedback
---

Confirmed 2026-08-09 (fifty-fifth filing), stated explicitly by the
dispatching engineer after resolving two consecutive open questions
this librarian had flagged as unresolvable without a shell:
`d2d03a5`'s hash-verification chain (fifty-third/fifty-fourth filings)
and the `ae59ce3`/Pass-24.0 contradiction (fifty-fourth filing, closed
same day it was raised once evidence arrived).

**The pattern.** This librarian frequently has no `Bash`/`PowerShell`
tool on a given dispatch (hard rule 8 forbids inferring commit content
or git state from documents alone in that case — the correct move is
to flag the question and stop, not guess). Both open questions above
were, in the engineer's own words, "shell-shaped, not evidence-shaped"
— closed by three `git` invocations and one `grep` once someone with a
shell ran them. Two filings in a row could only flag what a shell
closes in under two minutes.

**Why this matters:** flagging-and-waiting is the CORRECT behavior per
hard rule 8 — the failure mode isn't guessing, it's that the gap then
sits open across filings (a session, sometimes several) until someone
happens to have a shell and remembers to check. The fix is not "give
this librarian a shell" (that's the dispatcher's call, not mine to
request) — it's that when the engineer already knows a filing will
turn on git history AND knows this dispatch of the librarian won't
carry a shell, the fastest closure is pasting the relevant `git
show`/`git log -S`/`grep` output directly into the dispatch prompt,
the same way the coordinator message that unblocked `ae59ce3` did.

**How to apply:** don't ask the engineer to change this — it is
already the fix, self-identified and self-applied the same session it
was needed. This memory exists so a FUTURE session (mine, without
context of this one) doesn't re-flag the identical class of question
as unresolvable when the dispatch could have simply included the
evidence. If a future dispatch again leaves a git-history question
open with no evidence attached, it's fine to note in the filing that
carrying the raw output would have closed it faster — that's
information for the dispatcher, not a rule this librarian enforces.

See also [[project_uncommitted_repo_worktree_risk]] for the sibling
discipline (hard rule 8) this pattern sits next to — both are about
this librarian's evidentiary boundary and what's on the correct side
of it.
