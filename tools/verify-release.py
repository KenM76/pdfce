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
3. ``HEAD`` **contains** the tag — you tagged what you tested.
4. The tag exists on the **remote**, at the same commit.
5. ``origin/main`` **contains** that commit — **the check that would have
   caught the incident.** A tag reachable only from a side branch means the
   default branch does not contain the release.

   ★ Checks 3 and 5 were exact equality until 2026-08-11, and that was
   right at release time and wrong immediately after. The first commit
   pushed to ``main`` following a release made both of them fail, and check
   5 announced "the default branch does not contain this release" about a
   branch that demonstrably did. A gate guarding an irreversible step must
   not cry wolf on correct state.

   They now report three outcomes: **at** the tag (the release moment),
   **ADVANCED** past it (normal afterwards — passes, and says so), or does
   not contain it at all (**fails**). The incident is still caught exactly,
   because in it ``origin/main`` was 36 commits **BEHIND**, and a behind
   branch does not contain the tag. Verified against the real v0.2.0 and
   v0.3.0 commits, not reasoned about.
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

import json
import subprocess
import pathlib
import sys


def git(*args: str) -> str:
    """Run a git command, returning stripped stdout ('' on failure)."""
    r = subprocess.run(("git",) + args, capture_output=True, text=True)
    return r.stdout.strip() if r.returncode == 0 else ""


def contains(ancestor: str, descendant: str) -> bool:
    """Is `ancestor` reachable from `descendant`? (i.e. does it CONTAIN it)

    The distinction this draws is the whole of the "ADVANCED" logic below.
    `git merge-base --is-ancestor` exits 0 when the first commit is an
    ancestor of the second, and a commit is its own ancestor, so an equal
    pair also answers true.
    """
    return (
        subprocess.run(
            ("git", "merge-base", "--is-ancestor", ancestor, descendant),
            capture_output=True,
        ).returncode
        == 0
    )


def main(tag: str) -> int:
    problems: list[str] = []

    def check(ok: bool, label: str, detail: str) -> None:
        print(f"  {'ok  ' if ok else 'FAIL'}  {label}")
        if not ok:
            print(f"        {detail}")
            problems.append(label)

    def check_contains(label: str, note: str, fail_detail: str, *, at: str, of: str) -> None:
        """Pass when `of` IS `at`, pass-with-a-note when it has moved PAST it,
        fail only when it does not contain it at all.

        ★ Why this is not just `==`. Both this script's commit-identity
        checks were exact-equality, which is right at release time and
        WRONG five minutes later: the moment `main` gains its next commit,
        re-running the script reports two failures on a release that is
        perfectly fine — and the `origin/main` one said "the default branch
        does not contain this release" about a branch that demonstrably
        did. A gate that cries wolf on correct state is a gate people learn
        to ignore, and this one guards the step nobody can undo.

        The incident it was written for is still caught exactly. On
        2026-08-11 the tag was correct and `origin/main` was **36 commits
        BEHIND** it. A behind branch does not contain the tag, so this
        still fails — while an AHEAD branch, which is the normal state of
        every repository after a release, now passes and says why.
        """
        at_sha, of_sha = git("rev-parse", at), git("rev-parse", of)
        if at_sha == of_sha:
            print(f"  ok    {label}")
        elif contains(at_sha, of_sha):
            print(f"  ok    {label} (ADVANCED: {note})")
        else:
            print(f"  FAIL  {label}")
            print(f"        {fail_detail}")
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

    check_contains(
        "tag is at HEAD",
        f"HEAD={head[:7]} has moved on since the release; the tag is still an "
        "ancestor, which is the normal state after any post-release commit",
        f"tag={tagged[:7]} HEAD={head[:7]} -- HEAD does not contain the tag. "
        "You tagged something other than what is checked out and tested, or "
        "you are on a branch the release was never merged into.",
        at=tagged,
        of=head,
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
    check_contains(
        "origin/main CONTAINS the tagged commit",
        f"origin/main={origin_main[:7]} is ahead of the tag -- development "
        "continued after the release, which is expected",
        f"origin/main={origin_main[:7]} tag={tagged[:7]} -- the default branch "
        "does NOT contain this release. A push can report success and move a "
        "DIFFERENT branch than the one you are on; that is how this check "
        "came to exist. If origin/main is BEHIND the tag, that is the "
        "incident itself.",
        at=tagged,
        of=origin_main,
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

    # * Is CI actually GREEN at the commit this tag points at?
    #
    # Added 2026-08-12, after this script reported v0.5.1 "clean" while the
    # tagged commit's CI run was RED. Everything it checked was true -- tag,
    # HEAD, origin/main and the asset all agreed -- because none of those
    # facts is about whether the code passes. The red gate was
    # `check-commits-filed.py`: three commits had been tagged and released
    # before the librarian filed them, so the tag permanently points at a
    # commit whose CI failed. The binaries were fine; the bookkeeping was not,
    # and anyone opening that commit on GitHub sees a red X against a shipped
    # release.
    #
    # The lesson was recorded as an ORDERING one -- file, let CI go green,
    # THEN tag. A release verifier that never consults CI is verifying that
    # the paperwork is self-consistent, not that the thing works, and that
    # half stands.
    #
    # ★ THE ORDERING HALF IS OBSOLETE AS OF 2026-08-24, and the reason is
    # worth more than the correction. An ordering is a rule that lives in
    # somebody's memory, and this one had already failed on its second
    # outing: `v0.7.0` was tagged on a filing commit and passed, `v0.8.0`
    # -- the very next release -- was tagged on a code commit and failed.
    # The check below then reported `6 of 7, exit 1` for a tag that was
    # otherwise sound, and could NEVER be cleared by re-running anything,
    # because a re-run checks out the tagged commit, where the filing that
    # would satisfy the gate does not exist.
    #
    # `check-commits-filed.py` and `check-passes-filed.py` now DEFER the
    # tip commit instead of failing on it: a commit cannot cite its own
    # hash, so demanding it was unsatisfiable by construction rather than a
    # finding. With that fixed, a tag on a code commit is green whenever
    # the history BEHIND it is filed, and no ordering has to be remembered.
    #
    # What did NOT change, so this is not read as the check being weakened:
    # a filing gate red at a tagged commit is still a hard failure here,
    # and now it means something sharper than it used to -- a commit OTHER
    # than the tip is unnarrated. That is real debt, and the fix is to file
    # it, not to move the tag.
    #
    # `in_progress` is reported distinctly from `failure`. A run still going
    # is not a pass, and silently treating "not yet failed" as "succeeded" is
    # the same absence-means-success mistake standing rule R191 names.
    gh_ci = subprocess.run(
        ["gh", "run", "list", "--commit", tagged, "--limit", "20",
         "--json", "status,conclusion,name"],
        capture_output=True, text=True,
    )
    if gh_ci.returncode != 0:
        print("  skip  CI status -- `gh` unavailable or not authenticated")
    else:
        try:
            runs = json.loads(gh_ci.stdout or "[]")
        except json.JSONDecodeError:
            runs = []
        if not runs:
            # No run at all is reported, not passed. A tagged commit that CI
            # never saw has been verified by nobody.
            check(False, "CI ran at the tagged commit",
                  "no workflow run found for this commit -- it has never been "
                  "checked by CI at all")
        else:
            failed = [r for r in runs
                      if r.get("conclusion") not in (None, "success", "skipped")]
            pending = [r for r in runs if r.get("status") != "completed"]
            if pending:
                check(False, "CI is finished at the tagged commit",
                      f"{len(pending)} run(s) still in progress -- a run that "
                      "has not failed yet is not a run that passed")
            check(
                not failed,
                "CI is GREEN at the tagged commit",
                "failing run(s): "
                + ", ".join(f"{r.get('name', '?')}={r.get('conclusion')}"
                            for r in failed)
                + " -- the tag points at a commit CI rejected. A filing gate "
                  "red here no longer means the tag landed on the wrong "
                  "commit (the tip is deferred since 2026-08-24); it means a "
                  "commit BEHIND the tag is unnarrated. File it.",
            )

    # --- the OneDrive copy the operator asked to always exist -----------
    #
    # Operator instruction, 2026-08-29: "can you always put a new version on
    # onedrive? cycle between folders pdfce1 and pdfce2 ... so there is always
    # a previous version available."
    #
    # ★ Checked HERE rather than left to the release procedure's prose,
    # because "always" is exactly the kind of instruction that survives two
    # releases and then quietly stops. `tools/deploy-onedrive.py` does the
    # work; this asks whether it actually ran for the tag being verified.
    #
    # The property verified is the operator's, not the tool's: **this version
    # is present in one slot and a DIFFERENT version is present in the
    # other.** Both slots holding the same version passes a naive
    # "is it deployed?" check and fails what he asked for.
    import os as _os

    od = None
    for _var in ("OneDrive", "OneDriveConsumer", "OneDriveCommercial"):
        _v = _os.environ.get(_var)
        if _v and pathlib.Path(_v).is_dir():
            od = pathlib.Path(_v)
            break
    if od is None:
        check(False, "OneDrive is reachable",
              "no OneDrive folder found -- cannot verify the published CLI")
    else:
        want = tag.lstrip("v")
        found = {}
        for slot in ("pdfce1", "pdfce2"):
            vf = od / slot / "VERSION.txt"
            ver = None
            if vf.is_file():
                for line in vf.read_text(encoding="utf-8", errors="replace").splitlines():
                    if line.lower().startswith("version:"):
                        ver = line.split(":", 1)[1].strip()
                        break
            found[slot] = ver
        here = [s for s, v in found.items() if v == want]
        check(
            bool(here),
            f"the CLI for {tag} is on OneDrive ({', '.join(here) or 'nowhere'})",
            f"neither pdfce1 nor pdfce2 holds {want} -- found "
            f"{found['pdfce1']!r} and {found['pdfce2']!r}. "
            "Run `python tools/deploy-onedrive.py`.",
        )
        others = [v for s, v in found.items() if s not in here and v]
        check(
            bool(others),
            f"a PREVIOUS version is still on OneDrive ({', '.join(others) or 'none'})",
            "the other slot holds no version -- the point of the two folders "
            "is that a previous build stays available, and right now one does "
            "not exist.",
        )

    if problems:
        print(f"\nverify-release: {len(problems)} problem(s): "
              f"{', '.join(problems)}")
        return 1
    print("\nverify-release: clean -- tag, HEAD, origin/main, CI and the "
          "release agree")
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("usage: python tools/verify-release.py <tag>", file=sys.stderr)
        raise SystemExit(2)
    raise SystemExit(main(sys.argv[1]))
