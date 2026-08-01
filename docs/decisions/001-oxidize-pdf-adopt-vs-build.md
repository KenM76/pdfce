# 001 — `oxidize-pdf`: adopt as foundation, fork, or build from scratch?

- **Status:** Accepted
- **Date:** 2026-07-30
- **Deciders:** KenAgent (autonomous-builder), on request of `pdfce-engineer`
- **Supersedes:** the "OPEN QUESTION" section of `docs/PRIOR_ART.md` (2026-07-23)
- **Closes:** the GATE on `docs/ROADMAP.md` Pass 1; `ARCHITECTURE.md` §12
  entry (b) of 2026-07-23 ("decision DEFERRED")
- **Scope:** the foundation of `pdfce-core` only. Does not decide
  pdfce's own OSS license (`LEGAL.md` §1, still open), and is not
  affected by it.

---

## 1. Context

`pdfce-core` is the crate that owns the PDF Carousel Object System
(COS) model, the tokenizer, the cross-reference machinery, the filter
decoders, fonts, encryption, signature handling, and the
content-stream model. It is the load-bearing crate: `pdfce-render`,
`pdfce-gui`, `pdfce-cli`, and the eventual `pdfce-web` WASM fork all
sit on top of it, and it is the crate whose design determines whether
the project's two hard invariants are achievable at all.

Pass 0 (2026-07-23) deliberately shipped a **header-probe-only**
`pdfce-core` — `probe_header`, `probe_file`, `PdfVersion`, `PdfError`,
and nothing else — specifically so that this decision would not be
made implicitly by writing a from-scratch parser before the question
had been asked. `docs/PRIOR_ART.md` recorded the reason:

> `oxidize-pdf` (MIT) may already cover most of pdfce-core's target
> scope — flagged as an open question requiring a dedicated audit
> before Pass 1, not yet decided. […] **Do not silently default to
> "build from scratch" without this audit having happened.**

That audit was performed 2026-07-30 against a shallow clone of
`bzsanti/oxidizePdf` HEAD `5f3e8b3` (v4.2.1, MIT). This record is the
resolution.

Two things changed the shape of the question between 2026-07-23 and
now:

1. **The audit produced concrete evidence** rather than the project's
   own marketing claims (99.3% parse success on 9,000+ PDFs, 7,993
   tests) that `PRIOR_ART.md` had to take at face value.
2. **Project scope expanded on 2026-07-30** to include Inkscape-level
   vector editing of PDF page content (`ROADMAP.md`, "Vector graphics
   editing (Inkscape-parity)"). That bucket's binding note (b) —
   "this scope raises the bar on `pdfce-core`'s content-stream model —
   it must support full round-trip decomposition of graphics operators
   into editable objects and minimal-diff re-emission" — turns out to
   be decisive here, for reasons §5.3 explains.

### 1.1 The invariants this decision has to serve

| # | Invariant | Source |
|---|---|---|
| I1 | **Round-trip / minimal-diff.** Objects pdfce did not logically modify re-emit byte-identical (full rewrite) or are omitted entirely (incremental save). Never normalize a PDF's structure as a side effect of an unrelated edit. | `ARCHITECTURE.md` §5 |
| I2 | **Incremental save is the DEFAULT save mode**, not an optimization — Acrobat's digital-signature model covers a byte range, so anything that perturbs earlier bytes invalidates signatures. | `ARCHITECTURE.md` §4, §5 |
| I3 | **GUI-core separation.** Zero windowing/GUI crates in `pdfce-core` / `pdfce-render`'s dependency tree, verified by `cargo tree`, not assumed. | `ARCHITECTURE.md` §3 |
| I4 | **Fail-clean parsing.** No silent data-corruption fallbacks; resource-limit guards on every decoder; fuzz target before Pass 1 ships. | `ARCHITECTURE.md` §10 |
| I5 | **Dirty set = structural diff against the base revision at save time**, never the union of every command ever run. | `ARCHITECTURE.md` §11.1 |
| I6 | **Permissive licenses only** (pending `LEGAL.md` §1); copyleft flagged to the operator, never decided solo; attribution generated via `cargo-about`. | `LEGAL.md` §6 |
| I7 | **No unknown-provenance PDFs in `fixtures/`.** | `LEGAL.md` §5 |
| I8 | **Content streams must decompose into an editable object model AND re-serialize with minimal diff** (new, 2026-07-30). | `ROADMAP.md` Inkscape-parity bucket, note (b) |

Any option that cannot serve I1/I2/I8 is not a candidate, regardless
of how much else it offers — those three are the whole product thesis.

---

## 2. Options considered

**(a) Depend** — add `oxidize-pdf` to `pdfce-core`'s `Cargo.toml` and
build pdfce's API surface as a thin layer over it.

**(b) Fork / vendor** — copy the repository into `crates/pdfce-core`
(or `vendor/`), take ownership, and evolve it toward pdfce's
requirements.

**(c) Reference-only** — build `pdfce-core` from scratch, using
`oxidize-pdf` as MIT-licensed prior art: read it for design
understanding, use it as a differential test oracle, and selectively
port individual modules with attribution where that is genuinely the
best engineering answer.

---

## 3. Evidence (audit of HEAD `5f3e8b3`, v4.2.1, 2026-07-30)

### 3.1 What is real and good

This is a substantial, non-trivial project and the audit should not be
read as dismissive of it:

- Substantial parser: COS model, classic + stream + **hybrid** xref
  (`xref.rs` alone is ~3,200 lines), a dedicated recovery module,
  object streams read **and** write.
- Real pure-Rust **JBIG2** (~8,000 lines) and **CCITT** (~1,000 lines)
  decoders.
- Real TrueType / CFF (including CID) font **subsetting**.
- Encryption read **and** write: RC4 R2–R4, AES-128 R4, AES-256 R5/R6.
- PKCS#7 / CMS signature **verification**.
- Content-stream parser producing a full `ContentOperation` enum
  (3,310 lines).
- ~9,371 `#[test]` attributes, `proptest` usage, two `cargo-fuzz`
  targets.
- Pure-Rust, permissive dependency tree; **no GUI dependencies**
  (would satisfy I3).
- Strong module-level documentation in the newer code.
- MSRV 1.88 — compatible with pdfce's pinned 1.97.1 / `rust-version`
  1.92 (a non-issue, recorded for completeness).

### 3.2 Disqualifying findings

**D1 — Round-trip fails by design.** The project carries **two
disconnected object models**: a read-only parser model and a
from-scratch generation model, with no bridge between them. There is
no `Document::load()` — its own doc comments mark it *"STATUS: Not yet
implemented."* This is not a missing feature at the edge of the API;
it is the absence of the single operation pdfce is built around
(open an existing file → edit → write it back).

**D2 — General incremental save does not exist.** The one API named
"incremental" appends pages while **dropping `/Outlines`, `/AcroForm`,
and `/Names` from the new catalog** — admitted in-code. Under I1/I2
that is not an incomplete implementation, it is silent document
destruction. The single correct incremental path in the codebase is
`IncrementalFormFiller` (AcroForm field fill only, ~1,084 lines),
which is genuinely exemplary ISO 32000-1 §7.5.6 append work over a
byte-verbatim base — and whose own module documentation concedes that
rehydrating a writable `Document` from an arbitrary parsed PDF *"would
corrupt complex documents on round-trip."* The upstream author has
diagnosed the same limitation we did.

**D3 — No content-stream serializer.** The operator model is read-only.
The write side is an unrelated from-scratch builder. The one place the
two meet — "page extraction" — replays a *subset* of operators, maps
fonts to standard-14 **approximations**, and inserts a literal
`"[Page extracted]"` placeholder on failure. Against I8 (Inkscape
parity) this is the opposite of what pdfce needs.

**D4 — Non-suppressible output fingerprint.** Every written PDF
receives a build-hash / edition watermark in the `/Info` dictionary,
deliberately not exposed in the public API (described upstream as
"resistant to spoofing"). Three separate problems: it modifies an
object pdfce did not logically modify (violates I1 outright on every
incremental save), it embeds a third-party identifier in the
operator's documents without their consent (against the spirit of
`ARCHITECTURE.md` §1.1's privacy posture), and it is by design not
removable by a downstream consumer.

**D5 — Silent-corruption filter fallbacks.** On zlib/predictor failure
the decoder returns the raw, undecoded bytes as though they had
decoded successfully. This is the precise inverse of I4. It is also
the worst possible failure mode for a differential oracle, and it
means every disagreement between our parser and theirs has to be
adjudicated against the spec rather than assumed to be our bug.

**D6 — Code-health and scope signals.** ~180 `unwrap`/`expect` in
library code; three overlapping object-model families; 40+ public
modules including `ai`, `dashboard`, `charts`, `invoice` — scope
sprawl around an "AI/RAG" pivot. The maintainer's current investment
axis is **text extraction**, not **edit fidelity**. That is a
legitimate product choice upstream and a direct misalignment with
pdfce's.

**D7 — Sustainability.** Bus factor 1 (1,114 commits versus the next
human's 3). 94 releases in 12.5 months, including **four major version
bumps in the first year**. For a foundation crate that pdfce would
carry for a decade, that churn rate is a standing tax.

**D8 — Open-core misalignment on the exact feature that matters most.**
"Digital signatures" sits on the commercial PRO tier; the MIT signing
code is a placeholder stub that writes zeroed bytes. PAdES signing is
the single feature for which I2 (incremental save) is load-bearing.
Depending on this crate would mean structurally depending on a project
whose business model gives it a reason **not** to ship, in the MIT
tree, the thing pdfce most needs.

**D9 — Repository contains unknown-provenance real-world PDFs.**
Directly incompatible with I7 if the repository were vendored. The
claimed 9,000-PDF validation corpus is maintainer-local and
unverifiable.

### 3.3 Ecosystem context

`hayro-write` (MIT OR Apache-2.0, Typst family) now performs
parse → rewrite via `pdf-writer`, but **full re-serialization only**.
Confirmed across this audit and the 2026-07-23 survey: **nobody in the
Rust ecosystem has signature-safe, byte-preserving incremental save.**
That gap is not an oversight pdfce can shop around for — it is pdfce's
differentiation, and building it is unavoidable under any of options
(a), (b), or (c).

---

## 4. Decision

**Option (c): reference-only. Build `pdfce-core` from scratch.**

Concretely:

1. **No `oxidize-pdf` in any shipping dependency graph.** `cargo tree`
   on all four crates must never show it.
2. **No fork, no wholesale vendor.**
3. **Zero literal code ports planned at Pass 1**, with two narrowly
   gated conditional candidates recorded in §6.3.
4. **Use it as an out-of-tree differential test oracle** (§7).
5. **Where oxidize-pdf holds a genuinely valuable self-contained
   subsystem, prefer the already-maintained permissive upstream crate**
   over a vendored copy — see §6.

---

## 5. Rationale

### 5.1 Options (a) and (b) fail on the invariants, not on taste

Option (a) **depend** is eliminated three times over independently:
by D1 (no `Document::load()` — the entry point does not exist), by D2
(no general incremental save, and the one that exists destroys
document structure), and by D4 (an unremovable third-party fingerprint
written into every output file, which violates I1 on literally every
save). Any one of those alone would be sufficient. D8 adds that the
gap most relevant to pdfce is one the upstream project has a business
reason to keep closed.

Option (b) **fork/vendor** is eliminated by the shape of what would be
inherited. The defect in D1 is not a bug to fix, it is the
architecture: two disconnected object models with no bridge. Repairing
it means designing and building the retained-byte `Document` and the
structural-diff writer — which *is* the from-scratch work — and then
additionally paying to reconcile the result with three overlapping
legacy object-model families, ~180 library `unwrap`s, 40+ modules of
out-of-scope surface (`ai`, `dashboard`, `charts`, `invoice`), and a
filter layer whose failure semantics (D5) are the exact inverse of I4.
A fork costs *more* than a clean build and yields a codebase whose
documentation cannot honestly satisfy the project's documentation-first
bar, because half of it would be inherited code nobody on this project
can explain. D9 adds a hard legal blocker: vendoring the repository
imports unknown-provenance real-world PDFs, which `LEGAL.md` §5
forbids.

### 5.2 The correction to "port the valuable modules"

The engineer's read proposed porting JBIG2, CCITT, hybrid-xref
handling, AES-256 R6 key derivation, CFF/TrueType subsetting, and
`incremental_form_fill.rs`. That instinct is sound — MIT genuinely
permits it, and those *are* the strong parts — but it does not survive
contact with `PRIOR_ART.md`'s own findings. Every item on that list is
either **(a) already better served by a maintained permissive crate**,
or **(b) spec-governed work that project rule 1 requires we derive
from the spec RAG**, or **(c) architecturally the wrong shape** for
pdfce:

| Candidate | Verdict | Because |
|---|---|---|
| JBIG2 (~8k lines) | **Don't port** | `hayro-jbig2` (Apache-2.0 OR MIT) already exists and is maintained by a team, per `PRIOR_ART.md`. Vendoring 8,000 lines we would then own and debug forever, from a bus-factor-1 source of unverified upstream provenance, is strictly worse than a dependency. |
| CCITT (~1k lines) | **Don't port** | `hayro-ccitt`, same reasoning. |
| CFF/TrueType subsetting | **Don't port** | `subsetter` (Typst, MIT OR Apache-2.0) is purpose-built for exactly PDF embedding and is what Typst ships; `allsorts` (Apache-2.0) is the fallback. |
| AES-256 R5/R6 key derivation | **Don't port** | Spec-governed (ISO 32000-2 §7.6.4, Algorithm 2.A/2.B). Project rule 1 forbids implementing spec-governed behavior from anything but the spec RAG. Their implementation is a legitimate *cross-check* when our own hardened-hash loop disagrees with a fixture — that is all. |
| `ContentOperation` enum | **Don't port** | Wrong shape — see §5.3. |
| Hybrid-xref + recovery heuristics | **Extract as knowledge, not code** | This is the one place oxidize-pdf holds something the spec RAG structurally *cannot* give us: empirical knowledge of how real-world files are broken. But empirical knowledge belongs in `C:\personal_rag\pdf\` as documented behaviors and regression fixtures — not as a vendored 3,200-line file. |
| `incremental_form_fill.rs` | **Study, cite, reimplement** | Genuinely the best worked ISO 32000-1 §7.5.6 example found anywhere in the Rust ecosystem, and worth reading closely. But it is a *special case* (AcroForm field fill over a byte-verbatim base), and pdfce's writer is a *general* structural-diff writer over a retained-byte `Document`. pdfce's version is not a generalization of theirs; it is a different design that happens to honor the same §7.5.6 byte contract. Extract the byte-layout checklist — `startxref` chaining, `/Prev` linkage, `/ID` preservation, trailer duplication rules — cite the file as prior art in our module docstring, and write our own. |

The general principle this establishes, worth carrying to future
prior-art decisions: **a port is a permanent maintenance liability
disguised as a one-time saving.** Porting is right when no maintained
alternative exists AND the code is architecturally aligned. Here,
neither condition holds for anything material.

### 5.3 The Inkscape-parity scope expansion is decisive

The strongest single argument against options (a) and (b) is the
newest one, and it is easy to miss because the module in question is
one of oxidize-pdf's *better* pieces.

Its `ContentOperation` enum is a **lossy semantic model with no
serializer**. It answers "what does this content stream draw?" — which
is exactly right for text extraction and RAG, the maintainer's actual
product.

pdfce needs it to answer a different question: **"how do I change one
path node on page 3 and leave the other several thousand operators on
that page byte-identical?"** That requires a *lossless, byte-span-
provenanced token model* in which every operator retains the source
byte range it was parsed from, with the editable semantic operator
view existing as a **projection** over those tokens rather than as the
primary representation. Then re-serialization emits verbatim spans for
untouched tokens and re-encodes only the ones actually edited.

This is the mechanical enactment of I1 at the content-stream level,
and after 2026-07-30 it is also the enabler for the entire
Inkscape-parity bucket (node/Bézier editing, boolean path ops,
gradients, z-order manipulation, OCG layers) — every one of which is an
edit to a *few* operators inside a stream containing *many*.

Adopting oxidize-pdf's operator model would not merely fail to help
here; it would actively fight the invariant, because the first
requirement of the model we need is precisely the information their
model discards.

### 5.4 What the licensing analysis does and does not say

`oxidize-pdf` is MIT. MIT permits literal copying with attribution.
There is **no clean-room requirement here** — this is categorically
unlike the MuPDF / Ghostscript / Poppler / Inkscape situation, where
`PRIOR_ART.md` and the Inkscape bucket's note (a) correctly warn
against transliteration. Reading it, copying from it, and shipping the
result are all legal, under any license pdfce eventually chooses
(`LEGAL.md` §1 — MIT prior art is compatible with a permissive *or* a
copyleft pdfce, so this decision is not gated on that open item and
does not gate it).

That matters because it means **the near-zero-port conclusion must be
argued and defended on engineering grounds, not legal ones.** A future
session that reopens this should not be able to say "we didn't port it
because of licensing" — we didn't port it because of maintenance
ownership, architectural fit, and upstream provenance uncertainty.

The one real legal caveat: **MIT at the repository root does not prove
clean provenance in every subtree.** Before any future literal port,
that specific module's own file headers, `NOTICE`, and git history must
be checked for derivation from GPL/AGPL sources (`jbig2dec`,
Ghostscript, Poppler, MuPDF). The JBIG2 decoder is the obvious place
this could bite, given that `PRIOR_ART.md` already flags GPL-3.0
`jbig2dec` as the ecosystem's dominant JBIG2 implementation and
`nipdf` as a live example of exactly that contamination path.

### 5.5 What we lose, honestly stated

This decision accepts real cost, and the operator should see it named
rather than buried:

- **Roughly 12,000+ lines of working decoder and font code** that would
  have to be re-obtained from other crates or written. Mitigated —
  `hayro-jbig2`, `hayro-ccitt`, `subsetter`, and `allsorts` cover most
  of it and cover it better — but not eliminated.
- **A tested COS parser and hybrid-xref implementation** that
  demonstrably works on a large real-world corpus. Partially mitigated
  by the differential oracle (§7), which lets us borrow the *validation
  signal* without borrowing the *code*.
- **Confirmation that pdfce is now committed to being the first project
  in the Rust ecosystem with signature-safe, byte-preserving
  incremental save.** Nobody upstream has it — including `hayro-write`,
  which does full re-serialization only. This is genuine
  differentiation and genuine solo engineering cost, and it should be
  an accepted cost rather than an assumption that someone else's crate
  will eventually cover it.

---

## 6. What this decision produces

### 6.1 Architecture requirements created (the Pass 1 payload)

These are the concrete design obligations that follow. All must land
in Pass 1 even though Pass 1 is a read-only viewer, because every one
of them is expensive-to-impossible to retrofit.

1. **`ByteSpan` provenance is a first-class field, not an
   optimization.** Every indirect object parsed from the source buffer
   retains `ByteSpan { start, len }` plus the retained original
   buffer. Save-time rule: an object that is span-backed and
   structurally equal to the base revision re-emits its source bytes
   verbatim (full rewrite) or is omitted entirely (incremental save).
   This is the mechanical enactment of I1 and I5 — a parser written
   without provenance has to be rewritten to gain it.
2. **Lossless content-stream token model with per-token spans**, with
   the semantic operator view as a projection over it. See §5.3.
3. **ONE object model.** Exactly one `Document` type that is
   simultaneously the parse result and the write source. Recorded in
   `ARCHITECTURE.md` §4 as a named invariant, because D1 shows precisely
   what happens when a project accretes a second, builder-only model
   "just for generation": the two drift, the bridge never gets built,
   and round-trip becomes structurally impossible.
4. **Fail-clean filters as a type-level contract.** Every decoder
   returns `Result<Vec<u8>, FilterError>`; no code path returns
   undecoded or partial bytes on failure. One regression test per
   filter asserting that a corrupted stream yields `Err`, not
   plausible-looking garbage. Call this out in the module docstring as
   a deliberate divergence from observed prior art (D5).
5. **Lint policy over `unwrap`.** `#![deny(clippy::unwrap_used,
   clippy::expect_used, clippy::panic, clippy::indexing_slicing)]` in
   `pdfce-core` (allowed under `#[cfg(test)]`). Turns D6's finding into
   enforcement and directly serves `ARCHITECTURE.md` §10.
6. **No output fingerprint.** pdfce writes no build hash, no edition
   marker, no non-suppressible producer identifier. On **incremental
   save the `/Info` dictionary is not rewritten** unless the operator
   actually changed metadata — rewriting it would touch an object we
   did not logically modify, violating I1 on every save. On full
   rewrite, `/Producer` is set to `pdfce <version>`, documented, and
   overridable via API and CLI flag. (D4, inverted into policy.)

### 6.2 Dependencies this decision selects

Adopted at Pass 1 (re-verify version and license at `Cargo.toml` time
per `LEGAL.md` §6.2): `flate2` with the `miniz_oxide` or `zlib-rs`
backend — **never** the C `zlib`/`zlib-ng` backend, which would break
the WASM fork; `tiny-skia` in `pdfce-render`.

Recorded now as the chosen answers for later Passes, in place of
ported oxidize-pdf modules: `hayro-ccitt`, `hayro-jbig2`,
`subsetter` (with `allsorts` as fallback), and the RustCrypto stack
(`aes`, `cbc`, `sha2`, `md-5`, `rc4`, `cms`, `x509-cert`,
`x509-parser`) for the security handler and PAdES work.

### 6.3 The two conditional port candidates

Neither is authorized now; both are recorded so a future session does
not have to re-derive the analysis.

- **xref recovery heuristics.** Gate: only after pdfce's own parser is
  measured against the veraPDF and PDF Association corpora. If our
  parse-success rate on broken/hybrid files materially lags (say below
  95%), harvest the *specific* heuristics as documented behaviors into
  `C:\personal_rag\pdf\` and implement independently. Port code only if
  that fails too.
- **JBIG2.** Gate: only if `hayro-jbig2` fails validation against our
  corpus, AND oxidize-pdf's decoder passes an upstream-provenance check
  proving it is not derived from GPL `jbig2dec`.

### 6.4 Attribution mechanics if a port ever happens

`cargo-about` generates `THIRD_PARTY_LICENSES.md` from `Cargo.lock` and
is therefore **blind to vendored source files**. Any ported fragment
must be attributed by hand: an MIT notice plus the
`bzsanti/oxidizePdf` copyright line in a clearly marked "vendored
source" section of `THIRD_PARTY_LICENSES.md`, plus a provenance
statement in the receiving file's header docstring (upstream path,
commit SHA, date, and what was changed). Recorded here because it is
exactly the kind of compliance gap that a generated-attribution
workflow creates a false sense of security about.

---

## 7. The differential test oracle

The audit's most useful practical output is not code — it is that
oxidize-pdf's COS parser and hybrid-xref handling are strong enough to
be worth *disagreeing with deliberately*.

**Mechanism.** An out-of-tree crate (`tools/difftest/`, listed under
`[workspace] exclude`) pinned to an exact `oxidize-pdf` version, which
runs pdfce's fixtures through both parsers and diffs the resulting COS
object graphs.

**Why out-of-tree, specifically.** Three reasons, all load-bearing:
it keeps `cargo test` fast and dependency-light; it keeps the I3
`cargo tree -p pdfce-core` invariant checks clean and unambiguous; and
it keeps `cargo-about`'s generated `THIRD_PARTY_LICENSES.md` an
accurate picture of the **shipping** dependency graph — which is what
`LEGAL.md` §6.3 says that file is for. A `[dev-dependencies]` entry
would pollute all three.

**The oracle is advisory, never authoritative.** D5 means oxidize-pdf
will confidently return wrong-but-plausible bytes on some malformed
inputs. Every disagreement is adjudicated against the `PDF_Spec` RAG,
never resolved in the oracle's favor by default.

**Fixture rule (I7).** Differential fixtures are pdfce-authored
synthetic PDFs, or files from the veraPDF / PDF Association corpora
with clear redistribution rights. **Never** a PDF taken from
oxidize-pdf's own repository — that repository commits
unknown-provenance real-world files (D9), and pulling one into
`fixtures/` would violate `LEGAL.md` §5 just as surely as downloading
it from the open web.

**Stand it up before the parser is written**, not after. It is cheapest
at Pass 1 and it de-risks the from-scratch parser at exactly the point
where oxidize-pdf is genuinely strong.

---

## 8. Consequences

**Positive**

- Every invariant I1–I8 is achievable by construction rather than by
  retrofit. The `ByteSpan` and token-model requirements exist because
  the audit showed concretely what their absence costs (D1, D3).
- No bus-factor-1 dependency under the whole product (D7); no
  structural dependency on a project with a commercial incentive to
  withhold the feature we most need (D8).
- Documentation-first is satisfiable. Every line of `pdfce-core` is
  explicable by this project, which the reconstruction test requires
  and which a fork could not deliver.
- The Inkscape-parity bucket becomes tractable rather than blocked,
  because the content-stream model is designed for it from the first
  commit.
- Free choice of maintained upstream crates for codecs and fonts,
  rather than ownership of vendored copies.
- Zero third-party fingerprints in operator documents; `/Info`
  untouched on incremental save.

**Negative**

- Materially more engineering work at Pass 1 and beyond. Accepted
  deliberately; the alternative was more work wearing a cheaper
  disguise (§5.1).
- pdfce carries sole responsibility for parser robustness against
  real-world malformed files, without oxidize-pdf's corpus-hardened
  head start. Partially mitigated by the oracle (§7), the veraPDF
  corpus, and `cargo-fuzz` (required for Pass 1 to ship per
  `ARCHITECTURE.md` §10.2).
- Confirmed first-in-ecosystem status for signature-safe incremental
  save means no upstream to learn from beyond ISO 32000-1 §7.5.6 and
  one exemplary 1,084-line special case.

**Neutral**

- Independent of `LEGAL.md` §1. MIT prior art is compatible with any
  license pdfce eventually chooses, so this decision neither waits on
  that one nor constrains it.
- `oxidize-pdf` remains a valuable, actively developed MIT project
  serving a different product goal well. Nothing here is a quality
  judgment on it — the mismatch is between its architecture and
  pdfce's invariants, and it is mutual.

---

## 9. Revisit triggers

Re-open this record if any of the following becomes true:

1. **oxidize-pdf ships a real `Document::load()` plus a general
   incremental save** that preserves untouched bytes without dropping
   `/Outlines` / `/AcroForm` / `/Names`, **and** removes the `/Info`
   build-hash fingerprint. That combination — and only that
   combination — would make "depend" worth re-auditing.
2. **`hayro-write` gains byte-preserving incremental append.** The most
   likely place in the ecosystem for this capability to appear from a
   multi-maintainer team. If it lands, evaluate depend-or-contribute
   rather than continuing solo.
3. **Any maintained permissive crate lands PAdES-conformant signing**
   (confirmed absent ecosystem-wide as of 2026-07-30). Adopt rather
   than build. Note that oxidize-pdf specifically will not be that
   crate — D8 is structural, not a backlog item.
4. **oxidize-pdf reaches sustained multi-maintainer status** (2+ humans
   with meaningful ongoing commit share) *and* 12 months without a
   major version bump. D7 is the largest non-technical disqualifier and
   it is the one most capable of changing.
5. **pdfce's own parser underperforms** against the veraPDF / PDF
   Association corpora relative to oxidize-pdf's claimed 99.3% →
   trigger the §6.3 xref-recovery harvest.
6. **oxidize-pdf relicenses or goes source-available** for a future
   major version. Cannot retroactively affect the audited HEAD
   (`5f3e8b3`, MIT), but would end its usefulness as an oracle —
   snapshot oracle outputs before this becomes a live risk.

---

## 10. Follow-up actions

**Engineering (Pass 1):** implement §6.1 items 1–6; add the §6.2
dependencies; stand up the §7 oracle before the parser is written;
dispatch `pdfce-spec-librarian` for a pre-read of ISO 32000-1 §7.5.4
(xref table), §7.5.5 (file trailer), §7.5.6 (incremental updates),
§7.5.7 (object streams), §7.5.8 (xref streams), and the
hybrid-reference `/XRefStm` mechanism — the byte-layout contract the
`ByteSpan` design must honor.

**Librarian (`pdfce-librarian`):** archive this record to
`docs/decisions/001-oxidize-pdf-adopt-vs-build.md`; rewrite
`PRIOR_ART.md`'s "OPEN QUESTION" section as **RESOLVED** with a pointer
here and change oxidize-pdf's verdict cell from `evaluate` to
`reference-only`; add a dated `ARCHITECTURE.md` §12 entry; strike the
GATE bullet from `ROADMAP.md` Pass 1 and record it closed; create
`C:\personal_rag\pdf\` with its `index.md` per
`C:\personal_rag\README.md`, seeded with the hybrid-xref/recovery
behaviors worth knowing and the observed silent-corruption-fallback
antipattern.

**Operator check-in (do not decide solo):** pdfce is now committed to
being the first project in the Rust ecosystem to implement
signature-safe, byte-preserving incremental save. The operator should
know this is a deliberate accepted cost, not an assumption that an
upstream crate will cover it later.

---

## 11. References

- `docs/ARCHITECTURE.md` §3 (workspace + GUI-core invariant), §4 (core
  data model), §5 (round-trip / minimal-diff), §10 (adversarial
  hardening), §11 (undo + dirty-set), §12 (decision log)
- `docs/PRIOR_ART.md` — "OPEN QUESTION" (superseded by this record),
  core-crate table, codec/font/crypto tables, copyleft landmines
- `docs/ROADMAP.md` — Pass 1 "Next up" + GATE; "Vector graphics editing
  (Inkscape-parity)" backlog bucket, notes (a)–(c)
- `docs/LEGAL.md` §1 (license undecided), §5 (test-corpus sourcing),
  §6 (dependency licensing + generated attribution)
- `CLAUDE.md` rules 1 (spec fidelity), 2 (GUI-core separation),
  3 (round-trip), 8 (license undecided), 10 (Rust style/API),
  13 (dependency licensing)
- Audit target: `bzsanti/oxidizePdf` HEAD `5f3e8b3`, v4.2.1, MIT,
  audited 2026-07-30
- ISO 32000-1:2008 §7.5.6 (incremental updates) — the byte contract
  both `IncrementalFormFiller` and pdfce's writer honor
