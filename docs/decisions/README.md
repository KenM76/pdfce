# docs/decisions/ — Technical decision records

This directory archives the Markdown rationale for every non-trivial
technical decision made in pdfce, as produced by the **KenAgent**
decision consultant (the `autonomous-builder` agent).

## Protocol (established by Ken, 2026-07-30)

When the engineer faces a non-trivial technical decision (architecture
choice, adopt-vs-build, significant dependency selection, anything the
engineer agent file says to "raise with the user"):

1. The engineer calls the `autonomous-builder` agent with the question,
   the project path (`D:\Dev\pdfce\`), and full context (audit
   findings, constraints, the invariants at stake).
2. KenAgent returns the decision with full reasoning in **JSON** and
   **Markdown**.
3. The **JSON** drives implementation; the **Markdown** is saved here
   as `NNN-short-slug.md` (zero-padded sequential number, kebab-case
   slug, e.g. `001-oxidize-pdf-adopt-vs-build.md`).

## What does NOT go through this protocol

- **Legal decisions** (OSS license choice, patent-risk calls, copyleft
  dependency approval) — those are Ken's directly, per `docs/LEGAL.md`.
- Routine engineering calls already covered by `docs/ROADMAP.md`
  standing rules, `docs/ARCHITECTURE.md`, or the spec RAG.

## Relationship to other decision records

- `docs/ARCHITECTURE.md` §12 (dated decision log) remains the canonical
  index of decisions affecting architecture — entries there should
  cross-reference the `NNN-*.md` file here when one exists.
- `docs/SESSION_LOG.md` records *when* a decision happened in session
  history.

Files here are append-only history: never edit a decision record after
the fact — a reversed decision gets a NEW record that references the
old one.
