# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**. What is
true now, plus a pointer. Corrections and their prior wording live in the
append-only record (`ROADMAP.md`, `SESSION_LOG.md`).

Written **2026-09-03** (second session of the day), at the end of a session
that shipped **Passes 245.0 and 246.0** and released **v0.26.0 and v0.27.0**.
Everything below was measured with a shell in that session; commands are
given so nothing here has to be trusted.

**For the ledger — Pass ceiling, rule ceiling, decision ceiling, filing count —
run `python tools/check-ledger-numbers.py`.** Do not mint from memory.

---

## §0 REDACTION NOW REMOVES EVERYTHING UNDER A MARK — text, images, vectors

The session answered pdfceGUI's request *"redaction refuses any region that
touches an image"* (the operator: *"every time I've tried the redact feature
it tells me it can't"*) and then removed the residual the answer exposed:

| content under a mark | before this session | now |
|---|---|---|
| text glyphs | removed (Pass 8) | unchanged |
| raster image, any codec | **whole apply REFUSED** | covered cells destroyed → **paper**, re-encoded Flate; wholly covered → `Do` removed + object tombstoned; shared → copy-on-write clone; undecodable → **mark RETAINED**, disclosed (`Pass 245.0`, `redact_image.rs`) |
| soft mask / stencil mask | — | cleared with the image (transparent / masked-out) |
| vector paths | **left in place, not even disclosed** | **CUT** at the boundary: strokes vs region + stroke width, fills clipped to the complement, wholly-inside deleted (`Pass 246.0`, `redact_vector.rs`) |
| `W`-marked path object | — | paint cut, ORIGINAL geometry kept as the clip (`vector_clips_kept`, noted) |
| malformed path object (foreign op inside, §8.2) | — | the residual: `vector_paths_intersecting`, carrier `DisclosedNotScrubbed` |

**Proof by pixels:** `crates/pdfce-render/tests/redaction_leaves_no_ink.rs`
rasterises an applied redaction and asserts zero inked pixels inside the
region. It found the black-block defect nothing else did.

**On the operator's files** (`C:\Users\Ken\OneDrive\pdfTests\Redact\`, the
two image-bearing drawings): whole-page marks exit 0 with the page blank
(780 / 1,089 path objects dropped, images removed); a corner mark cuts 25
objects and clears 195,960 image cells in 0.30 s.
Re-run: `pdfce-cli redact-mark --page 1 --rect 500,300,700,400 <in> -o m.pdf`
then `pdfce-cli redact-apply m.pdf -o out.pdf`.

---

## §1 NEXT: nothing is owed on redaction — pick from §A

Candidates in the order I would take them:

1. **pdfceGUI's consumption of the reply** — two new report fields groups
   (`images_*`, `vector_*`, `marks_retained`) are not surfaced by their
   panel yet; their mark-time image warning wording should change (the
   reply says how). Wait for their file in `open/`; nothing to build until
   they ask.
2. **Mesh shadings deposit spot planes** (§A 1) — the last flattening route
   of the spot arc; the two type 7 meshes on the operator's X-4 sheet.
3. **The 8 unresolved conformance patches** — read by eye against
   `D:/Dev/temp/acro-refs`.

---

## §A OTHER CANDIDATES, ranked by measured exposure

| # | Item | Measured exposure |
|---|---|---|
| 1 | **Mesh shadings deposit spot planes** — `mesh::paint_cmyk` takes `rules` and no planes. | 0.6 % of corpus; 2 visible pairs on the X-4 sheet |
| 2 | **`sh` shading paints under a redaction mark** — `sh` fills the current clip; the redaction interpreter does not track the clip, so a shading crossing a mark is neither cut nor counted. Unmeasured; no operator file has one. Count-and-disclose is an hour; cutting needs clip tracking. | 0 files seen |
| 3 | **Images and paths inside a form XObject under a mark** — still `form_intersect` disclosed only. The CAD exporters seen so far draw on the page directly. | 0 of 11 operator files |
| 4 | **`set_page_tabs(page, PageTabs)`** — pdfceGUI has not asked. | one request away |
| 5 | **73 undocumented public functions** in `tools/public-fns-undocumented-baseline.txt`. | rule 6 |
| 6 | **`N 1` on the display path**; **other `/Indexed` bases with a non-unit range** — carried from the prior hand-off, unmeasured. | 0 patches |

---

## §B STATE OF THE TREE — verified 2026-09-03 ~12:00Z

- **Shipped this session:** `Pass 245.0` (`98d4377`), `Pass 246.0`
  (`194b3a1`); bumps `7d94fe3` (v0.26.0) and `dfce8a9` (v0.27.0); filings
  390 and 391 (librarian). Decision 125 (tombstone-over-delete, per-mark
  retention). `RedactError::ImageRegion` is GONE (breaking → 0.26.0).
- **Releases:** v0.26.0 tagged, packaged, smoke-tested, pushed, deployed to
  OneDrive `pdfce1`, verified (CI green at `1d91816`). v0.27.0 tagged at
  `dfce8a9`, packaged to `D:\builds\pdfce-20260903-0717-dfce8a9`, fresh-folder
  smoke-tested (both binaries; the corner-mark run prints
  `vector_paths_intersecting=0`, exit 0). Check its push/deploy/verify state
  with `git log --oneline origin/main..HEAD`, `python tools/verify-release.py
  v0.27.0`. OneDrive slots: pdfce1 = 0.26.0 or 0.27.0, pdfce2 = the other —
  read `tools/deploy-onedrive.py --dry-run`.
- **GitHub releases stop at v0.17.0.** No version since has had one;
  `verify-release.py` skips the check because its `gh` subprocess reports
  unavailable. Not a defect fixed this session; a standing state.
  `gh release create` works from the interactive shell if the operator
  wants them resumed.
- **pdfceGUI channel:** `reply_images_are_destroyed_now_and_all_three_asks_ship.md`
  (with a same-day v0.27.0 update appended) awaits their consumption;
  their request file stays in `open/` until then. The two earlier files
  (`reply_disclosures_now_carries_the_why…`, `note_the_overprint_zero_tint…`)
  are also still unconsumed. The iccce reply
  `reply_all_four_asks_measured_and_your_bpc_would_have_done_nothing.md`
  is STILL unread by any pdfce session.
- **Disk:** `target/debug` reached **243.8 GiB** and D: hit 0 bytes free
  mid-sweep (`LNK1318`, "no space on device"). `cargo clean --profile dev`
  fixed it. Check `df -h /d` at session start; if `target/debug` is over
  ~50 GB, clean it BEFORE the gate sweep, not during.
- **`tools/run-gates.sh` cannot survive being backgrounded** (unchanged).
  Run the ~22 script gates in a foreground loop, then
  `cargo test --workspace`, then the no-default-features / wasm / fuzz
  checks. **Run them on the FINAL tree.** Test-total convention: the sum of
  every `test result:` line of `cargo test --workspace` — **5,089 / 0** at
  hand-off.
- **iccce pinned rev** `a4d9003b` (v0.3.0); dependency SET unchanged since
  v0.22.0; `THIRD_PARTY_LICENSES.md` did not move.
- **Every code commit is FILED** — `python tools/check-commits-filed.py`.
- **Backups:** refresh with
  `git bundle create /d/Dev/pdfce-backups/pdfce-<date>-<sha>-full.bundle --all`
  then `git bundle verify` on it. Not refreshed this session.

---

## §C THINGS A NEW SESSION MUST KNOW BEFORE TOUCHING ANYTHING

- **★ A Python heredoc through the Bash tool eats one level of
  backslashes** — and this session it did it to a **Rust byte escape**
  (`b'\n'` arrived as a literal LF; `grep` said "Binary file matches").
  Any patch containing ANY backslash goes through the `Write` tool to
  `D:\Dev\temp\<name>.py`, then `python <file>`. Every patch asserts its
  anchor count. Fourth recurrence; the memory file is updated.
- **A test suite is not a pixel.** Twenty-two green redaction tests said
  destroyed image cells were "cleared"; the render-level proof said they
  were BLACK inside a transparent mark. When the property is visual, assert
  it on a raster (`pdfce-render/tests`), not on the content stream.
- **Stage by path. Never `git add -A`.** Push a code commit and its filing
  commit together.
- **A licensed conformance suite's NAME must never appear in any repo file.**
  `python tools/check-suite-name-absent.py && git push`.
- **Check BOTH feature-request channels every session.**
  `D:\Dev\FeatureRequests\pdfce_FeatureRequests\open\` and
  `…\iccce_FeatureRequests\open\`.
- **`docs/core-api/` is engineer-owned and must move in the SAME commit** as
  any `pub` change to the redaction/`EditSession` surface. Both Passes did.
- **READ CI'S COLOUR FROM GITHUB, EARLY.**
  `gh run list --limit 5 --json status,conclusion,headSha`.

---

## §D ★★ MEASURED NEGATIVES — DO NOT RE-DERIVE THESE

1. **Do NOT clear destroyed image cells to zero.** Zero is black for
   Gray/RGB and the mark's default is transparent (Table 192); the pixel
   proof measured 6,241 black pixels inside a "transparent" region. Paper =
   the colour space's no-ink sample, `/Decode`-aware (`paper_is_ones`).
2. **Do NOT rewrite a `W`-marked path's geometry.** §8.5.4 applies the clip
   AFTER painting; the cut paint goes first and the ORIGINAL construction
   stays as `W n`, else later unmarked content vanishes.
3. **Do NOT reach for a polygon-boolean crate for fills.** `polygon ∩
   convex rect` via Sutherland–Hodgman preserves winding for any subject;
   four complement strips give the difference exactly. A wrong boolean is a
   silent wrong picture.
4. **Do NOT delete a redacted image object; tombstone it** (1×1 paper under
   the same number). `save_full` re-emits every object the dirty set does
   not name, and shared `/Resources` dicts would dangle (decision 125).
5. The prior hand-off's colour negatives (rewiring CMYK→sRGB display to
   iccce; extending `ink_reach` to images; the `cmyk_group_rules` widening;
   the redundant §11.7.4.2 guard) still stand — see `ROADMAP.md` Passes
   240.0–244.0.

---

## §E ITEMS OWED BY THE OPERATOR

- **Open question `(cb)`** — the device-model adjudication; both renders
  conform.
- **Open question `(ca)`** — 82 published commit messages carry the licensed
  suite's name; the gate stops the count growing.
- **GitHub releases** — resume them or not (§B). No one has asked.

---

## §F THE PATTERNS THIS SESSION HIT

**A scoped refusal coarser than what it protects reads as a broken
feature.** Pass 8's image refusal was correct and keyed on the bounding
box; on a CAD title block every rectangle grazes a logo, so the operator's
report was *"it never works"*. The fix was not to relax the rule but to make
the gate measure what the rule protects (the samples), and to refuse per
mark rather than per document.

**Fixing one residual exposed the next, and the next was worse.** The
request was about images; verifying the image fix on the operator's drawing
showed vector lines through the region — never removed, never disclosed.
Disclose first (same Pass), cut second (next Pass, same day). Never leave
the discovered gap silent while the reported one ships.

**The proof that matters is the one an auditor would run.** The unit tests
checked bytes; the auditor checks pixels. One render-level test found a
design choice (black clear value) that every byte-level test agreed with.
