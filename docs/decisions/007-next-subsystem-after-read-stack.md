# Decision 007 — The next major subsystem after the read/render stack

- **Date:** 2026-07-31
- **Status:** Decided
- **Decider:** KenAgent (autonomous-builder), per the ROADMAP standing rule
  "KenAgent decision routing (operator process rule, 2026-07-30)"
- **Question:** With the parse + render + codec arc complete and pdfce
  entirely read-only, which subsystem is next?
- **Outcome:** The incremental-save writer, sliced into three Passes, the
  first of which introduces no editing capability at all.
- **Supersedes:** nothing. **Amended by:** nothing yet.
- **Adds standing rules:** R32–R40.

---

## 1. Context — where pdfce actually is

The planned read arc is done:

- 2,892 of 2,914 conformance-corpus files load and render (99.2%).
  Every non-Ok file was inspected for this decision. All 22 are veraPDF
  or Isartor `*-fail-*` conformance files — files *designed* to be
  non-conformant — with exactly one exception:
  `pdf20examples/PDF 2.0 with offset start.pdf`, a genuine named gap
  (leading bytes before `%PDF-`, requiring xref offset rebasing).
- All five standard image-codec families decode (Passes 2.1–2.3).
- Text rendering including CID/`Identity-H`, on 14 bundled Foxit
  Base-14 faces (decision 004).
- 487 tests, 7 fuzz targets, zero crashes.
- GUI viewer and `pdfce-cli render-page` shipping.

And pdfce cannot write a single byte.

That last sentence is the whole decision. Everything below is the
reasoning for why it is the sentence that matters, and — more usefully
— for how to attack it without detonating the invariant it rests on.

### 1.1 What Pass 1 already built *for* the writer

This is the most consequential fact in the analysis, and it is easy to
miss because it lives in code rather than in the roadmap.

`crates/pdfce-core/src/object.rs:355–385` already defines:

```rust
pub enum Provenance {
    File(ByteSpan),                                  // complete definition,
                                                     // obj number → endobj
    ObjectStream { container: ObjId, index: u32 },   // §7.5.7, no file span
}

impl Provenance {
    pub const fn file_span(self) -> Option<ByteSpan> { … }
}
```

with a doc comment reading, verbatim: *"This is the accessor the writer
uses; returning `Option` rather than a sentinel is the whole point of
the enum."* The lossless, span-provenanced content-stream token model
(`content.rs`: `ContentToken`, `ContentTokenKind`, `Operation`) is
likewise built. `Document` retains the original byte buffer and exposes
it via `bytes()`.

Every one of these is one of the six Pass-1 obligations that decision
001 imposed *specifically so the writer could be built later*. They
exist. They are documented. Nothing consumes them.

Two implications:

1. **A is assembly against an existing contract, not green-field.**
   The hard design was front-loaded a Pass ago. The stated risk
   ("largest single subsystem") is real in volume but substantially
   overstated in *design* risk.
2. **Unexercised contracts decay.** `Provenance` is a promise nothing
   currently tests. Every further Pass that evolves the object model
   without a writer consuming it increases the chance the promise
   quietly stops being true. There is a cost to waiting that does not
   appear on any roadmap line.

---

## 2. The candidates, and the single question that separates them

Restated:

- **A — Incremental-save writer.** Serializer, xref append,
  ByteSpan-preserving re-emission.
- **B — Encryption (decrypt).** RC4 / AES-128 / AES-256 per §7.6.
- **C — Render-completeness remainders.** Type 3 fonts, text render
  modes 4–7, shading patterns, transparency groups and soft masks,
  pixel-parity harness.
- **D — Text extraction / structured content.** `ToUnicode`,
  `ActualText`, reading order.

The question that separates them is not "which is most valuable" —
all four are genuinely valuable — but **"what is each one a
precondition for?"**

| Candidate | Downstream features it gates |
|---|---|
| **A — writer** | Core document ops. Text and object editing. Inkscape-parity vector editing. AcroForms. Digital signatures. Redaction. Bates stamping and watermarks. Comments and markup. OCR output application. PDF/A conversion. Optimization and linearization. Portfolios. **Every operator-facing item in the Backlog, without exception.** |
| **D — text extraction** | Search and copy (direct). Accessibility / PDF-UA reading order. Redaction *verification*. OCR quality comparison. Text diff in the Comparison bucket. |
| **B — encryption** | Opening encrypted files (direct). Editing encrypted files — which sits behind A regardless. |
| **C — render remainders** | Nothing structurally. Pure fidelity improvement. |

A is not first among equals. **A is on the critical path of every
remaining feature the project exists to build.** Editing parity —
Acrobat Pro plus Inkscape — is, in the operator's own framing recorded
in ROADMAP's Vector-graphics bucket, the entire point. Without a
writer, pdfce is a very good PDF *viewer*, and it will remain exactly
that no matter how many more read-side Passes ship.

---

## 3. Why A, in detail

### 3.1 A is the capability that justified building from scratch

`docs/PRIOR_ART.md` line 104, recorded 2026-07-30:

> Confirmed 2026-07-30: nobody in the Rust ecosystem has signature-safe
> byte-preserving incremental save (lopdf issue #305 still open).

The survey is unambiguous and consistent across the ecosystem:

- `lopdf` — read+write COS, xref streams, object streams, but **no
  signature-safe incremental append** (issue #305, open).
- `pdf-writer` (Typst) — write-only, from-scratch-buffer,
  "architecturally incompatible with append-only saves."
- `krilla` — **first-party docs state encryption and signatures are
  explicitly out of scope.**
- `hayro-write` — closed the read→rewrite bridge in 2026-05, but **full
  re-serialization only, not byte-preserving.**
- `oxidize-pdf` — disqualified as a foundation partly *because* its
  "incremental" save is destructive.

Byte-preserving, signature-safe incremental save is the specific
ecosystem-wide gap that decision 001 cited to reject adopting
`oxidize-pdf` and build `pdfce-core` from scratch instead. **Deferring
it a fourth time leaves decision 001 unvalidated** — the project would
be paying the full cost of a from-scratch core while having built
nothing that a mature existing crate could not have given it.

That is not a sunk-cost argument. It is an argument that the
differentiator should be *demonstrated* before more effort compounds on
top of an unproven premise.

**Live revisit trigger.** Decision 001 §9 trigger 2 says: if
`hayro-write` gains byte-preserving incremental append, evaluate
depend-or-contribute rather than continuing solo. Re-check its
changelog as the **first action** of Pass 3.0. If it has landed, this
decision reopens on its own terms. Cheap to check; dishonest not to.

### 3.2 The render stack being done is precisely what makes A tractable *now*

This is the argument that most strongly settles the *timing*, as
opposed to the ranking.

A writer's hardest problem is not producing bytes. It is knowing
whether the bytes you produced mean the same thing as the bytes you
consumed. Structural checks (does it reload? do the offsets resolve?)
are necessary and nowhere near sufficient — they pass happily on a file
whose content streams were subtly mangled.

pdfce now has a semantic oracle: **render the input, render the output,
compare rasters.** A full-rewrite that produces a pixel-identical
render across 2,892 real conformance files is an extraordinarily strong
statement, and it is available today and was not available a month ago.

Doing the writer *before* the renderer would have meant building it
blind. Doing it *now* means every write is checkable against the most
demanding oracle the project will ever have. This is the payoff for
having done read first, and it is the reason A is right now
specifically rather than merely right eventually.

### 3.3 The stated risk is real, and it is answered by inverting the order

The concern raised with option A is exactly right: it is the largest
single subsystem, and the §5 round-trip/minimal-diff invariant is
load-bearing and subtle.

The wrong response is to defer A until it feels smaller. It will never
feel smaller, and every intervening Pass grows the object model that
the writer must faithfully re-emit.

The right response — and the one this project's own history endorses —
is to **prove the invariant before writing any code that could violate
it.**

Pass 2.2 closed the `/BlackIs1` polarity hazard "by an executable
byte-identity proof." Decision 005 set codec priority by walking 70 JPEG
codestreams marker-by-marker rather than by intuition. Decision 006
measured 37.4% of pixels differing rather than asserting a colour gap.
The standing rule about veraPDF §6.1.12 exists because *two* guards were
chosen by intuition and both were wrong.

The methodology is consistent and it works: **make the claim
executable, then measure it.**

Applied here: the first writer Pass ships a writer that **cannot edit
anything**. Its entire deliverable is a proof, across the full corpus,
that a load-and-save round trip changes nothing. Only once that gate is
green — and permanently guarding — does any mutation code get written.

This has three concrete properties worth stating plainly:

1. **It cannot regress silently later.** Every subsequent editing Pass
   runs against a 2,892-file byte-identity gate. The subtle invariant
   stops depending on anyone remembering it.
2. **It defuses the §11.4 collision.** ARCHITECTURE §11.4 mandates that
   the first Pass introducing *any* editing capability must build the
   command-log undo stack in the same Pass. A no-op writer introduces
   no editing capability, so §11.4 does not bind — which keeps the
   largest-risk subsystem from being fused to a second large subsystem.
   Undo lands in Pass 3.1 against a writer that is already proven.
3. **Failure is cheap and early.** If the invariant turns out not to be
   achievable as written, that is discovered with zero editing code in
   the tree. See §8 W14 for the contingency, which is explicitly *not*
   "quietly weaken §5."

---

## 4. Why not B, C, D — ranked, with the reasoning for each position

### 4.1 D — Text extraction — ranked **second**

D is the strongest of the three alternatives and the closest call.

**For:**
- It is the only candidate that ships operator-visible value on the
  existing read-only stack. Search and select-copy are what separate a
  viewer from a demo.
- It is largely independent of the writer, so it is a genuine parallel
  track and the natural **fallback** if a writer Pass hits the
  three-attempts wall.
- It is unusually well-prepared. Decision 004's R17 already drew its
  boundary in advance — `unicode-bidi` is permitted in a
  text-**extraction** reading-order path and is forbidden from
  `pdfce-render` — so the hardest scoping question is pre-answered.
  CMap machinery already exists from the CID/`Identity-H` work.
- It feeds redaction verification and OCR comparison later.

**Against being first:**
- It gates nothing structurally. Deferring it costs capability, not
  optionality.
- Its most trust-critical consumer, redaction verification, sits behind
  the writer anyway.

Placed immediately after the writer ladder, and explicitly nominated as
the interleave/fallback track. If the operator wants a visible-value
break between Pass 3.1 and 3.2, this is where it goes.

One correction to D's readiness: `ToUnicode` appears nowhere in
ROADMAP, SESSION_LOG, or any decision record. Text extraction has no
Backlog bucket at all. The librarian must create one before Pass 4 can
be scoped — D is well-**constrained** by R17 and R7, but it is not
well-**filed**.

### 4.2 B — Encryption — ranked **third**

B was described as "smaller, well-specified." The first half is true.
The second is, on inspection, the opposite.

**Measured corpus payoff: zero.** From `fixtures/external-report.tsv`:

| Category | Count |
|---|---|
| Ok | 2,892 |
| LoadError | 12 |
| RenderError | 6 |
| **RefusedEncrypted** | **4** |

All four encrypted files are conformance-suite failures by design:

```
veraPDF-corpus/PDF_A-4/6.1 File structure/6.1.3 File trailer/…6-1-3-t02-fail-a.pdf
veraPDF-corpus/PDF_A-2b/6.1 File structure/6.1.3 File trailer/…6-1-3-t02-fail-a.pdf
veraPDF-corpus/Isartor test files/PDFA-1b/…/isartor-6-1-3-t02-fail-a.pdf
veraPDF-corpus/PDF_UA-1/7.16 Security/7.16-t01-fail-a.pdf
```

They test that an encrypted PDF/A or PDF/UA file is **non-conformant**.
Implementing encryption moves the corpus number by nothing.

**pdfce cannot validate an encryption implementation with the corpus it
owns.** Four adversarial files is not a test suite for a subsystem that
touches every string and every stream in the document. B therefore
requires a synthetic encrypted-fixture generator as a prerequisite —
which is fine and follows the established `tools/gen-*-fixtures.py`
pattern, and is *required* anyway by LEGAL §5 — but it is real work that
"smaller, well-specified" does not capture.

**Its spec coverage is the thinnest of all four candidates.** The
PDF_Spec RAG has 70 files. On encryption it has exactly one:
`filters/filter__crypt.md` — and that documents the `/Crypt` *filter*,
not clause §7.6. The whole §7.6 tree is absent: standard security
handler, Algorithms 2 / 2.A / 4 / 5, key derivation, per-object key
computation, RC4 and AES-CBC application, crypt filters,
`/EncryptMetadata`, `/Perms`, and the PDF 2.0 AES-256 revision-6 path.
That is a full spec-librarian corpus-building session before a line of
code. Contrast with A, whose §7.5.4 / .5 / .6 / .8 files **already
exist** and need only a write-direction audit.

**Its risk profile is worse than it looks.** Decryption is not a
bolt-on: it is a stage that intercepts every string and every stream
between the parser and the object model. Retrofitting that into a stack
that has just reached 99.2% is a cross-cutting change with no corpus to
regression-test it against.

**And it is strictly cheaper after A.** Once the writer exists:
- the crypt stage slots into the serializer's object-encoder seam
  (R37) instead of being retrofitted;
- its ground truth becomes a round trip — decrypt → re-encrypt →
  byte-compare — which only the writer makes possible;
- encrypting newly-appended objects during an incremental save (which
  reuses the existing key and preserves `/Encrypt` and `/ID`) is
  designed in as a first-class requirement rather than discovered late.

Building decrypt-only first risks a design that must be reworked when
encrypt-on-write arrives. Designing the crypt stage *knowing* the
writer's shape is strictly better.

**The one honest argument for B, and how to settle it.** The corpus
cannot measure the wild. Real-world PDFs — corporate statements,
government forms, anything through Acrobat's "Restrict Editing" — very
commonly carry `/Encrypt` with an **empty user password**. Those open
silently in every reader. To an operator, pdfce refusing one reads as
*"pdfce cannot open a PDF that Chrome opens fine"* — the single worst
failure mode a viewer has.

That is a real concern about an **unmeasured number**, and this project
does not decide on unmeasured numbers. So: measure it, cheaply, in
parallel with Pass 3.0. Run a read-only census over an organic local
PDF collection reporting the `/Encrypt` share split by
empty-vs-real user password and by handler revision. Nothing is
committed to `fixtures/` (LEGAL §5); only aggregate counts are emitted.

**If the empty-user-password share is materially non-trivial, Pass 5
promotes ahead of Pass 4.** That is decision 005's methodology applied
to the one number that decides the ordering, and it costs under an hour.

*(Noted and rejected: a "tiny slice" supporting only empty-user-password
files. It saves only the password-prompt UI — key derivation, per-object
keys, and string/stream decryption are all still required. It does not
slice down usefully.)*

**Not a blocker for B:** dependencies. PRIOR_ART already classifies
`rc4`, `aes` and `cbc` (all MIT OR Apache-2.0, RustCrypto, `ring`
explicitly avoided for WASM and toolchain reasons). B's blocker is spec,
not licensing.

### 4.3 C — Render completeness — ranked **last**, and decomposed

C should not be one Pass, and most of it should not be a Pass at all.

**Its gaps are already honest.** `pdfce-render` reports Type 3 as
`UnsupportedFont::Type3` (`text.rs:539`, with a
`type3_font_is_counted_unsupported_not_rendered` test);
ExtGState `CA`/`ca`/`BM`/`SMask` are named deferrals
(`interpret.rs:1246`); `sh` is a counted deferred operator. Under R20
and R27 these are disclosed, counted, named gaps — not silent
wrongness. They cap fidelity; they do not mislead. That is a materially
lower-urgency category than a subsystem that does not exist.

**Half of it belongs to another bucket.** Shading patterns and
transparency groups / soft masks are prerequisites of the
Vector-graphics-editing (Inkscape-parity) backlog bucket — gradients
*are* axial and radial shading. Implementing them as a standalone
fidelity slice and then again as an editing feature is duplicated work.
Fold them into that bucket.

**The rest is already filed.** Type 3 fonts and text render modes 4–7
are **Pass 1.1 item 4** (LOW, near-zero corpus presence). `/SMask` and
`/Mask` are **Pass 1.1 item 6.3**, ranked immediately after the image
codecs and blocked on a `pdfce-spec-librarian` clause-11 dispatch. Only
general transparency groups and blend modes are scoped nowhere. C is
therefore not four unscoped items but two already-filed ones, one that
belongs to the vector-editing bucket, and one long-outstanding harness.

**And its single most valuable item gets cheaper as a side effect of
A.** Pass 3.0 needs a raster comparison oracle. But it needs a
*self-comparison* one — pdfce-before against pdfce-after — which
requires no reference renderer and is roughly a day of work. That is
**not** the same as Pass 1.1's outstanding reference-renderer parity
harness (pdfce against pdfium), and it must not be reported as closing
it. But it is most of the same plumbing. So C's best item becomes
**infrastructure for A rather than a competitor to it** — an either/or
that dissolves into a dependency once the two are looked at together.

---

## 5. The decision

**Build the incremental-save writer. Slice it into three Passes. Make
the first one a no-op.**

| Pass | Name | Editing capability | Undo required (§11.4) | Acrobat RAG required |
|---|---|---|---|---|
| **3.0** | Identity writer + round-trip proof harness | **none — deliberate** | No | No |
| **3.1** | Mutation writer + dirty-set diff + undo/redo command log | `/Info` metadata, page `/Rotate` | **Yes** | No |
| **3.2** | Structural page operations | merge/split/extract/insert/delete/reorder | Yes (extends 3.1) | **Yes** |
| **4** | Text extraction / structured content | — | — | For accessibility scope |
| **5** | Encryption (decrypt + encrypt-on-save) | — | — | Yes (permissions semantics) |
| **6+** | Render remainders, decomposed | — | — | — |

Pass IDs are indicative; `pdfce-librarian` assigns the real ones.

### 5.1 Why the split falls exactly there

- **3.0 / 3.1** splits on *mutation*, which is the line §11.4 draws. A
  writer with no mutation is genuinely half the subsystem and carries
  none of the undo obligation.
- **3.1 / 3.2** splits on *content-stream re-emission and object
  renumbering*. Pass 3.1's mutations (`/Info`, `/Rotate`) touch
  dictionary values only — no content stream, no appearance stream, no
  font. That isolates the dirty-set machinery so it can be tested
  without content re-emission confounding it. Pass 3.2 then forces the
  full-rewrite path, cross-document renumbering, and the
  compressed-object promote-vs-rewrite decision, all against a writer
  whose fundamentals are already green.

### 5.2 Pass 3.0 scope

**Deliverables**

1. `pdfce-core::writer` — a serializer for every `Object` variant per
   §7.3, byte-exact on the forms that matter: string escaping and
   hex-string form, name `#`-escaping, real-number formatting, and
   `/Length` agreement for streams.
2. `Document::save_full(path)` — full rewrite. Every `Provenance::File`
   object is re-emitted **from its retained source bytes verbatim**.
   Only xref, trailer, `startxref` and object offsets are newly
   generated.
3. `Document::save_incremental(path)` — the default mode. With a
   structurally-empty dirty set the output is **byte-identical to the
   input**, not "the input plus an empty revision." *Zero edits means
   zero bytes.*
4. Xref emission in **both** forms — table (§7.5.4) and stream (§7.5.8)
   — selected to match the input's newest section. Never normalized
   (R33).
5. **An object-encoder seam** in the serializer, identity implementation
   in this Pass, so the Pass-5 crypt stage plugs in rather than being
   retrofitted (R37).
6. `tools/corpus-report` extended with a round-trip mode.
7. A `parse → write → parse → compare` fuzz target (R40).
8. `pdfce-cli`: a `round-trip` / `save` subcommand exposing both modes
   and the verification result, with a documented exit-code contract
   distinguishing *not byte-identical* from *failed to reload* from
   *raster differs*.
9. **`ARCHITECTURE.md` §5 amendment** closing the three gaps this
   decision identified: redaction-forbids-incremental (R35), `/ID`
   discipline (R39), linearization invalidation (R36).
10. **No-output-fingerprint enforcement** (decision 001 §6.1
    obligation 6, whose enforcement point IS the writer):
    `save_incremental` NEVER rewrites `/Info` unless the operator
    actually changed metadata; `save_full` sets
    `/Producer = 'pdfce <version>'`, documented and overridable via
    both the pdfce-core API and a pdfce-cli flag. No build hash, no
    edition marker, no non-suppressible producer id anywhere. This is
    the structural prevention of the exact behavior that disqualified
    oxidize-pdf as a foundation.

**Acceptance criteria**

- Across all 2,892 currently-Ok corpus files: `save_incremental` with no
  edits produces a **byte-identical** file. Target 100%; any shortfall
  is enumerated **by file and by reason** — a counted shortfall in the
  R20 tradition, never rounded away.
- Across the same 2,892: `save_full` produces a file that (a) reloads,
  (b) contains every `File`-provenance object's definition bytes
  verbatim, and (c) **re-renders to a raster identical to the input's
  render** at fixed DPI.
- Byte identity is asserted **per object definition** for `save_full`
  and **whole-file** for empty-dirty-set `save_incremental`. Two
  different assertions; never conflated (R32).
- veraPDF §6.1.12 implementation-limits run against any new writer-side
  guard, per the existing standing rule (two prior intuition-guard
  incidents: `MAX_TOKEN_LEN`, `MAX_XOBJECT_DEPTH`).
- `cargo fmt --check` and `cargo clippy -- -D warnings` clean;
  `cargo tree -p pdfce-core` shows no GUI dependency.
- A test asserts that `save_incremental` on an unmodified document
  leaves `/Info` byte-untouched, and that `save_full`'s `/Producer` is
  suppressible through both front ends.

**Explicit non-goals** — binding, because "while we are in the writer"
is how this Pass doubles:

- No editing capability of any kind, therefore no undo stack.
- No object deletion, no free-list writing, no generation increments.
- No linearization writing. No optimization. **No normalization of
  anything, ever.**
- No encryption. The seam is built; the implementation is Pass 5.

**Parallel cheap task** — the `/Encrypt` census of §4.2, which decides
whether Pass 5 promotes ahead of Pass 4.

---

## 6. Standing rules added by this decision (R32–R40)

Binding, in the tradition of R1–R31. This section is the condensed
form; §5 and §8 above are the authority if any condensation is
ambiguous.

- **R32 — Byte identity is per object, not per file.** §5's invariant
  is a per-object-definition contract. A full rewrite legitimately
  changes xref offsets, the trailer and `startxref`, and can never be
  file-identical. Therefore: `save_full` asserts per-object-definition
  byte identity **plus** raster identity; `save_incremental` with an
  empty dirty set asserts **whole-file** byte identity. Conflating the
  two produces a test that fails universally or passes vacuously.

- **R33 — The writer never normalizes.** The xref form (table vs
  stream) matches the input's newest section. The PDF version header is
  never bumped. Object streams are never introduced, expanded, or
  reorganized as a side effect. Numbers, names and strings are never
  reformatted on a passthrough object. Normalization produces a
  plausible, working, wrong file — the hardest defect class to notice.

- **R34 — The round-trip gate guards every writer Pass.** Any Pass
  touching the writer re-runs the corpus round-trip harness.
  Regressions block the Pass. The invariant stops depending on anyone
  remembering it.

- **R35 — Incremental save structurally preserves superseded content;
  removal operations must therefore refuse it.** An incremental update
  leaves the prior bytes of every edited object in the file by
  construction. **Redaction — and any operation whose contract is
  removal — forces a full rewrite and must refuse incremental save.**
  A test greps the saved bytes for the redacted content. This closes a
  real, previously undocumented gap between §5, §11.2 and the fact that
  incremental is the *default* save mode.

- **R36 — Save mode is chosen by contract and disclosed, never chosen
  silently.** Signatures present → incremental by default; a full
  rewrite must name what it destroys before proceeding. Linearized
  input → an incremental save warns that Fast Web View is invalidated
  (never silently re-linearizes; that is the Optimization bucket).
  Redaction → full rewrite, mandatory, per R35.

- **R37 — The serializer takes an object-encoder seam from day one.**
  Identity implementation in Pass 3.0; the §7.6 crypt stage plugs into
  it in Pass 5. No layer below the seam writes bytes to the file. One
  page of design now; a cross-cutting rewrite avoided later.

- **R38 — Compressed-object edits promote; they do not rewrite
  containers.** A touched `Provenance::ObjectStream` object is promoted
  to an uncompressed object and superseded by a type-1 xref entry; the
  old container is left byte-untouched. Rewriting the container would
  perturb every *other* object inside it — a minimal-diff violation by
  proxy. Every promotion is a counted, named diagnostic. Note the
  interaction with R35: the stale compressed copy is exactly why
  redaction cannot use incremental save.

- **R39 — `/ID` discipline.** `/ID[0]` is preserved for the life of the
  file; `/ID[1]` is regenerated on every save (§14.4). Tested
  explicitly. It is also an input to §7.6 key derivation, so an error
  here surfaces in Pass 5 as a decryption failure that looks like a
  crypto bug.

- **R40 — Writer changes carry fuzz and differential coverage.** Every
  writer Pass extends the `parse → write → parse → compare` fuzz target
  and the `tools/difftest` oracle. `qpdf --check` is used as an external
  structural validator (Apache-2.0; PRIOR_ART line 227 clears direct
  reuse including test material, with attribution).

- **R41 — No output fingerprint, enforced at the writer.** Restates
  decision 001 §6.1 obligation 6 at its actual enforcement point:
  incremental save never rewrites `/Info` absent a real operator
  metadata change; full rewrite's `/Producer` is documented and
  overridable from both front ends; no build hash, edition marker, or
  non-suppressible producer id is ever emitted.

---

## 7. Spec prerequisites

| Item | RAG status | Blocking |
|---|---|---|
| **Write-direction audit** of §7.5.4, §7.5.5, §7.5.6, §7.5.8 | **Files exist**, but all four were built for the READ path in Pass 1 | **Yes — Pass 3.0** |
| §7.5.8.4 hybrid-reference files (`/XRefStm`) — write side | Read side only | **Yes — Pass 3.0** |
| §14.4 file identifiers (`/ID`) | **Absent** | **Yes — Pass 3.0** |
| Annex F Linearized PDF — *detection only* | Absent | No |
| §12.8 `/ByteRange` semantics only (justifies R36) | Absent | No |
| **§7.6 encryption — entire clause tree** | **Absent.** Only `filters/filter__crypt.md`, which is the `/Crypt` *filter* | Blocks **Pass 5** |
| Acrobat_Features — "Core document ops" | **RAG is effectively empty** (LEGAL_NOTE, _TEMPLATE, index only) | Blocks **Pass 3.2** |

Two notes that matter more than the table:

**The write-direction audit is the real prerequisite, not new
ingestion.** §7.5.4/.5/.6/.8 all exist. What is unverified is whether
they cover *emission*: the exactly-20-byte xref entry rule and its
permitted EOL forms; subsection header syntax; `startxref` / `%%EOF`
placement; `/Prev` chaining on an appended section; `/Size` semantics
on an incremental update; xref-stream `/W` `/Index` `/Filter`
`/Predictor` emission constraints; type-0 free-chain construction.
Dispatch `pdfce-spec-librarian` for an audit, not a build.

**The empty Acrobat_Features RAG does not block Passes 3.0 or 3.1.**
The feature-fidelity standing rule binds when a *Backlog Acrobat-parity
bucket* becomes a Pass. The writer is `ARCHITECTURE.md` §5
infrastructure, not an operator-facing feature bucket — there is no
"Acrobat behavior" for a serializer to match. It binds at Pass 3.2,
where "Core document ops" *is* such a bucket. Stated explicitly because
an empty RAG could otherwise be read as blocking the whole sequence.

---

## 8. Risks

Full enumeration in the JSON block; the ones that would actually bite:

**W2 — the redaction hole (trust-critical, previously undocumented).**
Incremental save preserves superseded content by construction, and
incremental is the default. A redaction saved incrementally leaves the
redacted content trivially recoverable. §5 and §11.2 between them cover
removal semantics and undo semantics and never state this. **R35.**

**W3 — compressed objects leak by a second path.** A promoted object
leaves its old copy inside an untouched object stream. Same leak; R35's
full-rewrite requirement closes both.

**W1 — per-object vs per-file identity.** The likeliest source of a
false green *or* a false red in Pass 3.0. **R32.**

**W7 — redact-a-signed-document is a genuine conflict.** R35 mandates
full rewrite for redaction; R36 mandates incremental to preserve
signatures. These are irreconcilable for a signed document, and that is
not an oversight — it is a real property of PDF. Surface it to the
operator as an explicit either/or. It belongs to the Redaction and
Signatures Passes to resolve; flagged here so it is not discovered
late.

**W10 — exactly-20-byte xref entries (§7.5.4).** Off-by-one or a
bare-LF variant yields a file most readers repair silently *and* that
pdfce's own lenient parser reloads happily — a false green. Assert entry
length in a unit test; do not rely on round-trip reload.

**W13 — decision 001 §9 trigger 2 is live.** Re-check `hayro-write`
first, not last.

**W14 — the invariant might not be achievable as written.** This is the
*good* outcome of doing Pass 3.0 first: it is discovered with zero
editing code in the tree. **Contingency: if the byte-identity gate
lands below ~98% and the shortfall is structural rather than a bug
list, STOP and re-decide the invariant in a new decision record. Do not
weaken §5 quietly to make a gate go green** — that is precisely the
failure this Pass exists to make impossible.

---

## 9. Revisit triggers

1. **`hayro-write` gains byte-preserving incremental append.** Decision
   001 §9 trigger 2 fires; the depend-or-contribute question reopens.
   Check before Pass 3.0 starts.
2. **The `/Encrypt` census shows a materially non-trivial
   empty-user-password share in organic files.** Pass 5 promotes ahead
   of Pass 4.
3. **Pass 3.0's byte-identity gate lands below ~98% for structural
   reasons.** Per W14 — re-decide, do not weaken.
4. **A writer Pass hits the three-attempts wall.** Switch to Pass 4
   (text extraction) as the designated fallback track; it is
   writer-independent by construction.
5. **The operator's priorities change** — e.g. an immediate need to open
   a specific encrypted file, or to search document text. Both are
   already-ranked candidates; promoting one is a re-order, not a
   re-decision.

---

## 10. Follow-up items for the librarian

1. File Passes 3.0 / 3.1 / 3.2 under *Next up*; file Pass 4 (text
   extraction) and Pass 5 (encryption) under *Next up* below them;
   annotate the Backlog's Encryption bucket with the measured
   four-corpus-file finding and the §7.6 RAG gap.
2. Add R32–R40 to the ROADMAP standing rules.
3. Amend `ARCHITECTURE.md` §5 with R35 (redaction forbids incremental),
   R36 (save-mode disclosure), R39 (`/ID`) — and amend §11.2 with a
   cross-reference to R35, since it currently discusses redaction's
   post-save irreversibility without noting that incremental save does
   not actually deliver it.
4. Append the §12 decision-log entry for decision 007.
5. Note in the Backlog's "Render completeness" area that shading and
   transparency fold into the Vector-graphics-editing bucket, and that
   Type 3 and text render modes 4–7 are independent small slices.
6. Correct the record on `pdf20examples/PDF 2.0 with offset start.pdf` —
   it is the **one** genuine (non-`*-fail-*`) load failure in the
   corpus, and it is not currently tracked as a named gap anywhere.

---

## 11. Corrections to the record, and housekeeping owed

Surfaced while grounding this decision. None changes the outcome; all
three are cheap and get more expensive with age.

1. **The encryption-refusal scope addition was never surfaced to the
   operator.** `SESSION_LOG:879` records the action as owed and it is
   still open. Encrypted-PDF refusal
   (`XrefErrorKind::EncryptionUnsupported`) was added on engineer
   judgment during Pass 1.1 item 1. It is the right behavior — refusing
   is strictly better than silently rendering ciphertext, and it is
   fail-clean in the R27 tradition — but the operator has not been told
   that pdfce declines a category of file that every other reader opens.
   That is precisely the decision this record's §4.2 census is meant to
   inform, so it should be surfaced before the census runs.

2. **Pass 2.3 (JPX) has no `SESSION_LOG` entry.** Continuation 14, the
   most recent, lists it as *next* and blocked on a `/SMaskInData` +
   Table 89 + Table 6 spec dispatch. The operator's brief for this
   decision states JPX decodes. Reconcile before filing anything on top
   of it; under the roadmap-discipline rule a shipped Pass owes a
   Shipped row and a log entry the same session.

3. **The corpus baseline is ambiguous.** `fixtures/external-report.tsv`
   tallies 2,892 Ok of 2,914 (99.24%); continuation 14 records 99.3%
   (2,886) at Pass 2.2. The delta is consistent with JPX having shipped
   after that entry — see item 2 — but it is not confirmed. **Re-run
   `tools/corpus-report` and pin the exact Ok count as Pass 3.0's
   baseline before the round-trip gate is written.** A gate whose
   denominator is uncertain cannot report an honest shortfall, and an
   honest counted shortfall is the entire deliverable.

Also carried, non-blocking: `filter__jbig2.md` Table 12 contents remain
unverified (Pass 2.2 was implemented against §7.4.7 prose), and
`filter__dct.md`'s sourcing-gap closure is owed per decision 006 §6.5.

---

## Appendix A — Effective JSON decision block (post-patch)

Artifact 1 (the base JSON decision block) with the consultation's
final-message patch operations merged in. **This is the effective
decision** — the JSON that drives implementation.

```json
{
  "decision": "A — the incremental-save writer — sliced into three Passes, the first of which introduces NO editing capability. Pass 3.0 ships a serializer plus save_full/save_incremental with a structurally-empty dirty set, and its entire acceptance bar is a corpus-wide executable proof of the ARCHITECTURE.md §5 round-trip/minimal-diff invariant. Editing capability arrives in Pass 3.1 only after the invariant is a measured regression gate rather than a promise. Ranking of the four candidates: A >> D > B > C.",

  "pass_sequence": [
    {
      "id": "Pass 3.0",
      "name": "Identity writer + round-trip proof harness",
      "editing_capability": "none — deliberate",
      "why_here": "Converts the load-bearing §5 invariant from prose into a 2,892-file executable gate BEFORE any code exists that could violate it. Introduces no mutation, so ARCHITECTURE.md §11.4's build-undo-into-the-first-editing-Pass obligation does not bind, keeping the largest-risk subsystem free of a second large subsystem.",
      "blockers": ["pdfce-spec-librarian write-direction audit (see spec_prerequisites)", "re-check decision 001 §9 trigger 2: has hayro-write gained byte-preserving incremental append?"],
      "acrobat_rag_required": false
    },
    {
      "id": "Pass 3.1",
      "name": "Mutation writer + dirty-set diff + undo/redo command log",
      "editing_capability": "smallest possible: document /Info metadata and page /Rotate",
      "why_here": "First real mutation. ARCHITECTURE.md §11.4 binds: the command-log undo stack is built here, not retrofitted. Mutation surface is deliberately chosen to touch no content stream, no appearance stream, and no font — so the Pass tests the dirty-set machinery, not content re-emission.",
      "key_test": "edit -> undo -> save must produce a byte-identical file. This is the §11.1 'union of every command ever run' bug, made executable.",
      "acrobat_rag_required": false
    },
    {
      "id": "Pass 3.2",
      "name": "Structural page operations (Core document ops bucket)",
      "editing_capability": "merge / split / extract / insert / delete / reorder / rotate",
      "why_here": "First operator-visible editing feature, and the first that forces the full-rewrite path, cross-document object renumbering, and the compressed-object promote-vs-rewrite decision. Ships the GUI flow and pdfce-cli subcommands together per standing rule.",
      "blockers": ["Acrobat_Features RAG is currently EMPTY (only LEGAL_NOTE.md, _TEMPLATE.md, index.md) — dispatch pdfce-acrobat-librarian for the 'Core document ops' bucket before scoping acceptance criteria"],
      "acrobat_rag_required": true
    },
    {
      "id": "Pass 4",
      "name": "Text extraction / structured content (candidate D)",
      "why_here": "Ranked second overall. Independent of the writer, so it is also the designated fallback track if a writer Pass hits the three-attempts wall. Delivers search + select-copy, which is what separates a viewer from a demo, and is a precondition for verifying redaction and for OCR comparison later. Decision 004 R17 already pre-scoped its boundary (unicode-bidi is permitted in a text-extraction reading-order path and nowhere else).",
      "correction": "text extraction has NO existing Backlog bucket — `ToUnicode` appears nowhere in ROADMAP, SESSION_LOG or any decision record. The librarian must CREATE the bucket before the Pass can be scoped. Only R17 (unicode-bidi permitted in a text-extraction reading-order path, forbidden in pdfce-render) and R7 (document text is a Pass-1-onward requirement) pre-constrain it."
    },
    {
      "id": "Pass 5",
      "name": "Encryption — decrypt and encrypt-on-save (candidate B)",
      "why_here": "Cheaper and safer after the writer exists: the crypt stage is a bidirectional encoder that slots into the Pass 3.0 serializer seam (R37), its ground truth is a decrypt->re-encrypt->byte-compare round trip that only the writer makes possible, and encrypting newly-appended objects during incremental save is a first-class requirement rather than a retrofit.",
      "promotion_trigger": "Runs AHEAD of Pass 4 if the organic-corpus census (see first_pass_scope.parallel_cheap_task) shows a materially non-trivial share of real-world PDFs carrying /Encrypt with an EMPTY user password — those open silently in every other reader, so pdfce's refusal reads to the operator as 'pdfce cannot open a file Chrome opens fine.'",
      "name_correction": "Encryption — decrypt all handlers; encrypt-on-save AES-128/256 ONLY. RC4 is read-compat only and is NEVER written, per the standing Backlog posture that R28 cites as its own precedent.",
      "dependency_citation": "decision 001 §6.2 is the authority, not PRIOR_ART alone: aes, cbc, sha2, md-5, rc4 are pre-selected for the security handler (cms, x509-cert, x509-parser for later PAdES)."
    },
    {
      "id": "Pass 6+",
      "name": "Render-completeness remainders (candidate C), decomposed — NOT one Pass",
      "why_here": "Ranked last and deliberately broken up. Shading patterns and transparency groups/soft masks fold into the Vector-graphics-editing (Inkscape-parity) bucket, because gradients ARE axial/radial shading and implementing them twice is waste. Type 3 fonts and text render modes 4-7 are small independent slices. The reference-renderer pixel-parity harness stays a Pass 1.1 remainder, made materially cheaper by Pass 3.0 (see first_pass_scope.note_on_candidate_C).",
      "correction": "Two of candidate C's four items are ALREADY FILED as Pass 1.1 items, not new work: Type 3 fonts + Tr 4-7 clipping = Pass 1.1 item 4 (LOW, near-zero corpus presence); /SMask + /Mask = Pass 1.1 item 6.3, ranked immediately after the image codecs and BLOCKED on a pdfce-spec-librarian clause-11 dispatch. Only general transparency groups and blend modes are scoped nowhere. The reference-renderer pixel-parity harness is cheaper than stated: decision 006 §3.2 already ran an ad-hoc pypdfium2 comparison over 9 files, so the tooling precedent exists."
    }
  ],

  "first_pass_scope": {
    "pass_id": "Pass 3.0",
    "deliverables": [
      "pdfce-core::writer module: a serializer emitting every Object variant per ISO 32000-1 §7.3, byte-exact on the forms that matter (string escaping, name #-escaping, real-number formatting, stream /Length agreement).",
      "Document::save_full(path) — full rewrite. Every object whose Provenance is File(ByteSpan) is re-emitted from its retained source bytes verbatim; only xref, trailer, startxref and object offsets are newly generated.",
      "Document::save_incremental(path) — the default save mode. With a structurally-empty dirty set the output is REQUIRED to be byte-identical to the input (not 'input plus an empty revision'). Zero edits means zero bytes.",
      "Xref emission in BOTH forms — table (§7.5.4) and stream (§7.5.8) — selected to match the input's newest section, never normalized (R33).",
      "An object-encoder seam in the serializer (identity implementation in this Pass) so the future crypt stage is a plug-in, not a retrofit (R37).",
      "tools/corpus-report extended with a round-trip mode; a new tools/roundtrip harness or subcommand.",
      "A parse->write->parse->compare fuzz target added to fuzz/fuzz_targets (R40).",
      "pdfce-cli: a `round-trip` / `save` subcommand exposing both modes plus the verification result, with a documented exit-code contract distinguishing 'not byte-identical' from 'failed to reload' from 'raster differs'.",
      "ARCHITECTURE.md §5 amendment closing the three documented gaps: redaction-forbids-incremental (R35), /ID discipline (R39), linearization invalidation (R36).",
      "No-output-fingerprint enforcement (decision 001 §6.1 obligation 6, whose enforcement point IS the writer): save_incremental NEVER rewrites /Info unless the operator actually changed metadata; save_full sets /Producer = 'pdfce <version>', documented and overridable via both the pdfce-core API and a pdfce-cli flag. No build hash, no edition marker, no non-suppressible producer id anywhere. This is the structural prevention of the exact behavior that disqualified oxidize-pdf as a foundation."
    ],
    "acceptance_criteria": [
      "Over all 2,892 currently-Ok corpus files: save_incremental with no edits produces a file byte-identical to the input. Target 100%; any shortfall is enumerated by file and by reason, never rounded away (R20-style counted shortfall).",
      "Over the same 2,892: save_full produces a file that (a) reloads without error, (b) contains every File-provenance object's definition bytes verbatim, and (c) re-renders to a raster identical to the input's render at a fixed DPI. Criterion (c) is the semantic oracle, and it is available ONLY because the render stack shipped first — this is the payoff for having done read first and the specific reason A is right NOW.",
      "Byte-identity is asserted PER OBJECT DEFINITION, never per file, for save_full — xref offsets and the trailer legitimately differ (R32). Getting this wrong is the single likeliest source of a false green or a false red in this Pass.",
      "veraPDF §6.1.12 implementation-limits suite run against any new writer-side guard, per the existing standing rule (two prior incidents: MAX_TOKEN_LEN, MAX_XOBJECT_DEPTH).",
      "cargo fmt --check and cargo clippy -- -D warnings clean; cargo tree -p pdfce-core shows no GUI dependency.",
      "A test asserts that save_incremental on an unmodified document leaves /Info byte-untouched, and that save_full's /Producer is suppressible through both front ends."
    ],
    "explicit_non_goals": [
      "No editing capability of any kind, so no undo stack (§11.4 does not bind a Pass with no mutation).",
      "No object deletion, no free-list writing, no generation increments — those arrive with Pass 3.1/3.2 where they can be tested against a real mutation.",
      "No linearization writing. No optimization. No normalization of anything, ever.",
      "No encryption. The seam is built; the implementation is Pass 5."
    ],
    "parallel_cheap_task": "An organic-corpus /Encrypt census — run a read-only scan over a real-world local PDF collection (Ken's own files) reporting: share carrying /Encrypt, split by empty-user-password vs real user password, by handler revision (R2/R3/R4/R6) and by /V. Measurement only; nothing is committed to fixtures (LEGAL.md §5 forbids unknown-provenance PDFs in the repo — this scan reads in place and emits only aggregate counts). This is decision 005's measurement-first methodology applied to the one number that decides whether Pass 5 promotes ahead of Pass 4, and it costs under an hour.",
    "note_on_candidate_C": "Pass 3.0 needs a raster comparison oracle, but a SELF-comparison one (pdfce-before vs pdfce-after), which needs no reference renderer and is roughly a day of work. That is deliberately NOT the same thing as Pass 1.1's outstanding reference-renderer pixel-parity harness (pdfce vs pdfium). Building the self-comparison oracle is however most of the plumbing for the reference one, so candidate C's single most valuable item gets materially cheaper as a side effect of Pass 3.0 rather than competing with it. Do not overclaim this as 'C is done'."
  },

  "rationale_summary": "The writer is the only candidate on the critical path of EVERY remaining operator-facing feature. Core document ops, text and object editing, Inkscape-parity vector editing, AcroForms, digital signatures, redaction, Bates stamping, comments and markup, OCR output, PDF/A conversion, optimization — none of them can ship a single byte without it, and editing parity is the project's entire stated purpose. B gates exactly one thing (opening encrypted files) and its own downstream value, editing encrypted files, sits behind A anyway. C improves an already-99.2% read stack whose remaining gaps are already named, counted and disclosed, so they cap fidelity without misleading anyone. D is genuinely valuable and ships on the read-only stack, which is why it ranks second, but it is not a precondition for anything the way A is. Three further facts settle it: (1) the ByteSpan/Provenance substrate and lossless content-token model were already built in Pass 1 explicitly FOR the writer, so A is assembly against an existing contract rather than green-field, and that investment decays if it sits unexercised while the model keeps evolving; (2) PRIOR_ART.md records that byte-preserving signature-safe incremental save is a gap NOBODY in the Rust ecosystem has closed, and that gap is the stated justification for decision 001 choosing build-from-scratch over adopting oxidize-pdf — deferring it a fourth time leaves decision 001 unvalidated; (3) B's corpus payoff is measured at exactly zero (all 4 RefusedEncrypted files are veraPDF *-fail-* conformance files that are supposed to be non-conformant), its spec coverage is the thinnest of the four, and pdfce cannot validate an encryption implementation with the corpus it owns. The stated risk in A — that the round-trip invariant is subtle and load-bearing — is answered not by deferring A but by inverting the usual order: ship the writer with NO editing capability first and spend the entire Pass proving the invariant across 2,892 files, so that every later editing Pass lands against a measured gate instead of a promise. That is the same measurement-first discipline decisions 005 and 006 established, applied to the largest subsystem rather than the smallest.",

  "risks": [
    {
      "id": "W1",
      "risk": "Per-object vs per-file byte identity confusion. §5's invariant is per-object; a full rewrite CANNOT be byte-identical file-wide because offsets shift. A test asserting file-level identity for save_full fails universally; a test asserting only reloadability passes vacuously.",
      "mitigation": "R32 states the contract precisely: save_full asserts per-object-definition byte identity plus raster identity; save_incremental with an empty dirty set asserts whole-file byte identity. Two different assertions, never conflated."
    },
    {
      "id": "W2",
      "risk": "TRUST-CRITICAL AND CURRENTLY UNDOCUMENTED — incremental save structurally preserves superseded content. The old bytes of every edited object remain in the file by construction. A redaction saved incrementally leaves the redacted content trivially recoverable. ARCHITECTURE.md §5 and §11.2 discuss redaction's removal semantics and undo semantics but never state this, and incremental is the DEFAULT save mode.",
      "mitigation": "R35: redaction, and any operation whose contract is removal, forces a full rewrite and must refuse incremental save. Close the §5 gap in this Pass, before any editing code exists. Ship a test that greps the saved bytes for the redacted content."
    },
    {
      "id": "W3",
      "risk": "Compressed-object edits (Provenance::ObjectStream, file_span() == None) have no verbatim bytes to re-emit. Rewriting the container perturbs every OTHER object inside it — a minimal-diff violation by proxy — and leaves a stale copy of the edited object in the old container, which is W2's leak by a second path.",
      "mitigation": "R38: promote-to-uncompressed is the default for a touched compressed object; the old container is left untouched and the new type-1 xref entry supersedes the type-2 one. Each promotion is a counted, named diagnostic. Under R35 redaction's full rewrite closes the stale-copy path."
    },
    {
      "id": "W4",
      "risk": "Silent normalization. Emitting an xref stream where the input had a table (or vice versa), bumping the PDF version header, introducing or expanding object streams, or reformatting numbers — each is a §5 violation that produces a plausible, working, WRONG file.",
      "mitigation": "R33 forbids all of it. The corpus round-trip gate catches it mechanically: any normalization shows up immediately as a byte diff across hundreds of files."
    },
    {
      "id": "W5",
      "risk": "Linearized (Fast Web View) input silently invalidated. An incremental save appends past the first-page xref and hint tables, so the linearization is stale — the file still opens but its Fast Web View claim is now false.",
      "mitigation": "R36: detect Annex F linearization on load and name it. Saving warns that Fast Web View is invalidated. Never repair silently; re-linearization is the Optimization backlog bucket, not this Pass."
    },
    {
      "id": "W6",
      "risk": "Trailer /ID mishandled. §14.4 requires the first element stable for the life of the file and the second regenerated per save. It is also an input to encryption key derivation, so an error here becomes a decryption failure in Pass 5 that looks like a crypto bug.",
      "mitigation": "R39, plus a test that a save preserves /ID[0] and changes /ID[1]. Getting this right costs minutes now and hours later."
    },
    {
      "id": "W7",
      "risk": "Digital signatures destroyed by full rewrite. A signature covers a byte range; any full rewrite invalidates it. Full rewrite is exactly what R35 mandates for redaction — so 'redact a signed document' is a genuine conflict, not an oversight.",
      "mitigation": "R36: signature presence forces incremental by default, and a full rewrite must name what it destroys before proceeding. The redact-a-signed-document conflict is surfaced to the operator as an explicit either/or, never resolved silently. Flag it now; it belongs to the Redaction and Signatures Passes to resolve."
    },
    {
      "id": "W8",
      "risk": "Encryption retrofitted into a finished serializer. The crypt stage touches EVERY string and EVERY stream, and incremental save of an encrypted document must encrypt newly-appended objects with the existing key. Bolting that onto a completed writer is a cross-cutting rewrite.",
      "mitigation": "R37: the serializer takes an object-encoder seam from day one, identity implementation in Pass 3.0. Roughly a page of design now, and it is the single cheapest de-risking move available for Pass 5."
    },
    {
      "id": "W9",
      "risk": "Free-list and generation-number errors on deletion. A malformed type-0 free chain produces files Acrobat tolerates and stricter readers reject — the worst failure shape, because the obvious test passes.",
      "mitigation": "Deletion is deliberately OUT of Pass 3.0's scope. It lands in Pass 3.2 with real mutations to test against, and validates against qpdf --check as an external oracle (Apache-2.0, PRIOR_ART line 227 clears it for direct reuse including test material)."
    },
    {
      "id": "W10",
      "risk": "Exactly-20-byte xref entries (§7.5.4). Each entry is exactly 20 bytes including a 2-byte EOL. Off-by-one or a bare-LF variant yields a file most readers repair silently and pdfce's own lenient parser will happily reload — a false green.",
      "mitigation": "Named as a spec prerequisite for write-direction audit; assert entry length in a unit test rather than relying on round-trip reload."
    },
    {
      "id": "W11",
      "risk": "Hybrid-reference files (§7.5.8.4, /XRefStm) — writing an update to a file that carries both a table and a stream raises the question of which the appended section must match.",
      "mitigation": "Included in the spec-librarian write-direction audit. If it cannot be resolved cleanly, hybrid inputs fail clean and counted by name rather than being guessed at (R27's posture, applied to the writer)."
    },
    {
      "id": "W12",
      "risk": "Scope creep — 'while we are in the writer' pulls in linearization, optimization, object-stream authoring or font subsetting. Any of them individually doubles the Pass.",
      "mitigation": "The explicit_non_goals list is binding. Pass 3.0 ships an identity writer or it does not ship."
    },
    {
      "id": "W13",
      "risk": "Decision 001 §9 revisit trigger 2 is live: if hayro-write has gained byte-preserving incremental append since 2026-07-30, the depend-or-contribute question reopens BEFORE this Pass, not after.",
      "mitigation": "Re-check hayro-write's changelog as the first action of the Pass. Cheap, honest, and the decision record should say so out loud."
    },
    {
      "id": "W14",
      "risk": "The invariant turns out not to be achievable as written — e.g. a meaningful share of the corpus cannot round-trip byte-identically for a structural reason nobody anticipated.",
      "mitigation": "This is the good outcome of doing Pass 3.0 first: finding out with zero editing code written. Contingency: if the byte-identity gate lands below roughly 98% and the shortfall is structural rather than a bug list, STOP and re-decide the invariant explicitly in a new decision record. Do not weaken §5 quietly to make a gate go green — that is precisely the failure this Pass exists to make impossible."
    },
    {
      "id": "W15",
      "risk": "The proposed corpus round-trip gate is CI-enforced, and CI HAS NEVER RUN — pdfce has no git remote, and everything has been uncommitted across six continuations. A gate that exists only as a local command is a gate that silently stops running the first time someone is in a hurry.",
      "mitigation": "Pass 3.0 must make the round-trip harness runnable and green LOCALLY as a hard acceptance criterion, and must not depend on CI for its correctness claim. Whether to establish a remote is gated on LEGAL.md §1 (license undecided) and is the operator's call, not the engineer's — flag it, do not resolve it."
    },
    {
      "id": "W16",
      "risk": "Corpus baseline ambiguity. fixtures/external-report.tsv currently tallies 2,892 Ok of 2,914 (99.24%), while SESSION_LOG continuation 14 records 99.3% (2,886) at Pass 2.2. The delta is consistent with Pass 2.3 (JPX) having shipped afterward, but the SESSION_LOG has no Pass 2.3 entry.",
      "mitigation": "Re-run tools/corpus-report and pin the exact Ok count as Pass 3.0's baseline BEFORE the round-trip gate is written. A gate whose denominator is uncertain cannot report an honest shortfall."
    }
  ],

  "standing_rules": [
    "R41 — No output fingerprint, enforced at the writer. Restates decision 001 §6.1 obligation 6 at its actual enforcement point: incremental save never rewrites /Info absent a real operator metadata change; full rewrite's /Producer is documented and overridable from both front ends; no build hash, edition marker, or non-suppressible producer id is ever emitted."
  ],

  "spec_prerequisites": [
    {
      "item": "WRITE-DIRECTION AUDIT of §7.5.4 (cross-reference table), §7.5.5 (file trailer), §7.5.6 (incremental updates), §7.5.8 (cross-reference streams)",
      "status": "All four files EXIST in D:\\Dev\\Rag-Specialized\\PDF_Spec\\iso32000\\ — but every one was built for the READ path during Pass 1. They are not known to cover emission constraints.",
      "needed": "Dispatch pdfce-spec-librarian to confirm each covers the write direction: the exactly-20-byte xref entry rule and its permitted EOL forms; subsection header syntax; startxref and %%EOF placement and the trailing-EOL rule; /Prev chaining for appended sections; /Size semantics on an incremental update; the xref-stream /W /Index /Filter /Predictor emission constraints; and free-entry (type 0) chain construction.",
      "blocking": true
    },
    {
      "item": "§7.5.8.4 hybrid-reference files (/XRefStm)",
      "status": "Covered only insofar as §7.5.8 covers it; the write-side question is unaddressed.",
      "needed": "Which section form an appended update must take when the input is hybrid, and what a conforming reader that honors only one of the two will see.",
      "blocking": true
    },
    {
      "item": "§14.4 file identifiers (/ID)",
      "status": "NOT in the RAG.",
      "needed": "Exact semantics of the two-element array, which element changes on save, and the generation recommendation. Also load-bearing for Pass 5 key derivation.",
      "blocking": true
    },
    {
      "item": "Annex F, Linearized PDF",
      "status": "NOT in the RAG. Referenced in docs only as a qpdf capability (PRIOR_ART line 227) and an Optimization backlog bucket.",
      "needed": "Enough to DETECT linearization reliably and state what an incremental append invalidates. Pass 3.0 does not write it, so a detection-level entry suffices.",
      "blocking": false
    },
    {
      "item": "§12.8 digital signatures — /ByteRange semantics only",
      "status": "NOT in the RAG.",
      "needed": "Only the byte-range coverage model, sufficient to justify R36's signatures-force-incremental rule. Full PAdES sourcing belongs to the Signatures Pass.",
      "blocking": false
    },
    {
      "item": "§7.6 encryption — the entire clause tree (7.6.1-7.6.7): standard security handler, Algorithms 2 / 2.A / 4 / 5, key derivation, RC4 and AES-CBC application, per-object keys, crypt filters, /EncryptMetadata, /Perms",
      "status": "NOT in the RAG. Only filters/filter__crypt.md exists, and that covers the /Crypt FILTER, not the encryption clause. This is the single largest spec gap across all four candidates.",
      "needed": "A full spec-librarian corpus-building session before Pass 5. Sizing this honestly is itself an argument for Pass 5's placement.",
      "blocking": false,
      "blocks": "Pass 5"
    },
    {
      "item": "Acrobat_Features RAG — 'Core document ops' bucket",
      "status": "The RAG is effectively EMPTY — LEGAL_NOTE.md, _TEMPLATE.md and index.md only. No feature content has been written yet.",
      "needed": "Dispatch pdfce-acrobat-librarian before Pass 3.2. NOT required for Pass 3.0 or 3.1: the writer is ARCHITECTURE §5 infrastructure, not an Acrobat-parity feature bucket, so the feature-fidelity standing rule does not bind it. Worth stating explicitly, because a reader could otherwise treat the empty RAG as blocking the whole sequence.",
      "blocking": false,
      "blocks": "Pass 3.2"
    },
    {
      "item": "qpdf as a structural oracle",
      "status": "Already cleared. PRIOR_ART.md line 227: Apache-2.0, 'safe to reuse code/tests directly, including literal algorithm porting, with attribution', and named 'the cleanest architecture model for pdfce-core's structural-integrity layer'.",
      "needed": "No new licensing work. Use qpdf --check as an external validator for written files, and its structural test material as portable test cases with attribution. Applies to Pass 3.x AND Pass 5 (qpdf implements encryption too).",
      "blocking": false
    },
    {
      "item": "PDF_Spec clause 11 (transparency) — for Pass 1.1 item 6.3, not for the writer",
      "status": "Flagged GAP. Item 6.3 explicitly requires a pdfce-spec-librarian dispatch before starting.",
      "blocking": false,
      "blocks": "Pass 1.1 item 6.3"
    },
    {
      "item": "Owed non-blocking spec debts carried from Passes 2.1/2.2",
      "status": "filter__jbig2.md Table 12 contents still unverified (Pass 2.2 implemented against §7.4.7 prose); filter__dct.md sourcing-gap closure owed per decision 006 §6.5.",
      "blocking": false
    }
  ],

  "operator_actions_owed": [
    "SESSION_LOG:879 records an action still open: surface to the operator that encrypted-PDF refusal (XrefErrorKind::EncryptionUnsupported) was added as an engineer-judgment scope addition. The operator has not been told that pdfce refuses encrypted files.",
    "Reconcile Pass 2.3 (JPX): the brief states it decodes; SESSION_LOG continuation 14 lists it as next and spec-blocked, with no shipped entry."
  ]
}
```

## Appendix B — Raw patch block (verbatim, for auditability)

The `_patch_to` block exactly as delivered in the consultation's final
message. Already merged into Appendix A above; retained so the merge
is auditable.

```json
{
  "_patch_to": "decision 007 JSON block",

  "first_pass_scope.deliverables.APPEND": [
    "No-output-fingerprint enforcement (decision 001 §6.1 obligation 6, whose enforcement point IS the writer): save_incremental NEVER rewrites /Info unless the operator actually changed metadata; save_full sets /Producer = 'pdfce <version>', documented and overridable via both the pdfce-core API and a pdfce-cli flag. No build hash, no edition marker, no non-suppressible producer id anywhere. This is the structural prevention of the exact behavior that disqualified oxidize-pdf as a foundation."
  ],

  "first_pass_scope.acceptance_criteria.APPEND": [
    "A test asserts that save_incremental on an unmodified document leaves /Info byte-untouched, and that save_full's /Producer is suppressible through both front ends."
  ],

  "pass_sequence.PATCH": {
    "Pass 4": {
      "correction": "text extraction has NO existing Backlog bucket — `ToUnicode` appears nowhere in ROADMAP, SESSION_LOG or any decision record. The librarian must CREATE the bucket before the Pass can be scoped. Only R17 (unicode-bidi permitted in a text-extraction reading-order path, forbidden in pdfce-render) and R7 (document text is a Pass-1-onward requirement) pre-constrain it."
    },
    "Pass 5": {
      "name_correction": "Encryption — decrypt all handlers; encrypt-on-save AES-128/256 ONLY. RC4 is read-compat only and is NEVER written, per the standing Backlog posture that R28 cites as its own precedent.",
      "dependency_citation": "decision 001 §6.2 is the authority, not PRIOR_ART alone: aes, cbc, sha2, md-5, rc4 are pre-selected for the security handler (cms, x509-cert, x509-parser for later PAdES)."
    },
    "Pass 6+": {
      "correction": "Two of candidate C's four items are ALREADY FILED as Pass 1.1 items, not new work: Type 3 fonts + Tr 4-7 clipping = Pass 1.1 item 4 (LOW, near-zero corpus presence); /SMask + /Mask = Pass 1.1 item 6.3, ranked immediately after the image codecs and BLOCKED on a pdfce-spec-librarian clause-11 dispatch. Only general transparency groups and blend modes are scoped nowhere. The reference-renderer pixel-parity harness is cheaper than stated: decision 006 §3.2 already ran an ad-hoc pypdfium2 comparison over 9 files, so the tooling precedent exists."
    }
  },

  "risks.APPEND": [
    {
      "id": "W15",
      "risk": "The proposed corpus round-trip gate is CI-enforced, and CI HAS NEVER RUN — pdfce has no git remote, and everything has been uncommitted across six continuations. A gate that exists only as a local command is a gate that silently stops running the first time someone is in a hurry.",
      "mitigation": "Pass 3.0 must make the round-trip harness runnable and green LOCALLY as a hard acceptance criterion, and must not depend on CI for its correctness claim. Whether to establish a remote is gated on LEGAL.md §1 (license undecided) and is the operator's call, not the engineer's — flag it, do not resolve it."
    },
    {
      "id": "W16",
      "risk": "Corpus baseline ambiguity. fixtures/external-report.tsv currently tallies 2,892 Ok of 2,914 (99.24%), while SESSION_LOG continuation 14 records 99.3% (2,886) at Pass 2.2. The delta is consistent with Pass 2.3 (JPX) having shipped afterward, but the SESSION_LOG has no Pass 2.3 entry.",
      "mitigation": "Re-run tools/corpus-report and pin the exact Ok count as Pass 3.0's baseline BEFORE the round-trip gate is written. A gate whose denominator is uncertain cannot report an honest shortfall."
    }
  ],

  "standing_rules.APPEND": [
    "R41 — No output fingerprint, enforced at the writer. Restates decision 001 §6.1 obligation 6 at its actual enforcement point: incremental save never rewrites /Info absent a real operator metadata change; full rewrite's /Producer is documented and overridable from both front ends; no build hash, edition marker, or non-suppressible producer id is ever emitted."
  ],

  "spec_prerequisites.APPEND": [
    {
      "item": "PDF_Spec clause 11 (transparency) — for Pass 1.1 item 6.3, not for the writer",
      "status": "Flagged GAP. Item 6.3 explicitly requires a pdfce-spec-librarian dispatch before starting.",
      "blocking": false,
      "blocks": "Pass 1.1 item 6.3"
    },
    {
      "item": "Owed non-blocking spec debts carried from Passes 2.1/2.2",
      "status": "filter__jbig2.md Table 12 contents still unverified (Pass 2.2 implemented against §7.4.7 prose); filter__dct.md sourcing-gap closure owed per decision 006 §6.5.",
      "blocking": false
    }
  ],

  "operator_actions_owed": [
    "SESSION_LOG:879 records an action still open: surface to the operator that encrypted-PDF refusal (XrefErrorKind::EncryptionUnsupported) was added as an engineer-judgment scope addition. The operator has not been told that pdfce refuses encrypted files.",
    "Reconcile Pass 2.3 (JPX): the brief states it decodes; SESSION_LOG continuation 14 lists it as next and spec-blocked, with no shipped entry."
  ]
}
```

## Orchestrator reconciliation note (2026-07-31, at archival)
Items in §11 and operator_actions_owed were grounded against SESSION_LOG Continuation 14; while this consultation ran, the following were resolved: Pass 2.3 (JPX) WAS shipped and is now filed (ROADMAP Shipped entry + SESSION_LOG Continuation 15, 2026-07-31) — the 'no Pass 2.3 entry' and 'corpus baseline ambiguity' items are closed: the confirmed post-2.3 baseline is Ok 2,892/2,914 (99.2%), images unsupported 3, codec-unsupported 0. filter__jbig2.md Table 12 was verified and closed by pdfce-spec-librarian the same day (one key, /JBIG2Globals; no code defects; jbig2.rs doc-comment paraphrase corrected same day). filter__dct.md's sourcing-gap closure was completed in a prior continuation. STILL OPEN: the encryption-refusal scope addition (SESSION_LOG:879) remains un-surfaced to the operator — carried on the operator-actions list; W15 (no git remote / CI never run / everything uncommitted) remains true and is the operator's call per LEGAL.md §1.
