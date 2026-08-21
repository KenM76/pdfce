# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. Overwrite it once acted on.

**Written 2026-08-21**, replacing the 2026-08-20 (evening) handoff whose
headline task — `Pass 97.x`, the colour compositor — **is where this session
went**, and which it re-scopes.

---

## §0 — DO THESE THREE THINGS BEFORE ANYTHING ELSE

**1. `ls` BOTH FeatureRequests channels.** They are outside this repository,
so **no gate will ever contradict a stale sentence about them** — including
this one.

```
D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\
D:\Dev\FeatureRequests\iccce_FeatureRequests\open\
```

At this handoff: pdfce's channel had nothing new since our own 02:14 reply.
**iccce's had TWO notes that landed at 03:04 and 03:15 — after the previous
handoff was written — and both were compositor input.** They were read and
used. That is the second session running where the `ls` found something a
document said was not there.

**2. Run the gates — `ls tools/check-*`, do not trust any list.** 15 today,
14 wired into CI (`check-image-colorspace-truth.py` takes a fixture dir and
is a sweep tool, deliberately out). All green at this handoff.

**★ `tools/check-ledger-numbers.py` was rewritten this session** and its
live ceiling moved **118 → 121**. If you read a Pass ceiling from a document
older than today, it is wrong by three families. Re-run the gate.

**3. Read `docs/compositor-plan.md`'s 2026-08-21 amendment** before scoping
anything in the `97.x` family. It carries the derivation that re-orders
Stage B, and §7's two struck-through items carry measured negatives that
change what is worth doing next.

---

## §0.5 — WHAT SHIPPED

| commit | |
|---|---|
| `7160819` | **`Pass 97.0a`** — `compositor.rs`; the four non-separable modes were blending against **imaginary white paper** |
| `9b49ca0` | **`Pass 97.0b`/`97.0c`** — non-isolated groups get their backdrop; §11.4.6 knockout implemented |
| `86a7b70` | **`Pass 97.0d`** — soft mask applied to the group **result**, not to each object inside it |
| `0d5fc29` | **`Pass 97.1`, first two deliverables** — `/Indexed` colorants, and the counter for images that never reach overprint |
| `05ba72a` | five stale comments + a user-facing message + the ledger gate, third instance of one class |

Plus librarian filings (216th), **`R208`** minted, **decision 077**.

### The numbers, because the headline one did not move

**Ghent board: `26 pass · 14 FAIL · 11 UNRESOLVED` — identical before and
after.** What moved underneath it:

| | before | after |
|---|---:|---:|
| total traps across failing patches | 67 | **55** |
| `1_GWG161` (non-isolated knockout) | 14 traps | **2** |
| `1_GWG1610` strip correlation | 0.576 | **0.962** |
| `1_GWG168` strip correlation | 0.725 | **0.978** |
| `1_GWG169` strip correlation | 0.905 | **0.986** |

Full-corpus render parity, 4,023 files, **run twice** — this branch and a
worktree built at `2e6bb83` — **identical** bucket for bucket, band for
band, same single unexplained file. That is the intended result and it is
why the *isolated* group composite still goes through
`tiny_skia::draw_pixmap`: pdfce's `f32` path computes the same function with
different rounding, so routing the already-correct case through it would
move every anti-aliased edge in the corpus and turn the parity gate into a
rounding detector.

---

## §1 — ★★★ THE FINDING THAT RE-ORDERS THE PLAN. Read before scoping `97.1`.

`docs/compositor-plan.md` §4 expects **seven patches** from Stage A. It got
zero patch verdicts, **and the group model is not why.**

Derived by hand on `1_GWG162`'s `Difference` cell, whose two operands are
printed in the file — X1 = `DeviceCMYK 0 1 0 0` (magenta), X2 =
`DeviceCMYK 0 0 0 1` (black), `/BM /Difference`, and the surround a correct
engine must produce is RGB `(0, 165, 79)`:

```text
§11.3.4 complement:  cb′ = (1,0,1,1)   cs′ = (1,1,1,0)
|cb′ − cs′|              = (0,1,0,1)
complement back          = (1,0,1,0) = DeviceCMYK 1 0 1 0 = GREEN
```

**That is the surround, exactly.** pdfce renders `(237,1,140)`; pdfium
renders `(202,29,108)`; both blend in RGB and both are wrong, differently.
The trap is authored on the **blending colour space**, and no amount of
group-model correctness reaches it.

★ **Every Ghent transparency patch declares `/Group /CS /DeviceCMYK` on the
PAGE** — including `3_GWG161`, whose own objects are `ICCBased` RGB. So the
blending space is CMYK for all of them regardless of what the artwork is
coloured in.

⇒ **§11.3.4's subtractive complement is `Pass 97.1`'s leading deliverable,
ahead of the spot planes.** And it needs a real colorant buffer: the
4→3→4 reconstruction is measurably lossy — `DeviceCMYK 0 1 0 0` painted and
recovered from the sRGB buffer comes back as `(0, 0.995, 0.409, 0.071)`, and
that `Y = 0.41` is not a rounding error.

---

## §2 — THE QUEUE, in the order I would take it

1. **`Pass 97.1` — the colorant buffer, led by §11.3.4.** The big one, and
   now the only thing that can move the transparency panels. `Pass 97.0`
   built the group model it plugs into. §3 below is what to build it from.

2. **`Pass 119.1`** — `unshare_form` (copy-on-write a shared form onto one
   page). Carried from the previous handoff, unstarted, still a *separate
   verb* and not a mode of `edit_text` (`decision 076`).

3. **`Pass 80.0`** (note text on markup) and **`Pass 81.1`** (markup
   opacity, write half) — both `pdfceGUI` requests, both already scoped,
   both untouched by this session.

4. **`Pass 119.3`** — align `pdfce-render`'s nested-form resource fallback
   with `text_edit::forms`.

5. **The reference-strip threshold for `ghent-check.py`** — see §5. Small,
   and it is the difference between three patches reading UNRESOLVED and
   reading what they are.

---

## §3 — WHAT `Pass 97.0` BUILT, so it is not rebuilt or mistrusted

**`crates/pdfce-render/src/compositor.rs`** is now the single place the
standard's compositing arithmetic lives: §11.4.4's element formula,
§11.4.8's knockout variant, §11.4.4's backdrop removal, §11.3.7.3's `Union`,
and all thirteen Table 136 separable blend functions — transcribed **with**
the corpus's four printing errata (`GD-1`…`GD-4`: `ColorDodge`'s and
`ColorBurn`'s second branches, `SoftLight`'s `D(x)`, and `Difference`'s
missing absolute-value bars, which every text extractor loses because they
are path-drawn).

**`Canvas::group`** replaces `Canvas::layer` for transparency groups.
`Canvas::layer` stays, unchanged, for an annotation's `/CA` — that one
really is "composite this sub-drawing as one object" over a transparent
start.

★ **The two-walk mechanism, and why it is not expensive.** The standard
needs two per-pixel quantities one `Pixmap` cannot both hold: `C_n`
(accumulated **over** the backdrop) and `α_gn` (the group's own alpha,
**excluding** it). So a non-isolated group's contents are walked twice. The
second walk is **skipped under §11.4.4 NOTE 5's own condition** — with the
interior compositing `Normal` throughout, the one-walk answer is
*identically* exact. On the operator's real CAD drawing the counter is zero
and the render time is unchanged at **0.96 s**.

**`KnockoutTarget`** — four planes where a `Pixmap` has one. Each element is
rasterised into a reused scratch **at full opacity**, so its alpha comes
back as pure coverage (`f_s`), with `q_s` taken out of the paint — because
§11.4.8 scales the destination by `(1 − f_s)` where the ordinary formula has
`(1 − α_s)`. §11.4.6 NOTE 6's nesting rule is honoured: a non-isolated group
inside a knockout group inherits the **outer** group's initial backdrop.

**The soft mask** is lifted out of the contents' clip at a group `Do` and
applied once to the composite. Folding into the clip is **still** what
happens to an elementary object and is **correct** there — §11.6.4.1 makes
the mask value that object's `q_m`. Two clauses, one implementation, not a
fix half-applied.

### Three counters whose meaning you must not misread

- **`transparency_groups_knockout_approximated` CHANGED MEANING.** It used
  to be `1` for every `/K true` group; it now counts the **elements** inside
  one that read the destination back and so could not be given knockout
  semantics. A knockout group pdfce renders exactly reports **zero**.
- **`groups_backdrop_reruns`** is a **cost** counter, not a shortfall — the
  only place in the renderer where one page's content stream is interpreted
  more than once. Zero is normal and does not mean non-isolated groups were
  mishandled.
- **`overprint_images_unsupported`** is new and deliberately **not**
  `overprint_refused`. `refused` = "offered and could not run"; this =
  "never offered this object class at all".

---

## §4 — ★★ TWO FIXES THAT MOVED NOTHING, AND WHY THAT IS THE USEFUL PART

Both were listed in `docs/compositor-plan.md` as measured-and-confirmed.
Both were real. Both are inert today.

**`/Indexed` colorant classification.** §8.6.6.3 puts an `/Indexed`
operand's colour values in the **base** space, and Table 149 keys on which
colorants the source *names* — so `/Indexed [/DeviceN [/Cyan] /DeviceCMYK …]`
was classified as "some other process space" and Table 149 decided what
survives from a colorant list it had never read. Fixed in three places (the
third — `overprint_would_change` — was **not** in the plan's write-up, and
it is the predicate that decides whether the composite is called at all).

**Measured A/B, pre- and post-fix binaries, same corpus:**
`overprint_effective` / `overprint_composited` / `overprint_refused` /
`overprint_pixels` **identical to the digit** on all four patches that carry
`/Indexed`. Cause, verified structurally: **every one of those spaces is an
IMAGE colour space and nothing else** (`1_GWG190`'s two are `/XO1` and
`/XO2`, both `/Subtype /Image`), and `overprint::composite` has no image
call site. **The plan listed the `/Indexed` half first and the image half
second; the dependency runs the other way round.**

**Stage A itself**, same shape at a larger scale — see §1.

★ **The transferable half, and it is now in the engineer's agent memory:**
a plan's *ordering* of two related items is a claim, and it is the claim
least likely to have been checked. Keep the previous binary
(`git worktree add /tmp/x <sha>` + `cargo build --release`, four minutes) —
it is the only thing that separates *"this fix did nothing"* from *"this fix
did nothing **yet**"*.

---

## §5 — AN INSTRUMENT PROBLEM, NAMED AND DELIBERATELY NOT SOLVED

`ghent-check.py` has **no calibrated threshold** for reference-strip
patches — its own output says so — so the three soft-mask patches stay
UNRESOLVED at 0.96–0.99 correlation.

There is now a bimodal split to calibrate against: **0.962 / 0.978 / 0.986**
for the three soft-mask patches against **0.039 / 0.053 / 0.062 / 0.406** for
the 16-bit and shading ones.

**It was left alone on purpose.** Calibrating the instrument immediately
after making it report what you wanted is not a measurement. Do it in its
own session, with its own justification, and preferably against a patch
whose expected verdict is known independently — which is exactly how
`ghent-check`'s *trap* threshold was calibrated in the first place
(GWG 16.0, 2026-08-17).

---

## §6 — `3_GWG161`: 14 traps, UNEXPLAINED — with two explanations RULED OUT

This is the largest single unknown left on the board, and it is recorded as
unknown rather than assumed.

**Ruled out (1) — GWG's own CMM tolerance.** The patch's ReadMe says in
terms: *"A faint X is due to differences in the CMM and does not indicate a
failure."* pdfce **has** a different CMM — its `DeviceCMYK` → sRGB is the
naive additive conversion, and the parity harness measures `DeviceCMYK`-only
pages diverging at **5.4×** the clean-page mean against pdfium's
`AdobeCMYK_to_sRGB1`. So this was the obvious candidate. **Measured:** the
X-versus-surround worst-channel gap across the fourteen trapping cells is
**39, 47, 68, 69, 77, 88, 90, 90, 101, 121, 126, 171, 176, 178**. One cell is
arguably faint. Thirteen are not.

**Ruled out (2) — "the blend never ran."** pdfium's value sits **between**
pdfce's and the surround on essentially every cell. A blend that never ran
returns `C_s` unchanged and would put pdfce *at* the source, not past pdfium
in the same direction. pdfce is applying the blend **harder** than it
should — a wrong operand, not a missing operation.

**Where to look first.** The ReadMe describes a **three-layer**
construction: *"Upper object — ICCbased with transparency effect (Fill),
Lower object — ICCbased, Background object — DeviceCMYK."* **Which form
XObject pdfce takes as which has never been established.** A wrong operand
is what the evidence points at, and the operand pairing is the thing nobody
has checked.

---

## §7 — HOUSEKEEPING, and two of these are owed

- **`tools/render-parity/out/summary.json`, the standing gate's RECORDED
  BASELINE, is STALE and not comparable to a current run.** It records a
  bucket vocabulary the harness no longer emits (`benign`/`known-gap`
  against `below-band`/`disclosed-gap-small`). **`--gate` mode is
  meaningless until it is re-based.** Comparing two current runs is the only
  thing that means anything today, and that is what was done here.
- **`check-ledger-numbers.py` was rewritten**, third instance of one class
  (anchor one decoration-spelling behind — first stars, then more stars, now
  backticks). The anchor is **gone** rather than widened a third time; a
  heading declares a Pass iff it is an `###` whose pre-em-dash prefix
  **leads with** a Pass ID, with parenthesised mentions stripped and
  struck-through headings excluded. All four refinements were **measured**
  against `ROADMAP.md`, not assumed. A fifth defect fell out: the
  staged-ship qualifier regex capped the parenthetical at 40 characters, so
  `Pass 52.2`'s two legitimate halves matched neither and a correct filing
  was reported as a collision.
- **`overprint::composite` still carries the "a transparent pixel is white
  paper" convention** that `Pass 97.0a` removed from the two blend
  composites. Deliberate: it is Table 149 *decision* logic rather than a
  blend function, and changing it belongs with the colorant buffer that will
  replace its input.
- **`/TR` on a soft mask is still read, counted and never evaluated.** It
  needs the §7.10 function machinery inside `pdfce-render`. `/TR` is where a
  mask gets inverted, so an ignored one can leave visible exactly what the
  document meant to hide.
- **Backups are 4 days and ~50 commits stale** (newest bundle 2026-08-17).
  Librarian-measured, not inferred.
- **Nothing pushed.** `origin/main` is behind; `v0.7.0` is bumped but **not
  tagged** (`git tag -l` tops at `v0.6.0`). Standing operator go-ahead for
  builds/releases since 2026-08-17.

---

## §8 — BROTLI, and the operator's condition has NOT fired

Ken asked, 2026-08-21: *"please note somewhere that we are going to have to
add brotli compression when it becomes part of the pdf 2.0 standard."*

`pdfce-spec-librarian` established the answer and wrote
`D:\Dev\Rag-Specialized\PDF_Spec\filters\filter__brotli.md`:

- **Measured negative from the licensed primary:** `brotli` returns **zero
  hits** in ISO 32000-2:2020 (1,023 pp) *and* ISO 32000-1:2008. §7.4 ends at
  7.4.10; **Table 6 enumerates exactly ten filters in both editions**; and a
  scan of all 2,840 Errata Collection 3 annotations found nothing, so the
  negative holds for the corrected text too.
- **But the specification EXISTS and is finished.** `EXTN-BROTLI-1 v1.3`,
  *Brotli compression in PDF 2.0*, PDF Association PDF TWG, announced
  **2026-08-19** — two days before he asked. CC-BY-4.0. Filter name
  **`/BrotliDecode`**. It is an **extension**, registered under the `PDFa`
  developer prefix, **not** on a dated ISO path.

★ **So his condition describes an event that has not been scheduled, while a
specification pdfce could implement against already exists.** That is
**open operator question `(bq)`** and it is his call, not the engineer's.
Default if unanswered: wait.

Three things a future session should not have to rediscover: **a false
citation is circulating** — every web-search summary says the filter is in
"ISO 32000-2:2020 §7.4.11", and **§7.4.11 does not exist** (traced to an
unmerged pypdf PR); **`FlateDecode`'s predictors apply verbatim**, so
pdfce's predictor code is reusable as-is; and **`brotli` 8.0.4 is
BSD-3-Clause AND MIT**, so a read-side addition needs no operator licence
call. Today pdfce returns `FilterError::UnsupportedFilter("BrotliDecode")`,
which is correct behaviour and not a bug.

---

## §9 — TRY IT

```
pdfce-cli render-page <a Ghent transparency patch> --page 1 --scale 2 -o out.png
python tools/ghent-check.py D:\Dev\temp\ghent-patches
python tools/ghent-cell-probe.py <patch stem> --reference-dir <pdfium renders>
```

`render-page`'s stable stdout line gained three keys this session, all
appended last: `groups_backdrop_reruns`, `soft_masks_on_group_result`,
`overprint_images_unsupported`.

**And run it on his real drawing before calling anything shipped** — that
rule earned itself three defects in one session last week. It has no
transparency groups, so this session's work could not touch it, and that
was verified rather than assumed: 0.96 s, both new group counters at zero.

---

## §10 — THE BENCHMARK FILE, written once

    D:\Dev\temp\pdfce\ncored-benchmark-cad-drawing.pdf

★ **Written here once, on its own line, and referenced from everywhere else
rather than repeated** — a Windows path in prose is the single most reliably
mangled string this project handles. The engineer's agent memory carries the
rule: **any shell metacharacter breaks heredoc patching — write the file, or
use `Edit`.**
