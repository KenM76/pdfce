# NEXT SESSION — start here

**Written 2026-08-09.** Read this, then `docs/ROADMAP.md` and the latest
`docs/SESSION_LOG.md` entry as usual. This file is a *handoff*, not a
record — the record is the librarian's. Delete or overwrite it once its
contents have been acted on.

Not owned by `pdfce-librarian`. It is safe to edit here without racing a
filing.

---

## State at handoff

- Branch `pass-8-redaction`, working tree clean, HEAD = **`d2d03a5`**.
- **2643 workspace tests, 0 failed.** `cargo clippy --workspace
  --all-targets` = 0. `cargo fmt --all --check` clean.
- All pre-existing gates under `tools/` were clean at last run.

### Permissions were changed this session

The operator reported being *"constantly nagged for permissions"*.
`.claude/settings.local.json` now sets `defaultMode: "bypassPermissions"`.
It is machine-local and **not** checked in (a global gitignore rule at
`C:\Users\Ken/.config/git/ignore` covers `**/.claude/settings.local.json`).

**This did not weaken project rule 8.** `bypassPermissions` skips
*prompts*; it does not skip *denials*, and deny rules are evaluated first.
`git push`, `git remote add/set-url`, `gh repo create`, `gh release`,
`gh pr create`, `cargo publish` and `npm publish` are all **denied**, so
publishing is still mechanically blocked rather than merely remembered.
History-rewriting commands are on `ask`, which `bypassPermissions`
explicitly does not skip.

---

## What shipped this session (Pass 52 — PDF→DXF export)

| Slice | What | Commit | Filed? |
|---|---|---|---|
| 52.0 | Core DXF writer (`crates/pdfce-core/src/export/dxf.rs`) | `3c4aca4` | librarian dispatched |
| 52.1 | `pdfce-cli export-dxf` | `3c4aca4` | librarian dispatched |
| 52.3 | Page text → `TEXT` entities on layer `PDFCE_TEXT` | `1f4839d` | librarian dispatched |
| 52.2 | **core+CLI half only** — scale derived from ce dimensions | `d2d03a5` | **NOT filed — owed** |

52.0/52.1/52.3 were filed by `pdfce-librarian` (ROADMAP Shipped entry,
FEATURES row `core`/`cli` ticked with `gui` still open, SESSION_LOG entry,
ARCHITECTURE §12 **decision 035**, plus RAG escalations to
`C:\personal_rag\dxf\` and a new `D:\dev\rag\rust\` methodology file).
**`d2d03a5` post-dates that filing and is not in it** — file it first
thing, together with the GUI half when that lands.

One caveat the librarian raised itself: it was dispatched without a shell,
so it could not run `git show` and the two commit hashes in its filing are
**relayed from the dispatch, not verified against `git log`**. It said so
in every file it touched. Worth a spot-check.

**Origin.** The operator asked whether pdfce could satisfy SOLIDWORKS'
"PDF import needs Acrobat Pro licensed" gate. Answer given: no, and it
does not matter — doing so would mean impersonating Adobe's COM
registration to defeat another vendor's licence check (**declined by
name**), and it would buy nothing, because SOLIDWORKS imports DXF
natively with no Adobe dependency. Pass 52 makes the gate irrelevant
instead of working around it.

### Read before touching any of it

`crates/pdfce-core/src/export/dxf.rs`'s module docs carry the full
rationale: the AC1015-vs-R12 correction, the AutoCAD LT 2004 constraints
that hold by construction, why arcs are recognised rather than flattened,
and why text goes on its own layer.

---

## THE NEXT TASK — Pass 52.2, GUI half

The core substrate is **done and tested**; what remains is GUI wiring.

### Substrate that already exists

```rust
// crates/pdfce-core/src/export/dxf.rs
pub enum DxfScaleSuggestion {
    Uncalibrated,
    Calibrated { scale: f64, units: DxfUnits, group: String, agreeing: usize },
    Conflicting { candidates: Vec<DxfScaleCandidate> },
}
pub fn suggest_scale(model: &DimensionModel) -> DxfScaleSuggestion;
pub const fn DxfUnits::for_unit(unit: Unit) -> DxfUnits;
```

Tested in `crates/pdfce-core/tests/dxf_scale.rs` (6 tests). The
unit-cancellation (`effective_scale(u) / u.baseline_per_point()`) is
load-bearing — without it a millimetre group and an inch group describing
the same 1:1 sheet look like a conflict.

### `pdfce-ui-specialist` design (dispatched and returned this session)

Its verdict, condensed. It read `ribbon.rs`, `main.rs`, `ui_text.rs`,
`split.rs`, `edit.rs` and both disclosure gates before answering.

**1. Placement — `RibbonTab::File`, new `RibbonGroup::Export`.**
Not `Edit`: DXF export does not change the open document, it produces a
derivative file — the same shape as Save-As. Do **not** copy the
`Action::ExportFormData` precedent (a button inside the Forms dock); that
is scoped to a panel that exists for an unrelated reason. One button,
labelled `Export DXF…` (ellipsis = dialog-opener, matching `Set scale…`).
`pdfce_core::export` holds only `dxf.rs`, so a menu would be premature.

**2. Options — one non-modal `egui::Window`**, not a wizard:

```
Export DXF
├─ Page(s):  <read-only, from doc.selected_pages / current page>
├─ Units:    ( ) Inches   ( ) Millimetres        [radio row, not a ComboBox]
├─ Scale:    [________]   <caption — see 3>
├─ Text:     ( ) Include as TEXT entities  ( ) Omit entirely
├─ ▸ Advanced                                    [CollapsingHeader, closed]
│    ├─ Fit circles/arcs  [x]
│    └─ Arc tolerance (pt): [0.05]   <enabled only when Fit is checked>
└─ [ Cancel ]                        [ Export… ]  <- disabled until scale resolves
```

`fit_arcs=true` / `arc_tolerance=0.05` stay implicit — they match
`DxfOptions::default()` and the core docs already justify them.

**3. Scale disclosure — the load-bearing part.**

A `File`-tab window has a fixed position, so decision 024 §4.4's complaint
(a confirm box anchored to page geometry that moves on zoom/scroll) does
not apply here. It was never at risk.

Reuse `ui_text::group_scale_summary(scale, unit)` (`ui_text.rs` ~L5619) —
it already renders the tri-state as `"no scale set"` / `"1:1 (set by
operator)"` / `"1 mm = 28.35 pt"`. **Do not invent a "(1:2)" ratio
string**; the established convention is `1 unit = n pt`.

- **Calibrated** → pre-fill the field; caption underneath:
  *"pre-filled from group "Floor Plan" — 1 mm = 28.35 pt. Change it if
  this isn't the right one."* An editable field the operator can see and
  overwrite before a fixed Export button **already satisfies rule 4** —
  this action mutates nothing in the open document, so no Accept/Reject
  step. (`MeasureScale`'s Accept exists because accepting re-values every
  dimension in the group; nothing here does.)
- **Uncalibrated** → **do not pre-fill 1.0.** Leave blank, disable
  `Export…` until the operator either types a number or ticks a distinct
  toggle: *"Export at paper scale (1:1) — this drawing has no scale set."*
  The CLI can only warn after the fact; the GUI can gate before it. The
  destination is a plasma table and a wrongly-scaled cut is metal.
- **Conflicting** → radio list, nothing pre-selected, plus "Enter a scale
  manually". `Export…` disabled until one is picked.

**4. Accessor gap — this is a real blocker, and it is a `pdfce-core`
change.** `DimensionRecord` carries **no page association**;
`DimensionModel::groups()`/`dimensions()` are document-global. The only
place page ownership is resolved is `EditSession::dimension_rects(page_index)`
(`edit.rs` ~L12796), which cross-references each dimension's annotation
`/P` against the page object id. Needed:

```rust
/// Every distinct group with at least one dimension on `page_index`,
/// resolved the way `dimension_rects` resolves page ownership.
pub fn dimension_groups_on_page(&self, page_index: usize) -> Vec<GroupId>
```

Additive, mirrors an existing pattern. Per rule 2 the GUI must not
reconstruct `/P` ownership itself.

*Note:* `suggest_scale` currently takes the whole `DimensionModel` and is
therefore document-wide. Once the accessor exists it likely wants a
page-scoped sibling, or a filtered model. **The CLI's behaviour is
document-wide today and that is a known, unstated limitation** — worth
deciding rather than inheriting.

**5. Outcome disclosure — a new sibling field, not `SaveOutcome`.**
`SaveOutcome` is specifically about writing the open document's edits;
DXF export never touches it. Add `dxf_export_result: Option<…>` rendered
by its own `dxf_export_result_bar(ui)` beside `copy_result_bar` in
`status_bar_body`. Not a toast (fades before *"are labels missing?"* gets
asked), not a modal (friction this doesn't earn). Show only non-zero
lines; warn-colour `unreadable_text` and `skipped_images`; plain colour
for `skipped_text` (that was the operator's own choice). Must go through
a traced setter or `tools/check-disclosure-channel.sh` fails.

**6. Multi-page.** Reuse `doc.selected_pages` (already driven by the Pages
dock, used by rotate/delete). Empty → current page, `save_file()`.
Non-empty → one DXF per page into `pick_folder()`, named
`"{stem}_p{page}.dxf"` zero-padded to the widest page number. Don't expose
an editable template in P0. If the selected pages don't share one detected
scale, leave the field blank and say so.

**7. Correctness trap flagged unprompted — heed this one.** The GUI must
call `decompose_page` against **`doc.session.view()`** (the overlay-aware
`EditSession::view()`), never a fresh `Document::load` of the on-disk
bytes. The CLI legitimately uses `Document::load` because it has no
session. Copying that call into the GUI would export a DXF that silently
does not match what is on screen for anyone with unsaved edits.

**8. Strings** — full list of `ui_text.rs` names in the specialist's
reply; naming follows the existing `forms_export_*` / `group_*`
convention. Port the CLI's existing stderr prose verbatim where it already
says the right thing (`cmd_export_dxf`, `pdfce-cli/src/main.rs` ~L13241+)
rather than re-deriving new wording.

**9. Write path** — go through the existing `write_atomic(path, bytes)`
helper (`main.rs` ~L18234). Rule 5 is not scoped to the primary document;
a half-written DXF is the same failure for a file headed to a CNC pipeline.

---

## Also outstanding (carried, not new)

- **Eleven owed commits** in `tools/commits-filed-baseline.txt` still need
  proper filing. That file is DEBT, not an allowlist — shortening it is
  the intended direction, and `tools/check-commits-filed.py` explains why
  adding to it is forbidden.
- **`ae59ce3` claims "Pass 24.0 (part)"** while `ROADMAP.md` files 24.0 as
  NOT STARTED. Unresolved contradiction; untouched this session.
- **`LEGAL.md` needs the git-history blocker entry.** The librarian
  declined it as outside its remit, so it is owed by the engineer.
- **Publishing is blocked on an operator decision** (open question (bh)):
  confidential third-party material removed in `817d518` still exists in
  288 earlier commits. Options are rewrite history, squash to a fresh
  initial commit, or accept it. **Only Ken can decide this** — do not pick.
- **Decision record** for the Bézier→DXF mapping / `$INSUNITS` default /
  text-layer choice — librarian was asked to open one (likely **035**;
  034 is claimed for write-side CMYK) and to confirm the free number
  itself.

## A finding worth acting on

Twice this session I read a RAG file and then substituted my own guess for
its recommendation — the AC1015-vs-R12 mistake is the sharp one: the RAG
had already named AC1015, all 12 tests passed because they *grepped* the
output for strings, and it was caught only by parsing with the operator's
`ezdxf`. Standing rule **R172** currently reads "grep the RAG before
driving the harness"; it has now been **widened in place** by the
librarian to cover *substituting judgment for a recommendation already
read*, outside the GUI-harness domain it was minted for. Ceiling stays
R172, next free R173. Nothing further owed on this.

**Generalisable:** a grep-based test cannot validate a structured format.
Parse it with a real reader. `ezdxf` 1.4.3 is installed and
`d.audit()` returns errors/fixes — that is what caught this.
