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
| [x] | [x] | [x] | [x] | Recovers a damaged/malformed cross-reference table; tolerates common lenient-PDF defects (Pass 13a/13b, `/Contents`-defect fix, missing-`endobj` repair `49dfe81`) |
| [x] | [x] | [x] | [x] | Rotate pages, incl. refusal disclosure (Pass 3.2 core/CLI; GUI rotate buttons + `5b2682b` disclosure fix) |
| [x] | [x] | [x] | [x] | Delete pages (Pass 3.2 core/CLI; GUI `delete_pages` call in `main.rs`) |
| [x] | [x] | [x] | [x] | Reorder pages — thumbnail drag or keyboard (Pass 3.2 core/CLI; GUI `apply_reorder`/`move_pages_keyboard` in `main.rs`) |
| [x] | [x] | [x] | [x] | Merge / combine multiple files into one (Pass 3.2 core/CLI; GUI `merge_tool` add-files button in `main.rs`) |
| [x] | [x] | [x] | [x] | Split a document into multiple files, `EveryN` criterion only (Pass 3.2 core/CLI; Pass 3.4 GUI) — break-point and bookmark criteria still unbuilt, named in the panel |
| [x] | [x] | [x] | [x] | Insert pages from another file — whole source, Start/End only (Pass 3.2 core/CLI; Pass 3.5 GUI) — writes a NEW FILE, not an in-place edit; true in-place insert needs `EditSession::insert_pages` (core-only, unbuilt — see *Planned*) |
| [x] | [x] | [x] | [x] | Extract pages to a new file (Pass 3.2 core/CLI; GUI `PdfceApp::extract_selection`, `main.rs` ~L3601, reached via `Action::ExtractSelection` from a thumbnail-rail button ~L8936 — the GUI names it "extract selection," not "extract pages," which is why a name-based grep for `extract_pages` missed it; see R156) |
| [x] | [x] | [x] | [x] | Edit document metadata (`/Info` dict) (Pass 3.2 `set-info`; GUI Properties dock panel) |
| [x] | — | [x] | [x] | Undo/redo command log (Pass 3.1 core; wired into the GUI at Pass 14.3) |
| [x] | [x] | [x] | [x] | Save (incremental-by-default or full-rewrite); **"Save a copy" only — no true in-place overwrite yet** (see *Planned*). **Full rewrite REFUSES a file whose highest object number exceeds `MAX_REWRITE_OBJECT_NUMBER` = 8,388,607** (Annex C Table C.1 max indirect objects; `WriteError::ObjectNumberTooLarge`, `0df6158`) — §7.5.4 makes a one-section xref table cost O(largest object NUMBER), so pdfium's 1.2 KB `bug_455199.pdf` (which names `2147483648 0 obj`) would need ~40 GB of entries and used to grind for about an hour. **A refusal, not a repair** — emitting a sparse table would be malformed, renumbering would break byte-identity. **Reading such a file is unaffected** (`inspect`/`extract-text` both work); the bound refuses nothing a conforming producer can write, since 2³¹ exceeds Table C.1's integer maximum. **A full rewrite also DROPS any bytes before `%PDF-`** (`fa4f83c`) — veraPDF reads xref offsets as *header-relative* whenever a preamble exists, so every preamble-preserving file pdfce wrote was unreadable to it even with offsets absolute and correct; emitting the header at byte 0 makes both readings the same number. **Incremental and identity-append saves still preserve the preamble** (they promise byte identity / a byte prefix; a full rewrite promises only per-object identity). See `ARCHITECTURE.md` §5.6.1 |
| — | — | [x] | [x] | **Multiple documents open at once** (Pass 39.0) — a parked list beside the active document; each parked document keeps its own unsaved edits, selection and scroll position. The switcher row appears only at 2+ documents. `cli` `—`: a one-shot batch tool has no "open documents". **Restriction:** `redact_search_query` is app-level, not per-document, so a redaction search query follows you across a switch |
| — | — | [x] | [x] | **Close a document, with a three-way save prompt** (Pass 39.1) — save-and-close / close-without-saving / keep-open. **Save-and-close only closes if the save SUCCEEDED** (a cancelled dialog or refused write leaves the document open); a clean document closes with no prompt at all |

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
| [x] | [x] | [x] | **[ ]** | Move a node (anchor point), incl. `re` rectangle corners and reused subpath starts (Pass 26.0, 30.0). **Pass 26.3** (`f8bbdd4`) claims grab-without-descent, a hover mark state and live drag preview — **build record OWED**, see `ROADMAP.md` ⚠ FILING GAP #2 |
| [x] | [x] | [x] | **[ ]** | Delete a node (Pass 36.1) |
| [x] | [x] | [x] | **[ ]** | Edit a Bézier handle / control point (Pass 30.1 core+CLI; Pass 26.1 GUI; Pass 26.3 grab/hover/live-preview — build record owed, as above) |
| — | — | [x] | **[ ]** | Level-ladder rung readout (Object/Part/Point) + Escape-to-ascend navigation (Pass 26.0, 36.2) — no clickable breadcrumb yet |
| — | — | [x] | — | Level survival across an edit: an edit/undo/redo truncates your rung one step at a time instead of ejecting you to Page (Pass 26.2). `Acrobat` `—`: Acrobat has no level ladder, so the question is not meaningful. Ladder proven by unit test; the entry gesture is **not** driven end-to-end |
| — | — | [x] | **[ ]** | **Select MULTIPLE nodes** (anchor points) — additive under Shift **or** Ctrl/Cmd, toggle-off on a re-click, every selected anchor drawn as selected (Pass 41.0; discharges part of Pass 23.3). **Restriction: you cannot MOVE them together** — needs a `move_nodes` core verb that does not exist, and an N-call loop would break one-gesture-one-undo. **Verification limit:** the Node rung is unreachable through the scripted harness (injected clicks never become double-clicks), so the rule is unit-tested, not driven in-app |
| — | — | [x] | [x] | **Modeless select/edit — no tool armed IS the object-edit tool** (Pass 40.0): click selects, double-click descends, drag on the selection moves it, drag on empty paper is still a marquee. Creation verbs (Add Text, Measure) keep explicit arming, because "place new text here" cannot be disambiguated from a marquee by what is under the pointer. **Clicking a TEXT object still does not begin text editing** — operator question (bc) is **RESOLVED**: Edit Text stays a toggle, but the toggles are now independent (Pass 42.0), so arming it costs you nothing else |
| — | — | [x] | ? | **Object tree sidebar — nested object → subpath → node** (Pass 43.0), toggled from View ▸ Objects, dockable and tabbed. Clicking a row **sets the canvas level**: a row's `(object, subpath, node)` triple *is* an `EnteredObject`, so tree and canvas agree by construction. Front-most-first; virtualized, so a collapsed tree costs exactly the object count. **Restrictions: no marked-content / OCG grouping** — deliberately refused, that structure is not decomposed yet (Pass 23.2's core half is unbuilt) and inventing it would misdescribe the document; **scroll-reveal follows a selected OBJECT only**, not a selected subpath or node |

### ce dimensions

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [x] | [x] | [x] | ◐ | Author a ce dimension with snapping: linear/radius/diameter (Pass 12.M1, 12.M2, 12.M2b) |
| [x] | [x] | [x] | **[ ]** | ce-dimension groups: scale, number format (decimal/fraction), ANSI/ISO drafting standard (Pass 12.M2, 27.2; GUI controls in Properties since Pass 34.2 — **reached from the ribbon's File tab and rendered in the Tool compartment since Pass 43.0**) — group-wide: a change regenerates every member's `/AP` |
| [x] | [x] | [x] | **[ ]** | Change a PLACED circular ce dimension between the radius and diameter reading (Pass 34.2, `EditSession::set_dimension_display` / `pdfce-cli dimension-display`) — **circular only**; a linear target is refused by name (`EditError::NotACircularDimension`). Per-object: regenerates exactly one `/AP` |
| [x] | [x] | [x] | [x] | Drag to reposition a ce dimension (Pass 25.5) |
| [x] | [x] | [x] | ? | Edit a placed ce dimension's placement NUMERICALLY — standoff / value-position (Pass 27.1 core+CLI `dimension-offset`; Pass 34.2 GUI spinners, which mirror the drag rather than replace it) |
| [x] | [x] | [x] | [x] | Toggle a ce-dimension group's OCG layer visibility (Properties since Pass 34.2 — **ribbon-activated, rendered in the Tool compartment since Pass 43.0**; the floating Group Manager window is gone, closing R81's last named holdout) |
| [x] | [x] | [x] | [x] | Delete a ce dimension: annotation + `/AP` + `/PieceInfo` sidecar, together (Pass 25.6) |

### Annotations & markup

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [x] | [x] | [x] | [x] | Render & count annotations/widget appearances, honouring the annotation-flag set (Pass 6.0) |
| [x] | [x] | [x] | [x] | Author geometric markup: Ink/Square/Circle/Line/Polygon (Pass 6.1) — minimal menu affordance only; no canvas drag/freehand drawing yet |
| [x] | [x] | [x] | [x] | Author text-bearing annotations: FreeText/Stamp with §12.7.3.3 variable-text appearance (Pass 6.2) |
| [x] | [ ] | [x] | [x] | **Read an annotation's note text, author and modification date** — `/Contents` (via the §7.9.2 text-string decoder, so UTF-16BE is right), `/T` (**Table 170, markup-only** — `None` means "this subtype has no such concept", never "anonymous"), `/M` (**stored RAW**, because §12.5.2 types it "date *or* text string" and obliges readers to accept any format) (Pass 38.4 core). `cli` `[ ]`: `list-annotations` does not yet print `contents=`/`author=` — **cheap now the fields exist**, Pass 38.5 |
| — | — | [x] | [x] | **Comments panel — browse every note and markup in the document** (Pass 38.4): subtype · page · author · note text, with a Go-to button per row. Widgets and popups excluded; **ce dimensions ARE listed** (they are `/Line` annotations, and excluding by subtype would also hide a genuine hand-drawn `/Line` markup). When *every* row lacks a note the panel says why, once, at the top — because pdfce's own markup authoring never sets `/Contents`, so a correct list would otherwise read as a broken one. **Restrictions: no Delete** (needs the `delete_annotation` core verb — omitted, not greyed, per R83) and **no click-to-select-on-canvas** (needs Pass 22.0) |

### Forms (AcroForm)

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [x] | [x] | [x] | [x] | Fill form fields — core/CLI: text, checkbox, radio, choice; **GUI: text + checkbox only** (Pass 7.0, 7.1 core/CLI; Pass 37.2 GUI Forms panel, P0) — radio groups and choice fields are recognised, listed, and disabled-with-a-reason (P1), as are read-only, signature and pushbutton fields. **Rich-text fields are REFUSED in the GUI** (filling would silently discard stored formatting) — core/CLI do NOT yet refuse; see `ROADMAP.md` *Next up* |
| [x] | [x] | [ ] | [x] | Flatten a form to static page content (Pass 7.1) — GUI is Pass 37.2's P1, not built |
| [x] | [x] | [ ] | [x] | Import/export form data, FDF/XFDF (Pass 7.1) — GUI (Batch Tools) is Pass 37.2's P1, not built |
| [x] | [x] | ◐ | [x] | **CREATE a form field — text, check box, **radio**, choice — with a name-collision RESOLVER** — `add_text_field` / `add_check_box` / **`add_radio_button`** / `add_choice_field` over `forms_author::resolve_field_path`, CLI `add-text-field` / `add-check-box` / **`add-radio-button`** / `add-choice-field` (Pass 20.1 PARTIAL `8e799e9`; Pass 20.2 + 20.3 PARTIAL `bca60c9`; **Pass 20.0 + Pass 20.1 (completion) `a3d885b`+`f809857`**; **Pass 20.2 COMPLETE `69ab966`+`834d256`+`817b268`**; all 2026-08-07). Field+widget+`/AP`+`/Annots`+`/Fields` land as ONE undo. Check boxes get a keyed `/AP /N` sub-dict with a **vector-drawn** check (R92 forbids a second, symbolic-font generator); choice fields carry `/Opt` export↔display pairs and Combo/Edit/MultiSelect/Sort, and `Sort` **sorts the array**, not just the flag. **Same-name adds now MERGE rather than refuse** — the resolver classifies every add as CREATE (vacant), MERGE (same-type terminal, incl. Shape A→B promotion), or one of two refusals (different-type collision; name held by a grouping node), all four outcomes live across all three verbs. Refuses XFA, any certified document (strict gate — creation is structural), a degenerate rect, an empty name, `Off`/empty as a check box's on-state, `Edit` without `Combo`, a duplicated `/Opt` export. **RADIO GROUPS are built OUT OF the merge primitive, not by a radio-specific path** — three `add-radio-button` calls sharing one name make ONE field with THREE widgets (§12.7.3.2 grouping by shared FQN), and the already-shipped `set_button_state` supplies mutual exclusion untouched. Round widget = pdfce's own stated design choice (the spec distinguishes radio by `/Ff` bit 16 alone), because a group drawn as squares lies about its behaviour. Refuses duplicate export values **unless `RadiosInUnison`**, refuses cross-KIND merge (check box ↔ radio), and refuses **positional-`/Opt` groups BY NAME** (Table 227's index semantics are unresolvable on pdfce's write side — decision 020 §8.3's refusal branch, chosen). Group flags (`NoToggleToOff`/`RadiosInUnison`) are **DISCLOSED, not applied**, since a joining member must not silently rewrite how existing members behave. **Limits: `/MK` borders do NOT paint in pdfce** (R43) — so a check box shows a box and a choice field does not; **no `/I`/`/TI`, no push buttons, no field PROPERTY editing** (rename/reflag/move/resize). **`/TU` is now MANDATORY-OR-DECLINED** (`TooltipChoice` — R105; `EditError::TooltipDecisionRequired` refuses omitting both `--tooltip <text>` and `--no-tooltip`; declining writes no `/TU` at all, not an empty one) **and the tagged-document + `/Tabs /S` untagged-field disclosures now fire** (decision 020 §3.5.3/§3.4.3, `FieldAuthorDisclosures`/`report_field_disclosures`, all shared across the three verbs) — both closed 2026-08-07 by `50a5461`. **Note: this is the DISCLOSURE only — `/Tabs` tab-order AUTHORING is still not built (Forms P2, below).** **CLI verb shape RULED 2026-08-07: the three flat `add-*` verbs STAND; decision 020's `forms add-field --type …` is SUPERSEDED** (52 flat subcommands in `pdfce-cli`, zero nested — `docs/decisions/020-form-field-authoring.md` §0). No longer an open item. **`gui` is `◐`, not `[x]`, as of Pass 20.5 PARTIAL (`8a8678e`+`165dd49`, 2026-08-07):** all four types can be **placed** on the canvas (`CanvasTool::PlaceField`, new ribbon group beside Forms; click = type-dependent default box, drag = explicit; auto-name **scans and cannot collide**, unlike Acrobat's session counter, because pdfce merges same-name same-type fields; R105 tooltip gated in the pane; merge outcome disclosed only **after** Accept, since the resolver runs inside the core verb). **What the GUI cannot yet set: the per-type detail fields — multiline, initial state, and the choice option editor — all deliberately CUT.** Push button is **absent rather than greyed** (R83/R124 — its verb name is still NOT RULED). |
| [x] | [x] | [x] | [x] | **DELETE a form field or a single widget** — `EditSession::delete_field` / `delete_widget`, CLI `delete-field` / `delete-widget` (Pass 20.2 completion, `817b268`, 2026-08-07; decision 020 §3.6.3). Un-lists the widget from its page `/Annots`, drops it from `/Kids`, deletes the object, and prunes grouping nodes left childless (a named node with nothing under it still occupies its §12.7.3.2 FQN slot and would refuse a later field). **The rule that earns its code: deleting the widget whose on-state is the field's `/V` leaves `/V` naming a state no remaining widget can display** — a malformed field that parses perfectly — so `/V` and every surviving kid's `/AS` go to `/Off` **and the operator is TOLD** (`selection_cleared=1` + prose; §3.6.3 says silence either way is the sneaky outcome). Last-member `delete-widget` **delegates to `delete_field`** so the two paths cannot disagree about what *gone* means. **No Shape-B→A collapse on the way down** (R102) — a 3→1 group keeps its `/Kids` parent, because deletion has no business rewriting object identities nobody asked it to change. Two verbs, not one optional `--index`, deliberately. ~~**`gui` stays `[ ]` BY CHOICE, not by oversight:** the Forms-panel deletion surface was scoped into Pass 20.5 and **deliberately cut** — *"half of both is worse than all of one"* — and stays owed under Pass 20.5's own ID.~~ **`gui` IS NOW `[x]` (Pass 20.5 addendum, `fc51786`+`69db1c6`, 2026-08-07):** the Forms panel gives one widget a single "Delete field" and several widgets a row each plus "Delete entire field" — **which verb fires is decided by the row level clicked**, not by a checkbox. **Read-only, signature and push-button rows get the control too** (the panel used to skip them for being unfillable — deletability is a different question; **R83 amended**). Both `FieldDeletion` disclosures are reported (`selection_cleared` and `emptied_parents`). No confirm gate — decision 024 §4.4 — with a tooltip for the one unpredictable consequence (removing the widget holding a radio group's selection). Gated by a NEW `EditSession::deletion_refusal()`, the **strict** certification gate, not `fill_refusal`'s `/P`-aware one: **on a certified fillable form at `/P 2` the fill control is enabled and the delete control is disabled**, and until this commit the GUI had no way to ask. **Still owed under Pass 20.5: the per-type detail fields, and the rendered appearance is not visually verified.** |

### Redaction & security

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [x] | [x] | [x] | [x] | Mark redactions by text search or named region (Pass 8.0, 8.1) |
| [x] | [x] | [x] | [x] | Mark redactions by PATTERN — `#` = any digit, `?` = any character, so `###-##-####` marks every SSN-shaped run in one action (Pass 8 core/CLI `redact-mark --pattern`; Pass 37.0 GUI `Match: Exact text \| Pattern` switch over the existing query box) |
| [x] | [x] | [x] | ◐ | Apply redaction with a runtime-verified true-removal proof (Pass 8.0, 8.1) — Acrobat applies, but gives the operator no automated proof it removed what it claims to |

### Fonts & rendering

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| [x] | — | [x] | [x] | Rasterize page content: vector paths, text, images (Pass 1, 1.1). **Large CAD drawings are SLOW but no longer pathological** (`76200e9`, 2026-08-07): a 129,515-path ArchiCAD sheet went 1× 32,313 → **18,870 ms** (−42%), 2× 447,862 → 214,714 ms (−52%), **output byte-identical**. **Painting every path is 0.87 s of that — the clip machinery was 95% of render time**, and the fix removed a page-sized `clip.clone()` per paint (~108 GB of memcpy for one page) and bounded `intersect_clip`'s multiply to the path's own device bounds (an identity, not an approximation). **Parse is not the cost at any scale** — read + parse + page tree ≈ 5 ms, ~0.005%. ~~**Still ~18 s, not interactive** — see *Planned*~~ **★ IMPROVED AGAIN (`4475fe6`, 2026-08-07): `GraphicsState.clip` became `Arc<Mask>` — a clip is never mutated in place, so `q` needs a reference, not a buffer. `q`/`Q` clone cost 6.80 s → 0.01 s; 1× 17.47 → **10.18 s**, 2× 214.71 → **51.52 s**. From the original baseline that is **~3× at 1× and ~8.7× at 2×**. **Byte-identical on the CAD sheet AND 52 synthetic fixtures** (JPX, bilevel, annotations, text, vector, CMYK — the CAD page has zero images and 242 text elements, so it cannot carry a no-pixel-changes claim alone). **`Arc` not `Rc` so `GraphicsState` stays `Send`** for future off-thread rendering. **Still ~10 s, not interactive** — see *Planned* |
| [x] | — | [x] | [x] | Image codecs: DCT/JPEG, LZW, RunLength, CCITT Fax, JBIG2, JPEG 2000 (Pass 2.1–2.3) |
| [x] | [x] | [x] | — | Glyph-coverage gate on the app's own UI chrome — no tofu glyphs in pdfce's own strings (Pass 18.7) |

### Shell & UX

| core | cli | gui | Acrobat | Feature |
|:----:|:---:|:---:|:-------:|---------|
| — | [x] | — | **[ ]** | A first-class scriptable CLI (`pdfce-cli`) over the capabilities above — Acrobat has no CLI at all (Action Wizard, embedded JS and COM only) |
| [x] | [x] | [x] | — | Live-edit canvas: renders the edited revision, not a static page image (Pass 17.x) |
| — | — | [x] | — | Interactive canvas: pan, zoom-to-cursor, marquee select (Pass 12.0, 18.8) |
| — | — | [x] | — | Dockable panel shell — **the left dock is Pages + Tool, no tab bar** (Pass 18.1, 34.1, 34.2, 38.2, 38.4, **43.0**). **The ribbon owns the surfaces and the Tool compartment shows them**: Properties (File), Forms (Edit), Comments (Review), Batch Tools + Redact (Tools), and the measure options, each rendering in Tool when activated, with a named "Back to tools" exit. Properties still holds the same three scope-named sections (selected ce dimension · ce-dimension groups · document `/Info`); no persistent floating window remains (R81). **Pass 43.0 partially reversed 38.2's four-compartment layout** — R157 (*watched state gets a compartment, entered workflows share one*) survives; **Properties was on the wrong side of it** |
| — | — | [x] | ? | **Independent edit toggles + one master edit switch** (Pass 42.0) — Edit Text, Add Text, Obj and the measure tools can all be **on at once**; a click is resolved by a documented, **enable-order-independent** precedence ladder (TextEdit → AddText → measure → VectorEdit), and the Tool pane **says which tool has the canvas**. One switch turns **all** editing off, covering the ce-dimension drag, form filling and redaction marking too — each disabled-and-explained, so the document still reads. Turning editing off **keeps the tool set**. **Restriction: the three measure tools remain mutually exclusive AMONG THEMSELVES** (they share one state struct, so two-on is a state with no meaning) — they are fully independent of the text and object tools |
| — | — | [x] | — | **Dock density convention** — one row-spacing constant applied at the single chokepoint every pane renders through (Pass 38.1, `UI_PREFERENCES.md` §11). Measured: 285 label rows before and after, 11 px reclaimed — same ink, shorter panel. **Three refusals are written into the convention** so a later pass cannot spend them: click targets never shrink (`interact_size.y` stays 18.0 — density comes from the gap, not the control), no text gets smaller, and no explanatory line is deleted to save a row (those lines are the screen-reader surface) |
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
| [ ] | [ ] | [ ] | — | Pass 38.5 — shell-redesign P1, **now the only unbuilt shell-redesign slice**: general `delete_annotation` core verb (three named hazards: dangling `/AcroForm /Fields`, `/Popup` companions, `/IRT` reply chains) + the Delete row action it unlocks; `pdfce-cli list-annotations` `contents=`/`author=` — **the cheapest item left, since Pass 38.4 added the core fields**. **Two of 38.5's four items were INVALIDATED by Pass 43.0** — the four-compartment default-height tuning and the four-compartment collapse-state reset both named a layout that no longer exists (the left dock is Pages + Tool); they need an engineer re-scope or a retirement. (Pass 38.1 · 38.2 · 38.4 have shipped — see *Implemented*. Pass 38.3 is the property-4 confirmation, an operator question rather than a capability, so it has no row here) |
| [ ] | — | — | ? | **Interactive-speed rendering of a large CAD drawing** — 1× is **10.18 s** at `4475fe6` (was 18,870 ms at `76200e9`, 32,313 ms originally); the reference product is *"feels instant"* (uninstrumented; ~1.6 s cold-to-sharp on an M4 Pro). **No Pass ID — deliberately, and this row is why: a number minted a commit ago would now point at a dead end.** ~~(1) stop allocating a page-sized mask per clip — `Mask::new` is **10.1 s of the remaining 18 s**, and most clips are `re W n` rectangles needing no mask at all~~ **★ BOTH HALVES FALSIFIED 2026-08-07: `Mask::new` is 1.02 s, not 10.1 s (an R164 instance — the figure came from an ablation that measured construction *plus* use), and only 612 of 24,128 clips — 2.5% — are rectangles. The rectangle special-case was DECLINED ON MEASUREMENT, not built.** ~~Revised order: **(1′)** size the mask to the clip, not the page — clips are **100% single-subpath, mean 7 segments, mean bbox 0.663% of the page**, so the mismatch is one of *extent*, not *shape*~~ **★★ 1′ IS RETIRED 2026-08-07 (`6b33789`) — THE PREMISE WAS WRONG BY 100×: mean clip bbox is 66.36% of the page, not 0.663%** (a fraction printed as a percent; first clips measure 87/65/100/81/95%). **THREE INDEPENDENT REFUTATIONS**: *size* (a mask sized to a 66%-of-page clip **is** page-sized); *API* (tiny-skia's `RasterPipelineBlitter::new` returns `None` on a mask/pixmap size mismatch — a `log::warn!` and a **silently dropped paint**, so it would have stopped painting, not sped up); *cost* (`Mask::fill_path` is 10.3 µs on a 64×64 mask vs 8.3 µs page-sized — dominated by three raster-pipeline compilations, and `scan::path_aa` already bounds to `path.bounds()`). **The clip-representation line of attack is CLOSED.** Live order is now **(2′)** the cache cliff — **measured and partly discharged**, 1×→2× was 14.1×, now **5.1×**, still above the ~3.2× a pixel-quadrupling costs elsewhere — then **(3)** tiling/threading **last, unchanged**: painting every path is only **0.87 s**, so they would today optimise 5%. `cli`/`gui` `—`: the work is entirely in `pdfce-render`, so **both shells get it with no change of their own** — not "no benefit". **`6b33789` changed NO timings — 1× is still ~10 s; it bought a correction and `tools/render-profile`, the standing instrument that makes a second measurement cheap. No fork is live as of that commit (operator-stated)** **★ THIS ROW IS NOW GOVERNED BY `R166` (minted 2026-08-07): a number whose instrument no longer exists may be reported but may NOT scope, order or build anything — so the live order below (2′) then (3) may only be re-ranked on figures `tools/render-profile` can reproduce.** **★★ THE FLOOR IS MEASURED 2026-08-07 (`fa17d54`, `--ablate-sweep`): 0.49–0.53 s while pixels vary 64× — SCALE-FLAT, PER-OPERATION (148,517 operators). Complete map at 1×: interpreter floor 0.5 s · painting ~0.8 s · mask sampling FREE at the noise floor · CLIP CONSTRUCTION ~8.4 s = 86%, which reproduces the earlier per-phase sum (5.24+2.26+1.02 = 8.52 s) within 4% BY A SECOND METHOD — so this row's ordering is now `R166`-clean. (3) is REFUTED as an answer, not merely last: tiles render fewer PIXELS, not fewer OPERATORS. And at 0.25× the FULL render is 2.57 s, not 0.67 s, so A LOW-RESOLUTION PROXY IS BOUNDED BELOW BY ~2.6 s — proxies and progressive refinement help LESS than pixel count suggests, and clips bind either way. The non-clip work sums to ~1.3 s, inside the reference's ~1.6 s — but that is ARITHMETIC OVER SEPARATELY MEASURED PARTS, NOT A MEASUREMENT (R164 applies); nobody has rendered this file in 1.3 s.** **★★★ THE 86% IS BROKEN DOWN 2026-08-07 (`110b8c9`, TIMED not ablated): `Mask::new` 1.03 s (11.8%) · `fill_path` 5.22 s (216 µs/clip, 59.9%) · multiply 2.46 s (28.3%) = 8.72 s; sum + floor = 9.26 s against a 9.49 s render — THE ARITHMETIC CLOSES. TWO FIGURES CORRECTED: `fill_path` was filed at 8–10 µs/call and is 216 µs (22× — the original experiment varied the BUFFER while the cost is set by the PATH'S EDGES; the conclusion it supported, that buffer size does not matter, is STILL TRUE, so item 1′ stays retired), and PAINTING IS ~0.27 s, NOT 0.87/0.8 s (those were floor + painting — R164's third instance in one day), so (3) tiling/threading addresses UNDER 3%, not 5%. ★ THE PER-CLIP DISTRIBUTION IS UNIFORM — 85.0% in 256–512 µs, p90 and p99 both <1024 µs, only 108 of 24,128 over 1 ms and only 36 under 256 µs. NO TAIL AND NO HEAD, so THERE IS NO SPECIAL CASE TO FIND AND ANY FIX MUST CHANGE THE WORK FOR ALL 24,128 CLIPS. ★ `fill_path` GROWS 2× PER 4× PIXELS — it tracks the LINEAR dimension (the scanline converter follows the path's PERIMETER), and at 0.25× it is STILL 56% of the whole render: the MEASURED reason culling and proxies underdeliver. LIVE CANDIDATE: dedup/cache built clip masks — BLOCKED, deliberately, on censusing how many of the 24,128 are re-applications of an already-built clip path BEFORE anything is built (R166 applied prospectively).** |
| [ ] | [ ] | — | **[ ]** | **`move_nodes` core verb** — move a multi-node selection as ONE surgery and ONE undo entry (no Pass number yet; filed 2026-08-06 with Pass 41.0, and the same obligation Pass 23.3 calls `plan_move_nodes`). The GUI selection set exists; **this is the only thing between it and multi-node drag**, and an N-call `move_node` loop is refused because it would break one-gesture-one-undo. `gui` `—`: no new GUI surface needed, the gesture is already there |
| [ ] | [ ] | [ ] | ? | **Note-text authoring for geometric markup** — Pass 6.1 markup never sets `/Contents`, which is why the Comments panel shows "No note text" on every pdfce-authored annotation. Shell-redesign spec §8 item 12; **not scoped to a Pass** |
| [ ] | [ ] | [ ] | **[ ]** | Pass 35.0 — ce-dimension tolerance & tolerance types, SolidWorks-style (None/Basic/Bilateral/Symmetric/Limit/Min/Max) — zero existing representation today. **Next up**: Pass 34.2 built the per-ce-dimension property surface its controls need to live in |
| [ ] | [ ] | [ ] | ? | Pass 35.1 — drag a ce dimension's extension lines to extend/retract |
| [ ] | [ ] | [ ] | — | Pass 33.0 — real fix for reflow's auto-detected wrap width inheriting a prior edit's overflow (disclosure ships today; the fix itself — clamp / median-width / refuse — is undecided) |
| [ ] | [ ] | [ ] | ? | Pass 32.0 — delete one text run without deleting every run sharing its text object (fixes: deleting one CAD label deletes all 237 sharing a `BT`…`ET` block on the operator's own drawing) |
| ◐ | ◐ | [ ] | **[ ]** | Decision 028 remainder — Tab/Shift+Tab node cycling; **arrow-key node nudge (items 10/11 — still owed for a SINGLE node, which is why there is nothing for a multi-node nudge to extend; scope it with `move_nodes` above)**; readout-row corrections; clickable breadcrumb navigation. (**Pass 26.2 SHIPPED** 2026-08-06; **Pass 26.3** — the grab/hover/live-preview plan items — **code committed `f8bbdd4`, build record owed**. Both moved to *Implemented*) |
| [ ] | [ ] | [ ] | [x] | Pass 22.0 — make ce dimensions and foreign annotations selectable, marqueeable, and deletable from the canvas. **Also gates the Comments panel's click-a-row-to-select-on-canvas** |
| [ ] | [ ] | ◐ | ◐ | Pass 23.0–23.3 — ce-dimension format/units GUI control; re-measure + whole-dimension move; descend into nested form-XObject containers; multi-node select/move/delete. **23.3 is PARTLY DISCHARGED:** node *selection set* shipped as Pass 41.0, node *delete* as Pass 36.1; **only multi-node MOVE remains**, and it is core work (`move_nodes`, above) |
| — | — | [ ] | — | Pass 24.0, 24.2–24.5 — remaining ribbon slices (fixed-anchor confirm strip, contextual tool tabs, selection tabs, overflow/collapse, keyboard & accessibility) |
| [ ] | [ ] | [ ] | [x] | `EditSession::insert_pages` — true in-place page insertion (no Pass number yet; filed 2026-08-05, Pass 3.5's ship) — the core command that would let Insert edit the open document instead of always writing a new file |
| ◐ | ◐ | ◐ | [x] | Form field creation/authoring — **IN PROGRESS since 2026-08-07** on the operator's direct instruction (*"get form field creation/editing done next"*, which answers open question **(m)**). **Text, check box, RADIO and choice creation, WITH a name-collision resolver, have shipped** (core+CLI — moved to *Implemented*), **as has field/widget DELETION**: Pass 20.1 PARTIAL, Pass 20.2 + 20.3 PARTIAL, Pass 20.0 + 20.1(completion), **Pass 20.2 COMPLETE** (`69ab966`+`834d256`+`817b268`, 2026-08-07). **Decision 020's F2 is therefore DONE in full** — checkbox, radio and deletion — and **§8.3's positional-`/Opt` item is discharged as a reasoned REFUSAL**, which §8.3 permits as an equal outcome. **`/TU` R105 handling and the `/Tabs` disclosure SHIPPED 2026-08-07 (`50a5461`)** — moved to *Implemented*, above. **Still planned:** `/I`/`/TI`, push buttons (F3's remainder — Pass 20.3 stays PARTIAL), tab order AUTHORING (F4, blocked on a spec-RAG `/Tabs` dispatch — distinct from the disclosure, which is done), the GUI surface (F5, needs a UI-specialist dispatch first), field **property editing** (F6 — where F2's `--defaults-from` was deferred to). **The CLI verb-shape ruling is now MADE (2026-08-07): flat verbs stand, decision 020's `forms add-field --type …` superseded** — so the radio/push-button/deletion verbs will be flat too. **Verb NAMES also ruled 2026-08-07 (decision 020 §0.1): radio is `add-radio-button`** (member-shaped, matching `add-check-box`; `add-radio-group` would misdescribe §6's one-call-per-member design) **and deletion is `delete-field` / `delete-widget`** (`delete` is the house word — five shipped `delete` verbs, zero `remove-*`; forms is a verb-first domain). ~~These are names only — **nothing here is built yet.**~~ **BOTH ARE NOW BUILT** (2026-08-07, Pass 20.2 complete). **The push-button verb name is still NOT ruled, and must not be inferred from `add-radio-button`** — R161 supplies the *shape* (forms is verb-first, so `add-<thing>`), not the word. **Still owed to the engineer:** F0's disposition (owed retroactively / deferred / retired). **★ AMENDED 2026-08-07 (eighth filing): the GUI surface (F5) is NO LONGER wholly planned — Pass 20.5 is HEADED and PARTIAL** (`8a8678e`+`165dd49`): the **creation** surface is built (see *Implemented*), and what stays owed under that same ID is the **GUI deletion surface** and the **per-type detail fields** (multiline, initial state, choice option editor), **both deliberately CUT rather than missed**. **One verification is OWED and is not a formality: the rendered appearance was never visually checked** (the operator was at the machine; a capture would have photographed his desktop) — three defects that session were caught only by looking. **Also newly owed:** the `/MK`-border disclosure the R43 amendment asked F5's pane to carry; the pane now exists and does not carry it. **★ AMENDED AGAIN 2026-08-07 (tenth filing): F5's cut is HALF DISCHARGED** — `fc51786`+`69db1c6` built the **GUI deletion surface** (see *Implemented*), so what stays owed under Pass 20.5 narrows to the **per-type detail fields**, the `/MK`-border disclosure, and the **rendered-appearance verification** (now owed for both halves). **Newly owed:** per-widget rows for a **multi-page repeated field** are labelled from position, not page identity, when `/P` is absent — and no fixture exercises it. |
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
