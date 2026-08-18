# pdfce — Feature status

**A scan, not a record.** One question only: *what can pdfce do, and
what's missing.* `ROADMAP.md` is organised by Pass and carries the
reasoning, history, Pass IDs and commit hashes — **none of that belongs
here.** If a row needs an argument, the argument goes in `ROADMAP.md`.

**One row per capability, one sentence, saying what works and what is
missing.** When a row changes, **replace** the sentence — never append a
note to it. A row that has grown a history has stopped being a scan.

Maintained by `pdfce-librarian` in the same filing as every `ROADMAP.md`
"Pass shipped" update. **`ROADMAP.md` wins any disagreement**; this file
is stale until the next filing.

## Legend

`[x]` built · `[ ]` **a gap** · `—` **not applicable**, a shape mismatch
rather than a gap (e.g. "pan the canvas" has no batch form) · `◐` partial.
Confusing `[ ]` with `—` is the mistake this legend exists to prevent.
In a *Planned* row only, `?` means **which of `[ ]` and `—` this is has
not been decided yet** — never "probably built".

**core** = `pdfce-core`/`pdfce-render` (headless) · **cli** =
`pdfce-cli` · **gui** = `pdfce-gui`.

**ce dimension** = one pdfce itself authors. **pdf dimension** = one
already in the file from CAD; pdfce reads it, never alters it
(`CLAUDE.md` rule 15).

`to-pdfa`, `validate-pdfa`, `sign` and `bates-stamp` exist in
`pdfce-cli --help` as stubs that print "not implemented". Not ticked
anywhere; listed under *Planned*.

### The `Acrobat` column

Does **Acrobat Pro** have the *capability* — never how its UI exposes it.

`[x]` has it · `[ ]` **verified absence** · `◐` has part of it · `—` not
meaningful to ask · `?` **not looked up.** Never read `?` as absence;
that is `[ ]` and only `[ ]`.

Authority: `Acrobat_Features\comparison__pdfce_feature_column.md`.
**Confidence is not uniform.** Redaction, forms, page ops, text and
measuring rest on dated RAG files. Signatures, encryption, Bates, OCR,
accessibility, comparison, portfolios, optimisation, PDF/A and prepress
rest on a single web search and have **no** dedicated RAG file —
provisional; re-verify before any acceptance criterion leans on them.

## Implemented

### Document & pages

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [x] | [x] | [x] | [x] | Open, parse and render a PDF. |
| [x] | [x] | [x] | [x] | Recover a damaged cross-reference table and tolerate common lenient-PDF defects. |
| [x] | [x] | [x] | [x] | Rotate, delete, reorder and extract pages; insert pages from another file. |
| [x] | [x] | [ ] | [x] | Set a page's size (`/MediaBox`) — writing a value equal to the page's own inherited size removes its own override rather than duplicating it, so a sibling page's size is never silently pinned by an unrelated edit. **No GUI surface.** |
| [x] | [x] | [x] | [x] | Merge several files into one. |
| [x] | [x] | [x] | [x] | Split a document — `EveryN` only; no bookmark- or size-based criteria. |
| [x] | [x] | [x] | ? | Keep preseparated-file (`/SeparationInfo`) page sets intact across delete/extract/split/merge. |
| [x] | [x] | [x] | [x] | Edit document metadata (`/Info`). |
| [x] | — | [x] | [x] | Undo/redo command log. |
| [x] | [x] | [x] | [x] | Save incrementally (default) or by full rewrite. **"Save a copy" only** — true in-place save is gated on autosave existing first. |
| — | — | [x] | [x] | Several documents open at once; close with a three-way save prompt. |

### Text

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [x] | [x] | [x] | [x] | Extract and copy text — search index, `ToUnicode`, reading order, page/document clipboard copy. |
| [x] | — | [ ] | [x] | Select text on the canvas and copy the selection. **No GUI surface.** |
| [x] | [x] | [x] | [x] | Edit existing text runs in place, including composite/CID (`/Type0`). |
| [x] | [x] | [x] | ◐ | Text formatting — size, colour, family/style, char and word spacing, horizontal scale, super/subscript, synthetic bold/italic. |
| [x] | [x] | [x] | [x] | Reflow within a block, including justified alignment. |
| [x] | [x] | [x] | [x] | Add new page text — point insert and wrapped multi-line box. |
| [x] | [x] | [x] | **[ ]** | Add non-Latin text via a subsetted, embedded donor font. |
| [x] | [x] | [x] | ? | Delete one text run without deleting others sharing its text object. |

### Vector objects (Inkscape-style editing)

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [x] | [x] | [x] | [x] | Select objects — click, marquee, Alt+click to cycle overlapping hits. |
| [x] | [x] | [x] | [x] | Move or delete a whole object. |
| [x] | [x] | [x] | **[ ]** | Descend into an object and select, move or delete one subpath. |
| [x] | [x] | [x] | **[ ]** | Move or delete a node, including `re` rectangle corners and reused subpath starts. |
| [x] | [x] | [x] | **[ ]** | Edit a Bézier handle, with grab/hover/live preview. |
| [x] | [x] | [x] | **[ ]** | Select several nodes and move them as one surgery, one undo entry. |
| — | — | [x] | **[ ]** | Level ladder (Object/Part/Point) with Escape to ascend; an edit truncates your rung one step rather than ejecting you. |
| — | — | [x] | [x] | Modeless select/edit — no tool armed *is* the object-edit tool. |
| — | — | [x] | ? | Object tree sidebar, nested object → subpath → node. |

### ce dimensions

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [x] | [x] | [x] | ◐ | Author a ce dimension with snapping — linear, radius, diameter. |
| [x] | [x] | [x] | **[ ]** | Groups carrying scale, number format (decimal/fraction) and ANSI/ISO drafting standard. |
| [x] | [x] | [x] | **[ ]** | Switch a placed circular dimension between radius and diameter. |
| [x] | [x] | [x] | [x] | Reposition by drag, or numerically. |
| [x] | [x] | [x] | [x] | Toggle a group's OCG layer visibility. |
| [x] | [x] | [x] | [x] | Delete a ce dimension — annotation, `/AP` and `/PieceInfo` sidecar together. |
| [x] | [x] | [x] | **[ ]** | Pick two lines to dimension them from the canvas — parallel gives a linear ce dimension, angled gives an angular one, collinear refused by name; a settable near-parallel threshold, an override that still prints the angle it overrode, and a virtual apex dimensioned and disclosed. |
| [x] | [x] | **[ ]** | ? | ce-dimension style — a per-group default with a per-ce-dimension override, independently per property (text height, line weight, arrow length, arrow form, colour, unit, fraction/precision, decimal marker, drafting standard); which tier supplied each value is readable. **No GUI surface yet**, so the override is settable only from the CLI. |
| [x] | [x] | **[ ]** | **[ ]** | ce-dimension tolerance — symmetric, deviation, limit, basic (boxed), min and max, inheriting through the style cascade like any other property, with its own precision slot; values in the displayed unit, refusals by name for a negative magnitude or an inverted limit pair. **The ISO 286 fit classes are not implemented** (the reference's class list is unverified). **No GUI surface yet**, so it is settable only from the CLI. |

### Annotations & markup

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [x] | [x] | [x] | [x] | Render and count annotations, honouring the annotation-flag set and an annotation's own `/CA` opacity — composited once as one object, so a shape's own overlaps do not darken at the seams. |
| [x] | [x] | [x] | [x] | Author geometric markup — Ink, Square, Circle, Line, Polygon. The GUI draws under the pointer as a canvas tool with options in the side pane. **Cannot set note text** (`/Contents`), opacity (`/CA`) or a cloudy border (`/BE`); a two-vertex "polygon" is wrongly accepted; **a placed markup cannot be selected, moved or resized.** |
| [x] | [x] | [x] | [x] | Author text-bearing annotations — FreeText and Stamp with variable-text appearance. |
| [x] | [x] | [x] | [x] | Read an annotation's note text, author and modification date. |
| [x] | [x] | [x] | [x] | Delete an annotation. |
| [x] | [x] | [ ] | [x] | Restyle a placed markup annotation — colour, interior fill, line width, opacity and arrowheads, across seven subtypes. Refuses a ce dimension by name (regenerating its `/AP` from style alone would strip its label and witness lines). **Distinct from markup AUTHORED at reduced opacity, below (Planned) — this is restyling something already placed.** **No GUI surface.** |
| — | — | [x] | [x] | Comments panel — browse every note and markup. |

### Forms (AcroForm)

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [x] | [x] | [x] | [x] | Fill fields — text, check box, radio, choice. Rich text (`/RV`) is read and exported but can only be *replaced* by a disclosed downgrade to plain text. |
| [x] | [x] | [ ] | [x] | Flatten a form to static page content. **No GUI surface.** |
| [x] | [x] | [x] | [x] | Import and export form data — FDF, XFDF and two-column CSV. CSV values that a spreadsheet would read as formulae are prefixed and disclosed. |
| [x] | [x] | ◐ | [x] | Create a field — text, check box, radio, choice, push button — with a name-collision resolver, border style, visibility, password and comb. **The GUI cannot create a push button.** **Adding a *selected* radio button is the one verb that can fail *after* committing** — its `Err` does not mean "nothing happened" (`Pass 73.1`). |
| [x] | [x] | [x] | [x] | Delete a field, a single widget, or a grouping node and its named subtree. |
| [x] | [x] | [x] | [x] | Rename a field, reporting how many descendants the rename reached. |
| [x] | [x] | [ ] | [x] | Move a widget, carrying its artwork rather than regenerating it. **No GUI surface.** |
| [x] | [x] | [x] | [x] | Reset a form to its defaults — `/V` is *removed* where no `/DV` exists, never blanked. |
| [x] | [x] | [ ] | [x] | Recognise and disclose script-driven fields, and natively recompute a whitelisted built-in subset. **No script is ever executed.** Dates, numbers, percentages and the fixed masks format; an ambiguous stored date is refused rather than guessed. **No GUI surface for the script census.** |
| [x] | [x] | — | [x] | Read a choice field's `/I`/`/TI` and a widget's caption (`/MK /CA`). Other `/MK` keys are not read. |
| [x] | [x] | [x] | ? | Detect XFA, and warn that filling a hybrid form leaves the XFA half stale. **The XFA half is never written.** |

### Redaction & security

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [x] | [x] | [x] | [x] | Mark redactions by text search, named region or pattern. |
| [x] | [x] | [x] | ◐ | Apply redaction — true removal, forced full rewrite, in all three. **The runtime proof that the removed text is absent from the output bytes runs in the GUI only**; the CLI writes the file before its gate, and that gate checks a different property (the carrier sweep). |
| [x] | [x] | [ ] | ? | Redaction mark appearance follows Table 192's precedence ladder — `/OverlayText` burnt in via the shared variable-text layout path, absent `/IC` left transparent (not painted black), `/RO` disclosed and left undrawn (falls back to a visible box), `/Repeat` disclosed and ignored — reachable from manual, search AND pattern marking alike. **No GUI control authors overlay text, reads `/RO`, or sets `/Repeat`; overlay-text colour is not settable from any shell (hard-coded black `/DA`).** |
| [x] | [x] | [ ] | ? | Detect an unencrypted wrapper (§7.6.7) and warn that the visible page is a cover, not the document. **No GUI surface.** |

### Fonts & rendering

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [x] | [x] | [x] | [x] | Rasterize vector paths, text and images; `render-page` writes a PNG. |
| [x] | [ ] | [ ] | ? | Rasterize an arbitrary page **region**, so magnification is bounded by viewport size instead of page size. **No shell calls it.** Cost is dominated by page interpretation, not pixels — **~99% on a dense CAD sheet (691 ms floor), ~36% on a text page (3.2 ms)** — so never tile for speed: the 3×3 penalty is 9× on the first and 1.9× on the second. Every render re-interprets the page; nothing is cached between calls. |
| [x] | — | [x] | ? | Off-thread cancellable rendering; a large CAD drawing stays interactive. |
| [x] | — | [x] | [x] | Image codecs — DCT/JPEG, LZW, RunLength, CCITT Fax, JBIG2, JPEG 2000. |
| [x] | [x] | — | — | JPEG write path. |
| [x] | [x] | [ ] | ? | Colour spaces and PDF functions — all four function types, spot colour rendered through the document's own tint transform, twelve render-diagnostic counters (unresolved spaces, ICC fallback, tint success/failure, pattern/shading/indexed shortfalls) printed by `render-page`. **GUI surface deliberately paused by the operator, not yet built** — no GUI code path reads any of the twelve counters. **No ICC engine**; `ICCBased` falls back to `/Alternate`, disclosed. |
| [x] | [x] | [ ] | [x] | `/Separation`/`/DeviceN`/`Lab`/`CalGray`/`CalRGB` colour spaces on image XObjects — the per-pixel path, delegated to the same colour-space module vector fills already used (no second implementation). `/Separation /None` correctly paints nothing (§8.6.6.4 "shall never be painted") — pdfium paints it solid black, a reference-renderer divergence, not a pdfce gap. **GUI surface deliberately paused by the operator, not yet built.** |
| [x] | [x] | [ ] | [x] | Paint shading via the `sh` operator — axial and radial (types 2–3), per-pixel against the current clip. Ghent PDF Output Suite coverage 0/16 → 14/16; the 2 unpainted are mesh shadings, not this row. Shading **patterns** (`PatternType 2`) are a separate row, below. **GUI surface deliberately paused by the operator, not yet built.** |
| [x] | [x] | [ ] | [x] | Paint shading patterns (`PatternType 2`, named via `scn`) — anchored to the pattern's own base CTM (the parent content stream's CTM at entry), not the current CTM `sh` uses (§8.7.2 NOTE 1/PM5), so a pattern is immune to a `cm` issued after it is selected. The fill path becomes the paint mask, resolved through the same shading painter `sh` uses. Tiling patterns (`PatternType 1`) still paint nothing — see *Planned*. **GUI surface deliberately paused by the operator, not yet built.** |
| [x] | [x] | [ ] | [x] | Blend modes — the eleven separable modes (Multiply, Screen, Overlay, Darken, Lighten, ColorDodge, ColorBurn, HardLight, SoftLight, Difference, Exclusion; Normal/Compatible already correct as a no-op) read from `ExtGState /BM`, including its array form. **Computed in device sRGB, not the transparency group's own colour space (§11.3.4) — wrong for a CMYK-space group**, which requires subtractive components complemented before and after blending (`blend_subtractive(cb, cs) = 1 − B(1 − cb, 1 − cs)`); `Pass 97.1` is the fix. The page itself is now modelled as an isolated group over a transparent backdrop, composited to white once at the end (§11.4.7) — not filled opaque white before painting, which is why the switch was previously undetectable for `Normal`. **Non-separable modes (Hue/Saturation/Color/Luminosity) are a separate row, below — refused, not this row.** **GUI surface deliberately paused by the operator, not yet built.** |
| [x] | [x] | [ ] | [x] | Transparency GROUP compositing — a group renders into its own page-sized offscreen buffer with contents-state reset to initial (`Normal`/alpha 1.0) at entry, then composites as a unit carrying the outer blend mode/alpha (§11.4.5). Ghent PDF Output Suite GWG 16.0 (non-knockout blend-mode panel) renders clean; `groups_flattened` 187→0 on that file. Knockout groups (`/K true`, plus §9.3.8 default-`/TK` text and §11.7.4.4 `B`/`b`) now buffer correctly but **still composite as ordinary groups internally, not correct** — approximated and counted, see *Planned*. **A group buffered for the OUTER-state reason (non-Normal blend/reduced alpha at the invoking `Do`, not the group's own `/I`) starts fully transparent — isolated semantics — even when the group itself is non-isolated, so its contents blend against nothing instead of the page's accumulated backdrop (§11.4.4 backdrop removal): wrong on 14, 15 and 7 of 16 cells across three Ghent transparency patches, all counted as successes by `transparency_groups_composited`. `Pass 97.0` (the real compositor) is the fix — see *Planned*.** **GUI surface deliberately paused by the operator, not yet built.** |
| [x] | [x] | [ ] | [x] | Overprint SIMULATION — ISO 32000-1 §11.7.4.3 Table 149's `CompatibleOverprint` blend, applied to fills, strokes, text, images and shadings. An overprinting paint is rasterised to a coverage mask by the same rasteriser a normal paint uses, then blended per pixel in CMYK reconstructed from the RGB buffer (`rgb_to_cmyk`/`cmyk_to_rgb` are exact inverses, verified over 4,913 colours to 1e-5); a composite that cannot run falls back to a normal paint and reports itself as `overprint_refused`. Ghent PDF Output Suite 22 → 25 of 51 patches passing. **Simulating is a policy choice, not an obligation — §8.6.7 makes ignoring `/OP` conformant.** Spot colorants are still flattened into RGB, so Table 149's SPOT row is not honoured and 7 Ghent overprint patches still fail; that needs a real per-colorant buffer, and the cheap approximation was measured and rejected. **GUI surface deliberately paused by the operator, not yet built.** |
| [x] | [x] | [ ] | [x] | Overprint (`/OP`, `/op`, `/OPM`) tracked and disclosed — Table 58's rule (`/OP` sets both parameters unless `/op` is in the SAME dictionary), counted per page, printed on `render-page`'s stable line alongside the three simulation counters (`overprint_composited`, `overprint_refused`, `overprint_pixels`); `overprint_refused` non-zero means the operator is seeing knocked-out backdrops where a press would show ink, which is the one shortfall that cannot be seen on the page. Fires on 20 of 51 Ghent patches; a further `overprint_effective` predicate (§11.7.4.3) counts where honouring it would actually change the painted result, non-zero on transparency patches too, not only the classic overprint set. **Visual simulation now ships — separate row, above.** **GUI surface deliberately paused by the operator, not yet built.** |
| [x] | [x] | [ ] | [x] | Soft masks from `ExtGState /SMask` — `/Alpha` and `/Luminosity` mask groups built and APPLIED (§11.6.5), distinct from the per-image `/SMask` row above; until now they were parsed, counted and disclosed as unimplemented. `/BC` defaults to the group colour space's *initial value* (black), not to all-zeros — all-zeros is black in RGB and **pure white** in `DeviceCMYK`, which would open the mask exactly where it should be shut. The backdrop pre-fills the mask buffer, so "outside the `/BBox`" is `TR(lum(BC))` with no special case; the group composites over a fully opaque backdrop (§11.5.3); luminosity is `0.30R + 0.59G + 0.11B` with **no** gamma compensation (§11.5.3 NOTE 3, deliberately not Rec.709); the mask's space is `/Matrix` × the CTM **at `gs`**, baked eagerly so a later `cm` cannot displace it. The mask is multiplied into the CLIP (which every paint site already honours), so `/SMask /None` restores a snapshot — a `W n` under a live mask makes that snapshot stale and is COUNTED (`soft_masks_reset_stale`), never silently mis-clipped. **`/TR` is read, counted and disclosed but NOT evaluated** (`soft_mask_tr_ignored`): `/TR` is where a mask gets inverted, so an ignored one can leave visible exactly the content a document meant to hide. **The mask is applied to each element INSIDE a group rather than to the group's RESULT (§11.4.5), so no Ghent soft-mask patch passes yet** — strip correlation moved on all three measurable ones (e.g. `GWG168` 0.661 → 0.725 against a 0.981 reference). **GUI surface deliberately paused by the operator, not yet built.** |
| [x] | — | [x] | ? | Calibrated `DeviceCMYK`→sRGB, one conversion site document-wide. |
| [x] | — | [x] | ? | Transparency — `/SMask`, `/Mask` (soft, explicit, colour-key) and `/Matte`. |
| [x] | [x] | [x] | [x] | Insert an image as a new XObject, including GUI drag-and-drop. |
| [x] | [x] | [x] | [x] | Optional content (layers) affects rendering — content-stream and XObject `/OC`. |
| [x] | [x] | [x] | — | Glyph-coverage gate on the app's own UI chrome. |
| [x] | [x] | [x] | ◐ | Report a document's fonts — `/BaseFont`, subtype, encoding, embedded/subset status, byte size (raw+decoded), `fsType`, `/ToUnicode`, a stated-reason removability verdict, and named surface-coverage. Acrobat exposes none of the first three and refuses unembed candidates silently. |
| [x] | [x] | [x] | ◐ | Remove an embedded font's program, refusing by name (reason shown) where content-stream codes are positions into that program (`Identity-H`/CID) or otherwise unsafe (8 of 9 verdicts refuse). Strips the subset tag and `/CIDSet`/`/CharSet` by default; a full rewrite is required to reclaim the freed bytes. |
| [x] | [x] | [x] | ◐ | Embed a font that's referenced but missing — attach a program to an existing descriptor, or synthesise a Base-14 descriptor/widths/encoding from pdfce's own metrics; resolves `/BaseFont` via exact match, subset-stripped name, standard-14 alias or (opt-in only) pdfce's own bundled face, disclosing which; glyph positions cannot move, only shapes change. When a bundled face is actually embedded, the BSD-3-Clause notice is attached to the output as `FONT-LICENSE-NOTICE.txt`; a run answered from `--font-dir` attaches nothing. |

### Reading, navigation & printing

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [x] | [x] | [x] | [x] | Find text on the page. |
| — | [x] | [x] | [x] | Read and navigate an existing bookmark tree. **No authoring.** |
| [x] | [x] | [ ] | [x] | List, extract, attach and detach embedded attachments — detach removes the stream too, not just the name-tree entry; extract refuses to derive a path from the document-supplied name. A multi-node (`/Kids`) name tree is refused by name. **No GUI surface at all.** |
| [x] | [x] | [x] | ◐ | Print — enumerate printers, spool a real job honouring the operator's CMYK intent and the sheet orientation it's planned against (the GUI preview turns with it), with duplex, copies, page subsets and a four-way comments-and-forms filter; GUI dialog is tabbed, resizable with both scrollbars, previews real page content zoomable/pannable, and opens on Ctrl+P. |
| [x] | [x] | [ ] | ? | Printer paper selection — the print job's DEVMODE is built from the driver's own real default (`DocumentProperties`), not a zeroed struct, so a chosen paper size actually reaches the driver. **No GUI surface for this specifically** (the general print dialog above is GUI-complete; this row is the driver-fidelity fix underneath it). |
| [x] | [x] | [ ] | ? | Printer properties — every OTHER driver-configured setting (colour mode, quality, and whatever else lives in the driver-private `dmDriverExtra` tail — up to 7,972 B measured on a real device) survives into the job, not only the fields pdfce itself sets. **No GUI surface.** |
| [x] | [x] | [ ] | ? | Input-tray selection — a three-state `FormSourceSupport` (`Supported`/`NotListed`/`Unsupported`), since some devices report no tray list at all while still having a real default source; the chosen tray now actually reaches the job. **No GUI surface.** |
| [x] | [x] | [ ] | ? | Per-page print orientation — a printed page honours its OWN `/Rotate`, not only the job-level Portrait/Landscape choice; previously the print path ignored `/Rotate` entirely. **No GUI surface.** |
| — | [x] | [ ] | [x] | Imposition — N-up, booklet, poster; mutually exclusive, refused in combination. **No GUI surface at all.** |
| [x] | [x] | [x] | [x] | View and toggle a foreign producer's `/OCProperties` layer tree. |
| — | — | [x] | [x] | Read mode and full screen, as two separate toggles. |
| [x] | [x] | [x] | [x] | Report a signature's `/ByteRange` coverage. **Not cryptographic verification** — no digest is computed. |

### Shell & UX

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| — | [x] | — | **[ ]** | A first-class scriptable CLI over the capabilities above. |
| [x] | [x] | [x] | — | Live-edit canvas — renders the edited revision, not a static image. |
| — | — | [x] | — | Pan, zoom-to-cursor, marquee select. **Marquee is full-enclosure only.** |
| — | — | [x] | — | Dockable panel shell, ribbon, fixed Quick Access Toolbar, density convention. |
| — | — | [x] | ? | Arming a tool raises its options in the side pane, from wherever the ribbon last sent it. |
| — | — | [x] | ? | The GUI binary answers `--help`/`--version` on the terminal without opening a window; an unknown flag exits 2. |
| — | — | [x] | ? | Independent edit toggles plus one master edit switch. |
| — | — | [x] | ? | Multi-target verbs act on the whole selection or refuse — never a silent subset. |
| — | — | [x] | — | Implicit gesture commit — clicking away accepts an in-progress edit; Enter commits a typing draft. |
| — | — | [x] | — | Nothing floats over the canvas. |
| [x] | [x] | [x] | — | SVG icon set covering every GUI feature. |
| [x] | [x] | [x] | ? | Persisted user settings. |
| [x] | [x] | [x] | — | Strip an optional capability at build time — `--no-default-features` drops it from every shell and the lighter binary refuses it **by name**, never rendering a blank. JPEG 2000 is the only gated capability today. |
| [x] | — | [x] | ? | Theming — three presets with a live picker. |

### Export

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [x] | [x] | [x] | [ ] | Export vector page content to DXF (R2000/AC1015), with scale inference per page. |

## Planned, in predicted order

Order is derived from `ROADMAP.md`'s *Next up* then *Backlog*; past the
first few rows it is a judgement call, not a promise. A `[x]` here means
the model or verb exists and only the named shell is missing. The
`Acrobat` column says whether Acrobat already has the thing.

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [ ] | [ ] | ◐ | ? | **Redaction absence proof in `pdfce-core`** — the three-way verdict (decoded-stream survivor ⇒ refuse; raw-bytes-only ⇒ disclose and require acknowledgement; found nowhere ⇒ *verified*) so every shell gets it. The GUI has its own copy today; core and the CLI have none. |
| [x] | [x] | [x] | [x] | **Encryption** — RC4 (40–128 bit), AES-128 (`/AESV2`) and AES-256 at `/R` 5 (`/AESV3`) decrypt read-only, including the empty-user-password case every other reader opens silently; CLI (`--open-password`/`--open-password-file`) and GUI (inline canvas prompt) can supply a password for any of the three. All eight Table 22 permission bits shown read-only in Properties > Security, captioned declared-by-the-author and unenforced; a `/Perms` mismatch (possible only at `/R` 5) is reported, never refused on. `/R` 6 is still refused by name — its Algorithm 2.B is unsourced. **Writing an encrypted document is still unimplemented in every configuration.** |
| [ ] | ? | [ ] | ? | **Reusable parsed page handle (display list)** — a shell holds it across frames and replays it against one region per frame, so repeat renders of an unchanged page cost fill (tens of ms) instead of interpretation (~700 ms on a dense CAD sheet). Keyed on `(page, edit epoch)`. The CLI column is `?` on purpose: a one-shot invocation may have nothing to hold it across, and whether that is `—` or `[ ]` is decided in the Pass. |
| [ ] | — | [ ] | [x] | Paint shading — function-based (type 1) and mesh (types 4–7). Type 1's colour ramp resolves but its geometry is not painted (zero occurrences measured in the Ghent corpus); mesh geometry (§8.7.4.5.5–.8) is not yet ingested from spec. Axial/radial (types 2–3) shipped — see *Implemented*. |
| [ ] | — | [ ] | [x] | Paint tiling patterns (`PatternType 1`). Currently deferred; a fill naming a tiling pattern via `scn` still paints nothing, a conforming default per Table 74, not a crash. Shading patterns (`PatternType 2`) now paint — see *Implemented*. |
| [ ] | — | [ ] | [x] | Non-separable blend modes — Hue, Saturation, Color, Luminosity. **Refused by name, not mapped**: tiny-skia 0.11.4's implementation is measurably wrong against both ISO 32000-1 and W3C Compositing-1 (up to 107/255 error on 9.4–15.5% of random colour pairs; root cause a `clip_color` sign-condition bug that hard-clamps instead of rescaling). Separable blend modes shipped — see *Implemented*, above. No conformant pure-Rust alternative identified yet. |
| [ ] | — | [ ] | [x] | Knockout transparency groups, and applying a soft mask to a group's RESULT rather than to each element inside it. Distinct from the per-image `/SMask`/`/Mask` and from `ExtGState /SMask` soft masks, both of which now ship (see *Implemented*), and from non-knockout group compositing, which also ships. **The knockout population is far larger than explicit `/K true`**: §9.3.8's `/TK` defaults `true` (every text object), §11.7.4.4 makes `B`/`B*`/`b`/`b*` knockout, §11.6.7 makes shading patterns knockout — `groups_knockout_approx` (47 on the Ghent corpus) measures only the explicit-`/K` slice. Knockout groups composite as ordinary groups today — correct outer boundary (buffering fixed), incorrect internal occlusion order (§11.4.6: each element should composite against the group's INITIAL backdrop, not the accumulated result); GWG 16.1 still shows its crosses. **Isolated knockout is representable bit-exact in pdfce's buffer model (spec-confirmed); non-isolated knockout (the common case) is not, pending buffer-model work.** Soft masks fold into the clip, which attenuates each element rather than the group's composited result, so all 4 Ghent soft-mask patches still fail; that is the same offscreen-buffer work as knockout and belongs with it. `/TR` transfer functions on a soft mask are also still unevaluated — they need the PDF-function machinery inside `pdfce-render`. |
| [ ] | — | [ ] | [x] | Type 3 font rendering and `Tr` 4–7 text-clipping modes. |
| [ ] | — | [ ] | [x] | Per-colorant (n-channel) compositing buffer — one plate per colorant with RGB synthesised only at display. Needed for the 7 Ghent overprint patches that simulation still fails (Table 149's SPOT row preserves the backdrop in both overprint modes, and a flattened-RGB backdrop cannot say whether it was spot or process), and for blending in a `DeviceCMYK` blending colour space. A cheaper page-sized spot-ink multiplier plate was built, ablated and reverted — it flipped no patch and regressed one. Overprint simulation itself ships — see *Implemented*. |
| [ ] | — | — | [x] | `/OutputIntents`-aware CMYK conversion, using a PDF/X file's own embedded destination profile instead of pdfce's baked fallback table. Gated on the sibling `iccce` project (`ROADMAP.md` Backlog). |
| [ ] | — | — | [x] | `/ICCBased` colour spaces resolved through a real ICC profile instead of the `/Alternate` fallback. Gated on `iccce`. |
| [ ] | [ ] | [ ] | [x] | **Markup opacity (`/CA`) — the WRITE half only.** Reading shipped first (see the render row above); authoring a markup at reduced opacity does not exist yet, in any shell. |
| [ ] | [ ] | [ ] | [x] | **Revision clouds** — `/BE << /S /C /I n >>` on a polygon, and the half that matters more, a cloudy border on a dragged rectangle. |
| [ ] | [ ] | [ ] | ? | **Note text on geometric markup** — `/Contents` with `/T` and `/M` together, since a note listed with no author reads as a broken panel. The read side already ships. **A shell is waiting on this.** |
| — | [ ] | [ ] | — | **Operator-initiated download** — pinned URL + SHA-256 verified before anything reaches disk, in the sibling crate `pdfce-fetch`; **built and tested, but no shell depends on it**, so nothing is operator-reachable. `core` is `—`: the engine is network-free permanently, so this can never live there. |
| [ ] | [ ] | [ ] | ◐ | Re-subset an embedded font down to only the glyphs used — no removal, no visual change, works even where unembedding is refused. |
| [ ] | [ ] | [ ] | ◐ | Convert text to vector paths — the only one of the font operations that works where unembedding is refused; irreversible, and the text stops being text. |
| [ ] | [ ] | [ ] | **[ ]** | Replace one font with another across a document, remapping encodings and widths — Acrobat has no equivalent. |
| — | — | [ ] | [x] | Imposition in the GUI — needs the sheet composition extracted into `pdfce-print` so both shells share one implementation. |
| [ ] | [ ] | [ ] | [x] | Move and resize anything carrying a `/Rect` — widgets, markup, redaction marks, links, ce dimensions. |
| [ ] | [ ] | [ ] | ? | Resize a vector object. |
| ◐ | ◐ | [ ] | **[ ]** | Node-editing remainder — Tab cycling and arrow-key nudge. |
| [ ] | [ ] | [ ] | [x] | Make ce dimensions and foreign annotations selectable and deletable from the canvas; also gates click-a-comment-to-select. |
| [ ] | [ ] | [ ] | **[ ]** | ce-dimension tolerance, the ISO 286 fit forms — fit, fit-with-tolerance, fit-tolerance-only, plus block and general tolerance. Every other tolerance form is built; these six need a sourced class/table lookup pdfce does not have. |
| — | — | [ ] | ? | ce-dimension style AND tolerance in the GUI — **one panel covering both**, showing which values are inherited and which are overridden and letting the override be set. The model and the CLI already do all of it; only the disclosure surface is missing. |
| [ ] | [ ] | [ ] | ? | Drag a ce dimension's extension lines. |
| [ ] | [ ] | [ ] | [x] | Re-measure a placed ce dimension — change what it measures without losing its id, group and placement. |
| [ ] | [ ] | [ ] | [x] | Select, move and resize a placed markup annotation on the canvas. |
| [ ] | [ ] | [ ] | [x] | True in-place page insertion, so Insert edits the open document. |
| [ ] | [ ] | [ ] | ? | Wide/batch CSV — one row per document, for filling many copies of one form. |
| [ ] | [ ] | [ ] | ? | Static-XFA hybrid — read and fill the XFA half in step with AcroForm. |
| [ ] | [ ] | [ ] | [x] | Links and bookmarks authoring — create, edit, reorder, named destinations. |
| [ ] | [ ] | [ ] | [x] | Redaction: mark by dragging on the canvas; Sanitize / Remove Hidden Information. **Sanitize is blocked on `Pass 73.0`** — the rule that would force it to a full rewrite (R58) is asserted, not enforced, so a Sanitize built today would save incrementally and its absence test could not fail. |
| [ ] | [ ] | [ ] | [x] | Digital signatures — PKCS#7/PAdES signing and verification. |
| [ ] | [ ] | [ ] | [x] | Bates numbering. |
| [ ] | [ ] | [ ] | [x] | PDF/A conformance and validation. |
| [ ] | [ ] | [ ] | [x] | OCR — core writes the invisible sandwich layer (`ocr::layer`: §9.3.6 mode 3, additive, the scan never re-encoded) and now refuses a certified change-forbidding document (`/Perms` → `/DocMDP`) as its `add_text` twin always did, the `ocrs` engine is bound behind a Cargo feature, and the CC-BY-SA-4.0 weights now ship (12,240,008 B), so a build **can** recognise text end to end; **no shell has a surface, so nothing is operator-reachable**, and recognition **quality is unproven** — the only test documents are vector PDFs that already contain text. |
| [ ] | [ ] | [ ] | [x] | Accessibility (PDF/UA) tagging. |
| [ ] | [ ] | [ ] | [x] | Document comparison. |
| [ ] | [ ] | [ ] | [x] | Portfolios (PDF Package). |
| [ ] | [ ] | [ ] | [x] | Optimization and linearization. |
| [ ] | [ ] | [ ] | [x] | Print and prepress (PDF/X). |
| [ ] | [ ] | [ ] | ◐ | Vector editing at Inkscape-parity breadth. |
| [ ] | [ ] | [ ] | [x] | Bulleted and numbered list authoring. |
| [ ] | [ ] | [ ] | ? | Ribbon and keyboard customisation with saved layouts. |
| [ ] | — | [ ] | — | Non-Latin UI font coverage. |
| [ ] | — | [ ] | ? | Autosave and crash recovery, then true in-place save. |
| [ ] | [ ] | — | — | Release and distribution channel. |
| — | — | [ ] | ? | Right-click context menus, canvas and object tree. |
| — | — | [ ] | — | Remaining ribbon slices — contextual tabs, overflow, keyboard accessibility. |

## Cannot

Structurally unavailable — facts about pdfce's architecture and legal
posture, not editorial choices. These will not acquire rows above.

- **Adobe Document Cloud storage/sync.** pdfce has no cloud backend and
  forbids undisclosed network calls by design.
- **Acrobat Sign as a hosted routing service.** pdfce's own
  PKCS#7/PAdES signing is not blocked by this.
- **AEM Document Security and third-party DRM handlers** (FileOpen,
  Locklizard). Each needs an Adobe-hosted rights server or an
  undocumented handler — nothing to implement against, not merely
  declined.
- **Adobe's cloud services** — Sensei auto-tagging, Liquid Mode, AI
  Assistant. The hosted mechanism cannot be replicated; the underlying
  capabilities can, offline.
- **Anything requiring GPL/AGPL code.** pdfce is MIT, so
  MuPDF/Poppler/Ghostscript are out.

## Will not

Scope decisions already taken, from pdfce's own records.

- **Dynamic XFA** — post-Acrobat-8.1 it carries no AcroForm, so there is
  no behaviour to match.
- **Static-XFA hybrid field *creation*** — pdfce can write the AcroForm
  half but not the XFA half, and a one-sided add makes two viewers
  disagree on field count. Reading and filling is planned; creating is
  not.
- **Telemetry, analytics, crash reporting, licence callback, or any
  silent phone-home** — off by design; anything network-touching is
  opt-in, off by default and disclosed. The engine (`pdfce-core`,
  `pdfce-render`) additionally **cannot** contain a network client, and
  a fail-closed CI gate enforces that half. Operator-initiated
  downloads in the shells (models, updates, add-ins) are **permitted as
  of 2026-08-13**; the primitive exists in the sibling crate
  `pdfce-fetch`, and **no shell links it yet** (`ROADMAP.md` `R12`,
  decision 061, `Pass 77.0`).
- **Executing embedded JavaScript** — a sandboxed engine is prohibited
  by standing rule. Recognised built-ins are reimplemented natively
  instead.
- **Barcode form fields.**
