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

## 2026-07-23 — Pass 0: workspace bootstrap

**Shipped:**
- **Pass 0 — Workspace bootstrap.** Cargo workspace with four crates
  (`ARCHITECTURE.md` §3/§7), each at its Pass 0 acceptance bar:
  - `pdfce-core` (dep `thiserror` 2.0.19) — `%PDF-` header probe:
    `probe_header(&[u8])`, `probe_file(&Path)` → `PdfVersion{major,minor}`;
    `PdfError` (`thiserror`, `#[non_exhaustive]`, C-GOOD-ERR). Scans
    first 1024 bytes (`HEADER_SCAN_WINDOW`) tolerating leading
    BOM/whitespace, parses `M.N`. Cites ISO 32000-2:2020 §7.5.2. Zero
    GUI deps.
  - `pdfce-render` — Pass 0 stub: re-exports core's
    `PdfVersion`/`PdfError` + `PLANNED_RASTERIZER` const; `tiny-skia`
    deferred to Pass 1. Zero GUI deps.
  - `pdfce-gui` — `eframe` 0.35.0 (GLOW backend) + `rfd` 0.17.2. Native
    window, blank canvas, working Open… dialog that runs the probe and
    shows version or a clear error.
  - `pdfce-cli` — `clap` 4.6.4. `inspect <file>` implemented; 9 stub
    subcommands print "not implemented, later Pass". Exit-code contract:
    0 ok / 1 generic / 3 IO / 4 not-a-PDF / 64 unimplemented (2 reserved
    by clap). Zero GUI deps.
- 13 tests passing (8 core unit + 2 core doctests + 3 cli).
- `cargo tree` GUI-core-separation invariant verified explicitly on
  `pdfce-core`/`pdfce-render`/`pdfce-cli` (zero
  egui/eframe/winit/wgpu/glow/glutin/rfd); `pdfce-gui` has glow, not
  wgpu. `fmt --check` + `clippy -D warnings` clean workspace-wide.
- Packaging smoke test PASSED: static-CRT release build, both binaries
  copied to a fresh temp folder and run with no install — `pdfce-cli
  inspect` → "PDF 1.7" exit 0; `pdfce-gui` opened a window (glow/OpenGL
  init, valid handle, no crash). `dumpbin /dependents` confirms no VC++
  redistributable dependency. Sizes: cli 0.83 MB, gui 7.27 MB.
- `cargo-about` set up; `THIRD_PARTY_LICENSES.md` generated (158 crates,
  all permissive + 2 bundled-font licenses OFL-1.1/Ubuntu-font-1.0 via
  `epaint_default_fonts`; ZERO copyleft).

**Decisions made this session:** (full text in `ARCHITECTURE.md` §12,
dated 2026-07-23 Pass 0)
- **egui/eframe CONFIRMED over iced** (user decision) — closes the
  `ARCHITECTURE.md` §2.1 open question.
- **`oxidize-pdf` decision DEFERRED** — user chose a thin,
  header-probe-only `pdfce-core` for Pass 0; the
  build-from-scratch-vs-adopt audit stays the gate before Pass 1
  (unchanged).
- **Rendering backend = GLOW, not wgpu** (eframe's default). Reason:
  the wgpu 29.0.4 stack FAILS TO COMPILE on Windows MSVC — `wgpu-hal`
  29.0.4 uses `windows` crate 0.61.2 while `gpu-allocator` 0.28.0 uses
  `windows` 0.62.2, and their D3D12 `ID3D12Heap` types are mutually
  incompatible (`windows_core::imp::CanInto` mismatch in
  `CreatePlacedResource`). `ARCHITECTURE.md` §2 already specified "wgpu,
  falls back to glow/OpenGL if needed" — this exercises that documented
  fallback; glow is also lighter for single-folder packaging. A routine
  engineering call within the pre-authorized §2 design, not a reversal.
  Revisit wgpu when the upstream `windows`-crate versions realign.
- **Toolchain pinned 1.97.1; edition 2024; resolver 3; MSRV = 1.92**
  (driven up from the edition-2024 floor of 1.85 by `eframe` 0.35, which
  needs rustc 1.92 — exactly the "deps force MSRV higher" re-check
  §2.1a anticipated); `Cargo.lock` committed.
- **Static CRT on Windows** (`.cargo/config.toml`,
  `target-feature=+crt-static`) so binaries need no VC++ redistributable
  — serves the §6 single-folder/no-system-runtime requirement, verified
  via `dumpbin`.
- **`.gitattributes` hardened** — `*.pdf`/`*.bin`/`*.png`/etc. marked
  `binary` so EOL normalization can't corrupt byte-exact binary PDF
  fixtures.

**Findings + decisions:**
- Rust **1.97.1 was installed on the machine this session** — it was
  absent before Pass 0.
- The wgpu-29-on-Windows build break and the glow-backend feature-flag
  recipe are captured in `D:\dev\rag\egui\` (four findings); the
  `cargo-about --features cli`, static-CRT, MSRV-resolver, and
  `.gitattributes`-binary findings in `D:\dev\rag\rust\` (four
  findings). egui/eframe version line in `D:\dev\rag\egui\index.md`
  updated to 0.35.0; the rust `index.md` toolchain line to edition
  2024 / 1.97.1 / MSRV 1.92 / resolver 3.

**Still in flight:**
- **`oxidize-pdf` audit** — the outstanding gate before Pass 1
  (build-from-scratch vs adopt/vendor). Highest-leverage open decision.
- Pass 1 proper: `tiny-skia` rasterizer, FlateDecode filter, and the
  tokenizer/COS object model + xref parsing (none started).

**For next session:**
- Run the `oxidize-pdf` audit BEFORE any from-scratch `pdfce-core`
  parser work (round-trip fidelity on adversarial input, signature-safe
  incremental-save semantics, PAdES signing vs PKCS#7-verify-only).
- **Kick off `pdfce-spec-librarian`** for the object-model / xref /
  FlateDecode slice of the spec RAG as Pass 1 starts — the first
  spec-governed behavior lands there.
- Wire the `cargo-fuzz` tokenizer/parser target and FlateDecode
  output-size ceiling into Pass 1's acceptance criteria
  (`ARCHITECTURE.md` §10) as that Pass is scoped.

## 2026-07-30 — scope expansion + decision-protocol setup

*(Session in progress — this entry may get a same-day continuation as
the session develops.)*

**Shipped:**
- Nothing code-wise yet this session; this entry records the session
  kickoff, a scope expansion, and a new operator process rule.

**Decisions made this session:**
- **Scope expansion: Inkscape-parity vector editing.** Operator's
  words: "For this project I want it to have all of the capabilities
  to edit pdfs that inkscape and acrobat pro does." Acrobat Pro parity
  was already the target; the NEW part is Inkscape-level vector
  editing of PDF page content. Filed as a new `ROADMAP.md` Backlog
  bucket **"Vector graphics editing (Inkscape-parity)"** (adjacent to
  and cross-referenced from "Text & object editing"): node/Bézier path
  editing, boolean path ops, stroke/fill/gradient/pattern editing,
  precise transforms, alignment/distribution, z-order, group/ungroup,
  text-to-path, OCG layers. Binding notes recorded in the bucket:
  Inkscape (GPL-2.0-or-later) is a behavioral reference ONLY, never a
  dependency or code source; the scope raises the bar on
  `pdfce-core`'s content-stream model (full round-trip decomposition
  of graphics operators into editable objects, minimal-diff
  re-emission per `ARCHITECTURE.md` §5); Pass-scoping will need an
  Inkscape capability catalog (capability/behavior only, never GUI
  mechanics — same discipline as the Acrobat Features RAG).
- **KenAgent decision protocol established (operator process rule,
  2026-07-30).** Non-trivial technical decisions are routed through
  the "KenAgent" autonomous-builder agent, which returns a decision
  with reasoning in JSON (used to implement) + Markdown (archived to
  `docs/decisions/NNN-slug.md` for project history). Legal/license
  decisions remain Ken's directly, unchanged. Recorded in
  `ROADMAP.md` Standing rules.

**Still in flight:**
- **The `oxidize-pdf` adopt-vs-build audit — the outstanding Pass-1
  gate — was launched this session.** Its decision will be routed
  through KenAgent per the new protocol and archived in
  `docs/decisions/`. Outcome not yet recorded at time of this entry.

**For next session:**
- If the audit concludes this session, expect a same-day continuation
  below with the decision and its `docs/decisions/` archive path;
  otherwise pick up the audit's state from `docs/decisions/` and
  `ROADMAP.md` Pass 1's gate note.

**Same-day continuation — audit completed, decision accepted, gate
closed:**

- **`oxidize-pdf` adopt-vs-build audit COMPLETED** (target:
  `bzsanti/oxidizePdf` HEAD `5f3e8b3`, v4.2.1, MIT). Headline
  findings: **round-trip fails by design** — two disconnected object
  models (read-only parser vs from-scratch generator) with no bridge
  and no `Document::load()`; **general incremental save is
  absent/destructive** (the one "incremental" API drops
  `/Outlines`/`/AcroForm`/`/Names`); **no content-stream serializer**
  (operator model is read-only, lossy); **non-suppressible build-hash
  fingerprint in `/Info`** on every written PDF; **silent filter
  fallbacks** (raw undecoded bytes returned on zlib/predictor
  failure); **bus factor 1 with four major version bumps in year
  one**; **real signing is PRO-gated** (MIT tree ships a placeholder
  stub). Also corrected two PRIOR_ART claims: the JPX decoder claim is
  FALSE ("not yet implemented") and the DCT claim MISLEADING
  (validates/trims markers, delegates pixel decode to the optional
  `image` crate); JBIG2/CCITT are real.
- **Decision 001 ACCEPTED via the KenAgent protocol (first use):
  option (c) reference-only — build `pdfce-core` from scratch.** Zero
  literal ports planned; maintained permissive crates preferred over
  vendoring (`hayro-jbig2`, `hayro-ccitt`, `subsetter`/`allsorts`,
  RustCrypto); oxidize-pdf serves only as an out-of-tree differential
  oracle (`tools/difftest/`, workspace-excluded, pinned,
  advisory-never-authoritative, fixtures never from its repo).
  Record archived: `docs/decisions/001-oxidize-pdf-adopt-vs-build.md`.
- **Pass 1 GATE CLOSED** (`ROADMAP.md` updated): Pass 1 gains the
  ByteSpan-provenanced object model + lossless content-stream token
  model as in-Pass scope, and stands up `tools/difftest/` BEFORE the
  parser is written. `ARCHITECTURE.md` §12 records the six Pass-1
  architecture obligations (ByteSpan provenance; lossless token model;
  ONE object model; fail-clean filter contract; unwrap-deny lints in
  `pdfce-core`; no output fingerprint / `/Info` untouched on
  incremental save).
- **`pdfce-spec-librarian` dispatched for the Pass 1 spec slice** —
  ISO 32000-1 §7.5.4–§7.5.8 + hybrid `/XRefStm` (the byte-layout
  contract the ByteSpan design must honor). **In flight** at the time
  of this entry.
- **PRIOR_ART corrections filed**: OPEN QUESTION rewritten as RESOLVED
  (2026-07-30); oxidize-pdf verdict now "reference-only / differential
  oracle (out-of-tree only, never a shipping dep)"; new `hayro-write`
  row (parse→rewrite via `pdf-writer` now exists — full
  re-serialization only, NOT byte-preserving incremental save;
  lopdf #305 still open, so the ecosystem gap pdfce fills stands).
- **`C:\personal_rag\pdf\` extended** (subject already existed —
  seeded 2026-07-29 from the Open PDF Studio project): two new
  lessons — the silent-corruption filter-fallback antipattern
  (quirk/HIGH, cites oxidize-pdf `filters.rs:1600+`) and the
  xref-recovery conditional-harvest methodology (decision 001 §6.3
  gate). Subject + master indexes updated.

**Operator flag (from decision 001 §10 — do not lose this):** pdfce is
now deliberately committed to being the **first project in the Rust
ecosystem to implement signature-safe, byte-preserving incremental
save**. Nobody upstream has it (including `hayro-write`). This is
accepted solo-engineering cost — a deliberate differentiation bet, not
an assumption that an upstream crate will eventually cover it. Revisit
triggers are listed in decision 001 §9.

**Still in flight (continuation):**
- `pdfce-spec-librarian` Pass 1 spec-slice dispatch (see above).
- Pass 1 engineering proper: not started; next session begins with
  `tools/difftest/` per the new Pass 1 scope order.

**Same-day continuation 2 — decision 002 accepted; Pass 1 parse stack
landed:**

- **Decision 002 (i18n/l10n architecture) ACCEPTED via the KenAgent
  protocol (second use).** Record archived:
  `docs/decisions/002-i18n-timing.md`. Outcome: centralized
  zero-dependency function-based string catalog
  (`crates/pdfce-gui/src/ui_text.rs`, `pub fn` entries, English-only);
  `pdfce-cli` English-only PERMANENTLY by design with a binding
  locale-invariant-stdout contract (clap's own text is untranslatable,
  clap-rs/clap#380); `pdfce-core` errors never localized but must
  carry structured data (R4 — binds decision 001's new Pass-1 error
  variants); eight standing rules R1–R8 added to `ROADMAP.md`; new
  `ui-strings` CI grep job; `gettext-rs` pre-disqualified (LGPL static
  link on Windows). The old "Internationalization/localization"
  deferred bullet is struck as RESOLVED; a new Backlog entry files the
  independent live tofu bug ("UI font coverage for non-Latin file
  paths and document metadata" — epaint bundles no CJK fonts,
  emilk/egui#3060/#5233). `ARCHITECTURE.md` §12 entry added; egui
  text-stack finding escalated to
  `D:\dev\rag\egui\epaint_0.35_text_stack_i18n_limits.md` and the clap
  ceiling to `D:\dev\rag\rust\clap_4_no_i18n_own_text_hardcoded.md`.
- **Harness note (KenAgent protocol):** the first dispatch attempts
  for decision 002 hit a stale-worktree `EEXIST` harness bug; fixed by
  pruning `.claude/worktrees` before re-dispatch.
- **Engineering progress this session — full Pass 1 parse stack landed
  in `pdfce-core`:** span / lexer / object / parser / xref / document
  modules, plus the filter layer with FlateDecode and PNG/TIFF
  predictors. **113 unit tests + 4 doctests green; clippy/fmt clean.**
  `flate2` added with the pure-Rust backend per decision 001 §6.2
  (never the C `zlib`/`zlib-ng` backend). GUI-core invariant
  re-verified via `cargo tree`.
- **`tools/difftest/` oracle upgraded** from header-probe comparison to
  **full-`Document` comparison**, and it AGREEs with `oxidize-pdf` on
  the synthetic fixtures.
- **In progress at time of this entry (decision 002 engineering
  items):** `ui_text.rs` + the `ui-strings` CI job + the `pdfce-cli`
  module-doc paragraph (R5, English-only-by-design +
  locale-invariant-stdout beside the exit-code contract).

**Still in flight (continuation 2):**
- Decision 002 §10 engineering items 1–5 (ui_text.rs / CI job / CLI
  doc paragraph) — in progress, see above.
- Remaining Pass 1 scope: `pdfce-render` content-stream interpreter +
  `tiny-skia` rasterization, GUI viewer chrome, cargo-fuzz target,
  FlateDecode resource-limit ceiling, fixture-corpus pull-down.

**Same-day continuation 3 — Pass 1 parse surface COMPLETE; decision
002 engineering done; decision 003 dispatched (pre-compaction
capture):**

- **`pdfce-core`'s Pass 1 parse surface is now COMPLETE** — two more
  modules landed on top of continuation 2's parse stack:
  - **`page_tree.rs`** — the ISO 32000 §7.7.3 page-tree walk:
    top-down inheritance of the exactly-four inheritable attributes
    (`/Resources`, `/MediaBox`, `/CropBox`, `/Rotate`); the normative
    empty-dict-vs-absent `/Resources` distinction preserved; §7.9.5
    rectangle corner normalization; negative `/Rotate` normalized via
    positive modulo; `/Count` never trusted (pages counted by actual
    walk); `/Kids`-cycle detection plus hard guards
    (`MAX_TREE_DEPTH` 64, `MAX_PAGES` 1M); structural fallback when
    `/Type` is absent; `/Contents` accepted in all three spec shapes
    (absent = empty page / single stream / array of streams).
  - **`content.rs`** — decision 001 §6.1.2's lossless
    span-provenanced content-stream token model:
    `ContentToken{kind,span}` where spans index the **decoded**
    buffer; a semantic `operations()` view exists only as a pure
    projection over the tokens (never a second stored
    representation); reuses the §7.2 lexer — no second lexer;
    §8.9.7 inline images captured as single `BI..EI` tokens with an
    EI-detection strategy ladder (unfiltered = computed length from
    the image dict; `AHx`/`A85` = the filter's own EOD marker; other
    filters = whitespace-delimited-`EI` scan, documented in-code as a
    non-spec heuristic); Table 93/94 abbreviation normalization
    including the `I` = `Interpolate`-(key)-vs-`Indexed`-(value)
    disambiguation; `/Contents`-array concatenation with an LF
    separator per §7.7.3.3; `true`/`false`/`null` classified as
    operands; the reference syntax `1 0 R` deliberately left as an
    unknown operator so the interpreter rejects it per §7.8.2.
  - Totals: **137 unit tests + 4 doctests green**;
    `clippy -D warnings` + `fmt --check` clean workspace-wide;
    GUI-core invariant re-verified.
- **Decision 002's engineering actions are COMPLETE** (items that were
  "in progress" at continuation 2): `crates/pdfce-gui/src/ui_text.rs`
  catalog created with all 7 GUI strings moved into it
  (functions-not-consts per decision 002 §5.2); the `ui-strings` CI
  job added to `.github/workflows/ci.yml` and validated locally (zero
  false positives); `pdfce-cli`'s module docs gained the
  English-only-by-design + locale-invariant-stdout (R5) paragraph.
- **Decision 003 dispatched, in flight:** distribution posture —
  cross-platform scope beyond Windows-first, plus the update/release
  mechanism for the no-installer portable app (both were deliberately
  deferred flags from the 2026-07-23 bootstrap session). Routed
  through KenAgent per the standing protocol; the record will land at
  `docs/decisions/003-distribution-posture.md` when it returns. Not
  yet concluded at time of this capture.
- **Harness lesson (hit twice today):** stale `.claude/worktrees`
  state blocks agent dispatch with `EEXIST`; the fix is pruning all
  three residues — the `.claude/worktrees/` dirs, the repo's
  `.git/worktrees/` metadata, and leftover `worktree-agent-*`
  branches. Written up as a lesson in `C:\personal_rag\claude_code\`.

**Still in flight (continuation 3):**
- **Decision 003** (distribution posture) — KenAgent dispatch
  outstanding, see above.
- **`pdfce-render` content-stream interpreter + `tiny-skia`
  rasterizer** — next engineering item. The render-gap spec slice it
  needs landed today (§8.2 graphics-state machine, §8.4/§8.5
  graphics + path ops, §9.x text/fonts). **Spec-librarian flag, do
  not lose:** the standard-14 AFM widths (Adobe TN #5004) and the
  Adobe Glyph List are still-unsourced items the text pipeline will
  need — dispatch `pdfce-spec-librarian` for them before the text
  rendering path is written.
- GUI page canvas with pan/zoom + thumbnails; `pdfce-cli render-page`.
- `cargo-fuzz` tokenizer/parser target — still required before Pass 1
  ships (`ARCHITECTURE.md` §10.2).
- Corpus fixtures via `fixtures/fetch-corpora.sh`; packaging smoke
  test.
- **Everything remains UNCOMMITTED in git** — the operator has not
  yet said to commit.

**Same-day continuation 4 — decision 003 accepted; pdfce-render came
alive; font sourcing complete:**

- **Decision 003 (distribution posture) ACCEPTED via the KenAgent
  protocol (third use) and archived:**
  `docs/decisions/003-distribution-posture.md`. Outcome: **v1 ships
  Windows x64 only, as a deliberate decision**; the code stays
  platform-clean, enforced by cross-target `cargo check` CI (macOS +
  wasm32 on the ubuntu runner — the wasm32 check is a first positive
  web-fork invariant guard) instead of new runners; **manual
  folder-replace is the only update mechanism permanently** (pdfce
  never self-updates, R13), discovery delegated to Scoop-then-WinGet
  manifests (gated on `LEGAL.md` §1); **no network client crate ever**
  without a new decision record (R12, fail-closed `no-network` CI
  job). Eight standing rules **R9–R16** added to `ROADMAP.md`; both
  remaining "deliberately deferred" bullets struck as RESOLVED — **the
  deferred list is now EMPTY** (every bootstrap-flagged product-scope
  question answered within one week). New Backlog entry "Release &
  distribution channel" (Scoop/WinGet manifests, per-release SHA-256,
  README §6.3 copy verbatim — BLOCKED ON `LEGAL.md` §1).
  `ARCHITECTURE.md` §12 entry added, including the
  `webbrowser`-unconditional-in-eframe finding (§1.1 precision
  correction pending engineering) and the two latent CI defects
  (wrong-target `cargo tree` invariant check; toolchain-action
  mismatch). Findings escalated: two to `D:\dev\rag\egui\`
  (webbrowser hardcoded via egui-winit `links`; cross-target check +
  dlopen/musl), one to `D:\dev\rag\rust\` (cross-target `cargo check`
  portability gate + multi-target cargo-about recipe).
- **`pdfce-render` CAME ALIVE this continuation** — the crate went
  from Pass 0 stub to a working rasterizer: `gstate` / `interpret` /
  `lib` modules landed; path rendering, clipping, transforms, and
  device colours all working. **10 pixel-level tests green first-run**
  (cm premultiplication, even-odd vs nonzero fill, deferred W clip,
  `/Rotate` 90 geometry, q/Q restore, stroke). `clippy -D warnings` +
  `fmt --check` clean. Dependencies added to `pdfce-render`:
  `tiny-skia` 0.11 + `thiserror` (licenses checked per `LEGAL.md` §6).
- **Font-sourcing session COMPLETE** (spec RAG `fonts\` dir, 8 files,
  by `pdfce-spec-librarian` — closes continuation 3's "do not lose"
  flag): licensing verdicts **APAFML** for the Core 14 AFM data
  (permissive, embeddable; Adobe copyright notice must be retained in
  the generated file header) and **BSD-3-Clause** for the Adobe Glyph
  List (NOT Apache-2.0); all 14 fonts' width tables + descriptor
  metrics; AGL mapping. **Text rendering is now unblocked.** Both are
  non-Cargo data sources invisible to `cargo-about` →
  `THIRD_PARTY_LICENSES.md` needs a hand-maintained "embedded data"
  section, filed as a Pass 1 obligation in `ROADMAP.md` (documented
  exception to `LEGAL.md` §6.3). The std-14 NAME-ALIASING empirical
  lesson (/Helv, /Arial, /ArialMT, …) written to
  `C:\personal_rag\pdf\`.

**Still in flight (continuation 4) — remaining for Pass 1:**
- Text-rendering slice in `pdfce-render` (std-14 metrics now sourced).
- GUI viewer canvas (pan/zoom, thumbnails); `pdfce-cli render-page`.
- `cargo-fuzz` tokenizer/parser target (`ARCHITECTURE.md` §10.2).
- Fixture corpus pull-down.
- Ship checklist + launch (packaging smoke test, `cargo tree`
  invariants, decision 003 §10 engineering items 1–7 on the next
  CI/packaging touch).

**Same-day continuation 5 — /loop autonomous mode; decision 004
accepted; decision 003 engineering completed:**

- **Operator engaged `/loop` autonomous mode** — the session now
  self-paces through the Pass 1 queue.
- **Decision 004 (Pass 1 text-rendering font strategy) ACCEPTED via
  the KenAgent protocol (fourth use) and archived:**
  `docs/decisions/004-text-rendering-fonts.md`. Headline: **`skrifa`
  is zero-cost** (pinned 0.42 to epaint's resolved version — zero new
  lock packages; its `raw::ps` module closes the PRIOR_ART "Type1
  weakest link" risk outright); **Foxit shapes** (base-14 bundled,
  BSD-3-Clause via pdfium, 264,741 bytes, provenance-verified);
  **URW AGPL trap caught** (Nimbus is
  `AGPL-3.0-only WITH PS-or-PDF-font-exception-20170817` — document
  embedding only, not app bundling; rejected on the merits, never
  escalated because unneeded); **Pass 1 scope grows to `Identity-H`**
  composite fonts (`CIDFontType2` + `CIDFontType0`), Type 3, and the
  full simple-font chain, per the record's documented departure from
  the spec RAG ladder (§4.3 — steps 4–6 collapse to cheap once the
  parser question is settled; steps 1–3 alone would render no text in
  most modern subsetted PDFs). Six standing rules **R17–R22** added to
  `ROADMAP.md`; `ARCHITECTURE.md` §12 entry added. Findings escalated:
  one to `C:\personal_rag\pdf\` (read-fonts Type 1 parser coverage +
  the `%!PS-AdobeFont` header trap), one to `D:\dev\rag\rust\`
  (`skrifa::raw` re-export = bare Type1/CFF from one dependency +
  the version-pin-to-avoid-duplicates pattern).
- **Decision 003 engineering completed earlier this continuation:**
  cross-check + `no-network` CI jobs added and locally validated;
  `gui-core-separation` now checks the shipped Windows target
  (`cargo tree --target x86_64-pc-windows-msvc`); toolchain action
  pinned 1.97.1; `ARCHITECTURE.md` §1.1 `webbrowser` precision clause
  + §6 platform-scope/R15 partition amendments landed.

**Still in flight (continuation 5) — next:**
- **Font implementation slice** per decision 004 §10 items 1–8:
  `skrifa = "0.42"` dep + `cargo tree --duplicates` guard,
  `tools/extract-base14/` (pdfium CFF arrays → assets + PROVENANCE),
  `src/font/{mod,program,bundled,select}.rs` + the
  `FontEnvironment`/`RenderOptions` seam + R20 diagnostics, text
  interpreter ops (`BT`…`ET`, `Tj`/`TJ`/`'`/`"`).

**Same-day continuation 6 — Pass 1 SHIPPED:**

- **Pass 1 SHIPPED 2026-07-30** — the entire Pass, spec RAG to
  launched GUI, in one day, same day as decisions 001–004, completed
  in the operator-initiated `/loop` autonomous session. Moved to
  `ROADMAP.md` Shipped (full record there). The numbers:
  - `pdfce-core` complete read stack (span/lexer/object/parser/xref/
    document/page_tree/content/filters/fontdata), 197+ unit tests;
    ByteSpan provenance first-class; ONE object model; fail-clean
    filters with bomb guards; lossless content token model; std-14
    metrics/Annex-D/AGL tables generated from sourced data.
  - `pdfce-render`: full path rendering (all Table 59/60/61 ops,
    deferred-`W` clip, §8.3 CTM math), device colours,
    dash/caps/joins, `gs` subset, `BX`/`EX`, AND full text rendering —
    simple fonts through the complete §9.6.6 encoding chain (all four
    FontFile flavors via `skrifa`), `Identity-H` composites
    (CIDFontType0+2), §9.4.4 Trm/advance math, std-14 substitution via
    14 bundled Foxit faces (BSD-3-Clause, provenance-verified);
    R17–R22 enforced.
  - `pdfce-gui` read-only viewer (canvas pan/zoom with debounce,
    thumbnail rail, page nav with shortcuts, three-way failure
    distinction incl. honest "unsupported ≠ broken", R20 diagnostics
    status bar, ui-specialist-reviewed, HiDPI-correct); `pdfce-cli
    render-page` with a documented machine-parseable stdout contract.
  - Fuzzing: 3 cargo-fuzz targets, ~4M ASan-instrumented execs, ZERO
    crashes — §10.2 requirement met.
  - `THIRD_PARTY_LICENSES.md` regenerated (164 crates, zero copyleft)
    + manual embedded-data epilogue (APAFML/AGL/Foxit); the template's
    empty-license-text bug found + fixed along the way.
  - New fixture `fixtures/synthetic/hello.pdf` (loadable,
    text+graphics). 245 workspace tests green; fmt/clippy clean; ALL
    CI invariant guards verified locally (GUI-separation native +
    shipped-target, duplicates, ui-strings, no-network, wasm32
    cross-check).
- **Packaging smoke test PASSED** — fresh temp folder, no install
  step: `pdfce-cli render-page` produced correct output; `pdfce-gui`
  launched and rendered. Per the launch-on-completion rule, the GUI
  was launched on the operator's desktop at Pass completion.
- **The `/loop` fan-out worked:** the fontdata agent and the
  text-interpreter agent composed cleanly against a pre-agreed API
  contract; the fuzz agent and the shell agent ran in parallel.
- **Honest remainders filed as Pass 1.1 — corpus validation &
  hardening** (`ROADMAP.md` Next up): veraPDF/PDF-Association corpus
  pull + pixel-parity measurement (the original acceptance bar is NOT
  yet demonstrated — synthetic fixtures only); the
  `/Resources`-missing tolerance question (strict-refusal today;
  `fixtures/synthetic/minimal.pdf` itself unloadable for this reason);
  xref/object streams + hybrid files (biggest real-world gap,
  currently clean-refused); Type 3 rendering; `Tr` 4–7 clipping;
  GUI file argument (open-with); R20 panel revisit when editing lands.
- **RAG escalations this continuation:**
  - `D:\dev\rag\egui\egui_0.35_zoom_with_keyboard_vs_app_zoom_chords.md`
    — eframe/egui 0.35 `zoom_with_keyboard` defaults ON and consumes
    Ctrl+Plus/Minus/0 for UI scaling (set `false` when the app owns
    those chords), plus the wheel-event routing fact (zoom-modifier
    wheel events become `zoom_delta` and contribute nothing to
    `smooth_scroll_delta`, so pan and zoom can't fight).
  - `C:\personal_rag\pdf\lesson_20260730_resources_required_but_omitted_open_question.md`
    — the `/Resources` Required-inheritable vs real-world-omission
    OPEN question (corpus data needed; Pass 1.1 owns resolution).
  - `D:\dev\rag\rust\` — the fuzz and cargo-about lessons
    (`cargo_fuzz_windows_msvc_asan_dll_path.md`,
    `cargo_about_template_text_var_and_epilogue.md`) were already
    written and indexed by the fuzz agent; verified present, no gaps
    to fill.

**Still in flight (continuation 6):**
- Pass 1.1 (see `ROADMAP.md` Next up) — not started.
- **Everything remains UNCOMMITTED in git** — the operator has not
  yet said to commit.

**For next session:**
- Start Pass 1.1 with the corpus pull-down
  (`fixtures/fetch-corpora.sh`), then the `/Resources` frequency
  measurement — it gates both the tolerance decision and the
  minimal.pdf fixture question.

**Same-day continuation 7 — Pass 1.1 corpus measurement; data-driven
reprioritization; MAX_TOKEN_LEN bug fixed:**

- **Corpus run executed** via `tools/corpus-report` (new
  workspace-excluded measurement tool; corpora fetched to gitignored
  `fixtures/external/`): **2,907 veraPDF files + 7 PDF-Association
  files = 2,914 total. ZERO panics, ZERO timeouts.** 82.4% achieved
  full load + render of page 1.
- **Failure profile is a single spike, not a spread:**
  - `RefusedXrefStream`: **489 files (16.8% of corpus) = 97.8% of ALL
    failures**, concentrated in the PDF/UA and PDF/A-2/4 subcorpora
    (modern conformance files use cross-reference streams).
  - `RefusedHybrid`: exactly 1 file.
  - `MissingResources`: **ZERO in both corpora.** Conformance corpora
    are hand-built to spec, so this does NOT close the real-world
    `/Resources` tolerance question — but it does establish that
    strict mode is corpus-cost-free today.
  - All 11 `LoadError`s and 10 of 11 `RenderError`s are deliberate
    `*-fail-*` conformance files — the strict parser rejecting exactly
    what validators must reject. Correct behavior, not bugs.
- **Aggregate render honesty counters** (fidelity, distinct from
  parse rate): 3,387 deferred ops (XObjects/shading — the biggest
  fidelity gap), 732 substituted glyphs, 303 unsupported fonts, 7
  notdefs, 3 unknown ops. `LZWDecode` observed in actual use (2
  files) — not a dead filter.
- **One real bug found and FIXED same session:** the lexer's
  `MAX_TOKEN_LEN` 8 KiB guard rejected veraPDF `6-1-12-t02-pass-k.pdf`
  — a VALID file (PDF/A §6.1.12: readers must not impose Annex C
  implementation limits). Raised to 1 MiB with a corpus-cited doc
  comment; file verified rendering (exit 0). This was the ONLY
  pass-classified corpus file pdfce mishandled.
- **Decision 001 §6.3 gate CLEARED by measurement:** landing xref
  streams + object streams projects to ~99.6% parse rate — far above
  the <~95% threshold that would have triggered the oxidize-pdf
  xref-recovery harvest. **No harvest needed.**
- **Pass 1.1 reprioritized on the data** (recorded in `ROADMAP.md`):
  1. xref streams + object streams (489 files — everything else is
  noise by comparison); 2. MAX_TOKEN_LEN — already DONE this
  continuation; 3. `/Resources` tolerance DEPRIORITIZED (zero corpus
  pressure; needs organic real-world non-conformance files); 4.
  Type 3 / `Tr` 4–7 low (near-zero corpus presence). The 3,387
  deferred XObject/shading ops are the biggest FIDELITY item — a
  future rendering Pass, not a Pass 1.1 parse-rate item.
- **RAG escalations this continuation:**
  - `C:\personal_rag\pdf\lesson_20260730_resources_required_but_omitted_open_question.md`
    — amended with the corpus datum (zero omissions in 2,914
    conformance files; question stays OPEN pending real-world files).
  - `C:\personal_rag\pdf\lesson_20260730_filter_names_case_sensitive.md`
    — NEW: filter names are case-sensitive per spec and real
    fail-corpora test exactly that (`Flatedecode`); pdfce correctly
    refuses.

**Still in flight (continuation 7):**
- Pass 1.1 continues — xref/object streams now the sole big-ticket
  item; MAX_TOKEN_LEN sub-item done.
- Everything remains UNCOMMITTED in git — the operator has not yet
  said to commit.

**For next session:**
- Implement xref streams + object streams (ISO 32000 §7.5.8 /
  §7.5.7) — 489-file payoff, clears the corpus to ~99.6%.
- Re-run `tools/corpus-report` after landing to confirm the projected
  parse rate and record it in the Pass 1.1 ship entry.

**Same-day continuation 8 — Pass 1.1 item 1 SHIPPED (xref streams /
object streams / hybrid files); harvest gate cleared by measurement:**

- **All three PDF-1.5 structures implemented:**
  - **Cross-reference streams** (§7.5.8; Tables 17/18): `W` array
    field widths including the field-1 zero-width default (type
    defaults to 1), entry types 0 (free) / 1 (uncompressed) / 2
    (in-object-stream).
  - **Object streams** (§7.5.7): new `objstm.rs`; decoded container
    streams are cached per-container; `/Extends` is inert by design
    (accepted, never followed — no chain semantics pdfce needs).
  - **Hybrid files** (§7.5.8.4): `/XRefStm` consulted before `/Prev`,
    per the spec's search order.
- **Corpus re-run** (`tools/corpus-report`): veraPDF Ok **2,395 →
  2,884 (99.2%)**; `RefusedXrefStream` **489 → 0**; ALL 24 remaining
  non-Ok files across both corpora are deliberate `*-fail-*`
  conformance files; ZERO panics, ZERO timeouts; 12,927 pages
  rendered.
- **Decision 001 §6.3 gate now CLEARED BY MEASUREMENT, not
  projection:** 82.4% → **99.2% actual**, vs the <~95% trigger that
  would have forced the oxidize-pdf xref-recovery harvest. No harvest
  ever needed. Formal close recorded in `ARCHITECTURE.md` §12
  (2026-07-30, continuation 8 entry).
- **Tests:** 280 workspace tests green (17 new end-to-end PDF-1.5
  tests). **Fuzz smoke:** 879k executions/60 s, zero crashes;
  libFuzzer dictionary confirmed exploring xref-dict inputs.
- **API changes in `pdfce-core`:**
  - `IndirectObject.span` → `provenance: Provenance` enum
    (`File(ByteSpan)` | `ObjectStream { container, index }`) with a
    `file_span()` accessor. This makes the §5 round-trip contract
    **expressible-or-consciously-absent** for compressed objects: a
    compressed object has no contiguous file bytes, so a writer must
    promote it to an uncompressed object or rewrite its container —
    the obligation is documented on the type itself.
  - `XrefEntry` is now `#[non_exhaustive]`, with a new `InStream`
    variant.
- **Scope addition (engineer judgment — flagged to operator):**
  `XrefErrorKind::EncryptionUnsupported` — encrypted PDFs (§7.6) are
  now refused up front instead of silently rendering ciphertext. GUI
  `Status::Unsupported` repointed accordingly; 4 corpus files
  reclassified `RefusedEncrypted` (they are among the 24 non-Ok
  `*-fail-*` accounting above).
- **Tolerances chosen (recorded here so the posture is auditable):**
  - `/Type` **absent** is tolerated on XRef and ObjStm streams;
    `/Type` **present-but-wrong** is refused.
  - A malformed individual xref-stream row is skipped, per
    §7.5.8.3's unknown-type posture (treat as absent, keep going).
  - A broken `/XRefStm` degrades to the classic xref view —
    spec-guaranteed safe per §7.5.8.4's completeness guarantee (the
    classic section must be usable by pre-1.5 readers).
  - ObjStm `/N` and `/First` must be direct objects.
- **RAG escalations this continuation:**
  - `C:\personal_rag\pdf\lesson_20260730_xref_stream_w_default_hybrid_fallback_objstm_drop.md`
    — NEW: the corpus-verified xref-stream facts (W field-1
    zero-width default = type 1; hybrid `/XRefStm`-before-`/Prev`
    with classic-view fallback safe; §7.5.7's streams-in-objstm
    prohibition is what makes dropping the decoded container buffer
    safe).
  - `ARCHITECTURE.md` §12 dated entry (gate closure + `Provenance`
    API evolution + encryption-refusal scope addition) and a §5
    body amendment for the compressed-object round-trip posture.

**Still in flight (continuation 8):**
- Pass 1.1 continues — remaining items: pixel-parity measurement
  against a reference renderer; `pdfce-gui` file argument; Type 3 /
  `Tr` 4–7 (low); `/Resources` tolerance (deprioritized, awaiting
  organic files).
- Everything remains UNCOMMITTED in git — the operator has not yet
  said to commit.

**For next session:**
- Pick up the remaining Pass 1.1 items (pixel parity is the
  substantive one — it is what closes Pass 1's original acceptance
  bar).
- Surface the encryption-refusal scope addition to the operator (it
  was engineer judgment, not an operator request).

**Same-day continuation 9 — Pass 1.1 slice SHIPPED (Form/Image
XObjects `Do` + inline images); MAX_XOBJECT_DEPTH corpus-corrected:**

- **Shipped:** Form and Image XObjects (`Do`) + inline images — the
  biggest measured render-fidelity gap flagged by continuation 7's
  corpus run. Full detail in `ROADMAP.md` Shipped, "Pass 1.1 (slice) —
  Form and Image XObjects (`Do`) + inline images." New
  `crates/pdfce-core/src/filters/ascii.rs` (`ASCIIHexDecode`/
  `ASCII85Decode`, §7.4.2/§7.4.3 — required first because they're the
  only two filters that make an inline image's data length unambiguous
  per §8.9.7); new `crates/pdfce-render/src/image.rs` (image XObjects +
  inline images → RGBA, full §8.9.5.2 pipeline, DeviceGray/RGB/CMYK +
  CalGray/CalRGB + ICCBased-via-`/N` + Indexed, bpc 1/2/4/8/16, stencil
  masks as a separate polarity-switch path, `MAX_IMAGE_PIXELS` = 32 Mpx
  guard); `crates/pdfce-render/src/interpret.rs` gained `Do` dispatch
  (§8.10.1 five-step form procedure) and inline-image routing.
- **Decisions made this session (recorded `ARCHITECTURE.md` §12,
  2026-07-30 continuation 9):**
  1. Nested form execution = fresh `Interpreter` over a cloned
     `GraphicsState`, never `q`/`Q` on the shared stack — makes
     §8.10.1 steps (a)/(e) structural, gives each form its own font
     cache (correctness, not optimization: `/F1` in a form's own
     `/Resources` is a different font than the page's `/F1`).
  2. XObject cycle guard keyed on object number, not resource name.
  3. Text objects don't cross the form boundary (§9.4.1 BT/ET scoping
     holds structurally); text *state* is inherited (§9.3 makes it
     graphics state). Pinned by a test.
  4. Images drawn via a tiny-skia `Pattern` shader over the user-space
     unit square, not `Pixmap::draw_pixmap` (whose integer x/y origin
     can't express §8.9.4's arbitrary-affine placement). Escalated to
     `D:\dev\rag\rust\tiny_skia_0.11_pattern_shader_arbitrary_affine_image_placement.md`
     (ecosystem-wide tiny-skia finding, not pdfce-specific).
  5. `MAX_XOBJECT_DEPTH` raised 16 → 64.
- **Findings + decisions:**
  - **The `MAX_XOBJECT_DEPTH` = 16 intuition guard overflowed on
    exactly one of 2,914 corpus files** —
    `veraPDF-corpus/PDF_A-1b/6.1 File structure/6.1.12 Implementation
    limits/veraPDF test suite 6-1-12-t08-pass-c.pdf`, a **conformant**
    32-deep form-XObject chain (objects 19–50). Annex C sets no
    form-nesting limit; PDF/A §6.1.12 forbids imposing Annex C limits.
    Raised to 64 (2× the deepest conformant structure measured);
    corpus-wide overflows now 0. **Second incident of the identical
    bug shape** (first was `MAX_TOKEN_LEN`, continuation 7) — new
    `ROADMAP.md` Standing rule added: validate every resource guard
    against veraPDF's §6.1.12 suite specifically before it ships.
    Filed: `C:\personal_rag\pdf\lesson_20260730_max_xobject_depth_verapdf_32_deep_conformant_chain.md`.
  - **Corpus re-run** (same 2,914 files, isolated via a scratch
    build with only `Do`/inline-image arms reverted): deferred ops
    7,347 → 6,079 (−17.3%); images rendered 0 → 76; images unsupported
    0 → 137; forms rendered 0 → 1,168; glyphs substituted +37 (text
    inside forms now paints); xobject depth overflows 0; zero panics/
    timeouts/hangs. Full table in `ROADMAP.md`'s Shipped entry.
  - **`images_unsupported` (137) now EXCEEDS `images_rendered` (76) —
    DCTDecode (baseline JPEG) is the measured next priority**, ahead
    of CCITT/JBIG2/JPX/LZW. Filed:
    `C:\personal_rag\pdf\lesson_20260730_corpus_image_codec_priority_dct_first.md`.
    `ROADMAP.md` Pass 1.1 item 6 records the full follow-on priority
    list (DCTDecode; CCITT/JBIG2/JPX/LZW; `/SMask`+`/Mask` — blocked
    on a spec-RAG clause-11 GAP, dispatch `pdfce-spec-librarian`
    first; Lab/Separation/DeviceN for images; `/OC` on XObjects).
  - `tiny-skia` 0.11 `Pattern`-shader-vs-`draw_pixmap` finding written
    to `D:\dev\rag\rust\` (decision 4 above) — `Shader::transform`
    post-concatenates `fill_path`'s CTM argument into the pattern's own
    transform, giving image→user→device in one matrix; `draw_pixmap`
    hardcodes `anti_alias: false` and an integer blit origin, so it
    cannot be used for arbitrary-affine image placement.
- **Tests:** 337 workspace tests green (was 245 at Pass 1; 74 new in
  pdfce-render). `cargo fmt --all --check` + `cargo clippy --workspace
  --all-targets --all-features -- -D warnings` clean.
- **Invariant checks:** `cargo tree` for pdfce-core/pdfce-render/
  pdfce-cli on host, `x86_64-pc-windows-msvc`, and
  `wasm32-unknown-unknown` — no egui/eframe/winit/wgpu; `cargo tree
  --duplicates` clean; `cargo check -p pdfce-core -p pdfce-render
  --target wasm32-unknown-unknown` clean. One new test-only
  dev-dependency (`flate2` on pdfce-render, already resolved via
  pdfce-core, adds zero packages — no `THIRD_PARTY_LICENSES.md`
  regeneration needed).
- **Packaging:** untouched this slice, no smoke test owed; per the
  launch-on-completion rule the GUI was rebuilt and launched, and
  `pdfce-cli render-page` run against real corpus files (a gradient
  image, a 4-image page, the 32-deep form chain) with visually
  verified PNG output.

**Still in flight (continuation 9):**
- Pass 1.1 remains open: pixel-parity measurement (the substantive
  remaining item), `pdfce-gui` file argument, R20 diagnostics panel,
  and the item-6 image-codec/transparency follow-on (DCTDecode first).
- Everything remains UNCOMMITTED in git — the operator has not yet
  said to commit.

**For next session:**
- Implement `DCTDecode` next — corpus-justified as the single
  highest-value remaining image filter (`images_unsupported` >
  `images_rendered` today).
- Dispatch `pdfce-spec-librarian` for PDF_Spec clause 11 (soft/
  explicit/colour-key transparency) before starting `/SMask`/`/Mask` —
  currently a flagged GAP in the spec RAG.

**Same-day continuation 10 — addendum to continuation 9's Form/Image
XObjects slice (fuzz-target extension + post-slice refinements):**

- **`cargo-fuzz` coverage extended in the same slice**, per
  `ARCHITECTURE.md` §10.2's "expand fuzz targets to each filter decoder
  as they're implemented."
  `fuzz/fuzz_targets/content_and_filters.rs` now also calls
  `pdfce_core::filters::ascii::decode_hex` and `decode_85` directly on
  the raw fuzz input (not through `decode_stream`, so no dictionary
  shape stands between libFuzzer and the byte loops). Rationale
  (recorded in the target's module docs): ASCII85 has genuine overflow
  surface — a five-digit group accumulates to a value that can
  legitimately exceed `u32::MAX` (`uuuuu` = 85^5 − 1), partial final
  groups index a fixed-size array by a running count, and `z`/`~>` are
  position-sensitive.
- **Campaign result: 588,048 ASan-instrumented executions in 120 s,
  ZERO crashes** on the extended target
  (nightly-x86_64-pc-windows-msvc, using the documented
  `clang_rt.asan_dynamic-x86_64.dll` PATH workaround from
  `D:\dev\rag\rust\cargo_fuzz_windows_msvc_asan_dll_path.md` — that
  lesson was followed and remains accurate, no update needed to it).
- **Two small test/API refinements after the main dispatch:** a
  regression test pinning §8.10's "a `/BBox` with zero width or height
  is legal and means paint nothing" (the failure mode guarded against
  is treating the degenerate rectangle as *absent* and painting the
  form unclipped — the exact opposite of the spec), and removal of an
  unused `pub` helper (`ImageNotes::any`) from `pdfce-render`'s public
  surface, per the Rust API Guidelines' don't-ship-unused-public-items
  posture.
- **Final counts (supersede continuation 9's provisional numbers,
  `ROADMAP.md` Shipped entry amended to match):** **338 workspace
  tests passing** (not the 337 quoted in continuation 9), fmt clean,
  clippy `-D warnings` clean, GUI-separation verified for
  pdfce-core/pdfce-render/pdfce-cli on host + `x86_64-pc-windows-msvc`
  + `wasm32-unknown-unknown`, `cargo tree --duplicates` clean, wasm32
  `cargo check` clean.
- `ROADMAP.md`'s Shipped entry for "Pass 1.1 (slice) — Form and Image
  XObjects" was amended in place (same Pass, same day, still an open
  entry — not a rewrite of settled history) to fold in the fuzz-target
  extension, the campaign result, the two refinements, and the
  corrected test count.

**Still in flight (continuation 10):**
- Same as continuation 9 — Pass 1.1 remains open: pixel-parity
  measurement (substantive remaining item), `pdfce-gui` file argument,
  R20 diagnostics panel, item-6 image-codec/transparency follow-on
  (DCTDecode first).
- Everything remains UNCOMMITTED in git — the operator has not yet
  said to commit.

**Same-day continuation 11 — decision 005 (image-codec strategy)
accepted + archived; Pass 2.1/2.2/2.3 scoped:**

- **Decision 005 accepted and archived** (fifth use of the KenAgent
  protocol): `docs/decisions/005-image-codecs.md`. Headline: **five
  permissive pure-Rust codecs, zero copyleft escalations needed** —
  the GPL C alternatives that were the obvious answers for two of the
  five (`jbig2dec`, OpenJPEG wrappers) were made moot; `LEGAL.md` §1
  stays open and unconstrained, and every codec is
  `forbid(unsafe_code)` compiler-enforced in the configuration pdfce
  builds (SIMD features off, R24).
- **Decisions made this session (recorded `ARCHITECTURE.md` §12,
  2026-07-30 decision-005 entry):**
  1. **Two-tier codec architecture** (R23): image codecs
     (DCT/CCITT/JBIG2/JPX) are a terminal stage dispatched by a new
     `pdfce_core::image_codec` module returning `CodedImage` (samples
     + codec-declared geometry + colour model, never a bare
     `Vec<u8>`); `filters::decode_stream` returns
     `FilterError::ImageCodec` for them; LZW alone stays a
     byte-stream filter in the cascade.
  2. **Crate selections:** `zune-jpeg 0.5` (DCT), `weezl 0.2` (LZW),
     `hayro-ccitt 0.3` (+`fax` fallback), `hayro-jbig2 0.3`,
     `hayro-jpeg2000 0.4` — all `default-features = false` per R24.
  3. **Priority order set by measurement** (2,914-file corpus:
     DCT 82.3% of unimplemented-filter occurrences / LZW 10.4% /
     JPX 7.3% / CCITT+JBIG2 zero *by corpus construction*): Pass 2.1
     DCT+LZW, Pass 2.2 CCITT+JBIG2 (roadmap-dependency-driven, one
     vendor), Pass 2.3 JPX. Filed under `ROADMAP.md` Next up with the
     full measurement tables so the order stays auditable.
  4. Standing rules **R23–R28** filed in `ROADMAP.md` (terminal-stage
     codecs; SIMD/unsafe features off with CI feature-state
     assertion; explicit ceilings, never vendor defaults; codec layer
     never decides colour; named fail-clean diagnostics; read-compat
     only — pdfce writes none of these codecs, LZW never).
- **3 BLOCKING spec verifications dispatched** to
  `pdfce-spec-librarian`, one gating each Pass: §7.4.8 Table 13
  `/ColorTransform` (blocks 2.1), Table 11
  `/Columns`/`/EndOfBlock`/`/BlackIs1` defaults (blocks 2.2 —
  `BlackIs1` inverts every fax image plausibly if defaulted wrong),
  §8.9.5 `/SMaskInData` + Table 89 JPX overrides (blocks 2.3).
- **RAG escalations this continuation:**
  - `D:\dev\rag\rust\cargo_lock_unenabled_optional_deps_not_build_graph.md`
    — lockfile presence ≠ build-graph presence (the record's §3.6
    correction: `zune-jpeg`/`weezl` sat in `Cargo.lock` as unenabled
    optional deps of `image`; verify with `cargo tree -i`, never
    `grep Cargo.lock`).
  - `D:\dev\rag\rust\unsafe_simd_cargo_features_cfg_attr_forbid.md`
    — "does this crate use unsafe" is a feature-set question;
    feature unification can flip it silently; assert with
    `cargo tree -e features` in CI.
  - `C:\personal_rag\pdf\lesson_20260730_corpus_jpeg_shape_progressive_dri_app14.md`
    — corpus JPEG shape (14% progressive, 74% DRI, 80% APP14, 100%
    8-bit, 0% 4-component).
  - `C:\personal_rag\pdf\lesson_20260730_zune_jpeg_cmyk_app14_ambiguity.md`
    — zune-jpeg's CMYK report is ambiguous; PDF consumers must sniff
    APP14 themselves.
- **Still in flight (continuation 11):**
  - **Pass 2.1 (DCT + LZW) is next** — gated only on the Table 13
    spec verification above.
  - Pass 1.1 remainders unchanged (pixel parity, `pdfce-gui` file
    argument, R20 diagnostics panel).
  - Everything remains UNCOMMITTED in git — the operator has not yet
    said to commit.

**Same-day continuation 12 — Pass 2.1 SHIPPED (DCTDecode + LZWDecode
+ RunLengthDecode; two-tier image_codec architecture); decision 005
§3.2 measurement corrected; decision 006 dispatched:**

**Shipped:**
- **Pass 2.1** — full detail in `ROADMAP.md` Shipped. Highlights:
  - `DCTDecode` via `zune-jpeg 0.5.15` (SIMD off,
    compiler-enforced `forbid(unsafe_code)`, R24). The blocking
    Table 13 verification returned and the implemented precedence is
    verified against it: **APP14 marker overrides the dict
    `/ColorTransform`; default 1 for 3-component, 0 otherwise.**
    APP14 pre-sniff in pdfce's own adapter; **in-house YCCK→CMYK**
    (zune-jpeg has no YCCK→CMYK arm — YCCK passthrough requested).
  - `LZWDecode` via `weezl 0.2.1` (byte-stream cascade per R23, both
    `/EarlyChange` modes); `RunLengthDecode` in-house ~130 lines
    (small scope addition).
  - The **two-tier image-codec architecture landed** (R23):
    `pdfce_core::image_codec`, `CodedImage` seam, `terminal_codec`
    dispatch; **explicit ceilings** `MAX_IMAGE_PIXELS` /
    `MAX_IMAGE_DIMENSION` / `MAX_IMAGE_SAMPLE_BYTES` per R25
    (zune-jpeg's 16,384-pixel vendor default overridden).
  - **Corpus:** Ok 99.2% → **99.3% (2,886)**; images rendered
    **74 → 201**; images unsupported **135 → 8** (7 JPX for
    Pass 2.3 + 1 deliberate `/Lzw`-misspelling fail-file). Zero
    panics. **Tests: 412** workspace (was 338); fmt/clippy clean.
    **Fuzz: 4 targets, zero crashes** (per-codec targets + both LZW
    `EarlyChange` modes in `content_and_filters.rs`).
  - `THIRD_PARTY_LICENSES.md` regenerated: **+3 permissive entries**
    (`zune-jpeg`/`zune-core`/`weezl`), zero copyleft.
  - **All invariant gates green incl. the new R24 feature assertion**
    (`cargo tree -e features`: no `x86`/`neon`/`simd`).

**Decisions made this session (recorded `ARCHITECTURE.md` §12,
continuation-12 entry):**
- Three **engineer deviations from decision 005's §6.3 API sketch**,
  all additive: (1) `CodedImage::codec` is an `Option` with an
  `Unspecified` variant; (2) `decode_image` takes an `inline: bool`
  parameter; (3) RunLength truncation is a strict `Err`.
- **Decision 006 dispatched** (KenAgent protocol) — the CMYK/YCCK
  inversion rule, now that revisit trigger 2 fired (below).

**Findings + decisions:**
- **CORRECTION — decision 005 §3.2's "0 four-component JPEGs" was
  WRONG: 12 exist**, in veraPDF's "6.2.4.3 Uncalibrated -Device
  colour spaces" section (the §3.2 scan missed them — the record's
  own method caveats anticipated this failure mode). **Revisit
  trigger 2 (§9) is LIVE:** `6-2-4-3-t02-pass-a.pdf` is
  `/DeviceCMYK` `/DCTDecode` with Adobe APP14 transform 2 (YCCK) and
  NO `/Decode` array — it relies on the bare Adobe convention;
  pdfce currently passes raw samples through (§5.5's deliberate
  no-guess), so these 12 likely render inverted today. Filed as a
  dated addendum at the END of
  `docs/decisions/005-image-codecs.md` (record not rewritten);
  corpus-shape lesson amended to match.
- **Pass 1 inline-image bug found and fixed (content.rs):** `ID`
  followed by CRLF is ONE white-space character (§8.9.7's "single
  white-space character" read with §7.2.2's CRLF-is-one-EOL rule);
  Pass 1 consumed only the CR, leaving a stray `\n` prepended to the
  image data — silently corrupting 4 corpus inline DCT images
  (caught by their SOI check failing).
- **zune-jpeg defers the 3-component/transform-0 CMYK→RGB fixup into
  `decode_into`** — `input_colorspace()` at header time reports CMYK
  for such images, so sizing/requesting by header-time colorspace
  fails; and a no-APP14 `/ColorTransform 0` stream needs a **YCbCr
  passthrough** request, not RGB — two visually-identical cases with
  different correct requests.
- **RAG escalations this continuation:**
  - `C:\personal_rag\pdf\lesson_20260730_inline_image_id_crlf_single_whitespace.md`
    — NEW: the ID-CRLF inline-image trap.
  - `C:\personal_rag\pdf\lesson_20260730_zune_jpeg_cmyk_app14_ambiguity.md`
    — AMENDED (dated addendum): the `decode_into` fixup deferral +
    the YCbCr-vs-RGB request distinction.
  - `C:\personal_rag\pdf\lesson_20260730_corpus_jpeg_shape_progressive_dri_app14.md`
    — AMENDED (dated footer): the 4-component figure corrected
    (0 → 12, veraPDF 6.2.4.3).
  - `docs/decisions/005-image-codecs.md` — dated addendum appended
    (the §3.2 correction + trigger-2 activation).

**Still in flight (continuation 12):**
- **Decision 006 (CMYK/YCCK inversion rule) is pending** — until it
  lands, the 12 four-component corpus files likely render inverted
  (visible, counted, deliberate per §5.5).
- Pass 2.2 (CCITT + JBIG2) is next in the codec sequence — still
  gated on its Table 11 blocking verification.
- Pass 1.1 remainders unchanged (pixel parity, `pdfce-gui` file
  argument, R20 diagnostics panel).
- Everything remains UNCOMMITTED in git — the operator has not yet
  said to commit.

**For next session:**
- Consume decision 006 when it returns; implement the sourced
  inversion rule and re-render the 12 four-component files as its
  acceptance check.
- Pass 2.2 after its Table 11 verification (BlackIs1 polarity is the
  dangerous default).

**Continuation 13 (2026-07-31) — decision 006 RETURNED and consumed:
the premise was falsified; the rule is the null rule; no behavioral
change needed:**

**Shipped:**
- Nothing behavioral — by decision. Decision 006's deliverables are a
  permanent rule, a corrected/split diagnostic, documentation, and six
  regression fixtures (engineering follow-ups this continuation).

**Decisions made this session (recorded `ARCHITECTURE.md` §12,
2026-07-31 entry; full record
`docs/decisions/006-cmyk-jpeg-inversion.md`):**
- **Decision 006 (CMYK/YCCK JPEG inversion) ACCEPTED via the KenAgent
  protocol (sixth use). The rule is the NULL rule: pdfce never applies
  an "Adobe CMYK inversion"** — not on APP14 presence, transform-byte
  value, component count, or producer sniffing; `/Decode` is the sole
  polarity control (R29). Sourced by four-engine consensus (pdf.js,
  pdfium, MuPDF PDF path, Poppler — conditions read at source) plus a
  twice-shipped-twice-reverted upstream trail for the marker heuristic
  (cairo issue 156, Firefox bug 674619).
- **The 005-Addendum premise was FALSIFIED, in a good way:** pdfce
  already pixel-matches pdfium on ALL real four-component corpus JPEGs
  — count corrected 12 → **9** (two independent scans; 2 payloads,
  9 PDFs, all YCCK transform-2, no `/Decode`). The "likely render
  inverted today" inference was flatly false; §5.5's no-guess posture
  was CORRECT, not merely cautious — the plausible APP14-gated guess
  would have broken all 9. Second dated addendum appended to
  `docs/decisions/005-image-codecs.md` (first addendum left as
  written).
- **TN #5116 negative result:** the normative-by-reference primary
  contains the word "invert" zero times; §13.1's `255 −` is the YCCK
  definition (on true ink), and §18 does not enumerate transform
  values 0/1/2. The inversion convention is undocumented Photoshop
  behavior, compensated out of band via `/Decode` — never detectable
  from the codestream.
- **R31 born from a near-miss:** Pillow (first reference consulted)
  applies `CMYK;I` to every 4-component JPEG unconditionally and
  reported the exact complement — trusting it would have "fixed" a
  non-bug and broken all 9 files under a green suite. Reference
  decoders are evidence only after their conventions are verified.
- **R26 clarified: observing is not applying** — the codec adapter may
  observe `dict` to classify diagnostics; its anti-inversion clause is
  now permanent and sourced.
- Standing rules **R29–R31 + the R26 clarification** added to
  `ROADMAP.md` (librarian filing, this continuation).

**Findings + decisions:**
- **Separate `DeviceCMYK`→RGB colorimetry gap found in passing (006
  §3.7), deliberately excluded from 006:** `Rgb::from_cmyk`
  (`gstate.rs:112`) naive additive vs pdfium's `AdobeCMYK_to_sRGB1` —
  37.4% of pixels differ >8 in some channel (max Δ `[11,37,30]`) on
  the corpus CMYK image; affects EVERY DeviceCMYK fill/stroke, not
  just images. **Filed as a new `ROADMAP.md` Backlog entry
  ("DeviceCMYK→RGB colorimetry")** — scope via
  `pdfce-acrobat-librarian` before engineering.
- **RAG escalations this continuation** (four new
  `C:\personal_rag\pdf\` lessons + indexes; corpus-shape lesson
  re-amended 12 → 9):
  - `lesson_20260731_pillow_cmyk_i_rawmode_trap.md` (methodology/HIGH)
  - `lesson_20260731_cmyk_jpeg_never_invert_four_engine_consensus.md`
    (quirk/HIGH)
  - `lesson_20260731_libjpeg_ycck_255_minus_is_definitional.md`
    (format-spec/HIGH)
  - `lesson_20260731_corpus_cmyk_resurvey_9_not_12.md`
    (methodology/MEDIUM)

**Follow-ups dispatched / in flight (006 §10):**
- **Engineering items 2–6 (this continuation, docs/diagnostics/
  fixtures ONLY — no behavioral change):** dct.rs module + function
  doc rewrite (settled R29 rule + TN #5116 §13.1 citation); split
  `dct_cmyk_images` (benign census, no warning) vs
  `dct_cmyk_polarity_unverifiable` (R30 named warning citing 006),
  classified in `dct::decode`; CLI/GUI note text updated so only the
  R30 counter warns; six `fixtures/synthetic/cmyk-variants/`
  regression fixtures (transform 2/0/marker-removed × ±`/Decode`)
  asserting CMYK sample values at named pixels + render polarity,
  with CC BY 4.0 veraPDF attribution per `LEGAL.md` §5.
- **`pdfce-spec-librarian` dispatch pending** for `filter__dct.md`
  (close the SOURCING GAP with the negative result; rewrite the
  hazard note; add the consensus table + revert trail) — 006 §6.5.
- Re-derive or retire the "12" figure (006 §10 item 10).

**Still in flight (continuation 13):**
- Pass 2.2 (CCITT + JBIG2) — unchanged, still gated on Table 11
  verification.
- Pass 1.1 remainders unchanged (pixel parity, `pdfce-gui` file
  argument, R20 diagnostics panel).
- Everything remains UNCOMMITTED in git — operator has not said to
  commit.

**For next session:**
- Decision 006 is fully consumed once the engineering follow-ups above
  are verified green (fmt/clippy/tests/ui-strings/invariants). The
  "re-render the 12 files as acceptance check" item from continuation
  12 is superseded: the files (9, not 12) already render correctly —
  the acceptance check became the six committed fixtures.
- Scope the DeviceCMYK colorimetry Backlog entry when reached.

**Continuation 14 (2026-07-31) — Pass 2.2 SHIPPED (CCITTFaxDecode +
JBIG2Decode + shared bilevel sink):**

**Shipped:**
- **Pass 2.2** — full detail in `ROADMAP.md` Shipped. Highlights:
  - `CCITTFaxDecode` via `hayro-ccitt 0.3` (zero deps, `no_std`,
    `forbid(unsafe_code)`): `DecodeSettings` maps 1:1 onto Table 11,
    defaults verified against the returned blocking spec dispatch;
    `/K` trichotomy implemented; **`/Rows` 0 → `/Height` fallback is
    load-bearing** (hayro-ccitt decodes ZERO rows at `rows: 0`).
  - `JBIG2Decode` via `hayro-jbig2 0.3` (`image`/`simd` off, R24);
    `Image::new_embedded()` carries `/JBIG2Globals`; inline-image
    path rejects JBIG2 per §7.4.7/§8.9.7.
  - **Shared bilevel sink** (§8.9.3 packing, per-row byte budget) —
    one packing implementation behind both codecs.
  - **Polarity chain PROVEN by executable identity:** `/BlackIs1` →
    hayro `invert_black` by DIRECT assignment (hayro XORs
    internally, sink writes 1-for-white, PDF 0 = black); JBIG2's
    T.88 1 = black inverted unconditionally; the same Group-4
    payload decodes **byte-identical** through the CCITT route and
    the JBIG2-MMR-generic-region route (asserted in `pdfce-core` +
    pixel-identical renders). Decision 005 §10 item 2's
    silent-inversion hazard closed by proof, not code read.
  - **Corpus honestly UNCHANGED at 99.3%** — 0 files / 0 occurrences
    of either filter in the conformance corpora, confirming the
    decision 005 §5.1 zero-by-construction prediction by direct
    scan. Ship-on-zero-pressure was the scoped, deliberate posture
    (OCR/scanned-document buckets are the demand signal).
  - **Tests: 457** workspace (was 420; ~45 new incl. 17 core +
    6 render). **Fuzz: 6 targets, 60 s each, zero crashes.**
    **R25 gate: veraPDF §6.1.12 suite 44/44.**
  - `THIRD_PARTY_LICENSES.md` regenerated: **+2 permissive entries**
    (`hayro-ccitt`/`hayro-jbig2`), 169 total, zero copyleft.
    **Binary +306.5 KiB (+10.28%), measured.**

**Decisions made this session (engineer deviations, all additive —
recorded in the `ROADMAP.md` Shipped entry):**
- Extra named diagnostic for CCITT damaged rows beyond the scoped set
  (R27 posture).
- `pdfce-render` needed NO code change — `SampleLayout` was already
  generic over 1-bpc data; the render side is 6 new tests, zero code.
- New `fixtures/synthetic/bilevel/` (5 demo PDFs + `PROVENANCE`).
- One **honest test relaxation:** `[0xFF; 32]` is VALID Group 4 data —
  T.4/T.6 carry no checksum, so fail-clean cannot detect undetectable
  garbage; documented in a comment rather than asserted falsely.

**Findings + decisions:**
- **libtiff's fax codec is bit-based and IGNORES
  PhotometricInterpretation** — T.4 white runs correspond to Pillow
  BLACK pixels, so fixture generators must build the visual
  complement (byte-exact asserted). Lesson filed (below).
- **hayro-ccitt `rows: 0` decodes nothing** — PDF's `/Rows`-absent/
  0 → `/Height` fallback is what makes real streams decode; treating
  it as cosmetic default-plumbing would have shipped a decoder that
  silently produces zero rows. Lesson filed (below).
- **CCITT↔JBIG2 polarity identity** — the same Group-4 payload
  embedded as a JBIG2 MMR generic region decodes byte-identical to
  the CCITTFaxDecode route once each side's convention (BlackIs1
  XOR; T.88 unconditional inversion) is applied; a reusable
  cross-codec differential oracle. Lesson filed (below).

**RAG escalations this continuation:**
- `C:\personal_rag\pdf\lesson_20260731_libtiff_fax_ignores_photometric_generators_complement.md`
- `C:\personal_rag\pdf\lesson_20260731_hayro_ccitt_rows_zero_decodes_nothing.md`
- `C:\personal_rag\pdf\lesson_20260731_ccitt_jbig2_mmr_polarity_identity.md`
- `D:\dev\rag\rust\libfuzzer_arbitrary_derive_parameter_space_fuzzing.md`
  — deriving codec parameter dicts from fuzz input alongside the
  payload (the CCITT parameter cross-product pattern).
- Subject + master indexes updated for all four.

**Open spec item (carried in ROADMAP too):**
- `PDF_Spec\filter__jbig2.md` still marks **Table 12's exact contents
  unverified** — Pass 2.2 implemented against §7.4.7's quoted prose;
  a future `pdfce-spec-librarian` dispatch closes it. T.4/T.6/T.88
  source specs staged in the spec RAG, code tables unextracted.

**Still in flight (continuation 14):**
- Pass 2.3 (JPXDecode) is next in the codec sequence — still gated on
  its §8.9.5 `/SMaskInData` + Table 89 blocking verification.
- Pass 1.1 remainders unchanged (pixel parity, `pdfce-gui` file
  argument, R20 diagnostics panel).
- Everything remains UNCOMMITTED in git — operator has not said to
  commit.

**For next session:**
- Pass 2.3 after its blocking spec verification (§8.9.5
  `/SMaskInData`, Table 89 JPX overrides, Table 6 no-parameters).
- Dispatch `pdfce-spec-librarian` to close the Table 12 gap in
  `filter__jbig2.md` (and extract the staged T.4/T.6/T.88 code
  tables) when convenient — non-blocking.
- Re-measure CCITT/JBIG2 corpus presence the moment an organic
  (non-conformance) corpus exists.

**Continuation 15 (2026-07-31) — Pass 2.3 SHIPPED (JPXDecode) — Pass 2
COMPLETE:**

**Shipped:**
- **Pass 2.3** — full detail in `ROADMAP.md` Shipped. Highlights:
  - `JPXDecode` via `hayro-jpeg2000 0.4` (`default-features = false,
    features = ["std"]` — `simd`/`image` off per R24; Apache-2.0 OR
    MIT, permissive, no license escalation). New `image_codec/jpx.rs`
    + `fixtures_jpx.rs` (12 generated fixtures,
    `tools/gen-jpx-fixtures.py` via OpenJPEG/Pillow 12.1.0,
    lossless-round-trip-asserted) + 6 demo PDFs in
    `fixtures/synthetic/jpx/` + new fuzz target `image_codec_jpx`.
  - **Fuzz bug found AND fixed:** a 310-byte codestream declaring a
    65,536-tile grid over 512×1024 → 32 s decode; the tile grid is
    declared independently of image size, so NO existing pixel/byte
    ceiling saw it. `jpx::MAX_TILES = 4096` (8× most aggressive real
    tiling; 32 Mpx can still tile 91×91); same input now 3 ms; kept
    as fuzz corpus seed + an accept-side test so the ceiling can't
    silently over-tighten. Final campaign 15,694 runs / 60 s / zero
    crashes.
  - **Corpus** (same 2,914): Ok holds 2,892 (99.2%); images rendered
    204 → 210; images unsupported 9 → 3; codec-unsupported 7 → 0;
    codec-FEATURE-unsupported 0 → 1 — NAMED:
    JPX/enumerated-colour-space (CIEJab, space 19, §7.4.9-permitted,
    unimplemented upstream; was a generic corrupt-file error before).
    Zero panics/timeouts.
  - **Tests: 487** workspace (was 457). Gates all clean incl. MSRV
    1.92 builds core+render (no bump), `cargo-about` regen (+1
    entry), R24 assertion on 4 targets, `--duplicates` guard, wasm32.
    GUI launched on `jpx-rgba-smaskindata1.pdf`.
  - **Pass 2 (image codecs, decision 005) is COMPLETE as planned** —
    all five standard filter families now decode or fail with named
    diagnostics.

**Decisions made this session (six engineer deviations — full record
in `ARCHITECTURE.md` §12, continuation-15 entry; also condensed in
the `ROADMAP.md` Shipped entry):**
- **The dispatch brief stated Table 89 precedence BACKWARDS**;
  verified rule implemented: a PRESENT `/ColorSpace` wins ("any
  colour space specifications in the JPEG2000 data shall be
  ignored"); the codestream wins only when absent.
  `/BitsPerComponent` + `/Decode` ignored as briefed. Test:
  `jpx_present_colour_space_still_wins`.
- `/Width`/`/Height` are not a Table 89 override — dict-for-placement
  / codestream-for-stride split retained, divergence counted;
  per-filter contrast table added to `mod.rs` docs.
- Bit depth normalized by full-range scale to 8
  (`round(v/(2^d−1)×255)`), not high-byte — Table 89 leaves depth to
  the conforming reader; 16-bit fixture carries a `0x00FF`
  discriminator pixel.
- `/SMaskInData` 2 recognize-and-defer: preblended colour returned as
  stored, alpha not exposed; counter `jpx_smask_in_data_preblended` →
  CLI key `jpx_preblended` (appended, stable-line contract kept) +
  GUI line.
- EXTRA Table 89 gap found in the audit and closed: `decode_stencil`
  hard-required 1-bit and would have sheared a JPX `/ImageMask`'s
  8-bit samples 8× — stencil path now takes stride/depth from the
  codec, thresholds against zero, `/Decode` still honoured (§7.4.9
  exemption).
- hayro's `data_u8()` deliberately unused: interleaves alpha AND
  computes `1 << bit_depth` on a palette-box depth that may be 128 —
  shift-overflow panic reachable from fuzzed input; pdfce interleaves
  itself and refuses depths outside `1..=31` (`JPX/bit-depth`).

**Findings + decisions:**
- **`jpx::MAX_TILES` — declared-work amplification is its own guard
  class:** a decode-WORK ceiling orthogonal to every pixel/byte
  ceiling; third guard-by-intuition encounter (after MAX_TOKEN_LEN
  and MAX_XOBJECT_DEPTH), FIRST found by the fuzzer rather than a
  rejected real file. Lesson filed (below).
- **CIEJab / §7.4.9-ladder divergence:** when the codec can't decode
  an unsupported enumerated colour space at all, the spec's "fall
  back to Device* by channel count" rung is unreachable —
  decode-then-classify orderings matter. Lesson filed (below).
- **hayro-jpeg2000 `data_u8()` shift-overflow hazard** (deviation
  above) as an api-usage lesson. Lesson filed (below).
- **libFuzzer slow-unit calibration:** the 1 s slow-unit threshold
  isn't calibrated for megapixel decoders — a 17 Mpx legitimate image
  is 1.0 s native vs 22 s under ASan (false positive); prescribe
  `-report_slow_units=30` in the target's docs. Lesson filed (below).

**RAG escalations this continuation:**
- `C:\personal_rag\pdf\lesson_20260731_jpx_max_tiles_declared_work_amplification.md`
- `C:\personal_rag\pdf\lesson_20260731_jpx_ciejab_decode_then_classify_fallback_unreachable.md`
- `C:\personal_rag\pdf\lesson_20260731_hayro_jpeg2000_data_u8_shift_overflow_palette_depth.md`
- `D:\dev\rag\rust\libfuzzer_slow_units_asan_threshold_calibration.md`
- Subject + master indexes updated for all four.

**Correction to Continuation 14 (recorded here per the append-only
rule — 14 is not edited):** its "Still in flight" note carried
Pass 2.3 as "still gated on its §8.9.5 `/SMaskInData` + Table 89
blocking verification." That verification had in fact ALREADY
returned before Pass 2.3 was dispatched — the Pass started unblocked.
(The dispatch brief derived from that verification then stated the
Table 89 precedence backwards; caught and implemented correctly —
see the first deviation above.)

**Still in flight (continuation 15):**
- The Pass 2.2 open Table 12 spec item: the `pdfce-spec-librarian`
  dispatch is now **IN FLIGHT** (running in parallel with this
  filing) — dispatched, not pending. Covers `filter__jbig2.md`'s
  Table 12 gap + the staged T.4/T.6/T.88 code-table extraction.
- **Decision 007** (next-subsystem priority) — KenAgent consultation
  in flight; the next Pass is unscoped until it returns.
- Pass 1.1 remainders unchanged (pixel parity, `pdfce-gui` file
  argument, R20 diagnostics panel).
- Everything remains UNCOMMITTED in git — operator has not said to
  commit.

**For next session:**
- Scope the next subsystem Pass once decision 007 returns.
- Fold the Table 12 dispatch result into the spec RAG's
  `filter__jbig2.md` open item when it lands (closes the Pass 2.2
  carried item).
- Re-measure CCITT/JBIG2/JPX corpus presence the moment an organic
  (non-conformance) corpus exists.

**Continuation 16 (2026-07-31) — decision 007 RETURNED and filed
(next subsystem = the incremental-save writer, sliced 3.0 → 3.1 →
3.2); Table 12 spec gap CLOSED same day; write-direction audit
dispatched:**

**Decisions made this session:**
- **Decision 007** (next-subsystem-after-read-stack) — KenAgent
  consultation returned and consumed; archived to
  `docs/decisions/007-next-subsystem-after-read-stack.md` via the
  transcript-extraction pattern: Markdown record + base JSON decision
  block, the consultation's final-message patch applied (effective
  JSON in Appendix A; raw patch block retained verbatim in Appendix B
  for auditability), and an orchestrator reconciliation note appended
  at archival grounding the record's §11 housekeeping items against
  Continuation 15's state.
- **Outcome: candidate A — the incremental-save writer — sliced into
  three Passes, the first with NO editing capability.** Ranking
  A ≫ D > B > C. Pass 3.0 = identity writer + corpus-wide round-trip
  proof harness (the §5 invariant becomes an executable 2,892-file
  gate BEFORE any mutation code exists; §11.4's undo obligation does
  not bind a Pass with no mutation). Pass 3.1 = mutation writer +
  dirty-set diff + undo/redo command log (`/Info` + `/Rotate` only;
  key test: edit → undo → save byte-identity). Pass 3.2 = structural
  page ops (blocked on the EMPTY Acrobat_Features RAG — a
  `pdfce-acrobat-librarian` "Core document ops" dispatch is owed
  before its acceptance criteria are written). Pass 4 = text
  extraction (Backlog bucket newly CREATED — ranked second overall,
  designated fallback track if a writer Pass hits the three-attempts
  wall). Pass 5 = encryption (decrypt ALL handlers; encrypt-on-save
  AES-128/256 ONLY, RC4 never written; promotion ahead of Pass 4
  rides on the organic `/Encrypt` census run in parallel with
  Pass 3.0; §7.6 is the largest spec gap — a full spec-librarian
  corpus session precedes it). Pass 6+ = render remainders
  DECOMPOSED, not one Pass (shading/transparency fold into the
  Vector-graphics-editing bucket; Type 3 + `Tr` 4–7 and
  `/SMask`/`/Mask` are already Pass 1.1 items 4 and 6.3). Sixteen
  risks enumerated **W1–W16** (W2's redaction/incremental-save
  content-leak hole and W1's per-object-vs-per-file identity
  confusion the sharpest); standing rules **R32–R41** added to
  `ROADMAP.md`. Filing anomaly recorded at the ROADMAP rules block:
  the archived record's header says "Adds standing rules: R32–R40" —
  R41 arrived via the final-message patch; the archived file is not
  edited (append-only), a dated librarian note at the rules block
  carries the reconciliation.
- **ROADMAP restructured accordingly:** Pass 3.0 filed as the active
  Next-up Pass (10 deliverables, 6 acceptance criteria, 4 binding
  non-goals, the parallel `/Encrypt` census, the self-vs-reference
  raster-oracle distinction, both blockers recorded — the
  write-direction audit and the hayro-write changelog re-check);
  Passes 3.1/3.2 queued behind it; the "Text extraction / structured
  content" Backlog bucket created; the Encryption Backlog entry
  updated; the Vector-graphics bucket gained the decision 007
  fold-in note; `ARCHITECTURE.md` §12 entry appended.

**Findings + decisions:**
- **Table 12 spec gap CLOSED same day** (Pass 2.2's carried open spec
  item; the dispatch was in flight at Continuation 15):
  `pdfce-spec-librarian` verified Table 12 — **one key,
  `/JBIG2Globals`**; **no code defects** in pdfce; the `jbig2.rs`
  doc-comment paraphrase was corrected the same day, with
  `cargo fmt --check` / `cargo check` clean. One follow-up robustness
  question filed in ROADMAP as a Pass 2.x remainder: §7.4.7 rule 3a
  (segment page association = 1) is should-only — `hayro-jbig2`'s
  behavior on non-1 page association (blank-page risk) and on
  Annex D.2 random-access input is undetermined.

**Still in flight (continuation 16):**
- **`pdfce-spec-librarian` write-direction audit DISPATCHED, in
  flight** — Pass 3.0 blocker (a): §7.5.4/.5/.6/.8 emission
  coverage, §7.5.8.4 hybrid write side, §14.4 `/ID` (absent from the
  RAG).
- Pass 3.0 blocker (b) is the engineer's designated FIRST action:
  re-check `hayro-write`'s changelog for byte-preserving incremental
  append (decision 001 §9 trigger 2 is live).
- KenAgent worktree pruned after archival.
- Pass 1.1 remainders unchanged (pixel parity, `pdfce-gui` file
  argument, R20 diagnostics panel).

**For next session / operator items still open:**
- **Encryption-refusal sign-off (SESSION_LOG:879)** — decision 007
  flags this as the TOP operator action: pdfce refuses encrypted PDFs
  (`XrefErrorKind::EncryptionUnsupported`, an engineer-judgment scope
  addition during Pass 1.1 item 1) and the operator has not yet been
  told pdfce declines a category of file every other reader opens.
  Surface it BEFORE the `/Encrypt` census runs.
- License decision (`LEGAL.md` §1) — still open; gates public repo,
  release, and distribution manifests.
- Commit authorization — everything remains UNCOMMITTED in git.
- Git remote / CI (decision 007 **W15**): CI has never run; whether
  to establish a remote is the operator's call, gated on LEGAL §1.
  Pass 3.0's round-trip gate must accordingly be runnable and green
  LOCALLY as a hard acceptance criterion, never depending on CI for
  its correctness claim.

**Continuation 17 (2026-07-31/08-01) — Pass 3.0 SHIPPED (identity
writer + round-trip proof harness; the §5 invariant is now an
executable, GREEN, corpus-wide gate); `/Encrypt` census RETURNED
(promotion trigger NOT met); Pass 3.1 engineer DISPATCHED:**

**Shipped:**
- **Pass 3.0 — identity writer + round-trip proof harness** (full
  entry: `ROADMAP.md` Shipped, Pass 3.0; deviations:
  `ARCHITECTURE.md` §12 continuation-17 entry). Headline numbers,
  over 2,898 loadable of 2,914 corpus files (16 NotLoadable =
  deliberate `*-fail-*` files): empty-dirty-set `save_incremental`
  whole-file byte identity **2,898/2,898 = 100.00%**; append
  identity 2,898/2,898; `save_full` per-object-definition verbatim
  2,897/2,898 = 99.97%, the single miss a CORRECT named refusal
  (hybrid "Isartor test suite manual.pdf" →
  `WriteError::HybridFullRewrite`, CLI exit 8, R33/R27 posture;
  incremental works on it via form A); raster self-oracle
  5,783/5,783; 0 objects re-serialized under
  `SaveOptions::identity()`; 0 panics/timeouts; W14's ~98% STOP
  threshold never approached. Structural census byproduct: 2,410
  classic / 487 xref-stream / 1 hybrid / 36 live-linearized.

**Decisions made this session:**
- **Blocker (b) — the engineer's designated first action — resolved
  NEGATIVE:** `hayro-write` 0.7.0 (2026-05-27) self-describes as an
  internal `pdf-writer` converter, ~580 LoC, no incremental append.
  Decision 001 §9 trigger 2 does NOT fire; depend-or-contribute
  stays closed and Pass 3.0 proceeded on pdfce's own writer.
- **Six engineer deviations, recorded as decisions**
  (`ARCHITECTURE.md` §12 continuation-17, items a–f):
  `ProducerPolicy::Set` never CREATES a missing `/Info` (R41
  anti-stamping); `save_full` carries object streams intact, zero
  promotions (structurally avoids W3 — type-2 entries name
  container+index, not offsets); hybrid full-rewrite refused BY
  NAME; no predictor on emitted xref streams (§7.5.8 never mentions
  write-side predictors — negative audit result); no wildcard match
  arms in the writer (`#[non_exhaustive]` doesn't bind in the
  defining crate — new variant = compile error, not silent null); a
  NUL-bearing Name emits `#00` and fails reload deliberately
  (§7.3.5).
- **`ARCHITECTURE.md` §5.1–5.6 + §11.2 amendments LANDED in-Pass**
  (deliverable 9), closing the body-section deferral carried by the
  continuation-16 §12 entry.

**Findings + decisions:**
- **`/Encrypt` census RETURNED** (decision 007 parallel cheap task,
  run by a parallel agent — the engineer's residual saying it "was
  not run" is STALE): 19,940 organic PDFs scanned (20k cap hit,
  Dropbox-dominated; read-only, aggregate counts only, nothing
  copied — LEGAL §5). **134 = 0.67% carry `/Encrypt`**; 26 R2 /
  30 R3 / 67 R4 / 10 R6 / 1 undetermined-R (FOPN FileOpen DRM,
  non-Standard handler — never silently openable); **92.5% legacy
  R≤4**; empty-vs-real user password not determinable pre-Pass-5.
  **Promotion trigger NOT met — Pass 5 stays behind Pass 4.** Dated
  result recorded at the Encryption Backlog entry.
- Demo run: `round-trip` identical=1 (709 → 709 bytes);
  append-identity `/Prev`=base-startxref, 20-byte SP-LF entries;
  producer preserve-vs-set; hybrid refusal exit 8; linearization
  warning. GUI launched but opened BLANK — `pdfce-gui` still lacks a
  file argument (open Pass 1.1 remainder); rendering verified via
  the CLI `render-page` PNG instead.
- Gates: fmt/clippy clean; **585 workspace tests** (was 487);
  GUI-free invariant on 3 targets; wasm32; `--duplicates` guard;
  veraPDF §6.1.12 suite **44/44** on the new writer-side guards; all
  8 fuzz targets build; `writer_roundtrip` fuzz campaign 661,190
  ASan execs / 61 s, zero crashes; dependency set UNCHANGED (no
  `cargo-about` regeneration owed).
- **RAG escalations filed this continuation:**
  `C:\personal_rag\pdf\lesson_20260731_span_backed_stream_derived_partialeq_cross_buffer.md`
  (span-backed Stream derived-PartialEq trap; the
  `equivalent_across_buffers` fix pattern),
  `C:\personal_rag\pdf\lesson_20260731_roundtrip_verify_linear_span_reload_not_quadratic_substring.md`
  (linear span-comparison-through-reload beats quadratic per-object
  substring search AND is strictly stronger — proves reachability
  via the new xref, not mere presence), and
  `D:\dev\rag\rust\non_exhaustive_no_effect_defining_crate_wildcard_free_match.md`
  (`#[non_exhaustive]` has no effect inside the defining crate).
  All three indexes updated.

**Still in flight (continuation 17):**
- **Pass 3.1 (mutation writer + dirty-set diff + undo/redo command
  log) engineer DISPATCHED — in flight, parallel with this filing.**
  Promoted to ROADMAP In progress.

**For next session / operator items still open (corrected list — the
engineer's carried items 1 and 5 predated same-day closures: the
census WAS run, `filter__jbig2` Table 12 CLOSED 2026-07-31, the
`filter__dct` sourcing gap closed earlier):**
- Pass 1.1 pdfium pixel-parity harness — still open; NOT closed by
  Pass 3.0's self-comparison oracle (do not overclaim; the
  raster-oracle note binds).
- Encryption-refusal operator sign-off (SESSION_LOG:879) — still the
  TOP operator action; now informed by the census: 0.67% of organic
  files are affected, 92.5% of those legacy R≤4.
- W15 — no git remote, CI never run; operator's call per LEGAL §1.
- License decision (LEGAL §1) — still open.
- Commit authorization — everything remains UNCOMMITTED in git.

**Continuation 18 (2026-07-31) — Pass 3.1 SHIPPED (mutation writer +
dirty-set diff + undo/redo command log; §11.1's union-bug is now an
executable green gate); CRITICAL stale-copy correction to decision
007 W3 / ARCHITECTURE §5.2 filed forward; Pass 3.2 promoted (blocked,
acrobat-librarian dispatch in flight):**

**Shipped:**
- **Pass 3.1 — mutation writer + dirty-set + undo command log** (full
  entry: `ROADMAP.md` Shipped, Pass 3.1; deviations + correction:
  `ARCHITECTURE.md` §12 continuation-18 entry; design record: §5.7 +
  §11.5). New surface: `EditSession` command log
  (`crates/pdfce-core/src/edit.rs`, 1,608 lines), `writer/fileid.rs`
  (§14.4 `/ID[1]` derivation), `DirtySet` (replacements + trailer
  patch + `changes_content`), `save_full` now takes `&DirtySet` (one
  writer path — `DirtySet::empty()` makes identity a strict pinned
  subset), CLI `set-info` / `rotate-page` / `--verify-undo` / exit 9 /
  appended `promoted=` key, GUI properties panel + rotate +
  undo/redo + "Save a copy…", `tools/roundtrip` mutation mode, fuzz
  edit-history extension. **Key test: edit → undo → save
  byte-identical 2,897/2,897 (100%)** + 6 dedicated fixture tests
  (incl. an object-stream file, a 12-command history, undo → redo →
  save). Pass 3.0 identity gate UNPERTURBED per R34 (2,892/2,892 +
  6/6; full-rewrite 2,891/2,892 with the same correct hybrid named
  refusal; raster 5,783/5,783; 0 re-serialized). Mutation gate: edit
  applied + reloaded 100%; all other objects byte-verbatim 100%.
  52 new tests (32 core + 20 CLI) over the 585 baseline; fmt/clippy
  clean; GUI-core separation verified; dependency set UNCHANGED;
  nothing committed.

**CRITICAL correction (prominent by design — recorded forward; the
archived 007 decision file is NOT edited):**
- Decision 007 W3's mitigation and `ARCHITECTURE.md` §5.2's original
  framing claimed R35's full rewrite "closes the stale-copy path" for
  promoted compressed objects. **FALSE — object streams carry through
  verbatim in BOTH save modes**, so a promoted object's old value
  survives inside its untouched container. Documented at the creating
  code. **The Redaction Pass must rewrite/decompose every container
  stream holding a redacted object** — R35's incremental-save refusal
  is necessary but NOT sufficient. Filed: dated correction note at
  `ROADMAP.md` R38; §5.2 correction footer + new §5.7 + §11.2
  cross-ref in `ARCHITECTURE.md`; §12 continuation-18 entry.

**Decisions made this session (engineer deviations 1–5, recorded as
decisions — `ARCHITECTURE.md` §12 continuation-18, items a–e):**
- (1) One writer path: `save_full` takes `&DirtySet`; identity is a
  strict pinned subset via `DirtySet::empty()`.
- (2) `/ID` never synthesised when absent, either mode (R41) — the
  spec RAG's synthesise-on-full-rewrite recommendation DECLINED;
  deferred to a real Save-As path.
- (3) Rotate-to-base-value writes nothing — exact spelling restored,
  4 quarter-turns net zero, `/Rotate 450` not normalised (R33).
- (4) Text encoding ASCII-or-UTF-16BE+BOM only — §7.9.2/Annex D.3
  PDFDocEncoding is a RECORDED RAG GAP; undecodable bytes → U+FFFD
  with `exact: false` surfaced in the GUI (fuzzy-never-sneaky).
- (5) GUI applies on button, not keystroke — one undo step per
  operator intent.

**Findings + decisions:**
- **Fuzzer found + fixed a real bug:** object creation raised `/Size`
  and RESURRECTED xref entries the base `/Size` was suppressing (they
  then failed to parse). Fix: `next_object_number` allocates above
  the UNFILTERED chain maximum (was reusing live numbers) + creation
  refused by name when `/Size` suppresses entries
  (`EditError::ObjectCreationWouldExposeHiddenObjects`, exit 9;
  editing existing objects still works). Post-fix 408,886 runs / 91 s
  zero crashes; `load_document` 681,645 / 61 s clean.
- **R38 promotion is fixture-covered, NOT corpus-covered:** 75 corpus
  files hold 2,197 compressed objects but page objects are
  uncompressed in all — rotation never promotes on the corpus; the
  harness reports both numbers so the gap stays visible.
- **RAG escalations filed this continuation:**
  `C:\personal_rag\pdf\lesson_20260731_xref_size_suppresses_trailing_entries_raising_resurrects.md`
  (under-reported `/Size` is a live real-world shape: entries beyond
  `/Size` must stay hidden; raising `/Size` resurrects them — the
  fuzz find). `D:\dev\rag\rust\cargo_fuzz_windows_msvc_asan_dll_path.md`
  CONFIRMED still accurate (already names
  `clang_rt.asan_dynamic-x86_64.dll` + the PATH fix; `last_verified`
  bumped, no duplicate written). Indexes updated.

**Still in flight (continuation 18):**
- **UI follow-up items handed to `pdfce-ui-specialist` — review in
  flight, parallel with this filing.**
- **Pass 3.2 promoted to In progress but BLOCKED:
  `pdfce-acrobat-librarian` dispatched for the "Core document ops"
  bucket — in flight, parallel.** Engineer not yet dispatched;
  acceptance criteria wait for the bucket.

**For next session / operator items still open (unchanged from
continuation 17 except as noted):**
- Pass 1.1 pdfium pixel-parity harness — still open.
- Encryption-refusal operator sign-off (SESSION_LOG:879) — still the
  TOP operator action.
- W15 — no git remote, CI never run; operator's call per LEGAL §1.
- License decision (LEGAL §1) — still open.
- Commit authorization — everything remains UNCOMMITTED in git
  (Pass 3.1 included).

**Continuation 19 (2026-07-31) — Pass 3.2 SHIPPED (structural page
operations — the first operator-visible editing feature: seven ops,
real free-list deletion, DocMDP-grounded signature awareness); the
R36 rule-number collision reconciled (§5.4's rule is now R42); the
Tools-dock/toolbar-cap UI conventions adopted as standing decisions;
Pass 4 (text extraction) promoted, spec sourcing in flight:**

**Shipped:**
- **Pass 3.2 — structural page operations** (full entry:
  `ROADMAP.md` Shipped, Pass 3.2; decisions + deviations:
  `ARCHITECTURE.md` §12 continuation-19 entry; implemented against
  `docs/ui_specs/pass-3.2-page-ops.md` + the acrobat-librarian
  "Core document ops" bucket). New pdfce-core surface: `graph.rs`
  (`ObjectGraph` — ONE page-tree walk over the loaded file OR the
  `EditSession` overlay; `edit.rs`'s Pass-3.1 comment predicted the
  need), `signature.rs` (810 lines), `pageops/` (2,833),
  `tests/page_ops.rs` (967). Seven ops, two shapes: in-place
  `EditSession` commands `delete_pages`/`reorder_pages`/
  `rotate_pages` (one undo entry each); producers
  `extract`/`merge`/`split`/`insert` via one shared `assemble()`
  (new documents, no undo). W9 deletion: `DirtySet::delete` +
  `apply_free_list` (type-0, gen+1 saturating 65,535,
  front-spliced; pre-existing detached free entries untouched per
  R33; two-closure sweep proves shared objects never freed).
  **769 workspace tests (was 707).**

**Decisions made this session (`ARCHITECTURE.md` §12
continuation-19):**
- **UI-surface conventions adopted as standing decisions** (from the
  ui-specialist spec §1–2): the Tools dock is pdfce's ONE "more
  tools" secondary surface (future buckets become dock entries,
  never new floating windows; Properties stays the single legacy
  exception); the toolbar is CAPPED at 6 groups + the Tools toggle;
  rail-vs-dock rule — pages-in-the-open-document arguments live on
  the thumbnail rail, outside-the-document arguments live in the
  Tools dock.
- **R36 collision reconciled (the UI spec's flagged record
  defect):** `ARCHITECTURE.md` §5.4's linearization-never-repaired
  rule is now **R42** (dated note in `ROADMAP.md` Standing rules);
  decision 007's R36 (save-mode disclosure / signature either-or)
  keeps the number — code comments stay as-is; §5.4's citation
  corrected. No rule content changed, numbering only.
- **`SignatureImpact::ByteRangePreserved`** — renamed from the
  spec's `PreservedIncremental` per the mid-Pass DocMDP relay
  (§12.8.1 NOTE 1 preserves the byte range; DocMDP validity is a
  separate verdict). Classification via `/Reference` →
  `/TransformMethod` (`/DocMDP` never `/Perms`; `/P` defaults 2);
  `/Perms`→`/DocMDP` with forbidding `/P` ⇒
  `EditError::CertificationForbidsChange` NAMED refusal (Table
  258); `/FieldMDP` recognized. Spec closure:
  `PDF_Spec\iso32000__s__12.8.md` now 689 lines (a/b/c verdicts +
  the ByteRangePreserved-never-reported-alone rule).
- **Carryover policy table** (documented + cited in `pageops/`):
  outlines subset+repoint / per-source-top-level merge /
  target-only insert; `/Dests` never carried, carried bookmarks
  rewritten explicit; `/PageLabels` stale-for-insert + named
  diagnostic, dropped-for-subsets; `/StructTreeRoot` dropped +
  counted; form fields `Doc<N>_` auto-rename, straddling fields
  dropped whole + counted; barrier hits counted.
- **Engineer deviations from the UI spec (recorded):** insert is a
  producer, not an `EditSession` command (overlay insert needs
  per-object source buffers + an overlay-aware renderer — GUI
  Insert deferred, the dock names the CLI command); rail checkbox =
  one interaction + position test; `egui::Window` not `egui::Modal`
  (the spec's named fallback); `signature_impact_of_save(mode)`
  takes the mode; split's file-size criterion deferred + named.
- **Spec priority ledger:** P0 ALL shipped (incl. the REAL
  SignatureImpact API, not the fallback, and the dangling-reference
  count shipping WITH Delete); P1 signature API + dangling count
  shipped, GUI Insert deferred; P2 Merge GUI SHIPPED, Split GUI
  deferred (CLI complete).
- **Carried small items applied:** Apply/Revert grey-out, per-field
  lossy marking, command-named undo tooltips, **GUI file argument —
  the Pass 1.1 remainder is CLOSED**, rotate shortcut `[`/`]`.

**Findings + decisions:**
- **Two real bugs caught by this Pass's own tests:** (1) reorder
  lost inherited rotation — `materialize_for` was one-directional;
  `preserve_inherited` now writes §7.7.3.4's DEFAULT when the new
  parent chain supplies a value the old chain didn't (the
  silent-rotation bug class); (2) extract left `/Dest [null /Fit]`
  — the reference barrier now propagates through WHOLE destination
  arrays, dropping + counting the composite.
- **Gates:** fmt/clippy clean; 3.0/3.1 gates UNMOVED per R34
  (identity 2,892/2,892; full-rewrite 2,891/2,892 with the same
  correct hybrid refusal; edit → undo → save 2,891/2,891; raster
  5,771/5,771); corpus page-op sweep 2,892 extract-ok + 23/23
  delete-ok, 0 failures; §6.1.12 40 files clean, guard headroom
  MEASURED (outlines 10 vs 200k, dests 62 vs 100k, depth 3 vs 64,
  pages 10k vs 1M); fuzz `pageops_sequence` 130,400/61 s zero
  crashes + `writer_roundtrip` re-run clean; GUI-free both targets;
  wasm32 + aarch64 clean; `--duplicates` + R24 clean;
  **`ui-strings` R1 gate clean for the FIRST time** (3 pre-existing
  false positives fixed — more evidence CI has never run, W15). No
  new dependencies. GUI launched PID 23332 (via the new file
  argument); CLI demo split-by-bookmark → reverse-merge → render.
- **RAG escalations filed this continuation:**
  `C:\personal_rag\pdf\lesson_20260731_inherited_page_attr_move_writes_default_direction_trap.md`
  (bidirectional inherited-attribute materialization — write the
  spec default when the NEW chain supplies a value the old didn't)
  and
  `C:\personal_rag\pdf\lesson_20260731_dest_array_null_element_barrier_whole_array.md`
  (reference barriers must treat destination arrays as semantic
  units — never emit `[null /Fit]`). Subject + master indexes
  updated.

**Still in flight (continuation 19):**
- **Pass 4 (text extraction / structured content) promoted to In
  progress; engineer not yet dispatched;
  `pdfce-spec-librarian` §9.10 sourcing dispatch IN FLIGHT**
  (parallel with this filing) — `ToUnicode` CMaps, `/ActualText`,
  reading order.

**For next session / operator items still open:**
- `/Info` edits not certification-gated (`/P 1` strict reading —
  owed decision, recorded at `check_certification`).
- `PermissionGate::NotApplicableYet` — awaits Pass 5.
- Delete corpus coverage thin (23 multi-page files) — fixtures +
  fuzz carry it; re-measure on an organic corpus.
- `qpdf` not on PATH — R40's external oracle unused; an
  operator-installable improvement.
- Pass 1.1 pdfium pixel-parity harness — still open (GUI file
  argument is now closed; the harness is the main Pass 1.1
  remainder).
- Encryption-refusal operator sign-off (SESSION_LOG:879) — still
  the TOP operator action.
- W15 — no git remote, CI never run (the `ui-strings` false
  positives sat undetected until this Pass); operator's call per
  LEGAL §1.
- License decision (LEGAL §1) — still open.
- Commit authorization — everything remains UNCOMMITTED in git
  (Pass 3.2 included).

**Continuation 20 (2026-08-01) — Pass 4 SHIPPED (text extraction /
structured content: the §9.10.2 ladder verbatim, sourced total
99.78%, plain/sourced dual API); the placement taxonomy extended
three-way; a real pre-existing Ctrl+S GUI bug found and fixed; Pass 5
(encryption) promoted, §7.6 spec corpus session dispatched:**

**Shipped:**
- **Pass 4 — text extraction / structured content** (full entry:
  `ROADMAP.md` Shipped, Pass 4; decisions: `ARCHITECTURE.md` §12
  continuation-20 entry; implemented against the returned
  `pdfce-spec-librarian` §9.10 corpus + the ui-specialist spec
  `docs/ui_specs/pass-4-text-extraction.md`, 573 lines). 5,469 new
  `pdfce-core` lines: `textstring.rs` (§7.9.2 + Annex D.3
  PDFDocEncoding built from the annex's FOUR STRUCTURAL RULES, not
  256 transcribed rows — 4 source-table typos caught: 0xA0 = EURO,
  0xAD undefined, 0x18–0x1F modifier letters; 232 defined
  cross-checked vs D.2's 229 + 3);
  `text_extract/{cmap,font,page,layout,mod}.rs` — §9.10.3 ToUnicode
  parser, the §9.10.2 ladder VERBATIM with rung 3 structural+named
  (`Rung3Gap::{IdentityNoToUnicode, Ucs2NotBundled,
  PredefinedCmapNotBundled}` — never silently skipped), derived
  layout isolated in `layout.rs`. API: `ExtractedText` with
  `plain_text()` vs `sourced_text()` (the Drucker `/ActualText`
  example verifies both directions: sourced "Drucker", plain
  "Druc\nker" with one labelled derived break). **875 workspace
  tests (was 769).**

**Decisions made this session (`ARCHITECTURE.md` §12
continuation-20):**
- **(a) Confirmation-dialog convention → standing pattern** (two
  independent uses: Pass 3.2 signature confirmation + Pass 4
  pre-copy reliability gate): one centre-anchored, one-question,
  input-blocking window; the gate lives in the action dispatcher,
  not the window code.
- **(b) Placement taxonomy is now THREE-way** — rail
  (pages-in-document) / Tools dock (outside-the-document files) /
  toolbar-menu snapshot actions (copy-text, the first instance) — a
  dated EXTENSION of the Pass 3.2 rail-vs-dock convention (that
  record stands; forward pointer appended in §12). Copy-text is
  deliberately NOT a Tools-dock entry.
- **(c) `plain_text()`/`sourced_text()` dual API adopted as THE
  fuzzy-never-sneaky pattern for all extraction-like features** (OCR
  next): sourced characters and derived judgments are separate API
  surfaces, every derived insertion labelled.
- **Two additive counted deviations:** per-code fallthrough (§9.10.3
  NOTE 4 — unsourced universal practice) + glyph-name extension for
  fonts failing method 2's whole-array precondition;
  `FontNote::BuiltinEncodingUnreadable` names the one R21-unreachable
  case (embedded symbolic built-in encoding → StandardEncoding
  fallback — counted as extension, never sourced).
- **Bidi deferred-not-half-done:** RTL detected + counted;
  `unicode-bidi` NOT added (B1–B3 make reordering wholly derived).
- **Extraction diagnostics are a snapshot surface**, separate from
  the per-frame render header (merging would lie on navigation);
  pre-copy gate fires on `identity_fonts_without_to_unicode > 0 ||
  sourced < 50%` (deliberately not a low threshold).

**Findings + decisions:**
- **Measurements (2,907 files, 281,516 codes, 0 panics/timeouts):**
  rung 1 78,101 (27.74%); rung 2 202,793 (72.04%); rung 3 ZERO;
  extension 39 (0.01% — almost all the deliberately non-conforming
  Isartor 6-3-7 encodings file); failed 583 (0.21%); **SOURCED TOTAL
  99.78%**. Derived: 752 spaces, 1,905 line breaks.
- **Real pre-existing GUI bug found by the ui-specialist's
  verification and FIXED:** Ctrl+S fired through a live signature
  confirmation (doc comment claimed a guard that didn't exist;
  Pass 4's second centre-anchored window made it collidable) —
  one-question gate now at the top of `apply()`, doc comments
  corrected; `status_is_open()` now requires a page (`/Count 0`
  nit).
- **Gates:** fmt/clippy clean; **875 tests**; GUI-free both targets;
  wasm32/`--duplicates`/R24/`no-network`/`ui-strings` clean; Pass
  3.x gates UNMOVED; §6.1.12 44/44 with measured headroom (1,674
  CMap singles vs 500k, 2,044 ranges vs 100k); fuzz `text_extract`
  50,215 / 61 s zero crashes, 10 targets build; NO new deps;
  `cargo-about` byte-identical. Demo: CLI `hello.pdf` + CID fixtures
  both directions; GUI relaunched (PID 41588); 20-page tagged manual
  34,037 codes 100% sourced in 66 ms (the specialist's
  background-extraction concern measured-and-unneeded).
- **RAG escalations filed this continuation:**
  `C:\personal_rag\pdf\lesson_20260801_character_table_from_structural_rules_not_transcription.md`
  (build character tables from a source's structural rules, not
  row-by-row transcription, when the source table has known typos —
  the Annex D.3 pattern; the construction caught 4 typos);
  `C:\personal_rag\pdf\lesson_20260801_actualtext_no_per_glyph_offset_correspondence.md`
  (an `/ActualText` run has NO per-glyph offset correspondence — the
  API must model sourced text as run-level, the Drucker trap);
  `D:\dev\rag\egui\egui_0.35_two_center_anchored_windows_pending_state_gate_dispatcher.md`
  (two centre-anchored modal-ish windows collide silently in
  immediate mode — pending-state gates must live in the action
  dispatcher, not the window code; the Ctrl+S bug class — filed at
  the egui tier because it is immediate-mode-GUI-generic, not
  PDF-domain). Subject + master + egui indexes updated.

**Still in flight (continuation 20):**
- **Pass 5 (encryption) promoted to In progress — by decision-007
  SEQUENCE, not promotion** (the census result stands recorded at
  the Encryption Backlog entry: 0.67% `/Encrypt`, 92.5% legacy R≤4,
  trigger NOT met). **Engineer not yet dispatched;
  `pdfce-spec-librarian` §7.6 spec-corpus session DISPATCHED**
  (parallel with this filing) — §7.6 is the largest spec gap across
  all decision-007 candidates.

**For next session / operator items still open:**
- Pass 4 residuals (all named in the Shipped entry): bidi reordering;
  `/Alt`//`/E` counted-not-substituted; nested `/ActualText`
  outermost-wins; artifacts excluded-by-policy but present in runs;
  structure-tree order recognition-only; axis-aligned derived-layout
  assumption (rotated text over-produces line breaks); canvas
  text-selection deferred with its spec written (no core addition
  needed — `ExtractedGlyph` carries per-glyph `LadderRung` +
  geometry).
- `/Info` edits not certification-gated (`/P 1` strict reading) —
  still owed.
- Delete corpus coverage thin; `qpdf` not on PATH (R40 oracle);
  Pass 1.1 pdfium pixel-parity harness — all still open.
- Encryption-refusal operator sign-off — still the TOP operator
  action, now directly on Pass 5's path.
- W15 (no remote/CI), license decision, commit authorization —
  everything remains UNCOMMITTED in git (Pass 4 included).

**Addendum to Continuation 20 (2026-08-01, recorded per the
append-only rule):** two facts from the Pass 4 filing brief that the
entry above omits.
(1) **The decision-007 read→write→edit→extract stack is COMPLETE with
Pass 4** — Pass 4 was the last member of the 007 ranked sequence
except Pass 5 (encryption), which the `/Encrypt` census left
untriggered (0.67%) and which now proceeds in sequence order (see
Still in flight, above, and the `ROADMAP.md` In progress entry).
(2) **Decision 008 — the next subsystem AFTER the 007 sequence — is
in consultation via KenAgent (autonomous-builder), dispatched at
Pass 4's ship, in flight parallel with this filing.** Its result
files as `docs/decisions/008-*.md` per the KenAgent decision-routing
standing rule (2026-07-30) and will govern what follows Pass 5. A
dated librarian note carrying both facts is also at the `ROADMAP.md`
In progress (Pass 5) entry.

**Continuation 21 (2026-08-01) — Decision 008 CONSULTED and ARCHIVED;
next subsystem = Annotations & markup, sliced; Pass 5 repositioned;
R43–R52 added:**

**Decisions made this session (`ARCHITECTURE.md` §12
continuation-21; full record
`docs/decisions/008-next-subsystem-after-extract.md`, archived in
parallel by another agent):**
- **Decision 008 outcome:** after the decision-007
  read→write→edit→extract stack, the next subsystem is **Annotations &
  markup (candidate A)**, SLICED. Ranking across candidates
  **A ≫ B > C > E > D > F** (A = Annotations & markup,
  B = Forms/AcroForm, C = Redaction, E = Vector/Inkscape-parity,
  D = Text-&-object editing, F = Signatures/PAdES).
- **The slice / new sequence** (recorded in `ROADMAP.md` "Next up →
  Decision 008 sequence"): **Pass 6.0** — annotation & widget
  appearance rendering (read-side), now IN PROGRESS; **Pass 6.1** —
  authored streams + the project's first content-stream serializer +
  geometric markup authoring (Ink/Square/Circle/Line/Polygon/
  quad-point); **Pass 6.2** — text-bearing annotations + §12.7.3.3
  variable text (no `harfrust` per R17; Base-14 + embedded widget
  widths); **Pass 7** — Forms/AcroForm (B, second overall; display
  half IS 6.0, appearance half IS 6.2; embedded-JS posture is an
  explicit security decision — recommend never-execute); **Pass 8** —
  Redaction (C; content-stream surgery + container decomposition per
  §5.7; `/Redact` mark consumes 6.1; promotion trigger = a real
  operator redaction need); **Pass 9+** — Vector/Inkscape parity (E,
  sliced a–g; foundation is 6.1 so it can promote right after 6.1);
  **then Pass 5** — Encryption (fallback/interleave track, retains its
  007 ID); **Pass 10** — Signatures/PAdES (F, last; read half already
  far along).
- **Standing rules R43–R52 added** (`ROADMAP.md` Standing rules):
  R43 render-from-`/AP`-or-not-at-all (display sibling of R29);
  R44 generated appearances written to the file, never rendered from a
  private buffer; R45 authored bytes in a session staging buffer,
  `Stream` keeps its span model, the `DocumentView` assertion
  discharged by amending the type; R46 content-stream serializer proven
  by a corpus identity gate before it authors; R47 an annotation edit
  never touches the page content stream; R48 flatten is destructive and
  discloses incremental-save recoverability (R35 sibling); R49 a widget
  is an annotation first (one appearance pipeline); R50 hidden
  annotations honored AND counted (forensics); R51 `/NeedAppearances`
  disclosed, never silent auto-generate; R52 redaction mark and apply
  are separate operations with separate confirmations.

**Findings + decisions:**
- **Census (read-only, pypdf, aggregates only, LEGAL §5 posture):**
  conformance corpus **338/2,914 files (11.6%)** have annotations,
  228 with `/AP`, 127 `/AcroForm`, 4 `/XFA`; organic sample
  **2,500/25,203 Dropbox files** — 814 (32.6%) annotations, **753
  (30.1%) `/AcroForm`**, 43,508/55,545 annots have `/AP` (78.3%),
  `/Widget` 87.8% of annots, `/Tx` 99.8% of 47,868 fields, `/SigFlags`
  16 (0.64%), `/XFA` 2 (0.08%). **Per-file figures robust; the
  per-annotation figures are concentration-skewed and MUST be
  re-measured with pdfce's own tooling before any becomes a gate
  denominator** (decision 008 caveat W16). 0.64% `/SigFlags` recorded
  against the Signatures Backlog bucket; 0.08% `/XFA` recorded against
  the XFA bucket AND the standing `CLAUDE.md` "verify XFA status" open
  item — this answers the DEMAND half only; Adobe's XFA
  deprecation/support status is still unverified and NOT closed.
- **Structural finding F1 — pdfce renders NO annotations and does not
  even COUNT them: an UNDISCLOSED shortfall, unique in the project.**
  Every other unsupported item is R20/R27-counted; annotations are the
  one gap filed nowhere and reported to no operator. A new "Annotation
  display (read-side)" Backlog bucket was created for it (exactly as
  text extraction was unfiled pre-decision-007). R50 is the fix (honor
  AND count). F2: the §8.10.1 form-XObject execution path already
  shipped in Pass 1.1 and an `/AP` `/N` IS a form XObject — 6.0's
  rendering primitive already exists. F3: there is no content-stream
  writer yet and `Stream` cannot hold authored bytes (Pass 6.1 builds
  the first one). F4: the pageops/assemble staging-buffer pattern is
  the model for authored bytes, and `DocumentView`'s written assertion
  "a Pass that authors stream bytes must revisit this type" is
  discharged in 6.1 by deliberately amending the type (R45).
- **Pass-5-repositioning correction (append-only):** continuation 20
  promoted Pass 5 (Encryption) to In progress "by the decision-007
  SEQUENCE." Decision 008 supersedes that sequencing — the next
  subsystem is Annotations, not Encryption. **Pass 5 keeps its 007 ID,
  is NOT next, and moves to the fallback/interleave track after
  Pass 7.** Scope and the 0.67% `/Encrypt` census unchanged. A dated
  append-only correction note is at the `ROADMAP.md` In-progress
  (Pass 6.0) entry; the Pass 4 Shipped entry was NOT rewritten.
- **Two §4 staleness items logged as OWED (not fixed):** (1)
  `ARCHITECTURE.md` §4 still describes the Pass-0 header-probe state —
  decisions 001/004/005 owe their §4 core-data-model integration; (2)
  a consolidation session is owed to bring §4 up to Passes-1–5 reality
  BEFORE the annotation data model is documented there.

**Still in flight (continuation 21) — blocking dispatches:**
- **`pdfce-spec-librarian` §12.5 (Annotations) corpus session** —
  Pass 6.0 is blocked on it; IN FLIGHT; the engineer is NOT yet
  dispatched.
- **`pdfce-acrobat-librarian` "Comments & markup" bucket** —
  dispatched to ground Pass 6.1's acceptance criteria; IN FLIGHT.
- **Decision-008 record archival** to
  `docs/decisions/008-next-subsystem-after-extract.md` — another agent,
  IN FLIGHT (parallel with this filing).
- **Pass 9+ (Vector) needs an UNOWNED Inkscape capability catalog** —
  FLAGGED: no librarian owns an Inkscape feature-parity RAG today; one
  must be commissioned before 9+ is scoped into real Passes.

**For next session / STILL-OPEN operator items (ordered by age):**
- **Encryption-refusal operator sign-off** — now the OLDEST owed
  operator item, flagged by decision 008 as overdue. (Pass 5 has moved
  behind Pass 7, so the sign-off is no longer on the immediate build
  path, but it remains unresolved and is the longest-standing open
  operator action.)
- **License decision (`LEGAL.md` §1)** — still undecided; gates the
  public repo/release, the distribution-channel work, and whether
  copyleft prior art is ever usable.
- **Commit authorization** — everything remains UNCOMMITTED in git;
  Passes 0–4 (~20k+ lines) are all uncommitted, awaiting the operator's
  go-ahead (tied to the license decision).
- **W15 — no remote/CI** — CI has never run; the operator's call per
  `LEGAL.md` §1.
- Carried Pass-4 residuals and the `/Info`-not-certification-gated
  item remain open (see continuation 20).

**Continuation 22 (2026-08-01) — §7.6 encryption spec-corpus session
COMPLETE; Pass 5 spec-unblocked (still queue-deferred behind Pass 7);
`/R 6` sourcing gap found; two operator decisions raised:**

*(Spec-corpus filing, NOT a Pass ship. Append-only. A pre-compaction
dispatch that just returned.)*

**Shipped:**
- Nothing code-wise. This entry records the return of the
  `pdfce-spec-librarian` §7.6 corpus session dispatched at Pass 4's
  ship (continuation 20) and two operator decisions it surfaced.

**Findings + decisions:**
- **§7.6 encryption spec-corpus COMPLETE** (`pdfce-spec-librarian`, at
  `D:\Dev\Rag-Specialized\PDF_Spec\`): **7 new + 2 updated files** —
  `iso32000__s__7.6.1`–`7.6.5.md`; new `security__aes256_r5_r6.md`
  under a new `security__` prefix; `iso32000__ref__encryption_impl.md`
  (derived implementation checklist); `filter__crypt.md` de-stubbed;
  the Adobe ExtensionLevel 3 supplement staged. **This CLOSES the
  §7.6-largest-spec-gap prerequisite** that both decision 007 and
  decision 008 named as the blocker before Pass 5. **Pass 5 is no
  longer spec-blocked** — it remains **queue-deferred behind Pass 7**
  as the fallback/interleave track per decision 008 (only the spec
  prerequisite closed; queue position unchanged).
- **Finding that changes the Pass 5 plan (recorded, dated, at the
  ROADMAP Encryption Backlog bucket):** ISO 32000-1 contains **NO
  AES-256** — AESV3/AESV5, revisions R5/R6, SHA-256, Algorithms
  1.A/2.A/2.B and 8–13, and the `/OE` `/UE` `/Perms` `/Encrypt` entries
  are ALL sourced from **Adobe's ExtensionLevel 3 supplement**, not the
  base ISO standard. **`/R 6` — the AES-256 revision Acrobat X+ actually
  WRITES — could NOT be sourced** (ISO 32000-2 paywalled; no public
  ExtensionLevel 8 locatable; pdfa.org 403). The spec-librarian
  **correctly REFUSED to reconstruct Algorithm 2.B from memory** — the
  same no-fabrication discipline as the retracted URW claim.
- **Consequence for decision 007's "encrypt-on-save AES-128/256 only":**
  AES-256 is buildable **only at `/R 5`** (published weaknesses) today;
  `/R 6` cannot be implemented from sourced material. **Three honest
  options, a Pass-5 OPEN SUB-DECISION to settle BEFORE Pass 5 is scoped
  (a future KenAgent decision when Pass 5 activates):** (i) close the
  `/R 6` sourcing gap first; (ii) ship **AES-128 (`/V 4` AESV2) as the
  only write target**; (iii) **decrypt-only for AES-256** (never write
  it).

**Two OPERATOR decisions added to the standing operator-items list
(Ken's calls — cannot be resolved autonomously):**
1. **LEGAL.md §2 contradiction (do NOT edit LEGAL.md — flagged for
   Ken):** LEGAL.md §2 lists Adobe Supplements as freely publishable,
   but the ExtensionLevel 3 supplement's own copyright page says "no
   part … may be reproduced, stored … or transmitted" (a Technical Note
   #5004 posture, NOT the ISO 32000-1 posture). The spec-librarian
   applied the conservative reading (`license_basis:
   free_secondary_paraphrase` — paraphrase only, no bulk quotation) and
   flagged it in the RAG index's licensing table. Ken must confirm or
   override the LEGAL.md §2 wording.
2. **`/R 6` sourcing method:** closing the `/R 6` gap via
   cross-implementation triangulation (deriving Algorithm 2.B from ≥3
   permissively-licensed impls — qpdf / PDFBox / pdf.js) is permitted by
   the RAG's licensing rules but would be the FIRST time this corpus
   sources a NORMATIVE algorithm from code rather than a document.
   Alternative: Ken provides a purchased ISO 32000-2 copy. Ken's call.

**Still in flight (continuation 22):**
- **Pass 5 (Encryption)** — spec-unblocked, queue-deferred behind
  Pass 7 (decision 008). Engineer not dispatched; the two operator
  decisions above and the `/R 6` open sub-decision gate the Pass-5
  scoping when it activates.
- **Pass 6.0** and the decision-008 dispatches from continuation 21
  (`pdfce-spec-librarian` §12.5; `pdfce-acrobat-librarian` "Comments &
  markup"; decision-008 archival) — unchanged, see continuation 21.

**For next session / STILL-OPEN operator items (ordered by age —
oldest first):**
- **Encryption-refusal operator sign-off** — STILL the oldest owed
  operator item (unchanged). Now doubly relevant: the §7.6 corpus that
  would let Pass 5 replace the up-front refusal is complete, but Pass 5
  itself stays behind Pass 7.
- **NEW — LEGAL.md §2 Adobe-supplement copyright contradiction** (see
  operator decision 1 above): confirm/override the "Adobe Supplements
  freely publishable" wording against the ExtensionLevel 3 supplement's
  own restrictive copyright page. Flagged for Ken; LEGAL.md NOT edited.
- **NEW — `/R 6` sourcing method** (see operator decision 2 above):
  cross-implementation triangulation vs a purchased ISO 32000-2 copy.
- **License decision (`LEGAL.md` §1)** — still undecided (unchanged).
- **Commit authorization** — everything remains UNCOMMITTED in git
  (unchanged).
- **W15 — no remote/CI** — unchanged.
- Carried Pass-4 residuals and the `/Info`-not-certification-gated
  item remain open (see continuation 20).

**Same-day continuation 23 — Pass 6.0 SHIPPED (annotation & widget
appearance rendering, read-side):**

**Shipped:**
- **Pass 6.0 — Annotation & widget appearance rendering (read-side).**
  Moved to `ROADMAP.md` Shipped (full record there; top of Shipped,
  above Pass 4). The read-side display half of decision 008's
  Annotations & markup subsystem (candidate A). **ZERO authoring —
  R43 honored throughout:** pdfce paints an existing `/AP` `/N` or
  counts-by-name, and never synthesizes an appearance. Direct remedy
  for decision 008 finding **F1** (annotations were the project's one
  UNDISCLOSED, uncounted shortfall — every other unsupported item was
  already R20/R27-counted). **F2 confirmed in code:** an `/AP` `/N` IS
  a form XObject, so §12.5.5 placement routes through the EXISTING
  Pass 1.1 `interpret::run_form_at` → `do_form` path — X8 resource
  scoping, cycle guard, `MAX_XOBJECT_DEPTH`, and the per-form font
  cache all inherited unchanged, pinned by
  `appearance_uses_its_own_resources_not_the_page_font`.
- New surface: `pdfce-core` `annot.rs` (§12.5 walk/model/select,
  `AnnotFlags` §12.5.3 — `/Hidden`/`/NoView`/`/Print`/`/NoZoom`/
  `/NoRotate` each honored + counted per R50 — the `/AP`→`/N` + `/AS`
  `Appearance` taxonomy, and the document-scoped `need_appearances`
  query per R51); `pdfce-render` `annot.rs`; new public API
  `RenderOptions.annotations` + `RenderOptions::with_annotations`;
  Diagnostics +8 appended keys; CLI `render-page --no-annotations` +
  new `list-annotations` subcommand; GUI toolbar visibility toggle +
  status-bar disclosure line; `tools/corpus-report` census +
  `tools/gen-annot-fixtures.py` + `fixtures/synthetic/annot/PROVENANCE.md`
  (16 fixtures) + `tools/annot-pdfium-diff.py`; fuzz target 11
  `annot_walk.rs` (1.1M runs / 46 s, 0 crashes — cyclic `/AP`,
  degenerate/inverted `/Rect`, missing `/AS` state, `/N` neither stream
  nor dict).

**Findings + decisions:**
- **Census baseline PINNED (pdfce-native; replaces decision 008's
  pypdf conformance figures per W16 — now DISCHARGED for the
  conformance corpus):** 2,914 files, **ZERO panics** — **338 with
  annotations / 429 annotations total / 224 USABLE `/AP` `/N` / 127
  `/AcroForm` / 34 `/Popup` / 87 `/Widget`.** The per-file 338 and 127
  match pypdf exactly (tooling agreement). **The 224-vs-228 (pypdf)
  `/AP` gap is DEFINITIONAL, not an error** — pdfce counts a *usable*
  `/AP` `/N` (resolvable stream / selectable `/AS` state), pypdf counts
  raw `/AP`-key presence; the 4-file difference = ~2 `/AS`-unresolved
  state subdicts + ~2 absent/dangling/non-stream `/N`. pdfce's
  predicate is deliberately stronger. Recorded as a `personal_rag/pdf`
  finding (see RAG escalations).
- **Placement correctness (X2), NOT a pixel-parity close:**
  `tools/annot-pdfium-diff.py` (pypdfium2, decision 006 §3.2
  precedent) — 7/7 pure-geometry placement fixtures agree with pdfium
  within 4 px, 0 mismatches (identity, non-origin BBox, BBox-larger,
  BBox-smaller, Matrix-scale, Matrix-rotate, inverted `/Rect`); 6
  blank-expected cases (hidden/noview/popup/no-AP/state-missing/
  degenerate) correctly blank. **The engineer explicitly does NOT claim
  the Pass 1.1 pixel-parity remainder is closed** — this is an ink-bbox
  differential on the annotation subset only; full-page pixel parity
  over real corpus pages stays OWED.
- **Four deviations, recorded as decisions** (`ARCHITECTURE.md` §12
  continuation-23, item (c)): (1) `/NoZoom`/`/NoRotate` post-AA
  transform DEFERRED (counted + named; rare, near-exclusively on icon
  subtypes lacking `/AP` anyway; a wrong post-transform is worse than a
  disclosed omission); (2) `/OC` optional-content visibility test not
  implemented (consistent with NO optional content anywhere — BDC/EMC
  deferred, §8.11 a RAG GAP; an OC-off annotation currently paints,
  named); (3) `need_appearances_documents` is a document-scoped query,
  not folded into per-page `Diagnostics`; (4) GUI diagnostics are a
  separate always-evaluated status line below the content-diagnostics
  header, not folded into the content unsupported-tally (avoids
  destabilizing the tested content clean-return path; still honest
  R50/R27/R51; flagged for future ui-specialist refinement).
- **Durable FIVE-way GUI placement taxonomy** (ui-specialist
  deliverable — resolves the X14 drift; `ARCHITECTURE.md` §12
  continuation-23 (b); supersedes/extends the continuation-20 three-way
  taxonomy): view-state → toolbar view group; edit → toolbar/window;
  selection-scoped → rail; advanced → Tools dock; disclosure → status
  bar. THE settled convention for all future GUI placement.
- **Demand signals for 6.1/6.2/7:** the `annotations_without_ap`
  by-subtype histogram is corpus-dominated by no-`/AP` `/Link`,
  `/Widget`, `/Circle`; `/NoZoom`-`/NoRotate` and `/OC` are the two
  named display deferrals.

**Gates:**
- **901 workspace tests green (was 875)**; `cargo fmt --check` /
  `clippy -D warnings` clean; GUI-free invariant verified host +
  `x86_64-pc-windows-msvc`; wasm32; `--duplicates`; `ui-strings`;
  `no-network` all clean.
- **R34 holds STRUCTURALLY** — no pinned reference raster exists; the
  round-trip oracle is a runtime self-comparison, so painting
  annotations perturbs nothing the Pass 3.x/4 gates measure.
- **veraPDF §6.1.12:** new `MAX_ANNOTS_PER_PAGE = 1_000_000` (pure
  memory backstop — Annex C imposes no limit, §6.1.12 forbids imposing
  one; busiest corpus page ≪100, >10,000× headroom); `/AP` recursion
  reuses `MAX_XOBJECT_DEPTH = 64` unchanged (2× headroom vs veraPDF's
  32-deep conformant chain).

**RAG escalations this continuation** (`C:\personal_rag\pdf\`):
- `lesson_20260801_pdfium_widget_appearances_need_fpdf_ffldraw.md` —
  **api-usage / MEDIUM** — pdfium renders `/Widget` appearances only via
  `FPDF_FFLDraw` (the form-fill layer), not `FPDF_RenderPageBitmap`
  alone; a differential harness that omits it reports false diffs. The
  two apparent diffs in pdfce's pdfium comparison were REFERENCE
  divergences, not pdfce errors — pdfium also SYNTHESIZES the no-`/AP`
  `/Circle` `/IC` fill that R43 makes pdfce refuse. Verify a reference
  engine's own synthesis/fill behavior before trusting it as ground
  truth (sibling of the Pillow-CMYK and R31 lessons).
- `lesson_20260801_quadpoints_ccw_vs_z_order_producer_divergence.md` —
  **format-spec / MEDIUM — OPEN QUESTION** — §12.5.6 QuadPoints are
  spec'd CCW, but real producers / Acrobat emit them in Z / reading
  order; a generator that assumes CCW mis-orders quads on real files.
  Read-side (Pass 6.0) paints whatever `/AP` `/N` exists, so this does
  NOT bite until Pass 6.1 generates quad-point appearances. Carried to
  Pass 6.1 as the first place it matters.
- `lesson_20260801_usable_ap_n_vs_raw_ap_key_census_predicate.md` —
  **methodology / MEDIUM** — "has an appearance" is definitional: a raw
  `/AP`-key census (pypdf) over-counts vs a *usable* `/AP` `/N` census
  (resolvable stream / selectable `/AS` state). pdfce 224 usable vs
  pypdf 228 raw over 2,914 files; the 4-file gap = `/AS`-unresolved
  state subdicts + absent/dangling/non-stream `/N`. Pin the stronger
  predicate and re-measure before any census figure becomes a gate
  denominator (discharges decision 008's W16 for the conformance
  corpus).
- Subject + master indexes updated.

**Still in flight (continuation 23):**
- **Pass 6.1** (authored streams + content-stream serializer +
  geometric markup) — promoted to In progress; **blocked on the
  §8.10.2 form-XObject WRITE-direction audit** (`pdfce-spec-librarian`
  dispatched, in flight, parallel with this filing). The "Comments &
  markup" `pdfce-acrobat-librarian` bucket is COMPLETE. First Pass
  where the carried QuadPoints CCW-vs-Z-order question bites.
- **Pass 5 (Encryption)** — spec-unblocked, queue-deferred behind
  Pass 7 (decision 008); its `/R 6` open sub-decision and the two
  operator decisions gate scoping when it activates. Unchanged.

**For next session / STILL-OPEN operator items (ordered by age —
oldest first, unchanged from continuation 22):**
- **Encryption-refusal operator sign-off** — still the oldest owed
  operator item.
- **LEGAL.md §2 Adobe-supplement copyright contradiction** — flagged
  for Ken; LEGAL.md NOT edited.
- **`/R 6` sourcing method** — cross-implementation triangulation vs a
  purchased ISO 32000-2 copy. Ken's call.
- **License decision (`LEGAL.md` §1)** — still undecided.
- **Commit authorization** — everything remains UNCOMMITTED in git.
- **W15 — no remote/CI** — unchanged.
- **Full-page pixel-parity remainder** (Pass 1.1) — still owed; Pass
  6.0's 4-px annotation-subset differential is NOT a substitute.
- Carried Pass-4 residuals and the `/Info`-not-certification-gated item
  remain open (see continuation 20).

**Same-day continuation 24 — Pass 6.1 SHIPPED (authored streams +
content-stream serializer + geometric markup authoring — the project's
FIRST content-stream authoring Pass):**

**Shipped:**
- **Pass 6.1 — Authored streams + content-stream serializer +
  geometric markup authoring.** Moved to `ROADMAP.md` Shipped (full
  record there; top of Shipped, above Pass 6.0). Discharges decision
  008 findings **F3** (no content-stream serializer; `Stream` couldn't
  hold authored bytes) and **F4/R45** (authored bytes need a session
  staging buffer, not a mutated `Stream` type). Authors the
  **pure-geometry** markup annotations — Ink, Square, Circle, Line,
  Polygon, PolyLine, and the quad-point text-markup family
  (Highlight/Underline/StrikeOut/Squiggly). **NO text-bearing
  annotations** — FreeText/Text/Stamp + §12.7.3.3 variable text
  deferred to Pass 6.2 (one appearance pipeline). R43/R44/R47 honored:
  every authored appearance is a real baked `/AP` `/N` displayed by the
  SAME Pass-6.0 read path — never a private "what I just drew" render.
- New surface: `pdfce-core` `writer/content.rs` (`ContentBuilder` —
  §8.2 path/paint/gstate/colour operators + the §8.10 WF6 form-XObject
  ordering from the unblocking spec audit; `reemit_canonical` +
  `number_divergence_reason` for the R46 gate), `annot_author.rs`
  (`MarkupSpec`/`Color`/`Quad`/`LineEnding`/`TextMarkupKind`;
  `build_appearance` → `AuthoredAppearance` = annotation dict + `/AP`
  `/N` form XObject + content bytes). Modified: `writer/serialize.rs`
  (`write_real`/`write_name`/`write_string` now `pub(crate)`);
  `writer/mod.rs` (`DirtySet` gains the R45 staging buffer +
  `combined_source()`); `writer/save.rs` (serialize replacement/created
  objects against base++staging); `edit.rs` (`EditSession::add_markup`
  + staging + guards + `authored_source()` + COW `/Annots` patching +
  `AnnotKind` + `CommandKind::AddAnnotation` +
  `EditError::{DocumentEncrypted, EmptyGeometry, AnnotsNotAnArray}`);
  `pageops/assemble.rs` (`DocumentView` doc comment AMENDED to
  discharge — not delete — the R45 written assertion). **CLI**
  `annotate` subcommand (unified `--type`, per-subtype geometry flags,
  stable append-only stdout). **GUI** toolbar "Markup ▾" menu
  (minimal). `tools/content-identity/` (R46 corpus gate, out-of-tree).
  **Fuzz target 12** `annot_author.rs`.

**Findings + decisions:**
- **R46 content-stream identity gate — HEADLINE (serializer proven
  BEFORE it authors):** over the full corpus — **12,936 content streams
  across 2,898 loadable files; 12,854 byte-identical (99.37%); 82
  non-identical (0.63%); 0 CORRUPTED → GATE PASS.** The 82 are all
  spec-legal, VALUE-PRESERVED number re-spellings, enumerated by
  file+reason (R20): `.05`→`0.05` leading-zero insertion (20×), `-0`→`0`
  (18×), one 300-digit pathological real, `1.`→`1.0`. **Framing
  (recorded so it is never misread as fidelity loss):** this is a
  SERIALIZER-correctness test — the gate deliberately re-emits every
  stream. pdfce NEVER re-serializes untouched page content in normal
  save (span re-emission keeps it byte-verbatim, §5); authoring writes
  only NEW streams — so these divergences never occur in production
  save. X6 (silent normalization) is caught mechanically.
- **R44 round-trip verified** author→save→reload→paint: CLI authored a
  square+highlight+ink incrementally, `undo_identical=1` on the first
  (minimal-diff holds); `render-page` reports annots=3/painted=3/forms=3
  through Pass 6.0's read path; a render test confirms the red square
  paints red after reload — every appearance a real baked `/AP`.
- **X5** extract-from-authoring-session resolves via `authored_source()`
  (base++staging); the authored appearance survives BYTE-EXACT (the
  `DocumentView` assertion discharged by amendment). **X7** `/Annots`
  create / append-direct / COW-shared-indirect-array (sibling page
  provably untouched, tested) + a compressed-page fixture promotes out
  (R38, reload verified).
- **QuadPoints policy DECIDED (closes the Pass 6.0 carried open item):**
  pdfce authors quads in **Z / reading order (UL, UR, LL, LR)** — the
  dominant Acrobat/PDFBox/pdf.js convention, chosen for max third-party
  interop, documented in `annot_author.rs`. pdfce's own render is
  convention-independent (it paints the baked `/AP`) — so this is an
  interop decision, not a correctness one.
- **Deviations / residuals (recorded as decisions):** (1) X11
  certification gating is CONSERVATIVE — reuses `check_certification()`
  (refuses on ANY enforced `/DocMDP`), over-refuses annotation addition
  that `/DocMDP` `/P 3` permits; fail-clean-safe; per-`/P` refinement is
  a scoped spec-verified follow-up (check
  `certification_permission == Some(3)` for annotation-adding — §12.8
  already sourced). (2) X10 encryption guard is a forward-compat R37
  seam — encrypted files refused at LOAD today, so `DocumentEncrypted`
  in `add_markup` is unreachable until Pass 5; a test pins the load-time
  refusal. (3) NO `/M` mod-date / `/CreationDate` on authored
  annotations — avoids clock non-determinism breaking byte-compare
  tests; named residual (revisit when a deterministic-clock-injection or
  metadata policy exists). (4) Line-ending set = None/OpenArrow/
  ClosedArrow (Acrobat default Open honored); full §12.5.6.7 Table 176
  set not authored. (5) Square/Circle/Underline/StrikeOut/Squiggly
  default colours are pdfce's own (Acrobat RAG marks them a GAP);
  Highlight yellow + Multiply is the sourced default, locked.
- **Named GUI follow-up slice (Pass-6.1-followup, filed to the
  "Comments & markup" Backlog bucket):** the full canvas markup drawing
  state machine (drag/marquee/multi-click/ink-freehand + live preview +
  the screen↔page transform pass-4-only-planned + the ten-tool set +
  keyboard map) and P1 glyph-accurate text-selection markup.
  `docs/ui_specs/pass-6.1-markup-tools.md` is the design; the shipped
  GUI is a minimal menu affordance authoring at a default page-centred
  rect through the same `EditSession::add_markup` path. Core authoring
  path complete → this slice is pure GUI/interaction work, promotable
  independently of 6.2.
- **ARCHITECTURE §12 continuation-24 entry** appended (content-stream
  serializer landing + R45 staging-buffer reality + R46 gate result +
  QuadPoints Z-order decision).

**Gates:**
- **939 workspace tests green (was 901)**; `cargo fmt --check` /
  `clippy -D warnings` clean; **R34 re-runs GREEN** (Pass 3.0 identity
  identical=1 raster_identical=1 — authoring never perturbs untouched
  objects); GUI-free invariant verified core+render host +
  `x86_64-pc-windows-msvc`; wasm32; `--duplicates`; `ui-strings`;
  `no-network` all clean; fuzz target 12 `annot_author.rs` 696,098 runs
  / 61 s, 0 crashes.
- **ZERO new dependencies** — content serializer + markup authoring
  hand-rolled (no `harfrust`, consistent with R17);
  `THIRD_PARTY_LICENSES.md` unchanged.

**RAG escalations this continuation** (`C:\personal_rag\pdf\`):
- `lesson_20260801_quadpoints_ccw_vs_z_order_producer_divergence.md` —
  **UPDATED OPEN → RESOLVED-for-authoring** — pdfce authors quads in
  Z / reading order (the de-facto Acrobat/PDFBox/pdf.js convention);
  render is convention-independent (paints the baked `/AP`), so the
  ordering choice is interop, not correctness. Dated amendment footer
  added; the read-side note preserved.
- `lesson_20260801_serializer_number_respelling_reemit_gate_catalogue.md`
  — **NEW — quirk / MEDIUM** — the R46-gate number-respelling catalogue:
  which real-world number spellings a canonical PDF number serializer
  diverges from — leading-zero-absent `.05`→`0.05`, `-0`→`0`,
  trailing-dot `1.`→`1.0`, a 300-digit pathological real — all
  VALUE-PRESERVING. A re-emit-everything identity gate surfaces these
  (82 / 12,936 streams over 2,898 files); production span-re-emission
  never does (untouched streams pass through byte-verbatim). Serializer-
  authoring lesson.
- Subject + master indexes updated.

**Still in flight (continuation 24):**
- **Pass 6.2** (text-bearing annotations FreeText/Text/Stamp +
  §12.7.3.3 variable-text appearance generation) — promoted to In
  progress; **blocked on the §12.7.3.3 variable-text spec**
  (`pdfce-spec-librarian` dispatched, in flight, parallel with this
  filing). No `harfrust` (R17); appearance text laid out with Base-14
  metrics + embedded widget-font widths from Pass 1. The "appearance
  half" Pass 7 (Forms) reuses (R49).
- **Pass-6.1-followup GUI slice** — named, filed to Backlog; core
  authoring path complete, promotable independently.
- **Pass 5 (Encryption)** — spec-unblocked, queue-deferred behind Pass
  7 (decision 008); `/R 6` open sub-decision + two operator decisions
  gate scoping when it activates. Unchanged.

**For next session / STILL-OPEN operator items (ordered by age —
oldest first, unchanged from continuation 23):**
- **Encryption-refusal operator sign-off** — still the oldest owed
  operator item.
- **LEGAL.md §2 Adobe-supplement copyright contradiction** — flagged
  for Ken; LEGAL.md NOT edited.
- **`/R 6` sourcing method** — cross-implementation triangulation vs a
  purchased ISO 32000-2 copy. Ken's call.
- **License decision (`LEGAL.md` §1)** — still undecided.
- **Commit authorization** — everything remains UNCOMMITTED in git
  (Passes 0–6.1 ALL uncommitted).
- **W15 — no remote/CI** — unchanged.
- **Full-page pixel-parity remainder** (Pass 1.1) — still owed.
- Carried Pass-4 residuals and the `/Info`-not-certification-gated item
  remain open (see continuation 20).

**Same-day continuation 25 — Pass 6.2 SHIPPED (text-bearing annotations +
§12.7.3.3 variable-text appearance generation — COMPLETES the decision-008
6.x annotation arc):**

**Shipped:**
- **Pass 6.2 — Text-bearing annotations + §12.7.3.3 variable-text
  appearance generation.** Moved to `ROADMAP.md` Shipped (full record
  there; top of Shipped, above Pass 6.1). Adds the text-bearing subtypes
  Pass 6.1 deferred — **FreeText, Text (sticky note), Stamp** — plus the
  shared §12.7.3.3 variable-text pipeline. **This COMPLETES the
  decision-008 6.x arc: 6.0 (display) → 6.1 (geometry) → 6.2 (text), all
  SHIPPED.** The `vartext.rs` pipeline is the appearance half **Pass 7
  (Forms) REUSES** for widget-field appearances (R49). R43/R44/R47
  honored: every authored appearance is a real baked `/AP` `/N` shown by
  the SAME Pass-6.0 read path.
- New surface: `pdfce-core` **`vartext.rs`** (the §12.7.3.3 variable-text
  pipeline — `/DA` parsing, the auto-font-size `0` rule, field-value →
  appearance-stream layout with line breaking / `/Q` quadding / baseline
  placement; the shared FreeText + widget generator Pass 7 reuses).
  Modified: `writer/content.rs` (`ContentBuilder` + text/marked-content/
  clip/matrix operators BT/ET/Tf/Td/TD/TL/Tj/Tc/Tw/Tz/q/Q/BMC/EMC/W/cm +
  `emit_literal_string` — **PURELY ADDITIVE**; the R46 re-emission path
  `reemit_canonical`/`emit_token_canonical`/`number_divergence_reason`/
  `emit_number` NOT touched); `annot_author.rs` (`TextAnnotSpec`
  FreeText/Sticky/Stamp, `StickyIcon`, `StampName`, `AuthoredTextAnnot`,
  `build_text_annotation`, R44 icon/stamp look); `edit.rs`
  (`EditSession::add_text_annotation` + X10/X11 inherited-conservative
  guards + R45 staging + X7 `/Annots` multi-append + one-command undo
  incl. the `/Popup` companion; `AnnotKind::{FreeText,Text,Stamp}`;
  `EditError::VariableText`). **CLI** `annotate --type freetext|text|stamp`
  (`--text`/`--font`/`--size`/`--quad`/`--multiline`/`--icon`/
  `--stamp-name`). **GUI** "Text ▾" menu + modeless text-entry popup.

**Findings + decisions:**
- **R44 TEXT round-trip verified WITH GLYPH PIXELS (headline):**
  `authored_freetext_paints_glyph_pixels_after_reload_r44` — author
  FreeText → save incremental → reload fresh `Document` → render via the
  Pass 6.0 read path → **>100 dark glyph pixels, `annotations_painted=1`**.
  Demo: `annots_painted=3 / forms=3`, substituted 35→55 = **20 authored
  glyphs** via bundled Foxit substitutes. The text analogue of Pass 6.1's
  baked-`/AP` proof.
- **`/Q` alignment measured vs AFM widths**
  (`quadding_places_lines_by_afm_width`): "AV" Helvetica = 13.34 pt; the
  `Tm` x-origin matches left / centre / right exactly.
- **Bare standard-14 font dict renders with NO embedded program**
  (`bare_standard14_font_dict_renders_with_no_embedded_program`): no
  `/FontDescriptor`, no `/Widths`. Modality choice — authored the bare
  form, reader-shall-honour §9.6.2.1, PDF-1.5 should-embed deprecation
  noted; `+/Encoding /WinAnsiEncoding` for deterministic Latin
  byte→glyph.
- **Auto-size VT1 heuristic (no spec formula):**
  `auto_size(rect_h) = ((rect_h − 2·PAD) / 1.15).clamp(4.0, 12.0)`,
  `PAD = 2`, line-factor 1.15 — reviewable, every appearance reports
  `applied_autosize`, never presented as spec-mandated (S-class spec
  silence → counted heuristic).
- **Deviations (recorded as decisions):** (1) text specs in a SEPARATE
  `TextAnnotSpec` enum, NOT folded into `MarkupSpec` — keeps the
  R46/R34-proven geometric `add_markup` path + its exhaustive match arms
  byte-unchanged (text needs `/DA`, popup, `/NoZoom`/`/NoRotate`); (2)
  FreeText font dict is 4-key (adds `/Encoding /WinAnsiEncoding`), not
  literally 3-key — deterministic Latin byte→glyph for the glyph-pixel
  proof + `/Q` measurement; still program-free (the gate's real meaning);
  (3) CLI `--fill` doubles as the optional FreeText border colour; (4)
  `/M` // `/CreationDate` still omitted (inherited 6.1 residual — clock
  non-determinism breaks byte-compare).
- **Residuals (named):** Base-14 **LATIN ONLY** (no complex-script / RTL
  shaping — R17; non-WinAnsi chars → "?" counted as `unencodable_chars`);
  `/RC` rich text recognition-only (VT3 non-goal); no comb fields (Pass 7;
  comb = field-flag bit 25 = 16777216); X11 certification gating still
  conservative (over-refuses `/P 3` — scoped fix = check
  `certification_permission == Some(3)` for annotation-adding, §12.8
  already sourced); X10 encryption refusal still the load-time R37 seam.
  GUI: full in-canvas text editing + the sticky-note marker's exact
  artwork join the already-named Pass-6.1-followup GUI slice — NOT built
  here.
- **ARCHITECTURE §12 continuation-25 entry** appended (arc complete,
  `vartext.rs` pipeline landing, the bare-Base-14 modality choice, the
  auto-size heuristic).

**Full-corpus R46 — measurement correction + GATE PASS:** the engineer's
completion report ran the R46 content-identity gate over synthetic
fixtures only (46/46 byte-identical) plus proof-by-inspection that the
re-emission path is additive-only-untouched, having reported the
conformance corpus "not present on this machine." **That was a
path-resolution miss — the corpus IS present (3,020 files under
`fixtures/external` — veraPDF-corpus + pdf20examples).** The orchestrator
RE-RAN the full content-identity gate over the corpus: **GATE PASS —
every content stream semantically preserved, zero corruptions**; all
divergences are value-preserving number re-spellings, the same class
Pass 6.1 enumerated (`-0`→`0` majority, `.050003`→`0.050003` leading-zero
on the Isartor file, `-.001`→`-0.001`, one 300-digit pathological real
value-preserved within f64). This CONFIRMS Pass 6.2's additive-only claim
BY MEASUREMENT; the earlier fixture-only run is superseded. **For the
record: the corpus IS present and runnable at `fixtures/external` —
future Passes must NOT accept a "corpus absent" caveat without checking
`fixtures/external` first.**

**Gates:**
- **971 workspace tests green (was 939)**; `cargo fmt --check` /
  `clippy -D warnings` clean; GUI-free core+render invariant verified
  (zero egui/eframe/winit/wgpu); wasm32; `--duplicates`; `ui-strings`;
  `no-network` all clean; fuzz `annot_author` extended (`/DA` parsing +
  text-appearance gen: malformed `/DA`, unresolvable font, symbolic font,
  huge text, size 0) — **13,871 runs / 61 s, 0 crashes**; **no new
  §6.1.12 guards**.
- **ZERO new dependencies** — Base-14 only, **NO `harfrust`** (R17
  upheld; the text-authoring path reserved by decision 004 for a future
  `harfrust` built WITHOUT it); `THIRD_PARTY_LICENSES.md` unchanged.

**RAG escalations this continuation** (`C:\personal_rag\pdf\`):
- `lesson_20260801_bare_base14_font_dict_renders_no_embedded_program.md`
  — **NEW — format-spec / MEDIUM** — a Base-14 font dict authored with
  NO `/FontDescriptor` and NO `/Widths` (§9.6.2.1 reader-shall-supply
  metrics) renders correctly; the PDF-1.5 should-embed deprecation is a
  should, not a shall. Add `/Encoding /WinAnsiEncoding` for deterministic
  Latin byte→glyph (the 4-key vs 3-key deviation) when you need to assert
  glyph pixels / measure `/Q`. pdfce `vartext.rs` / `annot_author.rs`.
- `lesson_20260801_variable_text_autosize_is_implementation_defined.md`
  — **NEW — methodology / MEDIUM** — §12.7.3.3's auto-font-size (`/DA`
  size `0`) has NO spec formula (S-class spec silence); pick a reviewable
  heuristic, COUNT it, and surface it (never present it as spec-mandated).
  pdfce: `auto_size(rect_h) = ((rect_h − 2·PAD)/1.15).clamp(4.0,12.0)`,
  every appearance reports `applied_autosize`. General pattern for
  spec-silent layout parameters. pdfce `vartext.rs`.
- Subject + master indexes updated.

**Still in flight (continuation 25):**
- **Pass 7 (Forms / AcroForm)** — promoted to In progress; **blocked on
  TWO prerequisites both DISPATCHED in parallel**: the §12.7.1–12.7.4
  form-field spec (`pdfce-spec-librarian`) and the "Forms (AcroForm)"
  acrobat parity bucket (`pdfce-acrobat-librarian`). Open Pass-7
  sub-decision: the embedded-JavaScript posture (recommend never-execute —
  recognize + disclose; a SECURITY decision needing its own
  decision-008-continuation record at scoping time).
- **Pass-6.1-followup GUI slice** — now also carries Pass 6.2's GUI
  residuals (full in-canvas text editing + the sticky-note marker
  artwork); core authoring path complete, promotable independently.
- **Pass 5 (Encryption)** — spec-unblocked, queue-deferred behind Pass 7
  (decision 008); `/R 6` open sub-decision + two operator decisions gate
  scoping. Unchanged.

**For next session / STILL-OPEN operator items (ordered by age —
oldest first, unchanged from continuation 24):**
- **Encryption-refusal operator sign-off** — still the oldest owed
  operator item.
- **LEGAL.md §2 Adobe-supplement copyright contradiction** — flagged
  for Ken; LEGAL.md NOT edited.
- **`/R 6` sourcing method** — cross-implementation triangulation vs a
  purchased ISO 32000-2 copy. Ken's call.
- **License decision (`LEGAL.md` §1)** — still undecided.
- **Commit authorization** — everything remains UNCOMMITTED in git
  (Passes 0–6.2 ALL uncommitted).
- **W15 — no remote/CI** — unchanged.
- **Full-page pixel-parity remainder** (Pass 1.1) — still owed.
- Carried Pass-4 residuals and the `/Info`-not-certification-gated item
  remain open (see continuation 20).

**Same-day continuation 27 — Pass 7.1 SHIPPED (form flatten + FDF/XFDF +
choice fields + regenerate-all — COMPLETES the AcroForm subsystem CORE):**

Pass 7.1 finishes the residuals Pass 7.0 named. With 7.0 (field model +
text/checkbox fill) and 7.1 (flatten + data interchange + choice fields +
regenerate-all), **the AcroForm subsystem CORE is done** — the remaining
forms items (GUI form-fill slice, field auto-detection, posture-B native
recompute) are FOLLOW-UP SLICES, not core.

**Shipped:**
- **Pass 7.1 — Form flatten + FDF/XFDF + choice fields + regenerate-all.**
  Moved to `ROADMAP.md` Shipped (full record there; top of Shipped, above
  Pass 7.0).
- **New `pdfce-core` `fdf.rs` (~700 lines)** — FDF (§12.7.7) + XFDF
  import/export. The FDF reader REUSES `crate::parser::Parser` (FDF is
  PDF-syntax). XFDF uses a **HAND-ROLLED ~200-line scoped XML reader**
  (element/attribute/text, the 5 predefined entities, numeric character
  refs, comments, `<?xml?>`/DOCTYPE skip, MAX_XML_DEPTH-guarded) — **ZERO
  new dependencies** (rule 13; the brief's preference — no `quick-xml`/
  `roxmltree` for a reader this small and scoped).
- **`edit.rs`** — `set_choice_value`, `regenerate_appearances`,
  `flatten_fields`, `export_form_data`/`import_form_data`,
  `regen_field_appearance`, flatten helpers (`burn_target`,
  `page_of_widget`, `append_page_content`, `add_page_xobjects`,
  `effective_resources`, `remove_from_annots`, `remove_fields_from_form`,
  `clear_need_appearances_write`), the §12.5.5 `fit_matrix_for`,
  `match_option`, `choice_display_text`; `RegenOutcome`/`ImportOutcome`/
  `FlattenOutcome` (+9 tests).
- **`forms.rs`** — `scan_javascript` + the `FormJavaScript` histogram
  (decision 009 posture A, recognition-only) (+2 tests).
- **`writer/content.rs`** — `ContentBuilder::invoke_xobject` (`/Name Do`).
- **CLI** — `regenerate-appearances` / `flatten` / `export-data` /
  `import-data` subcommands + choice routing + `|`-multi-select in
  `fill-field` + the JS histogram on `list-fields`.
- **ARCHITECTURE §5.8 + §12 continuation-27 entries** appended (the
  flatten overlay-append design + the AcroForm-core-complete milestone).

**Key design win — FLATTEN is APPEND-not-rewrite (adopted design,
supersedes the brief's in-place-rewrite anticipation):** flatten appends a
NEW overlay content stream to the page `/Contents` array and invokes the
widget's existing `/AP` `/N` as a page XObject
(`ContentBuilder::invoke_xobject`), rather than rewriting the existing page
content stream. Consequence: **existing content streams stay byte-verbatim,
so the R46 re-emit-everything gate has ZERO flattened-page exceptions**
(GATE PASS over `fixtures/synthetic` + `fixtures/external`; all divergences
the known value-preserving `-0`→`0` number re-spellings, 0 corruptions).
This is MORE minimal-diff than the in-place content-stream surgery the
scope anticipated — recorded as a general pattern (overlay-append beats
content-stream-surgery for additive burn-in). **R48 verified:** incremental
flatten leaves the field dict recoverable in the prior revision;
`--full-rewrite` output has no `/FT`/`/Tx` yet renders the burned value.
Flatten uses the STRICT cert gate (refused on `/P 2` certified, by test —
correct: flatten is STRUCTURAL, distinct from the fill path's `/P >= 2`
permit).

**Choice-field matrix (recorded):** single-select combo → `/V` = EXPORT
value, `/I=[idx]`, appearance shows DISPLAY value; multi-select list →
`/V` array + `/I` array; single-value-given-multiselect-required →
`ChoiceRequiresMultiSelect` refusal; unknown-value non-editable →
`ChoiceValueNotInOptions`; editable combo (`Combo|Edit`) accepts free text,
no `/I`. FDF/XFDF round-trip: fill → export FDF+XFDF → re-import →
identical `/V` + regenerated appearances; import SKIPS fields the doc lacks
(counted, never an error).

**Gates:**
- **1,010 workspace tests** (core lib 620, was 601); `cargo fmt --check` /
  `clippy -D warnings` clean; GUI-free `cargo tree` core+render (zero
  egui/eframe/winit/wgpu); wasm32; `--duplicates`; `no-network` clean;
  `ui-strings` N/A (no GUI changes); **R34** (Pass 3.0 identity) green +
  **R46** re-emit-everything GATE PASS (Pass 7.1 additive — flatten
  appends, never rewrites); fuzz **target 14 `fdf_parse`** 624,202 runs /
  61 s, 0 crashes (malformed FDF/XFDF, huge arrays, entity edges).
- **veraPDF §6.1.12 N/A** — MAX_XML_DEPTH is XFDF-only, outside PDF-
  conformance scope. **ZERO new dependencies**; `THIRD_PARTY_LICENSES.md`
  unchanged.

**Gotcha found + fixed (RAG-escalated to `D:\dev\rag\rust\`):** adding the
CLI subcommands overflowed the DEBUG `pdfce-cli` main-thread stack on
Windows (clap's `debug_assert` recursion vs the MSVC ~1 MB main-thread
stack), surfacing as `TryFromIntError(NegOverflow)` in CLI integration
tests. Fixed by running `main()` on a 16 MB worker thread. The engineer
had noted it to agent memory (`reference_clap_windows_stack.md`) — promoted
to `D:\dev\rag\rust\` + index this continuation.

**Deviations:** (1) flatten APPEND-not-rewrite (a POSITIVE deviation —
more minimal-diff than scoped); (2) JS histogram is posture-A ONLY
(`custom_scripts` counts all field-level JS actions, no whitelist recompute
— posture B is Pass 7.x per decision 009), surfaced on `list-fields` + a
loud stderr flag when network/launch `/AA` actions are present.

**Residuals (named):** list-box multi-select appearance is a simplified
display-text newline-join, NOT the §12.7.4.4 highlight-rectangle rendering
(named simplification); corpus flatten-burn coverage is thin (sampled
external forms were certified / pushbutton / no-`/AP` per the 6.0 census —
synthetic fixtures + unit tests carry the burn path); import applies as
per-field commands (each undoable), NOT one atomic `ImportFormData`
command. STILL OPEN from Pass 7 as the forms FOLLOW-UP slices (NOT core):
GUI form-fill (`docs/ui_specs/pass-7-form-fill.md`), field auto-detection
("Prepare Form", fuzzy-never-sneaky HINT), posture-B native recompute
(decision 009), X10/encryption, full-page pixel-parity.

**Demo:** CLI `fill` (text + checkbox) → `export-data` FDF+XFDF → re-import
round-trip → `flatten` (`fields_flattened=1 widgets_burned=1`) →
`list-fields` shows the field gone → `render-page` renders the burned
appearance (`forms=1`) → `--full-rewrite` removes it. GUI launched
(PID 42444) on the flattened file.

**RAG escalations this continuation:**
- (`C:\personal_rag\pdf\`)
  `lesson_20260801_flatten_overlay_append_beats_content_stream_surgery.md`
  — **NEW — quirk / MEDIUM** — burning a widget appearance into page
  content by APPENDING a new overlay content stream + `Do`-invoking the
  existing `/AP` `/N` as a page XObject keeps every existing content stream
  byte-verbatim (R46 GATE PASS, zero flattened-page exceptions), and is
  strictly more minimal-diff than rewriting the page content stream in
  place. General pattern: overlay-append beats content-stream-surgery when
  the goal is additive burn-in; reserve in-place surgery for true removal
  (redaction). pdfce Pass 7.1 `edit.rs::flatten_fields` /
  `ContentBuilder::invoke_xobject`. Subject + master indexes updated.
- (`D:\dev\rag\rust\`)
  `clap_windows_debug_stack_overflow.md` — clap 4's `debug_assert`
  command-tree recursion overflows the MSVC ~1 MB main-thread stack in
  DEBUG builds as subcommand count grows, surfacing as an opaque
  `TryFromIntError(NegOverflow)` in tests; run `main()` on a large
  (16 MB) worker thread. Subdir index updated.

**Still in flight (continuation 27):**
- **Pass 8 (Redaction)** — promoted to In progress (the standing R35
  obligation; the one truly destructive op). Blocked on two prerequisites
  both DISPATCHED in parallel: the Redaction acrobat-parity bucket
  (`pdfce-acrobat-librarian`) + a redaction spec dispatch for
  container-decomposition + `/Redact`-apply semantics
  (`pdfce-spec-librarian`). Redaction is the OPPOSITE discipline from
  7.1's flatten — it DOES rewrite existing page content (R46's named
  exception) AND must decompose object-stream containers (§5.7).
- **AcroForm CORE COMPLETE** — 7.0 (model+fill) + 7.1 (flatten+data+choice
  +regenerate); forms GUI + auto-detection + posture-B are follow-up
  slices tracked in Backlog.
- **Pass-6.1-followup GUI slice** — unchanged (full in-canvas text editing
  + sticky-note marker artwork); promotable independently.
- **Pass 5 (Encryption)** — spec-unblocked, queue-deferred behind Pass 7
  (decision 008); now behind Pass 8 on the fallback/interleave track.
  `/R 6` open sub-decision + two operator decisions gate scoping.

**For next session / STILL-OPEN operator items (ordered by age —
oldest first, unchanged from continuation 26 except the commit-scope
line):**
- **Encryption-refusal operator sign-off** — still the oldest owed
  operator item.
- **LEGAL.md §2 Adobe-supplement copyright contradiction** — flagged for
  Ken; LEGAL.md NOT edited.
- **`/R 6` sourcing method** — cross-implementation triangulation vs a
  purchased ISO 32000-2 copy. Ken's call.
- **License decision (`LEGAL.md` §1)** — still undecided.
- **Commit authorization** — everything remains UNCOMMITTED in git
  (**Passes 0–7.1 ALL uncommitted**).
- **W15 — no remote/CI** — unchanged.
- **Full-page pixel-parity remainder** (Pass 1.1) — still owed.
- Carried Pass-4 residuals and the `/Info`-not-certification-gated item
  remain open (see continuation 20).

**Same-day continuation 26 — Pass 7.0 SHIPPED (AcroForm field model +
text/checkbox fill — the forms FOUNDATIONAL SLICE, NOT all of Forms) +
decision 009 (embedded form/document JavaScript posture) FILED:**

Pass 7 was **split on ship.** The engineer delivered the field-model read
path plus the dominant fill path and honestly named the rest as
residuals; those residuals are filed as **Pass 7.1 (In progress) —
"completes the forms subsystem."** This is NOT "Forms shipped."

**Shipped:**
- **Pass 7.0 — AcroForm field model + text/checkbox fill.** Moved to
  `ROADMAP.md` Shipped (full record there; top of Shipped, above Pass
  6.2). The `/AcroForm` field-model parser + the field↔widget merge + the
  `/P`-aware fill gate + text/checkbox fill through the SAME §12.7.3.3
  appearance generator Pass 6.2 built (R49 — one appearance pipeline for
  widgets and annotations alike).
- **New `pdfce-core` `forms.rs` (~1,050 lines, 13 model tests).**
  `parse_acroform(graph)` walks `/AcroForm` → `/Fields` DFS, resolving
  §12.7.3.1 inheritance of `/FT`//`/V`//`/DV`//`/Ff`//`/DA`//`/Q` down
  `/Kids` via `/Parent`, building the dotted fully-qualified field name
  (§12.7.3.2). The **field-vs-widget MERGE** (R49): *Shape A* single
  merged dict (~88% of real fields), *Shape B* field + `/Kids` widget
  array. `FieldFlags` verbatim bits pinned by test (Radio 32768 /
  Pushbutton 65536 / NoToggleToOff 16384 / RadiosInUnison 33554432;
  Multiline 4096 / Comb 16777216; Combo 131072 / MultiSelect 2097152).
  Per-type `/V` decode; `/Opt` export/display pairing; `/I`//`/TI`; XFA
  **detect-only** (`XfaPresence`); `/SigFlags`; `/CO` count. Generic over
  `ObjectGraph` (loaded `Document` + `EditSession`). Cycle-guarded
  (visited set + `MAX_FIELD_TREE_DEPTH = 64`), bounded (`MAX_FORM_FIELDS =
  500_000`).
- **Fill (`edit.rs`, 6 fill tests):** `fill_text_field` sets `/V` and
  regenerates `/AP` for every widget via the shared §12.7.3.3 generator
  (R49 reuses Pass 6.2 `vartext.rs`, wrapped by
  `annot_author::build_field_text_appearance`; `/DA` font from `/DR` via
  `basefont_to_std14`). `set_button_state` selects checkbox/radio `/V` +
  `/AS` with no regen (RadiosInUnison, `/Off` convention). One command
  per fill (undo inherited); encryption + `/Size` guards inherited.
- **`/P`-aware certification gate** (`check_certification_for_fill`, built
  per the orchestrator's mid-Pass correction): permits fill at `/DocMDP`
  `/P >= 2` (incl. absent = 2), refuses BY NAME at `/P 1`, refuses on any
  `/FieldMDP` — structural gate stays STRICT. Proven by
  `certification_p2_permits_fill_p1_refuses`. This is the per-`/P`
  refinement the 6.1/6.2 X11 residual scoped, applied to the fill path.
- **Decision 009 honored structurally:** fill touches only `/V`//`/AP`//
  `/AS`, never the AcroForm dict — so `/CO`//`/AA`//`/Names /JavaScript`
  re-emit byte-verbatim. `has_additional_actions` + `calc_order_count`
  surfaced recognition-only (the full JS-disclosure histogram → Pass 7.1
  per decision 009 posture A).
- **CLI** `list-fields` + `fill-field --set Name=value` (text +
  checkbox/radio; auto-size disclosed).
- **ARCHITECTURE §12 continuation-26 entry** appended (Pass 7.0 + decision
  009, with the field-vs-widget merge, the fill gate, and the hollow-shall
  finding).

**R44 form-fill round-trip with GLYPH PIXELS (HEADLINE):** the demo
authors a text fill + a checkbox on-state → save incremental
(`undo_identical=1`, minimal-diff holds) → reload fresh `Document` →
`render-page` **paints 11 real glyphs**, `annots_painted=2 forms=2`. An
authored field value is real glyph pixels through the SAME Pass-6.0 read
path — never a private "what I just filled" render.

**Decision 009 (embedded form/document JavaScript) — FILED this
continuation.** Archived at `docs/decisions/009-forms-javascript-posture.md`;
discharges the decision-008 §5.1 embedded-JS scope trap and the Pass 6.2
open sub-decision. **Outcome: NEVER execute embedded PDF JavaScript** —
field `/AA`, document `/AA`, `/OpenAction`, `/Names /JavaScript`,
built-in or custom, on load or interaction. Fully ISO-conformant:
**§12.6.4.16 is a "hollow shall"** — it mandates execution but defines no
JS semantics/API/DOM/security model (deferring to two non-ISO external
docs), specifying only the carrier (Table 217) + hook points; there is no
normative JS behavior to conform to, so non-execution forfeits nothing.
Phased hybrid: **posture A** (recognize + classify + disclose + byte-exact
round-trip, zero execution) = the mandatory floor + Pass 7's whole JS
scope; **posture B** (native recompute of an exact-match whitelist —
`AFSimple_Calculate` SUM/AVG/PRD/MIN/MAX changes `/V`, `AF*_Format`
changes display only, opt-in / off-by-default, every recompute a
reviewable/undoable `EditSession` edit leaving the source script in place)
= deferred demand-driven Pass 7.x; **posture C** (a sandboxed JS engine) =
REJECTED + prohibited (re-imports the attack surface Adobe's broker
contains; hook points reference `/URI`//`/SubmitForm`//`/ImportData`//
`/Launch` — R12/R13 forbid; nothing to conform to).
- **Standing rules R53–R57 added** (decision 009's R-JS-1…R-JS-5,
  renumbered to the next-free slot after R52 — verified against the
  R1–R52 list): R53 never-execute (+ C prohibited); R54
  no-trigger-ever-fires (R51 sibling, enforced by R12/R13); R55
  JS-carriers-byte-preserved-never-stripped-never-baked; R56
  recognize+disclose / recompute opt-in+whitelisted+fuzzy-never-sneaky+
  leaves-source-script-in-place; R57 recompute-changing-`/V` is
  DocMDP/FieldMDP-gated.
- **Spec prerequisites (queued Pass 7.x, non-blocking):** verify §12.6
  carrier/hook coverage + formalize the hollow-shall finding
  (`pdfce-spec-librarian`); source the `AF*` canonical shapes
  (`pdfce-acrobat-librarian`); confirm PDF/A forbids JS actions.

**Full-corpus R46 — GATE PASS (post-7.0 re-run, residual DISCHARGED):**
the orchestrator re-ran the R46 content-identity gate over `fixtures/
external` (3,020 files) post-7.0: **every content stream semantically
preserved, zero corruptions**, identical to the post-6.2 result (same
value-preserving number-respelling divergences). Additivity confirmed BY
MEASUREMENT — fill authors NEW `/AP` streams via the proven §12.7.3.3
generator; the re-emission path (`reemit_canonical` /
`emit_token_canonical` / `number_divergence_reason` / `emit_number`) is
byte-unchanged. This DISCHARGES the "full-corpus R46 re-run owed" residual
from the engineer's report. **R34 (Pass 3.0 roundtrip) accepted as
additivity-preserved** (fill authors new streams, never re-serializes
untouched objects) — not separately re-run this Pass.

**Gates:**
- **601 `pdfce-core` lib tests green (was 582; +13 model +6 fill)** +
  integration green; `cargo fmt --check` / `clippy -D warnings` clean
  (core + cli); GUI-free `cargo tree` invariant verified core + render
  (zero egui/eframe/winit/wgpu); fuzz **target 13 `form_model`**
  1,306,476 runs / 61 s, 0 crashes (cyclic `/Parent`//`/Kids`, huge
  `/Kids`, merge-shape edges, malformed values); real-corpus
  `list-fields` clean on all `/AcroForm` files (pushbutton `flags =
  0x10000`, no panics).
- **veraPDF §6.1.12:** the two new guards `MAX_FORM_FIELDS` /
  `MAX_FIELD_TREE_DEPTH = 64` are pure memory backstops (corpus max ≈ 63
  fields/file).
- **ZERO new dependencies**; `THIRD_PARTY_LICENSES.md` unchanged. R34/R46
  preserved BY ADDITIVITY (new module + additive methods/variants + one
  new `pub fn`; re-emission path + `add_markup`/`add_text_annotation`
  byte-unchanged).

**Demo:** `fixtures/synthetic/forms/demo-form.pdf` (+ `PROVENANCE.md` +
`tools/gen-form-fixtures.py`): `list-fields` → `fill-field --set
FullName="Ada Lovelace" --set Subscribe=on` (auto-size disclosed) →
incremental save `undo_identical=1` (minimal-diff) → reload shows "Ada
Lovelace" + "Yes" with regenerated `/AP` → `render-page` paints 11 glyphs,
`annots_painted=2 forms=2`.

**RAG escalation this continuation** (`C:\personal_rag\pdf\`):
- `lesson_20260801_field_widget_merge_shape_a_vs_b.md` — **NEW — quirk /
  MEDIUM** — a terminal AcroForm field with a single widget MERGES field
  dict + widget dict into ONE dictionary (§12.7.3.3 / §12.5.6.19); this is
  the ~88% common case. A field with multiple widgets keeps a `/Kids`
  array of separate widget annotations (Shape B). A reader that ALWAYS
  expects `/Kids` widgets breaks on the Shape-A common case. pdfce
  `forms.rs::parse_acroform`. Subject + master indexes updated.

**Still in flight (continuation 26):**
- **Pass 7.1 (Completes the forms subsystem)** — promoted to In progress.
  regenerate-all + clear `/NeedAppearances` (R51); **Flatten** (destructive
  R48 — the FIRST controlled modification of EXISTING page content,
  page-content-stream APPEND + byte-grep test, distinct from 6.1's
  new-stream-only authoring); FDF/XFDF import/export (XFDF needs a minimal
  XML reader — classify per rule 13); choice-field multi-select (array
  `/V`, `/I` maintenance, `/Opt` display↔export); the JS-disclosure
  histogram (decision 009 posture A); GUI form-fill
  (`docs/ui_specs/pass-7-form-fill.md` P0). Field auto-detection ("Prepare
  Form") and posture-B native recompute (Pass 7.x) are explicitly OUT of
  7.1.
- **Pass-6.1-followup GUI slice** — unchanged (full in-canvas text editing
  + sticky-note marker artwork); promotable independently.
- **Pass 5 (Encryption)** — spec-unblocked, queue-deferred behind Pass 7
  (decision 008); `/R 6` open sub-decision + two operator decisions gate
  scoping. Unchanged.

**For next session / STILL-OPEN operator items (ordered by age —
oldest first, unchanged from continuation 25 except the commit-scope
line):**
- **Encryption-refusal operator sign-off** — still the oldest owed
  operator item.
- **LEGAL.md §2 Adobe-supplement copyright contradiction** — flagged for
  Ken; LEGAL.md NOT edited.
- **`/R 6` sourcing method** — cross-implementation triangulation vs a
  purchased ISO 32000-2 copy. Ken's call.
- **License decision (`LEGAL.md` §1)** — still undecided.
- **Commit authorization** — everything remains UNCOMMITTED in git
  (**Passes 0–7.0 ALL uncommitted**).
- **W15 — no remote/CI** — unchanged.
- **Full-page pixel-parity remainder** (Pass 1.1) — still owed.
- Carried Pass-4 residuals and the `/Info`-not-certification-gated item
  remain open (see continuation 20).

**Same-day continuation 28 — Pass 8.0 SHIPPED (Redaction — mark + apply,
text + region: the highest-stakes Pass, and the cardinal rule held —
never claim redacted what isn't):**

Pass 8.0 discharges the standing **R35** obligation and is the ONE
operation whose contract is genuine REMOVAL (§5's sole deliberate
exception; R46's one named content-stream-surgery exception). Redaction
MARK and APPLY are separate operator actions (R52): a mark is a reviewable,
reversible `/Redact` annotation drawn as a RED OUTLINE (never a solid
fill); apply is the destructive act that proves the covered bytes are GONE
from the entire saved file. This is the MIRROR IMAGE of Pass 7.1's flatten
— flatten ADDS by overlay-append and never touches page content; redaction
REMOVES and is the one op that DOES rewrite existing page content.

**Shipped:**
- **Pass 8.0 — Redaction (mark + apply, text + region).** Moved to
  `ROADMAP.md` Shipped (full record there; top of Shipped, above Pass 7.1).
- **New `pdfce-core` `redact.rs`** — the self-contained advance-preserving
  content-stream surgery interpreter + apply orchestration + carrier sweep
  + container decomposition + `RedactionReport` + `count_redaction_marks`.
- **`edit.rs`** — `add_redaction`, `mark_redactions_by_search`/`_by_pattern`,
  `find_matches`/`find_pattern_matches`.
- **`annot_author.rs`** — `RedactSpec` + `build_redact_mark` (RED-OUTLINE
  preview, never solid fill — the mark-vs-apply rule made visible).
- **`text_extract/font.rs`** — exposed `codes`/`width`/`to_unicode`/
  `bytes_per_code`/`width_estimated`/`base_font_name` as `pub(crate)` (the
  surgery interpreter needs per-code widths for the exact advance).
- **CLI** — `redact-mark` (`--rect`/`--search`/`--pattern`), `redact-apply`
  (report + `--acknowledge-residuals`; **exit 10 `REDACTION_RESIDUALS`**),
  `list-redactions`.
- **GUI — the ONE non-negotiable item:** a persistent status-bar disclosure
  of unapplied `/Redact` marks (computed from the document's own
  annotations), targeting the #1 real-world redaction failure — saving a
  marked-but-not-applied document believing it is redacted.
- **Fuzz target 15 `redact_apply`**; `tools/gen-redact-fixtures.py` +
  `fixtures/synthetic/redact/`.
- **ARCHITECTURE §5.9 + §12 continuation-28 entries** appended (the
  removal/scrub-forces-full-rewrite rule generalizing R35; the redaction
  landing).

**THE HEADLINE — ABSENCE PROOF PASSES (R46 INVERTED):** demo on
`demo-secret.pdf` ("SECRET" in heading + body, "PUBLIC" surrounding,
`/Info /Title (SECRET dossier)`): `redact-mark --search SECRET` → **3
marks, doc NOT yet redacted**; `redact-apply` → `glyphs_removed=21
info_strings_scrubbed=1`; `grep "SECRET" redacted.pdf` → **0** (control
`marked.pdf` → 3). ZERO occurrences in the ENTIRE saved file — raw bytes
AND every decoded content stream. R46 inverted: absence proven for
redacted content, presence preserved for everything else.

**The proofs:**
1. **Advance preservation.** Redacted-page render: "SECRET" is a baked
   black box while "dossier"/"PUBLIC text" sit EXACTLY where they were (not
   shifted left) — proven visually AND numerically (survivor x moved
   <1.0 pt); the removed run is replaced by a `TJ` advance
   `N = −Σtx·1000/(Tfs·Th)`.
2. **Container decomposition (§7.5.7 Strategy B).** A redacted `/Info`
   compressed in an `/ObjStm` would survive verbatim without it (§5.7); the
   test proves absence AND `containers_decomposed >= 1`.
3. **Forced full rewrite (R35).** Output has no `/Prev`; prior revisions
   (holding un-redacted content) dropped; every carrier scrub rides
   `save_full`.
4. **Image handling — REFUSE by name** (`RedactError::ImageRegion`, NO
   output written) — never overlay-and-leave-pixels.
5. **Carrier sweep/report.** `/Info`+XMP SCRUBBED (asserted absent);
   object-streams+prior-revisions DROPPED-BY-REWRITE; OCG
   REDACTED-BY-GEOMETRY (ignores `/OC` visibility); XFA/structure-tree/
   attachments DETECTED+DISCLOSED (`DISCLOSED_NOT_SCRUBBED`), gated by
   `--acknowledge-residuals` (exit 10 otherwise — ui-spec §4.4).

**NEW STANDING RULE R58 (the ui-specialist finding — generalizes R35):**
every removal/scrub operation must ride R35's forced FULL REWRITE —
including any future Sanitize / Remove-Hidden-Information — because an
incremental save leaves the "removed" content recoverable in the prior
revision, defeating removal. R35 covered redaction-apply; R58 generalizes
it to ALL scrub ops. Recorded in ROADMAP Standing rules + `ARCHITECTURE.md`
§5.9 + the ui-spec that surfaced it.

**Gates:**
- **1,018 workspace tests (+8)**; `cargo fmt --check` / `clippy -D warnings`
  clean workspace-wide; GUI-free core+render (zero egui/eframe/winit/wgpu);
  wasm32; `--duplicates`; `no-network`; `ui-strings` all clean; **R34/R46
  additive-preserved** (`writer/` + `content.rs` re-emission paths + gates
  byte-unchanged — surgery is a NEW code path, identity path does not move);
  fuzz **target 15 `redact_apply`** 9,262 runs / 61 s, 0 crashes (multi-byte
  CID, nested q/Q, overlapping/degenerate quads, all/none covered — the
  security assert held).
- **ZERO new dependencies**; `THIRD_PARTY_LICENSES.md` unchanged. GUI
  launched PID 40828.

**Deviations/residuals (ALL disclosed, none silent):** image pixels
refuse-not-clear (named, safe, disclosed); `/RO`+`/OverlayText` burn-in
deferred — apply draws the `/IC`/default-black fill (Acrobat default),
overlay-text LABEL not drawn (COSMETIC only, content removed regardless,
disclosed at mark time); form-XObject content in-region NOT surgically
redacted — disclosed loudly (`form_intersect` note), never claimed
removed; XFA/structure-tree `/ActualText`/attachments detect+disclose not
scrubbed this cut; GUI apply-button + canvas marking DEFERRED to the named
GUI follow-up (it depends on the Pass 6.1 canvas tool-mode that never
shipped — the engineer correctly did NOT build a parallel drag tool).

**Demo:** `redact-mark --search SECRET` on `demo-secret.pdf` → 3 marks (doc
NOT yet redacted) → `redact-apply` → `glyphs_removed=21
info_strings_scrubbed=1 containers_decomposed>=1` → `grep "SECRET"` = 0 →
`list-redactions` on control `marked.pdf` = 3. GUI launched (PID 40828) with
the unapplied-marks status-bar disclosure live.

**RAG escalations this continuation** (`C:\personal_rag\pdf\`):
- `lesson_20260801_redaction_advance_preserving_content_stream_surgery.md`
  — **NEW — format-spec / HIGH** — remove a text run without shifting
  surviving same-line text: delete the `Tj`, substitute a `TJ` offset
  consuming the exact `tx` (`N = −Σtx·1000/(Tfs·Th)`). The
  content-stream-surgery correctness lesson; the mirror image of Pass 7.1's
  overlay-append flatten. pdfce Pass 8.0 `redact.rs`.
- `lesson_20260801_redaction_absence_proof_acceptance_gate.md` — **NEW —
  methodology / HIGH** — redaction's four-shalls embodied as an executable
  gate: grep the WHOLE saved output (raw bytes AND every decoded content
  stream) for the redacted bytes → zero (R46 inverted — R46 proves
  presence for untouched content, the absence test proves deletion for
  redacted content). Control-vs-treatment (`marked.pdf` = 3 /
  `redacted.pdf` = 0). pdfce Pass 8.0.
- `lesson_20260801_redaction_diligence_carriers_checklist.md` — **NEW —
  methodology / HIGH** — the carriers a naive region-redact misses:
  ObjStm survivors (§7.5.7 Strategy B decomposition), prior revisions
  (drop `/Prev` / full rewrite), `/Info`, XMP, XFA, overlapping annots,
  attachments, OCG (redact by geometry, ignore `/OC` visibility),
  StructTree `/ActualText`. Refusing incremental save (R35) is necessary
  but NOT sufficient (§5.7). pdfce Pass 8.0 `redact.rs` carrier sweep.
- Subject + master indexes updated.

**Still in flight (continuation 28):**
- **Decision 010 (post-redaction priority)** — promoted to In progress;
  KenAgent consultation IN FLIGHT (vector/Inkscape editing vs GUI-editing
  consolidation vs render-fidelity verification vs encryption). Record will
  land at `docs/decisions/010-post-redaction-priority.md`.
- **MILESTONE:** read → write → edit → extract → annotations → forms →
  redaction ALL shipped.
- **Accumulated GUI-editing follow-up slices** — canvas markup drawing /
  form-fill / redaction-marking / in-canvas text editing; a candidate for
  decision 010's GUI-editing-consolidation option; promotable independently.
- **Pass 5 (Encryption)** — spec-unblocked, queue-deferred; now a decision
  010 candidate. `/R 6` open sub-decision + two operator decisions gate
  scoping.

**For next session / STILL-OPEN operator items (ordered by age —
oldest first, unchanged from continuation 27 except the commit-scope
line):**
- **Encryption-refusal operator sign-off** — still the oldest owed
  operator item.
- **LEGAL.md §2 Adobe-supplement copyright contradiction** — flagged for
  Ken; LEGAL.md NOT edited.
- **`/R 6` sourcing method** — cross-implementation triangulation vs a
  purchased ISO 32000-2 copy. Ken's call.
- **License decision (`LEGAL.md` §1)** — still undecided.
- **Commit authorization** — everything remains UNCOMMITTED in git
  (**Passes 0–8.0 ALL uncommitted**).
- **W15 — no remote/CI** — unchanged.
- **Full-page pixel-parity remainder** (Pass 1.1) — still owed.
- Carried Pass-4 residuals and the `/Info`-not-certification-gated item
  remain open (see continuation 20).

**Same-day continuation 29 — Decision 010 concluded + archived; the
C→B→A sequence; Pass 11 (render-fidelity verification) DISPATCHED.**

**Decisions made this session:**
- **Decision 010 (post-redaction priority) consulted + archived** at
  `docs/decisions/010-highest-value-investment-after-the-editing-arc.md`
  (KenAgent consultation of continuation 28 RETURNED). **Outcome — the
  DESTINATION is UNCHANGED, the PATH is AMENDED:** vector/Inkscape editing
  (decision 008's candidate E / Pass 9) remains the highest-value major
  investment and pdfce's distinctive purpose. The accumulated GUI-editing +
  render-verification debt amends the PATH into the three-Pass sequence
  **C → B → A** = **Pass 11** (render-fidelity verification) → **Pass 12**
  (canvas-interaction foundation + editing-GUI consolidation) → **Pass 9**
  (vector editing, repositioned onto C+B, keeping its decision-008 Pass ID).
  D = encryption (Pass 5) stays fallback/interleave; E = signatures
  (Pass 10) unchanged-last. *(Decision-010's candidate letters A–E are LOCAL
  and differ from decision 008's A–F — not conflated.)*
- **Decision 010 AMENDS decision 008's revisit-trigger-7** (the clean jump
  to Pass 9 straight after Pass 6.1) into the C→B→A sequence; decision 008's
  ranking and Pass IDs are otherwise intact.
- **Three standing rules added (R59/R60/R61):** R59 render-fidelity gate
  (prove against an INDEPENDENT renderer at corpus scale before any
  subsystem edits content it re-renders; self-comparison proves
  agreement-with-self, not correctness; re-run on every render-touching
  Pass; residual enumerated by file+reason, never a threshold tuned to pass
  — W14); R60 one-canvas-interaction-substrate (exactly one focusable-canvas/
  transform/tool-mode/hit-test/selection/overlay; markup/form-fill/
  redaction/vector all layer on it; a second parallel path forbidden — R49
  applied to interaction); R61 Inkscape-is-behavioral-reference-only
  (GPL-2.0-or-later, never a dependency/code-source/GUI-mimicry;
  `pdfce-inkscape-librarian` catalogs capability/behavior/limits,
  `pdfce-ui-specialist` designs the UI independently).

**Filed this session:**
- **ROADMAP In progress** = **Pass 11 — Render-fidelity verification
  harness** (full scope from decision 010's `first_pass_scope`: generalize
  `tools/annot-pdfium-diff.py` to full-page pdfium/pypdfium2 pixel-parity
  over the loadable corpus; documented justified tolerance band reporting
  DISTRIBUTIONS; three-bucket classification [benign-renderer-noise /
  known-disclosed-gap (subtract the Diagnostics unsupported-tally, don't
  re-report) / unexplained], enumerated by file+reason; triage+fix cheap
  bucket-iii bugs, file the rest as counted named render-gaps; encode the
  known pdfium reference-divergences [`FPDF_FFLDraw` widgets + synthesized
  no-`/AP` appearances R43 refuses]; WIRE into the standing gate set;
  DeviceCMYK colorimetry characterized corpus-wide, fixed only if bounded).
  **Engineer DISPATCHED** (no blocking spec — pure measurement).
- **ROADMAP Next up** = **Pass 12** (canvas-interaction foundation +
  editing-GUI consolidation — reconciles the three named GUI follow-up
  slices [Pass-6.1 markup-drawing state machine; Pass-7 form-fill GUI
  `docs/ui_specs/pass-7-form-fill.md`; Pass-8 redaction-marking GUI
  `docs/ui_specs/pass-8-redaction.md`] as SLICES on ONE shared substrate,
  not three independent buckets), then **Pass 9** (Vector/Inkscape,
  repositioned after Pass 12, promoted onto C+B + the 6.1 serializer + 8.0
  surgery interpreter, sliced (a)–(g) per decision 008 §5.3).
- **ARCHITECTURE §12** — dated decision-010 entry (the C→B→A sequence, the
  render-fidelity + one-canvas + Inkscape-reference rules, the
  destination-unchanged/path-amended framing, the
  `pdfce-inkscape-librarian` commissioning).

**THE Pass-1.1-remainder discharge (via Pass 11):** Pass 11 is scoped to
DISCHARGE the long-owed "full-page pixel-parity remainder (Pass 1.1)"
carried in every recent "still open" list — EXPLICITLY and conditionally:
report it closed ONLY if the harness genuinely generalizes to full-page
corpus scale (not a "pixel-perfect" claim).

**Commissioning:** a new sibling agent **`pdfce-inkscape-librarian`** + the
**`Inkscape_Features` RAG** (`D:\Dev\Rag-Specialized\Inkscape_Features\`)
are being COMMISSIONED 2026-08-01 in parallel (another agent creating the
agent file + scaffold) — closes decision 008 §11.4's previously-unowned
Inkscape-catalog item. Registered in the project agent roster
(`CLAUDE.md`'s "Project agents" table — the roster row only). A private
development-reference corpus (same posture as the Acrobat Features RAG):
never shipped, never committed to the pdfce repo.

**Still in flight (continuation 29):**
- **Pass 11 (render-fidelity verification) — IN PROGRESS, engineer
  dispatched.** No blocking spec.
- **Pass 12 (canvas-interaction foundation) — NEXT.** Scope into full
  acceptance criteria via `pdfce-ui-specialist` when reached; governed by
  R60 (one substrate).
- **Pass 9 (Vector/Inkscape) — DESTINATION, after Pass 12.** Awaits the
  `Inkscape_Features` catalog being built out; R61 (behavioral reference
  only).

**For next session / STILL-OPEN operator items (RE-SURFACED, not re-filed;
ordered by age — oldest first, led by the encryption sign-off which is now
owed across FOUR decisions 007/008/009/010):**
- **Encryption-refusal operator sign-off** — STILL the oldest owed
  operator item; now owed across FOUR decisions (007/008/009/010). Pass 5
  stays fallback/interleave; this sign-off gates its scoping.
- **License decision (`LEGAL.md` §1)** — still undecided (gates any public
  repo/release and what copyleft prior art is even usable).
- **Commit authorization** — everything remains UNCOMMITTED in git
  (**Passes 0–8.0 ALL uncommitted**).
- **W15 — no remote/CI** — unchanged.
- **`/R 6` sourcing method** — cross-implementation triangulation vs a
  purchased ISO 32000-2 copy. Ken's call (gates Pass 5).
- **LEGAL.md §2 Adobe-supplement copyright contradiction** — flagged for
  Ken; LEGAL.md NOT edited.
- Carried Pass-4 residuals and the `/Info`-not-certification-gated item
  remain open (see continuation 20).

**Same-day continuation 30 — Pass 11 (render-fidelity verification) SHIPPED;
operator reprioritization to a measurement/dimensioning beta (decision 011 IN
FLIGHT).**

**Shipped:**
- **Pass 11 — Render-fidelity verification harness SHIPPED 2026-08-01** (full
  record: `ROADMAP.md` Shipped). Decision 010's candidate C, the first Pass of
  the C → B → A sequence, delivered as **PURE MEASUREMENT — ZERO Rust touched,
  ZERO new pdfce dependency** (pypdfium2 is dev-tooling only, out-of-tree, NOT
  vendored, absent from `THIRD_PARTY_LICENSES.md`). New files:
  `tools/render-parity/render_parity.py` (drives `pdfce-cli render-page` +
  pypdfium2, aligns rasters, per-channel per-pixel deltas — mirrors
  `tools/content-identity/`), `tools/render-parity/README.md`, and
  `tools/render-parity/out/{summary.txt,summary.json,per-page.tsv,diffs/}`.

**Findings + decisions:**
- **The tolerance band is EMPIRICAL, not tuned (the analytical core, Y1/W14).**
  Metric `frac_over_32` = fraction of pixels whose max-channel |Δ| > 32/255.
  Benign AA/hinting/sub-pixel noise is confined to a thin edge band (small
  AREA) even where edge pixels swing full-range, so the noise-robust
  discriminator is AREA-fraction, not max delta. Band = **p99.9 of
  `frac_over_32` over the 1,728 clean-by-construction pages** (zero disclosed
  gaps + no DeviceCMYK) — a property of the known-benign population, so it
  CANNOT be tuned to pass a bug (W14 structurally satisfied). This run: band
  **0.0294**; clean floor mean 0.00096 / p95 0.0022 / p99 0.0098 (tight,
  well-separated). The report always prints the DISTRIBUTION.
- **Three buckets — full loadable corpus (2,914 files → 2,890 pages, 125 DPI,
  content-only; ZERO panics/timeouts; 24 skips = unloadable `fail-*` files):**
  (i) benign-renderer-noise 2,840; (ii) known-disclosed-gap 49
  (cross-referenced against pdfce's existing Diagnostics tally so
  already-counted gaps are SUBTRACTED, not re-reported — 48 deferred-op
  sh/OC/Type3, 7 font-unsupported, 6 DeviceCMYK-file, 2 substituted, 2
  image-unsupported, 1 codec-feature); (iii) **unexplained-divergence 1.**
- **The single unexplained page = `A019-pdfa2-pass-a.pdf`** (TWG test suite): a
  form XObject fills a triangle with a vertex at x ≈ 3.4028e38 (≈ `f32::MAX`);
  pdfium rejects/clips the out-of-range path, pdfce rasterizes a spurious cyan
  bar. **FILED as a named counted render-gap (R20/R27), NOT fixed** — the fix
  is a clamp/reject-policy call in `pdfce-render` (R34 risk), Pass-9-adjacent;
  the measurement-only non-goal (Y3) binds.
- **DeviceCMYK = FIRST NAMED RESIDUAL (NOT fixed).** DeviceCMYK-only pages
  diverge 3.0× the clean-page mean; the delta lights the whole filled area
  uniformly with POLARITY IDENTICAL (R29 holds) — the naive additive
  `Rgb::from_cmyk` vs pdfium `AdobeCMYK_to_sRGB1` gap. Filed as a follow-up
  colour Pass (decision 006 §3.4 polarity matrix must be re-pinned FIRST; 006
  revisit-trigger 7; scope via `pdfce-acrobat-librarian`). Cross-refs decision
  010 revisit-trigger 6.
- **R59 discharged for the first time.** `--gate --max-unexplained <baseline>`
  returns non-zero when the unexplained count rises; **baseline = 1** (the A019
  file), verified PASS. A REQUIRED re-run on every render-touching Pass
  (R34/R46 pattern), especially Pass 9. Local-corpus gate (pypdfium2 not in CI,
  like content-identity / roundtrip). Reference-side pdfium quirks (`--annots`
  mode: `FPDF_FFLDraw` widgets + synthesized no-`/AP` looks R43 refuses)
  bucketed reference-side (Y2), 3 pages verified.
- **Pass 1.1 pixel-parity remainder DISCHARGED — stated exactly.** The harness
  genuinely generalizes to full-page corpus scale (per-channel per-pixel; full
  loadable corpus; first-page coverage of every file; multi-page via
  `--pages-per-file 0`, demonstrated) — decision 010's exact bar. Scope named
  precisely (first-page corpus coverage + a multi-page knob), NOT overclaimed
  as exhaustive-multi-page or pixel-perfect. **STRUCK from the "still open"
  lists going forward** (prior entries not rewritten).

**Reprioritization (operator, 2026-08-01):**
- **Operator requested a measurement/dimensioning BETA as his first usable
  deliverable** — scaled dimensions + vector selection/snapping + basic vector
  editing. Its architecture is being decided via KenAgent as **decision 011 —
  IN FLIGHT** (record will land at `docs/decisions/011-*.md`). The beta PULLS
  FORWARD decision 010's **Pass 12** (canvas-interaction foundation / B) + the
  **first slices of Pass 9** (vector editing / A) and adds a new dimensioning
  subsystem. Mechanism = decision 010 revisit-trigger 3 (operator wants vector
  editing sooner, now on a *corpus-measured* render rather than spot-checked).
- **Decision 010's C → B → A sequence CONTINUES after the beta;** Pass 11 (C)
  is now Shipped so the render is VERIFIED for the editing work. The beta's
  Pass IDs / slices are defined by decision 011, NOT invented here. ROADMAP In
  progress = "Beta: scaled measurement/dimensioning tool (decision 011
  IN FLIGHT)"; the decision-010 forward-sequence Pass-11 bullet flipped to
  SHIPPED.

**Filed this session:**
- **ROADMAP Shipped** = Pass 11 (full record). **ROADMAP In progress** replaced
  with the Beta entry (decision 011 in flight); the carried Pass-5
  reconciliation pointer retained. **ROADMAP Next up** decision-010 sequence:
  Pass 11 → SHIPPED + a reprioritization note. **ROADMAP Backlog** = two new
  named render-fidelity residuals: (a) out-of-range (near-`f32::MAX`)
  path-coordinate robustness in `pdfce-render` (the A019 gap); (b) DeviceCMYK →
  sRGB colorimetry colour Pass. **ARCHITECTURE §12** = dated Pass-11-shipped
  entry (harness, area-fraction band, 1 unexplained, DeviceCMYK residual,
  Pass-1.1 discharge, R59 baseline, the beta reprioritization).
- **RAG escalations:** `C:\personal_rag\pdf\` — new lesson on the
  area-fraction-not-max-delta tolerance-band methodology (separating benign
  independent-renderer noise from real divergence; band from the
  clean-by-construction population so it can't be tuned to pass).
  `D:\dev\rag\rust\` — new lesson on the `nohup`-detach background-sweep gotcha.
  Subject + master + subdir indexes updated.

**Gates:** `cargo fmt --check` clean; `cargo tree` core+render GUI-free; ZERO
Rust delta → clippy/test/R34/R46 unmoved by construction; no `Cargo.toml`
change → `THIRD_PARTY_LICENSES.md` unchanged; deterministic/locale-invariant
(sorted files, fixed DPI, no clocks).

**Still in flight (continuation 30):**
- **Beta (measurement/dimensioning) — IN PROGRESS;** decision 011 (architecture)
  IN FLIGHT via KenAgent.
- **Pass 12 (canvas-interaction foundation) + Pass 9 (vector/Inkscape)** — the
  remaining C → B → A work, resumes after the beta.
- **Pass 5 (Encryption)** — fallback/interleave, unchanged.

**For next session / STILL-OPEN operator items (RE-SURFACED, not re-filed;
ordered by age — oldest first; MINUS the now-discharged Pass 1.1 pixel-parity
item):**
- **Encryption-refusal operator sign-off** — the oldest owed operator item
  (now owed across FOUR decisions 007/008/009/010).
- **LEGAL.md §2 Adobe-supplement copyright contradiction** — flagged; LEGAL.md
  NOT edited.
- **`/R 6` sourcing method** — Ken's call (gates Pass 5).
- **License decision (`LEGAL.md` §1)** — still undecided.
- **Commit authorization** — everything remains UNCOMMITTED
  (**Passes 0–8.0 + the `tools/render-parity` additions ALL uncommitted**).
- **W15 — no remote/CI** — unchanged.
- Carried Pass-4 residuals and the `/Info`-not-certification-gated item remain
  open (see continuation 20).

**Same-day continuation 31 — GUI polish (current feature set) + launcher
shipped (operator-requested interlude, NOT a feature Pass); measurement/
dimensioning beta prerequisites COMPLETE, awaiting operator go-ahead.**

**Shipped:**
- **GUI polish (current feature set) + launcher** (full record: `ROADMAP.md`
  Shipped, top — filed WITHOUT a Pass number, correctly: no new document
  capability, `pdfce-gui` + `ui_text.rs` only, ZERO new deps). Operator's
  ask: "get the GUI polished up for the current feature set, then give me a
  way to launch it from `D:\Dev\pdfce`." Executed against
  `docs/ui_specs/gui-polish-current-featureset.md`.
- **All 6 P0 items:** (1) `open_path()` resets stale per-document narration
  (`edit_note`/`copy_result`/`copy_detail_expanded`/`pending_text_kind`/
  `text_input`); (2) Properties panel reseeds on opening a second file (no
  empty grid); (3) window title reflects the open file
  (`ViewportCommand::Title`; new `ui_text` `window_title_idle`/`_open`); (4)
  status-bar height cap (`ScrollArea max_height=220`, no disclosure
  suppressed); (5) real empty state (heading + inline Open button + drop
  hint) + working drag-and-drop (`dropped_files`, `.pdf`, restricted to
  Idle/Failed/Unsupported so unsaved edits can't be silently discarded); (6)
  annotation-visibility toggle uses `ICON_BUTTON_SIZE`.
- **All P1 items:** colour-not-sole-signal on the four toggles (bold active
  label); keyboard-shortcuts reference window (⌨ button,
  `ui_text::shortcuts_reference`, doc-commented to stay in step with
  `collect_keyboard_actions`); text-menu wording + colour note + per-add
  jitter (`author_jitter` mod-6×12pt so repeated author-at-center adds don't
  stack invisibly); utility-cluster spacing; Revert-disabled tooltip;
  **accessible names on every glyph-only icon button** via a new
  `Self::icon_button()` helper (egui 0.35 `Response::widget_info` +
  `WidgetInfo::labeled`/`selected` — API verified available in 0.35).
- **Launcher (repo root, NEW):** `D:\Dev\pdfce\pdfce.bat` +
  `D:\Dev\pdfce\pdfce.ps1` — double-clickable / drag-a-PDF / `pdfce.bat
  [file]`; both `cd` to repo root, `cargo build --release -p pdfce-gui`
  (fast freshness check → always latest), then `Start-Process` the exe
  detached with an optional file arg. Smoke-tested end-to-end (release GUI
  launches, no startup crash).

**Gates:** `cargo fmt --check` / `clippy -D warnings` clean; **31 pdfce-gui
tests pass**; GUI-core-separation invariant confirmed (`cargo tree -p
pdfce-core`/`-p pdfce-render` still egui/eframe/winit/wgpu/glow/rfd-free);
`ui-strings` R1 gate clean; release rebuilt + smoke-tested. **ZERO new
dependencies** → `THIRD_PARTY_LICENSES.md` unchanged.

**Deferred (named follow-ups, NOT built — filed to `ROADMAP.md` Backlog):**
- **Polish residuals (cosmetic, low priority):** P2-1 recent-files list
  (needs settings persistence); P2-2 window/taskbar app-icon asset (needs
  artwork); P2-3 light-mode visual QA pass (no hardcoded colours added —
  stays OS-theme-driven); P2-4 markup colour-picker tooltip; P2-5
  screenshot-driven spacing QA.
- **TWO DATA-SAFETY items — NOT polish, real, still-open** (the crash-safe-
  autosave / non-destructive-by-default standing UX rule, ui-specialist's
  territory; filed as their own Backlog bucket, above the polish residuals):
  (1) **no autosave / crash-recovery scratch file exists** — an unsaved
  editing session is lost on a crash; (2) **true in-place Save remains
  (correctly) GATED on that autosave existing** — "Save a copy" is still the
  only save affordance. Recorded prominently so a future session does not
  read them as done.

**Still in flight (continuation 31):**
- **Beta (measurement/dimensioning) — decision 011 CONCLUDED + ARCHIVED**
  at `docs/decisions/011-first-beta-scaled-measurement-dimensioning-tool.md`
  (five slices **12.0 / 9a / 12.M1 / 12.M2 / 9c-min**). Its research
  **prerequisites are COMPLETE** — spec §12.9/§14.5/§8.11, the Acrobat
  measuring-tools bucket, and the Inkscape selection+snapping bucket are all
  sourced. **The beta build awaits operator go-ahead** (Ken is reviewing the
  plan); the engineer starts on his confirmation. Decision 010's C → B → A
  sequence continues after the beta (Pass 11 shipped → render verified).
- **Operator is now actively / interactively using the GUI** — the `/loop`
  autonomous mode is STOPPED; work is interactive from here.
- **Pass 12 (canvas-interaction foundation) + Pass 9 (vector/Inkscape)** —
  the remaining C → B → A work, resumes after the beta.
- **Pass 5 (Encryption)** — fallback/interleave, unchanged.

**For next session / STILL-OPEN operator items (RE-SURFACED, not re-filed;
ordered by age — oldest first; PLUS the two new data-safety follow-ups):**
- **Encryption-refusal operator sign-off** — the oldest owed operator item
  (owed across decisions 007/008/009/010).
- **LEGAL.md §2 Adobe-supplement copyright contradiction** — flagged;
  LEGAL.md NOT edited.
- **`/R 6` sourcing method** — Ken's call (gates Pass 5).
- **License decision (`LEGAL.md` §1)** — still undecided.
- **Commit authorization** — everything remains UNCOMMITTED (**Passes 0–8.0,
  the `tools/render-parity` additions, AND the GUI-polish + launcher changes
  ALL uncommitted**).
- **W15 — no remote/CI** — unchanged.
- **NEW — Autosave / crash-recovery scratch file** — none exists; an unsaved
  editing session is lost on a crash (data-safety, standing UX rule).
- **NEW — True in-place Save** — deliberately gated on the autosave/recovery
  mechanism; "Save a copy" is the only save affordance until it lands.
- Carried Pass-4 residuals and the `/Info`-not-certification-gated item remain
  open (see continuation 20).

**Same-day continuation 32 — OPERATOR PRIORITIZATION DIRECTIVE recorded:
Acrobat TEXT-handling parity is the NEXT MAJOR FOCUS after the in-flight
decided work, ahead of the Inkscape/vector breadth.**

**Operator directive (Ken, 2026-08-01) — verbatim intent:** *"Continue the
autonomous loop, but when you finish doing the decided work, focus on bringing
the software to parity with Adobe Acrobat's text-handling capabilities such as
paragraphs, etc. Focus on bringing parity with Acrobat first before continuing
to build what Inkscape is better at."*

**Reprioritization recorded (append-only — no prior entry rewritten):**
- **NEXT MAJOR FOCUS = Adobe Acrobat TEXT-handling parity** — "Edit PDF"-style
  in-place text editing, paragraph/text-block recognition, reflow, text
  formatting, and font-handling-on-edit. This is a **NEW major subsystem:
  editing the document's own page TEXT CONTENT** — explicitly distinct from
  the shipped text **EXTRACTION** path (Pass 4) and the text-bearing
  **ANNOTATIONS** path (Pass 6.2, overlays authored on top of the page).
- **It starts only AFTER the currently-DECIDED / IN-FLIGHT work completes:**
  operator-supplied **font-supply** (decision 012, building now); the **Pass
  12.0 canvas-interaction substrate** (decision 010 candidate B / beta slice,
  being designed→built); **xref-recovery** (decision 013, in KenAgent
  consultation now); and the **measurement/dimensioning beta foundation**
  (decision 011). The directive defines what comes NEXT — it does not
  interrupt the decided work.
- **Prioritized AHEAD of the further Inkscape/vector-editing BREADTH.** It
  LEAPFROGS decision 008's Pass 9 vector-editing slices **(b)–(g)** (boolean
  ops; gradients/shading/transparency; node/Bézier beyond basic; text-to-path;
  OCG layers) and the "Vector graphics editing (Inkscape-parity)" Backlog
  bucket. **Recorded as AMENDING decision 010's destination-ranking:** decision
  010 made vector/Inkscape editing candidate **A** (highest-value
  post-foundation investment); the operator now places **Acrobat TEXT parity
  ahead of the Inkscape-vector breadth**. Pass 9's ID + destination survive —
  a ranking amendment, not a cancellation. Formal record will be KenAgent
  decision ~014 once the in-flight work + parity catalog land (rule-12
  discipline: parity reference → scope → KenAgent decision → build).

**Shared-canvas note (why decision 010's C and B are UNAFFECTED):** candidate
C (render-fidelity verification, **Pass 11**) is SHIPPED; candidate B (**Pass
12** canvas foundation) proceeds unchanged. Acrobat-style in-place text editing
needs the **same interactive canvas substrate** (R60: focusable canvas +
screen↔page transform + hit-test/selection + live-preview overlay) as the
Pass-9 vector work, so the canvas is **doubly justified** and continues.
Acrobat-text parity is a CONSUMER of that substrate.

**Beta-sequencing FLAG (decision 011 — not cancelled):** the beta's SHARED
Pass-12.0 canvas foundation proceeds; the beta is Ken's stated "first beta."
Its dimensioning slices are unaffected. But its **vector-selection /
basic-editing slices (9a / 9c-min)** are Inkscape-adjacent, so their placement
RELATIVE to Acrobat-text parity is an **operator sequencing question to
confirm** — recorded as a flag, NOT a cancellation of the beta.

**Teed up NOW:** `pdfce-acrobat-librarian` is cataloging Acrobat's **"Edit
PDF" text-handling** capabilities (in-place edit, paragraph/reflow, formatting,
font-on-edit, limits) at `D:\Dev\Rag-Specialized\Acrobat_Features\` — the
parity reference that will ground the future KenAgent architecture decision.

**Recorded in (this continuation):**
- `ROADMAP.md` Backlog — new top "★ NEXT MAJOR FOCUS — Acrobat text-handling
  parity" bucket (full framing + capability list + ahead-of-Inkscape
  prioritization + acrobat-librarian-cataloging-now status).
- `ROADMAP.md` Next up — dated AMENDMENT note in the decision-010
  forward-sequence block (the destination-ranking amendment; C/B unchanged;
  shared canvas; beta sequencing flag).

**Still-open operator items (UNCHANGED — re-surfaced, not re-filed; ordered
oldest-first):**
- **Encryption-refusal operator sign-off** — the oldest owed operator item.
  **Now DOUBLY confirmed as low-payoff:** the OSS-corpus `/Encrypt` sweep at
  **~5%** (92.5% legacy R≤4) PLUS the operator's stamped-drawings context both
  point the same way; promotion trigger still NOT met.
- **LEGAL.md §2 Adobe-supplement copyright contradiction** — flagged; LEGAL.md
  not edited.
- **`/R 6` sourcing method** — Ken's call (gates Pass 5).
- **License decision (`LEGAL.md` §1)** — still undecided.
- **Commit authorization** — everything remains UNCOMMITTED.
- **W15 — no remote/CI** — unchanged.
- **Autosave / crash-recovery scratch file** + **true in-place Save** (gated on
  it) — the two data-safety follow-ups (continuation 31), still open.
- **Top ROBUSTNESS item — the xref-recovery finding at ~85% of real-file
  failures** (decision 013, in KenAgent consultation now) is the current
  leading robustness priority within the in-flight decided work.

**Same-day continuation 33 — CONSOLIDATED filing: root-cause font fix +
operator-supplied fonts (decision 012) + Pass 12.0 canvas substrate +
decision 013 (xref recovery, Pass 13a shipped / 13b in progress) + test
infrastructure; standing-rule collision reconciled (R62–R68 assigned);
autonomous /loop RESUMED.**

**LOOP-STATUS CORRECTION (supersedes continuation 31):** continuation 31
recorded the `/loop` autonomous mode as STOPPED/interactive. That is now
CORRECTED — the operator RESUMED the autonomous loop
(`/loop @agent invoke autonomous-builder`). **The autonomous loop is ACTIVE
again**, with interactive check-ins interleaving. (Append-only correction;
continuation 31's entry is not rewritten.)

**Shipped (full records in `ROADMAP.md` Shipped):**
- **Font-fix — NUL-misroute of no-cmap CIDFontType2 embedded TrueType —
  COMPLETE.** The root-cause bug behind the operator's real drawing
  rendering with missing text. Format detection trimmed leading whitespace
  **including NUL** before magic-sniffing → the leading NUL of sfnt magic
  `0x00010000` was stripped → `01 00 …` matched bare-CFF magic → TrueType
  handed to the CFF parser ("offset out of bounds" that *looked* like a
  read-fonts objection but was a caller-side misroute). Fix: match magics on
  RAW bytes, trim only on the Type 1 `%!` text path, never NUL. **skrifa
  stays 0.42.1 pinned — pdfce-side routing bug, no bump.** Class impact: all
  embedded TrueType from SolidWorks/AutoCAD/Office CAD. **Render-parity
  footer IN → COMPLETE** (the earlier "corpus-regression footer owed"
  residual is discharged): R59 gate `--max-unexplained 1` exit 0 over 2,914
  files / 2,922 pages — unexplained **1→1** (no regression; the 1 is the
  pre-existing A019 f32 case), font-unsupported **7→0**, benign **2840→2868**,
  known-gap **49→53** (correct rise — text now renders, revealing already-
  disclosed shading/marked-content gaps previously MASKED by the whole-font
  skip), band **0.02942→0.02963**. New `Diagnostics::fonts_unsupported_by_
  reason` (+6 CLI tokens). Synthetic CC0 fixture
  `fixtures/synthetic/text/cidfonttype2-nocmap-embedded.pdf` +
  `tools/gen-cidfont-nocmap-fixtures.py` + `tests/cidfont_nocmap_render.rs`
  (never the proprietary file). 1,018+ tests green, ZERO dep change, release
  rebuilt. **Residual (Backlog):** no dedicated `font_program` fuzz target
  yet.
- **Operator-supplied fonts (decision 012 first cut).** Non-embedded,
  non-Base-14 SIMPLE fonts render from an operator-supplied folder via the
  `FontEnvironment.named` seam (decision 004 §5.3). `substituted: bool` →
  `GlyphSource {Embedded, Bundled, Supplied}`; `substitute_face` returns the
  source + subset-tag retry; `face_names()` on the one skrifa parser (R21);
  `Diagnostics.glyphs_supplied`/`supplied_fonts` distinct from bundled. CLI
  `--font-dir` (repeatable, render-page) + shell folder-walk + three-way
  disclosure. GUI "Font folders" tool (**session-state — not persisted; the
  R15 user-state partition doesn't exist yet, so persistence deferred with
  it**). Acceptance all met: non-embedded Calibri renders bundled without /
  supplied with `--font-dir`; **positions BYTE-IDENTICAL** when
  supplied==bundled (positions come from `/Widths`); subset-tag resolution;
  corrupt files skip-and-note; composite still `CompositeNotEmbedded`; the
  R64-equiv font-dir-independence gate holds. **1,045 tests, all gates green,
  ZERO new deps, release rebuilt.** Deviations: `--font-dir` render-page only;
  GUI session-state not persisted; inline "supply this font" link deferred.
  Fast-follows FF1 (OS-font enumeration) / FF2 (composite via Unicode route) /
  FF3 (descriptor auto-routing). **FONT-ON-EDIT CONNECTION:** decision 012 is
  the enabler for the upcoming Acrobat text-editing (a typed glyph needs the
  font available).
- **Pass 12.0 — canvas-interaction substrate.** The single shared substrate
  R60 mandates, shipped UNINHABITED (no tools → viewer behavior unchanged).
  New `crates/pdfce-gui/src/canvas.rs` (`CanvasTool` ships uninhabited;
  `CanvasTargetProvider` trait + `EmptyTargetProvider`; selection-set model;
  pure state-machine fns + tests). `viewer.rs` FOUR geometry bridges —
  `screen_to_page`/`page_to_screen` + the **new `canvas_to_pdf_space`/
  `pdf_space_to_canvas`** built by inverting `page_device_geometry`'s
  `Transform` (a genuine finding beyond decision 011's literal 12.0
  deliverables: device-Y-down ↔ PDF-Y-up correctness), proven at 0/90/180/270°
  + 1/zoom invariance. `main.rs` wiring (focusable canvas
  `Sense::click_and_drag`, pan-suppression, four-way Escape precedence,
  overlay). **`MarkupTool` → `CanvasTool` rename** (permanent; noted vs
  pass-6.1/pass-8 specs). 47 gui tests, full-workspace gates green, GUI-core
  separation intact, wasm32 clean, ZERO new deps, release rebuilt.
  Deviations: image drag-sense gated on `suppress_pan` (egui 0.35 pans
  first); `target_provider = Some(EmptyTargetProvider)` not `None`
  (observably identical). Out-of-scope pre-existing fix: a doc-comment clippy
  error in `pdfce-core/document.rs` (zero functional impact). Residuals: Pass
  9a plugs the real target provider + marquee-vs-pan; 6.1/8/12.M2/9c-min plug
  real `CanvasTool` variants; Pass 7 the global-vs-focused keyboard
  reconciliation. Decision 010's Pass 12 / candidate-B foundation slice —
  **doubly justified** (shared by Acrobat text editing + measurement +
  vector per the continuation-32 reprioritization).
- **Pass 13a — cross-reference EOL/CRLF audit (decision 013 Pass A, NEGATIVE
  RESULT filed).** Parser confirmed EOL/CRLF-correct (9 synthetic legal-EOL
  fixtures all parse); **547/567 sampled real failures are OFFSET-SHIFT
  corruption** (LF→CRLF byte-growth invalidating startxref + offsets), **0
  genuine parser bugs**; no parser code changed (tests + fixtures + tools
  only: `fixtures/synthetic/xref-eol/`, `tools/gen-xref-eol-fixtures.py`,
  `tools/xref-crlf-classify.py`, `tests/xref_eol.rs`). Surfaced a `gen-65536`
  tolerance candidate (17 files, out-of-spec generation > 65535, NOT
  CRLF-related — a separate future decision).
- **Test infrastructure (standing gates + tools).** (a) font-parse
  regression harness `tools/font-parity/` (parses every embedded font,
  asserts routing-or-clean-fail, 0 misroutes, guards the NUL bug; standing
  rule **R68**); (b) `tools/realdrawings-smoke/` (operator's private
  read-only `R:\Products` render smoke — **results gitignored, nothing
  proprietary committed**; font fix holds across 339 real drawings,
  `unsupported=0`); (c) OSS-corpus expansion — **+1,109 real-world PDFs**
  (pdfium BSD 331, qpdf Apache 639, PDFBox Apache 139) into gitignored
  `fixtures/external/` with per-source PROVENANCE (**pdf.js SKIPPED** —
  unclear per-file provenance; GPL/AGPL projects avoided). Corpus now **~4,000
  files**. Sweep tooling `fixtures/external/realworld-sweep.sh`.

**Decisions filed this session:**
- **Decision 012 (operator-supplied fonts)** — archived
  `docs/decisions/012-operator-supplied-fonts.md`, dated 2026-07-31. Folder-
  based supply for non-embedded SIMPLE fonts; three trust levels; renderer
  bytes-in; composite/OS-fonts/descriptor-routing deferred as named fast-
  follows.
- **Decision 013 (xref recovery)** — archived
  `docs/decisions/013-xref-recovery.md`, dated 2026-07-31. Two sequenced
  Passes: 13a (EOL audit — done, negative result) → 13b (rebuild-by-scan —
  in progress). Subsumes decision 007 §10 item 6 (offset-start file).

**STANDING-RULE COLLISION RECONCILED (the gating action):** three recent
decisions proposed COLLIDING R-numbers. Verified the current highest assigned
= **R61** (decision-010 Inkscape-behavioral-reference). Assigned the next-free
numbers IN ORDER and recorded the mapping in `ROADMAP.md` Standing rules:
- **Decision 012** proposed R61–R65 (R61 taken) → assigned **R62–R66**
  (record-R61→R62 supplied-shell-sourced; R62→R63 three-trust-levels;
  R63→R64 supplied-outside-determinism-gate; R64→R65 composite-Unicode-route-
  only; R65→R66 OS-fonts-opt-in).
- **Decision 013** proposed R59 (taken) → assigned **R67**
  (recovered-base-forces-full-rewrite).
- **Font-parse regression harness** proposed R62 (would collide) → assigned
  **R68** (embedded font programs route to the correct parser or fail clean;
  magic/variant disagreement = gate failure; R46/R59 re-run pattern).

**OWED CODE FOLLOW-UPS (recorded, NOT done — librarian does not edit code):**
1. The operator-supplied-fonts `pdfce-render` implementation uses R61/R62/R63
   in in-code doc comments (the record's proposed numbers) — must be updated
   to the **assigned R62/R63/R64**.
2. Any Pass-13b code comments citing the recovered-base rule as "R59" must be
   updated to **R67** when Pass 13b lands.

**Findings + decisions:**
- ARCHITECTURE `§12` gained dated decision-log entries for decisions 012 and
  013; `§5.10` (recovered-base-forces-full-rewrite, sibling to §5.2/R35 and
  §5.9/R58) written **marked pending Pass-13b ship**.
- RAG lessons filed this continuation (see below).

**RAG escalations this continuation:**
- `C:\personal_rag\pdf\` — the NUL-misroute finding: read-fonts `FontRef::new`
  ACCEPTS valid no-cmap subset-TrueType CIDFontType2; an "offset out of
  bounds" is a caller-side misroute (NUL-as-whitespace stripping the sfnt
  magic), not a read-fonts objection.
- `D:\dev\rag\rust\` — `skrifa::FontRef` is a re-export of
  `read_fonts::FontRef`; `FontRef::new` is lenient, so `ReadError::OutOfBounds`
  from a font-detection wrapper almost always means the WRONG parser was
  invoked (CFF on TrueType bytes); NUL-as-whitespace corrupts the `0x00010000`
  magic.

**Still in flight (continuation 33):**
- **Pass 13b (rebuild-by-scan xref recovery)** — IN PROGRESS (the 85%
  real-world robustness fix). Queued to avoid concurrent pdfce-cli edits with
  the just-shipped font-supply work.
- **Beta (measurement/dimensioning, decision 011)** — foundation (Pass 12.0)
  now SHIPPED; remaining slices await operator go-ahead. The 9a/9c-min
  vector-selection slices' placement relative to Acrobat-text parity is the
  flagged operator sequencing question (continuation 32).
- **★ NEXT MAJOR FOCUS — Acrobat text-handling parity** — routes through a
  future KenAgent decision (~014) AFTER the in-flight decided work
  (012/013/Pass 12.0/011) lands; `pdfce-acrobat-librarian` cataloging "Edit
  PDF" text-handling now. **Decision 012 is its font-on-edit enabler.**
- **Pass 5 (Encryption)** — fallback/interleave; now DOUBLY confirmed
  low-payoff (~5% corpus `/Encrypt` + operator stamped-drawings context).

**Still-open operator items (UNCHANGED — re-surfaced, not re-filed; ordered
oldest-first):**
- **Encryption-refusal operator sign-off** — oldest owed; now the #2
  real-world gap (~5%, empty-password permissions) + the stamped-drawings
  context; promotion trigger still NOT met.
- **LEGAL.md §1 license decision** — still undecided (LEGAL.md not edited).
- **LEGAL.md §2 Adobe-supplement copyright contradiction** — flagged; not
  edited.
- **`/R 6` sourcing method** — Ken's call (gates Pass 5).
- **Commit authorization** — everything remains UNCOMMITTED; the tree is now
  **very large** (Passes 0–8.0, Pass 11, Pass 12.0, Pass 13a, the font-fix +
  decision-012 font-supply, the render-parity + font-parity + realdrawings +
  OSS-sweep tooling, GUI-polish + launcher — all uncommitted).
- **W15 — no remote/CI** — unchanged.
- **Autosave / crash-recovery scratch file** + **true in-place Save** (gated
  on it) — the two data-safety follow-ups (continuation 31), still open; the
  GUI "Font folders" persistence is now DEFERRED with the same R15 user-state
  partition.

**For next session:**
- Land **Pass 13b** (rebuild-by-scan recovery) against decision 013 §3.3–§5
  acceptance; then flip `ARCHITECTURE.md` §5.10 from "pending Pass-13b ship"
  to shipped and dispatch the librarian to move Pass 13b to Shipped.
- Apply the **two owed code follow-ups** (R61/R62/R63 → R62/R63/R64 in the
  font-supply doc comments; "R59" → R67 in Pass-13b comments).
- Work the ranked OSS real-world gaps (Backlog): after xref recovery, the
  `/Resources`-omission tolerance, LZW EarlyChange edges, remaining font
  subtypes, and the undecodable-image cases.

**Same-day continuation 34 — decision 014 filed (Acrobat text-handling
parity, the ★ NEXT MAJOR FOCUS), Pass 13.x→14.x RENUMBER + R69–R74;
Pass 13b SHIPPED (the 85%-real-world-recovery win, zero regression);
autonomous loop still ACTIVE:**

**Decisions filed this session:**
- **Decision 014 (Acrobat-style in-place text editing) ACCEPTED via the
  KenAgent protocol and archived: `docs/decisions/014-acrobat-text-editing.md`
  (dated 2026-07-31).** This is the formal architecture record the
  operator's ★ NEXT MAJOR FOCUS directive (continuation 32) named as
  "~014" — Acrobat "Edit PDF" parity: in-place editing of the page's OWN
  text content (distinct from the shipped EXTRACTION path, Pass 4, and the
  ANNOTATIONS overlay path, Pass 6.2). Headline design calls: **M-hier**
  text model (Run→Line→Block, derived from Pass 4's extraction, fully
  reviewable — never authoritative); **E-surgery** edit mechanism (extends
  Pass 8.0's advance-preserving REMOVE interpreter to REPLACE — the second
  sanctioned page-content-rewrite operation after redaction, R47's line);
  **F-refuse** font-on-edit posture (edit only with glyphs the run's font
  can already provide; refuse-and-disclose a glyph an embedded SUBSET
  lacks; font-subsetting deferred as FF-C, permissive-only); **RL-line**
  first-cut relayout (single-line advance-preserving; block reflow is
  FF-A/FF-B, the ladder's exceed-Acrobat play — pdfce's offline cross-block
  reflow beats Acrobat's cloud-gated, English-only one); **default
  INCREMENTAL save (R36)**, explicitly NOT a fourth forced-full-rewrite
  sibling (redaction stays the one operation that truly removes); **T-
  disclose** tagged-PDF handling (preserve BDC/EMC+MCID, disclose
  `/ActualText` staleness — Acrobat's own in-place edit is known to
  corrupt the structure tree; pdfce doesn't).
- **PASS-NUMBER RENUMBER, recorded explicitly (do not lose this):**
  decision 014's own archived record proposes the family as "Pass 13.x
  (13.0–13.3)". **13.x was already taken** — Pass 13a and Pass 13b
  (decision 013, xref recovery) were both assigned that number and BOTH
  SHIPPED before decision 014 landed (see below). The librarian assigned
  the text-editing family the next free MAJOR number, **Pass 14.x**:
  **14.0** read-only text model + block recognition (core + CLI
  `inspect --text-blocks`); **14.1** in-place edit + single-line relayout
  + the font-on-edit gate + CLI `edit-text` (core surgery); **14.2**
  formatting on a selection — size/colour/gated family (core + CLI);
  **14.3** edit UI on the Pass 12.0 canvas (gui, `pdfce-ui-specialist`
  first). Filed in `ROADMAP.md` "Next up" (new top entry) with the full
  fast-follow ladder (FF-A offline reflow → FF-H spacing/synthetic
  styles) and honest limits named up front.
- **Six standing rules filed, NO collisions: R69–R74** (highest prior was
  R68 — the font-parity harness). In order per decision 014 §5.1: **R69**
  text-edit-is-surgery-not-overlay; **R70**
  text-edit-is-incremental-not-a-scrub; **R71** font-on-edit-trust-ladder;
  **R72** recognized-blocks-and-reflow-are-reviewable-hints; **R73**
  tagged-edits-disclose-never-corrupt; **R74**
  text-model-in-core-edit-UI-in-gui. Added to `ROADMAP.md` Standing rules.
- **`ARCHITECTURE.md` updated**: new **§5.11** (in-place text editing is
  surgery-under-incremental-save, explicitly NOT a fourth member of the
  §5.2/§5.9/§5.10 forced-full-rewrite family — marked DECIDED, pending
  Pass 14.0–14.3 ship, mirroring how §5.10 was written ahead of Pass 13b);
  a new §12 decision-log entry for decision 014 (renumber recorded, all
  six design calls summarized, R69–R74 cross-referenced).
- **Backlog "★ NEXT MAJOR FOCUS" bullet amended (not deleted, append-only
  spirit honored)** — a dated 2026-08-01 note now forward-points to the
  new Pass 14.x "Next up" entry as the live record; the Backlog bullet
  itself stays as the historical directive record. The "~014" placeholder
  references at two other spots in `ROADMAP.md` (the decision-010
  forward-sequence amendment block; the Backlog STATUS sub-bullet) both
  got a matching forward-pointer footnote.
- **Timing note recorded:** all four gating items the operator's
  directive named before this focus could begin — font-supply (decision
  012), the Pass 12.0 canvas substrate, xref-recovery (decision 013), and
  the measurement/dimensioning beta foundation (Pass 12.0, shared) — are
  now ALL SHIPPED (see Pass 13b below). Starting Pass 14.0 is an
  engineering scheduling call, not a blocked prerequisite. The beta's
  remaining vector-selection/basic-editing slices (9a/9c-min) vs. Pass
  14.x ordering remains the flagged, unresolved operator sequencing
  question from continuations 32/33.

**Shipped this session:**
- **Pass 13b — Rebuild-by-scan cross-reference recovery (decision 013
  Pass B) — SHIPPED, CLOSING decision 013.** The #1 real-world robustness
  fix. **Headline: 566 previously-strict-failing real-world files now
  open** (corpus of 1,109: qpdf Apache 639 / pdfium BSD 331 / PDFBox
  Apache 139), reason-bucketed — `NotAnXrefSection` 417, `TrailerParse`
  99, `BadEntry` 20, `BadXrefStream` 13, `StartxrefNotFound` 7,
  `BadStartxrefOffset` 7, `MissingHeader`/offset-start 3. **Zero
  regression** on the 2,907-file veraPDF corpus — 2,892 clean files
  unchanged via the strict path, **0 clean files diverted into
  recovery** (verified by an object-outcome tally, not assumed); 6
  still-failing `BadObject` files unchanged. **The hardest gate —
  `*-fail-*` reconciliation — is COMPLETE:** all 5 veraPDF status changes
  (refused→opens) are PDF/A-conformance files failing a File-header or
  colour-space rule, never an xref-parse bug — defensible reader
  recovery, qpdf/pdfium open the same files too. **Named non-goal
  (unchanged scope):** 53 real-world files with OBJECT-level corruption
  AFTER a clean xref recovery — filed to Backlog as a future Pass
  (object-level lenient loading), not silently absorbed. Encrypted-and-
  refused 58 (unchanged gap); recovery-refused 9 (`NoCatalog` 2,
  `NoObjects` 7). Gates: fuzz 21,595 runs / 0 crashes; fmt/clippy clean;
  `cargo tree` core+render GUI-dep-free; **ZERO new dependency**; 638
  `pdfce-core` lib tests + integration suites green. Demo:
  `add-contents.pdf` opens `(recovered)`, `round-trip --mode full` →
  clean reloadable PDF, incremental refused by name (CLI exit 8),
  recovery-load reports CLI exit 11. Files: `crates/pdfce-core/
  src/recover.rs` (new) + edits to `document.rs`/`objstm.rs`/`xref.rs`/
  `writer/{mod,save}.rs`; `pdfce-cli` exit 11; `pdfce-gui` recovery
  banner; `fixtures/synthetic/xref-recover/` +
  `tools/gen-xref-recover-fixtures.py`; `tests/xref_recover.rs`; fuzz
  `fuzz/fuzz_targets/recover_roundtrip.rs`; `tools/recover-sweep/`.
  **`ARCHITECTURE.md` §5.10 FLIPPED from "pending Pass-13b ship" to
  shipped/active — R67 is now IN FORCE**, not merely filed against
  future code.

**Two engineer flags recorded (NOT actioned by the librarian — code is
the engineer's territory):**
1. **Code-comment number lag, being discharged this session.** `recover.rs`
   cites the recovered-base rule descriptively as "~R62/R59"; the
   canonical number is **R67**. The engineer is fixing this (R59→R67) in
   `recover.rs` this session — recorded here as being discharged, not as
   an outstanding owed item.
2. **`gen-65536` deviation — deliberate, defensible, flagged.** Rebuild-
   by-scan opens some recoverable gen-65536 files via the `BadEntry`
   trigger (one of decision 013's own target buckets — a malformed
   generation number is exactly the entry-corruption shape recovery
   routes around). This is **NOT** the separate strict-parser gen-65536
   TOLERANCE question Pass 13a flagged (Backlog) — the strict parser
   still correctly REJECTS gen 65536 today; only the recovery path
   (which never reads the original malformed entry) opens these files.
   Written up as a new `personal_rag/pdf` lesson (below) since the
   distinction is non-obvious and durable beyond this session.

**RAG escalations this continuation:**
- `C:\personal_rag\pdf\lesson_20260801_gen_65536_recoverable_via_badentry_not_strict_tolerance.md`
  (NEW) — the gen-65536-via-`BadEntry`-recovery vs. strict-parser-
  tolerance distinction (engineer flag 2, above). Subject + master
  indexes updated.

**Still in flight (continuation 34):**
- **Pass 14.0** (editable text model + block recognition) — next
  engineering item once the operator confirms sequencing against the
  beta's remaining 9a/9c-min slices (the flagged, still-open question).
- **Beta (measurement/dimensioning, decision 011)** — foundation (Pass
  12.0) SHIPPED; remaining slices still await operator go-ahead /
  sequencing confirmation relative to Pass 14.x.
- **Pass 5 (Encryption)** — fallback/interleave, unchanged, low-payoff.
- **The autonomous `/loop` remains ACTIVE** (resumed continuation 33,
  unchanged this continuation).

**Still-open operator items (UNCHANGED — re-surfaced, not re-filed;
ordered oldest-first):**
- **Encryption-refusal operator sign-off** — oldest owed.
- **LEGAL.md §1 license decision** — still undecided.
- **LEGAL.md §2 Adobe-supplement copyright contradiction** — flagged.
- **`/R 6` sourcing method** — Ken's call (gates Pass 5).
- **Commit authorization** — everything remains UNCOMMITTED; the tree
  keeps growing (now includes Pass 13b's recovery module + tooling).
- **W15 — no remote/CI** — unchanged.
- **Autosave / crash-recovery scratch file** + **true in-place Save**
  (gated on it) — still open, unchanged.

**For next session:**
- Confirm the operator's sequencing call: Pass 14.0 (Acrobat text-editing)
  now vs. the beta's remaining 9a/9c-min vector slices first.
- Once sequencing is confirmed, start **Pass 14.0** per decision 014 §5.2
  / `ROADMAP.md`'s Pass 14.x entry — provenance-linkage extension of Pass
  4 first, then the Run→Line→Block clustering pass.
- Confirm the engineer's in-session R59→R67 `recover.rs` comment fix
  landed; if not, it remains the one owed code follow-up from this
  continuation.

**Same-day continuation 35 — Pass 14.0 SHIPPED (read-only text model +
block recognition + provenance substrate for 14.1); Pass 14.1 PROMOTED;
autonomous loop still ACTIVE:**

**Shipped:**
- **Pass 14.0 — Editable text model + block recognition (read-only;
  decision 014 Pass 1 of 4), SHIPPED and independently re-verified green
  in the main tree** (all 10 new tests pass: 5 core `text_edit.rs` + 5
  CLI `inspect_text_blocks.rs`, including
  `sourced_view_is_unchanged_by_provenance_capture`, which pins Pass 4's
  output as byte-identical). Full record now in `ROADMAP.md` Shipped
  (above); summary here for the session trail:
  - New `pdfce-core` module `text_edit` (`mod.rs` + `model.rs`): a
    Run→Line→Block recognition pipeline built as a SECOND clustering
    pass over Pass 4's `PageText.runs` — no re-extraction. Lines split at
    Pass 4's `DerivedLineBreak` + a defensive baseline-jump check;
    columns cluster by horizontal overlap then order left-to-right
    (derived §14.8.2.3.1 reading order); blocks/paragraphs break on
    leading-gap or first-line indent. All four thresholds exposed in
    `BlockRecognitionOptions`; every inference counted in
    `BlockDiagnostics`; the sourced-only view is always available via
    `EditableTextModel::sourced_view()`. Everything DERIVED/COUNTED/
    REVIEWABLE (§14.8 S1–S9, R72).
  - Provenance linkage added to the read path (the substrate Pass 14.1
    surgery needs), gated behind `ExtractOptions::capture_provenance`
    (default OFF → Pass 4 output byte-for-byte unchanged). New per-glyph
    fields: show-operator byte span, content-stream ref (page vs. form
    object), font resource name, `Tf` size, fill colour (g/rg/k decoded;
    sc/scn → `Other`, never guessed), text matrix, CTM.
  - CLI: `inspect --text-blocks [--pages …] [--json]` (plain `inspect`
    unchanged, pinned by a regression test). Derived-structure
    disclosures go to stderr; `--json` carries full structure + per-line
    provenance.
  - Fixture: `fixtures/synthetic/textblocks/multi-column.pdf` (CC0
    synthetic, 1,154 bytes, 2 columns × 2 paragraphs × 10 lines; content
    emitted left-then-right to prove geometric ordering; one paragraph in
    blue to exercise colour provenance) + `tools/gen-textblocks-fixtures.py`
    + PROVENANCE.md.
  - **Gates:** `cargo fmt --check` clean; `clippy --workspace --all-targets
    -D warnings` clean (new code uses checked `.get()` per the crate's
    panic-free `#![deny(clippy::indexing_slicing)]` policy); `cargo tree
    -p pdfce-core` / `-p pdfce-render` GUI-dep-free; **no new
    dependency**; full workspace tests green (core lib 645,
    `text_extract` integration 26 UNCHANGED, `text_edit` 5,
    `inspect_text_blocks` 5, render/gui green, 6 doctests).
  - **Public API surface added to `pdfce-core`** (rule-10 API-guidelines
    trail): `text_extract::ContentStreamRef` / `TextColor` /
    `GlyphProvenance` (all #[non_exhaustive]); `ExtractedGlyph.provenance`
    (new field); `ExtractOptions.capture_provenance` + `with_provenance`;
    new `text_edit` module (`EditableTextModel`, `GlyphRef`,
    `TextPosition`, `Line`, `Block`, `BlockKind`, `BlockDiagnostics`,
    `BlockRecognitionOptions`).

**Decisions made this session (continued):**
- **Four engineer judgment calls recorded** (all defensible, none
  blocking — filed to `ROADMAP.md`'s Shipped entry for the API-guidelines
  trail): (1) `ExtractedGlyph` dropped `Copy` (now owns a `Vec` via the
  provenance `Option`), kept `Clone` — technically breaking but zero
  external consumers and every workspace consumer accesses glyphs by
  reference; (2) `TextPosition` uses a **byte** offset (glyph-boundary),
  not the decision record's literal "char-offset" wording, because Pass 4
  already keys glyphs by byte offsets — a UI layer converts to char index
  if needed; (3) fill-colour deliberately partial (device g/rg/k decoded,
  named-space sc/scn → `TextColor::Other`, never guessed to black — rule
  4); (4) `ActualText` runs left atomic (counted, not glyph-split —
  §14.9.4 N4), artifact runs excluded + counted.
- **Pass 14.1 PROMOTED from Next up to In progress** (`ROADMAP.md`) —
  its font-on-edit surgery consumes Pass 14.0's `text_edit` model +
  provenance linkage directly. Status at promotion: spec grounding
  (§9.4.x advance math + inverse encoding) is being sourced in parallel
  by `pdfce-spec-librarian`; 14.1's build starts once that lands. 14.2/
  14.3 remain in Next up, scope unchanged.

**Findings + decisions:**
- **Recurring systemic finding: autonomous-builder worktree isolation +
  the uncommitted git substrate = an empty workspace.** The autonomous
  builder was again launched into an isolated git worktree that lacked
  the uncommitted Pass 1–13 substrate — everything in `crates/` is
  uncommitted on the main tree, so a worktree branched from the bootstrap
  commit (`67967b2`) can't see any of it. The builder worked around it by
  writing the authoritative deliverable to the main tree per its
  instructions; the stale worktree was then removed. This has now bitten
  **multiple** builder dispatches (not a one-off) — written up as a new
  `D:\dev\rag\rust\` finding (below) since it's a general Rust/git-
  worktree + Claude-Code-orchestration gotcha, not PDF-specific. It also
  strengthens the case for the still-pending commit authorization: an
  initial commit would make worktree-based dispatch actually viable
  instead of relying on the "write to main tree" workaround every time.

**Still in flight:**
- **Pass 14.1** — spec grounding in flight (`pdfce-spec-librarian`,
  §9.4.x advance math + inverse encoding); build starts once that lands.
- **Pass 14.2 / 14.3** — queued behind 14.1, unchanged scope.
- **Beta (measurement/dimensioning, decision 011)** — foundation (Pass
  12.0) SHIPPED; remaining slices still await operator go-ahead/
  sequencing confirmation relative to Pass 14.x (unchanged).
- **Pass 5 (Encryption)** — fallback/interleave, unchanged, low-payoff.
- **The autonomous `/loop` remains ACTIVE.**

**Still-open operator items (UNCHANGED — re-surfaced, not re-filed;
ordered oldest-first):**
- Encryption-refusal operator sign-off — oldest owed.
- `LEGAL.md` §1 license decision — still undecided.
- `LEGAL.md` §2 Adobe-supplement copyright contradiction — flagged.
- `/R 6` sourcing method — Ken's call (gates Pass 5).
- Commit authorization — everything remains UNCOMMITTED; the tree keeps
  growing (now includes Pass 14.0's `text_edit` module + fixtures). **This
  session's worktree-isolation finding (above) adds a concrete engineering
  cost to the "still uncommitted" state, beyond the standing risk.**
- W15 — no remote/CI — unchanged.
- Autosave / crash-recovery scratch file + true in-place Save (gated on
  it) — still open, unchanged.

**RAG escalations this continuation:**
- `D:\dev\rag\rust\autonomous_builder_worktree_isolation_uncommitted_substrate.md`
  (NEW) — the worktree-isolation-vs-uncommitted-substrate finding above.
  Subject + master indexes updated.

**For next session:**
- Once `pdfce-spec-librarian`'s §9.4.x spec grounding lands, start Pass
  14.1's build proper (advance-preserving REPLACE interpreter, inverse
  encoding, font-on-edit gate, incremental save + prior-text disclosure).
- Consider whether commit authorization should be revisited given the
  recurring worktree-isolation cost — flagged, not actioned (Ken's call).

**Same-day continuation 36 — Pass 14.1 SHIPPED (in-place text editing
now works); list-authoring Backlog gap filed; alignment adjudicated;
14.2 fill-colour design decision recorded:**

- **Pass 14.1 SHIPPED 2026-08-01** — in-place text editing via
  content-stream surgery + the font-on-edit refusal gate + CLI
  `edit-text` (decision 014 Pass 2 of 4). Full record moved to
  `ROADMAP.md` Shipped (above Pass 14.0's entry). Independently
  re-verified in the main tree: 19 core `text_edit` unit tests + 6 CLI
  `edit_text` integration tests all pass; a live edit ("Hello"→"Hi")
  produced `advance_delta = -16.008` with the Tm-follower repositioned
  and all three disclosures surfaced; a subset-missing glyph was
  refused BY NAME (R-INV-1, exit 9, verbatim Acrobat
  "embedded-but-not-local" framing). **In-place editing of a PDF's own
  page text now works end-to-end for the first time in this project.**
- **What it built:** REMOVE→REPLACE content-stream surgery extending
  Pass 8.0's machinery (REMOVE is the `A_new = 0` case); new
  `text_edit/encoding.rs` (inverse-encoding builder inverting the
  font's OWN resolved `/Encoding` via AGL, never `/ToUnicode` —
  documented non-injective/lossy) + `text_edit/edit.rs` (REPLACE
  surgery, single-line relayout, font-on-edit gate, incremental save);
  advance-delta relayout is REFLOW-default with a `--pin` fallback to
  Pass 8.0's compensating-`TJ` path; CLI `edit-text` subcommand; five
  new synthetic fixtures + generator + PROVENANCE.md.
- **Gates:** fmt/clippy clean (panic-free); `cargo tree` GUI-dep-free
  on core/render; full workspace green (core lib 657 incl. 19 new; 6
  new CLI; Pass 4 + Pass 14.0 tests unchanged); R59 on the edited
  `embedded_full.pdf` fixture = substituted=0/notdef=0/unsupported=0;
  round-trip = edited output is a byte-identical prefix on all five
  test flows.
- **Five engineer judgment calls recorded** (all defensible, none
  blocking — filed to the `ROADMAP.md` Shipped entry for the
  API-guidelines trail): (1) trust levels split across the crate seam —
  core reports `Embedded`/`NonEmbedded` only, CLI refines
  `NonEmbedded`→`Bundled`/`Supplied` via its own `FontEnvironment`
  (keeps core rasterizer-free, R21); (2) `subset = "ABCDEF+"` tag,
  "carried" = codes used on the page (a safe under-approximation,
  disclosed, not an overclaim); (3) anchor = find-in-operator with an
  optional `pinned_span` from Pass 14.0's `GlyphProvenance` — Form-
  XObject content, `'`/`"` anchors, and cross-`TJ`-element matches are
  refused BY NAME (first-cut non-goals); (4) multi-stream pages
  collapse into the first content object + empty extras (disclosed);
  (5) reflow applies `ΔA` to ALL absolute `Tm` operators on the line,
  not just the edited run's own.
- **Follow-up flagged, not yet a Pass:** R-INV-2/3/4 (symbolic-no-
  encoding, ToUnicode-only, composite) are logic-covered in
  `classify_font` but have NO fixture exercising them end-to-end —
  clean scoped follow-up before FF-E/FF-F (composite/CJK/RTL editing)
  is attempted, since those slices depend on the same gate paths.
- **New Backlog gap filed: bulleted/numbered list authoring.** Surfaced
  while scoping Pass 14.2 against
  `D:\Dev\Rag-Specialized\Acrobat_Features\text_edit__formatting_options.md`
  — real Acrobat Edit-PDF behavior with no home anywhere in decision
  014's Pass 14.x family or the FF-A..FF-H ladder, not even as a named
  deferral. It's content AUTHORING (kin to FF-D, but structured), not
  in-place editing of existing runs. Filed to `ROADMAP.md` Backlog as
  an open bucket with no invented Pass number/priority — an **operator
  scope question surfaced to Ken**: do we want list authoring as
  Acrobat parity, and if so, where in the sequence?
- **Alignment ADJUDICATED, not a gap.** The same scoping pass flagged
  text alignment (left/center/right/justified) as possibly unscoped —
  it isn't: decision 014 already covers it, FF-A (left/center/right,
  within-block reflow) + FF-B (adds justified). Alignment only applies
  when a block re-wraps, so it correctly lives in the reflow ladder,
  not Pass 14.2. Recorded as an engineer decision (2026-08-01) in
  `ROADMAP.md` so this was adjudicated, not overlooked.
- **Pass 14.2 fill-colour design decision recorded** (forward-looking,
  attached to the Pass 14.2 Backlog/Next-up bullet ahead of its own
  ship entry): unlike Acrobat, which always stores `DeviceRGB`
  regardless of the picker mode shown, pdfce will let the operator
  choose RGB/CMYK/gray and STORE the actual chosen space (`rg`/`k`/`g`
  respectively) — a minimal-diff parity-plus. Binding constraint: a
  size-only edit must never touch the fill-colour operator at all
  (byte-identical on that operator when colour wasn't part of the
  requested edit).

**Still in flight (continuation 36):**
- **Pass 14.2 (formatting on a selection)** — build dispatched in
  parallel per the operator's `/loop` continuation; not yet landed.
  Will move to Shipped once its own build report arrives.
- **Pass 14.3 (edit UI on the Pass 12.0 canvas)** — queued behind 14.2,
  unchanged scope; needs `pdfce-ui-specialist` dispatched first.
- The R-INV-2/3/4 fixture-coverage follow-up (above) — not yet
  scheduled as a Pass.
- The Beta (measurement/dimensioning) — unchanged, still awaiting
  operator go-ahead/sequencing confirmation relative to Pass 14.x.
- **The autonomous `/loop` remains ACTIVE.**

**Still-open operator items (UNCHANGED — re-surfaced, not re-filed;
ordered oldest-first):**
- Encryption-refusal operator sign-off — oldest owed.
- `LEGAL.md` §1 license decision — still undecided.
- `LEGAL.md` §2 Adobe-supplement copyright contradiction — flagged.
- `/R 6` sourcing method — Ken's call (gates Pass 5).
- Commit authorization — everything remains UNCOMMITTED; the tree keeps
  growing (now includes Pass 14.1's `text_edit::encoding`/`edit`
  modules + fixtures). Unchanged recurring worktree-isolation cost.
- W15 — no remote/CI — unchanged.
- Autosave / crash-recovery scratch file + true in-place Save (gated on
  it) — still open, unchanged.
- **NEW this continuation — list-authoring scope question** (see
  above): does the operator want bulleted/numbered list authoring as
  an Acrobat-parity target, and if so where in the Pass sequence?

**For next session:**
- Land Pass 14.2's build report (formatting on a selection) and file it
  to Shipped once it arrives.
- Dispatch `pdfce-ui-specialist` ahead of Pass 14.3 (edit UI on the
  canvas) per decision 014's prescribed dispatch order.
- Consider scheduling the R-INV-2/3/4 fixture follow-up before FF-E/
  FF-F (composite/CJK/RTL editing) is attempted.
- Get an operator answer on the list-authoring scope question before
  it becomes load-bearing for any Pass sequencing decision.

**Same-day continuation 37 — Pass 14.2 SHIPPED (formatting on a
selection); decision 014's text-editing subsystem now feature-complete
at core/CLI level; Pass 14.3 UI design dispatched in parallel:**

- **Pass 14.2 SHIPPED 2026-08-01** — formatting on a selection: size
  (`Tf`), fill colour (`rg`/`g`/`k`), gated font-family/style change
  (decision 014 Pass 3 of 4). Full record moved to `ROADMAP.md` Shipped
  (directly below Pass 14.1's entry, ahead of Pass 13b). Independently
  re-verified in the main tree: 10 core `text_edit::format` unit tests
  + 11 CLI `format_text` integration tests all pass; a live CMYK colour
  change stored the `k` operator (NOT DeviceRGB) with the parity-plus
  disclosure surfaced; full workspace **1134 passed / 0 failed**.
  **Decision 014's in-place-text-editing subsystem is now
  feature-complete at the core/CLI level** — 14.0 (model) + 14.1 (edit)
  + 14.2 (format) all shipped; only 14.3 (the canvas UI) remains in the
  family.
- **What shipped:** new `crates/pdfce-core/src/text_edit/format.rs`
  (the three ops + `set_format`); `edit.rs` extended with fill-colour
  graphics-state tracking (`FillState`/`DeviceSpace`) added to the
  shared walk across `g`/`rg`/`k`/`cs`/`sc`/`scn`, plus walk/record/
  match/classify/emit/save helpers exposed `pub(crate)` for reuse —
  **14.1's `edit_text` output bytes are unchanged**, its tests pass
  verbatim; `mod.rs` re-exports; CLI `format-text` subcommand +
  `cmd_format_text` + `parse_set_color`; new
  `crates/pdfce-cli/tests/format_text.rs` (11 tests); three new
  fixtures (`format_color`/`format_other`/`format_family`) + generator
  update + PROVENANCE.md, with the 5 existing 14.1 fixtures
  regenerated byte-identical.
- **Mechanism — state-wrap-and-restore emission**, reused by all three
  ops: the anchor operator is split at the matched code-range into
  `pre | mid | post` and re-emitted as `[pre] <state-set> [mid]
  <state-restore> [post]`, so only the anchor operator's bytes change
  and every following operator stays byte-verbatim. Size wraps
  `/F newsize Tf … /F origsize Tf` (fill operator untouched). Colour
  swaps in the chosen device operator then restores the recorded prior
  `FillState` byte-verbatim (advance unaffected). Family swaps
  `/Ftarget Tf` and re-encodes via 14.1's `InverseEncoding` against the
  target's `/Encoding`, gated by 14.1's `classify_font` + the
  embedded-subset carried-codes floor. All three reuse 14.1's
  locate→recompute-advance→relayout→incremental-save pipeline.
- **Fill-colour parity-PLUS demonstrated exactly as designed** (the
  forward-looking decision recorded ahead of this Pass's ship, back in
  continuation 36): the operator picks RGB/CMYK/gray and pdfce STORES
  the actual space (`rg`/`k`/`g`), never force-converted to DeviceRGB
  like Acrobat — disclosed. A run whose original space is non-device
  (`Other`: ICCBased/Separation/DeviceN/Indexed) has its tail restored
  byte-verbatim, with the edited `mid`'s narrowing to device DISCLOSED,
  never silent. Size-only edits never touch the colour operator
  (minimal-diff) — verified by test.
- **The anti-Acrobat-tag-corruption test**: a tagged-run colour change
  keeps `/MCID` 0 and discloses ActualText/tag staleness rather than
  silently invalidating or regenerating the tag
  (`tagged_run_color_change_keeps_mcid_and_discloses`) — this is the
  Pass's structural-tagging correctness proof, parallel to 14.1's
  tagged-run preservation guarantee.
- **Six engineer judgment calls recorded** (all defensible, none
  blocking — filed to the `ROADMAP.md` Shipped entry for the
  API-guidelines trail): (1) state-wrap-and-restore emission chosen for
  robustness — handles substring matches and `TJ`-array matches
  uniformly; (2) the coverage gate for family changes stays
  encoding-level (rasterizer-free, R21) with trust-level
  (Embedded/Bundled/Supplied) staying in the CLI shell, not core; (3)
  non-device (`Other`) colour-space restore uses the recorded raw
  operator bytes rather than re-deriving from a decoded model; (4)
  size- and colour-only edits are deliberately NOT gated by
  R-INV-2/3/4 — a symbolic-no-encoding, ToUnicode-only, or composite
  font can still be resized/recoloured; only a family CHANGE runs the
  full classifier against the target font; (5) family target is
  restricted to an existing page font resource only — no new
  embedding, no resource-dict edit, a missing/new target is a clean
  named refusal pointing at FF-C (font-subsetting/glyph-embedding);
  (6) Pin = trailing compensating `TJ`; Reflow (default) adjusts
  absolute-`Tm` followers by `ΔA`; colour-only edits (`ΔA = 0`) never
  relayout.
- **Honest note (not a defect):** a Calibri-Bold family change
  discloses the R-INV-5 ambiguity for the space character (WinAnsi
  maps space at two codes) — the inverse map picks the lowest code and
  discloses it, the same established fuzzy-never-sneaky behavior 14.1
  already exhibits for other ambiguous mappings.
- **Gates (re-verified main tree):** `cargo fmt --all --check` clean;
  `clippy -p pdfce-core -p pdfce-cli --all-targets -D warnings` clean
  (panic-free); `cargo tree -p pdfce-core` / `-p pdfce-render` zero GUI
  deps; **ZERO new dependency**; full workspace **1134 passed / 0
  failed** (14.1/14.0/Pass-4 tests unchanged); R59 render (notdef=0,
  unsupported=0) + round-trip (reloaded=1) green on all three
  formatted outputs.
- **Pass 14.3 (edit UI on the Pass 12.0 canvas) is next** — the sole
  remaining slice of decision 014's family. `pdfce-ui-specialist`'s
  interaction-design work is being produced IN PARALLEL now (design
  only); 14.3's implementation follows once that design lands. Prereqs
  unchanged: 14.0–14.2 (all now shipped) + Pass 12.0.
- **The autonomous `/loop` remains ACTIVE.**

**Still in flight (continuation 37):**
- **Pass 14.3** — UI-specialist interaction-design dispatch in
  progress; implementation not started; will move to In progress once
  design lands and build begins, then to Shipped on its own build
  report.
- The R-INV-2/3/4 fixture-coverage follow-up (flagged at Pass 14.1) —
  still not yet scheduled as a Pass.
- The Beta (measurement/dimensioning) — unchanged, still awaiting
  operator go-ahead/sequencing confirmation relative to Pass 14.x.
- Everything remains UNCOMMITTED in git — the tree keeps growing (now
  includes Pass 14.2's `text_edit::format` module + fixtures).

**Still-open operator items (UNCHANGED — re-surfaced, not re-filed;
ordered oldest-first):**
- Encryption-refusal operator sign-off — oldest owed.
- `LEGAL.md` §1 license decision — still undecided.
- `LEGAL.md` §2 Adobe-supplement copyright contradiction — flagged.
- `/R 6` sourcing method — Ken's call (gates Pass 5).
- Commit authorization — everything remains UNCOMMITTED; the tree keeps
  growing (now includes Pass 14.2's `text_edit::format` module +
  fixtures). Unchanged recurring worktree-isolation cost.
- W15 — no remote/CI — unchanged.
- Autosave / crash-recovery scratch file + true in-place Save (gated on
  it) — still open, unchanged.
- List-authoring scope question (filed continuation 36) — does the
  operator want bulleted/numbered list authoring as an Acrobat-parity
  target, and if so where in the Pass sequence? Still no operator
  answer.

**For next session:**
- Land Pass 14.3's UI-specialist design output, then dispatch the
  implementation build; file it to Shipped once it arrives.
- Consider scheduling the R-INV-2/3/4 fixture follow-up before FF-E/
  FF-F (composite/CJK/RTL editing) is attempted.
- Get an operator answer on the list-authoring scope question before
  it becomes load-bearing for any Pass sequencing decision.
- Re-surface commit authorization and the license decision — both
  remain the two oldest-standing, highest-leverage unresolved operator
  items in the project.

**Same-day continuation 38 — Pass 14.3 SHIPPED (on-canvas text-editing
UI + the `EditSession` undo-integration prerequisite, decision 014 Pass
4 of 4, FINAL SLICE); decision 014's Acrobat in-place text-editing
subsystem is now COMPLETE end-to-end; FF-A scoped + KenAgent decision
015 opened, including a flagged justified-alignment question:**

- **Pass 14.3 SHIPPED 2026-08-01** — the FINAL slice of decision 014.
  Full record moved to `ROADMAP.md` Shipped (directly below Pass
  14.2's entry, ahead of Pass 13b). Independently re-verified in the
  main tree: 39 core `text_edit` tests (incl. new `EditSession` edit/
  format commands, undo/redo, and a byte-identical-to-free-function
  minimal-diff proof) + 50 GUI tests all pass; release GUI builds and
  launches without panic. The GUI was launched live with the
  multi-column fixture, Edit Text tool / `Ctrl+E` operable.
- **Core — the blocking §0.2 `EditSession` undo-integration
  prerequisite, discharged this Pass:** `text_edit/edit.rs`'s
  `edit_text` surgery split into `plan_edit(...) -> EditPlan` (used by
  both the free function and the session path) + `write_incremental`;
  the matching `plan_format(...) -> FormatPlan` split in `format.rs`;
  `model.rs` gained `line_at`/`word_range_at`/`line_range_at` (+
  `word_bounds`) accessors + 4 tests; `edit.rs` gained
  `EditSession::edit_text`/`format_text`, `current_page_content`,
  `text_edit_command`, `CommandKind::{EditText, FormatText}` + 6
  session tests. Each edit is ONE undo-able command over the session's
  in-memory object graph; multi-edit accumulation walks the session's
  staged content; **proven byte-identical to the free function for a
  single edit.** The free functions are behaviorally UNCHANGED — 14.1's
  and 14.2's tests pass verbatim.
- **Render/CLI hoist:** `FontEnvironment::subset_stem` +
  `classify_nonembedded` shared between `pdfce-render` and
  `pdfce-cli`, deleting the CLI's private duplicate — no behavior
  change.
- **GUI — the first slice with a real `CanvasTool` variant.**
  `CanvasTool::TextEdit` (previously-synthetic `resolve_escape`/
  `canvas_suppresses_pan`/gesture-interrupt branches now actually
  fire); `TextEditState`/`PendingEdit` wiring in `main.rs`; ~30 new
  `ui_text.rs` strings. Shipped the full P0 spine: click→caret,
  Shift-click extend, double-click→word, rotation/zoom-correct
  caret+selection rendering (first live consumers of the Pass-12.0
  `canvas_to_pdf_space`/`pdf_space_to_canvas` bridges), live preview
  (mask + draft text in an egui font + a dashed "PREVIEW — not yet
  applied" tag), real Accept/Reject buttons, the verbatim disclosure
  strip + refusal strip (§8.2 "what would lift it" table), cross-run
  refusal, a read-only block-boundary review overlay (split/merge/
  reorder named as a deferred non-goal), and the property bar (size /
  colour-model RGB-CMYK-Gray / font `ComboBox`, trust-labelled).
- **Named simplifications (deferred, substrate already shipped):**
  selection-replace-on-type (insert+backspace at caret, not a single
  replace op); triple-click/drag-select/arrow-Home-End caret nav (the
  `line_at`/`line_range_at` accessors Home/End needs are shipped and
  tested, ready to wire); property-bar edits apply via an explicit
  "Apply" button rather than commit-on-focus-loss.
- **Five judgment calls recorded** (filed to the `ROADMAP.md` Shipped
  entry): (1) "free functions unchanged" read as behaviorally
  unchanged (mechanical `plan_edit`/`plan_format` extraction, 14.1/14.2
  tests verbatim); (2) multi-edit accumulation walks the session's
  staged content, a first-edit-gated extra-stream-emptying step keeps
  undo clean; (3) session methods surface `text_edit::EditError`/
  `FormatError` directly, not wrapped in a session-local error type;
  (4) the preview draws draft text in an egui font (no new
  font-shaping dependency); (5) the delegated GUI sub-fork returned 0
  tool-uses, so the builder implemented the GUI directly.
- **Gates (re-verified main tree):** `cargo fmt --all --check` clean;
  `clippy --workspace --all-targets -D warnings` clean; `cargo tree -p
  pdfce-core` / `-p pdfce-render` still zero egui/eframe/winit/wgpu/
  glow (GUI-core separation intact); `cargo test --workspace` 23/23
  binaries, 0 failures (677 core incl. §0.2 tests, 42 gui incl. new
  canvas tests per the build report; independently re-run at 39 core
  `text_edit` + 50 gui, all green); R59 + round-trip green; GUI release
  build launches, no startup panic; **ZERO new dependency**.
- **MILESTONE — decision-014's Acrobat in-place-text-editing subsystem
  is now COMPLETE end-to-end** (core model → in-place edit →
  formatting → GUI edit tool, 14.0 through 14.3). The operator's
  directed "Acrobat text-handling parity" focus (Backlog's ★ NEXT MAJOR
  FOCUS, filed 2026-08-01) is **substantially achieved at the P0
  level**. The GUI was launched for the operator. Remaining
  text-parity work is the reflow ladder (FF-A within-block next, FF-B
  cross-block after) plus the named GUI refinements above.
- **FF-A scoped; KenAgent decision 015 opened.**
  `pdfce-acrobat-librarian` scoped FF-A (within-block reflow) into
  `D:\Dev\Rag-Specialized\Acrobat_Features\text_edit__paragraph_reflow_and_auto_adjust_layout.md`.
  A KenAgent decision (**015**) is being taken to settle FF-A's
  architecture (rule 12: parity reference → scope → KenAgent decision
  → build). **Two FF-A differentiators the scoping surfaced:** (1)
  alignment auto-detect + preserve through reflow (never reset to a
  default); (2) never silently drop page-overflowed content
  (fuzzy-never-sneaky — disclose, don't truncate).
- **FLAGGED OPEN QUESTION for decision 015, NOT DEFAULTED — the
  justified-alignment tension.** Acrobat exposes a **Justify** button
  on its BASE (non-cloud) Edit-Text panel, in tension with decision
  014's working assumption that justified alignment needs FF-B
  (cross-block reflow). Whether Acrobat's classic Justify does a true
  re-wrap-and-distribute (meaning justified could ship at FF-A, ahead
  of FF-B) or only a lighter per-line nudge (consistent with the
  original FF-A/FF-B split) is **unresolved** and must be decided by
  015, not assumed. Filed to `ROADMAP.md`'s ★ NEXT MAJOR FOCUS entry
  (Next up) and the Backlog bucket's amendment chain.
- **The autonomous `/loop` remains ACTIVE.**

**Still in flight (continuation 38):**
- **KenAgent decision 015** — FF-A architecture, including the
  justified-alignment question — not yet decided.
- The Beta (measurement/dimensioning) — unchanged, still awaiting
  operator go-ahead/sequencing confirmation.
- The R-INV-2/3/4 fixture-coverage follow-up (flagged at Pass 14.1) —
  still not yet scheduled as a Pass.
- The list-authoring scope question (filed continuation 36) — still no
  operator answer.
- Everything remains UNCOMMITTED in git — the tree keeps growing (now
  includes Pass 14.3's `EditSession` text-edit/format commands + GUI
  `CanvasTool::TextEdit` + fixtures) and is now VERY LARGE, compounding
  the recurring worktree-isolation cost on every autonomous-builder
  dispatch.

**Still-open operator items (re-surfaced, ordered oldest-first; one
addition this continuation):**
- Encryption-refusal operator sign-off — oldest owed.
- `LEGAL.md` §1 license decision — still undecided.
- `LEGAL.md` §2 Adobe-supplement copyright contradiction — flagged.
- `/R 6` sourcing method — Ken's call (gates Pass 5).
- Commit authorization — everything remains UNCOMMITTED; the tree is
  now VERY LARGE (23 test binaries, the full decision-014 text-editing
  subsystem, the whole shipped-Pass history) — the worktree-isolation
  workaround cost compounds with every additional Pass. Highest-leverage
  unresolved item alongside the license decision.
- W15 — no remote/CI — unchanged.
- Autosave / crash-recovery scratch file + true in-place Save (gated on
  it) — still open, unchanged.
- List-authoring scope question (filed continuation 36) — does the
  operator want bulleted/numbered list authoring as an Acrobat-parity
  target, and if so where in the Pass sequence? Still no operator
  answer.
- **NEW — justified-alignment question (filed this continuation,
  2026-08-01):** does Acrobat's base-panel Justify button do a true
  re-wrap (implying justified is FF-A-reachable) or a lighter nudge
  (consistent with the original FF-B gating)? Needed to scope KenAgent
  decision 015.

**For next session:**
- Take KenAgent decision 015 (FF-A architecture) — resolve the
  justified-alignment question as part of it, not as an afterthought.
- Re-surface commit authorization and the license decision — both
  remain the two oldest-standing, highest-leverage unresolved operator
  items in the project, now more pointed given the tree's size.
- Consider scheduling the R-INV-2/3/4 fixture follow-up before FF-E/
  FF-F (composite/CJK/RTL editing) is attempted.
- Get an operator answer on the list-authoring scope question before
  it becomes load-bearing for any Pass sequencing decision.
- Confirm the beta (measurement/dimensioning) sequencing relative to
  the now-complete Pass 14.x family and the upcoming FF-A work.

**Same-day continuation 39 — KenAgent decision 015 filed (FF-A: within-block
offline reflow); justified-alignment question RESOLVED (relocated FF-B →
FF-A); decision 014 amended; R75–R77 assigned; new ★ Pass 15.x reflow family
filed; ARCHITECTURE §5.11 flipped to shipped; Pass 15.0 dispatched to build:**

- **Decision 015 ACCEPTED** via the KenAgent protocol. Full record:
  `docs/decisions/015-ffa-within-block-offline-reflow.md`. Scopes FF-A
  (decision 014's fast-follow ladder) as the active thread now that Pass
  14.0–14.3 is complete end-to-end.
- **The justified-alignment open question (flagged continuation 38) is
  RESOLVED, not left open.** Acrobat's Justify button sits on the BASE
  (non-cloud) Edit-Text panel — proof it is a classic-engine, single-block
  capability. Decision 015 §3.1 moves justified OUT of FF-B and INTO FF-A
  as a fourth within-block alignment mode (peer of left/center/right); FF-B
  narrows to cross-block + cross-page offline reflow only (the genuine
  exceed-Acrobat headline, since Acrobat's cross-block reflow is
  cloud-gated + English-only).
- **decision 014 AMENDED (not rewritten).** Dated footnotes/pointers added
  at `docs/decisions/014-acrobat-text-editing.md`'s header ("Amended by"
  line), §3 (Reflow paragraph), §5.3 (fast-follow ladder), and §6
  (justified/Knuth-Plass bullet) — each marks the justified-relocation and
  points at decision 015 §3.1/§6. The original 014 prose is left in place;
  nothing was deleted.
- **New standing rules R75–R77 filed** (ceiling was R74, no collisions):
  **R75** reflow-is-explicit-reviewable-single-block-one-undo-command
  (Pass 14.1's single-line relayout stays the default; reflow is opt-in);
  **R76** reflow-overflow-discloses-never-disappears (off-page content
  emitted as real recoverable content, never clipped-to-deleted — a
  deliberate divergence from Acrobat's own documented silent-disappear
  behavior); **R77** alignment-auto-detected-and-preserved-through-rewrap
  (counted, operator-overridable; single-line block defaults to left +
  disclosed ambiguity). **Kept as three separate rules, not folded** — R77
  was NOT folded into R75 (the decision left this to librarian discretion);
  each names a genuinely distinct invariant (operation shape/scope,
  overflow disclosure, alignment fidelity), matching the granularity
  decision 014's own six rules (R69–R74) already established for this
  family.
- **New Pass family filed: ★ Pass 15.x (assigned FRESH, not folded into
  14.4–14.6 — decision 015 §6 explicitly delegated this call).** Keeps
  "Pass 14.x = in-place editing" and "Pass 15.x = reflow" as two coherent,
  separately-citable families, the same precedent set when 14.x itself was
  assigned fresh after 13a/13b had already taken 13.x. Filed to
  `ROADMAP.md` "Next up", directly after the Pass 14.x entry:
  - **15.0 — Alignment inference + within-block greedy re-wrap engine
    (core, READ-ONLY, CLI inspect). DISPATCHED TO BUILD NOW.** `ReflowEngine`
    building a `ReflowPreview` via a greedy breaker factored out of
    `vartext.rs`'s packing core (provenance-§9.4.4-advance measurer, one
    breaker two callers); alignment auto-detect from Pass 14.0's x-band
    geometry; `pdfce-cli inspect --reflow-preview`. No write; no UI;
    single-block only.
  - **15.1 — Reflow surgery + one undo-able `CommandKind::ReflowBlock`
    (core + CLI).** Applies an accepted preview via 14.1's advance-preserving
    surgery; justified slack via `TJ` (§9.4.3) / `Tw` (§9.3.3); default
    incremental save; page-overflow disclose-and-allow (R76); `pdfce-cli
    edit-text --reflow`. Prereqs: 15.0, Pass 14.1; a `pdfce-spec-librarian`
    dispatch queued for §9.4.3/§9.3.3.
  - **15.2 — Reflow UI on the Pass 12.0 canvas (gui).** Preview overlay,
    width/alignment drag-adjust, accept/reject. Prereqs: 15.0–15.1 + Pass
    14.3; `pdfce-ui-specialist` dispatched first.
  - Also amended the ROADMAP's Pass-14.x "Fast-follow ladder" bullet, the
    "Alignment cross-reference — adjudicated" note, the "OPEN QUESTION
    flagged for decision 015" note, and the Backlog ★ NEXT MAJOR FOCUS
    bucket's amendment chain — each got a dated footnote pointing at the
    new ★ Pass 15.x entry rather than being silently rewritten.
- **`ARCHITECTURE.md` §5.11 FLIPPED from "pending Pass 14.0–14.3 ship" to
  the actual shipped module layout** (an owed fix — all four slices have
  been shipped since Pass 14.3, but §5.11 still read as forward-looking).
  Now documents: `text_edit/model.rs` (Run→Line→Block, `BlockDiagnostics`,
  the Pass-14.3 navigation accessors), `text_edit/edit.rs` (the
  REMOVE→REPLACE surgery, the font-on-edit gate, AND the `EditSession`
  undo-integration — `plan_edit`/`plan_format`/`write_incremental`,
  `CommandKind::{EditText, FormatText}`), `text_edit/format.rs`
  (formatting-on-selection), and the Pass 14.3 GUI layer
  (`CanvasTool::TextEdit`, `TextEditState`/`PendingEdit`, `ui_text.rs`).
  Reconfirmed explicitly: text editing is **surgery-under-incremental-save,
  NOT a fourth forced-full-rewrite sibling** (R34/R70) — a content CHANGE,
  not a removal, distinct from R35/R58/R67's family. Also added a forward
  pointer from §5.11 to the new Pass 15.x reflow family, and a new §12
  decision-log entry for decision 015 (cross-referencing the decision file,
  R75–R77, Pass 15.x, and the 014 amendment).
- **Pass 15.0 (read-only reflow engine) is DISPATCHED TO BUILD NOW.**
- **The autonomous `/loop` remains ACTIVE.**

**Still in flight (continuation 39):**
- **Pass 15.0** — dispatched to build; not yet shipped.
- **Pass 15.1/15.2** — scoped, not yet started; 15.1 needs a
  `pdfce-spec-librarian` dispatch for §9.4.3/§9.3.3 first; 15.2 needs
  `pdfce-ui-specialist` first.
- The Beta (measurement/dimensioning) — unchanged, still awaiting operator
  go-ahead/sequencing confirmation relative to Pass 15.x.
- The R-INV-2/3/4 fixture-coverage follow-up (flagged at Pass 14.1) — still
  not yet scheduled as a Pass.
- The list-authoring scope question (filed continuation 36) — still no
  operator answer.
- Everything remains UNCOMMITTED in git — the tree keeps growing (now also
  includes decision 015's record and whatever Pass 15.0 lands) and is VERY
  LARGE, compounding the recurring worktree-isolation cost on every
  autonomous-builder dispatch.

**Still-open operator items (re-surfaced, ordered oldest-first — unchanged
in substance this continuation, no new items):**
- Encryption-refusal operator sign-off — oldest owed.
- `LEGAL.md` §1 license decision — still undecided.
- `LEGAL.md` §2 Adobe-supplement copyright contradiction — flagged.
- `/R 6` sourcing method — Ken's call (gates Pass 5).
- **Commit authorization — now especially pointed.** Everything remains
  UNCOMMITTED; the tree is VERY LARGE (23+ test binaries, the full
  decision-014 text-editing subsystem, decision 015's reflow work about to
  land, the whole shipped-Pass history) — the worktree-isolation workaround
  cost compounds with every additional Pass, and Pass 15.x will add more.
  Highest-leverage unresolved item alongside the license decision.
- W15 — no remote/CI — unchanged.
- Autosave / crash-recovery scratch file + true in-place Save (gated on
  it) — still open, unchanged.
- List-authoring scope question (filed continuation 36) — does the
  operator want bulleted/numbered list authoring as an Acrobat-parity
  target, and if so where in the Pass sequence? Still no operator answer.
- Justified-alignment question — **RESOLVED this continuation** by decision
  015 (relocated FF-B → FF-A); struck from this list going forward.

**For next session:**
- Ship Pass 15.0 (read-only reflow engine); verify the acceptance criteria
  in decision 015 §6 / `ROADMAP.md`'s ★ Pass 15.x entry (greedy wrap
  matches hand-computed breaks; L/C/R/justified inferred correctly;
  single-line → left + disclosed; oversized word → one overflowing line +
  disclosure; no write; `cargo tree -p pdfce-core` clean; Pass 14.0 tests
  unchanged; fmt/clippy clean).
- Dispatch `pdfce-spec-librarian` for §9.4.3 `TJ` / §9.3.3 `Tw` ahead of
  Pass 15.1.
- Re-surface commit authorization and the license decision — both remain
  the two oldest-standing, highest-leverage unresolved operator items,
  now more pointed than last continuation given the tree's continued
  growth.
- Get an operator answer on the list-authoring scope question.
- Confirm the beta (measurement/dimensioning) sequencing relative to Pass
  15.x.

**Same-day continuation 40 — Pass 15.0 SHIPPED (read-only within-block
reflow engine + alignment auto-detect, FF-A slice 1 of 3); Pass 15.1
PROMOTED; autonomous loop remains ACTIVE:**

**Shipped:**
- **Pass 15.0 — Within-block greedy reflow engine + alignment
  auto-detect (read-only; decision 015, FF-A slice 1 of 3), SHIPPED and
  independently re-verified green in the main tree** (11 core
  `reflow::tests` + 11 CLI `inspect_reflow_preview` tests pass;
  **vartext's 17 tests pass unchanged**, confirming the greedy-core
  factoring preserved behavior; the demo detected left/right/center/
  justified correctly across the 4 fixture pages with new bboxes/
  height-deltas computed). Full record now in `ROADMAP.md` Shipped
  (above); summary here for the session trail:
  - NEW `crates/pdfce-core/src/linebreak.rs` — the factored shared
    greedy first-fit breaker `greedy_pack(word_count, max_width,
    line_width_closure) -> Vec<Range<usize>>` (pure index arithmetic +
    a width-measuring closure). NEW
    `crates/pdfce-core/src/text_edit/reflow.rs` (~1030 lines, 11
    tests) — the `ReflowEngine` + alignment auto-detect +
    `ReflowPreview`. `vartext.rs`'s `wrap_lines` now calls
    `greedy_pack` with an Std14-AFM measurer — byte-for-byte identical
    output, all 17 vartext tests pass verbatim. `lib.rs` gains `pub mod
    linebreak`; `text_edit/mod.rs` re-exports.
  - CLI: `inspect --reflow-preview --block N --width W [--align
    L|R|C|J] [--leading pt] [--json]`. Fixture:
    `fixtures/synthetic/reflow/reflow.pdf` (5-page Courier synthetic:
    left/right/center/justified + a small page proving computed
    page-overflow) + `tools/gen-reflow-fixtures.py` + PROVENANCE.md.
    Tests: `crates/pdfce-cli/tests/inspect_reflow_preview.rs` (11
    tests).
  - ONE greedy breaker, two callers (vartext = AFM measurer; reflow =
    provenance §9.4.4-advance measurer via `ExtractedGlyph::advance` —
    no font re-measurement). Whitespace-only breaks, no hyphenation.
    READ-ONLY: no surgery/session/save path exists in 15.0.
  - Alignment auto-detect from the 14.0 line boxes: per-line left/
    right/mid edges, `tol = max(2.0, 0.5·size)` pt; priority
    Justified(n≥3, left-flush + all-but-last right-flush + short
    last) → Left → Right → Center → Left/Ambiguous; single-line →
    Left/SingleLineDefault. Counted + disclosed + overridable.
    Justified preview computes per-line slack; last line never
    justified.
  - Page-overflow COMPUTED + disclosed here (all lines still computed
    with negative baselines), applied/enforced in 15.1 (R76).
  - **Gates:** `cargo fmt --all --check` clean; `clippy --workspace
    --all-targets --all-features -D warnings` clean; `cargo test
    --workspace` all green (core lib 694; CLI reflow-preview 11;
    text-blocks 5 UNCHANGED; edit/format/undo/render all pass;
    doctests incl. `greedy_pack` + `ReflowRequest::new`); `cargo tree`
    core/render GUI-dep-free; **ZERO new dependency** (no
    `Cargo.toml`/`Cargo.lock` touched); Pass 14.x + vartext tests
    unchanged.
  - **Public API surface added to `pdfce-core`** (rule-10
    API-guidelines trail): `ReflowEngine<'m, 'a>` (new/
    detect_alignment/preview); `enum BlockAlignment { Left, Right,
    Center, Justified }` (as_str/parse/is_justified); `enum
    AlignmentSource { Detected, SingleLineDefault, AmbiguousDefault,
    Overridden }`; `DetectedAlignment`; `ReflowLine`; `PageOverflow`;
    `ReflowDiagnostics`; `ReflowPreview` (+ `height_delta()`);
    `ReflowRequest` (builders new/with_wrap_width[_opt]/
    with_alignment[_opt]/with_leading[_opt]/with_page_cropbox); `enum
    ReflowError { BlockIndexOutOfRange, EmptyBlock, BadWidth }`
    (`thiserror`). `#[non_exhaustive]` on options/outputs; builders
    exist because `#[non_exhaustive]` blocks cross-crate literals.

**Decisions made this session (continued):**
- **Engineer judgment call recorded** (defensible, non-blocking —
  filed to `ROADMAP.md`'s Shipped entry for the API-guidelines trail):
  a right/center/justified paragraph has ragged LEFT edges, which Pass
  14.0's first-line-indent recognizer rule would fragment into
  single-line blocks; reflow therefore recognizes with
  **indent-splitting relaxed** (`indent_ratio` pushed out of practical
  reach; leading-gap splitting kept unchanged) —
  `reflow_recognition_options()` in the CLI / `recognise_relaxed` in
  tests, documented at both call sites. Left/justified (flush-left)
  paragraphs are unaffected. Threshold constants named + documented as
  corpus-tunable (decision 015 §10 revisit trigger 2).
- **Pass 15.1 PROMOTED from Next up to In progress** (`ROADMAP.md`) —
  its reflow surgery consumes Pass 15.0's `ReflowEngine`/
  `ReflowPreview` directly, the same direct-prerequisite promotion
  pattern used for every prior slice in this family. Status at
  promotion: `pdfce-spec-librarian` is sourcing §9.4.3 `TJ` numeric-
  position-adjustment distribution + the §9.3.3 `Tw` single-byte-
  code-32 caveat **in parallel with** 15.1's build start (the operator
  explicitly authorized starting the build without waiting on the spec
  dispatch to land first, only that its findings be confirmed before
  the justified-slack surgery path is finalized). 15.2 (reflow UI)
  remains in Next up, unscheduled until 15.0–15.1 + Pass 14.3 are all
  consumed by it.

**Findings + decisions:**
- No new generalizable Rust/egui or PDF-domain finding this
  continuation beyond what's already captured — the greedy-breaker
  factoring and the indent-relaxation call are both pdfce-internal
  engineering judgment calls, not ecosystem- or domain-generalizable
  discoveries, so nothing new was filed to `D:\dev\rag\rust\`,
  `D:\dev\rag\egui\`, or `C:\personal_rag\pdf\` this continuation.

**Still in flight (continuation 40):**
- **Pass 15.1** — promoted to In progress; not yet started building;
  `pdfce-spec-librarian`'s §9.4.3/§9.3.3 dispatch running in parallel.
- **Pass 15.2** — scoped, not yet started; needs `pdfce-ui-specialist`
  first; also needs 15.0–15.1 + Pass 14.3 (already shipped) consumed.
- The Beta (measurement/dimensioning) — unchanged, still awaiting
  operator go-ahead/sequencing confirmation relative to Pass 15.x.
- The R-INV-2/3/4 fixture-coverage follow-up (flagged at Pass 14.1) —
  still not yet scheduled as a Pass.
- The list-authoring scope question (filed continuation 36) — still no
  operator answer.
- Everything remains UNCOMMITTED in git — the tree keeps growing (now
  also includes Pass 15.0's `linebreak.rs`/`reflow.rs` + fixtures) and
  is VERY LARGE, compounding the recurring worktree-isolation cost on
  every autonomous-builder dispatch.
- The autonomous `/loop` remains ACTIVE — Pass 15.1 (reflow surgery) is
  next.

**Still-open operator items (re-surfaced, ordered oldest-first — one
item's framing escalated, no items newly added or resolved this
continuation):**
- Encryption-refusal operator sign-off — oldest owed.
- `LEGAL.md` §1 license decision — still undecided.
- `LEGAL.md` §2 Adobe-supplement copyright contradiction — flagged.
- `/R 6` sourcing method — Ken's call (gates Pass 5).
- **Commit authorization — escalating further this continuation.**
  Everything remains UNCOMMITTED; the tree is VERY LARGE and grew
  again this continuation (Pass 15.0's reflow engine + fixtures on top
  of the full decision-014 text-editing subsystem and the entire
  shipped-Pass history) — the worktree-isolation workaround cost
  compounds with every additional Pass, and Pass 15.1/15.2 will add
  more still. Remains the highest-leverage unresolved item alongside
  the license decision; flagging again per the standing instruction to
  keep escalating the framing as the tree keeps growing, not just
  repeat the same wording.
- W15 — no remote/CI — unchanged.
- Autosave / crash-recovery scratch file + true in-place Save (gated on
  it) — still open, unchanged.
- List-authoring scope question (filed continuation 36) — does the
  operator want bulleted/numbered list authoring as an Acrobat-parity
  target, and if so where in the Pass sequence? Still no operator
  answer.
- Justified-alignment question — remains RESOLVED (decision 015,
  continuation 39); not re-opened, listed here only for continuity of
  the oldest-first trail, no action needed.

**For next session:**
- Build and ship Pass 15.1 (reflow surgery + `CommandKind::ReflowBlock`);
  confirm `pdfce-spec-librarian`'s §9.4.3 `TJ` / §9.3.3 `Tw` findings
  before finalizing the justified-slack distribution path; verify the
  acceptance criteria in decision 015 §6 / `ROADMAP.md`'s ★ Pass 15.x
  entry (re-wrap correctness on embedded-full and non-embedded blocks;
  only the block's own content-stream object changed; incremental-
  save-safe; justified slack correct with last line un-justified;
  page-overflow disclosed never clipped; undo restores byte-identical
  pre-reflow stream; R59 + round-trip green; fmt/clippy clean).
- Re-surface commit authorization and the license decision — both
  remain the two oldest-standing, highest-leverage unresolved operator
  items, now more pointed than last continuation given the tree's
  continued growth.
- Get an operator answer on the list-authoring scope question.
- Confirm the beta (measurement/dimensioning) sequencing relative to
  Pass 15.x.

**Same-day continuation 41 — Pass 15.1 SHIPPED (reflow now APPLIES —
justified re-wrap demonstrated with correct right-flush + un-stretched
last line, undo byte-identical, overflow emitted-not-clipped, composite
refused); Pass 15.2 PROMOTED; only 15.2 remains to complete FF-A;
autonomous loop remains ACTIVE:**

**Shipped:**
- **Pass 15.1 — Reflow surgery + one undo-able
  `CommandKind::ReflowBlock` + CLI `reflow` (decision 015, FF-A slice 2
  of 3), SHIPPED and independently re-verified green in the main tree**
  (6 core `reflow_apply` + 5 CLI `reflow` + 2 render tests pass; a live
  justified reflow on page 4 at width 180 re-wrapped 4→5 lines with 4
  justified lines + an un-stretched last line, only the block's content
  object changed, round-trip reports `identical=1, raster_identical=1,
  reloaded=1`). Full record now in `ROADMAP.md` Shipped (above); summary
  here for the session trail:
  - NEW `crates/pdfce-core/src/text_edit/reflow_apply.rs` (~660 LoC +
    tests) — re-emits a block's show operators at the new line
    origins/breaks via Pass 14.1's advance-preserving machinery
    (`emit_tm`/`splice`/`write_incremental`/`make_raw_stream`/§9.4.4
    advance) from a Pass 15.0 `ReflowPreview`. CHANGED `reflow.rs`
    (`WordTok` carries source `codes`; `tokenise_block` now
    `pub(crate)` — 15.0's preview behaviour byte-unchanged), `mod.rs`
    (re-exports), `edit.rs` (`CommandKind::ReflowBlock { lines_before,
    lines_after }` + `EditSession::reflow_block`, mirroring 14.3's
    plan/effect split), `pdfce-cli/src/main.rs` (`reflow` subcommand,
    `--page`/`--block`/`--width`/`--align`/`--leading`),
    `pdfce-render/src/lib.rs` (2 new R59 tests), `pdfce-gui/src/
    ui_text.rs` (undo label). NEW `pdfce-cli/tests/reflow.rs` (5
    tests). Reused `fixtures/synthetic/reflow/reflow.pdf`
    (`PROVENANCE.md` updated).
  - Justify (`TJ` general path): per full non-last line `N_gap =
    −(S/G)·1000/emit_scale`, `emit_scale = Tfs·Th·a·ca`, emitted as one
    `[ (w0 SP) N (w1 SP) … (wlast) ] TJ` with the original code-32
    spaces kept + `0 Tw` set once — sign-mirror of 14.1's compensating-
    `TJ` pin. Last line and single-word lines never stretched. Justify
    with non-zero `Tc`/`Tw` refused-and-disclosed. `Tw`-word-spacing
    documented as the non-goal alternative (can't serve composite;
    leaks into the last line).
  - Line origin = recipe C (absolute `Tm` per line, whole block
    re-emitted as one fresh `BT…ET`) — immune to the §3.1 relative-`Td`
    re-basing bug, drives L/C/R/justified uniformly. `(a,b,c,d)` from
    provenance text-matrix; `(e,f)` from the preview origin through the
    axis-aligned CTM.
  - Codes carried, never re-encoded (only R-INV-4/composite applies).
    Page-overflow (R76): all lines emitted at true position, never
    clipped, disclosed. Tagged (R72): block's own `BT…ET` re-emitted
    preserving the enclosing `BDC`/`EMC` + `MCID` by construction.
    Incremental save (R34): only the block's own content object
    re-emitted.
  - **Gates:** `cargo fmt --all --check` clean; `clippy --workspace
    --all-targets --all-features -D warnings` clean; `cargo tree`
    core/render GUI-dep-free; full workspace green, 25 ok-blocks 0
    failures (core lib 702; CLI reflow 5; render reflow 2); R59 on
    reflowed output (real glyphs, `unknown_ops=0`, justified ink
    reaches box right margin); round-trip `identical=1
    raster_identical=1`; **ZERO new dependency**; Pass 14.x/15.0/
    vartext tests unchanged.
  - **Public API surface added to `pdfce-core`** (rule-10
    API-guidelines trail): `apply_reflow(&Document, page_index,
    block_index, &ReflowRequest) -> Result<ReflowOutcome,
    ReflowApplyError>`; `ReflowOutcome { bytes, report }`;
    `ReflowApplyReport { block_index, lines_before/after, alignment,
    justified_lines, base_font, glyph_source, tagged_mcid,
    height_delta, overflow, content_object, extra_objects_emptied,
    disclosures }`; `ReflowApplyError` (`thiserror`,
    `#[non_exhaustive]`); `EditSession::reflow_block(page_index,
    block_index, &ReflowRequest)`; `CommandKind::ReflowBlock
    { lines_before, lines_after }`. ISO-cited doc comments (§9.4.3/
    §9.3.3/§9.4.2/§9.3.5 + R-INV/R76/R72).

**Decisions made this session (continued):**
- **Eight engineer judgment calls recorded** (defensible, non-blocking
  — filed to `ROADMAP.md`'s Shipped entry for the API-guidelines
  trail): (1) codes carried, never re-encoded; (2) recipe C over
  compact `Td`/`T*`; (3) region = the block's own `BT…ET` re-emitted as
  one fresh `BT…ET`, refused if the block shares a text object /
  is non-contiguous / has a show op outside `BT`/`ET` — keeps the
  surgery provably safe and preserves the MCID wrapper by
  construction; (4) axis-aligned scope only — rotated/skewed/
  multi-transform/form-XObject text refused by name, recipe-C rotation
  left as a documented future extension, not a claimed-but-untested
  path; (5) justify requires `Tc = Tw = 0` else refused-and-disclosed;
  (6) **`reflow_block` plans against BASE content and refuses if the
  page was already edited earlier in the same session** (offsets are
  base-relative; a clean named refusal beats a silent mis-splice) —
  recorded explicitly as a **known first-cut limitation** to lift in a
  later Pass, not a permanent constraint; (7) filtered out the 15.0
  preview's carried-through disclosures whose wording asserted
  "nothing written / READ-ONLY" and re-emitted apply-stage-equivalent
  disclosures so nothing contradicts the write that just happened;
  (8) text state (`Tf`/`Tz`/`Tc`/`Tw`) read from the content walk,
  geometry from provenance — kept as two deliberately separate
  sources.
- **Pass 15.2 PROMOTED from Next up to In progress** (`ROADMAP.md`) —
  its reflow UI consumes Pass 15.1's `CommandKind::ReflowBlock` +
  15.0's `ReflowPreview` directly, the same direct-prerequisite
  promotion pattern used for every prior slice in this family.
  `pdfce-ui-specialist` dispatch is the required first step before any
  GUI code, per the standing rule for non-trivial UI changes. Once
  15.2 ships, decision 015 / FF-A is COMPLETE end-to-end and the
  fast-follow ladder (FF-B onward) becomes the next scoping question.

**Findings + decisions:**
- No new generalizable Rust/egui or PDF-domain finding this
  continuation beyond what's already captured — all eight judgment
  calls above are pdfce-internal engineering trade-offs (surgery
  scope, refusal posture, disclosure wording), not ecosystem- or
  domain-generalizable discoveries, so nothing new was filed to
  `D:\dev\rag\rust\`, `D:\dev\rag\egui\`, or `C:\personal_rag\pdf\`
  this continuation.

**Still in flight (continuation 41):**
- **Pass 15.2** — promoted to In progress; not yet started building;
  needs `pdfce-ui-specialist` dispatch first.
- FF-A is now ONE Pass from complete end-to-end (15.0 and 15.1 both
  shipped; only 15.2's UI remains).
- The Beta (measurement/dimensioning) — unchanged, still awaiting
  operator go-ahead/sequencing confirmation relative to Pass 15.x.
- The R-INV-2/3/4 fixture-coverage follow-up (flagged at Pass 14.1) —
  still not yet scheduled as a Pass.
- The list-authoring scope question (filed continuation 36) — still no
  operator answer.
- Item 6 above (already-edited-this-session refusal) is a named,
  disclosed first-cut limitation, not a bug — worth a fixture-coverage
  follow-up whenever the reflow-ladder work resumes past 15.2.
- Everything remains UNCOMMITTED in git — the tree keeps growing (now
  also includes Pass 15.1's `reflow_apply.rs` + CLI/render/GUI changes)
  and is VERY LARGE, compounding the recurring worktree-isolation cost
  on every autonomous-builder dispatch.
- The autonomous `/loop` remains ACTIVE — Pass 15.2 (reflow UI) is
  next.

**Still-open operator items (re-surfaced, ordered oldest-first — one
item's framing escalated again, no items newly added or resolved this
continuation):**
- Encryption-refusal operator sign-off — oldest owed.
- `LEGAL.md` §1 license decision — still undecided.
- `LEGAL.md` §2 Adobe-supplement copyright contradiction — flagged.
- `/R 6` sourcing method — Ken's call (gates Pass 5).
- **Commit authorization — escalating further this continuation.**
  Everything remains UNCOMMITTED; the tree is VERY LARGE and grew again
  this continuation (Pass 15.1's reflow-apply surgery, CLI `reflow`
  subcommand, and render/GUI changes on top of the full decision-014/
  015 text-editing and reflow subsystems, plus the entire shipped-Pass
  history) — the worktree-isolation workaround cost compounds with
  every additional Pass, and Pass 15.2 will add more still. Remains the
  highest-leverage unresolved item alongside the license decision;
  flagging again per the standing instruction to keep escalating the
  framing as the tree keeps growing, not just repeat the same wording.
- W15 — no remote/CI — unchanged.
- Autosave / crash-recovery scratch file + true in-place Save (gated on
  it) — still open, unchanged.
- List-authoring scope question (filed continuation 36) — does the
  operator want bulleted/numbered list authoring as an Acrobat-parity
  target, and if so where in the Pass sequence? Still no operator
  answer.
- Justified-alignment question — remains RESOLVED (decision 015,
  continuation 39); not re-opened, listed here only for continuity of
  the oldest-first trail, no action needed.

**For next session:**
- Build and ship Pass 15.2 (reflow UI: preview overlay + width/
  alignment adjust + accept/reject) — dispatch `pdfce-ui-specialist`
  FIRST; verify the acceptance criteria in decision 015 §6 /
  `ROADMAP.md`'s ★ Pass 15.x entry (ghost preview matches 15.0's
  `ReflowPreview` exactly; width drag re-runs preview live; alignment
  picker round-trips through 15.1's surgery correctly including
  justified; accept commits exactly the one `CommandKind::ReflowBlock`
  15.1 defines with byte-identical undo; reject writes nothing;
  overflow/staleness disclosures visible in the panel not buried in a
  log). Once shipped, decision 015 / FF-A is COMPLETE end-to-end.
- Re-surface commit authorization and the license decision — both
  remain the two oldest-standing, highest-leverage unresolved operator
  items, now more pointed than last continuation given the tree's
  continued growth.
- Get an operator answer on the list-authoring scope question.
- Confirm the beta (measurement/dimensioning) sequencing relative to
  Pass 15.x (now that FF-A is nearly complete, this becomes more
  pressing).

**Same-day continuation 42 — Pass 15.2 SHIPPED (on-canvas within-block
reflow UI — the FINAL FF-A slice); DECISION 015 / FF-A COMPLETE
end-to-end (15.0 + 15.1 + 15.2); GUI relaunched and running live
against the reflow fixture; autonomous loop remains ACTIVE:**

**Shipped:**
- **Pass 15.2 — On-canvas within-block reflow UI (decision 015, FF-A
  slice 3 of 3, FINAL SLICE), SHIPPED and independently re-verified
  green in the main tree**: 60 core `text_edit` tests; CLI reflow
  intact after the P0 dedup (5 `reflow` + 11
  `inspect_reflow_preview`); 53 GUI tests; release GUI builds +
  launches; the GUI is confirmed running with the reflow fixture, the
  reflow sub-mode of the Edit Text tool live. Full record now in
  `ROADMAP.md` Shipped (above); summary here for the session trail:
  - **P0 consolidation paid down before the UI landed:**
    `reflow_recognition_options()` (the relaxed block-recognition that
    keeps ragged-left justified/right/center paragraphs whole — R77)
    collapsed to ONE `pub fn` in `pdfce-core::text_edit::reflow`
    (re-exported at `pdfce_core::text_edit::reflow_recognition_options`).
    The `pub(crate)` duplicate in `reflow_apply.rs` and the private
    duplicate in `pdfce-cli/src/main.rs` are DELETED; every consumer
    (CLI inspect, apply/session path, engine tests, GUI ×3) now calls
    the one source. CLI reflow tests stayed green across the dedup.
  - **NEW `EditableTextModel::block_at(pos) -> Option<usize>`**
    (`#[must_use]`) — sugar over `line_at` + `Line::block`, no GUI type
    in the signature.
  - **GUI (`crates/pdfce-gui/src/main.rs` + `ui_text.rs`): reflow is a
    SUB-MODE of `CanvasTool::TextEdit`, NOT a new tool variant** (R60).
    `TextEditState.reflow: Option<ReflowState>` is mutually exclusive
    with `pending` (14.3's in-place-edit state). "Reflow paragraph…"
    targets the caret's block via the relaxed recognition; the
    property bar offers width (`DragValue` + a canvas drag-handle),
    alignment (pre-filled DETECTED, switchable L/C/R/Justify), and
    leading. Ghost preview + solid targeted-block highlight reuse
    14.3's preview/mask rendering language. Accept commits exactly one
    undo-able `EditSession::reflow_block` (`CommandKind::ReflowBlock`,
    15.1's command); Reject discards, nothing written. Overflow (R76),
    the tagged/trust-level disclosures (R72/R73), and the
    already-edited-this-session refusal (15.1 judgment call 6) surface
    VERBATIM via 14.3's disclosure rendering. Two-stage Esc rejects,
    matching 14.3's convention.
  - Pure helpers `reflow_button_enabled`/`reflow_alignment_is_override`/
    `reflow_refusal_hint` are headless-tested; the egui wiring itself is
    compile-and-launch-verified, consistent with this project's
    established GUI-testing posture.
  - **Public API surface added to `pdfce-core`** (rule-10
    API-guidelines trail): `pub fn reflow_recognition_options() ->
    BlockRecognitionOptions` (`#[must_use]`, WHY/trade-off doc +
    R77/§0.3 cites); `pub fn EditableTextModel::block_at(&self, pos:
    TextPosition) -> Option<usize>` (`#[must_use]`).
  - **Gates (re-verified in the main tree):** `cargo fmt --all --check`
    clean; `clippy --workspace --all-targets --all-features -D
    warnings` clean; `cargo tree` core/render GUI-dep-free (no
    egui/eframe/winit/wgpu/glow/accesskit); `cargo test --workspace` —
    1198 passed, 0 failed; release GUI build succeeds and launches
    without panic against the reflow fixture; **ZERO new dependency**;
    Pass 14.x/15.0/15.1/`vartext` tests all unchanged.

**Decisions made this session (continued):**
- **Seven engineer judgment calls recorded** (defensible, non-blocking
  — filed to `ROADMAP.md`'s Shipped entry for the API-guidelines
  trail): (1) wired §7/§8 disclosure surfacing to 15.1's REAL shipped
  types (`ReflowApplyReport`/`ReflowApplyError`, `report.disclosures`),
  not the original decision-015 spec's hypothesized
  `ReflowReport`/`ReflowSessionError` — the spec predates 15.1's actual
  implementation; (2) **found and fixed a bug in the spec's own §4.3
  override-detection snippet** while implementing it — its `else if
  val == detected` pattern reset the override flag every frame;
  override is instead decided on the CLICK of the clicked alignment
  value; (3) Accept/Reject are buttons + Esc rejects, matching the
  ALREADY-SHIPPED 14.3 `PendingEdit` button-only-Accept convention
  rather than inventing a new keybinding; (4) `ReflowApplyError::
  Unsupported` covers BOTH the already-edited-this-session refusal
  (15.1 call 6) AND rotated/shared/non-contiguous blocks (15.1 call
  3) — one variant, two triggers; core's `Display` names which
  specific condition fired, so the GUI shows one hint without
  duplicating the classification logic; (5) live preview is
  recomputed every frame (pure/cheap at this block-level scale) rather
  than cached-and-invalidated; (6) added `block_at` (a 15.0/15.1-named
  P1 nice-to-have) as a clean, testable, `#[must_use]` accessor; (7)
  the width drag-handle is painted with a faint fill purely for
  discoverability — an invisible drag target is a known egui usability
  trap.
- **`ROADMAP.md`'s ★ Pass 15.x entry (Next up) closed out** with a
  final amendment recording all three slices (15.0/15.1/15.2) shipped
  and decision 015 / FF-A COMPLETE end-to-end; the entry is now
  historical record, the same treatment the ★ Pass 14.x entry got once
  decision 014 completed. The "In progress" section's Pass-15.2 heading
  was removed (moved to Shipped) and replaced with a MILESTONE note
  naming what remains open in the text-parity space now that FF-A is
  done: FF-B (cross-block/cross-page reflow — the genuine
  exceed-Acrobat headline), the named Pass-14.3 GUI refinements
  (selection-replace-on-type, triple-click/drag-select, arrow/Home/End
  caret nav), FF-D (add new text), FF-H (spacing/synthetic styles), and
  the still-open list-authoring scope question. None of these are
  scheduled to a Pass yet.

**Findings + decisions:**
- No new generalizable Rust/egui or PDF-domain finding this
  continuation beyond what's already captured — the seven judgment
  calls above (including the spec-snippet bug fix in item 2) are
  pdfce-internal engineering trade-offs specific to this reflow-UI
  build, not ecosystem- or domain-generalizable discoveries, so nothing
  new was filed to `D:\dev\rag\rust\`, `D:\dev\rag\egui\`, or
  `C:\personal_rag\pdf\` this continuation.

**MILESTONE — FF-A (within-block offline reflow) is COMPLETE
end-to-end (15.0 engine + 15.1 surgery + 15.2 UI, all shipped
2026-08-01).** pdfce now does reviewable, undo-able within-block
reflow: greedy re-wrap, alignment auto-detect/preserve across all four
modes (left/center/right/justified), and working justified alignment
(`TJ` slack distribution) — entirely offline. This reaches, and on
justify-reliability/alignment-detection/overflow-honesty exceeds,
Acrobat's own offline reflow (decision 015 §9's exceed-Acrobat list is
now fully delivered, not just claimed). Combined with the already-
shipped Pass 14.x in-place-editing family, pdfce's Acrobat
text-handling parity is now broad and deep at the P0 level. What
remains in the text-parity space: **FF-B** (cross-block/cross-page
reflow — the exceed-Acrobat headline, since Acrobat's own cross-block
reflow is cloud-gated + English-only), the named Pass-14.3 GUI
refinements (selection-replace-on-type, triple-click/drag-select,
arrow/Home/End caret nav), **FF-D** (add new text), **FF-H**
(spacing/synthetic styles), and the open list-authoring scope question.
None of FF-B/FF-D/FF-H are yet scoped to a Pass — that scoping is the
natural next step, same protocol as FF-A (parity-reference sourcing →
KenAgent decision → build).

**Still in flight (continuation 42):**
- FF-A is fully closed. FF-B onward is the next scoping question for
  the text-editing/reflow line of work; no decision has been opened for
  it yet.
- The Beta (measurement/dimensioning) — unchanged, still awaiting
  operator go-ahead/sequencing confirmation, now more pressing given
  FF-A's completion frees the engineer's next-focus slot.
- The R-INV-2/3/4 fixture-coverage follow-up (flagged at Pass 14.1) —
  still not yet scheduled as a Pass.
- The list-authoring scope question (filed continuation 36) — still no
  operator answer.
- The already-edited-this-session refusal (Pass 15.1 judgment call 6)
  remains a named, disclosed first-cut limitation, not a bug — still
  worth a fixture-coverage follow-up in a later Pass.
- Everything remains UNCOMMITTED in git — the tree keeps growing (now
  also includes Pass 15.2's GUI reflow sub-mode, the `block_at`
  accessor, and the recognition-options consolidation) and is VERY
  LARGE, compounding the recurring worktree-isolation cost on every
  autonomous-builder dispatch.
- The autonomous `/loop` remains ACTIVE. With FF-A complete, the next
  focus is either the Beta (measurement/dimensioning, awaiting operator
  go-ahead) or a new FF-B scoping pass — an operator sequencing call.

**Still-open operator items (re-surfaced, ordered oldest-first —
commit authorization escalated further given the tree's continued
growth through Pass 15.2; no items newly added or resolved this
continuation):**
- Encryption-refusal operator sign-off — oldest owed.
- `LEGAL.md` §1 license decision — still undecided.
- `LEGAL.md` §2 Adobe-supplement copyright contradiction — flagged.
- `/R 6` sourcing method — Ken's call (gates Pass 5).
- **Commit authorization — escalating further this continuation.**
  Everything remains UNCOMMITTED; the tree is VERY LARGE and grew again
  this continuation (Pass 15.2's on-canvas reflow UI, the
  `block_at` accessor, and the `reflow_recognition_options`
  consolidation, on top of the now-COMPLETE decision-014 in-place-
  editing family AND the now-COMPLETE decision-015 reflow family — two
  full multi-Pass subsystems sitting uncommitted end-to-end). The
  worktree-isolation workaround cost compounds with every additional
  Pass; with FF-A now fully shipped, this is the largest uncommitted
  span in the project's history to date. Remains the highest-leverage
  unresolved item alongside the license decision.
- W15 — no remote/CI — unchanged.
- Autosave / crash-recovery scratch file + true in-place Save (gated on
  it) — still open, unchanged.
- List-authoring scope question (filed continuation 36) — does the
  operator want bulleted/numbered list authoring as an Acrobat-parity
  target, and if so where in the Pass sequence? Still no operator
  answer.
- Justified-alignment question — remains RESOLVED (decision 015,
  continuation 39); not re-opened, listed here only for continuity of
  the oldest-first trail, no action needed.

**For next session:**
- Get an operator decision on what comes next now that FF-A is
  complete: the Beta (measurement/dimensioning, awaiting go-ahead) vs. a
  new FF-B (cross-block/cross-page reflow) scoping pass vs. one of the
  smaller named items (Pass-14.3 GUI refinements, FF-D, FF-H).
- Re-surface commit authorization and the license decision — both
  remain the two oldest-standing, highest-leverage unresolved operator
  items, now more pointed than ever given the tree now holds two
  COMPLETE multi-Pass subsystems (decision 014 + decision 015)
  entirely uncommitted.
- Get an operator answer on the list-authoring scope question.
- Confirm the beta (measurement/dimensioning) sequencing now that FF-A
  is complete and the engineer's next-focus slot is open.

**Same-day continuation 43 — Pass 14.4 SHIPPED (the four Pass-14.3
deferred GUI interactions — selection-replace-on-type, triple-click
line-select, drag-select, full arrow/Home/End caret navigation — all
land, plus a latent Backspace-swallow bug fixed as a side effect); the
text-editing beta's interaction set is now COMPLETE; GUI relaunched for
the operator; autonomous loop remains ACTIVE:**

**Shipped:**
- **Pass 14.4 — Text-edit GUI refinements (completing the four
  Pass-14.3 deferred interactions), SHIPPED and independently
  re-verified green in the main tree**: 4 new core caret-navigation
  tests + 56 GUI tests pass; release GUI build launches; relaunched
  live for the operator (pid 40764). Full record now in `ROADMAP.md`
  Shipped (above); summary here for the session trail:
  - **Selection-replace-on-type:** typing over a single-run selection
    now replaces it in one step (previously insert-then-backspace);
    Backspace/Delete delete the selection outright; stays a reviewable
    `PendingEdit`; the font-on-edit refusal-and-disclosure gate at
    Accept is untouched.
  - **Triple-click → line select**, inlined over the already-shipped
    `line_range_at` (Pass 14.3 §0.2 accessor).
  - **Drag-select:** press sets anchor + caret; each dragged frame
    moves the focus caret via a per-move hit-test; selection resolves
    anchor..focus through `resolve_range`.
  - **Arrow / Home / End caret navigation:** Left/Right cross run/line
    boundaries; Up/Down land at the nearest x-position on the adjacent
    line; Home/End via `line_range_at`; Shift extends the active
    selection.
  - All four ride the existing `CanvasTool::TextEdit` — no new tool
    variant, no new dependency, everything a reviewable `PendingEdit`.
  - **Modules changed:** `crates/pdfce-core/src/text_edit/model.rs`
    (new caret-nav accessors + tests); `crates/pdfce-gui/src/canvas.rs`
    (selection-replace pure helpers + tests);
    `crates/pdfce-gui/src/main.rs` (gesture/nav/Delete-key/
    keyboard-gating/selection-replace wiring).
  - **Public API surface added to `pdfce-core`** (rule-10
    API-guidelines trail): `EditableTextModel::{caret_x(pos) ->
    Option<f32>, caret_on_line_nearest_x(line_index, x) ->
    Option<TextPosition>, caret_left(pos), caret_right(pos),
    caret_up(pos, desired_x), caret_down(pos, desired_x)}`.
    (`pdfce-gui`'s `canvas.rs` also gained `single_run_selection_range`
    + `selection_after_type` — GUI-crate helpers, not core API
    surface.)
  - **Notable fix (side effect, not a separately-scoped bug hunt):**
    `collect_keyboard_actions` now yields Home/End/Delete/Backspace to
    the canvas whenever a tool is active — this un-swallowed the
    text-edit Backspace key that the global `DeleteSelection`
    keybinding was silently eating in shipped Pass 14.3. A latent bug,
    caught and fixed as a side effect of this Pass's keyboard-gating
    reconciliation.
  - **Gates (re-verified in the main tree):** `cargo fmt --all --check`
    clean; `clippy --workspace --all-targets --all-features -D
    warnings` clean; `cargo test --workspace` all green (708 core lib
    tests + 56 GUI + integration suites; the 1 ignored test is
    pre-existing, unrelated); `cargo tree` core/render GUI-dep-free
    (GUI-core separation intact); release GUI build succeeds and
    launches without panic, relaunched live for the operator (pid
    40764); **ZERO new dependency**; Pass 14.x/15.x/`vartext` tests all
    unchanged.

**Decisions made this session (continued):**
- **Five engineer judgment calls recorded** (defensible, non-blocking —
  filed to `ROADMAP.md`'s Shipped entry for the API-guidelines trail):
  (1) model-dependent caret navigation lives in `pdfce-core`, not
  `pdfce-gui`, because `PageText`/`TextRun`/`ExtractedGlyph` are
  `#[non_exhaustive]` and only constructible/headless-testable from
  inside core — matches the Pass 14.3 §4.3 "core owns the derived
  structure" precedent; only model-FREE string helpers stayed in
  `canvas.rs`; (2) keyboard-gating reconciled per the Pass 14.3 §4.5
  spec; (3) Up/Down use a per-press `desired_x` from `caret_x` — no
  sticky goal-column across repeated presses, named as a deferred
  nicety, not a defect; (4) arrow-nav is gated to `pending.is_none()`
  — inside an open `PendingEdit` the caret is the draft's own cursor,
  so model-space arrow-nav while composing is a named first-cut line
  (typing/Backspace/Delete still work there); (5) multi-run selection
  refusal is NOT regressed — `cross_run` still suppresses typing and
  disables Accept, and `single_run_selection_range` returns `None` for
  a cross-run span.
- **`ROADMAP.md`'s "In progress" section updated**: the Pass-14.3
  GUI-refinements deferral note is removed from "what remains open in
  the text-parity space" (it's now discharged) and replaced with a
  MILESTONE noting the text-editing beta's interaction set is COMPLETE.
  What remains open, unchanged in substance: FF-B (cross-block/
  cross-page reflow), FF-D (add new page text), FF-H (spacing/
  synthetic styles), and the list-authoring scope question.

**Findings + decisions:**
- No new generalizable Rust/egui or PDF-domain finding this
  continuation — the five judgment calls above are pdfce-internal
  engineering trade-offs specific to this GUI-refinement build, not
  ecosystem- or domain-generalizable discoveries, so nothing new was
  filed to `D:\dev\rag\rust\`, `D:\dev\rag\egui\`, or
  `C:\personal_rag\pdf\` this continuation.

**MILESTONE — the text-editing beta's interaction set (Pass 14.0–14.4)
is now COMPLETE.** click-to-caret, Shift-click/drag-select/triple-click
selection, selection-replace-on-type, and full arrow/Home/End caret
navigation are all shipped and headless-tested where the underlying
model logic lives in core. Combined with FF-A's completion
(continuation 42), pdfce's Acrobat text-handling parity is broad and
deep at the P0 level across both editing AND reflow. Candidate next
steps in the text-parity space, none yet scoped to a Pass: **FF-B**
(cross-block/cross-page reflow — the genuine exceed-Acrobat headline,
Acrobat's own cross-block reflow is cloud-gated + English-only),
**FF-D** (add new page text), **FF-H** (`Tc`/`Tw`/`Tz`/`Ts` spacing +
synthetic styles), and the still-open list-authoring scope question.

**Still in flight (continuation 43):**
- The text-editing beta's interaction set is fully closed (Pass 14.4).
  FF-B onward is the next scoping question for the text-editing/reflow
  line of work; no decision has been opened for it yet.
- The Beta (measurement/dimensioning) — unchanged, still awaiting
  operator go-ahead/sequencing confirmation.
- The R-INV-2/3/4 fixture-coverage follow-up (flagged at Pass 14.1) —
  still not yet scheduled as a Pass.
- The list-authoring scope question (filed continuation 36) — still no
  operator answer.
- The already-edited-this-session refusal (Pass 15.1 judgment call 6)
  remains a named, disclosed first-cut limitation — still worth a
  fixture-coverage follow-up in a later Pass.
- Everything remains UNCOMMITTED in git — the tree keeps growing (now
  also includes Pass 14.4's caret-nav/selection-replace GUI work) and
  is VERY LARGE, compounding the recurring worktree-isolation cost on
  every autonomous-builder dispatch.
- The autonomous `/loop` remains ACTIVE. With both FF-A and the
  Pass-14.3 GUI deferrals now closed, the next focus is either the Beta
  (measurement/dimensioning, awaiting operator go-ahead), a new FF-B
  scoping pass, or one of the smaller named items (FF-D, FF-H,
  list-authoring) — an operator sequencing call.

**Still-open operator items (re-surfaced, ordered oldest-first —
commit authorization escalated further given the tree's continued
growth through Pass 14.4; no items newly added or resolved this
continuation):**
- Encryption-refusal operator sign-off — oldest owed.
- `LEGAL.md` §1 license decision — still undecided.
- `LEGAL.md` §2 Adobe-supplement copyright contradiction — flagged.
- `/R 6` sourcing method — Ken's call (gates Pass 5).
- **Commit authorization — escalating further this continuation.**
  Everything remains UNCOMMITTED; the tree is VERY LARGE and grew again
  this continuation (Pass 14.4's selection-replace/triple-click/
  drag-select/caret-nav GUI work, plus the Backspace-swallow fix), on
  top of the already-COMPLETE decision-014 in-place-editing family AND
  the already-COMPLETE decision-015 reflow family — the tree now holds
  TWO full multi-Pass subsystems PLUS this GUI-polish Pass, all sitting
  uncommitted end-to-end. This is now the largest uncommitted span in
  the project's history to date, larger again than at continuation 42.
  Remains the highest-leverage unresolved item alongside the license
  decision.
- W15 — no remote/CI — unchanged.
- Autosave / crash-recovery scratch file + true in-place Save (gated on
  it) — still open, unchanged.
- List-authoring scope question (filed continuation 36) — does the
  operator want bulleted/numbered list authoring as an Acrobat-parity
  target, and if so where in the Pass sequence? Still no operator
  answer.
- Justified-alignment question — remains RESOLVED (decision 015,
  continuation 39); not re-opened, listed here only for continuity of
  the oldest-first trail, no action needed.

**For next session:**
- Get an operator decision on what comes next now that both FF-A and
  the Pass-14.3 GUI deferrals are complete: the Beta (measurement/
  dimensioning, awaiting go-ahead) vs. a new FF-B (cross-block/
  cross-page reflow) scoping pass vs. one of the smaller named items
  (FF-D, FF-H, list-authoring).
- Re-surface commit authorization and the license decision — both
  remain the two oldest-standing, highest-leverage unresolved operator
  items, now more pointed than ever given the tree holds two COMPLETE
  multi-Pass subsystems (decision 014 + decision 015) plus this
  GUI-polish Pass, entirely uncommitted.
- Get an operator answer on the list-authoring scope question.
- Confirm the beta (measurement/dimensioning) sequencing now that both
  FF-A and the Pass-14.3 GUI deferrals are complete and the engineer's
  next-focus slot is open.

**Same-day continuation 44 — KenAgent decision 016 filed (prioritizes
the text-parity fast-follow ladder; scopes FF-D — add NEW page text —
as ★ Pass 16.x; FF-C and list-authoring recorded operator-gated,
unscheduled); R78–R79 filed; decision 014 §5.3 amended; the autonomous
`/loop` remains ACTIVE:**

**Shipped:**
- Nothing shipped this continuation — a decision-filing/roadmap-update
  continuation, not a build. Pass 16.0 (add-new-text engine + point-
  text insert) is now the recommended next build once this continuation's
  filing is done.

**Decisions made this session (continued):**
- **KenAgent decision 016 ACCEPTED** — full record
  `docs/decisions/016-ffd-add-new-page-text.md`. Two jobs: (1)
  prioritizes the remaining decision-014 fast-follow ladder now that
  both decision 014 (Pass 14.x, in-place editing) and decision 015
  (Pass 15.x, FF-A reflow) are COMPLETE end-to-end — FF-D ranked #1
  (solo-startable, maximal leverage of the shipped 14.x/15.x substrate,
  lowest landmine profile); FF-C ranked #2 on value (lifts the
  embedded-subset refusal wall) but **operator-gated** (rule 13
  copyleft + rule 8 license-undecided); FF-B deferred (rarest daily
  action, largest new subsystem); FF-H deferred (partly premature,
  couples to a not-yet-built a11y subsystem); list-authoring
  **operator-gated** (scope call); and (2) scopes FF-D concretely: a
  new `BT…ET` text object appended to the page `/Contents` array
  (§7.7.3.3, original stream byte-identical) — structurally distinct
  from, and NEVER conflated with, the already-shipped Pass-6.2
  FreeText annotation path (a real, sourced Acrobat naming collision
  the parity catalog documents); default font is a bundled Standard-14
  permissive face (§9.6.2.2, no embedding) via decision 012's
  `GlyphSource`, which is precisely why FF-D needs no FF-C to ship;
  routed through the SAME 14.x edit/format + 15.x reflow pipeline as
  any other page text once added; one undo-able
  `CommandKind::AddText`.
- **Pass-number call: fresh ★ Pass 16.x, not 14.5–14.7.** Decision 016
  §6 delegated the renumbering choice to the librarian. Chose a fresh
  **Pass 16.x** ("text authoring") to keep three coherent,
  separately-referenceable families: 14.x = in-place editing, 15.x =
  reflow, 16.x = authoring NEW text — the same precedent set when 15.x
  itself was assigned fresh rather than folded into 14.4–14.6
  (continuation 39). Sliced as **16.0** (add-new-text engine +
  point-text insert, core + CLI `text add --at`) — RECOMMENDED NEXT
  BUILD, spec grounding for §7.7.3.3/§7.8.3/§9.4/§9.4.2/§9.6.2.2 being
  sourced by `pdfce-spec-librarian` in parallel now — **16.1** (boxed
  add + wrap via the already-shipped 15.x reflow engine, CLI `text add
  --box`), and **16.2** (add-text canvas UI, DISPATCH
  `pdfce-ui-specialist` first).
- **Standing rules R78–R79 filed** (current ceiling was R77): **R78**
  add-new-text-is-page-content-surgery-never-freetext (sibling of R69
  for the add-new-content case); **R79**
  new-text-uses-bundled-supplied-face-no-embedding-disclosed-provenance
  (why FF-D needs no FF-C). Both added to `ROADMAP.md` Standing rules
  in order, and referenced from the new ★ Pass 16.x Next-up entry.
- **`docs/decisions/014-acrobat-text-editing.md` §5.3 amended** — a
  dated forward-pointer footnote added (matching the existing 015
  amendment's footnote style, history intact, nothing rewritten):
  "FF-D scheduled 2026-08-01 by decision 016 → Pass 16.x; see 016."
- **`ROADMAP.md` Backlog: FF-C and list-authoring recorded explicitly
  operator-gated, NOT scheduled.** List-authoring's existing Backlog
  entry (filed continuation 36) got a dated amendment footer
  re-confirming it is still awaiting an operator "do we even want
  this?" call (decision 016 ranked it #5, sequences after FF-D
  regardless — no new information, just re-confirmed). A **new** FF-C
  Backlog bullet was filed: font subsetting/glyph embedding is ranked
  #2 by value but cannot start solo — adding a subsetter is a Cargo
  dependency, triggering rule 13 (copyleft classification, operator
  approval never solo) and gated by rule 8 (license undecided,
  `LEGAL.md` §1). Both bullets are marked "AWAITING OPERATOR DECISION
  — DO NOT SCHEDULE TO A PASS." Recommendation surfaced (decision
  016's own): unblock FF-C in parallel with the Pass 16.x build —
  approve a permissive-only subsetter path and, ideally, settle the
  license — so FF-C can follow FF-D directly; the font-subsetting spec
  dispatch already named at decision 014 stays queued meanwhile.
- **`ROADMAP.md` "In progress" status paragraph updated** to reflect
  FF-D's new scoped status (no longer "none of these scheduled to a
  Pass yet") and to point at the new ★ Pass 16.x entry.

**Findings + decisions:**
- No new generalizable Rust/egui or PDF-domain finding this
  continuation — a pure decision-filing/roadmap-bookkeeping pass, no
  code touched, nothing ecosystem- or domain-generalizable surfaced.

**Still in flight (continuation 44):**
- Pass 16.0 (add-new-text engine + point-text insert) is the
  recommended next build; its spec grounding (§7.7.3.3, §7.8.3, §9.4/
  §9.4.2, §9.6.2.2) is being sourced by `pdfce-spec-librarian` in
  parallel now, per decision 016's own dispatch instruction.
- FF-B (cross-block/cross-page reflow) and FF-H (spacing + synthetic
  styles + StructTree) remain named, unscoped fast-follows — no
  decision opened for either yet.
- FF-C and list-authoring are now explicitly recorded operator-gated
  in `ROADMAP.md` Backlog (see above) — neither may enter a Pass
  without an explicit operator call.
- The Beta (measurement/dimensioning) — unchanged, still awaiting
  operator go-ahead/sequencing confirmation.
- Everything remains UNCOMMITTED in git — no code changed this
  continuation (decision-filing only), so the uncommitted span is
  unchanged in size from continuation 43, still the largest in the
  project's history to date.
- The autonomous `/loop` remains ACTIVE.

**Still-open operator items (re-surfaced, ordered oldest-first — two
NEW items added this continuation, both from decision 016 §10; nothing
resolved):**
- Encryption-refusal operator sign-off — oldest owed.
- `LEGAL.md` §1 license decision — still undecided. **Now doubly
  pointed:** decision 016 flags this as one of two things (alongside
  approving a permissive-only subsetter path) that would unblock FF-C
  to follow directly behind the now-scoped Pass 16.x (FF-D) build.
- `LEGAL.md` §2 Adobe-supplement copyright contradiction — flagged.
- `/R 6` sourcing method — Ken's call (gates Pass 5).
- **Commit authorization** — unchanged this continuation (no code
  touched); still the largest uncommitted span in the project's
  history (two COMPLETE multi-Pass subsystems — decision 014 + decision
  015 — plus the Pass 14.4 GUI-polish work, all uncommitted). Remains
  the highest-leverage unresolved item alongside the license decision.
- W15 — no remote/CI — unchanged.
- Autosave / crash-recovery scratch file + true in-place Save (gated on
  it) — still open, unchanged.
- List-authoring scope question (filed continuation 36; re-confirmed
  operator-gated by decision 016 §10 this continuation) — does the
  operator want bulleted/numbered list authoring as an Acrobat-parity
  target at all, and if so where in the Pass sequence? Still no
  operator answer. Sequences after Pass 16.x (FF-D) regardless.
- **NEW — FF-C (font subsetting/glyph embedding) license/dependency
  gate** (filed this continuation, decision 016 §10). Ranked #2 by
  value in the text-parity fast-follow ladder — lifts the
  embedded-subset edit-refusal wall — but blocked on an explicit
  operator call: approve a permissive-only subsetter dependency (rule
  13) and, ideally, settle `LEGAL.md` §1 first. Recommendation: unblock
  in parallel with the Pass 16.0 build so FF-C can follow FF-D
  directly.
- Justified-alignment question — remains RESOLVED (decision 015,
  continuation 39); not re-opened, listed here only for continuity of
  the oldest-first trail, no action needed.

**For next session:**
- Build **Pass 16.0** (add-new-text engine + point-text insert, core +
  CLI `text add --at`) — the recommended next build; confirm
  `pdfce-spec-librarian`'s §7.7.3.3/§7.8.3/§9.4/§9.4.2/§9.6.2.2 grounding
  has landed before or during the build.
- Re-surface commit authorization and the license decision — both
  remain the two oldest-standing, highest-leverage unresolved operator
  items; the license decision is now doubly pointed since it also
  gates unblocking FF-C.
- Get the operator's FF-C unblock call (permissive-only subsetter
  approval + ideally the license decision) so FF-C can be scoped to
  follow Pass 16.x directly.
- Get an operator answer on the list-authoring scope question.
- Confirm the beta (measurement/dimensioning) sequencing whenever the
  operator wants to revisit it — unchanged, still open.

**Same-day continuation 45 — Pass 16.0 SHIPPED (add-new-text engine +
point-text insert, core + CLI `add-text`; decision 016, FF-D slice 1 of
3); a certification-signature-guard gap flagged and filed to Backlog,
not actioned; 16.1 (boxed add) and 16.2 (add-text canvas UI) promoted
to In progress in parallel; the autonomous `/loop` remains ACTIVE:**

**Shipped:**
- **Pass 16.0 — Add-new-text engine + point-text insert (core + CLI),
  SHIPPED and independently re-verified green in the main tree**: 9
  core `add_text` tests + 8 CLI `add_text` tests pass; a live add
  ("Added by pdfce" at 100,700) produced exactly two new objects
  (content_object=6, font_object=7), bundled Helvetica disclosed, and
  round-trip reports `identical=1, raster_identical=1, reloaded=1`
  (original page byte-untouched). Full record now in `ROADMAP.md`
  Shipped (above); summary here for the session trail:
  - **New engine module:** `crates/pdfce-core/src/text_edit/addtext.rs`
    (NEW); `mod.rs` (re-exports); `fontdata/mod.rs` (added
    `std14_base_font_name(Std14) -> &'static str`, the inverse of
    `std14_by_base_font`, to write `/BaseFont`); `edit.rs`
    (`CommandKind::AddText` + `EditSession::add_text`);
    `crates/pdfce-cli/src/main.rs` (`add-text` subcommand +
    `cmd_add_text` + `parse_at_pair`/`parse_rgb_triple`). NEW tests
    `crates/pdfce-core/tests/add_text.rs` (9),
    `crates/pdfce-cli/tests/add_text.rs` (8). NEW fixtures
    `fixtures/synthetic/addtext/{plain,inherited-resources,tagged}.pdf`
    + generator + `PROVENANCE.md`.
  - **Three load-bearing recipes:** (1) `/Contents` single→array append
    — the incremental update re-emits ONLY the page dict + the 2 new
    objects, the original content stream NEVER re-emitted
    (byte-identical, R32/R46). (2) Standard-14 Type1 font dict, NO
    `/FontFile` (R79), `/FontDescriptor` deliberately omitted to keep
    exactly 2 new objects. (3) Inheritance-safe `/Font` add — rebuilds
    the page's `/Resources` INLINE from the effective (own-or-inherited)
    resources with a merged `/Font` subdict + a collision-free
    `/pdfceF{n}` name, never mutating the shared ancestor `/Pages`
    dict (verified on the inherited-resources fixture).
  - **Public API surface added to `pdfce-core`** (rule-10
    API-guidelines trail): `text_edit::add_text`, `AddTextRequest`,
    `AddTextReport`, `AddTextOutcome`, `AddTextError` (`thiserror`,
    `#[non_exhaustive]`), `FontProvenance { Bundled, Supplied }`,
    `NewTextColor { Black, Rgb }`; `edit::CommandKind::AddText`,
    `EditSession::add_text`; `fontdata::std14_base_font_name`. All
    doc-commented with runnable examples + ISO cites (§7.7.3.3/§8.4.2/
    §7.8.3/§9.6.2.2/§9.4.2/§9.4.3 + R78/R79/R71/R73).
  - **Acceptance, all tested:** original byte-identical; re-extracts as
    an editable (14.1) and formattable (14.2) block; inheritance-safe;
    tagged-page R73 disclosure; R71 missing-glyph refusal (core no
    output + CLI exit 9); undo restores byte-identical; both
    `Bundled`/`Supplied` font provenance disclosed; R59 render check
    (notdef=0, both runs rasterize).
  - **Gates (re-verified in the main tree):** `cargo fmt --all --check`
    clean; `clippy --workspace --all-targets --all-features -D
    warnings` clean (panic-free); `cargo tree -p pdfce-core`/
    `-p pdfce-render` zero GUI deps; full workspace `cargo test` 0
    failed (708 core unit + integration incl. the 9+8 new, plus 47
    doctests); Pass 14.x/15.x/`vartext` tests all unchanged; **ZERO new
    dependency** (no `Cargo.toml` touched).

**Decisions made this session (continued):**
- **Four engineer judgment calls recorded** (defensible, non-blocking —
  filed to `ROADMAP.md`'s Shipped entry for the API-guidelines trail):
  (1) CLI subcommand named `add-text` (flat kebab, matching shipped
  `edit-text`/`format-text`/`reflow`), NOT decision 016's literal "text
  add" group name — internal CLI-surface consistency won; a future
  migration to a `text` subcommand group is a separate cosmetic pass;
  (2) `/FontDescriptor` omitted (full `/Widths` form kept) — §9.6.2.1
  would force it indirect for zero metric benefit, keeps the add at
  exactly 2 new objects; (3) `/Resources` always rebuilt inline,
  uniform across own/indirect/inherited resource dicts, referencing
  rather than mutating/duplicating shared sub-dicts; (4) a space
  character emits an R-INV-5 "ambiguous" disclosure (WinAnsi maps space
  at both code 32 and code 160) — the shared 14.1 gate behaving as
  designed, left as-is for cross-Pass consistency but flagged as mildly
  noisy (a multi-word add can emit two near-identical space
  disclosures) — a candidate future polish to de-dup R-INV-5 space
  disclosures within one add.
- **`ROADMAP.md` Backlog: NEW follow-up filed, NOT actioned —
  certification-signature guard gap on `add_text`/
  `EditSession::add_text`.** Unlike `add_markup` (Pass 6.x), the
  `add_text` free function and `EditSession::add_text` both check
  encryption and suppressed-objects guards, but neither reaches
  `check_certification` (a private `EditSession` method the
  free-function engine has no access to). Recorded as a gap, scope
  named for whenever it's actioned: (1) add the guard to the
  `add_text`/`EditSession::add_text` path mirroring `add_markup`'s
  existing check; (2) consider exposing `check_certification` (or an
  equivalent hook) so other free-function engines (e.g. the 15.x reflow
  engine, if it has the same gap) can reach it. No Pass number
  invented; not dispatched this continuation.
- **`ROADMAP.md`'s "In progress" and ★ Pass 16.x sections updated**:
  Pass 16.0 moved out of "recommended next build" into Shipped; **16.1
  (boxed add + wrap via the 15.x reflow engine) promoted to In
  progress**; **16.2 (add-text canvas UI) also now in flight** in
  parallel, `pdfce-ui-specialist` dispatched first per the standing
  rule for non-trivial UI changes.

**Findings + decisions:**
- No new generalizable Rust/egui or PDF-domain finding this
  continuation — the four judgment calls above and the
  certification-guard gap are pdfce-internal engineering findings
  specific to this build's surgery/session-command design, not
  ecosystem- or domain-generalizable discoveries, so nothing new was
  filed to `D:\dev\rag\rust\`, `D:\dev\rag\egui\`, or
  `C:\personal_rag\pdf\` this continuation.

**Still in flight (continuation 45):**
- **16.1 (boxed add + wrap via the 15.x reflow engine)** is now In
  progress — the next slice of decision 016/FF-D.
- **16.2 (add-text canvas UI)** is in flight in parallel;
  `pdfce-ui-specialist` dispatch is the required first step per
  standing rule.
- The certification-signature-guard gap on `add_text`/
  `EditSession::add_text` is filed to Backlog, flagged, NOT actioned —
  awaits an explicit dispatch, no operator decision required to action
  it (it's an engineering completeness gap, not a scope call), but not
  yet scheduled to a Pass.
- FF-B (cross-block/cross-page reflow) and FF-H (spacing + synthetic
  styles + StructTree) remain named, unscoped fast-follows — unchanged.
- FF-C and list-authoring remain explicitly operator-gated in
  `ROADMAP.md` Backlog — unchanged, no operator answer yet.
- The Beta (measurement/dimensioning) — unchanged, still awaiting
  operator go-ahead/sequencing confirmation.
- Everything remains UNCOMMITTED in git — the tree grew again this
  continuation (Pass 16.0's new `addtext.rs` engine module, CLI
  subcommand, and three new fixtures), on top of the already-COMPLETE
  decision-014 (in-place editing) and decision-015 (reflow) subsystems
  plus the Pass-14.4 GUI-polish work — now THREE complete/near-complete
  subsystems' worth of code sitting uncommitted end-to-end, larger
  again than at continuation 43.
- The autonomous `/loop` remains ACTIVE — 16.1 (boxed-add build)
  dispatched in parallel with 16.2 (UI design).

**Still-open operator items (re-surfaced, ordered oldest-first — no
items newly added or resolved this continuation; commit authorization
escalated further given the tree's continued growth through Pass
16.0):**
- Encryption-refusal operator sign-off — oldest owed.
- `LEGAL.md` §1 license decision — still undecided. Doubly pointed:
  also gates unblocking FF-C (font subsetting) to follow Pass 16.x.
- `LEGAL.md` §2 Adobe-supplement copyright contradiction — flagged.
- `/R 6` sourcing method — Ken's call (gates Pass 5).
- **Commit authorization — escalating further this continuation.**
  Everything remains UNCOMMITTED; the tree grew again (Pass 16.0's new
  `addtext.rs` engine, `add-text` CLI subcommand, three new fixtures +
  generator), on top of the already-COMPLETE decision-014 in-place-
  editing family, the already-COMPLETE decision-015 reflow family, and
  the Pass-14.4 GUI-polish work — the tree now holds essentially the
  entire text-parity subsystem to date (in-place editing + reflow +
  the first slice of new-text authoring), all sitting uncommitted. This
  is now the largest uncommitted span in the project's history,
  larger again than at continuation 43. Remains the highest-leverage
  unresolved item alongside the license decision.
- W15 — no remote/CI — unchanged.
- Autosave / crash-recovery scratch file + true in-place Save (gated on
  it) — still open, unchanged.
- List-authoring scope question (filed continuation 36; re-confirmed
  operator-gated by decision 016 §10) — does the operator want
  bulleted/numbered list authoring as an Acrobat-parity target at all,
  and if so where in the Pass sequence? Still no operator answer;
  sequences after Pass 16.x (FF-D) regardless.
- **FF-C (font subsetting/glyph embedding) license/dependency gate**
  (filed continuation 44, decision 016 §10) — ranked #2 by value in the
  text-parity fast-follow ladder, but blocked on an explicit operator
  call: approve a permissive-only subsetter dependency (rule 13) and,
  ideally, settle `LEGAL.md` §1 first. Recommendation stands: unblock in
  parallel with the Pass 16.x build so FF-C can follow FF-D directly.
- Justified-alignment question — remains RESOLVED (decision 015,
  continuation 39); not re-opened, listed here only for continuity of
  the oldest-first trail, no action needed.

**For next session:**
- Continue Pass 16.1 (boxed add + wrap via the 15.x reflow engine) to
  completion, then Pass 16.2 (add-text canvas UI, via
  `pdfce-ui-specialist`) — decision 016/FF-D end-to-end.
- Consider dispatching the certification-signature-guard follow-up (add
  the guard to `add_text`/`EditSession::add_text`, mirroring
  `add_markup`) — an engineering-completeness item, not gated on an
  operator scope call, but not yet scheduled.
- Re-surface commit authorization and the license decision — both
  remain the two oldest-standing, highest-leverage unresolved operator
  items, now more pointed than ever given the tree holds essentially
  the entire text-parity subsystem (in-place editing + reflow + the
  first slice of new-text authoring), entirely uncommitted.
- Get the operator's FF-C unblock call and a list-authoring scope
  answer — both still open, unchanged.
- Confirm the beta (measurement/dimensioning) sequencing whenever the
  operator wants to revisit it — unchanged, still open.

**Same-day continuation 46 — Pass 16.1 SHIPPED (boxed add-new-text:
multi-line wrap/justify/overflow via the 15.x reflow engine, core +
CLI `add-text --box`; decision 016, FF-D slice 2 of 3); Pass 16.2
(add-text canvas UI) design SHIPPED and its build now dispatched — the
FINAL slice of decision 016/FF-D; the autonomous `/loop` remains
ACTIVE:**

**Shipped:**
- **Pass 16.1 — Boxed add-new-text + wrap via the 15.x reflow engine
  (core + CLI), SHIPPED and independently re-verified green in the main
  tree**: 16 core `add_text` tests + 13 CLI `add_text` tests pass (up
  from 9+8 at Pass 16.0); a live boxed justified add wrapped to 2 lines
  with the derived-layout disclosure, round-trip `identical=1,
  raster_identical=1` (original page byte-untouched). Full record now
  in `ROADMAP.md` Shipped (above); summary here for the session trail:
  - **Modules changed:** CHANGED `addtext.rs` (boxed branch lives
    entirely inside the SHARED `plan_add_text` planner — 16.0's
    `/Contents` append, inheritance-safe `/Resources`/`/Font` merge,
    Std-14 no-embed dict, F-refuse encode, and both call sites (free
    `add_text` + `EditSession::add_text`) all inherited verbatim;
    boxed session integration was FREE, no `edit.rs` change, same
    `CommandKind::AddText`); CHANGED `reflow.rs` (`align_origin_x` +
    `line_natural_width` hoisted to `pub(crate)` for reuse); CHANGED
    `crates/pdfce-cli/src/main.rs` (`add-text --box/--align/--leading`).
    Tests: +7 core, +5 CLI. Reused `fixtures/synthetic/addtext/plain.pdf`
    (Courier monospace, hand-computable breaks; no new fixture).
  - **Reuse, not duplication:** 15.x's `linebreak::greedy_pack` + the
    two hoisted reflow helpers reused as-is; the only real difference
    from a reflow is the MEASURER (a fresh box has no glyphs, so it
    measures by the chosen face's §9.4.4 AFM `/Widths`, like
    `vartext`, instead of provenance advances). 15.1's negative-`TJ`
    justified-slack emission recipe reused (sign-mirror of
    `reflow_apply::emit_justified_line`).
  - **Public API added to `pdfce-core`** (rule-10 trail, all
    `#[non_exhaustive]`, non-breaking): `AddTextRequest` fields
    `wrap_box: Option<Rect>`, `alignment: BlockAlignment`, `leading:
    Option<f64>` + builders `with_box`/`with_alignment`/`with_leading`;
    `AddTextReport` fields `wrapped_lines`, `box_overflow_lines`,
    `page_overflow_pt`, `alignment`; `AddTextError` variants
    `InvalidBox(f64, f64)`, `NoWordsToWrap`.
  - **Acceptance, all tested:** wrap-matches-hand-computed; L/C/R
    placement correct (origin_x 72/116/160); justified right-flush +
    last-line-unstretched; original-byte-identical; overflow emitted-
    not-clipped per R76; undo restores original; re-recognized by the
    14.0 model; R71 refusal; `InvalidBox`/`NoWordsToWrap` clean
    refusals; CLI `--at`/`--box` mutual exclusion + R59 render clean.
  - **Gates (re-verified in the main tree):** fmt/clippy clean
    (panic-free); `cargo tree` core/render GUI-dep-free; full workspace
    0 failures (16 core + 13 CLI `add_text`; 14.x/15.x/16.0/`vartext`
    unchanged); R59 notdef=0; round-trip byte-verbatim; **ZERO new
    dependency**.
- **Pass 16.2 — Add-text canvas UI: DESIGN shipped, build dispatched.**
  `pdfce-ui-specialist` returned its critique + change list for the
  final decision-016/FF-D slice. Three key calls: (1) a dedicated
  `CanvasTool::AddText` variant — not overloaded onto the existing
  FreeText-annotation tool; (2) a REQUIRED tooltip/label disambiguating
  "add page text" from "add FreeText annotation" at the point of
  interaction (not just in docs — the catalog's real Acrobat
  naming-collision finding, decision 016 §3.1, must not repeat in
  pdfce's own UI); (3) a new pure, read-only wrap-preview accessor so
  box-mode dragging shows live wrap feedback without mutating document
  state ahead of commit. Build now dispatched.

**Decisions made this session (continued):**
- **Nine engineer judgment calls recorded** (defensible, non-blocking
  — filed to `ROADMAP.md`'s Shipped entry for the API-guidelines
  trail): (1) emission recipe C, absolute `Tm` per line, immune to
  relative-`Td` accumulation; (2) `\n` = hard paragraph breaks, each
  wrapped independently with its own un-justified last line, words
  split on ASCII whitespace; (3) wrap width = full box width, left
  origin = box `llx`, NO padding inset (unlike `vartext`'s `TEXT_PAD`);
  (4) first baseline = `box_top − 0.75·size`, descent `0.25·size`,
  matching the 14.0/15.x line-box convention; (5) box = `(x,y,w,h)`
  with `(x,y)` = lower-left (PDF `Rect` convention); (6) alignment is
  an explicit input, default `Left` — deliberately NOT 15.0's
  auto-detect (a fresh box has no glyphs to detect from); (7) added
  `--leading`/`with_leading`, default `1.2·size`, disclosed-derived,
  overridable; (8) `NoWordsToWrap`/`InvalidBox` clean refusals → CLI
  `EDIT_REFUSED`; (9) justified single-word/last/overflowing-word
  lines left un-stretched, space width from the face's space glyph,
  fallback `0.25·size` disclosed.
- **`ROADMAP.md`'s In progress / ★ Pass 16.x sections updated**: Pass
  16.1 moved out of In progress into Shipped; **16.2 (add-text canvas
  UI) is now the SOLE remaining slice of decision 016/FF-D**, its
  design shipped, build in progress. Decision 016/FF-D is now TWO
  THIRDS complete end-to-end (16.0 + 16.1 shipped, 16.2 build in
  flight).

**Findings + decisions:**
- No new generalizable Rust/egui or PDF-domain finding this
  continuation — the nine judgment calls above are pdfce-internal
  engineering findings specific to this build's surgery/session-command
  design, not ecosystem- or domain-generalizable discoveries, so
  nothing new was filed to `D:\dev\rag\rust\`, `D:\dev\rag\egui\`, or
  `C:\personal_rag\pdf\` this continuation.

**Still in flight (continuation 46):**
- **16.2 (add-text canvas UI)** build is in progress — the design is
  settled (dedicated `CanvasTool::AddText`, required disambiguation
  tooltip, read-only wrap-preview accessor); this is the LAST slice of
  decision 016/FF-D.
- The certification-signature-guard gap on `add_text`/
  `EditSession::add_text` remains filed to Backlog, flagged, NOT
  actioned — unchanged from continuation 45.
- FF-B, FF-H remain named, unscoped fast-follows — unchanged.
- FF-C and list-authoring remain explicitly operator-gated in
  `ROADMAP.md` Backlog — unchanged, no operator answer yet.
- The Beta (measurement/dimensioning) — unchanged, still awaiting
  operator go-ahead/sequencing confirmation.
- Everything remains UNCOMMITTED in git — the tree grew again this
  continuation (Pass 16.1's boxed-wrap engine extension, CLI
  `--box/--align/--leading` flags, plus 16.2's in-flight canvas-UI
  build), on top of the already-COMPLETE decision-014 and decision-015
  subsystems, the Pass-14.4 GUI-polish work, and Pass 16.0 — now FOUR
  complete/near-complete subsystems' worth of code sitting uncommitted
  end-to-end, larger again than at continuation 45.
- The autonomous `/loop` remains ACTIVE — 16.2 (canvas-UI build)
  dispatched as the sole remaining decision-016/FF-D slice.

**Still-open operator items (re-surfaced, ordered oldest-first — no
items newly added or resolved this continuation; commit authorization
escalated further given the tree's continued growth through Pass
16.1):**
- Encryption-refusal operator sign-off — oldest owed.
- `LEGAL.md` §1 license decision — still undecided. Doubly pointed:
  also gates unblocking FF-C (font subsetting) to follow Pass 16.x.
- `LEGAL.md` §2 Adobe-supplement copyright contradiction — flagged.
- `/R 6` sourcing method — Ken's call (gates Pass 5).
- **Commit authorization — escalating further this continuation.**
  Everything remains UNCOMMITTED; the tree grew again (Pass 16.1's
  boxed-wrap engine extension + CLI flags, plus 16.2's in-flight
  canvas-UI build), on top of the already-COMPLETE decision-014
  in-place-editing family, the already-COMPLETE decision-015 reflow
  family, the Pass-14.4 GUI-polish work, and Pass 16.0 — the tree now
  holds essentially the ENTIRE text-parity subsystem to date (in-place
  editing + reflow + point-and-boxed new-text authoring), all sitting
  uncommitted, with only the canvas-UI slice left to land. This is now
  the largest uncommitted span in the project's history, larger again
  than at continuation 45. Remains the highest-leverage unresolved item
  alongside the license decision.
- W15 — no remote/CI — unchanged.
- Autosave / crash-recovery scratch file + true in-place Save (gated on
  it) — still open, unchanged.
- List-authoring scope question (filed continuation 36; re-confirmed
  operator-gated by decision 016 §10) — does the operator want
  bulleted/numbered list authoring as an Acrobat-parity target at all,
  and if so where in the Pass sequence? Still no operator answer;
  sequences after Pass 16.x (FF-D) regardless.
- **FF-C (font subsetting/glyph embedding) license/dependency gate**
  (filed continuation 44, decision 016 §10) — ranked #2 by value in the
  text-parity fast-follow ladder, but blocked on an explicit operator
  call: approve a permissive-only subsetter dependency (rule 13) and,
  ideally, settle `LEGAL.md` §1 first. Recommendation stands: unblock in
  parallel with the Pass 16.x build so FF-C can follow FF-D directly.
- Justified-alignment question — remains RESOLVED (decision 015,
  continuation 39); not re-opened, listed here only for continuity of
  the oldest-first trail, no action needed.

**For next session:**
- Complete Pass 16.2 (add-text canvas UI build) — the final slice of
  decision 016/FF-D; once shipped, decision 016 is COMPLETE end-to-end
  (matching decisions 014 and 015's completion pattern).
- Consider dispatching the certification-signature-guard follow-up (add
  the guard to `add_text`/`EditSession::add_text`, mirroring
  `add_markup`) — an engineering-completeness item, not gated on an
  operator scope call, but not yet scheduled.
- Re-surface commit authorization and the license decision — both
  remain the two oldest-standing, highest-leverage unresolved operator
  items, now more pointed than ever given the tree holds essentially
  the entire text-parity subsystem (in-place editing + reflow + both
  point and boxed new-text authoring), entirely uncommitted, with only
  the canvas-UI slice remaining before decision 016 closes out.
- Get the operator's FF-C unblock call and a list-authoring scope
  answer — both still open, unchanged.
- Confirm the beta (measurement/dimensioning) sequencing whenever the
  operator wants to revisit it — unchanged, still open.

**Same-day continuation 47 — Pass 16.2 SHIPPED (on-canvas Add-Text UI;
decision 016, FF-D slice 3 of 3, FINAL SLICE) — DECISION 016 / FF-D
COMPLETE END-TO-END; the broader Acrobat text-handling parity arc
(decisions 014 + 015 + 016) is now COMPLETE at the P0 level; the
autonomous `/loop` remains ACTIVE:**

**Shipped:**
- **Pass 16.2 — On-canvas Add-Text UI (gui), SHIPPED and independently
  re-verified green in the main tree**: 18 core `add_text` tests
  (incl. the `preview_wrap`↔`add_text` parity proof) + 59 GUI tests
  pass; release GUI build succeeds and launches; relaunched live for
  the operator (pid 25280) with the Add-Text tool available. Full
  record now in `ROADMAP.md` Shipped (above); summary here for the
  session trail:
  - **P0 pure wrap-preview (core):** NEW
    `pdfce_core::text_edit::preview_wrap(text, wrap_box, page_crop,
    font: Std14, size, alignment, leading) -> Result<AddTextWrapPreview,
    AddTextError>` — GENUINELY factored out of 16.1's boxed layout
    (`layout_boxed` now takes explicit inputs and carries each line's
    original text alongside its emission codes, so `add_text`'s boxed
    path and `preview_wrap` share ONE `layout_boxed` pass — no
    duplicated wrap/origin/overflow math). Pure/read-only, no
    `&Document`, no mutation, no GUI dependency. CHANGED `addtext.rs` +
    `mod.rs` (re-exports) + `fontdata/mod.rs` (`Std14::ALL`).
  - **GUI:** `CanvasTool::AddText` — a SECOND real tool variant,
    mutually exclusive with `CanvasTool::TextEdit` (the opposite call
    from 15.2's reflow-sub-mode approach) — plus pure placement helpers
    (`resolve_drag_placement`) and `run_add_text_tool` (click→point /
    drag→box rubber-band / typing→live wrap-preview ghost / property
    bar size+colour+font+alignment / Accept→`EditSession::add_text`
    landing one `CommandKind::AddText` / Reject+Esc discards), a
    toolbar button, and a keyboard chord (Ctrl+Shift+E). CHANGED
    `canvas.rs`, `ui_text.rs`, `main.rs`.
  - **Tooltip disambiguation (required companion, R78 bidirectional):**
    `text_menu_tooltip()` now states the FreeText/markup tool is "a
    removable annotation, not page content … use Add Text instead";
    `edit_text_tool_tooltip()` names Add Text; new
    `add_text_tool_tooltip()` is the three-sentence disambiguator —
    fixes the LIVE tooltip collision (the shipped FreeText tooltip
    previously read "Add a text box…").
  - **Public API added to `pdfce-core`** (rule-10 trail, all
    `#[non_exhaustive]`, non-breaking): `fn preview_wrap(...) ->
    Result<AddTextWrapPreview, AddTextError>`; `struct
    AddTextWrapPreview { lines, wrapped_lines, box_overflow_lines,
    page_overflow_pt, alignment, disclosures }`; `struct
    WrapPreviewLine { text, origin_x, baseline_y }`; `Std14::ALL:
    [Std14; 14]`.
  - **Headless-tested:**
    `preview_wrap_lines_match_committed_boxed_add_for_identical_inputs`
    (parses `add_text`'s ACTUAL emitted `Tm` operands, asserts equality
    with the preview's per-line origins ±1e-4, across L/C/R/Justified,
    two faces, explicit+derived leading, and an R76 overflow case) +
    `preview_wrap_refuses_where_the_commit_would_refuse` (refusal
    parity); pure GUI helpers `resolve_drag_placement` + mutual-
    exclusion invariant tests `tool_builds_text_edit`/
    `tool_builds_add_text` (against the ACTUAL `SelectCanvasTool`
    dispatch). The egui wiring itself is compile-and-launch-verified,
    per this project's established GUI-testing posture.
  - **Gates (re-verified in the main tree):** fmt/clippy clean
    (panic-free); `cargo tree` core/render GUI-dep-free; full workspace
    0 failures (708 core lib + 18 core `add_text` [up from 16] + 59 GUI
    unit, plus CLI/doctests unchanged — 14.x/15.x/16.0/16.1/`vartext`
    all unchanged); release GUI build launches without panic; **ZERO
    new dependency**.

**Decisions made this session (continued):**
- **Five engineer judgment calls recorded** (defensible, non-blocking —
  filed to `ROADMAP.md`'s Shipped entry for the API-guidelines trail):
  (1) box mode was BUILT not blocked, since 16.0+16.1 both shipped by
  the time the `pdfce-ui-specialist` spec was implemented; (2) Enter
  semantics split by mode — point mode plain Enter = Accept; box mode
  plain Enter = paragraph break (`\n`), Ctrl+Enter = Accept; (3)
  `preview_wrap` returns the EXISTING `AddTextError` (no new error type
  invented), GUI stores/surfaces its `Display` string verbatim; (4)
  colour surface uses `color_edit_button_srgba` →
  `NewTextColor::Black|Rgb` only (no Gray/CMYK widget — matches core
  Add-Text's own colour-model limit, no phantom GUI capability); (5)
  `too_many_arguments`/`type_complexity` allow-with-reason on the
  9-field `layout_boxed`, matching the codebase's existing convention
  for justified, spec-driven parameter lists.
- **`ROADMAP.md` fully updated for the FF-D-complete milestone:** Pass
  16.2 moved to Shipped (top, reverse-chronological); the ★ Pass 16.x
  entry (Next up) got its closing AMENDMENT — "decision 016 and FF-D
  are COMPLETE end-to-end," now historical record same as the ★ Pass
  15.x and ★ Pass 14.x entries before it; the 16.2 bullet updated from
  "UI DESIGN SHIPPED, build in progress" to "SHIPPED — see Shipped
  above," with its acceptance criteria filled in; the "In progress"
  section's Pass-16 paragraph rewritten to "Pass 16.0, 16.1, AND 16.2
  all shipped … decision 016 / FF-D is now COMPLETE end-to-end."
- **NEW milestone paragraph recorded (both in the Shipped entry and in
  "In progress"): the broader Acrobat text-handling parity arc is
  COMPLETE at the P0 level** — decision 014 (in-place editing, Pass
  14.0–14.4), decision 015 / FF-A (within-block reflow incl. justified,
  Pass 15.0–15.2), and decision 016 / FF-D (add-new-text, Pass
  16.0–16.2) are ALL shipped, on top of the earlier root-cause font fix
  (continuation 33) and the xref-recovery work (Pass 13.x). Framed
  explicitly as a CLEAN DECISION POINT, not open engineering work: FF-B
  (cross-block/cross-page reflow) and FF-H (spacing/synthetic styles/
  StructTree) are lower-priority-deferred and unscheduled; FF-C (font
  subsetting) and list-authoring remain explicitly operator-gated. No
  Pass number invented for any of the four.

**Findings + decisions:**
- No new generalizable Rust/egui or PDF-domain finding this
  continuation — the five judgment calls above are pdfce-internal
  engineering findings specific to this build's UI-wiring/API-surface
  design, not ecosystem- or domain-generalizable discoveries, so
  nothing new was filed to `D:\dev\rag\rust\`, `D:\dev\rag\egui\`, or
  `C:\personal_rag\pdf\` this continuation.

**Still in flight (continuation 47):**
- **Decision 016 / FF-D is CLOSED — no remaining slices.** The
  text-parity arc (decisions 014 + 015 + 016) is now a complete,
  shipped P0 milestone; nothing is "in flight" within it.
- The certification-signature-guard gap on `add_text`/
  `EditSession::add_text` remains filed to Backlog, flagged, NOT
  actioned — this is now the most likely next bounded engineering step
  (no operator scope call needed to action it, unlike FF-C/
  list-authoring).
- FF-B, FF-H remain named, unscoped fast-follows — unchanged.
- FF-C and list-authoring remain explicitly operator-gated in
  `ROADMAP.md` Backlog — unchanged, no operator answer yet.
- The Beta (measurement/dimensioning) — unchanged, still awaiting
  operator go-ahead/sequencing confirmation.
- Everything remains UNCOMMITTED in git — the tree grew again this
  continuation (16.2's `CanvasTool::AddText` UI, `preview_wrap`, the
  tooltip disambiguation), closing out the FOUR-subsystem span named at
  continuation 46 (decision 014, decision 015, Pass 14.4, decision 016)
  as now FIVE complete subsystems sitting uncommitted end-to-end:
  in-place editing, reflow, GUI-polish, add-new-text, AND — new this
  continuation — the fact that the entire text-parity ARC is now a
  single completed, shippable-in-principle milestone with zero of it
  in version control. This is the largest uncommitted span in the
  project's history to date, larger again than at continuation 46.
- The autonomous `/loop` remains ACTIVE.

**Still-open operator items (re-surfaced, ordered oldest-first — no
items newly added or resolved this continuation; commit authorization
escalated further given FF-D's completion):**
- Encryption-refusal operator sign-off — oldest owed.
- `LEGAL.md` §1 license decision — still undecided. Doubly pointed: also
  gates unblocking FF-C (font subsetting) now that FF-D has shipped.
- `LEGAL.md` §2 Adobe-supplement copyright contradiction — flagged.
- `/R 6` sourcing method — Ken's call (gates Pass 5).
- **Commit authorization — escalating further this continuation.**
  Everything remains UNCOMMITTED; the tree now holds the ENTIRE
  Acrobat text-handling parity arc, complete and shipped in-fiction —
  decision 014 (in-place editing), decision 015 (reflow), Pass 14.4
  (GUI polish), and decision 016 (add-new-text, point + boxed + canvas
  UI) — sitting on top of a single bootstrap commit. This is the
  largest uncommitted span in the project's history, larger again than
  at continuation 46, and remains the highest-leverage unresolved item
  alongside the license decision.
- W15 — no remote/CI — unchanged.
- Autosave / crash-recovery scratch file + true in-place Save (gated on
  it) — still open, unchanged.
- List-authoring scope question (filed continuation 36; re-confirmed
  operator-gated by decision 016 §10) — does the operator want
  bulleted/numbered list authoring as an Acrobat-parity target at all,
  and if so where in the Pass sequence? Still no operator answer; now
  that FF-D has shipped, this sequences next if/when the operator says
  yes.
- **FF-C (font subsetting/glyph embedding) license/dependency gate**
  (filed continuation 44, decision 016 §10) — ranked #2 by value in the
  text-parity fast-follow ladder, now that FF-D (#1) has shipped end-to-
  end. Blocked on an explicit operator call: approve a permissive-only
  subsetter dependency (rule 13) and, ideally, settle `LEGAL.md` §1
  first. Recommendation stands: unblock now, since FF-D is done.
- Justified-alignment question — remains RESOLVED (decision 015,
  continuation 39); not re-opened, listed here only for continuity of
  the oldest-first trail, no action needed.

**For next session:**
- The Acrobat text-handling parity arc (decisions 014 + 015 + 016) is
  now a COMPLETE, closed P0 milestone — no further slices to build
  within it. The likely next bounded engineering step is the
  certification-signature-guard follow-up flagged at Pass 16.0's ship
  (add a guard to `add_text`/`EditSession::add_text`, mirroring
  `add_markup`'s existing check) — engineering-completeness work, not
  gated on an operator scope call, but not yet scheduled or dispatched.
- Get the operator's FF-C unblock call (now higher-value, since FF-D
  has shipped and FF-C is ranked #2) and a list-authoring scope answer
  — both still open, unchanged.
- Re-surface commit authorization and the license decision — both
  remain the two oldest-standing, highest-leverage unresolved operator
  items, now maximally pointed: the tree holds the entire completed
  text-parity arc, entirely uncommitted.
- Confirm the beta (measurement/dimensioning) sequencing whenever the
  operator wants to revisit it — unchanged, still open.

**Same-day continuation 48 — FF-D follow-up hardening SHIPPED
(certification-signature guard on `add_text`/`EditSession::add_text`,
closing the Pass 16.0 flagged gap); Backlog entry RESOLVED; the
FF-D/text-parity-arc milestone now has no known loose threads; the
autonomous `/loop` THROTTLED to a long idle heartbeat, AWAITING
OPERATOR STEER:**

**Shipped:**
- **FF-D follow-up hardening — certification-signature guard on
  `add_text`/`EditSession::add_text` (a correctness hardening, NOT a
  new Pass; closes the Backlog "FF-D follow-up" gap flagged at Pass
  16.0's ship).** Independently re-verified green in the main tree: 15
  CLI `add_text` tests (incl. the certified-refusal cases) pass, core
  lib 713 pass, and a live `add-text` against the certified fixture is
  refused with the verbatim §12.8.4 DocMDP message. Full record now in
  `ROADMAP.md` Shipped (top of section, above the Pass 16.2 entry);
  summary here for the session trail:
  - Adding page content to a certified-signed PDF whose enforced
    `/Perms /DocMDP` forbids structural changes is now REFUSED,
    mirroring `EditSession::add_markup`'s existing guard. Previously
    `add_text`/`EditSession::add_text` checked encryption and
    suppressed-objects only, not certification — closing exactly the
    gap flagged at Pass 16.0's ship.
  - `crates/pdfce-core/src/text_edit/addtext.rs` — new
    `AddTextError::CertificationForbidsChange { permission: u8 }`, its
    `#[error]` message a VERBATIM copy of
    `EditError::CertificationForbidsChange`'s (same wording, same ISO
    32000-1 §12.8.4 `/Perms /DocMDP P=` citation — reused, not
    reinvented, asserted by a message-parity unit test). New shared
    `pub(crate) fn refuse_if_certification_forbids<G: ObjectGraph>(graph)`
    reuses the SAME machinery as `EditSession::check_certification`
    (`crate::signature::census` + `SignatureCensus::forbids_structural_change()`
    + the `/P`-absent-defaults-to-2 rule). Wired into the free
    `add_text` engine between the encryption and suppressed-objects
    guards (matching `add_markup`'s
    encryption→certification→suppressed order).
  - `crates/pdfce-core/src/edit.rs` — `EditSession::add_text` calls the
    same shared guard in the same position; the boxed add shares the
    planner so it is covered automatically (tested).
  - `crates/pdfce-cli/src/main.rs` — `cmd_add_text` maps the new
    variant to `exit::EDIT_REFUSED`.
  - **Free-function guard posture chosen: (a)** — `census`/
    `forbids_structural_change` are already `pub` in `signature.rs` and
    reachable from the free function (`Document: ObjectGraph`), so the
    free `add_text` engine guards ITSELF via the shared helper, and
    both entry points (GUI `EditSession::add_text`, CLI/free
    `add_text`) call that ONE helper — every operator-reachable path is
    covered with zero drift, no unguarded entry remains. This also
    discharges the Backlog gap's item (2) ("expose a guard hook for
    other free-function engines") — the shared helper itself IS that
    hook.
  - **Fixture** `fixtures/synthetic/addtext/certified-locked.pdf`
    (`plain.pdf` + an enforced `/Perms /DocMDP` P=1 cert sig), added to
    `tools/gen-addtext-fixtures.py` (byte-stable/idempotent,
    md5-confirmed) + `PROVENANCE.md`.
  - **Tests:** core (point/box/free-fn refused with
    `CertificationForbidsChange { permission: 1 }`, session left
    unmodified; uncertified doc still adds — regression guard;
    message-parity) + CLI (point/box → `EDIT_REFUSED`, stderr cites
    §12.8.4, no output).
  - **Gates (re-verified main tree):** core lib 713 passed / 0 failed;
    CLI `add_text` 15 passed; `cargo test --workspace` all green;
    `cargo fmt --all --check` clean; `cargo clippy --workspace
    --all-targets -D warnings` clean; `cargo tree -p pdfce-core` /
    `-p pdfce-render` GUI-dep-free; **ZERO new dependency** (only
    intra-crate references — no `Cargo.toml`/`Cargo.lock` touched);
    Pass 14.x/15.x/16.x/`vartext` tests unchanged.

**Decisions made this session (continued):**
- **`ROADMAP.md` fully updated for the hardening's close-out:** a new
  Shipped entry ("FF-D follow-up hardening…") added at the TOP of
  Shipped (most recent); the Backlog "FF-D follow-up" bullet
  strikethrough-marked and annotated **RESOLVED 2026-08-01** with a
  pointer to the Shipped entry and posture (a) (matching the existing
  strikethrough+RESOLVED/CLOSED convention used elsewhere in Backlog —
  e.g. the i18n/cross-platform/update-mechanism entries); the ★ Pass
  16.x entry (Next up) got a new closing AMENDMENT recording the
  hardening and the now-clean decision point; both "In progress"
  paragraphs that named the gap as "the most likely next bounded
  engineering step" were updated to say it shipped and is closed.
- **Posture (a) confirmed as the chosen (not merely proposed) shape**
  for the guard: the free `add_text` function guards itself directly,
  rather than `EditSession` exposing `check_certification` outward to
  free functions or duplicating a second private check. Recorded as
  the reusable pattern for any future free-function engine (e.g. a
  hypothetical 15.x-reflow-adjacent engine) that needs the same
  certification guard — reach for `signature::census` +
  `forbids_structural_change()` directly, the same way `add_text` now
  does, rather than inventing a new access path through `EditSession`.
- **With this hardening, the FF-D/text-parity-arc milestone (decisions
  014 + 015 + 016, Pass 14.0–16.2) has no known loose threads.** The
  only remaining items in that space — FF-B, FF-H (lower-priority-
  deferred, unscheduled) and FF-C, list-authoring (explicitly
  operator-gated) — are all clean decision points, not dangling
  engineering gaps. This is now recorded in `ROADMAP.md`'s "In
  progress" milestone paragraph.

**Findings + decisions:**
- No new generalizable Rust/egui or PDF-domain finding this
  continuation — the guard-sharing pattern (posture (a)) is a
  pdfce-internal engineering decision about this codebase's own crate
  boundaries (`signature.rs`'s `pub` visibility, `EditSession` vs. free
  functions), not an ecosystem- or domain-generalizable discovery, so
  nothing new was filed to `D:\dev\rag\rust\`, `D:\dev\rag\egui\`, or
  `C:\personal_rag\pdf\` this continuation.

**Still in flight:**
- **Nothing is in flight within the text-parity arc.** Decision 016/
  FF-D is complete end-to-end AND its one flagged follow-up is now
  closed — the arc (decisions 014+015+016) is a fully shipped,
  fully-hardened P0 milestone with no open engineering thread.
- FF-B, FF-H remain named, unscoped fast-follows — unchanged.
- FF-C and list-authoring remain explicitly operator-gated in
  `ROADMAP.md` Backlog — unchanged, no operator answer yet.
- The Beta (measurement/dimensioning) — unchanged, still awaiting
  operator go-ahead/sequencing confirmation.
- **The autonomous `/loop` is now THROTTLED to a long idle heartbeat,
  AWAITING OPERATOR STEER on the next major direction** — it is not
  spawning further feature work. With the text-parity arc's one known
  loose thread now closed, there is no more self-evident, non-operator-
  gated bounded engineering step queued; further progress in this
  project's highest-value area (text parity) now requires an operator
  call (FF-C unblock, list-authoring scope, or a new direction
  entirely) rather than autonomous continuation.
- Everything remains UNCOMMITTED in git — this hardening (the new
  `AddTextError` variant, the shared `refuse_if_certification_forbids`
  helper, the `certified-locked.pdf` fixture, and their tests) adds to,
  rather than resolves, the largest uncommitted span named at
  continuation 47 (the entire completed text-parity arc). The whole
  arc PLUS this closing fix now sit on top of a single bootstrap commit
  with zero of it in version control.

**Still-open operator items (re-surfaced, oldest-first; none newly
resolved this continuation except the FF-D follow-up gap itself, which
is now closed and dropped from this list):**
- Encryption-refusal operator sign-off — oldest owed, unchanged.
- `LEGAL.md` §1 license decision — still undecided. Doubly pointed:
  also gates unblocking FF-C (font subsetting) now that FF-D (incl.
  this hardening) is fully shipped and closed.
- `LEGAL.md` §2 Adobe-supplement copyright contradiction — flagged,
  unchanged.
- `/R 6` sourcing method — Ken's call (gates Pass 5) — unchanged.
- **Commit authorization — escalated further this continuation.**
  Everything remains UNCOMMITTED; the tree now holds the ENTIRE
  Acrobat text-handling parity arc, complete, shipped, AND
  hardening-closed in-fiction — decision 014 (in-place editing),
  decision 015 (reflow), Pass 14.4 (GUI polish), decision 016
  (add-new-text, point + boxed + canvas UI), and now this
  certification-signature-guard fix — sitting on top of a single
  bootstrap commit. This is the largest uncommitted span in the
  project's history yet, larger again than at continuation 47, and
  remains the highest-leverage unresolved item alongside the license
  decision.
- W15 — no remote/CI — unchanged.
- Autosave / crash-recovery scratch file + true in-place Save (gated on
  it) — still open, unchanged.
- List-authoring scope question (filed continuation 36; re-confirmed
  operator-gated by decision 016 §10) — does the operator want
  bulleted/numbered list authoring as an Acrobat-parity target at all,
  and if so where in the Pass sequence? Still no operator answer.
- **FF-C (font subsetting/glyph embedding) license/dependency gate**
  (filed continuation 44, decision 016 §10) — ranked #2 by value in the
  text-parity fast-follow ladder. Blocked on an explicit operator call:
  approve a permissive-only subsetter dependency (rule 13) and,
  ideally, settle `LEGAL.md` §1 first. Recommendation stands: unblock
  now, since FF-D (including this hardening) is fully done and closed.
- Justified-alignment question — remains RESOLVED (decision 015,
  continuation 39); not re-opened, listed here only for continuity of
  the oldest-first trail, no action needed.

**For next session:**
- **The text-parity arc (decisions 014+015+016) is now fully shipped
  AND fully hardened — zero known open engineering threads within it.**
  The `/loop` is throttled to an idle heartbeat and will not spawn new
  feature work on its own; the next move in this space is an operator
  decision, not autonomous engineering.
- Get the operator's FF-C unblock call and a list-authoring scope
  answer — both still open, unchanged, now the two live decision points
  in the text-parity space.
- Re-surface commit authorization and the license decision — both
  remain the two oldest-standing, highest-leverage unresolved operator
  items, now maximally pointed: the tree holds the entire completed and
  hardened text-parity arc, entirely uncommitted.
- Confirm the beta (measurement/dimensioning) sequencing whenever the
  operator wants to revisit it, OR use this idle window to give the
  `/loop` its next major-direction steer — unchanged, still open.

**Same-day continuation 49 — FIRST IMPLEMENTATION COMMIT (operator-
authorized, LOCAL ONLY): commit `d8b3903` on branch `pass-8-redaction`;
the "commit authorization" operator item is DISCHARGED — push remains
gated on the `LEGAL.md` §1 license decision:**

**Shipped:**
- No new Pass this continuation — this is a git-process/institutional
  milestone, not a feature Pass. The operator said "commit all work";
  the engineer performed the project's FIRST implementation commit
  since the 2026-07-23 bootstrap commit (`67967b2`).
  - **Commit `d8b3903`** on branch **`pass-8-redaction`** (branched
    from `67967b2`): **373 files changed, 168,217 insertions.** Title:
    "Implement pdfce: full PDF read/write/edit stack + Acrobat
    text-handling parity."
  - **LOCAL ONLY — NOT pushed to any remote.** `LEGAL.md` §1 (OSS
    license choice) remains undecided, and project rule 8 explicitly
    forbids publishing to a public repository, a release, or
    describing the project as "open source" before that decision is
    made. A local commit satisfies "commit all work" without crossing
    that line; pushing is a separate, still-ungranted authorization.
  - `.gitignore` was extended BEFORE committing (proactive, not
    asked-for) to exclude generated/scratch artifacts:
    `/fixtures/external-report.tsv`; `tools/**/out/` and
    `tools/**/out-*/` (corpus-harness output dirs); `/.claude/worktrees/`;
    `/demo_*.pdf` and `/demo_*.png`. `target/` (~20GB) and
    `/fixtures/external/` (354MB real-world OSS corpus) were already
    ignored from bootstrap.
  - **Committed:** all crate source (`crates/`); `docs/decisions/`
    (17-file KenAgent decision log); `docs/ui_specs/`; synthetic-only
    fixtures (`fixtures/synthetic/`); corpus-harness SOURCE plus fuzz
    targets/seeds (`tools/`); `.claude/agent-memory/` and the
    inkscape-librarian agent definition; launchers (`pdfce.bat`/
    `pdfce.ps1`); `cargo-about` scaffolding (`about.toml`/`about.hbs`);
    `Cargo.toml`/`Cargo.lock`; `rust-toolchain.toml`;
    `THIRD_PARTY_LICENSES.md`.
  - **Explicitly NOT committed:** build artifacts (`target/`), the
    external real-world OSS PDF corpus, harness output/report
    directories, any proprietary or unknown-provenance PDFs, and no
    secrets/tokens are present in the commit.
  - **Working tree is now clean** — nothing uncommitted except
    gitignored artifacts. This RETIRES the "entire project
    uncommitted" risk tracked since at least Pass 14.0 (see
    `D:\Dev\pdfce\.claude\agent-memory\pdfce-librarian\project_uncommitted_repo_worktree_risk.md`,
    now marked RESOLVED) and closes out the largest-uncommitted-span
    framing repeated through continuations 39/42/46/47/48.

**Decisions made this session:**
- **Operator granted commit authorization for the first time this
  project — the long-standing #1 open operator item.** Scope as
  executed: LOCAL commit only. The engineer deliberately did NOT read
  "commit all work" as also authorizing a push, given the still-open
  `LEGAL.md` §1 license decision and project rule 8's explicit
  prohibition on a public-facing commit posture before that decision.
  If the operator intends to also authorize a push, that needs its own
  explicit confirmation in a future session — it is not implied by
  this one.
- `.gitignore` was extended proactively before the commit so generated/
  scratch/output artifacts never entered version control in the first
  place, rather than committing them and cleaning up after.

**Findings + decisions:**
- No new generalizable Rust/egui or PDF-domain finding this
  continuation — this is pdfce's own git-process housekeeping, not an
  ecosystem- or domain-generalizable discovery. Nothing filed to
  `D:\dev\rag\rust\`, `D:\dev\rag\egui\`, or `C:\personal_rag\pdf\`.

**Still in flight:**
- Unchanged from continuation 48: the text-parity arc (decisions
  014+015+016) is fully shipped and hardened; the `/loop` remains
  throttled to an idle heartbeat awaiting operator steer; the Beta
  (measurement/dimensioning) awaits operator go-ahead.
- **The one structural change: the tree is no longer uncommitted.**
  Future autonomous-builder `git worktree` dispatches now check out
  real, current content instead of the stale 2026-07-23 bootstrap
  commit — retiring the recurring worktree-isolation cost documented
  in `D:\dev\rag\rust\autonomous_builder_worktree_isolation_uncommitted_substrate.md`.
  New work from this point commits normally onto `d8b3903` (or
  whatever branch it lands on); it does not reopen this risk unless a
  future session again lets uncommitted work pile up.

**Still-open operator items (re-surfaced, oldest-first; COMMIT
AUTHORIZATION REMOVED THIS CONTINUATION — discharged, see above):**
- Encryption-refusal operator sign-off — oldest owed, unchanged.
- `LEGAL.md` §1 license decision — still undecided. **Now doubly
  pointed in a new way:** it already gated the FF-C unblock (unchanged)
  and now ALSO gates whether the just-made local commit (`d8b3903`) may
  ever be PUSHED to a remote/public repository (project rule 8). The
  local commit does not resolve this decision — it raises the stakes
  of getting it right, since a push call is now the only remaining
  step between the working tree and public visibility.
- `LEGAL.md` §2 Adobe-supplement copyright contradiction — flagged,
  unchanged.
- `/R 6` sourcing method — Ken's call (gates Pass 5) — unchanged.
- W15 — no remote/CI — unchanged; also relevant to the push question
  above, since no CI exists to validate a pushed branch yet regardless.
- Autosave / crash-recovery scratch file + true in-place Save (gated on
  it) — still open, unchanged.
- List-authoring scope question — still no operator answer, unchanged.
- FF-C (font subsetting/glyph embedding) license/dependency gate —
  unchanged; recommendation stands to unblock now that FF-D is done.
- Justified-alignment question — remains RESOLVED (decision 015,
  continuation 39); listed only for the oldest-first trail's
  continuity, no action needed.
- ~~Commit authorization~~ — **DISCHARGED this continuation.** Local
  commit `d8b3903` performed on branch `pass-8-redaction`, operator-
  authorized. Dropped from active tracking; superseded by the
  push/license question above, which is now the load-bearing open item
  in this space.

**For next session:**
- **Get the operator's explicit call on PUSHING** — a separate
  authorization from the local-commit one just exercised, gated on
  `LEGAL.md` §1. Do not push without it.
- Everything else unchanged from continuation 48: FF-C unblock,
  list-authoring scope answer, Beta sequencing confirmation, or a new
  `/loop` major-direction steer.

**Same-day continuation 50 — LICENSE DECIDED (MIT) + NEW OPERATOR
PRIORITY SEQUENCE SET: dimensioning tool (active) → GUI icons → finish
text-handling → form-building tools:**

**Shipped:**
- No new feature Pass this continuation — this is a legal-decision +
  reprioritization milestone, not a build.
- **License artifacts implemented (engineer, this continuation):**
  repo-root `LICENSE` (standard MIT text, "Copyright (c) 2026 Ken
  Mantle"); `license = "MIT"` in `Cargo.toml` `[workspace.package]`;
  `license.workspace = true` added to all four member crates
  (`pdfce-core`, `pdfce-render`, `pdfce-gui`, `pdfce-cli`) —
  `cargo metadata` confirms each resolves to MIT.
- **Dependency-license audit performed as part of the decision:**
  every dependency in the lockfile is permissive (MIT/Apache-2.0/BSD/
  ISC/Zlib/Unicode) — zero copyleft — verified against
  `THIRD_PARTY_LICENSES.md`. MIT is fully compatible; no dependency
  needed to change.

**Decisions made this session:**
- **`LEGAL.md` §1 — OSS license DECIDED: MIT.** Operator's explicit
  choice, delivered in the same instruction as the new work-focus
  directive below. Recorded in `docs/LEGAL.md` §1/§6.1/§7 and
  `docs/ARCHITECTURE.md` §12 (both updated this continuation, mirrored
  entries) — see "Files touched" below.
  - **Consequence (binding, recorded in both files):** GPL/AGPL prior
    art — MuPDF, Poppler, Ghostscript (`docs/PRIOR_ART.md`) — is now
    categorically and *permanently* excluded as a real dependency.
    This was already the practical posture (nothing copyleft was ever
    adopted), but the license decision forecloses the hypothetical
    "choose AGPL instead and unlock them" branch for good.
  - **Project rule 8's license precondition is now satisfied** — pdfce
    may be described as having a real, settled license, and a
    public-facing commit posture is no longer blocked *by the license
    question specifically*.
  - **Push/publish is explicitly NOT authorized by this decision.**
    The operator asked for the license choice and the new priority
    sequence (below) — not a push. The existing local commit
    (`d8b3903`, continuation 49) stays local-only until a separate,
    explicit go-ahead is given. This is a narrower, distinct open item
    from the license decision itself — see the still-open list below.
- **Operator set a four-item priority sequence for upcoming work**
  (verbatim: *"get the dimensioning tool completely functional in the
  gui interface. add d:/dev/scriptree style icons for all gui
  features. finish off all the text handling stuff. work on form
  building tools after if that makes sense."*). Recorded in
  `ROADMAP.md` as a new "★★★ Operator priority sequence" block at the
  top of "Next up":
  1. **Dimensioning tool → completely functional in the GUI.** Promoted
     from "awaits go-ahead" to **ACTIVE**. Current state: only Pass
     **12.0** (canvas substrate, uninhabited) is shipped; the four
     remaining decision-011 slices — **9a** (object/selection model +
     centerline), **12.M1** (snapping), **12.M2** (dimensioning +
     scale/group + hybrid storage + OCG), **9c-min** (basic editing:
     move/delete/drag-node) — are NOT built. **Pass 9a dispatched to
     build now** (first of the four, per decision 011's own dependency
     order). **`pdfce-inkscape-librarian` dispatched now** for the
     9a/12.M1 grounding (selection + snapping capability bucket).
     **Queued for the 12.M1/12.M2 stage** (not yet dispatched):
     `pdfce-spec-librarian` (§12.9 measurement / §14.5 optional
     content / §8.11 measurement dicts), `pdfce-acrobat-librarian`
     (measuring-tools bucket), `pdfce-ui-specialist` (dimension-tool
     canvas UX) — decision 011's own prerequisite grounding was already
     sourced earlier, so these are queued for their build stage, not a
     fresh research round.
  2. **ScripTree-style SVG icons for all GUI features.** NEW backlog
     item, filed this continuation as a new "★ Icon set" entry under
     Next up — not yet scoped to a Pass. Styled after
     `D:\Dev\ScripTree\icons\*.svg`; applies across every current GUI
     tool/feature, present and future (including the dimensioning tool
     once built). Queued behind the dimensioning tool.
  3. **Finish all text-handling.** FF-B (cross-block/cross-page
     reflow), FF-H (spacing/synthetic styles + StructTree), and FF-C
     (font subsetting/glyph embedding) are now all schedulable — FF-C
     specifically because its rule-8 license gate is lifted by the MIT
     decision, and because the operator's "finish off all the text
     handling stuff" instruction is itself the go-ahead decision 016
     was waiting on. Rule 13 (copyleft flag) still applies to whichever
     concrete subsetter crate is chosen for FF-C — the license decision
     doesn't pre-clear an unverified crate. **List-authoring is
     explicitly NOT resolved by this instruction** — it's a separate,
     still-open scope question (see below); "text handling" and "list
     authoring" are tracked as two distinct open items in `ROADMAP.md`,
     and the operator did not answer the list-authoring one.
  4. **Form-building tools, after — "if that makes sense."** Queued
     last. This is form-FIELD CREATION/authoring (new AcroForm fields),
     distinct from the shipped Pass 7.0/7.1 form-FILL/flatten
     subsystem. Recorded as an amendment to the existing "Forms
     (AcroForm)" Backlog bucket. The operator's own hedge ("if that
     makes sense") is noted verbatim — re-evaluate scope when items 1–3
     are done rather than treating it as unconditionally committed.

**Findings + decisions:**
- No new generalizable Rust/egui or PDF-domain finding this
  continuation — this is a legal/process decision plus a
  reprioritization instruction, not an empirical or ecosystem finding.
  Nothing filed to `D:\dev\rag\rust\`, `D:\dev\rag\egui\`, or
  `C:\personal_rag\pdf\`.
- **Files touched this continuation:** `docs/LEGAL.md` (§1 rewritten
  DECIDED, §6.1 amended, §7 new dated entry); `docs/ARCHITECTURE.md`
  (§12 new dated entry, §9 body amended to reflect the decided
  license); `docs/ROADMAP.md` ("In progress" GIT STATUS note amended;
  Beta/dimensioning entry promoted to ACTIVE with dispatch detail; new
  "★★★ Operator priority sequence" and "★ Icon set" entries added under
  Next up; the FF-D text-parity milestone paragraph amended; the FF-C
  Backlog bullet marked UNBLOCKED; the "Forms (AcroForm)" Backlog
  bucket amended with the form-building-tools priority note; the
  "Release & distribution channel" Backlog bullet amended — license
  manifest-property blocker lifted, still blocked on the separate
  push/publish authorization).
- **Flag for the operator (not an edit made by the librarian):**
  `C:\Users\Ken\.claude\CLAUDE.md`'s project-instructions mirror in
  `D:\Dev\pdfce\CLAUDE.md` lists "OSS license — not yet chosen" under
  "Outstanding open items." That file is engineer/operator-owned, not
  the librarian's to edit — flagging so the engineer updates it to
  reflect the MIT decision in a future pass over that file.
- **Also noticed, not actioned:** `ARCHITECTURE.md` §12's decision log
  has no entry for decision 016 (FF-D) — Pass 16.0/16.1/16.2 and the
  FF-D follow-up hardening are all recorded in `ROADMAP.md` and in
  `docs/decisions/016-ffd-add-new-page-text.md`, but the §12 mirror
  entry was apparently never written. Out of scope for this
  continuation's dispatch (MIT decision + reprioritization only); flag
  for a future "decision log entry" dispatch to backfill.

**Still in flight:**
- The Beta (measurement/dimensioning) is now the project's ACTIVE
  focus (see above) — Pass 9a is dispatched and building.
- The text-parity fast-follow ladder (FF-B/FF-C/FF-H) is now
  operator-directed to finish, but not yet scoped to specific Passes
  or dispatched to build — that scoping is upcoming work, sequenced
  behind the dimensioning tool and the icon set per the priority
  order.
- The icon-set work and the form-building-tools work are both filed as
  new backlog items this continuation; neither has started.
- The `/loop`'s prior "throttled, awaiting operator steer" status
  (continuation 48) is now resolved — the operator steer arrived this
  continuation. Whether the `/loop` reactivates autonomously or the
  engineer works interactively through this priority sequence is an
  engineer-session call, not recorded here.

**Still-open operator items (re-surfaced, oldest-first):**
- Encryption-refusal operator sign-off — oldest owed, unchanged.
- ~~`LEGAL.md` §1 license decision~~ — **DECIDED this continuation:
  MIT.** Removed from active tracking as an open item; see "Decisions
  made this session" above for the full record.
- **Push/publish authorization — NEW, narrow, optional item, split
  off from the now-resolved license decision.** The local commit
  `d8b3903` (continuation 49) remains unpushed; MIT satisfies rule 8's
  license precondition, but the operator has not asked for a push or a
  public release, and none should happen without that separate,
  explicit go-ahead.
- `LEGAL.md` §2 Adobe-supplement copyright contradiction — flagged,
  unchanged.
- `/R 6` sourcing method — Ken's call (gates Pass 5) — unchanged.
- W15 — no remote/CI — unchanged; also relevant to the push question
  above, since no CI exists to validate a pushed branch yet regardless.
- Autosave / crash-recovery scratch file + true in-place Save (gated on
  it) — still open, unchanged.
- List-authoring scope question — still no operator answer, unchanged.
  **Explicitly NOT answered by this continuation's "finish off all the
  text handling stuff" instruction** — see "Decisions made this
  session" item 3 above for why the two are tracked separately.
- FF-C (font subsetting/glyph embedding) license/dependency gate —
  **RESOLVED this continuation** (MIT decision lifts rule 8; operator
  directive is the go-ahead). Kept in this list one more continuation
  for the oldest-first trail's continuity, then drop it once a Pass
  number is assigned.
- Justified-alignment question — remains RESOLVED (decision 015,
  continuation 39); listed only for the oldest-first trail's
  continuity, no action needed.

**For next session:**
- Continue/complete **Pass 9a** (object/selection model + centerline)
  and proceed through decision 011's sequence (12.M1 → 12.M2 →
  9c-min), dispatching `pdfce-spec-librarian` / `pdfce-acrobat-librarian`
  / `pdfce-ui-specialist` at the 12.M1/12.M2 stage as queued.
  Dimensioning-tool completeness (priority #1) is the top of the queue.
  - **When the beta lands "completely functional," move to priority
    #2 (ScripTree-style icons)**, dispatching `pdfce-ui-specialist`
    for the icon→feature mapping first.
  - **Then priority #3** (FF-B/FF-C/FF-H) — pick a permissive-only
    subsetter for FF-C and flag it per rule 13 even though rule 8 is
    clear; scope FF-B/FF-H into real Passes off the existing decision
    016 §2 prioritization.
  - **Then priority #4** (form-building tools) — re-evaluate scope
    against the "if that makes sense" hedge before committing, and
    dispatch `pdfce-acrobat-librarian` for the field-creation
    capability bucket first.
- Get the operator's explicit call on **pushing** the existing local
  commit (or a later one) — separate from, and not implied by, the
  MIT decision.
- Backfill `ARCHITECTURE.md` §12 with a decision-016 (FF-D) entry —
  flagged above as a pre-existing gap noticed but not fixed this
  continuation.

**Same-day continuation 51 — Pass 9a SHIPPED and COMMITTED (`e13f3e6`):**

**Shipped:**
- **Pass 9a** — read-only vector object/selection model + centerline
  derivation (decision 011 slice 2 of 5; the first BUILDABLE slice on
  top of Pass 12.0's uninhabited canvas substrate). NEW
  `crates/pdfce-core/src/vector/` module (`mod.rs`, `geometry.rs`,
  `decompose.rs` ~1000 lines, `hit.rs`, `centerline.rs`): decomposes a
  page's content-token stream into selectable `PathObject`/
  `TextObject`/`ImageObject` nodes (user+page space, effective
  graphics state, a captured-not-yet-used content-token
  `TokenRange`/`ByteSpan` editing handle reserved for 9c-min); point +
  marquee hit-testing; thin-filled-bar centerline derivation
  (`CENTERLINE_ASPECT_THRESHOLD = 8.0`) as a confirmable, never
  auto-applied dimensioning hint. Additive `pdfce-render` cross-check
  hook (`trace_paths`) returns `None` on every render/save path — zero
  output-byte impact. New `pdfce-gui::object_provider::
  ObjectModelProvider` wires selection onto the Pass 12.0 canvas.
  Fixtures `fixtures/synthetic/vector/{paths,curves,mixed,
  centerline}.pdf`; fuzz target `vector_decompose` (686k execs, 0
  crashes).
- **Verification (re-confirmed independently in the main tree):** full
  workspace `cargo test` all green — core lib **749** (up from 713,
  +36 new vector tests), cross-check + provider + fuzz all pass;
  `cargo fmt --all --check` clean; `cargo clippy --workspace
  --all-targets -D warnings` clean; `cargo tree -p pdfce-core` /
  `-p pdfce-render` GUI-dep-free (invariant intact); **zero new Cargo
  dependencies**.
- **Committed as `e13f3e6`**, on top of `79d1c6f` (the MIT-license
  artifacts commit, itself on top of `d8b3903` the first
  implementation commit). Both `79d1c6f` and `e13f3e6` are local-only —
  same not-yet-pushed posture, push authorization still a separate,
  not-yet-granted operator item. **Commit-cadence note:** the engineer
  is now landing shipped work as logical per-Pass/per-decision commits
  (license artifacts, then Pass 9a) rather than repeating the single
  large tree-wide commit made at continuation 49 — a deliberate
  cadence change worth tracking going forward.
- Full `ROADMAP.md` record: Shipped section (new Pass 9a entry, top of
  file); "In progress" Beta section amended (state now 2 of 5 slices
  shipped, Pass 12.M1 promoted from dispatched to in-progress); GIT
  STATUS note amended with the `e13f3e6` commit.

**Decisions made this session:** none new — Pass 9a executes decision
011's already-decided architecture; no fresh KenAgent decision filed.

**Findings + decisions:**
- **Dimensioning-tool state, precisely:** Pass 12.0 (canvas substrate)
  and Pass 9a (object/selection model + centerline) are both shipped;
  **Pass 12.M1 (snapping engine) is next, now in progress**; Pass 12.M2
  (dimensioning + scale/group + hybrid storage + OCG layer) and Pass
  9c-min (basic vector editing) remain after that. 12.M2 is already
  fully grounded for its eventual build — spec §12.9 (measurement),
  §14.5 (optional content/OCG), and §8.11 (measurement dictionaries)
  plus the Acrobat measuring-tool capability bucket and the Inkscape
  selection/snapping capability bucket are all catalogued in their
  respective RAGs. `pdfce-ui-specialist` is designing the Pass 12.M2
  dimension-tool canvas UX now, in parallel with the 12.M1 engineering
  build.
- **Two flags filed to `ROADMAP.md` Backlog this continuation (recorded,
  NOT actioned):**
  1. Marquee-vs-pan canvas-drag default change (Pass 9a repurposed
     plain-drag from pan to rubber-band marquee select, moving pan to
     wheel/scrollbars — the Inkscape/Illustrator convention). Owed a
     `pdfce-ui-specialist` review, slated for the Pass 12.M1 dispatch
     since that same specialist is already engaged on 12.M2's dimension
     UX and can fold the question in rather than a separate round-trip.
  2. Integration-test temp-path collision risk (low severity,
     non-blocking): some tests build temp paths from
     `std::env::temp_dir()` + `process::id()`, which is process-unique
     but not thread-unique, so parallel `cargo test --workspace` runs
     can theoretically collide. A build agent saw one transient,
     non-reproducing `RecoveredBaseForbidsIncremental` failure under a
     full parallel run that did NOT reproduce on the clean main-tree
     verification run (which was fully green). Filed as a test-hygiene
     hardening item (add a thread-unique counter alongside the PID) —
     explicitly NOT a product bug and NOT blocking Pass 9a's ship.
- No new generalizable Rust/egui or PDF-domain finding graduated to
  `D:\dev\rag\rust\`, `D:\dev\rag\egui\`, or `C:\personal_rag\pdf\` this
  continuation — the agree-by-construction pattern (shared
  render/object-model primitives as the correctness oracle) is
  pdfce-architecture-specific enough that it's recorded in the Pass 9a
  Shipped entry rather than generalized; revisit if a future project
  hits the same render-vs-model-drift problem.

**Still in flight:**
- Pass 12.M1 (snapping engine) is now the active build — next slice in
  decision 011's dependency chain.
- The marquee-vs-pan UX flag and the temp-path test-hygiene flag are
  both open Backlog items, unscheduled to a specific Pass.
- Priority items #2 (icon set), #3 (finish text-handling: FF-B/FF-H/
  FF-C), and #4 (form-building tools) from the continuation-50 operator
  sequence are all unchanged — still queued behind the dimensioning
  tool, no work started on any of them.
- Push/publish authorization for the local commits (`79d1c6f`,
  `e13f3e6`, and the earlier `d8b3903`) remains a separate, not-yet-
  granted operator item.

**For next session:**
- Continue decision 011's sequence: build **Pass 12.M1** (snapping
  engine) to completion, then **Pass 12.M2** (dimensioning UI — already
  grounded by spec/Acrobat/Inkscape research, `pdfce-ui-specialist`
  design in progress), then **9c-min** (basic vector editing).
- Fold the marquee-vs-pan UX review into the 12.M1 or 12.M2
  `pdfce-ui-specialist` engagement — don't let it go unreviewed past
  12.M2's ship.
- Pick up the temp-path test-hygiene hardening opportunistically (no
  Pass number assigned; not urgent).
- Still owed: operator push/publish call; `ARCHITECTURE.md` §12
  decision-016 backfill (flagged continuation 50, still not done);
  encryption-refusal sign-off; the other oldest-first still-open items
  carried from continuation 50 (see that continuation's list —
  unchanged this continuation, not re-enumerated here to avoid drift
  between two near-duplicate lists).

**Same-day continuation 52 — Pass 12.M1 SHIPPED and COMMITTED (`801a748`):**

**Shipped:**
- **Pass 12.M1** — snapping engine + fuzzy snap indicator (decision 011
  slice 3 of 5). NEW `crates/pdfce-core/src/vector/snap.rs`:
  tool-agnostic `snap_candidates(query, &SnapConfig, &PageObjects) ->
  Vec<SnapCandidate>` over Pass 9a's object geometry. Seven-level
  priority (`Node < Endpoint < Center < Midpoint < Intersection <
  DerivedCenterline < SegmentCenterline < Axis`, 8 `SnapKind`s incl.
  the derived filled-quad midline), deterministic
  `(priority, distance, x, y, source)` tie-break + `1e-3` coincident-
  point dedup, H/V axis constraint correct at 0/90/180/270°,
  zoom-invariant screen→page tolerance (`px/zoom`). Intersection-snap
  defaults OFF and is neighbourhood-bounded
  (`near_query_segments` bbox pre-filter,
  `MAX_NEIGHBOURHOOD_SEGMENTS = 256`, no global all-pairs search — the
  Inkscape-freeze precedent, Z4 mitigation, cited in-code). pdfce-gui:
  fuzzy snap indicator (per-`SnapKind` marker glyph + type label
  pre-commit, Tab-cycle ties, Alt-override, master toggle; distinct
  glyph + two-click confirm for the derived centerline —
  fuzzy-never-sneaky). New `ObjectModelProvider::page_objects()`
  exposes the ONE per-page decomposition to both selection and
  snapping (swapped `OpenDoc`'s boxed `dyn` target-provider for a
  concrete `object_model` field + on-demand `target_provider()` —
  closes the double-decompose risk a literal "add a second field"
  reading would have reintroduced). `vector_snap` fuzz target added.
  **Public API (rule-10 trail):** `pdfce_core::vector::{SnapKind,
  SnapCandidate, SnapConfig, AxisConstraint, snap_candidates,
  constrained_second_point, measured_length, SNAP_FLATTEN_STEPS,
  MAX_NEIGHBOURHOOD_SEGMENTS, MAX_CANDIDATES}`; gui
  `ObjectModelProvider::page_objects` + canvas snap-indicator helpers.
- **Marquee-vs-pan UX flag (owed since Pass 9a) — RESOLVED, KEPT.**
  `pdfce-ui-specialist` reviewed the Pass-9a drag-to-marquee-select
  change during this Pass's dispatch: Measure/dimension tools use
  click-point-A-then-click-point-B, not drag, so marquee-select-drag
  and dimension-picking never contend for the same gesture. No
  behavior change from Pass 9a's shipped default. Backlog entry marked
  RESOLVED.
- **Verification (re-confirmed independently in the main tree):** full
  workspace `cargo test` — core lib **772** (up from 749 at Pass 9a's
  ship, **+23 new snap tests**), 70 GUI tests, all green; `cargo fmt
  --all --check` clean; `cargo clippy --workspace --all-targets -D
  warnings` clean; `cargo tree -p pdfce-core` / `-p pdfce-render`
  GUI-dep-free (core also confirmed free of `tiny-skia`); GUI release
  build launches; **zero new Cargo dependencies**.
- **Committed as `801a748`**, on top of `19ed865` (docs), `e13f3e6`
  (Pass 9a), `79d1c6f` (MIT license artifacts), and `d8b3903` (first
  implementation commit). All five remain **local-only** — push
  authorization still a separate, not-yet-granted operator item.
- Full `ROADMAP.md` record: new Pass 12.M1 Shipped entry (top of
  Shipped, above Pass 9a); "In progress" Beta section amended (state
  now 3 of 5 slices shipped, Pass 12.M2 promoted from queued to
  in-progress, `pdfce-engineer` dispatched to build it this same
  continuation); marquee-vs-pan Backlog flag amended to RESOLVED;
  GIT STATUS note amended with the `19ed865`/`801a748` commits;
  ★★★ operator priority sequence item 1 state line updated.

**Decisions made this session:** none new — Pass 12.M1 executes
decision 011's already-decided architecture; no fresh KenAgent decision
filed. The five engineer judgment calls made during the build (Node-
vs-Endpoint semantics; Center = bbox-centre of an all-cubic closed
subpath, Taubin best-fit deferred to 12.M2 under the same `SnapKind`;
`SnapConfig` struct chosen over a bare parameter list; bbox-corner
snapping explicitly OUT as a documented fast-follow; concrete
`object_model` field replacing the boxed `dyn` provider) are recorded
in full in the Pass 12.M1 Shipped entry, not repeated here.

**Findings + decisions:**
- **Dimensioning-tool state, precisely:** Pass 12.0 (canvas substrate),
  Pass 9a (object/selection model + centerline), and Pass 12.M1
  (snapping) are all shipped — 3 of decision 011's 5 slices. **Pass
  12.M2 (dimensioning + scale/group + hybrid storage + OCG layer) is
  dispatched and now the active build** — it was already fully
  grounded before this continuation (spec §12.9/§14.5/§8.11, the
  Acrobat measuring-tool bucket, the Inkscape snapping bucket, and the
  `pdfce-ui-specialist` dimension-tool UX design all in hand), so no
  further research round-trip was needed to start it. **9c-min** (basic
  vector editing) remains after 12.M2, last of the five slices.
  Nothing is "completely functional in the GUI" yet — that milestone
  lands with 12.M2 and/or 9c-min.
- The marquee-vs-pan UX flag from Pass 9a is now fully closed (see
  Shipped above) — no open UX review debt carried forward on the
  dimensioning-tool beta.
- No new generalizable Rust/egui or PDF-domain finding graduated to
  `D:\dev\rag\rust\`, `D:\dev\rag\egui\`, or `C:\personal_rag\pdf\`
  this continuation — the snapping-engine priority/tie-break design is
  pdfce-architecture-specific (built directly on Pass 9a's
  `PageObjects`) rather than a generalizable Rust/egui-ecosystem or
  PDF-domain finding; revisit if a future project needs a similar
  CAD-style snap-priority engine from scratch.

**Still in flight:**
- Pass 12.M2 (dimensioning + scale/group + hybrid storage + OCG layer)
  is now the active build — the render-touching, R59-gated slice, next
  in decision 011's dependency chain.
- 9c-min (basic vector editing) remains after 12.M2 — last of the
  beta's five slices.
- Priority items #2 (icon set), #3 (finish text-handling: FF-B/FF-H/
  FF-C), and #4 (form-building tools) from the continuation-50 operator
  sequence are all unchanged — still queued behind the dimensioning
  tool, no work started on any of them. **The ScripTree icon design
  (priority #2) is being designed in parallel** (not yet built) and
  will surface an SVG-rendering-approach decision (pre-rasterize to
  PNG at build time vs. adopt an MPL-2.0 SVG-rendering crate at
  runtime) once the icon BUILD itself is scoped — that decision needs
  operator/KenAgent sign-off per rule 13's licensing-decision
  discipline (applied to a runtime dependency choice, not just the art
  assets' own provenance, which was already addressed at continuation
  50).
- Push/publish authorization for the local commits (`d8b3903`,
  `79d1c6f`, `e13f3e6`, `19ed865`, `801a748`) remains a separate,
  not-yet-granted operator item.

**For next session:**
- Continue decision 011's sequence: build **Pass 12.M2** (dimensioning
  UI — already grounded by spec/Acrobat/Inkscape research plus the
  `pdfce-ui-specialist` design) to completion, then **9c-min** (basic
  vector editing) to close out the beta's five slices.
- When the icon-set BUILD (priority #2) is scoped, flag the SVG-
  rendering-approach decision (pre-rasterize vs. MPL-2.0 crate) for
  explicit operator/KenAgent sign-off before implementation — don't
  let it get decided silently as an implementation detail.
- Still owed: operator push/publish call; `ARCHITECTURE.md` §12
  decision-016 backfill (flagged continuation 50, still not done);
  encryption-refusal sign-off; the other oldest-first still-open items
  carried from continuation 50 (see that continuation's list —
  unchanged this continuation, not re-enumerated here to avoid drift
  between two near-duplicate lists).

**Same-day continuation 53 — Pass 12.M2 SHIPPED and COMMITTED
(`c7c1744`); on-canvas authoring gesture split off as "Pass 12.M2b,"
now building; icon DESIGN complete (build not started):**

**Shipped:**
- **Pass 12.M2** — dimensioning + scale/group + hybrid storage + OCG
  layer (decision 011 slice 4 of 5, THE HEADLINE CAPABILITY). NEW
  `crates/pdfce-core/src/dimension/` (mod, fit [Taubin], units, group,
  measure_dict, author, sidecar). CHANGED `edit.rs`
  (add_dimension/add_dimension_group/set_group_scale/
  toggle_dimension_layer/dimension_model + 3 new `CommandKind`s),
  `annot.rs` (`Annotation.oc` + optional_content_default_off/
  oc_is_hidden), `render/annot.rs` (OCG visibility gate in
  survey_page_annotations), `cli/main.rs` (6 subcommands),
  `gui/{canvas, ui_text, main}` (3 Measure `CanvasTool` variants +
  "Measure ▾" menu + status overlay), `lib.rs` (`pub mod dimension`).
  Tests: dimension_roundtrip.rs (6) + 39 unit. Fixtures
  `fixtures/synthetic/dimension/` + generator.
  - **Taubin best-fit circle** (hand-rolled Chernov variant, chosen for
    the short-arc regime pdfce's tool actually hits): a 1200-trial test
    (90° arc, r=100, σ=1.5) proves Taubin bias <1.5% AND less than
    Kåsa's; a real-file fit recovered r=100.00 exactly on the
    12-segment short-arc fixture. **Radius/diameter dimensioning
    EXCEEDS Acrobat** (no equivalent Acrobat baseline exists).
  - **Units/scale:** 6 units incl. architectural feet-inches (144pt @
    12.5ft → 12'-6", spec §12.9 Table 263) — EXCEEDS Acrobat. Tri-state
    `ScaleState` (NeverSet/OneToOne/Calibrated, deliberately never
    collapsed to `Option<f64>`); both entry paths (real-length L/D,
    ratio N:M×basis); scale is authoritative from the `/X` array's
    first `/C`. **Named per-group scale/units EXCEEDS Acrobat's
    per-viewport-only geometric scoping.**
  - **Hybrid storage:** native `/Line`+`/IT /LineDimension`+baked `/AP`
    (universal-viewer render); per-annotation `/Measure` mirror
    (interop convenience, NOT spec-guaranteed to survive cross-tool
    round-trips, since `/PieceInfo` survival is likewise not
    guaranteed); authoritative §14.5 `/PieceInfo /pdfce` sidecar.
    Foreign `/PieceInfo` keys + existing OCGs preserved. All additive —
    existing content bytes byte-verbatim. Per-group §8.11 OCG
    registered in `/OCProperties /D` (default-hidden via `/D /OFF`),
    annotation `/OC` → its group's OCG; render honors annotation-level
    `/OC` (content-stream BDC/EMC-level OCG honoring deliberately
    deferred, out of scope for annotation-only dimensioning).
  - **Public API (rule-10 trail):**
    `dimension::{fit_circle_taubin, fit_circle_taubin_refined,
    FitCircle, Unit, NumberFormat, FractionMode, ScaleState,
    ScaleEntry, ScalePreview, MeasurementDisplay, preview_group_scale,
    format_measurement, Group, GroupId, DimensionId, DimensionKind,
    DimensionRecord, DimensionModel, DEFAULT_GROUP_ID,
    AuthoredDimension, author_dimension, build_measure_dict,
    build_ocg, build_ocproperties, serialize_model,
    deserialize_model}`; `EditSession::{add_dimension,
    add_dimension_group, set_group_scale, toggle_dimension_layer,
    dimension_model}`; `annot::{optional_content_default_off,
    oc_is_hidden}` + `Annotation.oc`. CLI: dimension-add (--kind
    linear/radius/diameter --points --group), dimension-list,
    group-add, group-set-scale, layer-toggle.
  - **Verification (re-confirmed independently in the main tree):**
    core `dimension` module 39 unit + 6 round-trip tests green; full
    workspace `cargo test` **1389** passing; `cargo fmt --all --check`
    clean; `cargo clippy --workspace --all-targets -D warnings` clean;
    `cargo tree -p pdfce-core` / `-p pdfce-render` GUI-dep-free
    (invariant intact); **zero new Cargo dependencies**; R59 agrees
    with pdfium (one documented, CORRECT divergence — see Findings
    below); additive existing-content byte-verbatim; undo
    byte-identical. Live CLI smoke test: `dimension-add` authored a
    linear dimension, round-trip `identical=1, raster_identical=1`.
  - **Committed as `c7c1744`**, on top of `801a748` (Pass 12.M1),
    `19ed865` (docs), `e13f3e6` (Pass 9a), `79d1c6f` (MIT license
    artifacts), `d8b3903` (first implementation commit). All six remain
    **local-only** — push authorization still a separate, not-yet-
    granted operator item.
  - Full `ROADMAP.md` record: new Pass 12.M2 Shipped entry (top of
    Shipped, above Pass 12.M1); GIT STATUS note amended with the
    `c7c1744` commit; Beta "In progress" section amended (state now 4
    of 5 slices shipped, "Pass 12.M2b" on-canvas authoring dispatched
    and IN PROGRESS, 9c-min queued behind it, icon-design-complete note
    added); ★★★ operator priority sequence item 1 state line updated;
    ★ Icon set Next-up entry amended with the design-complete record +
    its two gated decisions, original scoping-notes bullets marked done
    inline.

**Decisions made this session:**
- **Engineer judgment call: GUI scope for 12.M2 capped at menu + tools
  + disclosure; the on-canvas snap-pick authoring gesture (click point
  A, click point B) is DEFERRED to a new follow-up slice, "Pass
  12.M2b — on-canvas dimension authoring," dispatched to build this
  same continuation.** This is the slice the operator's "completely
  functional in the GUI" requirement is actually waiting on — CLI-only
  authoring is a real, disclosed capability (fuzzy-never-sneaky) but
  not the GUI-complete milestone. This effectively splits decision
  011's originally-planned 5th-slice gap into two GUI slices in
  practice (12.M2b then 9c-min); decision 011's own architecture
  document is unchanged, this is an engineer-assigned Pass ID for a
  scope split, not a librarian-invented resequencing.
- Four smaller engineer judgment calls recorded in full in the Pass
  12.M2 Shipped entry, not repeated here: per-annotation `/Measure`
  over page-level `/Viewport` (sidesteps overlapping-scale-group
  geometric partitioning); radius/diameter as one geometry + a
  display-only toggle (3 `CanvasTool` variants, per ui-spec §1.1);
  `/LastModified` fixed placeholder for byte-stable unchanged sidecars
  (trivial follow-up, not a substantive deferral); reused
  `AddDimension` `CommandKind` for group-add.

**Findings + decisions:**
- **R59 note, recorded explicitly so it is never mistaken for a
  regression later:** on `ocg-hidden.pdf`, pdfce correctly HIDES the
  OFF-layer dimension (renders only the base line) while pdfium with
  `draw_annots=True` paints it regardless of OCG state. This is pdfce
  being MORE correct — honoring §8.11.3.3 optional-content visibility —
  not a fidelity defect against the pdfium baseline. Documented in the
  fixture's `PROVENANCE.md`.
- **Dimensioning-tool state, precisely:** 4 of decision 011's 5 slices
  are shipped (12.0, 9a, 12.M1, 12.M2). Dimensions are fully authorable
  today via the CLI and fully disclosed in the GUI (menu, tools,
  status overlay, layer toggle) — but the GUI cannot yet AUTHOR a new
  dimension by clicking on the canvas. **"Pass 12.M2b" (on-canvas
  dimension authoring) is now the active build** — closing this is
  precisely what makes the tool "completely functional in the GUI" per
  the operator's original instruction. **9c-min** (basic vector
  editing) remains queued behind 12.M2b, last of the beta's slices.
- **Icon design (operator priority #2) is now COMPLETE — the BUILD has
  not started.** `pdfce-ui-specialist` authored
  `docs/ui_specs/icon-set-and-toolbar.md` in parallel with the 12.M2
  engineering build: full audit of the current inconsistent icon
  treatment (emoji+text / bare Unicode dingbat / plain text — three
  kinds, no actual images), a reverse-engineered ScripTree style
  contract (48×48 viewBox, `stroke="currentColor"`, outline-only), and
  a mapping across all 27 controls audited, plus a deliberate
  solid-filled exception for redaction's icon (the one glyph in an
  otherwise all-outline set that should NOT be outline-only, since an
  outline would understate what redaction actually does). **Two
  decisions are explicitly named as still operator/KenAgent-gated
  before the BUILD is scoped, per the spec's own §7 and rule 13:**
  (a) SVG-in-egui rendering pipeline — pre-rasterize to PNG at build
  time (zero new dependency, fixed resolution unless multi-DPI baked)
  vs. a runtime `resvg`/`usvg`-style crate (crisp at any DPI, but
  MPL-2.0 — a real new-dependency classification question even under
  MIT); (b) confirming the ScripTree icon set's own provenance/
  licensing before bundling any SVG into pdfce's asset tree (likely a
  non-issue since Ken owns both projects, but must be confirmed per
  rule 13, not assumed).
- No new generalizable Rust/egui finding graduated to `D:\dev\rag\rust\`
  or `D:\dev\rag\egui\` this continuation — the Taubin-vs-Kåsa
  short-arc bias result and the hybrid-storage design are both
  pdfce-architecture-specific (built directly on decision 011's own
  spec-grounded requirements) rather than generalizable Rust/egui-
  ecosystem findings.
- No new PDF-domain finding graduated to `C:\personal_rag\pdf\` this
  continuation — the R59 ocg-hidden divergence is a pdfce-vs-pdfium
  correctness comparison (both against the same §8.11.3.3 spec clause),
  not an empirical "how real-world PDFs diverge from spec" finding;
  it's recorded in the fixture's own `PROVENANCE.md` and in this log,
  which is the right home for it.

**Still in flight:**
- **"Pass 12.M2b" (on-canvas dimension authoring)** is now the active
  build — the deferred click-point-A-then-click-point-B canvas gesture
  consuming 12.M1's snap engine and 12.M2's authoring backend.
- **9c-min** (basic vector editing) remains after 12.M2b — last of the
  beta's slices; decision 011's original five-slice count is now
  effectively six in practice (12.0/9a/12.M1/12.M2/12.M2b/9c-min).
- Icon-set BUILD (priority #2) has not started; its two gated decisions
  (SVG pipeline choice, ScripTree-asset provenance confirmation) are
  named and waiting on operator/KenAgent sign-off whenever the build is
  scoped — not before.
- Priority items #3 (finish text-handling: FF-B/FF-H/FF-C) and #4
  (form-building tools) remain unchanged — still queued behind items
  #1 and #2, no work started on either.
- Push/publish authorization for the local commits (`d8b3903`,
  `79d1c6f`, `e13f3e6`, `19ed865`, `801a748`, `c7c1744`) remains a
  separate, not-yet-granted operator item.

**Still-open operator items (re-surfaced, ordered oldest-first; full
list reprinted this continuation for completeness — two prior
continuations pointed back to continuation 50's list rather than
re-enumerating it, this one restates it in full plus the two new icon
sub-decisions):**
- Encryption-refusal operator sign-off — oldest owed, unchanged.
- Push/publish authorization for the local commit chain — unchanged,
  now six commits deep (`d8b3903` → `79d1c6f` → `e13f3e6` → `19ed865`
  → `801a748` → `c7c1744`), all local-only.
- `LEGAL.md` §2 Adobe-supplement copyright contradiction — flagged,
  unchanged.
- `/R 6` sourcing method — Ken's call (gates Pass 5) — unchanged.
- W15 — no remote/CI — unchanged.
- Autosave / crash-recovery scratch file + true in-place Save (gated on
  it) — still open, unchanged.
- List-authoring scope question — still no operator answer, unchanged;
  explicitly distinct from "finish off all the text handling stuff"
  (continuation 50, item 3).
- `ARCHITECTURE.md` §12 decision-016 (FF-D) backfill — flagged
  continuation 50, still not done.
- **NEW this continuation — icon-set SVG-in-egui rendering pipeline
  choice** (pre-rasterize-PNG, zero new dependency, vs. runtime
  `resvg`/`usvg`, MPL-2.0) — operator/KenAgent sign-off required before
  the icon BUILD is scoped, per rule 13.
- **NEW this continuation — icon-set ScripTree-asset provenance
  confirmation** (likely a non-issue, Ken owns both projects, but must
  be confirmed not assumed before bundling any SVG into pdfce's asset
  tree) — required before the icon BUILD is scoped.
- Justified-alignment question — remains RESOLVED (decision 015,
  continuation 39); listed only for the oldest-first trail's
  continuity, no action needed.

**For next session:**
- Continue decision 011's sequence: build **"Pass 12.M2b"** (on-canvas
  dimension authoring) to completion — this is the slice that actually
  delivers "completely functional in the GUI" — then **9c-min** (basic
  vector editing) to close out the beta.
- When the icon-set BUILD (priority #2) is scoped (after the beta
  closes), get explicit operator/KenAgent sign-off on both named
  decisions (SVG pipeline, asset provenance) before any implementation
  — don't let either get decided silently.
- Still owed: operator push/publish call (now six commits deep);
  `ARCHITECTURE.md` §12 decision-016 backfill; encryption-refusal
  sign-off; the other oldest-first still-open items above.

**Same-day continuation 54 — Pass 12.M2b SHIPPED and COMMITTED
(`7c93cc3`): on-canvas dimension authoring gesture; DIMENSIONING TOOL
NOW COMPLETE END-TO-END IN THE GUI (operator priority #1, substantially
met); icon set's two gated decisions RESOLVED by the operator (priority
#2 unblocked); test-hygiene flake fix dispatched:**

**Shipped:**
- **Pass 12.M2b** — on-canvas dimension authoring gesture (decision 011
  slice 5 of 6 in practice — the deferred GUI slice from Pass 12.M2's
  judgment call 1). NEW `crates/pdfce-gui/src/measure_tool.rs` — pure,
  headless-tested authoring state machines (19 tests). CHANGED
  `main.rs` (`run_measure_tool`/`run_dimension_groups_panel`/
  `scale_entry_widget`, tool entry/teardown, Escape/gesture-interrupt,
  "Manage Dimension Groups…" menu item), `object_provider.rs`
  (`object_sample_points` accessor — one geometry decomposition now
  feeds selection + snap + circle-fit), `ui_text.rs`.
  - Three gestures, all reusing the shipped 12.M2 engine + 12.M1 snap
    indicator + the 14.3/15.2/16.2 preview/Accept idiom: **MeasureLinear**
    (click A snapped → live constrained preview + scaled readout →
    click B → Accept → `EditSession::add_dimension`); **MeasureCircular**
    (toggle pick-set → live `fit_circle_taubin` preview with residual →
    Accept → radius/diameter dimension); **MeasureScale** (reference
    line → scale dialog, real-length or ratio → `set_group_scale`
    re-propagates). Plus a **dimension-groups panel** (create / set
    scale+units+format / layer-toggle / select-active via the shipped
    `EditSession` methods).
  - **Canvas==CLI equivalence PROVEN**
    (`gui_linear_kind_equals_cli_linear_kind`,
    `gui_circular_kind_equals_cli_circular_kind`) — GUI authoring
    produces the identical `DimensionKind` the CLI's `dimension-add`
    builds, so the additive `/Line`+`/IT`+`/PieceInfo`+OCG bytes match
    by construction, not by coincidence between two independent code
    paths.
  - **Gates (re-verified in the main tree):** gui 87/87, core 811/811
    tests passing; `cargo fmt --all --check` clean; `cargo clippy
    --workspace --all-targets -D warnings` clean; `cargo tree -p
    pdfce-core` / `-p pdfce-render` GUI-dep-free (invariant intact);
    **zero new Cargo dependencies**; GUI release build launches (pid
    46476).
  - **Committed as `7c93cc3`**, on top of `6150e1a` (docs), `c7c1744`
    (Pass 12.M2), `801a748` (Pass 12.M1), `19ed865` (docs), `e13f3e6`
    (Pass 9a), `79d1c6f` (MIT license artifacts), `d8b3903` (first
    implementation commit). All eight commits remain **local-only** —
    push authorization still a separate, not-yet-granted operator
    item.
  - Full `ROADMAP.md` record: new Pass 12.M2b Shipped entry (top of
    Shipped, above Pass 12.M2), including the milestone paragraph;
    GIT STATUS note amended with the `7c93cc3` chain; Beta "In
    progress" section amended (5 of decision 011's originally-named 5
    slices now shipped, only 9c-min remaining, promoted to In
    progress); ★★★ operator priority sequence item 1 updated to
    "SUBSTANTIALLY MET"; ★ Icon set entry amended with both gated
    decisions RESOLVED; Backlog test-hygiene flake entry amended
    "FIX IN PROGRESS."

**Decisions made this session:**
- **Engineer judgment calls recorded in full in the Pass 12.M2b Shipped
  entry, not repeated here:** raw second point stored for the linear
  gesture (constrained line is display-only, needed for CLI byte-
  equivalence); group rename/delete NOT implemented (needs core
  sidecar-rewrite support — GUI-only implementation would violate the
  GUI-core separation invariant, named as a follow-up); Ctrl+Shift+D
  chord left unbound (unverified-unclaimed, no conflict check run); a
  pre-existing doc-comment/`#[allow]` misattachment between
  `run_measure_tool` and `run_add_text_tool` (leftover from Pass 16.2)
  fixed opportunistically while wiring the new tool into the same
  region of `main.rs`.
- **Operator decision — icon-set SVG rendering pipeline: PRE-RASTERIZE
  to PNG at build time.** No new Cargo dependency; the runtime
  `resvg`/`usvg`-style crate alternative is explicitly rejected, not
  merely deferred. Resolves gated decision (a) from continuation 53's
  icon-design-complete record.
- **Operator decision — ScripTree icon provenance/style, verbatim:**
  *"Scriptree icons are mine, use from it what makes sense and create
  new ones in its style when necessary, try to make them close to what
  inkscape and Adobe use for similar commands without running into
  copyright issues."* Concretely: use the ScripTree SVGs (Ken's own
  art) where they fit an existing pdfce control; create new icons in
  that same flat/outline style where none fits; for new icons, aim for
  the recognizable visual CONVENTION Inkscape/Adobe use for the
  equivalent command (the metaphor — hand for pan, magnifier for zoom,
  etc.), explicitly WITHOUT copying their actual icon artwork, so there
  is no copyright exposure. Resolves gated decision (b) from
  continuation 53. A `PROVENANCE.md` recording this confirmation is
  still owed in the new asset directory when the icon BUILD itself
  lands (per the ui-spec's own recommendation) — the decision is
  resolved, the paperwork artifact is a build-time task, not an open
  question.

**Findings + decisions:**
- **MILESTONE — the operator's #1 directed priority, "get the
  dimensioning tool completely functional in the gui interface," is now
  SUBSTANTIALLY MET.** Decision 011's dimensioning capability is
  complete end-to-end in the GUI: Pass 12.0 (canvas substrate) → Pass
  9a (object model) → Pass 12.M1 (snapping) → Pass 12.M2 (dimensioning/
  scale/storage/OCG engine) → Pass 12.M2b (on-canvas authoring gesture)
  combine so an operator can draw linear/radius/diameter/scale
  dimensions by clicking (snapped), manage named groups with per-group
  scale/units (including architectural feet-inches), and toggle
  per-group layer visibility — all on the canvas, with several
  capabilities exceeding Acrobat (Taubin-fit radius/diameter dimensions,
  feet-inches units, named per-group scale, a first-class CLI). **The
  ONLY remaining decision-011 beta slice is 9c-min** (basic vector
  editing: move/delete/drag-node) — a distinct capability, editing
  EXISTING vector objects rather than authoring dimensions, now
  promoted to In progress.
- **Icon-set BUILD (operator priority #2) is now fully unblocked** —
  design was already complete (continuation 53); both decisions that
  were gating the build itself are now resolved by direct operator
  answer (see Decisions above). The build remains queued behind
  9c-min per the operator's own four-item priority sequence — resolving
  the gates does not jump the queue.
- **Test-hygiene flake now confirmed by a SECOND independent builder.**
  The intermittent `RecoveredBaseForbidsIncremental` failure under a
  full parallel `cargo test --workspace` run (temp-path collision from
  `process::id()` not being per-thread-unique), first flagged at Pass
  9a's ship and non-reproducing on a clean run, has now also been
  observed during the Pass 12.M2b build. Still not a product bug, still
  not blocking any ship, but two independent sightings raise it above
  "note and move on" — a bounded fix (thread-unique temp paths, e.g. an
  `AtomicUsize`/`AtomicU64` counter alongside the PID) is dispatched
  now rather than left purely opportunistic. `ROADMAP.md`'s Backlog
  entry amended to "FIX IN PROGRESS."
- No new generalizable Rust/egui finding graduated to `D:\dev\rag\rust\`
  or `D:\dev\rag\egui\` this continuation — the measure-tool state
  machines and the canvas==CLI equivalence tests are pdfce-architecture-
  specific (built directly on decision 011's own requirements), not
  generalizable Rust/egui-ecosystem findings. (If the test-hygiene fix,
  once shipped, yields a generalizable "thread-unique temp path in
  `cargo test --workspace`" pattern, that is a `D:\dev\rag\rust\`
  candidate to write when it lands — flagged here so it isn't missed.)
- No new PDF-domain finding graduated to `C:\personal_rag\pdf\` this
  continuation — nothing this Pass surfaced was an empirical "how
  real-world PDFs diverge from spec" observation; it was all pdfce-
  internal GUI-authoring engineering.

**Still in flight:**
- **9c-min** (basic vector editing: move/delete/drag-node) — now the
  active build, last of decision 011's originally-named five slices;
  closes the beta out entirely once shipped.
- Icon-set BUILD (priority #2) unblocked but not yet dispatched —
  queued behind 9c-min, no work started.
- Priority items #3 (finish text-handling: FF-B/FF-H/FF-C) and #4
  (form-building tools) remain unchanged — still queued behind items
  #1 (now substantially met) and #2, no work started on either.
- Test-hygiene fix (thread-unique integration-test temp paths) —
  dispatched, in progress; no Pass number assigned.
- Push/publish authorization for the local commits (`d8b3903`,
  `79d1c6f`, `e13f3e6`, `19ed865`, `801a748`, `c7c1744`, `6150e1a`,
  `7c93cc3`) remains a separate, not-yet-granted operator item.

**Still-open operator items (re-surfaced, ordered oldest-first):**
- Encryption-refusal operator sign-off — oldest owed, unchanged.
- Push/publish authorization for the local commit chain — unchanged,
  now eight commits deep (`d8b3903` → `79d1c6f` → `e13f3e6` →
  `19ed865` → `801a748` → `c7c1744` → `6150e1a` → `7c93cc3`), all
  local-only.
- `LEGAL.md` §2 Adobe-supplement copyright contradiction — flagged,
  unchanged.
- `/R 6` sourcing method — Ken's call (gates Pass 5) — unchanged.
- W15 — no remote/CI — unchanged.
- Autosave / crash-recovery scratch file + true in-place Save (gated on
  it) — still open, unchanged.
- List-authoring scope question — still no operator answer, unchanged;
  explicitly distinct from "finish off all the text handling stuff"
  (continuation 50, item 3).
- `ARCHITECTURE.md` §12 decision-016 (FF-D) backfill — flagged
  continuation 50, still not done.
- Icon-set SVG-in-egui rendering pipeline and ScripTree-asset
  provenance/style — **RESOLVED this continuation** (see Decisions
  above); listed here only for the oldest-first trail's continuity, no
  further action needed on the decisions themselves. A `PROVENANCE.md`
  write-up is still owed when the icon BUILD lands.
- Justified-alignment question — remains RESOLVED (decision 015,
  continuation 39); listed only for the oldest-first trail's
  continuity, no action needed.

**For next session:**
- Build **9c-min** (basic vector editing: move/delete/drag-node) to
  completion — this closes out decision 011's beta entirely.
- When 9c-min ships and the icon-set BUILD (priority #2) is dispatched,
  proceed directly to implementation — both gating decisions are
  already resolved; no further sign-off needed before the build itself,
  only remember to write the `PROVENANCE.md` confirmation record.
- Land the test-hygiene fix (thread-unique temp paths) opportunistically;
  consider writing a `D:\dev\rag\rust\` finding once it ships if the
  pattern generalizes beyond pdfce's own test helpers.
- Still owed: operator push/publish call (now eight commits deep);
  `ARCHITECTURE.md` §12 decision-016 backfill; encryption-refusal
  sign-off; the other oldest-first still-open items above.

**Same-day continuation 55 — Pass 9c-min SHIPPED + decision-011 beta
COMPLETE + subagent budget exhausted (2026-08-01, engineer-authored — the
subagent limit was reached, so the librarian filing is done directly).**
- **Shipped:** Pass 9c-min (basic vector editing: move/delete/drag-node),
  committed `76485b5`. Content-stream surgery on Pass 9a's object model
  (Pass 8.0 REPLACE substrate reuse; linear-CTM-inverse page→user delta;
  agree-by-construction node ordering), CLI object-move/object-delete/
  node-move + `CanvasTool::VectorEdit`. Gates: 33 suites green,
  content-identity (one changed stream), §5.7 objstm promotion, R59
  faithful, undo byte-identical, fuzz 400k/0. Named limits: single edit
  per GUI session (VectorEditNeedsReopen), rect-corner/handle/text-image
  node editing stay full Pass 9.
- **MILESTONE:** decision 011's measurement/editing beta is COMPLETE —
  all six slices (`12.0→9a→12.M1→12.M2→12.M2b→9c-min`) shipped+committed.
  The operator's #1 directed priority ("dimensioning tool completely
  functional in the GUI") is fully met. GUI relaunched for the operator.
- **Also this continuation (engineer-direct, no agents):** `ARCHITECTURE.md`
  §12 backfilled with the decision-016 entry (committed `dd3a8b8` — closes
  the flagged §12 gap; the §12 log now records 013/014/015/016 + MIT); the
  integration-test temp-path flake fixed (committed `2abbd75`).
- **CONSTRAINT — subagent budget (200) EXHAUSTED.** No further work can be
  DELEGATED to builder/librarian/spec agents this session. Remaining
  operator priorities (ScripTree icons; finish text-handling FF-B/FF-H/
  FF-C; form-building) must be built DIRECTLY in the main loop (slower,
  in-context) OR the operator raises `CLAUDE_CODE_MAX_SUBAGENTS_PER_SESSION`.
  Surfaced to the operator for a decision on how to proceed.
- **Commit chain (11):** `d8b3903 → 79d1c6f → e13f3e6 → 19ed865 → 801a748
  → c7c1744 → 6150e1a → 7c93cc3 → 2abbd75 → dd3a8b8 → 76485b5`.
- **Still owed (operator):** how to proceed past the agent limit;
  push/publish call; encryption-refusal sign-off; `LEGAL.md` §2; `/R 6`;
  list-authoring scope.

## 2026-08-02 — GUI usability triage: the headline live-edit-rendering finding, decisions 017/018, Pass 18.0 shipped

Branch `pass-8-redaction`, HEAD `0569373` at session start (continues
directly from 2026-08-01 continuation 55 — a new calendar day, fresh
subagent budget). All work this session is **uncommitted** — no commit
has been made or requested.

**Operator report that started this session, verbatim:** "there's a lot
of work to be done to get the gui functional. The commands along the
top still don't have icons. If I click the one to edit objcts, I don't
seem to be able to click on objects. Sometimes I click and get a box
highlighting on the screen that doesn't seem to correspond to anything.
The Tools dock should be able to have other tools docked in tabs as
well, like any other modern program would. I'd like to have a layer
tree there for the document that I can also click on to select
objects. at least that way we can troublshoot better what I am clicking
on in the GUI area. The dimensioning tool didn't seem to have a way to
actually set the dimensions. Maybe the underlying work is functional,
but I can't tell with the current state of the GUI."

**Shipped:**
- **Pass 18.0** — zoom-invariant selection tolerance + gesture-
  preserving zoom (uncommitted). Direct fix for "I don't seem to be
  able to click on objects." Full record: `ROADMAP.md` Shipped (top).

**Decisions made this session:**
- **★★★★ HEADLINE FINDING — the GUI never renders unsaved edits.** ONE
  shared read-path bug (`OpenDoc::rasterize_current` /
  `ensure_object_provider` in `pdfce-gui/src/main.rs` both read
  `session.document()`, the BASE revision — `edit.rs:962`'s own doc
  comment says so), not fourteen broken features. Every editing feature
  from Pass 3.1 through Pass 16.2 writes to `EditSession`'s in-memory
  overlay and is invisible/unclickable in the running GUI as a result.
  `refresh_pages`'s doc comment ("the document is not reloaded, because
  the base revision ... has not changed") was TRUE through Pass 3.1 and
  FALSE since Pass 6.1 — a stale comment, not a stale cache-invalidation
  mechanism (the cache-clearing itself was already correct). Reframes
  project status: every Pass 3.1–16.2 met its stated gates and shipped
  a feature the operator could not see — a GATE defect ("done" never
  required "observed in the running app"), not an engineering defect.
  Full record: `docs/decisions/018-edited-state-is-what-the-canvas-
  renders.md`; `ROADMAP.md`'s ★★★★ HEADLINE FINDING note (top of "In
  progress") and ★ Pass 17.x entry (Next up).
- **Decision 018 — generalize `pdfce-render` + `decompose_page` over
  `ObjectGraph`/`DocumentView`, extended with a new `StreamSource`
  (contiguous-vs-split byte source).** Measured: `pdfce-render`'s
  entire `Document` surface is 3 methods / 50 call sites, 45 of which
  compile unchanged. Rejected re-serialize+re-parse (routes VIEWING
  through the WRITER — recovered/hybrid documents could display
  NOTHING, since the writer refuses them; a viewer must never be less
  capable than the parser). Rejected GUI-side compositing (can't
  represent content-stream surgery — most of what shipped; recreates
  the Z2 two-decompositions-diverge pattern). Sliced as Pass 17.0
  (core+render generalization + the 2-line GUI fix)/17.1 (finish the
  `session.document()` audit — confirmed a second live instance,
  `main.rs:4606`, redaction marks added this session not counted in the
  GUI)/17.2 (CLI parity + headless preview-equals-saved oracle). Not
  yet built. Full record: `docs/decisions/018-...md`; `ROADMAP.md` ★
  Pass 17.x.
- **Decision 017 — hand-roll a two-compartment vertical panel row list
  in the existing right-hand dock; no docking dependency.** Answers the
  operator's tabbed-dock/layer-tree ask. `egui_dock` REJECTED
  PERMANENTLY (binary splits only; zero accessibility instrumentation
  repo-wide — 0 hits for `widget_info`/`accesskit`/`keyboard`; depends
  on `paste`, which carries RUSTSEC-2024-0436; this also closes
  `PRIOR_ART.md`'s open 0.19.1-vs-0.20.1 egui_dock version gap at
  0.20.1). `egui_tiles` 0.16.0 fully vetted (MIT OR Apache-2.0, 1 new
  package, wasm-clean, exact MSRV/egui match) and PRE-APPROVED behind
  one named trigger (Ken answers escalation Q1 with the VS Code/Blender
  whole-content-area model). **This record reverses its own initial
  recommendation** after a `pdfce-ui-specialist` review surfaced that
  the dock is 320pt wide and `egui_tiles` draws horizontal tab bars
  only — a rejection on FIT, not on dependency hygiene. Sliced as Pass
  18.1 (tabbed/panel shell + Objects tree + Properties selection
  panel + selection feedback) / 18.2 (`object-list` CLI subcommand,
  new gap found this session) / 18.3 (Measure ▾ affordance fix — a
  "Set Scale…" button beside the existing dead-end disclosure label).
  Pass 18.0 (the tolerance/gesture fix) shipped standalone, above. Full
  record: `docs/decisions/017-tabbed-dockable-panel-system.md`;
  `ROADMAP.md` ★ Pass 18.x.
- **Design conflict flagged, not resolved:** `docs/ui_specs/
  pass-17-dock-and-layer-tree.md` §A (authored earlier the same session)
  designs a horizontal 3-tab strip; decision 017 (authored later, after
  UI-specialist review) rejects that shell design and replaces it with
  the two-compartment vertical list. §A.1–A.5 (the tab-strip widget)
  is SUPERSEDED for the shell; §B (object tree)/§C (selection
  feedback)/§D (Measure ▾ fix) are unaffected. Recorded in `ROADMAP.md`'s
  Pass 18.1 entry so a future session builds decision 017's shell, not
  the superseded ui-spec §A design.
- **Pass-number renumbering (recorded, do not lose):** the ui-spec's
  own filename claims "pass-17"; decision 018 also claims "Pass 17" for
  live-edit rendering (drafted independently, same session). Decision
  018 keeps Pass 17 (it was first to a real decision record); the
  ui-spec/decision-017 family is renumbered to **Pass 18.x**. Same
  renumber pattern as decision 014's Pass 13→14 move.
- **Standing-rule numbering collision resolved.** Both decision 017 and
  018 were drafted concurrently against "highest existing rule is R79"
  and both initially proposed starting at R80. Decision 018 self-
  corrected before filing (provisionally R85/R86, explicitly avoiding
  017's R80–R84). Librarian ratifies both ranges as final, non-
  colliding, in `ROADMAP.md` Standing rules: **R80–R84 (decision 017)**
  — dock host, floating-is-transient-only (supersedes `ARCHITECTURE.md`
  §12 continuation-19's "single legacy floating exception" — see the
  §12 entry filed this session), layout rides R15, no-affordance-
  without-capability, selected-state-never-colour-alone. **R85–R86
  (decision 018)** — preview-equals-saved (in force); operator-visible
  definition of done (**PROPOSED, not yet in force — pending operator
  sign-off**, recorded now so the number is reserved).
- **`ARCHITECTURE.md` §12 filed:** two new dated entries (decision 018's
  live-edit-rendering architecture; decision 017's panel system,
  including the correction to continuation-19's now-false "Properties
  is the single legacy floating exception" claim — Pass 12.M2's
  Dimension Groups panel already breached it as a second floating
  window, named as the remaining floating-window holdout for a
  follow-up migration into the new dock).

**Findings + decisions (empirical):**
- **Zoom-inverted selection tolerance is a units bug, not a
  hit-testing correctness bug.** `object_provider.rs`'s
  `SELECT_TOLERANCE = 3.0` was fixed in CANVAS space while the pointer
  reaching it had already been divided by `zoom` — so the on-screen
  catch radius was `3.0 × zoom` px, meaning zooming OUT (exactly what
  an operator does to see a whole page before clicking something on
  it) made clicking HARDER, the inverse of every other viewer. The fix
  vector already existed in the codebase (the Pass 12.M1 snap engine's
  `screen_tolerance_to_page`) and needed reuse, not invention. Filed to
  `D:\dev\rag\egui\` (see below) as a generalizable immediate-mode-canvas
  gotcha.
- **Root cause of "a box that doesn't correspond to anything":**
  `TextObject`'s bounding box is deliberately origin-inflated and
  approximate (`decompose.rs`, `approximate: bool` always `true`) with
  zero UI surfacing of that fact today — combined with the tolerance
  bug, the most likely explanation is the operator hit a text object's
  inflated whitespace margin and the app never said so. Confirmed by
  code-reading, not yet fixed (ui-spec §C.2 names the fix: an
  explicit disclosure sentence, buildable with zero core changes).
- **The dimensioning tool is confirmed functional, not broken.**
  Reading `run_measure_tool`/`scale_entry_widget` in full: the `Measure
  ▾` menu, two-point pick/live-preview, and the full scale-entry
  sub-panel are all already implemented and close to spec. The
  operator's complaint is a discoverability/dead-end-affordance gap
  (no icon; a disclosure label with no next-action button), not a
  missing feature — ui-spec §D / Pass 18.3 names the fix.
- **`object-list` CLI subcommand does not exist**, despite
  `object-move`'s own help text telling operators to get indices from
  it. Filed as Pass 18.2.
- **Icon pipeline blocker:** the recorded pre-rasterize-to-PNG plan
  (SESSION_LOG continuation 54) is not executable on this machine (no
  Inkscape, no ImageMagick, `cairosvg` libcairo load failure). Engineer
  proposed an alternative needing no new dependency (SVG-path-`d`
  parser feeding the already-present `tiny-skia`) — awaiting operator
  answer, not recorded as decided.

**Still in flight:**
- Pass 17.x (live-edit rendering) — decided, not built. Engineer
  recommends it lands before any further item in the ★★★ operator
  priority sequence; **awaiting operator sign-off** on that
  reordering.
- Pass 18.1–18.3 (dock/tree/selection-feedback/Measure-affordance) —
  decided/specced, not built.
- Pass 12.M2c (dimension-tool bug-fix cluster: 6 named bugs, file:line
  cited) — filed, not scoped into a build plan.
- Icon-set BUILD — design complete, both original gated decisions
  resolved, but now blocked on the pipeline-executability question
  above and re-queued behind whatever sequencing answer resolves the
  Pass-17-first proposal.

**For next session:**
- Six new/updated open questions recorded in `ROADMAP.md`'s "Open
  operator questions" section (icon pipeline switch; decision 017 Q1/
  Q2/Q3; decision 018's proposed rule R86; decision 018's proposed
  Pass-17-first reordering) plus the carried prior list (push/publish;
  encryption `/R 6` + `LEGAL.md` §2; list-authoring scope). Read that
  section first.
- If the operator answers the Pass-17-first sequencing question, Pass
  17.0 is the concrete next build (per decision 018 §8's own slice
  plan); if not answered, the existing ★★★ sequence (icons → text-
  handling → forms) stays authoritative and Pass 17.x queues behind it
  — but note the icon build is independently blocked on question (a)
  regardless.
- Two `D:\dev\rag\egui\` findings filed this session (zoom-inverted
  screen/page-space tolerance pattern; immediate-mode render-path
  divergence from a separate edit-overlay/mutation path) — see that
  RAG's index for the general-purpose lesson, distinct from this
  project's own decision-018 record of the same bug.

**Same-day continuation 56 — Pass 17.0 SHIPPED (live-edit rendering,
decision 018), Pass 18.0/18.2 confirmed committed, operator answered
both open sequencing/pipeline questions, four more Passes/fixes
shipped, repo-integrity incident found and fixed.** Branch
`pass-8-redaction`; five new commits landed this continuation on top of
`0569373` (session-start HEAD): `9a68d6f` (Pass 18.0, already recorded
last continuation but INCORRECTLY as uncommitted — corrected below),
`3a56b55` (Pass 17.0), `f2d5fae` (GUI observation harness), `c998521`
(selection-outline feedback fix), `dae0139` (Pass 18.2 `object-list`
CLI), `b73604d` (`.gitattributes` repo-integrity fix). Workspace: 1474
tests passing, 0 failed; `cargo fmt --check` clean; `cargo clippy
--workspace --all-targets -D warnings` clean; `cargo tree -p
pdfce-core`/`-p pdfce-render` GUI-dep-free; zero new Cargo dependencies
across all six.

**Shipped:**
- **Pass 17.0** — live-edit rendering (decision 018), committed
  `3a56b55`. `DocumentView` promoted to `pdfce_core::view`;
  `StreamSource { Contiguous | Split }`; `impl ObjectGraph for
  DocumentView` kept 45/50 `pdfce-render` call sites unchanged; canvas
  now reads `self.session.view()`, not `self.session.document()`.
  Roundtrip corpus 4,023 files unchanged, raster oracle 6566/6566. Full
  record: `ROADMAP.md` Shipped (top).
- **GUI observation harness** — `tools/observe-gui.ps1` +
  `tools/gui-click.ps1`, committed `f2d5fae`. Foreground-window
  verification on both; found the canvas-click limitation recorded
  below.
- **Selection-outline feedback fix** — committed `c998521`. Second,
  independent root cause of "can't click objects": the Obj tool armed
  selection/delete/drag but drew no visual feedback at all.
- **Pass 18.2** — `object-list` CLI subcommand + `--hit`/`--tolerance`
  hit-test query, committed `dae0139`. Closes the gap where
  `object-move`'s own help text pointed at a command that didn't exist.
- **`.gitattributes` ordering fix** — repo-integrity incident, committed
  `b73604d`. See Findings below.

**Corrections to prior filing (this continuation):**
- Pass 18.0 was recorded last continuation as "uncommitted" — it is
  committed, `9a68d6f`, same as everything else this session.

**Decisions made this session:**
- **Operator answered two of the six open questions filed last
  continuation, both "yes":** (a) icon SVG pipeline — use the
  tiny-skia SVG-path-`d` parser (no new Cargo dependency), NOT the
  pre-rasterize-to-PNG plan (confirmed non-executable on this machine);
  (f) sequence Pass 17.x before the rest of the ★★★ operator priority
  sequence (icons, text-handling, forms). Both recorded as RESOLVED in
  `ROADMAP.md`'s Open operator questions section; the ★★★★★ reordering
  entry is now a confirmed reordering, not a proposal. **Consequence:**
  icon build still doesn't start yet — not blocked on either open
  question any longer, just correctly queued behind Pass 17.1/17.2.
  Item (e) (R86 bless-as-standing-rule) and decision 017 Q1/Q2/Q3
  (items b/c/d) remain unanswered.
- **`ARCHITECTURE.md` §12 updated:** a continuation-56 follow-up entry
  to the decision-018 record, documenting Pass 17.0 as shipped and its
  two implementation deviations (`image_codec::decode_image` needed the
  same generalization; `DocumentView::bytes()` is `Option<&[u8]>`, not
  `&[u8]`, because a `Split` view has no single buffer) plus
  confirming decision 018 §10 hazard 2 as a real, fixed bug (three
  canvas commit sites bypassing `refresh_pages`). §4's forward-pointer
  note updated from "planned" to "implemented."

**Findings + decisions (empirical):**
- **The operator's "can't click objects" complaint had THREE
  contributing causes, all now fixed, not one:** the zoom-inverted
  tolerance (Pass 18.0), the missing selection-outline feedback
  (`c998521`), and the base-vs-edited read path (Pass 17.0). Headless
  proof via the new `object-list --hit --tolerance` CLI query on
  `fixtures/synthetic/dimension/linear-base.pdf` (a zero-height-bbox
  degenerate case): hit-testing succeeds at tolerance 0 (the stroke's
  own half-width is the hittable band) and correctly misses at 3pt off
  with tolerance 0.5 — **core hit-testing was never buggy.**
- **`.gitattributes` last-match-wins pattern ordering corrupted 4
  fixtures in the git index itself** (not just on checkout) — CR bytes
  stripped at `git add` time because `* text=auto` sat below `*.pdf
  binary` in the file and, being the last match, won. Two of the four
  damaged files were the ISO 32000-1 §7.5.4 CRLF-xref-entry test
  fixtures, so the exact bytes under test were the bytes destroyed.
  Diagnosed with `git check-attr text -- <path>`; fixed by reordering +
  `git add --renormalize .`; verified with a fresh `git worktree add`
  (the long-lived main tree stayed green throughout because it never
  re-checked-out the damaged blobs — a false negative that would have
  bitten the next fresh clone). Filed to `D:\dev\rag\rust\
  gitattributes_last_match_wins_ordering_corrupts_index.md`.
- **Synthetic OS-level pointer input does not satisfy egui's
  `Response::clicked()` on canvas/custom-`Sense` widgets**, even though
  it partially activates simple controls (toolbar hover/pressed
  styling). Ruled out timing as the cause (25ms–140ms holds,
  with/without pre-motion, all failed). `egui_kittest` is the supported
  programmatic-interaction path — already referenced in decision 017's
  accessibility analysis; this is the concrete evidence for why OS-level
  automation can't substitute for it. Filed to `D:\dev\rag\egui\
  synthetic_os_pointer_input_not_response_clicked.md`; new Backlog entry
  filed in `ROADMAP.md` recommending an `egui_kittest`-based canvas-
  gesture testing harness as the follow-up.

**Still in flight:**
- Pass 17.1 (finish the `session.document()` audit — confirmed live bug
  at `main.rs:4606`, redaction marks added this session undercounted in
  the GUI) and Pass 17.2 (CLI parity + headless preview-equals-saved
  oracle harness covering all twelve R85 operations) — both not yet
  built. Per the operator's confirmed reordering, these gate the icon
  build, text-handling fast-follows, and form-building.
- Pass 18.1 (tabbed/panel dock shell + Objects tree + Properties
  selection panel) and Pass 18.3 (Measure ▾ affordance fix) — decided/
  specced, not built.
- Pass 12.M2c (dimension-tool bug-fix cluster, 6 named bugs) — filed,
  not scoped.
- Icon-set BUILD — both original gates AND both newly-discovered gates
  (pipeline executability, sequencing) are now resolved; queued behind
  Pass 17.1/17.2, ready to start the moment they ship.

**For next session:**
- Concrete next build: **Pass 17.1** (finish the `session.document()`
  audit), then **Pass 17.2** (headless preview-equals-saved oracle) —
  the operator-confirmed sequencing makes this the unambiguous next
  step, no further sequencing question to resolve first.
- Still open: Open operator questions (e) (R86 standing-rule blessing)
  and (b)/(c)/(d) (decision 017 Q1/Q2/Q3) — read `ROADMAP.md`'s Open
  operator questions section.
- Carried, unchanged: push/publish call; encryption `/R 6` +
  `LEGAL.md` §2; list-authoring scope.
- Two new cross-project RAG findings filed this continuation (see
  Findings above) — `D:\dev\rag\rust\` and `D:\dev\rag\egui\` index
  files both updated.

**Same-day continuation 57 — Pass 18.3 SHIPPED (ScripTree-style icon
set + toolbar overflow wrapping + Measure ▾ affordance fix, icon-set
Next-up entry RESOLVED), decision 017 AMENDMENT A filed to
`ARCHITECTURE.md` §12 (`egui_tiles` ADOPTED), three more RAG findings.**
Branch `pass-8-redaction`; two commits landed on top of `b73604d`
(continuation-56 HEAD): `f9bb560` (docs — continuation-56 librarian
filing, decision 017 Amendment A, a status notice atop
`docs/ui_specs/pass-17-dock-and-layer-tree.md`, and a `tools/roundtrip`
determinism fix) → `c59b0c4` (Pass 18.3). Chain now 20 commits, still
all local-only.

**Shipped:**
- **Pass 18.3** — ScripTree-style SVG icon set + toolbar overflow
  wrapping + Measure ▾ affordance fix, committed `c59b0c4`. New
  `crates/pdfce-gui/src/icons.rs` (SVG-path-`d` → tiny-skia → egui
  texture, zero new Cargo dependencies), new `crates/pdfce-gui/assets/
  icons/` (35 SVGs: 8 verbatim from ScripTree, 2 derived, 25 new, plus
  `PROVENANCE.md`), toolbar now WRAPS instead of clipping or hiding
  controls behind an overflow menu. Workspace 1504 tests passing (was
  1474), 0 failed; fmt/clippy clean; `cargo tree` GUI-dep-free
  invariant intact; zero new dependencies. Full record: `ROADMAP.md`
  Shipped (top).
- **`tools/roundtrip` determinism fix** — committed `f9bb560`. The R38
  promotion-object census sampled `ObjId`s straight off a `HashMap`
  iterator without sorting, so the sampled subset (and the census
  output) drifted between separate runs of the SAME binary over the
  SAME input. Fixed by sorting `ObjId`s before truncating. Verified
  with two back-to-back runs over `fixtures/synthetic` (byte-identical)
  — a smoke test only; the fixture corpus holds <256 objects, so the
  cap rarely binds there and a real reproduction needs the external
  corpus.

**Decisions made this session:**
- **Decision 017 AMENDMENT A filed** (`docs/decisions/
  017-tabbed-dockable-panel-system.md`, authored by `pdfce-engineer`
  directly per §6.1's own instruction that a fired trigger is recorded
  as a dated amendment, not a new decision record; landed in commit
  `f9bb560`). Asked decision 017 §10 Q1 (does the panel system own only
  the right-hand dock, or eventually the whole content area), the
  operator answered *"Use egui_tiles… has the flexibal docking that
  works as well as inkscape's,"* firing the §6.1 trigger in its widest
  direction. `egui_tiles` 0.16.0 is now ADOPTED — one new Cargo
  dependency, MIT OR Apache-2.0, §6.2's vetting stands unchanged.
  Superseded: the hand-rolled two-compartment vertical row list (§3/
  §8.2) AND the ui-spec's original horizontal-tab-strip §A design — both
  are now non-binding shell mechanisms. Survives: the underlying
  simultaneity requirement (Layers+Properties visible together), now
  realized as an `egui_tiles` vertical split instead of two fixed
  compartments; `enum DockPanel` + one `panel_body(...)` dispatcher
  survives verbatim as the future pane payload. **This was NOT yet in
  `ARCHITECTURE.md` §12 when this continuation started** — filed this
  continuation as a full dated §12 entry, plus an addendum to the
  continuation-19 forward-pointer that had previously pointed only at
  the now-also-superseded two-compartment design. Open operator
  question (b) (decision 017 Q1) is now RESOLVED in `ROADMAP.md`.
- **Icon-set Next-up entry marked SHIPPED** and the ★ Pass 18.x entry's
  Pass 18.1 bullet rewritten to describe the `egui_tiles` shell instead
  of the two-compartment design — both in `ROADMAP.md`.
- **DEVIATION FLAGGED, not authorized:** Pass 18.3 (including the icon
  build) shipped WITHOUT waiting for Pass 17.1/17.2, contradicting the
  ★★★★★ REORDERING entry's explicit "do not start the icon build...
  until 17.1/17.2 are also shipped." Recorded as a dated deviation note
  in that entry, not silently absorbed — flag to the operator at next
  contact; not retroactively self-authorized.

**Findings + decisions (empirical):**
- **eframe presents a blank/unpresented frame until it receives a real
  OS input event** — a screenshot harness must drive a synthetic input
  (mouse move suffices) before capturing, confirmed identical in a
  known-good baseline build (ruling out an app regression). Cost the
  builder several diagnostic cycles and the engineer one wasted
  black-screenshot capture this session. Filed to `D:\dev\rag\egui\
  eframe_blank_until_first_input_reactive_repaint.md`.
- **SVG arc-command flags must be lexed as single characters, not via
  the general number lexer** — ScripTree's `link.svg` contains
  `a6 6 0 008 8`, where a naive number-grabber reads `008` as `8` and
  silently shifts every later field, drawing a wrong-but-plausible
  glyph with no error. Pinned by a regression test in `icons.rs`. Filed
  to `D:\dev\rag\egui\svg_arc_flag_single_char_lexing_not_number.md`.
- **`HashMap`'s default hasher reseeds per-process**, so iteration
  order is stable within one run but drifts across separate runs of the
  identical binary on identical input — root cause of the
  `tools/roundtrip` census-drift bug above. Filed to `D:\dev\rag\rust\
  hashmap_iteration_order_drifts_between_runs_of_same_binary.md`.
- **Toolbar overflow: wrapping chosen over an overflow menu, on
  principle, not convenience** — an overflow menu needs to know what
  didn't fit BEFORE layout runs (a frame of lag or a rotting hard-coded
  priority list in egui's immediate-mode model) and still hides
  controls behind a click; wrapping is the only option where nothing is
  ever hidden, so R83 ("no affordance without the capability") holds
  structurally. A naive first wrapping attempt inflated the toolbar
  height at 640pt (one-character-per-line label wrap) and pushed later
  control groups off-panel — worse than the clipping it replaced — fixed
  with `wrap_mode = Extend` so the wrap decision is taken at control
  boundaries. Not RAG-filed separately; recorded here and in the
  `ROADMAP.md` Pass 18.3 Shipped entry as project-internal design
  rationale, not a generalizable egui gotcha on its own.
- **UI-spec self-inconsistencies found in `docs/ui_specs/
  icon-set-and-toolbar.md`** (§4.1 icon-size guidance contradicts
  itself; §1.2's heading overstates what its own body found; §2 has no
  entry for the Pass-9c-min "Obj" toggle since it predates that Pass;
  §3 has no icon assigned to the rail's ▲/▼ reorder arrows) — recorded
  in the Pass 18.3 Shipped entry for `pdfce-ui-specialist`'s next pass
  over that document; not fixed by the engineer, since it isn't the
  engineer's document to edit.
- **Open, dispatched, not fixed:** `▾` (U+25BE) has no glyph in egui's
  bundled fonts and renders as tofu on 4 controls (`Markup □`,
  `Text □`, `Measure □`, Copy's `⧉ □`) — confirmed pre-existing (same
  in the stashed baseline build), not a Pass 18.3 regression.
  `pdfce-ui-specialist` dispatched to adjudicate the fix and audit
  `ui_text.rs` for other unrenderable codepoints; will produce
  `docs/ui_specs/menu-affordance-and-glyph-coverage.md`.

**Still in flight:**
- Pass 17.1 (finish the `session.document()` audit) and Pass 17.2 (CLI
  parity + headless preview-equals-saved oracle) — still not built,
  still the nominal gate for text-handling fast-follows and
  form-building (though NOT observed as a gate for the icon build this
  continuation — see the DEVIATION note above).
- Pass 18.1 (tabbed/panel dock shell + Objects tree + Properties
  selection panel) — decided/specced (now via `egui_tiles`, not the
  superseded two-compartment design), not built.
- Pass 12.M2c (dimension-tool bug-fix cluster, 6 named bugs) — filed,
  not scoped.
- `menu-affordance-and-glyph-coverage.md` — dispatched to
  `pdfce-ui-specialist`, not yet written.
- Open operator questions (c) (decision 017 Q2 — multi-monitor undock)
  and (e) (R86 standing-rule blessing) remain unanswered; (d) (panel
  pairing) reworded for the `egui_tiles` context but still defaults to
  "ship the proposed split" if unanswered.

**For next session:**
- Concrete next build: still **Pass 17.1** then **Pass 17.2** per the
  operator-confirmed sequencing (unaffected by this continuation's
  icon-build deviation) — OR, if the operator blesses the deviation
  retroactively / restates a new priority, follow that instead. Ask
  first; do not assume.
- **Flag to the operator explicitly:** the icon build shipped ahead of
  the confirmed Pass-17-first sequencing — surface this at next
  contact rather than letting it pass silently.
- `pdfce-ui-specialist`'s glyph-coverage audit is in flight — check for
  its output (`docs/ui_specs/menu-affordance-and-glyph-coverage.md`)
  before starting unrelated GUI-text work.
- Carried, unchanged: push/publish call; encryption `/R 6` +
  `LEGAL.md` §2; list-authoring scope; open operator questions (c)/(e).
- Three new cross-project RAG findings filed this continuation — two in
  `D:\dev\rag\egui\`, one in `D:\dev\rag\rust\`; both index files
  updated.

**Same-day continuation 58 (real date 2026-08-03) — Pass 17.1 + Pass
17.2 SHIPPED (decision 018 COMPLETE end-to-end, and the oracle they
built found real silent data loss on its first run), Pass 18.1 SHIPPED
(`egui_tiles` dock + object/layer tree, decision 017 Amendment A now
actually BUILT), the menu-affordance/glyph-coverage tofu class CLOSED,
and a THIRD independent root cause of "can't click objects" found and
fixed.** Branch `pass-8-redaction`; eight commits landed on top of
`c59b0c4` (continuation-57 HEAD): `85a6cac` (docs: `pdfce-ui-specialist`'s
menu-affordance-and-glyph-coverage audit) → `437a6f7` (Pass 17.1 + Pass
17.2) → `a1badc1` (fix: real chevron + "opens a menu" accessible name
for menu-affordance buttons) → `d15c360` (tools: `observe-gui.ps1`
refuses a uniform blank/black capture) → `eeadbcb` (docs: glyph fix
verified by observation, second tofu pair found) → `f963895` (Pass
18.1, `egui_tiles` dock + object/layer tree) → `3f6f5ae` (fix: canvas
hit-testing offset by the page-centring margin) → `869d891` (fix:
chevrons for the reorder arrows, closing the glyph class). Chain now 28
commits, still all local-only. Workspace: 1538 tests passing (1504 →
1521 → 1538 across the three shipped units this continuation), 0
failed; `cargo fmt --check` clean; `cargo clippy --workspace
--all-targets -D warnings` clean; `cargo tree -p pdfce-core`/`-p
pdfce-render`/`-p pdfce-cli` GUI-dep-free; exactly one new Cargo
dependency (`egui_tiles`, `default-features = false`), licence-
classified and attributed.

**Shipped:**
- **Pass 17.1 + Pass 17.2** — `session.document()` audit finishes + R85
  preview-equals-saved oracle harness, committed `437a6f7`. Decision 018
  (live-edit rendering) is now COMPLETE end-to-end (Pass 17.0 + 17.1 +
  17.2 all shipped). Full record: `ROADMAP.md` Shipped (top).
- **Pass 18.1** — `egui_tiles` dock shell + object/layer tree panel,
  committed `f963895`. Decision 017 Amendment A is now actually BUILT,
  not merely adopted-in-principle. All four numbered Pass 18.x
  engineering slices (18.0/18.1/18.2/18.3) are now shipped. Full record:
  `ROADMAP.md` Shipped (top).
- **Canvas hit-testing coordinate-mapping fix** — committed `3f6f5ae`.
  Third, independent root cause of "can't click objects" (the first two
  were Pass 18.0's zoom-inverted tolerance and the missing selection-
  outline draw, `c998521`).
- **Menu-affordance & glyph-coverage: tofu class CLOSED** — committed
  `85a6cac`/`a1badc1`/`eeadbcb`/`869d891`. `glyph_button` deleted — pdfce
  has no text-glyph buttons left anywhere.
- **GUI observation-harness hardening** — committed `d15c360`. Refuses a
  uniform (all-one-colour) capture; third guard of its kind added to
  `tools/observe-gui.ps1`.

**Decisions made this session:**
- **`ARCHITECTURE.md` §11.1 gets a new architectural rule, §12 gets two
  new dated entries.** §11.1: at most one `ObjectWrite` per object id
  per command — a second whole-dict write to an id already written
  earlier in the same command REPLACES rather than merges (root cause of
  the `flatten_fields` bug below). §12: one entry closing out decision
  018 (Pass 17.1/17.2, the three bugs the oracle found), one entry
  confirming decision 017 Amendment A's actual build (Pass 18.1) plus a
  correction to the continuation-57 vetting (see Findings below).
- **★★★★★ REORDERING entry (`ROADMAP.md` Next up) marked GATE CLEARED.**
  Pass 17.1/17.2 shipping means the "do not start icon build/text-
  handling/forms until Pass 17 finishes" condition is now genuinely
  satisfied, not merely deviated around (the icon build had already
  shipped ahead of it at continuation 57 — that deviation stays recorded
  as history, not retracted, but nothing about it blocks anything going
  forward). Items #3 (text-handling fast-follows) and #4 (form-building)
  from the ★★★ operator priority sequence are now genuinely unblocked.
- **★ Pass 17.x entry retired** (all 3 slices shipped, decision 018
  complete) — condensed to a short pointer-and-summary entry rather than
  the full original build plan, per the same retirement convention
  already used for Pass 16.x. **★ Pass 18.x entry updated in place, NOT
  retired** — Pass 18.1 marked SHIPPED, but the ui-spec's §B.4 (core
  data-model additions) and §C (full selection-legibility asks) were
  flagged as a DEVIATION from that bullet's own "binding asks" framing:
  NOT delivered as part of Pass 18.1, now a consolidated Backlog entry
  ("ui-spec §B.4/§C follow-ons").
- **R85 (`ROADMAP.md` Standing rules) amended, not rewritten:** a dated
  note records that `redact-apply` is STRUCTURALLY uncoverable by the
  oracle (applying redaction consumes a `Document` and emits a file
  directly, it is not an `EditSession` operation — no live-session
  left-hand side exists to compare against). The original rule text
  (which listed `redact-apply` in its coverage) is left as-is per
  append-only discipline; the amendment records the gap.

**Findings + decisions (empirical):**
- **Headline finding: the R85 oracle found real, silent data loss on
  its FIRST run.** `flatten_fields` issued three whole-dictionary
  `ObjectWrite`s to the SAME page object in one command (`/Contents`,
  `/Resources /XObject`, `/Annots`), each cloned from the pre-command
  state. Nothing commits mid-command, so they overwrote rather than
  composed and `/Annots` won: flatten deleted the fields, created the
  burn stream and appearance XObjects, and left the page referencing
  NEITHER. **Every flattened form silently lost its visible values**,
  while `fields_flattened`/`widgets_burned`/`pages_touched` all reported
  correctly. Every existing flatten test passed throughout because none
  rendered the result. Reproduced independently against the pre-fix
  binary: value ABSENT pre-fix, PRESENT post-fix. Two more silent-wrong-
  answer bugs from the same audit: `author_text_matches` extracted from
  BASE page indices then fed `add_redaction`, which resolves through
  SESSION `page_slots` — after any delete/reorder a search redaction
  would silently mark the wrong page with plausible geometry; and
  `extract_selection` paired a session graph with base bytes, so a
  stream authored this session copied out empty. `redact-apply` is
  structurally uncoverable by R85 — applying is not an `EditSession`
  operation. Consistently, the GUI has no apply flow at all
  (mark-and-disclose only; apply is CLI-only) — filed as a real Backlog
  gap.
- **A THIRD root cause of "I don't seem to be able to click on
  objects."** `canvas()` allocated the page image inside
  `ui.centered_and_justified(...)` and used `image_response.rect` as the
  page↔screen mapping origin — that helper returns the JUSTIFIED
  CONTAINER rect while drawing the image CENTRED inside it, so every
  hit-test was wrong by the centring margin whenever the page was
  smaller than the viewport. Worst at the zoom used to see a whole page
  — the most common working zoom. **Correction to the record, stated
  explicitly:** after the selection-outline fix (`c998521`), the
  engineer attributed still-failing synthetic canvas clicks to the
  harness not satisfying `Response::clicked()` — that reasoning, while
  the underlying RAG finding remains true in isolation, was NOT the
  actual explanation for the failures being diagnosed at the time; the
  harness was fine, the app was mapping coordinates incorrectly. Net:
  the operator's single sentence had THREE independent, now all-fixed,
  causes, not one and not two.
- **Pass 18.1 build:** `egui_tiles` 0.16.0 added to `pdfce-gui` only
  with `default-features = false` — the crate's DEFAULT features include
  `serde`, which the continuation-57 vetting did not separately flag,
  and leaving defaults on would have silently contradicted that entry's
  own "don't enable serde yet" instruction. AccessKit tab-naming gap
  confirmed on the UNFIXED side (zero `widget_info`/`accesskit` hits in
  the pinned release); egui 0.35's `WidgetType` has no `Tab`/`TabList`
  member at all, a gap that cannot be closed downstream short of an
  upstream egui change.
- **Glyph coverage:** root cause of the `▾`/`▲`/`▼` tofu was pdfce
  setting no custom fonts — egui's default Proportional chain covers
  none of them. Milestone: `glyph_button` removed (no callers) — pdfce
  now has NO text-glyph buttons anywhere. `✓`/`✕` on three tools'
  Accept/Reject buttons remain unverified (needs an in-progress tool
  gesture the harness hasn't yet driven).
- **Harness hardening:** `observe-gui.ps1` now refuses a uniform (all-
  one-colour) capture, after three distinct causes (sleeping monitor,
  eframe's blank-until-first-input idle state, a mid-flight window
  animation) each independently produced one this session.

**Still in flight:**
- No GUI redaction-apply flow (see Findings above) — filed to Backlog,
  not yet scoped into a Pass.
- `✓`/`✕` glyph verification — filed to Backlog.
- ui-spec §B.4/§C follow-ons (`TextObject`/`ImageObject` core additions;
  full selection-legibility asks; the newly-found zero-height-path
  case) — consolidated into one new Backlog entry.
- Pass 12.M2c (dimension-tool bug-fix cluster, 6 named bugs) — still
  filed, not scoped.
- Open operator questions (c) (decision 017 Q2, multi-monitor undock)
  and (e) (R86 standing-rule blessing) remain unanswered.

**For next session:**
- Pass 17.1/17.2/18.1 are all now shipped — decision 018 is complete and
  decision 017's numbered engineering slices are complete; the ★★★
  operator priority sequence's items #3 (text-handling: FF-B, FF-H,
  FF-C) and #4 (form-building) are now genuinely unblocked and are the
  next concrete dispatch, per the operator's continuation-50 sequence,
  unless the operator gives a new steer (e.g. the redaction-apply GUI
  gap, or the ui-spec §B.4/§C follow-ons, both newly surfaced this
  continuation).
- Flag to the operator at next contact: (1) the icon build shipped ahead
  of the Pass-17-first sequencing at continuation 57 (carried,
  unresolved flag); (2) the GUI has no redaction-apply flow at all,
  newly surfaced; (3) R86 (item (e)) remains unanswered and is now
  arguably strengthened as a candidate by this session's own findings
  (R85 and R86 are complementary, not substitutes).
- Carried, unchanged: push/publish call; encryption `/R 6` +
  `LEGAL.md` §2; list-authoring scope.
- Three new cross-project RAG findings filed this continuation — two in
  `D:\dev\rag\egui\` (`centered_and_justified_returns_container_rect_
  not_child_rect.md`, `egui_035_no_tab_tablist_widgettype.md`), one in
  `D:\dev\rag\rust\` (`crate_default_features_can_silently_contradict_
  project_policy.md`); both index files updated.

**Same-day continuation 59 (real date 2026-08-03) — Pass 18.4 SHIPPED
(selection legibility, ui-spec §C, closing most of the deviation flagged
at Pass 18.1's ship), and the `ui-strings` CI gate — found red at
baseline on 140 hits, hiding a real R1 violation — FIXED and moved to a
local script.** Branch `pass-8-redaction`; two commits landed on top of
`869d891` (continuation-58 HEAD): `be62e48` (Pass 18.4, selection
legibility) → `a5d1d18` (fix: `ui-strings` gate + relocation to
`tools/check-ui-strings.sh`). Chain now 30 commits, still all
local-only, still no git remote configured at all. Workspace: `cargo
test --workspace` 1538 → 1559 passed, 0 failed; `cargo fmt --check` /
`cargo clippy --workspace --all-targets -D warnings` clean; `cargo tree
-p pdfce-core`/`-p pdfce-render` GUI-dep-free; zero new Cargo
dependencies this continuation.

**Shipped:**
- **Pass 18.4** — selection legibility (ui-spec §C), committed
  `be62e48`. New `object_summary.rs` fact-record module
  (`describe_object(&VectorObject) -> ObjectSummary`, prose-free, no
  strings at all) shared verbatim by the Objects tree row and the canvas
  status readout — a test pins the two never disagreeing. Per-kind
  selection-outline treatment (solid/dashed, letter badge `P`/`T`/`I`/`F`,
  degenerate-rect inflation via `visible_outline_rect` +
  `MIN_OUTLINE_EXTENT_PX` in screen space). New one-line-plus-
  `CollapsingHeader` status readout. Full record: `ROADMAP.md` Shipped
  (top).
- **`ui-strings` CI gate fix** — committed `a5d1d18`. Was red at
  baseline on 140 hits (not enforcing decision 002 R1 at all); fixed a
  real violation it was hiding (three Measure sub-tool names as bare
  literals) and two false-positive classes (test-assertion prose,
  `Display`-impl diagnostic text); relocated to
  `tools/check-ui-strings.sh`, a character-level scanner replacing the
  old whole-line regex (which mis-parsed adjacent string literals as one
  spanning literal). Full record: `ROADMAP.md` Shipped (top).

**Decisions made this session:**
- **Pass-ID collision resolved by filing, not by editing history.** The
  `be62e48` commit's own message says "Pass 18.2: selection legibility,"
  but Pass 18.2 was already taken (the `object-list` CLI subcommand,
  `dae0139`, 2026-08-02). Per the hard rule that Pass IDs are stable and
  never reused, this session's feature is filed in `ROADMAP.md` as
  **Pass 18.4** (next free slot in the 18.x family) — the commit message
  itself is left as committed (git history isn't rewritten for this);
  the roadmap entry is the canonical Pass-ID record going forward.
- **`ROADMAP.md` Standing rules amended (not rewritten):** a dated note
  on the Rust Style Guide / API Guidelines rule records that `ui-strings`
  enforcement now runs locally via `tools/check-ui-strings.sh`, not CI
  alone. **`ARCHITECTURE.md` §12's decision-002 entry gets the matching
  dated addendum** so the body text and the decision log stay in sync,
  per the librarian's own "both need to change together" discipline.
- **Backlog "ui-spec §B.4/§C follow-ons" entry split by outcome, not
  deleted.** §C's full selection-legibility asks are now marked SHIPPED
  (Pass 18.4); §B.4's `pdfce-core` additions (`TextObject` string/font
  preview, `ImageObject` pixel dimensions) remain owed and stay open,
  now with three newly-surfaced sub-items (ui-spec text-bbox correction,
  the status-bar/fit-zoom hazard, the `hit_test_point_all` core API).

**Findings + decisions (empirical):**
- **A lint/gate that is red at baseline enforces NOTHING and can hide a
  real violation inside its own noise.** 140 baseline hits broke down as
  125 test-assertion messages, 14 `Display`-impl diagnostic text, 3 a
  detector-regex bug, and 1 genuine violation (three Measure sub-tool
  names). Two of the three genuine-violation literals (`"Linear"`,
  `"Radius/Diameter"`) would not even have been caught by a
  baseline-clean version of the SAME regex, since it only flags literals
  containing whitespace — moved anyway because the rule is about
  operator-visible text living in one place, not about detector
  coverage. **Corollary, stated as a standing methodology lesson:
  verify a gate by making it fail on purpose, not only by making it
  pass** — the first planted-failure test was silently swallowed because
  it landed after `#[cfg(test)]`, where the checker truncates; a green
  result on that first attempt would have meant "the check is broken,"
  not "the code is clean." Escalated to `D:\dev\rag\rust\`.
- **The regex `"[^"]*[[:space:]][^"]*"` does not match Rust string
  literals — it spans from one literal's CLOSING quote to the NEXT
  literal's OPENING quote.** `"svg" | "?xml"` parsed as one literal
  containing `" | "`. Character-scanning with quote-open/close state is
  the correct approach for detecting whitespace-bearing Rust string
  literals; a regex written against a whole line cannot distinguish
  literal boundaries from literal contents. Escalated to
  `D:\dev\rag\rust\`.
- **A dynamic bottom panel's height change can retrigger a `Fit page`
  zoom recompute, invalidating click coordinates between frames.**
  First observed as a real UI bug during Pass 18.4 (a growing status
  readout shrank the canvas, which re-fit the page smaller, three times
  in a row as lines accumulated) — generalizes to any egui app combining
  a dynamic bottom/side panel with a fit-to-viewport zoom mode, not
  specific to pdfce's status bar. Escalated to `D:\dev\rag\egui\`.
- **The ui-spec's own model of pdfce's text-bbox approximation is
  backwards.** `docs/ui_specs/pass-17-dock-and-layer-tree.md` §0.2/§B.3
  describe it as "wider and taller than the ink"; empirically (confirmed
  against `fixtures/synthetic/vector/mixed.pdf`) it is inflated from
  glyph ORIGINS by the largest `Tf` size, giving a box narrower than and
  offset from the actual glyph ink — clicking directly on visible text
  can miss the hit region. This is project-internal UI-spec accuracy
  (not a generalizable Rust/egui/PDF-domain fact), so it is NOT escalated
  to any RAG — filed to `ROADMAP.md` Backlog for a `pdfce-ui-specialist`
  re-dispatch to correct the spec text. It is a FOURTH named contributing
  cause of the operator's original "can't click objects" complaint.
- **`icons::Icon` cannot supply object-kind badges** — no glyph exists
  for path/image/form-XObject, and `Icon::Text` already denotes the text
  TOOL, not "this object is text." Reusing it for a badge would assert
  an affordance that doesn't exist (R83). Letter badges are the honest
  interim, not a placeholder to feel bad about — project-internal, not
  escalated.

**Still in flight:**
- §B.4's core `pdfce-core` additions (`TextObject` string/font-name/size
  preview, `ImageObject` pixel width/height) — filed, not built.
- `hit_test_point_all` core API for Alt+click cycling through
  overlapping objects — filed, not built.
- ui-spec §0.2/§B.3 text-bbox-model correction — needs a
  `pdfce-ui-specialist` re-dispatch, not yet done.
- No GUI redaction-apply flow (carried, unchanged from continuation 58).
- `✓`/`✕` glyph verification (carried, unchanged from continuation 58).
- Pass 12.M2c (dimension-tool bug-fix cluster) — still filed, not
  scoped.
- Open operator questions (c) (multi-monitor undock) and (e) (R86
  standing-rule blessing) remain unanswered.
- The ★★★ operator priority sequence's items #3 (text-handling: FF-B,
  FF-H, FF-C) and #4 (form-building) remain the next concrete dispatch —
  unchanged from continuation 58, nothing this continuation blocks or
  advances them.

**For next session:**
- Items #3 (text-handling fast-follows) and #4 (form-building) from the
  operator's continuation-50 priority sequence are still the recommended
  next dispatch, unless the operator gives a new steer (the ui-spec
  text-bbox correction and the §B.4 core additions are both small,
  high-value fast-follow candidates surfaced this continuation).
- Flag to the operator at next contact, all carried or newly precise:
  (1) the icon build shipped ahead of the Pass-17-first sequencing at
  continuation 57 (carried, unresolved flag); (2) the GUI has no
  redaction-apply flow at all (carried); (3) R86 remains unanswered
  (carried); (4) **there is no git remote configured at all** — the
  30-commit chain exists solely on this machine, a verified backup
  bundle (`D:\Dev\pdfce-backups\pdfce-20260803.bundle`) exists as a
  stopgap only; (5) the branch is still named `pass-8-redaction` though
  it now spans Passes 9–18.4 — worth a rename whenever a push is
  authorized.
- Carried, unchanged: push/publish call; encryption `/R 6` +
  `LEGAL.md` §2; list-authoring scope.
- Two new cross-project RAG findings filed this continuation — one in
  `D:\dev\rag\rust\` covering BOTH the red-at-baseline-gate lesson and
  the regex-literal-span bug as two separate files
  (`ci_gate_red_at_baseline_enforces_nothing.md`,
  `regex_whitespace_literal_detector_spans_across_adjacent_rust_string_literals.md`),
  one in `D:\dev\rag\egui\`
  (`bottom_panel_height_change_retriggers_fit_to_viewport_zoom.md`); all
  three index files updated.

**Same-day continuation 60 (real date 2026-08-03) — Pass 18.5 SHIPPED
(`hit_test_point_all` + Alt+click click-through cycling + text/image
object detail — the last two items owed from the whole Pass 18.x /
decision-017-Amendment-A family), a correction to the Pass 18.4
disclosure text, and a correction to the librarian's OWN previous
commit-chain filing.** Branch `pass-8-redaction`; four commits landed
on top of `a5d1d18` (continuation-59 HEAD): `25b4783` (docs: correct
the commit chain itself) → `d296666` (fix: the Pass 18.4
`ApproximateTextBounds` disclosure was itself wrong) → `9998a6b` (Pass
18.5) → `6a6a48f` (tools: blank-capture guard now samples the CLIENT
rect). Chain now **35 commits** from the first implementation commit,
**36 on the branch total**, still all local-only, still no git remote
configured at all. Workspace: `cargo test --workspace` 1559 → 1599
passed, 0 failed; doc-tests 69 passed; `cargo fmt --check` / `cargo
clippy --workspace --all-targets -D warnings` clean; `cargo tree -p
pdfce-core`/`-p pdfce-render` GUI-dep-free; `bash
tools/check-ui-strings.sh` exit 0; `fuzz/` `cargo check --all-targets`
clean; zero new Cargo dependencies this continuation.

**Shipped:**
- **Pass 18.5** — `hit_test_point_all` + Alt+click click-through
  cycling + text/image object detail, committed `9998a6b`. Closes BOTH
  items deferred at Pass 18.4's ship: the core `hit_test_point_all` API
  (with `hit_test_point` now structurally defined as its head, so the
  two provably cannot disagree) and §B.4's core `pdfce-core` additions
  (`TextObject` string/font preview via a new `FontResolver` seam,
  `ImageObject` pixel dimensions). **All six numbered Pass 18.x slices
  (18.0–18.5) are now shipped.** Full record: `ROADMAP.md` Shipped
  (top).
- **Pass 18.4 disclosure correction** — committed `d296666`. The
  `ApproximateTextBounds` copy shipped at Pass 18.4 repeated the same
  wrong text-bbox model the ui-spec itself carries, and reassured the
  operator a surprising selection was "correct" while disclosing
  nothing about the opposite, worse failure (a click on visible text
  can MISS the object). Rewritten to state the real box construction
  and both failure directions. Disclosure only — the underlying
  geometry fix is in progress (ui-spec §E, builder implementing).
- **GUI observation-harness fix** — committed `6a6a48f`. The blank-
  capture guard (`d15c360`, continuation 58) sampled the whole WINDOW
  rect, which always varies (OS-painted title bar), so it could PASS on
  a white, unpresented client area — a strictly worse failure than the
  black-window case it was built to catch. Fixed to sample the CLIENT
  rect only. `gui-click.ps1` also gained `-Modifiers Shift|Ctrl|Alt`
  support (needed to verify Alt+click in the running app for Pass
  18.5, per R86).
- **Librarian filing correction** — committed `25b4783`. The commit
  chain the librarian filed at continuation 59 was missing `7274fdd`
  (itself the commit that repaired an earlier fabricated hash) and had
  conflated "commits in the chain" with "commits on the branch." Both
  fixed and re-verified against `git rev-list`.

**Decisions made this session:**
- **New `pdfce-core` API-design invariant, filed to `ROADMAP.md`'s
  Standing rules by cross-reference and to `D:\dev\rag\rust\`
  generally: a singular "best match" query is always defined as the
  structural head of its plural "all matches" sibling** — one private
  iterator, `.next()` for the singular form, `.collect()` for the
  plural, so the two provably cannot disagree. Applied at both the
  `pdfce-core` function level (`hit_test_point`/`hit_test_point_all`)
  and the `pdfce-gui` trait level (`CanvasTargetProvider::hit_test`/
  `hit_test_all`, singular now a PROVIDED default over the REQUIRED
  plural method).
- **Memory/work-bound judgment call: text previews are capped at
  DECOMPOSITION time (`MAX_TEXT_PREVIEW_CHARS = 64`), not display
  time, and stored as owned truncated `String`s, not borrowed spans.**
  Reasoned explicitly in the Pass 18.5 Shipped entry — a span-based
  design would keep the source `ContentStream` alive for the object's
  whole lifetime and re-run the font-decode ladder on every
  Objects-tree row redraw (which happens every frame); an owned,
  pre-truncated string pays the decode cost once. A separate, smaller
  GUI-layer display cap (`ROW_TEXT_CHARS = 32`) is kept independent of
  the core cap so tuning one can never silently retypeset the other.
- **New `ROADMAP.md` Standing rule R87** — hashes and commit/test
  counts handed to a doc-writing agent (the librarian included) are
  filed as fact with no independent verification; this is the SECOND
  filing error this project's own "verify against `git`" habit has
  caught. R87 makes explicit that these figures must be engineer-
  produced directly from `git`/`cargo test` and spot-checked after
  filing, never recalled from memory or a prior summary.
- **Pass-count amendment:** the ★ Pass 18.x family header now reads
  "ALL 6 numbered slices 18.0–18.5 SHIPPED" (was 5, ending at 18.4) —
  the only item left open from the whole decision-017/Amendment-A/
  ui-spec family is the ui-spec's own text-bbox-model wording
  correction, now IN PROGRESS rather than merely filed.

**Findings + decisions (empirical):**
- **`TextPreview` is sourced-text-only — no derived word-space or
  line-break.** `simple-winansi.pdf` previews as
  `"HelloworldSecond line"` because its word gap is a `TJ` kerning
  offset (no literal space glyph) and its line break a bare `Td` move
  (§14.8.2.5 layout modes S3/S5) — `text_extract`'s `plain_text` mode
  derives both; a preview intentionally does not, judged more honest
  than presenting a reader's guess as the document's own content.
  Project-internal (a documented `pdfce-core` behavior with spec
  citations), not escalated to any RAG.
- **`TextFont::size` is the literal `Tf` operand, not the rendered
  glyph size** — `/F1 1 Tf` + a `12 0 0 12 .. Tm` scaling matrix
  renders at 12pt but reports `size: 1.0`. Documented on the field and
  in CLI help; project-internal, not escalated (this is a `pdfce-core`
  API-shape decision, not a generalizable Rust/egui/PDF-domain fact).
- **A blank-capture guard sampling the WINDOW rect can pass on a
  white, unpresented client area, because the OS-painted title bar
  alone supplies enough pixel variance to satisfy a uniformity check.**
  Escalated to
  `D:\dev\rag\egui\blank_capture_guard_must_sample_client_rect_not_window_rect.md`.
- **Define a singular "best match" query as the structural head of its
  plural "all matches" sibling** (one private iterator, `.next()` vs
  `.collect()`) so the two provably cannot disagree — generalizes
  beyond hit-testing to any resolve-with-fallbacks/ranked-match API.
  Escalated to
  `D:\dev\rag\rust\define_singular_query_as_head_of_plural_query.md`.
- **Doc-writing agents have no shell — any hash/count handed to them
  is filed as fact.** Confirmed a second time this project (see
  Decisions, above); recorded as `ROADMAP.md` R87, not escalated
  further (project-internal methodology, not a Rust/egui/PDF-domain
  fact).

**Still in flight:**
- ui-spec §E text-bbox hit-target geometry fix — IN PROGRESS, builder
  implementing; not yet shipped.
- `-Pid` parameter on `tools/gui-click.ps1`/`observe-gui.ps1` — filed to
  Backlog, not built.
- No GUI redaction-apply flow (carried, unchanged).
- `✓`/`✕` glyph verification (carried, unchanged).
- Pass 12.M2c (dimension-tool bug-fix cluster) — still filed, not
  scoped.
- Open operator questions (c) (multi-monitor undock) and (e) (R86
  standing-rule blessing) remain unanswered.
- The operator priority sequence's items #3 (text-handling: FF-B,
  FF-H, FF-C) and #4 (form-building) remain the next concrete dispatch
  — unchanged from continuation 59, nothing this continuation blocks or
  advances them.

**For next session:**
- Items #3 (text-handling fast-follows) and #4 (form-building) remain
  the recommended next dispatch unless the operator gives a new steer.
  The ui-spec §E geometry fix is already in flight independently and
  doesn't block either.
- Flag to the operator at next contact, all carried or newly precise:
  (1) the icon build shipped ahead of the Pass-17-first sequencing at
  continuation 57 (carried, unresolved flag); (2) the GUI has no
  redaction-apply flow at all (carried); (3) R86 remains unanswered
  (carried); (4) **there is no git remote configured at all** — the
  36-commit branch exists solely on this machine; the verified backup
  bundle (`D:\Dev\pdfce-backups\pdfce-20260803.bundle`) has NOT been
  regenerated against this continuation's four new commits — treat it
  as stale until refreshed; (5) the branch is still named
  `pass-8-redaction` though it now spans Passes 9–18.5 — worth a rename
  whenever a push is authorized.
- Carried, unchanged: push/publish call; encryption `/R 6` +
  `LEGAL.md` §2; list-authoring scope.
- Two new cross-project RAG findings filed this continuation — one in
  `D:\dev\rag\egui\`
  (`blank_capture_guard_must_sample_client_rect_not_window_rect.md`),
  one in `D:\dev\rag\rust\`
  (`define_singular_query_as_head_of_plural_query.md`); both index
  files updated. No `personal_rag/pdf` finding this continuation — all
  four commits were pdfce-internal engineering/tooling/API-design, not
  PDF-domain-empirical, and nothing here is "what the spec says" either
  (no dispatch to `pdfce-spec-librarian` needed).

**Same-day continuation 61 (real date 2026-08-03) — Pass 18.6 SHIPPED
(text hit-target geometry now derived from font metrics, not
glyph-origin inflation — ui-spec §E), closing the FOURTH and last named
contributing cause of the operator's 2026-08-02 "I don't seem to be
able to click on objects" report.** One commit, `1b38e34`, on top of
`6a6a48f` (continuation-60 HEAD). Branch `pass-8-redaction`, now **37
commits**, still no git remote configured. Backup bundle refreshed at
`D:\Dev\pdfce-backups\pdfce-20260803-0830.bundle` (verified complete —
supersedes the stale `pdfce-20260803.bundle` flagged at continuation
60). Workspace: `cargo test --workspace` 1599 → 1613 passed, 0 failed;
`cargo fmt --check` / `cargo clippy --workspace --all-targets -D
warnings` clean; `bash tools/check-ui-strings.sh` exit 0; `cargo tree -p
pdfce-core`/`-p pdfce-render` GUI-dep-free; zero new Cargo dependencies.

**Shipped:**
- **Pass 18.6** — text hit-target geometry now derived from font
  metrics, committed `1b38e34`. `TextObject`'s bbox was the pen-START
  point of each show operator inflated symmetrically by the largest
  `Tf` size (a square centred on the run's start, for the common
  single-`Tj` case); it is now the summed advance widths across the run
  for the horizontal extent and `/FontDescriptor` ascent/descent for
  the vertical, via a four-rung fallback ladder (`/Ascent`+`/Descent` →
  `/FontBBox` → compiled-in standard-14 → nominal em, flagged). Measured
  on `mixed.pdf`: `bbox=16,136,44,164` → `bbox=30,147.102,70.46,160.052`.
  Engineer-verified via the CLI hit-test oracle AND on screen: a click on
  visible glyphs that used to MISS now hits; a click on blank paper to
  the left that used to FALSE-HIT now misses. Full record: `ROADMAP.md`
  Shipped (top).

**Decisions made this session:**
- **Four bbox bases shipped, not the two ui-spec §E asked for**
  (`TextBoundsBasis::{FontMetrics, MetricAdvancesNominalHeight,
  EstimatedAdvances, EmBox}`) — judged necessary, not scope creep: a
  Type 3 or descriptor-less CIDFont has real advances but a guessed
  height; a non-standard-14 font with no `/Widths` has estimated
  advances. Collapsing either into `FontMetrics` would reproduce
  exactly the "sentence that no longer matches the box" failure §E
  exists to prevent, and the third case is not hypothetical — the
  project's own `text/identity-h-no-tounicode.pdf` fixture exercises it.
- **New single-source-of-truth invariant for the text-advance formula:**
  `advance_tx(w0, tfs, tc, tw, th)` is now the ONE copy of §9.4.4's
  displacement formula, shared by `text_extract::page::show_code`,
  `redact::glyph`, and the decompose walk — a third, independent
  implementation was about to be written before this consolidation.

**Findings + decisions (empirical):**
- **Two latent decompose-walk bugs found and fixed in passing, both
  invisible under the old ±1-em-inflated geometry:** (1) `'` and `"` did
  not perform their `T*` line move, and `"` did not set `Tw`/`Tc`
  (§9.4.3 Table 109); (2) `Tc`/`Tw`/`Tz`/`Ts` were not tracked in the
  decomposer at all — `GState` now carries them with Table 105 initial
  values, saved/restored across `q`/`Q`. General lesson: making geometry
  honest exposed correctness bugs the sloppy geometry had been masking.
  Project-internal (pdfce's own decompose walk), not escalated to any
  RAG.
- **PDF-domain finding, escalated to `C:\personal_rag\pdf\`:** ISO
  32000-1 §9.8 Table 122 marks `/Ascent`/`/Descent` Required on any
  non-Type-3 font descriptor, but real subsetted CIDFonts (and pdfce's
  own synthetic CID fixture) frequently omit the descriptor ENTIRELY —
  the `/FontBBox`-then-standard14-then-nominal-em fallback ladder is
  load-bearing in practice, not defensive. Filed as
  `C:\personal_rag\pdf\lesson_20260803_cidfont_descriptor_ascent_descent_often_absent.md`;
  subject `index.md` and master `index.md` both updated. This subject
  (`personal_rag/pdf`) already existed on disk (seeded 2026-07-29 from
  the separate Open PDF Studio fork project, per that subject's own
  `index.md`) — not created this continuation, just added to.
- **CLI output contract change (rule 11), additive not breaking:**
  `object-list` text rows gain `bounds=font-metrics|
  metric-advances-nominal-height|estimated-advances|em-box`; bbox
  coordinates now print at 4 dp with trailing zeros trimmed (previously
  seventeen digits of `f64`/`f32`-widening artefact, four orders below
  the hit tolerance).

**Still in flight:**
- **Documentation-only reconciliation still owed, NOT built this
  continuation** (flagged by the engineer, out of scope for a builder
  — this is `pdfce-ui-specialist` territory):
  `docs/ui_specs/pass-17-dock-and-layer-tree.md` §0.2/§B.3 still
  describe the OLD "wider and taller than the ink" bbox model in the
  present tense, even though the same file's own §E (written earlier
  this family) already carries the correct model. Needs a
  `pdfce-ui-specialist` re-dispatch to reconcile the two sections
  against each other.
- All six numbered Pass 18.x slices (18.0-18.5) plus this Pass 18.6
  follow-on are now SHIPPED. The whole decision-017/Amendment-A/ui-spec
  family is closed except the documentation item immediately above.
- `-Pid` on the observation scripts; no GUI redaction-apply flow;
  `✓`/`✕` verification; status-bar/fit-zoom feedback loop (standing
  hazard, unchanged); letter badges pending real icons; `egui_kittest`
  harness — all carried, unchanged from continuation 60.
- Operator priority sequence items #3 (text-handling: FF-B/FF-H/FF-C)
  and #4 (form-building) remain the next concrete dispatch — this
  continuation was a bug-fix follow-on, not a step toward either.

**For next session:**
- Items #3 (text-handling fast-follows) and #4 (form-building) remain
  the recommended next dispatch unless the operator gives a new steer.
- Flag to the operator at next contact, all carried: (1) the icon build
  shipped ahead of the Pass-17-first sequencing at continuation 57
  (unresolved flag); (2) the GUI has no redaction-apply flow at all;
  (3) R86 (whether headless-only verification is acceptable going
  forward) remains unanswered; (4) there is still no git remote
  configured — the 37-commit branch exists solely on this machine, and
  the refreshed backup bundle
  (`D:\Dev\pdfce-backups\pdfce-20260803-0830.bundle`) is the only
  off-session copy; (5) the branch is still named `pass-8-redaction`
  though it now spans Passes 9-18.6 — worth a rename whenever a push is
  authorized. Also newly worth a dispatch: a `pdfce-ui-specialist`
  re-visit to reconcile ui-spec §0.2/§B.3 against its own §E (item
  above).
- Carried, unchanged: push/publish call; encryption `/R 6` +
  `LEGAL.md` §2; list-authoring scope.
- One new `C:\personal_rag\pdf\` finding filed this continuation (see
  above); no `D:\dev\rag\rust\`/`D:\dev\rag\egui\` findings this
  continuation (nothing ecosystem-generalizable surfaced — the two
  latent decompose-walk bugs and the four-basis bbox design are
  pdfce-internal, and the CID-descriptor-omission finding is PDF-domain,
  not Rust/egui).

**Same-day continuation 62 (real date 2026-08-03) — Decision 019 filed
(FF-H re-scope: `Tc`/`Tz` + free-form `Ts` + synthetic bold/italic ship,
`Tw` evidence-gated by census, minimal StructTree/`/ActualText` CUT and
re-filed as FF-I); ui-spec §0.2/§B.3 marked historical, closing the
last open Pass-18.x/decision-017 reconciliation item.** Two commits,
hashes engineer-verified via `git cat-file -t` (R87): `67f49bb` (ui-spec
historical marking, on top of `1b38e34`, continuation-61 HEAD) →
`743e463` (decision 019's record, `docs/decisions/019-ffh-spacing-scaling-synthetic-styles.md`).
Branch `pass-8-redaction`, chain now **39 commits**, still no git remote
configured. Backup bundle: `D:\Dev\pdfce-backups\pdfce-20260803-0830.bundle`
(unchanged this continuation — no new build artifact to re-verify
against; this was a docs/decision-filing continuation, not a code
ship). **This continuation is pure librarian filing** — no `cargo test`/
`cargo fmt`/`cargo clippy`/`cargo tree` gates apply (no `pdfce-core`/
`pdfce-render`/`pdfce-gui`/`pdfce-cli` source changed); Pass 19.0 itself
(the code consolidation this decision names as its first slice) is
**IN PROGRESS**, being built by a separate dispatch concurrently with
this filing, not yet shipped.

**Shipped:**
- Nothing code-shipped this continuation. This is a decision-filing/
  roadmap-bookkeeping continuation, same pattern as continuation 44
  (decision 016) — Pass 19.0 (text-state consolidation + ambient
  publication) is the first concrete build against this decision and is
  IN PROGRESS, not SHIPPED, as of this entry.

**Decisions made this session:**
- **Decision 019 ACCEPTED (KenAgent protocol).** Full record:
  `docs/decisions/019-ffh-spacing-scaling-synthetic-styles.md`. Amends
  decision 014 §5.3 (FF-H's original four-operator bundle) and decision
  016 §2 (FF-H's "defer" ranking — superseded by the operator's
  priority-#3 directive; §2's own StructTree-is-premature reasoning is
  upheld and acted on, not overturned).
- **The parity premise for `Tw`/free-form `Ts` collapsed before any code
  was written.** Acrobat-parity research
  (`D:\Dev\Rag-Specialized\Acrobat_Features\text_edit__spacing_and_scaling_controls.md`,
  sourced to Dov Isaacs, former Adobe Principal Scientist) establishes
  Acrobat itself dropped word spacing and free-form baseline offset when
  text editing consolidated into the single Edit Text & Images tool;
  retained: "adding, deleting, bold, italic, font size, leading,
  kerning, and horizontal scaling." FF-H's own name lists four
  operators; parity covers two.
- **Q1 — the operators SPLIT on font-model universality vs. marginal
  cost, not parity-vs-exceed.** Free-form `Ts` SHIPS as a deliberate
  exceed (superscript/subscript is a parity must-have and forces the
  mechanism regardless; works identically on every font model, no void
  case). `Tw` does NOT ship as a direct authoring control in the core
  slices — its job already belongs to the 15.1 reflow layer's `TJ`
  design (decision 015 rejected `Tw` there for exactly the properties
  that make it a poor control); any future promotion is gated behind a
  corpus census with explicit bands (≥60% build / ≤25% close / 25–60%
  escalate to the operator — R91).
- **Q2 — synthetic bold/italic: one shared policy for in-place-edit
  (14.x) and add-text (16.x); only the remedy-offer ORDER differs, and
  that's disclosed, not designed asymmetry.** Mechanism: `Tr 2`
  (stroke+fill, stroking colour matched to fill, stroke width
  user-space-derived via `Tfs × |Tm| × |CTM|` — both real bugs if
  missed, §9.3.6) + `Tm` shear for oblique; **double-strike explicitly
  rejected** (doubles glyph count, breaks the byte↔glyph correspondence
  provenance depends on). Self-evident emission, no private marker
  (R90).
- **Q3 — build order FF-H → FF-C → FF-B, decided on Pass 19.0 being a
  shared correctness prerequisite the other two inherit, not on FF-H's
  own value** (judged the least of the three). Pass 18.6's `GState`
  tracker is explicitly NOT groundwork for this decision (~5% credit at
  most) — it's a private *reading*-path walk, not callable from any
  authoring path; its real contribution is being the THIRD private
  ambient-state tracker, which makes the Pass 19.0 consolidation
  argument unarguable rather than merely tidy.
- **StructTree/`/ActualText` CUT from FF-H entirely, re-filed as its own
  ungated Backlog item, FF-I** — a partial structure-tree writer judged
  worse than none. No Pass number assigned.
- **New standing rules R88–R91** (ceiling was R87): R88
  restore-by-value scoping (q/Q illegal inside `BT…ET`, §8.2 Table
  51/Figure 9); R89 store-ratios-derive-at-emit (`Tc`/`Ts` are unscaled
  text-space units per §9.3, not rescaled by `Tfs`); R90 synthesis is
  per-use/declinable/fallback-only/self-evident; R91 `Tw` is
  font-model-capability-gated, composite inter-word distribution is
  `TJ`-only.
- **`ROADMAP.md` edited:** Backlog's "ui-spec §B.4/§C follow-ons" item 1
  marked RESOLVED (commit `67f49bb`) — §0.2/§B.3 were corrected earlier
  the same day to describe the em-box geometry accurately, then Pass
  18.6 replaced that geometry, so the corrected text became an accurate
  description of behavior that no longer exists; `67f49bb` marks it
  historical (before/after bboxes both kept) rather than deleting it.
  New ★ Pass 19.x entry under Next up (slices 19.0–19.4, 19.0 marked IN
  PROGRESS). New Backlog entry "FF-I — minimal StructTree/`/ActualText`
  update," no Pass number. ★★★ Operator priority sequence item 3 gets a
  dated amendment pointing at the new scoping. Standing rules R88–R91
  added. Five new items filed to Open operator questions (g)–(k): the
  `Tw` census middle band; FF-C's rule-13 dependency classification
  (MIT lifted rule 8, did not pre-approve any crate); the FF-I
  StructTree cut (Ken may have counted it inside "finish off all the
  text handling stuff"); list-authoring re-surfaced (still unanswered,
  unchanged); kerning — a parity gap this decision found but did not
  scope (Isaacs lists kerning among Acrobat's retained controls; pdfce
  has no kerning surface distinct from `Tc`). Commit-chain count in the
  carried push/publish item updated 30 → 39 (engineer-verified, R87).
- **`ARCHITECTURE.md` edited:** §5.11 gets a new forward-pointer
  paragraph (same convention as decision 015's FF-A pointer) recording
  the two binding architectural facts (`q`/`Q` illegal inside `BT…ET`;
  `Tc`/`Ts` unscaled-by-`Tfs`) and the three-private-trackers finding.
  §12 gets the dated decision-019 entry.

**Findings + decisions (empirical):**
- **Three code-audit findings, independently engineer-verified in the
  main tree before filing (not assumed from the decision record alone):**
  (1) `reflow_apply.rs` emits `Tc`/`Tz`/`Tw` in its preamble and
  terminates at `ET` with no restore and no `q`/`Q` — a live, currently
  masked state leak, benign only because the justify gate refuses
  whenever `|tc| > ε || |tw| > ε`, which is exactly the gate Pass 19.1
  would want to relax; (2) `Ts`/`Tr` are tracked nowhere in the
  authoring path (`text_edit::edit::Walk` has no `b"Ts"`/`b"Tr"` arm)
  while the read path tracks both and drops them at
  provenance-construction time; (3) ambient spacing state is tracked
  three times privately (`text_extract::page::TextState`,
  `text_edit::edit::Walk`/`reflow_apply::BlockTextState`,
  `vector::decompose::GState`) and published zero times —
  `text_extract::font::advance_tx`'s own doc comment already concedes
  "three copies that agree today." All three are project-internal
  (pdfce's own code structure), not escalated to any RAG.
- **A removal during a UI consolidation is strong evidence about the
  consolidation and weak evidence about the feature on its own — worth
  recording as reasoning method, not just this decision's conclusion.**
  For `Tw` specifically a SECOND, independent signal points the same
  way (structurally void for 2-byte composite/CID runs, §9.3.3, and its
  one honest job already collapsed into 15.1's `TJ` design) — no such
  second signal exists for `Ts`, which is why the two operators split
  despite Acrobat dropping both.
- **`q`/`Q` scoping inside a text object is spec-ILLEGAL** (§8.2 Table
  51/Figure 9), not merely an inferior choice — this rules out the
  "wrap the formatted run in its own `q…Q`" design outright, not just
  as a style preference. Spec-governed fact, already citeable directly
  from the ISO text; not escalated as a personal_rag/pdf finding (that
  tier is for empirical real-world-PDF divergence, not spec text
  itself) and not a `pdfce-spec-librarian` dispatch needed (the citation
  is already precise and uncontested).

**Still in flight:**
- **Pass 19.0** (text-state consolidation + ambient publication) — IN
  PROGRESS, a builder is on it now, concurrently with this filing.
  19.1–19.4 not started.
- **Five new Open operator questions (g)–(k)** — see above; none block
  Pass 19.0's build.
- Carried, unchanged from continuation 61: `-Pid` on the observation
  scripts; no GUI redaction-apply flow; `✓`/`✕` glyph verification;
  status-bar/`Fit page`-zoom feedback loop (standing hazard); letter
  badges pending real icons; `egui_kittest` harness gap.
- Branch still named `pass-8-redaction` though it now spans Passes
  9–19.0 — worth a rename whenever a push is authorized (unchanged
  flag, now more pointed at 39 commits).
- No git remote configured; backup bundle
  (`D:\Dev\pdfce-backups\pdfce-20260803-0830.bundle`) remains the only
  off-session copy.

**For next session:**
- Continue Pass 19.0 (consolidation) to completion, then dispatch 19.1
  (`Tc`+`Tz`+super/subscript, the Acrobat-parity slice) — per ★ Pass
  19.x's Q3 ordering, FF-H finishes before FF-C or FF-B start.
- Before 19.2 (free-form `Ts` + synthesis) starts, run the named
  prerequisite check: confirm `pdfce-render` actually honours `Tr 2`
  and a sheared `Tm`, or R85 (preview-equals-saved) breaks the moment
  that slice ships a feature the canvas can't paint.
- Dispatch `pdfce-ui-specialist` before 19.3 (the GUI property surface).
- Flag to the operator at next contact, all carried or newly filed:
  (1) the five new Open operator questions (g)–(k), above; (2) push/
  publish call still ungranted, chain now 39 commits, still no remote;
  (3) branch-rename-on-push still pending; (4) the GUI still has no
  redaction-apply flow; (5) R86 (headless-vs-observed "done" definition)
  still unanswered.

**Same-day continuation 63 (real date 2026-08-03) — Pass 19.0 SHIPPED
(`38fffad`, shared text-state model); decision 019 Amendment A filed
(`1a2e265`, three deviations from the decision as originally written +
a live `q`/`Q` defect fixed in already-shipped Pass 14.2 code); the
observation-scripts `-Pid` Backlog item RESOLVED (`f45d8d6`,
`-ProcessId` disambiguation). Three commits, hashes engineer-verified
via `git cat-file -t` (R87): `f45d8d6` → `38fffad` → `1a2e265`. Branch
`pass-8-redaction`, engineer-reported chain now **43 commits**, still no
git remote configured. Pass 19.1 (`Tc`/`Tz`/super-subscript authoring)
is now IN PROGRESS, a separate builder dispatch.**

**Shipped:**
- **`f45d8d6` — observation scripts refuse an ambiguous target.** Both
  `observe-gui.ps1` and `gui-click.ps1` previously picked a target
  process via `Select-Object -First 1` over the process name; with two
  `pdfce-gui` instances running that silently selects one, and a
  synthesized click landing in the wrong window is an unintended action
  on some other running application, not merely a failed observation.
  Both scripts now take an optional `-ProcessId` parameter and REFUSE
  when several candidates exist and none was named, listing the
  running ids. Named `-ProcessId`, not `-Pid` — `$Pid` is a PowerShell
  automatic variable and shadowing it fails confusingly. Verified all
  three paths live (single instance / `-ProcessId` given / ambiguous-
  and-refused); the verification run incidentally caught the
  client-area blank-frame guard (`6a6a48f`) working as intended on a
  real failure (a fully white client area under a painted title bar),
  and confirmed a mouse *move* does not wake eframe where a real click
  does — first time this failure mode was caught by the harness rather
  than reasoned about after the fact. Resolves the Backlog item filed
  at Pass 18.5's ship.
- **`38fffad` — Pass 19.0 (shared text-state model), decision 019's
  first slice, SHIPPED.** New `pdfce-core/src/text_state.rs`:
  `TextStateParam`/`TextStateParams` (identity + resolved values) and
  `AmbientValue`/`AmbientOrigin`/`AmbientTextState`/`AmbientRestoreError`
  (values + restore provenance, now four rungs — see Amendment A
  below). One `apply_operator` rule shared by all three former private
  walks. `GlyphProvenance` gains `text_state` + `composite`.
  `cargo test --workspace` 1613 → 1643 passed, 0 failed; fmt/clippy
  clean; `check-ui-strings.sh` exit 0; `cargo tree` clean; zero new
  dependencies; `fixtures/synthetic` roundtrip byte-identical.
- **`1a2e265` — decision 019 Amendment A**, recording the three
  deviations below plus the `q`/`Q` defect. `ARCHITECTURE.md` §5/§12
  both updated to match (decision log + body section together, per this
  file's own discipline).

**Decisions made this session:**
- **R88's restore ladder needed a FOURTH rung, and its wording is
  CORRECTED, not merely clarified.** The original wording ("observed
  raw operand bytes when set") assumed a setter's bytes are either
  available or absent. There is a third case: **available and
  poisonous.** `TD` sets `TL` as a documented side effect of moving the
  line (§9.4.3 Table 108); `"` sets `Tw`/`Tc` **while showing a
  string** (Table 109) — replaying a captured `"`'s raw bytes as a
  spacing-only restore *repaints the text*. Resolved with
  `AmbientOrigin::ObservedIndirect { setter }`: the value is known but
  its source operator did more than set it, so the restore RE-SPELLS
  the value in its own dedicated operator, and `is_byte_faithful()`
  reports `false` for disclosure. **New R88 wording:** restore from raw
  bytes where they are a faithful and side-effect-free record →
  re-spell where the value is known but its source operator did more
  than set it → refuse where unobservable.
- **§3.4's tier-3 case (i), multi-stream `/Contents`, is architecturally
  UNREACHABLE today, not merely rare.** `ContentStream::from_page`
  concatenates the entire `/Contents` array before any operator walk,
  and a decode failure fails the whole page rather than yielding a
  partial prefix. Recorded with the condition that would make it real
  again (lazy/per-element concatenation) rather than manufacturing an
  untestable trigger to exercise a currently-dead branch.
- **`Tf`/`Tfs` are explicitly OUT of the R89 unification — "exactly one
  definition" is narrowed to the six single-operand parameters R88
  covers.** The extraction walk narrows `Tfs` to `f32` to publish
  `GlyphProvenance::tf_size`, then re-widens it for the §9.4.4 advance
  computation; unifying to `f64` throughout would perturb already-
  published glyph positions bit-for-bit (same narrow-then-divide vs.
  divide-then-narrow trap applies to `Tz`). Also: `pdfce-render::text::
  TextState` remains a deliberate FOURTH tracker (not consolidated),
  kept independent on purpose so render-parity work cannot share a bug
  with the authoring-side model.
- **Verification-methodology finding: a before/after comparison must be
  demonstrated to compare two different artifacts, not merely
  asserted to.** The first roundtrip-comparison attempt ran `git stash`
  immediately before the "before" build, but the tree was already
  clean at that point — `git stash` silently no-op'd, so both builds
  used the identical binary and "byte-identical" proved nothing. Redone
  from a genuine pre-change `git worktree` checkout. Escalated to
  `D:\dev\rag\rust\git_stash_on_clean_tree_makes_before_after_comparison_vacuous.md`.

**Findings + decisions (empirical):**
- **A live defect in already-shipped Pass 14.2 code, found by this
  slice: `text_edit::edit::Walk` had NO `q` and NO `Q` arm at all**
  (engineer-verified 0 → 1 occurrences before/after this fix). Text
  state AND fill colour leaked past a `Q` in the in-place-edit model —
  shipped Pass 14.2 behavior could re-emit a fill colour a `Q` had
  already discarded. Decision 019 §1.2's own audit of missing arms
  reported the missing `Ts`/`Tr` cases and missed this one. Recorded as
  a meta-point, not just a bug: that audit was otherwise the strongest
  part of the decision, and "the audit was thorough" is exactly the
  belief that let this gap through. Fixed with two new regression
  tests in the same Pass.
- **The `reflow_apply` state-leak flagged at decision 019's filing
  (continuation 62) is now closed, and the justify gate is left
  untouched.** `restore_ops` emits only on divergence, and emits
  nothing on any current fixture (which is why roundtrip is unchanged);
  a dedicated tripwire test
  (`reflow_leaves_the_following_text_state_unchanged`) fails the moment
  19.1 relaxes the justify gate without a restore.
- **The `"` operator's raw bytes are a general re-paint hazard for any
  text-state restore mechanism, not just pdfce's.** Escalated to
  `C:\personal_rag\pdf\lesson_20260803_quote_operator_side_effect_poisons_raw_byte_restore.md`
  — any PDF editor capturing "the bytes that set this parameter" for
  later replay must classify the setting operator as side-effect-free
  vs. side-effect-bearing first.
- **Rule 11 (CLI parity) — deliberately not extended this Pass.** No
  CLI surface added. Recorded recommendation for 19.1:
  `extract-text --json` should carry the published ambient state (not
  `object-list`, the wrong home — paint-order/hit-test inventory keyed
  on vector objects, not per-run text state) — and not until 19.1
  decides `MetricSpec::{Absolute,Relative}`, so the flag's output shape
  is fixed once, not twice.
- **Commit-count arithmetic flag, not silently reconciled.** The
  continuation-60 chain (36, incl. bootstrap) + continuation-61's
  `1b38e34` + continuation-62's two commits (`67f49bb`, `743e463`) sums
  to 39 (matches the figure recorded at continuation 62); this
  continuation's three new commits would bring that to 42, one short of
  the engineer-reported **43**. Recorded per R87 rather than quietly
  absorbed — whoever next runs `git rev-list --count HEAD` should
  resolve whether a fourth, unreported commit landed or the running
  total drifted by one in continuations 60–62.

**Still in flight:**
- **Pass 19.1** (`Tc`+`Tz`+super/subscript authoring, the Acrobat-parity
  slice) — IN PROGRESS, a separate builder dispatch, concurrent with
  this filing. 19.2–19.4 not started.
- Carried, unchanged: no GUI redaction-apply flow; `✓`/`✕` glyph
  verification; status-bar/`Fit page`-zoom feedback loop (standing
  hazard); letter badges pending real icons; `egui_kittest` harness
  gap; the five Open operator questions (g)–(k) from continuation 62;
  list-authoring scope call; `LEGAL.md` §2; Encryption's `/R 6`
  sourcing method.
- Branch still named `pass-8-redaction`, now spanning Passes 9–19.0 —
  worth a rename whenever a push is authorized.
- No git remote configured; backup bundle
  (`D:\Dev\pdfce-backups\pdfce-20260803-0830.bundle`) is now STALE — it
  predates all three commits this continuation and has not been
  regenerated.

**For next session:**
- Continue Pass 19.1 to completion (`Tc`/`Tz`/super-subscript +
  `MetricSpec::{Absolute,Relative}` + the `Tz`×justify disclosure), then
  the named prerequisite check before 19.2 starts: confirm
  `pdfce-render` actually honours `Tr 2` and a sheared `Tm`.
- **RESOLVED (engineer, same continuation): the 39→43 commit-count flag.**
  `git rev-list --count HEAD` = **43**, which is correct. The apparent
  gap was an anchoring error: 39 was the count at `743e463`, but the
  immediately preceding filing was `0c385a9`, where the count was 40 —
  so 40 + 3 (`f45d8d6`, `38fffad`, `1a2e265`) = 43. The flag was right
  to be raised; raising it rather than silently reconciling is exactly
  what R87 asks for, and it is the third filing-integrity issue this
  habit has caught. Lesson kept: a running total is only as good as the
  anchor it is added to, and "the last number I filed" is not
  necessarily "the number at the last commit I am counting from".
  Backup bundle regenerated at
  `D:\Dev\pdfce-backups\pdfce-20260803-1145.bundle` (verified), which
  also closes the stale-bundle flag.
- Regenerate the backup bundle to cover the three continuation-63
  commits.
- Flag to the operator at next contact, all carried: (1) the five Open
  operator questions (g)–(k); (2) push/publish call still ungranted,
  chain now 43 commits (verified by `git rev-list`), still no
  remote; (3) branch-rename-on-push still pending; (4) the GUI still
  has no redaction-apply flow; (5) R86 (headless-vs-observed "done"
  definition) still unanswered.
- No `D:\dev\rag\rust\`/`D:\dev\rag\egui\`/`C:\personal_rag\pdf\`
  findings filed this continuation — everything surfaced (the three
  code-audit findings, the `q`/`Q` illegality, the removal-as-evidence
  reasoning) is either pdfce-internal, already-spec-citeable, or already
  housed in the Acrobat-parity RAG (`pdfce-acrobat-librarian`'s
  territory, not re-filed here).

**Same-day continuation 64 (real date 2026-08-03) — the 39→43
commit-count flag CLOSED for real (`5c1f5dc`); Pass 19.1 SHIPPED
(`603b051`, `Tc`/`Tz`/super-subscript direct text-state authoring, the
decision-019 Acrobat-parity slice); decision 019 Amendment B filed
(three corrections found while building 19.1, one new standing rule
R92). Two commits, both engineer-verified via `git cat-file -t`:
`5c1f5dc` → `603b051`. Branch `pass-8-redaction`, **45 commits**
confirmed directly by `git rev-list --count HEAD` (not merely
engineer-reported this time), still no git remote configured. Pass
19.2 (free-form `Ts` + synthetic bold/italic) is now IN PROGRESS, a
separate builder dispatch.**

**Shipped:**
- **`5c1f5dc` — the continuation-63 commit-count flag (39→43) is
  resolved for real, and the backup bundle is regenerated.** The
  arithmetic gap was an ANCHORING error, not a computation error: 39
  was the commit count at `743e463`, but the filing that reported "43"
  was actually anchored on the immediately preceding filing, `0c385a9`,
  where the count was **40** — so 40 + 3 (`f45d8d6`, `38fffad`,
  `1a2e265`) = 43, and 43 was correct all along. Backup bundle
  regenerated at `D:\Dev\pdfce-backups\pdfce-20260803-1145.bundle`
  (verified against the 43-commit chain at the time of its creation).
- **`603b051` — Pass 19.1 SHIPPED.** `Tc`/`Tz`/super-subscript direct
  text-state authoring, extending `format.rs`'s existing splice with no
  parallel path. New `MetricSpec`/`ScriptPosition`/`ScriptMetrics`
  types, three `FormatRequest` fields, seven `FormatReport` fields, two
  `FormatError` variants, `push_state_param`, `derived_operand`, five
  new disclosures. CLI: `--char-spacing`/`--h-scale`/`--superscript`/
  `--subscript`/`--no-script`. `cargo test --workspace` 1643 → 1663, 0
  failed; fmt/clippy clean; `check-ui-strings.sh` exit 0; `cargo tree`
  clean; zero new dependencies; R85 16/16 (new
  `format_text_spacing_preview_equals_saved` case); roundtrip unchanged,
  proven non-vacuous by `md5sum`-distinct binaries from a genuine
  pre-change `git worktree` build. See the Pass 19.1 Shipped entry
  (top of `ROADMAP.md` Shipped) for the full record, including the
  engineer-verified emitted content stream, the superscript/subscript
  ratio disclosure, and the restore-ladder rung coverage (1/2/3
  end-to-end; rung 4 unreachable end-to-end by design, same posture as
  decision 019 Amendment A.2).

**Decisions made this session:**
- **Decision 019 Amendment B filed** — three corrections found while
  building Pass 19.1, recorded in
  `docs/decisions/019-ffh-spacing-scaling-synthetic-styles.md`
  Amendment B, `ARCHITECTURE.md` §12/§5.11, and this file's ★ Pass 19.x
  entry + Pass 19.1 Shipped entry:
  1. **The `Tz` × justify disclosure named the wrong MECHANISM.** `Th`
     genuinely rescales every `TJ` numeric adjustment per §9.3.4 — but
     the specific `TJ` numbers carrying a 15.1-justified line's slack
     sit OUTSIDE the formatted run's `set_ops`/`restore_ops` wrap (in
     the `pre`/`post` splice segments) and therefore run at ambient,
     unchanged `Th` — they are NOT rescaled by the edit. The
     conclusion (a `Tz` edit invalidates a justified line and needs a
     re-justify offer) survives; the cause is the run's changed
     rendered WIDTH (`ΔA`, §9.4.4), not a `TJ`-value rescale. General
     lesson: a spec fact being true in general (`Th` scales `TJ`) is
     not evidence it's the operative cause in a SPECIFIC architecture
     — the wrap/splice boundary has to be checked directly.
  2. **A flagged spec-citation error (`Ts` cited as §9.3.6 instead of
     §9.3.7) was verified NOT to exist in the decision document** —
     only in `text_state.rs` (three comment citations, already fixed).
     The document's own internal "(§1.3.6)" is a cross-reference to its
     own §1.3 item 6, not an ISO clause, and its actual References
     section already correctly said "§9.3.7 rise." Closed with an
     explanation rather than an unnecessary edit.
  3. **R89's "`Tfs`" is now stated explicitly as the BASE font size** —
     the decision text left this ambiguous (which `Tfs`, if size and
     superscript/subscript are edited together); the implementation had
     already chosen base, and the record now says so.
  A fourth flagged item (R88's four-rung wording in `ROADMAP.md`'s
  Standing Rules) was checked and found **already correct** — no edit
  needed, closed by inspection rather than left open.
- **New standing rule R92 (methodology).** A predicate that
  hand-duplicates the shape of a data structure it inspects (an
  exhaustive no-op/emptiness check, a hand-listed operator-arm list)
  drifts silently the moment the structure gains a field or case.
  **Second occurrence of this exact bug shape in this project:** the
  first was decision 019 Amendment A.4 (`text_edit::edit::Walk`'s
  missing `q`/`Q` arms); the second, found this continuation, is
  `EditSession::format_text`'s own hand-listed no-op predicate
  (`set_size.is_none() && set_fill.is_none() && set_font.is_none()`),
  which Pass 19.1's new `FormatRequest` fields bypassed entirely —
  making a spacing-only request a phantom `NoOp` on the **GUI-facing
  `EditSession` path specifically** (the CLI's `set_format` path, using
  the real `FormatRequest` directly, was unaffected). Fixed with
  `req.is_empty()`. Caught by the R85 oracle — its SECOND real catch
  this arc (the first was the Pass 17.x era's `flatten_fields`
  silent-data-loss family).

**Findings + decisions (empirical):**
- **Float noise nobody anticipated, now fixed:** `12.0 × 0.60` is
  `7.199999999999999` under Rust's shortest-round-trip formatting, and
  that noise was headed straight into the emitted content stream. New
  `derived_operand` rounds to 6 dp, applied ONLY to values pdfce itself
  derives (ratio-to-absolute conversions) — an operator-supplied
  absolute value passes through completely untouched, because rounding
  a typed number would be a silent modification of the caller's own
  input, not noise suppression. Filed:
  `D:\dev\rag\rust\shortest_roundtrip_float_format_needs_derived_value_rounding.md`.
- **A latent follower-mispositioning bug, fixed in passing:** the `ΔA`
  advance-delta computation evaluated one side at the ambient
  `Tc`/`Th` and the other at the new values inconsistently — correct
  only while neither could change, which Pass 19.1 is precisely the
  slice that makes untrue. Now evaluated consistently at the NEW
  values. Also: `Tz ≤ 0` is refused by name (collapses or mirrors the
  run) rather than silently clamped.
- **RAG escalations this continuation:**
  `C:\personal_rag\pdf\lesson_20260803_tz_th_rescales_tj_adjustments_not_slack_outside_wrap.md`
  (the `Tz`×justify mechanism finding, generalized beyond pdfce: verify
  which mechanism is actually live in your own splice/wrap boundaries
  before describing a spec-adjacent interaction) and
  `D:\dev\rag\rust\shortest_roundtrip_float_format_needs_derived_value_rounding.md`
  (round only derived values, never caller-supplied ones). Both indexed
  in their subject's `index.md` this same continuation.

**Still in flight:**
- **Pass 19.2** (free-form `Ts` + synthetic bold/italic) — IN PROGRESS,
  a separate builder dispatch, concurrent with this filing. Its named
  prerequisite (confirm `pdfce-render` honours `Tr 2` and a sheared
  `Tm`) appears satisfied BY INSPECTION
  (`interpret.rs:1446-1457`/`text.rs:304-313`/`interpret.rs:1134-1138`)
  but is not yet confirmed empirically — the builder is doing that with
  a rendered fixture before relying on it. 19.3–19.4 not started.
- Carried, unchanged: no GUI redaction-apply flow; `✓`/`✕` glyph
  verification; status-bar/`Fit page`-zoom feedback loop (standing
  hazard); letter badges pending real icons; `egui_kittest` harness gap;
  the five Open operator questions (g)–(k) from continuation 62;
  list-authoring scope call; `LEGAL.md` §2; Encryption's `/R 6` sourcing
  method.
- Branch still named `pass-8-redaction`, now spanning Passes 9–19.1 —
  worth a rename whenever a push is authorized.
- No git remote configured; backup bundle
  (`D:\Dev\pdfce-backups\pdfce-20260803-1145.bundle`) is now STALE again
  — it predates both commits this continuation (45 total vs. 43
  covered) and has not been regenerated.

**For next session:**
- Continue Pass 19.2 to completion (free-form `Ts` CLI/core + synthetic
  bold/italic), confirming the render-parity prerequisite empirically
  first, then dispatch `pdfce-ui-specialist` before 19.3 (the GUI
  property surface).
- Regenerate the backup bundle to cover this continuation's two commits
  (and whatever commit captures this librarian filing itself, expected
  next).
- Flag to the operator at next contact, all carried: (1) the five Open
  operator questions (g)–(k); (2) push/publish call still ungranted,
  chain now 45 commits (verified by `git rev-list`), still no remote;
  (3) branch-rename-on-push still pending; (4) the GUI still has no
  redaction-apply flow; (5) R86 (headless-vs-observed "done" definition)
  still unanswered.
- Note for whoever next audits a commit-count/hash figure: this is the
  THIRD filing-integrity issue the R87 audit habit has caught in this
  project. Keep raising discrepancies rather than silently reconciling
  them — the habit is earning its keep.

**Same-day continuation 65 (real date 2026-08-03) — Pass 19.2 SHIPPED
(`ebe35d8`, free-form `Ts` + synthetic bold/italic, the FF-H deliberate
exceed); decision 019 Amendment C filed (six corrections found while
building this slice); Amendment B's previously-pending hash confirmed
as `450a44b` and folded into the record by `8664912`. Branch
`pass-8-redaction`, **48 commits** (`git rev-list --count HEAD`), all
three hashes this filing engineer-verified via `git cat-file -t`, still
no git remote configured. Pass 19.3 (the GUI spacing/style property
surface) is now IN DESIGN — a `pdfce-ui-specialist` dispatch is
concurrently writing `docs/ui_specs/pass-19.3-text-formatting-surface.md`
as a new file only, untouched by this filing.**

**Shipped:**
- **`ebe35d8` — Pass 19.2 SHIPPED.** New
  `crates/pdfce-core/src/text_edit/synth.rs`: `StyleSynthesis`
  (shared policy type across Add-Text and in-place edit), `SynthesisPath`
  (remedy *order* is the only asymmetry between the two paths),
  `SynthesisOffer`, `OBLIQUE_TAN`/`BOLD_STROKE_RATIO`, `shear_into` (a
  true matrix premultiplication — tested against a pre-rotated matrix,
  where a naive single-component overwrite loses the lean entirely),
  `matrix_scale` (determinant-based, so a shear doesn't perturb the
  derived bold stroke width), and `detect` (reload-time re-detection,
  pdfce's own synthesis and other producers'). CLI: `--rise`,
  `--bold-synthetic`, `--italic-synthetic`. `cargo test --workspace`
  1663 → 1708, 0 failed; fmt/clippy clean; `check-ui-strings.sh` exit
  0; `cargo tree` clean; zero new dependencies; **R85 20/20** (4 new
  cases: rise, synthetic bold, synthetic italic, bold+italic+rise);
  roundtrip byte-identical over `fixtures/synthetic`, non-vacuity
  proved by `md5sum`-distinct harness binaries built from a `git
  archive` export of the base commit. Full record: the Pass 19.2
  Shipped entry, top of `ROADMAP.md` Shipped, including the
  engineer-verified emitted content stream (blue bold+italic+rise-5
  text: stroking colour matched to fill and restored to `0 G`, stroke
  width `0.264` = 2.2%×12pt restoring to `1 w`, the shear bracketed by
  absolute `Tm`s so it cannot reach the following run, no `q`/`Q`
  inside `BT…ET`).

**Decisions made this session:**
- **Decision 019 Amendment C filed** — six corrections found while
  building Pass 19.2, recorded in
  `docs/decisions/019-ffh-spacing-scaling-synthetic-styles.md`
  Amendment C, `ARCHITECTURE.md` §5.11/§12, and `ROADMAP.md`'s ★ Pass
  19.x entry + new Pass 19.2 Shipped entry + Standing Rules (R88/R90
  amended, no new rule number, ceiling stays R92):
  1. **§3.6 named the WRONG restore set.** Stroking colour and stroke
     line width are graphics state **shared with path painting**, not
     scoped text state — left unrestored, a synthetic-bold run's stroke
     settings leak into every later stroked *path* on the page, not
     just later text. Two restore obligations the decision omitted;
     now covered by two new `Walk` trackers.
  2. **§3.6's "re-emit followers with an absolute `Tm`" is narrower in
     practice, deliberately.** The builder did NOT convert a producer's
     own `Td`/`T*` into an absolute `Tm` (that rewrites the producer's
     own line structure past minimal-diff and cascades to every later
     relative move); pdfce instead REQUIRES the follower already be
     absolute and REFUSES, disclosed, otherwise — a twin test proves
     the refusal is not unconditional (the same run succeeds once the
     next line opens its own `BT…ET`).
  3. **The bold-width formula (`Tfs × |Tm| × |CTM|`) ships two of its
     three factors.** No `cm` (page-level CTM) model exists in the
     authoring walk, so a stroke synthesized inside a scaled `cm`
     context isn't compensated. **Disclosed verbatim in the builder's
     own report text** ("LIMIT, disclosed rather than hidden"), not
     found later as a silent gap.
  4. **Neither the decision nor Amendment A anticipated that synthetic
     italic needs `Tm`/`Tlm` tracking in the authoring walk at all.**
     Amendment A.3 scoped the shared text-state hoist to six parameters
     and excluded `Tf`/`Tfs`, saying nothing about `Tm` — but item 2's
     refusal gate can't be evaluated without knowing whether a follower
     is already absolute. Pass 19.2 built `Tm`/`Tlm` tracking into
     `text_edit::edit::Walk` (`BT` reset, `Td`/`TD`/`T*` derivation,
     §9.4.4 advance accumulation, a `matrix_known` honesty flag, a new
     `Rec::EndText` variant).
  5. **Two conflicts the decision never names, both refused rather
     than silently merged:** free-form rise vs. the superscript/
     subscript toggle (both write `Ts`); synthetic italic vs. `--pin`
     (the closing absolute `Tm` and `--pin`'s compensating `TJ`
     adjustment would each consume the same positional delta twice).
  6. **Add-Text synthesis is NOT wired — flagged as not delivered.**
     The shared type, gate, and wording exist and
     `SynthesisPath::AddText` is implemented and tested, but
     `addtext.rs` has no bold/italic request surface to reach it from.
     Matches the decision's own prediction the gate "will rarely even
     open here" (R79's Standard-14 default has real Bolds) but "rarely
     opens" is not "cannot be reached."
- **`ARCHITECTURE.md` §5.11's "exactly one definition" claim narrowed.**
  It is true of the six §9.3 text-state parameters specifically — the
  `Tm` matrix, stroke line width, and stroking colour are tracked
  separately and deliberately, not folded into `TextStateParams`.

**Findings + decisions (empirical):**
- **Verification-method finding, the headline one this continuation:**
  the render-honours-`Tr 2`-and-sheared-`Tm` prerequisite was confirmed
  by **mutation testing**, not by the by-inspection check the prior
  filing had recorded as sufficient. A new
  `crates/pdfce-render/tests/synthetic_style_render.rs` built fixtures,
  passed all 5 tests on first run, then the builder deliberately broke
  the renderer three separate ways and re-ran: dropping mode 2 from
  `strokes()` failed 2 tests; building `Tm` with `c = 0.0` failed 2;
  zeroing the rise failed 1 — each mutation failing exactly the tests
  it should, proving the suite's failure surface maps correctly onto
  the three mechanisms it claims to cover. With the renderer intact,
  the tests also established real facts: `2 Tr` + `2 w` paints strictly
  more ink than a plain fill with >20 of the new pixels OUTSIDE the
  filled silhouette (a true outline); `1 0 0 RG` on black-filled text
  produces genuinely red pixels (proving §9.3.6's stroking-colour rule
  is both implemented and load-bearing, not merely present); a sheared
  `Tm` moves the glyph TOP rightward by >3 px while the baseline row
  moves less than half that. Escalated as a general methodology finding
  (see RAG escalations, below) — this is the standard a by-inspection
  prerequisite check should itself have met.
- **Mode-2 faux bold is re-detectable across producers**, not just
  pdfce's own output, from `Tr` + stroke-width-to-size ratio +
  non-Bold `/BaseFont` alone — the false-positive guard is that a
  deliberate outline display style strokes at 5-10% of size (meant to
  be visually obvious) versus pdfce's synthesized ~2.2% (meant to be
  imperceptible as an effect).

**RAG escalations this continuation:**
- `D:\dev\rag\rust\prove_test_suite_non_vacuous_by_deliberately_breaking_the_thing_it_tests.md`
  — the mutation-testing methodology above, generalized: a passing
  "renderer honours feature X" test suite isn't proven non-vacuous
  until you break X and watch it fail on exactly the tests that
  exercise it. Pairs with the existing `git_stash`-vacuous-comparison
  finding (both are "a green result isn't evidence until it's gone red
  for a controlled reason").
- `C:\personal_rag\pdf\lesson_20260803_mode2_faux_bold_re_detectable_by_stroke_ratio.md`
  — the mode-2 faux bold re-detection finding above.
- Both indexed in their subject's `index.md` this same continuation.
  Also backfilled this continuation: the Pass 19.1-era
  `lesson_20260803_tz_th_rescales_tj_adjustments_not_slack_outside_wrap.md`
  had a subject-index entry but was missing from the master
  `C:\personal_rag\index.md` — added now, flagged so future index
  checks know it was a gap closed retroactively, not a new finding.

**Still in flight:**
- **Pass 19.3** (GUI: the spacing/style property surface) — IN DESIGN.
  `pdfce-ui-specialist` is concurrently writing
  `docs/ui_specs/pass-19.3-text-formatting-surface.md` as a new file
  only. Pass 19.4 (`Tw`, conditional on the census) not started.
- Carried, unchanged: no GUI redaction-apply flow; `✓`/`✕` glyph
  verification; status-bar/`Fit page`-zoom feedback loop (standing
  hazard); letter badges pending real icons; `egui_kittest` harness
  gap; the Open operator questions (g)–(k) from continuation 62; the
  Tw census middle-band judgement call; FF-C's rule-13 dependency
  classification; the FF-I StructTree cut; list-authoring scope call;
  the newly-found kerning parity gap (Isaacs lists it among Acrobat's
  retained controls; pdfce has no kerning surface distinct from `Tc`).
- Branch still named `pass-8-redaction`, now spanning Passes 9–19.2 —
  worth a rename whenever a push is authorized.
- No git remote configured; the backup bundle
  (`D:\Dev\pdfce-backups\pdfce-20260803-1145.bundle`) is now THREE
  commits stale (predates `450a44b`, `ebe35d8`, `8664912`) and has not
  been regenerated this continuation.

**For next session:**
- Once `docs/ui_specs/pass-19.3-text-formatting-surface.md` lands,
  build Pass 19.3 (the GUI property surface) per its spec, applying the
  R83 capability-gating discipline (consume 19.0's published composite
  flag; never reimplement the capability query in `pdfce-gui`, or the
  WASM fork loses it — R74).
- Regenerate the backup bundle to cover this continuation's three
  commits (48 total).
- Flag to the operator at next contact, all carried forward: (1) the
  Open operator questions (g)–(k); (2) push/publish call still
  ungranted, chain now 48 commits, still no remote; (3)
  branch-rename-on-push still pending; (4) the GUI still has no
  redaction-apply flow; (5) R86 (headless-vs-observed "done"
  definition) still unanswered; (6) the newly-found kerning parity gap
  is unscoped and may or may not fall inside the operator's "finish off
  all the text handling stuff" intent.
- Consider whether Add-Text synthesis (decision 019 Amendment C item 6
  — the shared type exists but has no request surface in `addtext.rs`)
  should be scoped as part of 19.3 or deferred to its own follow-on —
  not yet decided either way.

**Same-day continuation 66 (real date 2026-08-03) — Pass 19.3 SHIPPED
(`74052d3`, GUI spacing/style property surface), closing the FF-H
formatting-slice family down to the conditional Pass 19.4. Headline: a
project-wide correctness defect had disabled every property-bar Apply
in the shipped GUI since Pass 14.3, found and fixed in the same
commit. New standing rule R93. Branch `pass-8-redaction`, 51 commits,
all still local-only.**

**Shipped:**
- **`74052d3` — Pass 19.3 SHIPPED.** GUI slice: Option-B wrapper
  (`StyleOutcome`/`StyleResolution`/`probe_synthesis`/
  `preview_style_resolution` in `pdfce-core`, read-only and
  side-effect-free — `preview_style_resolution` calls `gate_synthesis`
  up to three times rather than re-deriving, byte-equality tested
  against a non-previewed commit); `pdfce-gui`: `MetricUnit`/
  `BaselineChoice`/`AmbientSnapshot`, 11 new `TextEditState` fields,
  five `FormatOp` variants, a `CollapsingHeader` property tree, five
  refusal hints, ~45 new `ui_text.rs` entries. Design record:
  `docs/ui_specs/pass-19.3-text-formatting-surface.md`, committed
  `e883e26` (`pdfce-ui-specialist`). `cargo test --workspace`
  1708 → 1722, 0 failed; fmt/clippy clean; `check-ui-strings.sh` exit
  0; `cargo tree` clean; zero new dependencies; **R85 20/20**; **R86
  observed** against a purpose-built non-default fixture (ambient
  seeded `31.2‰`/`0.7500 pt`, `92.0%`, `raised 2.5000 pt`, none
  defaulted; synthesis pre-resolution naming the real Bold resource;
  mixed-case refusal explaining the two-step path; R84 bold-on-selected
  pairing rendering correctly). Full record: the Pass 19.3 Shipped
  entry, top of `ROADMAP.md` Shipped.

**★★★★ HEADLINE FINDING this continuation — every property-bar
"Apply" in the shipped GUI, Pass 14.3 through Pass 19.2, had silently
refused every edit.** `GlyphProvenance::operator_span` publishes the
span of the operator token ALONE (`Tj`); `text_edit::edit`'s `OpRec`
records the OPERAND-INCLUSIVE extent of the same operation. `find_anchor`'s
pinned-request path (`pin_names_operator`) compared the two for EXACT
EQUALITY — since the GUI always pins from published provenance and the
authoring walk always records the wider span, the two never matched.
Confirmed live in the running application before the fix ("text to
format was not found in an editable run on the page"). **Survived
because two doc comments, on both the publisher and the consumer,
independently asserted the conventions already agreed** —
`EditRequest::pinned_span`'s "matches the same span," `text_edit/
page.rs`'s "the surgery locates the operator by exactly this span" —
both corrected in place. Found only because this Pass stopped
discarding failed pin queries with `.ok()`. Fix: `pin_names_operator`
now accepts either convention (`pin.end() == r.end && pin.start >=
r.start`); a regression test proves the relaxed match still
DISCRIMINATES a near-miss span. **Engineer-verified by mutation:**
reverting to exact-equality matching fails a purpose-built regression
test; restoring the fix passes it.

**Decisions made this session:**
- **Decision 019 Amendment D filed** (`ARCHITECTURE.md` §12, new entry
  this filing) — records the pinned-span defect as a live-defect
  finding this slice exposed, not a decision-019 design question (same
  framing as Amendment A's `q`/`Q`-arm finding). §5.11 gets a matching
  Pass 19.3 paragraph.
- **New standing rule R93** (`ROADMAP.md`) — a code comment asserting a
  cross-module contract holds is a claim, not evidence, even when two
  independent comments on both ends of the contract agree with each
  other. Third occurrence of this exact failure shape in this project:
  decision 018's `refresh_pages` comment (true through Pass 3.1,
  silently false from Pass 6.1), the `.gitattributes` ordering incident
  (the file's own `binary` rule silently overridden by a catch-all
  below it), and this continuation's pinned-span defect. Ceiling was
  R92.

**Findings + decisions (empirical):**
- **Correction to the builder's own report, engineer-verified by
  observation.** The builder had reported both `ⓘ` and `⚠` render as
  tofu. **`⚠` (U+26A0) does NOT** — captured at 4× magnification as a
  proper warning-triangle glyph, consistent with an earlier 3×
  observation; the two codepoints were conflated. `ⓘ` (U+24D8, used
  12×, the most-used symbol in `ui_text.rs`, Enclosed Alphanumerics,
  not emoji-recommended) is PLAUSIBLY tofu but remains UNVERIFIED — no
  reachable UI state displayed it this session. A future font-coverage
  pass should target U+24D8 specifically, not `⚠`. Usage tally: U+24D8
  ×12, U+26A0 ×10, U+2715 ×4, U+2714/U+2716/U+2713 ×3 each.
- Fresh-checkout integrity re-verified this session at 49 commits (1708
  tests green, fixtures byte-identical) — the `.gitattributes` ordering
  fix (`b73604d`) is holding under accumulation.

**RAG escalations this continuation — filed to `D:\dev\rag\rust\`, a
deliberate deviation from personal_rag/pdf (see the Pass 19.3 Shipped
entry's own RAG-escalation note: the lesson generalizes to any editor
publishing byte spans for later re-location, not to PDF-domain
producer-divergence behavior — a librarian judgment call under the
agent's explicit discretion to route findings):**
- `D:\dev\rag\rust\byte_span_convention_must_live_in_the_type_not_matching_doc_comments.md`
  — the pinned-span defect above, generalized: encode a published
  span's inclusion convention in the type, or relax the consuming
  matcher structurally with a near-miss discrimination test and a
  mutation-tested fix, never trust matching prose on both ends.
- `D:\dev\rag\rust\trust_but_verify_doc_comments_are_not_evidence.md`
  — the three-instance "confident wrong comment" pattern above,
  generalized as a methodology finding: two independent comments
  agreeing with each other is not corroboration when neither was
  checked against the actual data.
- Both indexed in `D:\dev\rag\rust\index.md` this same continuation.

**Still in flight:**
- Pass 19.4 (`Tw`, conditional on the census) — not started; the only
  remaining slice in the decision-019 family.
- Carried, unchanged: no GUI redaction-apply flow (R85-uncoverable by
  design); `✓`/`✕` (U+2713/U+2715) glyph verification still owed;
  status-bar/fit-zoom feedback loop; letter badges pending real icons;
  `egui_kittest` harness gap; Open operator questions (g)–(k); the `Tw`
  census middle-band judgement call; FF-C's rule-13 dependency
  classification; the FF-I StructTree cut; list-authoring scope call;
  the kerning parity gap.
- Branch still named `pass-8-redaction`, now spanning Passes 9–19.3 —
  worth a rename whenever a push is authorized.
- No git remote configured; backup bundle
  (`D:\Dev\pdfce-backups\pdfce-20260803-1400.bundle`) now two commits
  stale (predates `e883e26`, `74052d3`) and not regenerated this
  continuation.
- **Unresolved bookkeeping note:** the third engineer-reported hash
  this continuation, `25b2d0e`, was not independently described in the
  handoff this filing was built from — filed here as the presumed
  continuation-65 librarian-filing docs commit (matching the
  established per-continuation pattern), NOT asserted as engineer-
  confirmed fact. Flag for confirmation at next contact — see R87.

**For next session:**
- Flag to the operator at next contact, all carried forward: (1) the
  Open operator questions (g)–(k); (2) push/publish call still
  ungranted, chain now 51 commits, still no remote; (3)
  branch-rename-on-push still pending; (4) the GUI still has no
  redaction-apply flow; (5) R86 (headless-vs-observed "done"
  definition) still unanswered; (6) the kerning parity gap unscoped;
  (7) **NEW — the FF-H formatting family is now feature-complete except
  the conditional `Tw` census (Pass 19.4)**; (8) **NEW — confirm what
  `25b2d0e` actually is**, per the bookkeeping note above.
- Regenerate the backup bundle to cover this continuation's commits
  (51 total).
- Confirm the `ⓘ` (U+24D8) tofu suspicion by reaching a UI state that
  displays it, before scoping a font-coverage fix.

**Same-day continuation 67 (real date 2026-08-03) — the `Tw` census
(Pass 19.4's gating measurement, decision 019 §3.3) has been RUN:
BUILD band cleared (91.6% of show operators / 97.4% of glyphs, n=4,012
real PDFs), but Pass 19.4 itself has NOT started. The engineer
prioritized fixing a newly-found pdfce defect instead — 341 corpus
files (8.5%) refuse to open at all on a `/Contents` array element that
resolves to Null, a fail-clean violation, hand-verified as a legal
file wrongly refused. Branch `pass-8-redaction`, 54 commits, all still
local-only.**

**Shipped (measurement, not a Pass):**
- **`359d486`/`5387699` — the `Tw` corpus census, `tools/tw-census`.**
  New out-of-workspace crate (zero new Cargo dependencies, root
  `exclude`-list convention), both commits verified by `git cat-file
  -t`. Unit of measurement: the show operator, keyed by
  `(ContentStreamRef, ByteSpan)` from `GlyphProvenance` — decision
  019 §3.3's own named unit, deliberately not pdfce's `TextRun`. Keys
  pooled per page; deterministic sort order; the aggregating `HashMap`
  summed over exhaustively, never sampled; two full runs produced
  byte-identical aggregates. Ground-truth calibration built as a TEST
  (known-simple/known-composite fixtures must classify correctly), not
  a spot-check.

**Findings + decisions (empirical):**
- **The numbers.** Denominators exclude 627 unloadable files and 2,172
  zero-show-operator files: text-bearing denominator = 1,224 documents
  / 23,144 show operators / 620,858 shown character codes.

  | denominator | loose (simple font) | strict (simple AND code 32) |
  |---|---|---|
  | by document (n=1,224) | 86.7% | 43.9% |
  | by show operator (n=23,144) | **91.6%** | 36.9% |
  | by glyph (n=620,858) | **97.4%** | 55.7% |
  | median per-document glyph share | 100.0% | 0.0% |

  Sub-corpus (loose): pdf20examples 100% · qpdf 99.6% · pdfbox 89.2% ·
  veraPDF 87.6% · pdfium 42.1% (smallest sample, 30 docs). Font mix:
  all-simple 994 (81.2%) · all-composite 163 (13.3%) · mixed 67 (5.5%).
  Operator prevalence: `Tc` 19.6% · `Tw` 10.9% · `Tz` 1.2% · `TL` 17.6%
  · `Ts` 0.1% · `Tr` 7.1%.
- **VERDICT: 91.6% → R91's BUILD band (≥60%).** Not marginal — every
  loose denominator clears 60%, the median document is 100% simple,
  survives removing the four most-glyph-heavy files at 87.3%.
- **★★★ Decision 019's §3.2 reason 2 is FALSIFIED on this corpus.** The
  decision partly justified withholding `Tw` on producers defaulting
  to Type0/Identity-H composites "even for pure-Latin text... a large
  and growing share" — but 81.2% of text-bearing documents contain no
  composite run at all. The "growing" half is separately recorded as
  UNTESTABLE on this corpus (PDF-tooling test suites as old as Isartor
  2008, not a sample of recently-produced documents). Corpus-bias
  caveat: 72% of the text-bearing set is veraPDF (2,053/2,896 loadable
  veraPDF files have no text at all); `pdfbox`'s sub-corpus (real
  user-submitted bug attachments) is closest to organic documents here
  and is the MOST favourable to `Tw` (95.9% loose by glyph) — the
  blended figure under-states reachability if anything.
- **The strict metric is flagged untrustworthy, not acted on.** Lands
  in the 25–60% escalate band but moves 12 points on removing four
  files (top file = 18.6% of all glyphs; three biggest veraPDF
  contributors are implementation-limit probes with 32k–65k glyphs and
  zero code-32); also structurally asymmetric (composite runs carry
  space as a CID, not code 32 — corpus-wide composite code-32 total is
  73). The decision's bands are written against, and satisfied by, the
  loose metric only.
- Other honest limits: `/ActualText` blind spot (99.6% agreement with
  the independent text-extraction harness over 2,892 files, all 11
  disagreements being `/ActualText`/Unicode-CMap probes); 5 show
  operators disagreeing about the composite flag (impossible in
  principle, unchased); text-free bucket conflates blank pages with
  undecodable streams; a TSV header/failure-row shape mismatch the
  builder found and fixed in its own tool mid-run (R92 instance),
  disclosed rather than hidden.
- **★★★★ THE MORE VALUABLE FIND — a pdfce defect, engineer-verified:
  341 corpus files (8.5%) are unopenable** with "page /Contents is
  neither a stream nor an array of streams." Hand-verified NOT a
  correct rejection: `fixtures/external/qpdf/qpdf/qtest/qpdf/
  add-contents.pdf` is legal — `/Contents [ 4 0 R 5 0 R 6 0 R ]`, all
  eight objects present, three intact text-bearing streams — and
  pdfce refuses the whole document. Two separable causes: (1) Pass
  13b's rebuild-by-scan recovery undercounts objects on this file
  (reports 7 where 8 exist), so a `/Contents` element resolves to
  Null; (2) independent of (1), a single unresolvable element condemns
  the ENTIRE document, when §7.3.10 makes a dangling reference the
  null object and Table 30 makes `/Contents` optional. A fail-clean
  violation. **A builder is fixing both now**, instructed to keep the
  two causes separate, disclose rather than silently degrade, and
  prove newly-opening files render real content, not blank pages.

**Decisions made this session:**
- **Decision 019 Amendment E filed**
  (`docs/decisions/019-ffh-spacing-scaling-synthetic-styles.md`,
  `ARCHITECTURE.md` §5.11/§12) — records the census result, the BUILD
  verdict, and the §3.2-reason-2 falsification. Does not change §3.3's
  decision bands themselves, nor §3.2 reasons 1/3, nor the §9.3.3
  composite-run structural void.
- **Engineer prioritization call, recorded so it doesn't read as scope
  drift:** the newly-found `/Contents`-defect fix (341 real files
  unopenable) was prioritized above starting Pass 19.4 (a control
  reaching 91% of text-bearing documents), on the reasoning that a
  document-loading defect outranks a formatting-control build.
- **Open operator question (g) closed as moot**, not answered — the
  25–60% middle-band judgement call it asked about never became live,
  since the loose metric (what the decision bands are written against)
  landed in BUILD, not the middle band. The strict metric's
  escalate-band position is kept as recorded context, not reopened as
  a live question.

**RAG escalations this continuation:**
- `C:\personal_rag\pdf\lesson_20260803_tw_reachability_census_show_operator_91pct.md`
  — the reachability finding (91.6%/97.4%), with the corpus-vintage
  and corpus-composition caveats kept prominent (this measures what a
  PDF-tooling test-suite corpus looks like circa 2008–2024, not
  "modern producer defaults").
- `D:\dev\rag\rust\state_every_denominator_a_census_could_report.md`
  — methodology: this census's three natural denominators (document/
  operator/glyph) differ by 11 points on the identical measurement; a
  single headline figure would have been actionable-looking and wrong
  in whichever direction was omitted.
- Both indexed in their subject's `index.md` this same continuation.

**Still in flight:**
- The `/Contents`-defect fix (above) — in progress, a builder is
  working it now; not yet shipped.
- Pass 19.4 (`Tw` direct-authoring control) — cleared to build by the
  census, but sequenced behind the defect fix above; not started.
- Carried, unchanged: no GUI redaction-apply flow (R85-uncoverable by
  design); `✓`/`✕` (U+2713/U+2715) glyph verification still owed;
  `ⓘ` (U+24D8) tofu suspicion still unconfirmed; status-bar/fit-zoom
  feedback loop; letter badges pending real icons; `egui_kittest`
  harness gap; Open operator questions (h)–(k); FF-C's rule-13
  dependency classification; the FF-I StructTree cut; list-authoring
  scope call; the kerning parity gap.
- Branch still named `pass-8-redaction`, now spanning Passes 9–19.3
  plus the unshipped 19.4/defect-fix work — worth a rename whenever a
  push is authorized.
- No git remote configured; the backup bundle
  (`D:\Dev\pdfce-backups\pdfce-20260803-1400.bundle`) is now several
  commits stale and not regenerated this continuation.

**For next session:**
- Flag to the operator at next contact, carried forward: (1) push/
  publish call still ungranted, chain now 54 commits, still no remote;
  (2) branch-rename-on-push still pending; (3) the GUI still has no
  redaction-apply flow; (4) R86 (headless-vs-observed "done"
  definition) still unanswered; (5) the kerning parity gap unscoped;
  (6) Open operator questions (h)–(k) unanswered; (7) **NEW —** the
  `/Contents`-defect fix, once shipped, should get its own Shipped
  entry with the acceptance proof (files opening with real content,
  not blank pages) before Pass 19.4 starts.
- Once the defect fix ships, dispatch Pass 19.4 (`Tw`) per decision
  019 Amendment E's cleared BUILD verdict — the R83-gated,
  simple-font-only control with the refuse-and-disclose engine gate
  R91 already specifies.
- Regenerate the backup bundle to cover this continuation's commits.

## Same-day continuation 68 (real date 2026-08-03) — the `/Contents`
defect from last continuation is FIXED (`409a6b5`): 289 previously-
unopenable documents now read. Pass 19.4 (`Tw`) is now IN PROGRESS —
a builder started it this continuation, concurrent with this filing.
Branch `pass-8-redaction`, 56 commits, still no remote.

**Shipped:**
- `/Contents`-defect fix (no Pass ID — correctness fix), committed
  `409a6b5`. `BadContents` 341 → 1; text-bearing documents 1,224 →
  1,513; page-tree load failures 497 → 163; strict-path parse
  failures unchanged at 130 (confirms the fix did not loosen normal
  parsing). Zero regressions. See the new Shipped entry (top of
  `ROADMAP.md`) for the full numeric record and gates.

**Decisions made this session:**
- Chain-completeness correction filed same-continuation, committed
  `0395177`: `fb97abb` (the continuation-66 filing commit) had gone
  unreferenced in `docs/`, the SECOND time a missing commit was
  itself a filing commit rather than a code one. Standing rule R87
  amended (not a new rule) to record the structural shape of this
  blind spot: a continuation records the commits it's filing ABOUT,
  and the commit that lands the filing has no later entry to mention
  it — the audit catches it only because it compares against `git
  rev-list`, never against the previous entry's own total.
- Two new standing rules filed: **R94** (a repair that mutates a
  value must invalidate any verbatim-bytes provenance attached to
  it — generalizes the `Provenance::RecoveredFile` fix) and **R95**
  (a dangling reference inside an optional array-valued page entry
  degrades that one element, never the whole document — states the
  `/Contents`-degradation fix as binding, a read-side sibling of R67's
  forced-full-rewrite family).
- `ARCHITECTURE.md` §12 gained a decision-013 addendum entry (no new
  decision number) recording the corrected mechanism,
  `StreamLengthPolicy`, `Provenance::RecoveredFile`, and the round-
  trip-gate-catches-itself finding; §5's decision-019/FF-H body text
  updated from "fix in progress" to the resolved account.

**Findings + decisions (empirical):**
- **The diagnosis filed last continuation was wrong in MECHANISM, not
  just incomplete.** Rebuild-by-scan recovery does not undercount
  objects — the scan correctly proposes all 8 headers on
  `add-contents.pdf`; object 5 was dropped at strict-confirmation
  ("endstream not found where /Length points"). Real cause:
  `add-contents.pdf` is an LF file converted to CRLF after being
  written, so every `/Length` (measured on the LF form) is short by
  one byte per internal line, landing the declared extent
  mid-content — the SAME damage event that broke `startxref`/`xref`
  (why recovery engaged in the first place). One damage event, two
  symptoms; recovering from the first (xref) does not automatically
  reach the second (`/Length`) unless extents are explicitly
  re-derived from the `endstream` keyword.
- **The inferred SHAPE was also wrong.** Last continuation described
  an array of dangling references; ~300 of the 341 are actually a
  single indirect `/Contents N 0 R` resolving to null, only ~41 are
  the array form. Classified per element this continuation: 340
  `StreamExtentMismatch`, 12 `BadStreamLength`, 3 lexical, 2 missing
  `endobj`, 4 genuinely absent, 1 resolving to a dictionary — 337 of
  341 had the missing object's header physically present, dropped
  only at confirmation.
- **Two fixes kept deliberately separate:** `StreamLengthPolicy`
  (`Strict` default unchanged; `RecoverFromEndstream` re-derives
  extent from `endstream` per §7.3.8.2's own definition of `/Length`
  — normative, not heuristic, reachable only from recovery paths) and
  per-element `/Contents` degradation (a null-resolving reference
  degrades per §7.3.10/Table 30; a genuine type error is still
  `BadContents`, unchanged; a direct `null` is excluded from the
  disclosure count per §7.3.9).
- **★★★★ The round-trip gate caught a bug in the fix itself.** The
  first repair attempt corrected the recovered object's byte span but
  left its stale `/Length` untouched — because the writer copies
  `Provenance::File` objects verbatim, `save_full` produced a file
  pdfce itself could not reload. Fixed with a third
  `Provenance::RecoveredFile` variant forcing re-serialization
  instead of verbatim copy; both pre-existing verbatim-passthrough
  sites were already correct by construction (`let-else` skipping
  non-`File` provenance) and needed only comment updates.
- Round-trip verified non-vacuously (pre-change harness from a real
  `git archive HEAD`, confirmed `StreamLengthPolicy`-free, distinct
  binary hash from post-fix); every §5 metric identical; raster
  oracle 174 → 178 compared, all identical; `xref-recover/` alone
  0/0 → 4/4.
- Gates: `cargo test --workspace` 1722 → 1738, 0 failed; fmt/clippy
  clean; `check-ui-strings.sh` exit 0; `cargo tree` clean; zero new
  Cargo dependencies. New fixtures `xref-recover/{crlf-shifted-
  lengths,dangling-contents,dangling-contents-array}.pdf`; 7 existing
  fixtures regenerate byte-identically.
- Two follow-ups flagged, not built: 9 `load-failed` files hit
  `StreamExtentMismatch` on the strict (default) path, untouched
  correctly — a `--repair` opt-in could reach them; +5 page-tree
  cycle failures newly exposed by `BadContents` no longer masking
  them are pre-existing defects, not new breakage.

**RAG escalations this continuation:**
- `C:\personal_rag\pdf\lesson_20260803_crlf_conversion_invalidates_every_length.md`
  — the CRLF/`/Length` finding, with the normative-not-heuristic
  §7.3.8.2 reasoning and the two-symptoms-one-cause framing kept
  prominent.
- `D:\dev\rag\rust\repair_that_mutates_a_value_must_invalidate_verbatim_provenance.md`
  — the general shape of the round-trip-gate-catches-itself bug, for
  any system pairing a "these bytes are original" fast path with a
  repair mechanism that can mutate the value.
- Both indexed in their subject's `index.md` this same continuation;
  master `personal_rag/index.md` also updated.

**Still in flight:**
- Pass 19.4 (`Tw` direct-authoring control) — the blocking defect is
  fixed; a builder started the slice this continuation, concurrent
  with this filing. Not yet shipped.
- Carried, unchanged: no GUI redaction-apply flow (R85-uncoverable by
  design); `✓`/`✕` (U+2713/U+2715) glyph verification still owed;
  `ⓘ` (U+24D8) tofu suspicion still unconfirmed; status-bar/fit-zoom
  feedback loop; letter badges pending real icons; `egui_kittest`
  harness gap; Open operator questions (h)–(k); FF-C's rule-13
  dependency classification; the FF-I StructTree cut; list-authoring
  scope call; the kerning parity gap.
- Branch still named `pass-8-redaction`, now spanning Passes 9–19.3
  plus the shipped defect fix and the in-progress 19.4 — worth a
  rename whenever a push is authorized.
- No git remote configured; the backup bundle is stale and not
  regenerated this continuation.

**For next session:**
- Flag to the operator at next contact, carried forward: (1) push/
  publish call still ungranted, chain now 56 commits, still no
  remote; (2) branch-rename-on-push still pending; (3) the GUI still
  has no redaction-apply flow; (4) R86 still unanswered; (5) the
  kerning parity gap unscoped; (6) Open operator questions (h)–(k)
  unanswered; (7) the `/Contents`-defect fix is now SHIPPED — the
  item flagged last continuation is closed.
- When Pass 19.4 ships, dispatch the librarian to move it to Shipped
  and confirm the ★ Pass 19.x umbrella entry is fully retired (all
  five slices 19.0–19.4 complete).
- Regenerate the backup bundle to cover this continuation's commits.

## Same-day continuation 69 (real date 2026-08-03) — Pass 19.4 (`Tw`)
SHIPPED (`a1638f4`): **decision 019 / FF-H is COMPLETE end-to-end, all
five slices 19.0–19.4 shipped.** Hashes verified with `git cat-file -t`:
`77bc58e`, `a1638f4`. Branch `pass-8-redaction`, 58 commits, still no
remote. Engineer has since dispatched the GUI redaction-apply flow
(Backlog → In progress) as the next active work, concurrent with this
filing.

**Shipped:**
- Pass 19.4 — `Tw` (word spacing) direct-authoring control (core + CLI
  + GUI), committed `a1638f4`. Rides the existing `push_state_param`
  restore ladder + `pre|set|mid|restore|post` splice, no new authoring
  path; `MetricSpec` shared with `Tc`; CLI `--word-spacing V[pt|em]`
  via generalized `parse_text_metric`. Gates: `cargo test --workspace`
  1738 → 1756, 0 failed; fmt/clippy clean; `check-ui-strings.sh` exit
  0; `cargo tree` clean; zero new Cargo dependencies; R85 21/21;
  round-trip proven non-vacuous by two binaries differing in both MD5
  and size (3,396,096 vs 3,394,048 bytes). Full record: `ROADMAP.md`'s
  Pass 19.4 Shipped entry (top of Shipped).

**Decisions made this session:**
- Decision 019 **Amendment F** filed (`docs/decisions/
  019-ffh-spacing-scaling-synthetic-styles.md`), recording three
  findings Pass 19.4's build surfaced that neither the original
  decision nor Amendments A–E anticipated (full detail in Findings,
  below). `ARCHITECTURE.md` §5.11 gained a new Pass-19.4 paragraph + a
  MILESTONE note; §12 gained the matching dated decision-log entry.
  `ROADMAP.md`'s ★ Pass 19.x entry retitled COMPLETE/RETIRED; standing
  rule R91 gained a dated amendment footer; new standing rule **R96**
  filed (methodology: a guard clause behind a filter the guarded case
  cannot pass is dead code that looks live). Ceiling is now R96.

**Findings + decisions (empirical):**
- **★★★★ THE SHARPEST FINDING — a standing rule that would have
  compiled, read correctly, and NEVER FIRED.** R91's composite-run
  refusal was unreachable as `plan_format` was ordered: `Walk::
  record_show` does not decode a composite run's string, so
  `ShowData::text` is empty for every composite run, so `match_run`
  returns `NoMatch` before the font-aware gate can ever speak. Left
  alone, R91 would have shipped as code referenced in three documents
  and never once executed. Fixed by hoisting font resolution above
  `match_run`; verified by two tests — one proving the gate now fires,
  a second (`the_composite_gate_fires_only_for_word_spacing`) proving
  the OTHER three controls stay live on the same composite run, so
  this is a specific capability gate, not a blanket composite refusal.
  Neither decision 019 nor Amendments A–E anticipated this.
- **A named limit, recorded not papered over:** the fixed refusal is
  reachable through the pinned-span path (GUI, core tests) but NOT
  through CLI `--find` — composite-run text search finds nothing, so
  the CLI returns "not found in an editable run," a less specific
  refusal than the decision describes. Closing it needs composite
  decoding in the authoring walk (FF-E's scope).
- **`Tw` is multiplied by `Th`** (§9.4.4, same basis as `Tc`) —
  `--word-spacing 2 --h-scale 50` delivers a 1-unit gap, not 2.
  Decision 019 names this only as a reason `Tw` is awkward to expose,
  never as something needing disclosure; the disclosure now quotes the
  effective delivered value whenever `Th ≠ 1`.
- Two findings recorded as confirmations, not corrections: `Some(0)`
  affected spaces is emitted and disclosed as a real answer rather than
  suppressed as a no-op; and Amendment A.1's fourth restore rung needed
  no code change to correctly handle `"` setting `Tw`/`Tc` as a side
  effect of showing text — its first concrete, load-bearing test.
- Amendment E's falsification (§3.2 reason 2, the "large and growing"
  composite-default premise) held under implementation — nothing added
  this slice asserts a trend in composite adoption either direction.
- **R86 observed with `-ProcessId`** on a purpose-built simple+
  Type0/Identity-H fixture: live case dragged `Tw` to 57.0‰, applied,
  canvas visibly widened gaps, strip showed `Tw 0 -> 0.912`; refused
  case collapsed to grey read-only with the §9.3.3 explanation, no
  spinner/toggle/Apply (R83). The capture guard fired twice on uniform
  frames and the builder sent real clicks until it passed rather than
  defeating it.

**RAG escalations this continuation:**
- `D:\dev\rag\rust\dead_guard_clause_behind_a_filter_the_guarded_case_cannot_pass.md`
  — the unreachable-gate methodology finding (indexed in
  `D:\dev\rag\rust\index.md`).
- `C:\personal_rag\pdf\lesson_20260803_word_spacing_multiplied_by_horizontal_scaling.md`
  — the `Tw`×`Th` coupling finding (indexed in both
  `C:\personal_rag\pdf\index.md` and the master `C:\personal_rag\index.md`).

**Still in flight:**
- **GUI redaction-apply flow — PROMOTED TO IN PROGRESS this
  continuation**, a builder started it concurrent with this filing.
  Engineer sequencing call (flagged, not a correction): dispatched
  ahead of item #4 (form-building) in the ★★★ operator priority
  sequence, on the grounds that a half-shipped security feature
  outranks starting a new feature family — see the new "GUI
  redaction-apply flow" In-progress entry and new Open operator
  question (l) in `ROADMAP.md`.
- Carried, unchanged: `✓`/`✕` glyph verification still owed; `ⓘ`
  tofu suspicion unconfirmed; status-bar/fit-zoom feedback loop; letter
  badges pending real icons; `egui_kittest` harness gap; Open operator
  questions (h)–(k) (FF-C rule-13, FF-I StructTree cut, list-authoring,
  kerning) unanswered; FF-C and FF-B remain unscheduled per decision
  019's own Q3 build order.
- Branch still named `pass-8-redaction`, now spanning Passes 9–19.4
  plus the shipped defect fix and the in-progress redaction-apply
  flow — worth a rename whenever a push is authorized.
- No git remote configured; the backup bundle is stale and not
  regenerated this continuation.

**For next session:**
- Flag to the operator at next contact, carried forward: (1) push/
  publish call still ungranted, chain now 58 commits, still no remote;
  (2) branch-rename-on-push still pending; (3) the GUI redaction-apply
  flow is now IN PROGRESS (was: no flow at all); (4) R86 still
  unanswered; (5) the kerning parity gap unscoped; (6) Open operator
  questions (h)–(l) unanswered ((l) is new — the redaction-apply
  sequencing call); (7) FF-H is DONE — decision 019 is COMPLETE, the
  operator's priority-#3 item is done as far as FF-H's own scope goes
  (FF-C/FF-B remain).
- When the GUI redaction-apply flow ships, dispatch the librarian to
  move it to Shipped with a Pass ID (engineer assigns) and update the
  Backlog entry.
- Regenerate the backup bundle to cover this continuation's commits.

**Same-day continuation 70 (real date 2026-08-03) — Pass 8.1 SHIPPED
(`9a68999`): the GUI redaction-apply flow is done. THE HALF-SHIPPED
SECURITY FEATURE IS NOW WHOLE.** Hashes verified with `git cat-file
-t`: `24bdbc6`, `9a68999`. Branch `pass-8-redaction`, 60 commits, still
no remote. A KenAgent decision agent is concurrently writing to
`docs/decisions/` (form-building scope) — this filing did not touch
that directory.

**Shipped:**
- Pass 8.1 — GUI redaction-apply flow (mark/review/apply, all reachable
  from the running application), committed `9a68999`. Before this Pass,
  `grep -c "apply_redactions\|RedactApply" crates/pdfce-gui/src/main.rs`
  returned 0 — the GUI could mark redactions and warn the operator their
  document "is NOT redacted" but had no way to actually apply one; the
  operation was CLI-only. New `crates/pdfce-gui/src/redact_apply.rs`
  (640 lines) as a free function over `&EditSession`; new
  `DockPanel::Redact`; `Icon::Redact` (the icon set's only solid-filled
  glyph) un-reserved; core gained `RedactionMark`/`redaction_marks()`
  (the status-bar count and the panel's mark list now walk the SAME
  data) and `EditSession::delete_redaction_mark` +
  `CommandKind::DeleteRedactionMark` (refuses any non-`/Redact`
  subtype by construction — not a general `delete_annotation` back
  door). Gates: `cargo test --workspace` 1756 → 1768, 0 failed (measured
  baseline); fmt/clippy clean; `check-ui-strings.sh` clean; `cargo tree`
  clean on core/render; R85 21/21. Full record: `ROADMAP.md`'s Pass 8.1
  Shipped entry (top of Shipped).

**Decisions made this session:**
- No new architectural decision — this Pass implements decision 018's
  existing live-edit-rendering framing plus the already-decided Pass
  8.0 redaction design; no `docs/decisions/` or `ARCHITECTURE.md` §12
  entry was filed for it.
- The engineer's earlier sequencing call (dispatching this ahead of
  item #4/form-building in the ★★★ operator priority sequence) is now
  a completed fact rather than a pending flag — recorded as RESOLVED
  in `ROADMAP.md`'s In-progress and Open-operator-questions (l) entries.
  The ratification question (did the operator actually want this
  order) stays open; only the underlying work is done.

**Findings + decisions (empirical):**
- **The design decision worth recording above the feature itself:
  there is no incremental-save fallback because the code path does not
  exist to be taken — an absence, not a check that could be bypassed.**
  Engineer-verified: the only two occurrences of `to_incremental_bytes`
  in `redact_apply.rs` are comments explaining the absence; a precise
  grep for a CALL returns nothing. The librarian's own first grep of
  this claim was too coarse and appeared to contradict the builder —
  re-run precisely, the claim held. Filed as a new `D:\dev\rag\rust\`
  finding (below) because the pattern generalizes past PDF/redaction.
- **The security proof proves ABSENCE, not invisibility.**
  `applied_redaction_leaves_no_recoverable_trace_in_the_saved_bytes`
  drives the exact GUI pipeline and asserts three independent absences:
  the text extractor's output, EVERY decoded stream (page content,
  form XObjects, metadata, object-stream containers — a stale
  compressed copy would show up in any of these), and the raw file
  bytes — plus a negative control (`KEEPTHIS`) so a blank-page bug
  would fail the test, not pass it vacuously. Deliberately no raster
  assertion: a black box over live text is precisely the §12.5.6.23
  false-redaction failure mode.
- **The same proof runs at RUNTIME on the real output, before the
  confirmation dialog opens** — this is what licenses the word
  "verified" in the confirmation UI. A decoded-stream survivor refuses
  the whole apply; a raw-bytes-only survivor is disclosed as an
  acknowledgement-gated residual, worded to claim only what pdfce
  actually knows. Strings under 4 characters are excluded from the
  raw-byte check and counted in the report, not silently dropped.
- **Two defects found only by looking (R86), both fixed same commit:**
  the marks list's `max_height(240.0)` pushed "Review & Apply
  Redactions…" below the fold in a ~250 pt dock pane (reordered to
  state → action → detail; filed as new standing rule R99); and the
  confirmation report attributed the whole `annotations_removed` count
  to *overlapping* annotations when all three ARE the marks on the
  fixture (reworded to an accurate total).
- **Where the spec no longer fits current reality — six items, all
  recorded not silently deviated from:** §3.1's dedicated `SidePanel`
  is superseded by `DockPanel::Redact` (R80-compliant, not a
  violation, in the shipped build — the OLD spec text is what's now
  wrong); §3.1's icon-only button superseded by the shipped icon set;
  §4.3's permanence wording was factually WRONG (apply writes a new
  file, it does not mutate the open session); §4.3/§7 assumed a
  predicted report, but because `apply_redactions` is pure the apply
  now runs before the modal and the report states measurements (filed
  as new standing rule R98); §4.4's `could_not_remove` field does not
  exist in core — derived in one `residual_lines()` function instead;
  §3.2's `✕` glyph replaced with the word "Remove."
- **Not built — scope-called and named:** §2.2/§2.6 canvas
  drag-marking + its transient property bar (the canvas-substrate
  dependency has since landed; filed as a new Backlog follow-up,
  recommending a `CanvasTool::Redact` variant over a parallel drag
  implementation) and §6 Sanitize (unchanged, filed under the
  Redaction Backlog bucket, not yet scoped into a Pass).
- **Three new standing rules filed (R97–R99, ceiling was R96, now
  R99):** R97 (extract security-critical logic to a free function over
  data so the proof can be a test, not an inspection); R98 (a
  confirmation dialog for a pure destructive operation should compute
  and disclose the REAL outcome before confirming, not a prediction);
  R99 (in a bounded dock pane, a panel's primary action must precede
  its detail list). All three librarian-assigned, no decision number.

**RAG escalations this continuation:**
- `C:\personal_rag\pdf\lesson_20260801_redaction_absence_proof_acceptance_gate.md`
  — AMENDED with a dated 2026-08-03 footer recording the GUI-runtime
  extension (form-XObject/metadata/object-stream-container coverage,
  the pre-confirm-modal timing, the acknowledgement-gated residual
  disclosure wording, the <4-char count-not-skip rule). Not a new
  file — this is the same absence-proof methodology, extended, not a
  distinct finding; the existing lesson's index entries needed no
  change.
- `D:\dev\rag\rust\a_removed_code_path_is_a_stronger_guarantee_than_a_guarded_one.md`
  — new file: the "no incremental-save fallback because the call does
  not exist, not because it's guarded against" pattern, verifiable by
  grep. Indexed in `D:\dev\rag\rust\index.md` this continuation.

**Still in flight:**
- **Nothing is currently in progress.** Next per the ★★★ operator
  priority sequence is item #4 (form-building tools) — Acrobat-parity
  research for field CREATION/authoring is already done (5 new
  `forms__*.md` files + 3 dated addenda,
  `D:\Dev\Rag-Specialized\Acrobat_Features\`), and a KenAgent decision
  agent is concurrently scoping it in `docs/decisions/` as of this
  filing.
- **Headline research finding for whoever picks up form-building:**
  field-name collision is type-branched (same-type merges into
  `/Kids`, different-type refuses by name) — recommend `pdfce-core`'s
  field model be a `/Kids` object graph from day one, not a flat
  name-keyed list. Two unreconciled conflicts flagged, not guessed:
  Combine-Files auto-rename vs. link-by-default on merge; the
  encrypted-document field-creation permission workflow.
- **XFA scope needs an operator call, narrower than before.** Dynamic
  XFA has no AcroForm at all as of Acrobat 8.1+ (clean
  `out_of_scope`), but static-XFA-hybrid new-field-creation
  permissibility is an unresolved GAP, and Acrobat's exact
  deprecation-date-by-version remains unsourced. `CLAUDE.md`'s
  standing open item on XFA relevance is narrowed, not closed, by this
  session — see `ROADMAP.md`'s amended XFA Backlog entry.
- Carried, unchanged: `✓`/`✕` glyph verification still owed; `ⓘ` tofu
  suspicion unconfirmed; status-bar/fit-zoom feedback loop; letter
  badges pending real icons; `egui_kittest` harness gap; Open operator
  questions (h)–(k) unanswered; FF-C and FF-B remain unscheduled per
  decision 019's own Q3 build order.
- Branch still named `pass-8-redaction`, now spanning Passes 9 through
  19.4 plus the shipped `/Contents`-defect fix and Pass 8.1 — still
  worth a rename whenever a push is authorized.
- No git remote configured; the backup bundle is stale and not
  regenerated this continuation.

**For next session:**
- Flag to the operator at next contact, carried forward: (1) push/
  publish call still ungranted, chain now 60 commits, still no remote;
  (2) branch-rename-on-push still pending; (3) the GUI redaction-apply
  flow is now SHIPPED (Pass 8.1, `9a68999`) — the app can mark AND
  apply redactions end-to-end from the running GUI; (4) R86 still
  unanswered; (5) the kerning parity gap unscoped; (6) Open operator
  questions (h)–(l) unanswered — (l) is now "the sequencing call was
  right in outcome, still unratified in principle"; (7) form-building
  research is DONE and a KenAgent decision agent is actively scoping
  it — the next Pass in this family is likely close behind.
- When the form-building decision lands and a Pass is scoped, dispatch
  the librarian for "roadmap update — new request" to file the Pass
  ID(s) under Next up.
- Regenerate the backup bundle to cover this continuation's 2 new
  commits (`24bdbc6`, `9a68999`) plus whatever the concurrent
  form-building decision session adds.

**Same-day continuation 71 (real date 2026-08-03) — Pass 18.7
(glyph-coverage gate + tofu fixes) SHIPPED (`09be28d`); decision 020
(form field AUTHORING) filed, SCOPED, NOT STARTED (`d9960cd`);
R100–R105 filed (renumbered from a collision with continuation 70's
R97–R99); backup-bundle staleness flag CLOSED.** Hashes verified with
`git cat-file -t`: `09be28d`, `d9960cd`. Branch `pass-8-redaction`, 62
commits, still no remote.

**PASS-NUMBER CORRECTION (filed by `pdfce-librarian`, same real date,
a dispatch separate from and after the work below):** `09be28d`'s own
commit subject line reads *"Pass 19.4: glyph-coverage gate; fix tofu
Accept/Reject, info markers, arrows"* — that number was already taken,
by `a1638f4` (`Tw`, decision 019/FF-H, filed continuation 69). This
entry originally filed the work below with no Pass ID at all, which
was also wrong — the work has clear acceptance criteria and belongs in
the numbered ledger. **Corrected to Pass 18.7** (next free slot in the
18.x UI-quality line, not the 19.x decision-019 family) by
`pdfce-librarian`, recorded on the branch by a dedicated empty
correction commit, `1111652` ("docs: correct Pass number on `09be28d`
— glyph-coverage gate is 18.7, not 19.4"), verified via `git cat-file
-t` alongside the four hashes already named. **This is the THIRD
documented instance of this exact collision class on this project**
(Pass 18.4's commit called itself "Pass 18.2"; decision 014's design
proposed "Pass 13.x", already taken) **and lands the same real day as
the R97–R105 standing-rule renumbering below — the same underlying gap
(no automated uniqueness check on any of this project's numbered
ledgers) surfacing twice in one session.** New standing rule **R106**
filed in `ROADMAP.md` (ceiling was R105, now R106) recording the
methodology fix: read the live ceiling immediately before assigning
any Pass/rule/decision number, and treat a number proposed by
concurrently-drafted work as provisional until the librarian confirms
it against `ROADMAP.md` at filing time. Full record: `ROADMAP.md`'s
Pass 18.7 Shipped entry's own Pass-number note (top of Shipped).

**Shipped:**
- Pass 18.7 — Glyph-coverage gate + tofu-glyph fixes, committed
  `09be28d` (subject line self-reports "Pass 19.4" in error — see the
  Pass-number correction above; corrected to 18.7). A headless test
  (`scan_string_literal_chars`) reads `ui_text.rs`'s own bytes and
  checks every operator-visible character against the font stack the
  app actually runs on; found twelve broken glyphs across three
  already-shipped features (in-place text edit, reflow, add-text) —
  U+2713/U+2715 and U+24D8 rendered as tofu on every single edit, not
  once per session. Full record: `ROADMAP.md`'s Pass 18.7 Shipped
  entry (top of Shipped, above Pass 8.1).

**Decisions made this session:**
- **Decision 020 — form field AUTHORING** filed by a KenAgent decision
  agent, archived at `docs/decisions/020-form-field-authoring.md`.
  Status: DECIDED (six sub-questions answered), SCOPED (Pass
  20.0–20.7 assigned and verified free), **NOT STARTED** — this
  filing does not authorize building Pass 20.0. Full record:
  `ARCHITECTURE.md` §12's continuation-71 entry; `ROADMAP.md`'s
  amended Forms/AcroForm Backlog entry.
- **Standing rules R100–R105 filed**, all six from decision 020 §5,
  **renumbered from the decision document's own original R97–R102**
  because continuation 70 had already claimed R97–R99 for three
  unrelated Pass 8.1 findings by the time both filings landed
  concurrently. Renumbering was applied to BOTH the decision document
  (prose + Appendix A JSON, with a machine-readable mapping added
  there) and `ROADMAP.md`'s Standing rules, so the two records agree —
  no rule's substance changed, only its number. This is the fourth
  filing-integrity issue this project's dated-artifact discipline has
  caught (see R87's own note for the first three; this one is a
  concurrency collision rather than an arithmetic error, a new failure
  shape for the same underlying discipline).

**Findings + decisions (empirical):**
- **The methodology finding worth escalating past this session:
  `epaint::Fonts::has_glyph` is not a glyph-coverage oracle — it
  compares the resolved face against the REPLACEMENT face, and the
  replacement face is itself chosen by searching the font chain for a
  glyph (`◻` U+25FB) that the primary face (Ubuntu-Light) lacks, so the
  replacement face ends up being an emoji face.** Every symbol sharing
  that face reads as "missing" whether it renders or not — 15 false
  positives observed, including U+26A0 and U+2714, both demonstrably
  painted on screen today. Without a positive control (checking the
  oracle against characters already known to render) this would have
  produced twelve real fixes and three imaginary ones, on a green
  test's authority. Fixed oracle: `Font::glyph_width(c, size) > 0.0`.
- **A second defect found only by looking (R86), not by the new gate:**
  `add_sized(ICON_BUTTON_SIZE, ..)` is a layout CAP, not an
  accessibility FLOOR — egui wraps a label one character per line once
  it overflows the fixed size. "Place point" rendered as four stacked
  fragments; two of six affected buttons were "Accept reflow"/"Reject
  reflow." Fixed via `.min_size(ICON_BUTTON_SIZE)`. Stated as the
  session's crispest generalization: a test can prove a character has
  a glyph; only looking proves the operator can read the button.
  Considered for a new standing rule, declined — it sharpens R86's
  existing rationale rather than adding new behavior.
- **`snap_glyph()` DELETED, not repaired** — zero call sites, carried a
  now-disproven `#[allow(dead_code, reason = "drawn by the ... measure
  tools' overlay")]`; 7 of its 8 marks were uncovered by the corrected
  oracle. R93's exact shape (a comment vouching for behavior nobody
  checked), caught before anyone wired the function up rather than
  after.
- **Decision 020's headline finding: decision 009's byte-verbatim
  JS-carrier guarantee does not survive field creation, and no
  existing test will notice when it stops holding.** Fill never writes
  `/AcroForm`, so `/CO`/`/AA`/`/Names /JavaScript` re-emit verbatim —
  structurally, not by assertion. Field creation (Pass 20.1/F1) must
  write `/AcroForm/Fields`, which breaks the guarantee's structural
  basis silently. A forward-pointer note was added to the decision-009
  entry in `ARCHITECTURE.md` §12 itself (not only to decision 020's own
  entry), per this continuation's explicit instruction, because
  decision 020 changes what an ALREADY-SHIPPED guarantee will mean
  once F1 lands.
- **Decision 020's data-model correction to its own prior Backlog
  recommendation:** the shipped flat `AcroForm.fields: Vec<Field>` read
  projection is correct and unchanged — `Field.widgets: Vec<Widget>`
  already carries the one-to-many that matters. What was missing is a
  write-side-only graph resolver (`resolve_field_path`), not a rewrite
  of the fuzz-tested read path. The "build it as a `/Kids` graph from
  day one" line filed in this bucket back in continuation 50 is now
  superseded framing, retained for history per append-only discipline.
- **Four ways, not two, in the field-creation collision branch** — a
  fourth outcome (`Grouping`, refuse `NameIsGroupingNode`) exists
  because pdfce authors dotted hierarchy and Acrobat/the parity
  research never encounter it (neither exposes hierarchy authoring). A
  non-terminal field has no type (Table 220), so it collides with
  neither the same-type-merge nor different-type-refuse branches.
- **XFA: the standing hybrid-authoring GAP is now DECIDED, not
  resolved empirically** — static-XFA-hybrid field creation is refused
  by name, from pdfce's own capability boundary (it can write half a
  hybrid form, not the other half, and a one-sided write makes two
  viewers show two different field counts for one document). The
  standing "verify XFA deprecation status" open item is narrowed, not
  closed — filed as new Open operator question (p): retire it, or
  re-scope to "before any XFA read/fill work."

**RAG escalations this continuation (filenames corrected by
`pdfce-librarian`'s same-day Pass-number-correction dispatch — this
entry originally cited the first two files under different names,
before any file existed at those paths; the names below are the
canonical, actually-written ones):**
- `D:\dev\rag\egui\epaint_035_has_glyph_false_positive_via_replacement_face_fallback.md`
  — new file, the `has_glyph` false-positive finding + fix
  (`glyph_width`) + the concrete false-positive set.
- `D:\dev\rag\egui\egui_add_sized_is_a_layout_cap_not_an_accessibility_floor.md`
  — new file, the `add_sized` per-character-wrap finding + fix
  (`.min_size`).
- `D:\dev\rag\rust\ci_gate_red_at_baseline_enforces_nothing.md` —
  AMENDED with a dated 2026-08-03 footer generalizing point 2
  ("verify a gate's negative case") to also cover an ORACLE's
  specificity, not only a gate's sensitivity — cross-referencing the
  `has_glyph` finding as a second, independent instance of "verify the
  checker against a known-good case too." Not a new file — the
  existing lesson already stated the negative-case half of this
  principle; this extends it rather than duplicating it.
- `D:\dev\rag\egui\index.md` and `D:\dev\rag\rust\index.md` updated
  with the new/amended entries.
- Nothing filed to `C:\personal_rag\pdf\` this continuation — both
  findings are egui/rendering-domain, not PDF-domain.

**Still in flight:**
- **Nothing is currently in progress; Pass 20.0 (F0) is scoped but not
  authorized to start** — Open operator question (m) (should item #4
  start at all, given FF-C/FF-B are still open) is the gating
  question, and this continuation's default is explicitly NOT to
  start, unlike most other open items' "default to the stated
  fallback."
- Four more decision-020 items filed for the operator, none blocking:
  (n) pull "signature field for someone else to sign" into F3? (o)
  confirm the barcode-field parity subtraction; (p) XFA open-item
  retire/re-scope; (q) CLI-surface migration question, flagged as
  librarian/engineer territory, not Ken's.
- F4 (tab order) is pre-blocked on a `pdfce-spec-librarian` dispatch
  for Table 30's `/Tabs` row, §14.7 structure-order derivation, and
  the ISO 32000-2 `/Tabs` delta — verified absent from the spec RAG
  this session, named as a hard prerequisite in decision 020 itself.
- F5 (GUI) is pre-flagged as requiring a `pdfce-ui-specialist`
  dispatch before building, per standing rule (non-trivial UI).
- Carried, unchanged: `✓`/`✕` glyph verification — **now closed by
  this continuation's own Shipped entry**, remove from future
  "carried forward" lists; `ⓘ` tofu suspicion — **also now closed,
  same entry**; status-bar/fit-zoom feedback loop; letter badges
  pending real icons; `egui_kittest` harness gap; Open operator
  questions (h)–(k) unanswered; FF-C and FF-B remain unscheduled per
  decision 019's own Q3 build order — and are now also the gating
  fact behind new Open operator question (m).
- Branch still named `pass-8-redaction`, now spanning Passes 9 through
  19.4 plus Pass 18.7 and decision 020's filing — still worth a rename
  whenever a push is authorized.
- **Backup-bundle staleness flag CLOSED this continuation** (was open
  since continuation 70): refreshed to
  `D:\Dev\pdfce-backups\pdfce-20260803-1936.bundle`,
  `git bundle verify` reports "records a complete history," current to
  `d9960cd`. Will drift stale again with the next commit, per the
  standing pattern.

**For next session:**
- Flag to the operator at next contact, carried forward: (1) push/
  publish call still ungranted, chain now 62 commits, still no remote;
  (2) branch-rename-on-push still pending; (3) the twelve-glyph tofu
  defect is fixed and the two ui-spec items it was blocking are
  resolved; (4) R86 still unanswered (open question (e)); (5) the
  kerning parity gap unscoped (open question (k)); (6) decision 020 is
  DECIDED and SCOPED but explicitly NOT authorized to start — the
  operator's answer to open question (m) is the actual gate on Pass
  20.0, not this filing; (7) four smaller decision-020 questions (n)
  through (q) await an answer but do not block anything.
- When/if the operator answers (m) affirmatively, dispatch
  `pdfce-spec-librarian` for F4's prerequisite (Table 30 `/Tabs` +
  §14.7 + the ISO 32000-2 delta) in parallel with starting Pass 20.0
  (F0), since F0 does not depend on it.
- Regenerate the backup bundle again once further commits land — the
  one made this continuation (`...1936.bundle`) is already
  point-in-time as of `d9960cd`.

**Same-day continuation 72 (real date 2026-08-03) — `docs/ROADMAP.md`'s
duplicated Pass 4 heading fixed; the R106-flagged uniqueness checker
now exists and ships GREEN; FF-C's rule-13 licensing sub-question
CLEARED without an operator decision (no code changes this
continuation).**

**Fixed:**
- `docs/ROADMAP.md` — the `### Pass 4 — Text extraction / structured
  content` heading under `## Next up` was declared TWICE, back to
  back, with nothing between the two headings (an unknown-age
  accidental double-paste, not caught by anything before now). Removed
  the redundant heading, kept the body attached to the single
  remaining one. Verified no other section in `ROADMAP.md` has a
  same-section duplicate Pass ID by walking all 60 `### Pass ` /
  `#### Pass ` headings against the file's four top-level sections by
  hand (Shipped: 51 IDs, all distinct; In progress: none; Next up: 9
  IDs, all distinct post-fix; Backlog: none) — this librarian has no
  shell-execution tool this session, so `tools/check-ledger-numbers.py`
  itself was read and its logic reproduced manually rather than run;
  the engineer or a future session should still run
  `python tools/check-ledger-numbers.py --stats` directly for the
  authoritative exit code before treating this as fully closed.

**Filed:**
- `tools/check-ledger-numbers.py` (shipped at `4dc8cf8`, this real
  date) recorded against **R106** with a prose amendment appended
  directly under R106's existing text in `ROADMAP.md`'s Standing
  rules — deliberately NOT written as a new `- **R106 —` bullet, since
  the checker's own R26 allowlist note warns that shape reads as a
  competing rule definition unless allowlisted. The amendment records:
  per-section (not global) Pass-ID uniqueness, since a Pass legitimately
  appears twice — once planned, once Shipped — and nine such pairs
  exist today; the em-dash-prefix-only parse rule for Pass headings (a
  heading's descriptive half routinely names OTHER Passes); the R26
  amendment-shape allowlist trap; the `--stats` under-parse catch (5 of
  106 rules, `R53`–`R57`, use a `(was R-JS-1)`-style parenthetical the
  first pattern didn't allow for); and that this was the FIFTH
  numbering collision found the same real day, this one a duplicated
  heading rather than a proposed-number collision.
- `docs/ROADMAP.md`'s FF-D-fast-follow-FF-C Backlog bullet and Open
  operator question (h) both amended: FF-C's rule-13 dependency
  classification is **DONE and CLEARS without an operator decision** —
  `subsetter 0.2.6` (Typst) is `MIT OR Apache-2.0` with an all-permissive
  transitive graph (`cargo metadata` on a scratch crate, not crates.io
  pages or memory), `LEGAL.md` §6.2 step 3 applies (proceed and log,
  same disposition `egui_tiles` got). Net cost 2 new packages
  (`subsetter`, `write-fonts 0.48.1`); `write-fonts` resolves via
  `subsetter` to `read-fonts 0.39.2`/`font-types 0.11.3`, matching
  `pdfce-render`'s existing load-bearing `skrifa 0.42` pin by
  construction — a bare `cargo add write-fonts` would instead select
  0.51.0 and split the graph across two incompatible font-parser
  versions. Full record: `PRIOR_ART.md`'s new "FF-C dependency
  classification (rule 13) — COMPLETE, 2026-08-03" subsection under
  Fonts (filed by the engineer at `d738950`, prior to this dispatch —
  this continuation propagates that finding into `ROADMAP.md`'s two
  outward-facing pointers). **What remains for FF-C is scope/sequencing
  only (Q3 of ★ Pass 19.x: FF-H → FF-C → FF-B), not licensing.** A
  KenAgent decision agent is concurrently scoping FF-C into a Pass
  family (will land as decision 021, writing to `docs/decisions/` —
  this continuation did not touch that directory).
- `CLAUDE.md`'s XFA bullet — engineer-authored at `d738950`, prior to
  this dispatch, correctly reported as out of sync but outside this
  role's tiers; flag discharged, no librarian action needed (see
  `SESSION_LOG.md` continuation 71's XFA finding and the corresponding
  ROADMAP Open operator question (p) for the underlying content).

**RAG escalations this continuation:**
- `D:\dev\rag\rust\ci_gate_red_at_baseline_enforces_nothing.md` —
  AMENDED (second amendment this real date) with a new section: a
  hand-maintained numbered ledger (Pass IDs, standing-rule numbers,
  decision numbers) is a primary key with no integrity constraint; no
  existing lint/test/CI job has any concept of it; the fix that
  actually worked was a checker that PRINTS THE LIVE CEILING before a
  number gets assigned (prevention), not a duplicate-detector alone
  (detection-only fires after the wrong number is already committed).
  Generalizes past pdfce to any hand-maintained ID scheme (ticket
  prefixes, RFC/ADR numbers, migration sequence numbers). Tags
  extended with `id-collision`, `numbering`, `primary-key`.
  `D:\dev\rag\rust\index.md` updated with a summary of both amendments
  now on this file.

**Still in flight:**
- Unchanged from continuation 71: Pass 20.0 (F0) scoped, not
  authorized to start (Open operator question (m) is the gate); FF-B
  and (now) FF-C await only scope/sequencing, not a blocking decision;
  branch `pass-8-redaction` still unpushed.

**For next session:**
- Run `python tools/check-ledger-numbers.py --stats` for the
  authoritative confirmation this continuation's manual check stands
  in for (this librarian dispatch had no shell-execution tool
  available). Expected: exit 0, "ledger-numbers: clean — no duplicate
  Pass, rule, or decision numbers," ceilings printed as Pass family 19
  (highest ID 19.4), rules R106 → next free R107, decisions 020 → next
  free 021.
- FF-C's remaining open question is purely "when does it get scoped
  into a Pass," not "is it licensed cleanly" — don't re-open the
  licensing question when FF-C's turn comes.

**Same-day continuation 73 (real date 2026-08-03) — decision 021 filed
and confirmed (FF-C: font subsetting/glyph embedding, DECIDED/SCOPED/
NOT STARTED); the ledger-checker's mentioned-but-unheaded blind spot
folded into R106; continuation 72's manual-check caveat DISCHARGED
(the engineer ran `tools/check-ledger-numbers.py --stats` directly:
GREEN, exit 0, 61 (section, Pass ID) pairs / 106 rules / 21 decisions /
1 allowlisted amendment).**

**Filed:**
- `docs/decisions/021-ffc-font-subsetting-and-glyph-embedding.md`
  (committed `d30842c`, alongside the ledger-checker's companion fix).
  **Confirmed the engineer's own numbering correction rather than
  re-deriving it**: the scoping session's original draft claimed rule
  ceiling R99 and Pass 20.x free, both stale by the time it was
  written (three librarian filings had landed the same day); the
  engineer caught both before filing and corrected to R107–R110 and
  Pass 21.x, declining the record's own §10.3 recommendation to
  renumber decision 020 to 21.x (premised on 20.x being unclaimed — it
  is not). Verified against the live ledger: R106 was indeed the
  ceiling, Pass 20.x is indeed claimed (decision 020's Backlog prose,
  unheaded) — the correction stands as filed.
- **Standing rules R107–R110 added** to `ROADMAP.md` (ceiling now
  R110, was R106): R107 (FF-C add-only, never rewrites an existing
  font program/dictionary — object-id-disjointness TEST, not a runtime
  guard, per R96); R108 (embedding is an explicit per-action operator
  choice, real computed subset size/coverage shown, R98 applied, never
  a default); R109 (font-embedding permission read from the donor's
  `OS/2` `fsType` before subsetting and disclosed, never assumed —
  policy itself is new Open operator question (r)); R110 (a composite
  run is editable only where `/ToUnicode` is VERIFIED injective, per
  font per session — `Identity-H` with no `/ToUnicode` stays a
  permanent hard skip, R65 untouched).
- **★ Pass 21.x filed under `ROADMAP.md` Next up** — 21.0 (core+CLI,
  P0 floor, lifts the widest wall: pdfce today cannot add ANY text
  outside WinAnsi/Symbol/ZapfDingbats) → 21.1 (composite-run edit,
  makes 21.0's output editable — explicitly flagged: shipping 21.0
  without 21.1 and calling FF-C done would ship a capability
  regression against the already-shipped Std-14 add-text path, and
  every existing gate including the R85 raster oracle would report
  success while missing it — the `flatten_fields` failure shape) →
  21.2 (`set-font` to an embedded face; also makes the shipped
  `format_coverage_hint()` GUI text honest for the first time — it
  currently promises a remedy the write path does not deliver, a
  current honesty gap independent of when FF-C lands) → 21.3 (GUI,
  `pdfce-ui-specialist` dispatched first).
- **Amendments filed**: `ROADMAP.md` R21 (scope note — the write-side
  `subsetter` internal reader is admitted, discharging R21's own
  escape clause; `cargo tree --duplicates` guard unchanged), R71
  (FF-C ceases to be "a deferred writer subsystem," trust ladder gains
  a fourth rung), R79 ("no embedding" → "no embedding **by default**");
  the FF-D-fast-follow-FF-C Backlog bullet (decision 021 pointer +
  headline correction); two new Open operator questions (r) font-EULA
  policy and (s) complex-script refusal-by-name, both flagged as
  Ken's per `docs/decisions/README.md`, neither blocking on
  21.0/21.1; `docs/decisions/012-operator-supplied-fonts.md` §6
  ("the write side — unrelated" corrected — FF-C is the write-side
  consumer of decision 012's `--font-dir` supply mechanism);
  `PRIOR_ART.md`'s "FF-C dependency classification" section (net cost
  refines from 2 packages to **1** at `subsetter`'s
  `default-features = false` — `variable-fonts` was the only thing
  pulling in `write-fonts`/`kurbo`, unneeded at P0; the 2-package
  figure stays on record as the naive-`cargo add` cost);
  `ARCHITECTURE.md` §12 dated entry (decision 020's "no body-section
  update, nothing shipped yet" disposition applied identically here).
- **`ROADMAP.md` R106 amendment (second)** — folded the ledger
  checker's own blind spot into R106 rather than filing a new rule,
  since it is the same subject. The checker's ceiling report had
  scanned only `### Pass N` headings; decision 020 claims Pass
  20.0–20.7 in Backlog *prose* with no heading yet, so the checker
  reported "highest Pass family: 19" (true, useless) and, independently,
  this decision's own scoping session made the identical mistake
  reading the same heading-only view. **Generalized as recorded in
  R106's amendment: a ceiling computed only from completed/finished
  records under-reports, and does so specifically in the direction
  that causes collisions**, because the things most likely to collide
  with a fresh proposal are exactly the other fresh, not-yet-finished
  proposals a finished-only view excludes. The fix (already shipped by
  the engineer at `d30842c`, confirmed present in
  `tools/check-ledger-numbers.py`): scan every `Pass N` mention, not
  only headings, and name claimed-but-unheaded families explicitly as
  `CLAIMED BUT NOT YET HEADED`.
- Repo commit-count note updated: **66 commits, still no remote**
  (was 62 at continuation 71). Both figures are dispatching-engineer-
  reported, not librarian-verified — this librarian dispatch again had
  no shell-execution tool available. The engineer reports six hashes
  spot-verified on their side with `git cat-file -t` (`d30842c`,
  `4dc8cf8`, `d738950`, `1111652`, `d9960cd`, `09be28d` — all confirmed
  `commit` objects), closing continuation 72's open verification gap
  for `4dc8cf8`/`d738950` (previously engineer-reported only). Flagged
  that per-commit hash tracking is no longer exhaustive past
  continuation 62's count; use `git rev-list --count HEAD` for the
  live figure going forward rather than trusting a filed number.

**RAG escalations this continuation:**
- `D:\dev\rag\rust\ci_gate_red_at_baseline_enforces_nothing.md` —
  AMENDED (third amendment) with the generalized ceiling-under-reporting
  finding above, applicable beyond Pass IDs to any "is this name/slot
  taken" check (branch names, ticket numbers, feature-flag names,
  migration sequence numbers): count CLAIMS, not completions, and name
  the claimed-but-unfinished ones explicitly rather than folding them
  silently into "not taken." `D:\dev\rag\rust\index.md` updated with a
  one-line summary of the third amendment.

**Still in flight:**
- ★ Pass 21.x is filed and scoped but **NOT STARTED** — 21.0 is next
  when the engineer picks it up. `pdfce-spec-librarian` dispatch (spec
  RAG stub rewrite) is owed BEFORE any 21.0 code — the current stub
  actively misdescribes the mechanism.
- Two new operator questions (r)/(s) are non-blocking for 21.0/21.1
  specifically but will gate 21.0's own scope if Ken answers (s) before
  the engineer starts (refuse-complex-scripts-by-name is the
  recommendation, not yet confirmed).
- Branch `pass-8-redaction` still unpushed, 66 commits.

**For next session:**
- Before writing any Pass 21.0 code, dispatch `pdfce-spec-librarian` to
  rewrite `font__subsetting_ffc_queue.md` — it currently describes the
  wrong mechanism ("add outline to the document's existing `glyf`")
  and would actively mislead an implementer.
- The object-id-disjointness test (R107's enforcement) is the single
  highest-leverage thing to write FIRST in 21.0, per the decision
  document's own engineer handoff note — write it while the emitter is
  trivially correct, before the 21.2 "just widen the existing font"
  temptation exists.
- Surface (r) and (s) to Ken at the next natural check-in; neither
  blocks starting 21.0/21.1, but (s) shapes what 21.0's own "L1"
  headline is honestly allowed to claim.

**Same-day continuation 74 (real date 2026-08-03) — `pdfce-spec-librarian`'s
decision-021 dispatch returned: eight findings, two change the work.
Decision 021 AMENDED (§10), `ROADMAP.md` and `ARCHITECTURE.md` §12
updated to match. Two shipped operator-facing hints found FALSE and
corrected (`0893191`). One RAG-escalation item DECLINED and redirected —
it was spec-librarian's territory, not this librarian's, despite being
asked for directly.**

**Filed:**
- `docs/decisions/021-ffc-font-subsetting-and-glyph-embedding.md` — new
  "## 10. Spec review (2026-08-03)" section with all eight findings
  (C-1 through C-8); pointer notes added at §3.4, §3.6 item 2, and the
  refusal table in §3.1 so a reader hits the correction before the
  now-superseded claims; §4.2's dispatch table and §4.1's R109 bullet
  corrected/amended in place (citations fixed, not merely annotated,
  per the explicit instruction that a wrong pointer must not survive
  for someone to re-derive from).
- **C-3 (CHANGES THE WORK, scope call made this continuation):** Pass
  21.0's P0 floor is restricted to `glyf` (TrueType-outline) donors;
  CFF donors are refused by name (`DonorUnsupported`, extending the
  CFF2 diagnostic already named in the decision) until a later slice.
  Cause: `subsetter` wraps CFF donors in an `OTTO` sfnt (`lib.rs:492`,
  `FontFlavor::Cff => 0x4F54544F`), and ISO 32000-1 §9.9 Table 126
  requires `cmap` for CFF-outline `OpenType` programs — which
  `subsetter` strips unconditionally, and which `/CIDFontType0C`
  (the bare-CFF alternative) forbids wrapping in an OTTO container
  either way. No conformant emission path exists for CFF donors under
  the plan as filed. Recorded as a narrowing amendment to decision
  021 §3.4, not a new decision record — L1 (the headline non-Latin
  capability) survives intact because Noto Sans JP/CJK, DejaVu, and
  most Google Fonts are TrueType `glyf`; flagged to Ken as a narrowing,
  not a silent cut.
- **R109 amended** (`ROADMAP.md` Standing rules + decision 021 §4.1):
  fsType is two distinct refusals, not one `EmbeddingNotPermitted`.
  Bit 8 (`0x0100`, No subsetting) forbids the one thing FF-C does while
  still permitting whole-face embedding — `SubsettingNotPermitted`.
  Bit 9 (`0x0200`, Bitmap embedding only) is the spec's own
  "unembeddable" case — `EmbeddingNotPermitted`. Full bit table now
  sourced: `0x000F` usage sub-field valid values 0/2/4/8, bit 0
  permanently reserved, `0x00F0`/`0xFC00` reserved, bits 8–9 MUST be
  ignored on `OS/2` v0/v1.
- **Open operator question (r) narrowed** (`ROADMAP.md`): the
  forbids-embedding/forbids-subsetting cases are no longer Ken's call —
  spec-sourced, R109 refuses them by name. What remains open is
  strictly narrower: absent/unparseable `OS/2`, and the spec-silent
  `fsType == 1`. **The asymmetry that makes this a real trap, not a
  formality:** `fsType == 0` is *Installable*, the MOST permissive
  value — so "absent" cannot be modelled as `0` without recreating the
  exact silent-"permitted" failure R109 exists to forbid. Also recorded
  as a permanent finding: the fsType↔PDF bridge exists in **neither**
  specification (ISO 32000-1 names no such field; OpenType never
  mentions PDF) — which is precisely why this stays an operator call
  rather than a lookup.
- **Two favourable corrections recorded, not just the unfavourable
  ones** (C-1, C-2/C-6) — the decision as filed *understated* its own
  case: the emitted-table list omitted `HHEA`/`CVT`/`FPGM`/`PREP`,
  which §9.9 requires when present and `subsetter` does emit; and
  `cmap` removal plus the `/Type0`+`Identity-H` choice are `shall`s in
  §9.9, not merely inherited crate behavior — M2 is spec-directed, not
  crate-forced. Recording favourable findings alongside unfavourable
  ones is deliberate: reversing a correct-but-unsourced claim later
  would be worse than recording why it held.
- **Two citation fixes, applied verbatim** (C-4, C-5): `/CIDSet` is
  §9.8.3 Table 124, not §9.7.4.2; the subset-tag prefix rule is §9.6.4,
  not §9.8.1 (which has no subset rule). Fixed in place in decision
  021 §4.2's dispatch table.
- **C-8, flagged not decided:** ISO 32000-1 §9.9's opening paragraph —
  embedded font programs *"shall be used only to view and print the
  document"* absent contrary information, and new text needs *"a
  licensed copy of the font program, not a copy extracted from the PDF
  file"* — means an existing document's `/FontFile*` is not an
  admissible FF-C donor, independent of the "the bytes don't exist"
  reason already on record. Modality checked: the producer-side
  sentence is a `should`, not a `shall` — recorded as NOT a blanket
  embedding prohibition, to avoid overstating it. Filed as a candidate
  for standing-rule status (donor provenance); not assigned a number
  this continuation — that call belongs to the engineer/operator, not
  solo to this librarian.
- `ARCHITECTURE.md` §12 — new dated entry (2026-08-03, same day, after
  the original decision-021 entry) recording all of the above as a
  correction with a forward pointer, per the section's own
  append-only-with-forward-pointer discipline; the original entry is
  **not retracted**, it stands as the record of what was decided before
  the spec dispatch returned.
- `ROADMAP.md` — Pass 21.x entry gets a "SPEC-REVIEW AMENDMENT
  (2026-08-03)" block ahead of the Slices list; the 21.0 slice bullet
  now states the glyf-only restriction explicitly; "Honest limits"
  updated (CFF donors, not just CFF2, unsupported at P0); Standing
  rules R109 amended in place with a dated note; Open operator
  question (r) rewritten to the narrowed scope.

**Two shipped hints found FALSE, fixed (`0893191`) — filed by the
engineer, recorded here:**
- `r_inv_1_hint()` and `format_coverage_hint()` both told the operator
  that supplying a font via Tools › Font folders would lift a
  coverage/subset refusal. **False in every shipped build** — verified:
  `format.rs`'s check reads only `target.glyph_names()` and
  `carried_codes(recs, &resource)`; `addtext.rs:157` states pdfce
  *"writes an identical named non-embedded dict either way"*; and
  `pdfce-core` has no functional awareness of `FontEnvironment` (one
  crate-wide mention, a doc comment about display trust level only).
  An operator following the hint would install a font, watch the
  preview genuinely improve, retry the save, and be refused again with
  the identical message — a rule-4 failure of the quiet kind: not a
  wrong result, a wrong instruction, which makes the operator doubt
  themselves rather than the tool.
- **How it surfaced, worth keeping:** not testing — decision 021 had
  to enumerate which refusals FF-C lifts, which meant reading each
  refusal's message beside the code that raises it, and
  `format_target_missing_hint()`, six lines away, was already honest
  about FF-C. One hint naming a real limit next to two denying it was
  the tell. **Recorded as an observation for the engineer to judge as a
  possible standing-rule candidate ("scoping a feature is an audit of
  the copy around the refusals it touches") — not assigned a rule
  number by this librarian**, since it wasn't clearly generalizable
  enough to file to any cross-project RAG tier (not Rust/egui-
  ecosystem, not PDF-domain-empirical) and rule-adoption isn't this
  librarian's call to make solo.
- **Not observed on screen, stated honestly:** triggering either
  refusal needs a fixture with an embedded *subset* font plus a
  character outside it, and none exists (`fixtures/synthetic` has
  three files, none suitable). Verified instead: glyph gate clean,
  call sites untouched, `check-ui-strings` clean, 1770 tests green.
  **Filed as an owed fixture against Pass 21.0** (`ROADMAP.md`'s 21.0
  slice bullet, above) — it is a prerequisite for testing 21.0 and the
  right moment to finally observe these two hints on screen.

**RAG escalation DECLINED this continuation — redirect, not a write:**
- The dispatching message asked this librarian to write "ISO 32000-1
  §9.9 forbids using a font program extracted from a PDF as the source
  for newly authored text" (plus the fsType absent-`OS/2` asymmetry)
  to `C:\personal_rag\pdf\`. **Declined.** Per this agent's own hard
  rule 6: a finding that is "the canonical spec says X" belongs to
  `pdfce-spec-librarian`'s `D:\Dev\Rag-Specialized\PDF_Spec\`, not to
  `personal_rag/pdf`, which is scoped to empirical real-world-PDF
  divergence from spec. Both halves of the requested lesson are pure
  spec citation with no empirical "what we observed a real file/tool
  actually do" content — there is no PDF-producer-divergence angle
  here, just a spec clause. Correct action per the standing redirect
  instruction: point back at `pdfce-spec-librarian`'s existing corpus
  (it already ingested §9.9 and the fsType bit table for this same
  dispatch) rather than duplicate the citation into a different tier
  under a different voice. No file written to `personal_rag/pdf` this
  continuation.

**Repo status:** hashes `0893191` (the hint fix) and `d30842c`
(decision 021 + ledger-checker fix, carried from continuation 73) both
independently verified with `git cat-file -t` per the dispatching
message — this librarian still has no shell-execution tool and cannot
self-verify. Branch `pass-8-redaction`, **67 commits, still no
remote**. `tools/check-ledger-numbers.py` reported GREEN, exit 0.

**Still in flight:**
- ★ Pass 21.x is filed, scoped, and now spec-corrected but **NOT
  STARTED** — 21.0 is next, restricted to `glyf` donors at P0.
- `pdfce-spec-librarian`'s stub rewrite (`font__subsetting_ffc_queue.md`)
  is still owed before any 21.0 code — unaffected by this continuation
  beyond the citation fixes above; still describes the wrong mechanism
  until rewritten.
- Fixture owed: a synthetic embedded-subset-font PDF for
  `fixtures/synthetic`, needed to observe the two corrected hints on
  screen and to test 21.0 itself.
- Open operator questions (r) [narrowed] and (s) [unchanged] still
  await Ken; neither blocks starting 21.0/21.1.

**For next session:**
- Read decision 021 §10 before touching Pass 21.0 code — the P0 floor
  is narrower than the original filing implies (`glyf` donors only).
- Build the missing embedded-subset-font fixture early in 21.0; it
  unblocks both the fixture-owed test debt above and 21.0's own test
  plan.
- Dispatch `pdfce-spec-librarian` for the stub rewrite before writing
  the `SubsetPlan` producer, per the standing instruction (unchanged
  from continuation 73).
- Surface (r) [narrowed] and (s) to Ken at the next natural check-in.

**Same-day continuation 75 (real date 2026-08-04) — Pass 21.0 SHIPPED
(`48c6b77`): pdfce can now add non-Latin text to a PDF. `ROADMAP.md`,
this file, and `D:\dev\rag\rust\` updated.**

**Shipped:**
- Pass 21.0 — FF-C P0 floor (decision 021 §§3–4, narrowed by the
  2026-08-03 spec-review amendment): `subsetter`-backed font
  subsetting/embedding wired into `add-text` for `glyf`/TrueType
  donors, plus `pdfce-cli add-text --embed-font`. Six commits, chain
  `88b9487`→`0c4f490`→`d4e7355`→`5b7bed3`→`eb0bde5`→`48c6b77`, all six
  independently `git cat-file -t` verified by the operator. **This
  lifts the single widest wall in the product** — before this Pass,
  `add-text` could not write any character outside WinAnsi/Symbol/
  ZapfDingbats. Full build record filed as the Pass 21.0 Shipped entry
  (top of `ROADMAP.md`'s Shipped section).

**Decisions made this session:**
- No new architectural decision — this Pass executes decision 021 as
  already scoped (continuations 73/74). One correction propagated
  forward into the record: decision 021 §3.4 understated its own case
  on `/Type0`+`Identity-H` being forced — it is forced TWICE (both by
  `subsetter` stripping `cmap` AND independently by §9.9's own
  `shall`s), not once. Filed as a note on the Pass 21.0 Shipped entry,
  not a new decision record.
- Rule-adoption discipline held again this continuation, consistent
  with continuation 74's precedent: two rule-shaped findings from this
  Pass's bug hunt (a disclosure string needs its TEXT tested against
  the producing branch; an exit-code `_ =>` catch-all silently
  reclassifies future variants as crashes) were written to
  `D:\dev\rag\rust\` as generalizable Rust findings but NOT assigned
  new `ROADMAP.md` standing-rule numbers — adopting a new numbered
  standing rule not already named in an existing decision record isn't
  this librarian's call to make solo.

**Findings + decisions:**
- **R109's `fsType` donor-permission read, though named in decision
  021's original 21.0 slice bullet, did NOT ship with Pass 21.0.**
  `add-text --embed-font` currently embeds a donor face without
  reading or disclosing its `OS/2` `fsType` embedding-permission bits
  — a real gap against R108/R109's own design intent and against rule
  4 (fuzzy-never-sneaky), not mere deferred polish. Flagged
  prominently in three places: the Pass 21.0 Shipped entry's own "NOT
  yet implemented" section, a dated amendment on R109's Standing-rules
  bullet, and the new Pass 21.1 In-progress entry (which now also
  carries the fsType-read follow-up pending an engineer decision on
  whether to fold it into 21.1 or open a standalone slice). Until this
  lands, any `add-text --embed-font` output should be treated as
  UNVERIFIED against the donor's own embedding licence.
- **Composite-glyph-cycle fixture (`48c6b77`) is a worked example of
  "assert the property, don't guard against the unreachable"** —
  `subsetter`'s `closure()` walk is iteratively bounded by
  construction, so a depth guard in pdfce's own glue would be
  unreachable dead code dressed as a defence (R96 shape); the fixture
  proves termination directly instead, and fontTools independently
  corroborates the choice — it cannot even construct the adversarial
  cycle by the recursive route (`RecursionError`). Escalated to
  `D:\dev\rag\rust\assert_termination_property_instead_of_unreachable_depth_guard.md`.
- **`eb0bde5`'s bug hunt (running the CLI once, not just testing it)
  found four defects no automated gate caught**, the sharpest being a
  disclosure string (`base_font=Helvetica`, "no glyph embedding
  (R79)") that stayed true-looking on a run that had just embedded a
  font — R93's exact shape, and no existing test asserts a
  disclosure's exact text against the branch that produced it.
  Escalated to
  `D:\dev\rag\rust\disclosure_text_must_be_tested_against_producing_branch.md`.
  A fourth defect (`EmbeddedBoxedUnsupported` exiting 1 instead of its
  own named 9) traced to a `_ =>` catch-all arm in the exit-code
  mapping. Escalated to
  `D:\dev\rag\rust\exit_code_catchall_reclassifies_future_variants_as_crash.md`.
  All three new files indexed in `D:\dev\rag\rust\index.md` this
  continuation.
- `tools/fontfile-census`'s negative result (2 MiB would refuse none
  of 1,563 embedded programs across 4,023 real PDFs) does NOT set
  FF-C's donor byte ceiling — the census measures *embedded* font
  programs, and ISO 32000-1 §9.9 forbids using an embedded program
  extracted from a PDF as an FF-C donor (decision 021 §10 C-8). The
  tool prints this caveat in its own output. Filed as a PDF-domain
  empirical lesson to `C:\personal_rag\pdf\` this continuation (see
  below) — distinct from the Rust-RAG escalations above because the
  finding is about real-world PDF font-embedding practice, not Rust
  tooling.

**Still in flight:**
- Pass 21.1 (composite-run editability, R110) — promoted to In
  progress; NOT optional, decision 021 is explicit that FF-C is not
  "done" without it.
- R109's fsType read — owed, currently homeless between 21.1 and a
  possible standalone slice; needs an engineer call.
- Pass 21.2 (`set-font` to an embedded face) and 21.3 (GUI face
  picker, `pdfce-ui-specialist` dispatched first) — unchanged, Next up,
  NOT STARTED.
- Open operator questions (r) [narrowed] and (s) — still await Ken;
  neither blocks 21.1 or the fsType-read follow-up.
- Repo status: 74 commits, still no remote; backup bundle refreshed to
  `D:\Dev\pdfce-backups\pdfce-20260804-0015.bundle`
  (`git bundle verify`-clean).

**For next session:**
- Decide whether R109's fsType read is folded into Pass 21.1 or opened
  as its own small slice before 21.2 — this is an engineer call, named
  here so it isn't lost.
- Build the still-owed embedded-subset-font fixture
  (`fixtures/synthetic`) — needed to observe `format_coverage_hint()`/
  `r_inv_1_hint()` on screen for the first time and to test 21.0/21.1
  properly; owed since continuation 74, still not built.
- Do not describe FF-C as "shipped" or "complete" in any operator-
  facing summary until 21.1 (and ideally the fsType read) land — 21.0
  alone is a capability regression risk (can add text it can't edit)
  and a licence-disclosure gap (embeds without reading `fsType`).

**Same-day continuation 76 (real date 2026-08-04) — R109's fsType-read
gap CLOSED (`58fe3f6`); R110's primitive shipped (`c0ed638`); a
SHIPPED-BUT-UNREACHABLE R-INV-4 refusal found and fixed
(`8e08e80`+`87d3cb0`+`6b69956`). `ROADMAP.md`, `ARCHITECTURE.md` §12,
and two `D:\dev\rag\rust\` files updated. Pass 21.1 remains In
progress — NOT shipped, composite runs are locatable-but-refused, not
yet editable.**

**Shipped:**
- No new Pass entry this continuation — all five commits land inside
  the already-open Pass 21.1 (In progress). Continuation 75's own
  "Still owed" item (R109's fsType read) is closed as part of this
  continuation's build; see Findings, below.

**Decisions made this session:**
- **R109's fsType read folded into Pass 21.1's build** rather than
  opened as its own standalone slice — resolving the "fold in or
  standalone" question continuation 75 left open, an engineer call.
- **Two of Open operator question (r)'s three previously-open
  sub-cases now ship an interim disclose-and-proceed default** (absent/
  unparseable `OS/2`; `fsType == 4` Preview & Print) — an engineering
  default needed to ship working code, explicitly NOT a resolution of
  (r). Ken retains the final call on both; R109 was written to accept
  whichever policy he picks, so nothing about this default is
  load-bearing against a future override.
- **Rule-adoption discipline held again, consistent with continuations
  74/75's precedent.** This continuation's headline finding is now the
  FOURTH instance on this project of a confidently-worded comment
  asserting runtime behavior that does not occur (after three prior
  instances already on record in `D:\dev\rag\rust\
  trust_but_verify_doc_comments_are_not_evidence.md`, plus this
  session's own `snap_glyph` `#[allow]` and stale add-text disclosure
  findings, filed to Pass 21.0's own entry). Judged NOT to warrant a
  new numbered `ROADMAP.md` standing rule on this librarian's own
  authority — flagged to the engineer as a pattern frequent enough to
  be worth a deliberate elevation call, not silently adopted.

**Findings + decisions:**
- **R109's fsType read is now enforced, not merely specified.** Read
  from the donor's `OS/2` before subsetting (`subsetter` strips it);
  three named outcomes — `SubsettingNotPermitted` (bit 8, forbids the
  one thing FF-C ever does even though whole-face embedding stays
  legal — the reason bit 8 and bit 9 are separate refusals, not one
  `EmbeddingNotPermitted`), `EmbeddingNotPermitted` (bit 9,
  unconditionally unsatisfiable since pdfce embeds outlines, never
  bitmaps), and correct non-firing on `OS/2` v0/v1 (proven by a
  `nosubset`/`nosubset-v1` fixture pair with byte-identical bits and
  different enforcement — version gating is invisible unless something
  asserts the same bytes mean different things across versions). Seven
  fixtures, one per outcome. Full record: R109's Standing-rules bullet
  and `ARCHITECTURE.md` §12's 2026-08-04 entry.
- **The headline finding: a shipped refusal (R-INV-4) was unreachable
  from `edit-text` on composite runs.** `edit.rs` carried a comment
  claiming composite fonts are refused later, by R-INV-4 — false: the
  text-match stage returned `NoMatch` on every composite run before
  `classify_font` (R-INV-4's home) could run, so the operator got "text
  not found" instead of the correct font-limitation refusal, on ANY
  composite input, ever. Found by trying to reach the message and
  failing TWICE — once against an undecodable fixture (where `NoMatch`
  was arguably honest) and again against a purpose-built
  `cidfonttype2-with-tounicode.pdf` whose text is genuinely findable,
  still getting `NoMatch`. The second failure is what proved the bug.
  Fix is ORDERING: classify the font before matching text, since the
  font-level refusal is a property of the run, never of whether the
  sought text sits inside it. Verified all three arms by running them:
  injective-CMap composite → the real R-INV-4 refusal; no-CMap
  composite → still `NoMatch`, honestly (no character map, no way to
  say what text is there); simple font → unchanged. Same exact shape as
  the already-recorded Pass 19.4 `Tw`/R91 finding, different rule,
  different code path — filed as a SECOND occurrence of the existing
  RAG file, not a new one. Escalated to
  `D:\dev\rag\rust\dead_guard_clause_behind_a_filter_the_guarded_case_cannot_pass.md`
  (second occurrence, with a generalized framing: a precondition check
  on the OBJECT placed after a search step on the QUERY only ever fires
  for objects the search can already handle) and to
  `D:\dev\rag\rust\trust_but_verify_doc_comments_are_not_evidence.md`
  (fourth occurrence overall).
- **Honest limit carried forward, not new this continuation:** the
  widened composite decode assumes `Identity-H` specifically, which is
  what pdfce itself writes and what real composite text overwhelmingly
  uses in practice — other CMap encodings on a composite run stay
  invisible to this decode path, exactly as before the fix. Narrowed,
  not regressed. Not filed to `personal_rag/pdf` — no fresh corpus
  census backs the "overwhelmingly" claim this session (distinct from
  the `tw-census` corpus finding already on record there, which
  measures composite-run PREVALENCE, not CMap-encoding choice within
  composite runs).
- **R110's primitive shipped: `ToUnicodeCMap::injective_inverse()`.**
  Three named disqualifying obstructions (ligature: one code maps to a
  multi-character string, no code answers for the substring alone;
  many-to-one: two codes collide on one scalar, making the inverse a
  relation pdfce would have to arbitrarily resolve; empty map). Ranges
  materialised for this check specifically, unlike ordinary lazy
  `/ToUnicode` lookup, so a range/single collision can't hide. Tested
  against the standard's own §9.10.3 EXAMPLE 2 without asserting
  whether it inverts (that's a fact about the standard's example, not
  about pdfce) — the test only asserts the check runs to completion on
  a FOREIGN CMap and reaches a reasoned decision.
- **Still NOT shipped: actual composite-run editability.** Composite
  runs are now correctly located and refused for the right, disclosed
  reason — not yet rewritable. `ShowSlot::code` (currently `u8`) must
  widen to hold multi-byte CIDs and the operand writer must learn
  multi-byte show operators before R110's conditional lift has anything
  to attach to. Pass 21.0's capability-regression warning (pdfce can
  add composite text it cannot edit) is unchanged by this continuation.

**Still in flight:**
- Pass 21.1 — still In progress, not shippable: editability itself
  (`ShowSlot::code` widening + multi-byte operand writer) remains
  unbuilt.
- Open operator questions (r) [now carries a shipped interim default
  for two of its three sub-cases, still formally open] and (s)
  [unchanged] — still await Ken.
- Repo status: **79 commits, still no remote.** Five hashes spanning
  this continuation's build independently verified by the operator
  with `git cat-file -t` (`58fe3f6`, `c0ed638`, `8e08e80`, `87d3cb0`,
  `6b69956`). **Backup bundle is now STALE, two commits behind** —
  `...0015.bundle` (continuation 75) does not cover any of this
  continuation's five commits; not refreshed this continuation.
- `ARCHITECTURE.md` §3/§4 body-section update for Pass 21.0's new
  `pdfce-render::font::subset`/`pdfce-core::font_embed` modules is
  still owed from continuation 75's ship — flagged again, not silently
  absorbed into this filing.

**For next session:**
- Refresh the backup bundle — two commits stale as of this filing.
- Build `ShowSlot::code` widening + the multi-byte operand writer to
  actually close Pass 21.1 and retire the capability-regression
  warning; this is the remaining blocker between "locatable-but-refused"
  and "editable."
- Consider whether the four-instance "confident comment asserts untrue
  runtime behavior" pattern warrants a new numbered `ROADMAP.md`
  standing rule — an engineer/operator call, not filed solo this
  session.
- Do the owed `ARCHITECTURE.md` §3/§4 body-section sync for Pass
  21.0's new modules, carried over from continuation 75.
- Do not describe FF-C or Pass 21.1 as "shipped" or "complete" in any
  operator-facing summary — composite editability is still unbuilt.

**Same-day continuation 77 (real date 2026-08-04) — librarian-only
filing: no code shipped. Repo/backup state re-verified (79 commits,
backup bundle refreshed and verify-clean, current to `6b69956`);
standing-rule adoption call resolved as NO new rule (R93 and R96
amended in place instead, R86 given a queued scope note); the owed
`ARCHITECTURE.md` §3/§4 body-section sync for Pass 21.0's font-embed
modules is DISCHARGED; a post-reorder GUI smoke test is recorded, no
defect found.**

**Shipped:**
- No new Pass entry — this continuation is documentation/librarian
  work only. Pass 21.1 remains In progress, unchanged from
  continuation 76: composite runs are locatable-and-refused, not yet
  rewritable.

**Decisions made this session:**
- **Repo/backup state re-verified, not re-derived.** Branch
  `pass-8-redaction`, 79 commits, no remote — unchanged count from
  continuation 76. Backup bundle refreshed to
  `D:\Dev\pdfce-backups\pdfce-20260804-0325.bundle`, `git bundle
  verify`-clean, current to `6b69956` — discharges continuation 76's
  "backup bundle STALE, two commits behind" flag.
- **The four-instance "confident comment asserts untrue runtime
  behavior" pattern does NOT get a new numbered standing rule.**
  Reasoning, recorded because "considered and declined" is itself a
  decision worth not re-deriving: R93 already IS this rule (a code
  comment asserting a behavior is not evidence the behavior holds) —
  the `edit.rs`/R-INV-4 instance is filed as R93's fourth occurrence,
  not a fifth rule, because a new rule saying "comments lie" would not
  have caught this instance any better than R93 already states, and a
  standing-rules list's usefulness is inversely proportional to its
  length. What actually caught this instance was R86's habit (observe
  the behavior in the running application) applied to a REFUSAL path
  rather than a success path — that is the one place a rule TEXT
  needed sharpening, so it is filed as a queued scope note on R86
  itself (still PENDING, per item (e) — the note activates alongside
  the rule, not before) rather than as new machinery.
- **The same `edit.rs`/R-INV-4 instance is ALSO filed as R96's second
  occurrence**, because the general form it demonstrates is more
  useful than the instance: a PRECONDITION check (a property of the
  OBJECT) placed after a SEARCH step (a property of the QUERY) only
  ever fires for objects the search can already handle — so the cases
  the guard exists for are exactly the cases that never reach it. The
  only reliable defense is a test asserting the ERROR VARIANT a gate is
  meant to produce, since nothing in the type system requires a
  refusal to be reachable.

**Findings + decisions:**
- `docs/ROADMAP.md` Standing rules: **R93** updated from "third
  occurrence" to a fourth, adding the `edit.rs` composite-font comment
  and its false claim in full. **R96** gains a "second occurrence"
  paragraph recording the same instance with the generalized
  precondition-after-search framing (already filed to
  `D:\dev\rag\rust\dead_guard_clause_behind_a_filter_the_guarded_case_cannot_pass.md`
  as its second occurrence in continuation 76 — this filing brings the
  ROADMAP standing-rule text itself into agreement with that RAG file,
  which is where the "second occurrence" language previously lived
  without a matching ROADMAP bullet). **R86** gains a queued,
  not-yet-active scope note: once item (e) is answered and R86 goes
  live, "observed working in the running application" also covers
  refusal paths, not just success paths — a refusal is operator-facing
  behavior exactly as much as a working feature is.
- `docs/ARCHITECTURE.md` §3 (workspace layout) now documents
  `pdfce-core::font_embed.rs` (plain-data `FontEmbedPlan`/
  `SubsetGlyph`/`DescriptorMetrics`/`OutlineKind`, `build_objects`) and
  `pdfce-render::font::subset.rs` (`plan_subset`, `SubsetError`,
  `MAX_DONOR_BYTES`) under their owning crates, sourced from the
  modules' own doc comments and public signatures, not re-derived from
  the decision document. §4 (core data model) gains a full IMPLEMENTED
  entry for Pass 21.0 recording the same surface plus R107's
  allocate-only round-trip guarantee and the crate-split rationale
  (decision 021 §3.2): subsetting reads like a `pdfce-core` job because
  it is a write concern, but producing a subset first requires
  *parsing* the donor, and that parser already lives in
  `pdfce-render` — putting `subsetter` in `pdfce-core` would give a
  crate with no font-program parser two of them purely to avoid a
  plain-data seam, so the seam is deliberate and `pdfce-core` gains
  zero new dependencies from this Pass. Also recorded: `pdfce-core`
  still has no font-program parser after Pass 21.0 (`fontdata/` stays
  metrics-only), and the entry is explicit that Pass 21.0's contract
  is ADD-only — R110/Pass 21.1 governs editability and remains unbuilt,
  so this sync is not a claim that FF-C is complete. `ARCHITECTURE.md`
  §12 gains a dated 2026-08-04 entry closing the gap continuation 76
  flagged (no new decision — an implementation-record/documentation
  entry against the already-decided decision 021).
- **Smoke test after the shared-path reorder (`87d3cb0`), recorded, no
  defect.** `87d3cb0` reordered font classification above `match_run`
  in `edit.rs` — a path every font shares, not just composite runs —
  so it was verified in the running GUI rather than assumed safe from
  the regression test alone (R86's discipline, applied proactively).
  Release build, `hello.pdf`, Edit Text tool: clicked into a run, typed
  a character, accepted. Canvas updated to "Times will sXubstitute
  too.", the page thumbnail updated with it (Pass 17.0 live-edit
  rendering still correct), the title bar showed the unsaved-changes
  marker, and the "Last edit" panel showed its three disclosure
  bullets (non-embedded-face provenance, incremental-save/prior-text-
  survives, relayout/overflow). The `ℹ` markers rendered as real
  glyphs (Pass 18.7 holding). No regression, nothing owed from this —
  filed as an R86 observation because "a shared path was reordered and
  the interactive path was checked" is only reassuring if written
  down; otherwise a future reader has to re-derive whether it happened.

**Still in flight:**
- Pass 21.1 — still In progress, unchanged: `ShowSlot::code` widening
  past `u8` and a multi-byte operand writer remain unbuilt; composite
  runs stay locatable-but-refused, not editable.
- Open operator questions (r) [interim default live for two of three
  sub-cases, formally still open] and (s) [unchanged] — still await
  Ken.
- Repo status: 79 commits, no remote, backup bundle current and
  verified as of this filing (see Decisions, above).
- No `ARCHITECTURE.md` §3/§4 sync debt remains from Pass 21.0 — fully
  discharged this continuation.

**For next session:**
- Build `ShowSlot::code` widening + the multi-byte operand writer to
  close Pass 21.1 and retire the capability-regression warning — this
  is the one remaining blocker between "locatable-but-refused" and
  "editable," unchanged from continuation 76's ask.
- Do not describe FF-C or Pass 21.1 as "shipped" or "complete" in any
  operator-facing summary — composite editability is still unbuilt.
- When Open operator question (e) is answered and R86 goes live, apply
  its queued scope note (refusal paths count as operator-visible
  behavior too) at the same time — it is written and waiting, not a
  separate follow-up task.

**Same-day continuation 78 (real date 2026-08-04) — SESSION-ENDING
FILING. Pass 21.1 substrate for composite-run editability SHIPPED
(`31d2fdc`, `b98589a`); wiring itself DELIBERATELY NOT STARTED and
surveyed as four coupled changes. New RAG finding filed to
`D:\dev\rag\rust\`. Repo at 82 commits, backup bundle refreshed and
verify-clean to HEAD. `ROADMAP.md` and `D:\dev\rag\rust\index.md`
updated; no `ARCHITECTURE.md` change this continuation (no new
architectural decision).**

**Shipped:**
- No new Pass entry — all three commits this continuation land inside
  the already-open Pass 21.1 (In progress, unchanged status). Pass
  21.1 is closer to shippable than at continuation 77's filing but
  still NOT shippable: substrate is complete and tested; the actual
  edit-path wiring is unbuilt.

**Decisions made this session:**
- **Stopped deliberately after substrate, before wiring — recorded as
  a decision, not a stall.** The wiring survey found FOUR coupled
  changes to the shipped in-place-editing path (composite branch must
  precede the `Unsupported` bail in `glyph_names()`; `glyph_advance`
  needs `/W`/`/DW` per §9.7.4.3, a different table than `/Widths`, not
  a wider argument to the same lookup; `emit_edited_operator` needs a
  hex-string operand for composite runs instead of the literal
  `( … )` string it always writes; `carried_codes`' subset-floor
  accounting needs to become width-aware). A half-applied version of
  any one of these four, landed alone, risks an edit that types
  correctly but writes the wrong operand syntax or advances glyphs
  from the wrong table, with nothing visibly failing — judged worse
  than leaving the Pass open one more continuation. The survey is
  recorded IN THE CODE with these specifics (file, table, syntax), not
  as a bare "TODO: wire it up," so a resuming session does not have to
  re-derive the shape of the remaining work.
- **No new `ROADMAP.md` standing-rule number assigned this
  continuation** — R110 remains the ceiling; this continuation's work
  is filed as substrate additions on R110's existing bullet, consistent
  with how continuations 75–77 have kept rule-adoption calls off this
  librarian's own authority.

**Findings + decisions:**
- **`ShowSlot` widened `code: u8` → `code: u32`, plus a per-slot
  `width: u8` (1 simple, 2 Identity-H) (`31d2fdc`).** This was the
  SPECIFIC thing that made a composite run unrepresentable, not merely
  unimplemented. Landed alone; all 1801 tests passed unchanged — the
  entire claim for this commit is that the type widened and nothing
  downstream yet reads the new range. `match_run`'s `+ 1` advance
  became `+ width`, the same number for every code able to reach it
  before the widening (which is exactly why the old constant looked
  correct and no test caught its narrowness). Three narrowings back to
  `u8` (`prefer`, `carried_codes`, `MatchRun::old_codes`) all go
  through `filter_map`, never a bare cast, because a truncated code is
  a DIFFERENT, VALID code — a silent truncation would splice
  confidently wrong text or falsely tell R-INV-1 the page carries a
  glyph it does not.
- **The near-miss worth recording as a finding, not a footnote: the
  widened type made it possible to silently disarm the continuation-76
  regression test.** The obvious next move after widening the type is
  to start pushing slots for composite runs — it compiles, every test
  passes, and it would have silently disarmed
  `tests/composite_refusal_reachable.rs`. That test currently passes
  BECAUSE composite runs produce zero slots today (the match fails,
  the wrong-but-caught `NoMatch` surfaces, the test's assertion holds
  for the wrong reason) — not because it directly asserts the ordering
  it exists to guard. Give composite runs slots and the match starts
  succeeding; `classify_font` still refuses correctly, so the test's
  assertion (an error variant occurs) STILL PASSES, now on the correct
  ordering, meaning it would stay green even if a future edit silently
  moved classification back below matching. **Generalized and
  escalated:** a regression test that detects a fault via a SECOND,
  incidental property silently stops detecting it the moment that
  property changes for an unrelated, individually-correct reason —
  nothing in the test run reports the coverage loss. Fix: assert the
  SUBJECT directly, immune to the incidental property — here, ask
  `edit-text` for text known ABSENT from the page; correct ordering
  still returns the R-INV-4 refusal (a property of the font, never of
  whether the sought text is findable), broken ordering returns
  `NoMatch`. Written up in full and filed to
  `D:\dev\rag\rust\regression_test_guard_via_incidental_property_disarms_silently.md`
  (new file, indexed in `D:\dev\rag\rust\index.md` this continuation).
- **`CompositeEncoding` shipped (`b98589a`): character→CID, built ONLY
  from a verified-injective `/ToUnicode`.** Construction goes through
  `injective_inverse()` (the R110 primitive, continuation 76) — a
  ligature table or a colliding map never yields an encoder at all, so
  the refusal happens where the evidence already lives, not later at
  encode time when the caller has already committed. A SEPARATE type
  from the existing simple-font `InverseEncoding`, not a mode on it —
  the simple encoder reasons about glyph names, `/Differences`,
  ligature components, and code-occupancy, none of which exist for a
  CIDFont. **The load-bearing test is byte order:** `Identity-H` codes
  are big-endian per §9.7.6.2 — reversing the two bytes yields a
  different, VALID code pointing at a different glyph; nothing errors,
  the page just silently says something else. `to_bytes()` lives on
  the encoder's result type, one place to get this right. A CID above
  16 bits is refused, not truncated, same reasoning as the `u32`→`u8`
  narrowings.
- **New fixture `composite-editable.pdf`** (`/Type0`, three CIDs,
  injective `/ToUnicode`, extracts "ABC") built at HEAD, before the
  wiring code that will need it, deliberately, so the fixture is not
  shaped around what that code happens to do.

**Still in flight:**
- **Pass 21.1 — still In progress, closer but not shippable.**
  Substrate complete and tested (`ShowSlot` widened, `CompositeEncoding`
  shipped, `injective_inverse()` from continuation 76, editable
  fixture built). Composite runs remain LOCATABLE-BUT-REFUSED with an
  honest, specific, disclosed reason — not yet rewritable. The four
  coupled wiring changes surveyed above are the entire remaining scope.
  Pass 21.0's capability-regression warning (pdfce can add composite
  text it cannot edit) is unchanged by this continuation.
- Open operator questions (r) [interim default live for two of three
  sub-cases, formally still open] and (s) [unchanged] — still await
  Ken; see the consolidated list below.
- **Repo status: 82 commits, still no remote.** `31d2fdc` and
  `b98589a` independently `git cat-file -t` verified by the operator as
  `commit` objects; the third commit (fixture + wiring survey, at HEAD)
  is recorded as "HEAD at session end" rather than a specific hash
  string — its count was confirmed, its hash was not separately
  verified this filing (this librarian has no shell tool and cannot
  self-verify hashes). Backup bundle refreshed to
  `D:\Dev\pdfce-backups\pdfce-20260804-final.bundle`, `git bundle
  verify`-clean, current to HEAD — supersedes `...0325.bundle`
  (continuation 77).
- Test suite: 1806 tests passing; `cargo fmt --check`, `cargo clippy
  -- -D warnings`, `tools/check-ui-strings.sh`,
  `tools/check-ledger-numbers.py` all clean; `cargo tree -p pdfce-core`
  / `-p pdfce-render` still GUI-free — all re-confirmed this
  continuation, not carried over unverified.

**Operator decisions owed — CONSOLIDATED, priority order (Ken has been
away the whole session; this list exists so the next reader does not
have to re-derive it by grepping five separate continuations):**

1. **Font-EULA policy (Open operator question (r)) — a LEGAL call, not
   an engineering one.** Two sub-cases currently ship an interim
   disclose-and-proceed default, neither a resolution: (a) donor
   `OS/2` `fsType` ABSENT or UNPARSEABLE — proceeds, disclosed as
   unknown; the trap named repeatedly across this project's record is
   that `fsType == 0` means Installable, the MOST permissive value, so
   "absent" must never be silently modelled as `0`. (b) `fsType == 4`
   (Preview & Print) — proceeds; this value permits the embed itself
   but additionally obliges the *document* stay read-only thereafter,
   an obligation on every future reader that pdfce has no PDF field to
   express and cannot enforce, so "proceed" here is pragmatic, not a
   claim the obligation is satisfied. R109 is written to accept
   whichever policy Ken picks for either sub-case (refuse outright /
   disclose-and-require-acknowledgement / disclose-and-proceed as
   currently shipped).
2. **Complex-script posture (Open operator question (s)).** FF-C plus
   standing rule R17 (no shaping, ever) means Arabic/Devanagari/Thai
   text would EMBED but RENDER WRONG (glyphs placed by advance, no
   GSUB/GPOS). Engineer recommendation on record: refuse by name —
   painting confident nonsense is the rule-4 (fuzzy-never-sneaky)
   failure — but this caps a headline capability (FF-C's non-Latin
   story becomes "CJK/Cyrillic/Greek/Hebrew yes, Arabic/Devanagari/Thai
   no"), so it is filed as Ken's call, not a solo engineering decision.
3. **Sequencing — form-building tools remain unstarted.** Decision
   020's item #4 (Ken's stated ★★★★ priority-sequence item) is still
   queued behind FF-C/Pass 21.1, which itself is still queued behind
   the four-item wiring survey above. FF-B (cross-block/cross-page
   reflow) also remains unscheduled. This is not itself a question
   needing an answer — Ken has not objected to the engineer's
   redaction-apply-first resequencing (item (l), already logged) — but
   is named here because "text-handling" (priority #3) has now stayed
   open across five-plus continuations and is worth an explicit status
   check the next time Ken is present.
4. **R86's status (Open operator question (e)) — formally PENDING,
   practically already being followed.** R86 (a Pass does not ship
   until observed working in the running application, not merely
   tested headlessly) remains unratified — Ken confirmed the related
   Pass-17 sequencing question (f) without addressing (e) directly.
   Worth surfacing plainly: this session's own R86-shaped smoke test
   (continuation 77, the post-reorder GUI check) and the discipline
   behind this continuation's regression-test finding above are both
   examples of the rule already being PRACTISED in substance, even
   though it has not been formally adopted. Answering (e) would make
   explicit a habit that is already load-bearing.

**For next session:**
- Build the four coupled wiring changes surveyed above
  (`glyph_names()` composite branch, `/W`/`/DW` advance lookup, hex-
  string operand emission, width-aware `carried_codes`) to actually
  close Pass 21.1 and retire the capability-regression warning — this
  is the entire remaining scope, unchanged in kind from continuations
  76/77's ask but now precisely enumerated rather than a single bullet.
- When wiring lands, REWRITE `tests/composite_refusal_reachable.rs` (or
  add a sibling test) to search for text known ABSENT from the page —
  the discriminator that survives the slot-pushing change that would
  otherwise disarm the existing version; do not assume the existing
  test still proves what its name claims once slots exist.
- Do not describe FF-C or Pass 21.1 as "shipped" or "complete" in any
  operator-facing summary — composite editability is still unbuilt.
- The four consolidated operator decisions above are the standing ask
  for whenever Ken is next present — nothing here blocks further
  engineering work, but (1) and (2) in particular are legal/product
  calls this project has been shipping interim engineering defaults
  around rather than resolving.

**Same-day continuation 79 (real date 2026-08-04) — terminology ruling
filed: "pdf dimension" vs "ce dimension," never bare "dimension."
`pdfce-librarian`-only filing, no code shipped.**

**Shipped:**
- No new Pass entry. This continuation is documentation-only, per the
  operator's explicit terminology ruling and the librarian dispatch
  that followed it.

**Decisions made this session:**
- **Operator ruling (2026-08-04), codified as `CLAUDE.md` rule 15,
  commit `89c5837`: bare "dimension" is banned project-wide.** Two
  unrelated things share the word with OPPOSITE properties — a **pdf
  dimension** (CAD/authoring-tool-exported content already in the
  file, read-only from pdfce's side) and a **ce dimension** (a
  `/Line`+`/IT /LineDimension` annotation pdfce itself authors, fully
  editable/deletable). The distinction is provenance, not
  representation. Why a rule, not a style note: the operator could not
  decode analysis that used "dimension" throughout without saying
  which kind, in both directions — ambiguous agent output is hard for
  him to act on, and an ambiguous report from him can misdirect
  troubleshooting. Binding on every agent, reply, commit message, doc
  comment, decision record, RAG entry, and subagent dispatch. Full
  text: `CLAUDE.md` rule 15; cross-referenced from `ROADMAP.md`'s
  Glossary (new `pdf dimension` / `ce dimension` entries, this
  filing) and Standing rules (new ★ Terminology ruling entry, this
  filing).

**Findings + decisions:**
- **Audit result: the ROADMAP prose itself was mostly clean.** Grepped
  every "dimension" mention across `ROADMAP.md` and `SESSION_LOG.md`.
  Pass 21.x (FF-C font subsetting) has zero "dimension" mentions —
  unrelated subsystem, no action needed. The 2026-08-04 SESSION_LOG
  continuations (75–78) had zero "dimension" mentions prior to this
  filing. The one Backlog bucket that did — "Dimension-tool bug-fix
  cluster," Pass 12.M2c — was entirely about **ce dimensions**
  (pdfce's own dimensioning tool, Pass 12.M2/12.M2b family) and has
  been retitled/qualified in place ("Ce-dimension-tool bug-fix
  cluster"; Backlog is a mutable current-state section, not
  append-only, so this was a direct edit, not a dated footer).
- **★ The load-bearing finding is at the decision-record level, not
  the prose level.** Decisions 022 (annotations in canvas selection)
  and 023 (Obj-tool level navigation, node editing, dimension
  re-measure, format surface) exist at `docs/decisions/022-...md` and
  `docs/decisions/023-...md`, both dated 2026-08-04, status "Decided
  (consultant recommendation; engineer to schedule, librarian to
  file)" — **but neither has yet been promoted into a `ROADMAP.md`
  Pass entry or an `ARCHITECTURE.md` §12 decision-log entry.** Read
  both in full for this audit. **Both are scoped almost entirely
  around ce dimensions**: decision 022's root cause is that
  `decompose_page` never reads `/Annots`, so a ce dimension (painted
  via `pdfce-render`'s annotation pass) is invisible to every selection
  path; decision 023 inherits the same framing for re-measure (§5) and
  the display-format surface (§6). Decision 023 §0 finding 2 / §1.2 DOES
  independently find a second, structurally identical paint/select
  asymmetry in **form XObjects** — `pdfce-render`'s interpreter recurses
  into a `Do` on a form and paints its contents individually, while
  `decompose.rs` emits one opaque object for the same `Do` — which is a
  **pdf-dimension-shaped** defect (foreign, CAD-exported content: title
  blocks, hatches, placed drawing blocks), addressed by Pass 23.2
  (level navigation), not Pass 22.0.
- **The scoping question this produces, filed as new ROADMAP Open
  operator question (t).** The operator's original complaint — some
  objects in a CAD-exported drawing don't box-select, "like dimension
  lines and dimensions in that drawing" — was almost certainly
  describing **pdf dimensions**, not pdfce-authored ce ones. If the
  unselectable geometry in that drawing was flattened paths or a
  placed form (the common CAD-export shape), **Pass 22.0 alone does
  not fix what was reported.** 22.0 fixes a real, independently
  confirmed defect (ce dimensions were never selectable by any surface
  — a gap the operator had not yet hit, per decision 022 §0's own
  second finding), but it is a different defect from the one in the
  original report. Pass 23.2 is the more likely candidate fix for the
  literal complaint. Recorded as a recommendation, not a decision —
  confirming which the drawing's unselectable objects actually were is
  the operator's call, not resolved by this audit.
- **Filing decisions 022/023 into `ROADMAP.md`/`ARCHITECTURE.md` §12
  (including standing-rule number assignment against decision 022 §8's
  4 proposed rules and decision 023 §9's up-to-6) is deliberately NOT
  done this continuation.** Scoped out on purpose: filing them now,
  today, with the qualified `pdf dimension`/`ce dimension` terminology
  established in the same session as the ruling, means they enter the
  permanent record correctly from day one rather than needing a second
  correction pass. Flagged as owed, next session.

**Still in flight:**
- Pass 21.1 wiring — unchanged from continuation 78, still the entire
  remaining scope for FF-C composite-run editability; not touched this
  continuation.
- Decisions 022/023 — not yet filed as Pass entries; see above. Pass
  22.0 (select + delete ce dimensions) and Pass 23.0–23.3 (format
  surface, re-measure, level navigation, node editing) remain
  unscheduled in the numbered ledger despite being fully designed.
- New ROADMAP Open operator question (t) — the 022-vs-023 scoping
  question above — joins the existing consolidated operator-decisions
  list from continuation 78 (font-EULA policy, complex-script posture,
  form-tools sequencing, R86 ratification).

**For next session:**
- File decisions 022 and 023 into `ROADMAP.md` (Pass 22.0a/b/c under
  Next up; Pass 23.0–23.3 under Next up or Backlog per dependency
  ordering — 23.0 is independent, 23.1/23.2 depend on 22.0) and into
  `ARCHITECTURE.md` §12, using `pdf dimension`/`ce dimension`
  throughout. Assign standing-rule numbers against decision 022 §8 (4
  proposed rules, next free per the decision's own header was R111 at
  filing time — **re-verify the live ceiling before assigning, per
  this project's own repeated numbering-collision history**) and
  decision 023 §9 (up to 6 proposed rules, one explicitly hedged as
  "may be too small for a number" — judgment call, not automatic).
  Add the `TargetId` enum / composite-provider change to
  `ARCHITECTURE.md`'s body sections once actually built (decision-log
  entry now, body-section sync when Pass 22.0c ships — same
  disposition as decision 021's entry).
- Confirm Open operator question (t) with Ken before treating Pass
  22.0's eventual ship as closing the original box-select complaint —
  open the original CAD-exported drawing (or ask) to determine whether
  its unselectable "dimension lines and dimensions" were pdf
  dimensions or ce dimensions.
- New RAG file `D:\dev\rag\rust\overloaded_term_ambiguity_becomes_scope_ambiguity.md`
  (indexed) generalizes this session's finding — worth a skim next
  time an overloaded term shows up anywhere else in this project's
  vocabulary (candidates worth a future glance: "annotation" already
  qualified by subtype throughout, seems fine; "object" is used both
  for `PageObjects` vector objects and generic PDF objects — not
  audited this session, flagged only as a maybe).

**Same-day continuation 80 (real date 2026-08-04) — decisions 022/023
FILED into `ROADMAP.md`/`ARCHITECTURE.md` §12, exactly the follow-up
continuation 79 recommended. `pdfce-librarian`-only filing, no code
shipped.**

**Shipped:**
- No new Pass entry shipped or built. This continuation is documentation-
  only: Pass 22.0 and Pass 23.0–23.3 are now FILED (Backlog, not yet
  started) but not begun.

**Decisions made this session:**
- **Standing rules R111–R120 assigned** against the live ceiling
  (R110, re-confirmed by Grep against `ROADMAP.md` — this librarian has
  no Bash/shell tool and could not run
  `tools/check-ledger-numbers.py` directly). R111–R114 = decision 022's
  four proposed rules; R115–R120 = decision 023's six (including the
  "may be too small for a number" methodology rule — numbered anyway,
  on the R106 precedent that a methodology rule earns a number when it
  closes a concrete collision risk, which R120/`resolve_escape`'s
  two-concurrent-Pass signature hazard does). Two amendment notes
  added: R111 (decision 023 found a second live violation, form
  XObjects) and R112 (strengthened to require the handle also express
  LEVEL).
- **Pass families 22 and 23 confirmed free and headed for the first
  time** — `### Pass` headings did not exist for either before this
  filing (Grep-verified, not script-verified, same caveat as above).
  Filed as a new Backlog bucket (not Next up) — both decisions are
  "Decided, engineer to schedule," structurally identical in status to
  the Forms bucket (decision 020), which is the precedent followed for
  placement.
- **R58's staleness flagged, NOT fixed.** `ARCHITECTURE.md` §5.9 and
  `ROADMAP.md`'s R58 bullet both gain a note that R58's literal text
  ("every removal/scrub operation forces a full rewrite") is already
  contradicted by two SHIPPED operations (`delete_object`,
  `delete_redaction_mark`) that correctly stay under incremental save —
  the same confidentiality-contract-vs-not distinction §5.11/R70
  already drew for text editing. The wording fix itself is left to the
  operator (new Open operator question (v)) per decision 022 §5.4's own
  explicit deferral — this librarian added the flag, not the rewrite.
  New `ARCHITECTURE.md` §5.12 records the SETTLED half (annotation
  deletion is not a fifth forced-full-rewrite family member) separately
  from the unsettled wording question.

**Findings + decisions:**
- **No `ARCHITECTURE.md` §3/§4 body-section update this filing —
  deliberate, following the decision-020/021 precedent verbatim**
  ("nothing has shipped, so §4 describes no new reality yet"). This
  directly resolves what the dispatch flagged as a live risk ("you have
  carried §3/§4 sync debt before; do not let this one accrue") by NOT
  writing speculative pre-ship content into the core-data-model
  contract, rather than by writing it. The one exception (§5.9/§5.12)
  is a correction to ALREADY-shipped reality, not a preview of
  unbuilt work — same category as decision 020's decision-009
  forward-pointer exception, cited explicitly in both new §12 entries.
- **The Obj-tool-universality reconciliation (decision 023 §5.1–§5.2)
  recorded in its own right, per the dispatch's explicit ask** — added
  to `ROADMAP.md`'s Glossary as a durable project principle, not just
  restated inside the Pass entry: the Obj tool is universal at the
  SELECTION layer (it can select anything), not the verb layer (a ce
  dimension's re-measure gesture stays owned by the Measure tool, the
  Obj tool only routes to it). This is the sentence that lets the
  operator's "everything" instruction and decision 022's
  anti-silent-re-measure argument both hold without either being bent.
- **Eight new Open operator questions filed: (u)–(ab)** — drawn from
  decision 022 §9 items 2/3 (widget-annotation delete posture; R58
  wording) and decision 023 §10 items 1/2/3/4/5/7 (per-group vs.
  per-ce-dimension format; `reduce` GUI toggle; form-un-sharing command;
  navigation-depth model; node-delete curve-refit; snapping inside
  forms). Neither decision's own "for the operator" section had ever
  been given ROADMAP letters before this filing — both listed the
  items only inline in the decision documents themselves.
- **Ordering judgment given, as explicitly asked for by the operator:
  decision 023 §7.1's "Pass 23.0 first" is an engineering-risk-only
  argument (zero hierarchy risk, no dependency) and does not fix what
  was reported.** The terminology audit (continuation 79) found the
  operator's original complaint is very likely a pdf-dimension/
  form-XObject problem that neither 22.0 nor 23.0 touches — only 23.2
  (dependent on 22.0) does. Recorded plainly, as a recommendation not a
  decision, in both the new Backlog entry and a footer added to Open
  operator question (t): the fix-oriented build order is **22.0 → 23.2**,
  and question (t)'s own confirmation step (open the reported drawing,
  or ask) should happen before committing to any order at all, since it
  determines whether 22.0 alone or 22.0+23.2 is the actual fix. 23.0's
  zero-risk shape makes it safe to build in parallel or while waiting on
  that confirmation, not a reason to sequence it strictly first.
- **Cross-references tightened, not just added:** the Pass 12.M2c
  cluster's bug-#1 pointer (previously "see the ★ decisions 022/023
  filing-status note under Standing rules," which named no entry that
  actually existed under that description) now points at the real new
  Backlog bucket by name.

**Still in flight:**
- Pass 21.1 wiring — unchanged, still the entire remaining scope for
  FF-C composite-run editability; not touched this continuation.
- Pass 22.0/23.0–23.3 — now filed with stable IDs, standing rules, and
  open questions, but **zero code written**. Nothing promoted to Next
  up or In progress this continuation; that is the engineer's/
  operator's sequencing call, informed by (but not settled by) this
  filing's ordering judgment.
- Open operator question (t)'s core confirmation (whether the
  operator's drawing contained pdf dimensions, ce dimensions, or both)
  is UNCHANGED,
  still unresolved — only the "file the decisions" action item that
  question (t) itself named as owed is discharged this continuation.

**For next session:**
- Resolve Open operator question (t)'s confirmation before starting
  build — it determines whether Pass 22.0 alone, or 22.0+23.2, is the
  actual fix for the operator's reported box-select complaint.
- Once confirmed, decide build order: this filing's recommendation is
  22.0 → 23.2 (fix-oriented) rather than decision 023's own 23.0-first
  (risk-minimizing) — the operator/engineer may reasonably choose
  differently, but should choose knowingly rather than by default.
- Eight new open questions now live in `ROADMAP.md` ((u)–(ab)), joining
  continuation 79's (t); none block Pass 22.0/23.0 starting, all are
  worth a batched answer
  when Ken is next reviewing open items — several have stated defaults
  that will simply apply if left unanswered (see each item's own
  "Default:" line).
- §4 (core data model) and §3 (workspace layout) still owe their real
  body-section updates once Pass 22.0c (`TargetId` enum) and Pass 23.2
  (`PageObjects.containers`) actually ship — flagged here so it isn't
  rediscovered as "sync debt" the way earlier Passes' §3/§4 gaps were.
