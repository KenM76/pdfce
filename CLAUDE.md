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

Anything pdfce **inferred** — a value, a boundary, a classification, a
correction the operator did not directly specify (OCR text,
auto-detected form fields, recognised text blocks, snapped points,
best-fit geometry, derived centrelines, reflow results, suggested Bates
ranges, substituted or synthesised fonts) — is **visible before it
becomes document state**, and the operator can reject it without undoing
anything else.

This is a requirement on **disclosure**, not on any particular widget.
It is satisfied by the inferred value being on screen and the commit
being a deliberate act — a key press, or a click on a control at a
fixed, predictable position. It is **not** satisfied by a control whose
position is derived from the document. And it does **not** require a
two-click confirmation for a direct manipulation whose result is fully
visible on the canvas and reversible in one undo.

Where an inference is *inherently* uncertain (a best-fit residual, a
font-trust downgrade, a reflow that overflows), the uncertainty is
stated in the disclosure, not merely implied by the presence of a
confirm button.

Inherited from the user's MatExtractor project; same principle, new
domain.

**Narrowed 2026-08-05** (decision 024 §4.4). The original wording said
every algorithmic suggestion "is a reviewable hint the operator accepts
or overrides", and that was being read as *every gesture needs an Accept
button*. Two things went wrong with that reading. It put a confirm step
in front of direct manipulations the operator had just performed and
could see — placing a dimension, typing a replacement — where undo is
the honest escape hatch and a second click is friction. And because the
confirm controls were positioned relative to the PAGE, they moved on
every zoom, scroll and page change, which is what the operator actually
reported: *"there is a separate accept / reject box somewhere on the
screen to click — I've never seen any other software operate that way."*
The complaint was placement, not the confirm step. The narrowing keeps
the obligation exactly where it was meant to be — on things pdfce
GUESSED — and takes it off things the operator did.

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

### 8. License is MIT; publishing still needs a go-ahead

**Corrected 2026-08-05.** This rule said the license was undecided and
that the project must not be described as "open source". That has been
wrong since 2026-08-01, when the operator chose **MIT** (`LEGAL.md` §1,
`ARCHITECTURE.md` §12). `LICENSE` exists at the repo root,
`license = "MIT"` is set on all four crates, and every dependency is
permissive — zero copyleft, verified against the generated
`THIRD_PARTY_LICENSES.md`.

What still holds, and is the part that matters day to day:

- **Do not push to a public repository or cut a release** without an
  explicit, current go-ahead. The license no longer blocks it; the
  operator's decision to publish is a separate act and has not been
  given. There is still no git remote configured.
- **GPL/AGPL prior art is categorically out** as a dependency or code
  source — MuPDF, Poppler, Ghostscript, Inkscape. An MIT project cannot
  link them (`LEGAL.md` §6.1). Inkscape stays a *behavioural* reference
  only (R61).
- `THIRD_PARTY_LICENSES.md` is generated by `cargo-about`, never
  hand-edited; regenerate it whenever the dependency set changes.

### 9. Cross-project knowledge bases

- `D:\Dev\Rag-Specialized\PDF_Spec\` — canonical PDF-standard reference
  (spec text/summaries with citations). Read-heavy; written by
  `pdfce-spec-librarian`.
- `C:\personal_rag\pdf\` — empirical, project-internal findings about
  how real-world PDFs (from Word, LibreOffice, Chrome's "print to PDF",
  scanners, etc.) diverge from the spec in practice. Distinct from the
  spec RAG the same way `personal_rag/solidworks` is distinct from
  `sw_api_docs` for the user's SolidWorks work. **Exists and is in
  active use** (created 2026-08-04; grep it before re-deriving anything
  about producer behaviour). Written by `pdfce-librarian`, following
  `C:\personal_rag\README.md`'s template.
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

### 15. Dimension terminology: "pdf dimensions" vs "ce dimensions" — never bare "dimensions"

Two entirely different things share the word *dimension* in this project,
and they have **opposite properties**. Always qualify which:

- **pdf dimensions** — dimensions already present in the PDF, exported by
  CAD or another authoring tool. Existing page content (or foreign
  annotations). pdfce reads them, measures against them, and must not
  silently alter them. The `55 5/8"` printed on a drawing is a *pdf
  dimension*.
- **ce dimensions** — the dimension objects **pdfce authors**: `/Line` +
  `/IT /LineDimension` annotations with a baked `/AP`, their groups, scale,
  `/Measure` dict and `/PieceInfo` sidecar. Everything under
  `crates/pdfce-core/src/dimension/`. Authored, editable, deletable,
  re-measurable — pdfce's own.

**Binding on every agent**, and on every reply, commit message, doc comment,
decision record, RAG entry and **subagent dispatch**. Dispatches especially:
a subagent handed the ambiguity writes an entire analysis in it, which is
exactly how it reached the operator.

**Why (operator, 2026-08-04):** he could not decode analysis that used
"dimension" throughout without ever saying which kind — and he named the
failure in *both* directions: ambiguous output is hard for him to act on,
and an ambiguous report *from* him can send troubleshooting down the wrong
path. This is a mutual-intelligibility rule, not a style preference.

When the operator says "dimension" unqualified, infer from context and
**echo back the qualified term**, so a mismatch surfaces before the work
rather than after it.

The distinction is **provenance**, not representation: a ce dimension is
still a ce dimension after save-and-reopen, and a pdf dimension does not
become a ce dimension because pdfce can see it.

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
- OSS license — **DECIDED: MIT** (operator, 2026-08-01; `LEGAL.md` §1,
  `ARCHITECTURE.md` §12). Consequence: AGPL/GPL prior art (MuPDF, Poppler,
  Ghostscript) is now categorically **off the table** as a dependency — an
  MIT project cannot link GPL/AGPL (`LEGAL.md` §6.1). Publishing/pushing is
  now unblocked by the license but still awaits an explicit operator go-ahead.
- XFA scope — **NARROWED 2026-08-03, still open; do not treat as
  closed.** The original item read "verify Adobe's current XFA
  support/deprecation status before committing engineering time to
  it." Three things have since happened, and this bullet is kept
  accurate rather than retired because retiring it is Ken's call
  (`ROADMAP.md` Open operator question **(p)**), not the engineer's:
  - **Demand is measured** (decision 008 census): `/XFA` in 2 of 2,500
    organic files (0.08%) and 4 of 2,914 conformance files. Negligible.
  - **Both authoring branches are now decided** (decision 020): dynamic
    XFA is `out_of_scope` — as of Acrobat 8.1+ it carries no AcroForm
    at all, so there is no Acrobat behaviour to match. Static-XFA
    *hybrid* field creation is **refused by name**, decided from
    pdfce's own capability boundary rather than from Acrobat's: pdfce
    can write the AcroForm half of a hybrid but not the XFA half, and a
    one-sided add would make an XFA-aware viewer and a plain viewer
    show different field counts for the same document.
  - **What is still genuinely unverified**: Acrobat's exact
    version-level deprecation date (only third-party approximate
    timing found, no Adobe-primary source).
  Net: the verification is **no longer a prerequisite for form
  *authoring*** — both branches were decided without needing it. It
  would still be a prerequisite for any XFA **read/fill** work.
  Decision 020 recommends re-scoping the item to exactly that; whether
  to accept that narrowing, or retire the item outright, is question
  (p) for Ken. See `ROADMAP.md`'s XFA backlog entry for the full
  amendment chain.
- OCR engine binding — not yet decided. `PRIOR_ART.md` notes KillerPDF
  bundles Tesseract natively as a working precedent; OCRmyPDF's
  "sandwich" text-layer approach is the behavioral reference.
- Poppler's exact license (GPL vs LGPL) — unresolved in `PRIOR_ART.md`,
  re-verify before it matters to any decision.
- `pdfce-cli`'s exact subcommand surface — scoped incrementally,
  feature by feature (see `ROADMAP.md`); Pass 0 only needs the `clap`
  scaffold + a minimal `inspect` subcommand.
