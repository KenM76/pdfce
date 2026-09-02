# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**. What is
true now, plus a pointer. Corrections and their prior wording live in the
append-only record (`ROADMAP.md`, `SESSION_LOG.md`).

Written **2026-09-02**, at the end of a session that shipped **Passes 222.0
through 225.0**. Everything below was measured with a shell in that session;
commands are given so nothing here has to be trusted.

**For the ledger — Pass ceiling, rule ceiling, decision ceiling, filing count —
run `python tools/check-ledger-numbers.py`.** As of writing it reports
highest Pass **224.0**, next rule **R240**, next decision **118**, next
filing **366**.

⚠️ **The tool derives the Pass ceiling from the DOCUMENTS, and `Pass 225.0`
is committed (`16eaaa2`) but NOT YET FILED** — so the tool says 224.0 while
225.0 is already used in a commit message. **The next free Pass is 226.0.**
Verify with `git log --oneline -10` before minting a number. Once the
librarian files 225.0 the tool agrees again and this warning can be deleted.
`check-commits-filed.py` naming `16eaaa2` is the ONLY expected gate failure on
arrival; everything else was green at `61c3735`.

---

## §0 ACTIVE — the spot-colorant plane, step 3 of ~4: **THE DEPOSIT**

This is where the first pixel moves. Steps 1 and 2 are in and both were
**proved to change nothing**; step 3 is what gives them effect.

| step | Pass | commit | state |
|---|---|---|---|
| 1 — the `PixelCmyk::s` carrier | 217.0 | `643e270` | in, inert |
| 2 — storage, allocation, blending, collapse | 225.0 | `16eaaa2` | in, inert |
| **3 — the deposit + Table 149's spot rule** | — | — | **next** |
| 4 — images, shadings, knockout groups | — | — | later |

### Why this is the top item

**Seven of the ten failures the operator can see are this one thing** —
`PCS 2.0`, `3.0`, `3.1`, `4.0`, `4.1`, `8.1`, `8.01`. There is no cheap
version of any of them individually.

### What step 2 already gives you — do not rebuild it

All in `crates/pdfce-render/src/cmyk_buffer.rs` unless noted. Every item is
`#[allow(dead_code)]` with the reason on it; **the `allow`s come off in step
3**.

- `SpotPlane { colorant: Box<[u8]>, tint: Vec<Chan>, lut: SpotLut }`
- `CmykBuffer::spot_index(&mut self, colorant: &[u8], lut: impl FnOnce() -> SpotLut) -> Option<usize>`
  — find-or-allocate. The closure runs **only** on first allocation, which is
  what keeps a tint transform off the repeat-paint path.
- `CmykBuffer::spots_flattened()` — colorants refused by the roster cap or the
  byte ceiling. Counted, never silent.
- `SpotLut::build(|tint| -> [f32; 3])`, `SpotLut::transparent()`, `.at(tint)`
  — 256 samples, interpolated, endpoints exact.
- `compositor::blend_spots` — wired into `composite_element_cmyk` already.
- `CmykBuffer::fold_spots_srgb` — the collapse, wired into
  `to_srgb_over_white` already.

### ★★ THE DESIGN CHANGED FROM THE PREVIOUS HANDOFF — there is no roster

The prior version of this file scoped step 2 as *"roster + ONE plane"* with a
pre-pass over the page's resources. **That was not built and should not be.**

Planes are allocated **lazily, at first use**. The argument is correctness,
not speed: a plane created part-way through a page is all zeros behind it, and
**zero is the right value** — "no ink of this colorant" is exactly true of
every mark laid down before the document first named it.

A resource pre-pass would have to recurse into form XObjects, patterns,
annotation appearance streams and Type 3 glyph procedures to be complete, and
any colorant it *missed* would be flattened **silently**, because a roster is
only checkable against the render it was built for. Lazy allocation is
complete by construction.

### What step 3 has to do

1. **`interpret.rs`** — beside `authored_tints`, produce the SPOT half: for a
   `SourceKind::SeparationOrDeviceN`, the `(colorant_bytes, tint)` pairs whose
   names are **not** process channels. `overprint::authored_tints` currently
   drops exactly these on the floor (`process_channel(name)` returns `None` and
   the tint is discarded) — that discard is the defect.
2. **Carry them to the paint call.** `composite_mask` takes `colour: [Chan; 4]`
   and needs the spot pairs alongside. The production call site for the
   ordinary path is the one to find first — `interpret.rs:6226` is the
   **non-separable-blend** path only, not the general fill.
3. **Build each plane's `SpotLut`** from the space's own tint transform: render
   the colorant alone on white at 256 tints. `SpotLut::transparent()` is the
   documented fallback when the transform will not evaluate — **white, not
   black**, because white is multiply's identity and black would paint a solid
   rectangle nobody asked for.
4. **Table 149's spot rule under overprint**: a source that does not NAME a
   colorant leaves that colorant's plane alone. This is the half that fixes
   `PCS 3.0`.
5. **Take the `#[allow(dead_code)]`s off.**

### The measured target, already attributed — do not re-diagnose it

`PCS 3.0` (`2_GWG030…`), traps at device `(27, 68)` and `(28, 135)`, both
`0 0 0 .5 k` and `.5 g` **fills**. `docs/suite-patch-reference.md` §3 carries
the full attribution, measured with `--probe-ink`:

> the backdrop is `/CS1 = [/DeviceN [/Black <spot>] /DeviceCMYK]`; pdfce
> flattens the spot into C/M/Y (measured `0.443 0 0.885`), and a `DeviceCMYK`
> source then knocks C/M/Y out — **destroying a colorant it never named**.

Two leads are already **refuted** there and must not be re-followed: it is not
`overprint_zero_tint_scope`, and it is not the grey *stroke*.

### ★★ TWO HAZARDS THAT WILL NOT ANNOUNCE THEMSELVES

1. **The OPM edition gate flips the moment a fifth plane exists.** ISO 32000-1
   §8.6.7 disables overprint mode if the device space *"is not `DeviceCMYK`"*
   (an identity test); ISO 32000-2 says *"does not include CMYK device
   colourants"* (an inclusion test). **A CMYK+spot buffer is not DeviceCMYK** —
   so under a 1.7 reading, adding one spot plane turns OPM **inert** on that
   page, changing content that has nothing to do with spots. Both readings are
   conformant. Needs its own setting, defaulting to the 2.0 reading so today's
   behaviour is preserved.
2. **Knockout groups drop spot ink today.** `composite_at`'s knockout arm
   builds its initial backdrop from four planes and `s: [0.0; MAX_SPOTS]`.
   Knockout groups are already counted in `groups_approximated`, so this is a
   known approximation rather than a new one — but it is a *silent* one until
   step 4, and worth a counter before then.

### The spec is sourced — cite it, do not re-derive it

**ISO 32000-2:2020 §10.8.3 "Separation simulation"** specifies this in four
steps, with a capability name (`SeparationSimulation`, Table 275) and a NOTE
calling it *"Overprint Preview"*. Full corpus entry with every ambiguity
registered: `D:\Dev\Rag-Specialized\PDF_Spec\iso32000\iso32000__s__10.8.md`.

Four things from it that cost a lookup each:

- **`SEP-A2` is a trap.** Step (c) cites *"Table 133"* for the multiply blend.
  Table 133 is the **variables** table; the blend functions are Table 134
  (= 1.7's Table 136). No erratum filed. Implementing from the citation alone
  lands on the wrong table.
- **`SEP-A3`**: *"flat XYZ (no gamma)"* occurs **once in the whole standard**
  and is defined nowhere. pdfce multiplies in sRGB and says so.
- **`SEP-N1`**: **no `shall` anywhere in §10.8.** The algorithm binds the
  RESULT, not the METHOD. Not implementing it at all is conformant.
- **`SEP-6`**: simulation selects the page group's **blending colour space**;
  it is not an RGB post-filter. `fold_spots_srgb` sits inside
  `to_srgb_over_white` for that reason.

### One implementation note that is load-bearing

The collapse **must not** evaluate a tint transform per pixel. It does not —
`SpotLut` samples once per plane per page — and step 3 must not undo that. An
8.4 Mpx page with four spots would otherwise be 33.6 M evaluations of a
function of one scalar.

---

## §A OTHER CANDIDATES, ranked by measured exposure

| # | Item | Measured exposure |
|---|---|---|
| 1 | **`PCS 22.1`** — a Lab `L*=60` swatch renders `(35,31,32)` where Acrobat gives `(100,101,100)`. Cause NOT diagnosed; the swatch is a Lab fill with a form XObject and an ExtGState over it, so it is **not** simply the Lab conversion. Independent of everything else. | 1 patch, operator-visible |
| 2 | **`PCS 17.2`** — JPEG 2000 with an ICCBased RGB profile; `codestream_space` discards the profile the same way the image path did before `Pass 214.0`. | 1 patch |
| 3 | **73 undocumented public functions**, in `tools/public-fns-undocumented-baseline.txt`. The gate (`Pass 224.0`) stops it growing; shortening it is the stated direction. **Read the item ABOVE each one first** — two of the original 75 were doc blocks welded onto a neighbour, not comments never written. | rule 6, 2.2% of 3,377 |
| 4 | Make `sh` shadings selectable objects (currently counted only). Needs clip tracking — a `sh` fills the current clip and the decomposer does not track `W`/`W*`. | 0.6% of corpus |
| 5 | Resolve `/OC` layer visibility in the decomposer (currently counted only). Needs the catalog's `/OCProperties` default config, which the walk does not have. | **0** files with a layer OFF |

---

## §B STATE OF THE TREE — verified 2026-09-02

- `HEAD` = `61c3735`, **pushed; 0 unpushed at the time of writing.**
  Verify with `git log --oneline origin/main..HEAD` rather than trusting
  this line — it is the fastest-staling fact in this file. Pushing `main`
  is standing-authorized, so a non-zero count here is something to fix, not
  something to ask about.
- Version **0.19.0**, tag `v0.19.0` at `d19d4e4`.
- **`Pass 225.0` (`16eaaa2`) is UNFILED.** `python tools/check-commits-filed.py`
  names it. Dispatch `pdfce-librarian` with its full commit message — it is
  long and carries the design change, the two spec findings and the two
  sabotage results, none of which a subject line can supply.
- Conformance standing: **7 FAIL / 37 pass / 7 UNRESOLVED of 51**, unchanged
  across this whole session by construction
  (`python tools/suite-check.py D:/Dev/temp/suite-patches --reference-dir
  D:/Dev/temp/acro-refs`).
- Backups: last bundle is `pdfce-20260901-1855-faf699a-full.bundle`, which is
  now **six commits stale**. Refresh with
  `git bundle create /d/Dev/pdfce-backups/pdfce-<date>-<sha>-full.bundle --all`
  then `git bundle verify`.

---

## §C THINGS A NEW SESSION MUST KNOW BEFORE TOUCHING ANYTHING

- **Run `bash tools/run-gates.sh` in the FOREGROUND.** Launching it with an
  explicit background flag got it **killed mid-run** this session, which is
  exactly what this section warned about. The harness auto-moving a foreground
  run to the background is fine and has worked every time; asking for
  background up front is not.
- **A `cargo test --workspace` failure inside a gate sweep may be
  STARVATION**, not a real failure. It happened this session — leftover
  processes from the killed run — and the same command passed clean on its
  own. Re-run the named command alone before believing it.
- **Never put prose through a Bash heredoc.** Backslashes and backticks are
  eaten silently. Use `Write`/`Edit`, and `git commit -F <file>`.
  **★ And never `print()` a `★` or an em-dash from a Python one-liner** — the
  console code page raises rather than substitutes, which kills the script
  mid-way. Two scripts died that way this session. Gates in `tools/`
  `sys.stdout.reconfigure(encoding="utf-8", errors="replace")`; ad-hoc scripts
  should just stay ASCII.
- **Stage by path. Never `git add -A`** — the repository is public and agents
  share the working tree.
- **A licensed conformance suite's NAME must never appear in any repo file.**
  Use opaque ids (`PCS 3.0`). `tools/check-suite-name-absent.py` enforces it
  for the work tree and for unpushed commit messages.
- **Check BOTH feature-request channels every session.**
  `D:\Dev\FeatureRequests\pdfce_FeatureRequests\` and `…\iccce_FeatureRequests\`.
  They are outside the repo, so no gate can contradict a stale "it's empty".
- **`docs/core-api/` is engineer-owned and must move in the SAME Pass** as any
  `pub` change to `EditSession`. Run `python tools/check-core-api-verbs.py`,
  which also checks the line and clause counts stated in `index.md`.
- Pushing `main` is standing-authorized. **Cutting a tag/release is not.**

---

## §D ★★ MEASURED NEGATIVES — DO NOT RE-DERIVE THESE

Each cost a full ablation. Every one looked obviously right first.

1. **Do NOT colour-manage ICCBased images with `/N != 4`.** Measured 3× and
   1.8× WORSE on two patches (`20.59 → 62.51`, `17.87 → 31.50`). Managing an
   RGB image moves it onto the INK path, whose terminal CMYK→sRGB conversion is
   separately ~10 levels from the reference. A CMYK image was already
   ink-bound so the profile is pure gain; an RGB image was not.
2. **Do NOT rewire the terminal CMYK→sRGB display conversion to iccce.** Probed
   at all four intents through the document's own `/OutputIntent`: best case
   mean error **8.0** against today's **10.3**, and *every* intent clips red to
   0 where both pdfce and the reference show non-zero. The residual ~10-level
   offset is a known, accepted gap.
3. **Do NOT extend `Pass 201.0`'s shading `ink_reach` narrowing to images.**
   Measured `23.90 → 28.68`, a regression. A shading sits OVER a thin mark, so
   handing an untouched channel back restores it; a photograph COVERS an area,
   so handing back its untouched channels makes it partly transparent.
4. **`OverprintZeroTintScope`'s default is NOT "the reference's reading"** —
   measured false on process geometry. It is trap-neutral (17 → 17) because it
   corrects one cell and breaks another that passes only through a
   compensating error. The honest fix is the literal row assignment
   **together with** the spot plane.
5. **The §11.7.4.2 non-separable guard in `blend_spots` is REDUNDANT** —
   sabotage-verified. `blend_separable`'s own final arm already answers `cs`
   for a non-separable mode. It is kept for legibility and both sites say so;
   **do not read its test going green as proof the guard is load-bearing.**

---

## §E ONE ITEM OWED BY THE OPERATOR

**82 of 1,220 commit messages in published history contain the licensed
suite's name in plaintext**, and the repository is public. `ROADMAP.md` open
question `(ca)` records it with three options stated neutrally. Removing them
means rewriting published history, which project rule 8 reserves to Ken.

Option 3 — extend the gate so the count cannot grow — is **DONE**
(`Pass 208.0`). Options 1 and 2 are his.

Also open, and **not read by this session**: two replies from the sibling
`iccce` project sitting unactioned in its channel, dated 2026-09-01 —
`reply_depend_on_a_pinned_rev_and_the_four_intent_rules_are_accepted.md` and
`reply_all_four_asks_measured_and_your_bpc_would_have_done_nothing.md`. The
first names a dependency decision (how pdfce should pin iccce) that may
already be satisfied — pdfce pins `tag = "v0.3.0"` today — but that was not
verified against what the reply asks for.

---

## §F THE PATTERN THIS SESSION KEPT HITTING

**A claim that was accurate when written, falsified later by an improvement to
the very thing it described, with nothing able to notice.**

- The `--version` banner said pdfce did not link `iccce` for six days after it
  did. The detector waited on a signal (`DEP_ICCCE_PROVENANCE`) that its
  subject never emits — **a detector that cannot fire is indistinguishable,
  from outside, from a condition that has not occurred.**
- A doc comment welded onto its neighbour left `mark_dirty` describing
  `set_pixel`. rustfmt, clippy and `doc_lazy_continuation` are all content with
  a contiguous weld.
- `docs/core-api/` told a consuming project that link destinations were
  unreachable and to implement fit-style parsing itself. It was right when
  written.
- A gate citation named a file that does not exist, written from memory of
  what the gate does rather than from its filename.

⇒ **The detectable symptom is usually one step sideways from the defect.** An
undocumented *neighbour* is how a corrupted doc block shows; an
*unfalsifiable* detector is how a stale claim survives. Three of the four were
found by a reader — an agent or the operator — and not by anything automated,
which is the argument for dispatching one at the end of a Pass rather than
only at the start.
