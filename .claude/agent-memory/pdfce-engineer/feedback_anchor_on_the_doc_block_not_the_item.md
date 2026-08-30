---
name: anchor-on-the-doc-block-not-the-item
description: When splicing code before an item, anchor the patch on the FIRST LINE OF ITS DOC BLOCK, never on the `fn`/`struct`/variant line — anchoring on the item lands the new code between that item's doc comment/attributes and the item itself
metadata:
  type: feedback
---

When inserting code before an existing item, **anchor the replacement on the
first line of that item's doc comment**, not on its `fn` / `pub struct` /
variant line. Anchoring on the item line splices the new code **between the
existing item's `///` block (and its `#[derive]`/`#[error]` attributes) and the
item itself**.

**Why:** Rust attaches a doc comment and its attributes to whatever follows
them. Splicing in between silently re-parents them onto the new code. Two
distinct failure shapes, both seen:

- **Doc comment only** → the existing item loses its docs and the new item
  inherits them. In `clap`-derive this is *shipped UI* — the `///` IS the
  `--help` text (see [[a-doc-comment-can-be-shipped-ui]]).
- **Doc + `#[derive(...)]`** → a hard compile error (`E0119` conflicting trait
  implementations), because the derives now apply to the new struct as well.

**How to apply:** every `python -c` / `Edit` splice that inserts *before*
something. Concretely — anchor on `` "/// **Delete a ce dimension**…" ``, not
on `` "DimensionDelete {" ``; on `` "/// What `edit_widget` changed." ``, not
on `` "pub struct WidgetEditOutcome {" ``.

**This recurred THREE TIMES in the 2026-08-30 session alone**, in three
different shapes — a clap subcommand variant, a CLI handler `fn` (caught by
clippy's `doc_lazy_continuation` and by `tools/check-cli-help-leads.py`), and a
`pub struct` with derives (caught by `E0119`). The first two were caught by
gates; the third by the compiler. **A splice before an item is the single most
error-prone edit shape in this codebase**, and the fix costs nothing if the
anchor is chosen right the first time.

Related: [[inserting-before-an-anchor-orphans-its-doc-comment]] recorded the
original instance; this is the generalised rule plus the derive-collision
variant it did not cover.
