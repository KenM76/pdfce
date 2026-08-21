# Ghent suite — operator review, 2026-08-21, and what it says about the harness

**The operator read the annotated render cell by cell and found defects
`tools/ghent-check.py` structurally cannot see.** This file records his
observations verbatim-in-substance, my verification of each, and the two
instrument faults they expose. It exists because these are the only
independent judgements of pdfce's Ghent output that have ever been taken —
every other number in this project came from the harness scoring itself.

★ **Treat this as an ORACLE, not as feedback.** The harness had no
independent check before today; its thresholds were calibrated against one
patch (`GWG 16.0`, 2026-08-17) whose answer was known from a code change.
That is self-consistency, not ground truth. These rows are ground truth.

---

## 1. What he found, and what I confirmed

| # | GWG | harness said | operator says | I verified | verdict |
|---|---|---|---|---|---|
| 3 | 1.0 CMYK Overprint Test | **pass** | cells `d`,`e`,`i`,`j` are **clear fails**; `b`,`g` are faint outlines only | yes — the mask and shading cells carry large, obvious crosses | **FAIL** |
| 4 | 1.1 CMYK Overprint Mode | **FAIL, 1 cross** | **pass** except a faint outline | the combined render shows no cross at any contrast | **PASS** |
| 5 | 19.0 DeviceN Overprint (Black) | FAIL, 1 cross | cell `d` fails; `b` is a faint outline, so the operation is right and only the edge rounding differs | yes | FAIL (1 cell, `d`) |
| 6 | 19.1 DeviceN Overprint (Yellow) | FAIL, 4 crosses | `a`,`c` pass; `b`,`d` fail | yes — 4 was an over-count | FAIL (2 cells) |
| 8 | 8.2 DeviceN Support (4 colours) | **pass** | **fail — the two green check marks are missing** | yes, at 3× zoom: no check mark in either image's upper-right corner | **FAIL** |
| 20 | 5.0 Font Substitution | pass | — | check mark present and correct | PASS |
| 30 | 8.1 DeviceN Support (5 colours) | **pass** | — | **no green check marks** | **FAIL** |
| 31 | 8.01 DeviceN Support (6 colours) | **pass** | — | **no green check marks** | **FAIL** |

**Net effect on the headline: 29 pass becomes 26.** Four patches move
pass → fail (3, 8, 30, 31), one moves fail → pass (4). And three more
(`15.0`, `15.1`, `15.2`, optional content) use the same criterion as 8, 30
and 31 and are **not yet checked** — they are not on the combined pages, so
the operator could not see them either.

## 2. ★★ INSTRUMENT FAULT 1 — the harness implements ONE of the suite's TWO pass criteria

The suite marks a failure two ways, and its own artwork says so:

- **negative marker** — a cross that a correct renderer makes vanish.
  `ghent-check.py` implements this, thoroughly.
- **positive marker** — *"If a check mark is visible in the upper right
  corner then DeviceN is respected (= GOOD). If no check mark appears then
  DeviceN color was transformed to CMYK (= ERROR)."*
  **The harness cannot see this at all.** It hunts for a mark that should
  not be there; it has no notion of a mark that should be there and is not.

**Seven of the 51 patches use the positive criterion** — grep the ReadMes
for "check mark": `GWG050`, `GWG080`, `GWG081`, `GWG082`, `GWG150`,
`GWG151`, `GWG152`. Every one of them has been reported `clean` for the
harness's entire life, and at least three of them are failures.

⇢ **The failure mode is exactly the one this project keeps re-learning: an
absence is invisible to a detector built to find a presence.** A gate that
looks for the wrong thing does not report "I cannot tell" — it reports
"clean", which is indistinguishable from a pass.

## 3. ★ INSTRUMENT FAULT 2 — the contrast floor has no area term

`CONTRAST_MIN = 12.0` (8-bit levels) implements the suite's *"a **clear** X
… judged by a human at 0.5 m"*. It is a fixed number regardless of how big
the mark is.

Box 3's cells `d` and `i` are crosses roughly **three times the linear size**
of the calibration patch's, at a measured contrast of **9.8** — below the
floor, and unmistakable to the eye at normal viewing distance. Meanwhile
box 11's cells sit at 1.3–3.1 and genuinely are invisible.

⇢ Perceptibility scales with area as well as contrast. A floor with no area
term is calibrated for one mark size and wrong for every other. The fix is
not to lower the number — that would drag box 11's population in with it —
but to make the threshold a function of the mark's size.

## 4. What this does NOT change

**Nothing about the compositing work.** Patches 9, 10, 11 (`16.0`, `16.1`,
`16.2`), 36 and 37 are cross-criterion patches and the operator confirmed
the first three read correctly. The blending-colour-space census
(107 wrong → 0) counts blend operations, not traps, and is unaffected by
either fault above.

**What it changes is the scoreboard**, and specifically any sentence of the
form *"N of 51 Ghent patches pass"*. Every such sentence in `ROADMAP.md`,
`FEATURES.md` and `SESSION_LOG.md` — including numbers filed earlier today —
is an over-count by the size of the check-mark family.

## 5. Owed work

1. **Teach `ghent-check.py` the positive criterion.** Per-patch, from the
   ReadMe: which marker, where, what colour. Report `MISSING-MARK` as its
   own verdict rather than folding it into `X`.
2. **Give the contrast floor an area term**, calibrated against the rows in
   §1 — which is the first independent calibration set this harness has
   ever had.
3. **Re-measure and re-file.** The corrected suite standing is **26 pass**
   at minimum, pending the three unchecked optional-content patches.
4. **Check the three optional-content patches** (`15.0`, `15.1`, `15.2`),
   which no one has looked at.

## 6. The reason this document exists rather than a commit message

Because the operator's cell-level judgements are the calibration set for
items 1 and 2, and a calibration set that lives only in a commit message
cannot be re-read by the person doing the calibration. He also gave a
*mechanism* twice — "just an issue with the layer edge", "the math for the
edges of the x differs slightly with rounding" — which is a distinction the
harness has no way to express and which item 2 will need: a mark that is
present in outline only is a different fact from a mark that is present in
fill.
