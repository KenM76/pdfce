# NEXT SESSION — start here

Engineer-owned handoff (this filing written by `pdfce-librarian` at the
engineer's explicit request, per the dispatch brief for `Pass 5`
increment 3). Read this **before** the librarian's record — `ROADMAP.md`
says what shipped, this says what is in flight and what the next hour
should be. Overwrite it once acted on.

Written 2026-08-11, branch **`main`**, after commits `f79f044..f79d9a2`
(nine commits, `Pass 5` increment 3 — AES-256 read at `/R` 5).

---

## ★ Read this first: AES-256 now opens; `/R` 6 is the one gap left

`/V` 5, `/R` 5, `/CFM /AESV3` (AES-256) now decrypts across core, CLI and
GUI, at parity with RC4's and AES-128's own shell coverage — no new
shell-facing UI was needed, both already generalize over cipher.
Algorithms 3.2a/3.11/3.12/3.13 (Adobe Supplement ExtensionLevel 3 §3.5),
new module `crates/pdfce-core/src/crypto/r5.rs`.

**`/R` 6 is now the ONLY thing between pdfce and the common AES-256
case** — `/R` 6 is the default Acrobat X+ "AES-256" setting actually
produces, so it is plausibly the *common* real-world shape, not the
exotic one. The gap is exactly one function:

```
crates/pdfce-core/src/crypto/r5.rs — private fn hash
```

Its own doc comment names it as Algorithm 2.B's substitution point and
states everything AROUND it — the `/O`/`/U` layout, the `/UE`/`/OE`
unwrap, the `/Perms` check, the harness that calls it — is already
implemented and tested. **That is precisely the situation where filling
it from memory is most tempting and least detectable.** The refusal
fixture (`enc-aes-256-r6.pdf`) and the refusal tests exist specifically
to make that hard — do not remove or weaken either to "make progress."

**Routes to close it, unchanged from before this session, now the only
remaining item in this bucket:**
1. **ISO 32000-2 is $0.00 under PDF Association sponsored access** — but
   needs an account and a checkout. **This is the operator's act, not an
   agent's** — surface it to Ken rather than attempting a workaround.
2. Any other primary, citeable source for Algorithm 2.B (its inner
   AES-128-CBC step, SHA-256/384/512 selector, round count, termination
   condition) that isn't itself a derivation from another
   implementation's output — deriving from another implementation's
   *output* and then testing against that same implementation would be
   circular, which is exactly why `enc-aes-256-r6.pdf` is a
   refusal-only fixture today.

Once `/R` 6 is sourced, the remaining Encryption scope is: encrypt-on-
save (every cipher, every shell — entirely unstarted), and nothing else
new — `/R` 6 is genuinely the last read-side gap.

---

## Two new decisions this session, worth knowing before touching this code again

- **Decision 044 — a `/Perms` mismatch is REPORTED, never refused on,
  never silently substituted for.** `/R` 5 decoupled the file encryption
  key from `/P` entirely (Algorithm 3.2a has no `/P` dependency, unlike
  every earlier revision's Algorithm 2), so `/P` is editable in a hex
  editor without breaking a document's passwords, and `/Perms` — the
  only remaining integrity signal — is itself optional-in-effect
  (nothing re-derives it from `/P` at open time). `DocumentEncryption::
  perms` exposes `PermsCheck::{NotApplicable, MarkerMissing, Match,
  Mismatch}`; the GUI shows one conditional line only when the check
  ran and disagreed. **Never describe this as "security"** — `/P` is
  reader-convention enforcement, not cryptographic; this decision only
  narrows how a disagreement is surfaced, it does not change what `/P`
  ever actually protected.
- **Decision 045 — a non-ASCII `/R` 5 password is ATTEMPTED, never
  refused.** SASLprep (RFC 4013) is not implemented (no stringprep
  dependency taken); UTF-8 encoding and 127-byte truncation are exact.
  New `DocError::PasswordRequiresNormalisation`, raised only on an
  authentication FAILURE with a non-ASCII password at `/R` 5 — never on
  an all-ASCII password (SASLprep is the identity there) and never at
  `/R` ≤ 4 (SASLprep is `/R` 5-specific). The reasoning that makes
  "attempt, don't refuse" correct here is specific to `/R` 5's
  self-verifying authentication (SHA-256 either matches or it doesn't) —
  **do not generalize this "attempt then diagnose on failure" pattern to
  a context where a missing preprocessing step could produce a silently
  WRONG result instead of only a false failure.**

Full text, both: `docs/ARCHITECTURE.md` §12, hundred-and-tenth filing.

---

## The habit that caught this session's sharpest bug

A `strip_pkcs7` added to the `/UE`/`/OE` key unwrap passed **71 unit
tests, 20 end-to-end decrypts, a byte-identical render comparison, AND
qpdf's own published-key vector** — all of it, clean — because every
32-byte random key already in the corpus happened to end above the
valid-pad-length range (`1..=16`), which a uniformly random byte does
about 15 times out of 16. **A bigger corpus of real random keys would
not have caught this; only a DELIBERATELY CONSTRUCTED edge-case key
would.** Full finding:
`D:\dev\rag\rust\existing_fixture_of_the_right_shape_can_be_vacuous_for_a_new_measurement.md`
(4th instance). Worth carrying into `/R` 6 work and any future crypto
code in this project: when a branch's execution depends on a property
of RANDOM data crossing a threshold, build at least one fixture
deliberately on each side of that threshold — don't rely on a "realistic"
corpus to happen to hit it.

---

## What the operator can try

Latest portable build should be re-packaged before this is meaningful —
check `D:\builds\` for a build at or after `f79d9a2` before pointing Ken
at anything below; if none exists, `tools/package-portable.py` first.

- **`enc-aes-256-r5.pdf`** — now opens (previously refused by cipher
  name). `enc-emptyuser-aes-256-r5.pdf` — opens with no prompt at all,
  same as the RC4/AES-128 empty-user-password cases.
- **`enc-aes-256-r6.pdf`** — still refused, by cipher name, on purpose.
- Properties > Security — the permission bits section now shows a
  `/Perms`-mismatch line on any `/R` 5 file whose `/P` and `/Perms`
  disagree (none of the shipped fixtures currently exercise this; would
  need a hand-edited `/P` on an `/R` 5 file to see it fire).

CLI: `pdfce-cli --open-password userpw <cmd> enc-aes-256-r5.pdf` —
unchanged flag surface, now also reaches AES-256.

---

## Live decisions worth not re-litigating (carried from prior sessions, still current)

- **`R186` — SIX instances now recorded** (Standing rules, `ROADMAP.md`
  — full text there). Newest: a verification keyed on a marker (a `## `
  header) failing open when the same hazard arrives without the marker
  (`SESSION_LOG.md`'s hundred-and-ninth filing).
- **A limit set before there is a case to argue is worth more than one
  set during the argument.** `crypto/md5.rs` recorded in increment 1 why
  MD5/RC4 are hand-rolled AND that the reasoning does not extend to
  AES; increments 2 and 3 both honoured it without re-opening it — `aes`
  and `sha2` are both dependencies, decided once (decision 039),
  extended rather than re-litigated.
- **`DocError::PasswordRequired` is not a capability gap.**
- **A derived value with one producer cannot drift** (`149fd03`).
- **Decision 037** — `/BaseState /OFF` applies to registered groups only.
- **Decision 038** — cite **both** loci; `Table 101` is 1.7-only
  (ISO 32000-2 renumbers it to **Table 99**).
- **`EncryptionUnsupported::CipherNotImplemented` is UNREACHABLE** as of
  this session — pdfce implements all four of Table 25's `/CFM` values.
  Kept deliberately, documented in place, for whatever the standard
  adds next.

---

## Tooling — corrections that cost time in prior sessions, still true

- **`PDFCE_DIAG_VIEWPORT`**, not `PDFCE_VIEWPORT`. Four comma-separated
  numbers: `x,y,w,h`.
- **The diag script separator is `;`, not `,`.** A comma-separated
  script parses as ONE unparseable step, is silently skipped — the
  trace says `script-step-UNPARSEABLE`, read it.
- **`gui-shot.ps1` and `gui-drive.ps1` default to different window
  sizes.** Read the trace's own `rect=`, never a screenshot's pixels.
- **These two scripts move the REAL cursor and synthesise Ctrl+scroll
  and click-drag on the live desktop.** Say so before running one while
  the operator is at the machine; prefer headless verification (CLI,
  unit tests, `cargo test`) when it will do.

`tools/splice.py` — anchored substitution, all-or-nothing.
`tools/verify-release.py <tag>` · `tools/gen-encryption-fixtures.py`
(no arguments needed) · `tools/package-portable.py --note "..."`.

---

## Standing release authorisation (still in force)

The operator's 2026-08-11 instruction — *"please continue to post the
latest versions to git so I can try them on my laptop at home"* — is
ongoing. Rule 8's per-release ask does not apply to cutting a release of
THIS project: build it, tag it, publish the asset, run
`tools/verify-release.py`, report what went out. Scope is narrow:
authorises releasing pdfce builds for the operator's own testing, NOT
blanket publishing authority, NOT a licence to treat repository
visibility as an agent's own decision, NOT permission to skip
verification. `CLAUDE.md` rule 8's literal per-release wording is still
technically stale against this — flagged to the operator across two
prior filings, not yet amended by him; not this librarian's or the
engineer's file to edit.

---

## Open items, in the order they're likely to matter

1. **`/R` 6 sourcing** — see above. The only encryption read-side gap.
2. **Encrypted-save**, any cipher — entirely unstarted.
3. Two dead/stale printing items, filed to Backlog, deliberately not
   fixed: `DeviceSettings::pick_tray_by_page_size` sets no `DEVMODE`
   field at all; `build_devmode`'s doc claims a driver-default start
   the code doesn't actually do.
4. **Imposition has no GUI** — extract sheet composition into
   `pdfce-print` first so both shells share one implementation.
5. **No open operator questions** as of this filing — `(bj)` was
   answered and closed 2026-08-11; next free is `(bk)`.
6. Static hybrid XFA read/fill · wide-shape CSV · colour management
   (`D:\Dev\iccce\`, planned, no code).
7. **Ledger-accuracy defect, still not fixed** (carried from two
   sessions ago): filings ninety-two through ninety-five cite `(bh)`/
   `(bi)` as if `(bi)` had not been minted.
8. **Spec-librarian flag, still open**: confirm the eight-item
   never-encrypted list (E1–E9) is in the §7.6 corpus rather than only
   in pdfce's code.
