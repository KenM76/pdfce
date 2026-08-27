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

## ★★ THE TRIGGER, NAMED — it is not "editing a literal", it is WRITING RUST THROUGH A HEREDOC

Recorded 2026-08-26 after doing it **twice more in one day**, both times while
believing the rule did not apply.

**The Bash tool eats exactly one backslash level.** `python - <<'PY'` looks
quoted and safe, and it is — for the *shell*. What arrives at Python is
already one level down, so `"\\n\\"` in what I typed becomes `\n\` in the
file only if I counted a level I did not know was there. Both misses:

1. A `format!` string in `main.rs` — I appended a continuation line and the
   trailing `\` vanished, splitting one stdout key across two lines.
2. `settings/mod.rs` — the same thing inside the paragraph a *previous*
   commit had just repaired.

**The generative rule, which is stronger than "be careful with literals":**
*if a heredoc's payload is Rust source, do not use a heredoc.* Write the
script to a file with the Write tool and run it, or use Edit directly. The
payload does not have to contain a literal for this to bite — it bites on any
`\` anywhere, including in a doc comment, a path or a regex.

The Write-a-script-file route works and is now the default for anything that
generates code: the Write tool does no escaping pass at all.

## ★ 2026-08-26 — it happened AGAIN, in the repair commit again, and the gate that exists for it was GREEN

Exactly the trap above, third occurrence, same session shape: a heredoc
edit to `settings/mod.rs` lost the trailing backslash on two `\n\`
continuations. Two new facts, both worth more than the reminder:

**1. `check-string-gaps.sh` did not see it, and its header claims no false
negatives.** That gate matches a run of 3+ spaces *between word characters on
one source line*, which silently assumes `rustfmt` FOLDED the broken
continuation into its successor. **`rustfmt` cannot fold across a raw newline
inside a literal**, so the gap survives as *leading* indentation with no word
character in front of it. The gate is now widened to flag a displaced `\n`.

⇢ **Generalisation worth carrying past this file:** a gate that recognises a
defect by its POST-FORMATTING shape misses every instance the formatter was
unable to reshape.

**2. Round-trip tests cannot see it either.** Every settings test was
write → parse → compare, and a stray blank line round-trips *perfectly*
because `parse` trims before checking for `#`. The output was malformed in a
way no existing test could observe. The fix is an assertion on **what is
written**, not on what survives a round trip:
`every_line_of_the_written_file_is_a_comment_a_setting_or_a_blank`.

**And test the repair by REPRODUCING the defect.** My first widening of the
gate had a wrong discriminator (it keyed on "line does not end in a
backslash") that let a real variant through. Re-reading the rule would not
have found it; applying both variants to the real file and re-running the
gate did, in about a minute.

Related: [[windows-paths-need-literal-edits]] — same root cause (backslashes
crossing shell layers), same fix (Edit, or a written script file).
[[feedback_a_gate_that_underreports_looks_green]] — the same class one level
up: a gate whose output is wrong reads as a gate that passed.
