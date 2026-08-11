---
name: absence-needs-an-unscoped-query
description: Never conclude a file/symbol is absent from a path-scoped query returning empty — a wrong path returns nothing with exactly the confidence of a true negative
metadata:
  type: feedback
---

Before reporting that something **does not exist**, re-run the search with the
scope removed. A path-scoped query that returns empty is a fact about **the
path**, not about the repository.

**Why:** on 2026-08-11 I concluded `UI_PREFERENCES.md` "does not exist and has
never existed in git history," wrote that into memory as a *measured* fact, and
reported it to Ken. The evidence was `ls docs/UI_PREFERENCES.md` (empty) and
`git log --all -- docs/UI_PREFERENCES.md` (empty). The file was at the **repo
root**, 34 KB, git-tracked since `b0f57af`.

`--all` is what made it stick. It reads as exhaustive — *all branches, all
refs* — so an empty result feels like a strong negative. It is still
**path-scoped**, and the wrong path yields silence indistinguishable from
truth. `pdfce-ui-specialist` reached the same wrong conclusion the same day by
globbing. Two independent agents, one shared blind spot.

The compounding part: I had *just* been correcting stale claims for exactly
this reason, so the error arrived wearing the costume of diligence. Confidence
came from the act of checking, not from what was checked.

**How to apply:** for "does X exist?", the cheap unscoped forms first —
`git ls-files | grep -i name`, `git log --all -- '*name*'`,
`find . -iname '*name*'`, `rg --files | rg name`. Only after one of *those* is
empty is "absent" a claim worth making. Cost is a few seconds; the cost of the
false negative was a wrong memory, a wrong report to the operator, and a
librarian dispatch sent to investigate a non-problem.

Corollary, and the reason this is broader than one file: **when several
documents cite a stale path, the wrong conclusion becomes the easy one.** The
stale citation is the trap; the unscoped query is the escape. See
[[design-system-and-rule12-conflict]] for the specific file and the citations
still stale.

Related: [[gates-i-owe-myself]] — same family, verification I skip because it
feels already done.
