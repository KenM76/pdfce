# realdrawings-smoke — private regression smoke-test over real CAD drawings

A **private, read-only, results-only** guard that runs the freshly-built
release `pdfce-cli render-page --page 1` over every `*.pdf` under the
operator's real drawings tree (`R:\Products`, = `D:\Stanley
Dropbox\Resource\products`) and aggregates the CLI's stdout diagnostic line
into a font/image/failure report.

It exists to prove the **subset-CIDFontType2 no-cmap TrueType** font bug (and
its `unsupported_*` kin) stays fixed on the files the operator actually uses,
and to surface the *next* real-world gaps.

## HARD RULES (LEGAL.md §5 + project test-corpus rule)

The drawings under `R:\Products` are **PROPRIETARY (TOP Steel confidential)**.

- Files are read **in place** — never copied into the repo or `fixtures/`.
- Each page 1 renders to a **single throwaway temp PNG** (system temp),
  overwritten per file and deleted at the end. **Pixels never enter the repo.**
- Only **diagnostics** are emitted (counts + filenames + the CLI's own line).
- `out/` is **gitignored** and must never be committed. Nothing in this
  directory is to be committed at all.

## Re-run

```bash
python tools/realdrawings-smoke/realdrawings_smoke.py
```

Options:

| flag | default | meaning |
|---|---|---|
| `--root <dir>` | `R:\Products` | corpus root (recursive) |
| `--cli <exe>` | `target/release/pdfce-cli.exe` | uses the **already-built** release exe; never triggers a build |
| `--cap <N>` | `0` (all) | scan at most N files; logs when the cap is hit |
| `--timeout <sec>` | `120` | per-file subprocess timeout (hang guard) |
| `--no-extract-text` | off | skip the `extract-text --pages 1` cross-check (faster) |

Outputs (both under `out/`, gitignored):

- `report.txt` — human-readable aggregate + prioritized outliers
- `results.json` — per-file records (machine-readable, for diffing runs)

The same report is echoed to stdout.

## What it reports

- **Load/render status** per file: `clean` / `dirty` / `load_fail` /
  `render_fail` / `timeout` / `no_line`.
- **Font-fix verification**: total `unsupported` plus the six-way by-reason
  breakdown. `unsupported_unusable_program == 0` across the corpus ⇒ the
  just-fixed class is gone (**PASS**).
- **Prioritized outliers**: (1) unsupported fonts, (2) load/render failures,
  (3) notdef glyphs, (4) unsupported image codecs/features, (5) tolerated
  oddities, (6) text-cross-check anomalies (rendered clean of fonts yet 0
  chars extracted).

## Coverage caveat

**Page 1 only** — a fast smoke, not a full-document scan. A clean result means
page 1 of every drawing renders faithfully; deeper pages are not covered. Widen
with a per-page loop if a full sweep is ever needed (slower).

## Design notes

- The diagnostic line is parsed **generically** (`key=<int>` pairs), so new
  CLI counters (its contract is append-never-reorder) are picked up without
  editing this harness.
- `substituted` and `need_appearances` are treated as **benign** (faithful
  fallback / document property), not gaps — so they don't mark a file `dirty`.
