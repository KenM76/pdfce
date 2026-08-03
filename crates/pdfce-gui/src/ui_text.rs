//! # ui_text — pdfce-gui's single user-facing string catalog
//!
//! **Every user-facing string in this crate lives here and nowhere
//! else** (decision record `docs/decisions/002-i18n-timing.md`,
//! standing rule R1; enforced by the `ui-strings` CI job, which fails
//! the build on any whitespace-containing string literal elsewhere in
//! the crate unless the line carries `// ui-text-exempt: <reason>`).
//!
//! ## Why entries are FUNCTIONS, never consts (do not "simplify" this)
//!
//! The whole point of this module is that a future localization
//! retrofit converts **one file and zero call sites** (002 §5.2, §6.4):
//! each function body becomes a catalog lookup while its signature —
//! and therefore every call site — stays untouched. A `pub const`
//! cannot make that move (a const can't become a runtime lookup), so
//! converting these to consts would silently destroy the property the
//! module exists for. `&'static str` returns stay retrofit-safe too:
//! a startup-fixed locale catalog interns into a `OnceLock` and leaks
//! once, which is idiomatic for startup-fixed configuration.
//!
//! ## Authoring rules (002 §6.1, binding)
//!
//! - **R2 — one entry = one complete message.** Never assemble a
//!   sentence from fragments or nest one catalog entry inside another;
//!   parameterize with named `format!` placeholders inline so a
//!   translation can reorder freely.
//! - **R3 — never size layout to the English text** (translations run
//!   30–40% longer; budget +50% where a fixed extent is unavoidable).
//! - **R6 — numbers/dates/sizes shown to the user get formatting
//!   helper functions here**, not inline `format!` at call sites.
//! - Every entry's doc comment says **where in the UI it appears** —
//!   context a translator would otherwise have to reverse-engineer.
//!
//! ## Retrofit recipe (002 §6.4, verbatim so it survives context loss)
//!
//! 1. Add the chosen crate (`rust-i18n` or `fluent-static`; LEGAL.md
//!    §6.2 check; `gettext-rs` is pre-disqualified — LGPL static link
//!    on Windows). 2. Extract these English strings into the catalog
//!    format keyed by function name. 3. Rewrite each body as a lookup —
//!    signatures and call sites unchanged. 4. Startup locale selection;
//!    intern via `OnceLock`. 5. Pseudo-locale (`en-XA`, +40% length)
//!    walk of the whole UI. 6. If `pdfce-web` exists, promote this
//!    module to a shared crate FIRST.

use std::path::Path;

use pdfce_core::PdfVersion;
use pdfce_core::dimension::Unit;
use pdfce_core::edit::{CommandKind, InfoField};
use pdfce_core::vector::{AxisConstraint, FillRule, PaintStyle, Rgb, SnapKind};

use crate::object_summary::{Degeneracy, ObjectKind, ObjectNote, ObjectSummary, SelectionCensus};

// ---------------------------------------------------------------------------
// Toolbar — file
// ---------------------------------------------------------------------------

/// Label of the toolbar's file-open button.
///
/// Words only. The emoji prefix this used to carry
/// ("📂") was replaced by a real SVG icon
/// ([`crate::icons::Icon::Open`]) drawn beside the label. R1 is
/// unaffected — the user-visible string still lives here; only the
/// glyph moved out of it, into the asset tree.
pub fn open_button() -> &'static str {
    "Open…"
}

/// Tooltip on the file-open button.
pub fn open_tooltip() -> &'static str {
    "Open a PDF for viewing (Ctrl+O)"
}

/// Toolbar status before any file has been opened this session.
pub fn status_idle() -> &'static str {
    "No document open"
}

/// Toolbar status after a document loads: file name, declared version,
/// page count, and whether there are unsaved edits. One complete phrase
/// per R2 — a translation may reorder the facts freely.
///
/// The unsaved-changes marker is part of *this* entry rather than a
/// separate label so a translation can place it wherever its
/// conventions put it (leading asterisk, trailing word, a bracketed
/// note) instead of being forced to the position English happens to
/// use.
///
/// `modified` comes from the same save-time diff a save would compute,
/// so the indicator cannot disagree with what saving actually writes —
/// in particular an edit that was undone reports *no* unsaved changes.
pub fn status_open(path: &Path, version: PdfVersion, page_count: usize, modified: bool) -> String {
    if modified {
        format!(
            "{} — PDF {version}, {page_count} page(s) — unsaved changes",
            file_name(path)
        )
    } else {
        format!("{} — PDF {version}, {page_count} page(s)", file_name(path))
    }
}

/// Status-bar disclosure that a document carries UNAPPLIED redaction marks
/// (Pass 8, ISO 32000-1 §12.5.6.23). Persistent and loud: a marked
/// document is **not** redacted until apply removes the content, and the #1
/// real-world redaction failure is shipping a file believing otherwise.
pub fn redaction_marks_pending(count: usize) -> String {
    format!(
        "⚠ {count} UNAPPLIED redaction mark(s) — this document is NOT redacted; \
         its marked content is still present until you apply the redactions"
    )
}

/// Toolbar status after a failed open (damaged file, not a PDF,
/// unreadable). Distinct from [`status_unsupported`]: this one means
/// something is wrong with the *file*.
pub fn status_failed(path: &Path) -> String {
    format!("Could not open {}", file_name(path))
}

/// Toolbar status when the file was refused for a known, named
/// capability gap rather than damage. Deliberately NOT routed through
/// [`status_failed`], which reads as "your file is broken" — see
/// [`canvas_unsupported`] for the full reasoning.
pub fn status_unsupported(path: &Path) -> String {
    format!("Not supported yet: {}", file_name(path))
}

// ---------------------------------------------------------------------------
// Window / taskbar title (P0-3)
// ---------------------------------------------------------------------------

/// Window/taskbar title while no document is open, or after a failed or
/// unsupported open. A failed open does **not** rename the window after
/// the file that failed — only a document that actually opened earns a
/// place in the window chrome, matching the convention every other editor
/// uses.
pub fn window_title_idle() -> &'static str {
    "pdfce"
}

/// Window/taskbar title once a document is open. `modified` marks unsaved
/// edits with a plain leading asterisk on the file name — the same
/// convention most editors use, so it needs no legend. One complete entry
/// per R2 so a translation may reorder the name, marker and app name
/// however its conventions require.
pub fn window_title_open(path: &Path, modified: bool) -> String {
    if modified {
        format!("{}* — pdfce", file_name(path))
    } else {
        format!("{} — pdfce", file_name(path))
    }
}

/// Label of the toolbar's save button.
///
/// "Save a copy" rather than "Save", and the wording is load-bearing:
/// pdfce always writes to a path the operator picks and never
/// overwrites the open file unasked (non-destructive by default). A
/// button labelled "Save" that opens a save-as dialog is a small lie
/// that trains operators to distrust the label.
pub fn save_button() -> &'static str {
    "Save a copy…"
}

/// Tooltip on the save button. Names the shortcut, the fact that a new
/// file is written, and the incremental-update behaviour — all three
/// are things the operator would otherwise have to discover by trying.
pub fn save_tooltip() -> &'static str {
    "Write the document, including unsaved edits, to a file you choose (Ctrl+S). The original \
is never overwritten unless you pick it, and the edits are appended as an update so the \
previous version stays intact inside the file."
}

/// Default file name offered in the save dialog: the original's name
/// with a suffix, so the dialog never opens pre-aimed at overwriting
/// the file the operator has open.
pub fn suggested_save_name(path: &Path) -> String {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "document".to_owned());
    format!("{stem} (edited).pdf")
}

/// Status-bar line after a successful save. `objects` is how many
/// object definitions the appended revision carries and `appended` is
/// how many bytes it added — both are the operator's evidence that the
/// save was minimal rather than a wholesale rewrite.
pub fn save_succeeded(path: &Path, objects: usize, appended: usize) -> String {
    format!(
        "✔  Saved to {} — {objects} object(s) updated, {appended} byte(s) appended; \
everything else in the file is untouched.",
        file_name(path)
    )
}

/// Status-bar note when a save had to move objects out of a compressed
/// object stream to write them (ISO 32000-1 §7.5.7).
///
/// Disclosed rather than hidden: it is a structural change to the file
/// that the operator did not ask for, even though the alternative
/// (rewriting the whole container) would disturb far more.
pub fn save_promoted_objects(count: usize) -> String {
    format!(
        "Note: {count} edited object(s) were stored compressed inside the file and had to be \
rewritten in an uncompressed form to be changed. Their earlier values remain in the file, as \
they do for any edit saved this way."
    )
}

/// Status-bar line after a save that failed. The technical detail is
/// included because the operator's next step is usually to report it.
pub fn save_failed(message: &str) -> String {
    format!("✖  The document was not saved. Technical detail: {message}")
}

// ---------------------------------------------------------------------------
// Toolbar — page navigation
// ---------------------------------------------------------------------------

// NOTE (icon Pass): the bare-glyph label functions that used to live in
// this module — `prev_page_button` ("◀"), `next_page_button`,
// `zoom_out_button`, `zoom_in_button`, `rotate_left_button`,
// `rotate_right_button`, `undo_button`, `redo_button`,
// `rail_toggle_button`, `annotations_toggle_button` and
// `shortcuts_button` — were REMOVED, not merely unused. Those controls
// are now icon-only ([`crate::icons::Icon`]), so those strings had no
// user-visible surface left; keeping them would have implied a fallback
// that does not exist. R1 is untouched: every string a user can still
// SEE, including every tooltip below (which is also the accessible name
// an icon-only control publishes), is still defined here.

/// Tooltip on the "previous page" button. Names the keyboard shortcut,
/// because an icon-only control that never reveals its shortcut trains
/// nobody.
pub fn prev_page_tooltip() -> &'static str {
    "Go to the previous page (PageUp; Home for the first page)"
}

/// Tooltip on the "next page" button.
pub fn next_page_tooltip() -> &'static str {
    "Go to the next page (PageDown; End for the last page)"
}

/// Toolbar page-position readout between the navigation arrows.
/// `current` is 1-based, as shown to the operator.
pub fn page_nav_label(current: usize, total: usize) -> String {
    format!("Page {current} of {total}")
}

// ---------------------------------------------------------------------------
// Toolbar — zoom
// ---------------------------------------------------------------------------

/// Tooltip on the "zoom out" button.
pub fn zoom_out_tooltip() -> &'static str {
    "Zoom out (Ctrl+Minus, or Ctrl+scroll wheel)"
}

/// Tooltip on the "zoom in" button.
pub fn zoom_in_tooltip() -> &'static str {
    "Zoom in (Ctrl+Plus, or Ctrl+scroll wheel)"
}

/// Label of the "fit whole page" zoom button.
pub fn fit_page_button() -> &'static str {
    "Fit page"
}

/// Tooltip on the "fit whole page" button.
pub fn fit_page_tooltip() -> &'static str {
    "Scale the page so all of it is visible, and keep it fitted as the window resizes"
}

/// Label of the "fit page width" zoom button.
pub fn fit_width_button() -> &'static str {
    "Fit width"
}

/// Tooltip on the "fit page width" button.
pub fn fit_width_tooltip() -> &'static str {
    "Scale the page so its full width is visible, and keep it fitted as the window resizes"
}

/// Label of the "actual size" zoom button.
pub fn zoom_100_button() -> &'static str {
    "100%"
}

/// Tooltip on the "actual size" button. Says what 100% *means* in a
/// document viewer, which is not self-evident.
pub fn zoom_100_tooltip() -> &'static str {
    "Show the page at actual size — one PDF point per screen point (Ctrl+0)"
}

/// Zoom-percentage readout in the toolbar (R6 — the numeric formatting
/// lives here, never as an inline `format!` at the call site).
pub fn zoom_percent_label(percent: u32) -> String {
    format!("{percent}%")
}

// ---------------------------------------------------------------------------
// Toolbar — editing (Pass 3.1)
// ---------------------------------------------------------------------------

/// Tooltip on the rotate-left button.
///
/// Says *page*, and says *saved with the document*, because the same
/// glyph in a viewer usually means a view-only turn that is forgotten
/// on close. Confusing the two is how an operator ends up with a file
/// they did not mean to change — or, worse, believes they changed one
/// when they did not.
pub fn rotate_left_tooltip() -> &'static str {
    "Turn this page 90° counter-clockwise. This changes the document, not just the view, and \
is saved with it — use Undo to reverse it."
}

/// Tooltip on the rotate-right button.
pub fn rotate_right_tooltip() -> &'static str {
    "Turn this page 90° clockwise. This changes the document, not just the view, and is saved \
with it — use Undo to reverse it."
}

/// Label of the toolbar's document-properties toggle.
pub fn properties_button() -> &'static str {
    "Properties"
}

/// Tooltip on the document-properties toggle.
pub fn properties_tooltip() -> &'static str {
    "Show or hide the document's title, author, subject and keywords"
}

// -- Pass 6.1 markup authoring (minimal affordance) -------------------

/// Label of the toolbar's markup-authoring menu button.
pub fn markup_menu_button() -> &'static str {
    "Markup"
}

/// Tooltip on the markup menu button.
pub fn markup_menu_tooltip() -> &'static str {
    "Add a shape or highlight to the current page. This changes the document and is saved with \
it — use Undo to reverse it."
}

/// A one-line hint at the top of the markup menu.
pub fn markup_menu_hint() -> &'static str {
    "Adds the shape at the centre of the current page. Drawing-on-canvas tools are coming."
}

/// Label beside the markup colour picker.
pub fn markup_color_label() -> &'static str {
    "Colour:"
}

/// Markup menu item: add a square.
pub fn markup_square_item() -> &'static str {
    "Rectangle"
}

/// Markup menu item: add a circle/ellipse.
pub fn markup_circle_item() -> &'static str {
    "Ellipse"
}

/// Markup menu item: add a line.
pub fn markup_line_item() -> &'static str {
    "Arrow line"
}

/// Markup menu item: add a highlight.
pub fn markup_highlight_item() -> &'static str {
    "Highlight band"
}

/// Edit-note confirming a markup annotation was authored. `label` is the
/// item's own display name (already localised via the `markup_*_item`
/// functions), so this composes them rather than duplicating the names.
pub fn markup_added(label: &str) -> String {
    format!("Added: {label}. Use Undo to reverse this until you save.")
}

// -- Pass 6.2 text-bearing annotations (minimal affordance) -----------

/// Label of the toolbar's text-annotation menu button.
pub fn text_menu_button() -> &'static str {
    "Text"
}

/// Tooltip on the text menu button.
///
/// Pass 16.2 §1.1/§10 disambiguation (R78, bidirectional): now that a distinct
/// "Add Text" tool authors REAL page content, this annotation menu's tooltip
/// names the difference so an operator reaching for "add a text box to the
/// page" is pointed at the right control — a removable annotation here vs.
/// permanent page text there.
pub fn text_menu_tooltip() -> &'static str {
    "Add a text box, sticky note, or stamp to the current page. This is a removable annotation, \
not page content — for text that becomes a real, permanent part of the page itself (like the \
text already on it), use Add Text instead. This changes the document and is saved with it — use \
Undo to reverse it."
}

/// A one-line hint at the top of the text menu.
///
/// Reworded (P1-3a): the operator does not choose *where* the text goes —
/// there is no placement control — they choose which **kind** it is, and
/// it is always centred automatically. The old "choose where it goes"
/// read as if a placement control existed.
pub fn text_menu_hint() -> &'static str {
    "Type the text, then pick a kind below. It is placed at the centre of the current page; \
click-to-place editing on the canvas is coming."
}

/// Note under the Text menu's colour picker, disclosing the one exception
/// (P1-3b): the colour applies to a text box only — sticky notes and
/// stamps intentionally keep their own standard colours, which is what
/// makes them recognisable.
pub fn text_menu_color_note() -> &'static str {
    "Applies to the text box only — sticky notes and stamps use their own standard colours."
}

/// Label above the text-entry field.
pub fn text_input_label() -> &'static str {
    "Text:"
}

/// Text menu item: add a FreeText box.
pub fn text_freetext_item() -> &'static str {
    "Text box"
}

/// Text menu item: add a sticky note.
pub fn text_sticky_item() -> &'static str {
    "Sticky note"
}

/// Text menu item: add a Draft stamp.
pub fn text_stamp_item() -> &'static str {
    "Draft stamp"
}

/// The "Add" confirmation button in the text-entry popup.
pub fn text_add_button() -> &'static str {
    "Add to page"
}

/// The "Cancel" button in the text-entry popup.
pub fn text_cancel_button() -> &'static str {
    "Cancel"
}

/// A note shown when text authoring is refused (e.g. an unusable font).
pub fn text_add_failed(reason: &str) -> String {
    format!("Could not add the text: {reason}")
}

// ---------------------------------------------------------------------------
// Document-properties panel (ISO 32000-1 §14.3.3)
// ---------------------------------------------------------------------------

// REMOVED — `properties_window_title()` ("Document properties").
//
// It titled the floating `egui::Window` that decision 017 §8.3 / A.4 #2
// retired: the document-properties form is now a dock panel, and a dock
// panel's name is its TAB label — `dock_panel_properties_label()`. Deleting
// the entry rather than leaving it unused is deliberate, following the Pass
// 18.3 precedent for surfaceless catalog entries: a `ui_text.rs` function
// with no call site is a string nobody can find in the running app, and the
// catalog's value is that reading it tells you what the UI says.
//
// The ui-spec's §A.2 rename of this string to "Document Properties" is moot
// for the same reason: it existed to defuse a collision with a *selection*
// inspector that §A's superseded design would have introduced beside it. No
// such panel ships, and no second surface competes for the word.

/// Row label for one editable metadata field.
///
/// Takes the core enum rather than a string so the panel cannot show a
/// field the engine does not know how to write, and so adding a field
/// is a compile error here rather than a silently missing label.
pub fn info_field_label(field: InfoField) -> &'static str {
    match field {
        InfoField::Title => "Title",
        InfoField::Author => "Author",
        InfoField::Subject => "Subject",
        InfoField::Keywords => "Keywords",
        // `InfoField` is #[non_exhaustive]: a field added in a later
        // Pass must get a real label here, and until it does it shows
        // its own placeholder rather than silently reading as one of
        // the fields above.
        _ => "(unnamed field)",
    }
}

/// Placeholder shown in an empty metadata field, which also states what
/// leaving it empty *does* — clearing an entry and never having had one
/// are the same result, and neither is obvious from a blank box.
pub fn info_field_hint() -> &'static str {
    "not set — leave empty to remove"
}

/// Explanatory line under the properties fields.
pub fn properties_help() -> &'static str {
    "Changes are applied to the document as a single undoable edit, and are written to disk \
only when you save."
}

/// Label of the properties panel's apply button.
pub fn properties_apply_button() -> &'static str {
    "Apply"
}

/// Tooltip on the apply button.
pub fn properties_apply_tooltip() -> &'static str {
    "Apply these values to the document as one undoable change. Nothing is written to disk \
until you save."
}

/// Label of the properties panel's revert button.
pub fn properties_revert_button() -> &'static str {
    "Revert"
}

/// Tooltip on the revert button.
pub fn properties_revert_tooltip() -> &'static str {
    "Discard what you have typed here and show the document's current values again"
}

/// Tooltip on the Revert button when the draft already matches the
/// document — the disabled-state counterpart to
/// [`properties_apply_unchanged_tooltip`] (P1-5). Without it, the disabled
/// Revert sat beside a disabled Apply that *did* explain itself, which
/// reads as unfinished.
pub fn properties_revert_unchanged_tooltip() -> &'static str {
    "Nothing to revert — these values already match the document."
}

/// Warning shown when at least one metadata value could not be decoded
/// with certainty.
///
/// This is the *fuzzy, never sneaky* rule applied to text decoding:
/// pdfce could guess at the unknown bytes and be quietly wrong, or say
/// so. It says so, and names the consequence the operator actually
/// faces — that applying would replace the original bytes with the
/// approximation on screen.
pub fn properties_lossy_warning() -> &'static str {
    "⚠  Some of these values use text encoding pdfce cannot decode with certainty; the \
unreadable characters are shown as “�”. Fields you do not change are left exactly as they are \
in the file. Applying a field you have edited replaces its stored value with what you see here."
}

// ---------------------------------------------------------------------------
// Thumbnail rail
// ---------------------------------------------------------------------------

/// Tooltip on the thumbnail-rail toggle.
pub fn rail_toggle_tooltip() -> &'static str {
    "Show or hide the page thumbnail rail"
}

// ---------------------------------------------------------------------------
// Annotation-visibility toggle (Pass 6.0, ISO 32000-1 §12.5)
// ---------------------------------------------------------------------------

/// Tooltip when annotations are currently SHOWN. States the current state
/// (not just the action), because on a lightly-annotated page toggling it
/// off may produce no visible change — the operator needs the control
/// itself to confirm what it did (the ui-specialist's Rule-6 note).
pub fn annotations_toggle_tooltip_shown() -> &'static str {
    "Annotations are shown. Click to hide markup, stamps, and form-field appearances and view \
the page content alone."
}

/// Tooltip when annotations are currently HIDDEN.
pub fn annotations_toggle_tooltip_hidden() -> &'static str {
    "Annotations are hidden. Click to show the markup, stamps, and form-field appearances stored \
in this document."
}

/// Caption under each thumbnail, and the text shown inside a
/// placeholder for a page that has not been drawn yet. `number` is
/// 1-based.
pub fn thumbnail_page_number(number: usize) -> String {
    format!("{number}")
}

// ---------------------------------------------------------------------------
// Status bar — render diagnostics (decision 004 §6.4, rule R20)
// ---------------------------------------------------------------------------

// --- Pass 6.0 annotation disclosure (ISO 32000-1 §12.5) ----------------

/// Status line when the annotation-visibility toggle is OFF but the page
/// carries annotations — so a suppressed view never silently hides that
/// there is markup to see (R50/R27). One templated sentence (R2).
pub fn annotations_display_off(count: usize) -> String {
    format!(
        "Annotation display is off — this page has {count} annotation(s) that are not being \
shown. Use the annotation toggle in the toolbar to show them."
    )
}

/// Informational status line: how many annotations this page carries and
/// how many pdfce painted, plus the form-field share. Shown when
/// annotations are visible and the page has any.
pub fn annotations_painted_summary(total: usize, painted: usize, widgets: usize) -> String {
    format!("{total} annotation(s) on this page ({painted} painted, {widgets} form field(s)).")
}

/// Status line for annotations pdfce could NOT paint because they have no
/// usable appearance stream — R43's named-not-painted case. pdfce never
/// synthesises a look, so these are disclosed rather than invented.
///
/// The per-subtype breakdown is assembled **here** (not at the call site)
/// so the separators (`, `, `×`) live in the catalog with the prose — R2:
/// one entry is one complete message, list separators included, so a
/// translation can reorder or re-punctuate freely.
pub fn annotations_no_appearance(count: usize, subtypes: &[(String, usize)]) -> String {
    let list: Vec<String> = subtypes
        .iter()
        .map(|(subtype, n)| format!("{subtype} ×{n}"))
        .collect();
    format!(
        "{count} annotation(s) have no appearance stream pdfce can paint, so nothing was drawn \
for them (pdfce never invents a look): {}.",
        list.join(", ")
    )
}

/// Status line for annotations whose appearance state (`/AS`) could not be
/// resolved — displayed as nothing, never guessed (§12.5.5 NOTE 3).
pub fn annotations_state_missing(count: usize) -> String {
    format!(
        "{count} annotation(s) have an appearance state that could not be resolved; pdfce \
displayed nothing for them rather than guess which state to show."
    )
}

/// Status line for annotations whose appearance could not be placed
/// (missing /Rect or /BBox, or a degenerate transformed box). Refused by
/// name rather than mis-placed (risk X2).
pub fn annotations_degenerate(count: usize) -> String {
    format!(
        "{count} annotation(s) carry an appearance that could not be positioned on the page and \
were left unpainted rather than drawn in the wrong place."
    )
}

/// Status line for annotations suppressed on screen by the Hidden or
/// NoView flag. Honoured AND disclosed (R50) — hidden content the operator
/// cannot see is a fact they are entitled to know.
pub fn annotations_hidden(count: usize) -> String {
    format!(
        "{count} annotation(s) are hidden on screen by their own flags (Hidden or NoView). pdfce \
honours that and does not paint them, but discloses that they are there."
    )
}

/// Status line disclosing that the document's form asserts its field
/// appearances are stale (`/NeedAppearances`). pdfce reports it and never
/// silently regenerates on load (R51).
pub fn annotations_need_appearances() -> &'static str {
    "This document's form marks its field appearances as needing regeneration \
(/NeedAppearances). pdfce shows the appearances stored in the file as-is and does not silently \
rewrite them."
}

/// Status-bar diagnostics line when the current page rendered with no
/// substitutions and no unsupported content.
///
/// Shown *every* time, never omitted: an affordance that is sometimes
/// present and sometimes absent forces the operator to interpret its
/// absence, which is the opposite of the disclosure R20 asks for.
pub fn diagnostics_clean() -> &'static str {
    "✔  Rendered faithfully — no font substitutions or unsupported content on this page"
}

/// Expanded detail for the clean case — states what was actually
/// checked, so "clean" is verifiable rather than merely asserted.
pub fn diagnostics_clean_detail() -> &'static str {
    "Every glyph on this page was drawn from the document's own embedded font program, and \
every content operator on it is one pdfce implements."
}

/// Status-bar one-line summary when the current page's diagnostics are
/// not empty. One complete templated sentence per R2 — never assembled
/// from separately formatted counts.
///
/// The two substitute trust levels are named DISTINCTLY (decision 012
/// R62): `bundled_glyphs` are pdfce's own guessed shapes, `supplied_glyphs`
/// are drawn from an operator-supplied face. Folding them into one
/// "substituted" count is exactly the conflation the three-trust-levels
/// model replaced, so the headline keeps them apart.
pub fn diagnostics_summary(
    bundled_glyphs: usize,
    supplied_glyphs: usize,
    unsupported_items: usize,
) -> String {
    format!(
        "⚠  This page: {bundled_glyphs} bundled substitute glyph(s), {supplied_glyphs} \
operator-supplied glyph(s), {unsupported_items} unsupported item(s) — expand for detail"
    )
}

/// Heading of the diagnostics detail expander.
pub fn diagnostics_detail_heading() -> &'static str {
    "What pdfce could not draw faithfully on this page"
}

/// Detail line naming the substituted faces, and saying why the page is
/// still trustworthy for layout: the shapes are pdfce's, but every
/// glyph position came from the document's own width tables
/// (decision 004 §6.4).
pub fn diagnostics_substituted_fonts(count: usize, names: &str) -> String {
    format!(
        "{count} glyph(s) were drawn with bundled substitute faces because the document does \
not embed these fonts: {names}. Positions and spacing still come from the document's own \
width tables, so the layout is right even though the letterforms are not the original ones."
    )
}

/// Detail line naming the operator-supplied faces used on this page
/// (decision 012 R62), independent of the bundled-substitution line: it
/// restates the shapes-not-layout caveat on its own so an operator who
/// supplied a font does not infer the layout became authoritative, and
/// discloses that such a render is machine-dependent (R63).
pub fn diagnostics_supplied_fonts(count: usize, names: &str) -> String {
    format!(
        "{count} glyph(s) were drawn with fonts you supplied (via Font folders) because the \
document does not embed them: {names}. These are your own typefaces, not pdfce's substitutes — \
but positions and spacing still come from the document's own width tables, so a supplied font \
improves the letterforms, not the layout. Because the result depends on the fonts you supplied, \
it may differ on another machine."
    )
}

/// Detail line for glyphs that could not be matched to any font at all.
pub fn diagnostics_glyphs_notdef(count: usize) -> String {
    format!(
        "{count} glyph(s) could not be matched to any glyph in any available font, and were \
left blank rather than guessed at."
    )
}

/// Detail line for fonts whose machinery this build does not implement.
/// The "skipped, not approximated" phrasing is load-bearing: it is the
/// difference between text pdfce chose not to draw and text pdfce drew
/// wrongly.
///
/// `by_reason` is `Diagnostics::fonts_unsupported_by_reason`; when it is
/// populated the line names the actual reasons (R20 / honest
/// disclosure), so an operator can tell "supply the font" apart from
/// "this build defers that font kind" apart from "the embedded program
/// is broken" without reading a log.
pub fn diagnostics_fonts_unsupported(
    count: usize,
    by_reason: &std::collections::BTreeMap<&'static str, usize>,
) -> String {
    let mut line = format!(
        "{count} font(s) use machinery this version of pdfce does not implement. Their text \
was skipped entirely — never approximated with something that looks close."
    );
    let detail: Vec<String> = [
        (
            "CompositeNotEmbedded",
            "no embedded font program (supply the font)",
        ),
        (
            "UnusableProgram",
            "an embedded program pdfce could not parse",
        ),
        (
            "NonIdentityCmap",
            "a non-Identity character map (not yet supported)",
        ),
        ("VerticalWriting", "vertical writing (not yet supported)"),
        ("Type3", "Type 3 drawn-glyph fonts (not yet supported)"),
        ("UnknownSubtype", "an unrecognized font subtype"),
    ]
    .into_iter()
    .filter_map(|(key, label)| {
        let n = by_reason.get(key).copied().unwrap_or(0);
        (n > 0).then(|| format!("{n} with {label}"))
    })
    .collect();
    if !detail.is_empty() {
        line.push_str(" Breakdown: ");
        line.push_str(&detail.join("; "));
        line.push('.');
    }
    line
}

/// Detail line for content operators pdfce recognizes but does not yet
/// paint (shadings, marked content, …).
pub fn diagnostics_deferred_ops(count: usize, names: &str) -> String {
    format!(
        "{count} page element(s) that this version does not draw yet — such as gradient \
shadings — were skipped. Operators involved: {names}."
    )
}

/// Detail line for images pdfce could not decode at all.
///
/// Same "missing, not approximated" framing as
/// [`diagnostics_fonts_unsupported`], and for the same reason: an
/// operator needs to know whether what they are looking at is the whole
/// page. `notes` names the specific reason (usually a codec such as
/// JPEG or JPEG 2000) so the answer to "what would fix this?" is on
/// screen rather than a support question.
pub fn diagnostics_images_unsupported(count: usize, notes: &str) -> String {
    format!(
        "{count} image(s) on this page use a compression format or colour space this version \
of pdfce cannot read, so they are missing from the page entirely — nothing was drawn in \
their place. Details: {notes}."
    )
}

/// Detail line for images that WERE drawn but not exactly as specified
/// (an ignored soft mask, a short palette, truncated sample data).
pub fn diagnostics_image_divergences(notes: &str) -> String {
    format!(
        "Some images were drawn but not exactly as the document specifies — for example, \
transparency masks are not applied yet, so an image that should be partly see-through is \
drawn solid. Details: {notes}."
    )
}

/// Detail line for images refused because their **compression format**
/// is one this build has no decoder for.
///
/// Split out from [`diagnostics_images_unsupported`] because the answer
/// to "what would fix this?" is different: a missing codec is a pdfce
/// roadmap item, not something the operator can work around by editing
/// the file. `notes` names the format, because "JPEG 2000" is the word
/// the operator will search for.
pub fn diagnostics_codec_unsupported(count: usize, notes: &str) -> String {
    format!(
        "{count} image(s) use a compression format pdfce cannot read yet — support for it is \
planned but not in this version. Nothing was drawn in their place. Formats involved: {notes}."
    )
}

/// Detail line for images refused because a *variant* of an otherwise
/// supported format is missing (arithmetic-coded JPEG, 12-bit JPEG, …).
///
/// The distinction matters to an operator: pdfce *can* read JPEG, so
/// "the JPEGs in this file are unusual" is a different and much rarer
/// situation than "pdfce has no JPEG decoder", and re-saving the file
/// from its original application usually fixes it.
pub fn diagnostics_codec_feature_unsupported(count: usize, features: &str) -> String {
    format!(
        "{count} image(s) use an unusual variant of a format pdfce otherwise reads, and were not \
drawn. Re-saving the file from the application that produced it usually converts them to a \
common variant. Variants involved: {features}."
    )
}

/// Detail line for images whose embedded data disagrees with the page's
/// own description of them.
pub fn diagnostics_codec_geometry_mismatch(count: usize) -> String {
    format!(
        "{count} image(s) describe themselves differently inside their compressed data than the \
page does — a fault in the program that produced the file. pdfce used the page's size to place \
them and the image's own size to read the pixels, which is usually right, but they may look \
stretched or cropped."
    )
}

/// Detail line for four-component (CMYK) JPEGs — the benign census.
///
/// Pure volume, phrased neutrally on purpose: decision 006 verified
/// that this shape (YCCK storage, the only kind seen in practice)
/// decodes without any polarity ambiguity, pixel-matching the Chrome
/// PDF engine. The pre-006 wording told the operator to check for
/// inverted colours and "report the file" — retired, because it cried
/// wolf on known-good images. The shape that still deserves a warning
/// has its own entry, [`diagnostics_dct_cmyk_unverifiable`].
pub fn diagnostics_dct_cmyk(count: usize) -> String {
    format!("{count} image(s) use four-colour (CMYK) JPEG data and were drawn normally.")
}

/// Detail line for the one CMYK-JPEG shape whose colour polarity is
/// genuinely unverifiable (decision 006, rule R30).
///
/// This is a real warning, unlike the census above: a four-component
/// JPEG that carries neither a colour-transform declaration nor a
/// `/Decode` array may have been written with an undocumented
/// inverted-colour convention, and nothing in the file says which.
/// pdfce draws the stored values as-is — the same choice every major
/// PDF viewer makes — and *tells* the operator, per the
/// fuzzy-never-sneaky rule: any future repair is a reviewable
/// per-image toggle, never a silent auto-fix.
pub fn diagnostics_dct_cmyk_unverifiable(count: usize) -> String {
    format!(
        "{count} image(s) use a rare form of four-colour (CMYK) JPEG whose colours cannot be \
verified from the file alone. If they look like a photographic negative, the file relies on \
an undocumented convention pdfce deliberately does not guess at (decision 006). Other PDF \
viewers show the same thing. Please report the file."
    )
}

/// Detail line for JPEG 2000 images that arrived preblended with a
/// backdrop (`/SMaskInData 2`, ISO 32000-1 Table 89).
///
/// A genuine, bounded shortfall rather than a census: the image's
/// colour channels have a background colour mixed into them, and
/// separating it out again needs the transparency model pdfce has not
/// built yet. What is drawn *is* the picture composited over that
/// background — right where the image is opaque, showing the background
/// where it is not — so the wording says what the operator will
/// actually see rather than naming the spec entry.
///
/// Deliberately free of `/SMaskInData`, `Matte` and "premultiplied":
/// this panel is read by someone asking "is what I am looking at
/// correct?", and the answer is "mostly, and here is exactly where
/// not".
pub fn diagnostics_jpx_preblended(count: usize) -> String {
    format!(
        "{count} JPEG 2000 image(s) were stored already blended with a background colour, \
together with the transparency information needed to separate the two again. pdfce drew the \
blended version: correct wherever the image is solid, and showing that background wherever it \
was meant to be see-through. Full transparency support is still to come."
    )
}

/// Detail line for LZW streams framed non-conformantly.
pub fn diagnostics_lzw_framing(count: usize) -> String {
    format!(
        "{count} compressed stream(s) were not framed the way the PDF standard requires. pdfce \
read them anyway; the file's producer is at fault, not the file's contents."
    )
}

/// Detail line for form XObjects refused by the nesting/cycle guard.
pub fn diagnostics_xobject_overflows(count: usize) -> String {
    format!(
        "{count} reusable page element(s) were nested too deeply, or referred back to \
themselves in a loop, and were not drawn. This protects pdfce from a malformed or hostile \
file; the page you are seeing is incomplete."
    )
}

/// Detail line for operators pdfce does not recognize at all.
pub fn diagnostics_unknown_ops(count: usize) -> String {
    format!(
        "{count} content operator(s) were not recognized at all and were skipped. This usually \
means the file uses a private or malformed extension."
    )
}

/// Detail line for structural oddities the interpreter tolerated rather
/// than abandoning the page over.
pub fn diagnostics_tolerated(count: usize) -> String {
    format!(
        "{count} structural oddity(ies) in the page's content were tolerated and worked around \
rather than treated as fatal."
    )
}

/// Status-bar line while no document is open.
pub fn diagnostics_no_document() -> &'static str {
    "No page rendered yet"
}

// ---------------------------------------------------------------------------
// Canvas
// ---------------------------------------------------------------------------

/// Heading shown above the empty-canvas hint, before any file is open
/// (P0-5). Plain app name only — no tagline, no "open source"/release
/// claim (the project's licence is still undecided, CLAUDE.md rule 8).
pub fn empty_state_heading() -> &'static str {
    "pdfce"
}

/// Centered hint on the empty canvas when no document is open.
pub fn canvas_idle_hint() -> &'static str {
    "Open a PDF to view its pages.\n\npdfce reads the file and never writes to it \
unless you explicitly save."
}

/// Second line of the empty-canvas hint, naming the drop affordance
/// (P0-5). A separate entry from [`canvas_idle_hint`] (R2: one entry, one
/// complete thought) rather than folding a third sentence into it.
pub fn canvas_idle_drop_hint() -> &'static str {
    "Or drop a PDF file here to open it."
}

/// Centered canvas error after a failed open. Says plainly what this
/// class of failure means — the *file* is damaged or is not a PDF —
/// keeping the engine's own diagnostic as a secondary technical line.
///
/// `message` is `pdfce-core`'s English `Display` output (002 R4: core
/// errors are stable diagnostics; this entry is the *presentation*
/// wrapper a locale could translate around it).
pub fn canvas_failed(path: &Path, message: &str) -> String {
    format!(
        "pdfce could not open this file:\n{}\n\nThis usually means the file is damaged, \
truncated, or is not a PDF at all.\n\nTechnical detail: {message}",
        path.display()
    )
}

/// Centered canvas message when the file was refused for a *named
/// capability gap* — currently cross-reference streams (ISO 32000-1
/// §7.5.8) and hybrid-reference files (§7.5.8.4), both of which
/// `pdfce-core` detects deliberately and declines rather than
/// misparsing.
///
/// The wording leads with reassurance that the document is fine,
/// because the operator's first question on any refusal is "did I just
/// lose my file?" and answering it late is answering it too late. The
/// spec citation stays a secondary technical line: it is exactly what a
/// bug report needs and exactly what a non-technical reader should not
/// have to parse first.
pub fn canvas_unsupported(path: &Path, message: &str) -> String {
    format!(
        "pdfce cannot open this file yet:\n{}\n\nThe document uses a cross-reference \
structure this version of pdfce does not read yet. Your file is almost certainly fine — \
this is a gap in pdfce, not damage to the document, and pdfce stopped rather than risk \
misreading it.\n\nTechnical detail: {message}",
        path.display()
    )
}

/// Centered canvas message when the document loaded but a page failed
/// to rasterize (a content-stream decode failure, or a raster size past
/// the guard). The document stays open — only this page is unavailable.
pub fn canvas_render_failed(page_number: usize, message: &str) -> String {
    format!(
        "Page {page_number} could not be drawn.\n\nThe document is still open — try another \
page, or a lower zoom level.\n\nTechnical detail: {message}"
    )
}

/// Centered canvas message for a document whose page tree is empty.
pub fn canvas_no_pages() -> &'static str {
    "This document contains no pages."
}

// ---------------------------------------------------------------------------
// Native dialogs
// ---------------------------------------------------------------------------

/// File-type filter label in the native Open dialog (shown by the OS
/// file picker next to the `*.pdf` pattern). Easy to miss as a UI
/// string because it is not at an egui call site.
pub fn open_dialog_filter_label() -> &'static str {
    "PDF documents"
}

// ---------------------------------------------------------------------------
// Formatting helpers (R6)
// ---------------------------------------------------------------------------

/// Best-effort display name for a file: its final component, falling
/// back to the full path. Lossy conversion is acceptable — this is
/// display text, never a path the code reopens.
pub fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// Join a list of names (font names, operator names) for inclusion in a
/// diagnostics line.
///
/// R6 puts this here rather than at the call site because the separator
/// is a locale convention, not a code detail — several locales do not
/// use `", "`, and a future catalog needs one place to change it.
pub fn join_names(names: &[String]) -> String {
    names.join(", ")
}

// ---------------------------------------------------------------------------
// Toolbar — Tools dock toggle (Pass 3.2)
// ---------------------------------------------------------------------------

/// Label of the toolbar's Tools-dock toggle — the single control the
/// whole of Pass 3.2 adds to the toolbar.
pub fn tools_button() -> &'static str {
    "Tools"
}

/// Tooltip on the Tools toggle. Names what is inside, because a toggle
/// whose label is a generic noun is otherwise a mystery box.
pub fn tools_tooltip() -> &'static str {
    "Show or hide the Tools panel: combine several PDFs, split this one, \
or insert pages from another file. Tools that act on pages you can \
already see — delete, reorder, rotate, extract — live on the page \
thumbnails instead."
}

// ---------------------------------------------------------------------------
// Thumbnail rail — multi-selection (Pass 3.2)
// ---------------------------------------------------------------------------

/// Heading of the rail's selection action bar.
pub fn selection_bar_summary(count: usize) -> String {
    if count == 1 {
        "1 page selected".to_owned()
    } else {
        format!("{count} pages selected")
    }
}

/// Label of the clear-selection button.
pub fn selection_clear_button() -> &'static str {
    "Clear"
}

/// Tooltip on the clear-selection button; names the accelerator.
pub fn selection_clear_tooltip() -> &'static str {
    "Deselect every page (Esc)"
}

/// Tooltip on the batch rotate-left control.
pub fn batch_rotate_left_tooltip(count: usize) -> String {
    if count == 1 {
        "Turn the selected page a quarter turn counter-clockwise ([). \
Use Undo to reverse it."
            .to_owned()
    } else {
        format!(
            "Turn all {count} selected pages a quarter turn counter-clockwise ([). \
Each page turns from wherever it is now, so pages at different rotations stay \
different. Use Undo to reverse it."
        )
    }
}

/// Tooltip on the batch rotate-right control.
pub fn batch_rotate_right_tooltip(count: usize) -> String {
    if count == 1 {
        "Turn the selected page a quarter turn clockwise (]). \
Use Undo to reverse it."
            .to_owned()
    } else {
        format!(
            "Turn all {count} selected pages a quarter turn clockwise (]). \
Each page turns from wherever it is now, so pages at different rotations stay \
different. Use Undo to reverse it."
        )
    }
}

/// Label of the selection bar's delete button.
pub fn selection_delete_button() -> &'static str {
    "Delete"
}

/// Tooltip on the delete button.
///
/// The "delete is not redaction" half is **mandatory**, not decorative.
/// An operator who deletes a page believing its content is now gone is
/// wrong under the default save mode: the page is omitted from the new
/// page tree, not scrubbed from the file's bytes. Saying so here is the
/// fuzzy-never-sneaky rule applied to the most consequential
/// misunderstanding this feature invites.
pub fn selection_delete_tooltip(count: usize) -> String {
    let subject = if count == 1 {
        "the selected page".to_owned()
    } else {
        format!("the {count} selected pages")
    };
    format!(
        "Remove {subject} from this document. This can be undone until you save. \
Note: like every edit here, this does not erase the removed page's data from the \
file — the previous version can still contain it. If you need to permanently \
destroy content, use Redaction (not available yet), not Delete."
    )
}

/// Label of the selection bar's extract button.
pub fn selection_extract_button() -> &'static str {
    "Extract…"
}

/// Tooltip on the extract button.
pub fn selection_extract_tooltip(count: usize) -> String {
    let subject = if count == 1 {
        "the selected page".to_owned()
    } else {
        format!("the {count} selected pages")
    };
    format!(
        "Save {subject} as a new PDF file. This document is not changed, so there \
is nothing to undo."
    )
}

/// Status-bar line after a delete that orphaned references.
///
/// The disclosure `core_ops__delete_pages.md` recommends pdfce make and
/// Acrobat does not: Acrobat leaves these broken silently.
pub fn dangling_references_after_delete(
    pages: usize,
    bookmarks: usize,
    links: usize,
    destinations: usize,
) -> String {
    format!(
        "{pages} page(s) deleted. {bookmarks} bookmark(s), {links} link(s) and \
{destinations} named destination(s) pointed at a deleted page and now point \
nowhere — nothing else was changed. pdfce does not repoint them, because it \
cannot know which page you meant."
    )
}

/// Status-bar line after a delete that broke nothing.
pub fn delete_succeeded(pages: usize, objects: usize) -> String {
    format!(
        "{pages} page(s) deleted; {objects} object(s) removed from the document. \
Nothing else pointed at them. Use Undo to reverse this until you save."
    )
}

/// Status-bar line noting that a structural edit left the document's
/// page-label numbering stale.
///
/// Acrobat leaves it stale too — and silent. This is the parity-plus.
pub fn page_labels_now_stale() -> &'static str {
    "This document has custom page numbering (page labels). Changing which pages \
are in it does not adjust that numbering, so the labels are now out of step with \
the pages. Acrobat behaves the same way; pdfce tells you."
}

/// Informational note after a successful extract, when the source
/// document carries a signature.
pub fn extract_note_unsigned_output() -> &'static str {
    "This extracted file does not carry the original document's signature — the \
file you opened is untouched."
}

/// Status-bar line after a successful extract.
pub fn extract_succeeded(path: &Path, pages: usize, dangling: usize) -> String {
    let base = format!("Extracted {pages} page(s) to {}", file_name(path));
    if dangling == 0 {
        format!("{base}.")
    } else {
        format!(
            "{base}. {dangling} link(s) or destination(s) in those pages pointed \
somewhere outside the selection and were removed from the copy; the annotations \
themselves were kept."
        )
    }
}

// ---------------------------------------------------------------------------
// Reorder (Pass 3.2)
// ---------------------------------------------------------------------------

/// Tooltip on a thumbnail, naming both ways to move it.
///
/// Drag-and-drop is not keyboard-operable and egui's assistive-technology
/// support is a known gap, so the keyboard path is named here rather than
/// left to be discovered.
pub fn thumbnail_drag_tooltip(number: usize) -> String {
    format!(
        "Page {number}. Click to view it, click its checkbox to select it, or drag \
it to a new position. Keyboard: Alt+↑ / Alt+↓ moves the selected pages."
    )
}

/// Tooltip on the move-selection-up control.
pub fn move_selection_up_tooltip() -> &'static str {
    "Move the selected pages one position earlier (Alt+↑). One Undo step reverses \
the whole move."
}

/// Tooltip on the move-selection-down control.
pub fn move_selection_down_tooltip() -> &'static str {
    "Move the selected pages one position later (Alt+↓). One Undo step reverses \
the whole move."
}

// REMOVED 2026-08-03: `move_selection_up_button()` / `move_selection_down_button()`.
//
// They returned the bare text glyphs U+25B2 / U+25BC as the visible labels of
// the page-rail and Combine-files reorder controls. Observation of the running
// build showed both render as EMPTY BOXES in egui's default Proportional font
// chain (Ubuntu-Light -> NotoEmoji -> emoji-icon-font), the same gap that broke
// the menu buttons' U+25BE. Because these controls are glyph-ONLY, the missing
// glyph left them with no visible identity whatsoever.
//
// They now draw `Icon::ChevronUp` / `Icon::ChevronDown` through the icon
// pipeline, so no STRING is displayed and there is nothing for `ui_text` to
// own. Their tooltips — which are also their accessible names — remain here as
// `move_selection_up_tooltip()` / `move_selection_down_tooltip()` and
// `merge_move_up_tooltip()` / `merge_move_down_tooltip()`, so nothing about
// what a screen reader announces changed.
//
// Recorded rather than silently deleted, per the same convention Pass 18.3
// used when the icon set retired eleven glyph accessors. See
// docs/ui_specs/menu-affordance-and-glyph-coverage.md §8.2.

/// Status-bar line after a reorder.
pub fn reorder_succeeded(count: usize) -> String {
    format!("{count} page(s) moved. Use Undo to reverse this until you save.")
}

// ---------------------------------------------------------------------------
// Tools dock (Pass 3.2)
// ---------------------------------------------------------------------------

/// Title of the Tools dock panel.
pub fn tools_dock_title() -> &'static str {
    "Tools"
}

/// Heading above the dock's tool list.
///
/// Reworded for decision 012: the dock now holds both operations that act
/// on OTHER files (Combine/Split/Insert) and a standing preference that
/// changes how the OPEN document renders (Font folders), so the intro is
/// selection-neutral rather than claiming everything here acts on other
/// files.
pub fn tools_dock_intro() -> &'static str {
    "More operations, and preferences that shape how the open document renders."
}

/// Label of the dock's Combine-files entry.
pub fn tool_merge_label() -> &'static str {
    "Combine files…"
}

/// Label of the dock's Split entry.
pub fn tool_split_label() -> &'static str {
    "Split this document…"
}

/// Label of the dock's Insert-pages entry.
pub fn tool_insert_pages_label() -> &'static str {
    "Insert pages from a file…"
}

/// Label of the dock's Font-folders entry (decision 012).
pub fn tool_font_folders_label() -> &'static str {
    "Font folders…"
}

// ---------------------------------------------------------------------------
// Panel dock — tab labels, tooltips, chrome (decision 017 + Amendment A)
// ---------------------------------------------------------------------------
//
// Every tab label below doubles as a drag handle's visible name, and every
// tooltip below doubles as that tab's AccessKit accessible NAME (egui_tiles
// 0.16.0 ships its tab bars unnamed — see `crate::dock`'s module docs). So a
// tooltip here is not decoration: it is the only thing a screen reader has
// to go on. Decision 017 §8.6's rule applies with extra force — say WHEN to
// reach for the panel, never restate the label.

/// Tab label of the object/layer tree panel (ui-spec §B).
///
/// "Objects", not "Layers": the panel lists the page's *content-stream
/// objects* in paint order, and the model has no optional-content-group
/// (OCG) grouping for page content to build real layers from (§B.1). Calling
/// it "Layers" would promise a hierarchy that does not exist.
pub fn dock_panel_objects_label() -> &'static str {
    "Objects"
}

/// Purpose tooltip / accessible name for the Objects tab.
pub fn dock_panel_objects_tooltip() -> &'static str {
    "List everything drawn on this page, front to back, so you can work out what a click on \
the canvas is selecting — and select an object from the list when it is too small or too \
hidden to click."
}

/// Tab label of the document-properties panel.
///
/// Plain "Properties" is now unambiguous: decision 017 §8.3 retired the
/// floating document-properties window, so there is no second surface in the
/// app competing for the word. (The ui-spec's §A.2 rename to "Document
/// Properties" existed to defuse a collision with a *selection* inspector
/// that §A's superseded design would have added; no such panel ships here.)
pub fn dock_panel_properties_label() -> &'static str {
    "Properties"
}

/// Purpose tooltip / accessible name for the Properties tab.
pub fn dock_panel_properties_tooltip() -> &'static str {
    "Read and edit the whole document's title, author, subject and keywords. This is the \
file's own metadata, not anything about the object you have selected."
}

/// Tab label of the batch-tools panel.
///
/// **Renamed from "Tools" deliberately** (decision 017 §8.5, still binding
/// under A.4 #4): a row labelled "Tools" inside a container operators call
/// "the Tools dock" — reached from a toolbar button also labelled "Tools" —
/// is a three-way collision people trip on. "Batch" also says the true
/// scope: these act on files, mostly ones that are not even open.
pub fn dock_panel_batch_tools_label() -> &'static str {
    "Batch Tools"
}

/// Purpose tooltip / accessible name for the Batch Tools tab.
pub fn dock_panel_batch_tools_tooltip() -> &'static str {
    "Combine, split or insert pages across whole files, and manage the font folders pdfce \
draws missing typefaces from. Tools that act on pages you can already see — delete, reorder, \
rotate, extract — live on the page thumbnails instead."
}

/// Fallback tab title for a CONTAINER tab — a tab group the operator built
/// by dragging one group inside another.
///
/// pdfce's default layout never produces one, so this names the container's
/// shape rather than inventing a title pdfce has no basis for.
pub fn dock_container_tab_label(kind: egui_tiles::ContainerKind) -> &'static str {
    match kind {
        egui_tiles::ContainerKind::Tabs => "Tab group",
        egui_tiles::ContainerKind::Horizontal => "Side-by-side group",
        egui_tiles::ContainerKind::Vertical => "Stacked group",
        egui_tiles::ContainerKind::Grid => "Grid group",
    }
}

/// Tab title for a tile the layout engine can no longer resolve.
///
/// Should be unreachable. Shown rather than swallowed because a tab with no
/// title is indistinguishable from a rendering bug, and an operator who can
/// name what they saw can report it.
pub fn dock_missing_tile_label() -> &'static str {
    "(panel unavailable)"
}

/// Label of the dock header's reset control (decision 017 §8.12 / A.4 #6).
pub fn dock_reset_layout_button() -> &'static str {
    "Reset panel layout"
}

/// Tooltip on the reset control — says WHEN to reach for it.
///
/// A draggable layout can be wrecked in ways a fixed one cannot (a pane
/// dragged to a sliver, a group nested three deep), which is exactly why
/// Amendment A promoted this from "nice to have" to ships-in-the-same-Pass.
pub fn dock_reset_layout_tooltip() -> &'static str {
    "Put every panel back where it started. Use this if dragging has left a panel too small \
to read, or hidden behind a tab you cannot find."
}

/// The dock header's persistence disclosure (decision 017 §7 / A.6, R82).
///
/// Visible text, not a tooltip: the operator will arrange panels and then
/// close the app, and finding the arrangement gone with no prior warning is
/// precisely the surprise decision 012 set the precedent against.
pub fn dock_layout_session_only_note() -> &'static str {
    "Panel arrangement lasts for this session only — it is not saved when you close pdfce."
}

/// Shown in the Properties panel when no document is open.
///
/// The panel is never blanked: a blank region is indistinguishable from a
/// broken one, so the honest answer is a sentence naming the precondition.
pub fn properties_dock_no_document_hint() -> &'static str {
    "Open a document to read or edit its title, author, subject and keywords."
}

// ---------------------------------------------------------------------------
// Objects panel — the page's object/layer tree (ui-spec §B)
// ---------------------------------------------------------------------------

/// Intro line above the object list. States the ordering convention, because
/// "which end of this list is the front of the page?" is otherwise a guess.
pub fn objects_dock_intro() -> &'static str {
    "Everything drawn on this page, front-most first. Click a row to select it on the page; \
Shift+click to add it to, or remove it from, the selection."
}

/// Empty state: nothing is open.
pub fn objects_dock_no_document_hint() -> &'static str {
    "Open a document to see the objects on its pages."
}

/// Empty state: the page decomposed cleanly and holds nothing selectable.
///
/// Deliberately distinct from [`objects_dock_decompose_failed_hint`] — a
/// genuinely blank page and a page pdfce could not read must never look
/// identical (fuzzy, never sneaky: a failure state must not be
/// indistinguishable from a success state that happens to be empty).
pub fn objects_dock_empty_page_hint() -> &'static str {
    "This page has nothing selectable on it — no shapes, text or images that pdfce can \
address individually."
}

/// Empty state: the page's content could not be analysed.
pub fn objects_dock_decompose_failed_hint() -> &'static str {
    "pdfce could not analyse this page's contents, so it cannot list its objects. The page \
may still display correctly; clicking objects on it will find nothing."
}

/// Summary line under the object list: how many objects, and how many of
/// them are selected right now.
pub fn objects_dock_summary(total: usize, selected: usize) -> String {
    format!("{total} object(s) on this page, {selected} selected.")
}

/// Tooltip on an object row. One entry for every row rather than a
/// per-kind variant: the interaction is identical whatever was clicked, and
/// R2 forbids assembling this sentence from fragments.
pub fn objects_dock_row_tooltip() -> &'static str {
    "Select this object on the page. Shift+click to add it to, or remove it from, the current \
selection."
}

/// The plain-language name of an object kind — the ONE place each kind is
/// named, so the tree row, the status readout and the canvas badge tooltip
/// cannot drift into calling the same thing three names.
///
/// The three image kinds get three different names rather than one, because
/// they are three different answers to "what did I select?": an inline image
/// lives in the page's own byte stream, an image XObject is a shared resource,
/// and a form XObject is an entire nested drawing treated as one opaque
/// object — which is itself a common explanation for "why is the selection box
/// so much bigger than the thing I clicked?".
pub fn object_kind_label(kind: ObjectKind) -> &'static str {
    match kind {
        ObjectKind::Path => "Path",
        ObjectKind::Text => "Text",
        ObjectKind::InlineImage => "Image (inline)",
        ObjectKind::ImageXObject => "Image",
        ObjectKind::FormXObject => "Form",
    }
}

/// The one- or two-character badge drawn at the corner of a selection outline
/// on the canvas (ui-spec §C.1).
///
/// A LETTER, not an icon, and that is a decision rather than a shortcut: the
/// icon set (`icons::Icon`) has no glyph for "path", "image" or "form
/// XObject", and its `Text` glyph names the text *tool*, not a text object —
/// borrowing it would assert an affordance that does not exist. §C.1 names a
/// letter badge as the honest interim and says the badge's POSITION and
/// EXISTENCE are the durable part of the design; the glyph swaps later without
/// redesigning anything.
///
/// The letter is never the only cue: it sits inside a filled chip (a shape),
/// beside an outline whose dash pattern already distinguishes approximate from
/// measured bounds, and the full sentence is in the status readout. R84 —
/// never colour alone — is satisfied several times over.
pub fn object_kind_badge(kind: ObjectKind) -> &'static str {
    match kind {
        ObjectKind::Path => "P",
        ObjectKind::Text => "T",
        ObjectKind::InlineImage | ObjectKind::ImageXObject => "I",
        ObjectKind::FormXObject => "F",
    }
}

/// The detail clause for one object — everything after its kind name.
///
/// Empty for the kinds the core model cannot describe further (ui-spec
/// §B.3/§B.4): `TextObject` carries no string, font or size, and
/// `ImageObject` carries no pixel dimensions or colourspace, so there is
/// genuinely nothing more to say. Shipping an empty clause is the honest
/// answer; inventing `Helvetica 10pt` from the spec's illustrative example
/// would be a fabrication the operator has no way to catch.
fn object_detail(summary: &ObjectSummary) -> String {
    let Some(paint) = summary.paint else {
        return String::new();
    };
    let mut detail = paint_style_label(paint).to_owned();
    if let Some(colour) = summary.colour {
        detail.push(' ');
        detail.push_str(&rgb_hex(colour));
    }
    if let Some(width) = summary.line_width {
        detail.push_str(&format!(", {width:.2} pt wide"));
    }
    if let Some(nodes) = summary.nodes {
        detail.push_str(&format!(" · {nodes} node(s)"));
    }
    detail
}

/// One-line row text for any object — the Objects panel's row label.
///
/// `index` is the object's PAINT-ORDER index, printed verbatim so it
/// cross-references `pdfce-cli object-list`'s `index=` field and the
/// `object-move`/`object-delete`/`node-move` operands, which all address an
/// object by exactly this number. Showing a display-position number instead
/// (the list is drawn back-to-front) would produce a number that looks
/// authoritative and addresses the wrong object.
///
/// Takes an [`ObjectSummary`] rather than a `VectorObject`: the classification
/// (which colour is actually visible, how many nodes, which disclosures apply)
/// belongs to `object_summary::describe_object`, and this function only words
/// it. That is ui-spec §C.6's single-source-of-truth ask made structural — the
/// row and the selection readout below are two renderings of ONE record, so a
/// fill colour cannot be described one way here and another way there.
///
/// The trailing note marker is what makes a row diagnostic rather than
/// decorative: a text row, a clip path and a hairline all look ordinary until
/// the row says out loud that the box will not match what is on the paper.
pub fn object_row(index: usize, summary: &ObjectSummary) -> String {
    let kind = object_kind_label(summary.kind);
    let detail = object_detail(summary);
    let head = if detail.is_empty() {
        format!("#{index}  {kind}")
    } else {
        format!("#{index}  {kind} · {detail}")
    };
    match headline_note(summary) {
        Some(note) => format!("{head} · {}", object_note_short(note)),
        None => head,
    }
}

/// The one note worth putting on a single line beside an object's detail
/// clause, if any.
///
/// `PaintsNothing` is deliberately skipped: [`paint_style_label`] already
/// spells it out inside the detail clause, and a line reading "…paints
/// nothing (a clip or discarded path) · 4 node(s) · paints nothing" says it
/// twice and explains it neither time. Every other note adds a fact the
/// detail clause does not carry. The FULL sentence for `PaintsNothing` is
/// still shown in the expanded explanation, where it earns its space by
/// saying the object is real, selectable and editable.
fn headline_note(summary: &ObjectSummary) -> Option<ObjectNote> {
    summary
        .notes
        .iter()
        .copied()
        .find(|note| !matches!(note, ObjectNote::PaintsNothing))
}

/// The SHORT form of a disclosure, for a one-line row where a full sentence
/// would not fit (ui-spec §B.3).
///
/// Paired with [`object_note`]'s long form rather than replacing it: the row
/// flags that something needs explaining, the readout explains it. Two lengths
/// of the same fact, never two different facts.
pub fn object_note_short(note: ObjectNote) -> &'static str {
    match note {
        ObjectNote::ApproximateTextBounds => "approximate bounds",
        ObjectNote::PaintsNothing => "paints nothing",
        ObjectNote::DegenerateBounds(Degeneracy::VerticalRule) => "zero width",
        ObjectNote::DegenerateBounds(Degeneracy::HorizontalRule) => "zero height",
        ObjectNote::DegenerateBounds(Degeneracy::Point) => "a single point",
        ObjectNote::NoBounds => "no measurable bounds",
        ObjectNote::FormNotDecomposed => "a whole nested drawing",
    }
}

/// The FULL disclosure sentence for one fact about a selection — the direct
/// answer to the operator's *"sometimes I click and get a box highlighting on
/// the screen that doesn't seem to correspond to anything."*
///
/// Every one of these is a fact `pdfce-core` already computed and never
/// showed: `TextObject::approximate`, `PaintStyle::is_invisible`, an exact
/// zero-extent comparison on the bbox. None of them is a guess, which is what
/// makes surfacing them a disclosure rather than an inference the operator
/// would have to review (rule 4, fuzzy never sneaky).
///
/// Each sentence says WHAT is true and WHY the operator is seeing what they
/// are seeing, because "approximate bounds" on its own is a label, not an
/// explanation — and an explanation is the entire deliverable here.
pub fn object_note(note: ObjectNote) -> &'static str {
    match note {
        ObjectNote::ApproximateTextBounds => {
            "The box around text is approximate: pdfce measures a text object from where each \
run of glyphs STARTS, then pads by the largest type size it saw, so the box is normally wider \
and taller than the ink. Clicking blank space near text can therefore select the text — the \
selection is correct even though the box looks empty."
        }
        ObjectNote::PaintsNothing => {
            "This path paints nothing at all — it is a clipping path or a shape that was built \
and then discarded without being filled or stroked. It is a real object you can select, move \
and delete, but there is nothing on the paper to see inside the box."
        }
        ObjectNote::DegenerateBounds(Degeneracy::VerticalRule) => {
            "This object has zero width — it is a vertical rule. Its outline is widened on \
screen so you can see it; the object itself is a line, not a box."
        }
        ObjectNote::DegenerateBounds(Degeneracy::HorizontalRule) => {
            "This object has zero height — it is a horizontal rule. Its outline is thickened on \
screen so you can see it; the object itself is a line, not a box."
        }
        ObjectNote::DegenerateBounds(Degeneracy::Point) => {
            "This object is a single point — it has no width and no height. Its outline is \
enlarged on screen so you can see where it is."
        }
        ObjectNote::NoBounds => {
            "pdfce could not work out where this object is on the page, so no outline is drawn \
for it. It is still selected, and it is still listed in the Objects panel."
        }
        ObjectNote::FormNotDecomposed => {
            "This is a form XObject — a whole nested drawing that pdfce treats as ONE object. \
The box covers the entire nested drawing, and the shapes inside it cannot be selected \
individually yet."
        }
    }
}

/// The status bar's selection readout for exactly ONE selected object
/// (ui-spec §C.5/§C.6).
///
/// Lives in the status bar rather than only in the dock because the dock is
/// not open by default and the canvas is where the confusion happens: a
/// readout the operator has to first discover a panel to reach is not a
/// readout for the moment they are asking "what did I just click?".
///
/// Size and position are stated in PDF points, the same unit the Objects panel
/// and `pdfce-cli` use, and to one decimal — enough to tell a 0.0-pt-tall rule
/// from a 0.5-pt one, which is precisely the distinction that made a selection
/// look like nothing at all. That number is also what keeps the canvas's
/// deliberately-thickened outline for a degenerate object honest: the box on
/// screen is 6 pt tall so it can be seen, and this line says the object is
/// 0.0 pt tall, so the two can never be confused.
///
/// **One line, always.** The status bar is a bottom panel, so each line it
/// grows takes height from the canvas — and under "Fit page" that re-fits the
/// page smaller, making it visibly jump the instant something is selected.
/// The headline therefore carries the short form of the leading disclosure and
/// the full sentences live behind the expander (ui-spec §C.5: "the status bar
/// gets ONE short summary line, not the full detail").
pub fn selection_readout_single(summary: &ObjectSummary) -> String {
    let kind = object_kind_label(summary.kind);
    let detail = object_detail(summary);
    let mut head = if detail.is_empty() {
        format!("Selected: {kind}")
    } else {
        format!("Selected: {kind} · {detail}")
    };
    if let Some(note) = headline_note(summary) {
        head.push_str(" · ");
        head.push_str(object_note_short(note));
    }
    match summary.size() {
        Some((w, h)) => format!(
            "{head} — {w:.1} × {h:.1} pt at ({:.1}, {:.1}).",
            summary.bounds.min.x, summary.bounds.min.y
        ),
        None => format!("{head}."),
    }
}

/// The status bar's selection readout for a MULTI-object selection.
///
/// Orientation, not detail (ui-spec §C.6): after a marquee, the question is
/// "did I catch what I meant?", and a per-object dump buries the answer. The
/// Objects panel's highlighted rows are where per-object detail lives.
pub fn selection_readout_multi(census: SelectionCensus) -> String {
    let mut parts: Vec<String> = Vec::new();
    if census.paths > 0 {
        parts.push(format!("{} path(s)", census.paths));
    }
    if census.texts > 0 {
        parts.push(format!("{} text object(s)", census.texts));
    }
    if census.images > 0 {
        parts.push(format!("{} image(s)", census.images));
    }
    if census.forms > 0 {
        parts.push(format!("{} form(s)", census.forms));
    }
    if parts.is_empty() {
        return format!("Selected: {} object(s).", census.total);
    }
    format!("Selected: {} objects — {}.", census.total, parts.join(", "))
}

/// The status readout for a selection whose objects the current page model
/// can no longer resolve.
///
/// A transient rather than an error: a `TargetId` can outlive the object it
/// named for one frame after an edit, before `prune_canvas_selection` runs.
/// Saying so beats silence, which would read as "the readout is broken".
pub fn selection_readout_unresolved(count: usize) -> String {
    format!("Selected: {count} object(s) — details are not available for this page right now.")
}

/// Tooltip on the status bar's selection readout: says where the fuller
/// answer lives, so the readout doubles as the Objects panel's own
/// discoverability hint.
pub fn selection_readout_tooltip() -> &'static str {
    "What is selected on the page right now. Open the Objects panel for the full list, and to \
click rows to select objects you cannot see."
}

/// Plain-language name for a path's painting disposition (§8.5.3, Table 60).
///
/// Words rather than the CLI's machine tokens (`fill-nonzero+stroke`): this
/// is prose an operator reads, and the two surfaces have different
/// audiences. The `n` case is spelled out at length because "paints nothing"
/// is the direct answer to "why is there a selection box over blank paper?"
/// — a clip or discarded path is still a real, selectable object.
pub fn paint_style_label(style: PaintStyle) -> &'static str {
    match (style.fill, style.stroke) {
        (Some(FillRule::NonZero), true) => "filled and stroked",
        (Some(FillRule::NonZero), false) => "filled",
        (Some(FillRule::EvenOdd), true) => "filled (even-odd) and stroked",
        (Some(FillRule::EvenOdd), false) => "filled (even-odd)",
        (None, true) => "stroked",
        (None, false) => "paints nothing (a clip or discarded path)",
    }
}

/// Format a colour as `#RRGGBB` (R6: number formatting lives in the
/// catalog, never inline at a call site).
///
/// Components are clamped before scaling: a PDF may set a colour component
/// outside 0..1 and the decomposition records what it read rather than
/// silently repairing it, so the clamp belongs here, at the point of
/// display, not in the model.
pub fn rgb_hex(colour: Rgb) -> String {
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!(
        "#{:02X}{:02X}{:02X}",
        byte(colour.r),
        byte(colour.g),
        byte(colour.b)
    )
}

// ---------------------------------------------------------------------------
// Font folders (decision 012 — operator-supplied fonts)
// ---------------------------------------------------------------------------

/// Intro shown at the top of the Font-folders tool. States the scope (the
/// OPEN document's non-embedded fonts), the shapes-not-layout caveat
/// (§4.4), and the session-only persistence stance up front.
pub fn font_folders_intro() -> &'static str {
    "Point pdfce at folders of your own font files (.ttf/.otf) so it can draw the open \
document's missing text with the real typeface instead of a bundled substitute. This only \
changes how missing fonts look, not where text sits on the page. Folders are remembered for \
this session only."
}

/// Label of the add-folder button.
pub fn font_folders_add_button() -> &'static str {
    "Add folder…"
}

/// Tooltip on the add-folder button — says WHEN to reach for it, not just
/// what it does, and keeps the load-bearing "not where text sits" clause.
pub fn font_folders_add_tooltip() -> &'static str {
    "Choose a folder containing font files. pdfce will use a matching face to draw any font \
this document references but does not embed — improving how missing text looks, not where it \
sits on the page."
}

/// Tooltip on a Font-folders row's remove button.
pub fn font_folders_remove_tooltip() -> &'static str {
    "Stop using this folder's fonts. The folder and its files are not touched."
}

/// Empty-state line, so "no folders configured" reads as a stated fact
/// rather than a blank panel.
pub fn font_folders_empty_hint() -> &'static str {
    "No font folders added — pdfce uses its own bundled fonts for anything the document does \
not embed."
}

/// Session-level "supplied fonts active" indicator (decision 012 R63):
/// shown whenever any folder is configured, disclosing that a render
/// using supplied fonts is machine-dependent.
pub fn font_folders_active_indicator(count: usize) -> String {
    format!(
        "{count} font folder(s) active this session. Pages that use a supplied font may look \
different on another machine, since they depend on the fonts you provided here."
    )
}

/// Heading of the collapsible walk-notes list (which files registered
/// under which names, which were skipped).
pub fn font_folders_notes_heading() -> &'static str {
    "Which font files pdfce found"
}

/// Walk note: a font folder could not be read.
pub fn font_folder_note_unreadable_dir(
    dir: &dyn std::fmt::Display,
    err: &dyn std::fmt::Display,
) -> String {
    format!("Could not read folder {dir}: {err}")
}

/// Walk note: a file exceeded the size ceiling and was skipped.
pub fn font_folder_note_oversized(path: &dyn std::fmt::Display) -> String {
    format!("Skipped {path}: file is too large to be a font pdfce will load.")
}

/// Walk note: a file could not be read and was skipped.
pub fn font_folder_note_skipped(
    path: &dyn std::fmt::Display,
    err: &dyn std::fmt::Display,
) -> String {
    format!("Skipped {path}: {err}")
}

/// Walk note: a file did not parse as a usable font and was skipped.
pub fn font_folder_note_unparseable(path: &dyn std::fmt::Display) -> String {
    format!("Skipped {path}: not a font pdfce can read.")
}

/// Walk note: a face registered under the given name(s).
pub fn font_folder_note_registered(path: &dyn std::fmt::Display, names: &str) -> String {
    format!("Using {path} for: {names}")
}

/// Body text for a dock entry whose graphical form has not shipped yet,
/// naming the command-line equivalent that has.
///
/// A placeholder that says "coming soon" wastes the operator's time; one
/// that hands them a working command does not. `command` is a literal
/// command line and is deliberately **not** translated — it is syntax,
/// not prose.
pub fn tool_available_in_cli(command: &str) -> String {
    format!(
        "This tool does not have a window yet. It works today from the command \
line, which does exactly the same thing:\n\n    {command}\n\nRun it with --help \
for every option."
    )
}

// ---------------------------------------------------------------------------
// Combine files (Pass 3.2)
// ---------------------------------------------------------------------------

/// Label of the add-files button in the Combine tool.
pub fn merge_add_files_button() -> &'static str {
    "Add files…"
}

/// Tooltip on the add-files button.
pub fn merge_add_files_tooltip() -> &'static str {
    "Choose one or more PDFs to add to the list. You can pick several at once."
}

/// Row label for the currently open document in the Combine list.
pub fn merge_current_document_label(path: &Path) -> String {
    format!("{} (the document you have open)", file_name(path))
}

/// Tooltip on a Combine list row's remove button.
pub fn merge_remove_file_tooltip() -> &'static str {
    "Take this file out of the list. The file itself is not touched."
}

/// Tooltip on a Combine list row's move-up button.
pub fn merge_move_up_tooltip() -> &'static str {
    "Move this file earlier in the combined document."
}

/// Tooltip on a Combine list row's move-down button.
pub fn merge_move_down_tooltip() -> &'static str {
    "Move this file later in the combined document."
}

/// Hint shown while the Combine list has fewer than two files.
pub fn merge_needs_two_files_hint() -> &'static str {
    "Add at least two files to combine."
}

/// Label of the Combine commit button.
pub fn merge_commit_button() -> &'static str {
    "Combine…"
}

/// Tooltip on the Combine commit button.
pub fn merge_commit_tooltip(files: usize) -> String {
    format!(
        "Write a new PDF containing all {files} files in the order listed. None of \
them is changed, so there is nothing to undo."
    )
}

/// Checkbox label for per-source bookmark generation.
pub fn merge_bookmarks_checkbox() -> &'static str {
    "Add a bookmark for each file"
}

/// Tooltip on the per-source bookmark checkbox.
pub fn merge_bookmarks_tooltip() -> &'static str {
    "Adds one top-level bookmark per source file, named after that file, with \
that file's own bookmarks underneath it. This is what Acrobat does by default."
}

/// Suggested file name for a combined document.
pub fn suggested_merge_name(first: &Path) -> String {
    let stem = first
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "document".to_owned());
    format!("{stem} (combined).pdf")
}

/// Status-bar line after a successful combine.
pub fn merge_succeeded(path: &Path, files: usize, pages: usize, renamed: usize) -> String {
    let base = format!(
        "Combined {files} files into {} — {pages} page(s).",
        file_name(path)
    );
    if renamed == 0 {
        base
    } else {
        format!(
            "{base} {renamed} form field(s) were renamed because their names \
appeared in more than one file; without that, typing in one would have filled \
every copy."
        )
    }
}

/// Informational note after a combine whose sources included a signature.
pub fn merge_note_unsigned_output() -> &'static str {
    "The combined file does not carry any source document's signature — the \
source files are untouched."
}

// ---------------------------------------------------------------------------
// Signature interplay on save (Pass 3.2)
// ---------------------------------------------------------------------------

/// Title of the signature-invalidation confirmation.
pub fn signature_invalidation_title() -> &'static str {
    "Saving will invalidate this document's signature"
}

/// Body of the signature-invalidation confirmation.
///
/// States what will happen and why, without softening it. The
/// invalidation is a cryptographic consequence of changing a signed
/// document, not a pdfce policy that could be relaxed.
pub fn signature_invalidation_body() -> &'static str {
    "This document is digitally signed, and the changes you have made are not \
changes the signature allows. Saving will leave the file readable, but any \
program that checks the signature will report it as invalid or as altered since \
signing.\n\nThis cannot be avoided by saving differently — a signature covers the \
document as it was signed. If the signature matters, keep a copy of the original \
file before saving."
}

/// Confirm button of the signature-invalidation confirmation.
pub fn signature_invalidation_confirm_button() -> &'static str {
    "Save without the signature"
}

/// Cancel button of the signature-invalidation confirmation.
pub fn signature_invalidation_cancel_button() -> &'static str {
    "Don't save yet"
}

/// Persistent status-bar line after saving a document whose signature the
/// save invalidated.
///
/// A modal is dismissed and forgotten; the operator may ask "did I break
/// that signature?" an hour later, and this is what answers.
pub fn save_signature_invalidated_note() -> &'static str {
    "This save invalidated the document's digital signature. The original file, \
if you kept one, still carries a valid signature."
}

/// Persistent status-bar line after saving a signed document whose signed
/// byte range the save preserved.
///
/// ⚠️ Deliberately **not** reassuring, and this wording is load-bearing.
/// ISO 32000-1 §12.8.2.2.2 splits validation into two stages: the
/// byte-range digest, and whether the changes were ones the signer
/// permitted. An incremental save guarantees only the first. Rendering
/// that as "your signature is still valid" is the specific error the
/// spec's two-stage split exists to prevent.
pub fn save_signature_byte_range_preserved() -> &'static str {
    "This document is signed. The save added a new revision without altering the \
signed part of the file, so the signature's own checksum still matches — but that \
is only half of what makes a signature valid. Whether these changes are ones the \
signer allowed is a separate question, and a signature checker may still report \
the document as altered since signing."
}

// ---------------------------------------------------------------------------
// Undo/redo command labels (Pass 3.2)
// ---------------------------------------------------------------------------

/// A human name for what an undo-stack entry did.
///
/// `pdfce-core` returns a structured `CommandKind` rather than a string,
/// precisely so the naming happens here (decision 002 R1). This is the
/// mapping.
pub fn command_label(kind: CommandKind) -> String {
    match kind {
        CommandKind::SetInfoField(field) => {
            format!("change {}", info_field_label(field).to_lowercase())
        }
        CommandKind::ClearInfoField(field) => {
            format!("clear {}", info_field_label(field).to_lowercase())
        }
        CommandKind::SetPageRotation { page_index, .. } => {
            format!("rotate page {}", page_index + 1)
        }
        CommandKind::DeletePages { count } => format!("delete {count} page(s)"),
        CommandKind::ReorderPages { count } => format!("move {count} page(s)"),
        CommandKind::RotatePages { count, .. } => format!("rotate {count} page(s)"),
        CommandKind::ReflowBlock {
            lines_before,
            lines_after,
        } => format!("reflow block ({lines_before}->{lines_after} lines)"),
        // Pass 12.M2 dimensioning commands.
        CommandKind::AddDimension => "add dimension".to_owned(),
        CommandKind::SetGroupScale { members } => {
            format!("change group scale ({members} dimension(s) updated)")
        }
        CommandKind::ToggleDimensionLayer { visible } => {
            if visible {
                "show dimension layer".to_owned()
            } else {
                "hide dimension layer".to_owned()
            }
        }
        // `CommandKind` is #[non_exhaustive]: a future variant must not
        // make the tooltip lie, so it degrades to the generic word rather
        // than to a wrong one.
        _ => "the last change".to_owned(),
    }
}

/// Tooltip on the Undo control, naming what it would undo.
pub fn undo_tooltip_for(kind: Option<CommandKind>) -> String {
    match kind {
        Some(kind) => format!("Undo {} (Ctrl+Z)", command_label(kind)),
        None => "Nothing to undo (Ctrl+Z)".to_owned(),
    }
}

/// Tooltip on the Redo control, naming what it would redo.
pub fn redo_tooltip_for(kind: Option<CommandKind>) -> String {
    match kind {
        Some(kind) => format!("Redo {} (Ctrl+Y or Ctrl+Shift+Z)", command_label(kind)),
        None => "Nothing to redo (Ctrl+Y or Ctrl+Shift+Z)".to_owned(),
    }
}

// ---------------------------------------------------------------------------
// Properties panel — carried review items (Pass 3.1 follow-ups)
// ---------------------------------------------------------------------------

/// Marker appended to a properties field whose stored bytes could not be
/// decoded with certainty.
///
/// Per-field rather than only per-panel: the panel-level warning says
/// *something* here is uncertain, and the operator then has to guess
/// which box. This says which.
pub fn info_field_lossy_marker() -> &'static str {
    " ⚠"
}

/// Tooltip on a properties field that decoded lossily.
pub fn info_field_lossy_tooltip() -> &'static str {
    "Some bytes in this field have no certain meaning and are shown as �. If you \
apply the panel without editing this box, pdfce leaves the stored value alone \
rather than writing back the guess."
}

/// Tooltip on the properties Apply button when the draft matches what the
/// document already holds.
pub fn properties_apply_unchanged_tooltip() -> &'static str {
    "Nothing to apply — these values already match the document."
}

/// Suggested file name for an extracted page subset.
///
/// Names the pages so the operator can tell three extractions from the
/// same document apart in a folder — `report.pdf`, `report.pdf` and
/// `report.pdf` is not a naming scheme. A long or scattered selection
/// collapses to a count rather than producing a hundred-character name
/// that some filesystems reject.
pub fn suggested_extract_name(source: &Path, pages: &[usize]) -> String {
    let stem = file_stem(source);
    match (pages.first(), pages.last(), pages.len()) {
        (Some(only), _, 1) => format!("{stem} (page {}).pdf", only + 1),
        (Some(first), Some(last), n) if last - first + 1 == n => {
            format!("{stem} (pages {}-{}).pdf", first + 1, last + 1)
        }
        (_, _, n) => format!("{stem} ({n} pages).pdf"),
    }
}

/// A file's base name without its extension — the `{stem}` a suggested
/// output name and a per-source bookmark are both built from.
pub fn file_stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "document".to_owned())
}

/// Failure line when one of the Combine tool's inputs could not be read.
///
/// Names the file. "Could not combine" without saying which of six files
/// was the problem is a message that costs the operator six attempts.
pub fn merge_input_failed(path: &Path, message: &str) -> String {
    format!("Could not read {} — {message}", file_name(path))
}

/// The glyph painted inside a selected page's checkbox.
///
/// A glyph AND a fill, never colour alone — a colour-only state is
/// invisible to a substantial fraction of operators, and this is the
/// control that decides what a Delete button acts on.
pub fn selection_check_glyph() -> &'static str {
    "✔"
}

/// The command line the Tools dock shows for splitting.
///
/// A literal command, and therefore **never translated** even when the
/// rest of this catalog is: it is syntax the operator types, and a
/// localized flag name would not run. It lives here anyway so the R1
/// grep has nothing to find outside the catalog, and so that a future
/// localizer sees it and knows to leave it alone.
pub fn split_cli_command() -> &'static str {
    "pdfce-cli split <file.pdf> --out-dir <folder> --every 1"
}

/// The command line the Tools dock shows for inserting pages. Never
/// translated — see [`split_cli_command`].
pub fn insert_cli_command() -> &'static str {
    "pdfce-cli insert-pages <file.pdf> --source <other.pdf> --after 1 -o <out.pdf>"
}

// ---------------------------------------------------------------------------
// Copy text (Pass 4 — ISO 32000-1 §9.10 text extraction)
// ---------------------------------------------------------------------------
//
// Placement, and why it is NOT in the Tools dock: the dock's own intro
// sentence is "These tools work with files outside the one you have
// open" — the load-bearing rail-vs-dock rule from
// `docs/ui_specs/pass-3.2-page-ops.md` §1. Copy-text's argument is the
// open document itself, so putting it in the dock would make that
// sentence false the first time an operator read it. It lives on the
// toolbar instead, as a second UNGROUPED utility button beside the Tools
// toggle: it belongs to neither the view group (it changes nothing on
// screen) nor the edit group (it cannot touch the file — the answer to
// "does this button change my document?" is No, structurally), and
// forcing it into either would make that group's own organizing question
// unanswerable at a glance.
//
// The disclosure below is two-tier on purpose, and the tiers are not a
// severity ranking — they separate two different KINDS of fact:
//
//   * DERIVED WHITESPACE is routine. ISO 32000-1 guarantees no
//     inter-word signal at all outside a Tagged PDF (§14.8.2.5 and the
//     S1–S9 negative results), so nearly every real document needs some.
//     It is stated plainly, uncoloured, in the detail only. Colouring it
//     as a caution would teach operators to distrust something that is
//     true of almost every PDF they will ever open.
//   * UNREADABLE CHARACTERS are genuinely uncertain. Those get the
//     warn-colour treatment and the headline, matching how
//     `diagnostics_summary` handles unsupported render items.

/// Label of the toolbar's Copy-text button.
///
/// The trailing chevron says it opens a menu rather than acting
/// immediately — the operator must choose a scope, and a button that
/// copied *something* without saying which would be exactly the guess
/// this feature exists not to make.
pub fn copy_text_button() -> &'static str {
    "Copy"
}

/// Tooltip for the Copy-text toolbar button.
pub fn copy_text_tooltip() -> &'static str {
    "Copy text — this page (Ctrl+Shift+C) or the whole document"
}

/// Menu item: copy the page currently on screen.
pub fn copy_page_text_menu_item() -> &'static str {
    "Copy this page's text"
}

/// Tooltip for the page-scope menu item.
///
/// Names the inference in the tooltip rather than only in the result
/// line, so the operator meets it before the paste rather than after it.
pub fn copy_page_text_tooltip() -> &'static str {
    "Ctrl+Shift+C. Where a PDF does not say where words and lines end, pdfce \
works it out from the position of the letters — the result line says how much \
of the copy that was."
}

/// Menu item: copy every page.
pub fn copy_document_text_menu_item() -> &'static str {
    "Copy the whole document's text"
}

/// Tooltip for the document-scope menu item.
///
/// Names the delay honestly. A long document is read page by page, and
/// on a few hundred pages that is a visible pause with no progress
/// indication yet.
pub fn copy_document_text_tooltip() -> &'static str {
    "Reads every page. On a long document this can take a few seconds, during \
which the window will not respond."
}

/// Status line when the requested pages carried no text at all.
///
/// This case gets its own sentence rather than falling through to an
/// empty successful copy: an operator who pastes nothing and is told
/// nothing has no way to tell "this page has no text" from "the button
/// is broken". A scanned page with no OCR layer is the common cause and
/// is named, because that is the fact that suggests the next step.
pub fn copy_text_no_extractable_text() -> &'static str {
    "Nothing copied — there is no text here to copy. A scanned page holds a \
picture of text rather than text; reading it needs OCR, which pdfce does not \
have yet."
}

/// Status line after a successful page copy.
///
/// Names the page explicitly. Unlike the render diagnostics beside it —
/// which are re-read from the current page's raster every frame and are
/// therefore always about what is on screen — this result persists after
/// the operator navigates away, so "this page" would become a lie the
/// moment they turned the page.
pub fn copy_text_succeeded_page(page_number: usize, characters: usize) -> String {
    format!("Copied {characters} character(s) of text from page {page_number}.")
}

/// Status line after a successful whole-document copy.
pub fn copy_text_succeeded_document(pages: usize, characters: usize) -> String {
    format!("Copied {characters} character(s) of text from all {pages} page(s).")
}

/// The headline when every character came from the document itself.
///
/// Stated positively and always shown, for the same reason
/// [`diagnostics_clean`] is: silence is ambiguous between "clean" and
/// "the check did not run".
pub fn copy_text_headline_clean() -> &'static str {
    "Every character came from the document's own text."
}

/// The headline when some characters could not be read.
///
/// Warn-coloured by the caller AND carrying the warning mark: colour is
/// never the sole signal (rule 6).
pub fn copy_text_headline_unreliable(failed: u64, total: u64) -> String {
    format!("⚠ {failed} of {total} character(s) could not be read and were replaced.")
}

/// Heading above the copy-result detail.
pub fn copy_text_detail_heading() -> &'static str {
    "About this copy:"
}

/// The routine, uncoloured note: whitespace pdfce added.
///
/// The wording deliberately says what the FILE does not contain rather
/// than what pdfce did wrong, because nothing went wrong — ISO 32000-1
/// requires no inter-word signal at all outside a Tagged PDF.
pub fn copy_text_derived_whitespace_note(spaces: u64, lines: u64) -> String {
    format!(
        "{spaces} space(s) and {lines} line break(s) in this copy were added by \
pdfce. The file itself does not record where words or lines end, so their \
positions are worked out from where the letters sit on the page."
    )
}

/// The warn-coloured note: characters the file gave pdfce no way to read.
pub fn copy_text_ladder_failures_note(failed: u64, total: u64) -> String {
    format!(
        "{failed} of {total} character(s) appear as a replacement mark. The fonts \
they were shown in carry no record of which characters their shapes stand for, so \
there is nothing to read them from — a limit of the file, not of pdfce."
    )
}

/// The one-number summary of how much of the copy is the document's own.
pub fn copy_text_sourced_percent(fraction: f64) -> String {
    format!(
        "{:.1}% of the characters came from the document.",
        fraction * 100.0
    )
}

/// Note for a whole-document copy where some pages held no text.
pub fn copy_text_pages_without_text_note(count: usize) -> String {
    format!("{count} page(s) held no text and contributed nothing.")
}

/// Title of the pre-copy confirmation.
pub fn copy_text_unreliable_title() -> &'static str {
    "Most of this text cannot be read"
}

/// Body of the pre-copy confirmation.
///
/// Asked BEFORE the clipboard is written, on the same reasoning as the
/// signature-invalidation question: an operator who is going to back out
/// should not first have their clipboard replaced with a wall of
/// replacement marks, destroying whatever they had copied before.
pub fn copy_text_unreliable_body(failed: u64, total: u64) -> String {
    format!(
        "{failed} of {total} character(s) here cannot be read. The fonts used carry \
no record of which characters their shapes stand for, so pdfce has nothing to read \
them from — copying will mostly produce replacement marks.\n\nCopying will replace \
whatever is on your clipboard now."
    )
}

/// Confirm button of the pre-copy confirmation.
pub fn copy_text_unreliable_confirm_button() -> &'static str {
    "Copy anyway"
}

/// Cancel button of the pre-copy confirmation.
pub fn copy_text_unreliable_cancel_button() -> &'static str {
    "Don't copy"
}

// ---------------------------------------------------------------------------
// Keyboard-shortcuts reference (P1-2)
// ---------------------------------------------------------------------------

/// Tooltip on the keyboard-shortcuts button.
pub fn shortcuts_tooltip() -> &'static str {
    "Show every keyboard shortcut"
}

/// Title of the keyboard-shortcuts window.
pub fn shortcuts_window_title() -> &'static str {
    "Keyboard shortcuts"
}

/// The full keyboard-shortcut reference, as one complete block (R2).
///
/// A single catalog entry rather than a per-row reuse of each control's
/// tooltip: the tooltips are full explanatory sentences (several are
/// multi-sentence paragraphs) that read poorly stacked as a reference,
/// and not every bound chord is named in a tooltip at all. This block is
/// the one authoritative place the chords are listed for the operator and
/// **must be kept in step with `collect_keyboard_actions` in `main.rs`**
/// whenever a binding changes — the same maintenance obligation a
/// chord-naming tooltip already carries.
pub fn shortcuts_reference() -> &'static str {
    "Open a file — Ctrl+O\n\
     Previous / next page — PageUp / PageDown\n\
     First / last page — Home / End\n\
     Zoom in / out — Ctrl+Plus / Ctrl+Minus\n\
     Actual size (100%) — Ctrl+0\n\
     Save a copy — Ctrl+S\n\
     Undo — Ctrl+Z\n\
     Redo — Ctrl+Y or Ctrl+Shift+Z\n\
     Rotate the current page left / right — [ / ]\n\
     Move the selected pages earlier / later — Alt+Up / Alt+Down\n\
     Clear the page selection — Esc\n\
     Delete the selected pages — Delete or Backspace\n\
     Copy this page's text — Ctrl+Shift+C\n\
     Zoom the page with the mouse — Ctrl+scroll wheel"
}

// ---------------------------------------------------------------------------
// Pass 14.3 — in-place page-text editing tool
// (docs/ui_specs/pass-14.3-text-edit-ui.md §1, §4.4, §6–§9, §11)
// ---------------------------------------------------------------------------

/// Toolbar toggle for the in-place text-edit tool (§1.1).
///
/// The "Aa" text suffix only; the pencil is now an SVG icon
/// ([`crate::icons::Icon::EditText`]) drawn beside it. The text is
/// deliberately KEPT rather than dropped for an icon-only button: it is
/// what goes **bold** while the tool is active, so the selected state
/// still never rests on colour alone (rule 6) — exactly as it did before
/// the icon swap.
pub fn edit_text_tool_button() -> &'static str {
    "Aa"
}

/// Tooltip on the text-edit tool toggle. The second sentence disambiguates
/// from the Markup / Text annotation authoring tools; the third (Pass 16.2
/// §10) closes the loop the other way, naming Add Text so the three adjacent,
/// easily-conflated text controls each point at the others (R78).
pub fn edit_text_tool_tooltip() -> &'static str {
    "Edit the words already on this page — fix a typo, resize, or recolour \
     existing text (Ctrl+E). To add a NEW comment or sticky note instead, use \
     Markup or Text. To add brand-new page text, use Add Text (Ctrl+Shift+E)."
}

/// Title of the text-edit tool's floating property bar (§7).
pub fn text_edit_propbar_title() -> &'static str {
    "Edit Text"
}

/// Property-bar label for the point-size control (§7).
pub fn format_size_label() -> &'static str {
    "Size (pt):"
}

/// Property-bar button that applies the chosen size to the caret's run (§7).
pub fn format_apply_size() -> &'static str {
    "Apply size"
}

/// Property-bar label preceding the colour-MODEL radios (§7). Names the fact
/// that pdfce STORES the chosen space rather than force-converting it.
pub fn format_color_model_label() -> &'static str {
    "Store colour as:"
}

/// Property-bar button that applies the chosen fill colour (§7).
pub fn format_apply_color() -> &'static str {
    "Apply colour"
}

/// Property-bar label preceding the font-family ComboBox (§7).
pub fn format_font_label() -> &'static str {
    "Font:"
}

/// Property-bar button that applies the chosen font family/style (§7).
pub fn format_apply_font() -> &'static str {
    "Apply font"
}

/// The trust-level tag for an embedded run font, in the family ComboBox (§7).
pub fn font_trust_embedded() -> &'static str {
    "embedded"
}

/// The trust-level tag for an operator-supplied face (--font-dir; §7).
pub fn font_trust_supplied() -> &'static str {
    "supplied"
}

/// The trust-level tag for a bundled Base-14 substitute (§7).
pub fn font_trust_bundled() -> &'static str {
    "bundled"
}

/// One family-ComboBox entry: the base font name plus its trust level (§7).
pub fn font_entry_label(base_font: &str, trust: &str) -> String {
    format!("{base_font} ({trust})")
}

/// The read-only block-boundary overlay toggle label (§9).
pub fn block_overlay_toggle() -> &'static str {
    "Show paragraph guides"
}

/// Tooltip on the block-overlay toggle (§9): names it a reviewable hint, not a
/// fact stated by the PDF.
pub fn block_overlay_toggle_tooltip() -> &'static str {
    "Show/hide the recognised paragraph and column boundaries pdfce inferred \
     (not stated by the PDF itself — a reviewable hint). Read-only in this \
     version."
}

/// The corner tag on a pending (uncommitted) edit's preview (§6.3), reusing
/// redaction's marked-vs-applied visual language.
pub fn preview_tag() -> &'static str {
    "PREVIEW — not yet applied"
}

/// The Accept control for a pending edit (§6.4).
pub fn accept_edit() -> &'static str {
    "✓ Accept"
}

/// The Reject control for a pending edit (§6.4).
pub fn reject_edit() -> &'static str {
    "✕ Reject"
}

/// Heading of the accepted-edit disclosure strip (§8.1).
pub fn disclosure_strip_title() -> &'static str {
    "Last edit"
}

/// Heading of the refusal strip (§8.2).
pub fn refusal_strip_title() -> &'static str {
    "Not applied"
}

/// One disclosure bullet, rendered verbatim from the core report (§8.1); the
/// ⓘ marker is paired with the text, never a colour alone (rule 6).
pub fn disclosure_bullet(text: &str) -> String {
    format!("ⓘ {text}")
}

/// One refusal line, rendered verbatim from the core error `Display` (§8.2);
/// the ✖ marker is paired with the text, never a colour alone (rule 6).
pub fn refusal_line(text: &str) -> String {
    format!("✖ {text}")
}

/// A core refusal (`err`, verbatim) joined with its fixed "what would lift it"
/// hint (§8.2) — a refusal is informative, never a dead end.
pub fn refusal_with_hint(err: &str, hint: &str) -> String {
    format!("{err} — {hint}")
}

/// Inline notice when a selection spans more than one text run (§4.4): a
/// refusal-with-reason shown BEFORE any core call.
pub fn cross_run_selection_notice() -> &'static str {
    "This selection spans more than one text run (e.g., a formatting change \
     partway through) — pdfce's first-cut editor edits one run at a time. \
     Narrow the selection to edit or format it."
}

// §8.2 "what would lift it" hint table — one entry per refusal family.

/// R-INV-1 (embedded-subset floor).
pub fn r_inv_1_hint() -> &'static str {
    "Supply this font via a font folder (Tools → Font folders) so pdfce can \
     use its full character set, or keep this edit to characters already on \
     the page."
}

/// R-INV-2/3/4 (symbolic / ToUnicode-only / composite).
pub fn r_inv_encoding_hint() -> &'static str {
    "This font's encoding can't be safely inverted for character-level \
     editing — not yet supported for this font type."
}

/// R-INV-6 (ligature-only).
pub fn r_inv_ligature_hint() -> &'static str {
    "This character exists only as part of a ligature in this font — try a \
     font where it has its own glyph."
}

/// R-INV-7 (code occupied).
pub fn r_inv_code_occupied_hint() -> &'static str {
    "This exact substitution isn't representable in this run's encoding."
}

/// R-INV-8 (beyond repertoire).
pub fn r_inv_repertoire_hint() -> &'static str {
    "This character is outside what a simple (non-Unicode-wide) font can \
     address here."
}

/// `FormatError::CoverageFailure`.
pub fn format_coverage_hint() -> &'static str {
    "Choose a font that includes every character in this selection, or supply \
     one via Tools → Font folders."
}

/// `FormatError::TargetFontMissing`.
pub fn format_target_missing_hint() -> &'static str {
    "This page has no resource for that font — pdfce edits existing page fonts \
     only in this first cut; embedding a new one is a planned fast-follow \
     (FF-C)."
}

/// Fallback hint for any refusal without a more specific entry.
pub fn edit_generic_hint() -> &'static str {
    "See the message above for the specific reason this edit was not applied."
}

// ---------------------------------------------------------------------------
// Pass 15.2 — within-block reflow sub-mode of the text-edit tool (§11 catalog)
// ---------------------------------------------------------------------------

/// The property-bar button that enters the reflow sub-mode, targeting the
/// paragraph containing the caret (§1.3).
pub fn reflow_button_label() -> &'static str {
    "Reflow paragraph…"
}

/// Tooltip on the enabled "Reflow paragraph…" button (§1.3/§11).
pub fn reflow_button_tooltip() -> &'static str {
    "Re-wrap this paragraph to a new width, alignment, or line spacing — \
     review the result before it's applied."
}

/// Tooltip when the reflow button is greyed because no paragraph holds the
/// caret (§1.3/§11).
pub fn reflow_disabled_no_block_tooltip() -> &'static str {
    "Click into a paragraph first — Reflow works on the paragraph containing \
     your cursor."
}

/// Tooltip when the reflow button is greyed because a single-run edit is
/// still pending Accept/Reject (§1.3/§1.4).
pub fn reflow_disabled_pending_tooltip() -> &'static str {
    "Finish or cancel the current edit first (Accept/Reject below)."
}

/// The recognition-divergence caption shown under the reflow button (§3) — a
/// GUI-authored honesty cue about the two internal recognitions, NOT a
/// verbatim core disclosure (§3/§13-item-3). Shown as background context, not
/// an alarm.
pub fn reflow_recognition_note() -> &'static str {
    "ⓘ Reflow may group these paragraph guides slightly differently than the \
     boundaries shown above, to keep centred, right-aligned, or justified \
     paragraphs whole."
}

/// Title of the reflow property-bar body (§4.2).
pub fn reflow_body_title() -> &'static str {
    "Reflow ¶"
}

/// Label preceding the wrap-width DragValue (§4.4).
pub fn reflow_width_label() -> &'static str {
    "Width (pt):"
}

/// Label preceding the alignment picker (§4.2/§6.2).
pub fn reflow_alignment_label() -> &'static str {
    "Align:"
}

/// Label preceding the leading (line-spacing) DragValue (§4.5).
pub fn reflow_leading_label() -> &'static str {
    "Line spacing (pt):"
}

/// The alignment caption when the detected value is being kept (§6.2). `align`
/// is the detected alignment's keyword.
pub fn reflow_detected_caption(align: &str) -> String {
    format!("ⓘ Detected: {align} (from the original layout)")
}

/// The alignment caption when no clear signal was found and Left was the
/// fallback (§6.2) — ⚠, an honest limitation, not ⓘ.
pub fn reflow_ambiguous_caption() -> &'static str {
    "⚠ No clear alignment signal in this paragraph — defaulted to Left"
}

/// The alignment caption once the operator has overridden the detected value
/// (§6.2). `detected`/`chosen` are alignment keywords.
pub fn reflow_overridden_caption(detected: &str, chosen: &str) -> String {
    format!("{detected} was detected — you changed this to {chosen}")
}

/// Tooltip on the on-canvas width drag-handle (§6.1).
pub fn reflow_width_handle_tooltip() -> &'static str {
    "Drag to change how wide this paragraph re-wraps — or type an exact width \
     above."
}

/// Small corner tag on the pre-reflow (current) block outline in the ghost
/// preview (§5.2 item 4).
pub fn reflow_current_tag() -> &'static str {
    "current"
}

/// Small label on the wrap-width guide line (§5.2 item 5).
pub fn reflow_wrap_width_label() -> &'static str {
    "wrap width"
}

/// The Accept control for a reflow under review (§7) — distinct wording from
/// the plain-edit `accept_edit`, since both can render in the same strip.
pub fn reflow_accept() -> &'static str {
    "✓ Accept reflow"
}

/// The Reject control for a reflow under review (§7).
pub fn reflow_reject() -> &'static str {
    "✕ Reject reflow"
}

// §7.3 reflow refusal "what would lift it" hints — one per named condition.

/// `ReflowError::EmptyBlock` (via `ReflowApplyError::Preview`).
pub fn reflow_empty_block_hint() -> &'static str {
    "This paragraph has no measurable text to reflow (it may be entirely \
     invisible or /ActualText-only) — nothing to re-wrap here."
}

/// `ReflowError::BadWidth` (via `ReflowApplyError::Preview`).
pub fn reflow_bad_width_hint() -> &'static str {
    "Choose a positive width — drag the handle back onto the page, or type a \
     value above zero."
}

/// `ReflowApplyError::Unsupported` when the page was already text-edited this
/// session (15.1 judgment call #6) — a named, non-alarming condition.
pub fn reflow_already_edited_hint() -> &'static str {
    "If the reason above is that this page was already edited this session, \
     save and reopen the document, then reflow. Otherwise this paragraph's \
     layout (e.g. rotated, shared, or non-contiguous text) can't be \
     reflowed in this cut."
}

/// `ReflowApplyError::Refused` (composite/CJK font — R-INV-4).
pub fn reflow_font_refused_hint() -> &'static str {
    "This paragraph's font can't be reflowed in this cut (a multi-byte / \
     composite font) — supported for simple fonts first."
}

/// Fallback hint for any reflow refusal without a more specific entry.
pub fn reflow_generic_hint() -> &'static str {
    "See the message above for the specific reason this reflow was not applied."
}

// ---------------------------------------------------------------------------
// Pass 16.2 — the "Add Page Text" tool (on-canvas, decision 016 / FF-D)
// (docs/ui_specs/pass-16.2-add-text-ui.md §1, §3.3, §5, §6, §7, §10)
// ---------------------------------------------------------------------------

/// Toolbar toggle for the Add-Page-Text tool (§1.2). A real, distinct glyph:
/// the SAME "Aa" root Edit Text uses (family membership — both act on the
/// page's actual text) with a `+` prefix (the DIFFERENT operation: add vs.
/// modify), never the pencil, never a colour variant (rule 6).
pub fn add_text_tool_button() -> &'static str {
    "Aa"
}

/// Tooltip on the Add-Page-Text tool toggle (§1.2). Three sentences, each doing
/// one job: what it does, what it is NOT (the R78 disambiguator, naming the
/// competing Text control by its own visible label), and where the third
/// related-but-different tool (Edit Text) lives.
pub fn add_text_tool_tooltip() -> &'static str {
    "Add brand-new text to the page itself — a label, caption, or note that becomes real, \
permanent page content, exactly like the text already here (Ctrl+Shift+E). This is NOT the same \
as Text → Text box (a removable annotation) — for a comment or sticky note instead, use \
Text. To fix text that's already on the page, use Edit Text (Ctrl+E)."
}

/// Title of the Add-Text tool's floating property bar (§3.3/§5.2).
pub fn add_text_propbar_title() -> &'static str {
    "Add Text"
}

/// Label on the Pass 9c-min Edit Objects (vector-edit) tool toggle
/// (decision 011 §2.5). A distinct glyph from the text tools: a move-arrows
/// motif, since the tool's headline gesture is moving/nudging drawing
/// objects and their nodes.
pub fn vector_edit_tool_button() -> &'static str {
    "Obj"
}

/// Tooltip on the Edit Objects tool toggle. Names the three gestures and the
/// honesty caveat that Delete here removes a drawing object but is NOT
/// redaction (it does not securely remove covered content — that is
/// Redact).
pub fn vector_edit_tool_tooltip() -> &'static str {
    "Edit vector drawing objects on the page: click to select, drag to move the object, drag an \
anchor to move that node, or press Delete to remove it. Each edit is undoable. NOTE: Delete removes \
a drawing object from the page — it is NOT redaction (it does not securely remove covered content; \
use Redact for that)."
}

/// The transient note shown after a canvas object is deleted (Pass 9c-min).
pub fn vector_object_deleted() -> &'static str {
    "Deleted the selected object. This is undoable, and is NOT redaction — to securely remove \
content, use Redact instead."
}

/// The "Measure" toolbar menu label (Pass 12.M2 ui-spec §1.2). A menu, not
/// four toolbar icons, because dimensioning is used in short deliberate bursts
/// (rule 3 — avoid primary-toolbar icon creep).
pub fn measure_menu_button() -> &'static str {
    "Measure"
}

/// The active-tool names substituted into [`measure_menu_active_label`].
///
/// These are OPERATOR-VISIBLE strings — they appear on the toolbar as
/// "Measure: Linear" — and they previously lived as bare literals in
/// `main.rs`'s tool-name match. That is a decision 002 / R1 violation, and it
/// is instructive that it survived: the CI gate meant to catch exactly this
/// was failing at baseline on 140 unrelated hits, so nobody could see the one
/// real finding in the noise. A gate that cannot pass guards nothing.
///
/// Note "Linear" and "Radius/Diameter" would NOT have been caught even by a
/// green gate — its heuristic only flags literals containing whitespace, and
/// those two have none. They are moved here anyway, because the rule is
/// "operator-visible strings live in the catalog", not "strings the grep can
/// see live in the catalog".
pub fn measure_tool_name_linear() -> &'static str {
    "Linear"
}

/// See [`measure_tool_name_linear`].
pub fn measure_tool_name_circular() -> &'static str {
    "Radius/Diameter"
}

/// See [`measure_tool_name_linear`].
pub fn measure_tool_name_scale() -> &'static str {
    "Set Scale"
}

/// Appended to a menu button's ACCESSIBLE NAME (not its visible label) so a
/// screen-reader user learns the control opens a menu rather than performing
/// an action: "Markup, opens a menu".
///
/// # Why this string has to exist
///
/// egui's `WidgetType` has no menu/has-popup role, and `Ui::menu_button` sets
/// no `WidgetInfo` override, so "this opens a menu" cannot reach assistive
/// technology structurally — only as literal text.
///
/// Sighted users used to get that meaning from a `▾` appended to the label.
/// That glyph (U+25BE) is in none of the fonts in egui's default Proportional
/// chain, so it rendered as a tofu box; it is now drawn as a real chevron icon
/// instead (`icons::menu_chevron`). But an image is decorative and announces
/// nothing — so simply deleting the glyph and drawing a picture would have
/// made the control *less* accessible than the bug it fixed, since a tofu box
/// at least carries a Unicode name some readers speak aloud.
///
/// The visible cue and the announced cue are therefore supplied separately and
/// deliberately: the chevron for the eye, this suffix for the ear. See
/// `docs/ui_specs/menu-affordance-and-glyph-coverage.md` §3.
pub fn menu_button_accessible_suffix() -> &'static str {
    "opens a menu"
}

/// A menu button's full accessible name: its visible label plus
/// [`menu_button_accessible_suffix`].
pub fn menu_button_accessible_name(label: &str) -> String {
    format!("{label}, {}", menu_button_accessible_suffix())
}

/// The Measure menu's dynamic-label prefix shown when a measure tool is active
/// (ui-spec §1.2): "Measure: Linear ▾" etc., so the active tool is never hidden
/// by the menu's closed state.
pub fn measure_menu_active_label(tool_name: &str) -> String {
    format!("Measure: {tool_name}")
}

/// Tooltip on the Measure menu (§1.2 / §7.1) — states what it does and when to
/// reach for it versus the annotation-authoring Markup menu.
pub fn measure_menu_tooltip() -> &'static str {
    "Add scaled measurement dimensions — linear distances, radius/diameter from a best-fit \
circle, and per-group scale calibration. Dimensions are additive annotations on their own \
toggleable layer; the value updates when you change the group's scale. This is measurement, \
not drawing — for shapes and callouts use Markup."
}

/// The Measure menu's Linear-Dimension row (§1.2).
pub fn measure_linear_menu_item() -> &'static str {
    "Linear Dimension"
}

/// The Measure menu's Radius/Diameter row (§1.2). One tool — the display toggle
/// between radius and diameter is on the same best-fit geometry (§1.1).
pub fn measure_circular_menu_item() -> &'static str {
    "Radius / Diameter Dimension"
}

/// The Measure menu's Set-Group-Scale row (§1.2 / §4).
pub fn measure_set_scale_menu_item() -> &'static str {
    "Set Group Scale…"
}

/// The Measure menu's "Manage Dimension Groups…" row (Pass 12.M2b ui-spec §5.1)
/// — opens the modeless group panel; does not change the active tool.
pub fn measure_manage_groups_menu_item() -> &'static str {
    "Manage Dimension Groups…"
}

/// One-line hint at the top of the Measure property bar for the active tool
/// (Pass 12.M2b), stating the gesture (ui-spec §2.1/§3.2/§4.1).
pub fn measure_linear_hint() -> &'static str {
    "Click two points to measure a distance. Snap locks onto nearby geometry; press Tab to \
cycle candidates, hold Alt to skip snapping. Then Accept."
}

/// Circular-tool hint (ui-spec §3).
pub fn measure_circular_hint() -> &'static str {
    "Click a circle, or several line segments forming an arc, to best-fit a circle. Click again \
to add/remove. The fit and its residual preview live; then Accept."
}

/// Scale-tool hint (ui-spec §4).
pub fn measure_scale_hint() -> &'static str {
    "Draw a reference line of known real length, then enter that length (or a ratio) to set the \
group's scale. Every dimension in the group updates."
}

/// The property-bar "Group:" active-group picker label (ui-spec §2.6).
pub fn measure_group_label() -> &'static str {
    "Group:"
}

/// The property-bar button opening the group panel (ui-spec §5.1).
pub fn measure_open_groups_button() -> &'static str {
    "Groups…"
}

/// The linear/scale alignment-constraint segmented-control label (ui-spec §2.5).
pub fn measure_alignment_label() -> &'static str {
    "Alignment:"
}

/// The human label for an [`AxisConstraint`] segmented-control button
/// (ui-spec §2.5): Aligned (free), Horizontal (page X), Vertical (page Y).
pub fn axis_constraint_label(c: AxisConstraint) -> &'static str {
    match c {
        AxisConstraint::Aligned => "Aligned",
        AxisConstraint::Horizontal => "Horizontal",
        AxisConstraint::Vertical => "Vertical",
    }
}

/// The circular display-toggle label (ui-spec §3.4).
pub fn measure_display_label() -> &'static str {
    "Display:"
}

/// The circular "Radius" display option (ui-spec §3.4).
pub fn measure_radius_option() -> &'static str {
    "Radius"
}

/// The circular "Diameter" display option (ui-spec §3.4).
pub fn measure_diameter_option() -> &'static str {
    "Diameter"
}

/// The measure-tool Accept button (ui-spec §2.6) — authors the dimension /
/// commits the scale as ONE undoable command.
pub fn measure_accept() -> &'static str {
    "Accept"
}

/// The measure-tool Reject button (ui-spec §2.6) — discards the in-progress
/// gesture; nothing was written (rule 7).
pub fn measure_reject() -> &'static str {
    "Reject"
}

/// The live length readout while a linear dimension is in progress (ui-spec
/// §2.6): the raw page-space length and the scaled value side by side
/// (`12.40 pt → 3.10 m`), or the raw value alone when the group has no scale.
/// `scaled` is the group-formatted display string; `raw_units` is whether it is
/// already a raw page-units reading (⇒ the arrow form would be redundant).
pub fn measure_length_readout(raw_points: f64, scaled: &str, raw_units: bool) -> String {
    if raw_units {
        format!("{raw_points:.2} pt ({scaled})")
    } else {
        format!("{raw_points:.2} pt \u{2192} {scaled}")
    }
}

/// The best-fit-circle disclosure (ui-spec §3.4 / §6): "Best-fit circle from N
/// objects — radius R, fit residual ε (RMS)." Always shown while a fit exists,
/// the residual surfaced so the operator sees fit quality (decision 011 §2.3).
pub fn best_fit_circle_disclosure(count: usize, radius: f64, residual: f64) -> String {
    format!(
        "Best-fit circle from {count} object(s) — radius {radius:.2} pt, fit residual {residual:.2} pt (RMS)."
    )
}

/// The warn-coloured pairing shown when a best-fit's residual is large relative
/// to its radius (ui-spec §3.4 — colour is never the sole signal, rule 6).
pub fn best_fit_residual_high() -> &'static str {
    "The fit is loose — the picked geometry is not very circular."
}

/// The scale-entry sub-panel's drawn-reference-length caption (ui-spec §4.2).
pub fn scale_entry_drawn_length(pdf_length: f64) -> String {
    format!("Drawn reference length: {pdf_length:.1} pt")
}

/// The scale-entry real-length path label (recommended, ui-spec §4.2).
pub fn scale_entry_real_length_label() -> &'static str {
    "Real length of this line  (recommended)"
}

/// The scale-entry direct-ratio path label (ui-spec §4.2).
pub fn scale_entry_ratio_label() -> &'static str {
    "Direct ratio (paper : real)"
}

/// The `:` separator between the ratio's paper and real drag values (ui-spec
/// §4.2). A catalog entry (not an inline literal) so the whole scale-entry
/// sub-form's text lives in one place, per R1.
pub fn ratio_colon() -> &'static str {
    ":"
}

/// The scale-entry paper-unit-basis caption, always shown (ui-spec §4.2).
pub fn scale_entry_paper_basis_caption() -> &'static str {
    "Paper-unit basis: 1 in = 72 pt."
}

/// The scale-entry live preview (ui-spec §4.2 "→ scale = 1:100"). `ratio` is
/// the engine-computed `ScalePreview::ratio_label`, rendered verbatim.
pub fn scale_entry_preview(ratio: &str) -> String {
    format!("\u{2192} scale = {ratio}")
}

/// The group-panel window title (ui-spec §5.1).
pub fn group_manager_title() -> &'static str {
    "Dimension Groups"
}

/// The group-panel "+ New Group" button (ui-spec §5.2).
pub fn group_new_group_button() -> &'static str {
    "+ New Group"
}

/// Placeholder/label for the new-group name field (ui-spec §5.2).
pub fn group_new_group_name_label() -> &'static str {
    "Name:"
}

/// One group row's summary line (ui-spec §5.2): name, its scale summary, and
/// its member count.
pub fn group_row_summary(name: &str, scale_summary: &str, count: usize) -> String {
    format!("{name} — {scale_summary} — {count} dim(s)")
}

/// The "(hidden)" suffix on a group row whose layer is off (ui-spec §5.2 —
/// paired with the greyed styling, never the eye glyph alone, rule 6).
pub fn group_hidden_suffix() -> &'static str {
    "(hidden)"
}

/// The per-group "Set scale…" button that expands the inline scale editor
/// (ui-spec §5.2).
pub fn group_set_scale_button() -> &'static str {
    "Set scale…"
}

/// The inline group-scale editor's Apply button (ui-spec §5.2).
pub fn group_apply_button() -> &'static str {
    "Apply"
}

/// The inline group-scale editor's Cancel button (ui-spec §5.2).
pub fn group_cancel_button() -> &'static str {
    "Cancel"
}

/// The per-group layer visibility toggle button label, given the current
/// visibility (ui-spec §5.2 — show/hide the group's OCG layer).
pub fn group_visibility_button(visible: bool) -> &'static str {
    if visible { "Hide layer" } else { "Show layer" }
}

/// A human scale summary for a group (ui-spec §4.3 tri-state), for the group
/// row + the property bar: never-set, explicit 1:1, or a calibrated ratio.
pub fn group_scale_summary(scale: pdfce_core::dimension::ScaleState, unit: Unit) -> String {
    use pdfce_core::dimension::ScaleState;
    match scale {
        ScaleState::NeverSet => "no scale set".to_owned(),
        ScaleState::OneToOne => "1:1 (set by operator)".to_owned(),
        ScaleState::Calibrated { scale } if scale > 0.0 => {
            // Report as "1 <unit> = <pt> pt" — the inverse of the per-point
            // factor, the architectural reading (ui-spec §5.2 example).
            format!("1 {} = {:.2} pt", unit.token(), 1.0 / scale)
        }
        ScaleState::Calibrated { .. } => "calibrated".to_owned(),
    }
}

/// The unit-dropdown display label for a [`Unit`] (ui-spec §5.2 units menu).
pub fn unit_dropdown_label(u: Unit) -> &'static str {
    match u {
        Unit::Millimeter => "mm",
        Unit::Centimeter => "cm",
        Unit::Meter => "m",
        Unit::Inch => "in",
        Unit::DecimalFeet => "decimal ft",
        Unit::FeetInches => "ft-in",
    }
}

/// The status-strip confirm shown while a derived-centerline candidate is the
/// active snap target (ui-spec §2.3.1): the fuzzy inference needs a second
/// click to confirm it is a drawn line, not a rectangle (never auto-applied).
pub fn measure_confirm_derived_centerline() -> &'static str {
    "Derived centerline (a line drawn as a filled shape) — click again to confirm it is a \
drawn line, not a rectangle."
}

/// The post-Accept disclosure after authoring a dimension (Pass 12.M2b): the
/// dimension is additive on its group's toggleable layer, one undo step.
pub fn measure_dimension_authored(group_name: &str) -> String {
    format!("Dimension added to group '{group_name}' — additive, on its own layer, one undo step.")
}

/// The post-Accept disclosure after a scale calibration re-propagated to a
/// group's members (ui-spec §4.4 / §6): "Scale applied to '<name>' — N
/// dimension(s) updated."
pub fn measure_scale_applied(group_name: &str, updated: usize) -> String {
    format!("Scale applied to '{group_name}' — {updated} dimension(s) updated.")
}

/// A one-line hint under the property-bar title (§3): how to place, and the
/// §7.2 discoverability cue that the added run becomes ordinary page text.
pub fn add_text_hint() -> &'static str {
    "Click the page to place a single line, or drag to make a wrap box. Type your text, then \
Add. Once added it becomes ordinary page text — switch to Edit Text (Ctrl+E) to change it."
}

/// Label preceding the manual origin X/Y entry fields (§3.3) — the keyboard-
/// reachable placement path, so placing text is not pointer-only.
pub fn add_text_origin_label() -> &'static str {
    "Origin X, Y (pt):"
}

/// Label preceding the manual box width/height entry fields (§3.3, box half).
pub fn add_text_box_size_label() -> &'static str {
    "Box W, H (pt):"
}

/// The "Use a box instead" checkbox in the manual-entry row (§3.3).
pub fn add_text_use_box_checkbox() -> &'static str {
    "Use a box instead"
}

/// The keyboard-entry button that places a POINT draft at the typed origin
/// (§3.3).
pub fn add_text_place_point_button() -> &'static str {
    "Place point"
}

/// The keyboard-entry button that places a BOX draft at the typed origin/size
/// (§3.3).
pub fn add_text_place_box_button() -> &'static str {
    "Place box"
}

/// Property-bar label preceding the alignment picker (box mode only, §5.2).
pub fn add_text_align_label() -> &'static str {
    "Align:"
}

/// Property-bar label preceding the leading (line-spacing) field (box mode
/// only, §5.2). Names the `0 = auto` sentinel so a blank/zero value reads as
/// "use the derived 1.2× default" rather than an error.
pub fn add_text_leading_label() -> &'static str {
    "Line spacing (pt, 0 = auto):"
}

/// The Accept control for an add-text draft (§7.1) — distinct wording from
/// Edit Text's `accept_edit`/reflow's `reflow_accept`, since several review
/// controls can be visible across a session (15.2 §11's reasoning).
pub fn add_text_accept() -> &'static str {
    "✓ Add"
}

/// The Reject control for an add-text draft (§7.3).
pub fn add_text_reject() -> &'static str {
    "✕ Cancel"
}

/// Tooltip on the disabled Accept button while the draft is empty (§7.1/§6.3):
/// there is nothing to add yet.
pub fn add_text_empty_tooltip() -> &'static str {
    "Type some text first."
}

/// Label for the Font-folders panel's "default font for new page text"
/// preference control (§5.1) — a per-operator default, seeded into each new
/// draft and overridable per-use.
pub fn add_text_default_font_label() -> &'static str {
    "Default font for new page text:"
}

/// The §7.2 P1 continuity link shown in the disclosure strip after an accepted
/// add: switches to the Edit Text tool so the just-added run can be re-edited.
pub fn edit_this_text_now_button() -> &'static str {
    "Edit this text now →"
}

// §6.2 add-text "what would lift it" hint table — one entry per named
// `AddTextError` condition (the R-INV font-refusal triggers reuse 14.3's
// existing `r_inv_*_hint` functions, keyed by trigger, rather than a copy).

/// `AddTextError::InvalidSize`.
pub fn add_text_invalid_size_hint() -> &'static str {
    "Choose a positive font size."
}

/// `AddTextError::InvalidBox` (box mode).
pub fn add_text_invalid_box_hint() -> &'static str {
    "Draw or type a box with a positive width and height."
}

/// `AddTextError::NoWordsToWrap` (box mode, whitespace-only).
pub fn add_text_no_words_hint() -> &'static str {
    "Type at least one non-space word to wrap into the box."
}

/// `AddTextError::Encrypted`.
pub fn add_text_encrypted_hint() -> &'static str {
    "Adding text to an encrypted document is out of scope for this release."
}

/// `AddTextError::HiddenObjects`.
pub fn add_text_hidden_objects_hint() -> &'static str {
    "This file's cross-reference table currently hides some entries in a way adding new objects \
would expose — this is a rare structural limitation, not something to work around by retrying."
}

/// The reachable-but-should-not-happen internal-consistency conditions
/// (`PageIndex`/`EmptyText` — Accept is already gated on a non-empty draft on
/// the current page), framed as a bug rather than an operator action (§6.2).
pub fn add_text_internal_bug_hint() -> &'static str {
    "This looks like an internal inconsistency rather than something you did — please report the \
document and what you were adding."
}

/// Fallback hint for any add-text refusal without a more specific entry
/// (structural/save failures render their own `Display` verbatim above it).
pub fn add_text_generic_hint() -> &'static str {
    "See the message above for the specific reason this text was not added."
}

// ---------------------------------------------------------------------------
// Snapping engine — fuzzy snap indicator (Pass 12.M1)
// ---------------------------------------------------------------------------
//
// The marker glyph + type label a measure tool (Pass 12.M2) shows at the
// current snap candidate BEFORE the click commits (`docs/ui_specs/
// pass-12.M2-dimension-tools.md` §2.2). A DISTINCT glyph/label is reserved
// for the derived filled-quad centerline (§2.3.1), the one fuzzy inference
// that carries an extra two-click confirm. Shape distinguishes kind so colour
// is never the sole signal (rule 6). The glyphs are single Unicode marks
// rendered inline; the label is the human name of the snapped geometry.

/// The snap-indicator marker glyph for a candidate of `kind` — the small mark
/// drawn AT the candidate point (Pass 12.M2 §2.2). Shape (not colour) carries
/// the meaning (rule 6). The derived-centerline glyph is deliberately distinct
/// from the routine centerline tick so the operator sees at a glance that that
/// one candidate needs the extra confirm (§2.3.1).
#[allow(
    dead_code,
    reason = "Pass 12.M1 indicator glyph catalog; drawn by the Pass 12.M2 measure tools' overlay (spec 2.2)"
)]
pub fn snap_glyph(kind: SnapKind) -> &'static str {
    match kind {
        SnapKind::Node => "\u{25FC}",              // ◼ filled square
        SnapKind::Endpoint => "\u{25CF}",          // ● filled circle
        SnapKind::Center => "\u{2295}",            // ⊕ crosshair-in-circle
        SnapKind::Midpoint => "\u{25B2}",          // ▲ small triangle
        SnapKind::Intersection => "\u{2715}",      // ✕ cross
        SnapKind::DerivedCenterline => "\u{25A4}", // ▤ hatch square (distinct)
        SnapKind::SegmentCenterline => "\u{2504}", // ┄ dashed tick
        SnapKind::Axis => "\u{229E}",              // ⊞ grid glyph
    }
}

/// The snap-indicator type label beside the glyph (Pass 12.M2 §2.2): the human
/// name of the snapped geometry, so the operator sees exactly what was inferred
/// before committing (fuzzy-never-sneaky). The derived-centerline label carries
/// the "(unconfirmed)" qualifier the routine kinds do not (§2.3.1).
#[allow(
    dead_code,
    reason = "Pass 12.M1 indicator label catalog; drawn by the Pass 12.M2 measure tools' overlay (spec 2.2)"
)]
pub fn snap_indicator_label(kind: SnapKind) -> &'static str {
    match kind {
        SnapKind::Node => "node",
        SnapKind::Endpoint => "endpoint",
        SnapKind::Center => "center",
        SnapKind::Midpoint => "midpoint",
        SnapKind::Intersection => "intersection",
        SnapKind::DerivedCenterline => "derived centerline (unconfirmed)",
        SnapKind::SegmentCenterline => "centerline",
        SnapKind::Axis => "axis",
    }
}

/// The persistent master "Snap to content" toggle label (Pass 12.M2 §2.4) —
/// the property-bar checkbox that turns the whole snap query on/off. Default
/// on; off, every pick is the raw pointer position.
#[allow(
    dead_code,
    reason = "Pass 12.M1 master-toggle label; rendered in the Pass 12.M2 measure property bar (spec 2.4)"
)]
pub fn snap_toggle_label() -> &'static str {
    "Snap to content"
}

/// Tooltip for the master snap toggle — states what it does and how to override
/// it transiently (the Alt-hold suppression and Tab-cycle affordances,
/// §2.4), following the catalog's "say when to use it" tooltip convention.
#[allow(
    dead_code,
    reason = "Pass 12.M1 master-toggle tooltip; rendered in the Pass 12.M2 measure property bar (spec 2.4)"
)]
pub fn snap_toggle_tooltip() -> &'static str {
    "Snap measurement picks to nearby nodes, endpoints, centers, midpoints and lines. \
Hold Alt to suppress snapping for one pick; press Tab to cycle competing candidates."
}

/// The two-click-confirm disclosure shown when the current snap candidate is a
/// DERIVED centerline (a line drawn as a filled rectangle), Pass 12.M2 §2.3.1.
/// `aspect_ratio` is the measured long:short ratio the derivation was made on,
/// surfaced so the operator sees how line-like the shape is before confirming.
#[allow(
    dead_code,
    reason = "Pass 12.M1 derived-centerline confirm disclosure; shown by the Pass 12.M2 measure tools (spec 2.3.1)"
)]
pub fn snap_derived_centerline_confirm(aspect_ratio: f64) -> String {
    format!(
        "Centerline derived from a filled shape (long:short \u{2248} {aspect_ratio:.1}:1) — \
click again to confirm this is a drawn line, not a rectangle."
    )
}
