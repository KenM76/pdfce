---
name: a-sweep-is-only-as-good-as-its-spelling
description: When sweeping for a stale CLAIM, punctuation is the failure surface — narrow the file set and widen the pattern, never the reverse; two instances in one day, one of them inside a gate that argued it was immune
metadata:
  type: feedback
---

When sweeping for a stale claim, **narrow the FILE SET and widen the PATTERN**.
Never the reverse. The words of a claim are stable; the **punctuation around
them is what a writer varies**, and that is where a sweep goes blind.

**Why (2026-08-29, twice in one day, both in colour work):**

1. `pdfce-librarian` swept for the claim *"the §8.6.7 ambiguity"* after a Pass
   retired it, found **six survivors nobody else had seen** — excellent work —
   and **missed a seventh**, because it grepped the phrase *with the section
   sign* and one line spelled it without one.
2. Same day, `tools/check-ledger-numbers.py` printed *"decision records: 103 →
   next free is 104"* while **decision 104 already existed**. Its pattern
   required whitespace between the word and the digits; the declaration spells
   it `` decision `104` `` — **the project's own house style**. Third
   occurrence in that one tool, and nothing else detects a duplicate decision
   number, so it was actively inviting one.

★ **The compounding detail in #2 is the transferable part.** The pattern sat
under a **forty-line comment arguing this could not happen**: *"it cannot
under-report, whatever spelling a future filing invents."* That argument was
sound about the **source** (any mention, anywhere in the file) and completely
silent about the **separator**. ⇒ **A claim in a comment is not a check.** The
better the argument in the comment, the more convincing the blind spot.

★★ **And the fix's own first run caught the fix.** I widened the pattern, added
a `_self_check()` asserting it against real spellings — and the assertion
**failed immediately** on `**decision _103_**`, because `_` is a *word*
character so `\b` never matched. **Underscore is markdown italics, i.e. one of
the very wrapping styles the widening existed to tolerate: the bug and the fix
were the same character.** A checker that asserts its own coverage is the only
thing that would have caught that.

**How to apply:**
- A global grep for a bare word returns dozens of correct uses and is
  unreadable. The **same grep over six files is six seconds of reading** — so
  scope by file, not by phrase.
- Never end a markdown-tolerant number pattern in `\b`; use `(?!\d)`.
- If a checker makes a coverage claim, **make it assert that claim at run
  time**, with the spelling that broke it copied verbatim into the list.
  Exit `2`, not `1` — a checker that cannot see its own subject has found a
  fault in *itself*, which is a different answer to a different person.
- Related: [[feedback_absence_needs_an_unscoped_query]] is the same shape with
  the scope in the path rather than in the pattern, and
  [[feedback_a_gate_that_underreports_looks_green]] is what this produces.
