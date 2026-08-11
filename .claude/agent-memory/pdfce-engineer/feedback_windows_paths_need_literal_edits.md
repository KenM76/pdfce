---
name: windows-paths-need-literal-edits
description: Never edit text containing Windows paths through a shell heredoc or sed — \b \t \n \f \v \0 are real escapes and silently corrupt D:\builds, D:\temp, \fixtures
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

Related: [[absence-needs-an-unscoped-query]] — same family. Both are cases
where a tool returned something that *looked* like a normal result, and the
only defence was checking with an instrument rather than with a glance.
