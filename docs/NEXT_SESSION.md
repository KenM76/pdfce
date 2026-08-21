# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. Overwrite it once acted on.

**Written 2026-08-20 (evening)**, replacing the 2026-08-20 (morning) handoff
whose headline task — `Pass 119.0`, text editing inside form XObjects — **shipped
this session**, along with `Pass 119.2` which was not asked for.

---

## §0 — DO THESE TWO THINGS BEFORE ANYTHING ELSE

**1. `ls` BOTH FeatureRequests channels.** They are outside this repository, so
**no gate will ever contradict a stale sentence about them** — including this
one.

```
D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\
D:\Dev\FeatureRequests\iccce_FeatureRequests\open\
```

`R196` exists because a handoff said the pdfce channel was empty and it was not.
**This session found `request_an_object_clipboard_the_whole_capability_not_the_convenient_subset.md`
(2026-08-20 16:57) that way** — it had landed *after* the previous handoff was
written, was in no document, and `grep ROADMAP.md` for "clipboard" returned
**zero**. It is now filed as `Pass 120.0`–`120.4`.

**2. Run the gates — `ls tools/check-*`, do not trust any list.** There are 15
today. The previous handoff hard-coded 14 and warned, correctly, that its own
list was already the thing it was warning about.

All green at this handoff.

---

## §0.5 — WHAT SHIPPED, AND THE ONE THING IT LEAVES ON THE TABLE

| commit | |
|---|---|
| `cc57080` | **`Pass 119.0`** — text inside a form XObject is editable |
| `a10a5c1` | **`Pass 119.2`** — `format_text` reaches it too, + a document repair (§3) |

Plus two librarian filings (209th, 210th), `decision 076`, and Backlog
`119.1` / `119.3` / `120.0`–`120.4`.

**The operator's own framing, which is why this jumped the queue:** *"I need
that editing capability as it is 99% of the text I will want to edit."*
Measured on his benchmark CAD drawing, that is if anything low — the page's own
`/Contents` holds 3,007 single-character `Tj` of producer watermark, and **one
form XObject holds 1,696 show operators** carrying every label, the title block
and every *pdf dimension* callout.

### ★★ THE THING THAT IS NOT DONE, AND IT IS NOT CODE

**`pdfceGUI` still has a caret guard that refuses form text.** Until they delete
it, none of this reaches the operator. The reply telling them so is
`open/2026-08-20-form-xobject-text-is-editable-and-your-caret-guard-is-now-wrong.md`,
and `Editability::InsideForm` is now `#[deprecated]` **specifically** so their
build tells them where the arm is.

**Do not tick `FEATURES.md`'s gui box for either verb until they confirm.** The
librarian was told this explicitly and complied; it is repeated here because the
temptation on reading "shipped" is to tick everything.

---

## §1 — THE QUEUE, in the order I would take it

1. **`Pass 113.0`** — `transform_objects`. It was next before `119.0` jumped it
   and it is next again. `pdfceGUI` says their whole shell side is built and
   waiting. **`Pass 112.0` (the `Matrix` foundation) is shipped.**

   ★ Read the open question first: **operand rewriting cannot express
   rotation**, so this needs a `q…cm…Q` wrap, and that mechanism choice is filed
   as an OPEN QUESTION rather than decided. **Nothing is owed from `pdfceGUI`** —
   Ken answered both design questions himself (*"make things work both ways as
   options. default it to your best guess"*), so both defaults are acceptance
   criteria: mixed selections transform whole, a singular matrix is refused by
   name.

2. **`Pass 120.0`–`120.4`** — the object clipboard. Newest request, operator-
   widened (*"oh I might want all cases so we shouldn't be restrictive in our
   ask"*), and the underlying ask is a fortnight old and repeated: *"can you get
   cut copy and paste working for objects I select on the canvas?"*

   ★ **Verify their central claim before scoping from it.** They believe
   `EditSession::import_object` (`edit.rs:19367`) already does the hard half —
   recursive cross-document graph copy with reference remapping, cycle handling
   and stream re-staging — so the ask is *"expose the one you have at object
   granularity"*. **I did not verify that**, and told them so. They asked to be
   told early if the reading is wrong.

3. **`Pass 119.1`** — `unshare_form` (copy-on-write a shared form onto one
   page). See §2 for why this is a *separate verb* and not a mode of `edit_text`.

4. **`Pass 97.0/97.1/97.2`** — the colorant compositor. Still the highest-impact
   item by Ghent count. Plan of record `docs/compositor-plan.md`. ★ Its "16 of
   the 18 remaining Ghent failures" thesis is **AMENDED and owes a
   re-derivation** before the Pass is scoped from it. This is also where the
   **iccce dependency edge appears**, so `Pass 101.1` unblocks at the same
   moment. **Ghent standing has not been re-measured since 2026-08-19**
   (26 pass / 14 FAIL / 11 UNRESOLVED of 51) — re-measure rather than quoting.

5. **`Pass 80.0`** (note text on markup) and **`Pass 81.1`** (markup opacity,
   write half) — both `pdfceGUI` requests, both already scoped.

6. **`Pass 119.3`** — align `pdfce-render`'s nested-form resource fallback with
   `text_edit::forms`. Small, and see §2 for exactly what diverges.

7. **The `iccce` channel** — **`note_gray_black_routing_is_yours.md` is still
   unread**, and is still the highest-value unread file there. A boundary ruling
   handing pdfce the four-way gray/CMYK/`Separation`/`DeviceN` black
   equivalence, bearing directly on `Pass 97.x`. **Not a request — do not triage
   it as one.**

---

## §2 — WHAT `Pass 119.x` DECIDED, so it is not re-litigated

**Shared forms: edit-in-place, disclosed. `decision 076`.** A form XObject may
legally be painted from several pages (§8.10.1 states it as the *purpose* of the
feature) and **no clause in either ISO edition binds one to a page** (`FX-N1`,
permanent, argued three ways). So an edit changes every sheet, and there is
exactly one stream — pdfce cannot prevent it, only disclose it.

**Copy-on-write was weighed and declined as a default**, for two reasons and the
second is decisive:

1. Text in a shared form is **identical on every sheet by construction**, so
   wanting one sheet different is wanting to *break the sharing* — a distinct
   act, not a mode of "edit text".
2. **CoW is not always expressible.** A form invoked from inside another form
   cannot be re-bound without editing the parent, which may itself be shared. A
   default whose semantics silently depend on nesting structure is worse than one
   that always means the same thing.

**★ Acrobat's behaviour here is unsourceable, and that is a *sourced* result.**
Fifty tool calls across Adobe Community, the Acrobat SDK docs, Enfocus/PitStop,
Apryse and prepressure.com found nobody documenting what Acrobat does to a
shared form's stream — **including Acrobat's own competitors**. Recorded in
`Acrobat_Features\text_edit__form_xobject_shared_content_editing.md`, which also
flags a trap **by name**: a well-indexed forum thread titled almost exactly
*"text edits appear on every page"* is root-caused by AcroForm field-name
collisions, **not** shared form XObjects. Do not re-run that search expecting a
different answer.

**★ Two resource-merge tolerances were built and reverted before shipping.**
Merging a form's resources per *name*, then the weaker per-*category* version.
Both made the edit path resolve a font **`pdfce-render`'s interpreter does not**
(its `Do` handler takes the form's own `/Resources` when present and the
caller's only when absent), so the advance would be computed from `/Widths`
nothing else consults and text would land **visibly wrong while every internal
check reported success**. `text_extract` agrees with the renderer independently.
Settled on whole-dictionary, own-wins-entirely. **If you ever meet a
partially-declared form on real producer output, that measurement reopens the
decision** — there is a `personal_rag/pdf` entry waiting for it.

**The one divergence that survives, stated rather than hidden:** for a *nested*
resource-less form, `text_edit::forms` inherits the **page's** resources (the
clause's actual words) and `pdfce-render` inherits the **caller's** (the common
implementation). Identical at depth 0, which is every real file.
`FormRef::resource_tier` records which tier resolved. `Pass 119.3`.

**Deliberate non-goals, so they read as decisions:** `reflow_block` and
`add_text` still reach page-stream text only. `add_text` is **not merely
unfinished** — appending to a form's content stream changes what *every*
invocation site paints, a different disclosure from an in-place edit's.
`editability()` reports `edit_text`'s reach, so it is **optimistic** for those
two.

---

## §3 — ★★ THE DEFECT I SHIPPED, AND WHAT CAUGHT IT

`cc57080` **duplicated 141 lines of `docs/core-api/02-editing-and-saving.md`** —
the document `pdfceGUI` builds against. Sections 1.4 through 1.9 existed twice,
and **the second copy still carried the stale "refuse a caret on form text"
warning the commit was written to reverse.**

The cause was one missing argument:

```python
end = s.index(TABLE_MARKER)        # searches from 0
# should have been s.index(TABLE_MARKER, start)
```

`"| I want to… | Call | Line | Returns |"` appears in nearly every section, so
`end` resolved ~200 lines *before* `start` and the splice re-appended everything
from §1.3 onward.

**Three things worth carrying, and the second is the one that matters:**

- **The failure mode is silent by construction.** A bad splice end does not
  error, lose content or reorder anything — it **duplicates**, and a duplicated
  Markdown section is invisible to every check this project owns.
  `check-core-api-verbs.py` was **green** (it counts verbs and line-count
  claims; a duplicated section has the same verbs). `cargo test` was green. The
  commit's `+205 lines` looked plausible for a rewrite meant to add a hundred.
- **★ It was caught by a READER, not a check** — `pdfce-librarian`, reading the
  document while filing the Pass it had just been told about. That is the
  argument for the post-commit librarian dispatch being a *read* and not a
  transcription, and it is the second time this month an omission has lived
  precisely in the gap where nothing was looking.
- **Recorded, not promoted.** One occurrence against this project's two-instance
  bar, so no standing rule was minted. It is in the engineer's agent memory
  (`feedback_splice_end_marker_must_be_searched_from_start.md`) and escalated to
  `C:\personal_rag\claude_code\`. **A second instance changes that.**

Prefer the `Edit` tool for structured documents: it fails loudly on a non-unique
`old_string` instead of guessing.

---

## §4 — GATES: the same lesson, from the other side

`tools/check-outcome-disclosed.py` reported **clean** the whole time it could
not see `EditReport`. Its `OUTCOME_STRUCTS` list had been written from `edit.rs`
alone, so **every report type in a submodule was outside it.** Three fields were
added to `EditReport` this session — one of them `form_invocations`, whose only
purpose is to stop a shell changing six sheets while showing one — and the gate
would have stayed green if all three had been dropped on the floor.

**The finding was forecast before the gate ran**: 13 fields, 11 printed ⇒ two
problems, specifically `disposition` and `extra_objects_emptied`. It reported
exactly those two. Both now print. `FormatReport` followed. The gate covers
**100 fields across 14 structs**, up from 58 across 12.

★ **This is the fifth instance of the under-reporting-gate class and the first
whose mechanism is the SCOPE OF ITS INPUT LIST rather than the spelling of a
pattern.** The previous four were all "the regex missed a spelling". Widening a
pattern does not fix a list that was written from one file.

**`tools/check-one-commit-per-command.py` earned its keep the same day** — the
new session-path form loop committed per iteration. Harmless while the loop
returned on first success, and the exact latent shape `import_form_data`
shipped. Hoisted.

---

## §5 — STATE AT HANDOFF

- **Working tree clean apart from agent-memory files.** Two code commits today
  (`cc57080`, `a10a5c1`) plus the librarian's filings. **Nothing pushed.**
  `github.com/KenM76/pdfce` is public and has a remote — a careless
  `git push` reaches the world.
- `cargo test --workspace` green; `cargo fmt --check` clean;
  `clippy --all-targets --workspace` clean.
- `cargo tree -p pdfce-core` / `-p pdfce-render` / `-p pdfce-cli`: **no manifest
  was touched and no dependency was added this session**, so the GUI-free
  property is unchanged by construction.
- **One new wall-clock read in `pdfce-core`, the crate's first.** `cfg`-guarded
  off on `wasm32` (where `SystemTime::now` panics); the fallback is "leave
  `/LastModified` alone", never a fabricated constant. Worth knowing before the
  web fork.
- **`v0.7.0` is bumped but NOT tagged.** Standing operator go-ahead for
  builds/releases since 2026-08-17. Verify CI green on `HEAD`, then
  `verify-release.py` → tag → portable package → GitHub release → librarian
  release record. A CI-built release reports `revision: unknown` unless that
  workflow gets `fetch-depth: 0`.
- **Ledger, re-measure with `tools/check-ledger-numbers.py` rather than trusting
  this line** — it has been wrong before, by three, this same week.
- New spec corpus file: `iso32000__ref__form_xobject_text_edit.md` (68 kB),
  which did not exist before this Pass. It also **corrected the corpus**:
  `iso32000__ref__text_edit_surgery.md` had been silently page-`/Contents`-only,
  so an LLM grepping "text edit surgery" would have got a confidently wrong
  answer.
- New Acrobat corpus file:
  `text_edit__form_xobject_shared_content_editing.md` — a file whose finding is
  an **absence**.

---

## §6 — TRY IT

```
pdfce-cli inspect --forms  <a CAD drawing>
pdfce-cli edit-text        <in> --find "OLD" --replace "NEW" -o <out>
pdfce-cli format-text      <in> --find "OLD" --set-size 14   -o <out>
```

`inspect --forms` is the one to run first on a real drawing — the `paints`
column is the fan-out, and it is the number to look at before any batch edit.
