# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**: no
*"this paragraph read X until…"*. What is true now, plus a pointer.
Corrections and their prior wording live in the **append-only** record —
`ROADMAP.md` and `SESSION_LOG.md` — where a claim is dated and no later edit
can falsify it.

Written 2026-08-28 at `4094e49` + the 306th filing.

---

## §0 — ONE THING NEEDS THE OPERATOR, AND IT BLOCKS NOTHING

**The private suite's name is in 82 already-pushed commit messages** — 302
occurrences across 1,042 published commits, split across the gate's two
needles. Counted by decoding `check-suite-name-absent.py`'s own base64 forms
at run time and scanning `git log origin/main --format=%B`; **re-run it rather
than trusting this figure**, and do not write the decoded terms down. This
paragraph originally quoted both, which turned the gate red on the very
document explaining why the gate matters.

Why it is not already handled: **the gate scans the WORK TREE.** Its docstring
never claimed commit messages and it could not reach them. The 2026-08-10
operator decision that accepted material already in history predates the
2026-08-25 scrub ruling by two weeks, so it cannot be read as covering this.

The two options, and only the operator picks:

- **Accept**, as in 2026-08-10. Costs nothing, changes nothing.
- **Rewrite and force-push.** Gated by rule 8, and this repository has direct
  evidence against it: `0d9f4df` found **fourteen** documents left citing
  commits that a rewrite had destroyed.

**It blocks no work.** Fresh commits are clean — one occurrence was caught in
`885cf3a` and amended out before it was pushed (see §E).

---

## §A — RUN `bash tools/run-gates.sh`, NOT A HAND-TYPED GATE LOOP

Unchanged and re-earned this session. The sweep is **26 commands**; the ones
you would think to type are not the ones that catch you.

On 2026-08-28 the first sweep after `Pass 143.0` went red on **five**, and
**four were real omissions I did not know I had made**:

- **`check-settings-consumed.py`** — I had not added the new setting to
  `write_to_string`, so **saving settings would have silently dropped it.** I
  did not know this gate existed.
- **`check-string-gaps.sh`** — two Rust line-continuations eaten by a heredoc,
  in my own new test file. Exactly what it was written for.
- the **GUI settings-panel reachability test** — a setting with no control;
- the **`RenderPolicy` projection test** — whose *builder call* I had also
  omitted, so it would have passed while the builder did nothing.

---

## §B — WHAT TO PICK UP

### 1. The trap X on the grey/K-black overprint patch — cause unknown, and a strong lead

`Pass 143.0` shipped and that patch still FAILs with 3 traps. **It is not the
same defect** — the filed one is fixed.

Measured on `4094e49` with `tools/suite-cell-probe.py`:

| | X | surround |
|---|---|---|
| under `device_cmyk_only` and under the default alike | `127,127,127` | `81,119,40` |

The surround is already within a few levels of Acrobat's `84,120,34` **under
both settings** — so the symptom the Backlog entry recorded at `70c5919` had
already been fixed by some later Pass before `143.0` began. What fails now is
the **X**.

★ **The lead, and it is a good one.** Dumping the patch's content streams shows
it paints the same 50 % grey **both ways** — `0.5 g` *and* `0 0 0 0.5 k` — on
purpose, so an engine that treats them differently is caught. That is exactly
the comparison `grey_matches_the_cmyk_k_only_reference_exactly` makes, and it
now passes on the synthetic fixture while the suite cell still differs. **Find
what the fixture does not reproduce.** `G .5` (a DeviceGray *stroke*) also
appears in those streams and the synthetic fixtures use fills only — start
there.

Dump tool used: a throwaway zlib walk. Worth rebuilding if you need it; there
is no content-stream verb on `pdfce-cli`.

### 2. `AllProcessSpaces` is shipped and UNMEASURED

The widest value of `overprint_zero_tint_scope` has no oracle behind it.
pdfce's RGB→CMYK is naive, so a pure red preserves a cyan backdrop under it,
and **whether Acrobat agrees is not known** — no corpus patch exercises it. It
is not the default and its doc comments say all of this. Do not promote it
without a measurement.

### 3. A new inbound the librarian filed to *Next up* without an ID

`request_a_pinned_edit_still_matches_on_find_and_the_find_is_extractor_prose.md`
(08:31). A *pinned* `edit_text` is refused by string-comparing `find` against
`text_extract`'s synthesised inter-glyph spacing, while `format_text` got
`whole_operator(page, span)` in `Pass 145.0`. **Minting the Pass ID is the
engineer's, not the librarian's.**

Fourth consecutive item in the `R219` family.

### 4. Owed to the spec librarian

A **spec-ambiguity-register entry** for `overprint_zero_tint_scope` — *does
`OPM 1` reach `DeviceGray`?* The neighbouring `REND-A1` row exists; this one
does not. Second register entry owed;
`render.hairline_clamp_policy` has been owed since 2026-08-09.

---

## §C — WHAT SHIPPED

`Pass 151.0` (`4094e49`'s predecessor `c4425f0`) — `resize_annotation` +
`pdfce-cli resize-annotation`, with the Inkscape-style `scale_stroke_width`
toggle Ken ruled must exist. `Pass 143.0` (`4094e49`) — the §8.6.7
`overprint_zero_tint_scope` ambiguity setting. Plus `3dc9eb0`, extending
`check-core-api-verbs.py` to the four figures that had no gate.

Filings 305 and 306. Verb count **151**; `EditError` **90**.

---

## §D — VERIFIED FROM A SHELL AT WRITE TIME — re-run, do not copy forward

- suite: **6 FAIL / 29 pass / 16 unresolved of 51**, unchanged across
  `Pass 143.0` — no regression, and no improvement the corpus could score.
- the setting moves **3 of 51** patches (8,491 / 1,827 / 804 px), **0 of 3**
  change verdict. Method: render each patch twice with the shipped binary,
  `--overprint-zero-tint-scope device_cmyk_only` versus the default, and count
  differing pixels.
- backup bundle is **45 commits behind** (`pdfce-20260827-shutdown-a2b4e16`).
- `origin/main` and local `main`: check with `git status -sb`, do not assume.

---

## §E — THE LESSON THIS SESSION COST THE MOST TO LEARN

**A measurement taken while the thing it measures is being changed does not
survive the change — and it is most likely to be re-quoted in the very entry
documenting the fix.**

I instrumented the renderer and found **0 of 51** corpus patches painting a
`DeviceGray` source through Table 149. True at that moment. I then fixed
`overprint_would_change`, **which is precisely what lets grey sources reach
Table 149**, and never re-ran the scan. The real figure is **3 of 51**. The
wrong one reached a commit message, a Pass entry, two RAG trees, a fixture
generator, a test-file header and a report to the operator.

⇒ **Prefer the differential form for any number that will outlive the work.**
"Render twice and diff" re-measures itself on whatever tree it runs against;
an instrumented count is frozen at the moment it was taken.

Two more from the same session, both cheap to repeat:

- **Reaching the code is not changing the result.** The `classify` fix
  compiled, was demonstrably executed on 10 paints, and moved **zero pixels**.
  Only an A/B of rendered output separates "wrong fix" from "right fix waiting
  on something else". (`R219` clause (f).)
- **Two agreeing copies of one rule mutually mask sabotage.** My first cut
  duplicated the scope predicate; a mutation that widened one left the suite
  green because the other still refused. Neither copy was covered by any test,
  and the green run reported the duplication as safe. Ask the accepting code
  (`R221`).

**And amend before you push, never after.** `885cf3a` named the suite in its
message; amending an *unpushed* commit is not a history rewrite and rule 8
permits it — but it invalidated **23 hash citations across three documents**,
plus **10 more in the RAG trees that no gate can see**.
`check-cited-commits-exist.py` caught all 23 and named the replacement by
matching subjects. A gate's silence is only as wide as its input set.

---

## §F — OPEN, UNCHANGED

- The suite-triple discrepancy the librarian could not reconcile: last
  adjudicated `--reference-dir` run on record is **5 / 35 / 11**; this
  session's un-adjudicated run is **6 / 29 / 16**. The FAIL delta is
  explained; the six that moved pass → unresolved are not.
- Table 149's spot-component row family remains implemented, tested and
  **unreachable** — no caller, no plane. Belongs to `Pass 97.x`, not here.
- `pdfceGUI` has been told about the `#[non_exhaustive]` builder requirement
  and has not confirmed reading it.
