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

## Implemented

### Document & pages

| core | cli | gui | Feature |
|:----:|:---:|:---:|---------|
| [x] | [x] | [x] | Open, parse, and render a PDF (Pass 1) |
| [x] | [x] | [x] | Recovers a damaged/malformed cross-reference table; tolerates common lenient-PDF defects (Pass 13a/13b, `/Contents`-defect fix) |
| [x] | [x] | [x] | Rotate pages, incl. refusal disclosure (Pass 3.2 core/CLI; GUI rotate buttons + `5b2682b` disclosure fix) |
| [x] | [x] | [x] | Delete pages (Pass 3.2 core/CLI; GUI `delete_pages` call in `main.rs`) |
| [x] | [x] | [x] | Reorder pages — thumbnail drag or keyboard (Pass 3.2 core/CLI; GUI `apply_reorder`/`move_pages_keyboard` in `main.rs`) |
| [x] | [x] | [x] | Merge / combine multiple files into one (Pass 3.2 core/CLI; GUI `merge_tool` add-files button in `main.rs`) |
| [x] | [x] | [ ] | Split a document into multiple files (Pass 3.2 core/CLI) — GUI panel names `split_cli_command()` rather than performing the split; the app tells the operator to use the CLI |
| [x] | [x] | [ ] | Insert pages from another file (Pass 3.2 core/CLI) — same shape as split: GUI panel names `insert_cli_command()`, no GUI verb yet |
| [x] | [x] | [ ] | Extract pages to a new file (Pass 3.2 core/CLI) — no GUI surface; grepping `extract_pages` in the GUI only hits `extract_pages_view`, which is TEXT extraction (see Text section), a different feature |
| [x] | [x] | [x] | Edit document metadata (`/Info` dict) (Pass 3.2 `set-info`; GUI Properties dock panel) |
| [x] | — | [x] | Undo/redo command log (Pass 3.1 core; wired into the GUI at Pass 14.3) |
| [x] | [x] | [x] | Save (incremental-by-default or full-rewrite); **"Save a copy" only — no true in-place overwrite yet** (see *Planned*) |

### Text

| core | cli | gui | Feature |
|:----:|:---:|:---:|---------|
| [x] | [x] | [x] | Extract & copy text: search index, `ToUnicode`, reading order, plus page/document-scope clipboard copy (Pass 4 core+CLI `extract-text`; GUI Copy ▾ menu, `CopyScope::Page`/`Document`) |
| [x] | — | [ ] | Select text on the canvas and copy the selection — the selection itself exists (Edit Text tool: triple-click, `text_caret_after_click`, a live caret/anchor pair), but there is no `CopyScope::Selection` and no copy verb reaches it. `cli` marked — (not `[ ]`): "mouse-select a range, then copy" has no batch shape; page/document-scope copy above already covers the CLI case |
| [x] | [x] | [x] | In-place text editing of existing runs (Pass 14.x) |
| [x] | [x] | [x] | Text formatting: size/colour/font-family-style, char/word spacing, horizontal scale, super/subscript, synthetic bold/italic (Pass 19.x, FF-H — complete end-to-end) |
| [x] | [x] | [x] | Within-block reflow incl. justified alignment (Pass 15.x, FF-A) — open defect: auto-detected wrap width can inherit a prior edit's overflow (Pass 33.0, disclosed but not yet fixed) |
| [x] | [x] | [x] | Add new page text: point insert + wrapped multi-line box (Pass 16.x, FF-D) |
| [x] | [x] | [ ] | Edit composite/CID (`/Type0`) text runs (Pass 29.0) — no GUI yet |
| [x] | [x] | [ ] | Add non-Latin text via a subsetted, embedded donor font (Pass 21.0, FF-C P0, glyf/TrueType donors only) — GUI slice not started |

### Vector objects (Inkscape-style editing)

| core | cli | gui | Feature |
|:----:|:---:|:---:|---------|
| [x] | [x] | [x] | Select vector objects: click, marquee, Alt+click cycle through overlapping hits (Pass 9a, 12.0, 18.5) |
| [x] | [x] | [x] | Move / delete a whole vector object (Pass 25.x, 28.0) |
| [x] | [x] | [x] | Enter an object and select one part (subpath) (Pass 25.0–25.4) |
| [x] | [x] | [x] | Move / delete a subpath (Pass 28.0 core, Pass 25.2 delete, Pass 36.0 GUI gesture) |
| [x] | [x] | [x] | Move a node (anchor point), incl. `re` rectangle corners and reused subpath starts (Pass 26.0, 30.0) |
| [x] | [x] | [x] | Delete a node (Pass 36.1) |
| [x] | [x] | [x] | Edit a Bézier handle / control point (Pass 30.1 core+CLI; Pass 26.1 GUI) |
| — | — | [x] | Level-ladder rung readout (Object/Part/Point) + Escape-to-ascend navigation (Pass 26.0, 36.2) — no clickable breadcrumb yet |

### ce dimensions

| core | cli | gui | Feature |
|:----:|:---:|:---:|---------|
| [x] | [x] | [x] | Author a ce dimension with snapping: linear/radius/diameter (Pass 12.M1, 12.M2, 12.M2b) |
| [x] | [x] | [x] | ce-dimension groups: scale, number format (decimal/fraction), ANSI/ISO drafting standard (Pass 12.M2, 27.2) |
| [x] | [x] | [x] | Drag to reposition a ce dimension (Pass 25.5) |
| [x] | [x] | [x] | Toggle a ce-dimension group's OCG layer visibility (Group Manager — currently a floating window, dock relocation planned) |
| [x] | [x] | [x] | Delete a ce dimension: annotation + `/AP` + `/PieceInfo` sidecar, together (Pass 25.6) |

### Annotations & markup

| core | cli | gui | Feature |
|:----:|:---:|:---:|---------|
| [x] | [x] | [x] | Render & count annotations/widget appearances, honouring the annotation-flag set (Pass 6.0) |
| [x] | [x] | [x] | Author geometric markup: Ink/Square/Circle/Line/Polygon (Pass 6.1) — minimal menu affordance only; no canvas drag/freehand drawing yet |
| [x] | [x] | [x] | Author text-bearing annotations: FreeText/Stamp with §12.7.3.3 variable-text appearance (Pass 6.2) |

### Forms (AcroForm)

| core | cli | gui | Feature |
|:----:|:---:|:---:|---------|
| [x] | [x] | [ ] | Fill text/checkbox/radio/choice fields (Pass 7.0, 7.1) — GUI form-fill explicitly named a follow-up slice, not built |
| [x] | [x] | [ ] | Flatten a form to static page content (Pass 7.1) |
| [x] | [x] | [ ] | Import/export form data, FDF/XFDF (Pass 7.1) |

### Redaction & security

| core | cli | gui | Feature |
|:----:|:---:|:---:|---------|
| [x] | [x] | [x] | Mark redactions by text search or named region (Pass 8.0, 8.1) |
| [x] | [x] | [x] | Apply redaction with a runtime-verified true-removal proof (Pass 8.0, 8.1) |

### Fonts & rendering

| core | cli | gui | Feature |
|:----:|:---:|:---:|---------|
| [x] | — | [x] | Rasterize page content: vector paths, text, images (Pass 1, 1.1) |
| [x] | — | [x] | Image codecs: DCT/JPEG, LZW, RunLength, CCITT Fax, JBIG2, JPEG 2000 (Pass 2.1–2.3) |
| [x] | [x] | [x] | Glyph-coverage gate on the app's own UI chrome — no tofu glyphs in pdfce's own strings (Pass 18.7) |

### Shell & UX

| core | cli | gui | Feature |
|:----:|:---:|:---:|---------|
| [x] | [x] | [x] | Live-edit canvas: renders the edited revision, not a static page image (Pass 17.x) |
| — | — | [x] | Interactive canvas: pan, zoom-to-cursor, marquee select (Pass 12.0, 18.8) |
| — | — | [x] | Dockable panel shell: Pages / Tool Options / Properties / Object-Layer tree (Pass 18.1, 34.1) |
| — | — | [x] | Ribbon command surface — File/Edit/Review/Measure/Tools/View tabs, reset-layout chooser covering both docks (Pass 24.1, chain `6449859`→`2b12efe`) |
| — | — | [x] | Implicit gesture-commit: clicking away accepts an in-progress edit instead of requiring a separate Accept/Reject click (Pass 34.0; slice 4 of the related Pass 34.1 dock-relocation work still owed, see *Planned*) |
| [x] | [x] | [x] | ScripTree-style SVG icon set for every GUI feature (Pass 18.3) |

## Planned, in predicted order

**Order is a best-effort derivation from `ROADMAP.md`'s *Next up*
(closest first), then *Backlog*, respecting the operator's own ★★★
priority sequence and stated demand rankings — it is not a promised
build order, and everything past the first several rows is this
librarian's judgment call, not the operator's.** All boxes below are
unticked unless noted; a `[x]` in *Planned* means "the underlying
model/verb already exists — only the named shell is missing," which is
worth knowing when scoping the work.

| core | cli | gui | Feature |
|:----:|:---:|:---:|---------|
| — | — | [ ] | Pass 34.1 slice 4 — move the last floating status/disclosure strips into the Tool Options dock |
| [x] | [x] | [ ] | Pass 34.2 — per-ce-dimension property panel, docked (position/radius-diameter reachable after placement; Group Manager moves out of its floating window) |
| [ ] | [ ] | [ ] | Pass 35.0 — ce-dimension tolerance & tolerance types, SolidWorks-style (None/Basic/Bilateral/Symmetric/Limit/Min/Max) — zero existing representation today |
| [ ] | [ ] | [ ] | Pass 35.1 — drag a ce dimension's extension lines to extend/retract |
| [ ] | [ ] | [ ] | Pass 33.0 — real fix for reflow's auto-detected wrap width inheriting a prior edit's overflow (disclosure ships today; the fix itself — clamp / median-width / refuse — is undecided) |
| [ ] | [ ] | [ ] | Pass 32.0 — delete one text run without deleting every run sharing its text object (fixes: deleting one CAD label deletes all 237 sharing a `BT`…`ET` block on the operator's own drawing) |
| ◐ | ◐ | [ ] | Pass 26.2 + decision 028 remainder — level survival across an edit; Tab/Shift+Tab node cycling; arrow-key node nudge; readout-row corrections; clickable breadcrumb navigation |
| [ ] | [ ] | [ ] | Pass 22.0 — make ce dimensions and foreign annotations selectable, marqueeable, and deletable from the canvas |
| [ ] | [ ] | [ ] | Pass 23.0–23.3 — ce-dimension format/units GUI control; re-measure + whole-dimension move; descend into nested form-XObject containers; multi-node select/move/delete |
| — | — | [ ] | Pass 24.0, 24.2–24.5 — remaining ribbon slices (fixed-anchor confirm strip, contextual tool tabs, selection tabs, overflow/collapse, keyboard & accessibility) |
| [ ] | [ ] | [ ] | Form field creation/authoring — text/checkbox/radio/choice/pushbutton, tab order (decision 020, Pass 20.0–20.7 — decided, scoped, not started; **operator's own priority #4, "after, if that makes sense"**) |
| [x] | [ ] | [ ] | FF-C remainder — apply an embedded/subsetted font to existing text (Pass 21.2, `set-font`) |
| [ ] | [ ] | [ ] | FF-C remainder — GUI font-embedding surface (Pass 21.3) |
| [ ] | [ ] | [ ] | FF-B — cross-block / cross-page reflow |
| [ ] | [ ] | [ ] | Bulleted/numbered list authoring — **awaiting an explicit operator yes/no; do not schedule until answered** |
| [ ] | [ ] | [ ] | FF-I — minimal StructTree/`/ActualText` update on tagged-page text edits (deliberately cut from FF-H; needs its own decision record) |
| [ ] | [ ] | [ ] | Vector graphics editing, Inkscape-parity breadth — path booleans (union/difference/intersection/exclusion/division); stroke & fill incl. gradients/patterns; object transforms with numeric entry (move/scale/rotate/skew); align/distribute; z-order; group/ungroup; text-to-path; general-purpose OCG layer authoring (beyond ce-dimension groups) |
| [ ] | [ ] | [ ] | Encryption — decrypt all legacy handlers, encrypt-on-save AES-128/256 (Pass 5 — spec-grounded, blocked on an AES-256 `/R 5` vs `/R 6` sourcing sub-decision) |
| [ ] | [ ] | [ ] | Redaction: mark by dragging on the canvas + transient property bar (scope-called out of Pass 8.1, not built) |
| [ ] | [ ] | [ ] | Redaction: Sanitize / Remove Hidden Information — not yet scoped to a Pass |
| [ ] | [ ] | [ ] | Digital signatures — PKCS#7 sign/verify, PAdES B-B/B-T/B-LT/B-LTA, RFC 3161 timestamping (lowest real-world demand of the ranked Backlog set — `/SigFlags` in 0.64% of the organic census) |
| [ ] | [ ] | [ ] | Bates numbering / stamping — header/footer stamps, sequential numbering across a batch, watermarks |
| [ ] | [ ] | [ ] | PDF/A conformance — convert-to and validate-against PDF/A-1/2/3/4 |
| [ ] | [ ] | [ ] | OCR — recognize-text-in-scanned-page (engine binding not yet chosen) |
| [ ] | [ ] | [ ] | Accessibility (PDF/UA) — tagged structure-tree authoring, reading-order tools, alt-text prompts |
| [ ] | [ ] | [ ] | Comparison — visual + text diff between two PDF revisions |
| [ ] | [ ] | [ ] | Portfolios (PDF Package) — multi-file container support |
| [ ] | [ ] | [ ] | Optimization / linearization — Fast Web View, image downsampling, font subsetting on save |
| [ ] | [ ] | [ ] | Print & prepress (PDF/X) — low priority unless the operator signals otherwise |
| [ ] | — | [ ] | Non-Latin UI font coverage — CJK/Arabic/Hebrew glyphs in pdfce's own chrome (bundle a subset vs. runtime system-font discovery, undecided) |
| [x] | — | [ ] | DeviceCMYK → sRGB colorimetry — calibrated colour table (today's naive additive conversion diverges >8/255 on ~37% of CMYK pixels vs. pdfium) |
| [ ] | — | [ ] | Autosave / crash-recovery, then true in-place Save (in-place save is deliberately gated on autosave existing first) |
| [ ] | [ ] | — | Release & distribution channel — Scoop + WinGet manifests, published checksums (blocked on a separate, not-yet-granted publish authorization — independent of the license decision) |

`◐` = partially shipped; see the row's Pass IDs and `ROADMAP.md`'s
decision-028 item table for the exact split (some of the 6+ named
sub-items are done, some are not).
