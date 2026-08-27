---
name: sabotage-catches-false-comments
description: A sabotage that fails NO tests is a finding about the claim beside that code, not only about test coverage — record the line as deliberately uncovered rather than inventing a test or deleting the line
metadata:
  type: feedback
---

When a sabotage fails **no** tests, the first question is not "which test
should I add?" It is **"what did I claim about that line, and is the claim
true?"**

**Why:** 2026-08-27, `Pass 140.2`. I merged two diagnostics sources and wrote a
comment saying that merging only one of them "would leave every non-`Special`
image silent". Sabotaging the second merge failed **zero** of seven tests. The
comment was **false**: the code path it described is reached only by colour
spaces whose conversions are closed-form and record nothing, so there was never
anything for those images to say. The line's only real contribution is one
narrow probe case.

Three possible responses, and only one is honest:

- ✗ **Add a test to cover it.** Would have required a fixture for a transform
  that *loads* and then *fails to evaluate at zero* — buildable, but it would
  have been written to defend a sentence rather than a behaviour.
- ✗ **Delete the line.** The narrow case is real; deleting it trades a true
  belt-and-braces for a tidy diff.
- ✓ **Fix the claim and record the line as deliberately uncovered**, naming
  exactly which case it serves and why no fixture exercises it.

**How to apply:** run sabotages expecting some to survive, and read a survivor
as a claim audit. This complements [[feedback_a_test_can_become_vacuous_later]]
(where sabotage distinguishes *why* a test passes) — here it distinguishes
whether a comment is describing the code or describing an assumption.

★ Note the asymmetry with the rest of this project's culture: an uncovered line
recorded **as** uncovered, with its reason, is stronger than a test written to
make a number go up. Say so in the commit message too; a reader who sees
"3 sabotages, 2 failed tests" and no explanation will assume the third was an
oversight.

Related: [[feedback_a_claim_about_callers_is_a_measurement]],
[[feedback_gates_i_owe_myself]].
