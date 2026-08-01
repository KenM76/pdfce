# 014 — Acrobat-style in-place text editing: model, surgery, font-on-edit, reflow

**Date:** 2026-07-31
**Status:** Decided
**Amended by:** `docs/decisions/015-ffa-within-block-offline-reflow.md`
(2026-08-01) — §3 (Reflow), §5.3 (fast-follow ladder), and §6 below are
each amended: **justified alignment moves OUT of FF-B and INTO FF-A**;
FF-B's headline narrows to cross-block + cross-page offline reflow
only. See the dated footnotes at each affected section; this header
line is the pointer, the sections below are the historical record
(not rewritten).
**Decided by:** KenAgent (`autonomous-builder`), per `docs/decisions/README.md`
**Requested by:** `pdfce-engineer` (the operator's explicitly-directed NEXT
MAJOR FOCUS — Acrobat text-handling parity, ahead of Inkscape/vector breadth;
ROADMAP "★ NEXT MAJOR FOCUS", filed 2026-08-01)
**Supersedes:** nothing
**Amends:** `docs/ROADMAP.md` (adds the Pass 13.x text-editing family and six
standing rules); the decision-008 Pass-9 vector ordering (already repositioned
behind Acrobat-text by the ROADMAP directive — this decision fills the "route
through KenAgent (~014)" placeholder that directive named)
**Builds on:** decision 004 (fonts/seam), decision 012 (operator-supplied
fonts / GlyphSource), Pass 4 (text extraction + derived layout), Pass 6.1/6.2
(content serializer / vartext), Pass 8.0 (advance-preserving surgery), Pass
12.0 (canvas substrate)
**Does not touch:** `LEGAL.md` §1 (license undecided); redaction's R35
full-rewrite guarantee (editing is a DISTINCT operation — see §4.5)

---

## 1. Context

The operator directed that pdfce reach parity with **Adobe Acrobat's
text-handling** — "Edit PDF": selecting and re-editing the glyph runs already
in a page's content stream, plus paragraph recognition, reflow, and
formatting — ahead of the Inkscape/vector-editing breadth. This is a NEW major
subsystem, distinct from two shipped things it must not be confused with:

- the text **EXTRACTION** path (Pass 4) — reads glyphs OUT; and
- the text-bearing **ANNOTATIONS** path (Pass 6.2) — authors FreeText/Stamp
  overlays ON TOP of the page.

This decision is about editing the page's **own** content. The parity
reference is the `pdfce-acrobat-librarian` catalog
(`D:\Dev\Rag-Specialized\Acrobat_Features\text_edit__*.md`, 9 files). Its four
load-bearing findings frame every hard call below:

1. **Font-on-edit is make-or-break.** Editing needs NEW glyph outlines that an
   embedded SUBSET cannot supply. Acrobat requires the font available LOCALLY
   (it uses the system copy, not the embedded subset) for character-level
   editing. Embedded-but-not-local → only colour/size on existing glyphs.
   Neither-embedded-nor-local → inconsistent (block vs substitute).
2. **Reflow is limited.** Acrobat's OFFLINE engine reflows only WITHIN a single
   detected block. True multi-paragraph reflow (2021+) is CLOUD-gated and
   English-only. So a fully-offline reflow is an **exceed-Acrobat** opportunity.
3. **Outlined/vectorized text is permanently unreachable; scanned/raster text
   needs OCR first.**
4. **Editing a TAGGED PDF corrupts the accessibility tree in Acrobat.** pdfce's
   minimal-diff invariant is *protective* here — a differentiator.

### 1.1 The foundations already exist

This subsystem is mostly assembly of proven parts, not new invention:

- **Pass 4 derived layout** (`text_extract/layout.rs`) already turns positioned
  glyphs into `TextRun`s and inserts DERIVED line breaks and word spaces from
  three geometry ratios (`line_gap_ratio`, `backward_jump_ratio`,
  `word_gap_ratio`), and already documents — from ISO 32000-1 §14.8, negative
  results **S1-S9** — that *word/line/paragraph/column/reading-order do not
  exist in an untagged content stream*. Block recognition is a second
  clustering pass over that same output.
- **Pass 8.0 surgery** (`redact.rs`) already parses the content stream, finds
  `Tj`/`TJ` show operators, and — critically — replaces a removed run with a
  `TJ` numeric adjustment that preserves the §9.4.4 advance so un-edited text
  stays put. Editing extends this from REMOVE to REPLACE.
- **Pass 6.2 vartext** (`vartext.rs`) already does line-breaking/quadding for
  variable-text appearances — the reflow line-breaker.
- **Decision 012** shipped operator-supplied fonts (`--font-dir`,
  `GlyphSource {Embedded, Bundled, Supplied}`, the `FontEnvironment.named`
  seam) — pdfce's analog of Acrobat's "local font copy," and the font-on-edit
  enabler.
- **Pass 12.0 canvas** shipped the focusable canvas, `canvas_to_pdf_space`
  transform, `CanvasTargetProvider`, and selection scaffold — the edit UI
  surface (R60: exactly one canvas substrate).

### 1.2 Invariants this decision must serve

Round-trip/minimal-diff (§5, R32/R46); surgery-vs-overlay distinction (R47);
save-mode-by-contract (R36) and redaction's separate full-rewrite guarantee
(R35/R58); the R59 render-fidelity gate; fuzzy-never-sneaky (rule 4);
GUI-core separation; R21 (one font parser); rule 13 (no copyleft dependency).

---

## 2. Options considered

**Text model / block recognition.**
- **M-flat** — edit at the run level only, no block model. (Rejected: no
  selection scope, no caret navigation, no reflow target.)
- **M-hier** — Run→Line→Block hierarchy derived from glyph geometry, reviewable.

**In-place edit mechanism.**
- **E-overlay** — mask the old text with a white box and draw new text on top
  (the Pass-6.x overlay path). (Rejected: false editing — the old text
  survives underneath, violating rule 4 and §5's intent; it is what redaction
  exists NOT to do.)
- **E-surgery** — rewrite the show operators in place (extend Pass 8.0).

**Relayout scope of a single edit.**
- **RL-line** — relayout only the edited line (advance-preserving); the line
  may overflow the margin.
- **RL-block** — re-wrap the whole block (reflow) on every edit.

**Font-on-edit (the crux).**
- **F-refuse** — edit only with glyphs the run's font can already provide;
  refuse-and-disclose a missing glyph.
- **F-substitute** — silently substitute a face for the whole run.
- **F-embed** — embed the new glyph via font subsetting (a new writer
  capability).

**Reflow.**
- **RF-none** (first cut) / **RF-within-block** (greedy vs Knuth-Plass) /
  **RF-cross-block** (offline — exceeds Acrobat).

**Tagged-PDF handling.**
- **T-ignore** (corrupt like Acrobat) / **T-disclose** (preserve the MCID
  wrapper, disclose staleness) / **T-update** (minimally refresh the structure
  tree).

---

## 3. Decision

**Text model — M-hier.** A NEW `pdfce-core` module builds a **Run→Line→Block**
hierarchy on top of Pass 4's extraction: baseline-Y clustering into lines,
x-band clustering into columns (reusing `layout.rs`'s backward-jump signal),
and leading/indent segmentation into paragraphs. **Everything is DERIVED**
(§14.8 S1-S9), COUNTED, and REVIEWABLE — the operator can split/merge/resize/
reorder blocks (rule 4). A sourced-only view always remains. Each Run/Glyph
gains **provenance**: its source show-operator identity, byte span, and full
text-state (font resource, `Tf` size, fill colour, matrix) — the substrate the
surgery needs.

**Edit mechanism — E-surgery.** Editing extends the Pass 8.0 advance-preserving
interpreter from REMOVE to REPLACE: locate the show operator(s), re-encode the
new string in the run's font encoding (an inverse of Pass 4's §9.10.2 decode,
built from the font's Encoding/Differences), re-emit the run, and preserve the
§9.4.4 advance so un-edited text stays put. Only the edited content stream(s)
(+ any changed resource/font dict) are re-emitted; everything else is
byte-verbatim (R32/R46). This is **surgery, never overlay** (R47).

**Relayout scope — RL-line for the first cut.** After an edit, shift the rest
of the SAME line by the advance delta; the line may grow past the original
margin (DISCLOSED). This matches Acrobat's own non-reflow in-place behavior.
Block re-wrap (reflow) is a fast-follow.

**Font-on-edit — F-refuse (primary); F-substitute only as an explicit,
disclosed operator choice; F-embed DEFERRED as the named writer fast-follow.**
A keystroke is applied only when the run's font can already provide the glyph.
The four cases:

| Font case | First-cut behavior |
|---|---|
| Embedded **full** program | Edit freely within the program's coverage; re-encode; saveable (`GlyphSource::Embedded`). |
| Embedded **subset** program | Edit within glyphs the subset ALREADY carries (typo fixes reusing present letters, deletes, rearranges — always fine). A glyph the subset lacks → **REFUSE-and-disclose** by name. This is exactly Acrobat's "embedded-but-not-local" floor. |
| **Non-embedded** named simple | The most editable case — no subset limit. Type any character the bundled Base-14 or an operator-**supplied** face (decision 012) covers; **saveable by name+code, no embedding**. The face is needed only for the render/preview. |
| **Non-embedded composite (CID)** | DEFERRED (named non-goal); couples to decision 012 FF2 + FF-E. No Acrobat baseline. |

This ships real editing for the three high-coverage cases **without a font
embedder**, and names the single refusal case precisely instead of faking a
glyph. It reuses decision 012's GlyphSource + `--font-dir` verbatim: supplied
fonts are pdfce's "local font," and the future source outline for FF-C.

**Reflow — RF-none first, then the ladder.** First cut ships no reflow. FF-A:
within-block offline reflow (**greedy** line-break, reuse `vartext.rs`;
left/right/center) — reaches Acrobat's OFFLINE parity. FF-B: cross-block/
cross-page offline reflow — the **exceed-Acrobat** headline (Acrobat is
cloud-only + English-only; pdfce offline + script-agnostic) + justified
alignment. All reflow is a DERIVED PREVIEW the operator accepts.

> **AMENDED 2026-08-01 by decision 015 §3.1:** the "+ justified alignment"
> clause above is superseded. Justified moves OUT of FF-B and INTO FF-A as a
> fourth within-block alignment mode (peer of left/right/center) — Acrobat
> exposes Justify on its BASE (non-cloud) Edit-Text panel, proving it is a
> classic-engine, single-block capability, not a cross-block/cloud one. FF-B's
> scope narrows to cross-block + cross-page reflow only. See
> `docs/decisions/015-ffa-within-block-offline-reflow.md` §3.1 and
> `ROADMAP.md`'s ★ Pass 15.x entry.

**Formatting — first cut = size, fill colour, gated family/style.** Size →
`Tf`; fill colour → `rg`/`g`/`k`; font-family/style change → `Tf` with a
different resource, gated by availability + coverage (else refuse-and-disclose,
re-encode into the new face). Size/colour need no new glyphs (Acrobat's floor
allows exactly these) and always work. `Tc`/`Tw`/`Tz`/`Ts` spacing/scale/rise
and synthetic bold/italic (disclosed-fuzzy) are FF-H.

**Save mode — default INCREMENTAL (R36), NOT redaction's full rewrite.** Prior
text survives in history BY DESIGN and this is disclosed; truly removing text
is REDACTION (Pass 8, R35), a different operation, never conflated. The R59
render gate re-runs on any edited page; the round-trip gate stays green for
untouched objects.

**Tagged-PDF — T-disclose (disclose-not-corrupt).** Preserve the BDC/EMC + MCID
wrapper around the edited show operators so the structure tree's references
stay valid, and disclose "text changed; structure tree's /ActualText and
reading-order not updated." This BEATS Acrobat (which silently corrupts).
Minimal StructTree/ActualText update is FF-H.

---

## 4. Rationale

### 4.1 Why a derived, reviewable block model (not flat, not authoritative)

The PDF says nothing about words, lines, paragraphs, columns, or reading order
in an untagged content stream — this is not modesty, it is `layout.rs`'s
sourced position (§14.8 S1-S9). But an editor NEEDS those concepts: a caret
walks a line, a selection spans runs, a reflow targets a block width. The only
honest reconciliation is to DERIVE the structure, COUNT every inference, and
make it REVIEWABLE — the operator sees the recognized blocks and corrects them
before editing. Flat run-only editing (M-flat) can't express selection scope or
a reflow target; a *silent* authoritative structure would be the "sneaky"
failure rule 4 forbids. M-hier reuses the exact three ratios Pass 4 already
proved, so the block layer is one clustering pass, not new geometry.

### 4.2 Why surgery, never overlay

E-overlay (white box + new text on top) is *false editing*: the original glyphs
survive underneath, extractable and copyable — precisely the failure mode
redaction exists to prevent. E-surgery rewrites the actual show operators, so
the edited text IS the content. It also inherits Pass 8.0's hardest-won
property: the §9.4.4 advance-preservation that keeps un-edited survivors from
sliding. R47 already draws the surgery-vs-overlay line (annotation edits never
touch page content; redaction does); text editing is the second sanctioned
page-content surgery, and it borrows Pass 8.0's machinery wholesale.

### 4.3 Why F-refuse is the correct font-on-edit posture

The temptation is to "just draw the glyph" — but with what outline? An embedded
subset physically lacks it, and a substituted face changes every advance (so
the operator's page would differ from every other reader's). F-substitute
silently is the rule-4 violation. F-embed is *correct* but requires a font
subsetter — a large new writer subsystem with a real copyleft-landmine (rule
13). F-refuse ships editing for the three cases that DON'T need a new glyph —
embedded-full (has it), embedded-subset-within-existing-glyphs (typo fixes are
mostly this), and non-embedded-named (bounded only by the bundled/supplied
font's coverage, saveable by name+code) — and names the ONE case it can't yet
serve. That is a large, honest, buildable first cut. And it maps decision 012
directly onto Acrobat's finding: Acrobat needs the LOCAL font; decision 012's
`--font-dir` IS pdfce's local-font supply, so a supplied Calibri lifts a
non-embedded Calibri run's coverage exactly as Acrobat's local Calibri would —
and is disclosed as `Supplied`, never as the document's own.

### 4.4 Why offline reflow is the exceed-Acrobat play

Acrobat's OFFLINE reflow is within-block only; its cross-block reflow (2021+)
is cloud-gated AND English-only. pdfce runs entirely offline (decisions 003/004
determinism) and script-neutral within its font limits. So FF-A merely *reaches*
Acrobat's offline parity, while FF-B — cross-block offline reflow — is a genuine
lead: a capability Acrobat gates behind a cloud round-trip and one language.
Greedy line-breaking (reusing `vartext.rs`) matches Acrobat's within-block
behavior and is honest; Knuth-Plass is better typography at far more cost and is
deferred. Reflow stays DERIVED and reviewable — it is layout the file never
stated.

### 4.5 Why editing is incremental, and redaction is not

Redaction forces a full rewrite (R35/R58) because it is a SECURITY operation:
the removed bytes must not survive in any prior revision. Editing is NOT
security — it is a content change. Forcing a full rewrite on every keystroke
would be wrong (it would drop the document's revision history and violate the
minimal-diff default). So editing uses the default incremental save (R36): the
changed content stream is re-emitted, prior text remains in history — and pdfce
DISCLOSES this, pointing the operator at redaction if they need the old text
truly gone. Conflating the two would either weaken redaction's guarantee or
bloat every edit; keeping them distinct honors both R35 and R36.

### 4.6 Why disclose-not-corrupt on tagged PDFs (the Acrobat-beating property)

Acrobat's edit silently corrupts the structure tree — a known accessibility
regression. pdfce's minimal-diff surgery only perturbs the content stream, and
the structure tree references it by MCID. By preserving the BDC/EMC+MCID wrapper
around the edited operators, pdfce keeps the tree's references VALID; the only
staleness is the text content an /ActualText or reading-order note describes,
which pdfce DISCLOSES rather than silently corrupting. This is minimal-diff
turned into an accessibility guarantee — a real differentiator — and a minimal
structure-tree refresh is a clean fast-follow.

### 4.7 Why it costs no new dependency for the first cut

13.0-13.3 reuse Pass 4 extraction, Pass 8.0 surgery, `vartext` line-breaking,
decision 012 GlyphSource, and the Pass 12.0 canvas. The inverse-encoding builder
uses the one skrifa parser already in the read path (R21). Only FF-C (font
subsetting) would add a crate — gated permissive-only (rule 13) and flagged
early so it never blocks the shipping slices.

---

## 5. What this decision produces

### 5.1 Standing rules (binding; add to `ROADMAP.md` — librarian assigns the final `Rnn`, reconciling the filed R61 and decision 012's unfiled R61-R65)

- **Text edit is surgery, never overlay.** In-place edits rewrite the page
  content stream via advance-preserving surgery (kin to Pass 8.0), NOT the
  Pass-6.x overlay-append path (R47). Only edited content streams (+ changed
  resource/font dict) are re-emitted; everything else byte-verbatim.
- **Text edit is incremental, not a scrub.** Editing uses the default
  incremental save (R36); prior text survives in history by design and this is
  disclosed. Truly removing text is REDACTION (Pass 8, R35) — never conflated.
- **Font-on-edit trust ladder.** A keystroke is applied only when the run's
  font can already provide the glyph (embedded program's existing glyphs, or a
  non-embedded font's full bundled/supplied coverage — decision 012). A glyph
  an embedded SUBSET lacks is REFUSED with a named disclosure — never faked,
  never silently substituted. Font-subsetting/glyph-embedding is a deferred
  writer subsystem, permissive-only (rule 13).
- **Recognized blocks and reflow are reviewable hints.** Block recognition and
  reflow are DERIVED (§14.8 S1-S9), counted, and presented as a reviewable
  structure the operator accepts/corrects — never a silent re-layout (rule 4).
- **Tagged edits disclose, never corrupt.** An edit inside a marked-content
  sequence preserves its BDC/EMC+MCID wrapper (references stay valid) and
  discloses that /ActualText and reading-order were not updated. pdfce never
  silently corrupts the accessibility tree.
- **Text model/edit/reflow in core; edit UI in gui.** The model, hit-test,
  caret/selection, surgery, and reflow live in `pdfce-core` (no GUI dep,
  verified by `cargo tree`); the canvas interaction lives in `pdfce-gui` on the
  R60 canvas; the CLI gets a scriptable `edit-text` subcommand.

### 5.2 Pass slicing (the text-editing family — proposed Pass 13.x; librarian assigns the actual number)

- **13.0 — Editable text model + block recognition (READ-ONLY, core + CLI
  inspect).** Run→Line→Block from Pass 4; provenance linkage; hit-test +
  caret/selection API; `inspect --text-blocks`. Acceptance: blocks recognized
  and counted on a multi-paragraph/multi-column fixture; no write; core has no
  GUI dep; Pass 4 tests unchanged; fmt/clippy clean. Non-goals: any write,
  reflow, UI. Prereqs: Pass 4; no new dep.
- **13.1 — In-place edit + single-line relayout (core surgery + CLI), the
  font-on-edit gate.** Extend Pass 8.0 REMOVE→REPLACE; inverse-encoding builder;
  advance-preserving line relayout; the F-refuse gate; default incremental save
  with prior-text disclosure; tagged-run MCID preservation + disclosure;
  `pdfce-cli edit-text`. Acceptance: edit an embedded-full and a non-embedded
  run correctly, only the edited stream changed, survivors preserved; a
  subset-missing keystroke refused-and-disclosed; a supplied font lifts a
  non-embedded run and is disclosed `Supplied`; R59 + round-trip green.
  Non-goals: reflow, family-change (13.2), subsetting (FF-C), composite/CJK/RTL,
  add-new-text (FF-D). Prereqs: 13.0; decision 012; spec-librarian (§9.4.x +
  inverse encoding); queue a font-subsetting spec dispatch (permissive-only)
  for FF-C.
- **13.2 — Formatting on a selection (core + CLI).** Size (`Tf`), fill colour
  (`rg`/`g`/`k`), gated family/style change (`Tf` + re-encode). Acceptance:
  size/colour always minimal-diff; family change to an available covering face
  works; unavailable refused-and-disclosed; gates green. Non-goals:
  spacing/scale/rise, synthetic styles (FF-H). Prereqs: 13.1.
- **13.3 — Edit UI on the Pass 12.0 canvas (gui).** CanvasTool: click→caret,
  type→edit, drag→select, live preview accepted by the operator; block-boundary
  review overlay; three-trust-level + tagged disclosures surfaced. Prereqs:
  13.0-13.2 + Pass 12.0; DISPATCH `pdfce-ui-specialist` first.

### 5.3 Fast-follow ladder (named, not scheduled)

FF-A within-block offline reflow (greedy; Acrobat offline parity) · FF-B
cross-block/cross-page offline reflow + justified (the exceed-Acrobat headline)
· FF-C font-subsetting/glyph-embedding writer capability (lifts the subset
refusal; permissive-only, rule 13; spec dispatch) · FF-D add NEW page text ·
FF-E CJK/composite in-place editing (decision 012 FF2 Unicode-route) · FF-F
RTL/bidi editing (R17/`layout.rs` bidi deferral) · FF-G OCR-gated scanned-text
editing (edits the OCR layer, never the raster) · FF-H Tc/Tw/Tz/Ts + synthetic
styles (disclosed-fuzzy) + minimal StructTree/ActualText update.

> **AMENDED 2026-08-01 by decision 015 §6:** "FF-B ... + justified" above is
> superseded — justified is relocated to FF-A (see the §3 footnote above).
> FF-A now ships all four alignment modes (left/center/right/justified); FF-B
> keeps cross-block/cross-page only. Decision 015 also finalizes FF-A's Pass
> slicing as `ROADMAP.md`'s ★ Pass 15.x (15.0 engine / 15.1 surgery + CLI /
> 15.2 GUI).

> **FF-D scheduled 2026-08-01 by decision 016 → Pass 16.x; see 016.**
> `docs/decisions/016-ffd-add-new-page-text.md` prioritizes the remaining
> fast-follow ladder (FF-D ranked #1, solo-startable; FF-C ranked #2 but
> operator-gated; FF-B deferred; FF-H deferred; list-authoring operator-
> gated) and scopes FF-D concretely: a new `BT…ET` text object appended to
> the page `/Contents` array (never a Pass-6.2 FreeText annotation), default
> bundled Standard-14 font (no embedding, no FF-C dependency), routed through
> the same 14.x edit/format + 15.x reflow pipeline. Filed as `ROADMAP.md`'s
> ★ Pass 16.x (16.0 point-text engine+CLI / 16.1 boxed add+wrap / 16.2 canvas
> UI); new standing rules R78–R79. History above is unchanged; this is a
> forward pointer only.

### 5.4 Honest limits (named up front)

Outlined/vectorized text is permanently unreachable (detect + disclose "vector
art") · scanned/raster needs OCR first (FF-G) · embedded-subset fonts can't gain
glyphs until FF-C · composite/CJK (FF-E) and RTL/bidi (FF-F) deferred · reflow
is derived and always reviewable.

### 5.5 Where pdfce exceeds Acrobat

Offline cross-block reflow (FF-B; Acrobat is cloud + English-only) ·
minimal-diff tag preservation (Acrobat corrupts the structure tree) ·
first-class scriptable CLI `edit-text` (Acrobat has none).

---

## 6. What this decision explicitly does NOT decide

- **Font subsetting / glyph embedding** — named FF-C; a separate writer
  subsystem, permissive-only, its own spec dispatch and (given copyleft risk) a
  dependency-licensing escalation before it is built.
- **The exact edit-interaction UX** — `pdfce-ui-specialist`'s call at 13.3
  (caret rendering, selection affordance, block-boundary handles).
- **Knuth-Plass reflow, justified alignment** — FF-A ships greedy left/right/
  center; FF-B adds justified.
  **AMENDED 2026-08-01 by decision 015:** justified is relocated to FF-A
  (ships alongside left/right/center); Knuth-Plass remains deferred at FF-A
  (named non-goal, decision 015 §3.2) — greedy first-fit only. FF-B no longer
  carries an alignment-mode payload; its scope is cross-block/cross-page
  reflow only.
- **Composite/CJK/RTL/vertical in-place editing** — deferred, coupled to
  decision 012 and the bidi deferral.
- **Any change to redaction's R35 guarantee** — untouched; editing is the
  distinct incremental operation.

---

## 7. Revisit triggers

1. **A corpus measurement shows the embedded-subset refusal dominates real
   edits** → schedule FF-C (font subsetting), with the dependency-licensing
   escalation first.
2. **Operators hit line-overflow-without-reflow frequently** → pull FF-A
   (within-block reflow) forward.
3. **Block recognition mis-segments a material share of real layouts (tables/
   forms/columns)** → tune the heuristic against the corpus; the reviewable
   overlay is the safety net meanwhile.
4. **CJK/RTL editing is requested** → FF-E/FF-F, gated on decision 012 FF2 and
   the bidi work.
5. **A tagged-PDF accessibility workflow needs a live structure tree** → FF-H's
   minimal StructTree/ActualText update.

---

## 8. References

- **Code:** `text_extract/layout.rs` (derived Run/Line segmentation, the three
  ratios, S1-S9); `text_extract/{page.rs,font.rs,mod.rs}` (glyph geometry, the
  §9.10.2 decode ladder to invert); `redact.rs` (Pass 8.0 advance-preserving
  surgery + §7.5.7 decomposition); `vartext.rs` (line-breaking for reflow);
  `content.rs` (`ContentStream`/`ContentTokenKind`); decision 012's
  `GlyphSource` + `FontEnvironment.named` + `--font-dir`; the Pass 12.0 canvas
  (`canvas_to_pdf_space`, `CanvasTargetProvider`, selection scaffold).
- **Decisions/rules:** decision 004 (fonts/seam, §3.6 positions-from-Widths,
  R21); decision 012 (operator-supplied fonts, GlyphSource, R62); ARCHITECTURE
  §5 / §5.7; ROADMAP R32/R35/R36/R46/R47/R58/R59/R60; rule 4
  (fuzzy-never-sneaky); rule 13 (no copyleft dep); GUI-core separation.
- **Spec:** ISO 32000-1 §14.8 (S1-S9 derived layout), §9.4.4 (advance),
  §9.10.2 (decode ladder), §12.5.6.23 (redaction — the distinct operation).
- **Parity reference:** `D:\Dev\Rag-Specialized\Acrobat_Features\text_edit__*.md`.

## Appendix A — JSON decision block

```json
{
  "decision_id": "014-acrobat-text-editing",
  "title": "Acrobat-style in-place text editing: editable text model, advance-preserving content-stream surgery, glyph-availability font-on-edit posture, offline reflow ladder",
  "status": "Decided",
  "requested_by": "pdfce-engineer",
  "decided_by": "KenAgent (autonomous-builder)",
  "date": "2026-07-31",
  "confidence": "high (model / surgery / font-posture / slicing); medium (block-recognition heuristic tuning and reflow-alignment detection — inherently fuzzy, corpus-iterated)",
  "one_line": "Build an editable Run->Line->Block text model on top of Pass 4's derived-layout extraction (all DERIVED, reviewable); edit page text via advance-preserving content-stream SURGERY (kin to Pass 8.0 redaction, NOT the Pass-6.x overlay path); gate every keystroke on GLYPH AVAILABILITY (edit with glyphs the run's font can already provide; REFUSE-and-disclose a glyph an embedded SUBSET lacks; defer font-subsetting as the named writer fast-follow that lifts the refusal); ship real editing in a first cut of single-line relayout + basic formatting WITHOUT a font embedder; ladder reflow as the exceed-Acrobat opportunity (offline cross-block, Acrobat is cloud-only + English-only).",
  "note": "The full structured JSON block as produced by KenAgent is preserved in the session transcript and the SESSION_LOG entry for 2026-07-31; the prose sections 1-8 above are the canonical, implementation-driving record. Pass slicing: 13.0 read-only model+recognition; 13.1 in-place edit+single-line relayout+font-on-edit gate+CLI edit-text; 13.2 formatting-on-selection; 13.3 edit UI on the Pass 12.0 canvas. Fast-follows FF-A..FF-H. Six proposed standing rules (see 5.1) pending librarian Rnn reconciliation."
}
```

## Appendix B — Engineer handoff notes (not part of the archived record)

- **Start at 13.0's provenance linkage** — it is the quiet prerequisite for
  everything. Confirm what Pass 4 already carries per run (geometry yes; source
  operator identity + byte span + full text-state — verify) and extend the read
  path minimally without perturbing extraction fixtures.
- **The surgery reuse is REMOVE→REPLACE**, not a new interpreter — `redact.rs`
  already does the §9.4.4 advance bookkeeping; the new piece is the
  inverse-encoding builder (Unicode→code) and the "shift the rest of the line by
  the advance delta" positioning fixup.
- **The font-on-edit gate is the whole game** — implement it as a pre-flight
  check keyed on `GlyphSource` + glyph presence, returning a named refusal the
  UI/CLI surfaces verbatim. Do NOT let a missing glyph reach the writer.
- **Dispatch order:** `pdfce-acrobat-librarian` catalog is in hand;
  `pdfce-spec-librarian` for 13.1 (§9.4.x + inverse encoding) and a QUEUED
  font-subsetting dispatch for FF-C; `pdfce-ui-specialist` before 13.3; then
  `pdfce-librarian` to file this decision, the six rules (reconciling the R61
  collision), the Pass 13.x family, and a SESSION_LOG entry.
- Nothing here is blocked on the license decision. FF-C is the one place a
  dependency-licensing escalation (rule 13) is mandatory before code.
