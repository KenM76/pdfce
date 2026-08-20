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

3. **No two decision files share a number.** Decision numbers declared
   only in ARCHITECTURE.md §12 are counted toward the reported CEILING but
   are NOT uniqueness-checked — their declaration form is not separable
   from amendments and prose by regex. See collect_decisions().

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

  **It DOES, since 2026-08-07, check SESSION_LOG filing ORDINALS.** Those
  are a different thing from continuation numbers: they are used as
  identifiers in prose across every document ("the twenty-eighth filing's
  hypothetical", "amended by the thirtieth filing"), so a duplicate makes
  two distinct filings indistinguishable to every later reference.

  It was added because two librarians filing concurrently BOTH claimed the
  *thirtieth* filing. The second noticed and ceded to thirty-first — by
  reading, not by any check. Pass IDs, rule numbers and decision numbers
  all had uniqueness enforced; ordinals were used exactly like them and had
  nothing. A collision would have been invisible to every automated gate in
  the repo.
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


ROADMAP = os.path.join("docs", "ROADMAP.md")
SESSION_LOG = os.path.join("docs", "SESSION_LOG.md")
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
        # `★ ` is allowed between the hashes and "Pass". `ROADMAP.md` marks
        # umbrella/operator-request headings that way, and the anchor used to
        # require "Pass" immediately after the hashes — so TEN headings
        # (`★ Pass 15/16/17/18/19/21/24.0/26.0/33.0/38.1`) were invisible to
        # this checker and had never been uniqueness-checked at all.
        #
        # Found by the librarian predicting the delta before running the gate:
        # it declared three Pass IDs, the checker counted two, and the missing
        # one was a `★` heading. A gate whose blind spot is only discovered by
        # someone independently forecasting its output is a gate that has been
        # reporting less than it appeared to — the R53-R57 shape.
        #
        # ★★ IT RECURRED ON 2026-08-18, THE SAME SHAPE ONE STAR WIDER. The
        # anchor above accepted EXACTLY ONE star, so `★★ Pass ...` and
        # `★★★ Pass ...` were invisible — and the librarian filing Pass 101.0
        # wrote three such headings and watched the gate report the ceiling
        # unchanged at 100. Same discovery route as last time: somebody
        # predicted the delta and the gate disagreed.
        #
        # Widened to `★+`. Note what the first fix got wrong, because it is
        # the transferable part: it repaired the ONE spelling that had been
        # seen rather than the CLASS, so the emphasis convention this project
        # uses everywhere (one to three stars by weight) was still half
        # invisible. A gate anchored on a decorative prefix must accept every
        # spelling of that decoration, not the one in front of it that day.
        if not re.match(r"^#{2,4} (?:★+ )?Pass ", ln):
            continue
        prefix = ln.split("—")[0]
        # A STAGED-SHIP QUALIFIER makes two entries for one Pass legitimate.
        #
        # `Pass 32.0 (core + CLI)` and `Pass 32.0 (GUI half)` are ONE Pass
        # shipped in stages, each filed as it landed — which is the
        # append-only discipline working, not an ID collision. The hazard
        # this check exists for is a Pass ID minted twice for UNRELATED
        # work; two qualified halves of one Pass are the opposite of that.
        #
        # So the qualifier joins the key. That makes the check STRICTER
        # where it matters, not looser: two entries both qualified
        # `(GUI half)`, or two both unqualified, still collide — and those
        # are the shapes that actually mean somebody re-used an ID. Before
        # this, `Pass 32.0 (core + CLI)` twice and `Pass 32.0 (core + CLI)`
        # + `Pass 32.0 (GUI half)` were indistinguishable, so the gate
        # reported the harmless case and had no way to be louder about the
        # harmful one.
        #
        # Deliberately NOT a blanket "ignore repeats in Shipped": that would
        # be weakening a gate to make it green, which is the false-green
        # shape R106 has been amended four times over. A qualifier must be
        # PRESENT and DISTINCT to earn the exemption.
        qualifier = ""
        q = re.search(r"Pass " + PASS_ID + r"\s*\(([^)]{1,40})\)", prefix)
        if q:
            qualifier = " ".join(q.group(1).split()).lower()
        for pid in re.findall(rf"Pass ({PASS_ID})", prefix):
            found[(section_of(secs, n), pid, qualifier)].append((n, ln.strip()[:100]))
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


ARCHITECTURE = os.path.join("docs", "ARCHITECTURE.md")

# A dated decision entry in ARCHITECTURE.md §12, e.g.
#   "### Decision 040 — printing honours the operator's CMYK intent"
#   "**Decision 039 (2026-08-11)** — the `aes` dependency ..."
# Only a DECLARATION counts: a heading, or a bolded "Decision NNN" opening
# a paragraph. Prose back-references ("see decision 019") must not count,
# or every cross-reference would read as a second declaration.
ARCH_DECISION = re.compile(
    r"^(?:#{2,4}\s.*?|\*\*)[Dd]ecision\s+(\d{3})\b",
)

# ★ THE CEILING SCAN, WIDENED 2026-08-20 AFTER IT UNDER-REPORTED BY THREE.
#
# `ARCH_DECISION` above is a DECLARATION pattern: a heading, or a line
# beginning `**Decision NNN`. It is right for saying WHERE a decision was
# declared. It is NOT sufficient for the ceiling, and on 2026-08-20 this
# checker printed
#
#     decision records    : 071 -> next free is 072
#
# while decisions **072, 073 AND 074 already existed** in ARCHITECTURE.md.
# All three are written as a dated list item —
# `- **2026-08-19/20 — Decision 074. ...**` — so the `**` alternative, which
# requires `**` at line start followed immediately by "Decision", could not see
# any of them, and neither could the heading alternative.
#
# ★ THIS IS THE EXACT FAILURE collect_decisions()'s OWN DOCSTRING WAS WRITTEN
# ABOUT, RECURRING. It says: "this checker printed 'next free is 039' — a
# number that was ALREADY TAKEN, TWICE. That is worse than no answer … a
# confidently wrong ceiling is how a duplicate gets created rather than
# caught." The 2026-08-11 fix added ARCHITECTURE.md as a second SOURCE but kept
# a DECLARATION-shaped PATTERN, so the same hole reopened the moment the
# prevailing spelling changed. §12 duplicate detection is deliberately absent
# (see the docstring), so nothing else in this project would have caught the
# duplicate this was about to produce.
#
# THE FIX IS TO STOP PATTERN-MATCHING A SPELLING. For a CEILING, any mention of
# `decision NNN` anywhere in the file is safe, and that is provable rather than
# hopeful: `arch` drives the ceiling ONLY (never duplicate detection), the
# ceiling is a MAX, and a prose back-reference can only name a decision that
# already exists — so it can never raise the max above a real number. The
# widening therefore cannot produce a false ceiling, and it cannot
# under-report, whatever spelling a future filing invents.
#
# Direction of error, stated rather than left to be inferred: this over-counts
# WHICH numbers are "spoken for" if a filing ever writes a forward reference to
# a decision it does not then mint. That costs one skipped number. The opposite
# error costs a duplicate decision record that nothing here can see.
ARCH_DECISION_MENTION = re.compile(r"[Dd]ecision\s+(\d{3})\b")


def collect_decisions():
    """Every decision number that is SPOKEN FOR, from both places they live.

    ★ This used to read `docs/decisions/NNN-*.md` and nothing else, and that
    silently under-counted for a long time: not every decision gets its own
    file. On 2026-08-11 the files stopped at 038 while ARCHITECTURE.md §12
    had already minted 039 and 040, so this checker printed
    "next free is 039" — a number that was ALREADY TAKEN, TWICE.

    That is worse than no answer. The whole point of the line is to be
    trusted by whoever mints the next one, and a confidently wrong ceiling
    is how a duplicate gets created rather than caught. (Filed the same day
    a Pass ID was duplicated for exactly this reason — an unverified number
    relayed as fact. See `feedback_absence_needs_an_unscoped_query`'s
    cousin: a number you did not measure is not a number you know.)

    Numbers 034, 035, 036, 039 and 040 exist ONLY in §12 with no file, so
    file-only counting is not a stricter reading of the convention — it is
    a wrong one. Both sources are authoritative; the ceiling is the max.

    ★ THE TWO SOURCES ARE USED FOR DIFFERENT JOBS, AND THAT IS THE TRICK.

    `files` drives BOTH the ceiling and duplicate detection. `arch` drives
    the CEILING ONLY. Two reasons, and the second is the interesting one:

    1. A decision having both a file and a §12 entry is the NORMAL shape —
       027, 037 and 038 all do — so a merged list would flag every properly
       documented decision as a duplicate.
    2. §12 uniqueness is **not reliably detectable by regex**, and pretending
       otherwise was tried and reverted on 2026-08-11. Its entries put the
       number AFTER the em dash (`### 2026-08-11 (…) — decision 040: …`),
       and amendments reuse the identical shape
       (`— decision 038 RECONCILED: …`), as does ordinary bolded prose
       (`**Decision 038 is explicitly UNCHANGED by this entry`). A pattern
       tight enough to exclude those would exclude real declarations too.

       This is the same false-positive class `collect_passes` above already
       documents — "headings routinely name other Passes in their
       descriptive half … counting those as declarations produced 9 false
       duplicates on the first attempt." The first attempt here reproduced
       it exactly, on 038.

    So §12 duplicates are NOT checked. That is a real hole and it is stated
    in the module docstring's "WHAT IT DOES NOT CHECK" rather than left for
    someone to infer from a green run. The hole this closes is the worse
    one: before this, the printed ceiling was **wrong**, not merely
    incomplete — it said "next free is 039" while §12 had already minted
    039 AND 040. A confidently wrong ceiling is how a duplicate gets
    created; a missing check only fails to catch one.

    Returns `(files, arch)`, each `{number: [where]}`.
    """
    if not os.path.isdir(DECISIONS):
        print(f"ERROR: {DECISIONS} is not a directory.", file=sys.stderr)
        raise SystemExit(2)
    files = defaultdict(list)
    for name in sorted(os.listdir(DECISIONS)):
        m = re.match(r"^(\d+)-.*\.md$", name)
        if m:
            files[int(m.group(1))].append(name)

    # ...and the ones declared in ARCHITECTURE.md §12, which is where a
    # decision that never got its own file lives (034-036, 039, 040).
    arch = defaultdict(list)
    for lineno, line in enumerate(read_lines(ARCHITECTURE), start=1):
        stripped = line.strip()
        m = ARCH_DECISION.match(stripped)
        if m:
            arch[int(m.group(1))].append(f"{ARCHITECTURE}:{lineno}")
            continue
        # Anything else that names a decision number at all. CEILING ONLY —
        # see ARCH_DECISION_MENTION's note for why this cannot over-report a
        # ceiling, and why it must not be narrowed back to a declaration shape
        # the next time the prevailing spelling changes.
        for mention in ARCH_DECISION_MENTION.finditer(stripped):
            arch[int(mention.group(1))].append(f"{ARCHITECTURE}:{lineno} (mention)")
    return files, arch


def pass_sort_key(pid: str):
    """Sort Pass IDs numerically per segment so 18.7 < 18.10 and 13a < 13b."""
    key = []
    for seg in pid.split("."):
        digits = re.match(r"^(\d*)(.*)$", seg)
        key.append((int(digits.group(1) or 0), digits.group(2)))
    return key


# Ordinal words used by SESSION_LOG filing headings, e.g.
# "## 2026-08-07 (thirty-first filing) — ...". Only the DECLARATION form
# counts: a `##` heading. Prose references ("the twenty-eighth filing's
# hypothetical") are not declarations and must not be collected, or every
# back-reference would read as a duplicate.
_UNITS = [
    "", "first", "second", "third", "fourth", "fifth", "sixth", "seventh",
    "eighth", "ninth", "tenth", "eleventh", "twelfth", "thirteenth",
    "fourteenth", "fifteenth", "sixteenth", "seventeenth", "eighteenth",
    "nineteenth",
]
_TENS = {
    "twentieth": 20, "thirtieth": 30, "fortieth": 40, "fiftieth": 50,
    "sixtieth": 60, "seventieth": 70, "eightieth": 80, "ninetieth": 90,
}
_TENS_PREFIX = {
    "twenty": 20, "thirty": 30, "forty": 40, "fifty": 50,
    "sixty": 60, "seventy": 70, "eighty": 80, "ninety": 90,
}
# Cardinals, for the hundreds multiplier only: "two-hundred-and-fifth" has a
# CARDINAL "two" and an ordinal "fifth". Kept separate from _UNITS rather
# than merged, because merging would also make "two filing" parse as 2.
_CARDINAL_UNITS = {
    "one": 1, "two": 2, "three": 3, "four": 4, "five": 5,
    "six": 6, "seven": 7, "eight": 8, "nine": 9,
}
FILING_HEADING = re.compile(r"^#{2,4}\s.*?\(([a-z][a-z-]*)\s+filing\)", re.I)


def ordinal_to_int(word):
    """Return the integer for an ordinal word, or None if unrecognised.

    Returning None rather than raising is deliberate: an unrecognised word
    is reported as a parse gap, not a crash. A checker that dies on a
    heading it does not understand stops covering everything else too.

    ★ HUNDREDS ADDED 2026-08-11, ON THE DAY THEY WERE FIRST NEEDED. The
    vocabulary above stops at "ninety-nine". `SESSION_LOG.md` reached its
    hundredth filing that morning, so `hundredth`, `hundred-and-first` and
    `hundred-and-second` all parsed as None — which meant the three newest
    filings were the ONLY ones not uniqueness-checked, and the summary
    still printed "clean".

    That is the R186 shape in the tooling itself: a guard keyed on a
    vocabulary, meeting the case the vocabulary does not cover, and failing
    OPEN. Nothing broke; it just quietly stopped checking the entries most
    likely to collide (the newest ones, written by whoever is filing next).

    Accepts both `hundred-and-first` and the un-conjuncted `hundred-first`,
    because the heading text is written by hand and the checker should not
    fail over a hyphen the author chose differently.
    """
    w = word.lower()
    if w in _UNITS:
        return _UNITS.index(w)
    if w in _TENS:
        return _TENS[w]
    # The hundreds MULTIPLIER is a cardinal ("two-hundred-and-fifth"), not
    # an ordinal — only the final element takes the ordinal form. Checking
    # it against _UNITS (which holds ordinals) silently returned None for
    # every multiple of 100 above the first; caught by unit-testing the
    # function rather than by any document reaching filing 200.
    if w == "hundredth":
        return 100
    m = re.match(r"^(\w+)-hundredth$", w)
    if m:
        mult = _CARDINAL_UNITS.get(m.group(1))
        return mult * 100 if mult else None

    # "hundred-and-first" / "hundred-first" / "two-hundred-and-fifth".
    # Split off any leading multiplier, then recurse on the remainder so
    # the tens/units logic below is written exactly once.
    m = re.match(r"^(?:(\w+)-)?hundred(?:-and)?-(.+)$", w)
    if m:
        mult_word, rest = m.group(1), m.group(2)
        if mult_word is None:
            mult = 1
        elif mult_word in _CARDINAL_UNITS:
            mult = _CARDINAL_UNITS[mult_word]
        else:
            return None
        rest_val = ordinal_to_int(rest)
        # `rest` must be a sub-hundred remainder; guarding this stops
        # "hundred-and-hundredth" resolving to something.
        if rest_val is None or not 1 <= rest_val < 100:
            return None
        return mult * 100 + rest_val

    if "-" in w:
        tens, _, unit = w.partition("-")
        if tens in _TENS_PREFIX and unit in _UNITS and _UNITS.index(unit) < 10:
            return _TENS_PREFIX[tens] + _UNITS.index(unit)
    return None


def collect_filing_ordinals(text):
    """{ordinal_int: [heading_line_numbers]} plus any unparsed words."""
    seen = {}
    unparsed = []
    for lineno, line in enumerate(text.splitlines(), 1):
        m = FILING_HEADING.match(line)
        if not m:
            continue
        n = ordinal_to_int(m.group(1))
        if n is None:
            unparsed.append((lineno, m.group(1)))
            continue
        seen.setdefault(n, []).append(lineno)
    return seen, unparsed


def main() -> int:
    stats = "--stats" in sys.argv
    lines = read_lines(ROADMAP)
    secs = section_index(lines)

    passes = collect_passes(lines, secs)
    rules = collect_rules(lines)
    decision_files, decision_arch = collect_decisions()
    # Union, for the ceiling and the sanity check only.
    decisions = defaultdict(list)
    for src in (decision_files, decision_arch):
        for k, v in src.items():
            decisions[k].extend(v)

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
    for (sec, pid, qual), hits in sorted(dup_passes.items()):
        failures += 1
        how = f" (both qualified '{qual}')" if qual else " (neither qualified)"
        print(f"DUPLICATE Pass {pid} declared {len(hits)}x in section [{sec}]{how}:")
        for n, text in hits:
            print(f"    {ROADMAP}:{n}: {text}")
        print(
            "    Two entries for one Pass are legitimate ONLY when each carries a"
        )
        print(
            "    DISTINCT staged-ship qualifier, e.g. `Pass N.n (core + CLI)` and"
        )
        print("    `Pass N.n (GUI half)`. Same qualifier, or none, is a real collision.")

    # Staged ships are REPORTED, not silently accepted. A Pass filed across
    # several entries is a fact a reader of this output should see — and if
    # a qualifier was added purely to quiet the gate, this is where that
    # shows up as an entry nobody expected.
    staged = defaultdict(list)
    for (sec, pid, qual), hits in passes.items():
        if qual:
            staged[(sec, pid)].extend((n, qual) for n, _ in hits)
    for (sec, pid), parts in sorted(staged.items()):
        if len(parts) > 1:
            names = ", ".join(f"'{q}'" for _, q in sorted(parts))
            print(f"note  Pass {pid} filed in {len(parts)} stages in [{sec}]: {names}")

    dup_rules = {k: v for k, v in rules.items() if len(v) > 1}
    for num, hits in sorted(dup_rules.items()):
        failures += 1
        print(f"DUPLICATE standing rule R{num} defined {len(hits)}x:")
        for n, title in hits:
            print(f"    {ROADMAP}:{n}: {title}")

    # Files only — see collect_decisions for why §12 cannot be checked here.
    dup_decisions = {k: v for k, v in decision_files.items() if len(v) > 1}
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
    # Keys are now (section, id, qualifier) — the qualifier joined the key
    # when staged ships were recognised. The ceiling cares only about the id.
    for _, pid, _ in passes:
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
    # FAMILY level, deliberately — and this was MEASURED, not assumed.
    #
    # A minted sub-ID with no heading yet (Pass 26.3, minted 2026-08-06 for a
    # commit whose build record is still owed) does NOT surface here, because
    # family 26 is headed. The librarian found that, and it is a real gap: no
    # gate tracks a minted-but-unwritten sub-ID.
    #
    # A sub-ID-level check was measured before being built and REJECTED: 27
    # sub-IDs are mentioned without a heading, and 26 of them are legitimate
    # planned work (the whole 20.x family, 23.x, the shell redesign's own
    # 38.3-38.5). A gate that is 96% noise is the "cries wolf" failure that
    # `check-passes-filed.py`'s own first run had to be corrected for — it
    # would be ignored within a week and would then hide the 27th case too.
    #
    # So the debt is tracked by the ROADMAP entry that minted it, and by
    # `check-passes-filed.py`'s collision NOTE, and by nothing else. That is
    # stated here rather than left for the next person to discover the hard
    # way. If a cheap discriminator between "minted for existing work" and
    # "named as future work" ever appears, this is where it goes.
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
    # Filing ordinals. Reported beside the other ledgers because they are
    # used the same way — as identifiers in prose across every document.
    ord_lines = read_lines(SESSION_LOG)
    ordinals, unparsed_ordinals = collect_filing_ordinals("\n".join(ord_lines))
    # ★ A PARSE GAP IS A FAILURE, NOT A NOTE (changed 2026-08-11).
    #
    # This printed a NOTE and let the run finish "clean". On the day the
    # log reached its hundredth filing, three headings stopped parsing at
    # once and the summary still said clean — so the checker reported
    # success for a run in which it had checked less than it used to.
    #
    # "I could not read this, and I am therefore not covering it" is not a
    # note. It is the checker telling you it has a hole, and the only
    # honest exit code for that is non-zero: whoever extends the ordinal
    # vocabulary is the same person who would otherwise mint a duplicate.
    for word_line, word in unparsed_ordinals:
        failures += 1
        print(
            f"UNCHECKED  {SESSION_LOG}:{word_line}: filing heading ordinal "
            f"{word!r} not recognised — it is NOT uniqueness-checked. "
            f"Extend the vocabulary in ordinal_to_int() rather than "
            f"renaming the heading."
        )
    for num, at in sorted(ordinals.items()):
        if len(at) > 1:
            failures += 1
            print(
                f"DUPLICATE filing ordinal {num} declared {len(at)}x in "
                f"{SESSION_LOG}, lines {', '.join(str(a) for a in at)}"
            )
    if ordinals:
        top = max(ordinals)
        missing = [n for n in range(1, top) if n not in ordinals]
        if missing:
            print(
                "NOTE  filing ordinals with no heading: "
                + ", ".join(str(m) for m in missing)
            )

    print(f"  standing rules      : R{max(rules)}  -> next free is R{max(rules) + 1}")
    print(
        f"  decision records    : {max(decisions):03d} "
        f"-> next free is {max(decisions) + 1:03d}"
    )
    if ordinals:
        print(
            f"  SESSION_LOG filings : {max(ordinals)} "
            f"-> next free is {max(ordinals) + 1}"
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
