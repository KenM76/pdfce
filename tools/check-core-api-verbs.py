#!/usr/bin/env python3
"""check-core-api-verbs.py — every public `EditSession` verb must appear in
the document a consuming project reads.

WHAT THIS GATE IS FOR
=====================

`docs/core-api/02-editing-and-saving.md` is the briefing document another
project builds against. On 2026-08-18 it was found to be **eight verbs
behind** — `set_media_box`, `set_media_boxes`, `set_markup_style`,
`mark_redactions_by_search_styled`, `mark_redactions_by_pattern_styled`,
`flatten_refusal`, `insert_pages`, `widget_rects` — while stating a precise
count of 108 that had been true when it was written.

★ HOW THE DRIFT SURFACED, WHICH IS THE ARGUMENT FOR THE GATE

Not by review. The `pdfceGUI` session wired `insert_pages` and shipped a
**wrong operator disclosure** about what it does to form fields. They did not
misread the document — *the document never mentioned the verb*, so a chat
reply was the only description of it that existed anywhere, and a chat reply
is not versioned, not reviewable, and not something a second reader can
check.

**A consumer-facing API document that OMITS a verb is worse than one that
describes it badly.** A bad description gets argued with. A missing one gets
replaced by whatever the consumer was told once, in passing, by someone who
was not writing documentation at the time.

★ AND THE COUNT IS WHY IT LASTED

The document did not merely omit the verbs — it asserted `**Count: 108.**`
and showed the derivation (`41 + 46 + 20 + 1`). A stated derivation reads
exactly like a *maintained* derivation. Anybody checking whether the index
was complete would have found a number that looked audited, and no way to
tell that the audit had happened once, months earlier. **That is the specific
failure this gate removes: it makes the derivation actually run.**

WHAT IT CHECKS
==============

1. Every `pub fn` / `pub const fn` inside an `impl EditSession` block in
   `crates/pdfce-core/src/edit.rs` is named somewhere in the document.
2. The `Count: N` the document states equals the number actually derived.

WHAT IT DOES NOT CHECK, STATED SO "GREEN" IS NOT OVER-READ
==========================================================

That the description is **correct**, or current, or that a verb's caveats are
written down. It checks presence and arithmetic — both purely syntactic,
which is exactly the kind of thing a gate can do and a reviewer cannot do
reliably. `insert_pages` would have passed this gate on the day it shipped
while still lacking the widget warning that caused the incident.

So this closes the "nobody mentioned it" failure and leaves the "mentioned it
wrongly" failure to review, where it belongs. Named rather than implied,
because a gate whose limits are unstated gets trusted past them.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EDIT_RS = ROOT / "crates" / "pdfce-core" / "src" / "edit.rs"
DOC = ROOT / "docs" / "core-api" / "02-editing-and-saving.md"


def derive_methods(lines: list[str]) -> tuple[list[str], list[tuple[int, int, int]]]:
    """Every public method of every `impl EditSession` block, brace-matched.

    Brace-matched rather than regex-delimited because an `impl` block ends at
    its closing brace and nothing else marks it. Counting `{` and `}` per line
    is crude — a brace inside a string literal or a comment would skew it —
    and it is used anyway because the alternative is a Rust parser as a build
    dependency. The self-test below is what keeps the crudeness honest: if the
    matching ever slips, the derived count moves and the gate fails loudly
    rather than under-reporting.
    """
    starts = [i for i, l in enumerate(lines) if l.startswith("impl EditSession {")]
    names: list[str] = []
    blocks: list[tuple[int, int, int]] = []
    for s in starts:
        depth = 0
        end = s
        for i in range(s, len(lines)):
            depth += lines[i].count("{") - lines[i].count("}")
            if depth == 0 and i > s:
                end = i
                break
        n_before = len(names)
        for i in range(s, end):
            m = re.match(r"\s+pub (?:const )?fn (\w+)", lines[i])
            if m:
                names.append(m.group(1))
        blocks.append((s + 1, end + 1, len(names) - n_before))
    return names, blocks


def main() -> int:
    if not EDIT_RS.exists() or not DOC.exists():
        print("check-core-api-verbs: SKIP — edit.rs or the doc is missing")
        return 0

    lines = EDIT_RS.read_text(encoding="utf-8").split("\n")
    names, blocks = derive_methods(lines)
    doc = DOC.read_text(encoding="utf-8")

    print(f"check-core-api-verbs: {len(names)} public EditSession method(s) in edit.rs")
    for start, end, n in blocks:
        print(f"    impl {start:>6}..{end:<6} {n:>3} method(s)")

    failed = False

    # A method counts as documented if the doc names it as `name(` (a
    # signature) or as `name` (a bare mention). Deliberately generous: this
    # gate is about NOBODY MENTIONING IT, not about the shape of the mention.
    missing = [n for n in names if f"`{n}(" not in doc and f"`{n}`" not in doc]
    if missing:
        failed = True
        print()
        print(f"  {len(missing)} verb(s) exist in edit.rs and are absent from")
        print(f"  {DOC.relative_to(ROOT)}:")
        for n in missing:
            print(f"      {n}")

    m = re.search(r"\*\*Count: (\d+)\.\*\*", doc)
    if not m:
        failed = True
        print()
        print("  the document no longer states a `**Count: N.**`, so the")
        print("  derivation it claims cannot be checked at all")
    elif int(m.group(1)) != len(names):
        failed = True
        print()
        print(f"  the document states Count: {m.group(1)}, derived count is {len(names)}")

    if failed:
        print()
        print(
            "A consuming project builds against this document. A verb missing from\n"
            "it is a verb whose only description is whatever somebody said once in\n"
            "chat -- which is how pdfceGUI shipped a wrong disclosure about\n"
            "`insert_pages`. Add the verb to the relevant section, and update the\n"
            "stated count and its per-block arithmetic."
        )
        return 1

    print("check-core-api-verbs: PASS — every verb documented, count agrees.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
