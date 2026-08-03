# 019 — FF-H: direct text-state formatting (`Tc`/`Tz`/`Ts` + super/subscript), synthetic bold/italic, and the `Tw` evidence gate

**Date:** 2026-08-03
**Status:** Decided
**Decided by:** KenAgent (`autonomous-builder`, decision-consultant mode), per `docs/decisions/README.md`
**Requested by:** `pdfce-engineer` (operator priority #3, "finish off all the text
handling stuff" — `ROADMAP.md` ★★★ Operator priority sequence, item 3)
**Supersedes:** nothing
**Amends:**
- `docs/decisions/014-acrobat-text-editing.md` §5.3 — FF-H's named payload
  ("`Tc`/`Tw`/`Tz`/`Ts` + synthetic styles + minimal StructTree/ActualText
  update") is **re-scoped**: `Tw` is demoted from an FF-H authoring control to
  an evidence-gated conditional slice (§3.3); the **minimal StructTree/
  ActualText update is REMOVED from FF-H entirely** and re-filed as its own
  backlog item (§3.7).
- `docs/decisions/016-ffd-add-new-page-text.md` §2 — FF-H's rank-4 "DEFER
  (slice later)" verdict is superseded by the operator's priority-#3 directive;
  §2's *reasoning* is upheld and partly vindicated (§3.7 acts on its
  "StructTree piece is premature" finding by cutting it).
- `docs/ROADMAP.md` — adds the Pass 19.x family and four standing rules
  R88–R91 (pending librarian numbering; current ceiling **R87**).

**Builds on:** Pass 14.1 (`text_edit/edit.rs` — the advance-preserving
REMOVE→REPLACE surgery and its crate-private `Walk` ambient-state tracker),
Pass 14.2 (`text_edit/format.rs` — the `set_ops`/`restore_ops` symmetric
state-set/state-restore emission pattern this decision generalizes), Pass 15.1
(`text_edit/reflow_apply.rs` — `TJ` slack distribution and the
non-zero-`Tc`/`Tw` justify refusal gate), Pass 16.x (`text_edit/addtext.rs` —
the self-balanced `q … BT … ET … Q` new-content envelope), decision 012
(`GlyphSource`, operator-supplied fonts), Pass 18.6 (`vector/decompose.rs`
`GState` — a *reading*-path `Tc`/`Tw`/`Tz`/`Ts` tracker; see §4.6 for exactly
how little it contributes here)

**Does not touch:** redaction's R35 full-rewrite guarantee · R70's
incremental-save default for editing · the Pass-6.2 FreeText annotation path ·
R75's "reflow is opt-in beside 14.1's single-line relayout" · `LEGAL.md` §1
(MIT, decided 2026-08-01 — no license question arises here; FF-H adds **no**
Cargo dependency)

---

## 1. Context

FF-H was filed (decision 014 §5.3) as a single bundle: "`Tc`/`Tw`/`Tz`/`Ts` +
synthetic styles (disclosed-fuzzy) + minimal StructTree/ActualText update." The
operator has now made it priority #3. Two things have changed since it was
filed, and both change what it should contain.

### 1.1 The parity premise collapsed for two of the four operators

`D:\Dev\Rag-Specialized\Acrobat_Features\text_edit__spacing_and_scaling_controls.md`
(2026-08-03) establishes, from a named source — Dov Isaacs, former Adobe
Principal Scientist, Adobe Community 2019-01-03/04 — that **Acrobat removed
word spacing (`Tw`) and free-form baseline offset when text editing was
consolidated into the single Edit Text & Images tool.** Isaacs' retained list is
"adding, deleting, bold, italic, font size, leading, kerning, and horizontal
scaling." Current Acrobat therefore exposes **character spacing (`Tc`),
horizontal scaling (`Tz`), and a coarse superscript/subscript toggle** — and
nothing else in this family. His only offered remedy for the missing baseline
control is explicitly a hack: "Maybe if you are very clever and with a bit of
luck, you can modify baseline with a combination of the subscript or superscript
attribute and font size."

So FF-H's name lists four operators and **parity covers two**. For `Tw` and
free-form `Ts` there is no Acrobat behaviour to match. That is not the same as
"there is nothing to build" — it means the question stops being *parity* and
becomes *product*, which is what §3.1–§3.3 answer.

### 1.2 What the code actually has today (audited this session, not assumed)

This audit materially changed the decision; three of its findings invert
plausible assumptions.

- **`format.rs` (14.2) already implements the exact emission shape FF-H
  needs.** `plan_format` (`format.rs:535`) builds `set_ops` / `restore_ops`
  (`format.rs:627–640`) and splices the anchor as
  `pre | set_ops | mid | restore_ops | post` (`format.rs:655–659`). Today it
  carries exactly two families: `push_tf` (`/Name size Tf`) and `push_fill`
  (`rg`/`g`/`k`). **`Tc`/`Tz`/`Ts` slot into this with no structural change.**
  FF-H's core slice is an extension of a shipped, tested pattern — not a new
  mechanism. This is the single biggest cost input to §3.6's ordering call.
- **The ambient-restore *semantics* are already precedent, too.**
  `TextColor::restore_bytes` (`edit.rs:172`) restores `0 g` when the ambient was
  provably the spec default, restores the **raw observed bytes** when the
  ambient was a real operator (including `Other { raw }`, an unmodelled colour
  space), and the caller discloses a `fill_narrowed` warning
  (`format.rs:639`, `format.rs:722`) when the restore narrows semantics. FF-H
  should copy this ladder verbatim rather than invent one (§3.4).
- **`Ts` and `Tr` are not tracked in the authoring path at all.**
  `text_edit::edit::Walk` (`edit.rs:420–439`) has arms for `b"Tc"` (L526),
  `b"Tw"` (L532), `b"Tz"` (L538) — and **no `b"Ts"` and no `b"Tr"` arm.**
  `ShowData` (`edit.rs:382–396`) carries `tc`/`tw`/`th` and no rise. The
  *read* path is fine (`text_extract::page::TextState`, `page.rs:199–242`,
  tracks all of `Tc`/`Tw`/`Tz`/`TL`/`Ts`/`Tr`) — but it **drops every one of
  them at provenance-construction time**: `GlyphProvenance`
  (`text_extract/mod.rs:320–348`) carries only `tf_size`, `fill_color`,
  `text_matrix`, `ctm`, `font_resource`, `operator_span`, `content_stream`.
  Consequence: **pdfce cannot today restore an ambient rise it never observed.**
  That is a prerequisite gap, and it is why §6's slice 19.0 exists.
- **Ambient spacing state is tracked three times, privately, and published
  zero times.** `text_extract::page::TextState` (private),
  `text_edit::edit::Walk` + `reflow_apply::BlockTextState`
  (`reflow_apply.rs:796–802`, whose own doc comment concedes it exists because
  "provenance does not all carry" these), and `vector::decompose::GState`
  (`decompose.rs:1179–1204`, private, Pass 18.6). The codebase already documents
  this failure shape in the abstract — `text_extract::font::advance_tx`
  (`font.rs:314–323`) carries a comment about "three copies that agree today."
  Consolidation is overdue and FF-H is the forcing function.
- **There is a live, currently-masked state-leak surface in `reflow_apply.rs`.**
  The preamble conditionally emits `ts.tc Tc` / `ts.th*100 Tz` / `line_tw Tw`
  (L411–432) and the body terminates at `ET` (L455) with **no restore, no
  `q`/`Q`**. It is benign *today* only because of the justify gate (L397–405),
  which refuses justify when `|tc| > ε || |tw| > ε`, and because the
  non-justify path re-emits values equal to ambient. Any FF-H work that relaxes
  that gate — and FF-H is exactly the work that would want to — turns a latent
  hazard into a real one.
- **`addtext.rs` is already self-scoped.** Both `build_content` (L849–891) and
  `build_content_boxed` (L1475–1518) wrap their `BT … ET` in a balanced
  `q … Q`, so text state set inside is restored by `Q` for free. Insertion
  points for new operators are `addtext.rs:866` and `addtext.rs:1490`. **The
  add-text half of FF-H therefore needs no restore machinery at all** — an
  asymmetry §3.4 exploits.
- **Composite detection exists but is crate-private.**
  `ExtractFont::is_simple()` (`font.rs:724–726`, `pub(crate)`) over a private
  `enum CodeWidth { One, Two }` (`font.rs:208–216`; `Two` for `Identity-H/V`
  and every 2-byte codespace, `font.rs:504`). No public run-level composite
  flag exists on `TextRun`, `ExtractedGlyph`, `GlyphProvenance`, or
  `decompose::TextFont` — a GUI wanting "is this run 2-byte?" must today
  provoke an error return. R83 ("no affordance without the capability") cannot
  be honoured for a `Tw` control until this is published (§3.3).
- **`CommandKind::FormatText` already exists** (`edit.rs:~305`) with the
  session entry point `EditSession::format_text` (`edit.rs:1741`). FF-H needs
  **no new command variant** — spacing/style changes are format operations and
  ride the existing one-command-per-accepted-edit undo semantics.

### 1.3 The spec facts that constrain every option

From `PDF_Spec` `iso32000__s__9.3.md` and `iso32000__s__9.4.md`
(ISO 32000-1 §9.3, §9.4.4) and `iso32000__s__8.2.md` (§8.2 Table 51):

1. **`Tc`/`Tw`/`Tz`/`TL`/`Tf`/`Tr`/`Ts` are graphics state**, "retained across
   text objects in a single content stream," and "initialized to their default
   values at the beginning of each page." `BT` resets **only** `Tm`/`Tlm`
   (Table 107) — it does **not** reset text state.
2. **`q`/`Q` are "Special graphics state" operators and are NOT permitted
   inside a text object** (§8.2 Table 51 + Figure 9: a text object admits
   general graphics state, colour, text state, text-positioning, text-showing,
   marked content — *not* special graphics state). **The obvious "just wrap the
   run in `q`/`Q`" scoping strategy is illegal inside `BT…ET`.** It is legal
   *around* a whole text object, which is exactly why `addtext.rs` gets away
   with it and `format.rs` cannot.
3. **`Tw` applies only to single-byte code 32** — "It shall NOT apply to
   occurrences of the byte value 32 in multiple-byte codes." Structurally void
   for 2-byte composite/CID runs.
4. **`Tc` and `Ts` are in *unscaled text space units* — not scaled by `Tfs`.**
   A `Ts` of 3.3 is 3.3 text-space units whether the font is 10 pt or 30 pt.
5. **`Th` (= `Tz`/100) multiplies `Tc`, `Tw`, *and* every `TJ` numeric
   adjustment** in horizontal writing mode (§9.3.4), and appears in the advance
   formula `tx = ((w0 − Tj/1000)·Tfs + Tc + Tw)·Th` (§9.4.4).
6. **`Trise` enters the text rendering matrix as a translation**
   (`Trm` row 3 = `[0, Trise, 1]`), so it changes position but **not** advance.
7. **Stroked text uses the current *stroking* colour, and its line width is
   interpreted in *user space*, not text space** (§9.3.6) — so a stroke width
   is not scaled by `Tfs`.

---

## 2. Options considered

**Q1 — the `Tw`/`Ts` authoring surface.**
- **A-parity** — ship exactly Acrobat's current set: `Tc`, `Tz`, super/subscript
  toggle. Nothing more.
- **A-exceed-both** — ship `Tc`, `Tz`, toggle, **plus** free-form `Ts` **plus**
  gated `Tw`, restoring both capabilities Acrobat dropped.
- **A-split** — treat `Tw` and `Ts` as separate questions with separate answers,
  scoped by font-model universality and marginal cost rather than by
  "match/exceed."
- **A-engine-only** — expose nothing beyond parity, but make all four
  first-class in the engine (read, track, preserve, honour in advance math).

**Scoping mechanism (how an emitted operator is confined to its run).**
- **S-qQ** — wrap in `q`/`Q`.
- **S-restore** — emit the new value, show, then emit an explicit restore of the
  resolved ambient value (the `format.rs` `set_ops`/`restore_ops` pattern).
- **S-tj** — emit no state at all; express the effect as `TJ` numeric
  adjustments inside the show operator.
- **S-normalize** — reset text state at every `BT` stream-wide.

**Unit/model for size-relative quantities.**
- **U-absolute** — store and emit exactly what the operator typed, in text-space
  units.
- **U-ratio** — store as a fraction of the font size, derive the operand at emit
  time.
- **U-discriminated** — store a tagged `Absolute | Relative`, operator picks.

**Q2 — synthetic bold/italic across the two authoring paths.**
- **Y-identical** — one policy, one code path, both paths.
- **Y-divergent** — different policy for Add-Text vs in-place edit.
- **Y-addtext-only / Y-edit-only** — offer synthesis on only one path.

**Q2 mechanism.**
- **M-doublestrike** — show the run twice at slightly different sizes (the
  Enfocus-documented industry technique).
- **M-stroke** — text rendering mode 2 (fill-then-stroke) with a small stroke
  width.
- **M-shear-tm** — oblique by premultiplying a shear into `Tm`.
- **M-shear-glyph** — synthesize sheared outlines and embed them (requires FF-C).

**Q2 persistence.**
- **P-session** — provenance marker lives only in pdfce's session model.
- **P-private-key** — write a private `/PieceInfo`-style marker into the PDF.
- **P-selfevident** — choose emission mechanisms whose *bytes* are
  re-detectable by inspection on reload; no private extension.

**Q3 — ordering of FF-H / FF-B / FF-C.**
- **O-H-C-B**, **O-C-H-B**, **O-B-first**, **O-parallel**.

---

## 3. Decision

### 3.1 Q1 headline — **A-split**. Scope by font-model universality and marginal cost, not by "parity vs exceed."

"Match Acrobat" and "exceed Acrobat" is the wrong axis for this question, and
using it is what made FF-H look like one item. The right axis is: **does the
control work the same way on every run the operator can select, and what does
it cost given what is already built?** On that axis the four operators are not
a set of four — they are a group of three and an outlier.

| Operator | Universal across font models? | Marginal cost given §1.2 | Verdict |
|---|---|---|---|
| `Tc` | Yes — applies to every glyph, simple or composite | Low: one more `set_ops`/`restore_ops` pair | **Ship (parity)** |
| `Tz` | Yes — `Th` scales all horizontal displacement | Low: same | **Ship (parity)** |
| super/sub toggle | Yes — `Ts` + `Tfs`, both font-model-agnostic | Low: composes with the existing `push_tf` | **Ship (parity)** |
| `Ts` free-form | Yes — a `Trm` translation, identical for CID | **Near zero** once the toggle exists | **Ship (deliberate exceed)** |
| `Tw` | **No** — structurally void for 2-byte codes (§1.3.3) | Low to build, high to *explain* | **Evidence-gated (§3.3)** |

### 3.2 Free-form `Ts` **ships** — the one deliberate exceed, and it is nearly free

**Decision: expose a free-form numeric baseline-rise control in the GUI and
CLI, beyond Acrobat's coarse superscript/subscript toggle.**

Four reasons, in descending weight:

1. **pdfce must build the `Ts` emission path anyway.** Superscript/subscript is
   a *parity must-have*, and the only PDF-native mechanism for a baseline shift
   is `Ts`. Once `Ts` emission, `Ts` restore, `Ts` ambient-tracking, `Ts`
   round-trip tests and the `Ts` CLI plumbing all exist to serve the toggle,
   *withholding the number* is a deliberate act of hiding a built capability.
   The marginal cost of exposing it is a numeric field, one CLI flag, and two
   tests. There is no world in which that is the wrong trade.
2. **`Ts` is font-model-agnostic.** It is a translation in the text rendering
   matrix (§1.3.6). It behaves identically for Type1, TrueType, CFF, and
   Identity-H CID runs. It has no spec-level void case, no refusal class, no
   "it depends on your font" caveat to explain. It is the *cleanest* control in
   this entire family — cleaner than `Tw`, and cleaner even than `Tc` (which is
   at least entangled with `Th`).
3. **Acrobat's removal of it is confounded and carries almost no signal.**
   Isaacs attributes the loss to a *tool consolidation* — "What was done a
   number of releases back was to consolidate all text editing into the Edit
   Text & Images tool" — not to a usage study, a deprecation, or a technical
   obstacle. A consolidation drops whatever didn't fit the new panel. And the
   giveaway is his own workaround: he tells a user to abuse the
   superscript attribute plus a font-size change to fake a baseline shift.
   **Adobe's own former Principal Scientist describing a hack to recover the
   capability is evidence the need survived the feature.** (See §4.1 for the
   general form of this argument and where it does *not* apply.)
4. **The real use cases are not superscript.** Aligning an inline symbol to a
   scanned baseline, nudging a footnote marker that a producer placed 0.5 pt
   low, matching a signature line, correcting a chemistry/math run whose
   producer emitted a wrong rise. A two-position toggle cannot express any of
   them. This is precisely the class of "fix what the producer got slightly
   wrong" work that pdfce exists for.

**Units — U-discriminated.** `Ts` is in *unscaled text space units* (§1.3.4),
which produces a real trap: a superscript applied at 10 pt and then resized to
20 pt keeps its absolute rise and lands in the wrong place. So:

- **Superscript/subscript are stored as *ratios*** of the font size and the
  `Ts` operand is **re-derived whenever `Tfs` changes**. This is R89.
- **Free-form `Ts` is stored as *absolute*** by default — what the operator
  typed is what they get — via a discriminated `MetricSpec::{Absolute(f32),
  Relative(f32)}`. An operator may opt a free-form rise into `Relative`.
- The same discriminated type governs `Tc` (also unscaled text space units).
  The GUI's default presentation for `Tc` is **em/1000** (`Relative`), matching
  the typographic notion of tracking and `TJ`'s own unit space; an absolute
  point value is accepted and stored as `Absolute`.
- `Tz` needs no discrimination — it is a dimensionless percentage (§1.3.5).

**pdfce's own super/subscript defaults** (documented as pdfce's, explicitly
**not** a parity claim — Acrobat's actual values are an unsourced GAP in the
catalog): size factor **0.60 × Tfs**; superscript rise **+0.34 × Tfs**;
subscript rise **−0.18 × Tfs**. Operator-overridable, disclosed by value in the
report so nothing is a hidden magic number.

### 3.3 `Tw` does **not** ship as an authoring control in FF-H's core — it is engine-first, R83-gated, and **evidence-gated behind a corpus census**

**Decision:** `Tw` is **fully first-class in the engine** (read, tracked,
published on provenance, honoured in advance math, preserved byte-verbatim,
displayed read-only in the properties panel with an explanation) and is **not**
an authoring control in slices 19.0–19.3. Whether it ever becomes one is
decided by a **corpus census** (§6, slice 19.4), not by argument.

Three independent reasons, any one of which would be sufficient to *not* put it
in the core slice:

1. **Its only real job already belongs to another subsystem, which does it
   better.** The honest use case for a word-spacing dial is "the gaps between
   these words are wrong" — which is a *justification* artifact. pdfce already
   owns that problem in the reflow layer, and decision 015 §3.1 already chose
   `TJ` over `Tw` as the general slack path *for exactly the right reasons*:
   `TJ` works on composite fonts, places slack only at chosen gaps (`Tw` hits
   **every** code-32, including leading, trailing and doubled spaces), and is
   immune to the state-persistence leak because the numbers are local to their
   array (`iso32000__s__9.3.md` "`Tw` over-count + LEAK hazards").
   **Adding a manual `Tw` dial re-introduces, as a feature, the exact
   mechanism decision 015 rejected as an implementation.** Two operator-facing
   dials that both look like "space between words," that interact
   multiplicatively through `Th`, and one of which silently does nothing on
   half the corpus, is a bad product regardless of how cleanly it is coded.
2. **Its availability is determined by a property the operator cannot see and
   did not choose.** `Tw` is void for 2-byte codes. Modern producers embed font
   subsets as Type0/Identity-H composites *even for pure-Latin text* — which
   would make the control inert on a large and **growing** share of exactly the
   documents pdfce targets. The catalog's `must_have` is a refuse-and-disclose
   gate; R83 ("no affordance without the capability") argues for going further
   and **not rendering the affordance at all** on a composite run. Either way
   the operator's experience is "this control sometimes exists." That is a
   defensible *engine* behaviour and a poor *product* behaviour.
3. **Nobody has measured whether it can ever apply.** Claim 2 above is a
   hypothesis with a high prior, not a measurement — and this project has an
   established habit of settling exactly this class of question with a corpus
   census rather than a guess (the 0.67% `/Encrypt` census that kept Pass 5 off
   the critical path is the precedent). Building a control on an unmeasured
   premise, in either direction, is the thing to avoid.

**The census (slice 19.4's gate), specified concretely.** Over the Pass-11
render-fidelity corpus, produce two numbers:

- **(a) Reachability** — of all show operators in the corpus, the fraction
  whose font is simple (`CodeWidth::One`). This bounds how often a `Tw` control
  could do anything at all.
- **(b) Prevalence** — the fraction of *documents* that set a non-default
  `Tw` / `Ts` / `Tc` / `Tz` anywhere. This sizes the **preservation** risk
  (§3.5) independently of the authoring question, and it is worth having no
  matter what (a) says.

**Gate:** (a) ≥ 60% → build `Tw` as an R83-gated control in 19.4. (a) ≤ 25% →
**close the item**, recording the census as the reason, and point the operator
at reflow/justify. Between 25% and 60% → **escalate to Ken** (§10) — that band
is a product call about how much surface a sometimes-control is worth, not a
technical one, and I should not make it on his behalf.

**What ships regardless:** if a `Tw` control is ever built, it emits `Tw` **only**
on simple-font runs, never on composite (R91), and inter-word distribution on a
composite run is `TJ`-only — the decision-015 path, reused, not re-derived.

### 3.4 Scoping mechanism — **S-restore**, with `q`/`Q` formally ruled out and a two-tier ambient ladder

**Decision: explicit restore-by-value, emitted inside the same text object,
extending the existing `format.rs` `set_ops`/`restore_ops` pattern.**

`q`/`Q` (**S-qQ**) is **not merely inferior, it is illegal** inside `BT…ET`
(§1.3.2, §8.2 Table 51/Figure 9). Splitting the text object to get around it
(`… ET q … BT …`) would discard `Tm` (§9.4.1: "`ET` … discarding the text
matrix"), forcing absolute re-positioning of everything downstream and
destroying the minimal-diff property. **Rejected on spec grounds, not taste.**
This is recorded explicitly because "wrap it in q/Q" is the first thing any
implementer will try.

**S-normalize** (reset text state at every `BT` across the stream) is rejected
by `ARCHITECTURE.md` §5.6 — pdfce never normalizes — and would rewrite objects
pdfce did not logically touch, violating R32/R46 directly.

**S-tj** is rejected as the *general* path: it can express `Tc` (per-glyph `TJ`
adjustments) and inter-word spacing, but **cannot express `Tz`** (which changes
glyph *shape*, not only displacement — §9.3.4: "shall affect both the glyph's
shape and its horizontal displacement") and **cannot express `Ts`** (a `Trm`
translation). It also bloats the stream and destroys semantic legibility: a
later reader, human or machine, can no longer tell that the run has tracking
applied. It is retained only as the composite-font fallback for inter-word
distribution (§3.3), where decision 015 already put it.

**The ambient-resolution ladder — copy `TextColor::restore_bytes`, do not
invent.** The existing precedent (`edit.rs:172`) is exactly right and FF-H
adopts it verbatim for all four operators:

1. Ambient provably at the Table-105 default (`Tc`=0, `Tw`=0, `Tz`=100,
   `Ts`=0, `Tr`=0) and never set in the stream prefix → restore by emitting the
   **spec default**.
2. Ambient set by an observed operator in the stream → restore the **observed
   value**, preferring the **raw operand bytes** where they are available
   (mirroring `TextColor::Device { raw } | Other { raw }`), so a `0.5000 Tc` is
   not silently renormalized to `0.5 Tc`.
3. **Ambient unobservable** → **refuse and disclose.** Two real cases:
   (i) the page's `/Contents` is an *array* of streams (§7.8.2) and state was
   set in an earlier element — the walk must span the concatenated array, and if
   it cannot, it must say so; (ii) the run lives inside a **Form XObject**,
   which inherits text state from its invoking context and therefore has no
   in-stream ambient at all. In both cases emitting a guessed `0 Tc` as the
   "restore" would silently change content pdfce did not touch — precisely the
   rule-4 failure. **Refuse-and-disclose by name; never guess the default.**
4. Where a restore is emitted that narrows semantics, **disclose it**, exactly
   as `fill_narrowed` / `disclosure_narrowing` already does (`format.rs:639`,
   `format.rs:722`).

**The add-text asymmetry is real and is exploited, not papered over.**
`addtext.rs` emits inside a balanced `q … Q` *outside* its `BT…ET`
(`addtext.rs:849–891`, `1475–1518`), so new text needs **no restore
obligation** — the `Q` does it. FF-H's add-text half inserts operators at
`addtext.rs:866` / `addtext.rs:1490` and stops. The in-place-edit half uses the
full ladder above. Same operator names, two correct mechanisms, both documented
so a future maintainer does not "unify" them into a bug.

**The `reflow_apply.rs` leak is closed as part of 19.0, before anything is
built on it.** `reflow_apply.rs:455` terminates at `ET` with no restore after
having conditionally emitted `Tc`/`Tz`/`Tw` (L411–432). Slice 19.0 adds the
symmetric restore and keeps the justify gate (L397–405) in place; slice 19.1
may then relax the gate *because* the leak is closed, not before.

### 3.5 Round-trip preservation of pre-existing `Tw`/`Ts` — the framing is **correct**, and it is bigger than stated

**Confirmed: preservation is mandatory regardless of the authoring answer, and
is not part of Q1.** It follows from R32/R46 minimal-diff and R69 (only the
edited content stream is re-emitted, everything else byte-verbatim) without any
new rule.

But the framing in the request understates it in one way worth recording,
because the understated part is where the actual bugs are. Preservation has
**three** distinct obligations, and only the first is automatic:

1. **Byte-verbatim survival of operators pdfce doesn't rewrite.** Automatic
   under R32/R46/R69 — pdfce simply doesn't touch those bytes. No work.
2. **Semantic survival across a rewrite.** When surgery re-emits a run's show
   operator, the enclosing/preceding state operators must survive and the
   restore must return the stream to its true ambient. This is §3.4's ladder.
   *Not* automatic; this is real work.
3. **Ambient state as an *input* to the surgery's own arithmetic.** §9.4.4 is
   `tx = ((w0 − Tj/1000)·Tfs + Tc + Tw)·Th`. An advance-preserving surgery that
   assumes `Tc=Tw=0, Th=1` mis-positions followers inside a `Tz 90` or
   `Tc 0.5` context. **Audited: pdfce gets this right for `Tc`/`Tw`/`Th`
   today** — `format.rs:610/614` feeds `anchor.tc`/`tw`/`th` into the advance
   math, and `edit.rs`'s `Walk` tracks all three. **It does not track `Ts` or
   `Tr` at all** (§1.2). `Ts` does not enter the advance formula, so this is a
   *restore* gap rather than a positioning corruption — but it is a real gap
   and it blocks §3.2 outright.

So: preservation is mandatory, is correctly framed as out of scope for Q1, and
its unfinished portion (obligations 2 and 3 for `Ts`/`Tr`, plus publishing the
state on provenance) **is the content of slice 19.0** — which is a correctness
slice that adds no operator-facing surface at all.

### 3.6 Q2 — synthetic bold/italic: **identical policy, shared core type, differing remedy *order*, self-evident emission**

**Decision: Y-identical on policy, with one deliberate, documented asymmetry
that is about *ordering the offer*, not about differing rules.**

**One policy, one code path.** The gate, the wording, the declinability, the
provenance marker, and the refusal behaviour are a single `pdfce-core` type used
by both `format.rs` (14.2 in-place) and `addtext.rs` (16.x add-text). Reasons:
(a) two policies means two behaviours the operator must learn and inevitable
drift; (b) both paths already share the *same* upstream font resolution —
decision 012's `GlyphSource` and R79's bundled/supplied face policy — so
sharing the *fallback* is the consistent choice; (c) the catalog could not
resolve what Acrobat does, so there is no parity cost to being coherent.

**The gate is unchanged from the catalog's `must_have` and is narrow.**
Synthesis is offered **only** when no real Bold/Italic resource resolves — it is
the fallback after `resolve_target_resource` (`format.rs:861–894`) fails, never
an alternative to a real face. It is **per-use, named, and declinable**
("no true Bold resource is available for `<font>` — synthesize a faux bold?
[accept] [decline]"), never a global preference. This is deliberately **stricter
than Acrobat**, whose mechanism is a set-and-forget "Enable Artificial
Bold/Italic styles" preference under Content Editing → Font Options with no
evidence of any per-use disclosure. Rule 4 (fuzzy, never sneaky) requires the
stricter posture; this is a direct application, not a new principle.

**The one asymmetry: the *order of offered remedies* differs, because the two
paths have different obligations to the document.**

- **Add-Text (16.x)** — new content owes nothing to any existing family. When
  no real bold resolves, the *better* first offer is **"use a face that has a
  real Bold"** (and per R79 the default is a bundled Standard-14 face, whose
  family — Helvetica-Bold, Times-Bold, Times-Italic, Courier-Bold — means a
  real variant almost always exists, so **the synthesis gate will rarely even
  open here**). Synthesis is offered second.
- **In-place edit (14.2)** — the run belongs to the document's own typography.
  Changing its family to get a real bold is the *more* visually disruptive
  action, and may not be what the operator wants at all. So synthesis is
  offered **first** (least disruption), and "change the family to one with a
  real Bold" second.

Both remedies are offered on both paths; both are declinable; only the ordering
differs, and the ordering is disclosed. Note this asymmetry is **emergent, not
special-cased** — it falls out of R79's default-font policy plus the gate,
which is why it costs nothing to implement.

**Mechanism — M-stroke + M-shear-tm, both spec-native.**

- **Synthetic bold: text rendering mode 2** (fill-then-stroke, §9.3.6 Table 106)
  with stroke width ≈ **0.022 × the effective rendered size**. Chosen over
  **M-doublestrike** (the Enfocus-documented "printing two characters on top of
  each other, one one point size bigger") because double-strike doubles the
  glyph count, changes text extraction output, corrupts the run's byte↔glyph
  correspondence that provenance and hit-testing depend on, and produces a
  *size*-inflated rather than *weight*-inflated result. Mode 2 is one operator.
  **Two spec gotchas that are bugs if missed** (§9.3.6): the stroke uses the
  **current stroking colour**, so it must be set to match the non-stroking
  colour and restored — otherwise coloured text acquires black outlines; and
  **line width for stroked text is interpreted in user space**, so the width
  must be derived from `Tfs × |Tm scale| × |CTM scale|`, not set as a constant.
  `Tr` is text state, so it rides §3.4's restore ladder — and `Tr` has no arm in
  `Walk` today (§1.2), which is a second reason 19.0 must precede 19.2.
- **Synthetic italic: a shear premultiplied into the run's `Tm`**, 12°
  (`tan 12° ≈ 0.2126`). `format.rs` already rewrites `Tm` operands in the
  `FollowerDisposition::Reflow` path (`format.rs:674–677`), so the hook exists.
  **M-shear-glyph** (synthesize sheared outlines and embed) is rejected: it
  requires FF-C and would make a *fallback* more expensive than the real thing.
- **A third leak mechanism, named because §1.3's constraint list does not cover
  it.** A shear injected into `Tm` is **not** text state and is **not** fixed by
  §3.4's restore ladder. `Td`/`TD`/`T*` derive `Tlm` by *translation* from the
  previous `Tlm`, so they **preserve the shear** — it propagates to every
  subsequent line in the text object. The fix is different: the follower must be
  re-emitted with an **absolute `Tm`**, not a relative `Td`/`T*`. This is a
  distinct correctness obligation from the `Tc`/`Tw`/`Tz`/`Ts` one and gets its
  own acceptance test.
- **`Ts` × synthetic-italic interaction.** A horizontal shear offsets x by
  `y · tan θ`. At baseline (`y = 0`) that is zero, which is why the shear is
  normally invisible in the advance. **With a non-zero `Ts` it is not zero** —
  a raised, sheared run is displaced horizontally by `Trise · tan θ`. Named
  acceptance test; this is exactly the kind of interaction that ships as a
  "why is my superscript slightly to the right" bug.

**Persistence — P-selfevident, not P-private-key.** The catalog asks (rightly)
that a synthesized style be a detectable characteristic, not a transient hint.
pdfce achieves this **without inventing a private PDF extension**:

- **In-session:** a `StyleSynthesis::{None, SyntheticBold, SyntheticItalic,
  SyntheticBoldItalic}` provenance marker on the run, surfaced in the properties
  panel, in `pdfce-cli inspect`, and in the save-time report. This follows the
  established `FontProvenance::{Bundled, Supplied}` pattern
  (`addtext.rs:205–211`) — a disclosure enum that is *never written into the
  PDF*.
- **On reload:** the chosen emission mechanisms are **self-evident from the
  bytes**. A run showing `Tr 2` with a small stroke width, in a font whose
  `/BaseFont` does not say Bold, is detectably synthetic. A `Tm` with a nonzero
  `c` term on a font whose name does not say Italic is detectably obliqued.
  pdfce can therefore **re-detect its own synthesis (and other producers'!) by
  inspection**, with no marker written to the file.
- **P-private-key is rejected**: writing a private key into a document to record
  a rendering choice adds bytes to a file for pdfce's benefit, is invisible to
  every other consumer, and sits badly beside §5.6 "never normalize" and the
  minimal-diff posture. **P-session alone is rejected** as insufficient — it
  fails the catalog's re-open requirement.

This is a genuine, cheap lead: the catalog's evidence is that Acrobat's own
output self-discloses nothing and that third-party preflight (Enfocus/PitStop)
has to *infer* the artifact class after the fact. pdfce infers it too — but
inside the editor, at open time, for the operator, on its own and other
producers' files.

### 3.7 The minimal StructTree/ActualText update is **cut from FF-H**

**Decision: remove it from FF-H's payload entirely and re-file it as its own
backlog item (suggested name **FF-I**; librarian's call) under a future
accessibility Pass. R73 (disclose, never corrupt) is unchanged and remains in
force in the meantime.**

Decision 016 §2 already reached this conclusion on the merits — "StructTree
couples to a not-yet-built a11y subsystem… StructTree premature" — and then
left it inside FF-H's name anyway. That is how a formatting Pass acquires a
half-built structure-tree writer as a tail.

Three reasons: (1) it shares **nothing** with the rest of FF-H — different
objects (the structure tree, not the content stream), different invariant
(structure validity, not text-state scoping), different test corpus (tagged
documents); (2) a *partial* structure-tree writer is worse than none, because
R73's current honesty ("references stay valid; `/ActualText` and reading order
are stale, and here is the disclosure") is a **coherent, shippable posture**,
whereas "we update some structure sometimes" is not; (3) FF-H's own tagged
obligation is already fully specified by R73 and needs no new machinery — every
FF-H slice preserves the BDC/EMC + MCID wrapper and discloses staleness, exactly
as 14.1/14.2 already do.

### 3.8 Q3 — ordering: **FF-H → FF-C → FF-B** (O-H-C-B)

**Decision: FF-H first, FF-C second, FF-B last.**

**FF-H first**, for reasons that are about *dependency and risk*, not about
value (FF-H is the *least* valuable of the three on the catalog's own
`should_have` rating):

1. **It contains a correctness prerequisite for the other two.** Slice 19.0
   publishes the ambient text state on provenance, adds the missing `Ts`/`Tr`
   tracking, consolidates three private trackers into one, and closes the
   `reflow_apply` restore hole. **FF-B re-emits text across block and page
   boundaries** — strictly harder than within-block, and it inherits whatever
   ambient-state model exists when it is built. **FF-C re-encodes runs into
   newly-subset fonts** — also on top of that model. Building either on three
   divergent private trackers and a missing `Ts` arm compounds the problem
   across a much larger surface. 19.0 is cheap now and expensive later.
2. **It is the smallest and most-leveraged.** §1.2 established that
   `format.rs`'s `set_ops`/`restore_ops` pattern already *is* the mechanism —
   FF-H's core slice extends a shipped, tested emission path rather than
   building a subsystem. Decision 016 §2 said "spacing extends 14.2 cheaply"
   before this audit; the audit confirms it more strongly than 016 knew.
3. **It finishes a dimension.** After FF-H, the *formatting* dimension of text
   handling is complete. FF-B is the *layout* dimension and FF-C is the *font-
   writer* dimension; each is a large rock. Finishing the small dimension first
   gives the operator a completed capability rather than two half-built ones.
4. **Its acceptance tests are sharp and cheap** — emit → restore → assert
   following text unaffected is a fixture and an assertion, verifiable against
   the existing R59 render gate and R85 preview-equals-saved oracle.

**FF-C second, despite ranking higher on value** (decision 016 §2 ranked it #2
overall and called it the lifter of "the single most common wall in real
editing"). The MIT decision (2026-08-01) lifted its **rule-8** license gate —
but **not** rule 13: a font subsetter is still a new Cargo dependency whose
license must be classified, escalated if copyleft, and folded into a regenerated
`THIRD_PARTY_LICENSES.md` via `cargo-about`. That is a real gate with an
operator step in it (§10). It is the right *second* item: highest value, and by
then the text-state foundation is solid under it.

*Checked and dismissed:* one might argue FF-C should precede FF-H because
subsetting would reduce the need for synthetic bold/italic. **It would not.**
FF-C lets pdfce embed glyphs from a face it *has*; it does not conjure a Bold
face that is not installed. The synthesis gate ("no real Bold/Italic resource
resolves") and the FF-C gate ("the embedded subset lacks this glyph") are
independent failure modes with independent remedies. No ordering pressure
either way — which removes the only real argument for reordering.

**FF-B last.** Decision 016 §2's assessment stands and nothing has weakened it:
prestige-high, **daily-frequency-low**, and the largest new subsystem of the
three. It also benefits most from FF-H's foundation (see 1). Its status as "the
genuine exceed-Acrobat headline" is an argument about *marketing*, and this
project's ordering has consistently been driven by leverage and risk instead —
correctly.

**How much does Pass 18.6 actually help FF-H? Very little — roughly 5% — and it
must not be credited as groundwork.** The request's caution is right and the
audit confirms it precisely:

- `vector::decompose::GState` (`decompose.rs:1179–1204`) is **private**, not
  re-exported by `vector/mod.rs`, `Clone` but not `Copy`, and carries
  render-only baggage (`Arc<ExtractFont>`, stroke colour, line width). The
  public `TextObject` it feeds (`decompose.rs:601–624`) exposes **zero**
  text-state spacing fields. **Nothing in it is callable from an authoring
  path.**
- It is a *reading* walk that consumes these four values for one purpose —
  an approximate text bbox at `decompose.rs:2037–2101`. FF-H needs them for two
  different purposes: **restoration** (which additionally requires the raw
  operand bytes and the enclosing scope, which `GState` does not keep) and
  **emission** (which `GState` has no notion of at all).
- **What it genuinely contributes:** a correct, tested reference implementation
  of the Table-105 initial values (`GState::initial()`, L1206) and of the
  `Trm = [Tfs·Th 0 0 Tfs 0 Ts] × Tm × CTM` composition (L2100–2101) to copy
  semantics from — and, more valuably, it is the **third** private tracker,
  which makes the consolidation argument unarguable. It is *evidence for* slice
  19.0's existence, not *progress on* it.

---

## 4. Rationale (the reasoning behind the reasoning)

### 4.1 What Acrobat removing a feature is, and is not, evidence of

The request asks directly. The answer is different for the two operators, and
that difference is the spine of §3.2 vs §3.3.

**A vendor removing a feature during a UI consolidation is evidence about the
consolidation, and only weak evidence about the feature's value.** Isaacs states
the mechanism explicitly — text editing was consolidated into one tool, and the
new tool carried a subset. Features die in consolidations for reasons that have
nothing to do with usage: panel real estate, the new tool's internal model,
engineering budget, whoever owned the old code having left. Absent a stated
rationale, "removed in a consolidation" is close to uninformative about value.

**But there is a second, independent signal available for `Tw`, and it points
the same way.** `Tw`'s *technical* utility has been collapsing over the same
period for reasons entirely independent of Adobe's UI: producers moved to
Type0/Identity-H composite embedding as the default, and `Tw` is spec-void for
2-byte codes (§1.3.3). So Acrobat dropping `Tw` is **consistent with** a control
that had genuinely become inert on most new documents — the removal is
confounded, but the confound points *toward* low value, and there is a
first-principles spec reason to believe it. **That is why `Tw` gets a census
rather than a build.**

**No such second signal exists for `Ts`.** Baseline rise works identically on
every font model, on every document, today and in ten years. Its removal has no
technical excuse — it is pure consolidation collateral, and Adobe's own
Principal Scientist had to hand a user a luck-dependent hack in its place. **That
is why `Ts` gets built.**

The general rule this yields, worth carrying forward: *treat a competitor's
removal as evidence only when you can find a second, independent reason the
capability declined. Otherwise treat it as evidence about their product, not
about the need.*

### 4.2 Why "exceeding is legitimate but not free" resolves to a cost test, not a taste test

The request correctly notes pdfce already exceeds deliberately (`pdfce-cli` in
full, FF-B's headline) and correctly notes it isn't free. The way to make that
operational is to stop asking "should we exceed here?" and ask "**what is the
marginal cost of the exceed, given what the parity feature already forces us to
build?**"

- `Ts` free-form: parity **forces** the `Ts` emission/restore/tracking/test path
  (super/subscript has no other mechanism). Marginal cost of the exceed ≈ a
  numeric field + a CLI flag + two tests. **Exceed.**
- `Tw`: parity forces **nothing** — Acrobat has no control, and pdfce's own
  reflow layer deliberately avoids the operator. Every byte of a `Tw` control is
  net-new permanent surface: a UI affordance with conditional visibility, a
  composite gate, a disclosure string, undo, round-trip cases, CLI flags, docs,
  and a promise. **Do not exceed on an unmeasured premise.**

Same principle, opposite answers, because the costs differ by an order of
magnitude. This is the test to reuse next time.

### 4.3 Why the correctness slice comes before the feature slices

19.0 adds no operator-visible capability. It is still first, because every
FF-H feature depends on being able to *restore* an ambient value, and pdfce
today cannot observe two of the five relevant parameters (`Ts`, `Tr`) in the
authoring path at all (§1.2). A feature slice built first would have to invent a
local tracker — creating a **fourth** private copy of the same state, in the one
subsystem whose whole job is to write these operators correctly. Doing 19.0
first also closes the `reflow_apply` restore hole (`reflow_apply.rs:455`) while
it is still masked by a gate, rather than after a later slice relaxes that gate.

### 4.4 Why one synthesis policy with a different remedy order beats two policies

Two policies would be defensible only if the two paths had genuinely different
*obligations*. They do not — they have different *defaults* (R79's bundled
Standard-14 for new text vs. the document's own family for existing text), and
those defaults already produce the right behavioural difference on their own:
the gate rarely opens for Add-Text (Standard-14 has real bold and italic
variants) and opens regularly for in-place edit (an embedded subset may have no
bold sibling anywhere). Encoding that difference as *two policies* would
hard-code a consequence that already falls out of the data. Encoding it as
*remedy ordering* captures the one thing that genuinely differs — how disruptive
the alternative remedy is to that path's content — and nothing else.

### 4.5 Why self-evident emission beats a private marker

The catalog's requirement is that a synthesized style be detectable on re-open.
There are two ways: write something into the file, or choose mechanisms whose
bytes already say it. The second is strictly better here — it costs nothing,
adds no bytes, requires no private extension, does not sit awkwardly against
§5.6/never-normalize, and it generalizes: the same detector that recognizes
pdfce's own faux bold recognizes **Word's and InDesign's**, which is a genuinely
useful editor feature ("this run's bold is fake") that Acrobat leaves to
third-party preflight tools.

### 4.6 Why the ordering is leverage-driven rather than value-driven

FF-C is the highest-value of the three and is second. That is deliberate and
consistent with how this project has ordered work throughout (decision 010's
C → B → A; decision 016 choosing FF-D over the higher-ceiling FF-C). The reason
holds again: FF-C's gate has an operator step in it (rule 13 dependency
classification, §10), FF-H's does not, and FF-H removes shared foundation risk
from both of the others. Doing the cheap prerequisite first, then the expensive
high-value item, then the expensive prestige item is the sequence that keeps the
operator's "finish off all the text handling stuff" directive on a monotone
path to done.

---

## 5. Standing rules (binding; add to `ROADMAP.md` — librarian assigns final `Rnn`; **current ceiling R87**)

- **R88 — Direct text-state formatting is scoped by explicit restore-by-value,
  never by `q`/`Q`, never by normalization.** Any `Tc`/`Tw`/`Tz`/`Ts`/`Tr`
  pdfce emits to affect one run is followed by an explicit restore of the
  resolved ambient value, emitted **inside the same text object** — `q`/`Q` are
  "Special graphics state" operators and are **not permitted inside `BT…ET`**
  (ISO 32000-1 §8.2 Table 51/Figure 9), and splitting the text object to use
  them would discard `Tm` (§9.4.1). The ambient value is resolved by the
  three-tier ladder already established for fill colour
  (`TextColor::restore_bytes`): spec default when provably unset → observed raw
  operand bytes when set in the stream → **refuse-and-disclose when
  unobservable** (multi-stream `/Contents`, Form-XObject-inherited state,
  unparseable prefix). A guessed default restore is never emitted. New text
  authored inside a balanced `q … BT … ET … Q` envelope (the `addtext.rs` path)
  is exempt: `Q` performs the restore.
- **R89 — Size-relative typographic quantities are stored as ratios and derived
  at emit time.** `Tc` and `Ts` are in *unscaled text space units* and are **not**
  scaled by `Tfs` (§9.3). pdfce's model therefore stores these as a
  discriminated `Absolute | Relative` quantity; superscript/subscript are always
  `Relative`, and the absolute operand is **re-derived whenever `Tfs` changes**,
  so a size change never silently mis-scales a rise or a tracking value.
- **R90 — Synthetic bold/italic is per-use, declinable, fallback-only, and
  self-evident.** Offered only when no real Bold/Italic resource resolves (the
  same coverage check that gates a real family/style change), never as a global
  preference; presented as a named, declinable, per-application choice
  (deliberately stricter than Acrobat's set-and-forget "Enable Artificial
  Bold/Italic styles" preference); emitted by spec-native means — text rendering
  mode 2 with a **user-space-derived** stroke width and the **stroking colour
  matched to the fill** (§9.3.6), and a `Tm` shear for oblique — so the result
  is re-detectable by byte inspection on reload; recorded in-session as
  `StyleSynthesis` provenance; **never written into the PDF as a private
  marker**. One policy across both the in-place-edit (14.x) and add-text (16.x)
  paths; only the *order* in which the alternative remedy ("use a face with a
  real Bold") is offered differs, and that difference is disclosed. A `Tm` shear
  is **not** text state and is **not** covered by R88 — it propagates through
  `Td`/`TD`/`T*`, so followers are re-emitted with an absolute `Tm`.
- **R91 — `Tw` is capability-gated by font model, and inter-word distribution on
  composite runs is `TJ`-only.** Word spacing applies only to single-byte code
  32 (§9.3.3) and is structurally void for 2-byte composite/CID runs. pdfce
  never emits `Tw` on a composite run, and never presents a word-spacing
  affordance for one (R83). Slack/inter-word distribution for composite runs
  uses `TJ` numeric adjustments — the decision-015 path — never `Tw`.

---

## 6. Pass slicing (proposed **Pass 19.x**; librarian finalizes numbering)

Pass IDs 12.x–18.x are taken; 19.x is the next free family. `CommandKind`
requires **no new variant** — every slice below rides the existing
`FormatText` (`edit.rs:~305`) / `EditSession::format_text` (`edit.rs:1741`)
one-command-per-accepted-edit path, and `AddText` for the add-text half.

### 19.0 — Text-state consolidation + ambient publication (CORRECTNESS; core + CLI; **no new operator surface**)

**Scope.** Hoist one shared `TextState` type (`Tc`, `Tw`, `Th`, `Trise`,
`Tmode`, `TL`, `Tf`/`Tfs`) into a common `pdfce-core` module and retire the
three private copies (`text_extract::page::TextState`,
`text_edit::edit::Walk` + `reflow_apply::BlockTextState`,
`vector::decompose::GState` — the last by composition if a full swap is too
invasive; do not leave four). Add the **missing `b"Ts"` and `b"Tr"` arms** to
`text_edit::edit::Walk` (`edit.rs:~526–544`). Publish the ambient state on
`GlyphProvenance` (`text_extract/mod.rs:320–348`), including the **raw operand
bytes** needed for a byte-faithful restore. Publish a run-level composite flag
(promote `ExtractFont::is_simple`/`CodeWidth` from `pub(crate)`, or expose an
equivalent public accessor on the run/provenance). Implement the R88 ambient
ladder including the **unobservable → refuse-and-disclose** tier (multi-stream
`/Contents` walk; Form-XObject-inherited state). **Close the
`reflow_apply.rs:455` restore hole**, keeping the justify gate (L397–405) in
place. `pdfce-cli inspect --text-state`.

**Acceptance.**
- A fixture with `0.5 Tc` / `90 Tz` ambient around an edited run: follower
  positions unchanged vs. the pre-edit render (R59 gate) and preview == saved
  (R85 oracle).
- A fixture with ambient `Ts` around an edited run: the rise survives the edit
  byte-faithfully (this **fails today** — it is the regression this slice
  fixes).
- Reflow of a block followed by unrelated text in the same stream: the
  following text's rendered spacing is unaffected (closes the L455 hole).
- Page whose `/Contents` is a 3-element array with `Tc` set in element 1 and the
  edited run in element 3: ambient resolved correctly across the concatenation.
- A run inside a Form XObject: **refused and disclosed by name**, not silently
  restored to `0`.
- Exactly one definition of the text-state parameters remains reachable from the
  text pipeline; `text_extract::font::advance_tx`'s "three copies" comment
  (`font.rs:314–323`) updated or retired.
- `cargo tree -p pdfce-core` / `-p pdfce-render`: zero egui/eframe/winit/wgpu/
  glow. Round-trip/R46 green. All Pass 14.x/15.x/16.x tests unchanged.
  `cargo fmt --check` + `cargo clippy -- -D warnings` clean.

**Non-goals.** Any operator-facing spacing control; any synthesis; any UI.

### 19.1 — `Tc` + `Tz` + superscript/subscript authoring (core + CLI) — the Acrobat-parity slice

**Scope.** Extend `format.rs`'s `set_ops`/`restore_ops` (`format.rs:627–640`)
with `Tc`, `Tz`, and `Ts`-for-super/subscript (the latter composing with the
existing `push_tf` size change). `MetricSpec::{Absolute, Relative}` per R89 with
re-derivation on size change. Mirror into `addtext.rs` at the `q…Q`-scoped
insertion points (`addtext.rs:866`, `addtext.rs:1490`) — no restore needed
there. Disclose the `Tz` × justify interaction: changing `Th` rescales every
`TJ` adjustment (§9.3.4), so a `Tz` change on a 15.1-justified line invalidates
its slack — **disclose and offer re-justify**, never silently leave it wrong.
CLI: `pdfce-cli format-text --char-spacing <v[unit]> --h-scale <pct>
--superscript | --subscript`.

**Acceptance.**
- **The leak test (the catalog's named `must_have`):** a fixture with unrelated
  text immediately following a spacing-formatted run — the following text's
  rendered spacing is provably unaffected.
- Restore emits the *observed raw operand bytes* where the ambient was set, the
  spec default where provably unset, and refuses where unobservable.
- Size-change re-derivation: superscript at 10 pt, resized to 20 pt, still sits
  at the same *proportional* height (R89).
- `Tz` change on a justified line: disclosed, with a re-justify offer.
- Tagged run: BDC/EMC + MCID wrapper preserved, staleness disclosed (R73).
- Only the edited content-stream object re-emitted (R32/R46/R69);
  incremental-save-safe (R34/R70); undo restores the byte-identical pre-format
  stream.
- Gates green; fmt/clippy clean.

**Non-goals.** `Tw` (19.4, gated); synthesis (19.2); UI (19.3); StructTree
(cut, §3.7).

### 19.2 — Free-form `Ts` + synthetic bold/italic (core + CLI) — the deliberate exceed

**Prerequisite check before starting:** confirm `pdfce-render` honours **text
rendering mode 2** (fill-then-stroke) and a sheared `Tm`. If it does not,
**preview ≠ saved** and R85 is violated the moment synthesis ships — fix the
renderer first or descope synthesis from this slice.

**Scope.** Free-form numeric `Ts` (CLI `--rise`, `Absolute` by default per
§3.2). `StyleSynthesis` provenance enum. The R90 gate wired behind
`resolve_target_resource`'s failure (`format.rs:861–894`), per-use and
declinable, one shared type used by both `format.rs` and `addtext.rs`, with the
§3.6 remedy-ordering difference. Mode-2 emission with user-space-derived stroke
width and stroking-colour match/restore; `Tm` shear for oblique with **absolute
`Tm` re-emission for followers**. Reload-time re-detection of synthesized styles
(pdfce's own and other producers'). CLI:
`pdfce-cli format-text --rise <v> --bold-synthetic --italic-synthetic`.

**Acceptance** (the catalog's four named cases, plus four this decision adds):
- (a) selection with a real resolvable Bold/Italic resource → the Pass-14.2
  real-resource path is taken, **no synthesis offered**;
- (b) selection with no real Bold/Italic resource anywhere resolvable →
  synthesis **offered** as a named declinable choice, **never auto-applied**;
- (c) operator declines → refuse-and-disclose, no partial/silent application;
- (d) synthesized run on a tagged document → MCID wrapper preserved, staleness
  disclosed, **explicitly asserted not to reproduce Acrobat's documented
  tag-corruption defect** for this edit type;
- (e) faux bold on **coloured** text → outlines are the text's colour, not
  black (stroking colour matched); stroking colour restored afterward;
- (f) faux bold at 10 pt and 72 pt → visually consistent weight (stroke width
  derived in user space, §9.3.6);
- (g) **`Ts` × synthetic-italic**: a raised sheared run is displaced by
  `Trise · tan θ` — asserted correct, and followers unaffected by the shear
  (absolute `Tm` re-emission);
- (h) save → reload → the synthesized run is **re-detected and labelled**
  without any private marker having been written; round-trip of the untouched
  objects still byte-identical.

### 19.3 — GUI: the spacing/style property surface (gui)

**DISPATCH `pdfce-ui-specialist` first.** Property-bar/panel controls for
character spacing, horizontal scaling, baseline rise, superscript/subscript, and
the synthesis offer. **R83 capability-gating** (no affordance without the
capability — this is where the published composite flag from 19.0 is consumed).
Unit labels that state which unit is in force (em/1000 vs pt) and which mode
(`Absolute`/`Relative`) a value is stored in. A "synthesized" badge on runs
carrying `StyleSynthesis`. Verbatim disclosure strips, as 14.3 already does.
**The capability query ("is this run composite?", "does a real Bold resolve?")
must be a `pdfce-core` API the GUI calls — never logic reimplemented in
`pdfce-gui`**, or the WASM fork loses it (R74, GUI-core separation).

### 19.4 — `Tw` (**CONDITIONAL — do not start without the census**)

Run the §3.3 census first and record both numbers in `ROADMAP.md`. Then:
(a) ≥ 60% → build as an R83-gated, simple-font-only control with a
refuse-and-disclose engine gate (R91); (b) ≤ 25% → **close the item**, recording
the census as the reason; (c) 25–60% → **escalate to Ken** (§10).

### Explicitly cut from FF-H

**Minimal StructTree/ActualText update** → re-filed as its own backlog item
(suggested **FF-I**) under a future accessibility Pass; R73 unchanged (§3.7).

---

## 7. Fast-follows beyond FF-H

`Tw` authoring if the census supports it (19.4) · FF-C font subsetting/glyph
embedding (next in the §3.8 order; rule-13 dependency classification first) ·
FF-B cross-block/cross-page reflow · FF-I minimal StructTree/`/ActualText`
update inside a real accessibility Pass · kerning-pair-level control (Isaacs
lists "kerning" among Acrobat's retained controls; pdfce currently has no
kerning surface distinct from `Tc` — **an unexamined parity gap surfaced by
this decision, not scoped here**) · leading (`TL`) as a direct formatting
control distinct from reflow's leading input · composite-font inter-word
distribution via `TJ` as an operator-facing control (FF-E-adjacent).

---

## 8. Honest limits (named up front)

No word-spacing authoring control ships in 19.0–19.3, and it may never ship
(census-gated) — existing `Tw` is preserved and displayed, not editable ·
synthesis is a *fallback*, never a general "make anything bold" button, and its
output is visually distinguishable from a real drawn face (uneven stroke
contrast on emboldened glyphs; naive shear geometry vs. a true italic) — this is
disclosed, not hidden · synthesized-style detection on reload is heuristic
(pattern-matching the bytes), so a producer using a different synthesis
technique may go undetected · `Ts`/`Tc` `Relative` mode re-derives on size
change, which is correct but *is* a behavioural difference an operator must be
told about · the `Tz` × justify interaction is disclosed, not auto-repaired ·
Form-XObject-hosted runs are refused for direct text-state formatting rather
than guessed · no structure-tree update (R73's disclose-never-corrupt posture
continues unchanged) · Acrobat's own units/ranges for character spacing and
horizontal scaling remain an unclosed catalog GAP, so pdfce's units are pdfce's
own documented choice and **not** a parity claim.

---

## 9. Where pdfce exceeds Acrobat (FF-H specifically)

Free-form numeric baseline rise (Acrobat removed it; its own former Principal
Scientist offers only a luck-dependent workaround) · per-use, named, declinable
synthesis disclosure (Acrobat: a global set-and-forget preference, no per-use
disclosure found) · synthesized styles recorded as provenance **and**
re-detectable on reload, including on *other* producers' files (Acrobat's output
self-discloses nothing; third-party preflight has to infer it) · a documented,
size-relative model for `Tc`/`Ts` that survives a font-size change (Acrobat's
behaviour here is unsourced and the underlying operators make the naive
implementation wrong) · explicit state-scoping with a disclosed refusal when the
ambient is unobservable · tag preservation with disclosed staleness (R73, vs.
Acrobat's documented structure-tree corruption on exactly this class of
character-style change) · a first-class scriptable CLI for every one of these
(Acrobat has none).

---

## 10. For Ken personally (do not decide these solo)

1. **The `Tw` census middle band.** If simple-font reachability lands between
   25% and 60%, the call is "is a control that works on roughly half of
   documents worth permanent surface area?" — a product judgement, not a
   technical one. The census will be run and reported; the decision in that band
   is yours.
2. **FF-C's rule-13 dependency classification.** The MIT decision lifted the
   **rule-8** license-undecided gate; it did **not** pre-approve any dependency.
   A font subsetter must still be classified, escalated to you if copyleft, and
   folded into a regenerated `THIRD_PARTY_LICENSES.md`. "MIT decided" ≠ "any
   crate is fine." Flagged because FF-C is next in the §3.8 order.
3. **Cutting StructTree/`/ActualText` out of FF-H (§3.7).** This is a scoping
   call I have made, but it changes the shape of a named backlog item you may
   have been counting as part of "finish off all the text handling stuff." If
   you consider a minimal structure-tree update part of that directive, say so
   and it gets its own Pass rather than being dropped — it should still not ride
   inside a formatting Pass.
4. **List-authoring remains separately gated and unanswered.** `ROADMAP.md`
   already records that "finish off all the text handling stuff" does **not**
   resolve it. Re-surfaced here only so it is not silently assumed either way.
5. **Kerning (§7).** Isaacs lists kerning among Acrobat's retained controls.
   pdfce has no kerning surface distinct from `Tc`. That is a parity gap this
   decision *found* but did not scope, and it may or may not fall inside your
   "finish off all the text handling" intent.

---

## 11. What this decision explicitly does NOT decide

- **Whether `Tw` authoring is ever built** — census-gated (§3.3, §6 slice 19.4).
- **The exact spacing/style interaction UX** — `pdfce-ui-specialist`'s call at
  19.3.
- **FF-C's and FF-B's internal scope** — ordered here (§3.8), not scoped.
- **The StructTree/`/ActualText` update's design** — cut from FF-H (§3.7); it
  needs its own decision inside an accessibility Pass.
- **Kerning, leading-as-direct-formatting, composite inter-word `TJ` as an
  operator control** — named fast-follows (§7), unscoped.
- **Any change to R35 (redaction full rewrite), R70 (edit is incremental), R73
  (tagged edits disclose), R75 (reflow is opt-in), or the Pass-6.2 FreeText
  path** — all untouched.
- **Composite/CJK/RTL text handling** — FF-E/FF-F, unchanged.

---

## 12. References

**Code** (paths relative to `crates/`):
`pdfce-core/src/text_edit/format.rs` (`plan_format` L535; `set_ops`/
`restore_ops` L618–659; `push_tf` L935; `push_fill` L948; `resolve_target_resource`
L861–894; `fill_narrowed` L639/L722; advance inputs L610/L614) ·
`pdfce-core/src/text_edit/edit.rs` (`Walk` L420–439; operator arms L526/L532/L538
— **no `Ts`/`Tr`**; `ShowData` L382–396; `TextColor::restore_bytes` L172;
`classify_font` L1069; `CommandKind` L221; `EditSession::format_text` L1741) ·
`pdfce-core/src/text_edit/reflow_apply.rs` (justify gate L397–405; preamble
L411–432; `emit_justified_line` L1035–1071; **unrestored `ET` L455**;
`BlockTextState` L796–802) ·
`pdfce-core/src/text_edit/addtext.rs` (`build_content` L849–891;
`build_content_boxed` L1475–1518; insertion points L866/L1490; `FontProvenance`
L205–211) ·
`pdfce-core/src/text_extract/mod.rs` (`GlyphProvenance` L320–348) ·
`pdfce-core/src/text_extract/page.rs` (private `TextState` L199–242; operator
arms L408–428) ·
`pdfce-core/src/text_extract/font.rs` (`is_simple` L724–726; `CodeWidth`
L208–216; `advance_tx` + the "three copies" comment L314–329) ·
`pdfce-core/src/vector/decompose.rs` (private `GState` L1179–1204; `initial()`
L1206; `Trm` composition L2100–2101) ·
`pdfce-cli/src/main.rs` (`FormatText` L1262; `AddText` L1371; `Reflow` L1318).

**Spec** (`D:\Dev\Rag-Specialized\PDF_Spec\iso32000\`): `iso32000__s__9.3.md`
(Table 104/105; the scope rule "retained across text objects… initialized at the
beginning of each page"; §9.3.3 single-byte-code-32; §9.3.4 `Th` scales `Tc`/
`Tw`/`TJ`; §9.3.6 stroking colour + user-space line width; §9.3.7 rise; the
"`Tw` over-count + LEAK hazards" note) · `iso32000__s__9.4.md` (§9.4.1 Table 107
— `BT` resets only `Tm`/`Tlm`; `ET` discards `Tm`; §9.4.4 the `tx` advance
formula and the `Trm` composition) · `iso32000__s__8.2.md` (**Table 51 —
`q`/`Q`/`cm` are "Special graphics state"; Figure 9 — not admitted inside a text
object**) · `iso32000__ref__reflow_emission.md` (the shipped 15.1 `TJ` slack
machinery and its `Tw` rejection).

**Parity reference** (`D:\Dev\Rag-Specialized\Acrobat_Features\`):
`text_edit__spacing_and_scaling_controls.md` (the `Tw`/baseline-offset removal;
Isaacs' retained-controls list; the composite gate and leak `must_have`s) ·
`text_edit__synthetic_bold_italic_styles.md` (the "Enable Artificial Bold/Italic
styles" preference; the Enfocus double-strike mechanism; the four named test
cases; the Add-Text-vs-edit GAP this decision resolves) ·
`text_edit__font_family_style_change_on_format.md` ·
`text_edit__accessibility_tag_structure_interaction.md` ·
`text_edit__formatting_options.md`.

**Decisions/rules:** 014 (§5.3 ladder — amended here) · 015 (`TJ`-over-`Tw`
precedent, §3.1) · 016 (§2 prioritization — superseded by the operator directive,
reasoning upheld; §10 operator-owned items) · `ARCHITECTURE.md` §5 (round-trip),
§5.6 (never normalize), §5.11 (text editing is incremental surgery), §11 (edit
model) · `ROADMAP.md` R20, R32, R34, R36, R46, R47, R59, R69–R79, R83, R85, R86,
R87 · project rules 2 (GUI-core separation), 3 (round-trip), 4 (fuzzy never
sneaky), 6 (documentation-first), 10 (style/API guidelines), 11 (CLI parity),
13 (dependency licensing).

---

## Appendix A — JSON decision block (drives implementation)

```json
{
  "decision_id": "019-ffh-spacing-scaling-synthetic-styles",
  "title": "FF-H re-scoped: Tc/Tz/super-subscript at parity, free-form Ts as the deliberate exceed, Tw demoted to an evidence-gated conditional slice, synthetic bold/italic as one per-use declinable self-evident policy across both authoring paths, and the StructTree update cut out entirely",
  "status": "Decided",
  "date": "2026-08-03",
  "confidence": "high (Q1 split, Q2 policy + mechanism, Q3 ordering, the scoping mechanism); medium (the Tw census thresholds — deliberately a measurement, not a judgement; pdfce's chosen super/subscript and synthesis constants, which are documented defaults not parity claims)",
  "requested_by": "pdfce-engineer",
  "decided_by": "KenAgent (autonomous-builder, decision-consultant mode)",
  "amends": {
    "014": "§5.3 — FF-H payload re-scoped: Tw demoted to conditional; minimal StructTree/ActualText update REMOVED from FF-H and re-filed as its own backlog item (suggested FF-I).",
    "016": "§2 rank-4 'DEFER' verdict superseded by the operator's priority-#3 directive; §2's reasoning upheld and its 'StructTree piece is premature' finding acted on."
  },
  "one_line": "Scope FF-H by FONT-MODEL UNIVERSALITY and MARGINAL COST rather than by parity-vs-exceed: ship Tc + Tz + a superscript/subscript toggle (Acrobat's actual retained set) PLUS free-form Ts (the one deliberate exceed, near-free because the toggle forces the Ts emission path anyway); do NOT ship a Tw authoring control in the core slices (its only real job already belongs to the 15.1 reflow/justify layer which deliberately chose TJ over Tw, it is spec-void for composite runs, and its value is unmeasured) — instead make Tw first-class in the ENGINE and gate any control behind a corpus census with a 25%/60% decision band that escalates to Ken in the middle; scope every emitted operator by explicit RESTORE-BY-VALUE inside the text object (q/Q is ILLEGAL inside BT..ET per §8.2 Table 51) reusing the shipped format.rs set_ops/restore_ops pattern and the TextColor::restore_bytes ambient ladder, refusing-and-disclosing when the ambient is unobservable; make synthetic bold/italic ONE per-use declinable fallback-only policy shared by the 14.x and 16.x paths (differing only in the ORDER the alternative remedy is offered), emitted by spec-native Tr-2-plus-stroke and Tm-shear so it is re-detectable on reload with NO private PDF marker; CUT the StructTree/ActualText update out of FF-H entirely; and order the remaining text work FF-H -> FF-C -> FF-B because FF-H's slice 19.0 is a shared correctness prerequisite for the other two.",
  "answers": {
    "q1_tw_and_ts": {
      "verdict": "A-split — Tw and Ts get OPPOSITE answers; they are not a pair.",
      "ts_free_form": "SHIP as a genuine authoring control (deliberate exceed). Rationale: (1) the superscript/subscript toggle is a parity must-have and Ts is the only PDF-native baseline mechanism, so the emission/restore/tracking/test path is FORCED anyway — marginal cost of exposing the number is a field + a CLI flag + two tests; (2) Ts is font-model-agnostic (a Trm translation, §9.4.4) with no void case and no refusal class; (3) Acrobat's removal is pure consolidation collateral with no technical confound, and Isaacs handing users a luck-dependent hack in its place is evidence the NEED survived the feature; (4) the real use cases (aligning to a scanned baseline, nudging a mis-placed footnote marker, math/chemistry runs) cannot be expressed by a two-position toggle.",
      "tw": "DO NOT ship as an authoring control in slices 19.0-19.3. Engine-first only (read, tracked, published on provenance, honoured in advance math, preserved byte-verbatim, displayed READ-ONLY with an explanation). Any control is gated behind a corpus census (19.4). Rationale: (1) its only honest use case is inter-word distribution, which the 15.1 reflow layer already owns and does better via TJ — decision 015 rejected Tw as an IMPLEMENTATION for exactly the reasons that make it a bad CONTROL (hits every code-32 including leading/trailing/doubled spaces; leaks as graphics state; void on composite); adding it back as a feature re-introduces what 015 rejected, and creates two operator-facing dials that both look like 'space between words' and interact multiplicatively through Th; (2) its availability is decided by an invisible font property (2-byte codes) on a share of the corpus that is large and GROWING as producers default to Type0/Identity-H even for Latin text; (3) that share has never been measured, and this project settles exactly this class of question with a census (the 0.67% /Encrypt precedent), not a guess.",
      "shipped_set": ["Tc", "Tz", "superscript/subscript toggle", "Ts (free-form)"],
      "engine_only_set": ["Tw"]
    },
    "q1_evidence_reading": "Acrobat removing a feature during a UI consolidation is strong evidence about the CONSOLIDATION and only weak evidence about the feature's value — Isaacs states the mechanism explicitly and gives no usage rationale. BUT for Tw a SECOND, INDEPENDENT signal points the same way: Tw's technical utility collapsed over the same period as composite/Identity-H embedding became the producer default, and Tw is spec-void for 2-byte codes. So the Tw removal is confounded in a direction that supports low value, and there is a first-principles reason to believe it — hence a census. NO such second signal exists for Ts (it works identically on every font model, forever), so its removal is uninformative about need — hence build it. General rule to carry forward: treat a competitor's removal as evidence only when a second, independent reason for the decline can be found.",
    "q1_roundtrip_framing": "CONFIRMED CORRECT — preservation of pre-existing Tw/Ts is mandatory under R32/R46/R69 regardless, and is rightly out of scope for Q1. One amplification: preservation has THREE obligations, not one. (1) byte-verbatim survival of untouched operators — AUTOMATIC, no work. (2) semantic survival across a rewrite (the restore ladder) — real work, §3.4. (3) ambient state as an INPUT to the surgery's own §9.4.4 advance arithmetic — AUDITED: pdfce already gets this right for Tc/Tw/Th (format.rs:610/614 + edit.rs Walk), but tracks NEITHER Ts NOR Tr in the authoring path (no b\"Ts\"/b\"Tr\" arm in edit.rs Walk; GlyphProvenance carries none of the four). Ts does not enter the advance formula so this is a RESTORE gap not a positioning corruption — but it blocks the Ts control outright, and it is the content of slice 19.0.",
    "q2_synthetic_styles": {
      "policy": "IDENTICAL across Add-Text (16.x) and in-place edit (14.x) — one shared pdfce-core type, one gate, one disclosure wording, one declinability, one provenance marker. Both paths already share decision 012's GlyphSource + R79's face policy upstream, so sharing the fallback is the consistent choice, and the catalog resolves nothing about Acrobat here so there is no parity cost to coherence.",
      "deliberate_asymmetry": "Only the ORDER of offered remedies differs, and it is EMERGENT rather than special-cased. Add-Text owes nothing to an existing family and defaults to a bundled Standard-14 face (R79) whose family HAS real Bold/Italic variants — so the gate rarely opens there, and when it does the better first offer is 'use a face with a real Bold', synthesis second. In-place edit is bound to the document's own typography where changing family is the MORE disruptive action — so synthesis is offered first, family-change second. Both remedies on both paths, both declinable, ordering disclosed.",
      "gate": "Fallback-only: offered strictly after resolve_target_resource (format.rs:861-894) fails to find a real Bold/Italic resource. Never a global preference — deliberately stricter than Acrobat's set-and-forget 'Enable Artificial Bold/Italic styles' Content-Editing preference, per rule 4.",
      "mechanism_bold": "Text rendering mode 2 (fill-then-stroke, §9.3.6) with stroke width ~0.022 x effective rendered size. Rejected double-strike (the Enfocus-documented industry technique) because it doubles glyph count, changes extraction output, and breaks the byte<->glyph correspondence provenance and hit-testing depend on. TWO SPEC GOTCHAS THAT ARE BUGS IF MISSED: stroking uses the STROKING colour so it must be matched to the fill and restored (else coloured text gets black outlines); and line width for stroked text is in USER space so it must be derived from Tfs x |Tm scale| x |CTM scale|, never a constant. Tr is text state -> rides the R88 restore ladder, and Tr has NO arm in Walk today.",
      "mechanism_italic": "A ~12 degree shear (tan ~0.2126) premultiplied into the run's Tm; format.rs already rewrites Tm operands at L674-677. Rejected embedding sheared outlines (needs FF-C; makes a fallback costlier than the real thing). CRITICAL AND NOT COVERED BY THE STATED CONSTRAINTS: a Tm shear is NOT text state and is NOT fixed by the Tc/Tw/Tz/Ts restore ladder — Td/TD/T* derive Tlm by translation and PRESERVE the shear, so it propagates to every later line in the text object. Fix: re-emit followers with an ABSOLUTE Tm. Separate acceptance test.",
      "interaction_bug_named": "Ts x synthetic-italic: a horizontal shear offsets x by y*tan(theta), which is zero at baseline but NON-zero for a raised run — a rise'd sheared run is displaced by Trise*tan(theta). Named acceptance test (g).",
      "persistence": "P-selfevident, NOT P-private-key and NOT P-session-only. In-session: a StyleSynthesis::{None,SyntheticBold,SyntheticItalic,SyntheticBoldItalic} provenance marker following the existing FontProvenance::{Bundled,Supplied} pattern (a disclosure enum never written to the PDF). On reload: the chosen mechanisms are self-evident from the bytes (Tr 2 + small stroke on a non-Bold BaseFont; a nonzero Tm 'c' term on a non-Italic font), so pdfce re-detects its OWN synthesis — and other producers' — by inspection, with nothing written into the file. Beats Acrobat, whose output self-discloses nothing and where third-party preflight must infer the artifact class after the fact.",
      "render_prerequisite": "BEFORE 19.2: verify pdfce-render honours text rendering mode 2 and a sheared Tm. If it does not, preview != saved and R85 is violated the moment synthesis ships — fix the renderer first or descope synthesis from the slice."
    },
    "q3_order": {
      "verdict": "FF-H -> FF-C -> FF-B (O-H-C-B).",
      "ff_h_first_because": [
        "Slice 19.0 is a shared CORRECTNESS PREREQUISITE for both others: it publishes ambient text state on provenance, adds the missing Ts/Tr tracking, consolidates THREE private trackers into one, and closes the reflow_apply.rs:455 restore hole. FF-B re-emits text across block/page boundaries and FF-C re-encodes into newly-subset fonts — both inherit whatever state model exists when built. Cheap now, expensive later.",
        "It is the smallest and most-leveraged: format.rs's set_ops/restore_ops pattern ALREADY IS the mechanism, so the core slice extends a shipped tested path rather than building a subsystem. The audit confirms decision 016's 'spacing extends 14.2 cheaply' more strongly than 016 knew.",
        "It FINISHES the formatting dimension, leaving FF-B (layout) and FF-C (font writer) as one large rock each rather than two half-built ones.",
        "Its acceptance tests are sharp and cheap and verifiable against the existing R59 render gate and R85 preview-equals-saved oracle."
      ],
      "ff_c_second_because": "Highest value of the three (decision 016 ranked it #2 overall; it lifts the most common real editing wall), but the MIT decision lifted only its RULE-8 gate — rule 13 still requires dependency classification, possible copyleft escalation to Ken, and a cargo-about regeneration. Right second item: by then the text-state foundation is solid under it. CHECKED AND DISMISSED: FF-C does NOT reduce the need for synthetic bold/italic — FF-C embeds glyphs from a face you HAVE, it does not conjure an uninstalled Bold face; the two gates are independent failure modes with independent remedies, so there is no ordering pressure either way.",
      "ff_b_last_because": "Decision 016 §2's assessment stands unweakened: prestige-high, daily-frequency-LOW, largest new subsystem, and the biggest beneficiary of FF-H's foundation. Its 'exceed-Acrobat headline' status is a marketing argument; this project's ordering has consistently been leverage-and-risk driven instead, correctly.",
      "pass_18_6_credit": "~5%, and it must NOT be treated as groundwork. vector::decompose::GState (decompose.rs:1179-1204) is PRIVATE, not re-exported, Clone-not-Copy, carries render-only baggage (Arc<ExtractFont>, stroke colour, line width), and the public TextObject it feeds exposes ZERO text-state spacing fields — nothing in it is callable from an authoring path. It is a READING walk consuming these four values for ONE purpose (an approximate text bbox at L2037-2101); FF-H needs them for restoration (which additionally needs raw operand bytes and enclosing scope, which GState does not keep) and emission (which GState has no notion of). What it GENUINELY contributes: a correct tested reference for the Table-105 initials (GState::initial(), L1206) and the Trm composition (L2100-2101) to copy semantics from — and, more valuably, being the THIRD private tracker it makes the consolidation argument unarguable. It is EVIDENCE FOR slice 19.0's existence, not PROGRESS ON it."
    }
  },
  "scoping_mechanism": {
    "chosen": "S-restore — explicit restore-by-value emitted INSIDE the same text object, extending format.rs's existing set_ops/restore_ops pattern (L618-659).",
    "q_slash_Q_rejected": "ILLEGAL, not merely inferior: q/Q/cm are 'Special graphics state' operators and are NOT admitted inside a text object (ISO 32000-1 §8.2 Table 51 + Figure 9). Splitting the text object to use them (… ET q … BT …) would DISCARD Tm (§9.4.1) and force absolute re-positioning downstream, destroying minimal-diff. Recorded explicitly because 'just wrap it in q/Q' is the first thing any implementer will try.",
    "normalize_rejected": "ARCHITECTURE.md §5.6 (never normalize) + would rewrite objects pdfce did not logically touch (R32/R46).",
    "tj_emulation_rejected_as_general": "Can express Tc and inter-word spacing, but CANNOT express Tz (which changes glyph SHAPE, §9.3.4) or Ts (a Trm translation). Also bloats the stream and destroys semantic legibility. Retained ONLY as the composite-font fallback for inter-word distribution, where decision 015 already put it.",
    "ambient_ladder": [
      "1. Provably at the Table-105 default and never set in the prefix -> restore the SPEC DEFAULT.",
      "2. Set by an observed operator -> restore the OBSERVED RAW OPERAND BYTES (so 0.5000 Tc is not silently renormalized to 0.5 Tc), mirroring TextColor::Device{raw}|Other{raw}.",
      "3. UNOBSERVABLE -> REFUSE AND DISCLOSE BY NAME. Two real cases: a /Contents ARRAY where state was set in an earlier element (the walk must span the concatenation), and a run inside a FORM XOBJECT which inherits text state from its invoking context and has no in-stream ambient at all. Emitting a guessed '0 Tc' restore would silently change content pdfce did not touch — the rule-4 failure.",
      "4. Where a restore narrows semantics, DISCLOSE, exactly as fill_narrowed/disclosure_narrowing already does (format.rs:639, L722)."
    ],
    "addtext_exemption": "addtext.rs emits inside a balanced q…Q OUTSIDE its BT…ET (L849-891, L1475-1518), so new text has NO restore obligation — Q does it. Insertion points addtext.rs:866 and addtext.rs:1490. Same operator names, two correct mechanisms, both documented so a future maintainer does not 'unify' them into a bug.",
    "existing_leak_to_close": "reflow_apply.rs:455 terminates at ET with no restore after conditionally emitting Tc/Tz/Tw (L411-432). Benign TODAY only because of the justify gate at L397-405 and because the non-justify path re-emits values equal to ambient. Slice 19.0 closes it WITH the gate still in place; 19.1 may then relax the gate BECAUSE it is closed."
  },
  "units_model": {
    "rule": "R89 — store size-relative typographic quantities as RATIOS, derive the operand at emit time. Tc and Ts are in UNSCALED TEXT SPACE UNITS and are NOT scaled by Tfs (§9.3), which produces a real trap: a superscript applied at 10pt and resized to 20pt keeps its absolute rise and lands wrong.",
    "type": "MetricSpec::{Absolute(f32), Relative(f32)} — discriminated, operator-visible.",
    "tc": "GUI default presentation em/1000 (Relative), matching typographic tracking and TJ's own unit space; an absolute point value is accepted and stored as Absolute.",
    "ts_free_form": "Absolute by default (what the operator typed is what they get); opt-in to Relative.",
    "super_subscript": "ALWAYS Relative — re-derived on every Tfs change.",
    "tz": "No discrimination needed — a dimensionless percentage (§9.3.4). 100 = normal.",
    "pdfce_defaults_not_parity_claims": {"size_factor": 0.60, "superscript_rise": "+0.34 x Tfs", "subscript_rise": "-0.18 x Tfs", "synthetic_bold_stroke": "~0.022 x effective rendered size (user space)", "synthetic_italic_shear_deg": 12}
  },
  "cut_from_ff_h": {
    "item": "minimal StructTree / ActualText update",
    "action": "REMOVED from FF-H; re-filed as its own backlog item (suggested FF-I) under a future accessibility Pass. R73 (disclose-never-corrupt) unchanged and in force meanwhile.",
    "why": "Decision 016 §2 already reached this on the merits ('StructTree couples to a not-yet-built a11y subsystem… premature') and then left it inside FF-H's NAME anyway. It shares NOTHING with the rest of FF-H (different objects, different invariant, different corpus); a PARTIAL structure-tree writer is worse than none because R73's current posture is coherent and shippable while 'we update some structure sometimes' is not; and FF-H's own tagged obligation is already fully specified by R73."
  },
  "new_standing_rules": {
    "R88": "Direct text-state formatting is scoped by explicit restore-by-value, never by q/Q (ILLEGAL inside BT..ET per §8.2 Table 51/Figure 9), never by normalization; ambient resolved by the three-tier TextColor::restore_bytes ladder with REFUSE-AND-DISCLOSE when unobservable (multi-stream /Contents, Form-XObject-inherited state); a guessed default restore is never emitted; the balanced q…BT…ET…Q addtext envelope is exempt.",
    "R89": "Size-relative typographic quantities are stored as ratios and derived at emit time — Tc and Ts are unscaled text space units (§9.3), so super/subscript are always Relative and the operand is re-derived whenever Tfs changes.",
    "R90": "Synthetic bold/italic is per-use, declinable, fallback-only, and self-evident — one shared policy across the 14.x and 16.x paths (only the alternative-remedy ORDER differs, and it is disclosed); emitted by Tr 2 with a USER-SPACE-derived stroke width and the STROKING COLOUR MATCHED TO THE FILL (§9.3.6), and a Tm shear for oblique; re-detectable by byte inspection on reload; recorded as StyleSynthesis provenance; NEVER written into the PDF as a private marker. A Tm shear is NOT text state and is NOT covered by R88 — it propagates through Td/TD/T*, so followers are re-emitted with an absolute Tm.",
    "R91": "Tw is capability-gated by font model — void for 2-byte composite/CID runs (§9.3.3); pdfce never emits Tw on a composite run and never presents a word-spacing affordance for one (R83); composite inter-word distribution is TJ-only (the decision-015 path)."
  },
  "librarian_ceiling_note": "Current highest standing rule is R87; assign R88-R91. Pass family proposed 19.x (12.x-18.x are taken). Mark decision 014 §5.3 amended (FF-H re-scoped; StructTree cut) and decision 016 §2 superseded-by-directive-but-reasoning-upheld. CommandKind needs NO new variant — FF-H rides the existing FormatText/AddText commands.",
  "slices": [
    {"id": "19.0", "name": "Text-state consolidation + ambient publication (CORRECTNESS; core + CLI; NO new operator surface)", "scope": "Hoist ONE shared TextState type (Tc/Tw/Th/Trise/Tmode/TL/Tf/Tfs) and retire the three private copies (text_extract::page::TextState, text_edit::edit::Walk + reflow_apply::BlockTextState, vector::decompose::GState — the last by composition if a full swap is too invasive; do not leave four). Add the MISSING b\"Ts\" and b\"Tr\" arms to Walk (edit.rs ~L526-544). Publish ambient state on GlyphProvenance INCLUDING the raw operand bytes needed for a byte-faithful restore. Publish a run-level composite flag (promote ExtractFont::is_simple/CodeWidth from pub(crate)). Implement the R88 ladder including the unobservable->refuse tier (multi-stream /Contents walk; Form-XObject case). CLOSE the reflow_apply.rs:455 restore hole with the justify gate still in place. pdfce-cli inspect --text-state.", "acceptance": ["A fixture with 0.5 Tc / 90 Tz ambient around an edited run: follower positions unchanged vs pre-edit render (R59) and preview==saved (R85).", "A fixture with ambient Ts around an edited run: rise survives byte-faithfully (FAILS TODAY — this is the regression fixed).", "Reflow of a block followed by unrelated text in the same stream: following text's rendered spacing unaffected (closes L455).", "/Contents 3-element array with Tc set in element 1 and the edited run in element 3: ambient resolved across the concatenation.", "Run inside a Form XObject: REFUSED AND DISCLOSED BY NAME, not silently restored to 0.", "Exactly one text-state definition reachable from the text pipeline; font.rs:314-323's 'three copies' comment updated or retired.", "cargo tree -p pdfce-core / -p pdfce-render: zero egui/eframe/winit/wgpu/glow. Round-trip/R46 green. All 14.x/15.x/16.x tests unchanged. fmt + clippy -D warnings clean."], "non_goals": ["any operator-facing spacing control", "any synthesis", "any UI"]},
    {"id": "19.1", "name": "Tc + Tz + superscript/subscript authoring (core + CLI) — the Acrobat-parity slice", "scope": "Extend format.rs set_ops/restore_ops (L627-640) with Tc, Tz, and Ts-for-super/subscript (composing with the existing push_tf size change). MetricSpec per R89 with re-derivation on size change. Mirror into addtext.rs at the q..Q-scoped insertion points (L866, L1490) — no restore needed there. Disclose the Tz x justify interaction (Th rescales every TJ adjustment per §9.3.4, so a Tz change invalidates a 15.1-justified line's slack) and OFFER RE-JUSTIFY — never silently leave it wrong. CLI: pdfce-cli format-text --char-spacing <v[unit]> --h-scale <pct> --superscript|--subscript.", "acceptance": ["THE LEAK TEST (the catalog's named must_have): unrelated text immediately following a spacing-formatted run is provably unaffected.", "Restore emits observed raw operand bytes where ambient was set, spec default where provably unset, refusal where unobservable.", "Size-change re-derivation: superscript at 10pt resized to 20pt still sits at the same PROPORTIONAL height (R89).", "Tz change on a justified line: disclosed with a re-justify offer.", "Tagged run: BDC/EMC + MCID preserved, staleness disclosed (R73).", "Only the edited content-stream object re-emitted (R32/R46/R69); incremental-save-safe (R34/R70); undo restores the byte-identical pre-format stream.", "Gates green; fmt/clippy clean."], "non_goals": ["Tw (19.4, gated)", "synthesis (19.2)", "UI (19.3)", "StructTree (cut)"]},
    {"id": "19.2", "name": "Free-form Ts + synthetic bold/italic (core + CLI) — the deliberate exceed", "prerequisite_check": "CONFIRM pdfce-render honours text rendering mode 2 (fill-then-stroke) and a sheared Tm BEFORE starting. If not, preview != saved and R85 is violated the moment synthesis ships — fix the renderer first or descope synthesis.", "scope": "Free-form numeric Ts (CLI --rise, Absolute by default). StyleSynthesis provenance enum. The R90 gate wired behind resolve_target_resource's failure (format.rs:861-894), per-use and declinable, ONE shared type used by format.rs and addtext.rs, with the remedy-ordering difference. Mode-2 emission with user-space-derived stroke width and stroking-colour match/restore; Tm shear for oblique with ABSOLUTE Tm re-emission for followers. Reload-time re-detection of synthesized styles (pdfce's own and other producers'). CLI: --rise <v> --bold-synthetic --italic-synthetic.", "acceptance": ["(a) real resolvable Bold/Italic resource -> the 14.2 real-resource path is taken, NO synthesis offered.", "(b) no real resource anywhere resolvable -> synthesis OFFERED as a named declinable choice, NEVER auto-applied.", "(c) operator declines -> refuse-and-disclose, no partial/silent application.", "(d) synthesized run on a tagged document -> MCID preserved, staleness disclosed, EXPLICITLY asserted not to reproduce Acrobat's documented tag-corruption defect for this edit type.", "(e) faux bold on COLOURED text -> outlines are the text's colour not black (stroking colour matched), and the stroking colour is restored afterward.", "(f) faux bold at 10pt and 72pt -> visually consistent weight (user-space stroke width derivation, §9.3.6).", "(g) Ts x synthetic-italic -> a raised sheared run is displaced by Trise*tan(theta), asserted correct, AND followers unaffected by the shear (absolute Tm re-emission).", "(h) save -> reload -> the synthesized run is re-detected and labelled with NO private marker having been written; round-trip of untouched objects still byte-identical."]},
    {"id": "19.3", "name": "GUI: spacing/style property surface (gui)", "dispatch_first": "pdfce-ui-specialist", "scope": "Property controls for character spacing, horizontal scaling, baseline rise, super/subscript, and the synthesis offer. R83 capability-gating (consumes 19.0's published composite flag). Unit labels stating which unit AND which MetricSpec mode is in force. A 'synthesized' badge on runs carrying StyleSynthesis. Verbatim disclosure strips as 14.3 already does.", "invariant_note": "The capability query ('is this run composite?', 'does a real Bold resolve?') MUST be a pdfce-core API the GUI calls — never logic reimplemented in pdfce-gui, or the WASM fork loses it (R74)."},
    {"id": "19.4", "name": "Tw (CONDITIONAL — do not start without the census)", "gate": {"census": "Over the Pass-11 render-fidelity corpus, produce TWO numbers: (a) REACHABILITY — fraction of all show operators whose font is simple (CodeWidth::One), bounding how often a Tw control could do anything; (b) PREVALENCE — fraction of DOCUMENTS setting a non-default Tw/Ts/Tc/Tz anywhere, sizing the preservation risk independently and worth having regardless of (a).", "decision_bands": {"build": "(a) >= 60% -> build as an R83-gated simple-font-only control with a refuse-and-disclose engine gate (R91).", "close": "(a) <= 25% -> CLOSE the item, recording the census as the reason; point the operator at reflow/justify.", "escalate": "25% < (a) < 60% -> ESCALATE TO KEN — that band is a product call about how much surface a sometimes-control is worth, not a technical one."}}}
  ],
  "invariant_risks": {
    "round_trip_minimal_diff": ["The restore-by-value emission ADDS bytes to a stream pdfce is already rewriting — acceptable under R69, but the restore must never be emitted into an object pdfce did not otherwise touch.", "The temptation to normalize text state stream-wide to make restores easy is FORBIDDEN (ARCHITECTURE.md §5.6 + R32/R46).", "Restoring by re-serialized VALUE rather than by observed RAW BYTES would silently renormalize operands (0.5000 -> 0.5) — a minimal-diff violation on a byte pdfce did touch; hence the ladder's tier 2.", "Incremental save is unaffected — the same object is rewritten (R34/R70). FF-H is NOT a fourth forced-full-rewrite sibling."],
    "gui_core_separation": ["All emission, ambient resolution, composite detection, and Bold/Italic resolvability MUST live in pdfce-core; the only pdfce-gui piece is the property surface and the R83 gating. Verify with cargo tree -p pdfce-core / -p pdfce-render on every slice.", "SPECIFIC HAZARD: the R83 capability gate needs 'is this run composite?' and 'does a real Bold resolve?' at UI-draw time. If the GUI reimplements either instead of calling a core API, the WASM fork loses the gate — R74 violation. 19.0 publishes both precisely so 19.3 cannot be tempted."],
    "r85_preview_equals_saved": "Synthesis introduces render features (mode 2 stroking, Tm shear) that the canvas must reproduce EXACTLY as saved. If pdfce-render does not honour them, R85 breaks the moment 19.2 ships. Hence 19.2's explicit prerequisite check."
  },
  "for_ken_personally": [
    "The Tw census MIDDLE BAND (25-60% simple-font reachability): 'is a control that works on roughly half of documents worth permanent surface area?' is a product judgement, not a technical one. The census will be run and reported; the call in that band is yours.",
    "FF-C's RULE-13 dependency classification: the MIT decision lifted the rule-8 license-undecided gate, it did NOT pre-approve any dependency. A font subsetter still needs classification, escalation to you if copyleft, and a cargo-about regeneration. 'MIT decided' != 'any crate is fine.' Flagged because FF-C is next in the order.",
    "CUTTING StructTree/ActualText out of FF-H: a scoping call I made, but it changes the shape of a named backlog item you may have been counting inside 'finish off all the text handling stuff.' If you consider a minimal structure-tree update part of that directive, say so and it gets its own Pass — it should still not ride inside a formatting Pass.",
    "LIST-AUTHORING remains separately gated and unanswered — ROADMAP.md already records that the text-handling directive does not resolve it. Re-surfaced only so it is not silently assumed either way.",
    "KERNING: Isaacs lists kerning among Acrobat's RETAINED controls, and pdfce has no kerning surface distinct from Tc. That is a parity gap this decision FOUND but did not scope, and it may or may not fall inside your 'finish off all the text handling' intent."
  ],
  "does_not_decide": ["Whether Tw authoring is ever built (census-gated).", "The exact spacing/style interaction UX (pdfce-ui-specialist at 19.3).", "FF-C's and FF-B's internal scope (ordered here, not scoped).", "The StructTree/ActualText update's design (cut from FF-H; needs its own decision inside an accessibility Pass).", "Kerning, leading-as-direct-formatting, composite inter-word TJ as an operator control (named fast-follows, unscoped).", "Any change to R35, R70, R73, R75, or the Pass-6.2 FreeText path.", "Composite/CJK/RTL text handling (FF-E/FF-F, unchanged)."],
  "revisit_triggers": ["The census lands in the 25-60% band -> Ken decides Tw (see for_ken_personally).", "Operators frequently decline synthesis and change family instead -> flip the in-place-edit remedy ORDER to match Add-Text's.", "Reload-time synthesis re-detection produces false positives on real corpus files (e.g. a legitimately stroked display face) -> tighten the detector or demote the badge to a hint.", "FF-C ships and makes embedding a real Bold face routine -> re-evaluate whether synthesis should stay first in the in-place-edit remedy order.", "A tagged-PDF accessibility workflow needs a live structure tree -> schedule the cut FF-I item as a real accessibility Pass."]
}
```

---

# AMENDMENT A — 2026-08-03 — three corrections from building slice 19.0

**Filed by:** pdfce-engineer, after slice 19.0 shipped (`38fffad`). Each item
below is a place where the decision as written was wrong or unbuildable, found
by implementing it. Where this amendment and the record above differ, **this
amendment wins.**

## A.1 The restore ladder needs a FOURTH rung — and without it §19.1 ships a bug

§3.4 / R88 define a three-tier ladder: spec default when provably unset,
**observed raw operand bytes** when set, refuse-and-disclose when unobservable.
The middle rung says "raw operand bytes where available", which is right as far
as it goes and **names the wrong failure**. It assumes the bytes are either
available or absent. There is a third case: **available and poisonous.**

Two operators set text-state parameters as a *side effect* of doing something
else:

| Operator | Sets | Also does |
|---|---|---|
| `TD` (Table 108) | `TL` (leading) | moves to the next line |
| `"` (Table 109) | `Tw` **and** `Tc` | **shows a string** |

Re-emitting the captured bytes of a `"` as a word-spacing restore **repaints
the text**. The bytes are a faithful record of where the value came from and a
catastrophic restore instruction. A ladder that reaches for "the raw bytes" the
moment they exist walks straight into it.

**Resolution, implemented in 19.0:** a fourth origin,
`AmbientOrigin::ObservedIndirect { setter }`. The value is known and restorable,
but by **re-spelling** it in its own dedicated operator (`2 Tw`, `-14 TL`) rather
than by replaying the bytes. It reports `is_byte_faithful() == false` so callers
disclose the narrowing — the same posture already used for `fill_narrowed`.
Inside a Form XObject it degrades to the refuse tier like any other set value.

**R88's wording must change accordingly:** the rule is not "restore from raw
bytes where available" but *"restore from raw bytes where they are a faithful
and side-effect-free record of the value; re-spell where the value is known but
its source operator did more than set it; refuse where the value is
unobservable."*

## A.2 §3.4's tier-3 case (i) — multi-stream `/Contents` — is unreachable in pdfce

The record names two triggers for the refuse tier: state set in one element of a
multi-stream `/Contents` array, and state inherited through a Form XObject. Only
the second exists here.

`ContentStream::from_page` concatenates the entire `/Contents` array into one
decoded buffer *before* any walk runs, and a decode failure on any element fails
the whole page rather than yielding a partial prefix. So there is no
set-in-element-1 blind spot to refuse on. The builder correctly declined to
manufacture a trigger for it rather than leave an untestable branch.

This is a property of pdfce's architecture, not of the spec — **if the
concatenation ever becomes lazy or per-element, the case becomes real and the
refuse tier must grow it back.** Recorded here so that change knows what it
breaks.

## A.3 §19.0's "hoist `Tf`/`Tfs`" is not buildable without moving published output

The extraction walk narrows `Tfs` to `f32` — because `GlyphProvenance::tf_size`
publishes `f32` — and then widens it again for the §9.4.4 advance. The other two
walks carry `f64` throughout. A shared `f64 font_size` turns
`f64::from(v as f32)` into `v`, which **perturbs published glyph positions**.
The same trap applies to `Tz` (narrow-then-divide is not bit-identical to
divide-then-narrow).

So the shared type is **exactly the six single-operand text-state parameters** —
which is R88's own set — and `Tf`/`Tfs` stay with their consumers. Forcing the
hoist would have failed the roundtrip-unchanged gate, which is the gate that
makes a correctness-plumbing slice trustworthy.

Related, and deliberately not "fixed": `pdfce-render::text::TextState` is a
**fourth** tracker in another crate. It stays, for the same reason `advance_tx`
was not pushed across the crate boundary — the render-parity cross-check wants
an independent implementation. **§19.0's "exactly one definition" is true of
`pdfce-core` only**, and saying so is more useful than a claim that reads
tidier.

## A.4 An unrelated live defect this slice exposed

`text_edit::edit::Walk` had **no `q` and no `Q` arm at all** — verified before
and after (`grep -c 'b"q"'`: 0 → 1). Text state *and fill colour* leaked past
`Q` in the model, which means **shipped Pass 14.2 behaviour could re-emit a fill
colour that a `Q` had already discarded**. §1.2's audit reported the missing
`Ts`/`Tr` arms and missed this.

Not a 19.x prerequisite — a bug in already-shipped code, fixed in 19.0 with two
tests. Recorded because the audit that missed it was otherwise the strongest
part of the decision, and "the audit was thorough" is exactly the belief that
lets the next gap through.

---

# AMENDMENT B — 2026-08-03 — three corrections from building slice 19.1

**Filed by:** `pdfce-librarian`, on the engineer's report, after slice 19.1
shipped (`603b051`). Same posture as Amendment A: each item below is a place
where the decision as written was wrong, ambiguous, or (in B.2's case)
already-correct-but-worth-confirming, found by implementing §19.1. Where this
amendment and the record above (including Amendment A) differ, **this
amendment wins.**

## B.1 The `Tz` × justify disclosure named the wrong mechanism

§19.1's scope note (Appendix A JSON, `slices[1].scope`) and the parallel prose
in §3.1's options table both describe the interaction as: "`Th` rescales every
`TJ` numeric adjustment (§9.3.4), so a `Tz` change invalidates a justified
line's slack." **The rescale premise is true in isolation and wrong as an
account of pdfce's own architecture.**

`Th` genuinely does multiply every `TJ` numeric adjustment per §9.3.4 — that
much is accurate. But the `TJ` adjustments that carry a 15.1-justified line's
distributed slack live in `format.rs`'s `pre`/`post` splice segments, OUTSIDE
the `set_ops`/`restore_ops` wrap that scopes a `Tz` edit to its formatted run.
They therefore execute at the run's UNCHANGED ambient `Th`, not the edited
run's new one. Nothing about the pre-existing slack numbers is rescaled by
the edit at all.

**The conclusion survives; the cause does not.** What actually invalidates
the justified line is the formatted run's changed rendered WIDTH (`ΔA`, per
the §9.4.4 `tx` advance formula) — slack computed against the run's ORIGINAL
width is now wrong for its NEW width, regardless of whether any `TJ` number
was itself rescaled. Same practical consequence (re-justify needed),
different mechanism.

**Both the decision text and `ROADMAP.md`'s ★ Pass 19.x §19.1 slice bullet
are corrected** to name the width delta (ΔA) as the cause, not a `TJ`-
adjustment rescale — see the librarian's Pass 19.1 Shipped entry and the ★
Pass 19.x entry. Filed as a general finding (any editor coupling
horizontal-scale formatting to justification slack through an assumed-shared
mechanism should verify which mechanism is actually live in its own
splice/wrap boundaries):
`C:\personal_rag\pdf\lesson_20260803_tz_th_rescales_tj_adjustments_not_slack_outside_wrap.md`.

## B.2 The `Ts`/rise spec citation — verified correct in this document, the error was code-only

The engineer's report flagged "decision 019 §1.3.6 and `text_state.rs` both
cite `Ts` as §9.3.6" (text rendering mode, per `iso32000__s__9.3.md`) where
the correct clause for text rise is §9.3.7. **On inspection, this document
does not contain that error.** §1.3 item 6 ("`Trise` enters the text
rendering matrix as a translation") carries no spec-clause citation in its
own text — the "(§1.3.6)" seen at §3.2 is an internal cross-reference to item
6 of this document's own §1.3 numbering, not an ISO clause number, and every
literal ISO citation for text rise already in this document (§12 References:
"§9.3.7 rise") is correct. **No edit made to this document for B.2** — the
actual citation error was confined to `pdfce-core/src/text_state.rs` (three
comment citations), already fixed by the engineer in the same slice. Recorded
here only so the flag is closed with an explanation rather than silently
dropped.

## B.3 R89's "Tfs" is the BASE size — left ambiguous, now stated

§3.2's "Units — U-discriminated" section states superscript/subscript ratios
are "re-derived whenever `Tfs` changes" without specifying which `Tfs`: the
size in effect for the run BEFORE any format request that also changes size,
or some other resolved value if size and superscript/subscript are edited in
the same operation. The implementation had to choose and chose the **BASE**
size — the `Tf` size operand in effect for the run at the point of
formatting, i.e. the size the operator is setting the run TO if a size change
is part of the same request, not a pre-existing or intermediate value. **R89
is amended to state this explicitly:** ratios in `MetricSpec::Relative`
resolve against the base (post-edit, if the same request also changes size)
font size, not any other candidate value.

## Standing-rule text status (R88 four-rung wording)

The engineer's fourth flagged item — "R88's wording still needs Amendment
A's four-rung form in the `ROADMAP.md` standing-rules text" — was checked and
found **already satisfied**: `ROADMAP.md`'s Standing Rules section already
states the corrected four-rung ladder verbatim (restore from raw bytes where
faithful and side-effect-free → re-spell where known but side-effect-bearing
→ refuse where unobservable), carrying the "corrected from the original
three-rung wording" note. No further edit needed there; recorded here so the
item is closed rather than silently dropped.

---

# AMENDMENT C — 2026-08-03 — six corrections from building slice 19.2

**Filed by:** `pdfce-librarian`, on the engineer's report, after slice 19.2
shipped (`ebe35d8`). Same posture as Amendments A and B: each item below is a
place where the decision as written was wrong, incomplete, or narrower than
its own prose implied, found by implementing §19.2. Where this amendment and
the record above (including Amendments A and B) differ, **this amendment
wins.**

## C.1 §3.6 names the wrong restore set for stroking colour and line width

§3.6's mechanism section treats the stroking colour and the derived stroke
line width purely as things the synthetic-bold path must **set correctly**
(match the fill colour; derive from `Tfs × |Tm| × |CTM|`). It never names
either as something that must be **restored**. But both are ordinary
graphics state, **shared with path painting** — not scoped to text the way
`Tc`/`Tw`/`Tz`/`Ts`/`Tr` are. A synthetic-bold run that leaves `0.264 w` and
a substituted stroking colour in force changes the weight and colour of
every later stroked *path* on the page, not just later text. **Two restore
obligations §3.6 omits entirely; the builder added two new trackers to
`Walk` to close them** (line width and stroking colour, alongside the
existing `Tc`/`Tw`/`Tz`/`Ts`/`Tr` restore set). R88's restore ladder is
amended to cover these two shared-graphics-state parameters explicitly, not
only the six text-state parameters it was originally scoped to.

## C.2 §3.6's "followers must be re-emitted with an absolute `Tm`" is narrower in practice than written

§3.6 (and Appendix A's `mechanism_italic`) states the fix for the `Tm`-shear
propagation hazard as "the follower must be re-emitted with an absolute
`Tm`." **The builder deliberately did NOT convert a producer's own relative
`Td`/`T*` into an absolute `Tm`** — doing so would rewrite the producer's own
line-positioning structure, which exceeds minimal-diff (R32/R46) and, worse,
cascades: an absolute `Tm` substituted for one `Td` would still leave the
*next* `Td` relative to a rewritten baseline, forcing the rewrite to
propagate indefinitely. **pdfce instead requires that the follower already
be positioned by an absolute `Tm`, and refuses (by name, disclosed) when it
is not** — narrower than what the decision text describes, and deliberate.
A twin acceptance test proves the refusal is not "refuse unconditionally":
the same synthetic-italic run succeeds when the next line begins its own
independent `BT…ET` block (no propagation hazard exists across a fresh text
object, since `BT` does not inherit the prior object's `Tlm`/`Tm`).

## C.3 §3.6's bold-width formula ships two of its three factors — disclosed, not dropped

§3.6/Appendix A state the stroke width as derived from `Tfs × |Tm scale| ×
|CTM scale|`. **The authoring walk models the first two factors and not the
third** — it has no model of a page-level `cm` (current transformation
matrix set outside the text object, e.g. by a page-content wrapper or a
Form XObject invocation), so a stroke synthesized inside a scaled `cm`
context is not compensated for that scale. This is **disclosed verbatim in
the builder's report text** ("LIMIT, disclosed rather than hidden"), the
same posture R73/rule-4 already require elsewhere in this project, not a
silent gap found later. Filed as a named limit, not fixed in this slice —
closing it needs `cm`-tracking in the authoring walk, out of scope here.

## C.4 Neither the decision nor Amendment A anticipated that synthetic italic needs text-matrix tracking in the authoring walk at all

Amendment A.3 carefully scoped the shared `TextStateParams` hoist to
"exactly the six single-operand text-state parameters" and excluded `Tf`/
`Tfs` for the narrow-then-widen bit-identity reason given there. **It said
nothing about `Tm`/`Tlm`, because nothing in the decision as written, nor in
Amendment A, anticipated that synthetic italic (§3.6's `M-shear-tm`) needs
the authoring walk to track the text matrix at all** — C.2's absolute-`Tm`
refusal gate cannot be evaluated without knowing whether the follower's
positioning operator is already absolute. `text_edit::edit::Walk` had **no**
`Tm`/`Tlm` tracking before this slice. Pass 19.2 built it: `BT`-reset
semantics, `Td`/`TD`/`T*` next-line derivation, per-show-operator §9.4.4
advance accumulation, and a `matrix_known` honesty flag (so the walk
reports "I don't know" rather than guessing when a matrix-affecting
operator it does not model appears first). A new `Rec::EndText` variant
records the `BT…ET` boundary the reset semantics depend on.

## C.5 Two conflicts the decision never names, both refused rather than silently merged

Neither the decision text nor Amendments A/B anticipated two interactions
that only exist once **two** Pass-19.x mechanisms compose on the same run:

1. **Free-form rise (19.2) vs. the superscript/subscript toggle (19.1) —
   both write `Ts`.** A request that asks for both at once is refused by
   name rather than silently letting one win; there is no principled
   "the toggle wins" or "the free-form value wins" default that does not
   silently discard the operator's other stated intent.
2. **Synthetic italic (19.2) vs. `--pin` (a 19.1-adjacent follower-
   positioning mode).** The closing absolute `Tm` (C.2) and `--pin`'s own
   compensating `TJ` adjustment would each attempt to consume the same
   positional delta — applying both double-consumes it, silently
   mispositioning the follower by the second mechanism's correction on
   top of the first's. Refused by name rather than composed.

Both refusals are disclosed, not silent failures; both are named
acceptance tests in the shipped slice.

## C.6 Add-Text synthesis is not wired — flagged as not delivered, not implied

§3.6's Q2 policy states synthesis is shared identically across Add-Text
(16.x) and in-place edit (14.x), differing only in remedy order. **The
shared `pdfce-core` type (`StyleSynthesis`, the gate, the wording) is built
and `SynthesisPath::AddText` is implemented and tested** — but
`addtext.rs` has **no bold/italic request surface at all**, so the offer
cannot currently be reached from that path. This matches the decision's own
prediction that the gate "will rarely even open here" (R79 defaults Add-Text
to a bundled Standard-14 face, whose family has real Bold/Italic variants)
— but "rarely opens" is not "cannot be reached," and wiring it needs new
request/CLI surface beyond simply extending `format-text`. **Recorded as
not delivered, explicitly, rather than left to be assumed shipped because
the underlying type exists.**

---

**Standing-rule consequence of this amendment:** R88 (the restore ladder)
is extended to name stroking colour and line width as members of the
restore set alongside the six text-state parameters, when a formatting
operation (synthetic bold specifically) sets them. See the corresponding
`ROADMAP.md` Standing Rules update and the `ARCHITECTURE.md` §5.11/§12
entries filed alongside this amendment.

---

# AMENDMENT E — 2026-08-03 — the §3.3 census has been RUN: reachability
88.7%/91.6%/97.4% (doc/operator/glyph), BUILD band cleared; §3.2 reason 2
("large and growing" composite-default share) is FALSIFIED on this
corpus, and the "growing" half is UNTESTABLE on it

**Filed by:** `pdfce-librarian`, on the engineer's report, after new
out-of-workspace crate `tools/tw-census` (zero new Cargo dependencies,
root `exclude`-list convention) ran the census this decision specified
in §3.3. Commits `359d486`/`5387699`, both verified by `git cat-file
-t`. Same posture as Amendments A–C: this amendment resolves an open
question the decision text deliberately left to measurement, not a
correction of an error — but one of the decision's own supporting
arguments (§3.2 reason 2) is shown wrong by the result, and that must
be recorded plainly rather than only the census's operative verdict.

## E.1 Method — restated because the number is meaningless without it

Unit of measurement is the **show operator**, keyed by
`(ContentStreamRef, ByteSpan)` from `GlyphProvenance` — the literal
unit named in this file's own §3.3, deliberately NOT pdfce's
`TextRun` (which splits on geometry/marked-content and would
over-report). Keys are pooled per page, so a Form XObject invoked
twice counts once. The scan is deterministic (sorted path order) and
the one aggregating `HashMap` is summed over exhaustively, never
sampled. Two independent full runs produced byte-identical aggregates.
**Ground-truth calibration is a TEST, not a spot-check** — a
known-simple and a known-composite fixture must classify correctly or
the corpus figure is meaningless; both do.

Denominators are stated exactly and exclude 627 corpus files that
would not load at all and 2,172 that loaded with zero show operators.
**Text-bearing denominator: 1,224 documents / 23,144 show operators /
620,858 shown character codes**, drawn from the Pass-11 render-fidelity
corpus (4,012 files total) this decision's §3.3 specified.

## E.2 The numbers — §3.3's reachability question (a)

| denominator | loose (simple font) | strict (simple AND contains code 32) |
|---|---|---|
| by document (n=1,224) | 86.7% | 43.9% |
| **by show operator (n=23,144)** | **91.6%** | 36.9% |
| by glyph (n=620,858) | **97.4%** | 55.7% |
| median per-document glyph share | 100.0% | 0.0% |

Sub-corpus breakdown (loose, by run): pdf20examples 100% · qpdf
99.6% · pdfbox 89.2% · veraPDF 87.6% · pdfium 42.1% (sole outlier,
smallest sample — 30 text-bearing documents). Document font mix
across all 1,224 text-bearing documents: all-simple 994 (81.2%) ·
all-composite 163 (13.3%) · **mixed 67 (5.5%)**.

## E.3 The numbers — §3.3's prevalence question (b)

Operator prevalence across the 1,224 text-bearing documents: `Tc`
19.6% · **`Tw` 10.9%** · `Tz` 1.2% · `TL` 17.6% · `Ts` 0.1% · `Tr`
7.1%.

## E.4 Verdict — §3.3's decision bands, applied

**91.6% (by show operator, the loose metric) → the BUILD band (≥60%).
Slice 19.4 is cleared to build.** Not a marginal reading: every loose
denominator clears 60%, the weakest being by-document at 86.7%; the
median document is 100% simple; and the figure survives the most
adversarial robustness check applied to it (removing the four
most-glyph-heavy files still leaves 87.3%). **Pass 19.4 itself has
NOT started** — see the `ROADMAP.md` "★ pdfce defect" In-progress
entry filed alongside this amendment: the engineer found a real
document-loading defect via this same corpus sweep and prioritized
fixing it first. The census clearing the BUILD band is unaffected by
that sequencing decision; it is a statement about what the numbers
say, not about what gets built first.

## E.5 §3.2 reason 2 is FALSIFIED on this corpus — the "growing" half is UNTESTABLE on it

§3.2 reason 2 (above, "Its availability is determined by a property
the operator cannot see and did not choose") partly justified
withholding `Tw` on producers defaulting to Type0/Identity-H
composites "even for pure-Latin text... a large and **growing**
share." **81.2% of text-bearing documents in this corpus contain no
composite run at all.** The "large" half of that claim does not hold
here.

**The "growing" half cannot be evaluated by this corpus at all, in
either direction.** This corpus is drawn from PDF-tooling conformance
and regression test suites (veraPDF, qpdf's qtest, pdfbox's
user-submitted bug attachments, pdfium's test corpus, pdf20examples) —
Isartor dates to 2008, qpdf's qtest files are older still. Testing a
claim about *modern* producer defaults needs a corpus of
recently-produced documents (Word/LibreOffice/Chrome print-to-PDF
output) that `fixtures/external/` does not contain and this census did
not sample. **Record both halves**: the "large" premise is falsified
here; the "growing" premise is simply untested, not confirmed or
denied.

**Corpus-composition caveat, also load-bearing:** these are PDF-tooling
test suites deliberately full of edge cases and malformed files, not a
random sample of documents an operator would actually edit — 72% of
the text-bearing set is veraPDF, and 2,053 of veraPDF's 2,896 loadable
files have no text at all. `pdfbox`'s sub-corpus (real user-submitted
bug-report attachments) is the closest thing here to organic real-world
documents, and it is the **most** favourable to `Tw` reachability
(95.9% loose / 89.7% strict by glyph) of any sub-corpus with a
meaningful sample size — meaning the blended, veraPDF-heavy figure
above UNDER-states reachability if anything, which strengthens rather
than weakens the BUILD reading in E.4.

## E.6 The strict metric is flagged untrustworthy, not acted on

The strict variant (simple AND contains code 32) lands in the escalate
band (25–60%) at the operator level, but it is fragile: it moves 12
points on the removal of four files (the single largest contributor is
18.6% of all glyphs in the corpus; the top ten are 62%; the three
largest veraPDF contributors are implementation-limit conformance
probes carrying 32k–65k glyphs with ZERO code-32 occurrences). It is
also structurally asymmetric — an equivalent "has a space" test cannot
be applied to composite runs, because in an Identity-H subset the space
is a CID, not code 32 (corpus-wide, composite-run code-32 occurrences
total 73). **Reported here as context. §3.3's decision bands are
written against, and satisfied by, the loose metric — this amendment
does not open the strict metric's escalate-band position as a live
question.** (See `ROADMAP.md`'s Open operator questions item (g),
closed as moot for this same reason.)

## E.7 Other honest limits, recorded rather than smoothed over

- **`/ActualText` blind spot:** text arriving via `/ActualText` carries
  no glyph provenance and is invisible to this census. Cross-checked
  against the independently-written text-extraction harness: **99.6%
  agreement on the text/no-text predicate over 2,892 files**, all 11
  disagreements being `/ActualText`/Unicode-CMap conformance probes —
  the blind spot is real but small and its shape is known.
- **Five show operators had glyphs disagreeing about the composite
  flag** (2 pdfbox, 3 veraPDF) — impossible in principle, since one
  `Tf` governs one show operator. Immaterial to the aggregate; an
  unchased anomaly, recorded rather than silently dropped.
- The "text-free" bucket (2,172 files) mixes genuinely blank pages
  with content streams that deliberately fail to decode; the tool
  cannot currently separate the two.
- **A defect the builder found and fixed in its own tool, disclosed
  rather than hidden:** the TSV header and the failure-row shape were
  written by two separate code paths that disagreed by one tab field
  (all 627 failure rows had 29 fields against a 30-field header).
  Aggregates were never affected — computed in memory, not re-parsed
  from the TSV — and the TSV was regenerated. Both shapes now derive
  from one shared list with an assertion and two tests. Filed as a new
  instance of R92's duplicated-definition pattern (a hand-duplicated
  shape drifts silently the moment one copy changes).

## E.8 What this amendment changes and does not change

**Changes:** §3.3's reachability/prevalence questions are now answered
with numbers, not left open; §3.2 reason 2 is corrected from an
unmeasured hypothesis to a partly-falsified, partly-untestable claim;
slice 19.4 moves from "conditional, ungated numbers" to "cleared to
build, sequenced behind a higher-priority defect fix." **Does not
change:** §3.3's decision bands themselves (still ≥60% build / ≤25%
close / 25–60% escalate); reasons 1 and 3 of §3.2 (the `TJ`-does-it-
better argument and the "don't build on an unmeasured premise"
methodology point), neither of which this census bears on; the
composite-run structural void (§9.3.3) itself, which is untouched by
any corpus finding. Full numeric record, sub-corpus table, and the
pdfce document-loading defect this same sweep found: `ROADMAP.md`'s
continuation-67 In-progress entry and the "★ pdfce defect" entry
alongside it; `ARCHITECTURE.md` §5.11/§12.

