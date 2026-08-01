# 015 — FF-A: within-block offline text reflow (explicit reviewable re-wrap, greedy line-breaking, alignment auto-detect/preserve, justified moved into FF-A)

**Date:** 2026-08-01
**Status:** Decided
**Decided by:** KenAgent (`autonomous-builder`, decision-consultant mode), per `docs/decisions/README.md`
**Requested by:** `pdfce-engineer` (the next text-handling parity step now that the
Pass 14.x in-place-editing family — model 14.0 / edit 14.1 / format 14.2 /
GUI+session 14.3 — is complete)
**Supersedes:** nothing
**Amends:** `docs/decisions/014-acrobat-text-editing.md` §3 (Reflow), §5.3
(fast-follow ladder) and §6 — **justified alignment is moved OUT of FF-B and
INTO FF-A**; FF-B's headline narrows to cross-block + cross-page offline reflow
only. Also `docs/ROADMAP.md` (adds the Pass 15.x reflow family and three
standing rules R75–R77, pending librarian numbering; current ceiling R74).
**Builds on:** Pass 14.0 (the derived Run→Line→Column→Block model, its x-band/
column geometry and `BlockDiagnostics`), Pass 14.1 (advance-preserving
REMOVE→REPLACE content-stream surgery, `FollowerDisposition`, per-glyph
provenance carrying §9.4.4 advances, the single-line relayout + overflow
disclosure), Pass 14.2 (formatting surgery), Pass 14.3 (`EditSession`
command-log integration — `CommandKind::EditText`/`FormatText`), `vartext.rs`
(the greedy `wrap_lines` breaker and `/Q` `align_x` quadding), `edit.rs`
`EditSession` (the undo/redo command log)
**Does not touch:** `LEGAL.md` §1 (license undecided); redaction's R35
full-rewrite guarantee; the 14.1 default post-edit behavior (which FF-A adds an
opt-in beside, never replaces)

---

## 1. Context

The Pass 14.x family shipped Acrobat-style **in-place** text editing: a
derived, reviewable Run→Line→Column→Block model (14.0), advance-preserving
content-stream **surgery** that fixes a run and shifts the rest of its own line
by the advance delta — the edited line MAY overflow the original right margin,
and that overflow is **disclosed** (14.1) — plus formatting-on-selection (14.2)
and the GUI/`EditSession` integration where each accepted edit is one undo-able
command (14.3). 14.1's own disclosure string already points forward: *"block
re-wrap (reflow) is deferred (FF-A) — enable reflow to re-wrap."* FF-A is that
next slice.

Decision 014 named the reflow ladder: **FF-A** = within-block offline reflow
(greedy line-break; left/center/right) = *reach Acrobat OFFLINE parity*;
**FF-B** = cross-block/cross-page offline reflow + justified = the
*exceed-Acrobat headline* (Acrobat's true multi-paragraph reflow is
cloud-gated and English-only). This decision scopes FF-A concretely and settles
one genuine tension the Acrobat-parity scoping surfaced.

### 1.1 The substrate already exists

- **`text_edit/model.rs`** — `Block` (paragraph) with a union `bbox`,
  `line_indices`, and `column`; `Line` with `baseline_y`, `size`, `bbox`;
  `BlockRecognitionOptions` exposing the x-band `column_overlap_ratio`,
  `paragraph_leading_ratio`, `indent_ratio`, `line_baseline_ratio`; and
  `BlockDiagnostics` counting every derived inference (rule 4 made structural).
  The x-band/column geometry needed to infer a block's *alignment* from glyph
  x-positions is already computed here.
- **`text_edit/edit.rs`** — the 14.1 advance-preserving REMOVE→REPLACE surgery,
  `FollowerDisposition` (how the rest of the line is disposed), and the
  `EditReport.disclosures` verbatim-surfacing discipline. This is the exact
  machinery a reflow re-emits the block's lines through.
- **`vartext.rs`** — `wrap_lines`: a **greedy** word-wrap (accumulate words
  until the measured width is exceeded, break; a single word wider than the box
  becomes its own overflowing line) over WinAnsi bytes measured by Std14 AFM
  widths, plus `align_x`/`Quadding` for `/Q` left/center/right (§12.7.3.3).
  There is **no Justify** in `Quadding` today.
- **`edit.rs` `EditSession`** — the undo/redo command log; `CommandKind` already
  carries `EditText` and `FormatText` as one-command-per-accepted-edit entries.
  A reflow becomes one more: `ReflowBlock`.

### 1.2 The flagged open question (from the Acrobat scoping)

`text_edit__formatting_options.md` documents a **Justify** button on the
**BASE** (non-cloud) Edit-Text formatting panel, alongside left/center/right.
`text_edit__paragraph_reflow_and_auto_adjust_layout.md`'s FF-A grounding
session (finding 3) flags this as *in tension* with decision 014's placement of
justified in FF-B, and names it "the single highest-value open question" —
without resolving it either way. Settling it is §3.1 below.

### 1.3 Invariants this decision serves

Fuzzy-never-sneaky (rule 4): reflow is DERIVED layout the file never stated
(§14.8 S1–S9), so it must be a reviewable accept/reject preview, never a silent
re-layout. Minimal-diff/round-trip (R32/R46) and incremental-save-default
(R34/R36). The R59 render-fidelity gate. GUI-core separation (R21/§3): the
engine and surgery live in `pdfce-core`; only the canvas interaction is in
`pdfce-gui`. Tagged-content preservation (§14.6/§14.7, R72).

---

## 2. Options considered

**Justified placement (the flagged question).**
- **J-FFB** — keep justified in FF-B (decision 014 as written).
- **J-FFA** — move justified into FF-A as a fourth alignment mode.

**Line-breaking algorithm.**
- **LB-greedy** — greedy/first-fit (decision 014's choice; Acrobat publishes no
  algorithm, so it is a free choice).
- **LB-kp** — Knuth-Plass total-fit.

**vartext reuse.**
- **VT-asis** — call `wrap_lines` directly.
- **VT-extend** — factor the greedy packing core into a shared breaker taking a
  width-measuring closure; supply a provenance-advance measurer for FF-A.

**Trigger.**
- **TR-auto** — re-wrap automatically whenever an edit changes a line's width.
- **TR-explicit** — an explicit operator action producing a reviewable preview.

**Reflow unit.**
- **U-block** — exactly one recognized `Block`, never crossing a sibling.
- **U-column / U-page** — larger scopes (these are FF-B).

**Page overflow.**
- **O-silent** — drop/clip content past the page edge (Acrobat's behavior).
- **O-refuse** — refuse the reflow if it would overflow the page.
- **O-disclose-allow** — disclose the overflow, emit the real content off-page
  (recoverable), let the operator accept or reject.

**Alignment on re-wrap.**
- **AL-default-left** — always re-wrap left unless the operator picks otherwise.
- **AL-detect-preserve** — infer the block's alignment from glyph x-positions
  and preserve it by default; operator-overridable.

---

## 3. Decision

### 3.1 Justified is IN FF-A (amends decision 014) — the highest-value call

**J-FFA.** Move justified out of FF-B into FF-A.

Classic/offline **Justify is within-block inter-word slack distribution** over
an already-computed greedy wrap. It needs *only* FF-A's block-width plus
greedy-wrap machinery — nothing FF-B-specific. Justified is an **alignment
mode**, a peer of left/center/right (all three of which FF-A already ships), not
a reflow **scope**. FF-B's genuine headline is the orthogonal axis: content
moving *between* blocks and *across* pages — the cloud-gated, English-only
capability Acrobat lacks offline. Shipping three of four alignment modes in FF-A
and gating the fourth behind an unrelated cross-block engine would be
incoherent.

The Acrobat scoping is decisive: **Justify sits on the BASE (non-cloud)
Edit-Text panel** alongside L/C/R, so it is demonstrably a classic-engine
(offline, single-block) capability — precisely FF-A's domain — not an
Auto-Adjust/cloud one. The base-panel observation therefore *does* force
justified into FF-A; it does not leave it in FF-B.

**Mechanics.** Distribute each full line's slack (`block_width −
natural_line_width`) across its inter-word gaps as **`TJ` numeric position
adjustments (§9.4.3)** — robust across simple and composite fonts — with **`Tw`
word spacing (§9.3.3)** as an equivalent simple-font path. The **last line of
each paragraph stays at the block's base alignment** (typically left), never
stretched. A single-word line (no inter-word gap) falls back to inter-glyph
distribution, or is left flush-left and disclosed.

**Why this is also exceed-Acrobat on execution:** Acrobat's own Justify is
community-reported as unreliable ("not working" in some versions). A reliable
offline pdfce justify is parity on feature *presence* and a lead on *quality*.

### 3.2 Line-breaking — LB-greedy, VT-extend

**Greedy/first-fit, confirmed.** Acrobat publishes no line-breaking algorithm
(the scoping confirms this is unanswerable from Adobe sourcing), so pdfce is not
matching a documented algorithm — it is a free design choice, and greedy is the
honest, low-cost one decision 014 already named. **Knuth-Plass is deferred** (a
named non-goal: better typography, far more cost, no Acrobat baseline to match).

**Extend `vartext`, do not reuse it as-is.** `wrap_lines` packs WinAnsi bytes
measured by Std14 AFM widths; FF-A wraps the **block model's runs** measured by
**per-glyph §9.4.4 advances from provenance** (real embedded/supplied font
Widths) — a richer measurement. Factor the greedy **packing core** (accumulate
word-tokens until width exceeded → break; an oversized single word becomes its
own overflowing line) into a shared breaker that takes a **width-measuring
closure** and a word/space token stream. `vartext` keeps its Std14 path; FF-A
supplies a provenance-advance measurer. One greedy breaker, two callers.

**Break opportunities: whitespace (U+0020) only.** No soft-hyphen/U+00AD, no
dictionary **hyphenation** (the scoping found no evidence Acrobat hyphenates
offline — default: none), no CJK per-glyph breaking (CJK is FF-E). A word wider
than the block becomes one overflowing line, **disclosed** — pdfce diverges from
Acrobat's *silent* horizontal overflow by disclosing it.

### 3.3 Trigger + scope — TR-explicit, U-block

**Explicit operator action, not automatic-on-edit (TR-explicit).** Reflow is
derived layout the file never stated (§14.8 S1–S9); rule 4 requires an
accept/reject step, so it can never be a silent side effect of a keystroke.

**FF-A does not supersede 14.1 — it adds an opt-in beside it.** 14.1's
single-line relayout + overflow disclosure **remains the default** post-edit
behavior ("I fixed a typo — keep everything else byte-exactly put"). FF-A is the
operator-invoked "now re-wrap this paragraph to its box." Both remain; the
operator chooses.

**Reflow unit = exactly one recognized `Block` (U-block).** A re-wrap **never
crosses into a sibling block or an adjacent column band** — the load-bearing
bound, matching Acrobat's classic-engine box-independence (catalog must_have).
The **wrap width is the block's own detected `bbox` width** (analogous to
Acrobat's classic box width; catalog must_have), operator-adjustable before
accept.

### 3.4 Reviewable preview → one undo-able command (rule 4)

Reflow produces a derived **`ReflowPreview`** — new break points, new per-line
origins (`Tm`/`TD`), the alignment placement, the resulting new block `bbox`
(including its height change), and all diagnostics/disclosures. Nothing is
applied yet. It is presented as an **accept/reject overlay** (a ghost of the
re-wrap against the current block).

Before accepting, the operator adjusts the **inputs**, re-previewing live:
**block width** (drag the right edge / numeric), **alignment** (L/C/R/justify,
pre-filled with the auto-detected value — §3.6), and **leading**.

On **accept**, 14.1's advance-preserving surgery re-emits the block's show
operators with the new origins (unchanged lines byte-identical where provable,
only moved/re-wrapped lines get new `Tm`/`TJ`), and the whole thing lands as
**one `CommandKind::ReflowBlock`** on the `EditSession` command log —
undo/redo atomic exactly like `EditText`/`FormatText`, carrying the block
identity plus a lines-before/after magnitude for the Undo label. **Reject
mutates nothing.**

### 3.5 Vertical growth + page overflow — O-disclose-allow (rule-4 divergence)

A block **grows/shrinks vertically** as lines are added/removed (sourced: an
Acrobat box lengthens to fit more lines). Default anchor: **top-anchored** — the
first baseline is fixed and the block grows **downward**; the new `bbox` is part
of the preview.

Content pushed past the **page cropbox** is a **disclosed, reviewable
condition — never silent (O-disclose-allow)**. The preview still computes **all**
lines (the overflowing ones are real content in the preview) and discloses
*"reflow grows the block N pt past the page bottom (cropbox); M line(s) fall
outside the visible page."* On accept, that content is **emitted as real,
recoverable content at its true (off-page) position — not clipped-to-invisible,
not dropped**. The operator may instead **reject** and widen the box, shrink the
size, or defer to FF-B cross-block reflow.

This is deliberately **not** Acrobat's behavior: Acrobat's own documentation
says overflow past the page edge simply "disappears" (data-ambiguous — a render
clip vs actual loss). Reproducing that silent disappearance is exactly what rule
4 forbids. A hard **refuse** (O-refuse) is rejected too — it would lose the
operator's legitimate edit when the content genuinely exists. Disclose-and-allow
is the honest middle: the content is present, positioned truthfully, and the
operator was warned. The R59 render gate re-runs; off-page content renders where
the bytes say it is (no fidelity violation).

### 3.6 Alignment auto-detect + preserve — AL-detect-preserve (the differentiator)

Infer the block's **original alignment from glyph x-positions**, reusing the
14.0 x-band/column geometry:

- **Left** — line left-edges low-variance at the block `llx`; right edges ragged.
- **Right** — right-edges align at the block `urx`; left edges ragged.
- **Center** — per-line midpoints align at the block center; both edges ragged.
- **Justified** — all-but-last lines flush to **both** margins (near-zero
  right-edge raggedness) with variable inter-word spacing.

A **single-line block** is ambiguous → default **left** + disclose *"single-line
block: alignment inferred as left (ambiguous)."* Every inference is **counted**
in `BlockDiagnostics` and is **operator-overridable**. The detected alignment is
**preserved through the re-wrap by default**.

Acrobat has **no documented alignment auto-detect/preserve** — an operator
re-wrapping a centered or right-aligned paragraph there risks it silently
rendering left. This is a low-cost, well-evidenced **exceed-Acrobat** property,
not an incidental side effect. FF-A supports **left, center, right, and
justified** (§3.1).

### 3.7 Minimal-diff / incremental — confirmed

Reflow re-emits **only the reflowed block's own content-stream object** via the
same 14.1 advance-preserving surgery; unchanged lines byte-identical where
provable, everything outside the block byte-verbatim (R32/R46). Default save is
**incremental (R34/R36)** — the changed content-stream object goes in the
incremental update section; **not** a forced full rewrite (that is redaction's
R35 alone). **Confirmed incremental-save-safe.** A tagged block's BDC/EMC+MCID
wrapper is preserved and any /ActualText/reading-order staleness is disclosed
(R72), exactly as 14.1 does.

---

## 4. Rationale (condensed)

- **Why justified belongs in FF-A** (§3.1): it is an alignment mode driven by
  the same within-block width + greedy-wrap FF-A already builds; it is
  orthogonal to FF-B's cross-block/cross-page *scope*; and the sourced fact that
  Acrobat exposes Justify on the non-cloud base panel proves it is a
  classic-engine offline capability. Keeping it in FF-B would ship an
  incoherent 3-of-4 alignment set.
- **Why greedy + extend-not-reuse** (§3.2): greedy matches Acrobat's
  (undocumented, therefore un-matchable, therefore free-choice) behavior at
  least cost; the block model's runs carry real §9.4.4 advances that vartext's
  Std14-AFM path can't measure, so the packing core must be factored to take a
  measurer — one breaker, two callers, no duplicated logic.
- **Why explicit + single-block** (§3.3): rule 4 forbids silent derived
  re-layout; the single-block bound is the sourced Acrobat classic-engine
  behavior and the safe, well-defined wrap width; coexisting with 14.1 gives the
  operator both "keep it put" and "re-wrap it."
- **Why preview → one command** (§3.4): reflow is layout the file never stated,
  so it must be accept/reject; one `ReflowBlock` command keeps undo semantics
  identical to every other mutation.
- **Why disclose-and-allow overflow** (§3.5): Acrobat's silent "disappear" is
  the exact anti-pattern rule 4 exists to prevent; refusing would lose real work;
  disclosing while emitting truthful, recoverable content is the honest path.
- **Why auto-detect + preserve alignment** (§3.6): the 14.0 geometry already
  computes what's needed; Acrobat's lack of it is a real silent-left-align
  hazard; preserving is a cheap, evidenced lead.

---

## 5. Standing rules (binding; add to `ROADMAP.md` — librarian assigns final `Rnn`; current ceiling R74)

- **R75 — Reflow is an explicit, reviewable, single-block operation.**
  Within-block re-wrap is never automatic on edit; it is an operator-invoked
  action producing a DERIVED preview accepted/rejected before any mutation,
  scoped to exactly one recognized `Block` (never crossing a sibling block or
  column), applied as ONE undo-able `CommandKind::ReflowBlock`. 14.1's
  single-line relayout + overflow disclosure remains the default post-edit
  behavior; reflow is an opt-in.
- **R76 — Reflow overflow discloses, never disappears.** A re-wrap that grows a
  block past the page cropbox never silently clips-to-invisible or drops content
  (Acrobat's documented silent behavior); the overflow is a disclosed,
  reviewable condition and any accepted off-page content is emitted as real,
  recoverable content at its true position, never clipped-to-deleted.
- **R77 — Alignment is auto-detected and preserved through re-wrap.** A block's
  original alignment (left/center/right/justified) is inferred from its glyph
  x-positions via the 14.0 x-band geometry and preserved by default through a
  reflow; the inference is counted and operator-overridable; a single-line block
  defaults to left with a disclosed ambiguity note. (May fold into R75 at the
  librarian's discretion.)

## 6. Pass slicing (proposed Pass 15.x — the reflow family; librarian finalizes numbering)

Proposed **Pass 15.x** to keep 14.x = "in-place editing" coherent; the librarian
may instead continue it as **14.4–14.6** — that renumbering is the librarian's
call and changes nothing structural.

- **15.0 — Alignment inference + within-block greedy re-wrap engine (core,
  READ-ONLY).** The derived `ReflowEngine`: `Block` + width + alignment + leading
  → `ReflowPreview` via the factored greedy breaker over provenance §9.4.4
  advances; alignment auto-detect from glyph x-positions; `pdfce-cli inspect
  --reflow-preview`. **Acceptance:** greedy wrap matches hand-computed break
  points at a given width on a multi-line fixture; L/C/R/justified inferred
  correctly on aligned fixtures; single-line → left + disclosed; oversized-word →
  one overflowing line + disclosure; no write; core has no GUI dep (`cargo tree
  -p pdfce-core`); Pass 14.0 tests unchanged; fmt/clippy clean. **Non-goals:**
  any content-stream mutation, UI, cross-block. **Prereqs:** Pass 14.0; factor
  the greedy core out of `vartext.rs`.
- **15.1 — Reflow surgery + one undo-able `ReflowBlock` command (core + CLI).**
  Apply an accepted `ReflowPreview` via the 14.1 advance-preserving surgery;
  justified via `TJ` (§9.4.3) / `Tw` (§9.3.3); unchanged lines byte-identical;
  incremental save; `CommandKind::ReflowBlock` on `EditSession`; tagged-block
  MCID preservation + disclosure (R72); page-overflow disclose-and-allow (R76);
  `pdfce-cli edit-text --reflow` / a `reflow` subcommand. **Acceptance:** re-wrap
  an embedded-full and a non-embedded block correctly; only the block's
  content-stream object changed (R32/R46); incremental-save-safe (R34) verified;
  justified distributes slack (last line un-justified); page-overflow disclosed +
  content emitted not clipped; undo restores the byte-identical pre-reflow
  stream; R59 + round-trip green; fmt/clippy clean. **Non-goals:**
  cross-block/cross-page (FF-B), Knuth-Plass, hyphenation, composite/CJK/RTL, UI.
  **Prereqs:** 15.0; Pass 14.1 surgery; dispatch `pdfce-spec-librarian` for
  §9.4.3 `TJ` distribution + the §9.3.3 `Tw` single-byte-code-32 caveat.
- **15.2 — Reflow UI: preview overlay + width/alignment adjust + accept/reject
  (gui).** Canvas: invoke reflow on a block; ghost preview vs current;
  drag-adjust block width; alignment picker (L/C/R/justify, pre-filled
  auto-detected); leading; accept → one command, reject → nothing; overflow and
  disclosures surfaced. **Prereqs:** 15.0–15.1 + Pass 14.3; **DISPATCH
  `pdfce-ui-specialist` first.**

**What stays in FF-B:** cross-block reflow (content moving between sibling
blocks) and cross-page reflow (content to/from adjacent pages) — the genuine
exceed-Acrobat headline (Acrobat cloud + English-only). **Justified is removed
from FF-B** (moved to FF-A, §3.1).

## 7. Fast-follows beyond FF-A

FF-B cross-block/cross-page offline reflow (the exceed-Acrobat headline) ·
Knuth-Plass total-fit line-breaking (better justify/rag) · hyphenation
(soft-hyphen + dictionary) · FF-E composite/CJK reflow (couples to decision 012
FF2; `Tw` won't serve — TJ-only justify) · FF-F RTL/bidi reflow.

## 8. Honest limits (named up front)

Single-detected-block scope only (cross-block/cross-page is FF-B) · greedy
first-fit, no Knuth-Plass · no hyphenation; an oversized word overflows unbroken
and is disclosed · simple fonts only (composite/CJK is FF-E; `Tw` word spacing is
single-byte-code-32-only per §9.3.3) · LTR only (RTL/bidi is FF-F) · alignment
auto-detect is a reviewable, overridable heuristic, corpus-tuned · reflow is
always derived and reviewable, never silent.

## 9. Where pdfce exceeds Acrobat

Offline (no cloud, no qualification gate, no English-only limit) within-block
reflow · **reliable offline Justify** (Acrobat's own is community-reported
buggy) · **alignment auto-detect + preserve** (Acrobat has none — risks a silent
left-align on re-wrap) · **page-overflow disclosed, not silently disappeared** ·
reflow as an explicit one-undo reviewable command, not a silent black box ·
first-class scriptable CLI `reflow` (Acrobat has none) · minimal-diff — only the
block's own content stream changes and tags are preserved.

## 10. Revisit triggers

1. Oversized-word overflow or ragged justify shows up frequently on the real
   corpus → consider Knuth-Plass or hyphenation as a later slice.
2. Alignment auto-detect mis-classifies a material share of real blocks → tune
   the x-band thresholds; the reviewable overlay is the meanwhile safety net.
3. Operators frequently want overflow content to flow into the next block/page →
   pull FF-B forward.
4. CJK/RTL reflow requested → FF-E/FF-F, gated on decision 012 FF2 + the bidi
   work.

## 11. What this decision explicitly does NOT decide

- **FF-B cross-block/cross-page reflow** — named, not scoped here.
- **Knuth-Plass, hyphenation** — deferred fast-follows.
- **The exact reflow-interaction UX** — `pdfce-ui-specialist`'s call at 15.2.
- **Composite/CJK/RTL reflow** — FF-E/FF-F, coupled to decision 012 and the bidi
  deferral.
- **Any change to redaction's R35 or the 14.1 default post-edit behavior** —
  both untouched; FF-A is an opt-in beside 14.1.

## 12. References

- **Code:** `text_edit/model.rs` (`Block`/`Line` bboxes, `BlockRecognitionOptions`
  x-band/column geometry, `BlockDiagnostics`); `text_edit/edit.rs` (14.1
  advance-preserving REMOVE→REPLACE, `FollowerDisposition`,
  `EditReport.disclosures`, the "enable reflow" disclosure string); `vartext.rs`
  (`wrap_lines` greedy breaker to factor; `align_x`/`Quadding` `/Q` L/C/R — no
  Justify yet); `edit.rs` `EditSession` (command log; `CommandKind::EditText`/
  `FormatText` — add `ReflowBlock`).
- **Spec:** ISO 32000-1 §9.4.4 (glyph displacement/advance — measurement);
  §9.4.3 (text-showing `TJ` numeric position adjustments — justify slack
  distribution + re-positioning); §9.3.3 (`Tw` word spacing — single-byte code 32
  only; the simple-font caveat and why composite is FF-E); §14.8 S1–S9 (derived
  layout — reflow is layout the file never stated); §14.8.2.3.1 (derived reading
  order); §14.6/§14.7 (marked content / MCID preservation on tagged blocks);
  §12.7.3.3 `/Q` quadding (vartext's existing alignment analog).
- **Parity reference:**
  `D:\Dev\Rag-Specialized\Acrobat_Features\text_edit__paragraph_reflow_and_auto_adjust_layout.md`
  (FF-A grounding session — the justified open question, the page-overflow
  "disappears" finding, the single-block bound);
  `…\text_edit__formatting_options.md` (Justify on the base, non-cloud panel).
- **Decision amended:** `docs/decisions/014-acrobat-text-editing.md` (§3 Reflow,
  §5.3 ladder, §6).

---

## Appendix A — JSON decision block (drives implementation)

```json
{
  "decision_id": "015-ffa-within-block-offline-reflow",
  "title": "FF-A — within-block offline text reflow: explicit reviewable re-wrap of one recognized block, greedy line-breaking, alignment auto-detect/preserve, and justified moved from FF-B into FF-A",
  "status": "Decided",
  "date": "2026-08-01",
  "amends": "docs/decisions/014-acrobat-text-editing.md (§3 Reflow, §5.3 ladder, §6): justified moved OUT of FF-B INTO FF-A; FF-B narrows to cross-block + cross-page only.",
  "one_line": "Ship FF-A as an EXPLICIT, operator-invoked, single-block re-wrap producing a DERIVED preview (greedy first-fit, breaker factored from vartext.rs, NO hyphenation; alignment auto-detected from glyph x-positions and preserved) which the operator adjusts (width/alignment/leading) and accepts as ONE undo-able CommandKind::ReflowBlock applied via 14.1 advance-preserving surgery on only the block's own content-stream object (incremental-save-safe, R34); page-overflow is DISCLOSED-and-allowed, never silently disappeared; JUSTIFIED lands in FF-A as a within-block alignment mode.",
  "eight_settled_calls": {
    "d1_justified": "IN FF-A (amends 014): within-block inter-word slack distribution, an alignment peer of L/C/R, orthogonal to FF-B's cross-block scope; Acrobat exposes Justify on the base non-cloud panel = classic-engine offline capability. Slack via TJ (§9.4.3) / Tw (§9.3.3); last line un-stretched.",
    "d2_line_breaking": "Greedy/first-fit confirmed; EXTEND vartext (factor the packing core into a shared breaker taking a width-measurer; FF-A supplies a provenance-advance measurer); whitespace-only breaks; NO hyphenation; oversized word overflows one line disclosed; Knuth-Plass deferred.",
    "d3_trigger_scope": "EXPLICIT operator action (not auto-on-edit); coexists with 14.1's single-line relayout (does not supersede); unit = exactly one recognized Block, never crossing a sibling/column; wrap width = block bbox width, operator-adjustable.",
    "d4_preview_command": "Derived ReflowPreview (breaks, per-line Tm/TD origins, alignment, new bbox, disclosures) shown as accept/reject overlay; operator adjusts width/alignment/leading live; on accept -> ONE CommandKind::ReflowBlock on EditSession (undo/redo atomic); reject mutates nothing.",
    "d5_alignment": "Auto-detect L/C/R/justified from glyph x-positions (reuse 14.0 x-band geometry); single-line -> left+disclosed; counted in diagnostics; operator-overridable; preserved by default. Exceeds Acrobat (no auto-detect there).",
    "d6_overflow": "DISCLOSE-AND-ALLOW: block grows top-anchored downward; content past the cropbox is disclosed + emitted as real recoverable off-page content, never silently clipped/dropped (rule-4 divergence from Acrobat's silent disappear); never hard-refuse.",
    "d7_minimal_diff": "Only the reflowed block's own content-stream object re-emitted via 14.1 surgery; unchanged lines byte-identical where provable; default INCREMENTAL (R34/R36), not a forced full rewrite; tagged MCID preserved + staleness disclosed (R72).",
    "d8_slicing": "Pass 15.x: 15.0 engine (read-only) / 15.1 surgery + ReflowBlock command + CLI / 15.2 GUI. Librarian may fold as 14.4-14.6. FF-B keeps cross-block/cross-page only."
  },
  "new_standing_rules": {
    "R75": "Reflow is an explicit, reviewable, single-block, one-undo-command operation; 14.1 single-line relayout remains the default, reflow is opt-in.",
    "R76": "Reflow overflow discloses, never disappears; off-page content emitted as real recoverable content at its true position, never clipped-to-deleted.",
    "R77": "Alignment auto-detected from glyph x-positions and preserved through re-wrap; counted, overridable; single-line defaults to left + disclosed. (May fold into R75.)"
  },
  "librarian_ceiling_note": "Current highest standing rule is R74; assign R75-R77. Pass family proposed 15.x (or fold 14.4-14.6). Mark decision 014 §3/§5.3/§6 amended (justified relocated FF-B -> FF-A)."
}
```
