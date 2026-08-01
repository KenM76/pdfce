# 016 — Next Acrobat text-handling parity step: prioritize the FF-* ladder, and scope FF-D (add NEW text as real page content)

**Date:** 2026-08-01
**Status:** Decided
**Decided by:** KenAgent (`autonomous-builder`, decision-consultant mode), per `docs/decisions/README.md`
**Requested by:** `pdfce-engineer` (the next text-handling parity step now that the
in-place editing family — Pass 14.0-14.4 — and within-block reflow — Pass
15.0-15.2 — are all shipped)
**Supersedes:** nothing
**Amends:** `docs/decisions/014-acrobat-text-editing.md` §5.3 (schedules the
named fast-follow **FF-D** into a concrete Pass family); `docs/ROADMAP.md`
(adds the proposed Pass 16.x add-text family and two standing rules R78-R79,
pending librarian numbering; current ceiling R77).
**Builds on:** Pass 14.0 (the derived Run→Line→Block model + per-glyph
provenance), Pass 14.1 (advance-preserving REMOVE→REPLACE content-stream
surgery; the F-refuse font-on-edit gate; bundled/supplied-face *rendering*
already established for the non-embedded-named edit case), Pass 14.2
(size/fill-colour formatting surgery), Pass 14.3 (`EditSession` command log;
`CommandKind`), Pass 15.0-15.2 (greedy within-block reflow engine, alignment
auto-detect/preserve, on-canvas reflow UI), decision 012 (`GlyphSource
{Embedded, Bundled, Supplied}` + `--font-dir`).
**Does not touch:** `LEGAL.md` §1 (license undecided); redaction's R35
full-rewrite guarantee; the 14.1 default post-edit behavior; the Pass-6.2
FreeText annotation path (deliberately kept distinct — §3.1).

---

## 1. Context

Two shipped families now cover Acrobat text-handling at the P0 level:
**in-place editing** (14.0 model / 14.1 surgery + font-gate / 14.2 formatting /
14.3 GUI+session / 14.4 caret-nav/selection/reflow-UI refinements) and
**within-block offline reflow** (15.0 engine / 15.1 surgery + `ReflowBlock`
command + CLI / 15.2 on-canvas UI). Decision 014 §5.3 named an eight-item
fast-follow ladder (FF-A…FF-H); **FF-A is done** (decision 015, Pass 15.x).
This decision does two jobs:

1. **Prioritize** the remaining named items — FF-B, FF-C, FF-D, FF-H — plus the
   still-open **list-authoring** scope question, and name the single
   highest-value NEXT step (§2).
2. **Scope** that step concretely into a decision (§3 onward).

The parity reference is the `pdfce-acrobat-librarian` catalog
(`D:\Dev\Rag-Specialized\Acrobat_Features\text_edit__*.md`). The remaining
ladder items:

- **FF-B** — cross-block / cross-page offline reflow; the *exceed-Acrobat
  headline* (Acrobat's true multi-paragraph reflow is cloud-gated + English-only
  — `text_edit__paragraph_reflow_and_auto_adjust_layout.md`).
- **FF-C** — font-subsetting / glyph-embedding writer; lifts the embedded-subset
  edit refusal (`text_edit__font_handling_on_edit.md`); flagged copyleft-landmine
  (rule 13), needs a permissive-only subsetter + spec dispatch.
- **FF-D** — add NEW text as real page content; Acrobat's "Add Text"
  (`text_edit__add_new_text.md`).
- **FF-H** — `Tc`/`Tw`/`Tz`/`Ts` spacing + synthetic bold/italic + minimal
  StructTree/ActualText update (`text_edit__formatting_options.md`,
  `text_edit__accessibility_tag_structure_interaction.md`).
- **list-authoring** — bulleted/numbered lists; Acrobat exposes it (`must_have`
  in `text_edit__formatting_options.md`), not in any decision's ladder, flagged
  as needing an operator scope call.

### 1.1 Invariants this decision serves

Surgery-not-overlay (R47); minimal-diff / round-trip (R32/R46);
incremental-save-default (R34/R36) vs redaction's separate full rewrite (R35);
the R59 render-fidelity gate; fuzzy-never-sneaky (rule 4); font-on-edit trust
ladder (R71); tagged-edits-disclose-never-corrupt (R73); GUI-core separation
(R21/§3); no copyleft dependency (rule 13); license-undecided gate (rule 8).

---

## 2. The prioritization

Weighed on four axes: (a) real-world frequency/user value; (b) leverage of the
shipped 14.x/15.x substrate vs new subsystems; (c) risk/landmines; (d) whether
an operator decision is required first.

| Rank | Item | Verdict | Value | Leverage | Landmines | Operator gate |
|---|---|---|---|---|---|---|
| **1** | **FF-D** add-new-text | **RECOMMENDED — start now** | very high (among the most common Acrobat text actions) | **maximal** (14.0/14.1/14.2/14.3 + 15.x + decision 012) | **lowest** (no copyleft, no OCR, no bidi/CID; default-font is a documented GAP; must not build as FreeText) | **none** |
| 2 | FF-C subsetting/embedding | HIGH ceiling, **operator-gated** | very high (lifts the single most common editing wall) | moderate (new writer subsystem; reuses skrifa read parser, R21) | **HIGH** — copyleft-dep (rule 13) + license-undecided (rule 8) | **required** |
| 3 | FF-B cross-block/cross-page | DEFER (after FF-D) | prestige high, **daily frequency low** | partial (new engine over 15.x) | largest NEW subsystem; inter-block/page flow | none (sequencing) |
| 4 | FF-H spacing + synthetic + StructTree | DEFER (slice later) | `should_have`; StructTree piece is a real lead | good for spacing; StructTree couples to a not-yet-built a11y subsystem | synthetic bold/italic fuzzy; StructTree premature | none |
| 5 | list-authoring | **operator-gated** scope call; sequences after FF-D | Acrobat `must_have`, but authoring-ish & buggy in Acrobat | builds ON FF-D + reflow | word-processor scope creep | **required** |

**Recommendation: FF-D.** It is the highest-value step that is *both*
solo-startable *and* maximally leveraged. Adding a fresh text run is one of the
most common real Acrobat text operations (not merely repairing existing runs),
and it assembles almost entirely from already-shipped parts: the 14.0 model, the
14.1 surgery (an *insert* rather than a *replace*), 14.2 formatting, the 14.3
`EditSession`/`CommandKind`, and the 15.x reflow engine (for the multi-line box
variant). Its landmine profile is the lowest of the set — and, decisively, it
**does not depend on FF-C**: a new run in a bundled Standard-14 face needs no
glyph embedding and never touches the embedded-subset refusal (§3.3).

**FF-C is ranked #2 on value** — it lifts the single most common wall in real
editing (the embedded-subset "can't originate that glyph" refusal from the
font-handling catalog) — **but it cannot start solo** (§10). **FF-B is the
prestige headline but the least frequent daily action and the largest new
subsystem** — better after FF-D, when operators adding boxed text will
*motivate* cross-block flow. **FF-H** splits: spacing extends 14.2 cheaply, but
its StructTree/ActualText update is premature until an accessibility Pass exists.
**List-authoring** needs an operator "do we want this?" call and sequences after
FF-D anyway.

---

## 3. Decision — scope FF-D

### 3.1 Real page content, NEVER a FreeText annotation (the load-bearing call)

Add-Text synthesizes a **new `BT … ET` text object** (§9.4) appended to the
page's content, producing genuine static page content — editable afterward by
the *same* in-place mechanism as any other run. It is **never** implemented as,
or conflated with, the Pass-6.2 **FreeText annotation** path. The catalog is
explicit: the two "Add Text" features in Acrobat (Edit-PDF page content vs
Fill&Sign typewriter-descended `/FreeText`) are a real, sourced naming collision
with different removal/flatten/permission semantics
(`text_edit__add_new_text.md`, `markup__freetext_annotation.md`). pdfce already
ships the FreeText path (Pass 6.2); FF-D must be structurally distinct, and its
UI must use unambiguous labels (→ `pdfce-ui-specialist`, §8). This is surgery
(page content), kin to R47's sanctioned page-content edits — an *append*, not an
overlay-mask.

### 3.2 Placement — operator origin; point text first, boxed text second

The operator-chosen point sets the new run's **text matrix `Tm`** (§9.4.2); the
CLI takes explicit page coordinates. Two sub-modes, mirroring how 14.x/15.x were
sliced:

- **Point text (16.0)** — a single-line run growing from the origin, no wrap
  box, matching 14.1's single-line posture. Simplest honest first cut.
- **Boxed text (16.1)** — an operator-dragged rectangle; multi-line new text
  wraps through the **already-shipped 15.x reflow engine** (greedy, alignment
  auto/override, top-anchored vertical growth, page-overflow disclose-and-allow
  per R76). No new wrapping logic.

### 3.3 Default font — bundled Standard-14, no embedding (why FF-D needs no FF-C)

Acrobat's exact default font for Add-Text is a documented **GAP**
(`text_edit__add_new_text.md`) — "Let Acrobat choose" resolves to an
unconfirmed default. pdfce therefore **picks and documents its own**: default to
a **bundled Standard-14 permissive face** (§9.6.2.2 — the 14 standard fonts a
conformant processor supplies without embedding; Helvetica-equivalent, matching
Acrobat's Helvetica-for-form-fields precedent), **operator-configurable** (a
preference plus a per-use override), resolved through decision 012's
`GlyphSource` (`Bundled`, or `Supplied` via `--font-dir`).

Because the new run uses a bundled/supplied face saved **by name + code with no
embedding**, it is exactly decision 014's *most-editable* font case
("non-embedded named simple" — no subset limit) and it **never hits the
embedded-subset refusal wall FF-C exists to lift**. This is the crux of why FF-D
delivers real value *independent of* FF-C. The bundled/supplied-face *rendering*
path needed for preview already exists (14.1 established it for typing into
non-embedded runs), so FF-D adds no rendering prerequisite either.

### 3.4 Once added, it IS page text — route through the shipped logic

The new run becomes a **first-class `Run`/`Line`/`Block`** with provenance,
routed through the SAME 14.x edit/format and 15.x reflow logic (the catalog
confirms Acrobat behaves this way too). No separate edit path: the new run is
immediately editable (14.1), formattable — size via `Tf`, fill colour via `rg`,
per 14.2 — and reflowable (15.x, boxed variant). Glyphs are gated by the
existing **F-refuse ladder (R71)**: a character the chosen face covers is
typeable; one it lacks is refused-and-disclosed, never faked. Font source is
disclosed as `Bundled`/`Supplied`, never as the document's own.

### 3.5 Minimal-diff save — append a content stream, don't rewrite

Append the new run as an **additional content stream in the page `/Contents`
array** (§7.7.3.3 — a page's `/Contents` may be an array whose streams
concatenate), leaving the **original content stream byte-identical**; add one
`/Font` entry to the page `/Resources` (§7.8.3) — for a bundled Std-14 face this
is a simple Type1 font dict (`/BaseFont /Helvetica`, `/Encoding`), **no
`FontFile`**. Default **incremental save (R34/R36)**, minimal-diff (R32/R46);
**not** redaction's full rewrite (R35). This gives FF-D an unusually clean
minimal diff: the original page bytes are untouched; only a new stream object
and one resource entry are added.

Applied as **one undo-able `CommandKind::AddText`** on the `EditSession` command
log — atomic exactly like `EditText`/`FormatText`/`ReflowBlock`.

### 3.6 Tagged page — disclose untagged, never corrupt (R73)

New text added to a tagged page is emitted as **untagged page content** (or a
minimal artifact / plain marked-content wrapper) and pdfce **discloses**: *"new
run added as untagged page content; the structure tree's reading order was not
updated — no tag created."* pdfce never silently corrupts the structure tree and
never fabricates a mis-placed structure element (R73 — the same
disclose-not-corrupt posture 14.1 uses; Acrobat's own edits corrupt the tree).
Minimal StructTree/ActualText insertion for new content is **FF-H**.

---

## 4. Rationale (condensed)

- **Why FF-D over the alternatives** (§2): highest product of frequency ×
  leverage × low-risk × solo-startable. FF-C outranks it on ceiling value but is
  operator-gated; FF-B is rarer and heavier; FF-H is `should_have` + partly
  premature; list-authoring needs a scope call.
- **Why page content, not FreeText** (§3.1): the two are different features with
  different flatten/permission/removal semantics; the catalog's sourced naming
  collision is exactly the trap to avoid. pdfce's FreeText path already exists —
  reusing it would silently ship the wrong feature.
- **Why a bundled Std-14 default** (§3.3): Acrobat's default is a GAP, so pdfce
  documents its own; a Std-14 face needs no embedding (§9.6.2.2), which is
  precisely what decouples FF-D from FF-C and makes it buildable today.
- **Why route as ordinary page text** (§3.4): the catalog confirms added text is
  thereafter edited like any other run; reusing the 14.x/15.x pipeline is both
  the honest behavior and the cheapest — no parallel edit path to maintain.
- **Why append-a-stream** (§3.5): it yields the cleanest possible minimal diff
  (original bytes untouched) and honors incremental-save-default without any
  redaction-style full rewrite.
- **Why disclose-untagged** (§3.6): fabricating a structure element would be the
  "sneaky" failure rule 4/R73 forbid, and would risk the mis-placement Acrobat's
  own edits cause; disclosing is the honest, minimal-diff-consistent path.

---

## 5. Standing rules (binding; add to `ROADMAP.md` — librarian assigns final `Rnn`; current ceiling R77)

- **R78 — Add-new-text is page-content surgery, never a FreeText annotation.**
  The operation synthesizes a new `BT…ET` text object appended to the page
  `/Contents` (original streams byte-verbatim), routed into the same 14.x
  model/edit/format + 15.x reflow as existing page text, applied as ONE
  undo-able `CommandKind::AddText`; it is NEVER implemented as or conflated with
  the Pass-6.2 FreeText annotation path (distinct removal/flatten/permission
  semantics).
- **R79 — New text uses a bundled/supplied face by name+code, no embedding,
  with disclosed provenance.** A newly-added run defaults to a bundled
  Standard-14 permissive face (§9.6.2.2 — no embedding, sidestepping the FF-C
  embedded-subset wall), operator-configurable via decision 012 `GlyphSource`; a
  glyph the chosen face lacks is refused-and-disclosed (R71), never faked; the
  run's font source is disclosed (`Bundled`/`Supplied`), never presented as the
  document's own.

(The tagged-page "disclose untagged, never corrupt" behavior in §3.6 is governed
by the existing **R73** — no new rule needed.)

---

## 6. Pass slicing (proposed Pass 16.x — the "text authoring" family; librarian finalizes numbering)

Proposed **Pass 16.x** to keep 14.x = "in-place editing" and 15.x = "reflow"
coherent; the librarian may instead continue as **14.5-14.7** — the renumbering
is the librarian's call and changes nothing structural.

- **16.0 — Add-new-text engine + point-text insert (core + CLI).** Synthesize a
  new `BT…ET` object at operator coordinates (`Tm`, §9.4.2); append as a new
  content stream in the page `/Contents` array (§7.7.3.3) leaving the original
  byte-identical; add one page-`/Resources` `/Font` entry (§7.8.3);
  default-font policy (bundled Std-14, §9.6.2.2, decision 012 `GlyphSource`,
  configurable); route the new run into the 14.0 model as a first-class
  `Run`/`Line`/`Block`; F-refuse gate reuse (R71); 14.2 size/colour; incremental
  save; tagged-page untagged disclosure (R73); one `CommandKind::AddText`;
  `pdfce-cli text add --at "x,y" --text "…" [--font NAME|auto] [--size N]`.
  **Acceptance:** a new run is added and the original content stream is
  byte-verbatim (R32/R46); the new run appears as a recognized, editable
  (via 14.1) and formattable (via 14.2) block; a glyph the face lacks is
  refused-and-disclosed; a tagged-page add emits the untagged disclosure; a
  supplied face lifts coverage and is disclosed `Supplied`; incremental-save-safe
  (R34); R59 + round-trip green; **no new dependency**; core has no GUI dep
  (`cargo tree -p pdfce-core`); Pass 14.x/15.x tests unchanged; fmt/clippy clean.
  **Non-goals:** boxed wrap (16.1), lists, StructTree insertion (FF-H), any
  FreeText conflation, composite/CJK/RTL new text (FF-E/FF-F). **Prereqs:** Pass
  14.0-14.2, decision 012; **dispatch `pdfce-spec-librarian`** for §7.7.3.3
  (`/Contents` array concatenation), §7.8.3 (resource dicts), §9.4/§9.4.2
  (text objects / `Tm`), §9.6.2.2 (Standard-14 no-embed), and the new-content
  encoding path.
- **16.1 — Boxed text add + wrap via the 15.x reflow engine (core + CLI).**
  Operator-dragged box; multi-line new text wraps through the shipped 15.x
  greedy reflow (alignment auto/override; top-anchored growth; page-overflow
  disclose-and-allow, R76); `pdfce-cli text add --box "x,y,w,h" …`.
  **Acceptance:** multi-line new text wraps to the box width; alignment applies;
  overflow disclosed (R76) and emitted as real recoverable content; the whole
  add lands as one `CommandKind::AddText`; gates green; fmt/clippy clean.
  **Non-goals:** lists, cross-block flow (FF-B), StructTree insertion (FF-H).
  **Prereqs:** 16.0 + Pass 15.0-15.1.
- **16.2 — Add-text UI on the Pass 12.0 canvas (gui).** Click → place point
  text; drag → place a wrap box; type → live preview; commit → one
  `CommandKind::AddText`, cancel → nothing. Uses **unambiguous labels**
  distinguishing "add page text" from "add FreeText annotation" (the catalog
  naming-collision finding). Disclosures (font provenance, tagged-untagged,
  overflow) surfaced verbatim via 14.3's disclosure channel. **Prereqs:**
  16.0-16.1 + Pass 12.0/14.3; **DISPATCH `pdfce-ui-specialist` first.**

---

## 7. Fast-follows beyond FF-D

FF-B cross-block/cross-page reflow (the exceed-Acrobat headline) · FF-C
font-subsetting/embedding (operator-gated; lifts the subset refusal) · FF-H
`Tc`/`Tw`/`Tz`/`Ts` spacing + synthetic bold/italic + minimal
StructTree/ActualText update (including a structure element for FF-D's new
content) · list-authoring (operator scope call; builds on FF-D + reflow) ·
FF-E composite/CJK new text · FF-F RTL/bidi new text.

---

## 8. Honest limits (named up front)

Point + single-box add only (no auto-flow across boxes/pages — that is FF-B) ·
new content is UNtagged and its structure-tree absence is disclosed (structure
insertion is FF-H) · simple fonts only — new composite/CJK/RTL runs are FF-E/FF-F
· preview fidelity depends on the bundled/supplied face and is disclosed as such
(a bundled Std-14 face renders with pdfce's own metrics/outlines, not a
document-embedded program) · default-font policy is pdfce's own documented
choice, not a reverse-engineered Acrobat default (Acrobat's is a GAP) · new text
is always an explicit, reviewable, one-undo operator action, never silent.

---

## 9. Where pdfce reaches / exceeds Acrobat

Reaches Acrobat's Add-Text baseline (real page content, immediately re-editable)
· **exceeds** on minimal diff — the original content stream stays byte-identical
(append-a-stream), where Acrobat rewrites · **exceeds** on tagged-PDF honesty —
new content is disclosed-untagged, never a silent structure-tree corruption
(Acrobat's edits corrupt the tree) · **exceeds** with a first-class scriptable
`pdfce-cli text add` (Acrobat has no CLI) · **exceeds** with a documented,
deterministic default-font policy where Acrobat's is opaque/undocumented ·
disclosed font provenance (`Bundled`/`Supplied`) rather than silently presenting
a substitute as the document's own.

---

## 10. Operator-owned decisions (do NOT proceed without an operator call)

- **FF-C (font subsetting / glyph embedding) — license/copyleft escalation.**
  FF-C adds a Cargo dependency (a font subsetter). Per rule 13 its license must
  be classified and, if copyleft, **approved by the operator, never solo**; and
  per rule 8 pdfce's own license is still undecided, which gates what is even
  usable. **Recommendation:** unblock in parallel with FF-D — operator approves a
  permissive-only subsetter path and, ideally, settles the license — so FF-C can
  follow FF-D; queue the `pdfce-spec-librarian` font-subsetting dispatch (already
  named in decision 014) meanwhile.
- **List-authoring (bulleted/numbered) — scope call.** Acrobat exposes it
  (`must_have` in `text_edit__formatting_options.md`), but it is a
  word-processor-ish *authoring* feature somewhat outside "fix the PDF's own
  text," Acrobat's own implementation is community-reported buggy, and it has no
  home in any decision's ladder. It needs an explicit operator **"do we even want
  this?"** decision before entering a Pass, and it sequences naturally *after*
  FF-D (a list item is new/edited content + a list model + reflow), so deferring
  costs nothing.

---

## 11. What this decision explicitly does NOT decide

- **FF-C, FF-B, FF-H** — prioritized (§2), not scoped here; FF-C is
  operator-gated (§10).
- **List-authoring** — operator scope call (§10); not scheduled.
- **The exact Add-Text interaction UX** (point vs box affordance, live-preview
  rendering, label wording) — `pdfce-ui-specialist`'s call at 16.2.
- **A structure element for FF-D's new content on tagged PDFs** — FF-H; FF-D
  discloses-untagged only (R73).
- **Composite/CJK/RTL new text** — FF-E/FF-F.
- **The named Pass-14.3 property-bar "commit-on-focus-loss" nicety** — a minor
  usability item; may ride along with 16.2 at the engineer's discretion, not
  scoped here.
- **Any change to redaction's R35, the 14.1 default post-edit behavior, or the
  Pass-6.2 FreeText path** — all untouched.

---

## 12. Revisit triggers

1. A corpus measurement shows operators frequently want new text to **flow
   across boxes/pages** → pull FF-B forward.
2. The **embedded-subset refusal** dominates edits (existing text, not just new)
   → escalate FF-C's license gate (§10) and schedule it.
3. Operators frequently want new text **tagged/in reading order** → FF-H's
   StructTree insertion.
4. **CJK/RTL** new text requested → FF-E/FF-F, gated on decision 012 FF2 + the
   bidi work.
5. The operator answers the **list-authoring** scope question "yes" → scope it as
   a Pass on top of FF-D + reflow.

---

## 13. References

- **Code:** `text_edit/model.rs` (`Run`/`Line`/`Block`, provenance);
  `text_edit/edit.rs` (14.1 advance-preserving surgery, `FollowerDisposition`,
  `EditReport.disclosures`, F-refuse gate); `text_edit` reflow (15.x
  `ReflowEngine`/`ReflowPreview`/`CommandKind::ReflowBlock`); `edit.rs`
  `EditSession` (`CommandKind::EditText`/`FormatText`/`ReflowBlock` — add
  `AddText`); decision 012's `GlyphSource {Embedded, Bundled, Supplied}` +
  `FontEnvironment.named` + `--font-dir`; the Pass 12.0 canvas.
- **Spec:** ISO 32000-1 §9.4 (text objects `BT…ET`); §9.4.2 (`Tm`
  text-positioning); §9.4.3 (`Tj`/`TJ` show operators); §9.6.2.2 (Standard-14
  fonts — available without embedding); §9.10 (encoding); §7.7.3.3 (Page object
  — `/Contents` may be an array of concatenated streams: the append mechanism
  that keeps the original stream byte-identical); §7.8.3 (resource dictionaries
  — the new `/Font` entry); §14.6/§14.7 (marked content / artifacts for the
  new-content wrapper); §14.8 S1-S9 (derived layout — the new run's block
  recognition is still derived).
- **Parity reference:** `…\Acrobat_Features\text_edit__add_new_text.md`
  (page-content-vs-annotation disambiguation; default-font GAP; placement);
  `…\text_edit__font_handling_on_edit.md` (the font-availability matrix; why a
  bundled/supplied face sidesteps the subset wall); `…\text_edit__formatting_options.md`
  (list-authoring `must_have` + operator-scope flag; the reusable-style
  parity-plus note); `…\text_edit__accessibility_tag_structure_interaction.md`
  (disclose-not-corrupt on tagged content);
  `…\text_edit__paragraph_reflow_and_auto_adjust_layout.md` (FF-B cloud/
  English-only framing — why it is deferred, not the next step).
- **Decisions/rules:** decision 014 (§5.3 ladder — FF-D scheduled here; §4.3
  F-refuse; the font matrix); decision 015 (FF-A / 15.x reflow — reused by 16.1);
  decision 012 (operator-supplied fonts, `GlyphSource`); `ROADMAP.md`
  R21/R32/R34/R35/R36/R46/R47/R59/R71/R73/R76; rule 4 (fuzzy-never-sneaky);
  rule 8 (license undecided); rule 13 (no copyleft dependency).

---

## Appendix A — JSON decision block (drives implementation)

```json
{
  "decision_id": "016-ffd-add-new-page-text",
  "title": "Prioritize the next Acrobat text-handling parity step, and scope it: FF-D — add NEW text as real page content (point + boxed), reusing the 14.x edit substrate and 15.x reflow, with a bundled Standard-14 default font (no embedding, no FF-C dependency)",
  "status": "Decided",
  "date": "2026-08-01",
  "one_line": "Recommend FF-D as the next step and scope it: synthesize a NEW BT/ET text object at operator coordinates, APPEND it as a new content stream in the page /Contents array (§7.7.3.3) so the original stream stays byte-identical, default it to a BUNDLED Standard-14 permissive face (§9.6.2.2 — no embedding, sidesteps the FF-C embedded-subset wall) via decision 012 GlyphSource (operator-configurable), route the new run as a first-class Run/Line/Block through the SAME 14.x edit/format + 15.x reflow logic, gate glyphs via F-refuse (R71), disclose tagged-page new content as untagged (R73), save incremental (R34/R36), never conflate with the Pass-6.2 FreeText annotation path; ship it as Pass 16.x (16.0 point-insert engine+CLI / 16.1 boxed add + 15.x wrap / 16.2 canvas UI).",
  "prioritization_ranked": [
    {"rank": 1, "item": "FF-D add-new-page-text", "verdict": "RECOMMENDED — start now", "operator_gate": "none — solo-startable"},
    {"rank": 2, "item": "FF-C font-subsetting/glyph-embedding", "verdict": "HIGH ceiling but OPERATOR-GATED (rule 13 copyleft + rule 8 license)", "operator_gate": "REQUIRED"},
    {"rank": 3, "item": "FF-B cross-block/cross-page reflow", "verdict": "DEFER — after FF-D (rarest daily action, largest new subsystem)", "operator_gate": "none (sequencing)"},
    {"rank": 4, "item": "FF-H spacing + synthetic + StructTree", "verdict": "DEFER — slice later (StructTree couples to a nonexistent a11y subsystem)", "operator_gate": "none"},
    {"rank": 5, "item": "list-authoring", "verdict": "OPERATOR-GATED scope call; sequences after FF-D", "operator_gate": "REQUIRED"}
  ],
  "eight_settled_calls": {
    "d1": "Real page content (new BT/ET appended to /Contents), NEVER a Pass-6.2 FreeText annotation (R78).",
    "d2": "Operator origin sets Tm (§9.4.2); point text first (16.0), boxed second (16.1).",
    "d3": "Default bundled Standard-14 face (§9.6.2.2, no embedding) via decision 012 GlyphSource, configurable — this is why FF-D needs no FF-C (R79).",
    "d4": "Once added, routed as first-class page text through the same 14.x edit/format + 15.x reflow.",
    "d5": "Glyphs gated by F-refuse (R71); font provenance disclosed Bundled/Supplied.",
    "d6": "Append a content stream to the /Contents array (§7.7.3.3) — original byte-identical; +1 /Font entry (§7.8.3); incremental save (R34/R36); one CommandKind::AddText.",
    "d7": "Tagged page: new content is untagged + disclosed (R73), never silent structure-tree corruption.",
    "d8": "Pass 16.x: 16.0 point-insert engine+CLI / 16.1 boxed+wrap / 16.2 canvas UI (unambiguous add-page-text vs add-FreeText labels)."
  },
  "new_standing_rules": {
    "R78": "Add-new-text is page-content surgery (new BT/ET appended to /Contents, routed into the 14.x/15.x pipeline, one undo-able CommandKind::AddText), NEVER a FreeText annotation.",
    "R79": "New text uses a bundled/supplied face by name+code, no embedding (default bundled Std-14, §9.6.2.2, decision 012 GlyphSource); missing glyph refused-and-disclosed (R71); provenance disclosed, never presented as the document's own."
  },
  "operator_gated": {
    "FF-C_license": "Adding a font subsetter is a Cargo dependency → rule 13 (copyleft classification/approval, never solo) + rule 8 (license undecided). Recommend unblocking in parallel (permissive-only subsetter + ideally the license) so FF-C follows FF-D.",
    "list_authoring": "Needs an explicit operator 'do we want word-processor list authoring?' call; sequences after FF-D regardless."
  },
  "librarian_ceiling_note": "Current highest standing rule is R77; assign R78-R79. Pass family proposed 16.x (or fold as 14.5-14.7). Tagged-page disclosure governed by existing R73. Mark decision 014 §5.3 amended (FF-D scheduled). Do NOT schedule FF-C or list-authoring without the operator calls in §10."
}
```
