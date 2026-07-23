# pdfce — Roadmap

**The contract.** Every operator (Ken) request gets parsed into a Pass
entry here; every completion gets recorded here. Read this file at the
start of every session. Maintained by `pdfce-librarian`, dispatched by
`pdfce-engineer` — the engineer does not edit this file directly (see
`.claude/agents/pdfce-engineer.md`).

## Glossary

- **Pass** — a scoped unit of engineering work with acceptance criteria,
  numbered `Pass N[a-z]` (sub-letter when a feature needs splitting for
  shippability). IDs are stable and never reused.
- **pdfce-core** — the GUI-agnostic Rust crate: object model, parser,
  writer, filters, fonts, crypto, content-stream interpretation.
- **pdfce-render** — headless rasterizer (draw-ops -> pixels), no GUI deps.
- **pdfce-gui** — the native egui/eframe desktop shell.
- **pdfce-cli** — the command-line batch shell (merge/split/stamp/
  convert/sign/validate subcommands). Depends on pdfce-core +
  pdfce-render only, same zero-GUI-deps discipline as pdfce-gui. See
  `ARCHITECTURE.md` §7.
- **Incremental save** — appending a new xref section + changed objects
  only, leaving untouched bytes in place. Default save mode (see
  `ARCHITECTURE.md` §5). Required for signature-validity semantics.
- **Spec RAG** — the canonical PDF-standard reference corpus at
  `D:\Dev\Rag-Specialized\PDF_Spec\`, owned by `pdfce-spec-librarian`.
  Consult it before implementing any spec-governed behavior; don't
  guess byte layouts from training-data memory.
- **Feature RAG** — the Acrobat Pro feature-parity reference corpus at
  `D:\Dev\Rag-Specialized\Acrobat_Features\`, owned by
  `pdfce-acrobat-librarian`. Catalogs capability/behavior/edge-cases/
  limits per feature — never GUI mechanics. Consult it before scoping
  a Backlog bucket into a real Pass, so acceptance criteria reflect
  actual Acrobat behavior.
- **Prior art** — `docs/PRIOR_ART.md`, the survey/decision record of
  existing open-source crates and tools pdfce can depend on or learn
  from. See `docs/LEGAL.md` §6 for the binding license-classification
  and attribution rules that govern adding any dependency.
- **Fuzzy, never sneaky** — a UX principle inherited from the user's
  other projects (MatExtractor): algorithmic suggestions (OCR text,
  auto-detected form fields, suggested Bates ranges) are always
  reviewable hints, never silent auto-applies.

## Shipped

*(empty — project bootstrapped 2026-07-23, no engineering work has
started yet)*

## In progress

*(none)*

## Next up

### Pass 0 — Workspace bootstrap
- **Confirm with the user, before scaffolding `pdfce-core`: build from
  scratch, or adopt/vendor `oxidize-pdf` (MIT) as a foundation?** See
  `docs/PRIOR_ART.md`'s "OPEN QUESTION" — it claims to already cover
  most of pdfce-core's target scope (incremental saves, RC4/AES,
  PKCS#7 verify, pure-Rust JBIG2/CCITT/DCT/JPX) but is single-
  maintainer and unaudited. This is a foundational decision, not a
  routine dependency pick — do not default to "build from scratch"
  without raising this explicitly.
- Confirm egui vs iced with the user (see `ARCHITECTURE.md` §2.1).
- Create the Cargo workspace (`pdfce-core`, `pdfce-render`, `pdfce-gui`,
  `pdfce-cli`) per the layout in `ARCHITECTURE.md` §3 and §7.
- `pdfce-gui` Pass 0 acceptance: a native window opens, shows a blank
  canvas, has a working "Open File" dialog (`rfd`) that at minimum
  reads the file's first bytes and confirms the `%PDF-` header + version
  — full parsing comes in Pass 1.
- `pdfce-cli` Pass 0 acceptance: `pdfce-cli --version` and `pdfce-cli
  --help` work and list the planned subcommands (stubs are fine —
  real subcommand bodies land alongside each feature's own Pass); a
  minimal `pdfce-cli inspect <file>` subcommand reads the file's first
  bytes and confirms the `%PDF-` header + version, mirroring the GUI's
  Pass 0 bar. Establishes the `clap` scaffold and exit-code convention
  (`ARCHITECTURE.md` §7) other subcommands build on.
- Packaging smoke test per `ARCHITECTURE.md` §6 (copy-to-fresh-folder-
  and-run) passes for **both** `pdfce-gui` and `pdfce-cli` binaries on
  a clean machine/user profile if possible.
- `cargo tree -p pdfce-core`, `cargo tree -p pdfce-render`, and
  `cargo tree -p pdfce-cli` show zero GUI/windowing dependencies —
  verify this explicitly, don't assume it.
- `cargo fmt --check` and `cargo clippy -- -D warnings` clean across
  the whole workspace from the very first commit — establish the habit
  at Pass 0 rather than retrofitting it once there's a backlog of
  violations. See `ARCHITECTURE.md` §8 and
  `D:\dev\rag\rust\rust-style-guide-and-api-guidelines.md`.
- Consult `docs/PRIOR_ART.md` before picking crates for the initial
  workspace (parser/filter/font/crypto foundations especially) —
  don't default to the first crate name that comes to mind from
  training data without checking it against the survey. Set up
  `cargo-about` + `about.toml` as soon as the workspace has its first
  real dependencies, so `THIRD_PARTY_LICENSES.md` generation is a
  solved problem from day one rather than a retrofit. See `LEGAL.md`
  §6.

### Pass 1 — Minimal parse + render (read-only viewer)
- `pdfce-core`: tokenizer, COS object model, xref table (classic,
  non-stream) parsing, trailer, page tree walk + inherited-attribute
  resolution, FlateDecode filter (`flate2` + `miniz_oxide`/`zlib-rs`
  backend — never the C `zlib`/`zlib-ng` backend, see `PRIOR_ART.md`).
- `pdfce-render`: content-stream interpreter for the baseline graphics
  operators needed to render text + vector paths + raster images
  (`Tj`, `TJ`, `Td`, path-construction + paint ops, `Do` for XObjects),
  rasterizing via `tiny-skia` (`PRIOR_ART.md`).
- `pdfce-gui`: page canvas with pan/zoom, thumbnail rail, page
  navigation. Read-only — no editing yet (no undo/redo mechanism
  needed for this Pass; required starting the first editing Pass —
  see `ARCHITECTURE.md` §11.4).
- **`cargo-fuzz` target for the tokenizer/object parser** (raw bytes →
  tokenizer → COS parser, asserting no panic/no hang past a bounded
  timeout/no unbounded allocation) — required for this Pass to ship,
  not optional. See `ARCHITECTURE.md` §10.2.
- **Resource-limit guards** on the FlateDecode path (output-size
  ceiling) before it's considered done — see `ARCHITECTURE.md` §10.1.
- Pull down the veraPDF corpus + PDF Association PDF 2.0 examples via
  `fixtures/fetch-corpora.sh`, hand-select the fixture files this Pass
  actually needs.
- Acceptance: pdfce opens and correctly renders a small, legally-clean
  fixture corpus (see `LEGAL.md` §Test corpus sourcing) at pixel
  parity (or documented near-parity) with a reference renderer.

## Backlog (Acrobat-parity feature buckets — not yet scoped to Passes)

Grouped by rough Acrobat Pro feature area. Each bucket gets scoped into
real Pass entries as the engineer reaches it — this list exists so
nothing gets forgotten, not as a commitment to build in this order.

- **Core document ops** — merge/split/extract/insert/delete/rotate/
  reorder pages; page-size & rotation normalization.
- **Text & object editing** — in-place text edit with font re-flow,
  image replace/move/resize, vector object edit.
- **Forms (AcroForm)** — field creation/editing, appearance-stream
  generation, form-field auto-detection (as a *hint*, per
  fuzzy-never-sneaky), flatten-to-static.
- **XFA** — legacy Adobe forms tech. **Verify current status before
  scoping** — Adobe has been deprecating XFA in Acrobat; consult the
  spec RAG + a fresh web check before committing engineering time here.
  Likely low priority relative to AcroForm.
- **Digital signatures** — PAdES profiles (B-B, B-T, B-LT, B-LTA),
  PKCS#7 signing + verification, incremental-update-based signing
  (see `ARCHITECTURE.md` §5), timestamp authority (RFC 3161) support.
- **Encryption** — standard security handler, RC4 (legacy read-compat
  only, never write), AES-128/256, public-key (certificate) security
  handler.
- **Redaction** — true content removal (not visual-overlay-only), per
  `ARCHITECTURE.md` §5 corollary. This is a trust-critical feature;
  needs explicit test coverage proving removed content is actually
  gone from the saved bytes, not just hidden.
- **Bates numbering / stamping** — header/footer stamps, sequential
  numbering across a batch, watermarks.
- **Comments & markup** — annotations (text notes, highlights, ink,
  shapes, stamps), reply threads, markup summary/export.
- **OCR** — recognize-text-in-scanned-page. Needs a decision on OCR
  engine binding (candidate: `tesseract` via a Rust binding, or a
  pure-Rust OCR crate if one is production-quality by the time this
  is scoped — check current state, don't assume 2026-era training
  data is current). Output is always reviewable hint text, never
  silently baked in without operator confirmation.
- **Accessibility (PDF/UA)** — tagged-PDF structure tree authoring +
  validation, reading-order tools, alt-text prompts for images. Also
  see `.claude/agents/pdfce-ui-specialist.md` — the *app's own UI*
  should aim for screen-reader accessibility too, not just its output
  files.
- **Comparison** — visual diff + text diff between two PDF revisions.
- **Portfolios (PDF Package)** — multi-file container support.
- **Optimization / linearization** — "Fast Web View" linearized output,
  image downsampling, font subsetting on save, size-reduction reports.
- **PDF/A conformance** — convert-to and validate-against PDF/A-1/2/3/4
  profiles; surface non-conformance reasons in a way a non-specialist
  operator can act on.
- **Print & prepress (PDF/X)** — lower priority unless the user
  signals otherwise; flag as backlog-only until requested.
- **Product-scope decisions — deliberately deferred, not oversights**
  (identified 2026-07-23, flagged rather than silently skipped):
  - **Internationalization/localization.** No decision on whether v1
    ships English-only or externalizes UI strings from the start.
    Cheap to bake in now (route every UI string through a translation
    layer even if only `en` is populated); expensive to retrofit into
    a GUI codebase later. Flag to the user before Pass where the first
    real UI strings get written — this is the point of no return for
    "cheap to add."
  - **Cross-platform scope beyond "Windows first."** `ARCHITECTURE.md`
    §6 says Windows is the first packaging target — confirm with the
    user whether that's a deliberate v1 scope decision or just how the
    project happened to start (egui/eframe supports macOS/Linux
    natively, so it's not a technical blocker either way, just a
    testing/packaging-effort scope question).
  - **Update/release mechanism.** No installer means no auto-updater
    by default — is "download and replace the folder" the permanent
    answer, or does pdfce want an opt-in update checker later? Ties
    directly to `ARCHITECTURE.md` §1.1's privacy posture (any update
    mechanism must be opt-in, never silent phone-home).
- **CLI batch operations (`pdfce-cli`)** — a subcommand per feature
  bucket above, added *alongside* that feature's own GUI Pass rather
  than as a separate late-stage effort (e.g. the Bates-stamping Pass
  ships both the GUI flow and `pdfce-cli bates-stamp`, same session).
  See `ARCHITECTURE.md` §7 for the subcommand shape and exit-code
  convention. Pass 0 seeds the scaffold; this bucket tracks ongoing
  subcommand coverage as other features land.

## Standing rules

- **Documentation-first.** Every module gets a thorough header
  docstring (purpose, contracts, spec citations); every function gets
  a doc comment explaining WHY; every Pass gets a roadmap entry the
  same session it ships.
- **Spec-fidelity discipline.** Never implement spec-governed byte
  layout, filter, or structural behavior from memory — check
  `D:\Dev\Rag-Specialized\PDF_Spec\` first (via pdfce-spec-librarian
  if the answer isn't already cached in a prior session's notes).
- **Feature-fidelity discipline.** Before scoping a Backlog bucket
  into a real Pass, consult `D:\Dev\Rag-Specialized\Acrobat_Features\`
  (via `pdfce-acrobat-librarian` if the bucket isn't cataloged yet) so
  acceptance criteria reflect actual Acrobat Pro behavior, not
  assumption. That RAG describes capabilities only — never use it (or
  let it lead to) copying Acrobat's GUI structure; pdfce's UI is
  designed independently.
- **GUI-core separation is load-bearing, not a suggestion.** See
  `ARCHITECTURE.md` §3 invariant. Verify with `cargo tree` on any Pass
  that touches `pdfce-core` or `pdfce-render` dependencies.
- **Round-trip / minimal-diff editing** per `ARCHITECTURE.md` §5,
  with redaction as the sole deliberate, explicit exception.
- **Fuzzy, never sneaky** for every algorithmic suggestion (OCR,
  auto-detected fields, suggested Bates ranges, etc.).
- **Test-corpus sourcing discipline** per `LEGAL.md` — no
  unknown-provenance real-world PDFs in the repo.
- **Rust Style Guide + API Guidelines compliance.** `cargo fmt --check`
  and `cargo clippy -- -D warnings` clean before any Pass ships; any
  `pub` item added to `pdfce-core` checked against
  `D:\dev\rag\rust\rust-style-guide-and-api-guidelines.md`. See
  `ARCHITECTURE.md` §8.
- **Every feature Pass considers both `pdfce-gui` and `pdfce-cli`.**
  Not every feature needs a CLI subcommand on day one, but the default
  is to ship both together — see the "CLI batch operations" backlog
  entry above.
- **No dependency without a license check.** Every new `Cargo.toml`
  entry is classified permissive/weak-copyleft/strong-copyleft before
  it's added; copyleft is always flagged to the user, never decided
  solo. See `LEGAL.md` §6. Attribution is generated (`cargo-about` →
  `THIRD_PARTY_LICENSES.md`), never hand-maintained.
- **Adversarial-input hardening is not optional.** Every filter
  decoder gets an output-size ceiling; every recursive structure gets
  a depth/cycle guard; the parser gets a fuzz target before Pass 1
  ships. See `ARCHITECTURE.md` §10.
- **Undo/redo is command-log-based, built into the first editing
  Pass, not retrofitted.** The dirty-set for incremental save is
  computed as a diff against the base revision at save time, never as
  the union of every command ever run. See `ARCHITECTURE.md` §11.
- **No network calls without explicit user opt-in, ever.** No
  telemetry, no silent update-checks, no phone-home. See
  `ARCHITECTURE.md` §1.1.
- **Solo by default.** Workflow tool only when parallelism is genuine
  and the user has opted in (ultracode or explicit request).

## Update protocol

- New operator request → engineer parses into Pass entry/entries →
  dispatches pdfce-librarian to add under *Backlog* or *Next up* →
  reports assigned Pass IDs back to the operator.
- Backlog bucket → real Pass (scoping) → engineer dispatches
  `pdfce-acrobat-librarian` for the matching feature area first, so
  the Pass's acceptance criteria are grounded in actual Acrobat
  behavior before they're written down.
- Pass completion → engineer dispatches pdfce-librarian with
  completion details (summary, test results, packaging-smoke-test
  result) → librarian moves the entry to *Shipped* (top, reverse
  chronological) and appends a `SESSION_LOG.md` entry.
- Shipped entries are never rewritten. A reverted Pass gets a new
  "Pass NN — revert of Pass MM" entry, not a deletion.
