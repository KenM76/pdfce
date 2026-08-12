# NEXT SESSION — start here

Engineer-owned handoff (this filing written by `pdfce-librarian` at the
engineer's explicit request, same as every prior overwrite of this
file). Read this **before** the librarian's record — `ROADMAP.md` says
what shipped, this says what is in flight and what the next hour
should be. Overwrite it once acted on.

Written 2026-08-12, branch **`main`**, at `aad48c7`, after four commits
`0947cab` · `c3a4d2e` · `9ea0c88` · `aad48c7` and the **`v0.5.1`**
release.

**★ AMENDED 2026-08-12 (hundred-and-twenty-fourth filing) at
`b1ee1cfa93541a081eb01286607a11e88a7839b2`**, after two further commits
`b902ea0` (formatting-coverage gate) and `b1ee1cf` (release verifier now
consults CI). **`HEAD` = `origin/main` = `b1ee1cf`, tree clean, 0 ahead /
0 behind — measured.** Sections **5**, **5b** and the backup section are
amended below; the release-state section carries a correction. The rest
stands.

---

## ★★★ CORRECTION TO THE PREVIOUS HANDOFF, READ THIS FIRST

The prior version of this file said `Pass 67.0` phase E's problem — a
Barnes & Noble Press upload requiring embedded fonts — *"is now
solvable end to end"*. **That was true of the core and FALSE of the CLI
until today.**

`pdfce-cli embed-font <file> --font-dir <dir> --apply` produced an
**empty plan on every file**: `resolved=1`, then `fonts=0 exact=0
substitute=0 refused=0 unmatched=0`, and `--apply` wrote nothing. The
cause was `clap`, not font logic — the `--font`/`--all-missing`
`ArgGroup` was never marked `required`, so a command naming neither
member parsed happily and requested nothing. `unembed-font` had the
identical hole. Both are now `.required(true)` (`c3a4d2e`).

Three things to carry forward from that, because they are the reusable
part:

1. **The sweep numbers were never wrong.** 1,330 of 1,507 missing fonts
   embedded (88.3%) and 726 of 4,023 files closed to `not-embedded=0`
   (18.0%) **stand and were re-verified reproducible today.**
   `tools/embed-sweep` calls the core API directly and hard-codes
   `EmbedRequest::all_missing()`, so it never went through the broken
   argument group. Do not re-measure them; they are correct.
2. **It survived a full green gate** (3,535 tests at the time) because
   **no test drove the CLI's own argument surface.** `R151`'s shape one
   level lower: not a capability with no caller, a capability whose
   caller has one untested invocation shape that silently no-ops. Two
   CLI-level integration tests now exist.
3. **`R191` was minted from the engineer's own verification error.** The
   first harness reported all five files fixed and had tested nothing —
   it read `not-embedded=N` off a summary line that **omits that token
   when the count is zero**, and defaulted the regex miss to `0`. When a
   CLI omits a token to mean zero, absence of the token and failure of
   the command are indistinguishable. **Assert on positive per-item
   evidence** (count `embedded=` per font line), never on a summary
   token's absence, and never let a harness default a missing match to a
   passing value.

**As of `v0.5.1` the end-to-end claim is now true, and was checked from
a fresh copy of the portable folder**, not from the build tree:
`FC60_Times.pdf` 1 missing font → 0, output reopened and listed three
embedded fonts. Five real-world PDFs verified overall — `document.pdf`
1→0, `data-000001.pdf` 2→0, `eu-001.pdf` 1→0 (needs `--mode full`,
recovered base), `FC60_Times.pdf` 1→0, `PDFBOX-2984-rotations.pdf` 1→0.

---

## Open items, in the order they're likely to matter

### 1. ★ Open operator question `(bk)` — bundled-font embedding licensing. Ken's call, still open, and now MORE load-bearing

**May pdfce's own bundled Base-14 substitute faces (BSD-3-Clause,
pdfium's Foxit-origin set) be embedded into an operator's document?**
Embedding puts the face inside a document the operator then
distributes — a different act from pdfce merely drawing with it on the
operator's own screen — and carries the licence's binary-redistribution
attribution condition once it travels inside someone else's PDF. **This
is a legal call, and therefore Ken's** — surface it, don't resolve it.

**Why it weighs more this session than last.** `embed-font` now
actually works from the shell, so the bundled rung is no longer a
theoretical donor: **on a machine without a suitable system font folder,
bundled faces are the only donor there is.** Bundled faces alone embed
1,250 of 1,507 corpus missing fonts (83.0%) and are the **only** donor
for `Symbol`/`ZapfDingbats` (16% of missing fonts). The difference
between an answer of yes and no is closing ~83% versus ~11% of the
real-world gap.

Shipped in the meantime, deliberately not a resolution: `pdfce-cli
embed-font --use-bundled-fonts`, **off by default**, help text states
the obligation. **The GUI does not offer the bundled rung at all.** Full
text: `docs/ROADMAP.md`, *Open operator questions*, `(bk)`.

### 2. `Pass 67.0` phases C, D and F — all unstarted, none blocking

Three of six phases have shipped (A reporting, B unembed, E embed-
missing). The request that opened the family is answered. This is now
open-ended Pass work, not urgent-fix work — **ask the operator which (if
any) he wants next rather than guessing an order.**

- **C — Re-subset.** Shrink an already-embedded font's program to only
  the glyphs the document actually uses. Lowest risk of the three: no
  visual change, no text loss, works on *every* embedded font including
  the ~13–48% (method-dependent — see `Pass 67.0`'s own Shipped entries
  for both measurements) that phase B must refuse. The right answer when
  the motivation is file size.
- **D — Convert text to outlines.** The universal escape hatch — the
  only phase that works where phase B is refused outright, because
  glyphs become vector paths and no font program needs to sit at those
  positions at all. Substantial to build; irreversible in effect
  (search/copy/reflow all stop working on the converted text) —
  disclose the cost inline, not merely via a confirm button (rule 4).
- **F — Replace font X with Y.** The hardest of the six — not just
  swapping a program name, but remapping encodings and widths so text
  does not reflow wrongly. Acrobat has **no equivalent** (searched
  across three sessions by `pdfce-acrobat-librarian`, recorded as a
  genuine absence) — parity-plus, not parity.

**Reusable substrate for all three, already built:**
`FontEnvironment::resolve_for_embedding`'s four-rung donor ladder,
`fontinfo::Removability`'s nine-verdict classifier, and
`font_embed_missing.rs`/`font_unembed.rs`'s shared-descriptor/shared-
program reachability code.

### 3. ★ `/R` 6 encryption — still parked at the operator's explicit instruction, but the sourcing blocker is GONE

Encryption stayed parked all session (*"put the encryption aside for now
to work on later"*). `/R` 6 remains the **only** read-side gap, and it
is exactly one function: `crates/pdfce-core/src/crypto/r5.rs`, private
`fn hash` — Algorithm 2.B's substitution point. Everything around it
(`/O`/`/U` layout, `/UE`/`/OE` unwrap, `/Perms` check, the calling
harness) is implemented and tested. **That is precisely the situation
where filling it from memory is most tempting and least detectable** —
the refusal fixture (`enc-aes-256-r6.pdf`) and the refusal tests exist
to make that hard. Do not remove or weaken either to "make progress."

**★ NEW, AND IT CHANGES THE ROUTE: the operator supplied ISO 32000-2 on
2026-08-12.** The prior handoff listed "acquire sponsored access" as
step 1 and the operator's own act. **He did it.** Algorithm 2.B is now
sourceable from the primary standard whenever encryption is un-parked.

**The file, path corrected — verified by `ls` on the directory this
filing, because the path relayed to the librarian was one level off:**

```
D:\Dev\Rag-Specialized\PDF_Spec\_sources\ISO_32000-2_sponsored_EC3.pdf
```

— **not** the top level of `PDF_Spec\`. 19,203,156 B, 1,023 pages, PDF
Association sponsored release, Errata Collection 3 (2026-06-01),
acquired via `https://pdfa.org/sponsored-standards/`. Registered in
`D:\Dev\Rag-Specialized\PDF_Spec\LEGAL_NOTE.md` (rows at lines 36 and
124–160) with `license_basis: licensed_primary_private_rag`.

**⚠ LICENCE, BINDING.** The copy is **watermarked to the operator by
name** and states **"Single user only, copying and networking
prohibited."** It must **never** be committed to pdfce, **never**
shipped, **never** a release asset, and never copied out of
`_sources\`. **Paraphrase and cite only** — a clause number and a
restatement, never the clause text. The repository is public
(`CLAUDE.md` rule 8): anything committed here is published by default,
so this is not a hypothetical exposure. Route spec questions through
`pdfce-spec-librarian`, which owns that corpus; do not lift from the PDF
into pdfce's own docs or code comments.

### 4. ★ GUI interaction model (`Pass 46.0`/`46.1`) — the operator's most recent GUI request, and it is NOT done

`0947cab` fixed the **pane latch only**: `pane_subject` was set by the
ribbon and nothing ever set it back, so one visit to Properties (or
Redact, Forms, Comments, Layers, Signatures, Fonts, Batch Tools) latched
the Tool Options pane for the session and every later tool-arm was
swallowed silently. That answers the operator's two reports — *"when I
click any tool button I'd expect its options to pop up in the side bar"*
and *"page properties is still showing permanently in one of the
windows"* — and **nothing more**.

**Still unstarted:** annotations joining the `CanvasTool` gesture
framework, selection-driven Tool Options, drag/resize of anything
carrying a `/Rect`. Do not read "tool options fixed" as "the GUI request
is done."

**Worth carrying from `0947cab`:** both doc comments already asserted
the behaviour — `PaneSubject::ArmedTool` said *"Arming a tool comes back
here"* and `Action::SelectCanvasTool` said *"Pass 34.1: arming a tool
raises Tool Options"*. Neither was true. **Reading the comment IS the
check a reader would run, and it agreed with itself.** Fourth instance
this session of a claim with no implementation. When a doc comment
states a guarantee, the pinning test is what makes it a guarantee.

### 5. ✅ RESOLVED by `b902ea0` — and the item that sat here was WRONG IN BOTH HALVES

**This slot previously read:** *"`cargo fmt --check` fails in
`tools/difftest` — 109 diffs, never covered."* **Do not act on that; it
was false in both of its stated facts, and it sat here for several
sessions.**

- **`tools/difftest` is CLEAN**, and appears to have been for some time —
  **0 diffs**. Measured by its total absence from `git show --stat
  b902ea0`: the commit that formatted every unformatted excluded crate
  did not change one byte of it.
- **The unformatted code was in nine OTHER crates** — 41 diffs over 8
  (corpus-report 1, roundtrip 2, content-identity 2, recover-sweep 2,
  render-profile 8, unembed-sweep 5, embed-sweep 4, fuzz 17), plus
  `tools/fontfile-census`, which was in **neither `members` nor
  `exclude`** and so was invisible to `cargo fmt --all`, to `cargo
  test`, and to every workspace-wide gate at once. **9 of 12 excluded
  crates (75%) carried unformatted code; this item named one of the 3
  that were already clean.**

**Why it could persist, which is the part to carry forward.** `cargo fmt
--all` formats **workspace MEMBERS**; every crate in `Cargo.toml`'s
`exclude` array is unreachable by it. **A coverage hole does not merely
permit drift — it lets WRONG BELIEFS about the drift survive, because no
command exists that would contradict them.** Nobody was careless; the
project had no instrument that could disagree with the figure.

**RUN THE COMMAND, DO NOT INHERIT A FIGURE FROM THIS FILE:**

```
python tools/check-fmt-excluded.py
```

It derives its target list from `Cargo.toml` at run time (a hard-coded
list would go stale the first time someone adds a sweep tool — this same
failure one level up), checks all **12** out-of-workspace crates, and
also flags any crate under `tools/` or `fuzz/` that is in neither
`members` nor `exclude`. It is wired into CI's `fmt` job and is **proven
to run on Linux CI**, not merely locally on Windows. Current state:
**clean, 12 crates** — coverage went **0 of 12 → 12 of 12**.

### 5b. ★ NEW, small and actionable — the CI job's name does not name the gate that failed

Every red X on a release commit renders in GitHub's checks list as
**`verify pdfce-gui strings live in ui_text.rs`**. That job
(`.github/workflows/ci.yml:257–322`) has accumulated **three unrelated
gate steps** under a name describing only the first:
`check-ui-strings.sh`, `check-disclosure-channel.sh`, and
`check-commits-filed.py`. **In all five red runs examined (`v0.5.1`,
`v0.5.0`, `v0.3.0`, `b902ea0`, `b1ee1cf`) the failing step was the
THIRD** — the filing gate — **and the job name said the first.** Anyone
reading the checks list is told a UI-strings violation blocked the
release. It did not, in any of the five.

Fix: rename the job to something honest (it is now a general
"project gates" job) or split the three steps into three jobs, so a red
X names its own cause. `R174`'s shape aimed at a job label rather than a
message.

### 6. Everything below is carried forward, unchanged in substance

- **Encrypted-save**, any cipher, every shell — entirely unstarted.
  `/R` 6 is the last *read*-side gap; writing an encrypted document is
  unimplemented in every configuration.
- Two dead/stale printing items, filed to Backlog, deliberately not
  fixed: `DeviceSettings::pick_tray_by_page_size` sets no `DEVMODE`
  field at all; `build_devmode`'s doc claims a driver-default start the
  code doesn't do.
- **Imposition has no GUI** — extract sheet composition into
  `pdfce-print` first so both shells share one implementation.
- Static hybrid XFA read/fill · wide-shape CSV · colour management
  (`D:\Dev\iccce\`, planned, no code).
- **Ledger-accuracy defect, still not fixed:** filings ninety-two
  through ninety-five cite `(bh)`/`(bi)` as if `(bi)` had not been
  minted.
- **Spec-librarian flag, still open:** confirm the eight-item
  never-encrypted list (E1–E9) is in the §7.6 corpus rather than only in
  pdfce's code.

---

## ★ Take a backup — but the figure this file carried was WRONG, and the gap is minutes, not days

**★ CORRECTED 2026-08-12 (hundred-and-twenty-fourth filing), by `ls -la
D:\Dev\pdfce-backups\` run at correction time — not by re-reading this
file.** The paragraph here previously said the newest bundle was
`pdfce-20260807-2039.bundle` (2026-08-07 20:39) and that it predated
"the whole of `Pass 67.0`". **A fresh bundle had in fact been taken
today and neither this file nor the previous filing knew it.**

**Measured:**

- Newest bundle: **`pdfce-20260812-1100.bundle`**, 12,530,418 B,
  **2026-08-12 11:00**.
- Newest full artefact: `pdfce-full-20260808-0956.zip`, 183,439,240 B,
  **2026-08-08 10:01** (unchanged, and this half was correct).

**A bundle is still owed, but the gap is 2 commits over 8–11 minutes**
(`b902ea0` 11:08, `b1ee1cf` 11:11), **not the five days this file
implied.**

**This is the exact failure the librarian's hard rule 8 was amended to
prevent — a backup figure inherited from a document rather than read off
the disk — and it is the SECOND time this ledger has carried a wrong
one.** `ls` costs nothing. Run it; do not quote the number above without
re-running it, including when the number above is this one.

---

## Release state — `v0.5.1`, verified

**Measured this filing** (`git ls-remote --tags origin`,
`gh release view v0.5.1`, `git status --short`):

- Tag `v0.5.1` → `aad48c73…`, **equal to `HEAD`**; working tree
  **clean**; `origin` = `https://github.com/KenM76/pdfce.git`.
- Release *"pdfce v0.5.1 — embed-font works from the CLI"*, created
  2026-08-12T14:41:54Z; asset `pdfce-v0.5.1-portable-win64.zip`,
  **10,244,728 B**. ~~`tools/verify-release.py v0.5.1` reports clean.~~

  **★ CORRECTED 2026-08-12 (hundred-and-twenty-fourth filing) — that
  "clean" was the whole problem, and `b1ee1cf` is the fix.**
  `verify-release.py` reported clean **while the tagged commit's CI run
  was RED**. Every check it made was true — tag exists, at `HEAD`,
  pushed, `origin/main` contains it, asset present — **because not one
  of those facts is about whether the code passes.** It was verifying
  that the bookkeeping was self-consistent. The tool now consults CI and
  **`v0.5.1` FAILS the new check.**

  **The released CODE is fine and that is proven, not argued:** `git
  diff --name-only aad48c7 68408f1` returns **only `docs/` paths**, and
  `68408f1` passed CI fully. The red X on the tag is purely
  `check-commits-filed` — three commits were tagged and released before
  the librarian filed them. **Binaries fine, ordering wrong.**

  **★ THE ORDERING RULE, and it is now enforced by the tool: FILE, LET
  CI GO GREEN, THEN TAG.** Run `tools/verify-release.py <tag>` **before**
  tagging, not after. History measured this filing: **3 of the last 4
  releases (75%) were tagged at a commit CI had rejected** — `v0.5.1`
  and `v0.5.0` on the filing gate only, `v0.4.0` green, and **`v0.3.0`
  on `cargo test` + `cargo clippy` + the `aarch64-apple-darwin`
  cross-check simultaneously — a published release whose tests did not
  pass.**

  **Why `v0.3.0` was invisible, and why it will be again without CI:**
  those jobs run on `ubuntu-latest`, macOS and `wasm32` while
  verification happens locally on **Windows**. **Cross-platform breakage
  is invisible to local gate runs by construction.** No amount of local
  diligence closes that; consulting CI does.
- **The version bump came BEFORE the tag, deliberately.** `--version`
  now prints `CARGO_PKG_VERSION`, so tagging `v0.5.1` on a binary
  answering `0.5.0` would have shipped a false claim in the one place a
  user checks it. Keep that order on every future release.

**Standing release authorisation is still in force.** The operator's
2026-08-11 instruction — *"please continue to post the latest versions
to git so I can try them on my laptop at home"* — means rule 8's
per-release ask does not apply to cutting a pdfce release: build, tag,
publish the asset, run `tools/verify-release.py`, report what went out.
Scope is narrow: pdfce builds for the operator's own testing. **NOT**
blanket publishing authority, **NOT** licence to treat repository
visibility as an agent's decision, **NOT** permission to skip
verification. `CLAUDE.md` rule 8's literal per-release wording is still
stale against this — flagged to the operator across several filings, not
yet amended by him; not this librarian's or the engineer's file to edit.

---

## Tooling — corrections that cost time in prior sessions, still true

- **`PDFCE_DIAG_VIEWPORT`**, not `PDFCE_VIEWPORT`. Four comma-separated
  numbers: `x,y,w,h`.
- **The diag script separator is `;`, not `,`.** A comma-separated
  script parses as ONE unparseable step and is silently skipped — the
  trace says `script-step-UNPARSEABLE`, read it.
- **`gui-shot.ps1` and `gui-drive.ps1` default to different window
  sizes.** Read the trace's own `rect=`, never a screenshot's pixels.
- **Both scripts move the REAL cursor** and synthesise Ctrl+scroll and
  click-drag on the live desktop. Say so before running one while the
  operator is at the machine; prefer headless verification when it will
  do.
- **A GUI control's traced rect describes the layout *request*, not what
  survived clipping.** The embed batch button shipped unclickable for
  one build with every headless trace assertion passing. A control whose
  traced rect is wider than a sibling's in the same dock is the one to
  click-test first.

- **★ `gh run list --commit <SHORT-SHA>` returns an EMPTY LIST, not an
  error.** Hit directly this filing: three commits queried by short SHA
  returned nothing and were briefly read as "never pushed." They were
  pushed. **Always pass a full 40-character SHA** (`git rev-parse
  <ref>`). An empty result from a query that silently rejects the input
  *form* is indistinguishable from "no runs exist" — `R191`'s shape,
  landed on `gh`. `verify-release.py` is safe (it uses `git rev-parse
  <tag>^{commit}`), but the hazard is one careless argument away.

`tools/splice.py` — anchored substitution, all-or-nothing.
`tools/check-fmt-excluded.py` (no arguments; the fmt gate for the 12
crates `cargo fmt --all` cannot see — run it beside `cargo fmt --all
--check`, never instead of) ·
`tools/verify-release.py <tag>` — **run it BEFORE tagging; it now
consults CI** · `tools/gen-embed-fixtures.py` /
`tools/gen-unembed-fixtures.py` (no arguments needed) ·
`tools/package-portable.py --note "..."` ·
`tools/check-commits-filed.py` (every code commit must be named in the
record; **file the commit, don't add it to
`commits-filed-baseline.txt`** — a baseline entry suppresses the gate,
a filing satisfies it).
