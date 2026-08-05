# Decision 029 — Keep ONE session over the whole workspace; do NOT partition sessions by crate

**Status:** **DECIDED — outcome is NO CHANGE.** Raised by the operator,
argued by the engineer, agreed by the operator, 2026-08-05.
**Date:** 2026-08-05 (session-close handoff, continuation 87)
**Requested by:** the operator, who proposed **three parallel sessions —
one per crate (`pdfce-core` / `pdfce-cli` / `pdfce-gui`)** — with the
stated goal of *keeping a usable GUI while features are being built*.
**Filed by:** `pdfce-librarian`, at the engineer's referral (*"Judge
whether this warrants an `ARCHITECTURE.md` §12 decision entry — I think
it does, since the NEXT person to notice the clean crate split will
propose the same thing"*).

**★ Why a decision record exists for a decision that changed nothing.**
A "no change" outcome with real reasoning behind it is the single easiest
thing to lose. The crate split is **clean, visible, and load-bearing**
(`ARCHITECTURE.md` §3, CLAUDE.md rule 2) — which is exactly what makes it
*look* like a work boundary to anyone seeing it for the first time. This
record exists so the next person to have the idea meets the argument
instead of re-running the experiment.

**★ Provenance, stated up front — same deviation from this directory's
protocol as decision 027.** `docs/decisions/README.md` describes these
files as `autonomous-builder` (KenAgent) consultant output. **This one is
not.** No consultant was dispatched; the exchange was operator ↔
engineer, and the librarian is filing it. The file exists anyway because
`tools/check-ledger-numbers.py` derives the live decision ceiling from
`docs/decisions/NNN-*.md` **only** — a decision recorded solely in
`ARCHITECTURE.md` §12 is **invisible to the checker**, and the next
session would be told `next free is 029` and could mint a colliding
number (R133's guard, R106's discipline).

**Decision number 029 — and a ledger correction it forces.**
Continuation 86's filing deliberately left **029 free**, so that a Pass
33.0 fix *"made as a design choice rather than a bug fix"* could take it.
**029 is now spent on this record**, so that Pass 33.0 record, if it is
ever written, takes **030**. The superseding note is appended to R148's
entry in `ROADMAP.md`. **`tools/check-ledger-numbers.py --stats` was run
after this file was written and reported `clean`**, with
`decision records: 029 -> next free is 030` and
`standing rules: R149 -> next free is R150` — so **029 is verified
unique, not assumed**. *(The checker is a pure Markdown parse of
`docs/`: it observes no git, backup or working-tree state, so running it
does not cross librarian hard rule 8's boundary — and no claim about the
repository, the index, remotes or the backup bundle is made in this
record.)*

---

## 0. Summary

| | |
|---|---|
| **Proposal** | Split development into three concurrent sessions, one per crate, so the GUI stays usable while core/CLI features are built |
| **Outcome** | **Rejected. Work continues as one session over the whole workspace.** |
| **Operator's stated reason for agreeing** | **He did not want to do anything that reduces the ability to catch errors.** |
| **The underlying goal, which is legitimate and unmet** | *"A GUI in a stable state I can use"* |
| **The better answer to that goal** | A **release build to its own folder** — already the project's packaging target — which decouples *"usable app"* from *"how many sessions are running"* entirely. **Operator declined for now; recorded as an available option, NOT as a decision.** |

---

## 1. The argument: a crate boundary is a DEPENDENCY DIRECTION, not a work boundary

`pdfce-core` → `pdfce-render` → (`pdfce-gui`, `pdfce-cli`) is a statement
about **what may reference what**. It is enforced, it is checked
(`cargo tree -p pdfce-core`), and it is what keeps the eventual web fork
a shell-crate swap rather than a rewrite (`ARCHITECTURE.md` §3).

**None of that makes it a partition of the WORK.** A feature does not
live in a crate; it **crosses** them, by design and by rule:

- **CLAUDE.md rule 11 MANDATES the crossing.** *"Default: each feature
  Pass ships its `pdfce-cli` subcommand alongside the GUI flow, same
  session."* A crate-partitioned setup makes the project's own default
  workflow into a cross-session coordination problem.
- **Measured on this session's own commits** (engineer's figures, relayed
  in the session-close dispatch; **not independently verified by this
  librarian, which has no shell**): the **substantive feature Passes were
  cross-crate** — **Pass 30.0 touched all four crates in one commit**,
  **Pass 30.1 touched two** — while the **single-crate commits were
  mostly fixes**. The distribution is the opposite of what the proposal
  assumes: the big work is wide, the narrow work is small.

**So the proposal would partition sessions along an axis that the actual
work does not follow**, and every feature Pass would immediately need
two or three sessions to agree on an API, a Pass ID and a filing.

---

## 2. The decisive argument: this session's three most valuable findings were all BETWEEN crates

This is the part that answers the operator's own criterion — *do not
reduce the ability to catch errors* — because each of these was found by
someone holding **two crates at once**:

1. **R144's second firing.** A **refusal removed in `pdfce-core`** (Pass
   30.0 lifting `RectangleNode` / `ImplicitNode`) **silently un-gated a
   `pdfce-gui` drag gesture that had been relying on it.** The protection
   lived in the **caller**, not in the module that changed. **This is
   R147's entire content** — *when a refusal is removed, audit its
   CALLERS.* In a core-only session there is nobody to audit the callers,
   and the drag would have shipped ungated.
2. **The clipping-path gap** — a core-side capability whose consequence
   lands on **page content the GUI operator may not be able to see**.
3. **The reflow auto-width defect (Pass 33.0 / R148)** — a **composition
   of two individually-correct operations**, `edit-text` and `reflow`,
   where **neither module's own tests can catch it** because each is
   correct by its own lights and the failure exists only in the seam.

**All three are seam defects.** A crate-partitioned setup **optimises for
exactly the blindness that produced them**: it gives each session a
complete view of one module and no view of the composition. The
proposal's cost is therefore concentrated precisely where the operator
said he was unwilling to pay it.

---

## 3. The ledger argument: concurrent sessions make number collisions routine

pdfce runs **four hand-maintained numbered ledgers** — Pass IDs, standing
rules, decision records, operator questions — and every one of them is a
**primary key** (`tools/check-ledger-numbers.py`'s own header says so).

- **Pass ID 31.0 was burned THIS SESSION** by *one* librarian racing
  *one* engineer: the librarian minted 31.0 from uncommitted work in the
  tree; the engineer assigned 30.1; the ID is retired, never reusable
  (hard rule 2).
- **Five collisions were found on 2026-08-03 alone**, which is why R106
  and the checker exist at all.

**Three concurrent sessions minting numbers against the same file would
make that the normal case rather than the notable one.** The checker
detects collisions **after** the fact; nothing prevents two sessions
reading the same ceiling seconds apart.

---

## 4. The single-writer argument: the docs are the highest-churn files in the repo

`ROADMAP.md` and `SESSION_LOG.md` are the **most-written files in the
project** — **5 of the last 12 commits** touched them (engineer's figure,
relayed in the dispatch; **not verifiable from here**). They are also
**append-only by rule**, over 16,000 lines, and edited by **one writer**
— the librarian — which is a **deliberate design choice**, not an
accident of how the agents happened to be set up.

Three sessions would mean three concurrent writers to those two files,
with every conflict landing in append-only history that is not supposed
to be rewritten.

---

## 5. The better answer to the operator's ACTUAL goal — recorded as an option, not a decision

The goal behind the proposal was *"a GUI in a stable state I can use"* —
which is a statement about **having a working build**, not about session
topology. Splitting sessions is an indirect and expensive way to get
that; **building a release to its own folder is the direct way**, and the
project **already targets it**: single-folder portable packaging is an
established requirement (`ARCHITECTURE.md` §6).

That decouples *"an app I can use"* from *"how many sessions are
running"* entirely — the usable copy sits in its own folder and does not
move when the working tree does.

**The operator declined this for now.** It is recorded here as an
**available option**, explicitly **NOT** as a decision, so that a future
session does not read this record as authorising a packaging task nobody
asked for.

---

## 6. What this record does NOT say

- It does **not** say the crate split is wrong or should be softened.
  The split is load-bearing and enforced (CLAUDE.md rule 2); this record
  is about **who works where**, not about **what depends on what**.
- It does **not** forbid a second session for genuinely independent work
  — the librarian, the spec-librarian and the feature-parity librarians
  already run as separate agents, and a **read-only** investigation is
  not a writer.
- It does **not** claim parallelism is unavailable. What it claims is
  that **the crate boundary is the wrong seam to parallelise on**, for
  this project, because the work crosses it by rule and the defects live
  in the crossings.

---

## 7. Cross-references

- `ARCHITECTURE.md` §3 (workspace layout / GUI-core separation), §6
  (single-folder portable packaging), §12 (dated entry for this decision).
- CLAUDE.md rule 2 (GUI-core separation), rule 11 (`pdfce-cli` ships with
  each feature Pass, same session).
- `ROADMAP.md` standing rules **R106** (read the live ceiling before
  minting a number), **R133**, **R144** (removing a refusal removes
  incidental protection), **R147** (audit the CALLERS), **R148** (an
  inference measured from state a prior edit moved).
- The burned **Pass 31.0** ID — Pass 30.1's Shipped entry, `ROADMAP.md`.
