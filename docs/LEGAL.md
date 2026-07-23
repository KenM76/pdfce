# pdfce — Legal posture

This file exists because pdfce sits at the intersection of three legal
concerns most projects don't have all at once: (1) it aims to be
open-source and eventually public, (2) its entire purpose is
implementing a copyrighted, partially-paywalled ISO standard, and (3)
it deliberately targets feature parity with a specific commercial
product. None of this blocks the project; all of it needs a documented,
consistent posture so nobody (human or LLM) makes an ad hoc call under
time pressure that creates exposure later.

## 1. Open-source license — OPEN DECISION, ask the user before first commit

pdfce does not yet have a license. **Do not commit code to a public
repository, do not publish a release, do not accept the project being
"open source" as a settled fact in any user-facing copy** until the
user picks one. This is a case of the global "claim-bearing copy"
rule: license terms are a claim, not a detail to default plausibly.

Realistic candidates to present to the user when this comes up:

- **MIT or Apache-2.0** (or dual MIT/Apache-2.0, the common Rust-
  ecosystem default) — maximally permissive, easiest for others to
  build on or embed pdfce-core in their own tools (including a
  hypothetical future commercial fork by someone else — the user
  should decide if that's acceptable).
- **GPL-3.0 or AGPL-3.0** — copyleft; blocks proprietary forks/embeds
  without also open-sourcing them. AGPL specifically closes the
  "run it as a hosted service without releasing source" loophole,
  which is directly relevant if a future web-fork SaaS competitor
  tried to build on pdfce without contributing back.

Whichever is chosen, add a `LICENSE` file at the repo root and a
one-line `license` field in `Cargo.toml`, and note the decision + date
in `ARCHITECTURE.md` §12 Decision log.

## 2. ISO / ITU-T / ETSI standard copyright — how the spec RAG is scoped

The PDF ecosystem's normative documents have genuinely mixed licensing:

| Document | Publisher | Free to download? |
|---|---|---|
| ISO 32000-1:2008 (PDF 1.7) | ISO, but originally Adobe-authored | **Yes** — Adobe published the identical text freely; this is the practical primary source for the PDF 1.7 baseline. |
| ISO 32000-2:2020 (PDF 2.0) | ISO | **No** — paywalled (~200 CHF), not freely redistributable. |
| ISO 19005-1/2/3/4 (PDF/A) | ISO | **No** — paywalled. Free secondary sources exist (PDF Association technical notes, veraPDF validation rules/corpus, the Isartor test suite) that encode most of the same normative content in a legitimately free/open form. |
| ISO 14289 (PDF/UA) | ISO | **No** — paywalled. Free secondary sources: PDF Association technique documents, PAC (PDF Accessibility Checker) documentation. |
| ETSI EN 319 142-1/2 (PAdES) | ETSI | **Yes** — ETSI's standard practice is free publication. |
| ITU-T T.4 / T.6 (CCITT Group 3/4) | ITU-T | **Yes** — ITU-T Recommendations are freely downloadable from itu.int. |
| ITU-T T.88 (JBIG2) | ITU-T | **Yes** — same. |
| ITU-T T.81 (JPEG) / ISO 10918-1 | ITU-T (free) vs ISO (paywalled) — identical content | **Yes, via the ITU-T copy.** |
| ITU-T T.800 (JPEG2000) / ISO 15444-1 | ITU-T (free) vs ISO (paywalled) — identical content | **Yes, via the ITU-T copy.** |
| Adobe XMP Specification (Parts 1-3) | Adobe | **Yes** — Adobe publishes this directly. |
| ICC.1:2022 (ICC profile format) | International Color Consortium | **Yes** — color.org publishes it free. |
| OpenType spec | Microsoft (practical source) vs ISO/IEC 14496-22 (paywalled, same content) | **Yes, via Microsoft's free copy.** |
| Adobe Supplements / Extensions to ISO 32000, legacy XFA spec | Adobe | **Yes** (historically published freely, though URLs move — verify current location, don't trust a stale link). |

**The pattern:** wherever ISO paywalls a standard whose content
originated with (or is mirrored by) a standards body with an open-
publication norm (ITU-T, ETSI) or the original corporate author
(Adobe, Microsoft, the ICC), prefer that free source. `pdfce-spec-
librarian` owns applying this table — see its agent file for the
full sourcing protocol.

**Redistribution rule (binding on `pdfce-spec-librarian`):**

- RAG files may **paraphrase and summarize** normative content, and
  may include **short verbatim quotations** (a sentence, a table row)
  with a clear citation (document + clause/section/table number).
  This is standard, low-risk technical-reference practice.
- RAG files must **not** bulk-copy multi-paragraph verbatim text from
  a paywalled source (ISO 32000-2, ISO 19005, ISO 14289) into the RAG.
  For those, work from the free secondary sources (PDF Association /
  veraPDF notes, the freely-available ISO 32000-1 baseline plus public
  PDF-2.0 delta summaries) and mark paraphrased sections as such.
- The raw source documents themselves (whether freely downloaded or,
  if the user owns a purchased ISO copy, provided locally) are staged
  under `D:\Dev\Rag-Specialized\PDF_Spec\_sources\` and are **never**
  committed to the pdfce git repository and **never** referenced from
  any pdfce release artifact.
- The RAG directory `D:\Dev\Rag-Specialized\PDF_Spec\` itself lives
  **outside** the pdfce repository. If it is ever put under version
  control for the user's own backup purposes, that repository must be
  **private**, never public, never a release asset — same discipline
  as the existing "SolidWorks tools are PRIVATE" rule in the user's
  global CLAUDE.md, applied here to licensed reference material
  instead of proprietary work product.

## 3. Patent posture (brief, non-exhaustive — flag to the user if a specific filter/codec raises a real question)

- **JBIG2** arithmetic coding (MQ-coder) patents have expired; treat as
  clear, but if implementing JBIG2 symbol/refinement coding raises a
  specific still-live patent question, stop and ask rather than assume.
- **CCITT Group 3/4** — decades-old ITU-T fax standards, no known live
  patent concerns.
- **JPEG (baseline DCT)** — the historical JPEG patent disputes (e.g.
  Forgent Networks) are long resolved/expired; treat as clear.
- **JPEG2000** — most core patents have expired given its age; if a
  specific optional feature (e.g. certain wavelet variants) is flagged
  by a crate's own documentation as patent-encumbered, respect that
  crate's guidance rather than re-deriving a legal opinion from scratch.
- This section is a starting orientation, not a legal opinion. If a
  genuine patent-risk question comes up for a specific feature, that's
  a "ask the user" moment, not a "the engineer decides" moment — patent
  risk is qualitatively different from the usual engineering judgment
  calls this project's agents make solo.

## 4. Trademark posture

- "Acrobat", "Adobe", the Adobe PDF logo, and Adobe's product UI/icon
  trade dress are **not** to be used in pdfce's name, branding,
  marketing copy, icons, or about-box text. "Feature-for-feature
  replacement for Acrobat Pro" is fine as an internal engineering/
  roadmap framing (as used in `ARCHITECTURE.md` and `ROADMAP.md`); it
  needs softer, non-infringing phrasing in any public-facing copy
  ("a free, open-source alternative to commercial PDF editors" style) —
  this is a `pdfce-librarian` / user judgment call when public-facing
  copy is actually drafted, not a concern for internal engineering docs.
- The PDF format itself and the word "PDF" are not trademark-
  restricted for describing file-format compatibility; this is a
  different question from using Adobe's product branding.

### 4.1 "pdfce" name-collision check (2026-07-23)

Before treating "pdfce" as the final public-facing name (not just a
dev codename): a practical collision check was run, web-search-level
(not a formal trademark-registry search).

- **crates.io**: `pdfce` is unregistered — confirmed via a direct API
  query returning 404 "crate `pdfce` does not exist." Clear.
- **GitHub**: no `pdfce` user or organization exists — confirmed via
  direct query (404). A fuzzy name search turned up ~32 unrelated
  repos (`pdfcevir`, `PDFCertificateGenerator`, etc.), none with real
  prominence (single-digit stars). Clear.
- **Trademark**: no confirmed registered "PDFCE" mark found via
  general web search. **This was not a formal USPTO TMsearch-database
  query** (blocked/not attempted at that depth) — good enough to keep
  using the name now, but run an actual USPTO search before any formal
  trademark filing.
- **Confusion risk**: low. Doesn't phonetically or visually resemble
  Acrobat, Acrobat Pro, Acrobat Reader, or other known PDF tools
  (Foxit, Stirling-PDF, PDFCreator, PDFgear, pdfFiller). Reads as an
  initialism, not a merely-descriptive term like "PDFEdit" would be —
  plausibly more defensible if trademark protection is ever pursued,
  though that depends on what "CE" is understood to stand for.

**Bottom line: no blocking issue found.** Safe to keep "pdfce" as the
working and likely-final name; revisit only if a formal trademark
filing is ever pursued (do the USPTO search then, not now).

## 5. Test corpus sourcing (binding on pdfce-engineer)

- Fixture PDFs checked into `fixtures/` must be either: (a) synthetic,
  generated by pdfce's own tooling or a documented script, or (b)
  drawn from a corpus with clear redistribution rights (e.g. the PDF
  Association's public test suites, veraPDF's open corpus, or files
  the user personally authored and has rights to redistribute).
- **Never** check in a real-world PDF of unknown provenance (a
  downloaded invoice, a scanned document found online, an AI-generated
  "looks like a real business PDF" test file) without confirming its
  license/rights situation first. This mirrors the SWFormat project's
  "no client IP in any artifact" discipline, applied to PDFs instead
  of SOLIDWORKS files.
- If a bug report requires a specific real-world PDF to reproduce and
  its provenance is unclear, keep it in a local, non-committed
  scratch/debug location — describe the bug and the minimal structural
  cause in the SESSION_LOG / lesson instead of committing the file
  itself.

## 6. Open-source dependency licensing & attribution

pdfce leans on the existing Rust/OSS ecosystem rather than
reinventing everything (see `docs/PRIOR_ART.md` for the actual
survey). Every dependency brings its own license, and pdfce's own
license (§1, still undecided) determines what's even usable — this
section is the binding discipline for that intersection.

### 6.1 The permissive/copyleft split, and why it gates the §1 decision

- **Permissive** (MIT, Apache-2.0, BSD-2/3-Clause, Zlib): safe to
  depend on regardless of what pdfce's own license ends up being.
  Most of the Rust crate ecosystem defaults to MIT/Apache-2.0 dual.
- **Weak copyleft** (LGPL, MPL-2.0): usable as a dynamically-linked
  dependency in most cases without forcing pdfce's own license to
  match, but static linking (the Rust ecosystem's norm — everything
  compiles into one binary) can blur that line. **Flag any LGPL/MPL
  dependency to the user before adding it** rather than assuming
  static-linking is fine.
- **Strong copyleft** (GPL-2/3, AGPL-3): if pdfce **links** GPL/AGPL
  code into its own binary (not just "reads it for inspiration" —
  actual linking/embedding), pdfce's own distributed binary must also
  be GPL/AGPL-compatible. **This means: if pdfce ends up MIT/Apache-2.0
  licensed, GPL/AGPL dependencies are categorically off the table as
  real dependencies** — they can only ever be read-only architectural/
  algorithmic reference (independently reimplemented, not copied).
  Conversely, choosing AGPL-3.0 for pdfce itself would make certain
  otherwise-attractive GPL/AGPL prior art (e.g. MuPDF, Poppler,
  Ghostscript — see `docs/PRIOR_ART.md`) legally available as real
  dependencies. **This is a concrete, practical argument for resolving
  the §1 license decision early** — it isn't just an abstract
  preference, it changes which real engineering shortcuts are
  available.

### 6.2 Rule: no dependency added without a license check

Before `pdfce-engineer` adds ANY new crate to a `Cargo.toml` (not just
at Pass 0 — every time, for the life of the project):

1. Check the crate's license (its `Cargo.toml` `license` field, or its
   repo's `LICENSE` file if ambiguous).
2. Classify it per §6.1 above.
3. If permissive: proceed, log it in `docs/PRIOR_ART.md`'s adopted-
   dependencies table.
4. If weak or strong copyleft: **stop and ask the user** before adding
   it, even if pdfce's current license would technically allow it —
   this is a case where getting it wrong is expensive to unwind later
   (ripping out a load-bearing dependency after other code depends on
   its API is real rework), so it warrants a check-in every time, not
   just a one-time policy decision.
5. If a dependency is FFI to a non-Rust library (e.g. binding to a C
   library for JPEG2000/JBIG2 support), the same license check applies
   to that underlying library, AND it reopens the "single Rust binary,
   no heavy runtime" portability question from `ARCHITECTURE.md` §6 —
   flag both concerns together.

### 6.3 Attribution mechanism: generated, not hand-maintained

Hand-maintaining a NOTICE/THIRD_PARTY_LICENSES file is error-prone and
drifts from reality as dependencies change. pdfce uses **`cargo-about`**
(the standard Rust-ecosystem tool for this) to generate the attribution
file from the actual `Cargo.lock` dependency graph:

- Set up at Pass 0 (or as soon as the workspace has real dependencies):
  a `about.toml` config + a `cargo about generate` invocation that
  produces `THIRD_PARTY_LICENSES.md` (or `.html`) at the repo root.
- Regenerate it as part of the packaging step (§6), not just once —
  a stale attribution file shipped alongside a newer dependency set is
  a real (if usually low-stakes) compliance gap.
- This file **is** meant to ship with releases (unlike the private
  RAGs) — it's the actual legal notice a downstream user/redistributor
  needs. `pdfce-librarian` doesn't own it; `pdfce-engineer` regenerates
  it mechanically as part of the release/packaging checklist.

### 6.4 `docs/PRIOR_ART.md`

The living survey of candidate/adopted open-source dependencies and
reference projects, maintained by `pdfce-engineer` (dispatch
`pdfce-librarian` for the actual file edits, same discipline as
`ARCHITECTURE.md`'s decision log). Distinct from the generated
`THIRD_PARTY_LICENSES.md`: `PRIOR_ART.md` is the research/decision
record (why a crate was chosen or rejected, what the license
implications were); `THIRD_PARTY_LICENSES.md` is the mechanically
generated compliance artifact. Both matter; they serve different
readers (an engineer deciding what to depend on, vs. a downstream
user checking license compliance).

## 7. Decision log

- **2026-07-23** — Legal posture document created at project bootstrap.
  License: **undecided, open item**. No public commit/publish until
  decided. Spec-sourcing table established. Test-corpus rule established.
- **2026-07-23 (same-session amendment)** — Added §6, open-source
  dependency licensing & attribution discipline, per user request to
  survey existing OSS projects for prior art and ensure proper
  crediting. Established: permissive-vs-copyleft classification gates
  what's usable given pdfce's own (still undecided) license; no
  dependency added without a per-instance check, copyleft always
  flagged to the user; attribution via generated `cargo-about` output
  (`THIRD_PARTY_LICENSES.md`), not hand-maintained; research findings
  land in a new `docs/PRIOR_ART.md`.
- **2026-07-23 (same-session amendment 2)** — Fixed a section-numbering
  bug: this file jumped from §5 straight to a "§7" with no §6 —
  renumbered the dependency-licensing section to §6 (was mislabeled
  §7) and this decision log to §7 (was §8). Updated every cross-file
  reference to the old numbers. Also: **name-collision check on
  "pdfce" completed, came back clean** — no existing crates.io crate
  (confirmed 404), no existing GitHub user/org (confirmed 404), no
  confirmed trademark or well-known-product conflict via web search,
  low phonetic/visual confusion risk with Acrobat or other PDF tools.
  Not a formal USPTO trademark-database search (recommended before
  any actual trademark filing, not required to keep using the name
  for the repo/crate). See §4 Trademark posture.
