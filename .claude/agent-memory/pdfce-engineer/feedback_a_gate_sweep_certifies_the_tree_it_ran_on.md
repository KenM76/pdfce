---
name: a-gate-sweep-certifies-the-tree-it-ran-on
description: Running all gates green then making more commits ships a red CI — check-commits-filed.py is structurally unable to pass on a code commit made after its own filing
metadata:
  type: feedback
---

**A clean gate sweep certifies the tree it ran on, not the tree you push.**
Re-run the filing gates as the LAST act before pushing, after the final
commit exists.

**Why:** 2026-08-26, `v0.14.0`. I ran all 17 `tools/check-*` green, then made
two more commits (a prose correction, then the version bump), tagged, pushed,
and cut the GitHub release. CI came back red — one job of ten — because
`check-commits-filed.py` saw the prose-correction commit in no filing. The
release had to be re-tagged onto the filing commit and the asset rebuilt.
Nine of ten jobs were green; the failure was entirely self-inflicted ordering.

**The structural part, which is the bit worth carrying:**
`check-commits-filed.py` cannot be green on a code commit made *after* the
filing that was supposed to narrate it. A commit cannot cite its own hash, so
its filing is always a *later* commit. Only two orders work:

- `code → file` and then STOP, or
- `code → file → code → file` (every code commit gets a filing after it).

There is no order in which "file, then commit more code, then push" is green.
The tip commit is deferred (checked on the next run), which makes the version
bump itself safe — but anything *behind* the tip is checked in full.

**How to apply:** before `git push` on a release, run
`python tools/check-commits-filed.py` and `check-passes-filed.py` again,
right then, with nothing uncommitted and nothing left to commit. If they name
a commit, dispatch the librarian BEFORE pushing — filing after the push means
the tag points at a commit CI will reject, and moving a public tag is a fact
that then has to be recorded too.

Related: [[feedback_never_bundle_code_into_a_filing_commit]] (the same gate,
the opposite mistake), [[feedback_gates_i_owe_myself]],
[[feedback_run_the_projects_own_gates]].
