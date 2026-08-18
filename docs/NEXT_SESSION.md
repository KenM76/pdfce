# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. Overwrite it once acted on.

**Written 2026-08-18**, replacing the 2026-08-05 handoff (whose queue —
"do clause-11 transparency next" — has now been worked; see §3).

---

## ★★ THE HEADLINE: THE OVERPRINT WALL HAS A DEFINITIVE, SOURCED ANSWER — BUILD THE N-CHANNEL BUFFER

Ghent went **22 → 25 of 51** this session by implementing
`CompatibleOverprint` (Table 149) as a real per-pixel CMYK blend. The
remaining overprint patches do **not** yield to any further refinement of
that approach, and this is now established three independent ways:

1. **By measurement.** A spot-ink multiplier plate was built and ablated
   (same binary, one line changed): traps 17 → 16, **patches passing
   unchanged**, and one patch *regressed* 3 → 6. Reverted (`ac15158`).
2. **By research.** Seven engines — Ghostscript, MuPDF, Poppler, Harlequin,
   Mako, PDF Tools AG, Adobe — converge on one architecture. Artifex's
   colour architect states in a peer-reviewed paper that collapsing colour
   before compositing *"is not possible"* **specifically because of
   overprint**. Poppler documents pdfce's exact bug in its own words.
3. **By pdfce's own spec RAG**, which called this "stage 10, large —
   architectural, a different project" back on 2026-08-08.

**⇢ READ `docs/overprint-architecture-survey.md` FIRST.** It is the sourcing
record: the architecture, why the cheap route provably cannot work, the
unstandardised final-collapse step (a settings-shaped ambiguity), three
places pdfce can **exceed** Ghostscript/Poppler, and the licensing rules for
follow-up (all those projects are GPL/AGPL — behavioural reference only).

### What the build actually is

- One plane per colorant: CMYK + one per spot. **Pre-scan the page's
  resources to size it exactly** — Ghostscript's docs note this is possible
  in PDF and impossible in general in PostScript. That is pdfce's advantage.
- **Keep the tint transform OUT of the paint path** for any colorant that
  owns a plane. It is retained only to derive that colorant's *equivalent*
  colour for the final collapse.
- Apply Table 149 in colorant space (`pdfce_render::overprint` already has
  it as pure, tested logic — 12 tests, the table transcribed cell by cell).
- Collapse to RGB **once, at the end**. Pick a collapse model deliberately
  and disclose it (§6 of the survey; vendors disagree materially and Acrobat
  does not document its method).
- **Cap and fall back honestly:** beyond N planes, revert to the tint
  transform — which is precisely pdfce's current behaviour, so the fallback
  is already written and already disclosed.

**Scope it, do not make it page-wide by default.** Poppler #1565 (open):
enabling overprint preview routed the whole page through CMYK and visibly
shifted unrelated RGB raster content.

---

## §1 — Current Ghent standing (measured 2026-08-18, `2a75be1`)

```
25 pass · 18 FAIL · 8 UNRESOLVED (reference-strip) · 0 render errors  of 51
```

Command:
```bash
python tools/ghent-check.py /d/Dev/temp/ghent-patches \
       --reference-dir /d/Dev/temp/acro-refs
```
Corpus at `D:\Dev\temp\ghent-patches\` (51 PDFs); Acrobat reference strips I
generated at `D:\Dev\temp\acro-refs\` (51 PNGs). **Both are outside the repo
— test-corpus rules, `LEGAL.md` §5.**

The 18 failures cluster:

| Cluster | n | Patches |
|---|---|---|
| **Overprint** | 7 | `GWG011`, `GWG190`, `GWG191`, `GWG192`, `GWG020`, `GWG030`, `GWG040` |
| **Transparency groups** | 5 | `1_GWG160`, `1_GWG161`, `1_GWG162`, `3_GWG161`, `3_GWG164` |
| **Soft masks** | 4 | `GWG1610`, `GWG1611`, `GWG168`, `GWG169` |
| **Shading** | 1 | `GWG060` (mesh) |
| **ICC** | 1 | `3_GWG130` |

*(7+5+4+1+1 = 18 ✓. The librarian corrected an earlier "6 overprint" of
mine by exactly this arithmetic.)*

**⇢ `docs/ghent-patch-reference.md`** now holds the per-patch expected
appearance, extracted from GWG's ReadMes — which are **not on the web**,
they ship inside a 126 MB download. It includes DERIVED per-cell truth
tables for GWG030 and GWG040, and two corrections to GWG's own prose.

---

## §2 — Three cheap wins available before the big build

1. **The trap detector is probably over-counting.** GWG's stated criterion
   is **"Faint X does not indicate a failure!"**, judged by a human at
   0.5 m. `tools/ghent-check.py`'s `CONTRAST_MIN` was calibrated against
   pdfce's own output, **not** against that criterion. GWG pre-declares
   tolerant: **all ten cells of GWG020** and **cell d of every DeviceN
   patch**. Recalibrating may move the score without touching the renderer —
   and would make every future measurement more honest.
2. **The suite ships a Reference file** —
   `Ghent_PDF-Output-Test-V50_ALL_REFERENCE.pdf`, in the same ZIP, texts in
   Registration so they show in every separation. **pdfce is not using it as
   an oracle and should.**
3. **`/Indexed` over `/DeviceCMYK`:** colorants must be read from the **base**
   space (§8.6.6.3). Reading them off `/Indexed` yields none and gets
   **GWG010 and GWG031** wrong. Worth a grep — pdfce may already have this
   bug latent, since GWG010 passes for possibly the wrong reason.

---

## §3 — What shipped this session (9 commits, `ea5159e`..`2a75be1`)

| Commit | What |
|---|---|
| `ea5159e` | **Linux build break.** `cmd_print` called four `#[cfg(windows)]` callees while ungated. Windows green, CI red. Every sibling already had the pattern. |
| `cb55b6b` | **Real defect: OCR wrote into certified documents.** `add_ocr_layer` had no §12.8.4 certification check; its twin `add_text` always had one. Found by checking whether a gate's *stated warrant* was true. Closes `Pass 86.0`. |
| `a3080f0` | Cross-platform dead-code fallout. `cfg_attr(not(windows), allow(dead_code))` **not** `cfg(windows)` — gating breaks the tests, which are pure byte arithmetic and are the only coverage on the platform CI actually runs. |
| `bd9d5ef` | **`pdfce_render::overprint`** — Table 149 as pure logic, 12 tests, the table transcribed. Records `OP-N1`/`SP-N2`/`OP-A3`. |
| `bf75351` | **Overprint wired: 22 → 25.** Also: the glyph painter wasn't merely skipping overprint, it wasn't **counting** it — a disclosure counter blind to a whole object class reports a smaller problem than exists. Text took it 23 → 25. |
| `ac15158` | **The spot-plate negative result** (see headline). |
| `18a0f15` | Librarian filing — decision **069**, Ghent standing board, FEATURES rows. |
| `cb20770` | **Soft masks** (§11.6.5) implemented. |
| `2a75be1` | `.gitignore` for stray subagent fetch artefacts (see §6). |

---

## §4 — Soft masks: what's done, and the ONE thing left

Implemented `/Alpha` and `/Luminosity` (`cb20770`). Correlations improved on
every measurable patch:

| Patch | before → after | reference engine |
|---|---|---|
| `1_GWG1610` Text part1 | 0.515 → **0.575** | 0.966 |
| `1_GWG168` Vector part1 | 0.661 → **0.725** | 0.981 |
| `1_GWG169` Vector part2 | 0.884 → **0.905** | 0.983 |

**None passes yet, and the headline did not move** — reporting the
correlations because that is what changed.

**The residue is diagnosed, not guessed.** I dumped the mask groups and the
folded clips to PNG: **both are correct, properly placed soft gradients**.
So construction is right. The gap is **application**: §11.4.5 applies the
mask to a transparency group's **RESULT**, whereas pdfce folds it into the
clip, which applies it to each element **inside**. **⇢ That is the same
buffer work as isolated/knockout groups — do them together (§5), not
separately.**

Sourced contract now honoured (all in the code's doc comments): `/BC`
default = the colour space's initial value = **black** (all-zeros is black in
RGB but **pure white in CMYK** — this file's own masks carry
`[1.0 1.0 1.0 1.0]`, the trap in the wild); outside-BBox = `TR(lum(BC))`,
neither 0 nor 1; opaque backdrop `α₀ = 1`; luminosity `0.30/0.59/0.11` with
**no** gamma compensation and deliberately **not** Rec.709; matrix baked at
`gs` time, not paint time.

**Owed:** `/TR` is read and **counted** (`soft_mask_tr_ignored`) but not
evaluated — it needs the function machinery in the render crate. `/TR` is
where a mask gets **inverted**, so an ignored one can leave visible exactly
what a document meant to hide.

---

## §5 — Transparency groups: the research is in hand

Five patches. A second research pass returned a full formulation — **read it
in this session's transcript or re-commission it**; the substance not yet
written to a pdfce doc is:

- **Backdrop removal is in the spec**, §11.4.4 result block:
  `C = C_n + (C_n − C_0)·(α_0/α_gn − α_0)`, with NOTE 3's equivalent
  "intuitive" form. Danger is the single `1/α_gn`.
- **`0/0 = 0` by convention** — a `should` in ISO 32000-1 and a **`shall` in
  ISO 32000-2** §11.3.2. Adopt unconditionally. The `shall` is on
  *robustness*, not on the value: never emit NaN/Inf.
- **Knockout needs TWO buffers, not per-element copies.** §11.4.8 makes it a
  subscript `b ∈ {0, i−1}`. Memory is O(nesting depth).
- **The extra per-pixel state is smaller than feared:** un-premultiplied
  colour + one extra scalar (`α_g`). `α_i = Union(α_0, α_gi)` is derivable
  and `α_0` is the parent buffer. **f32, not u8** — the correction factor
  amplifies error by ~`1/α_gn`.
- **★ These patches may not be a group bug at all.** §11.3.4 requires
  blending **in the group's colour space**, with subtractive components
  **complemented before and after**. pdfce blends in RGB. A 14-trap count on
  `1_GWG161` is far more consistent with "every blend mode computed in the
  wrong space" than with "knockout mis-implemented". **Check that first — it
  is cheap and it may explain four patches at once.**
- **Verified, so don't chase it:** ISO 32000-1's `ColorDodge`/`ColorBurn`
  formulas are wrong (Adobe never implemented them; PDF 2.0 corrected them)
  — **but tiny_skia already implements the corrected branches.** I tested
  both edge cases empirically. Not a pdfce bug.

---

## §6 — Housekeeping, owed items, and one live hazard

**v0.7.0 is bumped but NOT tagged or released.** `git describe` =
`v0.6.0-36-g2a75be1` — **nothing released in 36 commits.** CI was red on
`check-commits-filed.py` for most of the session; `18a0f15` is green.
**Verify CI green on `HEAD`, then: `verify-release.py` → tag → portable
package → GitHub release → librarian release record.** Operator gave a
standing go-ahead for builds/releases on 2026-08-17.

**★ LIVE HAZARD — subagent fetch artefacts land in the repo root.** Two
batches in one session: `out.html` (caught by the librarian), and
`vp.txt`/`vp4.txt`/`pdfa4.txt` which **were committed** by a `git add -A`.
The repo is **public**. `.gitignore` now covers the shapes, **but the ignore
rules are the backstop, not the cure — stage narrowly.** One of the
committed files was literally a login page.

**Owed, small:**
- **★ `LUM-A1` was resolved BY CONSTRUCTION and needs a decision.** Its
  register entry warned it might be *"silently hard-coded one way, when
  soft-mask implementation starts"* — which is exactly what happened.
  `grep -rn "LUM-A1" crates/` returns nothing. The shipped path is a
  **third** reading, neither analytic form the register enumerated: the mask
  group renders through the ordinary paint path, so `DeviceCMYK` becomes
  sRGB *before* luminosity. The behaviour is defensible; the point is that
  `LUM-A1`'s stated `K·S ≤ 0.25` bound **does not bound the shipped path**.
  Decide it deliberately or record the third reading as the answer.
- A **`/P 2` fixture carrying page content.** The new OCR certification test
  uses `certified-locked.pdf`, which is `/P 1`, so it cannot prove the
  refusal is tier-independent — which is what the guard claims. The `/P 2`
  fixture exists but is in the *forms* set. (Librarian's catch.)
- **`LUM-A1` is still open** and is **not** the `/BC` question — that one is
  settled. `LUM-A1` is the `DeviceCMYK`→luminosity formula split (§11.5.3
  NOTE 3 vs §10.3.3, diverging by `K·S`, max 0.25). Spec-librarian
  recommends the 2.0/§10.3.3 form. Needs a decision.
- **`MSH-A1`** — mesh type 6/7 patch-record byte-padding granularity is
  unstated in **both** editions. Settings-shaped. Recommended default:
  per-patch-record padding.
- **GWG030 cell e/k** backdrop and **GWG041's epsilon** (0.2% vs 0.02%) are
  both unresolved in GWG's own documentation; each is ~10 minutes of
  content-stream reading. See `docs/ghent-patch-reference.md`.

- **`Pass 85.1` (mesh shadings) and the `85.4c` remainder (§11.4.6) were
  both recorded as BLOCKED on spec-corpus gaps that are now FILLED** by this
  session's spec-librarian pass. Nobody has re-scoped them. Same shape as
  the XFA bullet in `CLAUDE.md`: the answer was sourced in one document while
  another still said it was blocked.
- Cosmetic but confusing: the stdout key is **`soft_mask_tr_ignored`** while
  the field is **`soft_mask_transfer_ignored`**, so grepping the printed key
  finds no declaration.

**Mesh shadings (`GWG060`) are now fully ingested** — the spec-librarian
wrote `iso32000__s__8.7.4.5__mesh.md` (1,014 lines, Tables 82–**86**; the old
"Tables 82–84" was short by two, and 85/86 are exactly the edge-flag
inheritance rules). The "do not answer from recall" marker is retired. Two
traps recorded there: implement **type 7 only** and lift type 6 via the four
closed-form `1/9(…)` interior-point equations (writing two evaluators gives
two rendering paths for one content type); and `pdftotext -layout`
**column-jumbles all five mesh tables** in the staged 1.7 source.

---

## §7 — Standing discipline reminders that cost me this session

- **`cargo fmt`/`clippy`/`test` is NOT the gate set.** Run every
  `tools/check-*.py` and `tools/check-*.sh` **after committing** — the
  commits-filed gate's input *is* the commit list, so a pre-commit sweep
  reports clean vacuously. That cost two red CI runs.
- **Never patch a Rust `\`-line-continuation through a heredoc.** It becomes
  a literal `\n`. Happened **four times** this session, once while
  "repairing" the same bug. Build, clippy and fmt are all silent; only the
  stable-stdout-line test catches it. Use the Edit tool. Grep sweeps are in
  the agent memory note.
- **Verify a refusal's stated warrant before extending it.** A gate asked a
  bookkeeping question, the exception list's warrant said such a function
  "honours the certification gate", checking that claim found a real
  correctness defect (`cb55b6b`).
- **`cargo check --target x86_64-unknown-linux-gnu`** before pushing
  anything touching `#[cfg]`. Windows green ≠ CI green.
