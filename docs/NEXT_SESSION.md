# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** the librarian's record —
`ROADMAP.md` says what shipped, this says what is in flight and what the
next hour should be. Overwrite it once acted on.

Written 2026-08-11 at `ed6db1c`.

---

## Verified state

Measured, not relayed:

- `cargo test -p pdfce-core --lib` — **1485 passed, 0 failed**.
- Full workspace at `7d2b71b` — **81 suites, 3209 passed, 0 failed**.
- `cargo clippy --workspace --all-targets --all-features` — **0**.
- `cargo fmt --check`, `check-ui-strings.sh`, `check-theme-colors.sh` —
  clean.
- `check-commits-filed.py` — **one** commit outstanding, `ed6db1c`, whose
  filing was dispatched. Run it first; if it still lists anything else,
  that dispatch did not land and finishing it outranks new work.

Portable build: `D:\builds\pdfce-20260810-2128-e61f075` — **stale**, from
before Reset Form and the date/time helpers. Rebuild before asking the
operator to try anything.

---

## ★ Start here: pick one of two

**Reset Form is DONE** (`Pass 61.0`) — do not start it, and ignore the
previous handoff if you have it cached.

### Option A — the forms authoring gaps

The largest remaining cluster, all reachable from the existing
`forms_author.rs`, none of them blocked on sourcing:

- **Border style is hard-coded Solid.** `/BS /S` admits `S`/`D`/`B`/`I`/`U`
  (§12.5.4 Table 166) and pdfce writes one of five.
- **No password or comb authoring** — `/Ff` bits 14 and 25.
- **No `/F` visibility authoring at field creation** — the annotation flags
  (§12.5.3) are readable and not settable.
- **No XML/CSV import-export.** FDF and XFDF exist; the two formats an
  office actually passes around do not.
- **The GUI reaches only `tooltip`** of roughly thirty authoring
  properties. That is an R83 debt on a surface the operator can see.

### Option B — the two live-Acrobat spot-checks

Cheap, and each closes an `UNSOURCED` marker that currently makes pdfce
decline something it could probably do:

- **`sepStyle` 4** — one reimplementation clamps the parameter to `[0,4]`,
  implying a fifth mode; no source of any tier says what it renders.
  `form_script/format.rs` declines by name.
- **`currStyle`** — described everywhere as reserved and inert. pdfce
  accepts and ignores it. A live check would either confirm that or find
  a behaviour nobody has recorded.
- While there: **`AFPercent_Format`'s bare `%` for an empty field** is
  single-sourced from one reimplementation and flagged as possibly its
  own quirk rather than Acrobat's.

Acrobat **Reader** is available on this machine; Pro is not. That is
enough for the percent-display check and probably not for the two
authoring parameters.

---

## What shipped since the last handoff

- **`Pass 61.0` — Reset Form** (§12.7.5.3). Core `reset_form` +
  `reset_preview`, `pdfce-cli reset-form`, GUI section. Core / CLI / GUI
  all three in one filing.
- **Date and time formatting** (`form_script/datetime.rs`, `ed6db1c`) —
  the last three declines in the posture-B format layer now render.

---

## Live decisions worth not re-litigating

- **A reset does NOT re-run the `/CO` pass.** ISO says nothing about it;
  fusing them would hide one operation inside another. Decided, filed.
- **An ambiguous stored date is refused, not guessed.** `03/04/2026` is
  two different days depending on country. `FormatOutcome::NotADate` is a
  distinct outcome from "unsupported helper" on purpose.
- **`/CO` order is normative and pdfce follows it**, even where it is
  arguably wrong, because posture B exists to reproduce what another
  reader produces.
- **A format helper never writes `/V`** — a pdfce invariant declining
  spec-granted latitude (§12.6.3 Table 196's `F` row explicitly permits
  it). Never document that as compliance.

---

## Outstanding, roughly in value order

- The two options above.
- **Decision 037** — ruled, fixture built
  (`base-state-off-unregistered.pdf`), not implemented. The falsifier is
  opening that fixture in Acrobat Reader and checking the right-hand
  square. Cheap, and it is the only thing gating the decision.
- **Poster has no GUI route** — CLI only. `print_flow.rs` has zero
  poster references. R83 debt, though a smaller one than the forms GUI.
- **Colour management** — its own project at `D:\Dev\iccce\`, fully
  planned, no code. Its `docs/NEXT_SESSION.md` is the entry point.

---

## The habit worth carrying

Every expensive error this session had one shape: **an assertion nobody
had measured, which read as settled for as long as nobody looked.**

- Decision 009's Bibliography premise survived in four documents.
- A doc comment claimed `--poster --booklet` was mutually exclusive while
  nothing enforced it.
- A rule number was cited from working notes rather than from
  `ROADMAP.md`.
- A property test asserting token ordering was itself **backwards** —
  reading its failure as "the code is broken" would have reordered a
  correct table into a broken one.
- A GUI screenshot showed a feature missing because the binary had been
  launched while its own build was still running.

Two of those were caught only because a lint fired on code that had not
been touched. Prefer running the thing to reasoning about it, and when a
test fails, check the test before the code.
