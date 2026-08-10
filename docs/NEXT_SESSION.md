# NEXT SESSION — start here

**Rewritten 2026-08-10.** Read this, then `docs/ROADMAP.md` and the latest
`docs/SESSION_LOG.md` entry. This is a *handoff*, not a record — the
record is the librarian's. Overwrite it once acted on.

Not owned by `pdfce-librarian`. Safe to edit without racing a filing.

---

## State

- Branch `pass-8-redaction`, HEAD **`2ffd808`**. Working tree clean.
- **2680 workspace tests, 0 failed.** clippy 0, `cargo fmt --all --check`
  clean, **all seven gates green**, `cargo tree` shows no GUI dep in
  core or render.
- **★ THE REPOSITORY IS PUBLIC.** `github.com/KenM76/pdfce`, since
  2026-08-09. Anything committed here is published by default. The
  `tools/` temp-folder convention for test files is now the actual
  control, not tidiness.
- **The publish deny-rules are gone**, by the operator's explicit choice,
  from both `.claude/settings.json` and `.claude/settings.local.json`.
  `gh release` and `cargo publish` are still denied. **Rule 8 now has no
  fence behind it — the absence of a deny rule is not a go-ahead.**
- Open question **(bh) is CLOSED**, resolved *accept*. Do not re-open it.

---

## ★ FIRST: rich-text fill is UNBLOCKED, and has been since 2026-08-06

The operator has a standing ruling on it, quoted in `b8f96b1`:

> *"it should be able to handle rich text fills if acrobat can or if it
> makes it better than acrobat"*

That commit (2026-08-07) says the work is *"blocked on §12.7.3.4 — still a
named GAP in the spec RAG."* **It is not.** Line 21 of
`D:\Dev\Rag-Specialized\PDF_Spec\iso32000\iso32000__s__12.7.3.4.md`
reads `GAP CLOSED 2026-08-06` — the day *before* the commit that calls it
open. Verified directly, not relayed.

So a feature the operator explicitly asked for has been sitting behind a
blocker that cleared four days ago, and the only record saying otherwise
was a commit nobody had filed. It surfaced on 2026-08-10 only because the
baseline debt was being repaid.

**What it is:** `<value-richtext>` (XFDF) and `/RV` (FDF) carry formatting
alongside the plain value. pdfce's `FieldData` carries
`values: Vec<String>` and drops it. Today's behaviour is a *disclosure*
only — the export and import reports say so when the document has a
rich-text field.

**Scope help already in hand,** from the Acrobat-parity RAG via `b8f96b1`:
Acrobat's rich-text support is enumerable (font family/style/weight/
stretch/size/colour, alignment, underline, strikethrough, sub/superscript,
via Adobe's `Span` object), **lists are unsupported even in Acrobat**, and
both FDF and XFDF genuinely carry the formatting. It also records three
places Acrobat is unreliable — an Adobe-acknowledged font-substitution
bug, an auto-size persistence interaction, and inconsistent `richValue`
across script contexts. Those are the openings for the ruling's second
clause. **Unverified against `Acrobat_Features` — that is a
`pdfce-acrobat-librarian` dispatch and is recorded as owed.**

## Also check CI — it is the thing nobody was doing

```bash
gh run list --repo KenM76/pdfce --limit 5
```

`ef88973` fixed a defect that made **pdfce uncompilable on Linux**, red on
six of six runs since the repo went public, found by reading CI rather
than any local signal. **The fix is committed here and NOT PUSHED**, so
`main` upstream is still broken. Pushing is permitted on request and is
the operator's call. Standing rule **R176**.

## What shipped this session

| | |
|---|---|
| `0466281` | Pass 52.2 GUI — Export DXF, disabled until a scale resolves |
| `c58cca1` | CLI `--pages`/`--output-dir`; paper-scale warning's third gate |
| `aa9ed38` | filing; **R174** minted |
| `a3ba0f8` | **Pass 53.0** — field rename in the Forms panel |
| `4f0e443` | `list-fields` stops mangling names with spaces |
| `269361d` | docs corrected: the repo is public; **R175** |
| `ef88973` | **Linux build fix**; Enter commits an Edit Text draft |
| `b9819b4` | filing; **R176**; decision 003 §1.1 superseded |

Ledger: Pass **53.1**, rules **R178** (next R179), decisions **035**
(next 036), questions **(bh)** closed (next (bi)).

**Owed-commit debt: 11 → 5.** `338076a`, `1f319c0`, `55a0732`, `587e520`,
`9141ded` remain in `tools/commits-filed-baseline.txt`. Repaying it is not
housekeeping — THREE of the six repaid so far were the only home of a live
obligation (an owed UX review that turned out already closed, a stale
blocker, and this ruling).

---

## THE NEXT TASK — ranked

### 1. The harness cannot locate a text run, so Enter-on-canvas is unverified

**This is the honest gap in `ef88973` and it is the first thing to fix.**

Enter now commits an Edit Text draft. The regression that mattered (the
Forms panel and rename editor still committing on Enter) **was** driven
live. The positive case — Enter committing on the canvas — was **not**,
because a script cannot find a text run to click, and R172 forbids
guessing coordinates.

What is missing, established by looking:

- `viewer::page_to_screen` exists; there is **no `pdf_to_canvas_space`**
  inverse of `canvas_to_pdf_space`, so a run's PDF bbox cannot be
  projected to a screen point.
- No `rect=` trace on the Edit-Text path.
- A `text:pick <run>` step that sets `state.caret` directly (the way
  `tool:` sets a tool) would **not** be enough on its own: typing is
  gated on `image_response.has_focus()`, and only a real click grants
  canvas focus. So the useful affordance is emitting run rects and
  clicking one — which solves focus and aim together.

Scope it as a slice. It unblocks every future canvas-text test, not just
this one.

### 2. Renaming a pure grouping node is not possible from the GUI

Filed as Backlog by the librarian. `form.fields` is a projection of
**terminal** fields (`walk_field` returns early on a pure non-terminal),
so `Personal` in `Personal.Address.Zip` is not a row and cannot be
renamed. Needs its own row source. Operator-visible symptom: dotted
names that cannot be fully edited, with nothing saying why.

### 3. `pdfce-ui-specialist` polish item 10

Extend Pass 47.3's hover-highlight to persist while a rename editor is
open. Small.

### 4. The eleven owed commits in `tools/commits-filed-baseline.txt`

Still DEBT, not an allowlist. Two were cleared this session by proper
filing; eleven remain.

---

## Things learned this session that will save the next one time

**Two documents asserted unmeasured facts about the environment, in the
sections written to warn about those facts.** `LEGAL.md` §1.1 said "no
git remote" hours after the repo was pushed; `decisions/003` §1.1 called
"pdfce has no git remote, CI has never run once" *the framing fact
everyone should read first*, and both halves were false. That is
**R175**. `git remote -v` and `gh run list` cost nothing.

**Three defects this session were invisible to green tests** and were
found only by looking at output as its audience (**R174**): a raw
seventeen-digit `f64` in a dialog, a CLI telling an operator who passed
`--scale 1` that it did not know the scale, and a rename message quoting
a `/TU` label that a rename does not change.

**Dispatch the librarian with `git show -s --format=%B <hash>` pasted
in.** Three for three this session. Two consecutive filings before that
had to leave a question open that three `git` invocations closed.

**`gui-shot.ps1` defaults 1760×1150 and `gui-drive.ps1` 1600×1000.**
Coordinates do not transfer. Aiming a shot from screenshot pixels hit
Delete instead of Rename this session — pass `-W 1600 -H 1000` to
`gui-shot` and use the traced `rect=`.

**The ui-specialist twice caught things I had wrong before they shipped**
— prefilling a rename editor from `/TU` instead of `/T`, and a global
`consume_key(Enter)` that would have silently broken every panel text
field. Both were changes I was about to make. Dispatch it for anything
touching input or disclosure.
