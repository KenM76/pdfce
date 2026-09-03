# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**. What is
true now, plus a pointer. Corrections and their prior wording live in the
append-only record (`ROADMAP.md`, `SESSION_LOG.md`).

Written **2026-09-03**, at the end of a session that shipped **Passes 245.0,
246.0, 246.1 and 10.1**, released **v0.26.0 and v0.27.0** (v0.27.0 also on
GitHub — the first GitHub release since v0.17.0, on the operator's
instruction), and filed the operator's decision to **fork the project as
`pdfcer`**. Everything below was measured with a shell; commands are given so
nothing has to be trusted.

**For the ledger — Pass ceiling, rule ceiling, decision ceiling, filing count —
run `python tools/check-ledger-numbers.py`.** Do not mint from memory.

---

## §0 ★★ NEXT: THE FORK. `Pass 247.0` — clone to `D:\Dev\pdfcer`, strip the GUI

**The operator's rulings, verbatim, all on the record (397th filing, decision
128, question (cd)):** the product is **`pdfcer`** ("pdf-see-er": create,
edit, read); `pdfce` becomes its pre-release code name; *"Let's do it."*;
the GUI project renames to **`pdfcer-gui` at `D:\dev\pdfcer-gui`** in its
own session, by its own engineer. The full plan with acceptance criteria is
`ROADMAP.md`'s *Next up* head (`Pass 247.0` / `247.1` / `247.2`). In one
paragraph each:

1. **`247.0` — fork by clone, strip the in-repo GUI, green.**
   `git clone D:\Dev\pdfce D:\Dev\pdfcer` (a CLONE, never a fresh repo:
   2,040 cited commit hashes must survive, and the old folder becomes the
   untouched backup). In the clone: `git config core.hooksPath tools/hooks`
   (the sweep is red without it). Delete `crates/pdfce-gui`, drop it from
   `[workspace] members`, drop every dependency only it pulled (verify by
   `cargo tree` before/after), regenerate `THIRD_PARTY_LICENSES.md`
   (`cargo-about`). Delete the four gates that only read the GUI crate
   (`check-ui-strings.sh`, `check-theme-colors.sh`, `check-string-gaps.sh`,
   `check-disclosure-channel.sh`) with their CI steps and their
   `check-ci-parity.py` rows; strip the GUI branch from the five that also
   read other crates (`check-outcome-disclosed.py`,
   `check-settings-consumed.py`, `check-ci-job-names.py`,
   `package-portable.py`, `deploy-onedrive.py`). The *zero GUI deps* CI job
   STAYS — it is the invariant. `FEATURES.md`'s header and `ARCHITECTURE.md`
   §3/§6 lose the crate (librarian). Green = build, test, fmt, clippy,
   `run-gates.sh`, `cargo tree` GUI-free, and the test total recorded
   beside the pre-strip total (**5,114 / 0** at hand-off) so the difference
   is the GUI's own tests and nothing else. `ui_specs/` stays (history).
2. **`247.1` — the rename.** Mechanical, case-preserving: `pdfce-` →
   `pdfcer-`, `pdfce_` → `pdfcer_`, bare `pdfce` → `pdfcer`, over
   present-tense files only — **`ROADMAP.md`, `SESSION_LOG.md`,
   `ARCHITECTURE.md` §12 and `docs/decisions/` are history and are
   excluded.** CLI binary `[[bin]] name = "pdfcer"`. Crate directories
   renamed. `deploy-onedrive.py` → `pdfcer1`/`pdfcer2`;
   `verify-release.py` follows. Hard-rule-11 sweep for the claim "this
   product is called pdfce" in present-tense prose. **Coordination point
   with `pdfcer-gui`: agree the crate names (`pdfcer-core`,
   `pdfcer-render`) and both folders BEFORE either side switches its path
   dependency** — the channel note
   `note_the_engine_is_becoming_pdfcer_and_here_are_the_names_before_either_side_moves.md`
   already gives them the table; post the exact commit and the two
   `Cargo.toml` lines when `247.1` lands.
3. **`247.2` — publish, archive, release.** `gh repo create KenM76/pdfcer
   --public`; `git remote set-url origin`; push `main` **with `--tags`**.
   Archive `KenM76/pdfce` with a README pointer (one commit in the old
   folder — the only write it ever receives after the clone). First
   release under the new name **`v0.28.0`** (continuing the line, not a
   reset): tag → package → smoke → OneDrive `pdfcer1`/`pdfcer2` (old
   slots untouched) → GitHub release with the zip (decision 127) →
   `verify-release.py`. **Creating and archiving repositories are
   authorised for this Pass only** on the plan the operator approved —
   not standing.
4. The global `C:\Users\Ken\.claude\CLAUDE.md` references `D:\Dev\pdfce\`
   — flagged for the operator, not edited by an agent. A new folder gets a
   fresh auto-memory directory; the in-repo agent memory travels with the
   clone.

**Four hard-rule-11 survivors of `Pass 10.1`, owed in `247.0`'s first commit
(comments and one CLI string; the 398th filing found them):**
`crates/pdfce-cli/src/main.rs` — the `list-signatures` summary line still
says "no cryptographic verification" and its doc comment (search
`cryptographic verification`); `crates/pdfce-core/src/signature.rs:3`
("This module verifies nothing" — the sibling module now does) and `:145`
(names the stage `Pass 10.2`; it is `10.1`); `docs/DEPENDENCIES.md`'s
"implements itself" list lacks the six in-crate modules.

**Before the clone, confirm the tree is clean and pushed:** `git status
--short` empty, `git log --oneline origin/main..HEAD` empty. If not, commit
and push first — a clone carries only committed state.

---

## §1 SHIPPED THIS SESSION, in one table

| Pass | what an operator gets | commit |
|---|---|---|
| 245.0 | redaction destroys image samples (any codec), removes wholly-covered images, copies shared ones, RETAINS a mark over an undecodable image instead of refusing the document | `98d4377` (v0.26.0) |
| 246.0 | redaction CUTS vector paths at the region boundary; destroyed image cells are paper, not black; a pixel-level proof (`pdfce-render/tests/redaction_leaves_no_ink.rs`) | `194b3a1` (v0.27.0) |
| 246.1 | a shading under a mark is counted and disclosed (not cut yet) | `ff738a6` |
| 10.1 | **signature verification**: integrity (digest + CMS signature vs the signer's own cert) and coverage, trust NAMED as unchecked; RSA v1.5/PSS, ECDSA P-256/384, SHA-1/256/384/512; all in-crate, no new dependency; `pdfce-cli verify-signatures` | `22421b6` (unreleased — v0.28.0 is the first `pdfcer` release) |

Also: `tools/check-control-bytes.py` (a swallowed backslash fails the sweep),
`tools/hooks/pre-push` (R241: the three public-facing gates bound to `git
push`; `run-gates.sh` red when inactive), `verify-release.py` fails on a
missing GitHub release (decision 127).

---

## §2 QUEUED AFTER THE FORK

1. **`Pass 5.4` — encryption authoring, `/R` 6 only**: `EditSession::set_encryption`,
   `set_permissions`, `remove_encryption` (owner-auth refusal by name).
   `/R` 6 IS sourced (`PDF_Spec\security\security__aes256_r6.md`, since
   2026-08-12) — the crate's "not available in the spec corpus" strings
   (`crypto/standard.rs:177`, `UnsourcedRevision`, doc comments in
   `crypto/mod.rs`, `standard.rs`, `aes.rs`, `r5.rs`) are STALE and owed
   with it. Spec-side: `Contents` is never encrypted and the signature
   digest is over CIPHERTEXT (ETSI EN 319 142-1 §5.5) — the writer's
   exemption list must carry it. pdfceGUI's disclosure sentence for
   permissions is in the reply file, verbatim.
2. **pdfceGUI's consumption** of `reply_signature_integrity_first…` (the
   verify verb + panel wording) and `reply_images_are_destroyed…` (consumed
   2026-09-03 07:20 — archivable).
3. Mesh shadings deposit spot planes; the 8 unresolved conformance patches;
   `sh` cutting under a redaction mark; `set_page_tabs` when asked.

---

## §3 STATE OF THE TREE — verified 2026-09-03 at hand-off

- **Push state:** run `git log --oneline origin/main..HEAD`. Releases:
  v0.27.0 is tagged, on OneDrive `pdfce2`, and on GitHub as `Latest`
  (`pdfce-v0.27.0-windows-x64.zip`); `verify-release.py v0.27.0` clean.
  Commits after the tag (`Pass 246.1`, `10.1`, the gates, the filings) are
  unreleased by design — see §0.3.
- **Channels:** pdfce channel `open/` holds our unconsumed
  `reply_signature_integrity_first…` (with the SHIPPED section) and
  `note_the_engine_is_becoming_pdfcer…`; their two security requests stay
  open until they consume. iccce's
  `reply_all_four_asks_measured_and_your_bpc_would_have_done_nothing.md`
  is STILL unread by any pdfce session.
- **Disk:** `target/debug` reached 243.8 GiB and D: hit 0 bytes free
  mid-sweep this session; `cargo clean --profile dev` fixed it. A fresh
  clone starts empty — the first build is ~6 minutes per profile.
- **Test-total convention:** the sum of every `test result:` line of
  `cargo test --workspace` — **5,114 / 0**; no-default-features 3,503 / 0.
- **Backups:** `git bundle create /d/Dev/pdfce-backups/pdfce-<date>-<sha>-full.bundle --all`
  then `git bundle verify`. Last refreshed at `dfce8a9` this session; the
  clone itself is the fork's backup from here.

---

## §4 THINGS A NEW SESSION MUST KNOW BEFORE TOUCHING ANYTHING

- **★ A Python heredoc through the Bash tool eats one level of
  backslashes** — three recurrences this session, two caught by the new
  gate within minutes of writing it. Any patch containing ANY backslash
  goes through the `Write` tool to `D:\Dev\temp\<name>.py`, then
  `python <file>`; every patch asserts its anchor count.
- **A gate a human reads is not a gate** (R241): the pre-push hook runs the
  three public-facing gates; never chain `gate; other && git push`.
- **A test suite is not a pixel.** When the property is visual, assert on a
  raster in `pdfce-render/tests`; 22 byte-level tests agreed with a black
  block inside a transparent mark.
- **Verification's dependency posture** (decision 129): in-crate
  arithmetic and parsing are acceptable for VERIFICATION (no secret); a
  SIGNING implementation holds a private key, must be constant-time, and
  takes the audited dependency — the `aes` argument, not the `md5` one.
- **Stage by path. Never `git add -A`.** Push a code commit and its filing
  commit together. Read CI's colour from GitHub early:
  `gh run list --limit 5 --json status,conclusion,headSha`.
- **`docs/core-api/` is engineer-owned and moves in the SAME commit** as
  any `pub` change to the surface. `Pass 10.1` did (§12.5 of
  `01-reading-and-model.md`).

---

## §5 ★★ MEASURED NEGATIVES — DO NOT RE-DERIVE THESE

1. **Do NOT clear destroyed image cells to zero** — zero is black for
   Gray/RGB and a no-`/IC` mark is transparent; paper is the colour space's
   no-ink sample, `/Decode`-aware.
2. **Do NOT rewrite a `W`-marked path's geometry** — §8.5.4 applies the
   clip after painting; the cut paint goes first, the ORIGINAL construction
   stays as `W n`.
3. **Do NOT sign or verify over the `[0] IMPLICIT` tag** — the signed
   attributes are hashed as `SET OF` (`0x31`), RFC 5652 §5.4; sabotaging
   this fails every valid fixture.
4. **Do NOT treat `adbe.pkcs7.sha1` like `.detached`** — its content is the
   SHA-1 of the byte range, so `messageDigest = H(SHA1(D))`.
5. **Do NOT take the RustCrypto EC/RSA stack for verification** without a
   new decision: 25 crates, two pre-release, cfg-selected unsafe backends
   protecting a secret that verification does not have.
6. The prior hand-off's colour negatives (`ROADMAP.md` Passes 240.0–244.0)
   still stand.

---

## §6 ITEMS OWED BY THE OPERATOR

- **Open question `(cc)`** — four licence escalations from the `/R` 6
  sourcing (2026-08-12) that never reached the question list; conservative
  default filed; non-blocking for `Pass 5.4`.
- **Open questions `(ca)`, `(cb)`** — unchanged.
- **Global `CLAUDE.md`'s `D:\Dev\pdfce\` references** — his to edit after
  the fork.
