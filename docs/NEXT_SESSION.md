# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. Overwrite it once acted on.

**Written 2026-08-21 (evening)**, replacing the 2026-08-21 (afternoon)
handoff. §7 says which of its clauses this one supersedes.

---

## §0 — DO THESE THREE THINGS BEFORE ANYTHING ELSE

**1. `ls` BOTH FeatureRequests channels.** They are outside this
repository, so **no gate will ever contradict a stale sentence about them —
including this one.**

```
D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\
D:\Dev\FeatureRequests\iccce_FeatureRequests\open\
```

**★ THIS IS NOW THREE SESSIONS RUNNING WHERE THE `ls` FOUND SOMETHING A
DOCUMENT SAID WAS NOT THERE**, and this time it changed the code. Two
`iccce` notes landed at 02:37 and 03:15 — *after* the afternoon handoff was
written, *before* this session started reading. One of them
(`note_compositor_order_is_required_and_three_things_in_it_are_wrong.md`)
corrected a design sentence I would otherwise have implemented as written,
and a second correction inside it (*"a `(C, α)` buffer is a lossy
representation of the model"*) was worth **thirteen trap marks** on the
Ghent knockout patch. See §3.

A reply is filed:
`iccce_FeatureRequests/open/2026-08-21-reply-the-buffer-is-f32-and-three-of-your-corrections-were-load-bearing.md`.

**2. Run the gates — `ls tools/check-*`, do not trust any list.** 16 on
disk, 15 wired into CI. `python tools/check-ci-parity.py --list` prints the
eleven local stand-ins. **`R209`:** *"all gates green" names a set, and the
set somebody runs is not the set CI runs; a CI job with no local runner is
UNOBSERVED, not passing.*

**3. Read `docs/compositor-plan.md`** before scoping anything in the `97.x`
family. Its Stage B section is now **shipped**, and its §3.2 amendment
about shape-vs-alpha is the one that was proved in traps.

---

## §1 — WHAT SHIPPED, AND THE MEASUREMENT THAT SCOPES WHAT IS LEFT

`Pass 97.1e` (`a277931`) and `Pass 97.1f` (`ff4b4bf`): **a page whose group
declares a subtractive blending colour space is now composited in four
colorant planes**, from the first operator to the last, and converted to
screen colour once at the end.

**MEASURED, against a binary built from `06aaad3` in a git worktree — not
against a number carried in a document.** The worktree has been removed; to
reproduce, `git worktree add <tmp> 06aaad3` and
`cargo build --release -p pdfce-cli` in it (5m37s here). Doing that cost
five minutes and settled two contradictions between documents, which is why
it is worth the paragraph: the afternoon handoff's Ghent figures and a
memory entry's disagreed, and only a rebuild could say which was right.

| Ghent PDF Output Suite | baseline | 97.1e | **97.1f** |
|---|---:|---:|---:|
| pass | 26 | 28 | **29** |
| FAIL | 14 | 11 | **10** |
| UNRESOLVED | 11 | 12 | **12** |
| trap marks | 55 | 45 | **41** |

| blend-space census (Ghent) | baseline | **97.1f** |
|---|---:|---:|
| blend modes applied | 107 | 107 |
| of those, in the wrong space | 107 | **0** |

`1_GWG162` (Isolate), `3_GWG164` (ICCBasedCMYK) and `1_GWG161` (Knockout)
flip to pass. `1_GWG1611` loses its last trap and moves to UNRESOLVED.
`3_GWG161` goes 14 → 11 traps.

★ **Every remaining FAIL on the suite is an overprint, spot or ICC patch.
Not one of them is a blending-space failure any more.** That is the fact
that re-scopes the family: the next Ghent gains are `85.5`'s n-channel spot
work, not more compositing.

**Real-world corpus, A/B'd build against build** — every one of
`fixtures/external`'s 4,023 files rendered by both binaries and the PNGs
compared byte for byte:

> **3,731 identical · 4 CHANGED · 288 unrenderable by one or both.**

★ **AND THE FOUR ARE WORTH READING, BECAUSE TWO OF THEM REFUTE THE
PREDICTION IN THE AFTERNOON HANDOFF.** That handoff said *"both real-world
hits are veraPDF transparency CONFORMANCE fixtures"* and named two. Both
appear (`6-2-9-t05-fail-g`, `6-2-10-t05-fail-g` — PDF/A-4 §6.2.9 and
PDF/A-2b §6.2.10). **So do two more**: `6-2-4-3-t04-pass-e` and
`6-2-4-3-t04-fail-v`, from *6.2.4.3 Uncalibrated — Device colour spaces*.

The mechanism is instructive rather than alarming. The prediction was
derived from `blends_in_wrong_space`, which counts **blends**. Those two
files declare a subtractive page group and perform **no non-`Normal`
blend at all** — so they were never in that number — and their pixels move
anyway, because the *conversion* moved. Before, each paint was converted
CMYK→sRGB and composited on screen; now the page composites in ink and
converts once at the end, which is what §11.4.7 requires and what makes the
two orders differ on any partially-covered pixel.

Measured on `6-2-4-3-t04-pass-e`: **4,600 of 484,704 pixels (0.95 %), max
channel delta 11/255, confined to a single 110×80 bounding box.** One
object with partial coverage, not a page-wide shift — which is the
distinction that matters, because a page-wide shift is Poppler #1565 and
this is not it.

⇢ **The affected population is "pages with a subtractive group" (15 files),
not "pages that blend inside one" (2).** Any future scoping note that
quotes the smaller number is quoting an answer to a different question.

---

## §2 — THE QUEUE, in the order I would take it

1. **`Pass 97.1g` — a NON-ISOLATED ordinary group on a subtractive page.**
   The one construct still approximated: it is composited as if isolated,
   so its backdrop is dropped and §11.4.4's backdrop removal is skipped.
   The arithmetic already exists (`compositor::remove_backdrop_cmyk`,
   written and tested in `97.1f`); what is missing is the **second content
   walk** over a copy of the backdrop, which the additive path already does
   (`Canvas::group`, `Self::Paint` arm). Counted as
   `cmyk_groups_approximated`. **This is a port, not a design.**
2. **Native colorant paths for images and shadings** (`Pass 97.1h`). Both
   bridge through sRGB today, and the reason is upstream of the canvas:
   `DecodedImage` holds a `Pixmap`, and `ColorRamp::at` returns three-channel
   sRGB when the ramp is *built*. Fixing either means widening a type one
   layer up. Counted per pixel as `cmyk_bridged_pixels`.
3. **`Pass 119.1`** — `unshare_form` (copy-on-write a shared form onto one
   page). Carried unstarted through **four** handoffs now; still a *separate
   verb*, not a mode of `edit_text` (`decision 076`).
4. **`Pass 80.0`** (note text on markup) and **`Pass 81.1`** (markup opacity,
   write half) — both `pdfceGUI` requests, both scoped, both untouched.
5. **`Pass 119.3`** — align `pdfce-render`'s nested-form resource fallback
   with `text_edit::forms`.
6. **The reference-strip threshold for `ghent-check.py`** — 12 patches sit
   in UNRESOLVED and three of them are at 0.96–0.99 correlation. Still
   deliberately not calibrated in the session that moved those numbers; see
   the afternoon handoff's §5 for why that is a rule and not laziness.

---

## §3 — THE THREE THINGS THAT WERE WRONG ON THE WAY, because each cost a cycle and each is a *class*

**1. THE COLLAPSE ORDER.** §11.4.7 converts to the device space **before**
compositing the white medium. The intuitive order is the wrong one, both
orders look like a page, and the conversion is non-affine so they differ by
up to **117 of 255 levels** on saturated ink. Caught only because `iccce`
filed a clause-by-clause check of a sentence I had written in chat.

⇢ *A design sentence nobody has checked against the primary source is a
guess with good posture.*

**2. A TRANSPARENT BACKDROP IS NOT A BACKDROP.** Handing knockout groups an
empty initial backdrop took `1_GWG161` from **2 traps to 15** — worse than
doing nothing at all. A knockout group's entire definition is *"composite
against the group's initial backdrop"*.

⇢ *When a construct is defined in terms of a thing, supplying an empty
version of that thing is not a degraded implementation, it is a different
one.*

**3. ★ THE TWO CONVERSIONS ARE FOR DIFFERENT JOBS AND MIXING THEM DRIFTS.**
pdfce has a **calibrated** CMYK→sRGB lattice and an **exactly invertible**
max-GCR pair. A *terminal* conversion wants accuracy; a *round trip* wants
invertibility and **does not care about accuracy at all**, because the value
never reaches a screen in that form. Using the accurate one on a round trip
left `1_GWG161` at 10 traps; the invertible one took it to 4, and carrying
shape separately took it to 0.

⇢ This one generalises past pdfce and is in the reply to `iccce` as a
possible API distinction on their side.

---

## §4 — COUNTERS, AND ONE WHOSE MEANING CHANGED

`render-page`'s stable stdout line gained **five** keys, all appended last:
`cmyk_buffer`, `cmyk_buffer_refused`, `cmyk_bridged_pixels`,
`cmyk_groups_approximated`, `cmyk_unbridged_images`.

★★ **AND `blends_in_wrong_space` WAS NARROWED, WITHOUT WHICH THIS PASS
WOULD HAVE READ AS A NO-OP.** It used to increment on the blending *space*
alone, because pdfce had no way to honour §11.3.4 and every such blend
really was additive. It now increments only when the paint target is **not**
a colorant buffer.

Leaving it alone was tried, in the same session, and produced this:
`tools/measure-blend-space.py` went on reporting **107 of 107 wrong** on the
Ghent suite *after* the buffer landed and two patches started passing. **A
shortfall counter that cannot see its own fix reports the fix as nothing** —
and that script is the only instrument anyone runs at corpus scale for this
question, so it would have said so indefinitely.

⇢ *When you fix a thing, check the counter that measures the thing. It was
written under the assumption that the thing could not be fixed.*

---

## §5 — WHAT IS STILL NOT DONE, named so it does not read as done

- **Non-isolated ordinary groups on a subtractive page** — composited as if
  isolated. Queue item 1.
- **Images and shadings** reach the colorant buffer as converted sRGB.
  Queue item 2.
- **Implicit knockout.** Unchanged by this Pass and still true: only
  explicit `/K true` is honoured. §9.3.8's `/TK` **defaults true** (every
  text object), §11.7.4.4 makes `B`/`b` knockout, §11.6.7 makes shading
  patterns knockout. **The one pdfce implements is the rarest.**
- **`/TR` on a soft mask** is read, counted, never evaluated.
- **`/AIS true` is not distinguished** from `/AIS false` for a group mask.
- **Spot colorants.** Four planes, not runtime `N`. Every remaining Ghent
  FAIL is now in this bucket.
- **Per-paint rendering intent** (§11.7.5.3) — pdfce carries one intent per
  page. `iccce` has costed the alternative (`≈13` s of transform build,
  `≈290` MiB for a full intent × BPC cache) and asked for the consumer fact.
  **No corpus measurement of mid-page intent switching has been taken.**
  That is a gap in what we know, not a report that the number is zero.

---

## §6 — TWO SMALL THINGS ABOUT THE BUILD ITSELF

**`f32`, and the decision lives on one type alias** (`cmyk_buffer::Chan`).
The reason floats are needed at all is §11.4.4's `1/α_gn`, where an 8-bit
half-level becomes 25 levels and an `f32` one becomes about 1/2600th of a
level. `f64` was declined and `iccce` was told so, per their request —
widening `f32`→`f64` is exact and happens once per pixel at the collapse,
not inside the blend loop.

**A display list is now REFUSED on a subtractive page**
(`PoisonReason::ColorantBuffer`). Its replay target is a `Pixmap`, so a
cached page would render a *different and worse* picture than the uncached
one. That is a plausible wrong picture, which is what every other
`PoisonReason` in that module exists to prevent.

---

## §7 — WHAT THE PREVIOUS HANDOFF SAID THAT IS NOW SUPERSEDED

- Its §1 named `Pass 97.1e` as "the next build" and predicted the shape
  correctly, including that `BrushSpec` would need to carry authored
  colorants and that `paint_nonseparable`/`paint_overprint` were the
  template. **All three held.** It named two shortfalls that did not
  survive contact: it expected the buffer to be `CMYK + alpha`, and the
  missing shape plane cost thirteen traps (§3).
- Its §3's `blends_in_wrong_space` entry — *"the shortfall. This is the one"*
  — is still true and now means something narrower. See §4.
- Its §5 (the un-calibrated reference-strip threshold) is **unchanged and
  still owed**; nothing this session touched the instrument.
- Its §0.5 (`v0.7.0` released, tag in no bundle on disk) is **unchanged**.
  **Two more code commits now sit past that tag.** Cutting a bundle is
  still the operator's call.
