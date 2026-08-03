---
name: librarian-needs-exact-hashes
description: The librarian has no shell access — always paste exact commit hashes into its dispatch; never write a placeholder or say "see git log"
metadata:
  type: feedback
---

`pdfce-librarian` (and the other doc agents) have **Read/Write/Edit/Glob/Grep/
WebSearch/WebFetch only — no Bash, no PowerShell.** They cannot run `git log`,
`git rev-parse`, or anything else to discover a commit hash.

**The rule:** run `git log --oneline` yourself and paste the *exact* hashes into
the dispatch prompt. Never write a placeholder hash, never write
"`<hash>`-equivalent", and never instruct the agent to "see `git log`".

**Why:** on 2026-08-02 the engineer dispatched a filing with
"`3a0b1f1`-equivalent (see `git log`)". The librarian could not check, so it
used the placeholder verbatim — and wrote the **non-existent** hash `3a0b1f1`
into `docs/ROADMAP.md` and `docs/SESSION_LOG.md` **12 times**, including in the
commit chain. The real hash was `c59b0c4`.

A fabricated hash in an audit trail is worse than no hash at all: the first
person to run `git show 3a0b1f1` gets "not a valid object name", and that
discredits every other claim in the record. The whole point of the ROADMAP is
that it can be trusted without re-deriving it.

**How to apply:**
- Before any librarian dispatch that references commits, run
  `git log --oneline -N` and copy the real short hashes.
- After a filing lands, spot-check it: `grep -o '[0-9a-f]\{7\}' docs/ROADMAP.md`
  piped through `git cat-file -t` catches fabricated or stale refs cheaply.
- The same caution applies to any figure the agent cannot verify — test counts,
  file counts, timings. If you did not measure it, do not hand it over as fact,
  because it will be filed as fact. See [[feedback_engineer_does_the_observing]]
  for the same principle applied to behavior rather than metadata.
