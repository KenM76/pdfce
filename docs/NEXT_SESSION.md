# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. Overwrite it once acted on.

**Written 2026-08-20 (evening)**, replacing the 2026-08-20 (morning) handoff
whose headline task — `Pass 119.0`, text editing inside form XObjects — **shipped
this session**, along with `Pass 119.2`, `121.0`, `121.1`, `113.x` and `120.x`,
none of which were the brief.

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
| `97ed7fa` | **`Pass 121.0`** — a code was reported as colliding with **itself**, falsely refusing his drawing |
| `bab0a23` | **`Pass 121.1`** — one edit moved **1,676 labels**; reflow's "line" had no end |
| `e5be7d5` | **`Pass 113.0`/`113.1`/`113.2`** — `transform_objects`, on **every** object kind (§2.5) |
| `73fa218` | **`Pass 120.0`/`120.1`/`120.3`** — the object clipboard: copy, cut, paste, serialise (§2.6) |
| `af5989f` | **`Pass 120.2`/`120.4`** — the clipboard's last two parts, **and a third real-file defect** (§2.6) |
| `2ddbbbe` | CI ran **six of fifteen** gates; it runs **fourteen** now (§4) |

Plus two librarian filings (209th, 210th), `decision 076`, and Backlog
`119.1` / `119.3` / `119.4` / `120.0`–`120.4`.

### ★★ THE TWO DEFECTS ONLY THE REAL FILE COULD FIND — read §6 first

`119.0` and `119.2` were green on 20 tests and a fixture corpus. Then I ran
them on **the operator's actual benchmark CAD drawing** (see §7 for the exact
path, which is written there ONCE and nowhere else) and found two defects in a
row, **neither of them in the code I had just written**:

1. **`Pass 121.0`** — it refused outright, with *"codes 361 and 361 both map
   to 'Ʃ'"*. **The same number twice.** A code present in both a `bfchar` and
   a `bfrange` was materialised twice (because `lookup` consults the singles
   first), and the injectivity check reported a code colliding with **itself**.
   A **false refusal** on a perfectly invertible font. That sentence had been
   shippable since `R110` landed, because *a refusal message is the one string
   a developer never expects to see*.
2. **`Pass 121.1`** — with that fixed it edited, and reported
   **`followers_repositioned=1676`**. Render diff: **34,059 changed pixels
   across the whole page** versus **42 pixels in a 20×7 box** after the fix.
   Reflow shifted every following `Tm` until a `Td`-family boundary, and a CAD
   stream positions everything with `Tm` and **never emits `Td`** — so there
   was no boundary and one edit slid the rest of the drawing sideways. **The
   bug is `Pass 14.1`-era.** `119.0` is simply what first let the surgery reach
   a stream shaped that way.

**The transferable half:** a reach extension does not only find new text, **it
finds old assumptions**. Both defects were in code years older than the Pass
that exposed them, and no fixture in the corpus has the shape that triggers
either. **Run the new capability on his real file before calling it shipped.**

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

**★ The operator's standing instruction this session was
*"fix the housekeeping items first before moving on to the color compositor.
loop until done."*** The housekeeping is done: the clipboard family is closed,
CI runs fourteen of fifteen gates, and the iccce note is read and answered.
**`Pass 97.x`, the compositor, is next** — and §2.7 changes how it must be
scoped, so read that before anything else.

1. ~~**`Pass 113.0`** — `transform_objects`.~~ **SHIPPED this session**
   (`e5be7d5`, with `113.1` preview and `113.2` CLI). See §2.5 for what it
   decided, including the one thing a consumer must not get wrong.

2. ~~**`Pass 120.0`–`120.4`** — the object clipboard.~~ **THE WHOLE FAMILY
   SHIPPED** (`73fa218`, `af5989f`). See §2.6, including the third real-file
   defect it turned up. Only *serialising annotations* remains, filed on its
   own.

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

7. ~~**The `iccce` channel** — `note_gray_black_routing_is_yours.md`~~ **READ
   2026-08-21, and answered.** See §2.7 — it changes how `Pass 97.x` must be
   scoped, and the reply is
   `open/2026-08-21-reply-gray-routing-we-never-noticed-and-here-is-why.md`.

---

## §2.7 — ★★ WHAT THE iccce NOTE CHANGES ABOUT THE COMPOSITOR — read before scoping `Pass 97.x`

**Their question to us was: does pdfce route `DeviceGray` → CMYK per ISO
32000-2 cl. 10.3.2 (inside the colour-managed branch) or per cl. 10.4.2.1 (the
less-capable processor's fallback)?**

**Answered by measurement, not memory: NEITHER, because pdfce has no
`DeviceGray` → CMYK conversion at all.** Every device space converts to
**sRGB** — `gray_to_srgb`, `rgb_to_srgb`, `cmyk_to_srgb`, and there is no
fourth function. The render target is a `tiny_skia::Pixmap` (RGBA8) and there
is **no CMYK-native output path anywhere in the workspace.**

★ **So the routing decision belongs INSIDE the compositor Pass**, because
`97.x` is the first thing that gives pdfce a CMYK-native destination. On the
day it lands, the clause stops being a question and becomes a line somebody
writes. **Make it deliberately; do not let it arrive as a side effect.**

Three things from that note are now compositor input:

- **§8.6.5.7 NOTE 2** — a 4→3→4 conversion *"is unnecessary and results in a
  loss of fidelity in the black component"*. That is the standard's own warrant
  for not round-tripping `DeviceCMYK` through a PCS, i.e. **a conformance
  argument rather than a taste one**, which the compositor plan lacked.
- **The corrected GWG 23.0 panel values**: `DeviceGray` **25 %**, `DeviceCMYK`
  **0/0/0/75**, `Separation` **75**, `DeviceN` **75**. The note's *earlier*
  figures (50 % / 0/0/0/50) were wrong and were wrong **in our inbox**. Do not
  build a fixture from anything but these.
- **★ `1 − 0.25 = 0.75`** — GWG authored the patch on the device-space rule
  itself, so the artwork is independent evidence for the whole conclusion,
  arriving from a direction neither project used to reach it.

**A `pdfce-spec-librarian` dispatch is in flight** to ingest **clause 10**,
which our corpus does not hold *at all* — a gap iccce found from outside it —
plus §8.6.5.6/§8.6.5.7. It was asked to **evaluate** iccce's A52 resolution
rather than adopt it, because two projects agreeing because one relayed it to
the other is not corroboration. **Check its result before scoping `97.x`.**

★ **A licence constraint that binds anyone touching this**: the ISO 32000-2
source in `_sources/` is a **single-user PDF Association copy licensed to the
operator**, watermarked *"copying and networking prohibited"*. Short quotation
with citation is fine; **no bulk transcription into any repository** — both
this one and `iccce` are MIT and public.

---

## §2.6 — WHAT `Pass 120.x` FOUND, and the two parts still open

**★★ The claim the Backlog entry flagged for verification is CONFIRMED AND
INSUFFICIENT**, and that distinction is the scoping outcome. `pdfceGUI`
believed `import_object` already did the hard half. It does — and it is the
*smaller* half. `import_object` copies **indirect objects**; a page's content
objects are **byte ranges inside a content stream** whose operators name their
resources **by page-local name**. On the destination page `/F1` is a different
font, so pasting verbatim draws the right glyphs in the wrong typeface — **and
nothing errors**, because a resource name is not a reference and there is
nothing in the graph to remap.

So the feature is name rebinding: copy records the names each item consumes and
carries the objects behind them; paste re-binds each to a fresh name and
**rewrites the names inside the copied bytes**, driven by a 10-operator /
7-category table.

**The clip owns its resources** (transitive closure by value, payloads as bytes
not spans), which is why cross-document paste is the same code path and why
`to_bytes` shipped the same session rather than forcing that structure to be
invented later.

**`Pass 120.2` and `120.4` shipped 2026-08-21 (`af5989f`) — the family is
CLOSED.**

- **`120.2`** — `ObjectClip::to_pdf()`. The page **is** the selection. Still a
  *different* format from `to_bytes`, and **a test now enforces it**:
  `from_bytes` refuses a PDF by name, so a future "simplify these into one"
  cannot pass quietly.
- **`120.4`** — annotations got their own address space (`copy_annotations`,
  `copy_selection`). ★ **The filed "refuse loudly" framing was aimed at a hole
  that did not exist** — content indices could not *name* an annotation to
  refuse it. Once the address space existed, markup and ce dimensions turned
  out **paste-able**, copied through pdfce's own models (a ce dimension carries
  its group's **name and unit**, matched or created on paste). **Widgets are
  refused by name**: a renamed field is a *different* field.
- **Still open, filed as its own item:** serialising annotations into a clip
  payload. It needs a second versioned encoding beside the content one.

### ★★★ AND HIS REAL DRAWING FOUND A THIRD DEFECT — read this before the next reach extension

Exporting a selection from the operator's CAD file produced a PDF whose text
**pdfce's own extractor** read as `chars=0 codes=4 failed=4`:

> *a show operator appeared with no font selected (§9.4.1 requires Tf first)*

**Text state is graphics state (§8.4.1 Table 52).** A producer may set
`/F8 12 Tf` **once** and emit many `BT`…`ET` blocks that inherit it — which is
what a CAD exporter does. A text object's byte span is its `BT`…`ET`, so **the
`Tf` is not in it.** Copy bound no font, paste emitted fontless text, and
**nothing errored at either end.** Same argument for a path stroked after
`0.5 g 2 w`: it would have pasted black and hairline.

Fixed with a **prelude** — the inherited state, captured at copy time, emitted
inside the paste wrapper. `ClipItem::bytes` stays verbatim so *what the
producer wrote* and *what pdfce re-established* stay distinguishable.

**★ THREE CONSECUTIVE PASSES, THREE DEFECTS, NONE FOUND BY A FIXTURE.** The
false font refusal, the reflow that moved 1,676 labels, and this. All three
were in code older than the Pass that exposed them. **Run it on his file
before calling anything shipped** — it is now the last step, not an optional
one.

---

## §2.5 — WHAT `Pass 113.x` DECIDED, and the one thing not to get wrong

**The open question is closed by IMPOSSIBILITY, not by preference.** `q…cm…Q`
wrapping beat operand rewriting because operand rewriting *cannot* do the job:
a rotated rectangle has no `re` spelling, `line_width` is a user-space scalar a
coordinate scale leaves behind, and text and images carry no coordinate
operands at all. Wrapping never looks at an operand, so **kind-agnosticism is a
property of the mechanism** rather than a match arm somebody must remember to
extend.

**★★ The matrix emitted is NOT the matrix passed in.** `matrix` is page space
(that is where the operator gestures); `cm` composes into the CTM in force at
that point, which is the object's *user* space. pdfce emits
**`X = CTM × M × CTM⁻¹`** from each object's own captured CTM. Emitting the
request directly is correct only where an object's CTM is the identity and
**silently wrong everywhere else** — the object lands twice as far as the
pointer went, with nothing erroring. Sabotage-verified: removing the
compensation fails exactly one of sixteen tests, the one written for it. **If
you touch this code, run that test first.**

**`TextObject` gained a `ctm` field.** Paths and images always had one; text did
not, because until now no verb needed it.

**Both `R206` options ship** — mixed selections **transform whole** (option:
refuse, naming both kinds); a singular matrix is **refused by name** (option:
`Clamp { min }`, which clamps and discloses; a *sheared* singular matrix
refuses the clamp, because its degeneracy is a direction rather than an axis).
**This Pass is `R206`'s founding instance and discharges it.**

**★ A negative scale is not singular** — a mirror is invertible — and there is
a test whose only job is to stop somebody "fixing" this by refusing
non-positive scales, which would break mirroring while passing every other
test.

**One measured number a consumer will hit:** the operator's CAD page takes
**~4 s to decompose in a debug build**, and *both* the verb and the preview
decompose. `pdfceGUI` was told to call the preview on selection change, not per
frame.

**Still unstarted from that request:** `Pass 114.0`–`117.0` — annotation
`/NoZoom`/`/NoRotate` placement, markup and redaction-mark transforms,
per-variant ce-dimension rotate/scale, widget resize.

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

### ★★ AND THE GATES THEMSELVES WERE ONLY HALF-WIRED — fixed 2026-08-21 (`2ddbbbe`)

`tools/` held **fifteen** check scripts and CI invoked **six**. The other nine
ran only when somebody remembered to run them locally — which is exactly the
"green because nobody looked" failure each of those scripts was written to end,
one level up.

**The ratio had been mis-filed twice before anyone counted it**: one session
recorded "3 of 16", the next corrected it to "5 of 14", and both were derived
by *reading* rather than by counting. `ls tools/check-*` plus one grep over
`ci.yml` are the only two commands that answer it.

Fourteen are wired now. The fifteenth, `check-image-colorspace-truth.py`, is
**deliberately** out — it takes a fixture directory and checks a corpus, not
the repo, so it is a sweep tool rather than a gate. Named in `ci.yml` so the
next person counting does not record it as a ninth miss.

They went into the `ui-strings` job on purpose: it already checks out with
`fetch-depth: 0`, which `passes-filed` needs to walk history, and **a shallow
clone would have made it pass vacuously.**

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

## §6 — TRY IT (and this is where the two 121.x defects came from)

**Run it on a real drawing before believing a green test run.** Both defects
above were invisible to 20 passing tests and a fixture corpus, and both
surfaced within two commands of pointing the binary at the file named in §7.

```
pdfce-cli inspect --forms  <a CAD drawing>
pdfce-cli edit-text        <in> --find "OLD" --replace "NEW" -o <out>
pdfce-cli format-text      <in> --find "OLD" --set-size 14   -o <out>
```

`inspect --forms` is the one to run first on a real drawing — the `paints`
column is the fan-out, and it is the number to look at before any batch edit.
On his drawing it reports `/Fm3 object=23 depth=0 paints=1` — object 23, which
is the exact form the original request measured at 1,696 show operators.

**And check `followers_repositioned` on the way past.** It is now the cheapest
tell that a reflow has over-reached: on absolutely-placed CAD content it should
be `0`, and a large number means the edited "line" ran further than the line.

---

## §7 — THE BENCHMARK FILE, written once

The operator's own CAD drawing, and the input that found both `121.x` defects:

    D:\Dev\temp\pdfce\ncored-benchmark-cad-drawing.pdf

★ **Written here once, on its own line, and referenced from everywhere else in
this document rather than repeated** — a Windows path in prose is the single
most reliably-mangled string this project handles. Both earlier copies in this
file arrived as `D:\Dev<TAB>emp\pdfce<NEWLINE>cored-...`, because a scripted
patch turned `\t` and `\n` into the characters they escape. The engineer's
agent memory carries the rule (*any backslash breaks heredoc patching — write
the file, or use `Edit`*), and this is its second instance in one session.
