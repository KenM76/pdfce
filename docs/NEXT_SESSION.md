# NEXT SESSION — start here

Engineer-owned handoff. Read this **before** `ROADMAP.md` — that says what
shipped, this says what to do next. Overwrite it once acted on.

---

## ★ STATE AT HANDOFF — 2026-08-17. THE QUEUE CHANGED; READ THIS FIRST.

**The operator tested the Ghent PDF Output Suite 5.0 X-4 conformance file
and said pdfce was "a long way off from being compatible."** He was right,
and that measurement now drives the queue. Everything below the horizontal
rule further down is the *previous* handoff (OCR, `Pass 71.0`) — **it is
not wrong, it is no longer first.**

`HEAD` = `9839d6f`. **3,747 tests, 0 failures.** `fmt`/`clippy` clean; all
seven scripted gates clean; `cargo tree` shows no GUI dependency in
`pdfce-core` or `pdfce-render`. **Not pushed, not tagged.** Nothing
half-built.

### What shipped today

| commit | what |
|---|---|
| `d0f8c5f` | `Pass 84.0` — twelve `ColorDiagnostics` counters reached a shell for the first time; a false runtime string in `device_n_to_rgb` fixed |
| `016fc31` | the `iccce` request channel named in this file |
| `33ea830` | `Pass 85.0` slice 1 — `pdfce_render::shading`, the model + inventory |
| `9839d6f` | `Pass 85.0` slice 2 — axial and radial shadings **paint** |

### The Ghent gap inventory — this is the queue

Measured from pdfce's own diagnostics on that file, not estimated.

| gap | state | Pass |
|---|---|---|
| shadings, axial + radial | **DONE**, 14/16 of the file | `85.0` |
| shading **patterns** (`PatternType 2`) | not started — **only `sh` paints today** | `85.x` |
| tiling patterns | not started | `85.x` |
| `Separation`/`DeviceN` **images** | not started — **18 images missing across the file, the biggest remaining visual gap** | `85.x` |
| clause-11 transparency (`/BM`, `/SMask`) | not started | `85.x` |
| Type 3 fonts | not started | `Pass 1.1` item 4 |
| overprint | not started; **gated on the Acrobat RAG**, see below | `86.0` |
| `/OutputIntents`, `ICCBased` | **not pdfce's half** — `iccce`, see the channel | — |

### Three things that will bite the next session

1. **`sh` is the ONLY paint route wired.** `crate::shading::Shading::load`
   has exactly one caller (`interpret.rs`, the `sh` handler). `scn` naming
   a pattern still increments `patterns_unpainted` and draws nothing. The
   two routes **anchor to different coordinate spaces** — `sh` is
   CTM-relative, a pattern is base-CTM-relative (§8.7.2 NOTE 1) — so the
   unfinished half is the harder one, not the leftover one.
2. **The GUI's honesty summary regressed, and it is filed rather than
   fixed.** It sums `deferred_ops + unknown_ops`; lifting `sh` out of the
   deferred bucket means a page whose shadings pdfce still *cannot* paint
   (type 1, meshes) now reports clean there. GUI work is operator-paused,
   so this is a known defect the new GUI project inherits.
3. **Mesh shading geometry is NOT in the spec corpus** and the index
   carries an explicit *"do not answer from recall"* marker for it.
   Dispatch `pdfce-spec-librarian` for §8.7.4.5.5–.8 before any mesh Pass.

### The method that earned its keep today — do this again

**The sabotage check caught a hole nothing else could.** Inverting the
greatest-`s` radial selection left **all 17 tests green**, on precisely the
sentence the spec corpus flags as the most-misimplemented in the clause.
Cause: every fixture had only one admissible root, so the choice never
arose. Fixed by building geometry where both roots are admissible *and*
both lie in [0,1].

**A green suite is not evidence until you have seen it go red.** Two
user-visible strings were also caught today only by *reading the actual
output* — a wrong clause number and a capability claim that went false the
moment the feature shipped. No gate can see either.

---


> **[APPENDED 2026-08-13 by `pdfce-librarian`, hundred-and-forty-first
> filing — THE LINE ABOVE IS STALE AND THIS FILE IS ENGINEER-OWNED, so it
> is amended rather than overwritten.]** Since it was written,
> **`ed05033`** landed: **`Pass 71.0` slice 2, the OCR sandwich writer**
> (`pdfce_core::ocr::layer`). **3,667 → 3,688 tests, +21, 0 failures**;
> every gate still clean; `cargo tree` re-verified on `pdfce-core` and
> `pdfce-render`. **Still not pushed, not tagged.** Full record: top of
> `ROADMAP.md`'s *Shipped*, and §1's own amendment below.

> **[APPENDED 2026-08-13 by `pdfce-librarian`, hundred-and-forty-second
> filing — SUPERSEDES THE AMENDMENT DIRECTLY ABOVE ON EVERY FIGURE IT
> RESTATES.]** Two more commits landed:
>
> | commit | what | tests |
> |---|---|---|
> | **`49af8fb`** | **`Pass 71.0` slice 3** — `pdfce_core::ocr::engine_ocrs`, the `ocrs` engine bound to `OcrEngine` behind a **default-ON Cargo feature**; **20 crates added, ZERO copyleft**; `THIRD_PARTY_LICENSES.md` regenerated | 3,688 → **3,690** |
> | **`2fe6216`** | **`Pass 74.0`** — `pdfce_render::render_page_region` (a **NEW Pass family**, minted in the commit; ceiling 73 → **74**) | 3,690 → **3,695** |
>
> **`HEAD` = `2fe6216`, 3,695 tests, 0 failures, every gate clean**
> (slice 3 additionally: `--no-default-features` **build and tests**,
> `cargo check --target wasm32-unknown-unknown`, the no-network denylist
> and the R24 SIMD gate). `cargo tree` re-verified on `pdfce-core` and
> `pdfce-render` at both commits. **Still not pushed, not tagged.**
>
> **Nothing is half-built** — `git status --short` at the head of this
> dispatch showed **no modified source file**, only agent-memory
> scratch under `.claude/`. (This filing then modified five files under
> `docs/`, which are its own output and are uncommitted at the time of
> writing; **the librarian does not commit.**) Full record:
> `ROADMAP.md`'s two top-of-*Shipped* entries and `SESSION_LOG.md`'s
> hundred-and-forty-second filing.

---

## ⇢ ★ FIRST, EVERY SESSION: CHECK THE GUI REQUEST CHANNEL

**`D:\Dev\FeatureRequests\pdfce_FeatureRequests`** — the only channel
between this session and the `pdfceGUI` project (operator instruction,
2026-08-13). **List it before planning anything.**

- `request_*.md` → something the GUI session needs from core or CLI.
  **Triage it into the roadmap** via `pdfce-librarian` so it is tracked
  where everything else is tracked, then rename to `done_*.md`.
- **A request is a FINDING, not a favour.** Decision 058: anything that
  project needs moved in core is *a place the boundary was drawn wrong*.
  A workaround they had to invent is a defect report about `pdfce-core`.
- That folder's `README.md` is the outbound briefing and points at
  `docs/core-api/index.md`. **Keep it current when core changes under
  it** — a stale briefing is worse than none, because it is trusted.

### ★ THERE ARE NOW **TWO** CHANNELS — list both (added 2026-08-17)

**`D:\Dev\FeatureRequests\iccce_FeatureRequests`** — the channel between this
session and **`D:\Dev\iccce\`**, a from-scratch MIT ICC colour management
module whose own `README.md` names pdfce as its first consumer. Created
2026-08-17 at the operator's instruction, same layout and conventions as the
GUI channel so neither has to be learned twice.

**Two differences from the GUI channel, both load-bearing:**

1. **Requests flow BOTH ways.** `iccce` is a library with its own roadmap and
   a standing reason to ask pdfce things — *"what shape does a PDF hand you a
   profile in?"*, *"is this API callable per-pixel?"*. The GUI channel is
   one-way; this one is not. Expect `request_*.md` written by them.
2. **A colour claim must carry its oracle and its number.** `iccce`'s project
   rule 1 is *"a wrong colour looks exactly like a right one"*, and rule 3
   distinguishes ground truth from a cross-check against another
   implementation. pdfce's own house style is looser than that; **inside this
   folder pdfce is bound by theirs.** "The CMYK looks off" is not a finding.
   Note that pdfce's shipped `cmyk_table.rs` is fitted to **pdfium**, which
   makes it a cross-check and not ground truth — say so when quoting it.

**Open at time of writing** (three files, all pdfce → iccce, none blocking
anything on pdfce's critical path): `request_pdf_output_intent_cmyk.md`,
`request_iccbased_colour_spaces.md`, `note_boundary_and_overprint.md`.

**The boundary, because a shared subject is where one side builds the other's
half:** iccce owns *conversion* — profile parsing, transform construction,
intents, ΔE. pdfce owns *compositing* — overprint, blend modes, transparency
groups — and everything about what a PDF's components mean. **Overprint is
pdfce's and is the row most likely to be mis-filed**; it is gated on iccce
only because a CMYK compositing buffer needs a credible CMYK→display
conversion at the end of it.

---

## ⇢ ★ THE STANDING CONSTRAINT — READ THIS BEFORE PLANNING ANYTHING

**The operator paused ALL GUI work on 2026-08-13**, verbatim:

> *"continue the planned work except for gui related, don't do any more
> work on the gui until I say so."*

He gave no reason and was not asked for one. **Do not infer one, and do not
record an inferred one as fact.** It is unambiguous as it stands.

> **[AMENDED 2026-08-13 by `pdfce-librarian`, hundred-and-thirty-seventh
> filing — HE HAS SINCE GIVEN THE REASON, so the two sentences above no
> longer describe the situation.]** The paragraph is left standing rather
> than rewritten (this file is engineer-owned; the librarian appends, it
> does not overwrite). **The reason, from the operator, verbatim and in
> full:**
>
> > *"FYI I paused GUI production in this branch because it was unusable
> > and I realised it needed a separate project plan rather than the
> > current method which just seems to be low priority and a patchwork
> > things stuck together as they are added. The new one is being built in
> > d:\dev\pdfceGUI in another session and if successful will likely
> > replace the current one and may have its dev folder merged into this
> > one."*
>
> **The objection is to the METHOD, not the priority** — so *"a
> well-built new panel"* does not answer it — and **`crates/pdfce-gui`
> may be REPLACED WHOLESALE by a project built outside this repo.** **Do
> not invest in the current shell.** *"Do not infer a reason"* still
> holds for anything beyond his sentence; **nothing about
> `D:\dev\pdfceGUI` beyond it is known here.** The pause itself is
> **unchanged and still standing — a reason is not a lift.** Full record
> and the five engineering consequences: `ROADMAP.md`'s **GUI pause**
> block at the head of *In progress*, and **decision 058** in
> `ARCHITECTURE.md` §12.

- Paused: `crates/pdfce-gui/`, `tools/gui-drive.ps1`, `tools/gui-shot.ps1`.
- Not paused: core, render, CLI, print, docs, RAGs, tests, fuzz, tooling.
- A Pass whose GUI half is deferred ships `core [x] · cli [x] · gui [ ]`
  and the ROADMAP entry records it as an **operator instruction, not an
  engineering shortfall**. `Pass 69.0`/`69.1` are the worked precedent.
- **If he asks for GUI work, he has lifted it.** Do not quote the pause
  back at him.

---

## ⇢ IF THE OPERATOR JUST SAID "CONTINUE"

**★ SUPERSEDED 2026-08-17 — this section used to send you to §1 (OCR).**
It is left standing rather than deleted because §1 is still real work and
still unblocked; it is simply no longer what the operator is watching.

**Take the Ghent queue, in this order**, and the order is measured rather
than assumed:

1. **`Separation`/`DeviceN` images.** The biggest remaining *visual* gap —
   18 images are missing from the operator's own test file, which is why
   the reference images beside the now-working gradients are still blank.
   The §7.10 function evaluator already exists and is already wired for
   vector fills; this is the per-pixel path only.
2. **Shading patterns + tiling patterns.** Finishes what `Pass 85.0`
   started. The pattern-space anchoring is fully sourced in
   `iso32000__s__8.7.md` (PM1–PM9) — the coordinate-space half is the part
   most easily got wrong and it is already written down.
3. **Clause-11 transparency** (`/BM`, `/SMask` in an ExtGState).
4. **Overprint** LAST. It is architectural — compositing into a CMYK
   buffer — and it is the one item genuinely gated on outside work: the
   `iccce` project has to supply a credible CMYK→display conversion before
   a CMYK buffer means anything. See `open/note_boundary_and_overprint.md`
   in the channel.

**Do not ask `(bl)` again** — answered, yes. Do not re-raise the `iccce`
adoption question as an *accuracy* argument either; their own reply
established it would be a lateral move in evidence class (pdfium
cross-check → lcms2 cross-check). The defensible case is **conformance**.

---

## What just shipped, so you do not redo it

`Pass 69.0` **and** `Pass 69.1` — the ce-dimension **style cascade** and
**tolerance** — both **core + CLI complete, GUI deferred**.

| commit | what |
|---|---|
| `d5431a4` | `Pass 69.0` — `dimension::style`, the three-tier per-property cascade + `group-style` / `dimension-style` / `dimension-list --style` |
| `be41d75` | the hundred-and-thirty-fourth filing (decision **056**) |
| `c057682` | `Pass 69.1` — `dimension::tolerance`, seven notation types, as the cascade's tenth property |

The model, in one box:

```text
factory (StyleDefaults::FACTORY) -> group (Group::style)
    -> ce dimension (DimensionRecord::style)
```

**Eleven properties, each an independent `Option` — that `Option` IS the
operator's requested checkbox.** `resolve_style()` is the single resolution
point; `style_provenance()` answers *which tier supplied this*, and
`StyleSource::follows_group()` answers the question a panel actually asks
(*will a group edit move this?*) — **`true` for `Factory` as well as
`Group`**, which is the easy thing to get wrong when deriving it by hand.

### Three things worth carrying, all about tests

1. **★ An ABSENCE assertion on PDF bytes is vacuous under an incremental
   save**, because the superseded object is still in the file. Both
   load-bearing tests here save with `--mode full` for that reason. Filed
   as a lesson in `C:\personal_rag\pdf\`.
2. **★ It COMPOSES with the `Pass 68.0` octal-escaping lesson, and the pair
   met for real here.** The `±` sign is the **second** non-ASCII character
   this writer has ever emitted; the first shipped broken. Get the encoding
   right but the save mode wrong, or the reverse, and the test still cannot
   fail. Both halves, every time.
3. **The sabotage check was run**: the appearance test was **seen to fail**
   (cascade replaced by `From<&Group>`) before being trusted. A cascade
   that resolves correctly in memory and is discarded on the way to the
   document has exactly one symptom — it works in the panel and vanishes in
   the file — and only a test that reads the **baked appearance** sees it.

### One trap the GUI will hit when it is un-paused

**`EditSession::set_group_style` returns the number REGENERATED, not the
number that will visibly MOVE.** Those differ whenever a member overrides
the edited property. The operator's *"cannot change one and be surprised 40
others changed or didn't"* is asking for the second number, and it must be
computed (via `style_provenance` per member) **before** the edit is applied
if it is to be disclosed before the edit is applied. Written up in
`docs/ui_specs/tool-options-dock-and-ce-dimension-properties.md`
**Amendment B**, which also records what else the GUI owes.

---

## 1. ★★ `(bl)` IS ANSWERED — **YES** — AND `Pass 71.0` IS NOW PURE ENGINEERING

> **[APPENDED 2026-08-13 by `pdfce-librarian`, hundred-and-forty-second
> filing — STEPS 1 AND 2 OF THE ORDERED PLAN BELOW ARE DONE. Read this
> before working the list; the list is left standing rather than
> rewritten, because this file is engineer-owned.]**
>
> **DONE — `49af8fb`, `Pass 71.0` slice 3:** step 1 (the Cargo feature,
> **named `ocrs` after the crate rather than `ocr` after the capability**,
> default ON, forwarded from **every** shell including `pdfce-gui`'s
> manifest) and step 2 (`OcrsEngine: OcrEngine` in
> `crates/pdfce-core/src/ocr/engine_ocrs.rs`). **The y-flip was NOT done
> in the engine** — `words_to_page_space` still owns it, exactly as the
> list says. **The wasm32 gate was verified empirically at adoption**,
> with the feature in the default set, rather than taken from the survey.
>
> **★ The one thing the plan could not predict: `reports_confidence()`
> returns `false`, and it is a fact rather than a stub.** `ocrs`'s output
> type is a char and a rectangle — there is no score at any level. **The
> first real implementation of the trait landed on the side a convenience
> default would have got wrong**, which is the argument for
> required-with-no-default made by the world instead of by reasoning.
> Downstream, `OcrLayerReport.confidence_available` is what carries it.
>
> **STILL OWED, and this is the whole remaining Pass, in order:**
>
> 1. **The WEIGHTS — step 3 below, but the open item is the COMMIT, not
>    the licence.** `(bl)` is answered **YES**; **do not re-raise it**.
>    What has **not** been put to the operator is that **~12 MB of
>    `.rten` binary entering a PUBLIC repo's history is permanent.** The
>    engineer raised it in the 2026-08-13 session summary as a heads-up.
>    **Until he answers, the files are neither authorised nor
>    forbidden.** Everything else in step 3 (pin the exact artifact —
>    **the S3 and HF copies are NOT byte-identical** — hash it, hand-author
>    `PROVENANCE.md`) still binds.
> 2. **`pdfce-cli ocr`** (step 4, rule 11). **This is the box that moves
>    first** in `FEATURES.md`; OCR is still `core [ ] cli [ ] gui [ ]`
>    because no build can recognise text and no shell has a surface.
> 3. **The SECOND engine** — `ocr-rs`/PaddleOCR, **Apache-2.0**, **50+
>    languages** (the operator's own stated ranking criterion), **no
>    WASM**, so it is a sibling feature the wasm32 build omits rather than
>    a replacement. The feature naming above is what makes this a drop-in.
>
> Then the rule-4 / decision-059 **off-canvas** review surface (hardest
> case rule 4 has met: it must state that *nothing was scored*) and
> language selection. `docs/PRIOR_ART.md` now carries an **OCR engines**
> table — **Surya is recorded there as REJECTED BY NAME**; do not
> re-evaluate it on accuracy.

**Operator, 2026-08-13, verbatim and in full:** *"yes to the license. keep
going."* A **CC-BY-SA-4.0 model file may ship inside pdfce's MIT portable
folder.** Do **not** re-raise it.

**Four things that answer does NOT cover** — the gap is where a wrong
inference would live:

1. **Not authority to publish or release.** Rule 8 untouched. The repo is
   public, so **a bundled weight file is published the moment it is
   committed**.
2. **Not an engine choice.** That was answered 2026-08-12 (*"just build
   for both"*) → both, behind Cargo features. `(bl)` only removes the
   licence obstacle in front of the pure-Rust one.
3. **Not clearance for an ADAPTATION.** Fine-tuning, quantizing,
   retraining, or converting the weights to another runtime's format
   plausibly creates **Adapted Material**, which must then be
   CC-BY-SA-4.0. *"We'll fine-tune it for CAD drawings later"* is a
   decision with a licence attached and needs **its own** operator answer.
4. **It CREATES an attribution obligation rather than closing one.**
   `cargo-about` builds `THIRD_PARTY_LICENSES.md` from the **Cargo
   dependency graph**; a model file is not a Cargo dependency, so it
   **will not be seen, will not be attributed, and nothing will fail**
   (survey §3.3). `tools/check-shipped-assets.py` is what catches it —
   `PROVENANCE.md` naming the licence, plus a citation in `about.hbs`.

### The engine binding — ordered, and none of it is blocked

Everything below is measured and already on record; **do not re-derive
it**. `docs/ocr-engine-survey.md` is the source for every figure.

1. **Add `ocrs = "0.12.2"` behind a Cargo feature**, following `Pass 70.0`'s
   strippable-capability convention (`fbcb946`): **default ON**, **forwarded
   from EVERY shell**, **both CI gates**, **refuse by name when stripped**.
   Forgetting to forward does not break the build — it removes a capability,
   silently.
   - **Licence work is already done**: 42 packages, **every one permissive,
     zero copyleft** (survey §3.2). Two notes, not blockers — `flatbuffers`
     is **Apache-2.0 only** (no MIT arm), and `unicode-ident`'s
     `AND Unicode-3.0` is already in pdfce's graph today via `syn`.
   - **The `no-network` denylist passes**, verified the way CI runs it.
2. **Implement `pdfce_core::ocr::OcrEngine` for it.** The trait is
   deliberately tiny: 8-bit greyscale in, `RecognizedWord`s in **image
   pixel coords, y-down**, out. **Do not do the y-flip in the engine** —
   `words_to_page_space` owns it, precisely so every engine is wrong or
   right together. `reports_confidence()` has no default on purpose.
   > **[APPENDED 2026-08-13 by `pdfce-librarian`, hundred-and-forty-first
   > filing.] THE WRITER THE ENGINE FEEDS NOW EXISTS — `ed05033`,
   > `pdfce_core::ocr::layer`.** `build_layer_content` (pure) and
   > `add_ocr_layer` (incremental save, additive, the scan never
   > re-encoded, input a byte prefix of output). **Do not write a second
   > one**, and note what this changes about the ordering: after step 2
   > the pipeline is **complete end to end in core**, so steps 3–4 are
   > what make it *reachable*, not what make it *work*.
   > **Two things this slice makes concrete for the engine work:**
   > (a) `OcrLayerReport` is the **off-canvas** disclosure surface
   > decision **059** requires — including `confidence_available: bool`
   > kept **separate** from `mean_confidence`, so an engine reporting
   > nothing cannot look better than one reporting honestly; the trait's
   > `reports_confidence()` is what feeds it.
   > (b) `words_substituted` is counted because the layer **substitutes
   > and discloses** where `add_text` refuses (`R71`, scoped this filing)
   > — **a Standard-14 WinAnsi face cannot represent CJK, Cyrillic, Greek
   > or Arabic**, so a high count on a multi-language document is the
   > signal that an **embedded composite face** is the next axis, not a
   > bug report.
3. **Ship the two `.rten` files and PIN them.** `text-detection` ≈ 2.52 MB
   and `text-rec-checkpoint` ≈ 9.72 MB, **≈12.24 MB total**, from
   `robertknight/ocrs`. **The Hugging Face and S3 copies are NOT
   byte-identical** (13,280 B smaller / 124 B larger, different filenames),
   so **pin exactly which artifact ships and hash it** — "the ocrs models"
   is not one thing. `ocrs-models` has **no LICENSE file**; the CC-BY-SA
   declaration exists only on the HF model card.
   **No network code may enter any pdfce crate** — the download lives in
   `ocrs-cli`, not the `ocrs` library, so pdfce loads from disk via the
   already-shipped `ocr::models::resolve_model_dir` (`af5580e`).
4. **CLI subcommand** (rule 11), and the GUI stays unbuilt under the pause.

**Weights are ~12 MB of binary in a public repo's history, permanently.**
That is worth one sentence to the operator before the commit that adds
them — not a permission request, a heads-up he can act on.

**Still owed from him:** confirm the model-**downloader** withdrawal
(`af5580e`) — he agreed on a wrong estimate, so that agreement was
uninformed.

## 2. `Pass 67.0` phase C — re-subset fonts — the best non-GUI next move

Lowest-risk of the three remaining phases, needs nothing from the operator,
entirely core + CLI. D (text to outlines) is irreversible and needs an
inline disclosure; F (replace font X with Y) has **no Acrobat equivalent**,
so its acceptance criteria are a design question, not a parity one. **Ask
which he wants rather than guessing** — but C is the one to start if he
does not answer.

## 3. Other non-GUI work available

- **Imposition has no GUI** — but the right first step is core anyway:
  extract sheet composition into `pdfce-print` so both shells can share one
  implementation. Unblocked by the pause.
- **`v0.1.0` / `v0.4.0` / `v0.5.0` have no release record.** Measured
  figures are already filed in `ROADMAP.md`'s hundred-and-thirty-third
  filing; backfilling is cheap and needs no operator input. Cause is
  `R192`'s exact shape: `check-commits-filed.py` counts *commits*, each
  version-bump commit *was* filed, so nothing watches for a release with no
  filing.
- **`R192` is PROPOSED, NOT MINTED** — *an obligation that falls between
  two correct tools is enforced by neither.* The engineer's ruling is owed.
- **Two dead/stale printing items** (Backlog, deliberately unfixed):
  `DeviceSettings::pick_tray_by_page_size` sets no `DEVMODE` field;
  `build_devmode`'s doc claims a driver-default start the code does not do.

> **[APPENDED 2026-08-13 by `pdfce-librarian`, hundred-and-forty-second
> filing — TWO NEW BACKLOG ITEMS, both from `Pass 74.0`, both arriving
> with their evidence already measured. Both are core-only, so neither is
> touched by the GUI pause.]**
>
> - **A display-list cache in `pdfce-render`.** Measured at `2fe6216`: a
>   **2-pixel** render costs **691 ms** and a **120,701-pixel** render
>   costs **699 ms**, so **~99 % of the cost is interpretation, not
>   fill**. A reusable parsed representation replayed against N regions
>   takes the **second and subsequent** renders of a page from ~700 ms to
>   tens of ms. **It is the highest-value optimisation this crate has**,
>   and every alternative divides the 1 % — **tiling MULTIPLIES it (9× for
>   a 3 × 3 ring) and parallelism duplicates the floor per worker.** Not
>   started; the `pdfceGUI` project has been told plainly that it does not
>   exist. Positions fixed in **decision 060**.
> - **A `/Rotate` fixture gap.** **No file in `fixtures/synthetic/`
>   carries a `/Rotate` key at all**, so `page_device_geometry`'s four
>   rotation branches have **zero file-level coverage**. Found the worst
>   way: a rotation test written as an integration test **found nothing to
>   load and skipped, reporting success while testing nothing.** Cheap to
>   close (three generated fixtures, 90/180/270, via a
>   `tools/gen-*-fixtures.py` sibling so they are reproducible per rule 7)
>   and it unblocks honest coverage for rendering, region mapping,
>   imposition, print orientation and DXF export at once.

## 4. Deferred BY THE PAUSE — not forgotten, not startable

- The GUI half of `Pass 69.0` + `69.1` (group-tier controls, the
  per-ce-dimension section, the follows-group disclosure).
- `Pass 46` slices 2–4 — post-hoc select/move/resize a placed annotation.
  Gates click-a-comment-to-select and canvas selection of ce dimensions.
- The GUI attachments surface (core + CLI finished in `95c3416`).

## 5. ★ TWO ESCALATIONS STILL AWAITING THE OPERATOR — raise, don't resolve

1. **The broken no-git convention** (`iccce`).
2. **Agents' in-progress files swept into a public repo.**

Carried across five filings now with **no supporting detail supplied**.
**Recorded so a compaction does not lose them — not as established
findings.** Get the actual statement. `af5580e` remains the one measured
instance of (2). `D:\Dev\iccce\` **does** contain a `.git` directory, so
whatever (1) is, it is not "that project has no repository."

The convention it bought is holding: every commit this session was staged
**by name**, `git status --short` run first, and
`tools/render-profile/Cargo.lock` — dirty since before the session — was
left alone in all three.

---

## 6. Release state

**`v0.5.3` is the current tag and is clean** (all seven `verify-release.py`
checks, CI green at the tagged commit). Three commits sit past it,
**unpushed and untagged**. Nothing about `Pass 69.0`/`69.1` requires a
release; the operator's standing authorisation covers **builds for his own
testing**, **not** publishing.

Keep the order: **FILE → LET CI GO GREEN → TAG**, run
`tools/verify-release.py` *before* tagging, and **bump the version before
the tag** (`--version` prints `CARGO_PKG_VERSION`, so tagging a version the
binary does not report ships a false claim where a user checks it).

## 7. Backup — re-measure, do not quote

Measured 2026-08-13 ~07:55:
**`D:\Dev\pdfce-backups\pdfce-20260813-0755.bundle`**, `git bundle verify`
okay, `refs/heads/main` = `6db55ae…` = `HEAD` at handoff, and
`refs/tags/v0.5.3` (`b2d0595…`) is inside it, so the last release is
recoverable from backup alone.

**This ledger has carried a wrong backup figure twice. Re-run `ls -t` and
`git bundle list-heads` — including when the number above is this one.**

> **[APPENDED 2026-08-13 by `pdfce-librarian`, hundred-and-forty-second
> filing — THE BLOCK ABOVE IS STALE, AND ITS OWN LAST SENTENCE IS WHY
> THIS AMENDMENT NAMES ITS COMMANDS.]** Re-measured in this dispatch:
>
> | fact | figure | command |
> |---|---|---|
> | newest bundle | **`D:\Dev\pdfce-backups\pdfce-20260813-post-rebase.bundle`**, mtime **2026-08-13 09:27:53** | `ls -lt --time-style=full-iso D:/Dev/pdfce-backups/` |
> | its `refs/heads/main` | **`dc5a77f`** | `git bundle list-heads <bundle>` |
> | **staleness** | **7 commits behind `HEAD` (`2fe6216`)** | `git rev-list --count dc5a77f..HEAD` |
>
> **`pdfce-20260813-0755.bundle` named above is no longer the newest**,
> and the `v0.5.3` recoverability claim attached to it was not re-checked
> here — **re-verify it against the post-rebase bundle before relying on
> it.** A fresh bundle is cheap and seven commits is not nothing.

---

## Tooling — corrections that cost time

- **★ NEW: a quoted heredoc (`<<'PYEOF'`) through the Bash tool failed
  twice on large Rust/Python payloads**, with `unexpected EOF while looking
  for matching '`. Same content written via the Write tool and run as a
  script worked every time. **For any multi-line patch, write a file and
  run it** — do not fight the heredoc.
- **★ NEW: a `str.replace()` in a patch script with no `assert` is a
  silent no-op.** One tolerance listing line went missing exactly that way
  after `cargo fmt` had reflowed the anchor text. **Assert every anchor.**
- **`cargo fuzz` on this machine needs the MSVC ASan DLL on PATH** or the
  binary dies with `STATUS_DLL_NOT_FOUND`. The path has spaces — set it
  **literally**, do not build it from `find` output (word-splitting turns it
  into `/c`). Filed in `D:\dev\rag\rust\`.
- **`gui-shot.ps1` and `gui-drive.ps1` cannot share coordinates** — two
  independent reasons (different default window sizes; `gui-shot` must run
  on-screen). Moot while the GUI is paused; do not delete the note.
- **The diag script separator is `;`, not `,`.** Click steps are
  `move:`/`down:`/`up:` — there is no `click:`.
- **`git show <sha> -- <path> | grep` SEARCHES THE COMMIT MESSAGE TOO.**
  Use `git diff A^ A -- <path>`.
- **`gh run list --commit <SHORT-SHA>` returns an EMPTY LIST, not an
  error.** Always pass a full 40-char SHA.
- **Resolve every short hash yourself** — librarians have no shell in some
  dispatches; paste real hashes.
- **A gate's DOCSTRING is not the gate** — verify by making the hazard
  occur, in three states (passing, genuinely-failing, correctly-exempt).
- **The CI job's NAME does not name the gate**, demonstrated in both
  directions. Rename or split — small and actionable, not GUI work.

`tools/splice.py` · `tools/check-fmt-excluded.py` (run **beside**
`cargo fmt --all --check`) · `tools/check-shipped-assets.py` ·
`tools/verify-release.py <tag>` — **before** tagging ·
`tools/check-commits-filed.py` — **file the commit; do NOT extend the
baseline** · `tools/check-ledger-numbers.py` · `tools/gen-embed-fixtures.py`
/ `tools/gen-unembed-fixtures.py` · `tools/package-portable.py --note "..."`.

**Live ceilings — re-run `check-ledger-numbers.py`, do not trust this
line.** Measured after the hundred-and-thirty-fifth filing (`d2e614d`),
not forecast: rules **R191** → next free **R192** (**claimed by an unruled
PROPOSAL**) · decisions **057** → next **058** · filings **135** → next
**136** · Pass families to **71** → next **`Pass 72`** · operator questions
**(bl)** → next **(bm)**.

`tools/check-commits-filed.py` at HEAD: **clean**, 393 code commits over
the whole history, 5 known-unfiled carried in the baseline — **that
baseline is DEBT, not an allowlist**, and shortening it is the intended
direction.

**★ Local is 5 commits ahead of `origin/main` (`4069cbb`). `Pass 69.0` and
`Pass 69.1` are NOT pushed.** Stated as a fact, not a request — pushing is
the operator's act and no go-ahead has been given for these.

> **[APPENDED 2026-08-13 by `pdfce-librarian`, hundred-and-forty-second
> filing — BOTH FIGURES ABOVE ARE NOW WRONG, and the ceiling line above
> them is wrong on four ledgers.]** Measured, with the commands, in this
> dispatch:
>
> - **`origin/main` is `42364db`**, not `4069cbb` (`git log -1
>   origin/main`), and it **is** an ancestor of `HEAD`
>   (`git merge-base --is-ancestor origin/main HEAD` → yes).
> - **Local is 22 commits ahead**, not 5
>   (`git rev-list --count origin/main..HEAD`). Still unpushed, still
>   untagged; pushing remains the operator's act.
> - **Live ceilings, from `python tools/check-ledger-numbers.py` run in
>   this dispatch — the line above is measured after the *135th* filing
>   and is stale on four of five:** rules **R191** → next free **R192**
>   (**still claimed by an unruled PROPOSAL; a SECOND unruled proposal now
>   claims `R193`**) · decisions **060** → next **061** · filings **142**
>   → next **143** · **Pass families to 74** → next free family number
>   **75** · operator questions **(bm)** → next **(bn)**.
>
> **Re-run the checker anyway.** The instruction directly above this
> block is right and this amendment does not replace it — it only makes
> the stale numbers non-misleading in the meantime.

**One deliberately-unpatched staleness, so nobody "fixes" it twice:**
Amendment B §2 in the ce-dimension ui-spec says `StyleProvenance::each()`
returns *"nine pairs"* and forecasts *"a tenth"*. At HEAD it returns
**eleven** — tolerance brought two properties, since it has its own
precision slot. The design half of that prediction held exactly; only the
arithmetic was one short, and the paragraph is self-correcting by
construction, which is the very thing it was describing (a fixed-size
array gives a compile error, not a shorter list).
