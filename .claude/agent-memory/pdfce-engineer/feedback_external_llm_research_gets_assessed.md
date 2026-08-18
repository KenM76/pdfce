---
name: external-llm-research-gets-assessed
description: Ken wants external LLMs / web-search agents used to offload research and SAVE HIS TOKENS — findings must still be confirmed against pdfce's own docs before acting
metadata:
  type: feedback
---

**Offload research; keep the judgment.** Ken, 2026-08-18, confirming after I
had declined a Gemini consultation: *"I think you should use Gemini to aid you
in your work if it can help reduce token useage on my plan. You can disagree
with it, and you should confirm its claims and plans… Gemini is probably a lot
more capable and faster at web searching than you are… Save your tokens for
the actual coding if you can."*

**Why:** the plan's token budget is his, and search is the cheapest thing to
delegate. It is not a statement that the outside answer is authoritative — he
said in the same breath to disagree with it and to confirm it.

**How to apply:**
- **Default to delegating open-ended web research**, especially "what do other
  engines actually do" questions. Reserve my own context for code.
- Chrome/browser tools are **not always loaded** in this session type — check
  with `ToolSearch` before promising a Gemini round-trip. When they are
  absent, a **subagent with WebSearch/WebFetch** achieves the same saving:
  the searching burns the subagent's context, and only its briefing reaches
  mine. `general-purpose`, `pdfce-spec-librarian` and
  `pdfce-acrobat-librarian` all have web access. See
  [[dispatch-subagents-without-asking]].
- **Confirm before acting.** Project rule 1 is untouched by this: spec-governed
  behaviour is never implemented from any LLM's recall, so anything actionable
  is re-sourced against `D:\Dev\Rag-Specialized\PDF_Spec\`.
- **Grade every concrete recommendation against the hard constraints first** —
  `ARCHITECTURE.md` §3 (no GPU/windowing in the engine crates), the wasm32 CI
  gate (rules out C-library bindings), rule 13 licence classification, and the
  GPL/AGPL-is-behavioural-reference-only rule. The 2026-08-18 Gemini paste was
  mostly right and mostly already in the RAG at higher resolution, but two of
  its four crate picks (`lcms2`, `vello`) violated those constraints outright.
- **Write rejections into a pdfce doc with the reason** so a future session
  meeting the same advice does not re-derive it — done in
  `docs/compositor-plan.md` §6.
- Say plainly when an outside source is at lower resolution than the RAG, and
  equally plainly when it converges independently on the same conclusion —
  that convergence is real evidence. See
  [[feedback-two-modes-one-pattern-is-one-measurement]].
