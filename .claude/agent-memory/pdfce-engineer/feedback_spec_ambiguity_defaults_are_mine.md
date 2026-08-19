---
name: spec-ambiguity-defaults-are-mine
description: Every spec contradiction/ambiguity becomes a setting AND I pick the default myself — Ken 2026-08-19 explicitly refused to be asked
metadata:
  type: feedback
---

Any **contradiction or ambiguity in the PDF specs** gets an option in settings,
and **the default is my best guess**. Do not ask Ken to choose the default.

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
