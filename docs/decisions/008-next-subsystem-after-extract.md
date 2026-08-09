# Decision 008 — The next major subsystem after read → write → edit → extract

- **Date:** 2026-07-31
- **Status:** Decided
- **Decider:** KenAgent (autonomous-builder), per the ROADMAP standing rule
  "KenAgent decision routing (operator process rule, 2026-07-30)"
- **Question:** With decision 007's ranked read → write → edit → extract
  sequence complete, which subsystem is next?
- **Outcome:** Annotations & markup — sliced into three Passes, the first
  of which introduces no authoring capability at all and is a *read-side*
  Pass.
- **Supersedes:** nothing. **Amends:** decision 007's "Pass 6+"
  placeholder (never a real allocated ID). Decision 007's Pass 5
  (Encryption) keeps its ID and its rationale; only its queue position moves.
- **Adds standing rules:** R43–R52.

---

## 1. Context — where pdfce actually is

Decision 007's sequence is complete, and it delivered more than it promised:

- 2,892 of 2,914 conformance files load and render (99.2%).
- All five image-codec families decode.
- Full text render including CID/`Identity-H`.
- An identity writer with a **100.00%** whole-file byte-identity
  round-trip gate over 2,898 loadable files, and `save_full` at 99.97%
  with the single miss a correct named hybrid refusal.
- A mutation writer with a command-log undo stack whose defining bug —
  §11.1's "union of every command ever run" — is *structurally
  unexpressible*, pinned at edit → undo → save byte-identical
  2,897/2,897.
- Seven structural page operations with a real free list, a reachability
  sweep, and DocMDP-aware signature-impact classification.
- Text extraction at **99.78% sourced**, with the sourced/derived
  boundary exposed as two separate API surfaces.
- 875 tests, 10 fuzz targets, zero crashes. GUI and CLI both shipping.

pdfce is now a genuinely good PDF viewer that can also restructure
documents. And there is a category of content it does not draw at all.

### 1.1 The finding that reframes the question

**pdfce renders no annotations.** Not highlights, not stamps, not
sticky notes, not form fields. `crates/pdfce-render/src` contains no
`/Annots` code path. The word "annotation" appears in that crate only in
doc comments describing what the Form-XObject machinery will *eventually*
be used for (`interpret.rs:143`, `interpret.rs:452`).

Worse than missing: **undisclosed**. There is no `Diagnostics` counter
for it. Every other gap in pdfce — Type 3 fonts, `sh`, `/SMask`, JPX
enumerated colour spaces, substituted glyphs, dropped `/StructTreeRoot`
— is counted and named under R20/R27. This one is silent. A page
carrying forty filled form fields renders as though the page were blank
of them, and pdfce says nothing.

### 1.2 How much content that actually is — measured

This project decides on numbers, so I measured it. Read-only census,
aggregate counts only, nothing copied — the same LEGAL §5 posture as
decision 007's `/Encrypt` census.

**Conformance corpus (all 2,914 files):**

| Metric | Count | Share |
|---|---|---|
| Files with ≥1 annotation | **338** | **11.6%** |
| Annotations total | 429 | |
| Annotations carrying `/AP` | **228** | 53.1% |
| Files with `/AcroForm` | 127 | 4.4% |
| Files with `/XFA` | 4 | 0.14% |

Top subtypes: `/Link` 98, `/Widget` 87, `/Circle` 47, `/Popup` 34,
`/Highlight` 14, `/3D` 14, `/FileAttachment` 13.

**Organic sample — 2,500 randomly selected (seed 20260731) of 25,203
PDFs discovered under `the operator's document store`:**

| Metric | Count | Share |
|---|---|---|
| Files with ≥1 annotation | **814** | **32.6%** |
| Files with `/AcroForm` | **753** | **30.1%** |
| Annotations total | 55,545 | |
| Annotations carrying `/AP` | **43,508** | **78.3%** |
| `/Widget` | 48,781 | 87.8% of annots |
| `/Link` | 5,180 | |
| `/Square` | 1,067 | |
| `/Stamp` | 224 | |
| `/FreeText` | 42 | |
| `/Highlight` | 37 | |
| AcroForm fields | 47,868 (`/Tx` 47,774 = 99.8%) | |
| Files with `/SigFlags` | 16 | 0.64% |
| Files with `/XFA` | 2 | 0.08% |

**Roughly one in three of the operator's own PDFs carries annotations,
and roughly one in three carries a form.** pdfce draws none of it.

**Honest caveats, stated up front rather than buried:**

1. **Per-file figures are robust; per-annotation figures are
   concentration-skewed.** 47,774 `/Tx` fields across 753 files averages
   ~63 fields per form-bearing file, which almost certainly means a
   minority of form-heavy documents dominate the count. Reason from
   32.6% and 30.1%, not from 48,781.
2. **The organic population is this operator's**, Dropbox-dominated
   the operator's own real-world working documents. That is the *right*
   population for prioritizing this operator's tool. It is not a claim
   about PDFs in general and must never be written as one.
3. **R31 binds me too.** This census used pypdf 6.7.0 and I did not
   independently verify its conventions (inherited `/Annots` resolution,
   malformed-array tolerance). The numbers are indicative. They **must**
   be re-measured with pdfce's own machinery before they become a gate
   denominator — the same W16 discipline that re-pinned Pass 3.0's
   denominator at 2,898. A gate whose denominator is uncertain cannot
   report an honest shortfall, and an honest counted shortfall is the
   entire deliverable.

Two useful byproducts fell out. **XFA is 0.08% of the operator's files**
(2 of 2,500; 4 of 2,914 conformance) — a measured answer to the demand
half of the standing "verify XFA status before committing engineering
time" item. And **`/SigFlags` is 0.64%** — which supports candidate F's
last place.

### 1.3 What is already built *for* this subsystem

As with decision 007, the most consequential facts live in code rather
than in the roadmap.

**The Form-XObject execution machinery is done.** An `/AP` `/N` stream
*is* a form XObject. §8.10.1's five-step procedure, the
fresh-`Interpreter`-over-a-cloned-`GraphicsState` design, the
object-number-keyed cycle guard, `MAX_XOBJECT_DEPTH = 64`, and the
per-interpreter font cache that made resource scoping *correct* rather
than merely conventional — all shipped in the Pass 1.1 XObject slice.
Pass 6.0 is annotation-dictionary walking, appearance selection,
placement, and flags. It is not a rendering engine.

**Base-14 metrics are in core, GUI-free.** `fontdata::std14_width`,
`std14_descriptor`, `std14_builtin_encoding`, `encoding_glyph_name`.
Everything a variable-text appearance generator needs for the common
case, on the correct side of the §3 invariant.

**And R17 already reserved the road.** Decision 004: *"`harfrust` may
only ever enter a future text-**authoring** path."* That path has never
existed. It appears for the first time in Pass 6.2 — and should be built
*without* `harfrust`, on Base-14 and embedded-font widths, with complex
scripts named as a gap rather than half-supported.

### 1.4 The capability that does *not* exist yet

**There is no content-stream writer in this codebase, and `Stream`
cannot represent authored bytes.**

`object.rs:246` defines `Stream { dict: Dict, data_span: ByteSpan }` —
data is a span into a retained source buffer, never owned.
`writer/serialize.rs:182` exposes `stream_data(stream, source) ->
Option<&[u8]>` and that is the entire stream-emission story. Pass 3.0's
headline result was literally *"0 objects re-serialized under
`SaveOptions::identity()`"* — the writer's whole design point is **not**
re-emitting content.

The precedent and the tripwire both already exist, written down.
`pageops/assemble.rs` invented a **staging buffer** for cross-document
stream copies:

> *"every copied stream's raw, still-filter-encoded payload is appended
> to a staging buffer and the copy's span is repointed at it… the
> existing serializer works unmodified, because 'a value tree plus the
> buffer its spans index into' is precisely its interface."*

And `DocumentView`'s doc comment says, verbatim:

> *"no Pass 3.2 session edit introduces stream bytes the base buffer
> does not already hold; **a Pass that changes that must revisit this
> type**, and the assertion is written down here so it cannot be
> forgotten."*

Pass 6.1 is the Pass that changes that. The engineer inherits a working
pattern **and** an explicit obligation. See R45 and risk X5.

---

## 2. The candidates, and the question that separates them

Restated: **A** annotations & markup, **B** forms/AcroForm,
**C** redaction, **D** encryption, **E** vector/content-stream editing,
**F** signatures/PAdES.

Decision 007's separating question was *"what is each one a precondition
for?"* That question is less discriminating now — the writer unblocked
everything, so all six are buildable. The question that separates them
now is different, and it is two-part:

**(i) What *new capability class* does each require?**
**(ii) Which one can be measured?**

| | New capability required | Corpus-measurable? |
|---|---|---|
| **A** annotations | Appearance *display*, then authored streams | **YES — 11.6% conformance, 32.6% organic** |
| **B** forms | A's appearance pipeline + variable text + field model | **Partly — 4.4% / 30.1% carry `/AcroForm`** |
| **C** redaction | Content-stream **surgery** + container decomposition | No |
| **E** vector editing | Content-stream **surgery** (largest scope) | No |
| **D** encryption | Crypt stage at the R37 seam | No — measured **zero** corpus payoff |
| **F** signatures | Crypto (needs D) + heaviest external sourcing | No |

Two capability classes, not six subsystems:

- **Authored streams** — build a *new* content stream from scratch and
  package it as a form XObject. Needed by A, B, and C's mark step.
- **Content-stream surgery** — decompose an *existing* content stream,
  modify it, re-emit minimally. Needed by C and E.

A is the natural first consumer of the first class. C and E share the
second. B sits entirely on top of A. D and F are their own island.

### 2.1 The argument that settles A over C and E

**An annotation edit touches no page content stream.**

An annotation is a separate indirect object referenced from the page's
`/Annots` array; its appearance is a separate form XObject. Authoring one
means *adding objects and patching one array*. Page content streams stay
byte-verbatim.

This is not a convenience — it is the minimal-diff invariant's **best
case**, and it is literally the motivating example `ARCHITECTURE.md` §5
uses:

> *"Acrobat users expect that adding one comment to a 400-page contract
> does not perturb the other 399 pages' bytes."*

Annotation authoring extends Pass 3.1's mutation model into
content-bearing territory while staying in pure incremental-save
territory. C and E both require surgery on existing content streams — a
strictly larger, strictly riskier step onto ground nobody has walked.

And §11.3 already pre-assigned this work: *"The command-pattern model is
the default for content-level edits (text, **annotations**, form fields,
single-page operations)"*, plus *"Don't invent a second, parallel undo
system."* §11.4 is discharged. A new editing subsystem **inherits** undo
rather than owing it — a whole obligation Pass 3.1 already paid.

---

## 3. The decision

**Build annotations & markup. Slice it into three Passes. Make the first
one read-only.**

| Pass | Name | Authoring capability | Acrobat RAG |
|---|---|---|---|
| **6.0** | Annotation & widget appearance **rendering** | **none — deliberate** | recommended |
| **6.1** | Authored streams + content-stream serializer + markup authoring | Ink, Square, Circle, Line, Polygon, quad-point markup | **required** |
| **6.2** | Text-bearing annotations + variable text | FreeText, Text+Popup, Stamp | **required** |
| **7** | Interactive forms / AcroForm (**B**) | field model, fill, flatten | **required** |
| **8** | Redaction (**C**) | true removal | **required** |
| **9+** | Vector / content editing — Inkscape parity (**E**), sliced | — | — |
| **5** | Encryption (**D**) — ID retained, position moved | — | yes |
| **10** | Signatures / PAdES (**F**) | — | yes |

Ranking: **A ≫ B > C > E > D > F**, with two qualifications that matter
more than the ordering:

1. **Pass 6.0 serves A and B jointly.** A form field's `/Widget` *is* an
   annotation with an `/AP`. 87.8% of organic annotations are widgets.
   The first Pass is not "annotations instead of forms" — it is the
   display half of both.
2. **Pass 6.1 delivers C's and E's prerequisite.** The content-stream
   serializer plus its corpus identity gate is what redaction and vector
   editing are both waiting on. The project's distinctive
   Inkscape-parity purpose has its foundation land **second**, not last.

### 3.1 Why the slice falls exactly there

- **6.0 / 6.1 splits on authoring**, which is the line §11.4 draws.
  A read-only Pass carries no undo obligation and — more importantly —
  no way to produce a wrong file.
- **6.1 / 6.2 splits on text.** 6.1's annotation set is purely
  geometric: coordinates, paths, colours. No font resolution, no `/DA`
  parsing, no text layout. That isolates the *infrastructure* (staging
  buffer, content-stream emitter, form-XObject packaging, `/AP` wiring)
  so it can be tested without variable text confounding it. 6.2 then
  adds §12.7.3.3 variable text against infrastructure already proven —
  and hands Pass 7 a generator it did not have to build under feature
  pressure.

### 3.2 Why 6.0 is read-only — the Pass 3.0 argument, one level over

Decision 007's central move was to ship a writer that could not edit,
and spend the whole Pass proving an invariant across 2,892 files. It
worked: the gate closed at 100.00%, W14's stop threshold was never
approached, and every editing Pass since has landed against a measured
gate instead of a promise.

The same three properties hold here, and one more:

1. **It cannot regress silently later.** Every appearance pdfce
   *generates* from 6.1 onward is validated by the same renderer that
   6.0 proved against 228 real appearance streams.
2. **It defuses §11.4.** No mutation, no undo obligation, so the largest
   new subsystem is not fused to a second one.
3. **Failure is cheap and early.** If appearance placement turns out not
   to be reconcilable with real files, that is discovered with zero
   authoring code in the tree.
4. **New here — it is the only candidate a corpus can measure.** The
   operator's brief correctly observes that most of this slate consists
   of creation features conformance corpora will never exercise.
   Appearance *consumption* is the exception, and it is a large one:
   228 appearance streams across 338 conformance files, 43,508 across
   the organic sample. That is not a proxy metric. It is the actual
   thing.

There is a fifth reason, less about method and more about honesty: 6.0
closes a gap where pdfce is currently *silent*. Under R20/R27 this
project counts and names everything it cannot do. Annotations are the one
place it does not. Fixing the gap and fixing the silence are the same
Pass.

### 3.3 The appearance-stream generator — the question, answered

The brief asks whether the appearance generator should be its own
shared-infrastructure Pass. **Split it.**

**The consumption half is its own Pass — and it is Pass 6.0.** It has an
oracle available today, it introduces no capability that can produce a
wrong file, and it is shared by A, B, and C's mark step. That is exactly
the profile that justified Pass 3.0.

**The generation half is not a standalone Pass.** An appearance generator
with no annotation to generate for has no acceptance criteria and would
be built blind — precisely the failure decision 007 §3.2 argued against
for the writer (*"doing the writer before the renderer would have meant
building it blind"*). Its oracle is *"does the authored annotation
display, round-trip, and survive reload"*, which requires a consumer.
So it ships **inside Pass 6.1** with the geometrically simplest
annotation set.

What is genuinely shared, and lands in 6.1: form-XObject packaging
(`/BBox`, `/Matrix`, `/Resources`), `/AP` `/N`/`/D`/`/R` and `/AS` state
wiring, the §12.5.5 placement algorithm, the content-stream token
emitter, and the authored-stream representation.

What is **not** shared, and must not be in 6.1: **variable text**
(§12.7.3.3 `/DA`, `/Q` quadding, comb fields, multiline wrap, `/RC` rich
text). That is the single hardest piece of appearance generation. Putting
it in the same Pass as the infrastructure is how this subsystem doubles.

### 3.4 One more infrastructure gate, folded in rather than split out

Pass 6.1 introduces the project's first content-stream serializer. It
should therefore carry a **content-stream identity gate**: parse →
re-emit → byte-compare every content stream in the loadable corpus, plus
raster identity.

This is Pass 3.0's move applied one level down. It is nearly free —
`content.rs` already carries per-token byte spans, built in Pass 1
specifically so the semantic operator view could stay *a projection over
the tokens, never the primary representation*. And it de-risks **both**
remaining large subsystems: redaction and vector editing are content-
stream surgery, and surgery on a serializer that silently normalizes
would produce plausible, working, wrong files at scale.

It is folded into 6.1 rather than made its own Pass because, unlike
6.0, it has no operator-facing meaning on its own and no reason to be
sequenced separately — it is an acceptance criterion, not a deliverable.
See **R46**.

---

## 4. Pass 6.0 — full scope

### Deliverables

1. **`pdfce-core` annotation model** — walk a page's `/Annots`, resolve
   each annotation dictionary, expose `/Subtype`, `/Rect`, `/F` flags,
   `/AP` (`/N`, `/R`, `/D`) and `/AS`. Placement follows decision 005's
   axis (R26): **core decodes and models, render paints.** Core selects
   the appearance stream; core does not paint and does not decide colour.
2. **`pdfce-render` appearance painting** — through the **existing**
   §8.10.1 form-execution path. No second, shorter path (see X8).
3. **§12.5.5 placement, implemented as specified and cited** — transform
   `/BBox` by `/Matrix`, take the bounding box of the result, compute the
   matrix `A` mapping that box to `/Rect`, render with `A × /Matrix`.
   A degenerate transformed box is a **named refusal**, never a
   divide-by-zero.
4. **Appearance selection** — `/AP` `/N` may be a stream *or* a
   sub-dictionary keyed by appearance state, with `/AS` selecting.
   Missing `/AS` against a multi-entry sub-dictionary is a named,
   counted diagnostic — never a guessed pick.
5. **§12.5.3 flags (Table 165)** — Hidden and NoView are not painted;
   Print is honored for a future print path; `/Popup` is never painted as
   page content. Every suppression is **counted** (R50).
6. **Annotations with no `/AP`** — counted by subtype, never
   synthesized (R43). This counter is the measured demand signal for
   6.1/6.2/7: it is the number that says how much appearance
   *generation* real files actually need.
7. **`Diagnostics` extension** — `annotations_total`,
   `annotations_painted`, `annotations_without_ap` (by subtype),
   `annotations_hidden`, `annotations_appearance_state_missing`,
   `annotations_widget`, `need_appearances_documents`. New CLI keys are
   **appended**, never reordered (the standing stable-line contract).
8. **GUI** — an annotation-visibility toggle, display-only. No
   selection, no hit-testing. Dispatch `pdfce-ui-specialist` (X14).
9. **`pdfce-cli`** — `render-page` honors annotations with an explicit
   suppression flag (so the pre-6.0 raster stays reproducible);
   `list-annotations` emits the per-page inventory in the
   locale-invariant stable-line format.
10. **`tools/corpus-report`** extended with the annotation counters,
    plus a sweep that re-measures §1.2's census with pdfce's own
    machinery.
11. **Fuzz target 11** — annotation walking + appearance selection:
    cyclic `/AP`, degenerate and inverted `/Rect`, `/AS` naming a missing
    state, `/AP` `/N` that is neither stream nor dictionary.

### Acceptance criteria

- **The census is re-run with pdfce's own tooling and pins the
  baseline** before the gate is written (W16 discipline). If pdfce's
  count differs materially from pypdf's, run the discrepancy down — it
  is a finding either way, not something to average.
- Every annotation with an `/AP` `/N` resolving to a form XObject is
  painted, or refused by a **named** diagnostic. Any shortfall is
  enumerated **by file and by reason** (R20 tradition).
- **pdfium raster differential** on the annotation-bearing subset.
  This Pass is what finally justifies the Pass 1.1 reference-renderer
  harness remainder, because appearance *placement* is a silent-wrongness
  class a self-comparison oracle structurally cannot catch — pdfce
  agreeing with pdfce says nothing about whether the stamp is in the
  right place. Decision 006 §3.2's ad-hoc `pypdfium2` run is the tooling
  precedent. **Do not claim the Pass 1.1 remainder closed** unless the
  harness is genuinely generalized; if it runs only on the annotation
  subset, say exactly that.
- **Synthetic geometry fixtures** pin placement from both directions:
  non-origin `/BBox`, rotated `/Matrix`, scaling `/Matrix`, `/BBox`
  larger and smaller than `/Rect`, inverted `/Rect`, degenerate `/BBox`.
  `tools/gen-annot-fixtures.py` in the established pattern, with a
  `PROVENANCE` file (LEGAL §5).
- Hidden/NoView provably not painted (fixture where it would be
  unmistakable) and counted. `/Popup` provably not painted.
- **Pass 3.x and Pass 4 gates UNMOVED (R34)** — noting that the writer
  round-trip raster oracle is a *self*-comparison and is structurally
  unaffected, while any pinned reference raster changes and must be
  re-baselined **deliberately and visibly**.
- veraPDF §6.1.12 against every new guard, reporting **measured
  headroom** per the Pass 3.2 / Pass 4 precedent.
- `cargo fmt --check`, `clippy -D warnings`, `cargo tree` GUI-free on
  host + `x86_64-pc-windows-msvc`, wasm32, `--duplicates`, `ui-strings`,
  `no-network` — all clean.
- No new dependencies expected; LEGAL §6 classification + `cargo-about`
  regeneration in-Pass if any is proposed.

### Explicit non-goals — binding

- **No authoring of any kind.** No creation, editing, deletion, or `/AP`
  generation. Therefore no undo obligation.
- **No appearance generation for annotations lacking `/AP`.** They are
  counted. Generating one here is the exact silent-guess failure R29/R30
  exist to prevent, and it would pre-empt 6.1's design.
- **No `/NeedAppearances` honoring** beyond counting the documents that
  set it (R51).
- No hit-testing, no selection, no annotation panel, no comment
  list/summary/export.
- No `/RichMedia` / `/3D` / `/Movie` / `/Sound` / `/Screen` playback.
  Their static `/AP` `/N` poster frame paints like any other appearance;
  the media itself is permanently out of scope absent a new record.
- **No link activation.** Links paint their appearance (most have none)
  and nothing else. Navigation has its own security posture; R12/R13
  govern anything touching the network or launching a process.
- No print-specific rendering path.
- **No content-stream writing of any kind.** That is 6.1, and it is the
  line this Pass must not cross.

### Parallel tasks

- `pdfce-spec-librarian` for the §12.5 clause tree — **blocking**.
- `pdfce-acrobat-librarian` for "Comments & markup" — not blocking for
  6.0, **blocking for 6.1**, so starting it now costs nothing and
  removes 6.1's blocker.
- The pdfce-native census re-run.

---

## 5. Why not B, C, E, D, F first

### 5.1 B — Forms — ranked **second**, and it is close

**For:** 30.1% of the operator's own files carry an `/AcroForm`, the
highest measured per-file share on the slate. `/Tx` is 99.8% of fields,
so the dominant case is *the one kind of field a variable-text generator
handles*. Filling a form is arguably the single most common thing a
person does to a PDF that isn't reading it.

**Against being first:** its display half *is* Pass 6.0 and its
appearance half *is* Pass 6.2. Running B first means building the
appearance pipeline anyway, but under feature pressure, with the field
model, `/NeedAppearances`, flatten, and field auto-detection all landing
in the same Pass. Sequencing B after 6.x converts its largest single risk
— appearance regeneration — from new work into reuse.

There is also a scope trap that has to be named before B is scoped:
**embedded JavaScript.** AcroForms carry calculation, validation, and
format scripts. pdfce's posture on executing them is a *security*
decision that must be made explicitly (my strong recommendation: never
execute, recognize and disclose), and it does not belong to a Pass that
is already large. Route it to the acrobat-librarian as a capability
question and to a decision record as a posture question.

### 5.2 C — Redaction — ranked **third**, with a live promotion trigger

Redaction is the most **over-specified unbuilt subsystem in the
project**. Its constraints are already fully written down: R35 (forces
full rewrite, refuses incremental), the §5.7 / §11.2 container
correction, §5.5's redact-a-signed-document either/or, §11.2's
irreversibility and confirmation requirement, §7's CLI parity clause, and
the owed byte-grep test with object-stream-compressed fixtures. Its
implementation is entirely absent.

Placed third because it needs a capability class nothing before it has:

- **Content-stream surgery** — removing operators and slicing glyph runs
  out of `Tj`/`TJ`. Sits on Pass 6.1's serializer and identity gate.
- **Container decomposition** — and this one is *proven mandatory and
  unsupported*. The Pass 3.1 correction is unambiguous: object streams
  carry through **verbatim in both save modes**, because `save_full`
  re-emits containers intact with zero promotions *by design*. So a
  redacted object's old value survives inside its untouched container
  under either mode. R35 is necessary and **not sufficient**.
- **The mark phase** — a `/Redact` annotation with an `/RO` overlay,
  i.e. it consumes Pass 6.1's authoring. This is *why* Acrobat's
  two-phase mark-then-apply model separates cleanly, and it is exactly
  the fuzzy-never-sneaky shape (R52).

Its search input and its verification oracle are the text extraction
that just shipped — the brief is right about that tie.

**Promotion trigger, and it is cheap:** if the operator states a real
redaction need, Pass 8 promotes ahead of Pass 7 immediately. By then
6.1 has already delivered the content-stream prerequisite and the
`/Redact`-capable authoring. Only the surgery, container decomposition,
and image redaction remain.

### 5.3 E — Vector / Inkscape parity — ranked **fourth**, and *not* deferred

This is the project's distinctive purpose and it deserves a precise
statement rather than a ranking: **E's load-bearing foundation lands
second.** Pass 6.1's content-stream serializer plus corpus identity gate
is E's prerequisite. Pass 6.0/6.1's annotation selection and hit-testing
is the same interaction problem as clicking a path.

E is placed after redaction because it is the largest candidate by a
wide margin, it must itself be sliced, and it benefits most from landing
on infrastructure a smaller, trust-critical consumer has already proven.
Suggested slicing: (a) object model + hit-test + selection;
(b) transforms / z-order / group-ungroup; (c) node/Bézier editing;
(d) boolean path operations; (e) gradients + shading + transparency
(decision 007 already folded these in); (f) OCG layers;
(g) text-to-path.

**If the operator wants this sooner, it can promote directly after
Pass 6.1.** Say so when briefing — "the distinctive-purpose work is two
Passes from its foundation, not six" is the accurate framing, and it is
materially different from what the ordering alone suggests.

One unowned obligation: the existing ROADMAP note requires *"a
feature-parity catalog of Inkscape's editing capabilities — capability/
behavior only, never its GUI mechanics."* That catalog has **no owner**;
the acrobat-librarian's remit does not cover it. Assign one before this
bucket is scoped, and keep the GPL-2.0-or-later behavioral-reference-only
rule loud.

### 5.4 D — Encryption — ranked **fifth**, ID retained, and the fallback track

Decision 007's reasoning stands unchanged and its census already
answered the promotion question: 0.67% of organic files, 92.5% legacy
R≤4, trigger **not met**. §7.6 remains the single largest spec gap on
the slate. pdfce still cannot validate an encryption implementation with
the corpus it owns.

Two things change:

1. **It is this decision's designated fallback/interleave track**,
   exactly as Pass 4 was for decision 007. It is fully independent of the
   appearance/authoring arc, so it is the switch target if a 6.x Pass
   hits the three-attempts wall.
2. **Pass 6.1 creates a new dependency on it.** Authoring annotations
   means authoring *strings* (`/Contents`, `/T`, `/Subj`), which are
   encrypted per object. Until Pass 5, annotation authoring on an
   encrypted document is **refused by name** (X10). The R37
   object-encoder seam already exists, so the eventual fix is a plug-in,
   not a retrofit — which is precisely what R37 was for.

Its real promotion trigger is not a percentage. It is the operator
hitting a file that blocks actual work.

### 5.5 F — Signatures / PAdES — ranked **last**

Depends on D for crypto primitives; heaviest external sourcing on the
slate (ETSI EN 319 142 B-B/B-T/B-LT/B-LTA, RFC 3161, PKCS#7/CMS); 0.64%
of organic files carry `/SigFlags`.

Worth recording that pdfce's *read* half is disproportionately far
along: `signature.rs` is 810 lines, `iso32000__s__12.8.md` is 689 lines,
DocMDP/FieldMDP classification ships, and §5.5's *"pdfce never
re-serializes a signature dictionary, even identically"* is an enforced
structural rule. **Signing** is the missing half — and the
incremental-update-based signing model it needs is already the default
save mode. F is last by dependency, not by distance.

---

## 6. Standing rules added by this decision (R43–R52)

Binding, in the tradition of R1–R42.

- **R43 — pdfce renders an annotation from its appearance stream, or not
  at all.** §12.5.5 makes `/AP` `/N` normative when present. pdfce
  **never** synthesizes a look at display time — not for a `/Widget`
  with an `/MK` and no `/AP`, not for a `/Square` with an `/IC` and no
  `/AP`, not ever. An annotation without a usable appearance is counted
  by subtype and not painted. This is the display-side sibling of R29's
  null rule, and it exists because a synthesized appearance is a
  plausible, working, *wrong* picture of a document: the operator sees
  something no other reader shows them, with no way to tell it was
  invented.

- **R44 — Any appearance pdfce generates is written into the file, never
  rendered privately.** From 6.1 onward, a generated appearance is
  emitted as a real `/AP` form XObject in the saved document. pdfce never
  carries a pdfce-only rendering of a pdfce-authored annotation —
  that produces a document that looks correct in pdfce and blank
  everywhere else, the worst possible outcome for an editor whose purpose
  is producing files other tools consume.

- **R45 — Authored stream bytes live in an explicit session staging
  buffer; `Stream` keeps its span model.** The `ByteSpan`-into-a-buffer
  design is what §5's verbatim re-emission is built on; an owned-bytes
  variant would fork the verbatim path at every match site. The first
  subsystem authoring new stream bytes extends `EditSession` with a
  staging buffer and repoints spans at it — the `pageops::assemble`
  pattern, generalized. `DocumentView`'s written assertion is
  **discharged by amending the type**, never by deleting the sentence.

- **R46 — The content-stream serializer is proven by an identity gate
  before it authors anything.** Parse → re-emit → byte-compare every
  content stream in the loadable corpus, plus raster identity, as an
  acceptance criterion of the Pass introducing the serializer. Same
  inversion as Pass 3.0, one level down, nearly free because `content.rs`
  already carries per-token byte spans. Every later Pass touching the
  serializer re-runs it (the R34 pattern).

- **R47 — An annotation edit never touches a page content stream.**
  Authoring adds objects and patches `/Annots`; page content stays
  byte-verbatim. Any annotation-adjacent operation that *would* rewrite
  page content (flatten, burn-in, apply-redaction) is a separate,
  explicitly named, separately confirmed operation — never a side effect
  of annotating.

- **R48 — Flatten is destructive and discloses what incremental save
  cannot undo.** Flattening destroys the editable object. Under
  incremental save the pre-flatten annotation **survives in the prior
  revision** and is trivially recoverable — R35's structural property in
  a different costume. Flatten must disclose the save-mode consequence
  and offer the full rewrite, and the flatten Pass owes a byte-grep test.
  Never a save-time side effect, never a default, never silent.

- **R49 — A widget is an annotation first.** The AcroForm field model
  layers *on top of* the annotation model. Exactly one appearance
  pipeline and one `/AP` resolution path exists in pdfce, shared by
  markup and fields. A second, forms-only pipeline is forbidden — the
  same failure §11.3 already forbids for undo.

- **R50 — Hidden is honored AND counted.** Table 165's Hidden and NoView
  suppress painting, but a suppressed annotation is never silently
  ignored: it is counted and surfaced. A page carrying content the
  operator cannot see is a fact they are entitled to know. Hidden
  annotations are a recognized document-forensics vector; a tool that
  neither shows them nor mentions them is worse than one that does
  neither loudly.

- **R51 — `/NeedAppearances` is a disclosed condition, never a silent
  auto-generate.** A document setting it true is asserting its field
  appearances are stale. pdfce reports that and regenerates only on
  request. Regenerating on load would rewrite objects the operator never
  touched — a §5 violation dressed as helpfulness, and a
  fuzzy-never-sneaky violation on top.

- **R52 — Redaction's mark and apply are separate operations with
  separate confirmations.** Marking creates a `/Redact` annotation
  (reviewable, undoable, non-destructive, saveable). Applying performs
  the removal (destructive, full rewrite per R35, container decomposition
  per §5.7, irreversible after save per §11.2). Never one button, and
  applying is never a save-time side effect of a mark being present.

---

## 7. Spec prerequisites

| Item | RAG status | Blocks |
|---|---|---|
| §12.5.1–12.5.3 annotation dict, Tables 164/165/166 | **ABSENT** | **Pass 6.0** |
| §12.5.5 appearance streams, `/AS` selection, BBox→Matrix→Rect algorithm | **ABSENT** | **Pass 6.0** |
| §12.5.6.x per-subtype requirements (measured subtypes only) | **ABSENT** | 6.0 display / 6.1–6.2 generation |
| §8.10.2 form XObjects — **write-direction audit** | Exists, built for the read path | **Pass 6.1** |
| §12.7.3.3 variable text (`/DA`, `/Q`, `/DR`, comb, multiline) | **ABSENT** | 6.2, 7 |
| §12.7.1–12.7.4 interactive-form clause tree | **ABSENT** | 7 |
| §7.6 encryption clause tree | **ABSENT** (only `filter__crypt.md`) | 5 |
| ETSI EN 319 142 / RFC 3161 / PKCS#7 | **ABSENT** | 10 |

Two notes that matter more than the table.

**§12.5.5 is the one that decides whether this Pass is correct.** The
placement algorithm is where silent misplacement comes from, and
misplacement is the defect class a self-comparison oracle cannot see.
Source it properly; do not reconstruct it from memory.

**The §8.10.2 write-direction audit is an audit, not a build** — the
same distinction decision 007 drew for §7.5.x. That audit produced a
valuable *negative* result on its own (*"§7.5.8 never mentions predictors
on the write side"*). Expect the same shape here: what does the spec
constrain about a `/BBox` and `/Matrix` you are *emitting* rather than
consuming?

---

## 8. Acrobat-librarian routing

| Bucket | For | Status |
|---|---|---|
| **Comments & markup** | Pass 6.1 | **BLOCKING** |
| Comments & markup — *display/visibility semantics* | Pass 6.0 | recommended, not blocking |
| **Forms (AcroForm)** | Pass 7 | **BLOCKING** |
| **Redaction** | Pass 8 | **BLOCKING** |
| Vector editing (Inkscape) | Pass 9+ | **UNOWNED — assign** |

Pass 6.0 is not blocked for the same reason Pass 3.0 was not: appearance
*display* is spec-governed, and the feature-fidelity rule binds when a
Backlog Acrobat-parity bucket becomes a Pass. Displaying an annotation
correctly is conformance, not parity. But the visibility questions
(Hidden vs NoView on screen and in print, `/Popup`, an `/AS` naming a
missing state) do have Acrobat-specific behavior worth having, and
starting the bucket now removes 6.1's blocker at zero cost.

Capability, behavior, edge cases, limits **only** — never GUI mechanics.
The Redaction dispatch has the highest value in the set, and the highest-
value question in it is the inverse one: **what does Acrobat's apply
demonstrably *not* remove?** That is where pdfce's differentiator lives.

---

## 9. Risks

Full enumeration in the JSON block. The ones that would actually bite:

**X2 — appearance misplacement (the sharpest).** A wrong composition of
`/BBox`, `/Matrix` and `/Rect` renders beautifully in the wrong place, at
the wrong scale, or mirrored. pdfce's self-comparison oracle **cannot**
catch it. Two independent checks, both required: known-geometry synthetic
fixtures, and a pdfium differential. This Pass is the forcing consumer
that finally justifies the Pass 1.1 harness remainder.

**X5 — Pass 6.1 silently falsifies `DocumentView`'s assertion.** The
concrete failure: extract/merge/split from a session carrying authored
appearances reads the *base* buffer for spans pointing into the
*staging* buffer — garbage or a panic, in the code path least likely to
be exercised by an annotation test. R45; and a 6.1 test must extract and
merge from a session containing an authored annotation.

**X6 — content-stream normalization.** R33 at the token level. Emitting
`1.0` as `1`, collapsing whitespace, dropping comments — plausible,
working, wrong, and it passes every structural check. R46's identity gate
catches all of it mechanically, the same way the object-level gate made
R33 self-enforcing.

**X7 — `/Annots` patching hazards.** A shared indirect `/Annots` array
silently annotates every sharing page. And an object-stream-compressed
page object triggers R38 promotion — which is **fixture-covered, not
corpus-covered**, because page objects are uncompressed in all 75 corpus
files holding compressed objects. Build the fixture; the corpus cannot
supply one.

**X8 — appearance resource scoping.** An `/AP` stream's `/Resources` is
its own. This project has *already hit this bug once*: continuation-9's
finding that a per-interpreter font cache is a **correctness**
requirement because `/F1` in a form's own resources is a different font
than the page's `/F1`. Route appearances through the existing form path
rather than writing a second, shorter one — and pin it with a fixture
where page and appearance both define `/F1` differently.

**X11 — certification interaction.** §12.8.2.2's `/P` gradation treats
annotation addition and form filling differently. The machinery shipped
in Pass 3.2 (`signature_impact_of_save`, the `/Reference` →
`/TransformMethod` walk, `CertificationForbidsChange`). It must be
**wired**, not re-derived. Note the existing open residual — `/Info`
edits are not certification-gated — becomes *more* visible here, not
less.

**X13 — my own census is pypdf-sourced and skewed.** R31 binds me. See
§1.2's caveats and acceptance criterion 1.

**X16 — unchanged from W15.** No remote, CI has never run, Passes 0–4
all uncommitted. Pass 3.2 found three pre-existing `ui-strings` false
positives that had sat undetected *because the gate had never actually
run*. Every gate this Pass adds must be green **locally** as a hard
criterion.

---

## 10. Revisit triggers

1. **Pass 6.0's coverage lands materially low for a structural reason** →
   stop and re-decide in a new record (the W14 precedent). Do not weaken
   R43 to make a gate go green.
2. **pdfium shows systematic placement divergence pdfce cannot reconcile
   from the spec** → the §12.5.5 sourcing is wrong or incomplete;
   re-dispatch before writing a heuristic.
3. **The operator states a real redaction need** → Pass 8 promotes ahead
   of Pass 7. Cheap by construction.
4. **The operator's work turns out to be form-*filling* rather than
   markup** → promote Pass 7's fill path ahead of 6.2's markup breadth
   (6.2 is Pass 7's prerequisite either way — this re-orders feature
   surface, not infrastructure).
5. **A real encrypted file blocks actual work** → Pass 5 promotes.
6. **A 6.x Pass hits the three-attempts wall** → switch to Pass 5, the
   designated fallback track.
7. **The operator wants Inkscape-style vector editing sooner** → Pass 9
   promotes directly after Pass 6.1.

---

## 11. Corrections to the record, and housekeeping owed

Surfaced while grounding this decision. None changes the outcome; all get
more expensive with age.

1. **The `SESSION_LOG` continuation-20 entry for Pass 4 does not
   exist** — yet `ROADMAP.md:60` cites *"the SESSION_LOG continuation-20
   entry"* and `ROADMAP.md:73` cites *"`ARCHITECTURE.md` §12
   continuation-20 entry (c)"* as though both do. Under the project's own
   append-only protocol a shipped Pass owes both the same session. File
   it before anything lands on top of Pass 4.
2. **`ARCHITECTURE.md` §4 is materially stale.** It still describes the
   Pass-0 header-probe state, and decisions 001, 004 and 005 each carry
   an unpaid "integrate the design text into §4" obligation. Anything
   Pass 6.x writes there lands on unamended ground. One consolidation
   session, before the annotation model is documented.
3. **The annotation *display* gap is filed nowhere.** "Comments &
   markup" covers authoring. The read-side gap needs its own Backlog
   entry — exactly as text extraction was unfiled before decision 007
   created its bucket.
4. **The Inkscape capability catalog has no owner.** The requirement is
   written into the ROADMAP bucket; the agent roster does not cover it.
5. **Measured answers now exist for two standing open items:** XFA at
   0.08% of organic files, `/SigFlags` at 0.64%. Neither closes its item,
   but both replace an assumption with a number.

**Still open, carried forward from decision 007 and now the oldest item
on the list:** the encrypted-PDF refusal
(`XrefErrorKind::EncryptionUnsupported`) has still never been surfaced to
the operator. The census that was meant to inform it returned three
Passes ago. pdfce declines a category of file every other reader opens,
and the operator has not been told.

## Appendix A — Effective JSON decision block

```json
{
  "decision_id": "008",
  "title": "The next major subsystem after read → write → edit → extract",
  "date": "2026-07-31",
  "status": "Decided",
  "decider": "KenAgent (autonomous-builder), per the ROADMAP standing rule 'KenAgent decision routing'",
  "supersedes": "nothing",
  "amends": "decision 007's 'Pass 6+' placeholder (never a real allocated ID — its contents were already decomposed into Pass 1.1 items 4/6.3 and the Vector-graphics-editing bucket). Decision 007's Pass 5 (Encryption) keeps its ID and its rationale; only its position in the queue moves.",

  "decision": "A — Annotations & markup — but sliced so that the FIRST Pass introduces no authoring capability at all. Pass 6.0 ships annotation APPEARANCE RENDERING (read-side only), because pdfce today draws no annotation at all, does not count them, and that is a measured fidelity gap on 11.6% of the conformance corpus and 32.6% of the operator's own PDFs. Authoring arrives in Pass 6.1 only after the appearance model is proven against real files. Ranking of the six candidates: A >> B > C > E > D > F, with the load-bearing qualification that Pass 6.0 serves A and B JOINTLY (a form field's /Widget IS an annotation with an /AP), and Pass 6.1's content-stream serializer is the shared prerequisite C and E both wait on.",

  "ranking": ["A — Annotations & markup", "B — Interactive forms / AcroForm", "C — Redaction", "E — Vector / content-stream editing (Inkscape parity)", "D — Encryption/decryption", "F — Digital signatures / PAdES"],

  "appearance_stream_generator_question": {
    "asked": "Is the appearance-stream generator its own shared-infrastructure Pass first?",
    "answer": "SPLIT IT. The CONSUMPTION half is its own Pass (6.0) and it is the recommended first Pass. The GENERATION half is NOT a standalone Pass — it ships inside Pass 6.1 with its first consumer.",
    "why_consumption_is_its_own_pass": "It is the exact Pass 3.0 inversion, and it qualifies for the same reason Pass 3.0 did: it has a corpus-wide executable oracle available TODAY (render the /AP form XObjects that 2,914 conformance files and the operator's own library already contain, and compare rasters), it introduces zero editing capability so ARCHITECTURE.md §11.4 does not bind, and it closes a live undisclosed read-side gap rather than adding a speculative feature. It is also the only candidate in the entire slate that a conformance corpus can measure — the operator's brief correctly notes that creation features cannot be corpus-measured; appearance CONSUMPTION is the exception.",
    "why_generation_is_not_its_own_pass": "An appearance generator with no annotation to generate for has no acceptance criteria and would be built blind — precisely the failure decision 007 §3.2 argued against for the writer ('doing the writer before the renderer would have meant building it blind'). Its oracle is 'does the authored annotation display, round-trip, and survive reload', which requires a consumer. Build it in Pass 6.1 with the geometrically simplest annotation set (Ink/Square/Circle/Line/Polygon + quad-point markup — no text), so the generator is exercised without the variable-text problem confounding it.",
    "shared_scope_of_the_generator": [
      "Shared by A, B, and C's redaction-mark step: Form XObject packaging (/BBox, /Matrix, /Resources), /AP /N (and /D, /R, /AS state dictionaries) wiring, the §12.5.5 BBox→/Rect placement algorithm, a content-stream token EMITTER, and an authored-stream representation.",
      "NOT shared, and deliberately NOT in Pass 6.1: variable-text appearance generation (§12.7.3.3 /DA, /Q quadding, comb fields, multiline wrap, /RC rich text). That is Pass 6.2's deliverable and it is the single hardest piece of appearance generation. Putting it in the same Pass as the infrastructure is how this subsystem doubles."
    ]
  },

  "measurement": {
    "methodology": "Read-only census with pypdf 6.7.0, aggregate counts only, nothing copied (LEGAL.md §5 posture, mirroring decision 007's /Encrypt census). Script at C:\\Users\\Ken\\AppData\\Local\\Temp\\pdfce_annot_census.py (temp, not in the repo). Two populations: the full 2,914-file conformance corpus at D:\\Dev\\pdfce\\fixtures\\external, and a seeded random sample (seed 20260731) of 2,500 of 25,203 PDFs discovered under the operator's document store.",
    "conformance_corpus_2914_files": {
      "files_with_annots": 338,
      "files_with_annots_pct": 11.6,
      "annots_total": 429,
      "annots_with_AP": 228,
      "files_with_acroform": 127,
      "files_with_xfa": 4,
      "files_with_sigflags": 6,
      "top_subtypes": {"/Link": 98, "/Widget": 87, "/Circle": 47, "/Popup": 34, "/Highlight": 14, "/3D": 14, "/FileAttachment": 13, "/Screen": 12, "/Movie": 11, "/Line": 9, "/Stamp": 8, "/Text": 6},
      "acroform_field_types": {"/Btn": 39, "/Tx": 36, "/Sig": 7}
    },
    "organic_sample_2500_of_25203": {
      "files_with_annots": 814,
      "files_with_annots_pct": 32.6,
      "files_with_acroform": 753,
      "files_with_acroform_pct": 30.1,
      "annots_total": 55545,
      "annots_with_AP": 43508,
      "annots_with_AP_pct": 78.3,
      "acroform_fields_total": 47868,
      "files_with_xfa": 2,
      "files_with_xfa_pct": 0.08,
      "files_with_sigflags": 16,
      "files_with_sigflags_pct": 0.64,
      "subtypes": {"/Widget": 48781, "/Link": 5180, "/Square": 1067, "/Stamp": 224, "/Popup": 146, "/FileAttachment": 49, "/FreeText": 42, "/Highlight": 37, "/Line": 14, "/Text": 2, "/3D": 1, "/Caret": 1, "/Ink": 1},
      "acroform_field_types": {"/Tx": 47774, "/Btn": 63, "/Sig": 31}
    },
    "honest_caveats": [
      "PER-FILE figures (32.6% annots, 30.1% AcroForm, 11.6% conformance) are robust. PER-ANNOTATION figures are concentration-skewed: 47,774 /Tx fields across 753 files is ~63 fields/file on average, which almost certainly means a minority of form-heavy documents dominate the count. The per-file numbers are the ones to reason from.",
      "The organic population is the operator's own real-world working documents. It is representative of THIS operator, which is the right population for prioritization, and is NOT representative of PDFs in general. State it that way in any user-facing claim.",
      "R31 applies to me as much as to a reference decoder: pypdf's conventions were not independently verified for this census (e.g. inherited /Annots resolution, malformed-array tolerance). The numbers are indicative and MUST be re-measured with pdfce's own machinery before they are used as a Pass gate denominator — the W16 'a gate whose denominator is uncertain cannot report an honest shortfall' discipline.",
      "The 4 conformance files reported as /XFA and the 2 organic ones are a MEASURED answer to the standing CLAUDE.md open item 'XFA scope — verify before committing engineering time': 0.08% of the operator's own files. That is not a mandate to build XFA. It stays Backlog."
    ]
  },

  "structural_findings_that_drove_the_decision": [
    {
      "id": "F1",
      "finding": "pdfce renders NO annotations at all, and does not count them. `crates/pdfce-render/src` contains zero /Annots handling — the word 'annotation' appears only in doc comments describing what the Form-XObject machinery will EVENTUALLY be used for (interpret.rs:143, interpret.rs:452). There is no Diagnostics counter for it (Diagnostics is at interpret.rs:174).",
      "significance": "This is not merely a missing feature — it is an UNDISCLOSED shortfall, which is a departure from the R20/R27 posture the project holds everywhere else. A page carrying a highlight, a stamp, or 40 filled form fields renders as though the page were blank of them, with no diagnostic. Every other named gap in pdfce (Type 3 fonts, `sh`, /SMask, JPX enumerated colour spaces) is counted and named. This one is silent."
    },
    {
      "id": "F2",
      "finding": "The Form-XObject execution machinery an appearance stream needs is ALREADY BUILT. §8.10.1's five-step procedure, the fresh-Interpreter-over-cloned-GraphicsState decision, the object-number-keyed cycle guard, MAX_XOBJECT_DEPTH = 64, and the resource-scoping correctness fix all shipped in the Pass 1.1 XObject slice (ARCHITECTURE §12 continuation-9). An /AP /N stream IS a form XObject.",
      "significance": "Pass 6.0 is assembly against an existing contract, exactly as decision 007 §1.1 argued for the writer. The new work is annotation-dictionary walking, appearance SELECTION (§12.5.5), the BBox/Matrix→/Rect placement algorithm, and flag handling (§12.5.3) — not a rendering engine."
    },
    {
      "id": "F3",
      "finding": "There is NO content-stream writer in the codebase, and `Stream` cannot represent authored bytes. `object.rs:246` defines `Stream { dict: Dict, data_span: ByteSpan }` — data is a span into a retained source buffer, never owned. `writer/serialize.rs:182`'s `stream_data(stream, source) -> Option<&[u8]>` is the whole stream-emission story. Pass 3.0's headline result is literally '0 objects re-serialized under SaveOptions::identity()'.",
      "significance": "The first subsystem that authors an appearance stream is the first content-stream writer this project has ever had. That is a genuinely new capability class and it must be named as such rather than absorbed silently into a feature Pass."
    },
    {
      "id": "F4",
      "finding": "The precedent AND the tripwire both already exist, written down. `pageops/assemble.rs` invented a STAGING BUFFER for cross-document stream copies ('every copied stream's raw, still-filter-encoded payload is appended to a staging buffer and the copy's span is repointed at it… the existing serializer works unmodified, because \"a value tree plus the buffer its spans index into\" is precisely its interface'). And `DocumentView`'s doc comment states verbatim: 'no Pass 3.2 session edit introduces stream bytes the base buffer does not already hold; a Pass that changes that must revisit this type, and the assertion is written down here so it cannot be forgotten.'",
      "significance": "Pass 6.1 is the Pass that changes that. The engineer inherits both a working design pattern and an explicit obligation to discharge the assertion rather than quietly falsify it. Extract/merge/split from a session carrying authored appearances is the concrete thing that breaks if this is missed."
    },
    {
      "id": "F5",
      "finding": "Everything a text-bearing appearance generator needs for the Base-14 case is ALREADY IN pdfce-core, GUI-free: `fontdata::std14_width`, `std14_descriptor`, `std14_builtin_encoding`, `encoding_glyph_name` (fontdata/mod.rs).",
      "significance": "Pass 6.2's variable-text generator does not require a render dependency in core, and does not violate §3. It also does not require shaping — R17 already anticipated this: 'harfrust may only ever enter a future text-AUTHORING path.' Pass 6.2 is that path's first appearance, and it should be built WITHOUT harfrust (Base-14 + embedded-font widths, no complex script support) and say so."
    },
    {
      "id": "F6",
      "finding": "Annotation authoring touches NO page content stream. An annotation is a separate indirect object referenced from the page's /Annots array; its appearance is a separate form XObject.",
      "significance": "This is the single strongest architectural argument for A over C and E. Annotation authoring is the minimal-diff invariant's BEST CASE — it is literally the motivating example ARCHITECTURE §5 uses ('adding one comment to a 400-page contract does not perturb the other 399 pages' bytes'). It extends Pass 3.1's mutation model into content-bearing territory with new objects and one array patch, and it lands in pure incremental-save territory. C and E both require content-stream SURGERY, a strictly larger and riskier step onto ground that has never been walked."
    }
  ],

  "pass_sequence": [
    {
      "id": "Pass 6.0",
      "name": "Annotation & widget appearance rendering (read-side)",
      "editing_capability": "none — deliberate",
      "why_here": "Same inversion as Pass 3.0, for the same three reasons: (1) it makes a load-bearing model executable against a real corpus BEFORE any code exists that could author into it; (2) it introduces no mutation, so §11.4's undo obligation does not bind and the largest-risk subsystem is not fused to a second one; (3) it is the only candidate on the slate a conformance corpus can measure. It also closes an undisclosed gap (F1) and serves candidates A and B jointly (a /Widget is an annotation).",
      "acrobat_rag_required": false,
      "acrobat_rag_recommended": true,
      "spec_blockers": ["§12.5.1-12.5.3 annotation dictionary + Tables 164/165/166 (flags)", "§12.5.5 appearance streams, including the BBox→Matrix→/Rect placement algorithm and appearance-state (/AS) selection", "§12.5.6.x per-subtype appearance requirements for the subtypes actually present in the corpus"]
    },
    {
      "id": "Pass 6.1",
      "name": "Authored streams + content-stream serializer + markup annotation authoring",
      "editing_capability": "Ink, Square, Circle, Line, Polygon/PolyLine, and the quad-point markup family (Highlight/Underline/StrikeOut/Squiggly). NO text-bearing annotations.",
      "why_here": "First authoring Pass. Delivers three things at once because they cannot be separated honestly: an authored-stream representation (session staging buffer, per F4), a content-stream token EMITTER (the project's first), and the appearance generator. The annotation set is chosen to be purely geometric so the infrastructure is exercised without variable text confounding it. §11.4 is already discharged by Pass 3.1 — this Pass inherits the EditSession command log rather than owing one (§11.3 pre-assigns annotations to exactly that model and forbids a second parallel undo system).",
      "key_gate": "CONTENT-STREAM IDENTITY: parse → re-emit → byte-compare every content stream in the loadable corpus, plus raster identity. This is Pass 3.0's move applied one level down, it is nearly free because content.rs already carries per-token byte spans, and it is what de-risks BOTH C and E. Fold it in here rather than making it a separate Pass.",
      "acrobat_rag_required": true,
      "acrobat_rag_bucket": "Comments & markup"
    },
    {
      "id": "Pass 6.2",
      "name": "Text-bearing annotations + variable-text appearance generation",
      "editing_capability": "FreeText, Text (sticky note) + /Popup, Stamp (standard stamp set)",
      "why_here": "Builds §12.7.3.3 variable text (/DA parsing, /Q quadding, multiline wrap) — the hardest single piece of appearance generation, and the piece Pass 7 (forms) cannot ship without. Isolating it means forms inherits a proven generator instead of building one under feature pressure. /RC rich text is an explicit non-goal.",
      "acrobat_rag_required": true
    },
    {
      "id": "Pass 7",
      "name": "Interactive forms / AcroForm (candidate B)",
      "why_here": "Ranked second overall and MEASURED second: 30.1% of the operator's own PDFs carry an /AcroForm, and /Tx is 99.8% of fields. It arrives here rather than first because its display half is delivered by Pass 6.0 and its appearance half by Pass 6.2 — sequencing it after them converts the largest single risk (appearance regeneration) from new work into reuse. Scope: field model (/FT, /Ff, /V, /DV, field-vs-widget merge, hierarchy + /T name paths), fill, appearance regeneration, /NeedAppearances handling, flatten-to-static, field auto-detection as a HINT only (fuzzy-never-sneaky). Gates FieldMDP work.",
      "acrobat_rag_required": true
    },
    {
      "id": "Pass 8",
      "name": "Redaction (candidate C)",
      "why_here": "The standing R35 obligation and the one truly destructive operation. Placed here because it needs a capability class nothing before it has: content-stream SURGERY (removing operators and slicing glyph runs out of Tj/TJ), which sits on top of Pass 6.1's serializer + identity gate; plus CONTAINER DECOMPOSITION, which the Pass 3.1 correction proved is mandatory and unsupported today (save_full re-emits object streams verbatim with zero promotions BY DESIGN, so a redacted object's old value survives inside its untouched container under BOTH save modes). Its search input and its verification oracle are the text extraction that just shipped. Its mark phase is a /Redact annotation with an /RO overlay — i.e. it consumes Pass 6.1's authoring, which is why marking and applying are cleanly separable into the two-phase model.",
      "promotion_trigger": "Promotes AHEAD of Pass 7 the moment the operator states a real redaction need (legal / tax / disclosure work). The promotion is cheap by construction: Pass 6.1 already delivers its content-stream prerequisite and Pass 6.1's /Redact-capable authoring delivers its mark phase. Only the surgery, container decomposition, and image redaction remain.",
      "acrobat_rag_required": true
    },
    {
      "id": "Pass 9+",
      "name": "Vector / content-stream editing — Inkscape parity (candidate E), sliced",
      "why_here": "The project's distinctive purpose, and it is NOT being deferred to last — its load-bearing foundation (content-stream serializer + corpus identity gate) lands SECOND, in Pass 6.1, and its selection/hit-test interaction model is prototyped in Pass 6.0/6.1 (clicking an annotation to select it is the same problem as clicking a path). It is placed after redaction because it is the largest candidate by a wide margin, it must itself be sliced, and it benefits most from landing on infrastructure that a smaller, trust-critical consumer has already proven. Decision 007 already folded shading patterns and transparency groups / soft masks INTO this bucket. Suggested slicing: (a) object model + hit-test + selection; (b) transforms / z-order / group-ungroup; (c) node/Bézier editing; (d) boolean path ops; (e) gradients + shading + transparency; (f) OCG layers; (g) text-to-path.",
      "acrobat_rag_required": false,
      "note": "Inkscape is GPL-2.0-or-later — behavioral reference ONLY, never a dependency or code source (existing binding note). A capability catalog for Inkscape is owed before this bucket becomes a Pass, and it is NOT the acrobat-librarian's remit as currently written."
    },
    {
      "id": "Pass 5 (ID retained from decision 007)",
      "name": "Encryption — decrypt all handlers; encrypt-on-save AES-128/256 only (candidate D)",
      "why_here": "Keeps its decision-007 ID and rationale unchanged; only its queue position moves. It measured 0.67% of the organic corpus and its promotion trigger was explicitly NOT met. It gates exactly one thing pdfce cannot otherwise do (open encrypted files) plus candidate F's crypto primitives. §7.6 remains the single largest spec gap on the slate and requires a full spec-librarian corpus-building session before a line of code. It is the DESIGNATED FALLBACK/INTERLEAVE TRACK for this decision, exactly as Pass 4 was for decision 007: it is independent of the entire appearance/authoring arc, so it is the switch target if a 6.x Pass hits the three-attempts wall.",
      "new_interaction_introduced_by_this_decision": "Pass 6.1 authors STRINGS into documents (/Contents, /T, /Subj). Authoring into an encrypted document requires per-object encryption of those strings. Until Pass 5 ships, annotation authoring on an encrypted file must be refused BY NAME (R27 posture), not attempted."
    },
    {
      "id": "Pass 10",
      "name": "Digital signatures / PAdES (candidate F)",
      "why_here": "Last. Depends on Pass 5 for crypto primitives, is the heaviest external-spec-sourcing item on the slate (ETSI EN 319 142 B-B/B-T/B-LT/B-LTA + RFC 3161 + PKCS#7/CMS), and measured 0.64% of the operator's organic files carry /SigFlags. Note that pdfce already has the READ half disproportionately far along — signature.rs is 810 lines, iso32000__s__12.8.md is 689 lines, and §5.5's 'pdfce never re-serializes a signature dictionary, even identically' is already an enforced structural rule. Signing is the missing half, and the incremental-update-based signing model it requires is already the default save mode."
    }
  ],

  "first_pass_scope": {
    "pass_id": "Pass 6.0",
    "name": "Annotation & widget appearance rendering (read-side)",
    "deliverables": [
      "pdfce-core: an annotation model — walk a page's /Annots, resolve each annotation dictionary, expose /Subtype, /Rect, /F flags, /AP (/N, /R, /D) and /AS. Placement per the 'core decodes and models, render paints' axis (R26's precedent, decision 005): core MODELS the annotation and selects the appearance stream; render PAINTS it. Core must not paint and must not decide colour.",
      "pdfce-render: paint the selected /AP /N form XObject through the EXISTING §8.10.1 form-execution path (fresh Interpreter over a cloned GraphicsState, object-number-keyed cycle guard, MAX_XOBJECT_DEPTH). No new rendering engine.",
      "The §12.5.5 placement algorithm implemented as spec'd and cited: transform /BBox by /Matrix, take the bounding box of the result, compute the matrix A that maps that box to /Rect, and render with A × /Matrix. A degenerate (zero-width or zero-height) transformed box is a NAMED refusal, not a divide-by-zero.",
      "Appearance selection: /AP /N may be a stream OR a sub-dictionary keyed by appearance state; when it is a sub-dictionary, /AS selects. Missing /AS with a multi-entry /N sub-dictionary is a named, counted diagnostic — never a guessed pick.",
      "§12.5.3 flag handling (Table 165): Hidden and NoView annotations are NOT painted; Print is honored for a future print path; /Popup subtype is never painted as page content. Every suppression is COUNTED (R50).",
      "Annotations with no /AP at all: counted by subtype, never synthesized. This counter is the measured demand signal for Passes 6.1/6.2/7 — it is the number that says how much appearance GENERATION real files actually need.",
      "Diagnostics extension (new counters, appended to the CLI stable-line contract per the standing convention, never reordered): annotations_total, annotations_painted, annotations_without_ap (by subtype), annotations_hidden, annotations_appearance_state_missing, annotations_widget, need_appearances_documents.",
      "GUI: an annotation-visibility toggle (show/hide annotations) — display-only, no selection, no hit-testing, no editing; plus the new counters folded into the existing diagnostics surfaces. Dispatch pdfce-ui-specialist: this is a new visibility control and it must not become a fourth ad-hoc placement pattern after Pass 3.2's rail/dock binary and Pass 4's toolbar-menu-button.",
      "pdfce-cli: `render-page` honors annotations with an explicit flag to suppress them (so the pre-6.0 raster is still reproducible for comparison), plus a `list-annotations` subcommand emitting the per-page inventory in the locale-invariant stable-line format.",
      "tools/corpus-report extended with the annotation counters; a `tools/annot-corpus-check.py` sweep or equivalent that RE-MEASURES this decision's census numbers with pdfce's own machinery and pins the Pass baseline.",
      "A fuzz target covering annotation-dictionary walking + appearance selection (11th target), including cyclic /AP references, /Rect degenerate and inverted, /AS naming a missing state, and an /AP /N that is neither stream nor dictionary."
    ],
    "acceptance_criteria": [
      "The census is RE-RUN with pdfce's own tooling and its numbers replace this decision's pypdf figures as the pinned baseline BEFORE the gate is written (W16 discipline: a gate whose denominator is uncertain cannot report an honest shortfall). Expect ~338 conformance files with annotations and ~228 with /AP; if pdfce's own count differs materially from pypdf's, that discrepancy is itself a finding to run down, not to average away.",
      "Every annotation carrying an /AP /N that resolves to a form XObject is painted, or is refused by a NAMED diagnostic. Target 100% of resolvable appearances; any shortfall is enumerated BY FILE AND BY REASON in the R20 counted-shortfall tradition, never rounded away.",
      "Raster differential against pdfium (pypdfium2) on the annotation-bearing subset. THIS PASS IS WHAT FINALLY JUSTIFIES the Pass 1.1 reference-renderer pixel-parity harness remainder, because appearance PLACEMENT (BBox × Matrix → Rect) is a silent-wrongness class that a self-comparison oracle structurally cannot catch — pdfce agreeing with pdfce proves nothing about whether the stamp is in the right place. Decision 006 §3.2's ad-hoc pypdfium2 comparison is the tooling precedent. Do not claim the Pass 1.1 remainder is CLOSED unless the harness is actually generalized; if it is only run on the annotation subset, say exactly that.",
      "Synthetic fixtures with KNOWN geometry pin the placement algorithm from both directions: non-origin /BBox, rotated /Matrix, scaling /Matrix, /BBox larger than /Rect, /BBox smaller than /Rect, inverted /Rect (x2<x1), degenerate /BBox. Generated by a tools/gen-annot-fixtures.py in the established pattern, with a PROVENANCE file (LEGAL §5).",
      "Hidden/NoView annotations are provably not painted (a fixture whose hidden annotation would be visually obvious if painted), and are counted.",
      "/Popup annotations are provably not painted as page content.",
      "Pass 3.x and Pass 4 gates UNMOVED (R34). Note explicitly: the writer round-trip raster oracle is a SELF-comparison (same build both sides), so painting annotations does not perturb it — but any PINNED reference raster does change, and the re-baselining must be stated in the Pass entry rather than discovered.",
      "veraPDF §6.1.12 implementation-limits suite run against every new guard (annotation-count ceiling, /AP recursion depth) — the standing rule, with two prior intuition-guard incidents on record. Report the MEASURED headroom, per the Pass 3.2/Pass 4 precedent, not just a pass.",
      "cargo fmt --check and cargo clippy -- -D warnings clean; cargo tree -p pdfce-core and -p pdfce-render show no GUI dependency on host AND x86_64-pc-windows-msvc; wasm32 check clean; --duplicates guard clean; ui-strings R1 gate clean; no-network clean.",
      "No new dependencies expected. If any is proposed, LEGAL §6 classification first and cargo-about regeneration in-Pass."
    ],
    "explicit_non_goals": [
      "NO authoring of any kind. No annotation creation, no editing, no deletion, no /AP generation. Therefore no undo obligation (§11.4 does not bind a Pass with no mutation).",
      "NO appearance generation for annotations that lack an /AP. They are COUNTED. Generating one here would be the exact silent-guess failure R29/R30 exist to prevent, and it would pre-empt Pass 6.1's design.",
      "NO /NeedAppearances honoring beyond counting the documents that set it. Regenerating field appearances is Pass 7.",
      "NO hit-testing, NO selection, NO annotation panel, NO comment list/summary/export. Display only.",
      "NO /RichMedia, /3D, /Movie, /Sound, /Screen playback. Their static /AP /N (a poster frame) paints like any other appearance; the media itself is out of scope permanently absent a new decision record.",
      "NO link ACTIVATION (/Link /A actions). Links paint their appearance (most have none) and nothing else. Navigation is a separate feature with its own security posture — R12/R13 govern anything that would touch the network or launch a process.",
      "NO print-specific rendering path.",
      "NO content-stream writing of any kind. That is Pass 6.1 and it is the line this Pass must not cross."
    ],
    "parallel_cheap_tasks": [
      "Dispatch pdfce-spec-librarian for the §12.5 clause tree (see spec_prerequisites) — BLOCKING, must return before code.",
      "Dispatch pdfce-acrobat-librarian for the 'Comments & markup' bucket. NOT blocking for 6.0 (appearance display is spec-governed, and the writer precedent in decision 007 §7 establishes that infrastructure Passes are not bound by the feature-fidelity rule) but BLOCKING for 6.1, so starting it in parallel with 6.0 costs nothing and removes 6.1's blocker.",
      "Re-run the annotation census with pdfce's own tooling (see acceptance criterion 1)."
    ]
  },

  "standing_rules": [
    "R43 — pdfce renders an annotation from its appearance stream, or not at all. §12.5.5 makes the /AP /N normative when present. pdfce NEVER synthesizes a look for an annotation at display time — not for a /Widget with an /MK but no /AP, not for a /Square with a /IC but no /AP, not ever. An annotation without a usable appearance is COUNTED BY SUBTYPE and not painted. This is the display-side sibling of R29's null rule, and it exists because a synthesized appearance is a plausible, working, WRONG picture of a document — the operator would see something no other reader shows them, with no way to tell it was invented.",
    "R44 — Any appearance pdfce generates is written into the file, never rendered privately. When Pass 6.1+ generates an appearance, it is emitted as a real /AP form XObject in the saved document. pdfce never carries a pdfce-only rendering of a pdfce-authored annotation, because that produces a document that looks correct in pdfce and blank everywhere else — the single worst outcome for an editor whose whole purpose is producing files other tools consume.",
    "R45 — Authored stream bytes live in an explicit session staging buffer, and Stream keeps its span model. `Stream { dict, data_span }` indexing into a retained buffer is what §5's verbatim re-emission is built on; giving Stream an owned-bytes variant would fork the verbatim path at every match site. The first subsystem authoring new stream bytes extends EditSession with a staging buffer and repoints spans at it — the `pageops::assemble` pattern, generalized. `DocumentView`'s written-down assertion ('no Pass 3.2 session edit introduces stream bytes the base buffer does not already hold; a Pass that changes that must revisit this type') is DISCHARGED by amending the type, never by deleting the sentence.",
    "R46 — The content-stream serializer is proven by an identity gate before it authors anything. Parse → re-emit → byte-compare every content stream in the loadable corpus, plus raster identity, as an acceptance criterion of the Pass that introduces the serializer. Same inversion as Pass 3.0, one level down, and nearly free because content.rs already carries per-token byte spans. Every later Pass touching the serializer re-runs it (the R34 pattern).",
    "R47 — An annotation edit never touches a page content stream. Annotation authoring adds indirect objects and patches the page's /Annots array; the page's content stream stays byte-verbatim. Any annotation-adjacent operation that WOULD rewrite page content (flatten, burn-in, apply-redaction) is a separate, explicitly named, separately confirmed operation — never a side effect of annotating.",
    "R48 — Flatten is destructive and discloses what incremental save cannot undo. Flattening an annotation or form field into page content destroys the editable object. Under incremental save the pre-flatten annotation SURVIVES in the prior revision and is trivially recoverable — the same structural property R35 names for redaction, in a different costume. Flatten must therefore disclose the save-mode consequence explicitly and offer the full rewrite, and the flatten Pass owes a test that greps the saved bytes. Flatten is never a save-time side effect, never a default, and never silent.",
    "R49 — A widget is an annotation first. The AcroForm field model layers ON TOP of the annotation model; there is exactly one appearance pipeline and one /AP resolution path in pdfce, shared by markup annotations and form fields. A second, forms-only appearance pipeline is forbidden — it is the same failure mode §11.3 already forbids for undo ('don't invent a second, parallel undo system').",
    "R50 — Hidden is honored AND counted. §12.5.3 Table 165's Hidden and NoView flags suppress painting. But a suppressed annotation is never silently ignored: it is counted and surfaced, because a page carrying content the operator cannot see is a fact they are entitled to know. Hidden annotations are a recognized document-forensics vector; a tool that neither shows them nor mentions them is worse than one that does neither loudly.",
    "R51 — /NeedAppearances is a disclosed condition, never a silent auto-generate. A document setting /NeedAppearances true is asserting its field appearances are stale. pdfce reports that, and regenerates only when the operator asks. Regenerating on load would rewrite objects the operator never touched — a §5 violation dressed up as helpfulness, and a fuzzy-never-sneaky violation on top.",
    "R52 — Redaction's mark and apply are separate operations with separate confirmations. Marking creates a /Redact annotation (reviewable, undoable, non-destructive, saveable). Applying performs the removal (destructive, forces full rewrite per R35, requires container decomposition per the §5.7 correction, irreversible after save per §11.2). They are never one button, and applying is never a save-time side effect of a mark being present."
  ],

  "spec_prerequisites": [
    {
      "item": "§12.5.1–§12.5.3 — annotation dictionary, Table 164 (entries common to all annotation dictionaries), Table 165 (annotation flags), Table 166 (appearance-related entries)",
      "status": "ABSENT from the RAG. D:\\Dev\\Rag-Specialized\\PDF_Spec\\iso32000\\ has no 12.5 file. §12.8 (signatures, 689 lines) is the only clause-12 content present.",
      "blocking": "Pass 6.0"
    },
    {
      "item": "§12.5.5 — appearance streams: the /AP (/N, /R, /D) structure, appearance SUB-DICTIONARIES and /AS state selection, and the BBox→Matrix→Rect placement algorithm in full (including the degenerate-box case and what a conforming reader does with it)",
      "status": "ABSENT. This is the single most load-bearing missing clause for the Pass — the placement algorithm is where silent misplacement comes from.",
      "blocking": "Pass 6.0"
    },
    {
      "item": "§12.5.6.x — per-subtype requirements for the subtypes actually measured in the corpora: Link, Widget, Square/Circle, Line, Polygon/PolyLine, Highlight/Underline/StrikeOut/Squiggly (quad points), Text, FreeText, Stamp, Ink, Popup, FileAttachment, Redact",
      "status": "ABSENT.",
      "blocking": "Pass 6.0 (display semantics), Pass 6.1/6.2 (generation semantics)",
      "note": "Scope the 6.0 dispatch to what DISPLAY requires; a second, deeper dispatch covers what GENERATION requires. Do not build the whole clause tree speculatively."
    },
    {
      "item": "§8.10.2 — form XObjects: /BBox and /Matrix semantics on the WRITE side",
      "status": "§8.10 EXISTS but was built for the read path in Pass 1.1. Needs the same write-direction audit treatment decision 007 §7 applied to §7.5.x — a negative result there ('§7.5.8 never mentions predictors on the write side') was worth the dispatch on its own.",
      "blocking": "Pass 6.1"
    },
    {
      "item": "§12.7.3.3 — variable text: /DA default appearance strings, /Q quadding, /DR default resources, comb fields, multiline",
      "status": "ABSENT.",
      "blocking": "Pass 6.2 and Pass 7"
    },
    {
      "item": "§12.7.1–§12.7.4 — the interactive-form clause tree: /AcroForm dictionary, field dictionaries, field/widget merge, /Ff flag bits, field hierarchy and /T name paths, /NeedAppearances",
      "status": "ABSENT.",
      "blocking": "Pass 7"
    },
    {
      "item": "§7.6 encryption — the entire clause tree",
      "status": "ABSENT (only filters/filter__crypt.md, which is the /Crypt FILTER). Unchanged from decision 007.",
      "blocking": "Pass 5",
      "new_note": "Pass 6.1 raises a NEW dependency on this: authoring annotation STRINGS into an encrypted document needs per-object encryption. Until Pass 5, annotation authoring on an encrypted file is refused by name."
    },
    {
      "item": "ETSI EN 319 142 (PAdES B-B/B-T/B-LT/B-LTA), RFC 3161, PKCS#7/CMS",
      "status": "ABSENT.",
      "blocking": "Pass 10"
    }
  ],

  "acrobat_librarian_routing": [
    {"bucket": "Comments & markup", "needed_for": "Pass 6.1 (BLOCKING) and Pass 6.2", "questions": "Which markup annotation types does Acrobat Pro actually author, with what default appearance (colour, opacity, border width, line ending styles)? What does its ink smoothing do? What is the standard stamp set? What are its reply-thread and comment-summary/export capabilities and their limits? What does it do with an annotation that has no /AP? Behavior and limits ONLY — never its GUI mechanics."},
    {"bucket": "Comments & markup — display/visibility semantics", "needed_for": "Pass 6.0 (RECOMMENDED, not blocking)", "questions": "How does Acrobat treat Hidden vs NoView vs Print flags on screen and in print? Does it render /Popup as page content (no) and under what circumstance does it show one? What does it do with an /AS naming a state absent from the /N sub-dictionary?"},
    {"bucket": "Forms (AcroForm)", "needed_for": "Pass 7 (BLOCKING)", "questions": "Field-type coverage and limits; /NeedAppearances behavior on open; auto-detection behavior and its false-positive posture; flatten semantics and what flatten destroys; calculation/validation script scope (and pdfce's own posture on embedded JavaScript, which is a SECURITY decision that must be made explicitly and probably answered 'never execute'); field-name collision handling."},
    {"bucket": "Redaction", "needed_for": "Pass 8 (BLOCKING)", "questions": "Mark-vs-apply two-phase model; what Acrobat's apply actually removes (text, images, vector, annotations, metadata, bookmarks, attachments, hidden layers); search-and-redact behavior; redaction-code overlay text; what it warns about; what it demonstrably does NOT remove — that last one is where pdfce's differentiator lives, and it is a capability question, not a GUI question."},
    {"bucket": "Vector graphics editing (Inkscape parity)", "needed_for": "Pass 9+", "note": "NOT the acrobat-librarian's remit as its agent file is written. The existing binding note in ROADMAP already says this bucket needs 'a feature-parity catalog of Inkscape's editing capabilities — capability/behavior only, never its GUI mechanics — same discipline as the Acrobat Features RAG.' That catalog has no owner. Assign one (a new sibling RAG, or an explicit extension of the acrobat-librarian's remit) BEFORE the bucket is scoped, and keep the GPL-2.0-or-later behavioral-reference-only rule loud."}
  ],

  "risks": [
    {"id": "X1", "risk": "Painting annotations changes rendered output on ~11.6% of the corpus, and R34 says prior gates stay UNMOVED.", "mitigation": "The writer round-trip raster oracle is a SELF-comparison (same build renders both sides), so it is structurally unaffected. Any PINNED reference raster does change and must be re-baselined deliberately, with the re-baselining stated in the Pass entry — never a quiet regeneration of expected images. The CLI suppress-annotations flag keeps the pre-6.0 raster reproducible for A/B comparison."},
    {"id": "X2", "risk": "APPEARANCE MISPLACEMENT — the sharpest silent-wrongness class in this Pass. A wrong composition of /BBox, /Matrix and /Rect yields an annotation that renders beautifully in the wrong place, at the wrong scale, or mirrored. pdfce's self-comparison oracle CANNOT catch it: pdfce agreeing with pdfce says nothing about whether the stamp is where Acrobat puts it.", "mitigation": "Two independent checks, both required. (1) Synthetic fixtures with known geometry, pinned from both directions (non-origin BBox, rotated/scaling Matrix, BBox larger and smaller than Rect, inverted Rect, degenerate BBox). (2) A pdfium raster differential on the annotation-bearing subset — this Pass is the forcing consumer that finally justifies the Pass 1.1 reference-renderer harness remainder. Do not claim that remainder CLOSED unless the harness is genuinely generalized."},
    {"id": "X3", "risk": "Hidden/NoView flags ignored — pdfce paints content Acrobat hides. The inverse is worse: honoring them without counting means pdfce silently withholds content that IS in the file, which is a document-forensics failure.", "mitigation": "R50: honor AND count, both tested with a fixture whose hidden annotation would be visually unmistakable if painted."},
    {"id": "X4", "risk": "/Popup painted as page content — the classic annotation bug. Every sticky note's popup box paints over the page, and it looks deliberate.", "mitigation": "Explicit non-goal, explicit test. 34 /Popup annotations exist in the conformance corpus and 146 in the organic sample, so the corpus sweep catches it immediately if the rule is wrong."},
    {"id": "X5", "risk": "Pass 6.1 authors stream bytes and silently falsifies `DocumentView`'s written assertion. Concrete failure: extract/merge/split from a session carrying authored appearances reads the base buffer for spans that point into the staging buffer — producing garbage stream data or a panic, in the one code path (pageops) least likely to be exercised by an annotation test.", "mitigation": "R45. The assertion is DISCHARGED by amending the type, never by deleting the sentence. A Pass 6.1 test must extract and merge from a session containing an authored annotation and assert the appearance survives byte-exact."},
    {"id": "X6", "risk": "Content-stream serializer normalization — R33 at the token level. Re-emitting `1.0` as `1`, collapsing whitespace, dropping comments, or reordering an inline-image dictionary produces a plausible, working, wrong content stream, and the resulting file passes every structural check.", "mitigation": "R46's corpus-wide content-stream identity gate catches all of it mechanically, because normalization shows up as a byte diff across hundreds of files the moment it is introduced. This is the same mechanism that made R33 self-enforcing for the object writer."},
    {"id": "X7", "risk": "/Annots array patching hazards: the array may be an indirect object SHARED between pages (patching it silently annotates every sharing page), it may be absent (must be created), or the page object may be compressed (R38 promotion, which is fixture-covered but NOT corpus-covered — page objects are uncompressed in all 75 corpus files holding compressed objects, so the corpus will not exercise this).", "mitigation": "Detect sharing and copy-on-write the array, counted and named. Build an object-stream-compressed page fixture specifically — the corpus cannot supply one, which is exactly why R38's coverage gap is already documented as visible rather than closed."},
    {"id": "X8", "risk": "Appearance-stream resource scoping. An /AP stream's /Resources is its own; assuming the page's resource dictionary is in scope is a §7.8.3 violation that produces the wrong font or the wrong colour space — and it is a bug this project has ALREADY HIT ONCE in the form-XObject path (continuation-9 decision (a): a per-interpreter font cache is a correctness requirement, because /F1 in a form's own /Resources is a different font than the page's /F1).", "mitigation": "The existing form-execution path already gets this right. Route appearances through it rather than writing a second, shorter path — and pin it with a fixture where the page and the appearance both define /F1 as different fonts."},
    {"id": "X9", "risk": "Pass 6.2's /DA is a content-stream FRAGMENT inside a PDF string, referencing font names that must resolve in /DR. Parsing it with an ad-hoc scanner instead of the real content lexer is how divergent-parser bugs are born.", "mitigation": "Reuse the content lexer over the string's bytes. Fonts unresolvable in /DR are a named refusal, not a substitution — a substituted font silently changes field text metrics and therefore what the operator sees versus what everyone else sees."},
    {"id": "X10", "risk": "Authoring into an encrypted document. Annotation /Contents, /T and /Subj are strings, which are encrypted per object. Writing them plaintext into an encrypted file produces a document that opens and shows mojibake — plausible, working, wrong.", "mitigation": "Refuse annotation authoring on an encrypted document BY NAME until Pass 5 ships (R27 posture). The R37 object-encoder seam already exists in the serializer, so the eventual fix is a plug-in, not a retrofit."},
    {"id": "X11", "risk": "Certification/DocMDP interaction: adding an annotation to a certified document may be forbidden by /DocMDP /P, and adding a form field is forbidden at a stricter level than adding a comment (§12.8.2.2's /P 1/2/3 gradation treats annotation addition and form filling differently).", "mitigation": "The machinery already exists and shipped in Pass 3.2 — signature_impact_of_save(mode), the /Reference → /TransformMethod walk, EditError::CertificationForbidsChange. It must be WIRED to annotation and field mutations, not re-derived. Note the existing open residual: /Info edits are not yet certification-gated, and this Pass makes that inconsistency more visible, not less."},
    {"id": "X12", "risk": "Scope creep — 'while we're in annotations' pulls in forms, redaction marks, image stamps, rich text (/RC), reply threads, comment export, link activation, or media playback. Any one of them doubles the Pass.", "mitigation": "The explicit_non_goals list is binding. Pass 6.0 displays annotations or it does not ship."},
    {"id": "X13", "risk": "This decision's census is pypdf-sourced and organically skewed. R31 says a reference tool is evidence only after its own conventions are verified, and I did not verify pypdf's. The per-annotation counts are additionally concentration-skewed (~63 /Tx fields per form-bearing file strongly implies a minority of documents dominate).", "mitigation": "Acceptance criterion 1: re-measure with pdfce's own machinery and pin THAT as the baseline. Report the per-file figures as robust and the per-annotation figures as skewed. If pdfce's count differs materially from pypdf's, run the discrepancy down — it is a finding either way."},
    {"id": "X14", "risk": "A visibility toggle is the fourth GUI placement pattern in three Passes (rail, Tools dock, toolbar menu button, now a view control). Placement conventions are drifting one Pass at a time.", "mitigation": "Dispatch pdfce-ui-specialist and ask specifically for a placement TAXONOMY, not just a placement — the project is now at the point where the accumulated conventions should be written down once rather than re-decided per Pass."},
    {"id": "X15", "risk": "R48's flatten leak is a NEW instance of the R35 class that nobody has named until now, and it will be discovered during Pass 7 under feature pressure if it is not written down first.", "mitigation": "R48, filed now, before the Pass that needs it exists — the same move R35 made for redaction one Pass ahead of need."},
    {"id": "X16", "risk": "Unchanged from decision 007 W15: there is still no git remote, CI has never run, and every Pass from 0 through 4 is UNCOMMITTED. Pass 3.2 found three pre-existing ui-strings false positives that had sat undetected precisely because the gate had never actually run.", "mitigation": "Every gate this Pass adds must be runnable and green LOCALLY as a hard acceptance criterion, never contingent on CI. Establishing a remote remains gated on LEGAL.md §1 and is the operator's call, not the engineer's."}
  ],

  "revisit_triggers": [
    "Pass 6.0's measured appearance-render coverage lands materially below expectation for a STRUCTURAL reason (not a bug list) → stop and re-decide in a new record, per the W14 precedent. Do not weaken R43 to make a gate go green.",
    "The pdfium differential shows systematic placement divergence pdfce cannot reconcile from the spec → the §12.5.5 algorithm sourcing is wrong or incomplete; re-dispatch the spec-librarian before writing a heuristic.",
    "The operator states a real redaction need → Pass 8 promotes ahead of Pass 7 immediately. The promotion is cheap by construction because Pass 6.1 already delivers the content-stream prerequisite and the /Redact mark.",
    "The operator's actual work turns out to be forms-FILLING rather than markup → promote Pass 7's fill path ahead of Pass 6.2's markup breadth (6.2's variable-text generator is Pass 7's prerequisite either way, so this is a re-order of feature surface, not of infrastructure).",
    "The operator hits a real encrypted file that blocks work → Pass 5 promotes. Its census showed 0.67%, so the trigger is an actual blocked task, not a number.",
    "A 6.x Pass hits the three-attempts wall → switch to Pass 5 (encryption), the designated fallback track, which is independent of the entire appearance/authoring arc.",
    "The operator asks for Inkscape-style vector editing sooner than this sequence delivers it → Pass 9 can promote directly after Pass 6.1, because 6.1's content-stream serializer + identity gate IS its prerequisite. Say this out loud when briefing: the distinctive-purpose work is two Passes away from its foundation, not six."
  ],

  "librarian_followups": [
    "File Passes 6.0 / 6.1 / 6.2 under Next up; file Pass 7 (Forms), Pass 8 (Redaction) and Pass 9+ (Vector editing, sliced) below them; RE-POSITION Pass 5 (Encryption) after Pass 7 as the designated interleave/fallback track, keeping its ID and its decision-007 rationale intact.",
    "CREATE a Backlog bucket for annotation DISPLAY — the 'Comments & markup' bucket exists but covers authoring; the read-side gap (F1) is currently filed nowhere, exactly as text extraction was unfiled before decision 007.",
    "Add R43–R52 to the ROADMAP standing rules.",
    "Amend ARCHITECTURE.md: §3 (where the annotation model lives, and the 'core models, render paints' axis applied to appearances); a new §5 subsection for R45's authored-stream staging buffer and R46's content-stream identity gate; §11 for R47/R48; §12 decision-log entry for decision 008.",
    "Record the measured XFA figure (4/2,914 conformance, 2/2,500 organic = 0.08%) against the XFA Backlog bucket and the standing CLAUDE.md open item 'XFA scope — verify current status'. It does not close the item (Adobe's own deprecation status is still unverified) but it answers the demand half of it with a number.",
    "Record the measured /SigFlags figure (0.64% organic) against the Digital signatures bucket.",
    "MISSING RECORD: the SESSION_LOG continuation-20 entry for Pass 4 does not exist, yet ROADMAP.md:60 cites 'the SESSION_LOG continuation-20 entry' and ROADMAP.md:73 cites 'ARCHITECTURE.md §12 continuation-20 entry (c)' as though both do. Under the project's own append-only logging protocol a shipped Pass owes both the same session. File it before anything lands on top of Pass 4.",
    "ARCHITECTURE.md §4 is materially stale — it still describes the Pass-0 header-probe state, and decisions 001/004/005 each carry an unpaid 'integrate the full design text into §4' obligation. Anything Pass 6.x writes into §4 lands on unamended ground. Worth one consolidation session before the annotation model is documented there."
  ],

  "operator_actions_owed": [
    "STILL OPEN from decision 007, and now the oldest item on the list: the encrypted-PDF refusal (XrefErrorKind::EncryptionUnsupported) was added on engineer judgment during Pass 1.1 and the operator has never been told that pdfce declines a category of file every other reader opens. The census that was supposed to inform this has since returned (0.67%, 92.5% legacy R≤4). Surface it.",
    "The license decision (LEGAL.md §1) — still gates the public repo, any release, the distribution manifests, and whether copyleft prior art is even usable.",
    "Commit authorization — Passes 0 through 4 are ALL uncommitted. This is now roughly 20,000+ lines of unversioned work with no remote and no CI history.",
    "W15 — whether to establish a git remote at all. Operator's call, gated on the license decision."
  ]
}
```

## Orchestrator note (2026-08-01, at archival)

Decision 008 archived from the KenAgent consultation. Outcome: candidate A (annotations & markup) sliced 6.0 (read-side, no authoring) → 6.1 (authored streams + content-stream serializer + geometric markup) → 6.2 (variable text); ranking A≫B>C>E>D>F; Pass 5 (encryption) retains its decision-007 ID and becomes the fallback/interleave track. Adds standing rules R43–R52. Reconciliation vs the record's §11 corrections at archival time: SESSION_LOG Continuation 20 DOES now exist (the KenAgent read a mid-write snapshot; the Pass 4 filing completed it) — item 11.1 is CLOSED. Still open and carried to the librarian: ARCHITECTURE §4 staleness (11.2), the annotation-DISPLAY backlog bucket not yet filed (11.3), the unowned Inkscape capability catalog (11.4), and the still-un-surfaced encryption-refusal operator sign-off (the oldest owed item). Blocking spec prereq §12.5 dispatched to pdfce-spec-librarian in parallel with this archival; Comments-&-markup acrobat bucket dispatched (blocking Pass 6.1). Pass 6.0 engineer dispatch is gated on the §12.5 spec return.
