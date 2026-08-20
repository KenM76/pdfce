---
name: spec-ambiguity-defaults-are-mine
description: Any two-defensible-answers question becomes an option AND I pick the default myself — Ken refused to be asked, twice, and the second time it was not a spec question at all
metadata:
  type: feedback
---

**Any question with two defensible answers** gets BOTH as options, and **the
default is my best guess at what a normal operator would expect**. Do not ask
Ken to choose the default.

★ **WIDENED 2026-08-20, and the widening is the point.** This started as a rule
about *spec* ambiguity. It is not limited to that. Ken, 2026-08-20, verbatim:

> *"for you two questions, make things work both ways as options. default it to
> your best guess as to what would be normally expected."*

**Neither of those two questions was a spec ambiguity.** They were pure
interaction design on an unbuilt verb — *should a transform act on a mixed
selection?* and *what happens when an operator drags a resize handle through
zero?* PDF has nothing to say about either. He applied the identical posture
anyway, unprompted, which makes the rule about **decision-making authority**
rather than about specs.

Note also *"what would be normally expected"* — the default is chosen from
**operator expectation**, not from what is safest to implement, easiest to
defend, or most conservative.

**Why:** Ken, 2026-08-19, verbatim: *"any contradictions or ambiguities in the
specs get an option in settings with the default as YOUR best guess. do not ask
me for the default as you know more about this than I ever will."*

This **strengthens** [[fix-bugs-on-discovery-and-make-ambiguity-a-setting]]
(2026-08-08, *"never hard-code a choice the standard leaves open"*). That one
established the *setting*. This one assigns the *default* to me, and does so by
explicitly declining a question I had just asked.

**What triggered it:** I found a genuine spec contradiction — zero-filling a
subtractive compositing buffer inverts every luminosity mask, and §8.6.8 gives
`ICCBased` all-zeros while `DeviceCMYK` gets `[0 0 0 1]`, so the *same ink set*
gets opposite defaults depending on how it is declared. I wrote *"I'll bring
you a recommendation when Stage B needs it rather than deciding it silently."*
He overruled that. Deciding it and disclosing it **is not** deciding it
silently — the setting plus its documented reasoning is the disclosure.

**How to apply:**

- **A two-defensible-answers question is never an open operator question.**
  Not a spec ambiguity, not an interaction-design choice, not a "which of these
  would you prefer". Ship both, pick the default, write down why, move.
- ★ **This includes questions asked of a CONSUMING PROJECT, not just of Ken.**
  On 2026-08-20 I ended a reply to `pdfceGUI` by asking them both questions
  above. Ken answered them himself before they read it, and I had to amend the
  reply in place to say *"do not answer them"*. The near-miss is the lesson:
  **a question sent to a consuming project is a commitment to act on their
  answer**, and had it been read first I would have had two answers to
  reconcile — one of them from someone with no standing to make the call.
  Decide it, tell them what you decided and why, and offer the option.
- A spec ambiguity is **never** a blocker, never an open operator question, and
  never a reason to defer a Pass. Decide, default, document, move.
- Still **write down the reasoning** — which readings exist, which I picked,
  why, and what the other reading would produce. The operator delegated the
  *choice*, not the *record*. Rule 4 (fuzzy never sneaky) is untouched.
- Keep the ambiguity's register id (`A52`, `SEP-A3`, `NT-A1`, `OP-A3`, …) in
  the setting's doc comment so the setting and the corpus entry find each
  other.
- The **licence/legal** carve-out is unchanged and is NOT a spec ambiguity:
  copyleft dependencies, publishing, releasing still go to Ken (project rules
  8 and 13). "Which reading of §11.4 is right" is mine; "may we ship this
  CC-BY-SA model file" is his.
- Where the two editions differ (1.7 vs 2.0) that is usually **not** an
  ambiguity — it is a version-scoped fact, and the setting should be keyed on
  the document's version rather than on operator taste. Only genuine
  intra-edition contradictions need a knob.

**Related:** [[exceed-the-parity-reference-when-you-can]] — same delegation
shape. Ken sets the goal; the technical call is mine, with the divergence
recorded.
