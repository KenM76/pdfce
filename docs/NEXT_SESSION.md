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

**The previous session shipped SIX Passes: two print-colour, two OCR, two
form-recursion. The print-colour pair were found by running the suite; the
other four came from the operator's own reports on the GUI channel:**

- **`Pass 130.2`** (`fafc0c2`) — **an overprinting image ignored overprint
  while a rectangle of the same colour did not.** §11.7.4.3 now runs for a
  sampled image. Suite: 7 failures → 4.
- **`Pass 130.3`** (`cd4de8d`) — **a spot colour painted nothing at all on a
  print-bound page.** One function was answering two questions; see `§D`.

### ★ Verified from a shell at write time — re-run, do not copy forward

| fact | value | command |
|---|---|---|
| `HEAD` | `0d9f4df` | `git rev-parse --short HEAD` |
| `git describe --tags` | `v0.14.0-27-g0d9f4df` | `git describe --tags` |
| `origin/main` | ★ **22 commits BEHIND `main` — none of this is PUSHED** | `git rev-list --count origin/main..main` |
| tag at `HEAD` | none; highest is `v0.14.0` | `git tag --points-at HEAD` |
| working tree | **clean** | `git status --porcelain` |
| newest backup bundle | `pdfce-20260827-end-0d9f4df-full.bundle`, `git bundle verify` **okay** — covers every commit including this handoff's predecessor | `ls -lt D:\Dev\pdfce-backups\` |
| gates on disk | **19**; 18 run with no arguments | `ls tools/check-*` |
| CI at `HEAD` | **UNKNOWABLE — `HEAD` HAS NEVER BEEN PUSHED.** No run exists to read | — |
| `docs/core-api` verbs | **146** | `python tools/check-core-api-verbs.py` |
| `crates/pdfce-core/src/edit.rs` | **32,159** lines | `grep -c "" …` |

★★ **`main` is 22 commits ahead of `origin` and deliberately NOT pushed.**
`CLAUDE.md` rule 8 wants a current go-ahead and none was given; the operator was
asked twice and the question is still open. Everything is bundled and the bundle
verifies, so nothing is at risk. **Ask before you push.**

★ **A NEW GATE EXISTS: `check-cited-commits-exist.py`.** Every commit hash cited
anywhere under `docs/` must be an ancestor of `HEAD`. It found **fourteen**
pre-existing stale citations, five of them in `docs/core-api/` — the document a
separate project builds against. All repaired. **If you `git commit --amend` or
rebase anything that a document already names, this is what will tell you.**

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

## §C — BOTH OPERATOR REQUESTS ARE SHIPPED. WHAT IS LEFT OF THEM

★ **Do not start either from scratch — read this first.** Both 2026-08-26
requests from `pdfceGUI` are done, answered on their channel
(`open/note_form_recursion_and_ocr_as_an_edit_both_shipped.md`) and filed.

### Shipped

- **Clicking objects works.** `PageObjects::leaves` (`FormLeaf`) +
  `hit_test_point_deep` + `HitTarget`. A form is **excluded** from the
  first-click candidate list outright, because a `/BBox` is a §8.10.1
  clipping-**extent** declaration and not a statement about coverage.
- **OCR is an edit.** `EditSession::add_ocr_layer`, one undo entry per run,
  reads the session's current revision. Plus `pdfce-cli ocr --in-place`
  through the same verb.

### ★★ What was owed out of them — TWO OF FOUR ARE NOW DONE

1. ~~**`unshare_form` (`Pass 119.1`)**~~ — **SHIPPED** (`cd5e5cc`). The `R206`
   obligation is **discharged**, and decision 076's compliance claim now reads
   *satisfied* after a three-step correction (false → outstanding → satisfied,
   all three readable in order in `ARCHITECTURE.md` §12). An operator can give
   one page a private copy of a shared form; a nested invocation is refused by
   name.
2. ~~**A CLI surface for leaves**~~ — **SHIPPED** (`5f6ac58`, amended from
   `a2f7b48` after a mangled commit message was rewritten). `object-list`
   prints `leaf` rows with containment and `editable=false`, plus `leaves=`,
   `form_cycles=` and `form_depth_overflows=` appended to the summary line.
3. **`--in-place` on the other editing subcommands.** ★ Still owed. Two
   subcommands' `--help` promised the flag while it existed nowhere; the
   wording is corrected and `ocr` has it, the rest do not.
4. **Retire `pdfce-render`'s `MAX_XOBJECT_DEPTH`.** ★ Still owed. The nesting
   bound now lives once in `content::MAX_FORM_DEPTH` and `text_extract` and
   `vector` both take it from there; render still has its own `pub const`, and
   retiring it is a cross-crate breaking change.

### ★ And a fourth, newly visible

**`Pass 119.4`** — retarget `reflow_block` / `add_text` into form XObjects.
Named in its own Backlog entry as needing **its own disclosure design before it
can ship safely**: appending to a form's content stream changes *every*
invocation site, which is a different disclosure shape from `edit_text`'s
single-target report. Do not treat it as plumbing.

### ★★★ THE THING TO NOT UNDO

**The recursion is not a read-only change**, and the design turns on it.
Thirteen CLI verbs take a paint-order object index and **eleven sites in
`edit.rs` resolve one** — every one writing to the **page's** content stream. A
leaf's token range indexes the **form's** stream: a different buffer, and an
*in-range* one, so the corruption would be silent.

⇒ `PageObjects::leaves` is a **separate list** from `PageObjects::objects`, and
`the_flat_object_list_is_unchanged_by_recursion` guards it. Merging them
"for tidiness" re-points eleven write sites at the wrong buffer. If editing
*through* the recursion is ever built, it needs a vector-side invocation census
first and a verb that takes a containment path — not an object index.

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
