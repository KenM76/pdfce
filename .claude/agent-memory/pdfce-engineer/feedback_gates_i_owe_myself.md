---
name: gates-i-owe-myself
description: The fuzz-target gate is the one I skip; and a harness that enumerates a module's entry points goes stale every time the module grows
metadata:
  type: feedback
---

Two gates are easy to declare met without meeting them. Both bit on 2026-08-05.

**1. The cargo-fuzz gate is the one that gets skipped.** ARCHITECTURE.md §10.2
and the engineer role's "Always" list both require: new code touching
untrusted-input parsing extends a `cargo-fuzz` target. On Pass 36.1 I shipped
`plan_delete_node` with 10 fixture tests, ran fmt/clippy/tests/ui-strings/
ledger/cargo-tree, and reported the Pass complete — **without the fuzz arm.**
The librarian caught it, not me. Fixture tests feel like enough; they are not
the gate that was asked for.

**Why:** the other gates are one command with a pass/fail line. Fuzzing needs a
DLL on PATH ([[fuzz-asan-dll]]), a minute of runtime, and a judgement about
which branches to drive — so it reads as optional in a way `cargo clippy`
never does. It is not optional.

**How to apply:** before declaring ANY Pass done that adds a function taking
parsed-from-file data, grep the fuzz targets for the module. If the module is
listed and the new entry point is not, that is the gate, unmet.

**2. A fuzz/test harness that enumerates a module's entry points goes stale
silently.** Adding the owed arm exposed that `fuzz/fuzz_targets/vector_edit.rs`
still drove only the three Pass 9c-min planners it was written for:
`plan_delete_subpath` (Pass 25.2), `plan_move_subpath` (Pass 28.0) and
`plan_move_handle` (Pass 30.1) had **never been fuzzed at all**. Three Passes
each added a planner beside them and none extended the target.

This is the same family as R151 (a `pub fn` with no caller) and R152 (a caller
that confirms nothing): the harness has no way to complain about what it does
not mention. The check is cheap — a planner in the module's public surface that
appears in no fuzz target is findable by grep.

**How to apply:** when extending a harness, do not just add your own arm — diff
the harness's list against the module's current public surface. The gap will be
older than your change.

---

## ★ AMENDED 2026-08-21 — BOTH RECURRED, sixteen days later, IN THE SAME FILE

This memory was correct and did not prevent either recurrence. That is
worth more than the recurrences themselves, and it is the warrant the
librarian minted **`R209`** on.

**1 again.** `cargo fuzz build` had been **red for three days** on a
one-line compile break — `MarkupSpec::Square` gained a `border_effect`
field (`Pass 82.0`) and `fuzz/fuzz_targets/annot_author.rs`, which
constructs that variant, was not updated. Found while verifying CI before
tagging `v0.7.0`, not by any local run.

★ **The mechanism this memory missed, and it is not carelessness.** "Run
the gates" in this project means `for g in tools/check-*`. That is
fourteen scripts and **it is ONE of CI's nine jobs.** A green local sweep
and a red CI were **never a contradiction** — there was simply no place
where the two were compared. Grepping the fuzz targets, which the "how to
apply" above tells you to do, would not have caught a *compile* break
either.

**2 again.** Reading the file to fix the compile error found the dispatch
at `match c.byte() % 8` against an **eight-variant enum with one arm spent
twice**, so `MarkupSpec::Cloud` had **never been fuzzed at all**. Exactly
the `vector_edit.rs` shape, in a different target.

★ **And a sharper form of the check:** the modulo **is** a coverage claim,
phrased as an integer. A claim asserting a **boundary** rather than a
**member** — a count, a modulo, an absence, a closed either/or — contains
no token tying it to what changed, so no grep for the new thing finds it.
`% 8` and *"…are **not** counted as `overprint_refused`"* are the same
defect in two spellings.

**How to apply, updated:**

* **`python tools/check-ci-parity.py --list`** prints the local stand-ins
  for every CI job. The fuzz one is **`cd fuzz && cargo check --bins`** —
  six seconds, no nightly, no ASan, and it catches the entire class that
  has ever broken that job.
* When you touch a fuzz target, **diff its dispatch arity against the
  enum**, not just its arms against your change.

---

Related: [[run-the-projects-own-gates]] (the gate set is wider than fmt/clippy/
tests), [[fuzz-asan-dll]] (why running one is not one command on this machine).
