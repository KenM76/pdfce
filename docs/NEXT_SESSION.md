# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**: no
*"this paragraph read X until…"*. What is true now, plus a pointer.
Corrections and their prior wording live in the **append-only** record —
`ROADMAP.md` and `SESSION_LOG.md` — where a claim is dated and no later edit
can falsify it.

---

## §A — COLD START

**The previous session shipped two Passes, both print-colour, both found by
running the print-conformance suite rather than by reading code:**

- **`Pass 130.2`** (`fafc0c2`) — **an overprinting image ignored overprint
  while a rectangle of the same colour did not.** §11.7.4.3 now runs for a
  sampled image. Suite: 7 failures → 4.
- **`Pass 130.3`** (`cd4de8d`) — **a spot colour painted nothing at all on a
  print-bound page.** One function was answering two questions; see `§D`.

### ★ Verified from a shell at write time — re-run, do not copy forward

| fact | value | command |
|---|---|---|
| `HEAD` | `3e70cdb` | `git rev-parse --short HEAD` |
| `git describe --tags` | `v0.14.0-9-g3e70cdb` | `git describe --tags` |
| `origin/main` | ★ **4 commits BEHIND `main` — this work is NOT PUSHED** | `git rev-list --count origin/main..main` |
| tag at `HEAD` | none; highest is `v0.14.0` | `git tag --points-at HEAD` |
| working tree | **clean** | `git status --porcelain` |
| newest backup bundle | `pdfce-20260827-0049-3e70cdb-full.bundle`, verified `okay`, **contains everything** | `ls -lt D:\Dev\pdfce-backups\` |
| gates on disk | **18**; 17 run with no arguments | `ls tools/check-*` |
| CI at `HEAD` | **UNKNOWN AND UNKNOWABLE — `HEAD` HAS NEVER BEEN PUSHED.** There is no run to read | — |
| `docs/core-api` verbs | **144** | `python tools/check-core-api-verbs.py` |
| `crates/pdfce-core/src/edit.rs` | **31,655** lines | `grep -c "" …` |

★★ **`main` is 4 commits ahead of `origin` and was deliberately NOT pushed** —
`CLAUDE.md` rule 8 wants a current go-ahead and none was given. Everything is
bundled, so nothing is at risk; the operator simply has not said "push". **Ask
before you do.**

---

## §B — ★★ THE FIRST THING, AND IT IS NOT CODE: THE DISK IS FULL

**`D:` is at 99–100%.** 954 GB total, **~7 GB free**, and it went under 3 GB
mid-session.

**It is not a theoretical problem — it corrupted a build.** A
`cargo test --workspace` ran out of space part-way, left truncated artefacts,
and the *next* run failed with `can't find crate for 'std'` and
`could not exec the linker` — errors that read like a broken toolchain or
broken code, and are neither. Half an hour went into it.

What was done, all of it regenerable, none of it operator data:

```
rm -rf target/debug target/debug/incremental fuzz/target \
       target/wasm32-unknown-unknown target/aarch64-apple-darwin \
       target/x86_64-unknown-linux-gnu
```

and the test sweep was then run with **`CARGO_PROFILE_DEV_DEBUG=false`**, which
is the trick worth carrying: debug info is most of the bulk, and the workspace's
debug tree is ~23 GB with it and a fraction of that without. `cargo test`
otherwise does not fit.

**`D:\Dev\pdfce\target` is ~12 GB of the total.** The rest is not pdfce's and is
not this role's to touch. **This is an operator item — surface it.**

---

## §C — WHAT TO DO NEXT: TWO NEW REQUESTS, AND ONE IS OPERATOR-BLOCKING

★ **Both arrived at 22:58 on 2026-08-26, i.e. AFTER the pre-flight `ls` earlier
that evening.** The channel is checked at the start of a session and things land
during one. Check it again at the end.

### 1. ★★★ `request_decompose_recurses_into_form_xobjects.md` — TAKE THIS FIRST

**The operator cannot click on objects to edit them.** His words, quoted in the
request: *"when I click on one of the objects all I get is the page selected."*
He is right, and he is selecting a real object — a page-sized **form XObject**.

`pdfceGUI` ran a fourteen-agent audit and concluded this is **the single item
it cannot work around**. It is `pdfce-core` work, not GUI work.

**Both line-level claims were verified against the tree this session, so you can
start from them rather than re-deriving:**

- `crates/pdfce-core/src/vector/decompose.rs` — the `XObjectShape::Form` arm
  calls `emit_image(ImageSource::Form, …)` with the form's `/BBox` corners and
  **returns without entering the form's content stream**.
- `crates/pdfce-core/src/vector/hit.rs` — `object_hit`'s `VectorObject::Image`
  arm is `i.page_bbox.inflate(tolerance).contains(point)`, a plain rectangle.

⇒ a page-sized form is a page-sized hit target above everything drawn before
it, and it wins every click at every point.

**The request is unusually good and has already done work for you**: it inventories
what recursion costs *on the GUI side* (index renumbering, count explosion, its
own truncating `.find()`) and explicitly takes those as theirs. It asks for
**read access to the leaves only** — not editing through the recursion, not a
`PageText` change, and it explicitly asks that the *editing-through-recursion*
question be **answered as a decision before anything is built on it**.

★★ **THE DESIGN IS ALREADY SETTLED AND ANSWERED ON THEIR CHANNEL** —
`open/reply_decompose_form_recursion_the_editing_question_is_already_ruled.md`.
Do not re-litigate it; build it. In short:

1. **Recursion is the model, not a flag.** A flag means two walks over the same
   content that can disagree.
2. **★ Their object indices do NOT renumber.** One walk, **two projections**:
   the existing flat list keeps its exact present meaning (page-stream objects,
   same order, same indices), and the leaves are an *additional* view. They
   listed renumbering first among what recursion costs them; they do not have
   to pay it.
3. **★★★ AND THAT IS A SAFETY PROPERTY, NOT A COURTESY — it is the finding
   that decides the design.** Thirteen CLI verbs take a paint-order object
   index and **eleven core sites resolve one**, and every one writes to the
   **page's** stream. A leaf inside a form carries a token range into the
   **form's** stream. Put leaves in the same list and those verbs will apply a
   form-relative range to the page and corrupt it, silently. ⇒ **the recursion
   is not a read-only change**, which is the one thing the request did not say.
   Keeping leaves out of the list those eleven sites read makes them
   unreachable rather than guarded — and "add a guard to eleven sites" is
   exactly the shape that produced `Pass 130.3`.
4. **The form container stays in the flat list**, so "select the container"
   remains a distinct act; part 3 (the form's bbox not answering a first click)
   lands in the **hit test**, which consumes the leaf view.
5. **Read access only**, as they asked.

★★★ **AND MOST OF THE HARD WORK ALREADY EXISTS IN `pdfce-core` — GREP BEFORE
YOU BUILD.** `text_extract` has recursed into form XObjects since `Pass 1.1`
(`text_extract/page.rs`, around the `Subtype /Form` arm) and already solves
every difficult part: an **object-number-keyed** cycle guard (a name-keyed one
misses the cycle, because one stream is reachable under two names), a depth
bound with a diagnostic, resource inheritance with fallback to the invoker's,
and the `view.slice(span)` subtlety for a form the **session** authored.

It also already carries the two types the vector model needs and I was about to
invent:

- **`ContentStreamRef::{Page, Form { object }}`** — "which buffer does this
  span index?"
- **`TextRun::is_editable()`** — "can this be edited through the page-stream
  path?", which the GUI **already binds for text**.

⇒ a vector leaf must carry a `ContentStreamRef` and answer `is_editable()` the
same way a `TextRun` does, so a form-interior path and a form-interior text run
describe themselves **identically**. The GUI has to reconcile both in one
selection; two vocabularies for one fact would be its problem and our fault.

**Depth/cycle constants:** the renderer's `MAX_XOBJECT_DEPTH = 64` is
corpus-corrected (veraPDF ships a *conformant* 32-deep chain) and
`text_extract` hand-duplicates it as `max_form_depth`. That is already two
copies; **do not make a third.** Consolidating them into one is a small,
worthwhile part of this Pass.

### ★ The editing question they asked to be ruled on — ALREADY ANSWERED, and a
### correction came out of answering it

They asked: a leaf inside a form invoked in several places — edit in place,
clone the form, or refuse? **Decision 076 already ruled it for text
(edit-in-place, disclosed), and its decisive reason is not text-specific**: a
form invoked from *inside another form* cannot be re-bound without editing the
parent, which may itself be shared. Extended to vector by reference; not
re-decided.

★★ **Answering it exposed a false claim in decision 076 itself**, now corrected
(`6787e7b`): it certified `R206` compliance on the basis that *"Both are
shipped — `Pass 119.0` and `Pass 119.1` (`unshare_form`)"*. **`unshare_form`
has never existed** — grepped under six plausible names, zero hits — and
`Pass 119.1` is still Backlog. So `R206`'s "ship both" is **outstanding**, and
there is currently **no verb that lets an operator break a form's sharing**.
The GUI has been told not to offer one.

### 2. `request_ocr_as_an_edit_to_the_open_session.md`

`add_ocr_layer` takes `&Document` and returns a whole new PDF, so OCR cannot be
an undoable edit to the open document. Operator: *"Why do I have to save a copy
instead of just go back into my pdf and save over it?"* The request surveys six
tools and reports **zero of six** force a Save-As on the open-document path.

Smaller than the first and unblocked by it.

### 3. Still unanswered from earlier

`request_extraction_drops_the_writing_direction.md` (root, 2026-08-26 16:29),
plus four under `open/`: `adopt_widget` pre-flight, markup-opacity-in-two-verbs,
`insert_pages` orphaned widgets, restyle-an-existing-text-run. On the `iccce`
channel, `note_your_name_gate_has_the_two_defects_mine_had.md` is unanswered and
is about a gate this project also owns.

---

## §D — WHAT THE TWO PASSES DECIDED, IF YOU TOUCH THIS GROUND

**No rule and no decision was minted.** Ceiling: rules **`R218`** (next free
`R219`), decisions **`089`** (next free `090`). Confirm with
`python tools/check-ledger-numbers.py`.

1. **★★★ ONE FUNCTION WAS ANSWERING TWO QUESTIONS, AND BOTH ANSWERS WERE
   RIGHT.** `overprint::authored_tints` answers *"which **process** tints did
   this source state?"* — Table 149's question. A spot names none, so it
   answers `[0,0,0,0]`, **correctly**. `authored_cmyk` handed that same answer
   to the compositor as the paint's **colour**, where zero ink is blank paper.
   Neither function was wrong. ⇒ *When two questions have the same type
   signature, one function will eventually answer both, and the failure is
   invisible at both ends.*

2. **★★★ A FAILURE COUNT THAT RISES WHEN THE RENDERER IMPROVES IS THE
   SIGNATURE OF A FALSE PASS BEING REMOVED.** `Pass 130.3` took the suite from
   4 failures to **5** while moving the two patches concerned **closer** to
   Acrobat (mean abs distance 24.8→19.9 and 41.4→28.5). They paint CMYK over a
   spot backdrop; while the spot was invisible there was nothing to wrongly
   knock out, and a white trap cross on white paper has no contrast to detect.
   **They were passing because they were rendering nothing** — five of six
   cells on one were blank.

3. **★★ TWO "OBVIOUS FIXES" ARE REFUTED, NOT UNTRIED.** For a spot-only source
   under `/OP true`, preserving the whole backdrop looks like a defect. It is
   not:

   | behaviour | suite failures |
   |---|---|
   | preserve the backdrop — shipped | **4** |
   | ink union, `max(c_b, c_s)` | 6 |
   | paint the flattened tint normally | 8 |

   Three patches exist whose whole subject is that a white spot set to
   overprint must not knock out what is under it. Recorded on
   `overprint::erases_the_paint`; the refuted `ComponentRule::MoreInk` was
   **removed** rather than left callable-and-uncalled. **Do not re-attempt
   without a stronger oracle.**

4. **★★ A SHADING-SITE REFUSAL WAS ALSO BUILT AND REVERTED.** It made a missing
   451×29 spot gradient bar appear and **took the check marks drawn under it
   away** — which is the criterion that patch actually states. The suite score
   did not move either way, because that patch is `mark?` and the harness
   cannot score it. ⇒ *A harness reporting "no change" can be blind rather than
   reassuring.*

5. **★ RECORDED GROUND TRUTH WAS WRONG AND IS CORRECTED** (`3e70cdb`).
   `suite-check.py`'s docstring said pdfce drew none of two patches' check
   marks; it draws **all** of them. What it misses is the *bar behind* them.
   The error's shape is the lesson: the original measurement asked the right
   question of a whole-page diff **dominated by the missing bar**, and "this
   region differs and the marks are in it" was read as "the marks are missing".
   **A large real difference swallowed a smaller question about the same
   pixels.**

---

## §E — THE PRINT SUITE, AND HOW TO RUN IT PROPERLY

**Corpus:** `D:\Dev\temp\suite-patches` (51 PDFs). **Reference renders:**
`D:\Dev\temp\acro-refs` (51 PNGs). The private map outside this repository holds
the id↔filename mapping and the two environment variables.

★ **Its `manifest.json` and `README.md` pointed at a pre-scrub directory name
that no longer exists** — a session following the map got "corpus not found"
and would reasonably have concluded the corpus was gone. **Repaired
2026-08-26.**

★★ **ALWAYS PASS `--reference-dir`.** It existed and was going unused. Measured
on the same tree, same binary, only the flag differing:

```
without   5 FAIL, 30 pass, 16 UNRESOLVED
with      5 FAIL, 35 pass, 11 UNRESOLVED
```

It is also the only way to run **the control that separates a pdfce defect from
an instrument artefact**: point the trap detector at the reference engine's own
render of a failing patch. Of the four failures on 2026-08-26, three tripped
**zero** traps there and one tripped **two** — so that patch's 12 is not 12
pdfce defects, and nobody could have known without running it.

```
python tools/suite-check.py D:\Dev\temp\suite-patches --reference-dir D:\Dev\temp\acro-refs
```

**The 5 remaining failures, and who owns them:**

| patch | cause | owner |
|---|---|---|
| `PCS2_020`, `PCS2_040`, `PCS2_081` | the spot-plane gap | **pdfce** — the per-colorant buffer |
| `PCS3_130`, `PCS3_161` | ICC transforms | ★ **`iccce`, decision 064.** Do **not** file these as pdfce work and do not build a CMM here |

⇒ **Every remaining pdfce-owned failure in the suite is now the same one
thing: there is no plane for a spot colorant.** That is the per-colorant buffer,
and it is the whole of what is left. ★ The cheap page-sized spot-ink multiplier
was built, ablated and reverted in an earlier session; §D item 3 refutes two
more shortcuts. **The next attempt should be the real buffer or nothing.**

---

## §F — THE REST OF THE QUEUE

1. **`Pass 122.7`** — the undiagnosed blue-channel residual. Green matches
   Acrobat to ~1 level, blue does not (55.7 vs 2.6, down from 209.2). A large
   improvement, not a completed one, and nobody knows why.
2. **`Pass 127.2`** — `redact-mark`'s stdout carries no diagnostics field;
   `find-text` prints unreadable-font counts to stdout and `redact-mark` only to
   stderr, so a batch caller parsing stdout cannot tell a clean run from an
   unreadable one.
3. **`Pass 122.3`** — band a large page render. Two of three acceptance clauses
   were discharged by `Pass 132.0`; **banding is the third and untouched.**
4. **CLI surface for the four ce-dimension group-management verbs** — rename,
   delete, delete-with-policy, re-parent. Core ships all four, no subcommand
   reaches any (`R151` shape).
5. **The sibling-gate check.** Carried forward and **still owed**:
   `check-string-gaps.sh` was widened after proving blind to a defect
   `rustfmt` could not fold onto one line. Do `check-ui-strings.sh` and
   `check-theme-colors.sh` carry the same post-formatting-shape assumption?
   ★★ Those two are the line-scanners that **exist**; earlier handoffs named a
   `check-strong-text.sh` that never has.
6. **An absent-mark detector.** Four patches are scored by a check mark that
   should be **present** and the harness has no detector for an absence, so it
   reports `MARK?`. A first cut keyed on "reference has ink where pdfce has
   paper" was prototyped and **fails**, because the marks sit *on images*, not
   on paper. Whatever is built must key on the mark's own pixels relative to a
   reference render.

**Two `D:/dev/rag/` escalations remain OWED at n = 1, both with a named
second-occurrence trigger** — *in-crate tests are privileged in ways a consumer
is not*, and *an infallible allocation is a bound's silent partner*. Unchanged
this session.

---

## §G — PRE-FLIGHT, EVERY SESSION

**1. `ls` BOTH FeatureRequests channels, ROOT and `open/`, as TWO commands.**
They are outside this repository, so **no gate will ever contradict a stale
sentence about them — including this one.** ★ And check again at the END: two
requests landed mid-session this time.

```
D:\Dev\FeatureRequests\pdfce_FeatureRequests\
D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\
D:\Dev\FeatureRequests\iccce_FeatureRequests\open\
```

**2. Run the gates — `ls tools/check-*`, do not trust any list**, including
`§A`'s count.

★★ **AND READ EACH GATE'S OWN EXIT CODE.** A sweep written this session as
`r=$(python "$g" | tail -2); ec=$?` captured **`tail`'s** status, not the
gate's, and printed `[0]` beside a gate that was **red**. It was caught only
because that gate also prints its verdict in words. `R209` says the set somebody
runs is not the set CI runs; this adds: **the exit code somebody reads is not
always the exit code that was returned.**

**3. ★★ RUN `cargo +nightly fuzz build` ON WINDOWS.** CI cannot do it for you
and `cargo check --bins` is not a substitute — CI's fuzz job runs on Linux and
was green through a window in which the harness was completely unbuildable
here, and the documented local stand-in passes in both states because it never
links.

**4. Read `docs/ARCHITECTURE.md` §12** — cross-project boundaries live there and
no gate can catch a violation. **Decision 064 is the one that bites right now:
ICC colour conversion is `iccce`'s, not pdfce's.**

**5. Read `docs/compositor-plan.md`** before scoping anything in `97.x` or the
per-colorant buffer.

---

## §H — THE SUBMIT / SCRIPTING PLAN IS STILL PARKED

Full detail: `docs/plan-scripting-submit-and-plugins.md`. Eight operator rulings
quoted verbatim there; do not paraphrase them from this file. Nothing this
session touched it.

- A push button that does anything was blocked by **`R54`**, not by the
  JavaScript rule; the operator ruled *"change the rule"* and decision `088`
  amends it to a dispatch allow-list.
- **The JavaScript half is deferred by the operator** (*"defer for now"*).
  `R53` stands. Phases 1–3 must each work with **no scripting engine present at
  all**.
- The plugin boundary is a **versioned message format, not a binary**.
- Submission is permitted, destination open by default, always disclosed.

**Still owed before any submit code:** a decision record for `R12`'s new
destination class, the `R13` clause 5 ruling (an add-in is executed code and
`R13`'s *"never executes anything it fetched"* was **not** narrowed by decision
061 — **no add-in Pass can be scoped until the operator rules**), the transport
question, and a `pdfce-ui-specialist` dispatch before any GUI surface.
