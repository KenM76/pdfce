---
name: compositor-state
description: 2026-08-21 — Pass 97.0 (the group model) shipped in four sub-passes; the Ghent transparency panels are blocked on §11.3.4's blending colour space, not on the group model
metadata:
  type: project
---

**`Pass 97.0` — Stage A of `docs/compositor-plan.md` — shipped 2026-08-21**
in `7160819`, `9b49ca0`, `86a7b70`. `Pass 97.1`'s first two deliverables in
`0d5fc29`.

**Why:** the plan expected 7 Ghent patches from Stage A. It delivered **0
patch verdicts** and a great deal of real correctness. The gap is the
finding, and it re-scopes `Pass 97.1`.

**What exists now, so it is not rebuilt:**

- **`crates/pdfce-render/src/compositor.rs`** — pdfce owns §11.4.4's
  element formula, §11.4.8's knockout variant, backdrop removal, `Union`,
  and all thirteen Table 136 separable blend functions (with the corpus's
  four printing errata `GD-1`…`GD-4`).
- **`Canvas::group`** — non-isolated groups render over their own backdrop.
  The content stream is walked **twice**, and the second walk is skipped
  under §11.4.4 NOTE 5's own condition (interior all-`Normal` ⇒ one walk is
  exact). Counter `groups_backdrop_reruns`.
- **`KnockoutTarget`** — real §11.4.6, four planes, §11.4.6 NOTE 6's
  nesting rule honoured. **Explicit `/K true` only**; the implicit
  population (§9.3.8 `/TK` default-`true` text, §11.7.4.4 `B`/`b`, §11.6.7
  shading patterns) is still not knockout.
- **Soft mask on the group RESULT** (§11.4.5), lifted out of the contents'
  clip. Counter `soft_masks_on_group_result`. Folding into the clip is
  still correct for an *elementary* object (§11.6.4.1's `q_m`).

**★ THE BLOCKER, derived by hand and reproducible.** `1_GWG162`'s
`Difference` cell: magenta `0 1 0 0 k` under black `0 0 0 1 k`. §11.3.4's
complement gives `1 − |cb′ − cs′|` = `DeviceCMYK 1 0 1 0` = the surround
colour **exactly**. pdfce renders `(237,1,140)`, pdfium `(202,29,108)` —
both blend in RGB, both wrong, differently. **Every Ghent transparency
patch declares `/Group /CS /DeviceCMYK` on the PAGE**, including
`3_GWG161`, whose own objects are ICCBased RGB. So §11.3.4's subtractive
complement is `Pass 97.1`'s **leading** deliverable, ahead of spot planes,
and it needs a real colorant buffer — the 4→3→4 reconstruction is measurably
lossy (`0 1 0 0` comes back as `0, .995, .409, .071`).

**Measured state at the handoff:** Ghent `26 pass · 14 FAIL · 11
UNRESOLVED`, unchanged; traps `67 → 55`; `1_GWG161` `14 → 2`; soft-mask
strip correlations `0.576→0.962`, `0.725→0.978`, `0.905→0.986`. Full-corpus
render parity (4,023 files) **identical** to a `2e6bb83` worktree build,
bucket for bucket.

**How to apply:** read `docs/compositor-plan.md`'s 2026-08-21 amendment
before scoping anything in this family — it carries the derivation, the
instrument gap (`ghent-check.py` has no calibrated reference-strip
threshold, deliberately not added in the session that moved those numbers),
and the `3_GWG161` diagnosis with two explanations ruled out.

⚠ **`tools/render-parity/out/summary.json` is STALE** — it records a bucket
vocabulary the harness no longer emits, so `--gate` mode is meaningless
until re-based. Compare two current runs instead.

See [[a-correct-fix-can-be-unreachable]].
