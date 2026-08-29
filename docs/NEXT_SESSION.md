# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**. What is
true now, plus a pointer. Corrections and their prior wording live in the
**append-only** record — `ROADMAP.md` and `SESSION_LOG.md`.

Written 2026-08-29. Ledger at write time: **Pass ceiling 161.2**, rules
**R226**, decisions **096**, filings **317+**.

---

## §0 — THE OPERATOR'S STANDING INSTRUCTION

> *"do all 4"* — the four things not completely editable, which he asked me to
> enumerate and then to fix.

**Three are done. ONE is left.**

| # | gap | state |
|---|---|---|
| 1 | **Rotation** of anything carrying a `/Rect` | **SHIPPED** — `Pass 155.0`, core + CLI. Annotation family only. |
| 2 | **ce dimensions** | **DONE.** Two of the three gaps reported never existed. Rotation shipped as `Pass 159.0`; scaling **declined by name**, operator confirmed. |
| 3 | **Bookmarks** — rename, delete, reorder, re-parent | **DONE 2026-08-29.** Rename/delete were `156.0`/`157.0`; **reorder and re-parent shipped as `Pass 161.0`** (core + CLI, same Pass), hardened by `161.1`. `set_outline_open` shipped alongside. |
| 4 | **Fonts** — restyle to a face the document lacks; replace a font throughout | **THE ONLY ONE LEFT. Measured and split — read §0a; most of it already exists.** |

⇒ **§0a is the next session's work.**

## §0a — GAP 4 (FONTS): MEASURED 2026-08-29 AGAINST THE SHIPPED BINARY

### The gap, confirmed by running it — not carried forward

`format-text --set-font` refuses **any** face that is not already a font
resource on the page. Measured on `fixtures/synthetic/textedit/format_family.pdf`
(which carries exactly `/F1` Times-Roman, `/F2` Calibri-Bold, `/F3` Times-Bold):

```
--set-font Helvetica → refused: "not an existing font resource on this page;
                                 adding a new font resource / embedding a new
                                 face is deferred (FF-C)"
--set-font Arial     → same    --set-font Courier → same    --set-font F9 → same
```

The refusal is honest and by name. `embed-font` does **not** close this — it
supplies a missing font *program* for a face the PDF already **references**
(that correction shipped in `Pass 160.0`).

### ★★★ THE WHOLE AUTHORING HALF ALREADY EXISTS AND HAS NO CALLER FROM THIS
### PATH. `Pass 162.0` IS WIRING, NOT BUILDING.

Measured 2026-08-29 by grepping for a writer, **after** first assuming one
would have to be written. Both of these are in `text_edit/addtext.rs`, already
exercised by the add-new-text path:

| what | where | state |
|---|---|---|
| `fn build_font_dict(base_font, symbolic, enc, font) -> Object` | `addtext.rs:1866` | Emits the **full** form — `/Type /Font`, `/Subtype /Type1`, `/BaseFont`, `/FirstChar 32`, `/LastChar 255`, `/Widths` built from `fontdata::encoding_glyph_name` + `std14_width`, **and correctly omits `/Encoding` when `symbolic`**. This is exactly the form the spec RAG recommends. |
| `pub(crate) fn pick_font_name(existing: &Dict) -> Vec<u8>` | `addtext.rs:1912` | `pdfceF1`, `pdfceF2`, … — first unused. Its doc explains the `pdfce` prefix keeps it clear of the `/F{n}` producer convention, **and that the page and an appended stream share one effective resource dict (§7.8.3)**, so a colliding name would shadow a font the original content depends on. |

⇒ `Pass 162.0` = call these two from `format_text`'s set-font path, write the
new object, patch `/Resources /Font`, and disclose. **Do not write a third
font-dict builder.**

**One mechanical detail so it is not a surprise:** `pick_font_name` is already
`pub(crate)` and callable from `format.rs`, but **`build_font_dict` is
module-private** (`fn`, no visibility) in `text_edit::addtext`, and
`text_edit::format` is a **sibling** module — so it must be widened to
`pub(crate)`. That is the whole of the access change; both modules are declared
in `text_edit/mod.rs`.

★ **And the acceptance gate needs no change either.** `accept_font_target`
(`format.rs:2193`) takes `target_dict: &Dict` — a plain dictionary, not an
object id — so the **synthesized dict can be handed to the existing coverage
check before anything is written**. Preview and commit therefore cannot drift
(`R221`) without any new machinery. Build the dict with all-**direct** values
(no `/FontDescriptor` indirect reference) and there is no chicken-and-egg
between planning and object allocation.

### A duplicate worth collapsing while you are in there (R171, not urgent)

`vartext::base_font_name` (private, `vartext.rs:389`) and
`fontdata::std14_base_font_name` (`pub`, `fontdata/mod.rs:317`) are **two
14-arm tables of the same §9.6.2.2 spellings**. Compared 2026-08-29: **they
agree on all fourteen**, so this is a duplicate rather than a live defect —
but it is two answers to "how is this face spelled?" in one binary, and the
`/BaseFont` name is the thing a reader matches on.

`vartext::standard14_font_dict` is a **third** builder, and deliberately
different: the **minimal 4-key form with no `/Widths`**, for an appearance
stream's `/Resources` or an `/AcroForm` `/DR`. That difference is defensible
and documented; do not "unify" it with `build_font_dict` without reading why.

### The rest of what already exists — do not rebuild it either.

- **`crates/pdfce-core/src/fontdata/mod.rs` already contains a complete
  `Std14` module** — `pub enum Std14`, `std14_by_base_font`, `std14_width`,
  `std14_descriptor`, built-in encodings for Symbol/ZapfDingbats. Licence
  cleared (**APAFML**, embeddable in pdfce source; see
  `PDF_Spec/fonts/font__std14_afm_licensing.md`).
- **The spec RAG already carries the authoring direction**, decided, in
  `PDF_Spec/iso32000/iso32000__s__9.6.md` § "ADD-NEW-TEXT addendum":
  - emit the **full** form — `/FirstChar` + `/LastChar` + `/Widths`, optionally
    `/FontDescriptor` with **no** `/FontFile` — not the minimal 3-key dict;
  - `/Encoding /WinAnsiEncoding` for the **12 Latin faces**;
  - **OMIT `/Encoding` for `Symbol` and `ZapfDingbats`** (built-in encodings,
    Annex D.5/D.6);
  - `/BaseFont` **shall** be one of the 14 exact PostScript names.
- **PDF 1.5 deprecates the standard-14 special treatment as a `should`, not a
  `shall`** — *"conforming readers shall still provide the special treatment"*.
  **PDF 2.0's position is UNVERIFIED (paywalled); do NOT cite a 2.0 clause.**
  Emitting `/Widths` is the posture that is safe regardless of how 2.0 reads.

### So the gap splits, and the halves are very different sizes

- **4a — add a STANDARD-14 font resource to a page that lacks it**, then let
  `--set-font` accept it. **No font program, no subsetting, no descriptor
  synthesis, and no new dictionary builder** — see the section above: the
  builder and the resource-name picker both already exist and are already
  exercised. This is **wiring plus disclosure**, and is the **recommended next
  Pass (`162.0`)**. It makes `--set-font Helvetica` work on the fixture above.
- **4b — add a NON-standard-14 face.** Needs a font program: subset from
  `--font-dir`, build a `/FontDescriptor` with `/FontFile2`/`/FontFile3`, a
  widths array, an encoding, and a **unique six-uppercase-letter subset tag
  that must not collide with any already in the document** (§9.6.4 ST1–ST4,
  and ST4's uniqueness is **per file**, so the existing document's font names
  must be scanned, not just session state). This is `Pass 142.0` in *Backlog*,
  **de-prioritised but NOT closed** — the consuming shell said *"synthetic is
  enough, drop 142.0 down the queue"* and explicitly scoped that as a report of
  **their** use, not a decision about ours.

### ★ THE CONSTRAINT THAT WILL BITE, and it is written down in the code already

**The resource dictionary may belong to a Form XObject, not the page.**
`text_edit/format.rs`'s `plan_format_target` doc says so outright, and says
that `set_font` being **read-only** about resources is exactly what keeps this
verb from writing into a form's `/Resources`:

> *"retargeting cannot make this verb write to a form's `/Resources`, and the
> selector resolves against the **form's** dictionary, which is the correct
> one: `/F1` inside a form is a different font from `/F1` on the page
> (§8.10.1)."*

**Pass 162.0 removes that read-only property.** Adding a font to a **shared**
form's `/Resources` affects **every place that form is drawn**. Decision 076
already rules that shared forms edit **in place**, so this is consistent rather
than novel — but it is a side effect the operator did not ask for and it
**must be disclosed** (project rule 4). Decide and disclose; do not ask.

★ **The disclosure data is already in hand.** `EditPlanTarget`
(`text_edit/edit.rs:1592`) carries `form: Option<FormRef>` **and**
`invocations: Option<InvocationSet>` — the form's document-wide invocation set.
So "this font was added to a form drawn in N places" needs **no new
measurement**, just the sentence.

---

## §0b — THE CONFORMANCE VALIDATOR BUCKET (unchanged, still unscoped)

Filed 2026-08-28 at the **bottom of `ROADMAP.md`'s Backlog**, **no Pass ID**,
exactly as the operator asked. **Do not promote it without him saying so.**

Four arcs A–D, split by **whether the work can be GRADED**. Arc A (PDF/A all 11
levels, PDF/UA-1, PDF/UA-2, WTPDF) has oracle **and** corpus on this machine
already: **veraPDF 1.30.2** at `D:\tools\verapdf\verapdf.bat` and **2,907
files** at `fixtures/external/veraPDF-corpus/`.

Two facts that change the sizing, and one parity headline:

1. **UA-2's real cost is not its own 91 rules but the 1,636 ISO/TS 32005
   structure rules it leans on.** A session reading "91" under-scopes by an
   order of magnitude.
2. **The corpus's expected verdicts are mechanical from the FILENAME**, not the
   outline bookmark. Over 2,906 files: 2,874 agree, **4 disagree**, 28 lack the
   bookmark. Read the filename.
3. ★ **Acrobat Pro has no tool that fully validates PDF/UA.** A place to
   **exceed** the reference rather than match it.

**Settled before any code: the validator reports and never mutates.** Any
repair is a separate, named verb.

Reference material (named here because a cross-RAG deliverable recorded only in
the producing RAG has been *filed*, not *handed off*):
`PDF_Spec/conformance/conformance__ref__validator_scope.md` (989 lines, 76
`CV-*` ids), plus seven files under `Acrobat_Features/`.

**PDF/E-2 was never published** — the CAD/engineering conformance target is
**PDF/A-4e** (arc A, 19-file corpus). Do not chase PDF/E.

---

## §A — RUN `bash tools/run-gates.sh`

★ **Now 27 commands, not 26** — `tools/check-clap-help.py` was added
2026-08-29 and wired into both `ci.yml` and `check-ci-parity.py`.

★ **"PASS — 27 commands" over-claims and the label says so.** Two skips are
deliberate: `cargo about` (only when the dependency set changes) and
**`cargo test --workspace --all-features`, replaced by plain
`cargo test --workspace`**. ⇒ **every green this project reports is a
default-features green.** Do not quote it as if `--all-features` had passed.

★★ `R226`: also run `python tools/check-passes-filed.py --strict-tip` before
ending any session whose tip commit claims a Pass ID. The plain run
**deliberately defers the tip** and reports `clean`.

★★★ **A gate sweep certifies the tree it ran on.** Two sweeps were invalidated
this session by editing source while they ran. Re-run after the last edit, not
before it.

---

## §B — OWED, consolidated

1. **Widget rotation (`/MK /R`)** and **ce-dimension rotation** — both refused
   by name by `rotate_annotation`, both unbuilt.
2. **The trap X on the grey/K-black conformance patch.** Cause unknown and
   **not** the defect `Pass 143.0` fixed. Lead: that patch paints the same 50 %
   grey **both ways** (`0.5 g` and `0 0 0 0.5 k`) deliberately, and `G .5` — a
   grey *stroke* — appears in its streams while every synthetic fixture uses
   fills only.
3. **`OverprintZeroTintScope::AllProcessSpaces` is unmeasured.** pdfce's
   RGB→CMYK is naive, so a pure red preserves a cyan backdrop under it and
   whether Acrobat agrees is unknown. Not the default; do not promote it
   without a measurement.
4. **`iccce`'s "invisible X" is NOT the trap X in item 2.** Theirs is
   `PCS3_130`, a CMM/ICC-source-profile patch, filed as *theirs* under
   decision 064; ours is the grey/K-black **overprint** patch. Two X's, two
   patches, two causes, one word.
5. **`iccce`'s ΔE00 figures are withdrawn by its own author** (`DL-070`: no ΔE
   against a screen capture). Only the 8-bit deltas may be quoted.
6. Ambiguity-register entries owed to `pdfce-spec-librarian`:
   `overprint_zero_tint_scope`, and `render.hairline_clamp_policy` since
   2026-08-09.
7. **`rotate-annotation` still has no CLI test.** The bookmark half of this
   item was closed 2026-08-29 by `crates/pdfce-cli/tests/bookmarks.rs`
   (18 tests, covering `rename-bookmark` and `delete-bookmark` retroactively as
   well as the two new verbs). `rotate-annotation` was not.
8. **`pdfceGUI` gap: 71 of 152 implemented capabilities unwired (46.7 %).**
   The CLI gap is **0 of 149** — rule 11 is holding, and `Pass 161.0` kept it
   holding by shipping core and CLI in one Pass rather than splitting as
   `156.0`/`157.0` did.
9. **★ A pre-push check for a deferred tip.** `R226` closes the hole
   *procedurally*, and a procedural rule is precisely what was skipped to
   create it. The mechanical form is cheap: refuse a push when the tip is a
   Pass-claiming commit that `--strict-tip` rejects. **Not built, and NOT to be
   built unasked** — it changes the operator's own push workflow. Raise it.
10. **★ `.gitignore:20` (`/fixtures/external/`) is LOAD-BEARING and nothing
    says so.** The staged veraPDF corpus declares CC BY 4.0 over content that
    includes the **Isartor** suite, whose own manual — shipped inside the
    corpus — states *"Redistributing all or parts of the Isartor test suite is
    also not allowed."* **The repository is public**, so committing that tree
    would *be* redistribution. One `.gitignore` line is the entire control.
    See operator question `(bx)`.
11. **A lossless markup clipboard copy** — filed to Backlog 2026-08-29,
    unscoped, **awaiting `pdfceGUI`'s answer on whether they need it.** See §F.

---

## §C — MEASURED AT WRITE TIME. Re-run; do not copy forward

- Conformance suite: **6 FAIL / 29 pass / 16 unresolved of 51** — **CARRIED
  FORWARD, NOT RE-MEASURED since 2026-08-27.** No render code has changed
  since, so it should hold; **verify before quoting.**
- `cargo doc --workspace --no-deps`: **373 rustdoc warnings, 151 unresolved
  intra-doc links** (131 distinct). **No gate builds docs.** But see §D — the
  dangerous subclass is a population of **one** and it is closed.
- `MAX_OUTLINE_DEPTH` = **32** (`outline.rs:218`);
  `MAX_OUTLINE_ITEMS` = **200_000** (`pageops/references.rs:64`). The gap
  between those two numbers is what `Pass 161.1` was about.
- Real-world outline corpus: of 8 nested outlines under `fixtures/external/`,
  6 move cleanly; the other 2 hit a **pre-existing correct refusal** (recovered
  xref ⇒ incremental save refused, `--mode full` succeeds). Not a defect.
- **Latest backup bundle and portable build are both STALE** — the newest on
  disk predate this session's three commits. Re-take both.
- **`main` was 3 commits ahead of `origin/main`** before this session's filing
  commit (`b885777`, `6601783`, `1fc02f7`).

---

## §D — LESSONS

### ★★★ Prose is what sabotage audits, and an ELABORATE justification is the
strongest signal that something is wrong

**Three instances in one session**, all in code I had just written:

1. A comment claiming a per-verb "skip unchanged objects" filter *"IS the
   minimal-diff guard"*. Deleting the filter left **all 19 tests green** —
   `dirty_set()` enforces minimal diff **centrally**, by diffing against the
   base revision at save time (§11.1). ⇒ **Generalisable, now in the code: a
   per-verb "write only what changed" filter is UNOBSERVABLE through the public
   API and therefore cannot be covered by any test. A green suite is not
   evidence one is present.**
2. A `treat_open: Option<ObjId>` parameter with a five-line doc explaining why
   `set_outline_open` needed it. Sabotage: **green**. The chain starts at the
   item's **parent**, so the item can never appear in it and the override could
   never fire. **I had reasoned my way into an argument for dead code and
   written it down persuasively.**
3. The cycle guard's justification for asking downward rather than upward
   (`161.1`). Both halves wrong, and the second was a **real hole**.

⇒ **I do not write five defensive lines about something obvious. I write them
when I have REASONED rather than MEASURED — which is exactly when I am most
likely to be wrong. Sabotage the code your comment is proudest of.**

### ★★ A dispatch is the least-checked artefact this project produces

I told `pdfce-librarian` that the fifth doc-comment orphaning was *"Fixed"*.
**It was not** — I had found it, described it, and moved on. The librarian
**verified the claim against live source rather than taking the dispatch on
faith** and reported it back; `Pass 161.2` fixed it.

Every other claim here has a gate behind it. The only thing standing behind a
subagent dispatch is the subagent choosing to check. **Finding a defect and
reporting a defect are separate acts, and the gap between them is invisible.**

### ★★ Doc-comment orphaning: EIGHT instances, and the class has MORE THAN ONE
cause

Six were found by eye over weeks. `tools/check-clap-help.py`, written in twenty
minutes, found **two more in seconds** (`print-preview`, `render-page`, both
shipping **blank** `--help`), **neither caused by a splice**. ⇒ the existing
remedy — *"insert after a closing brace, never before a named anchor"* — could
never have closed the class, and neither could more careful reading.

**In a clap-derive CLI a doc comment IS shipped operator-facing UI**, and
nothing checks it: not the compiler, not clippy, not `missing_docs` (private
items in a binary crate), and no test, **because no test reads help text**.

**What did NOT work, so it is not re-derived:** a structural detector for the
**weld itself** — *"a doc line whose predecessor is non-empty and whose
successor is blank"* — produced **8,136 candidates**, because that is the shape
of every ordinary paragraph ending. Abandoned rather than shipped noisy. The
gate catches the **donor** of a weld, never the **recipient**.

### ★ The 151 broken doc links are HYGIENE, not a correctness risk

Measured: the dangerous subclass — a doc naming a **public verb that does not
exist**, the `insert_pages`-disclosure failure mode — was **exactly one**
(`EditSession::move_outline_item`, dangling since `Pass 156.0`), and
`Pass 161.0` closed it **by shipping the verb**. The other four `EditSession::*`
misses all resolve to real `pub fn`s and fail on **path scope**. The Backlog
entry should be read at that priority.

### Still biting — `R225`: a fixture whose two candidate answers coincide

Avoided **twice** deliberately in `161.0`: nesting Chapter 2 under Chapter 1
produces a flat title order **character-for-character identical** to the input,
so a title-list assertion would pass whether the re-parenting happened or not.
Assertions are on **structure** and `/Count`; the CLI suite asserts on `level=`.

A related trap caught mid-Pass: the dirty-set test's first version moved the
**last** top-level item, so the root's `/Last` genuinely moved and the root was
rightly dirty. **Counts cancelling and an object being unwritten are two
different claims.**

---

## §E — OPERATOR DECISIONS OUTSTANDING

**None of these block anything.**

- **The private suite's name in pushed commit messages** — 82 already-pushed
  messages, 302 occurrences across 1,042 published commits. The gate scans the
  **work tree** and never could reach commit messages. Accept, or rewrite and
  force-push — **his call**. Fresh commits are clean. Re-run the count with the
  gate's own decoded needles, and **do not write the decoded terms down**.
- **`(bv)`–`(bz)`, five licence questions** filed with the validator scope, each
  with a conservative default already chosen. **`(bw)` is the one to read
  first** — veraPDF's machine-readable **rule definitions** are **CC BY 4.0,
  not GPL**, softer than assumed and therefore most likely to be waved through;
  CC BY 4.0 is not a software licence and carries perpetual attribution.
- **`(bl)` OCR model weights** — whether a **CC-BY-SA-4.0** model file may ship
  inside pdfce's **MIT** portable folder. *Default if unanswered: ship neither
  model set.*

---

## §F — OUTBOUND, AWAITING A REPLY

`D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\reply_the_clipboard_route_is_the_same_loss_and_bookmarks_now_move.md`
(2026-08-29) answers `pdfceGUI`'s clipboard-fidelity question **by
measurement**:

`copy_annotations` → `ObjectClip` → `paste_objects` is the **same loss** they
already have — `ClipAnnotation::Markup` is literally `Box<MarkupSpec>`, and
`paste_objects` calls **plain `add_markup`**, so the clip route additionally
drops the **opacity** they just wired. And `ObjectClip::to_bytes` **does not
serialise annotations at all**, so a persisted clipboard would arrive empty of
markup. **They were told to keep their current path.**

A lossless markup copy (`/T`, `/M`, note, `/CA`, `/Popup`, `/IRT`, `/RC`) needs
`MarkupSpec` extended **and** `/IRT` reference rewriting — **a graph problem,
not a value problem**: copying a reply without its parent dangles, copying both
must rewrite the reference to the pasted parent's new id. Filed to Backlog,
unscoped. **Do not build it speculatively — they were asked to say whether they
need it.**

**Check BOTH channels every session** — `pdfce_FeatureRequests` and
`iccce_FeatureRequests`. They live outside the repo, so **no gate can
contradict a stale "it's empty" claim.**
