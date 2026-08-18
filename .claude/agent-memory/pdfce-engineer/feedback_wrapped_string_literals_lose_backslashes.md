---
name: wrapped-string-literals-lose-backslashes
description: Patching Rust multi-line string literals through heredocs/scripts silently drops the trailing backslash, shipping error messages with 10-space gaps mid-sentence
metadata:
  type: feedback
---

When a Rust string literal is wrapped across source lines, the trailing
`\` is what suppresses the next line's indentation. Patch that literal
through a shell heredoc, a Python string, or any other layer that eats one
level of escaping, and the `\` disappears — the code still compiles, the
tests still pass, and the shipped message reads
`"…enforced          (ISO 32000-1 §12.8.4…"`.

**Why:** it bit four literals in one session (2026-08-17, pdfce
`set_markup_style` / `set_media_box`), and a grep then found **two
pre-existing ones shipped since `95c3416`** — an attachment refusal and
the DocMDP certification refusal — plus two more in
`text_edit/encoding.rs`. Nothing caught them for months, because nothing
*can*: `cargo fmt` does not reflow string contents, clippy has no lint for
it, and a test that asserts on the message compares it against another
copy of the same broken string. It surfaced only by **running the CLI and
reading the output**.

**How to apply:**

1. After patching any file containing wrapped literals, grep for the
   signature before declaring done:
   `grep -rn '"[^"]*[a-z,.] \{3,\}[a-zA-Z]' crates/ --include=*.rs`
   Filter out `///` doc lines and deliberate column alignment
   (`"skip   field="`); everything else is a hit.
2. Prefer the Write tool over `python - <<'PY'` heredocs when the payload
   contains backslashes — this is the same hazard as
   [[windows-paths-need-literal-edits]], which already says *any*
   backslash breaks heredoc patching, not just path separators. This is
   the Rust-escape half of that rule, and it was learned again the hard
   way.
3. A message duplicated in two modules will have a mirror test
   (`add_text_certification_message_is_a_verbatim_mirror_of_edit_error`).
   Fixing one copy turns it red — that is the test working, not a
   regression. Fix both.
