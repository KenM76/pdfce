# Decision 010 — The highest-value next major investment after the editing arc, and whether the accumulated debt changed decision 008's priority

- **Date:** 2026-07-31
- **Status:** Decided
- **Decider:** KenAgent (autonomous-builder / decision-consultant), per the
  ROADMAP standing rule "KenAgent decision routing (operator process rule,
  2026-07-30)".
- **Question:** With the read → write → edit → extract → annotations →
  forms → redaction arc complete (Passes 0,1,1.1-partial,2.x,3.x,4,6.x,7.x,8.0
  all shipped), what is the highest-value next major investment — and has
  the accumulated GUI-editing debt and render-fidelity verification debt
  (which decision 008 did not fully weigh) changed the priority it set?
- **Outcome:** The **destination is unchanged** — Vector/Inkscape-parity
  content editing (candidate **A**) remains the highest-value major
  investment and the project's distinctive stated purpose. The debt changes
  the **path**: two owed prerequisites are sequenced ahead of A —
  **C (render-fidelity verification)** first, then **B (a shared
  canvas-interaction foundation + consolidation of the three deferred
  editing GUIs)** — because vector editing is the first subsystem whose
  correctness oracle is independent *visual* fidelity, and because A's UI
  is literally impossible without the canvas foundation. **Sequence:
  C → B → A**, with D (encryption) the unchanged fallback track and
  E (signatures) unchanged-last.
- **Amends:** decision 008's revisit-trigger 7 ("operator wants vector
  editing sooner → Pass 9 promotes directly after Pass 6.1"). Decision
  008's six-candidate ranking and its Pass IDs are otherwise intact.
- **Adds standing rules:** a render-fidelity verification gate; a
  one-canvas-substrate rule; an Inkscape-reference rule (librarian assigns
  R-numbers, expected R59+).

> **Candidate-letter warning.** This record's letters **A–E are local to
> decision 010 and differ from decision 008's A–F.** Here: **A** =
> Vector/Inkscape-parity editing (008's candidate E / Pass 9); **B** =
> GUI-editing consolidation (the deferred canvas foundation + markup/
> form-fill/redaction-mark GUIs); **C** = render-fidelity verification
> (the owed Pass 1.1 pixel-parity remainder); **D** = Encryption (Pass 5);
> **E** = Signatures/PAdES (Pass 10).

---

## 1. Where pdfce actually is

The arc decision 007 and decision 008 laid out is complete, and it
delivered a genuinely deep engine:

- A from-scratch Rust PDF engine: 99.2% conformance-corpus load+render,
  all image codecs, full text including Identity-H.
- Identity + mutation + structural writers with **measured byte-identity
  round-trip gates** (R34 Pass 3.0 identity; R46 content-stream identity,
  12,854/12,936 streams byte-identical, 0 corrupted).
- Text extraction at 99.78% sourced.
- The full annotation arc: appearance display (6.0), geometric markup
  authoring (6.1), text-bearing annotations + variable text (6.2).
- The AcroForm core: field model + fill + appearance regeneration +
  flatten + FDF/XFDF (7.0/7.1), with decision 009's never-execute-JS
  posture A honored.
- Redaction with a passing absence-proof (8.0).
- ~1018 tests, 15 fuzz targets, a GUI viewer + a rich CLI.

Two foundations that matter for what comes next now exist: the
**content-stream serializer** (6.1) and an **advance-preserving
content-stream surgery interpreter** (8.0). Vector editing's hardest
foundations are in place. This is exactly what decision 008 §5.3
predicted ("E's load-bearing foundation lands second").

But pdfce today is a **strong engine + CLI + a viewer**. The thing a
human actually uses for "Acrobat + Inkscape parity" — an interactive
editing GUI — is thin. And the render stack's absolute fidelity is
**unmeasured at scale**. These are the two pieces of new information
decision 008 did not fully weigh.

---

## 2. The two debts decision 008 did not weigh

### 2.1 GUI-editing debt

Every editing subsystem shipped its **engine + CLI + a minimal menu
affordance**, deferring the real interactive canvas to a named follow-up
slice. Three UI specs already exist and were never built:

- `docs/ui_specs/pass-6.1-markup-tools.md` — the tool-mode drawing state
  machine (drag/marquee/multi-click/ink), live preview, property bar,
  keyboard map. **Not shipped.**
- `docs/ui_specs/pass-7-form-fill.md` — interactive form-fill GUI.
- `docs/ui_specs/pass-8-redaction.md` — redaction-marking canvas.

The decisive detail is in the 6.1 spec's own P0 table: **"Canvas made
focusable/interactive — resolves the long-standing `main.rs` caveat —
Prerequisite for everything else in this spec."** The canvas is not even
focusable yet; `main.rs`'s Pass-1 caveat ("revisit when the canvas gains
focusable content") is still open. **The drawing infrastructure never
landed** — each later Pass re-deferred it and re-flagged the dependency.

And all three specs independently describe the **same** primitives: a
focusable `Response` with `Sense::click_and_drag`; `screen_to_page` /
`page_to_screen` transforms storing geometry in page-space; a tool-mode
dispatch state machine; live-preview overlay painting; hit-testing +
selection. That is one substrate, specced three times, built zero times.

### 2.2 Render-fidelity verification debt

Full-page pixel-parity against an independent reference renderer
(pdfium) has been **owed since Pass 1.1 and never built**. Render
correctness is proven only by:

- spot checks (9 CMYK files in decision 006; the annotation subset,
  7/7 placement fixtures within 4px, in Pass 6.0), and
- the self-comparison round-trip oracle.

The self-comparison oracle proves **pdfce agrees with itself** — not
that it matches pdfium. Pass 6.0's own ship notes were explicit: "This
is NOT a claim that the Pass 1.1 pixel-parity remainder is CLOSED … full-
page pixel parity over real corpus pages remains OWED." **The whole
render stack's fidelity is unmeasured at scale.**

### 2.3 The oldest operator item (context, not a candidate)

Encryption is still refused-at-load (0.67% census); the operator sign-off
on that refusal is the oldest open item, now carried across decisions
007/008/009. It is an **operator action**, not an engineering investment,
and does not compete here — but it must be surfaced again.

---

## 3. The candidates, and the one question that separates them

- **A** — Vector/Inkscape-parity content editing (008's Pass 9). The
  distinctive stated purpose. Largest subsystem; needs slicing; needs the
  **unowned** Inkscape capability catalog assigned; its foundations
  (content serializer, surgery interpreter, selection/hit-test prototyped
  in annotations) now largely exist.
- **B** — GUI-editing consolidation. Build the deferred interactive
  canvas: the tool-mode drawing infrastructure, markup drawing, form-fill,
  redaction marking. Makes the engines usable by a human. Shares the
  canvas-interaction foundation with A's UI.
- **C** — Render-fidelity verification. Build the general pdfium
  pixel-parity harness (the owed Pass 1.1 remainder), measure the whole
  render stack at corpus scale, fix what it surfaces. Comparatively small
  and bounded; de-risks everything downstream.
- **D** — Encryption (Pass 5). Decrypt handlers + AES encrypt-on-save.
  0.67% census; /R6 sub-decision open.
- **E** — Signatures/PAdES (Pass 10). Depends on D; heaviest sourcing;
  0.64% census; read-half far along.

Decision 008's separating question was "what is each a precondition
for?" That is less discriminating now. The question that separates these
is: **which of these is a precondition the *destination* (A) newly
requires — and did not require until now?**

Two of them are, and that is the whole decision:

- **A's correctness oracle is independent visual fidelity → C.** Vector
  editing modifies existing page content streams and then re-renders the
  edited page. Its acceptance test is "does the page still render
  correctly?" — which a self-comparison oracle *structurally cannot*
  answer (pdfce agreeing with pdfce after an edit proves nothing about
  whether the edit is visually correct). Redaction (8.0) dodged this
  entirely: its oracle is byte-*absence* ("is the content gone from the
  bytes"), not visual correctness. **A is the first subsystem in the
  entire project whose oracle is independent visual fidelity, and C is
  the only thing that provides it.**
- **A's UI cannot exist without B's substrate.** Clicking a path,
  marquee-selecting objects, dragging a Bézier node, numeric-transforming
  a selection — these are the *same* canvas-interaction problem the three
  deferred UI specs describe, on a canvas that is not yet even focusable.

So the debt does not introduce new destinations. It reveals that the
destination everyone already agreed on (A) has two unpaid prerequisites.

---

## 4. The decision: sequence C → B → A

### 4.1 Destination unchanged, path amended

**A remains the highest-value major investment.** It is Ken's kickoff
purpose ("all the capabilities inkscape and acrobat pro" have); decision
008 already ranked vector "not deferred to last — its foundation lands
second"; and its foundations now exist. Nothing about the debt demotes A.

What the debt changes is decision 008's **revisit-trigger 7** — the
imagined "promote Pass 9 directly after Pass 6.1" clean jump onto the
content-stream serializer. That jump would land on (1) a render stack
whose fidelity is unmeasured — so a wrong render after a vector edit
could not be attributed to the edit vs a pre-existing render bug — and
(2) a canvas with no interaction foundation at all. The amendment inserts
**C then B** ahead of A, converting the clean jump into "prove the
render, build the canvas, then build vector editing on solid ground."

This is not a new idea. It is the project's **own proven inversion**,
applied one level up:

- Pass 3.0 shipped a writer that could not edit and spent the whole Pass
  proving byte-identity — *before* any editing Pass.
- Pass 6.0 shipped annotation *display* (read-side) — *before* 6.1
  authoring.
- Decision 009 made posture A (recognize + disclose) the floor —
  *before* posture B (recompute).

Every time, the most valuable capability shipped *after* the thing that
proved the ground under it. C and B are that thing, for A.

### 4.2 Why C is the first Pass, not B

C leads for five reasons, and the counter-case is stated honestly:

1. **It is the inversion pattern one more time.** A pure measurement Pass
   with a corpus-wide executable oracle available *today* (pdfium /
   pypdfium2; precedent: decision 006 §3.2, `tools/annot-pdfium-diff.py`).
   Same profile that justified Pass 3.0 and Pass 6.0 as standalone
   read-side Passes: no capability that can produce a wrong file,
   de-risks *more than one* downstream subsystem (both A and B), closes a
   live owed item.
2. **It is bounded and small** relative to A and B — the harness
   precedent exists; it only needs generalizing from the 7-fixture
   annotation subset to full-page corpus scale.
3. **It de-risks both B and A.** An editing GUI (B) whose live visual
   feedback runs on an unverified render is itself unverified; a
   content-stream edit (A) cannot be validated without an independent
   visual reference.
4. **It has zero dependency on B** (different crates: render/tooling vs
   GUI), so nothing is lost by doing it first.
5. **Its result can reprioritize everything.** If C surfaces a
   systematic, structural render divergence pdfce cannot cheaply
   reconcile, that is a finding that must be settled *before* A — which is
   exactly why C runs first. Discovering it now, with zero editing code
   depending on it, is the cheapest possible time.

*The counter-case, honestly:* B-first would deliver visible operator
value *now* from every engine already built (pdfce is engine-heavy /
GUI-light) and would unblock A's UI. It is rejected as *first* because B
is larger, produces no measurement, and building the editing GUI on an
unverified render means the operator's own visual feedback during editing
is unverified. C is cheap; doing it first makes B's feedback and A's
edit-verification trustworthy at negligible schedule cost.

### 4.3 The shared canvas foundation — built once, in B

Because A's vector-editing UI is the *same* interaction problem as the
three deferred editing GUIs, B's **first slice is the one substrate they
all share**: a focusable canvas, `screen_to_page`/`page_to_screen`,
tool-mode dispatch, hit-testing + selection, live-preview overlay. Then
the three deferred GUIs (markup drawing, form-fill, redaction marking)
land on it as subsequent slices, each already fully specced. A's UI later
inherits the same substrate. Building it once — rather than under three
separate deferred slices — is the **R49 one-pipeline discipline applied
to interaction** (a new standing rule below). B is where pdfce stops
being "a strong engine + CLI + viewer" and becomes an actual editor.

---

## 5. First Pass — full scope (Pass 11: render-fidelity verification)

*(Proposed ID Pass 11; librarian confirms. This discharges the long-owed
"full-page pixel-parity remainder (Pass 1.1)" carried in every recent
SESSION_LOG "still open" list — now scoped as its own major Pass rather
than a sub-slice.)*

**Purpose:** prove pdfce's render stack against an independent reference
renderer at corpus scale *before* any subsystem edits page content it
must then re-render — replacing "pdfce agrees with pdfce" with a measured,
bucketed, by-file/by-reason fidelity report.

### Deliverables

1. Generalize `tools/annot-pdfium-diff.py` from an ink-bbox differential
   on 7 fixtures into a **full-page pdfium (pypdfium2) pixel-parity
   harness** over the whole loadable corpus (`fixtures/external`, ~3,020
   files): render each page at fixed DPI in both engines, align rasters,
   compute per-channel per-pixel deltas. Out-of-tree, like the other
   corpus harnesses.
2. A **documented, justified tolerance band** separating benign
   independent-renderer noise (anti-aliasing, hinting, sub-pixel
   positioning, interpolation — two independent renderers *always* differ
   here; it is not a bug) from real divergence. Report the
   **distribution** (per-page mean / 95th-pct / max channel delta,
   fraction-of-pixels-over-threshold), never a bare pass/fail.
3. A corpus-scale run classifying every page's divergence into three
   buckets, enumerated **by file and by reason** (R20 discipline):
   (i) benign-renderer-noise; (ii) **known-disclosed-gap** — already-counted
   unsupported behavior (Type3 fonts, `sh` shading, `/SMask`, `/OC`,
   DeviceCMYK colorimetry), cross-referenced against the *existing*
   Diagnostics unsupported-tally so they are subtracted, not re-reported;
   (iii) **unexplained-divergence** — the genuine bug candidates, i.e. the
   residual after subtracting (i) and (ii).
4. Triage + fix of the in-scope subset of bucket (iii): fix what is cheap
   and clearly a pdfce render bug; file the rest as **counted, named
   render-gap** items (R20/R27 posture).
5. Encode the **known reference-divergence** cases so pdfium's own quirks
   are never misattributed to pdfce — specifically the Pass 6.0 finding
   that pdfium needs `FPDF_FFLDraw` for `/Widget` appearances and
   *synthesizes* some no-`/AP` appearances (e.g. `/Circle /IC` fill) that
   R43 makes pdfce correctly refuse. These are reference-side divergences.
6. **Wire the harness into the standing gate set** (the R34/R46 pattern)
   so every future render-touching Pass — especially A — re-runs it. This
   is the durable payoff: A's edits inherit a standing full-page fidelity
   gate.
7. **DeviceCMYK→RGB colorimetry** (already filed: naive additive
   `Rgb::from_cmyk` vs pdfium's `AdobeCMYK_to_sRGB1`; 37.4% of pixels >8
   delta on the corpus CMYK JPEG; affects *all* DeviceCMYK fills/strokes)
   is the first systematic divergence the harness will light up. Character-
   ize + quantify it corpus-wide in-Pass; fix here *only if bounded*
   (scoped via `pdfce-acrobat-librarian`'s already-filed "what does Acrobat
   do for uncalibrated DeviceCMYK→screen" question), else file as the
   harness's first named residual. **Do not confound the colour fix with
   the harness build** (decision 006's discipline; re-pin decision 006
   §3.4's polarity matrix before any colour change lands, 006
   revisit-trigger 7).

### Acceptance criteria

- Harness runs over the full loadable corpus with **zero panics/timeouts**,
  deterministic + locale-invariant per-file/per-page report.
- Every divergence classified into benign / known-gap / unexplained, the
  **tolerance band documented and justified — never tuned to make a number
  green** (W14: a systematic divergence is a finding to run down, not to
  average away).
- The "unexplained" bucket is genuinely the residual after subtracting
  benign noise and already-counted gaps — cross-checked against the
  Diagnostics unsupported-tally (no double-counting).
- Every "unexplained" item **fixed or filed as a named counted render-gap**
  (R20/R27), by file and reason.
- Known pdfium reference-divergences encoded and bucketed reference-side,
  never flagged as pdfce errors.
- Harness added to the standing gate set + documented as a **required
  re-run for every render-touching Pass** — the criterion that makes it
  de-risk A durably.
- Standing gates green (`cargo fmt --check`, `clippy -D warnings`,
  GUI-free `cargo tree` core+render on host + `x86_64-pc-windows-msvc`,
  wasm32, `--duplicates`, `no-network`; `ui-strings` N/A). Prior gates
  (R34, R46) **unmoved**.
- **No new runtime dependency** in pdfce. pypdfium2 is dev/tooling only,
  out-of-tree; it does not enter pdfce's shipped set or
  `THIRD_PARTY_LICENSES.md` (LEGAL §6 note if newly vendored).

### Explicit non-goals (binding)

- **No new render capability.** Type3/`sh`/`/SMask`/`/OC`/DeviceCMYK stay
  their own filed items; this Pass *measures* and buckets them (beyond the
  cheap, clearly-a-bug fixes it surfaces). Closing render gaps here would
  double the Pass and confound measurement with feature work.
- **No editing capability of any kind.**
- **Not chasing benign anti-aliasing / sub-pixel noise to zero** —
  characterizing it, not eliminating it. Demanding two independent
  renderers agree pixel-for-pixel is a category error.
- **No GUI visual-diff surface** — tooling-only corpus harness + gate.
  (A GUI "compare-against-reference" view is a natural later B addition.)
- **Not a "pixel-perfect" claim.** The deliverable is a measured, bucketed
  report with the residual named (R20/R27). Do **not** report the Pass 1.1
  remainder "closed" unless the harness genuinely generalizes to full-page
  corpus scale — the exact caveat Pass 6.0 already flagged.

### Spec / RAG prerequisites

- Minimal — measurement, not spec-governed behavior; no blocking
  spec-librarian dispatch.
- If the DeviceCMYK fix is taken in-Pass: `pdfce-acrobat-librarian` for
  the already-filed uncalibrated-DeviceCMYK question (non-blocking).
- pypdfium2 tooling already precedented (decision 006 §3.2).

### Risks (full list in the JSON block; the ones that bite)

- **Y1 — noise swamps signal.** The central risk: two independent
  renderers always differ. Wrong band ⇒ real bugs hide, or benign noise is
  chased forever. Mitigation: define the band empirically + document the
  justification; report distributions; subtract known-gaps first; the two
  forbidden failure modes are tuning-until-green (W14) and declaring noise
  a bug. Separating noise from bugs *is* the analytical core of this Pass.
- **Y2 — pdfium quirks misattributed to pdfce** (FPDF_FFLDraw /
  synthesized appearances). Mitigation: encode + bucket reference-side.
- **Y3 — scope creep into fixing every gap.** Mitigation: the non-goal
  binds; fix cheap/clear bugs, file the rest.
- **Y4 — a systematic divergence pdfce can't cheaply fix.** It is a
  *finding*, not a failure — precisely why C runs before A; file it, it
  may reprioritize (revisit trigger).
- **Y5 — DeviceCMYK colour fix confounds the harness build.** Mitigation:
  characterize as a bucket first; fix separately; re-pin decision 006's
  polarity matrix first.

---

## 6. Second and third Passes (summary; scoped in full when reached)

### Pass 12 — Canvas-interaction foundation + editing-GUI consolidation (B)

- **First slice, build once:** the shared substrate — focusable canvas
  (`Sense::click_and_drag`, resolving `main.rs`'s Pass-1 caveat);
  `screen_to_page`/`page_to_screen` (page-space geometry, rotation-correct
  via `current_extent`); tool-mode dispatch; hit-test + selection;
  live-preview overlay; drag-vs-pan suppression. This is
  `pass-6.1-markup-tools.md` §1–§2 verbatim, built generically for markup,
  form-fill, redaction *and* vector editing.
- **Subsequent slices:** land the three deferred, already-specced editing
  GUIs on the foundation — markup drawing (`pass-6.1-markup-tools.md`),
  form-fill (`pass-7-form-fill.md`), redaction marking
  (`pass-8-redaction.md`, the R52 mark-then-apply canvas).
- **Why before A:** A's vector-editing UI is impossible without this
  substrate; consolidating the three GUIs onto one foundation is the R49
  one-pipeline discipline applied to interaction. Dispatch
  `pdfce-ui-specialist` (it owns the five-way placement taxonomy the specs
  reference).

### Pass 9 — Vector / Inkscape parity, sliced (A)

- **Promoted onto:** a trustworthy render (C) + an existing canvas
  foundation (B) + Pass 6.1's serializer + Pass 8.0's surgery interpreter.
  Decision 008 §5.3's "foundation lands second" is now fully realized.
- **Slicing** (008 §5.3 / the ROADMAP bucket): (a) object model +
  hit-test + selection [reuses B's selection directly]; (b) transforms /
  z-order / group-ungroup; (c) node/Bézier editing; (d) boolean path ops;
  (e) gradients + shading + transparency (007 fold-in); (f) OCG layers;
  (g) text-to-path.

#### Inkscape capability catalog — owner ASSIGNED

Decision 008 §11.4 flagged this catalog **unowned** and outside the
acrobat-librarian's remit. Since A is the destination, this record
assigns it:

- **Commission a new sibling librarian — `pdfce-inkscape-librarian`** —
  modeled on `pdfce-acrobat-librarian`, owning an LLM-optimized Inkscape
  capability-parity RAG (proposed `D:\Dev\Rag-Specialized\Inkscape_Features\`,
  mirroring `Acrobat_Features\`).
- **Remit:** capability / behavior / edge-cases / limits **only** — never
  Inkscape's GUI mechanics (menu paths, dialogs, trade dress). pdfce's
  vector-editing UI is designed independently by `pdfce-ui-specialist`.
  Same capability-not-GUI discipline (rule 12) and LLM-optimized grep-first
  format (rule 14) as the acrobat RAG.
- **Binding license note:** Inkscape is GPL-2.0-or-later — behavioral /
  capability reference **only**, never a dependency, code source, or
  GUI-mechanics mimicry (loud; same rule as MuPDF/Ghostscript in
  PRIOR_ART.md; gates against pdfce's undecided license, rules 8/13).
- **Why a new agent, not extending the acrobat-librarian:** the
  acrobat-librarian's identity is "Adobe Acrobat Pro feature parity."
  Folding a second reference product into it muddies both RAGs and both
  agents' single-concern identity. A clean sibling matches the
  one-agent-per-concern roster.
- **When:** commission **now, during Pass 11**, as a cheap non-blocking
  parallel task (exactly how the "Comments & markup" acrobat bucket was
  dispatched ahead of 6.1). It must exist before A is sliced into real
  Passes.

---

## 7. Unchanged tracks

- **D — Encryption (Pass 5):** keeps its decision-007 ID and its
  decision-008 role as the independent **fallback/interleave track** — the
  switch target if any of Pass 11/12/9 hits the three-attempts wall.
  Census payoff unchanged (0.67%, promotion trigger not met). §7.6
  spec-corpus now complete, so no longer spec-blocked — but the **/R6
  AES-256 sourcing gap** remains a Pass-5 open sub-decision (only /R5
  buildable today). Real promotion trigger: an actual encrypted file that
  blocks operator work.
- **E — Signatures/PAdES (Pass 10):** unchanged-last. Depends on D;
  heaviest sourcing; 0.64% census; read-half far along (`signature.rs` 810
  lines, §12.8 sourced). Signing is the missing half; the incremental-
  update signing model is already the default save mode.

---

## 8. Standing rules added (librarian assigns numbers, expected R59+)

- **Render-fidelity verification gate** — pdfce's render stack is proven
  against an *independent* reference renderer at corpus scale before any
  subsystem edits page content it must then re-render. The self-comparison
  round-trip oracle proves pdfce agrees with *itself* (sufficient for
  additive authoring, where page content stays byte-verbatim) but not that
  pdfce matches an independent renderer (required once content-stream
  *surgery* re-renders edited pages). The full-page pixel-parity harness
  re-runs on every render-touching Pass (R34/R46 pattern); its residual is
  enumerated by file and reason — never a bare pass/fail, never a threshold
  tuned to pass (W14).
- **One canvas-interaction substrate** — exactly one focusable canvas +
  screen↔page transform + tool-mode dispatch + hit-test/selection +
  live-preview overlay. Markup drawing, form-fill, redaction marking, and
  vector-object editing all layer onto it; a second parallel
  canvas-interaction path is forbidden (R49's one-pipeline discipline
  applied to interaction).
- **Inkscape is a behavioral/capability reference only** — GPL-2.0-or-later,
  never a dependency, code source, or GUI-mechanics mimicry.
  `pdfce-inkscape-librarian` catalogs capability/behavior/limits;
  `pdfce-ui-specialist` designs the UI independently. (Formalizes the
  existing binding ROADMAP note.)

---

## 9. Revisit triggers

1. **Pass 11 (C) surfaces a systematic, structural render divergence
   pdfce cannot cheaply reconcile** → stop and re-decide before Pass 12/9.
   Vector editing on a render you can't trust is the exact "building on
   sand" this sequence prevents; a large render-correctness finding may
   itself become the next major investment.
2. **Operator wants the usable editor before render verification** → B can
   promote ahead of C, accepting that B's live visual feedback runs on an
   unverified render until C lands. State the tradeoff; don't reorder
   silently.
3. **Operator wants vector editing immediately, accepting unverified-render
   risk** → A can promote after B alone (B is the hard UI prerequisite; C
   is the correctness prerequisite). Brief that this ships vector edits
   whose visual correctness is spot-checked, not corpus-measured — a real,
   disclosed risk (fuzzy-never-sneaky applied to scheduling).
4. **Any of Pass 11/12/9 hits the three-attempts wall** → switch to Pass 5
   (encryption), the designated independent fallback track.
5. **A real encrypted file blocks actual work** → Pass 5 promotes.
6. **DeviceCMYK colorimetry dominates the Pass 11 "unexplained" bucket** →
   it graduates from a residual to its own scoped colour Pass (via
   `pdfce-acrobat-librarian`).

---

## 10. Operator actions owed (surfaced, not new)

- **Encryption-refusal sign-off** (`XrefErrorKind::EncryptionUnsupported`)
  — the oldest owed item, now carried across decisions 007/008/009/010.
  pdfce declines a category of file every other reader opens; the operator
  has still never been told.
- **License decision** (LEGAL §1) — gates the public repo, any release,
  and the boundary around the GPL Inkscape reference.
- **Commit authorization** — Passes 0–8 are ALL uncommitted (~20,000+
  lines unversioned, no remote, no CI history; W15/X16).
- **/R6 AES-256 sourcing method + LEGAL §2 Adobe-supplement copyright
  contradiction** — Ken's calls when Pass 5 activates
  (SESSION_LOG continuation-22).

## References

- `docs/decisions/008-next-subsystem-after-extract.md` — the sequence this
  amends (ranking intact; revisit-trigger 7 amended); the F1–F4 findings;
  the unowned-Inkscape-catalog item (§11.4) this closes.
- `docs/decisions/009-forms-javascript-posture.md` — the posture-A-before-B
  precedent this sequence mirrors.
- `docs/decisions/006-cmyk-jpeg-inversion.md` — the pypdfium2 differential
  precedent and the DeviceCMYK colorimetry gap.
- `docs/ui_specs/pass-6.1-markup-tools.md`, `pass-7-form-fill.md`,
  `pass-8-redaction.md` — the three deferred GUIs and the shared
  canvas-interaction substrate B builds once.
- ROADMAP standing rules R34 / R46 (the re-run-on-every-touching-Pass
  gate pattern), R49 (one appearance pipeline — extended here to one
  interaction substrate), R20/R27 (counted, named shortfalls), W14 (don't
  weaken a gate to pass), W16 (an uncertain denominator can't report an
  honest shortfall).

## Appendix A — JSON decision block

```json
{
  "decision_id": "010",
  "title": "The highest-value next major investment after the read→write→edit→extract→annotations→forms→redaction arc — and whether the accumulated GUI-editing + render-verification debt changes the priority decision 008 set",
  "date": "2026-07-31",
  "status": "Decided",
  "decider": "KenAgent (autonomous-builder / decision-consultant), per the ROADMAP standing rule 'KenAgent decision routing (operator process rule, 2026-07-30)'",
  "supersedes": "nothing",
  "amends": "decision 008's sequence — specifically its revisit-trigger 7 ('operator wants Inkscape-style vector editing sooner → Pass 9 promotes directly after Pass 6.1'). That clean jump is amended, NOT cancelled: two owed prerequisite investments (render-fidelity verification, then a shared canvas-interaction foundation) are inserted ahead of vector editing. Decision 008's ranking of the six ORIGINAL candidates and its Pass IDs are otherwise intact.",
  "candidate_letters_note": "This record's candidate letters A–E are LOCAL to decision 010 and DIFFER from decision 008's A–F. Decision 010: A = Vector/Inkscape-parity content editing (decision 008's candidate E / Pass 9); B = GUI-editing consolidation (the deferred canvas-interaction foundation + markup-draw/form-fill/redaction-mark GUIs); C = render-fidelity verification (the owed Pass 1.1 pixel-parity remainder); D = Encryption (decision 008's Pass 5); E = Digital signatures/PAdES (decision 008's Pass 10).",

  "headline": "The DESTINATION is unchanged: A (Vector/Inkscape-parity content editing) remains the highest-value MAJOR investment and the project's distinctive stated purpose (Ken's kickoff: 'all the capabilities inkscape and acrobat pro' have). The accumulated debt does NOT demote A. What the debt changes is the PATH to A. Decision 008 imagined promoting vector editing directly onto Pass 6.1's content-stream serializer. The new information reveals that jump would land on (1) a render stack whose absolute fidelity is UNMEASURED at scale — so a wrong render after a vector edit could not be attributed to the edit vs a pre-existing render bug — and (2) a canvas with NO interaction foundation at all (it is not even focusable; main.rs's Pass-1 caveat is still open). Vector editing is the FIRST subsystem whose correctness oracle is independent VISUAL fidelity (redaction dodged this — its oracle is byte-absence). Therefore two owed prerequisites are sequenced ahead of A: C (render-fidelity verification) first, then B (the shared canvas-interaction foundation + consolidation of the three deferred editing GUIs). This is the project's own proven inversion — prove/measure the read side before building the authoring side (Pass 3.0 before edits, Pass 6.0 before 6.1, decision 009 posture A before B) — applied one level up, at the whole-product scale.",

  "decision": "Sequence C → B → A, with D as the fallback/interleave track (unchanged from 008) and E last (unchanged). Concretely: (1) FIRST PASS = C, a render-fidelity verification harness (full-page pdfium pixel-parity over the loadable corpus) — the long-owed Pass 1.1 remainder, finally scoped as a standalone measurement Pass because vector editing is the forcing consumer that newly requires it. (2) SECOND = B, build the ONE canvas-interaction substrate the pass-6.1/7/8 UI specs each independently assumed (focusable canvas, screen↔page transform, tool-mode dispatch, hit-test/selection), then land the three deferred editing GUIs (markup drawing, form-fill, redaction marking) on it — turning the rich engines into a usable editor and building A's UI substrate ONCE. (3) THIRD = A, Vector/Inkscape-parity, sliced (a)–(g) per decision 008 §5.3, promoted onto a now-trustworthy render and an existing canvas foundation. Commission the UNOWNED Inkscape capability catalog NOW (during C) as a cheap parallel task so it is ready when A is scoped.",

  "ranking": {
    "as_destination": "A (Vector/Inkscape) > B (make it usable) > C (prove it) — A is the stated purpose and highest-value goal.",
    "as_build_order": "C (render-fidelity verification) → B (canvas-interaction foundation + editing-GUI consolidation) → A (Vector/Inkscape, sliced) ; D (encryption) interleaves as the fallback track ; E (signatures) last.",
    "why_order_inverts_value": "The destination and the build order intentionally invert, and this is the whole decision. It is the identical inversion decision 008 §3.2 made: the most valuable thing (authoring) ships AFTER the thing that proves the ground under it (display/measurement). C and B are not detours from A — C is the correctness oracle A newly requires, and B is the interaction substrate A's UI is literally impossible without."
  },

  "does_the_debt_change_decision_008": {
    "destination": "NO. A/vector remains the highest-value major investment and the distinctive purpose. Decision 008 already ranked vector 'not deferred to last — its foundation lands second' and its revisit-trigger 7 anticipated promoting it. Nothing about the debt weakens that.",
    "path": "YES. The debt inserts C then B ahead of A, and it PROMOTES C from an owed 'Pass 1.1 remainder' footnote to the immediate next Pass. Reason: A is the first subsystem whose acceptance oracle is independent visual fidelity, and C is the only thing that provides it. Decision 008 could not weigh this because it framed the render self-comparison oracle as sufficient — it is sufficient for ADDITIVE authoring (annotations/forms overlay-append, page content stays byte-verbatim, self-comparison holds) but NOT for content-stream SURGERY that re-renders edited pages (vector editing), where 'pdfce agrees with pdfce' proves nothing about correctness.",
    "one_line": "Decision 008's destination stands; its revisit-trigger-7 'clean jump onto the 6.1 serializer' is amended into 'prove the render (C), build the canvas (B), then build vector editing on solid ground (A).'"
  },

  "shared_canvas_interaction_foundation": {
    "finding": "Three UI specs — pass-6.1-markup-tools.md, pass-7-form-fill.md, pass-8-redaction.md — each independently specify the SAME canvas-interaction primitives (a focusable canvas Response with Sense::click_and_drag; screen_to_page/page_to_screen transforms storing geometry in page-space; a tool-mode state machine dispatching pointer events; live-preview overlay painting; and, for form-fill/redaction/vector, hit-testing + selection). None of it shipped — every editing Pass landed engine + CLI + a MINIMAL menu affordance and deferred the real canvas to a named follow-up slice. pass-6.1-markup-tools.md §2.2 and its P0 table are explicit: 'Canvas made focusable/interactive — resolves the long-standing main.rs caveat — Prerequisite for everything else in this spec.'",
    "candidates_that_share_it": "B builds it. A's vector-editing UI (click a path, marquee-select objects, drag a Bézier node, numeric-transform a selection) is the SAME interaction problem and reuses it verbatim. The deferred markup-draw / form-fill / redaction-mark GUIs (all in B) reuse it. C's harness is tooling (no GUI) but a later GUI 'compare-against-reference' visual-diff view would reuse the same viewer. Build the substrate ONCE, in B's first slice — a second parallel canvas-interaction path is forbidden (the R49 one-pipeline discipline applied to interaction; proposed as a new standing rule).",
    "consequence_for_sequencing": "This is the strongest argument that B precedes A rather than being folded into it: A's UI cannot exist without B's substrate, and building that substrate under three separate deferred slices (markup, form-fill, redaction) is exactly the fragmentation R49 forbids. B consolidates them onto one foundation, and A inherits it."
  },

  "why_C_is_the_first_pass_not_B": {
    "reasons": [
      "C is the project's inversion pattern one more time, at product scale: a pure MEASUREMENT Pass with a corpus-wide executable oracle available TODAY (pdfium/pypdfium2, precedent: decision 006 §3.2, tools/annot-pdfium-diff.py). Same profile that justified Pass 3.0 and Pass 6.0 as standalone read-side Passes — no capability that can produce a wrong file, de-risks MORE than one downstream subsystem (both A and B), closes a live owed item.",
      "C is bounded and small relative to A and B; the harness precedent exists and only needs generalizing from the 7-fixture annotation subset to full-page corpus scale.",
      "C de-risks BOTH B and A: an editing GUI (B) whose live visual feedback runs on an unverified render is itself unverified; a content-stream edit (A) cannot be validated without an independent visual reference. Building either before C is decision 008's own 'building on sand.'",
      "C has ZERO dependency on B (different crates — render/tooling vs GUI), so nothing is lost by doing it first, and its result can REPRIORITIZE everything: if C surfaces a systematic, structural render divergence pdfce cannot cheaply reconcile, that is a finding that must be settled BEFORE A — which is precisely why C runs first (revisit trigger).",
      "The decisive point: A is the FIRST subsystem in the whole project whose acceptance oracle is independent visual fidelity. Redaction (8.0) dodged this (byte-absence oracle). So C is specifically the prerequisite A — and only A — newly demands, at a level nothing before it did."
    ],
    "counter_case_stated_honestly": "The case for B-first: it delivers visible operator value NOW from every engine already built (pdfce is engine-heavy/GUI-light), and it unblocks A's UI. It is rejected as FIRST because B is larger, produces no measurement, and building the editing GUI on an unverified render means the operator's own visual feedback during editing is unverified. C-first costs little (C is bounded) and makes B's visual feedback and A's edit-verification trustworthy. C then B then A is strictly safer with negligible schedule cost."
  },

  "first_pass_scope": {
    "pass_id": "Pass 11 (proposed; librarian confirms — this discharges the long-owed 'full-page pixel-parity remainder (Pass 1.1)' that appears in every recent SESSION_LOG 'still open' list, now scoped as its own major Pass rather than a sub-slice)",
    "name": "Render-fidelity verification harness (full-page independent-reference pixel parity, corpus-scale)",
    "one_line_purpose": "Prove pdfce's render stack against an independent reference renderer (pdfium) at corpus scale BEFORE any subsystem edits page content it must then re-render — replacing 'pdfce agrees with pdfce' with a measured, bucketed, by-file/by-reason fidelity report.",
    "deliverables": [
      "Generalize tools/annot-pdfium-diff.py from an ink-bbox differential on 7 annotation fixtures into a FULL-PAGE pdfium (pypdfium2) pixel-parity harness over the whole loadable corpus (fixtures/external, ~3,020 files): render each page at a fixed DPI in both pdfce and pdfium, align rasters, compute per-channel per-pixel deltas. Out-of-tree, like the other corpus harnesses (tools/content-identity/, tools/corpus-report).",
      "A DOCUMENTED, JUSTIFIED tolerance band separating benign independent-renderer noise (anti-aliasing, hinting, sub-pixel positioning, interpolation — two independent renderers ALWAYS differ here and it is NOT a bug) from real fidelity divergence. Report the DISTRIBUTION (per-page mean / 95th-percentile / max channel delta, fraction-of-pixels-over-threshold), never a bare pass/fail.",
      "A corpus-scale run producing a per-file, per-page report that classifies every page's divergence into three buckets, enumerated BY FILE AND BY REASON (R20 discipline): (i) benign-renderer-noise, (ii) known-disclosed-gap (already-counted unsupported behavior — Type3 fonts, sh shading, /SMask, /OC optional content, DeviceCMYK colorimetry — cross-referenced against the EXISTING Diagnostics unsupported-tally so they are subtracted, not re-reported), (iii) unexplained-divergence (the genuine bug candidates, i.e. the residual after subtracting i and ii).",
      "Triage + fix of the in-scope subset of bucket (iii): fix what is cheap and clearly a pdfce render bug; file the rest as counted, named render-gap items (the same R20/R27 counted-shortfall posture pdfce holds everywhere).",
      "Encode the KNOWN reference-divergence cases so pdfium's own quirks are never misattributed to pdfce — specifically the Pass 6.0 finding: pdfium requires FPDF_FFLDraw to draw /Widget appearances, and pdfium SYNTHESIZES some no-/AP appearances (e.g. /Circle /IC fill) that R43 makes pdfce correctly REFUSE. These are REFERENCE divergences; the harness must bucket them as such, not as pdfce errors.",
      "Wire the harness into the repeatable standing-gate set (the R34/R46 pattern) so every future render-touching Pass — ESPECIALLY A/vector — re-runs it. This is the durable payoff: A's content-stream edits inherit a standing full-page fidelity gate they otherwise would not have.",
      "DeviceCMYK→RGB colorimetry (already filed: Rgb::from_cmyk naive additive vs pdfium's AdobeCMYK_to_sRGB1; measured 37.4% of pixels >8 delta on the corpus CMYK JPEG, affecting ALL DeviceCMYK fills/strokes) is the first systematic divergence the harness will light up corpus-wide. Characterize + quantify it corpus-wide in-Pass; fix it here ONLY if bounded (adopt a calibrated table, scoped via pdfce-acrobat-librarian's already-filed 'what does Acrobat do for uncalibrated DeviceCMYK→screen' question), otherwise file it as the harness's first named residual. Do NOT confound the colour fix with the harness build (decision 006's don't-confound discipline; re-pin decision 006 §3.4's polarity matrix BEFORE any colour change lands, per 006 revisit-trigger 7)."
    ],
    "acceptance_criteria": [
      "The harness runs over the full loadable corpus with ZERO panics/timeouts, producing a deterministic, locale-invariant, per-file/per-page report.",
      "Every page's divergence is classified into benign-noise / known-disclosed-gap / unexplained, with the tolerance band DOCUMENTED and JUSTIFIED — never tuned to make a number green (the W14 discipline: do not weaken the threshold to pass; a systematic divergence is a finding to run down, not to average away).",
      "The 'unexplained' bucket is genuinely the residual AFTER subtracting benign noise and already-counted gaps — cross-checked against the existing Diagnostics unsupported-tally so no already-disclosed gap is double-counted as a new bug.",
      "Every item in the 'unexplained' bucket is either FIXED or filed as a named, counted render-gap (R20/R27) — enumerated by file and reason, never rounded away.",
      "Known pdfium reference-divergences (FPDF_FFLDraw widgets; pdfium's synthesized no-/AP appearances) are encoded and bucketed as reference-side, never flagged as pdfce errors.",
      "The harness is added to the standing gate set and documented as a REQUIRED re-run for every render-touching Pass (the R34/R46 pattern). This is the acceptance criterion that makes it de-risk A durably rather than being a one-shot report.",
      "Standing gates green: cargo fmt --check, clippy -D warnings, GUI-free cargo tree (pdfce-core + pdfce-render, host + x86_64-pc-windows-msvc), wasm32, --duplicates, no-network; ui-strings N/A (tooling, no GUI change). Prior Pass gates (R34 Pass 3.0 identity, R46 content-identity) UNMOVED.",
      "No new RUNTIME dependency in pdfce itself. pypdfium2 is a dev/tooling dependency, out-of-tree like the existing harnesses; if newly vendored, LEGAL §6 classification + note (it does NOT enter pdfce's shipped dependency set or THIRD_PARTY_LICENSES.md)."
    ],
    "explicit_non_goals": [
      "NO new render CAPABILITY. Type3 fonts, sh shading, /SMask, /OC, and DeviceCMYK colorimetry stay their own filed items; this Pass MEASURES and buckets them, it does not implement them (beyond the cheap, clearly-a-bug fixes it surfaces). Chasing every gap here would double the Pass and confound measurement with feature work.",
      "NO editing capability of any kind.",
      "NOT chasing benign anti-aliasing / sub-pixel noise to zero — the deliverable is CHARACTERIZING it, not eliminating it. Two independent renderers never agree at the pixel level and demanding they do is a category error.",
      "NO GUI visual-diff surface this Pass — the harness is a corpus report + gate, tooling-only. (A GUI 'compare-against-reference' view is a natural later B addition, not this Pass.)",
      "NOT a claim of 'pixel-perfect.' The deliverable is a MEASURED, bucketed fidelity report with the residual named — the R20/R27 counted-shortfall posture, not a marketing claim. Do NOT report the Pass 1.1 remainder 'closed' unless the harness genuinely generalizes to full-page corpus scale (the exact caveat Pass 6.0's ship notes already flagged)."
    ],
    "spec_rag_prerequisites": [
      "Minimal — this is measurement, not spec-governed behavior; no BLOCKING pdfce-spec-librarian dispatch.",
      "IF the DeviceCMYK colorimetry fix is taken in-Pass: pdfce-acrobat-librarian for the already-filed 'what does Acrobat actually do for uncalibrated DeviceCMYK→screen' question (capability/behavior only). Non-blocking for the harness itself.",
      "pypdfium2 tooling already precedented (decision 006 §3.2; tools/annot-pdfium-diff.py) — no new methodology to source."
    ],
    "risks": [
      {"id": "Y1", "risk": "Renderer-difference NOISE swamps the SIGNAL — the central risk. Two independent renderers always differ (AA, hinting, sub-pixel). If the tolerance band is set wrong, either real bugs hide under noise or benign noise is chased as bugs forever.", "mitigation": "Define the band EMPIRICALLY from the distribution and DOCUMENT the justification; report distributions not pass/fail; subtract known-gaps first; the two forbidden failure modes are (a) tuning the threshold until green (W14) and (b) declaring benign noise a bug. The analytical core of this Pass IS separating noise from bugs — budget for it."},
      {"id": "Y2", "risk": "pdfium's own quirks misattributed to pdfce (the Pass 6.0 FPDF_FFLDraw / synthesized-appearance finding).", "mitigation": "Encode known reference-divergences; the harness buckets them reference-side. Documented as a deliverable."},
      {"id": "Y3", "risk": "Scope creep into fixing every render gap the harness surfaces — doubles the Pass and confounds measurement with feature work.", "mitigation": "The non-goal binds: fix cheap/clear pdfce bugs, file the rest as named residuals. This is a MEASUREMENT Pass."},
      {"id": "Y4", "risk": "The harness surfaces a SYSTEMATIC divergence pdfce cannot cheaply fix (e.g. text positioning or blend at scale). Feels like a Pass failure.", "mitigation": "It is a FINDING, not a failure — it is precisely why C runs before A. File it; it may itself reprioritize (see revisit triggers). Discovering it now, with zero editing code depending on it, is the cheapest possible time."},
      {"id": "Y5", "risk": "DeviceCMYK colour fix confounds the harness build if done together (decision 006's polarity-vs-colour confounding hazard).", "mitigation": "Characterize colour as a bucket first; fix separately; re-pin decision 006 §3.4's polarity matrix before any colour change (006 revisit-trigger 7)."},
      {"id": "Y6", "risk": "Unchanged from every prior Pass (W15/X16): no git remote, CI never run, Passes 0–8 ALL uncommitted. A new harness gate that only ever runs locally can rot.", "mitigation": "The gate must be runnable + green LOCALLY as a hard criterion; establishing a remote stays the operator's LEGAL §1-gated call."}
    ]
  },

  "second_pass_scope_summary_B": {
    "pass_id": "Pass 12 (proposed; consolidates the three existing named GUI follow-up slices under one foundation)",
    "name": "Canvas-interaction foundation + interactive editing-GUI consolidation",
    "first_slice_build_once": "The ONE canvas-interaction substrate: make the canvas Response focusable (Sense::click_and_drag; resolve main.rs's Pass-1 'revisit when the canvas gains focusable content' caveat); add viewer::screen_to_page + page_to_screen (page-space geometry storage, rotation-correct via current_extent); a tool-mode dispatch state machine; hit-testing + selection; live-preview overlay painting; and the drag-vs-pan suppression rule. This is pass-6.1-markup-tools.md §1–§2 verbatim, built generically so markup-draw, form-fill, redaction-mark, AND vector-object editing all layer onto the SAME focusable widget.",
    "subsequent_slices": "Land the three deferred editing GUIs on the foundation, each already fully specced: markup drawing (pass-6.1-markup-tools.md P0 ten-tool state machine + property bar + keyboard map), interactive form-fill (pass-7-form-fill.md), redaction marking (pass-8-redaction.md, the R52 mark-then-apply canvas). Ship per the specs' own P0/P1/P2 cut lines; each slice is pure GUI/interaction work — the core authoring engines are complete.",
    "why_before_A": "A's vector-editing UI is the SAME canvas-interaction problem (select/drag/marquee/node-edit) and is impossible without this substrate. Building it here, once, converts A's UI from new work into reuse — and consolidating markup/form-fill/redaction onto one foundation is exactly the R49 one-pipeline discipline applied to interaction (a second parallel canvas path is forbidden).",
    "note": "B is where pdfce stops being 'a strong engine + CLI + viewer' and becomes an actual editor a human uses — the visible payoff of the entire engine-heavy arc. dispatch pdfce-ui-specialist for the foundation's interaction taxonomy (it already owns the five-way placement taxonomy the specs reference)."
  },

  "third_pass_A_and_inkscape_catalog_owner": {
    "pass_id": "Pass 9 (its existing reserved decision-008 ID — never renumbered)",
    "name": "Vector / content-stream editing — Inkscape parity, sliced",
    "promoted_onto": "A trustworthy render (C) + an existing canvas-interaction foundation (B) + Pass 6.1's content-stream serializer + Pass 8.0's advance-preserving content-stream surgery interpreter. Decision 008 §5.3's 'foundation lands second' is now fully realized: A's hardest foundations (serialize + surgery + selection/hit-test) all exist.",
    "slicing": "Per decision 008 §5.3 / the ROADMAP bucket: (a) object model + hit-test + selection; (b) transforms / z-order / group-ungroup; (c) node/Bézier editing; (d) boolean path ops; (e) gradients + shading + transparency (decision 007 fold-in); (f) OCG layers; (g) text-to-path. Slice (a) reuses B's selection/hit-test directly.",
    "inkscape_capability_catalog_owner_ASSIGNED": {
      "decision": "COMMISSION A NEW SIBLING LIBRARIAN AGENT — pdfce-inkscape-librarian — modeled on pdfce-acrobat-librarian, owning an LLM-optimized Inkscape capability-parity RAG (proposed location D:\\Dev\\Rag-Specialized\\Inkscape_Features\\, mirroring D:\\Dev\\Rag-Specialized\\Acrobat_Features\\).",
      "remit": "Catalogs Inkscape's editing CAPABILITY / behavior / edge-cases / limits ONLY — never its GUI mechanics (menu paths, dialogs, panels, trade dress). pdfce's vector-editing UI is designed INDEPENDENTLY by pdfce-ui-specialist. Same capability-not-GUI discipline the acrobat-librarian is bound by (project rule 12), and the LLM-optimized grep-first RAG format (project rule 14).",
      "binding_license_note": "Inkscape is GPL-2.0-or-later — behavioral/capability reference ONLY, NEVER a dependency, code source, or GUI-mechanics mimicry. Keep this loud (it is the same standing rule as MuPDF/Ghostscript in PRIOR_ART.md, and gates against pdfce's still-undecided license, project rules 8/13).",
      "why_new_agent_not_extend_acrobat_librarian": "The acrobat-librarian's identity is 'Adobe Acrobat Pro feature parity.' Folding a second reference product (Inkscape) into it muddies both RAGs and both agents' single-concern identity. A clean sibling matches the project's one-agent-per-concern roster (rejected alternative: extend the acrobat-librarian's remit).",
      "when": "Commission NOW, during Pass 11 (C). The catalog builds in parallel as a cheap, non-blocking task (exactly how the 'Comments & markup' acrobat bucket was dispatched ahead of Pass 6.1), so it is ready when A is scoped after C and B. It must exist BEFORE A is sliced into real Passes (decision 008 §5.3's still-open obligation)."
    }
  },

  "unchanged_tracks": {
    "D_encryption": "Pass 5 keeps its decision-007 ID and its decision-008 role as the FALLBACK/INTERLEAVE track — independent of the render/canvas/vector arc, the switch target if any of Pass 11/12/9 hits the three-attempts wall. Census payoff still 0.67% (promotion trigger not met). §7.6 spec-corpus now COMPLETE, so it is no longer spec-blocked — but the /R6 AES-256 sourcing gap remains a Pass-5 open sub-decision (only /R5 buildable today). Real promotion trigger stays: an actual encrypted file that blocks operator work.",
    "E_signatures": "Pass 10, LAST — unchanged. Depends on D's crypto; heaviest external sourcing; 0.64% /SigFlags census; read-half far along (signature.rs 810 lines, §12.8 sourced). Signing is the missing half; the incremental-update signing model is already the default save mode.",
    "oldest_operator_item": "The encrypted-PDF refusal (XrefErrorKind::EncryptionUnsupported) STILL has no operator sign-off — the oldest owed item, carried since decision 007. It is an OPERATOR ACTION, not an engineering priority, and does not compete as a 'major investment' — but surface it AGAIN when briefing; it has now been owed across four decisions."
  },

  "proposed_standing_rules": {
    "note": "Numbering continues the R1–R58 corpus; the librarian assigns actual next numbers (expected R59+). This record is the authority for their content.",
    "R-render-fidelity": "pdfce's render stack is proven against an INDEPENDENT reference renderer at corpus scale before any subsystem edits page content it must then re-render. The self-comparison round-trip oracle proves pdfce agrees with ITSELF (sufficient for additive authoring where page content stays byte-verbatim) but NOT that pdfce matches an independent renderer (required once content-stream SURGERY re-renders edited pages). The full-page pixel-parity harness re-runs on every render-touching Pass (the R34/R46 pattern), and its residual is enumerated by file and reason — never a bare pass/fail, never a threshold tuned to pass (W14).",
    "R-one-canvas": "There is exactly ONE canvas-interaction substrate (focusable canvas, screen↔page transform, tool-mode dispatch, hit-test/selection, live-preview overlay). Markup drawing, form-fill, redaction marking, and vector-object editing all layer onto it; a second parallel canvas-interaction path is forbidden — the R49 one-pipeline discipline applied to interaction.",
    "R-inkscape-reference": "Inkscape is a behavioral/capability reference ONLY — GPL-2.0-or-later, never a dependency, never a code source, never GUI-mechanics mimicry. pdfce-inkscape-librarian catalogs capability/behavior/limits (LLM-optimized, capability-not-GUI); pdfce's vector-editing UI is designed independently by pdfce-ui-specialist. (Formalizes the existing binding ROADMAP note as a standing rule.)"
  },

  "revisit_triggers": [
    "Pass 11 (C) surfaces a SYSTEMATIC, structural render divergence pdfce cannot cheaply reconcile → STOP and re-decide in a new record BEFORE Pass 12/9. Vector editing on a render you cannot trust is the exact 'building on sand' this sequence exists to prevent; a large render-correctness finding may itself become the next major investment ahead of B and A.",
    "The operator states they want the interactive EDITOR (usable GUI) before render verification → B can promote ahead of C, accepting that B's live visual feedback runs on an unverified render until C lands. State the tradeoff explicitly; do not reorder silently.",
    "The operator wants VECTOR editing immediately, accepting unverified-render risk → A/Pass 9 can promote after B alone (B is the hard UI prerequisite; C is the correctness prerequisite). Brief that this ships vector edits whose visual correctness is spot-checked, not corpus-measured — a real, disclosed risk, the fuzzy-never-sneaky posture applied to a scheduling decision.",
    "Any of Pass 11/12/9 hits the three-attempts wall → switch to Pass 5 (encryption), the designated independent fallback track (unchanged from decision 008).",
    "A real encrypted file blocks actual operator work → Pass 5 promotes (its census-based trigger is an actual blocked task, not a percentage).",
    "The DeviceCMYK colorimetry divergence (Y5) turns out to dominate the corpus 'unexplained' bucket → it graduates from a Pass 11 residual to its own scoped colour Pass, scoped via pdfce-acrobat-librarian (the already-filed backlog item)."
  ],

  "librarian_followups": [
    "File Pass 11 (render-fidelity verification) under In-progress/Next-up as the immediate next Pass; note it DISCHARGES the long-owed 'full-page pixel-parity remainder (Pass 1.1)' carried in every recent SESSION_LOG 'still open' list.",
    "File Pass 12 (canvas-interaction foundation + editing-GUI consolidation) after it, and RECONCILE the three existing named GUI follow-up slices (Pass-6.1-followup markup drawing state machine; Pass-7 form-fill GUI, docs/ui_specs/pass-7-form-fill.md; Pass-8 redaction-marking GUI, docs/ui_specs/pass-8-redaction.md) as its slices under one shared foundation — not three independent buckets.",
    "Keep Pass 9 (Vector/Inkscape) at its existing ID, repositioned AFTER Pass 12, promoted onto C+B.",
    "Add the three proposed standing rules (render-fidelity gate; one-canvas substrate; Inkscape-reference) at the next R-numbers.",
    "COMMISSION the pdfce-inkscape-librarian agent (.claude/agents/) + the Inkscape_Features RAG scaffold NOW, as a parallel non-blocking task during Pass 11 — mirroring pdfce-acrobat-librarian's structure, remit, and LLM-optimized format. This closes decision 008 §11.4's unowned-catalog item.",
    "Record that decision 010 AMENDS decision 008's revisit-trigger-7 (clean jump to Pass 9 after 6.1) into the C→B→A sequence, and that decision 008's ranking and Pass IDs are otherwise intact.",
    "Re-surface (do not re-file) the still-owed operator items: encryption-refusal sign-off (now owed across four decisions), license decision (LEGAL §1), commit authorization (Passes 0–8 ALL uncommitted), W15 (no remote/CI), the /R6 AES-256 sourcing gap, LEGAL §2 Adobe-supplement copyright contradiction."
  ],

  "operator_actions_owed": [
    "Encryption-refusal operator sign-off (XrefErrorKind::EncryptionUnsupported) — the oldest owed item, now carried across decisions 007/008/009/010. pdfce declines a category of file every other reader opens and the operator has still never been told.",
    "License decision (LEGAL §1) — still gates the public repo, any release, and whether copyleft prior art (including the GPL Inkscape reference's boundary) matters.",
    "Commit authorization — Passes 0–8 are ALL uncommitted; ~20,000+ lines of unversioned work, no remote, no CI history (W15/X16).",
    "The /R6 AES-256 sourcing method + the LEGAL §2 Adobe-supplement copyright contradiction — both Ken's calls when Pass 5 activates (SESSION_LOG continuation-22)."
  ]
}
```

## Orchestrator note (2026-08-01, at archival)

Decision 010 archived from the KenAgent consultation. Outcome: the destination is unchanged — vector/Inkscape-parity editing (candidate A, decision 008's Pass 9) remains the highest-value major investment and the project's distinctive purpose. The accumulated GUI-editing + render-verification debt amends the PATH, not the destination: sequence C (render-fidelity verification, proposed Pass 11) → B (shared canvas-interaction foundation + consolidation of the three deferred editing GUIs, proposed Pass 12) → A (vector editing, Pass 9, repositioned). D (encryption, Pass 5) stays the fallback/interleave track; E (signatures, Pass 10) unchanged-last. Amends decision 008's revisit-trigger-7 (the clean jump onto the 6.1 serializer). Adds 3 standing rules (render-fidelity gate; one-canvas substrate; Inkscape-reference — librarian assigns numbers, expected R59+). Commissions a new pdfce-inkscape-librarian sibling agent + Inkscape_Features RAG scaffold NOW (parallel, non-blocking, during Pass 11). At archival: Pass 11 engineer DISPATCHED (render-fidelity harness — no blocking spec, it is pure measurement); decision-010 filing + the inkscape-librarian commissioning dispatched in parallel. NOTE the candidate letters A–E in this record are LOCAL to decision 010 and differ from decision 008's A–F.
