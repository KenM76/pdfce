# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**. What is
true now, plus a pointer. Corrections and their prior wording live in the
**append-only** record — `ROADMAP.md` and `SESSION_LOG.md`.

Written 2026-08-28 late evening. Ledger at write time: **Pass ceiling 160.0**,
rules **R226**, decisions **096**, filings **316**.

**No code was written in the session that produced this file.** It was a
scoping and filing session. Everything in §0 is exactly where the previous
session left it, except item 1 of §B, which is unchanged but now verified
rather than assumed.

---

## §0 — THE OPERATOR'S STANDING INSTRUCTION

> *"do all 4"* — the four things not completely editable, which he asked me to
> enumerate and then to fix.

**Two are done. Two are not.** That is still the shape of the current task.

| # | gap | state |
|---|---|---|
| 1 | **Rotation** of anything carrying a `/Rect` | **SHIPPED** — `Pass 155.0`, core + CLI. Annotation family only. |
| 2 | **ce dimensions** | **DONE.** Two of the three gaps reported **never existed**. Rotation shipped as `Pass 159.0`; scaling **declined by name**, operator confirmed. |
| 3 | **Bookmarks** — rename, delete, reorder, re-parent | **PARTIAL** — rename and delete ship (`156.0` core, `157.0` CLI). **Reorder and re-parent do not.** ★ Re-verified 2026-08-28 by grep over `pdfce-cli/src/main.rs`, `pdfce-core/src/outline.rs` and `edit.rs`: no `reorder`/`reparent`/`move_bookmark` verb exists on the outline family. This is a **measured** absence, not a carried-forward claim. |
| 4 | **Fonts** — restyle to a face the document lacks; replace a font throughout | **NOT STARTED, but SCOPED — read §0a before writing any code.** |

**Recommended order: 3 then 4.** Item 3 is the smaller of the two and its
unlink half already exists (see §B item 1). Item 4 is scoped but is a genuinely
new capability.

## §0a — GAP 4 (FONTS): MEASURED, NOT GUESSED. Read this before writing code.

Scoped 2026-08-28 by running the shipped binary. The expensive part of this
Pass — finding out what already exists — is done.

### What is genuinely missing

**`format-text --set-font` can only select a font the page ALREADY carries.**
Verified: `font-preflight` on `format_family.pdf` lists exactly the three page
resources `/F1 /F2 /F3` and adjudicates among those. There is no path to a face
the page does not have.

**`embed-font` does NOT close this, and its help text now says so.** It
supplies the missing font *program* for a face the PDF already **references**.
It cannot introduce a new face. That correction shipped in `Pass 160.0` — the
help text was the surface an operator would read *before* filing a bug about
this exact gap, and it invited the misreading.

⇒ **The missing capability is: add a font resource to a page that lacks it,
then point a run at it.** `Pass 142.0` in *Backlog* is the entry for this; it
is **de-prioritised but NOT closed** — the consuming project said *"synthetic
is enough, drop 142.0 down the queue"* and explicitly scoped that as a report
of **their** use, not a decision about ours.

## §0b — NEW THIS SESSION: THE CONFORMANCE VALIDATOR BUCKET

Filed 2026-08-28 (**314th filing**) at the **bottom of `ROADMAP.md`'s Backlog**,
unscoped, **no Pass ID**, exactly as the operator asked. Do not promote it
without him saying so.

**Origin, because it explains the shape:** he surfaced a thread in
r/accessibility — a US state-org worker with 1000+ pages including fillable
forms to remediate before a deadline, alone, no budget. The thread's unanimous
verdict was *"Open source tools cannot remediate PDFs"*, with manual vendors
quoted at **~$5/page**. A validator is what produces the pass / fail /
needs-human triage buckets those same commenters recommended.

**Four arcs, A–D.** The split is by **whether the work can be GRADED**, not by
difficulty:

- **Arc A — implementable AND scoreable today.** PDF/A (all 11 levels),
  PDF/UA-1, PDF/UA-2, WTPDF. Oracle **and** corpus are already on this machine:
  **veraPDF 1.30.2** at `D:\tools\verapdf\verapdf.bat`, and **2,907 corpus
  files** at `fixtures/external/veraPDF-corpus/`. Nothing here needs research
  first.
- **Arc B — implementable but UNGRADED.** PDF/X, PDF/VT, PDF/R. veraPDF has no
  flavour for any of them, and no **redistributable** corpus exists. ★ **PDF/E-2
  was never published** — the CAD/engineering conformance target is
  **PDF/A-4e**, which is in arc A with a 19-file corpus. A future session
  chasing "PDF/E for the CAD files" is chasing a standard that stopped at draft.
  ★★ **CORRECTION, made 2026-08-28 after the filing and before the push:** the
  research concluded PDF/X was corpus-**absent**. It is not. **pdfce already
  holds a licensed PDF/X corpus locally** — the **print-conformance suite** it
  measures render parity against, whose patch files are labelled by PDF/X part
  (`…_x1a.pdf`, `…_x3.pdf`) in the private map at
  `D:\Dev\pdfce-private\suite\manifest.json`. What is absent is a corpus that
  may be **committed**, which is a different claim. ★★★ **But it is a WEAK
  oracle and must not be quoted as a strong one:** it is an
  **all-conforming** set, so it can detect a validator that **wrongly fails** a
  good file and cannot detect one that **wrongly passes** a bad one — the same
  limitation already recorded against the Cal Poly PDF/VT corpus. Whether it
  carries per-file conformance verdicts at all, as opposed to reference images
  for rendering, is **UNESTABLISHED**.
- **Arc C — PDF/UA and the human-judgement split.** The deliverable is a
  **three-bucket verdict — pass / fail / needs human — not a boolean**, and the
  third bucket is **required by the standard's own scope**: ISO 14289-2 §1
  newly *excludes* "requirements specific to content", so *"is this alt text
  correct?"* is outside the standard, not merely untestable.
- **Arc D — base syntax and PAdES.** Base syntax has **no standard test set by
  design** (both ISO 32000 editions exclude conformance validation from Scope);
  the licence-clean substitute is the Arlington PDF Model. PAdES is **not
  file-alone checkable** — verdicts include `INDETERMINATE` and inputs include
  trust anchors and a validation policy — and is gated behind `sign` existing
  at all, which it does not (`Command::Sign` is a stub; `signature.rs`'s own
  header opens *"This module verifies nothing"*).

**Reference material, named here because a cross-RAG deliverable recorded only
in the producing RAG has been filed, not handed off:**

- `D:\Dev\Rag-Specialized\PDF_Spec\conformance\conformance__ref__validator_scope.md`
  (989 lines, 76 `CV-*` ids) — rule counts per level, corpus verdict-encoding
  convention, licence basis per source.
- Seven files under `D:\Dev\Rag-Specialized\Acrobat_Features\` — four
  `pdfa_conform__*`, three `accessibility__*`.

**Two facts from those references that change how the work is sized:**

1. **UA-2's real cost is not its own 91 rules but the 1,636 ISO/TS 32005
   structure rules it leans on.** A session reading "91" will under-scope this
   by an order of magnitude.
2. **The corpus's expected verdicts are mechanical from the FILENAME**, not the
   outline bookmark. Measured over 2,906 files: 2,874 agree, **4 disagree**, 28
   lack the bookmark. Read the filename; a bookmark-reading implementation will
   mis-grade four files and never know.

**The parity headline:** ★ **Acrobat Pro has no tool that fully validates
PDF/UA.** Preflight stamps an identifier and checks isolated points; the
Accessibility Checker runs 508/WCAG heuristics, not tag-structure rules —
which is why practitioners run PAC alongside it. This is a place pdfce can
**exceed** the reference rather than match it.

**Design constraint, settled before any code:** the validator **reports and
never mutates.** Acrobat arrived at the same line independently — a Preflight
*check* only detects; only a *fixup* touches the file; and a fixup that cannot
fully correct something **discloses the leftover rather than guessing**. That
is project rule 4 in someone else's product. Any repair is a separate, named
verb.

---

## §A — RUN `bash tools/run-gates.sh`

26 commands. It has caught something real on nearly every Pass, including
defects in the Passes' own new code.

★ **"PASS — 26 commands" over-claims and the label says so.** Two skips are
deliberate: `cargo about` (only fires when the dependency set changes) and
**`cargo test --workspace --all-features`, replaced by plain
`cargo test --workspace`**. ⇒ **every green this project reports is a
default-features green.** Do not quote it as if `--all-features` had passed.

★★ **NEW — `R226`: also run `python tools/check-passes-filed.py --strict-tip`
before ending any session whose tip commit claims a Pass ID.** The plain run
**deliberately defers the tip** (a commit cannot cite its own hash) and reports
`clean`. That is correct reasoning with an expiry date, and see §D for how it
expired.

---

## §B — OWED, consolidated

1. **Bookmark reorder and re-parent.** The unlink half is written and exercised
   by `delete_outline_item`; the relink half is the same machinery pointed the
   other way. **This is §0 item 3 and the recommended next Pass.**
2. **Widget rotation (`/MK /R`)** and **ce-dimension rotation** — both refused
   by name by `rotate_annotation`, both unbuilt.
3. **The trap X on the grey/K-black conformance patch.** Cause unknown and
   **not** the defect `Pass 143.0` fixed. Lead: that patch paints the same 50 %
   grey **both ways** (`0.5 g` and `0 0 0 0.5 k`) deliberately, and `G .5` — a
   grey *stroke* — appears in its streams while every synthetic fixture uses
   fills only.
4. **`OverprintZeroTintScope::AllProcessSpaces` is unmeasured.** pdfce's
   RGB→CMYK is naive, so a pure red preserves a cyan backdrop under it and
   whether Acrobat agrees is unknown. Not the default; do not promote it
   without a measurement.
5. **Two `iccce` inbounds from 2026-08-28 15:03/15:04** — informational, no
   reply owed. **Its "invisible X" is NOT the trap X in item 3.** Theirs is
   `PCS3_130`, a CMM/ICC-source-profile patch they filed as *theirs* under
   decision 064; ours is the grey/K-black **overprint** patch. Two X's, two
   patches, two causes, one word — conflating them closes an open item on
   evidence about a different one.
6. **`iccce`'s ΔE00 figures are withdrawn by its own author** (`DL-070`: no ΔE
   against a screen capture). Only the 8-bit deltas may be quoted.
7. Ambiguity-register entries owed to `pdfce-spec-librarian`:
   `overprint_zero_tint_scope`, and `render.hairline_clamp_policy` since
   2026-08-09.
8. **No CLI tests** for `rename-bookmark`, `delete-bookmark` or
   `rotate-annotation`. `crates/pdfce-cli/tests/` holds 29 files including
   `move_annotation.rs` and `resize_annotation.rs`; the last two CLI-shipping
   Passes added none. Manual binary verification does not survive the session.
9. **`pdfceGUI` has 68 of 149 implemented capabilities unwired** (45.6 %). The
   CLI gap is **0 of 149** — rule 11 is holding. Every remaining gap in the
   *Implemented* half is a GUI gap.
10. **★ NEW — a pre-push check for a deferred tip.** `R226` closes §D's hole
    *procedurally*, and a procedural rule is precisely the kind of thing that
    was skipped to create the hole. The mechanical form is cheap: refuse a push
    when the tip is a Pass-claiming commit that `--strict-tip` rejects. **Not
    built, and NOT to be built unasked** — it changes the operator's own push
    workflow, and he was not asked. Raise it; do not assume it.
11. **★ NEW — the gitignore at `.gitignore:20` (`/fixtures/external/`) is
    LOAD-BEARING and nothing says so.** The staged veraPDF corpus declares
    CC BY 4.0 over content that includes the **Isartor** suite, whose own
    manual — shipped inside the corpus as `Isartor test suite manual.pdf` —
    states *"Redistributing all or parts of the Isartor test suite is also not
    allowed."* Verified 2026-08-28: `git check-ignore` matches and
    `git ls-files fixtures/external` returns **0**. **The repository is
    public**, so committing that tree would *be* redistribution. One
    `.gitignore` line is the entire control. See operator question `(bx)`.

---

## §C — MEASURED AT WRITE TIME. Re-run; do not copy forward

- `main` was **0 commits ahead of `origin/main`** before this session's filing
  commit.
- Latest backup bundle: `pdfce-20260828-2201-ae476bc-full.bundle` — **behind
  `HEAD`**; re-take.
- Latest portable build: `D:\builds\pdfce-20260828-1745-66efc9a`. (The previous
  handoff named `…-1639-eace74c`, which was already stale when written — a
  reminder that this section decays fastest.)
- Conformance suite: **6 FAIL / 29 pass / 16 unresolved of 51** — **CARRIED
  FORWARD, NOT RE-MEASURED this session.** No render code changed, so it should
  hold; verify before quoting.
- **Isartor is 204 test files, not 205.** The 205th is the suite's own manual.
  Both numbers are correct answers to different questions; the corpus-size
  figure that matters for grading is **204**.

---

## §D — THE LESSON THIS SESSION LEARNED, AND THE ONES STILL BITING

### ★★ NEW — two correct decisions that interact badly

**`Pass 160.0` shipped code at HEAD, was pushed to a public remote, and was
never filed.** It was caught 2026-08-28 by reading `check-ledger-numbers.py`'s
ceiling (159.0) against `git log` (160.0) — by hand, because nothing compares
those two automatically.

**Neither half was a defect on its own:**

- `check-passes-filed.py` **defers the tip commit by design** — a commit cannot
  cite a hash that does not exist yet — and reports `clean`. Correct, and it
  carries an implicit assumption: *the filing commit follows shortly.*
- Rule 8 was amended **2026-08-27** (decision **090**, *"always push"*) to make
  an ordinary fast-forward of `main` standing-authorized.

⇒ **The amendment removed the pause between committing and pushing, and that
pause was also — incidentally, unnoticed — the moment a deferred tip would be
spotted. One day later, a deferred tip was pushed.** The deferral's expiry
condition silently became "never".

★ **The generalisable shape: a check that DEFERS rather than fails depends on
something happening later, and nothing enforces the later thing.** Search for
other deferrals on that basis, not on whether they look risky.

★★ **And the recurrence is the sharpest part.** `Pass 160.0`'s own commit
message records catching the identical failure one Pass earlier — *"I told the
librarian 'Pass 158.0 was already filed'. It was not… A claim about what a
document CONTAINS is checkable in one command, and I did not run it."* The
remedy was applied backwards to `158.0` and never forwards to the commit
applying it. **Same shape as the doc-comment orphaning below: a remedy derived
from part of a finding, shipped as covering the whole.**

### Still biting — `R225`: a fixture whose two candidate answers coincide

**It cannot discriminate between them, and the test reads as passing coverage.**
Three instances in three subsystems in one day: an appearance whose `/Matrix`
was the identity (composing a rotation and replacing the matrix give the same
six numbers); a colour-scope pair with no non-grey source present; an **open**
bookmark, where visible-count equals subtree-size so the two readings of
`/Count` coincide.

★ Why it earned a rule rather than a note: **`R221` already names
sabotage-survival as the signature of a masking defect, but names a different
cause** (two agreeing predicates). Both return "nothing went red" and **the
remedies are opposite** — delete the duplicate versus change the input.

### Still biting — doc-comment orphaning, four instances

Splicing a function by anchoring on `pub fn name(` lands **inside the previous
item's doc block**, by construction. The walk-back-over-`///` mitigation failed
on the third instance because an **`#[allow(...)]` attribute** intervened, so
the walk terminated at zero steps.

⇒ **Insert AFTER a function's closing brace.** A doc comment binds to what
*follows* it, so there is no preceding run to land inside.

★ **A fourth instance was live at HEAD for nine days** and neither accident
that caught the others applied to it. **Two catches are not evidence of
coverage.**

### Still biting — an unticked `[ ]` box is unfalsifiable

`[x]` is falsifiable by the build. **`[ ]` is falsifiable by nothing** — no
test, no gate, no compiler notices a capability arriving. `FEATURES.md`
therefore decays in exactly one direction and is most wrong where it is most
consulted. ⇒ **Grep the Implemented section before believing the Planned one**,
and run the verb before reporting a gap. (§0 item 3 above was re-verified for
exactly this reason.)

---

## §E — OPERATOR DECISIONS OUTSTANDING

**None of these block anything.**

### Carried forward — the private suite's name in pushed commit messages

It appears in **82 already-pushed commit messages** (302 occurrences across
1,042 published commits). The gate scans the **work tree** and never could
reach commit messages; the 2026-08-10 acceptance of material already in history
predates the 2026-08-25 scrub ruling by two weeks. Accept, or rewrite and
force-push — **his call**. Fresh commits are clean. Re-run the count with the
gate's own decoded needles rather than trusting this figure, and **do not write
the decoded terms down**.

### New 2026-08-28 — five licence questions, `(bv)`–`(bz)`

Filed with the validator scope. Each has a conservative default already chosen,
so nothing is blocked. Full text in `ROADMAP.md` *Open operator questions*.

- **`(bv)`** veraPDF library is GPLv3+/MPLv2+ — invoking `verapdf.bat` as a
  separate dev-time process is the settled, unproblematic case; **linking is
  categorically out**. Recorded so the safe half is not re-litigated.
- **`(bw)`** ★ **The one to read first.** veraPDF's machine-readable **rule
  definitions** are **CC BY 4.0, not GPL** — softer than assumed, and therefore
  **the one most likely to be waved through**. CC BY 4.0 is not a software
  licence and carries perpetual attribution. Whether pdfce may derive its rule
  set from those profiles is unresolved.
- **`(bx)`** The Isartor redistribution conflict — see §B item 11.
- **`(by)`** Arlington PDF Model: `LICENSE` says Apache-2.0, `NOTICE.txt`
  splits software from documentation; which bucket `tsv/` falls in is
  unresolved.
- **`(bz)`** The public PDF/X and PDF/VT corpora the PDF Association's index
  lists (Altona, Cal Poly, and one whose name this repository does not carry
  under the 2026-08-25 ruling) state **no licence**, so under project rule 7
  they cannot be staged — which is the practical reason arc B is ungraded, over
  and above veraPDF having no flavour for them. ★ **Read this together with the
  arc B correction in §0b:** pdfce's own licensed **print-conformance suite**
  is a PDF/X corpus and is already on disk, so the question is about what may
  be **committed and redistributed**, not about whether any PDF/X files are
  reachable.

### Still open from earlier — OCR model weights, `(bl)`

Whether a **CC-BY-SA-4.0** model file may ship inside pdfce's **MIT** portable
folder. Unchanged. *Default if unanswered: ship neither model set.*
