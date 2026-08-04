#!/usr/bin/env python3
"""check-ledger-numbers.py — uniqueness gate for pdfce's numbered ledgers.

WHY THIS EXISTS
===============
pdfce keeps four hand-maintained numbered ledgers:

  * Pass IDs            — `### Pass 18.7 — ...` headings in docs/ROADMAP.md
  * Standing rules      — `- **R106 — ...` items in ROADMAP's Standing rules
  * Decision records    — docs/decisions/NNN-*.md
  * Open operator qs    — lettered (a), (b), ... in ROADMAP

Every one of them is a PRIMARY KEY. A ROADMAP entry, a session-log
continuation, a commit message and every later cross-reference all resolve
work by its number. Two things sharing a number is not a cosmetic problem:
it makes "see Pass 19.4" ambiguous forever, and the ambiguity is discovered
by whoever is trying to understand the history, which is the worst possible
time.

Nothing enforced their uniqueness. `cargo test`, `cargo clippy`, `cargo fmt`
and `tools/check-ui-strings.sh` have no concept of a Pass number, so a
duplicate is invisible to every automated check the project has. Numbers get
assigned by an agent reading the ROADMAP, and when two pieces of work are
drafted concurrently — which is normal here — both read the same ceiling and
both claim the same next number.

That is not hypothetical. FIVE collisions were found on 2026-08-03 alone:
three Pass-ID, one standing-rule (a decision document claimed R97–R102 while
another filing had already taken R97–R99), and one duplicated heading that
had been sitting in the file undetected. Standing rule R106 was written in
response and names a mechanical uniqueness check as the missing mitigation.
This is that check.

WHAT IT CHECKS, AND THE RULE BEHIND EACH
========================================
1. **No two headings declare the same Pass ID within one top-level section.**
   NOT "globally unique" — a Pass legitimately appears twice in the file, once
   as a planning entry under `Next up`/`Backlog` and once as a `Shipped`
   entry, and flagging that would make the gate permanently red for correct
   documentation. Within a single section, though, a repeat is either a
   double-paste or two different things claiming one number.

2. **No standing-rule number is defined twice.** With one documented
   exception class — see KNOWN_RULE_AMENDMENTS below.

3. **No two decision files share a number.**

4. **Reports the live ceiling of every ledger.** This is the part that
   actually prevents collisions rather than detecting them afterwards. R106
   says to read the live ceiling immediately before assigning a number; this
   prints it, so "read the ceiling" costs one command instead of a careful
   read of an 11,000-line file.

WHAT IT DOES NOT CHECK, STATED SO THE GREEN IS NOT OVERREAD
===========================================================
* It does not verify a number is *correct*, only that it is unused. Filing
  genuinely new work under an existing Pass's number in a different section
  is invisible to this tool.
* It does not check SESSION_LOG continuation numbers. They are append-only
  and monotonic, a different failure mode.
* It does not check that a referenced rule exists. `R999` in prose passes.
* It parses Markdown with regexes. If ROADMAP's heading or rule-item
  conventions change, this silently stops covering whatever changed — the
  same limitation `check-ui-strings.sh` documents about its own truncation.
  The `--stats` counts exist so a sudden drop is visible rather than silent.

EXIT CODES
==========
  0  no duplicates found
  1  at least one duplicate found (details printed)
  2  the file or directory could not be read, or a parse produced an
     implausible result (guards against reporting a vacuous pass)

USAGE
=====
  python tools/check-ledger-numbers.py            # check, print ceilings
  python tools/check-ledger-numbers.py --stats    # add parse counts
"""

from __future__ import annotations

import os
import re
import sys
from collections import defaultdict

ROADMAP = os.path.join("docs", "ROADMAP.md")
DECISIONS = os.path.join("docs", "decisions")

# A Pass ID: digits, optional dotted segments which may be alphanumeric
# (12.M2b), and an optional trailing letter with no dot (13a, 13b).
PASS_ID = r"[0-9]+(?:\.[0-9A-Za-z]+)*[a-z]?"

# Rule numbers whose SECOND definition-shaped occurrence is an amendment
# record rather than a competing rule. Keyed by (number, distinguishing text)
# so the entry survives the line moving, which it will.
#
# R26: ROADMAP carries `- **R26 — status change, text unchanged (decision
# 006).**` — a dated note that R26's provisional clause became permanent. It
# uses the same list-item shape as a definition because that is how this
# document records amendments, but it defines nothing. Allowlisted rather
# than pattern-matched on words like "status change", because such a pattern
# would also swallow a genuine future rule that happened to be worded that
# way.
KNOWN_RULE_AMENDMENTS = {
    (26, "status change, text unchanged"),
}


def read_lines(path: str) -> list[str]:
    try:
        with open(path, encoding="utf-8", errors="replace") as fh:
            return fh.read().split("\n")
    except OSError as exc:
        print(f"ERROR: cannot read {path}: {exc}", file=sys.stderr)
        raise SystemExit(2)


def section_index(lines: list[str]) -> list[tuple[int, str]]:
    """Top-level `## ` headings, as (1-based line number, title)."""
    return [
        (n, ln[3:].strip())
        for n, ln in enumerate(lines, 1)
        if ln.startswith("## ")
    ]


def section_of(secs: list[tuple[int, str]], line_no: int) -> str:
    current = "(preamble)"
    for n, name in secs:
        if n <= line_no:
            current = name
        else:
            break
    return current


def collect_passes(lines: list[str], secs):
    """Map (section, pass id) -> [(line, heading text)].

    Only the part of a heading BEFORE the em dash is scanned. Headings
    routinely name other Passes in their descriptive half — "Pass 19.1 —
    ... (decision 019, extends the Pass 19.0 consolidation)" — and counting
    those as declarations produced 9 false duplicates on the first attempt.
    A heading may legitimately declare more than one Pass ("Pass 17.1 + Pass
    17.2 — ..."), so every ID in the prefix counts.
    """
    found = defaultdict(list)
    for n, ln in enumerate(lines, 1):
        if not re.match(r"^#{2,4} Pass ", ln):
            continue
        prefix = ln.split("—")[0]
        for pid in re.findall(rf"Pass ({PASS_ID})", prefix):
            found[(section_of(secs, n), pid)].append((n, ln.strip()[:100]))
    return found


def collect_rules(lines: list[str]):
    """Map rule number -> [(line, title)] for definition-shaped items."""
    start = next(
        (n for n, ln in enumerate(lines) if ln.startswith("## Standing rules")),
        None,
    )
    if start is None:
        print("ERROR: no '## Standing rules' section in ROADMAP.", file=sys.stderr)
        raise SystemExit(2)

    found = defaultdict(list)
    for offset, ln in enumerate(lines[start:]):
        # The optional parenthetical is not decoration: R53–R57 are written
        # `- **R53 (was R-JS-1) — ...`, recording their pre-renumbering
        # identity. A pattern that demanded the em dash immediately after the
        # digits parsed 101 of 106 rules and silently left those five
        # unguarded — the exact "gate that covers less than it appears to"
        # failure this tool exists to prevent. Caught by comparing the parsed
        # count against the ceiling, which is why --stats prints both.
        m = re.match(r"^\s*- \*\*R(\d+)(?:\s*\([^)]*\))?\s*—\s*(.*)", ln)
        if not m:
            continue
        num, title = int(m.group(1)), m.group(2).strip()
        if any(
            num == an and hint in title for an, hint in KNOWN_RULE_AMENDMENTS
        ):
            continue
        found[num].append((start + offset + 1, title[:100]))
    return found


def collect_decisions():
    if not os.path.isdir(DECISIONS):
        print(f"ERROR: {DECISIONS} is not a directory.", file=sys.stderr)
        raise SystemExit(2)
    found = defaultdict(list)
    for name in sorted(os.listdir(DECISIONS)):
        m = re.match(r"^(\d+)-.*\.md$", name)
        if m:
            found[int(m.group(1))].append(name)
    return found


def pass_sort_key(pid: str):
    """Sort Pass IDs numerically per segment so 18.7 < 18.10 and 13a < 13b."""
    key = []
    for seg in pid.split("."):
        digits = re.match(r"^(\d*)(.*)$", seg)
        key.append((int(digits.group(1) or 0), digits.group(2)))
    return key


def main() -> int:
    stats = "--stats" in sys.argv
    lines = read_lines(ROADMAP)
    secs = section_index(lines)

    passes = collect_passes(lines, secs)
    rules = collect_rules(lines)
    decisions = collect_decisions()

    # Guard against a vacuous pass. If the conventions this parses ever
    # change, the counts collapse and every check trivially succeeds — which
    # would read as "no duplicates" when the truth is "nothing was read".
    if len(passes) < 20 or len(rules) < 20 or len(decisions) < 5:
        print(
            "ERROR: implausibly few ledger entries parsed "
            f"(passes={len(passes)}, rules={len(rules)}, "
            f"decisions={len(decisions)}). ROADMAP's heading or rule-item "
            "conventions have probably changed and this checker is no longer "
            "reading them. Refusing to report a pass it cannot justify.",
            file=sys.stderr,
        )
        return 2

    failures = 0

    dup_passes = {k: v for k, v in passes.items() if len(v) > 1}
    for (sec, pid), hits in sorted(dup_passes.items()):
        failures += 1
        print(f"DUPLICATE Pass {pid} declared {len(hits)}x in section [{sec}]:")
        for n, text in hits:
            print(f"    {ROADMAP}:{n}: {text}")

    dup_rules = {k: v for k, v in rules.items() if len(v) > 1}
    for num, hits in sorted(dup_rules.items()):
        failures += 1
        print(f"DUPLICATE standing rule R{num} defined {len(hits)}x:")
        for n, title in hits:
            print(f"    {ROADMAP}:{n}: {title}")

    dup_decisions = {k: v for k, v in decisions.items() if len(v) > 1}
    for num, names in sorted(dup_decisions.items()):
        failures += 1
        print(f"DUPLICATE decision number {num:03d}: {', '.join(names)}")

    # The preventive half: state the live ceilings so assigning the next
    # number does not require reading an 11,000-line file (standing rule
    # R106). Printed on success AND failure — it is useful either way.
    #
    # A Pass family is CLAIMED as soon as a decision record or a Backlog
    # entry names it, which happens well before any `### Pass N.n` heading
    # exists. Scanning only headings therefore reports a family as free
    # while it is already spoken for.
    #
    # That is not a theoretical gap — it fired within an hour of this tool
    # shipping. Decision 020 claimed Pass 20.0–20.7 in ROADMAP's Backlog
    # prose with no heading yet, and a scoping agent working from the
    # heading-only view proposed Pass 20.x for a *different* feature family.
    # The heading scan said "highest family: 19", which was true and
    # useless. So the ceiling is computed over every `Pass N` mention in
    # the file, and claimed-but-unheaded families are called out by name —
    # they are precisely the ones a reader cannot see by skimming.
    heading_families = defaultdict(list)
    for _, pid in passes:
        heading_families[pid.split(".")[0]].append(pid)

    mentioned = defaultdict(list)
    for ln in lines:
        for pid in re.findall(rf"Pass ({PASS_ID})", ln):
            mentioned[pid.split(".")[0]].append(pid)

    def top_of(fams):
        if not fams:
            return "?", "?"
        top = sorted(fams, key=pass_sort_key)[-1]
        return top, max(fams[top], key=pass_sort_key)

    head_top, head_high = top_of(heading_families)
    ment_top, ment_high = top_of(mentioned)
    claimed_only = sorted(
        (f for f in mentioned if f not in heading_families),
        key=pass_sort_key,
    )

    print()
    print("LIVE CEILINGS (read these before assigning any new number):")
    print(f"  Pass families with headings : up to {head_top} (highest ID {head_high})")
    print(f"  Pass families MENTIONED     : up to {ment_top} (highest ID {ment_high})")
    if claimed_only:
        print(
            "  CLAIMED BUT NOT YET HEADED  : "
            + ", ".join(claimed_only)
            + "  <- already spoken for; do NOT reuse"
        )
    print(f"  standing rules      : R{max(rules)}  -> next free is R{max(rules) + 1}")
    print(
        f"  decision records    : {max(decisions):03d} "
        f"-> next free is {max(decisions) + 1:03d}"
    )

    if stats:
        print()
        print("PARSE STATS (a sudden drop means the conventions moved):")
        print(f"  distinct (section, Pass ID) pairs : {len(passes)}")
        print(f"  standing rules defined            : {len(rules)}")
        print(f"  decision files                    : {len(decisions)}")
        print(f"  allowlisted rule amendments       : {len(KNOWN_RULE_AMENDMENTS)}")

    print()
    if failures:
        print(f"ledger-numbers: {failures} duplicate(s) — see above.")
        return 1
    print("ledger-numbers: clean — no duplicate Pass, rule, or decision numbers.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
