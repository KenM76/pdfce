# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**: no
*"this paragraph read X until…"*. What is true now, plus a pointer.
Corrections and their prior wording live in the **append-only** record —
`ROADMAP.md` and `SESSION_LOG.md` — where a claim is dated and no later edit
can falsify it.

---

## §A — COLD START: everything you need, in one screen

**The previous session shipped `Pass 132.0`** — the CMYK compositing ceiling
became readable (four `pub` items in `pdfce-render`) and settable
(`Settings::max_cmyk_buffer_bytes`, uncapped), because a sibling project could
neither size a raster to stay inside it nor let the operator move it. It then
**corrected nine operator-facing "A4" figures that had been computed on a page
that is not A4** (`daefceb`) and **cut, pushed and published `v0.14.0`**
(`d3b4f5a`).

### ★ Verified from a shell at write time — do not copy forward without re-running

| fact | value | command |
|---|---|---|
| `HEAD` | `d3b4f5a` | `git rev-parse HEAD` |
| `git describe --tags` | `v0.14.0` | `git describe --tags` |
| `origin/main` | `d3b4f5a` — **0 ahead, everything is public** | `git rev-list --count origin/main..main` |
| tag at `HEAD` | **`v0.14.0`** (annotated), **and it is being MOVED — see `§B`** | `git tag --points-at HEAD` |
| release | `v0.14.0` published 2026-08-26T20:17:37Z, one asset, 24,708,755 B | `gh release view v0.14.0` |
| CI at the tag | **RED on 1 job of 10**, and only for the filing gap `§B` closes | `gh run view 33010309213` |
| working tree | clean at read time, **dirty with the 271st filing when you get it** | `git status --porcelain` |
| newest backup bundle | `pdfce-20260826-0958-9a4fb18-full.bundle`, **4 commits behind `HEAD`** — **a bundle is OWED** | `ls -lt D:\Dev\pdfce-backups\` |
| gates on disk | **18**; **17 run with no arguments**, the 18th (`check-image-colorspace-truth.py`) needs a fixture dir | `ls tools/check-*`, then run them |

---

## §B — ★★★ DO THIS FIRST: `v0.14.0` IS RELEASED, BUT ITS TAG POINTS AT A COMMIT WHOSE CI IS RED — FINISH THE MOVE

**The release exists and the portable contract holds.** `d3b4f5a` bumped the
workspace `0.13.0` → `0.14.0`, the tag was pushed, the GitHub release was
created with `pdfce-v0.14.0-windows-x64-portable.zip` (24.7 MB), and the
packaging smoke test passed on a fresh path: `pdfce-cli.exe --version` reported
`0.14.0` / revision `v0.14.0`, a render with `--max-cmyk-buffer-bytes 512mib`
succeeded **and disclosed the override**, and `pdfce-gui.exe` launched there
and created its own `userdata/`.

**What is unfinished is the tag's target.** CI at the tag is **red on exactly
one job of ten** — *verify `pdfce-gui` strings live in `ui_text.rs`* → its
`check-commits-filed.py` step → **`daefceb` in no filing**. **All nine others
are green**, including `cargo test` on **both** operating systems, clippy,
fmt, the macOS/wasm32 cross-target check, the zero-GUI-deps check, the
no-network check, the third-party licence audit and `cargo fuzz build` on
nightly.

**The 271st filing closes that gap**, so once it is committed the reason for
the red no longer exists — but the **tag still points at the commit that was
red**. Finish it:

> **1.** Commit the 271st filing. **2.** Force-push `v0.14.0` onto that commit.
> **3.** **Rebuild the package**, so `BUILD-INFO.txt` names the newly-tagged
> commit. **4.** Replace the asset with `--clobber`. **5.** **Re-run the smoke
> test on the NEW artefact** — a re-cut release is a new artefact and does not
> inherit the old one's test. **6.** Confirm CI green at the tag.
> **7.** Take a backup bundle; the newest is 4 commits behind.

**Nothing is wrong with the gate.** `R217`'s deferral excuses the tip and
`d3b4f5a` is duly deferred; `daefceb` sits *behind* the tip and is real,
unfiled debt, which `R217` says still hard-fails. See `§J` for the ordering
lesson, which is the reusable half.

★ **The outbound reply's forward-dated sentence is now TRUE** —
`open/reply_cmyk_buffer_ceiling.md:4`'s *"released as `v0.14.0`"* was ahead of
the repository and is not any more. That file also now carries the corrected
A4 table and the peak-memory caveat, so the copy the other project reads is
correct in both.

★ **Still unanswered, and it will bite at the NEXT tag rather than this one:**
`Pass 129.1` changed a shipped default's *value* (`ocr --dpi` 300 → 150), which
a scripted caller can observe while adding no public core item. `768e934`
settles the additive case and does **not** settle this one. **Decide before the
next bump, not after.**

---

## §C — ★★★ THE ONE CODE DEFECT, STILL UNFIXED, STILL THE RIGHT FIRST TASK

**pdfce cannot see that a push button submits to a website.** Unchanged from
the previous handoff — nothing in `Pass 132.0` touched it.

`scan_javascript` / `FormJavaScript` in `crates/pdfce-core/src/forms.rs` exist
— in their own doc comment's words — *"to disclose what a document would run in
Acrobat/Reader"*, and `network_action_count` is documented as flagging *"`/AA`
actions referencing the network"*.

**It scans `/AA` only. A widget's PRIMARY action lives in `/A`.** A submit, a
URI, a launch on a push button all sit in `/A`; `/AA` carries only the
*additional* triggers (`/E` `/X` `/D` `/U` `/Fo` `/Bl`).

**Measured on a hand-built file whose button Acrobat then actually submitted to
`http://127.0.0.1:8765/declared-http`:**

| surface | output |
|---|---|
| `list-fields` | `aa=0 … js_network_actions=0 js_launch_actions=0` |
| `inspect` | no action / submit / network / URI / launch line at all |
| `list-annotations` | `subtype=Widget … author="Go"` — no action |

**Three surfaces, none of them disclosing it.** Ask pdfce whether a document
phones home and it says no about a file that demonstrably does.

**Fix shape.** Walk widget `/A` alongside `/AA` — and because `/A` also appears
on **link annotations, outline items and `/OpenAction`**, decide deliberately
which carriers are in scope rather than patching the one that was noticed. This
is **recognition only**: it fires no trigger, so it needs neither the `R54`
amendment (decision `088`) nor any new rule. Owes fixture tests and a
`FEATURES.md` touch (the disclosure rows currently over-claim).

★ **Why it ranks above everything in §E:** it is the read half of the exact
disclosure the whole submit plan rests on, it is shipped and wrong today, and it
is the same *"a check that under-reports reads as a clean bill of health"* shape
this project keeps meeting.

---

## §D — ★★ THE "A4" REPAIR IS DONE — AND THE COMMIT THAT DID IT BROKE A STRING LITERAL IN THE PARAGRAPH IT WAS FIXING

**All nine sites are repaired** by `daefceb`, on the *recompute-for-A4* branch,
whole rather than by halves. On true A4 (595 × 842 pt = 500,990 pt², against
`max_cmyk_composite_pixels(None)` = **13,421,772 px**): the default ceiling
reaches **518 %**, 1 GiB reaches **1035 %**, the `MAX_PIXMAP_EDGE` tier ends at
**1946 %**, the buffer wants **641 MB at 800 %** and **1.44 GB at 1200 %**, and
a square 16,384² raster wants **5.4 GB**. US-Letter is **379 DPI**. Every one
of those was re-derived independently in the 271st filing and all seven agree.

`grep -rn "534 %\|about 530\|2071\|5\.33x" crates/ docs/core-api/` returns
**one** hit, and it is the deliberate parenthetical at
`docs/core-api/03-capabilities.md:2040` telling the consuming project which of
*its own* request figures were on the wrong sheet. **Nothing stale survives.**

The doctest was fixed by moving the **literals** — `3076 × 4353` inside,
`3082 × 4362` past — which is the right half to move, because the compiler runs
the literal and nothing runs the label. The documentation test's ±30-point band
became **±5** (`513.0..523.0`), its bare `contains` moved to `"about 518%
zoom"`, and **a second number is now checked beside it** (1 GiB / `1035`),
because the wrong-page error moved both and a check on either alone would have
caught it only by luck.

### ★★★ WHAT THE REPAIR LEFT BEHIND: TWO REJOINED LINES IN THE GENERATED `settings.txt`, AND A GATE THAT CANNOT SEE THEM

> **★★ EVERYTHING IN THIS SUBSECTION IS DONE — read it as HISTORY, not as a
> work order.** It was written while the repairs were in flight and is left
> standing because the analysis is the part worth keeping. Three commits closed
> it: **`ffe9d4c`** repaired the two rejoined lines, **widened the gate** (with
> a `dirty4` fixture pinning the defect and a `blank_line_inside_a_literal`
> fixture pinning the legitimate shape), added a test asserting on **pdfce's own
> output** rather than on a round trip, and closed the
> `03-capabilities.md:2003` survivor; **`18b0438`** removed the header's two
> dangling `check-strong-text.sh` citations. **`rg '^\s*\\n' --glob '*.rs'
> crates/` now returns four, the four legitimate lines.**
>
> ★ **Two corrections to what is written below, both earned:** the
> discriminator recommended here — *"does not end in `\`"* — is **wrong**, and
> let a second variant through (one line loses its continuation while the next
> keeps its own); the shipped test is *"the line's first two characters are the
> escape AND it carries anything besides the continuation."* And the "0 false
> positives" measurement was sound, but ⇢ *a discriminator derived from one
> observed instance is a hypothesis about the whole family; the second variant
> is the test, and it costs one reproduction.*
>
> **The one thing still owed from this subsection is the SIBLING check** —
> queue item 2 above.

`crates/pdfce-core/src/settings/mod.rs:1990–1992`. The two new peak-memory
lines were appended to the settings-file prose literal **without their line
continuations** — each `\n\` became a **raw newline** with the `\n` escape
displaced to the start of the following line:

```rust
             # about 1035%, 4gib about the largest page pdfce will raster at all.
\n             # A page with layered transparency can need up to about FOUR TIMES
\n             # this at once, because each layer is given a buffer of its own.\n",
```

A bare newline inside a Rust string literal is legal and kept verbatim, so the
generated `settings.txt` gets **two blank lines and 13 spaces of source
indentation** in the middle of the paragraph. **Verified by compiling the
literal with `rustc` and printing it**, not by reading it. Non-fatal — the
parser trims before testing `starts_with('#')` (`settings/mod.rs:1663–1664`) —
**cosmetic and still wrong**, in a file pdfce writes onto the operator's disk.

★★★ **`check-string-gaps.sh` IS GREEN ON IT, and that is the transferable
half.** `daefceb`'s own message credits that gate with catching this family,
one file earlier. The gate matches **three or more spaces between two word-ish
characters on ONE source line** — a model that assumes **`rustfmt` folded the
two lines together**. `rustfmt` **cannot** fold these: the raw newline is part
of the literal's value, so the gap stays as **leading indentation at the start
of a line**, with no word-ish character in front of it. ⇢ ***A gate that
detects a defect by its POST-FORMATTING shape misses every instance the
formatter could not reshape.*** The gate's header claims *"there is no false
NEGATIVE that ships anything inert"*; **this is one.**

**The widening is measured, not speculative.** `rg '^\s*\\n' --glob '*.rs'
crates/` returns **six lines tree-wide**: four in `crates/pdfce-gui/src/main.rs`
(535, 537, 541, 545) that are **correct** — they carry their trailing
continuation backslash — and the two defects, which do not. **2 true positives,
0 false positives.** Pin **both** shapes in that gate's self-test, which it
already has the discipline for.

★ **Third occurrence of this family, second consecutive one inside a "correct
the prose" commit** (`2c3210a` → `6a9511a` → `daefceb`). ⇢ *A commit that names
a defect class in its own message is not thereby immune to it.*

~~★ **One minor survivor, reported not fixed:**~~ — **CLOSED in `ffe9d4c`.**
`docs/core-api/03-capabilities.md:2003` said **"a factor of four on A4"** flat,
where the crate's own doc comment was softened in the same commit to **"very
nearly a factor of four"** (1946 ÷ 518 = **3.757×**) — same quantity, two
precisions, in the document a sibling project reads. It now reads *"the gap
between them is very nearly a factor of four on A4 (3.76×)"*: the crate's own
wording plus the figure. ⇢ *`R212` satisfied — the two copies of the contract
agree.*

---

## §E — THE NON-JAVASCRIPT WORK QUEUE

The operator's standing instruction: **continue the other, non-JavaScript
work.** The submit/scripting plan is **parked by his own ruling** — see §G. Do
not re-open it.

Ordered by engineering judgement, not by Pass number:

0. **§B — finish the `v0.14.0` tag move.** Ahead of everything, because a
   public tag on a red CI run is what a downloader sees.
1. **§C — the `/A` disclosure defect.** Do it first among the code work.
2. **§D — the sibling-gate check.** ★ **The widening itself is DONE** in
   `ffe9d4c`, and the discriminator this role recommended was the wrong one —
   see §D. **What is still owed is the SIBLING check:** do
   `check-ui-strings.sh` and `check-theme-colors.sh` carry the same
   post-formatting-shape assumption? A sibling carrying it is what would earn
   the general form a `D:/dev/rag/rust/` file. ★★ **These are the two
   line-scanners that EXIST.** This item named `check-strong-text.sh` until
   `18b0438`; **there is no such gate and there never has been** — the name was
   read out of `check-string-gaps.sh`'s own header, which cited it twice, and
   repeated here as a work item. `18b0438` rewrote both sentences to argue on
   their own account and left a note saying so. ⇢ *A dangling reference inside
   a trusted document is indistinguishable from a real one until somebody runs
   `ls`.*
3. **`Pass 130.2` — per-sample image overprint for `Separation`/`DeviceN`
   images.** Re-scoped 2026-08-26 and **smaller than it looks**: Table 149
   excludes a sampled image from row 1 by name, so painting a **process** image
   normally is conforming and must NOT be "fixed". What is owed is row 3 — a
   process component takes the backdrop under `OP true`, so an overprinting
   `DeviceN` image must preserve it. Blocker: `Pass 130.1` captures colorants
   only for a `DeviceCMYK` base. Candidates: `PCS1_190`/`191`/`192`.
4. **`Pass 122.7` — the undiagnosed blue-channel residual.** Green matches
   Acrobat to ~1 level (187.5 vs 186.6); blue does not (55.7 vs 2.6, down from
   209.2). A large improvement, not a completed one, and nobody knows why.
5. **`Pass 127.2` — `redact-mark`'s stdout carries no diagnostics field.**
   `find-text` prints unreadable-font counts to stdout; `redact-mark` puts them
   on stderr only, so a batch caller parsing stdout still cannot tell a clean
   run from an unreadable one. Small, closes a real asymmetry.
6. **The per-colorant (spot) compositing buffer.** One plate per spot colorant.
   Every remaining print-suite FAIL is now an overprint, spot or ICC patch —
   **not one is a blending-space failure**. ★ The cheap page-sized spot-ink
   multiplier was built, ablated and reverted: it flipped no patch and regressed
   one. **Do not re-attempt it.**
7. **`Pass 122.3` — band a large page render.** Two of its three acceptance
   clauses were discharged by `Pass 132.0` (the refusal note now names the way
   out with numbers; the ceiling's doc comment now carries the arithmetic
   argument it demanded). **Banding is the third and it is untouched.**
   ⇢ *An operator-set ceiling trades memory for correctness; banding removes the
   trade* — peak usage is a multiple of nominal (parent + child + spare), so a
   ceiling admitting one large page admits three.
8. **CLI surface for the four ce-dimension group management verbs** — rename,
   delete, delete-with-policy, re-parent. Core ships all four; no subcommand
   reaches any of them, so from a script a group is still create-only (`R151`
   shape: callable-and-uncalled).

**A `D:/dev/rag/rust/` escalation is OWED and was deliberately not written.**
The general form — *an infallible allocation is a bound's silent partner; a
ceiling removed without `try_reserve` converts a typo into an abort* — is a real
ecosystem finding, but **no OOM was actually induced**, so it would be filed on
reasoning rather than measurement. Induce one (a 64 GiB ceiling on a machine
that cannot honour it, and check the process survives and discloses) and the
finding is worth writing. The pdfce-visible half is already in
`ARCHITECTURE.md` §10.1a and decision `089`.

---

## §F — WHAT `Pass 132.0` DECIDED, IN CASE YOU TOUCH THE SAME GROUND

**Decision `089` has two halves and they are one decision.**

1. **`ARCHITECTURE.md` §10's allocation-ceiling rule is about UNTRUSTED
   INPUT.** Every guard in §10.1 bounds a file-supplied quantity. A number the
   operator typed is not that, so an operator-set ceiling is **uncapped** — no
   guard, no warning, no preflight — on his own `max_zoom_percent` ruling:
   *"it is up to the user to determine how much of a performance hit they want
   to take."*
2. **That is only safe because the allocation became fallible in the same
   commit.** `vec![0.0; n]` aborts the process on allocation failure — no
   unwind, no `Err`, no page, **no disclosure**. `CmykBuffer::try_planes` uses
   `try_reserve_exact`.

⇢ **Binding: a session that raises or removes an operator-settable allocation
bound must check that the allocation behind it is fallible, in the same
change.** Full reasoning, alternatives weighed and the scope limits are in
`ARCHITECTURE.md` §12 decision `089` and the new §10.1a.

---

## §G — THE SUBMIT / SCRIPTING PLAN IS PARKED. READ THIS BEFORE RE-OPENING IT

Full detail: **`docs/plan-scripting-submit-and-plugins.md`**. Eight operator
rulings are quoted verbatim there; do not paraphrase them from this file.

- **A push button that does anything was blocked by `R54`** (*"no trigger event
  ever fires"*), **not** by the JavaScript rule. `R54`'s text outran its
  motivation and bit a plain, script-free Reset button. **The operator ruled
  "change the rule"; decision `088` amends it to a dispatch allow-list.**
- **The JavaScript half is deferred by the operator** (*"defer for now"*).
  `R53` stands. Phases 1–3 must each work with **no scripting engine present at
  all** — a design constraint, not a description.
- **The plugin boundary is a versioned MESSAGE FORMAT, not a binary**, ruled for
  the stated reason that it makes a web version easier. It may name no pipe, no
  path and no host-language type.
- **Submission is permitted, destination open by default, destination always
  disclosed, whitelist mode and payload disclosure available.**

**Still owed before any submit code:** a decision record for `R12`'s new
destination class (sending the operator's data where a *file's* author said is
a different class from fetching where *we* said), the `R13` clause 5 ruling
(deliberately not forced — Phase 3 is hand-install only), the transport question
(unencrypted destinations), and a `pdfce-ui-specialist` dispatch before any GUI
surface.

**Measured against the local Acrobat, and it corrected the research:** the
Security Warning names **scheme + host only** — not the port, not the path —
while the button's own hover tooltip shows the *full* URL. The
remember-this-site box is **ticked by default** and one Allow grants the host
for every PDF permanently. **Not established:** HTTP-vs-HTTPS,
scripted-vs-declared, and the raw FDF body (so the spec's claim that a baseline
FDF submission carries your file's path and document ID is **spec-sourced, not
observed** — do not upgrade it without the bytes).

★ **If you rebuild that probe harness** (it lived in `%TEMP%\pdfce-submit-probe\`
and is disposable): Acrobat ignores synthetic keystrokes and `BM_CLICK`
entirely; UI Automation can **read** its dialog but not press it. The working
technique is **read the true control rect via UI Automation, click it with the
mouse**. Two modals swallow clicks aimed at the page — sign-in, and crash
recovery — and **a swallowed click is indistinguishable from a refusal**, which
cost several runs and nearly produced a false security finding. And an `Allow`
with the box ticked **writes into the operator's profile**: a prior session's
entry was removed and `version:2|ikea.com:2` restored and verified. Any future
run owes the same.

---

## §H — PRE-FLIGHT, EVERY SESSION

**1. `ls` BOTH FeatureRequests channels.** They are outside this repository, so
**no gate will ever contradict a stale sentence about them — including this
one.**

```
D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\
D:\Dev\FeatureRequests\iccce_FeatureRequests\open\
```

At write time the pdfce channel's two newest files are
`reply_cmyk_buffer_ceiling.md` — **checked, and it is now correct in all three
respects: the release it claims exists, its A4 table is the recomputed one, and
it carries the peak-memory caveat** — and the
`request_cmyk_buffer_ceiling_is_invisible_to_the_gui.md` it answers. **Four
`request_*` notes remain unanswered:** `adopt_widget`
pre-flight, markup-opacity-in-two-verbs, `insert_pages` orphaned widgets, and
restyle-an-existing-text-run. The iccce channel's newest is
`note_your_name_gate_has_the_two_defects_mine_had.md` — **unanswered, and it is
about a gate this project also owns.**

**2. Run the gates — `ls tools/check-*`, do not trust any list**, including §A's
count. `R209`: *"all gates green" names a set, and the set somebody runs is not
the set CI runs.*

**3. ★★ RUN `cargo +nightly fuzz build` ON WINDOWS.** CI cannot do it for you
and `cargo check --bins` is not a substitute. Two traps, each defeating the
obvious escape from the other: CI's fuzz job runs on `ubuntu-latest` and was
green throughout a window in which the harness was completely unbuildable here
(`rten` declares a `cdylib`, and only Windows hands that libFuzzer's
`/include:main`); and the documented local stand-in **passes in both states**,
because `cargo check` never links and the break was a link break. **A cheap
proxy for a gate is a proxy for the part of the gate that is cheap.**

**4. Read `docs/ARCHITECTURE.md` §12** — cross-project boundaries live there and
no gate can catch a violation of them. **New since the last handoff: `089`, plus
§10.1a.**

**5. Read `docs/compositor-plan.md`** before scoping anything in `97.x`, and
before items 3, 4, 6 or 7 of §E.

---

## §I — ★★ A DISPATCH IS A SET OF CLAIMS, AND YOURS WILL BE WRONG

Carried forward because it keeps earning its place. The 263rd filing's dispatch
carried three factual premises and **all three were false** — a path the other
project had already consumed and renamed, a `FEATURES.md` box asserted `[ ]`
that had been wired six hours earlier, and a feature credited to the wrong
commit. None was careless; each was a reasonable inference from what the
engineer remembered doing. **Memory of one's own session is exactly the kind of
source that feels like a fact.**

★ **Fresh instance, 2026-08-26: the `Pass 132.0` dispatch carried "about 534 %
zoom on A4" as an established fact.** It was arithmetic on a different sheet,
repeated from the crate's own doc comments into the dispatch and then into a
decision. **It was caught by division**, not by review — which is hard rule
10's whole point. **All nine sites are now repaired** (`daefceb`); what the
repair left behind is in §D.

★★ **And the repair's own dispatch carried a false premise in turn:** it stated
that the commit's one rejoined string literal had been *caught and fixed*. One
was. **Two more, in a different file, were shipped in the same commit and no
gate saw them** — found by the filing, not by the sweep. **A dispatch's claim
about what a gate caught is a claim about the gate's coverage, and that is
exactly the kind nobody re-checks.**

**Write dispatches so a premise is checkable, and expect the agent to check.** A
dispatch that says *"X is at path P"* invites verification; *"as we discussed"*
does not. **Finish the code, then dispatch, then commit the filing last.**

---

## §J — THE FILING COMMIT MUST BE THE LAST COMMIT BEFORE ANY TAG — AND ★ THE DEFERRAL IS EXACTLY ONE COMMIT WIDE

`check-commits-filed.py` counts commits that no filing names. The tip-deferral
excuses a commit that cannot cite its own hash — **but only while it is the
tip**. The instant a filing lands on top, the excuse evaporates and the gate
flips red without anything about that commit changing.

> **Dispatch the librarian LAST, and commit its filing LAST.** Any code commit
> made after the dispatch has, by construction, no filing that can name it.

★★ **THIS JUST HAPPENED, AND IT SHOWS WHERE `R217`'s CLOSING CLAIM RUNS OUT.**
The engineer ran the full 17-gate sweep, it was clean, and **then made two more
commits** — so **the sweep certified a tree that no longer existed**, and CI
was red at the tag. `R217` ends *"a tag on a code commit is green whenever the
history behind it is filed; there is nothing left to remember or get wrong by
ordering."* **The tip-deferral covers exactly ONE trailing unfiled code
commit.** Make **two** and the second shields the first out of the deferral
window — it is no longer the tip — and the gate is red on the very first CI
run, with nothing about either commit having changed.

⇢ ***A one-commit deferral tolerates one trailing code commit. The second is
not deferred — it is merely no longer the tip.*** Recorded as a **named
candidate under `R217`** (n = 1, not minted; the mint is the operator's act) in
the `Pass 132.0` entry's 271st-filing amendment. **The orders that work are
(file → code → file) or (code → file, then stop).**

★ **`c29f5bd`, `76eb04c`, `daefceb` and `d3b4f5a` are all named by filings on
disk** — the 268th/269th name the first, the 270th the second, the 271st the
last two — so the gate has nothing outstanding *provided the 271st's commit is
the tip when you re-tag*.

Recovery if it goes wrong anyway (precedent: `v0.8.0`, `v0.10.0`, `v0.12.0`):
file the orphan, re-tag at the filing commit, force-push the tag, **rebuild the
package** so `BUILD-INFO.txt` names the tagged commit, replace the asset with
`--clobber`, and **re-run the smoke test on the new artefact** — a re-cut
release is a new artefact and does not inherit the old one's test.
