# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**. What is
true now, plus a pointer. Corrections and their prior wording live in the
append-only record (`ROADMAP.md`, `SESSION_LOG.md`).

Written **2026-09-02**, at the end of a session that shipped **Passes 240.0,
241.0 and 242.0**. Everything below was measured with a shell in that session;
commands are given so nothing here has to be trusted.

**For the ledger — Pass ceiling, rule ceiling, decision ceiling, filing count —
run `python tools/check-ledger-numbers.py`.** Do not mint from memory.

---

## §0 THE ICC ARC IS DONE FOR FILLS, TEXT AND IMAGES — shadings and meshes are the corner left

`ICCBased` colour is now converted through its own embedded profile on these
routes, and the routes agree with each other (the four-ways fixture,
`crates/pdfce-render/tests/icc_rgb.rs`):

| route | display (→ sRGB) | ink (→ `/OutputIntent`) | Pass |
|---|---|---|---|
| fill / stroke / text, `N 3` | managed | managed | 240.0 / 199.2 |
| fill / stroke / text, `N 4` | Table 66 fallback (`from_cmyk`) | managed | 199.2 |
| image, `N 3` — direct, `/Indexed`, JPX `colr` | managed | managed | 240.0 |
| image, `N 4` — direct | fallback | **managed (was raw samples — see §D 1)** | 240.0 |
| image, `N 4` — `/Indexed` | fallback | managed | 214.0 |
| `N 1`, any route | fallback | managed with an intent | — |
| `Lab` / `CalRGB` / `CalGray` fill, text, image, `/Indexed` image | `xyz_to_srgb` (unchanged) | **managed — PCS → output intent B2A** | **242.0** |
| **shading, mesh — ICCBased RGB OR a CIE space** | **fallback** | **unmanaged** | **owed** |

**Conformance standing: 2 FAIL / 41 pass / 8 unresolved of 51** — from
5 / 38 / 8 at the start of the session
(`python tools/suite-check.py D:/Dev/temp/suite-patches --reference-dir D:/Dev/temp/acro-refs`).
`PCS 13.0`, `17.2` and `22.1` pass. **The two that remain are `3.0` cell k
and `4.0` cell k — the device-model adjudication, decision 119, open operator
question `(cb)`. Both renders conform. Do not spend a Pass on either.** There
is nothing left on the sweep that is pdfce's to fix.

---

## §1 NEXT: shadings and meshes are the last unmanaged colour route

Every other object type now converts an `ICCBased` RGB colour through its
profile and a CIE colour through the output intent. A shading or mesh in
either space still resolves its colour inside `shading.rs:543` /
`mesh.rs:677` through `ColorSpace::to_rgb` (Table 66 reinterpretation on
screen) and `ColorSpace::to_cmyk` (`None`, so `rgb_to_cmyk` on an ink page).
The predicates to reuse are `Interpreter::display_bridge` (ICC → sRGB),
`IccBridgeCache::get` (ICC → ink) and `IccBridgeCache::pcs_bridge` (CIE →
ink); the cache and the intent live on the interpreter, and the shading
builder is called from it. Expect the same twin defect the last two Passes
found: fix the ramp and the mesh vertex reader in the SAME Pass, and write the
fixture as a fill-vs-shading agreement test (`tools/gen-shading-ink-fixtures.py`
is the template).

0 of 51 patches exercise it; corpus exposure unmeasured — **measure before
spending the Pass** (grep the corpus for `/ShadingType` under an `ICCBased`
or `Lab` `/ColorSpace`). If the population is zero, take §A item 1 instead.

---

## §A OTHER CANDIDATES, ranked by measured exposure

| # | Item | Measured exposure |
|---|---|---|
| 1 | **Mesh shadings deposit spot planes** — the last flattening route of the spot arc. `mesh::paint_cmyk` takes `rules` and no planes. Two type 7 meshes on the operator's sheet are the two still-wrong shading pairs. | 0.6 % of corpus |
| 2 | **`set_page_tabs(page, PageTabs)`** — deliberately not built in `Pass 237.0`; filed in Backlog with its sourcing. pdfceGUI has not asked. | one shell request away |
| 3 | **73 undocumented public functions** in `tools/public-fns-undocumented-baseline.txt`. The gate stops it growing. | rule 6 |
| 4 | Make `sh` shadings selectable objects; resolve `/OC` in the decomposer. | 0.6 % / 0 files |
| 5 | **`N 1` on the display path** — one-line widening of `components == 3` in `image::resolve_space_array` and `display_bridge`; unmeasured, no patch fails on it. Measure `PCS 18.2` before and after if it is ever tried. | 0 patches |
| 6 | **Other `/Indexed` bases with a non-unit component range** — `palette_entry` now scales into `Space::component_ranges()`, which is `0..1` for everything but a delegated CIE space. An `/Indexed` over `ICCBased` whose profile `/Range` is not `0..1` would still be wrong; pdfce does not read profile ranges anywhere (`default_decode` says so). | unmeasured |

---

## §B STATE OF THE TREE — verified 2026-09-02

- **`tools/run-gates.sh` cannot survive being backgrounded.** Twice this
  session the harness moved it to the background after 10 min and killed
  it mid-doctest (the first run of the day survived; the next two did not).
  Run it piecewise in the foreground instead: the ~25 fast gates in one
  loop (`bash tools/run-gates.sh --list` prints them), then
  `cargo test --workspace --no-run`, then `cargo test --workspace`, then
  the no-default-features / wasm / fuzz checks. Each stays under 10 min
  with a warm cache. A sweep certifies the tree it ran on, so run all of
  them on the FINAL tree.
- **Push state: run `git log --oneline origin/main..HEAD`.** Pushing `main`
  is standing-authorized; a non-zero count is something to fix.
- **Release state: run `git tag --sort=-v:refname | head -1` and compare with
  `Cargo.toml`'s `version`.** Releasing is standing-authorized (decision 121)
  — tag, `python tools/package-portable.py`, fresh-folder smoke test of both
  binaries, `python tools/deploy-onedrive.py`, `python tools/verify-release.py
  vX.Y.Z` — and it does not skip the gates.
- **iccce is pinned to a `rev`** (`a4d9003bf87c61299fa1c6f9c2e2ffffa30de0c3`,
  the `v0.3.0` tag's commit). The banner prints `iccce: 0.3.0 (rev a4d9003b,
  committed …)` since `Pass 240.0` (it printed the 40-char pin AND the
  abbreviation before). Dependency SET unchanged since v0.22.0.
- **Every code commit is FILED** — `python tools/check-commits-filed.py`.
- **Backups:** refresh with
  `git bundle create /d/Dev/pdfce-backups/pdfce-<date>-<sha>-full.bundle --all`
  then `git bundle verify` on it.

---

## §C THINGS A NEW SESSION MUST KNOW BEFORE TOUCHING ANYTHING

- **Run the gates PIECEWISE in the foreground** — see §B's first bullet;
  the whole-script run does not survive being backgrounded. **Run them on
  the FINAL tree** — a sweep certifies the tree it ran on.
- **★ Never type a bare `git checkout -- <file>` in a command chain.** Keep
  every multi-line edit in a script file under `D:\Dev\temp\` until it is
  committed; a heredoc edit cannot be re-run. Every sabotage must ASSERT it
  applied (`assert s.count(old) == 1`) and restore from a backup copy, not
  from git.
- **★ A Python heredoc through the Bash tool eats one level of
  backslashes in the script's own source** — an `old` anchor written as
  `\\` (to match a Rust string's line-continuation `\`) arrives as `\`, and
  `s.count(old)` returns 0 against a file that visibly contains the text.
  Non-ASCII is NOT the problem (tested: `len("a — b") == 5`), and the first
  diagnosis this session blamed it anyway. Any patch script containing a
  backslash goes through the `Write` tool to a file, then `python <file>`.
  Cost: one failed patch, caught by its own `assert s.count(old) == 1`.
- **Pillow cannot write or read an ICC profile in a JP2** (`icc_profile` is
  ignored on JPEG2000 save; `.info` never carries a `colr` profile back).
  `tools/gen-icc-rgb-fixtures.py` rewrites the `colr` box itself and
  re-walks the boxes to verify.
- **READ CI'S COLOUR FROM GITHUB, EVERY SESSION, EARLY.**
  `gh run list --limit 10 --json status,conclusion,headSha,createdAt`.
- **Push a code commit and its filing commit together.** CI runs one job per
  push, on the tip.
- **Stage by path. Never `git add -A`.**
- **A licensed conformance suite's NAME must never appear in any repo file.**
  `python tools/check-suite-name-absent.py && git push`. The temp folders
  under `D:\Dev\temp\suite-patches` carry it in their FILE NAMES — never
  paste a listing of that folder into a commit message or a dispatch.
- **Check BOTH feature-request channels every session.**
  `D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\` and
  `…\iccce_FeatureRequests\open\`. This session answered pdfceGUI's
  `disclosures()` note (`reply_disclosures_now_carries_the_why_of_a_set_claim_delete_your_workaround.md`,
  awaiting their consumption). The iccce reply
  `reply_all_four_asks_measured_and_your_bpc_would_have_done_nothing.md`
  (409 lines) has **still not been read** by any pdfce session.
- **`docs/core-api/` is engineer-owned and must move in the SAME Pass** as
  any `pub` change to `EditSession`. Neither Pass this session touched it.

---

## §D ★★ MEASURED NEGATIVES — DO NOT RE-DERIVE THESE

1. **★★ RETRACTED 2026-09-02 (`Pass 240.0`): "Do NOT colour-manage
   ICCBased images with `/N != 4` onto the ink path — measured 3× worse".**
   The numbers (`20.59 → 62.51`, `17.87 → 31.50`) were real and measured a
   DEFECT: a direct `Space::Icc` image was outside the texel loop's
   `tinting` route, so its ink arm wrote the RAW samples as C, M, Y, K. For
   `N 3` that was RGB written as ink. With `Icc` and `IccRgb` on the cached
   route the same two patches read `19.98 → 19.51` and `17.39 → 15.95`, and
   on `PCS 13.0` the image cell's ink equals the vector cell's at the probe
   (`c=0.850 m=0.030 y=1.000 k=0.150`). **The lesson replaces the rule:** a
   negative measured through an untested intermediate measures the
   intermediate; probe it (`render-page --probe-ink X,Y` on a vector/image
   pair of one authored colour) before recording a route as refused.
   **Also measured this session:** for a PDF/X page the ink route IS the
   reference's behaviour — Acrobat converts an ICC RGB image through the
   output intent (`(10,141,49)`), not profile→sRGB directly (`(0,154,0)`,
   which is what lcms2 and iccce both give). Direct-to-sRGB is right only on
   a page without an output intent.
2. **Do NOT rewire the terminal CMYK→sRGB display conversion to iccce.**
   Best intent through the document's `/OutputIntent`: mean error 8.0 vs
   today's 10.3, and every intent clips red to 0 where both pdfce and the
   reference are non-zero. (This is why `N 4` stays on `from_cmyk` for
   display; an embedded CMYK profile → sRGB is the same class of transform.)
   The residual on the two ICC patches that pass is this conversion: on
   `17.2` the `DeviceCMYK` surround is `(13,169,76)` against the reference's
   `(77,175,48)`.
3. **Do NOT extend `Pass 201.0`'s shading `ink_reach` narrowing to images.**
   Measured `23.90 → 28.68`. Stays for the plane-less fallback.
4. **`OverprintZeroTintScope`: LEAVE THE DEFAULT ALONE.** Per-patch trap
   counts with the spot plane in place: `grey_as_k_only` (default) 2,
   `all_process_spaces` 2, `device_cmyk_only` 4.
5. **The `cmyk_group_rules` mixed-source widening to `[Source; 4]` is
   correct WITHOUT planes and wrong WITH them.** `cmyk_group_rules_with_planes`
   carries the switch. Do not delete the widening.
6. **The §11.7.4.2 non-separable guard in `blend_spots` is REDUNDANT** —
   sabotage-verified; kept for legibility.

---

## §E ITEMS OWED BY THE OPERATOR

- **Open question `(cb)`** — the device-model adjudication behind `3.0 k`
  and `4.0 k`. Both renders conform; the choice is his.
- **Open question `(ca)`** — 82 published commit messages carry the licensed
  suite's name; the gate stops the count growing.
- **The `set_page_tabs` verb** needs no ruling until pdfceGUI asks.

---

## §F THE PATTERNS THIS SESSION HIT

**An agreement test finds the OLD defect beside the new route.** Writing
the three-ways `Lab` fixture for `Pass 242.0` exposed an `/Indexed`-over-
`Lab` palette that had decoded L\* 100× too dark since `palette_entry` was
written — on the CONTROL page, which the new route never touches. Fill vs
image vs palette of one colour, probed with `--probe-ink`, is now three for
three at finding a route twin on the day a route is fixed. Write it first.

**A default-valued fixture cannot falsify a carry** (R225, again): the
`Lab` fixtures declare D50, so the D50 adaptation in `to_pcs_xyz` is the
identity there and a sabotage that deleted it survived. The unit test on a
D65-declared space is what pins it. When a fixture's value equals what the
code would produce with the feature removed, the fixture is not a test of
the feature.

**A sabotage harness is itself a claim.** The first sweep reported all
four mutations surviving because `cargo test --test X --lib filter` applies
the filter to BOTH targets. Read the harness's own output for which tests
actually ran before believing a survivor.

**A measured negative can be a measurement of the wrong thing.** `Pass
214.0` tried the right route, measured 3×, and wrote a refusal — into its
commit, into `FEATURES.md`, and into this file's §D — that stood for a day.
The route had never been probed one level down; the agreement test between
the vector and image cells of one patch, plus one `--probe-ink` on each,
found the defect in under an hour. **An agreement test's failing half is not
always the new code, and a measured negative's cause is not always the thing
that was changed.** Probe the intermediate before recording a refusal.
