# `pdfce-core` consumer API, part 2 — editing and saving

**Audience:** an engineer or agent building a NEW GUI shell (`D:\dev\pdfceGUI`)
against `pdfce-core`, in a session that cannot ask questions here.
**This is not rustdoc.** Rustdoc answers *"what does this function do."* This
answers *"I want to do X — what do I call, in what order, and what will bite me."*

| | |
|---|---|
| **Date** | 2026-08-13 |
| **Verified against** | `5c37c7c` (`git rev-parse --short HEAD`) — *"he gave no reason" was a claim, and it has been corrected* |
| **Primary subject** | `crates/pdfce-core/src/edit.rs` (32505 lines) |
| **Covers** | `EditSession` end to end: construction, the command/undo/redo model, **all 149 public methods**, the `EditError` taxonomy, the save path (incremental vs full rewrite), the guard/refusal model (encryption, certification, sidecar version, `/Size` suppression), object allocation and byte staging |
| **Does NOT cover** | Document loading and the read-only object model → **`01-reading-and-model.md`**. Per-feature capability guides (ce dimensions, forms, annotations, redaction, OCR, printing) → **`03-capabilities.md`**. This document covers the *session mechanics* those features flow through; part 3 covers the features. |
| **Terminology** | Project rule 15. **ce dimensions** = the dimension objects pdfce authors (`/Line` + `/IT /LineDimension` + baked `/AP` + `/PieceInfo` sidecar). **pdf dimensions** = dimensions already present in the page content, exported by CAD. Never bare "dimension". This document only concerns ce dimensions. |

Every `file:line` below was read at `5c37c7c`. Where a fact could not be
established it is marked `UNVERIFIED — <what to check>` rather than guessed.

---

## 0. The mental model, in one screen

```
Document (BASE revision)             ← immutable for the session's life
   buf: retained source bytes        ← every ByteSpan indexes into this
   objects: ObjId -> parsed value
   trailer
        │  EditSession::new(doc)  — takes the Document BY VALUE
        ▼
EditSession                                         edit.rs:3325
   base:        Document                            ← never mutated
   state:       BTreeMap<ObjId, Object>             ← overlay: touched objects only
   deleted:     BTreeSet<ObjId>                     ← existence changes
   trailer:     Dict                                ← working copy
   next_number: Option<u32>                         ← object-number allocator
   staging:     Vec<u8>                             ← authored stream bytes (R45)
   undo:        Vec<Command>                        ← each carries its own inverse
   redo:        Vec<Command>
        │  dirty_set()  =  structural diff(state, base) COMPUTED AT SAVE TIME
        ▼
DirtySet ──► writer::save_incremental / writer::save_full ──► (Vec<u8>, SaveReport)
```

Five consequences a GUI author must internalise before writing any code:

1. **There is no `EditSession::save()`.** Saving is `to_incremental_bytes` /
   `to_full_bytes`, which return `(Vec<u8>, SaveReport)`. Writing the file is
   the shell's job. (`edit.rs:4053`, `edit.rs:4071`.)
2. **`session.document()` is the BASE, not the edited state.** Rendering it
   shows the file as loaded, with none of the operator's edits. Use
   `session.view()`. This exact defect shipped for 13 Passes — see trap T-01.
3. **The dirty set is a diff, not a log.** Edit-then-undo saves byte-identical.
   A "modified?" indicator derived from undo depth will disagree with the file.
4. **Every mutating verb is one undo entry** — with named exceptions
   (`import_form_data` is N entries; see §3.4).
5. **`EditSession` is the only mutation path in the whole project.** Module doc,
   `edit.rs:3-7`: *"`pdfce-gui` and `pdfce-cli` both go through `EditSession`,
   and nothing anywhere constructs a `DirtySet` with real changes except
   `EditSession::dirty_set`."*

---

## 1. Verb index — all 149 public `EditSession` methods

**Count: 149.** Established by brace-matched extraction of the four
`impl EditSession` blocks, matching `pub fn` / `pub const fn`, and checked
on every run by `tools/check-core-api-verbs.py` — which is what caught this
figure at 120 when `add_outline_item` landed.
There are no `EditSession` methods in any other file
(`grep -rn "impl EditSession" crates/pdfce-core/src/` returns those four lines only).

> ### ★ THIS COUNT SAID 108 AND HAD DRIFTED BY EXACTLY EIGHT
>
> Corrected 2026-08-18. The eight verbs this index had never mentioned:
> `set_media_box`, `set_media_boxes`, `set_markup_style`,
> `mark_redactions_by_search_styled`, `mark_redactions_by_pattern_styled`,
> `flatten_refusal`, `insert_pages`, `widget_rects`.
>
> **How it was found, and why that matters more than the correction.** The
> `pdfceGUI` session wired `insert_pages` and shipped a **wrong operator
> disclosure** derived from it. They did not misread this document — *this
> document never mentioned the verb*, so a chat reply was the only
> description of it in existence, and a chat reply is not reviewable, not
> versioned, and not something a second reader can check.
>
> **A consumer-facing API document that omits a verb is worse than one that
> describes it badly**, because a bad description gets argued with and a
> missing one gets replaced by whatever the consumer was told once.
>
> This is now checkable rather than hoped for: `tools/check-core-api-verbs.py`
> re-derives the list from `edit.rs` and fails if this file omits any public
> method or states a count that does not match. The drift was possible for as
> long as it was because nothing compared the two artefacts — the count was
> *stated* as derived, which reads exactly like being *kept* derived.

Read the columns as: **I want to…** → **call this** → **returns / what that means**.

### 1.1 Construct and dispose (2)

| I want to… | Call | Line | Returns — and what it means |
|---|---|---|---|
| Open an editing session | `new(doc: Document) -> Self` | 3368 | The session **is** the open document now. Takes `Document` **by value**; a second handle would be a stale view. |
| Give the document back, throwing away unsaved edits | `into_document(self) -> Document` | 3399 | The **base** document. Edits are discarded — this is not a commit. |

### 1.2 Read the current state (7)

| I want to… | Call | Line | Returns — and what it means |
|---|---|---|---|
| Read the file *as loaded* | `document(&self) -> &Document` | 3393 | ⚠️ The **base revision**. Not the edited state. |
| Read one object's *current* value | `value(&self, id: ObjId) -> Option<&Object>` | 3409 | Overlay if touched, else base. A deleted object reads `None`. |
| Walk the edited object graph | `graph(&self) -> SessionGraph<'_>` | 3429 | Base + overlay, deletions honoured. `impl ObjectGraph`. |
| Render / hit-test / decompose the edited document | `view(&self) -> DocumentView<'_>` | 3469 | Graph **plus stream bytes** via `StreamSource::Split{base,staged}`. **This is what the canvas draws.** Read-only — must never reach the writer. |
| Resolve a staged span from one flat buffer | `authored_source(&self) -> Cow<'_,[u8]>` | 3561 | `base` borrowed when nothing authored; `base ++ staging` **owned** otherwise. ⚠️ ~14 MB memcpy per call once anything is authored — never per frame. |
| List pages as the operator has them | `pages(&self) -> Result<Vec<Page>, PageTreeError>` | 4016 | Document order, all unsaved structural + rotation edits applied. |
| List pages as *structural slots* | `page_slots(&self) -> Result<Vec<PageSlot>, PageTreeError>` | 4032 | Parent node, index within it, ancestor chain, inherited raw attributes. Survives a damaged file that `pages()` cannot resolve. |

### 1.3 Dirty state (2)

| I want to… | Call | Line | Returns |
|---|---|---|---|
| Compute what a save would write | `dirty_set(&self) -> DirtySet` | 3497 | Structural diff vs base, **right now**. Never consults history. |
| Show an "unsaved changes" indicator | `is_modified(&self) -> bool` | 3580 | `!self.dirty_set().is_empty()`. Cannot disagree with the writer. |

### 1.4 Undo / redo (7)

| I want to… | Call | Line | Returns |
|---|---|---|---|
| Enable/disable the Undo control | `can_undo(&self) -> bool` | 3588 | |
| Enable/disable the Redo control | `can_redo(&self) -> bool` | 3594 | |
| Label the Undo control | `undo_kind(&self) -> Option<CommandKind>` | 3601 | Peeks; does not undo. |
| Label the Redo control | `redo_kind(&self) -> Option<CommandKind>` | 3607 | |
| Undo | `undo(&mut self) -> Option<CommandKind>` | 3617 | What was undone. `None` ⇒ stack empty. |
| Redo | `redo(&mut self) -> Option<CommandKind>` | 3634 | What was redone. |
| Show history depth | `undo_depth(&self) -> usize` | 3653 | Bounded by `MAX_UNDO_DEPTH` = **256** (`edit.rs:166`). |

### 1.5 Save (2)

| I want to… | Call | Line | Returns |
|---|---|---|---|
| Save (the default) | `to_incremental_bytes(&self, &SaveOptions) -> Result<(Vec<u8>, SaveReport), WriteError>` | 4053 | Bytes + report. §7.5.6 append. **Superseded objects stay in the file.** |
| Save as one revision | `to_full_bytes(&self, &SaveOptions) -> Result<(Vec<u8>, SaveReport), WriteError>` | 4071 | Bytes + report. ⚠️ Destroys every existing signature. |

Both take `&self` — saving does not mutate the session and does not clear the
undo stack or the dirty set. Neither writes to disk.

### 1.6 Signature / structure queries (3)

| I want to… | Call | Line | Returns |
|---|---|---|---|
| Ask what saving would do to signatures | `signature_impact_of_save(&self, mode: SaveMode) -> SignatureImpact` | 6992 | `None` / `ByteRangePreserved` / `Invalidated`. Ask **immediately before Save**, not at edit time. |
| Census the document's signatures | `signature_census(&self) -> SignatureCensus` | 6998 | Counts + `/P` + `perms_enforced`. |
| Ask whether pages were added/removed/moved | `changes_structure(&self) -> bool` | 7010 | Computed from the page tree, not from history. |

### 1.7 Document metadata (3)

| I want to… | Call | Line | Returns |
|---|---|---|---|
| Set or clear `/Title`, `/Author`, … | `set_info_field(&mut self, field: InfoField, value: Option<&str>) -> Result<(), EditError>` | 3689 | `Some` sets, `None` **removes the key**. Creates `/Info` if absent. **A no-op records no command.** |
| Read a field's raw bytes | `info_bytes(&self, field: InfoField) -> Option<Vec<u8>>` | 3788 | Reflects unsaved edits. |
| Read a field as text | `info_text(&self, field: InfoField) -> Option<InfoText>` | 3807 | `InfoText{ text, exact }`. `exact == false` ⇒ **do not write it back** unless the operator changed it. |

`InfoField` (`edit.rs:197`, `#[non_exhaustive]`) deliberately **excludes**
`/Producer` (R41 fingerprint rule) and `/CreationDate`/`/ModDate` (§7.9.4 dates
need their own policy).

### 1.8 Page rotation, geometry and organisation (9)

| I want to… | Call | Line | Returns |
|---|---|---|---|
| Set one page's absolute rotation | `set_page_rotation(&mut self, page_index, degrees: i32) -> Result<(), EditError>` | 3848 | Refuses non-multiples of 90 (`RotationNotMultipleOf90`). |
| Turn one page relative to its current rotation | `rotate_page_by(&mut self, page_index, delta: i32) -> Result<(), EditError>` | 3981 | Delegates to `set_page_rotation`. |
| Delete pages | `delete_pages(&mut self, indices: &[usize]) -> Result<DeleteOutcome, EditError>` | 14644 | See `DeleteOutcome`, §1.25. |
| Delete pages, answering the pre-separation question | `delete_pages_with(&mut self, indices, separations: SeparationPolicy) -> Result<DeleteOutcome, EditError>` | 14663 | |
| Reorder pages | `reorder_pages(&mut self, new_order: &[usize]) -> Result<(), EditError>` | 14944 | `NotAPermutation` if `new_order` is not one. ONE undo entry. |
| Rotate several pages | `rotate_pages(&mut self, indices: &[usize], delta: i32) -> Result<usize, EditError>` | 15063 | Count of pages turned. ONE undo entry. |
| Set one page's `/MediaBox` | `set_media_box(&mut self, page_index: usize, rect: page_tree::Rect) -> Result<MediaBoxChange, EditError>` | 5013 | |
| Set several pages' `/MediaBox` | `set_media_boxes(&mut self, indices: &[usize], rect: page_tree::Rect) -> Result<Vec<MediaBoxChange>, EditError>` | 5061 | |
| **Insert pages from another document** | `insert_pages(&mut self, source: &DocumentView<'_>, source_pages: &[usize], position: pageops::InsertPosition) -> Result<InsertOutcome, EditError>` | 16978 | `InsertOutcome { pages_inserted, orphaned_widgets }`. **Read the warning below before writing a disclosure about it.** |

> #### ★★ `insert_pages`: THE WIDGETS ARRIVE, THEIR FIELDS DO NOT — and one
> consumer has already shipped the wrong sentence about it
>
> `insert_pages` copies everything **reachable from the page**, and a page's
> `/Annots` reaches its widget annotations. A **field definition** is not
> reachable from the page — it lives in the document-level `/AcroForm`
> `/Fields`. So the two halves of a form field separate:
>
> ```text
> SOURCE  fields=Some(12)
> TARGET  fields=None   annots=13   widgets=13
> ```
>
> The result is **not** *"the form fields did not come across"*. It is
> **boxes that draw exactly like form fields, that an operator will click on,
> and that nothing can fill, because no field claims them.** That is worse
> than absence, and worse in a way this project already has a name for: a
> visible control that is silently inert — arriving through a document
> instead of through a ribbon.
>
> **Do not paraphrase this as "form fields did not come across."** An
> operator given that sentence goes looking for missing fields instead of at
> the inert ones in front of them. *A disclosure that names the wrong failure
> is worse than none, because it is believed.*
>
> Also not merged, and these ARE plain absences: outlines, named
> destinations, page labels, optional-content configuration.
> [`pageops::insert`] merges all of it and returns a new document; the
> difference is the cost of staying incremental.
>
> **`Pass 102.0` SHIPPED 2026-08-19** — `InsertOutcome::orphaned_widgets` is
> that count, so a shell can say *"three controls arrived that cannot be
> filled"* instead of warning unconditionally. It is **exact, not
> conservative**: `/AcroForm` is not merged and the copy remaps every object
> number, so no field in the target can be claiming a widget that just
> arrived.
>
> `Pass 102.1` will carry field definitions for fields whose widgets are
> *wholly* on inserted pages. **102.1 does not retire 102.0** — a field whose
> widgets are split across inserted and non-inserted pages leaves a residue
> no merge can absorb, so the count is permanent.

### 1.9 Page-text editing (5) — detail in part 3

> ### ★★ FORM-XOBJECT TEXT IS EDITABLE AS OF `Pass 119.0` — and the old warning here is REVERSED
>
> **This block used to say the opposite.** Until 2026-08-20 it read *"editing
> reaches PAGE-STREAM text only"* and told you to refuse a caret on
> `Editability::InsideForm`. That is now wrong in the most expensive direction
> — a shell that still refuses is refusing on **99 % of the text on a CAD
> drawing**, which is the operator's own estimate and the reason this Pass was
> escalated ahead of everything else.
>
> **What changed.** `edit_text` (and `format_text`'s sibling path is *not* yet
> included — see the limits below) now resolves the target stream instead of
> assuming the page's. `EditRequest::target` selects it:
>
> | `EditTarget` | what it edits |
> |---|---|
> | `Auto` (**default**, and what every existing caller already gets) | the page's own `/Contents` first, then each form XObject the page paints, **in `Do` order** — i.e. paint order, so when two streams both contain the text the one drawn first wins, which is predictable without knowing anything about PDF structure |
> | `PageContents` | the page's own content ONLY — the pre-119.0 reach, for a batch caller that wants a hard failure rather than a widened search |
> | `Form { object }` | that form XObject's stream, by object number. A form that is not painted by the page is an **error**, not a widened search |
>
> **`TextRun::editability()` now answers `Editable` for form content**, exactly
> as `Pass 118.0`'s documentation promised it would when the capability landed.
> `Editability::InsideForm` is **`#[deprecated]` and never returned** — delete
> the arm that matches it. The deprecation is deliberate: a silent change would
> have left your guard refusing carets forever, and deleting the variant would
> have broken your build with a message that said nothing about what to do.
>
> ### ★ THE ONE THING TO PUT IN FRONT OF THE OPERATOR: shared content
>
> A form XObject may legally be painted **from several pages and several times
> on one page** — ISO 32000-1 §8.10.1 states that as the *purpose* of the
> feature — and **no clause in either edition binds a form to a page.** That is
> a confirmed permanent negative result in pdfce's spec corpus (`FX-N1`), argued
> three independent ways.
>
> **So editing text inside a shared form changes every place it appears, and
> there is nothing pdfce can do about it: there is exactly one stream holding
> those glyphs.** The default is therefore edit-in-place, disclosed — chosen
> over copy-on-write because copy-on-write is *not always expressible* (a form
> invoked from inside another form cannot be re-bound without editing the
> parent, which may itself be shared), and a default whose semantics silently
> depend on document structure is worse than one that always means the same
> thing. **Adobe's behaviour here is unsourceable** — an `Acrobat_Features`
> sweep found no documentation of what Acrobat does to a shared form, from
> Adobe or from its competitors — so this is pdfce's own reasoned choice, not
> a parity claim.
>
> `EditReport` carries the fan-out:
>
> | field | meaning |
> |---|---|
> | `form_object: Option<u32>` | the form stream that was rewritten; `None` = the edit was in the page's own `/Contents` |
> | `form_invocations: u64` | **how many places in the DOCUMENT paint it**, transitively through nesting. `1` is the ordinary case; `0` means the edit was not in a form |
> | `form_pages: Vec<usize>` | the zero-based page indices it appears on |
>
> **Do not drop `form_invocations`.** A shell that ignores it is a shell that
> changes six drawing sheets while showing one. The matching disclosure string
> (`"SHARED CONTENT: ..."`) is in `report.disclosures` and is worded for direct
> display.
>
> To enumerate targets *before* an edit: `pdfce_core::text_edit::forms` —
> `scan_page_forms(doc, view, page)` returns every form the page paints with its
> object number, nesting depth, effective resources and dictionary;
> `invocation_map(doc, view)` answers the fan-out for all of them in **one**
> document walk. `pdfce-cli inspect --forms` prints the same thing.
>
> ### Limits, so you do not find them by pressing
>
> - **A `/Ref` reference XObject or an OPI proxy is refused by name.** Its
>   visible content is a placeholder a conforming reader may substitute
>   wholesale, so an edit there can appear to work and never reach what is
>   printed.
> - **A form whose `/Resources` is present but does not declare the font its own
>   text selects is refused**, not guessed at. Filling the gap from the page
>   would resolve a font `pdfce-render`'s interpreter does not, and the advance
>   arithmetic would then be computed from `/Widths` nothing else consults.
> - **A form that omits `/Resources` entirely IS editable** — §7.8.3's fourth
>   bullet makes inheritance from the page a `shall` on the reader, and PDF 2.0
>   removed the sentence calling it obsolete. The inheritance is disclosed.
> - **Nesting is guarded at 64 levels.** Deeper is *conforming* content pdfce
>   refuses; the refusal says so, because it is pdfce's limit and not the file's
>   defect.
> - **`format_text` was retargeted too (`Pass 119.2`)** — same `target`
>   selector, same default, same `form_object` / `form_invocations` /
>   `form_pages` on its report, same `"SHARED CONTENT: ..."` disclosure. It is
>   `FormatRequest::target`, and the type is deliberately the identical
>   `EditTarget`: a shell that has decided which stream a caret is in should not
>   have to translate that decision between two verbs acting on the same run.
>   A family change (`set_font`) resolves its target face against the **form's**
>   resource dictionary, which is the correct one — `/F1` inside a form is a
>   different font from `/F1` on the page — and is read-only about it, so a face
>   that does not resolve is still `TargetFontMissing`, never an insertion.
> - **`reflow_block` and `add_text` still reach page-stream text only.**
>   `editability()` reports `edit_text`'s reach, so for those two it is now
>   optimistic — check `GlyphProvenance::content_stream` directly if you gate on
>   them. `add_text` in particular is not merely unfinished work: appending to a
>   form's content stream changes what **every** invocation site paints, which
>   is a different disclosure from an in-place edit's and needs its own thinking.
>
> A pinned request that names no operator in the buffer reports
> `EditError::PinnedSpanNotFound { start, end }` rather than `NoMatch(find)` —
> the old message blamed the operator's own text for a pin pointing at the
> wrong buffer, and **it misled twice** before it was split (`Pass 118.0`).
>
> The measurement that drove all of this, on a CAD-exported sheet:
>
> | | where |
> |---|---|
> | 3,007 single-character `Tj` spelling the producer's watermark | the **page** stream — editable, and nobody wants to edit it |
> | 1,696 show operators: every label, the title block, every *pdf dimension* callout | a **form XObject** — editable as of `Pass 119.0` |

| I want to… | Call | Line | Returns |
|---|---|---|---|
| Replace text in place | `edit_text(&mut self, &EditRequest, &EditOptions) -> Result<EditReport, text_edit::EditError>` | 4126 | Report, **not** saved bytes. One undo entry. |
| Change size / colour / family in place | `format_text(&mut self, &FormatRequest, &FormatOptions) -> Result<FormatReport, text_edit::FormatError>` | 4171 | One undo entry. |
| Ask what synthetic bold/italic *would* do | `preview_style_resolution(&self, page_index, find, pinned_span, want) -> Result<StyleResolution, FormatError>` | 7388 | Pure query. **Decides where a style button routes** — see below; an empty `find` is not a wildcard here. |
| Ask which fonts `set_font` would ACCEPT for a run | `preview_font_resources(&self, page_index, find, pinned_span) -> Result<FontPreflight, FormatError>` | 7459 | Pure query. **Per RUN, not per page** — see below. |
| Re-wrap a recognised paragraph | `reflow_block(&mut self, page_index, block_index, &ReflowRequest) -> Result<ReflowApplyReport, ReflowApplyError>` | 4297 | One undo entry. **Planned against the BASE** — see trap T-14. |
| Add a new text run at coordinates | `add_text(&mut self, &AddTextRequest) -> Result<AddTextReport, AddTextError>` | 4365 | Appends a new content stream; originals stay byte-verbatim. |
| Add an invisible OCR text layer to one or more pages | `add_ocr_layer(&mut self, &[OcrPageLayer<'_>], &OcrLayerOptions) -> Result<Vec<OcrLayerReport>, OcrLayerError>` | 7313 | **ONE undo entry for the whole run**, however many pages. Reads the SESSION graph, not the base. |
| Give ONE page a private copy of a shared form XObject | `unshare_form(&mut self, page_index, form: ObjId) -> Result<UnshareFormReport, EditError>` | 7367 | Copy-on-write. Refuses a **nested** invocation by name. |

#### ★ The FOUR entry points that take a `find` and a pin all resolve it the same way (`Pass 148.0`)

There are exactly four, and after three Passes they finally agree. Listed
together because the whole class of defect below was *"one of them didn't"*:

| entry point | kind |
|---|---|
| `format_text` / `set_format` (`FormatRequest`) | commit |
| `edit_text` (`EditRequest`) | commit |
| `preview_font_resources` | query |
| `preview_style_resolution` | query |

**The rule, on all four:**

- **empty `find` + `pinned_span` ⇒ the whole pinned operator.** Resolved by
  one shared function, so a preview and the commit it previews cannot describe
  different characters.
- **empty `find`, no pin ⇒ refused**, `Unsupported("empty find text")`. It is
  *not* a wildcard and *not* a no-op: every string contains the empty string,
  so an unpinned empty `find` would otherwise silently name the page's **first
  show operator** — an answer about something the caller never asked about.

★★ **`preview_style_resolution`'s unpinned refusal is a behaviour change**
(`Pass 148.0`). It previously answered about that first operator. If you have a
call site passing an empty `find` with no pin, it now returns an error — which
is the point: that call was returning a **routing decision** (does Bold go to
`set_font` or `set_synthetic`?) about an arbitrary operator. On
`fixtures/synthetic/textedit/format_family.pdf` it named `Times-Bold`, the one
face on the page that cannot show the run, so a Bold button following it got a
refusal and there was **no bold by either route**.

★ **Why this is worth a section rather than a footnote.** One defect took three
Passes to close because each fix enumerated routes from the *function being
fixed*: `145.0` fixed the two commit paths, `147.0` fixed one query (reported
by a consuming project), `148.0` fixed the other (found by an unrelated sweep).
`crates/pdfce-core/tests/route_enumeration.rs` now asserts, as a **source
scan**, that every function calling `find_anchor` also calls the resolver — the
only assertion in this area that can fail on a **fifth** entry point added
later. If pdfce grows one, that test goes red before you see it.

#### `FormatRequest`'s `find` — and how to say "the whole operator" instead (`Pass 145.0`)

`FormatRequest::new(page, find)` takes `find` as *"the text to locate within
one show operator's decoded run"*. **It stayed required even when the
operator was already located by `pinned_span`** — so a caller holding an
operator had to hand back a string for pdfce to search for inside that very
operator. That round trip is where three consecutive attempts failed:

| attempt | outcome |
|---|---|
| `find: ""` with a pin | refused — *"empty find text"* |
| `find` = the run's `text` | `NoMatch` |
| `find` = the glyph-covered bytes | `NoMatch` on some runs |

The middle row is the one that will catch you: a run's `text` is **not** in
1:1 correspondence with its glyphs (`/ToUnicode` may map one glyph to several
characters — `01-reading-and-model.md` §8.4.0), so a rebuilt `find` need not
match the operator's own decoded text. It fails **invisibly on unligatured
test text** and **routinely on real typeset copy**.

**Use this instead:**

```rust
let span = model.provenance(gref)?.operator_span;
let req  = FormatRequest::whole_operator(page_index, span).size(24.0);
```

`FormatRequest::whole_operator(page_index, span)` ≡
`FormatRequest::new(page_index, "").pinned(span)`. Both spellings work and
produce identical bytes (pinned by a test); the named one says what it means.
`EditRequest` gets the same affordance by setting `pinned_span` with an empty
`find`.

**Three things about it:**

1. **An empty `find` with NO pin is still refused**, by the same name it
   always was. A caller who forgot to pin gets a refusal, not silent
   whole-operator behaviour on an operator pdfce chose for them.
2. **It targets the pinned OPERATOR, not the run.** 13 % of runs over pdfce's
   corpus carry glyphs from more than one show operator. The report carries a
   `whole operator:` disclosure stating the extent taken and naming that case
   — the extent was pdfce's choice, so rule 4 applies.
3. **The font-coverage gate sees the resolved text.** A whole-operator request
   combined with `set_font` checks the target against the operator's actual
   characters, not against the empty string — which every face would have
   "covered".

CLI: `format-text --pin-span START:LEN` with an empty (or omitted) `--find`.
Get the numbers from `extract-text --json --spans`.

#### `preview_font_resources` — read this before wiring a font or style control

`Pass 142.1`. A `&self` query that answers, for **one located run**, which of
the page's `/Font` resources `format_text`'s `set_font` would actually accept
— by calling the accepting code, not by describing it.

Three things it tells you that nothing else can, and each one has bitten
somebody:

1. **`selector` is the string to pass to `FontSelector::new`, and it is not
   always the `/BaseFont`.** A page routinely carries two `/Font` resources
   with the *same* `/BaseFont` — two independent subsets of one face, present
   in 87 % of embedding files by `pdfceGUI`'s own survey — and `set_font`'s
   name match reaches exactly one of them. When that happens `selector` falls
   back to the **resource key** and `base_font_ambiguous` is `true`. Display
   the `/BaseFont`; *select* with `selector`.

2. **Acceptance is per RUN.** The same face, on the same page, accepts
   `"hell"` and refuses `"hello world"` when its `/Encoding /Differences`
   reassigns the code for `o`. Any answer cached against a page rather than a
   selection is wrong for half the selections on it.

   ★ **An empty `find` with a `pinned_span` means the whole pinned operator**,
   exactly as it does for `FormatRequest` (`Pass 145.0`) and by the same call —
   the two cannot disagree about what an empty `find` means. `FontPreflight::text`
   reports the **resolved** characters, so you can read back what was tested.
   An empty `find` with **no** pin is **refused** by name. Before `Pass 147.0`
   neither held: the query tested coverage against zero characters and reported
   **every face on the page as accepted**, which looks like a richer list rather
   than a broken one.

3. **`real_bold()` / `real_italic()` decide where a style button routes.**
   `Some(sibling)` → call `format_text` with `set_font(sibling.selector)`.
   `None` → call it with `set_synthetic`. **`None` is not a reason to disable
   the control** — synthesis is a real route, and `03-capabilities.md` §3.6's
   *"do not grey out a bold button"* still stands. An entry that **is** the
   family's real bold reports **itself**, so a run already in a real bold face
   is never told to synthesize on top of one.

`FontAcceptance` is `#[non_exhaustive]`: use `is_accepted()` for the yes/no
and an `if let FontAcceptance::Refused { message, character }` when you want
the reason. `message` is the sentence `set_font` itself would produce,
verbatim — if it ever disagrees with the real attempt, that is a defect in
pdfce, not something for a shell to paper over.

**Errors are location failures only.** A font that would refuse is an
*answer*, never an `Err`.

These five return `text_edit`'s own error types, **not** `EditError`;
`add_ocr_layer` returns `ocr::layer::OcrLayerError`.

#### ★★ `add_ocr_layer` — the three things a caller must know

1. **It is an EDIT, and that is the entire point.** `ocr::layer::add_ocr_layer`
   (the free function) takes an immutable `&Document` and returns **a whole new
   PDF**, which made recognition the one capability that was not an edit: a
   shell holding an open session could only offer *"here is a different file,
   somewhere else"*, because its in-place save path cannot be used on a document
   it does not have. The session verb lands in the session and saves through the
   caller's ordinary path.

2. **★★★ It reads the SESSION graph, not the base — and this is not a
   refinement, it is the reason the verb exists.** The free function reads the
   **base** revision, so running it after any edit yields a recognised copy that
   **silently omits that edit**. The correct defence against that is a refusal,
   and a consuming shell duly refused to run OCR once the session was dirty —
   but a session never becomes clean again, *not even after a successful save*,
   so **OCR died for the rest of the session the first time anything was
   edited.** Planning against the session graph removes the divergence instead
   of policing it. Pinned by
   `crates/pdfce-core/tests/ocr_session.rs::session_ocr_sees_an_edit_made_earlier_in_the_session`,
   which was **verified to fail** against a base-reading build.

3. **★ A duplicated page index is REFUSED (`OcrLayerError::DuplicatePage`), and
   that is a correctness refusal.** Every page is planned against the graph as
   it stands *before* the commit — that is what makes a multi-page run one undo
   entry. Two entries for one page would both append to that page's *original*
   `/Contents`, and the second page-dictionary write would clobber the first:
   one layer written, one lost, and a report claiming both. Merge the word lists
   before calling if you want two sets of words on one page.

An empty slice is a **no-op that commits nothing** — no undo entry that would
undo nothing. Every refusal happens before any object is allocated, so a
rejected run leaves the session, its bytes and its undo stack exactly as they
were.

#### ★★ `unshare_form` — the "option" half of the shared-form edit default

A form XObject may legally be invoked from **more than one page and more than
once from one page** (§8.10.1 names CAD output as its own illustration). So
editing content inside one **necessarily changes every sheet that invokes it**
— there is exactly one stream object to write, and pdfce cannot prevent that
structurally.

That is the **default**: edit in place, disclosed, with
`EditReport::form_invocations` reporting every site the edit will reach
**before** the write. `unshare_form` is the separate, explicit act of
**breaking the sharing first**, so the edit that follows lands on one page only.

**★ Two things are privatised on the way, and skipping either produces a
"private" copy that is still shared:**

1. **`/Resources`, if inherited** (§7.7.3.4). A page with no `/Resources` of
   its own uses an ancestor's; re-pointing a name there re-points it for
   **every page under that ancestor**.
2. **The `/XObject` subdictionary, if shared** — commonly one indirect
   reference held by several pages. It is written back as a **direct**
   dictionary on this page's own `/Resources`.

**Refusals**, all before anything is allocated:

| error | when | what to do instead |
|---|---|---|
| `FormNestedInAnotherForm` | the form is reached only from **inside another form** | unshare the outer form, or edit in place and accept the blast radius |
| `FormNotOnPage` | nothing on that page names it | check the page or the object number |

★ The nested refusal is principled, not lazy: re-binding there means editing the
**parent** form, which may itself be shared, so the operation's reach would
depend on the document's nesting structure — the same reason decision 076 gives
for rejecting copy-on-write as the *default*.

**Granularity is the PAGE, not the invocation.** If a page invokes the form
under several names, all of them move to the one copy (`references_moved` says
how many). Splitting two invocations on one page would need a per-invocation
identity the object model does not carry.

★★ **Historical note worth keeping**, because it is why this verb's absence
went unnoticed: decision 076 argued its own `R206` compliance ("ship both
options, pick a default") on the premise that **both had shipped**. This one had
not — it was filed the same day and built a week later. A decision can certify
its compliance with a standing rule using a fact that is not true, and nothing
downstream checks it.

### 1.10 Vector geometry (19) — detail in part 3

Eleven of the thirteen return `Result<Vec<String>, EditError>`. **The
`Vec<String>` is the disclosure list**
(`crate::vector::PlannedEdit::disclosures`) — operator-facing strings the
surgery owes under project rule 4, usually empty. **They must be surfaced**; an
empty vector is the normal case, a non-empty one means the gesture changed an
operator's *form* (an `re` rectangle expanded to explicit segments, an implicit
`m` materialised, a curve discarded with a node).

The two exceptions are `transform_objects` / `transform_preview`
(`Pass 113.0`/`113.1`), which return a `TransformOutcome` — see below.

> ### ★★ `transform_objects` — the verb that is NOT `move_objects` with a matrix
>
> ```rust
> session.transform_objects(page, &[3, 4, 7], matrix, TransformOptions::default())
>     -> Result<TransformOutcome, EditError>
> session.transform_preview(page, &[3, 4, 7], matrix, TransformOptions::default())
>     -> Result<TransformOutcome, EditError>   // &self, commits nothing
> ```
>
> **You asked for this on the reasoning that `move_objects` just needed a
> matrix. It could not.** Operand rewriting expresses translation and nothing
> else:
>
> - a **rotated rectangle has no `re` spelling** — `re` carries an origin and a
>   size, so a rotate would have to expand every rectangle to four lines,
>   changing the file's shape to express a gesture that changed nothing about
>   what is drawn;
> - **`line_width` is a user-space scalar**, so a scaled path would keep its
>   original stroke weight;
> - **text and images have no coordinate operands at all**, which is precisely
>   why `move_objects` refuses them with `NotAPath`.
>
> So each object's operator run is wrapped in `q <cm> … Q`. That never looks at
> an operand, and is therefore **kind-agnostic by construction** — your own
> argument that *"a placed image and a placed text run are the same shape"*,
> granted by the mechanism rather than by a match arm per kind. Path, text,
> image XObject, form XObject and inline image are all just a byte span with a
> CTM.
>
> #### ★ `matrix` is in PAGE space, and it is NOT what gets emitted
>
> `cm` composes into the CTM in force at that point in the stream (§8.3.4:
> `CTM′ = M × CTM`), which is the object's **user** space. Emitting your matrix
> directly would be correct only where an object's CTM is the identity and
> **silently wrong at every scale or slant the producer left in force** — an
> object moved twice as far as the pointer went, with nothing erroring.
>
> pdfce emits `X = CTM × M × CTM⁻¹`, per object, from **that object's own**
> captured CTM. A selection spanning two local spaces gets two different `cm`
> operands for one gesture and both land in the same place on the page. **You
> pass page-space; that is the whole contract.**
>
> Use `Matrix::about(pivot)` (`Pass 112.0`) — the pivot is yours to choose, by
> your own request, and pdfce does not invent one.
>
> #### Two refusals, distinguished because you said they drive different UI
>
> | error | means | UI |
> |---|---|---|
> | `DegenerateCtm` | **this object cannot be transformed at all** (its own CTM is singular) | do not offer a handle |
> | `SingularTransform` | **this drag is degenerate** (the requested matrix maps area to zero) | offer the handle, refuse on release |
>
> ★ **A negative scale is NOT singular.** `scale(-1.0, 1.0)` is a mirror and is
> perfectly invertible, so dragging a grip through the *opposite* edge is an
> ordinary transform. Only exactly zero is degenerate — which a
> commit-on-release gesture makes nearly unreachable, so the default costs a
> well-behaved shell nothing.
>
> #### Both options ship, per the operator's own ruling (`R206`)
>
> He answered both of my design questions before you read them: *"make things
> work both ways as options. default it to your best guess as to what would be
> normally expected."*
>
> | question | **default** | option |
> |---|---|---|
> | mixed selection | **transform whole**, one command, one undo | `MixedSelection::RefuseHeterogeneous` |
> | singular matrix | **refuse by name** | `SingularPolicy::Clamp { min }` — clamps and discloses |
>
> `R168` is unaffected: if an object in the selection cannot be transformed at
> all, the whole call refuses with a stated reason rather than transforming the
> part that qualified.
>
> #### `TransformOutcome`
>
> | field | meaning |
> |---|---|
> | `objects_transformed: u64` | **not necessarily your index count** — duplicates, and an object whose span is contained inside another selected object's, are collapsed, because wrapping a contained span twice applies the transform to those marks twice |
> | `clamped: bool` | a singular transform was clamped rather than applied; the selection is not the size the gesture asked for |
> | `disclosures: Vec<String>` | as everywhere |
>
> #### The preflight you asked for three times
>
> `transform_preview` is `&self`, side-effect-free, and **shares one body with
> the verb** — so `preview(..).is_ok()` *is* the predicate, and a preview that
> says yes where the call then refuses is not a reachable state. Four cases
> (good transform, singular, stale index, refused mixed selection) are pinned
> equal by a test, because "they agree on the happy path" is what a second
> implementation would also manage.
>
> CLI equivalent: `pdfce-cli object-transform --objects 3,4,7 --scale 1.5
> --rotate 15 --translate 10,0 [--pivot X,Y] [--preview]`.

> ### ★★ The object clipboard — and the half of it that is NOT `import_object`
>
> ```rust
> session.copy_objects(page, &[3, 4, 7])      -> Result<ObjectClip, EditError>   // &self
> session.cut_objects(page, &[3, 4, 7])       -> Result<ObjectClip, EditError>   // ONE undo entry
> session.paste_objects(page, &clip, at)      -> Result<PasteOutcome, EditError>
> session.paste_preview(page, &clip, at)      -> Result<PasteOutcome, EditError> // &self
>
> clip.to_bytes()                             -> Vec<u8>
> ObjectClip::from_bytes(&bytes)              -> Result<ObjectClip, ClipError>
> ```
>
> **Your reading of `import_object` was right, and it is the smaller half.**
> That function copies *indirect objects*. A page's content objects are **byte
> ranges inside a content stream**, and the operators in those bytes name their
> resources **by page-local name** — `/F1 12 Tf`, `/Im1 Do`.
>
> **On the destination page, `/F1` is a different font.** Pasting the bytes
> verbatim draws the right shapes in the wrong typeface, or draws nothing, and
> **neither failure errors.** So copy records which names each item consumes
> and carries the objects behind them; paste re-binds every one to a fresh
> `pdfceP*` name on the destination page and rewrites the names inside the
> copied bytes. That is the part you would have had to build, and it is done.
>
> #### The clip owns its resources
>
> `ObjectClip` carries the transitive closure of everything its items
> reference, **by value**, with stream payloads owned as bytes rather than as
> spans into a document that may already be closed. So:
>
> - **copy → close the source → paste** works;
> - **cross-document paste is the same code path** as same-document paste —
>   there is no source to consult at paste time, so there is no case to
>   special-case;
> - **`to_bytes` was a serialisation problem, not a design one**, which is why
>   it shipped in the same session.
>
> #### Placement
>
> `paste_objects` takes `at`, a **page-space** matrix — the same contract
> `transform_objects` takes.
> `Matrix::IDENTITY` is paste-in-place, `Matrix::translate` is
> paste-with-offset, `Matrix::about` gives paste-scaled and paste-rotated. One
> verb, four gestures.
>
> `PasteOutcome::bbox` is what you draw your paste outline from — it maps **all
> four corners**, so it is right under a rotation and not only under a
> translation. Ask `paste_preview` for it before committing.
>
> #### `PasteOutcome`
>
> | field | meaning |
> |---|---|
> | `objects_pasted: u64` | how many arrived |
> | `resources_added: u64` | how many `/Resources` bindings the page gained — **worth showing somewhere**: every paste adds fresh entries, so a shell that pastes the same clip forty times and wonders why the file grew has the answer here |
> | `bbox` | page-space bounds after `at` |
> | `disclosures` | as everywhere; empty for an ordinary paste |
>
> #### Cut is one undo entry, and the ORDER is load-bearing
>
> You asked for exactly this: *"otherwise Ctrl+X then Ctrl+Z gives the operator
> their objects back but leaves the clipboard changed, or takes two presses."*
> `copy_objects` is `&self` and commits nothing, so only the deletion reaches
> the undo stack.
>
> **Copy runs first**, deliberately: a selection that cannot be copied is
> refused with **nothing deleted**. Reversed, a cut whose copy half failed
> would take the objects away with nothing on the clipboard — the one outcome
> the operator cannot recover from by pasting.
>
> **pdfce holds no clipboard state.** `cut_objects` *returns* the clip. If you
> want Ctrl+Z to restore the previous clip contents, you are holding the only
> stack that could.
>
> #### Serialisation, and the refusals on the way back in
>
> `to_bytes` writes a magic-prefixed, versioned, length-prefixed payload.
> Numbers are **bit-exact** — a matrix that changed in the last place on every
> copy/paste cycle would drift a shape visibly after enough of them. Object
> values go through the crate's own writer and come back through its own
> parser, so the COS grammar has one implementation per side.
>
> `from_bytes` refuses, by name: `ClipError::NotAClip` (checked **before** any
> length prefix is read, so an unrelated payload from the OS clipboard is
> refused with a sentence), `Truncated`, `NewerFormat`. A truncation sweep over
> **every** prefix length is pinned by a test.
>
> #### ★ Annotations are on the clipboard too (`Pass 120.4`) — a SECOND address space
>
> ```rust
> session.copy_annotations(page, &[0, 2])            // /Annots order
> session.copy_selection(page, &[3, 4], &[0, 2])     // both, one gesture
> ```
>
> `copy_annotations` takes the `/Annots` numbering; `copy_selection` takes
> both lists at once and is the verb to call when a marquee caught content and
> a comment, which on a marked-up drawing is the ordinary case.
>
> **Two index lists, not one, because they are genuinely different
> numberings.** An annotation is not content, so it has no paint-order index.
> Merging them into a tagged list would put you in the business of remembering
> which numbering a given index came from; this signature makes that mistake
> impossible.
>
> Annotations are copied **through pdfce's own models**, not as raw
> dictionaries — a markup through `MarkupSpec`, a ce dimension through its
> `DimensionKind` **plus its group's name and unit**. So the destination
> re-bakes the appearance and re-registers the sidecar itself, and a pasted ce
> dimension keeps measuring the thing it measured at home. A `GroupId` means
> nothing in another document; the group is matched **by name**, and created if
> absent.
>
> | kind | what happens |
> |---|---|
> | markup (`Square`, `Circle`, `Line`, `Ink`, `Polygon`, `Cloud`, `PolyLine`, text markup) | pasted |
> | **ce dimension** | pasted, with its group |
> | **widget**, and anything else | **refused by name, with the reason** — a widget carries an `/AcroForm` field registration, and a *renamed* field is a **different field**: any script, calculation order or parent-child relationship naming the old one breaks silently. That is a decision about your form, not a copy |
>
> `PasteOutcome::annotations_pasted` counts only the ones that landed, so it
> disagreeing with `clip.annotation_count()` **is** the signal that something
> was refused. The refusal text is in `disclosures`.
>
> ★ **A rotated `Square` or `Circle` ENCLOSES rather than refusing, and says
> so.** `/Rect` is axis-aligned by definition (§12.5.2), so a rotated rectangle
> has no spelling — the `re` problem from `Pass 113.0` in a different carrier.
> Enclosing is the only shape the format admits, so this is not pdfce choosing
> between two renderings; it is pdfce doing the one available and disclosing
> it. Point-based markup and every ce dimension rotate faithfully.
>
> ★★ **`to_bytes` does NOT carry annotations in this cut, and the clip says
> so**: `clip.annotations_survive_serialisation()`. Ask it before writing a
> clip to disk — the alternative is a payload that quietly loses the operator's
> comments, which is the kind of data loss only noticed later. An in-process or
> in-session annotation clipboard works today.
>
> #### One more limit, named so you do not find it by pressing
>
> - **A resource that does not resolve on the SOURCE page refuses the copy**,
>   not the paste. That is the earliest point the operator can be told, and
>   their selection is still on screen.
>
> CLI equivalent: `pdfce-cli object-copy --objects 3,4,7 --clip sel.pdfceclip
> [--cut out.pdf]` then `pdfce-cli object-paste --clip sel.pdfceclip
> --translate 10,20 -o out.pdf [--preview]`.

#### ★ 1.10.1 Object indices across an edit — which verbs RENUMBER

**All eleven verbs address objects by `decompose_page`'s paint-order index.
An index is a POSITION, not an identity, and a position is only an identity
while nothing moves.** So the question a shell holding a live selection must
be able to answer is: *does my index still name the same object after that
edit?*

There are three possible outcomes and only one of them is dangerous:

| outcome | consequence |
|---|---|
| resolves to the same object | correct |
| resolves to nothing | correct — clear the selection |
| **resolves to a DIFFERENT object** | **the outline redraws around the wrong thing and the next Delete removes it. Nothing errors.** |

**The answer, measured** (`crates/pdfce-core/tests/object_identity_across_edits.rs`
decomposes, edits, and decomposes again — this is not read off the planners):

| family | mechanism | renumbers? |
|---|---|---|
| `move_object` · `move_objects` · `move_subpath` · `move_node` · `move_nodes` · `move_handle` | rewrites operator **operands** in place | **NO** |
| `delete_object` · `delete_objects` · `delete_subpath` · `delete_node` · `delete_text_run` | excises byte **spans** | **YES** |

**A move changes numbers inside existing operators**, so no operator is added
or removed, the decomposition walks the same operators in the same order, and
indices are stable. **Build move, resize and node editing on indices — a
selection survives them unchanged.**

**For the delete family, remap:**

```rust
use pdfce_core::vector::remap_index_after_delete;

remap_index_after_delete(0, &[1]) == Some(0)   // below the hole — unmoved
remap_index_after_delete(1, &[1]) == None      // it WAS the hole
remap_index_after_delete(2, &[1]) == Some(1)   // shifted down
remap_index_after_delete(4, &[1, 3]) == Some(2)
```

`None` means *gone*. It never returns a different object's index. `deleted`
need not be sorted, and duplicates are handled — a shell unioning two
overlapping selections would otherwise shift a survivor twice.

**On durable tokens.** `VectorObject` already exposes `tokens() -> TokenRange`
(operator indices — **stable across a move**, since the operator count is
unchanged) and `bytes() -> ByteSpan` (**not** stable: rewriting `10` to `100`
lengthens the operand and shifts everything after it). Neither survives a
delete. A generation-tagged whole-stream handle is *possible* but is not built,
deliberately — given the table above it would buy nothing indices do not
already give for the move family.

**If `reorder` or `paste` are ever added, they must be checked against that
test file and this table extended.** A new verb that renumbers without saying
so re-opens exactly this hazard, silently.

*Raised by the `pdfceGUI` session, 2026-08-13, which correctly declined to
build move/resize until it was answered. Part 1 covers picking and snapping and
said nothing about identity across edits — this section is that gap closed.*

| I want to… | Call | Line |
|---|---|---|
| Move one object | `move_object(page_index, object_index, dx, dy)` | 4483 |
| Delete one object | `delete_object(page_index, object_index)` | 4533 |
| Move a multi-object selection, ONE undo entry | `move_objects(page_index, object_indices: &[usize], dx, dy)` | 4574 |
| Delete a multi-object selection, ONE undo entry | `delete_objects(page_index, object_indices: &[usize])` | 4641 |
| Delete one anchor node | `delete_node(page_index, object_index, node_index)` | 4751 |
| Delete one subpath | `delete_subpath(page_index, object_index, subpath_index)` | 4770 |
| Delete one show operator (text run) | `delete_text_run(page_index, object_index, run_index)` | 4825 |
| Move one subpath | `move_subpath(page_index, object_index, subpath_index, dx, dy)` | 4875 |
| Drag one anchor node | `move_node(page_index, object_index, node_index, to: Point)` | 4939 |
| Drag a multi-node selection, ONE undo entry | `move_nodes(page_index, object_index, moves: &[(usize, Point)])` | 5001 |
| Drag a Bézier control point | `move_handle(page_index, object_index, node_index, handle: Handle, to: Point)` | 5057 |

⚠️ **Never loop the singular verbs over a selection.** Indices go stale between
iterations because each call re-splices the content stream, and N calls are N
undo entries. Use `move_objects` / `delete_objects` / `move_nodes`
(`edit.rs:4600-4620`).

### 1.11 Form-field authoring (5)

All five return `Result<FieldAuthorOutcome, EditError>` and are **ONE undo
entry** each — field dict + widget + baked `/AP` + page `/Annots` + `/AcroForm
/Fields` registration land together.

| I want to… | Call | Line |
|---|---|---|
| Add a text field | `add_text_field(&mut self, spec: &NewTextField)` | 7087 |
| Add a check box | `add_check_box(&mut self, spec: &NewCheckBox)` | 8043 |
| Add one member of a radio group | `add_radio_button(&mut self, spec: &NewRadioButton)` | 8253 |
| Add a push button | `add_push_button(&mut self, spec: &NewPushButton)` | 9414 |
| Add a list box / combo box | `add_choice_field(&mut self, spec: &NewChoiceField)` | 9633 |

`FieldAuthorOutcome` (`edit.rs:990`): `field_id: ObjId`, `merged: bool`,
`disclosures: FieldAuthorDisclosures`. **Read the disclosures** —
`edit.rs:1077-1090` records that a successful push-button creation yields *"the
only creation verb whose successful result is a control that does not work"*
(no action attached).

### 1.12 Form-field structure (7)

| I want to… | Call | Line | Returns |
|---|---|---|---|
| Delete a whole field | `delete_field(&mut self, fqn: &str) -> Result<FieldDeletion, EditError>` | 8464 | Every widget + dict + registration + emptied grouping nodes. |
| Preview a grouping-node deletion | `field_group_deletion_preview(&mut self, fqn) -> Result<FieldGroupDeletion, EditError>` | 8535 | ⚠️ Takes `&mut self` although it writes nothing. |
| Delete a grouping node and its subtree | `delete_field_group(&mut self, fqn) -> Result<FieldGroupDeletion, EditError>` | 8574 | Refuses a terminal with `NotAGroupingNode` — deliberately **not** redirected to `delete_field`. |
| Delete ONE widget of a field | `delete_widget(&mut self, fqn, index: usize) -> Result<FieldDeletion, EditError>` | 8764 | Siblings survive. |
| Rename a field | `rename_field(&mut self, fqn, new_partial: &str) -> Result<FieldRename, EditError>` | 8889 | `new_partial` is **one path segment**, never an FQN; a period is refused. |
| Move one widget's `/Rect` | `move_widget(&mut self, fqn, index, dx, dy) -> Result<WidgetMove, EditError>` | 9032 | **No appearance regeneration** — §12.5.5 step b makes matrix **A** a pure translation. |
| Read an existing field's copyable properties | `field_defaults(&self, source: &str) -> Result<FieldDefaults, EditError>` | 9211 | For `--defaults-from` / "copy style from". |
| **Change a field's field-scope properties** | `edit_field(&mut self, fqn, edit: &FieldEdit) -> Result<FieldEditOutcome, EditError>` | — | `Pass 134.0`. Flags, `/MaxLen`, `/TU`, `/Opt`. **Shared by every widget the field owns.** |
| **Change ONE widget's properties** | `edit_widget(&mut self, fqn, index, edit: &WidgetEdit) -> Result<WidgetEditOutcome, EditError>` | — | `Pass 134.0`. `/Rect` (move **and resize**), `/BS`, `/F`, `/MK` `/CA`. **Per placement.** ★ All four are **readable** too since `Pass 146.0` — `forms::Widget::rect` / `border` / `visibility` + `annot_flags` / `caption`. This row listed four writable properties for months while only two could be read, which is how a consuming shell ended up with two controls it could not honestly populate. See `03-capabilities.md`'s `Widget` block. |

#### ★ 1.12a Editing a field after it exists (`Pass 134.0`) — read this before wiring a properties pane

Until this Pass every property was settable **only at creation**, and the
only way to change one was to delete the field and place a new one — losing
its position, its name, its tab order and any value in it.

**The verbs come in two, and the split is not pdfce's invention.** Acrobat's
own scripting model states it: some properties *"apply to all widgets that
are children of that field"*, others *"are specific to individual widgets"*.

| scope | verb | what lives there |
|---|---|---|
| **field** — one write, every widget | `edit_field` | `required`, `read_only`, `tooltip`, `multiline`, `password`, `comb`, `max_len`, `no_toggle_to_off`, `radios_in_unison`, `combo`, `editable`, `multi_select`, `sort`, `options` |
| **widget** — per placement | `edit_widget` | `rect`, `border`, `visibility`, `caption` |

Getting it backwards is **invisible on the ordinary one-widget field and
wrong on every radio group**, where "the border" can only mean one button and
"required" can only mean the group.

**Both specs are `#[non_exhaustive]`, so use the builders**, not struct
literals — `FieldEdit::new().with_required(true).with_max_len(Some(8))`. A
property you do not name is **left alone**; there is no "reset to default",
because a default is not a thing a file records.

**★ The standard's producer gates are checked against the RESULT, not against
your request.** This is the part that is easy to get wrong from outside:

- `edit_field(f, FieldEdit::new().with_max_len(None))` on a **comb** field is
  refused with `CombPreconditionUnmet` — Table 228 permits `Comb` only when
  `/MaxLen` is present, and your request never mentioned comb.
- `edit_field(f, FieldEdit::new().with_combo(false))` on an **editable**
  drop-down is refused with `ChoiceEditWithoutCombo` — Table 230 permits
  `Edit` only alongside `Combo`.
- Supplying both halves in **one** edit is accepted: `with_comb(true)` plus
  `with_max_len(Some(8))` is exactly the precondition met.

**There is no type change and there never will be.** Acrobat has offered none
since Acrobat 6; pdfce makes it *unrepresentable* rather than returning an
error for it. Delete and re-place.

**★ What you must surface (rule 4).** Three property changes leave the stored
value inconsistent, and **Acrobat performs all three silently**:

| change | what happens |
|---|---|
| `/MaxLen` shortened below the current value | the field is over its own limit |
| a selected choice option removed | the selection points at nothing |
| a check box's export value changed while checked | it renders **unchecked** |

pdfce neither truncates the operator's data nor re-points their selection —
both would be inventing document state — and does not refuse the edit, because
shortening a limit is a legitimate authoring act. It reports
`FieldEditOutcome::value_no_longer_fits`, a ready-made sentence. **Show it.**

Also surface `widgets_affected` when it is `> 1` ("one field, three things on
screen changed"), `siblings_untouched` from `edit_widget` for the same reason
in reverse, and `sort_claim_unmet` — setting `Sort` over an unsorted `/Opt`
makes the file claim something untrue, and pdfce will not silently reorder a
list whose order Table 230 makes significant.

**On geometry, `edit_widget` vs `move_widget`.** `move_widget` takes a delta
and regenerates nothing, because §12.5.5 makes a pure translation exact for
free. `edit_widget`'s `rect` **replaces**, so it both moves and resizes — and
a changed *extent* rebuilds the appearance, because the same clause would
otherwise SCALE the old artwork into the new box (a text field dragged twice
as wide would render its text twice as wide rather than gaining room).
`WidgetEditOutcome::resized` says which happened; a translation through
`edit_widget` takes the cheap path automatically.

`WidgetEditOutcome::appearance_stale` is non-empty when a resize could not
rebuild the artwork — a push button's baked caption, or a signature. The
widget now renders **distorted**, and that string says so.

### 1.13 Form-field values (10)

| I want to… | Call | Line | Returns |
|---|---|---|---|
| Fill a text or choice field | `fill_text_field(&mut self, fqn, text) -> Result<FillOutcome, EditError>` | 12340 | Refuses a rich-text field (`FieldIsRichText`). |
| Fill a rich-text field, downgrading it | `fill_text_field_downgrading_rich_text(&mut self, fqn, text) -> Result<FillOutcome, EditError>` | 12384 | **Lossy and deliberate** — clears `/Ff` bit 26, deletes `/RV`. |
| Select a check box / radio state | `set_button_state(&mut self, fqn, on_state) -> Result<(), EditError>` | 12570 | Sets `/V` + every widget `/AS`. No regeneration. |
| Preview a form reset | `reset_preview(&self, only: Option<&[String]>) -> Vec<ResetPreviewRow>` | 12755 | Rows for **every** field in scope, including ineligible and already-at-default ones. Filtering is the shell's job. |
| Reset fields to defaults | `reset_form(&mut self, only: Option<&[String]>) -> Result<ResetOutcome, EditError>` | 12884 | `/V` is **removed**, not blanked. Never writes `/DV`. Never recomputes calculated fields. |
| Set a choice selection | `set_choice_value(&mut self, fqn, selections: &[&str]) -> Result<FillOutcome, EditError>` | 13138 | `/V` + `/I` + regenerated `/AP`. |
| Export filled data | `export_form_data(&self) -> Option<fdf::FormData>` | 13446 | `None` ⇒ no interactive form. |
| Import data | `import_form_data(&mut self, data: &fdf::FormData) -> Result<ImportOutcome, EditError>` | 13471 | ⚠️ **Each field is its own undo entry.** Unknown names are counted and skipped, never an error. |
| Regenerate appearances, clear `/NeedAppearances` | `regenerate_appearances(&mut self) -> Result<RegenOutcome, EditError>` | 13600 | ONE undo entry. |
| Flatten fields into page content | `flatten_fields(&mut self, names: Option<&[&str]>) -> Result<FlattenOutcome, EditError>` | 13730 | **Destructive.** ONE undo entry. Burns by overlay-append (§5.8). |

### 1.14 Form refusal preflights (5) — see §6.4 for whether they are load-bearing

| I want to… | Call | Line | Returns |
|---|---|---|---|
| Ask whether filling would be refused | `fill_refusal(&self) -> Option<EditError>` | 12200 | ⚠️ **Strict subset** of what a fill enforces. |
| Ask whether field/widget deletion would be refused | `deletion_refusal(&self) -> Option<EditError>` | 12239 | |
| Ask whether a rename would be refused | `rename_refusal(&self) -> Option<EditError>` | 12268 | |

`edit.rs:12220-12222`: *"**there are documents where filling is offered and
deletion is refused.** They are not rare — a certified fillable form is the
ordinary case."* Gating a Delete control on `fill_refusal` ships a button that
always errors.

| Why a flatten would refuse, before attempting it | `flatten_refusal(&self) -> Option<EditError>` | 13915 | `None` when a flatten would proceed. |
| Where a page's widgets are | `widget_rects(&self, page_index: usize) -> Vec<(ObjId, [f64; 4])>` | 17893 | Annotation id and `/Rect`. A **query**, not an edit — useful for hit-testing and for reporting orphans (see `insert_pages`). |

### 1.15 Annotations (13) — detail in part 3

| I want to… | Call | Line | Returns |
|---|---|---|---|
| Author a geometric markup | `add_markup(&mut self, page_index, spec: &MarkupSpec) -> Result<ObjId, EditError>` | 9986 | New annotation id. Exactly `add_markup_with(.., &MarkupOptions::default())`. |
| Author a geometric markup **at an opacity** | `add_markup_with(&mut self, page_index, spec: &MarkupSpec, options: &MarkupOptions) -> Result<ObjId, EditError>` | — | `Pass 81.1`. `MarkupOptions { opacity: Option<f64> }` writes §12.5.2 Table 164 `/CA` onto the annotation dictionary. **One verb, one undo entry** — see the note below. |
| Author a text-bearing annotation | `add_text_annotation(&mut self, page_index, spec: &TextAnnotSpec) -> Result<ObjId, EditError>` | 12034 | FreeText / Text+`/Popup` / Stamp. Exactly `add_text_annotation_with(.., &MarkupOptions::default())`. |
| Author a text-bearing annotation **at an opacity** | `add_text_annotation_with(&mut self, page_index, spec: &TextAnnotSpec, options: &MarkupOptions) -> Result<ObjId, EditError>` | — | `Pass 81.1`. The twin of the above, shipped in the same Pass because Table 164 is the **markup-annotation** entry list and a sticky note is a markup annotation. `/CA` goes on the parent, never on its `/Popup`. |
| Author a `/Redact` mark (non-destructive) | `add_redaction(&mut self, page_index, spec: &RedactSpec) -> Result<ObjId, EditError>` | 10480 | A **mark**. Nothing is removed yet. |
| Un-mark a redaction | `delete_redaction_mark(&mut self, annot_id) -> Result<(), EditError>` | 10617 | Refuses any non-`/Redact` annotation. |
| Delete any annotation | `delete_annotation(&mut self, annot_id) -> Result<AnnotationDeletion, EditError>` | 10847 | **Routes** to the two specialised verbs above for `/Redact` and ce dimensions. |
| Preview an annotation deletion | `annotation_deletion_preview(&self, annot_id) -> Result<AnnotationDeletion, EditError>` | 11316 | Pure `&self` query. |
| Ask whether annotation deletion is refused document-wide | `annotation_deletion_refusal(&self) -> Option<EditError>` | 11492 | ⚠️ Takes no `annot_id`, so it cannot see the three per-annotation refusals. |

| Restyle an existing markup annotation | `set_markup_style(&mut self, annot_id: ObjId, style: &MarkupStyle) -> Result<MarkupStyleChange, EditError>` | 12372 | Rebuilds the baked `/AP`. |
| Set the `/QuadPoints` corner order | `set_quad_point_order(&mut self, order: QuadPointOrder)` | 5476 | ⚠️ **Session state, not a per-call argument.** Governs what is AUTHORED from now on; does **not** sweep the document. ~~*"decision 062 fixes markup authoring at one entry point, so an `add_markup_with` would be a second"*~~ — **corrected 2026-08-27**: `add_markup_with` now exists and is **not** a second entry point (see §1.15.1). The ruling stands on its own ground: quad order is a **document-wide convention**, so a per-call argument would let two annotations in one file disagree about what UL/UR/LL/LR means, which is the divergence `Pass 62.x` exists to prevent. |
| Read it back | `quad_point_order(&self) -> QuadPointOrder` | 5482 | Defaults to `ReadingOrder` — what Acrobat, PDFBox and pdf.js emit and expect. |

#### 1.15.1 ★ `add_markup_with` is not a second entry point, and decision 062 is intact

`Pass 81.1` added `add_markup_with` / `add_text_annotation_with`. **Decision
062 refused a second markup entry point by name**, so this needs saying
plainly rather than leaving a reader to reconcile the two.

**What 062 refused** was `add_markup_appearance` — a hatch letting a caller
hand the engine a **prebuilt annotation dictionary plus appearance stream**.
That would have bypassed four guards, three of which are document-safety
properties whose failure is *silent in the saved file*: an encrypted
document, a certified document that forbids annotation, and a document whose
`/Size` is hiding objects.

**What `add_markup_with` does** is take one more **validated scalar**
alongside the same `MarkupSpec`. There is **one body**: `add_markup`
delegates to `add_markup_with`, which validates and delegates to the private
`add_markup_inner`, which runs all four guards and calls the same
`annot_author::build_appearance` builder. Nothing is bypassed and nothing is
caller-supplied that the engine does not check — the Pass in fact adds a
*fifth* validation.

062's own words are the test: *"a guard that a second entry point can bypass
is not a guard, it is a convention."* This entry point bypasses no guard.

**Why it exists at all is an UNDO defect, not ergonomics.** `/CA` used to be
reachable only through `set_markup_style`, a **restyle** verb, so authoring a
40 %-opaque highlight meant *author opaque, then restyle* — **two undo
entries**. One Ctrl+Z left an **opaque highlight** on the page: a state the
operator never asked for and could not have created any other way.

**Why an options struct rather than a field on `MarkupSpec`.** The
requesting shell asked for `opacity: Option<f64>` on `MarkupSpec`.
`MarkupSpec` is an **enum of eight geometric variants**, so that is eight
copies of one field — and, worse, it puts a whole-annotation property
alongside `rect`, `border_width` and `endings`, which describe what the
appearance *draws*. `/CA` does not affect what the appearance draws:
§12.5.2 Table 164 makes it the alpha with which the **annotation is
composited onto the page**, and pdfce's generated appearances deliberately
leave their own graphics-state alpha at 1.0 so the two cannot compound.
`MarkupOptions` is shaped to carry `/T`, `/Contents`, `/NM` and `/M` as they
land.

**Three behaviours a shell should not have to discover:**

| | |
|---|---|
| `None` | **omits `/CA` entirely.** Table 164's default is 1.0, so writing it would add a key that changes nothing and make a pdfce-authored opaque annotation textually distinguishable from every other producer's. |
| `Some(1.0)` | **written.** Renders identically to `None` and is *not* the same bytes — a caller round-tripping an explicit 1.0 gets its key back, because collapsing it would be pdfce deciding what the caller meant. |
| out of range / `NaN` | **refused by name** (`EditError::MarkupOpacityOutOfRange`), **nothing authored**, session untouched. This is the *opposite* of `MarkupStyle::opacity`, which **clamps** — deliberately. A restyle corrects a value on an annotation the operator can see, so clamping keeps it renderable and visibly changes it; an author call with alpha 4.0 is a caller bug, and clamping would put a fully opaque annotation on the page while returning `Ok`. |

**`MarkupOptions` is deliberately not `#[non_exhaustive]`**, for the same
reason `MarkupStyle` is not: it is an INPUT struct a consumer *constructs*,
and `#[non_exhaustive]` would make it unbuildable from outside `pdfce-core`.
Adding a field is a breaking change; that is the honest price, paid here
rather than pushed onto every consumer as an unconstructable type.

### 1.16 Search-driven redaction marking (5)

| I want to… | Call | Line | Returns |
|---|---|---|---|
| Mark every literal occurrence | `mark_redactions_by_search(&mut self, query, case_insensitive) -> Result<Vec<ObjId>, EditError>` | 11512 | Created mark ids. **Matches LITERALLY.** |
| …with full options | `mark_redactions_by_search_with(&mut self, query, &TextSearchOptions) -> Result<Vec<ObjId>, EditError>` | 11556 | |
| …**and hear what the scan could not read** | `search_and_mark_redactions(&mut self, query, &TextSearchOptions) -> Result<RedactionMarking, EditError>` | 16199 | ✅★★ **Use this for any operator-facing redaction.** `created.is_empty()` cannot distinguish “the term is absent” from “this document's text was never recoverable as Unicode” — and on a redaction path those demand opposite reactions. Read `diagnostics.ladder_failures`, `.type3_fonts_without_to_unicode`, `.identity_fonts_without_to_unicode`. `Pass 127.1`. |
| …with an explicit mark appearance | `search_and_mark_redactions_styled(&mut self, query, &TextSearchOptions, &RedactAppearance)` | 16217 | Same, plus the fill / overlay text / quadding the operator chose. |
| Mark by simple pattern | `mark_redactions_by_pattern(&mut self, pattern, case_insensitive) -> Result<Vec<ObjId>, EditError>` | 11584 | `#` = ASCII digit, `?` = any char, everything else literal. `###-##-####` ⇒ SSN-shaped runs. |

| Mark by search, choosing the mark's appearance | `mark_redactions_by_search_styled(&mut self, query: &str, options: &TextSearchOptions, appearance: &annot_author::RedactAppearance) -> Result<Vec<ObjId>, EditError>` | 13201 | Ids of the marks created. |
| Mark by regex, choosing the mark's appearance | `mark_redactions_by_pattern_styled(&mut self, pattern: &str, case_insensitive: bool, appearance: &annot_author::RedactAppearance) -> Result<Vec<ObjId>, EditError>` | 13254 | |

### 1.17 Text search (2)

| I want to… | Call | Line | Returns |
|---|---|---|---|
| Find text (legacy shape) | `find_text(&mut self, needle, case_insensitive) -> Vec<TextMatch>` | 11792 | ⚠️ Hard-codes `with_wildcards(true)`. **Not what a Find bar wants.** |
| Find text with options | `find_text_with(&mut self, needle, &TextSearchOptions) -> Vec<TextMatch>` | 11853 | ✅ Use this. `TextSearchOptions::wildcards` defaults to `false`. |
| Search text **and hear what it could not read** | `search_text(&mut self, needle, &TextSearchOptions) -> TextSearch` | 16450 | ✅★ **Prefer this over `find_text_with` for any operator-facing search.** Returns `TextSearch { matches, diagnostics }` — the same hits, plus the extraction's `TextDiagnostics`. `matches.is_empty()` alone cannot tell *“the needle is absent”* from *“this document's text was never recoverable as Unicode”*; `diagnostics.type3_fonts_without_to_unicode`, `.identity_fonts_without_to_unicode` and `.ladder_failures` can. `Pass 127.0`. |

Both take `&mut self` despite changing nothing (they read `self.view()`).
`TextMatch` = `{ page_index, quad: Quad, text: String }` (`edit.rs:6080`).

### 1.18 Attachments (2)

| I want to… | Call | Line | Returns |
|---|---|---|---|
| Embed a file | `attach_file(&mut self, name, bytes, description: Option<&str>) -> Result<ObjId, EditError>` | 10147 | §7.11.4.1 route 2 (`/EmbeddedFiles` name tree). ONE undo entry. |
| Remove an attachment | `detach_file(&mut self, key: &[u8]) -> Result<(), EditError>` | 10347 | By name-tree key. ⚠️ **Not a redaction verb** — see §5.4. |

`AttachmentTreeUnsupported` (`edit.rs:2374`) is a refused name-tree shape;
`AttachmentNotFound` (`edit.rs:2386`) is an unknown key.

### 1.19 Outline / bookmarks (1)

> #### ★ Added 2026-08-19 — `Pass 103.0`
>
> Requested by `pdfceGUI`, which needed to carry a source document's
> bookmarks across `insert_pages` and found that pdfce **could read outlines
> and not write them**: `read_outline`, `parse_outline` and the walk have
> existed since the reader passes with zero authoring verbs opposite them.

| I want to… | Call | Returns |
|---|---|---|
| Add a bookmark | `add_outline_item(&mut self, parent: Option<ObjId>, title: &str, destination: Option<Destination>) -> Result<ObjId, EditError>` | The new item's id. Creates `/Outlines` if absent. ONE undo entry. |

`parent: None` means top level; otherwise the id of an existing item, which
you get from `read_outline`. The item is appended **last** among its
siblings. `/Prev`, `/Next`, `/First` and `/Last` are maintained for you.

#### ★★ `/Count` is two different quantities, and you will read it wrong once

This is the one thing to carry away from this section, and it is the reason
the verb exists rather than the shell assembling the dictionaries itself.

| | root `/Outlines` | an item |
|---|---|---|
| `/Count` counts | visible items at **every** level, **including** top-level | visible **descendants**, **excluding** itself |
| absent means | no open items | the item is a **leaf** |

On an item the **sign is the open/closed flag** — §12.3.3 defines no `/Open`
key, so the sign is the only carrier. Negative means collapsed, and the
magnitude is the count it *would* have if expanded.

pdfce propagates all of this. What the shell must know is that **the total
does not always go up**: an item added under a collapsed ancestor is not
visible, so the root count is unchanged. That is correct, and a UI that
reported "1 bookmark added" by diffing the root count would report zero.

#### What is refused, and why by name

| Case | Error |
|---|---|
| `parent` is not an outline item | `OutlineItemNotFound { id }` |
| `Destination::Named` / `Remote` | `UnsupportedDestination { kind: "named" \| "remote" }` |
| `DestView::Unknown` / `Absent` | `UnsupportedDestination { kind: "unknown-fit" \| "no-fit-style" }` |
| destination page past the end | `PageOutOfRange { index, count }` |

Only an **explicit** page destination is authored today. The rest are refused
rather than dropped because a bookmark with no destination still appears in
the panel and still looks clickable — the failure would reach the operator as
*"this bookmark does nothing"*, which reads as a viewer bug.

`DestView::Unknown` is the one that looks writable and is not: the reader
keeps an extension's fit **name** but discards its parameters, so re-emitting
it would produce `[page /FitSomething]` with the arguments silently gone.

A bad `parent` is refused rather than re-parented to the root, because a
stale handle and a deliberate top-level bookmark are indistinguishable once
the item has landed — and `parent: None` already says "top level".
`parent` validity is decided by **walking `/Parent` to the outline root**,
not by inspecting keys: every *page* also has `/Parent`, so a key-presence
check accepts a page and splices the bookmark into the page tree, where no
viewer looks for it. That save succeeds and the bookmark does not exist.

CLI: `pdfce-cli add-bookmark --title … [--page N] [--top Y] [--under n]`,
where `--under` takes the `n=` number `list-outline` prints. **The indices
shift after every add** — they are positions in the current tree, not stable
handles — so a batch of adds must re-list between them.
### 1.20 Form-field adoption (1)

> #### ★ Added 2026-08-19 — `Pass 103.1`
>
> Requested by `pdfceGUI`: *"`add_text_field` and friends **author a new**
> **widget**; we need to register an **existing** widget into `/AcroForm`."*
> The orphans `insert_pages` reports had correct geometry, correct
> appearance and correct values — everything except an owner.

| I want to… | Call | Returns |
|---|---|---|
| Adopt a widget | `adopt_widget(&mut self, widget: ObjId, name: Option<&str>) -> Result<AdoptOutcome, EditError>` | `AdoptOutcome { field_id, name, field_type, renamed, acroform_created }`. Creates `/AcroForm` if absent. ONE undo entry. |

**It writes no geometry, appearance or value** — only the `/AcroForm`
registration, plus `/T` when you rename. That is the point; the widget is
already correct.

`field_id` is **the same id as the widget**. Not an oversight: a merged
field-widget is one dictionary serving as both, so there is no separate
field object.

#### ★★ Two orphans that look identical and are not

Measured against a real AcroForm (`examples/orphan_probe.rs`, pdfbox corpus,
12 fields over 13 widgets). Of the 13 widgets an insert orphans:

| shape | count | what it carries | outcome |
|---|---|---|---|
| **merged field-widget** | 11 | its own `/FT`, `/T`, `/V`, `/DA` | adopts **losslessly** |
| **bare kid** (radio group) | 2 | no field keys at all; only `/Parent`, which the copy drops | **cannot** be adopted |

`InsertOutcome::orphaned_widgets_unrecoverable` counts the second row — new
in this Pass, and additive, so it does not break the struct you already
wired. **Warn on it before the operator starts adopting the others**, because
*"11 controls need re-registering"* is a chore and *"2 controls have lost
their identity and the only copy is in the file you inserted from"* is a
decision, and the undifferentiated total states only the chore.

A bare kid is refused with `WidgetHasNoFieldIdentity` unless you supply a
`name`. Guessing would pick a name the source never used — and for a radio
group it would turn one mutually-exclusive field into several independent
ones, a form that looks right and behaves wrong.

The predicate is **`/T`, not `/FT`**. §12.7.3.1 makes `/FT` *inheritable*, so
a widget can resolve a type through an ancestor and still have no name; once
adopted at the top level there is no ancestor left. A field with no name
cannot be filled, exported or referred to, so surviving `/FT` is decoration
on something unaddressable, not partial recovery.

#### What is refused

| Case | Error |
|---|---|
| no `/T` and no `name` | `WidgetHasNoFieldIdentity { id }` |
| not a `/Subtype /Widget` in this document | `NotAWidget { id }` |
| already reachable from `/AcroForm` `/Fields` | `WidgetAlreadyOwned { id }` |
| resulting name already exists | `FieldNameTaken { name }` |
| empty `name` | `FieldNameEmpty` |

**The collision refusal is the non-obvious one.** §12.7.3.1 makes the fully
qualified name the field's *identity*, so two top-level fields called
`Address` are **one field with two widgets** — typing in either fills both.
No viewer reports it; the operator finds it by typing. `pageops::assemble`
meets the same problem on merge and auto-renames (`form_fields_renamed`);
here the caller renames, because a shell adopting one widget at a time can
ask, and `Address_2` is a name nobody chose.

`field_type: None` means no `/FT` at all — legal, but once top-level there is
nothing to inherit from, so no viewer knows how to render or fill it. Worth
surfacing.

`renamed` and `acroform_created` **cannot be recovered by re-reading the**
**document**: a renamed widget looks like one that always had that name, and
a fresh `/AcroForm` looks like an old one. This struct is the only
disclosure.

CLI: `pdfce-cli adopt-widget --page N --index I [--name X]`, addressed by the
`page=`/`index=` pair `list-annotations` prints — an unregistered widget has
no name for `list-fields` to show. Note that **`pdfce-cli insert-pages` does
not produce orphans**: it calls `pageops::assemble`, which merges `/AcroForm`
(and reports `fields_renamed=` / `fields_dropped=`). Only
`EditSession::insert_pages` orphans.
#### ★★ `merge_document` — the merge that keeps the undo log (`Pass 106.0`)

| I want to… | Call |
|---|---|
| Merge a whole document in | `merge_document(&mut self, source: &DocumentView, position: InsertPosition) -> Result<MergeOutcome, EditError>` |

`MergeOutcome { pages_merged, fields_merged, fields_renamed, acroform_created }`.
ONE undo entry, `CommandKind::MergeDocument`.

**Why it exists.** `pdfceGUI` drew Merge and wired it to
`command-unimplemented`, because the only merge pdfce offered
(`pageops::insert`) **returns a whole new document's bytes** — wiring that
into an open editor discards the undo log. They shipped an inert button and
said so rather than silently eat the operator's history. That was right, and
this is the answer.

★ **It surfaced only when `FEATURES.md`'s `gui` column was re-based onto
their build and the Merge row went DOWN.** A capability marked present
against a crate nobody used had been hiding a real API-shape gap here.

#### ★ Why a WHOLE-document merge is strictly easier than a page subset

The observation the verb is built on, and it is not obvious.

`insert_pages` cannot carry `/AcroForm` because a field's `/Kids` may reach
widgets on pages that are **not** being inserted — carrying it would either
fracture the field or drag in widgets nobody asked for. That is `Pass 102.1`,
and it is why `InsertOutcome::orphaned_widgets` is permanent rather than
interim.

**Merging every page makes that case impossible.** No field can straddle a
boundary when there is no boundary. Same copy path, differing only in taking
every page.

#### The ordering that makes it work

Pages first, **then** the field tree, through the **same mapping table**. A
field's `/Kids` then resolve to the widgets the pages already brought across.
Importing fields first — or with a second mapping — silently **doubles every
widget**, and the result renders correctly, which is how it would survive
review. There is a test asserting object *identity* between the page's
`/Annots` and the fields' `/Kids` for exactly this reason.

Each merged widget then gets `/Parent` written back to its field. `/Parent`
is dropped on the way in (see `import_dict`), and §12.7.3.2 resolves `/FT`,
`/Ff`, `/V` and `/DA` by walking **up** — a viewer hit-testing a click lands
on the *widget*. Without `/Parent` the control draws, accepts a click and
belongs to nothing.

⚠ **`parse_acroform` cannot see this.** It walks downward (`/Fields` →
`/Kids`) and never reads `/Parent`, so a merge with the back-links missing
passes every assertion routed through it. Verified against raw bytes instead.

#### Collisions are RENAMED, not merged — and that differs from `adopt_widget`

§12.7.3.1 makes the fully qualified name the field's identity, so two
top-level `Address` fields are **one** field with two widgets and filling
either fills both. On a merge that is almost never meant: two documents that
each have an `Address` have two different addresses. So an arrival is renamed
(`Address` → `Address_2`) and counted in `fields_renamed`.

`adopt_widget` **refuses** the same collision. The difference is the caller's
knowledge: adopting one widget is a decision an operator is making now and
can be asked about; merging a hundred-field document is not, and refusing the
whole merge over one name is worse than a suffix.

`/NeedAppearances` is carried as a **logical OR**, never overwritten — it
means *"the appearances here may be stale"*, so if either document says so,
the result must. `/DR` merges key-by-key with the **target winning**, because
a target entry is already referenced by fields that were here first.
#### `merge_document` also carries navigation (`Pass 106.1`)

`MergeOutcome` gained three more fields: `named_destinations_carried`,
`named_destinations_renamed`, `outline_items_carried`.

★ **Destinations are carried BEFORE outlines, and the order is the feature.**
A source bookmark may point at a named destination whose key collided and had
to be suffixed here; only the rename map from the destination carry can
rewrite that bookmark onto the new key. Run it the other way and the bookmark
keeps a key nothing defines — which renders as a bookmark that does nothing,
the exact failure `add_outline_item` refuses a forward reference to avoid.

**Outlines arrive as top-level siblings**, appended after this document's own.
pdfce does **not** invent a "merged document" heading to nest them under:
that heading would be a bookmark pdfce authored, appearing in the operator's
panel beside bookmarks the documents' authors wrote.

`named_destinations_renamed` is worth surfacing for a reason beyond the
`fields_renamed` one: pdfce rewrites the **carried** bookmarks onto the new
keys, but **cannot rewrite a link it did not copy**. A `/GoToR` in a third
file naming the old key now resolves to *this* document's destination rather
than the source's.

**Still not carried:** page labels (decision 072 governs the policy) and
`/OCProperties`.
#### ★ `adopt_preview` — ask before pressing (`Pass 103.4`)

| I want to… | Call |
|---|---|
| Ask what adoption would do | `adopt_preview(&self, widget: ObjId, name: Option<&str>) -> Result<AdoptOutcome, EditError>` |

Same signature as `adopt_widget` with `&self`. Writes nothing, decides
nothing, and **shares the verb's entire body** (`adopt_plan`) rather than
re-implementing its guards — so "the preview said it would work and the call
refused" is not a state this code can reach.

Requested by `pdfceGUI` because the two widget shapes are **indistinguishable
from the outside**: one adopts losslessly and *recovers the field exactly*,
the other refuses and can only be turned into a **new, empty, typeless field
that is not the control that was lost**. Without this, a UI finds out by
pressing — a control whose applicability is only knowable after you use it.

Two of `AdoptOutcome`'s fields are inputs to the decision rather than
confirmations of it:

| field | why it belongs before the press |
|---|---|
| `name` | it is **in the file, not on screen**. *"Register as `Address`"* is a decision; *"Register"* is a guess |
| `field_type: None` | registration will **succeed and the field still will not be fillable** — `/FT` is inheritable and a top-level field has no ancestor left. Saying so afterwards tells an operator their successful action did not do what they wanted |

**It subsumes a refusal predicate.** A narrower
`adopt_refusal(..) -> Option<EditError>` was requested, matching
`fill_refusal` and `rename_refusal`. It was not shipped: `adopt_preview(w,
n).err()` *is* that predicate with more information in it, and two entry
points for one question is a cost paid forever. The substitution is tested,
not assumed.

CLI: `pdfce-cli adopt-widget --dry-run` (mutually exclusive with `--output`,
which becomes optional).

#### `source_outline_dropped` — the fourth insert disclosure

`InsertOutcome` gained one more additive `bool`: the source carried a
document outline whose bookmarks did not come across. Pure symmetry with
`source_page_labels_dropped`, requested for the same reason — the shell's
insert sentence named bookmarks and page labels **unconditionally**, and on a
CAD drawing with neither that is a paragraph about two things that never
existed, which is how an operator learns to stop reading it.

`/Outlines` is a catalog entry, unreachable from any page, so the copy never
sees it — the bookmarks are not lost in transit, they were never in the set
being copied. Carrying them means reading the source outline and replaying it
through `add_outline_item`, which is what `Pass 103.0` exists for.
#### ★ Page labels on insert — `Pass 103.2`, a measured divergence from Acrobat

`InsertOutcome` gained two more fields (both additive, `#[non_exhaustive]`):

| field | means |
|---|---|
| `source_page_labels_dropped` | the source had a `/PageLabels` tree; its labels did not come across |
| `page_labels_stale` | the target has one, and its ranges now describe different physical pages |

**pdfce writes nothing to `/PageLabels` on an insert.** That is deliberate,
and it is not what Acrobat does.

`Acrobat_Features/core_ops__page_labels_and_bates_interaction.md`
(2026-08-19; three independent Adobe Community threads, 2024–2025) found a
third behaviour neither obvious option predicts: Acrobat **actively
overwrites** every inserted page with a static copy of the label displayed on
the target page immediately *preceding* the insertion point — not the
source's label, not an incrementing continuation, the same single string on
all of them. The sourced case: a twelve-page chapter labelled `10-1`…`10-12`,
inserted after a page labelled `9-45`, came out with all twelve showing
`9-45`.

That is a wrong label on every inserted page, written silently, and the
threads it is sourced from are complaints about it. Matching it would be
matching a defect. pdfce leaves the tree alone — so inserted pages continue
whatever range already covered that position, which is what §12.4.2's
per-page computation gives on its own — and reports the two facts instead.

The labels are not **carried** either, for the reason
`pageops::assemble` already gives for its `page_labels_dropped`: a label tree
describes *physical page positions*, so carrying one onto a subset inserted
at an arbitrary offset yields labels confidently wrong about pages that are
not in the file. `pageops::assemble` exposes `carry_page_labels` for callers
who want the other answer with their eyes open; `EditSession::insert_pages`
has no options parameter and takes the conservative one.

The two flags are separate because **the remedies differ** — a stale tree
wants renumbering, a dropped one wants creating. A merged "something is wrong
with page labels" would name neither.
### 1.21 Named destinations (1)

> #### ★ Added 2026-08-19 — `Pass 103.3`
>
> Requested by `pdfceGUI` so a carried bookmark that points at a named
> destination resolves. `add_outline_item` shipped one Pass earlier refusing
> `Destination::Named` **by name**, which was the honest interim — CAD and
> Word exports use named destinations often enough that any real outline
> carry hits it.

| I want to… | Call | Returns |
|---|---|---|
| Define a named destination | `add_named_destination(&mut self, name: &[u8], destination: Destination) -> Result<(), EditError>` | Creates the `/Names` `/Dests` tree if absent. ONE undo entry. |

`add_outline_item` now **accepts** `Destination::Named`, so the pair is:
define the name, then point a bookmark at it.

#### ★★ Keys are BYTES, and the verb takes `&[u8]` for a reason

§7.9.6 imposes **no** character encoding on name-tree keys — *"any encoding
of the keys may be used as long as it is self-consistent."* Real keys are
UTF-16BE-with-BOM, PDFDocEncoded, a legacy platform encoding, or opaque. A
`&str` API would force a lossy round trip and destroy the ability to match a
key byte-for-byte against one already in a file.

ASCII callers pass `b"chapter1"` or `s.as_bytes()`. A caller **carrying a key
read out of another document** — which is the whole point of `Pass 103.3` —
passes it through untouched.

`pdfce-cli add-named-dest --name` takes text and hands over its UTF-8. That
narrowing belongs in the shell, not the engine: a key typed at a prompt is
text by construction; a key copied between documents is not.

#### The bookmark keeps the NAME — it is not resolved and baked

The single most important property, and the one a test cannot see through
the reader. `read_outline` **resolves** a defined name and reports
`Destination::Page`; `Destination::Named` in reader output means the key
resolved to *nothing*. So a writer that baked `[page /Fit]` in at author time
and a correct writer produce **identical reader output**.

Baking defeats exactly what §12.3.2.3 exists for: the indirection is what
lets links survive a page reorder. A baked bookmark is correct today and
silently wrong after the next one.

#### Which namespace, and why the collision check spans both

§12.3.2.3 defines two: the PDF 1.1 catalog `/Dests` **dictionary** (name-object
keys) and the PDF 1.2 `/Names` → `/Dests` **name tree** (string keys). The
type of the `/Dest` value *is* the namespace selector — the only discriminator
the standard gives.

pdfce **reads both** and **writes only the tree**. The two have no defined
precedence, so a key present in both is an anomaly pdfce's own reader already
reports (`cross_namespace_resolutions`); writing to the legacy dictionary
would be manufacturing it. The collision check therefore spans **both**
namespaces even though only one is ever written.

#### What is refused

| Case | Error |
|---|---|
| key already defined, either namespace | `NamedDestinationTaken { name }` |
| bookmark names an undefined key | `NamedDestinationNotFound { name }` |
| empty key | `FieldNameEmpty` |
| existing `/Dests` tree has `/Kids` | `NameTreeUnsupported` |
| non-explicit destination | `UnsupportedDestination { kind }` |

`NamedDestinationTaken` is checked by **membership, not reachability.**
`resolve_destination` folds undefined / dangling / remote all into `None` —
correct for its callers — so a writer reusing it would silently overwrite a
**defined but dangling** key, re-aiming every existing link that names it at
a page nobody chose. `DestinationResolver::lookup` exists as the separate
membership query, added in this Pass.

`NameTreeUnsupported` is a multi-node tree. pdfce reads those; it will not
rebuild one. Inserting correctly means splitting a leaf and repairing every
ancestor's `/Limits`, and getting it wrong yields a tree whose binary descent
**silently misses keys that are present** — a failure that looks like a
missing destination rather than a damaged tree.

#### The tree pdfce writes

A **single root node with `Names` only** — legal per Table 36, and the shape
every reader handles. **No `/Limits`**: Table 36 scopes that key to
intermediate and leaf nodes, and a root carrying one is among the malformed
shapes the spec digest's `NT-A1` records.

Keys are re-sorted on every insert using `[u8]`'s own `Ord`, which **is**
§7.9.6's rule (unsigned byte comparison, shorter prefix first) — so the sort
needs no comparator and cannot drift from the standard.

CLI: `pdfce-cli add-named-dest --name X --page N [--top Y]`, then
`add-bookmark --dest-name X` (mutually exclusive with `--page`).
### 1.22 ce dimensions (22) — detail in part 3

> #### ★ Group membership, renaming and deletion — added 2026-08-19
>
> Requested by `pdfceGUI` for the *Manage dimension groups* window and the
> Format tab's **Group** control, both of which had no verb behind them: a
> group could be created and never renamed or deleted, and a placed ce
> dimension took its `GroupId` at creation with no way to change it.
>
> | I want to… | Call | Returns |
> |---|---|---|
> | Rename a group | `rename_dimension_group(&mut self, group: GroupId, name: &str) -> Result<(), EditError>` | Metadata only — **no appearance is regenerated**, because the name is not drawn. Names are **not** required to be unique; `GroupId` is the identity. |
> | Delete an empty group | `delete_dimension_group(&mut self, group: GroupId) -> Result<(), EditError>` | Refuses a populated group with `DimensionGroupNotEmpty { members }`. |
> | Delete, answering the members question | `delete_dimension_group_with(&mut self, group: GroupId, policy: GroupDeletion) -> Result<usize, EditError>` | Count of members reassigned. ONE undo entry. |
> | **Move a dimension to another group** | `set_dimension_group(&mut self, dimension: DimensionId, group: GroupId) -> Result<(), EditError>` | ★ **RE-MEASURES it** — see below. |
>
> ★★ **`set_dimension_group` is not a field assignment, and a shell must not
> treat it as one.** A ce dimension's label is derived from its GROUP's scale,
> precision, unit and standard (the decision 011 §2.3 cascade). Re-parenting
> therefore changes what the dimension **reads**, and the verb regenerates the
> baked `/AP`, `/Rect`, `/Contents`, `/Measure` and `/L` through the one shared
> path. Undo restores the label as well as the group id.
>
> ★ **`GroupDeletion` has no `DeleteMembers`, deliberately.** Deleting a
> dimension also removes its annotation from the page's `/Annots` array, and
> doing that here would mean a second implementation of `delete_dimension`'s
> removal — while calling it in a loop would produce one undo entry per
> member, so undoing a group deletion would take forty presses and could stop
> halfway with the group already gone. `Reassign` covers the case an operator
> reaches for; the rest is a follow-on that factors `delete_dimension`'s core
> into a helper.
>
> **A ce dimension without a group cannot be measured or drawn** — it has no
> scale, format or unit — which is why the members question has no quiet
> default and the deletion refuses rather than orphaning them.

| I want to… | Call | Line | Returns |
|---|---|---|---|
| Read the sidecar model | `dimension_model(&self) -> DimensionModel` | 15361 | Overlay-aware; a fresh model if none stored. |
| Author a ce dimension | `add_dimension(&mut self, page_index, group: GroupId, kind: DimensionKind) -> Result<(ObjId, DimensionId), EditError>` | 15380 | Annotation id + model id. ONE undo entry. |
| Create a group | `add_dimension_group(&mut self, name, unit: Unit) -> Result<GroupId, EditError>` | 15523 | Scale-never-set, visible, OCG allocated lazily. |
| Set a group's scale + number format | `set_group_scale(&mut self, group, scale: ScaleState, format: NumberFormat) -> Result<usize, EditError>` | 15549 | **Count of members regenerated.** |
| Toggle a group's layer | `toggle_dimension_layer(&mut self, group, visible) -> Result<bool, EditError>` | 15600 | Resulting visibility. The default group is un-hideable. |
| Hit-test ce dimensions on a page | `dimension_rects(&self, page_index) -> Vec<(DimensionId, [f64;4])>` | 15645 | `[llx, lly, urx, ury]` page space. |
| List groups present on a page | `dimension_groups_on_page(&self, page_index) -> Vec<GroupId>` | 15725 | Model order. |
| **Drag** a ce dimension | `place_dimension(&mut self, dimension, offset: f64, text_along: f64) -> Result<(), EditError>` | 15804 | ✅ **This, not `move_dimension`, is what dragging does.** Value-preserving by construction. |
| Toggle radius ↔ diameter | `set_dimension_display(&mut self, dimension, show_diameter: bool) -> Result<(), EditError>` | 15921 | ⚠️ **Commits even when nothing changes** (opposite of `set_info_field`). |
| Set a group's drafting standard | `set_group_standard(&mut self, group, standard: DimStandard) -> Result<usize, EditError>` | 15983 | Count of members regenerated. |
| Set a group's style defaults | `set_group_style(&mut self, group, style: GroupStyle) -> Result<usize, EditError>` | 16052 | ⚠️ **Count REGENERATED, not count MOVED.** See §8, trap T-00. |
| Set one ce dimension's overrides | `set_dimension_style(&mut self, dimension, style: StyleOverrides) -> Result<usize, EditError>` | 16115 | ⚠️ Count of **properties overridden afterwards** — a different unit from the sibling above, same type. |
| Delete a ce dimension | `delete_dimension(&mut self, dimension) -> Result<(), EditError>` | 16178 | `/Annots` ref + dict + `/AP` + sidecar record. Group survives. |
| **Translate** a ce dimension | `move_dimension(&mut self, dimension, dx, dy) -> Result<(), EditError>` | 16779 | ⚠️ Translates the **measured points** — takes the ce dimension off the feature it was measuring. |

| **Move one vertex** of a ce dimension | `move_dimension_vertex(&mut self, dimension, index: usize, dx, dy) -> Result<VertexOutcome, EditError>` | 22304 | ⚠️ **The ONLY ce-dimension verb that deliberately RE-MEASURES.** Works on a perimeter at any index and on a linear at 0/1. Cannot refuse for a shape reason, so a drag preview may always be drawn. |
| Insert a vertex into a perimeter | `insert_dimension_vertex(&mut self, dimension, after: usize, at: Point) -> Result<VertexOutcome, EditError>` | 22339 | `after == len-1` splits the **closing** segment of a closed shape, or extends an open path. Refused on a linear ce dimension (structurally two points). |
| Remove a vertex from a perimeter | `remove_dimension_vertex(&mut self, dimension, index: usize) -> Result<VertexOutcome, EditError>` | 22367 | Refuses below **2** vertices (open) or **3** (closed) — pdfce policy, not a spec rule. |
| Preflight a vertex edit | `vertex_edit_preview(&self, dimension, edit: VertexEdit) -> Result<VertexOutcome, EditError>` | 22399 | ✅ **Load-bearing** — shares one body with the three verbs above. `.err()` **is** the refusal predicate; there is deliberately no second one. |

### 1.23 Fonts (6)

| I want to… | Call | Line | Returns |
|---|---|---|---|
| Preview an unembed | `unembed_preview(&self, &UnembedRequest) -> UnembedPlan` | 16278 | Pure. `bytes_reclaimable` is **vacuous under incremental save**. |
| Ask whether unembedding is refused | `unembed_refusal(&self) -> Option<EditError>` | 16303 | ✅ **Load-bearing** — the verb calls it. |
| Remove embedded font programs | `unembed_fonts(&mut self, &UnembedRequest) -> Result<UnembedPlan, EditError>` | 16373 | ONE undo entry however many fonts. ⚠️ **Bytes are reclaimed by a FULL REWRITE, not by this call.** |
| Preview an embed | `embed_preview(&self, &EmbedRequest) -> EmbedPlan` | 16530 | Pure. |
| Ask whether embedding is refused | `embed_refusal(&self) -> Option<EditError>` | 16553 | ✅ Load-bearing. |
| Add missing font programs | `embed_fonts(&mut self, &EmbedRequest) -> Result<EmbedPlan, EditError>` | 16625 | ONE undo entry. **The file gets bigger, and the save mode does not change that.** |

### 1.24 Images (1 session verb + 3 pure previews on `NewImage`)

| I want to… | Call | Line | Returns |
|---|---|---|---|
| Place a raster image | `add_image(&mut self, spec: &NewImage<'_>) -> Result<ImageAuthorOutcome, EditError>` | 17437 | `{ image_id, soft_mask_id, content_id, resource_name, placed_rect, disclosures }`. Image XObject + optional `/SMask` + `q…cm…Do…Q` overlay stream + page patches, ONE undo entry. Additive — originals stay byte-verbatim. |

#### ★ The pure preview trio on `NewImage` — call these, do not re-derive them

`&self`, no session, no side effects. The same relationship
`preview_group_scale` (§1.22) has to `set_group_scale`: **a front end drawing
a preview must show the number the edit will produce.**

| I want to preview… | Call | Returns |
|---|---|---|
| where the picture lands | `placed_rect(&self) -> Rect` | `rect` under `Stretch`; the largest centred same-aspect sub-rectangle under `Contain`. |
| the resolution it implies | `effective_dpi(&self) -> (f64, f64)` | Image pixels per **inch of page**, per axis. `0.0` on a degenerate axis — never `inf`. |
| whether that is soft | `below_screen_resolution(&self) -> bool` | `true` below 72 dpi on either axis. |

★★ **`add_image` computes `ImageAuthorDisclosures::effective_dpi` and
`below_screen_resolution` BY CALLING these**, and that — not the arithmetic —
is the property that matters. The `pdfceGUI` session asked for the pair on
2026-08-19 and was explicit: *"If that is awkward for how the outcome is
assembled, I would rather have nothing than have two implementations."*

A preview a shell trusts and an outcome that disagrees with it is the defect
of §1.6 trap (b) — a pane reading `77.5°` while the `/AP` read `77.47 pt`,
from two independent derivations of one display value. Pinned by
`the_pure_dpi_preview_equals_what_the_outcome_reports`, which fails if the
two are ever separated again.

**Measure the PLACED rectangle, not the requested one.** Under `Contain`
they differ, and the resolution an operator gets is the one over the area the
picture actually covers. `effective_dpi` already does this; a shell
re-deriving from `spec.rect` would report a lower number than the truth.

**Not a refusal.** `below_screen_resolution` is a stated boundary with a
number beside it. A 12 dpi placement is a legitimate deliberate act; the
requester asked for the number *before* commit, not for something that stops
them.

### 1.25 Outcome structs — field reference

Grep target for "what does this return actually contain".

| Struct | Line | Public fields |
|---|---|---|
| `FieldAuthorOutcome` | 990 | `field_id: ObjId`, `merged: bool`, `disclosures: FieldAuthorDisclosures` |
| `DeleteOutcome` | 5594 | `pages_removed`, `objects_freed`, `dangling: DanglingReport`, `separations: SeparationImpact`, `signature: SignatureImpact` |
| `ResetPreviewRow` | 5677 | `field: String`, `current: String`, `target: String`, `would_remove: bool`, `would_change: bool`, `ineligible: Option<ResetIneligible>` |
| `ResetOutcome` | 5707 | `fields_reset`, `values_defaulted`, `values_removed`, `widgets_updated`, `skipped_pushbuttons`, `skipped_signatures`, `skipped_read_only` |
| `FillOutcome` | 5756 | `field_id`, `widgets_updated`, `applied_autosize: Option<f64>`, `unencodable_chars`, **`xfa_may_disagree: bool`**, `top_index: Option<i64>` |
| `RegenOutcome` | 5809 | `regenerated`, `need_appearances_cleared`, `applied_autosize`, `unencodable_chars` |
| `ImportOutcome` | 5824 | `applied`, `skipped` |
| `WidgetMove` | 5847 | `from: Rect`, `to: Rect`, `siblings_left_behind: usize` |
| `AnnotationDeletion` | 5936 | `subtype: String`, `route: AnnotationDeletionRoute`, `popup_removed`, `parent_popup_cleared`, `replies_orphaned`, `group_members_promoted`, **`appearance_streams_removed`** |
| `FieldDeletion` | 6052 | `widgets_removed`, `field_removed`, `selection_cleared`, `emptied_parents` |
| `TextMatch` | 6080 | `page_index`, `quad: Quad`, `text: String` |
| `FieldGroupDeletion` | 6704 | `group_name`, `terminals: Vec<String>`, `widgets_removed`, `nodes_removed`, `nodes: Vec<String>` |
| `FieldRename` | 6775 | `from`, `to`, **`descendants_renamed: usize`** |
| `FlattenOutcome` | 6792 | `fields_flattened`, `widgets_burned`, `pages_touched` |
| `ImageAuthorOutcome` | 17172 | `image_id`, `soft_mask_id: Option<ObjId>`, `content_id`, `resource_name: Vec<u8>`, `placed_rect: Rect`, `disclosures` |
| `SaveReport` | `writer/save.rs:208` | `bytes_written`, `bytes_appended`, `objects_written`, `objects_verbatim`, `objects_reserialized`, `byte_identical`, `delinearized`, `promoted: Vec<ObjId>`, `objects_deleted` |

---

## 2. Construction, and the session's three read views

```rust
use pdfce_core::document::Document;
use pdfce_core::edit::EditSession;

let doc = Document::from_bytes(bytes)?;          // part 1
let mut session = EditSession::new(doc);         // edit.rs:3368 — BY VALUE
```

`new` takes the `Document` by value on purpose (`edit.rs:3363-3366`): *"the
session **is** the open document from this point on, and a second handle to the
same `Document` would be a second, stale view of it."* Recover it with
`into_document` — which **discards** unsaved edits; it is not a commit.

`new` clones the trailer and caches `doc.next_object_number()`
(`edit.rs:3369-3370`). It cannot fail.

### 2.1 The canonical lifecycle, verbatim from the tests

```
bytes → Document::from_bytes(bytes)
      → EditSession::new(doc)                       // takes Document BY VALUE
      → <mutating verb>                             // each is one undo entry
      → session.to_incremental_bytes(&SaveOptions)  // -> (Vec<u8>, SaveReport)
        or session.to_full_bytes(&SaveOptions)
      → std::fs::write(path, &bytes)                // the SHELL writes the file
```

The doctest on `EditSession` itself, `edit.rs:3304-3320` — this is the Pass's
headline contract:

```rust
let bytes: Vec<u8> = include_bytes!("../../../fixtures/synthetic/hello.pdf").to_vec();
let mut session = EditSession::new(Document::from_bytes(bytes.clone())?);

session.set_page_rotation(0, 90)?;
assert!(session.is_modified());

session.undo();
assert!(!session.is_modified());

let (out, report) = save_incremental(
    session.document(),
    &session.dirty_set(),
    &SaveOptions::identity(),
)?;
assert_eq!(out, bytes);
assert!(report.byte_identical);
```

The integration harness every test file re-declares
(`crates/pdfce-core/tests/edit_undo.rs:229-239`):

```rust
fn session(bytes: &[u8]) -> EditSession {
    EditSession::new(Document::from_bytes(bytes.to_vec()).unwrap())
}
fn save(session: &EditSession) -> Vec<u8> {
    session.to_incremental_bytes(&SaveOptions::identity()).unwrap().0
}
```

Edit → save → reload → verify (`edit_undo.rs:410-422`):

```rust
let base = classic_pdf(true, true);
let mut s = session(&base);
s.set_page_rotation(1, 90).unwrap();
let out = save(&s);

let back = Document::from_bytes(out.clone()).unwrap();
assert_eq!(page_rotation(&back, 1), 90, "the edit must be in the file");
assert_only_the_named_objects_changed(&base, &out, &[ObjId::new(4, 0)]);
```

**Atomic write is the shell's job.** `pdfce-gui` writes to a temp file in the
destination directory and renames (`crates/pdfce-gui/src/main.rs:5082-5112`);
nothing in `pdfce-core` does this for you.

### 2.2 There is no round trip back to `Document` — and you do not need one

★ **`into_document` (`edit.rs:3399`) has ZERO callers in the entire repository.**
`grep -rn "into_document" --include=*.rs crates/ tools/` returns exactly two
lines — the definition (`edit.rs:3399`) and one doc cross-reference
(`edit.rs:3366`). The only other mentions are prose in
`SESSION_LOG.md` / `ROADMAP.md` classifying it as
*"plumbing… not capabilities, nothing to reach"*. It returns `self.base` — it
**discards unsaved edits** and is not a commit path. Do not reach for it
expecting "finish editing and hand back the document."

`pdfce-gui` holds one `EditSession` for the life of the open document
(constructed at `crates/pdfce-gui/src/main.rs:4855`, `:4928`; owned by `OpenDoc`)
and never converts back. Three read views (§2.3) make conversion unnecessary.

The two idioms that genuinely produce a `Document` again:

**(a) The reopen loop** — save, then build a fresh session from the output.
This is the ordinary operator loop, and `edit_undo.rs:874-894` pins that
successive saves chain `/Prev` correctly into three revisions:

```rust
let mut s  = session(&base);  s.set_page_rotation(0, 90).unwrap();  let once  = save(&s);
let mut s2 = session(&once);  s2.set_page_rotation(1, 180).unwrap(); let twice = save(&s2);
assert!(twice.starts_with(&once));
assert_eq!(String::from_utf8_lossy(&twice).matches("%%EOF").count(), 3);
```

**(b) The "materialise" idiom** — `to_full_bytes` → `Document::from_bytes` →
further processing. Needed when a core API takes `&Document` but the operator's
edits are still in the overlay. The **only** occurrence in the repo is
redaction-apply (`crates/pdfce-gui/src/redact_apply.rs:279-296`): unsaved
`/Redact` marks are invisible to `redact::apply_redactions(&Document)`, so the
session is materialised first. Note it uses `to_full_bytes` — a materialise
step under incremental save would carry the prior revision forward into the
"redacted" output.

### 2.3 Which view to use where — a decision table

| The consumer needs | Use | Why |
|---|---|---|
| The file exactly as loaded (writer span lookups, "revert" comparisons) | `document()` `:3393` | The base revision. |
| One object's current value | `value(id)` `:3409` | Overlay, then base, `None` if deleted. |
| To walk the edited graph (page tree, forms, annotations) | `graph()` `:3429` | `SessionGraph` implements `ObjectGraph`. |
| **To render, hit-test, or decompose vectors** | **`view()` `:3469`** | Graph **+ stream bytes**, zero-allocation span resolution. Used by the renderer at `crates/pdfce-render/tests/preview_equals_saved.rs:414`. |
| A single flat buffer for a once-per-operation `pageops` call | `authored_source()` `:3561` | Returns `Cow`; owned ⇒ full memcpy. |

**T-01 — the defect that shipped for 13 Passes.** From `edit.rs:3438-3444`:
*"from Pass 3.1 to Pass 16.2 the GUI rasterized `EditSession::document` — the
BASE revision — so every edit the operator made was authored correctly and
displayed not at all."* Recorded in
`docs/decisions/018-edited-state-is-what-the-canvas-renders.md`. **A new shell
must pass `&session.view()` wherever a `&Document` looks like it would fit.**

`view()`'s return is read-only by contract (`edit.rs:3462-3467`): *"The returned
view must never reach the writer… Saving goes through `EditSession::dirty_set`,
which hands the writer the staging buffer under its own contract."*

---

## 3. The command / undo model, as a contract

### 3.1 What a command is

```rust
struct Command {                                          // edit.rs:691
    kind: CommandKind,                                    // :692
    objects: Vec<ObjectWrite>,                            // :693
    removals: Vec<Removal>,                               // :703
    trailer: Option<(Dict, Dict)>,                        // :709
}
struct ObjectWrite { id: ObjId, before: Option<Object>, after: Option<Object> }  // :715
struct Removal    { id: ObjId, was_deleted: bool, is_deleted: bool }             // :2288
```

`Command` and `ObjectWrite` are **private**. The only thing a consumer sees is
`CommandKind` (`edit.rs:238`, `pub`, `Copy`, `#[non_exhaustive]`), returned by
`undo_kind` / `redo_kind` / `undo` / `redo`.

`before`/`after` are `Option<Object>` because *absent from the overlay* is a
real, distinct value: it means read through to the base, and for a created
object it means the object does not exist (`edit.rs:685-689`).

**There is no `begin_command` / `push_command` / two-phase transaction API.**
(`NOT FOUND — searched `begin_command`, `push_command` across `crates/`.) Each
verb assembles a whole `Command` value in a local `Vec<ObjectWrite>` off to the
side and hands it to one infallible call:

```rust
fn commit(&mut self, command: Command) { … }              // edit.rs:5305, returns ()
```

`commit` applies every write (`:5306`), every removal (`:5309`), the trailer
swap (`:5312`), **clears the redo stack** (`:5315`), pushes onto `undo`
(`:5316`), and drops the oldest entry past `MAX_UNDO_DEPTH` (`:5318-5320`).

**`commit` cannot fail. That is the structural reason a verb is atomic:** it
either never reaches `commit` (nothing changed) or completes it (everything
changed).

### 3.2 Undo granularity — what makes ONE entry

`CommandKind` has **46 variants** (`edit.rs:238-641`) and each one's doc comment
states its granularity explicitly. The rule, stated once: **one operator gesture is one entry**, and
every object the gesture must touch to leave a valid document goes in that entry.

Worked examples from the source:

- **Field creation** (`CommandKind::AddFormField`, `edit.rs:295`): field dict +
  widget + `/AP` + page `/Annots` + `/AcroForm /Fields`, together — *"a field
  registered but not annotated, or annotated but not registered, is a document
  no undo can repair."*
- **Page reorder** (`ReorderPages`, `edit.rs:255`): one entry however many pages
  moved. §11.3's snapshot fallback, on the same stack — **do not build a second
  undo system for bulk edits.**
- **Group-wide ce-dimension regeneration** (`SetGroupScale`, `SetGroupStandard`,
  `SetGroupStyle`): one entry however many members regenerated.
- **`unembed_fonts` / `embed_fonts`** (`edit.rs:16346`, `:16585`): one entry
  however many fonts — *"a partial undo of it — three fonts restored, four not —
  is a document state the operator never asked for."*

### 3.3 Labelling the Undo control

`undo_kind()` returns `Option<CommandKind>`; `CommandKind` is `Copy` and
**deliberately carries no `String`**. Several variants carry a `usize` count so
a label can state a magnitude (`RotatePages{count, delta}`,
`RegenerateAppearances{count}`, `FlattenFields{count}`,
`UnembedFonts{count}`, `SetGroupStyle{members}`, `SetDimensionStyle{overrides}`,
`ReflowBlock{lines_before, lines_after}`). `edit.rs:290-294`: *"If a label ever
needs the name, the right move is an interned id, not `String`."*

Distinctions the enum preserves on purpose, which a label must not flatten:

| These are different sentences | Kinds |
|---|---|
| "delete a drawing view" vs "delete one line of it" vs "delete one point" | `DeleteObject` / `DeleteSubpath` / `DeleteNode` |
| "delete this label" vs "delete all 237 labels" | `DeleteTextRun` / `DeleteObject` |
| "move a point" vs "change a curve's shape" | `MoveNode` / `MoveHandle` |
| "I changed my mind about redacting" vs "delete annotation" | `DeleteRedactionMark` / `DeleteAnnotation` |

### 3.4 The three granularity exceptions a GUI must special-case

| Verb | Granularity | Line |
|---|---|---|
| `import_form_data` | **N entries** — one per field. Ctrl+Z after an FDF import undoes one field. | `edit.rs:13458-13460` |
| `set_info_field` | **0 entries on a no-op.** Setting a field to its existing value, or clearing an absent one, records nothing and leaves the redo stack alone. | `edit.rs:3677-3682` |
| `set_dimension_display` | **1 entry even on a no-op** — deliberately the inverse, so a toggle control's undo behaviour is not sometimes-present. | `edit.rs:15901-15908` |

A shell that keeps its own dirty counter incremented per successful call will
drift from `undo_depth()` on the first and second of these.

### 3.5 History bounding, and why it is free

`MAX_UNDO_DEPTH = 256` (`edit.rs:166`). Dropping the oldest command is safe
**only** because the dirty set is a diff rather than a replay. `edit.rs:65-69`:
*"Under a replay design, dropping a command would corrupt what gets saved; here
it costs exactly what it appears to cost — the operator can no longer step back
past that point."*

### 3.6 Redo invalidation

`commit` clears `redo` unconditionally (`edit.rs:5315`). Standard editor
behaviour; stated because a shell that caches "can redo" must re-query after
every mutation.

---

## 4. The dirty set — what a save is computed against

**This is the section that prevents the expensive bug.**

`ARCHITECTURE.md` §11.1, quoted in `edit.rs:44-50`:

> the "dirty set" … is computed as a **structural diff against the base
> revision at save time** — it is *not* the union of every object any command
> ever touched during the session. If a user edits an object and then undoes
> that specific edit before saving, that object must **not** appear in the
> incremental update.

`EditSession::dirty_set` (`edit.rs:3497`) does exactly four things:

1. For every `(id, value)` in `state`: **skip** if `base.get(id).value == value`
   — net-zero against the base is *not dirty*. Else `dirty.replace(id, …)`.
   (`edit.rs:3499-3506`.)
2. For every id in `deleted`: emit a free entry **only if the base defined it**
   — an id the base never had cannot be deleted into it. (`edit.rs:3511-3515`.)
3. For every trailer key that differs from the base's: `patch_trailer`.
   (`edit.rs:3516-3520`.)
4. If `staging` is non-empty: hand it to the `DirtySet` (R45), so an authored
   appearance stream's span — which points past the base — resolves.
   (`edit.rs:3527-3529`.)

Three properties fall out (`edit.rs:63-75`), each of which would need defensive
code under a history-replay design:

- **Bounding undo is free** (§3.5).
- **Coalescing is free.** N edits to one object leave one `state` entry, so the
  update section carries it once, not N times.
- **A "modified?" indicator cannot lie.** `is_modified()` asks the writer's own
  question.

**What a GUI must therefore NOT do:** derive "unsaved changes", or the set of
objects to write, from the undo stack. `can_undo() == true` after an
edit-then-undo, while `is_modified() == false` and the save is byte-identical.
Both are correct; they answer different questions.

`DirtySet` itself lives at `crates/pdfce-core/src/writer/mod.rs:215` with all
fields private. Relevant public methods: `empty()` `:271`, `is_empty()` `:392`
(note: **staging is not consulted**), `len()` `:401`, `changes_content()` `:414`
(the §14.4 `/ID[1]` regeneration trigger), `trailer_patch()` `:441`,
`staging()` `:471`, `combined_source()` `:488`.

Executably pinned (`ARCHITECTURE.md` §11.5): **edit → undo → save is
byte-identical across 2,897/2,897 corpus files (100%)**, plus fixture tests
including a 12-command history and undo → redo → save. The corpus gate lives in
`tools/roundtrip/src/main.rs:665-793` (`fn check_mutation`, *"Check 3: THE
contract"*, `:765-772`); the fixture tests are
`crates/pdfce-core/tests/edit_undo.rs:308` (single edit), `:350` (12-command
history), `:374` (undo → redo → save), `:923`
(`a_save_does_not_consume_the_undo_history`).

★ **A save does not consume the undo history** (`edit_undo.rs:923`). After
saving, `can_undo()` is still true and `is_modified()` is still false. A shell
that wires "Save" to "clear undo" is inventing a restriction the core does not
impose.

The same contract is exposed to operators as `pdfce-cli --verify-undo`
(`crates/pdfce-cli/src/main.rs:11192-11202`): undo everything, save, byte-compare
against the source. A new shell can offer the same self-check cheaply.

---

## 5. The save path

### 5.1 The two methods

```rust
pub fn to_incremental_bytes(&self, options: &SaveOptions)
    -> Result<(Vec<u8>, SaveReport), WriteError>          // edit.rs:4053
    // -> writer::save_incremental(&self.base, &self.dirty_set(), options)

pub fn to_full_bytes(&self, options: &SaveOptions)
    -> Result<(Vec<u8>, SaveReport), WriteError>          // edit.rs:4071
    // -> writer::save_full(&self.base, &self.dirty_set(), options)
```

Both take `&self`. **Saving does not clear the undo stack, does not reset
`is_modified()`, and does not write to disk.** A shell that wants
"saved ⇒ unmodified" must track that itself, or re-open from the saved bytes.

The mode is expressed by **which method you call** — there is no mode parameter.
`SaveMode` (`crates/pdfce-core/src/signature.rs:249`, variants `Incremental` /
`FullRewrite`) exists **only** for the signature-impact query.

★ **The two modes are given different `SaveOptions` in the shipped CLI, and the
asymmetry is deliberate.** `crates/pdfce-cli/src/main.rs:11177-11180`:

```rust
let saved = match mode {
    SaveMode::Incremental => session.to_incremental_bytes(&eol(SaveOptions::identity())),
    SaveMode::Full        => session.to_full_bytes(&options),
};
```

Incremental gets `SaveOptions::identity()` — **no producer stamp** (R41's
no-fingerprint rule); full rewrite gets the options carrying `ProducerPolicy`.
An append that quietly wrote pdfce's identity into a file the operator only
annotated would be a fingerprint they did not ask for. A new shell should keep
this split rather than pass one `SaveOptions` to both.

### 5.2 What each mode guarantees

| | `to_incremental_bytes` (default) | `to_full_bytes` |
|---|---|---|
| Mechanism | §7.5.6 append; prior bytes untouched | one-revision rewrite |
| Untouched objects | byte-verbatim, not re-serialized | byte-verbatim where possible |
| Superseded objects | **remain in the file, in the prior revision** | dropped (with one exception, below) |
| Existing signatures | byte range preserved iff no structural change | **all destroyed** (§12.8.1) |
| Linearization | invalidated, **reported** via `SaveReport::delinearized`, never repaired | same |
| `/ID[1]` | regenerated iff `dirty.changes_content()` | same rule |
| Empty dirty set | **byte-identical to the input**; `SaveReport::byte_identical == true` | not byte-identical |
| Refused when | base was loaded via xref recovery (`WriteError::RecoveredBaseForbidsIncremental`, `writer/save.rs:309`) | hybrid-reference input (`WriteError::HybridFullRewrite`, `save.rs:580`) |

Both refuse an encrypted document (`WriteError::EncryptedSaveUnsupported`,
`save.rs:318` / `:589`).

### 5.3 ★ Why an absence assertion over incrementally-saved bytes is vacuous

Incremental save **structurally preserves** superseded content — this is
required by §7.5.6, not a pdfce shortcoming. The old bytes of every replaced
object stay in the file by construction (`edit.rs:4042-4047`).

Therefore:

> **A test, an audit, or a UI claim of the form "X is no longer in the file",
> checked against the output of `to_incremental_bytes`, proves nothing.** The
> prior revision still holds X. The assertion is not merely weak — it is
> *always* satisfiable by the wrong file and *never* falsifiable by the right
> one.

The same applies to size claims: `UnembedPlan::bytes_reclaimable` is a
full-rewrite figure. `edit.rs:16337-16345`: *"An incremental save (the default)
appends an update section, so the freed objects' bytes stay in the prior
revision and the file gets **larger**… Every shell that reports the byte figure
must report the mode that delivers it."*

### 5.4 When a consumer MUST choose full rewrite

Three families, `ARCHITECTURE.md` §5.2 / §5.9 / §5.10:

| Rule | Trigger | Enforced where |
|---|---|---|
| **R35** | Redaction *apply* | **Structurally.** `redact.rs:95` imports `save_full` only; `redact::apply_redactions` (`redact.rs:1078`) has **no mode parameter** and calls `save_full` at `redact.rs:1224`. The caller cannot ask for incremental. |
| **R67** | Base document was loaded via cross-reference recovery | **In the writer.** `writer/save.rs:309-311`: `if doc.loaded_via_recovery() { return Err(WriteError::RecoveredBaseForbidsIncremental) }` — checked *first*, even for an empty dirty set. |
| **R58** | Every *other* removal/scrub operation | ★ **NOT ENFORCED IN CODE.** |

★ **The R58 gap is the single most important thing in this section for a new
shell.** `force_full` / `forces_full` / `requires_full` / `RequiresFullRewrite`
as identifiers: **NOT FOUND anywhere in `crates/**/*.rs`**. `R58` appears in
exactly two comments (`writer/mod.rs:757`, `writer/save.rs:308`). Nothing in
`EditSession` or the writer refuses an incremental save after a delete.

So these verbs will happily save incrementally, leaving the removed content
recoverable, and **only the shell can prevent it**:

| Verb | What its own doc says |
|---|---|
| `delete_pages` / `delete_pages_with` `:14644`/`:14663` | *"It is **not redaction**. Under the default incremental save the removed page's bytes remain in the file by construction… Front ends must say so."* (`edit.rs:14603-14608`) |
| `flatten_fields` `:13730` | *"under the default **incremental** save the prior revision still holds them … the R35 sibling. This is the R48 destructive-disclosure the caller must surface."* (`edit.rs:13710-13716`) |
| `detach_file` `:10347` | *"This frees the objects; it does not rewrite history… This is NOT a redaction verb and must not be described as one."* (`edit.rs:10327-10338`) |
| `delete_annotation` `:10847` | *"**Deleting a comment is not redacting it** … A full rewrite drops the bytes; the default save mode does not."* (`edit.rs:10771-10778`) |
| `unembed_fonts` `:16373` | *"Bytes are reclaimed by a FULL REWRITE, not by this call."* (`edit.rs:16337`) |

**Reference behaviour in the shipped shells:** `pdfce-cli` exposes
`--full-rewrite` and, when it is absent after a flatten, prints to stderr
*"flatten saved incrementally — the pre-flatten field values remain recoverable
in the prior revision"* (`crates/pdfce-cli/src/main.rs:10344-10356`).
`pdfce-gui`'s save dialog **always** calls `to_incremental_bytes`
(`crates/pdfce-gui/src/main.rs:5095-5097`); its only full-rewrite path is
redaction-apply (`crates/pdfce-gui/src/redact_apply.rs:279-284`, module doc:
*"There is deliberately no `to_incremental_bytes` call anywhere in…"*).

A new shell may reasonably do better than that — a "remove permanently"
affordance that selects `to_full_bytes` — but it must **choose**, because
nothing below it will.

### 5.5 One thing a full rewrite still does not remove

`edit.rs:4062-4065`: a full rewrite *"does **not** by itself remove the
superseded value of a compressed object that an edit promoted out of its object
stream."* The object stream carries through verbatim in **both** modes;
`SaveReport::promoted` names the affected ids. Redaction handles this by
decomposing containers (`redact.rs:1666`); nothing else does.

### 5.6 Signature impact — ask immediately before saving

```rust
let impact = session.signature_impact_of_save(SaveMode::Incremental);  // edit.rs:6992
```

| census | mode | structural change? | → |
|---|---|---|---|
| no signatures | either | — | `SignatureImpact::None` |
| any | `FullRewrite` | — | `Invalidated` |
| any | `Incremental` | no | `ByteRangePreserved` |
| any | `Incremental` | yes | `Invalidated` |

(`signature.rs:575-593`.) `changes_structure()` is computed from the page tree,
not from history, for the same reason `dirty_set` is (`edit.rs:7003-7008`) — and
it is the **only** caller of that method in the whole workspace.

⚠️ `edit.rs:6987-6989`: *"`SignatureImpact::ByteRangePreserved` must never be
rendered on its own as 'the signature is still valid'."* It means stage 1 (the
byte-range digest) still verifies, and *only* that.

Ask it **at save time, not at edit time** (`edit.rs:6982-6986`): the dirty set is
computed at save time, so "does this save change structure?" is not knowable
when the edit is made.

### 5.7 `SaveReport` — read it, do not synthesise it

`crates/pdfce-core/src/writer/save.rs:208`, `#[non_exhaustive]`. Field notes
worth carrying:

- `byte_identical` — *"Only ever true for an empty-dirty-set `save_incremental`."*
- `bytes_appended` — `0` for an empty-dirty-set incremental; equals
  `bytes_written` for a full rewrite.
- `promoted: Vec<ObjId>` — objects lifted out of object streams, **named** not
  just counted (decision 007 W3). *"there is deliberately no separate counter
  field"* — use `promoted.len()`.
- `objects_deleted` — counted, deliberately **not** named.
- `delinearized` — set from `Linearization::save_invalidates_fast_web_view()`,
  which is true only for a **Live** linearization; a `Stale` one returns false
  because a previous save already spent the property
  (`linearization.rs:110`, `:274-286`).

There is **no session-level save outcome type**. `SaveOutcome` exists but is
private to `pdfce-gui` (`crates/pdfce-gui/src/main.rs:1411`). `WriteReport`:
**NOT FOUND** anywhere in the workspace. A new shell will want its own.

---

## 6. The guard / refusal model

### 6.1 The four guards

| Guard | Error variant | Check | Meaning |
|---|---|---|---|
| **Encryption** | `EditError::DocumentEncrypted` `edit.rs:2765` | inlined `if self.base.trailer().contains_key(b"Encrypt")` — **no named helper** (`NOT FOUND — searched `refuse_if_encrypted`, `is_encrypted`, `encryption_refusal` across `crates/`); 38 occurrences in `edit.rs` | Today pdfce refuses to *load* an encrypted file at all, so this is a forward-compatible R37 seam (`edit.rs:19338`), not a path a loadable file currently reaches. |
| **Enforced certification** | `EditError::CertificationForbidsChange { permission: u8 }` `edit.rs:2722` | three functions, §6.2 | The catalog carries `/Perms → /DocMDP` **and** at least one signature exists (`signature.rs:332`). |
| **Sidecar version** | `EditError::SidecarWrittenByNewerBuild { found, supported }` `edit.rs:2415` | `check_dimension_sidecar` `edit.rs:16917` | The ce-dimension `/PieceInfo` sidecar declares a version above `SIDECAR_VERSION` (**currently 2**, `dimension/sidecar.rs:44`). No sidecar ⇒ `Ok`. |
| **`/Size` suppression** | `EditError::ObjectCreationWouldExposeHiddenObjects { count }` `edit.rs:2355` | `self.base.suppressed_object_count() > 0` | §7.5.5: objects at or above `/Size` *"shall be ignored and defined to be missing"*. Creating an object raises `/Size` and would resurrect objects nobody touched. **Only creation is refused; editing an existing object is unaffected.** |

Plus the allocator's `EditError::ObjectNumbersExhausted` (`edit.rs:2336`).

### 6.2 Certification is THREE gates, not one — and they disagree on purpose

```rust
fn check_certification(&self)                -> Result<SignatureCensus, EditError>  // :6970  STRICT
fn check_certification_for_annotation(&self) -> Result<(), EditError>               // :11449 permits /P 3
fn check_certification_for_fill(&self)       -> Result<(), EditError>               // :12289 refuses only /P 1
```

- **Strict** (`:6973`): refuses whenever `forbids_structural_change()`. `/P` is
  carried into the message only.
- **Annotation-aware** (`:11454`): refuses only when `permission < 3`.
- **Fill-aware** (`:12293`): refuses only `permission == Some(1)`; separately
  refuses any `/FieldMDP` with `EditError::FieldLockedBySignature`.

Table 254's default `/P = 2` is applied via `unwrap_or(2)` (`:6974`, `:11453`).
A DocMDP transform in a signature's `/Reference` alone is **detection only** —
the edit proceeds and the impact is reported. Only the catalog's `/Perms`
upgrades it to **prevention** (`edit.rs:6951-6957`).

**Which gate a verb takes:**

| Gate | Verbs |
|---|---|
| **Strict** `check_certification` | all 11 vector verbs (via `vector_surgery` `:5116`); all 5 field-creation verbs (via `field_authoring_preflight` `:7406`); `delete_field`, `delete_widget`, `move_widget` (via `deletion_preflight` `:9101`); `delete_field_group`, `field_group_deletion_preview` (`:8668`); `rename_field` `:8893`; `flatten_fields` `:13737`; `delete_pages_with` `:14668`; `reorder_pages` `:14945`; `rotate_pages` `:15064`; all 11 ce-dimension verbs; `unembed_refusal` `:16307`; `embed_refusal` `:16557`; `add_image` `:17450`; `deletion_refusal`/`rename_refusal` (via `structural_form_refusal` `:12286`) |
| **Annotation** | `add_markup` `:10008`; `attach_file` `:10156`; `detach_file` `:10351`; `add_redaction` `:10500`; `delete_redaction_mark` `:10632`; `delete_annotation`+`annotation_deletion_preview` (via `annotation_deletion_guards` `:11146`); `annotation_deletion_refusal` `:11496`; the three `mark_redactions_*` (via `author_text_matches` `:11677`); `add_text_annotation` `:12048` |
| **Fill** | `fill_text_field`, `fill_text_field_downgrading_rich_text`, `reset_form`, `set_choice_value`, `regenerate_appearances` (via `fill_guards` `:12308`); `set_button_state` `:12576`; `fill_refusal` `:12201` |
| **None** | `set_info_field` `:3689` (deliberate — argued at `edit.rs:6957-6968` as an owed decision, not an oversight); `set_page_rotation` `:3848`; `rotate_page_by` `:3981`; `delete_pages` `:14644` (encryption-ungated too) |

★ **The singular/plural page verbs diverge.** `rotate_pages` (plural, `:15063`)
takes the strict certification gate; `set_page_rotation` / `rotate_page_by`
(singular) take **no guard at all**. A shell that offers both must not assume
they refuse alike.

### 6.3 Refusals happen BEFORE any mutation — with one named exception

Project rule 4 in mechanical form: **a failed call leaves nothing half-written
and needs no rollback.** The structural reason is §3.1 — `commit` is infallible
and is the last thing a verb does.

Verified orderings (guard line < first mutation line < commit line):

| Verb | Encrypt | Cert | `/Size` | Sidecar | first `alloc_number` | `commit` |
|---|---|---|---|---|---|---|
| `add_markup` `:9986` | 9991 | 10008 | 10027 | — | 10034 | 10082 |
| `add_image` `:17437` | 17448 | 17450 | 17453 | — | 17472 | 17580 |
| `add_dimension` `:15380` | 15387 | 15389 | 15402 | 15405 | 15417 | 15505 |
| `flatten_fields` `:13730` | 13732 | 13737 | 13740 | — | 13816 | 13940 |

★ **The one genuine counterexample — `add_radio_button` (`edit.rs:8253`).** On
the merge-into-existing-group branch:

```
edit.rs:8383      self.commit(Command { kind: CommandKind::AddFormField, objects, … });
edit.rs:8389      if spec.selected {
edit.rs:8390          self.set_button_state(&spec.name, &spec.export_value)?;   // can Err AFTER the commit
```

`set_button_state` runs its own encryption guard (`:12574`) and fill
certification gate (`:12576`), either of which can return `Err` **after** the
merge has been applied. The session is then left holding a committed
`AddFormField` command and an unselected radio member. It is deliberate —
`edit.rs:8378-8382`: *"it is reached below, after the merge is committed, so it
sees the group the merge actually produced rather than a prediction of it
(R92)."* The stranded state is one `Command`, so a single `undo()` reverses it,
but **`add_radio_button` is the one verb whose `Err` return does not imply
"nothing happened."** A shell must either call `undo()` on that error path or
re-query the field state.

### 6.4 The six `*_refusal()` preflights — three are NOT load-bearing

All six are `&self`, `#[must_use]`, and return `Option<EditError>`.

| Accessor | Line | Body | Does the mutating verb re-check the same things? |
|---|---|---|---|
| `fill_refusal` | 12200 | `check_certification_for_fill().err()` | ❌ **STRICT SUBSET.** The verbs call `fill_guards` (`:12304`) = Encrypt + cert-for-fill + `/Size`. `fill_refusal` can say `None` on a document where the fill returns `DocumentEncrypted` or `ObjectCreationWouldExposeHiddenObjects`. `import_form_data` calls `fill_refusal()` (`:13489`) and then **immediately re-adds the missing encryption check** (`:13492-13494`) — the codebase knows. |
| `deletion_refusal` | 12239 | `structural_form_refusal()` = Encrypt + strict cert | ✅ Same checks, **independently open-coded** in `deletion_preflight` (`:9096-9101`). Advisory but currently exact. |
| `rename_refusal` | 12268 | `structural_form_refusal()` | ✅ Same, open-coded in `rename_field` (`:8890-8893`). |
| `annotation_deletion_refusal` | 11492 | Encrypt + annotation cert | ❌ **DOCUMENT-SCOPE ONLY.** Takes no `annot_id`, so it cannot run the three per-annotation refusals `annotation_deletion_guards` (`:11110`) runs first: `AnnotationLocked` (`:11120`), `AnnotationIsTrapNet` (`:11130`), `AnnotationIsWidget` (`:11141`). |
| `unembed_refusal` | 16303 | Encrypt + strict cert | ✅ **LOAD-BEARING** — `unembed_fonts` calls it at `:16375`. |
| `embed_refusal` | 16553 | Encrypt + strict cert | ✅ **LOAD-BEARING** — `embed_fonts` calls it at `:16627`. |

**GUI consequence.** Use these to *grey out or explain* controls, never as the
sole gate. Two of the six under-report: a control enabled because
`fill_refusal()` returned `None` can still fail, and a per-annotation Delete
enabled because `annotation_deletion_refusal()` returned `None` can still fail
on a locked, TrapNet, or widget annotation. The correct pattern is
**preflight for the label, handle the `Err` for the truth.**

The deliberate duplication of `deletion_refusal` and `rename_refusal` is argued
at `edit.rs:12253-12266`: *"both delegate to `structural_form_refusal`, so if a
future spec nuance ever separates them, the split happens HERE, once."*

### 6.5 Preflight-then-commit — the four preview pairs, and their tested contract

Distinct from the six `*_refusal()` accessors: these return a **description of
what would happen**, not a reason it would not. All four are tested to agree
with the verb they preview.

| Preview | Verb | Test pinning agreement |
|---|---|---|
| `reset_preview(&self, only)` `:12755` | `reset_form` `:12884` | `edit.rs:18252-18271` — `preview.iter().filter(\|r\| r.would_change).count() == out.fields_reset` |
| `annotation_deletion_preview(&self, id)` `:11316` | `delete_annotation` `:10847` | `tests/annot_deletion.rs:334-379` — *"preview said yes but the delete said {e}"*; plus `:385-399 previewing_changes_nothing` and `:403-420 the_preview_refuses_exactly_what_the_deletion_refuses` |
| `unembed_preview(&self, req)` `:16278` / `embed_preview` `:16530` | `unembed_fonts` `:16373` / `embed_fonts` `:16625` | `font_unembed.rs:1567-1580`, `font_embed_missing.rs:2260+` |
| `field_group_deletion_preview(&mut self, fqn)` `:8535` | `delete_field_group` `:8574` | `tests/form_field_hierarchy.rs:1166-1196` — `done.terminals == preview.terminals` |

Two of these previews are **called internally by the verb**, so the external
call is purely for showing the operator, not for correctness:
`unembed_fonts` calls `unembed_refusal()` then `unembed_preview(request)`
(`edit.rs:16375-16379`); `delete_field_group` calls
`field_group_deletion_preview` (`edit.rs:8573-8574`).

`annotation_deletion_preview` is explicitly designed to be safe to call every
frame while the pointer rests on a row (`annot_deletion.rs:385-399`).

### 6.6 Orderings that are required — and three that look required but are not

**NOT required — `add_dimension_group` before `add_dimension`.** An unknown
`GroupId` joins the always-present default group (`edit.rs:15372-15374`,
implementation `:15407-15410`, fallback `DEFAULT_GROUP_ID`). ~25 tests call
`add_dimension(0, DEFAULT_GROUP_ID, …)` with no prior group creation.
**UNVERIFIED — the unknown-`GroupId` fallback specifically is NOT TESTED**; no
test passes a bogus `GroupId` to `add_dimension` (searched
`DimensionGroupNotFound`, `GroupId(99`, across `crates/`).

**Required — a custom group must exist before `set_group_scale` /
`set_group_style` / `set_group_standard`**, which return
`EditError::DimensionGroupNotFound` (`edit.rs:15996`, `:16072`). The canonical
ordered fixture, `tests/dxf_scale.rs:313-331`: `new` → `add_dimension_group` ×2
→ `set_group_scale` → `add_dimension` ×2.

**Semantically ordered, not error-enforced:** calibrating a group's scale
*after* its members exist regenerates their labels
(`tests/dimension_roundtrip.rs:209-230`); calibrating *before* any member exists
labels nothing.

**NOT required — `find_text` before `mark_redactions_by_search`.** They are
siblings, not a sequence: the marking verb runs its own scan through the shared
`scan_text_matches` (`edit.rs:11679`). They also **disagree on query
semantics** (trap T-05), so feeding one's results to the other is wrong, not
merely redundant. `pdfce-gui` keeps `find_matches` separate from the Find bar
(`crates/pdfce-gui/src/main.rs:9812`, `:9932`) and never feeds them in.

**Required — mark → materialise → apply, for redaction.**
`prepare_redaction_apply` refuses with `NothingToApply` when no mark exists
(`crates/pdfce-gui/src/redact_apply.rs:275-277`), and reads the census from
`session.graph()` (the edited view) rather than `session.document()` — the
unsaved-mark trap, `redact_apply.rs:272-274`.

**Works immediately — author then fill.** A field pdfce just authored is
accepted by the ordinary fill path with no save/reopen
(`tests/form_field_authoring.rs:202-213`).

**Type-enforced — import then place, for images.** `image_import::import(&bytes)`
must produce an `ImportedImage` before `NewImage::new(page, rect, &img)` can
borrow it (`tests/image_placement.rs:238-247`).

### 6.7 The `EditError` taxonomy

`edit.rs:2300`, `#[derive(Debug, Clone, thiserror::Error)]`, `#[non_exhaustive]`.
**57 variants** (`edit.rs:2300-3136`, counted at depth 1). No inherent
`impl EditError` block and **no `is_*` classification helpers**
(`NOT FOUND — searched `impl EditError` in `edit.rs`); callers discriminate with
`matches!`. The five groups below partition all 57. `edit.rs:2295-2296`: *"Every variant names a
condition the operator (or the calling front end) can act on. There is
deliberately no catch-all 'edit failed'."*

Grouped for a shell's error presenter:

**Refusals a shell should render as policy, not failure**
`DocumentEncrypted` 2765 · `CertificationForbidsChange` 2722 ·
`SidecarWrittenByNewerBuild` 2415 · `ObjectCreationWouldExposeHiddenObjects` 2355 ·
`FieldLockedBySignature` 3044 · `AnnotationLocked` 2917 · `AnnotationIsTrapNet` 2945 ·
`AnnotationIsWidget` 2877 · `FieldAuthoringRefusedXfa` 2504 · `AttachmentTreeUnsupported` 2374

**Bad argument from the shell (a bug in the shell, not the document)**
`PageOutOfRange` 2303 · `RotationNotMultipleOf90` 2316 · `NotAPermutation` 3090 ·
`WidgetIndexOutOfRange` 2582 · `FieldNameEmpty` 2696 · `FieldRectDegenerate` 2517 ·
`ImageRectDegenerate` 3130 · `EmptyGeometry` 2771 · `NotALinearDimension` 2453 ·
`NotACircularDimension` 2468 · `InvalidTolerance` 2481 · `NotARedactionMark` 2832 ·
`ChoiceEditRequiresCombo` 2568 · `ChoiceRequiresMultiSelect` 3053 ·
`ChoiceValueNotInOptions` 3067 · `CheckBoxOnStateInvalid` 2654 ·
`ChoiceOptionDuplicate` 2665 · `RadioExportValueTaken` 2621 · `CombPreconditionUnmet` 2531 ·
`TooltipDecisionRequired` 2686 · `NotAGroupingNode` 2974 · `WouldRemoveEveryPage` 2735

**Not found**
`AttachmentNotFound` 2386 · `DimensionNotFound` 2395 · `DimensionGroupNotFound` 2700 ·
`FieldNotFound` 2954 · `AnnotationNotFound` 2850 · `NoInteractiveForm` 3078 ·
`NoFontsToUnembed` 2784 · `NoFontsToEmbed` 2804

**Document is malformed or unsupported**
`PageTree` 2311 (from `PageTreeError`) · `NotADictionary` 2327 ·
`ObjectNumbersExhausted` 2336 · `AnnotsNotAnArray` 2819 · `WidgetRectMissing` 2600 ·
`RadioGroupUsesPositionalOpt` 2644 · `FieldNotFillable` 3014 · `FieldIsRichText` 3007 ·
`FieldStateUnknown` 3023 · `TextExtraction` 3099 · `VectorEditNoContents` 3114

**Wrapped subsystem errors** (`#[from]` — a shell should unwrap and present the inner)
`FieldAuthoring(FormAuthorError)` 2560 · `SeparationSplit(SeparationSplitRefused)` 2748 ·
`VariableText(VarTextError)` 2812 · `FormData(FdfError)` 3081 ·
`VectorEdit(VectorEditError)` 3105 · `VectorEditContent(ContentError)` 3110

**Not in this taxonomy at all:** the five page-text verbs
(`edit_text`, `format_text`, `preview_style_resolution`, `reflow_block`,
`add_text`) return `crate::text_edit`'s own error types, and encryption there
surfaces as `text_edit::EditError::Encrypted`, **not** `EditError::DocumentEncrypted`
(guards at `edit.rs:4134`, `:4193`, `:4252`, `:4306`, `:4376`).

---

## 7. Object allocation and byte staging

Two pre-commit mutations exist, and both are bookkeeping rather than document
state.

```rust
fn alloc_number(&mut self) -> Result<u32, EditError>   // edit.rs:14451
fn stage_bytes(&mut self, content: &[u8]) -> ByteSpan  // edit.rs:14461
```

**Object numbers.** `alloc_number` hands out `next_number` and increments it.
`edit.rs:14448-14450`: *"The counter is **not** rewound on undo — §7.5.4/§7.5.7
never reuse a number, so a skipped one is harmless, and rewinding would risk a
collision with a redo."* One site open-codes the same logic
(`set_info_field`, `edit.rs:3759`).

**Consequence for a shell:** a verb that allocates a number and *then* hits a
fallible `?` leaks the number (and possibly staged bytes). This **does not dirty
the document** — `dirty_set` diffs `state`/`deleted`/`trailer`, and no `Command`
was pushed. Pinned by the test `a_refused_comb_stages_nothing` (`edit.rs:18171`).
Do not attempt to reclaim numbers.

**Staging (R45).** Authored appearance streams keep the span model rather than
owning bytes: `stage_bytes` appends to `session.staging` and returns a
`ByteSpan` in the **combined** coordinate system `base.len() + local`. Three
consumers resolve that span, and they are not interchangeable:

| Consumer | Mechanism | Cost |
|---|---|---|
| `view()` `:3469` | `StreamSource::Split { base, staged }` | one integer comparison, **no allocation** — safe per frame |
| `authored_source()` `:3561` | `Cow::Owned(base ++ staging)` | full memcpy, ~14 MB on the benchmark document — **once per operation only** |
| the writer | `DirtySet::combined_source` (`writer/mod.rs:488`), fed by `dirty_set()` step 4 | at save time |

A session that has authored nothing keeps `staging` empty, and the save path is
byte-for-byte identical to the pre-R45 path.

---

## 8. Traps

Ranked by how likely a new GUI is to hit them. Each is verified at HEAD.

### T-00 ★ `set_group_style` returns REGENERATED, not MOVED

**The worked example this document exists for.** Shipped 2026-08-13
(`7ebee12`, `dbc4aa9`).

```rust
pub fn set_group_style(
    &mut self,
    group: GroupId,
    style: crate::dimension::GroupStyle,
) -> Result<usize, EditError>                              // edit.rs:16052
```

Body (`edit.rs:16074-16090`): the returned `usize` is `members.len()`, where
`members` is *every wired member of the group* (`d.annot.is_some() &&
d.ap.is_some()`), unconditionally regenerated.

Method doc, verbatim (`edit.rs:16022-16024`):

> *"**Set a ce dimension GROUP's style defaults** and regenerate every wired
> member, as one undoable command (Pass 69.0). Returns how many members were
> regenerated."*

And on the command kind, `edit.rs:481-496`, verbatim:

> *"`members` counts what was REGENERATED, which is every wired member — not
> every member the change was VISIBLE on. Those differ whenever a member
> overrides the property that moved, and the count deliberately reports the
> cheaper, honest number: a regeneration that produced identical bytes still
> happened. A surface that wants to tell the operator how many ce dimensions
> actually MOVED must ask `crate::dimension::style_provenance` per member,
> because only that distinguishes 'followed the group' from 'overrode it'."*

**Why a GUI gets this wrong.** *"Applied to 17 ce dimensions"* is the natural
toast, and it is a confident answer to the wrong question: a member that
overrides the edited property regenerated to byte-identical output and did not
move. The method's own rationale (`edit.rs:16036-16043`) explains why it does
not filter — filtering would duplicate the cascade's logic in a second place
where the two could disagree.

**Two further facts the return value does not carry**, from the body:

- It is **not** the group's membership count either. Un-wired members
  (`annot`/`ap` absent) are excluded from both the regeneration and the number.
- `set_dimension_style` (`edit.rs:16115`) is `Result<usize, EditError>` too, but
  its `usize` is *"how many properties it overrides afterwards"* — a **property**
  count, not a member count. Two sibling methods, identical types, incompatible
  units. A status bar sharing one formatter prints "17 dimensions updated" for a
  3-property override.

**How to compute "actually moved".** For each wired member of the group, call

```rust
pdfce_core::dimension::style_provenance(&group, &member_overrides) -> StyleProvenance
//   crates/pdfce-core/src/dimension/style.rs:526
//   re-exported at crates/pdfce-core/src/dimension/mod.rs:90
```

read the `StyleSource` field named after the property you changed
(`StyleProvenance` has one field per property, `style.rs:390-405`), and count
those where `StyleSource::follows_group()` is `true` (`style.rs:378`).

**Sub-trap — do not hand-match the variant.** `follows_group` is
`matches!(self, Self::Factory | Self::Group)`. `Factory` **counts as moving**:
`style.rs:370-376` — *"a factory-sourced property DOES follow a group edit,
because the group has simply not spoken yet."* Counting only `Group` under-reports
by every member sitting on a factory default, which on a fresh document is most
of them.

**Sub-trap — four properties are two-tier only.** `style_provenance` never
returns `Factory` for `unit`, `fraction`, `decimal_marker`, or `standard`,
because the group's tier for those is a concrete field rather than an `Option`
(`style.rs:527-539`: *"saying otherwise would be a lie an operator could act on
('that will follow the factory default' — it will not; it follows the group)"*).
A "Factory" chip on those four in an inheritance panel is unreachable dead UI.

**Sub-trap — `StyleOverrides::tolerance`'s `None` is not "no tolerance."**
`style.rs:302-306`: `None` means *inherit*; "no tolerance" is
`Some(Tolerance::None)`, *"deliberately distinct from `None`"*. A checkbox that
writes `None` to mean "off" silently re-enables the group's tolerance.

**Sub-trap — pair the two queries against one snapshot.** `resolve_style` and
`style_provenance` are deliberately separate functions (`style.rs:521-524`);
values and sources are computed independently. Rendering a value from one and a
source from a stale call to the other shows mismatched pairs.

### T-01 ★ `document()` is the base; the canvas must render `view()`

§2.3. Shipped as a defect for 13 Passes. `edit.rs:3438-3444`.

### T-02 ★ `delete_subpath` has NO doc comment, and `delete_node`'s rustdoc opens with subpath semantics

`edit.rs:4770` — `pub fn delete_subpath` has **no `///` block at all**. The
preceding `///` run (`edit.rs:~4676-4749`) is one contiguous block and therefore
documents `delete_node` (`edit.rs:4751`) in full. Its first half describes
subpath deletion — *"Content-stream surgery via `plan_delete_subpath`… Lands as
one `CommandKind::DeleteSubpath`… **Deleting the only subpath deletes the
object**"* — and only then continues with `delete_node`'s actual contract.

**Consequence:** a GUI author reading `delete_node`'s rustdoc will implement
subpath semantics for a node delete, and will find `delete_subpath` documented
nowhere. `missing_docs` is **not** enforced on this crate (it appears only as an
aspirational comment at `crates/pdfce-core/Cargo.toml:108`), so nothing catches
it. Trust the **bodies**: `delete_node` → `plan_delete_node` +
`CommandKind::DeleteNode`; `delete_subpath` → `plan_delete_subpath` +
`CommandKind::DeleteSubpath`.

### T-03 ★ An absence assertion over incrementally-saved bytes is vacuous

§5.3. Applies to tests, audits, and any "removed permanently" UI copy.

### T-04 ★ R58 is documentation only — nothing forces a full rewrite for non-redaction removals

§5.4. `force_full` / `requires_full` / `RequiresFullRewrite`: **NOT FOUND** in
`crates/**/*.rs`.

### T-05 `find_text` is a wildcard search; `find_text_with` is not; `mark_redactions_by_search` is literal

`find_text` (`edit.rs:11792`) hard-codes `with_wildcards(true)`, so `#` matches
any ASCII digit and `?` matches any character — *"Searching for a literal `?`
therefore matches every character on the page"* (`edit.rs:11762-11790`).
`TextSearchOptions::wildcards` defaults to **`false`** (`edit.rs:6502-6520`).
`mark_redactions_by_search` matches **literally**, so pairing the two behind a
"redact every hit" control produces *"the search highlights hits the redaction
then declines to mark."*

**Fixed in the front end by design, not in the function** — a Find bar must call
`find_text_with` with default options and offer wildcards as an explicit toggle.

### T-06 `move_dimension` re-measures; `place_dimension` is what a drag does

`edit.rs:15804` vs `:16779`. `place_dimension` writes only fields the value
function does not read — value-preserving by construction. `move_dimension`
translates the measured points, *"it does take the ce dimension off the feature
it was measuring."* A drag handler wired to `move_dimension` silently changes
what the ce dimension says.

### T-07 `AnnotationDeletion::appearance_streams_removed == 0` means "not tracked", not "none"

`edit.rs:6030-6046`, verbatim: *"**Not a count of zero — a 'not tracked'.**"*
Always `0` on the delegated routes (`delete_redaction_mark`, `delete_dimension`)
and always `0` from `annotation_deletion_preview` (`edit.rs:11293-11298`:
*"reported as **0**, not computed"*).

### T-08 `FillOutcome::xfa_may_disagree` — a success return that is not a success

`edit.rs:5791`: the document also carries an XFA packet, so the filled value may
not be what an XFA-aware viewer shows, and *"which one an operator sees depends
on their viewer."* Rendering the new value with no warning shows a value some
readers will never display.

### T-09 `FieldRename::descendants_renamed` — one request can rename six fields

`edit.rs:6785`, `edit.rs:8865-8868`: only one dictionary is written; every
descendant's FQN re-derives. *"an operator not told so has silently broken every
FDF, JavaScript reference and submit mapping that named them (rule 4)."* A tree
view refreshing only the renamed row shows stale FQNs. Related:
`new_partial` is **one path segment**; feeding a displayed FQN back yields
`Personal.Address.Personal.Address.Zip`, and a period is refused outright.

### T-10 `InfoText::exact == false` ⇒ do not write the field back

`edit.rs:3282-3284`. Re-encoding would not reproduce the original bytes. A
metadata dialog that writes all fields on OK silently corrupts non-exact values.

### T-11 Never loop a singular vector verb over a selection

`edit.rs:4600-4620`: indices go stale between iterations (each call re-splices
the stream), and N calls are N undo entries. Use `move_objects` `:4574`,
`delete_objects` `:4641`, `move_nodes` `:5001`.

### T-12 `find_text`, `find_text_with`, `field_group_deletion_preview` take `&mut self` but change nothing

`edit.rs:11792`, `:11853`, `:8535`. They read `self.view()` or run a preflight,
so the borrow is exclusive. A shell holding any other borrow of the session
while the Find bar updates will not compile; a `RefCell`/`RwLock` wrapper must
take the **write** lock to search. Note the asymmetry:
`annotation_deletion_preview` (`:11316`) and `reset_preview` (`:12755`) are
`&self`.

### T-13 `import_form_data` is N undo entries; `set_info_field` may be zero

§3.4.

### T-14 `reflow_block` refuses after an in-session text edit on the same page

`edit.rs:4281-4290`: it is planned against the **base** document, so it returns
`ReflowApplyError::Unsupported` when `edit_text`/`format_text` already rewrote
that page's content object this session. *"Save and reopen to reflow after an
in-session edit of the same page."*

### T-15 One object, one merged write per command — last write wins, silently

`ARCHITECTURE.md` §11.1 correction (2026-08-03, Pass 17.1) and
`edit.rs:14026-14033`. `EditSession` applies a command's `ObjectWrite`s in
sequence against the **pre-command** state, so a second whole-dictionary write to
the same id **replaces** the first. Found via `flatten_fields`, which issued
three whole-dict writes to one page object (`/Contents`, `/Resources /XObject`,
`/Annots`); the last landed and silently discarded the other two, so every
flattened form lost its burned-in values **while still reporting correct
counters**. Binding rule: accumulate ONE merged dictionary write per id per
command.

Related, for anything building a multi-object command:
`edit.rs:7844-7851` — *"a parent this same call had just created is NOT there to
read — its `ObjectWrite` is still pending in the command being assembled, and
`self.value` sees committed state."*

### T-16 Redaction marks must be placed against the session, not the base

`edit.rs:11610-11626`: after `delete_pages` / `reorder_pages`, base page indices
and `page_slots` diverge, and a mark lands *"on a **different page** than the one
holding the matched text — silently, with correct-looking geometry."* A shell
caching `find_text` results across a page reorder and then redacting them
reproduces exactly this.

### T-17 `reset_form` removes `/V`, does not blank it, and never recomputes

`edit.rs:12815-12824`: *"An absent key and a key holding an empty string are
different bytes, a different incremental delta, and — for a choice field — a
different meaning."* Skipped push-button / signature / read-only fields are
**counted**, never silently cleared. `/DV` is never written. Calculated fields
are **not** recomputed (`edit.rs:12869`). `reset_preview` returns a row for
**every** field in scope including no-ops — filter on `would_change`.

### T-18 `delete_field` / `delete_field_group` / `delete_widget` remove different amounts

`edit.rs:8464` / `:8574` / `:8764`. `delete_field_group` on a terminal returns
`NotAGroupingNode` and is *"deliberately not redirected to `delete_field` — the
two remove different amounts, and guessing which the caller meant is exactly the
sneakiness rule 4 forbids"* (`edit.rs:8528-8532`). Preview with
`field_group_deletion_preview` before offering the group verb.

### T-19 `TextSearchOptions::whole_word` does not fix cross-run matching

`edit.rs:6572-6584`: *"Matching itself remains **per run** … a needle split
across two runs is still not found at all, with or without this option."*
`word_boundary` is ignored when `whole_word` is `false` but must not be reset on
toggle (`edit.rs:6588-6592`).

### T-20 A successful push-button creation yields a control that does nothing

`edit.rs:1077-1090`: *"the only creation verb whose successful result is a
control that does not work."* The disclosure says so; a shell that reports
"field created" without it ships a dead button.

### T-21 `FieldDefaults` caption ambiguity is silently resolved

`edit.rs:1159-1161`: `caption` and `on_state` are read from the **first** widget
of a multi-widget field. On-state disagreement is reported
(`defaults_on_state_ambiguous`, `edit.rs:1070-1076`); **caption disagreement is
not.** A "copy defaults from" UI picks one of N captions with no flag.

### T-22 `set_dimension_display` commits on a no-op; `set_info_field` does not

§3.4. `edit.rs:15901-15908` argues the inverse policy deliberately: *"an early
return on `show_diameter == current` would make an undo stack that sometimes
gains an entry from a control press and sometimes does not."* Two same-shaped
setters, opposite policies. A caller wanting suppression must compare first.

### T-23 `embed_fonts` grows the file regardless of save mode; `unembed_fonts`'s saving depends on it

`edit.rs:16590-16599` vs `:16337-16345`. Same-shaped plan struct, one figure is
mode-dependent and one is not.

### T-24 `add_radio_button` can return `Err` after committing

§6.3. The only verb where an `Err` does not mean "nothing happened."

---

## 9. Stability notes, per area

Evidence: `git log -- crates/pdfce-core/src/edit.rs` shows **68 commits since
2026-07-25**. `EditError` and `CommandKind` are both `#[non_exhaustive]`, so a
consumer must have a `_ =>` arm on every match; adding variants is not a
breaking change and the project treats it as routine.

| Area | Assessment | Evidence |
|---|---|---|
| Session construction, overlay model, `commit`/`undo`/`redo`, `dirty_set` | **Stable.** Shipped Pass 3.1 (2026-07-31) and unchanged in shape since; `ARCHITECTURE.md` §11.5 records it as the executable form of a design-locked section, pinned by a 2,897-file corpus assertion. | `edit.rs:3325-3660`, `ARCHITECTURE.md` §11.5 |
| `to_incremental_bytes` / `to_full_bytes` / `SaveReport` | **Stable.** Two-method shape unchanged since Pass 3.x; `SaveReport` is `#[non_exhaustive]` and has gained fields additively (`objects_deleted`, `delinearized`). | `writer/save.rs:208` |
| Guard model (encryption / certification / `/Size`) | **Stable in shape, still growing in coverage.** The three-gate certification split is argued as a permanent design (`edit.rs:12253-12266`). The encryption guard is explicitly a *forward-compatible seam* — `edit.rs:19338` records that no loadable file currently reaches it, so its behaviour when encrypted loading ships is **UNVERIFIED — re-check when `Pass 5` (Encryption) delivers a decrypting loader**. | `edit.rs:6970`, `:11449`, `:12289`, `:19338` |
| Forms (authoring, structure, values) | **Recently active.** Four of the last fifteen `edit.rs` commits touch forms (`3fe8a19` border spec, `ce5642d` hybrid fail-open, `f83be5a` four authoring properties, `7d2b71b` reset-form). Expect additive changes to `New*Field` specs and to `FieldAuthorDisclosures`. | `git log -15 -- crates/pdfce-core/src/edit.rs` |
| ce dimensions (14 verbs) | **★ Actively changing — the least stable area.** `set_group_style` / `set_dimension_style` and the whole `dimension::style` cascade shipped **today** (`7ebee12`, 2026-08-13), and `dimension::tolerance` shipped in the next commit (`dbc4aa9`, same day) adding the tenth and eleventh cascade properties. `SIDECAR_VERSION` is 2 and the project has an explicit, argued policy about when it bumps (`ARCHITECTURE.md` §12 entries (R), (S), (T)). Treat every ce-dimension signature as provisional. | `git log -3 -- crates/pdfce-core/src/dimension/style.rs` |
| Attachments (`attach_file` / `detach_file`) | **New.** Shipped 2026-08-12 (`74582ca`, `95c3416`). Only two verbs; the refused name-tree shape (`AttachmentTreeUnsupported`) is a known boundary. | `ARCHITECTURE.md` §12 (Q) |
| Fonts (`embed_*` / `unembed_*`) | **New.** Shipped 2026-08-12–13 (`f3acd24`, `d87fb58`). These are the only two verbs whose `*_refusal` accessor is load-bearing, which may or may not be a pattern the project generalises. **UNVERIFIED — whether the other four refusal accessors will be made load-bearing; nothing in the source states an intent either way.** | `edit.rs:16375`, `:16627` |
| Vector geometry (11 verbs) | **Stable in shape, `Vec<String>` return is a weak contract.** Every verb returns an untyped disclosure list. `ARCHITECTURE.md` §4.1 (C) records decision 027 already changed five `EditSession` signatures in this family once and removed two error variants. **UNVERIFIED — whether `Vec<String>` will become a typed disclosure struct; decision 027 moved in that direction for `PlannedEdit` but stopped at the session boundary.** | `edit.rs:4483-5057`, `ARCHITECTURE.md` §4.1 (C) |
| Page-text editing (5 verbs) | **Stable, but separate error universe.** These return `text_edit`'s types and have done since Pass 14.3. A shell needs a second error presenter for them. | `edit.rs:4126-4365` |

---

## 10. What this document could not establish

- **UNVERIFIED — whether any consumer is expected to call `to_full_bytes` for
  non-redaction removals.** The doc comments say front ends "must say so"
  (disclose), not "must choose full". `pdfce-gui` never offers it; `pdfce-cli`
  offers `--full-rewrite` opt-in. There is no stated project position on what a
  *new* shell should default to. Ask the operator, or follow the CLI.
- **UNVERIFIED — the behaviour of the encryption guard on a loadable encrypted
  document.** No such document can currently be loaded, so the 38 guard sites
  are untested against a real path. Re-check when `Pass 5` delivers a
  decrypting loader.
- **UNVERIFIED — whether `EditSession` will ever gain a session-level save
  outcome type.** `SaveOutcome` is private to `pdfce-gui`; `WriteReport` does
  not exist. A new shell must define its own and map from `SaveReport` +
  `SignatureImpact`.
- **`impl EditError` block / `EditError::is_*` helpers — NOT FOUND.** Callers
  must use `matches!`.
- **`begin_command` / `push_command` / any transaction API — NOT FOUND.** The
  `Command` value is the transaction.
- **`WriteReport` — NOT FOUND** anywhere in the workspace.
- **`force_full` / `forces_full` / `requires_full` / `RequiresFullRewrite` —
  NOT FOUND** anywhere in `crates/**/*.rs`.

---

*Part 1: `01-reading-and-model.md` — loading a `Document`, the object model,
the read-only graph, page trees, extraction.*
*Part 3: `03-capabilities.md` — the per-feature guides: ce dimensions, forms,
annotations, redaction, OCR, printing.*
