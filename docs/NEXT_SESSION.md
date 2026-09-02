# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**. What is
true now, plus a pointer. Corrections and their prior wording live in the
append-only record (`ROADMAP.md`, `SESSION_LOG.md`).

Written **2026-09-02**, at the end of a session that shipped **Passes 237.0
through 239.0**. Everything below was measured with a shell in that session;
commands are given so nothing here has to be trusted.

**For the ledger — Pass ceiling, rule ceiling, decision ceiling, filing count —
run `python tools/check-ledger-numbers.py`.** Do not mint from memory.

---

## §0 THE SPOT-COLORANT PLANE ARC IS DONE ON THE PAINT ROUTES — one corner left

Every paint route now deposits a spot colorant into its own plane and the
planes survive every group construct:

| route | Pass | note |
|---|---|---|
| path fill, stroke, text | 228.0 / 229.0 / 230.0 | |
| stencil mask (`/ImageMask`) | 238.0 | painted as a fill with the image's shape |
| sampled `Separation`/`DeviceN` image, direct or `/Indexed` | 238.0 | |
| process-space image under `/OP true` | 238.0 | leaves the spot planes standing (`SpotSource::Preserve`) |
| `sh` shading, types 1–3 | 239.0 | |
| shading PATTERN fill | 239.0 | first native ink route this site ever had |
| isolated / non-isolated / knockout group merge | 239.0 | planes mapped by **colorant name**, never by index |
| **mesh shading, types 4–7** | **owed** | still flattens through the tint transform; `mesh::paint_cmyk` has no ramp-shaped spot half. Small, rare in the corpus (0.6 %), disclosed in the render-page paragraph |

**Conformance standing: 5 FAIL / 38 pass / 8 unresolved of 51** — from
7 / 37 / 7 at the start of the session
(`python tools/suite-check.py D:/Dev/temp/suite-patches --reference-dir D:/Dev/temp/acro-refs`).
`PCS 2.0` is clean in all ten cells; `PCS 3.1` renders its own "Correct"
reference; `PCS 8.0` and `8.1` show every check mark, images and gradient
bars alike.

★ **`tools/suite-check.py` changed:** a mark-criterion patch (check marks that
should be PRESENT) is routed to `MARK?` **before** the cross detector. A
check mark IS two diagonal strokes; on `8.1` the detector was counting
pdfce's correct marks as crosses. `MARK?` is not a pass — look at the render.

### The five that remain, and what each actually is

| patch | what it is | who owns it |
|---|---|---|
| `3.0` cell k, `4.0` cell k | **the device-model adjudication** — decision 119, `ARCHITECTURE.md` §12, open operator question `(cb)`. Both renders conform; pdfce is on the separation-simulation branch (2.0 §10.8.3), the reference is on alternate-space substitution (§8.6.6.4's `shall`). **Do not spend a Pass on either.** §D item 4 below has the measured negative for the scope setting | operator |
| `13.0` cell b | an `/Indexed` over `ICCBased /N 3` **image** whose profile is deliberately not sRGB; the same colour as a vector ICCBased fill (cell a) renders green because the fill is colour-managed and the image is not | **§1 below** |
| `17.2` | JPEG 2000 with an `ICCBased` RGB profile; same gap as 13.0 b plus `codestream_space` discarding the profile | §1 below |
| `22.1` | a Lab `L*=60` swatch renders `(35,31,32)` against `(100,101,100)`; a Lab fill under a form XObject + ExtGState, cause NOT diagnosed | §A |

---

## §1 NEXT: colour-manage `ICCBased /N 3` IMAGES to sRGB — and NOT through the ink path

**Read §D item 1 first**, because it looks like it forbids this and it does
not. That measured negative routed RGB images **onto the ink path** (manage →
CMYK → the terminal CMYK→sRGB conversion, which is separately ~10 levels
off), and got 3× worse. The route that fixes `13.0 b` is different: the
image's embedded profile → **sRGB directly** (iccce exposes a built-in sRGB
*destination*; `icc.rs` documents that it deliberately does not expose a
built-in sRGB *source*, which is the right asymmetry). An RGB image stays on
the RGB path; only its numbers get the meaning the document gave them.

Where it goes: `image.rs` `Space::Icc` already carries the profile for
`/N 4` (`Pass 214.0`); the `/N 3` case falls to `Space::Rgb` and loses it.
`resolve_indexed` needs the same for an `/Indexed` over such a base (that is
`13.0 b`'s exact shape — measured: `[/Indexed [/ICCBased 25 0 R (N 3, 346
bytes)] 255 …]`). The vector cell (a) is the oracle: same page, same profile,
same authored colour, and it is already green.

`17.2` then needs `codestream_space` to stop discarding the profile the same
way the image path did before `Pass 214.0`.

**Do not touch `cmyk_intent`, and do not route through the output intent** —
that is §D item 2.

---

## §A OTHER CANDIDATES, ranked by measured exposure

| # | Item | Measured exposure |
|---|---|---|
| 1 | **`PCS 22.1`** — the Lab swatch. Not diagnosed. Start by probing the pixel with and without the ExtGState. | 1 patch, operator-visible |
| 2 | **Mesh shadings deposit spot planes** — the last flattening route (§0 table). `mesh::paint_cmyk` takes `rules` and no planes; `Shade::Rgb` per-vertex colour has nowhere to put colorants. Two type 7 meshes on the operator's sheet are also the two still-wrong shading pairs (`Pass 137.0`'s measurement). | 0.6 % of corpus |
| 3 | **`set_page_tabs(page, PageTabs)`** — the verb `Pass 237.0` deliberately did not build. Gated: refuse `A`/`W` below PDF 2.0 and on a PDF/UA-1 file; offer `W` as the more expressive value with `TAB-A1` disclosed. Filed in Backlog with its sourcing (`iso32000__ref__annots_array_order.md` §8.3). pdfceGUI has not asked for it yet. | one shell request away |
| 4 | **Three stale worked examples** quote the provenance banner as `0.3.0 (tag v0.3.0, a4d9003b, …)` — `crates/pdfce-core/build.rs:272`, `crates/pdfce-core/src/build.rs:32` and `:109` — and the live banner now prints `rev a4d9003b…` because the printer derives from `Cargo.lock` (verified 2026-09-02 with `pdfce-cli --version`). Fix in the first code Pass of the next session; a worked example is a claim. `FEATURES.md`'s *Build provenance stamp* row quotes the same string — librarian. | rule 6 |
| 5 | **73 undocumented public functions** in `tools/public-fns-undocumented-baseline.txt`. The gate stops it growing. | rule 6 |
| 6 | Make `sh` shadings selectable objects; resolve `/OC` in the decomposer. | 0.6 % / 0 files |

---

## §B STATE OF THE TREE — verified 2026-09-02

- **Push state: run `git log --oneline origin/main..HEAD`.** Pushing `main`
  is standing-authorized; a non-zero count is something to fix.
- **Release state: run `git tag --sort=-v:refname | head -1` and compare with
  `Cargo.toml`'s `version`.** Releasing is standing-authorized since decision
  121 (tag, package, fresh-folder smoke test, OneDrive deploy,
  `verify-release.py`) — but it does not skip the gates. **`v0.21.0` was
  tagged last session with packaging / smoke / deploy / verify NOT recorded as
  done.** If this session's `v0.22.0` did not complete all five steps, finish
  them before anything else: `python tools/package-portable.py`, copy the
  folder somewhere fresh and launch both binaries, `python
  tools/deploy-onedrive.py`, `python tools/verify-release.py v0.22.0`.
- **iccce is pinned to a `rev`** (`a4d9003bf87c61299fa1c6f9c2e2ffffa30de0c3`,
  which IS the `v0.3.0` tag's commit) as of 2026-09-02, at iccce's own request
  (`iccce_FeatureRequests/open/reply_depend_on_a_pinned_rev_and_the_four_intent_rules_are_accepted.md`).
  Both lockfiles (`Cargo.lock`, `fuzz/Cargo.lock`) moved with it. The
  dependency SET did not change, so `THIRD_PARTY_LICENSES.md` did not.
- **Every code commit is FILED** — `python tools/check-commits-filed.py`.
- **Backups:** the last verified bundle predates this session. Refresh with
  `git bundle create /d/Dev/pdfce-backups/pdfce-<date>-<sha>-full.bundle --all`
  then `git bundle verify` on it.

---

## §C THINGS A NEW SESSION MUST KNOW BEFORE TOUCHING ANYTHING

- **Run `bash tools/run-gates.sh` in the FOREGROUND**, and let the harness
  background it. **Run it on the FINAL tree** — a sweep certifies the tree it
  ran on. This session ran it twice per Pass because both first sweeps caught
  the same two shapes: a wrapped string literal with a run of spaces, and a
  doc block orphaned by an insertion between `///` and its item. **Those two
  are now four-for-four across two Passes.** Insert BEFORE a doc block, never
  between it and its `fn`; never continue a string literal with `\` from a
  Python heredoc.
- **★ Never type a bare `git checkout -- <file>` in a command chain.** This
  session lost the uncommitted group-merge half of `Pass 239.0` to one that
  was typed as a "no-op" after a sabotage run. It was re-applied from the
  script that had generated it — which is the only reason it was cheap. Keep
  every multi-line edit in a script file under `D:\Dev\temp\` until it is
  committed; a heredoc edit cannot be re-run.
- **A sabotage that survives is a fixture question before it is a code
  question.** Three fixtures in `Pass 239.0` passed under sabotage on first
  writing, each for a construction reason: on white paper a deposited and a
  flattened spot collapse to the same sRGB *by construction*; a colorant the
  parent already holds at the child's index merges correctly *by luck*; under
  Normal blend a knockout's backdrop removal hides a missing initial spot
  *exactly*. Each generator records its own reason. Demand the discriminating
  geometry before believing green.
- **Never put prose through a Bash heredoc.** Use `Write`/`Edit`, and
  `git commit -F <file>`. Python one-liners must stay ASCII.
- **READ CI'S COLOUR FROM GITHUB, EVERY SESSION, EARLY.**
  `gh run list --limit 10 --json status,conclusion,headSha,createdAt`.
- **Push a code commit and its filing commit together.** CI runs one job per
  push, on the tip.
- **Stage by path. Never `git add -A`.**
- **A licensed conformance suite's NAME must never appear in any repo file.**
  `python tools/check-suite-name-absent.py && git push`.
- **Check BOTH feature-request channels every session.**
  `D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\` and
  `…\iccce_FeatureRequests\open\`. Two pdfce requests were answered this
  session (replies in `open/`, awaiting pdfceGUI's consumption); the iccce
  pinning reply is now actioned (§B). The second iccce reply
  (`reply_all_four_asks_measured_and_your_bpc_would_have_done_nothing.md`) was
  **not read** by this session.
- **`docs/core-api/` is engineer-owned and must move in the SAME Pass** as
  any `pub` change to `EditSession`. `python tools/check-core-api-verbs.py`
  also checks the line and clause counts in `index.md`.

---

## §D ★★ MEASURED NEGATIVES — DO NOT RE-DERIVE THESE

1. **Do NOT colour-manage ICCBased images with `/N != 4` ONTO THE INK PATH.**
   Measured 3× and 1.8× worse on two patches (`20.59 → 62.51`,
   `17.87 → 31.50`). The failure was the route (image → CMYK → the terminal
   CMYK→sRGB conversion), not the management. §1 above is the other route.
2. **Do NOT rewire the terminal CMYK→sRGB display conversion to iccce.**
   Best intent through the document's `/OutputIntent`: mean error 8.0 vs
   today's 10.3, and every intent clips red to 0 where both pdfce and the
   reference are non-zero.
3. **Do NOT extend `Pass 201.0`'s shading `ink_reach` narrowing to images.**
   Measured `23.90 → 28.68`. (With planes the narrowing is off for shadings
   too — `Pass 239.0` — so this item is now about a route that no longer
   exists for plated spots; it stays for the plane-less fallback.)
4. **`OverprintZeroTintScope`: LEAVE THE DEFAULT ALONE.** Per-patch trap
   counts with the spot plane in place: `grey_as_k_only` (default) 2,
   `all_process_spaces` 2, `device_cmyk_only` 4. The literal reading is twice
   as bad and moves the traps rather than removing them.
   ★ **And the scope no longer reaches a SPOT at all** (`Pass 238.0`): under
   every scope a spot plane is Table 149's `c_b`; the scope decides the four
   process rules only. The grey-over-spot fixtures pin preservation now; the
   scope discriminators are over process ink (`OP-N3`).
5. **The `cmyk_group_rules` mixed-source widening to `[Source; 4]` is
   correct WITHOUT planes and wrong WITH them.** `cmyk_group_rules_with_planes`
   carries the switch. Do not delete the widening: shadings without planes
   (meshes, refused rosters) still need it.
6. **The §11.7.4.2 non-separable guard in `blend_spots` is REDUNDANT** —
   sabotage-verified; kept for legibility.

---

## §E ITEMS OWED BY THE OPERATOR

- **Open question `(cb)`** — the device-model adjudication behind `3.0 k`
  and `4.0 k` (§0 table). Both renders conform; the choice is his.
- **Open question `(ca)`** — 82 published commit messages carry the licensed
  suite's name; removing them means rewriting published history. The gate
  stops the count growing (`Pass 208.0`).
- **The `set_page_tabs` verb** needs no ruling — but PDF/UA-1 forbids `/A`
  outright, so a shell offering it must know the file's conformance target.
  That is a product question for when pdfceGUI asks.

---

## §F THE PATTERN THIS SESSION KEPT HITTING

**A fix on one route exposes the same defect's twin on a route that had
looked correct.** Three times in one day, each found by an agreement test
between two routes rather than by a reference:

- The image path deposited a `/Indexed` spot correctly and rendered WHITE,
  because the FILL path had been depositing the palette **index** as the tint
  and building the plane's curve from the index space — two wrongs that
  cancelled on their own route for eleven Passes.
- The image path's mixed-`DeviceN` rules were the table's, and the FILL
  path's were the `Pass 195.0` widening; the duotone showed which.
- The shading path deposited its spot and the group merge dropped it,
  because every buffered group had been merging spot planes by **index**
  since the planes existed — invisible until something outside a group put
  a different colorant at index 0.

⇒ **An agreement test's failing half is not always the new code.** Probe
both sides before assuming; the ink probe (`render-page --probe-ink X,Y`,
which reports spot planes since `Pass 231.0`) settled each of these in one
run.
