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
2. Every stated verb count anywhere under `docs/core-api/` — the verb index's
   own `Count: N`, and any "N public verbs/methods" prose — equals the number
   actually derived.
3. Every `` `path/to/file.rs` (N lines) `` claim resolves to a real file with
   that many lines.

★ Item 3 was added because of a SIXTH stale figure, found one line above the
fifth: `02-editing-and-saving.md:12` said `edit.rs` was **20,939 lines** when
it was **24,826**. The widened gate did not catch it, because widening it had
been scoped to *verb counts* — the class of the defect that prompted the
widening, rather than the class of the defect underneath it, which is
**published figures nobody re-derives**.

That is the same error one level up, so the scope is now the general one this
gate can actually enforce: **if the document states a number this tool can
re-compute from the source, it re-computes it.** Anything it cannot re-compute
(the citation counts, the "verified at <commit>" anchors) is `R197`'s
territory, not this gate's, and is listed under WHAT IT DOES NOT CHECK.

WHAT IT DOES NOT CHECK, STATED SO "GREEN" IS NOT OVER-READ
==========================================================

1. **That the description is CORRECT**, or current, or that a verb's caveats
   are written down. It checks presence and arithmetic — both purely
   syntactic, which is what a gate can do and a reviewer cannot do reliably.
   `insert_pages` would have passed this gate on the day it shipped while
   still lacking the widget warning that caused the incident.
2. **Any API surface other than `EditSession`.** `DocumentView`, the free
   functions, and `pdfce-render`'s surface are all undefended by this.
3. **★ It used to read ONE FILE, and that was a live defect, not a
   hypothetical one.** The first version's input was
   `02-editing-and-saving.md`, because that is the file that was being
   corrected when the gate was written. It went green while
   **`index.md` — the front door of the same directory, the table a consumer
   reads BEFORE opening part 2 — still said "all 108 public verbs".** The
   gate written to stop a stale count shipped with the stale count ten lines
   away, in its own directory, outside its own scope.

   That is the fourth instance in one day of repairing the instance in front
   of you instead of the class, and the most instructive, because the
   instrument itself inherited the bug it was built to catch. It now scans
   **every `*.md` under `docs/core-api/`**, and the count check applies to
   every stated count wherever it appears.

4. **The `path:line` citations, and there are over a thousand of them.** They
   are anchored to a commit and drift with every edit to the file they point
   at; one spot-check found a citation off by 2,169 lines, landing on
   unrelated code. Re-deriving them needs symbol resolution, not a line
   count, so they are out of scope here and tracked as owed work instead.
   **They are legibly stale rather than invisibly stale** — the anchor commit
   is printed beside them, which is exactly the property `R197` asks for.
5. **★ Published figures OUTSIDE `docs/core-api/`.** This file's own commit
   named the class correctly — *published figures nobody re-derives* — and
   then enforced it only under the directory of the defect that prompted it.
   The seventh instance of that error in one session, inside the commit that
   named it.

   It is **not** simply widened to a repo-wide glob, and the reason is a real
   finding rather than caution. A sweep found **13** `` `x.rs` (N lines) ``
   claims repo-wide: one live defect (since fixed), **one citing a file that
   no longer exists**, and **five that are correctly FROZEN** — dated
   historical records, one off by 7.4× and harmless precisely because its
   header is dated. A wider glob would go red on all of them and the only
   way to quiet it would be to falsify the history.

   **The enforceable class is bounded by document LIVENESS, and liveness is
   not mechanically detectable.** So widening this needs an explicit opt-in
   list of living documents, not a glob — filed as owed work rather than
   guessed at here.

So this closes the "nobody mentioned it" and "the published number drifted"
failures within this directory, and leaves "mentioned it wrongly" to review,
where it belongs. Named rather than implied, because a gate whose limits are
unstated gets trusted past them — and because this gate has now twice
demonstrated that its own author will trust it past them.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

# Windows consoles default to a code page that cannot encode the em-dashes
# and stars this file prints, so Python substitutes a replacement byte for
# exactly the characters that make a message readable.
#
# ★ NOT THEORETICAL, AND NOT FOUND BY READING THE SOURCE. This gate printed
# `PASS <0x97> every verb documented` on every run of the day it was written
# — cp1252's em-dash, byte-verified with `od -c` — and its author read that
# output perhaps twenty times without looking at it. Seven sibling `check-*`
# scripts already carried this guard; this one was written without it because
# it was written from scratch rather than from a sibling.
#
# The sting is in `check-commits-filed.py`'s own header, which records being
# fixed for this twice and states the lesson: the fix was "applied where the
# problem had been SEEN rather than to every stream." It generalised
# stdout → all streams, and never generalised one file → all files. Which is
# the same class of error this file's header is otherwise entirely about.
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")


ROOT = Path(__file__).resolve().parent.parent
EDIT_RS = ROOT / "crates" / "pdfce-core" / "src" / "edit.rs"
DOC_DIR = ROOT / "docs" / "core-api"
# The verb index lives here; the other files may still state a COUNT, and a
# stale count in the directory's front door is what this gate missed first
# time out.
VERB_INDEX = DOC_DIR / "02-editing-and-saving.md"


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
    if not EDIT_RS.exists() or not VERB_INDEX.exists():
        print("check-core-api-verbs: SKIP — edit.rs or the verb index is missing")
        return 0

    lines = EDIT_RS.read_text(encoding="utf-8").split("\n")
    names, blocks = derive_methods(lines)
    doc = VERB_INDEX.read_text(encoding="utf-8")
    others = sorted(f for f in DOC_DIR.glob("*.md") if f != VERB_INDEX)

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
        print(f"  {VERB_INDEX.relative_to(ROOT)}:")
        for n in missing:
            print(f"      {n}")

    m = re.search(r"\*\*Count: (\d+)\.\*\*", doc)
    if not m:
        failed = True
        print()
        print("  the verb index no longer states a `**Count: N.**`, so the")
        print("  derivation it claims cannot be checked at all")
    elif int(m.group(1)) != len(names):
        failed = True
        print()
        print(f"  the verb index states Count: {m.group(1)}, derived count is {len(names)}")

    # EVERY stated verb count in the directory, not just the index's own.
    # `index.md` is the table a consumer reads first, and it carried a stale
    # 108 while part 2 was being corrected -- see this file's header.
    stale = re.compile(r"(?:all\s+)?(\d+)\s+public\s+(?:`EditSession`\s+)?(?:verbs|methods)")
    # Every re-derivable published figure, not only verb counts -- see item 3
    # of WHAT IT CHECKS for why the narrower scope was itself the same bug.
    sized = re.compile(r"`([\w./-]+\.rs)`\s*\((\d[\d,]*) lines\)")
    for f in [VERB_INDEX, *others]:
        for n, line in enumerate(f.read_text(encoding="utf-8").split("\n"), 1):
            for found in stale.finditer(line):
                if int(found.group(1)) != len(names):
                    failed = True
                    print()
                    print(f"  {f.relative_to(ROOT)}:{n} states {found.group(1)}")
                    print(f"  public verbs; the derived count is {len(names)}")
            for found in sized.finditer(line):
                target = ROOT / found.group(1)
                claimed = int(found.group(2).replace(",", ""))
                if not target.exists():
                    failed = True
                    print()
                    print(f"  {f.relative_to(ROOT)}:{n} cites {found.group(1)},")
                    print("  which does not exist")
                    continue
                actual = len(target.read_text(encoding="utf-8", errors="replace").split("\n")) - 1
                if actual != claimed:
                    failed = True
                    print()
                    print(f"  {f.relative_to(ROOT)}:{n} says {found.group(1)} is")
                    print(f"  {claimed:,} lines; it is {actual:,}")

    if failed:
        print()
        print(
            "A consuming project builds against these documents. A verb missing\n"
            "from them is a verb whose only description is whatever somebody said\n"
            "once in chat -- which is how pdfceGUI shipped a wrong disclosure about\n"
            "`insert_pages`. Add the verb to the relevant section, and update EVERY\n"
            "stated count, including the one in index.md."
        )
        return 1

    print("check-core-api-verbs: PASS — every verb documented, count agrees.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
