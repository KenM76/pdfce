# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**. What is
true now, plus a pointer. Corrections and their prior wording live in the
**append-only** record — `ROADMAP.md` and `SESSION_LOG.md`.

Written 2026-08-31, rewritten by the 357th filing after five commits shipped
in one session: `Pass 193.0` (structure inspection/dump), `Pass 194.0`
(editable structure round trip), `Pass 192.1` (isolated luminosity mask,
spec-correct, no observable change), `Pass 192.0` (the Bevel-and-Emboss
shadow-half root cause, found and fixed), `Pass 195.0` (a mixed `/DeviceN`'s
discarded spot ink), and `Pass 196.0`/`196.1` (two false-green fixes: a new
`suite-check.py` `CRIT?` verdict, and the CLI overprint note's corrected
"nothing is owed there" claim). **§0's prior active/queued items
(`Pass 193.0`, `Pass 192.0`) are both SHIPPED — nothing is currently active
or queued from this file's own §0.** **For the ledger — Pass ceiling,
standing-rule ceiling, decision ceiling, filing count — run
`python tools/check-ledger-numbers.py`.** It derives all four and is the
only thing that cannot be stale. **This role had no shell this session;
nothing below marked "measured" or with a command attached was re-run
here — confirm before trusting a number in this file.**

---

## §0 NOTHING IS CURRENTLY ACTIVE OR QUEUED FROM A PRIOR §0. See §A for candidates.

★ **Closed items are kept below as one-line verdicts rather than deleted**,
so a reader can see they were **answered**, not dropped.

### ~~`Pass 193.0` — PDF internal-structure inspection/dump~~ — **SHIPPED 2026-08-31 by `Pass 193.0` (`4ee56bb`), filed here by the 357th filing**

**VERDICT: shipped**, beyond the provisional scope this file's prior
revision recorded — `pdfce-core::structure` + `pdfce-cli dump-object`/
`dump-structure`/`list-objects`, plus the physical layout (xref style,
`/ObjStm` membership, incremental-revision/trailer chain, linearization)
and a reverse-reference map. Acrobat has no machine-readable equivalent at
all (parity-plus, per `pdfce-acrobat-librarian`). One test was found
**silently vacuous and fixed before shipping** — it pointed at a corpus
file with no object streams, took its skip branch, and reported `ok`
while asserting nothing. **Full record:** `ROADMAP.md`'s `Pass 193.0`
*Shipped* entry.

### ~~`Pass 192.0` — the green Bevel-and-Emboss cell's missing shadow half~~ — **SHIPPED 2026-08-31 by `Pass 192.0` (`185500d`), filed here by the 357th filing**

**VERDICT: root cause found and fixed.** A second `gs /SMask` REPLACES the
mask in force (ISO 32000-1 Table 58) — pdfce was multiplying onto it
instead, so a bevel's two complementary masks collapsed to ≈0 and the
shadow vanished. Measured: cell mean-abs-diff 29.70 → 2.95 (of 18,496 px);
pixels >30, 6,466 → 155; wrong quadrant's luminance 101.3 → 58.5 against a
reference of 58.8. **A prior "refuted by ablation" verdict for this defect
is corrected in the same filing** — the ablation forced a function
(`overprint::classify`) this content's route never calls, so its null
result was ineffective, not exculpatory; assessed as a standing-rule
candidate and **declined** (fourth phrasing of an already-recorded idea;
written up as a cross-project RAG finding instead). **Full record:**
`ROADMAP.md`'s `Pass 192.0` *Shipped* entry.

---

## §A — Candidates, ordered by my read of value. None is a commitment.

1. **★ CHECK BOTH FeatureRequests CHANNELS FIRST.** They live outside the repo
   so no gate can contradict a stale "it's empty":
   - `D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\`
   - `D:\Dev\FeatureRequests\iccce_FeatureRequests\open\`

   **Not re-checked by this filing — no shell available to this role this
   session.** Last checked by the 355th filing (three answered requests
   still sitting in `open/` with outbound replies beside them).

2. **The four new *Backlog* entries filed this session**, all naming a
   residual of `Pass 196.0`/`196.1`'s fixes:
   - A committed test pins a known-wrong value —
     `crates/pdfce-render/tests/grey_overprint.rs`'s third assertion,
     re-derive once the n-channel buffer lands.
   - A disclosure counter for "a process-space image over an already-
     flattened spot" — `overprint_images_unsupported` should fire here and
     reads 0.
   - `tools/overprint_image.rs`'s CI signature table is missing the row
     that would catch the above.
   - Larger, unscoped: invert `tools/suite-check.py`'s default so `clean`
     requires the patch's own text to state a criterion this harness
     checks — needs an X-language detector first; scope with the operator.

3. **`Pass 142.0`** — a font face outside the standard 14. The largest
   remaining *named* feature, **de-prioritised by the consuming project's
   own use report**, not by us: *"Synthetic is enough. Drop `142.0` down the
   queue."* Not closed, not declined.
   ★ `bind_font_resource` (`text_edit/addtext.rs`) is the single
   implementation of "add a `/Font` entry"; `142.0` extends it and does not
   write a second one. Three save paths exist — `EditSession::format_text`,
   its form twin, and the one-shot `text_edit::set_format` (the one the CLI
   uses); `Pass 162.0` wired two, every unit test passed, and the binary
   printed a disclosure about a resource it had not written.

4. **The `text_edit` resolver residual** (`ROADMAP.md` *Backlog*).
   `edit_text`, `format_text` and the two `preview_*` verbs still pass
   `&self.base` to the text planner **as the object resolver**, so a `/Font`
   created this session cannot be resolved through it. **The outcome is a
   clean refusal, not a wrong edit**, unchanged since before `Pass 186.0`.
   ★ **Re-size it before scheduling it** — the obvious shape is threading a
   view through `text_edit`'s ~40 `doc: &Document` signatures, wide and
   mechanical, in the crate's most defect-prone module.

5. **`/AP` `/D` (the pressed appearance) and `/MK` icon/label layout**, the
   remainder of `Pass 131.0`. ★ It is **appearance work** (`R43`'s
   neighbourhood), not a continuation of the button-action work.

6. **The n-channel (per-spot-colorant) buffer** — the only path to the print
   suite's remaining overprint/spot FAILs. **Now a FIVE-member bucket**,
   not four: `PCS2_020`/`030`/`031`/`040`/`081` (`PCS2_031` joined this
   session, per `Pass 196.0`). **Operator's call; do not scope it without
   him.**

7. **`CmykIntent::Calibrated`'s cool greys** — still not ours to fix
   (decision `064` puts the conversion in `iccce`'s domain; the operator
   ruled the default 2026-08-28). ★ **The black end of that same table is a
   FALSE-DEFECT TRAP** — pdfce is the *closer* answer there and `iccce` said
   so unprompted. Read `settings/mod.rs`'s doc comment before touching
   anything CMYK.

8. **`resize_annotation`'s `/AP` `/N` overwrite gap** (`ROADMAP.md`
   *Backlog*, filed by `Pass 191.1`) — needs the whole-document
   `/Contents`-stream reference census `appearance_streams_owned_by`'s own
   doc comment already names as an open bound, not merely a type test.

9. **Five ungated derivable counts in `docs/core-api/`** (found 354th
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

1. **★★★★★ NEW, this filing — AN ABLATION OF CODE OFF THE EXECUTION PATH
   PROVES NOTHING ABOUT THAT PATH.** A prior session forced
   `overprint::classify` unconditionally, observed no change in a bevel's
   rendering, and recorded overprint classification as **refuted** as the
   cause. The bevel's shadow-layer content never calls `classify` at all —
   the ablation ran on nothing, so its silence was guaranteed by placement,
   not evidence. The real cause (`Pass 192.0`) was a second `gs /SMask`
   REPLACING rather than intersecting the mask in force (Table 58).
   **Before trusting a "no change under ablation" result, prove the
   ablated function was actually called on the input under test** — a hit
   counter or `debug_assert!` inside it, not an inference from its general
   presence in the binary. Declined as a numbered standing rule (fourth
   phrasing of an already-recorded family); written to
   `D:/dev/rag/rust/an_ablation_of_code_off_the_execution_path_proves_nothing.md`.

2. **★★★★★ A GUARD WRITTEN FOR ONE CARRIER IS A CLAIM ABOUT A CLASS, AND SO
   IS ITS ERROR MESSAGE.** `refuse_if_in_page_tree` went from three callers
   to ten in `Pass 191.1` and its message still named only the two carriers
   it was built for. Every existing test asserted on the error's
   **variant**, which stayed right, so nothing caught the **text** drifting
   away from its widened caller set. `R219`'s trigger, extended from code
   to operator-facing strings — check a guard's *message*, not only its
   *logic*, whenever you add a caller to it.

3. **★★★★ A GREEN SUITE CAN BE *VACUOUS* RATHER THAN WEAK, AND RUNNING IT
   HARDER WILL NEVER SAY SO.** A per-verb test suite tests verbs; a
   property that lives *between* verbs (e.g. two edits in one session) has
   no home in it unless something is written specifically to hold that
   state. `crates/pdfce-core/tests/session_overlay_skew.rs` is the only
   place in the crate with that shape. **This filing's own instance:**
   `Pass 193.0`'s test suite had exactly this shape (a skip branch silently
   passing on a fixture with no object streams) and it was caught pre-ship.

4. **★★★ ASSERT ON THE BYTES, NOT ON THE OUTCOME. A VERB'S OUTCOME STRUCT IS
   A READER THE VERB WRITES ITSELF.** `Pass 191.0`'s and `191.1`'s own
   acceptance criteria are sabotage-checked over saved bytes, not outcome
   structs, precisely because of this — an outcome struct can report success
   while the bytes it describes were never written.

5. **★★★ A MEMO'S KEY MUST BE THE WHOLE DEPENDENCY SET — AND WHERE THE
   INPUTS CANNOT NAME A DEPENDENCY, TAKE THE KEY FROM THE WALK'S OUTPUT.**
   `R237`'s founding shape: a memo key that cannot see a change is a stale
   index applied to the wrong object, not a stale answer.

6. **★★★ THE BASH TOOL ON THIS MACHINE STRIPS BACKSLASHES FROM QUOTED
   HEREDOCS.** Rust string-literal line continuations and Python escape
   sequences get eaten when source is written that way, and `cargo fmt` can
   then flatten the gap into innocuous-looking spaces. Author such content
   with the Write/Edit tools, never a heredoc. `tools/check-string-gaps.sh`
   is the backstop, not the first line of defence.

7. **★★★★ A `debug_assert` TELLS YOU *WHERE THE CHECK RUNS*, NEVER *WHAT THE
   SHIPPING BUILD DOES WHEN THE PROPERTY IS FALSE* — `R238`.** Before sizing
   any assertion, answer in writing: *panics anyway* / *returns an error* /
   **returns `Ok` and writes wrong bytes**. Only the third is urgent, and
   only measurement distinguishes them.

8. **★★★ A GATE MINTED TO FIX A NUMBER PROTECTS THE PHRASING THAT PROMPTED
   IT — `R239`.** A check written against one document's exact wording of a
   fact will not catch the same fact phrased differently in a neighbouring
   document. `docs/ROADMAP.md`'s *Update protocol*, "How a figure is
   filed," is the project-visible half. **This filing's own instance of the
   underlying failure mode (not `R239` itself, but its close cousin): a
   stale FIGURE (`PCS2_031 = 1, a patch that PASSES`) survived unrevised in
   FIVE separate documents for over a week** after the counter it was
   drawn from changed meaning at `Pass 130.2` — found only by a
   hard-rule-11 sweep, not by any gate.

9. **A REFUSAL THAT FIRES FOR AN UNRELATED REASON IS NOT A GUARD.** Declined
   as a project standing rule at `n = 1` (two-instance promotion bar not
   yet met) but written to the cross-project RAG regardless:
   `D:/dev/rag/rust/a_refusal_that_fires_for_an_unrelated_reason_is_not_a_guard.md`.
   Mint trigger: a second instance anywhere in this project where a
   property-harness's "clean" summary is later shown to have masked a real
   finding because an unrelated refusal or short-circuit prevented its own
   assertion from running on the input that would have failed it.

10. **A TEST-HARNESS "CLEAN" DEFAULT CAN MASK A DETECTOR GAP, NOT JUST A
    RENDERER BUG.** `tools/suite-check.py`'s third false-pass class
    (`Pass 196.0`) is the same failure mode as the `REF`/`MARK?` gaps
    before it: a patch whose criterion the harness has no detector for
    fell through to `clean` by default, three separate times, three
    separate fixes. The larger, unscoped fix (invert the default) is
    §A item 2's last bullet, above — worth reading before proposing a
    fourth patch-around.

---

## §D — State of the tree

★ **NOT RE-MEASURED by the 357th filing — no shell available to this role
this session, same as the 355th and 356th.** Everything below is carried
forward and is now at least **three** filings staler than stated. Per hard
rule 8: **run the commands, do not trust the numbers below.**

Run these rather than trusting a sentence:

```
python tools/check-ledger-numbers.py      # all four ceilings
bash tools/run-gates.sh                   # the full sweep; it derives its
                                          # own list, so do not memorise a count
git log --oneline -10                     # confirm the five commits this
                                          # filing records land in the order
                                          # this filing assumed — NOT verified
                                          # by this role this session
git rev-list --count origin/main..main    # how far ahead
ls -lt D:/Dev/pdfce-backups/               # newest bundle
gh run list --limit 3                     # CI's colour, from GitHub
```

- **Unpushed-commit count: NOT ASSERTED by this filing.** The five commits
  this filing records (`4ee56bb`, `bd2df96`, `7360696`, `185500d`,
  `4299174`) are additional to whatever was unpushed at the 356th filing's
  own last-measured figure, which this filing does not repeat because
  repeating it would be stale by construction. Run `git rev-list --count
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
