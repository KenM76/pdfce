# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** the librarian's record —
`ROADMAP.md` says what shipped, this says what is in flight and what the
next hour should be. Overwrite it once acted on.

Written 2026-08-11 at `17f4d82`, branch `post-v0.2.0`.

---

## Verified state

Measured, not relayed:

- `cargo test --workspace` — **3,311 passing / 0 failing**, measured at
  `17f4d82` (this commit).
- `cargo clippy --workspace --all-targets --all-features` — 0.
- `cargo fmt --check`, `check-ui-strings.sh`, `check-theme-colors.sh`,
  `check-ledger-numbers.py`, `check-passes-filed.py` — clean.
- `cargo tree -p pdfce-core` names no GUI crate. **Zero dependencies were
  added this session**, including for the crypto.

Portable build **`D:\builds\pdfce-20260811-0846-f5f5b06`** is current
at HEAD and carries everything described below.

Filing gate: `check-commits-filed.py` may list `17f4d82`; a librarian
dispatch was in flight when this was written. Run the gate and file
whatever it names before new work.

---

## What shipped: encryption, increment 1 (Pass 5)

**pdfce opens password-protected PDFs.** It could not, at all, before
this session.

| | |
|---|---|
| **Reads** | RC4 — `/V` 1, 2 at `/R` 2, 3; `/V` 4 at `/R` 4 with `/CFM /V2`. 40–128 bit. |
| **Refuses, by name** | AES-128 / AES-256 (unimplemented cipher) · `/R` 6 (**unsourced** algorithm) · `/V` 0, `/V` 3 (*no conforming reader* may open these) · non-`/Standard` handler |
| **Cannot** | write an encrypted document — **saving is refused in both modes** |

The operator-visible payoff is the **empty user password** case: a
permissions-only PDF opens with no prompt anywhere, because §7.6.3.1
requires trying it first and silently. That works in core, CLI and GUI
today with no flag.

New surface: `Document::load_with_password` /
`from_bytes_with_password` (`None` ≠ empty password), `doc.encryption()`
→ `DocumentEncryption { config, auth }`, `DocError::PasswordRequired`,
`DocError::Encryption(EncryptionUnsupported)`,
`WriteError::EncryptedSaveUnsupported`. CLI: global `--open-password` /
`--open-password-file`.

### The fidelity proof, and why it is the one that matters

Load tests prove a document *loads*. RC4 with a wrong key **does not
fail** — it yields bytes a lenient parser walks — so a transposed
50-round loop produces a document that opens and is subtly wrong.

`decrypting_reproduces_the_plaintext_document_exactly` renders the
plaintext source and all three RC4 encryptions of it and requires the
four PNGs byte-identical (`bc2dfede94ef290e7c7a7f7e509fea98` × 4).
**Keep that test.** If AES work breaks decryption, this is what says so.

---

## ✅ The GUI password prompt — BUILT (`94cf228`, `17f4d82`)

**Increment 1 is now complete across core, CLI and GUI.**

This section is kept as the record of *why* it is shaped the way it is:
`pdfce-ui-specialist` wrote no spec file, so this is the only durable
account of the design. What follows describes **what shipped**, not what
to do next.

Built: `Status::NeedsPassword`; the inline prompt (Enter submits, Escape
cancels via a new top rung on the ladder, show-password toggle,
wrong-password on the same surface with the field cleared);
`begin_save` refusing *before* the file dialog so Ctrl+S cannot carry an
operator as far as a file picker; both status-bar disclosures computed
live; Save disabled with a stated reason; and the read-only Properties
**Security** section with its non-enforcement caption.

**★ Start here instead: encryption increment 2 — AES-128** (below).

### The states — one new `Status` variant, not two

```rust
Status::NeedsPassword { path: PathBuf }   // NEW — that is the whole variant
```

- **Opens normally** → `Status::Open`, unchanged.
- **Opens, encrypted, no prompt** (`AuthKind::EmptyUser`) → still
  `Status::Open`. **No new variant, no new field on `OpenDoc`.** Read
  `doc.session.document().encryption()` **live, every frame**.
- **Needs a password** → the new variant. There is no `Document` at all
  when this happens, so it cannot be a flag on `Status::Open`.
- **Refused cipher** → `Status::Unsupported`, already correct as of
  `e4b6533`. `EncryptionUnsupported`'s `Display` text is already
  specific; `canvas_unsupported(path, message)` interpolates it.

Typed-input state (`input`, `show`, `rejected`) goes in a **separate
app-level `PasswordPrompt` struct**, sibling to `PendingCopy` — not
nested in the enum, which would mean mutating a `String` inside the
`Status` you are matching on.

### The prompt

- **Inline canvas arm, not a modal.** A fourth arm of the same
  `match &self.status` that already draws `Idle`/`Failed`/`Unsupported`.
  **Zero new dialog conventions** — there are already three.
- **Do NOT add it to `apply()`'s pending-gate.** That gate protects an
  *already-open* document's in-progress question. Here nothing is open,
  and gating would lock the operator away from the documents they *do*
  have open in the switcher.
- **Escape hatch is structural**: the switcher stays live, and
  `switch_to_parked` already discards `NeedsPassword` for free. Add an
  explicit **Cancel** anyway (→ `Status::Idle`) for the first-file-of-
  the-session case, plus Escape — but check the existing 4-way Escape
  precedence chain (~L13228) before wiring a new top rung.
- **Enter submits.** This is *not* the redaction-apply no-Enter case —
  that rule protects the app's one irreversible action; a password guess
  is reversible and routine.
- **Wrong password → same surface**, `rejected = true`, an
  `error_fg_color` line above the field, and **clear the input**. Never
  `rejected` on first arrival — the empty password was already tried
  silently, so claiming "wrong password" for something never typed is a
  lie.
- **Show-password checkbox**, default masked, `.password(!show)` — the
  idiom already exists at ~L18500. Tooltip should say pdfce **does not
  store the password**; nothing currently discloses that.
- **No retry limit, no lockout.** N6 — the spec has no error model here;
  inventing one is pdfce policy dressed as security.

### Disclosure, at open time

- One factual status-bar line when the open document is encrypted, naming
  the `AuthKind`. Computed **live**, never cached.
- **Disclose the save-refusal at OPEN, not at save.** An operator who
  edits for an hour and learns at `Ctrl+S` was let down by a UI that knew
  the whole time. Warn-coloured, same block.
- **Disable Save / Save As** with an explaining tooltip rather than
  leaving them clickable to fail. `WriteError::EncryptedSaveUnsupported`
  stays as the safety net.
- **Permission bits → Properties panel, read-only "Security" section**,
  never toggles. One pinned caption doing the N4 / §7.6.3.1 work: these
  are author-declared, PDF encryption does not enforce them, **and pdfce
  does not check or restrict anything based on them**. Do *not* repeat it
  at each action — that is a nag.

### Still deliberately not built

Re-encrypt-on-save UI · permission **editing** (the Security section is
read-only, and pdfce has no path to write a `/P` value at all) · attempt
limiting (**N6** — ISO 32000-1 states no error model here, so a counter
would be pdfce policy wearing the costume of a security feature) · an
owner-vs-user password chooser (`authenticate()` tries both and reports
`AuthKind` after the fact — one field, no selector).

---

## ★ Start here: encryption increment 2 — AES-128

Sourcing is done (`iso32000__ref__encryption_impl.md`). The plumbing
exists and is proven. What is genuinely new:

- **★ The dependency decision (rule 13) is RESEARCHED — read this before
  running `cargo add`.** The candidates are RustCrypto's `aes` + `cbc`.
  Measured 2026-08-11 by adding them, reading the resolved tree, and
  reverting (nothing is in the manifest now — do not go looking for it):

  | | |
  |---|---|
  | Versions | `aes 0.9.2`, `cbc 0.2.1` |
  | Transitive | `cipher`, `crypto-common`, `inout`, `block-padding`, `typenum`, `hybrid-array`, `cpufeatures`, `cpubits`, `cfg-if` |
  | Licences | **every one `MIT OR Apache-2.0`** — fully permissive, and note the contrast with `jpeg-encoder`'s `(MIT OR Apache-2.0) AND IJG`: there is **no** conjunctive attribution obligation here |
  | Copyleft in the resulting tree | none introduced |

  So rule 13's escalation trigger does **not** fire — but one thing
  does, and it is the reason this is written down:

  **★ R24's lever does not exist for `aes`.** Every codec dependency in
  this project is compiler-enforced zero-unsafe by keeping
  `default-features = false` (zune-jpeg, jpeg-encoder and
  hayro-jpeg2000 all turn a `simd` *feature* off). `aes 0.9.2` has
  exactly one feature, `hazmat`, and selects its x86/aarch64 intrinsic
  backends — 26 `unsafe` sites in `lib.rs` alone — on a **`cfg`**
  (`aes_backend = "soft"`), not a feature. A cfg cannot be set from
  `Cargo.toml`'s dependency line; it needs `RUSTFLAGS` or
  `.cargo/config.toml`, which is global and affects every crate in the
  build.

  That is a genuine departure from the project's established pattern
  and it should be a deliberate, recorded choice rather than a
  side-effect of `cargo add`. Three honest options: accept the
  hardware backend (RustCrypto is the standard, heavily-audited choice
  and the unsafe is confined to intrinsic dispatch); force soft
  globally and pay for it everywhere; or write AES-128-CBC decrypt
  in-crate — **which `crypto/md5.rs` explicitly argues against**, and
  that argument still stands. Recommend the first, recorded in
  `ARCHITECTURE.md` §12 with this paragraph as its rationale.
- **★ AES breaks the property this increment leaned on — but there is a
  clean answer, and it was verified.** RC4 preserves length exactly, so
  plaintext was written back over ciphertext in the retained buffer and
  every `ByteSpan`, `/Length` and provenance record stayed true. AES
  output is IV + padding, so **plaintext is strictly SHORTER** — by at
  least 17 bytes.

  Shorter is the easy direction. The plaintext still **fits** at
  `span.start`; what has to change is the recorded length. And
  `data_span` is what every reader actually slices — `content.rs`,
  `attachments.rs`, `document.rs`'s object-stream path, `edit.rs`
  throughout — so writing plaintext at `span.start` and setting
  `stream.data_span.len = plain.len()` makes AES work exactly as RC4
  does, with no change to `Stream` at all.

  Two things to check while doing it, neither yet verified: the
  stream dictionary's `/Length` will then disagree with `data_span`
  (harmless if nothing re-reads it after parse — confirm), and
  `attachments.rs:1379` compares `data_span.len` against a declared
  size, which under this scheme becomes a comparison against the
  plaintext length. That is arguably *more* correct, but it is a
  behaviour change and deserves a test.
- `/R` 5 (AES-256) is next after that. **`/R` 6 stays blocked** — its
  Algorithm 2.B is unsourced past step (a), and deriving it from another
  implementation then testing against that same implementation could not
  fail. `enc-aes-256-r6.pdf` is a **refusal** fixture on purpose.

---

## Other outstanding work

- **Imposition has no GUI at all.** Extract sheet composition into
  `pdfce-print` FIRST so both shells share one implementation. The three
  modes are mutually exclusive and a GUI must express that as a *choice*.
- **Static hybrid XFA read/fill** — the staleness disclosure shipped
  (`ce5642d`); syncing the XFA half has not.
- **Wide-shape CSV** — one column per field, for filling many copies.
- **Colour management** — `D:\Dev\iccce\`, fully planned, no code.
- **Ledger-accuracy defect** (librarian-reported, not fixed): filings
  ninety-two through ninety-five cite `(bh)`/`(bi)` as if `(bi)` had not
  been minted. Historical entries were left as snapshots; a future
  "index check" dispatch should add correction footers.
- **Spec-librarian flag**: confirm the eight-item never-encrypted list
  (E1–E9) is in the §7.6 corpus rather than only in pdfce's code.

---

## Live decisions worth not re-litigating

- **`R186` — now FOUR instances.** A guard keyed on a *marker* fails open
  when the hazard arrives without it. The `/Encrypt`-keyed refusal a
  §7.6.7 wrapper walks past · `fill_guards` never checking XFA ·
  `inspect` reporting a clean version line for an unloadable body · and
  now the GUI's `is_unsupported_structure`, which kept matching an error
  variant that had been **re-scoped underneath it**, so AES documents
  read as *damaged*. Nothing failed. No test went red. The guard just
  stopped covering its own case.
- **`DocError::PasswordRequired` is not a capability gap.** pdfce can
  open it and has not been told how. Calling it "unsupported" is the same
  untruth as calling it damaged, pointing the other way.
- **A derived value with one producer cannot drift** (`149fd03`).
  `recovery_note` was cached and *cleared* by two of three call sites,
  so a recovery banner vanished on document switch. Fixed by deleting the
  cache, not by adding a third reset — which would have fixed today's two
  and left the fourth waiting.
- **Decision 037** — `/BaseState /OFF` applies to registered groups only.
  The *literal* reading of the text is still correct as a reading; what
  is falsified is that any reader implements it.
- **Decision 038** — cite **both** loci; `Table 101` is 1.7-only (ISO
  32000-2 renumbers it to **Table 99**).
- **A format helper never writes `/V`** · **an ambiguous stored date is
  refused** · **CSV values that look like formulae are neutralised and
  disclosed.**

---

## Tooling

`tools/splice.py` — anchored substitution, validates every anchor before
applying any, refuses an ambiguous anchor, writes all-or-nothing. Use it
for the big files. **It caught a bad edit this session by refusing.**

`tools/verify-release.py <tag>` — clean tree, tag at HEAD, tag pushed,
**`origin/main` at the tagged commit**, release has an asset.

`tools/gen-encryption-fixtures.py` — no arguments needed now; source and
output default to committed paths.

`tools/package-portable.py --note "..."` — dated portable build in
`D:\builds`.

---

## What the operator can try in the build

`D:\builds\pdfce-20260811-0846-f5f5b06\pdfce-gui.exe`:

- **Open `fixtures\synthetic\encryption\enc-emptyuser-rc4-128.pdf`** — it
  just opens, no prompt. The status bar says it is encrypted and that
  saving is unavailable. Properties → Security lists the eight declared
  permissions and states that nobody enforces them.
- **Open `enc-rc4-128.pdf`** — the prompt appears. `userpw` or `ownerpw`
  both open it; anything else says so and clears the field. Escape or
  Cancel abandons it.
- **Press Ctrl+S on either** — refused with a reason, and no file dialog
  opens.
- **Open `enc-aes-128.pdf`** — refused *by cipher name*, not as damage.

CLI: `pdfce-cli --open-password userpw inspect <file>`, or
`--open-password-file <path>` (`-` reads stdin).

---

## The habit worth carrying

Every expensive error has one shape: **an assertion nobody had measured,
which read as settled for as long as nobody looked.** Four fresh
instances, all found by *using* something rather than reading it:

- The empty-password path had one fixture and it was **AES**, which is
  refused *before authentication is reached*. The most operator-visible
  behaviour in clause 7.6 was implemented, believed, and never once
  executed. **A fixture that cannot fail for the reason you care about is
  not covering that reason.**
- The fixture corpus was not reproducible — its plaintext source was
  never committed — so re-running the generator silently produced a
  *different* corpus. Found only because adding a seventh fixture made
  the other six change size, **one week after `PROVENANCE.md`'s own
  closing sentence warned about exactly that.**
- The `--open-password` sweep missed a load site that **was not a call**:
  `Document::from_bytes` passed to `.and_then()` as a function
  *reference*. `inspect` would have accepted the flag and ignored it.
- `recovery_note`'s comment argued its case correctly and emphatically —
  on the wrong axis. **A carefully-reasoned comment about the wrong axis
  is not a smaller error than no comment**; it reads as though the
  question was settled.

And the discipline that caught them: **prove a guard by making it fail.**
`verify-release.py` was run against a stale tag; the recovery regression
test was run with the bug deliberately reinstated. A test that has never
been seen to fail is a test nobody has tested.
