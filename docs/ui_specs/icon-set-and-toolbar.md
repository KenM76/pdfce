# UI Spec — ScripTree-style icon set + toolbar (operator priority #2)

> Authored by `pdfce-ui-specialist`, on dispatch from the engineer
> (`ROADMAP.md` "★ Icon set — ScripTree-style SVG icons for all GUI
> features," operator priority #2, filed 2026-08-01). This is the
> implementable icon→feature mapping + toolbar-layout spec; the engineer
> implements it, deviating only with a recorded reason (the standing
> Pass 3.2/6.1/7/8/12.0/14.3/15.2/16.2/12.M2 spec convention). This spec
> does **not** write Rust, does **not** pick the SVG-in-egui rendering
> pipeline, and does **not** decide licensing — those are named as
> explicit engineer/KenAgent flags in §7.
>
> Read before implementing: `D:\Dev\ScripTree\icons\*.svg` (the source
> set, audited in full in §1); `crates/pdfce-gui/src/main.rs` (the
> toolbar, read in full for this spec — §0 records exactly what exists
> today, confirmed by reading the code, not assumed); `crates/pdfce-gui/
> src/ui_text.rs` (every `*_button()`/`*_tooltip()` string that names a
> current glyph or label); `docs/ui_specs/pass-12.M2-dimension-tools.md`
> (the not-yet-built Measure ▾ menu this spec must also cover, since the
> operator's instruction is "all GUI features," not just shipped ones).

---

## 0. What exists today (audited by reading the code, 2026-08-01)

**There is no icon *image* anywhere in pdfce today.** Every "icon" in the
current toolbar is one of three things, and the three are not
consistent with each other:

| Kind | Examples | Where |
|---|---|---|
| Emoji glyph + text | `📂  Open…`, `💾  Save a copy…`, `🧰  Tools`, `📋 ▾` (Copy) | `ui_text::open_button`, `save_button`, `tools_button`, `copy_text_button` |
| Bare Unicode dingbat, icon-only | `◀`/`▶` (nav), `−`/`+` (zoom), `↺`/`↻` (rotate), `↶`/`↷` (undo/redo), `▤` (rail), `🗩` (annotations), `⌨` (shortcuts) | `prev_page_button`, `zoom_out_button`, `rotate_left_button`, `undo_button`, `rail_toggle_button`, `annotations_toggle_button`, `shortcuts_button` |
| Plain text, no glyph at all | `Properties`, `Fit page`, `Fit width`, `100%`, `Markup ▾`, `Text ▾`, `✎ Aa` (Edit Text), `+ Aa` (Add Text) | `properties_button`, `fit_page_button`, `fit_width_button`, `zoom_100_button`, `markup_menu_button`, `text_menu_button`, `edit_text_tool_button`, `add_text_tool_button` |

**Existing accessible-name infrastructure is real and must be preserved.**
`PdfceApp::icon_button` (`main.rs` ~line 3633) already wraps every
icon-only glyph button with `ICON_BUTTON_SIZE = (28.0, 24.0)` sizing and
an explicit `WidgetInfo::labeled` accessible name sourced from the
tooltip text — this is the P1-6 fix noted in the GUI-polish Shipped
entry. **An SVG-icon swap must reuse this exact wrapper, not bypass
it** — swapping the `glyph: &str` argument for an image widget inside
`icon_button` is the correct, minimal-diff way to do this (§4.1).

**Toggle-state cue today is bold text + background fill**
(`Self::toggle_label`, `main.rs` ~line 3650) — a selected
`selectable_label`/`Button::selectable` gets a background highlight
AND bold text, so colour is never the sole signal (rule 6, P1-1). An
icon-only button has no text to bold, so this spec names a replacement
non-colour cue for icon-only toggles (§5.3) — **this is the one place
an icon swap can silently regress an existing standing-rule guarantee
if done carelessly**, and is called out as a ❌-must-not-regress item.

**Toolbar structure (read in full, `main.rs` ~3658–4069), in order:**

```
[File]      Open…  |  Save a copy… (only when a doc is open)
────────────────────────────────────────────────────────────
[View]      Rail toggle (▤)  |  Annotations toggle (🗩, selectable)
────────────────────────────────────────────────────────────
[Nav]       ◀ Prev  |  "Page N of M"  |  Next ▶
[Zoom]      − | "NN%" | +  |  [Fit page] [Fit width] (selectable) | [100%]
────────────────────────────────────────────────────────────
[Edit]      ↺ Rotate-left | ↻ Rotate-right | [Properties] (selectable)
            | Markup ▾ (menu: colour + Rectangle/Ellipse/Arrow line/Highlight band)
            | Text ▾ (menu: colour + FreeText/Sticky/Stamp)
            | ✎ Aa Edit Text (selectable) | + Aa Add Text (selectable)
            | [Measure ▾ — NOT YET SHIPPED, see pass-12.M2, inserted here]
────────────────────────────────────────────────────────────
[History]   ↶ Undo | ↷ Redo
────────────────────────────────────────────────────────────
(gap)       📋 ▾ Copy (menu: Copy page text / Copy document text)
            🧰 Tools (opens the Tools dock)
            ⌨ Shortcuts (opens the shortcuts window)
```

Groups are `ui.separator()`-divided per the module's own documented
convention (`main.rs` ~line 125) — **this spec does not reorder or
regroup anything**; it only assigns an image to each existing control
and slots the not-yet-built Measure ▾ into the Edit group per
`pass-12.M2` §1.2's own placement reasoning (last item in Edit, right
after Add Text, before the History separator).

**Tools dock** (`ui_text::tools_dock_*`, advanced-bucket per the
five-way placement taxonomy — `ARCHITECTURE.md` §12(b)): Combine
files…, Split this document…, Insert pages from a file…, Font
folders….

**Not yet a GUI feature at all (confirmed by grep, §0 discipline
applied the same way `pass-12.M2` §0.1 applied it):** Search/Find (no
GUI text-search bar exists; `mark_redactions_by_search`/`find_matches`
are core+CLI only), Redaction mark/apply (only the status-bar
"unapplied marks" disclosure ships — `ui_text::redaction_marks_pending`
— the canvas tool is deferred), Form-fill (CLI-only; GUI is
`docs/ui_specs/pass-7-form-fill.md`'s still-open P0), Encryption (no
GUI or core write path at all yet — Pass 5 is queue-deferred). These
are covered at reservation depth in §8, not full interaction depth —
this is an icon spec, not a second interaction spec for those
features.

---

## 1. The ScripTree icon set — style contract (reverse-engineered from the source files, so new icons match exactly)

Every relevant file in `D:\Dev\ScripTree\icons\*.svg` (52 total; the
vendor-logo files — `icon-autocad`, `icon-inventor`, `icon-msoffice`,
`icon-revit`, `icon-solidworks` — are excluded below, §1.1) shares one
mechanical contract:

```xml
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48" fill="none" aria-hidden="true">
  <!-- Generic <concept> shape — placeholder, not a vendor trademark logo -->
  <path/rect/circle ... stroke="currentColor" stroke-width="2.5"
        stroke-linecap="round" stroke-linejoin="round"/>
</svg>
```

- **48×48 viewBox**, content inset ~6–8px from the edge on every icon
  inspected (e.g. `icon-lock`'s rect is `x=10..38`, `icon-search`'s
  circle radius 13 centred at (21,21)) — a consistent internal margin.
- **`fill="none"`, `stroke="currentColor"`** — every icon is a pure
  outline glyph that inherits the CSS/paint colour of whatever draws
  it. This is the property that makes the theming plan in §6 possible,
  and the property a new icon must not break (no icon in the set has
  a hardcoded fill colour except the two non-matching files named in
  §1.2).
- **`stroke-width="2.5"`** uniformly, **`stroke-linecap="round"`** and
  **`stroke-linejoin="round"`** on every multi-segment path — rounded
  terminals and joins throughout, never a sharp/mitred corner.
- **`aria-hidden="true"`** plus a one-line HTML comment naming the
  concept and explicitly disclaiming trademark risk — every file in
  the set was authored as a generic placeholder, not a copied vendor
  mark. **New icons this spec asks for must carry the same comment
  convention** (`<!-- Generic <concept> shape — placeholder, not a
  vendor trademark logo -->` or `-- generic, no trademark risk --` for
  the ones the source set itself marks risk-free, e.g. magnifier,
  folder, picture).

### 1.1 Files excluded from pdfce's icon language

`icon-autocad.svg`, `icon-cli.svg` (has its own terminal glyph but is a
ScripTree-brand concept, not a pdfce feature), `icon-inventor.svg`,
`icon-msoffice.svg`, `icon-revit.svg`, `icon-solidworks.svg` are
vendor-application logos (Autodesk/Microsoft/Dassault marks, even
rendered generically) — **none of these map to any pdfce feature and
none should be pulled into pdfce's asset tree.** `icon-mesh.svg` is a
literal duplicate of `icon-package.svg`'s path data (verified by
reading both — identical `d` attributes) with a different label; not a
distinct concept, skip it. `icon-forest.svg` is a completely different
asset family (1024×1024 viewBox, filled circles/lines, a fractal
tree-of-circles glyph, no `currentColor`/outline convention at all) —
this is ScripTree's own category-tree decoration, not part of the flat
tool-icon language pdfce is adopting; **do not use it as a style
reference for anything in pdfce.**

### 1.2 Two files that do not match the contract — flagged, not silently used

`icon-cli.svg` and (checked, not shown above) a couple of the
tech-stack files use `rx="3"` instead of the more common `rx="2"` and
otherwise match; this is a trivial, immaterial variance, noted only so
a future auditor does not treat it as evidence of a second style
generation. No file breaks the `currentColor`/`stroke-width:2.5`/
`viewBox 0 0 48 48` contract except the excluded ones in §1.1.

---

## 2. Icon → feature mapping — SHIPPED controls (the must-do-now set)

Legend: **REUSE** = an existing ScripTree SVG maps cleanly, no new art;
**NEW** = no existing file fits, a new SVG must be drawn *in the exact
style contract of §1* (construction described precisely enough to
draw); ⚠️ = a reuse that is safe only because the two controls are
never visible at the same time (different toolbar vs. dock vs. menu
context) — flagged so it is a conscious choice, not an oversight; ❌ =
a collision that would be visible at the same time and MUST use two
different icons.

| # | Feature (current `ui_text` fn) | Current glyph | Icon decision | Tooltip (unchanged, cite as-is) |
|---|---|---|---|---|
| 1 | Open (`open_button`) | 📂 + text | **REUSE** `icon-folder.svg` | `open_tooltip()` |
| 2 | Save a copy (`save_button`) | 💾 + text | **NEW** — see §3.1 "save" | `save_tooltip()` |
| 3 | Rail toggle (`rail_toggle_button`) | ▤ | **NEW** — see §3.1 "sidebar" | `rail_toggle_tooltip()` |
| 4 | Annotations toggle (`annotations_toggle_button`) | 🗩 | **NEW** — see §3.1 "comment-bubble" | `annotations_toggle_tooltip_shown/hidden()` |
| 5 | Prev page (`prev_page_button`) | ◀ | **NEW** — see §3.1 "chevron" | `prev_page_tooltip()` |
| 6 | Next page (`next_page_button`) | ▶ | **NEW** — mirror of #5 | `next_page_tooltip()` |
| 7 | Zoom out (`zoom_out_button`) | − | **NEW** — see §3.1 "magnifier±" | `zoom_out_tooltip()` |
| 8 | Zoom in (`zoom_in_button`) | + | **NEW** — mirror of #7 | `zoom_in_tooltip()` |
| 9 | Fit page (`fit_page_button`) | text only | **NEW** — see §3.1 "frame-fit" | `fit_page_tooltip()` |
| 10 | Fit width (`fit_width_button`) | text only | **NEW** — variant of #9, see §3.1 | `fit_width_tooltip()` |
| 11 | Actual size / 100% (`zoom_100_button`) | text only | **RECOMMEND: no icon** — see §3.2 | `zoom_100_tooltip()` |
| 12 | Rotate left (`rotate_left_button`) | ↺ | **NEW** — see §3.1 "rotate-page" | `rotate_left_tooltip()` |
| 13 | Rotate right (`rotate_right_button`) | ↻ | **NEW** — mirror of #12 | `rotate_right_tooltip()` |
| 14 | Properties (`properties_button`) | text only | **REUSE** `icon-document.svg` | `properties_tooltip()` |
| 15 | Markup ▾ (`markup_menu_button`) | text only | **NEW** — see §3.1 "shapes" | `markup_menu_tooltip()` |
| 15a | Markup → Rectangle (`markup_square_item`) | none | **NEW**, trivial — see §3.3 | — |
| 15b | Markup → Ellipse (`markup_circle_item`) | none | **NEW**, trivial — see §3.3 | — |
| 15c | Markup → Arrow line (`markup_line_item`) | none | **NEW**, trivial — see §3.3 | — |
| 15d | Markup → Highlight band (`markup_highlight_item`) | none | **NEW**, trivial — see §3.3 | — |
| 16 | Text ▾ (`text_menu_button`) | text only | **NEW** — see §3.1 "note" | `text_menu_tooltip()` |
| 16a | Text → FreeText (`text_freetext_item`) | none | **NEW** — see §3.3 | — |
| 16b | Text → Sticky (`text_sticky_item`) | none | **NEW** — see §3.3 | — |
| 16c | Text → Stamp (`text_stamp_item`) | none | **NEW** — see §3.1 "stamp" (shared, §3.4) | — |
| 17 | Edit Text tool (`edit_text_tool_button`) | ✎ Aa | **REUSE** `icon-edit.svg` (+ keep the "Aa" text suffix, §5.3) | `edit_text_tool_tooltip()` |
| 18 | Add Text tool (`add_text_tool_button`) | + Aa | **NEW** — see §3.1 "text-cursor-plus" (+ keep "Aa" suffix) | `add_text_tool_tooltip()` |
| 19 | Undo (`undo_button`) | ↶ | **NEW** — see §3.1 "history-arrow" | `undo_tooltip_for(...)` |
| 20 | Redo (`redo_button`) | ↷ | **NEW** — mirror of #19 | `redo_tooltip_for(...)` |
| 21 | Copy ▾ (`copy_text_button`) | 📋 ▾ | **NEW** — see §3.1 "copy" | `copy_text_tooltip()` |
| 22 | Tools (`tools_button`) | 🧰 + text | **REUSE** `icon-tool.svg` | `tools_tooltip()` |
| 23 | Shortcuts (`shortcuts_button`) | ⌨ | **NEW** — see §3.1 "keyboard" | `shortcuts_tooltip()` |
| 24 | Combine files… (Tools dock) | none | ⚠️ **REUSE** `icon-link.svg` | (dock row, plain text today) |
| 25 | Split this document… (Tools dock) | none | **REUSE** `icon-scissors.svg` | — |
| 26 | Insert pages from a file… (Tools dock) | none | ⚠️ **REUSE** `icon-upload.svg` | — |
| 27 | Font folders… (Tools dock) | none | ⚠️ **REUSE** `icon-folder.svg` (same file as #1, different context — see §3.5) | — |

### 2.1 The one real collision risk, named explicitly

**#25 (Split → scissors) vs. a future Redaction icon (§8.1).**
Redaction is the highest-stakes feature in the app (R35/R52/R58) and
must never share a glyph with an ordinary, low-stakes structural
operation like Split — an operator scanning the toolbar for "the
redact tool" must never land on Split by icon-similarity. §8.1 gives
Redaction a deliberately different, non-scissors icon for exactly this
reason. This is the only ❌-grade risk found in the full audit;
everything else marked ⚠️ is a same-icon reuse across contexts that are
never on screen together (Open's toolbar button vs. Font Folders' dock
row; a `Tools ▾` dock entry is icon+text, so a repeated icon there
degrades gracefully to "text disambiguates," unlike two icon-only
toolbar buttons which have no such fallback).

---

## 3. New icons to draw — precise construction (same 48×48 / `currentColor` / stroke-2.5 / round-cap-join contract as §1)

Every construction below is described precisely enough to draw
directly as an SVG without further design judgment calls — matching
the level of specificity `icon-lock.svg`/`icon-search.svg` etc.
already demonstrate (a handful of primitive shapes, no gradients, no
fills except the one deliberate exception in §8.1).

### 3.1 Toolbar icons

- **"save"** (#2) — a rounded rect body (`x=8,y=8,w=32,h=32,rx=2`,
  matching `icon-document`'s corner radius) with a small notch cut from
  the top-right corner (an 8×8 diagonal cut, mirroring `icon-pdf`'s
  page-fold triangle at the SAME corner) — this reuses the
  fold-corner motif already established for "this is a file/page"
  (`icon-pdf`, `icon-document`) so Save visually belongs to the same
  document-object family as Open/Properties, while a single horizontal
  bar at 2/3 height (`y=26`) reads as a save-slot/label distinct from
  either. Do NOT reuse `icon-download.svg` for Save — reserved for
  genuine export/import actions (§8, Insert-pages already claims
  `icon-upload`; symmetrically reserve `icon-download` for a future
  Export-data/Extract-pages action) so the download/upload pair stays
  meaningful as an "in/out of this document" pair rather than being
  spent on Save.
- **"sidebar"** (#3, rail toggle) — a rounded outer rect (`x=6,y=8,
  w=36,h=32,rx=2`, matching `icon-window`'s frame) with ONE vertical
  divider line at `x=18` running the full inner height — the standard
  "sidebar" glyph (a window split into a narrow left pane + wide right
  pane). Distinct from `icon-window`'s own horizontal title-bar divider
  (used nowhere in pdfce currently, kept free for a future "app window"
  concept if ever needed).
- **"comment-bubble"** (#4, annotations toggle) — a rounded-rect speech
  balloon (`x=8,y=9,w=32,h=22,rx=8`) with a small triangular tail at
  the bottom-left corner (matching `icon-filter`'s funnel-taper
  angularity for the tail point) and two short horizontal lines inside
  (like `icon-document`'s text lines, but only two, shorter, since the
  balloon is smaller) to read as "text inside a comment," not an empty
  balloon.
- **"chevron"** (#5/#6, prev/next page) — a single `‹`/`›` chevron:
  two line segments meeting at a point, `M30,12 L18,24 L30,36` for
  prev (mirror `M18,12 L30,24 L18,36` for next), stroke-only, no fill,
  round caps/joins — the simplest possible construction in this style,
  deliberately smaller/lighter than a full arrow so it reads as
  "step," not "jump" (contrast with rotate/undo's full curved arrows).
- **"magnifier±"** (#7/#8, zoom out/in) — `icon-search`'s exact circle
  (`cx=21,cy=21,r=13`) and handle (`M30,30 L39,39`), with a horizontal
  bar (`M15,21 L27,21`) added inside the circle for zoom-OUT, and a
  plus cross (the same horizontal bar plus a vertical
  `M21,15 L21,27`) for zoom-IN. This is the one place this spec
  explicitly recommends EXTENDING an existing file rather than
  drawing from scratch — the magnifier is the correct, immediately
  legible metaphor and `icon-search.svg`'s geometry is reused verbatim
  as the base.
- **"frame-fit"** (#9, Fit page) — four independent L-shaped corner
  brackets (top-left `M10,16 L10,10 L16,10`, and the mirrored/rotated
  copies for the other three corners), the universal "fit within
  frame" glyph used by every camera/photo-viewer app — nothing in the
  ScripTree set has this shape, but it is a trivial, unambiguous
  4-stroke construction.
- **"frame-fit-width"** (#10, Fit width) — the SAME left/right corner
  brackets as "frame-fit" (top-left/bottom-left and top-right/
  bottom-right pairs only — 4 short strokes, not 8), plus a horizontal
  double-headed arrow between them at mid-height
  (`M14,24 L34,24` with small arrowheads at both ends) — deliberately
  related to "frame-fit" (shared corner-bracket family) but visually
  distinct via the added arrow, so the two read as siblings, not
  near-duplicates an operator might swap by mistake.
- **"rotate-page"** (#12/#13) — a small rounded-square "page"
  (`x=17,y=17,w=14,h=14,rx=2`, matching `icon-chip`'s inner-square
  proportions) with a ~270° curved arrow wrapping around its
  top-right quadrant, arrowhead pointing counter-clockwise for #12 /
  clockwise for #13 (mirror). The page-square is the load-bearing
  element that distinguishes this from Undo/Redo's bare arrows (§2.1
  discipline extended here) — an operator must never confuse "rotate
  THIS PAGE, saved with the document" with "step through edit
  history," and Rotate's own tooltip already carries this exact
  warning in text (`rotate_left_tooltip()`), so the icon should
  reinforce it, not blur it.
- **"shapes"** (#15, Markup ▾) — a square outline (`x=10,y=10,w=18,
  h=18,rx=1`) overlapping a circle outline (`cx=30,cy=30,r=11`),
  offset so they overlap by roughly a third — the standard
  "shape tools" glyph (draws directly on the square-plus-circle
  vocabulary the individual menu items in §3.3 already use
  separately, so the menu BUTTON icon visually previews what the menu
  CONTAINS).
- **"note"** (#16, Text ▾) — a rounded rect (`x=9,y=8,w=30,h=32,rx=2`)
  with a folded top-right corner LIKE `icon-pdf`'s fold but smaller/
  squarer (an 8×8 fold vs. `icon-pdf`'s ~10×10), and three short
  horizontal lines inside — visually a smaller, squarer sibling of
  "save" (§3.1 above) and `icon-document`/`icon-pdf`, on purpose: Text
  ▾ authors NEW text content, so it belongs to the same
  document/page-content family, while Markup's "shapes" icon (a
  totally different silhouette) correctly signals a completely
  different authoring concept (geometric, not textual).
- **"text-cursor-plus"** (#18, Add Text tool) — a vertical I-beam
  glyph (two short horizontal serifs at top and bottom joined by a
  vertical stroke, `x=20` column, `y=10..38`) with a small plus badge
  (`+`, ~10×10) at the top-right corner — pairs conceptually with
  `icon-edit`'s pencil (#17, Edit Text) as "the page-text-editing
  family" (per the existing code comment at `main.rs` ~3942 calling
  these "the page-text family") while remaining visually distinct: a
  pencil (modify existing marks) vs. a text-insertion cursor with a
  plus (create new marks) is a real, legible distinction, not an
  arbitrary one.
- **"history-arrow"** (#19/#20, Undo/Redo) — a single bare curved arrow
  (no page-square, per §2.1/"rotate-page" discussion above),
  counter-clockwise ~250° sweep for Undo (`M32,14 A16,16 0 1 0 32,34`
  with an arrowhead at the end) / mirrored clockwise for Redo.
  Deliberately the SIMPLEST possible construction in the whole set —
  Undo/Redo are used constantly and must read at a glance, so no
  extra element (no page, no document) should compete with the arrow
  shape itself.
- **"copy"** (#21, Copy ▾) — two overlapping rounded rects
  (`x=10,y=13,w=20,h=24,rx=2` behind, `x=18,y=8,w=20,h=24,rx=2` in
  front, both stroked, no fill) — the universal "duplicate/copy"
  glyph, offset diagonally exactly like a stack of two note-cards.
- **"keyboard"** (#23, Shortcuts) — a wide rounded rect
  (`x=6,y=14,w=36,h=20,rx=3`) containing a 4×2 grid of small squares
  (matching `icon-chip`'s pin-grid rhythm but coarser — 8 keys, not a
  chip's fine pin array) plus one wide bar along the bottom third
  representing the spacebar.

### 3.2 One deliberate non-icon: Actual size / 100%

**Recommend leaving `zoom_100_button()` as plain text ("100%"), not
iconified.** A numeral read at a glance ("I am at exactly true size")
is clearer than any glyph substitute could be (a magnifier-with-"1"
badge or a "1:1" pictograph both add a decode step a bare percentage
does not need) — this is a discoverability-checklist call (§1 of this
agent's own review discipline: "is the control labelled in plain
English," and here plain English already beats any icon), not a
refusal to comply with "icons for all features." Flagged explicitly so
the engineer does not feel obligated to force an icon here against the
better outcome — if overruled, the two-circle "1:1" unity glyph
sketched in §2 row 11 is the fallback, not a first choice.

### 3.3 Markup ▾ / Text ▾ sub-menu row icons (trivial primitives, listed together)

These are simple enough to specify in one block rather than one bullet
each:

| Row | Construction |
|---|---|
| Rectangle | a plain outlined square, `x=12,y=12,w=24,h=24,rx=1` |
| Ellipse | a plain outlined circle, `cx=24,cy=24,r=14` |
| Arrow line | a single diagonal line `M12,36 L36,12` with one open arrowhead at the `36,12` end |
| Highlight band | a wide flat rounded rect `x=8,y=19,w=32,h=10,rx=2` filled with a light diagonal HATCH pattern (thin parallel lines at 45°, stroke-width 1, not the standard 2.5 — a texture fill standing in for "translucent highlighter colour," since these are single-colour outline icons with no palette to show the operator's actual chosen highlight colour) |
| FreeText | a plain rect `x=10,y=10,w=28,h=28,rx=2` with THREE short horizontal lines inside (a generic "text box") |
| Sticky | a smaller square `x=13,y=13,w=22,h=22,rx=1` with a folded bottom-right corner (a literal sticky-note silhouette, folded corner distinguishes it from FreeText's plain rect) |

### 3.4 "stamp" — shared between Text ▾ → Stamp and future Bates numbering (§8.4)

A rounded rect "stamp head" (`x=14,y=8,w=20,h=16,rx=3`) sitting on a
narrower rectangular "base/handle" (`x=20,y=24,w=8,h=14,rx=1`), which
itself sits on a wide flat base line (`M12,40 L36,40`) — the classic
rubber-stamp silhouette. Deliberately reused for both the Text ▾ Stamp
annotation row (a single stamp placed once) and the future Bates
numbering feature (a stamp applied across a batch) — both are
genuinely the same "mark applied with a stamp" concept, invoked from
two different feature areas that are never simultaneously visible
(one is a canvas-authoring menu row, the other would be a Tools-dock
batch-operation entry), so this is a ⚠️-grade reuse, not a collision.

### 3.5 Font Folders reusing `icon-folder` (#27) — confirmed intentional, not an oversight

Font Folders (Tools dock) and Open (toolbar, #1) are never on screen
together in a way that would confuse an operator — Open is a top-level
toolbar action, Font Folders is three levels deep in a settings-shaped
dock panel, and the dock row carries its own text label
(`tool_font_folders_label()` = "Font folders…"). Reusing the plain
folder glyph is semantically correct in both places (Open = "pick a
file from a folder," Font Folders = "point me at folders of font
files") and cheaper than inventing a folder-with-a-tiny-"Aa"-badge
variant for a marginal, dock-only gain. Recorded explicitly per this
agent's own review discipline (flag every reuse as a conscious choice).

---

## 4. Toolbar layout, sizing, and widget wiring

### 4.1 Reuse `PdfceApp::icon_button`, do not build a parallel path

The existing wrapper (`main.rs` ~3633) already solves click-target
sizing (`ICON_BUTTON_SIZE = (28.0, 24.0)`) and the accessible-name
fix (P1-6). The image swap is: replace `egui::Button::new(glyph)`'s
text-button construction with an `egui::Button::image(texture)` (or
`egui::ImageButton`, whichever the chosen SVG-rasterization pipeline
in §7.1 hands back as a `TextureHandle`), sized to fit inside
`ICON_BUTTON_SIZE` with a small margin (recommend the rendered glyph
occupy roughly 18–20px of the 28×24 button, leaving a few px of
padding on every side — this keeps the CLICK TARGET larger than the
VISIBLE glyph, a real Fitts's-law/touch-accuracy win independent of
the icon swap itself). **`WidgetInfo::labeled`'s accessible-name
override must be kept verbatim** — an image button's default
accessible name would otherwise be empty/unhelpful, a real regression
risk if the wrapper is bypassed rather than extended.

### 4.2 Icon + text vs. icon-only — matches the EXISTING pattern, not a new rule

The current code already distinguishes these two cases and this spec
does not change the boundary:

- **Icon + short text** — Open, Save, Tools, Copy ▾, Markup ▾, Text ▾,
  Edit Text ("✎ Aa" → icon + "Aa"), Add Text ("+ Aa" → icon + "Aa"),
  the future Measure ▾. These are either infrequent-but-important
  (Open/Save/Tools), or a MENU/MODE control whose label carries real
  information (which tool is active — `measure_menu_button`'s dynamic
  label precedent from `pass-12.M2` §1.2 is the model to follow when
  Measure ▾ is built). **Keep the text.** An icon swap here means
  "icon replaces the emoji/bare-glyph prefix, text suffix stays," not
  "icon replaces the whole label."
- **Icon-only, tooltip-carried** — nav arrows, zoom in/out, rotate,
  undo/redo, rail toggle, annotations toggle, shortcuts. These are
  either used constantly (nav/zoom — a text label would be pure
  clutter at that frequency) or already fully explained by an
  always-visible neighbour (Undo/Redo's own tooltip already names the
  specific command, e.g. "Undo delete 3 pages" — a text label would
  duplicate that). **Icon-only stays icon-only**; the tooltip is the
  discoverability surface, per the existing convention.

### 4.3 Where Measure ▾ goes (not yet built — reserved slot)

Per `pass-12.M2` §1.2's own placement reasoning: appended to the
Edit group, immediately after Add Text, before the History
separator. This spec does not re-litigate that placement — it only
assigns the icon (`icon-ruler.svg`, §2 row "Measure ▾" — actually
listed at full depth in §8.2 since the tool itself is unbuilt).

---

## 5. States: hover / active / disabled / selected

### 5.1 Hover

Unchanged from egui's default `Button`/`selectable_label` hover
styling (a light background highlight) — nothing about the icon swap
should touch hover behaviour; `.on_hover_text(tooltip)` stays exactly
as it is on every control today.

### 5.2 Disabled

`add_enabled_ui(false, ...)` already fades every widget it wraps
(text buttons visibly grey out today — Undo/Redo, rotate, nav arrows
all use this). **The icon's tint colour must be read from
`ui.visuals().text_color()` INSIDE the disabled `Ui` scope, not
hoisted above it** — egui computes a faded/greyed variant for a
disabled context, and reading the colour after entering that scope is
what makes a tinted icon fade exactly the way the existing text
buttons already do, with zero new fade logic. This is a consistency
requirement (§6's tint mechanism is the delivery vehicle), not a new
mechanism.

### 5.3 Selected/active toggle — the one real regression risk

**Rule 6 (colour is never the sole signal) is currently satisfied by
bold text on a background-highlighted `selectable_label`
(`Self::toggle_label`).** An icon has no text to bold. Two controls are
affected today: the Annotations-visibility toggle (icon-only) and
Edit Text / Add Text (icon + "Aa" text — the "Aa" CAN still bold, so
these two are lower-risk). **Binding recommendation:** for icon-only
toggles (Annotations-visibility; the future Measure ▾ sub-tools once
built), pair the existing background-fill selected state with a
visible stroke/outline ring drawn around the icon button when
selected (egui already paints a selection background on
`Button::selectable`; add an explicit `egui::Stroke` border in the
selected branch, using the widget's own accent colour so it reads as
"outlined/framed," a SHAPE cue, not only a colour-fill cue). For
icon+text toggles (Edit Text, Add Text), keep bolding the "Aa" text
exactly as today — no change needed, the existing cue already
survives the icon swap because the text half of the label is
untouched.

---

## 6. Theming — mask + tint, not baked colour variants

Every source SVG is `stroke="currentColor"` — a single-colour outline
with no fixed palette. Whatever rasterization pipeline is chosen
(§7.1), the THEMING strategy should be:

1. Rasterize each icon ONCE as a **white-on-transparent alpha mask**
   (or any single opaque colour on transparent — the colour value
   itself is discarded, only the alpha channel matters).
2. At draw time, apply `.tint(colour)` where `colour` is sourced from
   `ui.visuals().text_color()` (the same colour egui already uses for
   ordinary button text/glyphs in the current theme) for the normal
   state, `ui.visuals().text_color()` read inside a disabled scope for
   the disabled state (§5.2), and the selected/accent colour for the
   selected-outline treatment (§5.3).

This means **ONE raster asset per icon serves light mode, dark mode,
disabled, and normal** — no light-variant/dark-variant PNG pairs to
keep in sync, and no risk of an icon looking hardcoded-black on a dark
background or hardcoded-white on a light one (a real, common icon-set
bug this design avoids structurally). This recommendation holds
regardless of which half of §7.1's fork the engineer picks — a
build-time rasterizer and a runtime `resvg` loader can both be told to
emit a mask instead of a coloured raster.

---

## 7. Flagged for the engineer / KenAgent — NOT decided here

### 7.1 SVG-in-egui rendering pipeline — an engineering fork, not a UX decision

egui/eframe does not render SVG natively. Two real options, with
different UX consequences worth naming (the choice itself is the
engineer's/KenAgent's, per this agent's own scope boundary):

- **(a) Pre-rasterize at build time, embed as PNG/`include_bytes!`.**
  **No new Cargo dependency.** Fixed resolution baked in — an icon
  rasterized for a 28×24 button at the default DPI will look soft or
  pixelated at a much higher display scale factor (e.g. a 200% Windows
  scale setting, or a future larger-icon mode) unless multiple
  resolutions are baked and picked at runtime. Simpler, zero licensing
  surface.
- **(b) A runtime SVG-rasterizing crate (e.g. `resvg`/`usvg`).**
  Renders crisp at ANY DPI/zoom, including a future "larger toolbar
  icons" accessibility option. **Potential NEW dependency** — `resvg`
  is MPL-2.0 (weak-copyleft; per rule 13 this is a real classification
  question, not automatically fine just because pdfce is now MIT) and
  per project rule 13 **any new dependency, copyleft or not, is
  flagged to the operator, never decided solo** by an agent. This
  needs a `docs/PRIOR_ART.md` check and an explicit go/no-go before
  it enters `Cargo.toml`.

This spec's icon designs (§3) work under EITHER pipeline unchanged —
nothing here assumes a specific rasterizer.

### 7.2 Icon provenance/licensing — confirm, don't assume

`D:\Dev\ScripTree\icons\*.svg` are the operator's own project assets,
and every file inspected for this spec carries a `<!-- Generic … —
placeholder, not a vendor trademark logo -->` comment suggesting they
were authored generically rather than derived from a third-party icon
pack. **This is likely a non-issue since Ken owns both projects, but
it should be confirmed, not assumed**, per rule 13's asset-licensing
discipline (`LEGAL.md` §5's spirit, applied to icon art rather than
test-corpus PDFs) — specifically: were these SVGs drawn from scratch
for ScripTree, or adapted from an existing icon font/set (Feather,
Lucide, Font Awesome, etc.) whose own license would then travel with
them into pdfce's asset tree? Confirm before bundling. Recommend
copying the chosen/adapted SVGs into a new `crates/pdfce-gui/assets/
icons/` tree (or wherever the engineer's chosen pipeline expects
source assets) with a short `PROVENANCE.md` noting "sourced from
D:\Dev\ScripTree\icons, [confirmed/date] as [original art by Ken /
derived from X under license Y]."

---

## 8. Backlog / not-yet-built features — icon reservations (lighter depth, by design)

These features have no shipped GUI surface yet (§0's audit). This
section reserves an icon assignment now so the icon LANGUAGE is
coherent when each feature ships, without re-deriving the mapping
later — it is deliberately NOT a full interaction spec (that is each
feature's own future `pdfce-ui-specialist` dispatch, same discipline
as `pass-7-form-fill.md`/`pass-8-redaction.md`/`pass-12.M2-dimension-
tools.md`).

| Feature | Icon decision | Note |
|---|---|---|
| 8.1 Redaction mark/apply (canvas tool, deferred) | **NEW, deliberately SOLID-FILLED** — a solid black rounded-bar (`x=10,y=20,w=28,h=8,rx=1`, `fill="currentColor"` — the ONE icon in the whole set permitted a fill) over a faint outlined rect suggesting covered text behind it | **Rule-based exception, not a style drift:** every other icon in this set is a pure outline; redaction's icon is the one place a SOLID mark is the honest depiction, because that is what redaction actually leaves — an outline-only icon would visually understate what the feature does. Must NOT be `icon-scissors` (§2.1) or any other existing "cut" glyph — mark and apply share one icon; the mark-vs-apply distinction lives in the label/dialog/confirmation text (rule 2), not a second icon |
| 8.2 Measure ▾ (Pass 12.M2, in progress) | **REUSE** `icon-ruler.svg` (menu button); Linear = ruler itself; Radius/Diameter = **NEW** circle + one radius line + centre dot; Set Group Scale = **REUSE** `icon-convert.svg`; Manage Dimension Groups = **REUSE** `icon-ring.svg` | Full interaction spec already exists at `docs/ui_specs/pass-12.M2-dimension-tools.md` — this row only assigns icons for that spec's already-designed menu (§1.2 there) |
| 8.3 Search/Find (unbuilt) | **REUSE** `icon-search.svg` | Canonical/primary claim on this icon — recommend a real toolbar icon-only button (Ctrl+F), not buried in the Tools dock, since Find is a frequent view-scoped action per the five-way placement taxonomy |
| 8.4 Bates numbering / stamping (unbuilt) | **REUSE** the "stamp" icon from §3.4 | Shared with Text ▾ → Stamp, justified in §3.4 |
| 8.5 OCR (unbuilt) | **REUSE** `icon-image.svg` | A scanned page is fundamentally an image; paired with a text label in the Tools dock so exactness matters less there |
| 8.6 Encryption (unbuilt, Pass 5 queue-deferred) | **REUSE** `icon-lock.svg` (Encrypt/Set Password); **REUSE** `icon-key.svg` (permissions password / certificate security, a secondary control) | Two different concerns get two different icons on purpose |
| 8.7 Digital signatures (unbuilt) | **REUSE** `icon-shield.svg` (trust/validity display); **REUSE** `icon-pin.svg` (place a signature field — Acrobat's own convention is a drop-target pin, a capability fact not a copied GUI mechanic) | |
| 8.8 Sanitize / remove hidden information (R58-adjacent, unbuilt) | **REUSE** `icon-filter.svg` | "Filter out unwanted content" reads correctly |
| 8.9 Comparison (unbuilt) | **REUSE** `icon-chart.svg` | Paired with a text label in the Tools dock |
| 8.10 Portfolios / PDF Package (unbuilt) | **REUSE** `icon-package.svg` | Exact conceptual match |
| 8.11 PDF/A conformance (unbuilt) | **REUSE** `icon-pdf.svg` | Distinct from Properties' `icon-document` (generic page) — PDF/A is specifically about the PDF format, so the PDF-flavoured fold-corner icon is the correct one here |
| 8.12 Print & prepress / PDF-X (unbuilt, low priority) | **REUSE** `icon-printer.svg` | Exact conceptual match |
| 8.13 Accessibility / PDF-UA tooling (unbuilt) | **NEW** — a small rounded-rect "page" with a tag/ribbon shape attached to one edge (representing structure TAGS, the actual PDF/UA mechanism) | Deliberately NOT a generic disability pictogram (wheelchair/eye/ear glyphs) — a structure-tag icon is both more accurate to the mechanism and avoids any risk of a tokenistic/stereotyped symbol; matches this agent's standing accessibility discipline of naming real mechanisms rather than reaching for a generic icon |
| 8.14 Form-fill GUI (`pass-7-form-fill.md`, unbuilt P0) | **No dedicated toolbar icon, by design** | That spec's own decision is direct-click-no-mode (fields are clicked directly, no separate "form-fill tool" is entered) — assigning a toolbar icon here would contradict the already-designed interaction. Flatten action (a distinct, separate button once built) gets a **NEW** icon: a form-field rectangle with a small downward chevron pressing onto it (burn-in metaphor) |
| 8.15 Form-field creation/authoring (unbuilt, operator priority #4) | **Deferred** — no icon assigned yet; this feature's own future UI spec should assign it once its interaction shape (toolbar tool vs. dock panel) is designed, per this project's standing "UI-specialist review precedes canvas-UI build" convention |
| 8.16 Optimization / linearization (unbuilt) | **NEW** — two arrows pointing inward toward a small page rect (a "compress" glyph) | |

---

## 9. Discoverability / accessibility checklist run against this spec

- **Plain-English labels:** unchanged — every icon this spec adds sits
  beside its EXISTING tooltip string (cited by function name
  throughout §2), never replaces it. Icon-only controls keep their
  tooltip as the discoverability surface (§4.2); icon+text controls
  keep their text.
- **Tooltip explains WHEN, not just WHAT:** already true of every
  existing `*_tooltip()` string audited in §0/§2 (e.g. `rotate_left_
  tooltip()` names both the effect AND that it survives to the saved
  file) — this spec changes no tooltip copy, only the glyph beside it.
- **Keyboard shortcut on anything destructive:** already satisfied
  structurally — this spec adds no new commands, only images for
  existing ones; the one destructive feature this spec touches at all
  (Redaction, §8.1, still unbuilt) inherits its shortcut/confirmation
  design from `docs/ui_specs/pass-8-redaction.md` unchanged.
- **Visible current state:** unchanged — §5.3 is the one place this
  spec adds a NEW state cue (the selected-icon outline ring) rather
  than merely preserving an existing one, specifically because an
  icon swap would otherwise regress rule 6 for icon-only toggles.
- **Tab order / reading order:** unchanged — this spec touches no
  widget tree structure, only the image drawn inside widgets that
  already sit in the existing, audited Tab chain (`main.rs`'s own
  module-doc discussion of Tab order, §0 of this spec).
- **Known egui accessibility gap, inherited, not widened:** the
  existing standing note (`main.rs` ~line 191–208) that pdfce has not
  been tested with a screen reader and the canvas itself conveys
  nothing to assistive technology is UNCHANGED by an icon swap — an
  image button still publishes the same `WidgetInfo::labeled`
  accessible name as the text button it replaces (§4.1's binding
  requirement), so this spec neither closes nor widens that gap.

---

## 10. Open items for the librarian

1. **This spec's icon-set assignments are additive to, not a
   replacement for, the existing `ui_text.rs` catalog** — no tooltip
   string changes, only the glyph/image drawn beside each one. Worth a
   one-line pointer from the Icon-set Backlog/Next-up entry to this
   file, the same courtesy prior specs extended to their own governing
   decision records.
2. **§8.1's solid-fill exception for the Redaction icon** is a named,
   deliberate rule-based exception to the outline-only style contract
   (§1) — worth recording so a future icon audit does not "fix" it
   back to an outline by mistake, mistaking it for a style violation
   rather than a load-bearing honesty choice.
3. **The SVG-in-egui rendering pipeline (§7.1) and the icon-provenance
   confirmation (§7.2) are both still-open engineering/operator
   decisions**, not resolved by this spec — flag both to the operator
   alongside this deliverable, same as every other named engineering
   fork in this project's specs.
4. **§2.1's Split-vs-Redaction collision** is the one real ❌-grade
   finding in the whole audit — worth a one-line note in the Redaction
   Backlog/GUI-follow-up entry pointing back here, so whoever eventually
   builds the Redaction canvas tool does not reach for `icon-scissors`
   out of habit (it is the single most obvious wrong instinct for a
   "removal" feature) without reading this spec first.
