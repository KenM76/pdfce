# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

**REWRITTEN 2026-08-24 by the engineer.** Per standing rule `R216` this file
carries **no edit-history layer**: no *"this paragraph read X until…"*. What is
true now, plus a pointer. Corrections and their prior wording live in the
**append-only** record — `ROADMAP.md` and `SESSION_LOG.md` — where a claim is
dated and no later edit can falsify it.

---

## §A — COLD START: everything you need, in one screen

**`HEAD` is the librarian filing for `Pass 122.2`. The working tree is clean.
Nothing is blocked on anybody. Both FeatureRequests channels were checked and
hold nothing newer than 2026-08-21.**

**★ THE HAZARD THAT DOMINATED THE LAST HANDOFF IS GONE, AND NOT BY THE ROUTE IT
RECOMMENDED.** Do not re-derive it. Details in §0 — read it once, then stop
thinking about release ordering.

### What this run built (2026-08-24)

Three pieces of work, in the order they landed:

| commit | what an operator would notice |
|---|---|
| `bb154ed` | **CI can be green again.** Both filing gates had been asking each commit to cite a hash that would not exist until a *later* commit — unsatisfiable by construction. The tip is now DEFERRED and disclosed, never failed |
| `c4a85d0` | **`Pass 97.1g` — a non-isolated group on an ink page can finally see what is under it.** §11.4.4's second content walk on a subtractive page. ★ **Its headline result is NEGATIVE and that is the honest framing: it changes no rendering in any corpus pdfce owns** |
| `f6457ee` | **`Pass 122.2` — the Ghent harness stops scoring four patches it never examined.** Board `29 pass` → `24 pass` harness-reported. **Its own conclusion that standing was 25 was superseded hours later by `122.4` — it is 26; see §A** |

### ★★★★★ THE MOST IMPORTANT THING ON THIS PAGE — ONE ROOT CAUSE UNDER 24 OF 51 PATCHES

**Found at the end of the session by chasing `Pass 122.4`. No code shipped;
this is a measurement, and it should set the next session's priority.**

`pdfce` composites a page in its **ink (colorant) buffer only when the page
declares `/Group /CS /DeviceCMYK`.** Verified directly, not inferred: the
individual Ghent patch files contain **zero** `/Group` occurrences and **do**
carry an `/OutputIntent`; the combined suite document contains **217**.

Consequence, measured three ways on `GWG 1.1`'s OPM-1 swatch:

| render | the X the suite traps on |
|---|---|
| pdfce, **individual** patch (PDF/X-3, no `/Group`) | **solid, contrast 18.4** |
| pdfce, same artwork in the **combined** doc (PDF/X-4, has `/Group`) | **none** |
| **Adobe Acrobat**, individual patch | **none** |

⇒ Acrobat agrees with the combined render. **pdfce's individual-patch render
is the outlier and is wrong**, so `GWG 1.1` PASSES and the operator's own
review was right about it. Standing moves to **26**.

**★ And the population is the whole remaining failure set.** Across all 51
patches, **24 request overprint and receive no colorant buffer**
(`cmyk_buffer=0` with `overprint_requested>0`); only 13 get one. Those 24
contain **every remaining Ghent FAIL** and **all four `MARK?` patches**. The
board's long-standing note about *"the ONE architectural item that unblocks
the largest cluster"* now has a named, measured mechanism — and it is very
likely **one fix, not twenty**.

**⇒ THE SPEC READING IS DONE AND IT CHANGES THE SHAPE OF THE WORK. Read
this before touching `Pass 122.5`.**

★ **pdfce IS CONFORMING TODAY.** ISO 32000-1 §11.4.7 and §11.6.3 each state
independently that *"if not otherwise specified, the page group's colour space
**shall** be inherited from the native colour space of the output device"* —
determinate, `shall`, no hedge — and `/OutputIntent` is **absent from the 1.7
transparency model entirely** (measured, not assumed). §8.6.7 then *prescribes*
the degenerate branch pdfce takes: *"source colours **shall** be converted to
the device's native colour space, and **all components participate in the
conversion, whatever their values**."* ⇒ Today's behaviour is **conforming but
degenerate**, never "unspecified". Staying put would be legitimate, and would
make the Acrobat divergence a **disclosable deviation to record rather than
repair**.

★ **ISO 32000-2 is where it opens, and only informatively.** §11.4.7 inherits
from the *"actual, **assumed or simulated**"* device and says the processor
**can** choose; **Annex P is informative** and offers *"from the output device,
**or** from the output intent"* with **no ranking, no condition, no
precedence**. The only body-text rung is §10.8.3(a), a **`should`**, and it
selects a *colourant set*, not a blending space. ⇒ Two conformant 2.0
processors render the same file in two different spaces and both cite Annex P.

⇒ **THAT IS A SETTING, AND THE DEFAULT WAS MINE TO PICK.**
`page_blend_space_source` ∈ `device_native` | `output_intent_if_subtractive` |
`output_intent_always`, **defaulting to `output_intent_if_subtractive`** —
chosen on the operator's own stated criterion, *"what would be normally
expected"*: a print file should render the way Acrobat renders it. Rule 4's
disclosure rides with it — **name the blending space and its provenance
off-canvas** (status line / CLI line); draw nothing on the page.

★★ **THE OPERATOR QUESTION I OPENED FOR THIS WAS WITHDRAWN, AND THE REASON IS
WORTH MORE THAN THE ITEM.** I filed it as `(bs)` for Ken to rule on. He has
removed exactly this class from his queue **twice** — *"do not ask me for the
default as you know more about this than I ever will"* (2026-08-19) and *"make
things work both ways as options, default it to your best guess"* (2026-08-20).
I had both in memory and opened it anyway, because a 24-patch cross-cutting
finding **stops looking like a spec ambiguity and starts looking like an
architecture decision**. ⇒ **The costume is the failure mode, not ignorance of
the rule.** The test is *"do two defensible answers exist?"*, never *"does this
feel big?"*

★★★ **AND ONE CORRECTION THAT CHANGES THE CODE, NOT JUST THE PROSE:
PDF/X-1a AND PDF/X-3 FORBID LIVE TRANSPARENCY.** Most of the 24-patch
population is `_x3` / `_x1a`, so those files have no transparency, never select
a blending space, and their overprint is an **OPAQUE-MODEL** question —
**§8.6.7 and Table 148**, *not* §11.7.4.3 / Table 149. Same n-colorant buffer
either way, **different citation, and the citation is what goes in the doc
comment.**

★★★★ **DO NOT ATTEMPT A CHEAPER FIX — overprint in an additive space is
STRUCTURALLY UNREPRESENTABLE, not merely unsimulated.** §11.7.4.3's second
bullet: `B(c_b, c_s)` is `c_s` for all components *"specified in the current
colour space"*, otherwise `c_b`. In sRGB every source colour has already been
converted to all three components ⇒ every component is "specified" ⇒
`B = c_s` **everywhere**. No shader work fixes this; only an n-colorant buffer
does. This forecloses a whole family of cheaper-looking attempts.

**Also settled:** for the X-3 patches the decision is **structural (how many
colorants), not colorimetric** — the trap X can be made correct with a nominal
CMYK and no ICC transform at all. And §11.7.4.3's OPM-1 predicate names *"the
current colour space **and group colour space**"* and **never the output
device**, so a `DeviceCMYK` page group satisfies it on an RGB display; the spec
corpus had recorded that as `SP-A3`, dormant since 2026-08-08, and this is the
case that fired it.

**Spec deliverables — cite these, do not re-derive:**
`D:\Dev\Rag-Specialized\PDF_Spec\iso32000\iso32000__ref__page_group_absent_blending_space.md`
(`PGB-1`…`PGB-14`, `PGB-N1/N2`, `PGB-A1`…`PGB-A4`) and
`D:\Dev\Rag-Specialized\PDF_Spec\pdfx\pdfx__ref__transparency_blending_space.md`
(`PX-1`…`PX-11`, `PX-N1`…`PX-N3`). **No ISO 15930 clause number is asserted
anywhere in that corpus and none may be added from recall** — the standard is
paywalled and all three free routes failed. Two follow-ons are recorded rather
than re-derivable: `PGB-A2` (*which* output intent, when there are several —
same shape as the existing `SEP-A1`, **do not solve it twice, differently**)
and `PGB-A4` (the answer is **edition-dependent**; consider one
`pdf_semantics_edition` knob rather than three).

**★★ HOW IT WAS FOUND, because the method is the transferable part.**
`Pass 122.2` recorded the harness-versus-operator disagreement on `GWG 1.1`
as *"disputed, left open in both directions"* rather than picking a winner.
**That refusal is the only reason this exists.** Had either side been declared
right, the 24-patch root cause would still be undiscovered. ⇒ **When an oracle
and an instrument disagree, the disagreement is DATA; resolving it by
authority discards the signal.**

### ★★ THE THREE THINGS MOST LIKELY TO BE MISREAD

1. **`cmyk_groups_approximated` went 118 → 0 and that is MOSTLY A COUNTER
   GETTING HONEST.** Of the 118 the old code counted across the Ghent suite,
   **13** actually needed the second walk; the other **105** rendered exactly
   right and were counted anyway, because the test asked *"is this group
   non-isolated?"* instead of §11.4.4 NOTE 2's *"does its interior read the
   backdrop?"*. **Do not read that drop as 118 groups repaired.** The CLI
   metrics-key table now carries the warning at the key itself.
2. **The Ghent board went DOWN, and that is the instrument improving.**
   `10 FAIL / 29 pass / 12 unresolved` → **`11 FAIL / 24 pass / 16
   UNRESOLVED`**. One patch was always failing and the floor hid it; four were
   never examined at all and said `clean`. **A falling number here is the
   harness telling the truth for the first time**, not a regression.
3. **`Pass 97.1g` ships a capability nothing exercises.** Measured against a
   pre-fix binary: **0 differing pixels across all 51 Ghent patches**; **0
   differing renders across the 114 external-corpus PDFs that mention
   DeviceCMYK**; and of 4012 external PDFs, **zero** contain both
   `Transparency` and `DeviceCMYK`. The only evidence it works is four
   purpose-built fixtures. That is stated plainly rather than dressed up.

### Ledger at end of session

Next free in the `97.1` family: **`97.1k`** (filed Backlog, unbuilt).
`122.0`–`122.5` all filed; **`122.4` and `122.5` are NEW this session**, next free **`122.6`**. Operator question **`(bs)` was opened and WITHDRAWN the same day** (see §A) — the letter is consumed, **next free `(bt)`**, and **nothing is awaiting Ken** except pushing and cutting a backup, both his acts.
`119.1` still unstarted. Standing rules unchanged — **no rule was minted this
session**, and one was *proposed and withdrawn* because `R143` already owned
the shape. Decisions unchanged. **Version `0.8.0`.** 17 `tools/check-*` on
disk, **16 runnable as bare gates; all 16 exit 0.** Ghent: **26 pass at
minimum** of 51 (harness-reported `24 / 11 / 16`; `GWG 1.1`'s dispute is
RESOLVED in the operator's favour — see §A).

---

## §0 — THE RELEASE HAZARD IS CLOSED. Read once; do not re-open.

The previous handoff's §0 said CI had been red for fourteen runs *"by
construction"* and that the durable remedy was to **make the release step tag
from a filing commit**. Both halves needed correcting.

**What was actually true.** Replaying the fixed gate at all fourteen red
`headSha`s: **three** were structural (the only hash printed was the tip —
runs `32679353296` at `08a88bd`, `32595460670` at `c24ad7a`, `32520914981` at
`6a2c13f`). The other **eleven** named genuinely unfiled *older* commits and
were **the gate working**. A gate doing real work eleven times out of fourteen
is a different situation from one that is red for nothing.

**What fixed it.** `bb154ed`. A commit cannot cite its own hash, so demanding
it is unsatisfiable — the same argument `check-commits-filed.py`'s own
docstring already made for docs-only filing commits, which nobody had noticed
applies to *any* code commit on its own CI run. The tip is now **deferred by
exactly one commit** and **printed on both the clean and the failing path**.
`--strict-tip` restores the old behaviour for the one caller that can satisfy
it: a librarian checking its own filing.

**⇒ The ordering discipline is no longer needed.** A tag on a code commit is
green whenever the history *behind* it is filed. `v0.7.0` got the ordering
right by discipline and `v0.8.0`, the very next release, regressed
immediately — **a rule living in a memory of an ordering is not a fix**, and
that is why this was solved structurally instead.

**Also corrected:** the previous §A said `HEAD` was `08a88bd` and the `v0.8.0`
tag object was `2ed21e4`. The tag had already been **moved to the librarian
filing commit and force-pushed** after that handoff was written — option (B)
from its own §0, taken without §0 being updated. `verify-release.py v0.8.0` is
now clean.

**Demonstration, on this session's own history:** `c4a85d0` is a Pass-claiming
code commit — the exact case that would have failed **both** filing gates that
morning. Both report clean, tip disclosed as deferred. And once `f6457ee` was
no longer the tip, the gate correctly flagged **it** as unfiled real debt.

---

## §1 — THE PRE-FLIGHT CHECKS, unchanged and still earning their place

**1. `ls` BOTH FeatureRequests channels.** They are outside this repository, so
**no gate will ever contradict a stale sentence about them — including this
one.**

```
D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\
D:\Dev\FeatureRequests\iccce_FeatureRequests\open\
```

**2. Run the gates — `ls tools/check-*`, do not trust any list.** `R209`: *"all
gates green" names a set, and the set somebody runs is not the set CI runs.*
The 17th, `check-image-colorspace-truth.py`, exits `1` on a bare invocation
because it takes a fixture-directory argument and is **not** a gate. **Count
them; do not quote a count.**

**3. Read `docs/compositor-plan.md`** before scoping anything in `97.x`.

**4. ★ BEFORE ANY PACKAGING OR RELEASE BUILD, CHECK FREE SPACE.**
`df -h . && du -sh target target/debug`. On 2026-08-23 a packaging run died on
a full disk with `target/debug` at 103 GB. Delete **`target/debug` only** —
never `cargo clean`, which also drops the warm `target/release` you are about
to need. At the time of writing: **82 % used, 178 GB free**, `target` 6.6 GB.

**5. The Ghent corpus is at `D:\Dev\temp\ghent-patches` (51 patches), and
Acrobat reference renders of all 51 are at `D:\Dev\temp\acro-refs`.** The
second of those is the calibration set `Pass 122.2` used and the previous
handoff never mentioned. Neither is in the repository.

---

## §2 — ★★★★★ THE METHODOLOGY LESSONS FROM THIS RUN. Read before writing any test or fixture.

**1. A fixture that avoids a feature's precondition cannot tell a fix from a
no-op.** `Pass 97.1g`'s first two fixtures each rendered a plausible picture
and proved nothing. The second failure is the instructive one:
`needs_buffer = is_transparency_group && (!outer_is_neutral || isolated ||
knockout)`, so a non-isolated group under a **neutral** outer state is painted
**inline** — which is not an approximation of non-isolated semantics, it **is**
non-isolated semantics. The gap only exists where a buffer is **forced**. The
fixture used `/ca 1`, rendered identically before and after, and looked exactly
like a fix that did not work. **Every fixture in
`fixtures/synthetic/transparency/` now carries `/ca 0.5`, and the reason is
commented where the `0.5` is written.**

**2. A vacuous assertion beside a wrong literal is worse than no assertion.**
That same generator asserted `(catalog, pages, …) == (1,2,3,4,5,6,7)` — exactly
what its `add()` helper returns, so it could never fail — while the
dictionaries beside it spelled `/GSO 7 0 R` and `/GSI 8 0 R`, both off by one.
The outer graphics state picked up the interior's blend mode and the interior's
reference dangled. **It reads as verification.** (`R162`, in a fixture
generator rather than a test.)

**3. ★ A fix aimed at a MISDIAGNOSED cause is more dangerous than no fix,
because it consumes the suspicion.** The operator review's §3 said the Ghent
contrast floor *"has no area term"* and that the missed crosses were *"three
times the linear size"* of the calibration ones. Measured at the scale the
harness renders: **36–38 px versus 38 px. The same size.** Had the area term
been built as recommended, the patch would have gone on reporting `clean` with
a fix in place and a plausible reason to stop looking.

**4. ★ A detector keyed on an incidental property will confirm itself.** The
first check-mark detector keyed on the mark's **colour**, measured from
`GWG 8.2` (olive). On `GWG 8.01` it reported the mark PRESENT — by matching the
green end of that patch's gradient bar — while **both real marks were absent**.
**A false green produced by the fix for a false green.** `GWG 8.2`'s marks are
olive; `GWG 8.01`'s are dark green. It was thrown away rather than shipped,
which is why `122.2` ships no detector.

**5. A grep for a phrase finds a MENTION, not a criterion.** The review's
"seven patches use the positive criterion" came from grepping ReadMes for
*"check mark"*. Three of the seven mention it only while describing what the
failure cross is drawn **out of**. It is four.

**6. Still true from the last run, and still the nastiest:** a differential
test proves agreement only over **the range it samples**, and the range is part
of the claim (`R211` clause (e)). Sample across the **predicate**, not across
"a reasonable range".

---

## §3 — THE QUEUE, in the order I would take it

**Nothing in this queue is blocked on anybody.**

1. **`Pass 122.5` — UNBLOCKED AND FULLY DESIGNED. Start here.** The
   24-of-51 root cause in §A. The spec reading is done, the setting and its
   default are chosen, and §A carries the four constraints that shape the
   implementation — the opaque-model citation for `_x3`/`_x1a` files, the
   structural-not-colorimetric corollary, the `SP-A3` predicate, and the
   proof that no cheaper fix exists. ★ `Pass 122.4` is ANSWERED and needs
   nothing further; **`(bs)` is WITHDRAWN and must not be reissued**.
2. **`Pass 97.1k`** — native colorant paths for **images and shadings**, which
   bridge through sRGB today (`cmyk_bridged_pixels`, `cmyk_unbridged_images`).
   The last structural item in the `97.x` family. Means widening a type one
   layer up: `DecodedImage` holds a `Pixmap`; `ColorRamp::at` returns
   three-channel sRGB when the ramp is *built*. **Take this while `122.5` is
   blocked** — same subsystem, no dependency on the spec answer.
3. **The check-mark detector** — `122.2` deliberately shipped none. It must key
   on **presence relative to a reference render**, not on a hue (§2 item 4).
   Ground truth for all four patches is in `tools/ghent-check.py`'s docstring
   and `D:\Dev\temp\acro-refs` has the references.
4. **Build `R214`'s positional-reference gate** — a grep over a closed
   vocabulary (*of those*, *the above*, *this slice*, *the former*, *the
   latter*, *as above*, *see below*) in doc comments. **Measure its baseline
   first, repair, then wire — never wire it red.** ★ `R216`'s companion
   vocabulary rides the same instrument; build **one** script with two
   vocabularies.
5. **`Pass 122.1`** — per-sample image overprint. Diagnosed: it is why
   `GWG 8.2`'s check mark is missing. The mark is painted in yellow
   *underneath* the images, which overprint it; pdfce paints the image normally
   and covers it (`overprint_images_unsupported = 2`). pdfium fails it too.
6. **`Pass 122.0`** — multithreading, the operator's own request. His design (a
   runtime max-cores setting) is right and is kept; decision 080 adds a
   **compile-time target gate**, because `std::thread` and `rayon` both
   `cargo check` cleanly for `wasm32` and the CI wasm job therefore cannot
   catch a threading regression. ★★ **Read §2 item 6 and `R215` before writing
   its acceptance table** — *"byte-identical output at any core count"* is a
   differential claim over an **unbounded** parameter. Sample across the switch
   points (1, 2, `n-1`, `n`, oversubscribed). ★ The banana page is a genuinely
   useful benchmark: ~1 000 000 path operators in 342 forms rather than one big
   image.
7. **`Pass 119.1`** — `unshare_form`. Carried unstarted through **ten**
   handoffs now.
8. **`Pass 122.3`** — the colorant buffer's byte ceiling. Interactive use is
   unaffected, but a **full-page** render above ~375 DPI refuses the buffer and
   silently composites in the wrong space, so one page can have different
   colours at different resolutions.
9. **`R215`'s retro-application** — not started. Any Pass filed with a
   *"required after"* column must be re-read against `R215` before that column
   is used as a gate. Runs over `docs/` **and both RAG tiers**.

---

## §4 — STANDING NOT-DONE LIST, named so it does not read as done

Known gaps in shipped behaviour. None is a regression; each is a capability
pdfce does not have yet.

- **`resolve_indexed`** builds its palette with a **scratch `ColorDiagnostics`
  that is discarded**, so a tint failure inside a palette never reaches the
  operator.
- **Implicit knockout**: only explicit `/K true` is honoured. `/TK` defaults
  true (every text object), and `B`/`b` and shading patterns are knockout.
  **The one pdfce implements is the rarest.**
- **`/TR` on a soft mask** is read, counted, never evaluated.
- **`/AIS true`** is not distinguished from `/AIS false`.
- **Spot colorants** — four planes, not runtime `N`. Every remaining
  trap-criterion Ghent FAIL is in this bucket.
- **Per-paint rendering intent** (§11.7.5.3) — pdfce carries one per page. No
  corpus measurement of mid-page intent switching has been taken.
- **A non-isolated group whose SECOND buffer cannot be allocated** still falls
  back to isolated semantics — counted, disclosed, and now the *only* way a
  non-isolated group reaches `cmyk_groups_approximated`.
- **No GUI code path reads** `forms_culled`, `subpixel_culled`,
  `annots_out_of_scope`, `page_content_suppressed`, `render_page_region` or the
  display list, **and no GUI exposes `--fast-subpixel`.** GUI work is paused;
  recorded so the `[ ] gui` boxes in `FEATURES.md` are not mistaken for
  oversights.

---

## §5 — THE TWO OPERATOR ITEMS

- **PUSHING — NOT AUTHORISED for anything after `v0.8.0`.** Ken's
  *"Relese everything to github and include the banana pdf"* discharged the
  `v0.8.0` release and **does not carry forward**. `v0.8.1` needs its own
  go-ahead (`CLAUDE.md` rule 8). **Nothing from this session has been pushed.**
- **CUTTING A BACKUP — STILL NOT DISCHARGED.** A GitHub release is an offsite
  copy of one build and one PDF; it is **not a `git bundle`** and does not
  contain the history. **Re-measure before quoting** —
  `ls D:/Dev/pdfce-backups/`, `git bundle list-heads <newest>`,
  `git rev-list --count <bundle-head>..main`. Do not carry a number forward
  from any previous handoff; each was true at a different `HEAD`.
