# NEXT SESSION — start here

Engineer-owned handoff (this filing written by `pdfce-librarian` at the
engineer's explicit request, same as every prior overwrite of this
file). Read this **before** the librarian's record — `ROADMAP.md` says
what shipped, this says what is in flight and what the next hour should
be. Overwrite it once acted on.

**Written 2026-08-12 (hundred-and-twenty-fifth filing), branch `main`, at
`95c34165d8fdd7642ddde1b265aac9681cc63275`**, after four commits
`7825424` · `ee4e1e4` · `74582ca` · `95c3416`. The previous version of
this file was written at `aad48c7` and amended at `b1ee1cf`; **this is a
full overwrite, not an amendment.**

---

## 1. ★★★ THREE NEW OPERATOR REQUESTS (2026-08-12). HE IS WAITING ON THEM. THIS IS THE TOP OF THE QUEUE.

Filed as **`Pass 68.0`** and **`Pass 69.0`**, both under `ROADMAP.md`'s
***Next up***, both **UNSTARTED**. Verbatim, so acceptance criteria stay
faithful:

> **(i)** *"dimensioning tool should allow the selection of two lines. if
> those lines are parallel it makes a linear dimension between them like
> SolidWorks would, if they are at an angle it makes an angle
> dimension."*

> **(ii)** *"groups of dimensions should have a default dimensioning and
> tolerance style that can be set for the group, but these should have a
> checkbox to override and set differently."*

> **(iii)** *"they should have the same options as SolidWorks does for
> dimensions."*

**★ TERMINOLOGY, binding on every dispatch these generate (project
rule 15):** this is all about **ce dimensions** — the ones **pdfce
authors** (`/Line` + `/IT /LineDimension` + `/Measure` + the
`/PieceInfo` sidecar, everything under
`crates/pdfce-core/src/dimension/`). **pdf dimensions** — CAD-exported
ones already in the file — enter only as **pickable page geometry** for
(i), and pdfce still must not alter them. **A subagent handed the
unqualified word writes an entire analysis in it; that is how the
ambiguity reached the operator last time.**

- **`Pass 68.0`** = request (i): a two-line pick model + an **angular**
  third `DimensionKind`.
- **`Pass 69.0`** = requests (ii)+(iii): the ce-dimension **style +
  tolerance** model, per-group default with a per-ce-dimension
  **override checkbox**, at SolidWorks option breadth.
- **They are independent.** `69.0` is the one with a written spec already
  waiting; `68.0` is the one with a hole in the data model. Splitting
  `69.0` into `69.0` (style) + `69.1` (tolerance) is fine.

**★ `Pass 68.0` IS ALREADY UNDERWAY IN THE WORKING TREE — MEASURED, NOT
RELAYED.** `git status --short` at the end of this filing shows
**untracked `crates/pdfce-core/src/vector/linepick.rs`**, plus modified
`crates/pdfce-core/src/vector/mod.rs` (+4) and
`crates/pdfce-core/src/settings/mod.rs` (+84) — **all uncommitted.** The
new module's header states the problem in the same terms the survey below
does and names the same three near-misses independently. **If you are
resuming this session: that work is in flight, not lost. If you are a
fresh session: `git status` before you write a line of it.** This filing
touched **nothing** outside `docs/`.

### The survey that sizes this work — READ IT BEFORE ESTIMATING ANYTHING

Full detail lives in the `Pass 68.0` / `Pass 69.0` entries and in the
hundred-and-twenty-fifth `SESSION_LOG.md` entry. The headlines, all
**re-verified on live source** by the filing librarian:

1. **NO TOLERANCE EXISTS ANYWHERE** — core, GUI, CLI, sidecar. A
   case-insensitive grep for `tolerance` across
   `crates/pdfce-core/src/dimension/` returns **zero hits**. The hits
   elsewhere are **`SnapConfig::tolerance`, an unrelated hit-test
   radius** — do not mistake that for a half-built feature.
2. **NO PER-ce-dimension STYLE.** `DimensionRecord`
   (`dimension/group.rs:419`) is exactly `{ id, group, kind, annot, ap }`.
3. **The only style type is TRANSIENT.** `DimensionStyle`
   (`dimension/author.rs`) is a projection of the group rebuilt at every
   regeneration and carries `{ scale, format, standard }` only. **Line
   weight, text height and arrow length are hard-coded module
   constants** (`LABEL_SIZE 10.0`, `LINE_WIDTH 0.75`, `ARROW_LEN 7.0`,
   `TEXT_BREAK_PAD 3.0`, `TEXT_ABOVE_GAP 3.0` — the last two already
   annotated in-source as *"Convention, not mandated"*).
4. **NO PER-GROUP STYLE OBJECT EITHER.** `Group` (`dimension/group.rs:48`)
   is flat: `{ id, name, scale, format, ocg, visible, standard }`. **No
   colour, no arrowhead, no text height.** The operator's *"default style
   set for the group"* has **no field to set today.**
5. **NOTHING RETURNS "THE LINE YOU CLICKED" AS TWO ENDPOINTS.**
   `snap_candidates` returns single `Point`s. `hit_test_subpaths`
   (`vector/hit.rs:340`) returns subpath **indices**.
   `CenterlineCandidate` (`vector/centerline.rs:42`) is the one existing
   two-endpoint result and only for filled thin quads with **aspect
   ratio ≥ 8.0**. **`MeasureLinear`/`MeasureScale` are strictly
   POINT-based**; `MeasureCircular` picks objects but reduces them to an
   unordered anchor cloud for a Taubin fit. **Request (i) needs a new
   pick primitive, not a tweak.**
6. **`DimensionKind` has exactly two variants** — `Linear`, `Circular`
   (`dimension/group.rs`, enum at line 137). **An angular ce dimension is
   a third variant that does not exist**, with its own geometry, `/AP`
   authoring, `/Measure` semantics (**source it — rule 1, dispatch
   `pdfce-spec-librarian`**) and a sidecar migration.
7. **THERE IS NO RE-MEASURE VERB.** Editing what a placed ce dimension
   measures today means **delete + re-add, losing the id, group and
   placement**. **Decision 023's "dimension re-measure" is
   unimplemented.** You will hit this mid-Pass.
8. **A SPEC ALREADY EXISTS AND IS PARTLY UNBUILT —
   `docs/ui_specs/tool-options-dock-and-ce-dimension-properties.md`.**
   **Build on it; do not re-derive it.** §C.11 is titled *"What already
   exists — read before designing anything new"*. §C.11.1 covers
   group-level-vs-per-dimension **and the inheritance disclosure** —
   the override-checkbox request is **already designed**. It already
   sketches `enum ToleranceType { None, Symmetric, Deviation, Limit }`
   and argues it belongs on **`DimensionRecord`, not `DimensionKind`**
   (right — `DimensionKind` is documented in-source as *"the immutable
   geometry"*). **Item 2 (selection-driven per-ce-dimension surface) HAS
   shipped as `selected_dimension_section`; items 1 (tolerance) and 3
   (extension-line drag) are entirely unimplemented.**
9. **Sidecar migration path is documented — follow it.**
   `SIDECAR_VERSION = 1` (`dimension/sidecar.rs:41`), a **RANGE** version
   gate at line 100, every added key **optional-with-default**, and
   `EditError::SidecarWrittenByNewerBuild` guarding the write side. **A
   ce dimension authored before these Passes must reopen identical.**

### The SolidWorks RAG — named here AND in `Pass 69.0`, so the merge cannot go untracked

**`D:\Dev\Rag-Specialized\SolidWorks_Dimensions\`** — the SolidWorks
dimension/tolerance option catalog grounding request (iii), built
concurrently by a separate agent. **The directory exists and carries an
`index.md`** (verified at filing time).

**Read it before firming `Pass 69.0`'s acceptance criteria**, the same
way `Acrobat_Features` is read before scoping an Acrobat-parity bucket.
It is named in a pdfce doc **deliberately**, per the engineer file's
standing instruction that a RAG deliverable is not handed off until a
pdfce doc names it — the precedent being
`comparison__pdfce_feature_column.md`, which went untracked for exactly
this reason.

**Parity posture:** SolidWorks is the **floor** for the option set, not
the ceiling (user memory *exceed the parity reference when you can*).
Record any deliberate divergence; do not take one silently.

---

## 2. ✅ Open operator question `(bk)` is ANSWERED. DO NOT RE-SURFACE IT.

**The operator answered on 2026-08-12 and chose BOTH options, A and B.**
Both shipped in `74582ca`.

- **A** — `--use-bundled-fonts` stays **opt-in, off by default**; **the
  GUI still does not offer it.** Unchanged, deliberately.
- **B** — when a bundled face is **actually embedded**, pdfce attaches
  the BSD-3-Clause notice to the output as **`FONT-LICENSE-NOTICE.txt`**,
  so the licence travels inside the document. **A run answered entirely
  from `--font-dir` attaches nothing.**

**Ceiling stays `(bk)`, next free `(bl)` — closed, not retired.** The
previous version of this file led with `(bk)` as an open item; that slot
is now closed and this is its replacement.

**Two things worth carrying from how it was built:**

- **An attachment rather than XMP, and the reason was RULE 1, not
  preference.** The spec corpus records **`xmp__* = 0 files`** — writing
  XMP would have meant writing a metadata format from training-data
  recall. **§7.11 is fully sourced (Tables 44–47).** A sourcing boundary
  chose the mechanism, and chose it correctly.
- **The licence text is a verbatim REPRODUCTION, pinned by three tests**
  that diff the shipped notice against
  `crates/pdfce-render/assets/fonts/PROVENANCE.md` **in both
  directions** — trimming the record fails as loudly as editing the
  notice. **A summary does not satisfy a reproduction requirement.**

---

## 3. ★★ `Pass 46` slice 1 SHIPPED today (`7825424`) — SLICES 2–4 ARE NOT BUILT, and slice 2 is the operator's OTHER HALF

**What shipped:** markup is **drawn where you point**, as a real canvas
tool with its options in the side pane. `CanvasTool` gains `Markup`
carrying its kind; `GuiMarkupKind` and `Action::AddMarkupShape` are
**deleted**, not left beside the new path.

**⚠ WHAT DID NOT SHIP — the operator's report had two halves and only one
is answered:**

> *"I tried adding the review tools and they just drop things into the
> center of the pdf window. **I can't drag or resize them.** …"*

- **Slice 2 — post-hoc SELECT / MOVE / RESIZE of an annotation already on
  the page. NOT BUILT.** This is the sentence above, and it is the next
  thing to do on this request.
- **Slice 3 — the remaining six markup kinds. NOT BUILT.**
- **Slice 4 — Family B reshape. NOT BUILT.**

**Do not read "markup is drawn where you point" as "the markup request is
done."** Spec: `docs/ui_specs/pass-46-canvas-interaction-model.md`.

**Two findings from slice 1 worth keeping in hand:**

1. **`drag_started()` does not fire on the frame of the press.** It fires
   once the pointer has moved far enough to count as a drag, so
   `interact_pointer_pos()` reports a position **already travelled to**.
   A drag that should have spanned **50.5 pt** produced **42.0**.
   **`press_origin()` is the press itself.** `run_place_field_tool` had
   the identical pattern and the identical offset — **fixed too, and it
   was never reported only because a form field's exact corner is less
   scrutinised than a drawn shape's.**
2. **A count-only test passed for the whole life of that defect.** One
   shape was added, one shape existed; the bug was entirely *where*.
   **Assert coordinates, and read the baseline from the fixture.**

---

## 4. The GUI has NO attachments surface at all

`95c3416` finished the capability in **core and CLI only**:
`EditSession::attach_file` / `detach_file`, and CLI
`extract-attachment` / `attach-file` / `detach-file`. Verified end to end
on the release binary (attach → list → extract → detach, bytes
byte-identical).

**`core [x] cli [x] gui [ ]` — and the `gui` box is deliberately not
rounded up in `FEATURES.md`.** There is no attachments panel, no menu
entry, nothing.

Three properties of that surface a GUI slice must preserve:

- **`extract-attachment` REQUIRES an explicit output path and never
  derives one from the attachment's own name.** That name is
  **attacker-controlled** and ISO 32000-1 constrains nothing about it —
  `..`, NUL, reserved device names, RTL overrides rendering `gnp.exe` as
  `exe.png`. **A GUI that defaulted the save name to the stored name
  would reintroduce the path-traversal primitive the CLI refuses to be.**
- **`detach-file` is NOT a redaction.** Incremental save keeps every
  prior revision **by design**, so the bytes stay recoverable; `--mode
  full` is the answer, and the GUI must say so too (rule 4).
- **A multi-node (`/Kids`) `/EmbeddedFiles` name tree is refused by name**
  by both verbs, because a wrong `/Limits` repair breaks the attachments
  **already in the document**. Surface the refusal with its reason; do
  not hide the file.

---

## 5. `/R` 6 encryption — still parked at the operator's explicit instruction

Encryption stays parked (*"put the encryption aside for now to work on
later"*). `/R` 6 remains the **only read-side gap**, and it is exactly
one function: `crates/pdfce-core/src/crypto/r5.rs`, private `fn hash` —
Algorithm 2.B's substitution point. Everything around it (`/O`//`U`
layout, `/UE`//`OE` unwrap, `/Perms` check, the calling harness) is
implemented and tested.

**That is precisely the situation where filling it from memory is most
tempting and least detectable.** The refusal fixture
(`enc-aes-256-r6.pdf`) and the refusal tests exist to make that hard —
**do not remove or weaken either to "make progress."**

**The sourcing blocker is gone.** ISO 32000-2, supplied by the operator
2026-08-12:

```
D:\Dev\Rag-Specialized\PDF_Spec\_sources\ISO_32000-2_sponsored_EC3.pdf
```

19,203,156 B, 1,023 pages, PDF Association sponsored release, Errata
Collection 3 (2026-06-01). Registered in that corpus's `LEGAL_NOTE.md`
with `license_basis: licensed_primary_private_rag`.

**⚠ LICENCE, BINDING.** The copy is **watermarked to the operator by
name** and states **"Single user only, copying and networking
prohibited."** It must **never** be committed to pdfce, **never**
shipped, **never** a release asset, and never copied out of `_sources\`.
**Paraphrase and cite only** — a clause number and a restatement, never
the clause text. **The repository is public** (`CLAUDE.md` rule 8):
anything committed here is published by default, so this is not a
hypothetical exposure. Route spec questions through
`pdfce-spec-librarian`, which owns that corpus.

**Also still unstarted:** **writing** an encrypted document, in every
configuration and every shell.

---

## 6. ★ TWO ESCALATIONS STILL AWAITING THE OPERATOR — raise them, don't resolve them

1. **The broken no-git convention** (`iccce`).
2. **Agents' in-progress files swept into a public repo.**

**Both are carried from the engineer's context with no supporting detail,
and the filing librarian could find NO written record of either anywhere
in `docs/`.** They are recorded here so a compaction does not lose them —
**not** as established findings. **The exact content of both must come
from the operator or the engineer.**

One check that bears on the first: **`D:\Dev\iccce\` DOES contain a
`.git` directory** (verified by `ls` at filing time), so whatever the
claim is, it is **not** "that project has no repository." Get the actual
statement before acting.

---

## 7. ★ TAKE A BACKUP — 7 commits owed, and this figure was MEASURED, not inherited

**Measured at filing time**, by `git bundle list-heads` +
`git rev-list --count`, not by reading any document:

- Newest bundle: **`pdfce-20260812-1100.bundle`**, whose `refs/heads/main`
  is **`68408f18980fa2bfd61d81f4627b59a173d8c0a9`**.
- `git rev-list --count 68408f1..HEAD` = **7**.
- `HEAD` = **`95c34165d8fdd7642ddde1b265aac9681cc63275`**.
- `git remote -v` = `https://github.com/KenM76/pdfce.git`.

**This ledger has carried a WRONG backup figure twice.** `ls` and
`git bundle list-heads` cost nothing. **Re-run them; do not quote the
number above without re-running it, including when the number above is
this one.**

---

## 8. Carried forward, unchanged in substance

- **`Pass 67.0` phases C, D and F** — all unstarted, none blocking.
  Three of six shipped (A reporting, B unembed, E embed-missing); the
  request that opened the family is answered. **Ask the operator which
  (if any) he wants next rather than guessing an order.** C = re-subset
  (lowest risk, no visual change, works where B must refuse). D = convert
  text to outlines (the universal escape hatch; irreversible — disclose
  the cost inline, not merely via a confirm button). F = replace font X
  with Y (hardest; Acrobat has **no equivalent** — parity-plus).
  Reusable substrate already built:
  `FontEnvironment::resolve_for_embedding`'s four-rung donor ladder,
  `fontinfo::Removability`'s nine-verdict classifier, and
  `font_embed_missing.rs`/`font_unembed.rs`'s shared-descriptor
  reachability code.
- **The CI job's NAME does not name the gate that fails.** Every red X
  renders as **`verify pdfce-gui strings live in ui_text.rs`**, but that
  job (`.github/workflows/ci.yml:257–322`) runs **three** unrelated gate
  steps and **in all five red runs examined the failing step was the
  THIRD** (`check-commits-filed.py`). Rename or split it so a red X names
  its own cause. Small and actionable.
- Two dead/stale printing items, filed to Backlog, deliberately not
  fixed: `DeviceSettings::pick_tray_by_page_size` sets no `DEVMODE` field
  at all; `build_devmode`'s doc claims a driver-default start the code
  does not do.
- **Imposition has no GUI** — extract sheet composition into
  `pdfce-print` first so both shells share one implementation.
- Static hybrid XFA read/fill · wide-shape CSV · colour management
  (`D:\Dev\iccce\`, planned, no code).
- **Ledger-accuracy defect, still not fixed:** filings ninety-two through
  ninety-five cite `(bh)`/`(bi)` as if `(bi)` had not been minted.
- **Spec-librarian flag, still open:** confirm the eight-item
  never-encrypted list (E1–E9) is in the §7.6 corpus rather than only in
  pdfce's code.
- **`CLAUDE.md` rule 8's literal per-release wording is stale** against
  the operator's 2026-08-11 standing release authorisation — flagged
  across several filings, not yet amended by him; **not the librarian's
  or the engineer's file to edit.**

---

## Release state — `v0.5.1`

Tag `v0.5.1` → `aad48c73…`. **Its CI run is RED on
`check-commits-filed` only** — three commits were tagged and released
before the librarian filed them. **The released CODE is fine and that is
proven:** `git diff --name-only aad48c7 68408f1` returns **only `docs/`
paths**, and `68408f1` passed CI fully. **Binaries fine, ordering wrong.**

**★ THE ORDERING RULE, now enforced by the tool: FILE, LET CI GO GREEN,
THEN TAG.** Run `tools/verify-release.py <tag>` **before** tagging.
History: **3 of the last 4 releases (75%) were tagged at a commit CI had
rejected** — `v0.5.1` and `v0.5.0` on the filing gate only, `v0.4.0`
green, and **`v0.3.0` on `cargo test` + `cargo clippy` + the
`aarch64-apple-darwin` cross-check simultaneously — a published release
whose tests did not pass.** Those jobs run on Linux/macOS/wasm while
local verification happens on Windows: **cross-platform breakage is
invisible to local gate runs by construction.**

**The version bump comes BEFORE the tag, deliberately** — `--version`
prints `CARGO_PKG_VERSION`, so tagging a version the binary does not
report would ship a false claim in the one place a user checks it.

**Standing release authorisation is in force** (operator, 2026-08-11:
*"please continue to post the latest versions to git so I can try them on
my laptop at home"*): build, tag, publish the asset, run
`tools/verify-release.py`, report what went out. **Scope is narrow** —
pdfce builds for the operator's own testing. **NOT** blanket publishing
authority, **NOT** licence to treat repository visibility as an agent's
decision, **NOT** permission to skip verification.

---

## Tooling — corrections that cost time in prior sessions

- **★ NEW, and it was costing the operator his mouse:** `gui-shot.ps1`
  and `gui-drive.ps1` **used to leak a live `pdfce-gui` process** on any
  non-happy path — the kill was on the last line under
  `ErrorActionPreference='Stop'`. **The leaked window parks OFF-SCREEN at
  the caller's viewport, so it takes pointer input while showing
  nothing.** Fixed in `ee4e1e4`: `try/finally`, pre-launch PID snapshot
  so it can never kill an instance the operator opened himself, a
  **verified** kill with a 5 s poll, and a printed `taskkill` line if a
  process will not die. **If the operator ever reports a fighting mouse
  again, this is no longer the explanation — go looking elsewhere.**
- **`observe-gui.ps1` had `try/finally` around bitmap disposal only, not
  the process** — the same one-sibling-hardened shape as the `fmt` gate.
  **When you find a guard on one sibling, check what it actually wraps
  before crediting the others with it.**
- **`PDFCE_DIAG_VIEWPORT`**, not `PDFCE_VIEWPORT`. Four comma-separated
  numbers: `x,y,w,h`.
- **The diag script separator is `;`, not `,`.** A comma-separated script
  parses as ONE unparseable step and is silently skipped — the trace says
  `script-step-UNPARSEABLE`, read it. **`tool:markup` is now in the
  vocabulary** (added by `7825424`).
- **`gui-shot.ps1` and `gui-drive.ps1` default to different window
  sizes.** Read the trace's own `rect=`, never a screenshot's pixels.
- **Both scripts move the REAL cursor** and synthesise Ctrl+scroll and
  click-drag on the live desktop. Say so before running one while the
  operator is at the machine; prefer headless verification when it will
  do.
- **A GUI control's traced rect describes the layout *request*, not what
  survived clipping.** A control whose traced rect is wider than a
  sibling's in the same dock is the one to click-test first.
- **`gh run list --commit <SHORT-SHA>` returns an EMPTY LIST, not an
  error.** **Always pass a full 40-character SHA** (`git rev-parse
  <ref>`). An empty result from a query that silently rejects the input
  *form* is indistinguishable from "no runs exist" — `R191`'s shape.
- **Resolve every short hash yourself with `git rev-parse`.** A
  fabricated full hash reached a filing once already and had to be
  corrected. The command costs nothing.

`tools/splice.py` — anchored substitution, all-or-nothing ·
`tools/check-fmt-excluded.py` (no arguments; the fmt gate for the 12
crates `cargo fmt --all` cannot see — run it **beside** `cargo fmt --all
--check`, never instead of) ·
`tools/verify-release.py <tag>` — **run it BEFORE tagging; it consults
CI** · `tools/check-commits-filed.py` — **run it before assuming a
dispatch listed every unfiled commit; it found one this session that a
dispatch did not name.** File the commit, **do not** add it to
`tools/commits-filed-baseline.txt` (a baseline entry suppresses the gate,
a filing satisfies it) · `tools/check-ledger-numbers.py` — the live
ceilings for Pass IDs, standing rules, decision records and filing
ordinals · `tools/gen-embed-fixtures.py` /
`tools/gen-unembed-fixtures.py` (no arguments needed) ·
`tools/package-portable.py --note "..."`.

**Live ceilings at this filing** (by `check-ledger-numbers.py`): standing
rules **R191** → next free **R192** · decision records **054** → next free
**055** · SESSION_LOG filings **125** → next free **126** · **Pass
families up to 69** → next free **`Pass 70`** · operator questions
**(bk)** → next free **(bl)**.
