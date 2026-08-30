# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**. What is
true now, plus a pointer. Corrections and their prior wording live in the
**append-only** record — `ROADMAP.md` and `SESSION_LOG.md`.

Written 2026-08-30. Ledger: **Pass ceiling 174.6**, rules **R234**, decisions
**104**, filings **330**. ★ Do not trust that line — run
`python tools/check-ledger-numbers.py`, which derives all four and is the only
thing that cannot be stale. It is stated here once because a reader wants an
order of magnitude, and it was already wrong within an hour of the first save.

---

## §0 — ★★★ THE COLOUR CLUSTER IS CLOSED

The operator's instruction was to **finish the colour work for the licensed
print-conformance suite** (his own wording named it; the name is scrubbed per
the 2026-08-25 ruling, and writing his sentence out verbatim is how this file
failed `check-suite-name-absent.py` on its first save) — the six-item colour
cluster in the previous handoff's §A, items 4–9, plus item 10.
**All of it is discharged.** This is the first session in weeks with no owed
colour item.

| was | now |
|---|---|
| **4.** the `PCS3_132` residual | already fixed (`Pass 165.0`); **the attribution is now SPLIT and the sibling has accepted it** — `Pass 174.0` |
| **5.** `CmykIntent::Calibrated`'s doc comment vs the evidence | **measured and corrected** — `Pass 174.1` |
| **6.** `PCS3_130` cells c/d, ~8 counts apart, "ours to look at" | **answered; the resampling hypothesis REFUTED by ablation; cause is in the file** — reported to the sibling |
| **7.** `PCS2_030`'s trap X, "cause unknown" | **attributed** — same bucket as `PCS020`/`PCS040`/`PCS081` — `Pass 174.2` |
| **8./9.** the withdrawn ΔE figures; both patches PASS their own criterion | honoured in every outbound; counts only, and the accuracy-gap-on-a-passing-panel distinction kept |
| **10.** two ambiguity-register entries owed to `pdfce-spec-librarian` since 2026-08-09 | **filed** (`OP-A5`, `LW-A1`), and the audit found **two of my statements wrong** — see §B |

**What is NOT closed and was never in scope:** the **n-channel
(per-spot-colorant) buffer**, which is the only path to the suite's remaining
overprint/spot FAILs. Unchanged, unscoped, and now carrying one more patch
(`PCS2_030`) that used to be a separate mystery. **Do not scope it without the
operator** — it is a large architectural change to the compositor.

---

## §A — OWED. This is the pick-from list.

1. **★★ THE ONE UNANSWERED INBOUND.**
   `D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\request_a_dotted_name_silently_swallows_an_existing_terminal_field.md`
   — from `pdfceGUI`, **2026-08-29 17:35, unanswered, and named in no pdfce
   document until the 330th filing.** Not colour, so it was out of scope for
   the session that found it. **Read it first**: a channel request outranks
   anything on this list, and the last two sessions' work both came from the
   channel rather than from here.

2. **★★ THE ce-DIMENSION TEXT OVERRIDE — decision 097, the operator's own
   ruling (2026-08-29).** Still the only item on this list Ken has personally
   specified, and still unstarted. Verbatim: *"dimension text override should
   be an option if it can be selected to be overridden or not so the override
   can be undone. otherwise we need to add a manual dimension tool where the
   user just enters the value of the dimension."*
   **Two branches with a condition, not one instruction:**
   - **Branch 1, preferred — the override is a SELECTABLE STATE.** The ce
     dimension **keeps its measured value** while an override is in force;
     overriding is a flag plus a string; **clearing it restores the measured
     text** with no re-measure, **and that must survive save-and-reopen** — so
     the measured value persists in the sidecar *alongside* the override.
   - **Branch 2, fallback only — a manual dimension tool.** A **different
     object class**, not an override. Do not build it as a substitute.
   - **Achievable:** `/PieceInfo` already carries pdfce's per-dimension state,
     so `override: Option<String>` beside the measured value is additive and
     round-trips.
   - ★ **Naming is constrained:** `Pass 163.0` made a refusal cite
     `set_dimension_label` and say it is NOT BUILT YET.
     `tools/check-cited-verbs-exist.py` enforces the pair.
   - **Prerequisite: the disclosure story.** What does a shell show when a
     dimension's text is not its measurement?

3. **`Pass 142.0`** — a font face **outside** the standard 14. The largest
   remaining item. Needs a real font program: subset from `--font-dir`, a
   `/FontDescriptor` with `/FontFile2`/`/FontFile3`, widths, encoding, and a
   **six-uppercase-letter subset tag unique across the WHOLE FILE** (§9.6.4
   ST4's uniqueness is per *file*, so the existing document's font names must
   be scanned, not just session state). De-prioritised, not declined.
   ★ **`bind_font_resource` (`text_edit/addtext.rs`) is the single
   implementation of "add a `/Font` entry"; `142.0` extends it, it does not
   write a second one.** And **there are THREE save paths**:
   `EditSession::format_text`, its form twin, and the one-shot
   `text_edit::set_format` — **which is the one the CLI uses.** `Pass 162.0`
   wired two, every unit test passed, and the binary printed a disclosure
   about a resource it had not written.

4. **`rotate_widget`** — `/MK /R` (§12.5.6.19 Table 189), a **quantised
   0/90/180/270 declaration**, not a free-angle transform. Named by one of
   pdfce's own refusals and not built. Well-specified and small.

5. **The n-channel buffer** — see §0. Operator's call.

6. **A lossless markup clipboard copy** — Backlog, unscoped, **awaiting
   `pdfceGUI`'s answer on whether they need it.** Needs `MarkupSpec` extended
   **and** `/IRT` reference rewriting: a graph problem, not a value problem.
   **Do not build it speculatively.**

7. **A rustdoc-cleanliness gate** — Backlog, **LOW**. The dangerous subclass
   (a doc naming a public verb that does not exist) was a population of
   **one** and is closed; the other ~150 are path-scope noise.

8. **★ A pre-push check for a deferred tip.** `R226` closes the hole
   *procedurally*. **Not built, and NOT to be built unasked** — it changes the
   operator's own push workflow. Raise it.

9. **★ `.gitignore:20` (`/fixtures/external/`) is LOAD-BEARING and nothing
   says so.** The staged veraPDF corpus declares CC BY 4.0 over content
   including the **Isartor** suite, whose own manual forbids redistribution.
   **The repository is public**, so committing that tree would *be*
   redistribution. One `.gitignore` line is the entire control. Operator
   question `(bx)`.

10. **`(bv)`–`(bz)`, five licence questions** and **`(bl)`, the OCR model
    weights**, each with a conservative default already chosen. None blocks
    anything.

---

## §B — ★★★ THE LESSON OF THIS SESSION, AND IT IS NOT ABOUT COLOUR

**Two of my own claims were audited and found wrong, by two different agents,
in the same session I made them. Neither was caught by any of the 29 gates,
and both lived in documents another project builds against.**

1. `--probe-ink`'s **worked example** shipped `srgb=24,140,108` — the
   **pre-`Pass 165.0` defect value** — in the CLI's `--help` and in
   `docs/core-api/03-capabilities.md`. So the example restated, in
   operator-facing documentation, exactly the premise the same session had
   just measured away, while a test twenty files over asserted the correct
   value for the same operand. **The librarian found it.**
2. `OverprintZeroTintScope`'s doc called itself *"a genuine spec ambiguity"*
   with *"no sentence resolving it either way"*. **Three sentences resolve
   it** under ISO 32000-1 (§8.6.7's next sentence excludes *"conversions from
   some other colour space"*; Tables 148/149 row 2 tabulate the case and give
   it `OPM 0` behaviour). **`pdfce-spec-librarian` found it**, auditing the
   dispatch it was given. ISO 32000-**2** deletes two of the three, so the
   question is **edition-gated**.

⇒ **A WORKED EXAMPLE IS A CLAIM.** A stale one is a wrong claim that reads as
an illustration, and nothing in this project checks one. ⇒ **AND AN AUDIT
DISPATCH IS WORTH MORE THAN THE FILING IT ASKS FOR.** Both agents were sent
to *write something*; both returned corrections to the thing that sent them,
because the dispatch stated its premises explicitly enough to be checked.
**State your premises in a dispatch. That is what makes it auditable.**

### Two more from this session

**A measurement whose outcome is entailed by the spec is not a measurement of
the implementation.** `Pass 174.2` ablated `overprint_zero_tint_scope` over a
grey-over-**spot** patch and got bit-identical ink from all three settings —
but Tables 148/149 force that result on a spot backdrop regardless. It would
have been identical on a correct implementation and a broken one. The
discriminating case is grey over **process** components (`OP-N3`).

**`R233`, minted this session, fired twice on the same day.** An aggregate's
discriminating power is the fraction of the population where the correct and
the **null** answers differ, not the population's size. Instance 1: *125 of
132 achromatic regions stay achromatic (95 %)* — but 125 of them are **paper
white**, where a conversion that did nothing also scores perfectly, and the
real population is **7 mid-greys of which 0 stay neutral**. Instance 2: an
operand census where **one file supplied 59 %** of the population.

---

## §C — HOW TO RUN THE GATES

★ **29 commands.** ★ **"PASS — 29" over-claims and the label says so:** two
deliberate skips — `cargo about` (only when the dependency set changes) and
**`cargo test --workspace --all-features`, replaced by plain
`cargo test --workspace`**. ⇒ **every green this project reports is a
default-features green.**

★★★ **THE RECIPE. Deviating from it cost an hour on 2026-08-29 and cost
another 20 minutes on 2026-08-30:**

1. **Warm the cache first**: `cargo test --workspace --no-run`.
2. **Run in the FOREGROUND**: `timeout 580 bash tools/run-gates.sh`.
   ★ **Chaining the warm-up and the sweep in one command pushes the WHOLE
   THING past the foreground limit and into the background**, which is exactly
   what the recipe forbids. Two commands, not one.
3. **Do nothing else while it runs.** No `du`, no `find`, no `git status` on a
   worktree, no second cargo, **no subagent**. Concurrent heavy I/O starves
   the doctest binary and it fails with `0xc0000142` / *"Couldn't compile the
   test"* across dozens of unrelated files. **That looks exactly like a broken
   tree and is not.** Confirm with `cargo test -p pdfce-core --doc` alone
   before diagnosing anything.

★★ `R226`: also run `python tools/check-passes-filed.py --strict-tip` before
ending any session whose tip commit claims a Pass ID. The plain run
**deliberately defers the tip** and reports `clean`.

★★★ **A gate sweep certifies the tree it ran on.** Re-run after the last edit.
**Do not hand-type a subset** to dodge the time limit — that is how the two
filing gates got omitted and CI went red.

---

## §D — MEASURED AT WRITE TIME. Re-run; do not copy forward

- **Conformance suite, RE-MEASURED 2026-08-29 (not carried forward):**
  **6 FAIL / 29 clean / 16 unresolved of 51, 0 render errors — unchanged.**
  The 16 unresolved split 8 `REF` / 5 `REF-PASS` / 3 `MARK?`. The 6 FAILs:
  `PCS2_020` (6 traps), `PCS2_030` (3), `PCS2_040` (3), `PCS2_081` (1),
  `PCS3_130` (4), `PCS3_161` (12).
  Command: `python tools/suite-check.py "$PDFCE_SUITE_DIR"
  --reference-dir "$PDFCE_SUITE_REFS" --json`.
- **`docs/core-api/`'s sizes and counts: DELIBERATELY NOT STATED HERE.** Run
  `python tools/check-core-api-verbs.py` — it derives and prints all four, and
  it is the only thing that cannot be stale. (This bullet held numbers twice
  and they rotted both times, the second time within hours.)
- **Latest backup bundle and portable build are both STALE** — both predate
  this session. Re-take.
- **`main` is ahead of `origin`.** Push is standing-authorized (decision 090,
  *"always push"*) — but **cutting a tag or a release is not**, and neither is
  a force push or a non-`main` branch.

### The two colour instruments, both new and both out-of-tree

- `tools/flat-color-parity.py` — pdfce vs the reference engine on **flat
  regions only**, each region **eroded before sampling** so edge antialiasing
  cannot enter the number. `--neutrals` restricts to achromatic reference
  regions. **This is the only instrument pdfce has that measures colour
  accuracy**; `render-parity` conflates colour with structure and
  `suite-check` reads conformance traps.
- `tools/cmyk-operand-census.py` — which `DeviceCMYK` operands the table is
  actually asked for. **Lower bound by construction** (no `sc`/`scn` behind a
  resolved space, no images, no shadings) and it says so. `--set` emits the
  operand list alone, with no file names — use that whenever the corpus is
  licensed.

Both need `PDFCE_SUITE_DIR` / `PDFCE_SUITE_REFS`; both skip loudly without.

---

## §E — THE CHANNELS. Check BOTH every session.

`D:\Dev\FeatureRequests\pdfce_FeatureRequests\` and
`D:\Dev\FeatureRequests\iccce_FeatureRequests\`. **They live outside the repo,
so no gate can contradict a stale "it's empty" claim.**

★ **`iccce` reorganised its channel mid-session on 2026-08-29**: `open/` +
`archive/YYYY-MM-DD-<topic>-N-<party>-<slug>.md` + an `INDEX.md` that is *the
memory*. **A session lists `open/` and nothing else**; `archive/` is read only
when an `INDEX.md` row points at it. A file you wrote may have been archived
out from under you — mine was, twenty minutes after I wrote it.

**Colour thread status: CLOSED by them, nothing owed.** They accept the
probe's answer, **withdrew their own "the fallback beats the buffer by 3×"**
as an artefact of an older binary, and returned **(65, 171, 61)** for
`.75 0 1 0` against our (47, 181, 73) and the capture's (59, 171, 51) — closer
on all three channels, lcms2-corroborated to four decimal places.

★★ **Carry their standing limit into pdfce's own thinking, because it is
right and it cuts against us:** *the reference capture is a REFERENCE, NOT A
TARGET.* "Closer to Acrobat" must never harden into "tune toward Acrobat" —
that makes a screen capture the ground truth.

**Outbound, and ANSWERED the same evening — nothing owed either way.**
pdfce sent `note_the_operand_list_you_asked_for_49_tuples_covering_95_percent.md`
(49 operands covering 95 % of paint events, pdfce's sRGB beside each);
`iccce` answered in
`open/reply_the_49_rows_and_the_black_end_is_where_i_am_weaker.md` with their
column for all 49, through the patch's own `/DestOutputProfile` to the
OS-shipped sRGB, media-relative, lcms2-corroborated to **0.22 counts** across
the whole set.

★★★ **Three things in that reply are worth more than the numbers:**

1. **The two tables answer DIFFERENT QUESTIONS.** pdfce's table exists for
   documents with **no output intent**; theirs converts through a **declared**
   output condition. *"Where a document declares an output intent, you should
   not be consulting a table at all."* That is a real gap in pdfce and nobody
   has scoped it.
2. **The disagreement is regional, and each region has a different cause.**
   On the **black end** iccce is lighter and **pdfce is closer to the
   reference** — because their black-point estimator **refuses by name** on
   that profile, and *"refusing and being wrong look identical in a table."*
   On the **achromatic axis** it goes the other way: their greys are neutral,
   pdfce's are cool by 2–5 counts on every row. **Their row 3 is 158,159,159
   against pdfce's 147,148,152 and the reference's 156,156,156.**
   ⇒ *"Which engine is better"* is the wrong question.
3. ⇒ **That is a THIRD independent line against `CmykIntent::Calibrated`'s
   stated rationale** on the grey axis — the reference's exact neutrality, the
   flat-colour sweep, and now the profile's own answer. `Pass 174.1` corrected
   the doc comment and deliberately left the table alone; **if that ever gets
   revisited, this is the evidence, and the achromatic axis is where to
   start.**

★ **Do NOT fit the table to those 49 numbers** (decision 064, and their own
sign-flipped rule). Keep them as a regression datum.
