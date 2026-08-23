# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. Overwrite it once acted on.

**Written 2026-08-22 (evening), by `pdfce-librarian` at the engineer's
request** during the 228th filing. **Amended 2026-08-22 (229th filing)**, then
**REWRITTEN 2026-08-22 (230th filing)**: §0.5's metrics-contract repair is
**fully discharged and gated** by `ea413a4`, the banana page has gained a
molecule box at 1:1 (`PASS 74.6`/`74.6b`), and the deep-zoom ceiling turned out
to have been filed against the wrong quantity. **`(br)` in §0 is still the only
thing awaiting Ken.**

---

## §0 — THE BANANA IS DONE, AND IT NOW GOES DOWN TO MOLECULES. GO LOOK, THEN ANSWER ONE QUESTION

The operator's stated first task — *"the first thing I am going to do is
refine the banana.pdf"* — **shipped** as `Pass 74.4` + `Pass 74.5` (`e36f96e`)
and again as **`PASS 74.6` + `PASS 74.6b`** (`950e3af`, `65e1910`).

### What to look at, first thing

**`C:\Users\Ken\OneDrive\pdfTests\`** — `banana-at-scale.pdf` and **ELEVEN
numbered renders**, `_1-whole-page` through `_11-ten-molecules`. **Tiers 10 and
11 are new** (`_10-molecule-box-pointer`, `_11-ten-molecules`) and they are the
two to open first.

**Below the cells, at the end of a tapering dart, a 50 × 35 nm box holding the
ten most abundant molecules in a banana cell — at the SAME 1:1 scale as
everything else on the page.** The banana is **412 585 829 water molecules
long**, and both are drawn in one coordinate system with **no scale break
anywhere between them**.

- **Authored in picometres**, because that is the unit bond lengths and atomic
  radii are published in — O–H `96`, C–C `154`, oxygen's van der Waals radius
  `152`, potassium's ionic radius `138`. Every number is checkable against a
  reference table **without arithmetic**.
- **Eleven Form XObjects** (ten molecules plus the box that invokes them), so
  the page now contains **nested forms** for the first time.
- **Space-filling rather than ball-and-stick**, deliberately: a stick model is
  mostly empty space and this box exists to say **how big these things are**.
  The cost is stated rather than hidden — rings come out as lumpy blobs,
  connectivity is sacrificed, and bonds are drawn underneath so they still show
  in the gaps.
- **Three label lines each**: name, **share of the fruit by mass**, size.
  **Adding the shares RE-ORDERED the list** — water 74.9, starch 5.4, glucose
  5.0, fructose 4.9, sucrose 2.4, cellulose 1.2, protein 1.1, pectin 0.7, malic
  acid 0.4, potassium 0.36, summing to **96.4 %** of the fruit. **Five of ten
  moved** from the prior remembered ranking.
- **Ripe figures, and the subtitle says so** — green pulp is ~20–25 % starch and
  ~1–2 % sugars. The page draws a **yellow** banana beside cells with **green**
  chloroplasts and packed starch grains; **that seam is disclosed, not tidied**.
- **Water's label reads 0.37 nm, not the 0.28 nm everybody quotes.** `0.28` is
  the kinetic diameter (the hole it fits through); `0.37` is the space it
  occupies. The box draws at 1:1 and invites you to measure it, so the label
  the drawing disproves is the wrong one to print.

### ★★★ THE RENDER IS OFF-CENTRE ON PURPOSE. That framing is EVIDENCE.

**Do not "fix" it.** `tools/gen-scale-demo/README.md` carries the command to
reproduce it. Nudging `--region` does **nothing**: two successive nudges give
**byte-identical framing**, because `--region` is `f64` all the way through
`region_base_geometry_of` while the content's device translation is `f32` and
**quantised in ~500 px steps at `scale = 8.1×10⁶`**. See §0.5.

### What changed on the page before this (`Pass 74.5`, still current)

Every mitochondrion is drawn to its **real internal anatomy** rather than an
ellipse with chords — a crista is an **invagination of the inner membrane**
joined through a narrow neck, and the space inside it is continuous with the
intermembrane space, not the matrix. **342 instances from 12 shared forms,
7 571 F1 heads**, all four populations sharing one module. Before that they
were three copy-pastes that had drifted to three cristae / one or two / none.

### ★★ THE ONE THING WAITING ON KEN — open question `(br)`

**May pdfce skip geometry smaller than a pixel?**

- Deep zoom is now **fast**: the `/BBox` viewport cull skips forms that
  cannot mark a pixel, and the deepest tier went **802 ms → 120 ms**. That
  cull is **exact** — ISO 32000-1 §8.10.1 makes `/BBox` a clip, and the
  test asserts the raster is byte-identical to the same page with the `Do`
  removed.
- **Page-fit zoom is the slow case now**: all 342 forms *are* on the canvas
  at ~1/70th of a pixel each, every path inside them is still rasterised,
  **~1.5 s** (was ~110 ms before the page gained ~1 000 000 path
  operators).
- Skipping sub-pixel geometry would fix that and **would be lossy** —
  those paths contribute anti-aliased coverage. **Decision 082** says a
  render may skip work only where skipping is exact, so this is **his
  call, not the engineer's.** Default if unanswered: **never skip**.

**Do not take this as a performance ticket.** It is `ROADMAP.md`'s open
operator question `(br)`, with three costed answers written out there.

### What is still on the imperfections list

Untouched by this Pass, and still worth doing if he wants more refinement:
pulp-cell **label collisions** (`nucleus` / `nucleolus + chromatin`
leaders converge; `central vacuole` sits near a starch grain); the
banana's **stem and blossom scar** read as placeholders; the **dart's
tail** is a blunt cut; `easter_egg.FONT` is **all-caps only** (lower case
needs curves, and curves from 1.5 µm beads stop reading as letters — the
honest route is *larger letters, not smaller beads*); the **heart glyph
sits low** on its line; **standard-14 Helvetica, not embedded** (what stops
this being PDF/A); **no `/Info` dictionary**; the page's **lower third is
empty**.

Two constraints that are load-bearing rather than decorative: **labels are
1/10 the largest cell's height** (30 µm = 0.085 pt, so cell size drives
label size), and **the arrow is a tapered dart rather than a shaft-and-head
because a conventional arrowhead is ~8 pt across — nine times wider than
the thing it points at.** The easter egg's binding constraint is
**legibility, not space**: ~8 beads per stroke is enough to read and not
enough to look smooth, which is why the anniversary line is ordinary type.

### How to look at any tier

`--region` takes **two corners in PDF user space**, not an origin and a
size. The viewport is what costs time, so keep the region small and put the
magnification in `--scale`. `tools/gen-scale-demo/README.md` carries the
current tier table with timings and the regions for all eight tiers; it was
rewritten in `e36f96e` and is accurate.

```
python tools/gen-scale-demo/gen_banana.py <out.pdf>
pdfce-cli render-page banana.pdf --page 1 --scale 1.6 -o page.png
```

---

## §0.5 — THE METRICS CONTRACT IS **DONE AND GATED**. WHAT REPLACED IT IS A CEILING IN THE WRONG PLACE.

### The metrics contract — CLOSED, do not redo any of it

**`ea413a4`** discharged all three steps the 229th filing owed, **in the
required order**: template repaired (**59 → 87 of 87**), per-key table repaired
(**44 → 87 of 87**, **57 rows added**), **then**
`tools/check-metrics-line-contract.py` wired into `.github/workflows/ci.yml`
and registered in `tools/check-ci-parity.py` — **which caught the omission on
its first run, exit 1, before anyone had thought to add it.** The placeholder
scheme went from `<K>`…`<ai>` to a uniform `<n>`, with `need_appearances=<0|1>`
kept distinct because it is the one field that is not a count, and the
`annots=…` elision that hid six real keys is expanded.
**`python tools/check-metrics-line-contract.py` → exit 0, 87 keys.**

★ **One thing to carry from building it**, recorded against `R212`: the gate's
**first** version refused to check per-key table coverage, arguing that a
missing row is an *incomplete explanation* while a missing template key is a
*wrong published specification*. **True — and the half it discarded was the
LARGER gap and a disjoint set.** A scope argument that is **correct** passes
review and stops it. The fix is one word: **`and`**.

### ★★★★★ THE CEILING WAS FILED AGAINST THE WRONG QUANTITY — read this before quoting any deep-zoom figure

**`Pass 74.1`/`74.2` pushed deep zoom past a trillion percent. That claim is
TRUE and it is about the VIEWPORT.** It was never a claim about page-space
geometry, and **nothing in four documents distinguished the two** until the
molecule box needed both. **Nothing was wrong.** That is what made it survive.

**Three `f32` limits sit under the CONTENT:**

1. **Path coordinates.** An `f32` near `x = 540 pt` has a spacing of
   `6.1×10⁻⁵ pt` = **21.5 µm**. Anything smaller written as an **absolute page
   coordinate** is quantised away. **This is why everything small on this page
   lives in a Form XObject with small local coordinates — not a workaround, the
   only representation that works.**
2. **The placement matrix.** Concatenating a `cm` carrying a page coordinate
   leaves the CTM's translation as the difference of two large nearly-equal
   `f32`s. Drift ≈ `page_x × scale / 16 700 000` px — **~5 px** at the
   mitochondrion tier, **~400 px** at the box tier, **past the viewport above
   `scale = 5×10⁶`**.
3. **Consequently device position is QUANTISED** — ~500 px steps at
   `scale = 8.1×10⁶`.

**Measured**, 1600 px framed on the water molecule, on the box's eleven forms:
**11/11 forms at `scale` 2e6 and 5e6, 7 at 1.25e7, 3 at 2.5e7, 1 at 5e7.**
Confirmed **not** the Form XObject path and **not** `Pass 74.4`'s cull: a
synthetic page drawing the same square three ways loses all three at the same
magnification, **including the one using neither `cm` nor a form**.

**Standing rule `R213` was minted from this** — *a magnitude claim is a claim
about ONE quantity; name the quantity in the LABEL*. Two `docs/` survivors were
repaired in the same filing: `FEATURES.md` row 201, and `ARCHITECTURE.md`'s
sub-heading **"Numerical reach"** (the paragraph under it was always correct;
the heading named nothing, and **the heading is what gets quoted**).

### What is owed: `PASS 74.7`, and it is the ENGINEER'S CALL, not Ken's

**Carry the CTM in `f64` through content-stream `cm` concatenation and narrow
only at paint** — the same trick `Pass 74.2` used for the base CTM, one level
down. Full entry under `ROADMAP.md` *Backlog*, **with the measured table above
as its acceptance baseline**.

- **Decision 082 does NOT gate it.** 082 governs *lossy* speed-ups. `74.7`
  **removes** a fidelity loss rather than trading one, so no choice is being
  made and no operator ruling is needed. **It is not `(bs)`.**
- **But it is BIG**, which is why Ken is being told it is queued: every `cm`
  concatenation in the interpreter, the narrowing point moved, and **`R211`
  binds where that narrowing happens** because of the round-trip invariant — a
  blanket `f32 → f64` can change a rasterised byte on a page nobody edited.
  Owed-item 8 (the stale render-parity baseline) is a **soft prerequisite**.
- **Limit 1 is a SEPARATE defect and `74.7` does not fix it.** A coordinate
  already written into the content stream at `f32` resolution is lost before
  any matrix applies. Say so in the Pass's own disclosure.
- **`PASS 74.8` is opened beside it and is small**: `annotations_out_of_scope`
  and `page_content_suppressed` are computed, merged and documented in
  `pdfce-render` and **printed NOWHERE**, so `render-page --no-annotations`
  withholds annotations and prints **no number saying how many**. **That is a
  rule-4 disclosure gap, not a stale comment.** Two keys, two table rows, two
  template entries, one test-list extension — and the new gate will fail you
  for doing two of the three.

### Owed to `crates/`, in its OWN commit — eight stale `Diagnostics` doc comments

Found by the engineer while drafting the 57 table rows. Full list in
`ROADMAP.md` / `SESSION_LOG.md`; the three that matter:

1. `ColorDiagnostics::patterns_unpainted` says pdfce *"does not paint"* tiling
   and shading patterns — **a comment ~300 lines below it in the same file
   exists specifically to refute that sentence**, and the CLI's stderr note
   describes the counter as the remainder after shading patterns paint.
2. The *"shadings are found but never painted"* narrative survives in **three**
   places after the analytic painting slice shipped
   (`ShadingDiagnostics::painted`, `::paintable`, `Diagnostics::shading`).
3. `transparency_groups_special`'s *"Of those"* now sits **after**
   `soft_masks_on_group_result` in declaration order, so **its antecedent reads
   wrong**. ★ **No word in it is stale — it broke because something else moved,
   and no grep or gate can find that.**

Also: `overprint_requested`'s head sentence still says *"while pdfce does not
simulate it"* while its own body corrects that; `transparency_groups_composited`
still says *"over-reports until `Pass 97.0` lands"* and 97.0 shipped
2026-08-21; and `render-page --region`'s doc block now owes a **third** number
(it still carries `74.1`'s 20 000× ceiling, never gained `74.2`'s, and now owes
the content-side one too).

**Take these in their OWN commit, not bundled into anything else.** A filing
commit that touches `crates/` or `tools/` becomes a code commit in no filing
and turns `check-commits-filed.py` red on the commit meant to make it green —
`d4721d8`'s memory file.

---

## §1 — THE THREE PRE-FLIGHT CHECKS, unchanged and still earning their place

**1. `ls` BOTH FeatureRequests channels.** They are outside this
repository, so **no gate will ever contradict a stale sentence about them —
including this one.**

```
D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\
D:\Dev\FeatureRequests\iccce_FeatureRequests\open\
```

Three sessions running, that `ls` found something a document said was not
there. On 2026-08-21 it found two `iccce` notes that had landed overnight:
one corrected a design sentence before it was implemented, and another —
*"a `(C, α)` buffer is a lossy representation of the model"* — was worth
**thirteen trap marks**. A reply is filed in that channel.

**2. Run the gates — `ls tools/check-*`, do not trust any list.**
`python tools/check-ci-parity.py --list` prints the local stand-ins.
**`R209`:** *"all gates green" names a set, and the set somebody runs is
not the set CI runs; a CI job with no local runner is UNOBSERVED, not
passing.*
*(★★ **THE 230th FILING RAN THE WHOLE SET AND COUNTED IT: 17 `tools/check-*`
scripts on disk (12 `.py` + 5 `.sh`), 16 runnable as bare gates, ALL 16 EXIT
0.** The 17th, `check-image-colorspace-truth.py`, exits 1 on a bare invocation
**because it takes a fixture-directory argument** and is not a gate. ★ **The
dispatch that commissioned that filing said "all fifteen local gates" —
fifteen was the 215th filing's figure, and TWO gates have landed since**:
`check-ci-parity.py` (`5884ed1`) and `check-metrics-line-contract.py`
(`ea413a4`, in the very commit being filed). The 228th filing ran four, the
229th ran five, the 230th ran sixteen. **That progression IS `R209`'s point:
"all gates green" names a set, and the set somebody runs is not the set CI
runs. Count them; do not quote a count.**)*

**3. Read `docs/compositor-plan.md`** before scoping anything in `97.x`.

---

## §2 — ★★ THE GHENT PASS COUNT IS AN OVER-COUNT. Read this before quoting any figure.

`tools/ghent-check.py` implements **one of the suite's two pass criteria**.
It hunts for a **cross that should not be there**; seven patches instead
mark failure by the **absence of a check mark** (GWG 050, 080, 081, 082,
150, 151, 152) and have scored `clean` since the harness was written. At
least three of them are failures. A second fault — the contrast floor has
no **area** term — puts `GWG 1.0` in the same category.

**Corrected standing: 26 at most, not 29.**

⇒ **THE DELTAS SURVIVE, THE LEVELS DO NOT.** Every board ever filed is
over-counted by the same family, so a before/after comparison is sound and
any absolute "N of 51" is not.

The operator's cell-by-cell judgements are in
`docs/ghent-operator-review-2026-08-21.md` — **the only independent check
this harness has ever had**, and the calibration set for fixing it
(`Pass 122.2`).

---

## §3 — WHAT SHIPPED 2026-08-21/22

**`Pass 74.4` — Form XObject viewport culling** (`e36f96e`). `do_form`
skips a `Do` whose `/BBox`, mapped through the CTM, misses the canvas or
the clip, **without decoding its stream**. Exact by §8.10.1. New counter
`forms_culled`, printed beside `forms`.

★ **The finding is not the cull, it is where the cull went.** The first
version sat beside the `/BBox` clip — the natural-looking home — reported
the **same `339 of 342`** and bought almost nothing, because the stream had
already been sliced, flate-decoded and parsed: **~110 kB × 342 ≈ 37 MB of
inflate per render for content about to be discarded.** Hoisted above the
decode: **802 ms → 120 ms**. **A cull is worth only what it skips, and a
counter that reads identical either way is what makes the wrong version
convincing.** Accept an early-out on a wall clock, never on a hit rate.

**`Pass 74.5` — `tools/gen-scale-demo/mitochondrion.py`** (`e36f96e`) —
§0. Two silent bugs found by **rendering, not reading**: `PT_PER_NM`
written as a division where it needed the reciprocal (**8.03× too small**,
everything correctly placed and shaped, which reads as a drawing choice),
and a crista path that dived in on the wrong lateral side and **crossed its
own junction mouth twice** — stroked perfectly, while nonzero winding
filled every lumen with matrix colour. Catching the second took a
**160 000 000 % render of a single crista**.

**Deep zoom** (`bd9844d`, `71f7055` — `Pass 74.1`/`74.2`): `--region` on
the CLI, and a region's device geometry computed in `f64`. A requested
800×600 viewport used to come back **800×512** at 215 million percent; **the
VIEWPORT now holds to a trillion percent.** ★★ **That sentence's last three
words are load-bearing and used not to be there** — the claim is about the
returned pixmap, **not** about page-space geometry, which has three much lower
`f32` ceilings of its own (§0.5, `R213`, `PASS 74.7`).

**The colorant buffer** (`97.1e`, `97.1f`): a page whose group declares a
subtractive blending space composites in four ink planes end to end. Ghent
wrong-space blends **107/107 → 0/107**. Corpus A/B: **3 731 identical of
4 023, 4 changed**, all four prepress conformance files.

**Two performance repairs** (`97.1i`, `97.1j`) — the first fixes a
regression `97.1e` shipped the same morning:

| page | pre-Pass | after `97.1e` | now |
|---|---|---|---|
| 1 | 632 ms | 3 713 ms | **320 ms** |
| 2 | 4 086 ms | 7 689 ms | **2 486 ms** |

**An `/Indexed`-over-`/DeviceN` palette bug** (`97.1h`), found by the
operator reading a test patch's caption: a two-colorant tint transform was
handed four inputs, refused, and fell back to a neutral — a grey palette,
with no counter anywhere. pdfium now agrees with us pixel for pixel.

**The molecule box** (`950e3af`, `65e1910` — `PASS 74.6`/`74.6b`): §0.

**The metrics contract** (`ea413a4`, **no Pass ID**): §0.5.

**Pass IDs, and this is the sentence that matters when you commit:**
`74.1`–`74.6b` were all **minted by filings**, not by commit messages.
**`74.7` and `74.8` are CLAIMED under *Backlog* and not built, so the next
free in this family is `74.9`.** Next free elsewhere: `122.4`; `97.1g` is
reserved and unbuilt. Decisions next free **083**; standing rules next free
**`R214`**; open operator questions next free **`(bs)`**.

---

## §4 — THE QUEUE, in the order I would take it

0. **Answer-dependent:** if Ken answers `(br)` with "skip them" or "skip
   them behind a setting", that becomes the top item and it is a
   `pdfce-render` Pass with a **disclosure obligation** (a counter on the
   metrics line, off by default if it is a setting).
1. **The eight `crates/` `Diagnostics` doc repairs in §0.5** — one commit,
   and they are the freshest stale claims in the tree. **Item 3 of that list is
   NOT a comment fix**: it is `PASS 74.8`, a rule-4 disclosure gap
   (`--no-annotations` says nothing about how many it withheld), and it wants
   its own commit and its own metrics-line keys.
1b. **`PASS 74.7`** — the `f64` CTM through content-stream `cm`
   concatenation. **Ruled the engineer's call, not Ken's** (§0.5), acceptance
   baseline already measured, and **large**. Rank it against the `97.x` items
   below on how much deep-zoom work is actually wanted; nothing blocks it.
2. **`Pass 97.1g`** — non-isolated ordinary groups on a subtractive page
   are composited as if isolated. The arithmetic exists
   (`remove_backdrop_cmyk`); the second content walk does not. **A port of
   the additive path, not a design.**
3. **`Pass 97.1k`** — native colorant paths for images and shadings, which
   bridge through sRGB today (`cmyk_bridged_pixels`).
4. **`Pass 122.2`** — teach `ghent-check.py` the check-mark criterion and
   give its contrast floor an area term. §2 says why this is not optional.
5. **`Pass 122.1`** — per-sample image overprint. ★ Now **diagnosed**: it
   is why `GWG 8.2`'s check mark is missing. The mark is painted in yellow
   *underneath* the images, which overprint it so cyan-over-yellow reads
   green; pdfce paints the image normally and covers it
   (`overprint_images_unsupported = 2`). pdfium fails it too.
6. **`Pass 122.0`** — multithreading, the operator's request. His design
   (a runtime max-cores setting) is right and is kept; decision 080 adds a
   **compile-time target gate**, because `std::thread` and `rayon` both
   `cargo check` cleanly for `wasm32` and the CI wasm job therefore cannot
   catch a threading regression. Hard acceptance criterion: **byte-identical
   output at any core count.** ★ **Note the interaction with `Pass 74.4`:**
   the banana page is now a genuinely useful threading benchmark, because
   its cost is ~1 000 000 path operators in 342 forms rather than one big
   image.
7. **`Pass 119.1`** — `unshare_form`. Carried unstarted through six
   handoffs now.
8. **`Pass 122.3`** — the colorant buffer's byte ceiling. Interactive use
   is unaffected, but a **full-page** render above ~375 DPI refuses the
   buffer and silently composites in the wrong space, so one page can have
   different colours at different resolutions.

---

## §5 — STILL NOT DONE, named so it does not read as done

- **Sub-pixel geometry is still rasterised** at page-fit zoom — ~1.5 s on
  the banana page. **Blocked on `(br)`, deliberately.**
- **Page-space geometry is still `f32`** — 21.5 µm coordinate quantisation
  near `x = 540`, `cm` drift ≈ `page_x × scale / 16 700 000` px, device
  position quantised in ~500 px steps at `scale = 8.1×10⁶`. **`PASS 74.7`,
  unbuilt.** Not blocked on anything — just not taken.
- `resolve_indexed` builds its palette with a **scratch
  `ColorDiagnostics` that is discarded**, so a tint failure inside a
  palette never reaches the operator.
- **Implicit knockout**: only explicit `/K true` is honoured. `/TK`
  defaults true (every text object), and `B`/`b` and shading patterns are
  knockout. **The one pdfce implements is the rarest.**
- **`/TR` on a soft mask** is read, counted, never evaluated.
- **`/AIS true`** is not distinguished from `/AIS false`.
- **Spot colorants** — four planes, not runtime `N`. Every remaining
  trap-criterion Ghent FAIL is in this bucket.
- **Per-paint rendering intent** (§11.7.5.3) — pdfce carries one per page.
  `iccce` costed the alternative and asked for the consumer fact; **no
  corpus measurement of mid-page intent switching has been taken.**
- **No GUI code path reads `forms_culled`**, `render_page_region` or the
  display list. GUI work is paused; recorded so the `[ ] gui` boxes in
  `FEATURES.md` are not mistaken for oversights.

---

## §6 — SIX LESSONS FROM THIS RUN THAT ARE CLASSES, NOT INCIDENTS

1. **A cull is worth only what it skips.** Its hit counter is **invariant
   to its placement**, so the instrument you add to prove the optimisation
   works cannot detect that it barely works.
   `D:/dev/rag/rust/a_cull_placed_after_the_decode_reports_the_same_count_and_buys_almost_nothing.md`
2. **A unit constant inverted by a non-round factor renders as a drawing
   choice.** PDF user space has no declared unit, so nothing in the file
   and no validator can contradict a wrong scale. **Assert one known length
   at author time** — one line turns an invisible render defect into a
   build failure.
3. **A self-crossing subpath strokes perfectly and fills wrong.** Stroke
   and fill read one path through two unrelated rules, so **reviewing the
   outlined drawing is not evidence about the filled drawing** — and the
   outline is the one that looks diagnostic.
4. **Three copies of one drawing drift silently**, because a duplicated
   *drawing* has no compiler, no test and no diff anybody reads, and no two
   copies are ever on screen at the same magnification at the same time.
5. **★★ A ceiling is a claim about ONE quantity, and the quantity is the half
   that gets lost.** Unlike every other stale-claim failure this project has
   numbered, **this one leaves no disagreement behind** — the measurement was
   right, the sentence was true, and review answers *"is this true?"* with
   *yes*. It surfaces only when one artefact needs both quantities at once.
   n=2: `mask.fill_path`'s 217 µs-vs-8.3 µs, and *"holds to a trillion
   percent"*. **A claim loses its qualifier as it PROPAGATES, not as it is
   made** — so sweep what repeated it, and read the LABELS.
   `D:/dev/rag/rust/a_ceiling_is_a_claim_about_one_quantity.md`, `R213`.
6. **A TRUE scope distinction is the hardest argument against checking both.**
   A false justification fails review; a correct one **passes** it, and nothing
   prompts the next question. The fix is one word: **`and`**.
   `D:/dev/rag/rust/a_true_scope_distinction_is_the_hardest_argument_against_checking_both.md`.

---

## §7 — HOUSEKEEPING

- **`origin/main` is at `c24ad7a`, NINE COMMITS BEHIND `HEAD`.** Pushing is
  the operator's act and needs a current go-ahead (`CLAUDE.md` rule 8). The
  repository is public, so **anything committed is published by default.**
  *(Measured 2026-08-22, 230th filing: `git remote -v`,
  `git rev-parse --short origin/main`, `git rev-list --count origin/main..HEAD`
  → **9**. The 228th filing's "three" was correct for its own time.)*
- **Backups are 202 commits behind** — newest bundle
  `pdfce-20260817-v060.bundle` (2026-08-17 20:34) at `3c4c00e`. `v0.7.0`'s
  tag is in no bundle on disk. **Cutting one is the operator's call.**
  *(Measured 2026-08-22, 230th filing: `ls -lt D:\Dev\pdfce-backups\` and
  `git rev-list --count 3c4c00e..HEAD` → **202**. Re-measured rather than
  carried forward: the 228th filing said 196, and that was right then.)*
- **If `check-commits-filed.py` is red when you start, READ ITS OUTPUT FOR
  THE HASH.** Do not assume which commit it means, and **never extend
  `tools/commits-filed-baseline.txt`** — that file is pre-existing debt,
  not an allowlist. The 2026-08-22 afternoon handoff carried a conditional
  instruction that would have bought a duplicate filing, because its
  condition came true for a cause its author had not enumerated.
- ★ **A claim in `bd9844d`'s commit message is WRONG and stands corrected
  only in the filing.** It says `tools/check-string-gaps.sh` *"has caught
  it every time"*. It has not: `ae06440` (2026-08-20) is titled *"the
  string-gap gate reported two of three"*, and that miss was found by a
  human who knew there were three. The ordinal ("third time") survives; the
  "every time" does not.
- **Worktrees, measured 2026-08-22 (230th filing) by `git worktree list`:
  NINE entries.** `pdfce-base` is **gone**; **`pdfce-head` is still there**
  (`%TEMP%\pdfce-head`, detached at `0eff831`), plus **seven agent worktrees**
  under `.claude/worktrees/`. `git worktree list` is authoritative — do not
  quote this line, re-run it.
