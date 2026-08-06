# pdfce — Feature status

**Deliberately concise — this is a scan, not a record.** `ROADMAP.md` is
organized by Pass (engineering order) and carries the full reasoning,
acceptance criteria, and history; a single capability here is often
built across several Passes there. This file answers one question only
— *what can pdfce do, and what's missing* — and a features list nobody
can scan in about two minutes has failed its job. Don't "fix" this file
by expanding it; if a row needs an argument, that argument belongs in
`ROADMAP.md`, not here.

**Maintained by `pdfce-librarian`, in the same filing as every
`ROADMAP.md` "Pass shipped" update** — when a Pass moves to Shipped,
its row(s) here get ticked in the same edit. Not a separate chore.
**`ROADMAP.md` is the source of truth; if this file and `ROADMAP.md`
ever disagree, `ROADMAP.md` wins** and this file is stale until the
next filing.

## Legend

- `[x]` — built and shipped.
- `[ ]` — not yet built.
- `—` — **not applicable**, not "not built." A row marked `—` in the
  `cli` column means the capability has no sensible one-shot batch
  form (e.g. "pan the canvas"); a row marked `[ ]` in the `gui` column
  means it genuinely has no GUI surface yet and is reachable only from
  `pdfce-cli` today. Confusing these two is exactly the mistake this
  legend exists to prevent — a `[ ]` is a gap, a `—` is a shape
  mismatch.
- **core** = `pdfce-core`/`pdfce-render` (headless, no GUI deps).
  **cli** = `pdfce-cli`. **gui** = `pdfce-gui`.
- **ce dimension** = a `/Line`+`/IT /LineDimension` annotation **pdfce
  itself authors** (scale, group, `/Measure`, `/PieceInfo` sidecar).
  **pdf dimension** = a dimension already present in the file from CAD
  or another tool — pdfce reads and measures against it but never
  authors or alters it. See `CLAUDE.md` rule 15; never read "dimension"
  below as the other kind.
- A handful of `pdfce-cli --help` subcommands (`to-pdfa`,
  `validate-pdfa`, `sign`, `bates-stamp`) are **Pass-0 stubs** that
  print "not implemented, later Pass" — they exist in `--help` but are
  not a shipped capability. Not ticked anywhere below; the features
  they represent are listed under *Planned*.

### The `Acrobat` column

Does **Adobe Acrobat Pro** (current 2026 subscription release) have this
**capability**? Never how its UI exposes it. Four states plus a partial,
and they are not interchangeable:

- `[x]` — Acrobat has it. `[ ]` — **verified absence**, not "we didn't
  find it." `◐` — Acrobat has only **part** of what the row names; the
  RAG file below says which part. `—` — **not meaningful to ask** (a
  pdfce-internal concept, or GUI mechanics the Acrobat RAG deliberately
  never catalogues). `?` — **not looked up yet.** Never read `?` as
  "Acrobat lacks it"; that is the `[ ]` state and only the `[ ]` state.
- **Authority, and the per-row basis this one-character column cannot
  carry:**
  `D:\Dev\Rag-Specialized\Acrobat_Features\comparison__pdfce_feature_column.md`
  (`last_verified: 2026-08-05`).
- **The verdicts are NOT uniformly confident, and must not be read as
  if they were.** Redaction, Forms, core page ops, Text and the
  measuring tools rest on dated `Acrobat_Features` files — trustworthy
  at `must_have` acceptance-criteria grade. **Digital signatures,
  encryption, Bates, OCR, accessibility, comparison, portfolios,
  optimization, PDF/A and prepress rest on a single 2026-08-05 web
  search** — provisional; re-verify before any acceptance criterion
  leans on them. Those ten buckets have **zero** dedicated RAG files
  (filed as backlog in `ROADMAP.md`). A few `[x]` are high-confidence
  inferences or carry material caveats — Acrobat's word-spacing and
  baseline-shift controls were removed from its own UI; its cross-block
  reflow is cloud-gated and did not exist before 2021 — and the RAG
  file carries those per row.

## Implemented

### Document & pages

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [x] | [x] | [x] | [x] | Open, parse, and render a PDF (Pass 1) |
| [x] | [x] | [x] | [x] | Recovers a damaged/malformed cross-reference table; tolerates common lenient-PDF defects (Pass 13a/13b, `/Contents`-defect fix) |
| [x] | [x] | [x] | [x] | Rotate pages, incl. refusal disclosure (Pass 3.2 core/CLI; GUI rotate buttons + `5b2682b` disclosure fix) |
| [x] | [x] | [x] | [x] | Delete pages (Pass 3.2 core/CLI; GUI `delete_pages` call in `main.rs`) |
| [x] | [x] | [x] | [x] | Reorder pages — thumbnail drag or keyboard (Pass 3.2 core/CLI; GUI `apply_reorder`/`move_pages_keyboard` in `main.rs`) |
| [x] | [x] | [x] | [x] | Merge / combine multiple files into one (Pass 3.2 core/CLI; GUI `merge_tool` add-files button in `main.rs`) |
| [x] | [x] | [x] | [x] | Split a document into multiple files, `EveryN` criterion only (Pass 3.2 core/CLI; Pass 3.4 GUI) — break-point and bookmark criteria still unbuilt, named in the panel |
| [x] | [x] | [x] | [x] | Insert pages from another file — whole source, Start/End only (Pass 3.2 core/CLI; Pass 3.5 GUI) — writes a NEW FILE, not an in-place edit; true in-place insert needs `EditSession::insert_pages` (core-only, unbuilt — see *Planned*) |
| [x] | [x] | [x] | [x] | Extract pages to a new file (Pass 3.2 core/CLI; GUI `PdfceApp::extract_selection`, `main.rs` ~L3601, reached via `Action::ExtractSelection` from a thumbnail-rail button ~L8936 — the GUI names it "extract selection," not "extract pages," which is why a name-based grep for `extract_pages` missed it; see R156) |
| [x] | [x] | [x] | [x] | Edit document metadata (`/Info` dict) (Pass 3.2 `set-info`; GUI Properties dock panel) |
| [x] | — | [x] | [x] | Undo/redo command log (Pass 3.1 core; wired into the GUI at Pass 14.3) |
| [x] | [x] | [x] | [x] | Save (incremental-by-default or full-rewrite); **"Save a copy" only — no true in-place overwrite yet** (see *Planned*) |

### Text

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [x] | [x] | [x] | [x] | Extract & copy text: search index, `ToUnicode`, reading order, plus page/document-scope clipboard copy (Pass 4 core+CLI `extract-text`; GUI Copy ▾ menu, `CopyScope::Page`/`Document`) |
| [x] | — | [ ] | [x] | Select text on the canvas and copy the selection — the selection itself exists (Edit Text tool: triple-click, `text_caret_after_click`, a live caret/anchor pair), but there is no `CopyScope::Selection` and no copy verb reaches it. `cli` marked — (not `[ ]`): "mouse-select a range, then copy" has no batch shape; page/document-scope copy above already covers the CLI case |
| [x] | [x] | [x] | [x] | In-place text editing of existing runs (Pass 14.x) — arrow-key caret navigation fixed `7d368e6`: the canvas claims the arrows via `set_focus_lock_filter` instead of letting egui steal focus leftward |
| [x] | [x] | [x] | ◐ | Text formatting: size/colour/font-family-style, char/word spacing, horizontal scale, super/subscript, synthetic bold/italic (Pass 19.x, FF-H — complete end-to-end) |
| [x] | [x] | [x] | [x] | Within-block reflow incl. justified alignment (Pass 15.x, FF-A) — open defect: auto-detected wrap width can inherit a prior edit's overflow (Pass 33.0, disclosed but not yet fixed) |
| [x] | [x] | [x] | [x] | Add new page text: point insert + wrapped multi-line box (Pass 16.x, FF-D) |
| [x] | [x] | [x] | [x] | Edit composite/CID (`/Type0`) text runs (Pass 29.0) — the GUI's Edit-Text commit path (`commit_text_edit_draft`) calls `EditSession::edit_text` directly with no composite gate of its own, so it inherited the capability the moment core lifted the refusal; no GUI-side wiring was needed (see R156). **Exception:** word spacing (`Tw`) on a composite run is still refused (R91) — that gate is real and distinct from text-edit itself |
| [x] | [x] | [x] | **[ ]** | Add non-Latin text via a subsetted, embedded donor font (Pass 21.0 core/CLI; Pass 37.1 GUI — face picker in Add Text, supplied faces listed first, refusal-with-remedy, measured embed disclosure). glyf/TrueType donors only; **no shaping ever (R17)** — CJK/Cyrillic/Greek/Hebrew are right, Arabic/Devanagari/Thai embed and read WRONG. `fsType` licence disclosure (R109) still owed; applying a donor to EXISTING text still unbuilt (Pass 21.2/21.3) |

### Vector objects (Inkscape-style editing)

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [x] | [x] | [x] | [x] | Select vector objects: click, marquee, Alt+click cycle through overlapping hits (Pass 9a, 12.0, 18.5); **additive multi-select** under Shift **or** Ctrl/Cmd, for both click and marquee, and in the Obj tool (`7d3e44c` — Shift worked since Pass 9a, Ctrl was never consulted). **Verified in the running app** (`9328038`: `newsel=1` → `newsel=2` on Ctrl+click) |
| [x] | [x] | [x] | [x] | Move / delete a whole vector object (Pass 25.x, 28.0) |
| [x] | [x] | [x] | **[ ]** | Enter an object and select one part (subpath) (Pass 25.0–25.4) |
| [x] | [x] | [x] | **[ ]** | Move / delete a subpath (Pass 28.0 core, Pass 25.2 delete, Pass 36.0 GUI gesture) |
| [x] | [x] | [x] | **[ ]** | Move a node (anchor point), incl. `re` rectangle corners and reused subpath starts (Pass 26.0, 30.0) |
| [x] | [x] | [x] | **[ ]** | Delete a node (Pass 36.1) |
| [x] | [x] | [x] | **[ ]** | Edit a Bézier handle / control point (Pass 30.1 core+CLI; Pass 26.1 GUI) |
| — | — | [x] | **[ ]** | Level-ladder rung readout (Object/Part/Point) + Escape-to-ascend navigation (Pass 26.0, 36.2) — no clickable breadcrumb yet |
| — | — | [x] | — | Level survival across an edit: an edit/undo/redo truncates your rung one step at a time instead of ejecting you to Page (Pass 26.2). `Acrobat` `—`: Acrobat has no level ladder, so the question is not meaningful. Ladder proven by unit test; the entry gesture is **not** driven end-to-end |

### ce dimensions

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [x] | [x] | [x] | ◐ | Author a ce dimension with snapping: linear/radius/diameter (Pass 12.M1, 12.M2, 12.M2b) |
| [x] | [x] | [x] | **[ ]** | ce-dimension groups: scale, number format (decimal/fraction), ANSI/ISO drafting standard (Pass 12.M2, 27.2; GUI controls in the Properties dock compartment since Pass 34.2, **always visible since Pass 38.2**) — group-wide: a change regenerates every member's `/AP` |
| [x] | [x] | [x] | **[ ]** | Change a PLACED circular ce dimension between the radius and diameter reading (Pass 34.2, `EditSession::set_dimension_display` / `pdfce-cli dimension-display`) — **circular only**; a linear target is refused by name (`EditError::NotACircularDimension`). Per-object: regenerates exactly one `/AP` |
| [x] | [x] | [x] | [x] | Drag to reposition a ce dimension (Pass 25.5) |
| [x] | [x] | [x] | ? | Edit a placed ce dimension's placement NUMERICALLY — standoff / value-position (Pass 27.1 core+CLI `dimension-offset`; Pass 34.2 GUI spinners, which mirror the drag rather than replace it) |
| [x] | [x] | [x] | [x] | Toggle a ce-dimension group's OCG layer visibility (Properties dock compartment since Pass 34.2, **always visible since Pass 38.2** — the floating Group Manager window is gone, closing R81's last named holdout) |
| [x] | [x] | [x] | [x] | Delete a ce dimension: annotation + `/AP` + `/PieceInfo` sidecar, together (Pass 25.6) |

### Annotations & markup

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [x] | [x] | [x] | [x] | Render & count annotations/widget appearances, honouring the annotation-flag set (Pass 6.0) |
| [x] | [x] | [x] | [x] | Author geometric markup: Ink/Square/Circle/Line/Polygon (Pass 6.1) — minimal menu affordance only; no canvas drag/freehand drawing yet |
| [x] | [x] | [x] | [x] | Author text-bearing annotations: FreeText/Stamp with §12.7.3.3 variable-text appearance (Pass 6.2) |

### Forms (AcroForm)

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [x] | [x] | [x] | [x] | Fill form fields — core/CLI: text, checkbox, radio, choice; **GUI: text + checkbox only** (Pass 7.0, 7.1 core/CLI; Pass 37.2 GUI Forms panel, P0) — radio groups and choice fields are recognised, listed, and disabled-with-a-reason (P1), as are read-only, signature and pushbutton fields. **Rich-text fields are REFUSED in the GUI** (filling would silently discard stored formatting) — core/CLI do NOT yet refuse; see `ROADMAP.md` *Next up* |
| [x] | [x] | [ ] | [x] | Flatten a form to static page content (Pass 7.1) — GUI is Pass 37.2's P1, not built |
| [x] | [x] | [ ] | [x] | Import/export form data, FDF/XFDF (Pass 7.1) — GUI (Batch Tools) is Pass 37.2's P1, not built |

### Redaction & security

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [x] | [x] | [x] | [x] | Mark redactions by text search or named region (Pass 8.0, 8.1) |
| [x] | [x] | [x] | [x] | Mark redactions by PATTERN — `#` = any digit, `?` = any character, so `###-##-####` marks every SSN-shaped run in one action (Pass 8 core/CLI `redact-mark --pattern`; Pass 37.0 GUI `Match: Exact text \| Pattern` switch over the existing query box) |
| [x] | [x] | [x] | ◐ | Apply redaction with a runtime-verified true-removal proof (Pass 8.0, 8.1) — Acrobat applies, but gives the operator no automated proof it removed what it claims to |

### Fonts & rendering

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [x] | — | [x] | [x] | Rasterize page content: vector paths, text, images (Pass 1, 1.1) |
| [x] | — | [x] | [x] | Image codecs: DCT/JPEG, LZW, RunLength, CCITT Fax, JBIG2, JPEG 2000 (Pass 2.1–2.3) |
| [x] | [x] | [x] | — | Glyph-coverage gate on the app's own UI chrome — no tofu glyphs in pdfce's own strings (Pass 18.7) |

### Shell & UX

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| — | [x] | — | **[ ]** | A first-class scriptable CLI (`pdfce-cli`) over the capabilities above — Acrobat has no CLI at all (Action Wizard, embedded JS and COM only) |
| [x] | [x] | [x] | — | Live-edit canvas: renders the edited revision, not a static page image (Pass 17.x) |
| — | — | [x] | — | Interactive canvas: pan, zoom-to-cursor, marquee select (Pass 12.0, 18.8) |
| — | — | [x] | — | Dockable panel shell — **four always-visible left compartments, no tab bar**: Pages · Tool (armed tool's options) · Properties · Activities (Batch Tools / Redact / Forms behind a segmented control) (Pass 18.1, 34.1, 34.2, **38.2**). Properties still holds the same three scope-named sections (selected ce dimension · ce-dimension groups · document `/Info`) and is now **always on screen**; no persistent floating window remains (R81). Rule: **watched state gets a compartment, entered workflows share one** (R157) |
| — | — | [x] | — | Ribbon command surface — File/Edit/Review/Measure/Tools/View tabs, reset-layout chooser covering both docks (Pass 24.1, chain `6449859`→`2b12efe`) |
| — | — | [x] | — | Implicit gesture-commit: clicking away accepts an in-progress edit instead of requiring a separate Accept/Reject click (Pass 34.0) |
| — | — | [x] | — | **Nothing floats over the canvas** — every commit control, refusal and disclosure (Edit Text · Add Text · Measure) lives in the tool's dock compartment; `canvas::tool_strip_anchor`/`StripCorner` deleted (Pass 34.1 slice 4, `7f850c9`). ⚠ **Shipped but NOT yet filed in `ROADMAP.md`** — see the *FILING GAP #2* flag at the head of *Shipped* |
| [x] | [x] | [x] | — | ScripTree-style SVG icon set for every GUI feature (Pass 18.3) |

## Planned, in predicted order

**Order is a best-effort derivation from `ROADMAP.md`'s *Next up*
(closest first), then *Backlog*, respecting the operator's own ★★★
priority sequence and stated demand rankings — it is not a promised
build order, and everything past the first several rows is this
librarian's judgment call, not the operator's.** All boxes below are
unticked unless noted; a `[x]` in *Planned* means "the underlying
model/verb already exists — only the named shell is missing," which is
worth knowing when scoping the work. **The `Acrobat` column is not a
pdfce box** — it says whether Acrobat already has the thing we plan to
build.

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| — | — | [ ] | — | Pass 38.1 — shell-redesign slice 1: the density convention (`UI_PREFERENCES.md` §11), applied to the existing panels. **Skipped when 38.2 shipped out of order; still owed.** (Pass 38.3 is the property-4 confirmation — an operator question, not a capability, so it has no row here) |
| — | — | [ ] | ? | Pass 38.4 — the Comments / annotation-list panel. **Blocked on core:** `annot::Annotation` models no `/Contents`/`/T`/`/M`. Honest ceiling: Pass 6.1 markup never sets `/Contents`, so v1 shows mostly untitled rows |
| [ ] | [ ] | [ ] | — | Pass 38.5 — shell-redesign P1: general `delete_annotation` core verb + the Delete row action; `pdfce-cli list-annotations` `contents=`/`author=`; `ResetScope` also resetting per-compartment collapse state |
| [ ] | [ ] | [ ] | **[ ]** | Pass 35.0 — ce-dimension tolerance & tolerance types, SolidWorks-style (None/Basic/Bilateral/Symmetric/Limit/Min/Max) — zero existing representation today. **Next up**: Pass 34.2 built the per-ce-dimension property surface its controls need to live in |
| [ ] | [ ] | [ ] | ? | Pass 35.1 — drag a ce dimension's extension lines to extend/retract |
| [ ] | [ ] | [ ] | — | Pass 33.0 — real fix for reflow's auto-detected wrap width inheriting a prior edit's overflow (disclosure ships today; the fix itself — clamp / median-width / refuse — is undecided) |
| [ ] | [ ] | [ ] | ? | Pass 32.0 — delete one text run without deleting every run sharing its text object (fixes: deleting one CAD label deletes all 237 sharing a `BT`…`ET` block on the operator's own drawing) |
| ◐ | ◐ | [ ] | **[ ]** | Decision 028 remainder — Tab/Shift+Tab node cycling; arrow-key node nudge; readout-row corrections; clickable breadcrumb navigation. (**Pass 26.2 itself SHIPPED** 2026-08-06 — moved to *Implemented*) |
| [ ] | [ ] | [ ] | [x] | Pass 22.0 — make ce dimensions and foreign annotations selectable, marqueeable, and deletable from the canvas |
| [ ] | [ ] | [ ] | ◐ | Pass 23.0–23.3 — ce-dimension format/units GUI control; re-measure + whole-dimension move; descend into nested form-XObject containers; multi-node select/move/delete |
| — | — | [ ] | — | Pass 24.0, 24.2–24.5 — remaining ribbon slices (fixed-anchor confirm strip, contextual tool tabs, selection tabs, overflow/collapse, keyboard & accessibility) |
| [ ] | [ ] | [ ] | [x] | `EditSession::insert_pages` — true in-place page insertion (no Pass number yet; filed 2026-08-05, Pass 3.5's ship) — the core command that would let Insert edit the open document instead of always writing a new file |
| [ ] | [ ] | [ ] | [x] | Form field creation/authoring — text/checkbox/radio/choice/pushbutton, tab order (decision 020, Pass 20.0–20.7 — decided, scoped, not started; **operator's own priority #4, "after, if that makes sense"**) |
| [x] | [ ] | [ ] | **[ ]** | FF-C remainder — apply an embedded/subsetted font to existing text (Pass 21.2, `set-font`) |
| [ ] | [ ] | ◐ | **[ ]** | FF-C remainder — Pass 21.3 GUI font-embedding surface. Pass 37.1 shipped its Add-Text half (face picker, refusal-with-remedy, measured embed disclosure). **Still owed:** the `fsType` trust/licence disclosure (R109, owed since Pass 21.0 — core work) and a picker over EXISTING text (blocked on Pass 21.2) |
| [ ] | [ ] | [ ] | [x] | Forms P1 — radio groups, choice fields, flatten, FDF/XFDF in Batch Tools, regenerate-appearances, per-row page jump, canvas highlight (Pass 37.2's own named residuals) |
| [ ] | [ ] | — | — | Rich-text fill: should `pdfce-core`/`pdfce-cli` refuse too? Pass 37.2 guards the GUI only; `fill-field` can still silently discard stored formatting. Undecided — `ROADMAP.md` *Next up* |
| [ ] | [ ] | [ ] | [x] | Forms P2 — click-on-canvas-to-edit, comb cell dividers, `/Tabs` computed tab order, rich-text editing |
| [ ] | [ ] | [ ] | [x] | FF-B — cross-block / cross-page reflow — Acrobat's is cloud-gated and post-2021; a fully offline one exceeds it |
| [ ] | [ ] | [ ] | [x] | Bulleted/numbered list authoring — **awaiting an explicit operator yes/no; do not schedule until answered** |
| [ ] | [ ] | [ ] | ? | FF-I — minimal StructTree/`/ActualText` update on tagged-page text edits (deliberately cut from FF-H; needs its own decision record) |
| [ ] | [ ] | [ ] | ◐ | Vector graphics editing, Inkscape-parity breadth — path booleans (union/difference/intersection/exclusion/division); stroke & fill incl. gradients/patterns; object transforms with numeric entry (move/scale/rotate/skew); align/distribute; z-order; group/ungroup; text-to-path; general-purpose OCG layer authoring (beyond ce-dimension groups) |
| [ ] | [ ] | [ ] | [x] | Encryption — decrypt all legacy handlers, encrypt-on-save AES-128/256 (Pass 5 — spec-grounded, blocked on an AES-256 `/R 5` vs `/R 6` sourcing sub-decision) |
| [ ] | [ ] | [ ] | [x] | Redaction: mark by dragging on the canvas + transient property bar (scope-called out of Pass 8.1, not built) |
| [ ] | [ ] | [ ] | [x] | Redaction: Sanitize / Remove Hidden Information — not yet scoped to a Pass |
| [ ] | [ ] | [ ] | [x] | Digital signatures — PKCS#7 sign/verify, PAdES B-B/B-T/B-LT/B-LTA, RFC 3161 timestamping (lowest real-world demand of the ranked Backlog set — `/SigFlags` in 0.64% of the organic census) |
| [ ] | [ ] | [ ] | [x] | Bates numbering / stamping — header/footer stamps, sequential numbering across a batch, watermarks |
| [ ] | [ ] | [ ] | [x] | PDF/A conformance — convert-to and validate-against PDF/A-1/2/3/4 |
| [ ] | [ ] | [ ] | [x] | OCR — recognize-text-in-scanned-page (engine binding not yet chosen) |
| [ ] | [ ] | [ ] | [x] | Accessibility (PDF/UA) — tagged structure-tree authoring, reading-order tools, alt-text prompts |
| [ ] | [ ] | [ ] | [x] | Comparison — visual + text diff between two PDF revisions |
| [ ] | [ ] | [ ] | [x] | Portfolios (PDF Package) — multi-file container support |
| [ ] | [ ] | [ ] | [x] | Optimization / linearization — Fast Web View, image downsampling, font subsetting on save |
| [ ] | [ ] | [ ] | [x] | Print & prepress (PDF/X) — low priority unless the operator signals otherwise |
| [ ] | — | [ ] | — | Non-Latin UI font coverage — CJK/Arabic/Hebrew glyphs in pdfce's own chrome (bundle a subset vs. runtime system-font discovery, undecided) |
| [x] | — | [ ] | ? | DeviceCMYK → sRGB colorimetry — calibrated colour table (today's naive additive conversion diverges >8/255 on ~37% of CMYK pixels vs. pdfium) |
| [ ] | — | [ ] | ? | Autosave / crash-recovery, then true in-place Save (in-place save is deliberately gated on autosave existing first) |
| [ ] | [ ] | — | — | Release & distribution channel — Scoop + WinGet manifests, published checksums (blocked on a separate, not-yet-granted publish authorization — independent of the license decision) |

`◐` = partially shipped; the row names its own split. For the decision-028
row, `ROADMAP.md`'s decision-028 item table has the exact per-item state
(some of the 6+ named sub-items are done, some are not). In the
**Acrobat** column `◐` means something different — Acrobat has only part
of what the row names.

## Cannot

Structurally unavailable to pdfce — facts about its architecture and
legal posture, not editorial choices. Not a to-do list; these will not
acquire rows above.

- **Any Adobe Document Cloud storage/sync feature**, incl. mobile
  companion sync — pdfce has no cloud backend and `ARCHITECTURE.md` §1.1
  forbids undisclosed network calls by design.
- **Adobe Acrobat Sign as a hosted, multi-party e-signature routing
  service** (send-for-signature links, signing order, hosted audit
  trail). pdfce's own PKCS#7/PAdES signing is *not* blocked by this, nor
  is a local Fill-&-Sign equivalent.
- **AEM Document Security / persistent DRM with remote revoke** —
  requires an Adobe-hosted rights server; replicating it is a different
  product.
- **Adobe's specific cloud services** — Sensei-backed enhanced
  auto-tagging, Auto-Adjust Layout, Liquid Mode, AI Assistant. The
  *hosted mechanism* cannot be replicated; the underlying capabilities
  can, offline, and doing so is an exceed.
- **Anything only achievable by linking GPL/AGPL code** — pdfce is MIT
  (`LEGAL.md` §1), so MuPDF/Poppler/Ghostscript are out as shortcuts on
  any row above.

## Will not

Scope decisions already taken, from pdfce's own records — not this
file's judgment.

- **Dynamic XFA** — `out_of_scope` (decision 020); post-Acrobat-8.1 it
  carries no AcroForm, so there is no behaviour to match.
- **Static-XFA hybrid field creation** — refused by name (decision 020):
  pdfce can write the AcroForm half but not the XFA half, and a
  one-sided add makes two viewers disagree on field count.
- **Any network call without explicit, disclosed opt-in** —
  `ARCHITECTURE.md` §1.1, enforced by the fail-closed `no-network` CI
  gate (decision 003, R12). Rules out telemetry and silent update checks
  in their own right.
- **Barcode form fields** — Acrobat gates these to Pro as a narrow
  legal-workflow feature; out of scope here.
