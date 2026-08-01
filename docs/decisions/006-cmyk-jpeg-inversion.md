# 006 — The Adobe CMYK/YCCK JPEG inversion rule for DCTDecode images

**Date:** 2026-07-31
**Status:** Decided
**Decided by:** KenAgent (`autonomous-builder`), per the
`docs/decisions/README.md` protocol
**Requested by:** `pdfce-engineer`
**Supersedes:** nothing
**Closes:** decision 005 §5.5 (the deliberately-open CMYK-inversion
question) and 005 §9 revisit trigger 2
**Corrects:** the 005 Addendum of 2026-07-30 — on the instance count
(9, not 12) and, more importantly, on its inference that the affected
files "likely render inverted today." They do not. pdfce renders every
one of them correctly.
**Amends:** `docs/ROADMAP.md` (adds R29–R31, clarifies R26's status);
`D:\Dev\Rag-Specialized\PDF_Spec\filters\filter__dct.md` (closes its
SOURCING GAP with a *negative* result and rewrites its hazard note)
**Does not touch:** `LEGAL.md` — no new dependency, no license
implication. The one artifact consulted under copyright (Adobe TN
#5116) is reported as fact, never bulk-quoted.
**Requires no change to shipped behavior.** Everything this record
examines is already correct in the tree. The deliverables are a
permanent rule, a corrected diagnostic, and documentation that stops a
future engineer from "fixing" something that is not broken.

---

## 1. Context

Decision 005 §5.5 declined, on principle, to invent a rule for the
"Adobe CMYK inversion" — the widely-repeated claim that CMYK JPEGs
embedded in PDFs are frequently stored with complemented samples.
Pass 2.1 therefore shipped raw-CMYK passthrough plus a
`dct_cmyk_images` watch counter, so that the first real four-component
JPEG would announce itself.

That was the right call, and this record's most important finding is
*how* right: the guess §5.5 declined to make — "invert when the APP14
marker is present" — would have broken every real four-component JPEG
in the corpus. §5.5's reasoning is vindicated, not merely excused.

### 1.1 What the 005 Addendum claimed

The post-Pass-2.1 addendum (2026-07-30) corrected §3.2's "zero
four-component JPEGs" measurement, reporting **12** in veraPDF's
"6.2.4.3 Uncalibrated -Device colour spaces" suite, and drew this
inference:

> Under §5.5's deliberate no-guess posture (pass raw samples, let
> `/Decode` do its documented job) pdfce currently applies no
> inversion, so **these 12 files likely render inverted today**.

Both halves of that sentence needed testing. The premise (the count)
and the conclusion (the rendering) are separate claims, and this record
tests them separately. **The premise is off by three; the conclusion is
flatly false.**

### 1.2 What this decision had to establish

1. **Empirically:** what does pdfce actually produce for these files,
   and what is the correct output? Not "what does the source suggest" —
   what do the pixels say, against an independent production engine.
2. **By sourcing:** where does the inversion convention actually come
   from, what does the normative-by-reference document (Adobe TN #5116)
   actually say, and what condition do the shipping PDF engines
   actually test?
3. **By decision:** the exact rule; where it belongs given R26's
   layering; how it composes with an explicit `/Decode` array; and what
   the diagnostics should say now that the question is settled.

### 1.3 Why this was worth doing properly rather than quickly

Getting this wrong produces a photographic negative that *looks
deliberate*. There is no crash, no diagnostic, no visual cue that
anything went wrong — a print-origin image simply renders as its own
inverse, and a reviewer who has not seen the original cannot tell.
That is the exact failure profile `CLAUDE.md`'s claim-verification rule
exists to prevent, and it is why 005 §5.5 refused to guess.

---

## 2. Options considered

| | Rule | Who ships it |
|---|---|---|
| **A** | **Never invert.** APP14 selects the Table 13 colour transform only; `/Decode` is the sole polarity control. | pdf.js (PDF path), pdfium, MuPDF (PDF path), Poppler |
| **B** | Invert when the Adobe APP14 marker is present. | **Nobody.** Tried and reverted: cairo, Firefox |
| **C** | Invert when effective transform is 0 / no marker **and** no `/Decode` (the "Photoshop heuristic"). | Nobody, as an auto-apply |
| **D** | Invert only for YCCK (`Adobe_transform == 2`). | MuPDF's *standalone* `load-jpeg.c` only |
| **E** | Invert every four-component JPEG unconditionally. | ImageMagick, libvips, pdf.js *standalone* build |

Plus a placement question (**F**): if a rule were needed, does it
belong in `pdfce-core`'s DCT adapter or in `pdfce-render`'s §8.9.5.2
machinery, given R26?

---

## 3. Evidence

### 3.1 The corpus re-survey — the count is 9, not 12

Two independent raw-byte scans of all **2,914** PDFs under
`fixtures/external`, using different anchors so that a bug in one would
not reproduce in the other:

- **Scan 1** — locate every `stream`/`endstream` pair, filter to
  dictionaries mentioning `DCTDecode`, walk the JPEG markers to SOF and
  APP14.
- **Scan 2** — locate every `/DCTDecode` occurrence, take the following
  `stream` within 400 bytes, walk the markers, and read the enclosing
  dictionary back from the object header.

Both scans agree exactly:

| Metric | Value |
|---|---|
| Four-component DCT image instances | **9** |
| Distinct JPEG payloads | **2** |
| Containing PDFs | 9 (in `PDF_A-2b` and `PDF_A-4`) |
| Pages per PDF | 1 (so no multi-draw inflation) |
| SOF marker / precision / components | `0xC0` baseline / 8-bit / 4, IDs `[0,1,2,3]` |
| Dimensions | 300 × 232 |
| Adobe APP14 | present on **all**; version 100, flags 0/0 |
| **APP14 transform byte** | **2 (YCCK) on all 9 — no other value appears** |
| `/ColorSpace` | `/DeviceCMYK` on all 9 |
| `/Decode` array | **absent on all 9** |
| `/DecodeParms` / `/ColorTransform` | absent on all 9 |

The figure **12** could not be reproduced by either scan. Page counts
rule out the obvious explanation (one image drawn on several pages).
`fixtures/synthetic/` contains no DCT images at all. The count should be
re-derived from whatever produced it; **9** is what two independent
methods find, and nothing in this record depends on the difference —
the 9 are homogeneous, so a hypothetical 3 more of the same shape would
not change a single conclusion.

The engineer's brief cited a path under `veraPDF test suite 6.8/PDF_A-1b/…`.
No such directory exists in this checkout. The equivalent file is:

```
fixtures/external/veraPDF-corpus/PDF_A-2b/6.2 Graphics/
  6.2.4.3 Uncalibrated -Device colour spaces/
  veraPDF test suite 6-2-4-3-t02-pass-a.pdf
```

Its image XObject, verbatim:

```
13 0 obj
<<
/BitsPerComponent 8
/ColorSpace /DeviceCMYK
/Filter /DCTDecode
/Height 232
/Length 5088
/Name /X
/Subtype /Image
/Type /XObject
/Width 300
>>
```

No `/Decode`. No `/DecodeParms`. It relies entirely on the codestream —
exactly as the brief described.

### 3.2 The premise, tested: pdfce is already correct

**Ground truth.** `pypdfium2` — pdfium, the engine inside Chrome —
renders the page. The image is a **light-blue Space Invader on a white
background**. Unambiguous: an inverted render would be an orange
invader on black, which nobody could mistake for correct.

**pdfce's render of the same file** is the same light-blue invader on
white.

Across all **9** real corpus files, at scale 1.0, pdfce versus pdfium:

| File | mean Δ per channel | sign agreement about mid-grey |
|---|---|---|
| `t02-pass-a` (A-2b) | `[0.1, 0.5, 0.3]` | 100 % |
| `t02-pass-e` (A-2b) | `[-0.2, 0.3, 0.2]` | 100 % |
| `t02-fail-d` (A-2b) | `[0.1, 0.4, 0.4]` | 100 % |
| `t02-fail-e` (A-2b) | `[0.1, 0.4, 0.4]` | 100 % |
| `t04-fail-p` (A-4) | `[0.1, 0.4, 0.4]` | 100 % |

**There is no polarity difference of any kind.** The 005 Addendum's
"these 12 files likely render inverted today" is falsified.

**Why.** Instrumenting `pdfce_core::image_codec::decode_image` directly
(a throwaway probe crate outside the workspace) gives pdfce's CMYK
samples for `t02-pass-a`:

```
300x232 comps=4 bpc=8 model=Cmyk
  TL(2,2)   = [0, 3, 6, 0]      <- near-zero ink: correctly white
  CENTRE    = [81, 25, 8, 1]    <- cyan-dominant: correctly light blue
```

Recovering the **raw stored** components (by patching the APP14
transform byte to 0 so the passthrough path exposes them) gives
`TL = [253, 126, 130, 0]` as `(Y, Cb, Cr, K)`. pdfce's
`ycck_to_cmyk_in_place` maps that to `[0, 3, 6, 0]` — and so does
libjpeg's `ycck_cmyk_convert`, to within 2 units of YCbCr rounding.
**pdfce's YCCK inverse already replicates the reference implementation
exactly.**

### 3.3 The methodological trap that nearly inverted this decision

The *first* independent reference consulted was Pillow. It reported
`TL = (255, 252, 249, 255)` — the exact complement of libjpeg's answer —
which reads as damning evidence that pdfce is inverted.

It is not. `PIL.JpegImagePlugin` contains:

```python
if self.mode == "CMYK":
    rawmode = "CMYK;I"  # assume adobe conventions
```

Pillow applies its `;I` (inverted) rawmode to **every** four-component
JPEG, unconditionally, with no marker test — a 1996 Photoshop 2.5
workaround that is still the default. Its CMYK output is the complement
of libjpeg's, always.

Had Pillow been trusted as ground truth, this record would have
concluded that pdfce inverts, "fixed" it, and broken all 9 files while
producing a green test suite (because the fixtures would have been
built against the same wrong reference). Only the pdfium **page render**
and the source-level audit caught it.

This is generalized into **R31**.

### 3.4 The controlled variant matrix

Six minimal PDFs were hand-built wrapping the **same entropy-coded
JPEG** at 1:1, varying only two things: the APP14 transform byte, and
the presence of `/Decode [1 0 1 0 1 0 1 0]`. Rendered by pdfium and by
pdfce at scale 1.0.

| Variant | APP14 | `/Decode` | pdfium | pdfce | Polarity |
|---|---|---|---|---|---|
| `v2` | transform 2 | — | correct image | correct image | **agree** |
| `v2_dec` | transform 2 | `[1 0]×4` | black | black | **agree** |
| `v0` | transform 0 | — | no inversion | no inversion | **agree** |
| `v0_dec` | transform 0 | `[1 0]×4` | dark | dark | **agree** |
| `vn` | marker removed | — | no inversion | no inversion | **agree** |
| `vn_dec` | marker removed | `[1 0]×4` | dark | dark | **agree** |

(The marker is "removed" by rewriting the `APP14` marker byte to `COM`,
preserving every byte offset so the xref stays valid.)

Two conclusions, both load-bearing:

1. **pdfium applies no inversion on any branch** — not for YCCK, not
   for raw CMYK, not for a missing marker.
2. **pdfce matches pdfium on all six**, including both `/Decode`
   branches. The `/Decode` compose path is verified, not assumed.

### 3.5 What the shipping engines actually test

Source-level audit of four production PDF engines. Every condition
below is the actual boolean in the code, not a paraphrase.

| Engine | Inverts on the PDF path? | The actual condition | `/Decode` handling |
|---|---|---|---|
| **pdf.js** | **No** | `#getLinearizedBlockData`'s `[-256,255,…]` transform requires `!isSourcePDF && numComponents===4` **and** an `IMAGE_DECODERS` build. `readBlock()` always passes `isSourcePDF: true`. | Compiled into an affine `decodeTransform`. **Not** dropped, **not** negated — the "flips the `/Decode` array" special case does not exist in master. |
| **pdfium** | **No** | `m_bInverted` does not exist in the tree. The sole `saw_Adobe_marker` use (`jpegmodule.cpp:184-190`) is **3-component only**. Same logic verified back to 2016. | Folded into `decode_min_`/`decode_step_`; `[1 0…]` → `1 - data/255`. |
| **MuPDF** | **No** (PDF path) | `filter-dct.c` inverts on `invert_cmyk && num_components == 4` — but `pdf-stream.c`'s `build_compression_params` **hard-codes `invert_cmyk = 0`**. | Generic `fz_decode_tile` affine map. On **write**, `pdf_add_image` emits `/Decode [1 0 1 0 1 0 1 0]` *instead of* touching bytes. |
| **Poppler** | **No** | `DCTStream.cc` (271 lines) contains **zero** inversion arithmetic. APP14 sets `colorXform` only: `case 4: jpeg_color_space = colorXform ? JCS_YCCK : JCS_CMYK;` | Poppler's generic `GfxImage` `/Decode` machinery. |
| **libjpeg-turbo** | **No**, and refuses to | `examine_app14` records metadata only. `ycck_cmyk_convert`'s `MAXJSAMPLE -` touches only C/M/Y with **K unchanged** — the YCCK *definition*, not a polarity flip. | n/a |

`libjpeg.txt`, the closest thing to an authoritative account:

> "it appears that Adobe Photoshop writes inverted data in CMYK JPEG
> files: 0 represents 100% ink coverage… arguably a bug in Photoshop…
> We cannot 'fix' this in the library… because that would break other
> applications, notably Ghostscript."

**The opposite camp**, recorded so it is never mistaken for precedent:
ImageMagick's `coders/jpeg.c` inverts on `case 4:` alone —
`saw_Adobe_marker` appears **nowhere in the file**. libvips does the
same and left an APP14-gated refinement **commented out**. Both are
self-consistent on their own round-trip and wrong as a PDF-reader
model.

**The revert trail** — the decisive practical argument against
option B:

- **cairo** `b207a932` added `/Decode` on APP14 presence → double-inversion
  regression → **reverted** (cairo issue 156).
- **Firefox** bug 674619, a marker-heuristic inversion → **backed out**,
  RESOLVED INCOMPLETE.

Marker-gated inversion is not an untried idea. It is a **tried and
withdrawn** one, twice.

### 3.6 Adobe TN #5116 — the primary, and a negative result

ISO 32000-1 §7.4.8 footnote *a* makes TN #5116 normative by reference
("PDF DCT Encoding shall exactly follow those rules established by
Adobe for PostScript"), and `filter__dct.md` has carried a **SOURCING
GAP** ever since, on the APP14 transform-byte semantics.

The primary was obtained and read: *Supporting the DCT Filters in
PostScript Level 2*, 24 November 1992, PN LPS5116.

**§18 — the APP14 layout (facts, not quoted):** marker `0xFFEE`
("APPE"); two-byte length field = 14; the text `Adobe` as a
five-character ASCII big-endian string; a two-byte DCTEncode/DCTDecode
version number (presently `0x65`); two-byte `flags0` (bit `0x8000` =
encoder used Blend=1 downsampling); two-byte `flags1`; **one-byte
colour transform code**. Convention: zero bits are benign; `flags1`
one-bits are essential for decoding; a decoder skips any APPE segment
not beginning with `Adobe`.

Two findings change what may be cited, and both are *negative*:

1. **TN #5116 §18 does NOT enumerate the transform values 0/1/2.**
   That mapping is **de facto**, sourced from libjpeg's `jdapimin.c`
   and ExifTool, and paralleled by ISO 32000-1 Table 13's own
   `ColorTransform` semantics. `filter__dct.md`'s SOURCING GAP closes
   with the layout sourced to TN #5116 and the *values* explicitly
   sourced elsewhere, with the caveat stated in the file.

2. **The word "invert" appears zero times in the document.** §13.1's
   only `255 −` is the reversible CMYK→YCCK step: the forward transform
   applies RGB→YCC to `R = (255−C)`, `G = (255−M)`, `B = (255−Y)`, with
   `K` passed through. **The inverted-CMYK storage convention is not in
   the normative-by-reference source at all.** It is undocumented
   Photoshop behavior, and there is no polarity flag anywhere in APP14.

The second finding is the technical heart of this decision, and §5.3
draws out why.

### 3.7 A separate defect found in passing — colorimetry, not polarity

While establishing that pdfium and pdfce agree on *polarity*, they were
found to disagree on *colour*. On the 300×232 image at 1:1:

| Measure | Value |
|---|---|
| max abs Δ per channel | `[11, 37, 30]` |
| 95th percentile per channel | `[5, 27, 18]` |
| mean abs Δ per channel | `[2.47, 9.61, 6.82]` |
| pixels differing > 8 in some channel | **37.4 %** |
| the saturated blue: pdfce vs pdfium | `(173,229,246)` vs `(168,203,229)` |

Cause: `Rgb::from_cmyk` (`gstate.rs:112`) is the naive additive
`1.0 − min(c + k, 1.0)`; pdfium uses its calibrated
`AdobeCMYK_to_sRGB1` table. Channel correlations remain 0.99+ and sign
agreement is 100 %, which is exactly why this does **not** confound the
polarity conclusion — but it is a real, visible fidelity gap.

**It is deliberately excluded from this decision.** It affects every
`DeviceCMYK` fill and stroke, not just JPEGs, and belongs in its own
ROADMAP entry (follow-up 8). Conflating it with polarity would muddy
both.

---

## 4. Decision

### 4.1 The rule — **Option A: never invert**

> pdfce **never** applies an "Adobe CMYK inversion" to DCTDecode
> samples. Not on APP14 presence. Not on transform-byte value. Not on
> component count. Not on producer sniffing.

The Adobe APP14 transform byte is consumed for exactly one purpose,
already correctly implemented: selecting the ISO 32000-1 Table 13
colour transform.

| Components | Effective transform | Action |
|---|---|---|
| 4 | 0 | raw CMYK passthrough |
| 4 | 1 or 2 | YUVK/YCCK → CMYK inverse (`ycck_to_cmyk_in_place`) |

The `255 − x` inside `ycck_to_cmyk_in_place` **is the YCCK definition**
(TN #5116 §13.1), not a polarity flip, and it is correct as shipped.
The existing doc comment saying so was right; only its closing sentence
("a separate, unsettled question") needs updating.

Table 13's three-level precedence chain — marker outranks
`/DecodeParms` outranks the component-count default — is already
implemented correctly at `dct.rs:160-168` and matches MuPDF and Poppler.

### 4.2 Placement — **nothing moves**

The rule is a null rule, so structurally there is nothing to place.
The *argument* is recorded because a future engineer will re-ask it:

- It would **not** be a Table 11-style filter parameter. That is R26's
  one named adapter exception (CCITT `/BlackIs1`), and it earns the
  exception by being a *declared parameter of the filter*. A producer
  convention that no parameter announces is the opposite of that.
- It would **not** be `/Decode` semantics either, so it does not
  naturally join `pdfce-render`'s §8.9.5.2 machinery.
- **The deciding argument:** applying it would require the codestream's
  APP14 byte *and* the image dictionary's `/Decode` presence, and no
  single layer owns both as *authority*. A rule whose inputs straddle
  the architecture's main seam is a rule the architecture is telling
  you not to write.

**Diagnostic classification, by contrast, does belong in
`pdfce-core::image_codec::dct`.** `decode()` already receives `dict`,
so it can *observe* `/Decode` while remaining forbidden to *apply* it.
R26 gains that one clarification: **observing is not applying.**

### 4.3 `/Decode` interaction — **composes normally**

`/Decode` does not compose *with* the convention, does not replace it,
and cannot double-apply against it — because **no convention is
applied**. `/Decode` is read from the image dictionary and applied by
`pdfce-render` as the ordinary §8.9.5.2 affine map to the codec's
output samples, identically to every other image.

`/Decode [1 0 1 0 1 0 1 0]` **is** the inversion, and is the sanctioned
mechanism by which a producer declares inverted storage — a direct
descendant of the PostScript image operator's decode array, which is
how Adobe's own products compensated before PDF existed.

**No code change is required.** `image.rs:560-567` already computes a
signed slope and already carries the right comment:

> "A **NEGATIVE** slope is the `[1 0]` inversion and must survive —
> this is exactly where a `min`/`max` 'normalization' would destroy it."

That comment is now load-bearing under R29 and must not be softened.

### 4.4 Diagnostics — split the counter in two

`dct_cmyk_images` currently fires on **every** four-component JPEG and
prints a warning telling the operator to "check the colours." It now
cries wolf on 9 known-good files. Replace it with two counters,
classified in `dct::decode`:

| Counter | Condition | Operator-facing treatment |
|---|---|---|
| `dct_cmyk_images` | 4 components, effective transform 1 or 2 | **Neutral census.** Verified correct against pdfium. No warning. |
| `dct_cmyk_polarity_unverifiable` | 4 components **and** effective transform 0 **and** no `/Decode` | **Named warning** (R30). The one shape where the undocumented Photoshop convention can still bite. |

Current corpus: 9 and **0** respectively.

---

## 5. Rationale

### 5.1 Why "never invert" rather than any heuristic

Four independent production PDF engines — pdf.js, pdfium, MuPDF and
Poppler — implement precisely this rule, and they were written by teams
that do not share code, tests, or bug trackers. Convergence at that
level is the strongest evidence available short of a normative
statement, and there is no normative statement to be had (§3.6).

Against that, the marker-gated alternative has a **revert trail**:
cairo shipped it and reverted it after a double-inversion regression;
Firefox shipped a variant and backed it out. Adopting option B would be
re-deriving a known-bad idea from first principles, which is exactly
what 005 §5.5 was written to prevent.

The empirical check settles it locally: option B applied to pdfce's
corpus would invert **9 of 9** currently-correct files.

### 5.2 Why the folklore is true but inapplicable

"Adobe CMYK JPEGs are stored inverted" is a real phenomenon with a real
root cause: pre-1994 Photoshop wrote complemented CMYK samples (0 = 100 %
ink), Adobe never documented it, and IJG declined to compensate in
libjpeg because doing so would have broken Ghostscript.

But Adobe's own products compensate **out of band** — in PostScript via
the image operator's decode array, in PDF via
`/Decode [1 0 1 0 1 0 1 0]`. The convention was never meant to be
detected from the codestream; it was meant to be **declared by the
container**. A PDF reader that tries to detect it is solving a problem
the format already solved.

And it cannot be detected reliably in any case: APP14 presence is a
leaky proxy, because libjpeg emits APP14 for non-Adobe output too —
every ImageMagick-written CMYK JPEG carries one.

### 5.3 Why YCCK needs no inversion, and where the real risk lives

This is the part most likely to be misread, so it is stated as an
argument rather than an assertion.

TN #5116 §13.1 defines the CMYK→YCCK forward transform as RGB→YCC
applied to `R = (255−C)`, `G = (255−M)`, `B = (255−Y)`, with `K` passed
through — **defined in terms of true ink values**. Therefore inverting
it (YCC→RGB, then `C = 255 − R`) recovers **true ink directly**. No
further step exists to take.

That is why libjpeg, pdfium and pdfce all agree on this file with no
extra handling, and it is why `ycck_cmyk_convert`'s `MAXJSAMPLE −` is
so often misread as "the Adobe inversion." It is not. It is the
transform's own definition, and pdfce's `ycck_to_cmyk_in_place` is its
faithful reimplementation.

The Photoshop polarity problem therefore lives **exclusively** in the
transform-0 / no-APP14 raw-CMYK branch, where the stored bytes are
whatever the producer chose and nothing in the codestream disambiguates
them.

**The corpus contains zero files of that shape.** Every one of the 9 is
transform 2. The case everyone worries about is not the case that
showed up.

### 5.4 Why the residual case is reported rather than repaired

A transform-0 CMYK JPEG with no `/Decode` is genuinely ambiguous. All
four reference engines render it un-inverted and accept the gap; none
has a test fixture for it.

pdfce could differ — but not silently. `CLAUDE.md` rule 4 is explicit:
every algorithmic suggestion is a reviewable hint the operator accepts
or overrides, never a silent auto-apply. A polarity flip inferred from
a producer convention is the *paradigm* case: high-impact, invisible
when wrong, and undetectable by the operator without the original.

So R30 names it, counts it, and stops. If a repair is ever offered it
is a per-image reviewable toggle, never a default. pdfce's
differentiator here is not that it fixes what others do not — it is
that it **tells you**.

### 5.5 Why "verify the reference's conventions" becomes a rule

R31 exists because this investigation nearly inverted on a false
positive from Pillow (§3.3), and because pdfce is a **feature-parity
project**: it will lean on reference implementations constantly, for
rendering, forms, redaction, signatures. Every one of those comparisons
carries the same hazard — that the reference silently normalizes
something before you see it.

The cost of the rule is one sentence in a decision record. The cost of
skipping it, here, would have been shipping a photographic-negative
regression with a green test suite.

### 5.6 Why 005 §5.5 was right, stated plainly

It is worth recording as precedent, because the temptation to guess
recurs. §5.5 identified the correct rule ("pass raw CMYK through, let
`/Decode` do its documented job") **without** the sourcing to justify
it, and chose to ship it as an unjustified default with a watch counter
rather than dress it up as a decision. Sourcing has now confirmed it is
the four-engine consensus.

Had §5.5 instead guessed the *plausible* rule — "invert when APP14 is
present" — it would have been wrong on 9 of 9 files, and the failure
would have looked deliberate. **The no-guess posture did not merely
avoid embarrassment; it avoided the bug.**

---

## 6. What this decision produces

### 6.1 Standing rules (binding; add verbatim to `ROADMAP.md`, continuing R1–R28)

- **R29 — pdfce never applies an "Adobe CMYK inversion."**
  Four-component DCTDecode samples reach `pdfce-render` exactly as the
  codec's own Table 13 / TN #5116 §13.1 mandated transform produced
  them. `/Decode` is the sole polarity control for every image, in
  every colour space, at every bit depth. No APP14-conditioned,
  transform-byte-conditioned, component-count-conditioned, or
  producer-sniffed polarity flip may be added to any layer without a
  NEW decision record citing a source the four-engine consensus does
  not already contradict. Sourced: pdf.js, pdfium, MuPDF (PDF path) and
  Poppler all implement exactly this; marker-gated inversion has been
  shipped and reverted twice upstream (cairo issue 156, Firefox bug
  674619).

- **R30 — The residual inversion risk is reported, never repaired.**
  A four-component DCT image with **no `/Decode`** and an **effective
  `ColorTransform` of 0** (APP14 transform byte 0, or no Adobe marker)
  is the one shape where the undocumented Photoshop polarity convention
  can still produce a photographic negative. It gets its own named
  diagnostic, distinct from the benign YCCK census. If a repair is ever
  offered it is an operator-facing, per-image, reviewable toggle — per
  `CLAUDE.md` rule 4 — never a silent auto-apply and never a default.
  All four reference engines share this gap and accept it; pdfce's
  differentiator is that it names it.

- **R31 — A reference decoder is evidence only after its own
  conventions are verified.** Before treating any third-party render or
  decode as ground truth, establish what normalization the tool applies
  on its own initiative, and record it in the decision. Pillow applies
  rawmode `CMYK;I` to every four-component JPEG ("assume adobe
  conventions", no marker test), which produced a false positive for
  this exact investigation. Prefer a full-page render from a production
  PDF engine (pdfium via `pypdfium2` is the cheapest) over a bare
  image-library decode, and prefer a source-level read of the condition
  over both.

- **R26 — status change, text unchanged.** Its clause forbidding "an
  'Adobe CMYK inversion', or any polarity flip of its own" was
  provisional pending 005 §5.5's open question. It is now **permanent
  and sourced** via R29. One clarification is added: the codec adapter
  **may observe** the image dictionary for the purpose of emitting
  diagnostics (`dct::decode` already receives `dict`). **Observing is
  not applying.**

### 6.2 Code changes — documentation and diagnostics only

**No behavioral change.** Verified correct as shipped and explicitly
not to be touched: `ycck_to_cmyk_in_place`, `route`,
`passthrough_target`, `needs_ycck_inverse`, the `/Decode` ramp in
`image.rs`, and Table 13's precedence chain.

1. `crates/pdfce-core/src/image_codec/dct.rs` module docs (~94–107) —
   replace "the CMYK-inversion question is deliberately left open" with
   the settled rule, R29, and the TN #5116 §13.1 citation.
2. `ycck_to_cmyk_in_place` doc (~397–401) — keep "This is not the
   'Adobe inversion'"; change "a separate, unsettled question left to
   `/Decode`" to "settled by decision 006: there is no Adobe inversion;
   `/Decode` is the sole polarity control (R29)."
3. `CodecNotes::cmyk_image` doc (`mod.rs:225-229`) — still says "**Zero**
   exist in the 2,914-file conformance corpus." Now doubly wrong.
   Rewrite alongside the split in §6.3.

### 6.3 Diagnostics

Split `cmyk_image` into `dct_cmyk_images` (benign census, no warning)
and `dct_cmyk_polarity_unverifiable` (R30, named warning), classified in
`dct::decode`, which has both `frame.adobe_transform` and `dict`.

Update the operator-facing text in `pdfce-cli/src/main.rs:545-549`
(which currently warns on every four-component JPEG),
`pdfce-cli/src/main.rs:105-111`, and
`pdfce-gui/src/ui_text.rs:359`. Only the R30 counter warrants a
warning, and it should name the actual risk and cite 006.

### 6.4 Fixtures

Add the six controlled variants of §3.4 as regression fixtures. They
share one entropy-coded JPEG and vary only the APP14 transform byte and
`/Decode`, which is precisely the axis a silent upstream change would
move.

Construction (reproduced so it can be rebuilt from this record alone):

- Extract the JPEG from `6-2-4-3-t02-pass-a.pdf`.
- `v2` = transform byte `2`; `v0` = transform byte `0`; `vn` = rewrite
  the `APP14` marker byte to `COM` (`0xFE`) so the segment becomes a
  comment — **same length**, so every offset survives.
- Each in two forms: with and without `/Decode [1 0 1 0 1 0 1 0]`.
- Wrap each in a minimal single-page PDF with the image at 1:1, so a
  scale-1.0 render maps one sample to one pixel and no resampling
  allowance is needed.

**Assert the CMYK sample values at named pixels** (e.g. `v2` top-left
`[0,3,6,0]`, centre `[81,25,8,1]`; `v0` top-left `[253,126,130,0]`), not
merely that decoding succeeds. The sample values are what a silent
`zune-jpeg` passthrough change would break.

**Licensing:** the veraPDF corpus is **CC BY 4.0**, which permits
derivatives with attribution. Record the attribution and confirm
against `LEGAL.md` §5.

### 6.5 RAG corrections

Dispatch `pdfce-spec-librarian` for
`D:\Dev\Rag-Specialized\PDF_Spec\filters\filter__dct.md`:

- **Close the SOURCING GAP** — layout from the TN #5116 primary; value
  semantics cited to libjpeg `jdapimin.c` + ISO 32000-1 Table 13, with
  the explicit caveat that TN #5116 does **not** enumerate them.
- **Rewrite the "Known real-world hazard" paragraph.** It currently
  says Adobe CMYK JPEGs are "frequently stored inverted" and that
  "correct handling interacts with the image's `/Decode` array" — which
  overstates the hazard for YCCK and understates that `/Decode` **is**
  the handling.
- **Record the negative result**: "invert" appears zero times in TN
  #5116.
- **Add** the four-engine consensus table and the cairo/Firefox revert
  trail.

Dispatch `pdfce-librarian` for `C:\personal_rag\pdf\` (LLM-optimized
per `CLAUDE.md` rule 14): Pillow's `CMYK;I` trap; the four-engine
consensus with exact conditions; libjpeg's `ycck_cmyk_convert` `255−`
being definitional; and the corpus re-survey correction.

---

## 7. What this decision explicitly does NOT decide

- **The `DeviceCMYK` → RGB colorimetry gap (§3.7).** Real, measured,
  and out of scope. It affects every `DeviceCMYK` fill and stroke, not
  just images. Separate ROADMAP entry.
- **Whether pdfce should ever *write* CMYK JPEGs.** R28 forbids image
  encoders outright. If revisited, MuPDF's `pdf_add_image` is the
  model: emit `/Decode [1 0 1 0 1 0 1 0]`, never write inverted bytes.
- **ICC-based CMYK.** `/ICCBased` with `N 4` is out of scope here;
  `color__iccbased.md` owns it.
- **Whether the 12-vs-9 discrepancy indicates a defect in the
  corpus-report tooling.** Flagged for re-derivation; nothing in this
  record depends on it.
- **Acrobat's own behavior** for a transform-0 CMYK JPEG with no
  `/Decode` — unverifiable (closed source), and carved out as R30's
  diagnostic rather than guessed.

---

## 8. Consequences

- **pdfce ships correct four-component JPEG rendering today**, and now
  knows it, with pixel evidence against pdfium rather than an
  assumption in either direction.
- **The scary diagnostic goes quiet on the benign case** and gets
  sharper on the one case that matters. Nine known-good files stop
  telling the operator to "check the colours."
- **R29 makes the null rule permanent.** A future engineer reading
  ImageMagick's `case 4:` or MuPDF's `load-jpeg.c` and concluding
  "everyone inverts" will hit a standing rule and a sourced record
  rather than an open question.
- **`filter__dct.md`'s SOURCING GAP closes with a negative result** —
  the canonical source does not contain what the folklore attributes to
  it. That is worth more than a positive citation would have been,
  because it disarms every future appeal to TN #5116 on this point.
- **pdfce inherits the one gap all four engines share** (transform-0
  CMYK without `/Decode`) but is, as far as this record can determine,
  **the only one that names it**.
- **The corpus's silence is now itself documented.** Conformance
  corpora do not contain transform-0 CMYK JPEGs. That absence is a
  property of the corpus, not of the world — the same lesson §3.1 of
  005 learned about CCITT and JBIG2 zeroes, recurring.

---

## 9. Revisit triggers

1. **A real four-component DCT image with effective `ColorTransform` 0
   (transform byte 0, or no Adobe marker) is met.** Zero exist today.
   This is the only shape R30's diagnostic is for and the only shape
   that could reopen the decision. Capture the file, render it in
   pdfium **and** Acrobat if available, and file the answer in
   `C:\personal_rag\pdf\` **before** proposing any code change.
2. **A four-component DCT image *with* a `/Decode` array is met on real
   data.** The compose path is currently verified only against
   synthetic variants.
3. **An organic corpus is assembled** (Photoshop export, InDesign /
   Distiller output, print-shop PDFs). That is where transform-0 CMYK
   actually lives.
4. **Acrobat's behavior for the residual case becomes observable.** The
   single unverifiable in this record.
5. **`zune-jpeg` changes its four-component passthrough semantics.**
   `ycck_to_cmyk_in_place` assumes zune returns **raw, un-normalized**
   YCC when `input_colorspace == output_colorspace == YCCK`
   (`worker.rs:41-44`, `copy_removing_padding_4x`). Verified empirically
   at 0.5.15, **not contractual**. A silent change flips polarity with
   no diff to review — §6.4's fixtures are the tripwire.
6. **pdfce ever gains a CMYK JPEG writer** (R28 currently forbids).
   MuPDF's writer posture is the model.
7. **`DeviceCMYK` → RGB colorimetry is addressed.** Re-run §3.4's
   variant matrix and pin the polarity conclusions *before* the colour
   change lands, so the two are never confounded.

---

## 10. Follow-up actions

1. **Append a second addendum to `005-image-codecs.md`** (append-only;
   do not edit the first). State the corrected count (9 / 2 / 9), the
   falsified inference, and that §5.5's no-guess posture was correct
   rather than merely cautious.
2. Rewrite `dct.rs` module docs (~94–107) per §6.2.1.
3. Update `ycck_to_cmyk_in_place`'s closing sentence per §6.2.2.
4. Split the counter and rewrite `CodecNotes::cmyk_image`'s doc per
   §6.3.
5. Update CLI and GUI note text per §6.3.
6. Add the six fixtures per §6.4, asserting sample values at named
   pixels; record the CC BY 4.0 attribution.
7. Dispatch `pdfce-spec-librarian` for `filter__dct.md` per §6.5.
8. Dispatch `pdfce-librarian` for the four `C:\personal_rag\pdf\`
   lessons per §6.5, and to add R29–R31 to `ROADMAP.md`.
9. Open a separate ROADMAP entry for the `DeviceCMYK` → RGB colorimetry
   gap (§3.7); scope it via `pdfce-acrobat-librarian` before committing
   engineering time.
10. Re-derive the "12" figure, or retire it.

---

## 11. References

**Local:**
`docs/decisions/005-image-codecs.md` §3.2, §4.1, §5.5, §9, and the
2026-07-30 addendum · `crates/pdfce-core/src/image_codec/dct.rs`
(`:160-168` precedence chain, `:204-206` YCCK dispatch, `:337-364`
routing, `:402-425` the inverse) · `crates/pdfce-render/src/image.rs`
(`:560-567` the signed ramp, `:772-801` colour spaces) ·
`crates/pdfce-render/src/gstate.rs:112` (`from_cmyk`) ·
`D:\Dev\Rag-Specialized\PDF_Spec\filters\filter__dct.md`

**Specifications:** ISO 32000-1 §7.4.8 + Table 13 + footnote *a* ·
Table 89, Table 90, §8.9.5.2 · Adobe TN #5116 §13.1 and §18 (primary
read; reported, not quoted — Adobe copyright) · ITU-T T.81

**Implementations:** pdf.js `src/core/jpeg_stream.js`, `src/core/jpg.js`
(issue #9513, PR #10031) · pdfium `core/fxcodec/jpeg/jpegmodule.cpp`,
`core/fpdfapi/page/cpdf_dib.cpp`, `core/fxcodec/progressive_decoder.cpp`
· MuPDF `source/fitz/filter-dct.c`, `source/pdf/pdf-stream.c`,
`source/fitz/image.c`, `source/fitz/load-jpeg.c` · Poppler
`poppler/DCTStream.cc` · libjpeg-turbo `jdmarker.c`, `jdapimin.c`,
`jdcolor.c`, `doc/libjpeg.txt` · ImageMagick `coders/jpeg.c` ·
`zune-jpeg` 0.5.15 `src/worker.rs:41-44`, `src/headers.rs:255`,
`:485-514` · `PIL.JpegImagePlugin` (the `CMYK;I` rawmode)

**Regression history:** cairo `b207a932` / issue 156 · Firefox bug
674619 · libvips issue 1283

**Corpus:** veraPDF corpus (CC BY 4.0), `PDF_A-2b` and `PDF_A-4`,
"6.2.4.3 Uncalibrated -Device colour spaces"

**Method note:** the survey scripts, the probe crate, and the variant
generators were throwaway instrumentation kept outside the workspace.
§3.1 and §6.4 describe them in enough detail to rebuild; §6.4's
fixtures are the retained artifact. Per 005 §11's own caveat about
unretained scripts — which is exactly how the "zero four-component
JPEGs" error survived — the *fixtures* are the durable evidence, not
the scripts.
