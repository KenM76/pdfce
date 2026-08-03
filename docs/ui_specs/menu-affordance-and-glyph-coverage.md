# UI Spec — Menu-affordance glyph fix + full `ui_text.rs` non-ASCII audit

> Authored by `pdfce-ui-specialist`, on dispatch from the engineer
> (defect: the `▾` disclosure marker on Markup/Text/Measure/Copy
> renders as tofu — confirmed by screenshot and by reading the code).
> This spec covers (1) the confirmed defect and its fix, (2) the
> screen-reader question the dispatch explicitly asked, (3) a full
> non-ASCII audit of `crates/pdfce-gui/src/ui_text.rs` classified
> safe/suspect/confirmed-broken, and (4) two related findings surfaced
> by that audit that are not about fonts at all — a wrong placeholder
> codepoint and an accessible-name regression at a second call site.
> This spec does not write Rust; §6 is the literal change-list.

Read for this spec: `crates/pdfce-gui/src/ui_text.rs` (full non-ASCII
grep), `crates/pdfce-gui/src/main.rs` (every call site that consumes
the affected strings, plus the `icon_button`/`glyph_button`/
`icon_text`/`labeled_icon_button` wrapper family at ~3900–4070),
`crates/pdfce-gui/src/icons.rs` (the shipped `Icon` enum, the
mask+tint pipeline, `ICON_PTS`), `crates/pdfce-gui/assets/icons/
chevron-left.svg` / `chevron-right.svg` (the construction contract to
extend), `crates/pdfce-gui/src/canvas.rs` (to trace where the
`SnapKind` glyphs in `ui_text.rs` are actually painted — they are
not, see §4.4), egui 0.35.0 source (`epaint_default_fonts`'s
`lib.rs`, `epaint`'s `text/fonts.rs` `FontDefinitions::default`, and
`egui`'s `widget_info.rs`/`lib.rs` `WidgetType` enum) to ground the
font-coverage and AccessKit-role claims in this spec in the actual
shipped font stack rather than assumption.

---

## 0. Correction to the dispatch's own evidence

The dispatch describes Copy's on-screen glyph as `⧉ □`. **There is no
`⧉` (U+29C9) anywhere in `ui_text.rs` or `main.rs`** — grepped, zero
matches. What the screenshot shows is `icons::Icon::Copy`'s shipped
SVG (two overlapping rounded rects, icon-set spec §3.1 "copy"), which
visually resembles the informal "overlapping squares" reading of `⧉`
closely enough that whoever described the screenshot named the wrong
codepoint for it. **The icon itself renders correctly** — it is real
SVG art through the shipped mask+tint pipeline (§6 of
`docs/ui_specs/icon-set-and-toolbar.md`), not a font glyph. The actual
defect beside it is worse than a stray tofu box: see §4.1.

---

## 1. The confirmed defect

`▾` (U+25BE BLACK DOWN-POINTING SMALL TRIANGLE) is not present in any
font in pdfce's font stack. Traced precisely, not assumed:

- pdfce-gui sets no custom fonts anywhere (`grep -r FontDefinitions
  crates/pdfce-gui/src` — zero matches). It runs egui's
  `default_fonts` feature verbatim.
- `epaint-0.35.0/src/text/fonts.rs::FontDefinitions::default()` wires
  `FontFamily::Proportional` (the family every `RichText`/label in
  `main.rs` uses) to the fallback chain **Ubuntu-Light →
  NotoEmoji-Regular → emoji-icon-font**, in that order.
- U+25BE is in the Geometric Shapes Unicode block. None of the three
  fonts in that chain is a general symbol font with Geometric Shapes
  coverage: Ubuntu-Light is a Latin text face, NotoEmoji-Regular is
  scoped to the Unicode emoji recommendation (U+25BE is *not* on the
  emoji list — only a handful of Geometric Shapes codepoints are,
  e.g. the emoji-variant "small blue diamond" family, and U+25BE is
  not among them), and `emoji-icon-font` is a curated, small set of
  glyphs egui's own demos use (gear, play/pause triangles, etc.), not
  full block coverage.
- Net: **no font in the chain draws U+25BE.** The screenshot evidence
  (four tofu boxes on Markup/Text/Measure/Copy) is exactly what that
  predicts, and is confirmed present in the pre-icon-shipped baseline
  too — this was never a working glyph, the icon-set Pass just made it
  visible next to real art instead of next to another emoji.

This is a **pre-existing defect that the icon-set Pass did not
introduce and did not fix**, because the icon-set spec's own §2 table
mapped `markup_menu_button()`/`text_menu_button()`/
`measure_menu_button()`/`copy_text_button()` to a **leading** icon
(the shape/note/ruler/copy glyph) and explicitly left their text
labels — including the trailing `▾` — untouched (icon-set spec §4.2:
"icon replaces the emoji/bare-glyph prefix, text suffix stays"). The
`▾` was never in scope for that Pass. It is in scope for this one.

---

## 2. The fix: a drawn chevron, not a substitute codepoint

### 2.1 Why not "just pick a different Unicode character"

Rejected as the primary fix, though checked, not assumed away:
substituting a codepoint is exactly the reasoning that produced this
bug in the first place — a plausible-looking character picked without
verifying font coverage. Any replacement candidate (`⌄` U+2304,
`﹀` U+FE40, `˅` U+02C5, or the "safe" `v`/`⌵`) would need the same
verification this spec just did for `▾`, and several of them are
*more* obscure blocks (Spacing Modifier Letters, Small Form
Variants) than Geometric Shapes, i.e. probably worse odds. Given the
icon pipeline already exists, shipped, and is proven (Pass "icon-set
and toolbar" — 35 SVGs live, mask+tint theming live, DPI-crisp), a
drawn icon is strictly more reliable than a second guess at font
coverage, and is the option this spec recommends.

### 2.2 Rejected: bundling a font

Weighed per the dispatch's instruction. A bundled symbol font (e.g.
a Font Awesome/Material Symbols subset) would need: (a) a new Cargo
dependency or embedded asset, (b) a license classification under
rule 13 (flagged to the operator, never decided solo — this alone
makes it slower than the icon-pipeline option, which needs no new
dependency), and (c) does not solve the DPI-crispness problem the
icon pipeline already solves for every other control. Not
recommended; no reason to prefer it over an asset the project already
has the infrastructure to draw.

### 2.3 Recommended fix: `chevron-down.svg`, drawn in the shipped icon contract

Construction (same 48×48 viewBox / `stroke="currentColor"` /
`stroke-width="2.5"` / round cap+join contract as every other icon,
verified against `chevron-left.svg`/`chevron-right.svg` directly):

```xml
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48" fill="none" aria-hidden="true">
  <!-- Generic chevron shape — generic, no trademark risk -->
  <!-- Disclosure marker for a menu button, replacing the un-renderable
       "▾" (U+25BE) text-glyph affordance (see this file's own spec).
       Same construction family as chevron-left.svg/chevron-right.svg
       (icon-set spec §3.1 "chevron"), rotated: vertex points DOWN
       instead of left/right, matching a native combo-box/dropdown
       arrow's meaning ("this control opens something below it"). -->
  <path d="M12 18L24 30l12-12" stroke="currentColor" stroke-width="2.5"
        stroke-linecap="round" stroke-linejoin="round"/>
</svg>
```

Geometry check against the existing pair: `chevron-left`'s vertex
`(18,24)` sits 6pt off-center with 12×12 arms at 45°; this construction's
vertex `(24,30)` and arms to `(12,18)`/`(36,18)` use the identical
offset-6/arm-12 relationship, rotated 90°. It is a mechanical sibling
of the shipped pair, not a new style judgment call.

**Add to `icons.rs`:** `Icon::ChevronDown` → `include_str!("../assets/
icons/chevron-down.svg")`, name `"chevron-down"`, appended to
`Icon::ALL`.

### 2.4 Size the chevron down from `ICON_PTS`, and say why

Every existing feature icon draws at `ICON_PTS = 16.0` logical pt.
Using that size for the trailing chevron too is the *zero-new-code*
option, but this spec recommends against it: a full 16pt chevron
reads as a second co-equal icon beside the leading feature icon,
which is wrong (it is a subordinate disclosure cue, exactly like a
native OS combo-box's small drop arrow, never drawn at icon size in
any toolkit this reviewer is aware of) — and, per the dispatch's own
flag, **toolbar width is not free now that the row wraps.** A full
16pt chevron adds roughly the same width as today's already-present
(if tofu) `▾` glyph run (glyph + its leading space) across four
buttons — call it width-neutral at best. A **10pt** chevron is
narrower than today's `▾` glyph run and visually reads as
subordinate. Recommend:

```rust
/// A small trailing "opens a menu" disclosure marker, drawn beside a
/// menu button's label — replaces the un-renderable "▾" (U+25BE)
/// text-glyph affordance (docs/ui_specs/menu-affordance-and-glyph-
/// coverage.md §2).
///
/// Deliberately smaller than a leading feature icon ([`ICON_PTS`] =
/// 16pt): this is a subordinate disclosure cue, not a co-equal
/// feature icon, and every pt matters now that the toolbar wraps
/// instead of clipping. Always [`IconWeight::Regular`] — a menu
/// button's active/selected state (Measure ▾ is the one menu that
/// has one) is already carried by the LEADING icon
/// (`toggle_image`) and the bolded label (`toggle_label`);
/// making the chevron ALSO bold would overload one glyph with two
/// meanings instead of reusing the cues that already exist.
pub const MENU_CHEVRON_PTS: f32 = 10.0;

pub fn menu_chevron(ui: &egui::Ui) -> egui::Image<'static> {
    image_tinted(ui, Icon::ChevronDown, IconWeight::Regular, ui.visuals().text_color())
        .fit_to_exact_size(egui::vec2(MENU_CHEVRON_PTS, MENU_CHEVRON_PTS))
}
```

This reuses `image_tinted` verbatim — tint is read from
`ui.visuals().text_color()` at call time, so a chevron drawn inside
`add_enabled_ui(false, …)` fades exactly like every other icon
already does (§5.2 of the icon-set spec), with no new disabled-state
logic. This is the one new `pub` surface this spec asks for; check it
against `D:\dev\rag\rust\rust-style-guide-and-api-guidelines.md`
before shipping per project rule 10 (it is a small, precedented
addition alongside `image`/`selected_image`/`toggle_image`, so this
should be a fast check, not a blocking one).

### 2.5 Widget composition — confirmed to work under egui 0.35's Atoms system

`Button::new` in egui 0.35 takes `impl IntoAtoms<'a>`, and
`epaint-0.35.0`'s `atoms.rs` generates `IntoAtoms` for tuples up to 6
elements (`all_the_atoms!(T0..T5)`), verified by reading the macro
invocations directly. The existing `(image, RichText)` 2-tuples
already in `main.rs` (Markup/Text/Copy) are one instance of this;
**a 3-tuple `(leading_image, RichText, trailing_image)` is not a new
pattern, just one more element of an already-general mechanism** —
confirmed working, not assumed.

---

## 3. The screen-reader question (dispatch item 2) — answered directly

**Today, none of these four menu buttons announce "menu" to a screen
reader, and the fix must not accidentally make that worse.**

Traced precisely:

- `Ui::menu_button` (egui `ui.rs` ~2787) delegates to
  `menu::MenuButton::new(atoms).ui(...)`. Grepped
  `egui-0.35.0/src/containers/menu.rs` for `WidgetInfo`/`WidgetType` —
  **zero matches.** It sets no distinct accessible role; whatever
  `WidgetInfo` gets published is the default one the underlying
  `Button` widget produces.
- `egui::WidgetType` (checked, `egui-0.35.0/src/lib.rs` ~623–670) has
  `Button`, `ComboBox`, `SelectableLabel`, etc., but **no
  "MenuButton"/"has-popup" variant.** egui has no ARIA-`haspopup`-
  equivalent role to lean on for these controls at all — a real,
  tracked AccessKit-integration gap (rule 6's mandate to name gaps
  explicitly, not paper over them), not something this spec's fix can
  close by itself.
- Given that, the word "menu" (or equivalent) can **only** reach a
  screen-reader user if it is literally present in the text AT reads
  as the accessible name. Checked all four tooltip strings —
  `markup_menu_tooltip()`, `text_menu_tooltip()`,
  `measure_menu_tooltip()`, `copy_text_tooltip()` — **none contains
  the word "menu."** So even today, a sighted mouse user learns "this
  opens a menu" only by seeing the (tofu) triangle or by clicking it;
  a screen-reader user learns it from neither the tooltip's wording
  nor (per `icon_text()`'s own doc comment, `main.rs` ~4031–4036,
  "No accessible-name override is needed... the visible text IS the
  name") from any override — they get whatever raw text is on the
  button.
- **This is where the naive fix would regress something.** Pulling
  `▾` out of the RichText string (to fix the tofu) and replacing it
  with a purely decorative image atom (`aria-hidden="true"` in the
  SVG, and images generally carry no accessible text of their own
  through this pipeline) means the accessible name becomes **just the
  bare word** ("Markup", "Text", "Copy") — actually *less* menu-like
  than today's broken-but-present `▾` character, which at least has a
  Unicode name some screen readers announce ("black down-pointing
  small triangle") that a sharp user might learn to associate with
  "opens something." Silently *removing* that, even a bad cue, while
  visually keeping (and improving) the sighted cue, is exactly the
  kind of "fixed for sighted users, worse for AT users" regression
  rule 6 exists to catch.

**Binding recommendation: yes, the accessible name must explicitly
state that the control opens a menu, independent of what is drawn.**
Add one shared catalog string and route these four buttons through a
new wrapper (mirroring, not duplicating, `labeled_icon_button`'s
existing `WidgetInfo::labeled` pattern — see §6.2):

```rust
/// Accessible-name suffix for a menu-button control whose "opens a
/// menu" affordance is otherwise carried only by a decorative chevron
/// image. egui's `WidgetType` has no "opens a submenu" role distinct
/// from a plain `Button` (verified against egui 0.35.0's WidgetType
/// enum, not assumed) — so this word must live in the text
/// assistive technology actually announces.
pub fn menu_button_accessible_suffix() -> &'static str {
    "opens a menu"
}
```

Composed as `"{visible_label}, {suffix}"` — e.g. "Markup, opens a
menu", "Copy, opens a menu", "Measure: Linear, opens a menu" (the
dynamic label already in `measure_menu_active_label` composes
cleanly into this). This is a **net accessibility improvement over
today's state**, not merely a non-regression — today's accessible
name (the raw `▾`-suffixed string) never said "menu" either.

---

## 4. Full non-ASCII audit of `ui_text.rs` — safe / suspect / confirmed

Every non-ASCII codepoint in the file was grepped and classified
against the font chain from §1 (`Ubuntu-Light → NotoEmoji-Regular →
emoji-icon-font`, Proportional family — the family every `RichText`/
`painter().text()` call in this file's blast radius uses unless a
call site explicitly requests `FontFamily::Monospace`, which none of
the ones below do). Classification is evidence-based where evidence
exists (a Unicode block or emoji-list fact I can state precisely) and
explicitly marked **unverified** where it is not — per the dispatch's
own instruction, guessing is refused here, not offered as false
confidence.

### 4.1 CONFIRMED BROKEN

| Glyph | Codepoint | Where | Fix |
|---|---|---|---|
| `▾` | U+25BE | `markup_menu_button`, `text_menu_button`, `measure_menu_button`, `measure_menu_active_label`, `copy_text_button` | §2/§6 |

**`copy_text_button()` is a second, worse defect riding on the same
line, not just a tofu instance of the first.** Read at `ui_text.rs:
1759–1767`: the function returns **`"▾"` alone** — there is no
"Copy" word in the visible label at all, tofu or otherwise. The
icon-set spec's own §4.2 classified "Copy ▾" as belonging to the
**"icon + short text"** bucket alongside Markup/Text/Edit Text (i.e.
the spec's own intent was `icon + "Copy" + ▾`), but the shipped
string never carries "Copy." Even with the chevron fixed, this
button today reads (to a sighted user) as [copy icon] + [tofu box],
and (to a screen reader, once the tofu is a decorative image) as a
control literally named nothing. **Fix: `copy_text_button()` must
return `"Copy"`**, restoring the spec's own intended text, in the
same edit that removes the trailing `▾` (§6.1).

### 4.2 HIGH SUSPICION — same Unicode block as the confirmed bug, unconverted

| Glyph(s) | Codepoint(s) | Where | Why suspect |
|---|---|---|---|
| `▲` `▼` | U+25B2, U+25BC | `move_selection_up_button()`, `move_selection_down_button()` (`ui_text.rs:1271,1276`) | Same **Geometric Shapes** block as the confirmed-broken `▾`. `icons.rs`'s own doc comment (`main.rs` ~3923–3925) already names these as "the handful of icon-only controls that have no assigned SVG yet... and so still draw a bare Unicode glyph" — the codebase itself flags this as unfinished, this review just elevates the priority given the proven precedent. |
| `✓` `✕` | U+2713, U+2715 | Accept/Reject buttons: `"✓ Accept"`, `"✕ Reject"`, `"✓ Accept reflow"`, `"✕ Reject reflow"`, `"✓ Add"`, `"✕ Cancel"` (Pass 14.3 §6.4, Pass 15.2 §7, Pass 16.2 §7.1/§7.3) | These are the **plain** Dingbats check/cross, U+2713/U+2715 — distinct from the emoji-representable **heavy** variants U+2714/U+2716 used elsewhere in the file (`✔`/`✖`, §4.3). Unicode's emoji-recommended list includes U+2714 and U+2716 but **not** U+2713/U+2715, which is exactly the signal that predicted the `▾` failure. **This is the single highest-priority item in this whole audit to verify** — these are the terminal Accept/Reject controls for three shipped features (in-place text edit, reflow, add-text), used on every single edit in those tools, not a once-per-session menu click. |

### 4.3 LOWER SUSPICION / LIKELY SAFE — emoji-recommended or common-punctuation codepoints

| Glyph(s) | Codepoint(s) | Why lower risk |
|---|---|---|
| `⚠` | U+26A0 WARNING SIGN | Emoji-recommended codepoint (the "warning" emoji); heavy prior use across many shipped passes (redaction disclosure, properties lossy-decode warning, reflow refusal) with no prior tofu report despite being one of the most visible glyphs in the app. |
| `✔` `✖` | U+2714, U+2716 | The **heavy/emoji** check-mark and multiplication-X — both are on Unicode's emoji-recommended list (distinct from the plain U+2713/U+2715 flagged in §4.2). `✔` specifically is proven live today: `selection_check_glyph()` is painted via `ui.painter().text(..., egui::FontId::proportional(13.0), ...)` at `main.rs:5397–5403` for the multi-select checkmark, an already-shipped (Pass 3.2) feature. |
| `—` em dash, `“` `”` curly quotes, `×` multiplication sign, `¶` pilcrow | U+2014, U+201C/201D, U+00D7, U+00B6 | General Punctuation / Latin-1 Supplement — near-universal in any general-purpose Latin sans font. `—` alone appears hundreds of times across the file; if it were broken essentially the entire app's prose would be visibly tofu, which has not been reported. Not flagged further. |

### 4.4 UNVERIFIED — flagged per the dispatch's instruction not to guess

| Glyph(s) | Codepoint(s) | Where | Status |
|---|---|---|---|
| `ⓘ` | U+24D8 CIRCLED LATIN SMALL LETTER I | `disclosure_bullet()`, reflow's `recognition_divergence_note()`, alignment-detected caption (rule-6 "colour never alone" pairing for disclosure strips) | Enclosed Alphanumerics block — not emoji-recommended, not common-punctuation. No basis in hand to call this safe OR broken; genuinely needs a look at the running app. |
| `→` | U+2192 RIGHTWARDS ARROW | `"Edit this text now →"` (add-text continuity link), scale-entry live preview `"→ scale = 1:100"` | Arrows block. Moderate confidence only — basic arrows are common in general sans fonts, but this file's own proof point (`▾`) is also "a plausible, simple shape" that turned out missing, so confidence is stated, not assumed. |
| `↑` `↓` | U+2191, U+2193 | `merge_move_up_tooltip()`/`merge_move_down_tooltip()`'s `"Alt+↑"`/`"Alt+↓"`, shortcuts text | Same Arrows-block caveat as `→`. |
| `≈` | U+2248 ALMOST EQUAL TO | Centerline-derived-from-filled-shape disclosure, `"long:short ≈ {ratio}:1"` | Mathematical Operators block — unverified. |
| `◼` `●` `⊕` `▲` `✕` `▤` `┄` `⊞` | U+25FC, U+25CF, U+2295, U+25B2, U+2715, U+25A4, U+2504, U+229E | `snap_indicator_glyph()` (`ui_text.rs:2761–2779`, one per `SnapKind`) | **Not currently a rendering risk — this function is dead code.** Traced the actual on-canvas snap markers to `canvas.rs::snap_marker_shapes()` (called from `main.rs:8448`), which draws **vector primitives via `painter.extend(...)`** — filled rects, circles, line segments — not text glyphs at all. The accompanying word label (`snap_indicator_label(kind)` → `"node"`, `"endpoint"`, plain ASCII) is what actually reaches the screen at `main.rs:8452`, painted beside the vector shape. `snap_indicator_glyph()` has **zero call sites** anywhere in the crate (grepped) — it compiles clean only because unused `pub fn`s are not flagged by the dead-code lint. Not a font-coverage bug; flagged in §5 as a code-hygiene item instead. |

### 4.5 A genuinely wrong codepoint, unrelated to font coverage

`info_field_lossy_tooltip()` (`ui_text.rs:1656–1661`) says undecodable
property-field bytes "are shown as **￼**" — that character is
**U+FFFC OBJECT REPLACEMENT CHARACTER**. The sibling warning three
functions away, `properties_lossy_warning()` (`ui_text.rs:537–541`),
correctly says "unreadable characters are shown as **"�"**" —
**U+FFFD REPLACEMENT CHARACTER**. These describe the *same*
phenomenon (an undecodable Info-dictionary string field) with **two
different placeholder codepoints**, and only one of them is right:
grepped `pdfce-core` for the actual decode path and confirmed the
project's own documented convention is U+FFFD throughout
(`edit.rs:848`, "Bytes with no known mapping appear as U+FFFD";
`edit.rs:2440/2449` construct it explicitly), and the GUI's own
fallback (`String::from_utf8_lossy`, Rust's standard library) also
produces U+FFFD, never U+FFFC. **`info_field_lossy_tooltip()`'s
"￼" should be "�"** — both to match what the operator will actually
see on screen (this is a *fuzzy-never-sneaky* accuracy point: the
tooltip is telling the operator what glyph to look for, and it is
currently telling them the wrong one) and because U+FFFC (Specials
block, rarely implemented) is plausibly *also* a tofu risk in its own
right, compounding the inaccuracy with a second, unrelated rendering
gamble. Fix in §6.4.

---

## 5. A second finding, unrelated to fonts: an accessible-name gap at a live call site

`move_selection_up_button()`/`move_selection_down_button()`
(`ui_text.rs:1271,1276`, `▲`/`▼`) are consumed at **two** call sites,
and only one of them is accessibility-safe:

- `main.rs:5241–5252` (thumbnail-rail reorder, Pass 3.2) — correctly
  wrapped through `Self::glyph_button(ui, glyph, tooltip)`, which
  applies `ICON_BUTTON_SIZE` sizing and the `WidgetInfo::labeled`
  accessible-name override (P1-6) sourced from the tooltip.
- `main.rs:2879,2889` (the Combine-files merge-list reorder controls,
  Tools dock) — uses **plain `egui::Button::new(ui_text::
  move_selection_up_button())`** directly, with only
  `.on_hover_text(...)` attached. No `WidgetInfo` override. A screen
  reader at this second call site announces whatever AT derives from
  the raw `▲`/`▼` text — not the tooltip word ("Move earlier"/"Move
  later" per `merge_move_up_tooltip()`/`merge_move_down_tooltip()`).

`glyph_button`'s own doc comment (`main.rs` ~3923–3936) says it
exists precisely for "the handful of icon-only controls that have no
assigned SVG yet (the rail's keyboard reorder arrows)" — strong
circumstantial evidence this wrapper was authored with exactly this
glyph pair in mind, and the second call site was simply never
updated to use it. This is a straightforward, low-risk fix (swap the
`egui::Button::new(...)` construction for `Self::glyph_button(ui,
ui_text::move_selection_up_button(), ui_text::
merge_move_up_tooltip())`, wrapped in the same `add_enabled_ui`
pattern already used elsewhere in the file for a conditionally-
enabled icon control) — see §6.5.

---

## 6. Change-list — concrete, for the engineer

### 6.1 `ui_text.rs` — strip the trailing chevron text, restore Copy's missing word

- `markup_menu_button()`: `"Markup ▾"` → `"Markup"`
- `text_menu_button()`: `"Text ▾"` → `"Text"`
- `measure_menu_button()`: `"Measure ▾"` → `"Measure"`
- `measure_menu_active_label(tool_name)`: `format!("Measure: {tool_name} ▾")` → `format!("Measure: {tool_name}")`
- `copy_text_button()`: `"▾"` → `"Copy"` (restores the missing word, not just the tofu fix — §4.1)
- Add `menu_button_accessible_suffix() -> &'static str { "opens a menu" }` (§3), doc-commented with the AccessKit-gap rationale already written out above so it survives context loss.

### 6.2 `icons.rs` — new asset + variant + sizing helper

- New file `crates/pdfce-gui/assets/icons/chevron-down.svg`, exact content in §2.3.
- `Icon::ChevronDown` variant, wired into `source()`/`name()`/`Icon::ALL` following the existing pattern for every other variant.
- New `pub const MENU_CHEVRON_PTS: f32 = 10.0;` and `pub fn menu_chevron(ui: &egui::Ui) -> egui::Image<'static>` per §2.4.

### 6.3 `main.rs` — wire the chevron atom + the menu accessible-name wrapper into the four call sites

- Markup ▾ (~4342–4377), Text ▾ (~4384–4419), Copy ▾ (~4620–4645):
  change the 2-tuple `(icons::image(ui, icon), RichText::new(label))`
  to the 3-tuple `(icons::image(ui, icon), RichText::new(label),
  icons::menu_chevron(ui))`.
- Measure ▾ (~4504–4524): same 3-tuple treatment, appending
  `icons::menu_chevron(ui)` after the existing `toggle_label(...)`
  atom. **Do not** make the chevron itself bold/toggle-tinted — §2.4
  names this explicitly as a deliberate non-choice (the leading icon
  and the label already carry the active-state cue; don't double it
  onto the chevron).
- Add a new wrapper alongside `labeled_icon_button` (do not duplicate
  its body — same "one accessible-name fix, two entry points"
  discipline the codebase already uses for `icon_button`/
  `glyph_button`):

  ```rust
  /// The same accessible-name-override pattern as
  /// `labeled_icon_button`, for a menu button whose "opens a menu"
  /// affordance is otherwise carried only by a decorative chevron
  /// (menu-affordance-and-glyph-coverage.md §3).
  fn labeled_menu_button(
      ui: &mut egui::Ui,
      response: egui::Response,
      visible_label: &str,
  ) -> egui::Response {
      let name = format!("{visible_label}, {}", ui_text::menu_button_accessible_suffix());
      let enabled = ui.is_enabled();
      response.widget_info(move || {
          egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, name.clone())
      });
      response
  }
  ```

  Call it at each of the four `ui.menu_button(...).response` sites
  with the plain label text (`"Markup"`, `"Text"`, the dynamic Measure
  label, `"Copy"`) — same pattern already used for `.on_hover_text(...)`
  chaining on these responses today, one more `.` call, not a
  restructure.

### 6.4 `ui_text.rs` — fix the wrong placeholder codepoint

- `info_field_lossy_tooltip()`: `"...are shown as ￼."` → `"...are shown as \u{201C}\u{FFFD}\u{201D}."`
  (U+FFFC → U+FFFD, matching `properties_lossy_warning()`'s wording
  and pdfce-core's own documented convention — §4.5).

### 6.5 `main.rs` — close the accessible-name gap at the merge-dialog reorder buttons

- Lines ~2876–2894: replace the two plain
  `egui::Button::new(ui_text::move_selection_up_button())`/
  `move_selection_down_button()` constructions with
  `Self::glyph_button(ui, ui_text::move_selection_up_button(),
  ui_text::merge_move_up_tooltip())` (and the down equivalent),
  preserving the existing `index > 0`/`index + 1 <
  self.merge_inputs.len()` enabled-state gating via
  `ui.add_enabled_ui(condition, |ui| { ... })` around the call
  (matching the pattern already used for Markup/Text/Measure's
  `add_enabled_ui` wrapping elsewhere in this same file).

### 6.6 Flagged, not decided here — verify before treating as closed

- **§4.2's `✓`/`✕` (U+2713/U+2715) on the Accept/Reject buttons** —
  highest-priority item to actually look at in the running app; these
  are the terminal control of three shipped editing tools.
- **§4.2's `▲`/`▼`** — if confirmed tofu, the fix is a `chevron-up.svg`
  (mirror of `chevron-down.svg`: vertex up, `"M12 30L24 18l12 12"`)
  plus converting both call sites (§5's `main.rs:5241–5252` and
  `main.rs:2879–2889`, the latter already being fixed for the
  accessible-name gap in §6.5 regardless of tofu status) to
  `Self::glyph_button` with an SVG image argument instead of a raw
  glyph string. Recommended even if `▲`/`▼` turn out to render fine,
  for visual-family consistency with the now-complete chevron set and
  because §6.5's accessible-name fix is needed at that second call
  site independent of the tofu question.
- **§4.4's `ⓘ`, `→`, `↑`/`↓`, `≈`** — unverified; no fix proposed
  because no defect is confirmed. Recommend a quick visual pass in
  the running app (these are cheap to eyeball — disclosure strips,
  the add-text continuity link, the scale-entry preview, the merge
  dialog's shortcut hints) rather than pre-emptively drawing SVGs for
  codepoints that may already be fine.
- **`snap_indicator_glyph()` (§4.4 last row)** — genuinely dead code,
  not a rendering defect. Worth a one-line question back to the
  engineer: delete it (nothing references it and the feature it would
  have served is already implemented via vector shapes), or is there
  a reason it's intentionally kept as a reserved/documented
  alternative? Not this reviewer's call to make unilaterally (code
  removal is the engineer's territory), but worth surfacing since it
  was found as a direct byproduct of the requested audit.

---

## 7. Checklist run against the standing rules

- **Discoverability** — the four menu buttons keep their existing
  tooltip as the "when to use it" surface (unchanged by this spec);
  the fix adds a plainer, smaller, DPI-crisp visual "this opens a
  menu" cue than the tofu it replaces, and for the first time gives
  Copy its missing word back.
- **Accessibility (rule 6)** — this is the spec's central finding:
  color/shape are not the concern here (no toggle state changes), but
  the *decorative-image-has-no-accessible-text* trap is real and
  named explicitly (§3), with a concrete, minimal fix (§6.1/§6.3) that
  makes the accessible name of these four controls **better than it
  is today**, not merely not-worse. The known egui/AccessKit gap
  (`WidgetType` has no menu-button role) is named per rule 6's mandate
  to track gaps rather than paper over them, not something this fix
  claims to close.
- **Immediate-mode fit** — no new persisted state, no `egui::Id`
  needed; `menu_chevron` is a stateless per-frame draw call exactly
  like every other `icons::image` call already in the file.
- **Toolbar-width discipline (rule 3)** — explicitly weighed in §2.4;
  the 10pt sizing choice is a direct response to the dispatch's own
  flag that width is not free now that the row wraps.
- **Fuzzy-never-sneaky** — not directly implicated (no algorithmic
  suggestion involved), except obliquely in §4.5: a tooltip that
  states the wrong placeholder glyph is a small instance of the same
  "say what's actually true" discipline rule 2/fuzzy-never-sneaky is
  built on, which is why it is filed as a fix rather than a
  nice-to-have.

---

## 8. Engineer verification log (added 2026-08-03)

Direct observation of the running release build, via the screenshot harness
(`tools/observe-gui.ps1`) at 3x magnification. This section records what was
actually SEEN, as distinct from what was inferred from font coverage — §4's
classifications were deliberately hedged, and these confirm or close them.

### 8.1 CONFIRMED BROKEN — `▾` U+25BE (§4.1) — now FIXED

Seen as a tofu box on all four menu buttons (`Markup`, `Text`, `Measure`,
`Copy`) before the fix, and confirmed GONE after it: all four now draw
`chevron-down.svg` and render correctly. Copy additionally regained its
missing "Copy" word (§4.1's second finding). Fixed in `a1badc1`.

### 8.2 CONFIRMED BROKEN — `▲` U+25B2 / `▼` U+25BC (§4.2) — NOT yet fixed

§4.2 rated these "HIGH SUSPICION — same Unicode block as the confirmed bug".
**They are broken.** With a page checked in the rail, the selection toolbar
renders `1 page selected` followed by two rotate icons and then **two empty
boxes** where "Move selection up" / "Move selection down" should be. Both
controls are therefore unlabelled to a sighted operator.

Note this is a *worse* failure than the menu-button case was: those at least
carried a word beside the tofu ("Markup ▾"), whereas these buttons are
glyph-ONLY, so a broken glyph leaves them with no visible identity at all.
They do carry accessible names (they route through `glyph_button`), so the
defect is visual rather than assistive.

**Fix, mirroring §2.3:** `chevron-up.svg` (authored 2026-08-03, mirror of
`chevron-down.svg`, same offset-6/arm-12 construction) and the existing
`chevron-down.svg`, via a new `Icon::ChevronUp` and the ordinary
`icons::image` path — these are primary controls, not subordinate disclosure
markers, so they take the full `ICON_PTS` size rather than
`MENU_CHEVRON_PTS`. Both call sites already funnel through `glyph_button`
(the merge-dialog pair was migrated in `a1badc1`), so only the glyph source
changes; the accessible names are already correct and must be preserved.

### 8.3 STILL UNVERIFIED — `✓` U+2713 / `✕` U+2715 (§4.2)

Not settled. The rail's page checkbox does render a tick, but that is egui's
own `Checkbox` widget painting a vector check — not a font glyph — so it says
nothing about U+2713. The Accept/Reject buttons that use these codepoints
require an in-progress tool gesture to appear, and synthetic canvas input does
not currently satisfy egui's `Response::clicked()`, so the state could not be
reached. Remains flagged, not claimed either way.

### 8.4 OBSERVED SAFE

`⚠` (status-bar warning prefix) and `✓` in the "Rendered faithfully" status
line both render correctly, consistent with §4.3's "likely safe" rating.
