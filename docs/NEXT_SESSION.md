# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**. What is
true now, plus a pointer. Corrections and their prior wording live in the
**append-only** record — `ROADMAP.md` and `SESSION_LOG.md`.

Written 2026-08-31, end of the session-model / widget-appearance / form-geometry
session (`Pass 186.0` → `187.0` → `188.0`), **updated the same day after
`Pass 189.0` (`baf0c29`) and after `Pass 190.0`/`190.1` (`77631a6`) and its
353rd filing**. **For the ledger — Pass ceiling, standing-rule ceiling, decision
ceiling, filing count — run `python tools/check-ledger-numbers.py`.** It derives
all four and is the only thing that cannot be stale.

---

## §0 ALL FOUR PRIOR OWED ITEMS ARE CLOSED. ONE NEW ONE IS OPEN, AND ITS SEVERITY IS UNMEASURED.

Today's arc answered three inbound `pdfceGUI` requests end to end and then
closed every open item this file was carrying. **Three replies are out; nothing
is owed back on any of them.**

★ **The closed items are kept below as one-line verdicts rather than deleted**,
so a reader can see they were **answered**, not dropped.

### ★★★ THE ONE OPEN ITEM — `annot_delete_sequence` STILL FIRES, on a route nothing here has sized

**A tracked reproducer exists, so this is a bounded chase, not a hunt:**

```
fuzz/corpus/annot_delete_sequence/seed_openbug_badkid_dupobjnum.bin    (1,618 B)
```

- **Signature: `BadKid(ObjId 3)`** — **not** `NoPageTreeRoot`, which is the one
  `Pass 190.1` fixed. Different mechanism, and **the catalog/`/Type` guard does
  not reach it.**
- **The shape:** a document with **two `3 0 obj` definitions**, where object 3
  is the page named by `/Kids`.
- **★★ WHETHER IT IS RELEASE-VISIBLE HAS NOT BEEN MEASURED.** That sentence is
  the item, not a caveat on it.

⇒ **Start by measuring exactly that**, and standing rule **`R238`** (minted
2026-08-31, this route is its first customer) says how: *what does the shipping
build do when this assertion would have fired?* The answer is one of *panics
anyway* / *returns an error* / **returns `Ok` and writes wrong bytes**, and only
the third makes it urgent. **Do not size it from the fact that it is a
`debug_assert` — that is the exact mistake `R238` exists to stop, and it cost a
day on the item closed immediately below.**

⇒ **Nothing anywhere claims this target runs clean.** `ROADMAP.md`'s `R236`,
its `Pass 190.0`/`190.1` entry, and the commit message all say so explicitly.

### ~~OWED ITEM 1 — fuzz finding #3 (`delete_field_group`)~~ — **CLOSED 2026-08-31 by `Pass 190.0` (`77631a6`)**

**VERDICT: the sizing this file carried was WRONG, and that is the reusable
part.** It read *"a `debug_assert`, so in release it is a wrong `nodes_removed`
in a disclosure — not corruption"*, and the item was de-prioritised on that for
a day. **Three of four shapes are release-visible**: a `/T`-less terminal made
the verb **return `Ok` and delete nothing**; a terminal with no `/Parent` made
it **write a dangling `/Kids`**. Two root causes — *a name is not an identity*
(§12.7.3.2: a `/T`-less node contributes no name segment and aliases its
parent's), and *`/Parent` is a back-link, not the structure*. The outcome struct
also spliced a COUNT from one derivation and a LIST from another, so
**`pdfce-cli` and `pdfceGUI` reported different numbers for one deletion** and
**`--dry-run` disagreed with the real run on a destructive verb**. ⇒ **`R238`
minted from this at `n = 2`.**

### ~~OWED ITEM 2 — `R236`'s one named uncovered site~~ — **CLOSED 2026-08-31 by `Pass 190.1` (`77631a6`)**

**VERDICT: `fuzz/fuzz_targets/annot_delete_sequence.rs` exists and is tracked**,
and **reachability was MEASURED, not assumed** (a temporary `stderr` probe
confirmed the guarded code runs during the corpus pass; the assert itself has
not been falsified — *reached* and *not tripped* are two facts). The site has
moved to **`edit.rs:23079`**.

★★ **`R236` NOW HAS NO OUTSTANDING WORK ITEMS**, and three things must be read
together or the state is misreported:

1. **The rule does NOT retire.** Its trigger is the postcondition **set**, which
   grows with the crate. Re-measure the population; do not remember it.
2. **An empty ledger is NOT a clean verb.** The `BadKid` route above fires at
   **helper 1**, a site `R236` has recorded **COVERED** since it was minted.
   **Finding a defect is what coverage is for.** `R236` measures whether every
   tripwire has an input source, not whether the verbs are correct.
3. **The denominator is 22** (12 `pdfce-core` + 10 `pdfce-render`), unchanged at
   `77631a6`. The census's full table lives in **`R236`'s own text** in
   `ROADMAP.md`'s *Standing rules* — deliberately, because its first home was
   this file, which gets overwritten.

★ **Its site table did not add up and now does** — it summed to 23 against its
own denominator of 22, the extra row being a **comment line** counted as an
invocation, in a rule whose own text names that very line as one it excluded.
⇒ **A correction that fixes a figure must sweep for the figure's derivatives.**

### ~~OWED ITEM 3 — the ten `cmyk_buffer` `debug_assert`s are unaudited~~ — **CLOSED 2026-08-31 by `Pass 189.0` (`baf0c29`)**

**VERDICT: 0 covered / 0 open / 4 EXEMPT / 6 VACUOUS. No fuzz target is owed by
that file.** Two defects found and fixed in the same commit: six of the ten
**checked width only** while claiming a shared device grid, and **`into_knockout`
is the one site whose violation is SILENT in release** (it replaces the
receiver's planes wholesale, so nothing runs off the end) — promoted to a
**runtime refusal**. Full per-site table in `R236`'s text.

★ **The survivor this left in `crates/` is FIXED** — `cmyk_buffer.rs:807`'s
*"eight sibling dimension guards"* now reads **seven siblings**, publishes the
**command** rather than a figure, and states **4 exempt + 4 vacuous = 8**. ★★
**And correcting it inflated the grep again — 13 → 17 raw hits, population held
at 8** — which is why it publishes a command: **a census figure quoted inside
the prose it counts is self-invalidating.**

★ **A REAL GAP THIS DID NOT FILL, filed to *Backlog* rather than charged to
`R236`: the harness has NO rendering fuzz target at all.** Reaching that module
needs one driving a full page render of a subtractive page (`/Group /CS
/DeviceCMYK`, isolated / non-isolated / knockout groups, `DeviceCMYK` images,
`/DeviceN` shadings, overprint). It is a **new target family**, not an extension
— every current `pdfce-render`-linking target takes a byte slice and calls a
leaf parser. **Scope it with the operator.**

### ~~OWED ITEM 4 — three stale claims in `docs/core-api/01-reading-and-model.md`~~ — **CLOSED 2026-08-31, fixed and pushed in `93dc9ba`**

All three corrected: the `editable=false` example at `:1691`, the *"always
`false`"* field-table row at `:1706`, the selection-vs-editing paragraph at
`~:1719`. **`02-editing-and-saving.md` was already current** — the correction
had stopped at that file's boundary, which was the whole finding.

★ **Two SURVIVING-AND-CORRECT hits stand, and a future sweep must not "fix"
them:** `:1667` and `:1726`. Likewise `02-editing-and-saving.md:2455`.

### ★ THE SWEEP SURVIVORS FROM `Pass 190.0`/`190.1`, so the next one does not re-open them

All **correct at `77631a6`** and deliberately left alone:

- **`FEATURES.md`** *"matched by prefix on a grouping node, so the subtree counts
  too"* — that clause is about `action_targets_orphaned`'s census, which **is**
  still a name-prefix match. (The neighbouring *"named subtree"* claim WAS
  falsified and is corrected.)
- **`docs/core-api/03-capabilities.md`** *"do not derive grouping nodes by
  splitting an FQN … the failure is silent"* — **more** correct after
  `Pass 190.0`.
- **`docs/core-api/02-editing-and-saving.md:2698`** — `FieldGroupDeletion`'s
  field table; the struct's public shape is **unchanged**, only the derivation
  was collapsed.
- **`cmyk_buffer.rs`** *"all ten assertions … four exempt … six vacuous"* —
  **4 + 6 = 10** pre-fix, cross-checking post-fix at **4 + 4 = 8**.

★ **Reported, not fixed:** `docs/core-api/`'s `edit.rs` line citations are
systematically stale (`:8464` / `:8574` / `:8764` against live `:12975` /
`:18713` / `:23079` in the same file). **Not a false claim — a stale
convenience.** Renumbering a contract document is a Pass, not a filing.

★★ **ALSO REPORTED, NOT FIXED, AND IT IS A REAL WRONG NUMBER:**
`docs/core-api/02-editing-and-saving.md`'s grouped error presenter says *"the
five groups below partition all **57**"* against an `EditError` enum of
**113**. **Pre-existing, not caused by `Pass 190.x`, and NO GATE COVERS IT** —
`check-core-api-verbs.py` checks `index.md`'s figure, not this one. Completing
and renumbering that list is a Pass.

### ★★★ RUN THE DOC GATES AS PART OF THE FILING, NOT ONLY BEFORE THE PUSH

`check-core-api-verbs.py` went **RED (exit 1)** during the 353rd filing:
`Pass 190.1` added one `EditError` variant (112 → **113**) and
`docs/core-api/index.md:17` still said 112. **The hard-rule-11 claim-sweep run
minutes earlier had missed it** — the sweep searches for *the claims the Pass
falsified*, and **an error-variant count is not a claim anyone would search for
after a deletion-guard Pass.** ⇒ **A count-gate and a claim-sweep return
different sets, and the gate's is not a subset of the sweep's.** Both are fixed
and the gate is green; the habit is the takeaway.

**★★★ AND THE SEQUEL, ONE HOUR LATER (354th filing, `2ab18dd`): THE SAME GATE
HAD ALREADY MISSED THE SAME NUMBER, IN THE FILE NEXT DOOR, FOR THREE DAYS.**
`docs/core-api/02-editing-and-saving.md` §6.7 carried *"**88 variants**"* and,
one sentence later, *"the five groups below partition all **57**"* — **two
stale figures for one quantity in one paragraph, contradicting each other**,
against a real **113**. The gate's `errvars` check was minted on **2026-08-28**
precisely because this role found `index.md` claiming **88**; it was written to
that copy's phrasing (`` `EditError`'s (\d+) variants ``) and **§6.7's copy of
the same wrong number, lacking the prefix, was invisible to it.**
⇒ **A GATE MINTED TO FIX A NUMBER PROTECTS THE PHRASING THAT PROMPTED IT** —
hard rule 11's *"a minted rule protects the document it was minted in"* with a
**gate** in the sweep's place. `R239` minted from this pair.

### ★★ NEW AND OWED — ONE LINE IN `tools/check-core-api-verbs.py`, WITH ITS MEASUREMENT ALREADY DONE

**`docs/core-api/02-editing-and-saving.md:3794` says *"113 variants"* and is
CORRECT AND UNGATED right now.** The next `EditError` variant makes it stale
**silently**, exactly as 88 did. `index.md:17`'s copy **is** gated.

**The fix is a proximity anchor, not a wider regex** — measured in the 354th
filing over `docs/core-api/`:

| pattern | hits | false positives |
|---|---|---|
| `[0-9]+ variants`, bare | 7 | ★ **5** (`CommandKind` 46, `xref` 12, `SnapKind` 8, `FunctionError` ~28, `Destination` 6) |
| same line **also names `EditError`** | **2** | ★ **0** — and both are the right targets |

⇒ **Narrow the file set and widen the pattern** (hard rule 11 clause (e)
verbatim). ★ **Do not "fix" `FunctionError`'s *"~28 variants"*** — the tilde is
honest about being unchecked and is doing real work.
★ **The same measurement names five more ungated derivable counts in that
directory. Scoping those is a Pass, not a filing.**

### ★ TWO RAG FINDINGS ARE OWED AND NEITHER IS WRITTEN

- `D:/dev/rag/rust/an_assertions_compile_time_gating_is_not_a_severity_estimate.md`
  — `R238`'s cross-project derivation (owed since the 353rd filing). A second
  candidate from the same commit: *a guard written for one carrier is a claim
  about a class* (`R219`'s widened trigger).
- `D:/dev/rag/rust/publish_the_command_not_the_count.md` — `R239`'s derivation
  (owed from the 354th).

**Reported rather than claimed.**

### ★ WHAT THIS SESSION ADDED, so the arithmetic is legible

**Two** new fuzz targets. `fuzz/fuzz_targets/form_geometry_sequence.rs` —
**301,952 executions in 421 s = 717 exec/s, 0 crashes, 0 artifacts** — covering
`Pass 188.0`'s six new verbs at the moment they shipped. And
`fuzz/fuzz_targets/annot_delete_sequence.rs` (`Pass 190.1`), which **found a
release-silent page-tree destruction within seconds of first existing**, on a
**173-byte** input, and **still fires** on the second route in §0 above.

---

## §A — Candidates, ordered by my read of value. None is a commitment.

1. **★ CHECK BOTH FeatureRequests CHANNELS FIRST.** They live outside the repo
   so no gate can contradict a stale "it's empty":
   - `D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\`
   - `D:\Dev\FeatureRequests\iccce_FeatureRequests\open\`

   **Three inbound requests were answered by shipped work today and still sit in
   `open/`** (`request_edit_verbs_read_the_base_not_the_overlay`,
   `request_resizing_a_check_box_stretches_its_appearance`,
   `request_editing_through_recursion_into_a_form_xobject`), with **three
   outbound replies beside them**. **Nothing is owed back.**

2. **The ONE open item in §0 — the `annot_delete_sequence` `BadKid` route.**
   All four items this file previously carried are **CLOSED** (`Pass 189.0`
   `baf0c29`, `93dc9ba`, and `Pass 190.0`/`190.1` `77631a6` for the other two).
   The new one has a **tracked reproducer** and **no severity estimate**, which
   makes step one *measure the release behaviour*, per `R238`. ⇒ A bounded
   chase, not a hunt — but **do not assume it is small because it fires a
   `debug_assert`**; that assumption is what cost a day on the item it replaced.

3. **`Pass 142.0`** — a font face outside the standard 14. The largest remaining
   *named* feature, **de-prioritised by the consuming project's own use report**,
   not by us: *"Synthetic is enough. Drop `142.0` down the queue."* Not closed,
   not declined.
   ★ `bind_font_resource` (`text_edit/addtext.rs`) is the single implementation
   of "add a `/Font` entry"; `142.0` extends it and does not write a second one.
   And there are **THREE save paths** — `EditSession::format_text`, its form
   twin, and the one-shot `text_edit::set_format`, **which is the one the CLI
   uses**. `Pass 162.0` wired two, every unit test passed, and the binary printed
   a disclosure about a resource it had not written.

4. **The `text_edit` resolver residual** (`ROADMAP.md` *Backlog*, filed today).
   `edit_text`, `format_text` and the two `preview_*` verbs still pass
   `&self.base` to the text planner **as the object resolver**, so a `/Font`
   created this session cannot be resolved through it. **The outcome is a clean
   refusal, not a wrong edit, and it is unchanged from before `Pass 186.0`.**
   ★ **Re-size it before scheduling it.** The obvious shape is threading a view
   through `text_edit`'s ~40 `doc: &Document` signatures — wide, mechanical, in
   the crate's most defect-prone module — and §C item 2 below is exactly the
   warning that the obvious implementation's difficulty is not the problem's.

5. **`/AP` `/D` (the pressed appearance) and `/MK` icon/label layout**, the
   remainder of `Pass 131.0`. ★ It is **appearance work** (`R43`'s
   neighbourhood), not a continuation of the button-action work — a session
   scoping it as "the rest of the button actions" would scope the wrong thing.

6. **The n-channel (per-spot-colorant) buffer** — the only path to the print
   suite's remaining overprint/spot FAILs. **Operator's call; do not scope it
   without him.**

7. **`CmykIntent::Calibrated`'s cool greys** — three independent lines of
   evidence, and **still not ours to fix** (decision `064` puts the conversion in
   `iccce`'s domain; the operator ruled the default 2026-08-28).
   ★ **The black end of that same table is a FALSE-DEFECT TRAP** — pdfce is the
   *closer* answer there and `iccce` said so unprompted. Read `settings/mod.rs`'s
   doc comment before touching anything CMYK.

---

## §B — What is deliberately NOT being worked, and why

- **`/BS` `/W` does not change a check box's or radio button's drawn border.**
  pdfce authors it at a fixed **1.0**. That is **the artwork's existing
  contract**, not an oversight: honouring `/BS` `/W` would alter how **every
  pdfce-authored check box already in the wild** renders on its next
  regeneration. `ROADMAP.md` *Backlog*; **a decision to take with the operator,
  not under a bug report.**
- **Rounded corners on widget artwork are ANSWERED, not owed.** `pdfceGUI`
  checked and there is nothing to scale. A genuine toggle needs a
  **rounded-rectangle primitive** first, scoped with the operator.
- **Teaching `reflow_block`'s planner the overlay.** It is base-indexed because
  it needs extraction provenance the staging buffer does not carry — the same
  reason its pre-existing already-edited refusal exists. It now **refuses by
  name** when the page set changed this session (decision `111`'s one named
  exception). Converting it is a **real feature**, not a cleanup.
- **`census_dangling` will never see a field-name target.** That is a boundary,
  not a bug: a name is not a reference. Do not "fix" the census by teaching it
  names.
- **C9 — `/StructParent` / `/OBJR` orphaned by an annotation delete.** Owed since
  `Pass 38.5` and scoped out again: different graph, different carrier, no
  name-string component.
- **ce-dimension tolerance, the ISO 286 fit classes** — needs a sourced
  class/table lookup this project does not have.

---

## §C — ★★ READ BEFORE WRITING CODE. Eleven items from this session.

1. **★★★★ A GREEN SUITE CAN BE *VACUOUS* RATHER THAN WEAK, AND RUNNING IT HARDER
   WILL NEVER SAY SO.** All **4,861** tests were green **before and after**
   `Pass 186.0`'s fix for a verb that addressed the wrong sheet. Every test that
   exercised a content-editing verb did so on a session whose page set had not
   been structurally edited and whose content had not been appended this session
   — and **on such a session base and overlay agree by construction.**
   ⇒ **Before trusting a suite to protect a property, ask what STATE the
   property needs in order to exist.** This one needs **two verbs in one
   session**; a per-verb suite tests verbs, and a property that lives *between*
   verbs has no home in it. `crates/pdfce-core/tests/session_overlay_skew.rs` is
   the only place in the crate with that shape — do not delete it in a tidy-up.

2. **★★★ ASSERT ON THE BYTES, NOT ON THE OUTCOME. A VERB'S OUTCOME STRUCT IS A
   READER THE VERB WRITES ITSELF.** `Pass 187.0`'s worst route left `/Rect` at
   140×22, rebuilt the artwork at 380×100, and returned `resized: true` with a
   `rect_after` naming a box **never written**. Every assertion anyone had
   written was on the outcome. **On three of that Pass's four routes the outcome
   was correct and the bytes were not.** `R159`'s mechanism, with the return
   value playing the lenient parser.

3. **★★★ A MEMO'S KEY MUST BE THE WHOLE DEPENDENCY SET — AND WHERE THE INPUTS
   CANNOT NAME A DEPENDENCY, TAKE THE KEY FROM THE WALK'S *OUTPUT*.** Minted
   today as **`R237`** at `n = 2`, both in `PageModelKey`: it could not see an
   appended content stream or a changed `/Resources` (`186.0`), and could not see
   a form-stream rewrite (`188.0`). **Both times every verb returned `Ok` while
   the model was stale — and a stale index is not a stale answer, it is an edit
   applied to the wrong object.** *Which* forms a page paints cannot be computed
   before the walk that finds them, so the key comes from `containment`, the
   walk's own output. **Sabotage the KEY, not the value**, and note it reddens
   nothing until a two-verbs-in-one-session test exists.

4. **★★ A CONSISTENCY FIX CAN PRODUCE A WORSE INCONSISTENCY THAN THE ONE IT
   REMOVES.** Swapping `reflow_block`'s surrounding reads to the overlay without
   a guard would have spliced one sheet's reflowed bytes into a **different**
   sheet's content object. **Two wrong readings that agree with each other are
   survivable; one right and one wrong is not.** Where a component cannot be
   converted in the same change, **make it refuse** — do not leave its old
   reading beside the new one.

5. **★★ A FIXTURE SET CAN BE COMPLETE IN COVERAGE AND DEGENERATE IN
   DISCRIMINATION.** All **seven** pre-existing form fixtures placed their form
   with a **pure translation** — the one transform under which a wrong
   page-space → form-space conversion still looks plausible (a 10 pt drag moves
   10 pt either way, just from the wrong origin).
   `fixtures/synthetic/forms-xobject/scaled-form-placement.pdf` places its form
   at `2 0 0 2 40 30 cm`, so the answers differ **in magnitude**.
   ⇒ **Ask what property all your fixtures share that nobody listed as a
   property.** Related: **a bounding box is a lossy instrument for a node move** —
   dragging a rectangle corner *inward* does not change the box, because the two
   adjacent corners still hold the extremes.

6. **★★ A GUARD KEYED ON THE SHARED THING IS NOT A GUARD ON THE THING THAT MUST
   BE UNIQUE.** `FormLeafSelectionSpansForms` requires a multi-leaf selection to
   sit in **one invocation**, not merely one **form**: two invocations produce
   leaves naming the same form with different placements whose
   `form_object_index` values **collide**. Accepting such a selection moves one
   object twice through two different matrices — silently wrong, not refused.

7. **★★★ THE BASH TOOL ON THIS MACHINE STRIPS BACKSLASHES FROM QUOTED
   HEREDOCS.** Rust string-literal line continuations (a trailing backslash) and
   Python escape sequences are **eaten** when source is written that way. **It
   happened four times this session, one debugging cycle each.**
   `tools/check-string-gaps.sh` caught the one that reached a committed file — a
   refusal message that would otherwise have shipped with a ten-space hole in
   it. ⇒ **Author any such content with the Write/Edit tools, never a heredoc.**
   ★ Compounding hazard: a surviving line continuation can then be **flattened
   by `cargo fmt`**, which rejoins the literal and leaves the eaten indentation
   as a run of spaces — the corruption survives *and* is normalised into
   something that looks intentional. Written up in `D:/dev/rag/rust/`.

8. **Patch-script hazards, both of which destroy work silently.**
   `pathlib.write_text()` rewrites a whole file to CRLF unless you pass
   `newline`; and a sabotage loop that asserts on a *later* case leaves an
   *earlier* case's sabotage on disk if the restore is after the loop rather than
   in a `finally`. Validate every anchor before touching the file.

9. **A surviving sabotage has three causes and only one is a weak test:** an
   assertion that cannot see the change, a guarantee enforced elsewhere, and a
   mutation that is semantically a no-op. Ask in that order. **It paid out today
   on its first use** — two `Pass 186.0` tests survived the first sabotage; one
   was **vacuous** (a baseline taken *after* the change it was measuring) and one
   **passed for the wrong reason** (a failure from an unrelated resolution error
   that looked like the right answer). Both were **strengthened, not kept.**

10. **★ After amending a document, grep for the CLAIM you just falsified, not
    for the section you just edited.** `Pass 188.0` did this correctly on its own
    side: `FormLeaf::is_editable()` changing from a hard `false` falsified a
    property table, a load-bearing-properties paragraph, a test's own name and
    doc block, `linepick.rs`'s note **and** a hard-coded `editable=false` in
    shipped CLI output — all five corrected in the same commit.
    ⇒ **A placeholder that documents its own provisionality is still a false
    claim once it is wrong**, and the doc comment saying so does not reach the
    operator.

11. **★★★★ A `debug_assert` TELLS YOU *WHERE THE CHECK RUNS*, NEVER *WHAT THE
    SHIPPING BUILD DOES WHEN THE PROPERTY IS FALSE* — minted as `R238`
    (`Pass 190.0`).** Fuzz finding #3 was sized as *"a `debug_assert`, so in
    release it is a wrong number in a disclosure — not corruption"*, that
    sentence was propagated **verbatim to three documents**, and the item sat a
    day at the wrong priority. Measured, **three of four shapes are
    release-visible** and two are worse than a wrong number: **returns `Ok` and
    deletes nothing**, and **writes a dangling `/Kids`**.
    ⇒ **Before sizing any assertion, answer in writing: *panics anyway* /
    *returns an error* / **returns `Ok` and writes wrong bytes***. Only the
    third is urgent, and only measurement distinguishes them.
    ★ **The paragraph carrying the wrong sizing was the one headed *"Severity is
    part of the finding, not a footnote"*.** The moral survived; the figure did
    not. **Stating a severity is not measuring one**, and the wrong sizing
    looked *exemplary* — specific, hedged, warning against over-prioritising —
    which is exactly why nobody checked it.
    ★★ **Companion, same commit:** *a guard written for one carrier is a claim
    about a CLASS.* `refuse_if_in_page_tree` was built for `/AcroForm`
    `/Fields` and **never called from the `/Annots` half**, which cost a second
    release-silent page-tree destruction a day later. `R219` already required
    the enumeration and is **widened** to say so for carriers, not just routes.
    Its unchecked carrier list is in §0.

---

## §D — State of the tree

Run these rather than trusting the sentence:

```
python tools/check-ledger-numbers.py      # all four ceilings
bash tools/run-gates.sh                   # the full sweep; it derives its
                                          # own list, so do not memorise a count
git rev-list --count origin/main..main    # how far ahead
ls -lt D:/Dev/pdfce-backups/              # newest bundle
gh run list --limit 3                     # CI's colour, from GitHub
```

- **SIX COMMITS ARE UNPUSHED**, re-measured by `git rev-parse` /
  `git rev-list --count origin/main..main` at the start of the **354th**
  filing: `baf0c29` (`Pass 189.0`), `123b437` (`chore(agent-memory)`),
  `77631a6` (`Pass 190.0`/`190.1`), `4c178ab` (353rd filing), `2ab18dd`
  (`docs(core-api)`) and `4dc6bcb` (`render`, the `R236` exemption's own
  sibling count). `HEAD` = **`4dc6bcb`**, `origin/main` = **`93dc9ba`**
  (unmoved). The filing commit that accompanies this file makes **seven**.
  ★ **The engineer reported a full gate sweep running at that moment and will
  push when it is green**, so this figure is the pre-push one by construction.
  ~~**THREE COMMITS ARE UNPUSHED** … `HEAD` = `77631a6` — measured at the end
  of the 353rd filing, superseded above.~~ **Pushing is
  standing-authorized** (rule 8, decision `090` — *"always push"*); cutting a tag
  or release is **not**, and neither is a force push or a non-`main` branch.
  Scrub `check-suite-name-absent.py` green **before** pushing regardless — the
  repository is public, so a push publishes. **It was NOT run in the 352nd or
  353rd filings**, so there is no sentence here to carry: run it.
- **The backup bundle is 12 commits stale from `HEAD`, 6 from `origin/main`** —
  newest is `pdfce-20260830-2005-1e63186-full.bundle` (2026-08-30 20:05, by
  `ls -lt /d/Dev/pdfce-backups/`; counts by `git rev-list --count 1e63186..HEAD`
  and `..origin/main`, **both re-run in the 354th filing**, up from 9/6 in the
  353rd). **It is no longer even `origin/main`**, and it has gone staler by
  three since the last filing said so. A fresh one is cheap:
  `git bundle create <path> --all`.
- **★ THE WORKING TREE WAS CLEAN OF TRACKED MODIFICATIONS** at the start of the
  **354th** filing — untracked **`.fz.log`** and **`.gfinal.log`** only, neither
  committed nor ignored by any rule. Everything the 353rd filing saw in flight
  landed: `crates/pdfce-render/src/cmyk_buffer.rs` as **`4dc6bcb`**, that
  filing's own seven `docs/` files as **`4c178ab`**, and the §6.7 correction as
  **`2ab18dd`**.
  ⇒ **Re-measure and attribute before staging anything**; *stage by path, never
  `git add -A`* is not a tidiness note when agents share a tree. ⇒ **A
  `git status` has a shelf life of minutes here** — the **fifth** consecutive
  filing whose tree moved underneath it, and this one's figure was taken
  **before** the engineer's gate sweep finished.
- **★ `R217` does NOT constrain pushing.** It constrains what may land **on top
  of** an unfiled commit. Read its third amendment note before assuming
  otherwise.
- **Read CI's colour from GitHub**, not from a sentence in a document.
- **Untracked files come and go and none has ever been committed.**
  `.tmp_bench.py` (untracked for seven filings, and still present in this
  session's own opening `git status`) and `.g3.log` are **gone**; `.fz.log` is
  **new**. None is ignored by any rule. **The instruction they motivate is
  unchanged and is not a tidiness note: the repository is public, so stage by
  path, never `git add -A`.**
- ★ **The tree moved mid-filing twice this week.** `d7c4675` landed while the
  351st filing was being written, as `c17f1b5` and `8d8dbb5` did during the
  349th. **Re-measure git state before the commit, not only at the start** — a
  `git status` taken at the start of a filing is a measurement with a shelf
  life.
