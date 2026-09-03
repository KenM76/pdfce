# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**. What is
true now, plus a pointer. Corrections and their prior wording live in the
append-only record (`ROADMAP.md`, `SESSION_LOG.md`).

Written **2026-09-03**, at the end of a session that shipped **Passes 240.0
through 244.0** and released v0.23.0, v0.24.0 and v0.25.0. Everything below was measured with a shell in that session;
commands are given so nothing here has to be trusted.

**For the ledger — Pass ceiling, rule ceiling, decision ceiling, filing count —
run `python tools/check-ledger-numbers.py`.** Do not mint from memory.

---

## §0 THE COLOUR ARC IS DONE, AND THE SWEEP IS AT 0 FAIL

Every object type now converts an `ICCBased` colour through its embedded
profile and a CIE colour through the output intent, and the routes agree with
each other (`tests/icc_rgb.rs`, `tests/lab_ink.rs`, `tests/managed_shading.rs`):

| route | display (→ sRGB) | ink (→ `/OutputIntent`) | Pass |
|---|---|---|---|
| fill / stroke / text, `N 3` | managed | managed | 240.0 / 199.2 |
| fill / stroke / text, `N 4` | Table 66 fallback (`from_cmyk`) | managed | 199.2 |
| image, `N 3` — direct, `/Indexed`, JPX `colr` | managed | managed | 240.0 |
| image, `N 4` — direct | fallback | **managed (was raw samples — see §D 1)** | 240.0 |
| image, `N 4` — `/Indexed` | fallback | managed | 214.0 |
| `N 1`, any route | fallback | managed with an intent | — |
| `Lab` / `CalRGB` / `CalGray` fill, text, image, `/Indexed` image | `xyz_to_srgb` (unchanged) | **managed — PCS → output intent B2A** | **242.0** |
| shading, mesh — any of the above | as the fill in that space | as the fill in that space | **243.0** — `icc::ColorBridges` is the ONE copy of the ladder |

**Conformance standing: 0 FAIL / 43 pass / 8 unresolved of 51** — from
5 / 38 / 8 at the start of 2026-09-02
(`python tools/suite-check.py D:/Dev/temp/suite-patches --reference-dir D:/Dev/temp/acro-refs`).
The last two (`3.0` k, `4.0.1` k) were NOT the device-model question they
had been filed under — they were `OverprintZeroTintScope`, and the literal
ISO reading clears them now that spot inks have a plane (`Pass 244.0` flipped
the default). **Every patch the harness can judge passes.** The 8 unresolved
are reference-strip / positive-criterion / no-detector patches the harness
cannot score either way; the operator has read those by eye before.

**Renders of the operator's six-page X-4 sheet from the v0.25.0 binary** are
at `D:\Dev\temp\n3\sheet_out\x4_page1..6.png` (full size, scale 1.5) and
`…_small.png` (two-thirds, for viewing). Re-render with
`pdfce-cli render-page <the sheet, in the parity input folder under D:/Dev/temp> --page N --scale 1.5`.
Two things visible on those pages that are NOT harness failures and are
worth knowing before someone reports them:

* `4.1` cell b ("almost White"): the reference shows the same faint X — the
  patch is designed to; pdfce additionally draws a **hairline warm outline
  along the X's edges** where the reference has none. An antialiased
  overprint seam (the cross's edge pixels composite at partial coverage
  against the rect). Unmeasured; small; a candidate, not a failure.
* `6.0` / `6.1`: the two type 7 mesh pairs on page 1 are the still-wrong
  shading pairs — §1 item 1.

---

## §1 NEXT: there is no colour item left on the sweep — pick from §A

Candidates in the order I would take them:

1. **Mesh shadings deposit spot planes** (§A 1) — the last flattening
   route of the spot arc, and the two type 7 meshes on the operator's
   X-4 sheet are the two still-visibly-wrong shading pairs. `mesh::paint_cmyk`
   takes `rules` and no planes. The `ColorBridges` refactor put the mesh's
   colour resolution beside the ramp's; the plane deposit is the next
   asymmetry between them.
2. **The 8 unresolved patches** — read them by eye against
   `D:/Dev/temp/acro-refs` (the harness cannot score a reference-strip or a
   positive-criterion patch). Three of the `MARK?` ones are known failures
   per `tools/suite-check.py`'s docstring.
3. **`set_page_tabs`** when pdfceGUI asks.

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

## §B STATE OF THE TREE — verified 2026-09-03 07:30Z

- **Shipped this session (2026-09-02 → 03):** Passes 240.0 (ICCBased RGB on
  every route; the 214.0 refusal retracted), 241.0 (preset disclosures carry
  a set claim's why), 242.0 (Lab/CalRGB/CalGray through the output intent;
  the /Indexed-over-Lab palette fix), 243.0 (shadings and meshes through
  `ColorBridges`), 244.0 (zero-tint default → literal). Releases v0.23.0,
  v0.24.0, v0.25.0 — each tagged, packaged, smoke-tested, deployed, verified.
  Filings 384–389. Decision 124, rule R240 minted by the librarian.
- **Clean at hand-off:** `git status` empty; `origin/main` == HEAD
  (`7db84cb`); CI green at `6067d9a`; `check-commits-filed.py` clean;
  `verify-release.py v0.25.0` clean. OneDrive: pdfce2 = 0.25.0, pdfce1 =
  0.24.0.
- **pdfceGUI channel:** two files of ours await their consumption —
  `reply_disclosures_now_carries_the_why_of_a_set_claim_delete_your_workaround.md`
  and `note_the_overprint_zero_tint_default_moved_to_device_cmyk_only.md`.
  The iccce reply `reply_all_four_asks_measured_and_your_bpc_would_have_done_nothing.md`
  is STILL unread by any pdfce session.

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
  abbreviation before). Dependency SET unchanged since v0.22.0
  (`iccce-color` became a DIRECT edge in `Pass 242.0`; it was already in
  the tree, so `THIRD_PARTY_LICENSES.md` did not move).
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
4. **★★ RETRACTED 2026-09-03 (`Pass 244.0`): "`OverprintZeroTintScope`:
   LEAVE THE DEFAULT ALONE", counts `grey_as_k_only` 2 / `all_process_spaces`
   2 / `device_cmyk_only` 4.** Those counts predate `Pass 239.0`'s
   group-merge-by-name fix. Re-measured on the 2026-09-03 tree:
   `device_cmyk_only` **0 FAIL / 43 pass**, `grey_as_k_only` 2 FAIL. The
   default is now `DeviceCmykOnly` (ISO 32000-1 to the letter). What survives
   of the old item: the scope no longer reaches a SPOT (`Pass 238.0`) — under
   every scope a spot plane is Table 149's `c_b` — and the combination
   `alternate_space_substitution` + literal scope is WORSE (3 traps on 3.0
   alone), so the spot device model stays `simulate_separations`.
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

**A divergence kept for a compensating error must be re-measured the moment
the error is fixed.** `GreyAsKOnly` stayed the default for one stated reason
(no spot plane); the plane landed in `238.0`/`239.0`, and the default sat for
five more Passes and a stale §D bullet until the operator pointed at two cells.
When you fix the thing a divergence was compensating for, re-run the
divergence's own measurement in the SAME session.

**A measured negative can be a measurement of the wrong thing.** `Pass
214.0` tried the right route, measured 3×, and wrote a refusal — into its
commit, into `FEATURES.md`, and into this file's §D — that stood for a day.
The route had never been probed one level down; the agreement test between
the vector and image cells of one patch, plus one `--probe-ink` on each,
found the defect in under an hour. **An agreement test's failing half is not
always the new code, and a measured negative's cause is not always the thing
that was changed.** Probe the intermediate before recording a refusal.
