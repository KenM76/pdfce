# pdfce — Project Instructions

An open-source, non-monetized, feature-for-feature replacement for
Adobe Acrobat Pro. Native desktop GUI first (no web server/browser
runtime), single-folder portable, Rust + egui/eframe, plus a
first-class CLI (`pdfce-cli`) for scriptable batch operations. See
`README.md` and `docs/ARCHITECTURE.md` for the full picture.

These instructions are read **at the start of every Claude session**
in this project. Everything below is binding. The global rules in
`C:\Users\Ken\.claude\CLAUDE.md` also apply (documentation-first,
claim-bearing-copy verification, personal_rag lesson-writing
discipline, etc.) — this file adds pdfce-specific rules on top.

## Project agents

This project has six agents under `.claude/agents/`:

| Agent | Role | When to dispatch |
|---|---|---|
| `pdfce-engineer.md` | Single-session lead engineer | The default role for any engineering work in this project. If you're the orchestrator, **be this agent** — read its file at session start, follow its discipline. |
| `pdfce-librarian.md` | Institutional memory: `ROADMAP.md` / `SESSION_LOG.md` / `ARCHITECTURE.md` decision-log keeper | Dispatched by the engineer for every new request (→ roadmap entry), every Pass completion (→ Shipped row), pre-compaction captures, and generalizable findings that graduate to `D:\dev\rag\rust\` / `D:\dev\rag\egui\` (ecosystem-wide) or `C:\personal_rag\pdf\` (PDF-domain). |
| `pdfce-spec-librarian.md` | Builds/maintains the PDF-standard reference RAG at `D:\Dev\Rag-Specialized\PDF_Spec\` | Dispatched whenever a spec question needs canonical sourcing (object model, filters, fonts, crypto, PAdES, PDF/A, PDF/UA), and self-directed for corpus-building sessions. |
| `pdfce-acrobat-librarian.md` | Builds/maintains the Acrobat Pro feature-parity RAG at `D:\Dev\Rag-Specialized\Acrobat_Features\` | Dispatched when scoping a `ROADMAP.md` Backlog bucket into a real Pass, so acceptance criteria match actual Acrobat behavior. Catalogs capabilities only — never Acrobat's GUI mechanics. |
| `pdfce-inkscape-librarian.md` | Builds/maintains the Inkscape feature-parity RAG at `D:\Dev\Rag-Specialized\Inkscape_Features\` | Dispatched when scoping the vector-editing Passes (Pass 9), so acceptance criteria match actual Inkscape behavior. Catalogs capability/behavior/limits only — never Inkscape's GUI mechanics; Inkscape is a behavioral reference only (GPL-2.0-or-later, never a dependency or code source — standing rule R61). |
| `pdfce-ui-specialist.md` | egui/eframe UX design + review | Dispatched by the engineer for non-trivial UI changes (new panel, new tool, an accessibility/discoverability judgment call). Returns critique + a change list; does not write code. |

The engineer agent file is the single source of truth for *how* work
happens in this project day-to-day. Read it before doing anything
substantive.

## Read first

- **`docs/ARCHITECTURE.md`** — crate layout, core data model, the two
  load-bearing invariants (GUI-core separation, round-trip/minimal-diff
  editing), packaging strategy. Read every session.
- **`docs/ROADMAP.md`** — the contract. Read every session.
- **`docs/SESSION_LOG.md`** — most recent entry, for what the prior
  session left in flight.
- **`docs/LEGAL.md`** — license status (undecided — don't publish),
  PDF-spec sourcing/copyright rules, test-corpus rules.

## Project-specific rules (binding)

### 1. Spec-fidelity discipline

Never implement spec-governed behavior (object-model byte layout,
filter algorithms, xref structure, font encoding, crypto handshakes)
from training-data memory. Check `D:\Dev\Rag-Specialized\PDF_Spec\`
first; dispatch `pdfce-spec-librarian` if the RAG doesn't yet cover
the question. Cite the ISO/ITU-T/ETSI clause in code doc comments.

### 2. GUI-core separation (load-bearing invariant)

`pdfce-core` and `pdfce-render` must never gain a GUI/windowing
dependency. Verify with `cargo tree -p pdfce-core` /
`cargo tree -p pdfce-render` on any Pass touching their `Cargo.toml`.
This is what keeps the eventual web/WASM fork a shell-crate swap
instead of a rewrite. See `ARCHITECTURE.md` §3.

### 3. Round-trip / minimal-diff editing

Objects pdfce didn't logically touch are re-emitted byte-identical
(full rewrite) or simply omitted (incremental save — the default save
mode). Redaction is the one deliberate, explicit exception: it must
truly remove covered content, not just visually mask it. See
`ARCHITECTURE.md` §5.

### 4. Fuzzy, never sneaky

Every algorithmic suggestion (OCR text, auto-detected form fields,
suggested Bates ranges) is a reviewable hint the operator accepts or
overrides — never a silent auto-apply. Inherited from the user's
MatExtractor project; same principle, new domain.

### 5. Roadmap discipline

Every new operator request → engineer parses into Pass entry/entries
→ dispatches `pdfce-librarian` to file under *Backlog*/*Next up* →
reports assigned Pass IDs back. Every Pass completion → dispatch the
librarian to move it to *Shipped* + append a `SESSION_LOG.md` entry.

### 6. Documentation-first

Per the global rule: every module gets a thorough file-level
docstring (purpose, contracts, spec citations); every function gets a
doc comment explaining WHY; the docs are the logic, the code is the
syntax. If a competent engineer couldn't rebuild the module from the
docs alone, the docs are incomplete.

### 7. Test-corpus sourcing

Fixture PDFs are synthetic or clearly rights-cleared only — never a
downloaded real-world PDF of unknown provenance. See `LEGAL.md` §5.

### 8. License is undecided

Do not commit to a public repository, publish a release, or describe
the project as "open source" in any user-facing copy until the user
has picked a license (`LEGAL.md` §1).

### 9. Cross-project knowledge bases

- `D:\Dev\Rag-Specialized\PDF_Spec\` — canonical PDF-standard reference
  (spec text/summaries with citations). Read-heavy; written by
  `pdfce-spec-librarian`.
- `C:\personal_rag\pdf\` — empirical, project-internal findings about
  how real-world PDFs (from Word, LibreOffice, Chrome's "print to PDF",
  scanners, etc.) diverge from the spec in practice. Distinct from the
  spec RAG the same way `personal_rag/solidworks` is distinct from
  `sw_api_docs` for the user's SolidWorks work. **New as of this
  project — doesn't exist yet on disk.** `pdfce-librarian` creates it
  (with an `index.md`, following `C:\personal_rag\README.md`'s
  template) the first time it has a real finding to file.
- `D:\dev\rag\rust\` — Rust toolchain/Cargo/packaging quirks that
  generalize to **any** Rust project, not just pdfce. **Already
  exists** (part of the existing Cross-project Tool RAG, registered in
  `C:\Users\Ken\.claude\CLAUDE.md`) — also holds the canonical
  `rust-style-guide-and-api-guidelines.md` reference (rule 10 below).
- `D:\dev\rag\egui\` — egui/eframe/wgpu findings, same cross-project
  scope as above. Already exists.
- `C:\personal_rag\claude_code\` — Claude Code tooling patterns.

### 10. Rust Style Guide + API Guidelines compliance

`cargo fmt --check` and `cargo clippy -- -D warnings` clean before any
Pass ships, workspace-wide. Any `pub` item added to `pdfce-core`
(or `pdfce-cli`'s argument/output surface) is checked against
`D:\dev\rag\rust\rust-style-guide-and-api-guidelines.md` — naming
conventions, trait derives, error-type design (`thiserror`, not
stringly-typed errors), documentation with runnable examples. See
`ARCHITECTURE.md` §8.

### 11. CLI capabilities (`pdfce-cli`)

pdfce ships a real, scriptable command-line interface alongside the
GUI — not a debug tool, a genuine parity-plus feature (Acrobat Pro has
no equivalent first-class CLI). Same crate-separation, round-trip, and
fuzzy-never-sneaky discipline as the GUI applies to every subcommand.
Default: each feature Pass ships its `pdfce-cli` subcommand alongside
the GUI flow, same session. See `ARCHITECTURE.md` §7 and
`ROADMAP.md`'s "CLI batch operations" backlog entry.

### 12. Acrobat feature-parity RAG (`D:\Dev\Rag-Specialized\Acrobat_Features\`)

Before scoping a Backlog bucket into a real Pass, dispatch
`pdfce-acrobat-librarian` so acceptance criteria reflect what Acrobat
Pro actually does. This RAG catalogs capability/behavior/edge-cases/
limits **only** — it must never describe or inform copying Acrobat's
GUI structure (menu paths, panels, dialogs); pdfce's UI is designed
independently by `pdfce-ui-specialist`. See `ROADMAP.md`'s "Feature RAG"
glossary entry and "Feature-fidelity discipline" standing rule.

### 13. Open-source dependency licensing & attribution

Before adding any Cargo dependency, classify its license (permissive /
weak-copyleft / strong-copyleft — see `LEGAL.md` §6.1) and check
`docs/PRIOR_ART.md`. Copyleft dependencies are always flagged to the
user, never decided solo — pdfce's own license is still undecided
(rule 8), and that decision gates what's even usable. Attribution is
**generated** via `cargo-about` into `THIRD_PARTY_LICENSES.md`, never
hand-maintained; regenerate it whenever the dependency set changes and
before any packaging pass. See `LEGAL.md` §6 and `ARCHITECTURE.md` §9.

### 14. RAG format philosophy: LLM-optimized, not human-readable

Every RAG this project builds or writes to (`PDF_Spec`,
`Acrobat_Features`, `D:\dev\rag\rust`, `D:\dev\rag\egui`,
`personal_rag/pdf` once it exists) is written for **LLM consumption
only** — dense, schema-consistent, grep-first. No narrative
scene-setting, no restating context an LLM already has, no prose
padding "for the reader." If a sentence doesn't add a fact a future
lookup needs, cut it. This is a standing instruction from the user
(2026-07-23), binding on every agent that writes to any of these RAGs.

## How a typical Claude session goes

1. **Read `docs/ROADMAP.md`** for current state.
2. **Read the most recent `docs/SESSION_LOG.md` entry** for prior context.
3. **Receive the operator's request.** Parse into Pass entries;
   dispatch `pdfce-acrobat-librarian` if scoping a new Backlog bucket
   (so acceptance criteria are grounded in real Acrobat behavior),
   then dispatch `pdfce-librarian` to add the entries to `ROADMAP.md`.
4. **Work the in-progress Pass**, consulting the spec RAG for any
   spec-governed behavior and dispatching `pdfce-ui-specialist` for
   non-trivial UI decisions.
5. **Ship the Pass**: tests green, `cargo tree` invariant verified,
   packaging smoke test run if packaging changed.
6. **Dispatch `pdfce-librarian`** to move the entry to Shipped and
   append a `SESSION_LOG.md` entry.
7. **Brief the operator** on what changed, what to try, what's next.

## Outstanding open items (surface these proactively when relevant)

- **`oxidize-pdf` (MIT) foundation-vs-scratch decision** — may already
  cover most of `pdfce-core`'s scope; needs a dedicated audit before
  Pass 1. See `docs/PRIOR_ART.md`'s "OPEN QUESTION" and `ROADMAP.md`
  Pass 0. Not yet decided — surface this before any from-scratch
  `pdfce-core` scaffolding begins.
- egui vs iced — not yet confirmed with the user (default: egui/eframe).
- OSS license — not yet chosen. Also gates whether AGPL/GPL prior art
  (MuPDF, Poppler, Ghostscript) is usable as a real dependency later —
  see `LEGAL.md` §6.1 and `PRIOR_ART.md`'s copyleft-landmine notes.
- XFA scope — verify Adobe's current XFA support/deprecation status
  before committing engineering time to it (see `ROADMAP.md` backlog).
- OCR engine binding — not yet decided. `PRIOR_ART.md` notes KillerPDF
  bundles Tesseract natively as a working precedent; OCRmyPDF's
  "sandwich" text-layer approach is the behavioral reference.
- Poppler's exact license (GPL vs LGPL) — unresolved in `PRIOR_ART.md`,
  re-verify before it matters to any decision.
- `pdfce-cli`'s exact subcommand surface — scoped incrementally,
  feature by feature (see `ROADMAP.md`); Pass 0 only needs the `clap`
  scaffold + a minimal `inspect` subcommand.
