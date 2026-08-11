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

### Annotations & markup

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [x] | [x] | [x] | [x] | Render and count annotations, honouring the annotation-flag set. |
| [x] | [x] | [x] | [x] | Author geometric markup — Ink, Square, Circle, Line, Polygon. **Cannot set note text** (`/Contents`), so the Comments panel shows "No note text" on pdfce's own. |
| [x] | [x] | [x] | [x] | Author text-bearing annotations — FreeText and Stamp with variable-text appearance. |
| [x] | [x] | [x] | [x] | Read an annotation's note text, author and modification date. |
| [x] | [x] | [x] | [x] | Delete an annotation. |
| — | — | [x] | [x] | Comments panel — browse every note and markup. |

### Forms (AcroForm)

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [x] | [x] | [x] | [x] | Fill fields — text, check box, radio, choice. Rich text (`/RV`) is read and exported but can only be *replaced* by a disclosed downgrade to plain text. |
| [x] | [x] | [ ] | [x] | Flatten a form to static page content. **No GUI surface.** |
| [x] | [x] | [x] | [x] | Import and export form data — FDF, XFDF and two-column CSV. CSV values that a spreadsheet would read as formulae are prefixed and disclosed. |
| [x] | [x] | ◐ | [x] | Create a field — text, check box, radio, choice, push button — with a name-collision resolver, border style, visibility, password and comb. **The GUI cannot create a push button.** |
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
| [x] | [x] | [x] | ◐ | Apply redaction with a runtime-verified true-removal proof. |
| [x] | [x] | [ ] | ? | Detect an unencrypted wrapper (§7.6.7) and warn that the visible page is a cover, not the document. **No GUI surface.** |

### Fonts & rendering

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [x] | — | [x] | [x] | Rasterize vector paths, text and images. |
| [x] | — | [x] | ? | Off-thread cancellable rendering; a large CAD drawing stays interactive. |
| [x] | — | [x] | [x] | Image codecs — DCT/JPEG, LZW, RunLength, CCITT Fax, JBIG2, JPEG 2000. |
| [x] | [x] | — | — | JPEG write path. |
| [x] | — | [x] | ? | Colour spaces and PDF functions — all four function types, spot colour rendered through the document's own tint transform. **No ICC engine**; `ICCBased` falls back to `/Alternate`, disclosed. |
| [x] | — | [x] | ? | Calibrated `DeviceCMYK`→sRGB, one conversion site document-wide. |
| [x] | — | [x] | ? | Transparency — `/SMask`, `/Mask` (soft, explicit, colour-key) and `/Matte`. |
| [x] | [x] | [x] | [x] | Insert an image as a new XObject, including GUI drag-and-drop. |
| [x] | [x] | [x] | [x] | Optional content (layers) affects rendering — content-stream and XObject `/OC`. |
| [x] | [x] | [x] | — | Glyph-coverage gate on the app's own UI chrome. |

### Reading, navigation & printing

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [x] | [x] | [x] | [x] | Find text on the page. |
| — | [x] | [x] | [x] | Read and navigate an existing bookmark tree. **No authoring.** |
| [x] | [x] | [ ] | [x] | List embedded attachments. **Cannot extract their bytes**, and no GUI surface. |
| [x] | [x] | [x] | ◐ | Print — enumerate printers, spool a real job honouring the operator's CMYK intent and the sheet orientation it's planned against (the GUI preview turns with it), with duplex, copies, page subsets and a four-way comments-and-forms filter; GUI dialog is tabbed, resizable with both scrollbars, previews real page content zoomable/pannable, and opens on Ctrl+P. |
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
| — | — | [x] | ? | Independent edit toggles plus one master edit switch. |
| — | — | [x] | ? | Multi-target verbs act on the whole selection or refuse — never a silent subset. |
| — | — | [x] | — | Implicit gesture commit — clicking away accepts an in-progress edit; Enter commits a typing draft. |
| — | — | [x] | — | Nothing floats over the canvas. |
| [x] | [x] | [x] | — | SVG icon set covering every GUI feature. |
| [x] | [x] | [x] | ? | Persisted user settings. |
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
| [x] | [x] | [x] | [x] | **Encryption** — RC4 (40–128 bit), AES-128 (`/AESV2`) and AES-256 at `/R` 5 (`/AESV3`) decrypt read-only, including the empty-user-password case every other reader opens silently; CLI (`--open-password`/`--open-password-file`) and GUI (inline canvas prompt) can supply a password for any of the three. All eight Table 22 permission bits shown read-only in Properties > Security, captioned declared-by-the-author and unenforced; a `/Perms` mismatch (possible only at `/R` 5) is reported, never refused on. `/R` 6 is still refused by name — its Algorithm 2.B is unsourced. **Writing an encrypted document is still unimplemented in every configuration.** |
| — | — | [ ] | [x] | Imposition in the GUI — needs the sheet composition extracted into `pdfce-print` so both shells share one implementation. |
| [ ] | [ ] | [ ] | [x] | Move and resize anything carrying a `/Rect` — widgets, markup, redaction marks, links, ce dimensions. |
| [ ] | [ ] | [ ] | ? | Resize a vector object. |
| ◐ | ◐ | [ ] | **[ ]** | Node-editing remainder — Tab cycling and arrow-key nudge. |
| [ ] | [ ] | [ ] | [x] | Make ce dimensions and foreign annotations selectable and deletable from the canvas; also gates click-a-comment-to-select. |
| [ ] | [ ] | [ ] | **[ ]** | ce-dimension tolerances, SolidWorks-style (bilateral, symmetric, limit, min/max). |
| [ ] | [ ] | [ ] | ? | Drag a ce dimension's extension lines. |
| [ ] | [ ] | [ ] | ? | Note-text authoring for geometric markup. |
| [ ] | [ ] | [ ] | [x] | True in-place page insertion, so Insert edits the open document. |
| [ ] | [ ] | [ ] | ? | Wide/batch CSV — one row per document, for filling many copies of one form. |
| [ ] | [ ] | [ ] | ? | Static-XFA hybrid — read and fill the XFA half in step with AcroForm. |
| [x] | [ ] | [ ] | [x] | Extract an embedded attachment's bytes to disk. |
| [ ] | [ ] | [ ] | [x] | Links and bookmarks authoring — create, edit, reorder, named destinations. |
| [ ] | [ ] | [ ] | [x] | Redaction: mark by dragging on the canvas; Sanitize / Remove Hidden Information. |
| [ ] | [ ] | [ ] | [x] | Digital signatures — PKCS#7/PAdES signing and verification. |
| [ ] | [ ] | [ ] | [x] | Bates numbering. |
| [ ] | [ ] | [ ] | [x] | PDF/A conformance and validation. |
| [ ] | [ ] | [ ] | [x] | OCR. |
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
- **Any network call without explicit, disclosed opt-in** — enforced by
  a fail-closed CI gate. Rules out telemetry and silent update checks.
- **Executing embedded JavaScript** — a sandboxed engine is prohibited
  by standing rule. Recognised built-ins are reimplemented natively
  instead.
- **Barcode form fields.**
