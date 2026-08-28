# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**. What is
true now, plus a pointer. Corrections and their prior wording live in the
**append-only** record — `ROADMAP.md` and `SESSION_LOG.md`.

Written 2026-08-28 evening. Ledger at write time: **Pass ceiling 157.0**,
rules **R225**, decisions **095**, filings **312**.

---

## §0 — THE OPERATOR'S STANDING INSTRUCTION

> *"do all 4"* — the four things not completely editable, which he asked me to
> enumerate and then to fix.

**Two are done. Two are not started.** That is the whole shape of the current
task; everything else in this file is subordinate to it.

| # | gap | state |
|---|---|---|
| 1 | **Rotation** of anything carrying a `/Rect` | **SHIPPED** — `Pass 155.0`, core + CLI. Annotation family only. |
| 2 | **ce dimensions** — re-measure, rotate/scale, drag extension lines | **NOT STARTED** |
| 3 | **Bookmarks** — rename, delete, reorder, re-parent | **PARTIAL** — rename and delete ship (`156.0` core, `157.0` CLI). **Reorder and re-parent do not.** |
| 4 | **Fonts** — restyle to a face the document lacks; replace a font throughout | **NOT STARTED** |

### ★★ Take gap 4 (fonts) next, not gap 2 — and the reason is scope shape

This is the librarian's assessment and I agree with it.

**Gap 2 is five problems sharing one row.** `Linear` has to reconcile
`AxisConstraint`; `Angular` is **scale-invariant by design**, so "scale" means
something else or nothing; `Perimeter`'s centroid-relative offset needs
re-deriving; and `Circular`'s scale **collides conceptually with the existing
`set_group_scale` measurement-ratio verb**. A fresh session spends its first
hour deciding what the verb *means* per variant — **and that decision is
Ken's, not the engineer's**, because a ce dimension's text *is* its
measurement, so scaling one is a drafting-semantics question rather than a
geometry one.

**Gap 4 is one problem with a known shape.** `Pass 142.0` is already scoped,
and it was **de-prioritised rather than declined**. A fresh context can start
writing immediately.

**Cheaper option if the next session is short on room:** widget `/MK /R`
rotation. `rotate_annotation` refuses widgets by name and the refusal message
already states the mechanism (§12.5.6.19 Table 189, quantised 0/90/180/270).
Self-contained, no operator decision needed.

---

## §A — RUN `bash tools/run-gates.sh`

26 commands. It caught something real on **every** Pass this session, including
three defects in the Passes' own new code.

★ **"PASS — 26 commands" over-claims and the label now says so.** Two skips
are deliberate: `cargo about` (only fires when the dependency set changes) and
**`cargo test --workspace --all-features`, which is replaced by plain
`cargo test --workspace`**. ⇒ **every green this project reports is a
default-features green.** Do not quote it as if `--all-features` had passed.

---

## §B — OWED, consolidated

1. **Bookmark reorder and re-parent.** The unlink half is written and
   exercised by `delete_outline_item`; the relink half is the same machinery
   pointed the other way.
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
5. **★ Two `iccce` inbounds arrived 2026-08-28 15:03/15:04** — informational,
   no reply owed. **Its "invisible X" is NOT the trap X in item 3.** Theirs
   is `PCS3_130`, a CMM/ICC-source-profile patch they filed as *theirs* under
   decision 064; yours is the grey/K-black **overprint** patch. Two X's, two
   patches, two causes, one word — conflating them closes your open item on
   evidence about a different one.
6. **★ Its ΔE00 figures are withdrawn by its own author** (iccce `DL-070`: no
   ΔE against a screen capture). Only the 8-bit deltas may be quoted.
7. Ambiguity-register entries owed to `pdfce-spec-librarian`:
   `overprint_zero_tint_scope`, and `render.hairline_clamp_policy` since
   2026-08-09.
8. **No CLI tests** for `rename-bookmark`, `delete-bookmark` or
   `rotate-annotation`. `crates/pdfce-cli/tests/` holds 29 files including
   `move_annotation.rs` and `resize_annotation.rs`; the last two
   CLI-shipping Passes added none. Manual binary verification does not
   survive the session.
9. **`pdfceGUI` has 68 of 149 implemented capabilities unwired** (45.6 %). The
   CLI gap is **0 of 149** — rule 11 is holding. Every remaining gap in the
   Implemented half is a GUI gap.

---

## §C — MEASURED AT WRITE TIME. Re-run; do not copy forward

- `main` is **2 commits ahead of `origin/main`**.
- Backup bundle `pdfce-20260828-2010-988b22a-full.bundle` is **2 commits
  behind `HEAD`**. (It was 11 behind earlier today; taken at `988b22a`.)
- Latest portable build: `D:\builds\pdfce-20260828-1639-eace74c`.
- Conformance suite: **6 FAIL / 29 pass / 16 unresolved of 51**, unchanged
  across every Pass today.

---

## §D — THE LESSON THIS SESSION KEEPS RE-LEARNING

**A fixture whose two candidate answers coincide cannot discriminate between
them — and the test reads as passing coverage.** Minted as `R225` after three
instances in three different subsystems in one day:

- an appearance whose `/Matrix` was the identity, where *composing* a rotation
  and *replacing* the matrix produce the same six numbers;
- a colour-scope pair with no non-grey source present, so two enum values were
  indistinguishable;
- an **open** bookmark, where visible-count equals subtree-size, so the two
  readings of `/Count` coincide.

★ The reason it earned a rule rather than a note: **`R221` already names
sabotage-survival as the signature of a masking defect, but names a different
cause** (two agreeing predicates). Both return "nothing went red" and **the
remedies are opposite** — delete the duplicate versus change the input.

### And the one that bit three times in one file

**Doc-comment orphaning — FOUR instances, and the mitigation failed on the
third.** Splicing a function by anchoring on `pub fn name(` lands **inside the
previous item's doc block**, by construction.

The mitigation I invented — walk back over a contiguous run of `///` lines —
failed on the third instance, and **my first diagnosis of why was wrong.** I
recorded *"the block was not contiguous"*; the librarian read the source and
the block is **29 unbroken `///` lines**. What sits between it and the `fn` is
an **`#[allow(...)]` attribute**, so the walk terminated at **zero steps** and
spliced between the attribute and the function.

★ The two diagnoses have **opposite remedies** — *not contiguous* implies a
better run-detector, *an attribute intervenes* implies **no upward look can
ever be sufficient**. The remedy I shipped happens to be the right one:

⇒ **Insert AFTER a function's closing brace**, where there is no preceding doc
run to land inside. A doc comment binds to what *follows* it.

★★ And the RAG carrier had already written the attribute sentence. The
mitigation was built from that file, **the same day**, implementing its
doc-run clause while skipping the attribute clause two lines beneath. The
lesson is not "write it down" — it is that **a remedy was derived from part of
a finding and shipped as covering the whole.**

★★★ **A fourth instance was live at HEAD for nine days** (`cmd_list_fields`'
documentation attached to `cmd_add_bookmark` since `d32872a`, `Pass 103.0`) —
fixed in `Pass 158.0`. Neither accident that caught the others applied to it:
no trailing list to trip clippy's lazy-continuation lint, no stranded
attribute to make it report a wrong argument count. **Both catches this
session were luck, by a different accident each time. Two catches are not
evidence of coverage.**

---

## §E — OPERATOR DECISION STILL OUTSTANDING

The private suite's name is in **82 already-pushed commit messages** (302
occurrences across 1,042 published commits). The gate scans the **work tree**
and never could reach commit messages; the 2026-08-10 acceptance of material
already in history predates the 2026-08-25 scrub ruling by two weeks.

Accept, or rewrite and force-push — **his call, and it blocks nothing**. Fresh
commits are clean. Re-run the count with the gate's own decoded needles rather
than trusting this figure, and **do not write the decoded terms down**.
