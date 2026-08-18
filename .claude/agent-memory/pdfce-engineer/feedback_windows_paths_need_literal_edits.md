---
name: windows-paths-need-literal-edits
description: Never patch text containing ANY backslash (Windows paths, Rust/C escapes, line continuations) through a heredoc or sed — write a script file or use Edit; also, never `git checkout` to undo a sabotage
metadata:
  type: feedback
---

Edit any text containing a **Windows path** with a literal string tool
(Edit/Write), never through a `sed` expression or a Python/shell heredoc.

**Why:** on 2026-08-11 I rewrote `docs/NEXT_SESSION.md` through a Python
heredoc containing `D:\builds\pdfce-...`. Python read the `\b` as an escape,
ate the `b`, and wrote a literal **0x08 BACKSPACE byte** into the file. It
rendered as `D:uilds\...` — a path that does not exist, in the one document
whose job is to tell the next session where the build is.

It survived two readings because **a control character is invisible in normal
output.** It was found only by grepping for the *old* build hash expecting
zero matches and getting one — a search looking for staleness that turned up
corruption instead. Confirmed with `cat -v`, which is the tool that makes it
visible.

The exposure is wide, not a one-off: `\b`, `\t`, `\n`, `\f`, `\v`, `\0`, `\r`
and `\a` are all real escapes, and this project's own documents name
`D:\builds`, `D:\temp`, `D:\Dev\...`, `\fixtures`, `\target`. Any of those
after a backslash is a live grenade in a non-raw string.

**How to apply:**
- Paths → `Edit`/`Write`, always. They take literals; nothing is interpreted.
- If a heredoc is genuinely necessary, use a **raw** string (`r'''...'''`) or
  double every backslash — and then *verify with `cat -v`*, not by eye.
- After any bulk rewrite of a document, sweep for control characters:
  `python -c "d=open(F,encoding='utf-8').read(); print([hex(ord(c)) for c in set(d) if ord(c)<32 and c not in '\n\r\t'])"`

**★ WIDENED 2026-08-17 — it is not only PATHS, it is any backslash, and it
bit me three times in one session on SOURCE CODE.** The rule above scoped
the hazard to Windows paths; that scoping is what let me walk into it again.

- A Rust string **line-continuation** `\` at the end of a line inside a
  format string was eaten by a quoted heredoc (`<<'PYEOF'`), so the patch
  script's anchor never matched and the `assert` fired. Twice.
- Worse, once it *did* apply: I wrote `\\\n` intending a Rust continuation
  and produced a **literal `\n` escape** in the source, which Rust then
  compiled into a real newline. That broke `pdfce-cli render-page`'s
  one-line stdout contract — caught only because a contract test asserts
  `line.matches('\n').count() == 1`.

So the trigger is **a backslash in the payload**, whatever it means:
Windows paths, Rust/C string escapes, regex, LaTeX, `\|` in Markdown tables.

**How to apply, updated:** for any multi-line patch to source, **Write a
script file and run it**, or use `Edit` directly. Do not fight the heredoc —
the failure is silent when it is not loud, and the loud version costs a
build cycle.

**★ A SECOND, UNRELATED TRAP FROM THE SAME SESSION, filed here because it
also destroys work silently:** `git checkout <file>` to undo a **sabotage
check** reverts the file to `HEAD` — including the *feature work* you were
sabotaging, if it is not yet committed. I lost every change to
`crates/pdfce-render/src/color.rs` that way and had to re-apply them from
the patch scripts. **Copy the file aside before sabotaging**
(`cp x D:/Dev/temp/x_backup`) and restore from that copy, never from git.

**★★ AND I DID IT AGAIN ON 2026-08-18 — the scoping is what let me.** The
paragraph above says *"to undo a **sabotage** check"*, so when I wanted to
undo a **half-applied patch script**, the rule did not feel like it applied.
It is the same command with the same effect. I ran `git checkout -- crates/`
and lost three uncommitted edits to tracked files.

**The detail that makes this dangerous rather than merely annoying: it
spared every UNTRACKED file.** Three brand-new modules survived untouched
while three small edits to existing files vanished, so the tree looked ~90 %
intact and `cargo build` still nearly worked. Nothing announced the loss —
it surfaced as a compile error about a missing module, which reads like a
forgotten `mod` line, not like data loss.

**How to apply — the rule with no scope on it: NEVER use `git checkout`,
`git restore` or `git stash` to undo anything while uncommitted work is in
the tree.** Not for sabotage, not for a bad patch, not for "just this one
file". Undo by *editing the change back*, or by restoring from a copy you
made first. If a bulk revert genuinely seems necessary, commit the good work
first — a throwaway commit costs nothing and is reversible; a checkout is
not.

Related: [[absence-needs-an-unscoped-query]] — same family. Both are cases
where a tool returned something that *looked* like a normal result, and the
only defence was checking with an instrument rather than with a glance.
