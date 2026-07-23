# pdfce — Session log

Append-only. One section per session date. Never overwrite or reorder
a prior entry; corrections get a dated amendment footer appended to
the affected entry. Maintained by `pdfce-librarian`.

## 2026-07-23 — project bootstrap

**Shipped:**
- Project scaffolded at `D:\Dev\pdfce\`. No application code yet —
  this session's deliverable was the agent roster + supporting docs
  that future engineering sessions read first.

**Decisions made this session:**
- Target parity product: **Adobe Acrobat Pro** (user's explicit pick,
  among Acrobat Pro / Foxit PDF Editor Pro / Nitro PDF Pro / other).
- Tech stack: **Rust** core, native GUI via **egui/eframe**
  (recommended default — egui vs iced was left open, egui chosen for
  its first-class WASM/eframe web target, which directly serves the
  user's stated "fork to a web app later" goal; confirm at Pass 0).
- PDF-standard reference RAG location: **`D:\Dev\Rag-Specialized\PDF_Spec\`**
  (user's pick among project-local `D:\Dev\pdfce\rag\`, a new
  `C:\pdf_spec_rag\` top-level dir, or the Rag-Specialized convention;
  chosen because it matches the existing `Funding_Programs` precedent
  for large, stable, cross-project reference corpora).
- Four project agents created (see `.claude/agents/`): `pdfce-engineer`
  (lead engineer, opus), `pdfce-librarian` (institutional memory,
  sonnet), `pdfce-spec-librarian` (PDF-standard RAG builder/maintainer,
  opus), `pdfce-ui-specialist` (egui/eframe UX review, sonnet).
- License: **left undecided** — see `LEGAL.md` §1. Must be resolved
  before any public commit or release.

**Findings + decisions:**
- ISO 32000-2 (PDF 2.0), ISO 19005 (PDF/A), and ISO 14289 (PDF/UA) are
  all ISO-paywalled; ISO 32000-1 (PDF 1.7), ETSI PAdES, ITU-T CCITT/
  JBIG2/JPEG/JPEG2000, Adobe's XMP spec, the ICC profile spec, and
  Microsoft's OpenType spec are all freely available and cover most of
  the same normative ground. Full sourcing table in `LEGAL.md` §2 —
  this is the sourcing strategy `pdfce-spec-librarian` follows.
- Round-trip / minimal-diff editing (analogous to the SWFormat
  project's tail-bytes/lazy-round-trip discipline) is a load-bearing
  invariant here too, because Acrobat's digital-signature model
  depends on incremental updates leaving prior bytes untouched. See
  `ARCHITECTURE.md` §5.
- GUI-core separation (`pdfce-core` / `pdfce-render` must have zero
  windowing dependencies) is the specific invariant that keeps the
  future web fork cheap. See `ARCHITECTURE.md` §3.

**Decisions made this session (continued — same-day additions):**
- **CLI capabilities added as a first-class requirement.** New
  `pdfce-cli` crate (subcommands: merge/split/rotate/extract, Bates
  stamp, PDF/A convert + validate, sign, render-page-to-PNG), depends
  on `pdfce-core`/`pdfce-render` only, same zero-GUI-deps discipline as
  `pdfce-gui`. Packaged alongside `pdfce-gui` in the same single
  output folder. See `ARCHITECTURE.md` §7, `ROADMAP.md` Pass 0 +
  "CLI batch operations" backlog entry.
- **Rust Style Guide + API Guidelines adopted as binding discipline.**
  `cargo fmt`/`cargo clippy -D warnings` clean is now a Pass-shipping
  requirement; public API design (especially `pdfce-core`'s) checked
  against a new condensed reference file,
  `D:\dev\rag\rust\rust-style-guide-and-api-guidelines.md`. See
  `ARCHITECTURE.md` §8.
- **Corrected the Rust/egui knowledge-base plan.** Originally scoped
  as a new `personal_rag/rust` subject; corrected to use the existing
  Cross-project Tool RAG instead — `D:\dev\rag\rust\` and
  `D:\dev\rag\egui\` (both created this session, and registered in
  `C:\Users\Ken\.claude\CLAUDE.md`'s Cross-project Tool RAGs list).
  Rationale: Rust and egui are ecosystem-wide tools like Gradle/Docker/
  Next.js (useful to any future Rust project), not a pdfce/PDF-domain-
  specific subject the way `personal_rag/solidworks` or the still-
  planned `personal_rag/pdf` are. All four agent files and this
  project's `CLAUDE.md` were updated to reflect the correction.

**Decisions made this session (continued — same-day addition 3):**
- **Added a second reference RAG + fifth agent.**
  `D:\Dev\Rag-Specialized\Acrobat_Features\` catalogs Adobe Acrobat
  Pro's feature set — capability, behavior, edge cases, limits —
  explicitly excluding GUI mechanics (menu paths, panels, dialogs,
  trade dress). Purpose: ground `ROADMAP.md` acceptance criteria in
  real Acrobat behavior when scoping a Backlog bucket into a Pass.
  New agent `pdfce-acrobat-librarian` (sonnet) owns it; dispatched by
  `pdfce-engineer`/`pdfce-librarian` during Backlog-to-Pass scoping.
  Category taxonomy mirrors `ROADMAP.md`'s Backlog buckets 1:1.
- **Established a cross-RAG format rule**: every RAG this project
  builds or writes to — `PDF_Spec`, `Acrobat_Features`,
  `D:/dev/rag/rust`, `D:/dev/rag/egui`, and the future
  `personal_rag/pdf` — is written for **LLM consumption only, not
  human reading**. Dense, schema-consistent, grep-first; no narrative
  scene-setting or prose padding. Retrofitted a short note into each
  existing RAG's `index.md` and into `pdfce-spec-librarian.md`'s
  voice/format section; the new `Acrobat_Features` RAG was built to
  this standard from file one as the flagship example.

**Still in flight:**
- No Cargo workspace exists yet. Pass 0 (workspace bootstrap,
  `ROADMAP.md`) is queued as the next real engineering session — now
  includes scaffolding `pdfce-cli` alongside `pdfce-gui`.
- No content exists yet in `D:\Dev\Rag-Specialized\PDF_Spec\` or
  `D:\Dev\Rag-Specialized\Acrobat_Features\` beyond their scaffolds
  (`index.md`, `_TEMPLATE.md`, `LEGAL_NOTE.md`). Both are built
  incrementally, driven by `ROADMAP.md`, by their respective librarian
  agents — not all at once.
- `D:\dev\rag\rust\` and `D:\dev\rag\egui\` currently hold only the
  Style Guide/API Guidelines reference file and scaffold `index.md`s —
  no empirical findings yet, since no Rust/egui code has been written.

**Decisions made this session (continued — same-day addition 4):**
- **Prior-art research completed and synthesized into
  `docs/PRIOR_ART.md`.** Three parallel web-verified research passes
  (core PDF crates; supporting codec/font/crypto/CLI/egui crates;
  existing full OSS PDF tools). Full findings in that file; headline
  items:
  - **`oxidize-pdf` (MIT, github.com/bzsanti/oxidizePdf)** may already
    cover most of `pdfce-core`'s target scope (incremental saves,
    RC4/AES encryption, PKCS#7 verify, pure-Rust JBIG2/CCITT/DCT/JPX,
    7,993 tests claimed) — single-maintainer, unaudited. **Flagged as
    an open, undecided question** (`ROADMAP.md` Pass 0): audit before
    committing to a from-scratch `pdfce-core`.
  - Pure-Rust answers now exist for all three "problem filters"
    (JBIG2/CCITT/JPX) via the `hayro-*` crate family (Apache-2.0/MIT)
    — closes what would otherwise have been the biggest portability
    liability (C-library FFI for these codecs).
  - Confirmed gap: no Rust crate does signature-safe incremental saves
    or PAdES signing anywhere in the ecosystem — pdfce-core will need
    to build this from `cms` + `x509-cert` + RustCrypto primitives.
    This is real differentiation, not just a todo.
  - `tiny-skia` selected for `pdfce-render`'s rasterizer (CPU-only,
    pure Rust, proven via `resvg`).
  - **Licensing landmine flagged**: MuPDF and Ghostscript (both
    Artifex) are AGPL-3.0-or-later, dual-licensed with a paid
    commercial option — linking either forces pdfce itself to AGPL or
    a purchased license. Never link without a deliberate, user-
    confirmed decision.
  - **Competitive landscape confirmed clear**: no existing OSS project
    (web or desktop) combines pdfce's full target feature breadth.
    Open PDF Studio (LGPL-3.0) and KillerPDF (GPL-3.0) are the closest
    native-desktop attempts; both have confirmed major gaps (no OCR/
    Bates/PDF-A/accessibility for the former; GPL + Windows-only + no
    redaction/PDF-A for the latter), and neither uses a native Rust
    engine. Validates the project's premise.
  - One unresolved fact worth tracking: Poppler's exact license
    (GPL-2/3 vs LGPL) couldn't be directly confirmed this session —
    matters for future linking-risk analysis if it ever comes up.
- **Established the dependency-licensing discipline** (`LEGAL.md` §6):
  permissive/weak-copyleft/strong-copyleft classification, no
  dependency added without a check, copyleft always flagged to the
  user, attribution generated via `cargo-about` into
  `THIRD_PARTY_LICENSES.md` rather than hand-maintained.

**Decisions made this session (continued — same-day addition 5, gap-check pass):**
- **Undo/redo architecture designed** (`ARCHITECTURE.md` §11) — the
  user explicitly flagged this as a gap needing resolution. Command
  log over the in-memory object graph, diffed against the base
  revision at save time (not accumulated through undo history) to
  compute the incremental-save dirty set. Redaction is undo-able like
  any edit up until save; not reversible after a save actually
  happens (matches real-world expectation, ties to the UI
  confirmation-dialog rule). Bound the undo stack; snapshot commands
  are an acceptable specialization for bulk structural edits, not a
  parallel system. Must be built into the first editing Pass, not
  retrofitted.
- **Adversarial-input hardening designed** (`ARCHITECTURE.md` §10) —
  resource-limit guards (output-size ceilings on every filter, depth/
  cycle guards on recursive structures, operation/time budgets on
  content-stream interpretation) plus a mandatory `cargo-fuzz` target
  before Pass 1 ships. Previously just an implicit justification for
  choosing Rust; now a concrete structural requirement.
- **Privacy posture made explicit** (`ARCHITECTURE.md` §1.1) — no
  network calls of any kind by default, no telemetry, no phone-home;
  any future opt-in feature must be disclosed and off by default.
- **Toolchain/lockfile policy set** (`ARCHITECTURE.md` §2.1a) — pin
  `rust-toolchain.toml` at Pass 0, set MSRV then, commit `Cargo.lock`
  (application workspace, not a library).
- **"pdfce" name-collision check completed, clean** — no crates.io
  crate, no GitHub user/org, no confirmed trademark/product conflict,
  low confusion risk. See `LEGAL.md` §4.1. Safe to keep as the
  eventual public name; a formal USPTO search is only needed before an
  actual trademark filing, not before continuing to use the name.
- **Git repository initialized**, `.gitignore` added (Rust-standard;
  `Cargo.lock` and `THIRD_PARTY_LICENSES.md` deliberately NOT
  ignored, per the policies above), initial commit made of the full
  bootstrap scaffold.
- **CI scaffolded** (`.github/workflows/ci.yml`) — fmt/clippy/test/
  GUI-core-separation-verification jobs, targeting the Pass-0
  workspace layout ahead of the workspace actually existing (expected
  to not run meaningfully until there's a remote + Pass 0 lands;
  nothing pushed anywhere yet so no visible failing run).
- **Test-fixture sourcing plan created** (`fixtures/README.md` +
  `fixtures/fetch-corpora.sh`) — veraPDF corpus and PDF Association
  PDF 2.0 examples confirmed live via direct GitHub API checks;
  Isartor test suite's current URL NOT confirmed (site blocked a bot
  fetch) — don't guess it, verify via browser/WebFetch before using.
- **Standard OSS repo files added**: `CONTRIBUTING.md` (references the
  undecided-license state explicitly, DCO-style inbound=outbound
  contribution licensing once a license exists), `CODE_OF_CONDUCT.md`
  (Contributor Covenant 2.1, attributed), `SECURITY.md` (private
  disclosure process; explicitly frames redaction-doesn't-actually-
  redact and adversarial-input DoS as critical-severity, not cosmetic).
- **Product-scope decisions flagged, deliberately deferred** (not
  silent oversights) in `ROADMAP.md` Backlog: i18n/l10n (cheap now,
  expensive to retrofit — flag before first real UI strings are
  written), cross-platform scope beyond "Windows first," and an
  update/release mechanism for the no-installer portable app.
- **Fixed two section-numbering bugs** discovered while cross-checking
  references: `ARCHITECTURE.md`'s Decision log had drifted from §7 to
  §10 across earlier same-day edits without every cross-reference
  being updated (fixed — now §12, all references corrected); `LEGAL.md`
  jumped from §5 straight to a mislabeled "§7" with no §6 (fixed —
  dependency-licensing section is now correctly §6, decision log §7,
  all cross-references corrected). Lesson: when appending numbered
  sections mid-session, grep for cross-references immediately, don't
  assume they were caught in the moment.

**For next session:**
- Confirm egui vs iced with the user before scaffolding the workspace.
- Decide the OSS license (`LEGAL.md` §1) before any public-facing
  work — also now relevant to whether AGPL/GPL prior art ever becomes
  usable as a real dependency.
- **Decide build-from-scratch vs. `oxidize-pdf`-foundation for
  `pdfce-core` before Pass 1** — the single highest-leverage open
  decision from this session's research.
- Kick off `pdfce-spec-librarian` for the object-model/xref/FlateDecode
  slice of the spec RAG once Pass 0/1 engineering actually starts.
- Scaffold `pdfce-cli` with `clap` at the same time as `pdfce-gui` in
  Pass 0 — don't let the CLI slip to "later."
- When Pass 1+ starts scoping real Acrobat-parity features (forms,
  redaction, signatures, etc.), dispatch `pdfce-acrobat-librarian` to
  catalog that bucket before finalizing acceptance criteria.
- Set up `cargo-about` + `about.toml` as soon as the workspace has its
  first real dependencies (likely `flate2`, `weezl`, `tiny-skia` at
  minimum for Pass 1).
- Build the command-log undo/redo mechanism into whichever Pass first
  introduces editing — don't write edit code assuming direct mutation
  first and bolt undo on after.
- Resolve the Isartor test-suite URL via a real browser/WebFetch check
  before Pass 1 needs PDF/A conformance fixtures.
