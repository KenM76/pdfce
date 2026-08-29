# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**. What is
true now, plus a pointer. Corrections and their prior wording live in the
**append-only** record — `ROADMAP.md` and `SESSION_LOG.md`.

Written 2026-08-29. Ledger at write time: **Pass ceiling 162.0**, rules
**R226**, decisions **096**, filings **318+**.

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

⇒ **There is no obvious "next" handed down by the operator.** Pick from §A
(owed items) or raise the Backlog with him. **Do not promote the conformance
validator (§0b) without him saying so** — he asked for it to sit unscoped.

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

1. **`Pass 142.0`** — non-standard-14 faces (see §0a). The largest remaining
   item, and the natural continuation of `162.0`.
2. **Widget rotation (`/MK /R`)** and **ce-dimension rotation** — both refused
   by name by `rotate_annotation`, both unbuilt.
3. **`rotate-annotation` has no CLI test.** The bookmark half of this owed item
   was closed 2026-08-29 by `crates/pdfce-cli/tests/bookmarks.rs` (18 tests,
   covering `rename-bookmark`/`delete-bookmark` retroactively). This was not.
4. **The trap X on the grey/K-black conformance patch.** Cause unknown, **not**
   the defect `Pass 143.0` fixed. Lead: that patch paints the same 50 % grey
   **both ways** (`0.5 g` and `0 0 0 0.5 k`) deliberately, and `G .5` — a grey
   *stroke* — appears in its streams while every synthetic fixture uses fills.
5. **`OverprintZeroTintScope::AllProcessSpaces` is unmeasured.** pdfce's
   RGB→CMYK is naive, so a pure red preserves a cyan backdrop under it and
   whether Acrobat agrees is unknown. Not the default; do not promote it
   without a measurement.
6. **`iccce`'s "invisible X" is NOT the trap X in item 4.** Theirs is
   `PCS3_130`, a CMM/ICC-source-profile patch filed as *theirs* under decision
   064; ours is the grey/K-black **overprint** patch. Two X's, two patches,
   two causes, one word.
7. **`iccce`'s ΔE00 figures are withdrawn by its own author** (`DL-070`: no ΔE
   against a screen capture). Only the 8-bit deltas may be quoted.
8. Ambiguity-register entries owed to `pdfce-spec-librarian`:
   `overprint_zero_tint_scope`, and `render.hairline_clamp_policy` since
   2026-08-09.
9. **A lossless markup clipboard copy** — Backlog, unscoped, **awaiting
   `pdfceGUI`'s answer on whether they need it.** See §E.
10. **A rustdoc-cleanliness gate** — Backlog. ★ **Read it at LOW priority:**
    measured 2026-08-29, the dangerous subclass (a doc naming a public verb
    that does not exist) was a population of **one**, and `Pass 161.0` closed
    it by shipping the verb. The other 150 are path-scope noise.
11. **★ A pre-push check for a deferred tip.** `R226` closes the hole
    *procedurally*, and a procedural rule is what was skipped to create it.
    Mechanical form is cheap: refuse a push when the tip is a Pass-claiming
    commit that `--strict-tip` rejects. **Not built, and NOT to be built
    unasked** — it changes the operator's own push workflow. Raise it.
12. **★ `.gitignore:20` (`/fixtures/external/`) is LOAD-BEARING and nothing
    says so.** The staged veraPDF corpus declares CC BY 4.0 over content
    including the **Isartor** suite, whose own manual states *"Redistributing
    all or parts of the Isartor test suite is also not allowed."* **The
    repository is public**, so committing that tree would *be* redistribution.
    One `.gitignore` line is the entire control. Operator question `(bx)`.
13. **★ 28 GB of stale agent worktrees** under `.claude/worktrees/` (7 of
    them, gitignored, 0 tracked files). All checked 2026-08-29: **nothing
    unsaved would be lost** — the six untracked `docs/decisions/*.md` in them
    are already in the main repo. Deleting is destructive and outside the
    working tree, so it is **the operator's call**; it has been raised.
    Cleanup if he agrees: `git worktree remove --force <path>` each, then
    `git worktree prune`.

---

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
- **The 28 GB of stale worktrees** (§A item 13) — raised, awaiting his word.

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
