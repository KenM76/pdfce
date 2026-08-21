---
name: git-add-all-is-unsafe-with-live-subagents
description: Never `git add -A` while a subagent is running — their scratch files and their in-flight document edits land in your code commit, and this repo is public
metadata:
  type: feedback
---

**Stage explicit paths. Never `git add -A` in a session with live
subagents.**

**Why:** on 2026-08-21 a `git add -A` swept, into a commit whose message
described a rendering fix:

- **`jina.txt`** — a scraped PDF Association web page, a subagent's working
  file. Third-party content, and `CLAUDE.md` rule 8 makes that non-trivial:
  **`github.com/KenM76/pdfce` is public, so anything committed is published
  by default.**
- **681 lines of `ROADMAP.md`, plus `FEATURES.md`, `SESSION_LOG.md` and
  `ARCHITECTURE.md`** — the librarian's filing, mid-flight. A reader would
  have attributed the librarian's prose to the engineering commit, and the
  librarian was still writing when it was captured.

Caught by reading `git commit`'s own file list. The commit was reset and
re-made from explicit paths; the scratch file was deleted rather than
committed.

**How to apply:** `git status --short` first, then `git add <path> <path>`.
If the list is long enough that typing it is annoying, that is the signal —
a code commit should not touch that many files. The librarian's documents
get their **own** commit, made after the agent reports back, so its work is
attributable to it.

**Related:** the subagent working directory is the repo root, not a temp
dir. Anything a dispatched agent writes lands where your commit will find
it.
