---
name: project-pass-136-form-recursion-and-filing-count
description: Pass 136.0 (83fca59) + 136.1 (f62df4e, HEAD), 2026-08-27, 280th filing — objects inside a form XObject are now selectable; leaves are a SEPARATE list from PageObjects::objects, never merged, because eleven edit.rs sites resolve a page-relative token range.
metadata:
  type: project
---

**Pass 136.0** (`83fca59`) — `decompose_page` now descends into form
XObjects; leaves live on a **new, separate** `PageObjects::leaves`, never
merged into the existing flat `objects` list. **Pass 136.1** (`f62df4e`,
`HEAD`) — `hit_test_point_deep` + `HitTarget::{Object,Leaf}` do the actual
selection; `hit_test_point`/`hit_test_point_all` are UNCHANGED on purpose.

**Why the separate-list design matters, and why it's a reusable shape:**
eleven `edit.rs` sites resolve a paint-order index and write to the
**page's** content stream; a leaf's token range indexes the **form's**
stream. Merging the lists would let those sites apply a form-relative
range to the page and corrupt it silently, in bounds, nothing to catch it.
Keeping two lists makes all eleven sites correct **by construction**
rather than by a guard added to each — same shape that produced
`Pass 130.3` the day before (guard-per-site instead of design-that-can't-
be-wrong). **Cite this forward** whenever a new recursive/nested read path
is proposed alongside an existing flat index that editing code resolves
against — the question to ask first is "does anything treat this index as
an offset into a specific buffer?"

**Also worth reusing:** grepping `text_extract` (which already recursed
into forms since Pass 1.1) before building the vector-side recursion
turned a large Pass into a small one — it already had the cycle guard,
depth bound, and the `ContentStreamRef`/`is_editable()` vocabulary the new
`FormLeaf` now reuses directly. Always check the sibling model
(text vs vector) for a already-solved recursion before rebuilding one.

**Depth constant deduplicated:** `pdfce-render::MAX_XOBJECT_DEPTH` and
`text_extract::max_form_depth` (docstring only *asserted* they matched) are
joined by `content::MAX_FORM_DEPTH = 64` (2x veraPDF's conformant 32-deep
chain). Retiring render's own copy is FILED AS OWED, not done — cross-crate
breaking change.

**FEATURES.md:** new Planned row (Vector objects section) — `[x]` core /
`[ ]` cli (`object-list` still flat-only) / `[ ]` gui. Not rounded up.

**Filing-count landmark:** SESSION_LOG filings reached **280** as of this
entry (2026-08-27). Ledger unchanged: rules `R218` (next free `R219`),
decisions `089` (next free `090`) — per engineer, not independently
re-run (no shell available to this role this filing).
