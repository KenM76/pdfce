---
name: wrapped-string-literals-lose-backslashes
description: Never patch a Rust line-continuation backslash via a heredoc/python -c — it becomes a literal \n. Use the Edit tool. Happened four times in one session.
metadata:
  type: feedback
---

# Never patch a Rust `\`-continuation through a heredoc. Use Edit.

Rust wraps a long string literal with a **backslash at end of line**, which
strips the newline and the next line's leading whitespace:

```rust
"pdfce-cli: printing is available on Windows only in this build \
 (docs/decisions/003-distribution-posture.md §4.1)"
```

Patching that construct with `python - <<'PYEOF'` or `sed` **reliably
corrupts it**, in one of two ways:

1. The backslash is **lost**, so the two fragments concatenate with the
   second line's indentation baked in — a shipped message with a
   ten-space gap mid-sentence.
2. The backslash survives but the newline does not get escaped through the
   layers, so the source ends up containing a **literal `\n` two-character
   sequence** in the middle of the format string — which Rust then renders
   as a REAL newline at runtime.

**Why:** the payload crosses shell → heredoc → Python string literal → file,
and each layer has its own opinion about `\`. Getting `\\\n` to arrive as
backslash-plus-newline requires counting escapes correctly through all four,
and I have now got it wrong four times in a single session.

**How to apply:** the moment a patch touches a `\` at end of line inside a
Rust string — or any Rust escape at all — **stop and use the Edit tool**, or
write a script *file* with the Write tool (no shell layer). Never `sed`, never
`python - <<EOF`, never `echo`.

## The failure is invisible to every gate except one

`cargo build`, `cargo clippy` and `cargo fmt --check` are all **completely
silent** on both corruptions: the literal is still a valid literal, it just
says the wrong thing. Nothing type-checks a sentence.

The only thing that has ever caught it is `pdfce-cli`'s
**stable-stdout-line test**, and only because that test asserts stdout is
*exactly one LF-terminated line*. Outside that one contract the corruption
ships.

**So: after any edit to an operator-facing string, grep for it.**

```bash
# literal \n embedded mid-string (the runtime-newline form)
grep -rn '\\\\n' crates/*/src/*.rs | grep -v '"\\\\n"'
# ragged multi-space gap (the lost-backslash form)
grep -rn '"[^"]*[a-z,.] \{4,\}[a-zA-Z(]' crates/*/src/*.rs
```

The second sweep's hits are mostly `assert!` messages in tests, which are
harmless; the ones that matter are anything reachable by an operator.

## The specific trap: a "fix" that re-introduces the bug

On 2026-08-18 I found the ten-space-gap form in `cmd_list_printers`' message,
fixed it **with a heredoc**, and thereby created the literal-`\n` form in the
same string. It shipped in that state and was caught hours later by the
stable-line test failing for an unrelated reason. **Repairing this bug with
the tool that causes it is the actual trap**, not the original mistake.

Related: [[windows-paths-need-literal-edits]] — same root cause (backslashes
crossing shell layers), same fix (Edit, or a written script file).
