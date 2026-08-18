# Ghent PDF Output Suite 5.0 — per-patch reference (overprint patches)

**Written 2026-08-18** by the engineer, from a commissioned research pass
that extracted `GhentPDFOutputSuite50_ReadMes.pdf` (82 pp) and the combined
test-page captions.

**Why this file exists.** GWG's per-patch documentation is **not on the
web** — it ships inside the 126 MB individual-patches download
(<https://gwg.org/?wpdmdl=9076>; force HTTP/1.1, HTTP/2 truncates). The
ReadMes bundle is at <https://gwg.org/?wpdmdl=9080> (24,508,479 bytes, no
login). Re-obtaining this costs a download and an extraction pass, so the
extracted substance lives here.

**Label convention, carried from the research pass:**
- **PRIMARY (GWG ReadMe)** / **PRIMARY (GWG caption)** — quoted GWG text.
- **MEASURED** — inspection of the patch artwork itself.
- **DERIVED** — computed from ISO 32000-1 Tables 148/149. **Not published by
  anyone.** Treat as a strong prediction, not as ground truth.

Numbering: GWG010 = "patch 1.0", GWG192 = "patch 19.2".

---

## §0 — Two rules that change how the harness should score

**PRIMARY, stated by Stephan Jaeggi (Co-Chair, GWG Process Control
Subcommittee):**

> **"Faint X does not indicate a failure!"**

Evaluation is explicitly perceptual and explicitly tolerant — a human at
0.5 m / 20 in, *"you will not need a loupe"*. **`tools/ghent-check.py` may
therefore be over-counting**; it has a `CONTRAST_MIN` but that threshold was
calibrated against pdfce's own output, not against GWG's stated criterion.
Patches GWG pre-declares tolerant: **all ten cells of GWG020**, and **cell d
of each DeviceN patch**.

**The suite ships a Reference file** — `Ghent_PDF-Output-Test-V50_ALL_REFERENCE.pdf`,
in the same ZIP. Its texts are in Registration (`/Separation /All`) so they
appear in every separation. **pdfce is not currently using it as an oracle
and should.**

Composition of the 51: **27 CMYK-only, 8 SPOT, 16 CMS** (ICCBased /
colour-management). **So the spot axis is only 8 of 51** — that bounds the
direct conformance ROI of the n-channel work, though some CMS patches carry
spots too.

Slide decks (PRIMARY-adjacent, authored by a GWG officer):
<https://pdf-aktuell.ch/files/StephanJaeggi-Ghent_PDF_Output_Suite_5.pdf> (2018),
<https://www.fourpees.com/assets/media/Branded-imagery/Press-releases/220203_FourPeesCafe%CC%81_Stephan-Jaeggi_Ghent-PDF-Output-Suite-5.0_web.pdf> (2022).

---

## §1 — The five rules everything derives from

From ISO 32000-1 §8.6.7 + Tables 148/149:

- **R1** — `OP`/`op` true ⇒ colorants **not specified by the source colour
  space** are left unchanged. `OP` false ⇒ they are erased (painted 0.0).
- **R2** — `OPM 1` ⇒ additionally, a source component of **0.0 leaves that
  colorant unchanged**. Applies **only** when the current space is
  `DeviceCMYK` **specified directly** — *"shall not apply to the painting of
  images or to any colours that are the result of a computation, such as
  those in a shading pattern or conversions from some other colour space."*
- **R3** — `DeviceGray` is a **process** space: converting to a CMYK device
  it specifies **all four** process colorants, and `OPM` never applies to it.
  **DeviceGray cannot overprint DeviceCMYK, at either OPM.**
- **R4** — `Separation`/`DeviceN` specifies **only its named colorants**;
  all others unchanged under `OP` true. `OPM` never applies.
- **R5** — Table 149: *"For spot colour components, the value shall always
  be c_b"* (the backdrop).

**R3 corroborated by two vendors.** Peter Kleinheider (author of GWG041/132/133):
*"DeviceGray can not overprint DeviceCMYK"*. And Heidelberg Prinect:
*"'DeviceGray' colors overprint all spot colors lying lower down. However,
contrary to expectations, **CMY separations are knocked out**"* — with a
shipped remedy, *"Turn Overprinting Device Gray into K"*, which converts to
`/Separation/Black` because *"This conversion causes CMY separations to be
overprinted."*
<https://onlinehelp.prinect-lounge.com/Prinect_PDF_Toolbox/Version2021/en/Prinect/Color_management/Color_management-9.htm>

**Why `/Separation /Black` differs from `k`:** §8.6.6.4 reserves **only**
`/All` and `/None`; colorant names are *"arbitrary"*. `/Black` is matched
against *the device's* colorant list — on a subtractive device it paints K
alone; on an **additive** device a Separation *"never applies a process
colorant directly; it always reverts to the alternate colour space."* **For
pdfce that only works inside a simulated subtractive device** — another
route to the same n-channel conclusion.

---

## §2 — ★ GWG040 White Overprint — the patch that diagnosed pdfce's bug

**PRIMARY (ReadMe)**, 10 Nov 2006, Peter Claes. 4.0.1 replaced a withdrawn 4.0.

**MEASURED:** `/CS1 = [/Separation /GWG Green /DeviceCMYK]`, tint transform
`C0=[0,0,0,0] → C1=[0.5, 0, 1, 0]` — **GWG Green = 50C 0M 100Y 0K at full
tint, and carries no K**. `/CS2 = [/Separation /Black]`. Six ExtGStates
spanning OPM 0/1 × op true/false.

**PRIMARY (caption)** — 12 cells; left column OPM 0 (a–f), right OPM 1 (g–l):

| | OPM 0 | | OPM 1 |
|---|---|---|---|
| a | CMYK over spot | g | CMYK over spot |
| b | Gray over spot | h | Gray over spot |
| c | Sep. black over spot | i | Sep. black over spot |
| d | CMYK over CMYK | j | CMYK over CMYK |
| e | Gray over CMYK | k | Gray over CMYK |
| f | Sep. black over CMYK | l | Sep. black over CMYK |

**Only documented expected result — PRIMARY, deliberately unenumerated:**
*"Objects that are set to 0% and are set to overprint **disappear in most
cases, but not all cases**… includes examples of cases where objects would
be expected to disappear as well as cases where the proper behavior would be
to knock out the object below."*

### DERIVED per-cell truth table (not published by GWG)

| Cell | Result |
|---|---|
| **a** CMYK 0% over spot, OPM 0 | spot survives — **object INVISIBLE** |
| **g** same, OPM 1 | **identical to a — INVISIBLE** |
| **b** Gray 0% over spot, OPM 0 | R3 writes four process 0; spot unchanged — **INVISIBLE** |
| **h** same, OPM 1 | **identical to b** |
| **c** Sep/Black 0% over spot, OPM 0 | R4: Black 0, Green unchanged — **INVISIBLE** |
| **i** same, OPM 1 | **identical to c** |
| **d** CMYK 0% over CMYK, OPM 0 | all four written 0 → **knocks out — WHITE, VISIBLE** |
| **j** CMYK 0% over CMYK, OPM 1 | all four 0 leave backdrop — **INVISIBLE** |
| **e** Gray 0% over CMYK, OPM 0 | R3 → **knocks out — WHITE, VISIBLE** |
| **k** same, OPM 1 | R3: OPM inapplicable → **still knocks out — VISIBLE** |
| **f** Sep/Black 0% over CMYK, OPM 0 | R4: K zeroed, C,M,Y survive — **PARTIAL** |
| **l** same, OPM 1 | **identical to f** |

**Invariants:** a = g = b = h = c = i (all six "over spot" identical,
invisible); d, e, k knock out to white; j overprints; f = l partial.
**d vs j** is the sharpest OPM discriminator in the suite. **j vs k** is the
"DeviceGray ≠ DeviceCMYK" discriminator — same tint, same OPM 1, opposite
result.

### ★ Why this diagnosed pdfce

> Flattening spot through the tint transform to RGB before compositing
> destroys R1 and R5: once GWG Green is RGB there is no unspecified colorant
> left to leave unchanged, so painting 0% over it erases it. **That predicts
> a white X in exactly a, b, c (and g, h, i), and correct output in d, e, f
> where the right answer IS knockout.**

That prediction was derived independently of pdfce's output and **matches
the observed render exactly** (2026-08-18, `ac15158`). **The fix is
colorant-level compositing, not an overprint special case.**

---

## §3 — ★ GWG030 Gray / K black Overprint

**PRIMARY (ReadMe)**, 06 Jan 2006, Peter Claes. Same 12-cell geometry as
GWG040 (a–f OPM 0, g–l OPM 1): 50% K / 50% gray / 50% sep-black, each over
spot and over CMYK.

**MEASURED:** `/CS1 = [/DeviceN [/Black /GWG Green] /DeviceCMYK]` — **the
"spot" backdrop is a two-colorant DeviceN carrying Black**, not a plain
Separation. `/CS2 = [/Separation /Black /DeviceCMYK]`. Backdrop fills
`0.5 1 scn` and `0.5 0 1 0.5 k`. Foreground fills `0 0 0 .5 k`, `.5 g`,
`/CS2 cs .5 scn`. **Eleven ExtGStates with `/OP` and `/op` set
independently** — so this patch also discriminates **stroke-vs-fill
overprint**, which no GWG prose mentions.

### DERIVED per-cell truth table

| Cell | Result |
|---|---|
| **a / g** 50% K over DeviceN(Black .5, Green 1) | **Green + 50% K** |
| **b / h** 50% DeviceGray over spot | R3 → **Green + 50% K** |
| **c / i** 50% Sep/Black over spot | R4 → **Green + 50% K** |
| **d** 50% K over CMYK, OPM 0 | **knocks out — plain 50% K grey** |
| **j** same, OPM 1 | C,M,Y = 0 leave unchanged — **backdrop preserved, OVERPRINTS** |
| **e / k** 50% Gray over CMYK, both OPM | R3 → **knocks out — plain 50% K** |
| **f / l** 50% Sep/Black over CMYK | R4 → **backdrop preserved, OVERPRINTS** |

**a = b = c = g = h = i** — a strong checkable invariant: **the three
encodings of black must agree over a spot backdrop.**
**d vs f at OPM 0** is the sharpest single pair: same tint, same backdrop,
same OPM — DeviceCMYK knocks out, Separation overprints.

**★ Version warning — PRIMARY (GWG v4 whitepaper §2.3.2):** GWG 3.0 and
GWG 12.0 were **silently changed** in Output Suite 4.0 *"to prevent ghosting
effect"*, **filenames unchanged**. **Pin fixtures to a file hash.**

**UNRESOLVED — cell e/k backdrop.** The ReadMe says *"a 50% Gray vector
object is set to overprint **a Gray object**"*; the caption says *"50% gray
over **CMYK**"*. Wording is identical in the standalone ReadMe, the combined
ReadMes, **and the v3.0 manual** — stable ~20 years, so not an extraction
artifact. DERIVED reading: the caption is right, because the grid is
symmetric (d/e/f all "over CMYK") and *"50% Gray over a Gray object"* would
be a **degenerate no-op** that could never show an X. Settleable in ~10
minutes by mapping rectangles to cells in the content stream.

---

## §4 — GWG010 CMYK Overprint

Five object types × two OPM. **PRIMARY**, 07 Nov 2005. Caption columns
`font | vector | image | mask | shading`; a–e OPM 0, f–j OPM 1.

**★ Polarity is INVERTED for four cells — PRIMARY, stated twice:** *"Images
or image masks in CMYK should never overprint CMYK objects. If an X shows,
it means that overprints have been **wrongly applied**."* So **c/d/h/i fail
if you DO honour overprint**; a/b/f/g fail if you don't. Spec basis is R2.
The shading cells e/j likewise.

**Two corrections to the ReadMe — both MEASURED. Use the artwork, not the prose:**

1. **`/ImageMask` occurs ZERO times in the file.** One XObject, `/Im0`,
   `Subtype /Image`, `ColorSpace [/Indexed /DeviceCMYK 0 <lookup>]`
   (hival 0), 95×95, drawn **four** times — both the "image" and "mask"
   columns draw it. **GWG010 does not test image masks**, despite the
   ReadMe and caption saying so.
2. ReadMe cells h and i say *"with op mode 0"* while sitting in the OPM 1
   block. **MEASURED:** those draws use `/GS4 = {OP:false, OPM:1, op:true}`.
   **OPM 1 is correct; the prose is a copy-paste error.**

**Consequence for pdfce:** `/Indexed` over `/DeviceCMYK` means colorants
must be read from the **base** space (§8.6.6.3). Reading them off `/Indexed`
yields none and gets **both GWG010 and GWG031** wrong.

**Image masks + OPM is a genuine ambiguity** — §8.6.7 excludes *"the
painting of images"* without carving out image masks, but a stencil paints
with the current non-stroking colour (§8.9.6.2), satisfying §8.6.7's own
test; PDF/A-4 phrases its restriction as covering *"image masks"*; Enfocus
documents masks as OPM-**sensitive**. **GWG ships no image mask, so it has
no evidence either way.** → **settings-shaped**; default OPM-sensitive to
match Acrobat/Enfocus.

---

## §5 — GWG011 CMYK Overprint Mode

**PRIMARY**, 27 Dec 2006, Jaeggi. Two columns `OPM 0 | OPM 1`.

**Caption:** `Rect (overpr) 0/0/10/50` · `Cross 90/10/90/0` ·
`Cross (overpr) 90/10/90/0` · `Cross 0/0/10/50` · `Rect 90/10/10/50` —
*"If an X appears the Overprint Mode (OPM) is not respected."*

**Cleanest OPM statement GWG publishes:** *"The Overprint Mode specifies if
a CMYK channel with 0% does overprint an other CMYK color underneath
(OPM = 1) or does knock out (OPM = 0)."*

Both rects and crosses carry a zero in the M or C channel — that is what
makes the two modes composite differently. **Expect only ONE X on failure**
(pdfce currently reports exactly 1 trap).

---

## §6 — GWG020 Spot and CMYK Overprint

**PRIMARY**, 27 Nov 2005, **updated 15 Jun 2015**. Spot is "GWG Green".
Top row "cmyk over spot" (a–e), bottom "spot over cmyk" (f–j); columns
`font | vector | image | mask | shading`.

**Key contrast with GWG010:** here images and image masks **are** expected
to overprint, because the spot colorant is genuinely absent from the other
object's space (R1). GWG010's image cells must knock out because OPM cannot
reach images (R2). **Together they are a clean two-sided test of whether a
renderer keys overprint off the COLOUR SPACE rather than the OBJECT TYPE.**
This is the second patch pdfce's spot-flattening fails — in both directions.

**Documented tolerance — PRIMARY:** *"A faint 'X' in slightly darker green
may show in **all** of the tests; this is acceptable behavior in this patch."*

---

## §7 — GWG041 White Overprint Mode

The only overprint patch where GWG publishes **both** the correct appearance
**and** a per-failure-mode diagnostic. **PRIMARY**, 08 Apr 2008, Kleinheider.
Two cells. **MEASURED:** only two ExtGStates, both **OPM 1** — this patch
does not vary OPM.

- **a)** white vector in `/Separation /GWG Green`, overprint on, over CMYK.
- **b)** CMYK *"almost white (0.2% in each process color channel except black)"*, overprint on.

**Expected, stated positively:** *"If a PDF/X conforming workflow performs
the rendering, the patch to show up as **a green and a gray rectangle**."*
**The two cells have OPPOSITE correct behaviour: a must vanish, b must knock out.**

**PRIMARY failure table, verbatim:**
- white X in **a** = *"Overprint was deactivated or not honored"*
- white X in **a** (alt) = *"The spot color object was converted to CMYK, Overprint stayed on, but **OPM was not set to 1**"*
- **red** X in **b** = *"Due to **rounding errors the 0.2% colorant are treated as 0%** leading to an overprinting white element"*
- white X in **b** = *"The overprinting got deactivated or not honored"*

**UNRESOLVED — the epsilon.** Caption says `.2/.2/.2/0` and the cell says
*"0.2%"*; the Notes paragraph says *"'nearly white' (o.o2% in this
instance)"*. The whole cell tests a renderer's **rounding threshold**, so an
implementer needs the real number. For calibration, Kodak Prinergy ships
*"White is considered white when black (K) is less than 0.9%"*.

**pdfce currently passes GWG041.**

---

## §8 — GWG120 White Overprint / Knockout

**The bidirectional patch: half the cells are authored KNOCKOUT and must
stay knockout.** **PRIMARY**, 27 Dec 2006 rev 29 Aug 2013, Jaeggi.

Rows `Overprint` / `Knockout`; sub-rows `CMYK` / `Spot`; columns
`Vector | Text big (59 pt) | Text 6pt (8 pt)`.

**PRIMARY:** *"A lot of workflows and RIPs try to 'fix' white objects by
setting white always to knockout… When a workflow or RIP **changes** the
overprint behaviour of an element an X appears."*

GWG030/040 ask *"did you honour overprint?"*; **GWG120 asks "did you honour
the AUTHORED setting, whichever it was?"** A "white always knocks out" rule
passes the bottom half and fails the top; "white always overprints" does the
reverse. **MEASURED:** three ExtGStates, clean overprint/knockout binary at
fixed OPM 1.

**pdfce currently passes GWG120.**

---

## §9 — GWG190 / 191 / 192 DeviceN Overprint (Black / Yellow / White)

Best-documented overprint patches in the suite. **PRIMARY**, 12 Dec 2012,
Kleinheider. All three share one layout: four cells, a/c vector and b/d
image, *"OP is true for all topmost elements."*

| Patch | a + b must render | c + d must render |
|---|---|---|
| **GWG190** Black | solid **Black (100C100K)** | solid **Cyan** |
| **GWG191** Yellow | solid **Green** | solid **Cyan** |
| **GWG192** White | solid **Red** | solid **White** |

**The discriminator, identical in all three:** the **a/b** pair uses a
DeviceN whose colorant list **omits** the backdrop's colorants, so overprint
leaves them standing; the **c/d** pair uses a DeviceN that **includes** those
colorants at 0%, so they are written as zero and knock out.
`100C` vs `100C0Y0K`; `100C` vs `100C0Y`; `0K` vs `0C0M0Y0K`.
**The colorant LIST — not the tint values — decides what survives (R4).**
A renderer that flattens DeviceN to DeviceCMYK before compositing collapses
the distinction and **fails cell c specifically.**

**Shared per-cell diagnostics — PRIMARY, identical across all three:**
*"A cross in patch **c** indicates a colour conversion to DeviceCMYK prior
to rendering. The cross appears since OPM is set to 1. **However, if the
system performs colour conversion and sets the OPM for this patch c to 0,
the rendering is also fine.**"* — **cell c has TWO sanctioned correct
outcomes; do not treat it as binary.** *"A **faint** cross in patch **d**
indicates a colour conversion using inadequate ICC profiles or method"* —
faint = tolerated.

**MEASURED (GWG192):** spaces are
`[/DeviceN [/Cyan /Magenta /Yellow /Black /None] /DeviceCMYK …]` and
`[/DeviceN [/Black /None] /DeviceCMYK …]`. **The `/None` colorant is present
in both** — must mark nothing (§8.6.6.4) and **must not join the overprint
colorant set.** *(This is exactly what Ghostscript bug 709099 gets wrong —
see `docs/overprint-architecture-survey.md` §5.)*

**Acrobat-verified ground truth for GWG192 cell d** — Poppler issue #1410
(⚠️ GPL project; tracker **prose only**), closed FIXED: *"The expected
result is to have a completely white square for d. I checked with Adobe
Acrobat and it renders a white square indeed."*

---

## §10 — Harness caveats

- The test pages carry an embedded **Preflight Audit Trail** and a preflight
  signature that **invalidates on any modification** — a useful tamper check,
  but it trips if tooling rewrites the file.
- A strict pixel-diff produces **false failures** on exactly the cells GWG
  pre-declares tolerant (§0).
