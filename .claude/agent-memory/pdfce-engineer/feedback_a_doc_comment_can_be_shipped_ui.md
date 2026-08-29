---
name: a-doc-comment-can-be-shipped-ui
description: In clap-derive CLIs a doc comment IS the --help text, so an orphaned or missing one ships as blank operator-facing UI that no compiler, linter or test notices — grep for the class, don't wait to spot instances
metadata:
  type: feedback
---

**In a `clap`-derive CLI, a doc comment is not documentation — it is shipped
operator-facing UI.** `clap` turns the `///` on a `Command` variant into that
subcommand's `--help` description. A variant with no doc comment ships a
**blank** description, in the subcommand list and at the top of its own
`--help`.

**Why this needs its own rule:** *nothing* catches it. Not the compiler, not
`clippy`, not `missing_docs` (these are private items in a binary crate), and
**no test, because no test reads help text**. The build is green and the
operator sees an empty line.

**How to apply:** when a doc-comment defect class shows up anywhere, immediately
ask whether any of that project's doc comments are *rendered to a user*. If
they are, write the mechanical check rather than continuing to find instances by
reading. `pdfce` has `tools/check-clap-help.py`; the same shape applies to any
`clap`-derive tool, and to `#[derive(Parser)]` field docs (which become
per-flag help).

## The measurement that produced it

2026-08-29. `pdfce` had found **six** instances of doc-comment orphaning by eye
over several weeks — a splice anchored on `pub fn name(` lands *inside* the
preceding item's doc block, welding two together. The sixth was the first that
was operator-facing: `ExtractText`'s entire help sat 800 lines away on
`ListOutline`, so `list-outline --help` printed the *text-extraction*
description and `extract-text --help` printed nothing.

A gate written in twenty minutes found **two more within seconds**
(`print-preview`, `render-page`), both shipping blank, **neither caused by a
splice**. That is the load-bearing part: the class had **more than one cause**,
so the existing remedy — *"insert after a closing brace, never before a named
anchor"* — could never have closed it, and no amount of careful reading would
have either.

## ★ What did NOT work, so it is not re-derived

A structural detector for the **weld itself** — *"a doc line whose predecessor
is non-empty and whose successor is a blank `///`"* — produced **8,136
candidates** across the crate. That is also the shape of every ordinary
paragraph ending. Abandoned rather than shipped noisy.

⇒ The gate catches the **donor** of a weld (an item left with nothing), never
the **recipient** (an item left with two). Six of eight instances left a donor;
two did not. **Ship the half that is exactly checkable and state the limit in
the script's header** rather than shipping a fuzzy check for the whole class.

Related: [[feedback_inserting_before_an_anchor_orphans_its_doc_comment]],
[[feedback_a_gate_that_underreports_looks_green]],
[[feedback_an_unticked_box_is_unfalsifiable]].
