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
1. **Commits that touch `docs/`.** Those ARE filings (or carry their own
   record inline). A filing commit cannot cite its own hash — the hash
   does not exist until the commit is written — so requiring it would make
   the gate unsatisfiable by construction.

2. **Commits touching no `crates/`, `tools/` or `fixtures/` path.** Memory
   snapshots, agent-file edits and CI-only changes are not engineering
   work the roadmap is meant to narrate.

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
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")


ROOT = pathlib.Path(__file__).resolve().parent.parent
RECORD = [ROOT / "docs" / "ROADMAP.md", ROOT / "docs" / "SESSION_LOG.md"]
BASELINE = ROOT / "tools" / "commits-filed-baseline.txt"

# Paths whose change makes a commit "engineering work the record narrates".
CODE_PREFIXES = ("crates/", "tools/", "fixtures/")


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
        if any(f.startswith("docs/") for f in files):
            continue  # a filing commit — see the header
        if not any(f.startswith(CODE_PREFIXES) for f in files):
            continue  # not engineering work the record narrates
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
