# Decision 018 — The canvas renders the edited document

**Date:** 2026-08-02
**Status:** Decided (engineer), pending Pass 17.0
**Decided by:** KenAgent decision protocol (`docs/decisions/README.md`).
**Numbering note:** this record was authored concurrently with
`017-tabbed-dockable-panel-system.md` and both were drafted as "017". This one
is renumbered to **018**; 017 keeps the dock decision. See the standing-rule
collision note below.
**Supersedes/amends:** nothing; extends the `ObjectGraph` migration begun in
`crates/pdfce-core/src/graph.rs` (Pass 3.2) into `pdfce-render`.
**Proposed standing rules:** preview-equals-saved, and an operator-visible
definition of done. **RULE-NUMBER COLLISION — decision 017 also proposes
R80–R84.** These two were drafted concurrently against the same "highest
existing rule is R79" reading. The librarian assigns the real numbers; this
record's two rules are provisionally **R85** (preview-equals-saved) and **R86**
(operator-visible done). Do not cite R80/R81 for these.

---

## 1. The defect

The GUI never shows unsaved edits. Every editing feature shipped from Pass 3.1
through Pass 16.2 — dimensions, add-text, in-place text edit, reflow, markup
annotations, vector move/delete/node-drag — writes into `EditSession`'s
in-memory overlay, and the canvas rasterizes the **base** revision.

The defect is **not** fourteen broken features. It is one shared read path:

- `OpenDoc::rasterize_current` (`pdfce-gui/src/main.rs:1555`) passes
  `self.session.document()` — whose own doc comment says
  *"**This is the base revision, not the edited state.**"* (`edit.rs:962`).
- `OpenDoc::ensure_object_provider` (`main.rs:1473`) does the same, so
  hit-testing, selection and snapping also see only base geometry.
- `survey_page_annotations` (`pdfce-render/src/annot.rs:116`) reads `/Annots`
  off the base — which is why authored dimensions and markup annotations never
  appear even though their objects exist — and `optional_content_default_off`
  likewise, so layer visibility never reflects unsaved state.

Meanwhile `EditSession::pages()` walks `SessionGraph`, so the **page list** is
edit-aware while the **raster** and the **object model** are not.

### 1.1 Cache invalidation was never the problem

`refresh_pages` (`main.rs:1434`) already nulls `page_texture`, resets
`ThumbnailCache` and nulls `provider_page` on every edit, undo and redo. Its
doc comment is the fossil of the bug, verbatim:

> *"the document is not reloaded, because the base revision (and therefore
> every byte span the renderer resolves) has not changed."*

True through Pass 3.1. **False since Pass 6.1** introduced staged appearance
streams. The GUI faithfully re-rasterizes on every edit and faithfully
reproduces the base. Fix the parameter type and the pixels change; no
generation key is needed. Correct this comment (and the stale claim at
`main.rs:7684`) in the same commit.

---

## 2. Decision

**Generalize `pdfce-render` and `pdfce_core::vector::decompose_page` over the
existing `ObjectGraph` trait, threaded through `DocumentView`, extended with a
`StreamSource` that resolves a `ByteSpan` against either a contiguous buffer or
the disjoint base+staging pair.** Option (a).

---

## 3. Why (a) is far smaller than it looks

Measured, not assumed. `pdfce-render`'s *entire* `Document` surface is **three
methods, 50 call sites**:

| Method | Sites | Status |
|---|---|---|
| `doc.resolve(…)` | 44 | **Already on `ObjectGraph`, byte-identical signature** |
| `doc.bytes()` | 5 | Needs `StreamSource`; all 5 are `span.slice(doc.bytes())` |
| `doc.resolved(…)` | 1 | **Already on `ObjectGraph`, byte-identical signature** |

No `doc.get()`. Nothing else. **45 of 50 call-site bodies compile unchanged.**

Three pieces already exist:

1. `pdfce_core::pageops::assemble::DocumentView` (`assemble.rs:166`) is already
   `{ graph: &dyn ObjectGraph, bytes: &[u8], version }` — precisely this
   abstraction, with a doc comment already explaining the R45 staging
   coordinate system and already instructing session callers accordingly.
2. `pdfce_core::annot::{page_annotations, optional_content_default_off,
   oc_is_hidden}` are **already** `<G: ObjectGraph + ?Sized>`. The project
   already committed to this pattern; `pdfce-render` is the unfinished half.
3. `EditSession::graph() -> SessionGraph` already implements `ObjectGraph` as
   base+overlay.

This is not a refactor. It is **finishing a migration already begun**.

---

## 4. Stream data and staging

Authored stream payloads (dimension `/AP`, markup appearances) live in
`EditSession::staging`, not in `Document::bytes()`. `stage_bytes`
(`edit.rs:4651`) assigns `start = base.bytes().len() + staging.len()`.

**Rejected:** `EditSession::authored_source()`. It returns `Cow::Owned` when
staging is non-empty — a full `base ++ staging` memcpy, ~14 MB per call on the
benchmark file. Correct for its once-per-operation `pageops` callers; wrong for
a per-frame render loop. Add a "not for per-frame use" warning to its doc.

**Rejected:** caching a concatenated buffer in the session — one base-sized
allocation plus invalidation logic, for no benefit over the below.

**Chosen:**

```rust
pub enum StreamSource<'a> {
    Contiguous(&'a [u8]),                       // a plain Document
    Split { base: &'a [u8], staged: &'a [u8] }, // an EditSession (R45)
}
impl<'a> StreamSource<'a> {
    pub fn slice(&self, span: ByteSpan) -> Option<&'a [u8]>;
}
```

**A span provably never straddles the boundary:** staged spans start at
`>= base.len()` by construction; `Provenance::File` spans end at `<= base.len()`.
Dispatch is one comparison. Zero copy, zero allocation, no invalidation. Owe it
an explicit non-straddling test, so a future change to `stage_bytes`' offset
scheme fails loudly rather than silently mis-slicing.

---

## 5. The object-model / hit-test half

Same mechanism, same commit. `decompose_page` (`vector/decompose.rs:584`) uses
6 `doc.resolve()` (unchanged) plus `ContentStream::from_page` (one `doc.get` →
`view.graph().value`, one `doc.bytes()` → `view.slice`). Change its parameter to
`&DocumentView`, then `ObjectModelProvider::build(self.session.document(), …)`
→ `(&self.session.view(), …)`.

**Snapping is free.** The snap engine reads `page_objects()` off the *same*
`ObjectModelProvider` (the Pass 12.M1 §10 ask #4 wiring), so there remains
exactly one decomposition per page and no second path to diverge.

Raster, object model, hit-test and snapping are therefore fixed by the **same
parameter-type change on the same three methods**. Consistency is structural,
not maintained by discipline — the operator cannot get a page showing an object
he cannot click.

---

## 6. Measured latency

13.9 MB / 318 pages / 2,998 objects, real-world linearized PDF, shipped release
CLI, process-start baseline (49 ms) subtracted:

| Operation | Net |
|---|---|
| Parse | ~18 ms |
| **(b)** `save_incremental` (empty) + reparse | **~44 ms** |
| **(b)** `append-identity` (2,998 objs, 28 MB) + reparse | ~77 ms |
| Render one page @ scale 1.0 | **~140–160 ms** |
| **(a)** overhead | **~0 ms** (one vtable hop + one small-`BTreeMap` probe per resolve) |

(a) is trivially inside the ~100 ms budget. (b) is inside it *on this file*, but
scales with **file size, not edit size** (~600 ms+ on a 200 MB scan) and roughly
doubles memory.

Separately worth recording: **the existing render is already ~140–160 ms/page**,
so edit-to-visible under 100 ms is not met by *any* option today. That is a
pre-existing render-performance roadmap item, not something this decision
creates — but it is why (b)'s 44 ms tax lands on an already-over-budget path and
(a)'s ~0 ms does not.

---

## 7. Alternatives rejected

### (b) Re-serialize + re-parse after each edit — **disqualified**

Not on performance. **It routes *viewing* through the *writer*, so the viewer
inherits every refusal the writer is contractually obliged to make:**

- `WriteError::RecoveredBaseForbidsIncremental` (R67, `writer/save.rs:258`)
  refuses incremental save on a recovered document **even with an empty dirty
  set**. The fallback `save_full` **refuses a hybrid file by name** (§5.6). So
  recovered-and-hybrid documents could display **nothing**. A viewer must never
  be less capable than the parser — and Pass 13b exists precisely to make 566
  such files openable.
- `EditError::ObjectCreationWouldExposeHiddenObjects` makes `/Size`-suppressed
  files un-previewable.
- §5.4's warn-before-save on live-linearized files forces either per-commit
  disclosure spam or a **silent save variant** — and a silent save variant
  sitting in the codebase is exactly the artifact that later gets reused on a
  real save path and quietly drops a disclosure the operator was owed. A
  *fuzzy-never-sneaky* hazard created purely by the architecture choice.

**On §5 specifically:** (b) does **not** violate the round-trip invariant. The
shadow bytes are discarded, `save_incremental` is `&self`, and the live base and
`staging` are untouched. It is rejected on capability and coupling, not on §5.

**Not a stepping stone.** Nothing (b) builds — shadow-document lifecycle, span
remapping, refusal fallbacks, second-`Document` staleness keys — survives into
(a). The only shared work is "stop calling `session.document()`", one line
either way. (b)-then-(a) costs strictly more than (a).

### (c) GUI-side overlay compositing — **a trap, confirmed**

Cannot represent content-stream surgery, which is *most* of what shipped:
`edit-text`, `reflow`, `object-move`, `object-delete`, `node-move`,
`redact-apply`. Cannot fix the hit-test half at all. Creates a second painting
path that must agree with `pdfce-render` pixel-for-pixel — the exact
"two decompositions quietly diverge" Z2 pattern `object_provider.rs`'s own doc
comment cites decision 011 warning against. And it drags PDF appearance-stream
painting into `pdfce-gui`, eroding §3's separation in spirit and making the web
fork *more* expensive, not less.

### (d) variants

- *Dirty-page-only re-parse:* inherits every (b) refusal, and is not expressible
  — the incremental writer's unit is the object, not the page.
- *Render-time graph adapter:* this **is** (a).
- *Hybrid b-now/a-later:* loses on the measurement; (a) fits in one Pass.

---

## 8. Slice plan

**Pass 17.0 — "the canvas renders the edited document."** One slice makes edits
visible, because raster/object-model/hit-test/snapping/annotations/OCG all
bottom out in the same three methods.

*Core:* promote `DocumentView` into a top-level `pdfce_core::view` (re-export
from `pageops`); add `StreamSource`; `impl ObjectGraph for DocumentView`
delegating (**this is what keeps 45 render sites untouched**); add
`EditSession::view()` and `Document::view()`; generalize
`ContentStream::from_page` (audit its **14** callers for base-vs-session intent)
and `decompose_page`.

*Render:* 27 params `&Document` → `&DocumentView`; fix the 5 `bytes()` sites.
**Keep `render_page` / `render_page_with` (`&Document`) as thin wrappers** — so
`pdfce-cli`, `tools/roundtrip`, `tools/font-parity` and every existing render
test are untouched.

*GUI:* two lines (`main.rs:1555`, `main.rs:1473`), plus the two false doc
comments.

*Gates:* `cargo tree -p pdfce-core` / `-p pdfce-render` clean of
egui/eframe/winit/wgpu/glow; `cargo fmt --check`; `cargo clippy -- -D warnings`;
the preview-equals-saved oracle; `tools/roundtrip` corpus sweep unchanged and
green (proving the read-path change perturbed no writer behavior); the
`StreamSource` non-straddling test.

**Pass 17.1 — finish the `session.document()` audit.** Triage each remaining
site, recording intent:

| Site | Impact |
|---|---|
| `main.rs:4606` `count_redaction_marks` | **redaction marks added this session are not counted in the GUI** |
| `main.rs:4598` `need_appearances` | stale |
| `main.rs:6078` `page_font_entries` | font list stale after a font-changing format edit |
| `main.rs:1377 / 2300 / 2391 / 4953` | triage individually |
| `main.rs:1779` `recovery()`, `4491` `version()` | **legitimately base reads — leave alone** |

Thumbnails need a read fix, not a key: `refresh_pages` already clears the cache
wholesale.

**Pass 17.2 — CLI parity + headless oracle harness.** No observable CLI change
(rule 11): the wrappers preserve every signature, and the one-shot
`parse → edit → save` model never renders an unsaved session. What it enables is
a headless way to render a live `EditSession`, needed by the oracle. Prefer a
`tools/` harness or hidden test-only subcommand over widening the public CLI
surface.

---

## 9. The test that should have existed (proposed rule, provisionally R85)

> **Preview-equals-saved.** For every editing operation, the raster of the
> edited session view is pixel-identical to the raster of the saved-then-
> reloaded document. What the operator sees before saving is what they get
> after saving.

Headlessly checkable, reusing the raster oracle `round-trip` already has. Cover
`add-text`, `annotate`, `dimension-add`, `object-move`, `object-delete`,
`node-move`, `edit-text`, `format-text`, `reflow`, `flatten`, `fill-field`,
`redact-apply`.

This is the exact invariant whose absence let the defect survive fourteen
editing Passes: **every Pass proved *saved* output correct; none proved
*displayed* output correct.** It inverts cleanly against R46 (proves presence)
and §5.9's absence test (proves deletion).

---

## 10. Risks to the two invariants

**GUI-core separation (§3):** none. `DocumentView` moves between `pdfce-core`
modules; no crate gains a dependency. The standing `cargo tree` gate still runs.
*The real separation risk in this decision space is option (c).*

**Round-trip / minimal-diff (§5):** none from the change itself — (a) is a pure
read path and cannot perturb saved bytes under any bug. Two hazards named so
they cannot be introduced later:

1. **`DocumentView` must never become the writer's input.** The writer's source
   of truth stays `&Document` + `DirtySet::combined_source(base)`. A future
   refactor generalizing `save_full`/`save_incremental` over `DocumentView`
   could let a session's `StreamSource::Split` be mistaken for base bytes and
   splice staged bytes at wrong offsets — a **silent** §5 violation. Put the
   prohibition in the type's own doc comment (the codebase's established habit;
   `DocumentView`'s existing R45 note is the same genre) and implement nothing
   on it the writer would need.
2. **`Page` is a snapshot and stays one.** `Page { contents, resources }` is
   captured at `refresh_pages` time — fine today because every commit funnels
   through it. Audit that **all** commit sites do, canvas vector edits and
   text-edit Accept in particular. A missed one yields a correct view paired
   with a stale `Page`: harder to see than the current bug.

Non-invariant: `SessionGraph::value` probes `state` before `base`. Irrelevant at
current overlay sizes; measure only if a Pass ever creates a five-figure overlay.

---

## 11. For the operator

1. **Status correction.** The underlying work *is* functional — Ken's instinct
   was right. Every editing Pass was proven headlessly and each wired its GUI
   input and commit correctly. One shared read-path bug, not fourteen broken
   features.
2. **Operator decision — proposed rule (provisionally R86):** *a Pass that adds
   or changes operator-facing behavior does not ship until that behavior has
   been observed working in the running application, not merely tested
   headlessly.* Every Pass 3.1–16.2 met its stated gates and shipped a feature
   the operator could not see. That is a **gate** defect, not an engineering
   defect, and redefining "done" is his call. Engineer recommends yes.
3. **Operator decision — sequencing:** Pass 17 before new feature work. It gates
   meaningful acceptance of everything already shipped.
