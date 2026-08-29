# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**. What is
true now, plus a pointer. Corrections and their prior wording live in the
**append-only** record — `ROADMAP.md` and `SESSION_LOG.md`.

Written 2026-08-29. Ledger at write time: **Pass ceiling 164.0**, rules
**R228**, decisions **097**, filings **323**.

---

## §0 — ★★★ THE OPERATOR'S STANDING INSTRUCTION IS COMPLETE

> *"do all 4"* — the four things not completely editable, which he asked me to
> enumerate and then to fix.

**All four are done.** This is the first session since the instruction was
given where there is no next item under it.

| # | gap | shipped as |
|---|---|---|
| 1 | **Rotation** of anything carrying a `/Rect` | `Pass 155.0` — core + CLI, annotation family. |
| 2 | **ce dimensions** | `Pass 159.0` (rotation). Two of the three gaps reported **never existed**; scaling **declined by name**, operator confirmed. |
| 3 | **Bookmarks** — rename, delete, reorder, re-parent | `156.0`/`157.0` rename+delete; **`161.0` reorder + re-parent + expand/collapse**, hardened by `161.1`. |
| 4 | **Fonts** — restyle to a face the document lacks | **`Pass 162.0`** — the **standard-14 half**. See §0a for the half that remains. |

⇒ **The next thing IS handed down: the ce-dimension text override, decision
097.** He ruled on its shape on 2026-08-29 — see §A item 1. He did **not** rule on
the schedule, so it sits in Backlog rather than *Next up*, but it is the only
item on the list he has personally specified.

**Do not promote the conformance validator (§0b) without him saying so** — his
words, 2026-08-29: *"we'll deal with the conformance validator later."*

## §0a — WHAT GAP 4 DID AND DID NOT CLOSE

**Closed:** `format-text --set-font` now **authors a standard-14 font
resource on demand** when the target's `/Resources` does not carry the face.
Twelve Latin faces get `/Encoding /WinAnsiEncoding`; `Symbol` and
`ZapfDingbats` correctly get **none** (built-in, Annex D.5/D.6). The full form
with `/Widths` is emitted, no `/FontDescriptor`, no font program.

**Still open — `Pass 142.0`, narrowed:** a face **outside** the standard 14.
That needs a real font program: subset from `--font-dir`, build a
`/FontDescriptor` with `/FontFile2`/`/FontFile3`, widths, encoding, and a
**six-uppercase-letter subset tag unique across the WHOLE FILE** (§9.6.4
ST1–ST4 — ST4's uniqueness is per *file*, so the existing document's font names
must be scanned, not just session state). **De-prioritised, not declined** —
the consuming shell's *"synthetic is enough"* was scoped as a report of
**their** use, not a decision about ours.

### ★★ Three facts from `162.0` that will save the next session real time

1. **`bind_font_resource`** (`text_edit/addtext.rs`) is now the **single**
   implementation of "add a `/Font` entry to a page or form". It walks
   inheritance, patches shared dictionaries in place, and is taken over an
   `ObjectGraph` so all three save paths reach it. **`142.0` extends this, it
   does not write a second one.**
2. **There are THREE save paths, not one**, and this is the trap:
   `EditSession::format_text` (page), its form twin, and the one-shot
   `text_edit::set_format` (`&Document` + `DirtySet`) — **which is the one the
   CLI uses**. `162.0` wired two, every unit test passed, and the binary
   printed a disclosure about a resource it had not written. **Any future
   change to what a format edit writes must touch all three.**
3. **§7.8.3: a page's own `/Resources` REPLACES the inherited one**, it does
   not merge. Giving an inheriting page a direct `/Resources` holding only a
   new font orphans every font its existing content names. The file still
   parses. `inherited_resources_are_patched_not_shadowed` in
   `crates/pdfce-core/tests/set_font_new_resource.rs` is the regression test.

---

## §0b — THE CONFORMANCE VALIDATOR BUCKET (unchanged, still unscoped)

At the **bottom of `ROADMAP.md`'s Backlog**, **no Pass ID**, exactly as the
operator asked. **Do not promote it without him saying so.**

Four arcs A–D, split by **whether the work can be GRADED**. Arc A (PDF/A all 11
levels, PDF/UA-1, PDF/UA-2, WTPDF) has oracle **and** corpus on this machine:
**veraPDF 1.30.2** at `D:\tools\verapdf\verapdf.bat`, **2,907 files** at
`fixtures/external/veraPDF-corpus/`.

Three facts that change the sizing:

1. **UA-2's real cost is not its own 91 rules but the 1,636 ISO/TS 32005
   structure rules it leans on.** A session reading "91" under-scopes by an
   order of magnitude.
2. **Expected verdicts are mechanical from the FILENAME**, not the outline
   bookmark. Over 2,906 files: 2,874 agree, **4 disagree**, 28 lack the
   bookmark. Read the filename.
3. ★ **Acrobat Pro has no tool that fully validates PDF/UA** — a place to
   **exceed** the reference rather than match it.

**Settled before any code: the validator reports and never mutates.** Any
repair is a separate, named verb. **PDF/E-2 was never published** — the
CAD/engineering target is **PDF/A-4e** (arc A, 19-file corpus).

Reference: `PDF_Spec/conformance/conformance__ref__validator_scope.md` (989
lines, 76 `CV-*` ids) plus seven files under `Acrobat_Features/`.

---

## §A — OWED, consolidated. This is the pick-from list now.

1. **★★ THE ce-DIMENSION TEXT OVERRIDE — decision 097, the operator's own
   ruling (2026-08-29).** The only item here he has personally specified.
   Verbatim: *"dimension text override should be an option if it can be
   selected to be overridden or not so the override can be undone. otherwise we
   need to add a manual dimension tool where the user just enters the value of
   the dimension."*
   **Read it as two branches with a condition, not one instruction:**
   - **Branch 1, preferred — the override is a SELECTABLE STATE, not a
     destructive edit.** The ce dimension **keeps its measured value** while an
     override is in force; overriding is a flag plus a string; **clearing it
     restores the measured text** with no re-measure, **and that must survive
     save-and-reopen** — so the measured value persists in the sidecar
     *alongside* the override. This is what makes it compatible with project
     rule 15 (a ce dimension's text **is** its measurement) and rule 4 (an
     operator-supplied divergence is disclosed and reversible, never silent).
   - **Branch 2, fallback only — a manual dimension tool**, where the operator
     types a value that was never measured. A **different object class**, not
     an override: nothing to restore, nothing to diverge from. Do **not** build
     it as a substitute for branch 1.
   - **Engineering read: branch 1 is achievable.** `/PieceInfo` already carries
     pdfce's own per-dimension state, so `override: Option<String>` beside the
     measured value is additive and round-trips.
   - ★ **Naming is constrained**: `Pass 163.0` made a refusal cite
     `set_dimension_label` and say it is NOT BUILT YET. Whatever ships takes
     that name, or the message changes —
     `tools/check-cited-verbs-exist.py` enforces the pair.
   - **Prerequisite before code: the disclosure story.** What does a shell show
     when a dimension's text is *not* its measurement?

2. **`Pass 142.0`** — non-standard-14 faces (see §0a). The largest remaining
   item, and the natural continuation of `162.0`.

3. **Two verbs pdfce's own refusals NAME and does not have** — `Pass 163.0`
   made the messages honest; **it did not build them.**
   - **`rotate_widget`** — `/MK /R` (§12.5.6.19 Table 189), a **quantised
     0/90/180/270 declaration**, not a free-angle transform. Well-specified and
     the smaller of the two.
   - **`set_dimension_label`** — now superseded in scope by item 1; build it
     under decision 097's shape, not as a bare setter.

### ★★ The colour cluster — six items, and item 4 is the lead

These came out of the iccce exchange of 2026-08-28/29. **Read them together**;
three of them were previously mis-stated in this file.

4. **★★★ The `PCS3_132` residual — MEASURED 2026-08-29, and it is NOT the
   conversion table.** iccce asked for the table-isolated number and it splits
   the error decisively. `PCS3_132` paints its green as **`.75 0 1 0`**
   (recovered from the content stream, no render in the loop):

   | channel | Acrobat | table only | iccce's capture of our render |
   |---|---:|---:|---:|
   | red | 59 | 47 | 24 |
   | green | 171 | **181** | 140 |
   | blue | 51 | 73 | 108 |

   Split as Acrobat → table → everything else: the table contributes
   **−12 / +10 / +22**, the rest of the path **−23 / −41 / +35**.
   ★ **The rest is the larger term on every channel and on GREEN IT IS THE
   OPPOSITE SIGN** — the table takes 171 *up* to 181, something after it takes
   181 down to 140. So the render error **cannot be attributed to the table
   even partially by addition**; any account saying "the blue floor did it" is
   arithmetically excluded.
   ⇒ **MEASURED FURTHER, 2026-08-29. Three candidates eliminated:**

   | candidate | test | result |
   |---|---|---|
   | the conversion table | applied directly to `.75 0 1 0` | a third of it, **opposite sign on green** |
   | `OverprintZeroTintScope` (the **setting**) | rendered at all three scopes | **byte-identical**, pixel counts equal to the pixel |
   | `ICCBased` `/N 4` → `DeviceCMYK` fallback | synthetic `/ICCBased`, `/N 4`, no `/Alternate` | **`(47, 180, 73)`** — correct |

   Plain `DeviceCMYK` `.75 0 1 0 k` also renders `(47, 180, 73)`. So the paint
   path, the table and the ICCBased resolution each behave **on their own**.

   ★★ **AND A DISTINCTION THAT NEARLY GOT WRITTEN DOWN WRONG.** Having found
   the zero-tint scope inert, the tempting sentence was *"overprint is
   eliminated."* **That is false.** What was measured is that **one overprint
   SETTING is inert on this patch** — not that the overprint **leg** is
   uninvolved. `PCS3_132` carries `<</OP true /op true>>` and `<</OPM 0>>`
   ExtGStates, and the zero-tint axis governs a *different* rule.
   **Eliminating a knob is not eliminating the mechanism the knob sits on.**

   ★★★ **FIVE HYPOTHESES REFUTED, 2026-08-29.** Each by rendering a
   synthetic that isolates one ingredient. **Every one renders `(47, 180, 73)`
   — correct.** The residual survives all of them:

   | # | hypothesis | how tested | result |
   |---|---|---|---|
   | 1 | the conversion table | applied directly to `.75 0 1 0` | a third of it, **opposite sign on green** |
   | 2 | `OverprintZeroTintScope` | the patch at all three scopes | **byte-identical** output |
   | 3 | `/Separation /All` under `/OP true` | synthetic, tints 0 and 1 | knocks the green out to **white / black** — nowhere near |
   | 4 | the overprint STATE | synthetic × 5: `OPM 0`, `OPM 1`, `OP`+`op`, ICCBased and DeviceCMYK | all **correct** |
   | 5 | the real 1.4 MB source ICC profile | obj 19 lifted **verbatim** into the synthetic | **correct** — so pdfce really does use the `/Alternate` fallback its doc comment claims |

   ★★ **AND A DISTINCTION THAT NEARLY GOT WRITTEN DOWN WRONG.** After #2 the
   tempting sentence was *"overprint is eliminated."* **False.** #2 shows one
   overprint **SETTING** is inert on this patch; #4 is what actually clears the
   overprint **state**. **Eliminating a knob is not eliminating the mechanism
   the knob sits on**, and the two needed separate experiments.

   ★★★ **FOUND, 2026-08-29. It is the CMYK COMPOSITING BUFFER.**

   The chain: `PCS3_132` is **PDF/X-4** and declares `/OutputIntents` with a
   subtractive destination → pdfce's
   `PageBlendSpaceSource::OutputIntentIfSubtractive` composites the page in a
   **four-colorant CMYK buffer** instead of on screen → **that buffer is the
   entire residual.**

   Confirmed with the shipped `--max-cmyk-buffer-bytes` flag, which forces the
   documented on-screen fallback, on **both** the real patch and a 6-object
   synthetic:

   | render | `cmyk_buffer_refused` | green |
   |---|---|---:|
   | the patch, default | `0` | **(24, 140, 108)** |
   | the patch, `--max-cmyk-buffer-bytes 1024` | `1` | **(47, 180, 73)** |
   | synthetic, default | `0` | **(24, 140, 108)** |
   | synthetic, tiny buffer | `1` | **(47, 180, 73)** |

   ★★ **AND THE DIRECTION IS THE UNCOMFORTABLE PART.** Against Acrobat's
   `(59, 171, 51)`: on-screen is **−12 / +10 / +22**, the CMYK buffer is
   **−35 / −31 / +57**. **The buffer exists to be MORE faithful for print, and
   here it is roughly three times further from Acrobat on every channel than
   the fallback it replaces.** One patch, and a print-correct blend is not the
   same goal as a match-Acrobat blend — but the buffer's conversion back to
   sRGB had never been measured against anything, and the first time it was, it
   lost.

   ⚠ **The same page renders two different greens depending on whether a
   memory ceiling was hit.** `--max-cmyk-buffer-bytes`'s own help says the
   colours can differ at different scales; *"slightly"* understates 23/41/35
   counts.

   ★★★ **DIAGNOSED TO THE LINE, 2026-08-29. Two defects, and the second is
   the worse one by this project's own rules.**

   **The buffer is innocent.** `CmykBuffer::to_srgb_over_white` converts out
   through the *same* `cmyk_to_srgb_with` as the direct call. The value is
   wrong on the way **IN**. Same four numbers, same buffer, same intent:

   | entry path | rendered |
   |---|---:|
   | `DeviceCMYK` — `.75 0 1 0 k` | **(47, 181, 73)** ✅ |
   | `ICCBased` — `/CS1 cs .75 0 1 0 scn` | **(24, 140, 108)** ❌ |

   **DEFECT A — an authored CMYK value is thrown away and re-derived from its
   own sRGB approximation.** `overprint::authored_tints` keeps the authored
   tints for `SourceKind::DeviceCmykDirect` and for
   `SourceKind::SeparationOrDeviceN`, and returns **`None`** for
   `OtherProcess` — which is where an **ICCBased CMYK** lands. `interpret.rs`
   then falls through to `overprint::rgb_to_cmyk(paint colour)`, a **max-GCR**
   transform its own doc calls *"chosen for exact round-tripping, not for
   accuracy."*

   **Proven numerically, exact to the integer on all three channels:**

   ```text
   authored          cmyk .75 0 1 0        -> (47, 181, 73)
   reconstructed     cmyk .7382 0 .5942 .2906
   round-tripped                            -> (24, 140, 108)
   observed ICCBased                        -> (24, 140, 108)
   ```

   Yellow collapses **1.0 → 0.59** and black is **invented at 0.29**.

   ★ **The fix is one classification.** An `ICCBased` space with `/N 4` — which
   pdfce *already* resolves to `DeviceCMYK` through the `/Alternate` fallback —
   should classify as `DeviceCmykDirect`, not `OtherProcess`. Small change;
   **but it moves rendered colour on every PDF/X file with ICCBased CMYK**, so
   it needs the conformance suite re-measured in the same Pass. That is why it
   is not a tick-sized fix.

   ⚠⚠ **DEFECT B — `cmyk_bridged_pixels` REPORTS 0 WHILE THIS HAPPENS.**
   `cmyk_paint.rs`'s doc says *"Every such paint is counted by the buffer's own
   bridge counter, so a page composited largely from reconstructions says so."*
   Measured on the failing render: **`cmyk_bridged_pixels=0`** across 40 000
   reconstructed pixels — identical to the correct DeviceCMYK render.

   The reconstruction at `interpret.rs:5392` is **not** the one the counter
   watches. So pdfce approximated a colour and **reported that it had not**,
   which is project rule 4 broken rather than an accuracy gap. **Fix B even if
   A is deferred** — an honest counter would have led here in one render
   instead of six hypotheses.

   ★ Note `BrushSpec::with_cmyk` is called **only from tests**; production
   never populates that field. Worth understanding before touching either.

   ★ **The five refutations that came first**, each a synthetic isolating one
   ingredient, every one rendering the correct `(47, 180, 73)`:

   | # | hypothesis | result |
   |---|---|---|
   | 1 | the conversion table | a third of it, **opposite sign on green** |
   | 2 | `OverprintZeroTintScope` | all three scopes **byte-identical** |
   | 3 | `/Separation /All` under `/OP true` | knocks the green out to white/black |
   | 4 | the overprint **state** | five variants, all correct |
   | 5 | the real 1.4 MB **source** ICC profile, lifted verbatim | correct |

   ★ **#5 positively**: pdfce really does resolve `ICCBased` through the
   `/Alternate` fallback its doc comment claims — the profile bytes do not
   reach the result. A documented claim that survived being tested.

   ★★ **A knob is not a mechanism.** After #2 the tempting sentence was
   *"overprint is eliminated."* False — #2 clears a **setting**, #4 clears the
   **state**, and they needed separate experiments.

   ⚠⚠ **AND A TEST THAT PRINTED THE RIGHT-LOOKING ANSWER FOR THE WRONG
   REASON.** The first output-intent attempt matched `13 0 obj` inside a search
   for `3 0 obj`, attached a **1 253-byte `/Info` dictionary** as a 1.46 MB ICC
   profile, and returned `(47, 180, 73)` — **indistinguishable from the five
   genuine refutations.** Caught on the **byte count** against the file's own
   declared `/Length 1462566`, not on the result. Every lift now verifies
   against the declared length and aborts on mismatch.
   **A wrong test that agrees with your priors is the expensive kind**, and it
   was one line from being written up as a sixth refutation.

   ★ **iccce's capture was EXACT.** Rendering the patch here and reading the
   PNG pixels directly — no display path — gives `(24, 140, 108)`, 12 769 px.
   The same three integers as their screenshot. Their §11.2 display-path limit
   is still the right default, but on this quantity it cost nothing.

   ★ Also measured: **all three CMYK intents return the identical
   `(47, 181, 73)`** here, so the intent setting is **inert on this patch** and
   the `Naive` deletion costs nothing *for this measurement*. iccce's broader
   point about losing the three-way diagnostic instrument stands, narrowed.

5. **★ A shipped default's stated rationale has evidence against it.**
   `CmykIntent::Calibrated`'s doc comment justifies its cool mid-greys as
   *"what Acrobat shows, which is the point."* On `PCS3_230`'s 25 % gray iccce
   measured Acrobat at **exactly (98, 98, 98)** — perfectly neutral — against
   pdfce's **(99, 100, 103)**. One patch at one level is not the sample that
   claim is about, and it is a cross-renderer comparison carrying a
   display-path limit. But it is a **load-bearing justification for a shipped
   default with evidence pointing the other way**, and this project has a rule
   about that shape. Either widen the sample or soften the comment; do not
   leave it as it stands.

6. **`PCS3_130` cells c and d disagree with each other by ~8 counts** — same
   CMYK source, one **vector**, one **image**. iccce's cheap explanation is
   image-path resampling; untested. **Ours to look at.**

7. **The trap X on the grey/K-black conformance patch.** Cause unknown, **not**
   the defect `Pass 143.0` fixed. Lead: that patch paints the same 50 % grey
   **both ways** (`0.5 g` and `0 0 0 0.5 k`) deliberately, and `G .5` — a grey
   *stroke* — appears in its streams while every synthetic fixture uses fills.
   ★ **NOT the same as iccce's "invisible X"** — theirs is `PCS3_130`, a
   CMM/source-profile patch, filed as theirs under decision 064 and now
   answered. Two X's, two patches, two causes, one word.

8. **★ iccce's ΔE00 figures are WITHDRAWN by their own author** (`DL-070`: no
   ΔE of any formula, in any space, against a screen capture). That includes
   the *"49–58 ΔE00"* that was in one note's original **filename** and the
   *"296×/314× factor"* column. **Only 8-bit channel counts may be quoted**,
   which is why every number in items 4–6 above is in counts.

9. **`PCS3_132`/`PCS3_133` PASS their own criterion and must not be written up
   as suite failures.** Their pass condition is the *absence* of an X, and
   there is no X. The gap is an **accuracy gap on a conformance-passing
   panel** — a different claim, and iccce asked specifically that the
   distinction be kept.

### The rest

10. Ambiguity-register entries owed to `pdfce-spec-librarian`:
    `overprint_zero_tint_scope`, and `render.hairline_clamp_policy` since
    2026-08-09.
11. **A lossless markup clipboard copy** — Backlog, unscoped, **awaiting
    `pdfceGUI`'s answer on whether they need it.** See §E.
12. **A rustdoc-cleanliness gate** — Backlog. ★ **LOW priority:** measured
    2026-08-29, the dangerous subclass (a doc naming a public verb that does
    not exist) was a population of **one**, and `Pass 161.0` closed it by
    shipping the verb. The other 150 are path-scope noise.
13. **★ A pre-push check for a deferred tip.** `R226` closes the hole
    *procedurally*, and a procedural rule is what was skipped to create it.
    **Not built, and NOT to be built unasked** — it changes the operator's own
    push workflow. Raise it.
14. **★ `.gitignore:20` (`/fixtures/external/`) is LOAD-BEARING and nothing
    says so.** The staged veraPDF corpus declares CC BY 4.0 over content
    including the **Isartor** suite, whose own manual states *"Redistributing
    all or parts of the Isartor test suite is also not allowed."* **The
    repository is public**, so committing that tree would *be* redistribution.
    One `.gitignore` line is the entire control. Operator question `(bx)`.

## §B — HOW TO RUN THE GATES, because this cost an hour

★ **27 commands now** — `tools/check-clap-help.py` was added 2026-08-29 and
wired into `ci.yml` and `check-ci-parity.py`.

★ **"PASS — 27 commands" over-claims and the label says so.** Two deliberate
skips: `cargo about` (only when the dependency set changes) and **`cargo test
--workspace --all-features`, replaced by plain `cargo test --workspace`**. ⇒
**every green this project reports is a default-features green.**

★★ **THE RECIPE. Deviating from it wasted an hour on 2026-08-29:**

1. **Warm the cache first**: `cargo test --workspace --no-run`.
2. **Run in the FOREGROUND**: `timeout 580 bash tools/run-gates.sh`.
   A **background** run gets **killed** by a task-lifetime cap — it happened
   **three times**, always partway through `cargo test --workspace`, always
   with no failure recorded.
3. **Do nothing else while it runs.** No `du`, no `find`, no `git status` on a
   worktree, no second cargo. Concurrent heavy I/O starves the doctest binary
   and it fails with `0xc0000142` / *"Couldn't compile the test"* across dozens
   of unrelated files. **That looks exactly like a broken tree and is not.**
   Confirm by running `cargo test -p pdfce-core --doc` alone before diagnosing
   anything.

★★ `R226`: also run `python tools/check-passes-filed.py --strict-tip` before
ending any session whose tip commit claims a Pass ID. The plain run
**deliberately defers the tip** and reports `clean`.

★★★ **A gate sweep certifies the tree it ran on.** Re-run after the last edit,
not before it. **Do not hand-type a subset** to dodge the time limit — that is
exactly how the two filing gates got omitted and CI went red.

---

## §C — MEASURED AT WRITE TIME. Re-run; do not copy forward

- Conformance suite: **6 FAIL / 29 pass / 16 unresolved of 51** — **CARRIED
  FORWARD, NOT RE-MEASURED since 2026-08-27.** No render code has changed
  since, so it should hold; **verify before quoting.**
- `MAX_OUTLINE_DEPTH` = **32** (`outline.rs:218`);
  `MAX_OUTLINE_ITEMS` = **200_000** (`pageops/references.rs:64`). The gap
  between those two is what `Pass 161.1` was about.
- `docs/core-api/02-editing-and-saving.md`: **3,161 lines, 78 clauses, 159
  verbs, `EditError` 92 variants.** The gate re-derives all of these.
- Real-world outline corpus: of 8 nested outlines under `fixtures/external/`,
  6 move cleanly; the other 2 hit a **pre-existing correct refusal** (recovered
  xref ⇒ incremental save refused, `--mode full` succeeds). Not a defect.
- **Latest backup bundle and portable build are both STALE** — both predate
  this session's five commits. Re-take.

---

## §D — LESSONS

### ★★★ Prose is what sabotage audits, and an ELABORATE justification is the
strongest signal that something is wrong

**Three instances in one session**, all in code I had just written: a comment
claiming a per-verb "skip unchanged" filter *"IS the minimal-diff guard"* (it
is not — `dirty_set` is, §11.1, and the filter is **unobservable through the
public API** so no test can cover it); a `treat_open` parameter whose five-line
rationale described a branch that **could never fire**; and the cycle guard's
argument for asking downward rather than upward, whose second half was a **real
hole** (`161.1`).

⇒ **I do not write five defensive lines about something obvious. I write them
when I have REASONED rather than MEASURED — which is exactly when I am most
likely to be wrong. Sabotage the code your comment is proudest of.**

### ★★★ N of M paths, and the untested one was the shipped one

`Pass 162.0` wired **two of three** save paths for a new font resource. Every
unit test passed — they all exercise the session. **The CLI uses the third**,
and it printed a disclosure about a resource it had not written.

⇒ **Before changing what an edit WRITES, enumerate the save paths.** There are
three: `EditSession::format_text`, its form twin, and the one-shot
`text_edit::set_format`. Caught only by running the binary and reading the
**output file** rather than the report.

### ★★ A dispatch is the least-checked artefact this project produces

I told `pdfce-librarian` the fifth doc-comment orphaning was *"Fixed"*. **It
was not** — I had found it, described it in a commit message, and moved on. The
librarian **verified the claim against live source** and reported it back;
`Pass 161.2` fixed it. Every other claim here has a gate behind it. **Finding a
defect and reporting a defect are separate acts, and the gap is invisible.**

### ★★ Doc-comment orphaning: EIGHT instances, and MORE THAN ONE cause

Six found by eye over weeks. `tools/check-clap-help.py`, written in twenty
minutes, found **two more in seconds** — both shipping **blank** `--help`,
**neither caused by a splice**. ⇒ the *"insert after a closing brace"* remedy
could never have closed the class, and neither could more careful reading.

**In a clap-derive CLI a doc comment IS shipped operator-facing UI**, and
nothing checks it: not the compiler, not clippy, not `missing_docs` (private
items in a binary crate), and no test, **because no test reads help text**.

**What did NOT work, so it is not re-derived:** a structural detector for the
**weld itself** produced **8,136 candidates**, because that is the shape of
every ordinary paragraph ending. The gate catches the **donor** of a weld,
never the **recipient**.

### ★ `check-string-gaps.sh` earns its place

Three disclosure strings shipped with mangled line continuations in `162.0`,
patched in via heredoc. The gate caught all three. **Patch prose with the Edit
tool, never through a shell heredoc.**

### Still biting — `R225`: a fixture whose two candidate answers coincide

Avoided **twice** in `161.0`: nesting Chapter 2 under Chapter 1 produces a flat
title order **character-for-character identical** to the input, so a title-list
assertion would pass whether the re-parenting happened or not. Assert on
**structure** and `/Count`; the CLI suite asserts on `level=`.

---

## §E — OPERATOR DECISIONS OUTSTANDING

**None of these block anything.**

- **The private suite's name in pushed commit messages** — 82 already-pushed
  messages, 302 occurrences across 1,042 published commits. The gate scans the
  **work tree** and never could reach commit messages. Accept, or rewrite and
  force-push — **his call**. Fresh commits are clean. Re-run the count with the
  gate's own decoded needles, and **do not write the decoded terms down**.
- **`(bv)`–`(bz)`, five licence questions**, each with a conservative default
  already chosen. **`(bw)` first** — veraPDF's machine-readable **rule
  definitions** are **CC BY 4.0, not GPL**, softer than assumed and therefore
  most likely to be waved through; CC BY 4.0 is not a software licence and
  carries perpetual attribution.
- **`(bl)` OCR model weights** — whether a **CC-BY-SA-4.0** model file may ship
  inside pdfce's **MIT** portable folder. *Default if unanswered: ship neither
  model set.*

## §F — OUTBOUND, AWAITING A REPLY

`D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\reply_the_clipboard_route_is_the_same_loss_and_bookmarks_now_move.md`
(2026-08-29) answers `pdfceGUI`'s clipboard-fidelity question **by
measurement**: `copy_annotations` → `ObjectClip` → `paste_objects` is the
**same loss** they already have — `ClipAnnotation::Markup` is literally
`Box<MarkupSpec>`, and `paste_objects` calls **plain `add_markup`**, so the
clip route additionally drops the **opacity** they just wired. And
`ObjectClip::to_bytes` **does not serialise annotations at all**. **They were
told to keep their current path.**

A lossless markup copy (`/T`, `/M`, note, `/CA`, `/Popup`, `/IRT`, `/RC`) needs
`MarkupSpec` extended **and** `/IRT` reference rewriting — **a graph problem,
not a value problem**. Backlog, unscoped. **Do not build it speculatively.**

**Check BOTH channels every session** — `pdfce_FeatureRequests` and
`iccce_FeatureRequests`. They live outside the repo, so **no gate can
contradict a stale "it's empty" claim.**
