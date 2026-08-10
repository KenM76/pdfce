//! # icons — the SVG-path → tiny-skia → egui-texture toolbar icon pipeline
//!
//! Turns the outline SVGs in `crates/pdfce-gui/assets/icons/*.svg` into
//! tinted, DPI-correct egui images for the toolbar, menus and Tools dock,
//! implementing `docs/ui_specs/icon-set-and-toolbar.md` (referred to below
//! as "the ui-spec"). Nothing here knows what any icon *means* — the
//! meaning lives in [`Icon`]'s variant names and in the ui-spec's §2
//! mapping table; this module only knows how to turn a path `d` attribute
//! into pixels and how not to do it twice.
//!
//! ## Why a hand-rolled SVG subset parser instead of a crate
//!
//! egui renders no vector art natively, so an SVG has to become a raster
//! somewhere. The three candidate pipelines and why this one won:
//!
//! 1. **A runtime SVG crate (`resvg`/`usvg`).** Correct and general, but a
//!    NEW Cargo dependency — and `resvg` is MPL-2.0 (weak copyleft). Project
//!    rule 13 forbids an agent adding any dependency solo, copyleft or not,
//!    and the ui-spec §7.1 flagged it as an explicit operator go/no-go.
//!    Rejected by the operator (2026-08-02).
//! 2. **Pre-rasterize to PNG at build time.** Zero dependencies, but the
//!    resolution is baked: a raster sized for a 28x24 button at 100% display
//!    scale is visibly soft at 150%/200% Windows scaling, and would be wrong
//!    again for any future "larger toolbar icons" accessibility option. It
//!    was also *not executable on this machine* — there is no SVG rasterizer
//!    installed (no Inkscape, no ImageMagick, and `cairosvg`'s libcairo fails
//!    to load), so the conversion step had no tool to run.
//! 3. **Parse the path data ourselves and stroke it with `tiny-skia`** —
//!    what this module does. `tiny-skia` is ALREADY a dependency, reachable
//!    from `pdfce-gui` as `pdfce_render::tiny_skia` (the same re-export
//!    [`crate::object_provider`] uses), so this adds **zero** new crates. It
//!    rasterizes at whatever physical pixel size the current display scale
//!    implies, so icons are crisp at any DPI — strictly better than (2) and
//!    free of (1)'s licensing question.
//!
//! The cost of (3) is that a subset SVG parser now exists in this repo, and
//! a parser that silently mis-reads its input is worse than no parser at
//! all. That risk is contained two ways: the parser **refuses** rather than
//! guesses (see below), and [`tests::every_icon_parses`] parses every
//! shipped asset, so a malformed or out-of-subset icon fails the build's
//! test gate instead of shipping as a wrong glyph.
//!
//! ## What the parser supports, and what it refuses
//!
//! This is deliberately NOT a general SVG implementation. It reads exactly
//! the shape of file the ui-spec §1 style contract describes:
//!
//! * **Elements:** `<svg>` (opened/closed, attributes ignored), XML comments,
//!   `<path>`, `<rect>`, `<circle>`. **Any other element is an error** —
//!   `<g>`, `<use>`, `<text>`, `<defs>`, gradients, transforms and CSS are
//!   all out of subset and rejected loudly, never skipped, because silently
//!   skipping a `<g transform=...>` would draw a correctly-shaped glyph in
//!   the wrong place.
//! * **Attributes:** `d`, `x`, `y`, `width`, `height`, `rx`, `cx`, `cy`,
//!   `r`, `stroke`, `fill`, `stroke-width`, `stroke-linecap`,
//!   `stroke-linejoin`. Unknown attributes are ignored (they are cosmetic
//!   metadata like `aria-hidden`/`xmlns`, never geometry).
//! * **Paint:** `stroke="currentColor"` strokes; `stroke="none"`/absent does
//!   not. `fill="currentColor"` fills; `fill="none"`/absent does not. The
//!   colour VALUE is discarded — see "theming" below — but its presence or
//!   absence decides whether the shape is drawn at all, so
//!   `stroke="currentColor"` and `stroke="none"` are not interchangeable.
//!   `stroke-linecap` accepts `butt`/`round`/`square`; `stroke-linejoin`
//!   accepts `miter`/`round`/`bevel`. Any other value is an error rather
//!   than a silent fallback to the default, because a wrong cap on a
//!   2.5-unit stroke at 16 px is a visible defect that would otherwise ship
//!   unnoticed.
//! * **Path commands:** the complete SVG path grammar —
//!   `M m L l H h V v C c S s Q q T t A a Z z`. Implementing the whole
//!   grammar rather than only the commands today's assets happen to use
//!   costs a few dozen lines and removes a whole class of future failure
//!   (an icon redrawn with a smooth-quadratic `T` two years from now must
//!   not become a build break). Anything that is not one of those letters
//!   is [`IconError::UnsupportedPathCommand`].
//! * **Numbers:** the SVG number grammar including implicit separators and
//!   packed arc flags — `a6 6 0 008 8` really does mean
//!   `rx=6 ry=6 rot=0 large-arc=0 sweep=0 dx=8 dy=8`, and `icon-link.svg`
//!   in the source set is written exactly that way. Arc flags are therefore
//!   lexed as a SINGLE `0`/`1` character, never as a general number.
//!
//! Every failure mode returns an [`IconError`] carrying enough context to
//! find the offending byte; nothing falls back to "draw something".
//!
//! ### Arcs
//!
//! `tiny-skia` has no arc primitive, so elliptical-arc (`A`/`a`) segments
//! are converted to cubic Béziers by [`arc_to_cubics`], following the
//! endpoint→centre parameterization in the SVG 1.1 implementation notes
//! (F.6.5) and the ≤90°-per-segment subdivision of F.6.6. Two shipped
//! icons depend on this (`tool.svg`'s wrench jaw, `link.svg`'s chain
//! links), as do the 270° history and rotate arrows.
//!
//! ## Theming: one raster per icon, tinted at draw time (ui-spec §6)
//!
//! Every asset is `stroke="currentColor"` — a single-colour outline with no
//! palette. So each icon is rasterized ONCE as a **white-on-transparent
//! coverage mask**, and the colour is applied by egui at draw time via
//! [`egui::Image::tint`]. Consequences, all of them deliberate:
//!
//! * Light theme, dark theme, hovered and disabled all share ONE raster.
//!   There are no light/dark asset pairs to keep in sync, and structurally
//!   no way for an icon to end up hardcoded-black on a dark background.
//! * The tint is therefore **not** part of the cache key ([`CacheKey`]) —
//!   that is the entire point of the mask. Re-tinting is free; re-rastering
//!   is not.
//! * Disabled controls need no fade logic here at all. The tint is read
//!   from `ui.visuals().text_color()` *inside* whatever `Ui` scope the
//!   caller is in (ui-spec §5.2), and egui's own
//!   `Ui::disable()`/`add_enabled_ui(false, …)` additionally multiplies the
//!   painter's opacity, which applies to textured meshes exactly as it
//!   applies to text — so an icon fades precisely the way the text buttons
//!   beside it already do.
//!
//! Because the mask is white (255,255,255,a) premultiplied to (a,a,a,a),
//! egui's multiplicative tint yields (a·Tr, a·Tg, a·Tb, a) — a correctly
//! premultiplied, correctly antialiased tinted glyph, for any tint.
//!
//! ## Weight, and why selected state is not colour alone
//!
//! [`IconWeight::Bold`] rasterizes the same art with the stroke width
//! multiplied by [`BOLD_STROKE_FACTOR`]. It exists because of standing rule
//! "selected state is never colour alone" (project rule 6 / P1-1, restated
//! in ui-spec §5.3 as the one place an icon swap can silently regress an
//! existing guarantee). Text toggles satisfy that rule by going **bold**;
//! an icon has no text to embolden, so the *glyph* goes bold instead. The
//! toolbar pairs that weight cue with an explicit outline ring
//! (`PdfceApp::selected_icon_ring`), giving a selected icon-only control
//! three simultaneous cues — fill, ring, weight — of which two are not
//! colour.
//!
//! ## Caching, and why it is a thread-local
//!
//! The toolbar re-runs every frame (60/s), and rasterizing ~30 icons per
//! frame would be absurd, so [`IconCache`] memoizes the uploaded
//! [`egui::TextureHandle`] per [`CacheKey`] = (icon, physical pixel size,
//! weight). Nothing is rasterized twice unless the display scale changes or
//! a control becomes selected.
//!
//! The cache is reached through a `thread_local!` rather than a field on
//! `PdfceApp`, for a concrete borrow-checker reason: the toolbar body holds
//! `&self.status` (the open document) across almost its whole length while
//! *also* taking `&mut self.markup_color` inside menu closures. Threading a
//! `&mut IconCache` through that would force those disjoint field borrows
//! into a whole-`self` borrow and not compile; an interior-mutable global
//! sidesteps it with no behavioural cost, because the cache is pure
//! memoization — evicting it changes performance, never pixels. eframe runs
//! the UI on a single thread, so per-thread is per-app in practice, and a
//! second thread would simply get its own (unused) cache rather than a data
//! race.
//!
//! [`IconCache`] itself is public and constructible so the caching contract
//! is unit-testable without the thread-local (see [`tests`]).
//!
//! ## DPI
//!
//! Icons are laid out in **logical points** ([`ICON_PTS`]) but rasterized at
//! `ICON_PTS * ctx.pixels_per_point()` **physical pixels**, then drawn back
//! at the logical size. Rasterizing at the logical size instead would make
//! every icon visibly soft on any HiDPI display (a 16 px raster stretched
//! over 32 device pixels at 200% scale). Because the physical size is part
//! of the cache key, a display-scale change re-rasterizes automatically
//! rather than reusing a stale, wrongly-sized texture.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;

use eframe::egui;
use pdfce_render::tiny_skia::{
    Color, FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform,
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// The viewBox edge length every asset uses (`viewBox="0 0 48 48"`,
/// ui-spec §1). All geometry in the SVGs is in these units; the rasterizer
/// scales by `px / VIEWBOX` and lets tiny-skia scale the stroke width with
/// it, which is why a 2.5-unit stroke stays optically identical at every
/// output size.
const VIEWBOX: f32 = 48.0;

/// Icon edge length in **logical points** inside a toolbar button.
///
/// [`crate::ICON_BUTTON_SIZE`] is 28x24 pt and egui's default
/// `button_padding` is (4,1), leaving a 20x22 pt content box. The ui-spec
/// §4.1 suggests "roughly 18–20px … leaving a few px of padding on every
/// side"; those two halves of the sentence conflict — 18 pt in a 20 pt box
/// leaves 1 pt, not "a few". **Recorded deviation:** 16 pt is used instead.
/// It honours the paragraph's actual intent (the click target stays
/// meaningfully larger than the visible glyph — the Fitts's-law win the
/// spec is really asking for), leaves 2 pt of padding horizontally and 3 pt
/// vertically, and pairs optically with egui's 12.5 pt body text on the
/// icon+text controls, which an 18 pt glyph does not.
pub const ICON_PTS: f32 = 16.0;

/// Stroke-width multiplier for [`IconWeight::Bold`].
///
/// 1.35 was chosen as the smallest factor that is unambiguously visible at
/// 16 pt (2.5 → 3.375 viewBox units, ~1.1 physical px heavier at 100%
/// scale) without the glyph starting to blob shut at its tightest interior
/// features (`keyboard.svg`'s 3-unit key gaps, `shape-highlight.svg`'s
/// hatch). It is a *cue*, not a redesign.
const BOLD_STROKE_FACTOR: f32 = 1.35;

/// Hard cap on live cache entries before the whole cache is dropped.
///
/// The cache grows only along three axes — ~35 icons x 2 weights x however
/// many distinct physical sizes the display scale has taken this session —
/// so in normal use it settles around 40 entries and never reaches this.
/// The cap exists solely so that a session that repeatedly changes display
/// scale (dragging a window between a 100% and a 150% monitor) cannot
/// accumulate stale textures without bound. Clearing wholesale rather than
/// evicting least-recently-used is deliberate: it is one line, it happens
/// approximately never, and the recovery cost is one frame of
/// re-rasterization.
const CACHE_CAPACITY: usize = 256;

// ---------------------------------------------------------------------------
// The icon catalogue
// ---------------------------------------------------------------------------

/// Every icon pdfce ships, one variant per drawn glyph.
///
/// Variants are named for the *role* the ui-spec §2 mapping assigns, not
/// for the artwork, so a future re-draw of an icon changes one asset file
/// and touches no call site. Two roles deliberately share one asset file:
/// [`Icon::Open`] and [`Icon::FontFolders`] are both the plain folder glyph
/// (ui-spec §3.5 confirms that reuse as intentional — Open is a top-level
/// toolbar action, Font Folders is a labelled row three levels into a dock,
/// and they are never on screen together).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Icon {
    /// Open a file (ui-spec §2 #1). ScripTree `icon-folder.svg`.
    Open,
    /// Save a copy (§2 #2, §3.1 "save").
    Save,
    /// Thumbnail-rail visibility toggle (§2 #3, §3.1 "sidebar").
    Sidebar,
    /// Annotation-visibility toggle (§2 #4, §3.1 "comment-bubble").
    Comment,
    /// Previous page (§2 #5, §3.1 "chevron").
    ChevronLeft,
    /// Leave a ribbon-opened surface and return to the armed tools' own
    /// options (§3.1 "back-arrow").
    ///
    /// Authored 2026-08-06 under the operator's standing ruling that a
    /// missing glyph is AUTHORED, not worked around: the control wanted
    /// `←` (U+2190), the Pass 18.7 coverage gate correctly rejected it as
    /// having no glyph in the shipped stack, and the first fix reworded the
    /// button to plain text. Rewording spends the operator's affordance to
    /// protect the font stack; an icon costs one asset and keeps both.
    ///
    /// Distinct from [`Icon::ChevronLeft`] by its SHAFT — see `back.svg`'s
    /// own note. Same reasoning that made [`Icon::ChevronUp`] and
    /// [`Icon::ChevronDown`] exist: those two were also authored precisely
    /// because their text glyphs were tofu.
    Back,
    /// Next page (§2 #6).
    ChevronRight,
    /// "Move selection up" in the page rail and the Combine-files list,
    /// drawn instead of the text glyph `▲` (U+25B2) — VERIFIED tofu in the
    /// running build 2026-08-03, same Geometric Shapes block as `▾`. Those
    /// buttons are glyph-ONLY, so a missing glyph left them with no visible
    /// identity at all.
    ChevronUp,
    /// Menu-disclosure marker on a dropdown button, drawn instead of the
    /// text glyph `▾` (U+25BE) — which is absent from every font in egui's
    /// default Proportional chain and rendered as a tofu box on four
    /// shipped toolbar controls. See
    /// `docs/ui_specs/menu-affordance-and-glyph-coverage.md`.
    ChevronDown,
    /// A magnifier — Find. Empty lens: `ZoomIn`/`ZoomOut` are the same
    /// shape carrying a `+`/`-`, so the unmarked lens is what says
    /// "search" rather than "magnify by".
    Search,
    /// Dismiss / remove — drawn instead of the text glyph `✕` (U+2715),
    /// which is absent from every font of the shipped stack (Pass 47.4).
    ///
    /// Authored rather than reworded, per the operator's 2026-08-06 ruling
    /// that a missing glyph for a real control gets an icon created for it.
    /// First use site: removing one row from the Create Field pane's option
    /// editor.
    Close,
    /// Zoom out (§2 #7, §3.1 "magnifier±").
    ZoomOut,
    /// Zoom in (§2 #8).
    ZoomIn,
    /// Fit whole page (§2 #9, §3.1 "frame-fit").
    FitPage,
    /// Fit page width (§2 #10, §3.1 "frame-fit-width").
    FitWidth,
    /// Rotate page counter-clockwise (§2 #12, §3.1 "rotate-page").
    RotateCcw,
    /// Rotate page clockwise (§2 #13).
    RotateCw,
    /// Document properties (§2 #14). ScripTree `icon-document.svg`.
    Properties,
    /// Markup menu (§2 #15, §3.1 "shapes").
    Markup,
    /// Text menu (§2 #16, §3.1 "note").
    Text,
    /// Edit page text (§2 #17). ScripTree `icon-edit.svg`.
    EditText,
    /// Add page text (§2 #18, §3.1 "text-cursor-plus").
    AddText,
    /// Edit vector objects — the Pass 9c-min "Obj" tool. NOT in the
    /// ui-spec (that tool shipped after the spec was written); authored in
    /// the same style contract, see `assets/icons/PROVENANCE.md`.
    EditObjects,
    /// Create an interactive form field — decision 020's F5 "Create Field"
    /// tool. Authored 2026-08-07 under R158; not in the ui-spec, same style
    /// contract as [`Icon::EditObjects`].
    ///
    /// Deliberately NOT reusing any existing asset. The nearest candidates
    /// each would have said something false: `edit-objects.svg` promises
    /// vector editing, and a plain box would read as the FILL surface, which
    /// is a different capability in a different ribbon group.
    FormField,
    /// Measure/dimension menu (§8.2). ScripTree `icon-ruler.svg`.
    Measure,
    /// Undo (§2 #19, §3.1 "history-arrow").
    Undo,
    /// Redo (§2 #20).
    Redo,
    /// Copy-text menu (§2 #21, §3.1 "copy").
    Copy,
    /// Tools dock toggle (§2 #22). ScripTree `icon-tool.svg`.
    Tools,
    /// Keyboard-shortcuts window (§2 #23, §3.1 "keyboard").
    Keyboard,
    /// "Show points" view toggle (Pass 36.3) — draws every anchor of the
    /// object being worked inside, so the points can be aimed at BEFORE one is
    /// selected.
    ///
    /// Deliberately close to, and deliberately distinct from,
    /// [`Icon::EditObjects`]: both show square node marks, because both are
    /// about the same points. That one puts two on a Bezier (shape editing);
    /// this one puts three on a straight run with the middle one filled (the
    /// points themselves, and the canvas's selected-node vocabulary).
    ShowPoints,
    /// Bookmarks panel toggle (View → Panels) — the document's outline.
    ///
    /// A ribbon with a notch, which is the one shape read as "bookmark"
    /// without a label. Deliberately not a page-with-lines: that is
    /// [`Icon::Properties`]/document territory, and this panel is about
    /// places IN a document rather than the document itself.
    Bookmarks,
    /// Layers panel toggle (View → Panels) — optional content, §8.11.
    ///
    /// Three stacked sheets. Three rather than two so it does not read as
    /// [`Icon::Combine`]'s linked pair at 16px.
    Layers,
    /// Signatures panel toggle (View → Panels) — §12.8 coverage.
    ///
    /// A written flourish over a signing rule, and emphatically **not** a
    /// seal, badge, shield or checkmark: each of those reads as VALIDATED,
    /// and pdfce performs no cryptographic verification. The panel's first
    /// line says so; the glyph must not contradict it before the panel is
    /// open. An icon is a claim too.
    Signatures,
    /// Markup → Rectangle (§3.3).
    ShapeRect,
    /// Markup → Ellipse (§3.3).
    ShapeEllipse,
    /// Markup → Arrow line (§3.3).
    ShapeArrow,
    /// Markup → Highlight band (§3.3).
    ShapeHighlight,
    /// Text → FreeText box (§3.3).
    TextFreeText,
    /// Text → Sticky note (§3.3).
    TextSticky,
    /// Text → Stamp, and the reserved Bates-numbering glyph (§3.4).
    Stamp,
    /// Combine files… (Tools dock, §2 #24). ScripTree `icon-link.svg`.
    Combine,
    /// Split this document… (Tools dock, §2 #25). ScripTree
    /// `icon-scissors.svg`.
    Split,
    /// Insert pages from a file… (Tools dock, §2 #26). ScripTree
    /// `icon-upload.svg`.
    InsertPages,
    /// Font folders… (Tools dock, §2 #27) — the same folder art as
    /// [`Icon::Open`], per ui-spec §3.5.
    FontFolders,
    /// Redaction (§8.1) — **wired at Pass 8.1** to the toolbar control that
    /// opens the dock's redaction review panel.
    ///
    /// It is the one intentionally solid-FILLED glyph in an otherwise
    /// all-outline set, which is also why it is the pipeline's only coverage
    /// of the fill path (see `redaction_is_the_only_filled_icon`). The fill
    /// is not decoration: every other tool in this app draws or measures,
    /// and this one obliterates, so its glyph reads as a solid bar rather
    /// than an outline of one.
    ///
    /// The `#[allow(dead_code)]` this variant carried while it was reserved
    /// is now removed — it has a call site, and leaving the attribute would
    /// suppress the signal if a future refactor orphaned it.
    Redact,
}

impl Icon {
    /// Every icon, in catalogue order.
    ///
    /// Used exclusively by the test module (which is why it is
    /// `dead_code`-allowed: `cargo build` sees no use, `cargo test`
    /// does). It is the list [`tests::every_icon_parses`] walks, and it
    /// is what makes "every shipped asset is valid" an enforced property
    /// rather than a hope — so a new [`Icon`] variant MUST be added here
    /// or it ships unverified.
    #[allow(dead_code)]
    pub const ALL: &'static [Icon] = &[
        Icon::Open,
        Icon::Save,
        Icon::Sidebar,
        Icon::Comment,
        Icon::ChevronLeft,
        Icon::Back,
        Icon::ChevronRight,
        Icon::ChevronDown,
        Icon::Search,
        Icon::ChevronUp,
        Icon::Close,
        Icon::ZoomOut,
        Icon::ZoomIn,
        Icon::FitPage,
        Icon::FitWidth,
        Icon::RotateCcw,
        Icon::RotateCw,
        Icon::Properties,
        Icon::Markup,
        Icon::Text,
        Icon::EditText,
        Icon::AddText,
        Icon::EditObjects,
        Icon::FormField,
        Icon::Measure,
        Icon::Undo,
        Icon::Redo,
        Icon::Copy,
        Icon::Tools,
        Icon::Keyboard,
        Icon::ShowPoints,
        Icon::Bookmarks,
        Icon::Layers,
        Icon::Signatures,
        Icon::ShapeRect,
        Icon::ShapeEllipse,
        Icon::ShapeArrow,
        Icon::ShapeHighlight,
        Icon::TextFreeText,
        Icon::TextSticky,
        Icon::Stamp,
        Icon::Combine,
        Icon::Split,
        Icon::InsertPages,
        Icon::FontFolders,
        Icon::Redact,
    ];

    /// The asset's SVG source, embedded at compile time.
    ///
    /// `include_str!` rather than a runtime file read because pdfce ships
    /// single-folder portable (`ARCHITECTURE.md` §6): the executable must
    /// not depend on an `assets/` directory travelling beside it, and an
    /// icon that fails to load at startup is not a failure mode worth
    /// having when the whole set is 6 KB of text.
    pub const fn source(self) -> &'static str {
        match self {
            Icon::Open | Icon::FontFolders => include_str!("../assets/icons/folder.svg"),
            Icon::Save => include_str!("../assets/icons/save.svg"),
            Icon::Sidebar => include_str!("../assets/icons/sidebar.svg"),
            Icon::Comment => include_str!("../assets/icons/comment.svg"),
            Icon::ChevronLeft => include_str!("../assets/icons/chevron-left.svg"),
            Icon::Back => include_str!("../assets/icons/back.svg"),
            Icon::ChevronRight => include_str!("../assets/icons/chevron-right.svg"),
            Icon::ChevronDown => include_str!("../assets/icons/chevron-down.svg"),
            Icon::Search => include_str!("../assets/icons/search.svg"),
            Icon::ChevronUp => include_str!("../assets/icons/chevron-up.svg"),
            Icon::Close => include_str!("../assets/icons/close.svg"),
            Icon::ZoomOut => include_str!("../assets/icons/zoom-out.svg"),
            Icon::ZoomIn => include_str!("../assets/icons/zoom-in.svg"),
            Icon::FitPage => include_str!("../assets/icons/fit-page.svg"),
            Icon::FitWidth => include_str!("../assets/icons/fit-width.svg"),
            Icon::RotateCcw => include_str!("../assets/icons/rotate-ccw.svg"),
            Icon::RotateCw => include_str!("../assets/icons/rotate-cw.svg"),
            Icon::Properties => include_str!("../assets/icons/document.svg"),
            Icon::Markup => include_str!("../assets/icons/markup.svg"),
            Icon::Text => include_str!("../assets/icons/text.svg"),
            Icon::EditText => include_str!("../assets/icons/edit.svg"),
            Icon::AddText => include_str!("../assets/icons/add-text.svg"),
            Icon::FormField => include_str!("../assets/icons/form-field.svg"),
            Icon::EditObjects => include_str!("../assets/icons/edit-objects.svg"),
            Icon::ShowPoints => include_str!("../assets/icons/show-points.svg"),
            Icon::Bookmarks => include_str!("../assets/icons/bookmarks.svg"),
            Icon::Layers => include_str!("../assets/icons/layers.svg"),
            Icon::Signatures => include_str!("../assets/icons/signatures.svg"),
            Icon::Measure => include_str!("../assets/icons/ruler.svg"),
            Icon::Undo => include_str!("../assets/icons/undo.svg"),
            Icon::Redo => include_str!("../assets/icons/redo.svg"),
            Icon::Copy => include_str!("../assets/icons/copy.svg"),
            Icon::Tools => include_str!("../assets/icons/tool.svg"),
            Icon::Keyboard => include_str!("../assets/icons/keyboard.svg"),
            Icon::ShapeRect => include_str!("../assets/icons/shape-rect.svg"),
            Icon::ShapeEllipse => include_str!("../assets/icons/shape-ellipse.svg"),
            Icon::ShapeArrow => include_str!("../assets/icons/shape-arrow.svg"),
            Icon::ShapeHighlight => include_str!("../assets/icons/shape-highlight.svg"),
            Icon::TextFreeText => include_str!("../assets/icons/text-freetext.svg"),
            Icon::TextSticky => include_str!("../assets/icons/text-sticky.svg"),
            Icon::Stamp => include_str!("../assets/icons/stamp.svg"),
            Icon::Combine => include_str!("../assets/icons/link.svg"),
            Icon::Split => include_str!("../assets/icons/scissors.svg"),
            Icon::InsertPages => include_str!("../assets/icons/upload.svg"),
            Icon::Redact => include_str!("../assets/icons/redact.svg"),
        }
    }

    /// A stable, human-readable key used as the egui texture's debug name.
    ///
    /// egui keys textures by handle, not by name, so this is purely for
    /// debuggers and texture inspectors — but a texture list full of
    /// "icon" tells you nothing, and one full of "icon:rotate-ccw@32:bold"
    /// tells you everything.
    pub const fn name(self) -> &'static str {
        match self {
            Icon::Open => "open",
            Icon::Save => "save",
            Icon::Sidebar => "sidebar",
            Icon::Comment => "comment",
            Icon::Close => "close",
            Icon::ChevronLeft => "chevron-left",
            Icon::Back => "back",
            Icon::ChevronRight => "chevron-right",
            Icon::ChevronDown => "chevron-down",
            Icon::Search => "search",
            Icon::ChevronUp => "chevron-up",
            Icon::ZoomOut => "zoom-out",
            Icon::ZoomIn => "zoom-in",
            Icon::FitPage => "fit-page",
            Icon::FitWidth => "fit-width",
            Icon::RotateCcw => "rotate-ccw",
            Icon::RotateCw => "rotate-cw",
            Icon::Properties => "properties",
            Icon::Markup => "markup",
            Icon::Text => "text",
            Icon::EditText => "edit-text",
            Icon::AddText => "add-text",
            Icon::FormField => "form-field",
            Icon::EditObjects => "edit-objects",
            Icon::ShowPoints => "show-points",
            Icon::Bookmarks => "bookmarks",
            Icon::Layers => "layers",
            Icon::Signatures => "signatures",
            Icon::Measure => "measure",
            Icon::Undo => "undo",
            Icon::Redo => "redo",
            Icon::Copy => "copy",
            Icon::Tools => "tools",
            Icon::Keyboard => "keyboard",
            Icon::ShapeRect => "shape-rect",
            Icon::ShapeEllipse => "shape-ellipse",
            Icon::ShapeArrow => "shape-arrow",
            Icon::ShapeHighlight => "shape-highlight",
            Icon::TextFreeText => "text-freetext",
            Icon::TextSticky => "text-sticky",
            Icon::Stamp => "stamp",
            Icon::Combine => "combine",
            Icon::Split => "split",
            Icon::InsertPages => "insert-pages",
            Icon::FontFolders => "font-folders",
            Icon::Redact => "redact",
        }
    }
}

/// How heavily an icon's outline is stroked.
///
/// See the module docs, "Weight": this is the non-colour selected-state cue
/// that replaces bolding a text label on controls that have no text.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum IconWeight {
    /// The asset's authored stroke width — every ordinary control.
    #[default]
    Regular,
    /// Stroke width scaled by [`BOLD_STROKE_FACTOR`] — selected/active
    /// toggles only.
    Bold,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Why an icon asset could not be turned into geometry.
///
/// Every variant means "this asset is wrong", never "this input was
/// untrusted" — the assets are compiled in, so any of these is an
/// authoring bug that [`tests::every_icon_parses`] is there to catch before
/// it ships. They carry position/context because the alternative (a bare
/// "parse failed") turns a two-minute fix into an afternoon.
///
/// Hand-written `Display`/`Error` impls rather than `thiserror`: adding
/// `thiserror` to `pdfce-gui` would be a new dependency edge, and this Pass
/// is explicitly zero-new-dependencies.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum IconError {
    /// An element outside the supported subset (`<g>`, `<use>`, `<defs>`,
    /// …). Refused rather than skipped: skipping a wrapper that carries a
    /// `transform` would draw the right shape in the wrong place.
    UnsupportedElement(String),
    /// A path-data letter that is not one of `MmLlHhVvCcSsQqTtAaZz`.
    UnsupportedPathCommand(char),
    /// Path data that begins with something other than a `MoveTo`, or a
    /// command issued with no current point.
    NoCurrentPoint(char),
    /// A number that could not be lexed at the given byte offset.
    MalformedNumber { offset: usize },
    /// An arc flag that was neither `0` nor `1` at the given byte offset.
    MalformedFlag { offset: usize },
    /// Path/attribute data ran out mid-command.
    UnexpectedEnd,
    /// A shape element was missing a geometry attribute it cannot be drawn
    /// without (e.g. `<circle>` with no `r`).
    MissingAttribute {
        /// The element that was missing it.
        element: &'static str,
        /// The attribute name.
        attribute: &'static str,
    },
    /// A `stroke-linecap` / `stroke-linejoin` value outside the subset.
    /// Refused rather than defaulted, because a silently wrong cap is a
    /// visible defect nobody would think to look for.
    UnsupportedPaintValue {
        /// The attribute whose value was rejected.
        attribute: &'static str,
        /// The rejected value.
        value: String,
    },
    /// A tag was opened and never closed (no `>` before end of input).
    UnterminatedTag,
    /// The geometry parsed, but tiny-skia rejected it (an empty or
    /// non-finite path). Practically unreachable for a hand-authored icon;
    /// present so the `Option` from `PathBuilder::finish` is never
    /// `unwrap`ped.
    DegeneratePath,
}

impl fmt::Display for IconError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedElement(tag) => {
                write!(
                    f,
                    "unsupported SVG element <{tag}> (icon subset: svg, path, rect, circle)"
                )
            }
            Self::UnsupportedPathCommand(c) => {
                write!(f, "unsupported SVG path command '{c}'")
            }
            Self::NoCurrentPoint(c) => {
                write!(
                    f,
                    "path command '{c}' issued with no current point (data must start with M/m)"
                )
            }
            Self::MalformedNumber { offset } => {
                write!(f, "malformed number in path data at byte {offset}")
            }
            Self::MalformedFlag { offset } => {
                write!(
                    f,
                    "malformed arc flag at byte {offset} (must be exactly '0' or '1')"
                )
            }
            Self::UnexpectedEnd => write!(f, "SVG data ended mid-command"),
            Self::MissingAttribute { element, attribute } => {
                write!(
                    f,
                    "<{element}> is missing the required '{attribute}' attribute"
                )
            }
            Self::UnsupportedPaintValue { attribute, value } => {
                write!(f, "unsupported {attribute} value '{value}'")
            }
            Self::UnterminatedTag => write!(f, "unterminated SVG tag (no '>')"),
            Self::DegeneratePath => write!(f, "path produced no drawable geometry"),
        }
    }
}

impl std::error::Error for IconError {}

// ---------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------

/// One drawable element of an icon: geometry plus how to paint it.
///
/// Kept separate per element rather than merged into one path because the
/// set mixes paint styles within a single icon — `redact.svg` has stroked
/// outlines *and* one filled bar, and `shape-highlight.svg` deliberately
/// mixes a 2.5-unit contour with a 1-unit hatch.
#[derive(Debug)]
struct Shape {
    /// Geometry in viewBox units (0..48).
    path: pdfce_render::tiny_skia::Path,
    /// Stroke width in viewBox units, or `None` for `stroke="none"`.
    stroke_width: Option<f32>,
    /// Cap style for stroking (ignored when `stroke_width` is `None`).
    line_cap: LineCap,
    /// Join style for stroking (ignored when `stroke_width` is `None`).
    line_join: LineJoin,
    /// Whether `fill="currentColor"` was set — the ui-spec §8.1 exception.
    filled: bool,
}

/// A parsed icon: an ordered list of shapes in viewBox units.
///
/// Order is paint order, exactly as written in the file, because outline
/// icons routinely rely on a later stroke crossing an earlier one.
#[derive(Debug)]
pub struct IconArt {
    shapes: Vec<Shape>,
}

impl IconArt {
    /// Parse an SVG asset into drawable geometry.
    ///
    /// See the module docs for the exact supported subset. This is a
    /// scanner, not an XML parser: it walks the byte stream looking for
    /// `<`, dispatches on the tag name, and reads `name="value"` attribute
    /// pairs out of the tag body. That is sufficient (and safe) because the
    /// only inputs are the repo's own compiled-in assets, which are
    /// mechanically uniform by the ui-spec §1 contract — and any input that
    /// is *not* uniform hits [`IconError::UnsupportedElement`] rather than
    /// being interpreted loosely.
    pub fn parse(source: &str) -> Result<Self, IconError> {
        let bytes = source.as_bytes();
        let mut shapes = Vec::new();
        let mut i = 0usize;

        while i < bytes.len() {
            // Skip everything that is not a tag opener. Character data
            // between tags is whitespace in every asset; there is no <text>
            // in the subset, so it is simply ignored.
            if bytes[i] != b'<' {
                i += 1;
                continue;
            }

            // XML comment — the ui-spec §1 contract requires every asset to
            // carry one naming the concept and disclaiming trademark risk,
            // so this branch is hit by every file.
            if bytes[i..].starts_with(b"<!--") {
                match find(bytes, i + 4, b"-->") {
                    Some(end) => {
                        i = end + 3;
                        continue;
                    }
                    None => return Err(IconError::UnterminatedTag),
                }
            }

            let tag_end = match bytes[i..].iter().position(|&b| b == b'>') {
                Some(off) => i + off,
                None => return Err(IconError::UnterminatedTag),
            };
            // `get` rather than direct slicing: the indices come from a
            // byte scan, and a stray multi-byte character inside a tag
            // would make them non-char-boundaries. Refuse rather than
            // panic.
            let body = source
                .get(i + 1..tag_end)
                .ok_or(IconError::UnterminatedTag)?;

            // A close tag (`</svg>`) carries no geometry. Skipping it is
            // safe *because* the subset has no container elements: an
            // unsupported opener like `<g>` is rejected below, so no
            // close tag can ever be the end of something whose effect we
            // would be missing.
            if body.starts_with('/') {
                i = tag_end + 1;
                continue;
            }

            let name = tag_name(body);

            match name {
                // Structural, no geometry: the root element (whose own
                // fill="none" is the set-wide default this parser already
                // assumes), plus any XML declaration.
                "svg" | "?xml" => {}
                "path" => shapes.push(parse_path_element(body)?),
                "rect" => shapes.push(parse_rect_element(body)?),
                "circle" => shapes.push(parse_circle_element(body)?),
                other => return Err(IconError::UnsupportedElement(other.to_owned())),
            }
            i = tag_end + 1;
        }

        Ok(Self { shapes })
    }

    /// How many drawable shapes the asset contains. Used by tests to prove
    /// a multi-element asset was fully read rather than truncated at the
    /// first element. (Test-only, hence the lint allowance — the GUI never
    /// needs to count shapes.)
    #[allow(dead_code)]
    pub fn shape_count(&self) -> usize {
        self.shapes.len()
    }

    /// Whether any shape is filled — the ui-spec §8.1 solid-glyph
    /// exception. Exposed so a test can assert that redaction's icon really
    /// is the filled one and that nothing else in the set is. (Test-only.)
    #[allow(dead_code)]
    pub fn has_fill(&self) -> bool {
        self.shapes.iter().any(|s| s.filled)
    }

    /// Rasterize to a square white-on-transparent coverage mask of `px`
    /// physical pixels a side (module docs, "Theming").
    ///
    /// The colour written is always opaque white; only the alpha channel
    /// carries information, and egui's tint supplies the hue at draw time.
    /// Antialiasing is on — at 16 pt these glyphs are ~2 px strokes and
    /// aliased diagonals would be immediately obvious.
    ///
    /// Returns a transparent image (never panics, never `None`) if the
    /// pixmap allocation fails for an absurd `px`; a missing icon is a
    /// cosmetic defect, a crashed editor is not.
    pub fn rasterize(&self, px: u32, weight: IconWeight) -> egui::ColorImage {
        let px = px.max(1);
        let Some(mut pixmap) = Pixmap::new(px, px) else {
            return egui::ColorImage::new([1, 1], vec![egui::Color32::TRANSPARENT]);
        };

        let scale = px as f32 / VIEWBOX;
        let transform = Transform::from_scale(scale, scale);
        let weight_factor = match weight {
            IconWeight::Regular => 1.0,
            IconWeight::Bold => BOLD_STROKE_FACTOR,
        };

        let mut paint = Paint {
            anti_alias: true,
            ..Paint::default()
        };
        // Opaque WHITE, always: only the alpha channel of the result
        // carries information (module docs, "Theming"). The hue arrives
        // later, from egui's tint.
        paint.set_color(Color::WHITE);

        for shape in &self.shapes {
            // Fill first, then stroke, matching SVG's own painting order
            // for an element that has both (redact.svg's bar is fill-only,
            // but the ordering must be right if that ever changes).
            if shape.filled {
                pixmap
                    .as_mut()
                    .fill_path(&shape.path, &paint, FillRule::Winding, transform, None);
            }
            if let Some(width) = shape.stroke_width {
                let stroke = Stroke {
                    width: width * weight_factor,
                    line_cap: shape.line_cap,
                    line_join: shape.line_join,
                    ..Stroke::default()
                };
                // tiny-skia strokes in PATH space and transforms the
                // resulting outline, so the stroke width scales with
                // `transform` — which is exactly what a viewBox needs and
                // why no manual width compensation appears here.
                pixmap
                    .as_mut()
                    .stroke_path(&shape.path, &paint, &stroke, transform, None);
            }
        }

        // tiny-skia's buffer is premultiplied RGBA8 and egui's Color32 is
        // premultiplied sRGBA, so this is a straight reinterpretation with
        // no un-premultiply/re-premultiply round trip (which would lose
        // precision in the antialiased edge pixels that are the entire
        // visual quality of a 16 pt glyph).
        let pixels = pixmap
            .pixels()
            .iter()
            .map(|p| {
                egui::Color32::from_rgba_premultiplied(p.red(), p.green(), p.blue(), p.alpha())
            })
            .collect();
        egui::ColorImage::new([px as usize, px as usize], pixels)
    }
}

// ---------------------------------------------------------------------------
// Element parsing
// ---------------------------------------------------------------------------

/// The tag name at the start of a tag body (everything up to the first
/// whitespace, `/` or end).
fn tag_name(body: &str) -> &str {
    let end = body
        .find(|c: char| c.is_ascii_whitespace() || c == '/')
        .unwrap_or(body.len());
    &body[..end]
}

/// Find `needle` in `haystack` at or after `from`, returning its start.
fn find(haystack: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    if from >= haystack.len() {
        return None;
    }
    haystack[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}

/// Read a double-quoted attribute value out of a tag body.
///
/// Matches on `name="` with a preceding delimiter check so that looking up
/// `x` does not match `rx`, and `stroke` does not match `stroke-width` —
/// the single most likely silent-wrongness bug in a scanner this simple,
/// and the reason this is one shared helper rather than an inline `find`
/// at each call site.
fn attr<'a>(body: &'a str, name: &str) -> Option<&'a str> {
    let bytes = body.as_bytes();
    let pattern = format!("{name}=\"");
    let mut from = 0usize;
    while let Some(pos) = body[from..].find(pattern.as_str()) {
        let abs = from + pos;
        let preceded_ok = abs == 0 || bytes[abs - 1].is_ascii_whitespace();
        if preceded_ok {
            let start = abs + pattern.len();
            let end = body[start..].find('"')? + start;
            return Some(&body[start..end]);
        }
        from = abs + 1;
    }
    None
}

/// Parse an attribute as an `f32`, or report a malformed number.
fn attr_f32(body: &str, name: &str) -> Option<Result<f32, IconError>> {
    attr(body, name).map(|raw| {
        raw.trim()
            .parse::<f32>()
            .map_err(|_| IconError::MalformedNumber { offset: 0 })
    })
}

/// The paint attributes shared by every shape element.
///
/// Absent `stroke` means "not stroked" and absent `fill` means "not
/// filled", matching the root `<svg fill="none">` the ui-spec §1 contract
/// mandates. `stroke-width` defaults to the contract's 2.5 so an asset that
/// omits it still draws at the set's weight rather than tiny-skia's 1.0.
fn parse_paint(body: &str) -> Result<(Option<f32>, LineCap, LineJoin, bool), IconError> {
    let stroked = matches!(attr(body, "stroke"), Some(v) if v != "none");
    let stroke_width = if stroked {
        Some(match attr_f32(body, "stroke-width") {
            Some(v) => v?,
            None => 2.5,
        })
    } else {
        None
    };

    let line_cap = match attr(body, "stroke-linecap") {
        None | Some("butt") => LineCap::Butt,
        Some("round") => LineCap::Round,
        Some("square") => LineCap::Square,
        Some(other) => {
            return Err(IconError::UnsupportedPaintValue {
                attribute: "stroke-linecap",
                value: other.to_owned(),
            });
        }
    };
    let line_join = match attr(body, "stroke-linejoin") {
        None | Some("miter") => LineJoin::Miter,
        Some("round") => LineJoin::Round,
        Some("bevel") => LineJoin::Bevel,
        Some(other) => {
            return Err(IconError::UnsupportedPaintValue {
                attribute: "stroke-linejoin",
                value: other.to_owned(),
            });
        }
    };

    let filled = matches!(attr(body, "fill"), Some(v) if v != "none");
    Ok((stroke_width, line_cap, line_join, filled))
}

/// Assemble a [`Shape`] from a finished builder plus the element's paint.
fn finish_shape(builder: PathBuilder, body: &str) -> Result<Shape, IconError> {
    let path = builder.finish().ok_or(IconError::DegeneratePath)?;
    let (stroke_width, line_cap, line_join, filled) = parse_paint(body)?;
    Ok(Shape {
        path,
        stroke_width,
        line_cap,
        line_join,
        filled,
    })
}

/// `<path d="…"/>`.
fn parse_path_element(body: &str) -> Result<Shape, IconError> {
    let d = attr(body, "d").ok_or(IconError::MissingAttribute {
        element: "path",
        attribute: "d",
    })?;
    let mut builder = PathBuilder::new();
    parse_path_data(d, &mut builder)?;
    finish_shape(builder, body)
}

/// `<rect x y width height rx?/>`, including the rounded-corner form.
fn parse_rect_element(body: &str) -> Result<Shape, IconError> {
    let need = |name: &'static str| -> Result<f32, IconError> {
        attr_f32(body, name).unwrap_or(Err(IconError::MissingAttribute {
            element: "rect",
            attribute: name,
        }))
    };
    let x = need("x")?;
    let y = need("y")?;
    let w = need("width")?;
    let h = need("height")?;
    let rx = match attr_f32(body, "rx") {
        Some(v) => v?,
        None => 0.0,
    };

    let mut builder = PathBuilder::new();
    push_round_rect(&mut builder, x, y, w, h, rx);
    finish_shape(builder, body)
}

/// `<circle cx cy r/>`.
fn parse_circle_element(body: &str) -> Result<Shape, IconError> {
    let need = |name: &'static str| -> Result<f32, IconError> {
        attr_f32(body, name).unwrap_or(Err(IconError::MissingAttribute {
            element: "circle",
            attribute: name,
        }))
    };
    let cx = need("cx")?;
    let cy = need("cy")?;
    let r = need("r")?;

    let mut builder = PathBuilder::new();
    builder.push_circle(cx, cy, r);
    finish_shape(builder, body)
}

/// Circular-arc-to-cubic magic constant: the control-point offset, as a
/// fraction of the radius, that makes a single cubic Bézier approximate a
/// 90° circular arc to within ~0.02%. `4/3 * (sqrt(2) - 1)`.
const KAPPA: f32 = 0.552_284_8;

/// Emit a rounded rectangle as an explicit path.
///
/// tiny-skia's `push_rect` has no corner radius, and the set uses `rx` on
/// most rects, so the corners are drawn as four 90° cubic arcs. Radius is
/// clamped to half the shorter side, which is what SVG requires and what
/// stops a hand-edited `rx="99"` from turning into a self-intersecting
/// mess instead of a stadium.
fn push_round_rect(pb: &mut PathBuilder, x: f32, y: f32, w: f32, h: f32, rx: f32) {
    let r = rx.min(w / 2.0).min(h / 2.0).max(0.0);
    if r <= 0.0 {
        pb.move_to(x, y);
        pb.line_to(x + w, y);
        pb.line_to(x + w, y + h);
        pb.line_to(x, y + h);
        pb.close();
        return;
    }
    let k = r * KAPPA;
    let (x1, y1) = (x + w, y + h);
    pb.move_to(x + r, y);
    pb.line_to(x1 - r, y);
    pb.cubic_to(x1 - r + k, y, x1, y + r - k, x1, y + r);
    pb.line_to(x1, y1 - r);
    pb.cubic_to(x1, y1 - r + k, x1 - r + k, y1, x1 - r, y1);
    pb.line_to(x + r, y1);
    pb.cubic_to(x + r - k, y1, x, y1 - r + k, x, y1 - r);
    pb.line_to(x, y + r);
    pb.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
    pb.close();
}

// ---------------------------------------------------------------------------
// Path-data parsing
// ---------------------------------------------------------------------------

/// A cursor over an SVG path `d` string.
///
/// Byte-oriented because the grammar is pure ASCII; any non-ASCII byte can
/// only be a stray character and will fail the command/number lex rather
/// than being mis-sliced.
struct PathLexer<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> PathLexer<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            bytes: s.as_bytes(),
            pos: 0,
        }
    }

    /// Skip whitespace and the optional comma separator. SVG treats commas
    /// and whitespace interchangeably, and permits neither at all when the
    /// tokens are unambiguous (`M6 14h12l4 4`), so this is called before
    /// every token and is allowed to consume nothing.
    fn skip_separators(&mut self) {
        while self.pos < self.bytes.len() {
            match self.bytes[self.pos] {
                b' ' | b'\t' | b'\r' | b'\n' | b',' => self.pos += 1,
                _ => break,
            }
        }
    }

    fn at_end(&mut self) -> bool {
        self.skip_separators();
        self.pos >= self.bytes.len()
    }

    /// Peek the next byte without consuming, after separators.
    fn peek(&mut self) -> Option<u8> {
        self.skip_separators();
        self.bytes.get(self.pos).copied()
    }

    /// Consume a command letter.
    fn take_command(&mut self) -> Option<char> {
        let b = self.peek()?;
        if b.is_ascii_alphabetic() {
            self.pos += 1;
            Some(b as char)
        } else {
            None
        }
    }

    /// Lex one number: `[+-]? ( digits [. digits?] | . digits ) ( [eE] [+-]? digits )?`.
    ///
    /// Written by hand rather than by handing a slice to `f32::from_str`
    /// because the *extent* of the number is the hard part: `1.5.5` is two
    /// numbers, `1-2` is two numbers, and `M6 14h12l4 4` has no separators
    /// at all. Getting the extent wrong is precisely the "silently draws
    /// the wrong glyph" failure this module exists to avoid, so the extent
    /// is computed explicitly and only then handed to `from_str`.
    fn take_number(&mut self) -> Result<f32, IconError> {
        self.skip_separators();
        let start = self.pos;
        if self.pos < self.bytes.len()
            && (self.bytes[self.pos] == b'+' || self.bytes[self.pos] == b'-')
        {
            self.pos += 1;
        }
        let mut digits = false;
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
            self.pos += 1;
            digits = true;
        }
        if self.pos < self.bytes.len() && self.bytes[self.pos] == b'.' {
            self.pos += 1;
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
                self.pos += 1;
                digits = true;
            }
        }
        if !digits {
            return Err(IconError::MalformedNumber { offset: start });
        }
        // Exponent, only if it is actually well formed — a trailing `e`
        // that is not followed by digits belongs to the next token, not
        // this number.
        if self.pos < self.bytes.len() && (self.bytes[self.pos] | 0x20) == b'e' {
            let save = self.pos;
            self.pos += 1;
            if self.pos < self.bytes.len()
                && (self.bytes[self.pos] == b'+' || self.bytes[self.pos] == b'-')
            {
                self.pos += 1;
            }
            let exp_start = self.pos;
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
            if self.pos == exp_start {
                self.pos = save;
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| IconError::MalformedNumber { offset: start })?;
        text.parse::<f32>()
            .map_err(|_| IconError::MalformedNumber { offset: start })
    }

    /// Lex an arc flag: exactly ONE `0` or `1` character.
    ///
    /// This is not a number lex, and the difference is load-bearing.
    /// `link.svg` in the ScripTree set is written `a6 6 0 008 8`, where
    /// `008` is large-arc=0, sweep=0, x=8. A number lexer would swallow
    /// `008` as the single value 8 and draw a wildly wrong chain link.
    fn take_flag(&mut self) -> Result<bool, IconError> {
        self.skip_separators();
        match self.bytes.get(self.pos) {
            Some(b'0') => {
                self.pos += 1;
                Ok(false)
            }
            Some(b'1') => {
                self.pos += 1;
                Ok(true)
            }
            Some(_) => Err(IconError::MalformedFlag { offset: self.pos }),
            None => Err(IconError::UnexpectedEnd),
        }
    }
}

/// Parse an SVG path `d` string into `builder`.
///
/// Implements the full path grammar (module docs). Two pieces of state make
/// the whole thing work and are worth naming explicitly:
///
/// * `cur` — the current point. Relative commands are offsets from it, and
///   a command that needs it before any `M` is [`IconError::NoCurrentPoint`]
///   rather than an implicit origin, because an implicit origin silently
///   draws a glyph anchored at the viewBox corner.
/// * `start` — the current subpath's first point, which `Z` returns to and
///   which a command *after* a `Z` continues from (SVG's rule, and the one
///   most often got wrong).
/// * `reflect` — the previous cubic/quadratic control point, mirrored on
///   demand by the smooth forms `S`/`T`. Reset to the current point after
///   any non-curve command, per the spec: `S` after an `L` is a plain
///   curve, not a reflection of something three commands ago.
fn parse_path_data(d: &str, builder: &mut PathBuilder) -> Result<(), IconError> {
    let mut lex = PathLexer::new(d);
    let mut cur = (0.0f32, 0.0f32);
    let mut start = (0.0f32, 0.0f32);
    let mut cubic_reflect: Option<(f32, f32)> = None;
    let mut quad_reflect: Option<(f32, f32)> = None;
    let mut have_current = false;
    let mut command: Option<char> = None;

    loop {
        if lex.at_end() {
            break;
        }
        // A command letter may be omitted for repeated parameter sets
        // ("M10 10 20 20" is a moveto then an implicit lineto; "h12 4" is
        // two horizontal linetos). When the next token is not a letter we
        // reuse the previous command — with SVG's one special case that a
        // repeated `M`/`m` becomes `L`/`l`.
        if let Some(c) = lex.peek() {
            if c.is_ascii_alphabetic() {
                command = lex.take_command();
            } else {
                command = match command {
                    Some('M') => Some('L'),
                    Some('m') => Some('l'),
                    other => other,
                };
            }
        }
        let Some(c) = command else {
            return Err(IconError::UnexpectedEnd);
        };

        let relative = c.is_ascii_lowercase();
        let base = if relative { cur } else { (0.0, 0.0) };

        // Every command except M/m and Z/z needs a current point.
        if !have_current && !matches!(c, 'M' | 'm') {
            return Err(IconError::NoCurrentPoint(c));
        }

        match c {
            'M' | 'm' => {
                let x = lex.take_number()? + base.0;
                let y = lex.take_number()? + base.1;
                builder.move_to(x, y);
                cur = (x, y);
                start = (x, y);
                have_current = true;
                cubic_reflect = None;
                quad_reflect = None;
            }
            'L' | 'l' => {
                let x = lex.take_number()? + base.0;
                let y = lex.take_number()? + base.1;
                builder.line_to(x, y);
                cur = (x, y);
                cubic_reflect = None;
                quad_reflect = None;
            }
            'H' | 'h' => {
                let x = lex.take_number()? + base.0;
                builder.line_to(x, cur.1);
                cur = (x, cur.1);
                cubic_reflect = None;
                quad_reflect = None;
            }
            'V' | 'v' => {
                let y = lex.take_number()? + base.1;
                builder.line_to(cur.0, y);
                cur = (cur.0, y);
                cubic_reflect = None;
                quad_reflect = None;
            }
            'C' | 'c' => {
                let x1 = lex.take_number()? + base.0;
                let y1 = lex.take_number()? + base.1;
                let x2 = lex.take_number()? + base.0;
                let y2 = lex.take_number()? + base.1;
                let x = lex.take_number()? + base.0;
                let y = lex.take_number()? + base.1;
                builder.cubic_to(x1, y1, x2, y2, x, y);
                cur = (x, y);
                cubic_reflect = Some((x2, y2));
                quad_reflect = None;
            }
            'S' | 's' => {
                let (x1, y1) = match cubic_reflect {
                    Some((px, py)) => (2.0 * cur.0 - px, 2.0 * cur.1 - py),
                    None => cur,
                };
                let x2 = lex.take_number()? + base.0;
                let y2 = lex.take_number()? + base.1;
                let x = lex.take_number()? + base.0;
                let y = lex.take_number()? + base.1;
                builder.cubic_to(x1, y1, x2, y2, x, y);
                cur = (x, y);
                cubic_reflect = Some((x2, y2));
                quad_reflect = None;
            }
            'Q' | 'q' => {
                let x1 = lex.take_number()? + base.0;
                let y1 = lex.take_number()? + base.1;
                let x = lex.take_number()? + base.0;
                let y = lex.take_number()? + base.1;
                builder.quad_to(x1, y1, x, y);
                cur = (x, y);
                quad_reflect = Some((x1, y1));
                cubic_reflect = None;
            }
            'T' | 't' => {
                let (x1, y1) = match quad_reflect {
                    Some((px, py)) => (2.0 * cur.0 - px, 2.0 * cur.1 - py),
                    None => cur,
                };
                let x = lex.take_number()? + base.0;
                let y = lex.take_number()? + base.1;
                builder.quad_to(x1, y1, x, y);
                cur = (x, y);
                quad_reflect = Some((x1, y1));
                cubic_reflect = None;
            }
            'A' | 'a' => {
                let rx = lex.take_number()?;
                let ry = lex.take_number()?;
                let rot = lex.take_number()?;
                let large_arc = lex.take_flag()?;
                let sweep = lex.take_flag()?;
                let x = lex.take_number()? + base.0;
                let y = lex.take_number()? + base.1;
                match arc_to_cubics(cur, rx, ry, rot, large_arc, sweep, (x, y)) {
                    Some(segments) => {
                        for [x1, y1, x2, y2, ex, ey] in segments {
                            builder.cubic_to(x1, y1, x2, y2, ex, ey);
                        }
                    }
                    // SVG: a zero radius (or a zero-length arc) degenerates
                    // to a straight line rather than being an error.
                    None => builder.line_to(x, y),
                }
                cur = (x, y);
                cubic_reflect = None;
                quad_reflect = None;
            }
            'Z' | 'z' => {
                builder.close();
                cur = start;
                cubic_reflect = None;
                quad_reflect = None;
            }
            other => return Err(IconError::UnsupportedPathCommand(other)),
        }
    }

    Ok(())
}

/// Convert one SVG elliptical-arc segment to a list of cubic Béziers.
///
/// Implements the endpoint→centre parameterization of the SVG 1.1
/// implementation notes F.6.5, then subdivides the swept angle into
/// segments of at most 90° (F.6.6) because a single cubic cannot
/// approximate a larger arc acceptably — the 270° arcs in `undo.svg` and
/// `rotate-ccw.svg` become three cubics each.
///
/// Returns `None` for the degenerate cases the spec says to treat as a
/// straight line: either radius zero, or coincident endpoints.
///
/// Parameters mirror the SVG grammar exactly: `from`/`to` are endpoints,
/// `rx`/`ry` radii, `rot_deg` the x-axis rotation, and the two booleans the
/// large-arc and sweep flags. Radii are enlarged (never shrunk) when they
/// are too small to span the endpoints, per F.6.6 step 3 — otherwise
/// `sqrt` of a negative number would silently produce NaN geometry.
#[allow(clippy::too_many_arguments)]
fn arc_to_cubics(
    from: (f32, f32),
    rx: f32,
    ry: f32,
    rot_deg: f32,
    large_arc: bool,
    sweep: bool,
    to: (f32, f32),
) -> Option<Vec<[f32; 6]>> {
    let (x1, y1) = (from.0 as f64, from.1 as f64);
    let (x2, y2) = (to.0 as f64, to.1 as f64);
    let mut rx = (rx as f64).abs();
    let mut ry = (ry as f64).abs();
    if rx == 0.0 || ry == 0.0 || ((x1 - x2).abs() < f64::EPSILON && (y1 - y2).abs() < f64::EPSILON)
    {
        return None;
    }
    let phi = (rot_deg as f64).to_radians();
    let (sin_phi, cos_phi) = phi.sin_cos();

    // F.6.5.1 — endpoints into the rotated, midpoint-centred frame.
    let dx = (x1 - x2) / 2.0;
    let dy = (y1 - y2) / 2.0;
    let x1p = cos_phi * dx + sin_phi * dy;
    let y1p = -sin_phi * dx + cos_phi * dy;

    // F.6.6.2 — grow radii if they cannot span the chord.
    let lambda = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
    if lambda > 1.0 {
        let s = lambda.sqrt();
        rx *= s;
        ry *= s;
    }

    // F.6.5.2 — the centre in the rotated frame.
    let num = (rx * rx * ry * ry) - (rx * rx * y1p * y1p) - (ry * ry * x1p * x1p);
    let den = (rx * rx * y1p * y1p) + (ry * ry * x1p * x1p);
    let sign = if large_arc == sweep { -1.0 } else { 1.0 };
    let coef = sign * (num / den).max(0.0).sqrt();
    let cxp = coef * rx * y1p / ry;
    let cyp = -coef * ry * x1p / rx;

    // F.6.5.3 — back to user space.
    let cx = cos_phi * cxp - sin_phi * cyp + (x1 + x2) / 2.0;
    let cy = sin_phi * cxp + cos_phi * cyp + (y1 + y2) / 2.0;

    // F.6.5.5/6 — start angle and swept angle.
    let ux = (x1p - cxp) / rx;
    let uy = (y1p - cyp) / ry;
    let vx = (-x1p - cxp) / rx;
    let vy = (-y1p - cyp) / ry;
    let theta1 = angle_between(1.0, 0.0, ux, uy);
    let mut delta = angle_between(ux, uy, vx, vy);
    if !sweep && delta > 0.0 {
        delta -= std::f64::consts::TAU;
    } else if sweep && delta < 0.0 {
        delta += std::f64::consts::TAU;
    }

    // F.6.6 — subdivide into <=90 degree pieces.
    let count = (delta.abs() / std::f64::consts::FRAC_PI_2).ceil().max(1.0) as usize;
    let step = delta / count as f64;
    // The tangent-scaling factor for a cubic approximating a `step`-wide
    // arc. At step = 90 degrees this reduces to KAPPA.
    let alpha = 4.0 / 3.0 * (step / 4.0).tan();

    let point_at = |t: f64| -> (f64, f64) {
        let (s, c) = t.sin_cos();
        (
            cx + rx * c * cos_phi - ry * s * sin_phi,
            cy + rx * c * sin_phi + ry * s * cos_phi,
        )
    };
    let deriv_at = |t: f64| -> (f64, f64) {
        let (s, c) = t.sin_cos();
        (
            -rx * s * cos_phi - ry * c * sin_phi,
            -rx * s * sin_phi + ry * c * cos_phi,
        )
    };

    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let t1 = theta1 + step * i as f64;
        let t2 = t1 + step;
        let (p1x, p1y) = point_at(t1);
        let (p2x, p2y) = point_at(t2);
        let (d1x, d1y) = deriv_at(t1);
        let (d2x, d2y) = deriv_at(t2);
        out.push([
            (p1x + alpha * d1x) as f32,
            (p1y + alpha * d1y) as f32,
            (p2x - alpha * d2x) as f32,
            (p2y - alpha * d2y) as f32,
            (p2x) as f32,
            (p2y) as f32,
        ]);
    }
    // Snap the final endpoint back onto the requested one. The
    // trigonometric round trip lands within ~1e-6 of it, which is
    // invisible, but an exact match keeps the following command's relative
    // offsets exact and makes `Z` close cleanly.
    if let Some(last) = out.last_mut() {
        last[4] = to.0;
        last[5] = to.1;
    }
    Some(out)
}

/// Signed angle from vector *(ux, uy)* to *(vx, vy)*, in radians, in
/// (-pi, pi]. SVG F.6.5.4.
fn angle_between(ux: f64, uy: f64, vx: f64, vy: f64) -> f64 {
    let dot = ux * vx + uy * vy;
    let len = ((ux * ux + uy * uy) * (vx * vx + vy * vy)).sqrt();
    if len == 0.0 {
        return 0.0;
    }
    let mut a = (dot / len).clamp(-1.0, 1.0).acos();
    if ux * vy - uy * vx < 0.0 {
        a = -a;
    }
    a
}

// ---------------------------------------------------------------------------
// Cache
// ---------------------------------------------------------------------------

/// What uniquely identifies a raster.
///
/// The tint is deliberately absent — see the module docs, "Theming". Adding
/// it would multiply the cache by the number of theme/state colours for
/// exactly zero benefit, since egui applies the tint to the mask for free
/// at draw time.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct CacheKey {
    icon: Icon,
    /// Physical pixels a side — the DPI-resolved size, not the logical one.
    px: u32,
    weight: IconWeight,
}

/// Memoized icon textures.
///
/// Public and independently constructible so the caching contract can be
/// asserted in unit tests without going through the thread-local; the GUI
/// itself always uses [`with_cache`].
#[derive(Default)]
pub struct IconCache {
    textures: HashMap<CacheKey, egui::TextureHandle>,
    /// How many times an asset has actually been parsed + rasterized this
    /// session. Not used by the GUI; it is the observable that makes "the
    /// cache is doing its job" testable rather than merely plausible.
    rasterizations: usize,
}

impl IconCache {
    /// Fetch (or build) the texture for one icon at one physical size and
    /// weight.
    ///
    /// On a parse failure the icon degrades to a 1x1 transparent texture
    /// and a one-line stderr complaint rather than panicking: the assets
    /// are compiled in, so a failure here means the *build* shipped a
    /// broken asset — a condition [`tests::every_icon_parses`] is designed
    /// to catch first — and taking down an editor holding the operator's
    /// unsaved edits over a missing 16 px glyph would be a far worse
    /// outcome than a blank button with an intact tooltip and accessible
    /// name.
    pub fn texture(
        &mut self,
        ctx: &egui::Context,
        icon: Icon,
        px: u32,
        weight: IconWeight,
    ) -> egui::TextureHandle {
        let key = CacheKey { icon, px, weight };
        if let Some(handle) = self.textures.get(&key) {
            return handle.clone();
        }
        if self.textures.len() >= CACHE_CAPACITY {
            self.textures.clear();
        }

        let image = match IconArt::parse(icon.source()) {
            Ok(art) => art.rasterize(px, weight),
            Err(err) => {
                eprintln!(
                    // ui-text-exempt: stderr diagnostic, never rendered in the GUI. An icon is a
                    // compile-time `include_str!` asset, so this fires only when a DEVELOPER has
                    // committed a malformed SVG — it is a build-time defect report addressed to
                    // whoever broke the asset, not operator copy. The operator-visible consequence
                    // is the blank 1x1 image below, which is deliberately silent: pdfce never
                    // invents a look for something it could not draw.
                    "pdfce: icon asset '{}' failed to parse ({err}); drawing nothing",
                    icon.name()
                );
                egui::ColorImage::new([1, 1], vec![egui::Color32::TRANSPARENT])
            }
        };
        self.rasterizations += 1;

        let name = format!("icon:{}@{px}:{weight:?}", icon.name());
        // LINEAR filtering: the raster is produced at the exact physical
        // size it will be drawn at, so filtering is a no-op in the normal
        // case — but if egui ever draws it at a fractional offset, linear
        // is the difference between a soft edge and a shimmering one.
        let handle = ctx.load_texture(name, image, egui::TextureOptions::LINEAR);
        self.textures.insert(key, handle.clone());
        handle
    }

    /// How many rasterizations have happened — the cache's testable
    /// observable (see the `rasterizations` field docs). Test-only: the
    /// GUI never asks, which is the whole point of a transparent cache.
    #[allow(dead_code)]
    pub fn rasterizations(&self) -> usize {
        self.rasterizations
    }

    /// How many distinct textures are currently held. Test-only.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.textures.len()
    }

    /// Whether nothing has been cached yet. Present because a public
    /// `len()` without an `is_empty()` is an API-guidelines violation
    /// (project rule 10), not because anything needs it.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }
}

thread_local! {
    /// The process-wide (in practice, UI-thread-wide) icon cache.
    ///
    /// See the module docs, "Caching", for why this is a thread-local
    /// rather than a field on `PdfceApp`: threading `&mut IconCache`
    /// through the toolbar would collapse several disjoint field borrows of
    /// `self` into one whole-`self` borrow and fail to compile.
    static CACHE: RefCell<IconCache> = RefCell::new(IconCache::default());
}

/// Run `f` with the shared cache. Kept private so no caller can hold the
/// `RefMut` across a re-entrant call (which would panic); every public
/// entry point below borrows, does one lookup, and releases.
fn with_cache<R>(f: impl FnOnce(&mut IconCache) -> R) -> R {
    CACHE.with(|c| f(&mut c.borrow_mut()))
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

/// Build the drawable image for `icon`, tinted `tint`, at [`ICON_PTS`]
/// logical points.
///
/// The texture is rasterized at `ICON_PTS * pixels_per_point()` PHYSICAL
/// pixels and then declared to be `ICON_PTS` logical points wide, which is
/// what makes it crisp on a HiDPI display instead of a stretched blur. See
/// the module docs, "DPI".
pub fn image_tinted(
    ui: &egui::Ui,
    icon: Icon,
    weight: IconWeight,
    tint: egui::Color32,
) -> egui::Image<'static> {
    let ctx = ui.ctx();
    let px = (ICON_PTS * ctx.pixels_per_point()).round().max(1.0) as u32;
    let handle = with_cache(|cache| cache.texture(ctx, icon, px, weight));
    let sized = egui::load::SizedTexture::new(handle.id(), egui::vec2(ICON_PTS, ICON_PTS));
    egui::Image::from_texture(sized)
        .fit_to_exact_size(egui::vec2(ICON_PTS, ICON_PTS))
        .tint(tint)
}

/// An icon in the ordinary (non-selected) state.
///
/// The tint is `ui.visuals().text_color()` read from the CALLER's `Ui`,
/// which is what makes an icon inside `add_enabled_ui(false, …)` fade in
/// lockstep with the text buttons beside it (ui-spec §5.2) with no
/// disabled-state logic of its own.
pub fn image(ui: &egui::Ui, icon: Icon) -> egui::Image<'static> {
    image_tinted(ui, icon, IconWeight::Regular, ui.visuals().text_color())
}

/// An icon in the selected/active state of a toggle.
///
/// Two cues at once, neither of which is the background fill egui already
/// paints: the accent tint AND [`IconWeight::Bold`]. The toolbar adds a
/// third (an outline ring) for icon-only toggles. That layering is the
/// standing "selected state is never colour alone" rule (project rule 6 /
/// P1-1, ui-spec §5.3) surviving the loss of a text label to embolden.
pub fn selected_image(ui: &egui::Ui, icon: Icon) -> egui::Image<'static> {
    let tint = ui.visuals().selection.stroke.color;
    image_tinted(ui, icon, IconWeight::Bold, tint)
}

/// The right image for a toggle in either state — the two-line helper that
/// keeps every call site from re-deriving the same `if selected`.
pub fn toggle_image(ui: &egui::Ui, icon: Icon, selected: bool) -> egui::Image<'static> {
    if selected {
        selected_image(ui, icon)
    } else {
        image(ui, icon)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a path and report how many verbs it produced — the cheapest
    /// proxy for "the parser understood the command" that does not depend
    /// on tiny-skia's internal point layout.
    fn verbs(d: &str) -> Result<usize, IconError> {
        let mut pb = PathBuilder::new();
        parse_path_data(d, &mut pb)?;
        Ok(pb.len())
    }

    /// Every shipped asset must parse. This is the gate that makes the
    /// hand-rolled parser safe to rely on: a malformed or out-of-subset
    /// icon fails `cargo test` rather than shipping as a blank button.
    #[test]
    fn every_icon_parses() {
        for &icon in Icon::ALL {
            let art = IconArt::parse(icon.source())
                .unwrap_or_else(|e| panic!("icon '{}' failed to parse: {e}", icon.name()));
            assert!(
                art.shape_count() > 0,
                "icon '{}' parsed to zero shapes",
                icon.name()
            );
        }
    }

    /// Every asset must also rasterize to something visible. A glyph that
    /// parses but draws nothing (e.g. every shape `stroke="none"`) would
    /// otherwise sail through `every_icon_parses`.
    #[test]
    fn every_icon_rasterizes_to_visible_pixels() {
        for &icon in Icon::ALL {
            let art = IconArt::parse(icon.source()).expect("parses");
            let img = art.rasterize(32, IconWeight::Regular);
            assert_eq!(img.size, [32, 32]);
            let lit = img.pixels.iter().filter(|p| p.a() > 0).count();
            assert!(lit > 20, "icon '{}' rasterized nearly blank", icon.name());
        }
    }

    /// The ui-spec §8.1 exception, asserted so a future "style cleanup"
    /// cannot quietly turn redaction's honest solid bar into an outline,
    /// and so no other icon drifts into being filled.
    #[test]
    fn redaction_is_the_only_filled_icon() {
        for &icon in Icon::ALL {
            let art = IconArt::parse(icon.source()).expect("parses");
            assert_eq!(
                art.has_fill(),
                icon == Icon::Redact,
                "fill expectation violated for '{}'",
                icon.name()
            );
        }
    }

    /// The assets are ordinary text files, so this repo's
    /// `* text=auto` rule converts them to CRLF on checkout with
    /// `core.autocrlf=true` (see `.gitattributes`, whose header records
    /// how badly that bit the binary PDF fixtures). SVG is text and CRLF
    /// is harmless *provided* the scanner treats `\r` as a separator
    /// everywhere `\n` is one. This pins that: the same asset with every
    /// line ending doubled must produce identical geometry, so a fresh
    /// clone on a machine with autocrlf on cannot ship blank icons.
    #[test]
    fn crlf_line_endings_parse_identically() {
        for &icon in Icon::ALL {
            let lf = icon.source().replace("\r\n", "\n");
            let crlf = lf.replace('\n', "\r\n");
            let a = IconArt::parse(&lf).expect("LF parses");
            let b = IconArt::parse(&crlf)
                .unwrap_or_else(|e| panic!("CRLF form of '{}' failed: {e}", icon.name()));
            assert_eq!(
                a.shape_count(),
                b.shape_count(),
                "CRLF changed shape count for '{}'",
                icon.name()
            );
            assert_eq!(
                a.rasterize(24, IconWeight::Regular).pixels,
                b.rasterize(24, IconWeight::Regular).pixels,
                "CRLF changed the raster for '{}'",
                icon.name()
            );
        }
    }

    #[test]
    fn parses_absolute_and_relative_move_and_line() {
        // M + L, M + l, and the implicit-lineto-after-moveto rule.
        assert_eq!(verbs("M10 10L20 20").unwrap(), 2);
        assert_eq!(verbs("M10 10l10 10").unwrap(), 2);
        assert_eq!(verbs("M10 10 20 20 30 30").unwrap(), 3);
    }

    #[test]
    fn parses_horizontal_and_vertical() {
        assert_eq!(verbs("M6 14h12v22H6V14").unwrap(), 5);
    }

    #[test]
    fn parses_cubic_and_smooth_cubic() {
        assert_eq!(verbs("M10 34C10 18 24 34 38 14").unwrap(), 2);
        assert_eq!(verbs("M0 0C1 1 2 2 3 3S4 4 5 5").unwrap(), 3);
    }

    #[test]
    fn parses_quadratic_and_smooth_quadratic() {
        assert_eq!(verbs("M0 0Q1 1 2 2").unwrap(), 2);
        assert_eq!(verbs("M0 0Q1 1 2 2T4 4").unwrap(), 3);
    }

    #[test]
    fn parses_close() {
        // M, L, L, Z => 4 verbs.
        assert_eq!(verbs("M10 10L20 10L20 20Z").unwrap(), 4);
    }

    #[test]
    fn parses_arc_as_cubics() {
        // A 270 degree arc subdivides into three <=90 degree cubics, so
        // move + 3 cubics = 4 verbs. This is the exact construction
        // undo.svg uses.
        assert_eq!(verbs("M24 10A14 14 0 1 0 38 24").unwrap(), 4);
    }

    /// The packed-flag form that appears verbatim in the ScripTree
    /// `icon-link.svg` this project reuses. A number lexer would read
    /// `008` as `8` and silently draw a wrong glyph; this asserts the flags
    /// are lexed one character at a time.
    #[test]
    fn parses_packed_arc_flags() {
        let mut pb = PathBuilder::new();
        parse_path_data("M16 24l-4 4a6 6 0 008 8l4-4", &mut pb).expect("packed flags parse");
        // move, line, (arc -> >=1 cubic), line.
        assert!(pb.len() >= 4);

        // And the same arc written with separated flags must agree.
        let mut spaced = PathBuilder::new();
        parse_path_data("M16 24l-4 4a6 6 0 0 0 8 8l4-4", &mut spaced).expect("spaced flags parse");
        assert_eq!(pb.len(), spaced.len());
    }

    #[test]
    fn parses_negative_and_fractional_numbers_without_separators() {
        // "-2" terminates the previous number; ".5" needs no leading zero.
        assert_eq!(verbs("M8 10l2-2l.5.5").unwrap(), 3);
    }

    #[test]
    fn parses_exponent_numbers() {
        assert_eq!(verbs("M1e1 1E1L2e1 2e1").unwrap(), 2);
    }

    // -- refusal (never silent mis-drawing) -----------------------------

    #[test]
    fn refuses_unknown_path_command() {
        assert_eq!(
            verbs("M0 0 X10 10").unwrap_err(),
            IconError::UnsupportedPathCommand('X')
        );
    }

    #[test]
    fn refuses_command_before_any_moveto() {
        assert_eq!(verbs("L10 10").unwrap_err(), IconError::NoCurrentPoint('L'));
    }

    #[test]
    fn refuses_malformed_number() {
        // `M` needs two numbers; the second is not one.
        assert!(matches!(
            verbs("M10 abc").unwrap_err(),
            IconError::MalformedNumber { .. }
        ));
        // A lone sign is not a number.
        assert!(matches!(
            verbs("M10 -").unwrap_err(),
            IconError::MalformedNumber { .. }
        ));
    }

    #[test]
    fn refuses_truncated_command() {
        assert!(matches!(
            verbs("M10 10L20").unwrap_err(),
            IconError::MalformedNumber { .. } | IconError::UnexpectedEnd
        ));
    }

    #[test]
    fn refuses_bad_arc_flag() {
        assert!(matches!(
            verbs("M0 0A6 6 0 2 0 8 8").unwrap_err(),
            IconError::MalformedFlag { .. }
        ));
    }

    #[test]
    fn refuses_unsupported_element() {
        let svg = r#"<svg viewBox="0 0 48 48"><g transform="translate(4,4)"><path d="M0 0L1 1"/></g></svg>"#;
        assert_eq!(
            IconArt::parse(svg).unwrap_err(),
            IconError::UnsupportedElement("g".to_owned())
        );
    }

    #[test]
    fn refuses_unsupported_linecap() {
        let svg = r#"<svg><path d="M0 0L1 1" stroke="currentColor" stroke-linecap="flat"/></svg>"#;
        assert!(matches!(
            IconArt::parse(svg).unwrap_err(),
            IconError::UnsupportedPaintValue {
                attribute: "stroke-linecap",
                ..
            }
        ));
    }

    #[test]
    fn refuses_shape_missing_geometry() {
        let svg = r#"<svg><circle cx="4" cy="4" stroke="currentColor"/></svg>"#;
        assert_eq!(
            IconArt::parse(svg).unwrap_err(),
            IconError::MissingAttribute {
                element: "circle",
                attribute: "r"
            }
        );
    }

    #[test]
    fn refuses_unterminated_tag() {
        assert_eq!(
            IconArt::parse("<svg><path d=\"M0 0\"").unwrap_err(),
            IconError::UnterminatedTag
        );
    }

    // -- attribute scanning ---------------------------------------------

    /// `attr` must not confuse `x` with `rx`, nor `stroke` with
    /// `stroke-width`. Getting this wrong would move every rounded rect in
    /// the set.
    #[test]
    fn attribute_lookup_is_not_a_substring_match() {
        let body =
            r#"rect x="10" y="4" width="28" rx="2" stroke="currentColor" stroke-width="2.5""#;
        assert_eq!(attr(body, "x"), Some("10"));
        assert_eq!(attr(body, "rx"), Some("2"));
        assert_eq!(attr(body, "stroke"), Some("currentColor"));
        assert_eq!(attr(body, "stroke-width"), Some("2.5"));
        assert_eq!(attr(body, "height"), None);
    }

    // -- paint ------------------------------------------------------------

    #[test]
    fn stroke_none_is_not_drawn_but_fill_is() {
        let svg = r#"<svg><rect x="0" y="0" width="10" height="10" fill="currentColor" stroke="none"/></svg>"#;
        let art = IconArt::parse(svg).expect("parses");
        assert!(art.has_fill());
        assert_eq!(art.shape_count(), 1);
        let img = art.rasterize(32, IconWeight::Regular);
        assert!(img.pixels.iter().any(|p| p.a() > 0), "fill drew nothing");
    }

    #[test]
    fn bold_weight_covers_more_pixels_than_regular() {
        let art = IconArt::parse(Icon::ChevronLeft.source()).expect("parses");
        let regular = art.rasterize(48, IconWeight::Regular);
        let bold = art.rasterize(48, IconWeight::Bold);
        let lit = |img: &egui::ColorImage| img.pixels.iter().filter(|p| p.a() > 128).count();
        assert!(
            lit(&bold) > lit(&regular),
            "bold weight must be a visible cue: regular={} bold={}",
            lit(&regular),
            lit(&bold)
        );
    }

    #[test]
    fn mask_is_white_so_tinting_is_exact() {
        // Every non-transparent pixel must be premultiplied WHITE, i.e.
        // r == g == b == a. That property is what makes egui's
        // multiplicative tint produce a correctly premultiplied tinted
        // glyph for any tint colour (module docs, "Theming").
        let art = IconArt::parse(Icon::ShapeRect.source()).expect("parses");
        let img = art.rasterize(32, IconWeight::Regular);
        for p in &img.pixels {
            let a = i32::from(p.a());
            for c in [p.r(), p.g(), p.b()] {
                // +/-1 tolerance for tiny-skia's integer premultiply
                // rounding; anything larger would mean a coloured (i.e.
                // untintable) mask.
                assert!(
                    (i32::from(c) - a).abs() <= 1,
                    "mask pixel is not white: rgba=({},{},{},{})",
                    p.r(),
                    p.g(),
                    p.b(),
                    p.a()
                );
            }
        }
    }

    #[test]
    fn rasterizes_at_the_requested_physical_size() {
        let art = IconArt::parse(Icon::Undo.source()).expect("parses");
        assert_eq!(art.rasterize(16, IconWeight::Regular).size, [16, 16]);
        assert_eq!(art.rasterize(48, IconWeight::Regular).size, [48, 48]);
    }

    // -- cache ------------------------------------------------------------

    /// The load-bearing property: the toolbar asks for the same icon every
    /// frame, and only the FIRST ask may rasterize. Tint is not part of the
    /// key by design (mask + tint, ui-spec §6), so re-asking for a
    /// different colour must also be a cache hit.
    #[test]
    fn cache_serves_repeat_requests_without_re_rasterizing() {
        let ctx = egui::Context::default();
        let mut cache = IconCache::default();

        let first = cache.texture(&ctx, Icon::Undo, 16, IconWeight::Regular);
        assert_eq!(cache.rasterizations(), 1);
        assert_eq!(cache.len(), 1);

        // Same icon+size+weight, 60 more times (one simulated second of
        // toolbar frames). The tint would differ between light and dark
        // theme and between hovered and idle; none of that reaches here.
        for _ in 0..60 {
            let again = cache.texture(&ctx, Icon::Undo, 16, IconWeight::Regular);
            assert_eq!(again.id(), first.id());
        }
        assert_eq!(cache.rasterizations(), 1, "cache re-rasterized on a hit");
        assert_eq!(cache.len(), 1);
    }

    /// A display-scale change (or a future larger-icon option) changes the
    /// physical size, and that MUST produce a new raster — reusing the old
    /// one is exactly the blurry-on-HiDPI bug this pipeline exists to
    /// avoid.
    #[test]
    fn cache_re_rasterizes_for_a_different_size_or_weight() {
        let ctx = egui::Context::default();
        let mut cache = IconCache::default();

        let small = cache.texture(&ctx, Icon::Undo, 16, IconWeight::Regular);
        let large = cache.texture(&ctx, Icon::Undo, 32, IconWeight::Regular);
        assert_ne!(small.id(), large.id(), "different size reused a texture");
        assert_eq!(cache.rasterizations(), 2);

        let bold = cache.texture(&ctx, Icon::Undo, 16, IconWeight::Bold);
        assert_ne!(small.id(), bold.id(), "different weight reused a texture");
        assert_eq!(cache.rasterizations(), 3);
        assert_eq!(cache.len(), 3);

        // A different icon at an already-cached size is still a miss.
        let _ = cache.texture(&ctx, Icon::Redo, 16, IconWeight::Regular);
        assert_eq!(cache.rasterizations(), 4);
    }

    /// Every icon must survive a full round trip through the real cache,
    /// which is what the toolbar actually calls.
    #[test]
    fn cache_handles_the_whole_catalogue() {
        let ctx = egui::Context::default();
        let mut cache = IconCache::default();
        for &icon in Icon::ALL {
            let _ = cache.texture(&ctx, icon, 16, IconWeight::Regular);
        }
        // Open and FontFolders share one asset but are distinct cache
        // entries (distinct keys), which is intended and cheap.
        assert_eq!(cache.len(), Icon::ALL.len());
        assert_eq!(cache.rasterizations(), Icon::ALL.len());
    }
}
