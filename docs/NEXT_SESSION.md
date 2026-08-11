# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** the librarian's record —
`ROADMAP.md` says what shipped, this says what is in flight and what the
next hour should be. Overwrite it once acted on.

Written 2026-08-11 at `b3ba63b`.

---

## Verified state

Measured, not relayed:

- Full workspace `cargo test` — **81 suites, 3256 passed, 0 failed**.
- `cargo clippy --workspace --all-targets --all-features` — **0**.
- `cargo fmt --check`, `check-ui-strings.sh`, `check-theme-colors.sh` —
  clean.
- All four filing gates clean as of `5e4a602`. **`b3ba63b`, `3fe8a19`,
  `ecf2302` and `04f8acd`'s follow-ups are NOT yet filed** — run
  `check-commits-filed.py` first and dispatch `pdfce-librarian` for
  whatever it lists. That outranks new work.

Portable build `D:\builds\pdfce-20260811-0349-23eee9b` predates the last
few commits. Rebuild before asking the operator to try anything.

---

## ★ Start here: encryption, and the increment order matters

The operator asked for it (2026-08-11) and pdfce still **cannot open a
password-protected PDF at all** — `xref.rs` refuses with
`XrefErrorKind::EncryptionUnsupported`. This is `Pass 5`.

**Everything needed to start is now in place except the code.**

### Sourcing — done, do not re-dispatch

`D:\Dev\Rag-Specialized\PDF_Spec\iso32000\iso32000__ref__encryption_impl.md`
is the entry point. §7.6 is fully covered: Algorithms 1–7 verbatim,
the padding string, `/P`'s little-endian unsigned hashing, crypt
filters, the never-encrypted list (**eight** items — the §7.6.1 bullet
list alone is incomplete, three live in other clauses).

**Read the transposition traps before writing a line.** They are
numbered `T4`–`T16` in that file and each produces a wrong key for
every document. The worst pair: Algorithm 2 step (h) truncates the
digest to `n` bytes **between** its 50 rounds; Algorithm 3's 50-round
loop does **not**. Two 50-round MD5 loops with opposite truncation
rules, three pages apart.

### Fixtures — done

`fixtures/synthetic/encryption/` — six files, `PROVENANCE.md` states
what each can prove. **Read that file before assuming they should all
pass.** `enc-aes-256-r6.pdf` is a **refusal** fixture: its algorithm is
unsourced, so testing a derived implementation against the
implementation it was derived from cannot fail. Passwords are `userpw`
/ `ownerpw`; `enc-emptyuser.pdf`'s user password is the empty string.

Regenerate with `tools/gen-encryption-fixtures.py` (needs `pypdf`).

### ★ The dependency question, which decides the increment order

**`pdfce-core` has NO crypto dependencies today** — `thiserror`,
`flate2`, four codecs, `jpeg-encoder`. Nothing else. So:

| Increment | Needs | Dependency |
|---|---|---|
| **1. RC4 modes** (`/R` 2, 3, and 4 with `/CFM /V2`) | MD5 + RC4 | **none** — both are small and frozen; implement in-crate |
| **2. AES-128** (`/R` 4, `/CFM /AESV2`) | AES-128-CBC decrypt | one permissive crate, or ~200 lines |
| **3. `/R` 5** | SHA-256 | permissive crate |
| **4. `/R` 6** | **BLOCKED** — Algorithm 2.B unsourced past step (a) | — |

**Do increment 1 first and ship it.** It needs no dependency decision
(rule 13), covers the commonest real-world protected PDFs, and proves
the whole plumbing — `/Encrypt` parsing, password authentication,
per-object keys, and the decrypt hook in the parser — against two
independent fixtures. Everything after it is a cipher swap behind the
same machinery.

Implementing MD5 and RC4 in-crate rather than adding crates is a
judgement, not a rule: both are frozen, tiny, and used nowhere
security-critical here (the file must be decryptable to be read at
all). Say so in the module header, and **do not** hand-roll AES on the
same reasoning — that one is worth a dependency.

### Two things that will bite later, recorded now

- **ISO 32000-2 exempts a signature's `/Contents` from encryption and
  ISO 32000-1's never-encrypted list does not.** A writer following
  32000-1 literally breaks every signature it touches, and it presents
  as a bad certificate rather than as an encryption bug.
- **`/P` permission bits are not a security boundary**, and the
  standard says so itself (§7.6.3.1: *"There is nothing inherent in PDF
  encryption that enforces the document permissions"*). Quote that
  sentence; do not file this half under "security".

---

## Other outstanding work

- **Imposition has no GUI at all.** `print_flow.rs` contains zero
  references to `n_up`, `booklet` or `poster` — verified by grep. Do the
  sheet composition extraction into `pdfce-print` FIRST so both shells
  share one implementation; duplicating `cmd_print`'s loops into the GUI
  is the drift this session kept flagging. The three modes are mutually
  exclusive and a GUI must express that as a *choice*.
- **Static hybrid XFA read/fill.** The staleness disclosure shipped
  (`ce5642d`); syncing the XFA half has not.
- **Wide-shape CSV** — one column per field, for filling many copies
  from a spreadsheet. Backlog.
- **Colour management** — `D:\Dev\iccce\`, fully planned, no code.

---

## Live decisions worth not re-litigating

- **`R186`**: when a guard keys on a marker, ask what the same hazard
  looks like *without* it. **Three instances now** — the `/Encrypt`-keyed
  refusal a §7.6.7 wrapper walks past, `fill_guards` never checking XFA,
  and `inspect` reporting a clean version line for a document whose body
  would not load.
- **Decision 037 — answered by measurement.** `/BaseState /OFF` applies
  to registered groups only. But say it precisely: the *literal* reading
  of the text is still correct as a reading; what is falsified is that
  any reader implements it. Acrobat and `pdf.js` both narrow the
  quantifier to the `/OCGs` registry, which the standard nowhere states.
- **Decision 038 — reconciles, and pdfce was already right.** Table
  101's own redundancy sentences hold under exactly one processing
  order. Cite **both** loci; citing only §8.11.4.5 b) makes the code look
  like it ignored the table. `Table 101` is a 1.7-only citation — ISO
  32000-2 renumbers it to **Table 99**. `pdf.js` diverges here, so a
  "match other readers" tiebreak would push *away* from the ruling.
- **A format helper never writes `/V`** — an invariant declining
  spec-granted latitude, never compliance.
- **An ambiguous stored date is refused**, not guessed.
- **CSV values that look like formulae are neutralised and disclosed.**

---

## Tooling

`tools/splice.py` — anchored source substitution that validates every
anchor before applying any, refuses an ambiguous anchor, and writes
all-or-nothing. Use it for edits to the big files.

`tools/gen-encryption-fixtures.py` — regenerates the encrypted corpus.

---

## The habit worth carrying

Every expensive error this session had one shape: **an assertion nobody
had measured, which read as settled for as long as nobody looked.**
Decision 009's Bibliography premise across four documents. A comment
claiming `--poster --booklet` was exclusive while nothing enforced it. A
property test that was itself backwards. An empty-percent rendering
reproduced faithfully from a real source, flagged as single-tier, and
still wrong.

And two from the other direction, worth naming separately: a subagent
reported a `SESSION_LOG` ledger as *duplicated* when it was
*misplaced* — acting on the report as written would have destroyed the
only copy. A `check-ui-strings` gate overruled a judgement that would
have been defended with an exemption comment, and the gate was right.

Prefer running the thing. When a new test fails, check the test first.
When a report proposes something destructive, measure before acting.
