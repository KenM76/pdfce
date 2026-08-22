---
name: never-bundle-code-into-a-filing-commit
description: A librarian filing commit that also touches crates/ or tools/ cannot file itself and leaves check-commits-filed.py red; repairs go in their own commit
metadata:
  type: feedback
---

**Repairs go in their own commit, never inside the librarian's filing.**

**Why:** `check-commits-filed.py` counts a commit as needing a filing if it
touches **code**. A librarian filing is the thing that files other commits —
so the moment you bundle a code repair into it, that commit becomes a code
commit **in no filing**, and the gate goes red on the very commit that was
supposed to make it green. It cannot file itself.

**The instance (2026-08-22, `c24ad7a`).** The librarian's filing of three
deep-zoom commits was committed together with three hard-rule-11 repairs
it had reported: a wrong Pass family in a README, a superseded zoom
ceiling in `main.rs`'s `--region` doc block, and a carriage-return-corrupted
RAG path in an agent file. Because `main.rs` was in the diff, the gate
immediately reported *"1 code commit is in no filing"* and a second
librarian dispatch was needed to close a loop that should never have
opened.

**How to apply:**

- When the librarian reports survivors it cannot edit (`crates/`, `tools/`
  and `.claude/` are outside its remit), fix them in a **separate commit**
  — before the filing is fine, after is fine, inside is not.
- A filing commit should be **docs-only**. If it is docs-only, the gate
  does not count it and the loop terminates in one step.
- The same applies to any commit whose subject begins `librarian:` — that
  prefix is a promise about the diff's contents, and mixing code in breaks
  it for a reader as well as for the gate.
- **Do not reach for `tools/commits-filed-baseline.txt`.** The gate's own
  output says so: that file is pre-existing debt, and extending it
  silences exactly what the gate exists to catch.

★ **The shape, because it will recur in other forms:** a checker whose
input is *the commit list* can be defeated by the commit that carries its
own remedy. This project has met that shape before — see
`D:/dev/rag/rust/a_gate_whose_input_is_the_commit_list_is_vacuous_when_the_pre_commit_sweep_runs_it.md`.
Anything that both *reports* and *is reported on* needs its two roles kept
in separate commits.

See [[librarian-needs-exact-hashes]] and [[run-the-projects-own-gates]].
