# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. Overwrite it once acted on.

**Written 2026-08-21 (afternoon)**, replacing the 2026-08-21 (morning)
handoff, which is superseded in three of its own clauses (§7 below says
which, and why a *provenance* label is not a *freshness* label).

---

## §0 — DO THESE THREE THINGS BEFORE ANYTHING ELSE

**1. `ls` BOTH FeatureRequests channels.** They are outside this
repository, so **no gate will ever contradict a stale sentence about
them — including this one.**

```
D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\
D:\Dev\FeatureRequests\iccce_FeatureRequests\open\
```

Last session the `ls` found **two `iccce` notes that had landed after the
previous handoff was written**, both compositor input, both used. That is
two sessions running where the `ls` found something a document said was not
there.

**2. Run the gates — `ls tools/check-*`, do not trust any list.** There are
**16** on disk, **15 wired into CI** (`check-image-colorspace-truth.py`
takes a fixture directory and is a sweep tool, named in `ci.yml` so the
next person counting does not record it as a miss).

**★ And `tools/check-ci-parity.py` is new, and it exists because the local
sweep is not the CI set.** `for g in tools/check-*` is **one of CI's nine
jobs**. `cargo fuzz build`, the wasm32 cross-check, the no-network denylist
and the licence audit have no local runner anybody runs by habit — which is
how the fuzz job stayed red for **three days** while every local check was
green. `python tools/check-ci-parity.py --list` prints the eleven local
stand-ins. **`R209` is the rule this minted:** *"all gates green" names a
set, and the set somebody runs is not the set CI runs; a CI job with no
local runner is UNOBSERVED, not passing.*

**3. Read `docs/compositor-plan.md`'s 2026-08-21 amendments** before
scoping anything in the `97.x` family.

---

## §0.5 — `v0.7.0` IS RELEASED

Tag **`3bc8fbe`**, pushed, published, asset
`pdfce-v0.7.0-portable-win64.zip` (10,906,752 B, SHA-256
`25BDBB11…1042AFE`). `tools/verify-release.py v0.7.0` passes **all seven
checks**. CI run `32486133822` was **green on all ten runs, observed BEFORE
the tag was placed** — `R209`'s first live application — ending **nine
consecutive red runs** over three days.

Packaging smoke test passed on the tagged build, in a fresh folder: the CLI
reports `revision: v0.7.0` exactly, and the GUI creates `userdata/` inside
its own directory (decision 003 / `R15`).

⚠ **`v0.7.0`'s tag is in no bundle on disk.** Unlike `v0.6.0`, this release
is **not recoverable from backup**. The newest bundle is
`pdfce-20260817-v060.bundle`, head `3c4c00e`, **169 commits** behind — and
that head **is** `v0.6.0^{}`, so "commits since backup" and "commits in
this release" are the same 169 and a future filing that reports them
differently refutes itself. **Cutting a bundle is the operator's call.**

---

## §1 — WHERE THE COMPOSITOR IS, AND WHAT TO DO NEXT

`Pass 97.0` (a–d) and `Pass 97.1` (a–d) shipped. What exists:

⚠ **A Pass-ID collision to know about before citing anything.** `97.1a` and
`97.1b` were minted four filings earlier for the `/Indexed` colorant work
and `overprint_images_unsupported`. The compositor work in this session's
commit subjects therefore carries **the wrong letters** — where a commit
says `Pass 97.1a`/`97.1b`, `ROADMAP.md` says **`97.1c`** (the subtractive
arithmetic) and **`97.1d`** (reading the blending space). `ROADMAP.md` is
authoritative and carries a disambiguation table. The mechanism: the
minting filing put the IDs in `ROADMAP.md` only, and never into this file
— which is the one the engineer reads before committing.

| | |
|---|---|
| `compositor.rs` | §11.4.4's element formula, §11.4.8's knockout variant, backdrop removal, `Union`, **all thirteen Table 136 separable functions**, and — new — §11.3.4's **subtractive complement** and §11.3.5.3's CMYK detour with K **selected** by mode |
| `Canvas::group` | non-isolated groups render over their own backdrop; second walk skipped under §11.4.4 NOTE 5's own condition |
| `KnockoutTarget` | real §11.4.6, four planes, §11.4.6 NOTE 6's nesting rule |
| soft masks | applied to the group **result** (§11.4.5) |
| **blending colour space** | **read and disclosed** — page group and per-group, honouring Table 147's inheritance rule |

### ★★ THE NEXT BUILD, and the measurement that scopes it

`Pass 97.1e` — **a CMYK group buffer**. The arithmetic is done and tested;
what is missing is a buffer that holds ink rather than screen colour, so
the operands reach it un-round-tripped.

**Why a round trip will not do, measured:** `DeviceCMYK 0 1 0 0` painted
and recovered from the sRGB buffer comes back as
`(0, 0.995, 0.409, 0.071)`. That `Y = 0.41` is not a rounding error, and
§8.6.5.7 NOTE 2 names the 4→3→4 trip by hand as *"unnecessary and results
in a loss of fidelity in the black component"*.

**Why four channels and not N:** the leading deliverable is the *blending
space*, which is `DeviceCMYK` for every file that matters here. Spot planes
are the *second* deliverable and want a runtime-N **plane-major** buffer
(measured **3–5×** faster than interleaved, `compositor-plan.md` §3.2).
Building N now fuses two questions that fail independently.

**The shape that already worked once:** `paint_nonseparable` and
`paint_overprint` both rasterise a paint to a coverage `Mask` with the same
rasteriser a normal paint uses, then composite per pixel inside the path's
device bounds. That machinery exists and is proven; a `Canvas::Cmyk`
variant is the same move at group scope. `BrushSpec` will need to carry the
authored colorants alongside the sRGB it already bakes — the interpreter
*has* them (`ColorState::device_color`) and throws them away.

### ★★★ AND THE NUMBER THAT SHOULD SET YOUR EXPECTATIONS

| corpus | files with a subtractive space | blend modes applied | **in the wrong space** |
|---|---:|---:|---:|
| Ghent PDF Output Suite (51) | 13 (25.5 %) | 107 | **107 (100.0 %)** |
| `fixtures/external` (4,012 files, 3,735 rendered) | 15 (0.4 %) | 49 | **2 (4.1 %)** |

**One hundred percent on the suite built to test this; four percent on the
corpus of files people actually have.**

- The Ghent transparency panels **cannot** be passed without this build.
- It is a **prepress-shaped** problem, not a general one — and the claim
  is stronger than the 4.1 % suggests. **Both** real-world hits are
  **veraPDF transparency CONFORMANCE fixtures** (PDF/A-4 §6.2.9,
  PDF/A-2b §6.2.10), so **zero organic documents in a 4,012-file corpus
  are affected.** The buffer buys conformance and print-site credibility —
  the stated goal — and will change **nothing** about how ordinary
  documents look. Do not expect the render-parity buckets to move, and do
  not read that as failure.

---

## §2 — THE QUEUE, in the order I would take it

1. **`Pass 97.1e`** — the CMYK group buffer, above. (Next free letter;
   `a`–`d` are spent — see the collision note in §1.)
2. **`Pass 119.1`** — `unshare_form` (copy-on-write a shared form onto one
   page). Carried unstarted through three handoffs now; still a *separate
   verb*, not a mode of `edit_text` (`decision 076`).
3. **`Pass 80.0`** (note text on markup) and **`Pass 81.1`** (markup
   opacity, write half) — both `pdfceGUI` requests, both scoped, both
   untouched.
4. **`Pass 119.3`** — align `pdfce-render`'s nested-form resource fallback
   with `text_edit::forms`.
5. **The reference-strip threshold for `ghent-check.py`** — §5.

---

## §3 — COUNTERS WHOSE MEANING YOU MUST NOT MISREAD

`render-page`'s stable stdout line gained **five** keys this session, all
appended last:

- **`groups_backdrop_reruns`** — a **cost** counter, not a shortfall. The
  only place one page's content stream is interpreted more than once. Zero
  is normal.
- **`soft_masks_on_group_result`** — masks that reached §11.4.5's place.
  Folding into the clip is **still correct** for an elementary object
  (§11.6.4.1's `q_m`); the two are one implementation of two clauses.
- **`overprint_images_unsupported`** — deliberately **not**
  `overprint_refused`. `refused` = "offered and could not run"; this =
  "never offered this object class at all".
- **`blend_space_subtractive`** — a **census**. A page can be entirely
  `DeviceCMYK` and entirely correct.
- **`blends_in_wrong_space`** — the **shortfall**. This is the one.

And one **changed meaning**: `transparency_groups_knockout_approximated`
used to be `1` per `/K true` group; it now counts the *elements* inside one
that read the destination back. **A group pdfce renders exactly reports
zero.**

---

## §4 — WHAT IS STILL NOT DONE, named so it does not read as done

- **Implicit knockout.** Only explicit `/K true` is honoured. §9.3.8's
  `/TK` **defaults true** (every text object), §11.7.4.4 makes `B`/`b`
  knockout, §11.6.7 makes shading patterns knockout. Ranked by likely
  frequency that is `B`/`b` ≫ `/TK` > explicit `/K` — **the one pdfce
  implements is the rarest.**
- **`/TR` on a soft mask** is read, counted, never evaluated. `/TR` is
  where a mask gets inverted, so an ignored one can leave visible exactly
  what the document meant to hide.
- **`f_g` is approximated by `α_g`** for a group used as an element of a
  knockout group — exact when that group's own elements are opaque, which
  is §11.4 corpus §7.4's stated safe-skip condition.
- **`/AIS true` is not distinguished** from `/AIS false` for a group mask.
- **`overprint::composite` still treats a transparent pixel as white
  paper** — the convention `Pass 97.0a` removed from the two blend
  composites. Deliberate: it is Table 149 *decision* logic, and it belongs
  with the colorant buffer that will replace its input.

---

## §5 — AN INSTRUMENT PROBLEM, NAMED AND DELIBERATELY NOT SOLVED

`ghent-check.py` has **no calibrated threshold** for reference-strip
patches, so three soft-mask patches sit at 0.96–0.99 correlation and still
read UNRESOLVED. There is now a bimodal split to calibrate against:
**0.962 / 0.978 / 0.986** against **0.039 / 0.053 / 0.062 / 0.406**.

**Left alone on purpose.** Calibrating an instrument immediately after
making it report what you wanted is not a measurement. Do it in its own
session, against a patch whose verdict is known independently — which is
how the *trap* threshold was calibrated (GWG 16.0, 2026-08-17).

---

## §6 — THREE STALE-CLAIM FAILURES IN ONE DAY, AND THEY ARE THREE STAGES

Worth carrying, because a count of three says *"be more careful"* and three
named stages are each checkable after the fact:

1. **No sweep.** `Pass 97.0` shipped and four comments plus one
   operator-facing message still said knockout was unimplemented.
2. **Wrong key.** A claim phrased as an **absence** — *"…and are **not**
   counted as `overprint_refused`"* — carries no token tying it to the new
   name, so a grep for the new counter cannot find it and a grep for the
   old one finds a sentence that reads deliberate.
3. **Wrong boundary.** The `/Indexed` claim was corrected in two files and
   left standing in a third: **the sweep's boundary was the file, the
   claim's was the feature.** All three copies were born in **one** commit
   across two files, and `git log -S "<the claim's distinctive fragment>"`
   would have named them all.

⇒ *sweep at all → derive the key from meaning → derive the boundary from
the introducing commit.* The librarian recommends folding this into hard
rule 11's method paragraph rather than minting; **hard rule 11 lives in the
engineer's agent file, so that is the engineer's act and it is still owed.**

---

## §7 — WHAT THE PREVIOUS HANDOFF GOT WRONG, kept rather than deleted

Its §7 said backups were *"4 days and ~50 commits stale"*, that
*"`origin/main` is behind"*, and that the tag *"tops at `v0.6.0`"*. All
three are now false, and the first was **wrong by 3.4× when written** — it
reused the commits-ahead-of-remote number against a different denominator.

★ **The sentence carrying that error was the one labelled
*"librarian-measured, not inferred"*.** A **provenance** label is not a
**freshness** label, and a reader takes it for one. That is the transferable
half, and it cost three filings to notice.

---

## §8 — BROTLI: the operator's condition has NOT fired

`EXTN-BROTLI-1 v1.3`, *Brotli compression in PDF 2.0*, PDF Association,
announced **2026-08-19** — CC-BY-4.0, filter name **`/BrotliDecode`**. It is
an **extension** under the `PDFa` developer prefix, **not** on a dated ISO
path: `brotli` returns **zero hits** in ISO 32000-2:2020's 1,023 pages and
in 32000-1:2008, §7.4 stops at 7.4.10, and Table 6 lists exactly ten
filters in both editions.

⇒ *"when it becomes part of the pdf 2.0 standard"* describes an
**unscheduled** event while an implementable specification already exists.
**Open operator question `(bq)`; default is wait.**

Three things not to rediscover: the **circulating "§7.4.11" citation is
false** (that clause does not exist; traced to an unmerged pypdf PR);
**`FlateDecode`'s predictors apply verbatim**, so pdfce's predictor code is
reusable; and **`brotli` 8.0.4 is BSD-3-Clause AND MIT**, so a read-side
addition needs no operator licence call. Full sourcing:
`D:\Dev\Rag-Specialized\PDF_Spec\filters\filter__brotli.md`.

---

## §9 — TRY IT

```
pdfce-cli render-page <a Ghent transparency patch> --page 1 --scale 2 -o out.png
python tools/ghent-check.py D:\Dev\temp\ghent-patches
python D:\Dev\temp\pdfce\work\measure_space.py <a corpus dir>
```

**And run it on his real drawing before calling anything shipped.** That
rule earned itself three defects in one week. The CAD sheet has no
transparency groups, so this session's work could not touch it — verified
rather than assumed: **0.96 s**, all five new counters at zero.

---

## §10 — THE BENCHMARK FILE, written once

    D:\Dev\temp\pdfce\ncored-benchmark-cad-drawing.pdf

★ **Written here once, on its own line, and referenced from everywhere else
rather than repeated** — a Windows path in prose is the single most reliably
mangled string this project handles. The rule in the engineer's agent
memory: **any shell metacharacter breaks heredoc patching — write the file,
or use `Edit`.**
