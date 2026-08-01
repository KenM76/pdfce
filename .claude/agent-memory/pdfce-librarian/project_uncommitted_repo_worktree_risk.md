---
name: project-uncommitted-repo-worktree-risk
description: pdfce's entire crates/ tree is uncommitted in git; this recurringly breaks autonomous-builder subagent dispatch via git worktree isolation.
metadata:
  type: project
---

As of at least Pass 14.0 (2026-08-01), the entire pdfce workspace source
(`crates/`, `Cargo.toml`/`Cargo.lock`, `fixtures/synthetic/`, `tools/`,
`docs/decisions/`) sits **uncommitted** on top of a single bootstrap commit
(`67967b2`). This is deliberate — the operator hasn't given commit
authorization, partly because `LEGAL.md` §1 (OSS license choice) is still
undecided and a public-facing commit posture shouldn't get ahead of that.

**Why this matters to me (the librarian):** it has now caused a **recurring**
engineering cost, not a one-off — the autonomous "KenAgent"-style builder is
dispatched into an isolated `git worktree`, which checks out a *commit*, not
the uncommitted working-tree state. A worktree branched from `67967b2` can't
see any of Passes 1–13's uncommitted code, so the builder's isolated
workspace is effectively empty/stale relative to what the orchestrating
session actually has on disk. This has bitten multiple Pass dispatches (see
`D:\dev\rag\rust\autonomous_builder_worktree_isolation_uncommitted_substrate.md`
for the full mechanics writeup). The workaround each time is instructing the
builder that the main cwd is authoritative and to write its deliverable
there directly — a per-dispatch instruction, not a fix.

**How to apply:** every time I file a Pass-shipped or pre-compaction-capture
entry, keep surfacing "commit authorization" in the still-open operator
items list (oldest-first ordering, per the existing SESSION_LOG convention)
— don't let it quietly drop off just because it's been repeated many times.
As of SESSION_LOG continuation 39 (2026-08-01, decision 015 filed) this item
was explicitly called "now especially pointed" given the tree's continued
growth (decision 015's reflow work, Pass 15.x) — keep escalating the framing
as the uncommitted tree keeps growing, don't just repeat the same wording.
As of continuation 42 (2026-08-01, Pass 15.2 shipped — FF-A/decision 015
COMPLETE end-to-end, on top of the already-COMPLETE decision 014) the framing
escalated again: the tree now holds TWO full, complete, multi-Pass
subsystems (decision 014 in-place editing + decision 015 reflow) sitting
entirely uncommitted — "the largest uncommitted span in the project's
history to date." Keep escalating in concrete terms (what subsystems/Passes
are now at risk) rather than a generic repeat, every time this item is
re-surfaced.
When an engineer flag mentions a worktree/builder-dispatch anomaly, check
first whether it's this same recurring cause before treating it as a new
finding. If the operator ever grants commit authorization, that's the
structural fix (worktrees checkout HEAD cleanly once HEAD actually reflects
current state) — note that explicitly in whatever session log entry records
the commit actually happening, since it retroactively resolves this risk.
As of continuation 46 (2026-08-01, Pass 16.1 shipped — decision 016/FF-D
now TWO THIRDS complete, only the 16.2 canvas-UI slice remaining) the tree
holds FOUR complete/near-complete subsystems uncommitted end-to-end:
decision 014 (in-place editing), decision 015 (reflow), Pass 14.4
(GUI-polish interaction set), and decision 016/FF-D's point+boxed
new-text engine (16.0+16.1). Keep escalating in concrete, subsystem-named
terms each time this re-surfaces — the framing should track exactly which
completed subsystems are at risk, not just repeat "the tree grew again."
As of continuation 47 (2026-08-01, Pass 16.2 shipped — decision 016/FF-D
now COMPLETE end-to-end) the tree holds the ENTIRE Acrobat text-handling
parity arc uncommitted: decision 014 (in-place editing, Pass 14.0-14.4),
decision 015 (reflow, Pass 15.0-15.2), and decision 016/FF-D (add-new-text,
Pass 16.0-16.2) are all COMPLETE and all uncommitted — the largest span
yet, and now framed as "a whole completed milestone with zero of it in
version control" rather than just "another subsystem added." Use that
framing (completed-milestone-sized risk) the next time a multi-Pass
decision closes out while still uncommitted.
As of continuation 48 (2026-08-01, FF-D follow-up hardening shipped —
certification-signature guard on `add_text`/`EditSession::add_text`,
closing the last flagged gap in the text-parity arc) the risk escalated
once more, in a distinct way from prior continuations: this wasn't a new
subsystem, it was the CLOSING FIX on an already-complete arc, meaning the
uncommitted tree now holds a genuinely *finished-and-hardened* milestone
(zero known open threads) with zero of it in version control. Also newly
relevant: the autonomous `/loop` was throttled to an idle heartbeat at
this same continuation (awaiting operator steer, no longer spawning
feature work) — so the uncommitted-tree risk is no longer actively
growing on its own; the next growth trigger is an operator decision
(FF-C unblock, list-authoring, or a new steer), not autonomous
continuation. Keep noting this "growth has paused, but nothing shrank"
framing until either a commit happens or new autonomous work resumes.
