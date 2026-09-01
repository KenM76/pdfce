# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**. What is
true now, plus a pointer. Corrections and their prior wording live in the
**append-only** record — `ROADMAP.md` and `SESSION_LOG.md`.

Written 2026-08-31, rewritten by the 356th filing after `Pass 191.1`
(`ee7866d`), the `dc8cde7` filing debt, and two new items: `Pass 192.0`
(a reported bug, not started) and `Pass 193.0` (a new capability, IN
PROGRESS). **For the ledger — Pass ceiling, standing-rule ceiling, decision
ceiling, filing count — run `python tools/check-ledger-numbers.py`.** It
derives all four and is the only thing that cannot be stale. **This role had
no shell this session; nothing below marked "measured" or with a command
attached was re-run here — confirm before trusting a number in this file.**

---

## §0 THE ACTIVE ITEM IS `Pass 193.0`. `Pass 192.0` IS QUEUED BEHIND IT. `Pass 191.1` IS CLOSED.

★ **Closed items are kept below as one-line verdicts rather than deleted**,
so a reader can see they were **answered**, not dropped.

### ~~`Pass 191.1` — the eight-site deletion-safety audit~~ — **CLOSED 2026-08-31 by `Pass 191.1` (`ee7866d`), filed here by the 356th filing**

**VERDICT: all eight audited sites are fixed** (seven verbs directly; the
eighth, `delete_pages_with`, was already clean per `Pass 191.0`'s audit).
Twelve hostile fixtures, all twelve at `R238` rung 3 pre-fix, all twelve
green post-fix, sabotage-checked in the **release** profile specifically.
New `EditError::CarrierIsNotAStream` (113 → 114 variants). A second,
independently-found defect — `refuse_if_in_page_tree`'s error message still
named only its original two carriers after gaining seven more — is also
fixed. **Full record: `ROADMAP.md`'s `Pass 191.1` *Shipped* entry, second
from the top.** One item reported and NOT fixed, filed to *Backlog* instead
of folded into this Pass: `resize_annotation`'s `/AP` `/N` overwrite can
still reach a page's own `/Contents` stream (guarded against dictionaries
only as a side effect, not against streams generally).

### ~~`dc8cde7` — the `R239` remedy, unfiled~~ — **CLOSED 2026-08-31, filed under `ROADMAP.md` *Shipped* (no Pass ID) by the 356th filing**

A code commit made in a prior session, implementing the proximity-anchor fix
`R239` itself called for (`tools/check-core-api-verbs.py` now also matches
`docs/core-api/02-editing-and-saving.md:3794`'s copy of the `EditError`
variant count, not only `index.md`'s). It sat unfiled; `check-commits-filed.py`
caught it. **Verified working in the very commit that closed it above** —
`Pass 191.1`'s new variant moved 113 → 114 and the gate caught both copies.

### ★★★ THE ACTIVE ITEM — `Pass 193.0`, PDF internal-structure inspection and dump (core + cli), IN PROGRESS

Operator instruction, verbatim (relayed by the engineer): *"make the
structure inspection and structure dump part of the core and CLI, then use
it to diagnose."* Motivation: diagnosing `Pass 192.0` (below) needed to see
the `/ExtGState`/`/SMask` structure of a page, and pdfce has no way to
inspect its own object graph — the only route available was a throwaway
hand-rolled decompressor. **Scope is PROVISIONAL** — a parity read from
`pdfce-acrobat-librarian` on Acrobat's own internal-structure/Preflight
browser has been requested and is not yet in hand. Bullet scope, subject to
revision: bounded cycle-guarded COS-graph traversal/pretty-printing; raw and
decoded stream contents with an output-size ceiling (`ARCHITECTURE.md` §10);
the physical layout distinct from the logical graph (xref table vs. stream,
object-stream membership, incremental-revision boundaries, trailer chain,
linearization); `pdfce-cli` subcommands over all of it.

**Start `Pass 193.0` from `ROADMAP.md`'s own entry under *Next up*, not from
this paragraph** — acceptance criteria are not yet written there either;
write them once the acrobat-parity read lands and the API shape is settled.

### ★★ QUEUED BEHIND IT — `Pass 192.0`, `PCS 16.8`: the green Bevel-and-Emboss cell renders its highlight and omits its shadow

Reported by the operator. Measured by quadrant against the patch's own baked
reference cell (both 136×136 at scale 6, both located programmatically):
top-left (the bevel's lit side) is essentially exact — mean-abs-diff **1.9**,
0 pixels over threshold. The other three quadrants are wrong — mean-abs-diff
**20.0 / 33.5 / 63.4**, **1,184 / 1,868 / 3,414** pixels over threshold out of
4,624 each. **6,466 of 18,496 pixels (35.0%) differ by more than 30; max
per-channel difference 141.** Reading: **the highlight half of the bevel
renders; the shadow half does not.**

**Root cause NOT YET IDENTIFIED.** Untested hypotheses, no order implied:
the shadow layer's own blend mode; its luminosity soft mask's `/BC`
backdrop; a `/TR` transfer function on that mask. `Pass 193.0`'s structure
dump exists partly to make this diagnosable properly.

**A separate, smaller, suspected-not-confirmed defect found alongside this
one:** the `blend_modes_applied` CLI disclosure still reads *"pdfce blends
in device sRGB"* on a run that reports `blends_in_wrong_space=0` and
`blend_space_subtractive=6` — the note's prose looks stale relative to its
own counters. Not verified beyond reading the two together; do not treat as
diagnosed.

⇒ **Start `Pass 192.0` from `ROADMAP.md`'s own entry under *Next up*.**
Acceptance criteria are deliberately not yet written — root cause is
unknown, and criteria should follow diagnosis, not precede it.

---

## §A — Candidates, ordered by my read of value. None is a commitment.

1. **★ CHECK BOTH FeatureRequests CHANNELS FIRST.** They live outside the repo
   so no gate can contradict a stale "it's empty":
   - `D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\`
   - `D:\Dev\FeatureRequests\iccce_FeatureRequests\open\`

   As of the 355th filing, three inbound requests had been answered by
   shipped work and still sat in `open/` with outbound replies beside them,
   nothing owed back. **Not re-checked by this filing — no shell available
   to this role this session.**

2. **`Pass 193.0`** — the active item, §0 above. Start there.

3. **`Pass 192.0`** — queued behind it, §0 above.

4. **`Pass 142.0`** — a font face outside the standard 14. The largest
   remaining *named* feature, **de-prioritised by the consuming project's
   own use report**, not by us: *"Synthetic is enough. Drop `142.0` down the
   queue."* Not closed, not declined.
   ★ `bind_font_resource` (`text_edit/addtext.rs`) is the single
   implementation of "add a `/Font` entry"; `142.0` extends it and does not
   write a second one. Three save paths exist — `EditSession::format_text`,
   its form twin, and the one-shot `text_edit::set_format` (the one the CLI
   uses); `Pass 162.0` wired two, every unit test passed, and the binary
   printed a disclosure about a resource it had not written.

5. **The `text_edit` resolver residual** (`ROADMAP.md` *Backlog*).
   `edit_text`, `format_text` and the two `preview_*` verbs still pass
   `&self.base` to the text planner **as the object resolver**, so a `/Font`
   created this session cannot be resolved through it. **The outcome is a
   clean refusal, not a wrong edit**, unchanged since before `Pass 186.0`.
   ★ **Re-size it before scheduling it** — the obvious shape is threading a
   view through `text_edit`'s ~40 `doc: &Document` signatures, wide and
   mechanical, in the crate's most defect-prone module.

6. **`/AP` `/D` (the pressed appearance) and `/MK` icon/label layout**, the
   remainder of `Pass 131.0`. ★ It is **appearance work** (`R43`'s
   neighbourhood), not a continuation of the button-action work.

7. **The n-channel (per-spot-colorant) buffer** — the only path to the print
   suite's remaining overprint/spot FAILs. **Operator's call; do not scope
   it without him.**

8. **`CmykIntent::Calibrated`'s cool greys** — still not ours to fix
   (decision `064` puts the conversion in `iccce`'s domain; the operator
   ruled the default 2026-08-28). ★ **The black end of that same table is a
   FALSE-DEFECT TRAP** — pdfce is the *closer* answer there and `iccce` said
   so unprompted. Read `settings/mod.rs`'s doc comment before touching
   anything CMYK.

9. **`resize_annotation`'s `/AP` `/N` overwrite gap** (§0 above, filed to
   *Backlog* by `Pass 191.1`) — needs the whole-document `/Contents`-stream
   reference census `appearance_streams_owned_by`'s own doc comment already
   names as an open bound, not merely a type test.

10. **Five ungated derivable counts in `docs/core-api/`** (found 354th
    filing) — scoping those is a Pass, not a filing.

---

## §B — What is deliberately NOT being worked, and why

- **`/BS` `/W` does not change a check box's or radio button's drawn border.**
  pdfce authors it at a fixed **1.0**. That is **the artwork's existing
  contract**, not an oversight — honouring `/BS` `/W` would alter how every
  pdfce-authored check box already in the wild renders on its next
  regeneration. `ROADMAP.md` *Backlog*; a decision to take with the
  operator, not under a bug report.
- **Rounded corners on widget artwork are ANSWERED, not owed.** `pdfceGUI`
  checked and there is nothing to scale. A genuine toggle needs a
  rounded-rectangle primitive first, scoped with the operator.
- **Teaching `reflow_block`'s planner the overlay.** It is base-indexed
  because it needs extraction provenance the staging buffer does not carry
  — the same reason its pre-existing already-edited refusal exists. It now
  refuses **by name** when the page set changed this session (decision
  `111`'s one named exception). Converting it is a real feature, not a
  cleanup.
- **`census_dangling` will never see a field-name target.** A boundary, not
  a bug: a name is not a reference. Do not "fix" the census by teaching it
  names.
- **C9 — `/StructParent` / `/OBJR` orphaned by an annotation delete.** Owed
  since `Pass 38.5` and scoped out again: different graph, different
  carrier, no name-string component.
- **ce-dimension tolerance, the ISO 286 fit classes** — needs a sourced
  class/table lookup this project does not have.

---

## §C — ★★ READ BEFORE WRITING CODE. Lessons carried forward from prior sessions, plus one new one.

1. **★★★★★ NEW, this filing — A GUARD WRITTEN FOR ONE CARRIER IS A CLAIM
   ABOUT A CLASS, AND SO IS ITS ERROR MESSAGE.** `refuse_if_in_page_tree`
   went from three callers to ten in `Pass 191.1` and its message still
   named only the two carriers it was built for. Every existing test
   asserted on the error's **variant**, which stayed right, so nothing
   caught the **text** drifting away from its widened caller set. `R219`'s
   trigger, extended from code to operator-facing strings — check a guard's
   *message*, not only its *logic*, whenever you add a caller to it.

2. **★★★★ A GREEN SUITE CAN BE *VACUOUS* RATHER THAN WEAK, AND RUNNING IT
   HARDER WILL NEVER SAY SO.** A per-verb test suite tests verbs; a
   property that lives *between* verbs (e.g. two edits in one session) has
   no home in it unless something is written specifically to hold that
   state. `crates/pdfce-core/tests/session_overlay_skew.rs` is the only
   place in the crate with that shape.

3. **★★★ ASSERT ON THE BYTES, NOT ON THE OUTCOME. A VERB'S OUTCOME STRUCT IS
   A READER THE VERB WRITES ITSELF.** `Pass 191.0`'s and `191.1`'s own
   acceptance criteria are sabotage-checked over saved bytes, not outcome
   structs, precisely because of this — an outcome struct can report success
   while the bytes it describes were never written.

4. **★★★ A MEMO'S KEY MUST BE THE WHOLE DEPENDENCY SET — AND WHERE THE
   INPUTS CANNOT NAME A DEPENDENCY, TAKE THE KEY FROM THE WALK'S OUTPUT.**
   `R237`'s founding shape: a memo key that cannot see a change is a stale
   index applied to the wrong object, not a stale answer.

5. **★★★ THE BASH TOOL ON THIS MACHINE STRIPS BACKSLASHES FROM QUOTED
   HEREDOCS.** Rust string-literal line continuations and Python escape
   sequences get eaten when source is written that way, and `cargo fmt` can
   then flatten the gap into innocuous-looking spaces. Author such content
   with the Write/Edit tools, never a heredoc. `tools/check-string-gaps.sh`
   is the backstop, not the first line of defence.

6. **★★★★ A `debug_assert` TELLS YOU *WHERE THE CHECK RUNS*, NEVER *WHAT THE
   SHIPPING BUILD DOES WHEN THE PROPERTY IS FALSE* — `R238`.** Before sizing
   any assertion, answer in writing: *panics anyway* / *returns an error* /
   **returns `Ok` and writes wrong bytes**. Only the third is urgent, and
   only measurement distinguishes them. `Pass 191.0`'s and `191.1`'s own
   pinned-worktree, pre-fix measurements are `R238` applied correctly, twice
   in a row.

7. **★★★ A GATE MINTED TO FIX A NUMBER PROTECTS THE PHRASING THAT PROMPTED
   IT — `R239`.** A check written against one document's exact wording of a
   fact will not catch the same fact phrased differently in a neighbouring
   document. `dc8cde7` (§0, closed) is the fix; `docs/ROADMAP.md`'s *Update
   protocol*, "How a figure is filed," is the project-visible half.

8. **A REFUSAL THAT FIRES FOR AN UNRELATED REASON IS NOT A GUARD.** Declined
   as a project standing rule at `n = 1` (two-instance promotion bar not
   yet met) but written to the cross-project RAG regardless:
   `D:/dev/rag/rust/a_refusal_that_fires_for_an_unrelated_reason_is_not_a_guard.md`.
   Mint trigger: a second instance anywhere in this project where a
   property-harness's "clean" summary is later shown to have masked a real
   finding because an unrelated refusal or short-circuit prevented its own
   assertion from running on the input that would have failed it.

---

## §D — State of the tree

★ **NOT RE-MEASURED by the 356th filing — no shell available to this role
this session, same as the 355th.** Everything below is carried forward from
the 355th filing's own §D and is now at least **two** filings staler than
stated. Per hard rule 8: **run the commands, do not trust the numbers
below.**

Run these rather than trusting a sentence:

```
python tools/check-ledger-numbers.py      # all four ceilings
bash tools/run-gates.sh                   # the full sweep; it derives its
                                          # own list, so do not memorise a count
git log --oneline -10                     # confirm dc8cde7's actual position
                                          # relative to e4b3481 / ee7866d —
                                          # this file does NOT assert it
git rev-list --count origin/main..main    # how far ahead
ls -lt D:/Dev/pdfce-backups/              # newest bundle
gh run list --limit 3                     # CI's colour, from GitHub
```

- **Unpushed-commit count: NOT ASSERTED by this filing.** The 355th filing's
  own figure (six commits unpushed, `HEAD` = `4dc6bcb`) predates `Pass 191.0`,
  `Pass 191.1`, `dc8cde7` and this filing's own edits, so repeating it would
  be stale by construction rather than merely old. Run `git rev-list --count
  origin/main..main` before pushing.
- **Backup bundle currency: NOT ASSERTED by this filing**, for the same
  reason. Run `ls -lt D:/Dev/pdfce-backups/` and `git rev-list --count
  <bundle-hash>..HEAD` before trusting any figure about it.
- **Working-tree cleanliness: NOT ASSERTED by this filing** — no shell this
  session. Re-measure and attribute before staging anything; stage by path,
  never `git add -A`, per the standing instruction (the repository is
  public).
- **★ `R217` does NOT constrain pushing.** It constrains what may land **on
  top of** an unfiled commit. Read its amendment note in `ROADMAP.md`'s
  *Standing rules* before assuming otherwise.
- **Pushing `main` (ordinary fast-forward) is standing-authorized** (rule 8,
  decision `090` — *"always push"*); cutting a tag or release is **not**,
  and neither is a force push or a non-`main` branch push. Scrub
  `tools/check-suite-name-absent.py` green **before** pushing regardless —
  the repository is public, so a push publishes.
- **Read CI's colour from GitHub**, not from a sentence in a document.
