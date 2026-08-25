#!/usr/bin/env python3
"""check-suite-name-absent -- the licensed suite's name must not appear in this repository.

Operator ruling, 2026-08-25: the name of the licensed print-conformance suite
pdfce measures itself against is kept out of this public repository entirely --
file contents AND file names. `docs/ROADMAP.md` open question `(bt)`.

WHY THIS SCRIPT EXISTS AT ALL, AND WHY IT LOOKS ODD
===================================================
A check for "this word must never appear" has a bootstrapping problem: the
obvious implementation is `git grep -i <word>`, and the moment that command is
written into a tracked file, the tracked file contains the word. The gate
becomes its own first violation, and a reader cannot tell a real occurrence
from the gate that hunts them.

So the needles are stored **base64-encoded** and decoded at run time. This is
NOT obfuscation for its own sake and NOT security by obscurity -- there is
nothing secret here, and `manifest.json` in the private map directory names the
suite in full. It is the only way for the rule and its enforcement to live in
the same repository without contradicting each other.

WHAT IT CHECKS
==============
1. Tracked file CONTENTS, case-insensitively, excluding binaries. A binary OCR
   model matches a naive grep because its weights happen to contain the byte
   sequence; `git grep -I` excludes it, and that exclusion is deliberate rather
   than convenient -- a model's weights are not a mention.
2. Tracked file NAMES. A scrubbed file called `<name>-check.py` still fails.

EXIT CODES
==========
0  clean -- no occurrence in any tracked file's content or name
1  at least one occurrence, printed with `path:line` so it can be opened
2  the check could not run (not a git work tree, `git` missing)

The output deliberately prints the offending LINE NUMBER but NOT the line's
text, because printing it would reproduce the term in CI logs -- which are
themselves public on a public repository.
"""

import base64
import subprocess
import sys

# The two forbidden needles, base64 of the lowercase forms. Decoded at run time
# so that this file -- which is itself tracked -- does not contain them.
NEEDLES_B64 = ("Z2hlbnQ=", "Z3dn")


def needles():
    """Decode the forbidden terms.

    Kept as a function rather than a module constant so the decoded strings are
    short-lived and never appear in a traceback's frame locals dump.
    """
    return [base64.b64decode(n).decode("ascii") for n in NEEDLES_B64]


def tracked_hits(term):
    """`path:line` for every tracked TEXT file containing `term`, case-insensitively.

    `-I` excludes binary files (see the module docstring on why that is correct
    rather than expedient). `-n` gives line numbers, `-i` case-insensitivity, and
    `--name-only` is deliberately NOT used -- a reviewer needs the line number to
    open the right place without the text being echoed.
    """
    proc = subprocess.run(
        ["git", "grep", "-I", "-i", "-n", "-o", "-e", term],
        capture_output=True,
        text=True,
        check=False,
    )
    if proc.returncode not in (0, 1):
        print("check-suite-name-absent: git grep failed:", proc.stderr.strip())
        sys.exit(2)
    hits = []
    for line in proc.stdout.splitlines():
        # `-o` prints `path:line:match`; drop the match so the term is not echoed.
        parts = line.rsplit(":", 1)
        if parts:
            hits.append(parts[0])
    return hits


def tracked_names(term):
    """Tracked file PATHS containing `term`, case-insensitively.

    A file whose contents are clean but whose NAME still carries the term has
    not been scrubbed -- the name is published in every directory listing, every
    commit diff and the repository's web view.
    """
    proc = subprocess.run(["git", "ls-files"], capture_output=True, text=True, check=False)
    if proc.returncode != 0:
        print("check-suite-name-absent: not a git work tree")
        sys.exit(2)
    return [p for p in proc.stdout.splitlines() if term in p.lower()]


def main():
    bad_content, bad_names = [], []
    for term in needles():
        bad_content.extend(tracked_hits(term))
        bad_names.extend(tracked_names(term))

    if not bad_content and not bad_names:
        print("suite-name-absent: clean -- no tracked file names or mentions it")
        return 0

    for path in sorted(set(bad_names)):
        print("FILENAME  %s" % path)
    for where in sorted(set(bad_content)):
        print("CONTENT   %s" % where)
    print(
        "suite-name-absent: %d file name(s) and %d line(s) still carry it "
        "(operator ruling 2026-08-25; see the private map directory)"
        % (len(set(bad_names)), len(set(bad_content)))
    )
    return 1


if __name__ == "__main__":
    sys.exit(main())
