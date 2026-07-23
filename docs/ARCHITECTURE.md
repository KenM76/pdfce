# pdfce — Architecture

This document is the logic. The Rust code is the syntax that enacts it.
Per the user's standing global rule: a competent engineer (human or LLM)
should be able to reconstruct pdfce's design from this file (plus
`ROADMAP.md`, `LEGAL.md`, the PDF-spec RAG, and the Acrobat feature-
parity RAG) without reading a line of code.

## 1. Project goal (verbatim framing from the founding conversation, 2026-07-23)

An open-source, non-monetized, full-feature-for-feature replacement for
**Adobe Acrobat Pro**. The initial application is a native desktop GUI
that does **not** rely on running a web server, a browser runtime, or
any local network listener — everything happens in one native process.
It must run from a single folder, including all of its dependencies
(no installer, no registry writes, no system-wide runtime dependency).

pdfce also ships **CLI capabilities** from the start (`pdfce-cli`, see
§3 and §7) — batch/scriptable operations (merge, split, stamp, convert,
sign, validate) invokable without opening the GUI at all. This is
addressed by the user (2026-07-23) as an explicit project requirement,
not just a developer convenience: Acrobat Pro itself has no equivalent
first-class CLI (only in-GUI Action Wizard batch sequences), so a real
CLI is a genuine parity-plus feature for anyone scripting document
workflows.

A **later fork** (not this codebase's job yet, but a design constraint
on this codebase **today**) will turn the same core logic into a web
application. Every architectural decision below is chosen to keep that
fork cheap when the time comes, without over-building for it now.

**Competitive/prior-art landscape confirmed clear** (see
`docs/PRIOR_ART.md`, researched 2026-07-23): no existing open-source
project, web or desktop, currently combines pdfce's full target
feature breadth in one native application. The closest attempts (Open
PDF Studio, KillerPDF) each have confirmed major gaps and neither uses
a native Rust PDF engine. This validates the project's premise.

### 1.1 Privacy posture (explicit, binding — not merely implicit in "no web server")

pdfce makes **no network calls of any kind by default**: no telemetry,
no usage analytics, no crash reporting, no update-check phone-home, no
license-verification callback. Every document a user opens is
processed entirely locally, in-process, with no data ever leaving the
machine unless the user explicitly initiates it themselves (e.g.
emailing a file — an action pdfce doesn't perform on their behalf
anyway). If a future feature ever genuinely needs network access (an
opt-in update checker, say), it must be **off by default and
explicitly opted into**, disclosed plainly in the UI and in
`README.md`, never silently enabled. This is a load-bearing part of
the project's value proposition ("not a web app" is a promise about
data handling, not just deployment topology) and is treated with the
same weight as the GUI-core-separation and round-trip invariants —
don't add a network call without flagging it to the user first.

## 2. Language & toolkit decision (made 2026-07-23, by the user)

| Decision | Choice | Why |
|---|---|---|
| Systems language | **Rust** | Single self-contained native binary (no runtime to bundle), memory safety for a file-format parser that will be fed adversarial/malformed input from the public internet, first-class WASM target for the future web fork, mature crate ecosystem for compression (`flate2`, `weezl` for LZW), fonts (`ttf-parser`, `allsorts`), image codecs, and crypto (`rsa`, `aes`, `sha2`, `rustls`-adjacent primitives) that pdfce will need anyway. |
| GUI toolkit | **egui + eframe** (recommended default — see §2.1) | `eframe` is egui's application shell and already targets **both native (winit+wgpu/glow) and WASM+canvas from the same codebase** — this is the single biggest lever for making the later web fork cheap. Immediate-mode fits a tool-heavy, many-panel editor (canvas + thumbnails + inspector + toolbars) well; prior art includes rerun.io and many CAD-adjacent Rust tools built the same way. |
| Rendering backend | `wgpu` (falls back to `glow`/OpenGL if needed) | Cross-platform, matches eframe's default, no separate native-toolkit dependency to bundle. |

### 2.1a — Toolchain pin & lockfile policy (Pass 0 task)

- **Toolchain**: pin a specific stable Rust release via `rust-toolchain.toml`
  at the workspace root, created at Pass 0. Don't float on "whatever
  stable is installed" — reproducibility matters for a project other
  people will eventually build. Bump deliberately (dated decision-log
  entry), not silently.
- **MSRV** (minimum supported Rust version): not yet decided — set it
  at Pass 0 once the toolchain is pinned, document it in `Cargo.toml`'s
  `rust-version` field, and re-check it against `docs/PRIOR_ART.md`'s
  candidate dependencies (some crates there have their own MSRV floors
  that could force pdfce's own MSRV higher than expected).
- **`Cargo.lock`**: **commit it.** This is an application workspace
  (produces `pdfce-gui`/`pdfce-cli` binaries), not a pure library —
  the Rust ecosystem convention for binaries is to commit the lockfile
  for reproducible builds, unlike libraries which typically don't.
  Don't `.gitignore` it.

### 2.1 — egui vs iced: still confirm at Pass 0

The user's decision was "Rust core + native GUI (egui/iced)" — the
specific pick between the two was left to engineering judgment. This
document recommends **egui/eframe** for the WASM-parity reason above.
**pdfce-engineer**: treat this as a strong default, not yet a closed
decision — confirm it explicitly with the user at the start of Pass 0
(the first real coding session) before the workspace is scaffolded,
since reversing it later means rewriting the entire GUI crate.

## 3. Workspace layout (Cargo workspace, to be created at Pass 0)

```
D:\Dev\pdfce\
  Cargo.toml                  <- workspace root, [workspace] members below
  crates\
    pdfce-core\                <- COS object model, tokenizer, xref (table + stream),
                                   object streams, incremental-update writer, filters,
                                   fonts, color spaces, encryption/decryption, digital
                                   signature verification, content-stream interpreter
                                   (produces a display-list / draw-op stream, NOT pixels).
                                   ZERO windowing/GUI/rendering-backend dependencies.
                                   THIS is the crate that forks to WASM later.
    pdfce-render\               <- Takes pdfce-core's draw-op stream + resources
                                   (fonts, images, color spaces) and rasterizes to an
                                   in-memory pixel buffer via `tiny-skia` (CPU-only,
                                   pure Rust, no GPU/windowing context — see
                                   docs/PRIOR_ART.md, resolved 2026-07-23).
                                   Depends on pdfce-core. Still GUI-framework-agnostic
                                   (no egui/eframe dependency) — a headless render
                                   (e.g. "render page 3 to PNG") must work with zero
                                   windowing system present, which is also what makes
                                   the eventual web fork (canvas-based rendering) and
                                   any future CLI/batch tooling possible.
    pdfce-gui\                  <- The native desktop shell. egui/eframe application,
                                   window chrome, file dialogs (rfd crate), menus,
                                   docking layout (egui_dock or hand-rolled), the
                                   `fn main()` entry point and packaged executable.
                                   Depends on pdfce-core + pdfce-render.
    pdfce-cli\                  <- The command-line batch shell. Subcommand parsing
                                   (clap crate), one subcommand per batch operation
                                   (merge/split/rotate/extract, Bates stamp, convert
                                   to PDF/A, sign, validate PDF/A or PDF/UA conformance
                                   and print a report, render-page-to-PNG for scripted
                                   thumbnailing). `fn main()` entry point, packaged as
                                   its own executable alongside pdfce-gui in the same
                                   single-folder distribution. Depends on pdfce-core +
                                   pdfce-render, same as pdfce-gui — ZERO GUI/windowing
                                   dependencies of its own, see §7. Doubles as a fast,
                                   windowless way to exercise pdfce-core in tests.
    pdfce-web\ (future, not     <- The web fork. Same pdfce-core + pdfce-render,
      built in this phase)         compiled to wasm32-unknown-unknown, eframe's
                                   web target, served as static files (no server-side
                                   PDF processing — everything still runs in-browser,
                                   preserving the "not a web app in spirit" privacy
                                   posture even in the fork).
  docs\                        <- This file, ROADMAP.md, LEGAL.md, SESSION_LOG.md
  .claude\agents\               <- pdfce-engineer, pdfce-librarian,
                                   pdfce-spec-librarian, pdfce-ui-specialist
  tests\                       <- Integration tests: parse→render→compare fixture PDFs
  fixtures\                    <- ONLY synthetic or clearly-licensed-for-redistribution
                                   test PDFs (see LEGAL.md §Test corpus sourcing).
                                   Never a scanned/downloaded real-world PDF of unknown
                                   provenance.
```

**Invariant (do not violate):** `pdfce-core` and `pdfce-render` must
compile with zero GUI/windowing crates in their dependency tree. This
is checked, not just hoped for — `cargo tree -p pdfce-core` and
`cargo tree -p pdfce-render` should never show `egui`, `eframe`,
`winit`, `wgpu` (a headless CPU rasterizer like `tiny_skia` in
`pdfce-render` is fine; a *windowing* dependency is not). This is the
single invariant that keeps the future web fork a "swap the shell
crate" job instead of a rewrite.

## 4. Core data model (target contract — implemented incrementally per ROADMAP)

This is what `pdfce-core` will expose once Pass 1+ lands. Written now
as the target so early implementation work has a north star; update
this section the moment the real API diverges (the doc is the logic —
if code and doc disagree, that's a bug in one of them, fix it same-day).

- `Document` — owns the COS object graph, trailer, xref, and the
  original byte buffer (for lazy/unmodified-object passthrough on
  write — see the round-trip invariant below).
- `ObjectId(u32 /* number */, u16 /* generation */)`
- `Object` — enum: `Null | Bool | Integer | Real | String(Str) | Name |
  Array(Vec<Object>) | Dict(Dictionary) | Stream(Dictionary, StreamData) |
  Reference(ObjectId)`
- `StreamData` — lazy: holds the raw (still-encoded) bytes plus the
  filter chain; decoding happens on demand and is cached, never eager
  for every stream in a large document.
- `Page` — resolved view over a page dictionary: `MediaBox`, `Resources`,
  content stream(s) concatenated, inherited attributes resolved per
  §7.7.3.4 of the spec (page tree attribute inheritance).
- `Document::open(path) -> Result<Document, PdfError>`
- `Document::save(path) -> Result<(), PdfError>` — full rewrite.
- `Document::save_incremental(path) -> Result<(), PdfError>` — **the
  default save mode.** Appends a new xref section + updated/new objects
  only; every object pdfce did not touch is left byte-identical in the
  file. This is not an optimization, it's a correctness requirement:
  Acrobat's own digital-signature model depends on incremental updates
  (a signature covers a byte range; anything after that range is a
  later revision). pdfce must support this from day one, not bolt it on
  after signatures are implemented.
- `Document::render_page(index, dpi) -> Pixmap` (in `pdfce-render`,
  takes a `&Document`).

## 5. Round-trip / non-destructive-editing invariant

Analogous to the tail-bytes / lazy-round-trip discipline the user's
other format-RE project (SWFormat) established for SOLIDWORKS files —
same principle, different format:

- Any object pdfce did not logically modify is re-emitted **byte
  identical** (for full rewrite) or **omitted entirely** (for
  incremental save, since the old bytes are simply not touched).
- Never "normalize" a PDF's internal structure as a side effect of an
  unrelated edit (e.g. don't silently rewrite every xref table to xref
  streams just because pdfce opened the file). Minimal-diff editing is
  a hard requirement — Acrobat users expect that adding one comment to
  a 400-page contract does not perturb the other 399 pages' bytes,
  and forensic/signature-validity expectations depend on it.
- Corollary: **redaction is the one deliberate exception.** True
  redaction must actually remove the covered content from the object
  stream (not just draw a black box on top) — see ROADMAP backlog
  item "Redaction — true content removal". This is a documented,
  intentional violation of the minimal-diff rule for exactly the
  objects the user asked to redact, and only those.

## 6. Packaging: single-folder portable

- No installer. Build produces `pdfce.exe` (Windows first target) plus
  whatever DLLs/assets are needed, all in one output folder.
- No registry writes, no `%APPDATA%` requirement for the app to run
  (per-user settings/recents may still use a conventional config dir,
  but the app must run read-only-folder-clean with no config present).
- Verify every packaging pass with a **real smoke test**: zip the
  output folder, unzip it to an unrelated path (e.g. a fresh temp
  dir), launch from there with no prior install step, confirm it
  opens and renders a fixture PDF. This is the packaging equivalent of
  MatExtractor's "smoke-import MainWindow" rule — don't claim a
  packaging pass done without actually running the copied folder.

## 7. CLI capabilities (`pdfce-cli`)

pdfce ships a real command-line interface alongside the GUI, not as a
debug afterthought. Design points:

- **Same crate-separation discipline as the GUI.** `pdfce-cli` depends
  on `pdfce-core` + `pdfce-render` exactly like `pdfce-gui` does, and
  is held to the same zero-GUI-dependency-in-core invariant (§3) — the
  CLI's existence is itself proof that invariant is doing its job:
  two completely different front ends, one shared core, no logic
  duplicated.
- **Subcommand shape** (`clap`-based, final surface scoped alongside
  each feature's own Pass — see `docs/ROADMAP.md`): one subcommand per
  batch operation, e.g. `pdfce-cli merge a.pdf b.pdf -o out.pdf`,
  `pdfce-cli extract-pages in.pdf 3-7 -o out.pdf`, `pdfce-cli
  bates-stamp *.pdf --start 1 --format "DOC-{:06}"`, `pdfce-cli
  to-pdfa in.pdf --level 2b -o out.pdf`, `pdfce-cli validate-pdfa
  in.pdf` (prints a conformance report, non-zero exit on failure —
  scriptable in CI/document-pipeline contexts), `pdfce-cli sign in.pdf
  --cert cert.p12 -o out.pdf`, `pdfce-cli render-page in.pdf 3 -o
  page3.png --dpi 150`.
- **Exit codes matter.** Since this is meant to be genuinely scriptable
  (unlike Acrobat, which has no real CLI), follow normal Unix
  conventions: `0` success, non-zero on any failure, with a specific,
  documented meaning per non-zero code where it's useful for a calling
  script to distinguish failure modes (e.g. "input not found" vs
  "encrypted, no password given" vs "PDF/A validation failed").
- **Same round-trip / redaction / fuzzy-never-sneaky invariants apply.**
  A CLI redact command must truly remove content, same as the GUI
  path (§5); a CLI OCR command's output is still a hint the caller
  chooses to apply, not silently baked into the saved file, unless an
  explicit `--apply` (or similarly unambiguous) flag says otherwise.
- **Packaging**: `pdfce-cli.exe` ships in the same single output
  folder as `pdfce-gui`'s executable — one portable folder, two
  entry-point binaries, both zero-install. The packaging smoke test
  (§6) covers both.

## 8. Code style & public API design

`pdfce-core`'s public API (and, downstream of it, `pdfce-cli`'s
argument/output design) follows the official Rust ecosystem
conventions, not an invented house style:

- **Formatting** — the Rust Style Guide, enforced via `cargo fmt`.
- **API design** — the Rust API Guidelines checklist (naming
  conventions, trait derives, error-type design, documentation,
  predictability, type safety).

Full condensed reference, kept up to date as a cross-project resource
(useful to any future Rust project, not just pdfce):
`D:\dev\rag\rust\rust-style-guide-and-api-guidelines.md`. This is a
binding engineering discipline, not a style preference — see
`.claude\agents\pdfce-engineer.md` §"Code style & API design
discipline" for the enforcement mechanics (`cargo fmt --check` and
`cargo clippy -- -D warnings` clean before any Pass ships).

## 9. Open-source dependencies & attribution

pdfce builds on the existing Rust/OSS ecosystem rather than
reinventing every primitive — see `docs/PRIOR_ART.md` for the
survey/decision record and `docs/LEGAL.md` §6 for the binding
licensing discipline (permissive-vs-copyleft classification, the
mandatory per-dependency license check, and why pdfce's own §1
license decision gates which prior art is even usable). Attribution
for whatever's actually adopted is **generated**, not hand-maintained
— `cargo-about` produces `THIRD_PARTY_LICENSES.md` from the real
`Cargo.lock`, regenerated at every packaging pass (§6).

## 10. Adversarial input hardening & fuzzing

`pdfce-core` parses files from the public internet by design — every
PDF it opens must be treated as **untrusted, potentially adversarial
input**, not just "possibly malformed." This is a real gap identified
2026-07-23: the project justified choosing Rust partly on this basis
(§2) but had never written down what that actually requires structurally.

### 10.1 Resource-limit guards (decompression-bomb defense)

Every filter decoder (`FlateDecode`, `LZWDecode`, `CCITTFaxDecode`,
`JBIG2Decode`, `DCTDecode`, `JPXDecode`) **must** enforce a maximum
decoded-output-size cap before/while decoding, not just check the
result afterward — a few KB of compressed input can expand to
gigabytes (classic zip-bomb pattern), and PDF's filter chaining
(e.g. `ASCII85Decode` → `FlateDecode` → raw image data) can compound
this. Concretely:

- Every decoder takes an explicit output-size ceiling (a sane default,
  overridable) and returns an error rather than continuing once
  exceeded — never silently truncate, never allocate unbounded.
- Object/dictionary nesting (page tree, `Kids` arrays, `Resources`
  inheritance chains, annotation appearance-stream references) needs
  cycle detection and a depth cap — a maliciously crafted circular
  reference must fail cleanly, not hang or stack-overflow.
- Content-stream interpretation (path construction, clipping, nested
  `Form XObject`/`q`/`Q` graphics-state pairs) needs an operation-count
  or time budget per page — pathological but syntactically valid
  content streams (e.g. millions of degenerate path segments) must not
  be able to hang the renderer indefinitely.
- Object counts / xref table size get a sanity ceiling too (a
  100 MB file claiming 500 million objects is lying).

### 10.2 Fuzz-testing (required, not optional, before Pass 1 ships)

Set up a `cargo-fuzz` target against the tokenizer/object-parser as
part of Pass 1 (add explicitly to its acceptance criteria in
`docs/ROADMAP.md` if not already there by the time Pass 1 starts).
Minimum scope for the first fuzz target: raw byte-stream → tokenizer
→ COS object parser, asserting only "never panics, never hangs past a
bounded timeout, never allocates past a bounded ceiling" — not
semantic correctness (that's what the fixture-based tests in §5/§9
cover). Expand fuzz targets to each filter decoder as they're
implemented. Treat any fuzz-discovered crash as a release blocker for
the Pass that introduced the vulnerable code path, not a "file it and
move on" backlog item.

### 10.3 Where this lives in the codebase

Guard logic (size ceilings, depth counters, timeouts) belongs in
`pdfce-core` itself — not bolted on as a wrapper in `pdfce-gui`/
`pdfce-cli` — so both front ends (and the future WASM fork) inherit
the same hardening automatically. Document each guard's default limit
and rationale in the doc comment of the function it guards, per the
documentation-first rule; a reader should understand *why* the number
is what it is (e.g. "1 GiB default output cap — larger than any
legitimate single decoded PDF stream this project has seen, small
enough that hitting it can't exhaust a typical machine's memory").

## 11. Undo/redo architecture

Identified as a real design gap 2026-07-23: the UI standing rule
"every edit is undoable" (see `pdfce-ui-specialist.md`) was never
reconciled with the round-trip/minimal-diff invariant (§5) — the two
interact in a way that needs an explicit mechanism, not just a UX
promise.

### 11.1 The core design: command log over the in-memory object graph, diffed at save time

- Every edit the user makes is represented as a small command object
  (`PendingEdit` or similar) with `apply()` and an inverse/`revert()`,
  operating on `pdfce-core`'s **in-memory** `Document` object graph —
  never on file bytes directly, and never on the saved file at all.
  The undo stack holds these commands.
- The **original loaded byte buffer / object graph is retained
  unmodified** as the "base revision" for the life of the open
  document (this is already required by §5 for lazy round-trip
  passthrough — undo reuses the same retained state, doesn't add a
  new one).
- Undo/redo operates **entirely pre-save**: hitting Undo reverts the
  in-memory graph via the command's inverse. It has no relationship to
  what's on disk until the user actually invokes Save.
- **Critical rule**: the "dirty set" (which objects actually differ
  from the base revision, i.e. what an incremental save must include)
  is computed as a **structural diff against the base revision at save
  time** — it is *not* the union of every object any command ever
  touched during the session. If a user edits an object and then
  undoes that specific edit before saving, that object must **not**
  appear in the incremental update, because compared to the base
  revision nothing net changed. Tracking "was this object touched
  by history" instead of "does this object currently differ from
  base" would silently violate the minimal-diff promise the moment
  undo is involved — this is exactly the subtle bug this section
  exists to prevent someone from introducing.
- Redo stack is invalidated (cleared) the instant a new edit is made
  after an undo — standard editor behavior, stated here for
  documentation-first completeness, not because it's subtle.
- Bound the undo history (a configurable max operation count) rather
  than keeping it unbounded — large documents with long editing
  sessions shouldn't accumulate unbounded command-object memory.
  Acrobat itself bounds undo; matching that expectation is fine.

### 11.2 Redaction is the deliberate exception, and only after save

Redaction's true-content-removal behavior (§5 corollary) is
undo-able **like any other edit, right up until the document is
saved**. Once a redaction has actually been written to disk (the
underlying content genuinely gone from the saved bytes), that save is
not reversible by "Undo" in a later session — there is no data left
in the file to restore. This matches real-world expectation (redact +
save = permanent) and is exactly why the UI standing rule requires an
explicit, honest confirmation dialog for redaction specifically
(`pdfce-ui-specialist.md`) — the operator needs to understand *before*
saving that this is the one edit type Undo can't rescue them from
after the fact.

### 11.3 Snapshot fallback for bulk structural edits

The command-pattern model is the default for content-level edits (text,
annotations, form fields, single-page operations). For bulk structural
operations where per-item commands would be awkward (e.g. reordering
50 pages in one drag operation), a coarser "before/after page-order
snapshot" command is an acceptable specialization of the same pattern
— still one undo-stack entry, still diffed against the base revision
at save time via the same mechanism. Don't invent a second, parallel
undo system for this case.

### 11.4 Scope for Pass planning

Read-only Passes (Pass 1) need none of this. **The first Pass that
introduces any editing capability must build the command-log/undo-
stack mechanism as part of that Pass, not after** — retrofitting undo
onto edit code that was written assuming direct mutation is
significantly more expensive than designing it in from the first edit
feature. Flag this explicitly when `docs/ROADMAP.md` scopes the first
editing Pass.

## 12. Decision log

Append-dated entries here whenever an architectural decision is made
or revised. Don't rewrite history — if a decision changes, add a new
dated entry noting the change and why; leave the old entry in place
with a forward pointer.

- **2026-07-23** — Project bootstrap. Language: Rust. GUI: egui/eframe
  recommended (confirm at Pass 0). Target parity product: Adobe
  Acrobat Pro. PDF-spec RAG location: `D:\Dev\Rag-Specialized\PDF_Spec\`.
  Agents created: pdfce-engineer, pdfce-librarian, pdfce-spec-librarian,
  pdfce-ui-specialist.
- **2026-07-23 (same-session amendment)** — Added `pdfce-cli` as a
  first-class crate (§3, §7) per explicit user request: pdfce ships
  CLI batch capabilities from the start, not just a GUI. Added Rust
  Style Guide + API Guidelines as a binding, cross-project-referenced
  engineering discipline (§8), backed by a new reference file at
  `D:\dev\rag\rust\rust-style-guide-and-api-guidelines.md`. Corrected
  the cross-project-knowledge-base plan: Rust/egui/wgpu findings
  belong in the existing `D:\dev\rag\rust\` / `D:\dev\rag\egui\`
  Cross-project Tool RAG (registered in the user's global CLAUDE.md),
  not a new `personal_rag/rust` subject — `personal_rag/pdf` remains
  correct as-is for PDF-domain-specific empirical findings.
- **2026-07-23 (same-session amendment 2)** — Added a second reference
  RAG, `D:\Dev\Rag-Specialized\Acrobat_Features\`, cataloging Adobe
  Acrobat Pro's feature set (capability/behavior/edge-cases/limits)
  to ground `ROADMAP.md` acceptance criteria — explicitly excludes
  Acrobat's GUI mechanics, since pdfce's UI is designed independently
  by `pdfce-ui-specialist`. New agent `pdfce-acrobat-librarian` owns
  it. Also established a binding cross-RAG format rule: every RAG
  this project builds or writes to (`PDF_Spec`, `Acrobat_Features`,
  `D:/dev/rag/rust`, `D:/dev/rag/egui`, the future `personal_rag/pdf`)
  is written for LLM consumption only, not human reading — dense,
  schema-consistent, no narrative padding.
- **2026-07-23 (same-session amendment 3)** — Added §9, open-source
  dependencies & attribution, per user request to survey existing OSS
  prior art and ensure proper crediting. New `docs/PRIOR_ART.md`
  (research/decision record) + `LEGAL.md` §6 (binding licensing
  discipline: permissive-vs-copyleft classification, per-dependency
  license check before any `Cargo.toml` addition, generated
  `THIRD_PARTY_LICENSES.md` via `cargo-about` rather than hand-
  maintained attribution). Three research passes launched to survey
  core PDF crates, supporting codec/font/crypto crates, and existing
  full OSS PDF tools; findings pending synthesis into `PRIOR_ART.md`.
- **2026-07-23 (same-session amendment 4)** — Research synthesized
  into `docs/PRIOR_ART.md`. **Open question flagged, not yet decided:**
  `oxidize-pdf` (MIT) may already cover most of pdfce-core's target
  scope — needs a dedicated audit (round-trip fidelity, signature-safe
  incremental saves, PAdES signing) before Pass 1 locks in a
  from-scratch `pdfce-core`. Confirmed: pure-Rust filter answers now
  exist for JBIG2/CCITT/JPX (`hayro-*` crate family) — the "problem
  filter" set from §7's plan is resolved. Confirmed gap: no Rust crate
  does signature-safe incremental saves or PAdES signing — build from
  `cms`+`x509-cert`+RustCrypto. `tiny-skia` selected as the
  `pdfce-render` rasterizer. MuPDF and Ghostscript flagged as AGPL-3.0
  licensing landmines — never link without a deliberate, user-
  confirmed decision. Competitive landscape confirmed clear (§1).
