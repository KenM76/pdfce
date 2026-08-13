# `pdfce-core` — consumer API map

**For a shell being built against this crate from outside this repository**
(the new GUI project at `D:\dev\pdfceGUI`, a future WASM shell, any other
consumer). Written 2026-08-13, verified against the source at `e6609bc`.

This is **not** rustdoc. Rustdoc already exists and is good at *"what does
this function do."* These three files answer the question rustdoc cannot:
***"I want to do X — what do I call, in what order, and what will bite
me?"*** Every section leads with a grep-able *"I want to…" → "call this"*
index and ends with **Traps**.

| file | covers | size |
|---|---|---|
| [`01-reading-and-model.md`](01-reading-and-model.md) | loading, the COS object model, pages, content streams, text extraction, fonts, vector picking/snapping, filters, colour, navigation, metadata | 2,215 lines · 382 verified citations |
| [`02-editing-and-saving.md`](02-editing-and-saving.md) | `EditSession` end to end — **all 108 public verbs**, the command/undo contract, the dirty set, the save path, the guard/refusal model, `EditError`'s 57 variants | 1,419 lines · 128 citations |
| [`03-capabilities.md`](03-capabilities.md) | ce dimensions, forms, markup, redaction, OCR, print/imposition, rasterising — each with **★ what the UI must disclose** | 1,982 lines |

## Read these four things before writing any code against this crate

1. **Coordinate spaces.** PDF user space is **y-UP**; image and screen
   space are **y-DOWN**. Every geometry function states which it takes.
   Getting it wrong is silent — the page looks perfect until someone
   selects a line and gets a different one.
2. **Hit-test and snap tolerances are PAGE-space radii, and nothing checks
   them.** Pass raw screen pixels and it compiles, runs, and merely drifts
   with zoom.
3. **Rule 4, "fuzzy never sneaky."** Anything pdfce *inferred* — an OCR
   result, a best-fit circle and its residual, a snapped point, a
   substituted font, a near-parallel classification — must be visible
   **before** it becomes document state, and rejectable without undoing
   anything else. Part 3 names, per capability, exactly which values are
   inferences. A shell that does not know which ones they are ships a
   rule-4 violation and will not find out.
4. **A returned count is not always the count you want to show.** The
   worked example is `set_group_style`, which returns members
   *regenerated* (all of them), not members that will visibly *move*.

## How these were built, and what that means for trusting them

Every symbol was enumerated from source and its `file:line` **machine-checked
against HEAD** — a pass that caught 23 wrong line numbers and one false
claim about a re-export. Anything that could not be verified is written as
`UNVERIFIED — <what to check>` rather than guessed: **an honest gap is
useful; a confident wrong answer costs a day.** 18 such markers survive in
part 2 alone, and they are content, not omissions.

**Source is authoritative when these disagree with it.** They are a dated
snapshot of a moving crate; re-verify anything load-bearing before relying
on it, and prefer a `file:line` citation over prose.
