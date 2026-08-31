# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**. What is
true now, plus a pointer. Corrections and their prior wording live in the
**append-only** record — `ROADMAP.md` and `SESSION_LOG.md`.

Written 2026-08-31, end of the session-model / widget-appearance / form-geometry
session (`Pass 186.0` → `187.0` → `188.0`). **For the ledger — Pass ceiling,
standing-rule ceiling, decision ceiling, filing count — run
`python tools/check-ledger-numbers.py`.** It derives all four and is the only
thing that cannot be stale.

---

## §0 FOUR THINGS ARE OWED, AND THE FIRST IS UNCHANGED FROM THE LAST HANDOFF

Today's arc answered three inbound `pdfceGUI` requests end to end. A shell can
now edit an image it added a moment ago without saving first, resize a check box
and have it **redrawn** rather than magnified, and drag a node that lives inside
a form XObject. Three replies are out; **nothing is owed back on any of them.**

None of that touched the items below.

### ★ OWED ITEM 1 — fuzz finding #3, **still open, untouched, carried forward with its detail intact**

A `debug_assert_eq!` in **`delete_field_group`** at **`edit.rs:17486`**, where
the emptied-node **cascade** and its **prediction** disagree — specifically,
`remove_fields_from_form`'s `emptied` fixed point against
`group_deletion_preflight`'s tree walk. **Two independent derivations of one
quantity.**

**Sized honestly, and the sizing is part of the item.** It is a
`debug_assert_eq!`, so **in the build operators run it is a wrong
`nodes_removed` in a disclosure — not corruption.** Materially less severe than
the two page-tree destructions fixed in `Pass 185.1`/`185.2`, which it was found
beside. Do not spend a day at the wrong priority.

**Known:** the verb, the assertion, the location.
**Guess, labelled as one — start here but measure it:** the preview filters
`form.groups` by `fully_qualified_name != fqn` and then appends `fqn`, so **two
grouping nodes sharing one FQN** — trivially the empty name for a `/T`-less
node, which is exactly what a fuzzer reaches — would make the two derivations
count differently.

**How to run it, because the harness fights you:**
- **`-seed=1` makes a crash replayable.** Without it the same defect appears and
  vanishes across runs.
- **libFuzzer writes NO artifact here.** Rust's abort on Windows exits
  `0xc0000409` before the crash handler saves one, so there is nothing to
  reduce — consider making the target print its own input on panic.
- **Grep for the panic HEAD; never `tail` the output.** A `tail -40` keeps only
  libFuzzer's internal frames and drops both the message and the verb.
- ASan DLL path and invocation:
  `.claude/agent-memory/pdfce-engineer/reference_fuzz_asan_dll.md`.

⇒ **The absence of a reproduction is not evidence of a fix.**

### OWED ITEM 2 — `R236`'s one named uncovered site

**Annotation DELETION has no fuzz target.** `edit.rs:21669` carries the
postcondition *"the target was located on some page, so at least that page must
be patched"*; `annot_walk` **reads** and `annot_author` **writes**, and neither
deletes. **This is the concrete work `R236` creates**, and this session did not
do it.

The rule's full site table and its measured denominator (**24 invocations** — 12
in `pdfce-core`, 12 in `pdfce-render`, decomposed as `grep -c debug_assert` = 56
mentions = 24 invocations + 6 `cfg(debug_assertions)` + 2 `fn` definitions + 24
lines of prose) live in **`R236`'s own text** in `ROADMAP.md`'s *Standing
rules* — deliberately, because the census's first home was a file that gets
overwritten. This one.

### OWED ITEM 3 — the ten `cmyk_buffer` `debug_assert`s are **unaudited**

Render side. `mesh_shading` covers the `mesh.rs` pair; **the `cmyk_buffer.rs`
ten have never been classified** as *covered* / *open* / *exempt*. Second thing
to look at after item 2.

### OWED ITEM 4 — three stale claims in `docs/core-api/01-reading-and-model.md` (yours to fix)

Found by the 351st filing's hard-rule-11 sweep. **`02-editing-and-saving.md` was
swept thoroughly and is current; the correction stopped at that file's
boundary.** `01-reading-and-model.md` is the document a shell reads **first**:

| line | claim | state |
|---|---|---|
| `:1691` | *"A form leaf is reported as `leaf=N containment=… paint_order=… editable=false`"* | **FALSE** — `editable=` is real since `188.0` |
| `:1706` | field table row: `is_editable()` \| **always `false`** today | **FALSE** — it answers about the object (`true` for a path) |
| `~:1719` | *"For **selection**, use the deep test. For **editing**, use `hit_test_point` and you get back something you can actually edit."* | **MISLEADING** — a leaf is editable now; the sentence's real subject is that a leaf index must never be handed to an `objects`-indexed verb. Say that instead. |

★ **Two nearby hits are SURVIVING-AND-CORRECT — do not "fix" them:** `:1667`
(*"Nothing here is gated on `FormLeaf::is_editable()`, and that is correct"* — a
ce dimension against a form-interior line is a new **page** annotation, so the
reasoning is independent of editability) and `:1726` (the vocabulary note
pairing `stream()`/`is_editable()` with `text_extract`'s — **more** correct
after `188.0`). Likewise `02-editing-and-saving.md:2455` (*"the same model
`vector::decompose_page` returns"*) — that was the false claim `188.0` **fixed**;
it is true now.

### ★ WHAT THIS SESSION ADDED, so the arithmetic is legible

**One** new fuzz target — `fuzz/fuzz_targets/form_geometry_sequence.rs`,
**301,952 executions in 421 s = 717 exec/s, 0 crashes, 0 artifacts** — covering
`Pass 188.0`'s six new verbs at the moment they shipped. **It closes neither
owed item above.** The existing `vector_edit` target could not have covered them:
it drives the **planners** over one already-parsed content stream and cannot
reach leaf resolution, the containment walk, re-decomposition from a placement,
the selection guard or the reach count.

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

2. **The four owed items in §0.** Items 1–3 are fuzz work: item 1 is a bounded
   chase with a stated starting guess, item 2 is a new target over a verb family
   that already exists, item 3 is a classification pass rather than code. **Item
   4 is three sentences in a doc** and is fifteen minutes.

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

## §C — ★★ READ BEFORE WRITING CODE. Seven items from this session.

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

- **FOUR COMMITS ARE UNPUSHED** as of this handoff — the three Passes (`186.0`
  `7c2ee96`, `187.0` `52beaf6`, `188.0` `a8586cc`) plus an agent-memory chore
  (`d7c4675`). `HEAD` = `d7c4675`, `origin/main` = `1e63186`. The filing commit
  that accompanies this file makes **five**. **Pushing is standing-authorized**
  (rule 8, decision `090` — *"always push"*); cutting a tag or release is
  **not**, and neither is a force push or a non-`main` branch. Scrub
  `check-suite-name-absent.py` green **before** pushing regardless — the
  repository is public, so a push publishes. (It was run during the 351st
  filing and was clean; re-run it, do not carry that sentence.)
- **The backup bundle is 4 commits stale** — newest is
  `pdfce-20260830-2005-1e63186-full.bundle`, which is `origin/main`, not
  `HEAD`. A fresh one is cheap: `git bundle create <path> --all`.
- **★ `R217` does NOT constrain pushing.** It constrains what may land **on top
  of** an unfiled commit. Read its third amendment note before assuming
  otherwise.
- **Read CI's colour from GitHub**, not from a sentence in a document.
- **The working tree is clean of untracked files** as of this handoff. Two that
  earlier handoffs named — `.tmp_bench.py` (untracked for seven filings) and
  `.g3.log` (present at the start of the 351st filing) — are **no longer on
  disk**; neither was committed and neither is ignored by any rule. **The
  instruction they motivated stands unchanged: the repository is public, so
  stage by path, never `git add -A`.**
- ★ **The tree moved mid-filing twice this week.** `d7c4675` landed while the
  351st filing was being written, as `c17f1b5` and `8d8dbb5` did during the
  349th. **Re-measure git state before the commit, not only at the start** — a
  `git status` taken at the start of a filing is a measurement with a shelf
  life.
