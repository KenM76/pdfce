# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

**UPDATED 2026-08-23 by `pdfce-librarian` (two-hundred-and-thirty-ninth filing).
★ THIS IS THE SESSION-CLOSING PASS FOR 2026-08-22/23 — the last filing of the
run.** §A is written so a **cold reader needs nothing else**: no prior
conversation, no scrollback, no other document. The sections after it carry
detail and evidence, and §A does not depend on them.

> ★★★ **This file carries NO edit-history layer, by rule.** It used to explain
> its own corrections in place (*"this paragraph read X until …"*), and that layer
> is **deleted**, not corrected. Preserved wrong wording and correction narrative
> belong in the **append-only** record — `ROADMAP.md` and `SESSION_LOG.md` — where
> a claim is dated and no later edit can falsify it. **This is standing rule
> `R216`**, minted by the filing that wrote this line, and this file is the second
> of its two founding instances. Where a figure below has been corrected, the
> prior figure and the reason are in the record; here you get **what is true
> now**, and a pointer.

---

## §A — COLD START: everything you need, in one screen

**`HEAD` is `8522167`. The working tree holds this filing's docs-only edits and
nothing else. `tools/check-commits-filed.py` goes green on this filing's own
commit. Nothing is blocked on anybody. No decision is waiting on Ken except the
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
| **`74.10`** | `d8f3020` | ★★★★★ **The display list REFUSES a scale it cannot render truthfully.** `74.7` fixed the interpreter and left the recorder in `f32`, so for two commits pdfce had two rendering paths that agreed at ordinary scales and diverged at deep zoom. **The threshold is a SHARED PREDICATE** — `Mat64::needs_precise_paths`, the same one the interpreter uses to switch to `f64` — so below it both paths do identical arithmetic and above it one switches while the other refuses. **No scale exists at which they quietly disagree.** See §1 |

**Also this run, with no Pass ID:** `ea413a4` (the `render-page` metrics contract
made whole and gated — `87 of 87`, now `90`), and a **sequence of repair commits
against one README** — `1af75a1`, `4af0c08`, `641deb9`, `39b449f`, `a68498b`,
`bc6c818`, `8522167` — plus `1a60e73` (librarian hard rule 11 amended). The
sequence is **finished**; what it taught is §6, and the full round-by-round record
is in `ROADMAP.md` and `SESSION_LOG.md`, which is where it belongs.

### What is OPEN, in one list

1. **`R214`'s positional-reference gate** — recommended, unbuilt, **baseline
   unmeasured**. A grep over a closed vocabulary (*of those*, *the above*, *this
   slice*, *the next slice*, *the former*, *the latter*, *as above*, *see below*)
   in doc comments. **Measure, repair, then wire. Never wire it red**
   (`D:/dev/rag/rust/ci_gate_red_at_baseline_enforces_nothing.md`).
2. **`R216`'s companion gate** — recommended, unbuilt, **baseline unmeasured**.
   Same instrument, different vocabulary: *does any file outside the append-only
   set contain* `this paragraph read`, `this line read`, `this item read`,
   `until 2026-`, `kept per R215`? Closed vocabulary, known file set.
3. **`R215`'s retro-application** — not started. Any Pass filed with a
   *"required after"* column must be re-read against `R215` before that column is
   used as a gate. Runs over `docs/` **and both RAG tiers**.
4. **The engineering queue** — §4, in order: `97.1g`, `97.1k`, `122.2`, `122.1`,
   `122.0` (threading, the operator's own request), `119.1`, `122.3`.

**The `tools/gen-scale-demo/README.md` repair list is EMPTY.** Nothing is
recorded against that file.

### ★★ THE TWO ITEMS THAT ARE KEN'S, AND THEY ARE THE ONLY THINGS AWAITING HIM

Both are his acts by definition, not an agent's. **Both figures were measured on
2026-08-23 by the commands named beside them — re-run them, do not quote them.**

- **PUSHING.** `git rev-list --count origin/main..main` → **`32`**. The local
  `origin/main` tracking ref is at **`c24ad7a`**; **no `git fetch` was run**, so
  that is the tracking ref's position, **not a live query of the remote**.
  `git remote -v` → `origin  https://github.com/KenM76/pdfce.git`. Pushing needs
  a **current** go-ahead (`CLAUDE.md` rule 8), and **the repository is public —
  an agent must not push.**
- **CUTTING A BACKUP.** The newest bundle in `D:/Dev/pdfce-backups/` is
  `pdfce-20260817-v060.bundle` (2026-08-17 20:34), whose `refs/heads/main` is
  **`3c4c00e`** (`git bundle list-heads`). `git rev-list --count 3c4c00e..main`
  → **`225`**, and `git merge-base --is-ancestor` confirms **`HEAD` is not in
  it.** ⇒ **225 commits and six days behind.** `v0.7.0`'s tag is in **no** bundle
  on disk.

### Ledger at end of session

Next free Pass in the `74` family **`74.11`**; `122.4` the next free `122`;
**`97.1g` reserved and unbuilt**. Decisions **`084`**, next free **`085`**.
Standing rules **`R216`**, next free **`R217`** — `R216` minted 2026-08-23;
`R211` gained clauses (d)/(e), `R214` clause (d) and `R215` clause (d) earlier
the same day. Operator questions **`(br)`, CLOSED**, next free **`(bs)`**.
`render-page` metrics line at **`90` keys**. **17 `tools/check-*` on disk, 16
runnable as bare gates.** Filing ordinal **239**.

---

## §1 — WHAT `PASS 74.10` DID, AND WHY IT MATTERS MORE THAN ITS SIZE

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

---

## §2 — ★★★★★ READ THIS BEFORE WRITING ANY DIFFERENTIAL TEST — `R211` CLAUSE (e)

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

## §3 — THE PRE-FLIGHT CHECKS, unchanged and still earning their place

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
**Measured 2026-08-23 by `ls tools/check-* | wc -l` and by running each: 17
scripts on disk (12 `.py` + 5 `.sh`), 16 runnable as bare gates.** The 17th,
`check-image-colorspace-truth.py`, exits `1` on a bare invocation **because it
takes a fixture-directory argument** and is not a gate. **Count them; do not
quote a count.**

**3. Read `docs/compositor-plan.md`** before scoping anything in `97.x`.

**4. If `check-commits-filed.py` is red when you start, READ ITS OUTPUT FOR THE
HASH.** Do not assume which commit it means, and **never extend
`tools/commits-filed-baseline.txt`** — that file is pre-existing debt, not an
allowlist.

---

## §4 — ★★ THE GHENT PASS COUNT IS AN OVER-COUNT. Read this before quoting any figure.

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

## §5 — THE QUEUE, in the order I would take it

**Nothing in this queue is blocked on anybody.**

1. **Build `R214`'s positional-reference gate** — a grep over a closed
   vocabulary (*of those*, *the above*, *this slice*, *the next slice*, *the
   former*, *the latter*, *as above*, *see below*, *the previous field*) in doc
   comments. **Measure its baseline first, repair, then wire** — never wire it
   red. ★ `R216`'s companion vocabulary (§A item 2) rides the same instrument;
   build one script with two vocabularies rather than two scripts.
2. **`Pass 97.1g`** — non-isolated ordinary groups on a subtractive page are
   composited as if isolated. The arithmetic exists (`remove_backdrop_cmyk`);
   the second content walk does not. **A port of the additive path, not a
   design.**
3. **`Pass 97.1k`** — native colorant paths for images and shadings, which
   bridge through sRGB today (`cmyk_bridged_pixels`).
4. **`Pass 122.2`** — teach `ghent-check.py` the check-mark criterion and give
   its contrast floor an area term. §4 says why this is not optional.
5. **`Pass 122.1`** — per-sample image overprint. Now **diagnosed**: it is why
   `GWG 8.2`'s check mark is missing. The mark is painted in yellow *underneath*
   the images, which overprint it so cyan-over-yellow reads green; pdfce paints
   the image normally and covers it (`overprint_images_unsupported = 2`).
   pdfium fails it too.
6. **`Pass 122.0`** — multithreading, the operator's request. His design (a
   runtime max-cores setting) is right and is kept; decision 080 adds a
   **compile-time target gate**, because `std::thread` and `rayon` both
   `cargo check` cleanly for `wasm32` and the CI wasm job therefore cannot catch
   a threading regression. Hard acceptance criterion: **byte-identical output at
   any core count.** ★ **The banana page is a genuinely useful threading
   benchmark**, because its cost is ~1 000 000 path operators in 342 forms
   rather than one big image. ★★ **Read BOTH §2 and `R215` before writing its
   acceptance table** — *"byte-identical at any core count"* is a **bound**,
   which is the right shape (`R215`), **and it is a differential claim over an
   unbounded parameter, which is `R211` clause (e)'s exact subject.** Sample
   across the switch points (1, 2, `n-1`, `n`, oversubscribed), not across "a
   few core counts".
7. **`Pass 119.1`** — `unshare_form`. Carried unstarted through nine handoffs.
8. **`Pass 122.3`** — the colorant buffer's byte ceiling. Interactive use is
   unaffected, but a **full-page** render above ~375 DPI refuses the buffer and
   silently composites in the wrong space, so one page can have different
   colours at different resolutions.

---

## §6 — STANDING NOT-DONE LIST, named so it does not read as done

These are known gaps in shipped behaviour. None is a regression; each is a
capability pdfce does not have yet.

- **`resolve_indexed`** builds its palette with a **scratch `ColorDiagnostics`
  that is discarded**, so a tint failure inside a palette never reaches the
  operator.
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

## §7 — LESSONS FROM THIS RUN THAT ARE CLASSES, NOT INCIDENTS

1. **★★★★★ TWO PATHS THAT MUST AGREE SHARE THE PREDICATE THAT DECIDES WHEN THEY
   CAN — THEY NEVER EACH PICK A LIMIT.** §1 has the instance and the reason.
   Two constants that match today drift apart on any later edit, and **the band
   between them emits nothing**. `decision 084`, `R211` clause (d).
   `D:/dev/rag/rust/two_paths_that_must_agree_share_the_predicate_that_decides_when_they_can.md`
2. **★★★★★ A DIFFERENTIAL TEST PROVES AGREEMENT ONLY OVER THE RANGE IT SAMPLES.**
   §2 has the instance and the three-way table that keeps it apart from `R162`
   and `R215`. `R211` clause (e).
   `D:/dev/rag/rust/a_differential_test_proves_agreement_only_over_the_range_it_samples.md`
3. **★★★★ A FIX CAN CREATE THE DEFECT ITS OWN FILING THEN FINDS.** `74.7` was
   correct, tested, measured and shipped — **and it opened a divergence with a
   path it never touched.** ⇒ **when a change moves one implementation of a
   duplicated contract, the other implementation is part of the change**, whether
   or not it appears in the diff.
4. **★★★ AN EDIT THAT SILENTLY DROPS ONE HUNK OF A MULTI-HUNK CHANGE IS WORSE
   THAN ONE THAT FAILS LOUDLY, BECAUSE THE OTHER HUNKS LAND** and lend the
   missing one their credibility. A heredoc ate a line-continuation backslash and
   the README **contradicted itself a few lines apart**. **The check is to re-read
   the ARTEFACT, not the diff** — the diff showed exactly what was applied and
   looked fine. See
   `.claude/agent-memory/pdfce-engineer/feedback_windows_paths_need_literal_edits.md`.
5. **★★★ A REFUSAL THAT FALLS BACK COSTS PERFORMANCE; A REFUSAL THAT CHANGES THE
   OUTPUT COSTS FIDELITY.** Log the first through the error's own message; give
   the second its own counter. `74.10` adds **no** metrics key and that is
   deliberate; `74.9`'s `subpixel_culled` needed one. `decision 084` vs `083`.
6. **★★ A REFUSAL'S *KIND* IS PART OF ITS CONTRACT.** A capability refusal is a
   property of the **document** (*"this page uses an operator I cannot record"*);
   a precision refusal is a property of the **request** (*"this scale is one I
   cannot record truthfully"*). A caller that cached *"this page is not
   recordable"* would now be wrong — the same page records fine at a lower scale.
7. **★★ FILE A FIGURE BESIDE ITS OPERANDS AND IT BECOMES CORRECTABLE WITHOUT
   RE-MEASUREMENT.** *"23× faster … `93 s` to `1.3 s`"* is **self-refuting on its
   face** because the ratio and its operands are both written down. The
   provenance, settled: at `100 000×`, **`31 s`** = the same region with algorithm
   B disabled entirely (**the baseline**); **`93 s`** = the **rejected
   device-space attempt**; **`1.3 s`** = shipped. ⇒ `23.8×` is the speed-up,
   `3×` is the rejected attempt's penalty, and **`93 s → 1.3 s` = `71.5×`
   compares the shipped result against a rejected implementation rather than a
   baseline.** `crates/pdfce-render/src/gstate.rs` states it correctly at source.
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
    expectation invites checking; a result does not.* ⇒ **THE CORRECTION IS THE
    ARTEFACT, NOT THE CORRECTED VALUE** — record *"this said **X**; **X** is
    impossible because **Y**; it is **Z**"* and **keep X visible**. `R215` (d).
    `D:/dev/rag/rust/a_number_identified_as_wrong_in_an_expectation_comes_back_as_a_measured_result.md`
11. **★★★ A MINTED RULE PROTECTS THE DOCUMENT IT WAS MINTED IN, NOT THE ONES
    THAT QUOTE IT.** **Minting a rule about a number is itself a meaning-change
    event** and owes a sweep across **every** tree, not just the one holding the
    rule. Now librarian hard rule 11.
12. **★★ AN ACCEPTANCE ORACLE BUILT FROM THE BROKEN SYSTEM'S OWN OUTPUT ENCODES
    THE DEFECT AS THE REQUIREMENT.** `R215` (a)–(c).
13. **★ A FLOAT-PRECISION TEST WITH ROUND-NUMBER OPERANDS CANNOT FAIL.** Two
    **equal** large `f32` values cancel **perfectly**.
14. **★★ A REFERENCE BY POSITION CANNOT TELL YOU WHETHER IT IS BROKEN OR WRONG,
    AND REPAIRING ONE POSITIONALLY JUST MAKES ANOTHER.** `R214` — name the
    referent; repair at the referent. **Promote a prose pre/post boundary to a
    section heading**, because a heading cannot drift: the thing it delimits is
    the thing that would have to move it.
15. **★★★★ A STRUCTURAL REPAIR EMITS A NEW CLAIM OF A DIFFERENT KIND — A SCOPE
    STATEMENT — AND IT IS A UNIVERSAL QUANTIFIER NOBODY HAS CHECKED.** Promoting
    a boundary to a heading removes positional **drift** and buys **nothing**
    against **over-scoping**. `R214` clause (d). ⇒ **when you convert prose into
    structure, re-read the structure's own boundary statements as claims.**
16. **★ A limitation justified by "beyond any plausible use" is a PREDICTION,
    not a fact** — `R193`. **An error curve measured over the range you consider
    plausible cannot tell you the range is plausible** — §2's finding from the
    other direction.

### ★★★★★ 17. THE ONE TO READ BEFORE REPAIRING ANY DOCUMENT IN THIS PROJECT

**A sequence of repair commits ran against one README and ended by DELETING the
layer that was generating the defects instead of correcting it an eighth time.**
The measured result:

- **Zero defects were ever found in the engineering content.** The arithmetic,
  the three `f32` limits, the drift formula and the measurement table's values
  were correct from the first round and never moved.
- **Every defect lived in a sentence ABOUT a previous correction** — a running
  annotation of the file's own edit history, in which each new sentence was an
  unchecked claim about a **set**, a **position** or a **count**.
- **`n=4` rounds emitted a fresh defect of the class they were repairing**, and
  one of those was in the librarian's own correction footer.

⇒ **A DOCUMENT THAT ANNOTATES ITS OWN CORRECTIONS HAS MADE ITS CORRECTION
HISTORY PART OF ITS CONTENT — and that part has no tests, no measurements and no
reader who would notice.**

★★ **The sharpening you act on: the hazard is SELF-REFERENCE, not HISTORICITY.**
*"Three post-fix columns"*, *"§9's last paragraph"*, *"Everything in this
section"*, *"two lines above"* all depend on the document's **current** contents
and must be re-verified on every edit. *"The acceptance criterion was `11 of
11`"* is **static** — true when written, unfalsifiable by any later edit. **A
per-form rule cannot close the hole**, because each names one syntactic form and
the supply of forms is not enumerable. **Placement is enumerable; syntax is not.**

**⇒ WHAT TO DO, in four lines.**
**(1) Edit history and preserved wrong wording go in `ROADMAP.md` and
`SESSION_LOG.md`** — append-only, dated, read by someone looking for exactly
that. **An overwritten document carries a pointer, or nothing.** That is
**`R216`**, and this handoff is its second founding instance.
**(2) Prefer DELETION to a rewording** — a correction to a self-referential claim
is drawn from the **same distribution** as the defect, so a rewrite is a fresh
sample of it. The boundary: **preserve wrong VALUES, delete wrong POINTERS** — a
self-referential claim's whole content is a coordinate, so nothing is lost.
**(3) An ORDINAL is a claim; a SEQUENCE is not. A DISTANCE is a claim; a RELATION
is not.** Write *"first values, then pointers, then quantifiers"*, never *"round
3"*; write *"is falsified by it"*, never *"two lines above"*. **And when you
delete ordinals, delete the cardinal too** — *"six rounds"* is still a number
somebody has to maintain.
**(4) A DISPOSITION is a claim about current state and it ages like one.** *"Not
repaired"*, *"unbuilt"*, *"carried forward unstarted"* — **attach the date and
the hash** (*"not repaired as of `<hash>`"*), which makes it static.

Full text: `ROADMAP.md`'s *Update protocol* → *"How you know a document is
converging"* (original + three amendments), standing rule **`R216`**, and
`D:/dev/rag/rust/a_document_that_annotates_its_own_corrections_has_made_its_edit_history_part_of_its_content.md`,
whose sibling
`D:/dev/rag/rust/the_defect_class_left_after_repeated_repair_rounds_is_the_stopping_signal.md`
carries the review-instrument half.

---

## §8 — HOUSEKEEPING

**Every figure below was MEASURED on 2026-08-23 by the command named beside it
(librarian hard rule 8). Re-run them; do not quote these lines.**

- **`git remote -v`** → `origin  https://github.com/KenM76/pdfce.git`;
  **`git rev-list --count origin/main..main` → `32`**; `origin/main` at
  **`c24ad7a`**. **No `git fetch` was run** — that is the tracking ref's
  position, not a live query of the remote. **The repository is public, so
  anything committed is published by default.**
- **Backups `225` commits and six days behind `HEAD`.** Newest bundle
  `pdfce-20260817-v060.bundle` (2026-08-17 20:34), `refs/heads/main` at
  **`3c4c00e`** (`git bundle list-heads`); `git rev-list --count 3c4c00e..main`
  → **`225`**; `git merge-base --is-ancestor` confirms **`HEAD` is NOT in that
  bundle**. `v0.7.0`'s tag is in no bundle on disk.
- **Worktrees: `9` entries**, by `git worktree list | wc -l`. **★ They are OUT OF
  SCOPE for every sweep** — each is a transient detached copy of `docs/` and
  `crates/`, stale by construction. `rg` skips them (gitignored); a bare
  `grep -r` does not. **`rg` is the correct instrument**, so a future filing does
  not "discover" hundreds of survivors there and report a crisis.
- **Gates: `17` `tools/check-*` on disk, `16` runnable as bare gates; all `16`
  exit `0` after this filing's commit.** Before it, `check-commits-filed.py`
  exited `1` naming exactly `8522167`, which is the gate working as designed.
- **A commit may bundle doc-comment repairs; a FILING may not bundle code**
  (`R198`). The second direction is the one that is easy to miss: a librarian
  filing carrying a code change cannot file itself and manufactures one more
  unfiled commit.
