# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. Overwrite it once acted on.

**Written 2026-08-20**, replacing the 2026-08-18 handoff whose headline task
(the `pdfceGUI` reply + the orphan count) shipped as `Pass 102.x`/`103.x` and
whose queue is now three items shorter.

---

## §0 — DO THESE TWO THINGS BEFORE ANYTHING ELSE

**1. `ls` BOTH FeatureRequests channels.** They are outside this repository, so
**no gate will ever contradict a stale sentence about them** — including this
one.

```
D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\
D:\Dev\FeatureRequests\iccce_FeatureRequests\open\
```

`R196` exists because a handoff said the pdfce channel was empty and it was
not. **Today's session found the newest request by `ls`, three lines into the
session, before reading anything else.** That is the whole procedure.

**2. Run the gates, all of them — do not trust a handoff's list.** The previous
handoff's §6 said *"Gates all clean"* and named eight. Running the full set
today found **three that were red**, and **none of the three was among the
eight it named.** The claim was true about the set it listed and false about
the set that exists. **A handoff that enumerates gates by name ages badly the
moment one is added.**

```
bash tools/check-ui-strings.sh          bash tools/check-disclosure-channel.sh
bash tools/check-theme-colors.sh        bash tools/check-bypass-paths.sh
bash tools/check-string-gaps.sh         python tools/check-ledger-numbers.py
python tools/check-core-api-verbs.py    python tools/check-settings-consumed.py
python tools/check-fmt-excluded.py      python tools/check-shipped-assets.py
python tools/check-passes-filed.py      python tools/check-commits-filed.py
python tools/check-one-commit-per-command.py
python tools/check-outcome-disclosed.py
```

**That last one is new today (`Pass 110.0`), and this list is already the thing
§0 warns about** — it is a snapshot, and the next gate added makes it incomplete
without changing a word of it. Prefer `ls tools/check-*` over reading this
block.

All three that were red this morning are green, and so is everything else:
`check-fmt-excluded` (`tools/tw-census` unformatted), `check-settings-consumed`
(→ `Pass 108.0`) and `check-passes-filed` (→ four unfiled 2026-08-19 commits,
now filed). **`check-commits-filed.py` reports clean across all 489 code
commits** — the first time both record gates have been green together in this
session.

---

## §1 — WHAT SHIPPED 2026-08-20

| commit | |
|---|---|
| `ae06440` | `check-string-gaps.sh` widened inside `#[error(…)]` · `tw-census` fmt |
| `9940acf` | **`Pass 107.0` / `107.1` / `107.2`** — the perimeter ce dimension |
| `4a1416e` | librarian: `107.x` filed · **decision 074** · **`R204`** |
| `07c8c22` | `check-ledger-numbers.py`'s decision ceiling was wrong by three |
| `186a983` | **`Pass 108.0`** — `quad_point_order` was a setting that did nothing |
| `860d540` | librarian: `108.0` + the genuinely-unfiled `Pass 106.1` · **`R205`** |
| `3329202` | **`Pass 106.2`** — `merge-document` printed none of its three disclosures |
| `5d1a579` | the handoff + two agent-memory lessons |
| `e265f43` | librarian: the four unfiled 2026-08-19 commits · **`Pass 109.0`** |
| `1c169ba` | **`Pass 110.0`** — `check-outcome-disclosed.py`, the class gate |
| `720cb6f` | librarian: `106.2` + `110.0` filed |

### ★ THE RECORD GATES ARE CLEAN FOR THE FIRST TIME THIS SESSION

`check-commits-filed.py` started the day reporting **seven** unfiled code
commits and now reports **clean across all 489**; `check-passes-filed.py` is
clean. **Four of the seven were left by the 2026-08-19 session**, whose handoff
said the gates were clean.

**The shape is worth more than the cleanup.** `check-passes-filed.py` was green
that entire time, because every one of those four belongs to work that *did*
reach `ROADMAP.md`. **Filing a Pass and filing a COMMIT are two different
acts**, and only the second decays silently. Two gates, adjacent names, and only
one of them was looking.

One of the four (`f93f8da`, the commit that *introduced* `quad_point_order`)
**had no Pass ID at all** and is now `Pass 109.0` — so today's `Pass 108.0`
wired a setting whose own introduction was unfiled.

### `Pass 107.x` — the perimeter tool, whole, in one session

`pdfceGUI` asked for a perimeter measuring tool with vertex editing and *"all
the scaling options of the other dimension tools"*. All seven sections of the
request are answered; nothing was deferred. Reply written into the channel and
an `INDEX.md` row added **in the same edit as the reply**, per that folder's own
rule.

**`DimensionKind::Perimeter { points: Vec<Point>, closed, offset, text_along }`.**
Open and closed are ONE kind with a flag — they differ by exactly the closing
segment. Authored as `/Polygon` + `/IT /PolygonDimension` or `/PolyLine` +
`/IT /PolyLineDimension` with a flat `/Vertices`, ISO 32000-1 §12.5.6.9
Table 178.

**`DimensionKind` and `DimensionRecord` are no longer `Copy`** (decision 074).
The requester's own id-reference alternative was heard and declined. Blast
radius measured: 7 errors in core, 9 across the shells, all mechanical.

Three vertex verbs plus `vertex_edit_preview`, sharing **one** plan body, so
`preview(..).err()` **is** the refusal predicate. **The first ce-dimension verb
that deliberately re-measures.**

**Three numbers worth keeping**, because they are the answers to questions that
will be asked again:

- **It cannot refuse for a shape reason.** Self-intersection and zero-length
  segments are legal; every refusal is structural and knowable before the drag.
  **The shell's drag preview may always be drawn** — that is on the record and
  the shell was told.
- **The label anchors on the vertex CENTROID**, not the CAD-conventional
  longest segment, because a corner drag can change *which* segment is longest
  and the label would teleport. Vertex editing is this kind's headline feature.
- **`/Rect` must equal the `/AP` `/BBox`.** §12.5.5 step (b) **scales** the
  appearance to fill `/Rect`; nothing clips. A mismatch renders a perimeter at a
  length that disagrees with the number printed inside it, and is **invisible in
  an object dump**. The spec corpus calls it *"the single highest-risk authoring
  bug for a ce dimension"*. Now pinned by a test.

### ★ The defect that nearly shipped silently, and the rule it minted

`set_markup_style` refuses a ce dimension by name. The guard tested the literal
string `LineDimension`. **A perimeter is a `/Polygon`** — stroked, coloured,
byte-shaped exactly like a markup polygon pdfce can author — so **adding the
variant silently un-gated the refusal**, and a restyle would have reduced a
measurement to a bare outline with nothing reporting it.

**`R204`: widening the world past a refusal is the same act as removing the
refusal.** The mirror of `R143`/`R144`/`R147`. Operationally: *when a Pass adds
a variant to an enum that any refusal matches on, grep every match site for that
enum and re-verify each one covers the new case.*

`is_ce_dimension` now asks **twice** — the three `/IT` intents **OR** the
sidecar — because they fail in opposite directions. **Each arm has its own
isolated test and each was sabotage-verified to fail with the other arm
intact**, which was not true of the first test written: a real perimeter carries
both, so the obvious test passed with either arm deleted and proved nothing.

### `Pass 108.0` — a setting that did nothing

`Settings::quad_point_order` was parsed, validated, defaulted, documented in the
generated file's own comments, and **read by nothing**. Now session state
(`EditSession::set_quad_point_order`) rather than a second `add_markup_with`
entry point (decision 062), with the **shell** reading the store. Both call
sites — `add_markup` and `set_markup_style`'s regeneration.

The test asserts **what must not change** as well: the baked `/AP` is
byte-identical under both orders. If that ever diverged, a preference change
would silently alter how already-shipped markup *looks*.

---

## §2 — THE QUEUE, in the order I would take it

1. **`Pass 97.0 / 97.1 / 97.2`** — the colorant compositor. Still the
   highest-impact item in the project. Plan of record:
   `docs/compositor-plan.md`; collapse model in
   `docs/collapse-model-survey.md`. **★ Its "16 of the 18 remaining Ghent
   failures" thesis is AMENDED and owes a re-derivation before the Pass is
   scoped from it** — see §3.
   This is also where the **iccce dependency edge appears** (§4), so
   `Pass 101.1` unblocks at the same moment.
2. **`Pass 80.0`** (note text on markup) and **`Pass 81.1`** (markup opacity,
   write half) — both `pdfceGUI` requests, both already scoped.
3. **`Pass 98.0`** — read a foreign `/BE` back into `MarkupSpec`.
4. **`Pass 103.2` / `103.3`** — page labels for inserted pages, and named
   destinations so a carried bookmark resolves. `103.2` **needs a
   `pdfce-acrobat-librarian` dispatch before it is scoped** — nobody has
   measured what Acrobat does with page labels on an insert. Note that today's
   Acrobat dispatch **did** measure the adjacent junction: Acrobat overwrites
   every inserted page with a static copy of the preceding page's label, and
   pdfce deliberately does not match that (decision 072).
5. **The `iccce` channel** — 16 files, two requests genuinely owed, and
   **`note_gray_black_routing_is_yours.md` is still the highest-value unread
   file there.** It is a boundary ruling handing pdfce the four-way
   gray/CMYK/`Separation`/`DeviceN` black equivalence, and it bears directly on
   `Pass 97.x`. Not a request — do not triage it as one.

**★ Ghent standing, unchanged since 2026-08-19 and NOT re-measured today:
26 pass / 14 FAIL / 11 UNRESOLVED of 51.** Re-measure rather than quoting this
line. The GWG Reference file is still not on this machine.

---

## §3 — WHAT TODAY SAYS ABOUT GATES, because it happened three times

**A gate that under-reports is byte-indistinguishable from a green one.** The
only detector is an independent forecast — the exact labour a gate exists to
remove. Today produced two more instances, bringing the count to **four across
two files**:

- **`check-string-gaps.sh` reported TWO of THREE** gaps one Pass introduced.
  The invisible one had `{minimum}` after the gap; the class required a letter.
  Found because I knew there were three and the report listed two.
- **`check-ledger-numbers.py` printed `071 -> next free is 072` while decisions
  072, 073 AND 074 all existed.** All three are written as a dated list item, a
  spelling the declaration pattern could not see. **This was a live hazard**:
  §12 duplicate detection is deliberately absent, so the printed ceiling was the
  only thing preventing a duplicate. Found because the librarian reported
  minting 074 and the gate disagreed — neither was lying and only one could be
  right.

**★ Both prior fixes to these two files had already been written about this
exact failure, and both expired.** The star-anchor fix repaired the one spelling
that had been seen. The 2026-08-11 decision-ceiling fix added `ARCHITECTURE.md`
as a second **source** while keeping a declaration-shaped **pattern** — so the
hole reopened the moment the prevailing spelling changed. **Fixing a source
while leaving the pattern spelling-dependent is a fix that expires.** Prefer a
rule that *cannot* under-report over one that matches the instance you saw.

**And the first widening was wrong in the other direction.** Widening the
string-gap class globally took the tree from 0 findings to ~60, every one a
deliberately aligned report column in a dev tool. The distinguishing property is
not the characters — a `thiserror` message is PROSE, a `println!` in a sweep
tool is a TABLE. **The false-positive shape is now pinned in the gate's CLEAN
self-test**, which is the half that stops the next widening re-breaking it.

**A librarian ruling on whether this earns a standing rule was requested and is
pending** — check `ROADMAP.md` before proposing one.

---

## §4 — iccce: PENDING INTEGRATION, unchanged and still true

`iccce`'s `README.md`, second sentence: ***"Its first consumer is `pdfce`."***
Decision 064's status line: **"DECIDED (boundary), NOT STARTED (either
consumer)."** The absence is a **task, not an architecture** — the two look
identical in a one-word banner, which is how a previous session came to ask the
operator a question decision 064 already answers.

The edge appears when the compositor does (`docs/compositor-plan.md` ~line 355:
the ICC hop is iccce's, and iccce has already shipped the exact call
`Chain::with_destination(&src, Destination::None, intent)`). **There is no
separate "adopt iccce" Pass to schedule.**

`iccce` is at `D:\Dev\iccce`. **`--version` says
`not-linked-yet (integration pending -- Pass 97.x)`**, not `not-linked`, and the
difference is the whole point.

---

## §5 — THE THREE PASSES NOBODY ASKED FOR, AND THE ONE SHAPE BEHIND THEM

`Pass 106.2`, `108.0` and `110.0` were none of them the session's brief. They
belong together because they are **one shape, seen three times in one day**: a
value that is computed and never consumed.

- **`Pass 108.0`** — a SETTING parsed, validated, written back and read by
  nothing. *Inbound*: a caller fills it, nobody honours it.
- **`Pass 106.2`** — three OUTCOME fields computed, returned, dropped.
  *Outbound*: core fills them, no shell tells anybody.
- **`Pass 110.0`** — the gate for the outbound class, because `106.2` was found
  by the librarian **by luck**, while filing something else, and luck is not a
  detection mechanism. Nothing in Rust catches it: `#[must_use]` is about the
  struct, so reading ONE field satisfies the compiler forever.

**`110.0` is the one to read before writing any gate.** Its output was
**forecast before it was built** — 58 fields, 8 unread. Had it come back 50 it
would have been this morning's string-gap mistake again, where a global widening
produced ~60 findings that were all correct as written. *8 is a gate; 50 is a
wall nobody reads.*

★ **And its self-test found a broken regex in `check-settings-consumed.py` on
its first run.** Both gates filter assignments out of `.field` matches, and the
spelling both used lets the whitespace **backtrack to zero**, so
`settings.field = true` — an assignment, with a space — was counted as a READ.
The no-space form was correctly rejected, which is how it survived. That gate's
comment says it was sabotage-verified, **and it was** — against a defect whose
write form was a `&mut` borrow, caught by a *separate* filter beside it. **The
`=` filter was never the thing under test.** Fixed in both; the sibling stays
clean afterwards, so it was latent rather than hiding a live defect.

**Two transferable halves:** *sabotage-verifying a script proves the branch you
sabotaged, not the branch you documented*; and *the cheapest way to test an old
tool is to write a new one that shares its mechanism and hand it the inputs
nobody thought to try.*

The librarian **declined** to grow `R205` to cover this (a resemblance, not a
shared mechanism) and **declined** a new rule (the content reduces to a truism
once both files' doc comments carry the reasoning). Both refusals are on the
record — do not re-litigate without new evidence. The regex finding itself was
handed to `troubleshooting-librarian` for `C:\personal_rag\`, being a general
Python gotcha rather than a pdfce one.

---

## §6 — TRAPS THAT COST TIME TODAY

- **★ Anchoring a code insertion on `fn foo(` or on an enum variant splits it
  from its doc comment.** Happened **twice**, in the same file, in one session.
  The first instance **shipped a wrong `--help`**: `clap` derives its
  description from the doc comment, so `dimension-vertex` displayed
  `dimension-offset`'s text and `dimension-offset` displayed nothing. **Caught
  by running the binary — not by fmt, clippy, any test or any gate.** Anchor on
  the blank line *before* the doc comment, and read `--help` either side of a
  new subcommand.
- **The bash heredoc still eats backslashes**, and it produced all three of the
  `#[error(…)]` gaps above. `\\b` inside a `<<'PY'` heredoc reached Python as a
  **backspace character**. Write the script with `Write`, or use `Edit`. The
  safe form for a Rust string that would need a continuation is **one long
  line** — `rustfmt` leaves it alone.
- **A test that exercises a guard is not a test that covers it.** The first
  `is_ce_dimension` test used a real perimeter, which trips *both* arms — so it
  passed with either arm deleted. Isolating each arm took two more tests and
  two sabotage runs, and only then did the widening actually have coverage.
- **My own arithmetic in a test is a fixture, not an oracle.** Asserting `400.0`
  for a corner drag on a rectangle was wrong (a corner drag changes *two*
  segments, and one becomes a slant). Write the arithmetic out in the assertion
  so the test states its own reasoning.

---

## §7 — STATE AT HANDOFF

- **Working tree clean**; eleven commits today, listed in §1. **Nothing pushed.**
  `github.com/KenM76/pdfce` is public and has a remote — a careless `git push`
  reaches the world.
- `cargo fmt --check`, `clippy --all-targets --workspace -D warnings`, and
  `cargo test --workspace` (**106 suites, 0 failures**) all green.
- `cargo tree -p pdfce-core` / `-p pdfce-render` / `-p pdfce-cli`: **no**
  egui/eframe/winit/wgpu/glow. **No manifest was touched and no dependency was
  added this session.**
- **`v0.7.0` is bumped but NOT tagged.** Standing operator go-ahead for
  builds/releases since 2026-08-17. Verify CI green on `HEAD`, then
  `verify-release.py` → tag → portable package → GitHub release → librarian
  release record. A CI-built release reports `revision: unknown` unless that
  workflow gets `fetch-depth: 0`.
- **Ledger, re-measured at end of session** — next free Pass family **111**,
  decision **075**, standing rule **R206**, filing ordinal **204**.
  **Re-measure with `tools/check-ledger-numbers.py` rather than trusting this
  line; it has been wrong before and was wrong by three this morning.**
- New spec corpus file: `iso32000__s__12.5.6.9.md` (58 kB). It also **corrected
  the corpus**: Errata Issue #444 is `Completed`, not `Accepted` — the review
  state is a chain of `/IRT`-linked replies and only the first had been read.
  **376 of 547 `Accepted` state annotations carry a later `Completed` reply
  (68.7 %)**, so a state read from the first reply is stale two times in three.
  **One open `GAP` left behind:** `iso32000__delta__pdf20_encryption.md`'s
  Algorithm 2.B step-(a) `Accepted` claim needs the same `/IRT` walk.
- New Acrobat corpus file: `measure__perimeter_and_area_tools.md`. Its
  load-bearing finding: **Acrobat cannot insert or delete vertices on a
  committed Perimeter/Area measurement**, sourced from Adobe's own forum with an
  Adobe expert confirming scripting is the only route. **`Pass 107.1` is an
  exceed of the parity reference, not a catch-up**, and that divergence is on
  the record.
