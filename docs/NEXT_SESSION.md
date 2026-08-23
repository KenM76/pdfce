# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. Overwrite it once acted on.

**UPDATED 2026-08-23 by `pdfce-librarian` (two-hundred-and-thirty-seventh
filing). ★ THIS IS THE SESSION-CLOSING PASS FOR 2026-08-22/23 — the last filing
of the run.** §A is written so a **cold reader needs nothing else**: no prior
conversation, no scrollback, no other document. **§A is self-contained** — the
sections after it carry detail and evidence, and §A does not depend on them.

---

## §A — COLD START: everything you need, in one screen

**`HEAD` is `a68498b`. The working tree is clean. All 16 bare-runnable gates
exit 0. Nothing is blocked on anybody. No decision is waiting on Ken except the
two acts that are his by definition — pushing and cutting a backup.**

### What this run built (2026-08-22 → 2026-08-23)

**One theme: pdfce learned to render correctly at extreme magnification, and the
demo page that proves it grew a biology.** Seven Passes, in the order they
landed:

| Pass | commit | what an operator would notice |
|---|---|---|
| **`74.4`** | `e36f96e` | **The renderer stops executing Form XObjects it cannot see.** A `Do` whose `/BBox` maps entirely outside the canvas or clip is skipped **exactly** — no pixel changes. `339 of 342` culled on the demo page; hoisting the cull **above** the flate-decode took it from `802 ms` to `120 ms`, because ~37 MB of inflate per render was being spent on content about to be discarded |
| **`74.5`** | `e36f96e` | **Every mitochondrion on the demo page is drawn to real anatomy.** Two silent geometry bugs found by *rendering*, not reading: a unit constant inverted by `8.03×` (everything correctly placed, correctly shaped, an eighth of its size) and a self-crossing subpath whose nonzero winding filled every crista lumen with matrix colour |
| **`74.6`/`74.6b`** | `950e3af`, `65e1910` | **The molecule box** — ten molecules at 1:1 below the cells. This is the artefact that forced the ceiling question, because it is the first thing needing **both** a correct viewport and correct content at the same magnification |
| **`74.7`** | `1d6db9e` + `5b0d885` | ★★★★★ **The content-side `f32` ceiling raised, in TWO algorithms.** The CTM is carried in `f64` through content-stream composition and narrowed only at the leaf; and past a magnitude threshold the interpreter builds each path **relative to its own first point**, differencing in `f64`, so a page coordinate never has to survive being narrowed. **A single water molecule now renders sharply at scale `1.6e9` — about 190 billion percent.** ★ **And it made deep zoom `23.8×` faster as a side effect** (`31 s → 1.3 s` on a stroke-heavy CAD region at `100 000×`) — deep zoom was never slow *because* it was imprecise, it was slow *for the same reason* |
| **`74.8`** | `eca07ee` | `render-page --no-annotations` now **says how many annotations it withheld**. A counter that exists and is not surfaced is worse than one that does not exist — it makes the gap look measured |
| **`74.9`** | `296a23e` | **`--fast-subpixel`** — the opt-in **lossy** sub-pixel cull, **OFF by default**, with its own metrics key. `13.6×` faster with **zero of 1 242 640** pixels different in one window, and `62 of 255` worst channel delta in another near the threshold. **Loss is largest where the speed-up is smallest**, which is why it is a switch and not a heuristic. Closes operator question `(br)` |
| **`74.10`** | `d8f3020` | ★★★★★ **The display list REFUSES a scale it cannot render truthfully.** `74.7` fixed the interpreter and left the recorder in `f32`, so for two commits pdfce had two rendering paths that agreed at ordinary scales and diverged at deep zoom. **The threshold is a SHARED PREDICATE** — `Mat64::needs_precise_paths`, the same one the interpreter uses to switch to `f64` — so below it both paths do identical arithmetic and above it one switches while the other refuses. **No scale exists at which they quietly disagree.** See §0 |

**Also this run, with no Pass ID:** `ea413a4` (the `render-page` metrics contract
made whole and gated — `87 of 87`, now `90`), `1af75a1`, `4af0c08`, `641deb9`,
`39b449f`, `a68498b` (six rounds of repair on **one** README — see §5), and
`1a60e73` (librarian hard rule 11 amended).

### What is OPEN, in one list

1. **Four owed edits in `tools/gen-scale-demo/README.md`** — all prose, all the
   engineer's, **§5 has the exact repair for each.** The one that matters is
   **§9's *"three post-fix columns"***: the table has **four** columns of which
   **two** are post-fix, and the third item the sentence enumerates is a prose
   note, not a column.
2. **`R214`'s positional-reference gate** — recommended, unbuilt, **baseline
   unmeasured**. Measure, repair, then wire. **Never wire it red**
   (`D:/dev/rag/rust/ci_gate_red_at_baseline_enforces_nothing.md`).
3. **`R215`'s retro-application** — unstarted for a **sixth** filing. Any Pass
   filed with a *"required after"* column must be re-read against `R215` before
   that column is used as a gate. Runs over `docs/` **and both RAG tiers**.
4. **The engineering queue** — §4, unchanged in order: `97.1g`, `97.1k`,
   `122.2`, `122.1`, `122.0` (threading, the operator's own request), `119.1`,
   `122.3`.

### ★★ THE TWO ITEMS THAT ARE KEN'S, AND THEY ARE THE ONLY THINGS AWAITING HIM

Both are his acts by definition, not an agent's. **Both figures below were
measured on 2026-08-23 by the commands named beside them — re-run them, do not
quote them.**

- **PUSHING.** `git rev-list --count origin/main..main` → **`28`**. The local
  `origin/main` tracking ref is still at **`c24ad7a`**; **no `git fetch` was
  run**, so that is the tracking ref's position, **not a live query of the
  remote**. Pushing needs a **current** go-ahead (`CLAUDE.md` rule 8), and
  **the repository is public — an agent must not push.**
- **CUTTING A BACKUP.** The newest bundle in `D:/Dev/pdfce-backups/` is
  `pdfce-20260817-v060.bundle` (2026-08-17 20:34), whose `refs/heads/main` is
  **`3c4c00e`**. `git rev-list --count 3c4c00e..main` → **`221`**, and
  `git merge-base --is-ancestor` confirms **`HEAD` is not in it**. ⇒ **221
  commits and six days behind.** `v0.7.0`'s tag is in **no** bundle on disk.

### Ledger at end of session

Next free Pass in the `74` family **`74.11`**; `122.4` the next free `122`;
**`97.1g` reserved and unbuilt**. Decisions **`084`**, next free **`085`**.
Standing rules **`R215`**, next free **`R216`** — `R214` was amended today with
clause (d) and **no rule was minted by the closing filing**. Operator questions
**`(br)`, CLOSED**, next free **`(bs)`**. `render-page` metrics line at **`90`
keys**. **17 `tools/check-*` on disk, 16 runnable as bare gates.** Filing
ordinal **237**.

### ★★★★★ The one thing to read before repairing any document here

**Six rounds of repair against one README (`4af0c08` → `a68498b`) produced a
measured result that generalises: EACH ROUND'S REPAIR SEEDED THE NEXT ROUND'S
DEFECT.** Three consecutive rounds ended with a **quantifier defect introduced by
the repair of a quantifier defect**. And the closing filing's sweep of its **own**
documents found **four** instances of the same shapes it had spent the day
filing about — including **the wrong denominator that caused the current
survivor, published in the RAG finding written to warn about this mechanism.**

⇒ **A stable defect class after repeated rounds is evidence that the repair
process is generating its own input, not evidence that the document is
converging.** Full text: `ROADMAP.md`'s *Update protocol* →
*"How you know a document is converging"*, and
`D:/dev/rag/rust/the_defect_class_left_after_repeated_repair_rounds_is_the_stopping_signal.md`.
**Practical form: when you repair a sentence, re-read the sentence you wrote —
its quantifiers against the set, its references against the target — before you
commit it.** That is `R214` (c) and (d); nothing new is needed.

---

### What the last filing decided, in four lines

**`a68498b` filed with NO Pass ID** — the 236th filing's four survivors are all
discharged, and **the repair emitted three more**, one of them a **wrong column
count**. **No rule minted** (declined on coverage, not on `n`: `R214` (c)+(d)
already bind on a repair's own sentence). **`ROADMAP.md`'s convergence practice
is AMENDED** — its *"no wrong values left"* component is falsified, its class
ladder retired, its counter-caution promoted to the main finding at `n=3`.
**Ceiling unmoved at `R215`; `R216` still free; §12 stays at `084`.**

*(The 233rd filing's rewrite is preserved below and still current for
`PASS 74.10`: `decision 084` minted, `R211` amended with clauses (d) and (e),
and the display-list precision defect closed.)*

---

## §0 — WHAT `PASS 74.10` DID, AND WHY IT MATTERS MORE THAN ITS SIZE

**`PASS 74.7` fixed one rendering path and silently created a divergence with
the other.** That is the whole story and it is worth reading before touching
anything in `pdfce-render`.

- The **interpreter** got `f64` CTM composition (`Mat64`) and `f64` path
  differencing.
- The **display-list recorder** did not. A recorded op stores its CTM as an
  `f32` `Transform`, and `replay_region` post-translates by the region's device
  origin **also in `f32`**.
- ⇒ **two rendering paths that agreed at ordinary scales and diverged at deep
  zoom.** Measured: at a recording `scale` of **`5 000`** a letter page's
  transform carries a translation of **`~4 × 10⁶`**, where `f32` costs **half a
  device pixel**; at **`500 000`**, **`47 px`**.
- And nothing bounded it **on purpose**: `record_page`'s own comment says *"a
  recording allocates no raster, so the `MAX_PIXMAP_EDGE` ceiling does NOT apply
  here — that is the point of recording at a deep zoom."* **Deep zoom is the
  intended use of a recording.**

**The fix is a refusal, not a refactor.** `PoisonReason::ScaleBeyondF32`;
`record_page` returns `RenderError::PageNotRecordable`; the caller falls back to
a direct render correct at any scale — **the same fallback `pdfce-cli` already
performs for the seven capability refusals.**

### ★★★★★ THE PART TO REUSE — THE THRESHOLD IS A **SHARED PREDICATE**

The refusal fires on **`Mat64::needs_precise_paths`, the SAME predicate the
interpreter consults to decide it needs its `f64` route.**

- **Below it** — both paths do **identical `f32` arithmetic** and agree exactly.
- **Above it** — one switches to `f64`, the other **refuses**.

⇒ **No scale exists at which they quietly disagree.** A boundary **by
construction**, not a compromise, and not two limits that happen to match.

**★ Why that distinction is load-bearing, because the weaker version is what a
hurried edit produces.** Two independently-chosen constants can agree today and
drift apart on any later edit, and **the band between them is a magnitude range
where one path is precise, the other is not, and both believe they agree.** That
band emits nothing — no error, no counter, no red test — because it is defined
by the *absence* of a disagreement anybody checks for. A shared predicate makes
the band **unable to open**. ⇒ **Two paths that must agree share the predicate
that decides when they CAN, rather than each picking a limit.**
`decision 084`; `R211` clause (d);
`D:/dev/rag/rust/two_paths_that_must_agree_share_the_predicate_that_decides_when_they_can.md`.

**Also settled by this Pass:** `RenderOptions` **and** `PoisonReason` are both
`#[non_exhaustive]` (`crates/pdfce-render/src/font/mod.rs:427`,
`crates/pdfce-render/src/display_list.rs:362`) — **read, not relayed** — so
neither `subpixel_culling` nor `ScaleBeyondF32` is source-breaking downstream.
That question is off the owed list.

---

## §0.5 — ★★★★★ READ THIS BEFORE WRITING ANY DIFFERENTIAL TEST — `R211` CLAUSE (e)

**The oracle that should have caught the above runs at scales up to `220`. The
divergence begins near `530`.**

`crates/pdfce-render/tests/region_matches_full_page.rs` is a **good** test — it
caught a real 5,841-byte offset defect earlier the same week. Every assertion in
it is **correct, non-vacuous and independent.** It simply **never asked at a
scale where the answer changed.**

⇒ **A differential test proves agreement only over the range it samples, and the
range is part of the claim.**

**Three neighbouring failures, and this is the nastiest, so keep them apart:**

| failure | the question that catches it | did its assertions run? |
|---|---|---|
| **vacuous** (`R162`) | *could this ever have come out false?* | no |
| **wrong oracle** (`R215`) | *what mechanism produces this number?* | yes, demanding the wrong answer |
| **under-sampled domain** (`R211` (e)) | *over what range did it ask?* | **yes, and every answer was right** |

**What to do:** **sample across the PREDICATE, not across "a reasonable
range."** Where two paths share the predicate that switches behaviour — which
clause (d) obliges — **that predicate names the sample points**, and *"sample
more"* becomes bounded instead of infinite. **State the sampled domain in the
test's own name or doc comment.** And a *"these cannot disagree"* comment must
name the test **and its range**: here the test was **named, existed, ran, and
was green**, and **a named green test over the wrong domain is more persuasive
than no test at all.**

**★★ WATCH THE WORD "ANY".** *"Byte-identical output at **any** core count"* —
`Pass 122.0`'s acceptance criterion — is a claim over an unbounded domain that
will be tested at a handful of values. **Read this section and `R215` before
writing its acceptance table.**

---

## §1 — THE THREE PRE-FLIGHT CHECKS, unchanged and still earning their place

**1. `ls` BOTH FeatureRequests channels.** They are outside this repository, so
**no gate will ever contradict a stale sentence about them — including this
one.**

```
D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\
D:\Dev\FeatureRequests\iccce_FeatureRequests\open\
```

Three sessions running, that `ls` found something a document said was not there.

**2. Run the gates — `ls tools/check-*`, do not trust any list.**
`python tools/check-ci-parity.py --list` prints the local stand-ins.
**`R209`:** *"all gates green" names a set, and the set somebody runs is not the
set CI runs; a CI job with no local runner is UNOBSERVED, not passing.*
*(★★ **Re-measured 2026-08-23, 235th filing, by `ls tools/check-* | wc -l` and
by running each: 17 scripts on disk (12 `.py` + 5 `.sh`), 16 runnable as bare
gates, ALL 16 EXIT 0 after that filing's edits.** The 17th, `check-image-colorspace-truth.py`, exits 1 on a bare
invocation **because it takes a fixture-directory argument** and is not a gate.
**Count them; do not quote a count.**)*

**3. Read `docs/compositor-plan.md`** before scoping anything in `97.x`.

---

## §2 — ★★ THE GHENT PASS COUNT IS AN OVER-COUNT. Read this before quoting any figure.

`tools/ghent-check.py` implements **one of the suite's two pass criteria**. It
hunts for a **cross that should not be there**; seven patches instead mark
failure by the **absence of a check mark** (GWG 050, 080, 081, 082, 150, 151,
152) and have scored `clean` since the harness was written. At least three of
them are failures. A second fault — the contrast floor has no **area** term —
puts `GWG 1.0` in the same category.

**Corrected standing: 26 at most, not 29.**

⇒ **THE DELTAS SURVIVE, THE LEVELS DO NOT.** Every board ever filed is
over-counted by the same family, so a before/after comparison is sound and any
absolute "N of 51" is not.

The operator's cell-by-cell judgements are in
`docs/ghent-operator-review-2026-08-21.md` — **the only independent check this
harness has ever had**, and the calibration set for fixing it (`Pass 122.2`).

---

## §3 — WHAT SHIPPED 2026-08-22/23

| commit | Pass | what |
|---|---|---|
| `e36f96e` | `74.4` + `74.5` | Form XObject viewport culling (exact); mitochondria drawn to real anatomy |
| `ea413a4` + `950e3af` + `65e1910` | — / `74.6` / `74.6b` | metrics contract whole and gated; ten molecules at 1:1 |
| `eca07ee` | `74.8` | `--no-annotations` says how many it withheld; nine stale doc claims discharged |
| `1d6db9e` + `5b0d885` | `74.7` | the content-side `f32` ceiling raised, in two algorithms; `23.8×` on deep-zoom CAD as a side effect |
| `296a23e` | `74.9` | `--fast-subpixel`, the opt-in lossy cull; closes `(br)` |
| **`d8f3020`** | **`74.10`** | **the display list REFUSES a scale it cannot render truthfully — closes the second rendering path `74.7` opened** |
| **`1af75a1`** | **—** | **README repair: a heredoc silently ate a line-continuation backslash in `d8f3020`** |
| **`4af0c08`** | **—** | **the demo README's three stale claims discharged — `11 of 11` → `2`, the boundary sentence scoped, `93 s → 1.3 s` corrected to `31 s → 1.3 s = 23.8×`** |
| **`641deb9`** | **—** | **the §9 pre/post boundary promoted from a SENTENCE to a SECTION HEADING — §9 *"How the ceiling was found — history, not current behaviour"*, §10 *"Where the ceiling is now"*, both old boundary sentences DELETED. A third rewording was declined: repairing a positional pointer positionally always produces another wrong one** |
| **`1a60e73`** | **—** | **librarian hard rule 11 amended (`.claude/`, uncounted by the commits gate): MINTING A RULE ABOUT A NUMBER is itself a meaning-change event, and the sweep it owes runs over EVERY tree — `docs/`, `tools/`, `.claude/`, and both RAG tiers** |
| **`39b449f`** | **—** | **both §9 repairs narrowed: the scope sentence says what §9 IS, and the tables sentence loses its plural** |
| **`a68498b`** | **—** | ★ **the closing repair — §10's scope sentence, §9's missing antecedent, the tables sentence, and the deleted-label reference. Four in, three out: see §5** |

**Ledger after this filing:** next free Pass in the `74` family **`74.11`**;
decisions **084**, next free **085**; standing rules **`R215`** (ceiling
**unmoved** — `R211` amended by the 233rd filing, `R215` itself by the 234th
(clause (d)), **`R214` by the 236th (clause (d))**; the 235th amended
**librarian hard rule 11** instead, which is a procedural commitment and not a
standing rule), next free **`R216`**;
operator questions ceiling **`(br)`, CLOSED**, next free **`(bs)`**;
**`render-page` metrics line at `90` keys, unchanged**, measured by running
`tools/check-metrics-line-contract.py`.

---

## §4 — THE QUEUE, in the order I would take it

**The previous item 0 and item 1 are both gone** — `(br)` is answered and
shipped, and the display-list precision question turned out to be a live defect
and is fixed. **Nothing in this queue is blocked on anybody.**

1. **★ FOUR SMALL REPAIRS LEFT IN `tools/gen-scale-demo/README.md`, all about
   SCOPE and REFERENCE, and NOT ONE OF THEM IS A NUMBER.** The 235th filing's
   two are **discharged** (`39b449f`); the 236th found four more by reading the
   result. **Full text and the exact repair for each is in §5.** In one line
   each: **(A)** §10's scope sentence *"Everything in this section describes
   pdfce as it stands"* is **false of four of §10's own passages** — this is
   **`R214` clause (d)'s live second instance and the one that matters**;
   **(B)** §9's *"the sentence above it"* has no written antecedent; **(C)** §9's
   new tables sentence carries a universal **and** an existential hedge over a
   set of **one**, and omits the `correct` column; **(D)** §10 names its target
   *"§9's forms-rendered table"* — a label deleted for being wrong.
   ★ **The theme: the split bought immunity to POSITIONAL drift and NOT to
   OVER-SCOPING.** A heading-bound claim is still a claim about a set.
2. **Build `R214`'s positional-reference gate** — a grep over a closed
   vocabulary (*of those*, *the above*, *this slice*, *the next slice*, *the
   former*, *the latter*, *as above*, *see below*, *the previous field*) in doc
   comments. **Measure its baseline first, repair, then wire** — never wire it
   red (`ci_gate_red_at_baseline_enforces_nothing.md`). ★ **Item 1's second
   survivor is a fresh instance of exactly what this gate would catch**, in a
   README rather than a doc comment.
3. **`Pass 97.1g`** — non-isolated ordinary groups on a subtractive page are
   composited as if isolated. The arithmetic exists (`remove_backdrop_cmyk`);
   the second content walk does not. **A port of the additive path, not a
   design.**
4. **`Pass 97.1k`** — native colorant paths for images and shadings, which
   bridge through sRGB today (`cmyk_bridged_pixels`).
5. **`Pass 122.2`** — teach `ghent-check.py` the check-mark criterion and give
   its contrast floor an area term. §2 says why this is not optional.
6. **`Pass 122.1`** — per-sample image overprint. Now **diagnosed**: it is why
   `GWG 8.2`'s check mark is missing. The mark is painted in yellow *underneath*
   the images, which overprint it so cyan-over-yellow reads green; pdfce paints
   the image normally and covers it (`overprint_images_unsupported = 2`).
   pdfium fails it too.
7. **`Pass 122.0`** — multithreading, the operator's request. His design (a
   runtime max-cores setting) is right and is kept; decision 080 adds a
   **compile-time target gate**, because `std::thread` and `rayon` both
   `cargo check` cleanly for `wasm32` and the CI wasm job therefore cannot catch
   a threading regression. Hard acceptance criterion: **byte-identical output at
   any core count.** ★ **The banana page is a genuinely useful threading
   benchmark**, because its cost is ~1 000 000 path operators in 342 forms
   rather than one big image. ★★ **Read BOTH §0.5 and `R215` before writing its
   acceptance table** — *"byte-identical at any core count"* is a **bound**,
   which is the right shape (`R215`), **and it is a differential claim over an
   unbounded parameter, which is `R211` clause (e)'s exact subject.** Sample
   across the switch points (1, 2, `n-1`, `n`, oversubscribed), not across "a
   few core counts".
8. **`Pass 119.1`** — `unshare_form`. Carried unstarted through eight handoffs
   now.
9. **`Pass 122.3`** — the colorant buffer's byte ceiling. Interactive use is
   unaffected, but a **full-page** render above ~375 DPI refuses the buffer and
   silently composites in the wrong space, so one page can have different
   colours at different resolutions.

---

## §5 — STILL NOT DONE, named so it does not read as done

**★★★ THE §9 BOUNDARY SURVIVORS ARE DISCHARGED — STRUCTURALLY, BY `641deb9`.**
Both positional pre/post boundary sentences are **DELETED, not reworded**. §9 is
now **"How the ceiling was found — history, not current behaviour"**, §10 is
**"Where the ceiling is now"**, the easter egg is §11, and each section opens by
naming its own scope.

**★★★★★ THE PART TO CARRY FORWARD IS THE DECLINE, NOT THE SPLIT.** A third
reworded boundary sentence was **refused**, on the librarian's advice, and the
warrant is a general one:

> **A pre/post boundary written in prose is a pointer into a document that
> grows.** Rewording it buys the interval until the next paragraph lands, and
> nothing more. Two verbal repairs of the same claim had already failed, the
> second one **already false when it was written**. ⇒ **The failure is
> INTRINSIC to the form, not a symptom of imprecise wording.**
>
> **Promote the boundary to a section heading.** A heading cannot drift,
> **because the thing it delimits is the thing that would have to move it** —
> delimiter and content are the same object and cannot get out of step. §9 says
> this out loud in its own text, not only in `git log`.

**★★ And the companion rule: PRESERVE WRONG VALUES, DELETE WRONG POINTERS.**
Deleting the two sentences is not an `R215` (d) violation. `R215` (d) protects a
wrong **value**, because a bare corrected number gets re-derived back to the
wrong one by the next reader. **A positional pointer has no value to preserve** —
the evidence is *that the form fails*, and that is kept.

**★★★ THE 235th FILING'S TWO SURVIVORS ARE DISCHARGED BY `39b449f`.** §9's scope
sentence is narrowed to say what the section **is** — the investigation — and to
state that its tables show both sides; §10's *"the table above"* now names §9's
table.

**★★★★★ AND `R214` IS AMENDED WITH CLAUSE (d), OVERTURNING THE DISPATCH'S OWN
RECOMMENDED DECLINE.** The dispatch judged it a *"decline with a recorded fact —
it is one instance, and the general shape is close to platitude"*. **The general
shape is platitude and is not what was filed.** What was filed is the narrow,
checkable version the dispatch itself wrote:

> **A structural repair emits a NEW CLAIM OF A DIFFERENT KIND — a *scope
> statement*, which is a universal quantifier over the delimited contents — and
> it is an assertion nobody has checked.** Promoting a boundary to a heading
> removes positional **drift** and buys **nothing** against **over-scoping**:
> the heading fixes where the claim's **edge** is; the sentence beneath it still
> asserts what is **inside**; and those two are independent.

**★★★★ The decline is overturned because `n` is 2, not 1, and the second
instance is LIVE.** `641deb9` split §9 from §10 and **each new section emitted a
scope sentence. Both were false.** §9's was repaired by `39b449f`; **§10's was
never touched.**

**Why a clause of `R214` and not `R216`:** the defect is a **rider on `R214`'s
own prescribed remedy** (clause (a): *promote the boundary to a heading*), and
that remedy is what manufactures a scope sentence. A separate number would
**detach the warning from the remedy that causes it** — which is exactly how it
happened twice in one commit. Clause (c) already says *a repair is a new claim*;
it never said **what kind** of claim a **structural** repair makes.
**CEILING UNMOVED at `R215`; `R216` still free.**

**★★★ THE 236th FILING'S FOUR SURVIVORS ARE ALL DISCHARGED BY `a68498b`** — §10's
scope sentence is now *"This section is what pdfce does NOW"*, §9's paragraph
names its referent in its first sentence, the tables sentence is singular and
definite, and §10 says *"§9's measurement table"*. **Prior wording is kept
visible at three of the four sites, per `R215` (d)** — the fourth was a wrong
**label**, not a wrong **value**, and this project's companion practice is
*preserve wrong values, delete wrong pointers*. *(Their full text is in `ROADMAP.md`'s
`39b449f` entry; it is not repeated here.)*

**★★★★★ AND THE REPAIR EMITTED THREE MORE, WHICH IS THE POINT OF THIS SECTION.
Four survivors owed, all in `tools/`, all the engineer's, all reported and none
edited.** Found by reading `a68498b`'s **result**, not its diff — the method that
has now found survivors four filings running.

1. **★★★★ §9's *"THREE POST-FIX COLUMNS"* — A WRONG COUNT, AND THE THIRD
   CONSECUTIVE GENERATION OF A QUANTIFIER DEFECT INSIDE A QUANTIFIER REPAIR.**
   The sentence reads *"Its one measurement table … carries **three post-fix
   columns**: `after`, `correct`, and the note explaining why `correct` is not
   `11`."* **The table's header is `scale` / `before` / `after` / `correct` —
   four columns, of which TWO are post-fix** — and the third item enumerated is
   the ★ **paragraph** below the table. **A note is not a column.**
   ⇒ **Repair:** *"§9's one measurement table has four columns — `scale`,
   `before`, `after`, `correct` — of which `after` and `correct` are post-fix;
   the ★ note below it says why `correct` is not `11`."*
   ★★★★★ **Where the `3` came from is the finding, and it is this role's:** the
   236th filing's own RAG record of this very defect class
   (`D:/dev/rag/rust/the_defect_class_left_after_repeated_repair_rounds_is_the_stopping_signal.md`)
   said *"while omitting **the third** post-fix column entirely"*, and the same
   filing's three project documents said *"two-thirds of the evidence"* where the
   answer is **one-half**. **The same filing also stated the correct count
   twice.** ⇒ **the report held both figures and the wrong one is what reached
   the artefact.** Both RAG copies are corrected; the wrong wording is kept.
2. **★★★★ §9's *"THE GAP BETWEEN `after` AND `correct` … §9's LAST PARAGRAPH"* —
   A POSITIONAL POINTER, WRONG IN BOTH HALVES, TWENTY LINES ABOVE THE PARAGRAPH
   EXPLAINING WHY POSITIONAL POINTERS DRIFT, IN THE COMMIT THAT REMOVED ONE.**
   **(i)** The named gap does not exist — `after` and `correct` are **identical
   in 5 of 5 rows** (`11`/`11`, `11`/`11`, `2`/`2`, `2`/`2`, `2`/`2`); the gap
   the ★ note is about is between **`correct` and `11`**. **(ii)** §9's **last**
   paragraph is the *"`--region` is parsed as `f64` … quantised in ~500 px
   steps"* one, four paragraphs after the ★ note.
   ⇒ **Repair:** *"…and because the ★ note directly below the table explains why
   `correct` is not `11`."*
3. **★★ §10's ENUMERATION COVERS THREE OF THE FOUR PASSAGES.** Its new sentence
   names the `76 px / 288 px` before-figure, the `93 s` provenance and the
   preserved prior wording, then asserts *"**Each** is labelled where it
   appears"*. **The fourth passage — *"The number began as an expectation in a
   roadmap table"* — is neither a before-figure nor a quoted wording**; its
   subject is another document's history. And *"Each …"* is itself a universal
   over the enumerated set, in the paragraph citing `R214` (d). **Modest**: the
   passage self-labels in context. ⇒ **Repair:** add *"and one paragraph whose
   subject is the number's own travel through this project's roadmap"*.
4. **★ §9's *"THE FORMS-RENDERED COUNTS"* — THE DELETED LABEL MOVED RATHER THAN
   LEFT.** `a68498b` removed *"forms-rendered"* from §10 and **added it at line
   293** in the same commit. **Two occurrences before, two after** — live prose
   at 293, preserved evidence at 372. ★ **Better than the survivor it replaced**,
   because 293 carries a true live anchor beside it; ★ but a grep for the deleted
   label still returns live text and preserved evidence side by side.
   ⇒ **Repair (weakest):** *"the form counts"*.

**★★★★★ NOTE WHAT IS LEFT — AND NOTE THAT THE PREVIOUS ANSWER TO THIS QUESTION
WAS WRONG.** *(This paragraph read, until 2026-08-23: "After **five** rounds
against this one file, the residue is wrong QUANTIFIERS and wrong REFERENCES, and
no wrong VALUES. The number-shaped defects are swept out … everything left
requires reading for meaning." Kept per `R215` (d), because it is the claim round
6 refuted.)*

**Round 6 falsified two of that paragraph's three components.** *"No wrong values
left"* is **false** — survivor 1 is a wrong **count** and the report that
prompted it carried a wrong **fraction**. The residue is **not a residue, it is
production** — survivor 2 is a **fresh** wrong reference, of the class round 5
declared **structurally** fixed by promoting the boundary to a heading. And the
counter-caution — *the residue re-seeds itself* — is **confirmed and measured at
`n=3` consecutive rounds**.

**★★★★★ ⇒ THE AMENDED FINDING: a stable defect class after repeated rounds is
evidence that the REPAIR PROCESS IS GENERATING ITS OWN INPUT, not evidence that
the document is approaching a fixed point.** The re-seeding was filed as a
caveat; at `n=3` it is the **dominant mechanism**.

**★★★★ And the class LADDER is retired.** *Values → pointers → quantifiers →
antecedents* is a true description of **what each round found first** and **not**
a description of **what the document contains** — round 6 produced a wrong count
**and** a wrong positional pointer at once, classes one and two, both recorded as
swept. ⇒ **the classes RECUR; they do not RETIRE.** A reader who takes the ladder
literally stops looking one rung down.

**★★★ And the reviewer is inside the process.** The closing filing swept its
**own** documents and found **four** instances of the shapes it had spent the day
filing about: an ordinal pointer naming the wrong column in `ROADMAP.md`; the
*"two-thirds"* fraction in three documents at once; *"Everything below this line
is unchanged from the previous handoff"* in **this file**, false in the commit
that wrote it; and *"the third post-fix column"* in the RAG finding — **the copy
that propagated.** ⇒ *"the residue is the class only reading catches"* is
incomplete: **it is the class only reading catches, in a document nobody is
reading with the same intent.** **All four repaired, wrong wording preserved.**

**★★ NO RULE WAS MINTED FOR ANY OF THIS, AND THE DECLINE IS DELIBERATE.**
`R214` clauses (c) and (d) already bind on a repair's own sentence and cover all
three of round 6's new survivors. Three rounds of evidence that a rule was not
applied is **non-application, not a gap**, and a second number would **detach the
warning from the remedy that causes it** — verbatim the reason clause (d) was
filed as a clause. **`R216` stays free.**

**★★★★ HARD RULE 11 IS AMENDED AND THE AMENDMENT HAS BEEN EXERCISED ONCE
(`1a60e73`).** The trigger now fires on **minting a rule about a number**, and
the sweep runs over **every tree this project writes to**, not only the one
holding the rule. Two new survivor shapes are named: **a wrong number returning
as a RESULT rather than an expectation**, and **the claim living in a TABLE
HEADER or column label**.

**★★★ Its first run found three survivors, ALL OUTSIDE `docs/`, AND NONE OF THEM
WAS THE NUMBER IT WENT LOOKING FOR.** The `11`-forms claim and the `93 s` /
`71.5×` ratio are **fully discharged everywhere** — measured, site by site, not
assumed. What survived was **`PASS 74.7` recorded as *unfixed / Backlog*, three
days after it shipped**, in `C:/personal_rag/pdf/` (a lesson body **and** its
index bullet) and in `D:/dev/rag/rust/a_ceiling_is_a_claim_about_one_quantity.md`.
**All three repaired by the 235th filing**, with prior wording preserved per
`R215` (d).

**★★ One of the three carried `R215`'s founding error in its STRUCTURAL form, in
a tree `R215` never reached** — the lesson called its `11/7/3/1` broken-system
curve *"that Pass's acceptance baseline"*, under a bare column label reading
*"forms rendered"*. `R215` was minted 2026-08-23; that sentence was written
2026-08-22 in a different tree.

**⇒ THE TRANSFERABLE LESSON: SWEEP THE TREE, NOT THE STRING.** A sweep scoped to
*the number that prompted it* would have returned clean and been believed. **The
claim you are hunting is not necessarily the claim that is stale in the place you
have not looked before.**

**★ Sweep scope, ruled so it is not re-litigated:** `.claude/worktrees/` holds
nine detached agent working copies, each a full stale snapshot of `docs/` and
`crates/`. **They are out of scope by definition** — a worktree is a transient
copy, not a tree this project writes to, and its staleness is by construction.
`rg` skips them (they are gitignored) and a bare `grep -r` does not; **`rg` is
therefore the correct instrument.** Recorded so a future filing does not
"discover" hundreds of survivors there and report a crisis.

**★★ The `93 s` provenance itself is CLOSED.** At `100 000×`: **`31 s`** = the
same region with algorithm B disabled entirely — **the baseline**; **`93 s`** =
the **rejected device-space attempt**; **`1.3 s`** = shipped. ⇒ `23.8×` is the
speed-up; `3×` is the rejected attempt's penalty; and **`93 s → 1.3 s` = `71.5×`
compares the shipped result against a rejected implementation rather than against
a baseline.** Dated amendments are in `ROADMAP.md` in both places, and
`crates/pdfce-render/src/gstate.rs` states it correctly at the source.

**★ The items below are the standing not-done list.** *(This line read*
*"Everything below this line is unchanged from the previous handoff"* until
2026-08-23. **It was false in the commit that wrote it** — that same commit
rewrote §6 and §7 below this point. It is `R214` clause (d)'s exact
construction, a universal quantifier over delimited contents, written by the
filing that minted clause (d). Wrong wording kept per `R215` (d).)*

- **`R215`'s retro-application** — any Pass filed with a *"required after"*
  column should be re-read against the rule before that column is used as a
  gate. **Carried forward unstarted, for a FIFTH filing — and its scope is now
  known to be WIDER than the roadmap**: the 235th filing found the first
  instance outside `docs/`, so the re-read should run over both RAG tiers too.
- `resolve_indexed` builds its palette with a **scratch `ColorDiagnostics` that
  is discarded**, so a tint failure inside a palette never reaches the operator.
- **Implicit knockout**: only explicit `/K true` is honoured. `/TK` defaults true
  (every text object), and `B`/`b` and shading patterns are knockout. **The one
  pdfce implements is the rarest.**
- **`/TR` on a soft mask** is read, counted, never evaluated.
- **`/AIS true`** is not distinguished from `/AIS false`.
- **Spot colorants** — four planes, not runtime `N`. Every remaining
  trap-criterion Ghent FAIL is in this bucket.
- **Per-paint rendering intent** (§11.7.5.3) — pdfce carries one per page.
  `iccce` costed the alternative and asked for the consumer fact; **no corpus
  measurement of mid-page intent switching has been taken.**
- **No GUI code path reads** `forms_culled`, `subpixel_culled`,
  `annots_out_of_scope`, `page_content_suppressed`, `render_page_region` or the
  display list, **and no GUI exposes `--fast-subpixel`.** GUI work is paused;
  recorded so the `[ ] gui` boxes in `FEATURES.md` are not mistaken for
  oversights.

---

## §6 — LESSONS FROM THIS RUN THAT ARE CLASSES, NOT INCIDENTS

1. **★★★★★ TWO PATHS THAT MUST AGREE SHARE THE PREDICATE THAT DECIDES WHEN THEY
   CAN — THEY NEVER EACH PICK A LIMIT.** §0 has the instance and the reason.
   Two constants that match today drift apart on any later edit, and **the band
   between them emits nothing**. `decision 084`, `R211` clause (d).
   `D:/dev/rag/rust/two_paths_that_must_agree_share_the_predicate_that_decides_when_they_can.md`
2. **★★★★★ A DIFFERENTIAL TEST PROVES AGREEMENT ONLY OVER THE RANGE IT SAMPLES.**
   §0.5 has the instance and the three-way table that keeps it apart from `R162`
   and `R215`. `R211` clause (e).
   `D:/dev/rag/rust/a_differential_test_proves_agreement_only_over_the_range_it_samples.md`
3. **★★★★ A FIX CAN CREATE THE DEFECT ITS OWN FILING THEN FINDS.** `74.7` was
   correct, tested, measured and shipped — **and it opened a divergence with a
   path it never touched.** The filing of `74.7` flagged the question as *"either
   already handled, or a real gap; this role cannot tell which"*, and it was the
   second. ⇒ **when a change moves one implementation of a duplicated contract,
   the other implementation is part of the change**, whether or not it appears
   in the diff.
4. **★★★ AN EDIT THAT SILENTLY DROPS ONE HUNK OF A MULTI-HUNK CHANGE IS WORSE
   THAN ONE THAT FAILS LOUDLY, BECAUSE THE OTHER HUNKS LAND** and lend the
   missing one their credibility. `d8f3020` updated the prose around a shell
   snippet and lost the snippet itself to a heredoc eating a line-continuation
   backslash, so the README **contradicted itself a few lines apart**. **The
   check is to re-read the ARTEFACT, not the diff** — the diff showed exactly
   what was applied and looked fine. **Fifth instance in a week**; see
   `.claude/agent-memory/pdfce-engineer/feedback_windows_paths_need_literal_edits.md`.
   ★ The 233rd filing hit the same failure on its first attempt and wrote the
   file literally instead.
5. **★★★ A REFUSAL THAT FALLS BACK COSTS PERFORMANCE; A REFUSAL THAT CHANGES THE
   OUTPUT COSTS FIDELITY.** Log the first through the error's own message; give
   the second its own counter. `74.10` adds **no** metrics key and that is
   deliberate; `74.9`'s `subpixel_culled` needed one. `decision 084` vs `083`.
6. **★★ A REFUSAL'S *KIND* IS PART OF ITS CONTRACT.** A capability refusal is a
   property of the **document** (*"this page uses an operator I cannot record"*);
   a precision refusal is a property of the **request** (*"this scale is one I
   cannot record truthfully"*). A caller that cached *"this page is not
   recordable"* would now be wrong — the same page records fine at a lower scale.
7. **★★ HARD RULE 10 (a) PAID FOR ITSELF TWICE THIS WEEK, IN A FILE NOBODY WROTE
   IT FOR.** The `93 s` ambiguity was repairable **without re-measuring
   anything** only because both ratios were filed beside their operands; and the
   README's *"23× faster … 93 s to 1.3 s"* is **self-refuting on its face**
   because the ratio and its operands are both written down. **A figure filed in
   a form that can disagree with something is a figure that can be corrected.**
8. **★★ A PRECISION DEFECT AND A PERFORMANCE DEFECT WITH ONE CAUSE LOOK
   UNRELATED UNTIL ONE OF THEM IS FIXED** — `31 s → 1.3 s` = `23.8×` fell out of
   a precision fix.
   `D:/dev/rag/rust/a_precision_defect_and_a_performance_defect_with_one_cause_look_unrelated_until_one_is_fixed.md`
9. **★★ `tiny_skia` FLATTENS CURVES TO A TOLERANCE IN THE PATH'S OWN UNITS.**
   **Move the SUBTRACTION into the wide type; do not move the COORDINATES into a
   big space.**
   `D:/dev/rag/rust/tiny_skia_flattens_curves_to_a_tolerance_in_the_paths_own_units.md`
10. **★★★★★ A NUMBER IDENTIFIED AS WRONG IN AN *EXPECTATION* COMES BACK AS A
    MEASURED *RESULT*, AND THE GENRE CHANGE IS WHY IT SURVIVES.** *An
    expectation invites checking; a result does not.* It happens **naturally**:
    writing up a fix, you reach for the criterion you were working against and
    report it as met. ⇒ **THE CORRECTION IS THE ARTEFACT, NOT THE CORRECTED
    VALUE** — record *"this said **X**; **X** is impossible because **Y**; it is
    **Z**"* and **keep X visible**, because a bare `Z` is re-derived back to `X`
    by the next reader with the same faulty intuition, **including yourself an
    hour later**. `R215` **clause (d)**, added 2026-08-23.
    `D:/dev/rag/rust/a_number_identified_as_wrong_in_an_expectation_comes_back_as_a_measured_result.md`
11. **★★★ A MINTED RULE PROTECTS THE DOCUMENT IT WAS MINTED IN, NOT THE ONES
    THAT QUOTE IT.** `R215`'s founding number escaped forward in **two
    consecutive filings** — the mint swept `docs/`, and `11 of 11` was sitting in
    `tools/`. **Minting a rule about a number is itself a meaning-change event**
    and owes hard rule 11's sweep across **every** tree. Recommended as an
    amendment to `.claude/agents/pdfce-librarian.md`; **the engineer's act.**
12. **★★ AN ACCEPTANCE ORACLE BUILT FROM THE BROKEN SYSTEM'S OWN OUTPUT ENCODES
    THE DEFECT AS THE REQUIREMENT.** `R215` (a)–(c).
13. **★ A FLOAT-PRECISION TEST WITH ROUND-NUMBER OPERANDS CANNOT FAIL.** Two
    **equal** large `f32` values cancel **perfectly**.
14. **★★ A reference by POSITION cannot tell you whether it is BROKEN or WRONG,
    AND REPAIRING ONE POSITIONALLY JUST MAKES ANOTHER.** `R214` — name the
    referent; repair at the referent. **§5's survivors 2 and 4 are fresh
    instances**, in prose rather than doc comments.
15. **★ A limitation justified by "beyond any plausible use" is a PREDICTION,
    not a fact** — `R193`. **An error curve measured over the range you consider
    plausible cannot tell you the range is plausible** — which is §0.5's finding
    from the other direction.

16. **★★★★★ A STRUCTURAL REPAIR EMITS A NEW CLAIM OF A DIFFERENT KIND — A SCOPE
    STATEMENT — AND IT IS A UNIVERSAL QUANTIFIER NOBODY HAS CHECKED.** Promoting
    a boundary to a heading removes positional **drift** and buys **nothing**
    against **over-scoping**: the heading fixes where the claim's *edge* is, the
    sentence beneath still asserts what is *inside*, and the two are independent.
    `641deb9`'s split emitted **two** scope sentences and **both were false**.
    **`R214` clause (d)**, added 2026-08-23. ⇒ **when you convert prose into
    structure, re-read the structure's own boundary statements as claims.**
17. **★★★★★ THE REPAIR PROCESS GENERATES ITS OWN INPUT.** *(This item read
    "the CLASS of what is left after a repair round is the stopping signal — and
    it re-seeds itself. Values → references → quantifiers, over five rounds on
    one file." Kept per `R215` (d): round 6 refuted it.)* **Six rounds on one
    file, and `n=3` consecutive rounds ended with a quantifier defect introduced
    BY the repair of a quantifier defect.** Round 6 also produced a wrong
    **count** and a wrong **positional pointer** at once — both classes the
    previous round recorded as swept. ⇒ **the classes RECUR, they do not RETIRE**,
    and a stable residue class is evidence about **the repair process**, not
    about the document's convergence. ★★★ **The reviewer is inside the same
    process:** the closing filing's sweep of its own documents found **four**
    instances of the same shapes, one of them the wrong denominator that caused
    round 6's survivor, **published in the RAG finding written to warn about this
    mechanism.** Practice, not a rule (`R216` stays free).
    `D:/dev/rag/rust/the_defect_class_left_after_repeated_repair_rounds_is_the_stopping_signal.md`
18. **★★★ WHEN YOU REPAIR A SENTENCE, RE-READ THE SENTENCE YOU WROTE.** Its
    quantifiers against the set, its references against the target. That is
    `R214` (c) + (d) and **nothing new was minted for it** — three rounds of
    evidence that a rule was not applied is **non-application, not a gap**.
19. **★★★ CHECK A DRAFT CLAIM OF YOUR OWN AGAINST LIVE SOURCE BEFORE PUBLISHING
    IT.** The closing filing drafted *"the wrong denominator enters the record in
    this commit and nowhere earlier"*, on `git log -S` returning one commit.
    **`git log -S` searches the REPOSITORY, and the RAG tiers are not in it.**
    Checking the draft is what found the real source. Same corollary that found
    the real survivor at `main.rs:7027` on 2026-08-18.
---

## §7 — HOUSEKEEPING

**All four figures below were MEASURED on 2026-08-23 by the commands named
beside them (hard rule 8). Re-run them; do not quote these lines.**

- **`origin/main` is at `c24ad7a`, and `main` is `28` commits ahead** —
  `git remote -v` → `origin  https://github.com/KenM76/pdfce.git`;
  `git rev-list --count origin/main..main` → **`28`** (re-measured by the
  237th filing; was `26` one filing ago, `24` two ago). **No `git fetch` was
  run**, so that is the local remote-tracking ref's position, **not a live query
  of the remote.** Pushing is the operator's act and needs a current go-ahead
  (`CLAUDE.md` rule 8). **The repository is public, so anything committed is
  published by default.**
- **Backups are `221` commits and six days behind `HEAD`** (re-measured by the
  237th filing; was `219` one filing ago, `217` two ago). Newest bundle
  `pdfce-20260817-v060.bundle` (2026-08-17 20:34) with `refs/heads/main` at
  **`3c4c00e`** (by `git bundle list-heads`);
  `git rev-list --count 3c4c00e..main` → **`221`**;
  `git merge-base --is-ancestor` confirms **`HEAD` is NOT in that bundle**.
  `v0.7.0`'s tag is in no bundle on disk. **Cutting one is the operator's
  call** — and it is, alongside pushing, one of the only two outstanding items
  that are his rather than an agent's.
- **Worktrees: `9` entries**, by `git worktree list | wc -l`. **`git worktree
  list` is authoritative — do not quote this line, re-run it.**
- **If `check-commits-filed.py` is red when you start, READ ITS OUTPUT FOR THE
  HASH.** Do not assume which commit it means, and **never extend
  `tools/commits-filed-baseline.txt`** — that file is pre-existing debt, not an
  allowlist.
- **A commit may bundle doc-comment repairs; a FILING may not bundle code.** The
  rule `d4721d8` established has **two directions**, and the second is easy to
  miss: a librarian filing that carries a code change cannot file itself and
  manufactures one more unfiled commit. The reverse — doc repairs inside a code
  commit — costs nothing mechanically and is fine.
- ★ **A claim in `bd9844d`'s commit message is WRONG and stands corrected only
  in the filing.** It says `tools/check-string-gaps.sh` *"has caught it every
  time"*. It has not: `ae06440` (2026-08-20) is titled *"the string-gap gate
  reported two of three"*, and that miss was found by a human who knew there
  were three. The ordinal ("third time") survives; the "every time" does not.
