---
name: verify-each-instance-not-the-class
description: Running one instance of a change and generalising the result to its siblings is how a defect ships beside a verified twin — verify each, or use a gate that does
metadata:
  type: feedback
---

When a change produces **several instances of the same kind of thing**, an
oracle run against one of them does not cover the others. Run each, or find
an instrument that covers the class.

**Why:** on 2026-08-18 I added two `RenderError` variants twelve lines apart
in one commit. I verified `PageNotRecordable`'s message by *running the
binary and reading it* — the right method, and the one my own notes prescribe
for string literals. Then I treated the method as discharged for the commit
and never ran `DisplayListStale`. It shipped with ten literal spaces baked
into the middle of its sentence, from a `\` line-continuation the patch
tooling ate.

The librarian found it in a sweep. The shape is what makes it worth a memory
rather than a shrug: **the verified twin is what creates the confidence.**
Having watched one message render correctly, I was not careless about the
second — I believed it was already covered, which is a different and more
durable error.

This is the same failure the `check-ledger-numbers.py` star anchor has now
made twice: the first fix accepted `★ ` because `★ ` was the spelling that
had been seen, and `★★ ` stayed invisible for another eight months. Repairing
the instance in front of you is not repairing the class.

**How to apply:**
- Ask *"how many of these did I just create?"* before declaring an oracle
  discharged. Two is enough for this to bite.
- Prefer a **gate over a habit** when the class is syntactic. Reading each
  message aloud does not scale; `tools/check-string-gaps.sh` does, and it
  found 44 more the same afternoon.
- When a gate is anchored on a pattern, make the pattern accept every
  spelling of the thing it matches, not the one in front of you today.

Related: [[windows-paths-need-literal-edits]] (the mechanism that eats the
backslash), [[gates-i-owe-myself]] (the gates I skip), and
[[two-modes-one-pattern-is-one-measurement]] — same family: agreement
between things produced the same way is not verification.
