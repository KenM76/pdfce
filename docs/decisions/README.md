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

### ★ ONE NUMBER SPACE, AND GAPS IN THIS DIRECTORY ARE NORMAL

The decision numbers in `ARCHITECTURE.md` §12 and the filenames here are
**the same sequence**, not two. Decision 031 is `031-implicit-commit-
boundary-*.md` here and "decision 031" there, and there is no second
numbering to keep in step.

A file appears here only when the `autonomous-builder` consultant was
actually used. Decisions the engineer recorded directly in §12 — the
majority of them — are numbered from the same sequence and have **no
file here at all**. So this directory is expected to be sparse at the
top, and the highest filename is NOT the highest decision number.

Recorded explicitly on 2026-08-10 because it was mis-read as a defect:
a sweep found "33 numbered files, highest 033" while 034, 035 and 036
were cited as decided, and reported three decisions with nothing behind
them. All three exist in §12, in full. The convention was already
written above ("when one exists"); what was missing was any statement
that the shortfall is the NORMAL case rather than an accident, and a
count of files is not a count of decisions.

The next free number is therefore whatever §12 says, never
`ls docs/decisions | tail -1` plus one.
- `docs/SESSION_LOG.md` records *when* a decision happened in session
  history.

Files here are append-only history: never edit a decision record after
the fact — a reversed decision gets a NEW record that references the
old one.
