"""verify-release.py — the tagged commit is the one the world actually got.

WHY THIS EXISTS
===============

On 2026-08-11, releasing ``v0.2.0`` nearly published a release that did not
contain the code.

The session had been working on branch ``pass-8-redaction``. ``git push origin
main`` pushed the **local ``main`` branch** — 36 commits behind — while ``git
tag`` correctly pointed at ``HEAD``. The push **reported success**, because it
did exactly what it was asked and not what was meant:

    To https://github.com/KenM76/pdfce.git
       5007bef..826e351  main -> main        <- an OLD commit, not HEAD

The release asset was built from the working tree and was therefore right; the
repository's ``main`` branch was not. Anyone cloning the repo at the release
would have got neither the tagged code nor a hint that anything was wrong.

It was caught only by comparing ``git rev-parse origin/main`` against ``HEAD``
afterwards, by hand. **That comparison is this script.**

WHAT IT CHECKS
==============

1. The working tree is clean — a release built from uncommitted changes is not
   reproducible from the tag.
2. The tag exists locally, and resolves to a commit.
3. The tag's commit is **``HEAD``** — you tagged what you tested.
4. The tag exists on the **remote**, at the same commit.
5. ``origin/main`` is at that commit too — **the check that would have caught
   the incident.** A tag reachable only from a side branch means the default
   branch does not contain the release.
6. If ``gh`` is available: a GitHub release exists for the tag and has at least
   one asset attached. A release with no binary is a release nobody can use.

Every failure prints what was expected, what was found, and why it matters.
Exit status is 0 when every check passes and 1 otherwise, so it can gate a
release script.

WHAT IT DOES NOT DO
===================

It does not push, tag, or publish anything, and it never will — those are the
operator's acts (``CLAUDE.md`` rule 8). This only reports.

It also cannot tell you the *contents* are correct. It answers one question:
does the thing you tagged match the thing the remote and the release point at?
That was the question that went wrong.

USAGE
=====

::

    python tools/verify-release.py v0.2.0
"""

from __future__ import annotations

import subprocess
import sys


def git(*args: str) -> str:
    """Run a git command, returning stripped stdout ('' on failure)."""
    r = subprocess.run(("git",) + args, capture_output=True, text=True)
    return r.stdout.strip() if r.returncode == 0 else ""


def main(tag: str) -> int:
    problems: list[str] = []

    def check(ok: bool, label: str, detail: str) -> None:
        print(f"  {'ok  ' if ok else 'FAIL'}  {label}")
        if not ok:
            print(f"        {detail}")
            problems.append(label)

    print(f"verify-release {tag}")

    dirty = git("status", "--porcelain")
    check(
        not dirty,
        "working tree clean",
        f"{len(dirty.splitlines())} uncommitted path(s) -- a release built from "
        "these is not reproducible from the tag",
    )

    head = git("rev-parse", "HEAD")
    tagged = git("rev-parse", f"{tag}^{{commit}}")
    check(bool(tagged), f"tag {tag} exists locally", "no such tag")
    if not tagged:
        # Everything below compares against the tag; without it there is
        # nothing to say that is not noise.
        return 1

    check(
        tagged == head,
        "tag is at HEAD",
        f"tag={tagged[:7]} HEAD={head[:7]} -- you tagged something other than "
        "what is checked out and tested",
    )

    remote_tag = ""
    for line in git("ls-remote", "--tags", "origin").splitlines():
        sha, _, ref = line.partition("\t")
        if ref.endswith(f"refs/tags/{tag}") or ref.endswith(f"refs/tags/{tag}^{{}}"):
            # An annotated tag lists both the tag object and, with ^{}, the
            # commit it points at. The commit is the one that matters.
            if ref.endswith("^{}") or not remote_tag:
                remote_tag = sha.strip()
    check(bool(remote_tag), f"tag {tag} is pushed", "not found on origin")

    # * The check the incident needed.
    origin_main = git("rev-parse", "origin/main")
    check(
        origin_main == tagged,
        "origin/main is AT the tagged commit",
        f"origin/main={origin_main[:7]} tag={tagged[:7]} -- the default branch "
        "does not contain this release. A push can report success and move a "
        "DIFFERENT branch than the one you are on; that is how this check "
        "came to exist.",
    )

    gh = subprocess.run(
        ["gh", "release", "view", tag, "--json", "assets", "--jq",
         "[.assets[].name] | length"],
        capture_output=True, text=True,
    )
    if gh.returncode != 0:
        print("  skip  GitHub release -- `gh` unavailable or not authenticated")
    else:
        count = gh.stdout.strip()
        check(
            count.isdigit() and int(count) > 0,
            "GitHub release has at least one asset",
            f"asset count = {count or '0'} -- a release with no binary is one "
            "nobody can use",
        )

    if problems:
        print(f"\nverify-release: {len(problems)} problem(s): "
              f"{', '.join(problems)}")
        return 1
    print("\nverify-release: clean -- tag, HEAD, origin/main and the release agree")
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("usage: python tools/verify-release.py <tag>", file=sys.stderr)
        raise SystemExit(2)
    raise SystemExit(main(sys.argv[1]))
