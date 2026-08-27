---
name: a-claim-about-callers-is-a-measurement
description: A doc comment asserting who calls something (or that nothing does) is a measurement that goes stale silently — re-run the command it quotes before repeating OR correcting it
metadata:
  type: feedback
---

A sentence in a comment or document that says **who calls a function** — or
that **nothing does** — is a *measurement*, not a fact about the design. It
goes stale **silently**, because nothing recompiles when it does and no gate
can see it.

**Before repeating such a claim, or correcting it, re-run the command the claim
itself quotes.** It takes a second.

**Why:** on 2026-08-27 one comment in `ocr/layer.rs` was found to have been
wrong about its own callers **three times**:

1. *"called by the CLI"* — when nothing called it.
2. *"There is NO OCR subcommand — `grep -rn "ocr" crates/pdfce-cli/src/main.rs`
   returns nothing. An R151 instance: a capability with no shell caller."*
3. *"what still has no caller is THIS one-shot"*

All three were false when written. **(3) was mine, written while explicitly
correcting (2)**, and it went into a commit whose own message criticised
exactly this behaviour. The grep quoted in (2) does not return nothing and
probably never did — `pdfce-cli` had an `ocr` subcommand *and* a
`fetch-ocr-models` subcommand, "ocr" appeared 71 times in `main.rs`, and the
function was called from `main.rs:8673`.

I then repeated (3) into a Pass 135.0 commit message as *"CLI: not shipped, and
BLOCKED rather than skipped"* — a claim to the operator, sourced from a comment
rather than from the tree. **The librarian caught it by grepping** when it was
dispatched to file the Pass.

**The mechanism, which is the transferable part:** I knew a new caller had
appeared, so I *reasoned* about what the sentence should now say instead of
re-running the check. Correcting a stale measurement by inference produces a
new stale measurement that reads as freshly verified — worse than the original,
because it carries a recent date.

**How to apply:**
- Editing any sentence about callers, reachability, "no shell caller", `R151`
  status, or "X does not exist"? **Run the grep first**, even when — especially
  when — you are correcting it.
- Same for a claim of *absence* sent to another project. On 2026-08-27 I told
  `pdfceGUI` twice, emphatically, that `unshare_form` did not exist. It shipped
  hours later and the note had to be publicly withdrawn. **A note saying "X
  does not exist" has a shelf life**; if it advises suppressing a UI control,
  say so, or the control stays suppressed for months.
- Related but distinct: [[feedback-absence-needs-an-unscoped-query]] is about a
  query being *wrongly scoped*; this is about a *correct* query's result being
  quoted long after it was run.
