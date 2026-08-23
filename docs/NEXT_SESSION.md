# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. Overwrite it once acted on.

**REWRITTEN 2026-08-23 by `pdfce-librarian` (two-hundred-and-thirty-third
filing).** **`PASS 74.10` shipped** (`d8f3020`, plus `1af75a1` with no Pass ID).
**The previous handoff's queue item 1 — *"check this FIRST, because it is the
only item that could be a live defect rather than new work"* — was a live
defect, and it is closed.** **`decision 084` minted; `R211` amended (clauses (d)
and (e)); no new standing rule — the ceiling stays `R215`.** **Nothing is
awaiting Ken except pushing and cutting a backup**, both of which are his acts,
not an agent's.

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
*(★★ **Measured 2026-08-23, 233rd filing, by `ls tools/check-*` and by running
each: 17 scripts on disk (12 `.py` + 5 `.sh`), 16 runnable as bare gates, ALL 16
EXIT 0.** The 17th, `check-image-colorspace-truth.py`, exits 1 on a bare
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

**Ledger after this filing:** next free Pass in the `74` family **`74.11`**;
decisions **084**, next free **085**; standing rules **`R215`** (ceiling
**unmoved** — `R211` was **amended**, not superseded), next free **`R216`**;
operator questions ceiling **`(br)`, CLOSED**, next free **`(bs)`**;
**`render-page` metrics line at `90` keys, unchanged**, measured by running
`tools/check-metrics-line-contract.py`.

---

## §4 — THE QUEUE, in the order I would take it

**The previous item 0 and item 1 are both gone** — `(br)` is answered and
shipped, and the display-list precision question turned out to be a live defect
and is fixed. **Nothing in this queue is blocked on anybody.**

1. **★ Repair `tools/gen-scale-demo/README.md` §9 — THREE stale claims, and the
   first one is `R215`'s own wrong number wearing a result's clothes.** Details
   in §5. Small, and the first item is the kind of error this project has now
   minted a rule about.
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

**★★★ `tools/gen-scale-demo/README.md` §9 CARRIES THREE STALE CLAIMS.** All
three were found by **reading** the section, not by grep — the 233rd filing read
it precisely because the failure `1af75a1` repairs is a **silently lost edit**,
and the only check for that is reading the artefact rather than the diff.
**Reported, not edited — the librarian does not touch `crates/` or `tools/`.**

1. **★★★ `R215`'s FOUNDING WRONG NUMBER HAS PROPAGATED FORWARD AS A MEASURED
   RESULT — fix this one first.** §9's *"What changed"* paragraph reads: *"The
   table above is superseded: **11 of 11 forms survive at every scale tested**."*
   **The correct post-fix answer is `2 of 11` from `1.25 × 10⁷` up** — the box
   plus the one molecule actually in frame; the other nine leave the viewport
   and **`Pass 74.4`'s `/BBox` cull removes them exactly** (ISO 32000-1
   §8.10.1). Both the `PASS 74.7` Shipped entry and the 232nd `SESSION_LOG.md`
   entry state `2` **and** state that `2` is correct. **`11 of 11` is the number
   `R215` was minted about, now dressed as an outcome rather than as an
   expectation — which is strictly worse, because an expectation invites
   checking and a result does not.**
2. **★★ A POSITIONAL REFERENCE THAT IS NOW WRONG RATHER THAN BROKEN — `R214`'s
   exact species.** §9's fix notice says *"Everything **below** describes the
   behaviour before that Pass … What follows is the evidence, not the current
   state."* **The second half of the section describes the CURRENT state** — the
   computed region, the `(1100, 732)` measurement, *"What changed"*, and the
   `1.6e9` result. The reference names nothing, so it cannot tell a reader it
   has gone stale. **Repair at the referent**: scope the sentence to the
   paragraphs it actually covers, or split §9 into *how it was found* and *what
   it is now*.
3. **★ THE `93 s` FIGURE IS SELF-REFUTING THERE.** §9's last paragraph reads
   *"the same change made deep zoom **23× FASTER**. A stroke-heavy CAD region at
   100 000× went from **93 s to 1.3 s**."* **`93 / 1.3 = 71.5`, not `23`** — the
   ratio and its own operands disagree **in one sentence**. With the provenance
   now known the correct before-figure is **`31 s`**. ⇒ **`31 s → 1.3 s`,
   `23.8×`.**

**★★ The `93 s` provenance itself is CLOSED** (it was an owed item here for one
filing). At `100 000×`: **`31 s`** = the same region with algorithm B disabled
entirely — **the baseline**; **`93 s`** = the **rejected device-space attempt**;
**`1.3 s`** = shipped. ⇒ `23.8×` is the speed-up; `3×` is the rejected attempt's
penalty; and **`93 s → 1.3 s` = `71.5×`, which the 232nd filing published as a
second speed-up, compares the shipped result against a rejected implementation
rather than against a baseline.** Dated amendments are in `ROADMAP.md` in both
places.

**Everything below this line is unchanged from the previous handoff.**

- **`R215`'s retro-application** — any Pass filed with a *"required after"*
  column should be re-read against the rule before that column is used as a
  gate. **Carried forward unstarted.**
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
10. **★★ AN ACCEPTANCE ORACLE BUILT FROM THE BROKEN SYSTEM'S OWN OUTPUT ENCODES
    THE DEFECT AS THE REQUIREMENT.** `R215`. ★ **And it can propagate FORWARD**:
    §5 item 1 is that same `11 of 11` now written as a *result*.
11. **★ A FLOAT-PRECISION TEST WITH ROUND-NUMBER OPERANDS CANNOT FAIL.** Two
    **equal** large `f32` values cancel **perfectly**.
12. **★ A reference by POSITION cannot tell you whether it is BROKEN or WRONG.**
    `R214`. Name the referent; repair at the referent. §5 item 2 is a fresh
    instance.
13. **★ A limitation justified by "beyond any plausible use" is a PREDICTION,
    not a fact** — `R193`. **An error curve measured over the range you consider
    plausible cannot tell you the range is plausible** — which is §0.5's finding
    from the other direction.

---

## §7 — HOUSEKEEPING

**All four figures below were MEASURED on 2026-08-23 by the commands named
beside them (hard rule 8). Re-run them; do not quote these lines.**

- **`origin/main` is at `c24ad7a`, and `main` is `18` commits ahead** —
  `git remote -v` → `origin  https://github.com/KenM76/pdfce.git`;
  `git rev-list --count origin/main..main` → **`18`**. **No `git fetch` was
  run**, so that is the local remote-tracking ref's position, **not a live query
  of the remote.** Pushing is the operator's act and needs a current go-ahead
  (`CLAUDE.md` rule 8). **The repository is public, so anything committed is
  published by default.**
- **Backups are `211` commits and six days behind `HEAD`.** Newest bundle
  `pdfce-20260817-v060.bundle` (2026-08-17 20:34) with `refs/heads/main` at
  **`3c4c00e`** (by `git bundle list-heads`);
  `git rev-list --count 3c4c00e..main` → **`211`**;
  `git merge-base --is-ancestor` confirms **`HEAD` is NOT in that bundle**.
  `v0.7.0`'s tag is in no bundle on disk. **Cutting one is the operator's
  call** — and it is now the *only* outstanding item that is his rather than an
  agent's, alongside pushing.
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
