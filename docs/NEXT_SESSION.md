# NEXT SESSION — start here

Engineer-owned handoff (this filing written by `pdfce-librarian` at the
engineer's explicit request, same as every prior overwrite of this
file). Read this **before** the librarian's record — `ROADMAP.md` says
what shipped, this says what is in flight and what the next hour
should be. Overwrite it once acted on.

Written 2026-08-12, branch **`main`**, at `aad48c7`, after four commits
`0947cab` · `c3a4d2e` · `9ea0c88` · `aad48c7` and the **`v0.5.1`**
release.

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

### 5. `cargo fmt --check` fails in `tools/difftest` — 109 diffs, never covered

`tools/difftest` is **not a workspace member**, so `cargo fmt --all
--check` has never reached it and the gate has been green over code it
does not see. Either add it to the fmt sweep and fix the 109 diffs, or
record deliberately why it stays excluded — but the current state is a
gate with a blind spot nobody chose.

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

## ★ Take a backup — the newest one predates all of `Pass 67.0`

**Measured this filing by `ls -la D:\Dev\pdfce-backups\`, not inferred
from any document:** newest artefact is
`pdfce-full-20260808-0956.zip` (183,439,240 B, **2026-08-08 10:01**);
newest bundle is `pdfce-20260807-2039.bundle` (**2026-08-07 20:39**).
Both predate every commit in this filing **and the whole of `Pass
67.0`**. A fresh bundle is owed.

---

## Release state — `v0.5.1`, verified

**Measured this filing** (`git ls-remote --tags origin`,
`gh release view v0.5.1`, `git status --short`):

- Tag `v0.5.1` → `aad48c73…`, **equal to `HEAD`**; working tree
  **clean**; `origin` = `https://github.com/KenM76/pdfce.git`.
- Release *"pdfce v0.5.1 — embed-font works from the CLI"*, created
  2026-08-12T14:41:54Z; asset `pdfce-v0.5.1-portable-win64.zip`,
  **10,244,728 B**. `tools/verify-release.py v0.5.1` reports clean.
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

`tools/splice.py` — anchored substitution, all-or-nothing.
`tools/verify-release.py <tag>` · `tools/gen-embed-fixtures.py` /
`tools/gen-unembed-fixtures.py` (no arguments needed) ·
`tools/package-portable.py --note "..."` ·
`tools/check-commits-filed.py` (every code commit must be named in the
record; **file the commit, don't add it to
`commits-filed-baseline.txt`** — a baseline entry suppresses the gate,
a filing satisfies it).
