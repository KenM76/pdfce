# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. **Overwrite it once acted on.**

Per standing rule `R216` this file carries **no edit-history layer**. What is
true now, plus a pointer. Corrections and their prior wording live in the
append-only record (`ROADMAP.md`, `SESSION_LOG.md`).

Written **2026-09-01**, at the end of a long session that shipped **Passes
199.2 through 221.0** and cut **v0.19.0**. Everything below was measured in
that session with a shell; commands are given so nothing here has to be
trusted.

**For the ledger — rule ceiling, decision ceiling, filing count — run
`python tools/check-ledger-numbers.py`.** As of writing: next rule **R240**,
next decision **118**, next filing **363**.

★★ **BUT DO NOT TAKE THE PASS NUMBER FROM THAT TOOL RIGHT NOW.** It derives the
ceiling from the **documents**, and nine commits are unfiled — so it reports a
highest ID of **212.0** while **213.0 through 221.0 are already used in commit
messages**:

    213.0  1f1ef21   214.0  edd521e   215.0  d5d012e
    216.0  de3469c   217.0  643e270   218.0  4b4af37
    219.0  f7eb4a1   220.0  407336e   221.0  faf699a

**The next free Pass is 222.0.** Once the librarian files those nine (§B), the
tool agrees again and this warning can be deleted. Verify with
`git log --oneline -20 | head` before minting a number.

---

## §0 ACTIVE — the spot-colorant plane, step 1 of ~4 landed and INERT

`Pass 217.0` (`643e270`) added the carrier: `PixelCmyk::s: [f32; MAX_SPOTS]`
with `MAX_SPOTS = 4`, threaded through the compositor, **every value pinned at
zero**. It was proved to change nothing — the conformance sweep is byte-identical
before and after, which is the entire point of landing it separately.

**Nothing else of it exists**: no roster, no plane allocation, no deposit, no
Table 149 spot rules, no collapse.

### Why this is the top item

**Seven of the ten failures the operator can see are this one thing** — `PCS
2.0`, `3.0`, `3.1`, `4.0`, `4.1`, `8.1`, `8.01`. There is no cheap version of
any of them individually.

### The design is already scoped, do not re-derive it

A full scoping study ran this session. Its conclusions, all measured:

- **`spots: Vec<SpotPlane>` from day one, cap defaulting to 4.** Invasiveness
  is *identical* to a fully general n-channel buffer — the cap is one
  comparison — so a bounded first cut costs nothing later.
- **Change surface: 18 composite call sites across 7 files.** `PixelCmyk` is
  confined to `cmyk_buffer.rs` (19 uses) and `compositor.rs` (14).
- **Memory decides the cap, not the census.** 20 B/px today, 36 B/px at four
  spots; a 300 DPI Letter page with four spot planes needs **289 MiB** against
  the 256 MiB default ceiling and would be REFUSED. So planes must be allocated
  from a per-page **roster**, never provisioned to the maximum.
- **Corpus census (4,023 files): 98.6% name no spot colorant at all**, 99.85%
  name three or fewer, maximum seen anywhere is nine.
- **Identity rule: the decoded name BYTE STRING and only that.** §8.6.6.4's
  device test consults only the name; §7.3.5 NOTE 4 makes byte-differing names
  distinct even if they render identically. `/None` never gets a plane; `/All`
  is a **broadcast**, not a plane; `NChannel`'s `/Process` overrides the name
  test. (`Colorant::Named` was changed to `Box<[u8]>` in `Pass 210.0`
  specifically so this rule is implementable.)
- **Smallest first increment: roster + ONE plane, path fills only.** Predicted
  by `docs/suite-patch-reference.md` §3 to fix `PCS 3.0`'s traps at (27,68) and
  (28,135), which are `0 0 0 .5 k` and `.5 g` **fills** over a spot backdrop.

### ★★ TWO HAZARDS THAT WILL NOT ANNOUNCE THEMSELVES

1. **The OPM edition gate flips the moment a fifth plane exists.** ISO 32000-1
   §8.6.7 disables overprint mode if the device space *"is not `DeviceCMYK`"*
   (an identity test); ISO 32000-2 says *"does not include CMYK device
   colourants"* (an inclusion test). **A CMYK+spot buffer is not DeviceCMYK** —
   so under a 1.7 reading, adding one spot plane turns OPM **inert** on that
   page, changing content that has nothing to do with spots. Both readings are
   conformant. This needs its own setting, defaulting to the 2.0 reading so
   today's behaviour is preserved.
2. **§11.7.4.2 is a `shall`:** only separable, white-preserving blend modes for
   spot colours. `Blend::apply_subtractive`'s non-separable arm complements
   exactly three channels and is structurally CMYK-only — it **must not** be
   extended over spot planes.

### One implementation note that is load-bearing

The collapse (spot tint → CMYK via the tint transform) **must not run per
pixel**. A tint is one scalar, so each spot's transform is a 1-D function:
build a 256- or 1024-entry LUT per plane when the roster is fixed. Without it,
an 8.4 Mpx page with four spots is 33.6 M function evaluations at collapse time.

---

## §A OTHER CANDIDATES, ranked by measured exposure

| # | Item | Measured exposure |
|---|---|---|
| 1 | **`PCS 22.1`** — a Lab `L*=60` swatch renders `(35,31,32)` where Acrobat gives `(100,101,100)`. Cause NOT diagnosed; the swatch is a Lab fill with a form XObject and an ExtGState over it, so it is **not** simply the Lab conversion. Independent of everything else. | 1 patch, operator-visible |
| 2 | **Colour-manage ICCBased IMAGES for `/N != 4`** — currently restricted to 4-component sources. See §D why. | blocked on the display conversion |
| 3 | **`PCS 17.2`** — JPEG 2000 with an ICCBased RGB profile; `codestream_space` discards the profile the same way the image path did before `Pass 214.0`. | 1 patch |
| 4 | Make `sh` shadings selectable objects (currently counted only). Needs clip tracking — a `sh` fills the current clip and the decomposer does not track `W`/`W*`. | 0.6% of corpus |
| 5 | Resolve `/OC` layer visibility in the decomposer (currently counted only). Needs the catalog's `/OCProperties` default config, which the walk does not have. | **0** files with a layer OFF |

---

## §B STATE OF THE TREE — verified 2026-09-01

- `HEAD` = `faf699a`, `main` pushed, **0 unpushed**.
- Version **0.19.0**, tag `v0.19.0` at `d19d4e4`, pushed.
- Portable build at `D:\builds\pdfce-20260901-1146-d19d4e4`; CLI published to
  OneDrive slot `pdfce2` (slot `pdfce1` holds 0.17.0 as the previous version).
- **★ NINE code commits are unfiled** — `1f1ef21`, `edd521e`, `d5d012e`,
  `de3469c`, `643e270`, `4b4af37`, `f7eb4a1`, `407336e`, `faf699a` (the same
  nine that hold Passes 213.0–221.0 above). `python tools/check-commits-filed.py`
  lists them. **Dispatch `pdfce-librarian` with each commit's full message
  early** — they carry the measurements, and a one-line subject cannot supply
  them. Filing them also retires the Pass-number warning at the top.
- **Backups are current**: `pdfce-20260901-1855-faf699a-full.bundle`, verified.
  Refresh after the next batch with
  `git bundle create /d/Dev/pdfce-backups/pdfce-<date>-<sha>-full.bundle --all`
  and `git bundle verify`.
- Conformance standing: **7 FAIL / 37 pass / 7 UNRESOLVED of 51**
  (`python tools/suite-check.py D:/Dev/temp/suite-patches --reference-dir
  D:/Dev/temp/acro-refs`).

---

## §C THINGS A NEW SESSION MUST KNOW BEFORE TOUCHING ANYTHING

- **Run `bash tools/run-gates.sh` FOREGROUND, with a warm cache.** Backgrounded
  it gets killed and the SIGTERM (exit 143) looks exactly like a test failure.
  It does not fit one 600 s window; run the pieces if it times out.
- **Never put prose through a Bash heredoc.** Backslashes and backticks are
  eaten silently — it produced a literal `\n` inside a format string this
  session, and a NUL byte inside a Python source file. Use `Write`/`Edit`, and
  `git commit -F <file>`.
- **Stage by path. Never `git add -A`** — the repository is public and agents
  share the working tree.
- **A licensed conformance suite's NAME must never appear in any repo file**,
  contents or filenames. Use opaque ids (`PCS 8.01`). The private map is at
  `D:\Dev\pdfce-private\suite\`. `tools/check-suite-name-absent.py` enforces it
  for the work tree **and, since `Pass 208.0`, for unpushed commit messages**.
- **Check BOTH feature-request channels every session** —
  `D:\Dev\FeatureRequests\pdfce_FeatureRequests\` and `…\iccce_FeatureRequests\`.
  They are outside the repo, so no gate can contradict a stale "it's empty".
- **`docs/core-api/` is engineer-owned and must move in the SAME Pass** as any
  `pub` change to `EditSession`. Run `python tools/check-core-api-verbs.py`.
- Pushing `main` is standing-authorized. **Cutting a tag/release is not** —
  that needs an explicit, current go-ahead each time.

---

## §D ★★ MEASURED NEGATIVES — DO NOT RE-DERIVE THESE

Each cost a full ablation this session. Every one looked obviously right first.

1. **Do NOT colour-manage ICCBased images with `/N != 4`.** Tried; measured
   3× and 1.8× WORSE on two patches (`20.59 → 62.51`, `17.87 → 31.50`), a net
   conformance regression. **Why:** managing an RGB image moves it onto the INK
   path, whose terminal CMYK→sRGB conversion is separately ~10 levels from
   Acrobat. A CMYK image was already ink-bound so the profile is pure gain; an
   RGB image was not, and pays more than it gains. Restricting to 4-component
   sources keeps the win with zero regression.
2. **Do NOT rewire the terminal CMYK→sRGB display conversion to iccce.** Probed
   at all four intents through the document's own `/OutputIntent` profile: best
   case mean error **8.0** against today's **10.3**, and *every* intent clips
   red to 0 where both pdfce and Acrobat show non-zero. `CmykIntent` is already
   on `Calibrated`, the best-evidenced of its two values. The residual ~10-level
   offset is a known, accepted gap — **do not chase it as an ICC-source defect.**
3. **Do NOT extend `Pass 201.0`'s shading `ink_reach` narrowing to images.**
   Tried (`Pass 203.0`, reverted); measured `23.90 → 28.68` — a regression, and
   the marks it targeted did not return. **Why:** a shading sits OVER a thin
   mark, so handing an untouched channel back to the backdrop restores it; a
   photograph COVERS an area, so handing back its untouched channels makes it
   partly transparent to whatever is beneath. *"The same defect in a different
   object type"* was a false premise.
4. **`OverprintZeroTintScope`'s default is NOT "Acrobat's reading"** — measured
   false on process geometry (Acrobat `255,255,255`, this default
   `142,198,63`). It matches Acrobat over a SPOT backdrop and not over process
   components, so *"toward Acrobat"* is a property of the geometry you test on.
   **Do not flip the default alone**: it is trap-neutral (17 → 17) because it
   corrects one cell and breaks another that passes only through a compensating
   error. The honest fix is the literal row assignment **together with** the
   spot plane.

---

## §E ONE ITEM OWED BY THE OPERATOR

**82 of 1,220 commit messages in published history contain the licensed suite's
name in plaintext**, and the repository is public. `ROADMAP.md` open question
`(ca)` records it with three options stated neutrally. Removing them means
rewriting published history, which project rule 8 reserves to Ken and which
this project has direct evidence breaks every document citing a commit hash.

Option 3 — extend the gate so the count cannot grow — is **DONE**
(`Pass 208.0`). Options 1 and 2 are his.

★ File contents in history are **not** affected; a `git grep` hit on
`fixtures/synthetic/ocr/scan.pdf` is a three-byte coincidence in compressed
data, and the gate's exclusion of binaries is documented as deliberate.

---

## §F THE PATTERN THIS SESSION KEPT HITTING, worth carrying forward

Six separate defects had the same shape: **a counter, comment or threshold that
could not see part of its own subject, and therefore reported a different
question from the one its name asked.**

- A trap detector with no reference control, inventing one failure.
- The same detector rejecting a real trap for being **four pixels** too wide.
- A disclosure counter that could see graphics-state paints but not images.
- A gate anchored on a stale key, failing to find its target for several Passes
  while nobody read its output.
- Three comments asserting a spec gap did not exist, while the code beside them
  implemented the gap correctly.
- An object model with twelve diagnostics counters, **none** of which could
  report unmodelled state.

⇒ **When a measurement looks clean, ask what the instrument cannot see.** In
five of the six, the operator's eyes or a reference engine found it and nothing
internal could have. Running a detector against the *reference* — the engine
assumed correct — is the cheapest version of that check and it is now in
`suite-check.py`.
