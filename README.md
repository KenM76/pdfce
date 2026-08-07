# pdfce

An open-source, non-monetized, feature-for-feature replacement for
**Adobe Acrobat Pro**.

The initial application is a native desktop GUI — no web server, no
browser runtime, no local network listener. It runs from a single
folder, dependencies included, no installer. It also ships **CLI
capabilities** (`pdfce-cli`) alongside the GUI: scriptable batch
subcommands (merge, split, stamp, convert, sign, validate) that Acrobat
Pro itself has no real equivalent for — see `docs/ARCHITECTURE.md` §7.
A later fork will turn the same core engine into a web application;
the architecture is deliberately structured today to make that fork
cheap (see `docs/ARCHITECTURE.md` §3).

## Status

**Pre-code.** As of 2026-07-23 this repository contains project
scaffolding and the agent roster that will drive development — no
Rust workspace exists yet. See `docs/ROADMAP.md` for the plan
("Pass 0 — Workspace bootstrap" is next).

## Stack

- **Language:** Rust
- **GUI:** egui/eframe (recommended default; confirm at Pass 0 — see
  `docs/ARCHITECTURE.md` §2.1)
- **CLI:** `pdfce-cli`, a first-class batch/scriptable command-line
  shell shipped alongside the GUI from the start (`docs/ARCHITECTURE.md` §7)
- **Design invariant:** the core PDF engine (`pdfce-core`) and headless
  rasterizer (`pdfce-render`) have zero GUI/windowing dependencies —
  this is what keeps a future WASM/web fork a "swap the shell crate"
  job instead of a rewrite, and is also what makes the GUI and CLI two
  independent front ends over one shared core.
- **Code style:** official Rust Style Guide (`cargo fmt`) + Rust API
  Guidelines, condensed reference at
  `D:\dev\rag\rust\rust-style-guide-and-api-guidelines.md`
  (`docs/ARCHITECTURE.md` §8).

## Documentation map

| Doc | What's in it |
|---|---|
| `docs/ARCHITECTURE.md` | Crate layout, core data model, round-trip invariant, packaging strategy, decision log. **The logic — read this before writing any code.** |
| `docs/ROADMAP.md` | Pass-by-Pass plan and history. Shipped / in-progress / next-up / backlog / standing rules. |
| `docs/LEGAL.md` | License (**MIT**, chosen 2026-08-01) — but **do not publish**: pushing to a public repo or cutting a release still needs an explicit operator go-ahead, which is a separate decision and has not been given. Also PDF-spec copyright/sourcing posture, patent/trademark notes, test-corpus sourcing rules, dependency licensing & attribution discipline, and the veraPDF MPL-2.0 election (§6.5). |
| `docs/PRIOR_ART.md` | Survey/decision record of existing open-source crates and tools pdfce depends on or learned from — what was adopted, what was reference-only, and why. |
| `docs/SESSION_LOG.md` | Append-only session-by-session record. |
| `.claude/agents/` | The project's engineer, librarian, PDF-spec RAG builder, Acrobat feature-parity RAG builder, and GUI specialist agents — see `CLAUDE.md` for how they fit together. |

## The reference RAGs

Two dedicated, LLM-optimized reference corpora inform pdfce's
development — both private development aids, outside this repo,
never shipped or committed (see `docs/LEGAL.md` §2 and each RAG's own
`LEGAL_NOTE.md`). Both are written for LLM consumption, not human
reading: dense, schema-consistent, no prose padding.

- **`D:\Dev\Rag-Specialized\PDF_Spec\`** — the PDF standard itself
  (ISO 32000, PDF/A, PDF/UA, PAdES, and embedded specs for fonts/
  compression/color), so byte-level parsing/writing is spec-correct,
  not guessed from fuzzy memory. Built/maintained by
  `pdfce-spec-librarian`.
- **`D:\Dev\Rag-Specialized\Acrobat_Features\`** — what Adobe Acrobat
  Pro's features actually *do* (capability, behavior, edge cases,
  limits) — explicitly **not** how its GUI is navigated. Grounds
  `docs/ROADMAP.md` acceptance criteria in real product behavior;
  pdfce's own UI is designed independently. Built/maintained by
  `pdfce-acrobat-librarian`.

## License

**Not yet chosen.** See `docs/LEGAL.md` §1. Do not treat this project
as publicly licensed or redistributable until that's resolved.
