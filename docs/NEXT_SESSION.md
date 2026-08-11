# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** the librarian's record —
`ROADMAP.md` says what shipped, this says what is in flight and what the
next hour should be. Overwrite it once acted on.

Written 2026-08-11 at `23eee9b`.

---

## Verified state

Measured, not relayed:

- Full workspace `cargo test` — **81 suites, 3255 passed, 0 failed**.
- `cargo clippy --workspace --all-targets --all-features` — **0**.
- `cargo fmt --check`, `check-ui-strings.sh`, `check-theme-colors.sh` —
  clean.
- All four filing gates clean at `a64b5fd`: `check-commits-filed`
  (252 commits, whole history), `check-passes-filed`,
  `check-ledger-numbers`, `check-one-commit-per-command`.
- `23eee9b`'s own filing was dispatched; **run the gate first** and
  finish that filing if it still lists anything.

Portable build: `D:\builds\pdfce-20260811-0302-ce5642d` — predates the
CSV work. Rebuild before asking the operator to try that.

---

## ★ Start here: imposition has no GUI at all

`crates/pdfce-gui/src/print_flow.rs` contains **zero** references to
`n_up`, `booklet` or `poster` — verified by grep, not assumed. The whole
imposition family is CLI-only, and it is the largest single
capability-without-affordance gap left.

It is deliberately not started rather than half-started: three modes,
each with its own options (N-up's grid and border, booklet's binding
edge and subset, poster's scale, overlap, large-only and tile cap), and
the three are **mutually exclusive** — the CLI enforces that with a
guard added in `f3dd8ff` after `--poster --booklet` was found silently
discarding the poster. A GUI must express that exclusivity as a
*choice*, not as three independent checkboxes that can contradict each
other.

Everything it needs already exists and is tested: `pdfce_print::
imposition::{plan_n_up, plan_booklet, plan_poster}`, and `cmd_print`'s
own composition loops are the reference for how each mode turns a page
list into sheet bitmaps.

### Why this keeps happening, and the thing to do differently

**Three consecutive features landed in core and the CLI, then needed a
follow-up commit to reach the GUI** — the four authoring properties, and
CSV twice over. R83 covers "no affordance without capability"; this is
the mirror, and it recurs in one direction. When picking up any feature
below, plan the GUI surface in the same session as the core work.

---

## Other outstanding work, roughly by value

- **Encryption — `Pass 5`, and the operator asked for it.** pdfce still
  cannot open a password-protected PDF at all. §7.6 is now fully sourced
  in `PDF_Spec` (`iso32000__ref__encryption_impl.md` is the entry point).
  **One hard blocker:** ISO 32000-2 deprecates handler revisions 1–5,
  leaving `/R 6` the only non-deprecated one, and `/R 6`'s Algorithm 2.B
  is unsourced past step (a) behind the paywall. So there is **no
  conformant AES-256 write path** available today. AES-128 is fully
  sourced and writable. The spec librarian offered to triangulate 2.B
  from three permissively-licensed implementations and flagged that it
  would be the corpus's first normative algorithm sourced from *code* —
  **that needs Ken's go-ahead and has not been given.**
- **When encryption lands, do not miss this:** ISO 32000-2 exempts a
  signature's `/Contents` from encryption and **ISO 32000-1's
  never-encrypted list does not**. A writer following 32000-1 literally
  breaks every signature it touches, and it presents as a bad
  certificate rather than as an encryption bug.
- **Static hybrid XFA read/fill** — recommended increment; dynamic XFA
  stays out of scope. The staleness *disclosure* shipped (`ce5642d`);
  actually syncing the XFA half has not.
- **Wide-shape CSV** — one column per field, one row per form instance,
  for filling many copies from a spreadsheet. Filed as Backlog.
- **Remaining forms-authoring gaps** — the GUI reaches only a fraction
  of the ~30 authoring properties; four landed this session.
- **Decision 037** — ruled, fixture built
  (`base-state-off-unregistered.pdf`), not implemented. Falsifier: open
  that fixture in the installed Acrobat and check the right-hand square.
  Cheap, and it is the only thing gating the decision.
- **Colour management** — its own project at `D:\Dev\iccce\`, fully
  planned, no code.

---

## Live decisions worth not re-litigating

- **`R186`** (minted this session): when a guard keys on a marker, ask
  what the same hazard looks like *without* it. Two instances in one
  session — an `/Encrypt`-keyed refusal that a §7.6.7 wrapper walks past,
  and `fill_guards` never checking XFA.
- **A format helper never writes `/V`** — a pdfce invariant declining
  spec-granted latitude (§12.6.3 Table 196's `F` row explicitly permits
  it). Never document that as compliance.
- **`/CO` order is normative and pdfce follows it**, even where it looks
  wrong, because posture B exists to reproduce what another reader
  produces.
- **A reset does not re-run the `/CO` pass**, and **an ambiguous stored
  date is refused rather than guessed** (`03/04/2026`).
- **CSV values that look like formulae are neutralised and disclosed**,
  not refused and not silently written.

---

## Tooling worth reusing

`scratchpad/splice.py` — a source-splice helper that validates every
anchor before applying any substitution, refuses an ambiguous anchor,
and writes all-or-nothing. It exists because **six** edits this session
landed between an item and the attributes or doc comment above it, and
**twice** a script asserted out after printing "ok" but before its
single write, leaving the run looking half-successful and entirely
unapplied. It caught a four-way ambiguous anchor on first use.

If it is worth keeping, promote it out of the scratchpad into `tools/`.

---

## The habit worth carrying

Every expensive error this session had one shape: **an assertion nobody
had measured, which read as settled for as long as nobody looked.**

- Decision 009's Bibliography premise survived in four documents.
- A comment claimed `--poster --booklet` was exclusive while nothing
  enforced it.
- A rule number was cited from working notes rather than `ROADMAP.md`.
- A property test asserting token ordering was itself **backwards** —
  its failure argued for the wrong fix.
- An empty percent field's rendering was reproduced faithfully from a
  real source, disclosed as single-tier, and **still wrong** — a flag is
  a promise to check later, not a substitute for checking.
- A GUI screenshot showed a feature missing because the binary was
  launched while its own build was running.

Prefer running the thing to reasoning about it. When a new test fails,
check the test before the code. And when a gate overrules you, look
again before writing the exemption — one did this session, on a
judgement that would have been defended.
