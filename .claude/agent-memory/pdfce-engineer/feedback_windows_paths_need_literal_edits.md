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

**★★★ 2026-08-20 — THREE MORE, IN ONE SESSION, ALL THE SAME PATH.** Even
with the rule already widened to "any backslash", I put
`D:\Dev\temp\pdfce\ncored-benchmark-cad-drawing.pdf` through a **quoted**
heredoc (`<<'PY'`) with **doubled** backslashes — the form this file says is
acceptable — and it still arrived as `D:\Dev<TAB>emp\pdfce<NEWLINE>cored-…`.
Twice into `docs/NEXT_SESSION.md` and once into an agent-memory file.

**So the escape processing is happening BEFORE bash sees the heredoc, and
quoting does not stop it.** `<<'PY'` protects against *bash* expansion; it
does not protect against the tool layer. The mitigation this file offered —
"double every backslash" — is therefore **not sufficient**, and the sentence
above about raw strings only helps if the payload survives to Python intact,
which it does not.

**The rule, with the escape hatch removed: a backslash never goes through the
Bash tool. Not doubled, not quoted, not raw.** Use `Write` for a new file or
`Edit` for a change; both take literals end to end.

**And the structural mitigation that actually worked:** the benchmark path now
appears **once**, on its own line, in `docs/NEXT_SESSION.md` §7, and every
other mention references that section instead of repeating it. One place to
get right beats seven places to check.

**★★ 2026-08-21 — WIDENED AGAIN, TO BACKTICKS, and the trigger is not a
backslash at all.** I wrote a commit message with `git commit -m "…"` that
contained a backticked phrase. **Bash read the backticks as COMMAND
SUBSTITUTION** and spliced a fifteen-line file listing into the middle of a
sentence. It shipped; I caught it only because I printed the message back.

Note the shape, because it is why the previous wording did not protect me:
this file had grown into *the backslash rule*, and a backtick is not a
backslash — so the rule did not feel like it applied. **The real class is
"content that goes through a shell gets interpreted by the shell,"** and the
dangerous characters are all of `` ` ``, `$`, `\`, `!`, and unbalanced quotes.

**Operationally, one line covers every instance so far: NEVER put prose
through the Bash tool.** Not a commit message, not a heredoc, not a `-m`.
Write the text with `Write` and pass the file (`git commit -F file`), or use
`Edit`. Every long commit message this session went through `-F` and was
fine; the ONE that used `-m` because it was "short" is the one that broke.

**★★ AND IT HAD HAPPENED HERE BEFORE — `5047cb9`, 2026-08-11, in CI.** A
double-quoted shell string in `ci.yml` contained `` `aes` ``, meant as a
code-span; bash ran it as a command, found nothing, and spliced the empty
output, so the error message printed *"pdfce-core  enables an extra feature
on the  crate"* — **losing the single word it existed to say.** `set -e` did
not catch it, because `echo`'s exit status is 0.

Same root cause, **different medium**: a CI error string then, a commit
message now. Two occurrences across two media is this project's promotion
bar, and the pair is more instructive than either alone — *the danger is not
a file format or a tool, it is that a shell reads its input.*

Note also how each was found: the 2026-08-11 one only because somebody
deliberately made the gate FAIL to see what it printed; today's only because
I printed the message back. **Neither would have surfaced from a green
run**, which is the property they share with the vacuous tests in
[[splice-end-marker-must-be-searched-from-start]]'s neighbourhood.

Related: [[absence-needs-an-unscoped-query]] — same family. Both are cases
where a tool returned something that *looked* like a normal result, and the
only defence was checking with an instrument rather than with a glance. Also
[[splice-end-marker-must-be-searched-from-start]], from the same session:
another scripted patch, another silent corruption, found by a reader rather
than a check.
