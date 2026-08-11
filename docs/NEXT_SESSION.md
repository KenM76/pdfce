# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** the librarian's record —
`ROADMAP.md` says what shipped, this says what is in flight and what the
next hour should be. Overwrite it once acted on.

Written 2026-08-10 at `f3dd8ff`.

---

## Verified state at `f3dd8ff`

Measured, not assumed:

- `cargo test --workspace` — 81 suites, 0 failures.
- `cargo clippy --workspace --all-targets --all-features` — 0.
- `cargo fmt --check` — clean. Theme-colour gate — clean.
- Portable build: `D:\builds\pdfce-20260810-2128-e61f075`.

`tools/check-commits-filed.py` was **not** clean when this was written —
`pdfce-librarian` was mid-sweep on an 11-commit backlog. **Run it first**;
if it still reports, that sweep did not finish and finishing it outranks
new work.

---

## ★ Start here: Reset Form

The next item on the forms gap list, and the one whose sourcing is
**already done** — do not re-dispatch for it. `pdfce-spec-librarian`
returned §12.7.5.3 verbatim this session:

> "Upon invocation of a reset-form action, a conforming processor **shall
> reset** selected interactive form fields to their default values; that
> is, it **shall set the value of the `V` entry** in the field dictionary
> **to that of the `DV` entry** (see Table 220). **If no default value is
> defined for a field, its `V` entry shall be removed.** For fields that
> can have no value (such as pushbuttons), the action has no effect."

Both branches are `shall`, and the second one is the interesting half:

- **`/DV` present ⇒ `/V := /DV`.**
- **`/DV` absent ⇒ `/V` is REMOVED** — a key deletion in the incremental
  save delta, not a value rewritten to empty. Those are different bytes
  and a different round-trip.
- **`/DV` is inheritable** (Table 220). Resolve up `/Parent` *before*
  choosing the branch, or a child with an inherited default takes the
  removal branch and loses it.
- Pushbuttons are unaffected; §12.7.4.2.2 says they "shall not use the
  `V` and `DV` entries" at all.

Two points ISO leaves open, registered in the spec RAG's ambiguity
register — decide them deliberately and disclose, per the project's
standing habit of making an unresolvable ambiguity a setting rather than
a hard-coded guess:

- **Exclude-mode descendant expansion.** Table 239's descendant
  parenthetical appears only in the *"if clear"* sentence — an asymmetric
  row.
- **Whether a reset re-runs the `/CO` calculation pass.** Now directly
  relevant, because posture B exists: after a reset, are the calculated
  fields recomputed or left at their reset values? Acrobat presumably
  recomputes. pdfce should probably offer the reset and the recompute as
  two visible steps rather than fusing them.

### Implementation notes

`EditSession` has no reset verb. The key-deletion pattern it needs
already exists — `crates/pdfce-core/src/edit.rs:11396` removes `/RV` in
the rich-text downgrade, and `:7427` removes `/V`/`/AS` — so this is a
new verb over an established mechanism, not new machinery.

Ship it the usual way: core verb, `pdfce-cli reset-form`, and a GUI
button in the Forms panel beside "Flatten form". **A reset is
destructive and must not be a bare button** — it discards everything the
operator typed. Follow the recompute's shape: say what it will clear
before clearing it.

---

## What just shipped, in case it matters to what you touch

**Posture B** (`crates/pdfce-core/src/form_script/`, 7 files, 75 tests) —
native recompute of recognised Acrobat calculation helpers, with no
JavaScript engine anywhere. CLI `list-scripts` and `recompute`; GUI
"Calculated fields" in the Forms panel.

Read `form_script/shape.rs`'s header before touching any of it. The
false-positive/false-negative asymmetry there is the whole safety
argument, and the recogniser must **never** grow into a JavaScript
parser — that is the first step toward posture C, which R57 prohibits.

**Poster imposition** got its CLI route, closing the R83 debt `163742a`
opened.

---

## Three corrections filed this session — do not re-derive the old versions

1. **Decision 009's "hollow shall" premise was false.** The two
   JavaScript references are in ISO 32000-1 **clause 3, Normative
   references**, not the Bibliography; the clause's own "(see the
   Bibliography)" is one of 8+ instances of a systematic erratum. The
   conclusion survives on the **invocation verb** instead — §12.6.4.16
   says the documents "give details on", never "shall conform to". So
   non-execution is a deliberate decision not to implement one clause,
   **not** a free win. Decision 009 §0 carries the retraction.

2. **`/CO` is §12.7.2 Table 218, and its order is normative** — the
   `shall` is in §12.6.3 Table 196's `C` row, not in Table 218, whose own
   wording reads advisory.

3. **§12.6.3 Table 196's `F` row PERMITS a format action to modify the
   field's value**, and NOTE 2 uses that exact case as its example.
   pdfce's "a format helper never writes `/V`" is an invariant declining
   spec-granted latitude. Never document it as compliance.

---

## Outstanding, roughly in value order

- **Reset Form** — above.
- **Forms authoring gaps**: border style is hard-coded Solid; no
  password/comb authoring; no `/F` visibility authoring at creation;
  no XML/CSV import-export; the GUI reaches only `tooltip` of ~30
  authoring properties.
- **Decision 037** — ruled, fixture built
  (`base-state-off-unregistered.pdf`), not implemented. The falsifier is
  opening that fixture in Acrobat Reader and checking the right-hand
  square.
- **Date/time format helpers** decline rather than render
  (`form_script/format.rs`). The token grammar is fully sourced and
  implemented-ready; what is missing is how Acrobat *parses* a stored
  date string back out of a field, which nothing sourced covers. Needs a
  conservative parser that declines on ambiguity, not a guess.
- **`sepStyle` 4 and `currStyle`** are unsourced and decline by name. A
  live-Acrobat spot-check would close both cheaply.
- **Colour management** — planned as its own project at `D:\Dev\iccce\`,
  which has a full plan and no code. Its `docs/NEXT_SESSION.md` is the
  entry point.

---

## The habit worth carrying

Three separate errors this session had one shape: **an assertion nobody
had measured, which read as settled for as long as nobody looked.** The
Bibliography claim survived in four documents. A doc comment claimed
`--poster --booklet` was mutually exclusive while nothing enforced it. A
montage made correct poster tiles look wrong because every tile was
scaled to the same size.

`git remote -v` costs nothing. So does running the thing.
