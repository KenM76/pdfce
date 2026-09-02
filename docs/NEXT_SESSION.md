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
highest Pass **229.0**, next rule **R240**, next decision **119**, next
filing **371**. The tool and the commit log AGREE — every code commit is
filed, so the next free Pass is **230.0** with no caveat.

---

## §0 ACTIVE — the spot-colorant plane, step 3 of ~4: **THE DEPOSIT**

This is where the first pixel moves. Steps 1 and 2 are in and both were
**proved to change nothing**; step 3 is what gives them effect.

| step | Pass | commit | state |
|---|---|---|---|
| 1 — the `PixelCmyk::s` carrier | 217.0 | `643e270` | in, inert |
| 2 — storage, allocation, blending, collapse | 225.0 | `16eaaa2` | in, inert |
| 3a — the spot-tint READER | 227.0 | `983b438` | in, inert |
| 3b — the DEPOSIT (ordinary paint path) | 228.0 | `9a18510` | **in, PCS 2.0 7→4 traps** |
| 3c — the deposit under OVERPRINT | 229.0 | `f97c15b` | in; suite cannot reach it, unit-tested |
| **4 — images, shadings, knockout groups** | — | — | **next** |
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

### Step 3a is DONE (`Pass 227.0`, `983b438`) — the reader

`overprint::authored_spots(&SourceKind, &[f32]) -> Vec<(&[u8], f32)>` and
`overprint::names_a_spot_colorant(&SourceKind) -> bool`, with eight tests and
the three §8.6.6.4 identity rules (`/None` and `/All` never become planes; a
process-colorant name is not a spot). Nothing calls them; no pixel moved.

### Step 3b is next — the DEPOSIT, and the call chain is TRACED

Do not re-derive this; it cost a session's tracing. The ordinary fill reaches
the ink buffer through exactly this chain:

```text
interpret::solid_authored          (interpret.rs, builds the paint)
  -> BrushSpec::with_cmyk          (canvas.rs:204)
  -> BrushSpec.cmyk: Option<[f32;4]>
  -> cmyk_paint.rs:244-252         (the ONLY consumer)
  -> CmykBuffer::composite_mask
```

`cmyk_paint.rs:244` is the single place that decides authored-vs-reconstructed
(`let bridged = brush.cmyk.is_none()`), and `:252` is the only production call
to `composite_mask`. **That is where a spot tint has to arrive.**

The other three `canvas.cmyk_mut()` sites are NOT the general fill and should
not be touched first: `interpret.rs:4831` and `:6408` are shadings (bridged
through an sRGB scratch by design), `:6222` is the non-separable-blend path.
`interpret.rs:6095` is the OVERPRINT fill — `composite_overprint` — and it is
the one that fixes `PCS 3.0`, but the ordinary path should land first so the
two can be measured apart.

### ★★ THE OPEN DESIGN QUESTION, and it is the whole of step 3b

**How does the `SpotLut` reach `cmyk_paint.rs`?**

Building it needs the tint transform, which lives on the *colour space* and is
known only in `interpret.rs`. Consuming it happens in `cmyk_paint.rs`, which
sees only a `BrushSpec`. Four options, with what is wrong with each:

1. **Build the LUT in the interpreter and put it on `BrushSpec`.** 256 samples
   per paint. `BrushSpec` is `Clone` and is cloned per paint — this is the
   obvious design and it is the one the whole `SpotLut` type exists to avoid.
2. **Carry `Arc<SpotLut>` on `BrushSpec`, with a per-document cache in the
   interpreter keyed on the colorant name.** Clone is a refcount bump; the
   transform is sampled once per colorant per document. **This is the
   recommended one.** The cache key must be the raw name BYTES, for the same
   §7.3.5 NOTE 4 reason `SpotPlane` uses.
3. **Resolve the plane index in the interpreter** and put `(index, tint)` on
   the spec. Needs `&mut CmykBuffer` at spec-construction time, which
   `solid_authored` does not have.
4. **Pass a builder closure through.** Makes `BrushSpec` non-`Clone` or
   requires boxing; fights the type for no gain over (2).

Whichever is chosen, the LUT must be **the colorant alone on white** —
§10.8.3 step (b)'s *"background matte of all white"* — because step (c)'s
multiply treats each entry as a transmittance. `SpotLut::transparent()` is the
documented fallback when a transform will not evaluate: **white, not black**,
because white is multiply's identity and black paints a solid rectangle nobody
asked for.

### Then, in order

1. `composite_mask` gains the spot tints and writes them into `PixelCmyk::s`.
2. **Table 149's spot rule under overprint** — a source that does not NAME a
   colorant leaves that colorant's plane alone. This is the half that fixes
   `PCS 3.0`, and it belongs in `composite_overprint`, not `composite_mask`.
3. **Take the `#[allow(dead_code)]`s off** — they are on `SpotPlane`,
   `SpotLut`, `spot_index`, `spots_flattened` and the `spots_flattened` field.
4. Re-run the conformance sweep. **This is the first step where the numbers
   are allowed to move**, and the two traps to watch are named below.

### ★★ THE TARGET IS NARROWER NOW — MEASURED 2026-09-02, do not start over

`Passes 228.0`/`229.0` shipped the deposit. `PCS 2.0` went **7 traps → 4**.
`PCS 3.0` is unchanged at 3, and the cause is no longer "no spot plane" —
here is what was measured on the shipped binary rather than reasoned:

- **The deposit reaches `PCS 3.0`'s backdrop correctly.** Probed with a
  temporary counter: 12 paints arrive with Black at `0.5` in the process
  channels and the spot at full tint in its own plane. That half works.
- **At the reported trap centres the X and its surround are now
  IDENTICAL** — `--probe-ink` gives `c=0 m=0 y=0 k=0.500` and
  `srgb=82,115,37` at both `(27,68)` and a neighbour 13 px away. The patch
  reference records these as `0 0 0 0.500` against `0.443 0 0.885 0.500`
  **before** this work, so that specific divergence is gone.
- **But a grey `(147,149,152)` region persists next to the green
  `(82,115,37)`.** That grey is 50 % K over white with **no spot ink at
  all** — so somewhere in those cells the green backdrop is absent rather
  than merely knocked out.

⇒ **Next step is to map cells to content-stream rectangles** and find which
fill is landing without the plane. `docs/suite-patch-reference.md` §3 has
the 12-cell layout (a–f `OPM 0`, g–l `OPM 1`). The remaining suspects, in
order: the `sh`/pattern paths and the IMAGE path, neither of which
deposits; and knockout groups, whose initial backdrop is built from four
planes and drops spot ink.

★ `--probe-ink` reports the four PROCESS channels only — it does **not**
show a spot plane's tint. So "the ink is identical" from that probe means
identical *process* ink, and the sRGB is the only place the plane shows
up. A spot-plane read-out on that probe would have saved an hour here and
is worth adding.

### The original attribution, still true as far as it goes

`PCS 3.0`, traps at device `(27, 68)` and `(28, 135)`, both
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

- **Push state: run `git log --oneline origin/main..HEAD`.** This file
  deliberately does NOT name a tip hash — an earlier draft did, and it went
  stale inside the same session, twice, because the commit that updates the
  handoff necessarily changes the thing the handoff just measured. Pushing
  `main` is standing-authorized, so a non-zero count is something to fix,
  not something to ask about.
- The measurements in this section were taken at `1c448e7` (the 366th
  filing). Anything after that is this file's own commit.
- Version **0.19.0**, tag `v0.19.0` at `d19d4e4`.
- **Every code commit is FILED.** `python tools/check-commits-filed.py` is
  clean; `Pass 225.0` was filed as the 366th filing, which also minted
  **decision 118** (lazy spot-plane allocation replaces the pre-pass roster).
- Conformance standing: **7 FAIL / 37 pass / 7 UNRESOLVED of 51**, unchanged
  across this whole session by construction
  (`python tools/suite-check.py D:/Dev/temp/suite-patches --reference-dir
  D:/Dev/temp/acro-refs`).
- Backups: **current** —
  `/d/Dev/pdfce-backups/pdfce-20260902-0055-f0a55fe-full.bundle`, verified
  ("records a complete history"). Refresh after the next batch with
  `git bundle create /d/Dev/pdfce-backups/pdfce-<date>-<sha>-full.bundle --all`
  then `git bundle verify` on it.

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
- **★★ READ CI'S COLOUR FROM GITHUB, EVERY SESSION, EARLY.**
  `gh run list --limit 10 --json status,conclusion,headSha,createdAt`.

  **Measured 2026-09-02: CI was RED for roughly nineteen hours — 34 of the
  last 40 runs, unbroken from 2026-09-01 08:10Z — across the whole of the
  previous session and about twenty pushed commits.** It went green only when
  this session happened to fix the two causes for unrelated reasons: nine
  unfiled commits (`check-commits-filed`) and three baked-in string gaps
  (`check-string-gaps`), both of which were found by running the LOCAL sweep,
  not by looking at CI.

  Nobody looked. The rule was already in this file and in `CLAUDE.md` rule 8;
  a local green sweep reads as "everything is fine" and the remote had been
  saying otherwise since the morning.

- **★ PUSH A CODE COMMIT AND ITS FILING COMMIT TOGETHER, in one `git push`.**

  A filing commit cannot contain code (it would fail the gate it exists to
  satisfy) and a code commit cannot be filed before it exists. So there is
  always a moment where `check-commits-filed` is legitimately red — and if you
  push during it, **CI goes red for a non-defect.** That happened twice on
  2026-09-02 (`983b438` and `78958ff`), and diagnosing it cost a tick.

  **Evidence that pushing both at once fixes it, rather than an assumption:**
  the push `9d43079..61c3735` contained two commits (`16eaaa2` code +
  `61c3735` filing) and produced **exactly one** CI run, on the tip, green.
  GitHub runs one job per *push*, on the tip — not one per commit. Every
  other push this session carried a single commit, so that two-commit push is
  the only observation that distinguishes the two, and it is the one that
  settles it.

  ⇒ Commit the code, dispatch the librarian, commit the filing, **then** push.

- **Stage by path. Never `git add -A`** — the repository is public and agents
  share the working tree.
- **A licensed conformance suite's NAME must never appear in any repo file.**
  Use opaque ids (`PCS 3.0`). `tools/check-suite-name-absent.py` enforces it
  for the work tree, for STAGED content, and for unpushed commit messages.

  ★★ **RUN IT BEFORE EVERY PUSH, NOT BEFORE EVERY BUILD.** It was run before
  every code commit last session and clean each time; it was NOT run before
  the two DOCS commits, and the handoff you are reading leaked a patch
  filename into the **public** repository that way. The gate had become
  associated with *shipping code* rather than with *pushing anything*, and
  pushing `main` is standing-authorized, so nothing paused to ask.

  ★ It also reads **untracked files and your own commit message**. Both
  tripped while writing the incident report for the leak — an explanation of
  a leaked name is itself a place the name leaks, and quoting the string to
  describe the mistake is not an exemption from the rule the mistake broke.

  Practical form: `python tools/check-suite-name-absent.py && git push`.
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
