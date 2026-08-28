#!/usr/bin/env python3
"""check-history-not-rewritten.py — published history has not been rewritten.

WHY THIS EXISTS
===============

On 2026-08-28 a **subagent amended a commit that had already been pushed** —
**twice**, and the count is the warrant. Measured from ``git reflog --date=iso``
against push times read from GitHub:

===========  ===========  ===========  ==================  =========
orphaned     pushed at    amended      gap after push      recovery
===========  ===========  ===========  ==================  =========
``36e7b66``  05:33:21Z    ``eab7da4``  12 m 33 s           3 m 09 s
``0d20861``  05:50:10Z    ``fa16819``  **37 s**            **5 s**
===========  ===========  ===========  ==================  =========

The first was found only because ``git log --oneline -1`` was run for an
unrelated reason and the hash did not match what had been pushed twenty
minutes earlier. **The second landed on a librarian filing commit** — the
commit type whose entire content is hash citations — thirty-seven seconds
after that commit was pushed, and *after* the first had already been noticed
and hand-recovered.

★ **That is what makes this a gate rather than a note.** One lucky near-miss
is thin warrant. A behaviour that recurs inside the same minute, after being
caught once, is a reflex nobody announces.

★★ And the second recovery was ``reset: moving to origin/main`` — **this
gate's own printed remedy, executed 71 minutes before the gate existed.** What
was missing was never the knowledge. Only the announcement.

Both instances were harmless: each ``git diff`` was **empty**, so the amends
changed metadata only and ``git reset --mixed`` restored them with nothing
lost. The *class* is not harmless, and this repository has already paid for it
once: ``tools/check-cited-commits-exist.py`` was written after a rewrite
orphaned **fourteen** commit hashes cited across the documentation, because
every filing in this project cites the hash of the commit it narrates.

★ **AND CI CANNOT CATCH THIS, STRUCTURALLY.** Every other gate here answers a
question about the *tree*, which CI re-checks on the server. This one is about
the relationship between the local branch and the remote — and by the time CI
runs, the push has already happened. If the push was a force-push, the damage
is done and public. **A pre-push check is the only place this can live**, which
is why it is not in ``check-ci-parity.py``'s list and is run explicitly by
``tools/run-gates.sh``.

WHAT IT CHECKS
==============

One thing: **is ``origin/main`` an ancestor of ``HEAD``?**

If yes, the local branch is the published history plus zero or more new
commits — the only shape an ordinary fast-forward push can produce, and the
only shape decision 090's standing "always push" authorisation covers.

If no, published history has been rewritten: an amend, a rebase, a reset, or a
squash has landed on top of something already visible to anyone who has cloned
the repository. **The fix is never ``--force``.** It is to restore the branch
to the published commit and re-apply the work on top:

    git reset --mixed origin/main      # keeps the working tree
    # …then re-commit whatever the rewrite was carrying

WHAT IT DELIBERATELY DOES NOT DO
================================

It does not fetch. A gate that reaches the network is a gate that fails on a
train, and the question it answers is about the local ref ``origin/main`` — the
last state this clone observed — which is exactly the state a push will be
compared against. A stale ``origin/main`` can only make this check *lenient*,
never wrong in the direction that matters: if the remote has moved on, the push
will be rejected by the server anyway.

It also does not object to the branch being *behind*. Behind is normal and
recoverable; rewritten is neither.

EXIT CODES
==========

``0``  the branch is a clean descendant of what was published (or there is no
       remote yet, or the remote branch does not exist — both are states this
       project has genuinely been in).
``1``  published history has been rewritten. The report names the commits that
       exist on ``origin/main`` and no longer on ``HEAD``, because those are
       precisely the hashes any document may still be citing.
``2``  could not run (not a git repository, ``git`` missing).
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REMOTE_REF = "origin/main"


def git(*args: str) -> tuple[int, str]:
    """Run a git command in the repository root, returning (code, stdout)."""
    try:
        p = subprocess.run(
            ["git", *args],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
    except FileNotFoundError:
        print("history-not-rewritten: git is not on PATH", file=sys.stderr)
        raise SystemExit(2) from None
    return p.returncode, p.stdout.strip()


def main() -> int:
    code, _ = git("rev-parse", "--git-dir")
    if code != 0:
        print("history-not-rewritten: not a git repository", file=sys.stderr)
        return 2

    # No remote ref is a real state for this project — it had none until
    # 2026-08-09 — and it is not a failure. Say so rather than passing
    # silently, because "clean" and "nothing to compare" are different
    # answers and only one of them is reassuring.
    code, _ = git("rev-parse", "--verify", "--quiet", REMOTE_REF)
    if code != 0:
        print(
            f"history-not-rewritten: no {REMOTE_REF} in this clone — "
            "nothing published to rewrite."
        )
        return 0

    code, _ = git("merge-base", "--is-ancestor", REMOTE_REF, "HEAD")
    if code == 0:
        _, ahead = git("rev-list", "--count", f"{REMOTE_REF}..HEAD")
        n = ahead or "0"
        print(
            f"history-not-rewritten: clean — HEAD is a descendant of "
            f"{REMOTE_REF} ({n} commit(s) ahead)."
        )
        return 0

    # Rewritten. Name the orphans: those are the hashes documents may cite.
    _, orphans = git("log", "--oneline", f"HEAD..{REMOTE_REF}")
    print("history-not-rewritten: PUBLISHED HISTORY HAS BEEN REWRITTEN.")
    print()
    print(f"  {REMOTE_REF} is NOT an ancestor of HEAD. These commits are on")
    print("  the remote and no longer on this branch:")
    print()
    for line in (orphans or "(none listed)").splitlines():
        print(f"    {line}")
    print()
    print("  Any document citing one of those hashes now cites a commit that")
    print("  is unreachable from HEAD — the exact damage")
    print("  `check-cited-commits-exist.py` was written after, when a rewrite")
    print("  orphaned fourteen cited hashes at once.")
    print()
    print("  DO NOT `git push --force`. That publishes the rewrite and breaks")
    print("  it for everyone who has already cloned. Restore instead:")
    print()
    print(f"    git reset --mixed {REMOTE_REF}     # keeps the working tree")
    print("    # then re-commit whatever the rewrite was carrying")
    print()
    print("  A subagent can do this without announcing it — that is how BOTH")
    print("  2026-08-28 instances happened, the second 37 seconds after the")
    print("  commit it rewrote was pushed. Check the reflog for `(amend)`,")
    print("  `rebase` or `reset` to find out what did it.")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
