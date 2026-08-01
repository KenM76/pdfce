# fixtures/synthetic/redact — provenance

Synthetic, self-authored by `tools/gen-redact-fixtures.py`. No third-party
content (LEGAL §5). Regenerate with `python tools/gen-redact-fixtures.py`.

- `demo-secret.pdf` — two pages, each showing the literal word "SECRET" in
  a heading and a body line, with surrounding "PUBLIC" text that must
  survive a mid-line redaction in place. `/Info /Title` also carries
  "SECRET dossier" (the duplicate-carrier scrub target). Standard-14
  Helvetica (no `/Widths`), so advance widths come from the AFM tables.
- `demo-image.pdf` — one page whose only content is a raster image XObject
  positioned so a redaction region intersects it (the refuse-or-clear path).
