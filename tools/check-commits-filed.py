#!/usr/bin/env python3
"""check-commits-filed.py — every CODE commit is named in the record.

WHY THIS EXISTS
---------------
`check-passes-filed.py` already checks that every commit *claiming a Pass
ID* is filed. That is a real gate and it has caught real things — and its
blind spot is total: **a commit that claims no Pass ID is invisible to
it.** Most commits claim none.

The consequence was measured twice in one session on 2026-08-09:

1. Five commits from an earlier session (`d3ea5de`, `1edf4e3`, `9abf5b5`,
   `e167867`, `01b90c4`) had zero presence in `ROADMAP.md` or
   `SESSION_LOG.md`. They were found only because a librarian filing
   happened to grep for them. Among them: `/ca`/`/CA` transparency being
   dropped at one line, and text with no `/Widths` stacking every glyph on
   a single point.
2. That backlog was then declared closed — and a **sixth** commit surfaced
   an hour later, from a sentence inside one of the five's own commit
   messages. Pulling that thread found **five more**, including the
   removal of a third party's confidential material and a `gs` operator
   that was a silent no-op on essentially every real file.

Both discoveries were accidents. `check-passes-filed.py` was green
throughout, correctly, because none of the eleven claimed a Pass ID.

**A backlog found twice by accident will be found a third time by
accident.** This is the exhaustive walk that replaces the accident.

KNOWN WEAKNESS, stated rather than discovered later
---------------------------------------------------
The join is "this commit's short hash appears somewhere in the record". A
hash cited in an OWED list — an entry naming commits that still need
filing — therefore counts as filed. This is the same limit
`check-passes-filed.py`'s own docstring names, and it is live here rather
than theoretical: `ROADMAP.md`'s filing-51 entry lists the eleven
baseline commits by hash as owed work, so this gate now sees them as
cited.

That is why the baseline file exists as well, and why it is the thing to
read for the real debt. Two records of one obligation, deliberately —
the same "an obligation needs a record on both sides" pattern this
project applied to the `FEATURES.md` Acrobat column.

WHAT COUNTS AS FILED
--------------------
The commit's abbreviated hash appears in `docs/ROADMAP.md` or
`docs/SESSION_LOG.md`. That is this project's own citation convention —
Shipped entries read "committed `6d63d81`" — so it is the convention being
checked rather than a new one being imposed.

WHAT IS SKIPPED, AND WHY EACH EXCLUSION IS PRINCIPLED
-----------------------------------------------------
1. **Commits that touch NO code** — no `crates/`, `tools/` or `fixtures/`
   path. A pure filing commit is one of these, and it must be skipped for a
   structural reason: **a filing commit cannot cite its own hash**, because
   the hash does not exist until the commit is written. Requiring it would
   make the gate unsatisfiable by construction.

   ★ **THIS ITEM USED TO READ "Commits that touch `docs/`", AND THAT IS NOT
   WHAT THE CODE DOES.** It was true until 2026-08-11, when `b4a66ed` was
   caught carrying a real CLI/GUI change bundled with a filing of three
   *other* commits — a mixed commit self-certifying by containing the very
   record files it was never described in. The logic was corrected that day
   (see the comment at the `has_code` test); **this summary was not**, and it
   went on describing an exemption the gate had stopped granting.

   Corrected 2026-08-18, after it misled a librarian filing into planning a
   mixed code+docs commit on the belief that the code half would be exempt.
   A gate's prose and a gate's behaviour disagreeing is worse than either
   being wrong alone, because the prose is what people plan against.

2. Consequently: memory snapshots, agent-file edits and CI-only changes are
   not engineering work the roadmap is meant to narrate, and are skipped by
   the same test.

   **A commit with code in it is checked whatever else it touches**, which is
   the whole of the 2026-08-11 correction: a mixed commit is precisely where
   code hides.

THE RATCHET, AND WHY IT IS NOT A WEAKENING
------------------------------------------
Eleven commits were already unfiled when this gate was written. A gate
that fails at baseline enforces nothing and trains everyone who sees it
red to ignore it — `check-ui-strings.sh`'s header records that exact
failure, an inline CI grep that had been red on 140 hits for so long that
it was concealing a genuine violation.

So the known debt lives in `tools/commits-filed-baseline.txt`, one hash
per line, and the gate fails only on commits **outside** it. That makes
the debt VISIBLE (it is a file someone can read and shorten) rather than
INVISIBLE (the state it was in), and it stops the backlog growing while
it is worked down.

Removing a hash from the baseline once it is filed is the intended
workflow. **Adding one is not** — a new unfiled commit is what this gate
exists to catch, and silencing it by extending the baseline would be the
false-green shape R106 has been amended four times over.

EXIT CODES
----------
0  clean — every code commit outside the baseline is cited in the record.
1  one or more unfiled; each printed with its date and subject.

USAGE
-----
    python tools/check-commits-filed.py [--since 2026-08-01]
"""

from __future__ import annotations

import argparse
import pathlib
import subprocess
import sys

# Windows consoles default to a code page that cannot encode the em-dashes,
# arrows and stars this file prints, so Python substitutes "?" for exactly
# the characters that make a failure message readable. One reconfigure fixes
# every message in the file without flattening the typography.
#
# This is not theoretical: `check-commits-filed.py` was observed printing
# "each commit's full message ? they carry" while doing its job correctly.
# Found by reading a gate's output as its audience (R174), not by reading
# its source.
#
# ★ STDERR TOO, added 2026-08-11. This reconfigured only stdout, and the
# shallow-clone guard below writes to STDERR — so its em-dash came out as
# `?` on the first run. The same lesson, in the same file, one stream over:
# the fix was applied where the problem had been SEEN rather than to every
# stream that could have it.
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
if hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")


ROOT = pathlib.Path(__file__).resolve().parent.parent
RECORD = [ROOT / "docs" / "ROADMAP.md", ROOT / "docs" / "SESSION_LOG.md"]
BASELINE = ROOT / "tools" / "commits-filed-baseline.txt"

# Paths whose change makes a commit "engineering work the record narrates".
# ★ `.github/` ADDED 2026-08-21, and the hole it closes is the funny one:
# **the workflow that RUNS these gates was not covered by the gate that
# checks commits are filed.** A CI change is exactly the kind of change that
# belongs on the record -- 2ddbbbe wired eight previously-unrun gates into CI
# and would have gone unfiled forever, invisible, because this tuple did not
# name the directory it touched.
#
# Same shape as the `check-outcome-disclosed.py` finding two days earlier: a
# gate that is green because its INPUT LIST is too narrow, not because its
# pattern is wrong. Widening a pattern does not fix a list.
#
# Measured before changing, not after: seventeen commits in history touch
# `.github/`, and the sweep below was run to confirm they are already filed
# rather than assuming it. Any that are not go in the baseline as named debt,
# never silently.
# ★ `fuzz/` ADDED 2026-08-21, and it is the fourth prefix rather than the
# first three because nobody thought of it — which is this gate's own
# documented failure shape, applied to itself.
#
# `cargo-fuzz` targets are CODE: they construct the public specs they
# fuzz, so they break when a spec gains a field, and they go stale in
# silence when a spec gains a VARIANT. Both have happened. `2523860`
# (2026-08-05) is titled *"the vector-edit target had drifted three Passes
# behind its own module"*, and on 2026-08-19 the `annot_author` target
# stopped compiling because `MarkupSpec::Square` had gained
# `border_effect` — three days red, on every push.
#
# A commit touching only `fuzz/` was classified as docs-only and skipped
# by this gate, so the record has no obligation to explain any of it.
CODE_PREFIXES = ("crates/", "tools/", "fixtures/", ".github/", "fuzz/")


def git(*args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=ROOT, capture_output=True, text=True, check=False
    ).stdout


def load_baseline() -> set[str]:
    if not BASELINE.exists():
        return set()
    out = set()
    for line in BASELINE.read_text(encoding="utf-8").splitlines():
        line = line.split("#", 1)[0].strip()
        if line:
            out.add(line)
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    # The whole history by default, not a window. `check-passes-filed.py`
    # defaulted to the last 60 commits until 2026-08-09 and missed a
    # Pass-claiming commit five days old for exactly that reason — a gate
    # whose coverage is a function of how fast the project moves reports
    # "clean" most reliably when there is most to find. 0.5 s over 323
    # commits buys nothing worth that.
    ap.add_argument("--since", default="")
    args = ap.parse_args()

    # ★ REFUSE TO RUN ON A SHALLOW CLONE.
    #
    # This gate classifies a commit by the paths it touches, and a
    # docs-only commit is skipped because it IS a filing. On a shallow
    # clone the boundary commit has no parent to diff against, so git
    # reports it as adding every file in the tree — and a docs-only
    # filing commit gets classified as CODE and reported unfiled.
    #
    # That is exactly what happened in CI on 2026-08-11.
    # `actions/checkout` defaults to `fetch-depth: 1`, and this gate had
    # been running against a ONE-COMMIT history for as long as the job
    # existed. It did not error and it did not look broken: it printed a
    # confident, specific, WRONG list of unfiled commits. Reproduced with
    # `git clone --depth 1` before the fix; byte-identical to the CI
    # failure.
    #
    # The workflow now sets `fetch-depth: 0`. This check exists because
    # that line can regress, and the failure mode when it does is not a
    # crash but a plausible lie — the worst thing a gate can do, and the
    # exact hazard R176 is about (a correct-looking signal that is not
    # measuring what its reader believes).
    # `.strip()` is load-bearing: THIS script's `git()` returns raw stdout,
    # unlike `verify-release.py`'s identically-named helper which strips.
    # Written without it first, and the guard silently never fired —
    # `'true\n' == 'true'` is False. It cost a debug print to find, because
    # a guard that does not fire looks exactly like a condition that is not
    # met, which is the same class of quiet failure the guard itself exists
    # to prevent.
    if git("rev-parse", "--is-shallow-repository").strip() == "true":
        print(
            "ERROR: this is a SHALLOW clone, so commit classification is "
            "meaningless here.",
            file=sys.stderr,
        )
        print(
            "       A shallow boundary commit has no parent, so git reports "
            "it as adding every",
            file=sys.stderr,
        )
        print(
            "       file — and a docs-only filing commit then looks like "
            "unfiled CODE.",
            file=sys.stderr,
        )
        print(
            "       Fix the checkout (`fetch-depth: 0`); do not interpret "
            "any result below.",
            file=sys.stderr,
        )
        return 2

    record = "\n".join(
        p.read_text(encoding="utf-8", errors="replace") for p in RECORD if p.exists()
    )
    baseline = load_baseline()

    log_args = ["log", "--format=%h"]
    if args.since:
        log_args.append(f"--since={args.since}")
    hashes = [h for h in git(*log_args).split() if h]
    unfiled: list[tuple[str, str]] = []
    checked = 0

    for h in hashes:
        files = git("show", "--stat", "--format=", "--name-only", h).splitlines()
        files = [f.strip() for f in files if f.strip()]
        has_code = any(f.startswith(CODE_PREFIXES) for f in files)
        if not has_code:
            continue  # not engineering work the record narrates
        # A commit touching docs/ used to be skipped outright as "a filing
        # commit". That exempted MIXED commits — code bundled with a
        # filing — and a mixed commit is precisely where code hides.
        #
        # Found live on 2026-08-11: `b4a66ed` carried a real CLI/GUI change
        # plus a librarian filing of three OTHER commits, swept in by a
        # `git add -A`. Its own hash appears nowhere in the record, and the
        # gate reported clean. The commit self-certified by containing the
        # record files it was never described in.
        #
        # Now: docs-only commits are still skipped (nothing to narrate),
        # but a commit with code in it is checked whatever else it touches.
        if not has_code:
            continue
        checked += 1
        if h in baseline:
            continue
        if h not in record:
            subject = git("log", "-1", "--format=%ci %s", h).strip()
            unfiled.append((h, subject))

    if unfiled:
        print(f"commits-filed: {len(unfiled)} code commit(s) are in no filing.\n")
        for h, subject in unfiled:
            print(f"  {h}  {subject[:100]}")
        print(
            "\n  Dispatch `pdfce-librarian` with each commit's full message — they carry"
        )
        print("  the defect, the measurement and the owed follow-up, which is what a")
        print("  filing needs and what a one-line subject cannot supply.")
        print(
            "\n  Do NOT add these to tools/commits-filed-baseline.txt. That file is the"
        )
        print("  pre-existing debt this gate was written around; extending it would")
        print("  silence exactly what the gate exists to catch.")
        return 1

    known = len(baseline)
    print(
        f"commits-filed: clean — {checked} code commit(s) checked ({args.since or 'whole history'}); "
        f"{known} known-unfiled carried in the baseline"
    )
    if known:
        print(
            "  (that baseline is DEBT, not an allowlist — shortening it is the "
            "intended direction)"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
